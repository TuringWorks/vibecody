//! Per-broker root CA + ephemeral leaf-cert factory.
//!
//! The broker mints a self-signed root CA on first run. For every origin
//! the broker proxies, it mints a short-lived leaf cert signed by that
//! root and caches it in memory. The CA cert is the only thing that ever
//! touches disk; leaf private keys never leave the broker process.
//!
//! Design rationale (see `docs/design/sandbox-tiers/02-egress-broker.md`):
//! - host system CA store is never touched
//! - the sandbox is told to trust the broker root via env-var injection
//!   (`SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS`, …)
//! - clients pinning their own root pool refuse the broker — handled by
//!   per-rule MITM bypass in a follow-up slice

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
    KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("rcgen error: {0}")]
    Rcgen(String),
    #[error("rustls error: {0}")]
    Rustls(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<rcgen::Error> for TlsError {
    fn from(e: rcgen::Error) -> Self {
        TlsError::Rcgen(e.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct LeafCert {
    pub cert_pem: String,
    pub key_pem: String,
    /// SAN list at issue time. Useful for tests + audit.
    pub san_list: Vec<String>,
    /// rcgen-assigned serial bytes; lets callers tell two leaves apart.
    pub serial: Vec<u8>,
}

/// Owns the broker's root signing key + a leaf cache keyed by SNI host.
pub struct BrokerCa {
    ca_keypair_pem: String,
    ca_cert_pem: String,
    /// Live rcgen Certificate used as the `issuer` argument when minting
    /// leaves. On generate this is the freshly-self-signed cert; on
    /// load_or_generate we re-self-sign from disk-stored params + key.
    /// The DER bytes may differ from `ca_cert_pem` after reload, but the
    /// signing identity (DN + key + key-usages) is the same so leaves
    /// chain to the on-disk public PEM.
    ca_cert: Certificate,
    ca_kp: KeyPair,
    /// Cached leaves; rendered PEM strings.
    leaf_cache: Mutex<HashMap<String, LeafCert>>,
}

impl std::fmt::Debug for BrokerCa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrokerCa")
            .field("cached_origins", &self.leaf_cache.lock().unwrap().len())
            .finish()
    }
}

impl BrokerCa {
    /// Mint a fresh CA in memory. Test/dev convenience; production callers
    /// use `load_or_generate(path)` so the CA is stable across daemon
    /// restarts (clients keep trusting it).
    pub fn generate() -> Result<Self, TlsError> {
        let kp = KeyPair::generate()?;
        let cert = build_ca_certificate(&kp)?;
        Ok(BrokerCa {
            ca_keypair_pem: kp.serialize_pem(),
            ca_cert_pem: cert.pem(),
            ca_cert: cert,
            ca_kp: kp,
            leaf_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Read the CA from `<dir>/ca.pem` + `<dir>/ca.key.pem`, generating
    /// + saving a fresh one if either file is missing. The directory is
    /// created with mode 0700 on Unix.
    pub fn load_or_generate(dir: &Path) -> Result<Self, TlsError> {
        let cert_path = dir.join("ca.pem");
        let key_path = dir.join("ca.key.pem");
        if cert_path.exists() && key_path.exists() {
            let ca_cert_pem = std::fs::read_to_string(&cert_path)?;
            let ca_keypair_pem = std::fs::read_to_string(&key_path)?;
            let kp = KeyPair::from_pem(&ca_keypair_pem)?;
            // Recover params from the disk PEM so the in-memory issuer
            // has the same DN + key-usages as the original. We re-self-
            // sign to get a Certificate; its DER differs from the disk
            // bytes, but it carries identical signing identity, so leaves
            // it issues chain to the on-disk public PEM.
            let params = CertificateParams::from_ca_cert_pem(&ca_cert_pem)?;
            let cert = params.self_signed(&kp)?;
            return Ok(BrokerCa {
                ca_keypair_pem,
                ca_cert_pem,
                ca_cert: cert,
                ca_kp: kp,
                leaf_cache: Mutex::new(HashMap::new()),
            });
        }
        std::fs::create_dir_all(dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
        let ca = Self::generate()?;
        std::fs::write(&cert_path, &ca.ca_cert_pem)?;
        std::fs::write(&key_path, &ca.ca_keypair_pem)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(ca)
    }

    /// PEM-encoded CA cert (public). Inject into the sandbox env via
    /// `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS`, etc.
    pub fn ca_pem(&self) -> &str {
        &self.ca_cert_pem
    }

    /// Mint or fetch a cached leaf for `hostname`. Cache key is the bare
    /// SNI host (no port).
    pub fn leaf_for(&self, hostname: &str) -> Result<LeafCert, TlsError> {
        if let Some(cached) = self.leaf_cache.lock().unwrap().get(hostname) {
            return Ok(cached.clone());
        }
        let leaf_kp = KeyPair::generate()?;
        let mut leaf_params = CertificateParams::new(vec![hostname.to_string()])?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, hostname);
        leaf_params.distinguished_name = dn;
        leaf_params.is_ca = IsCa::ExplicitNoCa;
        leaf_params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        let leaf = leaf_params.signed_by(&leaf_kp, &self.ca_cert, &self.ca_kp)?;

        let leaf_cert = LeafCert {
            cert_pem: leaf.pem(),
            key_pem: leaf_kp.serialize_pem(),
            san_list: vec![hostname.to_string()],
            serial: leaf
                .params()
                .serial_number
                .as_ref()
                .map(|s| s.to_bytes())
                .unwrap_or_default(),
        };
        self.leaf_cache
            .lock()
            .unwrap()
            .insert(hostname.to_string(), leaf_cert.clone());
        Ok(leaf_cert)
    }

    /// Return rustls-shaped chain + key for the leaf, ready to feed into
    /// `tokio_rustls::rustls::ServerConfig::builder().with_single_cert(...)`.
    pub fn leaf_for_rustls(
        &self,
        hostname: &str,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), TlsError> {
        let leaf = self.leaf_for(hostname)?;
        let cert_der = parse_cert_pem(&leaf.cert_pem)?;
        let key_der = parse_private_key_pem(&leaf.key_pem)?;
        Ok((vec![cert_der], key_der))
    }
}

fn build_ca_certificate(kp: &KeyPair) -> Result<Certificate, TlsError> {
    let mut params = CertificateParams::new(Vec::<String>::new())?;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "VibeCody Sandbox Broker Root CA");
    dn.push(DnType::OrganizationName, "VibeCody");
    params.distinguished_name = dn;
    Ok(params.self_signed(kp)?)
}

fn parse_cert_pem(pem: &str) -> Result<CertificateDer<'static>, TlsError> {
    let mut bytes = pem.as_bytes();
    let mut iter = rustls_pemfile::certs(&mut bytes);
    match iter.next() {
        Some(Ok(cert)) => Ok(cert),
        Some(Err(e)) => Err(TlsError::Rustls(format!("cert decode: {e}"))),
        None => Err(TlsError::Rustls("no certificate in PEM".into())),
    }
}

fn parse_private_key_pem(pem: &str) -> Result<PrivateKeyDer<'static>, TlsError> {
    use rustls_pemfile::Item;
    let mut reader = pem.as_bytes();
    while let Some(item) = rustls_pemfile::read_one(&mut reader).transpose() {
        match item {
            Ok(Item::Pkcs8Key(k)) => return Ok(PrivateKeyDer::Pkcs8(k)),
            Ok(Item::Pkcs1Key(k)) => return Ok(PrivateKeyDer::Pkcs1(k)),
            Ok(Item::Sec1Key(k)) => return Ok(PrivateKeyDer::Sec1(k)),
            Ok(_) => continue,
            Err(e) => return Err(TlsError::Rustls(format!("key decode: {e}"))),
        }
    }
    Err(TlsError::Rustls("no private key in PEM".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ca_pem_starts_and_ends_correctly() {
        let ca = BrokerCa::generate().unwrap();
        let pem = ca.ca_pem();
        assert!(pem.starts_with("-----BEGIN CERTIFICATE-----"));
        assert!(pem.contains("-----END CERTIFICATE-----"));
    }

    #[test]
    fn leaf_for_includes_san() {
        let ca = BrokerCa::generate().unwrap();
        let leaf = ca.leaf_for("api.example.com").unwrap();
        assert!(leaf.san_list.contains(&"api.example.com".to_string()));
        assert!(leaf.cert_pem.starts_with("-----BEGIN CERTIFICATE-----"));
    }

    #[test]
    fn leaf_cache_returns_same_serial() {
        let ca = BrokerCa::generate().unwrap();
        let a = ca.leaf_for("api.example.com").unwrap();
        let b = ca.leaf_for("api.example.com").unwrap();
        assert_eq!(a.serial, b.serial);
    }

    #[test]
    fn distinct_hosts_get_distinct_leaves() {
        let ca = BrokerCa::generate().unwrap();
        let a = ca.leaf_for("api.example.com").unwrap();
        let b = ca.leaf_for("api.openai.com").unwrap();
        assert_ne!(a.cert_pem, b.cert_pem);
    }

    #[test]
    fn leaf_for_rustls_returns_chain_and_key() {
        let ca = BrokerCa::generate().unwrap();
        let (chain, _key) = ca.leaf_for_rustls("api.example.com").unwrap();
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn load_or_generate_persists_ca() {
        let dir = tempfile::tempdir().unwrap();
        let ca1 = BrokerCa::load_or_generate(dir.path()).unwrap();
        let pem1 = ca1.ca_pem().to_owned();
        drop(ca1);
        let ca2 = BrokerCa::load_or_generate(dir.path()).unwrap();
        assert_eq!(ca2.ca_pem(), pem1);
    }
}
