#![allow(dead_code)]
//! Pre-execution cost estimator — estimate token count and provider cost
//! before running an agent task.
//!
//! Matches Devin 2.0's pre-execution cost estimation feature.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Provider pricing
// ---------------------------------------------------------------------------

/// Price per 1,000 tokens (input and output) for a provider model.
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub provider: String,
    pub model: String,
    /// Cost per 1,000 input tokens in USD.
    pub input_per_1k: f64,
    /// Cost per 1,000 output tokens in USD.
    pub output_per_1k: f64,
}

impl ModelPricing {
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        input_per_1k: f64,
        output_per_1k: f64,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            input_per_1k,
            output_per_1k,
        }
    }

    /// Compute the cost for the given token counts.
    pub fn cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        (input_tokens as f64 / 1000.0) * self.input_per_1k
            + (output_tokens as f64 / 1000.0) * self.output_per_1k
    }
}

// ---------------------------------------------------------------------------
// Built-in pricing catalogue (April 2026)
// ---------------------------------------------------------------------------

/// Provides pricing for known models. Returns `None` for unknown models.
pub struct PricingCatalogue {
    entries: HashMap<String, ModelPricing>,
}

impl PricingCatalogue {
    pub fn default_catalogue() -> Self {
        let mut c = Self {
            entries: HashMap::new(),
        };
        // Anthropic
        c.add(ModelPricing::new("anthropic", "claude-opus-4-6", 0.015, 0.075));
        c.add(ModelPricing::new("anthropic", "claude-sonnet-4-6", 0.003, 0.015));
        c.add(ModelPricing::new("anthropic", "claude-haiku-4-5-20251001", 0.00025, 0.00125));
        // OpenAI
        c.add(ModelPricing::new("openai", "gpt-4o", 0.005, 0.015));
        c.add(ModelPricing::new("openai", "gpt-4o-mini", 0.00015, 0.0006));
        c.add(ModelPricing::new("openai", "o3", 0.01, 0.04));
        // Google
        c.add(ModelPricing::new("google", "gemini-2.0-flash", 0.0001, 0.0004));
        c.add(ModelPricing::new("google", "gemini-2.0-pro", 0.002, 0.008));
        // Groq
        c.add(ModelPricing::new("groq", "llama-3.3-70b-versatile", 0.00059, 0.00079));
        // Mistral
        c.add(ModelPricing::new("mistral", "mistral-large-latest", 0.003, 0.009));
        // Ollama (local — zero cost)
        c.add(ModelPricing::new("ollama", "llama3", 0.0, 0.0));
        c.add(ModelPricing::new("ollama", "deepseek-coder", 0.0, 0.0));
        c
    }

    pub fn add(&mut self, pricing: ModelPricing) {
        let key = format!("{}/{}", pricing.provider, pricing.model);
        self.entries.insert(key, pricing);
    }

    pub fn get(&self, provider: &str, model: &str) -> Option<&ModelPricing> {
        let key = format!("{provider}/{model}");
        self.entries.get(&key)
    }

    pub fn all_models(&self) -> Vec<&ModelPricing> {
        let mut v: Vec<_> = self.entries.values().collect();
        v.sort_by(|a, b| {
            a.provider.cmp(&b.provider).then(a.model.cmp(&b.model))
        });
        v
    }
}

// ---------------------------------------------------------------------------
// Token counter (heuristic)
// ---------------------------------------------------------------------------

/// Heuristic token counter (no tiktoken required).
/// Uses the ~4 characters per token approximation common for English prose.
pub struct TokenCounter;

impl TokenCounter {
    /// Estimate the number of tokens in the given text.
    pub fn count(text: &str) -> usize {
        // Approximate: 4 chars ≈ 1 token for English; code is slightly more.
        let char_count = text.chars().count();
        (char_count as f64 / 3.8).ceil() as usize
    }

    /// Count tokens in multiple strings combined.
    pub fn count_all(texts: &[&str]) -> usize {
        texts.iter().map(|t| Self::count(t)).sum()
    }
}

// ---------------------------------------------------------------------------
// Cost estimate
// ---------------------------------------------------------------------------

/// A cost estimate for a planned agent run.
#[derive(Debug, Clone)]
pub struct CostEstimate {
    pub provider: String,
    pub model: String,
    /// Estimated input tokens (context + system prompt).
    pub estimated_input_tokens: usize,
    /// Estimated output tokens (response + tool calls).
    pub estimated_output_tokens: usize,
    /// Estimated total cost in USD.
    pub estimated_cost_usd: f64,
    /// Confidence level of the estimate.
    pub confidence: EstimateConfidence,
    /// Breakdown by component.
    pub breakdown: HashMap<String, usize>,
}

impl CostEstimate {
    pub fn total_tokens(&self) -> usize {
        self.estimated_input_tokens + self.estimated_output_tokens
    }

    pub fn format_cost(&self) -> String {
        if self.estimated_cost_usd < 0.001 {
            format!("${:.6}", self.estimated_cost_usd)
        } else if self.estimated_cost_usd < 0.01 {
            format!("${:.4}", self.estimated_cost_usd)
        } else {
            format!("${:.3}", self.estimated_cost_usd)
        }
    }
}

impl std::fmt::Display for CostEstimate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}: ~{} input + ~{} output tokens ≈ {} ({:?} confidence)",
            self.provider,
            self.model,
            self.estimated_input_tokens,
            self.estimated_output_tokens,
            self.format_cost(),
            self.confidence
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstimateConfidence {
    /// No tool calls expected; simple Q&A.
    High,
    /// Some tool use expected; output length uncertain.
    Medium,
    /// Complex multi-step task; large output variance.
    Low,
}

// ---------------------------------------------------------------------------
// Estimator builder
// ---------------------------------------------------------------------------

/// Inputs to the cost estimator.
#[derive(Debug, Default)]
pub struct EstimatorInput {
    pub system_prompt: String,
    pub conversation_history: Vec<String>,
    pub user_message: String,
    /// Expected number of tool call round-trips.
    pub expected_tool_rounds: usize,
    /// Average tokens per tool call result.
    pub avg_tool_result_tokens: usize,
    /// Provider/model to estimate for.
    pub provider: String,
    pub model: String,
}

/// Estimates cost for a planned agent task.
pub struct CostEstimator {
    catalogue: PricingCatalogue,
    /// Tokens-per-output multiplier (conservative: 1.5x).
    output_multiplier: f64,
}

impl CostEstimator {
    pub fn new(catalogue: PricingCatalogue) -> Self {
        Self {
            catalogue,
            output_multiplier: 1.5,
        }
    }

    pub fn with_default_catalogue() -> Self {
        Self::new(PricingCatalogue::default_catalogue())
    }

    /// Estimate cost for the given inputs.
    pub fn estimate(&self, input: &EstimatorInput) -> CostEstimate {
        let system_tokens = TokenCounter::count(&input.system_prompt);
        let history_tokens: usize = input
            .conversation_history
            .iter()
            .map(|m| TokenCounter::count(m))
            .sum();
        let user_tokens = TokenCounter::count(&input.user_message);
        let tool_tokens = input.expected_tool_rounds * input.avg_tool_result_tokens;

        let input_tokens = system_tokens + history_tokens + user_tokens + tool_tokens;
        // Output is harder to predict; use multiplier on the user message as a proxy.
        let output_tokens =
            ((user_tokens as f64 * self.output_multiplier) as usize).max(100) +
            input.expected_tool_rounds * 200;

        let confidence = match input.expected_tool_rounds {
            0 => EstimateConfidence::High,
            1..=3 => EstimateConfidence::Medium,
            _ => EstimateConfidence::Low,
        };

        let cost = self
            .catalogue
            .get(&input.provider, &input.model)
            .map(|p| p.cost(input_tokens, output_tokens))
            .unwrap_or(0.0);

        let mut breakdown = HashMap::new();
        breakdown.insert("system_prompt".into(), system_tokens);
        breakdown.insert("history".into(), history_tokens);
        breakdown.insert("user_message".into(), user_tokens);
        breakdown.insert("tool_results".into(), tool_tokens);

        CostEstimate {
            provider: input.provider.clone(),
            model: input.model.clone(),
            estimated_input_tokens: input_tokens,
            estimated_output_tokens: output_tokens,
            estimated_cost_usd: cost,
            confidence,
            breakdown,
        }
    }

    /// Compare cost across multiple provider/model combinations.
    pub fn compare(
        &self,
        base_input: &EstimatorInput,
        models: &[(&str, &str)],
    ) -> Vec<CostEstimate> {
        models
            .iter()
            .map(|(provider, model)| {
                let i = EstimatorInput {
                    system_prompt: base_input.system_prompt.clone(),
                    conversation_history: base_input.conversation_history.clone(),
                    user_message: base_input.user_message.clone(),
                    expected_tool_rounds: base_input.expected_tool_rounds,
                    avg_tool_result_tokens: base_input.avg_tool_result_tokens,
                    provider: provider.to_string(),
                    model: model.to_string(),
                };
                self.estimate(&i)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn estimator() -> CostEstimator {
        CostEstimator::with_default_catalogue()
    }

    fn simple_input(provider: &str, model: &str) -> EstimatorInput {
        EstimatorInput {
            system_prompt: "You are a helpful assistant.".into(),
            conversation_history: vec![],
            user_message: "Please refactor this function to use iterators.".into(),
            expected_tool_rounds: 0,
            avg_tool_result_tokens: 0,
            provider: provider.into(),
            model: model.into(),
        }
    }

    #[test]
    fn test_estimate_returns_positive_tokens() {
        let est = estimator();
        let input = simple_input("anthropic", "claude-sonnet-4-6");
        let result = est.estimate(&input);
        assert!(result.estimated_input_tokens > 0);
        assert!(result.estimated_output_tokens > 0);
    }

    #[test]
    fn test_estimate_cost_for_known_model() {
        let est = estimator();
        let input = simple_input("anthropic", "claude-opus-4-6");
        let result = est.estimate(&input);
        assert!(result.estimated_cost_usd > 0.0);
    }

    #[test]
    fn test_unknown_model_zero_cost() {
        let est = estimator();
        let input = simple_input("unknown_provider", "mystery-model");
        let result = est.estimate(&input);
        assert_eq!(result.estimated_cost_usd, 0.0);
    }

    #[test]
    fn test_local_model_zero_cost() {
        let est = estimator();
        let input = simple_input("ollama", "llama3");
        let result = est.estimate(&input);
        assert_eq!(result.estimated_cost_usd, 0.0);
    }

    #[test]
    fn test_confidence_high_no_tools() {
        let est = estimator();
        let input = simple_input("anthropic", "claude-sonnet-4-6");
        let result = est.estimate(&input);
        assert_eq!(result.confidence, EstimateConfidence::High);
    }

    #[test]
    fn test_confidence_low_many_tools() {
        let est = estimator();
        let mut input = simple_input("anthropic", "claude-sonnet-4-6");
        input.expected_tool_rounds = 10;
        input.avg_tool_result_tokens = 500;
        let result = est.estimate(&input);
        assert_eq!(result.confidence, EstimateConfidence::Low);
    }

    #[test]
    fn test_token_counter_non_zero() {
        let count = TokenCounter::count("Hello, world! This is a test sentence.");
        assert!(count > 5);
    }

    #[test]
    fn test_token_counter_empty() {
        assert_eq!(TokenCounter::count(""), 0);
    }

    #[test]
    fn test_token_counter_all() {
        let total = TokenCounter::count_all(&["hello world", "foo bar baz"]);
        assert!(total > 5);
    }

    #[test]
    fn test_compare_returns_multiple_estimates() {
        let est = estimator();
        let input = simple_input("anthropic", "claude-sonnet-4-6");
        let results = est.compare(
            &input,
            &[
                ("anthropic", "claude-opus-4-6"),
                ("openai", "gpt-4o"),
                ("ollama", "llama3"),
            ],
        );
        assert_eq!(results.len(), 3);
        // Ollama should be cheapest (free)
        let ollama = results.iter().find(|r| r.provider == "ollama").unwrap();
        assert_eq!(ollama.estimated_cost_usd, 0.0);
    }

    #[test]
    fn test_format_cost_small() {
        let est = CostEstimate {
            provider: "anthropic".into(),
            model: "claude-haiku-4-5-20251001".into(),
            estimated_input_tokens: 100,
            estimated_output_tokens: 50,
            estimated_cost_usd: 0.0000125,
            confidence: EstimateConfidence::High,
            breakdown: HashMap::new(),
        };
        let formatted = est.format_cost();
        assert!(formatted.starts_with('$'));
    }

    #[test]
    fn test_total_tokens() {
        let est = CostEstimate {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            estimated_input_tokens: 1000,
            estimated_output_tokens: 500,
            estimated_cost_usd: 0.01,
            confidence: EstimateConfidence::Medium,
            breakdown: HashMap::new(),
        };
        assert_eq!(est.total_tokens(), 1500);
    }

    #[test]
    fn test_breakdown_contains_components() {
        let est = estimator();
        let input = simple_input("anthropic", "claude-sonnet-4-6");
        let result = est.estimate(&input);
        assert!(result.breakdown.contains_key("system_prompt"));
        assert!(result.breakdown.contains_key("user_message"));
    }

    #[test]
    fn test_catalogue_all_models() {
        let cat = PricingCatalogue::default_catalogue();
        assert!(cat.all_models().len() >= 8);
    }

    #[test]
    fn test_pricing_cost_calculation() {
        let p = ModelPricing::new("openai", "gpt-4o", 5.0, 15.0);
        // 1000 input @ $5.00/1K = $5.00; 1000 output @ $15.00/1K = $15.00 → $20.00
        let cost = p.cost(1000, 1000);
        assert!((cost - 20.0).abs() < 1e-9);
    }
}
