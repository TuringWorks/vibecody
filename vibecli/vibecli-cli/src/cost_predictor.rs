//! Real-time cost prediction and budget enforcement engine.
//!
//! Rivals Claude Code Cost Observer, Cursor Pricing Panel, and Devin Budget Dashboard with:
//! - Predictive cost forecasting per task type before execution
//! - Hard budget cap with agent rejection on overage
//! - Per-session, per-user, and per-project budget tracking
//! - Token-level cost breakdown: input, output, cache read/write
//! - Multi-provider cost normalization (all quoted in USD/1M tokens)
//! - Burn-rate alerts: warn at 70%, block at 100%
//! - Cost anomaly detection (sudden spikes vs. rolling average)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Token Pricing ────────────────────────────────────────────────────────────

/// Price per 1M tokens (in USD) for a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenPrice {
    pub model_id: String,
    pub provider: String,
    pub input_per_1m: f64,
    pub output_per_1m: f64,
    pub cache_read_per_1m: f64,
    pub cache_write_per_1m: f64,
}

impl TokenPrice {
    pub fn new(model_id: &str, provider: &str, input: f64, output: f64, cache_r: f64, cache_w: f64) -> Self {
        Self {
            model_id: model_id.to_string(),
            provider: provider.to_string(),
            input_per_1m: input,
            output_per_1m: output,
            cache_read_per_1m: cache_r,
            cache_write_per_1m: cache_w,
        }
    }

    /// Compute cost for a given token breakdown.
    pub fn compute_cost(&self, input_tokens: u64, output_tokens: u64, cache_read: u64, cache_write: u64) -> f64 {
        let input_cost  = (input_tokens  as f64 / 1_000_000.0) * self.input_per_1m;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_per_1m;
        let cr_cost     = (cache_read    as f64 / 1_000_000.0) * self.cache_read_per_1m;
        let cw_cost     = (cache_write   as f64 / 1_000_000.0) * self.cache_write_per_1m;
        input_cost + output_cost + cr_cost + cw_cost
    }
}

/// Built-in pricing table (April 2026 rates).
pub fn builtin_prices() -> Vec<TokenPrice> {
    vec![
        TokenPrice::new("claude-opus-4-6",   "anthropic", 15.0, 75.0, 1.5, 18.75),
        TokenPrice::new("claude-sonnet-4-6",  "anthropic",  3.0, 15.0, 0.3,  3.75),
        TokenPrice::new("claude-haiku-4-5",   "anthropic",  0.8,  4.0, 0.08, 1.0),
        TokenPrice::new("gpt-4o",             "openai",     5.0, 15.0, 0.5,  0.0),
        TokenPrice::new("gpt-4o-mini",        "openai",     0.15, 0.6, 0.075, 0.0),
        TokenPrice::new("gemini-2.5-pro",     "google",     3.5, 10.5, 0.0,  0.0),
        TokenPrice::new("gemini-2.5-flash",   "google",     0.15, 0.6, 0.0,  0.0),
        TokenPrice::new("llama-3.1-70b",      "ollama",     0.0,  0.0, 0.0,  0.0), // local = free
        TokenPrice::new("deepseek-r1",        "deepseek",   0.55, 2.19, 0.0, 0.0),
    ]
}

// ─── Budget Management ────────────────────────────────────────────────────────

/// Budget period granularity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetPeriod {
    Session,
    Daily,
    Weekly,
    Monthly,
    AllTime,
}

/// A budget allocation for a scope (user / project / session).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: String,
    pub scope: String,
    pub period: BudgetPeriod,
    pub limit_usd: f64,
    pub spent_usd: f64,
    pub warn_threshold: f64,   // fraction (e.g. 0.7 = 70%)
    pub block_threshold: f64,  // fraction (e.g. 1.0 = 100%)
}

impl Budget {
    pub fn new(id: &str, scope: &str, period: BudgetPeriod, limit_usd: f64) -> Self {
        Self {
            id: id.to_string(),
            scope: scope.to_string(),
            period,
            limit_usd,
            spent_usd: 0.0,
            warn_threshold: 0.7,
            block_threshold: 1.0,
        }
    }

    pub fn remaining(&self) -> f64 { (self.limit_usd - self.spent_usd).max(0.0) }

    pub fn utilization(&self) -> f64 {
        if self.limit_usd == 0.0 { 1.0 } else { self.spent_usd / self.limit_usd }
    }

    pub fn is_warning(&self) -> bool { self.utilization() >= self.warn_threshold }
    pub fn is_blocked(&self) -> bool { self.utilization() >= self.block_threshold }

    pub fn can_spend(&self, amount: f64) -> bool {
        self.spent_usd + amount <= self.limit_usd * self.block_threshold
    }

    pub fn record_spend(&mut self, amount: f64) {
        self.spent_usd += amount;
    }
}

/// Budget enforcement decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BudgetDecision {
    Allow,
    Warn { message: String, remaining_usd: f64 },
    Block { reason: String },
}

impl BudgetDecision {
    pub fn is_allowed(&self) -> bool { matches!(self, Self::Allow | Self::Warn { .. }) }
    pub fn is_blocked(&self) -> bool { matches!(self, Self::Block { .. }) }
}

// ─── Cost Prediction ──────────────────────────────────────────────────────────

/// Task type for cost estimation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    SimpleCompletion,   // short autocomplete
    CodeGeneration,     // generate a function/class
    Refactoring,        // rewrite existing code
    Explanation,        // explain code
    Testing,            // generate tests
    LargeContextRead,   // summarise a big codebase
    MultiTurnChat,      // interactive session (per turn)
    AgentLoop,          // full agent run with tool calls
}

impl TaskType {
    /// Estimated token ranges (input, output).
    pub fn token_estimate(&self) -> (u64, u64) {
        match self {
            Self::SimpleCompletion  => (200,   100),
            Self::CodeGeneration    => (800,   600),
            Self::Refactoring       => (2_000, 1_500),
            Self::Explanation       => (3_000,   800),
            Self::Testing           => (1_500, 1_200),
            Self::LargeContextRead  => (40_000, 2_000),
            Self::MultiTurnChat     => (1_200,   400),
            Self::AgentLoop         => (8_000, 4_000),
        }
    }
}

/// A cost prediction for a task before it is executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostPrediction {
    pub task_type: TaskType,
    pub model_id: String,
    pub estimated_input_tokens: u64,
    pub estimated_output_tokens: u64,
    pub estimated_cost_usd: f64,
    pub confidence: u8,  // 0-100
    pub alternatives: Vec<CostAlternative>,
}

/// An alternative model/strategy with lower cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAlternative {
    pub model_id: String,
    pub estimated_cost_usd: f64,
    pub quality_tradeoff: String,
}

/// Actual cost of a completed request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub request_id: String,
    pub model_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub actual_cost_usd: f64,
    pub task_type: TaskType,
    pub timestamp: u64,
}

/// Anomaly detection result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CostAnomaly {
    Normal,
    Spike { factor: f64, message: String },
    Runaway { total_usd: f64, message: String },
}

// ─── Cost Predictor Engine ────────────────────────────────────────────────────

/// Core cost prediction and budget enforcement engine.
pub struct CostPredictor {
    prices: HashMap<String, TokenPrice>,
    budgets: Vec<Budget>,
    history: Vec<CostRecord>,
}

impl CostPredictor {
    pub fn new() -> Self {
        let prices: HashMap<String, TokenPrice> = builtin_prices()
            .into_iter()
            .map(|p| (p.model_id.clone(), p))
            .collect();
        Self { prices, budgets: Vec::new(), history: Vec::new() }
    }

    /// Add a custom price entry.
    pub fn add_price(&mut self, price: TokenPrice) {
        self.prices.insert(price.model_id.clone(), price);
    }

    /// Add a budget.
    pub fn add_budget(&mut self, budget: Budget) {
        self.budgets.push(budget);
    }

    /// Predict cost for a task on a given model.
    pub fn predict(&self, task: &TaskType, model_id: &str) -> Option<CostPrediction> {
        let price = self.prices.get(model_id)?;
        let (input, output) = task.token_estimate();
        let cost = price.compute_cost(input, output, 0, 0);
        let alternatives = self.cheaper_alternatives(task, model_id, cost);
        Some(CostPrediction {
            task_type: task.clone(),
            model_id: model_id.to_string(),
            estimated_input_tokens: input,
            estimated_output_tokens: output,
            estimated_cost_usd: cost,
            confidence: 75,
            alternatives,
        })
    }

    fn cheaper_alternatives(&self, task: &TaskType, current_model: &str, current_cost: f64) -> Vec<CostAlternative> {
        let (input, output) = task.token_estimate();
        let mut alts: Vec<CostAlternative> = self.prices.values()
            .filter(|p| p.model_id != current_model)
            .filter_map(|p| {
                let cost = p.compute_cost(input, output, 0, 0);
                if cost < current_cost * 0.8 {  // at least 20% cheaper
                    Some(CostAlternative {
                        model_id: p.model_id.clone(),
                        estimated_cost_usd: cost,
                        quality_tradeoff: if cost == 0.0 {
                            "Local model (free, lower quality)".into()
                        } else if cost < current_cost * 0.3 {
                            "Significantly cheaper, may sacrifice quality".into()
                        } else {
                            "Slightly cheaper, similar quality tier".into()
                        },
                    })
                } else { None }
            })
            .collect();
        alts.sort_by(|a, b| a.estimated_cost_usd.partial_cmp(&b.estimated_cost_usd).unwrap_or(std::cmp::Ordering::Equal));
        alts.truncate(3);
        alts
    }

    /// Check budgets before allowing a request.
    pub fn check_budgets(&self, scope: &str, estimated_cost: f64) -> BudgetDecision {
        for budget in &self.budgets {
            if budget.scope == scope || budget.scope == "*" {
                if budget.is_blocked() {
                    return BudgetDecision::Block {
                        reason: format!("Budget '{}' exhausted (${:.4} spent of ${:.2})", budget.id, budget.spent_usd, budget.limit_usd),
                    };
                }
                if !budget.can_spend(estimated_cost) {
                    return BudgetDecision::Block {
                        reason: format!("Request would exceed budget '{}' by ${:.4}", budget.id, (budget.spent_usd + estimated_cost) - budget.limit_usd),
                    };
                }
                if budget.is_warning() {
                    return BudgetDecision::Warn {
                        message: format!("Budget '{}' at {:.0}% ({:.0}% warn threshold)", budget.id, budget.utilization() * 100.0, budget.warn_threshold * 100.0),
                        remaining_usd: budget.remaining(),
                    };
                }
            }
        }
        BudgetDecision::Allow
    }

    /// Record an actual cost and update all matching budgets.
    pub fn record_actual(&mut self, record: CostRecord) {
        let scope = record.model_id.split('/').next().unwrap_or("*").to_string();
        let cost = record.actual_cost_usd;
        self.history.push(record);
        for budget in &mut self.budgets {
            if budget.scope == "*" || budget.scope == scope {
                budget.record_spend(cost);
            }
        }
    }

    /// Detect cost anomalies vs. rolling average.
    pub fn detect_anomaly(&self, window: usize) -> CostAnomaly {
        if self.history.len() < 3 { return CostAnomaly::Normal; }
        // Check runaway total first (independent of baseline)
        let total: f64 = self.history.iter().map(|r| r.actual_cost_usd).sum();
        if total > 100.0 {
            return CostAnomaly::Runaway { total_usd: total, message: format!("Session total ${total:.2} — review agent loops") };
        }
        // Spike detection requires a baseline window
        let baseline_count = self.history.len().saturating_sub(window);
        if baseline_count == 0 { return CostAnomaly::Normal; }
        let recent: Vec<f64> = self.history.iter().rev().take(window).map(|r| r.actual_cost_usd).collect();
        let oldest = self.history.iter().rev().skip(window).take(window).map(|r| r.actual_cost_usd);
        let baseline: f64 = oldest.sum::<f64>() / baseline_count as f64;
        let recent_avg = recent.iter().sum::<f64>() / recent.len() as f64;
        if baseline > 0.0 {
            let factor = recent_avg / baseline;
            if factor > 5.0 {
                return CostAnomaly::Spike { factor, message: format!("Recent cost {factor:.1}x above baseline") };
            }
        }
        CostAnomaly::Normal
    }

    /// Total cost across all history.
    pub fn total_spent(&self) -> f64 {
        self.history.iter().map(|r| r.actual_cost_usd).sum()
    }

    /// Cost breakdown by model.
    pub fn by_model(&self) -> HashMap<String, f64> {
        let mut map: HashMap<String, f64> = HashMap::new();
        for rec in &self.history {
            *map.entry(rec.model_id.clone()).or_insert(0.0) += rec.actual_cost_usd;
        }
        map
    }

    pub fn history(&self) -> &[CostRecord] { &self.history }
    pub fn budgets(&self) -> &[Budget] { &self.budgets }
}

impl Default for CostPredictor {
    fn default() -> Self { Self::new() }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(model: &str, cost: f64) -> CostRecord {
        CostRecord {
            request_id: format!("req-{}", model),
            model_id: model.to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            actual_cost_usd: cost,
            task_type: TaskType::CodeGeneration,
            timestamp: 0,
        }
    }

    // ── TokenPrice ────────────────────────────────────────────────────────

    #[test]
    fn test_token_price_compute_cost_zero() {
        let p = TokenPrice::new("m", "p", 0.0, 0.0, 0.0, 0.0);
        assert_eq!(p.compute_cost(1_000_000, 1_000_000, 0, 0), 0.0);
    }

    #[test]
    fn test_token_price_compute_cost_basic() {
        let p = TokenPrice::new("m", "p", 10.0, 30.0, 0.0, 0.0);
        // 1M input = $10, 1M output = $30
        let cost = p.compute_cost(1_000_000, 1_000_000, 0, 0);
        assert!((cost - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_token_price_compute_cost_with_cache() {
        let p = TokenPrice::new("m", "p", 10.0, 30.0, 1.0, 5.0);
        let cost = p.compute_cost(0, 0, 1_000_000, 1_000_000);
        assert!((cost - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_token_price_partial_million() {
        let p = TokenPrice::new("m", "p", 1.0, 0.0, 0.0, 0.0);
        let cost = p.compute_cost(500_000, 0, 0, 0);
        assert!((cost - 0.5).abs() < 0.01);
    }

    // ── builtin_prices ────────────────────────────────────────────────────

    #[test]
    fn test_builtin_prices_non_empty() {
        assert!(!builtin_prices().is_empty());
    }

    #[test]
    fn test_builtin_prices_contains_claude_sonnet() {
        assert!(builtin_prices().iter().any(|p| p.model_id.contains("sonnet")));
    }

    #[test]
    fn test_builtin_prices_ollama_free() {
        let ollama = builtin_prices().into_iter().find(|p| p.provider == "ollama").unwrap();
        assert_eq!(ollama.compute_cost(1_000_000, 1_000_000, 0, 0), 0.0);
    }

    // ── Budget ────────────────────────────────────────────────────────────

    #[test]
    fn test_budget_new_zero_spent() {
        let b = Budget::new("b1", "user:alice", BudgetPeriod::Daily, 10.0);
        assert_eq!(b.spent_usd, 0.0);
        assert_eq!(b.remaining(), 10.0);
    }

    #[test]
    fn test_budget_utilization() {
        let mut b = Budget::new("b1", "u", BudgetPeriod::Daily, 10.0);
        b.spent_usd = 5.0;
        assert!((b.utilization() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_budget_is_warning_at_70pct() {
        let mut b = Budget::new("b1", "u", BudgetPeriod::Daily, 10.0);
        b.spent_usd = 7.0;
        assert!(b.is_warning());
    }

    #[test]
    fn test_budget_not_warning_below_70pct() {
        let mut b = Budget::new("b1", "u", BudgetPeriod::Daily, 10.0);
        b.spent_usd = 5.0;
        assert!(!b.is_warning());
    }

    #[test]
    fn test_budget_is_blocked_at_100pct() {
        let mut b = Budget::new("b1", "u", BudgetPeriod::Daily, 10.0);
        b.spent_usd = 10.0;
        assert!(b.is_blocked());
    }

    #[test]
    fn test_budget_can_spend_true() {
        let b = Budget::new("b1", "u", BudgetPeriod::Daily, 10.0);
        assert!(b.can_spend(5.0));
    }

    #[test]
    fn test_budget_can_spend_false_would_exceed() {
        let mut b = Budget::new("b1", "u", BudgetPeriod::Daily, 10.0);
        b.spent_usd = 9.0;
        assert!(!b.can_spend(2.0));
    }

    #[test]
    fn test_budget_remaining_never_negative() {
        let mut b = Budget::new("b1", "u", BudgetPeriod::Daily, 5.0);
        b.spent_usd = 10.0;
        assert_eq!(b.remaining(), 0.0);
    }

    #[test]
    fn test_budget_record_spend() {
        let mut b = Budget::new("b1", "u", BudgetPeriod::Daily, 100.0);
        b.record_spend(12.5);
        assert!((b.spent_usd - 12.5).abs() < 0.001);
    }

    // ── BudgetDecision ────────────────────────────────────────────────────

    #[test]
    fn test_budget_decision_allow_is_allowed() {
        assert!(BudgetDecision::Allow.is_allowed());
        assert!(!BudgetDecision::Allow.is_blocked());
    }

    #[test]
    fn test_budget_decision_warn_is_allowed() {
        let d = BudgetDecision::Warn { message: "".into(), remaining_usd: 1.0 };
        assert!(d.is_allowed());
        assert!(!d.is_blocked());
    }

    #[test]
    fn test_budget_decision_block_is_blocked() {
        let d = BudgetDecision::Block { reason: "over".into() };
        assert!(d.is_blocked());
        assert!(!d.is_allowed());
    }

    // ── TaskType ──────────────────────────────────────────────────────────

    #[test]
    fn test_task_type_agent_loop_large_tokens() {
        let (input, output) = TaskType::AgentLoop.token_estimate();
        assert!(input > 1000);
        assert!(output > 1000);
    }

    #[test]
    fn test_task_type_simple_completion_small_tokens() {
        let (input, output) = TaskType::SimpleCompletion.token_estimate();
        assert!(input < 1000);
        assert!(output < 500);
    }

    // ── CostPredictor ─────────────────────────────────────────────────────

    #[test]
    fn test_predictor_predict_known_model() {
        let pred = CostPredictor::new();
        let result = pred.predict(&TaskType::CodeGeneration, "claude-sonnet-4-6");
        assert!(result.is_some());
        let p = result.unwrap();
        assert!(p.estimated_cost_usd > 0.0);
    }

    #[test]
    fn test_predictor_predict_unknown_model() {
        let pred = CostPredictor::new();
        let result = pred.predict(&TaskType::CodeGeneration, "nonexistent-model");
        assert!(result.is_none());
    }

    #[test]
    fn test_predictor_predict_ollama_is_free() {
        let pred = CostPredictor::new();
        let result = pred.predict(&TaskType::LargeContextRead, "llama-3.1-70b").unwrap();
        assert_eq!(result.estimated_cost_usd, 0.0);
    }

    #[test]
    fn test_predictor_cheaper_alternatives() {
        let pred = CostPredictor::new();
        let result = pred.predict(&TaskType::AgentLoop, "claude-opus-4-6").unwrap();
        // Opus is the most expensive; should have cheaper alternatives
        assert!(!result.alternatives.is_empty());
        assert!(result.alternatives[0].estimated_cost_usd < result.estimated_cost_usd);
    }

    #[test]
    fn test_predictor_budget_allow() {
        let mut pred = CostPredictor::new();
        pred.add_budget(Budget::new("b1", "*", BudgetPeriod::Daily, 100.0));
        let decision = pred.check_budgets("*", 0.001);
        assert_eq!(decision, BudgetDecision::Allow);
    }

    #[test]
    fn test_predictor_budget_warn_at_threshold() {
        let mut pred = CostPredictor::new();
        let mut b = Budget::new("b1", "*", BudgetPeriod::Daily, 10.0);
        b.spent_usd = 8.0; // 80% > 70% warn threshold
        pred.add_budget(b);
        let decision = pred.check_budgets("*", 0.001);
        assert!(matches!(decision, BudgetDecision::Warn { .. }));
    }

    #[test]
    fn test_predictor_budget_block_when_exhausted() {
        let mut pred = CostPredictor::new();
        let mut b = Budget::new("b1", "*", BudgetPeriod::Daily, 5.0);
        b.spent_usd = 5.0; // 100%
        pred.add_budget(b);
        let decision = pred.check_budgets("*", 0.001);
        assert!(decision.is_blocked());
    }

    #[test]
    fn test_predictor_budget_block_would_exceed() {
        let mut pred = CostPredictor::new();
        let mut b = Budget::new("b1", "*", BudgetPeriod::Daily, 5.0);
        b.spent_usd = 4.5;
        pred.add_budget(b);
        let decision = pred.check_budgets("*", 1.0); // would push to $5.5
        assert!(decision.is_blocked());
    }

    #[test]
    fn test_predictor_record_actual_updates_history() {
        let mut pred = CostPredictor::new();
        pred.record_actual(make_record("claude-sonnet-4-6", 0.05));
        assert_eq!(pred.history().len(), 1);
        assert!((pred.total_spent() - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_predictor_record_actual_updates_budget() {
        let mut pred = CostPredictor::new();
        pred.add_budget(Budget::new("b1", "*", BudgetPeriod::Daily, 10.0));
        pred.record_actual(make_record("claude-sonnet-4-6", 2.0));
        assert!((pred.budgets()[0].spent_usd - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_predictor_total_spent() {
        let mut pred = CostPredictor::new();
        pred.record_actual(make_record("m1", 1.0));
        pred.record_actual(make_record("m2", 2.5));
        assert!((pred.total_spent() - 3.5).abs() < 0.001);
    }

    #[test]
    fn test_predictor_by_model() {
        let mut pred = CostPredictor::new();
        pred.record_actual(make_record("opus", 5.0));
        pred.record_actual(make_record("sonnet", 1.0));
        pred.record_actual(make_record("opus", 3.0));
        let by_model = pred.by_model();
        assert!((by_model["opus"] - 8.0).abs() < 0.001);
        assert!((by_model["sonnet"] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_predictor_anomaly_normal_with_few_records() {
        let pred = CostPredictor::new();
        assert_eq!(pred.detect_anomaly(5), CostAnomaly::Normal);
    }

    #[test]
    fn test_predictor_anomaly_spike_detected() {
        let mut pred = CostPredictor::new();
        // Baseline: 3 cheap requests
        for _ in 0..3 { pred.record_actual(make_record("m", 0.01)); }
        // Spike: 3 expensive requests (10x)
        for _ in 0..3 { pred.record_actual(make_record("m", 1.0)); }
        let anomaly = pred.detect_anomaly(3);
        assert!(matches!(anomaly, CostAnomaly::Spike { .. }));
    }

    #[test]
    fn test_predictor_anomaly_runaway_large_total() {
        let mut pred = CostPredictor::new();
        // Same level, but accumulate >$100
        for _ in 0..5 { pred.record_actual(make_record("m", 25.0)); }
        let anomaly = pred.detect_anomaly(5);
        assert!(matches!(anomaly, CostAnomaly::Runaway { .. }));
    }

    #[test]
    fn test_predictor_add_custom_price() {
        let mut pred = CostPredictor::new();
        pred.add_price(TokenPrice::new("my-model", "custom", 1.0, 2.0, 0.0, 0.0));
        assert!(pred.predict(&TaskType::SimpleCompletion, "my-model").is_some());
    }
}
