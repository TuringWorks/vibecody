#![allow(dead_code)]
//! Collaborative review protocol — structured multi-round human-AI code review.

use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewRole {
    Human,
    Agent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommentStatus {
    Open,
    Resolved,
    WontFix,
    Acknowledged,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewRound {
    AgentReviewsHuman,
    HumanReviewsAgent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewDecision {
    Approve,
    RequestChanges,
    Comment,
}

// ── Structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReviewReply {
    pub author_role: ReviewRole,
    pub content: String,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct ReviewComment {
    pub id: String,
    pub author_role: ReviewRole,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub content: String,
    pub severity: String,
    pub status: CommentStatus,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
    pub replies: Vec<ReviewReply>,
}

#[derive(Debug, Clone)]
pub struct ReviewRoundData {
    pub round_number: usize,
    pub round_type: ReviewRound,
    pub comments: Vec<ReviewComment>,
    pub decision: Option<ReviewDecision>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ReviewSession {
    pub id: String,
    pub title: String,
    pub files: Vec<String>,
    pub rounds: Vec<ReviewRoundData>,
    pub current_round: usize,
    pub status: String,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct ChecklistItem {
    pub label: String,
    pub checked: bool,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct ReviewChecklist {
    pub items: Vec<ChecklistItem>,
}

#[derive(Debug, Clone)]
pub struct ReviewQuality {
    pub total_comments: u64,
    pub resolved: u64,
    pub agent_caught_real_issues: u64,
    pub false_positives: u64,
    pub precision: f64,
    pub human_corrections: u64,
}

impl ReviewQuality {
    fn recalculate_precision(&mut self) {
        let denom = self.agent_caught_real_issues + self.false_positives;
        self.precision = if denom == 0 {
            1.0
        } else {
            self.agent_caught_real_issues as f64 / denom as f64
        };
    }
}

#[derive(Debug, Clone)]
pub struct ReviewConfig {
    pub max_rounds: usize,
    pub checklist_enabled: bool,
    pub agent_pushback: bool,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            max_rounds: 5,
            checklist_enabled: true,
            agent_pushback: true,
        }
    }
}

// ── ReviewEngine ───────────────────────────────────────────────────────

pub struct ReviewEngine {
    config: ReviewConfig,
    sessions: HashMap<String, ReviewSession>,
    quality: ReviewQuality,
    checklists: HashMap<String, ReviewChecklist>,
    next_id: u64,
    ts: u64,
}

impl ReviewEngine {
    pub fn new(config: ReviewConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            quality: ReviewQuality {
                total_comments: 0,
                resolved: 0,
                agent_caught_real_issues: 0,
                false_positives: 0,
                precision: 1.0,
                human_corrections: 0,
            },
            checklists: HashMap::new(),
            next_id: 1,
            ts: 1000,
        }
    }

    fn gen_id(&mut self, prefix: &str) -> String {
        let id = format!("{}-{}", prefix, self.next_id);
        self.next_id += 1;
        id
    }

    fn tick(&mut self) -> u64 {
        self.ts += 1;
        self.ts
    }

    pub fn start_session(&mut self, title: &str, files: Vec<String>) -> String {
        let id = self.gen_id("session");
        let ts = self.tick();
        let first_round = ReviewRoundData {
            round_number: 1,
            round_type: ReviewRound::AgentReviewsHuman,
            comments: Vec::new(),
            decision: None,
            started_at: ts,
            completed_at: None,
        };
        let session = ReviewSession {
            id: id.clone(),
            title: title.to_string(),
            files,
            rounds: vec![first_round],
            current_round: 1,
            status: "active".to_string(),
            created_at: ts,
        };
        self.sessions.insert(id.clone(), session);
        id
    }

    pub fn add_comment(
        &mut self,
        session_id: &str,
        comment: ReviewComment,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        if session.status != "active" {
            return Err("Session is not active".to_string());
        }
        let round_idx = session.current_round - 1;
        if round_idx >= session.rounds.len() {
            return Err("Invalid round index".to_string());
        }
        self.quality.total_comments += 1;
        session.rounds[round_idx].comments.push(comment);
        Ok(())
    }

    pub fn reply(
        &mut self,
        session_id: &str,
        comment_id: &str,
        reply: ReviewReply,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        for round in &mut session.rounds {
            for comment in &mut round.comments {
                if comment.id == comment_id {
                    comment.replies.push(reply);
                    return Ok(());
                }
            }
        }
        Err(format!("Comment {} not found", comment_id))
    }

    pub fn resolve(&mut self, session_id: &str, comment_id: &str) -> Result<(), String> {
        let ts = self.tick();
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        for round in &mut session.rounds {
            for comment in &mut round.comments {
                if comment.id == comment_id {
                    comment.status = CommentStatus::Resolved;
                    comment.resolved_at = Some(ts);
                    self.quality.resolved += 1;
                    return Ok(());
                }
            }
        }
        Err(format!("Comment {} not found", comment_id))
    }

    pub fn wontfix(&mut self, session_id: &str, comment_id: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        for round in &mut session.rounds {
            for comment in &mut round.comments {
                if comment.id == comment_id {
                    comment.status = CommentStatus::WontFix;
                    return Ok(());
                }
            }
        }
        Err(format!("Comment {} not found", comment_id))
    }

    pub fn next_round(&mut self, session_id: &str) -> Result<usize, String> {
        let ts = self.tick();
        let max = self.config.max_rounds;
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        if session.status != "active" {
            return Err("Session is not active".to_string());
        }
        if session.current_round >= max {
            return Err(format!("Maximum {} rounds reached", max));
        }
        // Complete current round
        let cur_idx = session.current_round - 1;
        session.rounds[cur_idx].completed_at = Some(ts);

        let next_num = session.current_round + 1;
        let next_type = if session.rounds[cur_idx].round_type == ReviewRound::AgentReviewsHuman {
            ReviewRound::HumanReviewsAgent
        } else {
            ReviewRound::AgentReviewsHuman
        };
        session.rounds.push(ReviewRoundData {
            round_number: next_num,
            round_type: next_type,
            comments: Vec::new(),
            decision: None,
            started_at: ts,
            completed_at: None,
        });
        session.current_round = next_num;
        Ok(next_num)
    }

    pub fn decide(
        &mut self,
        session_id: &str,
        decision: ReviewDecision,
    ) -> Result<(), String> {
        let ts = self.tick();
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        if session.status != "active" {
            return Err("Session is not active".to_string());
        }
        let cur_idx = session.current_round - 1;
        session.rounds[cur_idx].decision = Some(decision.clone());
        session.rounds[cur_idx].completed_at = Some(ts);
        if decision == ReviewDecision::Approve {
            session.status = "approved".to_string();
        }
        Ok(())
    }

    pub fn get_session(&self, id: &str) -> Option<&ReviewSession> {
        self.sessions.get(id)
    }

    pub fn list_sessions(&self) -> Vec<&ReviewSession> {
        self.sessions.values().collect()
    }

    pub fn open_comments(&self, session_id: &str) -> Result<Vec<&ReviewComment>, String> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        let mut result = Vec::new();
        for round in &session.rounds {
            for comment in &round.comments {
                if comment.status == CommentStatus::Open {
                    result.push(comment);
                }
            }
        }
        Ok(result)
    }

    pub fn get_quality(&self) -> &ReviewQuality {
        &self.quality
    }

    pub fn record_real_issue(&mut self, session_id: &str, comment_id: &str) {
        if let Some(session) = self.sessions.get(session_id) {
            for round in &session.rounds {
                for comment in &round.comments {
                    if comment.id == comment_id && comment.author_role == ReviewRole::Agent {
                        self.quality.agent_caught_real_issues += 1;
                        self.quality.recalculate_precision();
                        return;
                    }
                }
            }
        }
    }

    pub fn record_false_positive(&mut self, session_id: &str, comment_id: &str) {
        if let Some(session) = self.sessions.get(session_id) {
            for round in &session.rounds {
                for comment in &round.comments {
                    if comment.id == comment_id && comment.author_role == ReviewRole::Agent {
                        self.quality.false_positives += 1;
                        self.quality.recalculate_precision();
                        return;
                    }
                }
            }
        }
    }

    pub fn add_checklist(&mut self, session_id: &str, checklist: ReviewChecklist) {
        self.checklists.insert(session_id.to_string(), checklist);
    }

    pub fn get_checklist(&self, session_id: &str) -> Option<&ReviewChecklist> {
        self.checklists.get(session_id)
    }

    pub fn toggle_checklist_item(
        &mut self,
        session_id: &str,
        index: usize,
    ) -> Result<bool, String> {
        let checklist = self
            .checklists
            .get_mut(session_id)
            .ok_or_else(|| "Checklist not found".to_string())?;
        if index >= checklist.items.len() {
            return Err("Index out of range".to_string());
        }
        checklist.items[index].checked = !checklist.items[index].checked;
        Ok(checklist.items[index].checked)
    }

    pub fn close_session(&mut self, session_id: &str) -> Result<(), String> {
        let ts = self.tick();
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        session.status = "closed".to_string();
        let cur_idx = session.current_round - 1;
        if session.rounds[cur_idx].completed_at.is_none() {
            session.rounds[cur_idx].completed_at = Some(ts);
        }
        Ok(())
    }
}

// ── Helper to build a ReviewComment quickly ────────────────────────────

fn make_comment(
    id: &str,
    role: ReviewRole,
    file: &str,
    line_start: usize,
    line_end: usize,
    content: &str,
    severity: &str,
) -> ReviewComment {
    ReviewComment {
        id: id.to_string(),
        author_role: role,
        file_path: file.to_string(),
        line_start,
        line_end,
        content: content.to_string(),
        severity: severity.to_string(),
        status: CommentStatus::Open,
        created_at: 0,
        resolved_at: None,
        replies: Vec::new(),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> ReviewEngine {
        ReviewEngine::new(ReviewConfig::default())
    }

    fn sample_files() -> Vec<String> {
        vec!["src/main.rs".to_string(), "src/lib.rs".to_string()]
    }

    // 1
    #[test]
    fn test_default_config() {
        let c = ReviewConfig::default();
        assert_eq!(c.max_rounds, 5);
        assert!(c.checklist_enabled);
        assert!(c.agent_pushback);
    }

    // 2
    #[test]
    fn test_start_session() {
        let mut e = engine();
        let id = e.start_session("PR 42", sample_files());
        assert!(id.starts_with("session-"));
        let s = e.get_session(&id).unwrap();
        assert_eq!(s.title, "PR 42");
        assert_eq!(s.files.len(), 2);
        assert_eq!(s.current_round, 1);
        assert_eq!(s.status, "active");
    }

    // 3
    #[test]
    fn test_first_round_is_agent_reviews_human() {
        let mut e = engine();
        let id = e.start_session("t", sample_files());
        let s = e.get_session(&id).unwrap();
        assert_eq!(s.rounds[0].round_type, ReviewRound::AgentReviewsHuman);
    }

    // 4
    #[test]
    fn test_add_comment() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        let c = make_comment("c1", ReviewRole::Agent, "src/main.rs", 10, 15, "Potential null deref", "warning");
        assert!(e.add_comment(&sid, c).is_ok());
    }

    // 5
    #[test]
    fn test_add_comment_missing_session() {
        let mut e = engine();
        let c = make_comment("c1", ReviewRole::Agent, "f", 1, 1, "x", "info");
        assert!(e.add_comment("nope", c).is_err());
    }

    // 6
    #[test]
    fn test_comment_increments_total() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        assert_eq!(e.get_quality().total_comments, 0);
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "x", "info")).unwrap();
        assert_eq!(e.get_quality().total_comments, 1);
    }

    // 7
    #[test]
    fn test_reply_to_comment() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 2, "issue", "warning")).unwrap();
        let reply = ReviewReply { author_role: ReviewRole::Human, content: "Good catch".to_string(), created_at: 100 };
        assert!(e.reply(&sid, "c1", reply).is_ok());
    }

    // 8
    #[test]
    fn test_reply_missing_comment() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        let reply = ReviewReply { author_role: ReviewRole::Human, content: "x".to_string(), created_at: 0 };
        assert!(e.reply(&sid, "nope", reply).is_err());
    }

    // 9
    #[test]
    fn test_resolve_comment() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "x", "info")).unwrap();
        assert!(e.resolve(&sid, "c1").is_ok());
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.rounds[0].comments[0].status, CommentStatus::Resolved);
        assert!(s.rounds[0].comments[0].resolved_at.is_some());
    }

    // 10
    #[test]
    fn test_resolve_increments_quality() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "x", "info")).unwrap();
        e.resolve(&sid, "c1").unwrap();
        assert_eq!(e.get_quality().resolved, 1);
    }

    // 11
    #[test]
    fn test_wontfix() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Human, "f", 1, 1, "style nit", "info")).unwrap();
        e.wontfix(&sid, "c1").unwrap();
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.rounds[0].comments[0].status, CommentStatus::WontFix);
    }

    // 12
    #[test]
    fn test_wontfix_missing() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        assert!(e.wontfix(&sid, "nope").is_err());
    }

    // 13
    #[test]
    fn test_next_round_alternates() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        let r2 = e.next_round(&sid).unwrap();
        assert_eq!(r2, 2);
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.rounds[1].round_type, ReviewRound::HumanReviewsAgent);
    }

    // 14
    #[test]
    fn test_next_round_back_to_agent() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.next_round(&sid).unwrap();
        let r3 = e.next_round(&sid).unwrap();
        assert_eq!(r3, 3);
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.rounds[2].round_type, ReviewRound::AgentReviewsHuman);
    }

    // 15
    #[test]
    fn test_max_rounds_enforced() {
        let mut e = ReviewEngine::new(ReviewConfig { max_rounds: 2, ..Default::default() });
        let sid = e.start_session("t", sample_files());
        e.next_round(&sid).unwrap();
        assert!(e.next_round(&sid).is_err());
    }

    // 16
    #[test]
    fn test_decide_approve_closes_session() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.decide(&sid, ReviewDecision::Approve).unwrap();
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.status, "approved");
    }

    // 17
    #[test]
    fn test_decide_request_changes() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.decide(&sid, ReviewDecision::RequestChanges).unwrap();
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.status, "active");
        assert_eq!(s.rounds[0].decision, Some(ReviewDecision::RequestChanges));
    }

    // 18
    #[test]
    fn test_decide_comment_only() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.decide(&sid, ReviewDecision::Comment).unwrap();
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.rounds[0].decision, Some(ReviewDecision::Comment));
    }

    // 19
    #[test]
    fn test_open_comments_filters() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "a", "warning")).unwrap();
        e.add_comment(&sid, make_comment("c2", ReviewRole::Agent, "f", 5, 5, "b", "error")).unwrap();
        e.resolve(&sid, "c1").unwrap();
        let open = e.open_comments(&sid).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id, "c2");
    }

    // 20
    #[test]
    fn test_open_comments_missing_session() {
        let e = engine();
        assert!(e.open_comments("nope").is_err());
    }

    // 21
    #[test]
    fn test_list_sessions() {
        let mut e = engine();
        e.start_session("a", sample_files());
        e.start_session("b", sample_files());
        assert_eq!(e.list_sessions().len(), 2);
    }

    // 22
    #[test]
    fn test_record_real_issue() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "bug", "error")).unwrap();
        e.record_real_issue(&sid, "c1");
        assert_eq!(e.get_quality().agent_caught_real_issues, 1);
    }

    // 23
    #[test]
    fn test_record_false_positive() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "not a bug", "warning")).unwrap();
        e.record_false_positive(&sid, "c1");
        assert_eq!(e.get_quality().false_positives, 1);
    }

    // 24
    #[test]
    fn test_precision_calculation() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "real", "error")).unwrap();
        e.add_comment(&sid, make_comment("c2", ReviewRole::Agent, "f", 2, 2, "false", "warning")).unwrap();
        e.add_comment(&sid, make_comment("c3", ReviewRole::Agent, "f", 3, 3, "real2", "error")).unwrap();
        e.record_real_issue(&sid, "c1");
        e.record_real_issue(&sid, "c3");
        e.record_false_positive(&sid, "c2");
        let q = e.get_quality();
        assert_eq!(q.agent_caught_real_issues, 2);
        assert_eq!(q.false_positives, 1);
        let expected = 2.0 / 3.0;
        assert!((q.precision - expected).abs() < 1e-9);
    }

    // 25
    #[test]
    fn test_precision_all_real() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "x", "error")).unwrap();
        e.record_real_issue(&sid, "c1");
        assert!((e.get_quality().precision - 1.0).abs() < 1e-9);
    }

    // 26
    #[test]
    fn test_precision_default_is_one() {
        let e = engine();
        assert!((e.get_quality().precision - 1.0).abs() < 1e-9);
    }

    // 27
    #[test]
    fn test_human_comment_ignored_by_record_real_issue() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Human, "f", 1, 1, "x", "info")).unwrap();
        e.record_real_issue(&sid, "c1");
        assert_eq!(e.get_quality().agent_caught_real_issues, 0);
    }

    // 28
    #[test]
    fn test_human_comment_ignored_by_record_false_positive() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Human, "f", 1, 1, "x", "info")).unwrap();
        e.record_false_positive(&sid, "c1");
        assert_eq!(e.get_quality().false_positives, 0);
    }

    // 29
    #[test]
    fn test_add_checklist() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        let cl = ReviewChecklist {
            items: vec![
                ChecklistItem { label: "Tests pass".to_string(), checked: false, category: "ci".to_string() },
                ChecklistItem { label: "No warnings".to_string(), checked: false, category: "ci".to_string() },
            ],
        };
        e.add_checklist(&sid, cl);
        let got = e.get_checklist(&sid).unwrap();
        assert_eq!(got.items.len(), 2);
    }

    // 30
    #[test]
    fn test_toggle_checklist_item() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        let cl = ReviewChecklist {
            items: vec![ChecklistItem { label: "Lint".to_string(), checked: false, category: "ci".to_string() }],
        };
        e.add_checklist(&sid, cl);
        let now = e.toggle_checklist_item(&sid, 0).unwrap();
        assert!(now);
        let again = e.toggle_checklist_item(&sid, 0).unwrap();
        assert!(!again);
    }

    // 31
    #[test]
    fn test_toggle_checklist_out_of_range() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        let cl = ReviewChecklist { items: vec![] };
        e.add_checklist(&sid, cl);
        assert!(e.toggle_checklist_item(&sid, 0).is_err());
    }

    // 32
    #[test]
    fn test_toggle_checklist_no_checklist() {
        let mut e = engine();
        assert!(e.toggle_checklist_item("nope", 0).is_err());
    }

    // 33
    #[test]
    fn test_close_session() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.close_session(&sid).unwrap();
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.status, "closed");
    }

    // 34
    #[test]
    fn test_close_session_sets_completed_at() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.close_session(&sid).unwrap();
        let s = e.get_session(&sid).unwrap();
        assert!(s.rounds[0].completed_at.is_some());
    }

    // 35
    #[test]
    fn test_add_comment_to_closed_session_fails() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.close_session(&sid).unwrap();
        let c = make_comment("c1", ReviewRole::Agent, "f", 1, 1, "x", "info");
        assert!(e.add_comment(&sid, c).is_err());
    }

    // 36
    #[test]
    fn test_next_round_on_closed_session_fails() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.close_session(&sid).unwrap();
        assert!(e.next_round(&sid).is_err());
    }

    // 37
    #[test]
    fn test_decide_on_closed_session_fails() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.close_session(&sid).unwrap();
        assert!(e.decide(&sid, ReviewDecision::Approve).is_err());
    }

    // 38
    #[test]
    fn test_multiple_comments_same_round() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        for i in 0..10 {
            let cid = format!("c{}", i);
            e.add_comment(&sid, make_comment(&cid, ReviewRole::Agent, "f", i, i + 1, "issue", "warning")).unwrap();
        }
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.rounds[0].comments.len(), 10);
    }

    // 39
    #[test]
    fn test_comments_across_rounds() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "r1", "info")).unwrap();
        e.next_round(&sid).unwrap();
        e.add_comment(&sid, make_comment("c2", ReviewRole::Human, "f", 2, 2, "r2", "info")).unwrap();
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.rounds[0].comments.len(), 1);
        assert_eq!(s.rounds[1].comments.len(), 1);
    }

    // 40
    #[test]
    fn test_open_comments_across_rounds() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "a", "warning")).unwrap();
        e.next_round(&sid).unwrap();
        e.add_comment(&sid, make_comment("c2", ReviewRole::Human, "f", 2, 2, "b", "error")).unwrap();
        e.resolve(&sid, "c1").unwrap();
        let open = e.open_comments(&sid).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id, "c2");
    }

    // 41
    #[test]
    fn test_reply_preserves_content() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "f", 1, 1, "issue", "warning")).unwrap();
        e.reply(&sid, "c1", ReviewReply {
            author_role: ReviewRole::Human,
            content: "I disagree".to_string(),
            created_at: 200,
        }).unwrap();
        e.reply(&sid, "c1", ReviewReply {
            author_role: ReviewRole::Agent,
            content: "Let me explain".to_string(),
            created_at: 300,
        }).unwrap();
        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.rounds[0].comments[0].replies.len(), 2);
        assert_eq!(s.rounds[0].comments[0].replies[0].content, "I disagree");
        assert_eq!(s.rounds[0].comments[0].replies[1].author_role, ReviewRole::Agent);
    }

    // 42
    #[test]
    fn test_review_role_equality() {
        assert_eq!(ReviewRole::Human, ReviewRole::Human);
        assert_eq!(ReviewRole::Agent, ReviewRole::Agent);
        assert_ne!(ReviewRole::Human, ReviewRole::Agent);
    }

    // 43
    #[test]
    fn test_comment_status_variants() {
        let statuses = [CommentStatus::Open, CommentStatus::Resolved, CommentStatus::WontFix, CommentStatus::Acknowledged];
        assert_eq!(statuses.len(), 4);
        assert_ne!(CommentStatus::Open, CommentStatus::Resolved);
    }

    // 44
    #[test]
    fn test_review_decision_variants() {
        assert_ne!(ReviewDecision::Approve, ReviewDecision::RequestChanges);
        assert_ne!(ReviewDecision::RequestChanges, ReviewDecision::Comment);
    }

    // 45
    #[test]
    fn test_session_unique_ids() {
        let mut e = engine();
        let id1 = e.start_session("a", sample_files());
        let id2 = e.start_session("b", sample_files());
        let id3 = e.start_session("c", sample_files());
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
    }

    // 46
    #[test]
    fn test_full_review_flow() {
        let mut e = engine();
        let sid = e.start_session("Full flow PR", vec!["app.rs".to_string()]);

        // Round 1: Agent reviews human code
        e.add_comment(&sid, make_comment("c1", ReviewRole::Agent, "app.rs", 42, 45, "Missing error handling", "error")).unwrap();
        e.add_comment(&sid, make_comment("c2", ReviewRole::Agent, "app.rs", 100, 100, "Style: prefer if-let", "info")).unwrap();
        e.reply(&sid, "c1", ReviewReply { author_role: ReviewRole::Human, content: "Fixed".to_string(), created_at: 1 }).unwrap();
        e.resolve(&sid, "c1").unwrap();
        e.wontfix(&sid, "c2").unwrap();
        e.decide(&sid, ReviewDecision::RequestChanges).unwrap();

        // Round 2: Human reviews agent code
        e.next_round(&sid).unwrap();
        e.add_comment(&sid, make_comment("c3", ReviewRole::Human, "app.rs", 50, 55, "This refactor is wrong", "error")).unwrap();
        e.reply(&sid, "c3", ReviewReply { author_role: ReviewRole::Agent, content: "Reverted".to_string(), created_at: 2 }).unwrap();
        e.resolve(&sid, "c3").unwrap();

        // Round 3: Agent approves
        e.next_round(&sid).unwrap();
        e.decide(&sid, ReviewDecision::Approve).unwrap();

        // Quality tracking
        e.record_real_issue(&sid, "c1");
        e.record_false_positive(&sid, "c2");

        let s = e.get_session(&sid).unwrap();
        assert_eq!(s.status, "approved");
        assert_eq!(s.rounds.len(), 3);
        assert_eq!(e.get_quality().resolved, 2);
        assert_eq!(e.get_quality().total_comments, 3);
        assert_eq!(e.get_quality().agent_caught_real_issues, 1);
        assert_eq!(e.get_quality().false_positives, 1);
        assert!((e.get_quality().precision - 0.5).abs() < 1e-9);
    }

    // 47
    #[test]
    fn test_next_round_completes_previous() {
        let mut e = engine();
        let sid = e.start_session("t", sample_files());
        e.next_round(&sid).unwrap();
        let s = e.get_session(&sid).unwrap();
        assert!(s.rounds[0].completed_at.is_some());
    }

    // 48
    #[test]
    fn test_get_session_none_for_missing() {
        let e = engine();
        assert!(e.get_session("xyz").is_none());
    }

    // 49
    #[test]
    fn test_get_checklist_none_for_missing() {
        let e = engine();
        assert!(e.get_checklist("nope").is_none());
    }

    // 50
    #[test]
    fn test_custom_config() {
        let cfg = ReviewConfig { max_rounds: 10, checklist_enabled: false, agent_pushback: false };
        let e = ReviewEngine::new(cfg);
        assert_eq!(e.config.max_rounds, 10);
        assert!(!e.config.checklist_enabled);
        assert!(!e.config.agent_pushback);
    }
}
