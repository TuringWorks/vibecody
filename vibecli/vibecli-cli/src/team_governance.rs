use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginVisibility {
    Private,
    TeamOnly,
    Organization,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub team_id: String,
    pub visibility: PluginVisibility,
    pub approval_status: ApprovalStatus,
    pub sha256_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub plugin_id: String,
    pub requester: String,
    pub reviewers: Vec<String>,
    pub comments: Vec<String>,
    pub status: ApprovalStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernancePolicy {
    pub team_id: String,
    pub require_approval: bool,
    pub allowed_categories: Vec<String>,
    pub blocked_categories: Vec<String>,
    pub max_plugin_size_mb: u64,
    pub require_sha_pin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceIssue {
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub action: String,
    pub actor: String,
    pub target: String,
    pub timestamp: String,
}

pub struct TeamGovernanceManager {
    plugins: HashMap<String, TeamPlugin>,
    approval_requests: HashMap<String, ApprovalRequest>,
    policies: HashMap<String, GovernancePolicy>,
    audit_entries: Vec<AuditEntry>,
    next_id: u64,
}

impl TeamGovernanceManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            approval_requests: HashMap::new(),
            policies: HashMap::new(),
            audit_entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn register_plugin(&mut self, mut plugin: TeamPlugin) -> String {
        let id = format!("plugin-{}", self.next_id);
        self.next_id += 1;
        plugin.id = id.clone();
        plugin.approval_status = ApprovalStatus::Pending;

        self.audit_entries.push(AuditEntry {
            action: "register_plugin".to_string(),
            actor: plugin.author.clone(),
            target: id.clone(),
            timestamp: current_timestamp(),
        });

        self.plugins.insert(id.clone(), plugin);
        id
    }

    pub fn submit_for_approval(&mut self, plugin_id: &str, reviewers: Vec<String>) -> Result<(), String> {
        let plugin = self.plugins.get(plugin_id).ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        let request = ApprovalRequest {
            plugin_id: plugin_id.to_string(),
            requester: plugin.author.clone(),
            reviewers: reviewers.clone(),
            comments: Vec::new(),
            status: ApprovalStatus::Pending,
            created_at: current_timestamp(),
        };

        self.audit_entries.push(AuditEntry {
            action: "submit_for_approval".to_string(),
            actor: plugin.author.clone(),
            target: plugin_id.to_string(),
            timestamp: current_timestamp(),
        });

        self.approval_requests.insert(plugin_id.to_string(), request);
        Ok(())
    }

    pub fn approve_plugin(&mut self, plugin_id: &str, reviewer: &str) -> Result<(), String> {
        let request = self.approval_requests.get_mut(plugin_id)
            .ok_or_else(|| format!("No approval request for {}", plugin_id))?;

        if !request.reviewers.contains(&reviewer.to_string()) {
            return Err(format!("{} is not an authorized reviewer", reviewer));
        }

        request.status = ApprovalStatus::Approved;
        request.comments.push(format!("Approved by {}", reviewer));

        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.approval_status = ApprovalStatus::Approved;
        }

        self.audit_entries.push(AuditEntry {
            action: "approve_plugin".to_string(),
            actor: reviewer.to_string(),
            target: plugin_id.to_string(),
            timestamp: current_timestamp(),
        });

        Ok(())
    }

    pub fn reject_plugin(&mut self, plugin_id: &str, reviewer: &str, reason: &str) -> Result<(), String> {
        let request = self.approval_requests.get_mut(plugin_id)
            .ok_or_else(|| format!("No approval request for {}", plugin_id))?;

        if !request.reviewers.contains(&reviewer.to_string()) {
            return Err(format!("{} is not an authorized reviewer", reviewer));
        }

        request.status = ApprovalStatus::Rejected;
        request.comments.push(format!("Rejected by {}: {}", reviewer, reason));

        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.approval_status = ApprovalStatus::Rejected;
        }

        self.audit_entries.push(AuditEntry {
            action: "reject_plugin".to_string(),
            actor: reviewer.to_string(),
            target: plugin_id.to_string(),
            timestamp: current_timestamp(),
        });

        Ok(())
    }

    pub fn deprecate_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        let plugin = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| format!("Plugin {} not found", plugin_id))?;

        plugin.approval_status = ApprovalStatus::Deprecated;

        self.audit_entries.push(AuditEntry {
            action: "deprecate_plugin".to_string(),
            actor: "system".to_string(),
            target: plugin_id.to_string(),
            timestamp: current_timestamp(),
        });

        Ok(())
    }

    pub fn set_policy(&mut self, team_id: &str, policy: GovernancePolicy) {
        self.audit_entries.push(AuditEntry {
            action: "set_policy".to_string(),
            actor: "admin".to_string(),
            target: team_id.to_string(),
            timestamp: current_timestamp(),
        });

        self.policies.insert(team_id.to_string(), policy);
    }

    pub fn get_policy(&self, team_id: &str) -> GovernancePolicy {
        self.policies.get(team_id).cloned().unwrap_or_else(|| GovernancePolicy {
            team_id: team_id.to_string(),
            require_approval: true,
            allowed_categories: Vec::new(),
            blocked_categories: Vec::new(),
            max_plugin_size_mb: 50,
            require_sha_pin: false,
        })
    }

    pub fn list_team_plugins(&self, team_id: &str) -> Vec<TeamPlugin> {
        self.plugins.values()
            .filter(|p| p.team_id == team_id)
            .cloned()
            .collect()
    }

    pub fn list_pending_approvals(&self, team_id: &str) -> Vec<ApprovalRequest> {
        let team_plugin_ids: Vec<String> = self.plugins.values()
            .filter(|p| p.team_id == team_id)
            .map(|p| p.id.clone())
            .collect();

        self.approval_requests.values()
            .filter(|r| team_plugin_ids.contains(&r.plugin_id) && r.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }

    pub fn check_compliance(&self, plugin_id: &str) -> Vec<ComplianceIssue> {
        let mut issues = Vec::new();

        let plugin = match self.plugins.get(plugin_id) {
            Some(p) => p,
            None => {
                issues.push(ComplianceIssue {
                    description: format!("Plugin {} not found", plugin_id),
                    severity: "critical".to_string(),
                });
                return issues;
            }
        };

        let policy = self.get_policy(&plugin.team_id);

        if policy.require_sha_pin && plugin.sha256_hash.is_empty() {
            issues.push(ComplianceIssue {
                description: "Plugin missing required SHA-256 hash pin".to_string(),
                severity: "high".to_string(),
            });
        }

        if policy.require_approval && plugin.approval_status == ApprovalStatus::Pending {
            issues.push(ComplianceIssue {
                description: "Plugin requires approval but is still pending".to_string(),
                severity: "medium".to_string(),
            });
        }

        if plugin.description.is_empty() {
            issues.push(ComplianceIssue {
                description: "Plugin missing description".to_string(),
                severity: "low".to_string(),
            });
        }

        if plugin.version.is_empty() {
            issues.push(ComplianceIssue {
                description: "Plugin missing version".to_string(),
                severity: "medium".to_string(),
            });
        }

        if plugin.name.is_empty() {
            issues.push(ComplianceIssue {
                description: "Plugin missing name".to_string(),
                severity: "high".to_string(),
            });
        }

        for blocked in &policy.blocked_categories {
            if plugin.description.to_lowercase().contains(&blocked.to_lowercase()) {
                issues.push(ComplianceIssue {
                    description: format!("Plugin matches blocked category: {}", blocked),
                    severity: "high".to_string(),
                });
            }
        }

        if plugin.approval_status == ApprovalStatus::Deprecated {
            issues.push(ComplianceIssue {
                description: "Plugin is deprecated".to_string(),
                severity: "medium".to_string(),
            });
        }

        issues
    }

    pub fn audit_log(&self, team_id: &str) -> Vec<AuditEntry> {
        let team_plugin_ids: Vec<String> = self.plugins.values()
            .filter(|p| p.team_id == team_id)
            .map(|p| p.id.clone())
            .collect();

        self.audit_entries.iter()
            .filter(|e| e.target == team_id || team_plugin_ids.contains(&e.target))
            .cloned()
            .collect()
    }
}

fn current_timestamp() -> String {
    "2026-03-09T00:00:00Z".to_string()
}

fn make_test_plugin(team_id: &str) -> TeamPlugin {
    TeamPlugin {
        id: String::new(),
        name: "test-plugin".to_string(),
        version: "1.0.0".to_string(),
        author: "alice".to_string(),
        description: "A test plugin".to_string(),
        team_id: team_id.to_string(),
        visibility: PluginVisibility::TeamOnly,
        approval_status: ApprovalStatus::Pending,
        sha256_hash: "abc123def456".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_plugin() {
        let mut mgr = TeamGovernanceManager::new();
        let plugin = make_test_plugin("team-1");
        let id = mgr.register_plugin(plugin);
        assert_eq!(id, "plugin-1");
        assert!(mgr.plugins.contains_key(&id));
    }

    #[test]
    fn test_register_multiple_plugins() {
        let mut mgr = TeamGovernanceManager::new();
        let id1 = mgr.register_plugin(make_test_plugin("team-1"));
        let id2 = mgr.register_plugin(make_test_plugin("team-1"));
        assert_ne!(id1, id2);
        assert_eq!(mgr.plugins.len(), 2);
    }

    #[test]
    fn test_registered_plugin_starts_pending() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        let plugin = mgr.plugins.get(&id).expect("plugin should exist");
        assert_eq!(plugin.approval_status, ApprovalStatus::Pending);
    }

    #[test]
    fn test_submit_for_approval() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        let result = mgr.submit_for_approval(&id, vec!["bob".to_string()]);
        assert!(result.is_ok());
        assert!(mgr.approval_requests.contains_key(&id));
    }

    #[test]
    fn test_submit_for_approval_unknown_plugin() {
        let mut mgr = TeamGovernanceManager::new();
        let result = mgr.submit_for_approval("nonexistent", vec!["bob".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_approve_plugin() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        mgr.submit_for_approval(&id, vec!["bob".to_string()]).expect("submit ok");
        let result = mgr.approve_plugin(&id, "bob");
        assert!(result.is_ok());
        let plugin = mgr.plugins.get(&id).expect("plugin should exist");
        assert_eq!(plugin.approval_status, ApprovalStatus::Approved);
    }

    #[test]
    fn test_approve_unauthorized_reviewer() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        mgr.submit_for_approval(&id, vec!["bob".to_string()]).expect("submit ok");
        let result = mgr.approve_plugin(&id, "eve");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_plugin() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        mgr.submit_for_approval(&id, vec!["bob".to_string()]).expect("submit ok");
        let result = mgr.reject_plugin(&id, "bob", "security concern");
        assert!(result.is_ok());
        let plugin = mgr.plugins.get(&id).expect("plugin should exist");
        assert_eq!(plugin.approval_status, ApprovalStatus::Rejected);
    }

    #[test]
    fn test_reject_adds_comment() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        mgr.submit_for_approval(&id, vec!["bob".to_string()]).expect("submit ok");
        mgr.reject_plugin(&id, "bob", "bad code").expect("reject ok");
        let req = mgr.approval_requests.get(&id).expect("request exists");
        assert!(req.comments.iter().any(|c| c.contains("bad code")));
    }

    #[test]
    fn test_reject_unauthorized_reviewer() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        mgr.submit_for_approval(&id, vec!["bob".to_string()]).expect("submit ok");
        let result = mgr.reject_plugin(&id, "eve", "nope");
        assert!(result.is_err());
    }

    #[test]
    fn test_deprecate_plugin() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        let result = mgr.deprecate_plugin(&id);
        assert!(result.is_ok());
        let plugin = mgr.plugins.get(&id).expect("plugin should exist");
        assert_eq!(plugin.approval_status, ApprovalStatus::Deprecated);
    }

    #[test]
    fn test_deprecate_unknown_plugin() {
        let mut mgr = TeamGovernanceManager::new();
        let result = mgr.deprecate_plugin("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_and_get_policy() {
        let mut mgr = TeamGovernanceManager::new();
        let policy = GovernancePolicy {
            team_id: "team-1".to_string(),
            require_approval: true,
            allowed_categories: vec!["linting".to_string()],
            blocked_categories: vec!["crypto".to_string()],
            max_plugin_size_mb: 100,
            require_sha_pin: true,
        };
        mgr.set_policy("team-1", policy);
        let retrieved = mgr.get_policy("team-1");
        assert_eq!(retrieved.require_approval, true);
        assert_eq!(retrieved.max_plugin_size_mb, 100);
        assert_eq!(retrieved.require_sha_pin, true);
    }

    #[test]
    fn test_default_policy() {
        let mgr = TeamGovernanceManager::new();
        let policy = mgr.get_policy("unknown-team");
        assert_eq!(policy.team_id, "unknown-team");
        assert_eq!(policy.require_approval, true);
        assert_eq!(policy.max_plugin_size_mb, 50);
    }

    #[test]
    fn test_list_team_plugins() {
        let mut mgr = TeamGovernanceManager::new();
        mgr.register_plugin(make_test_plugin("team-1"));
        mgr.register_plugin(make_test_plugin("team-1"));
        mgr.register_plugin(make_test_plugin("team-2"));
        let team1_plugins = mgr.list_team_plugins("team-1");
        assert_eq!(team1_plugins.len(), 2);
    }

    #[test]
    fn test_list_team_plugins_empty() {
        let mgr = TeamGovernanceManager::new();
        let plugins = mgr.list_team_plugins("team-1");
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_list_pending_approvals() {
        let mut mgr = TeamGovernanceManager::new();
        let id1 = mgr.register_plugin(make_test_plugin("team-1"));
        let id2 = mgr.register_plugin(make_test_plugin("team-1"));
        mgr.submit_for_approval(&id1, vec!["bob".to_string()]).expect("ok");
        mgr.submit_for_approval(&id2, vec!["bob".to_string()]).expect("ok");
        mgr.approve_plugin(&id1, "bob").expect("ok");
        let pending = mgr.list_pending_approvals("team-1");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].plugin_id, id2);
    }

    #[test]
    fn test_compliance_missing_sha() {
        let mut mgr = TeamGovernanceManager::new();
        let policy = GovernancePolicy {
            team_id: "team-1".to_string(),
            require_approval: false,
            allowed_categories: Vec::new(),
            blocked_categories: Vec::new(),
            max_plugin_size_mb: 50,
            require_sha_pin: true,
        };
        mgr.set_policy("team-1", policy);

        let mut plugin = make_test_plugin("team-1");
        plugin.sha256_hash = String::new();
        let id = mgr.register_plugin(plugin);
        let issues = mgr.check_compliance(&id);
        assert!(issues.iter().any(|i| i.description.contains("SHA-256")));
    }

    #[test]
    fn test_compliance_blocked_category() {
        let mut mgr = TeamGovernanceManager::new();
        let policy = GovernancePolicy {
            team_id: "team-1".to_string(),
            require_approval: false,
            allowed_categories: Vec::new(),
            blocked_categories: vec!["crypto".to_string()],
            max_plugin_size_mb: 50,
            require_sha_pin: false,
        };
        mgr.set_policy("team-1", policy);

        let mut plugin = make_test_plugin("team-1");
        plugin.description = "A crypto mining plugin".to_string();
        let id = mgr.register_plugin(plugin);
        let issues = mgr.check_compliance(&id);
        assert!(issues.iter().any(|i| i.description.contains("blocked category")));
    }

    #[test]
    fn test_compliance_pending_approval_required() {
        let mut mgr = TeamGovernanceManager::new();
        let policy = GovernancePolicy {
            team_id: "team-1".to_string(),
            require_approval: true,
            allowed_categories: Vec::new(),
            blocked_categories: Vec::new(),
            max_plugin_size_mb: 50,
            require_sha_pin: false,
        };
        mgr.set_policy("team-1", policy);

        let id = mgr.register_plugin(make_test_plugin("team-1"));
        let issues = mgr.check_compliance(&id);
        assert!(issues.iter().any(|i| i.description.contains("requires approval")));
    }

    #[test]
    fn test_compliance_unknown_plugin() {
        let mgr = TeamGovernanceManager::new();
        let issues = mgr.check_compliance("nonexistent");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, "critical");
    }

    #[test]
    fn test_compliance_deprecated_plugin() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        mgr.deprecate_plugin(&id).expect("deprecate ok");
        let issues = mgr.check_compliance(&id);
        assert!(issues.iter().any(|i| i.description.contains("deprecated")));
    }

    #[test]
    fn test_audit_log() {
        let mut mgr = TeamGovernanceManager::new();
        let id = mgr.register_plugin(make_test_plugin("team-1"));
        mgr.submit_for_approval(&id, vec!["bob".to_string()]).expect("ok");
        mgr.approve_plugin(&id, "bob").expect("ok");
        let log = mgr.audit_log("team-1");
        assert!(log.len() >= 3);
        let actions: Vec<&str> = log.iter().map(|e| e.action.as_str()).collect();
        assert!(actions.contains(&"register_plugin"));
        assert!(actions.contains(&"submit_for_approval"));
        assert!(actions.contains(&"approve_plugin"));
    }

    #[test]
    fn test_audit_log_policy_change() {
        let mut mgr = TeamGovernanceManager::new();
        let policy = GovernancePolicy {
            team_id: "team-1".to_string(),
            require_approval: true,
            allowed_categories: Vec::new(),
            blocked_categories: Vec::new(),
            max_plugin_size_mb: 50,
            require_sha_pin: false,
        };
        mgr.set_policy("team-1", policy);
        let log = mgr.audit_log("team-1");
        assert!(log.iter().any(|e| e.action == "set_policy"));
    }

    #[test]
    fn test_visibility_enum_variants() {
        let private = PluginVisibility::Private;
        let team = PluginVisibility::TeamOnly;
        let org = PluginVisibility::Organization;
        let public = PluginVisibility::Public;
        assert_ne!(private, team);
        assert_ne!(org, public);
    }

    #[test]
    fn test_plugin_visibility_filtering() {
        let mut mgr = TeamGovernanceManager::new();

        let mut p1 = make_test_plugin("team-1");
        p1.visibility = PluginVisibility::Private;
        mgr.register_plugin(p1);

        let mut p2 = make_test_plugin("team-1");
        p2.visibility = PluginVisibility::Public;
        mgr.register_plugin(p2);

        let all = mgr.list_team_plugins("team-1");
        let public_only: Vec<_> = all.iter()
            .filter(|p| p.visibility == PluginVisibility::Public)
            .collect();
        assert_eq!(public_only.len(), 1);
        let private_only: Vec<_> = all.iter()
            .filter(|p| p.visibility == PluginVisibility::Private)
            .collect();
        assert_eq!(private_only.len(), 1);
    }

    #[test]
    fn test_full_approval_workflow() {
        let mut mgr = TeamGovernanceManager::new();

        let policy = GovernancePolicy {
            team_id: "team-1".to_string(),
            require_approval: true,
            allowed_categories: Vec::new(),
            blocked_categories: Vec::new(),
            max_plugin_size_mb: 50,
            require_sha_pin: true,
        };
        mgr.set_policy("team-1", policy);

        let id = mgr.register_plugin(make_test_plugin("team-1"));

        // Pending — should have compliance issues
        let issues = mgr.check_compliance(&id);
        assert!(issues.iter().any(|i| i.description.contains("requires approval")));

        // Submit and approve
        mgr.submit_for_approval(&id, vec!["reviewer1".to_string()]).expect("ok");
        mgr.approve_plugin(&id, "reviewer1").expect("ok");

        // After approval, pending-approval issue should be gone
        let issues = mgr.check_compliance(&id);
        assert!(!issues.iter().any(|i| i.description.contains("requires approval")));

        // Audit trail complete
        let log = mgr.audit_log("team-1");
        assert!(log.len() >= 3);
    }
}
