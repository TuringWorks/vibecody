//! `Skill` — a skill parsed from a VibeCody `skills/*.md` file.
//!
//! Format (validated against the shipped library): an **optional** YAML-ish
//! frontmatter block delimited by `---`, followed by a markdown body. Common
//! frontmatter keys are `triggers`, `tools_allowed`, `category` (plus the
//! variants `trigger` / `allowed_tools`, and ignored extras like `requires_bins`
//! / `description`). Array values are inline JSON (`["a", "b"]`). ~22% of the
//! shipped files have **no** frontmatter at all — the parser tolerates that.
//!
//! Parsing is infallible on content (`from_str_named` never errors); only the
//! filesystem read in [`Skill::from_file`] can fail.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// A parsed skill document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    /// Stable identifier — the file stem (matches the agent's skill loader).
    pub name: String,
    /// Intent phrases that fire this skill.
    pub triggers: Vec<String>,
    /// Tools the skill is permitted to use.
    pub tools_allowed: Vec<String>,
    /// Category bucket (`"uncategorized"` when unspecified).
    pub category: String,
    /// Markdown body (everything after the frontmatter).
    pub body: String,
    /// Rough token cost of the whole document (≈ 4 chars/token).
    pub token_estimate: usize,
}

impl Skill {
    /// Parse a skill from a file. The `name` is the file stem.
    pub fn from_file(path: &Path) -> anyhow::Result<Skill> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("skill")
            .to_string();
        Ok(Self::from_str_named(&stem, &content))
    }

    /// Parse from raw content, using `name_hint` as the identifier.
    pub fn from_str_named(name_hint: &str, content: &str) -> Skill {
        let (frontmatter, body) = split_frontmatter(content);

        let mut triggers = Vec::new();
        let mut tools_allowed = Vec::new();
        let mut category = String::new();

        for (key, value) in frontmatter {
            match key.as_str() {
                "triggers" | "trigger" => triggers = parse_str_array(&value),
                "tools_allowed" | "allowed_tools" => tools_allowed = parse_str_array(&value),
                "category" => category = unquote(&value),
                _ => {} // name/description/requires_bins/etc. — tolerated, ignored
            }
        }

        let category = if category.is_empty() {
            "uncategorized".to_string()
        } else {
            category
        };

        Skill {
            name: name_hint.to_string(),
            triggers,
            tools_allowed,
            category,
            token_estimate: content.chars().count() / 4,
            body: body.trim_start_matches('\n').to_string(),
        }
    }

    /// Return a copy with a new body (token estimate recomputed). Used by the
    /// optimizer when it applies an edit to produce a candidate skill.
    pub fn with_body(&self, body: impl Into<String>) -> Skill {
        let body = body.into();
        let mut next = self.clone();
        next.token_estimate = next.frontmatter_len() + body.chars().count() / 4;
        next.body = body;
        next
    }

    fn frontmatter_len(&self) -> usize {
        // Rough token overhead of the rendered frontmatter block.
        (self.triggers.iter().map(|t| t.len() + 4).sum::<usize>()
            + self
                .tools_allowed
                .iter()
                .map(|t| t.len() + 4)
                .sum::<usize>()
            + self.category.len()
            + 24)
            / 4
    }

    /// Reconstruct a `skills/*.md` document (frontmatter + body). This is the
    /// deployable artifact `skilloptai-rs` writes as `best_skill.md`.
    pub fn render(&self) -> String {
        let triggers = serde_json::to_string(&self.triggers).unwrap_or_else(|_| "[]".into());
        let tools = serde_json::to_string(&self.tools_allowed).unwrap_or_else(|_| "[]".into());
        format!(
            "---\ntriggers: {triggers}\ntools_allowed: {tools}\ncategory: {}\n---\n\n{}\n",
            self.category,
            self.body.trim_end()
        )
    }
}

/// Split a document into `(frontmatter key/value pairs, body)`.
///
/// A frontmatter block exists only when the file's first line is exactly `---`
/// and a later line is exactly `---`. Otherwise the whole document is the body.
fn split_frontmatter(content: &str) -> (Vec<(String, String)>, String) {
    let mut lines = content.lines();
    let first = lines.next().unwrap_or("");
    if first.trim() != "---" {
        return (Vec::new(), content.to_string());
    }

    let mut pairs = Vec::new();
    let mut body_lines: Vec<&str> = Vec::new();
    let mut closed = false;

    for line in lines {
        if !closed {
            if line.trim() == "---" {
                closed = true;
                continue;
            }
            if let Some(kv) = parse_kv(line) {
                pairs.push(kv);
            }
        } else {
            body_lines.push(line);
        }
    }

    // Unterminated frontmatter → treat the whole file as body (be lenient).
    if !closed {
        return (Vec::new(), content.to_string());
    }
    (pairs, body_lines.join("\n"))
}

/// Parse a `key: value` frontmatter line. Returns `None` for blank/comment/
/// non-`key:` lines so malformed frontmatter can't break parsing.
fn parse_kv(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let idx = line.find(':')?;
    let key = line[..idx].trim().to_string();
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some((key, line[idx + 1..].trim().to_string()))
}

/// Parse an inline JSON string array, with a lenient bracket-split fallback.
fn parse_str_array(value: &str) -> Vec<String> {
    let value = value.trim();
    if value.is_empty() {
        return Vec::new();
    }
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(value) {
        return arr;
    }
    value
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().trim_matches(['"', '\'']).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Strip a single layer of matching surrounding quotes.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'"' || bytes[0] == b'\'')
        && bytes[bytes.len() - 1] == bytes[0]
    {
        return s[1..s.len() - 1].to_string();
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_frontmatter() {
        let c = "---\ntriggers: [\"a\", \"b\"]\ntools_allowed: [\"read_file\"]\ncategory: safety-critical\n---\n\n# Title\nbody line\n";
        let s = Skill::from_str_named("my-skill", c);
        assert_eq!(s.name, "my-skill");
        assert_eq!(s.triggers, vec!["a", "b"]);
        assert_eq!(s.tools_allowed, vec!["read_file"]);
        assert_eq!(s.category, "safety-critical");
        assert!(s.body.starts_with("# Title"));
        assert!(s.body.contains("body line"));
        assert!(s.token_estimate > 0);
    }

    #[test]
    fn no_frontmatter_defaults() {
        let c = "# Just A Title\nsome body\n";
        let s = Skill::from_str_named("plain", c);
        assert_eq!(s.name, "plain");
        assert!(s.triggers.is_empty());
        assert_eq!(s.category, "uncategorized");
        assert!(s.body.contains("some body"));
    }

    #[test]
    fn tolerates_variant_keys_and_extras() {
        let c = "---\ntrigger: [\"x\"]\nallowed_tools: [\"bash\"]\ncategory: misc\nrequires_bins: [\"git\"]\ndescription: hi\n---\nbody\n";
        let s = Skill::from_str_named("v", c);
        assert_eq!(s.triggers, vec!["x"]);
        assert_eq!(s.tools_allowed, vec!["bash"]);
        assert_eq!(s.category, "misc");
    }

    #[test]
    fn unterminated_frontmatter_is_body() {
        let c = "---\ntriggers: [\"a\"]\nno closing marker here\n";
        let s = Skill::from_str_named("u", c);
        assert!(s.triggers.is_empty());
        assert_eq!(s.category, "uncategorized");
        assert!(s.body.contains("no closing marker"));
    }
}
