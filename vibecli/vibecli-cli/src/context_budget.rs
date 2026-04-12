#![allow(dead_code)]
//! Context budget enforcer — token budget bar, soft-warn + hard-limit enforcement,
//! and automatic pruning strategies.
//!
//! Matches GitHub Copilot Workspace v2's context budget feature.

use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Budget limits
// ---------------------------------------------------------------------------

/// Soft and hard token limits for a context window.
#[derive(Debug, Clone)]
pub struct BudgetLimits {
    /// Hard maximum — generation must not exceed this.
    pub hard_limit: usize,
    /// Soft warning threshold — surface a warning to the user.
    pub warn_at: usize,
    /// Auto-prune threshold — automatically prune context to make room.
    pub prune_at: usize,
}

impl BudgetLimits {
    pub fn new(hard_limit: usize) -> Self {
        Self {
            hard_limit,
            warn_at: (hard_limit as f64 * 0.80) as usize,
            prune_at: (hard_limit as f64 * 0.90) as usize,
        }
    }

    pub fn with_warn_at(mut self, warn_at: usize) -> Self {
        self.warn_at = warn_at;
        self
    }

    pub fn with_prune_at(mut self, prune_at: usize) -> Self {
        self.prune_at = prune_at;
        self
    }

    /// Fraction of hard limit consumed [0.0, 1.0+].
    pub fn utilisation(&self, used: usize) -> f64 {
        used as f64 / self.hard_limit as f64
    }
}

// ---------------------------------------------------------------------------
// Budget state
// ---------------------------------------------------------------------------

/// Action the enforcer recommends after an accounting update.
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetAction {
    /// Usage is below the warn threshold — all clear.
    Ok,
    /// Usage crossed the warn threshold — show a warning.
    Warn { used: usize, limit: usize },
    /// Usage crossed the prune threshold — prune context automatically.
    Prune { bytes_to_free: usize },
    /// Usage would exceed the hard limit — block the operation.
    Block { used: usize, limit: usize },
}

impl std::fmt::Display for BudgetAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetAction::Ok => write!(f, "ok"),
            BudgetAction::Warn { used, limit } => {
                write!(f, "warn({used}/{limit} tokens)")
            }
            BudgetAction::Prune { bytes_to_free } => {
                write!(f, "prune(free {bytes_to_free} tokens)")
            }
            BudgetAction::Block { used, limit } => {
                write!(f, "block({used} > {limit} limit)")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Context entry types (for pruning strategy)
// ---------------------------------------------------------------------------

/// Category of a context entry — determines pruning priority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryKind {
    /// Oldest tool result — pruned first.
    OldToolResult,
    /// File attachment — pruned second.
    Attachment,
    /// Conversation history (user/assistant turns) — pruned last.
    History,
    /// System prompt — never pruned.
    SystemPrompt,
}

impl std::fmt::Display for EntryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryKind::OldToolResult => write!(f, "tool_result"),
            EntryKind::Attachment => write!(f, "attachment"),
            EntryKind::History => write!(f, "history"),
            EntryKind::SystemPrompt => write!(f, "system_prompt"),
        }
    }
}

/// A tracked entry in the context window.
#[derive(Debug, Clone)]
pub struct ContextEntry {
    pub id: String,
    pub kind: EntryKind,
    pub tokens: usize,
    /// Whether this entry was pruned in the last auto-prune pass.
    pub pruned: bool,
}

impl ContextEntry {
    pub fn new(id: impl Into<String>, kind: EntryKind, tokens: usize) -> Self {
        Self {
            id: id.into(),
            kind,
            tokens,
            pruned: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Context budget enforcer
// ---------------------------------------------------------------------------

/// Tracks token usage across context entries and enforces budget limits.
pub struct ContextBudget {
    limits: BudgetLimits,
    entries: VecDeque<ContextEntry>,
    /// Running total of non-pruned token usage.
    used_tokens: usize,
}

impl ContextBudget {
    pub fn new(limits: BudgetLimits) -> Self {
        Self {
            limits,
            entries: VecDeque::new(),
            used_tokens: 0,
        }
    }

    /// Add a context entry and return the recommended action.
    pub fn add_entry(&mut self, entry: ContextEntry) -> BudgetAction {
        let new_total = self.used_tokens + entry.tokens;

        if new_total > self.limits.hard_limit {
            return BudgetAction::Block {
                used: new_total,
                limit: self.limits.hard_limit,
            };
        }

        self.used_tokens += entry.tokens;
        self.entries.push_back(entry);

        self.evaluate()
    }

    /// Update token count for an existing entry.
    pub fn update_entry(&mut self, id: &str, new_tokens: usize) -> Option<BudgetAction> {
        for entry in self.entries.iter_mut() {
            if entry.id == id && !entry.pruned {
                let delta = new_tokens as isize - entry.tokens as isize;
                entry.tokens = new_tokens;
                if delta > 0 {
                    self.used_tokens += delta as usize;
                } else {
                    self.used_tokens = self.used_tokens.saturating_sub((-delta) as usize);
                }
                return Some(self.evaluate());
            }
        }
        None
    }

    /// Remove an entry (e.g. tool result discarded by the agent).
    pub fn remove_entry(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| {
            if e.id == id && !e.pruned {
                self.used_tokens = self.used_tokens.saturating_sub(e.tokens);
                false
            } else {
                true
            }
        });
        self.entries.len() < before
    }

    /// Auto-prune entries using the strategy:
    /// OldToolResult → Attachment → History (never SystemPrompt).
    /// Returns list of pruned entry IDs.
    pub fn auto_prune(&mut self, target_tokens: usize) -> Vec<String> {
        let mut pruned_ids = vec![];
        let prune_order = [
            EntryKind::OldToolResult,
            EntryKind::Attachment,
            EntryKind::History,
        ];

        'outer: for kind in &prune_order {
            for entry in self.entries.iter_mut() {
                if self.used_tokens <= target_tokens {
                    break 'outer;
                }
                if &entry.kind == kind && !entry.pruned {
                    self.used_tokens = self.used_tokens.saturating_sub(entry.tokens);
                    entry.pruned = true;
                    pruned_ids.push(entry.id.clone());
                }
            }
        }

        pruned_ids
    }

    /// Current token usage (non-pruned entries only).
    pub fn used(&self) -> usize {
        self.used_tokens
    }

    /// Remaining tokens before the hard limit.
    pub fn remaining(&self) -> usize {
        self.limits.hard_limit.saturating_sub(self.used_tokens)
    }

    /// Percentage of hard limit used [0.0, 100.0+].
    pub fn utilisation_pct(&self) -> f64 {
        self.limits.utilisation(self.used_tokens) * 100.0
    }

    /// Number of active (non-pruned) entries.
    pub fn active_entry_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.pruned).count()
    }

    /// Render a text progress bar: `[████████░░░░] 80% (80,000 / 100,000)`
    pub fn render_bar(&self, width: usize) -> String {
        let pct = self.limits.utilisation(self.used_tokens).min(1.0);
        let filled = (pct * width as f64).round() as usize;
        let empty = width.saturating_sub(filled);
        let bar = format!(
            "[{}{}] {:.0}% ({} / {})",
            "█".repeat(filled),
            "░".repeat(empty),
            pct * 100.0,
            self.used_tokens,
            self.limits.hard_limit,
        );
        bar
    }

    // Evaluate current usage and return the appropriate action.
    fn evaluate(&self) -> BudgetAction {
        let used = self.used_tokens;
        if used >= self.limits.prune_at {
            let bytes_to_free = used.saturating_sub(self.limits.warn_at);
            BudgetAction::Prune { bytes_to_free }
        } else if used >= self.limits.warn_at {
            BudgetAction::Warn {
                used,
                limit: self.limits.hard_limit,
            }
        } else {
            BudgetAction::Ok
        }
    }
}

// ---------------------------------------------------------------------------
// Budget snapshot for reporting
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BudgetSnapshot {
    pub used: usize,
    pub hard_limit: usize,
    pub warn_at: usize,
    pub prune_at: usize,
    pub utilisation_pct: f64,
    pub active_entries: usize,
    pub bar: String,
}

impl ContextBudget {
    pub fn snapshot(&self) -> BudgetSnapshot {
        BudgetSnapshot {
            used: self.used_tokens,
            hard_limit: self.limits.hard_limit,
            warn_at: self.limits.warn_at,
            prune_at: self.limits.prune_at,
            utilisation_pct: self.utilisation_pct(),
            active_entries: self.active_entry_count(),
            bar: self.render_bar(30),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(hard: usize) -> ContextBudget {
        ContextBudget::new(BudgetLimits::new(hard))
    }

    fn entry(id: &str, kind: EntryKind, tokens: usize) -> ContextEntry {
        ContextEntry::new(id, kind, tokens)
    }

    #[test]
    fn test_ok_below_warn() {
        let mut b = budget(100_000);
        let action = b.add_entry(entry("e1", EntryKind::History, 1_000));
        assert_eq!(action, BudgetAction::Ok);
    }

    #[test]
    fn test_warn_at_80pct() {
        let mut b = budget(100_000);
        let action = b.add_entry(entry("e1", EntryKind::History, 82_000));
        assert!(matches!(action, BudgetAction::Warn { .. }));
    }

    #[test]
    fn test_prune_at_90pct() {
        let mut b = budget(100_000);
        let action = b.add_entry(entry("e1", EntryKind::History, 91_000));
        assert!(matches!(action, BudgetAction::Prune { .. }));
    }

    #[test]
    fn test_block_over_hard_limit() {
        let mut b = budget(10_000);
        b.add_entry(entry("e1", EntryKind::History, 9_000));
        let action = b.add_entry(entry("e2", EntryKind::Attachment, 5_000));
        assert!(matches!(action, BudgetAction::Block { .. }));
        // e2 should not have been added
        assert_eq!(b.used(), 9_000);
    }

    #[test]
    fn test_auto_prune_tool_results_first() {
        let mut b = budget(100_000);
        b.add_entry(entry("tool1", EntryKind::OldToolResult, 10_000));
        b.add_entry(entry("attach1", EntryKind::Attachment, 20_000));
        b.add_entry(entry("hist1", EntryKind::History, 30_000));
        assert_eq!(b.used(), 60_000);

        // prune down to 30_000
        let pruned = b.auto_prune(30_000);
        assert!(pruned.contains(&"tool1".to_string()));
        // attach may also be pruned if needed
        assert!(b.used() <= 30_000);
    }

    #[test]
    fn test_remove_entry() {
        let mut b = budget(100_000);
        b.add_entry(entry("r1", EntryKind::History, 5_000));
        assert_eq!(b.used(), 5_000);
        b.remove_entry("r1");
        assert_eq!(b.used(), 0);
    }

    #[test]
    fn test_update_entry_grows() {
        let mut b = budget(100_000);
        b.add_entry(entry("u1", EntryKind::History, 1_000));
        b.update_entry("u1", 3_000);
        assert_eq!(b.used(), 3_000);
    }

    #[test]
    fn test_render_bar_format() {
        let mut b = budget(100_000);
        b.add_entry(entry("e", EntryKind::SystemPrompt, 50_000));
        let bar = b.render_bar(20);
        assert!(bar.contains('%'));
        assert!(bar.contains('/'));
    }

    #[test]
    fn test_remaining_calculation() {
        let mut b = budget(10_000);
        b.add_entry(entry("x", EntryKind::History, 3_000));
        assert_eq!(b.remaining(), 7_000);
    }

    #[test]
    fn test_snapshot_fields() {
        let mut b = budget(50_000);
        b.add_entry(entry("s", EntryKind::History, 10_000));
        let snap = b.snapshot();
        assert_eq!(snap.hard_limit, 50_000);
        assert_eq!(snap.used, 10_000);
        assert_eq!(snap.active_entries, 1);
    }

    #[test]
    fn test_system_prompt_not_pruned() {
        let mut b = budget(100_000);
        b.add_entry(entry("sys", EntryKind::SystemPrompt, 5_000));
        b.add_entry(entry("old", EntryKind::OldToolResult, 30_000));
        b.auto_prune(10_000);
        // sys should survive
        let sys = b.entries.iter().find(|e| e.id == "sys").unwrap();
        assert!(!sys.pruned);
    }

    #[test]
    fn test_utilisation_pct() {
        let mut b = budget(200_000);
        b.add_entry(entry("x", EntryKind::History, 100_000));
        assert!((b.utilisation_pct() - 50.0).abs() < 0.01);
    }
}
