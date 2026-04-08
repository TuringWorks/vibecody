#![allow(dead_code)]
//! Skill distillation — automatically learn project-specific patterns from agent sessions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternType {
    LibraryPreference,
    NamingConvention,
    FileOrganization,
    ErrorHandling,
    TestStyle,
    CodeStyle,
    ArchitecturePattern,
    ConfigPreference,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternSource {
    Accepted,
    Rejected,
    Corrected,
    Observed,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PatternConfidence {
    Tentative,
    Weak,
    Moderate,
    Strong,
}

impl PatternConfidence {
    fn promote(&self) -> PatternConfidence {
        match self {
            PatternConfidence::Tentative => PatternConfidence::Weak,
            PatternConfidence::Weak => PatternConfidence::Moderate,
            PatternConfidence::Moderate => PatternConfidence::Strong,
            PatternConfidence::Strong => PatternConfidence::Strong,
        }
    }

    fn demote(&self) -> PatternConfidence {
        match self {
            PatternConfidence::Strong => PatternConfidence::Moderate,
            PatternConfidence::Moderate => PatternConfidence::Weak,
            PatternConfidence::Weak => PatternConfidence::Tentative,
            PatternConfidence::Tentative => PatternConfidence::Tentative,
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOutcome {
    pub session_id: String,
    pub edits: Vec<EditOutcome>,
    pub started_at: u64,
    pub completed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOutcome {
    pub file_path: String,
    pub accepted: bool,
    pub original_code: String,
    pub final_code: String,
    pub user_correction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub description: String,
    pub rule: String,
    pub examples: Vec<String>,
    pub counter_examples: Vec<String>,
    pub source: PatternSource,
    pub confidence: PatternConfidence,
    pub occurrences: u64,
    pub last_seen: u64,
    pub project_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledSkill {
    pub name: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub trigger_words: Vec<String>,
    pub generated_at: u64,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillConfig {
    pub min_occurrences: u64,
    pub min_confidence: PatternConfidence,
    pub auto_distill: bool,
    pub max_patterns: usize,
    pub ab_test_new_skills: bool,
}

impl Default for DistillConfig {
    fn default() -> Self {
        Self {
            min_occurrences: 3,
            min_confidence: PatternConfidence::Moderate,
            auto_distill: true,
            max_patterns: 200,
            ab_test_new_skills: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DistillMetrics {
    pub sessions_analyzed: u64,
    pub patterns_extracted: u64,
    pub skills_generated: u64,
    pub patterns_promoted: u64,
    pub patterns_demoted: u64,
    pub improvement_score: f64,
}

// ---------------------------------------------------------------------------
// PatternExtractor
// ---------------------------------------------------------------------------

pub struct PatternExtractor;

impl PatternExtractor {
    pub fn extract_from_outcome(outcome: &EditOutcome) -> Vec<LearnedPattern> {
        let mut patterns = Vec::new();
        let source = if outcome.accepted {
            if outcome.user_correction.is_some() {
                PatternSource::Corrected
            } else {
                PatternSource::Accepted
            }
        } else {
            PatternSource::Rejected
        };

        let code = if outcome.accepted {
            &outcome.final_code
        } else {
            &outcome.original_code
        };

        // Detect library preferences
        patterns.extend(Self::detect_library_preference(code).into_iter().map(|mut p| {
            p.source = source.clone();
            p
        }));

        // Detect naming conventions
        patterns.extend(Self::detect_naming_convention(code).into_iter().map(|mut p| {
            p.source = source.clone();
            p
        }));

        // Detect file organization
        patterns.extend(Self::detect_file_organization(&outcome.file_path).into_iter().map(|mut p| {
            p.source = source.clone();
            p
        }));

        // Detect error handling
        patterns.extend(Self::detect_error_handling(code).into_iter().map(|mut p| {
            p.source = source.clone();
            p
        }));

        patterns
    }

    pub fn detect_library_preference(code: &str) -> Vec<LearnedPattern> {
        let mut patterns = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();

            // Rust use/extern crate
            if trimmed.starts_with("use ") {
                if let Some(crate_name) = trimmed
                    .strip_prefix("use ")
                    .and_then(|s| s.split("::").next())
                    .map(|s| s.trim_end_matches(';').to_string())
                {
                    if !["std", "self", "super", "crate"].contains(&crate_name.as_str()) {
                        patterns.push(LearnedPattern {
                            id: String::new(),
                            pattern_type: PatternType::LibraryPreference,
                            description: format!("Prefers crate `{}`", crate_name),
                            rule: format!("use-crate:{}", crate_name),
                            examples: vec![trimmed.to_string()],
                            counter_examples: Vec::new(),
                            source: PatternSource::Observed,
                            confidence: PatternConfidence::Tentative,
                            occurrences: 1,
                            last_seen: 0,
                            project_scope: None,
                        });
                    }
                }
            }

            // JS/TS imports
            if trimmed.starts_with("import ") || (trimmed.starts_with("const ") && trimmed.contains("require(")) {
                if let Some(start) = trimmed.find('\"').or_else(|| trimmed.find('\'')) {
                    let rest = &trimmed[start + 1..];
                    if let Some(end) = rest.find('\"').or_else(|| rest.find('\'')) {
                        let module = &rest[..end];
                        if !module.starts_with('.') && !module.starts_with('/') {
                            let pkg = module.split('/').next().unwrap_or(module);
                            patterns.push(LearnedPattern {
                                id: String::new(),
                                pattern_type: PatternType::LibraryPreference,
                                description: format!("Prefers package `{}`", pkg),
                                rule: format!("use-package:{}", pkg),
                                examples: vec![trimmed.to_string()],
                                counter_examples: Vec::new(),
                                source: PatternSource::Observed,
                                confidence: PatternConfidence::Tentative,
                                occurrences: 1,
                                last_seen: 0,
                                project_scope: None,
                            });
                        }
                    }
                }
            }

            // Python: from X import Y
            if trimmed.starts_with("from ") && trimmed.contains(" import ") {
                let module = trimmed
                    .strip_prefix("from ")
                    .unwrap_or("")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .split('.')
                    .next()
                    .unwrap_or("");
                if !module.is_empty() {
                    patterns.push(LearnedPattern {
                        id: String::new(),
                        pattern_type: PatternType::LibraryPreference,
                        description: format!("Prefers Python module `{}`", module),
                        rule: format!("use-pymodule:{}", module),
                        examples: vec![trimmed.to_string()],
                        counter_examples: Vec::new(),
                        source: PatternSource::Observed,
                        confidence: PatternConfidence::Tentative,
                        occurrences: 1,
                        last_seen: 0,
                        project_scope: None,
                    });
                }
            }

            // Python: import X (plain — no braces, no quotes to avoid JS false positives)
            if trimmed.starts_with("import ")
                && !trimmed.contains('{')
                && !trimmed.contains('\"')
                && !trimmed.contains('\'')
            {
                let module = trimmed
                    .strip_prefix("import ")
                    .unwrap_or("")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .split('.')
                    .next()
                    .unwrap_or("");
                if !module.is_empty() && module != "os" && module != "sys" {
                    patterns.push(LearnedPattern {
                        id: String::new(),
                        pattern_type: PatternType::LibraryPreference,
                        description: format!("Prefers Python module `{}`", module),
                        rule: format!("use-pymodule:{}", module),
                        examples: vec![trimmed.to_string()],
                        counter_examples: Vec::new(),
                        source: PatternSource::Observed,
                        confidence: PatternConfidence::Tentative,
                        occurrences: 1,
                        last_seen: 0,
                        project_scope: None,
                    });
                }
            }
        }

        patterns
    }

    pub fn detect_naming_convention(code: &str) -> Vec<LearnedPattern> {
        let mut patterns = Vec::new();

        let mut snake_count = 0u32;
        let mut camel_count = 0u32;

        for line in code.lines() {
            let trimmed = line.trim();
            // Function / method / variable declarations
            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("const ")
                || trimmed.starts_with("let ")
            {
                let has_underscore = trimmed.contains('_');
                let has_camel = trimmed
                    .chars()
                    .zip(trimmed.chars().skip(1))
                    .any(|(a, b)| a.is_lowercase() && b.is_uppercase());
                if has_underscore {
                    snake_count += 1;
                }
                if has_camel {
                    camel_count += 1;
                }
            }
        }

        if snake_count > 0 && snake_count > camel_count {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::NamingConvention,
                description: "Prefers snake_case naming".to_string(),
                rule: "naming:snake_case".to_string(),
                examples: vec!["fn my_function()".to_string()],
                counter_examples: vec!["fn myFunction()".to_string()],
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: snake_count as u64,
                last_seen: 0,
                project_scope: None,
            });
        }
        if camel_count > 0 && camel_count > snake_count {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::NamingConvention,
                description: "Prefers camelCase naming".to_string(),
                rule: "naming:camelCase".to_string(),
                examples: vec!["function myFunction()".to_string()],
                counter_examples: vec!["function my_function()".to_string()],
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: camel_count as u64,
                last_seen: 0,
                project_scope: None,
            });
        }

        patterns
    }

    pub fn detect_file_organization(path: &str) -> Vec<LearnedPattern> {
        let mut patterns = Vec::new();

        if path.contains("/tests/") || path.contains("/test/") {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::FileOrganization,
                description: "Tests in dedicated tests/ directory".to_string(),
                rule: "file-org:tests-directory".to_string(),
                examples: vec![path.to_string()],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        if path.contains("__tests__") {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::FileOrganization,
                description: "Tests in __tests__/ directories (Jest convention)".to_string(),
                rule: "file-org:jest-tests".to_string(),
                examples: vec![path.to_string()],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        if path.ends_with("_test.go") {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::FileOrganization,
                description: "Go test files co-located with source".to_string(),
                rule: "file-org:go-colocated-tests".to_string(),
                examples: vec![path.to_string()],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        if path.contains(".test.") || path.contains(".spec.") {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::FileOrganization,
                description: "Test files use .test. or .spec. suffix".to_string(),
                rule: "file-org:suffix-test-files".to_string(),
                examples: vec![path.to_string()],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        if path.contains("/src/components/") || path.starts_with("src/components/") {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::FileOrganization,
                description: "React components in src/components/".to_string(),
                rule: "file-org:components-dir".to_string(),
                examples: vec![path.to_string()],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        if path.contains("/src/utils/") || path.contains("/src/helpers/")
            || path.starts_with("src/utils/") || path.starts_with("src/helpers/") {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::FileOrganization,
                description: "Utility code in utils/ or helpers/".to_string(),
                rule: "file-org:utils-dir".to_string(),
                examples: vec![path.to_string()],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }

        patterns
    }

    pub fn detect_error_handling(code: &str) -> Vec<LearnedPattern> {
        let mut patterns = Vec::new();

        let has_result = code.contains("Result<") || code.contains("-> Result");
        let has_unwrap = code.contains(".unwrap()");
        let has_expect = code.contains(".expect(");
        let has_question_mark = code.contains('?');
        let has_try_catch = code.contains("try {") || code.contains("try:");
        let has_panic = code.contains("panic!");

        if has_result && has_question_mark && !has_unwrap && !has_panic {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::ErrorHandling,
                description: "Uses Result with ? operator, avoids unwrap/panic".to_string(),
                rule: "error:result-question-mark".to_string(),
                examples: vec!["fn foo() -> Result<(), Error> { bar()? }".to_string()],
                counter_examples: vec!["bar().unwrap()".to_string()],
                source: PatternSource::Observed,
                confidence: PatternConfidence::Moderate,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }

        if has_expect && !has_unwrap {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::ErrorHandling,
                description: "Uses .expect() instead of .unwrap() for context".to_string(),
                rule: "error:expect-over-unwrap".to_string(),
                examples: vec!["file.open().expect(\"failed to open config\")".to_string()],
                counter_examples: vec!["file.open().unwrap()".to_string()],
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }

        if has_try_catch {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::ErrorHandling,
                description: "Uses try-catch for error handling".to_string(),
                rule: "error:try-catch".to_string(),
                examples: vec!["try { ... } catch(e) { ... }".to_string()],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }

        if has_unwrap && !has_result {
            patterns.push(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::ErrorHandling,
                description: "Uses .unwrap() liberally (may indicate prototype code)".to_string(),
                rule: "error:liberal-unwrap".to_string(),
                examples: vec!["value.unwrap()".to_string()],
                counter_examples: vec!["value.expect(\"reason\")".to_string()],
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }

        patterns
    }
}

// ---------------------------------------------------------------------------
// DistillationEngine
// ---------------------------------------------------------------------------

pub struct DistillationEngine {
    pub config: DistillConfig,
    patterns: HashMap<String, LearnedPattern>,
    skills: Vec<DistilledSkill>,
    metrics: DistillMetrics,
    next_id: u64,
    timestamp_counter: u64,
}

impl DistillationEngine {
    pub fn new(config: DistillConfig) -> Self {
        Self {
            config,
            patterns: HashMap::new(),
            skills: Vec::new(),
            metrics: DistillMetrics::default(),
            next_id: 1,
            timestamp_counter: 1,
        }
    }

    pub fn analyze_session(&mut self, outcome: SessionOutcome) {
        self.metrics.sessions_analyzed += 1;

        for edit in &outcome.edits {
            let extracted = PatternExtractor::extract_from_outcome(edit);
            for pattern in extracted {
                self.metrics.patterns_extracted += 1;
                self.merge_pattern(pattern);
            }
        }

        // Enforce max_patterns by evicting lowest-confidence, lowest-occurrence patterns
        while self.patterns.len() > self.config.max_patterns {
            let weakest = self
                .patterns
                .iter()
                .min_by(|a, b| {
                    a.1.confidence
                        .cmp(&b.1.confidence)
                        .then(a.1.occurrences.cmp(&b.1.occurrences))
                })
                .map(|(k, _)| k.clone());
            if let Some(key) = weakest {
                self.patterns.remove(&key);
            } else {
                break;
            }
        }

        // Auto-distill if configured
        if self.config.auto_distill {
            let _ = self.distill_skills();
        }
    }

    pub fn merge_pattern(&mut self, new: LearnedPattern) {
        let rule_key = new.rule.clone();
        if let Some(existing) = self.patterns.get_mut(&rule_key) {
            existing.occurrences += new.occurrences;
            existing.last_seen = self.timestamp_counter;
            // Promote confidence after enough occurrences
            if existing.occurrences >= self.config.min_occurrences
                && existing.confidence < PatternConfidence::Strong
            {
                existing.confidence = existing.confidence.promote();
            }
            for ex in new.examples {
                if !existing.examples.contains(&ex) && existing.examples.len() < 5 {
                    existing.examples.push(ex);
                }
            }
            for cex in new.counter_examples {
                if !existing.counter_examples.contains(&cex) && existing.counter_examples.len() < 5
                {
                    existing.counter_examples.push(cex);
                }
            }
        } else {
            let id = format!("pat-{}", self.next_id);
            self.next_id += 1;
            let mut pattern = new;
            pattern.id = id;
            pattern.last_seen = self.timestamp_counter;
            let key = pattern.rule.clone();
            self.patterns.insert(key, pattern);
        }
        self.timestamp_counter += 1;
    }

    pub fn promote_pattern(&mut self, id: &str) -> Result<(), String> {
        let pattern = self
            .patterns
            .values_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("Pattern '{}' not found", id))?;
        let old = pattern.confidence.clone();
        pattern.confidence = pattern.confidence.promote();
        if pattern.confidence != old {
            self.metrics.patterns_promoted += 1;
        }
        Ok(())
    }

    pub fn demote_pattern(&mut self, id: &str) -> Result<(), String> {
        let pattern = self
            .patterns
            .values_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("Pattern '{}' not found", id))?;
        let old = pattern.confidence.clone();
        pattern.confidence = pattern.confidence.demote();
        if pattern.confidence != old {
            self.metrics.patterns_demoted += 1;
        }
        Ok(())
    }

    pub fn distill_skills(&mut self) -> Vec<DistilledSkill> {
        let mut skills = Vec::new();

        // Group qualifying patterns by type
        let mut by_type: HashMap<PatternType, Vec<&LearnedPattern>> = HashMap::new();
        for p in self.patterns.values() {
            if p.occurrences >= self.config.min_occurrences
                && p.confidence >= self.config.min_confidence
            {
                by_type.entry(p.pattern_type.clone()).or_default().push(p);
            }
        }

        for (pt, group) in &by_type {
            let name = match pt {
                PatternType::LibraryPreference => "library-preferences".to_string(),
                PatternType::NamingConvention => "naming-conventions".to_string(),
                PatternType::FileOrganization => "file-organization".to_string(),
                PatternType::ErrorHandling => "error-handling".to_string(),
                PatternType::TestStyle => "test-style".to_string(),
                PatternType::CodeStyle => "code-style".to_string(),
                PatternType::ArchitecturePattern => "architecture-patterns".to_string(),
                PatternType::ConfigPreference => "config-preferences".to_string(),
                PatternType::Custom(ref s) => format!("custom-{}", s),
            };

            let description = format!(
                "Distilled {} patterns ({} rules)",
                name,
                group.len()
            );

            let pattern_rules: Vec<String> = group.iter().map(|p| p.rule.clone()).collect();
            let trigger_words: Vec<String> = group
                .iter()
                .flat_map(|p| {
                    p.rule
                        .split(':')
                        .next_back()
                        .map(|s| s.to_string())
                        .into_iter()
                })
                .collect();

            skills.push(DistilledSkill {
                name,
                description,
                patterns: pattern_rules,
                trigger_words,
                generated_at: self.timestamp_counter,
                version: 1,
            });
        }

        self.metrics.skills_generated = skills.len() as u64;
        self.skills = skills.clone();
        skills
    }

    pub fn get_patterns(&self) -> Vec<&LearnedPattern> {
        self.patterns.values().collect()
    }

    pub fn get_patterns_by_type(&self, pt: &PatternType) -> Vec<&LearnedPattern> {
        self.patterns
            .values()
            .filter(|p| &p.pattern_type == pt)
            .collect()
    }

    pub fn get_skills(&self) -> Vec<&DistilledSkill> {
        self.skills.iter().collect()
    }

    pub fn export_skills(&self) -> String {
        let mut md = String::new();
        md.push_str("# Distilled Skills\n\n");
        md.push_str(&format!(
            "Generated from {} sessions, {} patterns extracted\n\n",
            self.metrics.sessions_analyzed, self.metrics.patterns_extracted
        ));

        for skill in &self.skills {
            md.push_str(&format!("## {}\n\n", skill.name));
            md.push_str(&format!("{}\n\n", skill.description));
            md.push_str("### Rules\n\n");
            for rule in &skill.patterns {
                md.push_str(&format!("- `{}`\n", rule));
            }
            md.push_str("\n### Trigger Words\n\n");
            for tw in &skill.trigger_words {
                md.push_str(&format!("- {}\n", tw));
            }
            md.push_str(&format!("\n_Version {}_\n\n---\n\n", skill.version));
        }

        md
    }

    pub fn reset(&mut self) {
        self.patterns.clear();
        self.skills.clear();
        self.metrics = DistillMetrics::default();
        self.next_id = 1;
        self.timestamp_counter = 1;
    }

    pub fn get_metrics(&self) -> &DistillMetrics {
        &self.metrics
    }

    pub fn improvement_estimate(&self) -> f64 {
        if self.metrics.sessions_analyzed == 0 {
            return 0.0;
        }
        let strong_patterns = self
            .patterns
            .values()
            .filter(|p| p.confidence == PatternConfidence::Strong)
            .count() as f64;
        let moderate_patterns = self
            .patterns
            .values()
            .filter(|p| p.confidence == PatternConfidence::Moderate)
            .count() as f64;
        let total = self.patterns.len() as f64;
        if total == 0.0 {
            return 0.0;
        }
        // Weighted score: strong patterns contribute more
        let score = (strong_patterns * 3.0 + moderate_patterns * 1.5) / (total * 3.0) * 100.0;
        (score * 100.0).round() / 100.0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_edit(accepted: bool, original: &str, final_code: &str) -> EditOutcome {
        EditOutcome {
            file_path: "src/main.rs".to_string(),
            accepted,
            original_code: original.to_string(),
            final_code: final_code.to_string(),
            user_correction: None,
        }
    }

    fn make_session(edits: Vec<EditOutcome>) -> SessionOutcome {
        SessionOutcome {
            session_id: "sess-1".to_string(),
            edits,
            started_at: 100,
            completed_at: 200,
        }
    }

    // --- PatternType tests ---

    #[test]
    fn test_pattern_type_equality() {
        assert_eq!(PatternType::LibraryPreference, PatternType::LibraryPreference);
        assert_ne!(PatternType::LibraryPreference, PatternType::NamingConvention);
    }

    #[test]
    fn test_pattern_type_custom() {
        let custom = PatternType::Custom("logging".to_string());
        assert_eq!(custom, PatternType::Custom("logging".to_string()));
        assert_ne!(custom, PatternType::Custom("other".to_string()));
    }

    #[test]
    fn test_pattern_type_hash_map_key() {
        let mut map: HashMap<PatternType, u32> = HashMap::new();
        map.insert(PatternType::CodeStyle, 1);
        map.insert(PatternType::Custom("x".to_string()), 2);
        assert_eq!(map.get(&PatternType::CodeStyle), Some(&1));
        assert_eq!(map.get(&PatternType::Custom("x".to_string())), Some(&2));
    }

    // --- PatternConfidence tests ---

    #[test]
    fn test_confidence_promote() {
        assert_eq!(PatternConfidence::Tentative.promote(), PatternConfidence::Weak);
        assert_eq!(PatternConfidence::Weak.promote(), PatternConfidence::Moderate);
        assert_eq!(PatternConfidence::Moderate.promote(), PatternConfidence::Strong);
        assert_eq!(PatternConfidence::Strong.promote(), PatternConfidence::Strong);
    }

    #[test]
    fn test_confidence_demote() {
        assert_eq!(PatternConfidence::Strong.demote(), PatternConfidence::Moderate);
        assert_eq!(PatternConfidence::Moderate.demote(), PatternConfidence::Weak);
        assert_eq!(PatternConfidence::Weak.demote(), PatternConfidence::Tentative);
        assert_eq!(PatternConfidence::Tentative.demote(), PatternConfidence::Tentative);
    }

    #[test]
    fn test_confidence_ordering() {
        assert!(PatternConfidence::Tentative < PatternConfidence::Weak);
        assert!(PatternConfidence::Weak < PatternConfidence::Moderate);
        assert!(PatternConfidence::Moderate < PatternConfidence::Strong);
    }

    // --- DistillConfig tests ---

    #[test]
    fn test_default_config() {
        let config = DistillConfig::default();
        assert_eq!(config.min_occurrences, 3);
        assert_eq!(config.min_confidence, PatternConfidence::Moderate);
        assert!(config.auto_distill);
        assert_eq!(config.max_patterns, 200);
        assert!(!config.ab_test_new_skills);
    }

    // --- PatternExtractor: detect_library_preference ---

    #[test]
    fn test_detect_rust_crate() {
        let code = "use serde::{Serialize, Deserialize};\nuse tokio::runtime;";
        let patterns = PatternExtractor::detect_library_preference(code);
        assert!(patterns.len() >= 2);
        assert!(patterns.iter().any(|p| p.rule == "use-crate:serde"));
        assert!(patterns.iter().any(|p| p.rule == "use-crate:tokio"));
    }

    #[test]
    fn test_detect_std_ignored() {
        let code = "use std::collections::HashMap;";
        let patterns = PatternExtractor::detect_library_preference(code);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_detect_js_import() {
        let code = r#"import React from "react";"#;
        let patterns = PatternExtractor::detect_library_preference(code);
        assert!(patterns.iter().any(|p| p.rule == "use-package:react"));
    }

    #[test]
    fn test_detect_relative_import_ignored() {
        let code = r#"import { foo } from "./utils";"#;
        let patterns = PatternExtractor::detect_library_preference(code);
        // Should not contain a library preference for a relative import
        assert!(!patterns.iter().any(|p| p.rule.contains("utils")));
    }

    #[test]
    fn test_detect_python_from_import() {
        let code = "from flask import Flask";
        let patterns = PatternExtractor::detect_library_preference(code);
        assert!(patterns.iter().any(|p| p.rule == "use-pymodule:flask"));
    }

    #[test]
    fn test_detect_python_plain_import() {
        let code = "import numpy";
        let patterns = PatternExtractor::detect_library_preference(code);
        assert!(patterns.iter().any(|p| p.rule == "use-pymodule:numpy"));
    }

    #[test]
    fn test_detect_scoped_npm_package() {
        let code = r#"import { Client } from "@anthropic-ai/sdk";"#;
        let patterns = PatternExtractor::detect_library_preference(code);
        assert!(patterns.iter().any(|p| p.rule == "use-package:@anthropic-ai"));
    }

    // --- PatternExtractor: detect_naming_convention ---

    #[test]
    fn test_detect_snake_case() {
        let code = "fn my_function() {}\npub fn another_one() {}";
        let patterns = PatternExtractor::detect_naming_convention(code);
        assert!(patterns.iter().any(|p| p.rule == "naming:snake_case"));
    }

    #[test]
    fn test_detect_camel_case() {
        let code = "function myFunction() {}\nconst anotherThing = 1;";
        let patterns = PatternExtractor::detect_naming_convention(code);
        assert!(patterns.iter().any(|p| p.rule == "naming:camelCase"));
    }

    #[test]
    fn test_no_naming_pattern_for_empty_code() {
        let code = "// just a comment";
        let patterns = PatternExtractor::detect_naming_convention(code);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_naming_convention_occurrences_count() {
        let code = "fn one_thing() {}\nfn two_thing() {}\nfn three_thing() {}";
        let patterns = PatternExtractor::detect_naming_convention(code);
        let snake = patterns.iter().find(|p| p.rule == "naming:snake_case").expect("should find snake_case");
        assert_eq!(snake.occurrences, 3);
    }

    // --- PatternExtractor: detect_file_organization ---

    #[test]
    fn test_detect_tests_directory() {
        let patterns = PatternExtractor::detect_file_organization("src/tests/foo_test.rs");
        assert!(patterns.iter().any(|p| p.rule == "file-org:tests-directory"));
    }

    #[test]
    fn test_detect_jest_tests() {
        let patterns = PatternExtractor::detect_file_organization("src/__tests__/App.test.tsx");
        assert!(patterns.iter().any(|p| p.rule == "file-org:jest-tests"));
        assert!(patterns.iter().any(|p| p.rule == "file-org:suffix-test-files"));
    }

    #[test]
    fn test_detect_go_tests() {
        let patterns = PatternExtractor::detect_file_organization("pkg/handler_test.go");
        assert!(patterns.iter().any(|p| p.rule == "file-org:go-colocated-tests"));
    }

    #[test]
    fn test_detect_spec_file() {
        let patterns = PatternExtractor::detect_file_organization("src/utils.spec.ts");
        assert!(patterns.iter().any(|p| p.rule == "file-org:suffix-test-files"));
    }

    #[test]
    fn test_detect_components_dir() {
        let patterns = PatternExtractor::detect_file_organization("src/components/Button.tsx");
        assert!(patterns.iter().any(|p| p.rule == "file-org:components-dir"));
    }

    #[test]
    fn test_detect_utils_dir() {
        let patterns = PatternExtractor::detect_file_organization("src/utils/format.ts");
        assert!(patterns.iter().any(|p| p.rule == "file-org:utils-dir"));
    }

    #[test]
    fn test_detect_helpers_dir() {
        let patterns = PatternExtractor::detect_file_organization("src/helpers/math.ts");
        assert!(patterns.iter().any(|p| p.rule == "file-org:utils-dir"));
    }

    #[test]
    fn test_no_org_pattern_for_plain_path() {
        let patterns = PatternExtractor::detect_file_organization("src/main.rs");
        assert!(patterns.is_empty());
    }

    // --- PatternExtractor: detect_error_handling ---

    #[test]
    fn test_detect_result_question_mark() {
        let code = "fn foo() -> Result<(), Error> {\n    let x = bar()?;\n    Ok(x)\n}";
        let patterns = PatternExtractor::detect_error_handling(code);
        assert!(patterns.iter().any(|p| p.rule == "error:result-question-mark"));
    }

    #[test]
    fn test_detect_expect_over_unwrap() {
        let code = "let f = File::open(\"x\").expect(\"open failed\");";
        let patterns = PatternExtractor::detect_error_handling(code);
        assert!(patterns.iter().any(|p| p.rule == "error:expect-over-unwrap"));
    }

    #[test]
    fn test_detect_try_catch() {
        let code = "try {\n  fetch(url);\n} catch(e) {\n  console.error(e);\n}";
        let patterns = PatternExtractor::detect_error_handling(code);
        assert!(patterns.iter().any(|p| p.rule == "error:try-catch"));
    }

    #[test]
    fn test_detect_liberal_unwrap() {
        let code = "let v = x.unwrap();\nlet w = y.unwrap();";
        let patterns = PatternExtractor::detect_error_handling(code);
        assert!(patterns.iter().any(|p| p.rule == "error:liberal-unwrap"));
    }

    #[test]
    fn test_no_error_pattern_for_clean_code() {
        let code = "let x = 42;\nprintln!(\"{}\", x);";
        let patterns = PatternExtractor::detect_error_handling(code);
        assert!(patterns.is_empty());
    }

    // --- PatternExtractor: extract_from_outcome ---

    #[test]
    fn test_extract_accepted_edit() {
        let edit = EditOutcome {
            file_path: "src/tests/lib.rs".to_string(),
            accepted: true,
            original_code: String::new(),
            final_code: "use serde::Serialize;\nfn my_func() {}".to_string(),
            user_correction: None,
        };
        let patterns = PatternExtractor::extract_from_outcome(&edit);
        assert!(!patterns.is_empty());
        assert!(patterns.iter().all(|p| p.source == PatternSource::Accepted));
    }

    #[test]
    fn test_extract_rejected_edit() {
        let edit = EditOutcome {
            file_path: "src/main.rs".to_string(),
            accepted: false,
            original_code: "use tokio::runtime;".to_string(),
            final_code: String::new(),
            user_correction: None,
        };
        let patterns = PatternExtractor::extract_from_outcome(&edit);
        assert!(patterns.iter().all(|p| p.source == PatternSource::Rejected));
    }

    #[test]
    fn test_extract_corrected_edit() {
        let edit = EditOutcome {
            file_path: "src/main.rs".to_string(),
            accepted: true,
            original_code: String::new(),
            final_code: "use anyhow::Result;".to_string(),
            user_correction: Some("Use anyhow not thiserror".to_string()),
        };
        let patterns = PatternExtractor::extract_from_outcome(&edit);
        assert!(patterns.iter().all(|p| p.source == PatternSource::Corrected));
    }

    // --- DistillationEngine: creation and reset ---

    #[test]
    fn test_engine_new() {
        let engine = DistillationEngine::new(DistillConfig::default());
        assert_eq!(engine.patterns.len(), 0);
        assert_eq!(engine.skills.len(), 0);
        assert_eq!(engine.metrics.sessions_analyzed, 0);
    }

    #[test]
    fn test_engine_reset() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "test".to_string(),
            rule: "test-rule".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 5,
            last_seen: 0,
            project_scope: None,
        });
        assert!(!engine.patterns.is_empty());
        engine.reset();
        assert!(engine.patterns.is_empty());
        assert_eq!(engine.metrics.sessions_analyzed, 0);
        assert_eq!(engine.next_id, 1);
    }

    // --- DistillationEngine: merge_pattern ---

    #[test]
    fn test_merge_new_pattern() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "test".to_string(),
            rule: "style:braces".to_string(),
            examples: vec!["ex1".to_string()],
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        assert_eq!(engine.patterns.len(), 1);
        let p = engine.patterns.get("style:braces").expect("pattern should exist");
        assert_eq!(p.id, "pat-1");
        assert_eq!(p.occurrences, 1);
    }

    #[test]
    fn test_merge_existing_pattern_increments() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        for _ in 0..3 {
            engine.merge_pattern(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::CodeStyle,
                description: "test".to_string(),
                rule: "style:braces".to_string(),
                examples: vec!["ex".to_string()],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        let p = engine.patterns.get("style:braces").expect("should exist");
        assert_eq!(p.occurrences, 3);
    }

    #[test]
    fn test_merge_auto_promotes_confidence() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 2,
            ..Default::default()
        });
        for _ in 0..3 {
            engine.merge_pattern(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::CodeStyle,
                description: "test".to_string(),
                rule: "r1".to_string(),
                examples: Vec::new(),
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        let p = engine.patterns.get("r1").expect("should exist");
        assert!(p.confidence >= PatternConfidence::Weak);
    }

    #[test]
    fn test_merge_examples_capped_at_5() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        for i in 0..10 {
            engine.merge_pattern(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::CodeStyle,
                description: "test".to_string(),
                rule: "r1".to_string(),
                examples: vec![format!("ex-{}", i)],
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        let p = engine.patterns.get("r1").expect("should exist");
        assert!(p.examples.len() <= 5);
    }

    #[test]
    fn test_merge_assigns_unique_ids() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "a".to_string(),
            rule: "r1".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::ErrorHandling,
            description: "b".to_string(),
            rule: "r2".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        let ids: Vec<&str> = engine.patterns.values().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"pat-1"));
        assert!(ids.contains(&"pat-2"));
    }

    // --- DistillationEngine: promote/demote ---

    #[test]
    fn test_promote_pattern() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "test".to_string(),
            rule: "r1".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        let id = engine.patterns.get("r1").expect("exists").id.clone();
        engine.promote_pattern(&id).expect("should succeed");
        assert_eq!(engine.patterns.get("r1").expect("exists").confidence, PatternConfidence::Weak);
        assert_eq!(engine.metrics.patterns_promoted, 1);
    }

    #[test]
    fn test_demote_pattern() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "test".to_string(),
            rule: "r1".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        let id = engine.patterns.get("r1").expect("exists").id.clone();
        engine.demote_pattern(&id).expect("should succeed");
        assert_eq!(engine.patterns.get("r1").expect("exists").confidence, PatternConfidence::Moderate);
        assert_eq!(engine.metrics.patterns_demoted, 1);
    }

    #[test]
    fn test_promote_nonexistent_fails() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        assert!(engine.promote_pattern("nonexistent").is_err());
    }

    #[test]
    fn test_demote_nonexistent_fails() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        assert!(engine.demote_pattern("nonexistent").is_err());
    }

    #[test]
    fn test_promote_already_strong_no_metric_change() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "test".to_string(),
            rule: "r1".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        let id = engine.patterns.get("r1").expect("exists").id.clone();
        engine.promote_pattern(&id).expect("ok");
        assert_eq!(engine.metrics.patterns_promoted, 0);
    }

    #[test]
    fn test_demote_already_tentative_no_metric_change() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "test".to_string(),
            rule: "r1".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        let id = engine.patterns.get("r1").expect("exists").id.clone();
        engine.demote_pattern(&id).expect("ok");
        assert_eq!(engine.metrics.patterns_demoted, 0);
    }

    // --- DistillationEngine: analyze_session ---

    #[test]
    fn test_analyze_session_updates_metrics() {
        let mut engine = DistillationEngine::new(DistillConfig {
            auto_distill: false,
            ..Default::default()
        });
        let session = make_session(vec![make_edit(
            true,
            "",
            "use serde::Serialize;\nfn my_func() {}",
        )]);
        engine.analyze_session(session);
        assert_eq!(engine.metrics.sessions_analyzed, 1);
        assert!(engine.metrics.patterns_extracted > 0);
    }

    #[test]
    fn test_analyze_multiple_sessions() {
        let mut engine = DistillationEngine::new(DistillConfig {
            auto_distill: false,
            ..Default::default()
        });
        for i in 0..5 {
            let session = SessionOutcome {
                session_id: format!("sess-{}", i),
                edits: vec![make_edit(true, "", "use tokio::runtime;\nfn my_func() {}")],
                started_at: 0,
                completed_at: 100,
            };
            engine.analyze_session(session);
        }
        assert_eq!(engine.metrics.sessions_analyzed, 5);
    }

    #[test]
    fn test_analyze_session_max_patterns_enforced() {
        let mut engine = DistillationEngine::new(DistillConfig {
            max_patterns: 2,
            auto_distill: false,
            ..Default::default()
        });
        for i in 0..5 {
            engine.merge_pattern(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::CodeStyle,
                description: format!("pat-{}", i),
                rule: format!("rule-{}", i),
                examples: Vec::new(),
                counter_examples: Vec::new(),
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        let session = make_session(vec![]);
        engine.analyze_session(session);
        assert!(engine.patterns.len() <= 2);
    }

    #[test]
    fn test_analyze_empty_session() {
        let mut engine = DistillationEngine::new(DistillConfig {
            auto_distill: false,
            ..Default::default()
        });
        let session = make_session(vec![]);
        engine.analyze_session(session);
        assert_eq!(engine.metrics.sessions_analyzed, 1);
        assert_eq!(engine.metrics.patterns_extracted, 0);
    }

    // --- DistillationEngine: distill_skills ---

    #[test]
    fn test_distill_skills_empty() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        let skills = engine.distill_skills();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_distill_skills_requires_min_occurrences() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 3,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "test".to_string(),
            rule: "style:x".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        let skills = engine.distill_skills();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_distill_skills_requires_min_confidence() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Moderate,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "test".to_string(),
            rule: "style:x".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 10,
            last_seen: 0,
            project_scope: None,
        });
        let skills = engine.distill_skills();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_distill_skills_generates_skill() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Moderate,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::ErrorHandling,
            description: "Result + ?".to_string(),
            rule: "error:result-question-mark".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Accepted,
            confidence: PatternConfidence::Strong,
            occurrences: 5,
            last_seen: 0,
            project_scope: None,
        });
        let skills = engine.distill_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "error-handling");
        assert!(skills[0].patterns.contains(&"error:result-question-mark".to_string()));
    }

    #[test]
    fn test_distill_groups_by_type() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Moderate,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::ErrorHandling,
            description: "a".to_string(),
            rule: "error:a".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Accepted,
            confidence: PatternConfidence::Strong,
            occurrences: 5,
            last_seen: 0,
            project_scope: None,
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::NamingConvention,
            description: "b".to_string(),
            rule: "naming:b".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Accepted,
            confidence: PatternConfidence::Moderate,
            occurrences: 3,
            last_seen: 0,
            project_scope: None,
        });
        let skills = engine.distill_skills();
        assert_eq!(skills.len(), 2);
    }

    // --- DistillationEngine: get_patterns / get_patterns_by_type ---

    #[test]
    fn test_get_patterns() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "a".to_string(),
            rule: "a".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        assert_eq!(engine.get_patterns().len(), 1);
    }

    #[test]
    fn test_get_patterns_by_type() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "a".to_string(),
            rule: "a".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::ErrorHandling,
            description: "b".to_string(),
            rule: "b".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        assert_eq!(engine.get_patterns_by_type(&PatternType::CodeStyle).len(), 1);
        assert_eq!(engine.get_patterns_by_type(&PatternType::ErrorHandling).len(), 1);
        assert_eq!(engine.get_patterns_by_type(&PatternType::LibraryPreference).len(), 0);
    }

    // --- DistillationEngine: export_skills ---

    #[test]
    fn test_export_skills_markdown() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Moderate,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "braces style".to_string(),
            rule: "style:braces".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 10,
            last_seen: 0,
            project_scope: None,
        });
        engine.distill_skills();
        let md = engine.export_skills();
        assert!(md.contains("# Distilled Skills"));
        assert!(md.contains("code-style"));
        assert!(md.contains("`style:braces`"));
    }

    #[test]
    fn test_export_skills_empty() {
        let engine = DistillationEngine::new(DistillConfig::default());
        let md = engine.export_skills();
        assert!(md.contains("# Distilled Skills"));
        assert!(md.contains("0 sessions"));
    }

    // --- DistillationEngine: improvement_estimate ---

    #[test]
    fn test_improvement_estimate_zero_sessions() {
        let engine = DistillationEngine::new(DistillConfig::default());
        assert_eq!(engine.improvement_estimate(), 0.0);
    }

    #[test]
    fn test_improvement_estimate_no_patterns() {
        let mut engine = DistillationEngine::new(DistillConfig {
            auto_distill: false,
            ..Default::default()
        });
        engine.metrics.sessions_analyzed = 1;
        assert_eq!(engine.improvement_estimate(), 0.0);
    }

    #[test]
    fn test_improvement_estimate_all_strong() {
        let mut engine = DistillationEngine::new(DistillConfig {
            auto_distill: false,
            ..Default::default()
        });
        engine.metrics.sessions_analyzed = 5;
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "a".to_string(),
            rule: "a".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 10,
            last_seen: 0,
            project_scope: None,
        });
        let score = engine.improvement_estimate();
        assert!(score > 90.0);
    }

    #[test]
    fn test_improvement_estimate_mixed() {
        let mut engine = DistillationEngine::new(DistillConfig {
            auto_distill: false,
            ..Default::default()
        });
        engine.metrics.sessions_analyzed = 5;
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "strong".to_string(),
            rule: "r1".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 10,
            last_seen: 0,
            project_scope: None,
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "tentative".to_string(),
            rule: "r2".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        let score = engine.improvement_estimate();
        assert!(score > 0.0 && score < 100.0);
    }

    // --- DistillationEngine: get_metrics ---

    #[test]
    fn test_get_metrics() {
        let engine = DistillationEngine::new(DistillConfig::default());
        let m = engine.get_metrics();
        assert_eq!(m.sessions_analyzed, 0);
        assert_eq!(m.patterns_extracted, 0);
        assert_eq!(m.skills_generated, 0);
    }

    // --- DistillationEngine: get_skills ---

    #[test]
    fn test_get_skills_empty() {
        let engine = DistillationEngine::new(DistillConfig::default());
        assert!(engine.get_skills().is_empty());
    }

    #[test]
    fn test_get_skills_after_distill() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Tentative,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::ArchitecturePattern,
            description: "MVC".to_string(),
            rule: "arch:mvc".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Moderate,
            occurrences: 5,
            last_seen: 0,
            project_scope: None,
        });
        engine.distill_skills();
        assert_eq!(engine.get_skills().len(), 1);
        assert_eq!(engine.get_skills()[0].name, "architecture-patterns");
    }

    // --- Serialization tests ---

    #[test]
    fn test_pattern_type_serde() {
        let pt = PatternType::Custom("logging".to_string());
        let json = serde_json::to_string(&pt).expect("serialize");
        let back: PatternType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(pt, back);
    }

    #[test]
    fn test_learned_pattern_serde() {
        let p = LearnedPattern {
            id: "pat-1".to_string(),
            pattern_type: PatternType::LibraryPreference,
            description: "test".to_string(),
            rule: "use-crate:serde".to_string(),
            examples: vec!["use serde::Serialize;".to_string()],
            counter_examples: Vec::new(),
            source: PatternSource::Accepted,
            confidence: PatternConfidence::Strong,
            occurrences: 5,
            last_seen: 100,
            project_scope: Some("my-project".to_string()),
        };
        let json = serde_json::to_string(&p).expect("serialize");
        let back: LearnedPattern = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, "pat-1");
        assert_eq!(back.occurrences, 5);
    }

    #[test]
    fn test_distilled_skill_serde() {
        let s = DistilledSkill {
            name: "test-skill".to_string(),
            description: "desc".to_string(),
            patterns: vec!["r1".to_string()],
            trigger_words: vec!["tw".to_string()],
            generated_at: 42,
            version: 1,
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: DistilledSkill = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "test-skill");
    }

    #[test]
    fn test_distill_config_serde() {
        let c = DistillConfig::default();
        let json = serde_json::to_string(&c).expect("serialize");
        let back: DistillConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.min_occurrences, 3);
    }

    #[test]
    fn test_session_outcome_serde() {
        let s = SessionOutcome {
            session_id: "s1".to_string(),
            edits: vec![],
            started_at: 0,
            completed_at: 100,
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: SessionOutcome = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.session_id, "s1");
    }

    #[test]
    fn test_distill_metrics_serde() {
        let m = DistillMetrics {
            sessions_analyzed: 10,
            patterns_extracted: 50,
            skills_generated: 3,
            patterns_promoted: 5,
            patterns_demoted: 2,
            improvement_score: 75.5,
        };
        let json = serde_json::to_string(&m).expect("serialize");
        let back: DistillMetrics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.sessions_analyzed, 10);
        assert_eq!(back.improvement_score, 75.5);
    }

    // --- Integration / end-to-end tests ---

    #[test]
    fn test_end_to_end_learn_and_distill() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 2,
            min_confidence: PatternConfidence::Weak,
            auto_distill: false,
            ..Default::default()
        });

        for i in 0..3 {
            let session = SessionOutcome {
                session_id: format!("sess-{}", i),
                edits: vec![EditOutcome {
                    file_path: "src/tests/handler.rs".to_string(),
                    accepted: true,
                    original_code: String::new(),
                    final_code: "use tokio::runtime;\nfn my_handler() -> Result<(), Box<dyn std::error::Error>> {\n    let rt = tokio::runtime::Runtime::new()?;\n    Ok(())\n}".to_string(),
                    user_correction: None,
                }],
                started_at: i * 100,
                completed_at: i * 100 + 50,
            };
            engine.analyze_session(session);
        }

        assert_eq!(engine.metrics.sessions_analyzed, 3);
        assert!(!engine.get_patterns().is_empty());

        let skills = engine.distill_skills();
        assert!(!skills.is_empty() || engine.get_patterns().iter().all(|p| p.occurrences < 2));

        let md = engine.export_skills();
        assert!(md.contains("# Distilled Skills"));
    }

    #[test]
    fn test_auto_distill_on_session() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Tentative,
            auto_distill: true,
            ..Default::default()
        });
        let session = SessionOutcome {
            session_id: "auto-1".to_string(),
            edits: vec![EditOutcome {
                file_path: "src/components/App.tsx".to_string(),
                accepted: true,
                original_code: String::new(),
                final_code: "use serde::Serialize;".to_string(),
                user_correction: None,
            }],
            started_at: 0,
            completed_at: 50,
        };
        engine.analyze_session(session);
        // Auto-distill should have run
        assert!(engine.metrics.skills_generated > 0 || engine.get_patterns().is_empty());
    }

    #[test]
    fn test_project_scope_preserved() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::ConfigPreference,
            description: "eslint flat config".to_string(),
            rule: "config:eslint-flat".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Moderate,
            occurrences: 1,
            last_seen: 0,
            project_scope: Some("my-frontend".to_string()),
        });
        let p = engine.patterns.get("config:eslint-flat").expect("should exist");
        assert_eq!(p.project_scope, Some("my-frontend".to_string()));
    }

    #[test]
    fn test_counter_examples_capped() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        for i in 0..10 {
            engine.merge_pattern(LearnedPattern {
                id: String::new(),
                pattern_type: PatternType::CodeStyle,
                description: "test".to_string(),
                rule: "cex-test".to_string(),
                examples: Vec::new(),
                counter_examples: vec![format!("cex-{}", i)],
                source: PatternSource::Observed,
                confidence: PatternConfidence::Tentative,
                occurrences: 1,
                last_seen: 0,
                project_scope: None,
            });
        }
        let p = engine.patterns.get("cex-test").expect("exists");
        assert!(p.counter_examples.len() <= 5);
    }

    #[test]
    fn test_distill_custom_pattern_type_name() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Tentative,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::Custom("logging".to_string()),
            description: "structured logging".to_string(),
            rule: "logging:structured".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 5,
            last_seen: 0,
            project_scope: None,
        });
        let skills = engine.distill_skills();
        assert!(skills.iter().any(|s| s.name == "custom-logging"));
    }

    #[test]
    fn test_metrics_after_full_workflow() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Tentative,
            auto_distill: false,
            ..Default::default()
        });

        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::TestStyle,
            description: "test".to_string(),
            rule: "test:arrange-act-assert".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 5,
            last_seen: 0,
            project_scope: None,
        });

        let id = engine.patterns.get("test:arrange-act-assert").expect("exists").id.clone();
        engine.promote_pattern(&id).expect("ok");
        engine.promote_pattern(&id).expect("ok");
        engine.demote_pattern(&id).expect("ok");
        engine.distill_skills();

        let m = engine.get_metrics();
        assert_eq!(m.patterns_promoted, 2);
        assert_eq!(m.patterns_demoted, 1);
    }

    #[test]
    fn test_timestamp_counter_increments() {
        let mut engine = DistillationEngine::new(DistillConfig::default());
        let initial = engine.timestamp_counter;
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::CodeStyle,
            description: "a".to_string(),
            rule: "a".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Tentative,
            occurrences: 1,
            last_seen: 0,
            project_scope: None,
        });
        assert!(engine.timestamp_counter > initial);
    }

    #[test]
    fn test_skill_version_is_one() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Tentative,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::LibraryPreference,
            description: "x".to_string(),
            rule: "lib:x".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 5,
            last_seen: 0,
            project_scope: None,
        });
        let skills = engine.distill_skills();
        assert!(skills.iter().all(|s| s.version == 1));
    }

    #[test]
    fn test_skill_trigger_words_populated() {
        let mut engine = DistillationEngine::new(DistillConfig {
            min_occurrences: 1,
            min_confidence: PatternConfidence::Tentative,
            auto_distill: false,
            ..Default::default()
        });
        engine.merge_pattern(LearnedPattern {
            id: String::new(),
            pattern_type: PatternType::LibraryPreference,
            description: "serde".to_string(),
            rule: "use-crate:serde".to_string(),
            examples: Vec::new(),
            counter_examples: Vec::new(),
            source: PatternSource::Observed,
            confidence: PatternConfidence::Strong,
            occurrences: 5,
            last_seen: 0,
            project_scope: None,
        });
        let skills = engine.distill_skills();
        assert!(skills[0].trigger_words.contains(&"serde".to_string()));
    }
}
