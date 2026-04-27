//! Resolves `SecretRef` strings (`@profile.X`, `@workspace.Y`, `@env.Z`,
//! `@oauth.<provider>`) into actual secret values that the broker injects
//! into outbound requests.
//!
//! v1 ships an in-memory store the daemon populates from `ProfileStore`
//! and `WorkspaceStore` at sandbox spawn time. The trait shape is async-
//! optional via a sync interface — production callers materialize the
//! map up front; per-request resolution is a HashMap lookup.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::policy::SecretRef;

#[derive(Debug, Clone)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    /// Default region used when the policy rule doesn't pin one.
    pub region: String,
    /// AWS service name (`s3`, `lambda`, `sts`, …). Required for signing.
    pub service: String,
}

/// Pre-minted GCP OAuth2 access token. The token-mint flow (sign a JWT
/// claim with the service-account key, exchange for an access token via
/// `oauth2.googleapis.com`) is the daemon's job (slice B2.4); the broker
/// just consumes the resulting Bearer string.
#[derive(Debug, Clone)]
pub struct GcpAccessToken {
    pub token: String,
}

/// Pre-minted Azure Service Principal / Managed Identity access token.
#[derive(Debug, Clone)]
pub struct AzureAccessToken {
    pub token: String,
}

pub trait SecretStore: Send + Sync {
    fn resolve(&self, secret: &SecretRef) -> Option<String>;

    /// Optional AWS-creds lookup. Default impl returns None; in-memory and
    /// daemon-side impls override.
    fn resolve_aws(&self, _secret: &SecretRef) -> Option<AwsCredentials> {
        None
    }

    fn resolve_gcp(&self, _secret: &SecretRef) -> Option<GcpAccessToken> {
        None
    }

    fn resolve_azure(&self, _secret: &SecretRef) -> Option<AzureAccessToken> {
        None
    }
}

#[derive(Debug, Default)]
pub struct InMemorySecretStore {
    map: RwLock<HashMap<String, String>>,
    aws: RwLock<HashMap<String, AwsCredentials>>,
    gcp: RwLock<HashMap<String, GcpAccessToken>>,
    azure: RwLock<HashMap<String, AzureAccessToken>>,
}

impl InMemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        self.map.write().unwrap().insert(key.into(), value.into());
    }

    pub fn set_aws(&self, key: impl Into<String>, creds: AwsCredentials) {
        self.aws.write().unwrap().insert(key.into(), creds);
    }

    pub fn set_gcp(&self, key: impl Into<String>, token: GcpAccessToken) {
        self.gcp.write().unwrap().insert(key.into(), token);
    }

    pub fn set_azure(&self, key: impl Into<String>, token: AzureAccessToken) {
        self.azure.write().unwrap().insert(key.into(), token);
    }

    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let store = Self::new();
        for (k, v) in pairs {
            store.set(k, v);
        }
        store
    }
}

impl SecretStore for InMemorySecretStore {
    fn resolve(&self, secret: &SecretRef) -> Option<String> {
        self.map.read().unwrap().get(&secret.0).cloned()
    }

    fn resolve_aws(&self, secret: &SecretRef) -> Option<AwsCredentials> {
        self.aws.read().unwrap().get(&secret.0).cloned()
    }

    fn resolve_gcp(&self, secret: &SecretRef) -> Option<GcpAccessToken> {
        self.gcp.read().unwrap().get(&secret.0).cloned()
    }

    fn resolve_azure(&self, secret: &SecretRef) -> Option<AzureAccessToken> {
        self.azure.read().unwrap().get(&secret.0).cloned()
    }
}

/// Convenience: a store that has nothing in it. Use when no rules need
/// injection.
pub struct EmptySecretStore;

impl SecretStore for EmptySecretStore {
    fn resolve(&self, _: &SecretRef) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_store_round_trip() {
        let s = InMemorySecretStore::new();
        s.set("@profile.foo", "bar");
        let got = s.resolve(&SecretRef("@profile.foo".into()));
        assert_eq!(got.as_deref(), Some("bar"));
    }

    #[test]
    fn missing_returns_none() {
        let s = InMemorySecretStore::new();
        assert!(s.resolve(&SecretRef("@profile.absent".into())).is_none());
    }

    #[test]
    fn empty_store_resolves_nothing() {
        let s = EmptySecretStore;
        assert!(s.resolve(&SecretRef("@profile.foo".into())).is_none());
    }

    #[test]
    fn from_pairs_populates() {
        let s = InMemorySecretStore::from_pairs([("@profile.a", "1"), ("@profile.b", "2")]);
        assert_eq!(s.resolve(&SecretRef("@profile.a".into())).as_deref(), Some("1"));
        assert_eq!(s.resolve(&SecretRef("@profile.b".into())).as_deref(), Some("2"));
    }
}
