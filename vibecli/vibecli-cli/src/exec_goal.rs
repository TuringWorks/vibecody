//! Execution goals — durable, cross-session intent.
//!
//! A `Goal` is the forward-looking sibling of [`crate::recap::Recap`]:
//! recap summarizes past work, goal captures the *direction* the user
//! wants the agent to move in. Each goal is a persistent record of
//! intent (title + statement + success criteria + status) that can
//! decompose into a [`vibe_ai::planner::ExecutionPlan`] on demand and
//! gather a link graph of sessions, jobs, and recaps that contributed.
//!
//! Module layout mirrors [`crate::recap`]: this file holds the
//! cross-cutting type shapes plus pure helpers. CRUD and HTTP wiring
//! live in `session_store.rs` and `serve.rs` respectively.
//!
//! Goals live in the unencrypted `~/.vibecli/sessions.db` next to
//! sessions and recaps. Workspace scope is nullable — `None` means
//! "global" (visible from any project / mobile / watch); a `Some(path)`
//! goal is workspace-bound.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vibe_ai::planner::ExecutionPlan;

// ── Types ────────────────────────────────────────────────────────────────────

/// Stable wire shape for an execution goal. Mirrors the SQL row plus
/// the embedded `ExecutionPlan` JSON. `PartialEq` is intentionally not
/// derived because the embedded `vibe_ai::planner::ExecutionPlan`
/// doesn't implement it; compare specific fields instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    /// Workspace scope. `None` = global (visible everywhere, including
    /// mobile/watch). `Some(path)` = bound to a project.
    pub workspace: Option<PathBuf>,
    pub title: String,
    pub statement: String,
    pub status: GoalStatus,
    pub success_criteria: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Optional parent for hierarchical goals. Schema column reserved
    /// from day one; tree-query routes are deferred past v1.
    pub parent_goal_id: Option<String>,
    /// Latest `ExecutionPlan` for this goal, if one has been
    /// generated. Cleared on `PATCH` of `statement` or
    /// `success_criteria` — a stale plan is worse than no plan.
    pub current_plan: Option<ExecutionPlan>,
    /// Forward-compat marker. Start at 1; bump on breaking field
    /// changes, not additive ones.
    pub schema_version: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Active,
    Paused,
    Done,
    Abandoned,
}

impl GoalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            GoalStatus::Active => "active",
            GoalStatus::Paused => "paused",
            GoalStatus::Done => "done",
            GoalStatus::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(GoalStatus::Active),
            "paused" => Some(GoalStatus::Paused),
            "done" => Some(GoalStatus::Done),
            "abandoned" => Some(GoalStatus::Abandoned),
            _ => None,
        }
    }
}

/// A typed reference from a goal to a session, job, recap, or freeform
/// note. The join-table model keeps `sessions` untouched and lets a
/// session contribute to multiple goals over its lifetime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoalLink {
    pub id: String,
    pub goal_id: String,
    pub kind: GoalLinkKind,
    pub target_id: String,
    pub linked_at: DateTime<Utc>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalLinkKind {
    Session,
    Job,
    Recap,
    Note,
}

impl GoalLinkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GoalLinkKind::Session => "session",
            GoalLinkKind::Job => "job",
            GoalLinkKind::Recap => "recap",
            GoalLinkKind::Note => "note",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "session" => Some(GoalLinkKind::Session),
            "job" => Some(GoalLinkKind::Job),
            "recap" => Some(GoalLinkKind::Recap),
            "note" => Some(GoalLinkKind::Note),
            _ => None,
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Maximum title length. Pinned by validation so UIs can size headers
/// without truncation surprises.
pub const TITLE_MAX_LEN: usize = 120;

/// Validation error surfaced by `Goal::validate` and similar.
#[derive(Debug, thiserror::Error)]
pub enum GoalValidationError {
    #[error("title must not be empty")]
    TitleEmpty,
    #[error("title exceeds {TITLE_MAX_LEN} chars (was {0})")]
    TitleTooLong(usize),
}

impl Goal {
    /// Validate the user-facing fields. Called at the HTTP boundary;
    /// the DB layer trusts validated input.
    pub fn validate(&self) -> Result<(), GoalValidationError> {
        let trimmed = self.title.trim();
        if trimmed.is_empty() {
            return Err(GoalValidationError::TitleEmpty);
        }
        if trimmed.chars().count() > TITLE_MAX_LEN {
            return Err(GoalValidationError::TitleTooLong(trimmed.chars().count()));
        }
        Ok(())
    }

    /// Fresh goal with sensible defaults. Caller sets title/statement
    /// before insert. Used by the daemon `POST /v1/goals` handler.
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: new_goal_id(),
            workspace: None,
            title: title.into(),
            statement: String::new(),
            status: GoalStatus::Active,
            success_criteria: Vec::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            parent_goal_id: None,
            current_plan: None,
            schema_version: 1,
        }
    }

    /// Whether changing `statement` or `success_criteria` from the
    /// `prior` value should invalidate any cached `current_plan`. The
    /// rule: a plan written against an older statement is misleading.
    /// Returns `true` when the new goal's plan must be cleared.
    pub fn plan_should_invalidate(&self, prior: &Goal) -> bool {
        self.statement != prior.statement || self.success_criteria != prior.success_criteria
    }
}

/// Fresh goal id. UUIDv4 hex (no dashes) — same convention as
/// `recap::new_recap_id`. The sort order comes from `created_at`, not
/// the id, so this is safe even though it isn't a true ULID.
pub fn new_goal_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Fresh goal-link id. Same shape as `new_goal_id`.
pub fn new_goal_link_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_status_round_trips_via_str() {
        for status in [
            GoalStatus::Active,
            GoalStatus::Paused,
            GoalStatus::Done,
            GoalStatus::Abandoned,
        ] {
            assert_eq!(
                GoalStatus::from_str(status.as_str()),
                Some(status),
                "round-trip failed for {status:?}"
            );
        }
        assert_eq!(GoalStatus::from_str("nope"), None);
    }

    #[test]
    fn goal_link_kind_round_trips_via_str() {
        for kind in [
            GoalLinkKind::Session,
            GoalLinkKind::Job,
            GoalLinkKind::Recap,
            GoalLinkKind::Note,
        ] {
            assert_eq!(GoalLinkKind::from_str(kind.as_str()), Some(kind),);
        }
    }

    #[test]
    fn goal_status_serializes_snake_case() {
        let json = serde_json::to_string(&GoalStatus::Abandoned).unwrap();
        assert_eq!(json, "\"abandoned\"");
        let back: GoalStatus = serde_json::from_str("\"paused\"").unwrap();
        assert_eq!(back, GoalStatus::Paused);
    }

    #[test]
    fn validate_rejects_empty_and_oversize_titles() {
        let mut g = Goal::new("");
        assert!(matches!(g.validate(), Err(GoalValidationError::TitleEmpty)));
        g.title = "   ".to_string();
        assert!(matches!(g.validate(), Err(GoalValidationError::TitleEmpty)));
        g.title = "x".repeat(TITLE_MAX_LEN + 1);
        assert!(matches!(
            g.validate(),
            Err(GoalValidationError::TitleTooLong(_))
        ));
        g.title = "ship the /goal feature".to_string();
        assert!(g.validate().is_ok());
    }

    #[test]
    fn plan_invalidates_on_statement_or_criteria_change() {
        let mut a = Goal::new("ship /goal");
        a.statement = "land G1.1 through G1.7".into();
        a.success_criteria = vec!["all slices merged".into()];

        // Identical → no invalidation.
        let same = a.clone();
        assert!(!same.plan_should_invalidate(&a));

        // Statement edit → invalidates.
        let mut b = a.clone();
        b.statement = "land G1.1 first".into();
        assert!(b.plan_should_invalidate(&a));

        // Criteria edit → invalidates.
        let mut c = a.clone();
        c.success_criteria.push("docs updated".into());
        assert!(c.plan_should_invalidate(&a));

        // Title or status edit alone → does NOT invalidate plan.
        let mut d = a.clone();
        d.title = "ship /goal feature".into();
        d.status = GoalStatus::Paused;
        assert!(!d.plan_should_invalidate(&a));
    }

    #[test]
    fn fresh_goal_has_active_status_and_unique_ids() {
        let g1 = Goal::new("a");
        let g2 = Goal::new("b");
        assert_eq!(g1.status, GoalStatus::Active);
        assert_eq!(g1.schema_version, 1);
        assert!(g1.created_at <= g1.updated_at);
        assert_ne!(g1.id, g2.id);
    }
}
