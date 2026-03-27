//! Cross-tool Agent Skills standard compatibility.
//!
//! Implements a universal skill format that enables skill sharing between
//! AI coding tools (Claude Code, Cursor, Gemini CLI, Junie, Windsurf, etc.).
//! Provides conversion between VibeCody's internal skill format and the
//! cross-tool standard, plus validation, registry, discovery, and migration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Parameter type for skill input schema fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

impl std::fmt::Display for ParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamType::String => write!(f, "string"),
            ParamType::Number => write!(f, "number"),
            ParamType::Boolean => write!(f, "boolean"),
            ParamType::Array => write!(f, "array"),
            ParamType::Object => write!(f, "object"),
        }
    }
}

/// A single parameter definition within a skill's input schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillParam {
    pub name: String,
    pub param_type: ParamType,
    pub description: String,
    pub default_value: Option<String>,
}

/// Schema describing the expected inputs for a skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillInputSchema {
    pub parameters: Vec<SkillParam>,
    pub required: Vec<String>,
}

/// An example demonstrating expected skill behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillExample {
    pub title: String,
    pub input: String,
    pub expected_output: String,
}

/// The cross-tool standard skill format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandardSkill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub tags: Vec<String>,
    pub compatible_tools: Vec<String>,
    pub instructions: String,
    pub input_schema: Option<SkillInputSchema>,
    pub output_format: Option<String>,
    pub examples: Vec<SkillExample>,
    pub metadata: HashMap<String, String>,
}

/// VibeCody's internal skill representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VibeCodySkill {
    pub name: String,
    pub category: String,
    pub description: String,
    pub content: String,
    pub tags: Vec<String>,
    pub difficulty: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Result of validating a `StandardSkill`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validates `StandardSkill` instances against the specification.
pub struct SkillValidator;

impl SkillValidator {
    /// Validate a standard skill, returning errors and warnings.
    pub fn validate(skill: &StandardSkill) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Name must not be empty.
        if skill.name.trim().is_empty() {
            errors.push("Skill name must not be empty".to_string());
        }

        // Description must be present.
        if skill.description.trim().is_empty() {
            errors.push("Skill description must not be empty".to_string());
        }

        // Version must be semver-ish (major.minor.patch).
        if !Self::is_semver(&skill.version) {
            errors.push(format!(
                "Version '{}' is not valid semver (expected X.Y.Z)",
                skill.version
            ));
        }

        // At least one tag required.
        if skill.tags.is_empty() {
            errors.push("Skill must have at least one tag".to_string());
        }

        // Instructions must not be empty.
        if skill.instructions.trim().is_empty() {
            errors.push("Skill instructions must not be empty".to_string());
        }

        // Warnings for optional but recommended fields.
        if skill.author.trim().is_empty() {
            warnings.push("Author is empty; consider providing attribution".to_string());
        }

        if skill.compatible_tools.is_empty() {
            warnings.push("No compatible tools listed; skill may have limited discoverability".to_string());
        }

        if skill.examples.is_empty() {
            warnings.push("No examples provided; consider adding at least one".to_string());
        }

        if skill.output_format.is_none() {
            warnings.push("No output format specified".to_string());
        }

        // Validate input schema required fields reference real params.
        if let Some(ref schema) = skill.input_schema {
            let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
            for req in &schema.required {
                if !param_names.contains(&req.as_str()) {
                    errors.push(format!(
                        "Required parameter '{}' not found in schema parameters",
                        req
                    ));
                }
            }
        }

        let is_valid = errors.is_empty();
        ValidationResult {
            is_valid,
            errors,
            warnings,
        }
    }

    fn is_semver(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        parts.iter().all(|p| p.parse::<u64>().is_ok())
    }
}

// ---------------------------------------------------------------------------
// Converter
// ---------------------------------------------------------------------------

/// Converts between VibeCody and Standard skill formats.
pub struct SkillConverter;

impl SkillConverter {
    /// Convert a VibeCody skill to the cross-tool standard format.
    pub fn to_standard(vibe: &VibeCodySkill) -> StandardSkill {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "vibecody".to_string());
        metadata.insert("category".to_string(), vibe.category.clone());
        if let Some(ref diff) = vibe.difficulty {
            metadata.insert("difficulty".to_string(), diff.clone());
        }

        StandardSkill {
            name: vibe.name.clone(),
            description: vibe.description.clone(),
            version: "1.0.0".to_string(),
            author: "VibeCody".to_string(),
            tags: vibe.tags.clone(),
            compatible_tools: vec![
                "vibecody".to_string(),
                "claude-code".to_string(),
                "cursor".to_string(),
                "gemini-cli".to_string(),
            ],
            instructions: vibe.content.clone(),
            input_schema: None,
            output_format: None,
            examples: Vec::new(),
            metadata,
        }
    }

    /// Convert a standard skill back to VibeCody's internal format.
    pub fn from_standard(std_skill: &StandardSkill) -> VibeCodySkill {
        let category = std_skill
            .metadata
            .get("category")
            .cloned()
            .unwrap_or_else(|| "imported".to_string());
        let difficulty = std_skill.metadata.get("difficulty").cloned();

        VibeCodySkill {
            name: std_skill.name.clone(),
            category,
            description: std_skill.description.clone(),
            content: std_skill.instructions.clone(),
            tags: std_skill.tags.clone(),
            difficulty,
        }
    }

    /// Batch-convert VibeCody skills to standard format.
    pub fn batch_export(skills: &[VibeCodySkill]) -> Vec<StandardSkill> {
        skills.iter().map(Self::to_standard).collect()
    }

    /// Batch-convert standard skills to VibeCody format.
    pub fn batch_import(skills: &[StandardSkill]) -> Vec<VibeCodySkill> {
        skills.iter().map(Self::from_standard).collect()
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// In-memory registry of standard skills with search and JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRegistry {
    skills: Vec<StandardSkill>,
}

impl SkillRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    /// Add a skill. Returns an error if a skill with the same name already exists.
    pub fn add_skill(&mut self, skill: StandardSkill) -> Result<(), String> {
        if self.skills.iter().any(|s| s.name == skill.name) {
            return Err(format!("Skill '{}' already exists in registry", skill.name));
        }
        self.skills.push(skill);
        Ok(())
    }

    /// Remove a skill by name. Returns true if removed.
    pub fn remove_skill(&mut self, name: &str) -> bool {
        let before = self.skills.len();
        self.skills.retain(|s| s.name != name);
        self.skills.len() < before
    }

    /// Free-text search across name, description, and tags.
    pub fn search(&self, query: &str) -> Vec<&StandardSkill> {
        let q = query.to_lowercase();
        self.skills
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Search skills that contain the given tag (case-insensitive).
    pub fn search_by_tag(&self, tag: &str) -> Vec<&StandardSkill> {
        let t = tag.to_lowercase();
        self.skills
            .iter()
            .filter(|s| s.tags.iter().any(|st| st.to_lowercase() == t))
            .collect()
    }

    /// Search skills compatible with a specific tool.
    pub fn search_by_tool(&self, tool: &str) -> Vec<&StandardSkill> {
        let t = tool.to_lowercase();
        self.skills
            .iter()
            .filter(|s| s.compatible_tools.iter().any(|ct| ct.to_lowercase() == t))
            .collect()
    }

    /// Return all skills.
    pub fn list_all(&self) -> &[StandardSkill] {
        &self.skills
    }

    /// Number of skills in the registry.
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Import skills from a JSON string (array of StandardSkill).
    pub fn import_from_json(&mut self, json: &str) -> Result<usize, String> {
        let imported: Vec<StandardSkill> =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))?;
        let count = imported.len();
        for skill in imported {
            // Skip duplicates silently during bulk import.
            if !self.skills.iter().any(|s| s.name == skill.name) {
                self.skills.push(skill);
            }
        }
        Ok(count)
    }

    /// Export all skills as a JSON string.
    pub fn export_to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.skills)
            .map_err(|e| format!("JSON serialization error: {}", e))
    }

    /// Compute a compatibility score (0.0–1.0) for a skill with a given tool.
    ///
    /// Factors: explicit tool listing (0.5), shared tags (0.3), has examples (0.1),
    /// has input schema (0.1).
    pub fn compatibility_score(skill: &StandardSkill, tool: &str) -> f64 {
        let mut score = 0.0;

        // Explicit tool compatibility.
        let t = tool.to_lowercase();
        if skill
            .compatible_tools
            .iter()
            .any(|ct| ct.to_lowercase() == t)
        {
            score += 0.5;
        }

        // Tag richness (more tags = more discoverable).
        let tag_score = (skill.tags.len().min(5) as f64) / 5.0;
        score += 0.3 * tag_score;

        // Has examples.
        if !skill.examples.is_empty() {
            score += 0.1;
        }

        // Has input schema.
        if skill.input_schema.is_some() {
            score += 0.1;
        }

        score
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Discovers skills from remote registries (simulated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDiscovery {
    pub registry_urls: Vec<String>,
    pub cached_skills: HashMap<String, Vec<StandardSkill>>,
}

impl SkillDiscovery {
    /// Create a new discovery instance with the given registry URLs.
    pub fn new(registry_urls: Vec<String>) -> Self {
        Self {
            registry_urls,
            cached_skills: HashMap::new(),
        }
    }

    /// Simulate discovering skills from a URL. Returns a list of skills and
    /// caches them locally.
    pub fn discover(&mut self, url: &str) -> Result<Vec<StandardSkill>, String> {
        if url.trim().is_empty() {
            return Err("Registry URL must not be empty".to_string());
        }

        // Simulated: generate placeholder skills based on URL.
        let registry_name = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("unknown");

        let skills = vec![
            StandardSkill {
                name: format!("{}-code-review", registry_name),
                description: format!("Code review skill from {}", registry_name),
                version: "1.0.0".to_string(),
                author: registry_name.to_string(),
                tags: vec!["code-review".to_string(), "quality".to_string()],
                compatible_tools: vec!["claude-code".to_string(), "cursor".to_string()],
                instructions: "Review the provided code for quality, correctness, and style.".to_string(),
                input_schema: None,
                output_format: Some("markdown".to_string()),
                examples: vec![],
                metadata: HashMap::new(),
            },
            StandardSkill {
                name: format!("{}-refactor", registry_name),
                description: format!("Refactoring skill from {}", registry_name),
                version: "1.0.0".to_string(),
                author: registry_name.to_string(),
                tags: vec!["refactor".to_string(), "improvement".to_string()],
                compatible_tools: vec![
                    "claude-code".to_string(),
                    "cursor".to_string(),
                    "gemini-cli".to_string(),
                ],
                instructions: "Refactor the provided code to improve readability and performance.".to_string(),
                input_schema: None,
                output_format: Some("code".to_string()),
                examples: vec![],
                metadata: HashMap::new(),
            },
        ];

        self.cached_skills.insert(url.to_string(), skills.clone());
        Ok(skills)
    }

    /// Refresh cache for all known registry URLs.
    pub fn refresh_cache(&mut self) -> Result<usize, String> {
        let urls: Vec<String> = self.registry_urls.clone();
        let mut total = 0;
        for url in &urls {
            let skills = self.discover(url)?;
            total += skills.len();
        }
        Ok(total)
    }

    /// Get cached skills for a URL, if any.
    pub fn get_cached(&self, url: &str) -> Option<&Vec<StandardSkill>> {
        self.cached_skills.get(url)
    }
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

/// Report produced by a bulk migration operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationReport {
    pub total: usize,
    pub converted: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Bulk migration utilities for skill files.
pub struct SkillMigrator;

impl SkillMigrator {
    /// Simulate migrating a directory of skill files.
    ///
    /// In production this would walk the filesystem; here we simulate using
    /// the path to derive a deterministic result.
    pub fn migrate_directory(path: &str) -> MigrationReport {
        if path.trim().is_empty() {
            return MigrationReport {
                total: 0,
                converted: 0,
                skipped: 0,
                errors: vec!["Empty path provided".to_string()],
            };
        }

        // Simulate: use path length as a seed for deterministic output.
        let simulated_total = 10 + (path.len() % 20);
        let simulated_errors_count = path.len() % 3;
        let simulated_skipped = path.len() % 4;
        let simulated_converted = simulated_total - simulated_skipped - simulated_errors_count;

        let errors: Vec<String> = (0..simulated_errors_count)
            .map(|i| format!("Failed to parse skill file #{}: invalid format", i + 1))
            .collect();

        MigrationReport {
            total: simulated_total,
            converted: simulated_converted,
            skipped: simulated_skipped,
            errors,
        }
    }

    /// Validate and convert a batch of VibeCody skills, producing a migration report.
    pub fn migrate_skills(skills: &[VibeCodySkill]) -> (Vec<StandardSkill>, MigrationReport) {
        let mut converted = Vec::new();
        let mut errors = Vec::new();
        let mut skipped = 0;

        for skill in skills {
            if skill.name.trim().is_empty() {
                skipped += 1;
                continue;
            }
            if skill.content.trim().is_empty() {
                errors.push(format!("Skill '{}' has empty content", skill.name));
                continue;
            }
            converted.push(SkillConverter::to_standard(skill));
        }

        let report = MigrationReport {
            total: skills.len(),
            converted: converted.len(),
            skipped,
            errors,
        };

        (converted, report)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vibe_skill(name: &str) -> VibeCodySkill {
        VibeCodySkill {
            name: name.to_string(),
            category: "testing".to_string(),
            description: format!("{} description", name),
            content: format!("Instructions for {}", name),
            tags: vec!["test".to_string(), "rust".to_string()],
            difficulty: Some("intermediate".to_string()),
        }
    }

    fn make_standard_skill(name: &str) -> StandardSkill {
        StandardSkill {
            name: name.to_string(),
            description: format!("{} description", name),
            version: "1.0.0".to_string(),
            author: "test-author".to_string(),
            tags: vec!["test".to_string()],
            compatible_tools: vec!["claude-code".to_string(), "cursor".to_string()],
            instructions: format!("Do the {} thing", name),
            input_schema: None,
            output_format: Some("markdown".to_string()),
            examples: vec![SkillExample {
                title: "Example 1".to_string(),
                input: "input".to_string(),
                expected_output: "output".to_string(),
            }],
            metadata: HashMap::new(),
        }
    }

    // -- Conversion tests --

    #[test]
    fn test_to_standard_preserves_name_and_description() {
        let vibe = make_vibe_skill("my-skill");
        let std = SkillConverter::to_standard(&vibe);
        assert_eq!(std.name, "my-skill");
        assert_eq!(std.description, "my-skill description");
    }

    #[test]
    fn test_to_standard_sets_metadata() {
        let vibe = make_vibe_skill("alpha");
        let std = SkillConverter::to_standard(&vibe);
        assert_eq!(std.metadata.get("source").unwrap(), "vibecody");
        assert_eq!(std.metadata.get("category").unwrap(), "testing");
        assert_eq!(std.metadata.get("difficulty").unwrap(), "intermediate");
    }

    #[test]
    fn test_to_standard_sets_default_version() {
        let vibe = make_vibe_skill("v");
        let std = SkillConverter::to_standard(&vibe);
        assert_eq!(std.version, "1.0.0");
    }

    #[test]
    fn test_from_standard_extracts_category_from_metadata() {
        let mut std = make_standard_skill("s");
        std.metadata.insert("category".to_string(), "devops".to_string());
        let vibe = SkillConverter::from_standard(&std);
        assert_eq!(vibe.category, "devops");
    }

    #[test]
    fn test_from_standard_defaults_category_to_imported() {
        let std = make_standard_skill("s");
        let vibe = SkillConverter::from_standard(&std);
        assert_eq!(vibe.category, "imported");
    }

    #[test]
    fn test_roundtrip_vibe_to_standard_and_back() {
        let original = make_vibe_skill("roundtrip");
        let std = SkillConverter::to_standard(&original);
        let back = SkillConverter::from_standard(&std);
        assert_eq!(back.name, original.name);
        assert_eq!(back.description, original.description);
        assert_eq!(back.content, original.content);
        assert_eq!(back.tags, original.tags);
        assert_eq!(back.difficulty, original.difficulty);
        assert_eq!(back.category, original.category);
    }

    #[test]
    fn test_roundtrip_standard_to_vibe_and_back() {
        let mut original = make_standard_skill("roundtrip2");
        original.metadata.insert("category".to_string(), "security".to_string());
        original.metadata.insert("difficulty".to_string(), "advanced".to_string());
        let vibe = SkillConverter::from_standard(&original);
        let back = SkillConverter::to_standard(&vibe);
        assert_eq!(back.name, original.name);
        assert_eq!(back.instructions, original.instructions);
        assert_eq!(back.tags, original.tags);
    }

    // -- Batch export/import --

    #[test]
    fn test_batch_export() {
        let skills: Vec<VibeCodySkill> = (0..5).map(|i| make_vibe_skill(&format!("s{}", i))).collect();
        let exported = SkillConverter::batch_export(&skills);
        assert_eq!(exported.len(), 5);
        assert_eq!(exported[2].name, "s2");
    }

    #[test]
    fn test_batch_import() {
        let skills: Vec<StandardSkill> = (0..3).map(|i| make_standard_skill(&format!("std{}", i))).collect();
        let imported = SkillConverter::batch_import(&skills);
        assert_eq!(imported.len(), 3);
        assert_eq!(imported[1].name, "std1");
    }

    #[test]
    fn test_batch_export_empty() {
        let exported = SkillConverter::batch_export(&[]);
        assert!(exported.is_empty());
    }

    // -- Validation tests --

    #[test]
    fn test_validate_valid_skill() {
        let skill = make_standard_skill("valid");
        let result = SkillValidator::validate(&skill);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_empty_name() {
        let mut skill = make_standard_skill("x");
        skill.name = "".to_string();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn test_validate_empty_description() {
        let mut skill = make_standard_skill("x");
        skill.description = "  ".to_string();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("description")));
    }

    #[test]
    fn test_validate_bad_version() {
        let mut skill = make_standard_skill("x");
        skill.version = "not-semver".to_string();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("semver")));
    }

    #[test]
    fn test_validate_no_tags() {
        let mut skill = make_standard_skill("x");
        skill.tags.clear();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("tag")));
    }

    #[test]
    fn test_validate_empty_instructions() {
        let mut skill = make_standard_skill("x");
        skill.instructions = "".to_string();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("instructions")));
    }

    #[test]
    fn test_validate_warnings_for_missing_optional_fields() {
        let mut skill = make_standard_skill("x");
        skill.author = "".to_string();
        skill.compatible_tools.clear();
        skill.examples.clear();
        skill.output_format = None;
        let result = SkillValidator::validate(&skill);
        assert!(result.is_valid);
        assert!(result.warnings.len() >= 3);
    }

    #[test]
    fn test_validate_invalid_required_param_reference() {
        let mut skill = make_standard_skill("x");
        skill.input_schema = Some(SkillInputSchema {
            parameters: vec![SkillParam {
                name: "foo".to_string(),
                param_type: ParamType::String,
                description: "a foo".to_string(),
                default_value: None,
            }],
            required: vec!["bar".to_string()],
        });
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("bar")));
    }

    #[test]
    fn test_validate_multiple_errors() {
        let skill = StandardSkill {
            name: "".to_string(),
            description: "".to_string(),
            version: "bad".to_string(),
            author: "".to_string(),
            tags: vec![],
            compatible_tools: vec![],
            instructions: "".to_string(),
            input_schema: None,
            output_format: None,
            examples: vec![],
            metadata: HashMap::new(),
        };
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.len() >= 4);
    }

    // -- Registry tests --

    #[test]
    fn test_registry_add_and_count() {
        let mut reg = SkillRegistry::new();
        assert_eq!(reg.count(), 0);
        reg.add_skill(make_standard_skill("a")).unwrap();
        assert_eq!(reg.count(), 1);
    }

    #[test]
    fn test_registry_add_duplicate_fails() {
        let mut reg = SkillRegistry::new();
        reg.add_skill(make_standard_skill("dup")).unwrap();
        let result = reg.add_skill(make_standard_skill("dup"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_registry_remove_skill() {
        let mut reg = SkillRegistry::new();
        reg.add_skill(make_standard_skill("rm")).unwrap();
        assert!(reg.remove_skill("rm"));
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_registry_remove_nonexistent() {
        let mut reg = SkillRegistry::new();
        assert!(!reg.remove_skill("ghost"));
    }

    #[test]
    fn test_registry_search_by_name() {
        let mut reg = SkillRegistry::new();
        reg.add_skill(make_standard_skill("code-review")).unwrap();
        reg.add_skill(make_standard_skill("refactor")).unwrap();
        let results = reg.search("code");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "code-review");
    }

    #[test]
    fn test_registry_search_by_description() {
        let mut reg = SkillRegistry::new();
        reg.add_skill(make_standard_skill("alpha")).unwrap();
        let results = reg.search("alpha description");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_registry_search_by_tag() {
        let mut reg = SkillRegistry::new();
        let mut s = make_standard_skill("tagged");
        s.tags = vec!["security".to_string(), "audit".to_string()];
        reg.add_skill(s).unwrap();
        reg.add_skill(make_standard_skill("other")).unwrap();

        let results = reg.search_by_tag("security");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "tagged");
    }

    #[test]
    fn test_registry_search_by_tag_case_insensitive() {
        let mut reg = SkillRegistry::new();
        let mut s = make_standard_skill("ci");
        s.tags = vec!["CI-CD".to_string()];
        reg.add_skill(s).unwrap();
        let results = reg.search_by_tag("ci-cd");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_registry_search_by_tool() {
        let mut reg = SkillRegistry::new();
        let mut s = make_standard_skill("for-junie");
        s.compatible_tools = vec!["junie".to_string()];
        reg.add_skill(s).unwrap();
        reg.add_skill(make_standard_skill("generic")).unwrap();

        let results = reg.search_by_tool("junie");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "for-junie");
    }

    #[test]
    fn test_registry_list_all() {
        let mut reg = SkillRegistry::new();
        reg.add_skill(make_standard_skill("a")).unwrap();
        reg.add_skill(make_standard_skill("b")).unwrap();
        assert_eq!(reg.list_all().len(), 2);
    }

    // -- JSON serialization --

    #[test]
    fn test_registry_export_import_json() {
        let mut reg = SkillRegistry::new();
        reg.add_skill(make_standard_skill("json1")).unwrap();
        reg.add_skill(make_standard_skill("json2")).unwrap();
        let json = reg.export_to_json().unwrap();

        let mut reg2 = SkillRegistry::new();
        let count = reg2.import_from_json(&json).unwrap();
        assert_eq!(count, 2);
        assert_eq!(reg2.count(), 2);
    }

    #[test]
    fn test_registry_import_invalid_json() {
        let mut reg = SkillRegistry::new();
        let result = reg.import_from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_import_skips_duplicates() {
        let mut reg = SkillRegistry::new();
        reg.add_skill(make_standard_skill("existing")).unwrap();
        let json = serde_json::to_string(&vec![make_standard_skill("existing")]).unwrap();
        let count = reg.import_from_json(&json).unwrap();
        assert_eq!(count, 1); // Reported 1 in source, but skipped.
        assert_eq!(reg.count(), 1); // Still 1.
    }

    // -- Compatibility scoring --

    #[test]
    fn test_compatibility_score_full() {
        let mut skill = make_standard_skill("full");
        skill.input_schema = Some(SkillInputSchema {
            parameters: vec![],
            required: vec![],
        });
        // compatible_tools includes "claude-code", has 1 tag, has examples, has schema.
        let score = SkillRegistry::compatibility_score(&skill, "claude-code");
        assert!(score > 0.7);
    }

    #[test]
    fn test_compatibility_score_no_tool_match() {
        let skill = make_standard_skill("nomatch");
        let score = SkillRegistry::compatibility_score(&skill, "unknown-tool");
        assert!(score < 0.5);
    }

    #[test]
    fn test_compatibility_score_zero_for_empty_skill() {
        let skill = StandardSkill {
            name: "empty".to_string(),
            description: "".to_string(),
            version: "1.0.0".to_string(),
            author: "".to_string(),
            tags: vec![],
            compatible_tools: vec![],
            instructions: "".to_string(),
            input_schema: None,
            output_format: None,
            examples: vec![],
            metadata: HashMap::new(),
        };
        let score = SkillRegistry::compatibility_score(&skill, "anything");
        assert!(score < 0.01);
    }

    // -- Discovery tests --

    #[test]
    fn test_discovery_discover_returns_skills() {
        let mut disc = SkillDiscovery::new(vec!["https://registry.example.com/community".to_string()]);
        let skills = disc.discover("https://registry.example.com/community").unwrap();
        assert_eq!(skills.len(), 2);
        assert!(skills[0].name.contains("community"));
    }

    #[test]
    fn test_discovery_caches_results() {
        let mut disc = SkillDiscovery::new(vec![]);
        disc.discover("https://r.io/main").unwrap();
        let cached = disc.get_cached("https://r.io/main");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 2);
    }

    #[test]
    fn test_discovery_empty_url_fails() {
        let mut disc = SkillDiscovery::new(vec![]);
        let result = disc.discover("");
        assert!(result.is_err());
    }

    #[test]
    fn test_discovery_refresh_cache() {
        let mut disc = SkillDiscovery::new(vec![
            "https://a.io/skills".to_string(),
            "https://b.io/skills".to_string(),
        ]);
        let total = disc.refresh_cache().unwrap();
        assert_eq!(total, 4); // 2 skills per URL.
        assert_eq!(disc.cached_skills.len(), 2);
    }

    #[test]
    fn test_discovery_get_cached_miss() {
        let disc = SkillDiscovery::new(vec![]);
        assert!(disc.get_cached("https://nowhere.io").is_none());
    }

    // -- Migration tests --

    #[test]
    fn test_migrate_directory_nonempty_path() {
        let report = SkillMigrator::migrate_directory("/some/skills/dir");
        assert!(report.total > 0);
        assert_eq!(report.total, report.converted + report.skipped + report.errors.len());
    }

    #[test]
    fn test_migrate_directory_empty_path() {
        let report = SkillMigrator::migrate_directory("");
        assert_eq!(report.total, 0);
        assert!(!report.errors.is_empty());
    }

    #[test]
    fn test_migrate_skills_with_valid_batch() {
        let skills = vec![make_vibe_skill("a"), make_vibe_skill("b")];
        let (converted, report) = SkillMigrator::migrate_skills(&skills);
        assert_eq!(converted.len(), 2);
        assert_eq!(report.converted, 2);
        assert_eq!(report.skipped, 0);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_migrate_skills_skips_empty_name() {
        let skill = make_vibe_skill("good");
        let mut empty_name = make_vibe_skill("");
        empty_name.name = "  ".to_string();
        let skills = vec![skill.clone(), empty_name];
        let (converted, report) = SkillMigrator::migrate_skills(&skills);
        assert_eq!(converted.len(), 1);
        assert_eq!(report.skipped, 1);
    }

    #[test]
    fn test_migrate_skills_errors_on_empty_content() {
        let mut bad = make_vibe_skill("bad");
        bad.content = "".to_string();
        let (converted, report) = SkillMigrator::migrate_skills(&[bad]);
        assert_eq!(converted.len(), 0);
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].contains("empty content"));
    }

    // -- Edge cases --

    #[test]
    fn test_param_type_display() {
        assert_eq!(format!("{}", ParamType::String), "string");
        assert_eq!(format!("{}", ParamType::Number), "number");
        assert_eq!(format!("{}", ParamType::Boolean), "boolean");
        assert_eq!(format!("{}", ParamType::Array), "array");
        assert_eq!(format!("{}", ParamType::Object), "object");
    }

    #[test]
    fn test_standard_skill_serde_roundtrip() {
        let skill = make_standard_skill("serde-test");
        let json = serde_json::to_string(&skill).unwrap();
        let back: StandardSkill = serde_json::from_str(&json).unwrap();
        assert_eq!(skill, back);
    }

    #[test]
    fn test_vibe_skill_serde_roundtrip() {
        let skill = make_vibe_skill("serde-vibe");
        let json = serde_json::to_string(&skill).unwrap();
        let back: VibeCodySkill = serde_json::from_str(&json).unwrap();
        assert_eq!(skill, back);
    }

    #[test]
    fn test_validation_result_serde() {
        let vr = ValidationResult {
            is_valid: false,
            errors: vec!["e1".to_string()],
            warnings: vec!["w1".to_string()],
        };
        let json = serde_json::to_string(&vr).unwrap();
        let back: ValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(vr, back);
    }

    #[test]
    fn test_skill_with_full_input_schema() {
        let mut skill = make_standard_skill("schema-test");
        skill.input_schema = Some(SkillInputSchema {
            parameters: vec![
                SkillParam {
                    name: "language".to_string(),
                    param_type: ParamType::String,
                    description: "Target language".to_string(),
                    default_value: Some("rust".to_string()),
                },
                SkillParam {
                    name: "verbose".to_string(),
                    param_type: ParamType::Boolean,
                    description: "Verbose output".to_string(),
                    default_value: None,
                },
            ],
            required: vec!["language".to_string()],
        });
        let result = SkillValidator::validate(&skill);
        assert!(result.is_valid);
    }

    #[test]
    fn test_registry_default() {
        let reg = SkillRegistry::default();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_search_empty_query_returns_all() {
        let mut reg = SkillRegistry::new();
        reg.add_skill(make_standard_skill("x")).unwrap();
        reg.add_skill(make_standard_skill("y")).unwrap();
        // Empty string is contained in all strings.
        let results = reg.search("");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_conversion_without_difficulty() {
        let mut vibe = make_vibe_skill("no-diff");
        vibe.difficulty = None;
        let std = SkillConverter::to_standard(&vibe);
        assert!(!std.metadata.contains_key("difficulty"));
        let back = SkillConverter::from_standard(&std);
        assert_eq!(back.difficulty, None);
    }
}
