//! End-to-end: a sandboxed shell does the IMDSv2 dance through the
//! broker's IMDS faker via env-var redirection. This is the slice
//! that closes the loop for AWS SDKs running inside the sandbox with
//! no env-var creds and no ~/.aws/credentials.

#[cfg(target_os = "macos")]
mod e2e {
    use cucumber::{World, given, then, when};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::runtime::Runtime;
    use vibe_broker::{
        AwsCredentials, ImdsHandle, ImdsServer, InMemorySecretStore, SecretStore,
        policy::SecretRef,
    };
    use vibe_sandbox::Sandbox;
    use vibe_sandbox_native::macos::MacosSandbox;

    #[derive(Default, World)]
    pub struct EWorld {
        rt: Option<Arc<Runtime>>,
        sandbox: Option<MacosSandbox>,
        rw_dir: Option<TempDir>,
        imds_handle: Option<ImdsHandle>,
        imds_addr: Option<std::net::SocketAddr>,
        captured_body: String,
    }

    impl std::fmt::Debug for EWorld {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("EWorld")
                .field("imds_addr", &self.imds_addr)
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

    #[given(expr = "a running IMDS faker on a loopback address with role {string}")]
    fn boot_imds(world: &mut EWorld, role: String) {
        let key = "@workspace.aws_default".to_string();
        let secrets = Arc::new(InMemorySecretStore::new());
        secrets.set_aws(
            key.clone(),
            AwsCredentials {
                access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
                secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
                session_token: Some("FwoGZXIvYXdzELP".into()),
                region: "us-east-1".into(),
                service: "s3".into(),
            },
        );
        let store = secrets.clone() as Arc<dyn SecretStore>;
        let server = ImdsServer::new(role, SecretRef(key), store);
        let rt = world.rt();
        let handle = rt.block_on(async move { server.start("127.0.0.1:0").await.unwrap() });
        world.imds_addr = Some(handle.addr);
        world.imds_handle = Some(handle);
    }

    #[given("the sandbox profile permits outbound TCP to the IMDS port")]
    fn allow_imds_port(world: &mut EWorld) {
        let port = world.imds_addr.unwrap().port();
        world.sandbox.as_mut().unwrap().allow_loopback_tcp(port);
    }

    #[given("the sandbox env exposes AWS_EC2_METADATA_SERVICE_ENDPOINT pointing at the IMDS faker")]
    fn set_env(world: &mut EWorld) {
        let addr = world.imds_addr.unwrap();
        let url = format!("http://{}:{}/", addr.ip(), addr.port());
        world.sandbox.as_mut().unwrap().set_env(
            "AWS_EC2_METADATA_SERVICE_ENDPOINT",
            url,
        );
    }

    #[when("the sandbox runs the AWS IMDSv2 dance via curl")]
    fn run_imds_dance(world: &mut EWorld) {
        // Two curls: first PUT for the token, then GET creds with the token.
        // The shell pipeline captures both stdout streams to /work/out.txt.
        let work = std::fs::canonicalize(world.rw_dir.as_ref().unwrap().path()).unwrap();
        let cmd = format!(
            r#"
TOKEN=$(curl -s -X PUT \
  -H "x-aws-ec2-metadata-token-ttl-seconds: 21600" \
  "${{AWS_EC2_METADATA_SERVICE_ENDPOINT}}latest/api/token")
curl -s \
  -H "x-aws-ec2-metadata-token: ${{TOKEN}}" \
  "${{AWS_EC2_METADATA_SERVICE_ENDPOINT}}latest/meta-data/iam/security-credentials/vibe-broker-role" \
  > {work}/out.txt
cat {work}/out.txt
"#,
            work = work.display()
        );
        let sb = world.sandbox.as_ref().unwrap();
        let out = sb.run_capture("/bin/sh", &["-c", &cmd]).unwrap();
        if !out.status.success() {
            panic!(
                "sandbox imds dance failed: stderr={:?}\nstdout={:?}",
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            );
        }
        world.captured_body = String::from_utf8_lossy(&out.stdout).into_owned();
    }

    #[then(expr = "the captured response body contains {string}")]
    fn body_contains(world: &mut EWorld, needle: String) {
        assert!(
            world.captured_body.contains(&needle),
            "body was: {}",
            world.captured_body
        );
    }

    pub fn run() {
        futures::executor::block_on(EWorld::run("tests/features/e2e_sandbox_imds.feature"));
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        e2e::run();
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("e2e_sandbox_imds_bdd: skipped (not macOS)");
    }
}
