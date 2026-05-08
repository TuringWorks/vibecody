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
//! Red commit: types + signatures + tests. Impl bodies `todo!()` so the
//! tests panic at runtime — TDD red. Green commit fills in the bodies.

use std::path::{Path, PathBuf};

use anyhow::Result;
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
    pub fn summary(&self) -> String {
        todo!("B1: first non-empty line of body, leading hashes stripped");
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

    pub fn load_from(_dir: impl AsRef<Path>) -> Result<Self> {
        todo!("B1: scan dir for *.md, parse each, sort by name");
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

    pub fn list(&self, _category: Option<&str>, _query: Option<&str>) -> Vec<&Skill> {
        todo!("B1: filter by category exact-match + free-text substring across name/triggers/body");
    }

    pub fn get(&self, _name: &str) -> Option<&Skill> {
        todo!("B1: find skill by file-stem name");
    }

    pub fn categories(&self) -> Vec<String> {
        todo!("B1: distinct sorted category list, skipping unset");
    }
}

pub fn parse_skill_file(_path: &Path) -> Result<Skill> {
    todo!("B1: read file, split frontmatter, parse YAML, return Skill");
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
