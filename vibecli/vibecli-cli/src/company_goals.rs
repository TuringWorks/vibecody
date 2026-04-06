#![allow(dead_code)]
//! Hierarchical goal management for VibeCody company orchestration.
//!
//! Goals form a tree: company → team → agent → task-level goals.
//! Every agent action can trace back to a company mission via the goal chain.
//! Progress rolls up automatically from child goals to parent goals.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn new_id() -> String { uuid::Uuid::new_v4().to_string() }

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Planned,
    Active,
    Achieved,
    Cancelled,
}

impl GoalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Planned => "planned",
            Self::Active => "active",
            Self::Achieved => "achieved",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "achieved" => Self::Achieved,
            "cancelled" => Self::Cancelled,
            _ => Self::Planned,
        }
    }
}

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub company_id: String,
    pub parent_goal_id: Option<String>,
    pub owner_agent_id: Option<String>,
    pub title: String,
    pub description: String,
    pub status: GoalStatus,
    /// 0 = low, 3 = critical
    pub priority: i64,
    pub target_date: Option<u64>,
    /// 0–100
    pub progress_pct: i64,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Tree node for goal hierarchy display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalTreeNode {
    pub goal: Goal,
    pub children: Vec<GoalTreeNode>,
}

// ── GoalStore (methods added to CompanyStore's connection) ────────────────────

/// Extension methods for managing goals — operates on an existing Connection.
pub struct GoalStore<'a> {
    conn: &'a Connection,
}

impl<'a> GoalStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS goals (
                id              TEXT PRIMARY KEY,
                company_id      TEXT NOT NULL,
                parent_goal_id  TEXT REFERENCES goals(id) ON DELETE SET NULL,
                owner_agent_id  TEXT,
                title           TEXT NOT NULL,
                description     TEXT NOT NULL DEFAULT '',
                status          TEXT NOT NULL DEFAULT 'planned',
                priority        INTEGER NOT NULL DEFAULT 0,
                target_date     INTEGER,
                progress_pct    INTEGER NOT NULL DEFAULT 0,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_goals_company ON goals(company_id);
            CREATE INDEX IF NOT EXISTS idx_goals_parent ON goals(parent_goal_id);
        "#)?;
        Ok(())
    }

    pub fn create(&self, company_id: &str, title: &str, description: &str,
                  parent_goal_id: Option<&str>, owner_agent_id: Option<&str>,
                  priority: i64) -> Result<Goal> {
        let goal = Goal {
            id: new_id(),
            company_id: company_id.to_string(),
            parent_goal_id: parent_goal_id.map(|s| s.to_string()),
            owner_agent_id: owner_agent_id.map(|s| s.to_string()),
            title: title.to_string(),
            description: description.to_string(),
            status: GoalStatus::Planned,
            priority,
            target_date: None,
            progress_pct: 0,
            created_at: now_ms(),
            updated_at: now_ms(),
        };
        self.conn.execute(
            "INSERT INTO goals (id, company_id, parent_goal_id, owner_agent_id, title, description, status, priority, progress_pct, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                goal.id, goal.company_id, goal.parent_goal_id, goal.owner_agent_id,
                goal.title, goal.description, goal.status.as_str(), goal.priority,
                goal.progress_pct, goal.created_at as i64, goal.updated_at as i64,
            ],
        )?;
        Ok(goal)
    }

    pub fn list(&self, company_id: &str) -> Result<Vec<Goal>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, parent_goal_id, owner_agent_id, title, description,
                    status, priority, target_date, progress_pct, created_at, updated_at
             FROM goals WHERE company_id = ?1 ORDER BY priority DESC, created_at ASC",
        )?;
        let rows = stmt.query_map(params![company_id], |row| {
            Ok(Goal {
                id: row.get(0)?,
                company_id: row.get(1)?,
                parent_goal_id: row.get(2)?,
                owner_agent_id: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                status: GoalStatus::from_str(&row.get::<_, String>(6)?),
                priority: row.get(7)?,
                target_date: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                progress_pct: row.get(9)?,
                created_at: row.get::<_, i64>(10)? as u64,
                updated_at: row.get::<_, i64>(11)? as u64,
            })
        })?;
        rows.collect::<rusqlite::Result<_>>().map_err(|e| e.into())
    }

    pub fn get(&self, id: &str) -> Result<Option<Goal>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, parent_goal_id, owner_agent_id, title, description,
                    status, priority, target_date, progress_pct, created_at, updated_at
             FROM goals WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Goal {
                id: row.get(0)?,
                company_id: row.get(1)?,
                parent_goal_id: row.get(2)?,
                owner_agent_id: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                status: GoalStatus::from_str(&row.get::<_, String>(6)?),
                priority: row.get(7)?,
                target_date: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                progress_pct: row.get(9)?,
                created_at: row.get::<_, i64>(10)? as u64,
                updated_at: row.get::<_, i64>(11)? as u64,
            })
        })?;
        rows.next().transpose().map_err(|e| e.into())
    }

    pub fn update_status(&self, id: &str, status: GoalStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE goals SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now_ms() as i64, id],
        )?;
        Ok(())
    }

    pub fn update_progress(&self, id: &str, pct: i64) -> Result<()> {
        let pct = pct.clamp(0, 100);
        self.conn.execute(
            "UPDATE goals SET progress_pct = ?1, updated_at = ?2 WHERE id = ?3",
            params![pct, now_ms() as i64, id],
        )?;
        self.roll_up_progress(id)?;
        Ok(())
    }

    /// Roll up progress from child goals to parent.
    fn roll_up_progress(&self, goal_id: &str) -> Result<()> {
        let goal = match self.get(goal_id)? {
            Some(g) => g,
            None => return Ok(()),
        };
        if let Some(parent_id) = &goal.parent_goal_id {
            // Average progress of all direct children
            let avg: Option<f64> = self.conn.query_row(
                "SELECT AVG(progress_pct) FROM goals WHERE parent_goal_id = ?1",
                params![parent_id],
                |r| r.get(0),
            ).ok();
            if let Some(avg_pct) = avg {
                self.conn.execute(
                    "UPDATE goals SET progress_pct = ?1, updated_at = ?2 WHERE id = ?3",
                    params![avg_pct as i64, now_ms() as i64, parent_id],
                )?;
                self.roll_up_progress(parent_id)?;
            }
        }
        Ok(())
    }

    /// Build full goal tree for a company (recursive).
    pub fn build_tree(&self, company_id: &str) -> Result<Vec<GoalTreeNode>> {
        let all = self.list(company_id)?;
        let mut by_parent: std::collections::HashMap<Option<String>, Vec<Goal>> =
            std::collections::HashMap::new();
        for g in all {
            by_parent.entry(g.parent_goal_id.clone()).or_default().push(g);
        }
        fn build(
            parent_id: Option<String>,
            map: &mut std::collections::HashMap<Option<String>, Vec<Goal>>,
        ) -> Vec<GoalTreeNode> {
            let children = map.remove(&parent_id).unwrap_or_default();
            children.into_iter().map(|goal| {
                let id = Some(goal.id.clone());
                GoalTreeNode { goal, children: build(id, map) }
            }).collect()
        }
        Ok(build(None, &mut by_parent))
    }
}

// ── Display helpers ───────────────────────────────────────────────────────────

pub fn print_goal_tree(nodes: &[GoalTreeNode], depth: usize) -> String {
    let mut out = String::new();
    for node in nodes {
        let indent = "  ".repeat(depth);
        let status_icon = match node.goal.status {
            GoalStatus::Planned => "○",
            GoalStatus::Active => "●",
            GoalStatus::Achieved => "✓",
            GoalStatus::Cancelled => "✗",
        };
        let bar = progress_bar(node.goal.progress_pct);
        out.push_str(&format!(
            "{}{} {} [{}] {}\n",
            indent, status_icon, node.goal.title, bar, node.goal.progress_pct
        ));
        if !node.goal.description.is_empty() {
            out.push_str(&format!("{}   {}\n", indent, node.goal.description));
        }
        out.push_str(&print_goal_tree(&node.children, depth + 1));
    }
    out
}

fn progress_bar(pct: i64) -> String {
    let filled = (pct / 10) as usize;
    let empty = 10 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_store() -> GoalStore<'static> {
        let conn = Connection::open_in_memory().unwrap();
        let conn: &'static Connection = Box::leak(Box::new(conn));
        let store = GoalStore::new(conn);
        store.ensure_schema().unwrap();
        store
    }

    // ── GoalStatus ────────────────────────────────────────────────────────────

    #[test]
    fn goal_status_round_trip() {
        for (s, v) in &[
            ("planned",   GoalStatus::Planned),
            ("active",    GoalStatus::Active),
            ("achieved",  GoalStatus::Achieved),
            ("cancelled", GoalStatus::Cancelled),
        ] {
            assert_eq!(GoalStatus::from_str(s), *v);
            assert_eq!(v.as_str(), *s);
        }
    }

    #[test]
    fn unknown_status_defaults_to_planned() {
        assert_eq!(GoalStatus::from_str("unknown"), GoalStatus::Planned);
        assert_eq!(GoalStatus::from_str(""), GoalStatus::Planned);
    }

    // ── GoalStore CRUD ────────────────────────────────────────────────────────

    #[test]
    fn create_and_get_goal() {
        let store = make_store();
        let goal = store.create("co-1", "Launch v1.0", "First release", None, None, 2).unwrap();
        assert!(!goal.id.is_empty());
        assert_eq!(goal.title, "Launch v1.0");
        assert_eq!(goal.status, GoalStatus::Planned);
        assert_eq!(goal.progress_pct, 0);

        let fetched = store.get(&goal.id).unwrap().expect("goal should exist");
        assert_eq!(fetched.id, goal.id);
        assert_eq!(fetched.title, "Launch v1.0");
    }

    #[test]
    fn get_nonexistent_goal_returns_none() {
        let store = make_store();
        let result = store.get("no-such-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_goals_by_company() {
        let store = make_store();
        store.create("co-1", "Goal A", "", None, None, 1).unwrap();
        store.create("co-1", "Goal B", "", None, None, 2).unwrap();
        store.create("co-2", "Goal C", "", None, None, 1).unwrap();

        let co1_goals = store.list("co-1").unwrap();
        assert_eq!(co1_goals.len(), 2);
        assert!(co1_goals.iter().all(|g| g.company_id == "co-1"));
    }

    #[test]
    fn list_returns_empty_for_unknown_company() {
        let store = make_store();
        assert!(store.list("no-company").unwrap().is_empty());
    }

    #[test]
    fn list_orders_by_priority_desc() {
        let store = make_store();
        store.create("co-1", "Low",      "", None, None, 0).unwrap();
        store.create("co-1", "Critical", "", None, None, 3).unwrap();
        store.create("co-1", "Medium",   "", None, None, 1).unwrap();

        let goals = store.list("co-1").unwrap();
        assert_eq!(goals[0].priority, 3);
        assert_eq!(goals[1].priority, 1);
        assert_eq!(goals[2].priority, 0);
    }

    // ── update_status ─────────────────────────────────────────────────────────

    #[test]
    fn update_status_persists() {
        let store = make_store();
        let goal = store.create("co-1", "Goal", "", None, None, 0).unwrap();
        store.update_status(&goal.id, GoalStatus::Active).unwrap();
        let fetched = store.get(&goal.id).unwrap().unwrap();
        assert_eq!(fetched.status, GoalStatus::Active);
    }

    // ── update_progress ───────────────────────────────────────────────────────

    #[test]
    fn update_progress_clamps_to_100() {
        let store = make_store();
        let goal = store.create("co-1", "Goal", "", None, None, 0).unwrap();
        store.update_progress(&goal.id, 150).unwrap();
        let fetched = store.get(&goal.id).unwrap().unwrap();
        assert_eq!(fetched.progress_pct, 100);
    }

    #[test]
    fn update_progress_clamps_to_0() {
        let store = make_store();
        let goal = store.create("co-1", "Goal", "", None, None, 0).unwrap();
        store.update_progress(&goal.id, -10).unwrap();
        let fetched = store.get(&goal.id).unwrap().unwrap();
        assert_eq!(fetched.progress_pct, 0);
    }

    #[test]
    fn update_progress_sets_value() {
        let store = make_store();
        let goal = store.create("co-1", "Goal", "", None, None, 0).unwrap();
        store.update_progress(&goal.id, 75).unwrap();
        let fetched = store.get(&goal.id).unwrap().unwrap();
        assert_eq!(fetched.progress_pct, 75);
    }

    // ── Progress roll-up ──────────────────────────────────────────────────────

    #[test]
    fn progress_rolls_up_to_parent() {
        let store = make_store();
        let parent = store.create("co-1", "Parent", "", None, None, 2).unwrap();
        let child1 = store.create("co-1", "Child 1", "", Some(&parent.id), None, 1).unwrap();
        let child2 = store.create("co-1", "Child 2", "", Some(&parent.id), None, 1).unwrap();

        store.update_progress(&child1.id, 60).unwrap();
        store.update_progress(&child2.id, 40).unwrap();

        let p = store.get(&parent.id).unwrap().unwrap();
        // avg(60, 40) = 50
        assert_eq!(p.progress_pct, 50);
    }

    #[test]
    fn progress_rolls_up_transitively() {
        let store = make_store();
        let grandparent = store.create("co-1", "GP", "", None, None, 3).unwrap();
        let parent = store.create("co-1", "Parent", "", Some(&grandparent.id), None, 2).unwrap();
        let child = store.create("co-1", "Child", "", Some(&parent.id), None, 1).unwrap();

        store.update_progress(&child.id, 100).unwrap();

        let p = store.get(&parent.id).unwrap().unwrap();
        assert_eq!(p.progress_pct, 100);
        let gp = store.get(&grandparent.id).unwrap().unwrap();
        assert_eq!(gp.progress_pct, 100);
    }

    // ── build_tree ────────────────────────────────────────────────────────────

    #[test]
    fn build_tree_returns_empty_for_no_goals() {
        let store = make_store();
        let tree = store.build_tree("co-1").unwrap();
        assert!(tree.is_empty());
    }

    #[test]
    fn build_tree_structures_hierarchy() {
        let store = make_store();
        let root = store.create("co-1", "Root goal", "", None, None, 3).unwrap();
        let child = store.create("co-1", "Child goal", "", Some(&root.id), None, 1).unwrap();
        let _grandchild = store.create("co-1", "Grandchild goal", "", Some(&child.id), None, 0).unwrap();

        let tree = store.build_tree("co-1").unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].goal.title, "Root goal");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].goal.title, "Child goal");
        assert_eq!(tree[0].children[0].children.len(), 1);
        assert_eq!(tree[0].children[0].children[0].goal.title, "Grandchild goal");
    }

    #[test]
    fn build_tree_multiple_roots() {
        let store = make_store();
        store.create("co-1", "Root A", "", None, None, 2).unwrap();
        store.create("co-1", "Root B", "", None, None, 1).unwrap();

        let tree = store.build_tree("co-1").unwrap();
        assert_eq!(tree.len(), 2);
    }

    // ── print_goal_tree ───────────────────────────────────────────────────────

    #[test]
    fn print_goal_tree_contains_title_and_progress() {
        let store = make_store();
        let goal = store.create("co-1", "My Goal", "", None, None, 0).unwrap();
        store.update_progress(&goal.id, 50).unwrap();

        let tree = store.build_tree("co-1").unwrap();
        let output = print_goal_tree(&tree, 0);
        assert!(output.contains("My Goal"));
        assert!(output.contains("50"));
        assert!(output.contains("○") || output.contains("●") || output.contains("✓") || output.contains("✗"));
    }

    #[test]
    fn print_goal_tree_indents_children() {
        let store = make_store();
        let root = store.create("co-1", "Root", "", None, None, 1).unwrap();
        store.create("co-1", "Child", "", Some(&root.id), None, 0).unwrap();

        let tree = store.build_tree("co-1").unwrap();
        let output = print_goal_tree(&tree, 0);
        // Child should be indented relative to root
        let root_pos = output.find("Root").unwrap();
        let child_pos = output.find("Child").unwrap();
        assert!(child_pos > root_pos);
        // The child line should have leading spaces
        let child_line = output.lines().find(|l| l.contains("Child")).unwrap();
        assert!(child_line.starts_with("  ")); // 2 spaces per depth level
    }

    // ── progress_bar ─────────────────────────────────────────────────────────

    #[test]
    fn progress_bar_0_pct_is_empty() {
        let bar = progress_bar(0);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn progress_bar_100_pct_is_full() {
        let bar = progress_bar(100);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn progress_bar_50_pct_is_half() {
        let bar = progress_bar(50);
        assert_eq!(bar, "█████░░░░░");
    }

    // ── owner_agent_id ────────────────────────────────────────────────────────

    #[test]
    fn create_goal_with_owner() {
        let store = make_store();
        let goal = store.create("co-1", "Owned goal", "", None, Some("agent-1"), 0).unwrap();
        assert_eq!(goal.owner_agent_id.as_deref(), Some("agent-1"));
    }

    // ── Timestamps ───────────────────────────────────────────────────────────

    #[test]
    fn created_at_is_recent() {
        let store = make_store();
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let goal = store.create("co-1", "Goal", "", None, None, 0).unwrap();
        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        assert!(goal.created_at >= before);
        assert!(goal.created_at <= after);
    }
}
