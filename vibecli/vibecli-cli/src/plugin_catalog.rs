//! Core plugins that ship inside the binary, installable without a registry.
//!
//! The plugin stack could already install, verify, police and load bundles —
//! but the only way to get a bundle was to author, sign and pack one yourself,
//! so in practice every workspace had zero plugins and the Plugins panel had
//! nothing to show but a sentence pointing at the CLI. This module closes that
//! gap: a small first-party catalog compiled into the binary, installable in
//! one call, offline, with no registry to host and nothing to download.
//!
//! **Only skills and rules.** Both are plain Markdown that existing loaders
//! already read — `skill_catalog` merges plugin skills into the catalog and
//! `context_assembler::collect_plugin_rules` injects plugin rules into every
//! run — so an installed catalog plugin demonstrably changes what the agent
//! sees. Hooks and MCP servers are deliberately absent: a hook component is an
//! executable path, and the zip round-trip through MCPB does not carry the
//! exec bit, while plugin MCP servers are only registered by `mcp_governance`,
//! which nothing currently calls. Shipping either would put an entry in the
//! panel that looks live and does nothing.
//!
//! Installation reuses [`plugin_install::install_from_dir`] — the same
//! signature verification, atomic swap and policy write as a downloaded
//! bundle. The signature is real but attests integrity, not provenance: the
//! catalog is signed on this machine with a locally generated P-256 key (see
//! [`publisher_key`]), which proves an install has not been altered since it
//! was written, and proves nothing about who wrote the catalog. The embedded
//! publisher key in a third-party bundle carries exactly the same weight —
//! that is the trust model this format has.

use std::path::Path;

use p256::ecdsa::SigningKey;
// `try_generate_from_rng` is an extension-trait method, not an inherent one.
use p256::elliptic_curve::Generate;

use crate::plugin_install::{self, InstallError, InstalledPlugin};
use crate::plugin_manifest::{
    Components, DefaultPolicy, PluginManifest, Publisher, RuleComponent, SkillComponent,
};
use crate::plugin_signing::{self, MANIFEST_FILENAME, SIGNATURE_FILENAME};
use crate::signed_agent_card::jwk_from_verifying_key;
use crate::workspace_store::WorkspaceStore;

/// One component of a catalog plugin. Both variants are a Markdown file
/// written into the install directory; the enum exists so the manifest
/// builder knows which component list to put it in.
#[derive(Debug, Clone, Copy)]
pub enum CoreComponent {
    /// Merged into the skill catalog. `name` must equal the file stem —
    /// `parse_skill_file` derives the skill's name from the filename, so a
    /// mismatch means the panel and the agent disagree about what it is called.
    Skill {
        name: &'static str,
        category: &'static str,
        body: &'static str,
    },
    /// Injected into the assembled context of every run in the workspace.
    Rule {
        name: &'static str,
        body: &'static str,
    },
}

impl CoreComponent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Skill { name, .. } | Self::Rule { name, .. } => name,
        }
    }

    /// Path inside the install directory. Kept in `skills/` and `rules/`
    /// subdirectories so an operator reading `.vibecli/plugins/<name>/` can
    /// tell what a file is without opening the manifest.
    fn rel_path(&self) -> String {
        match self {
            Self::Skill { name, .. } => format!("skills/{name}.md"),
            Self::Rule { name, .. } => format!("rules/{name}.md"),
        }
    }

    fn body(&self) -> &'static str {
        match self {
            Self::Skill { body, .. } | Self::Rule { body, .. } => body,
        }
    }

    /// Wire label for the API and the panel.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Skill { .. } => "skill",
            Self::Rule { .. } => "rule",
        }
    }
}

/// A plugin the user can install from the panel with one click.
#[derive(Debug, Clone, Copy)]
pub struct CorePlugin {
    /// Manifest name and install-directory name. Kebab-case.
    pub name: &'static str,
    /// Human title for the catalog card.
    pub title: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub components: &'static [CoreComponent],
}

/// The catalog itself.
///
/// Kept small on purpose. Each entry has to earn its place by changing what
/// the agent does; a catalog padded with plausible-sounding plugins is a
/// catalog nobody reads.
pub const CATALOG: &[CorePlugin] = &[
    CorePlugin {
        name: "core-review-standards",
        title: "Review standards",
        version: "1.0.0",
        description: "What a code review must check before it approves — \
                      correctness first, then the failure modes a green build misses.",
        components: &[CoreComponent::Rule {
            name: "review-standards",
            body: REVIEW_STANDARDS,
        }],
    },
    CorePlugin {
        name: "core-secure-defaults",
        title: "Secure defaults",
        version: "1.0.0",
        description: "Secrets stay out of the repo and the transcript; input is \
                      bounded before it is allocated; auth checks are never weakened \
                      to make a test pass.",
        components: &[
            CoreComponent::Rule {
                name: "secret-handling",
                body: SECRET_HANDLING,
            },
            CoreComponent::Rule {
                name: "untrusted-input",
                body: UNTRUSTED_INPUT,
            },
        ],
    },
    CorePlugin {
        name: "core-commit-craft",
        title: "Commit craft",
        version: "1.0.0",
        description: "A skill for writing commit messages that say why, in the \
                      shape reviewers and `git log` expect.",
        components: &[CoreComponent::Skill {
            name: "commit-craft",
            category: "workflow",
            body: COMMIT_CRAFT,
        }],
    },
    CorePlugin {
        name: "core-test-first",
        title: "Test first",
        version: "1.0.0",
        description: "A skill for pinning behaviour with a failing test before \
                      changing it — and for telling a real test from a vacuous one.",
        components: &[CoreComponent::Skill {
            name: "test-first",
            category: "testing",
            body: TEST_FIRST,
        }],
    },
];

/// Look up a catalog entry by name.
pub fn find(name: &str) -> Option<&'static CorePlugin> {
    CATALOG.iter().find(|p| p.name == name)
}

/// Where the catalog's signing key lives in the profile store.
const SIGNING_KEY_PANEL: &str = "plugins";
const SIGNING_KEY_NAME: &str = "catalog_signing_key";
/// `kid` recorded in every catalog signature.
pub const CATALOG_KID: &str = "vibecody-local-catalog";

/// The key catalog manifests are signed with, and whether it survived.
pub struct PublisherKey {
    pub key: SigningKey,
    /// `false` when the profile store could not be reached and the key was
    /// generated for this call only.
    ///
    /// The install is just as valid either way — verification uses the key
    /// embedded in the manifest — but the fingerprint an operator sees will
    /// change on the next install, so callers surface this rather than let a
    /// rotating identity look like a stable one.
    pub persisted: bool,
}

/// Load the local catalog signing key, generating and storing it on first use.
///
/// One key per machine rather than per install, so every catalog plugin in
/// every workspace carries the same publisher fingerprint and an operator can
/// tell "these all came from this machine's catalog" apart from "someone
/// hand-built a bundle".
pub fn publisher_key() -> PublisherKey {
    let generate = || {
        SigningKey::try_generate_from_rng(&mut rand::rng())
            .expect("ThreadRng's error type is Infallible")
    };
    let store = match crate::profile_store::ProfileStore::new() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "plugin catalog: profile store unavailable; signing with an ephemeral key"
            );
            return PublisherKey {
                key: generate(),
                persisted: false,
            };
        }
    };
    let profile = store
        .get_default_profile_id()
        .unwrap_or_else(|_| "default".to_string());

    if let Ok(Some(hex_key)) = store.get(&profile, SIGNING_KEY_PANEL, SIGNING_KEY_NAME) {
        match hex::decode(&hex_key)
            .ok()
            .and_then(|b| SigningKey::from_slice(&b).ok())
        {
            Some(key) => {
                return PublisherKey {
                    key,
                    persisted: true,
                }
            }
            // A stored value that will not parse is corruption, not a reason
            // to fail the install: regenerate and overwrite below.
            None => {
                tracing::warn!("plugin catalog: stored signing key is unreadable; regenerating")
            }
        }
    }

    let key = generate();
    let persisted = store
        .set(
            &profile,
            SIGNING_KEY_PANEL,
            SIGNING_KEY_NAME,
            &hex::encode(key.to_bytes()),
        )
        .inspect_err(|e| {
            tracing::warn!(error = %e, "plugin catalog: could not persist the signing key");
        })
        .is_ok();
    PublisherKey { key, persisted }
}

/// Build the manifest for a catalog entry, bound to `key` as publisher.
fn manifest_for(plugin: &CorePlugin, key: &SigningKey) -> PluginManifest {
    let mut components = Components::default();
    for c in plugin.components {
        match c {
            CoreComponent::Skill { name, category, .. } => components.skills.push(SkillComponent {
                name: (*name).to_string(),
                path: c.rel_path(),
                category: Some((*category).to_string()),
            }),
            CoreComponent::Rule { name, .. } => components.rules.push(RuleComponent {
                name: (*name).to_string(),
                path: c.rel_path(),
            }),
        }
    }
    PluginManifest {
        name: plugin.name.to_string(),
        version: plugin.version.to_string(),
        publisher: Publisher {
            // Named for what it is. "VibeCody" alone would read as a signature
            // from the project; this key belongs to whoever runs this machine.
            name: "VibeCody core catalog (locally signed)".to_string(),
            url: Some("https://github.com/TuringWorks/vibecody".to_string()),
            key: jwk_from_verifying_key(key.verifying_key()),
        },
        description: plugin.description.to_string(),
        components,
        min_vibecli_version: None,
        // The user asked for this plugin by installing it, so it takes effect
        // immediately. `Required` is admin territory and is never set here —
        // it cannot be lowered without admin, and a one-click install must not
        // be able to pin something the same user cannot remove.
        default_policy: DefaultPolicy::On,
    }
}

/// What an install actually did, for the caller to report.
#[derive(Debug)]
pub struct CatalogInstall {
    pub installed: InstalledPlugin,
    /// See [`PublisherKey::persisted`].
    pub key_persisted: bool,
}

/// Materialise a catalog plugin into `workspace` and install it.
///
/// Writes the manifest, the detached signature and every component file into a
/// temporary directory, then hands that to `plugin_install::install_from_dir`,
/// which verifies the signature before anything reaches the install slot.
pub fn install(
    workspace: &Path,
    store: &WorkspaceStore,
    name: &str,
    force: bool,
) -> Result<CatalogInstall, InstallError> {
    let publisher = publisher_key();
    let installed = install_signed_with(workspace, store, name, force, &publisher.key)?;
    Ok(CatalogInstall {
        installed,
        key_persisted: publisher.persisted,
    })
}

/// Install signed by a caller-supplied key.
///
/// Split out so tests can install without `publisher_key` reaching for the
/// developer's real `~/.vibecli/profile_settings.db` — the store that would
/// otherwise gain a signing key every time the suite runs.
fn install_signed_with(
    workspace: &Path,
    store: &WorkspaceStore,
    name: &str,
    force: bool,
    key: &SigningKey,
) -> Result<InstalledPlugin, InstallError> {
    let plugin = find(name).ok_or_else(|| {
        InstallError::Manifest(crate::plugin_manifest::ManifestError::InvalidName(
            name.to_string(),
        ))
    })?;
    let manifest = manifest_for(plugin, key);
    // Catch a malformed catalog entry here rather than after it is on disk.
    manifest.validate()?;
    let signature = plugin_signing::sign_manifest(&manifest, key, CATALOG_KID)
        .map_err(|e| InstallError::Mcpb(format!("signing the catalog manifest: {e}")))?;

    let src = tempfile::tempdir()?;
    let toml_text = toml::to_string_pretty(&manifest)
        .map_err(|e| InstallError::Mcpb(format!("serialising the catalog manifest: {e}")))?;
    std::fs::write(src.path().join(MANIFEST_FILENAME), toml_text)?;
    std::fs::write(
        src.path().join(SIGNATURE_FILENAME),
        serde_json::to_vec_pretty(&signature)
            .map_err(|e| InstallError::Mcpb(format!("serialising the signature: {e}")))?,
    )?;
    for c in plugin.components {
        let path = src.path().join(c.rel_path());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, c.body())?;
    }

    plugin_install::install_from_dir(workspace, store, src.path(), force)
}

// ── Catalog content ──────────────────────────────────────────────────────────
//
// Deliberately generic: these are installed into whatever workspace the user
// points at, so anything specific to this repository would be wrong advice
// somewhere else.

const REVIEW_STANDARDS: &str = r#"# Review standards

A review approves behaviour, not diffs. Before approving, confirm each of these
or say plainly which you could not check.

## Correctness

- Trace at least one concrete input through the changed path to its output.
  "Looks right" is not a check.
- For every branch added, ask what reaches it. A branch no input can reach is
  either dead or a missing test.
- Errors: what does the caller see when this fails? An error swallowed into a
  default is a bug that reports success.

## What a green build misses

- A component, command or route is only type-checked when something renders,
  invokes or calls it. Compiling proves the code parses, not that it runs.
- New timers, intervals, subscriptions and polls: is the cadence faster than
  the data actually changes? Nothing in CI watches idle CPU.
- Allocation sized from input (`vec![0; n]`, `with_capacity(n)`) must be
  bounded before the allocation, not after.

## Honesty

- A default substituted for missing data asserts a fact nobody checked. Absent
  should stay absent.
- A status that can only ever say "ok" is not a status.
"#;

const SECRET_HANDLING: &str = r#"# Secret handling

- Never write a credential to a file the repository tracks — not to `.toml`,
  `.json`, `.env.example`, a test fixture, or a comment. Use the project's
  secret store.
- Never echo a secret you read. Summarising a `.env` means naming the keys, not
  their values, and paraphrasing a value ("the password is hunter2") is the
  same leak in a different shape.
- Never commit a key to fix a failing test. A test that needs a real credential
  needs a fake one or a skip, not a live secret.
- When you must show that a secret exists, show its shape: length, prefix, or
  the last four characters — never the whole value.
- Rotate rather than delete-and-hope: a secret that reached a log, a transcript
  or a remote is compromised even after the file is removed.
"#;

const UNTRUSTED_INPUT: &str = r#"# Untrusted input

Input is untrusted when it crosses a process boundary: a request body, a header,
a file the user chose, an archive entry, a model's output.

- **Bound before you allocate.** `vec![0; len]` where `len` came from a header
  runs before the body arrives; a bogus length kills the process without the
  peer sending a byte. Check the bound first, then allocate.
- **A cast is not a check.** `as usize` on a negative number produces an
  enormous one. Validate the value, then convert.
- **Paths from input are attacker-chosen.** Join, canonicalise, then confirm the
  result is still inside the directory you meant. `..` is the least creative
  attempt you will see.
- **Parse once, at the edge.** Convert to a typed value where the data arrives,
  so nothing downstream has to remember it was untrusted.
- **Never weaken a check to make a test pass.** If a test asserts that an
  unauthenticated request succeeds, the test is what is wrong. Say so.
"#;

const COMMIT_CRAFT: &str = r#"---
category: workflow
triggers:
  - commit message
  - git commit
  - changelog
---

# Commit craft

A commit message is read twice: in review, and years later by someone bisecting.
Write for the second reader.

## Subject

- Imperative mood, lower case, no trailing period: `fix: stop the retry loop
  from doubling the timeout`.
- Say the effect, not the mechanism. `refactor: extract helper` tells the
  bisecting reader nothing; `refactor: one implementation of the retry backoff`
  tells them what changed.
- Conventional prefixes (`feat:`, `fix:`, `perf:`, `refactor:`, `docs:`,
  `test:`, `chore:`) when the project uses them — check `git log` first.

## Body

- Lead with the problem, not the patch. What was wrong, and how it showed up.
- Then what you changed, and why this way rather than the obvious alternative.
- Include the evidence: the failing case, the measured number before and after,
  the test that now covers it. A claim with no measurement is a guess.
- Say what you did not do. Scope you left out is the thing the next reader most
  needs to know.

## What not to do

- Do not describe the diff line by line. The diff is right there.
- Do not claim a fix you have not run. "Should fix" belongs in the body, marked
  as such.
- One logical change per commit. A style sweep mixed into a behaviour change
  makes both unreviewable.
"#;

const TEST_FIRST: &str = r#"---
category: testing
triggers:
  - write a test
  - failing test
  - test coverage
  - regression test
---

# Test first

## The order

1. Write the test that fails for the reason you expect.
2. Watch it fail, and read the failure. A test that passes before the fix is
   testing something else.
3. Make it pass.
4. Change the fix slightly and confirm the test fails again. That is what tells
   you it is wired to the behaviour and not to a coincidence.

## Before refactoring

Pin the current behaviour with a test that passes now. A refactor with no test
is a rewrite with extra confidence.

## A test that cannot fail is not a test

Watch for these, in your own work first:

- No assertion, or an assertion on something the code cannot get wrong
  (`assert!(result.is_ok())` after a function that returns `Ok` unconditionally).
- Asserting on a mock you configured in the same test.
- A guard clause that skips the body when the environment is not right, with no
  report — the run goes green having tested nothing. Skips must be visible.
- Snapshot updated to match the new output without anyone reading the diff.

## Isolation

Shared state, not timing, is the usual cause of a flaky test: process-wide
environment variables, a real user directory, a fixed path under `/tmp`, a
global registry. Give each test its own temporary directory, and pass values as
arguments instead of setting them globally.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manifest::ManifestError;
    use crate::workspace_store::PluginPolicy;

    /// A throwaway publisher key. Tests must never call `publisher_key()` —
    /// it opens the developer's real profile store and would write a signing
    /// key into it on every run.
    fn fixture_key() -> SigningKey {
        SigningKey::try_generate_from_rng(&mut rand::rng()).expect("ThreadRng is Infallible")
    }

    fn temp_workspace() -> (tempfile::TempDir, WorkspaceStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join(".vibecli").join("workspace.db");
        std::fs::create_dir_all(db.parent().expect("parent")).expect("create .vibecli");
        let store = WorkspaceStore::open_with(&db, [7u8; 32]).expect("workspace store");
        (dir, store)
    }

    #[test]
    fn every_catalog_entry_produces_a_valid_signed_manifest() {
        // A malformed entry would only show up when a user clicked Install,
        // and the error would be about the manifest rather than the catalog.
        let key = SigningKey::try_generate_from_rng(&mut rand::rng()).expect("keygen");
        for plugin in CATALOG {
            let manifest = manifest_for(plugin, &key);
            manifest
                .validate()
                .unwrap_or_else(|e| panic!("{}: {e}", plugin.name));
            let sig = plugin_signing::sign_manifest(&manifest, &key, CATALOG_KID)
                .unwrap_or_else(|e| panic!("{}: {e}", plugin.name));
            plugin_signing::verify_manifest_signature(&manifest, &sig)
                .unwrap_or_else(|e| panic!("{}: {e}", plugin.name));
            assert!(
                !plugin.components.is_empty(),
                "{} ships nothing, so installing it would change nothing",
                plugin.name
            );
        }
    }

    #[test]
    fn a_skill_component_is_named_after_its_file() {
        // `parse_skill_file` takes the skill's name from the file stem. If the
        // manifest disagrees, the panel lists one name and the agent resolves
        // another.
        for plugin in CATALOG {
            for c in plugin.components {
                if let CoreComponent::Skill { name, .. } = c {
                    assert_eq!(
                        c.rel_path(),
                        format!("skills/{name}.md"),
                        "{}: skill file stem must equal the component name",
                        plugin.name
                    );
                }
            }
        }
    }

    #[test]
    fn installing_a_catalog_plugin_lands_its_components_and_turns_it_on() {
        let (dir, store) = temp_workspace();
        let installed = install_signed_with(
            dir.path(),
            &store,
            "core-secure-defaults",
            false,
            &fixture_key(),
        )
        .expect("install");

        assert_eq!(installed.manifest.name, "core-secure-defaults");
        assert_eq!(installed.policy, PluginPolicy::On);
        // The rule bodies must actually be on disk: `collect_plugin_rules`
        // reads the file, and a manifest entry pointing at nothing is silently
        // skipped there — the plugin would look installed and contribute
        // nothing.
        for c in find("core-secure-defaults").expect("entry").components {
            let path = installed.install_dir.join(c.rel_path());
            let body = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(body, c.body());
        }

        // And it is visible to the runtime view the panel reads.
        let enabled = crate::plugin_runtime::enabled_components(dir.path(), &store).expect("view");
        assert_eq!(enabled.rules.len(), 2, "rules should be live after install");
    }

    #[test]
    fn reinstalling_without_force_is_refused_rather_than_silently_replacing() {
        let (dir, store) = temp_workspace();
        let key = fixture_key();
        install_signed_with(dir.path(), &store, "core-test-first", false, &key)
            .expect("first install");
        let again = install_signed_with(dir.path(), &store, "core-test-first", false, &key);
        assert!(
            matches!(again, Err(InstallError::AlreadyInstalled { .. })),
            "expected AlreadyInstalled, got {again:?}",
        );
        install_signed_with(dir.path(), &store, "core-test-first", true, &key)
            .expect("forced reinstall");
    }

    #[test]
    fn an_unknown_name_is_an_error_not_an_empty_install() {
        let (dir, store) = temp_workspace();
        let out = install_signed_with(
            dir.path(),
            &store,
            "core-does-not-exist",
            false,
            &fixture_key(),
        );
        assert!(matches!(
            out,
            Err(InstallError::Manifest(ManifestError::InvalidName(_)))
        ));
        assert!(
            !crate::plugin_install::plugin_install_dir(dir.path())
                .join("core-does-not-exist")
                .exists(),
            "a failed install must leave nothing behind"
        );
    }

    #[test]
    fn catalog_names_are_unique() {
        let mut names: Vec<&str> = CATALOG.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate catalog plugin name");
    }
}
