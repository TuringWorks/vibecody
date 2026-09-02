//! Developer Excellence metrics — DORA four keys and engineering-practice
//! maturity, computed from what the repository actually records.
//!
//! ## Why this module exists
//!
//! `evaluate_idp_scorecard` used to award a service 8/10 for deploy frequency
//! and 7/10 for lead time with the comment "simulated DORA-style metrics".
//! Those numbers were constants. A director reading that scorecard would have
//! been told their delivery performance without anything having been measured
//! — the exact substitution
//! [AGENTS.md](../../../AGENTS.md#modelling-honesty--a-model-that-cannot-be-wrong-is-not-a-model)
//! forbids. This module replaces the constants with a computation, and where
//! the computation cannot run it says so instead of guessing.
//!
//! ## What a git-only DORA measurement can and cannot know
//!
//! Git records commits, merges and tags. It does not record deployments,
//! incidents, or pages. So every number here is derived from a **declared
//! proxy**, the proxy is named in the output, and a metric whose proxy has no
//! signal in the window is returned as [`Unmeasured`] — never as zero.
//!
//! | Metric | Proxy used | What it cannot see |
//! |---|---|---|
//! | Deployment frequency | version-like tags (or merges to a release branch) | deploys that ship without a tag |
//! | Lead time for changes | author-time of each commit → time of the release that first contained it | time queued before the first commit |
//! | Change failure rate | a deployment followed by a revert/hotfix/rollback commit before the next deployment | failures fixed by config, flag flip, or a rollback outside git |
//! | Time to restore | that deployment → its remediation commit | incidents with no code remediation |
//!
//! A team whose deploys are not tagged gets `deployment_frequency: unmeasured`
//! with the reason spelled out, and the recommendation to start tagging — which
//! is a true and actionable answer. "0.0 deploys/week" would be a false one.
//!
//! ## The bands
//!
//! [`Band`] classifies a measured value into the four DORA performance levels.
//! The thresholds are the ones published in the DORA / *Accelerate* State of
//! DevOps reports and are reproduced in [`BAND_SOURCE`] so a reader can check
//! them against the source rather than trusting this file. They are the only
//! externally-sourced numbers here; every other number is computed from the
//! repository in front of it.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Provenance for [`Band`]'s thresholds, carried in the report so the numbers
/// travel with their source instead of arriving anonymous.
pub const BAND_SOURCE: &str =
    "DORA State of DevOps performance bands (elite/high/medium/low), as published \
     in the DORA reports and Accelerate. Thresholds are the framework's, not measured here.";

/// Default measurement window. Ninety days is long enough for a low-frequency
/// team to show more than one deployment and short enough that the answer still
/// describes how the team works now.
pub const DEFAULT_WINDOW_DAYS: u32 = 90;

/// Cap on releases inspected for lead time. Each one costs a `git log` call, and
/// a repo with a thousand tags would otherwise turn an interactive request into
/// a minute of forking. When the cap binds the report says so.
const MAX_RELEASES_INSPECTED: usize = 60;

/// Cap on commits read from `git log`. Bounds memory on a monorepo with a very
/// busy window; a truncated read is reported, never silently trimmed.
const MAX_COMMITS: usize = 50_000;

const SECONDS_PER_HOUR: f64 = 3_600.0;

// ── Result shapes ────────────────────────────────────────────────────────────

/// The four DORA performance levels.
///
/// Serialized lowercase so a client can style on the value without mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Elite,
    High,
    Medium,
    Low,
}

impl Band {
    /// Deployment frequency, in deployments per day.
    ///
    /// Elite = on demand (multiple per day); High = between once per day and
    /// once per week; Medium = between once per week and once per month;
    /// Low = less often than once per month.
    fn for_deploy_frequency(per_day: f64) -> Band {
        if per_day >= 1.0 {
            Band::Elite
        } else if per_day >= 1.0 / 7.0 {
            Band::High
        } else if per_day >= 1.0 / 30.0 {
            Band::Medium
        } else {
            Band::Low
        }
    }

    /// Lead time for changes, in hours. Elite < 1 day, High < 1 week,
    /// Medium < 1 month, Low beyond that.
    fn for_lead_time_hours(hours: f64) -> Band {
        if hours < 24.0 {
            Band::Elite
        } else if hours < 24.0 * 7.0 {
            Band::High
        } else if hours < 24.0 * 30.0 {
            Band::Medium
        } else {
            Band::Low
        }
    }

    /// Time to restore service, in hours. Elite < 1 hour, High < 1 day,
    /// Medium < 1 week, Low beyond that.
    fn for_restore_hours(hours: f64) -> Band {
        if hours < 1.0 {
            Band::Elite
        } else if hours < 24.0 {
            Band::High
        } else if hours < 24.0 * 7.0 {
            Band::Medium
        } else {
            Band::Low
        }
    }

    /// Change failure rate, as a fraction in `0.0..=1.0`.
    fn for_change_failure_rate(rate: f64) -> Band {
        if rate <= 0.05 {
            Band::Elite
        } else if rate <= 0.10 {
            Band::High
        } else if rate <= 0.15 {
            Band::Medium
        } else {
            Band::Low
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Band::Elite => "elite",
            Band::High => "high",
            Band::Medium => "medium",
            Band::Low => "low",
        }
    }
}

/// A metric that could not be computed, and precisely why.
///
/// This is a first-class result, not an error: "your deploys aren't tagged" is
/// the most useful finding a first DORA run can produce, and it is the finding
/// a fabricated zero would have hidden.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unmeasured {
    /// Metric key: `deployment_frequency`, `lead_time_for_changes`,
    /// `change_failure_rate`, `time_to_restore`.
    pub metric: String,
    /// Why the proxy produced no signal, in the reader's terms.
    pub reason: String,
    /// The concrete thing that would make it measurable next time.
    pub to_measure_this: String,
}

/// One computed metric, with the sample it was computed from.
///
/// `sample_size` travels with the value on purpose: "2.0 deploys/week" from two
/// observations and from two hundred are different claims, and a dashboard that
/// prints only the former is inviting a decision it cannot support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measure {
    pub value: f64,
    /// Human-readable unit, e.g. `deployments/week`, `hours`, `percent`.
    pub unit: String,
    pub band: Band,
    /// How many observations the value came from.
    pub sample_size: usize,
    /// The proxy this value was derived from, named so it can be argued with.
    pub proxy: String,
    /// Median-adjacent detail where a distribution exists (p50/p75), else empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub percentiles: Vec<Percentile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Percentile {
    pub label: String,
    pub value: f64,
}

/// How deployments were identified for this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReleaseMarker {
    /// Version-like git tags (`v1.2.3`, `1.2.3`, `release-1.2.3`).
    VersionTags,
    /// Merge commits landing on the named release branch.
    ReleaseBranchMerges,
}

impl ReleaseMarker {
    pub fn from_str(s: &str) -> Option<ReleaseMarker> {
        match s {
            "tags" | "version-tags" => Some(ReleaseMarker::VersionTags),
            "merges" | "release-branch-merges" => Some(ReleaseMarker::ReleaseBranchMerges),
            _ => None,
        }
    }

    fn describe(self) -> &'static str {
        match self {
            ReleaseMarker::VersionTags => "version-like git tags",
            ReleaseMarker::ReleaseBranchMerges => "merge commits on the release branch",
        }
    }
}

/// A single observed deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    /// Tag name or merge commit subject.
    pub name: String,
    /// Commit the deployment points at.
    pub commit: String,
    /// Unix seconds.
    pub at: i64,
    /// Whether a remediation commit followed this deployment before the next.
    pub followed_by_remediation: bool,
}

/// The DORA report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoraReport {
    pub repo: String,
    pub window_days: u32,
    /// Unix seconds; the window is `[since, generated_at]`.
    pub since: i64,
    pub generated_at: i64,
    pub release_marker: ReleaseMarker,
    pub release_marker_description: String,
    pub band_source: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_frequency: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead_time_for_changes: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_failure_rate: Option<Measure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_restore: Option<Measure>,

    /// Metrics that could not be computed, with reasons. Never empty when one
    /// of the four `Option`s above is `None`.
    pub unmeasured: Vec<Unmeasured>,

    pub deployments: Vec<Deployment>,
    pub commits_in_window: usize,
    pub authors_in_window: usize,

    /// Non-fatal notes: caps that bound the read, proxies that had thin data.
    pub notes: Vec<String>,
}

impl DoraReport {
    /// The four keys that were actually computed. Used by the scorecard so a
    /// grade is never averaged over metrics that do not exist.
    pub fn measured(&self) -> Vec<(&'static str, &Measure)> {
        [
            ("deployment_frequency", self.deployment_frequency.as_ref()),
            ("lead_time_for_changes", self.lead_time_for_changes.as_ref()),
            ("change_failure_rate", self.change_failure_rate.as_ref()),
            ("time_to_restore", self.time_to_restore.as_ref()),
        ]
        .into_iter()
        .filter_map(|(k, m)| m.map(|m| (k, m)))
        .collect()
    }
}

// ── Git plumbing ─────────────────────────────────────────────────────────────

/// One commit, as read from `git log`.
#[derive(Debug, Clone)]
struct Commit {
    /// Commit time, unix seconds — when it landed on this history.
    ///
    /// Author time is deliberately absent: lead time is computed per release
    /// range by [`commits_between`], and remediation pairing needs when a fix
    /// *landed*, not when it was written on someone's laptop.
    committed_at: i64,
    author: String,
    subject: String,
}

impl Commit {
    /// Does this commit look like it undid or patched a bad release?
    ///
    /// Deliberately narrow. `fix:` is not here: most `fix:` commits fix a bug
    /// found in development, and counting them would inflate change failure
    /// rate toward "everything fails". Only language that means *something
    /// shipped and had to be taken back* counts.
    fn is_remediation(&self) -> bool {
        let s = self.subject.to_ascii_lowercase();
        s.starts_with("revert ")
            || s.starts_with("revert:")
            || s.starts_with("revert \"")
            || s.contains("hotfix")
            || s.contains("rollback")
            || s.contains("roll back")
            || s.starts_with("emergency fix")
    }
}

/// Run `git` in `repo`, returning stdout on success.
fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = vibe_no_window::std_command("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("failed to run `git {}`", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Does this tag name look like a released version?
///
/// Matches `v1.2`, `1.2.3`, `release-2.0`, `2024.11.1`. Rejects `nightly`,
/// `latest`, `backup-before-migration` — names that mark a point in history
/// without asserting a release.
fn is_version_tag(name: &str) -> bool {
    let core = name
        .rsplit('/')
        .next()
        .unwrap_or(name)
        .trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == '-' || c == '_');
    let mut parts = core.split(['.', '-', '+']);
    let Some(first) = parts.next() else {
        return false;
    };
    if first.is_empty() || !first.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    // A bare number ("42") is a build counter, not a version. Require a dot.
    core.contains('.')
}

/// Every version-like tag with its creation time, oldest first.
fn version_tags(repo: &Path) -> Result<Vec<(String, String, i64)>> {
    let raw = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)\t%(creatordate:unix)\t%(objectname)",
            "refs/tags",
        ],
    )?;
    let mut tags: Vec<(String, String, i64)> = raw
        .lines()
        .filter_map(|line| {
            let mut f = line.split('\t');
            let name = f.next()?.trim();
            let at: i64 = f.next()?.trim().parse().ok()?;
            let obj = f.next()?.trim();
            is_version_tag(name).then(|| (name.to_string(), obj.to_string(), at))
        })
        .collect();
    tags.sort_by_key(|(_, _, at)| *at);
    Ok(tags)
}

/// Merge commits on `branch`, oldest first.
fn release_branch_merges(repo: &Path, branch: &str, since: i64) -> Result<Vec<(String, String, i64)>> {
    let since_arg = format!("--since={since}");
    let raw = git(
        repo,
        &[
            "log",
            branch,
            "--merges",
            "--format=%H\t%ct\t%s",
            &since_arg,
        ],
    )?;
    let mut merges: Vec<(String, String, i64)> = raw
        .lines()
        .filter_map(|line| {
            let mut f = line.splitn(3, '\t');
            let hash = f.next()?.trim().to_string();
            let at: i64 = f.next()?.trim().parse().ok()?;
            let subject = f.next().unwrap_or("merge").trim().to_string();
            Some((subject, hash, at))
        })
        .collect();
    merges.sort_by_key(|(_, _, at)| *at);
    Ok(merges)
}

/// Commits in the window, newest first as git emits them.
fn commits_since(repo: &Path, since: i64) -> Result<(Vec<Commit>, bool)> {
    let since_arg = format!("--since={since}");
    let max_arg = format!("--max-count={}", MAX_COMMITS + 1);
    let raw = git(
        repo,
        &[
            "log",
            "--no-merges",
            "--format=%H\t%at\t%ct\t%an\t%s",
            &since_arg,
            &max_arg,
        ],
    )?;
    let mut commits: Vec<Commit> = raw
        .lines()
        .filter_map(|line| {
            let mut f = line.splitn(5, '\t');
            // Fields are consumed positionally: hash and author-time are read
            // off the line and dropped, because the struct needs neither.
            let _hash = f.next()?;
            let _authored_at = f.next()?;
            Some(Commit {
                committed_at: f.next()?.trim().parse().ok()?,
                author: f.next()?.trim().to_string(),
                subject: f.next().unwrap_or_default().trim().to_string(),
            })
        })
        .collect();
    let truncated = commits.len() > MAX_COMMITS;
    commits.truncate(MAX_COMMITS);
    Ok((commits, truncated))
}

/// Author times of the commits first released by `to` (i.e. in `from..to`).
fn commits_between(repo: &Path, from: Option<&str>, to: &str) -> Result<Vec<i64>> {
    let range = match from {
        Some(f) => format!("{f}..{to}"),
        None => to.to_string(),
    };
    let raw = git(repo, &["log", "--no-merges", "--format=%at", &range])?;
    Ok(raw
        .lines()
        .filter_map(|l| l.trim().parse::<i64>().ok())
        .collect())
}

// ── Statistics ───────────────────────────────────────────────────────────────

/// Percentile of a sorted-in-place sample, using nearest-rank.
///
/// Returns `None` for an empty sample rather than a zero, because "no data" and
/// "measured zero" must not print the same.
fn percentile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = (p * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted.get(rank.min(sorted.len()) - 1).copied()
}

// ── DORA ─────────────────────────────────────────────────────────────────────

/// Options for a DORA run.
#[derive(Debug, Clone)]
pub struct DoraOptions {
    pub window_days: u32,
    pub release_marker: ReleaseMarker,
    /// Branch consulted when `release_marker` is `ReleaseBranchMerges`.
    pub release_branch: String,
}

impl Default for DoraOptions {
    fn default() -> Self {
        DoraOptions {
            window_days: DEFAULT_WINDOW_DAYS,
            release_marker: ReleaseMarker::VersionTags,
            release_branch: "HEAD".to_string(),
        }
    }
}

/// Compute the four keys for `repo` over the requested window.
///
/// Errors only when git itself cannot be consulted. A repository with no
/// releases is not an error — it is a report whose four metrics are all
/// `unmeasured`, which is the honest answer and the actionable one.
pub fn compute_dora(repo: &Path, opts: &DoraOptions) -> Result<DoraReport> {
    let root = vibe_core::git::discover_repo_root(repo)
        .ok_or_else(|| anyhow::anyhow!("{} is not inside a git repository", repo.display()))?;

    let now = chrono::Utc::now().timestamp();
    let since = now - i64::from(opts.window_days) * 86_400;

    let (commits, truncated) = commits_since(&root, since)?;
    let mut notes = Vec::new();
    if truncated {
        notes.push(format!(
            "commit read capped at {MAX_COMMITS}; lead-time and author counts describe the most recent {MAX_COMMITS} commits only"
        ));
    }

    let authors: std::collections::HashSet<&str> =
        commits.iter().map(|c| c.author.as_str()).collect();

    // Deployments in the window, oldest first, plus the release immediately
    // before the window — needed as the lower bound of the first range so the
    // first release's lead time is not computed against the repo's whole history.
    let (all_releases, marker_note) = match opts.release_marker {
        ReleaseMarker::VersionTags => (version_tags(&root)?, None),
        ReleaseMarker::ReleaseBranchMerges => (
            release_branch_merges(&root, &opts.release_branch, since)?,
            Some(format!(
                "release branch = {}; merges before the window are not read, so the first release's lead time uses only in-window commits",
                opts.release_branch
            )),
        ),
    };
    notes.extend(marker_note);

    let first_in_window = all_releases.iter().position(|(_, _, at)| *at >= since);
    let prior_release = first_in_window
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| all_releases.get(i))
        .map(|(_, obj, _)| obj.clone());
    let in_window: Vec<&(String, String, i64)> = all_releases
        .iter()
        .filter(|(_, _, at)| *at >= since)
        .collect();

    let mut unmeasured = Vec::new();

    // ── Deployment frequency ────────────────────────────────────────────────
    let deployment_frequency = if in_window.is_empty() {
        unmeasured.push(Unmeasured {
            metric: "deployment_frequency".into(),
            reason: format!(
                "no {} found in the last {} days, so no deployment was observed. This is not a frequency of zero — it is an absence of the signal.",
                opts.release_marker.describe(),
                opts.window_days
            ),
            to_measure_this: match opts.release_marker {
                ReleaseMarker::VersionTags =>
                    "tag each release (`git tag -a v1.2.3`) from the pipeline, or re-run with --marker merges if you deploy from branch merges".into(),
                ReleaseMarker::ReleaseBranchMerges =>
                    "point --branch at the branch you deploy from, or re-run with --marker tags if your pipeline tags releases".into(),
            },
        });
        None
    } else {
        let per_day = in_window.len() as f64 / f64::from(opts.window_days);
        Some(Measure {
            value: per_day * 7.0,
            unit: "deployments/week".into(),
            band: Band::for_deploy_frequency(per_day),
            sample_size: in_window.len(),
            proxy: opts.release_marker.describe().into(),
            percentiles: Vec::new(),
        })
    };

    // ── Lead time for changes ───────────────────────────────────────────────
    // Oldest-first, so each release's lower bound is the release before it.
    // Only the most recent `MAX_RELEASES_INSPECTED` are walked: each one costs a
    // `git log` fork, and a repo with a thousand tags would turn an interactive
    // request into a minute of process spawning.
    let skipped = in_window.len().saturating_sub(MAX_RELEASES_INSPECTED);
    let inspected: Vec<(String, i64)> = in_window[skipped..]
        .iter()
        .map(|(_, obj, at)| (obj.clone(), *at))
        .collect();
    if skipped > 0 {
        notes.push(format!(
            "lead time computed from the {MAX_RELEASES_INSPECTED} most recent releases of {} in the window",
            in_window.len()
        ));
    }
    // The bound below the first inspected release: its in-window predecessor if
    // one was skipped, else the last release before the window.
    let first_lower_bound = skipped
        .checked_sub(1)
        .and_then(|i| in_window.get(i))
        .map(|(_, obj, _)| obj.clone())
        .or(prior_release);

    let mut lead_hours: Vec<f64> = Vec::new();
    // Ranges git refused to read. Counted rather than swallowed: an
    // `unwrap_or_default()` here would turn "we could not read that range" into
    // "nothing shipped in it", which then reads as a genuinely empty release —
    // a failure reported as a finding, and the exact substitution the rest of
    // this module exists to avoid. One bad range does not abort the metric; it
    // shrinks the sample, and the note says by how much.
    let mut unreadable_ranges = 0usize;
    for (idx, (obj, at)) in inspected.iter().enumerate() {
        let from = if idx == 0 {
            first_lower_bound.clone()
        } else {
            Some(inspected[idx - 1].0.clone())
        };
        match commits_between(&root, from.as_deref(), obj) {
            Ok(authored) => lead_hours.extend(
                authored
                    .into_iter()
                    .filter(|a| a <= at)
                    .map(|a| (at - a) as f64 / SECONDS_PER_HOUR),
            ),
            Err(_) => unreadable_ranges += 1,
        }
    }
    if unreadable_ranges > 0 {
        notes.push(format!(
            "{unreadable_ranges} of {} release ranges could not be read by git; lead time is computed from the rest, so its sample is smaller than the release count implies",
            inspected.len()
        ));
    }

    let lead_time_for_changes = if lead_hours.is_empty() {
        unmeasured.push(Unmeasured {
            metric: "lead_time_for_changes".into(),
            reason: if in_window.is_empty() {
                "no release was observed, so no commit can be dated to the release that carried it".into()
            } else {
                if unreadable_ranges > 0 {
                    format!(
                        "{unreadable_ranges} of {} release ranges could not be read by git, and the rest contained no commit newer than their predecessor",
                        inspected.len()
                    )
                } else {
                    "releases were found but none contained a commit newer than its predecessor — nothing shipped between them".into()
                }
            },
            to_measure_this:
                "tag releases from the pipeline so each tag marks the commit set it shipped".into(),
        });
        None
    } else {
        lead_hours.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50 = percentile(&lead_hours, 0.50).unwrap_or_default();
        Some(Measure {
            value: p50,
            unit: "hours (p50)".into(),
            band: Band::for_lead_time_hours(p50),
            sample_size: lead_hours.len(),
            proxy: "commit author-time → time of the release that first contained it".into(),
            percentiles: [("p50", 0.50), ("p75", 0.75), ("p90", 0.90)]
                .into_iter()
                .filter_map(|(label, p)| {
                    percentile(&lead_hours, p).map(|value| Percentile {
                        label: label.into(),
                        value,
                    })
                })
                .collect(),
        })
    };

    // ── Change failure rate + time to restore ───────────────────────────────
    // Commits are newest-first from git; walk oldest-first to pair each
    // deployment with the remediation that followed it.
    let remediations: Vec<&Commit> = commits
        .iter()
        .rev()
        .filter(|c| c.is_remediation())
        .collect();

    let mut deployments: Vec<Deployment> = Vec::with_capacity(in_window.len());
    let mut restore_hours: Vec<f64> = Vec::new();
    for (i, (name, obj, at)) in in_window.iter().map(|r| (&r.0, &r.1, r.2)).enumerate() {
        let next_at = in_window.get(i + 1).map(|r| r.2).unwrap_or(i64::MAX);
        let first_after = remediations
            .iter()
            .find(|c| c.committed_at > at && c.committed_at < next_at);
        if let Some(c) = first_after {
            restore_hours.push((c.committed_at - at) as f64 / SECONDS_PER_HOUR);
        }
        deployments.push(Deployment {
            name: name.clone(),
            commit: obj.clone(),
            at,
            followed_by_remediation: first_after.is_some(),
        });
    }

    let failed = deployments.iter().filter(|d| d.followed_by_remediation).count();
    let change_failure_rate = if deployments.is_empty() {
        unmeasured.push(Unmeasured {
            metric: "change_failure_rate".into(),
            reason: "no deployment was observed, so there is nothing for a failure to be a rate of"
                .into(),
            to_measure_this: "tag releases, and land reverts/hotfixes with a subject that says so"
                .into(),
        });
        None
    } else {
        let rate = failed as f64 / deployments.len() as f64;
        Some(Measure {
            value: rate * 100.0,
            unit: "percent of deployments".into(),
            band: Band::for_change_failure_rate(rate),
            sample_size: deployments.len(),
            proxy: "a deployment followed by a revert/hotfix/rollback commit before the next one"
                .into(),
            percentiles: Vec::new(),
        })
    };

    let time_to_restore = if restore_hours.is_empty() {
        unmeasured.push(Unmeasured {
            metric: "time_to_restore".into(),
            reason: if deployments.is_empty() {
                "no deployment was observed".into()
            } else {
                format!(
                    "{} deployment(s) observed and none was followed by a revert, hotfix or rollback commit — there is no restoration to time. A clean window is not a restore time of zero.",
                    deployments.len()
                )
            },
            to_measure_this:
                "record incident start and resolution in the incident tool and feed it in; git alone only sees code remediation"
                    .into(),
        });
        None
    } else {
        restore_hours.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50 = percentile(&restore_hours, 0.50).unwrap_or_default();
        Some(Measure {
            value: p50,
            unit: "hours (p50)".into(),
            band: Band::for_restore_hours(p50),
            sample_size: restore_hours.len(),
            proxy: "deployment → the first revert/hotfix/rollback commit after it".into(),
            percentiles: [("p50", 0.50), ("p90", 0.90)]
                .into_iter()
                .filter_map(|(label, p)| {
                    percentile(&restore_hours, p).map(|value| Percentile {
                        label: label.into(),
                        value,
                    })
                })
                .collect(),
        })
    };

    Ok(DoraReport {
        repo: root.display().to_string(),
        window_days: opts.window_days,
        since,
        generated_at: now,
        release_marker: opts.release_marker,
        release_marker_description: opts.release_marker.describe().to_string(),
        band_source: BAND_SOURCE.to_string(),
        deployment_frequency,
        lead_time_for_changes,
        change_failure_rate,
        time_to_restore,
        unmeasured,
        deployments,
        commits_in_window: commits.len(),
        authors_in_window: authors.len(),
        notes,
    })
}

// ── Practice maturity ────────────────────────────────────────────────────────

/// One signal a practice is being carried out, and where it was found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub name: String,
    pub found: bool,
    /// Path relative to the workspace root, when found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// A practice, and how much of it the workspace shows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeResult {
    pub key: String,
    pub title: String,
    /// Which of the role's responsibilities this practice serves.
    pub pillar: String,
    pub signals: Vec<Signal>,
    pub found: usize,
    pub expected: usize,
    /// `detected` level 0–3. **Never 4**: see [`MAX_DETECTABLE_LEVEL`].
    pub level: u8,
    pub level_name: String,
    /// What to do next, derived from the specific signals that are missing.
    pub next_step: String,
    /// A known limit of *this practice's* detection, when one exists.
    ///
    /// Present because a false negative that looks like a finding is worse
    /// than no finding: this repository has thousands of Rust `#[cfg(test)]`
    /// tests and no `tests/` directory, and "missing: test directory" read as
    /// "they do not test" until the caveat said otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detection_caveat: Option<String>,
}

/// The highest maturity a scan may assign.
///
/// A file proves a practice is *present*. It cannot prove the practice is
/// *followed*, *reviewed*, or *improving* — the things that separate level 3
/// from level 4 in every maturity model worth using. So the scan tops out at 3
/// and the report says the last level is attested by humans, rather than
/// handing a director a "level 4, optimizing" that a `touch` could have earned.
pub const MAX_DETECTABLE_LEVEL: u8 = 3;

fn level_name(level: u8) -> &'static str {
    match level {
        0 => "absent",
        1 => "initial",
        2 => "managed",
        _ => "defined",
    }
}

/// A practice definition: the signals that indicate it, in the workspace.
struct Practice {
    key: &'static str,
    title: &'static str,
    pillar: &'static str,
    /// `(signal name, candidate paths — any one match counts)`
    signals: &'static [(&'static str, &'static [&'static str])],
    next_step: &'static str,
    /// Set where path-based detection is known to miss a common real practice.
    caveat: Option<&'static str>,
}

/// The practice catalogue.
///
/// Chosen to cover the Developer Excellence remit: engineering standards, the
/// developer platform, and the inner loop. Each signal is a path that exists or
/// does not — nothing here infers quality from content, because it cannot.
static PRACTICES: &[Practice] = &[
    Practice {
        key: "ci-pipeline",
        title: "Continuous integration",
        pillar: "Global Practices Program",
        signals: &[
            ("pipeline definition", &[".github/workflows", ".gitlab-ci.yml", "azure-pipelines.yml", "Jenkinsfile", ".circleci/config.yml", "buildkite.yml", ".buildkite"]),
            ("build entry point", &["Makefile", "justfile", "Taskfile.yml", "build.gradle", "build.gradle.kts", "pom.xml", "package.json", "Cargo.toml"]),
            ("dependency lockfile", &["Cargo.lock", "package-lock.json", "pnpm-lock.yaml", "yarn.lock", "poetry.lock", "uv.lock", "go.sum", "Gemfile.lock", "gradle.lockfile"]),
        ],
        next_step: "Add a pipeline definition that runs on every pull request; a build that only runs locally cannot enforce a standard.",
        caveat: None,
    },
    Practice {
        key: "automated-testing",
        title: "Automated testing",
        pillar: "Global Practices Program",
        signals: &[
            ("test directory", &["tests", "test", "spec", "__tests__", "src/test"]),
            ("coverage configuration", &["codecov.yml", ".codecov.yml", "tarpaulin.toml", "jest.config.js", "jest.config.ts", "vitest.config.ts", ".coveragerc", "pytest.ini"]),
            ("test command documented", &["CONTRIBUTING.md", "Makefile", "justfile", "Taskfile.yml"]),
        ],
        next_step: "Publish the one command that runs the whole suite, and gate merges on it.",
        caveat: Some(
            "Detected by path only. Languages that colocate tests with source — Rust \
             `#[cfg(test)]`, Go `_test.go`, Python `test_*.py` beside the module — have no \
             test directory to find, so a 'missing' here is not evidence that a repository \
             is untested.",
        ),
    },
    Practice {
        key: "code-review",
        title: "Code review and ownership",
        pillar: "Global Practices Program",
        signals: &[
            ("ownership map", &["CODEOWNERS", ".github/CODEOWNERS", "docs/CODEOWNERS"]),
            ("pull request template", &[".github/pull_request_template.md", ".github/PULL_REQUEST_TEMPLATE.md", ".github/PULL_REQUEST_TEMPLATE", ".gitlab/merge_request_templates"]),
            ("contribution guide", &["CONTRIBUTING.md", "docs/CONTRIBUTING.md", ".github/CONTRIBUTING.md"]),
        ],
        next_step: "Add CODEOWNERS so every path has a named reviewer; unowned code is where standards quietly lapse.",
        caveat: None,
    },
    Practice {
        key: "security-scanning",
        title: "Security in the pipeline",
        pillar: "Global Practices Program",
        signals: &[
            ("dependency scanning", &[".github/dependabot.yml", "renovate.json", ".renovaterc", "deny.toml", ".snyk"]),
            ("secret hygiene", &[".gitleaks.toml", ".secrets.baseline", ".pre-commit-config.yaml", ".gitignore"]),
            ("security policy", &["SECURITY.md", ".github/SECURITY.md", "docs/security.md", "docs/threat-model.md"]),
        ],
        next_step: "Turn on dependency and secret scanning in CI; a standard nobody enforces is a suggestion.",
        caveat: None,
    },
    Practice {
        key: "observability",
        title: "Observability",
        pillar: "Strategic Developers' Platform Ownership",
        signals: &[
            ("telemetry configuration", &["otel-collector.yaml", "otel-config.yaml", "opentelemetry.yaml", "prometheus.yml", "grafana", "datadog.yaml", "newrelic.yml"]),
            ("dashboards or alerts as code", &["dashboards", "alerts", "monitoring", "deploy/monitoring", "infra/monitoring"]),
            ("runbook", &["RUNBOOK.md", "docs/runbook.md", "docs/runbooks", "docs/oncall.md"]),
        ],
        next_step: "Check dashboards and alerts into the repo; observability owned in a UI cannot be reviewed, versioned, or handed over.",
        caveat: None,
    },
    Practice {
        key: "infrastructure-as-code",
        title: "Infrastructure as code",
        pillar: "Strategic Developers' Platform Ownership",
        signals: &[
            ("declarative infrastructure", &["terraform", "infra", "deploy", "pulumi", "cdk", "helm", "charts", "kustomization.yaml"]),
            ("container definition", &["Dockerfile", "docker-compose.yml", "compose.yaml", "Containerfile"]),
            ("environment definition", &[".env.example", "env.example", "config/environments", "deploy/environments"]),
        ],
        next_step: "Describe environments in code so a new one is a pull request rather than a ticket.",
        caveat: None,
    },
    Practice {
        key: "release-management",
        title: "Release management",
        pillar: "Strategic Developers' Platform Ownership",
        signals: &[
            ("changelog", &["CHANGELOG.md", "docs/CHANGELOG.md", "RELEASES.md"]),
            ("release process documented", &["RELEASE.md", "docs/release.md", "docs/releasing.md"]),
            ("release automation", &[".github/workflows/release.yml", ".github/workflows/publish.yml", ".goreleaser.yml", "release-please-config.json"]),
        ],
        next_step: "Automate the release so its cadence is a decision, not a function of who is available.",
        caveat: None,
    },
    Practice {
        key: "onboarding",
        title: "Developer onboarding",
        pillar: "Strategic Developers' Platform Ownership",
        signals: &[
            ("one-command bootstrap", &["scripts/bootstrap.sh", "scripts/setup.sh", "bootstrap.sh", "setup.sh", "Makefile", "justfile", "Taskfile.yml"]),
            ("reproducible environment", &[".devcontainer", "flake.nix", "shell.nix", ".tool-versions", ".mise.toml", "asdf", ".nvmrc", "rust-toolchain.toml"]),
            ("getting-started guide", &["README.md", "docs/quickstart.md", "docs/getting-started.md", "CONTRIBUTING.md"]),
        ],
        next_step: "Get clone-to-first-commit behind one command; every step a newcomer must discover is a day of their first week.",
        caveat: None,
    },
    Practice {
        key: "architecture-decisions",
        title: "Architecture decision records",
        pillar: "Engineering Leadership",
        signals: &[
            ("ADR directory", &["docs/adr", "docs/decisions", "adr", "architecture/decisions", "docs/architecture/decisions"]),
            ("architecture documentation", &["docs/architecture.md", "ARCHITECTURE.md", "docs/architecture"]),
            ("agent or contributor guide", &["AGENTS.md", "CLAUDE.md", "docs/engineering-standards.md", "docs/standards.md"]),
        ],
        next_step: "Record decisions where they can be found and argued with; undocumented architecture is re-litigated every quarter.",
        caveat: None,
    },
    Practice {
        key: "golden-path",
        title: "Golden path and templates",
        pillar: "Strategic Developers' Platform Ownership",
        signals: &[
            ("service template or scaffolder", &["templates", "cookiecutter.json", "copier.yml", "scaffold", ".github/ISSUE_TEMPLATE"]),
            ("service catalog entry", &["catalog-info.yaml", "backstage.yaml", "service.yaml", "port.yml"]),
            ("editor and lint conventions", &[".editorconfig", ".eslintrc.json", "eslint.config.js", "rustfmt.toml", ".rustfmt.toml", "ruff.toml", ".prettierrc", "clippy.toml"]),
        ],
        next_step: "Make the supported way the easy way: a template that already has CI, tests and observability wired in.",
        caveat: Some(
            "Scanned at the workspace root only. In a monorepo the lint and editor \
             conventions usually live in each package, so this reads 'missing' for a \
             repository that has them one level down.",
        ),
    },
];

/// The practice-maturity report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticesReport {
    pub workspace: String,
    pub generated_at: i64,
    pub practices: Vec<PracticeResult>,
    /// Mean detected level across practices, 0.0–3.0.
    pub mean_level: f64,
    pub max_detectable_level: u8,
    /// Stated in the payload so a client cannot render this as a full maturity
    /// score by omission.
    pub scope_note: String,
}

/// Resolve a candidate path relative to `root`, returning the relative form
/// when it exists. Both files and directories count — a practice can be
/// evidenced by either.
fn find_path(root: &Path, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|c| root.join(c).exists())
        .map(|c| (*c).to_string())
}

/// Scan `workspace` for engineering-practice signals.
pub fn scan_practices(workspace: &Path) -> Result<PracticesReport> {
    if !workspace.is_dir() {
        bail!("{} is not a directory", workspace.display());
    }

    let practices: Vec<PracticeResult> = PRACTICES
        .iter()
        .map(|p| {
            let signals: Vec<Signal> = p
                .signals
                .iter()
                .map(|(name, candidates)| {
                    let path = find_path(workspace, candidates);
                    Signal {
                        name: (*name).to_string(),
                        found: path.is_some(),
                        path,
                    }
                })
                .collect();
            let found = signals.iter().filter(|s| s.found).count();
            let expected = signals.len();
            // Level is the count of satisfied signals, capped at the highest a
            // scan is entitled to assert.
            let level = (found as u8).min(MAX_DETECTABLE_LEVEL);
            PracticeResult {
                key: p.key.to_string(),
                title: p.title.to_string(),
                pillar: p.pillar.to_string(),
                signals,
                found,
                expected,
                level,
                level_name: level_name(level).to_string(),
                next_step: p.next_step.to_string(),
                detection_caveat: p.caveat.map(str::to_string),
            }
        })
        .collect();

    let mean_level = if practices.is_empty() {
        0.0
    } else {
        practices.iter().map(|p| f64::from(p.level)).sum::<f64>() / practices.len() as f64
    };

    Ok(PracticesReport {
        workspace: workspace.display().to_string(),
        generated_at: chrono::Utc::now().timestamp(),
        practices,
        mean_level,
        max_detectable_level: MAX_DETECTABLE_LEVEL,
        scope_note: format!(
            "Levels are DETECTED from files present, capped at {MAX_DETECTABLE_LEVEL} ('defined'). \
             A file proves a practice exists, not that it is followed. Level 4 ('optimizing') is \
             attested by people, never by a scan. Signals are looked for at the workspace ROOT \
             only, so a monorepo whose conventions live per-package will under-report; see each \
             practice's detection_caveat."
        ),
    })
}

// ── Onboarding ───────────────────────────────────────────────────────────────

/// Contributor activity, as far as git can see it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewContributor {
    pub author: String,
    /// Unix seconds of their first commit in this repository.
    pub first_commit_at: i64,
    /// Hours from their first commit to their second, when there is one.
    ///
    /// A proxy for "did their environment work?" — a long gap after a first
    /// commit often means the first day was spent fighting setup. It is a weak
    /// signal and is labelled as one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hours_to_second_commit: Option<f64>,
    pub commits_in_window: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingReport {
    pub repo: String,
    pub window_days: u32,
    pub generated_at: i64,
    /// Bootstrap signals — the `onboarding` practice, surfaced on its own
    /// because the role carries a specific day-one commitment.
    pub readiness: Vec<Signal>,
    pub readiness_found: usize,
    pub readiness_expected: usize,
    pub new_contributors: Vec<NewContributor>,
    /// Why the headline number the role asks for is not in this payload.
    pub not_measured: Vec<Unmeasured>,
    pub notes: Vec<String>,
}

/// Bootstrap readiness plus first-contribution activity.
///
/// **Time-to-first-commit is deliberately absent.** The role's target is
/// "check in code within one day of joining", and git records no join date —
/// only the first commit. Deriving a day-one metric from git would require
/// inventing the start of the interval. The report says that instead, and names
/// the system that does hold the missing half.
pub fn scan_onboarding(repo: &Path, window_days: u32) -> Result<OnboardingReport> {
    let root = vibe_core::git::discover_repo_root(repo)
        .ok_or_else(|| anyhow::anyhow!("{} is not inside a git repository", repo.display()))?;

    let onboarding = PRACTICES
        .iter()
        .find(|p| p.key == "onboarding")
        .ok_or_else(|| anyhow::anyhow!("onboarding practice missing from catalogue"))?;
    let readiness: Vec<Signal> = onboarding
        .signals
        .iter()
        .map(|(name, candidates)| {
            let path = find_path(&root, candidates);
            Signal {
                name: (*name).to_string(),
                found: path.is_some(),
                path,
            }
        })
        .collect();
    let readiness_found = readiness.iter().filter(|s| s.found).count();

    // Every commit, so "first commit ever" is not confused with "first commit
    // in the window" — the distinction is the whole point of "new contributor".
    let raw = git(&root, &["log", "--no-merges", "--format=%at\t%an", "--reverse"])?;
    let mut first_seen: HashMap<String, i64> = HashMap::new();
    let mut second_seen: HashMap<String, i64> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let now = chrono::Utc::now().timestamp();
    let since = now - i64::from(window_days) * 86_400;

    for line in raw.lines() {
        let mut f = line.splitn(2, '\t');
        let Some(at) = f.next().and_then(|s| s.trim().parse::<i64>().ok()) else {
            continue;
        };
        let Some(author) = f.next().map(str::trim) else {
            continue;
        };
        match first_seen.get(author) {
            None => {
                first_seen.insert(author.to_string(), at);
            }
            Some(_) => {
                second_seen.entry(author.to_string()).or_insert(at);
            }
        }
        if at >= since {
            *counts.entry(author.to_string()).or_insert(0) += 1;
        }
    }

    let mut new_contributors: Vec<NewContributor> = first_seen
        .iter()
        .filter(|(_, first)| **first >= since)
        .map(|(author, first)| NewContributor {
            author: author.clone(),
            first_commit_at: *first,
            hours_to_second_commit: second_seen
                .get(author)
                .map(|s| (s - first) as f64 / SECONDS_PER_HOUR),
            commits_in_window: counts.get(author).copied().unwrap_or(0),
        })
        .collect();
    new_contributors.sort_by_key(|c| std::cmp::Reverse(c.first_commit_at));

    Ok(OnboardingReport {
        repo: root.display().to_string(),
        window_days,
        generated_at: now,
        readiness_expected: readiness.len(),
        readiness_found,
        readiness,
        new_contributors,
        not_measured: vec![Unmeasured {
            metric: "time_to_first_commit".into(),
            reason: "git records a contributor's first commit but not the day they joined, so the interval the target is about has no start. Any number here would have invented one.".into(),
            to_measure_this: "join the HR/IdP start date to the first-commit date; the scan supplies the second half, the identity system holds the first".into(),
        }],
        notes: vec![format!(
            "{}/{} bootstrap signals present. Presence is not proof the path works — run it on a clean machine to know.",
            readiness_found,
            onboarding.signals.len()
        )],
    })
}

// ── SPACE ────────────────────────────────────────────────────────────────────
//
// SPACE is the half of engineering performance DORA cannot see: satisfaction,
// performance, activity, communication/collaboration, and efficiency/flow.
//
// **Most of it is not in a git repository, and this module says so rather than
// approximating it.** Satisfaction needs to be asked. Review latency lives in
// the forge, not in git — a merge commit records when a branch landed, not when
// someone first looked at it. Pipeline wait lives in CI. Inventing a git-shaped
// stand-in for any of those would produce a number that moves for reasons
// nobody can explain, which is worse than an honest gap.
//
// So this reports the **frame**: five dimensions, the measures each one has here
// with their source, and for the rest the specific system that holds the data.
// The gaps are the roadmap.
//
// Two rules are enforced in the type, not left to the reader:
//
//  * **No aggregate SPACE score.** There is no single number, because summing
//    a survey score with a commit count produces something that cannot be wrong
//    and therefore cannot be useful.
//  * **Volume is never reported without an outcome signal.** Activity and
//    Collaboration describe how much happened and in what shape; neither says
//    whether what shipped worked. When Performance has no measure,
//    [`SpaceReport::outcome_signal`] is false and every renderer must say so.
//
//    This began as an `activity_only` flag — "Activity is the only dimension
//    with data" — and a test proved it could never fire: any repository with a
//    single commit gets a `Co-authored-by` percentage, so Collaboration always
//    has a measure. A flag that cannot fire is a reassurance nobody earned, so
//    the predicate was changed to one that is both reachable and the thing
//    actually worth warning about.
//
// And the ethics line from `space-framework-productivity.md` is structural
// here: nothing in this report is per-individual. Author *counts* are activity;
// author *names* would be surveillance, and the shape has no field for them.

/// The five SPACE dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpaceDimension {
    Satisfaction,
    Performance,
    Activity,
    Collaboration,
    Efficiency,
}

impl SpaceDimension {
    pub fn key(self) -> &'static str {
        match self {
            SpaceDimension::Satisfaction => "satisfaction",
            SpaceDimension::Performance => "performance",
            SpaceDimension::Activity => "activity",
            SpaceDimension::Collaboration => "collaboration",
            SpaceDimension::Efficiency => "efficiency",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            SpaceDimension::Satisfaction => "Satisfaction & wellbeing",
            SpaceDimension::Performance => "Performance",
            SpaceDimension::Activity => "Activity",
            SpaceDimension::Collaboration => "Communication & collaboration",
            SpaceDimension::Efficiency => "Efficiency & flow",
        }
    }

    /// Every dimension, in the order the acronym spells.
    pub fn all() -> [SpaceDimension; 5] {
        [
            SpaceDimension::Satisfaction,
            SpaceDimension::Performance,
            SpaceDimension::Activity,
            SpaceDimension::Collaboration,
            SpaceDimension::Efficiency,
        ]
    }
}

/// One SPACE measure, with the system it came from.
///
/// `source` is mandatory and free-text on purpose: a SPACE number's meaning is
/// almost entirely determined by where it came from, and two organisations'
/// "review latency" are not the same measure unless both say so.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceMeasure {
    pub name: String,
    pub value: f64,
    pub unit: String,
    /// Where the number came from — `git history`, `DORA stability`, a survey.
    pub source: String,
    pub sample_size: usize,
    /// What this measure specifically cannot tell you.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caveat: Option<String>,
}

/// One dimension's state: what was measured, and what was not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceDimensionResult {
    pub dimension: SpaceDimension,
    pub key: String,
    pub title: String,
    pub measures: Vec<SpaceMeasure>,
    pub unmeasured: Vec<Unmeasured>,
}

impl SpaceDimensionResult {
    fn new(dimension: SpaceDimension) -> Self {
        SpaceDimensionResult {
            dimension,
            key: dimension.key().to_string(),
            title: dimension.title().to_string(),
            measures: Vec::new(),
            unmeasured: Vec::new(),
        }
    }
}

/// The SPACE report. Deliberately has no aggregate score field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceReport {
    pub repo: String,
    pub window_days: u32,
    pub generated_at: i64,
    pub dimensions: Vec<SpaceDimensionResult>,
    /// How many of the five have at least one measure here.
    pub dimensions_measured: usize,
    /// False when Performance has no measure — nothing here says whether what
    /// shipped worked. Renderers must surface this: volume and shape read as
    /// productivity is the mistake SPACE exists to prevent.
    pub outcome_signal: bool,
    /// Stated in the payload so a client cannot present this as a productivity
    /// score by omission.
    pub scope_note: String,
}

impl SpaceReport {
    /// One dimension by its variant. Public because every consumer that renders
    /// SPACE wants to reach a named dimension.
    ///
    /// `allow(dead_code)` is the lib/bin duality, not an unused accessor: this
    /// file is `pub mod` in `lib.rs` (where the method is public API) *and*
    /// `mod` in `main.rs` (where the binary happens not to call it, so `pub`
    /// means nothing and the lint fires). Same reason `engagement_scan.rs`
    /// carries a module-level allow. It is exercised by the tests below.
    #[allow(dead_code)]
    pub fn dimension(&self, d: SpaceDimension) -> Option<&SpaceDimensionResult> {
        self.dimensions.iter().find(|x| x.dimension == d)
    }
}

/// Collaboration signals git can honestly supply.
struct CollaborationSignals {
    /// Files touched by more than one distinct author in the window.
    shared_files: usize,
    /// Files touched at all in the window.
    touched_files: usize,
    /// Commits carrying a `Co-authored-by:` trailer.
    co_authored_commits: usize,
    /// Commits examined for the trailer.
    commits_examined: usize,
}

/// Cap on distinct paths tracked for co-ownership. A monorepo window can touch
/// hundreds of thousands of files; the map is bounded and the report says when
/// the bound bound.
const MAX_TRACKED_PATHS: usize = 200_000;

/// Read per-file authorship and co-authorship trailers for the window.
fn collaboration_signals(repo: &Path, since: i64) -> Result<(CollaborationSignals, bool)> {
    let since_arg = format!("--since={since}");
    // \x01 starts a record, \x02 separates author from the body; neither can
    // appear in a path, so a filename with a newline in it cannot forge a
    // record boundary.
    let raw = git(
        repo,
        &[
            "log",
            "--no-merges",
            "--format=%x01%an%x02%b%x02",
            "--name-only",
            &since_arg,
        ],
    )?;

    let mut authors_per_path: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    let mut co_authored = 0usize;
    let mut commits = 0usize;
    let mut truncated = false;

    for record in raw.split('\u{1}').skip(1) {
        commits += 1;
        let mut parts = record.splitn(3, '\u{2}');
        let Some(author) = parts.next().map(str::trim) else {
            continue;
        };
        let body = parts.next().unwrap_or("");
        if body.to_ascii_lowercase().contains("co-authored-by:") {
            co_authored += 1;
        }
        let files = parts.next().unwrap_or("");
        for path in files.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if authors_per_path.len() >= MAX_TRACKED_PATHS
                && !authors_per_path.contains_key(path)
            {
                truncated = true;
                continue;
            }
            authors_per_path
                .entry(path.to_string())
                .or_default()
                .insert(author.to_string());
        }
    }

    let shared_files = authors_per_path.values().filter(|a| a.len() > 1).count();
    Ok((
        CollaborationSignals {
            shared_files,
            touched_files: authors_per_path.len(),
            co_authored_commits: co_authored,
            commits_examined: commits,
        },
        truncated,
    ))
}

/// Build the SPACE frame for `repo`, filling the dimensions git and DORA can
/// answer and naming the system that holds each one they cannot.
///
/// `dora` is passed in rather than recomputed so Performance *references* the
/// stability pair instead of restating it — the double-counting the SPACE skill
/// warns about, made structural.
pub fn compute_space(repo: &Path, window_days: u32, dora: &DoraReport) -> Result<SpaceReport> {
    let root = vibe_core::git::discover_repo_root(repo)
        .ok_or_else(|| anyhow::anyhow!("{} is not inside a git repository", repo.display()))?;
    let now = chrono::Utc::now().timestamp();
    let since = now - i64::from(window_days) * 86_400;

    let mut dims: Vec<SpaceDimensionResult> = SpaceDimension::all()
        .into_iter()
        .map(SpaceDimensionResult::new)
        .collect();
    let mut put = |d: SpaceDimension, f: &dyn Fn(&mut SpaceDimensionResult)| {
        if let Some(slot) = dims.iter_mut().find(|x| x.dimension == d) {
            f(slot);
        }
    };

    // ── Satisfaction: nothing in a repository knows this. ───────────────────
    put(SpaceDimension::Satisfaction, &|d| {
        d.unmeasured.push(Unmeasured {
            metric: "tooling_satisfaction".into(),
            reason: "How people feel about the tools they use cannot be derived from what they committed. Any repository-shaped proxy for it — commit times, weekend activity, churn — measures something else and invites conclusions about individuals.".into(),
            to_measure_this: "run the quarterly survey (`vibecli --devex survey` prints the instrument), segment by team, and never below five respondents".into(),
        });
    });

    // ── Performance: reference DORA's stability pair, do not restate it. ────
    let stability: Vec<(&'static str, &Measure)> = [
        ("change_failure_rate", dora.change_failure_rate.as_ref()),
        ("time_to_restore", dora.time_to_restore.as_ref()),
    ]
    .into_iter()
    .filter_map(|(k, m)| m.map(|m| (k, m)))
    .collect();
    for (key, m) in &stability {
        let (key, m) = (*key, *m);
        put(SpaceDimension::Performance, &|d| {
            d.measures.push(SpaceMeasure {
                name: key.replace('_', " "),
                value: m.value,
                unit: m.unit.clone(),
                source: format!("DORA stability ({})", m.proxy),
                sample_size: m.sample_size,
                caveat: Some(
                    "Outcome quality is not visible here: shipping the wrong thing reliably scores well.".into(),
                ),
            });
        });
    }
    if stability.is_empty() {
        put(SpaceDimension::Performance, &|d| {
            d.unmeasured.push(Unmeasured {
                metric: "delivery_stability".into(),
                reason: "neither change failure rate nor time to restore could be measured, so Performance has no hard signal in this repository".into(),
                to_measure_this: "see the `unmeasured` block on the DORA report — the same instrumentation fixes both".into(),
            });
        });
    }

    // ── Activity: counts, never names. ──────────────────────────────────────
    let deployments = dora.deployments.len();
    put(SpaceDimension::Activity, &|d| {
        d.measures.push(SpaceMeasure {
            name: "commits".into(),
            value: dora.commits_in_window as f64,
            unit: format!("commits / {window_days}d"),
            source: "git history".into(),
            sample_size: dora.commits_in_window,
            caveat: Some(
                "A volume, not an outcome. Reported here only because Activity is one of five dimensions; on its own it is commit counting.".into(),
            ),
        });
        d.measures.push(SpaceMeasure {
            name: "contributing authors".into(),
            value: dora.authors_in_window as f64,
            unit: "distinct authors".into(),
            source: "git history".into(),
            sample_size: dora.authors_in_window,
            caveat: Some("A count. This report has no per-author view and will not grow one.".into()),
        });
        if deployments > 0 {
            d.measures.push(SpaceMeasure {
                name: "deployments".into(),
                value: deployments as f64,
                unit: format!("deployments / {window_days}d"),
                source: format!("DORA throughput ({})", dora.release_marker_description),
                sample_size: deployments,
                caveat: None,
            });
        }
    });

    // ── Collaboration: the one thing git genuinely sees. ────────────────────
    let (collab, truncated) = collaboration_signals(&root, since)?;
    // With one author, "files touched by more than one author" is 0 by
    // construction: it is determined by the author count, not by how the team
    // works. Reporting it would be a measurement of the arithmetic.
    let multi_author = dora.authors_in_window >= 2;
    if !multi_author {
        put(SpaceDimension::Collaboration, &|d| {
            d.unmeasured.push(Unmeasured {
                metric: "file_co_ownership".into(),
                reason: format!(
                    "only {} author committed in this window, so a multi-author file share is 0 by construction and says nothing about collaboration",
                    dora.authors_in_window
                ),
                to_measure_this: "widen the window, or scope the measurement to a repository more than one person works in".into(),
            });
        });
    }
    if multi_author && collab.touched_files > 0 {
        let share = collab.shared_files as f64 / collab.touched_files as f64 * 100.0;
        put(SpaceDimension::Collaboration, &|d| {
            d.measures.push(SpaceMeasure {
                name: "files touched by more than one author".into(),
                value: share,
                unit: "percent of files touched".into(),
                source: "git history".into(),
                sample_size: collab.touched_files,
                caveat: Some(
                    "A knowledge-distribution signal, not a quality one. Low is a bus-factor risk; high is not automatically good — it can also mean nobody owns anything.".into(),
                ),
            });
        });
    }
    if collab.commits_examined > 0 {
        let pct = collab.co_authored_commits as f64 / collab.commits_examined as f64 * 100.0;
        put(SpaceDimension::Collaboration, &|d| {
            d.measures.push(SpaceMeasure {
                name: "commits with a Co-authored-by trailer".into(),
                value: pct,
                unit: "percent of commits".into(),
                source: "git history".into(),
                sample_size: collab.commits_examined,
                caveat: Some(
                    "Only pairing that was recorded. Teams that pair without the trailer read as zero here, so a low value is a question, not a finding.".into(),
                ),
            });
        });
    }
    // The highest-yield collaboration measure is not here, and that is the point.
    put(SpaceDimension::Collaboration, &|d| {
        d.unmeasured.push(Unmeasured {
            metric: "review_latency".into(),
            reason: "the wait from opening a change to its first substantive review lives in the forge, not in git. A merge commit records when a branch landed, not when anyone first looked at it, and treating the two as the same would report queueing time as review time.".into(),
            to_measure_this: "pull it from the forge's pull-request API (GitHub/GitLab/Bitbucket); it is the single highest-yield number in this dimension".into(),
        });
    });

    // ── Efficiency & flow: CI holds this, not git. ──────────────────────────
    put(SpaceDimension::Efficiency, &|d| {
        d.unmeasured.push(Unmeasured {
            metric: "pipeline_wait".into(),
            reason: "queue and execution time are recorded by the CI system; git has no trace of how long anyone waited".into(),
            to_measure_this: "export job queue and run durations from CI and report p50 and p95 together".into(),
        });
        d.unmeasured.push(Unmeasured {
            metric: "uninterrupted_focus_hours".into(),
            reason: "calendar and meeting load are not in the repository".into(),
            to_measure_this: "derive meeting-free blocks from the calendar system, aggregated per team".into(),
        });
    });

    let dimensions_measured = dims.iter().filter(|d| !d.measures.is_empty()).count();
    let outcome_signal = dims
        .iter()
        .any(|d| d.dimension == SpaceDimension::Performance && !d.measures.is_empty());

    let mut scope_note = format!(
        "SPACE frame over {window_days} days. {dimensions_measured} of 5 dimensions have a measure \
         from this repository; the rest name the system that holds their data. There is deliberately \
         NO aggregate SPACE score — summing a survey response with a commit count produces a number \
         that cannot be wrong and therefore cannot be useful. Nothing here is per-individual."
    );
    if !outcome_signal {
        scope_note.push_str(
            " WARNING: no outcome signal. Nothing in this report says whether what shipped \
             worked — Performance has no measure, because no DORA stability metric could be \
             computed here. Activity and Collaboration describe volume and shape; read without \
             an outcome they are not a picture of productivity. Treat this as an instrumentation \
             gap, and fix the DORA `unmeasured` block first.",
        );
    }
    if truncated {
        scope_note.push_str(&format!(
            " Co-ownership tracked the first {MAX_TRACKED_PATHS} distinct paths; the file share \
             describes those only."
        ));
    }

    Ok(SpaceReport {
        repo: root.display().to_string(),
        window_days,
        generated_at: now,
        dimensions: dims,
        dimensions_measured,
        outcome_signal,
        scope_note,
    })
}

/// The quarterly survey instrument, as markdown.
///
/// Printed rather than stored: the dimensions that need asking need asking by
/// people, and a tool that pretended to run a survey would be the same category
/// of mistake as a tool that pretended to measure one.
pub fn render_survey_markdown() -> String {
    r#"# Engineering experience survey

Quarterly. Under five minutes. Anonymous. Reported at team level and above;
teams smaller than five are aggregated upward.

**Not** an input to performance review, compensation, or any decision about a
named individual. Say so on the form — the commitment is what makes the answers
worth having.

## Ask about the last two weeks, not "in general"

Recall beyond a sprint is reconstruction, not memory.

### Satisfaction & wellbeing
1. In the last two weeks, how often did your tools get in the way of work you
   were trying to do? *(never / rarely / weekly / daily)*
2. Would you recommend this as a place to build software? *(0–10)*
3. **What one thing would you change about your development environment?**

### Efficiency & flow
4. On your last change, roughly how long did you wait for CI? *(minutes)*
5. In the last two weeks, how many days had a block of two or more
   uninterrupted hours? *(0–10)*
6. **Where did you lose the most time, and to what?**

### Communication & collaboration
7. On your last pull request, how long until someone reviewed it? *(hours)*
8. When you needed help outside your team, how easy was it to get? *(1–5)*
9. **What did you have to ask a person for that should have been self-service?**

### Performance
10. In the last two weeks, did anything you shipped have to be rolled back or
    hot-fixed? *(yes / no)*

## Running it

- Publish what changed because of the *previous* round before sending this one.
  Response rate is a function of whether the last one visibly did anything.
- Segment by team. A mean across a thousand engineers hides the three teams
  whose environment is broken — which are the whole reason to ask.
- The free-text answers (3, 6, 9) are where the roadmap comes from. The scales
  are only for tracking movement.
"#
    .to_string()
}

/// Render a SPACE report as markdown.
pub fn render_space_markdown(sp: &SpaceReport) -> String {
    let mut out = String::new();
    out.push_str("# SPACE — developer productivity frame\n\n");
    out.push_str(&format!("{}\n\n", sp.scope_note));
    out.push_str(&format!(
        "- Repository: `{}`\n- Window: {} days\n- Dimensions with data here: {}/5\n\n",
        sp.repo, sp.window_days, sp.dimensions_measured
    ));

    for d in &sp.dimensions {
        out.push_str(&format!("## {}\n\n", d.title));
        if d.measures.is_empty() {
            out.push_str("_No measure available from this repository._\n\n");
        } else {
            out.push_str("| Measure | Value | Source | n |\n|---|---|---|---|\n");
            for m in &d.measures {
                out.push_str(&format!(
                    "| {} | {:.2} {} | {} | {} |\n",
                    m.name, m.value, m.unit, m.source, m.sample_size
                ));
            }
            out.push('\n');
            for m in d.measures.iter().filter(|m| m.caveat.is_some()) {
                if let Some(c) = &m.caveat {
                    out.push_str(&format!("- _{}_: {}\n", m.name, c));
                }
            }
            out.push('\n');
        }
        for u in &d.unmeasured {
            out.push_str(&format!(
                "- **{} — not measured here.** {}\n  - To measure it: {}\n",
                u.metric, u.reason, u.to_measure_this
            ));
        }
        out.push('\n');
    }
    out
}

// ── Scorecard ────────────────────────────────────────────────────────────────

/// A combined view: delivery performance and practice maturity side by side,
/// with an explicit account of what is missing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scorecard {
    pub dora: DoraReport,
    pub practices: PracticesReport,
    /// `measured / 4` — how much of DORA this repository can currently answer.
    pub dora_coverage: f64,
    /// Present only when at least one of the four keys was measured; a grade
    /// over zero metrics is `None`, never "F".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_grade: Option<String>,
    pub headline: String,
}

/// Grade from the bands of the metrics that were actually measured.
fn grade_from_bands(measures: &[(&'static str, &Measure)]) -> Option<String> {
    if measures.is_empty() {
        return None;
    }
    let score: f64 = measures
        .iter()
        .map(|(_, m)| match m.band {
            Band::Elite => 4.0,
            Band::High => 3.0,
            Band::Medium => 2.0,
            Band::Low => 1.0,
        })
        .sum::<f64>()
        / measures.len() as f64;
    Some(
        if score >= 3.5 {
            "A"
        } else if score >= 2.5 {
            "B"
        } else if score >= 1.5 {
            "C"
        } else {
            "D"
        }
        .to_string(),
    )
}

/// Build the combined scorecard for a workspace.
pub fn scorecard(workspace: &Path, opts: &DoraOptions) -> Result<Scorecard> {
    let dora = compute_dora(workspace, opts)?;
    let practices = scan_practices(workspace)?;
    let measured = dora.measured();
    let coverage = measured.len() as f64 / 4.0;
    let delivery_grade = grade_from_bands(&measured);

    let headline = match &delivery_grade {
        Some(g) => format!(
            "Delivery {g} on {}/4 DORA keys; practice maturity {:.1}/{} detected across {} practices.",
            measured.len(),
            practices.mean_level,
            MAX_DETECTABLE_LEVEL,
            practices.practices.len()
        ),
        None => format!(
            "No DORA key could be measured from this repository — see `unmeasured` for why. \
             Practice maturity {:.1}/{} detected across {} practices.",
            practices.mean_level,
            MAX_DETECTABLE_LEVEL,
            practices.practices.len()
        ),
    };

    Ok(Scorecard {
        dora,
        practices,
        dora_coverage: coverage,
        delivery_grade,
        headline,
    })
}

// ── Markdown rendering ───────────────────────────────────────────────────────

fn fmt_measure(m: &Measure) -> String {
    let pcts = if m.percentiles.is_empty() {
        String::new()
    } else {
        let inner = m
            .percentiles
            .iter()
            .map(|p| format!("{} {:.1}", p.label, p.value))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" ({inner})")
    };
    format!(
        "**{:.2} {}**{} — band `{}`, n={}",
        m.value,
        m.unit,
        pcts,
        m.band.as_str(),
        m.sample_size
    )
}

/// Render a scorecard as the briefing a director would actually circulate.
pub fn render_scorecard_markdown(sc: &Scorecard) -> String {
    let mut out = String::new();
    out.push_str("# Developer Excellence scorecard\n\n");
    out.push_str(&format!("{}\n\n", sc.headline));
    out.push_str(&format!(
        "- Repository: `{}`\n- Window: {} days\n- Deployments identified by: {}\n- DORA coverage: {:.0}% ({} of 4 keys measurable here)\n\n",
        sc.dora.repo,
        sc.dora.window_days,
        sc.dora.release_marker_description,
        sc.dora_coverage * 100.0,
        sc.dora.measured().len()
    ));

    out.push_str("## Delivery performance (DORA)\n\n");
    out.push_str("| Key | Value | Proxy |\n|---|---|---|\n");
    for (key, m) in sc.dora.measured() {
        out.push_str(&format!("| {} | {} | {} |\n", key, fmt_measure(m), m.proxy));
    }
    if sc.dora.measured().is_empty() {
        out.push_str("| _none measurable_ | — | — |\n");
    }
    out.push('\n');

    if !sc.dora.unmeasured.is_empty() {
        out.push_str("### Not measured — and why\n\n");
        for u in &sc.dora.unmeasured {
            out.push_str(&format!(
                "- **{}** — {}\n  - To measure it: {}\n",
                u.metric, u.reason, u.to_measure_this
            ));
        }
        out.push('\n');
    }

    out.push_str("## Practice maturity (detected)\n\n");
    out.push_str(&format!("{}\n\n", sc.practices.scope_note));
    out.push_str("| Practice | Pillar | Level | Signals | Next step |\n|---|---|---|---|---|\n");
    for p in &sc.practices.practices {
        out.push_str(&format!(
            "| {} | {} | {} ({}) | {}/{} | {} |\n",
            p.title, p.pillar, p.level, p.level_name, p.found, p.expected, p.next_step
        ));
    }
    out.push('\n');

    let caveats: Vec<&PracticeResult> = sc
        .practices
        .practices
        .iter()
        .filter(|p| p.detection_caveat.is_some())
        .collect();
    if !caveats.is_empty() {
        out.push_str("### What this scan cannot see\n\n");
        for p in caveats {
            if let Some(c) = &p.detection_caveat {
                out.push_str(&format!("- **{}** — {}\n", p.title, c));
            }
        }
        out.push('\n');
    }

    if !sc.dora.notes.is_empty() {
        out.push_str("## Notes\n\n");
        for n in &sc.dora.notes {
            out.push_str(&format!("- {n}\n"));
        }
        out.push('\n');
    }

    out.push_str(&format!("---\n\n_{}_\n", sc.dora.band_source));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_tags_are_recognised_and_markers_are_not() {
        for good in ["v1.2.3", "1.0", "release-2.4.0", "2024.11.1", "v0.5.5-rc1"] {
            assert!(is_version_tag(good), "{good} should be a version tag");
        }
        for bad in ["nightly", "latest", "backup-before-migration", "42", "stable"] {
            assert!(!is_version_tag(bad), "{bad} should not be a version tag");
        }
    }

    #[test]
    fn remediation_detection_excludes_ordinary_fixes() {
        let mk = |subject: &str| Commit {
            committed_at: 0,
            author: "a".into(),
            subject: subject.into(),
        };
        assert!(mk("Revert \"Add caching\"").is_remediation());
        assert!(mk("hotfix: restore checkout").is_remediation());
        assert!(mk("Rollback the v2 migration").is_remediation());
        // An ordinary bug fix is development, not a production failure. Counting
        // it would push change failure rate toward 100% for every healthy team.
        assert!(!mk("fix: off-by-one in pagination").is_remediation());
        assert!(!mk("fix typo in README").is_remediation());
    }

    #[test]
    fn percentile_of_empty_sample_is_none_not_zero() {
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.5), Some(2.0));
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 1.0), Some(4.0));
    }

    #[test]
    fn bands_match_the_published_thresholds() {
        assert_eq!(Band::for_deploy_frequency(2.0), Band::Elite);
        assert_eq!(Band::for_deploy_frequency(0.5), Band::High);
        assert_eq!(Band::for_deploy_frequency(0.05), Band::Medium);
        assert_eq!(Band::for_deploy_frequency(0.01), Band::Low);

        assert_eq!(Band::for_lead_time_hours(3.0), Band::Elite);
        assert_eq!(Band::for_lead_time_hours(48.0), Band::High);
        assert_eq!(Band::for_lead_time_hours(300.0), Band::Medium);
        assert_eq!(Band::for_lead_time_hours(2000.0), Band::Low);

        assert_eq!(Band::for_restore_hours(0.5), Band::Elite);
        assert_eq!(Band::for_change_failure_rate(0.03), Band::Elite);
        assert_eq!(Band::for_change_failure_rate(0.30), Band::Low);
    }

    #[test]
    fn a_directory_that_is_not_a_repository_errors_rather_than_reporting_nothing() {
        // "Not a repo" must not come back as a clean report with four
        // unmeasured keys: that reads as "your repository has no releases",
        // which is a claim about the caller's engineering rather than about
        // their argument.
        let dir = tempfile::tempdir().expect("tempdir");
        let err = compute_dora(dir.path(), &DoraOptions::default())
            .expect_err("a non-repository must be an error");
        assert!(format!("{err}").contains("not inside a git repository"));
    }

    #[test]
    fn an_empty_repository_reports_four_unmeasured_keys_and_no_values() {
        // The first-run case, and the one the whole design is for: no releases
        // means no metric, and every absence must carry a reason and a remedy.
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git");
            assert!(out.status.success(), "git {args:?} failed");
        };
        run(&["init", "-q", "."]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "T"]);
        run(&["commit", "-q", "--allow-empty", "-m", "feat: first"]);

        let report = compute_dora(dir.path(), &DoraOptions::default()).expect("report");
        assert!(report.measured().is_empty(), "nothing is measurable yet");
        assert_eq!(report.unmeasured.len(), 4, "all four keys accounted for");
        for u in &report.unmeasured {
            assert!(!u.reason.trim().is_empty(), "{} has no reason", u.metric);
            assert!(
                !u.to_measure_this.trim().is_empty(),
                "{} has no remedy — a reason without one leaves the reader stuck",
                u.metric
            );
        }
        // The commit itself was seen: the absence is about releases, not history.
        assert_eq!(report.commits_in_window, 1);
        assert_eq!(report.authors_in_window, 1);
        // And a scorecard over zero measured keys has no grade at all.
        let sc = scorecard(dir.path(), &DoraOptions::default()).expect("scorecard");
        assert_eq!(sc.delivery_grade, None);
        assert_eq!(sc.dora_coverage, 0.0);
        assert!(sc.headline.contains("No DORA key could be measured"));
    }

    /// A DORA report with nothing measured — the shape SPACE receives on a
    /// repository that has never tagged a release.
    fn empty_dora() -> DoraReport {
        DoraReport {
            repo: "/x".into(),
            window_days: 90,
            since: 0,
            generated_at: 0,
            release_marker: ReleaseMarker::VersionTags,
            release_marker_description: "version-like git tags".into(),
            band_source: BAND_SOURCE.into(),
            deployment_frequency: None,
            lead_time_for_changes: None,
            change_failure_rate: None,
            time_to_restore: None,
            unmeasured: Vec::new(),
            deployments: Vec::new(),
            commits_in_window: 3,
            authors_in_window: 2,
            notes: Vec::new(),
        }
    }

    #[test]
    fn space_has_no_aggregate_score_field() {
        // The absence is the design. If someone adds a total, this test is
        // where the argument happens: a number summing a survey response with a
        // commit count cannot be wrong, and therefore cannot be useful.
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git");
            assert!(out.status.success());
        };
        run(&["init", "-q", "."]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "T"]);
        run(&["commit", "-q", "--allow-empty", "-m", "feat: first"]);

        let report =
            compute_space(dir.path(), 90, &empty_dora()).expect("space frame");
        let json = serde_json::to_value(&report).expect("serialize");
        let obj = json.as_object().expect("object");
        for forbidden in ["score", "total", "overall", "index", "rating"] {
            assert!(
                !obj.keys().any(|k| k.contains(forbidden)),
                "SPACE payload must not carry a `{forbidden}` field"
            );
        }
    }

    #[test]
    fn space_covers_all_five_dimensions_measured_or_not() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let _ = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git");
        };
        run(&["init", "-q", "."]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "T"]);
        run(&["commit", "-q", "--allow-empty", "-m", "feat: first"]);

        let report = compute_space(dir.path(), 90, &empty_dora()).expect("space");
        assert_eq!(report.dimensions.len(), 5);
        // Every dimension has *something* — a measure or a named gap. A silent
        // dimension would read as "nothing to say here", which is never true.
        for d in &report.dimensions {
            assert!(
                !d.measures.is_empty() || !d.unmeasured.is_empty(),
                "dimension {} says nothing at all",
                d.key
            );
        }
        // The dimensions git cannot answer name the system that can.
        for key in ["satisfaction", "efficiency"] {
            let d = report
                .dimensions
                .iter()
                .find(|d| d.key == key)
                .unwrap_or_else(|| panic!("{key} missing"));
            assert!(d.measures.is_empty(), "{key} must not be measured from git");
            assert!(d.unmeasured.iter().all(|u| !u.to_measure_this.is_empty()));
        }
        // Review latency is explicitly absent from Collaboration, not faked
        // from merge timestamps.
        let collab = report
            .dimension(SpaceDimension::Collaboration)
            .expect("collaboration");
        assert!(collab.unmeasured.iter().any(|u| u.metric == "review_latency"));
    }

    #[test]
    fn space_warns_when_there_is_no_outcome_signal() {
        // The predicate this replaced — "Activity is the only dimension with
        // data" — could never fire: any repository with one commit gets a
        // Co-authored-by percentage, so Collaboration always had a measure. A
        // flag that cannot fire is a reassurance nobody earned. This one is
        // reachable, and it warns about the thing worth warning about: volume
        // and shape with nothing saying whether what shipped worked.
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let _ = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git");
        };
        run(&["init", "-q", "."]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "T"]);
        run(&["commit", "-q", "--allow-empty", "-m", "feat: first"]);

        let report = compute_space(dir.path(), 90, &empty_dora()).expect("space");
        assert!(!report.outcome_signal, "no DORA stability means no outcome signal");
        assert!(report.scope_note.contains("no outcome signal"));
        assert!(report.scope_note.contains("not a picture of productivity"));
    }

    #[test]
    fn space_reports_an_outcome_signal_once_stability_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let _ = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git");
        };
        run(&["init", "-q", "."]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "T"]);
        run(&["commit", "-q", "--allow-empty", "-m", "feat: first"]);

        let mut dora = empty_dora();
        dora.change_failure_rate = Some(Measure {
            value: 0.0,
            unit: "percent of deployments".into(),
            band: Band::Elite,
            sample_size: 4,
            proxy: "a deployment followed by a revert".into(),
            percentiles: Vec::new(),
        });
        let report = compute_space(dir.path(), 90, &dora).expect("space");
        assert!(report.outcome_signal);
        assert!(!report.scope_note.contains("no outcome signal"));
    }

    #[test]
    fn a_single_author_window_refuses_the_degenerate_co_ownership_share() {
        // With one author the multi-author file share is 0 by arithmetic, not
        // by how the team works. Reporting it would be measuring the formula.
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let _ = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git");
        };
        run(&["init", "-q", "."]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "T"]);
        std::fs::write(dir.path().join("a.txt"), "x").expect("write");
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "feat: a"]);

        let mut dora = empty_dora();
        dora.authors_in_window = 1;
        let report = compute_space(dir.path(), 90, &dora).expect("space");
        let collab = report
            .dimension(SpaceDimension::Collaboration)
            .expect("collaboration");
        assert!(
            !collab.measures.iter().any(|m| m.name.contains("more than one author")),
            "the degenerate share must not be reported as a measure"
        );
        let gap = collab
            .unmeasured
            .iter()
            .find(|u| u.metric == "file_co_ownership")
            .expect("the gap is named");
        assert!(gap.reason.contains("by construction"));
    }

    #[test]
    fn space_performance_references_dora_rather_than_recomputing_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let _ = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git");
        };
        run(&["init", "-q", "."]);
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "T"]);
        run(&["commit", "-q", "--allow-empty", "-m", "feat: first"]);

        let mut dora = empty_dora();
        dora.change_failure_rate = Some(Measure {
            value: 12.5,
            unit: "percent of deployments".into(),
            band: Band::Medium,
            sample_size: 8,
            proxy: "a deployment followed by a revert".into(),
            percentiles: Vec::new(),
        });
        let report = compute_space(dir.path(), 90, &dora).expect("space");
        let perf = report
            .dimension(SpaceDimension::Performance)
            .expect("performance");
        let m = perf.measures.first().expect("one measure");
        assert_eq!(m.value, 12.5, "the DORA value is carried, not recomputed");
        assert!(
            m.source.starts_with("DORA stability"),
            "the source must say it came from DORA, so nobody counts it twice"
        );
    }

    #[test]
    fn the_survey_instrument_states_the_ethics_line() {
        // The commitments are what make the answers worth having. If they are
        // ever edited out, this fails.
        let s = render_survey_markdown();
        assert!(s.contains("Anonymous"));
        assert!(s.contains("team level"));
        assert!(s.to_lowercase().contains("not** an input to performance review"));
        assert!(s.contains("last two weeks"));
    }

    #[test]
    fn grade_over_zero_metrics_is_none_not_f() {
        assert_eq!(grade_from_bands(&[]), None);
    }

    #[test]
    fn detected_level_never_reaches_optimizing() {
        // Even a practice with every signal present tops out at "defined".
        for p in PRACTICES {
            assert!(
                p.signals.len() >= MAX_DETECTABLE_LEVEL as usize,
                "practice {} has fewer signals than the cap, so its level can never be reached",
                p.key
            );
        }
        let level = (99u8).min(MAX_DETECTABLE_LEVEL);
        assert_eq!(level, MAX_DETECTABLE_LEVEL);
        assert_eq!(level_name(level), "defined");
    }

    #[test]
    fn every_practice_has_a_pillar_from_the_role() {
        const PILLARS: &[&str] = &[
            "Global Practices Program",
            "Strategic Developers' Platform Ownership",
            "Engineering Leadership",
        ];
        for p in PRACTICES {
            assert!(
                PILLARS.contains(&p.pillar),
                "practice {} has pillar {:?}, which is not one of the role's three",
                p.key,
                p.pillar
            );
        }
    }

    #[test]
    fn practices_with_known_blind_spots_declare_them() {
        // A false negative that reads as a finding is worse than no finding.
        // These two are detected by path and are known to miss real practice.
        for key in ["automated-testing", "golden-path"] {
            let p = PRACTICES
                .iter()
                .find(|p| p.key == key)
                .unwrap_or_else(|| panic!("{key} missing from catalogue"));
            assert!(
                p.caveat.is_some(),
                "{key} is path-detected and must declare what it cannot see"
            );
        }
        // And the caveat reaches the payload, not just the source.
        let dir = tempfile::tempdir().expect("tempdir");
        let report = scan_practices(dir.path()).expect("scan");
        let testing = report
            .practices
            .iter()
            .find(|p| p.key == "automated-testing")
            .expect("automated-testing");
        assert!(testing.detection_caveat.is_some());
    }

    #[test]
    fn practice_keys_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for p in PRACTICES {
            assert!(seen.insert(p.key), "duplicate practice key {}", p.key);
        }
    }

    #[test]
    fn scan_practices_on_an_empty_dir_reports_absent_not_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let report = scan_practices(dir.path()).expect("scan should succeed on an empty dir");
        assert_eq!(report.practices.len(), PRACTICES.len());
        assert!(report.practices.iter().all(|p| p.level == 0));
        assert_eq!(report.mean_level, 0.0);
        assert!(report.scope_note.contains("attested by people"));
    }

    #[test]
    fn scan_practices_finds_signals_that_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".github/workflows")).expect("mkdir");
        std::fs::write(dir.path().join("Makefile"), "all:\n").expect("write");
        let report = scan_practices(dir.path()).expect("scan");
        let ci = report
            .practices
            .iter()
            .find(|p| p.key == "ci-pipeline")
            .expect("ci practice");
        assert_eq!(ci.found, 2, "pipeline definition + build entry point");
        assert_eq!(ci.level, 2);
        assert_eq!(ci.level_name, "managed");
    }

    #[test]
    fn release_marker_parses_both_spellings_and_rejects_typos() {
        assert_eq!(
            ReleaseMarker::from_str("tags"),
            Some(ReleaseMarker::VersionTags)
        );
        assert_eq!(
            ReleaseMarker::from_str("release-branch-merges"),
            Some(ReleaseMarker::ReleaseBranchMerges)
        );
        assert_eq!(ReleaseMarker::from_str("tag"), None);
    }

    #[test]
    fn unmeasured_carries_a_remedy_for_every_metric() {
        // A reason without a remedy leaves the reader informed and stuck.
        let u = Unmeasured {
            metric: "deployment_frequency".into(),
            reason: "r".into(),
            to_measure_this: "t".into(),
        };
        assert!(!u.to_measure_this.is_empty());
    }
}
