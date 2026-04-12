#![allow(dead_code)]
//! Agent conditional pause primitive — lets an agent suspend execution
//! until an external condition is satisfied (process exit, file change,
//! log pattern match, port readiness, HTTP readiness, timer, or manual token).
//!
//! # Design
//!
//! The `AwaitRegistry` tracks in-flight conditions. The agent emits an
//! `AwaitTool` JSON blob into the tool-call stream; the host runtime
//! registers it and calls `satisfy` / `cancel` / `check_timers` as events
//! arrive.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Enums ───────────────────────────────────────────────────────────────────

/// Granularity of file-system change that should trigger satisfaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
    Any,
}

/// The family of condition the agent wants to wait for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionKind {
    ProcessExit { pid: u32 },
    LogPattern { source: String, pattern: String },
    FileChange { path: String, kind: FileChangeKind },
    PortOpen { host: String, port: u16 },
    HttpReady { url: String, expected_status: u16 },
    TimerElapsed { duration_secs: u64 },
    ManualResume { token: String },
}

/// Terminal outcome stored in a satisfied condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AwaitResult {
    Satisfied,
    TimedOut,
    Cancelled,
}

/// Current status of a registered condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AwaitStatus {
    Waiting,
    Satisfied(AwaitResult),
    Cancelled,
}

// ─── AwaitCondition ──────────────────────────────────────────────────────────

/// A registered condition with its lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitCondition {
    pub condition_id: String,
    pub kind: ConditionKind,
    pub timeout_secs: u64,
    pub registered_at_ms: u64,
    pub status: AwaitStatus,
}

// ─── AwaitRegistry ───────────────────────────────────────────────────────────

/// Manages registered conditions and their status transitions.
pub struct AwaitRegistry {
    conditions: HashMap<String, AwaitCondition>,
    next_id: u64,
}

impl AwaitRegistry {
    pub fn new() -> Self {
        Self {
            conditions: HashMap::new(),
            next_id: 1,
        }
    }

    /// Register a new condition. Returns its condition_id.
    pub fn register(&mut self, kind: ConditionKind, timeout_secs: u64) -> String {
        let condition_id = format!("await-{}", self.next_id);
        self.next_id += 1;
        let cond = AwaitCondition {
            condition_id: condition_id.clone(),
            kind,
            timeout_secs,
            registered_at_ms: 0,
            status: AwaitStatus::Waiting,
        };
        self.conditions.insert(condition_id.clone(), cond);
        condition_id
    }

    /// Mark a waiting condition as `Satisfied(AwaitResult::Satisfied)`.
    pub fn satisfy(&mut self, condition_id: &str) -> Result<(), String> {
        let cond = self
            .conditions
            .get_mut(condition_id)
            .ok_or_else(|| format!("condition not found: {}", condition_id))?;
        match cond.status {
            AwaitStatus::Waiting => {
                cond.status = AwaitStatus::Satisfied(AwaitResult::Satisfied);
                Ok(())
            }
            _ => Err(format!(
                "condition {} is not waiting (current: {:?})",
                condition_id, cond.status
            )),
        }
    }

    /// Cancel a waiting condition.
    pub fn cancel(&mut self, condition_id: &str) -> Result<(), String> {
        let cond = self
            .conditions
            .get_mut(condition_id)
            .ok_or_else(|| format!("condition not found: {}", condition_id))?;
        match cond.status {
            AwaitStatus::Waiting => {
                cond.status = AwaitStatus::Cancelled;
                Ok(())
            }
            _ => Err(format!(
                "condition {} is not waiting (current: {:?})",
                condition_id, cond.status
            )),
        }
    }

    /// Scan all waiting conditions and mark timed-out ones as
    /// `Satisfied(TimedOut)`.
    pub fn check_timers(&mut self, now_ms: u64) {
        for cond in self.conditions.values_mut() {
            if cond.status == AwaitStatus::Waiting
                && is_timer_expired(cond, now_ms)
            {
                cond.status = AwaitStatus::Satisfied(AwaitResult::TimedOut);
            }
        }
    }

    /// All conditions currently in the `Waiting` state.
    pub fn waiting(&self) -> Vec<&AwaitCondition> {
        self.conditions
            .values()
            .filter(|c| c.status == AwaitStatus::Waiting)
            .collect()
    }

    /// All conditions in any `Satisfied` state.
    pub fn satisfied(&self) -> Vec<&AwaitCondition> {
        self.conditions
            .values()
            .filter(|c| matches!(c.status, AwaitStatus::Satisfied(_)))
            .collect()
    }

    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }

    pub fn get(&self, condition_id: &str) -> Option<&AwaitCondition> {
        self.conditions.get(condition_id)
    }
}

impl Default for AwaitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Pure helpers ────────────────────────────────────────────────────────────

/// Human-readable description of a condition kind.
pub fn condition_description(kind: &ConditionKind) -> String {
    match kind {
        ConditionKind::ProcessExit { pid } => format!("process exit (pid {})", pid),
        ConditionKind::LogPattern { source, pattern } => {
            format!("log pattern '{}' in {}", pattern, source)
        }
        ConditionKind::FileChange { path, kind } => {
            format!("file {:?} at {}", kind, path)
        }
        ConditionKind::PortOpen { host, port } => {
            format!("port {}:{} open", host, port)
        }
        ConditionKind::HttpReady { url, expected_status } => {
            format!("HTTP {} returns {}", url, expected_status)
        }
        ConditionKind::TimerElapsed { duration_secs } => {
            format!("timer {}s elapsed", duration_secs)
        }
        ConditionKind::ManualResume { token } => {
            format!("manual resume token '{}'", token)
        }
    }
}

/// Returns `true` if the condition's deadline has passed relative to `now_ms`.
///
/// Deadline = `registered_at_ms + timeout_secs * 1000`.
pub fn is_timer_expired(cond: &AwaitCondition, now_ms: u64) -> bool {
    let deadline = cond
        .registered_at_ms
        .saturating_add(cond.timeout_secs.saturating_mul(1000));
    now_ms >= deadline
}

/// Returns `true` if `pattern` is a substring of `log_line`.
pub fn matches_log_pattern(log_line: &str, pattern: &str) -> bool {
    log_line.contains(pattern)
}

// ─── AwaitTool ───────────────────────────────────────────────────────────────

/// The serializable tool-call payload an agent emits to request a pause.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitTool {
    pub kind: ConditionKind,
    pub reason: String,
    pub timeout_secs: u64,
}

impl AwaitTool {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn waiting_cond(registered_at_ms: u64, timeout_secs: u64) -> AwaitCondition {
        AwaitCondition {
            condition_id: "c1".to_string(),
            kind: ConditionKind::TimerElapsed { duration_secs: timeout_secs },
            timeout_secs,
            registered_at_ms,
            status: AwaitStatus::Waiting,
        }
    }

    // ── AwaitRegistry basic ──────────────────────────────────────────────────

    #[test]
    fn registry_starts_empty() {
        let reg = AwaitRegistry::new();
        assert_eq!(reg.condition_count(), 0);
    }

    #[test]
    fn registry_register_returns_id() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ManualResume { token: "abc".to_string() }, 60);
        assert!(!id.is_empty());
        assert_eq!(reg.condition_count(), 1);
    }

    #[test]
    fn registry_register_multiple_unique_ids() {
        let mut reg = AwaitRegistry::new();
        let id1 = reg.register(ConditionKind::TimerElapsed { duration_secs: 5 }, 10);
        let id2 = reg.register(ConditionKind::TimerElapsed { duration_secs: 5 }, 10);
        assert_ne!(id1, id2);
    }

    #[test]
    fn registry_get_after_register() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ProcessExit { pid: 42 }, 30);
        let cond = reg.get(&id).unwrap();
        assert_eq!(cond.status, AwaitStatus::Waiting);
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let reg = AwaitRegistry::new();
        assert!(reg.get("ghost").is_none());
    }

    // ── satisfy ──────────────────────────────────────────────────────────────

    #[test]
    fn registry_satisfy_transitions_to_satisfied() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ProcessExit { pid: 1 }, 60);
        reg.satisfy(&id).unwrap();
        assert!(matches!(
            reg.get(&id).unwrap().status,
            AwaitStatus::Satisfied(AwaitResult::Satisfied)
        ));
    }

    #[test]
    fn registry_satisfy_missing_returns_err() {
        let mut reg = AwaitRegistry::new();
        assert!(reg.satisfy("no-such").is_err());
    }

    #[test]
    fn registry_satisfy_already_satisfied_returns_err() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ProcessExit { pid: 1 }, 60);
        reg.satisfy(&id).unwrap();
        assert!(reg.satisfy(&id).is_err());
    }

    // ── cancel ───────────────────────────────────────────────────────────────

    #[test]
    fn registry_cancel_transitions_to_cancelled() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ManualResume { token: "t".to_string() }, 60);
        reg.cancel(&id).unwrap();
        assert_eq!(reg.get(&id).unwrap().status, AwaitStatus::Cancelled);
    }

    #[test]
    fn registry_cancel_missing_returns_err() {
        let mut reg = AwaitRegistry::new();
        assert!(reg.cancel("ghost").is_err());
    }

    #[test]
    fn registry_cancel_already_cancelled_returns_err() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ManualResume { token: "t".to_string() }, 60);
        reg.cancel(&id).unwrap();
        assert!(reg.cancel(&id).is_err());
    }

    // ── check_timers ─────────────────────────────────────────────────────────

    #[test]
    fn check_timers_expires_overdue_condition() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::TimerElapsed { duration_secs: 1 }, 1);
        // registered_at_ms = 0, timeout = 1s = 1000ms; now = 2000 → expired
        reg.check_timers(2000);
        assert!(matches!(
            reg.get(&id).unwrap().status,
            AwaitStatus::Satisfied(AwaitResult::TimedOut)
        ));
    }

    #[test]
    fn check_timers_does_not_expire_fresh_condition() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::TimerElapsed { duration_secs: 60 }, 60);
        // registered_at_ms = 0, timeout = 60s = 60000ms; now = 100 → not expired
        reg.check_timers(100);
        assert_eq!(reg.get(&id).unwrap().status, AwaitStatus::Waiting);
    }

    #[test]
    fn check_timers_does_not_double_expire() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::TimerElapsed { duration_secs: 1 }, 1);
        reg.check_timers(2000);
        reg.check_timers(3000); // second call should be a no-op
        assert!(matches!(
            reg.get(&id).unwrap().status,
            AwaitStatus::Satisfied(AwaitResult::TimedOut)
        ));
    }

    #[test]
    fn check_timers_does_not_expire_cancelled() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::TimerElapsed { duration_secs: 1 }, 1);
        reg.cancel(&id).unwrap();
        reg.check_timers(9999);
        assert_eq!(reg.get(&id).unwrap().status, AwaitStatus::Cancelled);
    }

    // ── waiting / satisfied queries ──────────────────────────────────────────

    #[test]
    fn registry_waiting_returns_only_waiting() {
        let mut reg = AwaitRegistry::new();
        let id1 = reg.register(ConditionKind::ProcessExit { pid: 1 }, 60);
        let id2 = reg.register(ConditionKind::ProcessExit { pid: 2 }, 60);
        reg.satisfy(&id1).unwrap();
        let waiting = reg.waiting();
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].condition_id, id2);
    }

    #[test]
    fn registry_satisfied_returns_only_satisfied() {
        let mut reg = AwaitRegistry::new();
        let id1 = reg.register(ConditionKind::ProcessExit { pid: 1 }, 60);
        reg.register(ConditionKind::ProcessExit { pid: 2 }, 60);
        reg.satisfy(&id1).unwrap();
        assert_eq!(reg.satisfied().len(), 1);
    }

    #[test]
    fn registry_cancelled_not_in_waiting_or_satisfied() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ProcessExit { pid: 1 }, 60);
        reg.cancel(&id).unwrap();
        assert!(reg.waiting().is_empty());
        assert!(reg.satisfied().is_empty());
    }

    // ── ConditionKind variants ────────────────────────────────────────────────

    #[test]
    fn register_process_exit_kind() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ProcessExit { pid: 1234 }, 30);
        let cond = reg.get(&id).unwrap();
        assert!(matches!(cond.kind, ConditionKind::ProcessExit { pid: 1234 }));
    }

    #[test]
    fn register_log_pattern_kind() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(
            ConditionKind::LogPattern {
                source: "app.log".to_string(),
                pattern: "ERROR".to_string(),
            },
            60,
        );
        assert!(matches!(reg.get(&id).unwrap().kind, ConditionKind::LogPattern { .. }));
    }

    #[test]
    fn register_file_change_kind() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(
            ConditionKind::FileChange {
                path: "/tmp/foo".to_string(),
                kind: FileChangeKind::Created,
            },
            10,
        );
        assert!(matches!(reg.get(&id).unwrap().kind, ConditionKind::FileChange { .. }));
    }

    #[test]
    fn register_port_open_kind() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(
            ConditionKind::PortOpen { host: "localhost".to_string(), port: 8080 },
            15,
        );
        assert!(matches!(reg.get(&id).unwrap().kind, ConditionKind::PortOpen { .. }));
    }

    #[test]
    fn register_http_ready_kind() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(
            ConditionKind::HttpReady {
                url: "http://localhost:3000/health".to_string(),
                expected_status: 200,
            },
            20,
        );
        assert!(matches!(reg.get(&id).unwrap().kind, ConditionKind::HttpReady { .. }));
    }

    #[test]
    fn register_timer_elapsed_kind() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::TimerElapsed { duration_secs: 5 }, 10);
        assert!(matches!(reg.get(&id).unwrap().kind, ConditionKind::TimerElapsed { .. }));
    }

    #[test]
    fn register_manual_resume_kind() {
        let mut reg = AwaitRegistry::new();
        let id = reg.register(ConditionKind::ManualResume { token: "resume-xyz".to_string() }, 3600);
        assert!(matches!(reg.get(&id).unwrap().kind, ConditionKind::ManualResume { .. }));
    }

    // ── is_timer_expired ─────────────────────────────────────────────────────

    #[test]
    fn timer_expired_when_past_deadline() {
        let cond = waiting_cond(0, 5); // deadline = 5000 ms
        assert!(is_timer_expired(&cond, 5000));
        assert!(is_timer_expired(&cond, 6000));
    }

    #[test]
    fn timer_not_expired_before_deadline() {
        let cond = waiting_cond(0, 5);
        assert!(!is_timer_expired(&cond, 4999));
    }

    #[test]
    fn timer_expired_zero_timeout() {
        let cond = waiting_cond(1000, 0); // deadline = 1000
        assert!(is_timer_expired(&cond, 1000));
    }

    #[test]
    fn timer_registered_at_nonzero() {
        let cond = waiting_cond(5000, 10); // deadline = 15000
        assert!(!is_timer_expired(&cond, 14999));
        assert!(is_timer_expired(&cond, 15000));
    }

    // ── matches_log_pattern ──────────────────────────────────────────────────

    #[test]
    fn log_pattern_matches_substring() {
        assert!(matches_log_pattern("2026-01-01 ERROR: disk full", "ERROR"));
    }

    #[test]
    fn log_pattern_no_match() {
        assert!(!matches_log_pattern("INFO: all good", "ERROR"));
    }

    #[test]
    fn log_pattern_empty_pattern_always_matches() {
        assert!(matches_log_pattern("anything", ""));
    }

    #[test]
    fn log_pattern_case_sensitive() {
        assert!(!matches_log_pattern("error: something", "ERROR"));
    }

    #[test]
    fn log_pattern_exact_match() {
        assert!(matches_log_pattern("WARN", "WARN"));
    }

    // ── condition_description ────────────────────────────────────────────────

    #[test]
    fn description_process_exit() {
        let d = condition_description(&ConditionKind::ProcessExit { pid: 42 });
        assert!(d.contains("42"));
    }

    #[test]
    fn description_log_pattern() {
        let d = condition_description(&ConditionKind::LogPattern {
            source: "app.log".to_string(),
            pattern: "FATAL".to_string(),
        });
        assert!(d.contains("FATAL"));
        assert!(d.contains("app.log"));
    }

    #[test]
    fn description_file_change() {
        let d = condition_description(&ConditionKind::FileChange {
            path: "/tmp/data.csv".to_string(),
            kind: FileChangeKind::Modified,
        });
        assert!(d.contains("/tmp/data.csv"));
    }

    #[test]
    fn description_port_open() {
        let d = condition_description(&ConditionKind::PortOpen {
            host: "db".to_string(),
            port: 5432,
        });
        assert!(d.contains("5432"));
        assert!(d.contains("db"));
    }

    #[test]
    fn description_http_ready() {
        let d = condition_description(&ConditionKind::HttpReady {
            url: "http://svc/health".to_string(),
            expected_status: 200,
        });
        assert!(d.contains("200"));
    }

    #[test]
    fn description_timer_elapsed() {
        let d = condition_description(&ConditionKind::TimerElapsed { duration_secs: 30 });
        assert!(d.contains("30"));
    }

    #[test]
    fn description_manual_resume() {
        let d = condition_description(&ConditionKind::ManualResume { token: "tok-abc".to_string() });
        assert!(d.contains("tok-abc"));
    }

    // ── AwaitTool JSON round-trip ─────────────────────────────────────────────

    #[test]
    fn await_tool_to_and_from_json() {
        let tool = AwaitTool {
            kind: ConditionKind::HttpReady {
                url: "http://localhost/health".to_string(),
                expected_status: 200,
            },
            reason: "waiting for service".to_string(),
            timeout_secs: 30,
        };
        let json = tool.to_json();
        let decoded = AwaitTool::from_json(&json).unwrap();
        assert_eq!(decoded.reason, "waiting for service");
        assert_eq!(decoded.timeout_secs, 30);
    }

    #[test]
    fn await_tool_from_json_invalid_returns_err() {
        assert!(AwaitTool::from_json("not json").is_err());
    }

    #[test]
    fn await_tool_process_exit_round_trip() {
        let tool = AwaitTool {
            kind: ConditionKind::ProcessExit { pid: 99 },
            reason: "waiting for build".to_string(),
            timeout_secs: 120,
        };
        let json = tool.to_json();
        let decoded = AwaitTool::from_json(&json).unwrap();
        assert!(matches!(decoded.kind, ConditionKind::ProcessExit { pid: 99 }));
    }

    #[test]
    fn await_tool_timer_round_trip() {
        let tool = AwaitTool {
            kind: ConditionKind::TimerElapsed { duration_secs: 5 },
            reason: "cooldown".to_string(),
            timeout_secs: 10,
        };
        let json = tool.to_json();
        let back = AwaitTool::from_json(&json).unwrap();
        assert_eq!(back.reason, "cooldown");
    }

    #[test]
    fn await_tool_file_change_round_trip() {
        let tool = AwaitTool {
            kind: ConditionKind::FileChange {
                path: "/var/lock/deploy.lock".to_string(),
                kind: FileChangeKind::Deleted,
            },
            reason: "lock released".to_string(),
            timeout_secs: 60,
        };
        let json = tool.to_json();
        let back = AwaitTool::from_json(&json).unwrap();
        assert!(matches!(back.kind, ConditionKind::FileChange { .. }));
    }

    #[test]
    fn await_tool_manual_resume_round_trip() {
        let tool = AwaitTool {
            kind: ConditionKind::ManualResume { token: "approval-123".to_string() },
            reason: "awaiting human approval".to_string(),
            timeout_secs: 3600,
        };
        let json = tool.to_json();
        let back = AwaitTool::from_json(&json).unwrap();
        assert!(matches!(back.kind, ConditionKind::ManualResume { .. }));
        assert_eq!(back.timeout_secs, 3600);
    }

    #[test]
    fn await_tool_log_pattern_round_trip() {
        let tool = AwaitTool {
            kind: ConditionKind::LogPattern {
                source: "stderr".to_string(),
                pattern: "Listening on".to_string(),
            },
            reason: "server started".to_string(),
            timeout_secs: 15,
        };
        let json = tool.to_json();
        let back = AwaitTool::from_json(&json).unwrap();
        assert!(matches!(back.kind, ConditionKind::LogPattern { .. }));
    }
}
