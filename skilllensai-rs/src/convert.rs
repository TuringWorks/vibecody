//! `convert` — normalise raw agent runs into the unified [`Trajectory`] schema
//! (SkillLens `convert`). No LLM required.
//!
//! The input contract is a lenient [`RawRun`] (JSON per line). Fields are
//! optional where possible so a variety of recorders — including VibeCody's
//! decision-tracing — map on with minimal massaging.

use serde::Deserialize;

use crate::model::experience::ExperiencePool;
use crate::model::trajectory::{Outcome, Role, Step, ToolCall, Trajectory};

/// A raw recorded agent run, before normalisation.
#[derive(Debug, Clone, Deserialize)]
pub struct RawRun {
    #[serde(default)]
    pub id: Option<String>,
    pub task_id: String,
    #[serde(default)]
    pub messages: Vec<RawMessage>,
    /// Explicit boolean success, if the recorder knows it.
    #[serde(default)]
    pub success: Option<bool>,
    /// Graded score in `[0.0, 1.0]`, if known.
    #[serde(default)]
    pub score: Option<f32>,
    #[serde(default)]
    pub meta: serde_json::Value,
}

/// A raw message within a [`RawRun`].
#[derive(Debug, Clone, Deserialize)]
pub struct RawMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tool: Option<RawTool>,
}

/// A raw tool invocation within a [`RawMessage`].
#[derive(Debug, Clone, Deserialize)]
pub struct RawTool {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub result: Option<String>,
}

/// Convert one raw run into a [`Trajectory`]. `index` provides a deterministic
/// id fallback when the run carries none (no randomness — replay-safe).
pub fn convert_run(raw: RawRun, index: usize) -> Trajectory {
    let id = raw
        .id
        .clone()
        .unwrap_or_else(|| format!("{}-{}", raw.task_id, index));

    let steps = raw
        .messages
        .into_iter()
        .map(|m| Step {
            role: parse_role(&m.role),
            content: m.content,
            tool: m.tool.map(|t| ToolCall {
                name: t.name,
                args: t.args,
                result: t.result,
            }),
        })
        .collect();

    let outcome = match (raw.success, raw.score) {
        (Some(true), _) => Outcome::Success,
        (Some(false), _) => Outcome::Failure,
        (None, Some(s)) if s >= 1.0 => Outcome::Success,
        (None, Some(s)) if s <= 0.0 => Outcome::Failure,
        (None, Some(s)) => Outcome::Partial { score: s },
        (None, None) => Outcome::Partial { score: 0.0 },
    };

    Trajectory {
        id,
        task_id: raw.task_id,
        steps,
        outcome,
        score: raw.score,
        meta: raw.meta,
    }
}

/// Convert a JSONL blob (one [`RawRun`] per non-blank line) into a pool.
pub fn convert_jsonl(input: &str) -> anyhow::Result<ExperiencePool> {
    let mut pool = ExperiencePool::new();
    for (i, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let raw: RawRun =
            serde_json::from_str(line).map_err(|e| anyhow::anyhow!("line {}: {e}", i + 1))?;
        pool.push(convert_run(raw, i));
    }
    Ok(pool)
}

fn parse_role(s: &str) -> Role {
    match s.trim().to_lowercase().as_str() {
        "system" => Role::System,
        "assistant" | "ai" | "model" => Role::Assistant,
        "tool" | "function" => Role::Tool,
        _ => Role::User,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_basic_run() {
        let line = r#"{"task_id":"t1","success":true,"messages":[{"role":"user","content":"hi"},{"role":"assistant","content":"yo"}]}"#;
        let pool = convert_jsonl(line).unwrap();
        assert_eq!(pool.len(), 1);
        let t = &pool.trajectories[0];
        assert_eq!(t.task_id, "t1");
        assert_eq!(t.id, "t1-0"); // deterministic id fallback
        assert!(t.is_success());
        assert_eq!(t.steps.len(), 2);
        assert_eq!(t.steps[0].role, Role::User);
        assert_eq!(t.steps[1].role, Role::Assistant);
        assert_eq!(t.intent(), Some("hi"));
    }

    #[test]
    fn partial_score_and_roundtrip() {
        let line = r#"{"task_id":"t2","score":0.5,"messages":[]}"#;
        let pool = convert_jsonl(line).unwrap();
        let t = &pool.trajectories[0];
        assert!(matches!(t.outcome, Outcome::Partial { .. }));
        assert_eq!(t.score_value(), 0.5);
        // JSONL round-trips.
        let back = ExperiencePool::from_jsonl(&pool.to_jsonl()).unwrap();
        assert_eq!(back.trajectories, pool.trajectories);
    }

    #[test]
    fn blank_lines_skipped_and_errors_located() {
        let ok = "\n\n{\"task_id\":\"t\"}\n";
        assert_eq!(convert_jsonl(ok).unwrap().len(), 1);
        let bad = "{not json}";
        assert!(convert_jsonl(bad).is_err());
    }
}
