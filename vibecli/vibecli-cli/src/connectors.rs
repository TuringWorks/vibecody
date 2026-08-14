//! Workspace connectors: MCP servers the workspace can talk to, with their
//! credentials in the encrypted store.
//!
//! A connector is an MCP server definition plus the secrets it needs to run.
//! Definitions live in the workspace store (`connectors` setting); credentials
//! live in `workspace_secrets`, encrypted, and never in the definition — so a
//! connector list can be read, exported or logged without leaking a token.
//!
//! ## What a connector is worth today
//!
//! [`resolve_mcp_configs`] turns the enabled connectors into
//! `vibe_ai::mcp::McpServerConfig` values, which is what the REPL's `/mcp`
//! command speaks. That is the whole of it: the agent loop does not consume MCP
//! tools yet, so a connector makes tools reachable from `vibecli`, not from an
//! agent run. The panel says so rather than implying more.
//!
//! ## Status is measured, never inferred
//!
//! There is no "connected because a key is present" state here. A connector is
//! `Untested` until [`probe`] actually launches the server and lists its tools;
//! after that it is `Ok` with the tool count it saw, or `Failed` with the error
//! it got. The result is not persisted — a stored "ok" is a claim about the
//! past presented as a claim about now.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::workspace_store::WorkspaceStore;
use vibe_ai::mcp::McpServerConfig;

/// Workspace-store setting key holding the connector definitions.
const SETTING_KEY: &str = "connectors";

/// Placeholder substituted with the workspace root when a connector is
/// launched. Stored verbatim so a workspace that moves keeps working.
pub const WORKSPACE_PLACEHOLDER: &str = "{workspace}";

/// A credential a connector needs, described so the panel can prompt for it.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CredentialField {
    /// Environment variable the server reads it from.
    pub env: &'static str,
    /// Short label for the input.
    pub label: &'static str,
    /// Where the user gets one.
    pub help: &'static str,
}

/// What has to be installed for a connector's command to exist.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    /// Ships with VibeCody — the `vibecli` binary itself.
    Builtin,
    /// Needs Node's `npx` on PATH.
    Npx,
    /// Needs `uvx` (from `uv`) on PATH.
    Uvx,
}

impl Runtime {
    /// The program that has to be on PATH for this runtime.
    pub fn program(&self) -> &'static str {
        match self {
            Self::Builtin => "vibecli",
            Self::Npx => "npx",
            Self::Uvx => "uvx",
        }
    }
}

/// A connector the user can add in one click.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ConnectorSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub runtime: Runtime,
    pub command: &'static str,
    pub args: &'static [&'static str],
    pub credentials: &'static [CredentialField],
    pub docs_url: &'static str,
}

/// The connector catalog.
///
/// Every entry launches a real, published MCP server. None of them is verified
/// to work on this machine by being listed here — the package may not exist for
/// the installed runtime, the version may have moved, the network may be down.
/// That is what [`probe`] is for, and why nothing here reports a connector as
/// working until it has been run.
pub const CATALOG: &[ConnectorSpec] = &[
    ConnectorSpec {
        id: "vibecli",
        title: "VibeCLI tools",
        description: "This machine's own VibeCLI, exposed over MCP: file read/write, \
                      directory listing, shell, search and agent runs. No install and \
                      no credentials — it is the binary already running.",
        runtime: Runtime::Builtin,
        command: "vibecli",
        args: &["--mcp-server"],
        credentials: &[],
        docs_url: "https://github.com/TuringWorks/vibecody",
    },
    ConnectorSpec {
        id: "filesystem",
        title: "Filesystem",
        description: "Read and write files under this workspace through the reference \
                      filesystem MCP server. Scoped to the workspace root.",
        runtime: Runtime::Npx,
        command: "npx",
        args: &[
            "-y",
            "@modelcontextprotocol/server-filesystem",
            WORKSPACE_PLACEHOLDER,
        ],
        credentials: &[],
        docs_url: "https://github.com/modelcontextprotocol/servers",
    },
    ConnectorSpec {
        id: "git",
        title: "Git",
        description: "Inspect this repository's history, diffs and branches through the \
                      reference git MCP server.",
        runtime: Runtime::Uvx,
        command: "uvx",
        args: &["mcp-server-git", "--repository", WORKSPACE_PLACEHOLDER],
        credentials: &[],
        docs_url: "https://github.com/modelcontextprotocol/servers",
    },
    ConnectorSpec {
        id: "fetch",
        title: "Fetch",
        description: "Fetch a URL and convert it to Markdown. Reaches the network — \
                      anything it retrieves is untrusted input.",
        runtime: Runtime::Uvx,
        command: "uvx",
        args: &["mcp-server-fetch"],
        credentials: &[],
        docs_url: "https://github.com/modelcontextprotocol/servers",
    },
    ConnectorSpec {
        id: "memory",
        title: "Memory",
        description: "A knowledge graph the model can write to and read back across \
                      sessions, held in the reference memory MCP server.",
        runtime: Runtime::Npx,
        command: "npx",
        args: &["-y", "@modelcontextprotocol/server-memory"],
        credentials: &[],
        docs_url: "https://github.com/modelcontextprotocol/servers",
    },
    ConnectorSpec {
        id: "github",
        title: "GitHub",
        description: "Issues, pull requests, code search and file contents on GitHub. \
                      Needs a personal access token; the token is stored encrypted in \
                      this workspace, never in a file.",
        runtime: Runtime::Npx,
        command: "npx",
        args: &["-y", "@modelcontextprotocol/server-github"],
        credentials: &[CredentialField {
            env: "GITHUB_PERSONAL_ACCESS_TOKEN",
            label: "Personal access token",
            help: "github.com → Settings → Developer settings → Personal access tokens. \
                   Grant only the scopes you want the agent to have.",
        }],
        docs_url: "https://github.com/modelcontextprotocol/servers",
    },
];

pub fn spec(id: &str) -> Option<&'static ConnectorSpec> {
    CATALOG.iter().find(|c| c.id == id)
}

/// A connector as stored in the workspace.
///
/// `command`/`args` are a snapshot taken when the connector was added rather
/// than a lookup into [`CATALOG`] at launch time. A stored command is what the
/// user agreed to run; silently swapping it because the binary was upgraded
/// would change what executes without anyone approving it. `catalog_id` is kept
/// for display, and to let the panel offer a re-add when an entry moves on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Connector {
    /// Unique within the workspace. Also the MCP server name.
    pub id: String,
    /// Catalog entry this came from, or `None` for a hand-entered command.
    #[serde(default)]
    pub catalog_id: Option<String>,
    pub title: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables whose values live in the encrypted secret store,
    /// under `secret_key(connector_id, env)`. Names only — never values.
    #[serde(default)]
    pub env_keys: Vec<String>,
    pub enabled: bool,
    /// Unix millis. Recorded when added; absent for rows written before this
    /// field existed rather than back-filled with "now", which would assert a
    /// date nobody observed.
    #[serde(default)]
    pub added_at: Option<i64>,
}

/// Workspace-secret key holding one connector credential.
pub fn secret_key(connector_id: &str, env: &str) -> String {
    format!("connector:{connector_id}:{env}")
}

/// Read the stored connector list. A missing or unparseable setting yields an
/// empty list — the panel then offers the catalog, which is the useful state.
pub fn list(store: &WorkspaceStore) -> Result<Vec<Connector>, String> {
    let Some(raw) = store.setting_get(SETTING_KEY)? else {
        return Ok(Vec::new());
    };
    match serde_json::from_str::<Vec<Connector>>(&raw) {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::warn!(error = %e, "connectors: stored list is unreadable; treating as empty");
            Ok(Vec::new())
        }
    }
}

fn save(store: &WorkspaceStore, connectors: &[Connector]) -> Result<(), String> {
    let json = serde_json::to_string(connectors).map_err(|e| e.to_string())?;
    store.setting_set(SETTING_KEY, &json)
}

/// Add a connector from the catalog, storing its credentials encrypted.
///
/// `credentials` is keyed by environment variable name. A required credential
/// left empty is rejected rather than stored blank: a server launched without
/// its token fails in a way that looks like a broken package.
pub fn add_from_catalog(
    store: &WorkspaceStore,
    catalog_id: &str,
    credentials: &HashMap<String, String>,
    now_ms: i64,
) -> Result<Connector, String> {
    let spec = spec(catalog_id).ok_or_else(|| format!("unknown connector `{catalog_id}`"))?;
    let mut existing = list(store)?;
    if existing.iter().any(|c| c.id == spec.id) {
        return Err(format!(
            "connector `{}` is already configured in this workspace",
            spec.id
        ));
    }

    for field in spec.credentials {
        let value = credentials
            .get(field.env)
            .map(|s| s.trim())
            .unwrap_or_default();
        if value.is_empty() {
            return Err(format!("`{}` is required for {}", field.label, spec.title));
        }
        store.secret_set(&secret_key(spec.id, field.env), value, Some("connector"))?;
    }

    let connector = Connector {
        id: spec.id.to_string(),
        catalog_id: Some(spec.id.to_string()),
        title: spec.title.to_string(),
        command: spec.command.to_string(),
        args: spec.args.iter().map(|s| (*s).to_string()).collect(),
        env_keys: spec.credentials.iter().map(|c| c.env.to_string()).collect(),
        enabled: true,
        added_at: Some(now_ms),
    };
    existing.push(connector.clone());
    save(store, &existing)?;
    Ok(connector)
}

/// Add a hand-entered MCP server command.
///
/// The escape hatch for anything not in the catalog. `id` is used verbatim as
/// the MCP server name, so it is validated the same way a plugin name is:
/// kebab-case, no path separators, nothing that could escape a key namespace.
pub fn add_custom(
    store: &WorkspaceStore,
    id: &str,
    title: &str,
    command: &str,
    args: Vec<String>,
    credentials: &HashMap<String, String>,
    now_ms: i64,
) -> Result<Connector, String> {
    validate_id(id)?;
    if command.trim().is_empty() {
        return Err("a command is required".to_string());
    }
    let mut existing = list(store)?;
    if existing.iter().any(|c| c.id == id) {
        return Err(format!("connector `{id}` already exists"));
    }

    let mut env_keys: Vec<String> = Vec::new();
    for (env, value) in credentials {
        if value.trim().is_empty() {
            continue;
        }
        validate_env_name(env)?;
        store.secret_set(&secret_key(id, env), value.trim(), Some("connector"))?;
        env_keys.push(env.clone());
    }
    env_keys.sort();

    let connector = Connector {
        id: id.to_string(),
        catalog_id: None,
        title: if title.trim().is_empty() {
            id.to_string()
        } else {
            title.trim().to_string()
        },
        command: command.trim().to_string(),
        args,
        env_keys,
        enabled: true,
        added_at: Some(now_ms),
    };
    existing.push(connector.clone());
    save(store, &existing)?;
    Ok(connector)
}

/// Turn a connector on or off. Off means `resolve_mcp_configs` skips it; the
/// credentials stay, so turning it back on does not need them re-entered.
pub fn set_enabled(store: &WorkspaceStore, id: &str, enabled: bool) -> Result<Connector, String> {
    let mut all = list(store)?;
    let found = all
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("connector `{id}` not found"))?;
    found.enabled = enabled;
    let updated = found.clone();
    save(store, &all)?;
    Ok(updated)
}

/// Remove a connector and every credential it stored.
///
/// Returns the number of secrets deleted alongside it, so the caller can say
/// what actually went — "removed" without mentioning a token left behind in the
/// store would be the more comfortable lie.
pub fn remove(store: &WorkspaceStore, id: &str) -> Result<(bool, usize), String> {
    let mut all = list(store)?;
    let Some(pos) = all.iter().position(|c| c.id == id) else {
        return Ok((false, 0));
    };
    let removed = all.remove(pos);
    save(store, &all)?;

    let mut deleted = 0usize;
    for env in &removed.env_keys {
        if store.secret_delete(&secret_key(id, env))? {
            deleted += 1;
        }
    }
    Ok((true, deleted))
}

/// Which of a connector's declared credentials are actually present.
///
/// Reads `secret_list`, which returns metadata only — no value is decrypted to
/// answer this.
pub fn missing_credentials(store: &WorkspaceStore, c: &Connector) -> Result<Vec<String>, String> {
    let present: std::collections::HashSet<String> = store
        .secret_list()?
        .into_iter()
        .map(|m| m.key_name)
        .collect();
    Ok(c.env_keys
        .iter()
        .filter(|env| !present.contains(&secret_key(&c.id, env)))
        .cloned()
        .collect())
}

/// Build launchable MCP configs for every enabled connector.
///
/// Credentials are decrypted here and nowhere else. A connector whose secret
/// has gone missing is returned with that variable absent rather than blank:
/// an empty token produces an authentication error the user can act on, while
/// a variable set to "" looks to some servers like an intentional anonymous
/// mode.
pub fn resolve_mcp_configs(
    workspace: &Path,
    store: &WorkspaceStore,
) -> Result<Vec<McpServerConfig>, String> {
    let workspace = workspace.to_string_lossy().to_string();
    let mut out = Vec::new();
    for c in list(store)?.into_iter().filter(|c| c.enabled) {
        let mut env = HashMap::new();
        for key in &c.env_keys {
            if let Some(value) = store.secret_get(&secret_key(&c.id, key))? {
                env.insert(key.clone(), value);
            }
        }
        out.push(McpServerConfig {
            name: c.id.clone(),
            command: c.command.clone(),
            args: c
                .args
                .iter()
                .map(|a| a.replace(WORKSPACE_PLACEHOLDER, &workspace))
                .collect(),
            env,
        });
    }
    Ok(out)
}

/// Config-file MCP servers plus this workspace's enabled connectors.
///
/// The join that makes a connector worth adding: without it, connectors would
/// be rows in a database that nothing ever launched. A connector whose name
/// collides with a configured server loses — `~/.vibecli/config.toml` is the
/// user's explicit hand-written statement, and a UI-added row must not
/// silently take over a name they already bound.
///
/// Best-effort by design: an unopenable workspace store returns the configured
/// servers unchanged rather than failing the command, matching
/// `plugin_runtime::merge_with_plugin_hooks`.
pub fn merge_with_configured(
    workspace: &Path,
    configured: Vec<McpServerConfig>,
) -> Vec<McpServerConfig> {
    let store = match WorkspaceStore::open(workspace) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!(error = %e, "connectors: no workspace store; using configured servers only");
            return configured;
        }
    };
    let from_connectors = match resolve_mcp_configs(workspace, &store) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "connectors: could not resolve; using configured servers only");
            return configured;
        }
    };
    let taken: std::collections::HashSet<String> =
        configured.iter().map(|c| c.name.clone()).collect();
    configured
        .into_iter()
        .chain(
            from_connectors
                .into_iter()
                .filter(|c| !taken.contains(&c.name)),
        )
        .collect()
}

/// The outcome of actually launching a connector.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum ProbeResult {
    /// The server started, completed the handshake, and listed its tools.
    Ok { tools: Vec<String> },
    /// It could not be started, or it answered with an error.
    Failed { error: String },
    /// It started but said nothing before the deadline.
    TimedOut { after_secs: u64 },
}

/// How long a probe waits. Long enough for `npx` to download a package it has
/// never seen on a slow link, short enough that a hung server does not hold the
/// panel open indefinitely.
pub const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

/// Launch a connector and ask it for its tool list.
///
/// This is the only thing in this module that reports a connector as working,
/// and it does so by having made it work. Blocking — call it from
/// `spawn_blocking`.
///
/// The wait is bounded by running the client on its own thread and abandoning
/// it on timeout: `McpClient` reads its stdio with no deadline, so a server
/// that accepts a connection and then goes silent would otherwise block here
/// forever. An abandoned probe thread ends when the server it is waiting on
/// exits — which it does when our stdin handle drops — so the leak is bounded
/// by the server's own shutdown, not held open by us.
pub fn probe(cfg: McpServerConfig) -> ProbeResult {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome = vibe_ai::mcp::McpClient::connect(&cfg)
            .and_then(|mut client| client.list_tools())
            .map(|tools| tools.into_iter().map(|t| t.name).collect::<Vec<_>>())
            .map_err(|e| format!("{e:#}"));
        // The receiver is gone on timeout; nothing to do about it and nothing
        // to report to.
        let _ = tx.send(outcome);
    });

    match rx.recv_timeout(PROBE_TIMEOUT) {
        Ok(Ok(tools)) => ProbeResult::Ok { tools },
        Ok(Err(error)) => ProbeResult::Failed {
            // A failing server often echoes its own configuration back, and the
            // configuration is where the token is. `redact_secrets` removes the
            // secret shapes and leaves the sentence, which is the opposite of
            // `mask_secret` — that one masks a whole value and would leave the
            // user with an unreadable error.
            error: vibe_ai::trace::redact_secrets(&error),
        },
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => ProbeResult::TimedOut {
            after_secs: PROBE_TIMEOUT.as_secs(),
        },
        // The probe thread panicked or dropped the sender without sending.
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => ProbeResult::Failed {
            error: "the probe ended without a result".to_string(),
        },
    }
}

/// Whether a runtime's program is on PATH.
///
/// Advisory only, and reported as a distinct field rather than folded into
/// status: "npx is not installed" is a different problem from "the server
/// failed", and telling a user to check their token when the real answer is
/// "install Node" wastes their afternoon.
pub fn runtime_available(runtime: Runtime) -> bool {
    which_on_path(runtime.program())
}

fn which_on_path(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file() || candidate.with_extension("exe").is_file()
    })
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 {
        return Err("id must be 1–64 characters".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("id must be kebab-case lowercase (a-z, 0-9, -)".to_string());
    }
    Ok(())
}

fn validate_env_name(env: &str) -> Result<(), String> {
    if env.is_empty() || env.len() > 128 {
        return Err("environment variable name must be 1–128 characters".to_string());
    }
    if !env
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(format!(
            "`{env}` is not a valid environment variable name (A-Z, 0-9, _)"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, WorkspaceStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join(".vibecli").join("workspace.db");
        std::fs::create_dir_all(db.parent().expect("parent")).expect("mkdir");
        let store = WorkspaceStore::open_with(&db, [3u8; 32]).expect("store");
        (dir, store)
    }

    fn creds(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn a_credential_is_stored_encrypted_and_never_in_the_definition() {
        let (_dir, store) = temp_store();
        let c = add_from_catalog(
            &store,
            "github",
            &creds(&[("GITHUB_PERSONAL_ACCESS_TOKEN", "ghp_FIXTURE_NOT_REAL_0001")]),
            1_700_000_000_000,
        )
        .expect("add");

        // The definition carries the variable's name and nothing else.
        assert_eq!(c.env_keys, vec!["GITHUB_PERSONAL_ACCESS_TOKEN"]);
        let raw = store
            .setting_get(SETTING_KEY)
            .expect("setting")
            .expect("present");
        assert!(
            !raw.contains("ghp_FIXTURE_NOT_REAL_0001"),
            "the token reached the plaintext connector list: {raw}"
        );
        // …and it is retrievable for launch.
        assert_eq!(
            store
                .secret_get(&secret_key("github", "GITHUB_PERSONAL_ACCESS_TOKEN"))
                .expect("secret"),
            Some("ghp_FIXTURE_NOT_REAL_0001".to_string())
        );
    }

    #[test]
    fn a_required_credential_left_blank_is_refused() {
        let (_dir, store) = temp_store();
        let err = add_from_catalog(&store, "github", &creds(&[]), 0).expect_err("should refuse");
        assert!(err.contains("required"), "{err}");
        assert!(
            list(&store).expect("list").is_empty(),
            "a refused add must not leave a half-configured connector"
        );
    }

    #[test]
    fn removing_a_connector_takes_its_secrets_with_it() {
        let (_dir, store) = temp_store();
        add_from_catalog(
            &store,
            "github",
            &creds(&[("GITHUB_PERSONAL_ACCESS_TOKEN", "ghp_FIXTURE_NOT_REAL_0002")]),
            0,
        )
        .expect("add");

        let (removed, secrets) = remove(&store, "github").expect("remove");
        assert!(removed);
        assert_eq!(secrets, 1, "the token must not outlive the connector");
        assert_eq!(
            store
                .secret_get(&secret_key("github", "GITHUB_PERSONAL_ACCESS_TOKEN"))
                .expect("secret"),
            None
        );
        assert_eq!(remove(&store, "github").expect("second"), (false, 0));
    }

    #[test]
    fn the_workspace_placeholder_is_resolved_at_launch_not_at_add() {
        let (dir, store) = temp_store();
        add_from_catalog(&store, "filesystem", &creds(&[]), 0).expect("add");

        // Stored verbatim, so moving the workspace does not strand it…
        let stored = &list(&store).expect("list")[0];
        assert!(stored.args.iter().any(|a| a == WORKSPACE_PLACEHOLDER));

        // …and resolved against the real path when it is time to run.
        let cfgs = resolve_mcp_configs(dir.path(), &store).expect("resolve");
        assert_eq!(cfgs.len(), 1);
        assert!(
            cfgs[0]
                .args
                .iter()
                .any(|a| a == &dir.path().to_string_lossy()),
            "{:?}",
            cfgs[0].args
        );
    }

    #[test]
    fn a_disabled_connector_is_not_launchable() {
        let (dir, store) = temp_store();
        add_from_catalog(&store, "memory", &creds(&[]), 0).expect("add");
        assert_eq!(
            resolve_mcp_configs(dir.path(), &store).expect("on").len(),
            1
        );

        set_enabled(&store, "memory", false).expect("disable");
        assert!(
            resolve_mcp_configs(dir.path(), &store)
                .expect("off")
                .is_empty(),
            "a disabled connector must not be handed to the MCP client"
        );

        // Re-enabling does not need the connector re-added.
        set_enabled(&store, "memory", true).expect("enable");
        assert_eq!(
            resolve_mcp_configs(dir.path(), &store).expect("on").len(),
            1
        );
    }

    #[test]
    fn a_missing_secret_leaves_the_variable_unset_rather_than_empty() {
        let (dir, store) = temp_store();
        add_from_catalog(
            &store,
            "github",
            &creds(&[("GITHUB_PERSONAL_ACCESS_TOKEN", "ghp_FIXTURE_NOT_REAL_0003")]),
            0,
        )
        .expect("add");
        store
            .secret_delete(&secret_key("github", "GITHUB_PERSONAL_ACCESS_TOKEN"))
            .expect("delete");

        let cfgs = resolve_mcp_configs(dir.path(), &store).expect("resolve");
        assert!(
            !cfgs[0].env.contains_key("GITHUB_PERSONAL_ACCESS_TOKEN"),
            "an absent secret must not be passed as an empty string"
        );
        // And the panel can say which one is missing.
        let c = &list(&store).expect("list")[0];
        assert_eq!(
            missing_credentials(&store, c).expect("missing"),
            vec!["GITHUB_PERSONAL_ACCESS_TOKEN"]
        );
    }

    #[test]
    fn a_custom_connector_rejects_ids_and_env_names_that_could_escape() {
        let (_dir, store) = temp_store();
        for bad in ["../evil", "Has Spaces", "UPPER", ""] {
            assert!(
                add_custom(&store, bad, "x", "echo", vec![], &creds(&[]), 0).is_err(),
                "`{bad}` should be refused as an id"
            );
        }
        assert!(add_custom(
            &store,
            "ok-one",
            "OK",
            "echo",
            vec![],
            &creds(&[("not a var", "v")]),
            0
        )
        .is_err());
        // The valid shape does go through.
        let c = add_custom(
            &store,
            "ok-one",
            "OK",
            "echo",
            vec!["hi".into()],
            &creds(&[("TOKEN", "t")]),
            0,
        )
        .expect("valid custom connector");
        assert_eq!(c.env_keys, vec!["TOKEN"]);
        assert_eq!(c.catalog_id, None);
    }

    #[test]
    fn adding_the_same_connector_twice_is_refused() {
        let (_dir, store) = temp_store();
        add_from_catalog(&store, "memory", &creds(&[]), 0).expect("first");
        let err = add_from_catalog(&store, "memory", &creds(&[]), 0).expect_err("second");
        assert!(err.contains("already"), "{err}");
        assert_eq!(list(&store).expect("list").len(), 1);
    }

    #[test]
    fn a_probe_that_cannot_start_the_server_reports_failure_not_success() {
        // The whole point of the status model: no key, no guess. A command
        // that does not exist must come back Failed.
        let result = probe(McpServerConfig {
            name: "nope".into(),
            command: "vibecody-no-such-binary-0000".into(),
            args: vec![],
            env: HashMap::new(),
        });
        assert!(
            matches!(result, ProbeResult::Failed { .. }),
            "{result:?} — a server that cannot start is not working"
        );
    }

    #[test]
    fn catalog_entries_are_unique_and_describe_their_credentials() {
        let mut ids: Vec<&str> = CATALOG.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate connector id");

        for c in CATALOG {
            assert!(!c.command.is_empty(), "{}", c.id);
            for field in c.credentials {
                validate_env_name(field.env).unwrap_or_else(|e| panic!("{}: {e}", c.id));
                assert!(
                    !field.help.is_empty(),
                    "{} has an unexplained credential",
                    c.id
                );
            }
        }
    }
}
