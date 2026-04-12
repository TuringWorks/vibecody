//! Team onboarding — usage tracking, gap detection, learning paths, and guide generation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── UsageRecord ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub user_id: String,
    pub command: String,
    pub panel: Option<String>,
    pub timestamp_ms: u64,
    pub success: bool,
}

// ─── UsagePattern ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub user_id: String,
    pub total_sessions: u32,
    pub commands_used: Vec<String>,
    pub panels_used: Vec<String>,
    pub error_rate: f32,
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
}

/// Heuristic for identifying new/struggling team members.
pub fn is_likely_new_member(pattern: &UsagePattern) -> bool {
    pattern.total_sessions < 5
        || (pattern.commands_used.len() < 3 && pattern.error_rate > 0.3)
}

// ─── KnowledgeGap ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub feature_name: String,
    pub description: String,
    /// 0-100 scale.
    pub impact_score: u8,
    pub category: String,
}

// ─── GapReport ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapReport {
    pub user_id: String,
    pub gaps: Vec<KnowledgeGap>,
    pub generated_at_ms: u64,
}

impl GapReport {
    /// Returns the top `n` gaps sorted by impact_score descending.
    pub fn top_gaps(&self, n: usize) -> Vec<&KnowledgeGap> {
        let mut sorted: Vec<&KnowledgeGap> = self.gaps.iter().collect();
        sorted.sort_by(|a, b| b.impact_score.cmp(&a.impact_score));
        sorted.truncate(n);
        sorted
    }
}

// ─── LearningCheckpoint ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningCheckpoint {
    pub checkpoint_id: String,
    pub description: String,
    pub completed: bool,
}

// ─── LearningPath ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPath {
    pub user_id: String,
    pub steps: Vec<LearningCheckpoint>,
}

impl LearningPath {
    /// Returns completion percentage (0–100). Returns 0 if there are no steps.
    pub fn completion_pct(&self) -> f32 {
        if self.steps.is_empty() {
            return 0.0;
        }
        let completed = self.steps.iter().filter(|s| s.completed).count();
        completed as f32 / self.steps.len() as f32 * 100.0
    }

    /// Returns the first non-completed checkpoint, if any.
    pub fn next_step(&self) -> Option<&LearningCheckpoint> {
        self.steps.iter().find(|s| !s.completed)
    }
}

// ─── HotspotFile ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotFile {
    pub path: String,
    pub access_count: u32,
    pub veteran_access_count: u32,
}

// ─── HotspotMap ──────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct HotspotMap {
    files: Vec<HotspotFile>,
}

impl HotspotMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn record_access(&mut self, path: &str, is_veteran: bool) {
        if let Some(file) = self.files.iter_mut().find(|f| f.path == path) {
            file.access_count += 1;
            if is_veteran {
                file.veteran_access_count += 1;
            }
        } else {
            self.files.push(HotspotFile {
                path: path.to_string(),
                access_count: 1,
                veteran_access_count: if is_veteran { 1 } else { 0 },
            });
        }
    }

    /// Returns the top `n` files by veteran_access_count descending.
    pub fn top_veteran_files(&self, n: usize) -> Vec<&HotspotFile> {
        let mut sorted: Vec<&HotspotFile> = self.files.iter().collect();
        sorted.sort_by(|a, b| b.veteran_access_count.cmp(&a.veteran_access_count));
        sorted.truncate(n);
        sorted
    }

    pub fn all_files(&self) -> &[HotspotFile] {
        &self.files
    }
}

// ─── OnboardingGuide ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingGuide {
    pub user_id: String,
    pub top_features: Vec<String>,
    pub hotspot_files: Vec<String>,
    pub learning_path: LearningPath,
    pub generated_at_ms: u64,
}

impl OnboardingGuide {
    pub fn to_markdown(&self) -> String {
        let mut md = String::from("# Onboarding Guide\n\n");
        md.push_str("## Top Features\n");
        for feature in &self.top_features {
            md.push_str(&format!("- {}\n", feature));
        }
        md.push_str("\n## Hotspot Files\n");
        for file in &self.hotspot_files {
            md.push_str(&format!("- {}\n", file));
        }
        md.push_str("\n## Learning Path\n");
        for (i, step) in self.learning_path.steps.iter().enumerate() {
            let status = if step.completed { "[x]" } else { "[ ]" };
            md.push_str(&format!("{}. {} {}\n", i + 1, status, step.description));
        }
        md
    }
}

// ─── OnboardingEngine ────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct OnboardingEngine {
    /// user_id → list of UsageRecord
    records: HashMap<String, Vec<UsageRecord>>,
}

impl OnboardingEngine {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn record_usage(&mut self, record: UsageRecord) {
        self.records
            .entry(record.user_id.clone())
            .or_default()
            .push(record);
    }

    pub fn build_pattern(&self, user_id: &str) -> Option<UsagePattern> {
        let recs = self.records.get(user_id)?;
        if recs.is_empty() {
            return None;
        }

        let mut commands_used: Vec<String> = Vec::new();
        let mut panels_used: Vec<String> = Vec::new();
        let mut errors = 0u32;
        let mut first_seen = u64::MAX;
        let mut last_seen = 0u64;

        for r in recs {
            if !commands_used.contains(&r.command) {
                commands_used.push(r.command.clone());
            }
            if let Some(p) = &r.panel {
                if !panels_used.contains(p) {
                    panels_used.push(p.clone());
                }
            }
            if !r.success {
                errors += 1;
            }
            if r.timestamp_ms < first_seen {
                first_seen = r.timestamp_ms;
            }
            if r.timestamp_ms > last_seen {
                last_seen = r.timestamp_ms;
            }
        }

        let error_rate = errors as f32 / recs.len() as f32;
        // Approximate sessions: count unique timestamps bucketed by day (or just use record count / 5 as proxy)
        let total_sessions = (recs.len() as u32).max(1);

        Some(UsagePattern {
            user_id: user_id.to_string(),
            total_sessions,
            commands_used,
            panels_used,
            error_rate,
            first_seen_ms: if first_seen == u64::MAX { 0 } else { first_seen },
            last_seen_ms: last_seen,
        })
    }

    pub fn generate_gap_report(&self, user_id: &str, all_features: &[&str]) -> GapReport {
        let pattern = self.build_pattern(user_id);
        let commands_used = pattern
            .as_ref()
            .map(|p| p.commands_used.clone())
            .unwrap_or_default();

        let gaps: Vec<KnowledgeGap> = all_features
            .iter()
            .enumerate()
            .filter(|(_, feature)| !commands_used.iter().any(|c| c == *feature))
            .map(|(i, feature)| {
                // Deterministic impact score: position*7 % 50 + 50
                let impact_score = ((i as u8).wrapping_mul(7) % 50) + 50;
                KnowledgeGap {
                    feature_name: feature.to_string(),
                    description: format!("Feature '{}' has not been used yet", feature),
                    impact_score,
                    category: "unused-feature".to_string(),
                }
            })
            .collect();

        GapReport {
            user_id: user_id.to_string(),
            gaps,
            generated_at_ms: now_ms(),
        }
    }

    pub fn generate_guide(
        &self,
        user_id: &str,
        hotspot_map: &HotspotMap,
    ) -> Option<OnboardingGuide> {
        let pattern = self.build_pattern(user_id)?;

        let top_features: Vec<String> = pattern.commands_used.iter().take(5).cloned().collect();
        let hotspot_files: Vec<String> = hotspot_map
            .top_veteran_files(5)
            .iter()
            .map(|f| f.path.clone())
            .collect();

        let steps: Vec<LearningCheckpoint> = top_features
            .iter()
            .enumerate()
            .map(|(i, f)| LearningCheckpoint {
                checkpoint_id: format!("cp-{}", i),
                description: format!("Learn to use: {}", f),
                completed: false,
            })
            .collect();

        let learning_path = LearningPath {
            user_id: user_id.to_string(),
            steps,
        };

        Some(OnboardingGuide {
            user_id: user_id.to_string(),
            top_features,
            hotspot_files,
            learning_path,
            generated_at_ms: now_ms(),
        })
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(user_id: &str, command: &str, success: bool, ts: u64) -> UsageRecord {
        UsageRecord {
            user_id: user_id.to_string(),
            command: command.to_string(),
            panel: None,
            timestamp_ms: ts,
            success,
        }
    }

    fn make_pattern(sessions: u32, cmds: usize, error_rate: f32) -> UsagePattern {
        UsagePattern {
            user_id: "u1".into(),
            total_sessions: sessions,
            commands_used: (0..cmds).map(|i| format!("cmd{}", i)).collect(),
            panels_used: vec![],
            error_rate,
            first_seen_ms: 0,
            last_seen_ms: 1000,
        }
    }

    // ── is_likely_new_member ──────────────────────────────────────────────

    #[test]
    fn test_new_member_low_sessions() {
        let p = make_pattern(2, 5, 0.1);
        assert!(is_likely_new_member(&p));
    }

    #[test]
    fn test_new_member_4_sessions() {
        let p = make_pattern(4, 10, 0.0);
        assert!(is_likely_new_member(&p));
    }

    #[test]
    fn test_not_new_member_high_sessions() {
        let p = make_pattern(10, 5, 0.05);
        assert!(!is_likely_new_member(&p));
    }

    #[test]
    fn test_new_member_few_cmds_high_error_rate() {
        let p = make_pattern(10, 2, 0.5);
        assert!(is_likely_new_member(&p));
    }

    #[test]
    fn test_not_new_member_few_cmds_low_error_rate() {
        let p = make_pattern(10, 2, 0.1);
        assert!(!is_likely_new_member(&p));
    }

    #[test]
    fn test_boundary_5_sessions_not_new() {
        let p = make_pattern(5, 5, 0.0);
        assert!(!is_likely_new_member(&p));
    }

    // ── LearningPath ──────────────────────────────────────────────────────

    #[test]
    fn test_completion_pct_all_done() {
        let path = LearningPath {
            user_id: "u1".into(),
            steps: vec![
                LearningCheckpoint { checkpoint_id: "c1".into(), description: "step 1".into(), completed: true },
                LearningCheckpoint { checkpoint_id: "c2".into(), description: "step 2".into(), completed: true },
            ],
        };
        assert!((path.completion_pct() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_completion_pct_half_done() {
        let path = LearningPath {
            user_id: "u1".into(),
            steps: vec![
                LearningCheckpoint { checkpoint_id: "c1".into(), description: "step 1".into(), completed: true },
                LearningCheckpoint { checkpoint_id: "c2".into(), description: "step 2".into(), completed: false },
            ],
        };
        assert!((path.completion_pct() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_completion_pct_empty() {
        let path = LearningPath { user_id: "u1".into(), steps: vec![] };
        assert!((path.completion_pct() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_next_step_returns_first_incomplete() {
        let path = LearningPath {
            user_id: "u1".into(),
            steps: vec![
                LearningCheckpoint { checkpoint_id: "c1".into(), description: "done".into(), completed: true },
                LearningCheckpoint { checkpoint_id: "c2".into(), description: "next".into(), completed: false },
            ],
        };
        let next = path.next_step();
        assert!(next.is_some());
        assert_eq!(next.unwrap().checkpoint_id, "c2");
    }

    #[test]
    fn test_next_step_none_when_all_complete() {
        let path = LearningPath {
            user_id: "u1".into(),
            steps: vec![
                LearningCheckpoint { checkpoint_id: "c1".into(), description: "done".into(), completed: true },
            ],
        };
        assert!(path.next_step().is_none());
    }

    #[test]
    fn test_next_step_none_empty_path() {
        let path = LearningPath { user_id: "u1".into(), steps: vec![] };
        assert!(path.next_step().is_none());
    }

    // ── HotspotMap ────────────────────────────────────────────────────────

    #[test]
    fn test_hotspot_map_new_empty() {
        let m = HotspotMap::new();
        assert!(m.all_files().is_empty());
    }

    #[test]
    fn test_hotspot_map_record_access_new_file() {
        let mut m = HotspotMap::new();
        m.record_access("src/main.rs", false);
        assert_eq!(m.all_files().len(), 1);
        assert_eq!(m.all_files()[0].access_count, 1);
    }

    #[test]
    fn test_hotspot_map_record_veteran_access() {
        let mut m = HotspotMap::new();
        m.record_access("src/lib.rs", true);
        assert_eq!(m.all_files()[0].veteran_access_count, 1);
    }

    #[test]
    fn test_hotspot_map_accumulates_accesses() {
        let mut m = HotspotMap::new();
        m.record_access("src/main.rs", false);
        m.record_access("src/main.rs", true);
        m.record_access("src/main.rs", false);
        assert_eq!(m.all_files()[0].access_count, 3);
        assert_eq!(m.all_files()[0].veteran_access_count, 1);
    }

    #[test]
    fn test_hotspot_map_top_veteran_files_sorted() {
        let mut m = HotspotMap::new();
        m.record_access("a.rs", true);
        m.record_access("b.rs", true);
        m.record_access("b.rs", true);
        m.record_access("c.rs", false);
        let top = m.top_veteran_files(2);
        assert_eq!(top[0].path, "b.rs");
        assert_eq!(top[0].veteran_access_count, 2);
    }

    #[test]
    fn test_hotspot_map_top_n_limiting() {
        let mut m = HotspotMap::new();
        for i in 0..10 {
            m.record_access(&format!("file{}.rs", i), true);
        }
        let top = m.top_veteran_files(3);
        assert_eq!(top.len(), 3);
    }

    // ── GapReport ─────────────────────────────────────────────────────────

    #[test]
    fn test_gap_report_top_gaps_sorted() {
        let gaps = vec![
            KnowledgeGap { feature_name: "a".into(), description: "".into(), impact_score: 60, category: "".into() },
            KnowledgeGap { feature_name: "b".into(), description: "".into(), impact_score: 90, category: "".into() },
            KnowledgeGap { feature_name: "c".into(), description: "".into(), impact_score: 75, category: "".into() },
        ];
        let report = GapReport { user_id: "u1".into(), gaps, generated_at_ms: 0 };
        let top = report.top_gaps(2);
        assert_eq!(top[0].feature_name, "b");
        assert_eq!(top[1].feature_name, "c");
    }

    #[test]
    fn test_gap_report_top_n_limiting() {
        let gaps: Vec<KnowledgeGap> = (0..10)
            .map(|i| KnowledgeGap {
                feature_name: format!("f{}", i),
                description: "".into(),
                impact_score: i as u8 * 5 + 50,
                category: "".into(),
            })
            .collect();
        let report = GapReport { user_id: "u1".into(), gaps, generated_at_ms: 0 };
        assert_eq!(report.top_gaps(3).len(), 3);
    }

    // ── OnboardingEngine ──────────────────────────────────────────────────

    #[test]
    fn test_engine_record_and_build_pattern() {
        let mut e = OnboardingEngine::new();
        e.record_usage(make_record("u1", "edit", true, 100));
        e.record_usage(make_record("u1", "review", false, 200));
        let pattern = e.build_pattern("u1");
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert!(p.commands_used.contains(&"edit".to_string()));
        assert!(p.commands_used.contains(&"review".to_string()));
    }

    #[test]
    fn test_engine_pattern_not_found() {
        let e = OnboardingEngine::new();
        assert!(e.build_pattern("unknown").is_none());
    }

    #[test]
    fn test_engine_error_rate_calculation() {
        let mut e = OnboardingEngine::new();
        e.record_usage(make_record("u1", "cmd1", true, 100));
        e.record_usage(make_record("u1", "cmd2", false, 200));
        let p = e.build_pattern("u1").unwrap();
        assert!((p.error_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_engine_gap_report_filters_used_commands() {
        let mut e = OnboardingEngine::new();
        e.record_usage(make_record("u1", "edit", true, 100));
        let all_features = &["edit", "review", "explain"];
        let report = e.generate_gap_report("u1", all_features);
        assert!(!report.gaps.iter().any(|g| g.feature_name == "edit"));
        assert!(report.gaps.iter().any(|g| g.feature_name == "review"));
    }

    #[test]
    fn test_engine_gap_report_user_with_no_records() {
        let e = OnboardingEngine::new();
        let all_features = &["edit", "review"];
        let report = e.generate_gap_report("unknown", all_features);
        // All features are gaps
        assert_eq!(report.gaps.len(), 2);
    }

    #[test]
    fn test_engine_generate_guide_returns_some() {
        let mut e = OnboardingEngine::new();
        e.record_usage(make_record("u1", "edit", true, 100));
        let hotspot = HotspotMap::new();
        let guide = e.generate_guide("u1", &hotspot);
        assert!(guide.is_some());
    }

    #[test]
    fn test_engine_generate_guide_unknown_user() {
        let e = OnboardingEngine::new();
        let hotspot = HotspotMap::new();
        let guide = e.generate_guide("unknown", &hotspot);
        assert!(guide.is_none());
    }

    #[test]
    fn test_engine_guide_markdown_has_header() {
        let mut e = OnboardingEngine::new();
        e.record_usage(make_record("u1", "edit", true, 100));
        let hotspot = HotspotMap::new();
        let guide = e.generate_guide("u1", &hotspot).unwrap();
        let md = guide.to_markdown();
        assert!(md.starts_with("# Onboarding Guide"));
    }

    #[test]
    fn test_engine_guide_markdown_has_top_features() {
        let mut e = OnboardingEngine::new();
        e.record_usage(make_record("u1", "edit", true, 100));
        let hotspot = HotspotMap::new();
        let guide = e.generate_guide("u1", &hotspot).unwrap();
        let md = guide.to_markdown();
        assert!(md.contains("## Top Features"));
    }

    #[test]
    fn test_engine_guide_markdown_has_hotspot_section() {
        let mut e = OnboardingEngine::new();
        e.record_usage(make_record("u1", "edit", true, 100));
        let hotspot = HotspotMap::new();
        let guide = e.generate_guide("u1", &hotspot).unwrap();
        let md = guide.to_markdown();
        assert!(md.contains("## Hotspot Files"));
    }

    #[test]
    fn test_engine_guide_includes_hotspot_files() {
        let mut e = OnboardingEngine::new();
        e.record_usage(make_record("u1", "edit", true, 100));
        let mut hotspot = HotspotMap::new();
        hotspot.record_access("src/core.rs", true);
        let guide = e.generate_guide("u1", &hotspot).unwrap();
        assert!(guide.hotspot_files.contains(&"src/core.rs".to_string()));
    }
}
