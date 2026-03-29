#![allow(dead_code)]
//! Code Replay — time-travel through agent edits with branching and forking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EditType {
    Insert,
    Delete,
    Replace,
    FileCreate,
    FileDelete,
    FileRename,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplayState {
    Recording,
    Paused,
    Playing,
    Scrubbing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BranchStatus {
    Active,
    Merged,
    Abandoned,
    Forked,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditStep {
    pub id: String,
    pub step_number: usize,
    pub edit_type: EditType,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub old_content: String,
    pub new_content: String,
    pub reasoning: String,
    pub timestamp: u64,
    pub test_passed: Option<bool>,
    pub token_cost: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineBranch {
    pub id: String,
    pub name: String,
    pub parent_branch: Option<String>,
    pub fork_point: usize,
    pub steps: Vec<EditStep>,
    pub status: BranchStatus,
    pub created_at: u64,
    pub test_results: HashMap<usize, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub id: String,
    pub name: String,
    pub branches: HashMap<String, TimelineBranch>,
    pub active_branch: String,
    pub created_at: u64,
    pub total_steps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySession {
    pub timeline: Timeline,
    pub state: ReplayState,
    pub current_position: usize,
    pub playback_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineExport {
    pub format: String,
    pub content: String,
    pub step_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplayMetrics {
    pub total_timelines: u64,
    pub total_steps: u64,
    pub total_branches: u64,
    pub total_forks: u64,
    pub avg_steps_per_timeline: f64,
}

// ---------------------------------------------------------------------------
// DiffGenerator
// ---------------------------------------------------------------------------

pub struct DiffGenerator;

impl DiffGenerator {
    /// Generate a unified diff between old and new content for a given file path.
    pub fn generate_unified(old: &str, new: &str, file_path: &str) -> String {
        let old_lines: Vec<&str> = if old.is_empty() {
            Vec::new()
        } else {
            old.lines().collect()
        };
        let new_lines: Vec<&str> = if new.is_empty() {
            Vec::new()
        } else {
            new.lines().collect()
        };

        let mut diff = String::new();
        diff.push_str(&format!("--- a/{}\n", file_path));
        diff.push_str(&format!("+++ b/{}\n", file_path));
        diff.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            if old_lines.is_empty() { 0 } else { 1 },
            old_lines.len(),
            if new_lines.is_empty() { 0 } else { 1 },
            new_lines.len()
        ));

        // Simple line-by-line diff using LCS
        let lcs = Self::lcs_table(&old_lines, &new_lines);
        let mut i = old_lines.len();
        let mut j = new_lines.len();
        let mut result_lines: Vec<String> = Vec::new();

        while i > 0 || j > 0 {
            if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
                result_lines.push(format!(" {}", old_lines[i - 1]));
                i -= 1;
                j -= 1;
            } else if j > 0 && (i == 0 || lcs[i][j - 1] >= lcs[i - 1][j]) {
                result_lines.push(format!("+{}", new_lines[j - 1]));
                j -= 1;
            } else if i > 0 {
                result_lines.push(format!("-{}", old_lines[i - 1]));
                i -= 1;
            }
        }

        result_lines.reverse();
        for line in &result_lines {
            diff.push_str(line);
            diff.push('\n');
        }

        diff
    }

    fn lcs_table(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
        let m = a.len();
        let n = b.len();
        let mut table = vec![vec![0usize; n + 1]; m + 1];
        for i in 1..=m {
            for j in 1..=n {
                if a[i - 1] == b[j - 1] {
                    table[i][j] = table[i - 1][j - 1] + 1;
                } else {
                    table[i][j] = std::cmp::max(table[i - 1][j], table[i][j - 1]);
                }
            }
        }
        table
    }

    /// Generate a unified diff for a single EditStep.
    pub fn generate_step_diff(step: &EditStep) -> String {
        Self::generate_unified(&step.old_content, &step.new_content, &step.file_path)
    }

    /// Apply a sequence of edit steps to initial content, returning the final content.
    pub fn apply_steps(initial: &str, steps: &[&EditStep]) -> String {
        let mut content = initial.to_string();
        for step in steps {
            match step.edit_type {
                EditType::Insert => {
                    let mut lines: Vec<String> =
                        content.lines().map(|l| l.to_string()).collect();
                    let insert_at = step.line_start.min(lines.len());
                    for (i, new_line) in step.new_content.lines().enumerate() {
                        lines.insert(insert_at + i, new_line.to_string());
                    }
                    content = lines.join("\n");
                }
                EditType::Delete => {
                    let mut lines: Vec<String> =
                        content.lines().map(|l| l.to_string()).collect();
                    let start = step.line_start.min(lines.len());
                    let end = step.line_end.min(lines.len());
                    if start < end {
                        lines.drain(start..end);
                    }
                    content = lines.join("\n");
                }
                EditType::Replace => {
                    let mut lines: Vec<String> =
                        content.lines().map(|l| l.to_string()).collect();
                    let start = step.line_start.min(lines.len());
                    let end = step.line_end.min(lines.len());
                    if start < end {
                        lines.drain(start..end);
                    }
                    for (i, new_line) in step.new_content.lines().enumerate() {
                        lines.insert(start + i, new_line.to_string());
                    }
                    content = lines.join("\n");
                }
                EditType::FileCreate => {
                    content = step.new_content.clone();
                }
                EditType::FileDelete => {
                    content = String::new();
                }
                EditType::FileRename => {
                    // Content stays the same; only the path changes.
                }
            }
        }
        content
    }
}

// ---------------------------------------------------------------------------
// ReplayEngine
// ---------------------------------------------------------------------------

pub struct ReplayEngine {
    pub sessions: HashMap<String, ReplaySession>,
    pub metrics: ReplayMetrics,
    next_id: u64,
    timestamp_counter: u64,
}

impl ReplayEngine {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            metrics: ReplayMetrics::default(),
            next_id: 1,
            timestamp_counter: 1,
        }
    }

    fn next_id(&mut self) -> String {
        let id = format!("tl-{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn next_branch_id(&mut self) -> String {
        let id = format!("br-{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn next_ts(&mut self) -> u64 {
        let ts = self.timestamp_counter;
        self.timestamp_counter += 1;
        ts
    }

    fn update_metrics(&mut self) {
        let total_timelines = self.sessions.len() as u64;
        let mut total_steps: u64 = 0;
        let mut total_branches: u64 = 0;
        let mut total_forks: u64 = 0;

        for session in self.sessions.values() {
            let tl = &session.timeline;
            total_branches += tl.branches.len() as u64;
            for branch in tl.branches.values() {
                total_steps += branch.steps.len() as u64;
                if branch.parent_branch.is_some() {
                    total_forks += 1;
                }
            }
        }

        self.metrics.total_timelines = total_timelines;
        self.metrics.total_steps = total_steps;
        self.metrics.total_branches = total_branches;
        self.metrics.total_forks = total_forks;
        self.metrics.avg_steps_per_timeline = if total_timelines > 0 {
            total_steps as f64 / total_timelines as f64
        } else {
            0.0
        };
    }

    /// Create a new timeline and return its id.
    pub fn create_timeline(&mut self, name: &str) -> String {
        let tl_id = self.next_id();
        let ts = self.next_ts();
        let main_branch_id = self.next_branch_id();

        let main_branch = TimelineBranch {
            id: main_branch_id.clone(),
            name: "main".to_string(),
            parent_branch: None,
            fork_point: 0,
            steps: Vec::new(),
            status: BranchStatus::Active,
            created_at: ts,
            test_results: HashMap::new(),
        };

        let mut branches = HashMap::new();
        branches.insert(main_branch_id.clone(), main_branch);

        let timeline = Timeline {
            id: tl_id.clone(),
            name: name.to_string(),
            branches,
            active_branch: main_branch_id,
            created_at: ts,
            total_steps: 0,
        };

        let session = ReplaySession {
            timeline,
            state: ReplayState::Recording,
            current_position: 0,
            playback_speed: 1.0,
        };

        self.sessions.insert(tl_id.clone(), session);
        self.update_metrics();
        tl_id
    }

    /// Record a step on the active branch of the given timeline.
    pub fn record_step(&mut self, timeline_id: &str, step: EditStep) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let active_id = session.timeline.active_branch.clone();
        let branch = session
            .timeline
            .branches
            .get_mut(&active_id)
            .ok_or_else(|| "Active branch not found".to_string())?;

        if branch.status != BranchStatus::Active {
            return Err("Cannot record on a non-active branch".to_string());
        }

        branch.steps.push(step);
        session.timeline.total_steps += 1;
        session.current_position = branch.steps.len();
        self.update_metrics();
        Ok(())
    }

    /// Fork the active branch at the given step, creating a new branch. Returns the branch id.
    pub fn fork(
        &mut self,
        timeline_id: &str,
        at_step: usize,
        branch_name: &str,
    ) -> Result<String, String> {
        let session = self
            .sessions
            .get_mut(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let active_id = session.timeline.active_branch.clone();
        let parent_branch = session
            .timeline
            .branches
            .get(&active_id)
            .ok_or_else(|| "Active branch not found".to_string())?;

        if at_step > parent_branch.steps.len() {
            return Err(format!(
                "Fork point {} exceeds branch length {}",
                at_step,
                parent_branch.steps.len()
            ));
        }

        // Check for duplicate branch names
        for b in session.timeline.branches.values() {
            if b.name == branch_name {
                return Err(format!("Branch name '{}' already exists", branch_name));
            }
        }

        let forked_steps: Vec<EditStep> = parent_branch.steps[..at_step].to_vec();
        let ts = self.next_ts();
        let branch_id = self.next_branch_id();

        let new_branch = TimelineBranch {
            id: branch_id.clone(),
            name: branch_name.to_string(),
            parent_branch: Some(active_id),
            fork_point: at_step,
            steps: forked_steps,
            status: BranchStatus::Active,
            created_at: ts,
            test_results: HashMap::new(),
        };

        let session = self.sessions.get_mut(timeline_id).expect("checked above");
        session
            .timeline
            .branches
            .insert(branch_id.clone(), new_branch);
        session.timeline.active_branch = branch_id.clone();
        session.current_position = at_step;
        self.update_metrics();
        Ok(branch_id)
    }

    /// Switch to an existing branch.
    pub fn switch_branch(
        &mut self,
        timeline_id: &str,
        branch_id: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let branch = session
            .timeline
            .branches
            .get(branch_id)
            .ok_or_else(|| format!("Branch '{}' not found", branch_id))?;

        if branch.status == BranchStatus::Abandoned {
            return Err("Cannot switch to an abandoned branch".to_string());
        }

        let pos = branch.steps.len();
        session.timeline.active_branch = branch_id.to_string();
        session.current_position = pos;
        Ok(())
    }

    /// Merge a branch into the active branch by appending its steps after the fork point.
    pub fn merge_branch(
        &mut self,
        timeline_id: &str,
        branch_id: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let active_id = session.timeline.active_branch.clone();
        if active_id == branch_id {
            return Err("Cannot merge a branch into itself".to_string());
        }

        let source = session
            .timeline
            .branches
            .get(branch_id)
            .ok_or_else(|| format!("Branch '{}' not found", branch_id))?;

        if source.status == BranchStatus::Merged {
            return Err("Branch is already merged".to_string());
        }

        let fork_point = source.fork_point;
        let new_steps: Vec<EditStep> = source.steps[fork_point..].to_vec();
        let added = new_steps.len();

        let session = self.sessions.get_mut(timeline_id).expect("checked above");

        let target = session
            .timeline
            .branches
            .get_mut(&active_id)
            .ok_or_else(|| "Active branch not found".to_string())?;

        target.steps.extend(new_steps);
        session.timeline.total_steps += added;
        session.current_position = target.steps.len();

        let source = session
            .timeline
            .branches
            .get_mut(branch_id)
            .expect("checked above");
        source.status = BranchStatus::Merged;

        self.update_metrics();
        Ok(())
    }

    /// Scrub to a specific step position in the active branch.
    pub fn scrub_to(
        &mut self,
        timeline_id: &str,
        step: usize,
    ) -> Result<&EditStep, String> {
        let session = self
            .sessions
            .get_mut(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let active_id = session.timeline.active_branch.clone();
        let branch = session
            .timeline
            .branches
            .get(&active_id)
            .ok_or_else(|| "Active branch not found".to_string())?;

        if step == 0 || step > branch.steps.len() {
            return Err(format!(
                "Step {} out of range (1..{})",
                step,
                branch.steps.len()
            ));
        }

        session.state = ReplayState::Scrubbing;
        session.current_position = step;

        let session = self.sessions.get(timeline_id).expect("checked above");
        let branch = session
            .timeline
            .branches
            .get(&session.timeline.active_branch)
            .expect("checked above");
        Ok(&branch.steps[step - 1])
    }

    /// Get a step by 1-based index from the active branch.
    pub fn get_step(&self, timeline_id: &str, step: usize) -> Option<&EditStep> {
        let session = self.sessions.get(timeline_id)?;
        let branch = session
            .timeline
            .branches
            .get(&session.timeline.active_branch)?;
        if step == 0 || step > branch.steps.len() {
            return None;
        }
        Some(&branch.steps[step - 1])
    }

    /// Get the unified diff at a given step.
    pub fn get_diff_at(
        &self,
        timeline_id: &str,
        step: usize,
    ) -> Result<String, String> {
        let session = self
            .sessions
            .get(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let branch = session
            .timeline
            .branches
            .get(&session.timeline.active_branch)
            .ok_or_else(|| "Active branch not found".to_string())?;

        if step == 0 || step > branch.steps.len() {
            return Err(format!(
                "Step {} out of range (1..{})",
                step,
                branch.steps.len()
            ));
        }

        let edit = &branch.steps[step - 1];
        Ok(DiffGenerator::generate_step_diff(edit))
    }

    /// Get the reasoning string at a given step.
    pub fn get_reasoning_at(
        &self,
        timeline_id: &str,
        step: usize,
    ) -> Result<String, String> {
        let session = self
            .sessions
            .get(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let branch = session
            .timeline
            .branches
            .get(&session.timeline.active_branch)
            .ok_or_else(|| "Active branch not found".to_string())?;

        if step == 0 || step > branch.steps.len() {
            return Err(format!(
                "Step {} out of range (1..{})",
                step,
                branch.steps.len()
            ));
        }

        Ok(branch.steps[step - 1].reasoning.clone())
    }

    /// List all timelines (id, timeline reference).
    pub fn list_timelines(&self) -> Vec<(&String, &Timeline)> {
        self.sessions
            .iter()
            .map(|(id, s)| (id, &s.timeline))
            .collect()
    }

    /// List all branches for a timeline.
    pub fn list_branches(
        &self,
        timeline_id: &str,
    ) -> Result<Vec<&TimelineBranch>, String> {
        let session = self
            .sessions
            .get(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        Ok(session.timeline.branches.values().collect())
    }

    /// Get the current scrub/playback position.
    pub fn current_position(&self, timeline_id: &str) -> Result<usize, String> {
        let session = self
            .sessions
            .get(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;
        Ok(session.current_position)
    }

    /// Get total steps on the active branch.
    pub fn total_steps(&self, timeline_id: &str) -> Result<usize, String> {
        let session = self
            .sessions
            .get(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let branch = session
            .timeline
            .branches
            .get(&session.timeline.active_branch)
            .ok_or_else(|| "Active branch not found".to_string())?;

        Ok(branch.steps.len())
    }

    /// Export a timeline to the given format ("json" or "markdown").
    pub fn export_timeline(
        &self,
        timeline_id: &str,
        format: &str,
    ) -> Result<TimelineExport, String> {
        let session = self
            .sessions
            .get(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let tl = &session.timeline;
        let mut step_count = 0usize;
        for branch in tl.branches.values() {
            step_count += branch.steps.len();
        }

        let content = match format {
            "json" => serde_json::to_string_pretty(tl)
                .map_err(|e| format!("JSON serialization failed: {}", e))?,
            "markdown" => {
                let mut md = String::new();
                md.push_str(&format!("# Timeline: {}\n\n", tl.name));
                md.push_str(&format!("- **ID**: {}\n", tl.id));
                md.push_str(&format!("- **Created**: {}\n", tl.created_at));
                md.push_str(&format!("- **Branches**: {}\n\n", tl.branches.len()));

                for branch in tl.branches.values() {
                    md.push_str(&format!("## Branch: {} ({})\n\n", branch.name, branch.id));
                    md.push_str(&format!("Status: {:?}\n\n", branch.status));
                    for (i, step) in branch.steps.iter().enumerate() {
                        md.push_str(&format!(
                            "### Step {} — {:?} `{}`\n\n",
                            i + 1,
                            step.edit_type,
                            step.file_path
                        ));
                        md.push_str(&format!("**Reasoning**: {}\n\n", step.reasoning));
                        if !step.old_content.is_empty() || !step.new_content.is_empty() {
                            md.push_str("```diff\n");
                            md.push_str(&DiffGenerator::generate_step_diff(step));
                            md.push_str("```\n\n");
                        }
                    }
                }
                md
            }
            _ => return Err(format!("Unsupported export format: {}", format)),
        };

        Ok(TimelineExport {
            format: format.to_string(),
            content,
            step_count,
        })
    }

    /// Prune old steps from the active branch, keeping only the last `keep_last` steps.
    /// Returns the number of steps removed.
    pub fn prune(
        &mut self,
        timeline_id: &str,
        keep_last: usize,
    ) -> Result<usize, String> {
        let session = self
            .sessions
            .get_mut(timeline_id)
            .ok_or_else(|| format!("Timeline '{}' not found", timeline_id))?;

        let active_id = session.timeline.active_branch.clone();
        let branch = session
            .timeline
            .branches
            .get_mut(&active_id)
            .ok_or_else(|| "Active branch not found".to_string())?;

        let total = branch.steps.len();
        if total <= keep_last {
            return Ok(0);
        }

        let remove_count = total - keep_last;
        branch.steps.drain(..remove_count);

        // Re-number remaining steps
        for (i, step) in branch.steps.iter_mut().enumerate() {
            step.step_number = i + 1;
        }

        session.timeline.total_steps = session
            .timeline
            .total_steps
            .saturating_sub(remove_count);
        session.current_position = session.current_position.saturating_sub(remove_count);

        self.update_metrics();
        Ok(remove_count)
    }

    /// Delete an entire timeline.
    pub fn delete_timeline(&mut self, timeline_id: &str) -> Result<(), String> {
        if self.sessions.remove(timeline_id).is_none() {
            return Err(format!("Timeline '{}' not found", timeline_id));
        }
        self.update_metrics();
        Ok(())
    }

    /// Get a reference to the current metrics.
    pub fn get_metrics(&self) -> &ReplayMetrics {
        &self.metrics
    }
}

// ---------------------------------------------------------------------------
// Helper for tests
// ---------------------------------------------------------------------------

fn make_step(
    id: &str,
    step_number: usize,
    edit_type: EditType,
    file_path: &str,
    old: &str,
    new: &str,
    reasoning: &str,
) -> EditStep {
    EditStep {
        id: id.to_string(),
        step_number,
        edit_type,
        file_path: file_path.to_string(),
        line_start: 0,
        line_end: 0,
        old_content: old.to_string(),
        new_content: new.to_string(),
        reasoning: reasoning.to_string(),
        timestamp: 0,
        test_passed: None,
        token_cost: 10,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> ReplayEngine {
        ReplayEngine::new()
    }

    fn sample_step(n: usize) -> EditStep {
        make_step(
            &format!("s{}", n),
            n,
            EditType::Replace,
            "src/main.rs",
            &format!("old line {}", n),
            &format!("new line {}", n),
            &format!("reason {}", n),
        )
    }

    // -----------------------------------------------------------------------
    // Timeline creation & deletion
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_timeline() {
        let mut e = engine();
        let id = e.create_timeline("my-tl");
        assert!(e.sessions.contains_key(&id));
        assert_eq!(e.sessions[&id].timeline.name, "my-tl");
    }

    #[test]
    fn test_create_multiple_timelines() {
        let mut e = engine();
        let a = e.create_timeline("alpha");
        let b = e.create_timeline("beta");
        assert_ne!(a, b);
        assert_eq!(e.list_timelines().len(), 2);
    }

    #[test]
    fn test_delete_timeline() {
        let mut e = engine();
        let id = e.create_timeline("temp");
        assert!(e.delete_timeline(&id).is_ok());
        assert!(!e.sessions.contains_key(&id));
    }

    #[test]
    fn test_delete_nonexistent_timeline() {
        let mut e = engine();
        assert!(e.delete_timeline("nope").is_err());
    }

    #[test]
    fn test_timeline_has_main_branch() {
        let mut e = engine();
        let id = e.create_timeline("t");
        let session = &e.sessions[&id];
        let branch = session
            .timeline
            .branches
            .get(&session.timeline.active_branch)
            .unwrap();
        assert_eq!(branch.name, "main");
        assert_eq!(branch.status, BranchStatus::Active);
    }

    // -----------------------------------------------------------------------
    // Step recording
    // -----------------------------------------------------------------------

    #[test]
    fn test_record_single_step() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert!(e.record_step(&id, sample_step(1)).is_ok());
        assert_eq!(e.total_steps(&id).unwrap(), 1);
    }

    #[test]
    fn test_record_multiple_steps() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=5 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        assert_eq!(e.total_steps(&id).unwrap(), 5);
    }

    #[test]
    fn test_record_step_invalid_timeline() {
        let mut e = engine();
        assert!(e.record_step("bad", sample_step(1)).is_err());
    }

    #[test]
    fn test_current_position_after_recording() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        e.record_step(&id, sample_step(2)).unwrap();
        assert_eq!(e.current_position(&id).unwrap(), 2);
    }

    #[test]
    fn test_record_step_updates_total() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        assert_eq!(e.sessions[&id].timeline.total_steps, 1);
    }

    // -----------------------------------------------------------------------
    // Get step
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_step_valid() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        let step = e.get_step(&id, 1).unwrap();
        assert_eq!(step.id, "s1");
    }

    #[test]
    fn test_get_step_zero_returns_none() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        assert!(e.get_step(&id, 0).is_none());
    }

    #[test]
    fn test_get_step_out_of_range() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        assert!(e.get_step(&id, 99).is_none());
    }

    #[test]
    fn test_get_step_bad_timeline() {
        let e = engine();
        assert!(e.get_step("nope", 1).is_none());
    }

    // -----------------------------------------------------------------------
    // Scrubbing
    // -----------------------------------------------------------------------

    #[test]
    fn test_scrub_to_valid() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let step = e.scrub_to(&id, 2).unwrap();
        assert_eq!(step.id, "s2");
        assert_eq!(e.current_position(&id).unwrap(), 2);
    }

    #[test]
    fn test_scrub_to_first_step() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let step = e.scrub_to(&id, 1).unwrap();
        assert_eq!(step.id, "s1");
    }

    #[test]
    fn test_scrub_to_last_step() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let step = e.scrub_to(&id, 3).unwrap();
        assert_eq!(step.id, "s3");
    }

    #[test]
    fn test_scrub_to_zero_fails() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        assert!(e.scrub_to(&id, 0).is_err());
    }

    #[test]
    fn test_scrub_out_of_range() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        assert!(e.scrub_to(&id, 5).is_err());
    }

    #[test]
    fn test_scrub_sets_state() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        e.scrub_to(&id, 1).unwrap();
        assert_eq!(e.sessions[&id].state, ReplayState::Scrubbing);
    }

    #[test]
    fn test_scrub_invalid_timeline() {
        let mut e = engine();
        assert!(e.scrub_to("nope", 1).is_err());
    }

    // -----------------------------------------------------------------------
    // Fork
    // -----------------------------------------------------------------------

    #[test]
    fn test_fork_creates_branch() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let br = e.fork(&id, 2, "experiment").unwrap();
        let branches = e.list_branches(&id).unwrap();
        assert_eq!(branches.len(), 2);
        assert!(e.sessions[&id].timeline.branches.contains_key(&br));
    }

    #[test]
    fn test_fork_copies_steps_up_to_point() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=5 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let br = e.fork(&id, 3, "exp").unwrap();
        let branch = &e.sessions[&id].timeline.branches[&br];
        assert_eq!(branch.steps.len(), 3);
        assert_eq!(branch.fork_point, 3);
    }

    #[test]
    fn test_fork_switches_active_branch() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        let br = e.fork(&id, 1, "f").unwrap();
        assert_eq!(e.sessions[&id].timeline.active_branch, br);
    }

    #[test]
    fn test_fork_at_zero() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        let br = e.fork(&id, 0, "empty-fork").unwrap();
        let branch = &e.sessions[&id].timeline.branches[&br];
        assert_eq!(branch.steps.len(), 0);
    }

    #[test]
    fn test_fork_out_of_range() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        assert!(e.fork(&id, 99, "bad").is_err());
    }

    #[test]
    fn test_fork_duplicate_name() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        e.fork(&id, 1, "exp").unwrap();
        // Switch back to main to fork again — but "exp" already exists
        let main_id = e
            .sessions[&id]
            .timeline
            .branches
            .values()
            .find(|b| b.name == "main")
            .unwrap()
            .id
            .clone();
        e.switch_branch(&id, &main_id).unwrap();
        assert!(e.fork(&id, 1, "exp").is_err());
    }

    #[test]
    fn test_fork_invalid_timeline() {
        let mut e = engine();
        assert!(e.fork("nope", 0, "x").is_err());
    }

    // -----------------------------------------------------------------------
    // Switch branch
    // -----------------------------------------------------------------------

    #[test]
    fn test_switch_branch() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();

        let main_id = e.sessions[&id].timeline.active_branch.clone();
        let br = e.fork(&id, 1, "alt").unwrap();
        assert_eq!(e.sessions[&id].timeline.active_branch, br);

        e.switch_branch(&id, &main_id).unwrap();
        assert_eq!(e.sessions[&id].timeline.active_branch, main_id);
    }

    #[test]
    fn test_switch_to_nonexistent_branch() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert!(e.switch_branch(&id, "no-such").is_err());
    }

    #[test]
    fn test_switch_to_abandoned_branch_fails() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        let br = e.fork(&id, 1, "doomed").unwrap();

        // Manually abandon the branch
        e.sessions
            .get_mut(&id)
            .unwrap()
            .timeline
            .branches
            .get_mut(&br)
            .unwrap()
            .status = BranchStatus::Abandoned;

        let main_id = e
            .sessions[&id]
            .timeline
            .branches
            .values()
            .find(|b| b.name == "main")
            .unwrap()
            .id
            .clone();
        e.switch_branch(&id, &main_id).unwrap();
        assert!(e.switch_branch(&id, &br).is_err());
    }

    #[test]
    fn test_switch_branch_invalid_timeline() {
        let mut e = engine();
        assert!(e.switch_branch("nope", "br").is_err());
    }

    // -----------------------------------------------------------------------
    // Merge branch
    // -----------------------------------------------------------------------

    #[test]
    fn test_merge_branch() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let main_id = e.sessions[&id].timeline.active_branch.clone();
        let br = e.fork(&id, 2, "feature").unwrap();
        // Add a step on the fork
        e.record_step(&id, sample_step(10)).unwrap();

        // Switch back and merge
        e.switch_branch(&id, &main_id).unwrap();
        e.merge_branch(&id, &br).unwrap();

        // Main should have original 3 + 1 new from fork (after fork_point=2)
        assert_eq!(e.total_steps(&id).unwrap(), 4);
    }

    #[test]
    fn test_merge_marks_source_merged() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        let main_id = e.sessions[&id].timeline.active_branch.clone();
        let br = e.fork(&id, 1, "f").unwrap();
        e.record_step(&id, sample_step(2)).unwrap();
        e.switch_branch(&id, &main_id).unwrap();
        e.merge_branch(&id, &br).unwrap();

        assert_eq!(
            e.sessions[&id].timeline.branches[&br].status,
            BranchStatus::Merged
        );
    }

    #[test]
    fn test_merge_into_self_fails() {
        let mut e = engine();
        let id = e.create_timeline("t");
        let active = e.sessions[&id].timeline.active_branch.clone();
        assert!(e.merge_branch(&id, &active).is_err());
    }

    #[test]
    fn test_merge_already_merged_fails() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        let main_id = e.sessions[&id].timeline.active_branch.clone();
        let br = e.fork(&id, 1, "f").unwrap();
        e.switch_branch(&id, &main_id).unwrap();
        e.merge_branch(&id, &br).unwrap();
        assert!(e.merge_branch(&id, &br).is_err());
    }

    #[test]
    fn test_merge_invalid_timeline() {
        let mut e = engine();
        assert!(e.merge_branch("nope", "br").is_err());
    }

    #[test]
    fn test_merge_invalid_branch() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert!(e.merge_branch(&id, "nope").is_err());
    }

    // -----------------------------------------------------------------------
    // Diff generation
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_diff_at() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        let diff = e.get_diff_at(&id, 1).unwrap();
        assert!(diff.contains("--- a/src/main.rs"));
        assert!(diff.contains("+++ b/src/main.rs"));
    }

    #[test]
    fn test_get_diff_out_of_range() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert!(e.get_diff_at(&id, 1).is_err());
    }

    #[test]
    fn test_get_diff_invalid_timeline() {
        let e = engine();
        assert!(e.get_diff_at("nope", 1).is_err());
    }

    #[test]
    fn test_diff_generator_unified() {
        let diff = DiffGenerator::generate_unified("foo\nbar", "foo\nbaz", "test.rs");
        assert!(diff.contains("--- a/test.rs"));
        assert!(diff.contains("-bar"));
        assert!(diff.contains("+baz"));
    }

    #[test]
    fn test_diff_generator_empty_old() {
        let diff = DiffGenerator::generate_unified("", "hello", "new.rs");
        assert!(diff.contains("+hello"));
    }

    #[test]
    fn test_diff_generator_empty_new() {
        let diff = DiffGenerator::generate_unified("hello", "", "old.rs");
        assert!(diff.contains("-hello"));
    }

    #[test]
    fn test_diff_generator_step_diff() {
        let step = sample_step(1);
        let diff = DiffGenerator::generate_step_diff(&step);
        assert!(diff.contains("src/main.rs"));
    }

    // -----------------------------------------------------------------------
    // Apply steps
    // -----------------------------------------------------------------------

    #[test]
    fn test_apply_steps_file_create() {
        let step = make_step("c", 1, EditType::FileCreate, "f.rs", "", "hello world", "create");
        let result = DiffGenerator::apply_steps("", &[&step]);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_apply_steps_file_delete() {
        let step = make_step("d", 1, EditType::FileDelete, "f.rs", "stuff", "", "delete");
        let result = DiffGenerator::apply_steps("stuff", &[&step]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_apply_steps_insert() {
        let mut step = make_step("i", 1, EditType::Insert, "f.rs", "", "inserted", "ins");
        step.line_start = 1;
        let result = DiffGenerator::apply_steps("line0\nline1", &[&step]);
        assert!(result.contains("inserted"));
    }

    #[test]
    fn test_apply_steps_replace() {
        let mut step = make_step("r", 1, EditType::Replace, "f.rs", "old", "replacement", "rep");
        step.line_start = 0;
        step.line_end = 1;
        let result = DiffGenerator::apply_steps("original\nkeep", &[&step]);
        assert!(result.contains("replacement"));
        assert!(result.contains("keep"));
    }

    #[test]
    fn test_apply_steps_sequential() {
        let s1 = make_step("c", 1, EditType::FileCreate, "f.rs", "", "aaa\nbbb\nccc", "create");
        let mut s2 = make_step("r", 2, EditType::Replace, "f.rs", "bbb", "xxx", "replace");
        s2.line_start = 1;
        s2.line_end = 2;
        let result = DiffGenerator::apply_steps("", &[&s1, &s2]);
        assert!(result.contains("xxx"));
        assert!(!result.contains("bbb"));
    }

    #[test]
    fn test_apply_steps_rename_preserves_content() {
        let step = make_step("rn", 1, EditType::FileRename, "new.rs", "", "", "rename");
        let result = DiffGenerator::apply_steps("keep me", &[&step]);
        assert_eq!(result, "keep me");
    }

    // -----------------------------------------------------------------------
    // Reasoning retrieval
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_reasoning_at() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        assert_eq!(e.get_reasoning_at(&id, 1).unwrap(), "reason 1");
    }

    #[test]
    fn test_get_reasoning_out_of_range() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert!(e.get_reasoning_at(&id, 1).is_err());
    }

    #[test]
    fn test_get_reasoning_invalid_timeline() {
        let e = engine();
        assert!(e.get_reasoning_at("nope", 1).is_err());
    }

    // -----------------------------------------------------------------------
    // Export
    // -----------------------------------------------------------------------

    #[test]
    fn test_export_json() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        let export = e.export_timeline(&id, "json").unwrap();
        assert_eq!(export.format, "json");
        assert!(export.content.contains("\"name\""));
        assert_eq!(export.step_count, 1);
    }

    #[test]
    fn test_export_markdown() {
        let mut e = engine();
        let id = e.create_timeline("test-tl");
        e.record_step(&id, sample_step(1)).unwrap();
        let export = e.export_timeline(&id, "markdown").unwrap();
        assert_eq!(export.format, "markdown");
        assert!(export.content.contains("# Timeline: test-tl"));
        assert!(export.content.contains("Step 1"));
    }

    #[test]
    fn test_export_invalid_format() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert!(e.export_timeline(&id, "xml").is_err());
    }

    #[test]
    fn test_export_invalid_timeline() {
        let e = engine();
        assert!(e.export_timeline("nope", "json").is_err());
    }

    #[test]
    fn test_export_empty_timeline() {
        let mut e = engine();
        let id = e.create_timeline("empty");
        let export = e.export_timeline(&id, "json").unwrap();
        assert_eq!(export.step_count, 0);
    }

    // -----------------------------------------------------------------------
    // Pruning
    // -----------------------------------------------------------------------

    #[test]
    fn test_prune_removes_old_steps() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=10 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let removed = e.prune(&id, 3).unwrap();
        assert_eq!(removed, 7);
        assert_eq!(e.total_steps(&id).unwrap(), 3);
    }

    #[test]
    fn test_prune_keep_all() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let removed = e.prune(&id, 10).unwrap();
        assert_eq!(removed, 0);
        assert_eq!(e.total_steps(&id).unwrap(), 3);
    }

    #[test]
    fn test_prune_keep_zero() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=5 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let removed = e.prune(&id, 0).unwrap();
        assert_eq!(removed, 5);
        assert_eq!(e.total_steps(&id).unwrap(), 0);
    }

    #[test]
    fn test_prune_invalid_timeline() {
        let mut e = engine();
        assert!(e.prune("nope", 5).is_err());
    }

    // -----------------------------------------------------------------------
    // Metrics
    // -----------------------------------------------------------------------

    #[test]
    fn test_metrics_initial() {
        let e = engine();
        let m = e.get_metrics();
        assert_eq!(m.total_timelines, 0);
        assert_eq!(m.total_steps, 0);
    }

    #[test]
    fn test_metrics_after_create() {
        let mut e = engine();
        e.create_timeline("a");
        e.create_timeline("b");
        let m = e.get_metrics();
        assert_eq!(m.total_timelines, 2);
        assert_eq!(m.total_branches, 2);
    }

    #[test]
    fn test_metrics_after_steps() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=4 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let m = e.get_metrics();
        assert_eq!(m.total_steps, 4);
        assert!((m.avg_steps_per_timeline - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_after_fork() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        e.fork(&id, 1, "f1").unwrap();
        let m = e.get_metrics();
        assert_eq!(m.total_forks, 1);
        assert_eq!(m.total_branches, 2);
    }

    #[test]
    fn test_metrics_after_delete() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        e.delete_timeline(&id).unwrap();
        let m = e.get_metrics();
        assert_eq!(m.total_timelines, 0);
        assert_eq!(m.total_steps, 0);
    }

    #[test]
    fn test_metrics_avg_with_multiple_timelines() {
        let mut e = engine();
        let a = e.create_timeline("a");
        let b = e.create_timeline("b");
        for i in 1..=6 {
            e.record_step(&a, sample_step(i)).unwrap();
        }
        for i in 1..=4 {
            e.record_step(&b, sample_step(i)).unwrap();
        }
        let m = e.get_metrics();
        assert!((m.avg_steps_per_timeline - 5.0).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // List timelines & branches
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_timelines_empty() {
        let e = engine();
        assert!(e.list_timelines().is_empty());
    }

    #[test]
    fn test_list_branches() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        e.fork(&id, 1, "alt").unwrap();
        let branches = e.list_branches(&id).unwrap();
        assert_eq!(branches.len(), 2);
    }

    #[test]
    fn test_list_branches_invalid_timeline() {
        let e = engine();
        assert!(e.list_branches("nope").is_err());
    }

    // -----------------------------------------------------------------------
    // Current position & total steps
    // -----------------------------------------------------------------------

    #[test]
    fn test_current_position_invalid_timeline() {
        let e = engine();
        assert!(e.current_position("nope").is_err());
    }

    #[test]
    fn test_total_steps_empty() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert_eq!(e.total_steps(&id).unwrap(), 0);
    }

    #[test]
    fn test_total_steps_invalid_timeline() {
        let e = engine();
        assert!(e.total_steps("nope").is_err());
    }

    // -----------------------------------------------------------------------
    // State transitions
    // -----------------------------------------------------------------------

    #[test]
    fn test_initial_state_is_recording() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert_eq!(e.sessions[&id].state, ReplayState::Recording);
    }

    #[test]
    fn test_playback_speed_default() {
        let mut e = engine();
        let id = e.create_timeline("t");
        assert!((e.sessions[&id].playback_speed - 1.0).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // EditStep field coverage
    // -----------------------------------------------------------------------

    #[test]
    fn test_edit_step_test_passed() {
        let mut step = sample_step(1);
        step.test_passed = Some(true);
        assert_eq!(step.test_passed, Some(true));
    }

    #[test]
    fn test_edit_step_token_cost() {
        let step = sample_step(1);
        assert_eq!(step.token_cost, 10);
    }

    #[test]
    fn test_edit_types_all_variants() {
        let types = vec![
            EditType::Insert,
            EditType::Delete,
            EditType::Replace,
            EditType::FileCreate,
            EditType::FileDelete,
            EditType::FileRename,
        ];
        assert_eq!(types.len(), 6);
    }

    // -----------------------------------------------------------------------
    // Serialization round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn test_edit_step_serialize_roundtrip() {
        let step = sample_step(1);
        let json = serde_json::to_string(&step).unwrap();
        let back: EditStep = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, step.id);
        assert_eq!(back.reasoning, step.reasoning);
    }

    #[test]
    fn test_timeline_branch_serialize_roundtrip() {
        let branch = TimelineBranch {
            id: "br-1".to_string(),
            name: "main".to_string(),
            parent_branch: None,
            fork_point: 0,
            steps: vec![sample_step(1)],
            status: BranchStatus::Active,
            created_at: 100,
            test_results: HashMap::new(),
        };
        let json = serde_json::to_string(&branch).unwrap();
        let back: TimelineBranch = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "main");
    }

    #[test]
    fn test_replay_metrics_serialize() {
        let m = ReplayMetrics {
            total_timelines: 5,
            total_steps: 100,
            total_branches: 10,
            total_forks: 3,
            avg_steps_per_timeline: 20.0,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("100"));
    }

    // -----------------------------------------------------------------------
    // Complex scenarios
    // -----------------------------------------------------------------------

    #[test]
    fn test_fork_record_switch_back() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let main_id = e.sessions[&id].timeline.active_branch.clone();
        let br = e.fork(&id, 2, "exp").unwrap();
        e.record_step(&id, sample_step(10)).unwrap();
        e.record_step(&id, sample_step(11)).unwrap();

        // Forked branch has 2 (copied) + 2 (new) = 4 steps
        assert_eq!(e.total_steps(&id).unwrap(), 4);

        e.switch_branch(&id, &main_id).unwrap();
        // Main branch still has 3
        assert_eq!(e.total_steps(&id).unwrap(), 3);

        // Merge fork back
        e.merge_branch(&id, &br).unwrap();
        // 3 + 2 (steps after fork_point=2) = 5
        assert_eq!(e.total_steps(&id).unwrap(), 5);
    }

    #[test]
    fn test_multiple_forks_from_same_point() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        let main_id = e.sessions[&id].timeline.active_branch.clone();

        e.switch_branch(&id, &main_id).unwrap();
        let _br1 = e.fork(&id, 2, "fork-a").unwrap();
        e.switch_branch(&id, &main_id).unwrap();
        let _br2 = e.fork(&id, 2, "fork-b").unwrap();

        let branches = e.list_branches(&id).unwrap();
        assert_eq!(branches.len(), 3); // main + 2 forks
    }

    #[test]
    fn test_prune_then_record() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=5 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        e.prune(&id, 2).unwrap();
        assert_eq!(e.total_steps(&id).unwrap(), 2);

        e.record_step(&id, sample_step(99)).unwrap();
        assert_eq!(e.total_steps(&id).unwrap(), 3);
    }

    #[test]
    fn test_export_after_fork_counts_all_branches() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=3 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        e.fork(&id, 2, "alt").unwrap();
        e.record_step(&id, sample_step(10)).unwrap();

        let export = e.export_timeline(&id, "json").unwrap();
        // main has 3, alt has 2 (copied) + 1 (new) = 3 => total 6
        assert_eq!(export.step_count, 6);
    }

    #[test]
    fn test_delete_after_fork() {
        let mut e = engine();
        let id = e.create_timeline("t");
        e.record_step(&id, sample_step(1)).unwrap();
        e.fork(&id, 1, "f").unwrap();
        e.delete_timeline(&id).unwrap();
        assert!(e.sessions.is_empty());
    }

    #[test]
    fn test_scrub_forward_and_backward() {
        let mut e = engine();
        let id = e.create_timeline("t");
        for i in 1..=5 {
            e.record_step(&id, sample_step(i)).unwrap();
        }
        // Scrub backward
        let s = e.scrub_to(&id, 2).unwrap();
        assert_eq!(s.id, "s2");
        // Scrub forward
        let s = e.scrub_to(&id, 4).unwrap();
        assert_eq!(s.id, "s4");
        // Back to 1
        let s = e.scrub_to(&id, 1).unwrap();
        assert_eq!(s.id, "s1");
    }
}
