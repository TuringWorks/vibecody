/*!
 * task_scheduler.rs — Cron/interval/once task scheduler.
 *
 * Compute next-run times, detect due tasks, manage task registry.
 */

// ---------------------------------------------------------------------------
// Schedule
// ---------------------------------------------------------------------------

/// How a task is triggered.
#[derive(Debug, Clone, PartialEq)]
pub enum Schedule {
    /// Repeat every `secs` seconds.
    Interval { secs: u64 },
    /// Run exactly once at `at_secs` (Unix timestamp).
    Once { at_secs: u64 },
    /// Run at a specific hour:minute each day (simplified cron).
    Cron { minute: u8, hour: u8 },
}

impl Schedule {
    /// Compute the next trigger time after `from_secs`.
    pub fn next_after(&self, from_secs: u64) -> u64 {
        match self {
            Schedule::Interval { secs } => from_secs + secs,
            Schedule::Once { at_secs } => {
                if *at_secs > from_secs {
                    *at_secs
                } else {
                    u64::MAX
                }
            }
            Schedule::Cron { minute, hour } => {
                // Seconds into the current day
                let day_secs = 86_400u64;
                let target_offset = (*hour as u64) * 3600 + (*minute as u64) * 60;
                let day_start = (from_secs / day_secs) * day_secs;
                let candidate = day_start + target_offset;
                if candidate > from_secs {
                    candidate
                } else {
                    // Next day
                    day_start + day_secs + target_offset
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TaskStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

// ---------------------------------------------------------------------------
// CronTask
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CronTask {
    pub id: String,
    pub name: String,
    pub command: String,
    pub schedule: Schedule,
    pub last_run_secs: Option<u64>,
    pub next_run_secs: u64,
    pub status: TaskStatus,
    pub run_count: u32,
}

impl CronTask {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        command: impl Into<String>,
        schedule: Schedule,
        now_secs: u64,
    ) -> Self {
        let next_run_secs = schedule.next_after(now_secs);
        Self {
            id: id.into(),
            name: name.into(),
            command: command.into(),
            schedule,
            last_run_secs: None,
            next_run_secs,
            status: TaskStatus::Pending,
            run_count: 0,
        }
    }

    pub fn is_due(&self, now_secs: u64) -> bool {
        now_secs >= self.next_run_secs && self.status == TaskStatus::Pending
    }

    pub fn mark_run(&mut self, now_secs: u64) {
        self.last_run_secs = Some(now_secs);
        self.next_run_secs = self.schedule.next_after(now_secs);
        self.run_count += 1;
        // Reset to Pending so the task can run again (unless Once)
        if matches!(self.schedule, Schedule::Once { .. }) {
            self.status = TaskStatus::Completed;
        } else {
            self.status = TaskStatus::Pending;
        }
    }
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct Scheduler {
    pub tasks: Vec<CronTask>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, task: CronTask) {
        self.tasks.push(task);
    }

    pub fn due_tasks(&self, now_secs: u64) -> Vec<&CronTask> {
        self.tasks.iter().filter(|t| t.is_due(now_secs)).collect()
    }

    /// Mark all due tasks as run and return their IDs.
    pub fn tick(&mut self, now_secs: u64) -> Vec<String> {
        let mut ids = Vec::new();
        for task in &mut self.tasks {
            if task.is_due(now_secs) {
                ids.push(task.id.clone());
                task.mark_run(now_secs);
            }
        }
        ids
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let len_before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() < len_before
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

// ---------------------------------------------------------------------------
// Legacy priority-based scheduler — retained for BDD harness compatibility
// ---------------------------------------------------------------------------

use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// Priority level for scheduled tasks (higher value = higher priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A priority-based task unit managed by `TaskScheduler`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTask {
    pub id: String,
    pub label: String,
    pub priority: TaskPriority,
    /// Unix timestamp after which the task is eligible to run (0 = immediately).
    pub run_after: u64,
}

impl ScheduledTask {
    pub fn new(id: impl Into<String>, label: impl Into<String>, priority: TaskPriority) -> Self {
        Self { id: id.into(), label: label.into(), priority, run_after: 0 }
    }

    pub fn with_run_after(mut self, ts: u64) -> Self {
        self.run_after = ts;
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HeapEntry {
    priority: TaskPriority,
    run_after: u64,
    task: ScheduledTask,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
            .then_with(|| other.run_after.cmp(&self.run_after))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

/// BinaryHeap-based priority scheduler.
#[derive(Debug, Default)]
pub struct TaskScheduler {
    heap: BinaryHeap<HeapEntry>,
}

impl TaskScheduler {
    pub fn new() -> Self { Self::default() }

    pub fn push(&mut self, task: ScheduledTask) {
        self.heap.push(HeapEntry { priority: task.priority, run_after: task.run_after, task });
    }

    /// Dequeue the highest-priority task that is eligible at `now`.
    pub fn pop_ready(&mut self, now: u64) -> Option<ScheduledTask> {
        if let Some(top) = self.heap.peek() {
            if top.run_after <= now {
                return self.heap.pop().map(|e| e.task);
            }
        }
        None
    }

    pub fn len(&self) -> usize { self.heap.len() }
    pub fn is_empty(&self) -> bool { self.heap.is_empty() }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_next_after() {
        let s = Schedule::Interval { secs: 60 };
        assert_eq!(s.next_after(1000), 1060);
    }

    #[test]
    fn test_once_next_after() {
        let s = Schedule::Once { at_secs: 5000 };
        assert_eq!(s.next_after(1000), 5000);
        // Past: returns MAX
        assert_eq!(s.next_after(5001), u64::MAX);
    }

    #[test]
    fn test_cron_next_after_advances_by_day_secs() {
        // hour=0, minute=0 — the next midnight from noon
        let s = Schedule::Cron { minute: 0, hour: 0 };
        let noon = 12 * 3600u64; // 12:00:00 on day 0
        let next = s.next_after(noon);
        // Should land at next midnight (86400)
        assert_eq!(next, 86_400);
    }

    #[test]
    fn test_task_is_due_when_past() {
        let task = CronTask::new("t1", "T1", "echo", Schedule::Interval { secs: 10 }, 0);
        // next_run = 10; at now=10 it's due
        assert!(task.is_due(10));
    }

    #[test]
    fn test_task_not_due_when_future() {
        let task = CronTask::new("t1", "T1", "echo", Schedule::Interval { secs: 100 }, 0);
        assert!(!task.is_due(50));
    }

    #[test]
    fn test_mark_run_increments_count() {
        let mut task = CronTask::new("t1", "T1", "echo", Schedule::Interval { secs: 10 }, 0);
        task.mark_run(10);
        assert_eq!(task.run_count, 1);
        assert_eq!(task.last_run_secs, Some(10));
        assert_eq!(task.next_run_secs, 20);
    }

    #[test]
    fn test_scheduler_due_tasks() {
        let mut sched = Scheduler::new();
        sched.add(CronTask::new("a", "A", "cmd", Schedule::Interval { secs: 5 }, 0));
        sched.add(CronTask::new("b", "B", "cmd", Schedule::Interval { secs: 100 }, 0));
        let due = sched.due_tasks(10);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "a");
    }

    #[test]
    fn test_scheduler_tick_returns_ids() {
        let mut sched = Scheduler::new();
        sched.add(CronTask::new("x", "X", "cmd", Schedule::Interval { secs: 1 }, 0));
        sched.add(CronTask::new("y", "Y", "cmd", Schedule::Interval { secs: 1000 }, 0));
        let ids = sched.tick(5);
        assert_eq!(ids, vec!["x"]);
    }

    #[test]
    fn test_scheduler_remove() {
        let mut sched = Scheduler::new();
        sched.add(CronTask::new("r1", "R1", "cmd", Schedule::Interval { secs: 1 }, 0));
        assert_eq!(sched.task_count(), 1);
        assert!(sched.remove("r1"));
        assert_eq!(sched.task_count(), 0);
        assert!(!sched.remove("r1")); // already gone
    }
}
