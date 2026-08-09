//! The contract every client depends on, exercised against a **real daemon**.
//!
//! This exists because of a production cascade that no unit test could see and
//! a green build could not have caught. Three independently-correct-looking
//! decisions combined into a daemon that was healthy but unusable from every
//! client at once:
//!
//! ```text
//! /health shared a 10-req/min per-IP bucket with the other public routes
//!    │   three desktop apps + a 250ms startup poll all arrive from 127.0.0.1
//!    ▼
//! /health returns {"error":"Rate limit exceeded"} — no `service` field
//!    │
//!    ▼
//! probe() reads that as "answered, but not VibeCLI"
//!    │   the app tells the user "Port 7878 is in use by another program"
//!    ▼
//! the client spawns a replacement daemon
//!    │   which wrote ~/.vibecli/daemon.token BEFORE binding
//!    ▼
//! it loses the bind, exits — having clobbered the live daemon's token
//!    ▼
//! every client 401s against a daemon that was fine the whole time
//! ```
//!
//! Each assertion below pins one link in that chain. They are integration
//! tests on purpose: every link lived in the seam *between* components, where
//! unit tests with fakes agreed with each other and with nothing real.
//!
//! **`HOME` is redirected to a temp directory for every spawned daemon.** The
//! daemon writes its bearer token to `$HOME/.vibecli/daemon.token`, so a test
//! that inherits the real `HOME` overwrites the developer's own running
//! daemon's token — which is the very failure being tested for.
//!
//! Skipped (not failed) when the binary hasn't been built, so `cargo test`
//! works on a fresh checkout. Run `cargo build -p vibecli --bin vibecli` first.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use vibecli_cli::daemon_bootstrap::{port_is_occupied, probe, SERVICE_NAME};

/// A cold daemon warms caches and announces over mDNS before answering.
/// Generous on purpose; a short fixed wait here is the bug this suite guards.
const READY_TIMEOUT: Duration = Duration::from_secs(90);

fn built_binary() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest.parent()?.parent()?;
    ["debug", "release"]
        .iter()
        .map(|profile| repo_root.join("target").join(profile).join("vibecli"))
        .find(|p| p.is_file())
}

fn free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    l.local_addr().expect("local_addr").port()
}

struct DaemonGuard(Child);

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Spawn a daemon with an isolated `HOME`, so its token file is the test's own.
fn spawn(binary: &Path, port: u16, home: &Path) -> Child {
    Command::new(binary)
        .args(["--serve", "--port", &port.to_string()])
        .env("HOME", home)
        .current_dir(home) // a writable cwd; the daemon roots state off it
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn vibecli")
}

async fn wait_ready(port: u16) -> vibecli_cli::daemon_bootstrap::DaemonIdentity {
    let started = Instant::now();
    loop {
        if let Some(id) = probe(port).await {
            return id;
        }
        assert!(
            started.elapsed() < READY_TIMEOUT,
            "daemon did not answer /health within {}s",
            READY_TIMEOUT.as_secs()
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn token_path(home: &Path) -> PathBuf {
    home.join(".vibecli").join("daemon.token")
}

fn read_token(home: &Path) -> String {
    std::fs::read_to_string(token_path(home))
        .expect("daemon must write its bearer token — clients have no other way to learn it")
        .trim()
        .to_string()
}

/// Link 1 + 2: identity must survive the polling load three clients actually
/// produce, and a throttled reply must never read as a foreign service.
///
/// The old limit was 10/min shared across every public route and every client
/// on 127.0.0.1. Forty concurrent probes is what a few apps polling their
/// status dot look like, and it used to poison the identity check outright.
#[tokio::test(flavor = "multi_thread")]
async fn identity_survives_the_polling_load_real_clients_produce() {
    let Some(binary) = built_binary() else {
        eprintln!("skipping: no built vibecli binary");
        return;
    };
    let home = tempfile::tempdir().expect("temp home");
    let port = free_port();
    let _guard = DaemonGuard(spawn(&binary, port, home.path()));
    wait_ready(port).await;

    let url = format!("http://127.0.0.1:{port}/health");
    let client = reqwest::Client::new();
    let results = futures::future::join_all((0..40).map(|_| {
        let client = client.clone();
        let url = url.clone();
        async move {
            let res = client
                .get(&url)
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .expect("health request");
            let status = res.status();
            let body: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
            (
                status,
                body.get("service")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            )
        }
    }))
    .await;

    let throttled = results.iter().filter(|(s, _)| s.as_u16() == 429).count();
    let identified = results
        .iter()
        .filter(|(_, svc)| svc.as_deref() == Some(SERVICE_NAME))
        .count();

    assert_eq!(
        throttled, 0,
        "/health must not be throttled — it is the liveness AND identity probe \
         every client polls, and a 429 carries no `service` field, so throttling \
         it makes a healthy daemon look like a stranger on the port"
    );
    assert_eq!(
        identified,
        results.len(),
        "every /health reply must identify the service; got {identified}/{}",
        results.len()
    );

    // And the client-side conclusion that actually reaches the user.
    assert!(
        probe(port).await.is_some(),
        "probe() must still identify the daemon after that load"
    );
}

/// Link 3: a daemon that cannot have the port must not damage the one that has
/// it. The token file is a single shared path, so a loser that writes before
/// binding takes every client down with it.
#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_that_loses_the_port_does_not_clobber_the_live_token() {
    let Some(binary) = built_binary() else {
        eprintln!("skipping: no built vibecli binary");
        return;
    };
    let home = tempfile::tempdir().expect("temp home");
    let port = free_port();

    let _live = DaemonGuard(spawn(&binary, port, home.path()));
    wait_ready(port).await;
    let live_token = read_token(home.path());
    assert!(!live_token.is_empty());

    // A second daemon on the same port, sharing the same HOME — exactly what a
    // client does when it wrongly concludes nothing is running.
    let mut loser = spawn(&binary, port, home.path());
    let exited = {
        let started = Instant::now();
        loop {
            match loser.try_wait().expect("try_wait") {
                Some(status) => break Some(status),
                None if started.elapsed() > Duration::from_secs(30) => break None,
                None => tokio::time::sleep(Duration::from_millis(250)).await,
            }
        }
    };
    if exited.is_none() {
        let _ = loser.kill();
        let _ = loser.wait();
        panic!("second daemon should fail to bind an occupied port and exit");
    }

    assert_eq!(
        read_token(home.path()),
        live_token,
        "a daemon that lost the bind overwrote the live daemon's token; every \
         client then 401s against a healthy daemon with no way to recover"
    );

    // The token still opens a protected route — the property clients rely on.
    let res = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/jobs"))
        .header("Authorization", format!("Bearer {live_token}"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("authed request");
    assert!(
        res.status().is_success(),
        "the token file must authenticate against the running daemon, got {}",
        res.status()
    );
}

/// The auth contract itself: the token file is the only way a client learns the
/// bearer, protected routes require it, and public ones do not.
#[tokio::test(flavor = "multi_thread")]
async fn the_token_file_is_the_whole_auth_contract() {
    let Some(binary) = built_binary() else {
        eprintln!("skipping: no built vibecli binary");
        return;
    };
    let home = tempfile::tempdir().expect("temp home");
    let port = free_port();
    let _guard = DaemonGuard(spawn(&binary, port, home.path()));
    wait_ready(port).await;

    let client = reqwest::Client::new();
    let jobs = format!("http://127.0.0.1:{port}/jobs");

    let unauthed = client
        .get(&jobs)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("unauthed request");
    assert_eq!(
        unauthed.status().as_u16(),
        401,
        "a protected route must reject a request with no bearer"
    );

    let authed = client
        .get(&jobs)
        .header(
            "Authorization",
            format!("Bearer {}", read_token(home.path())),
        )
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("authed request");
    assert!(
        authed.status().is_success(),
        "the token from ~/.vibecli/daemon.token must be accepted, got {}",
        authed.status()
    );

    // `/health` is public: no bearer, and it still identifies itself.
    let health: serde_json::Value = client
        .get(format!("http://127.0.0.1:{port}/health"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("health request")
        .json()
        .await
        .expect("health json");
    assert_eq!(
        health.get("service").and_then(|v| v.as_str()),
        Some(SERVICE_NAME),
        "/health must be reachable unauthenticated and name the service"
    );

    // And a free port must not be mistaken for an occupied one.
    assert!(
        !port_is_occupied(free_port()).await,
        "an unused port must not read as occupied"
    );
}
