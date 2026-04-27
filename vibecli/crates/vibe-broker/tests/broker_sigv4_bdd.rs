//! BDD: AWS SigV4 injection through the broker MITM. The upstream is a
//! self-signed HTTPS server that captures the Authorization +
//! X-Amz-Date headers it received. Test asserts the broker-injected
//! SigV4 headers landed on the wire.

use cucumber::{World, given, then, when};
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use vibe_broker::{
    BoundAddr, Broker, BrokerCa, BrokerHandle, InMemorySecretStore, Policy, SecretStore, SsrfGuard,
    secrets::AwsCredentials,
};

#[derive(Default, World)]
pub struct SWorld {
    rt: Option<Arc<Runtime>>,
    upstream_addr: Option<std::net::SocketAddr>,
    upstream_trust: Option<Arc<RootCertStore>>,
    upstream_handle: Option<JoinHandle<()>>,
    observed_headers: Arc<Mutex<Vec<(String, String)>>>,
    broker_addr: Option<std::net::SocketAddr>,
    broker_handle: Option<BrokerHandle>,
    broker_ca: Option<Arc<BrokerCa>>,
    secrets: Option<Arc<InMemorySecretStore>>,
    upstream_host: String,
}

impl std::fmt::Debug for SWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SWorld").finish()
    }
}

impl SWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }
}

fn install_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn mint_self_signed(host: &str) -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
    let kp = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec![host.to_owned()]).unwrap();
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, host);
    params.distinguished_name = dn;
    let cert = params.self_signed(&kp).unwrap();
    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(kp.serialize_der().into());
    (cert_der, key_der)
}

#[given("a self-signed HTTPS upstream that echoes its received headers")]
fn stub_echo_headers(world: &mut SWorld) {
    install_provider();
    let host = "127.0.0.1".to_string();
    world.upstream_host = host.clone();
    let (cert, key) = mint_self_signed(&host);

    let mut roots = RootCertStore::empty();
    roots.add(cert.clone()).unwrap();
    world.upstream_trust = Some(Arc::new(roots));

    let server_cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));

    let observed = world.observed_headers.clone();
    let rt = world.rt();
    let (addr, handle) = rt.block_on(async move {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            loop {
                let (raw, _) = match listener.accept().await {
                    Ok(p) => p,
                    Err(_) => break,
                };
                let acceptor = acceptor.clone();
                let observed = observed.clone();
                tokio::spawn(async move {
                    let mut tls = match acceptor.accept(raw).await {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    let mut buf = vec![0u8; 8192];
                    let mut total = 0;
                    loop {
                        let n = match tls.read(&mut buf[total..]).await {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(_) => return,
                        };
                        total += n;
                        if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                        if total == buf.len() {
                            break;
                        }
                    }
                    let head = String::from_utf8_lossy(&buf[..total]);
                    let mut found = Vec::new();
                    for line in head.lines().skip(1) {
                        if line.is_empty() {
                            break;
                        }
                        if let Some((k, v)) = line.split_once(':') {
                            found.push((k.trim().to_owned(), v.trim().to_owned()));
                        }
                    }
                    *observed.lock().unwrap() = found;
                    let body = b"ok";
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = tls.write_all(resp.as_bytes()).await;
                    let _ = tls.write_all(body).await;
                });
            }
        });
        (addr, handle)
    });
    world.upstream_addr = Some(addr);
    world.upstream_handle = Some(handle);
}

#[given(expr = "the broker holds AWS credentials at {string} — region {string} service {string}")]
fn stash_aws(world: &mut SWorld, key: String, region: String, service: String) {
    let s = Arc::new(InMemorySecretStore::new());
    s.set_aws(
        key,
        AwsCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            session_token: None,
            region,
            service,
        },
    );
    world.secrets = Some(s);
}

#[given("a policy that allows the upstream on CONNECT and SigV4-injects \"@workspace.aws_default\" on GET")]
fn build_broker(world: &mut SWorld) {
    install_provider();
    let trust = world.upstream_trust.clone().unwrap();
    let secrets = world.secrets.clone().unwrap() as Arc<dyn SecretStore>;
    let ca = Arc::new(BrokerCa::generate().unwrap());
    world.broker_ca = Some(ca.clone());

    let policy = Policy::parse_toml(
        r#"
default = "deny"

[[rule]]
match.host = "127.0.0.1"
match.methods = ["CONNECT"]
match.require_tls = true

[[rule]]
match.host = "127.0.0.1"
match.methods = ["GET"]
match.require_tls = true
inject = { type = "aws-sigv4", profile = "@workspace.aws_default" }
"#,
    )
    .unwrap();

    let broker = Broker::new(policy, SsrfGuard::new().with_allow_host("127.0.0.1"))
        .with_tls_ca(ca)
        .with_upstream_trust(trust)
        .with_secret_store(secrets);
    let rt = world.rt();
    let handle = rt.block_on(async move {
        broker.start_tcp("127.0.0.1:0").await.unwrap()
    });
    if let BoundAddr::Tcp(addr) = handle.addr.clone() {
        world.broker_addr = Some(addr);
    }
    world.broker_handle = Some(handle);
}

#[when("the client performs CONNECT through the broker, then GET on root over TLS")]
fn run_client(world: &mut SWorld) {
    let upstream = world.upstream_addr.unwrap();
    let broker_addr = world.broker_addr.unwrap();
    let ca = world.broker_ca.as_ref().unwrap().clone();
    let host = world.upstream_host.clone();
    let port = upstream.port();
    let rt = world.rt();
    rt.block_on(async move {
        let mut tcp = TcpStream::connect(broker_addr).await.unwrap();
        let connect = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n");
        tcp.write_all(connect.as_bytes()).await.unwrap();
        let mut head = [0u8; 1024];
        let n = tcp.read(&mut head).await.unwrap();
        let head_text = String::from_utf8_lossy(&head[..n]);
        assert!(head_text.starts_with("HTTP/1.1 200"), "got: {head_text}");

        let mut roots = RootCertStore::empty();
        let ca_pem = ca.ca_pem();
        let mut bytes = ca_pem.as_bytes();
        for cert in rustls_pemfile::certs(&mut bytes) {
            roots.add(cert.unwrap()).unwrap();
        }
        let client_cfg = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(client_cfg));
        let server_name = ServerName::try_from(host.clone()).unwrap();
        let mut tls = connector.connect(server_name, tcp).await.unwrap();

        let req = format!(
            "GET / HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
        );
        tls.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = tls.read_to_end(&mut buf).await;
    });
}

fn header_value(world: &SWorld, name: &str) -> Option<String> {
    let want = name.to_ascii_lowercase();
    world
        .observed_headers
        .lock()
        .unwrap()
        .iter()
        .find(|(k, _)| k.to_ascii_lowercase() == want)
        .map(|(_, v)| v.clone())
}

#[then(expr = "the upstream observed Authorization starting with {string}")]
fn auth_starts_with(world: &mut SWorld, prefix: String) {
    let auth = header_value(world, "Authorization").unwrap_or_default();
    assert!(auth.starts_with(&prefix), "Authorization was: {auth}");
}

#[then("the upstream observed an X-Amz-Date header")]
fn x_amz_date_present(world: &mut SWorld) {
    let v = header_value(world, "X-Amz-Date");
    assert!(v.is_some(), "headers: {:?}", world.observed_headers.lock().unwrap());
}

#[then(expr = "the upstream Authorization includes {string}")]
fn auth_includes(world: &mut SWorld, needle: String) {
    let auth = header_value(world, "Authorization").unwrap_or_default();
    assert!(auth.contains(&needle), "Authorization was: {auth}");
}

fn main() {
    futures::executor::block_on(SWorld::run("tests/features/broker_sigv4.feature"));
}
