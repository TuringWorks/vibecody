//! Runtime view of installed plugins: components filtered by policy.
//!
//! B2.5 of the plugin-bundle work. The install layer (B2.4) lays out
//! plugins on disk; the policy layer (B2.3) decides which ones are
//! active in this workspace. This module is the join — it walks the
//! install dir, consults `WorkspaceStore::effective_plugin_policy`
//! for each plugin, and returns only the components that should
//! actually load.
//!
//! The existing component loaders (skill_catalog, mcp_governance,
//! hook_abort, rules loader) consume the per-kind lists from
//! `EnabledComponents` and concat them with their existing sources.
//! No loader changes shape — they just see a longer input list.
//!
//! Patent-distance anchor (fit-gap §18 principle #2): policy
//! enforcement is client-side. Components from a plugin whose policy
//! is `Off` (or has no row at all — the "unknown plugin" safe
//! default) are not enumerated; they cannot run.

use std::path::{Path, PathBuf};

use crate::plugin_install::{list_installed, InstalledPlugin};
use crate::plugin_manifest::{
    HookComponent, McpServerComponent, RuleComponent, SkillComponent, SubagentComponent,
};
use crate::workspace_store::{PluginPolicy, WorkspaceStore};

/// One component plus the metadata needed to load it: the owning
/// plugin's name and the absolute path it resolves to under the
/// install dir. Generic over the component kind so each loader gets
/// the exact spec type it's used to working with.
#[derive(Debug, Clone)]
pub struct PluginComponent<T> {
    /// Name of the owning plugin (so loaders can report "skill X is
    /// from plugin Y" in errors and the governance panel).
    pub plugin_name: String,
    /// The component spec verbatim from `vibecli-plugin.toml`.
    pub spec: T,
    /// Absolute path the component's relative `path` resolves to
    /// under `<workspace>/.vibecli/plugins/<plugin_name>/`.
    pub absolute_path: PathBuf,
    /// Current effective policy (`On` or `Required`). `Off` components
    /// never appear here, so this field is always one of those two.
    pub policy: PluginPolicy,
}

/// All currently-enabled components, partitioned by kind. Each loader
/// reads the field it cares about and ignores the rest.
#[derive(Debug, Default, Clone)]
pub struct EnabledComponents {
    pub mcp_servers: Vec<PluginComponent<McpServerComponent>>,
    pub skills: Vec<PluginComponent<SkillComponent>>,
    pub subagents: Vec<PluginComponent<SubagentComponent>>,
    pub rules: Vec<PluginComponent<RuleComponent>>,
    pub hooks: Vec<PluginComponent<HookComponent>>,
}

impl EnabledComponents {
    /// Total count across all kinds — handy for the governance panel
    /// summary line and as a sanity check in callers.
    pub fn total(&self) -> usize {
        self.mcp_servers.len()
            + self.skills.len()
            + self.subagents.len()
            + self.rules.len()
            + self.hooks.len()
    }
}

/// Build the live `EnabledComponents` snapshot for a workspace.
///
/// Walks `<workspace>/.vibecli/plugins/`, parses each install's
/// manifest, consults `store.effective_plugin_policy(name)`, and
/// includes a plugin's components iff the policy is `On` or
/// `Required`. `Off` (and absent rows) are filtered out.
///
/// Errors only on `WorkspaceStore` failures — a corrupted plugin
/// install is silently skipped (same conservatism as
/// `plugin_install::list_installed`). One bad install must not be
/// able to silently disable all the others.
pub fn enabled_components(
    workspace: &Path,
    store: &WorkspaceStore,
) -> Result<EnabledComponents, crate::workspace_store::PolicyError> {
    let mut out = EnabledComponents::default();
    let installed = list_installed(workspace, store).map_err(|e| {
        // list_installed only wraps IO + signature errors; map them
        // into PolicyError::Db for uniform handling at the call site.
        crate::workspace_store::PolicyError::Db(e.to_string())
    })?;
    for plugin in installed {
        if !is_active_policy(plugin.policy) {
            continue;
        }
        push_plugin_components(&plugin, &mut out);
    }
    Ok(out)
}

/// Equivalent to `enabled_components` but only the `skills` field.
/// Slim wrapper so the skill_catalog hot path doesn't allocate the
/// other four vectors. Same filtering rules apply.
pub fn enabled_skills(
    workspace: &Path,
    store: &WorkspaceStore,
) -> Result<Vec<PluginComponent<SkillComponent>>, crate::workspace_store::PolicyError> {
    Ok(enabled_components(workspace, store)?.skills)
}

/// MCP-server-only convenience for the mcp_governance loader.
pub fn enabled_mcp_servers(
    workspace: &Path,
    store: &WorkspaceStore,
) -> Result<Vec<PluginComponent<McpServerComponent>>, crate::workspace_store::PolicyError> {
    Ok(enabled_components(workspace, store)?.mcp_servers)
}

/// Hooks-only convenience for the hook_abort loader.
pub fn enabled_hooks(
    workspace: &Path,
    store: &WorkspaceStore,
) -> Result<Vec<PluginComponent<HookComponent>>, crate::workspace_store::PolicyError> {
    Ok(enabled_components(workspace, store)?.hooks)
}

/// Rules-only convenience for the rules loader.
pub fn enabled_rules(
    workspace: &Path,
    store: &WorkspaceStore,
) -> Result<Vec<PluginComponent<RuleComponent>>, crate::workspace_store::PolicyError> {
    Ok(enabled_components(workspace, store)?.rules)
}

/// Subagents-only convenience for the subagent loader.
pub fn enabled_subagents(
    workspace: &Path,
    store: &WorkspaceStore,
) -> Result<Vec<PluginComponent<SubagentComponent>>, crate::workspace_store::PolicyError> {
    Ok(enabled_components(workspace, store)?.subagents)
}

/// `On` and `Required` both mean "active". `Off` does not. Centralised
/// so a future policy variant (e.g. `Suspended`) has one place to
/// declare its activity.
fn is_active_policy(p: PluginPolicy) -> bool {
    matches!(p, PluginPolicy::On | PluginPolicy::Required)
}

fn push_plugin_components(plugin: &InstalledPlugin, out: &mut EnabledComponents) {
    let base = &plugin.install_dir;
    let policy = plugin.policy;
    let name = &plugin.manifest.name;

    for c in &plugin.manifest.components.mcp_servers {
        out.mcp_servers.push(PluginComponent {
            plugin_name: name.clone(),
            spec: c.clone(),
            absolute_path: base.join(&c.path),
            policy,
        });
    }
    for c in &plugin.manifest.components.skills {
        out.skills.push(PluginComponent {
            plugin_name: name.clone(),
            spec: c.clone(),
            absolute_path: base.join(&c.path),
            policy,
        });
    }
    for c in &plugin.manifest.components.subagents {
        out.subagents.push(PluginComponent {
            plugin_name: name.clone(),
            spec: c.clone(),
            absolute_path: base.join(&c.path),
            policy,
        });
    }
    for c in &plugin.manifest.components.rules {
        out.rules.push(PluginComponent {
            plugin_name: name.clone(),
            spec: c.clone(),
            absolute_path: base.join(&c.path),
            policy,
        });
    }
    for c in &plugin.manifest.components.hooks {
        out.hooks.push(PluginComponent {
            plugin_name: name.clone(),
            spec: c.clone(),
            absolute_path: base.join(&c.path),
            policy,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcpb_bundle;
    use crate::plugin_install::{install_from_file, plugin_install_dir};
    use crate::plugin_manifest::{
        Components, DefaultPolicy, HookComponent, McpServerComponent, Publisher, RuleComponent,
        SkillComponent, SubagentComponent,
    };
    use crate::plugin_signing::{sign_manifest, MANIFEST_FILENAME, SIGNATURE_FILENAME};
    use crate::signed_agent_card::jwk_from_verifying_key;
    use crate::workspace_store::{PluginPolicy, PolicySetter};
    use p256::ecdsa::SigningKey;
    use std::fs;
    use tempfile::tempdir;

    fn temp_workspace() -> (tempfile::TempDir, WorkspaceStore) {
        let dir = tempdir().unwrap();
        let db = dir.path().join(".vibecli").join("workspace.db");
        fs::create_dir_all(db.parent().unwrap()).unwrap();
        let store = WorkspaceStore::open_with(&db, [7u8; 32]).unwrap();
        (dir, store)
    }

    fn fixture_key() -> SigningKey {
        SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng)
    }

    /// Build a signed MCPB bundle with all five component kinds, one
    /// of each. Returns the bundle path.
    fn build_full_bundle(tmp: &Path, name: &str, policy: DefaultPolicy) -> std::path::PathBuf {
        let key = fixture_key();
        let manifest = crate::plugin_manifest::PluginManifest {
            name: name.to_string(),
            version: "1.0.0".into(),
            publisher: Publisher {
                name: "Test".into(),
                url: None,
                key: jwk_from_verifying_key(key.verifying_key()),
            },
            description: format!("{name} fixture"),
            components: Components {
                mcp_servers: vec![McpServerComponent {
                    name: format!("{name}-mcp"),
                    path: "bin/srv".into(),
                    args: vec![],
                }],
                skills: vec![SkillComponent {
                    name: format!("{name}-skill"),
                    path: "skills/s.md".into(),
                    category: None,
                }],
                subagents: vec![SubagentComponent {
                    name: format!("{name}-sub"),
                    path: "agents/a.toml".into(),
                }],
                rules: vec![RuleComponent {
                    name: format!("{name}-rule"),
                    path: "rules/r.md".into(),
                }],
                hooks: vec![HookComponent {
                    name: format!("{name}-hook"),
                    event: "PreToolUse".into(),
                    path: "hooks/h.sh".into(),
                }],
            },
            min_vibecli_version: None,
            default_policy: policy,
        };
        let sig = sign_manifest(&manifest, &key, "k").unwrap();

        let src = tmp.join(format!("src.{name}"));
        fs::create_dir_all(&src).unwrap();
        let outer = mcpb_bundle::BundleManifest {
            name: name.to_string(),
            version: "1.0.0".into(),
            command: "noop".into(),
            args: vec![],
            env: Default::default(),
            description: None,
        };
        fs::write(
            src.join("manifest.json"),
            serde_json::to_string(&outer).unwrap(),
        )
        .unwrap();
        fs::write(
            src.join(MANIFEST_FILENAME),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(
            src.join(SIGNATURE_FILENAME),
            serde_json::to_string(&sig).unwrap(),
        )
        .unwrap();

        let bundle = tmp.join(format!("{name}-1.0.0.mcpb"));
        mcpb_bundle::pack_bundle(&src, &bundle).unwrap();
        bundle
    }

    #[test]
    fn enabled_components_excludes_off_plugins() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let b_on = build_full_bundle(tmp.path(), "live", DefaultPolicy::On);
        let b_off = build_full_bundle(tmp.path(), "dead", DefaultPolicy::Off);

        install_from_file(ws_dir.path(), &store, &b_on, false).unwrap();
        install_from_file(ws_dir.path(), &store, &b_off, false).unwrap();

        let enabled = enabled_components(ws_dir.path(), &store).unwrap();
        assert_eq!(
            enabled.total(),
            5,
            "only 'live' contributes its 5 components"
        );
        // Every entry belongs to 'live', never 'dead'.
        assert!(enabled.skills.iter().all(|c| c.plugin_name == "live"));
        assert!(enabled.hooks.iter().all(|c| c.plugin_name == "live"));
    }

    #[test]
    fn enabled_components_includes_required_plugins() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let b = build_full_bundle(tmp.path(), "pinned", DefaultPolicy::On);
        install_from_file(ws_dir.path(), &store, &b, false).unwrap();
        store
            .set_plugin_policy("pinned", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();

        let enabled = enabled_components(ws_dir.path(), &store).unwrap();
        assert_eq!(enabled.total(), 5);
        assert!(enabled
            .skills
            .iter()
            .all(|c| c.policy == PluginPolicy::Required));
    }

    #[test]
    fn enabled_components_resolves_absolute_paths_under_install_dir() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let b = build_full_bundle(tmp.path(), "paths", DefaultPolicy::On);
        install_from_file(ws_dir.path(), &store, &b, false).unwrap();

        let enabled = enabled_components(ws_dir.path(), &store).unwrap();
        let skill = &enabled.skills[0];
        let expected = plugin_install_dir(ws_dir.path())
            .join("paths")
            .join("skills/s.md");
        assert_eq!(skill.absolute_path, expected);
    }

    #[test]
    fn user_lowering_on_to_off_immediately_excludes_components() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let b = build_full_bundle(tmp.path(), "flip", DefaultPolicy::On);
        install_from_file(ws_dir.path(), &store, &b, false).unwrap();
        assert_eq!(
            enabled_components(ws_dir.path(), &store).unwrap().total(),
            5
        );

        // User flips it off — components must disappear from the next
        // snapshot (no caching, fresh read each call).
        store
            .set_plugin_policy("flip", PluginPolicy::Off, PolicySetter::User)
            .unwrap();
        assert_eq!(
            enabled_components(ws_dir.path(), &store).unwrap().total(),
            0
        );
    }

    #[test]
    fn enabled_skills_convenience_matches_full_snapshot() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let b = build_full_bundle(tmp.path(), "match", DefaultPolicy::On);
        install_from_file(ws_dir.path(), &store, &b, false).unwrap();

        let full = enabled_components(ws_dir.path(), &store).unwrap();
        let skills = enabled_skills(ws_dir.path(), &store).unwrap();
        assert_eq!(full.skills.len(), skills.len());
        assert_eq!(full.skills[0].plugin_name, skills[0].plugin_name);
    }

    #[test]
    fn enabled_components_on_empty_workspace_returns_empty() {
        let (ws_dir, store) = temp_workspace();
        let enabled = enabled_components(ws_dir.path(), &store).unwrap();
        assert_eq!(enabled.total(), 0);
    }

    #[test]
    fn three_plugins_three_policies_only_active_appear() {
        let (ws_dir, store) = temp_workspace();
        let tmp = tempdir().unwrap();
        let b1 = build_full_bundle(tmp.path(), "p-on", DefaultPolicy::On);
        let b2 = build_full_bundle(tmp.path(), "p-off", DefaultPolicy::Off);
        let b3 = build_full_bundle(tmp.path(), "p-req", DefaultPolicy::On);

        install_from_file(ws_dir.path(), &store, &b1, false).unwrap();
        install_from_file(ws_dir.path(), &store, &b2, false).unwrap();
        install_from_file(ws_dir.path(), &store, &b3, false).unwrap();
        store
            .set_plugin_policy("p-req", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();

        let enabled = enabled_components(ws_dir.path(), &store).unwrap();
        // p-on + p-req contribute 5 each = 10; p-off contributes 0.
        assert_eq!(enabled.total(), 10);
        let names: std::collections::HashSet<String> = enabled
            .skills
            .iter()
            .map(|c| c.plugin_name.clone())
            .collect();
        assert!(names.contains("p-on"));
        assert!(names.contains("p-req"));
        assert!(!names.contains("p-off"));
    }
}
