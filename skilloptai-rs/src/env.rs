//! `Env` / `Task` / `Grader` — the benchmark seam.
//!
//! The shared grading primitives ([`EvalTask`], [`Grader`]) live in
//! `skilllensai::metrics::eval`; this module adds the training-specific
//! [`Env`] trait (enumerate tasks + a deterministic held-out split) and a
//! day-one [`StaticEnv`] concrete impl. The full `RepoAgentEnv` (real VibeCody
//! agent jobs over this repo) lives in the vibecli bridge (`Phase 3`), which
//! is the right layer for daemon coupling; external benchmarks (SWE-bench,
//! BFCL, …) plug in as additional `Env` impls behind a later `benchmarks`
//! feature.

use serde::{Deserialize, Serialize};

use skilllensai::metrics::eval::{EvalTask, Grader};

pub use skilllensai::metrics::eval::{EvalTask as Task, Grader as Grade};

/// A benchmark: enumerate gradeable tasks and split them into train / held-out
/// partitions deterministically by `seed`.
pub trait Env: Send + Sync {
    fn tasks(&self) -> Vec<EvalTask>;
    /// Deterministic train/val split. `val_frac` in `[0,1]` is the held-out
    /// fraction; `seed` drives the shuffle so a run is replayable.
    fn split(&self, val_frac: f32, seed: u64) -> (Vec<EvalTask>, Vec<EvalTask>) {
        let mut tasks = self.tasks();
        seeded_shuffle(&mut tasks, seed);
        let val = ((tasks.len() as f32) * val_frac.clamp(0.0, 1.0)).round() as usize;
        let val = val.min(tasks.len());
        let (train, val_tasks) = tasks.split_at(tasks.len() - val);
        (train.to_vec(), val_tasks.to_vec())
    }
}

/// A simple in-memory env over a fixed task list — the day-one concrete env
/// for tests, dry-runs, and any benchmark expressible as a static task set.
pub struct StaticEnv {
    tasks: Vec<EvalTask>,
}

impl StaticEnv {
    pub fn new(tasks: Vec<EvalTask>) -> Self {
        Self { tasks }
    }

    /// Load tasks from a JSONL file (one [`EvalTask`] per non-blank line).
    pub fn from_jsonl_path(path: &std::path::Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        Self::from_jsonl(&raw)
    }

    /// Load tasks from a JSONL string (one [`EvalTask`] per non-blank line).
    pub fn from_jsonl(s: &str) -> anyhow::Result<Self> {
        let mut tasks = Vec::new();
        for (i, line) in s.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let t: EvalTask = serde_json::from_str(line)
                .map_err(|e| anyhow::anyhow!("task line {}: {e}", i + 1))?;
            tasks.push(t);
        }
        Ok(Self { tasks })
    }
}

impl Env for StaticEnv {
    fn tasks(&self) -> Vec<EvalTask> {
        self.tasks.clone()
    }
}

/// A single eval task expressed as JSONL — convenience for `from_jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: String,
    pub prompt: String,
    pub grader: Grader,
}

impl From<TaskSpec> for EvalTask {
    fn from(s: TaskSpec) -> EvalTask {
        EvalTask {
            id: s.id,
            prompt: s.prompt,
            grader: s.grader,
        }
    }
}

/// Deterministic in-place shuffle with a seeded splitmix64 PRNG. No `rand` dep,
/// fully replayable — same `seed` ⇒ same order across runs.
pub(crate) fn seeded_shuffle<T>(items: &mut [T], seed: u64) {
    if items.len() < 2 {
        return;
    }
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    let mut i = items.len();
    while i > 1 {
        i -= 1;
        state = splitmix64(state);
        let j = (state % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

/// splitmix64 — a fast, deterministic 64-bit PRNG used only for shuffling.
fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str) -> EvalTask {
        EvalTask {
            id: id.into(),
            prompt: format!("prompt-{id}"),
            grader: Grader::Contains(id.into()),
        }
    }

    #[test]
    fn split_is_deterministic_and_partitions() {
        let env = StaticEnv::new(vec![task("a"), task("b"), task("c"), task("d")]);
        let (train1, val1) = env.split(0.5, 42);
        let (train2, val2) = env.split(0.5, 42);
        assert_eq!(train1, train2);
        assert_eq!(val1, val2);
        assert_eq!(val1.len(), 2);
        assert_eq!(train1.len(), 2);
        let mut all: Vec<String> = train1
            .iter()
            .chain(val1.iter())
            .map(|t| t.id.clone())
            .collect();
        all.sort();
        assert_eq!(
            all,
            vec!["a".to_string(), "b".into(), "c".into(), "d".into()]
        );
    }

    #[test]
    fn different_seeds_usually_differ() {
        let env = StaticEnv::new(vec![
            task("a"),
            task("b"),
            task("c"),
            task("d"),
            task("e"),
            task("f"),
        ]);
        let (_, val_a) = env.split(0.5, 1);
        let (_, val_b) = env.split(0.5, 2);
        assert_ne!(val_a, val_b);
    }

    #[test]
    fn split_handles_empty_and_full() {
        let env = StaticEnv::new(vec![task("a"), task("b"), task("c")]);
        let (train, val) = env.split(0.0, 7);
        assert_eq!(val.len(), 0);
        assert_eq!(train.len(), 3);
        let (train, val) = env.split(1.0, 7);
        assert_eq!(val.len(), 3);
        assert_eq!(train.len(), 0);
    }

    #[test]
    fn jsonl_roundtrip() {
        let env = StaticEnv::new(vec![task("a"), task("b")]);
        let jsonl: String = env
            .tasks()
            .iter()
            .filter_map(|t| serde_json::to_string(t).ok())
            .collect::<Vec<_>>()
            .join("\n");
        let back = StaticEnv::from_jsonl(&jsonl).unwrap();
        assert_eq!(back.tasks().len(), 2);
        assert_eq!(back.tasks()[0].id, "a");
    }

    #[test]
    fn shuffle_is_stable_for_same_seed() {
        let mut a = vec![1u32, 2, 3, 4, 5, 6, 7, 8];
        let mut b = a.clone();
        seeded_shuffle(&mut a, 99);
        seeded_shuffle(&mut b, 99);
        assert_eq!(a, b);
        let mut sorted = a.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
