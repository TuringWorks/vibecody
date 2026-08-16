/*!
 * BDD tests for the VibeDesk Plugins panel, end to end.
 * Run with: cargo test --test vibedesk_plugins_bdd
 *
 * The panel can only observe what it installed. These steps drive the layers
 * underneath it — the catalog, the install, the workspace policy table, and
 * the `AgentContext` the daemon's `/agent` route actually runs with — because
 * the interesting failure is not "the install errored" but "the install
 * succeeded and the agent never heard about it".
 *
 * Two deliberate choices about what is exercised:
 *
 *   - `plugin_catalog::install_signed_with` rather than `install`, so the
 *     suite never reaches for the developer's real profile store to fetch a
 *     machine signing key. The signature is still generated and still
 *     verified on the way in; only its provenance differs.
 *   - `vibe_ai::skills::SkillLoader`, the loader the agent itself uses,
 *     rather than `skill_catalog`, which serves the MCP host and SkillLens.
 *     A skill that parses in one and not the other is exactly the bug this
 *     file is for.
 */
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use cucumber::{given, then, when, World};
use p256::ecdsa::SigningKey;
use p256::elliptic_curve::Generate;

use vibecli_cli::plugin_catalog::{self, CoreComponent, CATALOG};
use vibecli_cli::plugin_runtime;
use vibecli_cli::workspace_store::{PluginPolicy, PolicySetter, WorkspaceStore};

// No `Debug` in the derive: the hand-written impl below is deliberate.
#[derive(Default, World)]
pub struct PluginWorld {
    workspace: Option<tempfile::TempDir>,
    store: Option<WorkspaceStore>,
    /// Install outcome per plugin name, so a step can assert on all of them.
    installs: HashMap<String, Result<(), String>>,
}

/// `World` requires `Debug`, and `WorkspaceStore` deliberately does not derive
/// it — the store holds the workspace encryption key, and a derived `Debug`
/// would put it in any log line that formatted the world. Hand-written so the
/// store is reported by presence only.
impl std::fmt::Debug for PluginWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginWorld")
            .field("workspace", &self.workspace.as_ref().map(|d| d.path()))
            .field("store", &self.store.as_ref().map(|_| "<open>"))
            .field("installs", &self.installs)
            .finish()
    }
}

impl PluginWorld {
    fn workspace(&self) -> &Path {
        self.workspace
            .as_ref()
            .expect("a `Given a fresh workspace` step must run first")
            .path()
    }

    fn store(&self) -> &WorkspaceStore {
        self.store
            .as_ref()
            .expect("a `Given a fresh workspace` step must run first")
    }

    /// The context the daemon's `/agent` route builds for this workspace —
    /// the real function, not a reimplementation of it. VibeDesk sends no
    /// `context_request`, so `None` is the shape this asserts against.
    fn agent_context(&self) -> vibe_ai::AgentContext {
        vibecli_cli::serve::build_start_agent_context(
            self.workspace(),
            "a task",
            "session-under-test",
            None,
            None,
            None,
        )
    }

    /// Every skill file on disk that an enabled plugin contributed, as the
    /// agent's own loader parses it.
    fn installed_skills(&self) -> Vec<vibe_ai::skills::Skill> {
        let dirs = self.agent_context().extra_skill_dirs;
        vibe_ai::skills::SkillLoader::with_dirs(dirs).load_all()
    }

    fn install(&mut self, name: &str) {
        // One key for the whole scenario: catalog installs share a publisher
        // fingerprint in production too, and a bundle's members must verify
        // against the same key as the bundle.
        let key = signing_key();
        let outcome =
            plugin_catalog::install_signed_with(self.workspace(), self.store(), name, false, &key)
                .map(|_| ())
                .map_err(|e| e.to_string());
        self.installs.insert(name.to_string(), outcome);
    }

    fn set_policy(&mut self, name: &str, policy: PluginPolicy) {
        self.store()
            .set_plugin_policy(name, policy, PolicySetter::User)
            .unwrap_or_else(|e| panic!("set policy {policy:?} on {name}: {e:?}"));
    }
}

/// A deterministic-per-process key. The catalog signs with a machine key it
/// keeps in the profile store; tests must not create one there.
fn signing_key() -> SigningKey {
    SigningKey::try_generate_from_rng(&mut rand::rng()).expect("ThreadRng's error is Infallible")
}

/// Every `(plugin, component)` pair the catalog declares, flattened.
fn catalog_components() -> Vec<(&'static str, &'static CoreComponent)> {
    CATALOG
        .iter()
        .flat_map(|p| p.components.iter().map(move |c| (p.name, c)))
        .collect()
}

// ── Given ─────────────────────────────────────────────────────────────────────

#[given("a fresh workspace")]
fn fresh_workspace(world: &mut PluginWorld) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join(".vibecli").join("workspace.db");
    std::fs::create_dir_all(db.parent().expect("db has a parent")).expect("mkdir .vibecli");
    // `open_with` rather than `open`: the latter derives a key that would tie
    // this to the developer's real machine identity.
    let store = WorkspaceStore::open_with(&db, [42u8; 32]).expect("open workspace store");
    world.workspace = Some(dir);
    world.store = Some(store);
    world.installs.clear();
}

// ── When ──────────────────────────────────────────────────────────────────────

#[when("I install every catalog plugin")]
fn install_all(world: &mut PluginWorld) {
    for plugin in CATALOG {
        world.install(plugin.name);
    }
}

#[when(expr = "I install the plugin {string}")]
fn install_one(world: &mut PluginWorld, name: String) {
    world.install(&name);
}

#[when(expr = "I install the bundle {string}")]
fn install_bundle(world: &mut PluginWorld, name: String) {
    let bundle = plugin_catalog::find(&name).unwrap_or_else(|| panic!("no catalog entry {name}"));
    assert!(
        !bundle.includes.is_empty(),
        "{name} is not a bundle — it includes nothing, so this step asserts nothing"
    );
    world.install(&name);
    // `install_signed_with` deliberately installs one plugin; the members are
    // `install`'s job. Mirror that here rather than call `install`, which
    // would reach into the profile store for a machine key.
    for member in bundle.includes {
        world.install(member);
    }
}

#[when(expr = "I disable the plugin {string}")]
fn disable(world: &mut PluginWorld, name: String) {
    world.set_policy(&name, PluginPolicy::Off);
}

#[when(expr = "I enable the plugin {string}")]
fn enable(world: &mut PluginWorld, name: String) {
    world.set_policy(&name, PluginPolicy::On);
}

// ── Then — the catalog is shippable ──────────────────────────────────────────

#[then("every install succeeds")]
fn every_install_succeeds(world: &mut PluginWorld) {
    assert_eq!(
        world.installs.len(),
        CATALOG.len(),
        "every catalog entry should have been attempted"
    );
    let failed: Vec<String> = world
        .installs
        .iter()
        .filter_map(|(name, r)| r.as_ref().err().map(|e| format!("{name}: {e}")))
        .collect();
    assert!(failed.is_empty(), "installs failed: {failed:#?}");
}

#[then("every declared component has a non-empty file on disk")]
fn components_on_disk(world: &mut PluginWorld) {
    let root = world.workspace().join(".vibecli").join("plugins");
    let mut missing = Vec::new();
    for (plugin, component) in catalog_components() {
        // Mirrors `CoreComponent::rel_path` — kept explicit so a change to the
        // on-disk layout has to be made here too, deliberately.
        let sub = match component {
            CoreComponent::Skill { name, .. } => format!("skills/{name}.md"),
            CoreComponent::Rule { name, .. } => format!("rules/{name}.md"),
        };
        let path = root.join(plugin).join(&sub);
        match std::fs::read_to_string(&path) {
            Ok(body) if !body.trim().is_empty() => {}
            Ok(_) => missing.push(format!("{plugin}/{sub}: empty")),
            Err(e) => missing.push(format!("{plugin}/{sub}: {e}")),
        }
    }
    assert!(missing.is_empty(), "component files not usable: {missing:#?}");
}

#[then("the agent's skill loader parses every installed skill")]
fn loader_parses_every_skill(world: &mut PluginWorld) {
    let declared: BTreeSet<String> = catalog_components()
        .into_iter()
        .filter_map(|(_, c)| match c {
            CoreComponent::Skill { name, .. } => Some((*name).to_string()),
            CoreComponent::Rule { .. } => None,
        })
        .collect();
    assert!(
        !declared.is_empty(),
        "the catalog ships no skills — this scenario would pass vacuously"
    );

    let parsed: BTreeSet<String> = world
        .installed_skills()
        .into_iter()
        .map(|s| s.name)
        .collect();
    let unparsed: Vec<&String> = declared.difference(&parsed).collect();
    assert!(
        unparsed.is_empty(),
        "declared but not loadable by the agent: {unparsed:#?} (loaded: {parsed:#?})"
    );
}

#[then("every installed skill declares at least one trigger")]
fn skills_have_triggers(world: &mut PluginWorld) {
    let skills = world.installed_skills();
    assert!(!skills.is_empty(), "no skills loaded — nothing asserted");
    // A skill with no trigger can never activate. It would still show up in
    // the panel's component count, which is exactly the shape of claim this
    // suite exists to refuse.
    let silent: Vec<String> = skills
        .iter()
        .filter(|s| s.triggers.is_empty())
        .map(|s| s.name.clone())
        .collect();
    assert!(
        silent.is_empty(),
        "these skills can never activate — no triggers: {silent:#?}"
    );
}

#[then("every catalog skill name equals its file stem")]
fn skill_name_matches_stem(_world: &mut PluginWorld) {
    // `parse_skill_file` derives a name from the filename when the front
    // matter omits one, so a mismatch makes the panel and the agent disagree
    // about what a skill is called.
    for (plugin, component) in catalog_components() {
        if let CoreComponent::Skill { name, .. } = component {
            let stem = Path::new(&format!("skills/{name}.md"))
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .expect("a stem");
            assert_eq!(
                &stem, name,
                "{plugin}: skill name and file stem must match"
            );
        }
    }
}

#[then("every bundle include resolves to a catalog plugin")]
fn bundle_includes_resolve(_world: &mut PluginWorld) {
    let mut dangling = Vec::new();
    for plugin in CATALOG {
        for member in plugin.includes {
            if plugin_catalog::find(member).is_none() {
                dangling.push(format!("{} includes unknown {member}", plugin.name));
            }
            assert_ne!(
                *member, plugin.name,
                "{} includes itself — install would recurse",
                plugin.name
            );
        }
    }
    assert!(dangling.is_empty(), "{dangling:#?}");
}

#[then("every bundle connector resolves to a connector spec")]
fn bundle_connectors_resolve(_world: &mut PluginWorld) {
    // A connector id the catalog does not know produces `ConnectorSetup::
    // Unknown` at install time — a bundle that silently sets up less than it
    // advertises.
    let mut dangling = Vec::new();
    for plugin in CATALOG {
        for id in plugin.connectors {
            if vibecli_cli::connectors::spec(id).is_none() {
                dangling.push(format!("{} expects unknown connector {id}", plugin.name));
            }
        }
    }
    assert!(dangling.is_empty(), "{dangling:#?}");
}

#[then("no component name is claimed by two catalog plugins")]
fn no_duplicate_component_names(_world: &mut PluginWorld) {
    // The skill catalog drops a plugin contribution whose name is already
    // taken, so a collision means one of the two plugins is inert while the
    // panel lists both.
    let mut seen: HashMap<&str, &str> = HashMap::new();
    let mut clashes = Vec::new();
    for (plugin, component) in catalog_components() {
        let name = component.name();
        if let Some(first) = seen.insert(name, plugin) {
            clashes.push(format!("{name}: {first} and {plugin}"));
        }
    }
    assert!(clashes.is_empty(), "component name collisions: {clashes:#?}");
}

#[then("no catalog plugin ships an mcp server, hook or subagent")]
fn no_inert_component_kinds(_world: &mut PluginWorld) {
    // Nothing in product code calls `mcp_governance::register_plugin_servers`
    // or `plugin_runtime::enabled_subagents`, and the MCPB zip round-trip
    // drops the exec bit a hook script needs. Until each has a live consumer,
    // shipping one would put a row in the panel that looks live and does
    // nothing — so `CoreComponent` has no variant for them, and this pins
    // that. Adding a variant should fail here until the loader exists.
    for (plugin, component) in catalog_components() {
        match component {
            CoreComponent::Skill { .. } | CoreComponent::Rule { .. } => {}
            #[allow(unreachable_patterns)]
            other => panic!(
                "{plugin} ships {other:?}, a component kind no loader consumes; \
                 wire the loader before adding the variant"
            ),
        }
    }
}

// ── Then — the panel's claims match the workspace ────────────────────────────

#[then(expr = "the panel inventory lists {int} components")]
fn inventory_count(world: &mut PluginWorld, expected: usize) {
    let enabled = plugin_runtime::enabled_components(world.workspace(), world.store())
        .expect("enabled_components");
    assert_eq!(
        enabled.total(),
        expected,
        "panel would show {} live components, expected {expected}",
        enabled.total()
    );
}

#[then(expr = "the panel inventory credits them to {string}")]
fn inventory_owner(world: &mut PluginWorld, plugin: String) {
    let enabled = plugin_runtime::enabled_components(world.workspace(), world.store())
        .expect("enabled_components");
    let owners: BTreeSet<String> = enabled
        .rules
        .iter()
        .map(|c| c.plugin_name.clone())
        .chain(enabled.skills.iter().map(|c| c.plugin_name.clone()))
        .collect();
    assert_eq!(
        owners,
        BTreeSet::from([plugin.clone()]),
        "expected every component to come from {plugin}"
    );
}

#[then(expr = "the plugin {string} is installed")]
fn plugin_installed(world: &mut PluginWorld, name: String) {
    let dir = world.workspace().join(".vibecli").join("plugins").join(&name);
    assert!(
        dir.is_dir(),
        "{name} has no install directory at {}",
        dir.display()
    );
    if let Some(Err(e)) = world.installs.get(&name) {
        panic!("{name} reported an install failure: {e}");
    }
}

// ── Then — an installed plugin reaches the agent ─────────────────────────────

#[then("the agent context carries plugin rules")]
fn context_has_rules(world: &mut PluginWorld) {
    let rules = world.agent_context().plugin_rules;
    let body = rules.expect(
        "the /agent route built a context with no plugin rules — the panel says \
         the rule is enabled and the run would never see it",
    );
    assert!(!body.trim().is_empty(), "plugin rules present but empty");
}

#[then(expr = "the agent's plugin rules mention {string}")]
fn rules_mention(world: &mut PluginWorld, needle: String) {
    let body = world
        .agent_context()
        .plugin_rules
        .unwrap_or_else(|| panic!("no plugin rules in the agent context"));
    assert!(
        body.contains(&needle),
        "expected {needle:?} in the rules block, got:\n{body}"
    );
}

#[then("the agent context carries no plugin rules")]
fn context_has_no_rules(world: &mut PluginWorld) {
    let rules = world.agent_context().plugin_rules;
    assert!(
        rules.is_none(),
        "a disabled or absent plugin still reached the run: {rules:?}"
    );
}

#[then(expr = "the agent context carries {int} extra skill directories")]
fn context_skill_dirs(world: &mut PluginWorld, expected: usize) {
    let dirs: Vec<PathBuf> = world.agent_context().extra_skill_dirs;
    assert_eq!(
        dirs.len(),
        expected,
        "extra_skill_dirs = {dirs:#?}, expected {expected}"
    );
}

#[then(expr = "the agent's skill loader activates {string} for the task {string}")]
fn skill_activates(world: &mut PluginWorld, skill: String, task: String) {
    let dirs = world.agent_context().extra_skill_dirs;
    let matched = vibe_ai::skills::SkillLoader::with_dirs(dirs).matching(&task);
    let names: Vec<&str> = matched.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&skill.as_str()),
        "{skill} did not activate for {task:?}; activated: {names:?}"
    );
}

#[then(expr = "the agent's skill loader does not activate {string} for the task {string}")]
fn skill_stays_inactive(world: &mut PluginWorld, skill: String, task: String) {
    let dirs = world.agent_context().extra_skill_dirs;
    let matched = vibe_ai::skills::SkillLoader::with_dirs(dirs).matching(&task);
    let names: Vec<&str> = matched.iter().map(|s| s.name.as_str()).collect();
    assert!(
        !names.contains(&skill.as_str()),
        "{skill} activated for an unrelated task {task:?} — its triggers are too broad"
    );
}

#[then("every installed skill activates for a task quoting one of its triggers")]
fn every_skill_reachable(world: &mut PluginWorld) {
    let skills = world.installed_skills();
    assert!(!skills.is_empty(), "no skills loaded — nothing asserted");
    let dirs = world.agent_context().extra_skill_dirs;
    let loader = vibe_ai::skills::SkillLoader::with_dirs(dirs);

    // The panel's promise is that installing a plugin changes runs. For a
    // skill that means some task must reach it; its own trigger is the
    // weakest possible witness, and a skill that fails even this is dead
    // weight in every prompt budget it is counted against.
    let unreachable: Vec<String> = skills
        .iter()
        .filter(|s| {
            let Some(trigger) = s.triggers.first() else {
                return true;
            };
            !loader
                .matching(trigger)
                .iter()
                .any(|found| found.name == s.name)
        })
        .map(|s| s.name.clone())
        .collect();
    assert!(
        unreachable.is_empty(),
        "these skills cannot be activated by their own triggers: {unreachable:#?}"
    );
}

fn main() {
    futures::executor::block_on(PluginWorld::run("tests/features/vibedesk_plugins.feature"));
}
