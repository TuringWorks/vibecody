//! Agent trust scoring — tracks model reliability per domain and computes
//! trust scores that drive auto-merge vs manual-review decisions.
//!
//! Gap 21 — Implements exponential decay, recovery on success, domain-scoped
//! trust, and human-readable explanations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trust score for a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustScore {
    pub model_id: String,
    pub score: f64,
    pub window_days: u32,
}

impl TrustScore {
    pub fn new(model_id: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            score: 50.0,
            window_days: 30,
        }
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score.clamp(0.0, 100.0);
        self
    }
}

/// Type of trust event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrustEventType {
    Success,
    Failure,
    Partial,
}

/// A recorded trust event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustEvent {
    pub model_id: String,
    pub event_type: TrustEventType,
    pub domain: Option<String>,
    pub timestamp: u64,
    pub details: String,
}

/// Domain-specific trust score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainTrust {
    pub domain: String,
    pub score: f64,
    pub event_count: u64,
}

impl DomainTrust {
    pub fn new(domain: &str) -> Self {
        Self {
            domain: domain.to_string(),
            score: 50.0,
            event_count: 0,
        }
    }
}

/// Configuration for trust computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustConfig {
    pub decay_rate: f64,
    pub recovery_rate: f64,
    pub initial_score: f64,
    pub failure_penalty: f64,
    pub partial_penalty: f64,
    pub success_bonus: f64,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.02,
            recovery_rate: 0.05,
            initial_score: 50.0,
            failure_penalty: 15.0,
            partial_penalty: 5.0,
            success_bonus: 3.0,
        }
    }
}

/// Thresholds for review decisions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewThresholds {
    pub auto_merge_min: f64,
    pub manual_review_max: f64,
}

impl Default for ReviewThresholds {
    fn default() -> Self {
        Self {
            auto_merge_min: 85.0,
            manual_review_max: 50.0,
        }
    }
}

/// Human-readable trust explanation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustExplanation {
    pub model_id: String,
    pub score: f64,
    pub recent_events_summary: String,
    pub trend: String,
    pub recommendation: String,
}

/// The trust engine tracking scores and events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEngine {
    pub scores: HashMap<String, TrustScore>,
    pub events: Vec<TrustEvent>,
    pub config: TrustConfig,
    pub thresholds: ReviewThresholds,
    domain_scores: HashMap<String, HashMap<String, DomainTrust>>,
}

impl TrustEngine {
    pub fn new(config: TrustConfig, thresholds: ReviewThresholds) -> Self {
        Self {
            scores: HashMap::new(),
            events: Vec::new(),
            config,
            thresholds,
            domain_scores: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(TrustConfig::default(), ReviewThresholds::default())
    }

    /// Record a trust event and update scores.
    pub fn record_event(&mut self, event: TrustEvent) {
        let model_id = event.model_id.clone();
        let domain = event.domain.clone();

        // Update global score
        let score_entry = self.scores.entry(model_id.clone())
            .or_insert_with(|| TrustScore::new(&model_id));

        match event.event_type {
            TrustEventType::Success => {
                score_entry.score = (score_entry.score + self.config.success_bonus).min(100.0);
            }
            TrustEventType::Failure => {
                score_entry.score = (score_entry.score - self.config.failure_penalty).max(0.0);
            }
            TrustEventType::Partial => {
                score_entry.score = (score_entry.score - self.config.partial_penalty).max(0.0);
            }
        }

        // Update domain score
        if let Some(ref d) = domain {
            let model_domains = self.domain_scores
                .entry(model_id.clone())
                .or_default();
            let dt = model_domains.entry(d.clone())
                .or_insert_with(|| DomainTrust::new(d));
            dt.event_count += 1;
            match event.event_type {
                TrustEventType::Success => {
                    dt.score = (dt.score + self.config.success_bonus).min(100.0);
                }
                TrustEventType::Failure => {
                    dt.score = (dt.score - self.config.failure_penalty).max(0.0);
                }
                TrustEventType::Partial => {
                    dt.score = (dt.score - self.config.partial_penalty).max(0.0);
                }
            }
        }

        self.events.push(event);
    }

    /// Get current trust score for a model.
    pub fn get_score(&self, model_id: &str) -> Option<f64> {
        self.scores.get(model_id).map(|s| s.score)
    }

    /// Get domain-specific score.
    pub fn get_domain_score(&self, model_id: &str, domain: &str) -> Option<f64> {
        self.domain_scores.get(model_id)
            .and_then(|m| m.get(domain))
            .map(|dt| dt.score)
    }

    /// Whether this model's score qualifies for auto-merge.
    pub fn should_auto_merge(&self, model_id: &str) -> bool {
        self.get_score(model_id)
            .map(|s| s >= self.thresholds.auto_merge_min)
            .unwrap_or(false)
    }

    /// Whether this model needs manual review.
    pub fn needs_manual_review(&self, model_id: &str) -> bool {
        self.get_score(model_id)
            .map(|s| s <= self.thresholds.manual_review_max)
            .unwrap_or(true)
    }

    /// Generate a human-readable explanation.
    pub fn explain_score(&self, model_id: &str) -> Result<TrustExplanation, String> {
        let score = self.scores.get(model_id)
            .ok_or_else(|| format!("Model {} not found", model_id))?;

        let model_events: Vec<&TrustEvent> = self.events.iter()
            .filter(|e| e.model_id == model_id)
            .collect();

        let total = model_events.len();
        let successes = model_events.iter().filter(|e| e.event_type == TrustEventType::Success).count();
        let failures = model_events.iter().filter(|e| e.event_type == TrustEventType::Failure).count();

        let summary = format!(
            "{} events: {} successes, {} failures, {} partial",
            total, successes, failures, total - successes - failures
        );

        let trend = if total < 3 {
            "insufficient data".to_string()
        } else {
            let recent: Vec<&TrustEvent> = model_events.iter().rev().take(5).cloned().collect();
            let recent_successes = recent.iter().filter(|e| e.event_type == TrustEventType::Success).count();
            if recent_successes > 3 {
                "improving".to_string()
            } else if recent_successes < 2 {
                "declining".to_string()
            } else {
                "stable".to_string()
            }
        };

        let recommendation = if score.score >= self.thresholds.auto_merge_min {
            "Auto-merge eligible".to_string()
        } else if score.score <= self.thresholds.manual_review_max {
            "Requires manual review for all changes".to_string()
        } else {
            "Standard review process".to_string()
        };

        Ok(TrustExplanation {
            model_id: model_id.to_string(),
            score: score.score,
            recent_events_summary: summary,
            trend,
            recommendation,
        })
    }

    /// Apply time-based decay to all scores.
    pub fn decay_scores(&mut self) {
        for score in self.scores.values_mut() {
            let decay = score.score * self.config.decay_rate;
            score.score = (score.score - decay).max(0.0);
        }
        for domains in self.domain_scores.values_mut() {
            for dt in domains.values_mut() {
                let decay = dt.score * self.config.decay_rate;
                dt.score = (dt.score - decay).max(0.0);
            }
        }
    }

    /// Calibrate a model's score to a specific value.
    pub fn calibrate(&mut self, model_id: &str, new_score: f64) -> Result<(), String> {
        if !(0.0..=100.0).contains(&new_score) {
            return Err("Score must be between 0 and 100".to_string());
        }
        let entry = self.scores.entry(model_id.to_string())
            .or_insert_with(|| TrustScore::new(model_id));
        entry.score = new_score;
        Ok(())
    }

    /// List all tracked model IDs.
    pub fn list_models(&self) -> Vec<String> {
        self.scores.keys().cloned().collect()
    }

    /// Get all domain scores for a model.
    pub fn get_all_domain_scores(&self, model_id: &str) -> Vec<DomainTrust> {
        self.domain_scores.get(model_id)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eng() -> TrustEngine {
        TrustEngine::with_defaults()
    }

    fn success_event(model: &str) -> TrustEvent {
        TrustEvent {
            model_id: model.to_string(),
            event_type: TrustEventType::Success,
            domain: None,
            timestamp: 1,
            details: "ok".to_string(),
        }
    }

    fn failure_event(model: &str) -> TrustEvent {
        TrustEvent {
            model_id: model.to_string(),
            event_type: TrustEventType::Failure,
            domain: None,
            timestamp: 1,
            details: "failed".to_string(),
        }
    }

    fn domain_event(model: &str, domain: &str, et: TrustEventType) -> TrustEvent {
        TrustEvent {
            model_id: model.to_string(),
            event_type: et,
            domain: Some(domain.to_string()),
            timestamp: 1,
            details: "domain event".to_string(),
        }
    }

    #[test]
    fn test_trust_score_new() {
        let s = TrustScore::new("gpt4");
        assert_eq!(s.score, 50.0);
        assert_eq!(s.window_days, 30);
    }

    #[test]
    fn test_trust_score_with_score() {
        let s = TrustScore::new("m").with_score(90.0);
        assert_eq!(s.score, 90.0);
    }

    #[test]
    fn test_trust_score_clamp() {
        let s = TrustScore::new("m").with_score(150.0);
        assert_eq!(s.score, 100.0);
        let s2 = TrustScore::new("m").with_score(-10.0);
        assert_eq!(s2.score, 0.0);
    }

    #[test]
    fn test_engine_new() {
        let e = eng();
        assert!(e.scores.is_empty());
        assert!(e.events.is_empty());
    }

    #[test]
    fn test_record_success() {
        let mut e = eng();
        e.record_event(success_event("m1"));
        assert!(e.get_score("m1").unwrap() > 50.0);
    }

    #[test]
    fn test_record_failure() {
        let mut e = eng();
        e.record_event(failure_event("m1"));
        assert!(e.get_score("m1").unwrap() < 50.0);
    }

    #[test]
    fn test_record_partial() {
        let mut e = eng();
        e.record_event(TrustEvent {
            model_id: "m1".to_string(),
            event_type: TrustEventType::Partial,
            domain: None,
            timestamp: 1,
            details: "partial".to_string(),
        });
        assert!(e.get_score("m1").unwrap() < 50.0);
    }

    #[test]
    fn test_score_capped_at_100() {
        let mut e = eng();
        e.calibrate("m1", 99.0).unwrap();
        e.record_event(success_event("m1"));
        assert!(e.get_score("m1").unwrap() <= 100.0);
    }

    #[test]
    fn test_score_floored_at_0() {
        let mut e = eng();
        e.calibrate("m1", 5.0).unwrap();
        e.record_event(failure_event("m1"));
        assert!(e.get_score("m1").unwrap() >= 0.0);
    }

    #[test]
    fn test_get_score_missing() {
        let e = eng();
        assert!(e.get_score("nope").is_none());
    }

    #[test]
    fn test_domain_score() {
        let mut e = eng();
        e.record_event(domain_event("m1", "rust", TrustEventType::Success));
        let ds = e.get_domain_score("m1", "rust").unwrap();
        assert!(ds > 50.0);
    }

    #[test]
    fn test_domain_score_missing() {
        let e = eng();
        assert!(e.get_domain_score("m1", "go").is_none());
    }

    #[test]
    fn test_should_auto_merge_high() {
        let mut e = eng();
        e.calibrate("m1", 90.0).unwrap();
        assert!(e.should_auto_merge("m1"));
    }

    #[test]
    fn test_should_auto_merge_low() {
        let mut e = eng();
        e.calibrate("m1", 60.0).unwrap();
        assert!(!e.should_auto_merge("m1"));
    }

    #[test]
    fn test_should_auto_merge_missing() {
        let e = eng();
        assert!(!e.should_auto_merge("nope"));
    }

    #[test]
    fn test_needs_manual_review_low() {
        let mut e = eng();
        e.calibrate("m1", 30.0).unwrap();
        assert!(e.needs_manual_review("m1"));
    }

    #[test]
    fn test_needs_manual_review_high() {
        let mut e = eng();
        e.calibrate("m1", 70.0).unwrap();
        assert!(!e.needs_manual_review("m1"));
    }

    #[test]
    fn test_needs_manual_review_missing() {
        let e = eng();
        assert!(e.needs_manual_review("nope"));
    }

    #[test]
    fn test_explain_score() {
        let mut e = eng();
        e.record_event(success_event("m1"));
        e.record_event(failure_event("m1"));
        let expl = e.explain_score("m1").unwrap();
        assert_eq!(expl.model_id, "m1");
        assert!(expl.recent_events_summary.contains("2 events"));
    }

    #[test]
    fn test_explain_score_not_found() {
        let e = eng();
        assert!(e.explain_score("nope").is_err());
    }

    #[test]
    fn test_explain_recommendation_auto_merge() {
        let mut e = eng();
        e.calibrate("m1", 90.0).unwrap();
        e.record_event(success_event("m1"));
        let expl = e.explain_score("m1").unwrap();
        assert!(expl.recommendation.contains("Auto-merge"));
    }

    #[test]
    fn test_explain_recommendation_manual() {
        let mut e = eng();
        e.calibrate("m1", 10.0).unwrap();
        e.record_event(failure_event("m1"));
        let expl = e.explain_score("m1").unwrap();
        assert!(expl.recommendation.contains("manual review"));
    }

    #[test]
    fn test_decay_scores() {
        let mut e = eng();
        e.calibrate("m1", 80.0).unwrap();
        let before = e.get_score("m1").unwrap();
        e.decay_scores();
        let after = e.get_score("m1").unwrap();
        assert!(after < before);
    }

    #[test]
    fn test_decay_domain_scores() {
        let mut e = eng();
        e.record_event(domain_event("m1", "rust", TrustEventType::Success));
        let before = e.get_domain_score("m1", "rust").unwrap();
        e.decay_scores();
        let after = e.get_domain_score("m1", "rust").unwrap();
        assert!(after < before);
    }

    #[test]
    fn test_calibrate() {
        let mut e = eng();
        e.calibrate("m1", 75.0).unwrap();
        assert_eq!(e.get_score("m1").unwrap(), 75.0);
    }

    #[test]
    fn test_calibrate_out_of_range() {
        let mut e = eng();
        assert!(e.calibrate("m1", 101.0).is_err());
        assert!(e.calibrate("m1", -1.0).is_err());
    }

    #[test]
    fn test_list_models() {
        let mut e = eng();
        e.record_event(success_event("m1"));
        e.record_event(success_event("m2"));
        let models = e.list_models();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_get_all_domain_scores() {
        let mut e = eng();
        e.record_event(domain_event("m1", "rust", TrustEventType::Success));
        e.record_event(domain_event("m1", "python", TrustEventType::Failure));
        let domains = e.get_all_domain_scores("m1");
        assert_eq!(domains.len(), 2);
    }

    #[test]
    fn test_get_all_domain_scores_empty() {
        let e = eng();
        assert!(e.get_all_domain_scores("nope").is_empty());
    }

    #[test]
    fn test_config_default() {
        let cfg = TrustConfig::default();
        assert_eq!(cfg.initial_score, 50.0);
        assert_eq!(cfg.failure_penalty, 15.0);
    }

    #[test]
    fn test_thresholds_default() {
        let t = ReviewThresholds::default();
        assert_eq!(t.auto_merge_min, 85.0);
        assert_eq!(t.manual_review_max, 50.0);
    }

    #[test]
    fn test_trust_event_serde() {
        let e = success_event("m1");
        let json = serde_json::to_string(&e).unwrap();
        let de: TrustEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, de);
    }

    #[test]
    fn test_domain_trust_new() {
        let dt = DomainTrust::new("rust");
        assert_eq!(dt.domain, "rust");
        assert_eq!(dt.score, 50.0);
        assert_eq!(dt.event_count, 0);
    }

    #[test]
    fn test_multiple_successes_increase() {
        let mut e = eng();
        for _ in 0..10 {
            e.record_event(success_event("m1"));
        }
        assert!(e.get_score("m1").unwrap() > 70.0);
    }
}
