//! Security Posture — unified scanner aggregator + finding shape.
//!
//! See `docs/design/security-posture/README.md` for the full design.
//!
//! This module owns:
//! - The `SecurityFinding` shape every scanner emits
//! - The `Scanner` trait every scanner implements
//! - The `run_all_scanners` aggregator that the Tauri command layer
//!   calls
//! - The `WorkspaceStore`-backed suppression / goal-link / audit-log
//!   persistence
//!
//! It does NOT own:
//! - Any actual scanning logic — adapters live in
//!   `security_posture::adapters::*` and new scanners in
//!   `security_posture_secrets.rs` / `_license.rs` / `_taint.rs`
//! - The Tauri command wrappers — those live in
//!   `vibeui/src-tauri/src/commands.rs::security_posture_*`
//!
//! ## Threat-model invariants
//!
//! 1. Finding snippets are bounded (≤ 240 chars) and redacted at the
//!    scanner boundary — never carry secret bytes, exec-payload
//!    bytes, or unbounded model output. The `SecurityFinding`
//!    constructor enforces the length cap.
//! 2. The aggregator never sends finding content through an LLM
//!    without wrapping it in `Tainted<T>` — see Goals bridge in
//!    `security_posture_create_goal` for the named-fields-only
//!    pattern.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Max bytes carried in a finding snippet. Anything longer is
/// truncated with a trailing `…`. Bounded to keep the panel feed
/// fast and to make accidental secret bytes hard to fit.
pub const SNIPPET_MAX_BYTES: usize = 240;

/// Max rows kept in the decision log per workspace before FIFO
/// eviction. Sized so a single workspace's audit log fits in a
/// single WorkspaceStore setting row comfortably.
pub const DECISION_LOG_MAX_ROWS: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    /// Numeric weight for sorting / aggregating into the feed.
    /// Higher = more severe.
    pub fn weight(self) -> u8 {
        match self {
            Severity::Critical => 5,
            Severity::High => 4,
            Severity::Medium => 3,
            Severity::Low => 2,
            Severity::Info => 1,
        }
    }
}

/// Vocabulary mirrors the DREAD ledger (see `threat-model.md`)
/// applied to *user* code. New scanners may emit `Other(...)` for
/// classes not yet promoted to a top-level variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "label")]
pub enum Category {
    PromptInjection,
    PathTraversal,
    SecretLeak,
    DependencyCve,
    Sast,
    LicenseRisk,
    CodeHealth,
    Other(String),
}

impl Category {
    pub fn as_str(&self) -> &str {
        match self {
            Category::PromptInjection => "prompt_injection",
            Category::PathTraversal => "path_traversal",
            Category::SecretLeak => "secret_leak",
            Category::DependencyCve => "dependency_cve",
            Category::Sast => "sast",
            Category::LicenseRisk => "license_risk",
            Category::CodeHealth => "code_health",
            Category::Other(label) => label.as_str(),
        }
    }
}

/// Finding status — drives the panel's filter chip + colour coding,
/// and decides whether a finding still counts in the "open" total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FindingStatus {
    Open,
    Suppressed {
        reason: String,
        at_unix_ms: i64,
    },
    GoalLinked {
        goal_id: String,
        at_unix_ms: i64,
    },
    /// The scanner emitted the finding-id in a previous scan but the
    /// most recent scan no longer produces it. Surfaced for one cycle
    /// so the user knows their fix landed, then dropped.
    Fixed {
        at_unix_ms: i64,
    },
}

/// Scanner-agnostic finding. Every scanner converts its native
/// shape into this at the adapter boundary so the panel speaks
/// exactly one vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    /// 16-char hex SHA-256 prefix of
    /// (scanner | category | file | line | rule_id). Stable across
    /// scans for unchanged input, so suppression / goal-link state
    /// survives re-scans.
    pub id: String,

    pub severity: Severity,
    pub category: Category,
    /// Stable scanner identifier — `"vulnerability_db"`, `"sonar"`,
    /// `"health"`, `"secrets"`, `"license"`, `"taint"`.
    pub scanner: String,

    /// Workspace-relative path. The aggregator strips the workspace
    /// prefix at adapter time so storage / UI never carry absolute
    /// paths (the user may move their workspace).
    pub file: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,

    /// Bounded redaction-safe snippet — at most `SNIPPET_MAX_BYTES`
    /// bytes. The `SecurityFinding::new` constructor enforces the
    /// cap; constructing directly bypasses it (test-only path).
    pub snippet: Option<String>,

    /// Scanner-specific rule identifier. Convention:
    /// `CWE-22`, `OWASP-A03`, `vuln-db:RUSTSEC-2021-0001`,
    /// `sonar:rust:S6249`, etc.
    pub rule_id: String,
    pub title: String,
    pub remediation: Option<String>,
    pub references: Vec<String>,

    pub status: FindingStatus,
    pub first_seen_unix_ms: i64,
    pub last_seen_unix_ms: i64,
}

impl SecurityFinding {
    /// Bounded constructor — truncates `snippet` to
    /// [`SNIPPET_MAX_BYTES`], computes the stable `id` hash, and
    /// stamps `first_seen` / `last_seen` to `now`.
    ///
    /// The `id` hash is deterministic across scans: if the same
    /// scanner emits the same (category, file, line, rule_id) tuple
    /// in two separate runs, both records share an id. That's the
    /// invariant suppression / goal-link / audit-log all rely on.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scanner: impl Into<String>,
        severity: Severity,
        category: Category,
        file: impl Into<PathBuf>,
        line: Option<u32>,
        column: Option<u32>,
        snippet: Option<String>,
        rule_id: impl Into<String>,
        title: impl Into<String>,
        remediation: Option<String>,
        references: Vec<String>,
    ) -> Self {
        let scanner = scanner.into();
        let file = file.into();
        let rule_id = rule_id.into();
        let title = title.into();

        let id = Self::compute_id(&scanner, &category, &file, line, &rule_id);
        let now = unix_ms_now();
        let snippet = snippet.map(truncate_snippet);

        Self {
            id,
            severity,
            category,
            scanner,
            file,
            line,
            column,
            snippet,
            rule_id,
            title,
            remediation,
            references,
            status: FindingStatus::Open,
            first_seen_unix_ms: now,
            last_seen_unix_ms: now,
        }
    }

    /// Compute the stable finding id from its identifying tuple.
    /// Exposed so persistence layers can recompute / verify.
    pub fn compute_id(
        scanner: &str,
        category: &Category,
        file: &Path,
        line: Option<u32>,
        rule_id: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(scanner.as_bytes());
        hasher.update(b"|");
        hasher.update(category.as_str().as_bytes());
        hasher.update(b"|");
        hasher.update(file.to_string_lossy().as_bytes());
        hasher.update(b"|");
        hasher.update(line.unwrap_or(0).to_le_bytes());
        hasher.update(b"|");
        hasher.update(rule_id.as_bytes());
        let digest = hasher.finalize();
        hex_prefix(&digest, 8) // 8 bytes = 16 hex chars
    }
}

fn truncate_snippet(s: String) -> String {
    if s.len() <= SNIPPET_MAX_BYTES {
        return s;
    }
    // Truncate at a char boundary ≤ SNIPPET_MAX_BYTES - 3 to leave
    // room for the ellipsis (U+2026 = 3 bytes in UTF-8).
    // `floor_char_boundary` is unstable so we walk back manually.
    let mut cut = SNIPPET_MAX_BYTES - '…'.len_utf8();
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = s[..cut].to_string();
    out.push('…');
    out
}

fn hex_prefix(bytes: &[u8], n: usize) -> String {
    let mut s = String::with_capacity(n * 2);
    for b in bytes.iter().take(n) {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub(crate) fn unix_ms_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Test-visible accessor for `unix_ms_now` so the persistence layer
/// can stamp records using the same wall-clock source the
/// constructor uses. Production code calls the private fn directly.
pub fn unix_ms_now_for_tests() -> i64 {
    unix_ms_now()
}

// ── Scanner trait ───────────────────────────────────────────────────

/// Every scanner — adapter over an existing module, or a new
/// purpose-built scanner — implements this trait. Sync because the
/// existing scanners (`sonar_rules::scan_content`,
/// `health_score::HealthEngine::scan`, `vulnerability_db`) are all
/// sync; async wrapping happens at the Tauri command layer via
/// `tokio::task::spawn_blocking`.
pub trait Scanner: Send + Sync {
    /// Stable scanner name — used in `SecurityFinding.scanner` and
    /// in the UI scanner-filter chip. Must match the per-scanner
    /// adapter contract documented in `scanners.md`.
    fn name(&self) -> &'static str;

    /// Run a full scan over `workspace`. Returns the unified
    /// findings, or a scanner-local error that the aggregator
    /// surfaces in the per-scanner error chip without blocking the
    /// rest of the run.
    fn scan(&self, workspace: &Path) -> Result<Vec<SecurityFinding>>;
}

// ── Decision log ─────────────────────────────────────────────────────

/// One row in the per-workspace audit log. Append-only, capped at
/// [`DECISION_LOG_MAX_ROWS`] with FIFO eviction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionLogEntry {
    pub at_unix_ms: i64,
    pub finding_id: String,
    pub operation: DecisionOperation,
    /// Free-text rationale captured at the time of the decision.
    /// Only populated for `Suppress` operations (the suppression
    /// modal requires a reason); other operations leave it `None`.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionOperation {
    Suppress,
    Unsuppress,
    LinkGoal,
    UnlinkGoal,
    /// A future scan no longer produced this finding — surfaced as
    /// a `Fixed { at }` status for one cycle then dropped.
    AutoResolved,
}

// ── Aggregator ───────────────────────────────────────────────────────

/// Run every supplied scanner over `workspace`, collect findings,
/// merge with persisted suppression / goal-link state, sort by
/// severity descending. Per-scanner errors are surfaced via the
/// returned `errors` map; one scanner failing never blocks the
/// rest.
pub fn run_all_scanners(
    workspace: &Path,
    scanners: &[Box<dyn Scanner>],
) -> AggregatorResult {
    let mut findings: Vec<SecurityFinding> = Vec::new();
    let mut errors: Vec<ScannerError> = Vec::new();

    for scanner in scanners {
        match scanner.scan(workspace) {
            Ok(mut batch) => findings.append(&mut batch),
            Err(e) => errors.push(ScannerError {
                scanner: scanner.name().to_string(),
                message: e.to_string(),
            }),
        }
    }

    findings.sort_by(|a, b| {
        b.severity
            .weight()
            .cmp(&a.severity.weight())
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.unwrap_or(0).cmp(&b.line.unwrap_or(0)))
    });

    AggregatorResult { findings, errors }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorResult {
    pub findings: Vec<SecurityFinding>,
    /// Per-scanner errors. Surfaced as banners at the top of the
    /// panel feed; never block the findings from other scanners.
    pub errors: Vec<ScannerError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerError {
    pub scanner: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_truncation_under_cap_unchanged() {
        let f = SecurityFinding::new(
            "test",
            Severity::Low,
            Category::CodeHealth,
            "src/lib.rs",
            Some(1),
            None,
            Some("short snippet".to_string()),
            "TEST-1",
            "title",
            None,
            vec![],
        );
        assert_eq!(f.snippet, Some("short snippet".to_string()));
    }

    #[test]
    fn snippet_truncation_over_cap_ellipsis() {
        let long = "x".repeat(SNIPPET_MAX_BYTES + 50);
        let f = SecurityFinding::new(
            "test",
            Severity::Low,
            Category::CodeHealth,
            "src/lib.rs",
            Some(1),
            None,
            Some(long),
            "TEST-1",
            "title",
            None,
            vec![],
        );
        let s = f.snippet.unwrap();
        assert!(s.ends_with('…'), "expected trailing ellipsis: {s}");
        assert!(s.len() <= SNIPPET_MAX_BYTES);
    }

    #[test]
    fn snippet_truncation_respects_utf8_boundary() {
        // 4-byte chars that would straddle the cap if naively sliced.
        let s = "🦀".repeat(SNIPPET_MAX_BYTES);
        let f = SecurityFinding::new(
            "test",
            Severity::Low,
            Category::CodeHealth,
            "src/lib.rs",
            Some(1),
            None,
            Some(s),
            "TEST-1",
            "title",
            None,
            vec![],
        );
        // If the truncation cut mid-char, this would panic during
        // serialize / display. The cap-walk-back ensures it doesn't.
        let truncated = f.snippet.unwrap();
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn id_stable_across_construction() {
        // Two findings with the same identifying tuple must hash to
        // the same id — that's the invariant suppression / goal-link
        // depend on. (first_seen / last_seen differ; id doesn't.)
        let f1 = SecurityFinding::new(
            "secrets",
            Severity::Critical,
            Category::SecretLeak,
            "src/api/keys.rs",
            Some(42),
            None,
            None,
            "secret-leak:aws-access-key",
            "AWS access key",
            None,
            vec![],
        );
        let f2 = SecurityFinding::new(
            "secrets",
            Severity::Critical,
            Category::SecretLeak,
            "src/api/keys.rs",
            Some(42),
            None,
            None,
            "secret-leak:aws-access-key",
            "AWS access key",
            None,
            vec![],
        );
        assert_eq!(f1.id, f2.id, "stable id required for suppression to survive re-scans");
        assert_eq!(f1.id.len(), 16, "id should be 16 hex chars");
    }

    #[test]
    fn id_differs_when_line_differs() {
        let f1 = SecurityFinding::new(
            "secrets", Severity::Critical, Category::SecretLeak,
            "src/api/keys.rs", Some(42), None, None,
            "secret-leak:aws-access-key", "AWS access key", None, vec![],
        );
        let f2 = SecurityFinding::new(
            "secrets", Severity::Critical, Category::SecretLeak,
            "src/api/keys.rs", Some(43), None, None,
            "secret-leak:aws-access-key", "AWS access key", None, vec![],
        );
        assert_ne!(f1.id, f2.id);
    }

    #[test]
    fn id_differs_when_scanner_differs() {
        // Same location, same rule_id, different scanner — they're
        // different findings (the user should be able to suppress
        // one without the other).
        let f1 = SecurityFinding::new(
            "secrets", Severity::Critical, Category::SecretLeak,
            "src/api/keys.rs", Some(42), None, None,
            "RULE", "t", None, vec![],
        );
        let f2 = SecurityFinding::new(
            "vulnerability_db", Severity::Critical, Category::SecretLeak,
            "src/api/keys.rs", Some(42), None, None,
            "RULE", "t", None, vec![],
        );
        assert_ne!(f1.id, f2.id);
    }

    #[test]
    fn severity_weight_orders_correctly() {
        assert!(Severity::Critical.weight() > Severity::High.weight());
        assert!(Severity::High.weight() > Severity::Medium.weight());
        assert!(Severity::Medium.weight() > Severity::Low.weight());
        assert!(Severity::Low.weight() > Severity::Info.weight());
    }

    struct StubScanner {
        name: &'static str,
        findings: Vec<SecurityFinding>,
        fail: bool,
    }

    impl Scanner for StubScanner {
        fn name(&self) -> &'static str {
            self.name
        }
        fn scan(&self, _workspace: &Path) -> Result<Vec<SecurityFinding>> {
            if self.fail {
                anyhow::bail!("stub failure")
            } else {
                Ok(self.findings.clone())
            }
        }
    }

    fn fixture_finding(scanner: &str, severity: Severity, line: u32) -> SecurityFinding {
        SecurityFinding::new(
            scanner.to_string(),
            severity,
            Category::Sast,
            "src/x.rs",
            Some(line),
            None,
            None,
            "RULE-1",
            "fixture",
            None,
            vec![],
        )
    }

    #[test]
    fn aggregator_sorts_critical_first() {
        let s1: Box<dyn Scanner> = Box::new(StubScanner {
            name: "a",
            findings: vec![
                fixture_finding("a", Severity::Low, 1),
                fixture_finding("a", Severity::Critical, 2),
            ],
            fail: false,
        });
        let result = run_all_scanners(Path::new("/tmp"), &[s1]);
        assert_eq!(result.findings.len(), 2);
        assert_eq!(result.findings[0].severity, Severity::Critical);
        assert_eq!(result.findings[1].severity, Severity::Low);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn aggregator_continues_on_scanner_failure() {
        let s_ok: Box<dyn Scanner> = Box::new(StubScanner {
            name: "ok",
            findings: vec![fixture_finding("ok", Severity::High, 1)],
            fail: false,
        });
        let s_fail: Box<dyn Scanner> = Box::new(StubScanner {
            name: "fail",
            findings: vec![],
            fail: true,
        });
        let result = run_all_scanners(Path::new("/tmp"), &[s_ok, s_fail]);
        assert_eq!(result.findings.len(), 1, "ok scanner's findings must survive");
        assert_eq!(result.errors.len(), 1, "failure must be surfaced");
        assert_eq!(result.errors[0].scanner, "fail");
        assert!(result.errors[0].message.contains("stub failure"));
    }

    #[test]
    fn aggregator_empty_when_no_scanners() {
        let result = run_all_scanners(Path::new("/tmp"), &[]);
        assert!(result.findings.is_empty());
        assert!(result.errors.is_empty());
    }
}
