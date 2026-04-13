//! dispatch_remote — Priority-ordered remote job dispatch queue.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus { Queued, Running, Completed(String), Failed(String) }

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub prompt: String,
    pub priority: u8,
    pub enqueued_at: u64,
    pub status: JobStatus,
}

#[derive(Debug, Default)]
pub struct DispatchQueue {
    jobs: HashMap<String, Job>,
    counter: u64,
}

impl DispatchQueue {
    pub fn new() -> Self { Self::default() }

    fn next_id(&mut self) -> String {
        self.counter += 1;
        format!("job-{}", self.counter)
    }

    pub fn enqueue(&mut self, prompt: &str, now: u64) -> String {
        self.enqueue_with_priority(prompt, 128, now)
    }

    pub fn enqueue_with_priority(&mut self, prompt: &str, priority: u8, now: u64) -> String {
        let id = self.next_id();
        self.jobs.insert(id.clone(), Job { id: id.clone(), prompt: prompt.to_string(), priority, enqueued_at: now, status: JobStatus::Queued });
        id
    }

    pub fn mark_running(&mut self, id: &str) {
        if let Some(job) = self.jobs.get_mut(id) { job.status = JobStatus::Running; }
    }

    pub fn mark_completed(&mut self, id: &str, output: &str) {
        if let Some(job) = self.jobs.get_mut(id) { job.status = JobStatus::Completed(output.to_string()); }
    }

    pub fn poll(&self, id: &str) -> Option<&Job> { self.jobs.get(id) }

    pub fn dequeue_next(&mut self) -> Option<Job> {
        let id = self.jobs.values()
            .filter(|j| j.status == JobStatus::Queued)
            .max_by(|a, b| a.priority.cmp(&b.priority).then(b.enqueued_at.cmp(&a.enqueued_at)))
            .map(|j| j.id.clone())?;
        let mut job = self.jobs.remove(&id)?;
        job.status = JobStatus::Running;
        Some(job)
    }

    pub fn pending_count(&self) -> usize {
        self.jobs.values().filter(|j| j.status == JobStatus::Queued).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_returns_id() {
        let mut q = DispatchQueue::new();
        let id = q.enqueue("do something", 0);
        assert!(!id.is_empty());
    }

    #[test]
    fn test_pending_count_increases() {
        let mut q = DispatchQueue::new();
        q.enqueue("job1", 0);
        q.enqueue("job2", 0);
        assert_eq!(q.pending_count(), 2);
    }

    #[test]
    fn test_dequeue_next_returns_highest_priority() {
        let mut q = DispatchQueue::new();
        q.enqueue_with_priority("low", 10, 0);
        q.enqueue_with_priority("high", 200, 1);
        let job = q.dequeue_next().unwrap();
        assert_eq!(job.prompt, "high");
        assert_eq!(job.status, JobStatus::Running);
    }

    #[test]
    fn test_dequeue_same_priority_earlier_first() {
        let mut q = DispatchQueue::new();
        q.enqueue_with_priority("first", 100, 0);  // enqueued_at = 0
        q.enqueue_with_priority("second", 100, 10); // enqueued_at = 10 (later)
        let job = q.dequeue_next().unwrap();
        // earlier enqueued (lower enqueued_at) wins
        assert_eq!(job.prompt, "first");
    }

    #[test]
    fn test_dequeue_empty_returns_none() {
        let mut q = DispatchQueue::new();
        assert!(q.dequeue_next().is_none());
    }

    #[test]
    fn test_mark_running_and_poll() {
        let mut q = DispatchQueue::new();
        let id = q.enqueue("task", 0);
        q.mark_running(&id);
        let job = q.poll(&id).unwrap();
        assert_eq!(job.status, JobStatus::Running);
    }

    #[test]
    fn test_mark_completed() {
        let mut q = DispatchQueue::new();
        let id = q.enqueue("task", 0);
        q.mark_completed(&id, "result");
        let job = q.poll(&id).unwrap();
        assert_eq!(job.status, JobStatus::Completed("result".to_string()));
    }

    #[test]
    fn test_running_jobs_not_in_pending_count() {
        let mut q = DispatchQueue::new();
        q.enqueue("a", 0);
        q.enqueue("b", 0);
        let _ = q.dequeue_next(); // marks one as running and removes from queue
        assert_eq!(q.pending_count(), 1);
    }
}
