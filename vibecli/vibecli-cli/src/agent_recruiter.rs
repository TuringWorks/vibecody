//! Dynamic agent recruiter — matches tasks to capable agents, manages hiring
//! and releasing agents from the pool. Integrates with `agent_registry`.
//! Matches Devin 2.0's agent recruitment system.
//!
//! Recruitment strategy:
//! 1. Score all candidate agents (capability match × load × priority preference)
//! 2. Assign best candidate, or queue if none available
//! 3. Release agents when tasks complete

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agent_registry::{AgentId, AgentRegistration};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A task that needs an agent assigned to it.
#[derive(Debug, Clone)]
pub struct RecruitmentTask {
    pub task_id: String,
    pub required_capabilities: Vec<String>,
    pub preferred_capabilities: Vec<String>,
    pub priority: TaskPriority,
    pub created_at_ms: u64,
    pub timeout_ms: Option<u64>,
    pub metadata: HashMap<String, String>,
}

impl RecruitmentTask {
    pub fn new(task_id: impl Into<String>, required: &[&str]) -> Self {
        Self {
            task_id: task_id.into(),
            required_capabilities: required.iter().map(|s| s.to_string()).collect(),
            preferred_capabilities: Vec::new(),
            priority: TaskPriority::Normal,
            created_at_ms: now_ms(),
            timeout_ms: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, p: TaskPriority) -> Self { self.priority = p; self }
    pub fn with_preferred(mut self, caps: &[&str]) -> Self {
        self.preferred_capabilities = caps.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn has_timed_out(&self) -> bool {
        if let Some(timeout) = self.timeout_ms {
            now_ms().saturating_sub(self.created_at_ms) > timeout
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Outcome of a recruitment attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecruitmentOutcome {
    /// Task was assigned to the given agent.
    Assigned(AgentId),
    /// No capable agent available — task queued.
    Queued,
    /// No capable agent exists in the registry at all.
    NoCapableAgent,
    /// Task timed out before assignment.
    TimedOut,
}

/// An assignment record.
#[derive(Debug, Clone)]
pub struct Assignment {
    pub task_id: String,
    pub agent_id: AgentId,
    pub assigned_at_ms: u64,
}

// ---------------------------------------------------------------------------
// Recruiter
// ---------------------------------------------------------------------------

/// Recruits agents for tasks using a scoring heuristic.
pub struct AgentRecruiter {
    assignments: HashMap<String, Assignment>,
    queue: VecDeque<RecruitmentTask>,
    pub max_queue_depth: usize,
}

impl Default for AgentRecruiter {
    fn default() -> Self { Self::new() }
}

impl AgentRecruiter {
    pub fn new() -> Self {
        Self {
            assignments: HashMap::new(),
            queue: VecDeque::new(),
            max_queue_depth: 256,
        }
    }

    /// Try to assign `task` to an available agent from `candidates`.
    /// Returns the outcome.
    pub fn recruit(
        &mut self,
        task: RecruitmentTask,
        candidates: &[&AgentRegistration],
    ) -> RecruitmentOutcome {
        if task.has_timed_out() {
            return RecruitmentOutcome::TimedOut;
        }

        // Filter to capable candidates
        let capable: Vec<&&AgentRegistration> = candidates.iter()
            .filter(|a| {
                a.is_available()
                    && task.required_capabilities.iter().all(|cap| a.has_capability(cap))
            })
            .collect();

        if capable.is_empty() {
            // Check if ANY agent (even unavailable) has the capabilities
            let any_capable = candidates.iter()
                .any(|a| task.required_capabilities.iter().all(|cap| a.has_capability(cap)));
            return if any_capable {
                if self.queue.len() < self.max_queue_depth {
                    self.queue.push_back(task);
                    RecruitmentOutcome::Queued
                } else {
                    RecruitmentOutcome::NoCapableAgent
                }
            } else {
                RecruitmentOutcome::NoCapableAgent
            };
        }

        // Score candidates
        let best = capable.iter()
            .max_by(|a, b| {
                let sa = self.score(a, &task);
                let sb = self.score(b, &task);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        let assignment = Assignment {
            task_id: task.task_id.clone(),
            agent_id: best.id.clone(),
            assigned_at_ms: now_ms(),
        };
        self.assignments.insert(task.task_id, assignment);

        RecruitmentOutcome::Assigned(best.id.clone())
    }

    /// Release an assignment when the task completes.
    pub fn release(&mut self, task_id: &str) -> Option<Assignment> {
        self.assignments.remove(task_id)
    }

    /// Drain the queue: try to assign queued tasks given the updated candidate list.
    pub fn drain_queue(&mut self, candidates: &[&AgentRegistration]) -> Vec<(String, AgentId)> {
        let mut assigned = Vec::new();
        let mut remaining: VecDeque<RecruitmentTask> = VecDeque::new();

        while let Some(task) = self.queue.pop_front() {
            if task.has_timed_out() {
                continue; // Drop timed-out tasks
            }
            let task_id = task.task_id.clone();
            let outcome = self.try_assign_direct(&task, candidates);
            if let Some(agent_id) = outcome {
                let assignment = Assignment { task_id: task_id.clone(), agent_id: agent_id.clone(), assigned_at_ms: now_ms() };
                self.assignments.insert(task_id.clone(), assignment);
                assigned.push((task_id, agent_id));
            } else {
                remaining.push_back(task);
            }
        }
        self.queue = remaining;
        assigned
    }

    fn try_assign_direct(&self, task: &RecruitmentTask, candidates: &[&AgentRegistration]) -> Option<AgentId> {
        let capable: Vec<&&AgentRegistration> = candidates.iter()
            .filter(|a| a.is_available() && task.required_capabilities.iter().all(|cap| a.has_capability(cap)))
            .collect();
        capable.iter()
            .max_by(|a, b| {
                let sa = self.score(a, task);
                let sb = self.score(b, task);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|a| a.id.clone())
    }

    /// Score = (1 - load) × 0.5 + preferred_match × 0.3 + priority_bonus × 0.2
    fn score(&self, agent: &&AgentRegistration, task: &RecruitmentTask) -> f32 {
        let load_score = 1.0 - agent.load;
        let preferred_count = task.preferred_capabilities.iter()
            .filter(|cap| agent.has_capability(cap))
            .count();
        let preferred_score = if task.preferred_capabilities.is_empty() {
            1.0
        } else {
            preferred_count as f32 / task.preferred_capabilities.len() as f32
        };
        let priority_bonus = match task.priority {
            TaskPriority::Critical => 0.2,
            TaskPriority::High => 0.1,
            _ => 0.0,
        };
        load_score * 0.5 + preferred_score * 0.3 + priority_bonus * 0.2
    }

    pub fn active_assignment_count(&self) -> usize { self.assignments.len() }
    pub fn queue_depth(&self) -> usize { self.queue.len() }
    pub fn get_assignment(&self, task_id: &str) -> Option<&Assignment> { self.assignments.get(task_id) }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_registry::{AgentHealth, AgentRegistration, Capability};
    use std::collections::HashSet;

    fn make_agent(id: &str, caps: &[&str], load: f32) -> AgentRegistration {
        let mut capabilities = HashSet::new();
        for c in caps { capabilities.insert(Capability::new(*c)); }
        AgentRegistration {
            id: AgentId::new(id),
            name: id.into(),
            version: "1.0.0".into(),
            capabilities,
            metadata: HashMap::new(),
            registered_at_ms: 0,
            last_heartbeat_ms: u64::MAX, // never stale in tests
            health: AgentHealth::Healthy,
            load,
            max_concurrent_tasks: 4,
            current_task_count: 0,
        }
    }

    #[test]
    fn test_recruit_assigns_capable_agent() {
        let mut recruiter = AgentRecruiter::new();
        let agent = make_agent("a1", &[Capability::CODE_EDIT], 0.1);
        let task = RecruitmentTask::new("t1", &[Capability::CODE_EDIT]);
        let outcome = recruiter.recruit(task, &[&agent]);
        assert_eq!(outcome, RecruitmentOutcome::Assigned(AgentId::new("a1")));
    }

    #[test]
    fn test_recruit_no_capable_agent() {
        let mut recruiter = AgentRecruiter::new();
        let agent = make_agent("a1", &[Capability::WEB_SEARCH], 0.1);
        let task = RecruitmentTask::new("t1", &[Capability::CODE_EDIT]);
        let outcome = recruiter.recruit(task, &[&agent]);
        assert_eq!(outcome, RecruitmentOutcome::NoCapableAgent);
    }

    #[test]
    fn test_recruit_queues_when_all_busy() {
        let mut recruiter = AgentRecruiter::new();
        let mut agent = make_agent("a1", &[Capability::CODE_EDIT], 1.0);
        agent.current_task_count = 4; // at max capacity
        let task = RecruitmentTask::new("t1", &[Capability::CODE_EDIT]);
        let outcome = recruiter.recruit(task, &[&agent]);
        assert_eq!(outcome, RecruitmentOutcome::Queued);
        assert_eq!(recruiter.queue_depth(), 1);
    }

    #[test]
    fn test_release_removes_assignment() {
        let mut recruiter = AgentRecruiter::new();
        let agent = make_agent("a1", &[Capability::CODE_EDIT], 0.1);
        recruiter.recruit(RecruitmentTask::new("t1", &[Capability::CODE_EDIT]), &[&agent]);
        recruiter.release("t1");
        assert_eq!(recruiter.active_assignment_count(), 0);
    }

    #[test]
    fn test_prefers_least_loaded() {
        let mut recruiter = AgentRecruiter::new();
        let high = make_agent("high", &[Capability::CODE_EDIT], 0.9);
        let low = make_agent("low", &[Capability::CODE_EDIT], 0.1);
        let outcome = recruiter.recruit(
            RecruitmentTask::new("t1", &[Capability::CODE_EDIT]),
            &[&high, &low],
        );
        assert_eq!(outcome, RecruitmentOutcome::Assigned(AgentId::new("low")));
    }

    #[test]
    fn test_preferred_capabilities_boost() {
        let mut recruiter = AgentRecruiter::new();
        let basic = make_agent("basic", &[Capability::CODE_EDIT], 0.5);
        let full = make_agent("full", &[Capability::CODE_EDIT, Capability::TEST_RUN], 0.5);
        let task = RecruitmentTask::new("t1", &[Capability::CODE_EDIT])
            .with_preferred(&[Capability::TEST_RUN]);
        let outcome = recruiter.recruit(task, &[&basic, &full]);
        assert_eq!(outcome, RecruitmentOutcome::Assigned(AgentId::new("full")));
    }

    #[test]
    fn test_drain_queue_assigns_when_available() {
        let mut recruiter = AgentRecruiter::new();
        let mut busy = make_agent("a1", &[Capability::CODE_EDIT], 1.0);
        busy.current_task_count = 4;
        recruiter.recruit(RecruitmentTask::new("t1", &[Capability::CODE_EDIT]), &[&busy]);
        assert_eq!(recruiter.queue_depth(), 1);

        let free = make_agent("a2", &[Capability::CODE_EDIT], 0.0);
        let assigned = recruiter.drain_queue(&[&free]);
        assert_eq!(assigned.len(), 1);
        assert_eq!(recruiter.queue_depth(), 0);
    }

    #[test]
    fn test_multiple_assignments() {
        let mut recruiter = AgentRecruiter::new();
        let a1 = make_agent("a1", &[Capability::CODE_EDIT], 0.1);
        recruiter.recruit(RecruitmentTask::new("t1", &[Capability::CODE_EDIT]), &[&a1]);
        recruiter.recruit(RecruitmentTask::new("t2", &[Capability::CODE_EDIT]), &[&a1]);
        assert_eq!(recruiter.active_assignment_count(), 2);
    }

    #[test]
    fn test_task_with_priority() {
        let task = RecruitmentTask::new("t1", &[]).with_priority(TaskPriority::Critical);
        assert_eq!(task.priority, TaskPriority::Critical);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_get_assignment() {
        let mut recruiter = AgentRecruiter::new();
        let agent = make_agent("a1", &[Capability::CODE_EDIT], 0.0);
        recruiter.recruit(RecruitmentTask::new("t1", &[Capability::CODE_EDIT]), &[&agent]);
        let a = recruiter.get_assignment("t1").unwrap();
        assert_eq!(a.agent_id.0, "a1");
    }
}
