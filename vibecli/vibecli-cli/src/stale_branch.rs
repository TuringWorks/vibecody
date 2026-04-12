//! Stale branch detection and cleanup recommendations.
//!
//! Claw-code parity Wave 3: identifies branches that are likely abandoned and
//! recommends whether to delete, archive, or revive them.

use serde::{Deserialize, Serialize};

// ─── Branch Info ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub last_commit_ms: u64,
    pub commit_count: u32,
    pub is_merged: bool,
    pub has_open_pr: bool,
    pub author: String,
    pub ahead_of_main: u32,
    pub behind_main: u32,
}

impl BranchInfo {
    pub fn new(name: impl Into<String>, last_commit_ms: u64, author: impl Into<String>) -> Self {
        Self {
            name: name.into(), last_commit_ms, author: author.into(),
            commit_count: 1, is_merged: false, has_open_pr: false,
            ahead_of_main: 0, behind_main: 0,
        }
    }

    pub fn merged(mut self) -> Self { self.is_merged = true; self }
    pub fn with_pr(mut self) -> Self { self.has_open_pr = true; self }
    pub fn ahead(mut self, n: u32) -> Self { self.ahead_of_main = n; self }
    pub fn behind(mut self, n: u32) -> Self { self.behind_main = n; self }
    pub fn with_commits(mut self, n: u32) -> Self { self.commit_count = n; self }

    pub fn age_days(&self, now_ms: u64) -> u64 {
        (now_ms - self.last_commit_ms.min(now_ms)) / (24 * 3600 * 1000)
    }
}

// ─── Staleness Classification ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StalenessLabel {
    /// Active — recent commits, open PR, or ahead of main.
    Active,
    /// Merged and safe to delete.
    MergedCleanup,
    /// No recent activity but has unmerged commits.
    Dormant,
    /// Very old, no PR, likely abandoned.
    Stale,
    /// Extremely old, clearly abandoned.
    Zombie,
}

impl std::fmt::Display for StalenessLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active        => write!(f, "active"),
            Self::MergedCleanup => write!(f, "merged-cleanup"),
            Self::Dormant       => write!(f, "dormant"),
            Self::Stale         => write!(f, "stale"),
            Self::Zombie        => write!(f, "zombie"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleanupAction { Keep, Delete, Archive, Review }

impl std::fmt::Display for CleanupAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keep    => write!(f, "keep"),
            Self::Delete  => write!(f, "delete"),
            Self::Archive => write!(f, "archive"),
            Self::Review  => write!(f, "review"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessReport {
    pub branch: String,
    pub label: StalenessLabel,
    pub action: CleanupAction,
    pub age_days: u64,
    pub reason: String,
}

// ─── Stale Branch Detector ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessThresholds {
    pub dormant_days: u64,
    pub stale_days: u64,
    pub zombie_days: u64,
}

impl Default for StalenessThresholds {
    fn default() -> Self { Self { dormant_days: 14, stale_days: 60, zombie_days: 180 } }
}

pub struct StaleBranchDetector {
    pub thresholds: StalenessThresholds,
}

impl StaleBranchDetector {
    pub fn new(thresholds: StalenessThresholds) -> Self { Self { thresholds } }

    pub fn classify(&self, branch: &BranchInfo, now_ms: u64) -> StalenessReport {
        let age = branch.age_days(now_ms);

        // Never touch protected branches
        if branch.name == "main" || branch.name == "master" || branch.name == "develop" {
            return StalenessReport {
                branch: branch.name.clone(), label: StalenessLabel::Active,
                action: CleanupAction::Keep, age_days: age, reason: "protected branch".into(),
            };
        }

        // Merged with no PR or unmerged commits → delete
        if branch.is_merged && !branch.has_open_pr {
            return StalenessReport {
                branch: branch.name.clone(), label: StalenessLabel::MergedCleanup,
                action: CleanupAction::Delete, age_days: age,
                reason: "merged into main".into(),
            };
        }

        // Has open PR → active regardless of age
        if branch.has_open_pr {
            return StalenessReport {
                branch: branch.name.clone(), label: StalenessLabel::Active,
                action: CleanupAction::Keep, age_days: age,
                reason: "has open pull request".into(),
            };
        }

        // Recent commits → active
        if age < self.thresholds.dormant_days {
            return StalenessReport {
                branch: branch.name.clone(), label: StalenessLabel::Active,
                action: CleanupAction::Keep, age_days: age,
                reason: format!("{age} days old, within active window"),
            };
        }

        // Zombie
        if age >= self.thresholds.zombie_days {
            return StalenessReport {
                branch: branch.name.clone(), label: StalenessLabel::Zombie,
                action: CleanupAction::Delete, age_days: age,
                reason: format!("{age} days without activity"),
            };
        }

        // Stale with unmerged commits → archive
        if age >= self.thresholds.stale_days {
            let action = if branch.ahead_of_main > 0 { CleanupAction::Archive } else { CleanupAction::Delete };
            return StalenessReport {
                branch: branch.name.clone(), label: StalenessLabel::Stale,
                action, age_days: age,
                reason: format!("{age} days old, {} unmerged commits", branch.ahead_of_main),
            };
        }

        // Dormant
        let action = if branch.ahead_of_main > 0 { CleanupAction::Review } else { CleanupAction::Delete };
        StalenessReport {
            branch: branch.name.clone(), label: StalenessLabel::Dormant,
            action, age_days: age,
            reason: format!("{age} days without commits"),
        }
    }

    /// Classify all branches and return sorted by age descending.
    pub fn classify_all(&self, branches: &[BranchInfo], now_ms: u64) -> Vec<StalenessReport> {
        let mut reports: Vec<_> = branches.iter().map(|b| self.classify(b, now_ms)).collect();
        reports.sort_by(|a, b| b.age_days.cmp(&a.age_days));
        reports
    }

    /// Branches recommended for immediate deletion.
    pub fn deletion_candidates<'a>(&self, reports: &'a [StalenessReport]) -> Vec<&'a StalenessReport> {
        reports.iter().filter(|r| r.action == CleanupAction::Delete).collect()
    }
}

impl Default for StaleBranchDetector {
    fn default() -> Self { Self::new(StalenessThresholds::default()) }
}

// ── FreshnessState ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessState {
    Fresh,
    Stale,
    Diverged,
}

impl std::fmt::Display for FreshnessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fresh   => write!(f, "fresh"),
            Self::Stale   => write!(f, "stale"),
            Self::Diverged => write!(f, "diverged"),
        }
    }
}

// ── StalePolicy ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StalePolicy {
    #[default]
    WarnOnly,
    Block,
    AutoRebase,
    AutoMergeForward,
}

impl std::fmt::Display for StalePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WarnOnly         => write!(f, "warn_only"),
            Self::Block            => write!(f, "block"),
            Self::AutoRebase       => write!(f, "auto_rebase"),
            Self::AutoMergeForward => write!(f, "auto_merge_forward"),
        }
    }
}

// ── BranchFreshness ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BranchFreshness {
    pub branch: String,
    pub base_branch: String,
    pub state: FreshnessState,
    pub commits_behind: usize,
    pub commits_ahead: usize,
    pub last_activity_secs: u64,
    pub missing_fixes_message: String,
}

// ── StaleBranchConfig ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleBranchConfig {
    /// Seconds of inactivity before a branch is considered stale.
    pub stale_threshold_secs: u64,
    /// Commits behind before considered diverged.
    pub diverge_threshold_commits: usize,
    pub policy: StalePolicy,
}

impl Default for StaleBranchConfig {
    fn default() -> Self {
        Self {
            stale_threshold_secs: 7 * 24 * 3600, // 7 days
            diverge_threshold_commits: 20,
            policy: StalePolicy::WarnOnly,
        }
    }
}

// ── FreshnessPolicyDetector ───────────────────────────────────────────────────
//
// Policy-based freshness assessment (commit distance + inactivity time).
// Distinct from `StaleBranchDetector` which is the original age-based
// cleanup classifier.

pub struct FreshnessPolicyDetector {
    pub config: StaleBranchConfig,
}

impl FreshnessPolicyDetector {
    pub fn new(config: StaleBranchConfig) -> Self { Self { config } }

    /// Format a human-readable "missing N fix(es)" message.
    pub fn format_missing_fixes(commits_behind: usize) -> String {
        if commits_behind == 0 {
            "up to date".to_string()
        } else if commits_behind == 1 {
            "missing 1 fix".to_string()
        } else {
            format!("missing {commits_behind} fixes")
        }
    }

    /// Assess branch freshness from metrics (no git subprocess — pure logic).
    pub fn assess(
        &self,
        branch: &str,
        base_branch: &str,
        commits_behind: usize,
        commits_ahead: usize,
        last_activity_secs: u64,
    ) -> BranchFreshness {
        let state = if commits_behind >= self.config.diverge_threshold_commits {
            FreshnessState::Diverged
        } else if last_activity_secs >= self.config.stale_threshold_secs {
            FreshnessState::Stale
        } else {
            FreshnessState::Fresh
        };

        BranchFreshness {
            branch: branch.to_string(),
            base_branch: base_branch.to_string(),
            state,
            commits_behind,
            commits_ahead,
            last_activity_secs,
            missing_fixes_message: Self::format_missing_fixes(commits_behind),
        }
    }

    /// Apply the configured policy to a freshness assessment.
    pub fn apply_policy(&self, _freshness: &BranchFreshness) -> &StalePolicy {
        &self.config.policy
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 2_000 * 24 * 3600 * 1000; // arbitrary "now" — large enough for 999-day tests

    fn ms_days_ago(days: u64, now_ms: u64) -> u64 { now_ms - days * 24 * 3600 * 1000 }

    fn detector() -> StaleBranchDetector { StaleBranchDetector::default() }

    fn branch_age(name: &str, days: u64) -> BranchInfo {
        BranchInfo::new(name, ms_days_ago(days, NOW), "alice")
    }

    #[test]
    fn test_active_recent_branch() {
        let b = branch_age("feat/new", 5);
        let r = detector().classify(&b, NOW);
        assert_eq!(r.label, StalenessLabel::Active);
        assert_eq!(r.action, CleanupAction::Keep);
    }

    #[test]
    fn test_merged_branch_delete() {
        let b = branch_age("feat/old", 30).merged();
        let r = detector().classify(&b, NOW);
        assert_eq!(r.label, StalenessLabel::MergedCleanup);
        assert_eq!(r.action, CleanupAction::Delete);
    }

    #[test]
    fn test_open_pr_keeps_active() {
        let b = branch_age("feat/pr", 90).with_pr();
        let r = detector().classify(&b, NOW);
        assert_eq!(r.label, StalenessLabel::Active);
    }

    #[test]
    fn test_dormant_no_unmerged_delete() {
        let b = branch_age("feat/old2", 20); // 14-60 days, 0 ahead
        let r = detector().classify(&b, NOW);
        assert_eq!(r.label, StalenessLabel::Dormant);
        assert_eq!(r.action, CleanupAction::Delete);
    }

    #[test]
    fn test_dormant_with_unmerged_review() {
        let b = branch_age("feat/wip", 20).ahead(3);
        let r = detector().classify(&b, NOW);
        assert_eq!(r.label, StalenessLabel::Dormant);
        assert_eq!(r.action, CleanupAction::Review);
    }

    #[test]
    fn test_stale_no_unmerged_delete() {
        let b = branch_age("feat/stale", 70); // 60-180 days
        let r = detector().classify(&b, NOW);
        assert_eq!(r.label, StalenessLabel::Stale);
        assert_eq!(r.action, CleanupAction::Delete);
    }

    #[test]
    fn test_stale_with_unmerged_archive() {
        let b = branch_age("feat/stale2", 70).ahead(5);
        let r = detector().classify(&b, NOW);
        assert_eq!(r.label, StalenessLabel::Stale);
        assert_eq!(r.action, CleanupAction::Archive);
    }

    #[test]
    fn test_zombie_delete() {
        let b = branch_age("feat/old3", 200);
        let r = detector().classify(&b, NOW);
        assert_eq!(r.label, StalenessLabel::Zombie);
        assert_eq!(r.action, CleanupAction::Delete);
    }

    #[test]
    fn test_main_always_kept() {
        let b = branch_age("main", 500);
        let r = detector().classify(&b, NOW);
        assert_eq!(r.action, CleanupAction::Keep);
    }

    #[test]
    fn test_master_always_kept() {
        let b = branch_age("master", 999);
        let r = detector().classify(&b, NOW);
        assert_eq!(r.action, CleanupAction::Keep);
    }

    #[test]
    fn test_classify_all_sorted_by_age() {
        let branches = vec![
            branch_age("a", 10),
            branch_age("b", 200),
            branch_age("c", 50),
        ];
        let reports = detector().classify_all(&branches, NOW);
        assert!(reports[0].age_days >= reports[1].age_days);
    }

    #[test]
    fn test_deletion_candidates() {
        let branches = vec![branch_age("a", 200), branch_age("b", 5)];
        let reports = detector().classify_all(&branches, NOW);
        let deletes = detector().deletion_candidates(&reports);
        assert!(deletes.iter().any(|r| r.branch == "a"));
    }

    #[test]
    fn test_branch_age_calculation() {
        let b = BranchInfo::new("x", ms_days_ago(7, NOW), "bob");
        assert_eq!(b.age_days(NOW), 7);
    }

    #[test]
    fn test_staleness_label_display() {
        assert_eq!(StalenessLabel::Zombie.to_string(), "zombie");
        assert_eq!(StalenessLabel::Active.to_string(), "active");
    }

    // ── FreshnessPolicyDetector tests ─────────────────────────────────────────

    fn policy_detector() -> FreshnessPolicyDetector {
        FreshnessPolicyDetector::new(StaleBranchConfig {
            stale_threshold_secs: 7 * 24 * 3600,
            diverge_threshold_commits: 20,
            policy: StalePolicy::WarnOnly,
        })
    }

    #[test]
    fn fresh_when_recently_active_and_not_behind() {
        let d = policy_detector();
        let f = d.assess("feat/x", "main", 0, 3, 3600); // 1 hour ago
        assert_eq!(f.state, FreshnessState::Fresh);
    }

    #[test]
    fn stale_when_inactive_beyond_threshold() {
        let d = policy_detector();
        let thirty_days = 30 * 24 * 3600;
        let f = d.assess("feat/x", "main", 0, 0, thirty_days);
        assert_eq!(f.state, FreshnessState::Stale);
    }

    #[test]
    fn diverged_when_behind_exceeds_threshold() {
        let d = policy_detector();
        let f = d.assess("feat/x", "main", 50, 0, 3600);
        assert_eq!(f.state, FreshnessState::Diverged);
    }

    #[test]
    fn missing_fixes_message_singular() {
        assert_eq!(FreshnessPolicyDetector::format_missing_fixes(1), "missing 1 fix");
    }

    #[test]
    fn missing_fixes_message_plural() {
        let msg = FreshnessPolicyDetector::format_missing_fixes(5);
        assert!(msg.contains("5"));
        assert!(msg.contains("fixes"));
    }

    #[test]
    fn missing_fixes_message_zero_is_up_to_date() {
        assert_eq!(FreshnessPolicyDetector::format_missing_fixes(0), "up to date");
    }

    #[test]
    fn apply_policy_returns_configured_policy() {
        let d = FreshnessPolicyDetector::new(StaleBranchConfig {
            policy: StalePolicy::AutoRebase,
            ..Default::default()
        });
        let f = d.assess("b", "main", 0, 0, 0);
        assert_eq!(d.apply_policy(&f), &StalePolicy::AutoRebase);
    }
}
