#![allow(dead_code)]
//! Built-in cron/scheduling system for VibeCLI.
//!
//! Allows users to schedule one-time and recurring agent tasks:
//!   /remind in 10m "check build status"
//!   /remind at 09:00 "daily standup summary"
//!   /schedule cron "0 2 * * *" "run nightly tests"
//!   /schedule list
//!   /schedule cancel <id>
//!
//! Jobs are persisted to ~/.vibecli/schedule.json and survive restarts.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A scheduled job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// Unique identifier (short UUID-style hex string).
    pub id: String,
    /// Human-readable task description.
    pub task: String,
    /// Schedule expression: ISO 8601 datetime for one-time, or cron expression for recurring.
    pub schedule: ScheduleExpr,
    /// Whether this job has been triggered at least once.
    pub triggered_count: u32,
    /// Timestamp (unix ms) of when this job was created.
    pub created_at: u64,
    /// Timestamp (unix ms) of last trigger (None if never).
    pub last_triggered: Option<u64>,
    /// Whether the job is active.
    pub active: bool,
}

/// Schedule expression for a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScheduleExpr {
    /// Fire once at a specific unix-ms timestamp.
    #[serde(rename = "once")]
    Once { at_ms: u64 },
    /// Fire repeatedly using a simplified cron-like schedule.
    /// Supported formats: "daily HH:MM", "hourly", "every Nm/Nh"
    #[serde(rename = "recurring")]
    Recurring { interval_secs: u64, next_at_ms: u64 },
}

/// Manages the schedule store.
pub struct Scheduler {
    path: PathBuf,
}

impl Scheduler {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let path = PathBuf::from(home).join(".vibecli").join("schedule.json");
        Self { path }
    }

    fn load_jobs(&self) -> Vec<ScheduledJob> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save_jobs(&self, jobs: &[ScheduledJob]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(jobs)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    fn short_id() -> String {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{:x}", t & 0xFFFF_FFFF)
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Add a one-time job that fires in `secs` seconds from now.
    pub fn add_in(&self, task: &str, secs: u64) -> Result<ScheduledJob> {
        let now = Self::now_ms();
        let job = ScheduledJob {
            id: Self::short_id(),
            task: task.to_string(),
            schedule: ScheduleExpr::Once { at_ms: now.saturating_add(secs.saturating_mul(1000)) },
            triggered_count: 0,
            created_at: now,
            last_triggered: None,
            active: true,
        };
        let mut jobs = self.load_jobs();
        jobs.push(job.clone());
        self.save_jobs(&jobs)?;
        Ok(job)
    }

    /// Add a recurring job that fires every `secs` seconds.
    pub fn add_recurring(&self, task: &str, interval_secs: u64) -> Result<ScheduledJob> {
        let now = Self::now_ms();
        let job = ScheduledJob {
            id: Self::short_id(),
            task: task.to_string(),
            schedule: ScheduleExpr::Recurring {
                interval_secs,
                next_at_ms: now.saturating_add(interval_secs.saturating_mul(1000)),
            },
            triggered_count: 0,
            created_at: now,
            last_triggered: None,
            active: true,
        };
        let mut jobs = self.load_jobs();
        jobs.push(job.clone());
        self.save_jobs(&jobs)?;
        Ok(job)
    }

    /// Cancel a job by id (prefix match).
    pub fn cancel(&self, id_prefix: &str) -> Result<Option<ScheduledJob>> {
        let mut jobs = self.load_jobs();
        let mut found = None;
        for job in &mut jobs {
            if job.id.starts_with(id_prefix) {
                job.active = false;
                found = Some(job.clone());
                break;
            }
        }
        self.save_jobs(&jobs)?;
        Ok(found)
    }

    /// List all active jobs.
    pub fn list(&self) -> Vec<ScheduledJob> {
        self.load_jobs()
            .into_iter()
            .filter(|j| j.active)
            .collect()
    }

    /// Poll for due jobs and return them (updating state).
    pub fn poll_due(&self) -> Vec<ScheduledJob> {
        let now = Self::now_ms();
        let mut jobs = self.load_jobs();
        let mut due = Vec::new();

        for job in &mut jobs {
            if !job.active { continue; }
            let is_due = match &job.schedule {
                ScheduleExpr::Once { at_ms } => now >= *at_ms,
                ScheduleExpr::Recurring { next_at_ms, .. } => now >= *next_at_ms,
            };
            if is_due {
                job.triggered_count += 1;
                job.last_triggered = Some(now);
                // For one-time jobs, deactivate after trigger
                match &mut job.schedule {
                    ScheduleExpr::Once { .. } => { job.active = false; }
                    ScheduleExpr::Recurring { interval_secs, next_at_ms } => {
                        *next_at_ms = now.saturating_add(interval_secs.saturating_mul(1000));
                    }
                }
                due.push(job.clone());
            }
        }

        let _ = self.save_jobs(&jobs);
        due
    }
}

/// Parse a human-readable duration string to seconds.
/// Supports: "10s", "5m", "2h", "1d"
pub fn parse_duration(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('s') {
        return n.trim().parse().ok();
    }
    if let Some(n) = s.strip_suffix('m') {
        return n.trim().parse::<u64>().ok().map(|v| v * 60);
    }
    if let Some(n) = s.strip_suffix('h') {
        return n.trim().parse::<u64>().ok().map(|v| v * 3600);
    }
    if let Some(n) = s.strip_suffix('d') {
        return n.trim().parse::<u64>().ok().map(|v| v * 86400);
    }
    None
}

/// Format a unix-ms timestamp as a relative human-readable string.
pub fn format_relative(at_ms: u64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    if at_ms <= now_ms {
        return "now".to_string();
    }
    let secs = (at_ms - now_ms) / 1000;
    if secs < 60 { return format!("in {}s", secs); }
    let mins = secs / 60;
    if mins < 60 { return format!("in {}m", mins); }
    let hours = mins / 60;
    if hours < 24 { return format!("in {}h {}m", hours, mins % 60); }
    let days = hours / 24;
    format!("in {}d {}h", days, hours % 24)
}

/// Format a duration (in seconds) as human-readable interval.
pub fn format_interval(secs: u64) -> String {
    if secs < 60 { return format!("every {}s", secs); }
    let mins = secs / 60;
    if mins < 60 { return format!("every {}m", mins); }
    let hours = mins / 60;
    if hours < 24 { return format!("every {}h", hours); }
    format!("every {}d", hours / 24)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_duration("30s"), Some(30));
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("10m"), Some(600));
    }

    #[test]
    fn parse_duration_hours() {
        assert_eq!(parse_duration("2h"), Some(7200));
    }

    #[test]
    fn parse_duration_days() {
        assert_eq!(parse_duration("1d"), Some(86400));
    }

    #[test]
    fn parse_duration_invalid() {
        assert_eq!(parse_duration("xyz"), None);
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn format_relative_past_is_now() {
        let past = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64 - 1000;
        assert_eq!(format_relative(past), "now");
    }

    #[test]
    fn format_interval_minutes() {
        assert_eq!(format_interval(300), "every 5m");
    }

    #[test]
    fn scheduled_job_serializes() {
        let job = ScheduledJob {
            id: "abc123".to_string(),
            task: "Run tests".to_string(),
            schedule: ScheduleExpr::Once { at_ms: 1700000000000 },
            triggered_count: 0,
            created_at: 1699999999000,
            last_triggered: None,
            active: true,
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"id\":\"abc123\""));
        assert!(json.contains("\"type\":\"once\""));
    }
}
