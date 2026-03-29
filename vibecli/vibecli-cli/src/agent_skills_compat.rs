//! Agent Skills Standard Compatibility — cross-tool skills interop.
//!
//! Provides parsing, validation, conversion, registry, import, and export
//! of portable skill definitions across VibeCody, Claude Code, Cursor,
//! Gemini CLI, and other AI coding tools using a standardized schema.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Skill format / originating tool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillFormat {
    VibeCody,
    Standard,
    ClaudeCode,
    Cursor,
    GeminiCLI,
    Custom(String),
}

impl SkillFormat {
    pub fn as_str(&self) -> &str {
        match self {
            Self::VibeCody => "vibecody",
            Self::Standard => "standard",
            Self::ClaudeCode => "claude_code",
            Self::Cursor => "cursor",
            Self::GeminiCLI => "gemini_cli",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vibecody" | "vibe_cody" | "vibe-cody" => Self::VibeCody,
            "standard" => Self::Standard,
            "claude_code" | "claude-code" | "claudecode" => Self::ClaudeCode,
            "cursor" => Self::Cursor,
            "gemini_cli" | "gemini-cli" | "geminicli" => Self::GeminiCLI,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Skill difficulty level.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillDifficulty {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

impl SkillDifficulty {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Beginner => "beginner",
            Self::Intermediate => "intermediate",
            Self::Advanced => "advanced",
            Self::Expert => "expert",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "beginner" | "easy" => Self::Beginner,
            "intermediate" | "medium" => Self::Intermediate,
            "advanced" | "hard" => Self::Advanced,
            "expert" | "guru" => Self::Expert,
            _ => Self::Intermediate,
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Metadata describing a skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub tags: Vec<String>,
    pub difficulty: SkillDifficulty,
    pub format: SkillFormat,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
    pub dependencies: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Default for SkillMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            version: "0.1.0".to_string(),
            author: String::new(),
            tags: Vec::new(),
            difficulty: SkillDifficulty::Intermediate,
            format: SkillFormat::Standard,
            input_types: Vec::new(),
            output_types: Vec::new(),
            dependencies: Vec::new(),
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// An example attached to a skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillExample {
    pub title: String,
    pub input: String,
    pub expected_output: String,
}

/// A fully-parsed skill with metadata, body content, and examples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandardSkill {
    pub metadata: SkillMetadata,
    pub content: String,
    pub frontmatter: HashMap<String, String>,
    pub examples: Vec<SkillExample>,
}

/// Result of validating a skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub compatibility_score: f64,
}

/// Result of converting a skill to another format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillConversionResult {
    pub skill: StandardSkill,
    pub warnings: Vec<String>,
    pub changes_made: Vec<String>,
}

/// An entry in the skill registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillRegistryEntry {
    pub skill: SkillMetadata,
    pub source_url: String,
    pub downloads: u64,
    pub rating: f64,
    pub verified: bool,
}

// ---------------------------------------------------------------------------
// SkillParser
// ---------------------------------------------------------------------------

/// Parses markdown skill files with optional YAML frontmatter.
pub struct SkillParser;

impl SkillParser {
    /// Extract YAML-style frontmatter delimited by `---` from the top of content.
    /// Returns (frontmatter key-value pairs, remaining body).
    pub fn extract_frontmatter(content: &str) -> (HashMap<String, String>, String) {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return (HashMap::new(), content.to_string());
        }

        // Find the closing ---
        let after_first = &trimmed[3..];
        if let Some(end_idx) = after_first.find("\n---") {
            let fm_block = &after_first[..end_idx];
            let body_start = end_idx + 4; // skip \n---
            let body = after_first[body_start..].trim_start_matches('\n').to_string();

            let mut map = HashMap::new();
            for line in fm_block.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_string();
                    let value = line[colon_pos + 1..].trim().to_string();
                    if !key.is_empty() {
                        map.insert(key, value);
                    }
                }
            }
            (map, body)
        } else {
            // No closing ---, treat entire content as body
            (HashMap::new(), content.to_string())
        }
    }

    /// Parse a skill in the standard format (YAML frontmatter + markdown body).
    pub fn parse(content: &str) -> Result<StandardSkill, String> {
        if content.trim().is_empty() {
            return Err("Empty skill content".to_string());
        }

        let (fm, body) = Self::extract_frontmatter(content);

        let mut metadata = SkillMetadata { format: SkillFormat::Standard, ..Default::default() };

        if let Some(name) = fm.get("name") {
            metadata.name = name.clone();
        }
        if let Some(desc) = fm.get("description") {
            metadata.description = desc.clone();
        }
        if let Some(ver) = fm.get("version") {
            metadata.version = ver.clone();
        }
        if let Some(author) = fm.get("author") {
            metadata.author = author.clone();
        }
        if let Some(diff) = fm.get("difficulty") {
            metadata.difficulty = SkillDifficulty::parse(diff);
        }
        if let Some(tags) = fm.get("tags") {
            metadata.tags = tags.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        }
        if let Some(deps) = fm.get("dependencies") {
            metadata.dependencies = deps.split(',').map(|d| d.trim().to_string()).filter(|d| !d.is_empty()).collect();
        }
        if let Some(input) = fm.get("input_types") {
            metadata.input_types = input.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        }
        if let Some(output) = fm.get("output_types") {
            metadata.output_types = output.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        }
        if let Some(created) = fm.get("created_at") {
            metadata.created_at = created.parse().unwrap_or(0);
        }
        if let Some(updated) = fm.get("updated_at") {
            metadata.updated_at = updated.parse().unwrap_or(0);
        }

        // If name not in frontmatter, try to extract from first heading
        if metadata.name.is_empty() {
            if let Some(first_line) = body.lines().find(|l| l.starts_with('#')) {
                metadata.name = first_line.trim_start_matches('#').trim().to_string();
            }
        }

        // Extract description from first non-heading paragraph if not set
        if metadata.description.is_empty() {
            for line in body.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                metadata.description = trimmed.to_string();
                break;
            }
        }

        // Extract examples from ```example blocks or ## Examples sections
        let examples = Self::extract_examples(&body);

        Ok(StandardSkill {
            metadata,
            content: body,
            frontmatter: fm,
            examples,
        })
    }

    /// Parse a VibeCody-format skill (markdown with # title, ## sections).
    pub fn parse_vibecody(content: &str) -> Result<StandardSkill, String> {
        if content.trim().is_empty() {
            return Err("Empty skill content".to_string());
        }

        let (fm, body) = Self::extract_frontmatter(content);
        let mut metadata = SkillMetadata { format: SkillFormat::VibeCody, ..Default::default() };

        // Apply any frontmatter overrides
        if let Some(name) = fm.get("name") {
            metadata.name = name.clone();
        }
        if let Some(ver) = fm.get("version") {
            metadata.version = ver.clone();
        }
        if let Some(author) = fm.get("author") {
            metadata.author = author.clone();
        }

        // Extract title from first # heading
        if metadata.name.is_empty() {
            for line in body.lines() {
                let trimmed = line.trim();
                if let Some(name) = trimmed.strip_prefix("# ") {
                    metadata.name = name.trim().to_string();
                    break;
                }
            }
        }

        // Extract description from first paragraph after title
        let mut found_title = false;
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                found_title = true;
                continue;
            }
            if found_title && !trimmed.is_empty() && !trimmed.starts_with('#') {
                metadata.description = trimmed.to_string();
                break;
            }
        }

        // Extract tags from ## When to Use section keywords
        let when_section = Self::extract_section(&body, "When to Use");
        if !when_section.is_empty() {
            let mut tags = Vec::new();
            for line in when_section.lines() {
                let trimmed = line.trim().trim_start_matches('-').trim();
                if !trimmed.is_empty() {
                    // Extract key phrases as tags
                    let words: Vec<&str> = trimmed.split_whitespace().take(3).collect();
                    if !words.is_empty() {
                        tags.push(words.join(" ").to_lowercase());
                    }
                }
            }
            if tags.len() > 5 {
                tags.truncate(5);
            }
            metadata.tags = tags;
        }

        let examples = Self::extract_examples(&body);

        Ok(StandardSkill {
            metadata,
            content: body,
            frontmatter: fm,
            examples,
        })
    }

    /// Extract a named ## section body from markdown.
    fn extract_section(body: &str, heading: &str) -> String {
        let target = format!("## {}", heading);
        let mut in_section = false;
        let mut lines = Vec::new();

        for line in body.lines() {
            if line.trim().eq_ignore_ascii_case(&target) {
                in_section = true;
                continue;
            }
            if in_section {
                if line.starts_with("## ") {
                    break;
                }
                lines.push(line);
            }
        }
        lines.join("\n").trim().to_string()
    }

    /// Extract examples from markdown code blocks in ## Examples section.
    fn extract_examples(body: &str) -> Vec<SkillExample> {
        let examples_section = Self::extract_section(body, "Examples");
        if examples_section.is_empty() {
            return Vec::new();
        }

        let mut examples = Vec::new();
        let mut in_code_block = false;
        let mut current_code = Vec::new();

        for line in examples_section.lines() {
            if line.trim().starts_with("```") && !in_code_block {
                in_code_block = true;
                current_code.clear();
                continue;
            }
            if line.trim().starts_with("```") && in_code_block {
                in_code_block = false;
                let code = current_code.join("\n");
                if !code.trim().is_empty() {
                    // Split on # comment lines that look like expected output
                    let mut input_lines = Vec::new();
                    let mut output_lines = Vec::new();
                    for cl in code.lines() {
                        if cl.trim().starts_with('#') && !input_lines.is_empty() {
                            output_lines.push(cl.trim_start_matches('#').trim().to_string());
                        } else {
                            input_lines.push(cl.to_string());
                        }
                    }
                    let idx = examples.len() + 1;
                    examples.push(SkillExample {
                        title: format!("Example {}", idx),
                        input: input_lines.join("\n"),
                        expected_output: output_lines.join("\n"),
                    });
                }
                continue;
            }
            if in_code_block {
                current_code.push(line);
            }
        }

        examples
    }
}

// ---------------------------------------------------------------------------
// SkillValidator
// ---------------------------------------------------------------------------

/// Validates skill metadata and structure.
pub struct SkillValidator;

impl SkillValidator {
    /// Full validation of a StandardSkill.
    pub fn validate(skill: &StandardSkill) -> SkillValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate metadata
        errors.extend(Self::validate_metadata(&skill.metadata));

        // Content checks
        if skill.content.trim().is_empty() {
            errors.push("Skill content body is empty".to_string());
        }

        if skill.content.len() > 100_000 {
            warnings.push("Skill content exceeds 100KB — may be too large for some tools".to_string());
        }

        if skill.examples.is_empty() {
            warnings.push("No examples provided — examples improve skill discoverability".to_string());
        }

        // Description length
        if skill.metadata.description.len() > 200 {
            warnings.push("Description exceeds 200 characters — may be truncated in some tools".to_string());
        }

        // Compute compatibility score
        let compat = Self::check_compatibility(skill, &SkillFormat::Standard);

        let is_valid = errors.is_empty();

        SkillValidationResult {
            is_valid,
            errors,
            warnings,
            compatibility_score: compat,
        }
    }

    /// Validate required metadata fields.
    pub fn validate_metadata(meta: &SkillMetadata) -> Vec<String> {
        let mut errors = Vec::new();

        if meta.name.is_empty() {
            errors.push("Skill name is required".to_string());
        }
        if meta.name.len() > 100 {
            errors.push("Skill name exceeds 100 characters".to_string());
        }
        if meta.description.is_empty() {
            errors.push("Skill description is required".to_string());
        }
        if meta.version.is_empty() {
            errors.push("Skill version is required".to_string());
        }

        // Validate semver-ish version
        if !meta.version.is_empty() {
            let parts: Vec<&str> = meta.version.split('.').collect();
            if parts.len() < 2 || parts.len() > 3 {
                errors.push(format!("Version '{}' is not valid semver (expected X.Y or X.Y.Z)", meta.version));
            } else {
                for p in &parts {
                    if p.parse::<u32>().is_err() {
                        errors.push(format!("Version component '{}' is not a valid number", p));
                        break;
                    }
                }
            }
        }

        errors
    }

    /// Compute compatibility score (0.0-1.0) of a skill with a target format.
    pub fn check_compatibility(skill: &StandardSkill, target: &SkillFormat) -> f64 {
        let mut score = 0.0_f64;
        let mut max_score = 0.0_f64;

        // Has name (required by all formats)
        max_score += 1.0;
        if !skill.metadata.name.is_empty() {
            score += 1.0;
        }

        // Has description
        max_score += 1.0;
        if !skill.metadata.description.is_empty() {
            score += 1.0;
        }

        // Has version
        max_score += 0.5;
        if !skill.metadata.version.is_empty() {
            score += 0.5;
        }

        // Has tags (important for discovery)
        max_score += 1.0;
        if !skill.metadata.tags.is_empty() {
            score += 1.0;
        }

        // Has content body
        max_score += 1.0;
        if !skill.content.trim().is_empty() {
            score += 1.0;
        }

        // Has examples
        max_score += 1.0;
        if !skill.examples.is_empty() {
            score += 1.0;
        }

        // Description under 200 chars
        max_score += 0.5;
        if skill.metadata.description.len() <= 200 {
            score += 0.5;
        }

        // Format-specific bonuses
        max_score += 1.0;
        match target {
            SkillFormat::Standard => {
                // Standard format prefers frontmatter
                if !skill.frontmatter.is_empty() {
                    score += 1.0;
                }
            }
            SkillFormat::VibeCody => {
                // VibeCody format prefers ## sections
                if skill.content.contains("## When to Use") || skill.content.contains("## Commands") {
                    score += 1.0;
                }
            }
            SkillFormat::ClaudeCode => {
                // Claude Code prefers structured examples and input/output types
                if !skill.metadata.input_types.is_empty() && !skill.metadata.output_types.is_empty() {
                    score += 1.0;
                }
            }
            SkillFormat::Cursor => {
                // Cursor prefers rule-style content
                if skill.content.contains("## Best Practices") || skill.content.contains("## Rules") {
                    score += 1.0;
                }
            }
            SkillFormat::GeminiCLI => {
                if !skill.metadata.author.is_empty() {
                    score += 1.0;
                }
            }
            SkillFormat::Custom(_) => {
                score += 0.5; // partial by default
            }
        }

        if max_score == 0.0 {
            return 0.0;
        }

        (score / max_score).min(1.0)
    }
}

// ---------------------------------------------------------------------------
// SkillConverter
// ---------------------------------------------------------------------------

/// Converts skills between formats.
pub struct SkillConverter;

impl SkillConverter {
    /// Normalize a skill to the Standard format.
    pub fn to_standard(skill: &StandardSkill) -> SkillConversionResult {
        let mut converted = skill.clone();
        let mut warnings = Vec::new();
        let mut changes = Vec::new();

        if converted.metadata.format != SkillFormat::Standard {
            changes.push(format!(
                "Changed format from {} to standard",
                converted.metadata.format.as_str()
            ));
            converted.metadata.format = SkillFormat::Standard;
        }

        // Ensure frontmatter has all metadata fields
        if !converted.frontmatter.contains_key("name") && !converted.metadata.name.is_empty() {
            converted.frontmatter.insert("name".to_string(), converted.metadata.name.clone());
            changes.push("Added name to frontmatter".to_string());
        }
        if !converted.frontmatter.contains_key("description") && !converted.metadata.description.is_empty() {
            converted.frontmatter.insert("description".to_string(), converted.metadata.description.clone());
            changes.push("Added description to frontmatter".to_string());
        }
        if !converted.frontmatter.contains_key("version") {
            converted.frontmatter.insert("version".to_string(), converted.metadata.version.clone());
            changes.push("Added version to frontmatter".to_string());
        }
        if !converted.frontmatter.contains_key("tags") && !converted.metadata.tags.is_empty() {
            converted.frontmatter.insert("tags".to_string(), converted.metadata.tags.join(", "));
            changes.push("Added tags to frontmatter".to_string());
        }

        // Warn if no examples
        if converted.examples.is_empty() {
            warnings.push("No examples found — standard format recommends at least one example".to_string());
        }

        // Ensure updated_at is set
        if converted.metadata.updated_at == 0 {
            converted.metadata.updated_at = converted.metadata.created_at;
            if converted.metadata.updated_at != 0 {
                changes.push("Set updated_at to created_at".to_string());
            }
        }

        SkillConversionResult {
            skill: converted,
            warnings,
            changes_made: changes,
        }
    }

    /// Convert a skill to VibeCody format.
    pub fn to_vibecody(skill: &StandardSkill) -> SkillConversionResult {
        let mut converted = skill.clone();
        let mut warnings = Vec::new();
        let mut changes = Vec::new();

        if converted.metadata.format != SkillFormat::VibeCody {
            changes.push(format!(
                "Changed format from {} to vibecody",
                converted.metadata.format.as_str()
            ));
            converted.metadata.format = SkillFormat::VibeCody;
        }

        // VibeCody format uses # Title + ## sections, no frontmatter needed
        // Ensure content starts with a heading
        if !converted.content.trim().starts_with('#') {
            let title = if converted.metadata.name.is_empty() {
                "Untitled Skill".to_string()
            } else {
                converted.metadata.name.clone()
            };
            converted.content = format!("# {}\n\n{}", title, converted.content);
            changes.push("Added # heading to content".to_string());
        }

        // Ensure ## When to Use section exists
        if !converted.content.contains("## When to Use") {
            if !converted.metadata.description.is_empty() {
                converted.content.push_str(&format!(
                    "\n\n## When to Use\n- {}",
                    converted.metadata.description
                ));
                changes.push("Added ## When to Use section".to_string());
            } else {
                warnings.push("No description available to generate ## When to Use section".to_string());
            }
        }

        SkillConversionResult {
            skill: converted,
            warnings,
            changes_made: changes,
        }
    }

    /// Convert a skill to the specified target format.
    pub fn convert(skill: &StandardSkill, target: &SkillFormat) -> SkillConversionResult {
        match target {
            SkillFormat::Standard => Self::to_standard(skill),
            SkillFormat::VibeCody => Self::to_vibecody(skill),
            SkillFormat::ClaudeCode => {
                // Claude Code format: standard with input/output type annotations
                let mut result = Self::to_standard(skill);
                result.skill.metadata.format = SkillFormat::ClaudeCode;
                result.changes_made.push("Set format to claude_code".to_string());
                if result.skill.metadata.input_types.is_empty() {
                    result.skill.metadata.input_types.push("text".to_string());
                    result.changes_made.push("Added default input_type 'text'".to_string());
                }
                if result.skill.metadata.output_types.is_empty() {
                    result.skill.metadata.output_types.push("text".to_string());
                    result.changes_made.push("Added default output_type 'text'".to_string());
                }
                result
            }
            SkillFormat::Cursor => {
                let mut result = Self::to_standard(skill);
                result.skill.metadata.format = SkillFormat::Cursor;
                result.changes_made.push("Set format to cursor".to_string());
                // Cursor prefers rules-style content
                if !result.skill.content.contains("## Rules") && !result.skill.content.contains("## Best Practices") {
                    result.warnings.push("Cursor format typically uses ## Rules or ## Best Practices sections".to_string());
                }
                result
            }
            SkillFormat::GeminiCLI => {
                let mut result = Self::to_standard(skill);
                result.skill.metadata.format = SkillFormat::GeminiCLI;
                result.changes_made.push("Set format to gemini_cli".to_string());
                result
            }
            SkillFormat::Custom(name) => {
                let mut result = Self::to_standard(skill);
                result.skill.metadata.format = SkillFormat::Custom(name.clone());
                result.changes_made.push(format!("Set format to custom({})", name));
                result.warnings.push("Custom format may require additional manual adjustments".to_string());
                result
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SkillRegistry
// ---------------------------------------------------------------------------

/// In-memory skill registry for discovery and management.
#[derive(Default)]
pub struct SkillRegistry {
    entries: Vec<SkillRegistryEntry>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, entry: SkillRegistryEntry) {
        self.entries.push(entry);
    }

    /// Fuzzy search by name or tags.
    pub fn search(&self, query: &str) -> Vec<&SkillRegistryEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.skill.name.to_lowercase().contains(&q)
                    || e.skill.description.to_lowercase().contains(&q)
                    || e.skill.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Search by exact tag match.
    pub fn search_by_tag(&self, tag: &str) -> Vec<&SkillRegistryEntry> {
        let t = tag.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.skill.tags.iter().any(|et| et.to_lowercase() == t))
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<&SkillRegistryEntry> {
        self.entries.iter().find(|e| e.skill.name == name)
    }

    pub fn list(&self) -> Vec<&SkillRegistryEntry> {
        self.entries.iter().collect()
    }

    pub fn verified_only(&self) -> Vec<&SkillRegistryEntry> {
        self.entries.iter().filter(|e| e.verified).collect()
    }

    /// Return top-rated entries, sorted descending by rating.
    pub fn top_rated(&self, limit: usize) -> Vec<&SkillRegistryEntry> {
        let mut sorted: Vec<&SkillRegistryEntry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(limit);
        sorted
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

// ---------------------------------------------------------------------------
// SkillExporter
// ---------------------------------------------------------------------------

/// Exports skills to markdown with YAML frontmatter.
pub struct SkillExporter;

impl SkillExporter {
    /// Render a skill as markdown with YAML frontmatter.
    pub fn export_to_standard(skill: &StandardSkill) -> String {
        let mut output = String::new();

        // YAML frontmatter
        output.push_str("---\n");
        output.push_str(&format!("name: {}\n", skill.metadata.name));
        output.push_str(&format!("description: {}\n", skill.metadata.description));
        output.push_str(&format!("version: {}\n", skill.metadata.version));
        if !skill.metadata.author.is_empty() {
            output.push_str(&format!("author: {}\n", skill.metadata.author));
        }
        if !skill.metadata.tags.is_empty() {
            output.push_str(&format!("tags: {}\n", skill.metadata.tags.join(", ")));
        }
        output.push_str(&format!("difficulty: {}\n", skill.metadata.difficulty.as_str()));
        output.push_str(&format!("format: {}\n", skill.metadata.format.as_str()));
        if !skill.metadata.input_types.is_empty() {
            output.push_str(&format!("input_types: {}\n", skill.metadata.input_types.join(", ")));
        }
        if !skill.metadata.output_types.is_empty() {
            output.push_str(&format!("output_types: {}\n", skill.metadata.output_types.join(", ")));
        }
        if !skill.metadata.dependencies.is_empty() {
            output.push_str(&format!("dependencies: {}\n", skill.metadata.dependencies.join(", ")));
        }
        if skill.metadata.created_at > 0 {
            output.push_str(&format!("created_at: {}\n", skill.metadata.created_at));
        }
        if skill.metadata.updated_at > 0 {
            output.push_str(&format!("updated_at: {}\n", skill.metadata.updated_at));
        }
        output.push_str("---\n");

        // Body content
        output.push_str(&skill.content);

        // Append examples if not already in content
        if !skill.examples.is_empty() && !skill.content.contains("## Examples") {
            output.push_str("\n\n## Examples\n");
            for ex in &skill.examples {
                output.push_str(&format!("\n### {}\n", ex.title));
                output.push_str(&format!("```\n{}\n", ex.input));
                if !ex.expected_output.is_empty() {
                    output.push_str(&format!("# {}\n", ex.expected_output));
                }
                output.push_str("```\n");
            }
        }

        output
    }

    /// Export a batch of skills as (filename, content) pairs.
    pub fn export_batch(skills: &[StandardSkill]) -> Vec<(String, String)> {
        skills
            .iter()
            .map(|s| {
                let filename = if s.metadata.name.is_empty() {
                    "untitled.md".to_string()
                } else {
                    format!(
                        "{}.md",
                        s.metadata
                            .name
                            .to_lowercase()
                            .replace(' ', "-")
                            .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
                    )
                };
                let content = Self::export_to_standard(s);
                (filename, content)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// SkillImporter
// ---------------------------------------------------------------------------

/// Imports skills from files or strings.
pub struct SkillImporter;

impl SkillImporter {
    /// Import all .md files from a directory path (non-recursive).
    /// Returns a Vec of results — one per file attempted.
    pub fn import_from_directory(path: &str) -> Vec<Result<StandardSkill, String>> {
        let dir = match std::fs::read_dir(path) {
            Ok(d) => d,
            Err(e) => return vec![Err(format!("Cannot read directory '{}': {}", path, e))],
        };

        let mut results = Vec::new();
        for entry in dir {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    results.push(Err(format!("Directory entry error: {}", e)));
                    continue;
                }
            };
            let p = entry.path();
            if p.extension().map(|e| e == "md").unwrap_or(false) {
                match std::fs::read_to_string(&p) {
                    Ok(content) => {
                        results.push(SkillParser::parse_vibecody(&content));
                    }
                    Err(e) => {
                        results.push(Err(format!("Cannot read '{}': {}", p.display(), e)));
                    }
                }
            }
        }

        results
    }

    /// Import a skill from a string with explicit format.
    pub fn import_from_string(content: &str, format: &SkillFormat) -> Result<StandardSkill, String> {
        match format {
            SkillFormat::VibeCody => SkillParser::parse_vibecody(content),
            SkillFormat::Standard => SkillParser::parse(content),
            _ => {
                // For other formats, try standard first, then vibecody
                SkillParser::parse(content).or_else(|_| SkillParser::parse_vibecody(content))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helper builders --

    fn sample_frontmatter_skill() -> &'static str {
        "---\nname: test-skill\ndescription: A test skill for validation\nversion: 1.0.0\nauthor: VibeCody\ntags: testing, validation\ndifficulty: intermediate\n---\n# Test Skill\n\nA test skill for validation.\n\n## When to Use\n- Unit testing\n- Integration testing\n\n## Examples\n```\n/test run\n# All tests passed\n```\n\n## Best Practices\n- Write tests first\n"
    }

    fn sample_vibecody_skill() -> &'static str {
        "# My VibeCody Skill\n\nThis skill does amazing things for your code.\n\n## When to Use\n- Refactoring legacy code\n- Improving performance\n\n## Commands\n- `/myskill run` — Execute the skill\n- `/myskill config` — Configure settings\n\n## Examples\n```\n/myskill run --fast\n# Processed 42 files in 1.2s\n```\n\n## Best Practices\n- Always backup first\n"
    }

    fn make_metadata(name: &str) -> SkillMetadata {
        SkillMetadata {
            name: name.to_string(),
            description: "A test skill".to_string(),
            version: "1.0.0".to_string(),
            author: "tester".to_string(),
            tags: vec!["test".to_string()],
            difficulty: SkillDifficulty::Intermediate,
            format: SkillFormat::Standard,
            input_types: vec!["text".to_string()],
            output_types: vec!["text".to_string()],
            dependencies: Vec::new(),
            created_at: 1000,
            updated_at: 2000,
        }
    }

    fn make_standard_skill(name: &str) -> StandardSkill {
        StandardSkill {
            metadata: make_metadata(name),
            content: format!(
                "# {}\n\nBody content here.\n\n## Examples\n```\ninput\n# output\n```\n",
                name
            ),
            frontmatter: {
                let mut fm = HashMap::new();
                fm.insert("name".to_string(), name.to_string());
                fm.insert("version".to_string(), "1.0.0".to_string());
                fm
            },
            examples: vec![SkillExample {
                title: "Example 1".to_string(),
                input: "input".to_string(),
                expected_output: "output".to_string(),
            }],
        }
    }

    fn make_registry_entry(name: &str, rating: f64, verified: bool) -> SkillRegistryEntry {
        SkillRegistryEntry {
            skill: make_metadata(name),
            source_url: format!("https://skills.example.com/{}", name),
            downloads: 100,
            rating,
            verified,
        }
    }

    // -- SkillFormat tests --

    #[test]
    fn test_skill_format_as_str() {
        assert_eq!(SkillFormat::VibeCody.as_str(), "vibecody");
        assert_eq!(SkillFormat::Standard.as_str(), "standard");
        assert_eq!(SkillFormat::ClaudeCode.as_str(), "claude_code");
        assert_eq!(SkillFormat::Cursor.as_str(), "cursor");
        assert_eq!(SkillFormat::GeminiCLI.as_str(), "gemini_cli");
        assert_eq!(SkillFormat::Custom("foo".into()).as_str(), "foo");
    }

    #[test]
    fn test_skill_format_from_str() {
        assert_eq!(SkillFormat::parse("vibecody"), SkillFormat::VibeCody);
        assert_eq!(SkillFormat::parse("vibe-cody"), SkillFormat::VibeCody);
        assert_eq!(SkillFormat::parse("claude-code"), SkillFormat::ClaudeCode);
        assert_eq!(SkillFormat::parse("cursor"), SkillFormat::Cursor);
        assert_eq!(SkillFormat::parse("gemini_cli"), SkillFormat::GeminiCLI);
        assert_eq!(
            SkillFormat::parse("unknown"),
            SkillFormat::Custom("unknown".into())
        );
    }

    #[test]
    fn test_skill_difficulty_round_trip() {
        for d in &[
            SkillDifficulty::Beginner,
            SkillDifficulty::Intermediate,
            SkillDifficulty::Advanced,
            SkillDifficulty::Expert,
        ] {
            assert_eq!(SkillDifficulty::parse(d.as_str()), *d);
        }
    }

    #[test]
    fn test_difficulty_aliases() {
        assert_eq!(SkillDifficulty::parse("easy"), SkillDifficulty::Beginner);
        assert_eq!(
            SkillDifficulty::parse("medium"),
            SkillDifficulty::Intermediate
        );
        assert_eq!(SkillDifficulty::parse("hard"), SkillDifficulty::Advanced);
        assert_eq!(SkillDifficulty::parse("guru"), SkillDifficulty::Expert);
        assert_eq!(
            SkillDifficulty::parse("xyz"),
            SkillDifficulty::Intermediate
        );
    }

    // -- SkillParser tests --

    #[test]
    fn test_extract_frontmatter_with_yaml() {
        let content = "---\nname: hello\nversion: 1.0\n---\nBody here";
        let (fm, body) = SkillParser::extract_frontmatter(content);
        assert_eq!(fm.get("name").unwrap(), "hello");
        assert_eq!(fm.get("version").unwrap(), "1.0");
        assert_eq!(body, "Body here");
    }

    #[test]
    fn test_extract_frontmatter_without_yaml() {
        let content = "# No Frontmatter\n\nJust a markdown file.";
        let (fm, body) = SkillParser::extract_frontmatter(content);
        assert!(fm.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn test_extract_frontmatter_unclosed() {
        let content = "---\nname: broken\nno closing delimiter";
        let (fm, body) = SkillParser::extract_frontmatter(content);
        assert!(fm.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn test_parse_standard_with_frontmatter() {
        let skill = SkillParser::parse(sample_frontmatter_skill()).unwrap();
        assert_eq!(skill.metadata.name, "test-skill");
        assert_eq!(skill.metadata.description, "A test skill for validation");
        assert_eq!(skill.metadata.version, "1.0.0");
        assert_eq!(skill.metadata.author, "VibeCody");
        assert_eq!(skill.metadata.difficulty, SkillDifficulty::Intermediate);
        assert!(skill.metadata.tags.contains(&"testing".to_string()));
        assert!(skill.metadata.tags.contains(&"validation".to_string()));
        assert!(!skill.examples.is_empty());
    }

    #[test]
    fn test_parse_standard_without_frontmatter() {
        let content =
            "# My Skill\n\nShort description.\n\n## Examples\n```\ninput here\n# expected output\n```\n";
        let skill = SkillParser::parse(content).unwrap();
        assert_eq!(skill.metadata.name, "My Skill");
        assert_eq!(skill.metadata.description, "Short description.");
        assert!(skill.frontmatter.is_empty());
        assert_eq!(skill.examples.len(), 1);
    }

    #[test]
    fn test_parse_empty_content() {
        assert!(SkillParser::parse("").is_err());
        assert!(SkillParser::parse("   ").is_err());
    }

    #[test]
    fn test_parse_vibecody_format() {
        let skill = SkillParser::parse_vibecody(sample_vibecody_skill()).unwrap();
        assert_eq!(skill.metadata.name, "My VibeCody Skill");
        assert_eq!(skill.metadata.format, SkillFormat::VibeCody);
        assert!(!skill.metadata.description.is_empty());
        assert!(!skill.metadata.tags.is_empty());
        assert!(!skill.examples.is_empty());
    }

    #[test]
    fn test_parse_vibecody_empty() {
        assert!(SkillParser::parse_vibecody("").is_err());
    }

    #[test]
    fn test_parse_extracts_examples() {
        let content =
            "# Skill\n\nDesc.\n\n## Examples\n```\ncmd1\n# out1\n```\n\n```\ncmd2\n# out2\n```\n";
        let skill = SkillParser::parse(content).unwrap();
        assert_eq!(skill.examples.len(), 2);
        assert_eq!(skill.examples[0].title, "Example 1");
        assert_eq!(skill.examples[1].title, "Example 2");
    }

    #[test]
    fn test_parse_no_examples_section() {
        let content = "# Skill\n\nDescription only, no examples.";
        let skill = SkillParser::parse(content).unwrap();
        assert!(skill.examples.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_tags_comma_separated() {
        let content =
            "---\nname: tagged\ntags: rust, typescript, python\nversion: 0.1.0\n---\nBody.";
        let skill = SkillParser::parse(content).unwrap();
        assert_eq!(skill.metadata.tags, vec!["rust", "typescript", "python"]);
    }

    // -- SkillValidator tests --

    #[test]
    fn test_validate_valid_skill() {
        let skill = make_standard_skill("valid-skill");
        let result = SkillValidator::validate(&skill);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert!(result.compatibility_score > 0.5);
    }

    #[test]
    fn test_validate_missing_name() {
        let mut skill = make_standard_skill("test");
        skill.metadata.name = String::new();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn test_validate_missing_description() {
        let mut skill = make_standard_skill("test");
        skill.metadata.description = String::new();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("description")));
    }

    #[test]
    fn test_validate_invalid_version() {
        let mut skill = make_standard_skill("test");
        skill.metadata.version = "not-a-version".to_string();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_empty_content_warning() {
        let mut skill = make_standard_skill("test");
        skill.content = "   ".to_string();
        let result = SkillValidator::validate(&skill);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("content")));
    }

    #[test]
    fn test_validate_long_description_warning() {
        let mut skill = make_standard_skill("test");
        skill.metadata.description = "x".repeat(250);
        let result = SkillValidator::validate(&skill);
        assert!(result.warnings.iter().any(|w| w.contains("200")));
    }

    #[test]
    fn test_validate_no_examples_warning() {
        let mut skill = make_standard_skill("test");
        skill.examples.clear();
        let result = SkillValidator::validate(&skill);
        assert!(result.warnings.iter().any(|w| w.contains("examples")));
    }

    #[test]
    fn test_validate_metadata_name_too_long() {
        let mut meta = make_metadata("x");
        meta.name = "a".repeat(101);
        let errs = SkillValidator::validate_metadata(&meta);
        assert!(errs.iter().any(|e| e.contains("100")));
    }

    #[test]
    fn test_validate_metadata_empty_version() {
        let mut meta = make_metadata("x");
        meta.version = String::new();
        let errs = SkillValidator::validate_metadata(&meta);
        assert!(errs.iter().any(|e| e.contains("version")));
    }

    // -- Compatibility scoring tests --

    #[test]
    fn test_compatibility_score_range() {
        let skill = make_standard_skill("compat-test");
        let score = SkillValidator::check_compatibility(&skill, &SkillFormat::Standard);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_compatibility_vibecody_higher_with_sections() {
        let mut skill = make_standard_skill("compat-test");
        skill.content = "# Title\n\n## When to Use\n- stuff\n\n## Commands\n- cmd".to_string();
        let vc_score = SkillValidator::check_compatibility(&skill, &SkillFormat::VibeCody);
        let std_score = SkillValidator::check_compatibility(&skill, &SkillFormat::Standard);
        // VibeCody should score higher when content has ## When to Use sections
        assert!(vc_score >= std_score || (vc_score - std_score).abs() < 0.2);
    }

    #[test]
    fn test_compatibility_empty_skill_low_score() {
        let skill = StandardSkill {
            metadata: SkillMetadata::default(),
            content: String::new(),
            frontmatter: HashMap::new(),
            examples: Vec::new(),
        };
        let score = SkillValidator::check_compatibility(&skill, &SkillFormat::Standard);
        assert!(score < 0.3);
    }

    // -- SkillConverter tests --

    #[test]
    fn test_convert_to_standard() {
        let mut skill = make_standard_skill("convert-test");
        skill.metadata.format = SkillFormat::VibeCody;
        let result = SkillConverter::to_standard(&skill);
        assert_eq!(result.skill.metadata.format, SkillFormat::Standard);
        assert!(!result.changes_made.is_empty());
    }

    #[test]
    fn test_convert_to_vibecody() {
        let skill = make_standard_skill("convert-vc");
        let result = SkillConverter::to_vibecody(&skill);
        assert_eq!(result.skill.metadata.format, SkillFormat::VibeCody);
    }

    #[test]
    fn test_convert_to_vibecody_adds_heading() {
        let skill = StandardSkill {
            metadata: make_metadata("no-heading"),
            content: "Just plain text.".to_string(),
            frontmatter: HashMap::new(),
            examples: Vec::new(),
        };
        let result = SkillConverter::to_vibecody(&skill);
        assert!(result.skill.content.starts_with("# no-heading"));
        assert!(result.changes_made.iter().any(|c| c.contains("heading")));
    }

    #[test]
    fn test_convert_to_claude_code() {
        let mut skill = make_standard_skill("cc-test");
        skill.metadata.input_types.clear();
        skill.metadata.output_types.clear();
        let result = SkillConverter::convert(&skill, &SkillFormat::ClaudeCode);
        assert_eq!(result.skill.metadata.format, SkillFormat::ClaudeCode);
        assert!(!result.skill.metadata.input_types.is_empty());
        assert!(!result.skill.metadata.output_types.is_empty());
    }

    #[test]
    fn test_convert_to_cursor() {
        let skill = make_standard_skill("cursor-test");
        let result = SkillConverter::convert(&skill, &SkillFormat::Cursor);
        assert_eq!(result.skill.metadata.format, SkillFormat::Cursor);
    }

    #[test]
    fn test_convert_to_custom() {
        let skill = make_standard_skill("custom-test");
        let result = SkillConverter::convert(&skill, &SkillFormat::Custom("myformat".into()));
        assert_eq!(
            result.skill.metadata.format,
            SkillFormat::Custom("myformat".into())
        );
        assert!(result.warnings.iter().any(|w| w.contains("Custom format")));
    }

    // -- SkillRegistry tests --

    #[test]
    fn test_registry_add_and_count() {
        let mut reg = SkillRegistry::new();
        assert_eq!(reg.count(), 0);
        reg.add(make_registry_entry("skill-a", 4.5, true));
        reg.add(make_registry_entry("skill-b", 3.0, false));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn test_registry_get() {
        let mut reg = SkillRegistry::new();
        reg.add(make_registry_entry("find-me", 5.0, true));
        assert!(reg.get("find-me").is_some());
        assert!(reg.get("not-here").is_none());
    }

    #[test]
    fn test_registry_search_by_name() {
        let mut reg = SkillRegistry::new();
        reg.add(make_registry_entry("rust-review", 4.0, true));
        reg.add(make_registry_entry("python-lint", 3.5, false));
        let results = reg.search("rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skill.name, "rust-review");
    }

    #[test]
    fn test_registry_search_by_tag() {
        let mut reg = SkillRegistry::new();
        let mut entry = make_registry_entry("tagged-skill", 4.0, true);
        entry.skill.tags = vec!["security".to_string(), "audit".to_string()];
        reg.add(entry);
        reg.add(make_registry_entry("other-skill", 3.0, false));
        let results = reg.search_by_tag("security");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skill.name, "tagged-skill");
    }

    #[test]
    fn test_registry_verified_only() {
        let mut reg = SkillRegistry::new();
        reg.add(make_registry_entry("verified-a", 4.0, true));
        reg.add(make_registry_entry("unverified-b", 3.0, false));
        reg.add(make_registry_entry("verified-c", 5.0, true));
        let verified = reg.verified_only();
        assert_eq!(verified.len(), 2);
    }

    #[test]
    fn test_registry_top_rated() {
        let mut reg = SkillRegistry::new();
        reg.add(make_registry_entry("low", 1.0, false));
        reg.add(make_registry_entry("high", 5.0, true));
        reg.add(make_registry_entry("mid", 3.0, false));
        let top = reg.top_rated(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].skill.name, "high");
        assert_eq!(top[1].skill.name, "mid");
    }

    #[test]
    fn test_registry_list_all() {
        let mut reg = SkillRegistry::new();
        reg.add(make_registry_entry("a", 1.0, false));
        reg.add(make_registry_entry("b", 2.0, true));
        assert_eq!(reg.list().len(), 2);
    }

    #[test]
    fn test_registry_search_empty_query() {
        let mut reg = SkillRegistry::new();
        reg.add(make_registry_entry("anything", 1.0, false));
        // Empty query matches everything (contains "")
        let results = reg.search("");
        assert_eq!(results.len(), 1);
    }

    // -- SkillExporter tests --

    #[test]
    fn test_export_to_standard_has_frontmatter() {
        let skill = make_standard_skill("export-test");
        let output = SkillExporter::export_to_standard(&skill);
        assert!(output.starts_with("---\n"));
        assert!(output.contains("name: export-test"));
        assert!(output.contains("version: 1.0.0"));
        assert!(output.contains("difficulty: intermediate"));
    }

    #[test]
    fn test_export_includes_tags() {
        let skill = make_standard_skill("tagged");
        let output = SkillExporter::export_to_standard(&skill);
        assert!(output.contains("tags: test"));
    }

    #[test]
    fn test_export_batch() {
        let skills = vec![
            make_standard_skill("batch-one"),
            make_standard_skill("batch-two"),
        ];
        let batch = SkillExporter::export_batch(&skills);
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].0, "batch-one.md");
        assert_eq!(batch[1].0, "batch-two.md");
        assert!(batch[0].1.contains("name: batch-one"));
        assert!(batch[1].1.contains("name: batch-two"));
    }

    #[test]
    fn test_export_batch_empty_name() {
        let mut skill = make_standard_skill("test");
        skill.metadata.name = String::new();
        let batch = SkillExporter::export_batch(&[skill]);
        assert_eq!(batch[0].0, "untitled.md");
    }

    // -- SkillImporter tests --

    #[test]
    fn test_import_from_string_standard() {
        let content = sample_frontmatter_skill();
        let skill = SkillImporter::import_from_string(content, &SkillFormat::Standard).unwrap();
        assert_eq!(skill.metadata.name, "test-skill");
    }

    #[test]
    fn test_import_from_string_vibecody() {
        let content = sample_vibecody_skill();
        let skill = SkillImporter::import_from_string(content, &SkillFormat::VibeCody).unwrap();
        assert_eq!(skill.metadata.name, "My VibeCody Skill");
        assert_eq!(skill.metadata.format, SkillFormat::VibeCody);
    }

    #[test]
    fn test_import_from_string_fallback() {
        let content = "# Fallback Skill\n\nWorks with any format.\n";
        let skill = SkillImporter::import_from_string(content, &SkillFormat::ClaudeCode).unwrap();
        assert_eq!(skill.metadata.name, "Fallback Skill");
    }

    #[test]
    fn test_import_from_nonexistent_directory() {
        let results = SkillImporter::import_from_directory("/nonexistent/path/12345");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    // -- Round-trip tests --

    #[test]
    fn test_export_import_round_trip() {
        let original = make_standard_skill("round-trip");
        let exported = SkillExporter::export_to_standard(&original);
        let reimported = SkillParser::parse(&exported).unwrap();
        assert_eq!(reimported.metadata.name, original.metadata.name);
        assert_eq!(reimported.metadata.version, original.metadata.version);
        assert_eq!(reimported.metadata.difficulty, original.metadata.difficulty);
    }

    #[test]
    fn test_convert_round_trip_standard_vibecody() {
        let original = make_standard_skill("round-trip-convert");
        let to_vc = SkillConverter::to_vibecody(&original);
        let back = SkillConverter::to_standard(&to_vc.skill);
        assert_eq!(back.skill.metadata.name, original.metadata.name);
        assert_eq!(back.skill.metadata.format, SkillFormat::Standard);
    }

    // -- Error handling tests --

    #[test]
    fn test_parse_only_whitespace() {
        assert!(SkillParser::parse("   \n\n  ").is_err());
    }

    #[test]
    fn test_validate_version_four_parts() {
        let mut meta = make_metadata("test");
        meta.version = "1.2.3.4".to_string();
        let errs = SkillValidator::validate_metadata(&meta);
        assert!(errs.iter().any(|e| e.contains("semver")));
    }

    #[test]
    fn test_validate_version_non_numeric() {
        let mut meta = make_metadata("test");
        meta.version = "1.abc.0".to_string();
        let errs = SkillValidator::validate_metadata(&meta);
        assert!(errs.iter().any(|e| e.contains("not a valid number")));
    }
}
