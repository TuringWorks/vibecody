//! `WorkspaceStore`-backed persistence for Security Posture.
//!
//! Reuses the existing encrypted SQLite store at
//! `<workspace>/.vibecli/workspace.db` — no new schema, just new
//! setting-key prefixes:
//!
//! | Key | Value |
//! |---|---|
//! | `posture:finding:<id>` | JSON `SecurityFinding` (last seen)   |
//! | `posture:suppress:<id>` | `{ reason, at_unix_ms }`            |
//! | `posture:goal_link:<id>` | `{ goal_id, at_unix_ms }`          |
//! | `posture:decision_log`  | JSON `Vec<DecisionLogEntry>`, FIFO |
//!
//! The decision log is intentionally kept as a single
//! JSON-vec row (not one row per entry) because the audit-write
//! must be transactional with respect to the read — we want every
//! reader to see a consistent view of the log even if a write is
//! in flight. The cap (`DECISION_LOG_MAX_ROWS`) keeps the row size
//! bounded.
//!
//! See `docs/design/security-posture/README.md` for the full
//! persistence design.

use crate::security_posture::{
    unix_ms_now_for_tests, DecisionLogEntry, DecisionOperation, FindingStatus, SecurityFinding,
    DECISION_LOG_MAX_ROWS,
};
use crate::workspace_store::WorkspaceStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};

const KEY_FINDING_PREFIX: &str = "posture:finding:";
const KEY_SUPPRESS_PREFIX: &str = "posture:suppress:";
const KEY_GOAL_LINK_PREFIX: &str = "posture:goal_link:";
const KEY_DECISION_LOG: &str = "posture:decision_log";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SuppressionRecord {
    reason: String,
    at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoalLinkRecord {
    goal_id: String,
    at_unix_ms: i64,
}

/// Read a single finding by id. Returns `None` if not present.
pub fn get_finding(store: &WorkspaceStore, id: &str) -> Result<Option<SecurityFinding>, String> {
    let key = finding_key(id);
    match store.setting_get(&key)? {
        Some(json) => serde_json::from_str::<SecurityFinding>(&json)
            .map(Some)
            .map_err(|e| format!("parse posture:finding:{id}: {e}")),
        None => Ok(None),
    }
}

/// List every persisted finding for this workspace, with any
/// suppression / goal-link state already merged into the
/// `status` field.
pub fn list_findings(store: &WorkspaceStore) -> Result<Vec<SecurityFinding>, String> {
    let entries = store.setting_list()?;
    let mut out: Vec<SecurityFinding> = Vec::new();
    for entry in entries {
        let key = entry["key"].as_str().unwrap_or("");
        if !key.starts_with(KEY_FINDING_PREFIX) {
            continue;
        }
        let id = &key[KEY_FINDING_PREFIX.len()..];
        if let Some(mut f) = get_finding(store, id)? {
            apply_status_overrides(store, &mut f)?;
            out.push(f);
        }
    }
    Ok(out)
}

/// Upsert a finding into the store. If a record with the same id
/// already exists, `first_seen_unix_ms` is preserved (the old
/// value wins) and `last_seen_unix_ms` is updated to the new
/// finding's timestamp.
pub fn upsert_finding(store: &WorkspaceStore, mut finding: SecurityFinding) -> Result<(), String> {
    if let Some(existing) = get_finding(store, &finding.id)? {
        finding.first_seen_unix_ms = existing.first_seen_unix_ms;
    }
    apply_status_overrides(store, &mut finding)?;
    let json = serde_json::to_string(&finding).map_err(|e| e.to_string())?;
    store.setting_set(&finding_key(&finding.id), &json)
}

/// Bulk upsert. Used by the aggregator after a full scan completes.
pub fn upsert_many(store: &WorkspaceStore, findings: Vec<SecurityFinding>) -> Result<(), String> {
    for f in findings {
        upsert_finding(store, f)?;
    }
    Ok(())
}

/// Suppress a finding with a free-text reason. The reason is
/// retained verbatim in the decision log so a security review can
/// reconstruct why suppression was granted.
pub fn suppress(store: &WorkspaceStore, id: &str, reason: &str) -> Result<(), String> {
    let now = unix_ms_now_for_tests();
    let record = SuppressionRecord {
        reason: reason.to_string(),
        at_unix_ms: now,
    };
    let json = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    store.setting_set(&suppress_key(id), &json)?;
    append_decision_log_entry(
        store,
        DecisionLogEntry {
            at_unix_ms: now,
            finding_id: id.to_string(),
            operation: DecisionOperation::Suppress,
            reason: Some(reason.to_string()),
        },
    )?;
    // Reflect the new status on the stored finding row so list_findings
    // surfaces it without re-merging.
    if let Some(mut f) = get_finding(store, id)? {
        f.status = FindingStatus::Suppressed {
            reason: reason.to_string(),
            at_unix_ms: now,
        };
        let json = serde_json::to_string(&f).map_err(|e| e.to_string())?;
        store.setting_set(&finding_key(id), &json)?;
    }
    Ok(())
}

/// Lift a suppression. The decision log records the unsuppress;
/// the original `Suppress` entry is preserved.
pub fn unsuppress(store: &WorkspaceStore, id: &str) -> Result<(), String> {
    store.setting_delete(&suppress_key(id))?;
    let now = unix_ms_now_for_tests();
    append_decision_log_entry(
        store,
        DecisionLogEntry {
            at_unix_ms: now,
            finding_id: id.to_string(),
            operation: DecisionOperation::Unsuppress,
            reason: None,
        },
    )?;
    if let Some(mut f) = get_finding(store, id)? {
        // Falls back to Open unless a goal link still exists.
        f.status = match read_goal_link(store, id)? {
            Some(g) => FindingStatus::GoalLinked {
                goal_id: g.goal_id,
                at_unix_ms: g.at_unix_ms,
            },
            None => FindingStatus::Open,
        };
        let json = serde_json::to_string(&f).map_err(|e| e.to_string())?;
        store.setting_set(&finding_key(id), &json)?;
    }
    Ok(())
}

/// Record that a finding has been promoted to a Goal. Used by the
/// goal-system bridge — `goal_id` is the id returned by the goal
/// system's create call.
pub fn link_goal(store: &WorkspaceStore, id: &str, goal_id: &str) -> Result<(), String> {
    let now = unix_ms_now_for_tests();
    let record = GoalLinkRecord {
        goal_id: goal_id.to_string(),
        at_unix_ms: now,
    };
    let json = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    store.setting_set(&goal_link_key(id), &json)?;
    append_decision_log_entry(
        store,
        DecisionLogEntry {
            at_unix_ms: now,
            finding_id: id.to_string(),
            operation: DecisionOperation::LinkGoal,
            reason: None,
        },
    )?;
    if let Some(mut f) = get_finding(store, id)? {
        f.status = FindingStatus::GoalLinked {
            goal_id: goal_id.to_string(),
            at_unix_ms: now,
        };
        let json = serde_json::to_string(&f).map_err(|e| e.to_string())?;
        store.setting_set(&finding_key(id), &json)?;
    }
    Ok(())
}

/// Decision log as a Vec, newest-first.
pub fn read_decision_log(store: &WorkspaceStore) -> Result<Vec<DecisionLogEntry>, String> {
    match store.setting_get(KEY_DECISION_LOG)? {
        Some(json) => {
            let mut entries: Vec<DecisionLogEntry> = serde_json::from_str(&json)
                .map_err(|e| format!("parse {KEY_DECISION_LOG}: {e}"))?;
            entries.sort_by(|a, b| b.at_unix_ms.cmp(&a.at_unix_ms));
            Ok(entries)
        }
        None => Ok(Vec::new()),
    }
}

// ── internals ────────────────────────────────────────────────────────

fn finding_key(id: &str) -> String {
    format!("{KEY_FINDING_PREFIX}{id}")
}

fn suppress_key(id: &str) -> String {
    format!("{KEY_SUPPRESS_PREFIX}{id}")
}

fn goal_link_key(id: &str) -> String {
    format!("{KEY_GOAL_LINK_PREFIX}{id}")
}

fn read_suppression(store: &WorkspaceStore, id: &str) -> Result<Option<SuppressionRecord>, String> {
    match store.setting_get(&suppress_key(id))? {
        Some(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| format!("parse posture:suppress:{id}: {e}")),
        None => Ok(None),
    }
}

fn read_goal_link(store: &WorkspaceStore, id: &str) -> Result<Option<GoalLinkRecord>, String> {
    match store.setting_get(&goal_link_key(id))? {
        Some(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| format!("parse posture:goal_link:{id}: {e}")),
        None => Ok(None),
    }
}

fn apply_status_overrides(
    store: &WorkspaceStore,
    finding: &mut SecurityFinding,
) -> Result<(), String> {
    // Suppression wins over goal-link (a suppressed finding is
    // explicitly silenced; the goal link is informational while
    // suppressed). On unsuppress, the goal link re-applies.
    if let Some(sup) = read_suppression(store, &finding.id)? {
        finding.status = FindingStatus::Suppressed {
            reason: sup.reason,
            at_unix_ms: sup.at_unix_ms,
        };
        return Ok(());
    }
    if let Some(link) = read_goal_link(store, &finding.id)? {
        finding.status = FindingStatus::GoalLinked {
            goal_id: link.goal_id,
            at_unix_ms: link.at_unix_ms,
        };
        return Ok(());
    }
    finding.status = FindingStatus::Open;
    Ok(())
}

fn append_decision_log_entry(
    store: &WorkspaceStore,
    entry: DecisionLogEntry,
) -> Result<(), String> {
    let mut entries = read_decision_log(store)?;
    entries.insert(0, entry); // newest-first
    if entries.len() > DECISION_LOG_MAX_ROWS {
        entries.truncate(DECISION_LOG_MAX_ROWS);
    }
    let json = serde_json::to_string(&entries).map_err(|e| e.to_string())?;
    store.setting_set(KEY_DECISION_LOG, &json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_posture::{Category, Severity};
    use tempfile::TempDir;

    fn fresh_store() -> (TempDir, WorkspaceStore) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("workspace.db");
        let store = WorkspaceStore::open_with(&path, [42u8; 32]).unwrap();
        (dir, store)
    }

    fn fixture_finding(id_seed: u32) -> SecurityFinding {
        SecurityFinding::new(
            "test_scanner",
            Severity::High,
            Category::Sast,
            format!("src/file_{id_seed}.rs"),
            Some(id_seed),
            None,
            Some("snippet".to_string()),
            format!("RULE-{id_seed}"),
            format!("title {id_seed}"),
            None,
            vec![],
        )
    }

    #[test]
    fn upsert_and_list_roundtrip() {
        let (_dir, store) = fresh_store();
        upsert_finding(&store, fixture_finding(1)).unwrap();
        upsert_finding(&store, fixture_finding(2)).unwrap();
        let listed = list_findings(&store).unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[test]
    fn upsert_preserves_first_seen() {
        let (_dir, store) = fresh_store();
        let mut f1 = fixture_finding(1);
        f1.first_seen_unix_ms = 100;
        f1.last_seen_unix_ms = 100;
        upsert_finding(&store, f1.clone()).unwrap();

        let mut f2 = fixture_finding(1);
        f2.first_seen_unix_ms = 200;
        f2.last_seen_unix_ms = 200;
        upsert_finding(&store, f2.clone()).unwrap();

        let read = get_finding(&store, &f1.id).unwrap().unwrap();
        assert_eq!(read.first_seen_unix_ms, 100, "first_seen must be preserved");
        assert_eq!(read.last_seen_unix_ms, 200, "last_seen must update");
    }

    #[test]
    fn suppress_then_list_shows_suppressed_status() {
        let (_dir, store) = fresh_store();
        let f = fixture_finding(1);
        let id = f.id.clone();
        upsert_finding(&store, f).unwrap();
        suppress(&store, &id, "false-positive in test fixture").unwrap();
        let listed = list_findings(&store).unwrap();
        assert_eq!(listed.len(), 1);
        match &listed[0].status {
            FindingStatus::Suppressed { reason, .. } => {
                assert_eq!(reason, "false-positive in test fixture");
            }
            other => panic!("expected Suppressed, got {other:?}"),
        }
    }

    #[test]
    fn unsuppress_returns_to_open() {
        let (_dir, store) = fresh_store();
        let f = fixture_finding(1);
        let id = f.id.clone();
        upsert_finding(&store, f).unwrap();
        suppress(&store, &id, "test").unwrap();
        unsuppress(&store, &id).unwrap();
        let listed = list_findings(&store).unwrap();
        assert!(matches!(listed[0].status, FindingStatus::Open));
    }

    #[test]
    fn link_goal_surfaces_in_status() {
        let (_dir, store) = fresh_store();
        let f = fixture_finding(1);
        let id = f.id.clone();
        upsert_finding(&store, f).unwrap();
        link_goal(&store, &id, "goal-abc-123").unwrap();
        let listed = list_findings(&store).unwrap();
        match &listed[0].status {
            FindingStatus::GoalLinked { goal_id, .. } => {
                assert_eq!(goal_id, "goal-abc-123");
            }
            other => panic!("expected GoalLinked, got {other:?}"),
        }
    }

    #[test]
    fn suppression_takes_precedence_over_goal_link() {
        // Both records present; suppression wins so the finding
        // shows as silenced rather than as actionable work.
        let (_dir, store) = fresh_store();
        let f = fixture_finding(1);
        let id = f.id.clone();
        upsert_finding(&store, f).unwrap();
        link_goal(&store, &id, "goal-xyz").unwrap();
        suppress(&store, &id, "won't fix in v1").unwrap();
        let listed = list_findings(&store).unwrap();
        assert!(matches!(listed[0].status, FindingStatus::Suppressed { .. }));
    }

    #[test]
    fn unsuppress_restores_goal_link_status() {
        // If a finding was suppressed *after* a goal link, lifting
        // the suppression should reveal the goal-linked state, not
        // collapse to Open.
        let (_dir, store) = fresh_store();
        let f = fixture_finding(1);
        let id = f.id.clone();
        upsert_finding(&store, f).unwrap();
        link_goal(&store, &id, "goal-xyz").unwrap();
        suppress(&store, &id, "temporary").unwrap();
        unsuppress(&store, &id).unwrap();
        let listed = list_findings(&store).unwrap();
        match &listed[0].status {
            FindingStatus::GoalLinked { goal_id, .. } => assert_eq!(goal_id, "goal-xyz"),
            other => panic!("expected goal link to be restored, got {other:?}"),
        }
    }

    #[test]
    fn decision_log_records_operations() {
        let (_dir, store) = fresh_store();
        let f = fixture_finding(1);
        let id = f.id.clone();
        upsert_finding(&store, f).unwrap();
        suppress(&store, &id, "reason A").unwrap();
        link_goal(&store, &id, "goal-1").unwrap();
        unsuppress(&store, &id).unwrap();
        let log = read_decision_log(&store).unwrap();
        assert_eq!(log.len(), 3, "three operations");
        assert!(matches!(log[0].operation, DecisionOperation::Unsuppress));
        assert!(matches!(log[1].operation, DecisionOperation::LinkGoal));
        assert!(matches!(log[2].operation, DecisionOperation::Suppress));
        assert_eq!(log[2].reason.as_deref(), Some("reason A"));
        assert!(log[0].reason.is_none(), "unsuppress carries no reason");
        assert!(log[1].reason.is_none(), "link_goal carries no reason");
    }

    #[test]
    fn decision_log_fifo_eviction_at_cap() {
        let (_dir, store) = fresh_store();
        let f = fixture_finding(1);
        let id = f.id.clone();
        upsert_finding(&store, f).unwrap();
        // Spam the log past the cap. Each suppress/unsuppress pair
        // appends two entries.
        for n in 0..(DECISION_LOG_MAX_ROWS) {
            suppress(&store, &id, &format!("attempt {n}")).unwrap();
            unsuppress(&store, &id).unwrap();
        }
        let log = read_decision_log(&store).unwrap();
        assert!(log.len() <= DECISION_LOG_MAX_ROWS, "FIFO eviction at cap");
    }
}
