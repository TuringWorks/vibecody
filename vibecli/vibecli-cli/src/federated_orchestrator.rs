//! Federated multi-org agent orchestration.
//!
//! GAP-v9-017: rivals Devin Org Orchestration, Cursor Multi-Workspace, Amazon Q Multi-Account.
//! - Org registry with capabilities, trust levels, and SLA tiers
//! - Cross-org task routing with capability matching
//! - Federation policies: allow-list, deny-list, data-residency constraints
//! - Result aggregation and conflict resolution (majority vote, priority merge)
//! - Audit log with per-org attribution
//! - Token budget enforcement per federated call

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Organisation ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrgId(pub String);

impl OrgId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
}

impl std::fmt::Display for OrgId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel { Untrusted, Verified, Trusted, Partner }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlaTier { BestEffort, Standard, Premium, Critical }

/// Data-residency constraint: which regions are permitted for this org's data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataResidency {
    pub allowed_regions: Vec<String>,
}

impl DataResidency {
    pub fn global() -> Self { Self { allowed_regions: vec!["*".into()] } }
    pub fn allows(&self, region: &str) -> bool {
        self.allowed_regions.iter().any(|r| r == "*" || r == region)
    }
}

/// A registered federated organisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgNode {
    pub id: OrgId,
    pub name: String,
    pub capabilities: Vec<String>,
    pub trust: TrustLevel,
    pub sla: SlaTier,
    pub residency: DataResidency,
    /// Max tokens this org may consume per federated call.
    pub token_budget: u64,
    pub online: bool,
}

impl OrgNode {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: OrgId::new(id), name: name.into(),
            capabilities: Vec::new(), trust: TrustLevel::Untrusted,
            sla: SlaTier::BestEffort, residency: DataResidency::global(),
            token_budget: 10_000, online: true,
        }
    }

    pub fn with_trust(mut self, t: TrustLevel) -> Self { self.trust = t; self }
    pub fn with_sla(mut self, s: SlaTier) -> Self { self.sla = s; self }
    pub fn with_budget(mut self, b: u64) -> Self { self.token_budget = b; self }
    pub fn add_capability(mut self, cap: impl Into<String>) -> Self { self.capabilities.push(cap.into()); self }
    pub fn with_residency(mut self, r: DataResidency) -> Self { self.residency = r; self }
    pub fn offline(mut self) -> Self { self.online = false; self }

    pub fn has_capability(&self, cap: &str) -> bool { self.capabilities.iter().any(|c| c == cap) }
}

// ─── Federation Policy ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPolicy {
    /// OrgIds explicitly allowed to receive tasks.
    pub allow_list: Vec<OrgId>,
    /// OrgIds explicitly denied.
    pub deny_list: Vec<OrgId>,
    /// Required minimum trust to participate.
    pub min_trust: TrustLevel,
    /// Task region (data must reside here).
    pub task_region: String,
}

impl FederationPolicy {
    pub fn permissive() -> Self {
        Self { allow_list: vec![], deny_list: vec![], min_trust: TrustLevel::Untrusted, task_region: "global".into() }
    }

    pub fn permits(&self, org: &OrgNode) -> bool {
        if !self.deny_list.is_empty() && self.deny_list.contains(&org.id) { return false; }
        if !self.allow_list.is_empty() && !self.allow_list.contains(&org.id) { return false; }
        if org.trust < self.min_trust { return false; }
        if !org.residency.allows(&self.task_region) { return false; }
        true
    }
}

// ─── Task & Result ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedTask {
    pub id: String,
    pub required_capability: String,
    pub payload: String,
    pub token_cost: u64,
    pub policy: FederationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedResult {
    pub task_id: String,
    pub org_id: OrgId,
    pub output: String,
    pub tokens_used: u64,
    pub success: bool,
}

// ─── Audit Entry ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub task_id: String,
    pub org_id: OrgId,
    pub event: AuditEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditEvent {
    TaskRouted,
    TaskCompleted { success: bool },
    TokenBudgetExceeded,
    PolicyDenied,
}

// ─── Conflict Resolution ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStrategy { FirstWins, MajorityVote, PriorityMerge, LastWins }

/// Resolve conflicting results from multiple orgs.
pub fn resolve_conflicts<'a>(results: &'a [FederatedResult], strategy: &ConflictStrategy) -> Option<&'a FederatedResult> {
    let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
    if successful.is_empty() { return None; }

    match strategy {
        ConflictStrategy::FirstWins => Some(successful[0]),
        ConflictStrategy::LastWins  => successful.last().copied(),
        ConflictStrategy::MajorityVote => {
            // Output with most matching exact strings wins.
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for r in &successful { *counts.entry(r.output.as_str()).or_insert(0) += 1; }
            let best = counts.into_iter().max_by_key(|(_, c)| *c).map(|(s, _)| s)?;
            successful.iter().find(|r| r.output == best).copied()
        }
        ConflictStrategy::PriorityMerge => {
            // Return result with fewest tokens used (most efficient).
            successful.iter().min_by_key(|r| r.tokens_used).copied()
        }
    }
}

// ─── Orchestrator ────────────────────────────────────────────────────────────

pub struct FederatedOrchestrator {
    pub orgs: HashMap<OrgId, OrgNode>,
    pub audit_log: Vec<AuditEntry>,
    pub conflict_strategy: ConflictStrategy,
}

impl FederatedOrchestrator {
    pub fn new(strategy: ConflictStrategy) -> Self {
        Self { orgs: HashMap::new(), audit_log: Vec::new(), conflict_strategy: strategy }
    }

    pub fn register(&mut self, org: OrgNode) { self.orgs.insert(org.id.clone(), org); }

    /// Select eligible orgs for a task (online, policy-permitted, has capability, budget sufficient).
    pub fn eligible_orgs(&self, task: &FederatedTask) -> Vec<&OrgNode> {
        self.orgs.values().filter(|org| {
            org.online
            && org.has_capability(&task.required_capability)
            && task.policy.permits(org)
            && org.token_budget >= task.token_cost
        }).collect()
    }

    /// Route task to all eligible orgs and collect results (simulated).
    pub fn route(&mut self, task: &FederatedTask, simulate_outputs: &HashMap<OrgId, (String, bool)>) -> Vec<FederatedResult> {
        let eligible: Vec<OrgId> = self.eligible_orgs(task).iter().map(|o| o.id.clone()).collect();
        let mut results = Vec::new();

        for org_id in &eligible {
            let org = &self.orgs[org_id];

            // Token budget check
            if org.token_budget < task.token_cost {
                self.audit_log.push(AuditEntry {
                    task_id: task.id.clone(), org_id: org_id.clone(),
                    event: AuditEvent::TokenBudgetExceeded,
                });
                continue;
            }

            self.audit_log.push(AuditEntry {
                task_id: task.id.clone(), org_id: org_id.clone(),
                event: AuditEvent::TaskRouted,
            });

            let (output, success) = simulate_outputs
                .get(org_id)
                .map(|(o, s)| (o.clone(), *s))
                .unwrap_or_else(|| ("(no response)".into(), false));

            let result = FederatedResult {
                task_id: task.id.clone(), org_id: org_id.clone(),
                output, tokens_used: task.token_cost, success,
            };
            self.audit_log.push(AuditEntry {
                task_id: task.id.clone(), org_id: org_id.clone(),
                event: AuditEvent::TaskCompleted { success: result.success },
            });
            results.push(result);
        }

        results
    }

    /// Route and automatically resolve via configured strategy.
    pub fn route_and_resolve(&mut self, task: &FederatedTask, simulate_outputs: &HashMap<OrgId, (String, bool)>) -> Option<String> {
        let results = self.route(task, simulate_outputs);
        resolve_conflicts(&results, &self.conflict_strategy).map(|r| r.output.clone())
    }

    /// Return audit entries for a specific task.
    pub fn audit_for_task(&self, task_id: &str) -> Vec<&AuditEntry> {
        self.audit_log.iter().filter(|e| e.task_id == task_id).collect()
    }

    /// Orgs that have been denied by policy (attempted but blocked).
    pub fn denied_orgs(&self, task: &FederatedTask) -> Vec<&OrgNode> {
        self.orgs.values().filter(|org| {
            org.online && org.has_capability(&task.required_capability)
            && !task.policy.permits(org)
        }).collect()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_org(id: &str) -> OrgNode {
        OrgNode::new(id, format!("Org {id}"))
            .with_trust(TrustLevel::Trusted)
            .add_capability("code-review")
            .with_budget(5000)
    }

    fn make_task() -> FederatedTask {
        FederatedTask {
            id: "t001".into(),
            required_capability: "code-review".into(),
            payload: "Review PR #42".into(),
            token_cost: 1000,
            policy: FederationPolicy::permissive(),
        }
    }

    fn sim(outputs: &[(&str, &str, bool)]) -> HashMap<OrgId, (String, bool)> {
        outputs.iter().map(|(id, out, ok)| (OrgId::new(*id), (out.to_string(), *ok))).collect()
    }

    #[test]
    fn test_org_has_capability() {
        let org = make_org("a");
        assert!(org.has_capability("code-review"));
        assert!(!org.has_capability("deploy"));
    }

    #[test]
    fn test_org_missing_capability_not_eligible() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        let org = OrgNode::new("a", "A").with_trust(TrustLevel::Trusted).add_capability("deploy");
        orch.register(org);
        let task = make_task(); // requires code-review
        assert!(orch.eligible_orgs(&task).is_empty());
    }

    #[test]
    fn test_offline_org_not_eligible() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a").offline());
        assert!(orch.eligible_orgs(&make_task()).is_empty());
    }

    #[test]
    fn test_insufficient_budget_not_eligible() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a").with_budget(500)); // task costs 1000
        assert!(orch.eligible_orgs(&make_task()).is_empty());
    }

    #[test]
    fn test_eligible_org_selected() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a"));
        assert_eq!(orch.eligible_orgs(&make_task()).len(), 1);
    }

    #[test]
    fn test_route_returns_result() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a"));
        let task = make_task();
        let outputs = sim(&[("a", "LGTM", true)]);
        let results = orch.route(&task, &outputs);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].output, "LGTM");
    }

    #[test]
    fn test_route_no_eligible_returns_empty() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a").offline());
        let results = orch.route(&make_task(), &sim(&[]));
        assert!(results.is_empty());
    }

    #[test]
    fn test_resolve_first_wins() {
        let r1 = FederatedResult { task_id: "t1".into(), org_id: OrgId::new("a"), output: "A".into(), tokens_used: 100, success: true };
        let r2 = FederatedResult { task_id: "t1".into(), org_id: OrgId::new("b"), output: "B".into(), tokens_used: 200, success: true };
        let items = [r1, r2];
        let winner = resolve_conflicts(&items, &ConflictStrategy::FirstWins);
        assert_eq!(winner.unwrap().output, "A");
    }

    #[test]
    fn test_resolve_last_wins() {
        let r1 = FederatedResult { task_id: "t1".into(), org_id: OrgId::new("a"), output: "A".into(), tokens_used: 100, success: true };
        let r2 = FederatedResult { task_id: "t1".into(), org_id: OrgId::new("b"), output: "B".into(), tokens_used: 200, success: true };
        let items = [r1, r2];
        let winner = resolve_conflicts(&items, &ConflictStrategy::LastWins);
        assert_eq!(winner.unwrap().output, "B");
    }

    #[test]
    fn test_resolve_majority_vote() {
        let make = |id: &str, out: &str| FederatedResult {
            task_id: "t1".into(), org_id: OrgId::new(id), output: out.into(), tokens_used: 100, success: true,
        };
        let results = vec![make("a", "LGTM"), make("b", "LGTM"), make("c", "NACK")];
        let winner = resolve_conflicts(&results, &ConflictStrategy::MajorityVote);
        assert_eq!(winner.unwrap().output, "LGTM");
    }

    #[test]
    fn test_resolve_priority_merge_cheapest() {
        let r1 = FederatedResult { task_id: "t1".into(), org_id: OrgId::new("a"), output: "A".into(), tokens_used: 500, success: true };
        let r2 = FederatedResult { task_id: "t1".into(), org_id: OrgId::new("b"), output: "B".into(), tokens_used: 100, success: true };
        let items = [r1, r2];
        let winner = resolve_conflicts(&items, &ConflictStrategy::PriorityMerge);
        assert_eq!(winner.unwrap().output, "B");
    }

    #[test]
    fn test_resolve_all_failed_returns_none() {
        let r = FederatedResult { task_id: "t1".into(), org_id: OrgId::new("a"), output: "err".into(), tokens_used: 0, success: false };
        assert!(resolve_conflicts(&[r], &ConflictStrategy::FirstWins).is_none());
    }

    #[test]
    fn test_resolve_empty_returns_none() {
        assert!(resolve_conflicts(&[], &ConflictStrategy::FirstWins).is_none());
    }

    #[test]
    fn test_audit_log_populated() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a"));
        let task = make_task();
        orch.route(&task, &sim(&[("a", "ok", true)]));
        let entries = orch.audit_for_task("t001");
        assert!(entries.iter().any(|e| e.event == AuditEvent::TaskRouted));
        assert!(entries.iter().any(|e| matches!(e.event, AuditEvent::TaskCompleted { success: true })));
    }

    #[test]
    fn test_policy_deny_list() {
        let mut policy = FederationPolicy::permissive();
        policy.deny_list = vec![OrgId::new("a")];
        let task = FederatedTask { id: "t".into(), required_capability: "code-review".into(), payload: "".into(), token_cost: 100, policy };
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a"));
        assert!(orch.eligible_orgs(&task).is_empty());
    }

    #[test]
    fn test_policy_allow_list_restricts() {
        let mut policy = FederationPolicy::permissive();
        policy.allow_list = vec![OrgId::new("b")];
        let task = FederatedTask { id: "t".into(), required_capability: "code-review".into(), payload: "".into(), token_cost: 100, policy };
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a"));
        assert!(orch.eligible_orgs(&task).is_empty());
    }

    #[test]
    fn test_policy_min_trust_blocks_untrusted() {
        let mut policy = FederationPolicy::permissive();
        policy.min_trust = TrustLevel::Partner;
        let task = FederatedTask { id: "t".into(), required_capability: "code-review".into(), payload: "".into(), token_cost: 100, policy };
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a")); // Trusted < Partner
        assert!(orch.eligible_orgs(&task).is_empty());
    }

    #[test]
    fn test_data_residency_allows_global() {
        let r = DataResidency::global();
        assert!(r.allows("us-east-1"));
        assert!(r.allows("eu-west-1"));
    }

    #[test]
    fn test_data_residency_restricts_region() {
        let r = DataResidency { allowed_regions: vec!["eu-west-1".into()] };
        assert!(r.allows("eu-west-1"));
        assert!(!r.allows("us-east-1"));
    }

    #[test]
    fn test_policy_data_residency_blocks() {
        let mut policy = FederationPolicy::permissive();
        policy.task_region = "us-east-1".into();
        let task = FederatedTask { id: "t".into(), required_capability: "code-review".into(), payload: "".into(), token_cost: 100, policy };
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        let org = make_org("a").with_residency(DataResidency { allowed_regions: vec!["eu-west-1".into()] });
        orch.register(org);
        assert!(orch.eligible_orgs(&task).is_empty());
    }

    #[test]
    fn test_route_and_resolve() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a"));
        let task = make_task();
        let out = orch.route_and_resolve(&task, &sim(&[("a", "Approved", true)]));
        assert_eq!(out, Some("Approved".into()));
    }

    #[test]
    fn test_multiple_orgs_routed() {
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::MajorityVote);
        orch.register(make_org("a"));
        orch.register(make_org("b"));
        orch.register(make_org("c"));
        let task = make_task();
        let outputs = sim(&[("a", "yes", true), ("b", "yes", true), ("c", "no", true)]);
        let out = orch.route_and_resolve(&task, &outputs);
        assert_eq!(out, Some("yes".into()));
    }

    #[test]
    fn test_denied_orgs_tracked() {
        let mut policy = FederationPolicy::permissive();
        policy.deny_list = vec![OrgId::new("a")];
        let task = FederatedTask { id: "t".into(), required_capability: "code-review".into(), payload: "".into(), token_cost: 100, policy };
        let mut orch = FederatedOrchestrator::new(ConflictStrategy::FirstWins);
        orch.register(make_org("a"));
        let denied = orch.denied_orgs(&task);
        assert_eq!(denied.len(), 1);
    }

    #[test]
    fn test_trust_level_ordering() {
        assert!(TrustLevel::Untrusted < TrustLevel::Verified);
        assert!(TrustLevel::Verified < TrustLevel::Trusted);
        assert!(TrustLevel::Trusted < TrustLevel::Partner);
    }

    #[test]
    fn test_org_id_display() {
        let id = OrgId::new("acme-corp");
        assert_eq!(id.to_string(), "acme-corp");
    }
}
