#![allow(dead_code)]
//! Agent capability discovery — dynamic advertisement and negotiation.
//! FIT-GAP v11 Phase 48 — closes gap vs Cursor 4.0, Devin 2.0.

use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A named capability with an optional version and parameters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability {
    pub name: String,
    pub version: String,
    pub params: Vec<String>,
}

impl Capability {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), version: "1.0".to_string(), params: Vec::new() }
    }
    pub fn with_version(mut self, v: impl Into<String>) -> Self {
        self.version = v.into();
        self
    }
    pub fn with_param(mut self, p: impl Into<String>) -> Self {
        self.params.push(p.into());
        self
    }
}

/// Capability advertisement from an agent.
#[derive(Debug, Clone)]
pub struct CapabilityAdvertisement {
    pub agent_id: String,
    pub capabilities: Vec<Capability>,
    pub timestamp_ms: u64,
    pub ttl_ms: u64,
}

impl CapabilityAdvertisement {
    pub fn new(agent_id: impl Into<String>, caps: Vec<Capability>, ts: u64, ttl: u64) -> Self {
        Self { agent_id: agent_id.into(), capabilities: caps, timestamp_ms: ts, ttl_ms: ttl }
    }
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms > self.timestamp_ms + self.ttl_ms
    }
    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities.iter().any(|c| c.name == name)
    }
}

/// Result of a capability negotiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiationOutcome {
    /// All required capabilities satisfied; `agent_id` selected.
    Satisfied { agent_id: String },
    /// Partially satisfied; some capabilities missing.
    Partial { agent_id: String, missing: Vec<String> },
    /// No agent can satisfy requirements.
    Unsatisfied { missing: Vec<String> },
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Tracks agent capability advertisements and resolves requests.
#[derive(Debug, Default)]
pub struct CapabilityRegistry {
    /// agent_id → latest advertisement
    advertisements: HashMap<String, CapabilityAdvertisement>,
}

impl CapabilityRegistry {
    pub fn new() -> Self { Self::default() }

    /// Register or update an advertisement.
    pub fn advertise(&mut self, ad: CapabilityAdvertisement) {
        self.advertisements.insert(ad.agent_id.clone(), ad);
    }

    /// Remove an advertisement by agent_id.
    pub fn withdraw(&mut self, agent_id: &str) -> bool {
        self.advertisements.remove(agent_id).is_some()
    }

    /// Remove expired advertisements.
    pub fn prune_expired(&mut self, now_ms: u64) -> usize {
        let before = self.advertisements.len();
        self.advertisements.retain(|_, ad| !ad.is_expired(now_ms));
        before - self.advertisements.len()
    }

    /// Find all live agents that advertise a given capability.
    pub fn find_by_capability(&self, cap_name: &str, now_ms: u64) -> Vec<&CapabilityAdvertisement> {
        self.advertisements.values()
            .filter(|ad| !ad.is_expired(now_ms) && ad.has_capability(cap_name))
            .collect()
    }

    /// Negotiate: find the best agent satisfying all `required` capabilities.
    pub fn negotiate(&self, required: &[&str], now_ms: u64) -> NegotiationOutcome {
        let required_set: HashSet<&str> = required.iter().copied().collect();
        let mut best_agent: Option<(&str, usize)> = None; // (agent_id, matched_count)

        for (agent_id, ad) in &self.advertisements {
            if ad.is_expired(now_ms) { continue; }
            let agent_caps: HashSet<&str> = ad.capabilities.iter().map(|c| c.name.as_str()).collect();
            let matched = required_set.iter().filter(|&&r| agent_caps.contains(r)).count();
            if matched > best_agent.map(|(_, n)| n).unwrap_or(0) {
                best_agent = Some((agent_id.as_str(), matched));
            }
        }

        match best_agent {
            Some((agent_id, matched)) if matched == required_set.len() => {
                NegotiationOutcome::Satisfied { agent_id: agent_id.to_string() }
            }
            Some((agent_id, _)) => {
                let ad = &self.advertisements[agent_id];
                let agent_caps: HashSet<&str> = ad.capabilities.iter().map(|c| c.name.as_str()).collect();
                let missing: Vec<String> = required_set.iter()
                    .filter(|&&r| !agent_caps.contains(r))
                    .map(|s| s.to_string())
                    .collect();
                NegotiationOutcome::Partial { agent_id: agent_id.to_string(), missing }
            }
            None => {
                NegotiationOutcome::Unsatisfied {
                    missing: required.iter().map(|s| s.to_string()).collect()
                }
            }
        }
    }

    /// List all distinct capability names across live agents.
    pub fn known_capabilities(&self, now_ms: u64) -> Vec<String> {
        let mut names: HashSet<String> = HashSet::new();
        for ad in self.advertisements.values() {
            if ad.is_expired(now_ms) { continue; }
            for c in &ad.capabilities {
                names.insert(c.name.clone());
            }
        }
        let mut v: Vec<_> = names.into_iter().collect();
        v.sort();
        v
    }

    pub fn agent_count(&self) -> usize { self.advertisements.len() }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ad(agent_id: &str, caps: &[&str], ts: u64, ttl: u64) -> CapabilityAdvertisement {
        let capabilities = caps.iter().map(|&n| Capability::new(n)).collect();
        CapabilityAdvertisement::new(agent_id, capabilities, ts, ttl)
    }

    #[test]
    fn test_advertise_and_count() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("agent-1", &["code_edit", "git_ops"], 0, 60_000));
        assert_eq!(r.agent_count(), 1);
    }

    #[test]
    fn test_withdraw() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("agent-1", &["code_edit"], 0, 60_000));
        assert!(r.withdraw("agent-1"));
        assert!(!r.withdraw("agent-1"));
    }

    #[test]
    fn test_find_by_capability() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("a1", &["code_edit", "test_run"], 0, 60_000));
        r.advertise(ad("a2", &["test_run"], 0, 60_000));
        let found = r.find_by_capability("test_run", 0);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_find_expired_excluded() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("a1", &["code_edit"], 0, 100));
        // now = 200 > 0 + 100
        let found = r.find_by_capability("code_edit", 200);
        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_prune_expired() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("a1", &["code_edit"], 0, 100));
        r.advertise(ad("a2", &["test_run"], 0, 5000));
        let pruned = r.prune_expired(200);
        assert_eq!(pruned, 1);
        assert_eq!(r.agent_count(), 1);
    }

    #[test]
    fn test_negotiate_satisfied() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("a1", &["code_edit", "git_ops", "test_run"], 0, 60_000));
        let out = r.negotiate(&["code_edit", "git_ops"], 0);
        assert!(matches!(out, NegotiationOutcome::Satisfied { .. }));
    }

    #[test]
    fn test_negotiate_partial() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("a1", &["code_edit"], 0, 60_000));
        let out = r.negotiate(&["code_edit", "deploy"], 0);
        assert!(matches!(out, NegotiationOutcome::Partial { .. }));
    }

    #[test]
    fn test_negotiate_unsatisfied() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("a1", &["test_run"], 0, 60_000));
        let out = r.negotiate(&["deploy", "web_search"], 0);
        assert!(matches!(out, NegotiationOutcome::Unsatisfied { .. }));
    }

    #[test]
    fn test_known_capabilities() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("a1", &["code_edit", "git_ops"], 0, 60_000));
        r.advertise(ad("a2", &["test_run", "git_ops"], 0, 60_000));
        let caps = r.known_capabilities(0);
        assert!(caps.contains(&"git_ops".to_string()));
        assert_eq!(caps.iter().filter(|c| *c == "git_ops").count(), 1);
    }

    #[test]
    fn test_capability_version() {
        let c = Capability::new("code_edit").with_version("2.1");
        assert_eq!(c.version, "2.1");
    }

    #[test]
    fn test_capability_params() {
        let c = Capability::new("file_read").with_param("max_size_mb=100");
        assert_eq!(c.params, vec!["max_size_mb=100"]);
    }

    #[test]
    fn test_has_capability() {
        let a = ad("a1", &["code_edit", "git_ops"], 0, 60_000);
        assert!(a.has_capability("code_edit"));
        assert!(!a.has_capability("deploy"));
    }

    #[test]
    fn test_overwrite_advertisement() {
        let mut r = CapabilityRegistry::new();
        r.advertise(ad("a1", &["code_edit"], 0, 60_000));
        r.advertise(ad("a1", &["test_run"], 0, 60_000));
        assert_eq!(r.agent_count(), 1);
        let found = r.find_by_capability("code_edit", 0);
        assert_eq!(found.len(), 0);
    }
}
