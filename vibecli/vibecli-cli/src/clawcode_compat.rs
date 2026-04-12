//! ClawCode compatibility layer — worker registry, task lifecycle, and REPL command mapping.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── ClawCodeTaskType ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClawCodeTaskType {
    CodeEdit,
    CodeReview,
    TestGen,
    Explain,
    Refactor,
    Custom(String),
}

// ─── ClawCodeTaskStatus ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClawCodeTaskStatus {
    Queued,
    Running,
    Completed(String),
    Failed(String),
}

// ─── ClawCodeTask ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawCodeTask {
    pub task_id: String,
    pub task_type: ClawCodeTaskType,
    pub payload: String,
    pub status: ClawCodeTaskStatus,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

// ─── WorkerCapability ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCapability {
    pub supported_languages: Vec<String>,
    pub available_tools: Vec<String>,
    pub context_window_k: u32,
    pub providers: Vec<String>,
}

// ─── WorkerRegistration ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegistration {
    pub worker_id: String,
    pub name: String,
    pub version: String,
    pub capabilities: WorkerCapability,
    pub endpoint: String,
    pub registered_at_ms: u64,
}

// ─── WorkerRegistry ──────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct WorkerRegistry {
    workers: HashMap<String, WorkerRegistration>,
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }

    /// Registers a worker. Returns Err if `worker_id` already exists.
    pub fn register(&mut self, reg: WorkerRegistration) -> Result<(), String> {
        if self.workers.contains_key(&reg.worker_id) {
            return Err(format!("worker '{}' is already registered", reg.worker_id));
        }
        self.workers.insert(reg.worker_id.clone(), reg);
        Ok(())
    }

    /// Removes a worker by ID. Returns true if it existed.
    pub fn deregister(&mut self, worker_id: &str) -> bool {
        self.workers.remove(worker_id).is_some()
    }

    pub fn get(&self, worker_id: &str) -> Option<&WorkerRegistration> {
        self.workers.get(worker_id)
    }

    /// Returns all workers that support the given language.
    pub fn workers_supporting_language(&self, lang: &str) -> Vec<&WorkerRegistration> {
        let lang_lower = lang.to_lowercase();
        let mut result: Vec<&WorkerRegistration> = self
            .workers
            .values()
            .filter(|w| {
                w.capabilities
                    .supported_languages
                    .iter()
                    .any(|l| l.to_lowercase() == lang_lower)
            })
            .collect();
        result.sort_by(|a, b| a.worker_id.cmp(&b.worker_id));
        result
    }

    pub fn count(&self) -> usize {
        self.workers.len()
    }
}

// ─── ClawCodeWorker ──────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ClawCodeWorker {
    registration: WorkerRegistration,
    tasks: HashMap<String, ClawCodeTask>,
    task_counter: u64,
}

impl ClawCodeWorker {
    pub fn new(registration: WorkerRegistration) -> Self {
        Self {
            registration,
            tasks: HashMap::new(),
            task_counter: 0,
        }
    }

    pub fn registration(&self) -> &WorkerRegistration {
        &self.registration
    }

    /// Submits a new task and returns its `task_id`.
    pub fn submit(&mut self, task_type: ClawCodeTaskType, payload: &str) -> String {
        self.task_counter += 1;
        let task_id = format!("{}-task-{}", self.registration.worker_id, self.task_counter);
        let now = now_ms();
        let task = ClawCodeTask {
            task_id: task_id.clone(),
            task_type,
            payload: payload.to_string(),
            status: ClawCodeTaskStatus::Running,
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.tasks.insert(task_id.clone(), task);
        task_id
    }

    pub fn complete(&mut self, task_id: &str, result: String) -> Result<(), String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("task '{}' not found", task_id))?;
        task.status = ClawCodeTaskStatus::Completed(result);
        task.updated_at_ms = now_ms();
        Ok(())
    }

    pub fn fail(&mut self, task_id: &str, error: String) -> Result<(), String> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| format!("task '{}' not found", task_id))?;
        task.status = ClawCodeTaskStatus::Failed(error);
        task.updated_at_ms = now_ms();
        Ok(())
    }

    pub fn running_tasks(&self) -> Vec<&ClawCodeTask> {
        self.tasks
            .values()
            .filter(|t| t.status == ClawCodeTaskStatus::Running)
            .collect()
    }

    pub fn completed_tasks(&self) -> Vec<&ClawCodeTask> {
        self.tasks
            .values()
            .filter(|t| matches!(t.status, ClawCodeTaskStatus::Completed(_)))
            .collect()
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

// ─── Free functions ──────────────────────────────────────────────────────────

/// Maps a task type to a REPL command string.
pub fn map_task_type_to_repl_command(task_type: &ClawCodeTaskType) -> String {
    match task_type {
        ClawCodeTaskType::CodeEdit => "edit".to_string(),
        ClawCodeTaskType::CodeReview => "review".to_string(),
        ClawCodeTaskType::TestGen => "test gen".to_string(),
        ClawCodeTaskType::Explain => "explain".to_string(),
        ClawCodeTaskType::Refactor => "refactor".to_string(),
        ClawCodeTaskType::Custom(s) => s.clone(),
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

    fn make_registration(id: &str, langs: Vec<&str>) -> WorkerRegistration {
        WorkerRegistration {
            worker_id: id.to_string(),
            name: format!("Worker {}", id),
            version: "1.0.0".into(),
            capabilities: WorkerCapability {
                supported_languages: langs.into_iter().map(|s| s.to_string()).collect(),
                available_tools: vec!["edit".into()],
                context_window_k: 128,
                providers: vec!["claude".into()],
            },
            endpoint: format!("http://localhost:800{}", id.len()),
            registered_at_ms: 0,
        }
    }

    // ── WorkerRegistry ────────────────────────────────────────────────────

    #[test]
    fn test_registry_new_empty() {
        let r = WorkerRegistry::new();
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn test_registry_register_success() {
        let mut r = WorkerRegistry::new();
        let result = r.register(make_registration("w1", vec!["rust"]));
        assert!(result.is_ok());
        assert_eq!(r.count(), 1);
    }

    #[test]
    fn test_registry_register_duplicate_fails() {
        let mut r = WorkerRegistry::new();
        r.register(make_registration("w1", vec!["rust"])).unwrap();
        let result = r.register(make_registration("w1", vec!["python"]));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_deregister_existing() {
        let mut r = WorkerRegistry::new();
        r.register(make_registration("w1", vec!["rust"])).unwrap();
        let removed = r.deregister("w1");
        assert!(removed);
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn test_registry_deregister_nonexistent() {
        let mut r = WorkerRegistry::new();
        let removed = r.deregister("nope");
        assert!(!removed);
    }

    #[test]
    fn test_registry_get_existing() {
        let mut r = WorkerRegistry::new();
        r.register(make_registration("w2", vec!["python"])).unwrap();
        assert!(r.get("w2").is_some());
    }

    #[test]
    fn test_registry_get_missing() {
        let r = WorkerRegistry::new();
        assert!(r.get("missing").is_none());
    }

    #[test]
    fn test_workers_supporting_language_match() {
        let mut r = WorkerRegistry::new();
        r.register(make_registration("w1", vec!["rust", "python"])).unwrap();
        r.register(make_registration("w2", vec!["typescript"])).unwrap();
        let matches = r.workers_supporting_language("rust");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].worker_id, "w1");
    }

    #[test]
    fn test_workers_supporting_language_no_match() {
        let mut r = WorkerRegistry::new();
        r.register(make_registration("w1", vec!["rust"])).unwrap();
        let matches = r.workers_supporting_language("go");
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_workers_supporting_language_case_insensitive() {
        let mut r = WorkerRegistry::new();
        r.register(make_registration("w1", vec!["Rust"])).unwrap();
        let matches = r.workers_supporting_language("rust");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_workers_supporting_multiple_workers() {
        let mut r = WorkerRegistry::new();
        r.register(make_registration("w1", vec!["python"])).unwrap();
        r.register(make_registration("w2", vec!["python", "rust"])).unwrap();
        let matches = r.workers_supporting_language("python");
        assert_eq!(matches.len(), 2);
    }

    // ── ClawCodeWorker ────────────────────────────────────────────────────

    #[test]
    fn test_worker_new() {
        let w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        assert_eq!(w.task_count(), 0);
        assert_eq!(w.registration().worker_id, "w1");
    }

    #[test]
    fn test_worker_submit_returns_id() {
        let mut w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        let id = w.submit(ClawCodeTaskType::CodeEdit, "fix the bug");
        assert!(!id.is_empty());
    }

    #[test]
    fn test_worker_submit_increases_count() {
        let mut w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        w.submit(ClawCodeTaskType::CodeEdit, "p1");
        w.submit(ClawCodeTaskType::Explain, "p2");
        assert_eq!(w.task_count(), 2);
    }

    #[test]
    fn test_worker_running_tasks() {
        let mut w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        w.submit(ClawCodeTaskType::CodeEdit, "p1");
        assert_eq!(w.running_tasks().len(), 1);
    }

    #[test]
    fn test_worker_complete_task() {
        let mut w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        let id = w.submit(ClawCodeTaskType::Explain, "explain this");
        let result = w.complete(&id, "done".to_string());
        assert!(result.is_ok());
        assert_eq!(w.completed_tasks().len(), 1);
        assert_eq!(w.running_tasks().len(), 0);
    }

    #[test]
    fn test_worker_fail_task() {
        let mut w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        let id = w.submit(ClawCodeTaskType::TestGen, "gen tests");
        let result = w.fail(&id, "timeout".to_string());
        assert!(result.is_ok());
        assert_eq!(w.running_tasks().len(), 0);
    }

    #[test]
    fn test_worker_complete_invalid_id() {
        let mut w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        let result = w.complete("bad-id", "done".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_worker_fail_invalid_id() {
        let mut w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        let result = w.fail("bad-id", "error".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_worker_completed_tasks_only_completed() {
        let mut w = ClawCodeWorker::new(make_registration("w1", vec!["rust"]));
        let id1 = w.submit(ClawCodeTaskType::CodeEdit, "p1");
        let _id2 = w.submit(ClawCodeTaskType::Refactor, "p2");
        w.complete(&id1, "ok".to_string()).unwrap();
        assert_eq!(w.completed_tasks().len(), 1);
        assert_eq!(w.running_tasks().len(), 1);
    }

    // ── map_task_type_to_repl_command ──────────────────────────────────────

    #[test]
    fn test_map_code_edit() {
        assert_eq!(map_task_type_to_repl_command(&ClawCodeTaskType::CodeEdit), "edit");
    }

    #[test]
    fn test_map_code_review() {
        assert_eq!(map_task_type_to_repl_command(&ClawCodeTaskType::CodeReview), "review");
    }

    #[test]
    fn test_map_test_gen() {
        assert_eq!(map_task_type_to_repl_command(&ClawCodeTaskType::TestGen), "test gen");
    }

    #[test]
    fn test_map_explain() {
        assert_eq!(map_task_type_to_repl_command(&ClawCodeTaskType::Explain), "explain");
    }

    #[test]
    fn test_map_refactor() {
        assert_eq!(map_task_type_to_repl_command(&ClawCodeTaskType::Refactor), "refactor");
    }

    #[test]
    fn test_map_custom() {
        assert_eq!(
            map_task_type_to_repl_command(&ClawCodeTaskType::Custom("my-cmd".into())),
            "my-cmd"
        );
    }

    #[test]
    fn test_map_custom_empty() {
        assert_eq!(
            map_task_type_to_repl_command(&ClawCodeTaskType::Custom(String::new())),
            ""
        );
    }
}
