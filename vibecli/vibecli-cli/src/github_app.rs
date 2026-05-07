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
//! auto_fix = false                        # push auto-fixes to PR branch
//! ```

use anyhow::Result;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
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
    /// Automatically push fixes to the PR branch.
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
                if !s.is_empty() { return Some(s); }
            }
        }
        self.webhook_secret
            .clone()
            .or_else(|| std::env::var("GITHUB_APP_WEBHOOK_SECRET").ok().filter(|s| !s.is_empty()))
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
            &full_name, head_sha, "pending",
            "VibeCody is reviewing this PR...", tok,
        ).await;
    }

    // 2. Fetch the PR diff
    let diff = fetch_pr_diff(owner, repo, pr_number, token.as_deref()).await?;

    // 3. Run BugBot review (static patterns + LLM)
    let mut bugbot = bugbot::BugBot::new(llm);
    if let Some(ref tok) = token {
        bugbot = bugbot.with_gh_token(tok.clone());
    }
    let reports = bugbot.review_diff(&diff).await;

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

    // 6. Post review comments to PR
    if !reports.is_empty() {
        let _ = bugbot.post_github_review(owner, repo, pr_number, &reports, head_sha).await;
    }

    // 7. Post final status check
    let summary = format!(
        "VibeCody found {} issue(s): {} critical, {} high, {} medium, {} low",
        reports.len(), counts.critical, counts.high, counts.medium, counts.low
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
    })
}

// ── GitHub API helpers ───────────────────────────────────────────────────────

/// Single source of truth for resolving a GitHub OAuth / PAT token.
///
/// Order (per AGENTS.md → Zero-Config First):
///   0. ProfileStore key `github` — `vibecli set-key github gh_pat_...`
///   1. `GITHUB_TOKEN` env var
///   2. `GH_TOKEN` env var (gh CLI compatibility)
///
/// Public so `bugbot.rs` and `vulnerability_db.rs` route through here
/// instead of each re-implementing their own env-only resolution.
pub fn resolve_github_token() -> Option<String> {
    if let Ok(store) = crate::profile_store::ProfileStore::new() {
        if let Ok(Some(t)) = store.get_api_key("default", "github") {
            if !t.is_empty() { return Some(t); }
        }
    }
    std::env::var("GITHUB_TOKEN").ok().filter(|s| !s.is_empty())
        .or_else(|| std::env::var("GH_TOKEN").ok().filter(|s| !s.is_empty()))
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
        anyhow::bail!("GitHub API returned {}: {}", resp.status(), resp.text().await.unwrap_or_default());
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
        eprintln!(
            "[github-app] Status check POST failed: {}",
            resp.status()
        );
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
    // Verify signature if webhook secret is configured
    if let Some(secret) = config.resolve_webhook_secret() {
        let sig = signature.unwrap_or("");
        if !verify_signature(&secret, payload, sig) {
            anyhow::bail!("Invalid webhook signature");
        }
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

    let pr = webhook.pull_request.ok_or_else(|| anyhow::anyhow!("Missing pull_request"))?;
    let repo = webhook.repository.ok_or_else(|| anyhow::anyhow!("Missing repository"))?;

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

    #[test]
    fn verify_valid_signature() {
        let secret = "test-secret";
        let payload = b"hello world";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());

        assert!(verify_signature(secret, payload, &format!("sha256={}", sig)));
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
        assert_eq!(counts.critical + counts.high + counts.medium + counts.low + counts.info, 0);
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
        assert!(verify_signature(secret, payload, &format!("sha256={}", sig)));
    }

    #[test]
    fn verify_signature_empty_payload() {
        let secret = "test";
        let payload = b"";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());
        assert!(verify_signature(secret, payload, &format!("sha256={}", sig)));
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
        assert_eq!(cfg.resolve_webhook_secret(), Some("inline-secret".to_string()));
    }

    #[test]
    fn severity_counts_individual_fields() {
        let mut counts = SeverityCounts::default();
        counts.critical = 1;
        counts.high = 2;
        counts.medium = 3;
        counts.low = 4;
        counts.info = 5;
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
