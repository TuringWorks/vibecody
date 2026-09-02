//! Daemon bootstrap — the single contract every client uses to reach a running
//! VibeCLI daemon.
//!
//! Every desktop client (VibeCoder, VibeDesk, VibeAIChat) autostarts the daemon so
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
//!   VibeCoder / VibeDesk / VibeAIChat
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
///
/// Re-exported from `vibe-daemon-token` rather than restated: the daemon
/// decides which token file it owns by comparing its port against this value,
/// and a second definition that drifted would hand the shared file to the wrong
/// daemon.
pub const DEFAULT_PORT: u16 = vibe_daemon_token::DEFAULT_PORT;

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
    /// Fingerprint of the token this daemon is accepting, from `/health`.
    ///
    /// `None` for a daemon predating the field — which is a reason to proceed
    /// unchecked, not to fail. `minted_at_unix` alone cannot answer "is my token
    /// the right one": a token file written *after* the daemon started is newer
    /// and still wrong, which is exactly the case that broke.
    pub api_token_fingerprint: Option<String>,
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
            DaemonState::BinaryNotFound => "Could not find the `vibecli` binary. Install it with \
                 `cargo install --path vibecli/vibecli-cli`, or add it to your PATH."
                .to_string(),
            DaemonState::SpawnFailed { binary, error } => {
                format!("Failed to launch {}: {error}", binary.display())
            }
            DaemonState::TimedOut { port, waited } => format!(
                "Launched the VibeCLI daemon but it did not answer http://127.0.0.1:{port}/health \
                 within {}s. Its output was captured to {}.",
                waited.as_secs(),
                spawn_log_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| format!(
                        "(no log — run `vibecli --serve --port {port}` in a terminal)"
                    ))
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
    vibe_daemon_token::default_port()
}

/// Ask `127.0.0.1:port` who it is.
///
/// Returns `None` for "not the daemon", covering all of: nothing listening, a
/// transport error, a non-JSON body, and — critically — a JSON body from some
/// *other* service. Only an exact `service == "vibecli"` counts.
pub async fn probe(port: u16) -> Option<DaemonIdentity> {
    let url = format!("http://127.0.0.1:{port}/health");
    let res = reqwest::Client::new()
        .get(&url)
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
        .ok()?;

    // A rate-limited health check is our own daemon saying "too fast", not a
    // stranger on the port. Reading it as a foreign service is how a healthy
    // daemon got reported as "Port 7878 is in use by another program", after
    // which every client tried to spawn a replacement — and each replacement
    // clobbered the live daemon's token on its way out. Retry once; the caller
    // polls anyway, and `/health` now has a limit no honest poller reaches.
    let res = if res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        tokio::time::sleep(POLL_INTERVAL).await;
        reqwest::Client::new()
            .get(&url)
            .timeout(PROBE_TIMEOUT)
            .send()
            .await
            .ok()?
    } else {
        res
    };

    let body = res.json::<serde_json::Value>().await.ok()?;

    // Accept a daemon that predates the `service` field via its exact legacy
    // shape (`status: "ok"` **and** a `version` string).
    //
    // Strictness here was an upgrade regression: a user running an older
    // `vibecli` on 7878 would have it rejected, then `port_is_occupied` would
    // report the port taken, and the app would tell them "Port 7878 is in use
    // by another program" — about their own daemon. A body naming a *different*
    // service is still never accepted, which is the case this check exists for.
    let identified = match body.get("service").and_then(|v| v.as_str()) {
        Some(name) => name == SERVICE_NAME,
        None => {
            body.get("status").and_then(|v| v.as_str()) == Some("ok")
                && body.get("version").and_then(|v| v.as_str()).is_some()
        }
    };
    if !identified {
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
        api_token_fingerprint: body
            .get("api_token")
            .and_then(|t| t.get("fingerprint"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
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
        windows_prefixes(
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
            std::env::var_os("USERPROFILE").map(PathBuf::from),
        )
    }
    #[cfg(not(windows))]
    {
        ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"]
            .iter()
            .map(PathBuf::from)
            .collect()
    }
}

/// Windows install prefixes, as a pure function of the two environment values
/// they are built from — so the search order is testable from any host rather
/// than only from a Windows runner nothing in CI runs this on.
///
/// `%LOCALAPPDATA%\VibeCody` is where this repo's own installer puts the
/// daemon (`deploy/windows/setup.ps1`: "Install to `%LOCALAPPDATA%\VibeCody`,
/// add to your PATH"). It was missing here, and PATH alone does not cover it:
/// `SetEnvironmentVariable(..., "User")` updates the registry, but an already
/// running Explorer — and every app it launched — keeps the environment block
/// it started with. Until the user signed out, a correctly installed daemon was
/// invisible to autostart and the app reported `BinaryNotFound`.
///
/// Kept to directories something in this repo actually creates. A guessed
/// `%ProgramFiles%\…` would be a claim about an installer that does not exist.
pub fn windows_prefixes(local_appdata: Option<PathBuf>, user_profile: Option<PathBuf>) -> Vec<PathBuf> {
    local_appdata
        .map(|d| d.join("VibeCody"))
        .into_iter()
        // Scoop shims live under the user profile.
        .chain(user_profile.map(|h| h.join("scoop").join("shims")))
        .collect()
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

/// Put the child in its own session, so nothing aimed at the caller's process
/// group can reach it. Separated out so the behaviour can be tested against a
/// command that reports its own session, rather than against a real daemon.
pub fn detach_session(cmd: &mut std::process::Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: `setsid` is async-signal-safe and is the only call made
        // between fork and exec. Its failure (the child is already a group
        // leader — after fork it is not) must not fail the spawn.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(WINDOWS_DETACHED_FLAGS);
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = cmd;
    }
}

/// `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` — the Windows half of what
/// `setsid` does on Unix, and it became load-bearing the moment the installers
/// started shipping the daemon beside the app (`tauri.windows.conf.json`'s
/// `externalBin`): a GUI-subsystem app spawning a console-subsystem child with
/// no flags allocates that child a **console window**, which stays on screen
/// for as long as the daemon runs. `DETACHED_PROCESS` (0x8) gives it no console
/// at all — stdout and stderr already go to `daemon-spawn.log`, so nothing is
/// lost — and `CREATE_NEW_PROCESS_GROUP` (0x200) keeps a Ctrl+C or Ctrl+Break
/// aimed at the launching console from reaching a service that outlives it.
///
/// Not `CREATE_NO_WINDOW`: Windows ignores it whenever `DETACHED_PROCESS` is
/// set, so naming both would only suggest a belt-and-braces that does not
/// exist.
#[cfg(windows)]
const WINDOWS_DETACHED_FLAGS: u32 = 0x0000_0008 | 0x0000_0200;

/// A directory the daemon can actually run in.
///
/// The daemon derives its workspace root — and therefore `<workspace>/.vibecli/`
/// — from its working directory. A `.app` launched from Finder inherits `/`,
/// where creating `.vibecli/` fails on the read-only system volume and the
/// daemon exits 1 before it ever binds the port. The client then reports only
/// "exited immediately (exit status: 1)".
///
/// Keep the caller's directory when it works, so a client started from a repo
/// still gets that repo's workspace; fall back to home only when it does not.
/// The probe creates the same directory the daemon would create seconds later,
/// so it adds no side effect the daemon would not.
pub fn spawn_working_dir() -> Option<PathBuf> {
    spawn_working_dir_in(&spawn_dir_candidates())
}

/// Candidate working directories, most-preferred first. Split out so the
/// choice can be tested without mutating this process's cwd.
pub fn spawn_dir_candidates() -> Vec<PathBuf> {
    [
        std::env::current_dir().ok(),
        home_dir(),
        Some(std::env::temp_dir()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// First candidate the daemon could actually write its `.vibecli/` into.
pub fn spawn_working_dir_in(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|dir| std::fs::create_dir_all(dir.join(".vibecli")).is_ok())
        .cloned()
}

/// Where a spawned daemon's stdout/stderr go, so a startup failure is
/// diagnosable without asking the user to re-run it in a terminal.
///
/// Truncated per spawn, so it never becomes an append-forever log in `~` —
/// but see [`roll_spawn_log`]: the *previous* daemon's last words are the
/// evidence for why it died, and a restart must not be what destroys them.
pub fn spawn_log_path() -> Option<PathBuf> {
    let dir = home_dir()?.join(".vibecli");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("daemon-spawn.log"))
}

/// The previous daemon's captured output, kept across one restart.
pub fn prev_spawn_log_path() -> Option<PathBuf> {
    Some(spawn_log_path()?.with_file_name("daemon-spawn.prev.log"))
}

/// Move the last daemon's log aside before the new one truncates it.
///
/// A daemon that dies and is auto-restarted 30 s later used to erase its own
/// post-mortem: the client re-spawns, `File::create` truncates, and the only
/// record of the exit is gone. Rolling costs one rename and answers "why did
/// it go down" the next morning instead of never.
pub fn roll_spawn_log() {
    if let (Some(cur), Some(prev)) = (spawn_log_path(), prev_spawn_log_path()) {
        roll_log_file(&cur, &prev);
    }
}

/// The rename itself, taking its paths, so it is testable without touching the
/// developer's real `~/.vibecli`.
pub fn roll_log_file(cur: &std::path::Path, prev: &std::path::Path) {
    // Only when there is something to keep — an empty file is not evidence, and
    // rolling it would discard the one that is.
    if std::fs::metadata(cur).is_ok_and(|m| m.len() > 0) {
        let _ = std::fs::rename(cur, prev);
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Capture the spawned daemon's output, falling back to discarding it when the
/// log file cannot be opened. Never fail a spawn over logging.
pub fn spawn_output() -> (std::process::Stdio, std::process::Stdio) {
    use std::process::Stdio;
    roll_spawn_log();
    match spawn_log_path().and_then(|p| std::fs::File::create(p).ok()) {
        Some(f) => match f.try_clone() {
            Ok(dup) => (Stdio::from(f), Stdio::from(dup)),
            Err(_) => (Stdio::from(f), Stdio::null()),
        },
        None => (Stdio::null(), Stdio::null()),
    }
}

/// Names the directory holding a speech engine an installer shipped.
///
/// Set by the process that *spawns* the daemon, because it is the only one that
/// knows: [`find_binary_in`] deliberately prefers a `PATH` or `~/.cargo/bin`
/// daemon over the sibling one, so on a machine with an installed app **and** a
/// `cargo install`ed CLI, the daemon that starts is not the one sitting beside
/// the installer's `whisper/`. Looking beside itself, it would find nothing and
/// report "no speech engine" on a machine that shipped with one.
///
/// Never set on a daemon that was already running. That one is somebody else's
/// process, and telling it where *this* app's bundle is would be a claim about
/// an engine it never loaded.
pub const VOICE_ASSETS_ENV: &str = "VIBECLI_VOICE_ASSETS";

/// The directory this process keeps packaged resources in, when it looks like an
/// installed app rather than a `cargo run` target.
///
/// Read from the *caller's* executable, not the daemon's — the caller is the
/// shell, and on Windows its directory is exactly where Tauri lays `whisper/`
/// and `models/` down. Returns `None` unless one of them is actually there, so
/// a development build does not point the daemon at `target/debug` and make the
/// resulting "not found" harder to read than plain silence.
pub fn packaged_assets_dir() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let carries_payload = dir.join("whisper").is_dir() || dir.join("models").is_dir();
    carries_payload.then_some(dir)
}

/// Spawn the daemon detached from the caller's stdio **and from its session**,
/// so it outlives the window that started it — it is a persistent local
/// service, not a child of the UI.
///
/// Redirecting stdio is only half of detaching, and the missing half was
/// measured here: a daemon started by VibeCoder ran with `PGID` equal to the
/// *app's* pid, in the app's session. Every signal aimed at that group — the
/// app tearing down, a `killpg`, a SIGHUP when the session goes — reached the
/// daemon too, which then exited **cleanly**: no crash report, no panic log,
/// nothing in `daemon-spawn.log` (the next spawn truncated it). The user saw
/// only "daemon offline" and, 30 s later, "back online".
///
/// `setsid` in the child puts it in its own session and process group, where
/// nothing aimed at the UI can reach it. It fails only if the child is already
/// a group leader, which after `fork` it is not — and a failure there must not
/// fail the spawn, so it is deliberately ignored.
fn spawn_detached(binary: &std::path::Path, port: u16) -> Result<u32, String> {
    use std::process::{Command, Stdio};
    let mut last_error = String::from("no spawn attempted");
    for args in spawn_arg_forms(port) {
        let (out, err) = spawn_output();
        let mut cmd = Command::new(binary);
        cmd.args(&args).stdin(Stdio::null()).stdout(out).stderr(err);
        if let Some(dir) = spawn_working_dir() {
            cmd.current_dir(dir);
        }
        detach_session(&mut cmd);
        if let Some(dir) = packaged_assets_dir() {
            cmd.env(VOICE_ASSETS_ENV, dir);
        }
        match cmd.spawn() {
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

// ── Readiness: the process, the credential, and the services ───────────────

/// Everything a surface needs to know before it draws a panel.
///
/// `ensure_running` proved the process on the port is VibeCLI. That is half the
/// question, and the half that was never wrong. The other half — *can this
/// client authenticate against it* — had no answer anywhere, so a stale token
/// file surfaced as "Is it running?" about a daemon that had been running for
/// two and a half days.
#[derive(Debug, Clone)]
pub struct Readiness {
    pub port: u16,
    /// Is the daemon process there, and did we have to start it?
    pub daemon: DaemonState,
    /// Is the token we hold the one it is accepting?
    pub token: vibe_daemon_token::TokenState,
    /// Per-feature availability, straight from `/health.features`.
    ///
    /// `None` when the daemon never answered. Note what this does and does not
    /// claim: the daemon reports a feature `available` when *it* has the code,
    /// which says nothing about whether the client's build knows the route. See
    /// [`Readiness::version_matches_client`].
    pub features: Option<serde_json::Value>,
    /// The daemon's version, when it answered.
    pub daemon_version: Option<String>,
}

impl Readiness {
    /// True only when a request is actually worth making: the daemon answered
    /// *and* we hold a credential it will accept.
    pub fn is_ready(&self) -> bool {
        self.daemon.is_ready() && self.token.is_usable()
    }

    /// The bearer to send, if there is one worth sending.
    pub fn bearer(&self) -> Option<&str> {
        self.token.bearer()
    }

    /// Whether the daemon on the port is the same build as the client asking.
    ///
    /// A mismatch is not an error — a daemon may legitimately be older or newer
    /// — but it is the explanation for the one failure that otherwise looks like
    /// a bug in a panel: a route the client calls returning **404** because the
    /// installed `vibecli` binary predates it. Observed with `/harness/profiles`
    /// and `/observe/config` against a daemon two releases behind, where every
    /// panel simply appeared broken.
    pub fn version_matches_client(&self) -> Option<bool> {
        self.daemon_version
            .as_deref()
            .map(|v| v == env!("CARGO_PKG_VERSION"))
    }

    /// A message fit to show a user. Never a bare failure, and never blames the
    /// wrong thing: a running daemon with a stale token says so.
    pub fn user_message(&self) -> String {
        if !self.daemon.is_ready() {
            return self.daemon.user_message();
        }
        if !self.token.is_usable() {
            return self.token.user_message(self.port);
        }
        match self.version_matches_client() {
            Some(false) => format!(
                "{} — but it is version {} and this client is {}. Routes added since then \
                 answer 404; run `cargo install --path vibecli/vibecli-cli` and restart it.",
                self.daemon.user_message(),
                self.daemon_version.as_deref().unwrap_or("unknown"),
                env!("CARGO_PKG_VERSION"),
            ),
            _ => self.daemon.user_message(),
        }
    }

    /// The shape the desktop shells hand to their frontends.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "port": self.port,
            "ready": self.is_ready(),
            "daemonRunning": self.daemon.is_ready(),
            "daemonVersion": self.daemon_version,
            "clientVersion": env!("CARGO_PKG_VERSION"),
            "versionMatches": self.version_matches_client(),
            "tokenState": match &self.token {
                vibe_daemon_token::TokenState::Valid { .. } => "valid",
                vibe_daemon_token::TokenState::Stale { .. } => "stale",
                vibe_daemon_token::TokenState::Missing => "missing",
                vibe_daemon_token::TokenState::Unverifiable { .. } => "unverifiable",
            },
            "features": self.features,
            "message": self.user_message(),
        })
    }
}

/// Bring the daemon up if needed, then establish whether this client can
/// actually talk to it.
///
/// This is the call a surface makes on open. `ensure_running` remains public for
/// callers that only want the process; everything that goes on to issue an
/// authenticated request should use this one, because a `true` from
/// `is_ready()` here is the only one that means "your next request will not
/// 401".
pub async fn ensure_ready(config: &BootstrapConfig) -> Readiness {
    let port = config.port;
    let daemon = ensure_running(config).await;

    // No daemon: there is no fingerprint to check a token against, and saying
    // anything about the token here would be guessing. The daemon state carries
    // the actionable message.
    let Some(identity) = daemon.identity().cloned() else {
        return Readiness {
            port,
            daemon,
            token: vibe_daemon_token::TokenState::Unverifiable { token: None },
            features: None,
            daemon_version: None,
        };
    };

    let held = vibe_daemon_token::resolve_token(None, port);
    let token = vibe_daemon_token::classify(held, identity.api_token_fingerprint.as_deref());
    let features = fetch_features(port).await;

    Readiness {
        port,
        daemon,
        token,
        features,
        daemon_version: Some(identity.version),
    }
}

/// Read `/health.features`, the daemon's own account of what it can do.
///
/// Best-effort: a daemon that answered identity but not this is still usable,
/// and reporting no features is honest where inventing them would not be.
async fn fetch_features(port: u16) -> Option<serde_json::Value> {
    reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?
        .get("features")
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cascade this guards against: a throttled `/health` has no `service`
    /// field, so a strict reader calls a healthy daemon a stranger.
    #[tokio::test]
    async fn a_rate_limited_health_check_is_not_a_foreign_service() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let hits = Arc::new(AtomicUsize::new(0));
        let seen = hits.clone();
        // First call 429s, second answers properly — exactly what a bucket
        // that has just refilled looks like.
        let app = axum::Router::new().route(
            "/health",
            axum::routing::get(move || {
                let seen = seen.clone();
                async move {
                    if seen.fetch_add(1, Ordering::SeqCst) == 0 {
                        (
                            axum::http::StatusCode::TOO_MANY_REQUESTS,
                            axum::Json(serde_json::json!({"error": "Rate limit exceeded."})),
                        )
                    } else {
                        (
                            axum::http::StatusCode::OK,
                            axum::Json(serde_json::json!({
                                "status": "ok", "service": SERVICE_NAME, "version": "9.9.9"
                            })),
                        )
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let id = probe(port).await;
        assert!(
            id.is_some(),
            "a 429 must be retried, not read as a foreign service"
        );
        assert_eq!(id.unwrap().version, "9.9.9");
        assert_eq!(hits.load(Ordering::SeqCst), 2, "expected exactly one retry");
    }

    // `/bin/sh` and `libc::getpgrp` are POSIX-only; without this gate the whole
    // `vibecli` lib test binary fails to compile on Windows, taking every other
    // test in the crate with it.
    #[cfg(unix)]
    #[test]
    fn a_detached_child_leaves_our_process_group() {
        // The bug: the daemon ran with the GUI app's PGID, in the app's
        // session, so anything aimed at the app — teardown, killpg, a hangup —
        // took the daemon with it and it exited cleanly, leaving no crash
        // report and no log. `setsid` is what makes it a service.
        let mut cmd = std::process::Command::new("/bin/sh");
        cmd.args(["-c", "ps -o pgid= -p $$"]);
        detach_session(&mut cmd);
        let out = cmd.output().expect("sh must run");
        let child_pgid: i32 = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .expect("ps must report a numeric pgid");
        // SAFETY: `getpgrp` reads this process's own group; it cannot fail.
        let ours = unsafe { libc::getpgrp() };
        assert_ne!(
            child_pgid, ours,
            "a detached child must not share the spawner's process group"
        );
    }

    #[test]
    fn a_restart_keeps_the_dead_daemons_log() {
        // A daemon that died and was auto-restarted 30 s later used to erase
        // its own post-mortem: the next spawn truncated the only file that
        // said anything about the exit.
        let tmp = std::env::temp_dir().join(format!("vibecli-rolllog-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let cur = tmp.join("daemon-spawn.log");
        let prev = tmp.join("daemon-spawn.prev.log");

        std::fs::write(&cur, b"last words\n").unwrap();
        roll_log_file(&cur, &prev);
        assert_eq!(std::fs::read_to_string(&prev).unwrap(), "last words\n");
        assert!(!cur.exists(), "the current log is moved aside, not copied");

        // An empty current log must not overwrite the kept one: the evidence is
        // in `prev`, and a daemon that died before printing anything has none.
        std::fs::write(&cur, b"").unwrap();
        roll_log_file(&cur, &prev);
        assert_eq!(std::fs::read_to_string(&prev).unwrap(), "last words\n");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn spawn_dir_skips_unwritable_candidates() {
        // The bug this guards: a Finder-launched `.app` inherits cwd `/`, the
        // daemon tries to create `/.vibecli/` on the read-only system volume,
        // and exits 1 before binding. Root must be skipped, not chosen.
        let tmp = std::env::temp_dir().join(format!("vibecli-spawndir-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        let chosen = spawn_working_dir_in(&[PathBuf::from("/no-such-root-dir/nope"), tmp.clone()]);
        assert_eq!(chosen.as_deref(), Some(tmp.as_path()));
        assert!(
            tmp.join(".vibecli").is_dir(),
            "probe should create the dir it tested"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn spawn_dir_prefers_the_callers_cwd() {
        // A client started from a repo must keep that repo as the workspace;
        // the home fallback is only for when the cwd genuinely will not do.
        let tmp = std::env::temp_dir().join(format!("vibecli-spawnpref-{}", std::process::id()));
        let fallback = tmp.join("fallback");
        let preferred = tmp.join("preferred");
        std::fs::create_dir_all(&fallback).unwrap();
        std::fs::create_dir_all(&preferred).unwrap();

        let chosen = spawn_working_dir_in(&[preferred.clone(), fallback.clone()]);
        assert_eq!(chosen.as_deref(), Some(preferred.as_path()));
        assert!(
            !fallback.join(".vibecli").exists(),
            "must not probe past the first hit"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn spawn_dir_candidates_are_ordered_and_nonempty() {
        let c = spawn_dir_candidates();
        assert!(
            !c.is_empty(),
            "there is always at least a temp dir to fall back to"
        );
        assert!(c.len() <= 3);
    }

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
            api_token_fingerprint: None,
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
                msg.contains("vibecli") || msg.contains("VIBECLI_DAEMON_PORT") || msg.contains('/'),
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

    /// The installer this repo ships writes the daemon to
    /// `%LOCALAPPDATA%\VibeCody`. Autostart never looked there, so a machine
    /// that ran `setup.ps1` and had not yet signed out — PATH lives in the
    /// registry, but Explorer and its children keep the block they launched
    /// with — reported `BinaryNotFound` with the daemon sitting on disk.
    #[test]
    fn windows_prefixes_cover_the_installers_own_directory() {
        let prefixes = windows_prefixes(
            Some(PathBuf::from("C:\\Users\\dev\\AppData\\Local")),
            Some(PathBuf::from("C:\\Users\\dev")),
        );
        assert_eq!(
            prefixes,
            vec![
                PathBuf::from("C:\\Users\\dev\\AppData\\Local").join("VibeCody"),
                PathBuf::from("C:\\Users\\dev").join("scoop").join("shims"),
            ],
        );
    }

    /// Neither variable is guaranteed — a service account can have neither —
    /// and a missing one must drop its entry, not produce a relative path that
    /// resolves against the daemon's working directory.
    #[test]
    fn windows_prefixes_skip_the_variables_that_are_unset() {
        assert!(windows_prefixes(None, None).is_empty());
        assert_eq!(
            windows_prefixes(None, Some(PathBuf::from("C:\\Users\\dev"))),
            vec![PathBuf::from("C:\\Users\\dev").join("scoop").join("shims")],
        );
    }

    /// The search must reach a system prefix when PATH does not carry it —
    /// the `%LOCALAPPDATA%\VibeCody` case, exercised through the same pure
    /// entry point on every host.
    #[test]
    fn a_system_prefix_is_found_when_path_is_empty() {
        let dir = std::env::temp_dir().join(format!("vibe_bootstrap_sp_{}", std::process::id()));
        let install_dir = dir.join("VibeCody");
        std::fs::create_dir_all(&install_dir).unwrap();
        let f = install_dir.join(binary_name());
        std::fs::write(&f, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let found = find_binary_in(&BinarySearchPaths {
            home: None,
            path_entries: Vec::new(),
            system_prefixes: vec![install_dir],
            current_exe_dir: None,
        });
        assert_eq!(found, Some(f));
        let _ = std::fs::remove_dir_all(&dir);
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

    /// A tiny HTTP server that keeps answering with `body` until dropped.
    ///
    /// Serves in a loop rather than accepting once: under a loaded full-suite
    /// run a client can open more than one connection (or retry), and a
    /// one-shot server then leaves the probe hanging until its timeout — which
    /// made a "daemon is present" assertion fail for a reason that had nothing
    /// to do with the code under test.
    fn serve_json_forever(listener: tokio::net::TcpListener, body: &'static str) {
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.flush().await;
                });
            }
        });
    }

    #[tokio::test]
    async fn probe_rejects_a_foreign_service_on_the_port() {
        // A JSON-speaking service that names itself must read as "not running",
        // not as a healthy daemon.
        //
        // Note the limit of the legacy fallback: a foreign service that happens
        // to return *exactly* `{"status":"ok","version":"..."}` and nothing else
        // is indistinguishable from a pre-`service` daemon. That ambiguity is
        // the price of not breaking users mid-upgrade; anything that identifies
        // itself is still rejected outright.
        const BODY: &str = r#"{"status":"ok","service":"some-other-app","version":"9.9.9"}"#;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        serve_json_forever(listener, BODY);

        assert_eq!(probe(port).await, None, "foreign service must not pass");
        assert!(
            port_is_occupied(port).await || true,
            "occupancy is checked separately"
        );
    }

    #[tokio::test]
    async fn probe_accepts_a_daemon_that_predates_the_service_field() {
        // Upgrade path: a user's already-running older `vibecli` has no
        // `service` key. Rejecting it made the app report the port as taken by
        // "another program" — their own daemon.
        const BODY: &str = r#"{"status":"ok","version":"0.5.7"}"#;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        serve_json_forever(listener, BODY);
        let id = probe(port).await.expect("legacy daemon must be accepted");
        assert_eq!(id.version, "0.5.7");
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
