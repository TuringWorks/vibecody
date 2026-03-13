//! Usage metering and credit system for VibeCody.
//!
//! Provides granular usage tracking for team billing and cost allocation,
//! including budget management, alert thresholds, chargeback generation,
//! and per-user/project/provider reporting.

use std::collections::HashMap;

/// Type of task that consumed tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    Chat,
    AgentRun,
    CodeReview,
    TestGeneration,
    Completion,
    Embedding,
    BatchJob,
    Custom(String),
}

impl TaskType {
    fn as_str(&self) -> &str {
        match self {
            TaskType::Chat => "Chat",
            TaskType::AgentRun => "AgentRun",
            TaskType::CodeReview => "CodeReview",
            TaskType::TestGeneration => "TestGeneration",
            TaskType::Completion => "Completion",
            TaskType::Embedding => "Embedding",
            TaskType::BatchJob => "BatchJob",
            TaskType::Custom(s) => s.as_str(),
        }
    }
}

/// A single usage record representing one metered operation.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageRecord {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub provider: String,
    pub model: String,
    pub task_type: TaskType,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub timestamp: u64,
    pub agent_id: Option<String>,
    pub duration_ms: u64,
}

/// Who owns a budget.
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetOwner {
    User(String),
    Team(String),
    Project(String),
    Global,
}

/// Budget period granularity.
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    Unlimited,
}

impl BudgetPeriod {
    /// Duration of the period in seconds.
    fn duration_secs(&self) -> Option<u64> {
        match self {
            BudgetPeriod::Daily => Some(86_400),
            BudgetPeriod::Weekly => Some(604_800),
            BudgetPeriod::Monthly => Some(2_592_000),
            BudgetPeriod::Quarterly => Some(7_776_000),
            BudgetPeriod::Yearly => Some(31_536_000),
            BudgetPeriod::Unlimited => None,
        }
    }
}

/// A credit budget with spending limits and alerts.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditBudget {
    pub id: String,
    pub name: String,
    pub owner_type: BudgetOwner,
    pub total_credits: f64,
    pub used_credits: f64,
    pub alert_threshold_percent: f64,
    pub hard_limit: bool,
    pub period: BudgetPeriod,
    pub period_start: u64,
}

/// Type of budget alert.
#[derive(Debug, Clone, PartialEq)]
pub enum AlertType {
    Warning,
    Critical,
    LimitReached,
}

/// A budget alert triggered when thresholds are crossed.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetAlert {
    pub budget_id: String,
    pub alert_type: AlertType,
    pub message: String,
    pub timestamp: u64,
    pub usage_percent: f64,
}

/// Per-provider usage aggregation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderUsage {
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub request_count: u64,
}

/// Per-model usage aggregation.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelUsage {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

/// Per-task-type usage aggregation.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskUsage {
    pub task_type: String,
    pub count: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

/// A usage report for a given time range.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageReport {
    pub period_start: u64,
    pub period_end: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub by_provider: HashMap<String, ProviderUsage>,
    pub by_model: HashMap<String, ModelUsage>,
    pub by_task: HashMap<String, TaskUsage>,
    pub by_user: HashMap<String, f64>,
    pub by_project: HashMap<String, f64>,
}

/// Pricing information for a specific model.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPricing {
    pub model: String,
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub provider: String,
}

/// A chargeback entry for cost allocation to departments.
#[derive(Debug, Clone, PartialEq)]
pub struct ChargebackEntry {
    pub project_id: String,
    pub department: String,
    pub cost_usd: f64,
    pub period: String,
}

/// Central usage metering engine.
#[derive(Debug, Clone)]
pub struct UsageMeter {
    pub records: Vec<UsageRecord>,
    pub budgets: HashMap<String, CreditBudget>,
    pub alerts: Vec<BudgetAlert>,
    pub pricing: HashMap<String, ModelPricing>,
}

impl UsageMeter {
    /// Create a new empty usage meter.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            budgets: HashMap::new(),
            alerts: Vec::new(),
            pricing: HashMap::new(),
        }
    }

    /// Record a usage event and check budgets, returning any triggered alerts.
    pub fn record_usage(&mut self, record: UsageRecord) -> Vec<BudgetAlert> {
        let mut new_alerts = Vec::new();
        let cost = record.cost_usd;
        let user_id = record.user_id.clone();
        let project_id = record.project_id.clone();
        self.records.push(record);

        // Check all budgets for matches
        let budget_ids: Vec<String> = self.budgets.keys().cloned().collect();
        for bid in budget_ids {
            let matches = {
                let b = &self.budgets[&bid];
                match &b.owner_type {
                    BudgetOwner::User(uid) => *uid == user_id,
                    BudgetOwner::Team(_) => false, // team matching would require team membership lookup
                    BudgetOwner::Project(pid) => *pid == project_id,
                    BudgetOwner::Global => true,
                }
            };
            if matches {
                let b = self.budgets.get_mut(&bid).expect("budget must exist");
                b.used_credits += cost;
                let usage_pct = if b.total_credits > 0.0 {
                    (b.used_credits / b.total_credits) * 100.0
                } else {
                    0.0
                };

                if usage_pct >= 100.0 {
                    let alert = BudgetAlert {
                        budget_id: b.id.clone(),
                        alert_type: AlertType::LimitReached,
                        message: format!("Budget '{}' limit reached ({:.1}%)", b.name, usage_pct),
                        timestamp: self.records.last().map(|r| r.timestamp).unwrap_or(0),
                        usage_percent: usage_pct,
                    };
                    new_alerts.push(alert);
                } else if usage_pct >= 90.0 {
                    let alert = BudgetAlert {
                        budget_id: b.id.clone(),
                        alert_type: AlertType::Critical,
                        message: format!("Budget '{}' at critical level ({:.1}%)", b.name, usage_pct),
                        timestamp: self.records.last().map(|r| r.timestamp).unwrap_or(0),
                        usage_percent: usage_pct,
                    };
                    new_alerts.push(alert);
                } else if usage_pct >= b.alert_threshold_percent {
                    let alert = BudgetAlert {
                        budget_id: b.id.clone(),
                        alert_type: AlertType::Warning,
                        message: format!(
                            "Budget '{}' exceeded warning threshold ({:.1}%)",
                            b.name, usage_pct
                        ),
                        timestamp: self.records.last().map(|r| r.timestamp).unwrap_or(0),
                        usage_percent: usage_pct,
                    };
                    new_alerts.push(alert);
                }
            }
        }

        self.alerts.extend(new_alerts.clone());
        new_alerts
    }

    /// Create a new budget, returning its id.
    pub fn create_budget(&mut self, budget: CreditBudget) -> String {
        let id = budget.id.clone();
        self.budgets.insert(id.clone(), budget);
        id
    }

    /// Get a budget by id.
    pub fn get_budget(&self, id: &str) -> Option<&CreditBudget> {
        self.budgets.get(id)
    }

    /// Update the total credits for a budget.
    pub fn update_budget_credits(&mut self, id: &str, new_total: f64) -> Result<(), String> {
        match self.budgets.get_mut(id) {
            Some(b) => {
                b.total_credits = new_total;
                Ok(())
            }
            None => Err(format!("Budget '{}' not found", id)),
        }
    }

    /// Find the first budget matching an owner.
    pub fn check_budget(&self, owner: &BudgetOwner) -> Option<&CreditBudget> {
        self.budgets.values().find(|b| b.owner_type == *owner)
    }

    /// Check if an owner is over budget.
    pub fn is_over_budget(&self, owner: &BudgetOwner) -> bool {
        match self.check_budget(owner) {
            Some(b) => b.used_credits >= b.total_credits,
            None => false,
        }
    }

    /// Remaining credits for an owner.
    pub fn remaining_credits(&self, owner: &BudgetOwner) -> f64 {
        match self.check_budget(owner) {
            Some(b) => (b.total_credits - b.used_credits).max(0.0),
            None => f64::INFINITY,
        }
    }

    /// Generate a usage report for a time range.
    pub fn generate_report(&self, start: u64, end: u64) -> UsageReport {
        self.build_report(start, end, |_| true)
    }

    /// Generate a usage report filtered to a single user.
    pub fn generate_report_for_user(&self, user_id: &str, start: u64, end: u64) -> UsageReport {
        self.build_report(start, end, |r| r.user_id == user_id)
    }

    /// Generate a usage report filtered to a single project.
    pub fn generate_report_for_project(
        &self,
        project_id: &str,
        start: u64,
        end: u64,
    ) -> UsageReport {
        self.build_report(start, end, |r| r.project_id == project_id)
    }

    fn build_report<F: Fn(&UsageRecord) -> bool>(
        &self,
        start: u64,
        end: u64,
        filter: F,
    ) -> UsageReport {
        let mut report = UsageReport {
            period_start: start,
            period_end: end,
            total_tokens: 0,
            total_cost_usd: 0.0,
            by_provider: HashMap::new(),
            by_model: HashMap::new(),
            by_task: HashMap::new(),
            by_user: HashMap::new(),
            by_project: HashMap::new(),
        };

        for r in &self.records {
            if r.timestamp < start || r.timestamp > end || !filter(r) {
                continue;
            }
            let tokens = r.input_tokens + r.output_tokens;
            report.total_tokens += tokens;
            report.total_cost_usd += r.cost_usd;

            // by_provider
            let prov = report
                .by_provider
                .entry(r.provider.clone())
                .or_insert_with(|| ProviderUsage {
                    provider: r.provider.clone(),
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_usd: 0.0,
                    request_count: 0,
                });
            prov.input_tokens += r.input_tokens;
            prov.output_tokens += r.output_tokens;
            prov.cost_usd += r.cost_usd;
            prov.request_count += 1;

            // by_model
            let mdl = report
                .by_model
                .entry(r.model.clone())
                .or_insert_with(|| ModelUsage {
                    model: r.model.clone(),
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_usd: 0.0,
                });
            mdl.input_tokens += r.input_tokens;
            mdl.output_tokens += r.output_tokens;
            mdl.cost_usd += r.cost_usd;

            // by_task
            let task_key = r.task_type.as_str().to_string();
            let tsk = report
                .by_task
                .entry(task_key.clone())
                .or_insert_with(|| TaskUsage {
                    task_type: task_key,
                    count: 0,
                    total_tokens: 0,
                    cost_usd: 0.0,
                });
            tsk.count += 1;
            tsk.total_tokens += tokens;
            tsk.cost_usd += r.cost_usd;

            // by_user
            *report.by_user.entry(r.user_id.clone()).or_insert(0.0) += r.cost_usd;

            // by_project
            *report.by_project.entry(r.project_id.clone()).or_insert(0.0) += r.cost_usd;
        }

        report
    }

    /// Add model pricing information.
    pub fn add_pricing(&mut self, pricing: ModelPricing) {
        self.pricing.insert(pricing.model.clone(), pricing);
    }

    /// Calculate cost for a given model and token counts.
    pub fn calculate_cost(&self, model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
        match self.pricing.get(model) {
            Some(p) => {
                (input_tokens as f64 * p.input_per_million / 1_000_000.0)
                    + (output_tokens as f64 * p.output_per_million / 1_000_000.0)
            }
            None => 0.0,
        }
    }

    /// Get all alerts.
    pub fn get_alerts(&self) -> &[BudgetAlert] {
        &self.alerts
    }

    /// Clear all alerts.
    pub fn clear_alerts(&mut self) {
        self.alerts.clear();
    }

    /// Reset budgets whose period has elapsed as of `now`.
    pub fn reset_period_budgets(&mut self, now: u64) {
        for budget in self.budgets.values_mut() {
            if let Some(dur) = budget.period.duration_secs() {
                if now >= budget.period_start + dur {
                    budget.used_credits = 0.0;
                    budget.period_start = now;
                }
            }
        }
    }

    /// Generate chargeback entries for cost allocation.
    pub fn generate_chargeback(
        &self,
        start: u64,
        end: u64,
        department_map: &HashMap<String, String>,
    ) -> Vec<ChargebackEntry> {
        let mut costs: HashMap<String, f64> = HashMap::new();
        for r in &self.records {
            if r.timestamp >= start && r.timestamp <= end {
                *costs.entry(r.project_id.clone()).or_insert(0.0) += r.cost_usd;
            }
        }

        let period_str = format!("{}-{}", start, end);
        let mut entries: Vec<ChargebackEntry> = costs
            .into_iter()
            .map(|(project_id, cost_usd)| {
                let department = department_map
                    .get(&project_id)
                    .cloned()
                    .unwrap_or_else(|| "unassigned".to_string());
                ChargebackEntry {
                    project_id,
                    department,
                    cost_usd,
                    period: period_str.clone(),
                }
            })
            .collect();
        entries.sort_by(|a, b| {
            b.cost_usd
                .partial_cmp(&a.cost_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries
    }

    /// Top consumers by cost in a time range.
    pub fn top_consumers(&self, limit: usize, start: u64, end: u64) -> Vec<(String, f64)> {
        let mut by_user: HashMap<String, f64> = HashMap::new();
        for r in &self.records {
            if r.timestamp >= start && r.timestamp <= end {
                *by_user.entry(r.user_id.clone()).or_insert(0.0) += r.cost_usd;
            }
        }
        let mut sorted: Vec<(String, f64)> = by_user.into_iter().collect();
        sorted.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(limit);
        sorted
    }

    /// Total spend in a time range.
    pub fn total_spend(&self, start: u64, end: u64) -> f64 {
        self.records
            .iter()
            .filter(|r| r.timestamp >= start && r.timestamp <= end)
            .map(|r| r.cost_usd)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(id: &str, user: &str, project: &str, cost: f64, ts: u64) -> UsageRecord {
        UsageRecord {
            id: id.to_string(),
            user_id: user.to_string(),
            project_id: project.to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            task_type: TaskType::Chat,
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: cost,
            timestamp: ts,
            agent_id: None,
            duration_ms: 200,
        }
    }

    fn make_budget(id: &str, owner: BudgetOwner, total: f64) -> CreditBudget {
        CreditBudget {
            id: id.to_string(),
            name: format!("Budget {}", id),
            owner_type: owner,
            total_credits: total,
            used_credits: 0.0,
            alert_threshold_percent: 80.0,
            hard_limit: true,
            period: BudgetPeriod::Monthly,
            period_start: 1000,
        }
    }

    #[test]
    fn test_new_meter_is_empty() {
        let meter = UsageMeter::new();
        assert!(meter.records.is_empty());
        assert!(meter.budgets.is_empty());
        assert!(meter.alerts.is_empty());
        assert!(meter.pricing.is_empty());
    }

    #[test]
    fn test_record_usage_stores_record() {
        let mut meter = UsageMeter::new();
        let r = make_record("r1", "alice", "proj1", 0.05, 2000);
        meter.record_usage(r);
        assert_eq!(meter.records.len(), 1);
        assert_eq!(meter.records[0].id, "r1");
    }

    #[test]
    fn test_record_multiple_usage() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "proj1", 0.05, 2000));
        meter.record_usage(make_record("r2", "bob", "proj2", 0.10, 2001));
        meter.record_usage(make_record("r3", "alice", "proj1", 0.03, 2002));
        assert_eq!(meter.records.len(), 3);
    }

    #[test]
    fn test_create_budget() {
        let mut meter = UsageMeter::new();
        let b = make_budget("b1", BudgetOwner::Global, 100.0);
        let id = meter.create_budget(b);
        assert_eq!(id, "b1");
        assert!(meter.budgets.contains_key("b1"));
    }

    #[test]
    fn test_get_budget() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 50.0));
        let b = meter.get_budget("b1").expect("budget should exist");
        assert_eq!(b.total_credits, 50.0);
        assert!(meter.get_budget("nonexistent").is_none());
    }

    #[test]
    fn test_update_budget_credits() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 50.0));
        assert!(meter.update_budget_credits("b1", 200.0).is_ok());
        assert_eq!(meter.get_budget("b1").expect("exists").total_credits, 200.0);
    }

    #[test]
    fn test_update_budget_credits_not_found() {
        let mut meter = UsageMeter::new();
        assert!(meter.update_budget_credits("nope", 100.0).is_err());
    }

    #[test]
    fn test_check_budget_global() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 100.0));
        assert!(meter.check_budget(&BudgetOwner::Global).is_some());
    }

    #[test]
    fn test_check_budget_user() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget(
            "b1",
            BudgetOwner::User("alice".into()),
            50.0,
        ));
        assert!(meter
            .check_budget(&BudgetOwner::User("alice".into()))
            .is_some());
        assert!(meter
            .check_budget(&BudgetOwner::User("bob".into()))
            .is_none());
    }

    #[test]
    fn test_is_over_budget_false_initially() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 100.0));
        assert!(!meter.is_over_budget(&BudgetOwner::Global));
    }

    #[test]
    fn test_is_over_budget_true_when_exceeded() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 0.10));
        meter.record_usage(make_record("r1", "alice", "proj1", 0.10, 2000));
        assert!(meter.is_over_budget(&BudgetOwner::Global));
    }

    #[test]
    fn test_is_over_budget_no_budget() {
        let meter = UsageMeter::new();
        assert!(!meter.is_over_budget(&BudgetOwner::Global));
    }

    #[test]
    fn test_remaining_credits() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 10.0));
        meter.record_usage(make_record("r1", "alice", "proj1", 3.0, 2000));
        let rem = meter.remaining_credits(&BudgetOwner::Global);
        assert!((rem - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_remaining_credits_no_budget() {
        let meter = UsageMeter::new();
        assert!(meter.remaining_credits(&BudgetOwner::Global).is_infinite());
    }

    #[test]
    fn test_remaining_credits_clamped_to_zero() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 1.0));
        meter.record_usage(make_record("r1", "alice", "proj1", 5.0, 2000));
        assert_eq!(meter.remaining_credits(&BudgetOwner::Global), 0.0);
    }

    #[test]
    fn test_alert_warning_threshold() {
        let mut meter = UsageMeter::new();
        let mut b = make_budget("b1", BudgetOwner::Global, 10.0);
        b.alert_threshold_percent = 80.0;
        meter.create_budget(b);
        // 85% usage
        let alerts = meter.record_usage(make_record("r1", "alice", "proj1", 8.5, 2000));
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, AlertType::Warning);
    }

    #[test]
    fn test_alert_critical_threshold() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 10.0));
        // 95% usage
        let alerts = meter.record_usage(make_record("r1", "alice", "proj1", 9.5, 2000));
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, AlertType::Critical);
    }

    #[test]
    fn test_alert_limit_reached() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 5.0));
        let alerts = meter.record_usage(make_record("r1", "alice", "proj1", 5.0, 2000));
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, AlertType::LimitReached);
    }

    #[test]
    fn test_no_alert_below_threshold() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 100.0));
        let alerts = meter.record_usage(make_record("r1", "alice", "proj1", 1.0, 2000));
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_alert_stored_in_meter() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 5.0));
        meter.record_usage(make_record("r1", "alice", "proj1", 5.0, 2000));
        assert_eq!(meter.get_alerts().len(), 1);
    }

    #[test]
    fn test_clear_alerts() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 5.0));
        meter.record_usage(make_record("r1", "alice", "proj1", 5.0, 2000));
        assert!(!meter.get_alerts().is_empty());
        meter.clear_alerts();
        assert!(meter.get_alerts().is_empty());
    }

    #[test]
    fn test_user_budget_tracking() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget(
            "b1",
            BudgetOwner::User("alice".into()),
            10.0,
        ));
        // alice's usage
        meter.record_usage(make_record("r1", "alice", "proj1", 4.0, 2000));
        // bob's usage should not affect alice's budget
        meter.record_usage(make_record("r2", "bob", "proj1", 20.0, 2001));
        assert!(!meter.is_over_budget(&BudgetOwner::User("alice".into())));
        let rem = meter.remaining_credits(&BudgetOwner::User("alice".into()));
        assert!((rem - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_project_budget_tracking() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget(
            "b1",
            BudgetOwner::Project("proj1".into()),
            5.0,
        ));
        meter.record_usage(make_record("r1", "alice", "proj1", 3.0, 2000));
        meter.record_usage(make_record("r2", "bob", "proj2", 10.0, 2001));
        assert!(!meter.is_over_budget(&BudgetOwner::Project("proj1".into())));
        meter.record_usage(make_record("r3", "alice", "proj1", 3.0, 2002));
        assert!(meter.is_over_budget(&BudgetOwner::Project("proj1".into())));
    }

    #[test]
    fn test_generate_report_empty() {
        let meter = UsageMeter::new();
        let report = meter.generate_report(0, 10000);
        assert_eq!(report.total_tokens, 0);
        assert_eq!(report.total_cost_usd, 0.0);
        assert!(report.by_provider.is_empty());
    }

    #[test]
    fn test_generate_report_basic() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "proj1", 0.05, 2000));
        meter.record_usage(make_record("r2", "bob", "proj2", 0.10, 2500));
        let report = meter.generate_report(1000, 3000);
        assert_eq!(report.total_tokens, 3000); // 2 * (1000+500)
        assert!((report.total_cost_usd - 0.15).abs() < 0.001);
        assert_eq!(report.by_provider.len(), 1);
        assert_eq!(report.by_user.len(), 2);
        assert_eq!(report.by_project.len(), 2);
    }

    #[test]
    fn test_generate_report_time_filter() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "proj1", 0.05, 1000));
        meter.record_usage(make_record("r2", "alice", "proj1", 0.10, 5000));
        let report = meter.generate_report(2000, 6000);
        assert_eq!(report.total_tokens, 1500);
        assert!((report.total_cost_usd - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_generate_report_for_user() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "proj1", 0.05, 2000));
        meter.record_usage(make_record("r2", "bob", "proj1", 0.10, 2500));
        let report = meter.generate_report_for_user("alice", 1000, 3000);
        assert_eq!(report.total_tokens, 1500);
        assert!((report.total_cost_usd - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_generate_report_for_project() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "proj1", 0.05, 2000));
        meter.record_usage(make_record("r2", "bob", "proj2", 0.10, 2500));
        let report = meter.generate_report_for_project("proj2", 1000, 3000);
        assert!((report.total_cost_usd - 0.10).abs() < 0.001);
        assert_eq!(report.by_user.len(), 1);
    }

    #[test]
    fn test_report_by_provider_aggregation() {
        let mut meter = UsageMeter::new();
        let mut r1 = make_record("r1", "alice", "proj1", 0.05, 2000);
        r1.provider = "openai".to_string();
        let mut r2 = make_record("r2", "bob", "proj1", 0.10, 2500);
        r2.provider = "anthropic".to_string();
        let mut r3 = make_record("r3", "alice", "proj1", 0.03, 2600);
        r3.provider = "openai".to_string();
        meter.record_usage(r1);
        meter.record_usage(r2);
        meter.record_usage(r3);
        let report = meter.generate_report(1000, 3000);
        assert_eq!(report.by_provider.len(), 2);
        let openai = &report.by_provider["openai"];
        assert_eq!(openai.request_count, 2);
        assert!((openai.cost_usd - 0.08).abs() < 0.001);
    }

    #[test]
    fn test_report_by_task_type() {
        let mut meter = UsageMeter::new();
        let mut r1 = make_record("r1", "alice", "proj1", 0.05, 2000);
        r1.task_type = TaskType::CodeReview;
        let mut r2 = make_record("r2", "bob", "proj1", 0.10, 2500);
        r2.task_type = TaskType::Chat;
        meter.record_usage(r1);
        meter.record_usage(r2);
        let report = meter.generate_report(1000, 3000);
        assert_eq!(report.by_task.len(), 2);
        assert_eq!(report.by_task["CodeReview"].count, 1);
        assert_eq!(report.by_task["Chat"].count, 1);
    }

    #[test]
    fn test_add_pricing_and_calculate_cost() {
        let mut meter = UsageMeter::new();
        meter.add_pricing(ModelPricing {
            model: "gpt-4".to_string(),
            input_per_million: 30.0,
            output_per_million: 60.0,
            provider: "openai".to_string(),
        });
        let cost = meter.calculate_cost("gpt-4", 1_000_000, 500_000);
        assert!((cost - 60.0).abs() < 0.001); // 30 + 30
    }

    #[test]
    fn test_calculate_cost_unknown_model() {
        let meter = UsageMeter::new();
        assert_eq!(meter.calculate_cost("unknown", 1000, 500), 0.0);
    }

    #[test]
    fn test_calculate_cost_zero_tokens() {
        let mut meter = UsageMeter::new();
        meter.add_pricing(ModelPricing {
            model: "gpt-4".to_string(),
            input_per_million: 30.0,
            output_per_million: 60.0,
            provider: "openai".to_string(),
        });
        assert_eq!(meter.calculate_cost("gpt-4", 0, 0), 0.0);
    }

    #[test]
    fn test_reset_period_budgets() {
        let mut meter = UsageMeter::new();
        let mut b = make_budget("b1", BudgetOwner::Global, 100.0);
        b.period = BudgetPeriod::Daily;
        b.period_start = 1000;
        b.used_credits = 50.0;
        meter.create_budget(b);

        // Not enough time elapsed
        meter.reset_period_budgets(50_000);
        assert_eq!(
            meter.get_budget("b1").expect("exists").used_credits,
            50.0
        );

        // Enough time elapsed (86400 seconds)
        meter.reset_period_budgets(1000 + 86_400);
        assert_eq!(
            meter.get_budget("b1").expect("exists").used_credits,
            0.0
        );
    }

    #[test]
    fn test_reset_unlimited_budget_not_reset() {
        let mut meter = UsageMeter::new();
        let mut b = make_budget("b1", BudgetOwner::Global, 100.0);
        b.period = BudgetPeriod::Unlimited;
        b.used_credits = 80.0;
        meter.create_budget(b);
        meter.reset_period_budgets(999_999_999);
        assert_eq!(
            meter.get_budget("b1").expect("exists").used_credits,
            80.0
        );
    }

    #[test]
    fn test_generate_chargeback() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "proj1", 10.0, 2000));
        meter.record_usage(make_record("r2", "bob", "proj2", 5.0, 2500));
        meter.record_usage(make_record("r3", "alice", "proj1", 3.0, 2600));

        let mut dept_map = HashMap::new();
        dept_map.insert("proj1".to_string(), "engineering".to_string());
        dept_map.insert("proj2".to_string(), "research".to_string());

        let entries = meter.generate_chargeback(1000, 3000, &dept_map);
        assert_eq!(entries.len(), 2);
        // Sorted by cost descending
        assert_eq!(entries[0].project_id, "proj1");
        assert!((entries[0].cost_usd - 13.0).abs() < 0.001);
        assert_eq!(entries[0].department, "engineering");
        assert_eq!(entries[1].project_id, "proj2");
        assert_eq!(entries[1].department, "research");
    }

    #[test]
    fn test_chargeback_unassigned_department() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "proj_unknown", 5.0, 2000));
        let entries = meter.generate_chargeback(1000, 3000, &HashMap::new());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].department, "unassigned");
    }

    #[test]
    fn test_top_consumers() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "p1", 10.0, 2000));
        meter.record_usage(make_record("r2", "bob", "p1", 20.0, 2500));
        meter.record_usage(make_record("r3", "carol", "p1", 5.0, 2600));
        meter.record_usage(make_record("r4", "alice", "p1", 8.0, 2700));

        let top = meter.top_consumers(2, 1000, 3000);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "bob");
        assert!((top[0].1 - 20.0).abs() < 0.001);
        assert_eq!(top[1].0, "alice");
        assert!((top[1].1 - 18.0).abs() < 0.001);
    }

    #[test]
    fn test_top_consumers_empty() {
        let meter = UsageMeter::new();
        let top = meter.top_consumers(5, 0, 10000);
        assert!(top.is_empty());
    }

    #[test]
    fn test_total_spend() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "p1", 1.5, 2000));
        meter.record_usage(make_record("r2", "bob", "p1", 2.5, 2500));
        meter.record_usage(make_record("r3", "carol", "p1", 3.0, 5000));

        let spend = meter.total_spend(1000, 3000);
        assert!((spend - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_total_spend_no_records() {
        let meter = UsageMeter::new();
        assert_eq!(meter.total_spend(0, 10000), 0.0);
    }

    #[test]
    fn test_task_type_custom() {
        let t = TaskType::Custom("Summarize".to_string());
        assert_eq!(t.as_str(), "Summarize");
    }

    #[test]
    fn test_task_type_variants() {
        assert_eq!(TaskType::Chat.as_str(), "Chat");
        assert_eq!(TaskType::AgentRun.as_str(), "AgentRun");
        assert_eq!(TaskType::Embedding.as_str(), "Embedding");
        assert_eq!(TaskType::BatchJob.as_str(), "BatchJob");
    }

    #[test]
    fn test_budget_period_duration() {
        assert_eq!(BudgetPeriod::Daily.duration_secs(), Some(86_400));
        assert_eq!(BudgetPeriod::Weekly.duration_secs(), Some(604_800));
        assert_eq!(BudgetPeriod::Monthly.duration_secs(), Some(2_592_000));
        assert_eq!(BudgetPeriod::Quarterly.duration_secs(), Some(7_776_000));
        assert_eq!(BudgetPeriod::Yearly.duration_secs(), Some(31_536_000));
        assert!(BudgetPeriod::Unlimited.duration_secs().is_none());
    }

    #[test]
    fn test_multiple_budgets_multiple_alerts() {
        let mut meter = UsageMeter::new();
        meter.create_budget(make_budget("b1", BudgetOwner::Global, 10.0));
        meter.create_budget(make_budget(
            "b2",
            BudgetOwner::User("alice".into()),
            5.0,
        ));
        // alice uses 5.0 -> global at 50% (no alert), user at 100% (limit reached)
        let alerts = meter.record_usage(make_record("r1", "alice", "proj1", 5.0, 2000));
        assert!(alerts.iter().any(|a| a.alert_type == AlertType::LimitReached));
    }

    #[test]
    fn test_report_by_model_aggregation() {
        let mut meter = UsageMeter::new();
        let mut r1 = make_record("r1", "alice", "p1", 0.05, 2000);
        r1.model = "gpt-4".to_string();
        let mut r2 = make_record("r2", "bob", "p1", 0.10, 2500);
        r2.model = "claude-3".to_string();
        meter.record_usage(r1);
        meter.record_usage(r2);
        let report = meter.generate_report(1000, 3000);
        assert_eq!(report.by_model.len(), 2);
        assert!(report.by_model.contains_key("gpt-4"));
        assert!(report.by_model.contains_key("claude-3"));
    }

    #[test]
    fn test_record_with_agent_id() {
        let mut meter = UsageMeter::new();
        let mut r = make_record("r1", "alice", "p1", 0.05, 2000);
        r.agent_id = Some("agent-007".to_string());
        meter.record_usage(r);
        assert_eq!(
            meter.records[0].agent_id,
            Some("agent-007".to_string())
        );
    }

    #[test]
    fn test_chargeback_time_filter() {
        let mut meter = UsageMeter::new();
        meter.record_usage(make_record("r1", "alice", "p1", 10.0, 1000));
        meter.record_usage(make_record("r2", "alice", "p1", 5.0, 5000));
        let entries = meter.generate_chargeback(2000, 6000, &HashMap::new());
        assert_eq!(entries.len(), 1);
        assert!((entries[0].cost_usd - 5.0).abs() < 0.001);
    }
}
