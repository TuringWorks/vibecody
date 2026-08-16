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
///
/// Two shapes share this type. A **plugin** carries components — the skills and
/// rules that change how the agent works. A **bundle** carries `includes` and
/// `connectors` instead: it is a job description, and installing it sets up the
/// whole kit. The distinction the user cares about is what each answers — a
/// connector answers "what can the agent reach", a bundle answers "what job is
/// it set up to do".
#[derive(Debug, Clone, Copy)]
pub struct CorePlugin {
    /// Manifest name and install-directory name. Kebab-case.
    pub name: &'static str,
    /// Human title for the catalog card.
    pub title: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    /// Marketplace section. Free text, like the connector catalog's — the
    /// section list is presentation, not a type.
    pub category: &'static str,
    pub components: &'static [CoreComponent],
    /// Other catalog plugins installed alongside this one.
    ///
    /// A bundle references the single-purpose plugins rather than copying their
    /// content: two plugins shipping a skill of the same name would collide in
    /// the catalog, and a second copy of a rule is a second thing to keep true.
    pub includes: &'static [&'static str],
    /// Connector ids from `connectors::CATALOG` this setup expects.
    ///
    /// Installing the plugin adds the ones that need no credential and *asks*
    /// for the ones that do. It never reports a connector as configured that
    /// the user has not given a token for — "already configured" has to mean it.
    pub connectors: &'static [&'static str],
}

/// The catalog itself.
///
/// Each entry has to earn its place by changing what the agent does; a catalog
/// padded with plausible-sounding plugins is a catalog nobody reads.
pub const CATALOG: &[CorePlugin] = &[
    // ── Bundles ──────────────────────────────────────────────────────────
    //
    // A job, not a capability. Each installs the single-purpose plugins it
    // needs and sets up the connectors that job assumes, so the answer to
    // "what is this agent set up to do" is one click rather than six.
    CorePlugin {
        name: "bundle-engineering",
        title: "Engineering",
        version: "1.0.0",
        description: "The everyday coding setup: review standards, test-first \
                      discipline and a debugging method, plus the repository and \
                      filesystem connectors that work assumes.",
        category: "Bundles",
        components: &[],
        includes: &["core-review-standards", "core-test-first", "core-debugging"],
        connectors: &["filesystem", "git", "github"],
    },
    CorePlugin {
        name: "bundle-on-call",
        title: "On-call",
        version: "1.0.0",
        description: "For the hour something is broken: incident response and \
                      debugging, wired to the error tracker, the repository and the \
                      channel where the incident is being run.",
        category: "Bundles",
        components: &[],
        includes: &["core-incident-response", "core-debugging"],
        connectors: &["sentry", "github", "slack"],
    },
    CorePlugin {
        name: "bundle-security-review",
        title: "Security review",
        version: "1.0.0",
        description: "Secret handling and input-bounding rules, dependency hygiene, \
                      and the repository access a review needs.",
        category: "Bundles",
        components: &[],
        includes: &["core-secure-defaults", "core-dependency-hygiene"],
        connectors: &["github", "git"],
    },
    CorePlugin {
        name: "bundle-data",
        title: "Data work",
        version: "1.0.0",
        description: "Schema-first database work: the review standards that apply to \
                      queries as much as code, plus read access to Postgres and SQLite.",
        category: "Bundles",
        components: &[],
        includes: &["core-review-standards"],
        connectors: &["postgres", "sqlite"],
    },
    CorePlugin {
        name: "bundle-research",
        title: "Research",
        version: "1.0.0",
        description: "Reading the web and writing down what it found: technical \
                      writing, page fetching, search, and a memory that survives the \
                      session.",
        category: "Bundles",
        components: &[],
        includes: &["core-technical-writing"],
        connectors: &["fetch", "brave-search", "memory"],
    },
    CorePlugin {
        name: "core-review-standards",
        title: "Review standards",
        version: "1.0.0",
        description: "What a code review must check before it approves — \
                      correctness first, then the failure modes a green build misses.",
        category: "Engineering Practice",
        components: &[CoreComponent::Rule {
            name: "review-standards",
            body: REVIEW_STANDARDS,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-test-first",
        title: "Test first",
        version: "1.0.0",
        description: "Pin behaviour with a failing test before changing it, and tell \
                      a real test from one that cannot fail.",
        category: "Engineering Practice",
        components: &[CoreComponent::Skill {
            name: "test-first",
            category: "testing",
            body: TEST_FIRST,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-commit-craft",
        title: "Commit craft",
        version: "1.0.0",
        description: "Commit messages that say why, in the shape reviewers and \
                      `git log` expect.",
        category: "Engineering Practice",
        components: &[CoreComponent::Skill {
            name: "commit-craft",
            category: "workflow",
            body: COMMIT_CRAFT,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-refactoring",
        title: "Safe refactoring",
        version: "1.0.0",
        description: "Behaviour-preserving change: what to pin first, which smells \
                      justify the move, and when to stop.",
        category: "Engineering Practice",
        components: &[CoreComponent::Skill {
            name: "safe-refactoring",
            category: "workflow",
            body: SAFE_REFACTORING,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-secure-defaults",
        title: "Secure defaults",
        version: "1.0.0",
        description: "Secrets stay out of the repo and the transcript; input is \
                      bounded before it is allocated; auth checks are never weakened \
                      to make a test pass.",
        category: "Security",
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
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-dependency-hygiene",
        title: "Dependency hygiene",
        version: "1.0.0",
        description: "Adding, upgrading and removing a dependency without importing \
                      someone else's incident.",
        category: "Security",
        components: &[CoreComponent::Skill {
            name: "dependency-hygiene",
            category: "security",
            body: DEPENDENCY_HYGIENE,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-performance",
        title: "Performance work",
        version: "1.0.0",
        description: "Measure, attribute to a call tree, fix, re-measure like for \
                      like — and the order in which wins usually appear.",
        category: "Performance",
        components: &[CoreComponent::Skill {
            name: "performance-work",
            category: "performance",
            body: PERFORMANCE_WORK,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-debugging",
        title: "Debugging",
        version: "1.0.0",
        description: "Reproduce, bisect the search space, and confirm the cause by \
                      turning the bug off and on again.",
        category: "Operations",
        components: &[CoreComponent::Skill {
            name: "debugging",
            category: "debugging",
            body: DEBUGGING,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-incident-response",
        title: "Incident response",
        version: "1.0.0",
        description: "Stop the bleeding first, keep a timeline, and write the \
                      follow-up that stops the repeat.",
        category: "Operations",
        components: &[CoreComponent::Skill {
            name: "incident-response",
            category: "operations",
            body: INCIDENT_RESPONSE,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-api-design",
        title: "API design",
        version: "1.0.0",
        description: "Make the wrong call impossible to write: names, errors, \
                      defaults, and what a version boundary owes its callers.",
        category: "Design",
        components: &[CoreComponent::Skill {
            name: "api-design",
            category: "design",
            body: API_DESIGN,
        }],
        includes: &[],
        connectors: &[],
    },
    CorePlugin {
        name: "core-technical-writing",
        title: "Technical writing",
        version: "1.0.0",
        description: "Documentation that stays true: what to write down, what the \
                      code already says, and how a doc goes stale.",
        category: "Design",
        components: &[CoreComponent::Skill {
            name: "technical-writing",
            category: "documentation",
            body: TECHNICAL_WRITING,
        }],
        includes: &[],
        connectors: &[],
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

/// What happened to one connector a plugin expects.
///
/// Four outcomes, kept apart because they need different things from the user.
/// The one that matters is `NeedsCredentials`: a bundle can wire up everything
/// that asks for nothing, and it cannot invent a token. Reporting that as
/// "configured" would be the whole feature lying at the moment it is most
/// believed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ConnectorSetup {
    /// The workspace already had it; left exactly as it was.
    AlreadyConfigured,
    /// Added by this install. It needed no credential.
    Added,
    /// Not added: it cannot run without a secret only the user has.
    NeedsCredentials { fields: Vec<String> },
    /// Named by the bundle but absent from the connector catalog — a bad
    /// bundle. Reported rather than skipped, or the setup silently ships
    /// short of what it promised.
    Unknown,
    /// Adding it failed. Carries the reason.
    Failed { error: String },
}

/// One connector a plugin expects, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ConnectorOutcome {
    pub id: String,
    pub title: String,
    #[serde(flatten)]
    pub setup: ConnectorSetup,
}

/// What an install actually did, for the caller to report.
#[derive(Debug)]
pub struct CatalogInstall {
    pub installed: InstalledPlugin,
    /// See [`PublisherKey::persisted`].
    pub key_persisted: bool,
    /// Plugins installed alongside this one because it includes them. Empty for
    /// a plain plugin; the members for a bundle.
    pub included: Vec<String>,
    /// Every connector the plugin expects, with what happened to it.
    pub connectors: Vec<ConnectorOutcome>,
}

/// Materialise a catalog plugin into `workspace` and install it.
///
/// Writes the manifest, the detached signature and every component file into a
/// temporary directory, then hands that to `plugin_install::install_from_dir`,
/// which verifies the signature before anything reaches the install slot.
///
/// For a bundle this also installs the plugins it includes and sets up the
/// connectors it expects. `now_ms` is passed in rather than read from the clock
/// here so the caller owns the timestamp it records.
pub fn install(
    workspace: &Path,
    store: &WorkspaceStore,
    name: &str,
    force: bool,
    now_ms: i64,
) -> Result<CatalogInstall, InstallError> {
    let publisher = publisher_key();
    let installed = install_signed_with(workspace, store, name, force, &publisher.key)?;
    let plugin = find(name).ok_or_else(|| {
        InstallError::Manifest(crate::plugin_manifest::ManifestError::InvalidName(
            name.to_string(),
        ))
    })?;

    // Members first: a bundle that fails halfway should still have put its
    // parts on disk, and each is independently removable afterwards.
    let mut included = Vec::new();
    for member in plugin.includes {
        match install_signed_with(workspace, store, member, false, &publisher.key) {
            Ok(_) => included.push((*member).to_string()),
            // Already there is the common case and not a failure — a bundle
            // installed over an existing plugin adopts it rather than
            // duplicating or refusing.
            Err(InstallError::AlreadyInstalled { name, .. }) => included.push(name),
            Err(e) => {
                tracing::warn!(member, error = %e, "bundle member failed to install");
            }
        }
    }

    let connectors = plugin
        .connectors
        .iter()
        .map(|id| setup_connector(store, id, now_ms))
        .collect();

    Ok(CatalogInstall {
        installed,
        key_persisted: publisher.persisted,
        included,
        connectors,
    })
}

/// Add one expected connector, or say precisely why it was not added.
fn setup_connector(store: &WorkspaceStore, id: &str, now_ms: i64) -> ConnectorOutcome {
    let Some(spec) = crate::connectors::spec(id) else {
        return ConnectorOutcome {
            id: id.to_string(),
            title: id.to_string(),
            setup: ConnectorSetup::Unknown,
        };
    };
    let out = |setup| ConnectorOutcome {
        id: spec.id.to_string(),
        title: spec.title.to_string(),
        setup,
    };

    let already = crate::connectors::list(store)
        .unwrap_or_default()
        .into_iter()
        .any(|c| c.id == spec.id);
    if already {
        return out(ConnectorSetup::AlreadyConfigured);
    }
    if !spec.credentials.is_empty() {
        return out(ConnectorSetup::NeedsCredentials {
            fields: spec.credentials.iter().map(|f| f.env.to_string()).collect(),
        });
    }
    match crate::connectors::add_from_catalog(
        store,
        spec.id,
        &std::collections::HashMap::new(),
        now_ms,
    ) {
        Ok(_) => out(ConnectorSetup::Added),
        Err(error) => out(ConnectorSetup::Failed { error }),
    }
}

/// Install signed by a caller-supplied key.
///
/// Split out so tests can install without `publisher_key` reaching for the
/// developer's real `~/.vibecli/profile_settings.db` — the store that would
/// otherwise gain a signing key every time the suite runs.
///
/// Public for integration tests in `tests/`, which live in a separate crate
/// and so cannot reach a private item. Product code wants [`install`], which
/// supplies the machine key and also handles a bundle's includes and
/// connectors; this installs exactly the one plugin named.
pub fn install_signed_with(
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

const SAFE_REFACTORING: &str = r#"---
category: workflow
triggers:
  - refactor
  - clean up
  - extract
  - rename
---

# Safe refactoring

A refactor changes structure and nothing else. The moment behaviour moves, it is
not a refactor any more — it is a change wearing a refactor's name, and it will
be reviewed as if it were safe.

## Before

1. Find the test that covers the behaviour. If there is none, write one that
   passes now. A refactor with no test is a rewrite with extra confidence.
2. Read the callers, not just the function. Most refactors that break something
   break it at a call site nobody opened.
3. Commit the pin separately, so the refactor's diff is only the refactor.

## What justifies one

Refactor toward a shape when the smell is there, never because the shape is
admired:

- A `match`/`switch` on a type tag that grows every time a case is added → make
  the type carry the behaviour.
- A boolean parameter that selects behaviour → two functions, or an enum.
- The same fix applied in three places → one implementation, three callers.
- A function whose name needs "and" → two functions.

## While

- One kind of change per commit. A rename mixed with an extraction is
  unreviewable; both look like noise and the real change hides in it.
- Keep the old name working through a deprecation if anything outside the repo
  calls it.
- Run the pinning test after every step, not at the end. A refactor that broke
  four steps ago is four times harder to find.

## When to stop

When the next move is speculative. Structure that anticipates a requirement
nobody has asked for is the most expensive kind to undo, because it looks
deliberate.
"#;

const DEPENDENCY_HYGIENE: &str = r#"---
category: security
triggers:
  - add a dependency
  - upgrade
  - npm install
  - cargo add
  - vulnerability
---

# Dependency hygiene

Every dependency is code you did not write running with your permissions.

## Adding one

- Ask what it replaces. Twenty lines of your own beats a package for anything
  you could write in an afternoon and will have to audit forever.
- Check when it was last released and how many people maintain it. One
  maintainer and two years of silence is a supply-chain risk, not a feature.
- Check what it pulls in. The transitive tree is the real dependency.
- Prefer the standard library, then the framework you already have, then a new
  package.

## Upgrading

- Read the changelog for the versions you skipped, not just the one you land on.
- A major bump is a porting job. Pinning the new version without changing the
  call sites produces a build that fails everywhere at once and a diff that
  claims to be routine.
- Upgrade one thing at a time when something breaks. A batch bump with a failure
  in it is a bisect you have to do by hand.

## Removing

- Delete the code first, then the dependency, then the lockfile entry. Removing
  the package first turns a clean deletion into a compile error hunt.
- An unused dependency is still an attack surface — it ships.

## Vulnerabilities

- Fix by upgrading, not by suppressing. A suppression with no expiry is a
  permanent decision made in a hurry.
- Check whether you actually reach the vulnerable path before treating severity
  as urgency, and say which it is when you report it.
"#;

const PERFORMANCE_WORK: &str = r#"---
category: performance
triggers:
  - slow
  - performance
  - optimize
  - profile
  - latency
---

# Performance work

## The loop

**Measure → attribute → fix → re-measure like for like → confirm the feature
still works.** Skipping the first step produces a fix for a problem nobody has;
skipping the last produces a fast, broken feature.

- **Measure** on the machine and the input that showed the problem. A synthetic
  benchmark measures the benchmark.
- **Attribute to a call tree, not a leaf list.** A flat profile says
  `memcpy` is hot. The call tree says which of your loops is calling it.
- **Re-measure like for like.** Same input, same machine, same build profile.
  A debug-vs-release comparison is not a result.

## Where the wins usually are, in order

1. **Cadence** — is this poll faster than the data it polls actually changes?
   The cheapest optimisation is not doing the work.
2. **Eager instantiation** — is this built at startup and used by one screen in
   twenty? Load it when it is needed.
3. **Recycling** — is a list rebuilding every row on every update?
4. **Dirty checks** — is work repeating on unchanged input?
5. **Algorithms** — now the O() actually matters.
6. **Allocation** — last, and only with an allocation count to prove it.

## Traps

- **A good number proves less than a bad one.** Confirm the feature still works
  after the fix, on screen, not just in the benchmark.
- **Idle cost is invisible to CI.** After touching any timer, interval or
  subscription, check idle CPU by hand.
- **Allocation sized from input** is a crash, not a slowdown. Bound it before
  allocating.
- **A regex compiled inside a loop** is the most expensive line in most hot
  paths. Hoist it.
"#;

const DEBUGGING: &str = r#"---
category: debugging
triggers:
  - bug
  - not working
  - crash
  - fails
  - investigate
---

# Debugging

## Reproduce first

A bug you cannot reproduce is a bug you cannot confirm you fixed. Get to a
command that fails every time before changing anything. Shrink it: fewest steps,
smallest input, least environment.

If it only reproduces sometimes, that is information — concurrency, ordering,
leftover state, clock, network. Say which, or say you do not know yet.

## Bisect the search space, not the code

Each step should halve what is left:

- Does it happen with the feature off? Before that commit? On another machine?
  With an empty database? With the network unplugged?
- `git bisect` works on any yes/no test and is faster than reading.
- Add a check in the middle of the pipeline and ask which side is wrong.

## Confirm the cause

You have not found it until you can turn it off and on. Make the bug appear on
demand by re-introducing the cause; if you cannot, you have found a correlation.

Then ask why it was not caught: no test, a test that skipped silently, a
swallowed error, a default that made the failure look like success.

## Say what you know

Distinguish "I reproduced it and the cause is X" from "the symptom is consistent
with X". The second is a hypothesis, and labelling it as one costs nothing and
saves the next person an afternoon.
"#;

const INCIDENT_RESPONSE: &str = r#"---
category: operations
triggers:
  - incident
  - outage
  - production is down
  - rollback
  - postmortem
---

# Incident response

## Order of operations

1. **Stop the bleeding.** Roll back, disable the flag, drain the node. The fix
   comes after service returns; understanding comes after that.
2. **Say what is happening.** One message, in the channel people are already
   watching: what is broken, who is affected, what is being done, when the next
   update comes. Then keep that cadence even when there is nothing new.
3. **Keep a timeline as you go.** Timestamps, what was observed, what was
   changed. Reconstructing it afterwards produces a tidy story that is wrong.

## While you work

- Change one thing at a time and write down what you changed. Three
  simultaneous mitigations mean nobody knows which one worked.
- Prefer reversible mitigations. A rollback you can undo beats a forward fix you
  cannot.
- If you are the only one who knows something, say it out loud. Silent
  competence extends outages.

## Afterwards

- Write the follow-up while it is fresh, and write it blameless: the question is
  what made the mistake easy, not who made it.
- Every action item gets an owner and a date, or it is not an action item.
- The most valuable line is usually "why we did not notice sooner". Fix the
  detection gap, not only the cause.
"#;

const API_DESIGN: &str = r#"---
category: design
triggers:
  - api design
  - interface
  - public API
  - breaking change
---

# API design

The goal is that the wrong call is hard to write and the right one is obvious.

## Shape

- **Make illegal states unrepresentable.** A type that cannot hold a bad
  combination beats a runtime check and a comment.
- **Name for the caller's world, not the implementation's.** `retry_after` not
  `backoff_ms_internal`.
- **Booleans lie about the future.** `create(force: true)` becomes
  `create(force: true, overwrite: false)` within a year. Take an enum.
- **Return what the caller needs to act.** An `Ok(())` that hides which of three
  things happened forces every caller to go looking.

## Errors

- Distinguish "you asked for something impossible" from "I could not do it right
  now" — the caller retries one and not the other.
- Carry the context the caller cannot reconstruct: which field, which id, which
  limit.
- Never make an error look like success. A default returned on failure is a bug
  the caller cannot see.

## Defaults

- A default is a decision made for everyone who did not think about it. Pick the
  safe one, not the convenient one.
- No default at all is better than a plausible one for anything the caller must
  know — a timeout, a scope, a destination.

## Versions

- Adding an optional field is compatible. Adding a required one is not.
- Removing anything needs a deprecation window and a message that names the
  replacement.
- Changing what a value means is the breaking change nobody catches, because
  everything still compiles.
"#;

const TECHNICAL_WRITING: &str = r#"---
category: documentation
triggers:
  - write documentation
  - README
  - docs
  - changelog
---

# Technical writing

## Write what the code cannot say

The code already says what it does. Documentation earns its keep by recording
what is not in it:

- Why this approach and not the obvious one.
- What was tried and did not work.
- The constraint that explains the odd bit.
- What breaks if you change this.

A comment restating the line above it is a maintenance cost with no return.

## Structure

- Lead with what the reader needs first, not with history. The person reading a
  README wants to run the thing.
- One page per question. A document that answers five questions is found by
  nobody looking for any of them.
- Show the command and its real output. Invented output is a promise the
  software has not made.

## Staying true

A doc goes stale the moment the thing it describes moves. Reduce the surface:

- Link to the source of truth rather than restating it.
- Generate what can be generated — flag lists, route tables, config keys.
- When you change behaviour, grep the docs for the old behaviour in the same
  commit. A changelog entry is not a substitute for fixing the page that now
  lies.
- Date anything that is a snapshot ("as of…"), so a reader can tell staleness
  from disagreement.

## Tone

Say what is true, plainly. Hedging every sentence makes the uncertain and the
certain look alike, and the reader has to guess which is which.
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

    /// `install` with a throwaway publisher key.
    ///
    /// Mirrors the bundle half of [`install`] so the member and connector
    /// behaviour is covered without `publisher_key()` writing into the
    /// developer's real profile store.
    fn install_bundle_with(
        workspace: &std::path::Path,
        store: &WorkspaceStore,
        name: &str,
        key: &SigningKey,
    ) -> CatalogInstall {
        let installed =
            install_signed_with(workspace, store, name, true, key).expect("bundle install");
        let plugin = find(name).expect("catalog entry");
        let mut included = Vec::new();
        for member in plugin.includes {
            match install_signed_with(workspace, store, member, false, key) {
                Ok(_) => included.push((*member).to_string()),
                Err(InstallError::AlreadyInstalled { name, .. }) => included.push(name),
                Err(e) => panic!("{member}: {e}"),
            }
        }
        let connectors = plugin
            .connectors
            .iter()
            .map(|id| setup_connector(store, id, 1_700_000_000_000))
            .collect();
        CatalogInstall {
            installed,
            key_persisted: false,
            included,
            connectors,
        }
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

    /// A bundle is a job, and installing it has to actually set the job up.
    #[test]
    fn a_bundle_installs_its_members_and_the_connectors_that_need_nothing() {
        let (dir, store) = temp_workspace();
        let out = install_bundle_with(dir.path(), &store, "bundle-research", &fixture_key());

        // Its member plugin is on disk and live…
        assert!(out.included.contains(&"core-technical-writing".to_string()));
        let enabled = crate::plugin_runtime::enabled_components(dir.path(), &store).expect("view");
        assert!(
            enabled
                .skills
                .iter()
                .any(|s| s.spec.name == "technical-writing"),
            "the bundle's skill did not become live"
        );

        // …and the connectors that ask for nothing are configured, while the
        // one that needs a key is reported rather than pretended.
        let by_id = |id: &str| {
            out.connectors
                .iter()
                .find(|c| c.id == id)
                .unwrap_or_else(|| panic!("{id} missing from the outcome"))
                .setup
                .clone()
        };
        assert_eq!(by_id("fetch"), ConnectorSetup::Added);
        assert_eq!(by_id("memory"), ConnectorSetup::Added);
        assert!(
            matches!(
                by_id("brave-search"),
                ConnectorSetup::NeedsCredentials { .. }
            ),
            "a connector needing an API key must not be reported as set up"
        );

        let configured = crate::connectors::list(&store).expect("connectors");
        assert!(configured.iter().any(|c| c.id == "fetch"));
        assert!(
            !configured.iter().any(|c| c.id == "brave-search"),
            "a connector was added without the credential it requires"
        );
    }

    #[test]
    fn a_bundle_adopts_a_connector_that_is_already_there() {
        let (dir, store) = temp_workspace();
        crate::connectors::add_from_catalog(
            &store,
            "fetch",
            &std::collections::HashMap::new(),
            1_700_000_000_000,
        )
        .expect("pre-existing connector");

        let out = install_bundle_with(dir.path(), &store, "bundle-research", &fixture_key());
        let fetch = out
            .connectors
            .iter()
            .find(|c| c.id == "fetch")
            .expect("fetch");
        assert_eq!(fetch.setup, ConnectorSetup::AlreadyConfigured);
        // Exactly one, not a duplicate.
        assert_eq!(
            crate::connectors::list(&store)
                .expect("list")
                .iter()
                .filter(|c| c.id == "fetch")
                .count(),
            1
        );
    }

    #[test]
    fn installing_a_bundle_twice_adopts_its_members_rather_than_failing() {
        // The second install of a member returns AlreadyInstalled; a bundle
        // that treated that as an error would report a broken setup for the
        // most ordinary case there is — installing two bundles that share a
        // plugin.
        let (dir, store) = temp_workspace();
        let key = fixture_key();
        install_bundle_with(dir.path(), &store, "bundle-engineering", &key);
        let out = install_bundle_with(dir.path(), &store, "bundle-on-call", &key);
        assert!(
            out.included.contains(&"core-debugging".to_string()),
            "a shared member must still be reported as part of the setup: {:?}",
            out.included
        );
    }

    #[test]
    fn every_bundle_names_real_members_and_real_connectors() {
        // A typo here ships a bundle that silently sets up less than it says.
        for p in CATALOG {
            for member in p.includes {
                assert!(
                    find(member).is_some(),
                    "{}: unknown member `{member}`",
                    p.name
                );
                assert_ne!(*member, p.name, "{} includes itself", p.name);
            }
            for id in p.connectors {
                assert!(
                    crate::connectors::spec(id).is_some(),
                    "{}: unknown connector `{id}`",
                    p.name
                );
            }
            assert!(
                !p.components.is_empty() || !p.includes.is_empty() || !p.connectors.is_empty(),
                "{} installs nothing at all",
                p.name
            );
        }
    }

    #[test]
    fn every_catalog_entry_is_filed_under_a_category() {
        // The marketplace groups by this; an entry with no category lands in a
        // section named after nothing.
        for p in CATALOG {
            assert!(!p.category.trim().is_empty(), "{} has no category", p.name);
        }
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
