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

/// Where a skill came from. Surfaced in the MCP `list_skills` payload
/// and the governance UI so the user can tell at a glance whether a
/// skill is a built-in or an opt-in plugin contribution. Defaults to
/// `Builtin` so existing callers don't need to change.
#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "plugin")]
pub enum SkillSource {
    #[default]
    Builtin,
    /// Skill came from a plugin's `vibecli-plugin.toml` `[[components.skills]]`
    /// entry. The string is the owning plugin's name.
    Plugin(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub name: String,
    pub path: PathBuf,
    pub frontmatter: SkillFrontmatter,
    pub body: String,
    /// B2.7 — provenance. `Builtin` for skills loaded from the
    /// VibeCody bundled tree, `Plugin(name)` for skills contributed
    /// by an installed plugin via `vibecli-plugin.toml`.
    #[serde(default)]
    pub source: SkillSource,
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
    /// of the catalog. Skills loaded this way are tagged
    /// `SkillSource::Builtin`.
    pub fn load_from(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let entries =
            std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))?;
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

    /// B2.7 — load built-in skills from `builtin_dir` AND every skill
    /// contributed by an installed plugin whose policy is `On` or
    /// `Required` in the given workspace. Plugin skills carry
    /// `SkillSource::Plugin(plugin_name)` so the governance UI and MCP
    /// `list_skills` payload can show provenance.
    ///
    /// Failures parsing a plugin skill file are silently skipped —
    /// same conservatism as the built-in loader. Policy enforcement
    /// happens in `plugin_runtime::enabled_skills`; this loader trusts
    /// that whatever it gets back is already permitted.
    ///
    /// Name collisions: plugin skills are appended AFTER built-ins,
    /// so a built-in with the same name as a plugin contribution
    /// shadows the plugin's. This is deliberate — built-ins should be
    /// stable while a workspace adds and removes plugins.
    /// Convenience for call sites that don't carry an explicit
    /// workspace + store: detect the workspace from `std::env::current_dir()`
    /// and, if a `<workspace>/.vibecli/workspace.db` exists or can be
    /// created, load plugin skills too. Falls back to built-ins-only
    /// when cwd has no workspace store yet — keeps the daemon
    /// runnable in scratch directories where no plugins are installed.
    pub fn load_from_with_cwd_plugins(builtin_dir: impl AsRef<Path>) -> Result<Self> {
        let workspace = match std::env::current_dir() {
            Ok(d) => d,
            Err(_) => return Self::load_from(builtin_dir),
        };
        match crate::workspace_store::WorkspaceStore::open(&workspace) {
            Ok(store) => Self::load_from_with_plugins(builtin_dir, &workspace, &store),
            Err(_) => Self::load_from(builtin_dir),
        }
    }

    pub fn load_from_with_plugins(
        builtin_dir: impl AsRef<Path>,
        workspace: &Path,
        store: &crate::workspace_store::WorkspaceStore,
    ) -> Result<Self> {
        let mut cat = Self::load_from(builtin_dir)?;
        let existing_names: std::collections::HashSet<String> =
            cat.skills.iter().map(|s| s.name.clone()).collect();
        match crate::plugin_runtime::enabled_skills(workspace, store) {
            Ok(plugin_skills) => {
                for c in plugin_skills {
                    if existing_names.contains(&c.spec.name) {
                        // Built-in wins; plugin's contribution with the
                        // same name is silently dropped.
                        continue;
                    }
                    if let Ok(mut s) = parse_skill_file(&c.absolute_path) {
                        // Use the plugin's declared skill name, not the
                        // file stem — manifest is authoritative.
                        s.name = c.spec.name.clone();
                        // Respect the manifest's category hint when the
                        // skill file itself doesn't set one.
                        if s.frontmatter.category.is_none() {
                            s.frontmatter.category = c.spec.category.clone();
                        }
                        s.source = SkillSource::Plugin(c.plugin_name.clone());
                        cat.skills.push(s);
                    }
                }
            }
            Err(_) => {
                // Workspace store unavailable (e.g. no .vibecli/ yet)
                // is a benign condition — return the built-in catalog
                // unchanged rather than failing the whole load.
            }
        }
        cat.skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(cat)
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
        source: SkillSource::default(),
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

    // ── B2.7: plugin-sourced skills ──────────────────────────────────────────

    /// End-to-end: install a signed plugin bundle that ships a single
    /// skill; load `SkillCatalog` with the workspace + store; confirm
    /// the plugin's skill appears alongside the built-in and carries
    /// `SkillSource::Plugin(name)`.
    ///
    /// This test exercises the full B2.1–B2.5 stack through the
    /// public install API, which is the cheapest way to get a real
    /// `InstalledPlugin` row with a matching on-disk file.
    #[test]
    fn load_from_with_plugins_includes_enabled_plugin_skills() {
        use crate::mcpb_bundle;
        use crate::plugin_install::install_from_file;
        use crate::plugin_manifest::{
            Components, DefaultPolicy, PluginManifest, Publisher, SkillComponent,
        };
        use crate::plugin_signing::{sign_manifest, MANIFEST_FILENAME, SIGNATURE_FILENAME};
        use crate::signed_agent_card::jwk_from_verifying_key;
        use crate::workspace_store::WorkspaceStore;
        use p256::ecdsa::SigningKey;

        // Built-in catalog has one skill.
        let builtin = tempdir().unwrap();
        write_skill(builtin.path(), "agent-loops", SAMPLE_AGENT);

        // Workspace + store.
        let ws = tempdir().unwrap();
        let db = ws.path().join(".vibecli").join("workspace.db");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let store = WorkspaceStore::open_with(&db, [11u8; 32]).unwrap();

        // Build + sign a plugin bundle that ships one skill.
        let key = SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let manifest = PluginManifest {
            name: "demo".into(),
            version: "1.0.0".into(),
            publisher: Publisher {
                name: "Demo Co".into(),
                url: None,
                key: jwk_from_verifying_key(key.verifying_key()),
            },
            description: "demo".into(),
            components: Components {
                skills: vec![SkillComponent {
                    name: "demo-skill".into(),
                    path: "skills/demo.md".into(),
                    category: Some("demo".into()),
                }],
                ..Default::default()
            },
            min_vibecli_version: None,
            default_policy: DefaultPolicy::On,
        };
        let sig = sign_manifest(&manifest, &key, "k").unwrap();

        let bundle_src = tempdir().unwrap();
        // MCPB outer.
        let outer = mcpb_bundle::BundleManifest {
            name: "demo".into(),
            version: "1.0.0".into(),
            command: "noop".into(),
            args: vec![],
            env: Default::default(),
            description: None,
        };
        std::fs::write(
            bundle_src.path().join("manifest.json"),
            serde_json::to_string(&outer).unwrap(),
        )
        .unwrap();
        std::fs::write(
            bundle_src.path().join(MANIFEST_FILENAME),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(
            bundle_src.path().join(SIGNATURE_FILENAME),
            serde_json::to_string(&sig).unwrap(),
        )
        .unwrap();
        // The skill file itself, addressed by the manifest.
        std::fs::create_dir_all(bundle_src.path().join("skills")).unwrap();
        std::fs::write(
            bundle_src.path().join("skills/demo.md"),
            "---\ntriggers: [\"demo\"]\n---\n\n# Demo skill\n\nBody.\n",
        )
        .unwrap();

        let bundle_dest = tempdir().unwrap().path().join("demo.mcpb");
        std::fs::create_dir_all(bundle_dest.parent().unwrap()).unwrap();
        mcpb_bundle::pack_bundle(bundle_src.path(), &bundle_dest).unwrap();

        install_from_file(ws.path(), &store, &bundle_dest, false).unwrap();

        let cat = SkillCatalog::load_from_with_plugins(builtin.path(), ws.path(), &store).unwrap();
        assert_eq!(cat.len(), 2, "built-in + plugin skill");
        let demo = cat.get("demo-skill").expect("plugin skill present");
        assert_eq!(demo.source, SkillSource::Plugin("demo".into()));
        assert_eq!(demo.frontmatter.category.as_deref(), Some("demo"));

        let builtin_skill = cat.get("agent-loops").expect("built-in skill present");
        assert_eq!(builtin_skill.source, SkillSource::Builtin);
    }

    #[test]
    fn load_from_with_plugins_skips_off_plugins() {
        use crate::mcpb_bundle;
        use crate::plugin_install::install_from_file;
        use crate::plugin_manifest::{
            Components, DefaultPolicy, PluginManifest, Publisher, SkillComponent,
        };
        use crate::plugin_signing::{sign_manifest, MANIFEST_FILENAME, SIGNATURE_FILENAME};
        use crate::signed_agent_card::jwk_from_verifying_key;
        use crate::workspace_store::WorkspaceStore;
        use p256::ecdsa::SigningKey;

        let builtin = tempdir().unwrap();
        let ws = tempdir().unwrap();
        let db = ws.path().join(".vibecli").join("workspace.db");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let store = WorkspaceStore::open_with(&db, [22u8; 32]).unwrap();

        let key = SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let manifest = PluginManifest {
            name: "muted".into(),
            version: "1.0.0".into(),
            publisher: Publisher {
                name: "M".into(),
                url: None,
                key: jwk_from_verifying_key(key.verifying_key()),
            },
            description: "muted".into(),
            components: Components {
                skills: vec![SkillComponent {
                    name: "muted-skill".into(),
                    path: "skills/m.md".into(),
                    category: None,
                }],
                ..Default::default()
            },
            min_vibecli_version: None,
            // Default Off — installed but its skill must NOT appear.
            default_policy: DefaultPolicy::Off,
        };
        let sig = sign_manifest(&manifest, &key, "k").unwrap();

        let bundle_src = tempdir().unwrap();
        let outer = mcpb_bundle::BundleManifest {
            name: "muted".into(),
            version: "1.0.0".into(),
            command: "noop".into(),
            args: vec![],
            env: Default::default(),
            description: None,
        };
        std::fs::write(
            bundle_src.path().join("manifest.json"),
            serde_json::to_string(&outer).unwrap(),
        )
        .unwrap();
        std::fs::write(
            bundle_src.path().join(MANIFEST_FILENAME),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(
            bundle_src.path().join(SIGNATURE_FILENAME),
            serde_json::to_string(&sig).unwrap(),
        )
        .unwrap();
        std::fs::create_dir_all(bundle_src.path().join("skills")).unwrap();
        std::fs::write(bundle_src.path().join("skills/m.md"), "# Muted").unwrap();

        let dest_dir = tempdir().unwrap();
        let bundle_dest = dest_dir.path().join("muted.mcpb");
        mcpb_bundle::pack_bundle(bundle_src.path(), &bundle_dest).unwrap();
        install_from_file(ws.path(), &store, &bundle_dest, false).unwrap();

        let cat = SkillCatalog::load_from_with_plugins(builtin.path(), ws.path(), &store).unwrap();
        assert_eq!(cat.len(), 0, "Off plugin's skill must be filtered out");
        assert!(cat.get("muted-skill").is_none());
    }

    #[test]
    fn load_from_with_plugins_builtin_wins_on_name_collision() {
        // A built-in `agent-loops` and a plugin contribution named
        // `agent-loops` collide; the built-in wins.
        use crate::mcpb_bundle;
        use crate::plugin_install::install_from_file;
        use crate::plugin_manifest::{
            Components, DefaultPolicy, PluginManifest, Publisher, SkillComponent,
        };
        use crate::plugin_signing::{sign_manifest, MANIFEST_FILENAME, SIGNATURE_FILENAME};
        use crate::signed_agent_card::jwk_from_verifying_key;
        use crate::workspace_store::WorkspaceStore;
        use p256::ecdsa::SigningKey;

        let builtin = tempdir().unwrap();
        write_skill(builtin.path(), "agent-loops", SAMPLE_AGENT);

        let ws = tempdir().unwrap();
        let db = ws.path().join(".vibecli").join("workspace.db");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        let store = WorkspaceStore::open_with(&db, [33u8; 32]).unwrap();

        let key = SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let manifest = PluginManifest {
            name: "clash".into(),
            version: "1.0.0".into(),
            publisher: Publisher {
                name: "C".into(),
                url: None,
                key: jwk_from_verifying_key(key.verifying_key()),
            },
            description: "clash".into(),
            components: Components {
                skills: vec![SkillComponent {
                    // Same name as the built-in.
                    name: "agent-loops".into(),
                    path: "skills/imposter.md".into(),
                    category: None,
                }],
                ..Default::default()
            },
            min_vibecli_version: None,
            default_policy: DefaultPolicy::On,
        };
        let sig = sign_manifest(&manifest, &key, "k").unwrap();

        let bundle_src = tempdir().unwrap();
        let outer = mcpb_bundle::BundleManifest {
            name: "clash".into(),
            version: "1.0.0".into(),
            command: "noop".into(),
            args: vec![],
            env: Default::default(),
            description: None,
        };
        std::fs::write(
            bundle_src.path().join("manifest.json"),
            serde_json::to_string(&outer).unwrap(),
        )
        .unwrap();
        std::fs::write(
            bundle_src.path().join(MANIFEST_FILENAME),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(
            bundle_src.path().join(SIGNATURE_FILENAME),
            serde_json::to_string(&sig).unwrap(),
        )
        .unwrap();
        std::fs::create_dir_all(bundle_src.path().join("skills")).unwrap();
        std::fs::write(
            bundle_src.path().join("skills/imposter.md"),
            "# Imposter agent-loops",
        )
        .unwrap();

        let dest_dir = tempdir().unwrap();
        let bundle_dest = dest_dir.path().join("clash.mcpb");
        mcpb_bundle::pack_bundle(bundle_src.path(), &bundle_dest).unwrap();
        install_from_file(ws.path(), &store, &bundle_dest, false).unwrap();

        let cat = SkillCatalog::load_from_with_plugins(builtin.path(), ws.path(), &store).unwrap();
        // Exactly one `agent-loops`, and it's the built-in.
        let agent_loops: Vec<_> = cat
            .all()
            .iter()
            .filter(|s| s.name == "agent-loops")
            .collect();
        assert_eq!(agent_loops.len(), 1);
        assert_eq!(agent_loops[0].source, SkillSource::Builtin);
        // Body comes from SAMPLE_AGENT, not "Imposter".
        assert!(agent_loops[0].body.contains("Plan, act, observe"));
    }
}
