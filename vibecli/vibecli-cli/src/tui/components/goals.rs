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
    pub updated_at: String,
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
    pub last_error: Option<String>,
}

impl GoalsComponent {
    pub fn new() -> Self {
        let mut c = Self {
            items: Vec::new(),
            selected_index: 0,
            status_filter: StatusFilter::Active,
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
        match store.list_goals(&filter) {
            Ok(rows) => {
                self.items = rows
                    .into_iter()
                    .map(|g| GoalRow {
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
                        updated_at: g.updated_at.to_rfc3339(),
                    })
                    .collect();
                self.last_error = None;
                if self.selected_index >= self.items.len() {
                    self.selected_index = self.items.len().saturating_sub(1);
                }
            }
            Err(e) => self.last_error = Some(format!("list: {e}")),
        }
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
            last_error: None,
        };
        c.next();
        c.previous();
        assert_eq!(c.selected_index, 0);
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
                    updated_at: "now".into(),
                },
                GoalRow {
                    id: "b".into(),
                    title: "b".into(),
                    status: "active".into(),
                    workspace_label: "global".into(),
                    updated_at: "now".into(),
                },
            ],
            selected_index: 1,
            status_filter: StatusFilter::Active,
            last_error: None,
        };
        c.next();
        assert_eq!(c.selected_index, 0, "wrap-around to top");
    }
}
