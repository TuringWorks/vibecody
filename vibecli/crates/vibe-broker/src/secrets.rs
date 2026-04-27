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

pub trait SecretStore: Send + Sync {
    fn resolve(&self, secret: &SecretRef) -> Option<String>;
}

#[derive(Debug, Default)]
pub struct InMemorySecretStore {
    map: RwLock<HashMap<String, String>>,
}

impl InMemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        self.map.write().unwrap().insert(key.into(), value.into());
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
