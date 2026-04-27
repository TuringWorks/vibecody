//! BDD: full TLS MITM through the broker. Spins up a tiny rustls server
//! using a self-signed cert minted on the fly, points the broker at that
//! cert as its upstream trust store, then has a TLS client perform the
//! full CONNECT + GET / dance through the broker.

use cucumber::{World, given, then, when};
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{
    CertificateDer, PrivateKeyDer, ServerName,
};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use vibe_broker::{
    BoundAddr, Broker, BrokerCa, BrokerHandle, Policy, SsrfGuard,
};

#[derive(Default, World)]
pub struct MWorld {
    rt: Option<Arc<Runtime>>,
    upstream_addr: Option<std::net::SocketAddr>,
    upstream_trust: Option<Arc<RootCertStore>>,
    upstream_handle: Option<JoinHandle<()>>,
    broker_addr: Option<std::net::SocketAddr>,
    broker_handle: Option<BrokerHandle>,
    broker_ca: Option<Arc<BrokerCa>>,
    upstream_host: String,
    response_status: Option<u16>,
    response_body: Vec<u8>,
}

impl std::fmt::Debug for MWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MWorld")
            .field("upstream_addr", &self.upstream_addr)
            .field("status", &self.response_status)
            .finish()
    }
}

impl MWorld {
    fn rt(&mut self) -> Arc<Runtime> {
        if self.rt.is_none() {
            self.rt = Some(Arc::new(Runtime::new().unwrap()));
        }
        self.rt.as_ref().unwrap().clone()
    }
}

fn install_default_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn mint_self_signed_for(host: &str) -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
    let kp = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec![host.to_owned()]).unwrap();
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, host);
    params.distinguished_name = dn;
    let cert = params.self_signed(&kp).unwrap();
    let cert_der = CertificateDer::from(cert.der().to_vec());
    // KeyPair::serialize_der returns PKCS#8 bytes
    let key_der = PrivateKeyDer::Pkcs8(kp.serialize_der().into());
    (cert_der, key_der)
}

#[given(expr = "a self-signed HTTPS upstream that replies {string}")]
fn stub_upstream(world: &mut MWorld, body: String) {
    install_default_provider();
    let host = "127.0.0.1".to_string();
    let (cert, key) = mint_self_signed_for(&host);

    // upstream trust = single self-signed cert
    let mut roots = RootCertStore::empty();
    roots.add(cert.clone()).unwrap();
    world.upstream_trust = Some(Arc::new(roots));
    world.upstream_host = host;

    let server_cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));
    let body_static: &'static str = Box::leak(body.into_boxed_str());

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
                tokio::spawn(async move {
                    let mut tls = match acceptor.accept(raw).await {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    let mut buf = [0u8; 1024];
                    let _ = tls.read(&mut buf).await;
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_static}",
                        body_static.len()
                    );
                    let _ = tls.write_all(resp.as_bytes()).await;
                });
            }
        });
        (addr, handle)
    });
    world.upstream_addr = Some(addr);
    world.upstream_handle = Some(handle);
}

#[given("a broker with TLS interception and the upstream cert in its trust store")]
fn broker_with_trust(world: &mut MWorld) {
    install_default_provider();
    let trust = world.upstream_trust.clone().unwrap();
    let ca = Arc::new(BrokerCa::generate().unwrap());
    world.broker_ca = Some(ca.clone());
    let policy = Policy::parse_toml(
        r#"
default = "deny"

[[rule]]
match.host = "127.0.0.1"
match.methods = ["CONNECT"]
match.require_tls = true
"#,
    )
    .unwrap();
    let broker = Broker::new(policy, SsrfGuard::new().with_allow_host("127.0.0.1"))
        .with_tls_ca(ca)
        .with_upstream_trust(trust);
    let rt = world.rt();
    let handle = rt.block_on(async move {
        broker.start_tcp("127.0.0.1:0").await.unwrap()
    });
    if let BoundAddr::Tcp(addr) = handle.addr.clone() {
        world.broker_addr = Some(addr);
    }
    world.broker_handle = Some(handle);
}

#[given("a policy allowing the upstream host on CONNECT")]
fn policy_allows(_world: &mut MWorld) {
    // built into broker_with_trust
}

#[when("the client performs CONNECT through the broker, then GET / over TLS")]
fn run_client(world: &mut MWorld) {
    let upstream = world.upstream_addr.unwrap();
    let broker_addr = world.broker_addr.unwrap();
    let ca = world.broker_ca.as_ref().unwrap().clone();
    let host = world.upstream_host.clone();
    let port = upstream.port();
    let rt = world.rt();
    let (status, body) = rt.block_on(async move {
        // 1. TCP to broker
        let mut tcp = TcpStream::connect(broker_addr).await.unwrap();
        // 2. CONNECT
        let connect = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n");
        tcp.write_all(connect.as_bytes()).await.unwrap();
        // 3. Read 200
        let mut head = [0u8; 1024];
        let n = tcp.read(&mut head).await.unwrap();
        let head_text = String::from_utf8_lossy(&head[..n]);
        assert!(head_text.starts_with("HTTP/1.1 200"), "got: {head_text}");

        // 4. TLS handshake with broker (trust the broker CA only)
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

        // 5. GET /
        let req = format!("GET / HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");
        tls.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        let _ = tls.read_to_end(&mut buf).await;

        // 6. Parse response
        let split = buf.windows(4).position(|w| w == b"\r\n\r\n").unwrap_or(buf.len());
        let head_text = String::from_utf8_lossy(&buf[..split]);
        let status_line = head_text.split("\r\n").next().unwrap();
        let status: u16 = status_line.split_whitespace().nth(1).unwrap().parse().unwrap();
        let body = if split + 4 <= buf.len() {
            buf[split + 4..].to_vec()
        } else {
            Vec::new()
        };
        (status, body)
    });
    world.response_status = Some(status);
    world.response_body = body;
}

#[then(expr = "the client receives status {int} and body {string}")]
fn assert_response(world: &mut MWorld, status: u16, body: String) {
    assert_eq!(world.response_status, Some(status));
    assert_eq!(String::from_utf8_lossy(&world.response_body), body);
}

fn main() {
    futures::executor::block_on(MWorld::run("tests/features/broker_mitm.feature"));
}
