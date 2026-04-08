#![allow(dead_code)]
//! Policy-as-code authorization engine — RBAC/ABAC policy evaluation.
//!
//! Inspired by Cerbos (open-source authorization). Provides a composable
//! policy engine with derived roles, condition evaluation, YAML serialization,
//! conflict detection, audit logging, and a built-in test harness.
//!
//! # Architecture
//!
//! ```text
//! CheckRequest (principal + resource + action)
//!   → PolicyEngine::check()
//!     ├─ DerivedRoleSet::resolve_roles()  — expand principal roles
//!     ├─ Find matching policies for resource
//!     ├─ ConditionEvaluator::evaluate()   — ABAC condition checks
//!     ├─ Apply rule priority + default-deny
//!     └─ AuditEntry recorded
//!   → CheckResult { effect, matched_rule, policy_id, ... }
//! ```

use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::{Deserialize, Serialize};

// ─── Enums ───────────────────────────────────────────────────────────────────

/// The effect of a policy rule evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Allow,
    Deny,
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow => write!(f, "EFFECT_ALLOW"),
            Self::Deny => write!(f, "EFFECT_DENY"),
        }
    }
}

/// Classification of a policy document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyType {
    ResourcePolicy,
    PrincipalPolicy,
    DerivedRoles,
    ExportVariables,
}

impl std::fmt::Display for PolicyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ResourcePolicy => write!(f, "resource"),
            Self::PrincipalPolicy => write!(f, "principal"),
            Self::DerivedRoles => write!(f, "derived_roles"),
            Self::ExportVariables => write!(f, "export_variables"),
        }
    }
}

/// Operators for condition evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionOperator {
    Eq,
    NotEq,
    In,
    NotIn,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    Gt,
    Lt,
    Gte,
    Lte,
    And,
    Or,
    Not,
}

/// Value types that conditions can compare against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionValue {
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<String>),
    Null,
}

// ─── Core data structures ────────────────────────────────────────────────────

/// A single condition within a policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub match_expr: String,
    pub operator: ConditionOperator,
    pub value: ConditionValue,
}

/// A rule within a policy — binds actions to effects with optional conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: String,
    pub name: String,
    pub actions: Vec<String>,
    pub effect: Effect,
    pub conditions: Vec<Condition>,
    pub roles: Vec<String>,
    pub derived_roles: Vec<String>,
    pub priority: u32,
}

/// A policy document that governs access to a particular resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub policy_type: PolicyType,
    pub resource: String,
    pub rules: Vec<PolicyRule>,
    pub variables: HashMap<String, String>,
    pub disabled: bool,
}

/// Identity making a request — carries roles and arbitrary attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub id: String,
    pub roles: Vec<String>,
    pub attributes: HashMap<String, String>,
}

/// The target resource being accessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub kind: String,
    pub id: String,
    pub attributes: HashMap<String, String>,
    pub policy_version: String,
}

/// A single authorization check request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequest {
    pub principal: Principal,
    pub resource: Resource,
    pub action: String,
    pub aux_data: HashMap<String, String>,
}

/// Result of evaluating a check request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub request_id: String,
    pub resource_id: String,
    pub action: String,
    pub effect: Effect,
    pub matched_rule: Option<String>,
    pub evaluation_duration_us: u64,
    pub policy_id: String,
}

/// A derived role — dynamically computed from parent roles + conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedRole {
    pub name: String,
    pub parent_roles: Vec<String>,
    pub conditions: Vec<Condition>,
}

/// Audit trail entry for every check performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub request: CheckRequest,
    pub result: CheckResult,
    pub policy_chain: Vec<String>,
}

// ─── Derived role resolution ─────────────────────────────────────────────────

/// A set of derived role definitions that can expand a principal's effective roles.
#[derive(Debug, Clone, Default)]
pub struct DerivedRoleSet {
    roles: Vec<DerivedRole>,
}

impl DerivedRoleSet {
    pub fn new() -> Self {
        Self { roles: Vec::new() }
    }

    pub fn add_role(&mut self, role: DerivedRole) {
        self.roles.push(role);
    }

    /// Resolve which derived roles a principal qualifies for given a resource.
    pub fn resolve_roles(&self, principal: &Principal, resource: &Resource) -> Vec<String> {
        let evaluator = ConditionEvaluator;
        let empty_aux = HashMap::new();
        let mut derived = Vec::new();

        for dr in &self.roles {
            // Principal must hold at least one parent role.
            let has_parent = dr.parent_roles.iter().any(|pr| principal.roles.contains(pr));
            if !has_parent {
                continue;
            }

            // All conditions must pass.
            let conditions_pass = dr.conditions.iter().all(|c| {
                evaluator.evaluate(c, principal, resource, &empty_aux)
            });
            if conditions_pass {
                derived.push(dr.name.clone());
            }
        }

        derived
    }
}

// ─── Condition evaluator ─────────────────────────────────────────────────────

/// Evaluates conditions against principal/resource attributes and auxiliary data.
pub struct ConditionEvaluator;

impl ConditionEvaluator {
    /// Resolve a match expression to its string value.
    ///
    /// Supported prefixes:
    /// - `P.attr.<key>` — principal attribute
    /// - `R.attr.<key>` — resource attribute
    /// - `R.kind`       — resource kind
    /// - `R.id`         — resource id
    /// - `P.id`         — principal id
    /// - `aux.<key>`    — auxiliary data
    fn resolve_expr(
        expr: &str,
        principal: &Principal,
        resource: &Resource,
        aux: &HashMap<String, String>,
    ) -> Option<String> {
        if let Some(key) = expr.strip_prefix("P.attr.") {
            return principal.attributes.get(key).cloned();
        }
        if let Some(key) = expr.strip_prefix("R.attr.") {
            return resource.attributes.get(key).cloned();
        }
        if expr == "R.kind" {
            return Some(resource.kind.clone());
        }
        if expr == "R.id" {
            return Some(resource.id.clone());
        }
        if expr == "P.id" {
            return Some(principal.id.clone());
        }
        if let Some(key) = expr.strip_prefix("aux.") {
            return aux.get(key).cloned();
        }
        // Literal value fallback
        Some(expr.to_string())
    }

    /// Evaluate a single condition.
    pub fn evaluate(
        &self,
        condition: &Condition,
        principal: &Principal,
        resource: &Resource,
        aux: &HashMap<String, String>,
    ) -> bool {
        let resolved = Self::resolve_expr(&condition.match_expr, principal, resource, aux);

        let is_none = resolved.is_none();
        let is_some = resolved.is_some();

        match &condition.operator {
            ConditionOperator::Eq => {
                let lhs = resolved.clone().unwrap_or_default();
                match &condition.value {
                    ConditionValue::String(s) => lhs == *s,
                    ConditionValue::Number(n) => lhs.parse::<f64>().is_ok_and(|v| (v - n).abs() < f64::EPSILON),
                    ConditionValue::Bool(b) => lhs.parse::<bool>() == Ok(*b),
                    ConditionValue::Null => is_none,
                    ConditionValue::List(_) => false,
                }
            }
            ConditionOperator::NotEq => {
                let lhs = resolved.clone().unwrap_or_default();
                match &condition.value {
                    ConditionValue::String(s) => lhs != *s,
                    ConditionValue::Number(n) => lhs.parse::<f64>().map_or(true, |v| (v - n).abs() >= f64::EPSILON),
                    ConditionValue::Bool(b) => lhs.parse::<bool>() != Ok(*b),
                    ConditionValue::Null => is_some,
                    ConditionValue::List(_) => true,
                }
            }
            ConditionOperator::In => {
                let lhs = resolved.unwrap_or_default();
                match &condition.value {
                    ConditionValue::List(items) => items.contains(&lhs),
                    _ => false,
                }
            }
            ConditionOperator::NotIn => {
                let lhs = resolved.unwrap_or_default();
                match &condition.value {
                    ConditionValue::List(items) => !items.contains(&lhs),
                    _ => true,
                }
            }
            ConditionOperator::Contains => {
                let lhs = resolved.unwrap_or_default();
                match &condition.value {
                    ConditionValue::String(s) => lhs.contains(s.as_str()),
                    _ => false,
                }
            }
            ConditionOperator::StartsWith => {
                let lhs = resolved.unwrap_or_default();
                match &condition.value {
                    ConditionValue::String(s) => lhs.starts_with(s.as_str()),
                    _ => false,
                }
            }
            ConditionOperator::EndsWith => {
                let lhs = resolved.unwrap_or_default();
                match &condition.value {
                    ConditionValue::String(s) => lhs.ends_with(s.as_str()),
                    _ => false,
                }
            }
            ConditionOperator::Regex => {
                let lhs = resolved.unwrap_or_default();
                match &condition.value {
                    ConditionValue::String(pattern) => {
                        Regex::new(pattern).is_ok_and(|re| re.is_match(&lhs))
                    }
                    _ => false,
                }
            }
            ConditionOperator::Gt => {
                let lhs = resolved.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                match &condition.value {
                    ConditionValue::Number(n) => lhs > *n,
                    _ => false,
                }
            }
            ConditionOperator::Lt => {
                let lhs = resolved.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                match &condition.value {
                    ConditionValue::Number(n) => lhs < *n,
                    _ => false,
                }
            }
            ConditionOperator::Gte => {
                let lhs = resolved.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                match &condition.value {
                    ConditionValue::Number(n) => lhs >= *n,
                    _ => false,
                }
            }
            ConditionOperator::Lte => {
                let lhs = resolved.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                match &condition.value {
                    ConditionValue::Number(n) => lhs <= *n,
                    _ => false,
                }
            }
            ConditionOperator::And => {
                // For And, the value should be Null; match_expr is unused.
                // And is evaluated at the rule level (all conditions must pass).
                true
            }
            ConditionOperator::Or => {
                // Or is evaluated at the rule level as a marker.
                true
            }
            ConditionOperator::Not => {
                // Negate: resolve the expression and compare to the value.
                let lhs = resolved.unwrap_or_default();
                match &condition.value {
                    ConditionValue::Bool(b) => lhs.parse::<bool>().is_ok_and(|v| v != *b),
                    ConditionValue::String(s) => lhs != *s,
                    _ => false,
                }
            }
        }
    }
}

// ─── Policy Engine ───────────────────────────────────────────────────────────

/// Core authorization engine. Stores policies, evaluates checks, records audits.
#[derive(Debug)]
pub struct PolicyEngine {
    policies: Vec<Policy>,
    derived_roles: DerivedRoleSet,
    audit_log: Vec<AuditEntry>,
    request_counter: u64,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            derived_roles: DerivedRoleSet::new(),
            audit_log: Vec::new(),
            request_counter: 0,
        }
    }

    /// Validate a policy for correctness. Returns a list of error messages.
    pub fn validate_policy(policy: &Policy) -> Vec<String> {
        let mut errors = Vec::new();

        if policy.id.is_empty() {
            errors.push("Policy id must not be empty".to_string());
        }
        if policy.name.is_empty() {
            errors.push("Policy name must not be empty".to_string());
        }
        if policy.resource.is_empty() {
            errors.push("Policy resource must not be empty".to_string());
        }
        if policy.version.is_empty() {
            errors.push("Policy version must not be empty".to_string());
        }
        if policy.rules.is_empty() {
            errors.push("Policy must have at least one rule".to_string());
        }

        for (i, rule) in policy.rules.iter().enumerate() {
            if rule.id.is_empty() {
                errors.push(format!("Rule {} id must not be empty", i));
            }
            if rule.actions.is_empty() {
                errors.push(format!("Rule '{}' must have at least one action", rule.id));
            }
            if rule.roles.is_empty() && rule.derived_roles.is_empty() {
                errors.push(format!(
                    "Rule '{}' must target at least one role or derived role",
                    rule.id
                ));
            }
            // Validate regex conditions
            for cond in &rule.conditions {
                if cond.operator == ConditionOperator::Regex {
                    if let ConditionValue::String(pattern) = &cond.value {
                        if Regex::new(pattern).is_err() {
                            errors.push(format!(
                                "Rule '{}' has invalid regex pattern: {}",
                                rule.id, pattern
                            ));
                        }
                    }
                }
            }
        }

        // Check for duplicate rule IDs.
        let mut seen_ids = std::collections::HashSet::new();
        for rule in &policy.rules {
            if !seen_ids.insert(&rule.id) {
                errors.push(format!("Duplicate rule id: {}", rule.id));
            }
        }

        errors
    }

    /// Add a policy after validation. Returns error if validation fails.
    pub fn add_policy(&mut self, policy: Policy) -> Result<(), String> {
        let errors = Self::validate_policy(&policy);
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        // Check for duplicate policy ID.
        if self.policies.iter().any(|p| p.id == policy.id) {
            return Err(format!("Policy with id '{}' already exists", policy.id));
        }
        self.policies.push(policy);
        Ok(())
    }

    /// Remove a policy by ID.
    pub fn remove_policy(&mut self, id: &str) -> Result<(), String> {
        let idx = self.policies.iter().position(|p| p.id == id);
        match idx {
            Some(i) => {
                self.policies.remove(i);
                Ok(())
            }
            None => Err(format!("Policy '{}' not found", id)),
        }
    }

    /// Get a policy by ID.
    pub fn get_policy(&self, id: &str) -> Option<&Policy> {
        self.policies.iter().find(|p| p.id == id)
    }

    /// List all policies.
    pub fn list_policies(&self) -> Vec<&Policy> {
        self.policies.iter().collect()
    }

    /// Add a derived role definition to the engine.
    pub fn add_derived_role(&mut self, role: DerivedRole) {
        self.derived_roles.add_role(role);
    }

    /// Evaluate a single authorization check.
    pub fn check(&mut self, request: &CheckRequest) -> CheckResult {
        let start = Instant::now();
        self.request_counter += 1;
        let request_id = format!("req-{}", self.request_counter);

        // Resolve derived roles for this principal + resource.
        let derived = self.derived_roles.resolve_roles(&request.principal, &request.resource);

        // All effective roles = direct roles + derived roles.
        let mut effective_roles: Vec<String> = request.principal.roles.clone();
        effective_roles.extend(derived);

        let evaluator = ConditionEvaluator;
        let mut best_match: Option<(Effect, String, String, u32)> = None; // (effect, rule_id, policy_id, priority)

        // Find matching policies for the resource.
        let matching_policies: Vec<&Policy> = self
            .policies
            .iter()
            .filter(|p| !p.disabled && Self::resource_matches(&p.resource, &request.resource.kind))
            .collect();

        let mut policy_chain: Vec<String> = Vec::new();

        for policy in &matching_policies {
            policy_chain.push(policy.id.clone());

            for rule in &policy.rules {
                // Check action match (supports wildcard "*").
                let action_match = rule.actions.iter().any(|a| a == "*" || a == &request.action);
                if !action_match {
                    continue;
                }

                // Check role match.
                let role_match = rule.roles.iter().any(|r| r == "*" || effective_roles.contains(r))
                    || rule.derived_roles.iter().any(|dr| effective_roles.contains(dr));
                if !role_match {
                    continue;
                }

                // Evaluate conditions.
                let conditions_pass = rule.conditions.iter().all(|c| {
                    evaluator.evaluate(c, &request.principal, &request.resource, &request.aux_data)
                });
                if !conditions_pass {
                    continue;
                }

                // Select the rule with highest priority (lowest number) or first Deny.
                match &best_match {
                    None => {
                        best_match = Some((
                            rule.effect.clone(),
                            rule.id.clone(),
                            policy.id.clone(),
                            rule.priority,
                        ));
                    }
                    Some((_, _, _, current_priority)) => {
                        // Lower priority number = higher precedence; at equal priority, Deny wins.
                        if rule.priority < *current_priority
                            || (rule.priority == *current_priority && rule.effect == Effect::Deny)
                        {
                            best_match = Some((
                                rule.effect.clone(),
                                rule.id.clone(),
                                policy.id.clone(),
                                rule.priority,
                            ));
                        }
                    }
                }
            }
        }

        let elapsed_us = start.elapsed().as_micros() as u64;

        let result = match best_match {
            Some((effect, rule_id, policy_id, _)) => CheckResult {
                request_id: request_id.clone(),
                resource_id: request.resource.id.clone(),
                action: request.action.clone(),
                effect,
                matched_rule: Some(rule_id),
                evaluation_duration_us: elapsed_us,
                policy_id,
            },
            None => CheckResult {
                request_id: request_id.clone(),
                resource_id: request.resource.id.clone(),
                action: request.action.clone(),
                effect: Effect::Deny, // Default deny
                matched_rule: None,
                evaluation_duration_us: elapsed_us,
                policy_id: String::new(),
            },
        };

        // Record audit entry.
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.audit_log.push(AuditEntry {
            timestamp,
            request: request.clone(),
            result: result.clone(),
            policy_chain,
        });

        result
    }

    /// Evaluate a batch of check requests.
    pub fn check_batch(&mut self, requests: &[CheckRequest]) -> Vec<CheckResult> {
        requests.iter().map(|r| self.check(r)).collect()
    }

    /// Get the full audit log.
    pub fn get_audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    /// Clear the audit log.
    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }

    /// Check whether a policy resource pattern matches a resource kind.
    /// Supports exact match and glob-style trailing wildcard (e.g., `document:*`).
    fn resource_matches(pattern: &str, kind: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if let Some(prefix) = pattern.strip_suffix(":*") {
            return kind.starts_with(prefix);
        }
        pattern == kind
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Policy testing harness ──────────────────────────────────────────────────

/// A single test case for policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyTestCase {
    pub name: String,
    pub request: CheckRequest,
    pub expected_effect: Effect,
    pub description: String,
}

/// A suite of test cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyTestSuite {
    pub name: String,
    pub tests: Vec<PolicyTestCase>,
}

/// Result of running a single test case.
#[derive(Debug, Clone)]
pub struct PolicyTestResult {
    pub test_name: String,
    pub passed: bool,
    pub expected: Effect,
    pub actual: Effect,
    pub message: String,
}

/// Runs test suites against a policy engine.
pub struct PolicyTester;

impl PolicyTester {
    pub fn run_suite(engine: &mut PolicyEngine, suite: &PolicyTestSuite) -> Vec<PolicyTestResult> {
        let mut results = Vec::new();

        for tc in &suite.tests {
            let check_result = engine.check(&tc.request);
            let passed = check_result.effect == tc.expected_effect;
            let message = if passed {
                format!("PASS: {}", tc.description)
            } else {
                format!(
                    "FAIL: {} — expected {}, got {}",
                    tc.description, tc.expected_effect, check_result.effect
                )
            };

            results.push(PolicyTestResult {
                test_name: tc.name.clone(),
                passed,
                expected: tc.expected_effect.clone(),
                actual: check_result.effect,
                message,
            });
        }

        results
    }
}

// ─── YAML serialization ─────────────────────────────────────────────────────

/// Serialize and deserialize policies to/from YAML.
pub struct PolicySerializer;

impl PolicySerializer {
    /// Serialize a policy to YAML.
    pub fn to_yaml(policy: &Policy) -> String {
        serde_yaml::to_string(policy).unwrap_or_else(|e| format!("# Serialization error: {}", e))
    }

    /// Deserialize a policy from YAML.
    pub fn from_yaml(yaml: &str) -> Result<Policy, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("YAML parse error: {}", e))
    }

    /// Generate a starter policy template for a given resource.
    pub fn generate_template(resource: &str) -> String {
        let template = Policy {
            id: format!("policy-{}", resource.replace(':', "-")),
            name: format!("{} access policy", resource),
            description: format!("Controls access to {} resources", resource),
            version: "1.0.0".to_string(),
            policy_type: PolicyType::ResourcePolicy,
            resource: resource.to_string(),
            rules: vec![
                PolicyRule {
                    id: "allow-read".to_string(),
                    name: "Allow read access".to_string(),
                    actions: vec!["read".to_string(), "list".to_string()],
                    effect: Effect::Allow,
                    conditions: Vec::new(),
                    roles: vec!["viewer".to_string(), "editor".to_string(), "admin".to_string()],
                    derived_roles: Vec::new(),
                    priority: 10,
                },
                PolicyRule {
                    id: "allow-write".to_string(),
                    name: "Allow write access".to_string(),
                    actions: vec!["create".to_string(), "update".to_string()],
                    effect: Effect::Allow,
                    conditions: Vec::new(),
                    roles: vec!["editor".to_string(), "admin".to_string()],
                    derived_roles: Vec::new(),
                    priority: 10,
                },
                PolicyRule {
                    id: "allow-delete".to_string(),
                    name: "Allow delete access".to_string(),
                    actions: vec!["delete".to_string()],
                    effect: Effect::Allow,
                    conditions: Vec::new(),
                    roles: vec!["admin".to_string()],
                    derived_roles: Vec::new(),
                    priority: 10,
                },
            ],
            variables: HashMap::new(),
            disabled: false,
        };

        Self::to_yaml(&template)
    }
}

// ─── Policy analytics ────────────────────────────────────────────────────────

/// Represents a conflict between two policy rules.
#[derive(Debug, Clone)]
pub struct PolicyConflict {
    pub policy_a: String,
    pub policy_b: String,
    pub resource: String,
    pub actions: Vec<String>,
    pub description: String,
}

/// Analyze policies for coverage, conflicts, and unused rules.
pub struct PolicyAnalytics;

impl PolicyAnalytics {
    /// Generate a coverage report showing which resources and actions are covered.
    pub fn coverage_report(engine: &PolicyEngine) -> String {
        let mut lines = Vec::new();
        lines.push("=== Policy Coverage Report ===".to_string());
        lines.push(String::new());

        let policies = engine.list_policies();
        if policies.is_empty() {
            lines.push("No policies defined.".to_string());
            return lines.join("\n");
        }

        lines.push(format!("Total policies: {}", policies.len()));
        lines.push(String::new());

        // Group by resource.
        let mut resource_actions: HashMap<String, Vec<String>> = HashMap::new();
        let mut resource_roles: HashMap<String, Vec<String>> = HashMap::new();

        for policy in &policies {
            let entry = resource_actions.entry(policy.resource.clone()).or_default();
            let role_entry = resource_roles.entry(policy.resource.clone()).or_default();
            for rule in &policy.rules {
                for action in &rule.actions {
                    if !entry.contains(action) {
                        entry.push(action.clone());
                    }
                }
                for role in &rule.roles {
                    if !role_entry.contains(role) {
                        role_entry.push(role.clone());
                    }
                }
            }
        }

        for (resource, actions) in &resource_actions {
            lines.push(format!("Resource: {}", resource));
            lines.push(format!("  Actions: {}", actions.join(", ")));
            if let Some(roles) = resource_roles.get(resource) {
                lines.push(format!("  Roles: {}", roles.join(", ")));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }

    /// Detect conflicts — overlapping rules on the same resource/actions with different effects.
    pub fn conflict_detection(engine: &PolicyEngine) -> Vec<PolicyConflict> {
        let mut conflicts = Vec::new();
        let policies = engine.list_policies();

        for i in 0..policies.len() {
            for j in (i + 1)..policies.len() {
                let pa = &policies[i];
                let pb = &policies[j];

                // Only compare policies for the same resource.
                if pa.resource != pb.resource {
                    continue;
                }

                for ra in &pa.rules {
                    for rb in &pb.rules {
                        // Find overlapping actions.
                        let overlap: Vec<String> = ra
                            .actions
                            .iter()
                            .filter(|a| {
                                rb.actions.contains(a)
                                    || rb.actions.contains(&"*".to_string())
                                    || **a == "*"
                            })
                            .cloned()
                            .collect();

                        if overlap.is_empty() {
                            continue;
                        }

                        // Find overlapping roles.
                        let role_overlap = ra
                            .roles
                            .iter()
                            .any(|r| rb.roles.contains(r) || rb.roles.contains(&"*".to_string()) || r == "*");

                        if !role_overlap {
                            continue;
                        }

                        // Different effects = conflict.
                        if ra.effect != rb.effect {
                            conflicts.push(PolicyConflict {
                                policy_a: pa.id.clone(),
                                policy_b: pb.id.clone(),
                                resource: pa.resource.clone(),
                                actions: overlap,
                                description: format!(
                                    "Rule '{}' ({}) in policy '{}' conflicts with rule '{}' ({}) in policy '{}'",
                                    ra.id, ra.effect, pa.id, rb.id, rb.effect, pb.id
                                ),
                            });
                        }
                    }
                }
            }
        }

        conflicts
    }

    /// Find rules that have never been matched in the audit log.
    pub fn unused_rules(engine: &PolicyEngine, audit_log: &[AuditEntry]) -> Vec<String> {
        let mut all_rule_ids: Vec<String> = Vec::new();
        for policy in engine.list_policies() {
            for rule in &policy.rules {
                all_rule_ids.push(format!("{}:{}", policy.id, rule.id));
            }
        }

        let matched: std::collections::HashSet<String> = audit_log
            .iter()
            .filter_map(|entry| {
                entry.result.matched_rule.as_ref().map(|rule_id| {
                    format!("{}:{}", entry.result.policy_id, rule_id)
                })
            })
            .collect();

        all_rule_ids
            .into_iter()
            .filter(|id| !matched.contains(id))
            .collect()
    }
}

// ─── Helper constructors ─────────────────────────────────────────────────────

impl Principal {
    pub fn new(id: &str, roles: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            roles: roles.into_iter().map(|r| r.to_string()).collect(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }
}

impl Resource {
    pub fn new(kind: &str, id: &str) -> Self {
        Self {
            kind: kind.to_string(),
            id: id.to_string(),
            attributes: HashMap::new(),
            policy_version: "default".to_string(),
        }
    }

    pub fn with_attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }
}

impl CheckRequest {
    pub fn new(principal: Principal, resource: Resource, action: &str) -> Self {
        Self {
            principal,
            resource,
            action: action.to_string(),
            aux_data: HashMap::new(),
        }
    }

    pub fn with_aux(mut self, key: &str, value: &str) -> Self {
        self.aux_data.insert(key.to_string(), value.to_string());
        self
    }
}

impl PolicyRule {
    pub fn new(id: &str, name: &str, actions: Vec<&str>, effect: Effect) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            actions: actions.into_iter().map(|a| a.to_string()).collect(),
            effect,
            conditions: Vec::new(),
            roles: Vec::new(),
            derived_roles: Vec::new(),
            priority: 10,
        }
    }

    pub fn with_roles(mut self, roles: Vec<&str>) -> Self {
        self.roles = roles.into_iter().map(|r| r.to_string()).collect();
        self
    }

    pub fn with_derived_roles(mut self, roles: Vec<&str>) -> Self {
        self.derived_roles = roles.into_iter().map(|r| r.to_string()).collect();
        self
    }

    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

impl Policy {
    pub fn new(
        id: &str,
        name: &str,
        resource: &str,
        policy_type: PolicyType,
        rules: Vec<PolicyRule>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            policy_type,
            resource: resource.to_string(),
            rules,
            variables: HashMap::new(),
            disabled: false,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_variable(mut self, key: &str, value: &str) -> Self {
        self.variables.insert(key.to_string(), value.to_string());
        self
    }
}

impl Condition {
    pub fn new(match_expr: &str, operator: ConditionOperator, value: ConditionValue) -> Self {
        Self {
            match_expr: match_expr.to_string(),
            operator,
            value,
        }
    }
}

impl DerivedRole {
    pub fn new(name: &str, parent_roles: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            parent_roles: parent_roles.into_iter().map(|r| r.to_string()).collect(),
            conditions: Vec::new(),
        }
    }

    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────────

    fn make_document_policy() -> Policy {
        Policy::new(
            "doc-policy",
            "Document Access",
            "document",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("read-rule", "Read", vec!["read", "list"], Effect::Allow)
                    .with_roles(vec!["viewer", "editor", "admin"]),
                PolicyRule::new("write-rule", "Write", vec!["create", "update"], Effect::Allow)
                    .with_roles(vec!["editor", "admin"]),
                PolicyRule::new("delete-rule", "Delete", vec!["delete"], Effect::Allow)
                    .with_roles(vec!["admin"]),
            ],
        )
    }

    fn make_viewer() -> Principal {
        Principal::new("user-1", vec!["viewer"])
    }

    fn make_editor() -> Principal {
        Principal::new("user-2", vec!["editor"])
    }

    fn make_admin() -> Principal {
        Principal::new("user-3", vec!["admin"])
    }

    fn make_document() -> Resource {
        Resource::new("document", "doc-42")
    }

    // ── Effect enum ─────────────────────────────────────────────────────

    #[test]
    fn test_effect_display() {
        assert_eq!(format!("{}", Effect::Allow), "EFFECT_ALLOW");
        assert_eq!(format!("{}", Effect::Deny), "EFFECT_DENY");
    }

    #[test]
    fn test_effect_equality() {
        assert_eq!(Effect::Allow, Effect::Allow);
        assert_ne!(Effect::Allow, Effect::Deny);
    }

    // ── PolicyType ──────────────────────────────────────────────────────

    #[test]
    fn test_policy_type_display() {
        assert_eq!(format!("{}", PolicyType::ResourcePolicy), "resource");
        assert_eq!(format!("{}", PolicyType::DerivedRoles), "derived_roles");
    }

    // ── Basic RBAC ──────────────────────────────────────────────────────

    #[test]
    fn test_rbac_viewer_can_read() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
    }

    #[test]
    fn test_rbac_viewer_cannot_write() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "create");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    #[test]
    fn test_rbac_viewer_cannot_delete() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "delete");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    #[test]
    fn test_rbac_editor_can_write() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_editor(), make_document(), "create");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
    }

    #[test]
    fn test_rbac_editor_cannot_delete() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_editor(), make_document(), "delete");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    #[test]
    fn test_rbac_admin_can_delete() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_admin(), make_document(), "delete");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
    }

    #[test]
    fn test_rbac_admin_can_do_everything() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        for action in &["read", "list", "create", "update", "delete"] {
            let req = CheckRequest::new(make_admin(), make_document(), action);
            let result = engine.check(&req);
            assert_eq!(result.effect, Effect::Allow, "admin should be allowed to {}", action);
        }
    }

    #[test]
    fn test_rbac_unknown_action_denied() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_admin(), make_document(), "archive");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    #[test]
    fn test_rbac_no_role_denied() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let principal = Principal::new("anon", vec![]);
        let req = CheckRequest::new(principal, make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    #[test]
    fn test_default_deny_no_policies() {
        let mut engine = PolicyEngine::new();
        let req = CheckRequest::new(make_admin(), make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
        assert!(result.matched_rule.is_none());
    }

    // ── Wildcard actions/roles ──────────────────────────────────────────

    #[test]
    fn test_wildcard_action() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(
            "catch-all",
            "Catch All",
            "document",
            PolicyType::ResourcePolicy,
            vec![PolicyRule::new("any-action", "Any", vec!["*"], Effect::Allow)
                .with_roles(vec!["superadmin"])],
        );
        engine.add_policy(policy).unwrap();

        let principal = Principal::new("sa-1", vec!["superadmin"]);
        let req = CheckRequest::new(principal, make_document(), "anything_goes");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
    }

    #[test]
    fn test_wildcard_role() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(
            "public-read",
            "Public Read",
            "document",
            PolicyType::ResourcePolicy,
            vec![PolicyRule::new("public", "Public", vec!["read"], Effect::Allow)
                .with_roles(vec!["*"])],
        );
        engine.add_policy(policy).unwrap();

        let req = CheckRequest::new(
            Principal::new("guest", vec!["anonymous"]),
            make_document(),
            "read",
        );
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
    }

    // ── Wildcard resource ───────────────────────────────────────────────

    #[test]
    fn test_wildcard_resource_pattern() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(
            "doc-wildcard",
            "Document Wildcard",
            "document:*",
            PolicyType::ResourcePolicy,
            vec![PolicyRule::new("r1", "R1", vec!["read"], Effect::Allow)
                .with_roles(vec!["viewer"])],
        );
        engine.add_policy(policy).unwrap();

        let resource = Resource::new("document:public", "d1");
        let req = CheckRequest::new(make_viewer(), resource, "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
    }

    // ── ABAC condition evaluation ───────────────────────────────────────

    #[test]
    fn test_condition_eq_string() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("department", "engineering");
        let resource = make_document();
        let cond = Condition::new("P.attr.department", ConditionOperator::Eq, ConditionValue::String("engineering".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_eq_string_no_match() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("department", "sales");
        let resource = make_document();
        let cond = Condition::new("P.attr.department", ConditionOperator::Eq, ConditionValue::String("engineering".into()));
        assert!(!evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_not_eq() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("status", "active");
        let resource = make_document();
        let cond = Condition::new("P.attr.status", ConditionOperator::NotEq, ConditionValue::String("disabled".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_in_list() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("region", "us-east");
        let resource = make_document();
        let cond = Condition::new(
            "P.attr.region",
            ConditionOperator::In,
            ConditionValue::List(vec!["us-east".into(), "us-west".into(), "eu-west".into()]),
        );
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_not_in_list() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("region", "ap-south");
        let resource = make_document();
        let cond = Condition::new(
            "P.attr.region",
            ConditionOperator::NotIn,
            ConditionValue::List(vec!["us-east".into(), "us-west".into()]),
        );
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_contains() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("email", "alice@example.com");
        let resource = make_document();
        let cond = Condition::new("P.attr.email", ConditionOperator::Contains, ConditionValue::String("example.com".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_starts_with() {
        let evaluator = ConditionEvaluator;
        let principal = Principal::new("u1", vec![]);
        let resource = Resource::new("file", "f1").with_attr("path", "/home/alice/docs/report.pdf");
        let cond = Condition::new("R.attr.path", ConditionOperator::StartsWith, ConditionValue::String("/home/alice".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_ends_with() {
        let evaluator = ConditionEvaluator;
        let principal = Principal::new("u1", vec![]);
        let resource = Resource::new("file", "f1").with_attr("name", "report.pdf");
        let cond = Condition::new("R.attr.name", ConditionOperator::EndsWith, ConditionValue::String(".pdf".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_regex() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("email", "alice@corp.example.com");
        let resource = make_document();
        let cond = Condition::new(
            "P.attr.email",
            ConditionOperator::Regex,
            ConditionValue::String(r"^[a-z]+@corp\.example\.com$".into()),
        );
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_regex_no_match() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("email", "alice@gmail.com");
        let resource = make_document();
        let cond = Condition::new(
            "P.attr.email",
            ConditionOperator::Regex,
            ConditionValue::String(r"^[a-z]+@corp\.example\.com$".into()),
        );
        assert!(!evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_gt() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("clearance", "5");
        let resource = make_document();
        let cond = Condition::new("P.attr.clearance", ConditionOperator::Gt, ConditionValue::Number(3.0));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_lt() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("risk_score", "2");
        let resource = make_document();
        let cond = Condition::new("P.attr.risk_score", ConditionOperator::Lt, ConditionValue::Number(5.0));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_gte() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("level", "10");
        let resource = make_document();
        let cond = Condition::new("P.attr.level", ConditionOperator::Gte, ConditionValue::Number(10.0));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_lte() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("age", "18");
        let resource = make_document();
        let cond = Condition::new("P.attr.age", ConditionOperator::Lte, ConditionValue::Number(21.0));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_eq_number() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("version", "3");
        let resource = make_document();
        let cond = Condition::new("P.attr.version", ConditionOperator::Eq, ConditionValue::Number(3.0));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_eq_bool() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("mfa_enabled", "true");
        let resource = make_document();
        let cond = Condition::new("P.attr.mfa_enabled", ConditionOperator::Eq, ConditionValue::Bool(true));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_not_operator() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("suspended", "false");
        let resource = make_document();
        let cond = Condition::new("P.attr.suspended", ConditionOperator::Not, ConditionValue::Bool(true));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_resolve_resource_kind() {
        let evaluator = ConditionEvaluator;
        let principal = Principal::new("u1", vec![]);
        let resource = Resource::new("document", "d1");
        let cond = Condition::new("R.kind", ConditionOperator::Eq, ConditionValue::String("document".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_resolve_resource_id() {
        let evaluator = ConditionEvaluator;
        let principal = Principal::new("u1", vec![]);
        let resource = Resource::new("document", "d1");
        let cond = Condition::new("R.id", ConditionOperator::Eq, ConditionValue::String("d1".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_resolve_principal_id() {
        let evaluator = ConditionEvaluator;
        let principal = Principal::new("alice", vec![]);
        let resource = Resource::new("document", "d1");
        let cond = Condition::new("P.id", ConditionOperator::Eq, ConditionValue::String("alice".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_resolve_aux_data() {
        let evaluator = ConditionEvaluator;
        let principal = Principal::new("u1", vec![]);
        let resource = Resource::new("document", "d1");
        let mut aux = HashMap::new();
        aux.insert("ip_country".to_string(), "US".to_string());
        let cond = Condition::new("aux.ip_country", ConditionOperator::Eq, ConditionValue::String("US".into()));
        assert!(evaluator.evaluate(&cond, &principal, &resource, &aux));
    }

    // ── ABAC in engine ──────────────────────────────────────────────────

    #[test]
    fn test_abac_condition_allow() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(
            "abac-doc",
            "ABAC Doc",
            "document",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("dept-read", "Dept Read", vec!["read"], Effect::Allow)
                    .with_roles(vec!["employee"])
                    .with_condition(Condition::new(
                        "P.attr.department",
                        ConditionOperator::Eq,
                        ConditionValue::String("engineering".into()),
                    )),
            ],
        );
        engine.add_policy(policy).unwrap();

        let principal = Principal::new("alice", vec!["employee"])
            .with_attr("department", "engineering");
        let req = CheckRequest::new(principal, make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
    }

    #[test]
    fn test_abac_condition_deny_wrong_dept() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(
            "abac-doc",
            "ABAC Doc",
            "document",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("dept-read", "Dept Read", vec!["read"], Effect::Allow)
                    .with_roles(vec!["employee"])
                    .with_condition(Condition::new(
                        "P.attr.department",
                        ConditionOperator::Eq,
                        ConditionValue::String("engineering".into()),
                    )),
            ],
        );
        engine.add_policy(policy).unwrap();

        let principal = Principal::new("bob", vec!["employee"])
            .with_attr("department", "marketing");
        let req = CheckRequest::new(principal, make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    #[test]
    fn test_abac_multiple_conditions() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(
            "multi-cond",
            "Multi Cond",
            "document",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("strict-read", "Strict Read", vec!["read"], Effect::Allow)
                    .with_roles(vec!["employee"])
                    .with_condition(Condition::new(
                        "P.attr.department",
                        ConditionOperator::Eq,
                        ConditionValue::String("engineering".into()),
                    ))
                    .with_condition(Condition::new(
                        "P.attr.clearance",
                        ConditionOperator::Gte,
                        ConditionValue::Number(3.0),
                    )),
            ],
        );
        engine.add_policy(policy).unwrap();

        // Both conditions met
        let p = Principal::new("alice", vec!["employee"])
            .with_attr("department", "engineering")
            .with_attr("clearance", "5");
        let req = CheckRequest::new(p, make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);

        // Missing clearance
        let p2 = Principal::new("bob", vec!["employee"])
            .with_attr("department", "engineering")
            .with_attr("clearance", "1");
        let req2 = CheckRequest::new(p2, make_document(), "read");
        let result2 = engine.check(&req2);
        assert_eq!(result2.effect, Effect::Deny);
    }

    // ── Derived roles ───────────────────────────────────────────────────

    #[test]
    fn test_derived_role_resolution() {
        let mut drs = DerivedRoleSet::new();
        drs.add_role(
            DerivedRole::new("owner", vec!["employee"])
                .with_condition(Condition::new(
                    "P.attr.owner_id",
                    ConditionOperator::Eq,
                    ConditionValue::String("doc-42".into()),
                )),
        );

        // Matches
        let p = Principal::new("alice", vec!["employee"]).with_attr("owner_id", "doc-42");
        let r = Resource::new("document", "doc-42");
        let roles = drs.resolve_roles(&p, &r);
        assert!(roles.contains(&"owner".to_string()));

        // Does not match — wrong owner_id
        let p2 = Principal::new("bob", vec!["employee"]).with_attr("owner_id", "doc-99");
        let roles2 = drs.resolve_roles(&p2, &r);
        assert!(!roles2.contains(&"owner".to_string()));
    }

    #[test]
    fn test_derived_role_no_parent_role() {
        let mut drs = DerivedRoleSet::new();
        drs.add_role(DerivedRole::new("owner", vec!["employee"]));

        let p = Principal::new("guest", vec!["visitor"]);
        let r = Resource::new("document", "d1");
        let roles = drs.resolve_roles(&p, &r);
        assert!(roles.is_empty());
    }

    #[test]
    fn test_derived_role_in_engine() {
        let mut engine = PolicyEngine::new();
        engine.add_derived_role(
            DerivedRole::new("owner", vec!["employee"])
                .with_condition(Condition::new(
                    "R.attr.owner_id",
                    ConditionOperator::Eq,
                    ConditionValue::String("alice".into()),
                )),
        );

        let policy = Policy::new(
            "owner-policy",
            "Owner Policy",
            "document",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("owner-delete", "Owner Delete", vec!["delete"], Effect::Allow)
                    .with_derived_roles(vec!["owner"]),
            ],
        );
        engine.add_policy(policy).unwrap();

        let principal = Principal::new("alice", vec!["employee"]);
        let resource = Resource::new("document", "doc-1").with_attr("owner_id", "alice");
        let req = CheckRequest::new(principal, resource, "delete");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
    }

    #[test]
    fn test_derived_role_not_matched() {
        let mut engine = PolicyEngine::new();
        engine.add_derived_role(
            DerivedRole::new("owner", vec!["employee"])
                .with_condition(Condition::new(
                    "R.attr.owner_id",
                    ConditionOperator::Eq,
                    ConditionValue::String("alice".into()),
                )),
        );

        let policy = Policy::new(
            "owner-policy",
            "Owner Policy",
            "document",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("owner-delete", "Owner Delete", vec!["delete"], Effect::Allow)
                    .with_derived_roles(vec!["owner"]),
            ],
        );
        engine.add_policy(policy).unwrap();

        let principal = Principal::new("bob", vec!["employee"]);
        let resource = Resource::new("document", "doc-1").with_attr("owner_id", "charlie");
        let req = CheckRequest::new(principal, resource, "delete");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    // ── Priority ────────────────────────────────────────────────────────

    #[test]
    fn test_rule_priority_higher_wins() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(
            "priority-test",
            "Priority Test",
            "document",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("low-pri", "Low", vec!["read"], Effect::Deny)
                    .with_roles(vec!["viewer"])
                    .with_priority(100),
                PolicyRule::new("high-pri", "High", vec!["read"], Effect::Allow)
                    .with_roles(vec!["viewer"])
                    .with_priority(1),
            ],
        );
        engine.add_policy(policy).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Allow);
        assert_eq!(result.matched_rule.as_deref(), Some("high-pri"));
    }

    #[test]
    fn test_deny_wins_at_equal_priority() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new(
            "equal-pri",
            "Equal Priority",
            "document",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("allow-rule", "Allow", vec!["read"], Effect::Allow)
                    .with_roles(vec!["viewer"])
                    .with_priority(10),
                PolicyRule::new("deny-rule", "Deny", vec!["read"], Effect::Deny)
                    .with_roles(vec!["viewer"])
                    .with_priority(10),
            ],
        );
        engine.add_policy(policy).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    // ── Disabled policies ───────────────────────────────────────────────

    #[test]
    fn test_disabled_policy_ignored() {
        let mut engine = PolicyEngine::new();
        let mut policy = make_document_policy();
        policy.disabled = true;
        engine.add_policy(policy).unwrap();

        let req = CheckRequest::new(make_admin(), make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
    }

    // ── Batch checks ────────────────────────────────────────────────────

    #[test]
    fn test_batch_check() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let requests = vec![
            CheckRequest::new(make_viewer(), make_document(), "read"),
            CheckRequest::new(make_viewer(), make_document(), "delete"),
            CheckRequest::new(make_admin(), make_document(), "delete"),
        ];

        let results = engine.check_batch(&requests);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].effect, Effect::Allow);
        assert_eq!(results[1].effect, Effect::Deny);
        assert_eq!(results[2].effect, Effect::Allow);
    }

    #[test]
    fn test_batch_empty() {
        let mut engine = PolicyEngine::new();
        let results = engine.check_batch(&[]);
        assert!(results.is_empty());
    }

    // ── Policy management ───────────────────────────────────────────────

    #[test]
    fn test_add_and_get_policy() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();
        assert!(engine.get_policy("doc-policy").is_some());
    }

    #[test]
    fn test_remove_policy() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();
        assert!(engine.remove_policy("doc-policy").is_ok());
        assert!(engine.get_policy("doc-policy").is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut engine = PolicyEngine::new();
        assert!(engine.remove_policy("nope").is_err());
    }

    #[test]
    fn test_list_policies() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();
        assert_eq!(engine.list_policies().len(), 1);
    }

    #[test]
    fn test_duplicate_policy_id_rejected() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();
        let result = engine.add_policy(make_document_policy());
        assert!(result.is_err());
    }

    // ── Validation ──────────────────────────────────────────────────────

    #[test]
    fn test_validate_empty_id() {
        let policy = Policy {
            id: String::new(),
            name: "Test".into(),
            description: String::new(),
            version: "1.0.0".into(),
            policy_type: PolicyType::ResourcePolicy,
            resource: "doc".into(),
            rules: vec![PolicyRule::new("r1", "R1", vec!["read"], Effect::Allow).with_roles(vec!["viewer"])],
            variables: HashMap::new(),
            disabled: false,
        };
        let errors = PolicyEngine::validate_policy(&policy);
        assert!(errors.iter().any(|e| e.contains("id must not be empty")));
    }

    #[test]
    fn test_validate_no_rules() {
        let policy = Policy {
            id: "p1".into(),
            name: "Test".into(),
            description: String::new(),
            version: "1.0.0".into(),
            policy_type: PolicyType::ResourcePolicy,
            resource: "doc".into(),
            rules: vec![],
            variables: HashMap::new(),
            disabled: false,
        };
        let errors = PolicyEngine::validate_policy(&policy);
        assert!(errors.iter().any(|e| e.contains("at least one rule")));
    }

    #[test]
    fn test_validate_rule_no_actions() {
        let policy = Policy::new(
            "p1",
            "Test",
            "doc",
            PolicyType::ResourcePolicy,
            vec![PolicyRule {
                id: "r1".into(),
                name: "R1".into(),
                actions: vec![],
                effect: Effect::Allow,
                conditions: vec![],
                roles: vec!["admin".into()],
                derived_roles: vec![],
                priority: 10,
            }],
        );
        let errors = PolicyEngine::validate_policy(&policy);
        assert!(errors.iter().any(|e| e.contains("at least one action")));
    }

    #[test]
    fn test_validate_rule_no_roles() {
        let policy = Policy::new(
            "p1",
            "Test",
            "doc",
            PolicyType::ResourcePolicy,
            vec![PolicyRule {
                id: "r1".into(),
                name: "R1".into(),
                actions: vec!["read".into()],
                effect: Effect::Allow,
                conditions: vec![],
                roles: vec![],
                derived_roles: vec![],
                priority: 10,
            }],
        );
        let errors = PolicyEngine::validate_policy(&policy);
        assert!(errors.iter().any(|e| e.contains("at least one role or derived role")));
    }

    #[test]
    fn test_validate_duplicate_rule_ids() {
        let policy = Policy::new(
            "p1",
            "Test",
            "doc",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("dup", "R1", vec!["read"], Effect::Allow).with_roles(vec!["admin"]),
                PolicyRule::new("dup", "R2", vec!["write"], Effect::Allow).with_roles(vec!["admin"]),
            ],
        );
        let errors = PolicyEngine::validate_policy(&policy);
        assert!(errors.iter().any(|e| e.contains("Duplicate rule id")));
    }

    #[test]
    fn test_validate_invalid_regex() {
        let policy = Policy::new(
            "p1",
            "Test",
            "doc",
            PolicyType::ResourcePolicy,
            vec![
                PolicyRule::new("r1", "R1", vec!["read"], Effect::Allow)
                    .with_roles(vec!["admin"])
                    .with_condition(Condition::new(
                        "P.attr.email",
                        ConditionOperator::Regex,
                        ConditionValue::String("[invalid".into()),
                    )),
            ],
        );
        let errors = PolicyEngine::validate_policy(&policy);
        assert!(errors.iter().any(|e| e.contains("invalid regex")));
    }

    #[test]
    fn test_validate_good_policy() {
        let errors = PolicyEngine::validate_policy(&make_document_policy());
        assert!(errors.is_empty());
    }

    // ── Audit log ───────────────────────────────────────────────────────

    #[test]
    fn test_audit_log_recorded() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        engine.check(&req);

        assert_eq!(engine.get_audit_log().len(), 1);
        assert_eq!(engine.get_audit_log()[0].result.effect, Effect::Allow);
    }

    #[test]
    fn test_audit_log_clear() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        engine.check(&req);
        assert_eq!(engine.get_audit_log().len(), 1);

        engine.clear_audit_log();
        assert!(engine.get_audit_log().is_empty());
    }

    #[test]
    fn test_audit_log_policy_chain() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        engine.check(&req);

        let entry = &engine.get_audit_log()[0];
        assert!(entry.policy_chain.contains(&"doc-policy".to_string()));
    }

    #[test]
    fn test_check_result_fields() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        let result = engine.check(&req);

        assert_eq!(result.resource_id, "doc-42");
        assert_eq!(result.action, "read");
        assert_eq!(result.policy_id, "doc-policy");
        assert!(result.matched_rule.is_some());
        assert!(!result.request_id.is_empty());
    }

    // ── Policy testing harness ──────────────────────────────────────────

    #[test]
    fn test_policy_test_suite_all_pass() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let suite = PolicyTestSuite {
            name: "Basic RBAC".into(),
            tests: vec![
                PolicyTestCase {
                    name: "viewer-read".into(),
                    request: CheckRequest::new(make_viewer(), make_document(), "read"),
                    expected_effect: Effect::Allow,
                    description: "Viewer can read".into(),
                },
                PolicyTestCase {
                    name: "viewer-no-delete".into(),
                    request: CheckRequest::new(make_viewer(), make_document(), "delete"),
                    expected_effect: Effect::Deny,
                    description: "Viewer cannot delete".into(),
                },
            ],
        };

        let results = PolicyTester::run_suite(&mut engine, &suite);
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_policy_test_suite_with_failure() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let suite = PolicyTestSuite {
            name: "Broken test".into(),
            tests: vec![PolicyTestCase {
                name: "wrong-expectation".into(),
                request: CheckRequest::new(make_viewer(), make_document(), "read"),
                expected_effect: Effect::Deny, // wrong
                description: "Intentionally wrong".into(),
            }],
        };

        let results = PolicyTester::run_suite(&mut engine, &suite);
        assert!(!results[0].passed);
        assert!(results[0].message.contains("FAIL"));
    }

    // ── YAML serialization ──────────────────────────────────────────────

    #[test]
    fn test_yaml_roundtrip() {
        let policy = make_document_policy();
        let yaml = PolicySerializer::to_yaml(&policy);
        let parsed = PolicySerializer::from_yaml(&yaml).expect("should parse");
        assert_eq!(parsed.id, policy.id);
        assert_eq!(parsed.name, policy.name);
        assert_eq!(parsed.rules.len(), policy.rules.len());
    }

    #[test]
    fn test_yaml_invalid() {
        let result = PolicySerializer::from_yaml("not: [valid: yaml: policy");
        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_template() {
        let template = PolicySerializer::generate_template("document");
        assert!(template.contains("document"));
        assert!(template.contains("allow-read"));
        assert!(template.contains("allow-write"));
        assert!(template.contains("allow-delete"));
    }

    #[test]
    fn test_yaml_template_parseable() {
        let template = PolicySerializer::generate_template("report");
        let parsed = PolicySerializer::from_yaml(&template);
        assert!(parsed.is_ok());
    }

    // ── Policy analytics ────────────────────────────────────────────────

    #[test]
    fn test_coverage_report_empty() {
        let engine = PolicyEngine::new();
        let report = PolicyAnalytics::coverage_report(&engine);
        assert!(report.contains("No policies defined"));
    }

    #[test]
    fn test_coverage_report_with_policy() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let report = PolicyAnalytics::coverage_report(&engine);
        assert!(report.contains("document"));
        assert!(report.contains("read"));
        assert!(report.contains("admin"));
    }

    #[test]
    fn test_conflict_detection_no_conflict() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let conflicts = PolicyAnalytics::conflict_detection(&engine);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_detection_finds_conflict() {
        let mut engine = PolicyEngine::new();
        let allow = Policy::new(
            "allow-doc",
            "Allow Doc",
            "document",
            PolicyType::ResourcePolicy,
            vec![PolicyRule::new("r1", "Allow Read", vec!["read"], Effect::Allow)
                .with_roles(vec!["viewer"])],
        );
        let deny = Policy::new(
            "deny-doc",
            "Deny Doc",
            "document",
            PolicyType::ResourcePolicy,
            vec![PolicyRule::new("r2", "Deny Read", vec!["read"], Effect::Deny)
                .with_roles(vec!["viewer"])],
        );
        engine.add_policy(allow).unwrap();
        engine.add_policy(deny).unwrap();

        let conflicts = PolicyAnalytics::conflict_detection(&engine);
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].actions.contains(&"read".to_string()));
    }

    #[test]
    fn test_unused_rules() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        // Only trigger the read rule.
        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        engine.check(&req);

        let unused = PolicyAnalytics::unused_rules(&engine, engine.get_audit_log());
        // write-rule and delete-rule should be unused.
        assert!(unused.iter().any(|u| u.contains("write-rule")));
        assert!(unused.iter().any(|u| u.contains("delete-rule")));
        assert!(!unused.iter().any(|u| u.contains("read-rule")));
    }

    #[test]
    fn test_unused_rules_all_used() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        // Trigger all rules.
        engine.check(&CheckRequest::new(make_viewer(), make_document(), "read"));
        engine.check(&CheckRequest::new(make_editor(), make_document(), "create"));
        engine.check(&CheckRequest::new(make_admin(), make_document(), "delete"));

        let unused = PolicyAnalytics::unused_rules(&engine, engine.get_audit_log());
        assert!(unused.is_empty());
    }

    // ── Resource matching ───────────────────────────────────────────────

    #[test]
    fn test_resource_match_exact() {
        assert!(PolicyEngine::resource_matches("document", "document"));
        assert!(!PolicyEngine::resource_matches("document", "file"));
    }

    #[test]
    fn test_resource_match_wildcard() {
        assert!(PolicyEngine::resource_matches("*", "anything"));
    }

    #[test]
    fn test_resource_match_prefix_wildcard() {
        assert!(PolicyEngine::resource_matches("document:*", "document:public"));
        assert!(!PolicyEngine::resource_matches("document:*", "file:public"));
    }

    // ── Multiple policies for same resource ─────────────────────────────

    #[test]
    fn test_multiple_policies_same_resource() {
        let mut engine = PolicyEngine::new();

        let p1 = Policy::new(
            "base-access",
            "Base",
            "document",
            PolicyType::ResourcePolicy,
            vec![PolicyRule::new("base-read", "Base Read", vec!["read"], Effect::Allow)
                .with_roles(vec!["viewer"])
                .with_priority(20)],
        );
        let p2 = Policy::new(
            "restricted-access",
            "Restricted",
            "document",
            PolicyType::ResourcePolicy,
            vec![PolicyRule::new("restrict-read", "No Read", vec!["read"], Effect::Deny)
                .with_roles(vec!["viewer"])
                .with_priority(1)], // Higher precedence
        );
        engine.add_policy(p1).unwrap();
        engine.add_policy(p2).unwrap();

        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        let result = engine.check(&req);
        assert_eq!(result.effect, Effect::Deny);
        assert_eq!(result.matched_rule.as_deref(), Some("restrict-read"));
    }

    // ── Serde round-trip ────────────────────────────────────────────────

    #[test]
    fn test_serde_json_roundtrip_effect() {
        let json = serde_json::to_string(&Effect::Allow).unwrap();
        let parsed: Effect = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Effect::Allow);
    }

    #[test]
    fn test_serde_json_roundtrip_condition_value() {
        let val = ConditionValue::List(vec!["a".into(), "b".into()]);
        let json = serde_json::to_string(&val).unwrap();
        let parsed: ConditionValue = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, val);
    }

    #[test]
    fn test_serde_json_roundtrip_check_request() {
        let req = CheckRequest::new(make_viewer(), make_document(), "read")
            .with_aux("ip", "1.2.3.4");
        let json = serde_json::to_string(&req).unwrap();
        let parsed: CheckRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.action, "read");
        assert_eq!(parsed.aux_data.get("ip").unwrap(), "1.2.3.4");
    }

    // ── Builder ergonomics ──────────────────────────────────────────────

    #[test]
    fn test_principal_builder() {
        let p = Principal::new("alice", vec!["admin", "viewer"])
            .with_attr("dept", "eng");
        assert_eq!(p.id, "alice");
        assert_eq!(p.roles.len(), 2);
        assert_eq!(p.attributes.get("dept").unwrap(), "eng");
    }

    #[test]
    fn test_resource_builder() {
        let r = Resource::new("document", "d1")
            .with_attr("classification", "public");
        assert_eq!(r.kind, "document");
        assert_eq!(r.attributes.get("classification").unwrap(), "public");
    }

    #[test]
    fn test_policy_builder() {
        let p = Policy::new("p1", "Test", "doc", PolicyType::ResourcePolicy, vec![
            PolicyRule::new("r1", "R1", vec!["read"], Effect::Allow).with_roles(vec!["viewer"]),
        ])
        .with_description("A test policy")
        .with_variable("env", "production");

        assert_eq!(p.description, "A test policy");
        assert_eq!(p.variables.get("env").unwrap(), "production");
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[test]
    fn test_condition_null_value() {
        let evaluator = ConditionEvaluator;
        let principal = Principal::new("u1", vec![]);
        let resource = Resource::new("doc", "d1");
        // Attribute does not exist — resolve returns None
        let cond = Condition::new("P.attr.missing", ConditionOperator::Eq, ConditionValue::Null);
        // resolved is None, match_expr "P.attr.missing" → None → unwrap_or_default gives "".
        // For Null, we check resolved.is_none() — but resolve_expr returns None when key missing.
        // Actually in our code, we call unwrap_or_default before the match. Let's handle via
        // the fact that the attribute is missing:
        // With our impl, Eq + Null checks `resolved.is_none()` — which is true.
        assert!(evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_condition_invalid_regex_returns_false() {
        let evaluator = ConditionEvaluator;
        let principal = make_viewer().with_attr("x", "test");
        let resource = make_document();
        let cond = Condition::new("P.attr.x", ConditionOperator::Regex, ConditionValue::String("[bad".into()));
        assert!(!evaluator.evaluate(&cond, &principal, &resource, &HashMap::new()));
    }

    #[test]
    fn test_evaluation_duration_recorded() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();
        let req = CheckRequest::new(make_viewer(), make_document(), "read");
        let result = engine.check(&req);
        // Duration should be non-negative (can be 0 on fast machines).
        assert!(result.evaluation_duration_us < 1_000_000);
    }

    #[test]
    fn test_request_ids_increment() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(make_document_policy()).unwrap();

        let r1 = engine.check(&CheckRequest::new(make_viewer(), make_document(), "read"));
        let r2 = engine.check(&CheckRequest::new(make_viewer(), make_document(), "read"));

        assert_eq!(r1.request_id, "req-1");
        assert_eq!(r2.request_id, "req-2");
    }
}
