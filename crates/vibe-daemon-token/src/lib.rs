//! Where the VibeCLI daemon's bearer token lives — and whether the one you
//! hold is the one the *live* daemon minted.
//!
//! # The bug this crate exists to make impossible
//!
//! `vibecli serve` mints a fresh random token on every start and writes it to
//! `~/.vibecli/daemon.token`. That path names no port, and the daemon can run on
//! any port. So two daemons on **different** ports both wrote the same file, and
//! the last writer won regardless of which one was still alive:
//!
//! ```text
//!   Aug 30 21:35  daemon A binds 7878, writes daemon.token = f392…74bf
//!   Sep  1 20:35  daemon B binds 7979, writes daemon.token = 1b7d…7575
//!   Sep  1 20:37  daemon B exits (SIGTERM)
//!   → A is healthy, listening, and every client on the machine 401s
//!     against it forever. Nothing rewrites the file until A restarts.
//! ```
//!
//! Observed exactly that, for two days. `serve.rs` had already been taught to
//! bind before writing, which closes the case where a *same-port* daemon loses
//! the race and clobbers the winner on its way out — but daemon B did not lose
//! any race. It bound a free port and wrote a shared file it had every reason to
//! believe was its own.
//!
//! The fix is that the file is named after the port: [`token_path`] is
//! `daemon-<port>.token`, so two daemons cannot collide. `daemon.token` is still
//! written — by the daemon whose port is the one clients resolve by default, and
//! only by that one — because a dozen readers across four languages still look
//! for it.
//!
//! # The second half: a token you cannot check is a token you cannot trust
//!
//! Reading a file tells you a token exists, not that it works. `/health` exposes
//! [`fingerprint`] of the live token — a SHA-256 prefix, which reveals nothing
//! about a 128-bit random secret — so a client can [`classify`] what it holds
//! *before* issuing a request, and say "your token file belongs to a daemon that
//! is no longer running" instead of "is the daemon running?" about a daemon that
//! plainly is.
//!
//! # Why its own crate
//!
//! `vibe-eval` and `vibe-desktop-voice` both authenticate against the daemon and
//! both have zero internal dependencies — `vibe-eval` is *depended on* by
//! `vibecli`, so it cannot depend back. Putting this in `vibecli-cli` would have
//! meant a fourth and fifth copy of the same path join, which is how there came
//! to be ten.

use std::path::{Path, PathBuf};

/// Default daemon port. Every client agrees on it through this constant.
pub const DEFAULT_PORT: u16 = 7878;

/// Environment variable naming the daemon port, and its VibeDesk-era alias.
///
/// Public so a caller can name them in an error message rather than restating
/// the strings and drifting from what is actually read.
pub const PORT_ENV: &str = "VIBECLI_DAEMON_PORT";
/// Legacy alias for [`PORT_ENV`], honoured for VibeDesk compatibility.
pub const PORT_ENV_LEGACY: &str = "VIBEDESK_DAEMON_PORT";

/// Environment variable holding an explicit bearer token.
pub const TOKEN_ENV: &str = "VIBECLI_TOKEN";

/// The port clients on this machine resolve to, absent an explicit override.
///
/// This is also the port whose daemon owns the legacy `daemon.token` file — see
/// [`write_for_daemon`]. The two answers must come from one place: a daemon that
/// disagreed with its clients about which port is "default" would either write a
/// file nobody reads or clobber one that is in use.
pub fn default_port() -> u16 {
    std::env::var(PORT_ENV)
        .or_else(|_| std::env::var(PORT_ENV_LEGACY))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// `~/.vibecli`, or `None` when the home directory cannot be determined.
pub fn vibecli_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".vibecli"))
}

/// The token file owned by a daemon on `port`, under `dir`.
///
/// Pure in its directory so the layout can be tested without touching the
/// developer's real `~/.vibecli` — the store-isolation rule in AGENTS.md.
pub fn token_path_in(dir: &Path, port: u16) -> PathBuf {
    dir.join(format!("daemon-{port}.token"))
}

/// The port-agnostic file every pre-existing reader looks for.
pub fn legacy_token_path_in(dir: &Path) -> PathBuf {
    dir.join("daemon.token")
}

/// The token file owned by a daemon on `port`.
pub fn token_path(port: u16) -> Option<PathBuf> {
    vibecli_dir().map(|d| token_path_in(&d, port))
}

/// The port-agnostic `~/.vibecli/daemon.token`.
pub fn legacy_token_path() -> Option<PathBuf> {
    vibecli_dir().map(|d| legacy_token_path_in(&d))
}

/// A non-secret identifier for a token: the first 16 hex characters of its
/// SHA-256.
///
/// Safe to publish on the unauthenticated `/health` route. The token is 128 bits
/// of CSPRNG output, so there is no dictionary to run a preimage search against
/// and 64 bits of digest narrows nothing an attacker could then guess. It exists
/// so a client can compare what it read from disk against what the daemon is
/// actually accepting, which is the difference between "restart your daemon" and
/// two days of unexplained 401s.
pub fn fingerprint(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(&digest[..8])
}

/// Read a token file, treating an empty or whitespace-only file as absent.
///
/// A zero-byte `daemon.token` is what a killed daemon leaves behind mid-write,
/// and an empty bearer is not a credential — returning `Some("")` would send an
/// `Authorization: Bearer ` header and turn a missing token into a puzzling 401.
fn read_file(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// The token a client talking to `port` should use, read from `dir`.
///
/// Port-specific file first, then the legacy shared one. The order matters:
/// during the transition a machine has both, and only the port-specific file is
/// guaranteed to belong to the daemon on the port being addressed.
pub fn read_token_in(dir: &Path, port: u16) -> Option<String> {
    read_file(&token_path_in(dir, port)).or_else(|| read_file(&legacy_token_path_in(dir)))
}

/// The token a client talking to `port` should use.
pub fn read_token(port: u16) -> Option<String> {
    vibecli_dir().and_then(|d| read_token_in(&d, port))
}

/// Full precedence for a client's bearer: explicit argument, then
/// `VIBECLI_TOKEN`, then the files.
///
/// The explicit-first order is right for a *first* attempt and wrong for a
/// retry — a stale `VIBECLI_TOKEN` outranking the file is usually the reason the
/// first attempt 401'd. Use [`read_token`] directly for the retry; see
/// `fresher_token` in `vibe-desktop-voice`.
pub fn resolve_token(explicit: Option<&str>, port: u16) -> Option<String> {
    explicit
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .or_else(|| {
            std::env::var(TOKEN_ENV)
                .ok()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
        })
        .or_else(|| read_token(port))
}

/// Which files a daemon on `port` owns, given the port clients resolve by
/// default.
///
/// Always its own `daemon-<port>.token`. Additionally `daemon.token` **only**
/// when it is the daemon clients will look for by default — that single
/// condition is the whole fix: a daemon on a non-default port no longer touches
/// the file the default-port daemon's clients are reading.
///
/// Pure, and separate from the writing, so the rule is testable without a
/// filesystem or an environment variable.
pub fn files_owned_by(dir: &Path, port: u16, clients_default_port: u16) -> Vec<PathBuf> {
    let mut paths = vec![token_path_in(dir, port)];
    if port == clients_default_port {
        paths.push(legacy_token_path_in(dir));
    }
    paths
}

/// Write the token files a daemon on `port` owns, each mode 0600 on Unix.
///
/// Returns the paths written, so the daemon can name them in its startup banner
/// rather than hard-coding a list that drifts from what it did.
///
/// Takes `clients_default_port` rather than calling [`default_port`] itself:
/// reading the environment in here would make every test of the ownership rule
/// depend on a variable the developer's shell might export.
///
/// A failure here is worth treating as fatal by the caller: the token is never
/// printed in full and is freshly random for the process, so these files are the
/// *only* way any client can learn it. A daemon that starts without them is one
/// that rejects every request with nothing on screen explaining why.
pub fn write_for_daemon(
    dir: &Path,
    port: u16,
    clients_default_port: u16,
    token: &str,
) -> std::io::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let paths = files_owned_by(dir, port, clients_default_port);
    for path in &paths {
        std::fs::write(path, token)?;
        restrict(path)?;
    }
    Ok(paths)
}

/// Tighten a token file to owner-only.
///
/// A world-readable bearer token is a local privilege-escalation gift. Unlike
/// the write itself this is not fatal — a filesystem without Unix modes (a FAT
/// volume, a mounted share) cannot honour it and the daemon is still usable —
/// but the caller should say so out loud.
#[cfg(unix)]
fn restrict(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Remove the token files a daemon on `port` owns.
///
/// Called on clean shutdown. Leaving them behind is what let a two-days-dead
/// daemon's credential keep being read as current: a token file that outlives
/// its daemon is not a cache, it is a wrong answer with a plausible shape.
///
/// Best-effort by design — a daemon killed with SIGKILL never runs this, which
/// is why [`classify`] exists rather than trusting the file's presence.
pub fn remove_for_daemon(dir: &Path, port: u16, clients_default_port: u16) -> Vec<PathBuf> {
    files_owned_by(dir, port, clients_default_port)
        .into_iter()
        .filter(|p| std::fs::remove_file(p).is_ok())
        .collect()
}

/// What the token a client holds is worth against the daemon it is talking to.
///
/// Four states, kept apart on purpose. Collapsing them into a boolean is what
/// produced "Could not read speech settings from the daemon (daemon returned
/// 401). Is it running?" — asked about a daemon that had been running for two
/// and a half days.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenState {
    /// The token on disk is the one the live daemon minted.
    Valid { token: String },
    /// A token was found, and it is **not** the daemon's. Nothing the client
    /// does will authenticate; the daemon has to be restarted (or the stale
    /// file removed) before anything works.
    Stale {
        token: String,
        /// What the daemon reports it is accepting.
        want: String,
        /// What the token we found actually fingerprints to.
        have: String,
    },
    /// No token file at all. Distinct from `Stale`: the daemon may simply not
    /// have written one yet, which is a wait rather than a restart.
    Missing,
    /// The daemon did not report a fingerprint, so the token cannot be checked
    /// ahead of time. A daemon predating that field — accepted, not failed,
    /// because refusing it would tell a user upgrading from an older `vibecli`
    /// that their own daemon is broken.
    Unverifiable { token: Option<String> },
}

impl TokenState {
    /// The bearer to send, if there is one worth sending.
    ///
    /// `Stale` yields `None`: sending it earns a guaranteed 401, and the retry
    /// that 401 triggers would re-read the same wrong file and 401 again.
    pub fn bearer(&self) -> Option<&str> {
        match self {
            TokenState::Valid { token } => Some(token),
            TokenState::Unverifiable { token } => token.as_deref(),
            TokenState::Stale { .. } | TokenState::Missing => None,
        }
    }

    /// True when a request is worth making.
    pub fn is_usable(&self) -> bool {
        matches!(
            self,
            TokenState::Valid { .. } | TokenState::Unverifiable { .. }
        )
    }

    /// A message that names the fix, never a bare failure.
    pub fn user_message(&self, port: u16) -> String {
        match self {
            TokenState::Valid { .. } => "Authenticated with the VibeCLI daemon".to_string(),
            TokenState::Stale { want, have, .. } => format!(
                "The saved daemon token ({have}…) is not the one the daemon on port {port} is \
                 accepting ({want}…). Another daemon overwrote it and has since exited. Restart \
                 the daemon on port {port} — it rewrites the token on every start."
            ),
            TokenState::Missing => format!(
                "No bearer token for the VibeCLI daemon on port {port}. Start it with \
                 `vibecli --serve --port {port}`; it writes the token on start."
            ),
            TokenState::Unverifiable { token: Some(_) } => {
                "The daemon is older than this client and does not report a token fingerprint; \
                 using the saved token unchecked."
                    .to_string()
            }
            TokenState::Unverifiable { token: None } => format!(
                "No bearer token found, and this daemon is too old to say which one it wants. \
                 Restart it with `vibecli --serve --port {port}`."
            ),
        }
    }
}

/// Classify a held token against the fingerprint the daemon reports.
///
/// Pure: both inputs come from the caller, so every state is testable without a
/// daemon, a filesystem, or an environment variable.
pub fn classify(have: Option<String>, daemon_fingerprint: Option<&str>) -> TokenState {
    match (have, daemon_fingerprint) {
        (token, None) => TokenState::Unverifiable { token },
        (None, Some(_)) => TokenState::Missing,
        (Some(token), Some(want)) => {
            let have = fingerprint(&token);
            if have == want {
                TokenState::Valid { token }
            } else {
                TokenState::Stale {
                    token,
                    want: want.to_string(),
                    have,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn a_daemon_on_a_non_default_port_does_not_own_the_shared_file() {
        // The whole bug in one assertion. Daemon B on 7979 wrote
        // `daemon.token` and killed authentication for daemon A on 7878.
        let dir = Path::new("/nowhere");
        let owned = files_owned_by(dir, 7979, 7878);
        assert_eq!(owned, vec![dir.join("daemon-7979.token")]);
        assert!(
            !owned.contains(&dir.join("daemon.token")),
            "a non-default-port daemon must never touch the shared token file"
        );
    }

    #[test]
    fn the_default_port_daemon_owns_both_files() {
        // Back-compat: a dozen readers across Rust, TypeScript, Kotlin and the
        // eval harness still look for `daemon.token`.
        let dir = Path::new("/nowhere");
        assert_eq!(
            files_owned_by(dir, 7878, 7878),
            vec![dir.join("daemon-7878.token"), dir.join("daemon.token")]
        );
    }

    #[test]
    fn ownership_follows_the_configured_default_not_the_constant() {
        // A machine with VIBECLI_DAEMON_PORT=7979 has its clients resolving
        // 7979, so 7979's daemon is the one that must own `daemon.token`.
        let dir = Path::new("/nowhere");
        assert!(files_owned_by(dir, 7979, 7979).contains(&dir.join("daemon.token")));
        assert!(!files_owned_by(dir, 7878, 7979).contains(&dir.join("daemon.token")));
    }

    #[test]
    fn the_port_specific_file_outranks_the_shared_one() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("daemon-7878.token"), "mine").unwrap();
        std::fs::write(tmp.path().join("daemon.token"), "someone-elses").unwrap();
        assert_eq!(
            read_token_in(tmp.path(), 7878).as_deref(),
            Some("mine"),
            "the file named after the port is the only one known to belong to it"
        );
    }

    #[test]
    fn the_shared_file_is_the_fallback_when_no_port_file_exists() {
        // Every daemon predating this crate wrote only `daemon.token`.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("daemon.token"), "legacy").unwrap();
        assert_eq!(read_token_in(tmp.path(), 7878).as_deref(), Some("legacy"));
    }

    #[test]
    fn an_empty_token_file_is_absent_not_an_empty_bearer() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("daemon-7878.token"), "   \n").unwrap();
        assert_eq!(read_token_in(tmp.path(), 7878), None);
    }

    #[test]
    fn a_written_token_reads_back_and_is_owner_only() {
        let tmp = TempDir::new().unwrap();
        let written = write_for_daemon(tmp.path(), 7878, 7878, "abc123").unwrap();
        assert!(written.contains(&tmp.path().join("daemon-7878.token")));
        assert_eq!(read_token_in(tmp.path(), 7878).as_deref(), Some("abc123"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for p in &written {
                let mode = std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600, "{} is readable by other local users", p.display());
            }
        }
    }

    #[test]
    fn a_fingerprint_never_contains_the_token() {
        let token = "f392000000000000000000000000074bf";
        let fp = fingerprint(token);
        assert_eq!(fp.len(), 16);
        assert!(!fp.contains(&token[..8]));
        assert_ne!(fp, fingerprint("a different token"));
        // Stable across calls — a client compares it against a value the
        // daemon computed in another process.
        assert_eq!(fp, fingerprint(token));
    }

    #[test]
    fn a_matching_fingerprint_is_valid() {
        let want = fingerprint("live");
        assert_eq!(
            classify(Some("live".into()), Some(&want)),
            TokenState::Valid {
                token: "live".into()
            }
        );
    }

    #[test]
    fn a_token_from_a_dead_daemon_is_stale_not_missing() {
        // The observed failure: a token exists, is well-formed, and is wrong.
        let want = fingerprint("live");
        let state = classify(Some("from-the-dead-7979-daemon".into()), Some(&want));
        assert!(matches!(state, TokenState::Stale { .. }));
        assert!(!state.is_usable());
        assert_eq!(state.bearer(), None, "sending a stale token buys a 401");
        let msg = state.user_message(7878);
        assert!(msg.contains("Restart"), "must name the fix: {msg}");
        assert!(
            !msg.contains("is it running"),
            "the daemon is running; saying otherwise sends people to the wrong place: {msg}"
        );
    }

    #[test]
    fn a_daemon_without_a_fingerprint_is_unverifiable_not_stale() {
        // An older `vibecli` on the port is not a broken one. Refusing to use
        // its token would tell an upgrading user their own daemon is dead.
        let state = classify(Some("whatever".into()), None);
        assert_eq!(state.bearer(), Some("whatever"));
        assert!(state.is_usable());
    }

    #[test]
    fn no_token_against_a_modern_daemon_is_missing() {
        let want = fingerprint("live");
        let state = classify(None, Some(&want));
        assert_eq!(state, TokenState::Missing);
        assert!(!state.is_usable());
        assert!(state.user_message(7878).contains("vibecli --serve"));
    }

    #[test]
    fn removing_a_daemons_files_leaves_another_daemons_alone() {
        let tmp = TempDir::new().unwrap();
        write_for_daemon(tmp.path(), 7979, 7878, "b").unwrap();
        std::fs::write(tmp.path().join("daemon-7878.token"), "a").unwrap();
        remove_for_daemon(tmp.path(), 7979, 7878);
        assert_eq!(read_token_in(tmp.path(), 7878).as_deref(), Some("a"));
        assert!(!tmp.path().join("daemon-7979.token").exists());
    }
}
