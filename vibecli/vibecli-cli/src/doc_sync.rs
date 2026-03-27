//! Living documentation sync engine.
//!
//! Tracks bidirectional links between specification sections and code files,
//! detects drift when code changes without corresponding spec updates, generates
//! alerts and reconciliation actions, and reports freshness metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How a spec section relates to code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkType {
    Implements,
    Tests,
    Documents,
    Configures,
    DependsOn,
}

/// Type of source-code change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed(String),
}

/// Kind of reconciliation action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncActionType {
    UpdateSpec,
    GenerateTask,
    MarkStale,
    RequestReview,
    AutoReconcile,
}

/// A bidirectional link between a spec section and one or more code files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecLink {
    pub spec_section: String,
    pub code_files: Vec<String>,
    pub link_type: LinkType,
    pub last_synced: u64,
    pub drift_score: f64,
}

/// A section of a specification document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecSection {
    pub id: String,
    pub title: String,
    pub content: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub linked_code: Vec<String>,
    pub freshness: f64,
}

/// A recorded change in a code file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeChange {
    pub file_path: String,
    pub change_type: ChangeType,
    pub lines_affected: Vec<usize>,
    pub timestamp: u64,
    pub description: Option<String>,
}

/// An alert raised when drift exceeds a threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriftAlert {
    pub id: String,
    pub spec_section_id: String,
    pub code_file: String,
    pub drift_score: f64,
    pub threshold: f64,
    pub message: String,
    pub created_at: u64,
    pub resolved: bool,
}

/// A suggested or auto-applicable reconciliation action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncAction {
    pub id: String,
    pub action_type: SyncActionType,
    pub spec_section_id: String,
    pub description: String,
    pub auto_applicable: bool,
    pub applied: bool,
}

/// Configuration for the sync engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncConfig {
    pub drift_threshold: f64,
    pub auto_reconcile: bool,
    pub watch_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub freshness_decay_per_day: f64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            drift_threshold: 20.0,
            auto_reconcile: false,
            watch_patterns: Vec::new(),
            ignore_patterns: Vec::new(),
            freshness_decay_per_day: 5.0,
        }
    }
}

/// Per-section freshness entry in a report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionFreshness {
    pub section_id: String,
    pub title: String,
    pub freshness: f64,
    pub last_synced: u64,
    pub linked_files_changed: u32,
}

/// Aggregate freshness report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreshnessReport {
    pub total_sections: usize,
    pub avg_freshness: f64,
    pub stale_count: usize,
    pub fresh_count: usize,
    pub sections: Vec<SectionFreshness>,
}

/// Aggregate metrics for the sync engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncMetrics {
    pub total_links: usize,
    pub total_changes_recorded: usize,
    pub total_alerts: usize,
    pub total_actions: usize,
    pub total_reconciled: usize,
    pub avg_drift: f64,
}

impl Default for SyncMetrics {
    fn default() -> Self {
        Self {
            total_links: 0,
            total_changes_recorded: 0,
            total_alerts: 0,
            total_actions: 0,
            total_reconciled: 0,
            avg_drift: 0.0,
        }
    }
}

/// Core engine that tracks spec-code links, detects drift, and generates actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSyncEngine {
    pub links: Vec<SpecLink>,
    pub sections: HashMap<String, SpecSection>,
    pub changes: Vec<CodeChange>,
    pub alerts: Vec<DriftAlert>,
    pub actions: Vec<SyncAction>,
    pub config: SyncConfig,
    pub metrics: SyncMetrics,
}

impl Default for DocSyncEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DocSyncEngine {
    /// Create a new engine with default configuration.
    pub fn new() -> Self {
        Self {
            links: Vec::new(),
            sections: HashMap::new(),
            changes: Vec::new(),
            alerts: Vec::new(),
            actions: Vec::new(),
            config: SyncConfig::default(),
            metrics: SyncMetrics::default(),
        }
    }

    /// Add a spec section. Returns error if id is empty or already exists.
    pub fn add_spec_section(&mut self, section: SpecSection) -> Result<(), String> {
        if section.id.is_empty() {
            return Err("Spec section id must not be empty".to_string());
        }
        if self.sections.contains_key(&section.id) {
            return Err(format!("Spec section '{}' already exists", section.id));
        }
        self.sections.insert(section.id.clone(), section);
        Ok(())
    }

    /// Remove a spec section by id, also removing related links. Returns error if not found.
    pub fn remove_spec_section(&mut self, id: &str) -> Result<SpecSection, String> {
        let section = self
            .sections
            .remove(id)
            .ok_or_else(|| format!("Spec section '{}' not found", id))?;
        self.links.retain(|l| l.spec_section != id);
        Ok(section)
    }

    /// Add a link between a spec section and code files.
    pub fn add_link(&mut self, link: SpecLink) -> Result<(), String> {
        if link.spec_section.is_empty() {
            return Err("Link spec_section must not be empty".to_string());
        }
        if link.code_files.is_empty() {
            return Err("Link must reference at least one code file".to_string());
        }
        self.links.push(link);
        self.metrics.total_links = self.links.len();
        Ok(())
    }

    /// Record a code change event.
    pub fn record_code_change(&mut self, change: CodeChange) -> Result<(), String> {
        if change.file_path.is_empty() {
            return Err("Code change file_path must not be empty".to_string());
        }
        self.changes.push(change);
        self.metrics.total_changes_recorded = self.changes.len();
        Ok(())
    }

    /// Check drift for all links by comparing most-recent change timestamps against
    /// the link's last_synced. Updates drift_score on each link and decays section
    /// freshness accordingly. Returns the number of links whose drift exceeds the
    /// configured threshold.
    pub fn check_drift(&mut self) -> usize {
        let mut drifted = 0usize;
        for link in &mut self.links {
            let latest_change_ts = self
                .changes
                .iter()
                .filter(|c| link.code_files.contains(&c.file_path))
                .map(|c| c.timestamp)
                .max()
                .unwrap_or(0);

            if latest_change_ts > link.last_synced {
                let diff = latest_change_ts - link.last_synced;
                // Each "day" (86400s) adds freshness_decay_per_day drift points.
                let days = diff as f64 / 86400.0;
                let raw = days * self.config.freshness_decay_per_day;
                link.drift_score = raw.min(100.0);
            } else {
                link.drift_score = 0.0;
            }

            if link.drift_score > self.config.drift_threshold {
                drifted += 1;
            }

            // Decay section freshness based on drift.
            if let Some(sec) = self.sections.get_mut(&link.spec_section) {
                sec.freshness = (100.0 - link.drift_score).max(0.0);
            }
        }

        // Recompute avg_drift.
        if self.links.is_empty() {
            self.metrics.avg_drift = 0.0;
        } else {
            let sum: f64 = self.links.iter().map(|l| l.drift_score).sum();
            self.metrics.avg_drift = sum / self.links.len() as f64;
        }

        drifted
    }

    /// Generate drift alerts for every link whose drift_score exceeds the threshold.
    /// Only creates new alerts (avoids duplicating an existing unresolved alert for the
    /// same section + file pair).
    pub fn generate_alerts(&mut self) -> usize {
        let mut new_count = 0usize;
        let threshold = self.config.drift_threshold;
        let mut pending: Vec<DriftAlert> = Vec::new();

        for link in &self.links {
            if link.drift_score <= threshold {
                continue;
            }
            for file in &link.code_files {
                let already_exists = self.alerts.iter().any(|a| {
                    a.spec_section_id == link.spec_section
                        && a.code_file == *file
                        && !a.resolved
                });
                if already_exists {
                    continue;
                }
                let alert_id = format!(
                    "alert-{}-{}-{}",
                    link.spec_section,
                    file.replace('/', "_"),
                    self.alerts.len() + pending.len()
                );
                pending.push(DriftAlert {
                    id: alert_id,
                    spec_section_id: link.spec_section.clone(),
                    code_file: file.clone(),
                    drift_score: link.drift_score,
                    threshold,
                    message: format!(
                        "Drift {:.1} exceeds threshold {:.1} for section '{}' / file '{}'",
                        link.drift_score, threshold, link.spec_section, file
                    ),
                    created_at: link.last_synced,
                    resolved: false,
                });
                new_count += 1;
            }
        }

        self.alerts.extend(pending);
        self.metrics.total_alerts = self.alerts.len();
        new_count
    }

    /// Generate sync actions based on current drift alerts and link state.
    /// Returns the number of new actions created.
    pub fn generate_sync_actions(&mut self) -> usize {
        let mut new_count = 0usize;
        let auto = self.config.auto_reconcile;
        let mut pending: Vec<SyncAction> = Vec::new();

        for alert in &self.alerts {
            if alert.resolved {
                continue;
            }
            let already = self.actions.iter().any(|a| {
                a.spec_section_id == alert.spec_section_id && !a.applied
            });
            if already {
                continue;
            }

            let (action_type, desc, auto_applicable) = if alert.drift_score >= 80.0 {
                (
                    SyncActionType::MarkStale,
                    format!("Mark section '{}' as stale (drift {:.1})", alert.spec_section_id, alert.drift_score),
                    auto,
                )
            } else if alert.drift_score >= 50.0 {
                (
                    SyncActionType::RequestReview,
                    format!("Request review for section '{}' (drift {:.1})", alert.spec_section_id, alert.drift_score),
                    false,
                )
            } else {
                (
                    SyncActionType::UpdateSpec,
                    format!("Update spec section '{}' (drift {:.1})", alert.spec_section_id, alert.drift_score),
                    auto,
                )
            };

            let action_id = format!("action-{}-{}", alert.spec_section_id, self.actions.len() + pending.len());
            pending.push(SyncAction {
                id: action_id,
                action_type,
                spec_section_id: alert.spec_section_id.clone(),
                description: desc,
                auto_applicable,
                applied: false,
            });
            new_count += 1;
        }

        self.actions.extend(pending);
        self.metrics.total_actions = self.actions.len();
        new_count
    }

    /// Apply all auto-applicable actions: mark them as applied, resolve matching
    /// alerts, and reset drift/freshness. Returns the count of actions applied.
    pub fn reconcile(&mut self) -> usize {
        let mut applied = 0usize;
        let mut reconciled_sections: Vec<String> = Vec::new();

        for action in &mut self.actions {
            if action.auto_applicable && !action.applied {
                action.applied = true;
                applied += 1;
                reconciled_sections.push(action.spec_section_id.clone());
            }
        }

        // Resolve alerts whose section was reconciled.
        for alert in &mut self.alerts {
            if reconciled_sections.contains(&alert.spec_section_id) && !alert.resolved {
                alert.resolved = true;
            }
        }

        // Reset drift on reconciled links and restore freshness.
        for link in &mut self.links {
            if reconciled_sections.contains(&link.spec_section) {
                link.drift_score = 0.0;
                // Bump last_synced to latest change timestamp for those files.
                let latest = self
                    .changes
                    .iter()
                    .filter(|c| link.code_files.contains(&c.file_path))
                    .map(|c| c.timestamp)
                    .max()
                    .unwrap_or(link.last_synced);
                link.last_synced = latest;
            }
        }
        for sec_id in &reconciled_sections {
            if let Some(sec) = self.sections.get_mut(sec_id) {
                sec.freshness = 100.0;
            }
        }

        self.metrics.total_reconciled += applied;
        applied
    }

    /// Build a freshness report across all tracked sections.
    pub fn get_freshness_report(&self) -> FreshnessReport {
        let mut entries: Vec<SectionFreshness> = Vec::new();

        for (id, sec) in &self.sections {
            let last_synced = self
                .links
                .iter()
                .filter(|l| l.spec_section == *id)
                .map(|l| l.last_synced)
                .max()
                .unwrap_or(0);

            let changed_count = self
                .links
                .iter()
                .filter(|l| l.spec_section == *id)
                .flat_map(|l| &l.code_files)
                .filter(|f| self.changes.iter().any(|c| c.file_path == **f && c.timestamp > last_synced))
                .count() as u32;

            entries.push(SectionFreshness {
                section_id: id.clone(),
                title: sec.title.clone(),
                freshness: sec.freshness,
                last_synced,
                linked_files_changed: changed_count,
            });
        }

        let total = entries.len();
        let avg = if total == 0 {
            0.0
        } else {
            entries.iter().map(|e| e.freshness).sum::<f64>() / total as f64
        };
        let stale = entries.iter().filter(|e| e.freshness < 50.0).count();
        let fresh = entries.iter().filter(|e| e.freshness >= 50.0).count();

        FreshnessReport {
            total_sections: total,
            avg_freshness: avg,
            stale_count: stale,
            fresh_count: fresh,
            sections: entries,
        }
    }

    /// Mark a drift alert as resolved by id.
    pub fn mark_resolved(&mut self, alert_id: &str) -> Result<(), String> {
        let alert = self
            .alerts
            .iter_mut()
            .find(|a| a.id == alert_id)
            .ok_or_else(|| format!("Alert '{}' not found", alert_id))?;
        alert.resolved = true;
        Ok(())
    }

    /// Return all sections whose freshness is below the given threshold.
    pub fn get_stale_sections(&self, threshold: f64) -> Vec<&SpecSection> {
        self.sections
            .values()
            .filter(|s| s.freshness < threshold)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_section(id: &str) -> SpecSection {
        SpecSection {
            id: id.to_string(),
            title: format!("Section {}", id),
            content: "lorem ipsum".to_string(),
            file_path: format!("docs/{}.md", id),
            line_start: 1,
            line_end: 10,
            linked_code: Vec::new(),
            freshness: 100.0,
        }
    }

    fn make_link(section: &str, files: Vec<&str>, ts: u64) -> SpecLink {
        SpecLink {
            spec_section: section.to_string(),
            code_files: files.into_iter().map(String::from).collect(),
            link_type: LinkType::Implements,
            last_synced: ts,
            drift_score: 0.0,
        }
    }

    fn make_change(file: &str, ts: u64) -> CodeChange {
        CodeChange {
            file_path: file.to_string(),
            change_type: ChangeType::Modified,
            lines_affected: vec![1, 2, 3],
            timestamp: ts,
            description: None,
        }
    }

    // --- Engine creation ---

    #[test]
    fn test_new_engine() {
        let e = DocSyncEngine::new();
        assert!(e.links.is_empty());
        assert!(e.sections.is_empty());
        assert!(e.changes.is_empty());
        assert!(e.alerts.is_empty());
        assert!(e.actions.is_empty());
        assert_eq!(e.config.drift_threshold, 20.0);
        assert_eq!(e.metrics.total_links, 0);
    }

    // --- Spec section management ---

    #[test]
    fn test_add_spec_section() {
        let mut e = DocSyncEngine::new();
        assert!(e.add_spec_section(make_section("s1")).is_ok());
        assert_eq!(e.sections.len(), 1);
    }

    #[test]
    fn test_add_duplicate_section() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        let err = e.add_spec_section(make_section("s1")).unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[test]
    fn test_add_empty_id_section() {
        let mut e = DocSyncEngine::new();
        let mut s = make_section("x");
        s.id = String::new();
        assert!(e.add_spec_section(s).unwrap_err().contains("must not be empty"));
    }

    #[test]
    fn test_remove_spec_section() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        let removed = e.remove_spec_section("s1").unwrap();
        assert_eq!(removed.id, "s1");
        assert!(e.sections.is_empty());
        assert!(e.links.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_section() {
        let mut e = DocSyncEngine::new();
        assert!(e.remove_spec_section("nope").is_err());
    }

    // --- Link creation ---

    #[test]
    fn test_add_link() {
        let mut e = DocSyncEngine::new();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        assert_eq!(e.links.len(), 1);
        assert_eq!(e.metrics.total_links, 1);
    }

    #[test]
    fn test_add_link_empty_section() {
        let mut e = DocSyncEngine::new();
        let mut l = make_link("s1", vec!["a.rs"], 100);
        l.spec_section = String::new();
        assert!(e.add_link(l).is_err());
    }

    #[test]
    fn test_add_link_no_files() {
        let mut e = DocSyncEngine::new();
        let l = make_link("s1", vec![], 100);
        assert!(e.add_link(l).unwrap_err().contains("at least one code file"));
    }

    // --- Code change recording ---

    #[test]
    fn test_record_code_change() {
        let mut e = DocSyncEngine::new();
        e.record_code_change(make_change("a.rs", 200)).unwrap();
        assert_eq!(e.changes.len(), 1);
        assert_eq!(e.metrics.total_changes_recorded, 1);
    }

    #[test]
    fn test_record_change_empty_path() {
        let mut e = DocSyncEngine::new();
        let mut c = make_change("a.rs", 200);
        c.file_path = String::new();
        assert!(e.record_code_change(c).is_err());
    }

    // --- Drift detection ---

    #[test]
    fn test_check_drift_no_links() {
        let mut e = DocSyncEngine::new();
        assert_eq!(e.check_drift(), 0);
        assert_eq!(e.metrics.avg_drift, 0.0);
    }

    #[test]
    fn test_check_drift_no_changes() {
        let mut e = DocSyncEngine::new();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        assert_eq!(e.check_drift(), 0);
        assert_eq!(e.links[0].drift_score, 0.0);
    }

    #[test]
    fn test_check_drift_with_change() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        // Change 2 days after last sync.
        e.record_code_change(make_change("a.rs", 100 + 86400 * 2)).unwrap();
        let drifted = e.check_drift();
        // drift = 2 * 5.0 = 10.0, below default threshold 20.0
        assert_eq!(drifted, 0);
        assert!((e.links[0].drift_score - 10.0).abs() < 0.01);
        assert!((e.sections["s1"].freshness - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_check_drift_exceeds_threshold() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        // 5 days => drift 25.0 > threshold 20.0
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        assert_eq!(e.check_drift(), 1);
    }

    #[test]
    fn test_drift_capped_at_100() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 0)).unwrap();
        // 30 days => 150.0, capped at 100
        e.record_code_change(make_change("a.rs", 86400 * 30)).unwrap();
        e.check_drift();
        assert_eq!(e.links[0].drift_score, 100.0);
        assert_eq!(e.sections["s1"].freshness, 0.0);
    }

    #[test]
    fn test_avg_drift_metric() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_spec_section(make_section("s2")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.add_link(make_link("s2", vec!["b.rs"], 100)).unwrap();
        // s1: 2 days drift=10, s2: no change drift=0
        e.record_code_change(make_change("a.rs", 100 + 86400 * 2)).unwrap();
        e.check_drift();
        assert!((e.metrics.avg_drift - 5.0).abs() < 0.01);
    }

    // --- Alert generation ---

    #[test]
    fn test_generate_alerts_none_when_no_drift() {
        let mut e = DocSyncEngine::new();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.check_drift();
        assert_eq!(e.generate_alerts(), 0);
    }

    #[test]
    fn test_generate_alerts_creates_alert() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        assert_eq!(e.generate_alerts(), 1);
        assert_eq!(e.alerts.len(), 1);
        assert!(!e.alerts[0].resolved);
        assert_eq!(e.metrics.total_alerts, 1);
    }

    #[test]
    fn test_no_duplicate_alerts() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        e.generate_alerts();
        // Call again — should not duplicate.
        assert_eq!(e.generate_alerts(), 0);
        assert_eq!(e.alerts.len(), 1);
    }

    #[test]
    fn test_alert_after_resolved_creates_new() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        e.generate_alerts();
        e.mark_resolved(&e.alerts[0].id.clone()).unwrap();
        // Now a second generate should create a new alert.
        assert_eq!(e.generate_alerts(), 1);
        assert_eq!(e.alerts.len(), 2);
    }

    // --- Sync action generation ---

    #[test]
    fn test_generate_sync_actions_update_spec() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        // drift ~25 => UpdateSpec action
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        e.generate_alerts();
        assert_eq!(e.generate_sync_actions(), 1);
        assert_eq!(e.actions[0].action_type, SyncActionType::UpdateSpec);
    }

    #[test]
    fn test_generate_sync_actions_request_review() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        // 11 days => drift 55 => RequestReview
        e.record_code_change(make_change("a.rs", 100 + 86400 * 11)).unwrap();
        e.check_drift();
        e.generate_alerts();
        e.generate_sync_actions();
        assert_eq!(e.actions[0].action_type, SyncActionType::RequestReview);
        assert!(!e.actions[0].auto_applicable);
    }

    #[test]
    fn test_generate_sync_actions_mark_stale() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        // 17 days => drift 85 => MarkStale
        e.record_code_change(make_change("a.rs", 100 + 86400 * 17)).unwrap();
        e.check_drift();
        e.generate_alerts();
        e.generate_sync_actions();
        assert_eq!(e.actions[0].action_type, SyncActionType::MarkStale);
    }

    #[test]
    fn test_no_duplicate_actions() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        e.generate_alerts();
        e.generate_sync_actions();
        assert_eq!(e.generate_sync_actions(), 0);
    }

    // --- Reconciliation ---

    #[test]
    fn test_reconcile_auto() {
        let mut e = DocSyncEngine::new();
        e.config.auto_reconcile = true;
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        e.generate_alerts();
        e.generate_sync_actions();
        let applied = e.reconcile();
        assert_eq!(applied, 1);
        assert!(e.actions[0].applied);
        assert!(e.alerts[0].resolved);
        assert_eq!(e.links[0].drift_score, 0.0);
        assert_eq!(e.sections["s1"].freshness, 100.0);
        assert_eq!(e.metrics.total_reconciled, 1);
    }

    #[test]
    fn test_reconcile_skips_non_auto() {
        let mut e = DocSyncEngine::new();
        e.config.auto_reconcile = false;
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        e.generate_alerts();
        e.generate_sync_actions();
        assert_eq!(e.reconcile(), 0);
    }

    // --- Mark resolved ---

    #[test]
    fn test_mark_resolved() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        e.generate_alerts();
        let id = e.alerts[0].id.clone();
        e.mark_resolved(&id).unwrap();
        assert!(e.alerts[0].resolved);
    }

    #[test]
    fn test_mark_resolved_not_found() {
        let mut e = DocSyncEngine::new();
        assert!(e.mark_resolved("nope").is_err());
    }

    // --- Freshness report ---

    #[test]
    fn test_freshness_report_empty() {
        let e = DocSyncEngine::new();
        let r = e.get_freshness_report();
        assert_eq!(r.total_sections, 0);
        assert_eq!(r.avg_freshness, 0.0);
        assert_eq!(r.stale_count, 0);
        assert_eq!(r.fresh_count, 0);
    }

    #[test]
    fn test_freshness_report_all_fresh() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_spec_section(make_section("s2")).unwrap();
        let r = e.get_freshness_report();
        assert_eq!(r.total_sections, 2);
        assert_eq!(r.avg_freshness, 100.0);
        assert_eq!(r.fresh_count, 2);
        assert_eq!(r.stale_count, 0);
    }

    #[test]
    fn test_freshness_report_with_drift() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 15)).unwrap();
        e.check_drift();
        let r = e.get_freshness_report();
        assert_eq!(r.total_sections, 1);
        assert!(r.avg_freshness < 50.0);
        assert_eq!(r.stale_count, 1);
    }

    // --- Stale section detection ---

    #[test]
    fn test_get_stale_sections_none() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        assert!(e.get_stale_sections(50.0).is_empty());
    }

    #[test]
    fn test_get_stale_sections_found() {
        let mut e = DocSyncEngine::new();
        let mut s = make_section("s1");
        s.freshness = 30.0;
        e.add_spec_section(s).unwrap();
        let stale = e.get_stale_sections(50.0);
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].id, "s1");
    }

    // --- Config defaults ---

    #[test]
    fn test_sync_config_defaults() {
        let c = SyncConfig::default();
        assert_eq!(c.drift_threshold, 20.0);
        assert!(!c.auto_reconcile);
        assert!(c.watch_patterns.is_empty());
        assert!(c.ignore_patterns.is_empty());
        assert_eq!(c.freshness_decay_per_day, 5.0);
    }

    // --- Enum variants ---

    #[test]
    fn test_link_type_variants() {
        let types = vec![
            LinkType::Implements,
            LinkType::Tests,
            LinkType::Documents,
            LinkType::Configures,
            LinkType::DependsOn,
        ];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_change_type_renamed() {
        let ct = ChangeType::Renamed("old.rs".to_string());
        if let ChangeType::Renamed(old) = &ct {
            assert_eq!(old, "old.rs");
        } else {
            panic!("expected Renamed");
        }
    }

    #[test]
    fn test_sync_action_type_variants() {
        let types = vec![
            SyncActionType::UpdateSpec,
            SyncActionType::GenerateTask,
            SyncActionType::MarkStale,
            SyncActionType::RequestReview,
            SyncActionType::AutoReconcile,
        ];
        assert_eq!(types.len(), 5);
    }

    // --- Metrics ---

    #[test]
    fn test_metrics_default() {
        let m = SyncMetrics::default();
        assert_eq!(m.total_links, 0);
        assert_eq!(m.total_changes_recorded, 0);
        assert_eq!(m.total_alerts, 0);
        assert_eq!(m.total_actions, 0);
        assert_eq!(m.total_reconciled, 0);
        assert_eq!(m.avg_drift, 0.0);
    }

    #[test]
    fn test_metrics_after_operations() {
        let mut e = DocSyncEngine::new();
        e.config.auto_reconcile = true;
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("a.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        e.generate_alerts();
        e.generate_sync_actions();
        e.reconcile();
        assert_eq!(e.metrics.total_links, 1);
        assert_eq!(e.metrics.total_changes_recorded, 1);
        assert_eq!(e.metrics.total_alerts, 1);
        assert_eq!(e.metrics.total_actions, 1);
        assert_eq!(e.metrics.total_reconciled, 1);
    }

    // --- Edge cases ---

    #[test]
    fn test_multiple_files_per_link() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs", "b.rs"], 100)).unwrap();
        e.record_code_change(make_change("b.rs", 100 + 86400 * 5)).unwrap();
        e.check_drift();
        assert!(e.links[0].drift_score > 0.0);
    }

    #[test]
    fn test_change_to_unlinked_file_no_drift() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        e.record_code_change(make_change("unrelated.rs", 100 + 86400 * 50)).unwrap();
        assert_eq!(e.check_drift(), 0);
        assert_eq!(e.links[0].drift_score, 0.0);
    }

    #[test]
    fn test_all_stale() {
        let mut e = DocSyncEngine::new();
        for i in 0..3 {
            let mut s = make_section(&format!("s{}", i));
            s.freshness = 10.0;
            e.add_spec_section(s).unwrap();
        }
        assert_eq!(e.get_stale_sections(50.0).len(), 3);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut e = DocSyncEngine::new();
        e.add_spec_section(make_section("s1")).unwrap();
        e.add_link(make_link("s1", vec!["a.rs"], 100)).unwrap();
        let json = serde_json::to_string(&e).expect("serialize");
        let e2: DocSyncEngine = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e2.sections.len(), 1);
        assert_eq!(e2.links.len(), 1);
    }
}
