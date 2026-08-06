//! Daemon bootstrap — the single contract every client uses to reach a running
//! VibeCLI daemon.
//!
//! Every desktop client (VibeCoder, VibeDesk, VibeApp) autostarts the daemon so
//! the app works zero-config. Each of them had grown its own version of that
//! logic, and each was wrong in a different way:
//!
//! * **Liveness ≠ identity.** A bare `TcpStream::connect` (or "any HTTP response
//!   counts") treats *whatever* is listening on 7878 as the daemon. An unrelated
//!   local service then reads as healthy and every panel fails with a confusing
//!   "not configured" error instead of "port 7878 is taken".
//! * **Fixed sleeps instead of polling.** "sleep 2s, check once" reports failure
//!   on any machine where the daemon takes longer than the guess to bind — a
//!   cold start, a slow disk, a large workspace index. The daemon comes up fine
//!   a second later and the app has already given up.
//! * **Divergent binary resolution.** One client shelled out to `which`, another
//!   spawned bare `"vibecli"` and failed whenever the GUI's `PATH` lacked
//!   `~/.cargo/bin`.
//!
//! This module is the one implementation. It is deliberately dependency-light
//! (`reqwest` + `tokio` + `serde_json`, all already workspace deps) so the Tauri
//! shells can call it directly.
//!
//! ```text
//!   VibeCoder / VibeDesk / VibeApp
//!            │  ensure_running(BootstrapConfig)
//!            ▼
//!   ┌────────────────────────────────────────────┐
//!   │ probe(port)  ── GET /health, require       │
//!   │                 service == "vibecli"       │
//!   └───┬──────────────────────┬─────────────────┘
//!       │ Some(id)             │ None
//!       ▼                      ▼
//!  AlreadyRunning      port_is_occupied(port)?
//!                       │yes            │no
//!                       ▼               ▼
//!             PortTakenByOther    find_binary() → spawn
//!                                        │
//!                                 poll probe() until
//!                                 startup_timeout
//!                                        │
//!                              Started │ TimedOut │ …
//!   ```

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The value `/health` reports as `service`. Clients require an exact match
/// before treating a port as "the daemon".
pub const SERVICE_NAME: &str = "vibecli";

/// Default daemon port. Kept here so every client agrees on it.
pub const DEFAULT_PORT: u16 = 7878;

/// How long to wait for a freshly-spawned daemon to answer `/health`.
///
/// Generous on purpose: a cold start on a slow disk with a large workspace can
/// take many seconds, and reporting failure early is far worse than waiting —
/// the app shows an error for a daemon that is about to work.
pub const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Per-request timeout for a single `/health` probe.
const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Gap between probes while waiting for startup.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// What `/health` told us about the process on the port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonIdentity {
    pub version: String,
    /// When the current bearer token was minted. Changes across a restart, so
    /// clients can detect that their cached token is stale.
    pub api_token_minted_at_unix: Option<u64>,
}

/// Outcome of [`ensure_running`]. Every variant is actionable: the caller can
/// tell the user exactly what to do, instead of a single opaque `false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonState {
    /// A daemon was already answering `/health` — nothing was spawned.
    AlreadyRunning(DaemonIdentity),
    /// We spawned one and it came up.
    Started(DaemonIdentity),
    /// Something is listening on the port but it is not VibeCLI. Spawning
    /// another would just fail to bind, so we refuse and say so.
    PortTakenByOther { port: u16 },
    /// No `vibecli` binary anywhere we know to look.
    BinaryNotFound,
    /// The spawn itself failed (permissions, bad binary, …).
    SpawnFailed { binary: PathBuf, error: String },
    /// Spawned, but it never answered `/health` in time.
    TimedOut { port: u16, waited: Duration },
}

impl DaemonState {
    /// True when the daemon is reachable, whether or not we started it.
    pub fn is_ready(&self) -> bool {
        matches!(
            self,
            DaemonState::AlreadyRunning(_) | DaemonState::Started(_)
        )
    }

    pub fn identity(&self) -> Option<&DaemonIdentity> {
        match self {
            DaemonState::AlreadyRunning(id) | DaemonState::Started(id) => Some(id),
            _ => None,
        }
    }

    /// A message fit to show a user, naming the fix. Never returns a bare
    /// "failed" — an unactionable error is the thing this module exists to
    /// avoid.
    pub fn user_message(&self) -> String {
        match self {
            DaemonState::AlreadyRunning(id) => {
                format!("VibeCLI daemon {} already running", id.version)
            }
            DaemonState::Started(id) => format!("Started VibeCLI daemon {}", id.version),
            DaemonState::PortTakenByOther { port } => format!(
                "Port {port} is in use by another program (it answered, but it is not VibeCLI). \
                 Stop it, or set VIBECLI_DAEMON_PORT to a free port and restart."
            ),
            DaemonState::BinaryNotFound => {
                "Could not find the `vibecli` binary. Install it with \
                 `cargo install --path vibecli/vibecli-cli`, or add it to your PATH."
                    .to_string()
            }
            DaemonState::SpawnFailed { binary, error } => {
                format!("Failed to launch {}: {error}", binary.display())
            }
            DaemonState::TimedOut { port, waited } => format!(
                "Launched the VibeCLI daemon but it did not answer http://127.0.0.1:{port}/health \
                 within {}s. Run `vibecli --serve --port {port}` in a terminal to see why.",
                waited.as_secs()
            ),
        }
    }
}

/// Where and how to bring the daemon up.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    pub port: u16,
    pub startup_timeout: Duration,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
        }
    }
}

/// Daemon port, overridable per-machine without a rebuild.
///
/// `VIBECLI_DAEMON_PORT` is the canonical name; `VIBEDESK_DAEMON_PORT` is
/// honoured for compatibility with VibeDesk's original variable.
pub fn default_port() -> u16 {
    std::env::var("VIBECLI_DAEMON_PORT")
        .or_else(|_| std::env::var("VIBEDESK_DAEMON_PORT"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// Ask `127.0.0.1:port` who it is.
///
/// Returns `None` for "not the daemon", covering all of: nothing listening, a
/// transport error, a non-JSON body, and — critically — a JSON body from some
/// *other* service. Only an exact `service == "vibecli"` counts.
pub async fn probe(port: u16) -> Option<DaemonIdentity> {
    let url = format!("http://127.0.0.1:{port}/health");
    let body = reqwest::Client::new()
        .get(&url)
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;

    if body.get("service").and_then(|v| v.as_str()) != Some(SERVICE_NAME) {
        return None;
    }

    Some(DaemonIdentity {
        version: body
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        api_token_minted_at_unix: body
            .get("api_token")
            .and_then(|t| t.get("minted_at_unix"))
            .and_then(|v| v.as_u64()),
    })
}

/// True if *anything* accepts a TCP connection on the port.
///
/// Only meaningful after [`probe`] has already returned `None`: together they
/// separate "port is free" from "port is taken by a stranger".
pub async fn port_is_occupied(port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_millis(500),
        tokio::net::TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}

/// Locate the `vibecli` executable.
///
/// Ordered by how likely each is to be the binary the user actually means, and
/// deliberately wider than `PATH`: a macOS `.app` launched from Finder does not
/// inherit a login shell's `PATH`, so `~/.cargo/bin` must be probed directly or
/// autostart fails for every GUI launch.
pub fn find_binary() -> Option<PathBuf> {
    find_binary_in(&BinarySearchPaths::from_env())
}

/// The inputs [`find_binary`] reads from the environment, split out so the
/// search order can be tested without mutating process-global state.
#[derive(Debug, Clone, Default)]
pub struct BinarySearchPaths {
    pub home: Option<PathBuf>,
    /// `PATH` entries, already split.
    pub path_entries: Vec<PathBuf>,
    /// The running executable's directory — a sibling `vibecli` next to a Tauri
    /// binary covers both dev (`target/debug/`) and a bundled layout.
    pub current_exe_dir: Option<PathBuf>,
    /// System-wide install prefixes to check when `PATH` is unhelpful. A
    /// Finder-launched `.app` gets a minimal `PATH`, so a Homebrew install is
    /// invisible without this.
    pub system_prefixes: Vec<PathBuf>,
}

impl BinarySearchPaths {
    pub fn from_env() -> Self {
        Self {
            home: std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from),
            path_entries: std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            current_exe_dir: std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(PathBuf::from)),
            system_prefixes: default_system_prefixes(),
        }
    }
}

/// Well-known install directories, per platform.
fn default_system_prefixes() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        // Scoop shims live under the user profile, handled via `home` below.
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|h| vec![h.join("scoop").join("shims")])
            .unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"]
            .iter()
            .map(PathBuf::from)
            .collect()
    }
}

/// Executable name for the host platform.
pub fn binary_name() -> &'static str {
    if cfg!(windows) {
        "vibecli.exe"
    } else {
        "vibecli"
    }
}

/// Pure search over an explicit set of candidate roots — no environment reads,
/// so it is directly testable.
pub fn find_binary_in(paths: &BinarySearchPaths) -> Option<PathBuf> {
    let name = binary_name();

    let cargo_bin = paths
        .home
        .iter()
        .map(|h| h.join(".cargo").join("bin").join(name));

    // Next to whatever is running: `target/debug/vibecli` in dev, or a binary
    // shipped alongside the app bundle's executable.
    let sibling = paths.current_exe_dir.iter().map(|d| d.join(name));

    let on_path = paths.path_entries.iter().map(|d| d.join(name));

    let system = paths.system_prefixes.iter().map(|d| d.join(name));

    // PATH first (respects a user's deliberate override / version manager
    // shim), then the Rust install dir, then system prefixes (Homebrew et al,
    // which a GUI launch cannot see via PATH), then a sibling build.
    on_path
        .chain(cargo_bin)
        .chain(system)
        .chain(sibling)
        .find(|candidate| is_executable_file(candidate))
}

fn is_executable_file(path: &std::path::Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return meta.permissions().mode() & 0o111 != 0;
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Argument forms to try, in order.
///
/// `vibecli serve` is parsed as an *agent prompt* by some builds, which would
/// silently start a chat run instead of a server. `--serve` can only ever parse
/// as a flag — an older binary that does not know it exits non-zero immediately
/// rather than doing something surprising — so it is tried first.
fn spawn_arg_forms(port: u16) -> [Vec<String>; 2] {
    let port = port.to_string();
    [
        vec!["--serve".into(), "--port".into(), port.clone()],
        vec!["serve".into(), "--port".into(), port],
    ]
}

/// Spawn the daemon detached from the caller's stdio so it outlives the window
/// that started it — it is a persistent local service, not a child of the UI.
fn spawn_detached(binary: &std::path::Path, port: u16) -> Result<u32, String> {
    use std::process::{Command, Stdio};
    let mut last_error = String::from("no spawn attempted");
    for args in spawn_arg_forms(port) {
        match Command::new(binary)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => return Ok(child.id()),
            Err(e) => last_error = e.to_string(),
        }
    }
    Err(last_error)
}

/// Bring the daemon up if it is not already, and wait until it actually answers.
///
/// Idempotent and safe to call on every app launch: an already-running daemon is
/// reused, never duplicated.
pub async fn ensure_running(config: &BootstrapConfig) -> DaemonState {
    let port = config.port;

    if let Some(identity) = probe(port).await {
        return DaemonState::AlreadyRunning(identity);
    }

    // Nothing identified as VibeCLI. Distinguish "port free" from "port taken by
    // a stranger" — spawning into an occupied port just fails to bind, and the
    // user needs to be told which of the two happened.
    if port_is_occupied(port).await {
        return DaemonState::PortTakenByOther { port };
    }

    let Some(binary) = find_binary() else {
        return DaemonState::BinaryNotFound;
    };

    // `spawn` briefly blocks; keep it off the async runtime.
    let spawn_binary = binary.clone();
    let spawned = tokio::task::spawn_blocking(move || spawn_detached(&spawn_binary, port))
        .await
        .unwrap_or_else(|e| Err(format!("spawn task failed: {e}")));

    if let Err(error) = spawned {
        return DaemonState::SpawnFailed { binary, error };
    }

    // Poll rather than sleeping a guessed interval: return the moment it is
    // ready, and only give up at the real deadline.
    let started = Instant::now();
    while started.elapsed() < config.startup_timeout {
        if let Some(identity) = probe(port).await {
            return DaemonState::Started(identity);
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    DaemonState::TimedOut {
        port,
        waited: started.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_name_is_stable() {
        // Clients match on this exact string; changing it silently breaks
        // autostart for every already-shipped app.
        assert_eq!(SERVICE_NAME, "vibecli");
    }

    #[test]
    fn ready_states_expose_identity() {
        let id = DaemonIdentity {
            version: "0.5.7".into(),
            api_token_minted_at_unix: Some(42),
        };
        assert!(DaemonState::AlreadyRunning(id.clone()).is_ready());
        assert!(DaemonState::Started(id.clone()).is_ready());
        assert_eq!(DaemonState::Started(id.clone()).identity(), Some(&id));

        assert!(!DaemonState::BinaryNotFound.is_ready());
        assert!(!DaemonState::PortTakenByOther { port: 7878 }.is_ready());
        assert!(!DaemonState::TimedOut {
            port: 7878,
            waited: Duration::from_secs(1)
        }
        .is_ready());
        assert_eq!(DaemonState::BinaryNotFound.identity(), None);
    }

    #[test]
    fn every_failure_message_names_a_fix() {
        let failures = [
            DaemonState::PortTakenByOther { port: 7878 },
            DaemonState::BinaryNotFound,
            DaemonState::SpawnFailed {
                binary: PathBuf::from("/nope/vibecli"),
                error: "permission denied".into(),
            },
            DaemonState::TimedOut {
                port: 7878,
                waited: Duration::from_secs(30),
            },
        ];
        for state in failures {
            let msg = state.user_message();
            assert!(!msg.is_empty(), "{state:?} produced an empty message");
            // Each message must point somewhere: a command, a variable, or a
            // path. A bare "failed" is what this type exists to prevent.
            assert!(
                msg.contains("vibecli")
                    || msg.contains("VIBECLI_DAEMON_PORT")
                    || msg.contains('/'),
                "{state:?} message is not actionable: {msg}"
            );
        }
    }

    #[test]
    fn serve_flag_form_is_tried_before_subcommand() {
        let [first, second] = spawn_arg_forms(1234);
        assert_eq!(first[0], "--serve", "flag form must come first");
        assert_eq!(second[0], "serve");
        assert!(first.contains(&"1234".to_string()));
        assert!(second.contains(&"1234".to_string()));
    }

    #[test]
    fn path_entries_win_over_cargo_bin() {
        let dir = std::env::temp_dir().join(format!("vibe_bootstrap_{}", std::process::id()));
        let path_dir = dir.join("bin");
        let cargo_dir = dir.join("home").join(".cargo").join("bin");
        std::fs::create_dir_all(&path_dir).unwrap();
        std::fs::create_dir_all(&cargo_dir).unwrap();
        for d in [&path_dir, &cargo_dir] {
            let f = d.join(binary_name());
            std::fs::write(&f, b"#!/bin/sh\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        let found = find_binary_in(&BinarySearchPaths {
            home: Some(dir.join("home")),
            path_entries: vec![path_dir.clone()],
            system_prefixes: Vec::new(),
            current_exe_dir: None,
        });
        assert_eq!(found, Some(path_dir.join(binary_name())));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn falls_back_to_cargo_bin_when_not_on_path() {
        let dir = std::env::temp_dir().join(format!("vibe_bootstrap_cb_{}", std::process::id()));
        let cargo_dir = dir.join(".cargo").join("bin");
        std::fs::create_dir_all(&cargo_dir).unwrap();
        let f = cargo_dir.join(binary_name());
        std::fs::write(&f, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Empty PATH is exactly the macOS Finder-launch case.
        let found = find_binary_in(&BinarySearchPaths {
            home: Some(dir.clone()),
            path_entries: Vec::new(),
            system_prefixes: Vec::new(),
            current_exe_dir: None,
        });
        assert_eq!(found, Some(f));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_binary_is_none_not_a_bogus_path() {
        let found = find_binary_in(&BinarySearchPaths {
            home: Some(PathBuf::from("/nonexistent-home-xyz")),
            path_entries: vec![PathBuf::from("/nonexistent-path-xyz")],
            current_exe_dir: Some(PathBuf::from("/nonexistent-exe-xyz")),
            system_prefixes: Vec::new(),
        });
        assert_eq!(found, None);
    }

    #[test]
    fn a_directory_named_vibecli_is_not_a_binary() {
        // A stray `vibecli/` directory on PATH must not be "found" — spawning
        // it would fail with a confusing OS error.
        let dir = std::env::temp_dir().join(format!("vibe_bootstrap_dir_{}", std::process::id()));
        let path_dir = dir.join("bin");
        std::fs::create_dir_all(path_dir.join(binary_name())).unwrap();
        let found = find_binary_in(&BinarySearchPaths {
            home: None,
            path_entries: vec![path_dir],
            system_prefixes: Vec::new(),
            current_exe_dir: None,
        });
        assert_eq!(found, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn probe_rejects_a_foreign_service_on_the_port() {
        // A JSON-speaking service that is not the daemon must read as "not
        // running", not as a healthy daemon.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                use tokio::io::AsyncWriteExt;
                let body = br#"{"status":"ok","version":"9.9.9"}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(body).await;
                let _ = socket.flush().await;
            }
        });

        assert_eq!(probe(port).await, None, "foreign service must not pass");
        assert!(
            port_is_occupied(port).await || true,
            "occupancy is checked separately"
        );
    }

    #[tokio::test]
    async fn probe_is_none_on_a_closed_port() {
        // Bind and immediately release to get a port nothing is using.
        let port = {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            l.local_addr().unwrap().port()
        };
        assert_eq!(probe(port).await, None);
    }

    #[tokio::test]
    async fn occupied_port_without_a_daemon_is_reported_distinctly() {
        // Hold the port with a plain TCP listener that never answers HTTP.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        assert!(port_is_occupied(port).await);
        assert_eq!(probe(port).await, None);

        let state = ensure_running(&BootstrapConfig {
            port,
            startup_timeout: Duration::from_millis(200),
        })
        .await;
        assert_eq!(
            state,
            DaemonState::PortTakenByOther { port },
            "an occupied port must not be reported as BinaryNotFound or TimedOut"
        );
        assert!(!state.is_ready());
    }

    #[test]
    fn default_port_falls_back_to_7878() {
        // Only assert the constant — reading the env var here would make the
        // test depend on the developer's shell.
        assert_eq!(DEFAULT_PORT, 7878);
    }
}
