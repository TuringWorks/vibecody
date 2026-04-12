#![allow(dead_code)]
//! Performance regression detection — baseline + threshold alerts.
//! FIT-GAP v11 Phase 48 — closes gap vs Devin 2.0.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single performance sample.
#[derive(Debug, Clone)]
pub struct PerfSample {
    pub benchmark: String,
    pub value: f64,
    pub unit: String,
    pub timestamp_ms: u64,
    pub metadata: HashMap<String, String>,
}

impl PerfSample {
    pub fn new(benchmark: impl Into<String>, value: f64, unit: impl Into<String>, ts: u64) -> Self {
        Self { benchmark: benchmark.into(), value, unit: unit.into(), timestamp_ms: ts, metadata: HashMap::new() }
    }
}

/// Baseline statistics for a benchmark.
#[derive(Debug, Clone)]
pub struct Baseline {
    pub benchmark: String,
    pub mean: f64,
    pub std_dev: f64,
    pub sample_count: usize,
}

impl Baseline {
    /// Check if a value is within `z_threshold` standard deviations of the mean.
    pub fn is_regression(&self, value: f64, z_threshold: f64) -> bool {
        if self.std_dev == 0.0 {
            return value > self.mean * 1.05; // 5% grace when std_dev = 0
        }
        let z = (value - self.mean) / self.std_dev;
        z > z_threshold
    }

    /// Percent change relative to baseline mean.
    pub fn pct_change(&self, value: f64) -> f64 {
        if self.mean == 0.0 { return 0.0; }
        ((value - self.mean) / self.mean) * 100.0
    }
}

/// Severity of a detected regression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegressionSeverity {
    Minor,   // 5–10% degradation
    Major,   // 10–25% degradation
    Critical, // >25% degradation
}

impl RegressionSeverity {
    pub fn from_pct(pct: f64) -> Option<Self> {
        if pct > 25.0 { Some(Self::Critical) }
        else if pct > 10.0 { Some(Self::Major) }
        else if pct > 5.0 { Some(Self::Minor) }
        else { None }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Minor => "minor",
            Self::Major => "major",
            Self::Critical => "critical",
        }
    }
}

/// A detected regression alert.
#[derive(Debug, Clone)]
pub struct RegressionAlert {
    pub benchmark: String,
    pub baseline_mean: f64,
    pub observed: f64,
    pub pct_change: f64,
    pub severity: RegressionSeverity,
    pub timestamp_ms: u64,
}

// ---------------------------------------------------------------------------
// Detector
// ---------------------------------------------------------------------------

/// Collects samples, builds baselines, and detects regressions.
#[derive(Debug, Default)]
pub struct PerfRegressionDetector {
    baselines: HashMap<String, Baseline>,
    /// History per benchmark (most recent N).
    history: HashMap<String, Vec<PerfSample>>,
    history_limit: usize,
    z_threshold: f64,
}

impl PerfRegressionDetector {
    pub fn new() -> Self {
        Self {
            baselines: HashMap::new(),
            history: HashMap::new(),
            history_limit: 100,
            z_threshold: 2.0,
        }
    }

    pub fn with_z_threshold(mut self, z: f64) -> Self {
        self.z_threshold = z;
        self
    }

    /// Set a baseline directly (e.g., from a known-good run).
    pub fn set_baseline(&mut self, b: Baseline) {
        self.baselines.insert(b.benchmark.clone(), b);
    }

    /// Build a baseline from the current history for a benchmark.
    pub fn compute_baseline(&mut self, benchmark: &str) -> Option<Baseline> {
        let samples = self.history.get(benchmark)?;
        if samples.is_empty() { return None; }
        let values: Vec<f64> = samples.iter().map(|s| s.value).collect();
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();
        let b = Baseline { benchmark: benchmark.to_string(), mean, std_dev, sample_count: values.len() };
        self.baselines.insert(benchmark.to_string(), b.clone());
        Some(b)
    }

    /// Record a sample and check for regressions.
    pub fn record(&mut self, sample: PerfSample) -> Option<RegressionAlert> {
        let benchmark = sample.benchmark.clone();
        let value = sample.value;
        let ts = sample.timestamp_ms;

        let h = self.history.entry(benchmark.clone()).or_default();
        h.push(sample);
        if h.len() > self.history_limit {
            h.remove(0);
        }

        // Check against existing baseline.
        if let Some(baseline) = self.baselines.get(&benchmark) {
            let pct = baseline.pct_change(value);
            if baseline.is_regression(value, self.z_threshold) {
                if let Some(severity) = RegressionSeverity::from_pct(pct) {
                    return Some(RegressionAlert {
                        benchmark,
                        baseline_mean: baseline.mean,
                        observed: value,
                        pct_change: pct,
                        severity,
                        timestamp_ms: ts,
                    });
                }
            }
        }
        None
    }

    /// Batch analysis: check all samples against their baselines.
    pub fn analyze_all(&self) -> Vec<RegressionAlert> {
        let mut alerts = Vec::new();
        for (benchmark, samples) in &self.history {
            if let Some(baseline) = self.baselines.get(benchmark) {
                for s in samples {
                    let pct = baseline.pct_change(s.value);
                    if baseline.is_regression(s.value, self.z_threshold) {
                        if let Some(severity) = RegressionSeverity::from_pct(pct) {
                            alerts.push(RegressionAlert {
                                benchmark: benchmark.clone(),
                                baseline_mean: baseline.mean,
                                observed: s.value,
                                pct_change: pct,
                                severity,
                                timestamp_ms: s.timestamp_ms,
                            });
                        }
                    }
                }
            }
        }
        alerts.sort_by(|a, b| b.severity.cmp(&a.severity));
        alerts
    }

    /// Summary text for detected regressions.
    pub fn summary(&self) -> String {
        let alerts = self.analyze_all();
        if alerts.is_empty() {
            return "No regressions detected.".to_string();
        }
        let mut lines = vec![format!("{} regression(s) detected:", alerts.len())];
        for a in &alerts {
            lines.push(format!("  [{}] {} — {:.1}% slower (baseline: {:.2}, observed: {:.2})",
                a.severity.as_str(), a.benchmark, a.pct_change, a.baseline_mean, a.observed));
        }
        lines.join("\n")
    }

    pub fn baseline_count(&self) -> usize { self.baselines.len() }
    pub fn history_len(&self, benchmark: &str) -> usize {
        self.history.get(benchmark).map(|h| h.len()).unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(name: &str, mean: f64, std_dev: f64) -> Baseline {
        Baseline { benchmark: name.to_string(), mean, std_dev, sample_count: 10 }
    }

    fn sample(name: &str, value: f64, ts: u64) -> PerfSample {
        PerfSample::new(name, value, "ms", ts)
    }

    #[test]
    fn test_no_regression_within_threshold() {
        let b = baseline("latency", 100.0, 10.0);
        assert!(!b.is_regression(105.0, 2.0)); // z = 0.5
    }

    #[test]
    fn test_regression_detected() {
        let b = baseline("latency", 100.0, 10.0);
        // z = (130 - 100) / 10 = 3.0 > 2.0
        assert!(b.is_regression(130.0, 2.0));
    }

    #[test]
    fn test_pct_change() {
        let b = baseline("tput", 1000.0, 50.0);
        assert!((b.pct_change(1100.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_pct_change_zero_mean() {
        let b = baseline("metric", 0.0, 0.0);
        assert_eq!(b.pct_change(5.0), 0.0);
    }

    #[test]
    fn test_severity_from_pct() {
        assert_eq!(RegressionSeverity::from_pct(4.0), None);
        assert_eq!(RegressionSeverity::from_pct(7.0), Some(RegressionSeverity::Minor));
        assert_eq!(RegressionSeverity::from_pct(15.0), Some(RegressionSeverity::Major));
        assert_eq!(RegressionSeverity::from_pct(30.0), Some(RegressionSeverity::Critical));
    }

    #[test]
    fn test_record_no_baseline_no_alert() {
        let mut d = PerfRegressionDetector::new();
        let alert = d.record(sample("latency", 200.0, 1));
        assert!(alert.is_none());
    }

    #[test]
    fn test_record_with_baseline_triggers_alert() {
        let mut d = PerfRegressionDetector::new();
        d.set_baseline(baseline("latency", 100.0, 5.0));
        // 200ms is way above baseline, should be critical
        let alert = d.record(sample("latency", 200.0, 1));
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().severity, RegressionSeverity::Critical);
    }

    #[test]
    fn test_record_good_value_no_alert() {
        let mut d = PerfRegressionDetector::new();
        d.set_baseline(baseline("latency", 100.0, 10.0));
        let alert = d.record(sample("latency", 101.0, 1));
        assert!(alert.is_none());
    }

    #[test]
    fn test_compute_baseline() {
        let mut d = PerfRegressionDetector::new();
        for i in 0..5 {
            d.record(sample("tput", 100.0 + i as f64, i as u64));
        }
        let b = d.compute_baseline("tput");
        assert!(b.is_some());
        assert!((b.unwrap().mean - 102.0).abs() < 0.01);
    }

    #[test]
    fn test_analyze_all() {
        let mut d = PerfRegressionDetector::new();
        d.set_baseline(baseline("cpu", 50.0, 2.0));
        d.record(sample("cpu", 80.0, 1)); // Major regression
        let alerts = d.analyze_all();
        assert!(!alerts.is_empty());
    }

    #[test]
    fn test_summary_no_regressions() {
        let d = PerfRegressionDetector::new();
        assert_eq!(d.summary(), "No regressions detected.");
    }

    #[test]
    fn test_summary_with_regression() {
        let mut d = PerfRegressionDetector::new();
        d.set_baseline(baseline("latency", 100.0, 5.0));
        d.record(sample("latency", 200.0, 1));
        let s = d.summary();
        assert!(s.contains("latency"));
    }

    #[test]
    fn test_history_limit() {
        let mut d = PerfRegressionDetector::new();
        d.history_limit = 5;
        for i in 0..10u64 {
            d.record(sample("m", 1.0, i));
        }
        assert_eq!(d.history_len("m"), 5);
    }
}
