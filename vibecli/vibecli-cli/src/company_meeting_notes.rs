#![allow(dead_code)]
//! Meeting notes ingestion for company orchestration.
//!
//! Parses meeting notes text using simple heuristics to extract:
//! - Action items / tasks
//! - Decisions / approvals
//! - Follow-ups

use serde::{Deserialize, Serialize};

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTask {
    pub title: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedApproval {
    pub subject: String,
    pub decision_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFollowup {
    pub text: String,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeetingIngestResult {
    pub tasks: Vec<ExtractedTask>,
    pub approvals: Vec<ExtractedApproval>,
    pub followups: Vec<ExtractedFollowup>,
}

// ── Prefixes ──────────────────────────────────────────────────────────────────

const TASK_PREFIXES: &[&str] = &[
    "Action:", "Action item:", "TODO:", "AI:",
    "action:", "action item:", "todo:", "ai:",
];

const APPROVAL_PREFIXES: &[&str] = &[
    "Decision:", "Decided:", "decision:", "decided:",
];

const FOLLOWUP_PREFIXES: &[&str] = &[
    "Follow up:", "Follow-up:", "Next step:", "Next steps:",
    "follow up:", "follow-up:", "next step:", "next steps:",
];

// ── Date extraction ───────────────────────────────────────────────────────────

const WEEKDAYS: &[&str] = &[
    "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
    "monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday",
];

/// Try to extract a due date from a line.
/// Looks for patterns like "by YYYY-MM-DD" or "by <weekday>".
fn extract_due_date(text: &str) -> (String, Option<String>) {
    // "by YYYY-MM-DD"
    if let Some(idx) = text.find(" by ") {
        let rest = &text[idx + 4..];
        // Check for ISO date
        if rest.len() >= 10 {
            let candidate = &rest[..10];
            if candidate.len() == 10
                && candidate.chars().nth(4) == Some('-')
                && candidate.chars().nth(7) == Some('-')
                && candidate[..4].chars().all(|c| c.is_ascii_digit())
                && candidate[5..7].chars().all(|c| c.is_ascii_digit())
                && candidate[8..10].chars().all(|c| c.is_ascii_digit())
            {
                let clean = text[..idx].to_string();
                return (clean, Some(candidate.to_string()));
            }
        }
        // Check for weekday
        for &day in WEEKDAYS {
            if rest.starts_with(day) {
                let clean = text[..idx].to_string();
                // Normalize to Title case
                let normalized = format!("{}{}", &day[..1].to_uppercase(), &day[1..].to_lowercase());
                return (clean, Some(normalized));
            }
        }
    }
    (text.to_string(), None)
}

/// Strip a prefix from a line (case-insensitive prefix match).
fn strip_prefix<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    if line.len() >= prefix.len() && line[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(line[prefix.len()..].trim())
    } else {
        None
    }
}

// ── Main ingestion function ───────────────────────────────────────────────────

pub fn ingest_meeting_notes(content: &str) -> MeetingIngestResult {
    let mut result = MeetingIngestResult::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Check task prefixes
        if let Some(body) = TASK_PREFIXES.iter().find_map(|&p| strip_prefix(line, p)) {
            let (title, due_date) = extract_due_date(body);
            result.tasks.push(ExtractedTask {
                title: title.trim().to_string(),
                owner: None,
                due_date,
            });
            continue;
        }

        // Check approval prefixes
        if let Some(body) = APPROVAL_PREFIXES.iter().find_map(|&p| strip_prefix(line, p)) {
            // Try to split "Subject: text" or just use the whole body as decision_text
            let (subject, decision_text) = if let Some(colon_pos) = body.find(':') {
                (body[..colon_pos].trim().to_string(), body[colon_pos + 1..].trim().to_string())
            } else {
                ("Decision".to_string(), body.to_string())
            };
            result.approvals.push(ExtractedApproval { subject, decision_text });
            continue;
        }

        // Check follow-up prefixes
        if let Some(body) = FOLLOWUP_PREFIXES.iter().find_map(|&p| strip_prefix(line, p)) {
            let (text, due_date) = extract_due_date(body);
            result.followups.push(ExtractedFollowup {
                text: text.trim().to_string(),
                due_date,
            });
            continue;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_action_prefix_when_ingested_then_extracted_as_task() {
        let notes = "Action: Write quarterly report by 2024-03-31";
        let result = ingest_meeting_notes(notes);
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].title, "Write quarterly report");
        assert_eq!(result.tasks[0].due_date.as_deref(), Some("2024-03-31"));
    }

    #[test]
    fn given_todo_prefix_when_ingested_then_extracted_as_task() {
        let notes = "TODO: Schedule follow-up meeting";
        let result = ingest_meeting_notes(notes);
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].title, "Schedule follow-up meeting");
    }

    #[test]
    fn given_ai_prefix_when_ingested_then_extracted_as_task() {
        let notes = "AI: Send summary email by Friday";
        let result = ingest_meeting_notes(notes);
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].due_date.as_deref(), Some("Friday"));
    }

    #[test]
    fn given_decision_prefix_when_ingested_then_extracted_as_approval() {
        let notes = "Decision: Move launch to Q2";
        let result = ingest_meeting_notes(notes);
        assert_eq!(result.approvals.len(), 1);
        assert!(result.approvals[0].decision_text.contains("Move launch"));
    }

    #[test]
    fn given_decided_prefix_when_ingested_then_extracted_as_approval() {
        let notes = "Decided: Use Rust for backend";
        let result = ingest_meeting_notes(notes);
        assert_eq!(result.approvals.len(), 1);
    }

    #[test]
    fn given_follow_up_prefix_when_ingested_then_extracted_as_followup() {
        let notes = "Follow up: Check with legal by Wednesday";
        let result = ingest_meeting_notes(notes);
        assert_eq!(result.followups.len(), 1);
        assert_eq!(result.followups[0].text, "Check with legal");
        assert_eq!(result.followups[0].due_date.as_deref(), Some("Wednesday"));
    }

    #[test]
    fn given_next_step_prefix_when_ingested_then_extracted_as_followup() {
        let notes = "Next step: Review PR before merge";
        let result = ingest_meeting_notes(notes);
        assert_eq!(result.followups.len(), 1);
        assert_eq!(result.followups[0].text, "Review PR before merge");
    }

    #[test]
    fn given_mixed_notes_when_ingested_then_all_categories_populated() {
        let notes = "\
            Action: Update docs\n\
            Decision: Approve new logo\n\
            Follow-up: Send invoice by 2024-04-15\n\
            TODO: Fix bug #123\n\
            Decided: Ship feature in v2\n\
        ";
        let result = ingest_meeting_notes(notes);
        assert_eq!(result.tasks.len(), 2);
        assert_eq!(result.approvals.len(), 2);
        assert_eq!(result.followups.len(), 1);
        assert_eq!(result.followups[0].due_date.as_deref(), Some("2024-04-15"));
    }

    #[test]
    fn given_empty_content_when_ingested_then_all_empty() {
        let result = ingest_meeting_notes("");
        assert!(result.tasks.is_empty());
        assert!(result.approvals.is_empty());
        assert!(result.followups.is_empty());
    }

    #[test]
    fn given_irrelevant_lines_when_ingested_then_nothing_extracted() {
        let notes = "Today we discussed the roadmap.\nAll good, no blockers.";
        let result = ingest_meeting_notes(notes);
        assert!(result.tasks.is_empty());
        assert!(result.approvals.is_empty());
        assert!(result.followups.is_empty());
    }
}
