//! `ExperiencePool` — a collection of trajectories awaiting extraction/scoring.

use serde::{Deserialize, Serialize};

use crate::model::trajectory::Trajectory;

/// A bag of trajectories with JSONL (de)serialisation helpers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExperiencePool {
    pub trajectories: Vec<Trajectory>,
}

impl ExperiencePool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, t: Trajectory) {
        self.trajectories.push(t);
    }

    pub fn len(&self) -> usize {
        self.trajectories.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trajectories.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Trajectory> {
        self.trajectories.iter()
    }

    /// Only the successful trajectories — the "positive" experience.
    pub fn successes(&self) -> impl Iterator<Item = &Trajectory> {
        self.trajectories.iter().filter(|t| t.is_success())
    }

    /// One JSON object per line.
    pub fn to_jsonl(&self) -> String {
        self.trajectories
            .iter()
            .filter_map(|t| serde_json::to_string(t).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse a JSONL blob (one [`Trajectory`] per non-blank line).
    pub fn from_jsonl(s: &str) -> anyhow::Result<Self> {
        let mut pool = Self::new();
        for (i, line) in s.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let t: Trajectory =
                serde_json::from_str(line).map_err(|e| anyhow::anyhow!("line {}: {e}", i + 1))?;
            pool.push(t);
        }
        Ok(pool)
    }
}
