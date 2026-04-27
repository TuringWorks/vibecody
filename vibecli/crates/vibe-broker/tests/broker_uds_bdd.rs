//! BDD: broker bound on a Unix domain socket.

#[cfg(unix)]
mod uds_run {
    use cucumber::{World, given, then, when};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;
    use tokio::runtime::Runtime;
    use vibe_broker::{BoundAddr, Broker, BrokerHandle, Policy, SsrfGuard, policy::DefaultRule};

    #[derive(Default, World)]
    pub struct UWorld {
        rt: Option<Arc<Runtime>>,
        sock_dir: Option<TempDir>,
        sock_path: Option<PathBuf>,
        broker_handle: Option<BrokerHandle>,
        response_status: Option<u16>,
        response_headers: Vec<(String, String)>,
        raw_response: Vec<u8>,
    }

    impl std::fmt::Debug for UWorld {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("UWorld")
                .field("sock_path", &self.sock_path)
                .finish()
        }
    }

    impl UWorld {
        fn rt(&mut self) -> Arc<Runtime> {
            if self.rt.is_none() {
                self.rt = Some(Arc::new(Runtime::new().unwrap()));
            }
            self.rt.as_ref().unwrap().clone()
        }
    }

    fn fresh_path() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broker.sock");
        (dir, path)
    }

    fn install_uds(world: &mut UWorld, broker: Broker) {
        let (dir, path) = fresh_path();
        world.sock_dir = Some(dir);
        world.sock_path = Some(path.clone());
        let rt = world.rt();
        let handle = rt.block_on(async move { broker.start_uds(&path).await.unwrap() });
        match handle.addr.clone() {
            BoundAddr::Unix(_) => {}
            other => panic!("expected Unix, got {other:?}"),
        }
        world.broker_handle = Some(handle);
    }

    #[given("a running broker on a UDS path with empty policy")]
    fn empty_policy(world: &mut UWorld) {
        let policy = Policy {
            default: DefaultRule::Deny,
            rule: vec![],
        };
        let broker = Broker::new(policy, SsrfGuard::new());
        install_uds(world, broker);
    }

    #[given(expr = "a running broker on a UDS path with a rule allowing {string} methods {string}")]
    fn one_rule(world: &mut UWorld, host: String, methods: String) {
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
        let policy = Policy::parse_toml(&toml).unwrap();
        let broker = Broker::new(policy, SsrfGuard::new());
        install_uds(world, broker);
    }

    #[when(expr = "I send {string} through the UDS broker")]
    fn send(world: &mut UWorld, req: String) {
        let mut parts = req.splitn(2, ' ');
        let method = parts.next().unwrap().to_owned();
        let url = parts.next().unwrap().to_owned();
        let parsed = url::Url::parse(&url).unwrap();
        let host = parsed.host_str().unwrap().to_owned();
        let path_q = format!(
            "{}{}",
            parsed.path(),
            parsed.query().map(|q| format!("?{q}")).unwrap_or_default()
        );
        let raw = format!(
            "{method} {path_q} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        );
        let sock = world.sock_path.clone().unwrap();
        let rt = world.rt();
        let resp = rt.block_on(async move {
            let mut stream = UnixStream::connect(&sock).await.unwrap();
            stream.write_all(raw.as_bytes()).await.unwrap();
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await.unwrap();
            buf
        });
        parse_into(world, &resp);
    }

    fn parse_into(world: &mut UWorld, resp: &[u8]) {
        let split = resp
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .unwrap_or(resp.len());
        let head = String::from_utf8_lossy(&resp[..split]);
        let mut lines = head.split("\r\n");
        if let Some(status_line) = lines.next() {
            let parts: Vec<_> = status_line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                world.response_status = parts[1].parse().ok();
            }
        }
        world.response_headers.clear();
        for line in lines {
            if line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                world
                    .response_headers
                    .push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
            }
        }
        world.raw_response = resp.to_vec();
    }

    #[when("I shut the broker down")]
    fn shutdown(world: &mut UWorld) {
        let handle = world.broker_handle.take().unwrap();
        handle.abort();
        // Drop runs the UdsCleanup, removing the socket file.
        drop(handle);
    }

    #[then(expr = "the UDS broker response status is {int}")]
    fn status_is(world: &mut UWorld, expected: u16) {
        assert_eq!(world.response_status, Some(expected),
            "headers: {:?}", world.response_headers);
    }

    #[then(expr = "the UDS broker response header {string} is {string}")]
    fn header_is(world: &mut UWorld, name: String, value: String) {
        let lower = name.to_ascii_lowercase();
        let actual = world
            .response_headers
            .iter()
            .find(|(n, _)| n == &lower)
            .map(|(_, v)| v.as_str());
        assert_eq!(actual, Some(value.as_str()), "headers: {:?}", world.response_headers);
    }

    #[then("the UDS path no longer exists")]
    fn path_gone(world: &mut UWorld) {
        let path = world.sock_path.clone().unwrap();
        // Give the OS a beat to settle the unlink.
        for _ in 0..20 {
            if !path.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("UDS path {} still exists after shutdown", path.display());
    }

    pub fn run() {
        futures::executor::block_on(UWorld::run("tests/features/broker_uds.feature"));
    }
}

fn main() {
    #[cfg(unix)]
    {
        uds_run::run();
    }
    #[cfg(not(unix))]
    {
        eprintln!("broker_uds_bdd: skipped (not Unix)");
    }
}
