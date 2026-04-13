//! task_scheduler — Priority-queue task scheduler with cron-style triggers.
//! Tasks are enqueued with a priority and optional run-at time, then dequeued
//! in priority order when their scheduled time has been reached.

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

/// A unit of work managed by the scheduler.
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

// BinaryHeap needs Ord — we want highest priority first, then earliest run_after.
#[derive(Debug, Clone, Eq, PartialEq)]
struct HeapEntry {
    priority: TaskPriority,
    run_after: u64,
    task: ScheduledTask,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority wins; ties broken by earlier run_after.
        self.priority.cmp(&other.priority)
            .then_with(|| other.run_after.cmp(&self.run_after))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

/// Scheduler state.
#[derive(Debug, Default)]
pub struct TaskScheduler {
    heap: BinaryHeap<HeapEntry>,
}

impl TaskScheduler {
    pub fn new() -> Self { Self::default() }

    /// Enqueue a task.
    pub fn push(&mut self, task: ScheduledTask) {
        self.heap.push(HeapEntry {
            priority: task.priority,
            run_after: task.run_after,
            task,
        });
    }

    /// Dequeue the highest-priority task that is eligible at `now`.
    pub fn pop_ready(&mut self, now: u64) -> Option<ScheduledTask> {
        // Peek first: only pop if the top entry is ready.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, priority: TaskPriority, run_after: u64) -> ScheduledTask {
        ScheduledTask::new(id, id, priority).with_run_after(run_after)
    }

    #[test]
    fn test_push_and_len() {
        let mut s = TaskScheduler::new();
        s.push(task("t1", TaskPriority::Normal, 0));
        assert_eq!(s.len(), 1);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_pop_ready_returns_none_when_not_ready() {
        let mut s = TaskScheduler::new();
        s.push(task("future", TaskPriority::High, 1000));
        assert!(s.pop_ready(500).is_none());
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_pop_ready_returns_task_when_ready() {
        let mut s = TaskScheduler::new();
        s.push(task("t1", TaskPriority::Normal, 100));
        let t = s.pop_ready(100);
        assert!(t.is_some());
        assert_eq!(t.unwrap().id, "t1");
    }

    #[test]
    fn test_priority_order_high_before_low() {
        let mut s = TaskScheduler::new();
        s.push(task("low", TaskPriority::Low, 0));
        s.push(task("high", TaskPriority::High, 0));
        s.push(task("normal", TaskPriority::Normal, 0));
        let first = s.pop_ready(u64::MAX).unwrap();
        assert_eq!(first.id, "high");
    }

    #[test]
    fn test_critical_before_all() {
        let mut s = TaskScheduler::new();
        s.push(task("normal", TaskPriority::Normal, 0));
        s.push(task("critical", TaskPriority::Critical, 0));
        s.push(task("high", TaskPriority::High, 0));
        assert_eq!(s.pop_ready(u64::MAX).unwrap().id, "critical");
    }

    #[test]
    fn test_same_priority_earlier_run_after_wins() {
        let mut s = TaskScheduler::new();
        s.push(task("late", TaskPriority::Normal, 200));
        s.push(task("early", TaskPriority::Normal, 50));
        let first = s.pop_ready(u64::MAX).unwrap();
        assert_eq!(first.id, "early");
    }

    #[test]
    fn test_empty_scheduler() {
        let mut s = TaskScheduler::new();
        assert!(s.is_empty());
        assert!(s.pop_ready(0).is_none());
    }

    #[test]
    fn test_task_priority_ord() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_drain_all_ready() {
        let mut s = TaskScheduler::new();
        for i in 0..5u64 {
            s.push(task(&format!("t{i}"), TaskPriority::Normal, i * 10));
        }
        let mut count = 0;
        while s.pop_ready(u64::MAX).is_some() { count += 1; }
        assert_eq!(count, 5);
        assert!(s.is_empty());
    }
}
