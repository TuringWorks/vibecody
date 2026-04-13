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
