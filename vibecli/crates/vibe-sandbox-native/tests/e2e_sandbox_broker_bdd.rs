//! End-to-end: sandboxed /bin/sh runs `curl --unix-socket` through a real
//! broker and gets policy-enforced 200 / 451 responses.
//!
//! macOS only — exercises the real `sandbox-exec` host plus a real
//! `tokio::net::UnixListener` broker. Falls through cleanly elsewhere.

#[cfg(target_os = "macos")]
mod e2e {
    use cucumber::{World, given, then, when};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::runtime::Runtime;
    use vibe_broker::{Broker, BrokerHandle, Policy, SsrfGuard, policy::DefaultRule};
    use vibe_sandbox::Sandbox;
    use vibe_sandbox_native::macos::MacosSandbox;

    #[derive(Default, World)]
    pub struct EWorld {
        rt: Option<Arc<Runtime>>,
        sandbox: Option<MacosSandbox>,
        rw_dir: Option<TempDir>,
        sock_dir: Option<TempDir>,
        sock_path: Option<PathBuf>,
        broker: Option<BrokerHandle>,
        captured_status: Option<u16>,
        captured_reason: Option<String>,
    }

    impl std::fmt::Debug for EWorld {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("EWorld")
                .field("sock_path", &self.sock_path)
                .field("status", &self.captured_status)
                .finish()
        }
    }

    impl EWorld {
        fn rt(&mut self) -> Arc<Runtime> {
            if self.rt.is_none() {
                self.rt = Some(Arc::new(Runtime::new().unwrap()));
            }
            self.rt.as_ref().unwrap().clone()
        }
    }

    #[given("a fresh macOS sandbox with a bound rw temp dir")]
    fn fresh_sandbox(world: &mut EWorld) {
        let mut sb = MacosSandbox::new().unwrap();
        let td = tempfile::tempdir().unwrap();
        sb.bind_rw(td.path(), &PathBuf::from("/work")).unwrap();
        world.rw_dir = Some(td);
        world.sandbox = Some(sb);
    }

    fn boot_broker(world: &mut EWorld, policy: Policy) {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("broker.sock");
        let rt = world.rt();
        let path_for_async = sock.clone();
        let handle = rt.block_on(async move {
            Broker::new(policy, SsrfGuard::new())
                .start_uds(&path_for_async)
                .await
                .unwrap()
        });
        world.sock_dir = Some(dir);
        world.sock_path = Some(sock);
        world.broker = Some(handle);
    }

    #[given("a running broker on a UDS path with empty policy")]
    fn empty_broker(world: &mut EWorld) {
        boot_broker(
            world,
            Policy {
                default: DefaultRule::Deny,
                rule: vec![],
            },
        );
    }

    #[given(expr = "a running broker on a UDS path with a rule allowing {string} methods {string}")]
    fn one_rule_broker(world: &mut EWorld, host: String, methods: String) {
        let toml = format!(
            r#"
default = "deny"

[[rule]]
match.host = "{host}"
match.methods = [{}]
match.require_tls = false
"#,
            methods
                .split(',')
                .map(|m| format!("\"{}\"", m.trim()))
                .collect::<Vec<_>>()
                .join(", "),
        );
        boot_broker(world, Policy::parse_toml(&toml).unwrap());
    }

    #[given("the sandbox profile permits outbound to the broker UDS")]
    fn allow_broker_socket(world: &mut EWorld) {
        let sock = world.sock_path.clone().unwrap();
        let canon = std::fs::canonicalize(&sock).unwrap();
        let sb = world.sandbox.as_mut().unwrap();
        sb.network(vibe_sandbox::NetPolicy::Brokered {
            socket: canon,
            policy_id: "test:e2e".into(),
        });
    }

    #[when(expr = "the sandbox runs curl with --unix-socket pointing at the broker, target {string}")]
    fn run_curl(world: &mut EWorld, target: String) {
        let mut parts = target.splitn(2, ' ');
        let method = parts.next().unwrap();
        let url = parts.next().unwrap();
        let canon_sock = std::fs::canonicalize(world.sock_path.as_ref().unwrap()).unwrap();
        let work = std::fs::canonicalize(world.rw_dir.as_ref().unwrap().path()).unwrap();
        // -s silent, -o /dev/null, -w writes status code, -D <file> dumps
        // headers to a file we then cat. Header file lives in the bound
        // rw dir so the sandbox can write it.
        let cmd = format!(
            "curl -s -X {method} \
             --unix-socket {sock} \
             -o /dev/null \
             -D {work}/headers.txt \
             -w 'STATUS=%{{http_code}}\\n' \
             {url} ; \
             cat {work}/headers.txt",
            sock = canon_sock.display(),
            url = url,
            work = work.display(),
        );

        let sb = world.sandbox.as_ref().unwrap();
        let out = sb.run_capture("/bin/sh", &["-c", &cmd]).unwrap();
        if !out.status.success() {
            panic!(
                "sandbox curl invocation failed: stderr={:?}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        // Parse STATUS=xxx line.
        if let Some(line) = stdout.lines().find(|l| l.starts_with("STATUS=")) {
            world.captured_status = line[7..].trim().parse().ok();
        }
        // Parse X-Vibe-Broker-Reason header.
        for line in stdout.lines() {
            let lower = line.to_ascii_lowercase();
            if let Some(stripped) = lower.strip_prefix("x-vibe-broker-reason:") {
                world.captured_reason = Some(stripped.trim().to_owned());
            }
        }
    }

    #[then(expr = "the curl HTTP status code captured was {int}")]
    fn status_was(world: &mut EWorld, expected: u16) {
        assert_eq!(world.captured_status, Some(expected),
            "captured_reason: {:?}", world.captured_reason);
    }

    #[then(expr = "the curl X-Vibe-Broker-Reason header captured was {string}")]
    fn reason_was(world: &mut EWorld, expected: String) {
        assert_eq!(world.captured_reason.as_deref(), Some(expected.as_str()));
    }

    pub fn run() {
        futures::executor::block_on(EWorld::run("tests/features/e2e_sandbox_broker.feature"));
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        e2e::run();
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("e2e_sandbox_broker_bdd: skipped (not macOS)");
    }
}
