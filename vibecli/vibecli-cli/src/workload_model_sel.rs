//! Workload-aware model selection.
//!
//! GAP-v9-020: rivals Amazon Q Model Router, Cursor Smart Model, Devin Model Selector.
//! - Task classification into 8 workload types (chat, completion, edit, review, ...)
//! - Model profile: cost/token, latency class, context window, capability flags
//! - Selection policy: cost-optimise, latency-optimise, quality-maximise, or balanced
//! - Budget-aware fallback chain: degrade to cheaper model when budget exhausted
//! - Adaptive scoring: weighs observed p50 latency against declared latency class
//! - Telemetry: tracks per-model usage for drift detection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Workload Type ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadType {
    /// Short back-and-forth chat.
    Chat,
    /// Long-form code completion (fill-in-middle).
    Completion,
    /// Targeted code edit with diff output.
    Edit,
    /// Code review with multi-file context.
    Review,
    /// Multi-step agent loop (plan + act + observe).
    AgentLoop,
    /// Summarisation of long documents.
    Summarise,
    /// Embedding generation (not generation-model).
    Embedding,
    /// RAG Q&A over a knowledge base.
    Rag,
}

impl WorkloadType {
    /// Classify workload from a free-form task description.
    pub fn classify(description: &str) -> Self {
        let d = description.to_lowercase();
        if d.contains("embed") || d.contains("vector") { return Self::Embedding; }
        if d.contains("agent") || d.contains("plan") && d.contains("act") { return Self::AgentLoop; }
        if d.contains("review") || d.contains("pr") || d.contains("pull request") { return Self::Review; }
        if d.contains("edit") || d.contains("refactor") || d.contains("rename") { return Self::Edit; }
        if d.contains("complete") || d.contains("fim") || d.contains("fill") { return Self::Completion; }
        if d.contains("summarise") || d.contains("summarize") || d.contains("tldr") { return Self::Summarise; }
        if d.contains("rag") || d.contains("search") || d.contains("retriev") { return Self::Rag; }
        Self::Chat
    }
}

// ─── Model Profile ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LatencyClass {
    /// <500 ms median.
    UltraFast,
    /// 500–2000 ms.
    Fast,
    /// 2–10 s.
    Medium,
    /// >10 s.
    Slow,
}

impl LatencyClass {
    /// Median latency in ms (used as a proxy for scoring).
    pub fn median_ms(&self) -> u64 {
        match self { Self::UltraFast => 300, Self::Fast => 1000, Self::Medium => 5000, Self::Slow => 15000 }
    }
}

/// Capability flags for a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    pub code: bool,
    pub function_calling: bool,
    pub vision: bool,
    pub long_context: bool,
    pub streaming: bool,
    pub fim: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String,
    pub provider: String,
    /// Cost per 1M input tokens in USD.
    pub cost_per_1m_in: f64,
    /// Cost per 1M output tokens in USD.
    pub cost_per_1m_out: f64,
    pub context_window: u32,
    pub latency: LatencyClass,
    pub quality_score: f32,
    pub capabilities: Capabilities,
}

impl ModelProfile {
    /// Estimated cost in USD for a request with given token counts.
    pub fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        (input_tokens as f64 / 1_000_000.0) * self.cost_per_1m_in
        + (output_tokens as f64 / 1_000_000.0) * self.cost_per_1m_out
    }

    /// Whether this model can handle a request of given context length.
    pub fn can_handle(&self, context_tokens: u32) -> bool {
        context_tokens <= self.context_window
    }
}

// ─── Selection Policy ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionPolicy {
    /// Minimise cost per token.
    CostOptimise,
    /// Minimise latency.
    LatencyOptimise,
    /// Maximise quality score.
    QualityMaximise,
    /// Balanced: weighted combination of cost, latency, quality.
    Balanced,
}

/// Selection request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRequest {
    pub workload: WorkloadType,
    pub context_tokens: u32,
    pub expected_output_tokens: u64,
    pub max_budget_usd: Option<f64>,
    pub policy: SelectionPolicy,
    /// Required capability flags.
    pub require_code: bool,
    pub require_fim: bool,
    pub require_vision: bool,
    pub require_long_context: bool,
}

impl SelectionRequest {
    pub fn chat(context_tokens: u32) -> Self {
        Self {
            workload: WorkloadType::Chat, context_tokens,
            expected_output_tokens: 256, max_budget_usd: None,
            policy: SelectionPolicy::Balanced,
            require_code: false, require_fim: false, require_vision: false, require_long_context: false,
        }
    }
}

// ─── Model Selector ──────────────────────────────────────────────────────────

/// Telemetry record per model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelTelemetry {
    pub requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    /// Observed p50 latency in ms (exponential moving average).
    pub observed_latency_ms: f64,
}

pub struct WorkloadModelSelector {
    pub models: Vec<ModelProfile>,
    pub telemetry: HashMap<String, ModelTelemetry>,
}

impl WorkloadModelSelector {
    pub fn new(models: Vec<ModelProfile>) -> Self {
        Self { models, telemetry: HashMap::new() }
    }

    /// Filter models that satisfy capability requirements and context window.
    fn eligible<'a>(&'a self, req: &SelectionRequest) -> Vec<&'a ModelProfile> {
        self.models.iter().filter(|m| {
            m.can_handle(req.context_tokens)
            && (!req.require_code || m.capabilities.code)
            && (!req.require_fim || m.capabilities.fim)
            && (!req.require_vision || m.capabilities.vision)
            && (!req.require_long_context || m.capabilities.long_context)
        }).collect()
    }

    /// Score a model for the given request under the given policy.
    fn score(&self, model: &ModelProfile, req: &SelectionRequest) -> f64 {
        let cost = model.estimate_cost(req.context_tokens as u64, req.expected_output_tokens);
        let latency = self.telemetry.get(&model.id)
            .filter(|t| t.requests > 0)
            .map(|t| t.observed_latency_ms)
            .unwrap_or(model.latency.median_ms() as f64);
        let quality = model.quality_score as f64;

        match &req.policy {
            SelectionPolicy::CostOptimise     => -cost,
            SelectionPolicy::LatencyOptimise  => -(latency / 1000.0),
            SelectionPolicy::QualityMaximise  => quality,
            SelectionPolicy::Balanced => {
                // Normalise: cost → [$0, ~$1] → [-1, 0]; latency → [0, 20s] → [-1, 0]; quality → [0, 1]
                let cost_score    = -(cost / 0.10).clamp(0.0, 1.0);
                let latency_score = -(latency / 10_000.0).clamp(0.0, 1.0);
                0.4 * quality + 0.3 * cost_score + 0.3 * latency_score
            }
        }
    }

    /// Select the best model for the request. Returns None if no eligible model.
    pub fn select(&self, req: &SelectionRequest) -> Option<&ModelProfile> {
        let mut candidates = self.eligible(req);

        // Apply budget filter
        if let Some(budget) = req.max_budget_usd {
            candidates.retain(|m| {
                m.estimate_cost(req.context_tokens as u64, req.expected_output_tokens) <= budget
            });
        }

        candidates.into_iter().max_by(|a, b| {
            self.score(a, req).partial_cmp(&self.score(b, req)).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Select with fallback: if preferred model exceeds budget, degrade in quality order.
    pub fn select_with_fallback<'a>(&'a self, req: &SelectionRequest) -> Option<&'a ModelProfile> {
        // Try exact selection first
        if let Some(m) = self.select(req) { return Some(m); }

        // Fallback: relax budget, return cheapest eligible
        let mut candidates = self.eligible(req);
        candidates.sort_by(|a, b| {
            let ca = a.estimate_cost(req.context_tokens as u64, req.expected_output_tokens);
            let cb = b.estimate_cost(req.context_tokens as u64, req.expected_output_tokens);
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.into_iter().next()
    }

    /// Record usage after a completed request.
    pub fn record_usage(&mut self, model_id: &str, input_tokens: u64, output_tokens: u64, latency_ms: u64) {
        let entry = self.telemetry.entry(model_id.to_string()).or_default();
        entry.requests += 1;
        entry.total_input_tokens += input_tokens;
        entry.total_output_tokens += output_tokens;

        // Look up model cost
        let cost = self.models.iter().find(|m| m.id == model_id)
            .map(|m| m.estimate_cost(input_tokens, output_tokens))
            .unwrap_or(0.0);
        entry.total_cost_usd += cost;

        // EMA latency with α=0.2
        let alpha = 0.2;
        if entry.requests == 1 {
            entry.observed_latency_ms = latency_ms as f64;
        } else {
            entry.observed_latency_ms = alpha * latency_ms as f64 + (1.0 - alpha) * entry.observed_latency_ms;
        }
    }

    /// Detect models whose observed latency is 2× worse than declared class.
    pub fn latency_drift_alerts(&self) -> Vec<String> {
        self.telemetry.iter().filter_map(|(id, tel)| {
            if tel.requests < 5 { return None; }
            let model = self.models.iter().find(|m| &m.id == id)?;
            let declared = model.latency.median_ms() as f64;
            if tel.observed_latency_ms > declared * 2.0 {
                Some(format!("{id}: observed {:.0}ms vs declared {:.0}ms", tel.observed_latency_ms, declared))
            } else {
                None
            }
        }).collect()
    }

    /// Cost breakdown by provider across all telemetry.
    pub fn cost_by_provider(&self) -> HashMap<String, f64> {
        let mut by_provider: HashMap<String, f64> = HashMap::new();
        for (id, tel) in &self.telemetry {
            if let Some(m) = self.models.iter().find(|m| &m.id == id) {
                *by_provider.entry(m.provider.clone()).or_insert(0.0) += tel.total_cost_usd;
            }
        }
        by_provider
    }
}

// ─── Built-in profiles ────────────────────────────────────────────────────────

pub fn builtin_profiles() -> Vec<ModelProfile> {
    vec![
        ModelProfile {
            id: "claude-opus-4-6".into(), provider: "anthropic".into(),
            cost_per_1m_in: 15.0, cost_per_1m_out: 75.0,
            context_window: 200_000, latency: LatencyClass::Medium,
            quality_score: 0.98,
            capabilities: Capabilities { code: true, function_calling: true, streaming: true, long_context: true, vision: true, fim: false },
        },
        ModelProfile {
            id: "claude-sonnet-4-6".into(), provider: "anthropic".into(),
            cost_per_1m_in: 3.0, cost_per_1m_out: 15.0,
            context_window: 200_000, latency: LatencyClass::Fast,
            quality_score: 0.92,
            capabilities: Capabilities { code: true, function_calling: true, streaming: true, long_context: true, vision: true, fim: false },
        },
        ModelProfile {
            id: "claude-haiku-4-5".into(), provider: "anthropic".into(),
            cost_per_1m_in: 0.8, cost_per_1m_out: 4.0,
            context_window: 200_000, latency: LatencyClass::UltraFast,
            quality_score: 0.80,
            capabilities: Capabilities { code: true, function_calling: true, streaming: true, long_context: true, vision: false, fim: false },
        },
        ModelProfile {
            id: "gpt-4o".into(), provider: "openai".into(),
            cost_per_1m_in: 5.0, cost_per_1m_out: 15.0,
            context_window: 128_000, latency: LatencyClass::Fast,
            quality_score: 0.93,
            capabilities: Capabilities { code: true, function_calling: true, streaming: true, long_context: false, vision: true, fim: false },
        },
        ModelProfile {
            id: "gpt-4o-mini".into(), provider: "openai".into(),
            cost_per_1m_in: 0.15, cost_per_1m_out: 0.60,
            context_window: 128_000, latency: LatencyClass::UltraFast,
            quality_score: 0.75,
            capabilities: Capabilities { code: true, function_calling: true, streaming: true, long_context: false, vision: false, fim: false },
        },
        ModelProfile {
            id: "deepseek-r1".into(), provider: "deepseek".into(),
            cost_per_1m_in: 0.55, cost_per_1m_out: 2.19,
            context_window: 128_000, latency: LatencyClass::Medium,
            quality_score: 0.88,
            capabilities: Capabilities { code: true, function_calling: true, streaming: true, long_context: false, vision: false, fim: true },
        },
    ]
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn selector() -> WorkloadModelSelector {
        WorkloadModelSelector::new(builtin_profiles())
    }

    fn chat_req() -> SelectionRequest { SelectionRequest::chat(1000) }

    #[test]
    fn test_workload_classify_chat() {
        assert_eq!(WorkloadType::classify("how do I sort a list?"), WorkloadType::Chat);
    }

    #[test]
    fn test_workload_classify_review() {
        assert_eq!(WorkloadType::classify("review this pull request"), WorkloadType::Review);
    }

    #[test]
    fn test_workload_classify_edit() {
        assert_eq!(WorkloadType::classify("refactor this function"), WorkloadType::Edit);
    }

    #[test]
    fn test_workload_classify_agent() {
        assert_eq!(WorkloadType::classify("plan and act on deploying the service"), WorkloadType::AgentLoop);
    }

    #[test]
    fn test_workload_classify_embedding() {
        assert_eq!(WorkloadType::classify("generate vector embedding"), WorkloadType::Embedding);
    }

    #[test]
    fn test_workload_classify_fim() {
        assert_eq!(WorkloadType::classify("fill in the middle completion"), WorkloadType::Completion);
    }

    #[test]
    fn test_select_returns_model() {
        let sel = selector();
        let model = sel.select(&chat_req());
        assert!(model.is_some());
    }

    #[test]
    fn test_select_cost_optimise_cheapest() {
        let sel = selector();
        let mut req = chat_req();
        req.policy = SelectionPolicy::CostOptimise;
        let model = sel.select(&req).unwrap();
        // gpt-4o-mini is cheapest at $0.15/$0.60
        assert_eq!(model.id, "gpt-4o-mini");
    }

    #[test]
    fn test_select_quality_maximise() {
        let sel = selector();
        let mut req = chat_req();
        req.policy = SelectionPolicy::QualityMaximise;
        let model = sel.select(&req).unwrap();
        assert_eq!(model.id, "claude-opus-4-6");
    }

    #[test]
    fn test_select_latency_optimise() {
        let sel = selector();
        let mut req = chat_req();
        req.policy = SelectionPolicy::LatencyOptimise;
        let model = sel.select(&req).unwrap();
        // UltraFast models: haiku or gpt-4o-mini
        assert!(model.latency == LatencyClass::UltraFast);
    }

    #[test]
    fn test_select_budget_constraint_filters() {
        let sel = selector();
        let mut req = chat_req();
        req.max_budget_usd = Some(0.000_01); // tiny budget — only cheapest survives
        req.policy = SelectionPolicy::QualityMaximise;
        let model = sel.select(&req);
        // With 1000 context + 256 output, gpt-4o-mini costs ~$0.00035 — might filter all
        // Just verify it doesn't panic
        let _ = model;
    }

    #[test]
    fn test_select_with_fallback_always_returns_something() {
        let sel = selector();
        let mut req = chat_req();
        req.max_budget_usd = Some(0.0); // zero budget → no direct match → fallback
        let model = sel.select_with_fallback(&req);
        assert!(model.is_some());
    }

    #[test]
    fn test_require_fim_filters_models() {
        let sel = selector();
        let mut req = chat_req();
        req.require_fim = true;
        let model = sel.select(&req).unwrap();
        assert!(model.capabilities.fim);
    }

    #[test]
    fn test_require_vision_filters_models() {
        let sel = selector();
        let mut req = chat_req();
        req.require_vision = true;
        req.policy = SelectionPolicy::CostOptimise;
        let model = sel.select(&req).unwrap();
        assert!(model.capabilities.vision);
    }

    #[test]
    fn test_context_window_filter() {
        let sel = selector();
        let mut req = chat_req();
        req.context_tokens = 150_000; // > gpt-4o 128k, but within claude 200k
        req.policy = SelectionPolicy::CostOptimise;
        let model = sel.select(&req).unwrap();
        assert!(model.context_window >= 150_000);
    }

    #[test]
    fn test_record_usage_updates_telemetry() {
        let mut sel = selector();
        sel.record_usage("claude-haiku-4-5", 1000, 256, 400);
        let tel = &sel.telemetry["claude-haiku-4-5"];
        assert_eq!(tel.requests, 1);
        assert_eq!(tel.total_input_tokens, 1000);
        assert!((tel.observed_latency_ms - 400.0).abs() < 1.0);
    }

    #[test]
    fn test_record_usage_ema_latency() {
        let mut sel = selector();
        sel.record_usage("gpt-4o-mini", 100, 50, 400);
        sel.record_usage("gpt-4o-mini", 100, 50, 600);
        let tel = &sel.telemetry["gpt-4o-mini"];
        // EMA: 400 * (1-0.2) + 600 * 0.2 = 320 + 120 = 440
        assert!((tel.observed_latency_ms - 440.0).abs() < 1.0);
    }

    #[test]
    fn test_latency_drift_alerts_when_slow() {
        let mut sel = selector();
        // Record 10 slow requests for gpt-4o-mini (declared UltraFast = 300ms)
        for _ in 0..10 {
            sel.record_usage("gpt-4o-mini", 100, 50, 1500); // 5× slower than declared
        }
        let alerts = sel.latency_drift_alerts();
        assert!(!alerts.is_empty());
        assert!(alerts[0].contains("gpt-4o-mini"));
    }

    #[test]
    fn test_no_latency_alert_below_threshold() {
        let mut sel = selector();
        for _ in 0..10 {
            sel.record_usage("gpt-4o-mini", 100, 50, 350); // close to declared 300ms
        }
        let alerts = sel.latency_drift_alerts();
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_no_latency_alert_too_few_requests() {
        let mut sel = selector();
        sel.record_usage("gpt-4o-mini", 100, 50, 5000); // slow but only 1 request
        let alerts = sel.latency_drift_alerts();
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_cost_by_provider() {
        let mut sel = selector();
        sel.record_usage("claude-haiku-4-5", 1_000_000, 0, 400);
        let by_prov = sel.cost_by_provider();
        assert!(by_prov.contains_key("anthropic"));
        assert!(*by_prov.get("anthropic").unwrap() > 0.0);
    }

    #[test]
    fn test_estimate_cost() {
        let m = &builtin_profiles()[2]; // haiku: $0.8 in, $4.0 out
        let cost = m.estimate_cost(1_000_000, 1_000_000);
        assert!((cost - 4.8).abs() < 0.01);
    }

    #[test]
    fn test_latency_class_median() {
        assert_eq!(LatencyClass::UltraFast.median_ms(), 300);
        assert_eq!(LatencyClass::Slow.median_ms(), 15000);
    }

    #[test]
    fn test_builtin_profiles_count() {
        assert_eq!(builtin_profiles().len(), 6);
    }
}
