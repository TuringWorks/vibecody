//! Does every connector in the catalog actually work?
//!
//! The catalog is a list of promises: seventeen entries, each claiming that a
//! command exists, starts, speaks MCP and offers tools. Nothing checked any of
//! that. The one that mattered was `everything`, which completed its handshake
//! and reported **zero of its thirteen tools** because the MCP client read a
//! notification as the reply — a connector that gave the agent nothing, shown
//! as working.
//!
//! Two levels here, deliberately kept apart:
//!
//!   * `the_catalog_is_well_formed` is a plain unit test. It runs everywhere,
//!     costs nothing, and covers all seventeen entries including the eight that
//!     need credentials this machine does not have.
//!   * `every_credential_free_connector_starts_and_offers_tools` actually
//!     launches them. It is `#[ignore]`d because it needs npx, uvx and the
//!     network, and it reports per-connector results rather than a bare
//!     pass/fail so a partial outage is legible.
//!
//! Run the live one with:
//!
//! ```text
//! cargo test -p vibecli --test connector_catalog_bdd -- --ignored --nocapture
//! ```

use std::collections::HashMap;

use vibe_ai::mcp::McpServerConfig;
use vibecli_cli::connectors::{probe, ProbeResult, CATALOG, WORKSPACE_PLACEHOLDER};

/// Given the shipped catalog,
/// When every entry is inspected,
/// Then each is complete enough to be launchable.
///
/// A malformed entry cannot be caught by the type system: `command: ""` and an
/// empty id both compile. This is the check that covers the credential-gated
/// connectors, which the live test below cannot reach.
#[test]
fn the_catalog_is_well_formed() {
    assert!(!CATALOG.is_empty(), "catalog should not be empty");

    let mut ids = std::collections::HashSet::new();
    for spec in CATALOG {
        assert!(!spec.id.is_empty(), "a connector has an empty id");
        assert!(
            ids.insert(spec.id),
            "duplicate connector id `{}` — the later one would shadow the earlier",
            spec.id
        );
        assert!(!spec.title.is_empty(), "{}: empty title", spec.id);
        assert!(!spec.description.is_empty(), "{}: empty description", spec.id);
        assert!(!spec.category.is_empty(), "{}: empty category", spec.id);
        assert!(
            !spec.command.is_empty(),
            "{}: empty command — nothing to launch",
            spec.id
        );
        assert!(
            spec.docs_url.starts_with("https://"),
            "{}: docs_url should be an https URL, got {:?}",
            spec.id,
            spec.docs_url
        );
        for cred in spec.credentials {
            assert!(
                !cred.env.is_empty(),
                "{}: a credential has no env var name",
                spec.id
            );
            assert!(
                cred.env.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()),
                "{}: credential `{}` should be an ENV_STYLE name",
                spec.id,
                cred.env
            );
            assert!(
                !cred.label.is_empty(),
                "{}: credential `{}` has no label, so the form field would be blank",
                spec.id,
                cred.env
            );
        }
    }
}

/// Build the config the daemon would build.
///
/// `{workspace}` in a catalog entry's args is a placeholder the daemon
/// substitutes in `resolve_mcp_configs`; `filesystem` is scoped by it. Passing
/// the args verbatim launches the server pointed at a literal `{workspace}`
/// directory, which fails — a defect in the caller, not the connector, and one
/// this test made itself before it made it anywhere else.
fn config_for(
    spec: &vibecli_cli::connectors::ConnectorSpec,
    workspace: &std::path::Path,
    env: HashMap<String, String>,
) -> McpServerConfig {
    let workspace = workspace.display().to_string();
    McpServerConfig {
        name: spec.id.to_string(),
        command: spec.command.to_string(),
        args: spec
            .args
            .iter()
            .map(|a| a.replace(WORKSPACE_PLACEHOLDER, &workspace))
            .collect(),
        env,
    }
}

/// Given every connector that needs no credentials,
/// When each is launched,
/// Then it completes a handshake and offers at least one tool.
///
/// "At least one tool" rather than an exact count: these are third-party
/// servers and pinning numbers would turn their releases into failures here.
/// But zero is not a number they are allowed to return — a connector offering
/// no tools is one the agent cannot use, which is exactly the state that hid
/// the notification bug.
///
/// `git` is given a real repository. It refuses to start outside one, which is
/// correct behaviour and not a catalog defect; running it in a bare temp dir
/// would report a working connector as broken.
#[test]
#[ignore = "launches npx/uvx servers; run with --ignored"]
fn every_credential_free_connector_starts_and_offers_tools() {
    let repo = tempfile::tempdir().expect("tempdir");
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(repo.path())
        .status()
        .expect("git init");

    let mut failures = Vec::new();
    let mut checked = 0;

    for spec in CATALOG.iter().filter(|s| s.credentials.is_empty()) {
        // `git` takes the repository as an argument; everything else is
        // launched as the catalog declares it.
        let mut cfg = config_for(spec, repo.path(), HashMap::new());
        if spec.id == "git" {
            cfg.args.push("--repository".to_string());
            cfg.args.push(repo.path().display().to_string());
        }

        checked += 1;
        match probe(cfg) {
            ProbeResult::Ok { tools } if !tools.is_empty() => {
                println!("  ok       {:<20} {} tools", spec.id, tools.len());
            }
            ProbeResult::Ok { tools: _ } => {
                println!("  NO TOOLS {:<20}", spec.id);
                failures.push(format!(
                    "{}: started but offered no tools — unusable by the agent",
                    spec.id
                ));
            }
            ProbeResult::Failed { error } => {
                println!("  FAILED   {:<20} {}", spec.id, error);
                failures.push(format!("{}: {}", spec.id, error));
            }
            ProbeResult::TimedOut { after_secs } => {
                println!("  TIMEOUT  {:<20} {}s", spec.id, after_secs);
                failures.push(format!("{}: no answer after {after_secs}s", spec.id));
            }
        }
    }

    assert!(checked >= 8, "expected the catalog's credential-free connectors, checked {checked}");
    assert!(
        failures.is_empty(),
        "connectors that do not work:\n  {}",
        failures.join("\n  ")
    );
}
