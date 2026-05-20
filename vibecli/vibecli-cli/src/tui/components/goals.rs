//! TUI Goals component (G3.1).
//!
//! Read-only list view that reads goals straight from
//! `SessionStore::list_goals` (no daemon round-trip). Mirrors the
//! `FileTreeComponent` shape — a `Vec` plus a selection index plus
//! cheap navigation methods. Status filter cycles through Active →
//! Paused → Done → Abandoned → All on `f`.

use crate::exec_goal::GoalStatus;
use crate::session_store::{GoalListFilter, SessionStore};

#[derive(Debug, Clone)]
pub struct GoalRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub workspace_label: String,
    /// G13.1 — raw workspace path (used as the pin key when toggling).
    /// `None` = global, mirroring `Goal::workspace`.
    pub workspace: Option<String>,
    pub updated_at: String,
    /// G11.1 — parent for tree-mode ordering. `None` in list mode and
    /// for true roots.
    pub parent_goal_id: Option<String>,
    /// G11.1 — depth in tree mode (0 = root). Always 0 in list mode.
    pub depth: u8,
    /// G13.1 — this goal id appears in `pinned_goals` for some
    /// workspace (or the global slot). UI shows ★ on the row.
    pub pinned: bool,
}

/// G11.1 — TUI Goals layout. `List` is the flat chronological view
/// (default); `Tree` indents children under parents using a client-
/// side BFS over `parent_goal_id`, mirroring VibeUI's `orderedGoals`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Tree,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::List => "list",
            ViewMode::Tree => "tree",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            ViewMode::List => ViewMode::Tree,
            ViewMode::Tree => ViewMode::List,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StatusFilter {
    Active,
    Paused,
    Done,
    Abandoned,
    All,
}

impl StatusFilter {
    pub fn label(self) -> &'static str {
        match self {
            StatusFilter::Active => "active",
            StatusFilter::Paused => "paused",
            StatusFilter::Done => "done",
            StatusFilter::Abandoned => "abandoned",
            StatusFilter::All => "all",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            StatusFilter::Active => StatusFilter::Paused,
            StatusFilter::Paused => StatusFilter::Done,
            StatusFilter::Done => StatusFilter::Abandoned,
            StatusFilter::Abandoned => StatusFilter::All,
            StatusFilter::All => StatusFilter::Active,
        }
    }

    fn as_goal_status(self) -> Option<GoalStatus> {
        match self {
            StatusFilter::Active => Some(GoalStatus::Active),
            StatusFilter::Paused => Some(GoalStatus::Paused),
            StatusFilter::Done => Some(GoalStatus::Done),
            StatusFilter::Abandoned => Some(GoalStatus::Abandoned),
            StatusFilter::All => None,
        }
    }
}

pub struct GoalsComponent {
    pub items: Vec<GoalRow>,
    pub selected_index: usize,
    pub status_filter: StatusFilter,
    pub view_mode: ViewMode,
    pub last_error: Option<String>,
}

impl GoalsComponent {
    pub fn new() -> Self {
        let mut c = Self {
            items: Vec::new(),
            selected_index: 0,
            status_filter: StatusFilter::Active,
            view_mode: ViewMode::List,
            last_error: None,
        };
        c.refresh();
        c
    }

    /// Pull the current list straight from `~/.vibecli/sessions.db`.
    /// Best-effort: failures leave the prior list in place and stash
    /// the error so the renderer can surface it.
    pub fn refresh(&mut self) {
        let store = match SessionStore::open_default() {
            Ok(s) => s,
            Err(e) => {
                self.last_error = Some(format!("open store: {e}"));
                return;
            }
        };
        let filter = GoalListFilter {
            status: self.status_filter.as_goal_status(),
            limit: 100,
            ..Default::default()
        };
        let pinned_ids = store.list_all_pinned_goal_ids().unwrap_or_default();
        match store.list_goals(&filter) {
            Ok(rows) => {
                let goal_rows: Vec<GoalRow> = rows
                    .into_iter()
                    .map(|g| GoalRow {
                        pinned: pinned_ids.contains(&g.id),
                        id: g.id,
                        title: g.title,
                        status: g.status.as_str().to_string(),
                        workspace_label: g
                            .workspace
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("global")
                            .to_string(),
                        workspace: g
                            .workspace
                            .as_ref()
                            .and_then(|p| p.to_str())
                            .map(|s| s.to_string()),
                        updated_at: g.updated_at.to_rfc3339(),
                        parent_goal_id: g.parent_goal_id,
                        depth: 0,
                    })
                    .collect();
                self.items = match self.view_mode {
                    ViewMode::List => goal_rows,
                    ViewMode::Tree => order_as_tree(goal_rows),
                };
                self.last_error = None;
                if self.selected_index >= self.items.len() {
                    self.selected_index = self.items.len().saturating_sub(1);
                }
            }
            Err(e) => self.last_error = Some(format!("list: {e}")),
        }
    }

    /// G13.1 — flip the pin state of the currently selected row. If
    /// the row is pinned anywhere we unpin every workspace slot that
    /// holds it (covers the rare both-workspace-and-global case). If
    /// it isn't pinned we pin it under its own workspace (or globally
    /// if the goal has no workspace) — matching how the watch/mobile
    /// pin flows scope their writes. Best-effort: errors are stashed.
    pub fn toggle_pin_current(&mut self) {
        let Some(row) = self.selected().cloned() else {
            return;
        };
        let store = match SessionStore::open_default() {
            Ok(s) => s,
            Err(e) => {
                self.last_error = Some(format!("open store: {e}"));
                return;
            }
        };
        let res = if row.pinned {
            match store.list_pin_workspaces_for_goal(&row.id) {
                Ok(workspaces) => {
                    for ws in workspaces {
                        let ws_opt = if ws.is_empty() { None } else { Some(ws.as_str()) };
                        let _ = store.unpin_goal(ws_opt);
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            store.pin_goal(row.workspace.as_deref(), &row.id)
        };
        if let Err(e) = res {
            self.last_error = Some(format!("pin toggle: {e}"));
        }
        self.refresh();
    }

    /// G11.1 — flip between flat list and tree layout.
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = self.view_mode.toggle();
        self.selected_index = 0;
        self.refresh();
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.items.len() - 1;
            }
        }
    }

    pub fn cycle_filter(&mut self) {
        self.status_filter = self.status_filter.cycle();
        self.selected_index = 0;
        self.refresh();
    }

    pub fn selected(&self) -> Option<&GoalRow> {
        self.items.get(self.selected_index)
    }
}

impl Default for GoalsComponent {
    fn default() -> Self {
        Self::new()
    }
}

/// G11.1 — re-order a flat goal list into parent → children sequence
/// with per-row `depth` so the renderer can indent. Roots are goals
/// whose `parent_goal_id` is either `None` or refers to a goal that
/// isn't in the current list (a parent filtered out by the status
/// filter is treated as a root so its children remain reachable).
/// Mirrors `orderedGoals` in `vibeui/src/components/GoalPanel.tsx`.
fn order_as_tree(rows: Vec<GoalRow>) -> Vec<GoalRow> {
    use std::collections::HashMap;
    if rows.is_empty() {
        return rows;
    }
    let id_set: std::collections::HashSet<String> =
        rows.iter().map(|r| r.id.clone()).collect();
    let mut by_parent: HashMap<Option<String>, Vec<GoalRow>> = HashMap::new();
    for r in rows {
        let key = r
            .parent_goal_id
            .clone()
            .filter(|p| id_set.contains(p));
        by_parent.entry(key).or_default().push(r);
    }
    let mut out: Vec<GoalRow> = Vec::new();
    walk_tree(&mut by_parent, None, 0, &mut out);
    out
}

fn walk_tree(
    by_parent: &mut std::collections::HashMap<Option<String>, Vec<GoalRow>>,
    parent: Option<String>,
    depth: u8,
    out: &mut Vec<GoalRow>,
) {
    if let Some(kids) = by_parent.remove(&parent) {
        for mut kid in kids {
            kid.depth = depth;
            let id = kid.id.clone();
            out.push(kid);
            walk_tree(by_parent, Some(id), depth.saturating_add(1), out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_filter_cycle_loops() {
        let mut s = StatusFilter::Active;
        s = s.cycle();
        assert!(matches!(s, StatusFilter::Paused));
        s = s.cycle();
        assert!(matches!(s, StatusFilter::Done));
        s = s.cycle();
        assert!(matches!(s, StatusFilter::Abandoned));
        s = s.cycle();
        assert!(matches!(s, StatusFilter::All));
        s = s.cycle();
        assert!(matches!(s, StatusFilter::Active));
    }

    #[test]
    fn next_and_previous_are_safe_when_empty() {
        let mut c = GoalsComponent {
            items: Vec::new(),
            selected_index: 0,
            status_filter: StatusFilter::Active,
            view_mode: ViewMode::List,
            last_error: None,
        };
        c.next();
        c.previous();
        assert_eq!(c.selected_index, 0);
    }

    fn row(id: &str, parent: Option<&str>) -> GoalRow {
        GoalRow {
            id: id.into(),
            title: id.into(),
            status: "active".into(),
            workspace_label: "global".into(),
            workspace: None,
            updated_at: "now".into(),
            parent_goal_id: parent.map(str::to_string),
            depth: 0,
            pinned: false,
        }
    }

    #[test]
    fn order_as_tree_indents_children_under_parents() {
        // Two roots; root-A has one child; root-B has a grand-child
        // chain. Output order: A, A1, B, B1, B1a, with depths 0 1 0 1 2.
        let input = vec![
            row("A", None),
            row("A1", Some("A")),
            row("B", None),
            row("B1", Some("B")),
            row("B1a", Some("B1")),
        ];
        let out = order_as_tree(input);
        let ordered: Vec<(&str, u8)> = out.iter().map(|r| (r.id.as_str(), r.depth)).collect();
        assert_eq!(
            ordered,
            vec![("A", 0), ("A1", 1), ("B", 0), ("B1", 1), ("B1a", 2)],
        );
    }

    #[test]
    fn order_as_tree_treats_orphan_parent_as_root() {
        // The parent_goal_id `MISSING` isn't in the list (filtered out
        // by status, say). The child must still render, as a root.
        let input = vec![row("orphan", Some("MISSING"))];
        let out = order_as_tree(input);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "orphan");
        assert_eq!(out[0].depth, 0);
    }

    #[test]
    fn view_mode_toggle_round_trips() {
        let m = ViewMode::List;
        assert_eq!(m.toggle(), ViewMode::Tree);
        assert_eq!(m.toggle().toggle(), ViewMode::List);
    }

    #[test]
    fn next_wraps_around() {
        let mut c = GoalsComponent {
            items: vec![
                GoalRow {
                    id: "a".into(),
                    title: "a".into(),
                    status: "active".into(),
                    workspace_label: "global".into(),
                    workspace: None,
                    updated_at: "now".into(),
                    parent_goal_id: None,
                    depth: 0,
                    pinned: false,
                },
                GoalRow {
                    id: "b".into(),
                    title: "b".into(),
                    status: "active".into(),
                    workspace_label: "global".into(),
                    workspace: None,
                    updated_at: "now".into(),
                    parent_goal_id: None,
                    depth: 0,
                    pinned: false,
                },
            ],
            selected_index: 1,
            status_filter: StatusFilter::Active,
            view_mode: ViewMode::List,
            last_error: None,
        };
        c.next();
        assert_eq!(c.selected_index, 0, "wrap-around to top");
    }
}
