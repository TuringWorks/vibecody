//! Next-task prediction at workflow level (FIT-GAP v7, Gap 13).
//!
//! Predicts the next developer action based on transition probabilities,
//! contextual rules, and workflow phase inference. Tracks accept/reject
//! feedback to measure prediction accuracy over time.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum DevAction {
    EditFile(String),
    CreateFile(String),
    DeleteFile(String),
    RunTests,
    RunBuild,
    RunLint,
    GitCommit,
    GitPush,
    CreatePR,
    Deploy,
    WriteDoc,
    AddDependency,
    Refactor,
    Debug,
}

impl DevAction {
    /// Canonical key used in the transition matrix (strips payload).
    fn key(&self) -> String {
        match self {
            DevAction::EditFile(_) => "EditFile".to_string(),
            DevAction::CreateFile(_) => "CreateFile".to_string(),
            DevAction::DeleteFile(_) => "DeleteFile".to_string(),
            DevAction::RunTests => "RunTests".to_string(),
            DevAction::RunBuild => "RunBuild".to_string(),
            DevAction::RunLint => "RunLint".to_string(),
            DevAction::GitCommit => "GitCommit".to_string(),
            DevAction::GitPush => "GitPush".to_string(),
            DevAction::CreatePR => "CreatePR".to_string(),
            DevAction::Deploy => "Deploy".to_string(),
            DevAction::WriteDoc => "WriteDoc".to_string(),
            DevAction::AddDependency => "AddDependency".to_string(),
            DevAction::Refactor => "Refactor".to_string(),
            DevAction::Debug => "Debug".to_string(),
        }
    }

    /// Build a default instance from a key string.
    fn from_key(key: &str) -> Option<DevAction> {
        match key {
            "EditFile" => Some(DevAction::EditFile(String::new())),
            "CreateFile" => Some(DevAction::CreateFile(String::new())),
            "DeleteFile" => Some(DevAction::DeleteFile(String::new())),
            "RunTests" => Some(DevAction::RunTests),
            "RunBuild" => Some(DevAction::RunBuild),
            "RunLint" => Some(DevAction::RunLint),
            "GitCommit" => Some(DevAction::GitCommit),
            "GitPush" => Some(DevAction::GitPush),
            "CreatePR" => Some(DevAction::CreatePR),
            "Deploy" => Some(DevAction::Deploy),
            "WriteDoc" => Some(DevAction::WriteDoc),
            "AddDependency" => Some(DevAction::AddDependency),
            "Refactor" => Some(DevAction::Refactor),
            "Debug" => Some(DevAction::Debug),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowPhase {
    Coding,
    Testing,
    Reviewing,
    Committing,
    Deploying,
    Documenting,
    Debugging,
}

// ---------------------------------------------------------------------------
// Data structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSuggestion {
    pub id: String,
    pub action: DevAction,
    pub description: String,
    pub confidence: f64,
    pub reasoning: String,
    pub phase: WorkflowPhase,
    pub priority: u32,
    pub estimated_effort_mins: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub action: DevAction,
    pub timestamp: u64,
    pub file_path: Option<String>,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionMetrics {
    pub total_suggestions: u32,
    pub accepted: u32,
    pub rejected: u32,
    pub accuracy: f64,
    pub by_phase: HashMap<String, PhaseStats>,
}

impl Default for PredictionMetrics {
    fn default() -> Self {
        Self {
            total_suggestions: 0,
            accepted: 0,
            rejected: 0,
            accuracy: 0.0,
            by_phase: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseStats {
    pub suggestions: u32,
    pub accepted: u32,
    pub accuracy: f64,
}

impl Default for PhaseStats {
    fn default() -> Self {
        Self {
            suggestions: 0,
            accepted: 0,
            accuracy: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// TransitionMatrix
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionMatrix {
    pub transitions: HashMap<String, HashMap<String, u32>>,
}

impl Default for TransitionMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl TransitionMatrix {
    pub fn new() -> Self {
        Self {
            transitions: HashMap::new(),
        }
    }

    pub fn record_transition(&mut self, from: &DevAction, to: &DevAction) {
        let from_key = from.key();
        let to_key = to.key();
        let inner = self.transitions.entry(from_key).or_default();
        *inner.entry(to_key).or_insert(0) += 1;
    }

    pub fn get_probability(&self, from: &DevAction, to: &DevAction) -> f64 {
        let from_key = from.key();
        let to_key = to.key();
        let inner = match self.transitions.get(&from_key) {
            Some(m) => m,
            None => return 0.0,
        };
        let total: u32 = inner.values().sum();
        if total == 0 {
            return 0.0;
        }
        let count = inner.get(&to_key).copied().unwrap_or(0);
        count as f64 / total as f64
    }

    pub fn top_next_actions(&self, from: &DevAction, n: usize) -> Vec<(DevAction, f64)> {
        let from_key = from.key();
        let inner = match self.transitions.get(&from_key) {
            Some(m) => m,
            None => return Vec::new(),
        };
        let total: u32 = inner.values().sum();
        if total == 0 {
            return Vec::new();
        }
        let mut pairs: Vec<(&String, &u32)> = inner.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        pairs
            .into_iter()
            .take(n)
            .filter_map(|(k, c)| {
                DevAction::from_key(k).map(|a| (a, *c as f64 / total as f64))
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// PredictionRule & ContextualRules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionRule {
    pub trigger: DevAction,
    pub suggested: DevAction,
    pub confidence: f64,
    pub description: String,
    pub min_recency: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextualRules {
    pub rules: Vec<PredictionRule>,
}

impl Default for ContextualRules {
    fn default() -> Self {
        Self {
            rules: vec![
                PredictionRule {
                    trigger: DevAction::EditFile(String::new()),
                    suggested: DevAction::RunTests,
                    confidence: 0.8,
                    description: "Run tests after editing".to_string(),
                    min_recency: 3,
                },
                PredictionRule {
                    trigger: DevAction::RunTests,
                    suggested: DevAction::Debug,
                    confidence: 0.9,
                    description: "Debug failing tests".to_string(),
                    min_recency: 2,
                },
                PredictionRule {
                    trigger: DevAction::EditFile(String::new()),
                    suggested: DevAction::RunLint,
                    confidence: 0.6,
                    description: "Lint after changes".to_string(),
                    min_recency: 3,
                },
                PredictionRule {
                    trigger: DevAction::RunTests,
                    suggested: DevAction::GitCommit,
                    confidence: 0.7,
                    description: "Commit after tests pass".to_string(),
                    min_recency: 2,
                },
                PredictionRule {
                    trigger: DevAction::GitCommit,
                    suggested: DevAction::GitPush,
                    confidence: 0.6,
                    description: "Push after commit".to_string(),
                    min_recency: 2,
                },
                PredictionRule {
                    trigger: DevAction::CreateFile(String::new()),
                    suggested: DevAction::EditFile(String::new()),
                    confidence: 0.8,
                    description: "Edit new file".to_string(),
                    min_recency: 2,
                },
                PredictionRule {
                    trigger: DevAction::GitPush,
                    suggested: DevAction::CreatePR,
                    confidence: 0.5,
                    description: "Create PR after push".to_string(),
                    min_recency: 2,
                },
            ],
        }
    }
}

impl ContextualRules {
    pub fn apply(&self, recent_actions: &[ActionRecord]) -> Vec<TaskSuggestion> {
        let mut suggestions = Vec::new();
        let mut id_counter = 0u32;

        for rule in &self.rules {
            let trigger_key = rule.trigger.key();
            let window = if rule.min_recency <= recent_actions.len() {
                &recent_actions[recent_actions.len() - rule.min_recency..]
            } else {
                recent_actions
            };
            let matched = window.iter().any(|r| r.action.key() == trigger_key);
            if matched {
                id_counter += 1;
                let phase = phase_for_action(&rule.suggested);
                suggestions.push(TaskSuggestion {
                    id: format!("rule-{}", id_counter),
                    action: rule.suggested.clone(),
                    description: rule.description.clone(),
                    confidence: rule.confidence,
                    reasoning: format!(
                        "Recent {} triggers rule: {}",
                        trigger_key, rule.description
                    ),
                    phase,
                    priority: (rule.confidence * 100.0) as u32,
                    estimated_effort_mins: None,
                });
            }
        }

        suggestions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suggestions
    }
}

fn phase_for_action(action: &DevAction) -> WorkflowPhase {
    match action {
        DevAction::EditFile(_)
        | DevAction::CreateFile(_)
        | DevAction::DeleteFile(_)
        | DevAction::AddDependency
        | DevAction::Refactor => WorkflowPhase::Coding,
        DevAction::RunTests | DevAction::RunBuild | DevAction::RunLint => WorkflowPhase::Testing,
        DevAction::GitCommit => WorkflowPhase::Committing,
        DevAction::GitPush | DevAction::CreatePR => WorkflowPhase::Reviewing,
        DevAction::Deploy => WorkflowPhase::Deploying,
        DevAction::WriteDoc => WorkflowPhase::Documenting,
        DevAction::Debug => WorkflowPhase::Debugging,
    }
}

// ---------------------------------------------------------------------------
// WorkflowTracker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTracker {
    pub recent_actions: Vec<ActionRecord>,
    pub phase: WorkflowPhase,
    pub session_start: u64,
}

impl WorkflowTracker {
    pub fn new(session_start: u64) -> Self {
        Self {
            recent_actions: Vec::new(),
            phase: WorkflowPhase::Coding,
            session_start,
        }
    }

    pub fn record_action(&mut self, record: ActionRecord) {
        self.recent_actions.push(record);
        self.phase = self.infer_phase();
    }

    pub fn infer_phase(&self) -> WorkflowPhase {
        let window_size = 5;
        let window = if self.recent_actions.len() > window_size {
            &self.recent_actions[self.recent_actions.len() - window_size..]
        } else {
            &self.recent_actions
        };
        if window.is_empty() {
            return WorkflowPhase::Coding;
        }

        let mut counts: HashMap<String, u32> = HashMap::new();
        for r in window {
            let phase_key = match phase_for_action(&r.action) {
                WorkflowPhase::Coding => "Coding",
                WorkflowPhase::Testing => "Testing",
                WorkflowPhase::Reviewing => "Reviewing",
                WorkflowPhase::Committing => "Committing",
                WorkflowPhase::Deploying => "Deploying",
                WorkflowPhase::Documenting => "Documenting",
                WorkflowPhase::Debugging => "Debugging",
            };
            *counts.entry(phase_key.to_string()).or_insert(0) += 1;
        }

        let top = counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .map(|(k, _)| k)
            .unwrap_or_else(|| "Coding".to_string());

        match top.as_str() {
            "Coding" => WorkflowPhase::Coding,
            "Testing" => WorkflowPhase::Testing,
            "Reviewing" => WorkflowPhase::Reviewing,
            "Committing" => WorkflowPhase::Committing,
            "Deploying" => WorkflowPhase::Deploying,
            "Documenting" => WorkflowPhase::Documenting,
            "Debugging" => WorkflowPhase::Debugging,
            _ => WorkflowPhase::Coding,
        }
    }

    pub fn get_recent(&self, n: usize) -> &[ActionRecord] {
        if n >= self.recent_actions.len() {
            &self.recent_actions
        } else {
            &self.recent_actions[self.recent_actions.len() - n..]
        }
    }

    pub fn phase_duration_secs(&self, now: u64) -> u64 {
        // Walk backwards to find when the current phase started.
        let current_phase = self.infer_phase();
        let mut phase_start = self.session_start;
        for record in self.recent_actions.iter().rev() {
            if phase_for_action(&record.action) == current_phase {
                phase_start = record.timestamp;
            } else {
                break;
            }
        }
        now.saturating_sub(phase_start)
    }
}

// ---------------------------------------------------------------------------
// PredictionEngine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionEngine {
    pub tracker: WorkflowTracker,
    pub transition_matrix: TransitionMatrix,
    pub suggestions: Vec<TaskSuggestion>,
    pub accepted_count: u32,
    pub rejected_count: u32,
    pub metrics: PredictionMetrics,
    contextual_rules: ContextualRules,
}

impl PredictionEngine {
    pub fn new(session_start: u64) -> Self {
        Self {
            tracker: WorkflowTracker::new(session_start),
            transition_matrix: TransitionMatrix::new(),
            suggestions: Vec::new(),
            accepted_count: 0,
            rejected_count: 0,
            metrics: PredictionMetrics::default(),
            contextual_rules: ContextualRules::default(),
        }
    }

    pub fn record_action(&mut self, action: DevAction, timestamp: u64, file_path: Option<String>) {
        // Record transition from previous action.
        if let Some(prev) = self.tracker.recent_actions.last() {
            self.transition_matrix
                .record_transition(&prev.action, &action);
        }

        let record = ActionRecord {
            action,
            timestamp,
            file_path,
            accepted: true,
        };
        self.tracker.record_action(record);
    }

    pub fn suggest_next(&mut self, n: usize) -> Vec<TaskSuggestion> {
        let mut all: Vec<TaskSuggestion> = Vec::new();

        // 1. Rule-based suggestions.
        let rule_suggestions = self.contextual_rules.apply(&self.tracker.recent_actions);
        all.extend(rule_suggestions);

        // 2. Transition-matrix suggestions.
        if let Some(last) = self.tracker.recent_actions.last() {
            let top = self.transition_matrix.top_next_actions(&last.action, n);
            for (i, (action, prob)) in top.into_iter().enumerate() {
                let phase = phase_for_action(&action);
                all.push(TaskSuggestion {
                    id: format!("trans-{}", i + 1),
                    action: action.clone(),
                    description: format!("Predicted from transition history (p={:.2})", prob),
                    confidence: prob,
                    reasoning: format!(
                        "Transition matrix: {} -> {} with probability {:.2}",
                        self.tracker
                            .recent_actions
                            .last()
                            .map(|r| r.action.key())
                            .unwrap_or_default(),
                        action.key(),
                        prob
                    ),
                    phase,
                    priority: (prob * 100.0) as u32,
                    estimated_effort_mins: None,
                });
            }
        }

        // Deduplicate by action key, keeping highest confidence.
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut deduped: Vec<TaskSuggestion> = Vec::new();
        for s in all {
            let key = s.action.key();
            if let Some(&idx) = seen.get(&key) {
                if s.confidence > deduped[idx].confidence {
                    deduped[idx] = s;
                }
            } else {
                seen.insert(key, deduped.len());
                deduped.push(s);
            }
        }

        // Sort by confidence desc, then priority desc.
        deduped.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.priority.cmp(&a.priority))
        });

        deduped.truncate(n);

        // Update metrics.
        self.metrics.total_suggestions += deduped.len() as u32;
        let phase_key = format!("{:?}", self.tracker.phase);
        let ps = self
            .metrics
            .by_phase
            .entry(phase_key)
            .or_default();
        ps.suggestions += deduped.len() as u32;

        self.suggestions = deduped.clone();
        deduped
    }

    pub fn accept_suggestion(&mut self, suggestion_id: &str) -> Result<(), String> {
        let found = self
            .suggestions
            .iter()
            .any(|s| s.id == suggestion_id);
        if !found {
            return Err(format!("Suggestion '{}' not found", suggestion_id));
        }
        self.accepted_count += 1;
        self.metrics.accepted += 1;
        self.update_accuracy();

        let phase_key = format!("{:?}", self.tracker.phase);
        let ps = self
            .metrics
            .by_phase
            .entry(phase_key)
            .or_default();
        ps.accepted += 1;
        ps.accuracy = if ps.suggestions > 0 {
            ps.accepted as f64 / ps.suggestions as f64
        } else {
            0.0
        };

        Ok(())
    }

    pub fn reject_suggestion(&mut self, suggestion_id: &str) -> Result<(), String> {
        let found = self
            .suggestions
            .iter()
            .any(|s| s.id == suggestion_id);
        if !found {
            return Err(format!("Suggestion '{}' not found", suggestion_id));
        }
        self.rejected_count += 1;
        self.metrics.rejected += 1;
        self.update_accuracy();
        Ok(())
    }

    pub fn get_accuracy(&self) -> f64 {
        let total = self.accepted_count + self.rejected_count;
        if total == 0 {
            return 0.0;
        }
        self.accepted_count as f64 / total as f64
    }

    pub fn get_current_phase(&self) -> &WorkflowPhase {
        &self.tracker.phase
    }

    pub fn reset_session(&mut self, session_start: u64) {
        self.tracker = WorkflowTracker::new(session_start);
        self.suggestions.clear();
        self.accepted_count = 0;
        self.rejected_count = 0;
        self.metrics = PredictionMetrics::default();
        // Keep the transition matrix — learned data persists across sessions.
    }

    fn update_accuracy(&mut self) {
        self.metrics.accuracy = self.get_accuracy();
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(offset: u64) -> u64 {
        1_000_000 + offset
    }

    // --- DevAction key / from_key round-trip ---

    #[test]
    fn test_dev_action_key_roundtrip() {
        let actions = vec![
            DevAction::EditFile("foo.rs".into()),
            DevAction::CreateFile("bar.rs".into()),
            DevAction::DeleteFile("baz.rs".into()),
            DevAction::RunTests,
            DevAction::RunBuild,
            DevAction::RunLint,
            DevAction::GitCommit,
            DevAction::GitPush,
            DevAction::CreatePR,
            DevAction::Deploy,
            DevAction::WriteDoc,
            DevAction::AddDependency,
            DevAction::Refactor,
            DevAction::Debug,
        ];
        for a in &actions {
            let key = a.key();
            let restored = DevAction::from_key(&key);
            assert!(restored.is_some(), "from_key failed for {}", key);
        }
    }

    #[test]
    fn test_from_key_unknown() {
        assert_eq!(DevAction::from_key("Unknown"), None);
    }

    // --- TransitionMatrix ---

    #[test]
    fn test_transition_matrix_empty() {
        let m = TransitionMatrix::new();
        assert_eq!(m.get_probability(&DevAction::RunTests, &DevAction::Debug), 0.0);
    }

    #[test]
    fn test_transition_record_and_query() {
        let mut m = TransitionMatrix::new();
        m.record_transition(&DevAction::EditFile("a".into()), &DevAction::RunTests);
        m.record_transition(&DevAction::EditFile("b".into()), &DevAction::RunTests);
        m.record_transition(&DevAction::EditFile("c".into()), &DevAction::RunLint);
        let p = m.get_probability(&DevAction::EditFile(String::new()), &DevAction::RunTests);
        assert!((p - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_transition_top_actions() {
        let mut m = TransitionMatrix::new();
        for _ in 0..5 {
            m.record_transition(&DevAction::RunTests, &DevAction::GitCommit);
        }
        for _ in 0..3 {
            m.record_transition(&DevAction::RunTests, &DevAction::Debug);
        }
        m.record_transition(&DevAction::RunTests, &DevAction::RunBuild);
        let top = m.top_next_actions(&DevAction::RunTests, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, DevAction::GitCommit);
        assert_eq!(top[1].0, DevAction::Debug);
    }

    #[test]
    fn test_transition_top_actions_empty() {
        let m = TransitionMatrix::new();
        let top = m.top_next_actions(&DevAction::RunTests, 5);
        assert!(top.is_empty());
    }

    #[test]
    fn test_transition_probability_single() {
        let mut m = TransitionMatrix::new();
        m.record_transition(&DevAction::GitCommit, &DevAction::GitPush);
        assert!((m.get_probability(&DevAction::GitCommit, &DevAction::GitPush) - 1.0).abs() < 1e-9);
    }

    // --- WorkflowTracker ---

    #[test]
    fn test_tracker_new_defaults() {
        let t = WorkflowTracker::new(100);
        assert_eq!(t.phase, WorkflowPhase::Coding);
        assert!(t.recent_actions.is_empty());
        assert_eq!(t.session_start, 100);
    }

    #[test]
    fn test_tracker_record_action() {
        let mut t = WorkflowTracker::new(0);
        t.record_action(ActionRecord {
            action: DevAction::RunTests,
            timestamp: 10,
            file_path: None,
            accepted: true,
        });
        assert_eq!(t.recent_actions.len(), 1);
        assert_eq!(t.phase, WorkflowPhase::Testing);
    }

    #[test]
    fn test_tracker_infer_phase_coding() {
        let mut t = WorkflowTracker::new(0);
        for i in 0..4 {
            t.record_action(ActionRecord {
                action: DevAction::EditFile(format!("f{}.rs", i)),
                timestamp: i as u64,
                file_path: None,
                accepted: true,
            });
        }
        assert_eq!(t.infer_phase(), WorkflowPhase::Coding);
    }

    #[test]
    fn test_tracker_infer_phase_testing() {
        let mut t = WorkflowTracker::new(0);
        t.record_action(ActionRecord { action: DevAction::RunTests, timestamp: 1, file_path: None, accepted: true });
        t.record_action(ActionRecord { action: DevAction::RunBuild, timestamp: 2, file_path: None, accepted: true });
        t.record_action(ActionRecord { action: DevAction::RunLint, timestamp: 3, file_path: None, accepted: true });
        assert_eq!(t.infer_phase(), WorkflowPhase::Testing);
    }

    #[test]
    fn test_tracker_infer_phase_empty() {
        let t = WorkflowTracker::new(0);
        assert_eq!(t.infer_phase(), WorkflowPhase::Coding);
    }

    #[test]
    fn test_tracker_get_recent() {
        let mut t = WorkflowTracker::new(0);
        for i in 0..10 {
            t.record_action(ActionRecord {
                action: DevAction::EditFile(format!("{}", i)),
                timestamp: i,
                file_path: None,
                accepted: true,
            });
        }
        assert_eq!(t.get_recent(3).len(), 3);
        assert_eq!(t.get_recent(20).len(), 10);
    }

    #[test]
    fn test_tracker_phase_duration() {
        let mut t = WorkflowTracker::new(100);
        t.record_action(ActionRecord {
            action: DevAction::RunTests,
            timestamp: 200,
            file_path: None,
            accepted: true,
        });
        t.record_action(ActionRecord {
            action: DevAction::RunBuild,
            timestamp: 300,
            file_path: None,
            accepted: true,
        });
        let d = t.phase_duration_secs(500);
        // Phase started at timestamp 200, so duration = 500-200 = 300
        assert_eq!(d, 300);
    }

    // --- ContextualRules ---

    #[test]
    fn test_default_rules_count() {
        let cr = ContextualRules::default();
        assert_eq!(cr.rules.len(), 7);
    }

    #[test]
    fn test_rules_apply_edit_triggers() {
        let cr = ContextualRules::default();
        let records = vec![ActionRecord {
            action: DevAction::EditFile("main.rs".into()),
            timestamp: 1,
            file_path: Some("main.rs".into()),
            accepted: true,
        }];
        let suggestions = cr.apply(&records);
        // Should match EditFile -> RunTests and EditFile -> RunLint
        let keys: Vec<String> = suggestions.iter().map(|s| s.action.key()).collect();
        assert!(keys.contains(&"RunTests".to_string()));
        assert!(keys.contains(&"RunLint".to_string()));
    }

    #[test]
    fn test_rules_apply_commit_triggers_push() {
        let cr = ContextualRules::default();
        let records = vec![ActionRecord {
            action: DevAction::GitCommit,
            timestamp: 1,
            file_path: None,
            accepted: true,
        }];
        let suggestions = cr.apply(&records);
        let keys: Vec<String> = suggestions.iter().map(|s| s.action.key()).collect();
        assert!(keys.contains(&"GitPush".to_string()));
    }

    #[test]
    fn test_rules_apply_empty_history() {
        let cr = ContextualRules::default();
        let suggestions = cr.apply(&[]);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_rules_apply_create_file_triggers_edit() {
        let cr = ContextualRules::default();
        let records = vec![ActionRecord {
            action: DevAction::CreateFile("new.rs".into()),
            timestamp: 1,
            file_path: None,
            accepted: true,
        }];
        let suggestions = cr.apply(&records);
        let keys: Vec<String> = suggestions.iter().map(|s| s.action.key()).collect();
        assert!(keys.contains(&"EditFile".to_string()));
    }

    #[test]
    fn test_rules_sorted_by_confidence() {
        let cr = ContextualRules::default();
        let records = vec![ActionRecord {
            action: DevAction::EditFile("a.rs".into()),
            timestamp: 1,
            file_path: None,
            accepted: true,
        }];
        let suggestions = cr.apply(&records);
        for w in suggestions.windows(2) {
            assert!(w[0].confidence >= w[1].confidence);
        }
    }

    #[test]
    fn test_rules_push_triggers_pr() {
        let cr = ContextualRules::default();
        let records = vec![ActionRecord {
            action: DevAction::GitPush,
            timestamp: 1,
            file_path: None,
            accepted: true,
        }];
        let suggestions = cr.apply(&records);
        let keys: Vec<String> = suggestions.iter().map(|s| s.action.key()).collect();
        assert!(keys.contains(&"CreatePR".to_string()));
    }

    // --- PredictionEngine ---

    #[test]
    fn test_engine_new() {
        let e = PredictionEngine::new(0);
        assert_eq!(e.accepted_count, 0);
        assert_eq!(e.rejected_count, 0);
        assert!(e.suggestions.is_empty());
    }

    #[test]
    fn test_engine_record_action() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, Some("a.rs".into()));
        assert_eq!(e.tracker.recent_actions.len(), 1);
    }

    #[test]
    fn test_engine_record_builds_transitions() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        e.record_action(DevAction::RunTests, 2, None);
        let p = e.transition_matrix.get_probability(
            &DevAction::EditFile(String::new()),
            &DevAction::RunTests,
        );
        assert!((p - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_engine_suggest_from_rules() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        let suggestions = e.suggest_next(5);
        assert!(!suggestions.is_empty());
        // Should have RunTests from rule
        assert!(suggestions.iter().any(|s| s.action.key() == "RunTests"));
    }

    #[test]
    fn test_engine_suggest_from_transitions() {
        let mut e = PredictionEngine::new(0);
        // Build up transition data
        for i in 0..10 {
            e.record_action(DevAction::EditFile(format!("{}", i)), ts(i * 2), None);
            e.record_action(DevAction::RunBuild, ts(i * 2 + 1), None);
        }
        // Last action is RunBuild; transitions from RunBuild should suggest EditFile
        let suggestions = e.suggest_next(5);
        assert!(suggestions.iter().any(|s| s.action.key() == "EditFile"));
    }

    #[test]
    fn test_engine_suggest_deduplicates() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        e.record_action(DevAction::RunTests, 2, None);
        // EditFile->RunTests exists in both rules and transitions
        e.record_action(DevAction::EditFile("b.rs".into()), 3, None);
        let suggestions = e.suggest_next(10);
        let run_test_count = suggestions.iter().filter(|s| s.action.key() == "RunTests").count();
        assert_eq!(run_test_count, 1, "RunTests should appear only once");
    }

    #[test]
    fn test_engine_accept_suggestion() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        let suggestions = e.suggest_next(3);
        assert!(!suggestions.is_empty());
        let id = suggestions[0].id.clone();
        assert!(e.accept_suggestion(&id).is_ok());
        assert_eq!(e.accepted_count, 1);
    }

    #[test]
    fn test_engine_reject_suggestion() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        let suggestions = e.suggest_next(3);
        let id = suggestions[0].id.clone();
        assert!(e.reject_suggestion(&id).is_ok());
        assert_eq!(e.rejected_count, 1);
    }

    #[test]
    fn test_engine_accept_nonexistent() {
        let mut e = PredictionEngine::new(0);
        let r = e.accept_suggestion("nonexistent-id");
        assert!(r.is_err());
    }

    #[test]
    fn test_engine_reject_nonexistent() {
        let mut e = PredictionEngine::new(0);
        let r = e.reject_suggestion("nope");
        assert!(r.is_err());
    }

    #[test]
    fn test_engine_accuracy_none() {
        let e = PredictionEngine::new(0);
        assert_eq!(e.get_accuracy(), 0.0);
    }

    #[test]
    fn test_engine_accuracy_mixed() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        let s = e.suggest_next(3);
        let id0 = s[0].id.clone();
        let id1 = s.get(1).map(|x| x.id.clone());
        e.accept_suggestion(&id0).unwrap();
        if let Some(id) = id1 {
            e.reject_suggestion(&id).unwrap();
        }
        let acc = e.get_accuracy();
        assert!(acc > 0.0);
    }

    #[test]
    fn test_engine_accuracy_all_accepted() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::GitCommit, 1, None);
        let s = e.suggest_next(1);
        let id = s[0].id.clone();
        e.accept_suggestion(&id).unwrap();
        assert!((e.get_accuracy() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_engine_accuracy_all_rejected() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::GitCommit, 1, None);
        let s = e.suggest_next(1);
        let id = s[0].id.clone();
        e.reject_suggestion(&id).unwrap();
        assert_eq!(e.get_accuracy(), 0.0);
    }

    #[test]
    fn test_engine_get_current_phase() {
        let mut e = PredictionEngine::new(0);
        assert_eq!(*e.get_current_phase(), WorkflowPhase::Coding);
        e.record_action(DevAction::RunTests, 1, None);
        e.record_action(DevAction::RunBuild, 2, None);
        e.record_action(DevAction::RunLint, 3, None);
        assert_eq!(*e.get_current_phase(), WorkflowPhase::Testing);
    }

    #[test]
    fn test_engine_reset_session() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        e.record_action(DevAction::RunTests, 2, None);
        let s = e.suggest_next(3);
        let id = s[0].id.clone();
        e.accept_suggestion(&id).unwrap();

        // Save transition data reference
        let had_transitions = !e.transition_matrix.transitions.is_empty();

        e.reset_session(1000);
        assert!(e.tracker.recent_actions.is_empty());
        assert_eq!(e.accepted_count, 0);
        assert_eq!(e.rejected_count, 0);
        assert!(e.suggestions.is_empty());
        assert_eq!(e.tracker.session_start, 1000);
        // Transition matrix persists across resets.
        assert_eq!(!e.transition_matrix.transitions.is_empty(), had_transitions);
    }

    #[test]
    fn test_engine_metrics_total_suggestions() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        let s = e.suggest_next(5);
        assert_eq!(e.metrics.total_suggestions, s.len() as u32);
    }

    #[test]
    fn test_engine_metrics_by_phase() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        e.suggest_next(3);
        // Phase should be Coding
        assert!(e.metrics.by_phase.contains_key("Coding"));
    }

    #[test]
    fn test_engine_phase_stats_accuracy() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::GitCommit, 1, None);
        let s = e.suggest_next(1);
        let id = s[0].id.clone();
        e.accept_suggestion(&id).unwrap();
        let phase_key = format!("{:?}", e.tracker.phase);
        let ps = e.metrics.by_phase.get(&phase_key).unwrap();
        assert!(ps.accuracy > 0.0);
    }

    #[test]
    fn test_engine_multiple_suggestions_priority_order() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("x.rs".into()), 1, None);
        let suggestions = e.suggest_next(10);
        // Should be sorted by confidence descending.
        for w in suggestions.windows(2) {
            assert!(
                w[0].confidence >= w[1].confidence,
                "Suggestions not sorted: {} >= {} failed",
                w[0].confidence,
                w[1].confidence
            );
        }
    }

    #[test]
    fn test_engine_no_history_suggestions() {
        let mut e = PredictionEngine::new(0);
        let suggestions = e.suggest_next(5);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_engine_single_action_suggestions() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::Deploy, 1, None);
        let suggestions = e.suggest_next(5);
        // Deploy doesn't match any rule trigger, so may be empty or from transitions.
        // Just ensure no panic.
        let _ = suggestions;
    }

    #[test]
    fn test_engine_all_same_actions() {
        let mut e = PredictionEngine::new(0);
        for i in 0..20 {
            e.record_action(DevAction::RunTests, ts(i), None);
        }
        // Transition RunTests -> RunTests should have probability 1.0
        let p = e.transition_matrix.get_probability(&DevAction::RunTests, &DevAction::RunTests);
        assert!((p - 1.0).abs() < 1e-9);
        let suggestions = e.suggest_next(3);
        assert!(!suggestions.is_empty());
    }

    #[test]
    fn test_suggestion_has_id() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        let suggestions = e.suggest_next(3);
        for s in &suggestions {
            assert!(!s.id.is_empty());
        }
    }

    #[test]
    fn test_suggestion_confidence_bounds() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        let suggestions = e.suggest_next(5);
        for s in &suggestions {
            assert!(s.confidence >= 0.0 && s.confidence <= 1.0);
        }
    }

    #[test]
    fn test_phase_for_action_coverage() {
        assert_eq!(phase_for_action(&DevAction::EditFile(String::new())), WorkflowPhase::Coding);
        assert_eq!(phase_for_action(&DevAction::CreateFile(String::new())), WorkflowPhase::Coding);
        assert_eq!(phase_for_action(&DevAction::DeleteFile(String::new())), WorkflowPhase::Coding);
        assert_eq!(phase_for_action(&DevAction::RunTests), WorkflowPhase::Testing);
        assert_eq!(phase_for_action(&DevAction::RunBuild), WorkflowPhase::Testing);
        assert_eq!(phase_for_action(&DevAction::RunLint), WorkflowPhase::Testing);
        assert_eq!(phase_for_action(&DevAction::GitCommit), WorkflowPhase::Committing);
        assert_eq!(phase_for_action(&DevAction::GitPush), WorkflowPhase::Reviewing);
        assert_eq!(phase_for_action(&DevAction::CreatePR), WorkflowPhase::Reviewing);
        assert_eq!(phase_for_action(&DevAction::Deploy), WorkflowPhase::Deploying);
        assert_eq!(phase_for_action(&DevAction::WriteDoc), WorkflowPhase::Documenting);
        assert_eq!(phase_for_action(&DevAction::Debug), WorkflowPhase::Debugging);
        assert_eq!(phase_for_action(&DevAction::AddDependency), WorkflowPhase::Coding);
        assert_eq!(phase_for_action(&DevAction::Refactor), WorkflowPhase::Coding);
    }

    #[test]
    fn test_tracker_infer_phase_debugging() {
        let mut t = WorkflowTracker::new(0);
        t.record_action(ActionRecord { action: DevAction::Debug, timestamp: 1, file_path: None, accepted: true });
        t.record_action(ActionRecord { action: DevAction::Debug, timestamp: 2, file_path: None, accepted: true });
        t.record_action(ActionRecord { action: DevAction::Debug, timestamp: 3, file_path: None, accepted: true });
        assert_eq!(t.infer_phase(), WorkflowPhase::Debugging);
    }

    #[test]
    fn test_tracker_infer_phase_documenting() {
        let mut t = WorkflowTracker::new(0);
        t.record_action(ActionRecord { action: DevAction::WriteDoc, timestamp: 1, file_path: None, accepted: true });
        t.record_action(ActionRecord { action: DevAction::WriteDoc, timestamp: 2, file_path: None, accepted: true });
        assert_eq!(t.infer_phase(), WorkflowPhase::Documenting);
    }

    #[test]
    fn test_transition_matrix_multiple_from_actions() {
        let mut m = TransitionMatrix::new();
        m.record_transition(&DevAction::EditFile("a".into()), &DevAction::RunTests);
        m.record_transition(&DevAction::RunTests, &DevAction::GitCommit);
        m.record_transition(&DevAction::GitCommit, &DevAction::GitPush);
        assert!((m.get_probability(&DevAction::EditFile(String::new()), &DevAction::RunTests) - 1.0).abs() < 1e-9);
        assert!((m.get_probability(&DevAction::RunTests, &DevAction::GitCommit) - 1.0).abs() < 1e-9);
        assert!((m.get_probability(&DevAction::GitCommit, &DevAction::GitPush) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_metrics_default() {
        let m = PredictionMetrics::default();
        assert_eq!(m.total_suggestions, 0);
        assert_eq!(m.accepted, 0);
        assert_eq!(m.rejected, 0);
        assert_eq!(m.accuracy, 0.0);
        assert!(m.by_phase.is_empty());
    }

    #[test]
    fn test_engine_suggest_truncates_to_n() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::EditFile("a.rs".into()), 1, None);
        let suggestions = e.suggest_next(1);
        assert!(suggestions.len() <= 1);
    }

    #[test]
    fn test_engine_long_workflow_sequence() {
        let mut e = PredictionEngine::new(0);
        // Simulate a real workflow
        e.record_action(DevAction::CreateFile("lib.rs".into()), ts(0), Some("lib.rs".into()));
        e.record_action(DevAction::EditFile("lib.rs".into()), ts(1), Some("lib.rs".into()));
        e.record_action(DevAction::RunTests, ts(2), None);
        e.record_action(DevAction::Debug, ts(3), None);
        e.record_action(DevAction::EditFile("lib.rs".into()), ts(4), Some("lib.rs".into()));
        e.record_action(DevAction::RunTests, ts(5), None);
        e.record_action(DevAction::GitCommit, ts(6), None);
        e.record_action(DevAction::GitPush, ts(7), None);
        e.record_action(DevAction::CreatePR, ts(8), None);

        // After CreatePR, no strong rule but transitions should exist
        let suggestions = e.suggest_next(3);
        // Just ensure no panics and engine is consistent
        assert_eq!(e.tracker.recent_actions.len(), 9);
        let _ = suggestions;
    }

    #[test]
    fn test_engine_feedback_updates_global_metrics() {
        let mut e = PredictionEngine::new(0);
        e.record_action(DevAction::GitCommit, 1, None);
        let s = e.suggest_next(2);
        if s.len() >= 2 {
            e.accept_suggestion(&s[0].id).unwrap();
            e.reject_suggestion(&s[1].id).unwrap();
            assert_eq!(e.metrics.accepted, 1);
            assert_eq!(e.metrics.rejected, 1);
            assert!((e.metrics.accuracy - 0.5).abs() < 1e-9);
        }
    }
}
