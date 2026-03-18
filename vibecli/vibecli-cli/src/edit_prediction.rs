//! RL-trained next-edit prediction — reinforcement learning for edit suggestions.
//!
//! Closes P3 Gap 18: RL-trained model predicts next edits based on user patterns,
//! cursor position, recent changes, and codebase context.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Edit event types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    Insert(String),
    Delete(usize),
    Replace(String, String),
    MoveCursor(usize, usize),
    Undo,
    Redo,
    Save,
    RunCommand(String),
}

impl EditAction {
    pub fn as_str(&self) -> &str {
        match self {
            EditAction::Insert(_) => "insert",
            EditAction::Delete(_) => "delete",
            EditAction::Replace(_, _) => "replace",
            EditAction::MoveCursor(_, _) => "move_cursor",
            EditAction::Undo => "undo",
            EditAction::Redo => "redo",
            EditAction::Save => "save",
            EditAction::RunCommand(_) => "run_command",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditEvent {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub action: EditAction,
    pub timestamp: u64,
    pub context_before: Option<String>,
    pub context_after: Option<String>,
}

// ---------------------------------------------------------------------------
// Edit pattern
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EditPattern {
    pub id: String,
    pub name: String,
    pub sequence: Vec<EditAction>,
    pub frequency: u64,
    pub avg_reward: f64,
    pub last_seen: u64,
}

impl EditPattern {
    pub fn confidence(&self) -> f64 {
        let freq_factor = (self.frequency as f64).ln().max(0.0) / 10.0;
        let reward_factor = self.avg_reward.clamp(0.0, 1.0);
        (freq_factor * 0.4 + reward_factor * 0.6).clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Prediction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EditPrediction {
    pub action: EditAction,
    pub confidence: f64,
    pub pattern_id: Option<String>,
    pub explanation: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PredictionOutcome {
    Accepted,
    Rejected,
    Modified,
    Ignored,
}

impl PredictionOutcome {
    pub fn reward(&self) -> f64 {
        match self {
            PredictionOutcome::Accepted => 1.0,
            PredictionOutcome::Modified => 0.5,
            PredictionOutcome::Ignored => 0.0,
            PredictionOutcome::Rejected => -0.3,
        }
    }
}

// ---------------------------------------------------------------------------
// RL model (Q-learning style)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EditState {
    pub file_type: String,
    pub recent_actions: Vec<String>,
    pub line_context: String,
    pub cursor_position: (usize, usize),
}

impl EditState {
    pub fn state_key(&self) -> String {
        let recent = if self.recent_actions.len() > 3 {
            self.recent_actions[self.recent_actions.len() - 3..].join(",")
        } else {
            self.recent_actions.join(",")
        };
        format!("{}:{}:{}", self.file_type, recent, self.line_context.len() % 10)
    }
}

pub struct RlModel {
    q_table: HashMap<String, HashMap<String, f64>>,
    learning_rate: f64,
    discount_factor: f64,
    exploration_rate: f64,
}

impl RlModel {
    pub fn new() -> Self {
        Self {
            q_table: HashMap::new(),
            learning_rate: 0.1,
            discount_factor: 0.95,
            exploration_rate: 0.1,
        }
    }

    pub fn with_params(learning_rate: f64, discount_factor: f64, exploration_rate: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            learning_rate: learning_rate.clamp(0.0, 1.0),
            discount_factor: discount_factor.clamp(0.0, 1.0),
            exploration_rate: exploration_rate.clamp(0.0, 1.0),
        }
    }

    pub fn get_q_value(&self, state: &str, action: &str) -> f64 {
        self.q_table
            .get(state)
            .and_then(|actions| actions.get(action))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn update(&mut self, state: &str, action: &str, reward: f64, next_state: &str) {
        let max_next = self.max_q_value(next_state);
        let current = self.get_q_value(state, action);
        let new_value = current + self.learning_rate * (reward + self.discount_factor * max_next - current);

        self.q_table
            .entry(state.to_string())
            .or_default()
            .insert(action.to_string(), new_value);
    }

    pub fn best_action(&self, state: &str) -> Option<(String, f64)> {
        self.q_table.get(state).and_then(|actions| {
            actions
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(k, v)| (k.clone(), *v))
        })
    }

    pub fn max_q_value(&self, state: &str) -> f64 {
        self.q_table
            .get(state)
            .and_then(|actions| actions.values().cloned().reduce(f64::max))
            .unwrap_or(0.0)
    }

    pub fn state_count(&self) -> usize {
        self.q_table.len()
    }

    pub fn total_entries(&self) -> usize {
        self.q_table.values().map(|a| a.len()).sum()
    }

    pub fn exploration_rate(&self) -> f64 {
        self.exploration_rate
    }

    pub fn decay_exploration(&mut self, factor: f64) {
        self.exploration_rate *= factor.clamp(0.0, 1.0);
        if self.exploration_rate < 0.01 {
            self.exploration_rate = 0.01;
        }
    }
}

impl Default for RlModel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Edit predictor (main engine)
// ---------------------------------------------------------------------------

pub struct EditPredictor {
    model: RlModel,
    patterns: Vec<EditPattern>,
    history: Vec<EditEvent>,
    predictions_made: u64,
    predictions_accepted: u64,
    max_history: usize,
    pattern_counter: u64,
}

impl EditPredictor {
    pub fn new() -> Self {
        Self {
            model: RlModel::new(),
            patterns: Vec::new(),
            history: Vec::new(),
            predictions_made: 0,
            predictions_accepted: 0,
            max_history: 1000,
            pattern_counter: 0,
        }
    }

    pub fn record_event(&mut self, event: EditEvent) {
        self.history.push(event);
        if self.history.len() > self.max_history {
            self.history.drain(..self.history.len() - self.max_history);
        }
        self.detect_patterns();
    }

    pub fn predict(&mut self, state: &EditState) -> Option<EditPrediction> {
        let state_key = state.state_key();

        // Check Q-table for best action
        if let Some((action_str, q_value)) = self.model.best_action(&state_key) {
            let confidence = sigmoid(q_value);
            if confidence > 0.3 {
                self.predictions_made += 1;
                let action = action_str_to_edit_action(&action_str);
                return Some(EditPrediction {
                    action,
                    confidence,
                    pattern_id: None,
                    explanation: format!("Q-value: {:.2}, confidence: {:.2}", q_value, confidence),
                    file: state.file_type.clone(),
                    line: state.cursor_position.0,
                    col: state.cursor_position.1,
                });
            }
        }

        // Fall back to pattern matching — extract data before mutating self
        let pattern_match = self.find_matching_pattern(state).and_then(|pattern| {
            pattern.sequence.last().map(|next_action| {
                (
                    next_action.clone(),
                    pattern.confidence(),
                    pattern.id.clone(),
                    pattern.name.clone(),
                    pattern.frequency,
                )
            })
        });
        if let Some((action, confidence, pat_id, pat_name, freq)) = pattern_match {
            self.predictions_made += 1;
            return Some(EditPrediction {
                action,
                confidence,
                pattern_id: Some(pat_id),
                explanation: format!("Pattern '{}' (freq: {})", pat_name, freq),
                file: state.file_type.clone(),
                line: state.cursor_position.0,
                col: state.cursor_position.1,
            });
        }

        None
    }

    pub fn record_outcome(&mut self, state: &EditState, action: &str, outcome: PredictionOutcome) {
        let reward = outcome.reward();
        let state_key = state.state_key();
        let next_state_key = state_key.clone(); // simplified: same state for now
        self.model.update(&state_key, action, reward, &next_state_key);

        if outcome == PredictionOutcome::Accepted {
            self.predictions_accepted += 1;
        }
    }

    pub fn acceptance_rate(&self) -> f64 {
        if self.predictions_made == 0 {
            return 0.0;
        }
        self.predictions_accepted as f64 / self.predictions_made as f64
    }

    pub fn predictions_made(&self) -> u64 {
        self.predictions_made
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    pub fn model_stats(&self) -> (usize, usize) {
        (self.model.state_count(), self.model.total_entries())
    }

    pub fn decay_exploration(&mut self, factor: f64) {
        self.model.decay_exploration(factor);
    }

    pub fn exploration_rate(&self) -> f64 {
        self.model.exploration_rate()
    }

    fn detect_patterns(&mut self) {
        if self.history.len() < 3 {
            return;
        }
        let recent: Vec<_> = self.history.iter().rev().take(5).collect();
        let action_seq: Vec<String> = recent.iter().rev().map(|e| e.action.as_str().to_string()).collect();

        // Check if this sequence matches an existing pattern
        for pattern in &mut self.patterns {
            let pat_seq: Vec<String> = pattern.sequence.iter().map(|a| a.as_str().to_string()).collect();
            if action_seq.ends_with(&pat_seq) {
                pattern.frequency += 1;
                pattern.last_seen = now();
                return;
            }
        }

        // Create new pattern if sequence repeats
        if action_seq.len() >= 3 {
            let sub = &action_seq[action_seq.len() - 3..];
            let count = count_subsequence_occurrences(&self.history, sub);
            if count >= 2 {
                self.pattern_counter += 1;
                let actions: Vec<EditAction> = sub.iter().map(|s| action_str_to_edit_action(s)).collect();
                self.patterns.push(EditPattern {
                    id: format!("pat-{}", self.pattern_counter),
                    name: format!("pattern-{}", sub.join("-")),
                    sequence: actions,
                    frequency: count as u64,
                    avg_reward: 0.5,
                    last_seen: now(),
                });
            }
        }
    }

    fn find_matching_pattern(&self, state: &EditState) -> Option<&EditPattern> {
        let recent: Vec<String> = state.recent_actions.to_vec();
        self.patterns.iter().max_by(|a, b| {
            let a_match = pattern_matches(&a.sequence, &recent);
            let b_match = pattern_matches(&b.sequence, &recent);
            let a_score = a_match as u64 * a.frequency;
            let b_score = b_match as u64 * b.frequency;
            a_score.cmp(&b_score)
        }).filter(|p| pattern_matches(&p.sequence, &recent))
    }
}

impl Default for EditPredictor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn action_str_to_edit_action(s: &str) -> EditAction {
    match s {
        "insert" => EditAction::Insert(String::new()),
        "delete" => EditAction::Delete(0),
        "replace" => EditAction::Replace(String::new(), String::new()),
        "move_cursor" => EditAction::MoveCursor(0, 0),
        "undo" => EditAction::Undo,
        "redo" => EditAction::Redo,
        "save" => EditAction::Save,
        "run_command" => EditAction::RunCommand(String::new()),
        _ => EditAction::Insert(s.to_string()),
    }
}

fn count_subsequence_occurrences(history: &[EditEvent], subseq: &[String]) -> usize {
    if subseq.is_empty() || history.len() < subseq.len() {
        return 0;
    }
    let actions: Vec<String> = history.iter().map(|e| e.action.as_str().to_string()).collect();
    let mut count = 0;
    for window in actions.windows(subseq.len()) {
        if window == subseq {
            count += 1;
        }
    }
    count
}

fn pattern_matches(pattern: &[EditAction], recent: &[String]) -> bool {
    if pattern.is_empty() || recent.is_empty() {
        return false;
    }
    let pat_strs: Vec<String> = pattern.iter().map(|a| a.as_str().to_string()).collect();
    // Check if any suffix of the pattern matches the end of recent actions
    for len in 1..=pat_strs.len().min(recent.len()) {
        let pat_suffix = &pat_strs[pat_strs.len() - len..];
        let recent_suffix = &recent[recent.len() - len..];
        if pat_suffix == recent_suffix {
            return true;
        }
    }
    false
}

fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(file: &str, line: usize, action: EditAction) -> EditEvent {
        EditEvent {
            file: file.to_string(),
            line,
            col: 0,
            action,
            timestamp: now(),
            context_before: None,
            context_after: None,
        }
    }

    fn make_state(file_type: &str, recent: &[&str], line: usize) -> EditState {
        EditState {
            file_type: file_type.to_string(),
            recent_actions: recent.iter().map(|s| s.to_string()).collect(),
            line_context: String::new(),
            cursor_position: (line, 0),
        }
    }

    // -- EditAction tests --

    #[test]
    fn test_edit_action_as_str() {
        assert_eq!(EditAction::Insert("x".into()).as_str(), "insert");
        assert_eq!(EditAction::Delete(5).as_str(), "delete");
        assert_eq!(EditAction::Undo.as_str(), "undo");
        assert_eq!(EditAction::Save.as_str(), "save");
        assert_eq!(EditAction::RunCommand("test".into()).as_str(), "run_command");
    }

    // -- PredictionOutcome tests --

    #[test]
    fn test_outcome_reward() {
        assert_eq!(PredictionOutcome::Accepted.reward(), 1.0);
        assert_eq!(PredictionOutcome::Modified.reward(), 0.5);
        assert_eq!(PredictionOutcome::Ignored.reward(), 0.0);
        assert!(PredictionOutcome::Rejected.reward() < 0.0);
    }

    // -- EditState tests --

    #[test]
    fn test_state_key() {
        let state = make_state("rs", &["insert", "delete", "save"], 10);
        let key = state.state_key();
        assert!(key.contains("rs"));
        assert!(key.contains("insert,delete,save"));
    }

    #[test]
    fn test_state_key_truncates_recent() {
        let state = make_state("py", &["a", "b", "c", "d", "e"], 1);
        let key = state.state_key();
        assert!(key.contains("c,d,e"));
        assert!(!key.contains("a,"));
    }

    // -- RlModel tests --

    #[test]
    fn test_rl_model_new() {
        let model = RlModel::new();
        assert_eq!(model.state_count(), 0);
        assert_eq!(model.total_entries(), 0);
        assert!((model.exploration_rate() - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rl_model_with_params() {
        let model = RlModel::with_params(0.2, 0.9, 0.05);
        assert!((model.learning_rate - 0.2).abs() < f64::EPSILON);
        assert!((model.discount_factor - 0.9).abs() < f64::EPSILON);
        assert!((model.exploration_rate() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rl_model_params_clamped() {
        let model = RlModel::with_params(2.0, -0.5, 1.5);
        assert!((model.learning_rate - 1.0).abs() < f64::EPSILON);
        assert!((model.discount_factor - 0.0).abs() < f64::EPSILON);
        assert!((model.exploration_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rl_model_update_and_get() {
        let mut model = RlModel::new();
        model.update("state1", "insert", 1.0, "state2");
        let q = model.get_q_value("state1", "insert");
        assert!(q > 0.0);
    }

    #[test]
    fn test_rl_model_best_action() {
        let mut model = RlModel::new();
        model.update("s1", "insert", 1.0, "s2");
        model.update("s1", "delete", 0.5, "s2");
        let (action, _) = model.best_action("s1").unwrap();
        assert_eq!(action, "insert");
    }

    #[test]
    fn test_rl_model_best_action_empty() {
        let model = RlModel::new();
        assert!(model.best_action("nonexistent").is_none());
    }

    #[test]
    fn test_rl_model_max_q_value() {
        let mut model = RlModel::new();
        model.update("s1", "a", 0.5, "s2");
        model.update("s1", "b", 1.0, "s2");
        let max = model.max_q_value("s1");
        assert!(max > 0.0);
    }

    #[test]
    fn test_rl_model_max_q_empty() {
        let model = RlModel::new();
        assert_eq!(model.max_q_value("nope"), 0.0);
    }

    #[test]
    fn test_rl_model_decay_exploration() {
        let mut model = RlModel::new();
        let initial = model.exploration_rate();
        model.decay_exploration(0.9);
        assert!(model.exploration_rate() < initial);
    }

    #[test]
    fn test_rl_model_decay_exploration_floor() {
        let mut model = RlModel::with_params(0.1, 0.95, 0.02);
        model.decay_exploration(0.01);
        assert!(model.exploration_rate() >= 0.01);
    }

    #[test]
    fn test_rl_model_state_and_entry_counts() {
        let mut model = RlModel::new();
        model.update("s1", "a", 1.0, "s2");
        model.update("s1", "b", 0.5, "s2");
        model.update("s2", "c", 0.3, "s3");
        assert_eq!(model.state_count(), 2);
        assert_eq!(model.total_entries(), 3);
    }

    // -- EditPattern tests --

    #[test]
    fn test_pattern_confidence() {
        let pattern = EditPattern {
            id: "p1".into(),
            name: "test".into(),
            sequence: vec![EditAction::Insert("x".into())],
            frequency: 10,
            avg_reward: 0.8,
            last_seen: 0,
        };
        let conf = pattern.confidence();
        assert!(conf > 0.0);
        assert!(conf <= 1.0);
    }

    #[test]
    fn test_pattern_confidence_low_freq() {
        let pattern = EditPattern {
            id: "p1".into(),
            name: "test".into(),
            sequence: vec![],
            frequency: 1,
            avg_reward: 0.0,
            last_seen: 0,
        };
        assert!(pattern.confidence() < 0.5);
    }

    // -- EditPredictor tests --

    #[test]
    fn test_predictor_new() {
        let pred = EditPredictor::new();
        assert_eq!(pred.predictions_made(), 0);
        assert_eq!(pred.history_len(), 0);
        assert_eq!(pred.pattern_count(), 0);
    }

    #[test]
    fn test_predictor_record_event() {
        let mut pred = EditPredictor::new();
        pred.record_event(make_event("main.rs", 1, EditAction::Insert("fn".into())));
        assert_eq!(pred.history_len(), 1);
    }

    #[test]
    fn test_predictor_acceptance_rate_zero() {
        let pred = EditPredictor::new();
        assert_eq!(pred.acceptance_rate(), 0.0);
    }

    #[test]
    fn test_predictor_record_outcome() {
        let mut pred = EditPredictor::new();
        let state = make_state("rs", &["insert"], 1);
        pred.record_outcome(&state, "insert", PredictionOutcome::Accepted);
        assert_eq!(pred.predictions_accepted, 1);
    }

    #[test]
    fn test_predictor_model_learns() {
        let mut pred = EditPredictor::new();
        let state = make_state("rs", &["insert", "save"], 10);
        // Train with positive outcomes
        for _ in 0..10 {
            pred.record_outcome(&state, "save", PredictionOutcome::Accepted);
        }
        let (states, entries) = pred.model_stats();
        assert!(states > 0);
        assert!(entries > 0);
    }

    #[test]
    fn test_predictor_predict_from_q_table() {
        let mut pred = EditPredictor::new();
        let state = make_state("rs", &["insert", "insert", "save"], 5);
        // Train heavily to ensure high Q-value
        for _ in 0..50 {
            pred.record_outcome(&state, "save", PredictionOutcome::Accepted);
        }
        let prediction = pred.predict(&state);
        // After many positive reinforcements, should predict save
        assert!(prediction.is_some());
    }

    #[test]
    fn test_predictor_no_prediction_empty() {
        let mut pred = EditPredictor::new();
        let state = make_state("rs", &[], 1);
        assert!(pred.predict(&state).is_none());
    }

    #[test]
    fn test_predictor_history_limit() {
        let mut pred = EditPredictor::new();
        pred.max_history = 5;
        for i in 0..10 {
            pred.record_event(make_event("a.rs", i, EditAction::Insert(format!("{}", i))));
        }
        assert_eq!(pred.history_len(), 5);
    }

    #[test]
    fn test_predictor_decay_exploration() {
        let mut pred = EditPredictor::new();
        let initial = pred.exploration_rate();
        pred.decay_exploration(0.5);
        assert!(pred.exploration_rate() < initial);
    }

    // -- Helper function tests --

    #[test]
    fn test_sigmoid() {
        assert!((sigmoid(0.0) - 0.5).abs() < f64::EPSILON);
        assert!(sigmoid(10.0) > 0.99);
        assert!(sigmoid(-10.0) < 0.01);
    }

    #[test]
    fn test_action_str_to_edit_action() {
        assert_eq!(action_str_to_edit_action("insert").as_str(), "insert");
        assert_eq!(action_str_to_edit_action("delete").as_str(), "delete");
        assert_eq!(action_str_to_edit_action("undo").as_str(), "undo");
        assert_eq!(action_str_to_edit_action("save").as_str(), "save");
    }

    #[test]
    fn test_count_subsequence_occurrences() {
        let events = vec![
            make_event("a.rs", 1, EditAction::Insert("x".into())),
            make_event("a.rs", 2, EditAction::Save),
            make_event("a.rs", 3, EditAction::Insert("y".into())),
            make_event("a.rs", 4, EditAction::Save),
        ];
        let subseq = vec!["insert".to_string(), "save".to_string()];
        assert_eq!(count_subsequence_occurrences(&events, &subseq), 2);
    }

    #[test]
    fn test_count_subsequence_empty() {
        assert_eq!(count_subsequence_occurrences(&[], &["insert".to_string()]), 0);
        let events = vec![make_event("a.rs", 1, EditAction::Insert("x".into()))];
        assert_eq!(count_subsequence_occurrences(&events, &[]), 0);
    }

    #[test]
    fn test_pattern_matches() {
        let pattern = vec![EditAction::Insert("x".into()), EditAction::Save];
        let recent = vec!["insert".to_string(), "save".to_string()];
        assert!(pattern_matches(&pattern, &recent));
    }

    #[test]
    fn test_pattern_matches_partial() {
        let pattern = vec![EditAction::Insert("x".into()), EditAction::Save];
        let recent = vec!["save".to_string()];
        assert!(pattern_matches(&pattern, &recent));
    }

    #[test]
    fn test_pattern_matches_empty() {
        assert!(!pattern_matches(&[], &["insert".to_string()]));
        assert!(!pattern_matches(&[EditAction::Save], &[]));
    }
}
