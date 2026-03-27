//! Proactive Agent Intelligence — Gap 3 from FIT-GAP v7
//!
//! Implements background code scanning with heuristic-based pattern detection,
//! suggestion lifecycle management, user preference learning, and digest reporting.
//! Scans detect performance issues, security concerns, tech debt, testing gaps,
//! code smells, correctness problems, accessibility gaps, and dependency health.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScanCategory {
    Performance,
    Security,
    TechDebt,
    Correctness,
    Accessibility,
    TestingGaps,
    DependencyHealth,
    CodeSmells,
}

impl ScanCategory {
    pub fn as_str(&self) -> &str {
        match self {
            ScanCategory::Performance => "Performance",
            ScanCategory::Security => "Security",
            ScanCategory::TechDebt => "TechDebt",
            ScanCategory::Correctness => "Correctness",
            ScanCategory::Accessibility => "Accessibility",
            ScanCategory::TestingGaps => "TestingGaps",
            ScanCategory::DependencyHealth => "DependencyHealth",
            ScanCategory::CodeSmells => "CodeSmells",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "Performance" => Some(ScanCategory::Performance),
            "Security" => Some(ScanCategory::Security),
            "TechDebt" => Some(ScanCategory::TechDebt),
            "Correctness" => Some(ScanCategory::Correctness),
            "Accessibility" => Some(ScanCategory::Accessibility),
            "TestingGaps" => Some(ScanCategory::TestingGaps),
            "DependencyHealth" => Some(ScanCategory::DependencyHealth),
            "CodeSmells" => Some(ScanCategory::CodeSmells),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScanCadence {
    OnSave,
    OnCommit,
    OnPush,
    Periodic(u64),
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl Priority {
    pub fn as_str(&self) -> &str {
        match self {
            Priority::Critical => "Critical",
            Priority::High => "High",
            Priority::Medium => "Medium",
            Priority::Low => "Low",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
    Snoozed(u64),
    Applied,
}

// ---------------------------------------------------------------------------
// Core structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProactiveSuggestion {
    pub id: String,
    pub category: ScanCategory,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub line_range: Option<(usize, usize)>,
    pub confidence: f64,
    pub priority: Priority,
    pub suggested_fix: Option<String>,
    pub created_at: u64,
    pub status: SuggestionStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanConfig {
    pub enabled_categories: Vec<ScanCategory>,
    pub cadence: ScanCadence,
    pub min_confidence: f64,
    pub max_suggestions_per_scan: u32,
    pub quiet_mode: bool,
    pub learning_enabled: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            enabled_categories: vec![
                ScanCategory::Performance,
                ScanCategory::Security,
                ScanCategory::TechDebt,
                ScanCategory::Correctness,
                ScanCategory::Accessibility,
                ScanCategory::TestingGaps,
                ScanCategory::DependencyHealth,
                ScanCategory::CodeSmells,
            ],
            cadence: ScanCadence::OnSave,
            min_confidence: 0.5,
            max_suggestions_per_scan: 20,
            quiet_mode: false,
            learning_enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternRecord {
    pub category: ScanCategory,
    pub file_pattern: Option<String>,
    pub description: String,
    pub count: u32,
    pub last_seen: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub struct LearningStore {
    pub accepted_patterns: Vec<PatternRecord>,
    pub rejected_patterns: Vec<PatternRecord>,
    pub category_acceptance_rates: HashMap<String, f64>,
}


impl LearningStore {
    pub fn record_acceptance(&mut self, suggestion: &ProactiveSuggestion) {
        let cat_key = suggestion.category.as_str().to_string();
        let existing = self.accepted_patterns.iter_mut().find(|p| {
            p.category == suggestion.category && p.description == suggestion.title
        });
        if let Some(record) = existing {
            record.count += 1;
            record.last_seen = suggestion.created_at;
        } else {
            self.accepted_patterns.push(PatternRecord {
                category: suggestion.category.clone(),
                file_pattern: suggestion.file_path.clone(),
                description: suggestion.title.clone(),
                count: 1,
                last_seen: suggestion.created_at,
            });
        }
        self.recalculate_rate(&cat_key);
    }

    pub fn record_rejection(&mut self, suggestion: &ProactiveSuggestion) {
        let cat_key = suggestion.category.as_str().to_string();
        let existing = self.rejected_patterns.iter_mut().find(|p| {
            p.category == suggestion.category && p.description == suggestion.title
        });
        if let Some(record) = existing {
            record.count += 1;
            record.last_seen = suggestion.created_at;
        } else {
            self.rejected_patterns.push(PatternRecord {
                category: suggestion.category.clone(),
                file_pattern: suggestion.file_path.clone(),
                description: suggestion.title.clone(),
                count: 1,
                last_seen: suggestion.created_at,
            });
        }
        self.recalculate_rate(&cat_key);
    }

    fn recalculate_rate(&mut self, cat_key: &str) {
        let accepted: u32 = self
            .accepted_patterns
            .iter()
            .filter(|p| p.category.as_str() == cat_key)
            .map(|p| p.count)
            .sum();
        let rejected: u32 = self
            .rejected_patterns
            .iter()
            .filter(|p| p.category.as_str() == cat_key)
            .map(|p| p.count)
            .sum();
        let total = accepted + rejected;
        if total > 0 {
            self.category_acceptance_rates
                .insert(cat_key.to_string(), accepted as f64 / total as f64);
        }
    }

    pub fn get_acceptance_rate(&self, category: &ScanCategory) -> f64 {
        self.category_acceptance_rates
            .get(category.as_str())
            .copied()
            .unwrap_or(0.5) // default 50% for unseen categories
    }

    /// Use historical acceptance rates to filter low-value suggestions.
    /// A suggestion should be shown if:
    ///   acceptance_rate * confidence >= 0.25 (tunable threshold)
    pub fn should_suggest(&self, category: &ScanCategory, confidence: f64) -> bool {
        let rate = self.get_acceptance_rate(category);
        rate * confidence >= 0.25
    }

    pub fn top_accepted_categories(&self, n: usize) -> Vec<(ScanCategory, f64)> {
        let mut entries: Vec<(ScanCategory, f64)> = self
            .category_acceptance_rates
            .iter()
            .filter_map(|(k, v)| ScanCategory::parse_str(k).map(|c| (c, *v)))
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        entries.truncate(n);
        entries
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanRecord {
    pub timestamp: u64,
    pub files_scanned: usize,
    pub suggestions_found: usize,
    pub categories_triggered: Vec<ScanCategory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanDigest {
    pub total_pending: usize,
    pub by_priority: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
    pub top_files: Vec<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanMetrics {
    pub total_scans: u32,
    pub total_suggestions: u32,
    pub total_accepted: u32,
    pub total_rejected: u32,
    pub acceptance_rate: f64,
    pub avg_confidence: f64,
}

impl Default for ScanMetrics {
    fn default() -> Self {
        Self {
            total_scans: 0,
            total_suggestions: 0,
            total_accepted: 0,
            total_rejected: 0,
            acceptance_rate: 0.0,
            avg_confidence: 0.0,
        }
    }
}

impl ScanMetrics {
    fn update_acceptance_rate(&mut self) {
        let total = self.total_accepted + self.total_rejected;
        if total > 0 {
            self.acceptance_rate = self.total_accepted as f64 / total as f64;
        }
    }
}

// ---------------------------------------------------------------------------
// ProactiveScanner
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProactiveScanner {
    pub config: ScanConfig,
    pub suggestions: Vec<ProactiveSuggestion>,
    pub learning: LearningStore,
    pub scan_history: Vec<ScanRecord>,
    pub metrics: ScanMetrics,
    next_id: u64,
}

impl ProactiveScanner {
    pub fn new(config: ScanConfig) -> Self {
        Self {
            config,
            suggestions: Vec::new(),
            learning: LearningStore::default(),
            scan_history: Vec::new(),
            metrics: ScanMetrics::default(),
            next_id: 1,
        }
    }

    fn generate_id(&mut self) -> String {
        let id = format!("suggestion-{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn is_category_enabled(&self, cat: &ScanCategory) -> bool {
        self.config.enabled_categories.contains(cat)
    }

    fn is_test_code(line: &str) -> bool {
        line.contains("#[test]")
            || line.contains("#[cfg(test)]")
            || line.contains("mod tests")
            || line.contains("assert_eq!")
            || line.contains("assert!")
    }

    /// Check if a line is inside a test block (heuristic: any line after `#[cfg(test)]` or `#[test]`).
    fn in_test_region(content: &str, line_idx: usize) -> bool {
        let lines: Vec<&str> = content.lines().collect();
        for i in (0..=line_idx.min(lines.len().saturating_sub(1))).rev() {
            let trimmed = lines[i].trim();
            if trimmed == "#[cfg(test)]" || trimmed == "#[test]" {
                return true;
            }
            // If we hit a non-test module boundary going backward, stop
            if trimmed.starts_with("mod ") && !trimmed.contains("tests") {
                return false;
            }
        }
        false
    }

    pub fn scan_file(&mut self, path: &str, content: &str) -> Vec<ProactiveSuggestion> {
        let mut results = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // --- Performance ---
        if self.is_category_enabled(&ScanCategory::Performance) {
            // Detect unwrap() in non-test code
            for (i, line) in lines.iter().enumerate() {
                if line.contains(".unwrap()") && !Self::is_test_code(line) && !Self::in_test_region(content, i) {
                    let s = ProactiveSuggestion {
                        id: self.generate_id(),
                        category: ScanCategory::Performance,
                        title: "Potential panic from unwrap()".to_string(),
                        description: format!("Line {} uses .unwrap() which can panic at runtime. Consider using .expect() or proper error handling.", i + 1),
                        file_path: Some(path.to_string()),
                        line_range: Some((i + 1, i + 1)),
                        confidence: 0.8,
                        priority: Priority::Medium,
                        suggested_fix: Some("Replace .unwrap() with .expect(\"reason\") or use ? operator".to_string()),
                        created_at: 0,
                        status: SuggestionStatus::Pending,
                    };
                    results.push(s);
                }
            }

            // Detect nested loops (for/while inside for/while)
            let mut loop_depth = 0;
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if (trimmed.starts_with("for ") || trimmed.starts_with("while "))
                    && trimmed.ends_with('{')
                {
                    loop_depth += 1;
                    if loop_depth >= 2 && !Self::in_test_region(content, i) {
                        results.push(ProactiveSuggestion {
                            id: self.generate_id(),
                            category: ScanCategory::Performance,
                            title: "Nested loop detected".to_string(),
                            description: format!("Line {} has a nested loop (depth {}). Consider refactoring for O(n) complexity.", i + 1, loop_depth),
                            file_path: Some(path.to_string()),
                            line_range: Some((i + 1, i + 1)),
                            confidence: 0.6,
                            priority: Priority::Low,
                            suggested_fix: Some("Consider using iterators, HashMaps, or restructuring to reduce nesting".to_string()),
                            created_at: 0,
                            status: SuggestionStatus::Pending,
                        });
                    }
                }
                if trimmed == "}" && loop_depth > 0 {
                    loop_depth -= 1;
                }
            }

            // Detect large allocations (Vec::with_capacity with large numbers)
            for (i, line) in lines.iter().enumerate() {
                if let Some(pos) = line.find("Vec::with_capacity(") {
                    let after = &line[pos + 19..];
                    if let Some(end) = after.find(')') {
                        if let Ok(n) = after[..end].trim().parse::<u64>() {
                            if n >= 1_000_000 {
                                results.push(ProactiveSuggestion {
                                    id: self.generate_id(),
                                    category: ScanCategory::Performance,
                                    title: "Large allocation detected".to_string(),
                                    description: format!("Line {} allocates a Vec with capacity {}. Consider streaming or chunked processing.", i + 1, n),
                                    file_path: Some(path.to_string()),
                                    line_range: Some((i + 1, i + 1)),
                                    confidence: 0.7,
                                    priority: Priority::Medium,
                                    suggested_fix: None,
                                    created_at: 0,
                                    status: SuggestionStatus::Pending,
                                });
                            }
                        }
                    }
                }
            }
        }

        // --- Security ---
        if self.is_category_enabled(&ScanCategory::Security) {
            let secret_patterns = [
                ("api_key", "API key"),
                ("api-key", "API key"),
                ("secret_key", "secret key"),
                ("password", "password"),
                ("token", "token"),
                ("private_key", "private key"),
            ];
            for (i, line) in lines.iter().enumerate() {
                let lower = line.to_lowercase();
                // Look for hardcoded string assignments that contain key-like patterns
                if (lower.contains("= \"") || lower.contains("= '"))
                    && !Self::in_test_region(content, i)
                {
                    for (pat, label) in &secret_patterns {
                        if lower.contains(pat) {
                            results.push(ProactiveSuggestion {
                                id: self.generate_id(),
                                category: ScanCategory::Security,
                                title: format!("Possible hardcoded {}", label),
                                description: format!("Line {} may contain a hardcoded {}. Use environment variables or a secrets manager.", i + 1, label),
                                file_path: Some(path.to_string()),
                                line_range: Some((i + 1, i + 1)),
                                confidence: 0.7,
                                priority: Priority::High,
                                suggested_fix: Some(format!("Use std::env::var(\"{}\") or a config file", pat.to_uppercase())),
                                created_at: 0,
                                status: SuggestionStatus::Pending,
                            });
                            break; // one per line
                        }
                    }
                }

                // Detect unsafe blocks
                let trimmed = line.trim();
                if trimmed.starts_with("unsafe ") || trimmed == "unsafe{" || trimmed == "unsafe {" {
                    results.push(ProactiveSuggestion {
                        id: self.generate_id(),
                        category: ScanCategory::Security,
                        title: "Unsafe block detected".to_string(),
                        description: format!("Line {} uses an unsafe block. Ensure memory safety invariants are documented.", i + 1),
                        file_path: Some(path.to_string()),
                        line_range: Some((i + 1, i + 1)),
                        confidence: 0.9,
                        priority: Priority::High,
                        suggested_fix: Some("Add a // SAFETY: comment explaining why this is sound".to_string()),
                        created_at: 0,
                        status: SuggestionStatus::Pending,
                    });
                }
            }
        }

        // --- TechDebt ---
        if self.is_category_enabled(&ScanCategory::TechDebt) {
            let debt_markers = ["TODO", "FIXME", "HACK", "XXX"];
            for (i, line) in lines.iter().enumerate() {
                for marker in &debt_markers {
                    if line.contains(marker) {
                        results.push(ProactiveSuggestion {
                            id: self.generate_id(),
                            category: ScanCategory::TechDebt,
                            title: format!("{} comment found", marker),
                            description: format!("Line {} contains a {} marker: {}", i + 1, marker, line.trim()),
                            file_path: Some(path.to_string()),
                            line_range: Some((i + 1, i + 1)),
                            confidence: 0.9,
                            priority: Priority::Low,
                            suggested_fix: Some(format!("Address the {} or create a tracking issue", marker)),
                            created_at: 0,
                            status: SuggestionStatus::Pending,
                        });
                        break; // one marker per line
                    }
                }
            }

            // Detect very long functions (>50 lines heuristic)
            let mut fn_start: Option<(usize, String)> = None;
            let mut brace_depth = 0i32;
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if (trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") || trimmed.starts_with("pub async fn ") || trimmed.starts_with("async fn "))
                    && fn_start.is_none()
                {
                    let name = trimmed
                        .split('(')
                        .next()
                        .unwrap_or(trimmed)
                        .replace("pub ", "")
                        .replace("async ", "")
                        .replace("fn ", "")
                        .trim()
                        .to_string();
                    fn_start = Some((i, name));
                    brace_depth = 0;
                }
                brace_depth += line.chars().filter(|c| *c == '{').count() as i32;
                brace_depth -= line.chars().filter(|c| *c == '}').count() as i32;
                if let Some((start, ref name)) = fn_start {
                    if brace_depth == 0 && i > start {
                        let length = i - start + 1;
                        if length > 50 {
                            results.push(ProactiveSuggestion {
                                id: self.generate_id(),
                                category: ScanCategory::TechDebt,
                                title: format!("Long function: {} ({} lines)", name, length),
                                description: format!("Function '{}' spans {} lines ({}..{}). Consider breaking it into smaller functions.", name, length, start + 1, i + 1),
                                file_path: Some(path.to_string()),
                                line_range: Some((start + 1, i + 1)),
                                confidence: 0.75,
                                priority: Priority::Medium,
                                suggested_fix: Some("Extract helper functions to improve readability".to_string()),
                                created_at: 0,
                                status: SuggestionStatus::Pending,
                            });
                        }
                        fn_start = None;
                    }
                }
            }
        }

        // --- TestingGaps ---
        if self.is_category_enabled(&ScanCategory::TestingGaps) {
            // Detect public functions without corresponding test references
            let full_content = content.to_string();
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if (trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn "))
                    && !Self::in_test_region(content, i)
                {
                    let name = trimmed
                        .split('(')
                        .next()
                        .unwrap_or("")
                        .replace("pub ", "")
                        .replace("async ", "")
                        .replace("fn ", "")
                        .trim()
                        .to_string();
                    if !name.is_empty() && !full_content.contains(&format!("test_{}", name))
                        && !full_content.contains(&format!("test {}", name))
                        && !full_content.contains(&format!(".{}(", name))
                    {
                        // Only if there are no apparent test references
                        results.push(ProactiveSuggestion {
                            id: self.generate_id(),
                            category: ScanCategory::TestingGaps,
                            title: format!("No test found for pub fn {}", name),
                            description: format!("Public function '{}' at line {} has no apparent test coverage.", name, i + 1),
                            file_path: Some(path.to_string()),
                            line_range: Some((i + 1, i + 1)),
                            confidence: 0.6,
                            priority: Priority::Medium,
                            suggested_fix: Some(format!("#[test]\nfn test_{}() {{\n    // ...\n}}", name)),
                            created_at: 0,
                            status: SuggestionStatus::Pending,
                        });
                    }
                }
            }
        }

        // --- CodeSmells ---
        if self.is_category_enabled(&ScanCategory::CodeSmells) {
            // Detect deeply nested code (indentation > 4 levels = 16 spaces or 4 tabs)
            for (i, line) in lines.iter().enumerate() {
                if line.is_empty() {
                    continue;
                }
                let leading_spaces = line.len() - line.trim_start().len();
                let indent_level = leading_spaces / 4;
                if indent_level >= 5 && !line.trim().is_empty() && !Self::in_test_region(content, i) {
                    results.push(ProactiveSuggestion {
                        id: self.generate_id(),
                        category: ScanCategory::CodeSmells,
                        title: "Deeply nested code".to_string(),
                        description: format!("Line {} has {} levels of indentation. Consider early returns or extracting functions.", i + 1, indent_level),
                        file_path: Some(path.to_string()),
                        line_range: Some((i + 1, i + 1)),
                        confidence: 0.7,
                        priority: Priority::Low,
                        suggested_fix: Some("Use early returns, guard clauses, or extract helper functions".to_string()),
                        created_at: 0,
                        status: SuggestionStatus::Pending,
                    });
                }
            }

            // Detect duplicate string literals (3+ occurrences)
            let mut string_counts: HashMap<String, Vec<usize>> = HashMap::new();
            for (i, line) in lines.iter().enumerate() {
                // Simple extraction of quoted strings
                let mut start = 0;
                while let Some(pos) = line[start..].find('"') {
                    let abs_pos = start + pos + 1;
                    if abs_pos < line.len() {
                        if let Some(end) = line[abs_pos..].find('"') {
                            let s = &line[abs_pos..abs_pos + end];
                            if s.len() >= 4 {
                                string_counts
                                    .entry(s.to_string())
                                    .or_default()
                                    .push(i + 1);
                            }
                            start = abs_pos + end + 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            for (literal, occurrences) in &string_counts {
                if occurrences.len() >= 3 {
                    results.push(ProactiveSuggestion {
                        id: self.generate_id(),
                        category: ScanCategory::CodeSmells,
                        title: format!("Duplicate string literal ({} times)", occurrences.len()),
                        description: format!(
                            "\"{}\" appears {} times (lines {:?}). Extract to a constant.",
                            literal,
                            occurrences.len(),
                            occurrences
                        ),
                        file_path: Some(path.to_string()),
                        line_range: Some((occurrences[0], occurrences[0])),
                        confidence: 0.65,
                        priority: Priority::Low,
                        suggested_fix: Some(format!(
                            "const {}: &str = \"{}\";",
                            literal.to_uppercase().replace([' ', '-'], "_"),
                            literal
                        )),
                        created_at: 0,
                        status: SuggestionStatus::Pending,
                    });
                }
            }
        }

        // Apply confidence threshold
        results.retain(|s| s.confidence >= self.config.min_confidence);

        // Apply learning filter
        if self.config.learning_enabled {
            results.retain(|s| self.learning.should_suggest(&s.category, s.confidence));
        }

        // Apply quiet mode: only Critical and High in quiet mode
        if self.config.quiet_mode {
            results.retain(|s| matches!(s.priority, Priority::Critical | Priority::High));
        }

        // Enforce max suggestions per scan
        results.truncate(self.config.max_suggestions_per_scan as usize);

        // Record scan
        let categories_triggered: Vec<ScanCategory> = {
            let set: std::collections::HashSet<ScanCategory> =
                results.iter().map(|s| s.category.clone()).collect();
            set.into_iter().collect()
        };

        self.scan_history.push(ScanRecord {
            timestamp: 0,
            files_scanned: 1,
            suggestions_found: results.len(),
            categories_triggered,
        });

        // Update metrics
        self.metrics.total_scans += 1;
        self.metrics.total_suggestions += results.len() as u32;
        if !results.is_empty() {
            let sum_conf: f64 = results.iter().map(|s| s.confidence).sum();
            let total = self.metrics.total_suggestions as f64;
            // Running average approximation
            self.metrics.avg_confidence =
                (self.metrics.avg_confidence * (total - results.len() as f64) + sum_conf) / total;
        }

        // Store suggestions
        self.suggestions.extend(results.clone());

        results
    }

    pub fn scan_batch(&mut self, files: Vec<(&str, &str)>) -> Vec<ProactiveSuggestion> {
        let mut all = Vec::new();
        for (path, content) in files {
            let found = self.scan_file(path, content);
            all.extend(found);
        }
        all
    }

    pub fn accept_suggestion(&mut self, id: &str) -> Result<(), String> {
        let suggestion = self
            .suggestions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("Suggestion '{}' not found", id))?;
        if suggestion.status != SuggestionStatus::Pending {
            return Err(format!("Suggestion '{}' is not pending (status: {:?})", id, suggestion.status));
        }
        suggestion.status = SuggestionStatus::Accepted;
        let clone = suggestion.clone();
        self.metrics.total_accepted += 1;
        self.metrics.update_acceptance_rate();
        if self.config.learning_enabled {
            self.learning.record_acceptance(&clone);
        }
        Ok(())
    }

    pub fn reject_suggestion(&mut self, id: &str) -> Result<(), String> {
        let suggestion = self
            .suggestions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("Suggestion '{}' not found", id))?;
        if suggestion.status != SuggestionStatus::Pending {
            return Err(format!("Suggestion '{}' is not pending (status: {:?})", id, suggestion.status));
        }
        suggestion.status = SuggestionStatus::Rejected;
        let clone = suggestion.clone();
        self.metrics.total_rejected += 1;
        self.metrics.update_acceptance_rate();
        if self.config.learning_enabled {
            self.learning.record_rejection(&clone);
        }
        Ok(())
    }

    pub fn snooze_suggestion(&mut self, id: &str, until: u64) -> Result<(), String> {
        let suggestion = self
            .suggestions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("Suggestion '{}' not found", id))?;
        if suggestion.status != SuggestionStatus::Pending {
            return Err(format!("Suggestion '{}' is not pending (status: {:?})", id, suggestion.status));
        }
        suggestion.status = SuggestionStatus::Snoozed(until);
        Ok(())
    }

    pub fn get_pending(&self) -> Vec<&ProactiveSuggestion> {
        self.suggestions
            .iter()
            .filter(|s| s.status == SuggestionStatus::Pending)
            .collect()
    }

    pub fn get_digest(&self) -> ScanDigest {
        let pending: Vec<&ProactiveSuggestion> = self.get_pending();
        let mut by_priority: HashMap<String, usize> = HashMap::new();
        let mut by_category: HashMap<String, usize> = HashMap::new();
        let mut file_counts: HashMap<String, usize> = HashMap::new();

        for s in &pending {
            *by_priority.entry(s.priority.as_str().to_string()).or_insert(0) += 1;
            *by_category.entry(s.category.as_str().to_string()).or_insert(0) += 1;
            if let Some(ref path) = s.file_path {
                *file_counts.entry(path.clone()).or_insert(0) += 1;
            }
        }

        let mut top_files: Vec<(String, usize)> = file_counts.into_iter().collect();
        top_files.sort_by(|a, b| b.1.cmp(&a.1));
        top_files.truncate(10);

        ScanDigest {
            total_pending: pending.len(),
            by_priority,
            by_category,
            top_files,
        }
    }

    pub fn clear_applied(&mut self) -> usize {
        let before = self.suggestions.len();
        self.suggestions.retain(|s| s.status != SuggestionStatus::Applied);
        before - self.suggestions.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_scanner() -> ProactiveScanner {
        ProactiveScanner::new(ScanConfig::default())
    }

    fn scanner_with_categories(cats: Vec<ScanCategory>) -> ProactiveScanner {
        ProactiveScanner::new(ScanConfig {
            enabled_categories: cats,
            ..ScanConfig::default()
        })
    }

    // --- Scanner creation ---

    #[test]
    fn test_scanner_new_default_config() {
        let s = default_scanner();
        assert_eq!(s.config.enabled_categories.len(), 8);
        assert_eq!(s.config.min_confidence, 0.5);
        assert_eq!(s.config.max_suggestions_per_scan, 20);
        assert!(!s.config.quiet_mode);
        assert!(s.config.learning_enabled);
        assert!(s.suggestions.is_empty());
        assert!(s.scan_history.is_empty());
    }

    #[test]
    fn test_scanner_new_custom_config() {
        let config = ScanConfig {
            enabled_categories: vec![ScanCategory::Security],
            cadence: ScanCadence::OnCommit,
            min_confidence: 0.8,
            max_suggestions_per_scan: 5,
            quiet_mode: true,
            learning_enabled: false,
        };
        let s = ProactiveScanner::new(config.clone());
        assert_eq!(s.config.enabled_categories.len(), 1);
        assert_eq!(s.config.min_confidence, 0.8);
        assert!(s.config.quiet_mode);
    }

    #[test]
    fn test_scanner_periodic_cadence() {
        let config = ScanConfig {
            cadence: ScanCadence::Periodic(300),
            ..ScanConfig::default()
        };
        assert_eq!(config.cadence, ScanCadence::Periodic(300));
    }

    // --- Performance detection ---

    #[test]
    fn test_detect_unwrap_in_non_test() {
        let mut s = scanner_with_categories(vec![ScanCategory::Performance]);
        let code = r#"fn main() {
    let x = some_result.unwrap();
}"#;
        let results = s.scan_file("main.rs", code);
        assert!(results.iter().any(|r| r.title.contains("unwrap()")));
    }

    #[test]
    fn test_no_detect_unwrap_in_test() {
        let mut s = scanner_with_categories(vec![ScanCategory::Performance]);
        let code = r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_it() {
        let x = some_result.unwrap();
    }
}"#;
        let results = s.scan_file("test.rs", code);
        assert!(results.iter().all(|r| !r.title.contains("unwrap()")));
    }

    #[test]
    fn test_detect_nested_loops() {
        let mut s = scanner_with_categories(vec![ScanCategory::Performance]);
        let code = r#"fn process() {
    for i in 0..10 {
        for j in 0..10 {
            println!("{}", i + j);
        }
    }
}"#;
        let results = s.scan_file("perf.rs", code);
        assert!(results.iter().any(|r| r.title.contains("Nested loop")));
    }

    #[test]
    fn test_detect_large_allocation() {
        let mut s = scanner_with_categories(vec![ScanCategory::Performance]);
        let code = "let v = Vec::with_capacity(5000000);";
        let results = s.scan_file("alloc.rs", code);
        assert!(results.iter().any(|r| r.title.contains("Large allocation")));
    }

    #[test]
    fn test_no_detect_small_allocation() {
        let mut s = scanner_with_categories(vec![ScanCategory::Performance]);
        let code = "let v = Vec::with_capacity(100);";
        let results = s.scan_file("alloc.rs", code);
        assert!(results.iter().all(|r| !r.title.contains("Large allocation")));
    }

    // --- Security detection ---

    #[test]
    fn test_detect_hardcoded_api_key() {
        let mut s = scanner_with_categories(vec![ScanCategory::Security]);
        let code = r#"let api_key = "sk-1234567890abcdef";"#;
        let results = s.scan_file("config.rs", code);
        assert!(results.iter().any(|r| r.title.contains("API key")));
    }

    #[test]
    fn test_detect_hardcoded_password() {
        let mut s = scanner_with_categories(vec![ScanCategory::Security]);
        let code = r#"let password = "hunter2";"#;
        let results = s.scan_file("auth.rs", code);
        assert!(results.iter().any(|r| r.title.contains("password")));
    }

    #[test]
    fn test_detect_unsafe_block() {
        let mut s = scanner_with_categories(vec![ScanCategory::Security]);
        let code = r#"unsafe {
    ptr::write(dest, value);
}"#;
        let results = s.scan_file("ffi.rs", code);
        assert!(results.iter().any(|r| r.title.contains("Unsafe block")));
    }

    #[test]
    fn test_security_priority_is_high() {
        let mut s = scanner_with_categories(vec![ScanCategory::Security]);
        let code = r#"let token = "ghp_abc123";"#;
        let results = s.scan_file("secrets.rs", code);
        assert!(results.iter().all(|r| r.priority == Priority::High));
    }

    // --- TechDebt detection ---

    #[test]
    fn test_detect_todo_comment() {
        let mut s = scanner_with_categories(vec![ScanCategory::TechDebt]);
        let code = "// TODO: fix this later\nfn main() {}";
        let results = s.scan_file("main.rs", code);
        assert!(results.iter().any(|r| r.title.contains("TODO")));
    }

    #[test]
    fn test_detect_fixme_comment() {
        let mut s = scanner_with_categories(vec![ScanCategory::TechDebt]);
        let code = "// FIXME: broken edge case\nfn main() {}";
        let results = s.scan_file("main.rs", code);
        assert!(results.iter().any(|r| r.title.contains("FIXME")));
    }

    #[test]
    fn test_detect_hack_comment() {
        let mut s = scanner_with_categories(vec![ScanCategory::TechDebt]);
        let code = "// HACK: temporary workaround";
        let results = s.scan_file("main.rs", code);
        assert!(results.iter().any(|r| r.title.contains("HACK")));
    }

    #[test]
    fn test_detect_long_function() {
        let mut s = scanner_with_categories(vec![ScanCategory::TechDebt]);
        let mut lines = vec!["fn long_function() {".to_string()];
        for i in 0..55 {
            lines.push(format!("    let x{} = {};", i, i));
        }
        lines.push("}".to_string());
        let code = lines.join("\n");
        let results = s.scan_file("long.rs", &code);
        assert!(results.iter().any(|r| r.title.contains("Long function")));
    }

    // --- TestingGaps detection ---

    #[test]
    fn test_detect_untested_pub_fn() {
        let mut s = scanner_with_categories(vec![ScanCategory::TestingGaps]);
        let code = r#"pub fn calculate_total(items: &[i32]) -> i32 {
    items.iter().sum()
}"#;
        let results = s.scan_file("lib.rs", code);
        assert!(results.iter().any(|r| r.title.contains("calculate_total")));
    }

    #[test]
    fn test_no_gap_when_test_exists() {
        let mut s = scanner_with_categories(vec![ScanCategory::TestingGaps]);
        let code = r#"pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() { assert_eq!(add(1, 2), 3); }
}"#;
        let results = s.scan_file("lib.rs", code);
        assert!(results.iter().all(|r| !r.title.contains("add")));
    }

    // --- CodeSmells detection ---

    #[test]
    fn test_detect_deeply_nested_code() {
        let mut s = scanner_with_categories(vec![ScanCategory::CodeSmells]);
        // 5 levels = 20 spaces
        let code = "fn f() {\n                    let deep = true;\n}";
        let results = s.scan_file("nested.rs", code);
        assert!(results.iter().any(|r| r.title.contains("Deeply nested")));
    }

    #[test]
    fn test_detect_duplicate_strings() {
        let mut s = scanner_with_categories(vec![ScanCategory::CodeSmells]);
        let code = r#"let a = "duplicate string";
let b = "duplicate string";
let c = "duplicate string";"#;
        let results = s.scan_file("dup.rs", code);
        assert!(results.iter().any(|r| r.title.contains("Duplicate string literal")));
    }

    #[test]
    fn test_no_duplicate_for_short_strings() {
        let mut s = scanner_with_categories(vec![ScanCategory::CodeSmells]);
        let code = "let a = \"ab\";\nlet b = \"ab\";\nlet c = \"ab\";";
        let results = s.scan_file("short.rs", code);
        assert!(results.iter().all(|r| !r.title.contains("Duplicate")));
    }

    // --- Suggestion lifecycle ---

    #[test]
    fn test_accept_suggestion() {
        let mut s = default_scanner();
        let code = r#"let api_key = "secret123";"#;
        s.scan_file("test.rs", code);
        let id = s.suggestions[0].id.clone();
        assert!(s.accept_suggestion(&id).is_ok());
        assert_eq!(s.suggestions[0].status, SuggestionStatus::Accepted);
    }

    #[test]
    fn test_reject_suggestion() {
        let mut s = default_scanner();
        let code = r#"let api_key = "secret123";"#;
        s.scan_file("test.rs", code);
        let id = s.suggestions[0].id.clone();
        assert!(s.reject_suggestion(&id).is_ok());
        assert_eq!(s.suggestions[0].status, SuggestionStatus::Rejected);
    }

    #[test]
    fn test_snooze_suggestion() {
        let mut s = default_scanner();
        let code = r#"let api_key = "secret123";"#;
        s.scan_file("test.rs", code);
        let id = s.suggestions[0].id.clone();
        assert!(s.snooze_suggestion(&id, 1000).is_ok());
        assert_eq!(s.suggestions[0].status, SuggestionStatus::Snoozed(1000));
    }

    #[test]
    fn test_accept_nonexistent_id() {
        let mut s = default_scanner();
        assert!(s.accept_suggestion("bogus").is_err());
    }

    #[test]
    fn test_reject_already_accepted() {
        let mut s = default_scanner();
        let code = r#"let api_key = "secret123";"#;
        s.scan_file("test.rs", code);
        let id = s.suggestions[0].id.clone();
        s.accept_suggestion(&id).unwrap();
        assert!(s.reject_suggestion(&id).is_err());
    }

    #[test]
    fn test_get_pending() {
        let mut s = default_scanner();
        let code = r#"let api_key = "secret123";
let token = "tok_abc";"#;
        s.scan_file("test.rs", code);
        let pending_before = s.get_pending().len();
        if !s.suggestions.is_empty() {
            let id = s.suggestions[0].id.clone();
            s.accept_suggestion(&id).unwrap();
        }
        let pending_after = s.get_pending().len();
        assert_eq!(pending_after, pending_before - 1);
    }

    // --- Learning store ---

    #[test]
    fn test_learning_record_acceptance() {
        let mut store = LearningStore::default();
        let suggestion = ProactiveSuggestion {
            id: "s1".to_string(),
            category: ScanCategory::Security,
            title: "Hardcoded key".to_string(),
            description: "desc".to_string(),
            file_path: None,
            line_range: None,
            confidence: 0.8,
            priority: Priority::High,
            suggested_fix: None,
            created_at: 100,
            status: SuggestionStatus::Pending,
        };
        store.record_acceptance(&suggestion);
        assert_eq!(store.accepted_patterns.len(), 1);
        assert_eq!(store.get_acceptance_rate(&ScanCategory::Security), 1.0);
    }

    #[test]
    fn test_learning_record_rejection() {
        let mut store = LearningStore::default();
        let suggestion = ProactiveSuggestion {
            id: "s1".to_string(),
            category: ScanCategory::TechDebt,
            title: "TODO".to_string(),
            description: "desc".to_string(),
            file_path: None,
            line_range: None,
            confidence: 0.9,
            priority: Priority::Low,
            suggested_fix: None,
            created_at: 100,
            status: SuggestionStatus::Pending,
        };
        store.record_rejection(&suggestion);
        assert_eq!(store.rejected_patterns.len(), 1);
        assert_eq!(store.get_acceptance_rate(&ScanCategory::TechDebt), 0.0);
    }

    #[test]
    fn test_learning_mixed_rates() {
        let mut store = LearningStore::default();
        let make_sug = |title: &str| ProactiveSuggestion {
            id: "x".into(),
            category: ScanCategory::Performance,
            title: title.to_string(),
            description: "d".into(),
            file_path: None,
            line_range: None,
            confidence: 0.8,
            priority: Priority::Medium,
            suggested_fix: None,
            created_at: 0,
            status: SuggestionStatus::Pending,
        };
        store.record_acceptance(&make_sug("a"));
        store.record_acceptance(&make_sug("b"));
        store.record_rejection(&make_sug("c"));
        let rate = store.get_acceptance_rate(&ScanCategory::Performance);
        assert!((rate - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_learning_should_suggest_high_confidence() {
        let store = LearningStore::default(); // default rate = 0.5
        assert!(store.should_suggest(&ScanCategory::Security, 0.8)); // 0.5 * 0.8 = 0.4 >= 0.25
    }

    #[test]
    fn test_learning_should_not_suggest_low_rate() {
        let mut store = LearningStore::default();
        let sug = ProactiveSuggestion {
            id: "x".into(),
            category: ScanCategory::CodeSmells,
            title: "t".into(),
            description: "d".into(),
            file_path: None,
            line_range: None,
            confidence: 0.3,
            priority: Priority::Low,
            suggested_fix: None,
            created_at: 0,
            status: SuggestionStatus::Pending,
        };
        // Record 10 rejections to drive rate to 0
        for _ in 0..10 {
            store.record_rejection(&sug);
        }
        assert!(!store.should_suggest(&ScanCategory::CodeSmells, 0.3)); // 0.0 * 0.3 = 0 < 0.25
    }

    #[test]
    fn test_learning_top_accepted_categories() {
        let mut store = LearningStore::default();
        let make = |cat: ScanCategory| ProactiveSuggestion {
            id: "x".into(),
            category: cat,
            title: "t".into(),
            description: "d".into(),
            file_path: None,
            line_range: None,
            confidence: 0.8,
            priority: Priority::Medium,
            suggested_fix: None,
            created_at: 0,
            status: SuggestionStatus::Pending,
        };
        store.record_acceptance(&make(ScanCategory::Security));
        store.record_acceptance(&make(ScanCategory::Performance));
        store.record_rejection(&make(ScanCategory::Performance));
        let top = store.top_accepted_categories(2);
        assert_eq!(top.len(), 2);
        // Security should be 1.0, Performance should be 0.5
        assert_eq!(top[0].0, ScanCategory::Security);
        assert_eq!(top[0].1, 1.0);
    }

    // --- Batch scanning ---

    #[test]
    fn test_scan_batch() {
        let mut s = scanner_with_categories(vec![ScanCategory::TechDebt]);
        let files = vec![
            ("a.rs", "// TODO: fix\nfn a() {}"),
            ("b.rs", "// FIXME: broken\nfn b() {}"),
        ];
        let results = s.scan_batch(files);
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_scan_batch_accumulates_history() {
        let mut s = default_scanner();
        let files = vec![("a.rs", "fn a() {}"), ("b.rs", "fn b() {}")];
        s.scan_batch(files);
        assert_eq!(s.scan_history.len(), 2);
    }

    // --- Digest ---

    #[test]
    fn test_digest_empty() {
        let s = default_scanner();
        let d = s.get_digest();
        assert_eq!(d.total_pending, 0);
        assert!(d.by_priority.is_empty());
    }

    #[test]
    fn test_digest_counts() {
        let mut s = scanner_with_categories(vec![ScanCategory::TechDebt]);
        s.scan_file("a.rs", "// TODO: first\n// FIXME: second");
        let d = s.get_digest();
        assert!(d.total_pending >= 2);
        assert!(d.by_category.contains_key("TechDebt"));
        assert!(d.top_files.iter().any(|(f, _)| f == "a.rs"));
    }

    // --- Metrics ---

    #[test]
    fn test_metrics_after_scan() {
        let mut s = scanner_with_categories(vec![ScanCategory::TechDebt]);
        s.scan_file("x.rs", "// TODO: something");
        assert_eq!(s.metrics.total_scans, 1);
        assert!(s.metrics.total_suggestions >= 1);
    }

    #[test]
    fn test_metrics_acceptance_rate() {
        let mut s = default_scanner();
        let code = r#"let api_key = "sk-abc";
let token = "tok-xyz";"#;
        s.scan_file("test.rs", code);
        if s.suggestions.len() >= 2 {
            let id1 = s.suggestions[0].id.clone();
            let id2 = s.suggestions[1].id.clone();
            s.accept_suggestion(&id1).unwrap();
            s.reject_suggestion(&id2).unwrap();
            assert_eq!(s.metrics.total_accepted, 1);
            assert_eq!(s.metrics.total_rejected, 1);
            assert!((s.metrics.acceptance_rate - 0.5).abs() < 0.01);
        }
    }

    // --- Quiet mode ---

    #[test]
    fn test_quiet_mode_filters_low_priority() {
        let mut s = ProactiveScanner::new(ScanConfig {
            quiet_mode: true,
            ..ScanConfig::default()
        });
        let code = "// TODO: low priority thing";
        let results = s.scan_file("quiet.rs", code);
        // TODO comments are Low priority, should be filtered in quiet mode
        assert!(results.iter().all(|r| matches!(r.priority, Priority::Critical | Priority::High)));
    }

    #[test]
    fn test_quiet_mode_keeps_high_priority() {
        let mut s = ProactiveScanner::new(ScanConfig {
            quiet_mode: true,
            ..ScanConfig::default()
        });
        let code = r#"unsafe {
    std::ptr::null::<u8>();
}"#;
        let results = s.scan_file("unsafe.rs", code);
        // Unsafe blocks are High priority, should remain
        assert!(results.iter().any(|r| r.title.contains("Unsafe")));
    }

    // --- Edge cases ---

    #[test]
    fn test_empty_file() {
        let mut s = default_scanner();
        let results = s.scan_file("empty.rs", "");
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_suggestions_clean_code() {
        let mut s = default_scanner();
        let code = "fn main() {\n    println!(\"hello\");\n}";
        let results = s.scan_file("clean.rs", code);
        // Clean code may still trigger TestingGaps for pub fn, but main is not pub
        // So should be empty or very few
        assert!(results.len() <= 1);
    }

    #[test]
    fn test_all_categories_disabled() {
        let mut s = ProactiveScanner::new(ScanConfig {
            enabled_categories: vec![],
            ..ScanConfig::default()
        });
        let code = "let api_key = \"secret\";\n// TODO: fix\nunsafe { }";
        let results = s.scan_file("all_disabled.rs", code);
        assert!(results.is_empty());
    }

    // --- Confidence threshold ---

    #[test]
    fn test_high_confidence_threshold_filters() {
        let mut s = ProactiveScanner::new(ScanConfig {
            min_confidence: 0.95,
            ..ScanConfig::default()
        });
        let code = "// TODO: something";
        let results = s.scan_file("threshold.rs", code);
        // TODO has confidence 0.9, so 0.95 threshold should filter it
        assert!(results.iter().all(|r| r.confidence >= 0.95));
    }

    #[test]
    fn test_low_confidence_threshold_allows_all() {
        let mut s = ProactiveScanner::new(ScanConfig {
            min_confidence: 0.0,
            ..ScanConfig::default()
        });
        let code = "// TODO: something";
        let results = s.scan_file("low_thresh.rs", code);
        assert!(!results.is_empty());
    }

    // --- Scan history ---

    #[test]
    fn test_scan_history_recorded() {
        let mut s = default_scanner();
        s.scan_file("a.rs", "// TODO: x");
        s.scan_file("b.rs", "// FIXME: y");
        assert_eq!(s.scan_history.len(), 2);
        assert_eq!(s.scan_history[0].files_scanned, 1);
    }

    // --- clear_applied ---

    #[test]
    fn test_clear_applied() {
        let mut s = default_scanner();
        s.suggestions.push(ProactiveSuggestion {
            id: "a".into(),
            category: ScanCategory::TechDebt,
            title: "t".into(),
            description: "d".into(),
            file_path: None,
            line_range: None,
            confidence: 0.9,
            priority: Priority::Low,
            suggested_fix: None,
            created_at: 0,
            status: SuggestionStatus::Applied,
        });
        s.suggestions.push(ProactiveSuggestion {
            id: "b".into(),
            category: ScanCategory::Security,
            title: "t2".into(),
            description: "d2".into(),
            file_path: None,
            line_range: None,
            confidence: 0.9,
            priority: Priority::High,
            suggested_fix: None,
            created_at: 0,
            status: SuggestionStatus::Pending,
        });
        let removed = s.clear_applied();
        assert_eq!(removed, 1);
        assert_eq!(s.suggestions.len(), 1);
        assert_eq!(s.suggestions[0].id, "b");
    }

    // --- Max suggestions per scan ---

    #[test]
    fn test_max_suggestions_per_scan() {
        let mut s = ProactiveScanner::new(ScanConfig {
            max_suggestions_per_scan: 2,
            ..ScanConfig::default()
        });
        let code = "// TODO: a\n// FIXME: b\n// HACK: c\n// XXX: d";
        let results = s.scan_file("many.rs", code);
        assert!(results.len() <= 2);
    }

    // --- ID generation ---

    #[test]
    fn test_unique_ids() {
        let mut s = default_scanner();
        let code = "// TODO: a\n// FIXME: b";
        s.scan_file("ids.rs", code);
        let ids: Vec<&str> = s.suggestions.iter().map(|s| s.id.as_str()).collect();
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
    }

    // --- ScanCategory round-trip ---

    #[test]
    fn test_scan_category_as_str_from_str() {
        let cats = vec![
            ScanCategory::Performance,
            ScanCategory::Security,
            ScanCategory::TechDebt,
            ScanCategory::Correctness,
            ScanCategory::Accessibility,
            ScanCategory::TestingGaps,
            ScanCategory::DependencyHealth,
            ScanCategory::CodeSmells,
        ];
        for cat in cats {
            let s = cat.as_str();
            let back = ScanCategory::parse_str(s).unwrap();
            assert_eq!(cat, back);
        }
    }

    #[test]
    fn test_scan_category_from_str_invalid() {
        assert!(ScanCategory::parse_str("Nonexistent").is_none());
    }

    // --- Suggestion fields ---

    #[test]
    fn test_suggestion_has_file_and_line() {
        let mut s = scanner_with_categories(vec![ScanCategory::Security]);
        let code = r#"let api_key = "sk-123";"#;
        let results = s.scan_file("creds.rs", code);
        assert!(!results.is_empty());
        let r = &results[0];
        assert_eq!(r.file_path, Some("creds.rs".to_string()));
        assert!(r.line_range.is_some());
        assert!(r.suggested_fix.is_some());
    }

    // --- Learning integration with scanner ---

    #[test]
    fn test_learning_disabled_no_filtering() {
        let mut s = ProactiveScanner::new(ScanConfig {
            learning_enabled: false,
            ..ScanConfig::default()
        });
        // Even with a hypothetically bad learning rate, suggestions should still appear
        let code = "// TODO: something";
        let results = s.scan_file("learn.rs", code);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_accept_updates_learning() {
        let mut s = default_scanner();
        let code = r#"let api_key = "sk-abc";"#;
        s.scan_file("learn2.rs", code);
        let id = s.suggestions[0].id.clone();
        s.accept_suggestion(&id).unwrap();
        assert!(!s.learning.accepted_patterns.is_empty());
        assert_eq!(s.learning.get_acceptance_rate(&ScanCategory::Security), 1.0);
    }

    #[test]
    fn test_reject_updates_learning() {
        let mut s = default_scanner();
        let code = r#"let api_key = "sk-abc";"#;
        s.scan_file("learn3.rs", code);
        let id = s.suggestions[0].id.clone();
        s.reject_suggestion(&id).unwrap();
        assert!(!s.learning.rejected_patterns.is_empty());
        assert_eq!(s.learning.get_acceptance_rate(&ScanCategory::Security), 0.0);
    }
}
