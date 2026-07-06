//! Unified Trajectory schema — the shared schema both crates speak.
//!
//! SkillLens `convert` produces these; `skilloptai-rs` rollouts also emit them.
//! Serde-serialisable so trajectories round-trip through JSONL and the store.

use serde::{Deserialize, Serialize};

/// Who produced a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A tool/function invocation attached to a step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// One turn in a trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub role: Role,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolCall>,
}

/// Terminal result of a trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Outcome {
    Success,
    Failure,
    /// Graded outcome in `[0.0, 1.0]`.
    Partial {
        score: f32,
    },
}

/// A normalised agent run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trajectory {
    pub id: String,
    pub task_id: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    pub outcome: Outcome,
    /// Task success signal in `[0.0, 1.0]`, when known separately from `outcome`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    /// Free-form provenance (provider, model, wall-time, tokens…).
    #[serde(default)]
    pub meta: serde_json::Value,
}

impl Trajectory {
    /// True when the run terminated in [`Outcome::Success`].
    pub fn is_success(&self) -> bool {
        matches!(self.outcome, Outcome::Success)
    }

    /// A single scalar score in `[0.0, 1.0]`: the explicit `score` if present,
    /// otherwise derived from the `outcome`.
    pub fn score_value(&self) -> f32 {
        if let Some(s) = self.score {
            return s.clamp(0.0, 1.0);
        }
        match self.outcome {
            Outcome::Success => 1.0,
            Outcome::Failure => 0.0,
            Outcome::Partial { score } => score.clamp(0.0, 1.0),
        }
    }

    /// The first `user` step's content — the observed "intent" of the run.
    pub fn intent(&self) -> Option<&str> {
        self.steps
            .iter()
            .find(|s| s.role == Role::User)
            .map(|s| s.content.as_str())
    }
}
