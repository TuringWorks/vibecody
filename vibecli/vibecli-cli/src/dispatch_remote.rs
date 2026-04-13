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
