//! Skill catalogue — reads VibeCody's bundled `*.md` skills (and any
//! plugin-provided skills) into an in-memory index that can be queried
//! by category, name, or free-text substring.
//!
//! Data layer behind the `list_skills` / `get_skill` MCP tools added in
//! Phase 54 (B1). Skills are authored as Markdown files with a YAML
//! frontmatter block:
//!
//! ```text
//! ---
//! triggers: ["3D modeling", "CAD"]
//! tools_allowed: ["read_file", "write_file", "bash"]
//! category: design
//! ---
//!
//! # Skill body
//!
//! Numbered guidance the agent follows when the skill is active...
//! ```
//!
//! Catalog construction is lazy and read-only — callers load a directory
//! and query it. Reload is just constructing a new `SkillCatalog`; this
//! module does not cache or watch the filesystem.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillFrontmatter {
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub tools_allowed: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub name: String,
    pub path: PathBuf,
    pub frontmatter: SkillFrontmatter,
    pub body: String,
}

impl Skill {
    /// First non-empty line of the body, stripped of any leading `#` so it
    /// reads as a one-line summary in list views.
    pub fn summary(&self) -> String {
        for line in self.body.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            return t.trim_start_matches('#').trim().to_string();
        }
        String::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SkillCatalog {
    skills: Vec<Skill>,
}

impl SkillCatalog {
    pub fn new(skills: Vec<Skill>) -> Self {
        Self { skills }
    }

    /// Load all `*.md` files in `dir` (non-recursive). Files that fail to
    /// parse are skipped — a single bad skill should not poison the rest
    /// of the catalog.
    pub fn load_from(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("read_dir {}", dir.display()))?;
        let mut skills: Vec<Skill> = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Ok(skill) = parse_skill_file(&p) {
                skills.push(skill);
            }
        }
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { skills })
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    pub fn all(&self) -> &[Skill] {
        &self.skills
    }

    /// List skills, optionally filtered by category (exact, case-sensitive)
    /// and free-text query (case-insensitive substring across name,
    /// triggers, and body).
    pub fn list(&self, category: Option<&str>, query: Option<&str>) -> Vec<&Skill> {
        let q = query.map(|s| s.to_ascii_lowercase());
        self.skills
            .iter()
            .filter(|s| match category {
                Some(c) => s.frontmatter.category.as_deref() == Some(c),
                None => true,
            })
            .filter(|s| match &q {
                Some(q) => skill_matches_query(s, q),
                None => true,
            })
            .collect()
    }

    /// Lookup by skill name (file stem). Case-sensitive.
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Distinct categories present in the catalog, sorted.
    pub fn categories(&self) -> Vec<String> {
        let mut cs: Vec<String> = self
            .skills
            .iter()
            .filter_map(|s| s.frontmatter.category.clone())
            .collect();
        cs.sort();
        cs.dedup();
        cs
    }
}

fn skill_matches_query(s: &Skill, q_lower: &str) -> bool {
    if s.name.to_ascii_lowercase().contains(q_lower) {
        return true;
    }
    if s.frontmatter
        .triggers
        .iter()
        .any(|t| t.to_ascii_lowercase().contains(q_lower))
    {
        return true;
    }
    if s.body.to_ascii_lowercase().contains(q_lower) {
        return true;
    }
    false
}

/// Parse a single `*.md` skill file. Frontmatter is optional — a file with
/// no `---` fence is treated as having empty frontmatter and the whole
/// file as the body.
pub fn parse_skill_file(path: &Path) -> Result<Skill> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read_to_string {}", path.display()))?;
    let (frontmatter_src, body) = split_frontmatter(&raw);
    let frontmatter: SkillFrontmatter = if frontmatter_src.is_empty() {
        SkillFrontmatter::default()
    } else {
        serde_yaml::from_str(frontmatter_src).unwrap_or_default()
    };
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    Ok(Skill {
        name,
        path: path.to_path_buf(),
        frontmatter,
        body: body.trim_start().to_string(),
    })
}

/// Split a Markdown source into (frontmatter, body) using the Jekyll /
/// Hugo / Obsidian `---\n…\n---\n` convention. If the source does not
/// start with `---`, frontmatter is empty and the whole input is the
/// body.
fn split_frontmatter(src: &str) -> (&str, &str) {
    let s = src.strip_prefix('\u{FEFF}').unwrap_or(src);
    if !s.starts_with("---") {
        return ("", s);
    }
    let after_open = match s.find('\n') {
        Some(i) => &s[i + 1..],
        None => return ("", s),
    };
    if let Some(idx) = after_open.find("\n---") {
        let fm = &after_open[..idx];
        let rest = &after_open[idx + 1..];
        let rest = rest.strip_prefix("---").unwrap_or(rest);
        let rest = rest.trim_start_matches('\n');
        return (fm, rest);
    }
    ("", s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_skill(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let p = dir.join(format!("{name}.md"));
        fs::write(&p, contents).unwrap();
        p
    }

    const SAMPLE_DESIGN: &str = "---
triggers: [\"3D modeling\", \"CAD\"]
tools_allowed: [\"read_file\", \"bash\"]
category: design
---

# 3D Modeling

Pick parametric tools when intent matters.
";

    const SAMPLE_AGENT: &str = "---
triggers: [\"agent\", \"planning\"]
category: agent
---

# Agent loops

Plan, act, observe, repeat.
";

    const SAMPLE_NO_FM: &str = "# No frontmatter

Just markdown body.
";

    // ── parse_skill_file ─────────────────────────────────────────────────────

    #[test]
    fn parse_skill_file_extracts_frontmatter_and_body() {
        let dir = tempdir().unwrap();
        let p = write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        let s = parse_skill_file(&p).unwrap();
        assert_eq!(s.name, "design-cad");
        assert_eq!(s.frontmatter.category.as_deref(), Some("design"));
        assert_eq!(s.frontmatter.triggers, vec!["3D modeling", "CAD"]);
        assert_eq!(s.frontmatter.tools_allowed, vec!["read_file", "bash"]);
        assert!(s.body.starts_with("# 3D Modeling"));
        assert!(s.body.contains("Pick parametric"));
    }

    #[test]
    fn parse_skill_file_handles_missing_frontmatter() {
        let dir = tempdir().unwrap();
        let p = write_skill(dir.path(), "plain", SAMPLE_NO_FM);
        let s = parse_skill_file(&p).unwrap();
        assert_eq!(s.name, "plain");
        assert!(s.frontmatter.category.is_none());
        assert!(s.frontmatter.triggers.is_empty());
        assert!(s.body.starts_with("# No frontmatter"));
    }

    #[test]
    fn skill_summary_uses_first_non_empty_line_without_hashes() {
        let dir = tempdir().unwrap();
        let p = write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        let s = parse_skill_file(&p).unwrap();
        assert_eq!(s.summary(), "3D Modeling");
    }

    // ── SkillCatalog::load_from ──────────────────────────────────────────────

    #[test]
    fn catalog_loads_all_md_files_sorted_by_name() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        write_skill(dir.path(), "agent-loops", SAMPLE_AGENT);
        write_skill(dir.path(), "plain", SAMPLE_NO_FM);
        fs::write(dir.path().join("notes.txt"), "ignored").unwrap();

        let cat = SkillCatalog::load_from(dir.path()).unwrap();
        assert_eq!(cat.len(), 3);
        let names: Vec<&str> = cat.all().iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["agent-loops", "design-cad", "plain"]);
    }

    #[test]
    fn catalog_skips_unparseable_files_without_failing_load() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "good", SAMPLE_AGENT);
        write_skill(
            dir.path(),
            "broken",
            "---\ntriggers: [unterminated\ncategory: oops\n---\n\nbody",
        );
        let cat = SkillCatalog::load_from(dir.path()).unwrap();
        assert!(cat.len() >= 1);
        assert!(cat.get("good").is_some());
    }

    // ── list / filter ────────────────────────────────────────────────────────

    #[test]
    fn list_with_no_filter_returns_all() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        write_skill(dir.path(), "agent-loops", SAMPLE_AGENT);
        let cat = SkillCatalog::load_from(dir.path()).unwrap();
        let listed = cat.list(None, None);
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn list_filters_by_category_exact_match() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        write_skill(dir.path(), "agent-loops", SAMPLE_AGENT);
        let cat = SkillCatalog::load_from(dir.path()).unwrap();

        let design = cat.list(Some("design"), None);
        assert_eq!(design.len(), 1);
        assert_eq!(design[0].name, "design-cad");

        let agent = cat.list(Some("agent"), None);
        assert_eq!(agent.len(), 1);
        assert_eq!(agent[0].name, "agent-loops");

        let none = cat.list(Some("nonexistent"), None);
        assert_eq!(none.len(), 0);
    }

    #[test]
    fn list_query_matches_name_triggers_and_body_case_insensitively() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        write_skill(dir.path(), "agent-loops", SAMPLE_AGENT);
        let cat = SkillCatalog::load_from(dir.path()).unwrap();

        let by_trigger = cat.list(None, Some("CAD"));
        assert_eq!(by_trigger.len(), 1);
        assert_eq!(by_trigger[0].name, "design-cad");

        let by_body = cat.list(None, Some("plan, act"));
        assert_eq!(by_body.len(), 1);
        assert_eq!(by_body[0].name, "agent-loops");

        let by_name = cat.list(None, Some("DESIGN"));
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].name, "design-cad");

        let none = cat.list(None, Some("nothing-here-zzz"));
        assert_eq!(none.len(), 0);
    }

    #[test]
    fn list_combines_category_and_query() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        write_skill(dir.path(), "agent-loops", SAMPLE_AGENT);
        let cat = SkillCatalog::load_from(dir.path()).unwrap();

        let hit = cat.list(Some("agent"), Some("plan"));
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].name, "agent-loops");

        let miss = cat.list(Some("agent"), Some("CAD"));
        assert_eq!(miss.len(), 0);
    }

    // ── get ───────────────────────────────────────────────────────────────────

    #[test]
    fn get_returns_skill_when_name_matches() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        let cat = SkillCatalog::load_from(dir.path()).unwrap();
        let s = cat.get("design-cad").unwrap();
        assert_eq!(s.name, "design-cad");
    }

    #[test]
    fn get_returns_none_when_name_unknown() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        let cat = SkillCatalog::load_from(dir.path()).unwrap();
        assert!(cat.get("does-not-exist").is_none());
    }

    // ── categories ───────────────────────────────────────────────────────────

    #[test]
    fn categories_returns_distinct_sorted_list() {
        let dir = tempdir().unwrap();
        write_skill(dir.path(), "design-cad", SAMPLE_DESIGN);
        write_skill(dir.path(), "agent-loops", SAMPLE_AGENT);
        write_skill(dir.path(), "plain", SAMPLE_NO_FM);
        let cat = SkillCatalog::load_from(dir.path()).unwrap();
        let cats = cat.categories();
        assert_eq!(cats, vec!["agent".to_string(), "design".to_string()]);
    }
}
