
use std::time::SystemTime;

/// Reinforcement learning for edit suggestions — predict what the user will edit next.

#[derive(Debug, Clone, PartialEq)]
pub enum EditType {
    Insert,
    Delete,
    Replace,
    Move,
    Rename,
    Refactor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditContext {
    SameFunction,
    SameFile,
    RelatedFile,
    TestFile,
    ConfigFile,
}

#[derive(Debug, Clone)]
pub struct EditPrediction {
    pub id: String,
    pub file_path: String,
    pub line: usize,
    pub edit_type: EditType,
    pub description: String,
    pub suggested_content: Option<String>,
    pub confidence: f64,
    pub context: EditContext,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub struct EditHistory {
    pub edits: Vec<RecordedEdit>,
    pub max_history: usize,
}

#[derive(Debug, Clone)]
pub struct RecordedEdit {
    pub file_path: String,
    pub line: usize,
    pub edit_type: EditType,
    pub before: String,
    pub after: String,
    pub timestamp: SystemTime,
    pub accepted: bool,
}

#[derive(Debug, Clone)]
pub struct EditPattern {
    pub pattern_type: String,
    pub trigger_edit: EditType,
    pub predicted_edit: EditType,
    pub predicted_context: EditContext,
    pub weight: f64,
    pub occurrences: usize,
}

#[derive(Debug, Clone)]
pub struct NextEditPredictor {
    pub history: EditHistory,
    pub patterns: Vec<EditPattern>,
    pub config: PredictorConfig,
    pub stats: PredictorStats,
}

#[derive(Debug, Clone)]
pub struct PredictorConfig {
    pub enabled: bool,
    pub max_predictions: usize,
    pub min_confidence: f64,
    pub learn_from_accepts: bool,
    pub pattern_decay: f64,
}

#[derive(Debug, Clone)]
pub struct PredictorStats {
    pub total_predictions: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub accuracy: f64,
}

impl Default for PredictorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_predictions: 3,
            min_confidence: 0.5,
            learn_from_accepts: true,
            pattern_decay: 0.95,
        }
    }
}

impl EditPrediction {
    pub fn new(file_path: &str, line: usize, edit_type: EditType, description: &str) -> Self {
        let id = format!(
            "pred-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                % 1_000_000
        );
        Self {
            id,
            file_path: file_path.to_string(),
            line,
            edit_type,
            description: description.to_string(),
            suggested_content: None,
            confidence: 0.5,
            context: EditContext::SameFile,
            reasoning: String::new(),
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.suggested_content = Some(content.to_string());
        self
    }

    pub fn with_context(mut self, context: EditContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_reasoning(mut self, reasoning: &str) -> Self {
        self.reasoning = reasoning.to_string();
        self
    }
}

impl EditHistory {
    pub fn new() -> Self {
        Self {
            edits: Vec::new(),
            max_history: 1000,
        }
    }

    pub fn with_max(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    pub fn record(&mut self, edit: RecordedEdit) {
        self.edits.push(edit);
        if self.edits.len() > self.max_history {
            self.cleanup();
        }
    }

    pub fn recent(&self, n: usize) -> Vec<&RecordedEdit> {
        let start = if self.edits.len() > n {
            self.edits.len() - n
        } else {
            0
        };
        self.edits[start..].iter().collect()
    }

    pub fn edits_in_file(&self, path: &str) -> Vec<&RecordedEdit> {
        self.edits.iter().filter(|e| e.file_path == path).collect()
    }

    pub fn most_edited_files(&self, n: usize) -> Vec<(&str, usize)> {
        let mut counts: Vec<(&str, usize)> = Vec::new();
        for edit in &self.edits {
            if let Some(entry) = counts.iter_mut().find(|(p, _)| *p == edit.file_path.as_str()) {
                entry.1 += 1;
            } else {
                counts.push((&edit.file_path, 1));
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        counts.truncate(n);
        counts
    }

    pub fn cleanup(&mut self) {
        if self.edits.len() > self.max_history {
            let drain_count = self.edits.len() - self.max_history;
            self.edits.drain(..drain_count);
        }
    }
}

impl RecordedEdit {
    pub fn new(
        file_path: &str,
        line: usize,
        edit_type: EditType,
        before: &str,
        after: &str,
    ) -> Self {
        Self {
            file_path: file_path.to_string(),
            line,
            edit_type,
            before: before.to_string(),
            after: after.to_string(),
            timestamp: SystemTime::now(),
            accepted: false,
        }
    }

    pub fn with_accepted(mut self, accepted: bool) -> Self {
        self.accepted = accepted;
        self
    }
}

impl EditPattern {
    pub fn default_patterns() -> Vec<EditPattern> {
        vec![
            EditPattern {
                pattern_type: "rename_follow_up".to_string(),
                trigger_edit: EditType::Rename,
                predicted_edit: EditType::Rename,
                predicted_context: EditContext::SameFile,
                weight: 0.8,
                occurrences: 0,
            },
            EditPattern {
                pattern_type: "test_after_impl".to_string(),
                trigger_edit: EditType::Insert,
                predicted_edit: EditType::Insert,
                predicted_context: EditContext::TestFile,
                weight: 0.7,
                occurrences: 0,
            },
            EditPattern {
                pattern_type: "refactor_cascade".to_string(),
                trigger_edit: EditType::Refactor,
                predicted_edit: EditType::Replace,
                predicted_context: EditContext::RelatedFile,
                weight: 0.65,
                occurrences: 0,
            },
            EditPattern {
                pattern_type: "delete_cleanup".to_string(),
                trigger_edit: EditType::Delete,
                predicted_edit: EditType::Delete,
                predicted_context: EditContext::SameFile,
                weight: 0.6,
                occurrences: 0,
            },
            EditPattern {
                pattern_type: "config_after_feature".to_string(),
                trigger_edit: EditType::Insert,
                predicted_edit: EditType::Replace,
                predicted_context: EditContext::ConfigFile,
                weight: 0.55,
                occurrences: 0,
            },
            EditPattern {
                pattern_type: "move_then_replace".to_string(),
                trigger_edit: EditType::Move,
                predicted_edit: EditType::Replace,
                predicted_context: EditContext::SameFunction,
                weight: 0.5,
                occurrences: 0,
            },
            EditPattern {
                pattern_type: "insert_then_refactor".to_string(),
                trigger_edit: EditType::Insert,
                predicted_edit: EditType::Refactor,
                predicted_context: EditContext::SameFunction,
                weight: 0.45,
                occurrences: 0,
            },
            EditPattern {
                pattern_type: "rename_in_related".to_string(),
                trigger_edit: EditType::Rename,
                predicted_edit: EditType::Rename,
                predicted_context: EditContext::RelatedFile,
                weight: 0.75,
                occurrences: 0,
            },
        ]
    }

    pub fn matches(&self, recent_edit: &RecordedEdit) -> bool {
        self.trigger_edit == recent_edit.edit_type
    }

    pub fn reinforce(&mut self) {
        self.weight = (self.weight + 0.05).min(1.0);
        self.occurrences += 1;
    }

    pub fn decay(&mut self, factor: f64) {
        self.weight *= factor;
        if self.weight < 0.01 {
            self.weight = 0.01;
        }
    }
}

impl PredictorStats {
    pub fn new() -> Self {
        Self {
            total_predictions: 0,
            accepted: 0,
            rejected: 0,
            accuracy: 0.0,
        }
    }

    pub fn update(&mut self, accepted: bool) {
        self.total_predictions += 1;
        if accepted {
            self.accepted += 1;
        } else {
            self.rejected += 1;
        }
        if self.total_predictions > 0 {
            self.accuracy = self.accepted as f64 / self.total_predictions as f64;
        }
    }
}

impl NextEditPredictor {
    pub fn new() -> Self {
        Self {
            history: EditHistory::new(),
            patterns: EditPattern::default_patterns(),
            config: PredictorConfig::default(),
            stats: PredictorStats::new(),
        }
    }

    pub fn with_config(config: PredictorConfig) -> Self {
        Self {
            history: EditHistory::new(),
            patterns: EditPattern::default_patterns(),
            config,
            stats: PredictorStats::new(),
        }
    }

    pub fn predict(&self, current_file: &str, current_line: usize) -> Vec<EditPrediction> {
        if !self.config.enabled {
            return Vec::new();
        }

        let recent = self.history.recent(5);
        if recent.is_empty() {
            return Vec::new();
        }

        let mut predictions = Vec::new();

        for pattern in &self.patterns {
            if pattern.weight < self.config.min_confidence {
                continue;
            }

            // Check if any recent edit matches this pattern's trigger
            let matched = recent.iter().any(|e| pattern.matches(e));
            if !matched {
                continue;
            }

            let (pred_file, pred_line) = match pattern.predicted_context {
                EditContext::SameFile => (current_file.to_string(), current_line + 5),
                EditContext::SameFunction => (current_file.to_string(), current_line + 1),
                EditContext::TestFile => {
                    let test_file = if current_file.contains("/src/") {
                        current_file.replace("/src/", "/tests/")
                    } else {
                        format!("{}_test", current_file)
                    };
                    (test_file, 1)
                }
                EditContext::RelatedFile => {
                    // Suggest edits in the same directory
                    (current_file.to_string(), current_line)
                }
                EditContext::ConfigFile => ("config.toml".to_string(), 1),
            };

            let prediction = EditPrediction::new(
                &pred_file,
                pred_line,
                pattern.predicted_edit.clone(),
                &format!("Predicted by pattern: {}", pattern.pattern_type),
            )
            .with_confidence(pattern.weight)
            .with_context(pattern.predicted_context.clone())
            .with_reasoning(&format!(
                "Pattern '{}' triggered with weight {:.2}",
                pattern.pattern_type, pattern.weight
            ));

            predictions.push(prediction);
        }

        // Sort by confidence descending
        predictions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        predictions.truncate(self.config.max_predictions);
        predictions
    }

    pub fn record_edit(&mut self, edit: RecordedEdit) {
        self.history.record(edit);
    }

    pub fn accept_prediction(&mut self, id: &str) {
        self.stats.update(true);

        if self.config.learn_from_accepts {
            // Reinforce patterns that could have generated this prediction
            for pattern in &mut self.patterns {
                if pattern.weight >= self.config.min_confidence {
                    // Simple heuristic: reinforce patterns with high weight
                    // In production, we'd track which pattern generated which prediction
                    pattern.reinforce();
                }
            }
        }

        // Store the prediction ID so we know it was accepted
        let _ = id; // Used for tracking in a full implementation
    }

    pub fn reject_prediction(&mut self, id: &str) {
        self.stats.update(false);

        // Decay all patterns slightly
        let decay = self.config.pattern_decay;
        for pattern in &mut self.patterns {
            pattern.decay(decay);
        }

        let _ = id;
    }

    pub fn learn_pattern(
        &mut self,
        trigger: EditType,
        predicted: EditType,
    ) -> EditPattern {
        let pattern = EditPattern {
            pattern_type: format!("learned_{}", self.patterns.len()),
            trigger_edit: trigger,
            predicted_edit: predicted,
            predicted_context: EditContext::SameFile,
            weight: 0.5,
            occurrences: 1,
        };
        self.patterns.push(pattern.clone());
        pattern
    }

    pub fn accuracy(&self) -> f64 {
        self.stats.accuracy
    }

    pub fn reset_stats(&mut self) {
        self.stats = PredictorStats::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_prediction_new() {
        let pred = EditPrediction::new("src/main.rs", 10, EditType::Insert, "Add function");
        assert_eq!(pred.file_path, "src/main.rs");
        assert_eq!(pred.line, 10);
        assert_eq!(pred.edit_type, EditType::Insert);
        assert_eq!(pred.description, "Add function");
        assert!(pred.id.starts_with("pred-"));
        assert_eq!(pred.confidence, 0.5);
        assert!(pred.suggested_content.is_none());
    }

    #[test]
    fn test_edit_prediction_with_confidence() {
        let pred = EditPrediction::new("f.rs", 1, EditType::Delete, "d")
            .with_confidence(0.9);
        assert_eq!(pred.confidence, 0.9);
    }

    #[test]
    fn test_edit_prediction_confidence_clamped() {
        let pred = EditPrediction::new("f.rs", 1, EditType::Delete, "d")
            .with_confidence(1.5);
        assert_eq!(pred.confidence, 1.0);

        let pred2 = EditPrediction::new("f.rs", 1, EditType::Delete, "d")
            .with_confidence(-0.5);
        assert_eq!(pred2.confidence, 0.0);
    }

    #[test]
    fn test_edit_prediction_with_content() {
        let pred = EditPrediction::new("f.rs", 1, EditType::Replace, "d")
            .with_content("new content");
        assert_eq!(pred.suggested_content, Some("new content".to_string()));
    }

    #[test]
    fn test_edit_history_new() {
        let history = EditHistory::new();
        assert!(history.edits.is_empty());
        assert_eq!(history.max_history, 1000);
    }

    #[test]
    fn test_edit_history_record() {
        let mut history = EditHistory::new();
        let edit = RecordedEdit::new("f.rs", 10, EditType::Insert, "", "let x = 1;");
        history.record(edit);
        assert_eq!(history.edits.len(), 1);
    }

    #[test]
    fn test_edit_history_recent() {
        let mut history = EditHistory::new();
        for i in 0..10 {
            let edit = RecordedEdit::new("f.rs", i, EditType::Insert, "", "line");
            history.record(edit);
        }
        let recent = history.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].line, 7);
        assert_eq!(recent[2].line, 9);
    }

    #[test]
    fn test_edit_history_recent_more_than_available() {
        let mut history = EditHistory::new();
        let edit = RecordedEdit::new("f.rs", 1, EditType::Insert, "", "x");
        history.record(edit);
        let recent = history.recent(10);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_edit_history_edits_in_file() {
        let mut history = EditHistory::new();
        history.record(RecordedEdit::new("a.rs", 1, EditType::Insert, "", "x"));
        history.record(RecordedEdit::new("b.rs", 1, EditType::Insert, "", "y"));
        history.record(RecordedEdit::new("a.rs", 2, EditType::Delete, "z", ""));
        let edits = history.edits_in_file("a.rs");
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn test_edit_history_most_edited_files() {
        let mut history = EditHistory::new();
        for _ in 0..5 {
            history.record(RecordedEdit::new("a.rs", 1, EditType::Insert, "", "x"));
        }
        for _ in 0..3 {
            history.record(RecordedEdit::new("b.rs", 1, EditType::Insert, "", "y"));
        }
        history.record(RecordedEdit::new("c.rs", 1, EditType::Insert, "", "z"));
        let top = history.most_edited_files(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "a.rs");
        assert_eq!(top[0].1, 5);
        assert_eq!(top[1].0, "b.rs");
        assert_eq!(top[1].1, 3);
    }

    #[test]
    fn test_edit_history_cleanup() {
        let mut history = EditHistory::new().with_max(5);
        for i in 0..10 {
            history.record(RecordedEdit::new("f.rs", i, EditType::Insert, "", "x"));
        }
        assert_eq!(history.edits.len(), 5);
        assert_eq!(history.edits[0].line, 5); // oldest trimmed
    }

    #[test]
    fn test_edit_pattern_default_patterns() {
        let patterns = EditPattern::default_patterns();
        assert!(patterns.len() >= 8);
        // Verify all have reasonable weights
        for p in &patterns {
            assert!(p.weight > 0.0 && p.weight <= 1.0);
        }
    }

    #[test]
    fn test_edit_pattern_matches() {
        let pattern = &EditPattern::default_patterns()[0]; // rename_follow_up
        let edit = RecordedEdit::new("f.rs", 1, EditType::Rename, "old", "new");
        assert!(pattern.matches(&edit));
        let edit2 = RecordedEdit::new("f.rs", 1, EditType::Insert, "", "x");
        assert!(!pattern.matches(&edit2));
    }

    #[test]
    fn test_edit_pattern_reinforce() {
        let mut pattern = EditPattern::default_patterns()[0].clone();
        let original_weight = pattern.weight;
        pattern.reinforce();
        assert!(pattern.weight > original_weight);
        assert_eq!(pattern.occurrences, 1);
    }

    #[test]
    fn test_edit_pattern_reinforce_cap() {
        let mut pattern = EditPattern::default_patterns()[0].clone();
        pattern.weight = 0.98;
        pattern.reinforce();
        assert!(pattern.weight <= 1.0);
    }

    #[test]
    fn test_edit_pattern_decay() {
        let mut pattern = EditPattern::default_patterns()[0].clone();
        let original_weight = pattern.weight;
        pattern.decay(0.9);
        assert!(pattern.weight < original_weight);
    }

    #[test]
    fn test_edit_pattern_decay_minimum() {
        let mut pattern = EditPattern::default_patterns()[0].clone();
        pattern.weight = 0.001;
        pattern.decay(0.001);
        assert!(pattern.weight >= 0.01);
    }

    #[test]
    fn test_predictor_stats_new() {
        let stats = PredictorStats::new();
        assert_eq!(stats.total_predictions, 0);
        assert_eq!(stats.accepted, 0);
        assert_eq!(stats.rejected, 0);
        assert_eq!(stats.accuracy, 0.0);
    }

    #[test]
    fn test_predictor_stats_update() {
        let mut stats = PredictorStats::new();
        stats.update(true);
        stats.update(true);
        stats.update(false);
        assert_eq!(stats.total_predictions, 3);
        assert_eq!(stats.accepted, 2);
        assert_eq!(stats.rejected, 1);
        assert!((stats.accuracy - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_next_edit_predictor_new() {
        let predictor = NextEditPredictor::new();
        assert!(predictor.config.enabled);
        assert_eq!(predictor.config.max_predictions, 3);
        assert!(!predictor.patterns.is_empty());
    }

    #[test]
    fn test_predict_empty_history() {
        let predictor = NextEditPredictor::new();
        let preds = predictor.predict("f.rs", 10);
        assert!(preds.is_empty());
    }

    #[test]
    fn test_predict_disabled() {
        let mut config = PredictorConfig::default();
        config.enabled = false;
        let mut predictor = NextEditPredictor::with_config(config);
        predictor.record_edit(RecordedEdit::new("f.rs", 1, EditType::Rename, "a", "b"));
        let preds = predictor.predict("f.rs", 10);
        assert!(preds.is_empty());
    }

    #[test]
    fn test_predict_after_rename() {
        let mut predictor = NextEditPredictor::new();
        predictor.record_edit(RecordedEdit::new("f.rs", 1, EditType::Rename, "old", "new"));
        let preds = predictor.predict("f.rs", 10);
        assert!(!preds.is_empty());
        // Should have predictions triggered by rename patterns
        assert!(preds.len() <= predictor.config.max_predictions);
    }

    #[test]
    fn test_predict_sorted_by_confidence() {
        let mut predictor = NextEditPredictor::new();
        predictor.record_edit(RecordedEdit::new("f.rs", 1, EditType::Insert, "", "x"));
        let preds = predictor.predict("f.rs", 10);
        if preds.len() >= 2 {
            for i in 0..preds.len() - 1 {
                assert!(preds[i].confidence >= preds[i + 1].confidence);
            }
        }
    }

    #[test]
    fn test_accept_prediction() {
        let mut predictor = NextEditPredictor::new();
        predictor.accept_prediction("pred-1");
        assert_eq!(predictor.stats.accepted, 1);
        assert_eq!(predictor.stats.total_predictions, 1);
        assert_eq!(predictor.accuracy(), 1.0);
    }

    #[test]
    fn test_reject_prediction() {
        let mut predictor = NextEditPredictor::new();
        predictor.reject_prediction("pred-1");
        assert_eq!(predictor.stats.rejected, 1);
        assert_eq!(predictor.accuracy(), 0.0);
    }

    #[test]
    fn test_learn_pattern() {
        let mut predictor = NextEditPredictor::new();
        let initial_count = predictor.patterns.len();
        let pattern = predictor.learn_pattern(EditType::Delete, EditType::Insert);
        assert_eq!(predictor.patterns.len(), initial_count + 1);
        assert_eq!(pattern.trigger_edit, EditType::Delete);
        assert_eq!(pattern.predicted_edit, EditType::Insert);
        assert_eq!(pattern.weight, 0.5);
    }

    #[test]
    fn test_reset_stats() {
        let mut predictor = NextEditPredictor::new();
        predictor.accept_prediction("p1");
        predictor.reject_prediction("p2");
        predictor.reset_stats();
        assert_eq!(predictor.stats.total_predictions, 0);
        assert_eq!(predictor.stats.accepted, 0);
        assert_eq!(predictor.stats.rejected, 0);
    }

    #[test]
    fn test_recorded_edit_new() {
        let edit = RecordedEdit::new("f.rs", 5, EditType::Replace, "old", "new");
        assert_eq!(edit.file_path, "f.rs");
        assert_eq!(edit.line, 5);
        assert_eq!(edit.before, "old");
        assert_eq!(edit.after, "new");
        assert!(!edit.accepted);
    }

    #[test]
    fn test_recorded_edit_with_accepted() {
        let edit = RecordedEdit::new("f.rs", 5, EditType::Replace, "old", "new")
            .with_accepted(true);
        assert!(edit.accepted);
    }

    #[test]
    fn test_prediction_with_context_and_reasoning() {
        let pred = EditPrediction::new("f.rs", 1, EditType::Insert, "test")
            .with_context(EditContext::TestFile)
            .with_reasoning("Test follows implementation");
        assert_eq!(pred.context, EditContext::TestFile);
        assert_eq!(pred.reasoning, "Test follows implementation");
    }
}
