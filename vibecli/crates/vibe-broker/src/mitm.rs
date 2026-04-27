//! Slice B1.8: TLS handshake + plaintext forwarding inside the CONNECT
//! tunnel.
//!
//! After `accept::handle_connect` writes `200 Connection Established`,
//! `run_mitm` is the next step:
//!   1. Mint (or fetch) a leaf cert for the requested SNI.
//!   2. Perform a rustls server handshake on the client-facing socket.
//!   3. Open TCP to the upstream `(host, port)` and TLS-handshake to it.
//!   4. Bidirectionally copy plaintext until either side closes.
//!
//! Per-request policy enforcement on decrypted traffic — the basis for
//! Bearer / SigV4 / IMDS injection (B2) — is layered in the next slice;
//! today the policy decision was already made on the CONNECT line.

use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::tls::{BrokerCa, TlsError};

#[derive(Debug, thiserror::Error)]
pub enum MitmError {
    #[error("tls config: {0}")]
    Config(String),
    #[error("tls handshake to client failed: {0}")]
    ServerHandshake(String),
    #[error("upstream connect failed: {0}")]
    UpstreamConnect(std::io::Error),
    #[error("tls handshake to upstream failed: {0}")]
    UpstreamHandshake(String),
    #[error("forwarding: {0}")]
    Forwarding(std::io::Error),
    #[error("tls module: {0}")]
    Tls(#[from] TlsError),
}

/// Run the MITM after CONNECT has already received `200 Connection
/// Established`. The caller is responsible for the CONNECT line + the
/// leading 200 response.
///
/// `client` is the raw TCP stream the broker accepted. `host` is the SNI
/// used for both the leaf cert and the upstream connection. `upstream_trust`
/// is the rustls RootCertStore used to verify the real upstream — defaults
/// to `webpki-roots`, but tests inject their own.
pub async fn run_mitm<S>(
    client: S,
    host: &str,
    port: u16,
    ca: &Arc<BrokerCa>,
    upstream_trust: Arc<RootCertStore>,
    timeout: Duration,
) -> Result<(), MitmError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // ---- 1. Server-side handshake (broker pretends to be `host`) ----
    let (chain, key) = ca.leaf_for_rustls(host)?;
    let server_cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(|e| MitmError::Config(e.to_string()))?;
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));
    let client_tls = match tokio::time::timeout(timeout, acceptor.accept(client)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(MitmError::ServerHandshake(e.to_string())),
        Err(_) => return Err(MitmError::ServerHandshake("timeout".into())),
    };

    // ---- 2. Open TCP + TLS-connect to the real upstream ------------
    let target = format!("{host}:{port}");
    let upstream_tcp = match tokio::time::timeout(timeout, TcpStream::connect(&target)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(MitmError::UpstreamConnect(e)),
        Err(_) => {
            return Err(MitmError::UpstreamConnect(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "upstream connect timeout",
            )))
        }
    };
    let client_cfg = ClientConfig::builder()
        .with_root_certificates(Arc::unwrap_or_clone(upstream_trust))
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(client_cfg));
    let server_name = ServerName::try_from(host.to_owned())
        .map_err(|e| MitmError::Config(format!("bad SNI {host}: {e}")))?;
    let upstream_tls = match tokio::time::timeout(
        timeout,
        connector.connect(server_name, upstream_tcp),
    )
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(MitmError::UpstreamHandshake(e.to_string())),
        Err(_) => return Err(MitmError::UpstreamHandshake("timeout".into())),
    };

    // ---- 3. Bidirectional plaintext copy ---------------------------
    let (mut cr, mut cw) = tokio::io::split(client_tls);
    let (mut ur, mut uw) = tokio::io::split(upstream_tls);

    let c2u = async {
        let _ = tokio::io::copy(&mut cr, &mut uw).await;
        let _ = uw.shutdown().await;
    };
    let u2c = async {
        let _ = tokio::io::copy(&mut ur, &mut cw).await;
        let _ = cw.shutdown().await;
    };

    tokio::join!(c2u, u2c);
    Ok(())
}

/// Default upstream-trust store backed by Mozilla's bundled root CAs via
/// the `webpki-roots` crate. Daemon callers will normally use this; tests
/// inject their own minimal store via `Broker::with_upstream_trust`.
pub fn default_upstream_roots() -> RootCertStore {
    let mut store = RootCertStore::empty();
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    store
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_upstream_roots_is_nonempty() {
        let store = default_upstream_roots();
        assert!(store.len() > 0);
    }
}
