//! RLCEF (Reinforcement Learning from Code Execution Feedback) training loop.
//!
//! Gap 18 — Records code execution outcomes, computes multi-signal reward scores,
//! detects recurring mistake patterns, and suggests strategy adjustments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Outcome of a single code execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub prompt: String,
    pub code: String,
    pub test_passed: bool,
    pub error_message: Option<String>,
    pub runtime_ms: u64,
    pub memory_kb: u64,
    pub language: String,
}

/// Multi-signal reward derived from an execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RewardSignal {
    pub test_score: f64,
    pub execution_time_score: f64,
    pub memory_score: f64,
    pub total: f64,
}

/// Category of a recurring mistake.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MistakeCategory {
    SyntaxError,
    TypeMismatch,
    LogicError,
    RuntimePanic,
    InfiniteLoop,
    ResourceLeak,
    ApiMisuse,
    TestFailure,
}

impl MistakeCategory {
    pub fn name(&self) -> &str {
        match self {
            Self::SyntaxError => "syntax_error",
            Self::TypeMismatch => "type_mismatch",
            Self::LogicError => "logic_error",
            Self::RuntimePanic => "runtime_panic",
            Self::InfiniteLoop => "infinite_loop",
            Self::ResourceLeak => "resource_leak",
            Self::ApiMisuse => "api_misuse",
            Self::TestFailure => "test_failure",
        }
    }

    pub fn all() -> Vec<MistakeCategory> {
        vec![
            Self::SyntaxError, Self::TypeMismatch, Self::LogicError,
            Self::RuntimePanic, Self::InfiniteLoop, Self::ResourceLeak,
            Self::ApiMisuse, Self::TestFailure,
        ]
    }
}

/// A detected recurring mistake pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MistakePattern {
    pub id: String,
    pub category: MistakeCategory,
    pub language: String,
    pub description: String,
    pub frequency: u32,
    pub example_code: Option<String>,
}

/// A suggested adjustment to model strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyAdjustment {
    pub model_id: String,
    pub parameter: String,
    pub old_value: String,
    pub new_value: String,
    pub reason: String,
}

/// Configuration for the RLCEF engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RlcefConfig {
    pub model_id: String,
    pub time_budget_ms: u64,
    pub memory_budget_kb: u64,
    pub test_weight: f64,
    pub time_weight: f64,
    pub memory_weight: f64,
    pub pattern_threshold: u32,
}

impl Default for RlcefConfig {
    fn default() -> Self {
        Self {
            model_id: "default".to_string(),
            time_budget_ms: 5000,
            memory_budget_kb: 512_000,
            test_weight: 0.6,
            time_weight: 0.2,
            memory_weight: 0.2,
            pattern_threshold: 3,
        }
    }
}

/// Aggregate metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RlcefMetrics {
    pub total_outcomes: u64,
    pub pass_count: u64,
    pub fail_count: u64,
    pub avg_reward: f64,
    pub avg_runtime_ms: f64,
    pub avg_memory_kb: f64,
}

impl Default for RlcefMetrics {
    fn default() -> Self {
        Self {
            total_outcomes: 0,
            pass_count: 0,
            fail_count: 0,
            avg_reward: 0.0,
            avg_runtime_ms: 0.0,
            avg_memory_kb: 0.0,
        }
    }
}

/// Core RLCEF engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlcefEngine {
    pub outcomes: Vec<ExecutionOutcome>,
    pub mistakes: HashMap<String, MistakePattern>,
    pub strategies: Vec<StrategyAdjustment>,
    pub config: RlcefConfig,
    pub metrics: RlcefMetrics,
    rewards: Vec<RewardSignal>,
}

impl RlcefEngine {
    pub fn new(config: RlcefConfig) -> Self {
        Self {
            outcomes: Vec::new(),
            mistakes: HashMap::new(),
            strategies: Vec::new(),
            config,
            metrics: RlcefMetrics::default(),
            rewards: Vec::new(),
        }
    }

    /// Record an execution outcome.
    pub fn record_outcome(&mut self, outcome: ExecutionOutcome) {
        let reward = self.compute_reward(&outcome);
        self.rewards.push(reward);

        // Update metrics
        self.metrics.total_outcomes += 1;
        if outcome.test_passed {
            self.metrics.pass_count += 1;
        } else {
            self.metrics.fail_count += 1;
        }
        let n = self.metrics.total_outcomes as f64;
        self.metrics.avg_runtime_ms =
            self.metrics.avg_runtime_ms * ((n - 1.0) / n) + outcome.runtime_ms as f64 / n;
        self.metrics.avg_memory_kb =
            self.metrics.avg_memory_kb * ((n - 1.0) / n) + outcome.memory_kb as f64 / n;

        let latest_reward = self.rewards.last().expect("just pushed").total;
        self.metrics.avg_reward =
            self.metrics.avg_reward * ((n - 1.0) / n) + latest_reward / n;

        self.outcomes.push(outcome);
    }

    /// Compute a multi-signal reward for an outcome.
    pub fn compute_reward(&self, outcome: &ExecutionOutcome) -> RewardSignal {
        let test_score = if outcome.test_passed { 1.0 } else { 0.0 };

        let time_score = if self.config.time_budget_ms == 0 {
            1.0
        } else {
            let ratio = outcome.runtime_ms as f64 / self.config.time_budget_ms as f64;
            (1.0 - ratio).max(0.0).min(1.0)
        };

        let memory_score = if self.config.memory_budget_kb == 0 {
            1.0
        } else {
            let ratio = outcome.memory_kb as f64 / self.config.memory_budget_kb as f64;
            (1.0 - ratio).max(0.0).min(1.0)
        };

        let total = test_score * self.config.test_weight
            + time_score * self.config.time_weight
            + memory_score * self.config.memory_weight;

        RewardSignal {
            test_score,
            execution_time_score: time_score,
            memory_score,
            total,
        }
    }

    /// Detect recurring mistake patterns from failed outcomes.
    pub fn detect_mistake_patterns(&mut self) -> Vec<&MistakePattern> {
        // Classify errors
        let mut pattern_counts: HashMap<(MistakeCategory, String), u32> = HashMap::new();
        let mut pattern_examples: HashMap<(MistakeCategory, String), String> = HashMap::new();

        for outcome in &self.outcomes {
            if outcome.test_passed {
                continue;
            }
            let error = outcome.error_message.clone().unwrap_or_default().to_lowercase();
            let lang = outcome.language.clone();

            let category = if error.contains("syntax") || error.contains("parse") {
                MistakeCategory::SyntaxError
            } else if error.contains("type") || error.contains("cast") {
                MistakeCategory::TypeMismatch
            } else if error.contains("panic") || error.contains("unwrap") {
                MistakeCategory::RuntimePanic
            } else if error.contains("timeout") || error.contains("infinite") {
                MistakeCategory::InfiniteLoop
            } else if error.contains("leak") || error.contains("resource") {
                MistakeCategory::ResourceLeak
            } else if error.contains("api") || error.contains("undefined") {
                MistakeCategory::ApiMisuse
            } else if error.contains("assert") || error.contains("test") {
                MistakeCategory::TestFailure
            } else {
                MistakeCategory::LogicError
            };

            let key = (category.clone(), lang.clone());
            *pattern_counts.entry(key.clone()).or_insert(0) += 1;
            pattern_examples.entry(key).or_insert_with(|| outcome.code.clone());
        }

        // Convert to patterns
        for ((cat, lang), count) in &pattern_counts {
            if *count >= self.config.pattern_threshold {
                let id = format!("{}-{}", cat.name(), lang);
                let example = pattern_examples.get(&(cat.clone(), lang.clone())).cloned();
                self.mistakes.insert(id.clone(), MistakePattern {
                    id,
                    category: cat.clone(),
                    language: lang.clone(),
                    description: format!(
                        "{} in {} occurred {} times",
                        cat.name(), lang, count
                    ),
                    frequency: *count,
                    example_code: example,
                });
            }
        }

        self.mistakes.values().collect()
    }

    /// Suggest strategy adjustments based on detected patterns.
    pub fn suggest_adjustments(&mut self) -> Vec<StrategyAdjustment> {
        let mut adjustments = Vec::new();
        let model_id = self.config.model_id.clone();

        // High failure rate -> suggest temperature reduction
        if self.metrics.total_outcomes > 5 {
            let fail_rate = self.metrics.fail_count as f64 / self.metrics.total_outcomes as f64;
            if fail_rate > 0.5 {
                adjustments.push(StrategyAdjustment {
                    model_id: model_id.clone(),
                    parameter: "temperature".to_string(),
                    old_value: "0.7".to_string(),
                    new_value: "0.3".to_string(),
                    reason: format!("High failure rate ({:.0}%)", fail_rate * 100.0),
                });
            }
        }

        // Slow execution -> suggest optimization prompt
        if self.metrics.avg_runtime_ms > self.config.time_budget_ms as f64 * 0.8 {
            adjustments.push(StrategyAdjustment {
                model_id: model_id.clone(),
                parameter: "system_prompt_suffix".to_string(),
                old_value: "".to_string(),
                new_value: "Optimize for execution speed.".to_string(),
                reason: "Average runtime approaching budget limit".to_string(),
            });
        }

        // Pattern-based adjustments
        for pattern in self.mistakes.values() {
            if pattern.frequency >= self.config.pattern_threshold * 2 {
                adjustments.push(StrategyAdjustment {
                    model_id: model_id.clone(),
                    parameter: format!("avoid_{}", pattern.category.name()),
                    old_value: "false".to_string(),
                    new_value: "true".to_string(),
                    reason: format!(
                        "Recurring {} in {} ({} occurrences)",
                        pattern.category.name(), pattern.language, pattern.frequency
                    ),
                });
            }
        }

        self.strategies = adjustments.clone();
        adjustments
    }

    /// Export outcomes as training data (JSONL format).
    pub fn export_training_data(&self) -> String {
        self.outcomes.iter().enumerate()
            .map(|(i, o)| {
                let reward = if i < self.rewards.len() { self.rewards[i].total } else { 0.0 };
                format!(
                    r#"{{"prompt":"{}","code":"{}","passed":{},"reward":{:.4},"language":"{}"}}"#,
                    o.prompt.replace('"', "\\\""),
                    o.code.replace('"', "\\\"").replace('\n', "\\n"),
                    o.test_passed,
                    reward,
                    o.language,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get language-specific profile.
    pub fn get_language_profile(&self, language: &str) -> (u64, u64, f64) {
        let filtered: Vec<&ExecutionOutcome> = self.outcomes.iter()
            .filter(|o| o.language == language)
            .collect();
        let total = filtered.len() as u64;
        let passed = filtered.iter().filter(|o| o.test_passed).count() as u64;
        let pass_rate = if total == 0 { 0.0 } else { passed as f64 / total as f64 };
        (total, passed, pass_rate)
    }

    /// Get outcomes where tests passed (positive patterns).
    pub fn positive_patterns(&self) -> Vec<&ExecutionOutcome> {
        self.outcomes.iter().filter(|o| o.test_passed).collect()
    }

    /// Pass rate as percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.metrics.total_outcomes == 0 {
            0.0
        } else {
            (self.metrics.pass_count as f64 / self.metrics.total_outcomes as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eng() -> RlcefEngine {
        RlcefEngine::new(RlcefConfig::default())
    }

    fn pass_outcome(lang: &str) -> ExecutionOutcome {
        ExecutionOutcome {
            prompt: "write hello world".to_string(),
            code: "println!(\"hello\")".to_string(),
            test_passed: true,
            error_message: None,
            runtime_ms: 100,
            memory_kb: 10_000,
            language: lang.to_string(),
        }
    }

    fn fail_outcome(lang: &str, error: &str) -> ExecutionOutcome {
        ExecutionOutcome {
            prompt: "write code".to_string(),
            code: "bad code".to_string(),
            test_passed: false,
            error_message: Some(error.to_string()),
            runtime_ms: 200,
            memory_kb: 20_000,
            language: lang.to_string(),
        }
    }

    #[test]
    fn test_engine_new() {
        let e = eng();
        assert!(e.outcomes.is_empty());
        assert_eq!(e.metrics.total_outcomes, 0);
    }

    #[test]
    fn test_record_outcome_pass() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        assert_eq!(e.metrics.total_outcomes, 1);
        assert_eq!(e.metrics.pass_count, 1);
        assert_eq!(e.metrics.fail_count, 0);
    }

    #[test]
    fn test_record_outcome_fail() {
        let mut e = eng();
        e.record_outcome(fail_outcome("rust", "syntax error"));
        assert_eq!(e.metrics.fail_count, 1);
    }

    #[test]
    fn test_compute_reward_pass() {
        let e = eng();
        let r = e.compute_reward(&pass_outcome("rust"));
        assert_eq!(r.test_score, 1.0);
        assert!(r.total > 0.5);
    }

    #[test]
    fn test_compute_reward_fail() {
        let e = eng();
        let r = e.compute_reward(&fail_outcome("rust", "error"));
        assert_eq!(r.test_score, 0.0);
        assert!(r.total < 0.5);
    }

    #[test]
    fn test_compute_reward_fast_execution() {
        let e = eng();
        let mut o = pass_outcome("rust");
        o.runtime_ms = 10; // very fast
        let r = e.compute_reward(&o);
        assert!(r.execution_time_score > 0.9);
    }

    #[test]
    fn test_compute_reward_slow_execution() {
        let e = eng();
        let mut o = pass_outcome("rust");
        o.runtime_ms = 10_000; // over budget
        let r = e.compute_reward(&o);
        assert_eq!(r.execution_time_score, 0.0);
    }

    #[test]
    fn test_compute_reward_low_memory() {
        let e = eng();
        let mut o = pass_outcome("rust");
        o.memory_kb = 100;
        let r = e.compute_reward(&o);
        assert!(r.memory_score > 0.9);
    }

    #[test]
    fn test_compute_reward_high_memory() {
        let e = eng();
        let mut o = pass_outcome("rust");
        o.memory_kb = 1_000_000;
        let r = e.compute_reward(&o);
        assert_eq!(r.memory_score, 0.0);
    }

    #[test]
    fn test_avg_reward_updates() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        assert!(e.metrics.avg_reward > 0.0);
    }

    #[test]
    fn test_avg_runtime_updates() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        assert!(e.metrics.avg_runtime_ms > 0.0);
    }

    #[test]
    fn test_detect_mistake_patterns_none() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_detect_mistake_patterns_syntax() {
        let mut e = eng();
        for _ in 0..5 {
            e.record_outcome(fail_outcome("rust", "syntax error on line 5"));
        }
        let patterns = e.detect_mistake_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.category == MistakeCategory::SyntaxError));
    }

    #[test]
    fn test_detect_mistake_patterns_type() {
        let mut e = eng();
        for _ in 0..3 {
            e.record_outcome(fail_outcome("typescript", "type mismatch: expected string"));
        }
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.iter().any(|p| p.category == MistakeCategory::TypeMismatch));
    }

    #[test]
    fn test_detect_mistake_patterns_panic() {
        let mut e = eng();
        for _ in 0..3 {
            e.record_outcome(fail_outcome("rust", "thread panicked at unwrap()"));
        }
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.iter().any(|p| p.category == MistakeCategory::RuntimePanic));
    }

    #[test]
    fn test_detect_mistake_patterns_below_threshold() {
        let mut e = eng();
        e.record_outcome(fail_outcome("rust", "syntax error"));
        e.record_outcome(fail_outcome("rust", "syntax error"));
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.is_empty()); // only 2, threshold is 3
    }

    #[test]
    fn test_detect_mistake_patterns_infinite_loop() {
        let mut e = eng();
        for _ in 0..3 {
            e.record_outcome(fail_outcome("python", "timeout exceeded — possible infinite loop"));
        }
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.iter().any(|p| p.category == MistakeCategory::InfiniteLoop));
    }

    #[test]
    fn test_detect_mistake_patterns_api_misuse() {
        let mut e = eng();
        for _ in 0..3 {
            e.record_outcome(fail_outcome("js", "undefined is not a function"));
        }
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.iter().any(|p| p.category == MistakeCategory::ApiMisuse));
    }

    #[test]
    fn test_detect_mistake_patterns_test_failure() {
        let mut e = eng();
        for _ in 0..3 {
            e.record_outcome(fail_outcome("rust", "assertion failed: test expected 5"));
        }
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.iter().any(|p| p.category == MistakeCategory::TestFailure));
    }

    #[test]
    fn test_suggest_adjustments_empty() {
        let mut e = eng();
        let adj = e.suggest_adjustments();
        assert!(adj.is_empty());
    }

    #[test]
    fn test_suggest_adjustments_high_fail_rate() {
        let mut e = eng();
        for _ in 0..8 {
            e.record_outcome(fail_outcome("rust", "error"));
        }
        for _ in 0..2 {
            e.record_outcome(pass_outcome("rust"));
        }
        let adj = e.suggest_adjustments();
        assert!(adj.iter().any(|a| a.parameter == "temperature"));
    }

    #[test]
    fn test_suggest_adjustments_slow_runtime() {
        let mut e = eng();
        let mut cfg = RlcefConfig::default();
        cfg.time_budget_ms = 100;
        e.config = cfg;
        let mut o = pass_outcome("rust");
        o.runtime_ms = 90;
        for _ in 0..6 {
            e.record_outcome(o.clone());
        }
        let adj = e.suggest_adjustments();
        assert!(adj.iter().any(|a| a.parameter == "system_prompt_suffix"));
    }

    #[test]
    fn test_suggest_adjustments_recurring_pattern() {
        let mut e = eng();
        e.config.pattern_threshold = 2;
        for _ in 0..5 {
            e.record_outcome(fail_outcome("rust", "syntax error"));
        }
        e.detect_mistake_patterns();
        // frequency 5 >= 2*2=4 so should suggest
        let adj = e.suggest_adjustments();
        assert!(adj.iter().any(|a| a.parameter.contains("syntax_error")));
    }

    #[test]
    fn test_export_training_data() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        e.record_outcome(fail_outcome("rust", "err"));
        let data = e.export_training_data();
        let lines: Vec<&str> = data.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"passed\":true"));
        assert!(lines[1].contains("\"passed\":false"));
    }

    #[test]
    fn test_export_training_data_empty() {
        let e = eng();
        assert!(e.export_training_data().is_empty());
    }

    #[test]
    fn test_get_language_profile() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        e.record_outcome(pass_outcome("rust"));
        e.record_outcome(fail_outcome("rust", "err"));
        e.record_outcome(pass_outcome("python"));
        let (total, passed, rate) = e.get_language_profile("rust");
        assert_eq!(total, 3);
        assert_eq!(passed, 2);
        assert!((rate - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_get_language_profile_unknown() {
        let e = eng();
        let (total, passed, rate) = e.get_language_profile("haskell");
        assert_eq!(total, 0);
        assert_eq!(passed, 0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_positive_patterns() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        e.record_outcome(fail_outcome("rust", "err"));
        e.record_outcome(pass_outcome("rust"));
        assert_eq!(e.positive_patterns().len(), 2);
    }

    #[test]
    fn test_pass_rate() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        e.record_outcome(fail_outcome("rust", "err"));
        assert!((e.pass_rate() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_pass_rate_empty() {
        let e = eng();
        assert_eq!(e.pass_rate(), 0.0);
    }

    #[test]
    fn test_mistake_category_all() {
        assert_eq!(MistakeCategory::all().len(), 8);
    }

    #[test]
    fn test_mistake_category_name() {
        assert_eq!(MistakeCategory::SyntaxError.name(), "syntax_error");
        assert_eq!(MistakeCategory::InfiniteLoop.name(), "infinite_loop");
    }

    #[test]
    fn test_config_default() {
        let c = RlcefConfig::default();
        assert_eq!(c.test_weight, 0.6);
        assert_eq!(c.time_weight, 0.2);
        assert_eq!(c.memory_weight, 0.2);
    }

    #[test]
    fn test_reward_weights_sum() {
        let c = RlcefConfig::default();
        let sum = c.test_weight + c.time_weight + c.memory_weight;
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_metrics_default() {
        let m = RlcefMetrics::default();
        assert_eq!(m.total_outcomes, 0);
        assert_eq!(m.avg_reward, 0.0);
    }

    #[test]
    fn test_execution_outcome_serde() {
        let o = pass_outcome("rust");
        let json = serde_json::to_string(&o).unwrap();
        let de: ExecutionOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(o, de);
    }

    #[test]
    fn test_reward_signal_serde() {
        let r = RewardSignal {
            test_score: 1.0, execution_time_score: 0.8,
            memory_score: 0.9, total: 0.9,
        };
        let json = serde_json::to_string(&r).unwrap();
        let de: RewardSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(r, de);
    }

    #[test]
    fn test_strategy_adjustment_serde() {
        let a = StrategyAdjustment {
            model_id: "m".to_string(),
            parameter: "temp".to_string(),
            old_value: "0.7".to_string(),
            new_value: "0.3".to_string(),
            reason: "high fail rate".to_string(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let de: StrategyAdjustment = serde_json::from_str(&json).unwrap();
        assert_eq!(a, de);
    }

    #[test]
    fn test_multiple_languages_profiles() {
        let mut e = eng();
        e.record_outcome(pass_outcome("rust"));
        e.record_outcome(pass_outcome("python"));
        e.record_outcome(fail_outcome("python", "indent error"));
        let (rt, rp, _) = e.get_language_profile("rust");
        let (pt, pp, _) = e.get_language_profile("python");
        assert_eq!(rt, 1);
        assert_eq!(rp, 1);
        assert_eq!(pt, 2);
        assert_eq!(pp, 1);
    }

    #[test]
    fn test_detect_logic_error_default() {
        let mut e = eng();
        for _ in 0..3 {
            e.record_outcome(fail_outcome("go", "wrong output"));
        }
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.iter().any(|p| p.category == MistakeCategory::LogicError));
    }

    #[test]
    fn test_detect_resource_leak() {
        let mut e = eng();
        for _ in 0..3 {
            e.record_outcome(fail_outcome("c", "resource leak detected"));
        }
        let patterns = e.detect_mistake_patterns();
        assert!(patterns.iter().any(|p| p.category == MistakeCategory::ResourceLeak));
    }
}
