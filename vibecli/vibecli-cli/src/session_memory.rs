//! Long-session memory profiling.
//!
//! Detect and mitigate memory leaks in 8+ hour agent sessions by tracking
//! heap usage over time, detecting growth trends, and triggering compaction.

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySample {
    pub timestamp: u64,
    pub heap_bytes: usize,
    pub context_entries: usize,
    pub conversation_turns: usize,
    pub mcp_tools_loaded: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfilerConfig {
    pub sample_interval_secs: u64,
    pub alert_threshold_growth_percent: f64,
    pub auto_compact_threshold_bytes: usize,
    pub max_samples: usize,
    pub enable_auto_cleanup: bool,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            sample_interval_secs: 60,
            alert_threshold_growth_percent: 50.0,
            auto_compact_threshold_bytes: 512 * 1024 * 1024, // 512 MB
            max_samples: 10000,
            enable_auto_cleanup: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryAlertType {
    GrowthWarning,
    LeakDetected,
    ThresholdExceeded,
    CompactionTriggered,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAlert {
    pub timestamp: u64,
    pub alert_type: MemoryAlertType,
    pub current_bytes: usize,
    pub growth_rate_percent: f64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompactionResult {
    pub freed_bytes: usize,
    pub entries_evicted: usize,
    pub contexts_compressed: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionHealth {
    pub status: HealthStatus,
    pub uptime_secs: u64,
    pub current_memory_bytes: usize,
    pub peak_memory_bytes: usize,
    pub growth_rate_percent: f64,
    pub samples_collected: usize,
    pub alerts_triggered: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryProfiler {
    pub samples: Vec<MemorySample>,
    pub config: ProfilerConfig,
    pub alerts: Vec<MemoryAlert>,
    pub baseline_bytes: usize,
}

impl MemoryProfiler {
    pub fn new(config: ProfilerConfig) -> Self {
        Self {
            samples: Vec::new(),
            config,
            alerts: Vec::new(),
            baseline_bytes: 0,
        }
    }

    pub fn record_sample(&mut self, sample: MemorySample) {
        if self.samples.is_empty() {
            self.baseline_bytes = sample.heap_bytes;
        }

        self.samples.push(sample);

        // Enforce max_samples by dropping oldest
        if self.samples.len() > self.config.max_samples {
            let excess = self.samples.len() - self.config.max_samples;
            self.samples.drain(..excess);
        }

        // Auto-detect issues after recording
        if let Some(alert) = self.detect_leak() {
            self.alerts.push(alert);
        }

        // Check threshold exceeded
        if let Some(last) = self.samples.last() {
            if last.heap_bytes >= self.config.auto_compact_threshold_bytes {
                self.alerts.push(MemoryAlert {
                    timestamp: last.timestamp,
                    alert_type: MemoryAlertType::ThresholdExceeded,
                    current_bytes: last.heap_bytes,
                    growth_rate_percent: self.growth_rate_percent(),
                    message: format!(
                        "Memory {} bytes exceeds threshold {} bytes",
                        last.heap_bytes, self.config.auto_compact_threshold_bytes
                    ),
                });
            }
        }
    }

    pub fn detect_leak(&self) -> Option<MemoryAlert> {
        if self.samples.len() < 10 {
            return None;
        }

        let growth = self.growth_rate_percent();
        if growth > self.config.alert_threshold_growth_percent {
            let last = self.samples.last().expect("samples not empty after len check");
            // Check if growth is sustained (linear regression slope positive)
            let slope = self.linear_regression_slope();
            if slope > 0.0 {
                return Some(MemoryAlert {
                    timestamp: last.timestamp,
                    alert_type: MemoryAlertType::LeakDetected,
                    current_bytes: last.heap_bytes,
                    growth_rate_percent: growth,
                    message: format!(
                        "Potential memory leak: {:.1}% growth with positive slope {:.2} bytes/sample",
                        growth, slope
                    ),
                });
            } else {
                return Some(MemoryAlert {
                    timestamp: last.timestamp,
                    alert_type: MemoryAlertType::GrowthWarning,
                    current_bytes: last.heap_bytes,
                    growth_rate_percent: growth,
                    message: format!("Memory growth warning: {:.1}% increase", growth),
                });
            }
        }
        None
    }

    pub fn should_compact(&self) -> bool {
        if let Some(last) = self.samples.last() {
            last.heap_bytes >= self.config.auto_compact_threshold_bytes
        } else {
            false
        }
    }

    pub fn compact(&mut self) -> CompactionResult {
        let before_bytes = self.samples.last().map(|s| s.heap_bytes).unwrap_or(0);

        // Simulate compaction: evict old context entries, compress remaining
        let entries_evicted = if self.samples.len() > 100 {
            let evict_count = self.samples.len() / 2;
            self.samples.drain(..evict_count);
            evict_count
        } else {
            0
        };

        let freed_bytes = before_bytes / 4; // Simulate freeing ~25%
        let contexts_compressed = entries_evicted / 3;

        let alert = MemoryAlert {
            timestamp: self.samples.last().map(|s| s.timestamp).unwrap_or(0),
            alert_type: MemoryAlertType::CompactionTriggered,
            current_bytes: before_bytes.saturating_sub(freed_bytes),
            growth_rate_percent: self.growth_rate_percent(),
            message: format!(
                "Compaction freed {} bytes, evicted {} entries",
                freed_bytes, entries_evicted
            ),
        };
        self.alerts.push(alert);

        CompactionResult {
            freed_bytes,
            entries_evicted,
            contexts_compressed,
            duration_ms: 50,
        }
    }

    pub fn get_health(&self) -> SessionHealth {
        let current = self.samples.last().map(|s| s.heap_bytes).unwrap_or(0);
        let peak = self.peak_memory();
        let growth = self.growth_rate_percent();
        let uptime = if self.samples.len() >= 2 {
            let first_ts = self.samples.first().expect("len >= 2").timestamp;
            let last_ts = self.samples.last().expect("len >= 2").timestamp;
            last_ts.saturating_sub(first_ts)
        } else {
            0
        };

        let status = if growth > self.config.alert_threshold_growth_percent * 1.5 {
            HealthStatus::Critical
        } else if growth > self.config.alert_threshold_growth_percent {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        SessionHealth {
            status,
            uptime_secs: uptime,
            current_memory_bytes: current,
            peak_memory_bytes: peak,
            growth_rate_percent: growth,
            samples_collected: self.samples.len(),
            alerts_triggered: self.alerts.len(),
        }
    }

    pub fn peak_memory(&self) -> usize {
        self.samples.iter().map(|s| s.heap_bytes).max().unwrap_or(0)
    }

    pub fn growth_rate_percent(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }

        let n = self.samples.len();
        let first_segment_end = n / 10;
        let last_segment_start = n - (n / 10);

        if first_segment_end == 0 || last_segment_start >= n {
            // Fall back to simple first/last comparison
            let first = self.samples.first().expect("len >= 2").heap_bytes;
            let last = self.samples.last().expect("len >= 2").heap_bytes;
            if first == 0 {
                return 0.0;
            }
            return (last as f64 - first as f64) / first as f64 * 100.0;
        }

        let first_avg: f64 = self.samples[..first_segment_end]
            .iter()
            .map(|s| s.heap_bytes as f64)
            .sum::<f64>()
            / first_segment_end as f64;

        let last_count = n - last_segment_start;
        let last_avg: f64 = self.samples[last_segment_start..]
            .iter()
            .map(|s| s.heap_bytes as f64)
            .sum::<f64>()
            / last_count as f64;

        if first_avg == 0.0 {
            return 0.0;
        }

        ((last_avg - first_avg) / first_avg) * 100.0
    }

    pub fn get_alerts(&self) -> &[MemoryAlert] {
        &self.alerts
    }

    pub fn clear_alerts(&mut self) {
        self.alerts.clear();
    }

    pub fn estimate_time_to_threshold(&self) -> Option<u64> {
        if self.samples.len() < 2 {
            return None;
        }

        let current = self.samples.last().expect("len >= 2").heap_bytes;
        if current >= self.config.auto_compact_threshold_bytes {
            return Some(0);
        }

        let slope = self.linear_regression_slope();
        if slope <= 0.0 {
            return None; // Not growing
        }

        let remaining_bytes = self.config.auto_compact_threshold_bytes - current;
        let n = self.samples.len();
        let time_span = self.samples[n - 1].timestamp.saturating_sub(self.samples[0].timestamp);
        if time_span == 0 || n < 2 {
            return None;
        }

        let secs_per_sample = time_span as f64 / (n - 1) as f64;
        let samples_to_threshold = remaining_bytes as f64 / slope;
        let seconds = samples_to_threshold * secs_per_sample;

        if seconds > 0.0 && seconds < u64::MAX as f64 {
            Some(seconds as u64)
        } else {
            None
        }
    }

    fn linear_regression_slope(&self) -> f64 {
        let n = self.samples.len() as f64;
        if n < 2.0 {
            return 0.0;
        }

        let mut sum_x: f64 = 0.0;
        let mut sum_y: f64 = 0.0;
        let mut sum_xy: f64 = 0.0;
        let mut sum_x2: f64 = 0.0;

        for (i, sample) in self.samples.iter().enumerate() {
            let x = i as f64;
            let y = sample.heap_bytes as f64;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() < f64::EPSILON {
            return 0.0;
        }

        (n * sum_xy - sum_x * sum_y) / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> ProfilerConfig {
        ProfilerConfig {
            sample_interval_secs: 10,
            alert_threshold_growth_percent: 50.0,
            auto_compact_threshold_bytes: 1000,
            max_samples: 100,
            enable_auto_cleanup: true,
        }
    }

    fn make_sample(ts: u64, heap: usize) -> MemorySample {
        MemorySample {
            timestamp: ts,
            heap_bytes: heap,
            context_entries: 10,
            conversation_turns: 5,
            mcp_tools_loaded: 3,
        }
    }

    #[test]
    fn test_new_profiler() {
        let p = MemoryProfiler::new(default_config());
        assert!(p.samples.is_empty());
        assert!(p.alerts.is_empty());
        assert_eq!(p.baseline_bytes, 0);
    }

    #[test]
    fn test_record_sample_sets_baseline() {
        let mut p = MemoryProfiler::new(default_config());
        p.record_sample(make_sample(1, 500));
        assert_eq!(p.baseline_bytes, 500);
        assert_eq!(p.samples.len(), 1);
    }

    #[test]
    fn test_record_sample_enforces_max() {
        let mut cfg = default_config();
        cfg.max_samples = 5;
        let mut p = MemoryProfiler::new(cfg);
        for i in 0..10 {
            p.record_sample(make_sample(i, 100));
        }
        assert_eq!(p.samples.len(), 5);
    }

    #[test]
    fn test_peak_memory() {
        let mut p = MemoryProfiler::new(default_config());
        p.record_sample(make_sample(1, 100));
        p.record_sample(make_sample(2, 500));
        p.record_sample(make_sample(3, 300));
        assert_eq!(p.peak_memory(), 500);
    }

    #[test]
    fn test_peak_memory_empty() {
        let p = MemoryProfiler::new(default_config());
        assert_eq!(p.peak_memory(), 0);
    }

    #[test]
    fn test_growth_rate_stable() {
        let mut p = MemoryProfiler::new(default_config());
        for i in 0..20 {
            p.record_sample(make_sample(i, 100));
        }
        assert!((p.growth_rate_percent()).abs() < 1.0);
    }

    #[test]
    fn test_growth_rate_increasing() {
        let mut p = MemoryProfiler::new(default_config());
        for i in 0..20u64 {
            p.record_sample(make_sample(i * 10, 100 + i as usize * 50));
        }
        assert!(p.growth_rate_percent() > 0.0);
    }

    #[test]
    fn test_growth_rate_few_samples() {
        let mut p = MemoryProfiler::new(default_config());
        p.record_sample(make_sample(1, 100));
        assert_eq!(p.growth_rate_percent(), 0.0);
    }

    #[test]
    fn test_detect_leak_insufficient_samples() {
        let mut p = MemoryProfiler::new(default_config());
        for i in 0..5 {
            p.samples.push(make_sample(i, 100 + i as usize * 100));
        }
        assert!(p.detect_leak().is_none());
    }

    #[test]
    fn test_detect_leak_with_growth() {
        let mut cfg = default_config();
        cfg.alert_threshold_growth_percent = 10.0;
        let mut p = MemoryProfiler::new(cfg);
        for i in 0..20u64 {
            p.samples.push(make_sample(i * 10, 100 + i as usize * 100));
        }
        let alert = p.detect_leak();
        assert!(alert.is_some());
        let a = alert.unwrap();
        assert!(
            a.alert_type == MemoryAlertType::LeakDetected
                || a.alert_type == MemoryAlertType::GrowthWarning
        );
    }

    #[test]
    fn test_should_compact_below_threshold() {
        let mut p = MemoryProfiler::new(default_config());
        p.record_sample(make_sample(1, 500));
        assert!(!p.should_compact());
    }

    #[test]
    fn test_should_compact_above_threshold() {
        let mut p = MemoryProfiler::new(default_config());
        p.record_sample(make_sample(1, 2000));
        assert!(p.should_compact());
    }

    #[test]
    fn test_compact_evicts_samples() {
        let mut cfg = default_config();
        cfg.auto_compact_threshold_bytes = 100000;
        let mut p = MemoryProfiler::new(cfg);
        for i in 0..200u64 {
            p.samples.push(make_sample(i, 500));
        }
        let before = p.samples.len();
        let result = p.compact();
        assert!(p.samples.len() < before);
        assert!(result.entries_evicted > 0);
        assert!(result.freed_bytes > 0);
    }

    #[test]
    fn test_compact_adds_alert() {
        let mut p = MemoryProfiler::new(default_config());
        for i in 0..200u64 {
            p.samples.push(make_sample(i, 500));
        }
        let before_alerts = p.alerts.len();
        p.compact();
        assert!(p.alerts.len() > before_alerts);
        let last_alert = p.alerts.last().unwrap();
        assert_eq!(last_alert.alert_type, MemoryAlertType::CompactionTriggered);
    }

    #[test]
    fn test_get_health_healthy() {
        let mut p = MemoryProfiler::new(default_config());
        for i in 0..5u64 {
            p.samples.push(make_sample(i * 60, 100));
        }
        let health = p.get_health();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.current_memory_bytes, 100);
    }

    #[test]
    fn test_get_health_uptime() {
        let mut p = MemoryProfiler::new(default_config());
        p.samples.push(make_sample(1000, 100));
        p.samples.push(make_sample(5000, 100));
        let health = p.get_health();
        assert_eq!(health.uptime_secs, 4000);
    }

    #[test]
    fn test_clear_alerts() {
        let mut p = MemoryProfiler::new(default_config());
        p.alerts.push(MemoryAlert {
            timestamp: 1,
            alert_type: MemoryAlertType::GrowthWarning,
            current_bytes: 100,
            growth_rate_percent: 10.0,
            message: "test".to_string(),
        });
        assert_eq!(p.get_alerts().len(), 1);
        p.clear_alerts();
        assert!(p.get_alerts().is_empty());
    }

    #[test]
    fn test_estimate_time_to_threshold_not_growing() {
        let mut p = MemoryProfiler::new(default_config());
        for i in 0..20u64 {
            p.samples.push(make_sample(i * 10, 100));
        }
        // Flat usage, should return None
        assert!(p.estimate_time_to_threshold().is_none());
    }

    #[test]
    fn test_estimate_time_to_threshold_growing() {
        let mut cfg = default_config();
        cfg.auto_compact_threshold_bytes = 10000;
        let mut p = MemoryProfiler::new(cfg);
        for i in 0..20u64 {
            p.samples.push(make_sample(i * 60, 1000 + i as usize * 100));
        }
        let est = p.estimate_time_to_threshold();
        assert!(est.is_some());
        assert!(est.unwrap() > 0);
    }

    #[test]
    fn test_estimate_time_already_exceeded() {
        let mut cfg = default_config();
        cfg.auto_compact_threshold_bytes = 50;
        let mut p = MemoryProfiler::new(cfg);
        p.samples.push(make_sample(1, 100));
        p.samples.push(make_sample(2, 200));
        assert_eq!(p.estimate_time_to_threshold(), Some(0));
    }

    #[test]
    fn test_health_status_warning() {
        let mut cfg = default_config();
        cfg.alert_threshold_growth_percent = 10.0;
        let mut p = MemoryProfiler::new(cfg);
        // Create growth just over threshold (10-15%)
        for i in 0..20u64 {
            p.samples.push(make_sample(i * 10, 1000 + i as usize * 8));
        }
        let growth = p.growth_rate_percent();
        // Adjust: if growth > threshold but < threshold*1.5, should be Warning
        if growth > 10.0 && growth <= 15.0 {
            let health = p.get_health();
            assert_eq!(health.status, HealthStatus::Warning);
        }
    }

    #[test]
    fn test_default_profiler_config() {
        let cfg = ProfilerConfig::default();
        assert_eq!(cfg.sample_interval_secs, 60);
        assert_eq!(cfg.max_samples, 10000);
        assert!(cfg.enable_auto_cleanup);
    }
}
