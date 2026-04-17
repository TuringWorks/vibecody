//! Token budget dashboard — real-time tracking of prompt/completion tokens.
//! FIT-GAP v11 Phase 48 — closes gap vs Claude Code 1.x.

use std::collections::{HashMap, VecDeque};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single token usage record for one LLM call.
#[derive(Debug, Clone)]
pub struct TokenRecord {
    pub call_id: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cached_tokens: u64,
    pub timestamp_ms: u64,
}

impl TokenRecord {
    pub fn total(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens
    }
    pub fn cost_usd(&self, prompt_rate: f64, completion_rate: f64) -> f64 {
        (self.prompt_tokens as f64 / 1_000_000.0) * prompt_rate
            + (self.completion_tokens as f64 / 1_000_000.0) * completion_rate
    }
}

/// Budget allocation per model or session.
#[derive(Debug, Clone)]
pub struct Budget {
    pub label: String,
    pub max_tokens: u64,
    pub used_tokens: u64,
}

impl Budget {
    pub fn new(label: impl Into<String>, max_tokens: u64) -> Self {
        Self { label: label.into(), max_tokens, used_tokens: 0 }
    }
    pub fn remaining(&self) -> u64 {
        self.max_tokens.saturating_sub(self.used_tokens)
    }
    pub fn utilization_pct(&self) -> f64 {
        if self.max_tokens == 0 { return 0.0; }
        (self.used_tokens as f64 / self.max_tokens as f64) * 100.0
    }
    pub fn is_exhausted(&self) -> bool {
        self.used_tokens >= self.max_tokens
    }
    pub fn warn_threshold(&self) -> bool {
        self.utilization_pct() >= 80.0
    }
}

/// Summary statistics for a dashboard view.
#[derive(Debug, Clone)]
pub struct DashboardStats {
    pub total_calls: usize,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_cached_tokens: u64,
    pub total_cost_usd: f64,
    pub by_model: HashMap<String, ModelStats>,
}

#[derive(Debug, Clone, Default)]
pub struct ModelStats {
    pub calls: usize,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

/// Token budget dashboard — tracks usage, budgets, and per-model stats.
#[derive(Debug)]
pub struct TokenDashboard {
    records: VecDeque<TokenRecord>,
    budgets: HashMap<String, Budget>,
    /// Per-model pricing: (prompt_per_M, completion_per_M)
    pricing: HashMap<String, (f64, f64)>,
    max_history: usize,
    session_id: String,
}

impl Default for TokenDashboard {
    fn default() -> Self {
        Self::new("default")
    }
}

impl TokenDashboard {
    pub fn new(session_id: impl Into<String>) -> Self {
        let mut dash = Self {
            records: VecDeque::new(),
            budgets: HashMap::new(),
            pricing: HashMap::new(),
            max_history: 1000,
            session_id: session_id.into(),
        };
        // Built-in pricing for common models (per 1M tokens)
        dash.pricing.insert("claude-opus-4-6".to_string(), (15.0, 75.0));
        dash.pricing.insert("claude-sonnet-4-6".to_string(), (3.0, 15.0));
        dash.pricing.insert("claude-haiku-4-5".to_string(), (0.25, 1.25));
        dash.pricing.insert("gpt-4o".to_string(), (5.0, 15.0));
        dash.pricing.insert("gpt-4o-mini".to_string(), (0.15, 0.60));
        dash.pricing.insert("gemini-2.5-pro".to_string(), (1.25, 5.0));
        dash
    }

    /// Record a token usage event.
    pub fn record(&mut self, record: TokenRecord) {
        // Update budget if one matches the model or a wildcard.
        let total = record.total();
        let model = record.model.clone();
        if let Some(budget) = self.budgets.get_mut(&model) {
            budget.used_tokens = budget.used_tokens.saturating_add(total);
        }
        if let Some(budget) = self.budgets.get_mut("*") {
            budget.used_tokens = budget.used_tokens.saturating_add(total);
        }
        if self.records.len() >= self.max_history {
            self.records.pop_front();
        }
        self.records.push_back(record);
    }

    /// Add a budget constraint.
    pub fn set_budget(&mut self, label: impl Into<String>, max_tokens: u64) {
        let lbl = label.into();
        self.budgets.insert(lbl.clone(), Budget::new(lbl, max_tokens));
    }

    /// Get budget info by label.
    pub fn budget(&self, label: &str) -> Option<&Budget> {
        self.budgets.get(label)
    }

    /// Compute dashboard stats over all records.
    pub fn stats(&self) -> DashboardStats {
        let mut total_prompt = 0u64;
        let mut total_completion = 0u64;
        let mut total_cached = 0u64;
        let mut total_cost = 0.0f64;
        let mut by_model: HashMap<String, ModelStats> = HashMap::new();

        for r in &self.records {
            total_prompt += r.prompt_tokens;
            total_completion += r.completion_tokens;
            total_cached += r.cached_tokens;
            let (pr, cr) = self.pricing.get(&r.model).copied().unwrap_or((0.0, 0.0));
            total_cost += r.cost_usd(pr, cr);
            let ms = by_model.entry(r.model.clone()).or_default();
            ms.calls += 1;
            ms.prompt_tokens += r.prompt_tokens;
            ms.completion_tokens += r.completion_tokens;
        }

        DashboardStats {
            total_calls: self.records.len(),
            total_prompt_tokens: total_prompt,
            total_completion_tokens: total_completion,
            total_cached_tokens: total_cached,
            total_cost_usd: total_cost,
            by_model,
        }
    }

    /// Format a human-readable dashboard summary.
    pub fn render_text(&self) -> String {
        let stats = self.stats();
        let mut lines = vec![
            format!("# Token Dashboard — session: {}", self.session_id),
            format!("Calls: {}  |  Prompt: {}  |  Completion: {}  |  Cached: {}",
                stats.total_calls,
                stats.total_prompt_tokens,
                stats.total_completion_tokens,
                stats.total_cached_tokens,
            ),
            format!("Est. Cost: ${:.4}", stats.total_cost_usd),
        ];
        if !stats.by_model.is_empty() {
            lines.push("\n## By Model".to_string());
            let mut models: Vec<_> = stats.by_model.iter().collect();
            models.sort_by_key(|(k, _)| k.as_str());
            for (model, ms) in models {
                lines.push(format!("  {} — {} calls, {} prompt, {} completion",
                    model, ms.calls, ms.prompt_tokens, ms.completion_tokens));
            }
        }
        if !self.budgets.is_empty() {
            lines.push("\n## Budgets".to_string());
            let mut budgets: Vec<_> = self.budgets.values().collect();
            budgets.sort_by_key(|b| b.label.as_str());
            for b in budgets {
                let warn = if b.warn_threshold() { " ⚠" } else { "" };
                lines.push(format!("  {} — {}/{} ({:.1}%){}",
                    b.label, b.used_tokens, b.max_tokens, b.utilization_pct(), warn));
            }
        }
        lines.join("\n")
    }

    /// Return records within a time window.
    pub fn records_in_window(&self, start_ms: u64, end_ms: u64) -> Vec<&TokenRecord> {
        self.records.iter()
            .filter(|r| r.timestamp_ms >= start_ms && r.timestamp_ms <= end_ms)
            .collect()
    }

    pub fn record_count(&self) -> usize { self.records.len() }
    pub fn session_id(&self) -> &str { &self.session_id }

    /// Set custom pricing for a model.
    pub fn set_pricing(&mut self, model: impl Into<String>, prompt_per_m: f64, completion_per_m: f64) {
        self.pricing.insert(model.into(), (prompt_per_m, completion_per_m));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(id: &str, model: &str, prompt: u64, completion: u64, ts: u64) -> TokenRecord {
        TokenRecord {
            call_id: id.to_string(),
            model: model.to_string(),
            prompt_tokens: prompt,
            completion_tokens: completion,
            cached_tokens: 0,
            timestamp_ms: ts,
        }
    }

    #[test]
    fn test_record_and_count() {
        let mut d = TokenDashboard::new("s1");
        d.record(make_record("c1", "claude-sonnet-4-6", 100, 50, 1000));
        assert_eq!(d.record_count(), 1);
    }

    #[test]
    fn test_stats_totals() {
        let mut d = TokenDashboard::new("s1");
        d.record(make_record("c1", "gpt-4o-mini", 200, 100, 1000));
        d.record(make_record("c2", "gpt-4o-mini", 300, 150, 2000));
        let s = d.stats();
        assert_eq!(s.total_prompt_tokens, 500);
        assert_eq!(s.total_completion_tokens, 250);
        assert_eq!(s.total_calls, 2);
    }

    #[test]
    fn test_budget_tracking() {
        let mut d = TokenDashboard::new("s1");
        d.set_budget("claude-sonnet-4-6", 1000);
        d.record(make_record("c1", "claude-sonnet-4-6", 400, 200, 1));
        let b = d.budget("claude-sonnet-4-6").unwrap();
        assert_eq!(b.used_tokens, 600);
        assert_eq!(b.remaining(), 400);
    }

    #[test]
    fn test_budget_warn_threshold() {
        let mut b = Budget::new("test", 1000);
        b.used_tokens = 850;
        assert!(b.warn_threshold());
    }

    #[test]
    fn test_budget_exhausted() {
        let mut b = Budget::new("test", 500);
        b.used_tokens = 500;
        assert!(b.is_exhausted());
    }

    #[test]
    fn test_budget_utilization_zero_max() {
        let b = Budget::new("test", 0);
        assert_eq!(b.utilization_pct(), 0.0);
    }

    #[test]
    fn test_wildcard_budget() {
        let mut d = TokenDashboard::new("s1");
        d.set_budget("*", 10000);
        d.record(make_record("c1", "gpt-4o", 500, 200, 1));
        let b = d.budget("*").unwrap();
        assert_eq!(b.used_tokens, 700);
    }

    #[test]
    fn test_cost_estimation() {
        let mut d = TokenDashboard::new("s1");
        // 1M prompt at $3/M = $3, 500k completion at $15/M = $7.5
        d.record(make_record("c1", "claude-sonnet-4-6", 1_000_000, 500_000, 1));
        let s = d.stats();
        assert!((s.total_cost_usd - 10.5).abs() < 0.01);
    }

    #[test]
    fn test_custom_pricing() {
        let mut d = TokenDashboard::new("s1");
        d.set_pricing("my-model", 1.0, 2.0);
        d.record(make_record("c1", "my-model", 1_000_000, 1_000_000, 1));
        let s = d.stats();
        assert!((s.total_cost_usd - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_by_model_breakdown() {
        let mut d = TokenDashboard::new("s1");
        d.record(make_record("c1", "model-a", 100, 50, 1));
        d.record(make_record("c2", "model-b", 200, 100, 2));
        d.record(make_record("c3", "model-a", 150, 75, 3));
        let s = d.stats();
        assert_eq!(s.by_model["model-a"].calls, 2);
        assert_eq!(s.by_model["model-b"].calls, 1);
    }

    #[test]
    fn test_records_in_window() {
        let mut d = TokenDashboard::new("s1");
        d.record(make_record("c1", "m", 10, 5, 100));
        d.record(make_record("c2", "m", 10, 5, 200));
        d.record(make_record("c3", "m", 10, 5, 300));
        let w = d.records_in_window(150, 250);
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn test_render_text() {
        let mut d = TokenDashboard::new("test-session");
        d.record(make_record("c1", "claude-sonnet-4-6", 100, 50, 1));
        let text = d.render_text();
        assert!(text.contains("test-session"));
        assert!(text.contains("claude-sonnet-4-6"));
    }

    #[test]
    fn test_max_history_eviction() {
        let mut d = TokenDashboard::new("s1");
        d.max_history = 5;
        for i in 0..10 {
            d.record(make_record(&format!("c{}", i), "m", 1, 1, i as u64));
        }
        assert_eq!(d.record_count(), 5);
    }
}
