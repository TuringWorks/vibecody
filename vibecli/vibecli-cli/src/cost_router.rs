//! Cost-optimized agent routing engine.
//!
//! Provides intelligent model selection based on task complexity, budget constraints,
//! quality requirements, and historical performance data. Supports multiple routing
//! strategies including A/B testing for continuous optimization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about an available AI model and its cost/quality characteristics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
    pub quality_score: f64,
    pub max_context_tokens: u32,
    pub supports_tools: bool,
    pub latency_ms_avg: u32,
}

/// Complexity classification for a task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskComplexity {
    Trivial,
    Simple,
    Moderate,
    Complex,
    Expert,
}

/// Profile describing a task to be routed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskProfile {
    pub description: String,
    pub file_count: usize,
    pub total_lines: usize,
    pub languages: Vec<String>,
    pub has_tests: bool,
    pub estimated_tokens: u32,
    pub complexity: TaskComplexity,
}

/// Strategy used to select a model for a task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    CheapestFirst,
    QualityFirst,
    Balanced,
    BudgetConstrained(f64),
}

/// The outcome of a routing decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub task_id: String,
    pub selected_model: String,
    pub strategy: RoutingStrategy,
    pub estimated_cost: f64,
    pub quality_score: f64,
    pub reasoning: String,
    pub alternatives: Vec<ModelAlternative>,
    pub timestamp: u64,
}

/// An alternative model that was considered but not selected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelAlternative {
    pub model_id: String,
    pub estimated_cost: f64,
    pub quality_score: f64,
    pub reason_not_selected: String,
}

/// Budget configuration for cost control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub total: f64,
    pub spent: f64,
    pub period: BudgetPeriod,
    pub hard_limit: bool,
    pub alert_threshold_percent: f64,
    /// Optional link to a company-scoped budget (company_id).
    pub company_id: Option<String>,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            total: 0.0,
            spent: 0.0,
            period: BudgetPeriod::Monthly,
            hard_limit: false,
            alert_threshold_percent: 80.0,
            company_id: None,
        }
    }
}

impl Budget {
    /// Construct a cost_router Budget from a company CompanyBudget record.
    pub fn from_company_budget(cb: &crate::company_budget::CompanyBudget) -> Self {
        Self {
            total: cb.limit_cents as f64 / 100.0,
            spent: cb.spent_cents as f64 / 100.0,
            period: BudgetPeriod::Monthly,
            hard_limit: cb.hard_stop,
            alert_threshold_percent: cb.alert_pct as f64,
            company_id: Some(cb.company_id.clone()),
        }
    }
}

/// Time period over which a budget applies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BudgetPeriod {
    Daily,
    Weekly,
    Monthly,
    PerProject,
    Unlimited,
}

/// Feedback on a completed routing decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingFeedback {
    pub decision_id: String,
    pub actual_cost: f64,
    pub success: bool,
    pub quality_rating: Option<f64>,
    pub timestamp: u64,
}

/// Aggregate metrics across all routing decisions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterMetrics {
    pub total_routed: u32,
    pub total_cost: f64,
    pub avg_cost: f64,
    pub cheapest_route: f64,
    pub most_expensive_route: f64,
    pub by_model: HashMap<String, ModelUsage>,
    pub by_strategy: HashMap<String, u32>,
}

impl Default for RouterMetrics {
    fn default() -> Self {
        Self {
            total_routed: 0,
            total_cost: 0.0,
            avg_cost: 0.0,
            cheapest_route: f64::MAX,
            most_expensive_route: 0.0,
            by_model: HashMap::new(),
            by_strategy: HashMap::new(),
        }
    }
}

/// Per-model usage statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelUsage {
    pub count: u32,
    pub total_cost: f64,
    pub success_count: u32,
    pub avg_quality: f64,
}

/// Heuristic complexity estimator based on task profile characteristics.
pub struct ComplexityEstimator;

impl ComplexityEstimator {
    /// Estimate task complexity from a profile.
    ///
    /// Base heuristic by total_lines: <50 Trivial, <200 Simple, <1000 Moderate,
    /// <5000 Complex, >=5000 Expert. Adjusted upward by language count (>3 bumps
    /// one level) and downward if tests are present.
    pub fn estimate(profile: &TaskProfile) -> TaskComplexity {
        let base = if profile.total_lines < 50 {
            0
        } else if profile.total_lines < 200 {
            1
        } else if profile.total_lines < 1000 {
            2
        } else if profile.total_lines < 5000 {
            3
        } else {
            4
        };

        let mut level = base;

        // More languages increases complexity
        if profile.languages.len() > 3 {
            level = (level + 1).min(4);
        }

        // Having tests reduces perceived complexity
        if profile.has_tests && level > 0 {
            level -= 1;
        }

        match level {
            0 => TaskComplexity::Trivial,
            1 => TaskComplexity::Simple,
            2 => TaskComplexity::Moderate,
            3 => TaskComplexity::Complex,
            _ => TaskComplexity::Expert,
        }
    }
}

/// A/B test result for a single invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbResult {
    pub cost: f64,
    pub success: bool,
    pub quality: f64,
}

/// An A/B experiment comparing two models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbExperiment {
    pub id: String,
    pub model_a: String,
    pub model_b: String,
    pub results_a: Vec<AbResult>,
    pub results_b: Vec<AbResult>,
}

/// A/B router for running experiments between models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbRouter {
    pub experiments: HashMap<String, AbExperiment>,
    pub active_experiment: Option<String>,
}

impl Default for AbRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl AbRouter {
    pub fn new() -> Self {
        Self {
            experiments: HashMap::new(),
            active_experiment: None,
        }
    }

    /// Create a new A/B experiment between two models and make it active.
    pub fn create_experiment(
        &mut self,
        id: &str,
        model_a: &str,
        model_b: &str,
    ) -> Result<(), String> {
        if self.experiments.contains_key(id) {
            return Err(format!("Experiment '{}' already exists", id));
        }
        let exp = AbExperiment {
            id: id.to_string(),
            model_a: model_a.to_string(),
            model_b: model_b.to_string(),
            results_a: Vec::new(),
            results_b: Vec::new(),
        };
        self.experiments.insert(id.to_string(), exp);
        self.active_experiment = Some(id.to_string());
        Ok(())
    }

    /// Route using the active experiment, alternating between model A and B.
    pub fn route_ab(&self) -> Result<String, String> {
        let exp_id = self
            .active_experiment
            .as_ref()
            .ok_or_else(|| "No active experiment".to_string())?;
        let exp = self
            .experiments
            .get(exp_id)
            .ok_or_else(|| "Active experiment not found".to_string())?;

        let total = exp.results_a.len() + exp.results_b.len();
        if total % 2 == 0 {
            Ok(exp.model_a.clone())
        } else {
            Ok(exp.model_b.clone())
        }
    }

    /// Record a result for the specified model in the active experiment.
    pub fn record_result(&mut self, model_id: &str, result: AbResult) -> Result<(), String> {
        let exp_id = self
            .active_experiment
            .as_ref()
            .ok_or_else(|| "No active experiment".to_string())?
            .clone();
        let exp = self
            .experiments
            .get_mut(&exp_id)
            .ok_or_else(|| "Active experiment not found".to_string())?;

        if model_id == exp.model_a {
            exp.results_a.push(result);
            Ok(())
        } else if model_id == exp.model_b {
            exp.results_b.push(result);
            Ok(())
        } else {
            Err(format!(
                "Model '{}' is not part of experiment '{}'",
                model_id, exp_id
            ))
        }
    }

    /// Determine the winner of an experiment based on composite score (quality - cost).
    /// Returns (winner_model_id, score_a, score_b).
    pub fn get_winner(&self, experiment_id: &str) -> Result<(String, f64, f64), String> {
        let exp = self
            .experiments
            .get(experiment_id)
            .ok_or_else(|| format!("Experiment '{}' not found", experiment_id))?;

        if exp.results_a.is_empty() && exp.results_b.is_empty() {
            return Err("No results recorded yet".to_string());
        }

        let score_a = Self::composite_score(&exp.results_a);
        let score_b = Self::composite_score(&exp.results_b);

        let winner = if score_a >= score_b {
            exp.model_a.clone()
        } else {
            exp.model_b.clone()
        };

        Ok((winner, score_a, score_b))
    }

    fn composite_score(results: &[AbResult]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        let n = results.len() as f64;
        let avg_quality: f64 = results.iter().map(|r| r.quality).sum::<f64>() / n;
        let avg_cost: f64 = results.iter().map(|r| r.cost).sum::<f64>() / n;
        let success_rate: f64 =
            results.iter().filter(|r| r.success).count() as f64 / n;
        // Weighted composite: quality matters most, penalize cost, reward success
        avg_quality * 0.5 + success_rate * 30.0 - avg_cost * 10.0
    }
}

/// Main cost router that selects models for tasks based on strategy and budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRouter {
    pub models: Vec<ModelInfo>,
    pub strategy: RoutingStrategy,
    pub budget: Option<Budget>,
    pub decision_log: Vec<RoutingDecision>,
    pub feedback_log: Vec<RoutingFeedback>,
    pub metrics: RouterMetrics,
}

impl CostRouter {
    /// Create a new router with the given strategy.
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            models: Vec::new(),
            strategy,
            budget: None,
            decision_log: Vec::new(),
            feedback_log: Vec::new(),
            metrics: RouterMetrics::default(),
        }
    }

    /// Register a model with the router.
    pub fn add_model(&mut self, model: ModelInfo) {
        self.models.push(model);
    }

    /// Remove a model by id. Returns true if found and removed.
    pub fn remove_model(&mut self, model_id: &str) -> bool {
        let before = self.models.len();
        self.models.retain(|m| m.id != model_id);
        self.models.len() < before
    }

    /// Estimate the cost of running a task on a model (input + output tokens).
    pub fn estimate_cost(model: &ModelInfo, estimated_tokens: u32) -> f64 {
        let input_tokens = estimated_tokens as f64;
        // Assume output is roughly 40% of input tokens
        let output_tokens = input_tokens * 0.4;
        (input_tokens / 1000.0) * model.input_cost_per_1k
            + (output_tokens / 1000.0) * model.output_cost_per_1k
    }

    /// Estimate complexity for a task profile (delegates to ComplexityEstimator).
    pub fn estimate_complexity(profile: &TaskProfile) -> TaskComplexity {
        ComplexityEstimator::estimate(profile)
    }

    /// Check whether the budget allows the given cost. Returns Ok(()) or an error.
    pub fn check_budget(&self, estimated_cost: f64) -> Result<(), String> {
        if let Some(ref budget) = self.budget {
            let remaining = budget.total - budget.spent;
            if budget.hard_limit && estimated_cost > remaining {
                return Err(format!(
                    "Budget exceeded: estimated ${:.4} but only ${:.4} remaining",
                    estimated_cost, remaining
                ));
            }
            let pct = (budget.spent + estimated_cost) / budget.total * 100.0;
            if pct >= budget.alert_threshold_percent && !budget.hard_limit {
                // Soft warning — still allow
            }
        }
        Ok(())
    }

    /// Return the remaining budget, or f64::INFINITY if no budget is set.
    pub fn remaining_budget(&self) -> f64 {
        match &self.budget {
            Some(b) => (b.total - b.spent).max(0.0),
            None => f64::INFINITY,
        }
    }

    /// Route a task to the best model given the current strategy and budget.
    pub fn route_task(&mut self, task_id: &str, profile: &TaskProfile) -> Result<RoutingDecision, String> {
        if self.models.is_empty() {
            return Err("No models registered".to_string());
        }

        // Filter models that can handle the token count
        let eligible: Vec<&ModelInfo> = self
            .models
            .iter()
            .filter(|m| m.max_context_tokens >= profile.estimated_tokens)
            .collect();

        if eligible.is_empty() {
            return Err("No model can handle the estimated token count".to_string());
        }

        let strategy = self.strategy.clone();
        let (selected, reasoning) = match &strategy {
            RoutingStrategy::CheapestFirst => self.select_cheapest(&eligible, profile),
            RoutingStrategy::QualityFirst => self.select_quality(&eligible),
            RoutingStrategy::Balanced => self.select_balanced(&eligible, profile),
            RoutingStrategy::BudgetConstrained(limit) => {
                self.select_budget_constrained(&eligible, profile, *limit)
            }
        }?;

        let est_cost = Self::estimate_cost(&selected, profile.estimated_tokens);

        // Budget check
        self.check_budget(est_cost)?;

        // Build alternatives list
        let alternatives: Vec<ModelAlternative> = eligible
            .iter()
            .filter(|m| m.id != selected.id)
            .map(|m| {
                let cost = Self::estimate_cost(m, profile.estimated_tokens);
                let reason = if cost > est_cost {
                    format!("Higher cost: ${:.4}", cost)
                } else if m.quality_score < selected.quality_score {
                    format!("Lower quality: {:.1}", m.quality_score)
                } else {
                    "Not optimal for current strategy".to_string()
                };
                ModelAlternative {
                    model_id: m.id.clone(),
                    estimated_cost: cost,
                    quality_score: m.quality_score,
                    reason_not_selected: reason,
                }
            })
            .collect();

        let decision = RoutingDecision {
            task_id: task_id.to_string(),
            selected_model: selected.id.clone(),
            strategy: strategy.clone(),
            estimated_cost: est_cost,
            quality_score: selected.quality_score,
            reasoning,
            alternatives,
            timestamp: current_timestamp(),
        };

        // Update metrics
        self.metrics.total_routed += 1;
        self.metrics.total_cost += est_cost;
        self.metrics.avg_cost =
            self.metrics.total_cost / self.metrics.total_routed as f64;
        if est_cost < self.metrics.cheapest_route {
            self.metrics.cheapest_route = est_cost;
        }
        if est_cost > self.metrics.most_expensive_route {
            self.metrics.most_expensive_route = est_cost;
        }

        let strategy_key = match &strategy {
            RoutingStrategy::CheapestFirst => "CheapestFirst".to_string(),
            RoutingStrategy::QualityFirst => "QualityFirst".to_string(),
            RoutingStrategy::Balanced => "Balanced".to_string(),
            RoutingStrategy::BudgetConstrained(_) => "BudgetConstrained".to_string(),
        };
        *self.metrics.by_strategy.entry(strategy_key).or_insert(0) += 1;

        let usage = self
            .metrics
            .by_model
            .entry(selected.id.clone())
            .or_insert(ModelUsage {
                count: 0,
                total_cost: 0.0,
                success_count: 0,
                avg_quality: 0.0,
            });
        usage.count += 1;
        usage.total_cost += est_cost;

        // Update budget spent
        if let Some(ref mut budget) = self.budget {
            budget.spent += est_cost;
        }

        self.decision_log.push(decision.clone());
        Ok(decision)
    }

    fn select_cheapest(
        &self,
        eligible: &[&ModelInfo],
        profile: &TaskProfile,
    ) -> Result<(ModelInfo, String), String> {
        let best = eligible
            .iter()
            .min_by(|a, b| {
                let ca = Self::estimate_cost(a, profile.estimated_tokens);
                let cb = Self::estimate_cost(b, profile.estimated_tokens);
                ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| "No eligible models".to_string())?;

        Ok((
            (*best).clone(),
            format!(
                "Selected cheapest model '{}' at ${:.4}/1k input",
                best.name, best.input_cost_per_1k
            ),
        ))
    }

    fn select_quality(
        &self,
        eligible: &[&ModelInfo],
    ) -> Result<(ModelInfo, String), String> {
        let best = eligible
            .iter()
            .max_by(|a, b| {
                a.quality_score
                    .partial_cmp(&b.quality_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| "No eligible models".to_string())?;

        Ok((
            (*best).clone(),
            format!(
                "Selected highest quality model '{}' with score {:.1}",
                best.name, best.quality_score
            ),
        ))
    }

    fn select_balanced(
        &self,
        eligible: &[&ModelInfo],
        profile: &TaskProfile,
    ) -> Result<(ModelInfo, String), String> {
        // Balanced: maximize quality_score / cost ratio
        let best = eligible
            .iter()
            .max_by(|a, b| {
                let ca = Self::estimate_cost(a, profile.estimated_tokens).max(0.0001);
                let cb = Self::estimate_cost(b, profile.estimated_tokens).max(0.0001);
                let ra = a.quality_score / ca;
                let rb = b.quality_score / cb;
                ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| "No eligible models".to_string())?;

        let cost = Self::estimate_cost(best, profile.estimated_tokens).max(0.0001);
        Ok((
            (*best).clone(),
            format!(
                "Selected balanced model '{}' (quality/cost ratio: {:.1})",
                best.name,
                best.quality_score / cost
            ),
        ))
    }

    fn select_budget_constrained(
        &self,
        eligible: &[&ModelInfo],
        profile: &TaskProfile,
        limit: f64,
    ) -> Result<(ModelInfo, String), String> {
        // Filter to models within the per-task budget limit, then pick highest quality
        let within_budget: Vec<&&ModelInfo> = eligible
            .iter()
            .filter(|m| Self::estimate_cost(m, profile.estimated_tokens) <= limit)
            .collect();

        if within_budget.is_empty() {
            return Err(format!(
                "No model fits within per-task budget of ${:.4}",
                limit
            ));
        }

        let best = within_budget
            .iter()
            .max_by(|a, b| {
                a.quality_score
                    .partial_cmp(&b.quality_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("within_budget is non-empty");

        Ok((
            (***best).clone(),
            format!(
                "Selected '{}' (best quality within ${:.4} budget)",
                best.name, limit
            ),
        ))
    }

    /// Record feedback for a previous routing decision.
    pub fn record_feedback(&mut self, feedback: RoutingFeedback) {
        // Update model usage metrics
        if let Some(decision) = self
            .decision_log
            .iter()
            .find(|d| d.task_id == feedback.decision_id)
        {
            let model_id = decision.selected_model.clone();
            if let Some(usage) = self.metrics.by_model.get_mut(&model_id) {
                if feedback.success {
                    usage.success_count += 1;
                }
                if let Some(q) = feedback.quality_rating {
                    let n = usage.count as f64;
                    usage.avg_quality = (usage.avg_quality * (n - 1.0) + q) / n;
                }
            }
        }
        self.feedback_log.push(feedback);
    }

    /// Adjust model quality scores based on accumulated feedback.
    pub fn adjust_quality_scores(&mut self) {
        // Group feedback by model
        let mut model_quality: HashMap<String, Vec<f64>> = HashMap::new();
        for fb in &self.feedback_log {
            if let Some(decision) = self
                .decision_log
                .iter()
                .find(|d| d.task_id == fb.decision_id)
            {
                if let Some(q) = fb.quality_rating {
                    model_quality
                        .entry(decision.selected_model.clone())
                        .or_default()
                        .push(q);
                }
            }
        }

        // Blend original score with feedback (70% feedback, 30% original)
        for model in &mut self.models {
            if let Some(ratings) = model_quality.get(&model.id) {
                if !ratings.is_empty() {
                    let avg: f64 = ratings.iter().sum::<f64>() / ratings.len() as f64;
                    model.quality_score = model.quality_score * 0.3 + avg * 0.7;
                }
            }
        }
    }

    /// Get recommendations for cost optimization.
    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recs = Vec::new();

        if self.models.is_empty() {
            recs.push("Add at least one model to begin routing.".to_string());
            return recs;
        }

        if self.metrics.total_routed == 0 {
            recs.push("No tasks routed yet. Start routing to collect data.".to_string());
            return recs;
        }

        // Check if one model dominates usage
        if let Some((top_model, usage)) = self
            .metrics
            .by_model
            .iter()
            .max_by_key(|(_, u)| u.count)
        {
            let pct = usage.count as f64 / self.metrics.total_routed as f64 * 100.0;
            if pct > 80.0 && self.models.len() > 1 {
                recs.push(format!(
                    "Model '{}' handles {:.0}% of tasks. Consider diversifying.",
                    top_model, pct
                ));
            }
        }

        // Budget warning
        if let Some(ref budget) = self.budget {
            let pct = budget.spent / budget.total * 100.0;
            if pct > budget.alert_threshold_percent {
                recs.push(format!(
                    "Budget usage at {:.1}% (${:.2} of ${:.2}). Consider switching to CheapestFirst.",
                    pct, budget.spent, budget.total
                ));
            }
        }

        // Suggest A/B testing if enough data
        if self.models.len() >= 2 && self.metrics.total_routed > 10 {
            recs.push(
                "Consider A/B testing between your top models to optimize quality/cost.".to_string(),
            );
        }

        if recs.is_empty() {
            recs.push("Routing is performing well. No changes recommended.".to_string());
        }

        recs
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cheap_model() -> ModelInfo {
        ModelInfo {
            id: "cheap-1".to_string(),
            name: "Cheap Model".to_string(),
            provider: "provider-a".to_string(),
            input_cost_per_1k: 0.001,
            output_cost_per_1k: 0.002,
            quality_score: 60.0,
            max_context_tokens: 8000,
            supports_tools: false,
            latency_ms_avg: 200,
        }
    }

    fn make_quality_model() -> ModelInfo {
        ModelInfo {
            id: "quality-1".to_string(),
            name: "Quality Model".to_string(),
            provider: "provider-b".to_string(),
            input_cost_per_1k: 0.03,
            output_cost_per_1k: 0.06,
            quality_score: 95.0,
            max_context_tokens: 128000,
            supports_tools: true,
            latency_ms_avg: 1500,
        }
    }

    fn make_balanced_model() -> ModelInfo {
        ModelInfo {
            id: "balanced-1".to_string(),
            name: "Balanced Model".to_string(),
            provider: "provider-c".to_string(),
            input_cost_per_1k: 0.005,
            output_cost_per_1k: 0.01,
            quality_score: 80.0,
            max_context_tokens: 32000,
            supports_tools: true,
            latency_ms_avg: 500,
        }
    }

    fn make_simple_profile() -> TaskProfile {
        TaskProfile {
            description: "Fix a typo".to_string(),
            file_count: 1,
            total_lines: 30,
            languages: vec!["rust".to_string()],
            has_tests: false,
            estimated_tokens: 500,
            complexity: TaskComplexity::Trivial,
        }
    }

    fn make_complex_profile() -> TaskProfile {
        TaskProfile {
            description: "Refactor authentication module".to_string(),
            file_count: 15,
            total_lines: 3000,
            languages: vec![
                "rust".to_string(),
                "typescript".to_string(),
                "sql".to_string(),
                "yaml".to_string(),
            ],
            has_tests: true,
            estimated_tokens: 12000,
            complexity: TaskComplexity::Complex,
        }
    }

    // --- Model registration tests ---

    #[test]
    fn test_add_model() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        assert_eq!(router.models.len(), 1);
        assert_eq!(router.models[0].id, "cheap-1");
    }

    #[test]
    fn test_add_multiple_models() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        router.add_model(make_quality_model());
        router.add_model(make_balanced_model());
        assert_eq!(router.models.len(), 3);
    }

    #[test]
    fn test_remove_model() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        router.add_model(make_quality_model());
        assert!(router.remove_model("cheap-1"));
        assert_eq!(router.models.len(), 1);
        assert_eq!(router.models[0].id, "quality-1");
    }

    #[test]
    fn test_remove_nonexistent_model() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        assert!(!router.remove_model("nonexistent"));
        assert_eq!(router.models.len(), 1);
    }

    // --- Complexity estimation tests ---

    #[test]
    fn test_complexity_trivial() {
        let profile = TaskProfile {
            description: "Tiny fix".to_string(),
            file_count: 1,
            total_lines: 20,
            languages: vec!["rust".to_string()],
            has_tests: false,
            estimated_tokens: 100,
            complexity: TaskComplexity::Trivial,
        };
        assert_eq!(ComplexityEstimator::estimate(&profile), TaskComplexity::Trivial);
    }

    #[test]
    fn test_complexity_simple() {
        let profile = TaskProfile {
            description: "Small change".to_string(),
            file_count: 2,
            total_lines: 100,
            languages: vec!["rust".to_string()],
            has_tests: false,
            estimated_tokens: 500,
            complexity: TaskComplexity::Simple,
        };
        assert_eq!(ComplexityEstimator::estimate(&profile), TaskComplexity::Simple);
    }

    #[test]
    fn test_complexity_moderate() {
        let profile = TaskProfile {
            description: "Medium task".to_string(),
            file_count: 5,
            total_lines: 500,
            languages: vec!["rust".to_string(), "ts".to_string()],
            has_tests: false,
            estimated_tokens: 2000,
            complexity: TaskComplexity::Moderate,
        };
        assert_eq!(ComplexityEstimator::estimate(&profile), TaskComplexity::Moderate);
    }

    #[test]
    fn test_complexity_complex() {
        let profile = TaskProfile {
            description: "Large refactor".to_string(),
            file_count: 20,
            total_lines: 3000,
            languages: vec!["rust".to_string()],
            has_tests: false,
            estimated_tokens: 10000,
            complexity: TaskComplexity::Complex,
        };
        assert_eq!(ComplexityEstimator::estimate(&profile), TaskComplexity::Complex);
    }

    #[test]
    fn test_complexity_expert() {
        let profile = TaskProfile {
            description: "Massive rewrite".to_string(),
            file_count: 50,
            total_lines: 10000,
            languages: vec!["rust".to_string()],
            has_tests: false,
            estimated_tokens: 50000,
            complexity: TaskComplexity::Expert,
        };
        assert_eq!(ComplexityEstimator::estimate(&profile), TaskComplexity::Expert);
    }

    #[test]
    fn test_complexity_many_languages_bumps_up() {
        let profile = TaskProfile {
            description: "Polyglot".to_string(),
            file_count: 5,
            total_lines: 100,
            languages: vec![
                "rust".to_string(),
                "ts".to_string(),
                "py".to_string(),
                "go".to_string(),
            ],
            has_tests: false,
            estimated_tokens: 1000,
            complexity: TaskComplexity::Simple,
        };
        // 100 lines -> Simple (level 1), 4 langs bumps to Moderate (level 2)
        assert_eq!(ComplexityEstimator::estimate(&profile), TaskComplexity::Moderate);
    }

    #[test]
    fn test_complexity_tests_reduce() {
        let profile = TaskProfile {
            description: "With tests".to_string(),
            file_count: 5,
            total_lines: 500,
            languages: vec!["rust".to_string()],
            has_tests: true,
            estimated_tokens: 2000,
            complexity: TaskComplexity::Moderate,
        };
        // 500 lines -> Moderate (level 2), tests reduce to Simple (level 1)
        assert_eq!(ComplexityEstimator::estimate(&profile), TaskComplexity::Simple);
    }

    #[test]
    fn test_complexity_tests_dont_go_below_trivial() {
        let profile = TaskProfile {
            description: "Tiny with tests".to_string(),
            file_count: 1,
            total_lines: 10,
            languages: vec!["rust".to_string()],
            has_tests: true,
            estimated_tokens: 50,
            complexity: TaskComplexity::Trivial,
        };
        assert_eq!(ComplexityEstimator::estimate(&profile), TaskComplexity::Trivial);
    }

    // --- Cost estimation tests ---

    #[test]
    fn test_estimate_cost() {
        let model = make_cheap_model();
        let cost = CostRouter::estimate_cost(&model, 1000);
        // input: 1000/1000 * 0.001 = 0.001
        // output: 400/1000 * 0.002 = 0.0008
        let expected = 0.001 + 0.0008;
        assert!((cost - expected).abs() < 1e-10);
    }

    #[test]
    fn test_estimate_cost_zero_tokens() {
        let model = make_cheap_model();
        let cost = CostRouter::estimate_cost(&model, 0);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_estimate_cost_quality_model() {
        let model = make_quality_model();
        let cost = CostRouter::estimate_cost(&model, 10000);
        // input: 10 * 0.03 = 0.30
        // output: 4 * 0.06 = 0.24
        let expected = 0.30 + 0.24;
        assert!((cost - expected).abs() < 1e-10);
    }

    // --- Routing strategy tests ---

    #[test]
    fn test_route_cheapest_first() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        router.add_model(make_quality_model());
        router.add_model(make_balanced_model());

        let decision = router.route_task("t1", &make_simple_profile()).unwrap();
        assert_eq!(decision.selected_model, "cheap-1");
    }

    #[test]
    fn test_route_quality_first() {
        let mut router = CostRouter::new(RoutingStrategy::QualityFirst);
        router.add_model(make_cheap_model());
        router.add_model(make_quality_model());
        router.add_model(make_balanced_model());

        let decision = router.route_task("t1", &make_simple_profile()).unwrap();
        assert_eq!(decision.selected_model, "quality-1");
    }

    #[test]
    fn test_route_balanced() {
        let mut router = CostRouter::new(RoutingStrategy::Balanced);
        router.add_model(make_cheap_model());
        router.add_model(make_quality_model());
        router.add_model(make_balanced_model());

        let decision = router.route_task("t1", &make_simple_profile()).unwrap();
        // Balanced picks best quality/cost ratio
        // cheap: 60 / cost, balanced: 80 / cost, quality: 95 / cost
        // cheap cost for 500 tokens: 0.0005 + 0.0004 = 0.0009 -> ratio 66666
        // balanced cost: 0.0025 + 0.002 = 0.0045 -> ratio 17777
        // quality cost: 0.015 + 0.012 = 0.027 -> ratio 3518
        // Cheap wins on ratio
        assert_eq!(decision.selected_model, "cheap-1");
    }

    #[test]
    fn test_route_budget_constrained() {
        let mut router = CostRouter::new(RoutingStrategy::BudgetConstrained(0.01));
        router.add_model(make_cheap_model());
        router.add_model(make_quality_model());
        router.add_model(make_balanced_model());

        let decision = router.route_task("t1", &make_simple_profile()).unwrap();
        // Within $0.01 budget, pick highest quality
        // cheap cost ~0.0009, balanced ~0.0045, quality ~0.027 (exceeds)
        // So balanced wins (highest quality among affordable)
        assert_eq!(decision.selected_model, "balanced-1");
    }

    #[test]
    fn test_route_budget_constrained_too_tight() {
        let mut router = CostRouter::new(RoutingStrategy::BudgetConstrained(0.00001));
        router.add_model(make_quality_model());

        let result = router.route_task("t1", &make_simple_profile());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No model fits within"));
    }

    // --- No models edge case ---

    #[test]
    fn test_route_no_models() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        let result = router.route_task("t1", &make_simple_profile());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No models registered"));
    }

    // --- Token limit edge case ---

    #[test]
    fn test_route_tokens_exceed_context() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        let mut model = make_cheap_model();
        model.max_context_tokens = 100;
        router.add_model(model);

        let profile = TaskProfile {
            description: "Too big".to_string(),
            file_count: 1,
            total_lines: 30,
            languages: vec!["rust".to_string()],
            has_tests: false,
            estimated_tokens: 500,
            complexity: TaskComplexity::Trivial,
        };
        let result = router.route_task("t1", &profile);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("token count"));
    }

    // --- Budget enforcement tests ---

    #[test]
    fn test_budget_hard_limit_blocks() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_quality_model());
        router.budget = Some(Budget {
            total: 0.001,
            spent: 0.0009,
            period: BudgetPeriod::Daily,
            hard_limit: true,
            alert_threshold_percent: 80.0,
            company_id: None,
        });

        let result = router.route_task("t1", &make_simple_profile());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Budget exceeded"));
    }

    #[test]
    fn test_budget_soft_limit_allows() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        router.budget = Some(Budget {
            total: 1.0,
            spent: 0.95,
            period: BudgetPeriod::Monthly,
            hard_limit: false,
            alert_threshold_percent: 80.0,
            company_id: None,
        });

        let result = router.route_task("t1", &make_simple_profile());
        assert!(result.is_ok());
    }

    #[test]
    fn test_remaining_budget() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.budget = Some(Budget {
            total: 10.0,
            spent: 3.5,
            period: BudgetPeriod::Monthly,
            hard_limit: true,
            alert_threshold_percent: 80.0,
            company_id: None,
        });
        assert!((router.remaining_budget() - 6.5).abs() < 1e-10);
    }

    #[test]
    fn test_remaining_budget_no_budget() {
        let router = CostRouter::new(RoutingStrategy::CheapestFirst);
        assert!(router.remaining_budget().is_infinite());
    }

    #[test]
    fn test_budget_updates_on_route() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        router.budget = Some(Budget {
            total: 10.0,
            spent: 0.0,
            period: BudgetPeriod::Monthly,
            hard_limit: false,
            alert_threshold_percent: 80.0,
            company_id: None,
        });

        let decision = router.route_task("t1", &make_simple_profile()).unwrap();
        assert!(router.budget.as_ref().unwrap().spent > 0.0);
        assert!((router.budget.as_ref().unwrap().spent - decision.estimated_cost).abs() < 1e-10);
    }

    // --- Single model tests ---

    #[test]
    fn test_route_single_model_cheapest() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_quality_model());

        let decision = router.route_task("t1", &make_simple_profile()).unwrap();
        assert_eq!(decision.selected_model, "quality-1");
        assert!(decision.alternatives.is_empty());
    }

    #[test]
    fn test_route_single_model_quality() {
        let mut router = CostRouter::new(RoutingStrategy::QualityFirst);
        router.add_model(make_cheap_model());

        let decision = router.route_task("t1", &make_simple_profile()).unwrap();
        assert_eq!(decision.selected_model, "cheap-1");
    }

    // --- Equal costs test ---

    #[test]
    fn test_route_equal_costs_cheapest() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        let mut m1 = make_cheap_model();
        let mut m2 = make_cheap_model();
        m2.id = "cheap-2".to_string();
        m2.name = "Cheap Model 2".to_string();
        m1.quality_score = 70.0;
        m2.quality_score = 70.0;
        router.add_model(m1);
        router.add_model(m2);

        let result = router.route_task("t1", &make_simple_profile());
        assert!(result.is_ok());
    }

    // --- Feedback loop tests ---

    #[test]
    fn test_record_feedback() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        let _ = router.route_task("t1", &make_simple_profile()).unwrap();

        router.record_feedback(RoutingFeedback {
            decision_id: "t1".to_string(),
            actual_cost: 0.001,
            success: true,
            quality_rating: Some(85.0),
            timestamp: 1000,
        });

        assert_eq!(router.feedback_log.len(), 1);
        let usage = router.metrics.by_model.get("cheap-1").unwrap();
        assert_eq!(usage.success_count, 1);
    }

    #[test]
    fn test_record_feedback_failure() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        let _ = router.route_task("t1", &make_simple_profile()).unwrap();

        router.record_feedback(RoutingFeedback {
            decision_id: "t1".to_string(),
            actual_cost: 0.001,
            success: false,
            quality_rating: Some(20.0),
            timestamp: 1000,
        });

        let usage = router.metrics.by_model.get("cheap-1").unwrap();
        assert_eq!(usage.success_count, 0);
    }

    // --- Quality score adjustment tests ---

    #[test]
    fn test_adjust_quality_scores() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model()); // quality 60.0
        let _ = router.route_task("t1", &make_simple_profile()).unwrap();
        let _ = router.route_task("t2", &make_simple_profile()).unwrap();

        router.record_feedback(RoutingFeedback {
            decision_id: "t1".to_string(),
            actual_cost: 0.001,
            success: true,
            quality_rating: Some(90.0),
            timestamp: 1000,
        });
        router.record_feedback(RoutingFeedback {
            decision_id: "t2".to_string(),
            actual_cost: 0.001,
            success: true,
            quality_rating: Some(80.0),
            timestamp: 1001,
        });

        router.adjust_quality_scores();

        // Original 60.0 * 0.3 + avg(90,80)=85 * 0.7 = 18 + 59.5 = 77.5
        let model = &router.models[0];
        assert!((model.quality_score - 77.5).abs() < 1e-10);
    }

    #[test]
    fn test_adjust_quality_no_feedback() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        router.adjust_quality_scores();
        assert!((router.models[0].quality_score - 60.0).abs() < 1e-10);
    }

    // --- Metrics tests ---

    #[test]
    fn test_metrics_update() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());

        let d1 = router.route_task("t1", &make_simple_profile()).unwrap();
        assert_eq!(router.metrics.total_routed, 1);
        assert!((router.metrics.total_cost - d1.estimated_cost).abs() < 1e-10);

        let d2 = router.route_task("t2", &make_simple_profile()).unwrap();
        assert_eq!(router.metrics.total_routed, 2);
        assert!(
            (router.metrics.avg_cost - (d1.estimated_cost + d2.estimated_cost) / 2.0).abs() < 1e-10
        );
    }

    #[test]
    fn test_metrics_by_strategy() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        let _ = router.route_task("t1", &make_simple_profile()).unwrap();
        let _ = router.route_task("t2", &make_simple_profile()).unwrap();

        assert_eq!(
            *router.metrics.by_strategy.get("CheapestFirst").unwrap(),
            2
        );
    }

    #[test]
    fn test_metrics_cheapest_and_most_expensive() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());

        let d = router.route_task("t1", &make_simple_profile()).unwrap();
        assert!((router.metrics.cheapest_route - d.estimated_cost).abs() < 1e-10);
        assert!((router.metrics.most_expensive_route - d.estimated_cost).abs() < 1e-10);
    }

    // --- Alternatives test ---

    #[test]
    fn test_decision_has_alternatives() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        router.add_model(make_quality_model());

        let decision = router.route_task("t1", &make_simple_profile()).unwrap();
        assert_eq!(decision.alternatives.len(), 1);
        assert_eq!(decision.alternatives[0].model_id, "quality-1");
    }

    // --- A/B router tests ---

    #[test]
    fn test_ab_create_experiment() {
        let mut ab = AbRouter::new();
        ab.create_experiment("exp1", "model-a", "model-b").unwrap();
        assert!(ab.experiments.contains_key("exp1"));
        assert_eq!(ab.active_experiment, Some("exp1".to_string()));
    }

    #[test]
    fn test_ab_create_duplicate_experiment() {
        let mut ab = AbRouter::new();
        ab.create_experiment("exp1", "model-a", "model-b").unwrap();
        let result = ab.create_experiment("exp1", "model-c", "model-d");
        assert!(result.is_err());
    }

    #[test]
    fn test_ab_route_alternates() {
        let mut ab = AbRouter::new();
        ab.create_experiment("exp1", "model-a", "model-b").unwrap();

        // First call -> model_a (0 total results, even -> A)
        assert_eq!(ab.route_ab().unwrap(), "model-a");

        // Record one result for A
        ab.record_result(
            "model-a",
            AbResult {
                cost: 0.01,
                success: true,
                quality: 80.0,
            },
        )
        .unwrap();

        // Now 1 total result (odd) -> B
        assert_eq!(ab.route_ab().unwrap(), "model-b");
    }

    #[test]
    fn test_ab_route_no_active_experiment() {
        let ab = AbRouter::new();
        let result = ab.route_ab();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No active experiment"));
    }

    #[test]
    fn test_ab_record_wrong_model() {
        let mut ab = AbRouter::new();
        ab.create_experiment("exp1", "model-a", "model-b").unwrap();
        let result = ab.record_result(
            "model-c",
            AbResult {
                cost: 0.01,
                success: true,
                quality: 80.0,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_ab_get_winner() {
        let mut ab = AbRouter::new();
        ab.create_experiment("exp1", "model-a", "model-b").unwrap();

        ab.record_result(
            "model-a",
            AbResult {
                cost: 0.01,
                success: true,
                quality: 90.0,
            },
        )
        .unwrap();
        ab.record_result(
            "model-b",
            AbResult {
                cost: 0.05,
                success: false,
                quality: 50.0,
            },
        )
        .unwrap();

        let (winner, score_a, score_b) = ab.get_winner("exp1").unwrap();
        assert_eq!(winner, "model-a");
        assert!(score_a > score_b);
    }

    #[test]
    fn test_ab_get_winner_no_results() {
        let mut ab = AbRouter::new();
        ab.create_experiment("exp1", "model-a", "model-b").unwrap();
        let result = ab.get_winner("exp1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No results"));
    }

    #[test]
    fn test_ab_get_winner_nonexistent() {
        let ab = AbRouter::new();
        let result = ab.get_winner("nope");
        assert!(result.is_err());
    }

    // --- Recommendations tests ---

    #[test]
    fn test_recommendations_no_models() {
        let router = CostRouter::new(RoutingStrategy::CheapestFirst);
        let recs = router.get_recommendations();
        assert_eq!(recs.len(), 1);
        assert!(recs[0].contains("Add at least one model"));
    }

    #[test]
    fn test_recommendations_no_routes() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        let recs = router.get_recommendations();
        assert!(recs[0].contains("No tasks routed"));
    }

    // --- Complex profile routing ---

    #[test]
    fn test_route_complex_profile() {
        let mut router = CostRouter::new(RoutingStrategy::QualityFirst);
        router.add_model(make_cheap_model());
        router.add_model(make_quality_model());
        router.add_model(make_balanced_model());

        let decision = router.route_task("t1", &make_complex_profile()).unwrap();
        assert_eq!(decision.selected_model, "quality-1");
        assert!(decision.estimated_cost > 0.0);
    }

    // --- Decision log test ---

    #[test]
    fn test_decision_log() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        let _ = router.route_task("t1", &make_simple_profile()).unwrap();
        let _ = router.route_task("t2", &make_simple_profile()).unwrap();
        assert_eq!(router.decision_log.len(), 2);
        assert_eq!(router.decision_log[0].task_id, "t1");
        assert_eq!(router.decision_log[1].task_id, "t2");
    }

    // --- Budget default test ---

    #[test]
    fn test_budget_default() {
        let budget = Budget::default();
        assert_eq!(budget.total, 0.0);
        assert_eq!(budget.spent, 0.0);
        assert_eq!(budget.period, BudgetPeriod::Monthly);
        assert!(!budget.hard_limit);
        assert!((budget.alert_threshold_percent - 80.0).abs() < 1e-10);
    }

    // --- Estimate complexity via CostRouter ---

    #[test]
    fn test_cost_router_estimate_complexity() {
        let profile = make_simple_profile();
        let complexity = CostRouter::estimate_complexity(&profile);
        assert_eq!(complexity, TaskComplexity::Trivial);
    }

    // --- Check budget standalone ---

    #[test]
    fn test_check_budget_no_budget() {
        let router = CostRouter::new(RoutingStrategy::CheapestFirst);
        assert!(router.check_budget(100.0).is_ok());
    }

    #[test]
    fn test_check_budget_hard_limit_ok() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.budget = Some(Budget {
            total: 10.0,
            spent: 5.0,
            period: BudgetPeriod::Daily,
            hard_limit: true,
            alert_threshold_percent: 80.0,
            company_id: None,
        });
        assert!(router.check_budget(4.0).is_ok());
    }

    #[test]
    fn test_check_budget_hard_limit_exceeded() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.budget = Some(Budget {
            total: 10.0,
            spent: 9.0,
            period: BudgetPeriod::Daily,
            hard_limit: true,
            alert_threshold_percent: 80.0,
            company_id: None,
        });
        assert!(router.check_budget(2.0).is_err());
    }

    // --- Model by_model usage ---

    #[test]
    fn test_model_usage_tracked() {
        let mut router = CostRouter::new(RoutingStrategy::CheapestFirst);
        router.add_model(make_cheap_model());
        let _ = router.route_task("t1", &make_simple_profile()).unwrap();
        let _ = router.route_task("t2", &make_simple_profile()).unwrap();

        let usage = router.metrics.by_model.get("cheap-1").unwrap();
        assert_eq!(usage.count, 2);
        assert!(usage.total_cost > 0.0);
    }
}
