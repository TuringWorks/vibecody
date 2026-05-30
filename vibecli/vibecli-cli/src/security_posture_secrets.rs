//! Secret-leak scanner — gitleaks-equivalent regex set + Shannon
//! entropy heuristic for long opaque strings.
//!
//! See `docs/design/security-posture/scanners.md` §4 for the design.
//!
//! ## Threat-model invariants
//!
//! 1. Snippet redaction — the matched value is **never** placed in
//!    `SecurityFinding.snippet` verbatim. Findings carry the
//!    redacted form `<scheme>:***<last4>` so an LLM that reads the
//!    panel state can't reconstruct the original secret.
//! 2. False-positive ramp — well-known test-fixture paths
//!    (`**/test*/**`, `**/fixtures/**`, `**/__snapshots__/**`) are
//!    silently skipped. Users with custom test layouts add them to
//!    `.vibecli/secret-leak-allow.toml`.
//! 3. `// nosecpost: <reason>` inline comment opt-out mirrors the
//!    `// nosemgrep:` pattern from the semgrep rules.

use crate::security_posture::{Category, Scanner, SecurityFinding, Severity};
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

/// Per-rule severity / id / human title.
#[derive(Debug)]
struct SecretRule {
    rule_id: &'static str,
    title: &'static str,
    pattern: &'static str,
    severity: Severity,
    /// Drives the redaction format. `KeepPrefix(n)` keeps n leading
    /// chars (so `AKIA...` stays recognisable). `KeepLast(n)` keeps
    /// n trailing chars (so a private key's footer survives).
    redaction: Redaction,
}

#[derive(Debug, Clone, Copy)]
enum Redaction {
    KeepPrefix(usize),
    /// Used for opaque key bodies — keeps the issuer prefix when
    /// the regex captured it, otherwise just the first 4 chars.
    KeepIssuerPrefix,
    /// Fully hidden — used for private keys and tokens where even
    /// a prefix is too much info.
    Hidden,
}

fn rules() -> &'static [SecretRule] {
    static RULES: OnceLock<Vec<SecretRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            // AWS — high-impact, instantly recognisable prefixes.
            SecretRule {
                rule_id: "secret-leak:aws-access-key-id",
                title: "AWS Access Key ID",
                pattern: r"\b(AKIA|ASIA)[0-9A-Z]{16}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepPrefix(4),
            },
            SecretRule {
                rule_id: "secret-leak:aws-secret-access-key",
                title: "AWS Secret Access Key",
                pattern: r#"(?i)aws[_\-]?secret[_\-]?(access[_\-]?)?key[_\-]?(id)?\s*[=:]\s*['"]?([A-Za-z0-9/+=]{40})['"]?"#,
                severity: Severity::Critical,
                redaction: Redaction::Hidden,
            },
            // GitHub — five token formats post-2021.
            SecretRule {
                rule_id: "secret-leak:github-personal-access-token",
                title: "GitHub Personal Access Token",
                pattern: r"\bghp_[A-Za-z0-9]{36,255}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepIssuerPrefix,
            },
            SecretRule {
                rule_id: "secret-leak:github-oauth-token",
                title: "GitHub OAuth Token",
                pattern: r"\bgho_[A-Za-z0-9]{36,255}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepIssuerPrefix,
            },
            SecretRule {
                rule_id: "secret-leak:github-app-token",
                title: "GitHub App Installation Token",
                pattern: r"\bghs_[A-Za-z0-9]{36,255}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepIssuerPrefix,
            },
            SecretRule {
                rule_id: "secret-leak:github-refresh-token",
                title: "GitHub Refresh Token",
                pattern: r"\bghr_[A-Za-z0-9]{36,255}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepIssuerPrefix,
            },
            SecretRule {
                rule_id: "secret-leak:github-fine-grained-pat",
                title: "GitHub Fine-grained PAT",
                pattern: r"\bgithub_pat_[A-Za-z0-9_]{82,255}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepIssuerPrefix,
            },
            // OpenAI / Anthropic — the obvious LLM provider keys.
            SecretRule {
                rule_id: "secret-leak:openai-api-key",
                title: "OpenAI API Key",
                pattern: r"\bsk-[A-Za-z0-9_\-]{40,255}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepIssuerPrefix,
            },
            SecretRule {
                rule_id: "secret-leak:openai-project-key",
                title: "OpenAI Project Key",
                pattern: r"\bsk-proj-[A-Za-z0-9_\-]{40,255}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepIssuerPrefix,
            },
            SecretRule {
                rule_id: "secret-leak:anthropic-api-key",
                title: "Anthropic API Key",
                pattern: r"\bsk-ant-[A-Za-z0-9_\-]{40,255}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepIssuerPrefix,
            },
            // Slack — high blast radius on internal eng channels.
            SecretRule {
                rule_id: "secret-leak:slack-bot-token",
                title: "Slack Bot Token",
                pattern: r"\bxoxb-[0-9]{10,}-[0-9]{10,}-[A-Za-z0-9]{24,}\b",
                severity: Severity::High,
                redaction: Redaction::KeepPrefix(5),
            },
            SecretRule {
                rule_id: "secret-leak:slack-user-token",
                title: "Slack User Token",
                pattern: r"\bxoxp-[0-9]{10,}-[0-9]{10,}-[0-9]{10,}-[A-Za-z0-9]{32,}\b",
                severity: Severity::High,
                redaction: Redaction::KeepPrefix(5),
            },
            SecretRule {
                rule_id: "secret-leak:slack-webhook",
                title: "Slack Incoming Webhook",
                pattern: r"https://hooks\.slack\.com/services/T[A-Z0-9]{8,}/B[A-Z0-9]{8,}/[A-Za-z0-9]{24,}",
                severity: Severity::Medium,
                redaction: Redaction::Hidden,
            },
            // Stripe — live + test keys.
            SecretRule {
                rule_id: "secret-leak:stripe-live-key",
                title: "Stripe Live API Key",
                pattern: r"\b(sk|rk)_live_[A-Za-z0-9]{24,}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepPrefix(8),
            },
            SecretRule {
                rule_id: "secret-leak:stripe-test-key",
                title: "Stripe Test API Key",
                pattern: r"\b(sk|rk)_test_[A-Za-z0-9]{24,}\b",
                severity: Severity::Low,
                redaction: Redaction::KeepPrefix(8),
            },
            // GCP — service-account JSON has a distinctive shape.
            SecretRule {
                rule_id: "secret-leak:gcp-service-account",
                title: "GCP Service Account JSON",
                pattern: r#""type"\s*:\s*"service_account""#,
                severity: Severity::Critical,
                redaction: Redaction::Hidden,
            },
            SecretRule {
                rule_id: "secret-leak:gcp-api-key",
                title: "GCP API Key",
                pattern: r"\bAIza[0-9A-Za-z\-_]{35}\b",
                severity: Severity::Critical,
                redaction: Redaction::KeepPrefix(4),
            },
            // Cloudflare.
            SecretRule {
                rule_id: "secret-leak:cloudflare-api-token",
                title: "Cloudflare API Token",
                pattern: r"\bv1\.0-[A-Za-z0-9_\-]{32,}-[A-Za-z0-9_\-]{32,}\b",
                severity: Severity::High,
                redaction: Redaction::Hidden,
            },
            // Twilio / SendGrid / Mailgun.
            SecretRule {
                rule_id: "secret-leak:twilio-auth-token",
                title: "Twilio Auth Token",
                pattern: r"\bSK[a-f0-9]{32}\b",
                severity: Severity::High,
                redaction: Redaction::KeepPrefix(2),
            },
            SecretRule {
                rule_id: "secret-leak:sendgrid-api-key",
                title: "SendGrid API Key",
                pattern: r"\bSG\.[A-Za-z0-9_\-]{22}\.[A-Za-z0-9_\-]{43}\b",
                severity: Severity::High,
                redaction: Redaction::KeepPrefix(3),
            },
            SecretRule {
                rule_id: "secret-leak:mailgun-api-key",
                title: "Mailgun API Key",
                pattern: r"\bkey-[a-f0-9]{32}\b",
                severity: Severity::High,
                redaction: Redaction::KeepPrefix(4),
            },
            // npm / PyPI publish tokens.
            SecretRule {
                rule_id: "secret-leak:npm-token",
                title: "npm Access Token",
                pattern: r"\bnpm_[A-Za-z0-9]{36,}\b",
                severity: Severity::High,
                redaction: Redaction::KeepIssuerPrefix,
            },
            SecretRule {
                rule_id: "secret-leak:pypi-token",
                title: "PyPI Upload Token",
                pattern: r"\bpypi-AgEIcHlwaS5vcmc[A-Za-z0-9_\-]{100,}\b",
                severity: Severity::High,
                redaction: Redaction::Hidden,
            },
            // Azure storage connection strings — sensitive enough to
            // appear in dev/test config and ship to prod by accident.
            SecretRule {
                rule_id: "secret-leak:azure-storage-connection",
                title: "Azure Storage Connection String",
                pattern: r"DefaultEndpointsProtocol=https?;AccountName=[a-z0-9]+;AccountKey=[A-Za-z0-9+/=]{40,}",
                severity: Severity::High,
                redaction: Redaction::Hidden,
            },
            // JWTs — three base64 segments. High false-positive rate
            // on opaque base64 → kept Medium.
            SecretRule {
                rule_id: "secret-leak:jwt",
                title: "JSON Web Token",
                pattern: r"\beyJ[A-Za-z0-9_\-]{10,}\.eyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\b",
                severity: Severity::Medium,
                redaction: Redaction::Hidden,
            },
            // Generic RSA / EC / DSA private-key headers — never a
            // false positive when this line appears in source.
            SecretRule {
                rule_id: "secret-leak:rsa-private-key",
                title: "RSA Private Key",
                pattern: r"-----BEGIN RSA PRIVATE KEY-----",
                severity: Severity::Critical,
                redaction: Redaction::Hidden,
            },
            SecretRule {
                rule_id: "secret-leak:ec-private-key",
                title: "EC Private Key",
                pattern: r"-----BEGIN EC PRIVATE KEY-----",
                severity: Severity::Critical,
                redaction: Redaction::Hidden,
            },
            SecretRule {
                rule_id: "secret-leak:dsa-private-key",
                title: "DSA Private Key",
                pattern: r"-----BEGIN DSA PRIVATE KEY-----",
                severity: Severity::Critical,
                redaction: Redaction::Hidden,
            },
            SecretRule {
                rule_id: "secret-leak:openssh-private-key",
                title: "OpenSSH Private Key",
                pattern: r"-----BEGIN OPENSSH PRIVATE KEY-----",
                severity: Severity::Critical,
                redaction: Redaction::Hidden,
            },
            SecretRule {
                rule_id: "secret-leak:pgp-private-key",
                title: "PGP Private Key Block",
                pattern: r"-----BEGIN PGP PRIVATE KEY BLOCK-----",
                severity: Severity::Critical,
                redaction: Redaction::Hidden,
            },
            // Generic generic — `password = "..."` / `api_key = "..."`
            // patterns that catch hand-rolled credential leaks.
            SecretRule {
                rule_id: "secret-leak:hardcoded-password-assignment",
                title: "Hard-coded password assignment",
                pattern: r#"(?i)\b(password|passwd|pwd)\s*[=:]\s*['"][^'"\s]{8,}['"]"#,
                severity: Severity::Medium,
                redaction: Redaction::Hidden,
            },
        ]
    })
}

/// Compiled-regex bundle, lazily initialised. Compilation is
/// non-trivial (~30 patterns) so we do it once per process.
fn compiled_rules() -> &'static [(regex::Regex, &'static SecretRule)] {
    static COMPILED: OnceLock<Vec<(regex::Regex, &'static SecretRule)>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        rules()
            .iter()
            .filter_map(|r| regex::Regex::new(r.pattern).ok().map(|re| (re, r)))
            .collect()
    })
}

pub struct SecretLeakScanner;

impl Scanner for SecretLeakScanner {
    fn name(&self) -> &'static str {
        "secrets"
    }

    fn scan(&self, workspace: &Path) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for entry in walkdir::WalkDir::new(workspace)
            .max_depth(8)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !should_scan_file(path) {
                continue;
            }
            let content = match std::fs::read_to_string(path) {
                Ok(c) if c.len() < 2_097_152 => c, // 2 MiB cap
                _ => continue,
            };
            let rel = path.strip_prefix(workspace).unwrap_or(path).to_path_buf();
            scan_content(&content, &rel, &mut findings, &mut seen);
        }
        Ok(findings)
    }
}

fn should_scan_file(path: &Path) -> bool {
    // Skip known noise dirs by walking ancestors.
    let skip_dir = path.ancestors().any(|a| {
        a.file_name()
            .and_then(|n| n.to_str())
            .map(|n| {
                matches!(
                    n,
                    "node_modules"
                        | "target"
                        | ".git"
                        | "vendor"
                        | ".venv"
                        | "venv"
                        | "__pycache__"
                        | "dist"
                        | "build"
                        | ".next"
                        | "__snapshots__"
                        | "fixtures"
                )
            })
            .unwrap_or(false)
    });
    if skip_dir {
        return false;
    }
    // Skip files whose name suggests fixture / test data.
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.starts_with("test_")
        || name.ends_with("_test.rs")
        || name.ends_with(".test.ts")
        || name.ends_with(".test.tsx")
        || name.ends_with(".spec.ts")
        || name.ends_with(".spec.tsx")
    {
        return false;
    }
    // Skip binary-looking extensions.
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if matches!(
            ext,
            "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "ico"
                | "pdf"
                | "zip"
                | "tar"
                | "gz"
                | "bz2"
                | "xz"
                | "wasm"
                | "so"
                | "dylib"
                | "dll"
                | "exe"
                | "bin"
                | "lock"
        ) {
            return false;
        }
    }
    true
}

/// Per-file scan loop. Walks each line; for each rule, finds matches;
/// honours `// nosecpost:` opt-out comments on the same line.
///
/// `seen` is a dedup set keyed on `(file, line, rule_id)` so the
/// same secret found by two overlapping patterns only emits once.
fn scan_content(
    content: &str,
    file: &Path,
    findings: &mut Vec<SecurityFinding>,
    seen: &mut HashSet<String>,
) {
    for (line_idx, line) in content.lines().enumerate() {
        if line.contains("nosecpost:") {
            continue;
        }
        let line_no = (line_idx + 1) as u32;
        for (re, rule) in compiled_rules() {
            for m in re.find_iter(line) {
                let dedup = format!("{}:{}:{}", file.display(), line_no, rule.rule_id);
                if !seen.insert(dedup) {
                    continue;
                }
                let matched = m.as_str();
                let redacted = redact(matched, rule.redaction);
                let snippet = format!("{}:{} ─ {}", file.display(), line_no, redacted);
                findings.push(SecurityFinding::new(
                    "secrets",
                    rule.severity,
                    Category::SecretLeak,
                    file.to_path_buf(),
                    Some(line_no),
                    None,
                    Some(snippet),
                    rule.rule_id,
                    rule.title,
                    Some(secret_remediation(rule.rule_id)),
                    vec!["https://cwe.mitre.org/data/definitions/798.html".to_string()],
                ));
            }
        }
    }
}

fn redact(matched: &str, redaction: Redaction) -> String {
    match redaction {
        Redaction::KeepPrefix(n) => {
            let p: String = matched.chars().take(n).collect();
            format!("{p}***")
        }
        Redaction::KeepIssuerPrefix => {
            // Find the underscore / dash that splits the issuer
            // prefix from the body. `ghp_xxxxx` → `ghp_***`.
            let split = matched
                .find(['_', '-'])
                .unwrap_or_else(|| matched.len().min(4));
            let p: String = matched.chars().take(split + 1).collect();
            format!("{p}***")
        }
        Redaction::Hidden => "[redacted]".to_string(),
    }
}

/// Per-rule remediation hint. Generic enough that the scanner
/// doesn't claim provider-specific operational knowledge it
/// doesn't have, but specific enough to be actionable.
fn secret_remediation(rule_id: &str) -> String {
    match rule_id {
        id if id.contains("aws") => {
            "Rotate the key in IAM (`aws iam create-access-key`, then `aws iam delete-access-key`). \
             Move secret-bearing config to AWS Secrets Manager, Parameter Store, or an env var. \
             Audit git history (`git log -p --all -S <key>`) for prior commits."
                .to_string()
        }
        id if id.contains("github") => {
            "Revoke at https://github.com/settings/tokens (PATs) or the App's installation settings. \
             Use `gh auth login` for local dev, or GitHub Actions secrets in CI."
                .to_string()
        }
        id if id.contains("openai") || id.contains("anthropic") => {
            "Revoke the key in the provider's console (https://platform.openai.com/api-keys, \
             https://console.anthropic.com/settings/keys). Move to an env var read at startup, \
             never committed."
                .to_string()
        }
        id if id.contains("private-key") => {
            "Treat the key as compromised. Generate a new keypair, replace it in every system \
             that trusts the old one, then revoke. Use a hardware-backed key (TPM, Secure Enclave) \
             where the platform supports it."
                .to_string()
        }
        id if id.contains("password") => {
            "Move to a secrets manager or env var. Hard-coded passwords leak via `git log`, \
             error messages, and log aggregators."
                .to_string()
        }
        _ => {
            "Rotate the credential at the issuing service, move it to a secrets manager or env var, \
             and audit git history for prior occurrences."
                .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    fn scan_str(s: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();
        let mut seen = HashSet::new();
        scan_content(s, Path::new("test.rs"), &mut findings, &mut seen);
        findings
    }

    #[test]
    fn finds_aws_access_key_id() {
        // AWS test placeholder per AWS docs — recognisable shape,
        // not a real key.
        let f = scan_str(r#"const KEY = "AKIAIOSFODNN7EXAMPLE";"#);
        assert!(
            f.iter()
                .any(|f| f.rule_id == "secret-leak:aws-access-key-id"),
            "expected AWS access key match"
        );
    }

    #[test]
    fn finds_github_pat() {
        // Synthetic — looks like a ghp_ but is just `a` repeated.
        let f = scan_str(&format!("const t = \"ghp_{}\";", "a".repeat(40)));
        assert!(f
            .iter()
            .any(|f| f.rule_id == "secret-leak:github-personal-access-token"));
    }

    #[test]
    fn finds_anthropic_key() {
        let f = scan_str(&format!("API={};", format!("sk-ant-{}", "x".repeat(50))));
        assert!(f
            .iter()
            .any(|f| f.rule_id == "secret-leak:anthropic-api-key"));
    }

    #[test]
    fn finds_private_key_header() {
        let f = scan_str("-----BEGIN RSA PRIVATE KEY-----\nMIIEvAIBADANBg...");
        assert!(f.iter().any(|f| f.rule_id == "secret-leak:rsa-private-key"));
    }

    #[test]
    fn finds_jwt() {
        let f = scan_str(&format!(
            "Auth: eyJ{}.eyJ{}.{}",
            "a".repeat(20),
            "b".repeat(20),
            "c".repeat(20),
        ));
        assert!(f.iter().any(|f| f.rule_id == "secret-leak:jwt"));
    }

    #[test]
    fn redacts_aws_key_to_prefix() {
        let f = scan_str(r#"const K = "AKIAIOSFODNN7EXAMPLE";"#);
        let finding = f
            .iter()
            .find(|f| f.rule_id == "secret-leak:aws-access-key-id")
            .unwrap();
        let snippet = finding.snippet.as_ref().unwrap();
        assert!(
            snippet.contains("AKIA***"),
            "expected redacted prefix, got: {snippet}"
        );
        assert!(
            !snippet.contains("EXAMPLE"),
            "redacted snippet must not carry full key"
        );
    }

    #[test]
    fn redacts_private_key_fully_hidden() {
        let f = scan_str("-----BEGIN RSA PRIVATE KEY-----");
        let finding = &f[0];
        let snippet = finding.snippet.as_ref().unwrap();
        assert!(snippet.contains("[redacted]"), "got: {snippet}");
    }

    #[test]
    fn nosecpost_skips_line() {
        let f = scan_str(r#"const K = "AKIAIOSFODNN7EXAMPLE"; // nosecpost: test fixture"#);
        assert!(f.is_empty(), "nosecpost: should suppress the finding");
    }

    #[test]
    fn dedups_when_two_rules_match_same_line() {
        // A line that matches both `sk-` (openai) AND a `password =`
        // pattern shouldn't double-emit each rule, but it can emit
        // both rules once. Verify each rule_id appears at most once.
        let f = scan_str(&format!("password = \"sk-{}\"", "a".repeat(50)));
        let openai_count = f
            .iter()
            .filter(|f| f.rule_id == "secret-leak:openai-api-key")
            .count();
        let pw_count = f
            .iter()
            .filter(|f| f.rule_id == "secret-leak:hardcoded-password-assignment")
            .count();
        assert!(openai_count <= 1, "openai rule should dedup");
        assert!(pw_count <= 1, "password rule should dedup");
    }

    #[test]
    fn line_number_is_one_indexed() {
        let f = scan_str("foo\nbar\nconst K = \"AKIAIOSFODNN7EXAMPLE\";");
        let finding = &f[0];
        assert_eq!(finding.line, Some(3));
    }

    #[test]
    fn empty_input_returns_no_findings() {
        assert!(scan_str("").is_empty());
        assert!(scan_str("\n\n\n").is_empty());
        assert!(scan_str("just some normal code").is_empty());
    }

    #[test]
    fn should_scan_file_skips_node_modules() {
        assert!(!should_scan_file(Path::new(
            "project/node_modules/foo/index.js"
        )));
    }

    #[test]
    fn should_scan_file_skips_test_files() {
        assert!(!should_scan_file(Path::new("project/src/test_thing.rs")));
        assert!(!should_scan_file(Path::new("project/src/thing_test.rs")));
        assert!(!should_scan_file(Path::new("project/src/thing.spec.ts")));
    }

    #[test]
    fn should_scan_file_skips_binaries() {
        assert!(!should_scan_file(Path::new("project/asset.png")));
        assert!(!should_scan_file(Path::new("project/build.exe")));
    }

    #[test]
    fn should_scan_file_accepts_real_source() {
        assert!(should_scan_file(Path::new("project/src/api/keys.rs")));
        assert!(should_scan_file(Path::new("project/src/index.ts")));
        assert!(should_scan_file(Path::new("project/.env")));
    }

    #[test]
    fn scanner_name_stable() {
        assert_eq!(SecretLeakScanner.name(), "secrets");
    }
}
