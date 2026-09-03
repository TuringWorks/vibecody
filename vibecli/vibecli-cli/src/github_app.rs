//! GitHub App webhook handler for CI/CD AI review.
//!
//! Receives `pull_request.opened` / `pull_request.synchronize` webhooks,
//! runs the VibeCLI code review pipeline, and posts results as:
//! - PR review comments
//! - Commit status checks (pending → success/failure)
//!
//! # Setup
//!
//! ```toml
//! [github_app]
//! app_id = 12345
//! private_key_path = "path/to/key.pem"   # or set GITHUB_APP_PRIVATE_KEY
//! webhook_secret = "your-webhook-secret"  # or set GITHUB_APP_WEBHOOK_SECRET
//! auto_fix = false                        # attach committable suggestion blocks
//! ```

use anyhow::Result;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use vibe_ai::provider::AIProvider;

use crate::bugbot;

type HmacSha256 = Hmac<Sha256>;

// ── Configuration ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubAppConfig {
    /// GitHub App ID.
    #[serde(default)]
    pub app_id: u64,
    /// Path to the PEM private key file (or set GITHUB_APP_PRIVATE_KEY env).
    #[serde(default)]
    pub private_key_path: Option<String>,
    /// Webhook secret for HMAC-SHA256 validation.
    #[serde(default)]
    pub webhook_secret: Option<String>,
    /// Attach a committable ```` ```suggestion ```` block to each finding that has
    /// an anchored fix, so a reviewer applies it with one click.
    ///
    /// This costs one extra model round-trip per actionable finding (bounded by
    /// [`crate::bugbot_autofix::AutofixLimits`]). It never pushes a commit and
    /// never claims a fix compiles — see [`crate::bugbot_autofix`].
    #[serde(default)]
    pub auto_fix: bool,
    /// Minimum severity threshold to fail the status check.
    /// One of: "critical", "high", "medium", "low" (default: "high").
    #[serde(default = "default_severity_threshold")]
    pub severity_threshold: String,
}

fn default_severity_threshold() -> String {
    "high".to_string()
}

impl Default for GithubAppConfig {
    fn default() -> Self {
        Self {
            app_id: 0,
            private_key_path: None,
            webhook_secret: None,
            auto_fix: false,
            severity_threshold: default_severity_threshold(),
        }
    }
}

impl GithubAppConfig {
    /// Resolve the webhook secret. Order (per AGENTS.md → Zero-Config First):
    ///   0. ProfileStore key `github_app_webhook_secret` (encrypted)
    ///   1. `webhook_secret` field on this struct (config.toml)
    ///   2. `GITHUB_APP_WEBHOOK_SECRET` env var (compat fallback)
    pub fn resolve_webhook_secret(&self) -> Option<String> {
        if let Ok(store) = crate::profile_store::ProfileStore::new() {
            if let Ok(Some(s)) = store.get_api_key("default", "github_app_webhook_secret") {
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
        self.webhook_secret.clone().or_else(|| {
            std::env::var("GITHUB_APP_WEBHOOK_SECRET")
                .ok()
                .filter(|s| !s.is_empty())
        })
    }
}

// ── Webhook types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WebhookPayload {
    pub action: String,
    pub pull_request: Option<PullRequestPayload>,
    pub repository: Option<RepoPayload>,
    pub installation: Option<InstallationPayload>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PullRequestPayload {
    pub number: u64,
    pub title: String,
    pub head: GitRef,
    pub base: GitRef,
    pub diff_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GitRef {
    pub sha: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepoPayload {
    pub full_name: String,
    pub clone_url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct InstallationPayload {
    pub id: u64,
}

// ── Webhook signature verification ───────────────────────────────────────────

/// Verify the webhook payload signature using HMAC-SHA256.
pub fn verify_signature(secret: &str, payload: &[u8], signature: &str) -> bool {
    // GitHub sends: sha256=hex_digest
    let hex_sig = signature.strip_prefix("sha256=").unwrap_or(signature);

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(payload);

    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

    mac.verify_slice(&expected).is_ok()
}

// ── Review result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CIReviewResult {
    pub pr_number: u64,
    pub repo: String,
    pub commit_sha: String,
    pub findings_count: usize,
    pub severity_counts: SeverityCounts,
    pub status: String, // "success" | "failure"
    pub summary: String,
    pub timestamp: u64,
    /// Findings that got a committable ```` ```suggestion ```` block.
    ///
    /// Always 0 unless `auto_fix` is on; a finding the fixer declined is not
    /// counted, so this is the number of fixes a reviewer can actually commit.
    #[serde(default)]
    pub fixes_proposed: usize,
    /// What the review actually read. `findings_count` is only a statement about
    /// the whole PR when `coverage.is_complete()`.
    #[serde(default)]
    pub coverage: bugbot::ReviewCoverage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

// ── Core review pipeline ─────────────────────────────────────────────────────

/// Fetch the PR diff from GitHub and run the review pipeline.
pub async fn review_pull_request(
    owner: &str,
    repo: &str,
    pr_number: u64,
    head_sha: &str,
    _base_ref: &str,
    llm: Arc<dyn AIProvider>,
    config: &GithubAppConfig,
) -> Result<CIReviewResult> {
    let full_name = format!("{}/{}", owner, repo);
    let token = resolve_github_token();

    // 1. Post pending status check
    if let Some(ref tok) = token {
        let _ = post_status_check(
            &full_name,
            head_sha,
            "pending",
            "VibeCody is reviewing this PR...",
            tok,
        )
        .await;
    }

    // 2. Fetch the PR diff
    let diff = fetch_pr_diff(owner, repo, pr_number, token.as_deref()).await?;

    // 3. Run BugBot review (static patterns + LLM)
    let llm_for_fixes = Arc::clone(&llm);
    let mut bugbot = bugbot::BugBot::new(llm);
    if let Some(ref tok) = token {
        bugbot = bugbot.with_gh_token(tok.clone());
    }
    let (reports, coverage) = bugbot
        .review_diff_planned(&diff, bugbot::ReviewPlan::default())
        .await;

    // 4. Count severities
    let mut counts = SeverityCounts::default();
    for r in &reports {
        match r.severity {
            bugbot::Severity::Error => counts.high += 1,
            bugbot::Severity::Warning => counts.medium += 1,
            bugbot::Severity::Info => counts.low += 1,
        }
    }

    // 5. Determine pass/fail based on threshold
    let failed = match config.severity_threshold.to_lowercase().as_str() {
        "critical" => counts.critical > 0,
        "high" => counts.critical > 0 || counts.high > 0,
        "medium" => counts.critical > 0 || counts.high > 0 || counts.medium > 0,
        "low" => counts.critical + counts.high + counts.medium + counts.low > 0,
        _ => counts.critical > 0 || counts.high > 0,
    };

    let status = if failed { "failure" } else { "success" };

    // 6. Propose committable fixes (opt-in via `auto_fix`).
    //
    //    Anchors come from the diff's own post-image, so a suggestion can only
    //    ever target lines this PR actually shows. Findings the fixer declines
    //    are still posted — as prose, exactly as before.
    let fixes = if config.auto_fix && !reports.is_empty() {
        let post = crate::bugbot_autofix::PostImage::from_diff(&diff);
        let attempts = crate::bugbot_autofix::propose_fixes(
            &llm_for_fixes,
            &post,
            &reports,
            crate::bugbot_autofix::AutofixLimits::default(),
        )
        .await;
        for (index, attempt) in &attempts {
            if let Err(reason) = attempt {
                tracing::debug!(
                    target: "vibecody::github_app::autofix",
                    finding = index,
                    %reason,
                    "no committable fix proposed"
                );
            }
        }
        attempts
            .into_iter()
            .filter_map(|(index, attempt)| attempt.ok().map(|fix| (index, fix)))
            .collect()
    } else {
        HashMap::new()
    };

    // 7. Post review comments to PR
    if !reports.is_empty() {
        let _ = bugbot
            .post_github_review_with_fixes(owner, repo, pr_number, &reports, &fixes, head_sha)
            .await;
    }

    // 8. Post final status check
    //
    //    The caveat matters more than the counts: "0 issues" over a partially
    //    reviewed diff is not the same claim as "0 issues" over all of it.
    let summary = format!(
        "VibeCody found {} issue(s): {} critical, {} high, {} medium, {} low{}{}",
        reports.len(),
        counts.critical,
        counts.high,
        counts.medium,
        counts.low,
        match fixes.len() {
            0 => String::new(),
            n => format!(" · {} committable fix(es) proposed", n),
        },
        match coverage.caveat() {
            None => String::new(),
            Some(caveat) => format!(" · {}", caveat),
        }
    );

    if let Some(ref tok) = token {
        let _ = post_status_check(&full_name, head_sha, status, &summary, tok).await;
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(CIReviewResult {
        pr_number,
        repo: full_name,
        commit_sha: head_sha.to_string(),
        findings_count: reports.len(),
        severity_counts: counts,
        status: status.to_string(),
        summary,
        timestamp: ts,
        fixes_proposed: fixes.len(),
        coverage,
    })
}

// ── GitHub API helpers ───────────────────────────────────────────────────────

/// ProfileStore keys a GitHub token can be stored under, in precedence order.
///
/// Two keys because two surfaces write it and both must be honoured: the
/// desktop apps save what the user types in Settings → Integrations →
/// Infrastructure, and the CLI saves `vibecli set-key github`. Reading only one
/// of them meant a token entered in the app was invisible to bugbot and the
/// vulnerability scanner, and vice versa, with nothing on screen to explain it.
pub const GITHUB_TOKEN_STORE_KEYS: [&str; 2] = ["integration.infra.github_token", "github"];

/// Single source of truth for resolving a GitHub OAuth / PAT token.
///
/// Order (per AGENTS.md → Zero-Config First):
///   0. ProfileStore, [`GITHUB_TOKEN_STORE_KEYS`] in order — encrypted, and
///      what a user typed into an app or the CLI
///   1. `GITHUB_TOKEN` env var
///   2. `GH_TOKEN` env var (gh CLI compatibility)
///
/// What the user configured wins over the ambient environment: the other way
/// round, editing the field changed nothing for anyone with the variable
/// exported. The env vars stay as the last resort so CI and shell-configured
/// setups keep working with nothing stored.
///
/// Public so `bugbot.rs` and `vulnerability_db.rs` route through here
/// instead of each re-implementing their own env-only resolution.
pub fn resolve_github_token() -> Option<String> {
    let store = crate::profile_store::ProfileStore::new().ok();
    pick_github_token(
        &|key| {
            store
                .as_ref()
                .and_then(|s| s.get_api_key("default", key).ok().flatten())
        },
        &|name| std::env::var(name).ok(),
    )
}

/// The precedence rule on its own, with both sources injected.
///
/// Split out so the ordering is testable without a real profile DB or mutating
/// the process environment — which is shared state, and the top cause of flaky
/// tests in this repo.
fn pick_github_token(
    stored: &dyn Fn(&str) -> Option<String>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let non_empty = |s: String| {
        let t = s.trim().to_string();
        (!t.is_empty()).then_some(t)
    };
    GITHUB_TOKEN_STORE_KEYS
        .iter()
        .find_map(|key| stored(key).and_then(non_empty))
        .or_else(|| {
            ["GITHUB_TOKEN", "GH_TOKEN"]
                .iter()
                .find_map(|n| env(n).and_then(non_empty))
        })
}

/// Split `owner/repo` out of any GitHub remote URL form.
///
/// Handles `git@github.com:o/r.git`, `https://github.com/o/r.git`,
/// `ssh://git@github.com/o/r`, and the bare `o/r` slug. Returns `None` for a
/// remote that is not GitHub — the caller then asks the user for `--repo`
/// rather than guessing a slug that would review someone else's code.
pub fn parse_github_slug(remote: &str) -> Option<(String, String)> {
    let trimmed = remote.trim().trim_end_matches('/');
    let rest = trimmed
        .strip_prefix("git@github.com:")
        .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))
        .or_else(|| trimmed.strip_prefix("https://github.com/"))
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .or_else(|| trimmed.strip_prefix("github.com/"))
        .or_else(|| {
            // Bare `owner/repo`, but nothing that looks like another host.
            (!trimmed.contains("://") && !trimmed.contains('@')).then_some(trimmed)
        })?;

    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let (owner, repo) = rest.split_once('/')?;
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// Read `origin`'s URL in `cwd` and split it into `owner/repo`.
pub fn detect_repo_slug(cwd: &std::path::Path) -> Option<(String, String)> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_github_slug(&String::from_utf8_lossy(&out.stdout))
}

/// Fetch a pull request's head commit SHA.
///
/// A review must anchor to the commit the PR actually points at. Local `HEAD`
/// is not that commit unless the caller happens to have the PR branch checked
/// out and up to date — anchoring to it would attach comments to lines that
/// commit never contained.
pub async fn fetch_pr_head_sha(
    owner: &str,
    repo: &str,
    pr_number: u64,
    token: Option<&str>,
) -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}",
        owner, repo, pr_number
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "VibeCody-CI-Bot");
    if let Some(tok) = token {
        req = req.header("Authorization", format!("Bearer {}", tok));
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "GitHub API returned {} fetching {}/{} PR #{}",
            resp.status(),
            owner,
            repo,
            pr_number
        );
    }

    let body: serde_json::Value = resp.json().await?;
    body.get("head")
        .and_then(|h| h.get("sha"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("PR #{} response had no head.sha", pr_number))
}

/// Fetch the unified diff of a PR.
async fn fetch_pr_diff(
    owner: &str,
    repo: &str,
    pr_number: u64,
    token: Option<&str>,
) -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}",
        owner, repo, pr_number
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3.diff")
        .header("User-Agent", "VibeCody-CI-Bot");

    if let Some(tok) = token {
        req = req.header("Authorization", format!("Bearer {}", tok));
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "GitHub API returned {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        );
    }

    Ok(resp.text().await?)
}

/// Post a commit status check.
async fn post_status_check(
    repo_full_name: &str,
    sha: &str,
    state: &str,
    description: &str,
    token: &str,
) -> Result<()> {
    let url = format!(
        "https://api.github.com/repos/{}/statuses/{}",
        repo_full_name, sha
    );

    let body = serde_json::json!({
        "state": state,
        "description": &description[..description.len().min(140)],
        "context": "vibecody/review"
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "VibeCody-CI-Bot")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        eprintln!("[github-app] Status check POST failed: {}", resp.status());
    }

    Ok(())
}

// ── Webhook handler (used by serve.rs) ───────────────────────────────────────

/// Process a GitHub webhook event. Returns the review result if applicable.
pub async fn handle_webhook(
    payload: &[u8],
    event_type: &str,
    signature: Option<&str>,
    config: &GithubAppConfig,
    llm: Arc<dyn AIProvider>,
) -> Result<Option<CIReviewResult>> {
    // Fail closed. `/webhook/github` is one of the handful of public daemon
    // routes, and a review is not a read: it spends model budget and calls the
    // GitHub API with the operator's token against whatever repository the
    // payload names. Without a secret there is nothing tying a request to
    // GitHub, so an unsigned POST from anywhere would drive both.
    let Some(secret) = config.resolve_webhook_secret() else {
        anyhow::bail!(
            "GitHub App webhook secret is not configured — refusing to act on an unsigned \
             webhook. Set it with `vibecli set-key github_app_webhook_secret <secret>`."
        );
    };
    if !verify_signature(&secret, payload, signature.unwrap_or("")) {
        anyhow::bail!("Invalid webhook signature");
    }

    // Only process pull_request events
    if event_type != "pull_request" {
        return Ok(None);
    }

    let webhook: WebhookPayload = serde_json::from_slice(payload)?;

    // Only process opened and synchronize actions
    match webhook.action.as_str() {
        "opened" | "synchronize" | "reopened" => {}
        _ => return Ok(None),
    }

    let pr = webhook
        .pull_request
        .ok_or_else(|| anyhow::anyhow!("Missing pull_request"))?;
    let repo = webhook
        .repository
        .ok_or_else(|| anyhow::anyhow!("Missing repository"))?;

    let parts: Vec<&str> = repo.full_name.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid repo full_name: {}", repo.full_name);
    }

    let result = review_pull_request(
        parts[0],
        parts[1],
        pr.number,
        &pr.head.sha,
        &pr.base.ref_name,
        llm,
        config,
    )
    .await?;

    Ok(Some(result))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Neither source has anything.
    fn none(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn stored_token_beats_the_environment() {
        // The other way round, editing the field in Settings changed nothing
        // for anyone with GITHUB_TOKEN exported, and the UI could not say why.
        let token = pick_github_token(
            &|key| (key == "integration.infra.github_token").then(|| "from-settings".into()),
            &|_| Some("from-env".into()),
        );
        assert_eq!(token.as_deref(), Some("from-settings"));
    }

    #[test]
    fn reads_the_cli_key_when_settings_is_empty() {
        // `vibecli set-key github` and the desktop Settings field write to
        // different keys; both have to be honoured or one surface goes blind.
        let token = pick_github_token(&|key| (key == "github").then(|| "from-cli".into()), &none);
        assert_eq!(token.as_deref(), Some("from-cli"));
    }

    #[test]
    fn a_blank_stored_token_falls_through_to_the_environment() {
        // A cleared field is not a token; treating "" as one would 401 every
        // request while claiming a token was configured.
        let token = pick_github_token(&|_| Some("   ".into()), &|name| {
            (name == "GITHUB_TOKEN").then(|| "from-env".into())
        });
        assert_eq!(token.as_deref(), Some("from-env"));
    }

    #[test]
    fn falls_back_to_gh_cli_variable_last() {
        let token = pick_github_token(&none, &|name| {
            (name == "GH_TOKEN").then(|| "from-gh".into())
        });
        assert_eq!(token.as_deref(), Some("from-gh"));
    }

    #[test]
    fn nothing_configured_resolves_to_none() {
        assert_eq!(pick_github_token(&none, &none), None);
    }

    #[test]
    fn verify_valid_signature() {
        let secret = "test-secret";
        let payload = b"hello world";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());

        assert!(verify_signature(
            secret,
            payload,
            &format!("sha256={}", sig)
        ));
    }

    #[test]
    fn verify_invalid_signature() {
        assert!(!verify_signature("secret", b"payload", "sha256=deadbeef"));
    }

    #[test]
    fn verify_empty_signature() {
        assert!(!verify_signature("secret", b"payload", ""));
    }

    #[test]
    fn default_config() {
        let cfg = GithubAppConfig::default();
        assert_eq!(cfg.app_id, 0);
        assert!(!cfg.auto_fix);
        assert_eq!(cfg.severity_threshold, "high");
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = GithubAppConfig {
            app_id: 42,
            private_key_path: Some("/tmp/key.pem".into()),
            webhook_secret: Some("s3cret".into()),
            auto_fix: true,
            severity_threshold: "medium".into(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: GithubAppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.app_id, 42);
        assert!(parsed.auto_fix);
        assert_eq!(parsed.severity_threshold, "medium");
    }

    #[test]
    fn severity_counts_default() {
        let counts = SeverityCounts::default();
        assert_eq!(
            counts.critical + counts.high + counts.medium + counts.low + counts.info,
            0
        );
    }

    #[test]
    fn webhook_secret_from_env() {
        let cfg = GithubAppConfig::default();
        // Without env var, should return None
        let secret = cfg.resolve_webhook_secret();
        // Can't assert None because env might have it; just ensure no panic
        let _ = secret;
    }

    #[test]
    fn parse_webhook_payload() {
        let json = r#"{
            "action": "opened",
            "pull_request": {
                "number": 42,
                "title": "Test PR",
                "head": { "sha": "abc123", "ref": "feature/test" },
                "base": { "sha": "def456", "ref": "main" },
                "diff_url": "https://github.com/test/repo/pull/42.diff"
            },
            "repository": {
                "full_name": "owner/repo",
                "clone_url": "https://github.com/owner/repo.git"
            },
            "installation": { "id": 123 }
        }"#;
        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.action, "opened");
        assert_eq!(payload.pull_request.unwrap().number, 42);
        assert_eq!(payload.repository.unwrap().full_name, "owner/repo");
    }

    #[test]
    fn verify_signature_without_prefix() {
        let secret = "my-secret";
        let payload = b"test payload";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());
        // Without the sha256= prefix — should still work
        assert!(verify_signature(secret, payload, &sig));
    }

    #[test]
    fn verify_signature_invalid_hex() {
        // Non-hex characters after sha256= prefix
        assert!(!verify_signature("secret", b"payload", "sha256=zzzz"));
    }

    #[test]
    fn verify_signature_empty_secret() {
        let secret = "";
        let payload = b"data";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());
        assert!(verify_signature(
            secret,
            payload,
            &format!("sha256={}", sig)
        ));
    }

    #[test]
    fn verify_signature_empty_payload() {
        let secret = "test";
        let payload = b"";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());
        assert!(verify_signature(
            secret,
            payload,
            &format!("sha256={}", sig)
        ));
    }

    #[test]
    fn config_with_all_fields_set() {
        let cfg = GithubAppConfig {
            app_id: 99999,
            private_key_path: Some("/etc/keys/gh.pem".into()),
            webhook_secret: Some("webhook-s3cret".into()),
            auto_fix: true,
            severity_threshold: "critical".into(),
        };
        assert_eq!(cfg.app_id, 99999);
        assert_eq!(cfg.private_key_path.as_deref(), Some("/etc/keys/gh.pem"));
        assert!(cfg.auto_fix);
        assert_eq!(cfg.severity_threshold, "critical");
    }

    #[test]
    fn config_resolve_webhook_secret_from_field() {
        let cfg = GithubAppConfig {
            webhook_secret: Some("inline-secret".into()),
            ..Default::default()
        };
        assert_eq!(
            cfg.resolve_webhook_secret(),
            Some("inline-secret".to_string())
        );
    }

    #[test]
    fn severity_counts_individual_fields() {
        let counts = SeverityCounts {
            critical: 1,
            high: 2,
            medium: 3,
            low: 4,
            info: 5,
        };
        assert_eq!(counts.critical, 1);
        assert_eq!(counts.high, 2);
        assert_eq!(counts.medium, 3);
        assert_eq!(counts.low, 4);
        assert_eq!(counts.info, 5);
    }

    #[test]
    fn severity_counts_serde_roundtrip() {
        let counts = SeverityCounts {
            critical: 3,
            high: 7,
            medium: 12,
            low: 20,
            info: 5,
        };
        let json = serde_json::to_string(&counts).unwrap();
        let parsed: SeverityCounts = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.critical, 3);
        assert_eq!(parsed.high, 7);
        assert_eq!(parsed.medium, 12);
        assert_eq!(parsed.low, 20);
        assert_eq!(parsed.info, 5);
    }

    #[test]
    fn ci_review_result_serde_roundtrip() {
        let result = CIReviewResult {
            pr_number: 42,
            repo: "owner/repo".to_string(),
            commit_sha: "abc123def456".to_string(),
            findings_count: 5,
            severity_counts: SeverityCounts {
                critical: 0,
                high: 2,
                medium: 3,
                low: 0,
                info: 0,
            },
            status: "failure".to_string(),
            summary: "Found 5 issues".to_string(),
            timestamp: 1700000000,
            fixes_proposed: 2,
            coverage: bugbot::ReviewCoverage {
                files_total: 4,
                files_reviewed: 3,
                llm_calls: 2,
                files_skipped: vec!["late.rs".into()],
                ..Default::default()
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: CIReviewResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pr_number, 42);
        assert_eq!(parsed.repo, "owner/repo");
        assert_eq!(parsed.commit_sha, "abc123def456");
        assert_eq!(parsed.findings_count, 5);
        assert_eq!(parsed.status, "failure");
        assert_eq!(parsed.severity_counts.high, 2);
        assert_eq!(parsed.timestamp, 1700000000);
        assert_eq!(parsed.fixes_proposed, 2);
        assert_eq!(parsed.coverage.files_skipped, vec!["late.rs".to_string()]);
        assert!(!parsed.coverage.is_complete());
    }

    #[test]
    fn parse_webhook_payload_without_optional_fields() {
        let json = r#"{
            "action": "closed",
            "pull_request": null,
            "repository": null,
            "installation": null
        }"#;
        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.action, "closed");
        assert!(payload.pull_request.is_none());
        assert!(payload.repository.is_none());
        assert!(payload.installation.is_none());
    }

    #[test]
    fn parse_pr_payload_fields() {
        let json = r#"{
            "number": 100,
            "title": "Fix critical bug",
            "head": { "sha": "deadbeef", "ref": "fix/bug-123" },
            "base": { "sha": "cafebabe", "ref": "develop" },
            "diff_url": "https://github.com/org/repo/pull/100.diff"
        }"#;
        let pr: PullRequestPayload = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 100);
        assert_eq!(pr.title, "Fix critical bug");
        assert_eq!(pr.head.sha, "deadbeef");
        assert_eq!(pr.head.ref_name, "fix/bug-123");
        assert_eq!(pr.base.ref_name, "develop");
        assert_eq!(pr.diff_url, "https://github.com/org/repo/pull/100.diff");
    }

    #[tokio::test]
    async fn an_unsigned_webhook_is_rejected_when_no_secret_is_configured() {
        // The route is public: without a secret there is nothing tying the
        // request to GitHub, so acting on it would spend model budget and hit
        // the GitHub API for anyone who can reach the daemon.
        let cfg = GithubAppConfig {
            webhook_secret: Some(String::new()), // empty resolves to "unset"
            ..Default::default()
        };
        if cfg.resolve_webhook_secret().is_some() {
            // A real secret exists in this developer's ProfileStore or env;
            // the fail-closed branch is unreachable here, so skip rather than
            // assert something the environment decided.
            return;
        }
        let err = handle_webhook(
            br#"{"action":"opened"}"#,
            "pull_request",
            None,
            &cfg,
            unreachable_provider(),
        )
        .await
        .expect_err("an unsigned webhook must not be acted on");
        assert!(err.to_string().contains("not configured"));
    }

    #[tokio::test]
    async fn a_wrongly_signed_webhook_is_rejected() {
        let cfg = GithubAppConfig {
            webhook_secret: Some("the-real-secret".into()),
            ..Default::default()
        };
        let err = handle_webhook(
            br#"{"action":"opened"}"#,
            "pull_request",
            Some("sha256=deadbeef"),
            &cfg,
            unreachable_provider(),
        )
        .await
        .expect_err("a bad signature must not be acted on");
        assert!(err.to_string().contains("Invalid webhook signature"));
    }

    /// A provider that panics if reached.
    ///
    /// A rejected webhook must cost nothing: the assertion that matters is not
    /// only the error, but that no model call happened on the way to it.
    fn unreachable_provider() -> Arc<dyn AIProvider> {
        struct Unreachable;

        #[async_trait::async_trait]
        impl AIProvider for Unreachable {
            fn name(&self) -> &str {
                "unreachable"
            }
            async fn is_available(&self) -> bool {
                true
            }
            async fn complete(
                &self,
                _ctx: &vibe_ai::provider::CodeContext,
            ) -> Result<vibe_ai::provider::CompletionResponse> {
                panic!("a rejected webhook must not reach the provider")
            }
            async fn stream_complete(
                &self,
                _ctx: &vibe_ai::provider::CodeContext,
            ) -> Result<vibe_ai::provider::CompletionStream> {
                panic!("a rejected webhook must not reach the provider")
            }
            async fn chat(
                &self,
                _messages: &[vibe_ai::provider::Message],
                _context: Option<String>,
            ) -> Result<String> {
                panic!("a rejected webhook must not reach the provider")
            }
            async fn stream_chat(
                &self,
                _messages: &[vibe_ai::provider::Message],
            ) -> Result<vibe_ai::provider::CompletionStream> {
                panic!("a rejected webhook must not reach the provider")
            }
        }

        Arc::new(Unreachable)
    }

    #[test]
    fn parses_every_github_remote_form() {
        let expected = Some(("TuringWorks".to_string(), "vibecody".to_string()));
        for remote in [
            "git@github.com:TuringWorks/vibecody.git",
            "git@github.com:TuringWorks/vibecody",
            "https://github.com/TuringWorks/vibecody.git",
            "https://github.com/TuringWorks/vibecody",
            "https://github.com/TuringWorks/vibecody/",
            "ssh://git@github.com/TuringWorks/vibecody.git",
            "github.com/TuringWorks/vibecody",
            "TuringWorks/vibecody",
            "  https://github.com/TuringWorks/vibecody.git\n",
        ] {
            assert_eq!(parse_github_slug(remote), expected, "remote: {remote}");
        }
    }

    #[test]
    fn refuses_to_guess_a_slug_for_a_non_github_remote() {
        // A GitLab remote must not be reviewed as if it were a GitHub repo.
        assert_eq!(parse_github_slug("git@gitlab.com:owner/repo.git"), None);
        assert_eq!(parse_github_slug("https://bitbucket.org/owner/repo"), None);
        assert_eq!(parse_github_slug("ssh://git@example.com/owner/repo"), None);
    }

    #[test]
    fn rejects_malformed_slugs() {
        assert_eq!(parse_github_slug(""), None);
        assert_eq!(parse_github_slug("https://github.com/owner"), None);
        assert_eq!(
            parse_github_slug("https://github.com/owner/repo/extra"),
            None
        );
        assert_eq!(parse_github_slug("https://github.com//repo"), None);
    }

    #[test]
    fn config_deserialization_with_defaults() {
        let json = r#"{"app_id": 555}"#;
        let cfg: GithubAppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.app_id, 555);
        assert!(!cfg.auto_fix);
        assert_eq!(cfg.severity_threshold, "high");
        assert!(cfg.webhook_secret.is_none());
        assert!(cfg.private_key_path.is_none());
    }
}
