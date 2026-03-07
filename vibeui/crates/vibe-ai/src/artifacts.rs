//! Artifacts system — structured, inspectable deliverables produced by agent runs.
//!
//! Agents produce `AgentArtifact`s alongside text responses. Each artifact is a
//! typed, annotatable unit that the user can inspect, comment on, and feed back
//! into the agent's next context window.
//!
//! # Artifact lifecycle
//!
//! 1. Agent produces an artifact (e.g. writes a file → `FileChange` artifact)
//! 2. Artifact is emitted via `AgentEvent::Artifact`
//! 3. UI renders artifact as a rich card
//! 4. User may add annotations (feedback comments)
//! 5. On the next agent invocation, pending annotations are injected as context

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// Re-export PlanStep for ImplementationPlan artifact
pub use crate::planner::PlanStep;

// ── Artifact variants ─────────────────────────────────────────────────────────

/// All structured artifact types an agent can produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Artifact {
    /// A to-do list extracted from the task description.
    TaskList { items: Vec<TaskItem> },

    /// A structured implementation plan with file estimates.
    ImplementationPlan {
        steps: Vec<PlanStep>,
        files: Vec<String>,
    },

    /// A single file the agent has written or patched.
    FileChange {
        path: String,
        /// Unified diff of the change. Empty if the file is brand-new.
        diff: String,
        /// The new full content (for new files).
        content: Option<String>,
    },

    /// Output from a bash command executed by the agent.
    CommandOutput {
        command: String,
        stdout: String,
        stderr: String,
        exit_code: i32,
    },

    /// Summary of a test run.
    TestResults {
        passed: usize,
        failed: usize,
        skipped: usize,
        output: String,
    },

    /// Structured code review report (from Phase 7.5).
    ReviewReport {
        issues: Vec<ReviewIssueRef>,
        summary: String,
        score: f32,
    },

    /// Raw text artifact — free-form agent output marked as an artifact.
    Text {
        title: String,
        content: String,
    },
}

impl Artifact {
    /// Short human-readable label for the artifact type.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::TaskList { .. } => "Task List",
            Self::ImplementationPlan { .. } => "Implementation Plan",
            Self::FileChange { .. } => "File Change",
            Self::CommandOutput { .. } => "Command Output",
            Self::TestResults { .. } => "Test Results",
            Self::ReviewReport { .. } => "Code Review",
            Self::Text { .. } => "Text",
        }
    }

    /// Icon for UI rendering.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::TaskList { .. } => "✅",
            Self::ImplementationPlan { .. } => "📋",
            Self::FileChange { .. } => "📝",
            Self::CommandOutput { .. } => "⚡",
            Self::TestResults { .. } => "🧪",
            Self::ReviewReport { .. } => "🔍",
            Self::Text { .. } => "📄",
        }
    }
}

// ── Supporting types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: usize,
    pub description: String,
    pub done: bool,
    pub file: Option<String>,
}

/// Lightweight reference to a review issue (avoids importing the full review module).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssueRef {
    pub file: String,
    pub line: u32,
    pub severity: String,
    pub description: String,
}

// ── Annotation ────────────────────────────────────────────────────────────────

/// A user comment on an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    /// The annotation text.
    pub text: String,
    pub timestamp: u64,
    /// Whether the agent has already incorporated this feedback.
    pub applied: bool,
}

impl Annotation {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            timestamp: unix_now(),
            applied: false,
        }
    }
}

// ── AgentArtifact ─────────────────────────────────────────────────────────────

/// A single artifact produced during an agent run, with user annotations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentArtifact {
    /// Unique ID (UUID-style, generated at creation).
    pub id: String,
    /// Which agent step produced this artifact (for trace correlation).
    pub step: usize,
    pub artifact: Artifact,
    pub timestamp: u64,
    pub annotations: Vec<Annotation>,
}

impl AgentArtifact {
    pub fn new(step: usize, artifact: Artifact) -> Self {
        Self {
            id: generate_id(),
            step,
            artifact,
            timestamp: unix_now(),
            annotations: Vec::new(),
        }
    }

    /// Add a user annotation to this artifact.
    pub fn annotate(&mut self, text: impl Into<String>) {
        self.annotations.push(Annotation::new(text));
    }

    /// Return all unapplied annotations as a context injection string.
    ///
    /// This is appended to the agent's next context window so the agent can
    /// act on user feedback without requiring a new top-level task.
    pub fn pending_feedback(&self) -> Option<String> {
        let pending: Vec<&str> = self.annotations
            .iter()
            .filter(|a| !a.applied)
            .map(|a| a.text.as_str())
            .collect();

        if pending.is_empty() {
            None
        } else {
            let artifact_label = format!(
                "{} {} (step {})",
                self.artifact.icon(),
                self.artifact.kind_label(),
                self.step
            );
            Some(format!(
                "User feedback on artifact '{}':\n{}",
                artifact_label,
                pending.join("\n")
            ))
        }
    }

    /// Mark all annotations as applied.
    pub fn mark_annotations_applied(&mut self) {
        for ann in &mut self.annotations {
            ann.applied = true;
        }
    }
}

// ── ArtifactStore ─────────────────────────────────────────────────────────────

/// In-memory store of artifacts for a session, with JSON persistence.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ArtifactStore {
    pub artifacts: Vec<AgentArtifact>,
}

impl ArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, artifact: AgentArtifact) {
        self.artifacts.push(artifact);
    }

    /// Collect all pending (unapplied) feedback across all artifacts.
    pub fn collect_pending_feedback(&self) -> String {
        self.artifacts
            .iter()
            .filter_map(|a| a.pending_feedback())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Find an artifact by ID and add an annotation to it.
    pub fn annotate(&mut self, id: &str, text: impl Into<String>) -> bool {
        if let Some(artifact) = self.artifacts.iter_mut().find(|a| a.id == id) {
            artifact.annotate(text);
            true
        } else {
            false
        }
    }

    /// Mark all annotations on an artifact as applied.
    pub fn mark_applied(&mut self, id: &str) -> bool {
        if let Some(artifact) = self.artifacts.iter_mut().find(|a| a.id == id) {
            artifact.mark_annotations_applied();
            true
        } else {
            false
        }
    }

    /// Save to a JSON file.
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load from a JSON file.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn generate_id() -> String {
    // Simple ID: timestamp_hex + pseudo-random suffix from process ID
    let ts = unix_now();
    let pid = std::process::id();
    // Use a counter for uniqueness within the same second
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{:x}-{:x}-{:x}", ts, pid, n)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_kind_labels() {
        assert_eq!(Artifact::TaskList { items: vec![] }.kind_label(), "Task List");
        assert_eq!(Artifact::FileChange { path: "f".into(), diff: "".into(), content: None }.kind_label(), "File Change");
        assert_eq!(Artifact::CommandOutput { command: "".into(), stdout: "".into(), stderr: "".into(), exit_code: 0 }.kind_label(), "Command Output");
    }

    #[test]
    fn annotation_pending_feedback() {
        let mut artifact = AgentArtifact::new(1, Artifact::Text {
            title: "test".to_string(),
            content: "hello".to_string(),
        });
        assert!(artifact.pending_feedback().is_none());

        artifact.annotate("Please also handle the error case");
        let feedback = artifact.pending_feedback().unwrap();
        assert!(feedback.contains("Please also handle"));
        assert!(feedback.contains("Text"));

        artifact.mark_annotations_applied();
        assert!(artifact.pending_feedback().is_none());
    }

    #[test]
    fn artifact_store_collect_feedback() {
        let mut store = ArtifactStore::new();
        let mut a1 = AgentArtifact::new(1, Artifact::FileChange { path: "main.rs".into(), diff: "".into(), content: None });
        a1.annotate("Add error handling here");
        store.push(a1);

        let feedback = store.collect_pending_feedback();
        assert!(feedback.contains("Add error handling"));
    }

    #[test]
    fn artifact_store_annotate_by_id() {
        let mut store = ArtifactStore::new();
        let a = AgentArtifact::new(1, Artifact::TaskList { items: vec![] });
        let id = a.id.clone();
        store.push(a);

        let found = store.annotate(&id, "Looks good!");
        assert!(found);

        let not_found = store.annotate("nonexistent", "test");
        assert!(!not_found);
    }

    #[test]
    fn artifact_store_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifacts.json");

        let mut store = ArtifactStore::new();
        store.push(AgentArtifact::new(1, Artifact::Text { title: "T".into(), content: "C".into() }));
        store.save(&path).unwrap();

        let loaded = ArtifactStore::load(&path).unwrap();
        assert_eq!(loaded.artifacts.len(), 1);
    }

    #[test]
    fn generate_id_unique() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
    }

    // ── Artifact::icon coverage ──────────────────────────────────────────

    #[test]
    fn artifact_icon_all_variants() {
        let cases: Vec<(Artifact, &str)> = vec![
            (Artifact::TaskList { items: vec![] }, "Task List"),
            (Artifact::ImplementationPlan { steps: vec![], files: vec![] }, "Implementation Plan"),
            (Artifact::FileChange { path: "p".into(), diff: "".into(), content: None }, "File Change"),
            (Artifact::CommandOutput { command: "c".into(), stdout: "".into(), stderr: "".into(), exit_code: 0 }, "Command Output"),
            (Artifact::TestResults { passed: 1, failed: 0, skipped: 0, output: "".into() }, "Test Results"),
            (Artifact::ReviewReport { issues: vec![], summary: "".into(), score: 0.9 }, "Code Review"),
            (Artifact::Text { title: "t".into(), content: "c".into() }, "Text"),
        ];
        for (artifact, expected_label) in &cases {
            assert_eq!(artifact.kind_label(), *expected_label);
            // icon() should return a non-empty string for every variant
            assert!(!artifact.icon().is_empty(), "icon for {} should not be empty", expected_label);
        }
    }

    // ── Artifact serde roundtrip (tagged enum) ───────────────────────────

    #[test]
    fn artifact_task_list_serde() {
        let artifact = Artifact::TaskList {
            items: vec![
                TaskItem { id: 1, description: "Do thing".into(), done: false, file: Some("main.rs".into()) },
                TaskItem { id: 2, description: "Done thing".into(), done: true, file: None },
            ],
        };
        let json = serde_json::to_string(&artifact).unwrap();
        assert!(json.contains("\"type\":\"task_list\""));
        let back: Artifact = serde_json::from_str(&json).unwrap();
        if let Artifact::TaskList { items } = back {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].description, "Do thing");
            assert!(items[1].done);
        } else {
            panic!("Expected TaskList variant");
        }
    }

    #[test]
    fn artifact_test_results_serde() {
        let artifact = Artifact::TestResults {
            passed: 10,
            failed: 2,
            skipped: 1,
            output: "test output".into(),
        };
        let json = serde_json::to_string(&artifact).unwrap();
        assert!(json.contains("\"type\":\"test_results\""));
        let back: Artifact = serde_json::from_str(&json).unwrap();
        if let Artifact::TestResults { passed, failed, skipped, .. } = back {
            assert_eq!(passed, 10);
            assert_eq!(failed, 2);
            assert_eq!(skipped, 1);
        } else {
            panic!("Expected TestResults variant");
        }
    }

    #[test]
    fn artifact_review_report_serde() {
        let artifact = Artifact::ReviewReport {
            issues: vec![ReviewIssueRef {
                file: "src/lib.rs".into(),
                line: 42,
                severity: "warning".into(),
                description: "Unused variable".into(),
            }],
            summary: "One issue found".into(),
            score: 0.85,
        };
        let json = serde_json::to_string(&artifact).unwrap();
        let back: Artifact = serde_json::from_str(&json).unwrap();
        if let Artifact::ReviewReport { issues, summary, score } = back {
            assert_eq!(issues.len(), 1);
            assert_eq!(issues[0].line, 42);
            assert_eq!(summary, "One issue found");
            assert!((score - 0.85).abs() < f32::EPSILON);
        } else {
            panic!("Expected ReviewReport variant");
        }
    }

    #[test]
    fn artifact_command_output_serde() {
        let artifact = Artifact::CommandOutput {
            command: "cargo test".into(),
            stdout: "all passed".into(),
            stderr: "".into(),
            exit_code: 0,
        };
        let json = serde_json::to_string(&artifact).unwrap();
        let back: Artifact = serde_json::from_str(&json).unwrap();
        if let Artifact::CommandOutput { command, exit_code, .. } = back {
            assert_eq!(command, "cargo test");
            assert_eq!(exit_code, 0);
        } else {
            panic!("Expected CommandOutput variant");
        }
    }

    // ── ArtifactStore edge cases ─────────────────────────────────────────

    #[test]
    fn artifact_store_mark_applied_nonexistent() {
        let mut store = ArtifactStore::new();
        assert!(!store.mark_applied("nonexistent"));
    }

    #[test]
    fn artifact_store_mark_applied_clears_pending() {
        let mut store = ArtifactStore::new();
        let mut a = AgentArtifact::new(1, Artifact::Text { title: "t".into(), content: "c".into() });
        a.annotate("feedback 1");
        a.annotate("feedback 2");
        let id = a.id.clone();
        store.push(a);

        // Before mark_applied, there should be pending feedback
        assert!(!store.collect_pending_feedback().is_empty());

        // After mark_applied, no pending feedback
        assert!(store.mark_applied(&id));
        assert!(store.collect_pending_feedback().is_empty());
    }

    #[test]
    fn artifact_store_default_is_empty() {
        let store = ArtifactStore::default();
        assert!(store.artifacts.is_empty());
        assert!(store.collect_pending_feedback().is_empty());
    }

    // ── Annotation ───────────────────────────────────────────────────────

    #[test]
    fn annotation_new_defaults_not_applied() {
        let ann = Annotation::new("test feedback");
        assert_eq!(ann.text, "test feedback");
        assert!(!ann.applied);
        assert!(ann.timestamp > 0);
    }

    #[test]
    fn annotation_serde_roundtrip() {
        let ann = Annotation::new("roundtrip test");
        let json = serde_json::to_string(&ann).unwrap();
        let back: Annotation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text, "roundtrip test");
        assert!(!back.applied);
    }

    // ── TaskItem ─────────────────────────────────────────────────────────

    #[test]
    fn task_item_serde_roundtrip() {
        let item = TaskItem {
            id: 5,
            description: "Write tests".into(),
            done: true,
            file: Some("tests/mod.rs".into()),
        };
        let json = serde_json::to_string(&item).unwrap();
        let back: TaskItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 5);
        assert!(back.done);
        assert_eq!(back.file.as_deref(), Some("tests/mod.rs"));
    }
}
