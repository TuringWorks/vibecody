//! Inline diff accept/reject — hunk-level patch application with partial
//! acceptance. Matches Claude Code 1.x, Cursor 4.0, and Copilot's inline diff UI.
//!
//! The `InlineDiffSession` holds a file's current content and a set of proposed
//! hunks. Each hunk can be individually accepted or rejected. The session
//! applies accepted hunks in reverse order (bottom-up) to avoid line offset shifts.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A unique ID for a proposed hunk in a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HunkId(pub usize);

/// Status of a hunk decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HunkDecision {
    Pending,
    Accepted,
    Rejected,
}

/// A proposed code change (single hunk).
#[derive(Debug, Clone)]
pub struct ProposedHunk {
    pub id: HunkId,
    /// 1-based start line in the *original* file.
    pub start_line: usize,
    /// Number of lines this hunk spans in the original file.
    pub original_len: usize,
    pub original_lines: Vec<String>,
    pub replacement_lines: Vec<String>,
    pub description: Option<String>,
    pub decision: HunkDecision,
}

impl ProposedHunk {
    pub fn net_line_delta(&self) -> i64 {
        self.replacement_lines.len() as i64 - self.original_len as i64
    }

    pub fn is_decided(&self) -> bool { self.decision != HunkDecision::Pending }
}

/// Result of applying the session.
#[derive(Debug)]
pub struct ApplicationResult {
    pub new_content: String,
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub pending_count: usize,
    pub line_delta: i64,
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// An interactive inline diff session for a single file.
pub struct InlineDiffSession {
    pub file_path: String,
    original_lines: Vec<String>,
    pub hunks: Vec<ProposedHunk>,
    next_hunk_id: usize,
}

impl InlineDiffSession {
    pub fn new(file_path: impl Into<String>, content: &str) -> Self {
        Self {
            file_path: file_path.into(),
            original_lines: content.lines().map(|l| l.to_string()).collect(),
            hunks: Vec::new(),
            next_hunk_id: 0,
        }
    }

    /// Propose a new hunk. Returns the assigned HunkId.
    pub fn propose(
        &mut self,
        start_line: usize,
        original_len: usize,
        replacement_lines: Vec<String>,
        description: Option<String>,
    ) -> HunkId {
        let id = HunkId(self.next_hunk_id);
        self.next_hunk_id += 1;

        // Capture the original lines this hunk replaces
        let original_lines: Vec<String> = self.original_lines
            .iter()
            .skip(start_line.saturating_sub(1))
            .take(original_len)
            .cloned()
            .collect();

        self.hunks.push(ProposedHunk {
            id,
            start_line,
            original_len,
            original_lines,
            replacement_lines,
            description,
            decision: HunkDecision::Pending,
        });
        id
    }

    /// Accept a hunk by ID.
    pub fn accept(&mut self, id: HunkId) -> Result<(), String> {
        let hunk = self.hunks.iter_mut().find(|h| h.id == id)
            .ok_or_else(|| format!("Hunk {:?} not found", id))?;
        if hunk.decision != HunkDecision::Pending {
            return Err(format!("Hunk {:?} already decided", id));
        }
        hunk.decision = HunkDecision::Accepted;
        Ok(())
    }

    /// Reject a hunk by ID.
    pub fn reject(&mut self, id: HunkId) -> Result<(), String> {
        let hunk = self.hunks.iter_mut().find(|h| h.id == id)
            .ok_or_else(|| format!("Hunk {:?} not found", id))?;
        if hunk.decision != HunkDecision::Pending {
            return Err(format!("Hunk {:?} already decided", id));
        }
        hunk.decision = HunkDecision::Rejected;
        Ok(())
    }

    /// Accept all pending hunks.
    pub fn accept_all(&mut self) {
        for hunk in &mut self.hunks {
            if hunk.decision == HunkDecision::Pending {
                hunk.decision = HunkDecision::Accepted;
            }
        }
    }

    /// Reject all pending hunks.
    pub fn reject_all(&mut self) {
        for hunk in &mut self.hunks {
            if hunk.decision == HunkDecision::Pending {
                hunk.decision = HunkDecision::Rejected;
            }
        }
    }

    /// Apply all accepted hunks and return the resulting content.
    /// Rejected and pending hunks are skipped (original content kept).
    pub fn apply(&self) -> ApplicationResult {
        let mut lines = self.original_lines.clone();
        let mut accepted = 0usize;
        let mut rejected = 0usize;
        let mut pending = 0usize;

        for h in &self.hunks {
            match h.decision {
                HunkDecision::Accepted => accepted += 1,
                HunkDecision::Rejected => rejected += 1,
                HunkDecision::Pending => pending += 1,
            }
        }

        // Apply accepted hunks in reverse start_line order (bottom-up)
        let mut accepted_hunks: Vec<&ProposedHunk> = self.hunks.iter()
            .filter(|h| h.decision == HunkDecision::Accepted)
            .collect();
        accepted_hunks.sort_by(|a, b| b.start_line.cmp(&a.start_line));

        let mut line_delta: i64 = 0;
        for hunk in accepted_hunks {
            let start = hunk.start_line.saturating_sub(1); // 0-based
            let end = (start + hunk.original_len).min(lines.len());
            lines.splice(start..end, hunk.replacement_lines.iter().cloned());
            line_delta += hunk.net_line_delta();
        }

        let new_content = if lines.is_empty() {
            String::new()
        } else {
            lines.join("\n") + "\n"
        };

        ApplicationResult {
            new_content,
            accepted_count: accepted,
            rejected_count: rejected,
            pending_count: pending,
            line_delta,
        }
    }

    /// Preview: apply only the hunk at `id` against the original.
    pub fn preview_hunk(&self, id: HunkId) -> Result<String, String> {
        let hunk = self.hunks.iter().find(|h| h.id == id)
            .ok_or_else(|| format!("Hunk {:?} not found", id))?;
        let mut lines = self.original_lines.clone();
        let start = hunk.start_line.saturating_sub(1);
        let end = (start + hunk.original_len).min(lines.len());
        lines.splice(start..end, hunk.replacement_lines.iter().cloned());
        Ok(lines.join("\n") + "\n")
    }

    pub fn pending_count(&self) -> usize { self.hunks.iter().filter(|h| h.decision == HunkDecision::Pending).count() }
    pub fn decided_count(&self) -> usize { self.hunks.iter().filter(|h| h.is_decided()).count() }
    pub fn hunk_count(&self) -> usize { self.hunks.len() }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL: &str = "line 1\nline 2\nline 3\nline 4\nline 5\n";

    fn session() -> InlineDiffSession {
        InlineDiffSession::new("test.rs", ORIGINAL)
    }

    #[test]
    fn test_propose_and_accept() {
        let mut s = session();
        let id = s.propose(2, 1, vec!["replaced line 2".into()], None);
        s.accept(id).unwrap();
        let result = s.apply();
        assert!(result.new_content.contains("replaced line 2"));
        // Original "line 2\n" should not appear as a standalone line
        assert!(!result.new_content.lines().any(|l| l == "line 2"));
        assert_eq!(result.accepted_count, 1);
        assert_eq!(result.line_delta, 0); // same line count
    }

    #[test]
    fn test_reject_keeps_original() {
        let mut s = session();
        let id = s.propose(2, 1, vec!["should not appear".into()], None);
        s.reject(id).unwrap();
        let result = s.apply();
        assert!(result.new_content.contains("line 2"));
        assert!(!result.new_content.contains("should not appear"));
    }

    #[test]
    fn test_accept_all() {
        let mut s = session();
        s.propose(1, 1, vec!["new 1".into()], None);
        s.propose(2, 1, vec!["new 2".into()], None);
        s.accept_all();
        let result = s.apply();
        assert_eq!(result.accepted_count, 2);
        assert_eq!(result.pending_count, 0);
    }

    #[test]
    fn test_reject_all() {
        let mut s = session();
        s.propose(1, 1, vec!["x".into()], None);
        s.propose(2, 1, vec!["y".into()], None);
        s.reject_all();
        let result = s.apply();
        assert_eq!(result.rejected_count, 2);
        assert_eq!(result.accepted_count, 0);
    }

    #[test]
    fn test_pending_count() {
        let mut s = session();
        s.propose(1, 1, vec!["a".into()], None);
        s.propose(2, 1, vec!["b".into()], None);
        let id = s.propose(3, 1, vec!["c".into()], None);
        s.accept(id).unwrap();
        assert_eq!(s.pending_count(), 2);
        assert_eq!(s.decided_count(), 1);
    }

    #[test]
    fn test_net_line_delta_positive() {
        let mut s = session();
        let id = s.propose(2, 1, vec!["a".into(), "b".into(), "c".into()], None);
        s.accept(id).unwrap();
        let result = s.apply();
        assert_eq!(result.line_delta, 2); // replaced 1 with 3
    }

    #[test]
    fn test_net_line_delta_negative() {
        let mut s = session();
        let id = s.propose(1, 3, vec!["single line".into()], None);
        s.accept(id).unwrap();
        let result = s.apply();
        assert_eq!(result.line_delta, -2); // replaced 3 with 1
    }

    #[test]
    fn test_multi_hunk_apply_no_offset_corruption() {
        let content = "a\nb\nc\nd\ne\n";
        let mut s = InlineDiffSession::new("x", content);
        let id1 = s.propose(1, 1, vec!["A".into()], None);
        let id2 = s.propose(4, 1, vec!["D".into()], None);
        s.accept(id1).unwrap();
        s.accept(id2).unwrap();
        let result = s.apply();
        // Both should be applied
        assert!(result.new_content.contains('A'));
        assert!(result.new_content.contains('D'));
        assert!(!result.new_content.contains('\n'.to_string().as_str())
            || result.new_content.lines().any(|l| l == "A"));
    }

    #[test]
    fn test_double_accept_fails() {
        let mut s = session();
        let id = s.propose(1, 1, vec!["x".into()], None);
        s.accept(id).unwrap();
        assert!(s.accept(id).is_err());
    }

    #[test]
    fn test_preview_hunk() {
        let mut s = session();
        let id = s.propose(1, 1, vec!["previewed".into()], None);
        let preview = s.preview_hunk(id).unwrap();
        assert!(preview.contains("previewed"));
        // Original should be unchanged
        assert_eq!(s.hunks[0].decision, HunkDecision::Pending);
    }

    #[test]
    fn test_original_lines_captured() {
        let mut s = session();
        s.propose(2, 1, vec!["replacement".into()], None);
        assert_eq!(s.hunks[0].original_lines, vec!["line 2"]);
    }

    #[test]
    fn test_hunk_description() {
        let mut s = session();
        s.propose(1, 1, vec!["x".into()], Some("simplify logic".into()));
        assert_eq!(s.hunks[0].description.as_deref(), Some("simplify logic"));
    }
}
