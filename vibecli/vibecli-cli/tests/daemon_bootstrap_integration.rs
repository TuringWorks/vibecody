//! End-to-end check of the launch contract every desktop client depends on.
//!
//! Unit tests cover `probe`/`find_binary` against fakes. This one runs the
//! **real daemon binary** and asserts the two things an app launch actually
//! needs:
//!
//! 1. `/health` identifies itself as `service: "vibecli"`, so a client can tell
//!    the daemon from any other process that happens to hold the port.
//! 2. `ensure_running` reuses a daemon that is already up, rather than starting
//!    a second one.
//!
//! Why this matters concretely: on the machine this was written on, a cold
//! daemon took **~16 seconds** to answer `/health` (it warms a memory-health
//! cache and announces over mDNS first). The autostart path used to sleep a
//! fixed 2 s, check once, and report failure — so a perfectly healthy daemon
//! was reported as broken on every launch. Any regression that reintroduces a
//! short fixed wait fails here.
//!
//! Skipped (not failed) when the binary hasn't been built, so `cargo test`
//! still works on a fresh checkout. Run `cargo build -p vibecli --bin vibecli`
//! first to exercise it.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use vibecli_cli::daemon_bootstrap::{
    ensure_running, port_is_occupied, probe, BootstrapConfig, DaemonState, SERVICE_NAME,
};

/// Locate the freshly-built daemon in `target/{debug,release}`.
fn built_binary() -> Option<PathBuf> {
    // CARGO_MANIFEST_DIR is <repo>/vibecli/vibecli-cli
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest.parent()?.parent()?;
    ["debug", "release"]
        .iter()
        .map(|profile| repo_root.join("target").join(profile).join("vibecli"))
        .find(|p| p.is_file())
}

/// A port nothing is listening on: bind, read the assigned port, release.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local_addr").port()
}

/// Kills the daemon when the test ends, however it ends.
struct DaemonGuard(Child);

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Generous on purpose — see the module docs. A cold start is genuinely slow.
const READY_TIMEOUT: Duration = Duration::from_secs(90);

#[tokio::test(flavor = "multi_thread")]
async fn real_daemon_identifies_itself_and_is_reused() {
    let Some(binary) = built_binary() else {
        eprintln!("skipping: no built vibecli binary (run `cargo build -p vibecli --bin vibecli`)");
        return;
    };

    let port = free_port();
    assert!(
        !port_is_occupied(port).await,
        "test port {port} should start free"
    );
    assert_eq!(
        probe(port).await,
        None,
        "nothing should identify as the daemon before we start it"
    );

    let child = Command::new(&binary)
        .args(["--serve", "--port", &port.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn vibecli");
    let _guard = DaemonGuard(child);

    // Poll for readiness exactly as the app does.
    let started = Instant::now();
    let identity = loop {
        if let Some(id) = probe(port).await {
            break id;
        }
        assert!(
            started.elapsed() < READY_TIMEOUT,
            "daemon did not answer /health within {}s",
            READY_TIMEOUT.as_secs()
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    };
    eprintln!(
        "daemon {} ready in {:?} — the old autostart gave up after 2s",
        identity.version,
        started.elapsed()
    );

    assert!(
        !identity.version.is_empty() && identity.version != "unknown",
        "/health must report a real version, got {:?}",
        identity.version
    );

    // The contract clients match on.
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/health"))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("health request")
        .json()
        .await
        .expect("health json");
    assert_eq!(
        body.get("service").and_then(|v| v.as_str()),
        Some(SERVICE_NAME),
        "/health must identify the service so clients can reject impostors"
    );

    // A running daemon must be reused, never duplicated — it is shared infra
    // for the mobile / watch / IDE clients.
    let state = ensure_running(&BootstrapConfig {
        port,
        startup_timeout: Duration::from_secs(5),
    })
    .await;
    match state {
        DaemonState::AlreadyRunning(id) => assert_eq!(id.version, identity.version),
        other => panic!(
            "expected AlreadyRunning, got {other:?} ({})",
            other.user_message()
        ),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_foreign_listener_is_never_mistaken_for_the_daemon() {
    // The failure this prevents: an unrelated local service holds the daemon
    // port, every client reads it as "daemon online", and every subsequent call
    // fails with an error that points at the daemon instead of the conflict.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();

    assert!(
        port_is_occupied(port).await,
        "listener should hold the port"
    );
    assert_eq!(probe(port).await, None, "a raw listener is not the daemon");

    let state = ensure_running(&BootstrapConfig {
        port,
        startup_timeout: Duration::from_millis(500),
    })
    .await;
    assert_eq!(state, DaemonState::PortTakenByOther { port });
    assert!(!state.is_ready());
    // The message has to name the conflict and the way out.
    let msg = state.user_message();
    assert!(
        msg.contains(&port.to_string()),
        "message must name the port: {msg}"
    );
    assert!(
        msg.contains("VIBECLI_DAEMON_PORT"),
        "message must offer the override: {msg}"
    );
}
