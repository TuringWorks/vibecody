//! Evidence-based compliance scanning of a project workspace.
//!
//! The compliance report used to be a fixed inventory of VibeCody's *own*
//! security features, returned verbatim no matter which project was open —
//! and any framework outside SOC 2 / FedRAMP fell through to two placeholder
//! controls that scored 100%. This module replaces that with an actual scan:
//! walk the workspace, collect [`Signal`]s backed by real file paths and line
//! numbers, then score a framework's control catalogue against what was found.
//!
//! Two rules shape the design:
//!
//! - **Absent evidence stays absent.** A control with no matching signal is a
//!   gap, never an assumed pass. The scanner reports the file it saw, not the
//!   control it hoped for.
//! - **"Cannot be assessed from source" is its own verdict.** Personnel
//!   screening, vendor contracts and physical access leave no trace in a
//!   repository; they are reported as [`ControlStatus::NotAssessed`] and kept
//!   out of the score's denominator, so the percentage describes only what was
//!   actually measured.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::compliance::{ComplianceControl, ComplianceFramework, ControlStatus};
use crate::proactive_scanner::discover_files;

// ── Scan budget ─────────────────────────────────────────────────────────────
// A scan runs on whatever directory the user has open, so every bound here is
// about a repository we know nothing about: read no single huge file, read no
// unbounded number of them, and say so when a bound is hit rather than
// reporting a partial scan as if it were complete.

/// Largest single file the scanner will read into memory.
const MAX_FILE_BYTES: u64 = 512 * 1024;
/// Largest number of files whose contents are read.
const MAX_FILES_READ: usize = 8_000;
/// Total bytes read across the whole scan.
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
/// Evidence entries retained per signal (the count keeps rising).
const MAX_EVIDENCE_PER_SIGNAL: usize = 8;
/// Evidence entries shown per control in the report.
const MAX_EVIDENCE_PER_CONTROL: usize = 6;

// ── Signals ─────────────────────────────────────────────────────────────────

/// A single observable fact about a project. Signals are deliberately narrow:
/// each one is something a reviewer could confirm by opening the cited file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Signal {
    License,
    CodeOwners,
    ContributionGuide,
    SecurityPolicy,
    ProjectDocs,
    ChangeLog,
    ThreatModel,
    IncidentRunbook,
    PrivacyNotice,
    DataInventory,
    BackupProcedure,
    CiPipeline,
    ReviewGate,
    AutomatedTests,
    DependencyLockfile,
    DependencyAudit,
    SecretScanning,
    StaticAnalysis,
    IacDefinitions,
    ContainerNonRoot,
    EnvExample,
    SecretsManager,
    CommittedSecretFile,
    HardcodedCredential,
    Authentication,
    Authorization,
    MultiFactor,
    SessionManagement,
    EncryptionInTransit,
    EncryptionAtRest,
    Logging,
    AuditLogging,
    LogRedaction,
    Telemetry,
    RateLimiting,
    InputValidation,
    DataRetention,
    PiiRedaction,
    ConsentTracking,
    DataErasure,
    DataPortability,
}

impl Signal {
    /// Human phrase used when reporting a signal as present or missing.
    pub fn describe(&self) -> &'static str {
        match self {
            Signal::License => "a license file",
            Signal::CodeOwners => "a CODEOWNERS file",
            Signal::ContributionGuide => "a contribution guide or code of conduct",
            Signal::SecurityPolicy => "a security policy (SECURITY.md)",
            Signal::ProjectDocs => "project documentation (README or docs/)",
            Signal::ChangeLog => "a changelog or release notes",
            Signal::ThreatModel => "a threat model or risk assessment document",
            Signal::IncidentRunbook => "an incident-response runbook",
            Signal::PrivacyNotice => "a privacy notice",
            Signal::DataInventory => "a data inventory / records of processing",
            Signal::BackupProcedure => "a backup or restore procedure",
            Signal::CiPipeline => "a CI pipeline definition",
            Signal::ReviewGate => "a pull-request review template",
            Signal::AutomatedTests => "automated tests",
            Signal::DependencyLockfile => "a dependency lockfile",
            Signal::DependencyAudit => "automated dependency vulnerability scanning",
            Signal::SecretScanning => "secret scanning",
            Signal::StaticAnalysis => "static analysis / linting",
            Signal::IacDefinitions => "infrastructure-as-code definitions",
            Signal::ContainerNonRoot => "a container image that drops to a non-root user",
            Signal::EnvExample => "an environment template (.env.example)",
            Signal::SecretsManager => "an external secrets manager",
            Signal::CommittedSecretFile => "a credential file committed to version control",
            Signal::HardcodedCredential => "a hard-coded credential literal",
            Signal::Authentication => "authentication code",
            Signal::Authorization => "authorization / role checks",
            Signal::MultiFactor => "multi-factor authentication",
            Signal::SessionManagement => "session expiry or revocation",
            Signal::EncryptionInTransit => "TLS / transport encryption",
            Signal::EncryptionAtRest => "encryption at rest",
            Signal::Logging => "application logging",
            Signal::AuditLogging => "an audit log of security-relevant actions",
            Signal::LogRedaction => "redaction of secrets in logs",
            Signal::Telemetry => "metrics / monitoring instrumentation",
            Signal::RateLimiting => "rate limiting",
            Signal::InputValidation => "input validation",
            Signal::DataRetention => "a data-retention policy in code or config",
            Signal::PiiRedaction => "PII redaction or pseudonymisation",
            Signal::ConsentTracking => "consent tracking",
            Signal::DataErasure => "a data-erasure path (account deletion)",
            Signal::DataPortability => "a data-export path",
        }
    }

    /// True when finding this signal is a *problem*, not evidence of a control.
    pub fn is_finding(&self) -> bool {
        matches!(
            self,
            Signal::CommittedSecretFile | Signal::HardcodedCredential
        )
    }
}

/// One observation: which file, which line, and what was recognised there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub path: String,
    pub line: Option<u32>,
    pub detail: &'static str,
}

impl Evidence {
    fn render(&self) -> String {
        match self.line {
            Some(n) => format!("{}:{} ({})", self.path, n, self.detail),
            None => format!("{} ({})", self.path, self.detail),
        }
    }
}

/// Everything the scan observed, plus how much of the tree it actually covered.
#[derive(Debug, Clone, Default)]
pub struct ProjectFacts {
    pub root: String,
    /// Files the walk saw (paths only).
    pub files_seen: usize,
    /// Files whose contents were read.
    pub files_read: usize,
    pub bytes_read: u64,
    /// The whole-scan budget was exhausted — files after that point were not
    /// read at all, so the evidence is a lower bound.
    pub scan_truncated: bool,
    /// Files skipped because they exceed the per-file size limit. Distinct from
    /// `scan_truncated`: one very large file does not mean the scan stopped.
    pub files_too_large: usize,
    /// Files tracked by git, or `None` when the root is not a git checkout.
    /// The committed-credential checks only run when this is `Some`.
    pub git_tracked: Option<usize>,
    /// The commit the scan saw, when the root is a git checkout.
    pub git_commit: Option<String>,
    /// Whether that commit had uncommitted changes. A report taken against a
    /// dirty tree cannot be reproduced from the commit alone, and an audit file
    /// that does not say so invites exactly that assumption.
    pub git_dirty: Option<bool>,
    signals: BTreeMap<Signal, Vec<Evidence>>,
    counts: BTreeMap<Signal, usize>,
}

impl ProjectFacts {
    pub fn has(&self, signal: Signal) -> bool {
        self.counts.contains_key(&signal)
    }

    pub fn evidence(&self, signal: Signal) -> &[Evidence] {
        self.signals.get(&signal).map_or(&[][..], |v| v.as_slice())
    }

    /// Total hits for a signal, including those past the evidence cap.
    pub fn count(&self, signal: Signal) -> usize {
        self.counts.get(&signal).copied().unwrap_or(0)
    }

    fn record(&mut self, signal: Signal, evidence: Evidence) {
        *self.counts.entry(signal).or_insert(0) += 1;
        let slot = self.signals.entry(signal).or_default();
        if slot.len() < MAX_EVIDENCE_PER_SIGNAL {
            slot.push(evidence);
        }
    }

    fn saturated(&self, signal: Signal) -> bool {
        self.signals
            .get(&signal)
            .is_some_and(|v| v.len() >= MAX_EVIDENCE_PER_SIGNAL)
    }
}

// ── Content rules ───────────────────────────────────────────────────────────

struct ContentRule {
    signal: Signal,
    detail: &'static str,
    pattern: &'static str,
}

/// Patterns matched against the text of every readable file. Each one names the
/// concrete artefact it recognises, so the evidence line says *why* the file
/// counted rather than just naming it.
///
/// Every pattern is lowercase and matched against an ASCII-lowercased copy of
/// the file, rather than carrying `(?i)`. A case-insensitive alternation of
/// this many literals inflates the NFA past the lazy DFA's cache and strips the
/// literal prefilter, which drops the search onto the PikeVM — the difference
/// between a scan of this repository taking seconds and taking minutes.
/// `to_ascii_lowercase` preserves byte length, so match offsets still address
/// the original text.
const CONTENT_RULES: &[ContentRule] = &[
    ContentRule {
        signal: Signal::Authentication,
        detail: "authentication",
        pattern: r"(require_auth|authorization:\s*bearer|bearer\s+token|jsonwebtoken|jwt\.(sign|verify)|passport\.(use|authenticate)|next-auth|oauth2|@login_required|isauthenticated|authenticate_user|verify_token|check_password|password_hash|bcrypt|argon2)",
    },
    ContentRule {
        signal: Signal::Authorization,
        detail: "authorization check",
        pattern: r"(\brbac\b|role[_-]?based|has_permission|check_permission|require_role|is_authorized|authorize\(|casbin|@preauthorize|permission_denied|access_denied|is_admin\b|policy\.enforce)",
    },
    ContentRule {
        signal: Signal::MultiFactor,
        detail: "multi-factor authentication",
        pattern: r"(\bmfa\b|two[_-]?factor|\b2fa\b|\btotp\b|webauthn|passkey|otp_secret|authenticator_app)",
    },
    ContentRule {
        signal: Signal::SessionManagement,
        detail: "session lifetime / revocation",
        pattern: r"(session_(id|token|store|expiry|timeout)|revoke_(token|session)|refresh_token|expires_at|token_expiry|max_age|logout)",
    },
    ContentRule {
        signal: Signal::EncryptionInTransit,
        detail: "transport encryption",
        pattern: r"(rustls|native[_-]tls|tls_config|tls_connector|https_only|force_ssl|strict-transport-security|\bhsts\b|ssl_context|require_tls|min_tls_version|certificate_verify|use_https|sslcontext)",
    },
    ContentRule {
        signal: Signal::EncryptionAtRest,
        detail: "encryption at rest",
        pattern: r"(aes[_-]?256|aes[_-]?gcm|chacha20|xchacha20|sqlcipher|encrypt_at_rest|encrypted_(store|db|column|field)|\bkms\b|libsodium|crypto_secretbox|fernet|age-encryption|\bsops\b)",
    },
    ContentRule {
        signal: Signal::SecretsManager,
        detail: "external secret storage",
        pattern: r"(hashicorp\s+vault|vault_addr|secretsmanager|secret_manager|key\s*vault|gcp_secret|google_secret_manager|keyring|keychain|1password|doppler|sops)",
    },
    ContentRule {
        signal: Signal::AuditLogging,
        detail: "audit log",
        pattern: r"(audit[_-]?(log|trail|event|record)|auditlog|security_event|log_(action|access|event)\()",
    },
    ContentRule {
        signal: Signal::Logging,
        detail: "application logging",
        pattern: r"(tracing::(info|warn|error)|log::(info|warn|error)|logger\.(info|warn|error)|logging\.getlogger|winston|serilog|structlog|zap\.(l|s)\()",
    },
    ContentRule {
        signal: Signal::LogRedaction,
        detail: "log redaction",
        pattern: r"(redact|mask_(secret|token|key|pii)|scrub_(log|secret)|sanitize_log|obfuscate_)",
    },
    ContentRule {
        signal: Signal::Telemetry,
        detail: "monitoring instrumentation",
        pattern: r"(opentelemetry|\botel\b|prometheus|sentry_sdk|sentry\.init|datadog|newrelic|new_relic|statsd|grafana|metrics::(counter|gauge|histogram))",
    },
    ContentRule {
        signal: Signal::RateLimiting,
        detail: "rate limiting",
        pattern: r"(rate[_-]?limit|ratelimiter|throttl(e|ing)|express-rate-limit|slowapi|token_bucket|leaky_bucket|governor::)",
    },
    ContentRule {
        signal: Signal::InputValidation,
        detail: "input validation",
        pattern: r"(\bzod\b|pydantic|deny_unknown_fields|validate_(input|request|payload)|sanitize|escape_html|prepared_statement|parameteri[sz]ed|class-validator|joi\.object|yup\.object|marshmallow)",
    },
    ContentRule {
        signal: Signal::DataRetention,
        detail: "retention policy",
        pattern: r"(retention_(days|policy|period)|data_retention|retain_for|purge_after|expire_after|ttl_(days|seconds)|auto_delete_after)",
    },
    ContentRule {
        signal: Signal::PiiRedaction,
        detail: "PII handling",
        pattern: r"(\bpii\b|personal_data|personally_identifiable|anonymi[sz]|pseudonymi[sz]|redact_pii|mask_email|de[_-]identif)",
    },
    ContentRule {
        signal: Signal::ConsentTracking,
        detail: "consent tracking",
        pattern: r"(consent_(given|record|withdrawn|status)|user_consent|cookie_consent|opt[_-]in|marketing_consent)",
    },
    ContentRule {
        signal: Signal::DataErasure,
        detail: "data erasure path",
        pattern: r"(delete_account|account_deletion|right_to_(be_forgotten|erasure)|erasure_request|purge_user|anonymize_user|forget_user)",
    },
    ContentRule {
        signal: Signal::DataPortability,
        detail: "data export path",
        pattern: r"(export_(user_)?data|data_export|download_my_data|\bdsar\b|subject_access_request|takeout)",
    },
    ContentRule {
        signal: Signal::DependencyAudit,
        detail: "dependency vulnerability scan",
        pattern: r"(cargo\s+audit|cargo[_-]deny|npm\s+audit|pnpm\s+audit|yarn\s+audit|pip-audit|safety\s+check|snyk|trivy|grype|osv-scanner|dependency-review-action|dependabot)",
    },
    ContentRule {
        signal: Signal::SecretScanning,
        detail: "secret scanning",
        pattern: r"(gitleaks|trufflehog|detect-secrets|git-secrets|ggshield|secret[_-]scanning)",
    },
    ContentRule {
        signal: Signal::StaticAnalysis,
        detail: "static analysis",
        pattern: r"(cargo\s+clippy|clippy::|eslint|semgrep|codeql|sonarqube|sonar-scanner|golangci-lint|bandit\s|ruff\s+check|mypy\s|shellcheck)",
    },
    ContentRule {
        signal: Signal::AutomatedTests,
        detail: "test execution",
        pattern: r"(cargo\s+test|npm\s+(run\s+)?test|pnpm\s+test|pytest|go\s+test|jest|vitest|gradle\s+test|mvn\s+test|rspec|phpunit)",
    },
    ContentRule {
        signal: Signal::BackupProcedure,
        detail: "backup procedure",
        pattern: r"(pg_dump|mysqldump|pg_basebackup|restic|borgbackup|velero|litestream|snapshot_schedule|backup_(job|schedule|policy|retention)|point[_-]in[_-]time[_-]recovery)",
    },
    ContentRule {
        signal: Signal::HardcodedCredential,
        // `=`, `:` and Go's `:=` all assign.
        detail: "credential literal in source",
        pattern: r#"(?:api[_-]?key|secret[_-]?key|client[_-]?secret|access[_-]?token|auth[_-]?token|password)\s*(?::=|[:=])\s*["']([a-z0-9_\-+/=\.]{16,})["']"#,
    },
];

/// The rules that compiled, paired with their compiled regex. A rule whose
/// pattern fails to compile is dropped rather than panicking a daemon path —
/// `content_rules_all_compile` fails the build if that ever happens.
static COMPILED_RULES: LazyLock<Vec<(&'static ContentRule, Regex)>> = LazyLock::new(|| {
    CONTENT_RULES
        .iter()
        .filter_map(|rule| Regex::new(rule.pattern).ok().map(|re| (rule, re)))
        .collect()
});


/// Values that look like a credential but are documentation.
static PLACEHOLDER: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"example|placeholder|your[_-]?|xxx|changeme|change[_-]me|dummy|sample|redacted|todo|fake|insert[_-]|\.\.\.|<.+>|\$\{").ok()
});

/// `USER` instruction in a container image, with the account it switches to.
static DOCKER_USER: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*user\s+([a-z0-9_.$-]+)").ok());

// ── File classification ─────────────────────────────────────────────────────

/// Extensions worth reading. Anything else is treated as opaque: its path may
/// still carry a signal, but its bytes are never loaded.
const TEXT_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "java", "kt", "kts", "swift", "rb",
    "php", "cs", "scala", "clj", "ex", "exs", "dart", "lua", "pl", "r", "jl", "c", "cc", "cpp", "h",
    "hpp", "m", "mm", "sh", "bash", "zsh", "fish", "ps1", "sql", "toml", "yaml", "yml", "json",
    "jsonc", "md", "mdx", "rst", "txt", "tf", "tfvars", "hcl", "ini", "cfg", "conf", "properties",
    "gradle", "xml", "html", "css", "scss", "vue", "svelte", "env", "example", "template", "nix",
];

/// Filenames with no extension that are still text worth reading.
const TEXT_BASENAMES: &[&str] = &[
    "dockerfile",
    "containerfile",
    "makefile",
    "jenkinsfile",
    "procfile",
    "vagrantfile",
    "codeowners",
    "license",
    "licence",
    "copying",
    "notice",
    "readme",
];

/// Generated or vendored files: large, noisy, and evidence of nothing.
const SKIP_READ_BASENAMES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "cargo.lock",
    "poetry.lock",
    "uv.lock",
    "pipfile.lock",
    "composer.lock",
    "gemfile.lock",
    "go.sum",
    "flake.lock",
];

const LOCKFILE_BASENAMES: &[&str] = &[
    "cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "poetry.lock",
    "uv.lock",
    "pipfile.lock",
    "composer.lock",
    "gemfile.lock",
    "go.sum",
    "gradle.lockfile",
    "packages.lock.json",
];

/// Credential files that must never be tracked. Binary keystore formats count
/// on sight; PEM-shaped files are read first, because a `.pem` is just as
/// likely to be a public certificate.
const SECRET_FILE_BASENAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.prod",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "credentials.json",
    "service-account.json",
    "serviceaccount.json",
    ".npmrc",
    ".pypirc",
];

const KEYSTORE_EXTENSIONS: &[&str] = &["p12", "pfx", "jks", "keystore"];
const PEM_EXTENSIONS: &[&str] = &["pem", "key"];

/// Paths whose credential-shaped literals are fixtures or documentation.
fn is_fixture_path(rel_lower: &str) -> bool {
    const MARKERS: &[&str] = &[
        "test", "spec", "fixture", "mock", "example", "sample", "docs/", "doc/", "demo",
        "__snapshots__", "benches/", "testdata",
    ];
    MARKERS.iter().any(|m| rel_lower.contains(m))
}

fn basename(rel_lower: &str) -> &str {
    rel_lower.rsplit('/').next().unwrap_or(rel_lower)
}

fn extension(base_lower: &str) -> Option<&str> {
    base_lower.rsplit_once('.').map(|(_, ext)| ext)
}

fn is_readable_text(base_lower: &str) -> bool {
    if SKIP_READ_BASENAMES.contains(&base_lower) {
        return false;
    }
    if base_lower.ends_with(".min.js")
        || base_lower.ends_with(".min.css")
        || base_lower.ends_with(".map")
    {
        return false;
    }
    // `dockerfile.dev`, `readme.md`, `license.txt`: the stem names the kind of
    // file, whatever follows the dot. Matched without allocating per candidate.
    if TEXT_BASENAMES.iter().any(|b| {
        base_lower == *b
            || base_lower
                .strip_prefix(*b)
                .is_some_and(|rest| rest.starts_with('.'))
    }) {
        return true;
    }
    // `.env.example`, `.gitignore`, `.gitleaks.toml` and friends: a leading dot
    // is part of the name, not an extension.
    if base_lower.starts_with(".env") || base_lower.starts_with(".git") {
        return true;
    }
    extension(base_lower).is_some_and(|ext| TEXT_EXTENSIONS.contains(&ext))
}

// ── Path signals ────────────────────────────────────────────────────────────

/// Signals a file carries by virtue of *where it is* — a workflow definition,
/// a license, a runbook. Content is not needed to recognise these.
fn path_signals(rel_lower: &str) -> Vec<(Signal, &'static str)> {
    let base = basename(rel_lower);
    let ext = extension(base);
    let mut out: Vec<(Signal, &'static str)> = Vec::new();

    let starts = |p: &str| base.starts_with(p);
    let contains = |p: &str| rel_lower.contains(p);

    if starts("license") || starts("licence") || base == "copying" {
        out.push((Signal::License, "license file"));
    }
    if base == "codeowners" {
        out.push((Signal::CodeOwners, "code owners"));
    }
    if starts("contributing") || starts("code_of_conduct") || starts("code-of-conduct") {
        out.push((Signal::ContributionGuide, "contribution guide"));
    }
    if starts("security.") || contains("/security/policy") {
        out.push((Signal::SecurityPolicy, "security policy"));
    }
    if starts("readme") || rel_lower.starts_with("docs/") || rel_lower.starts_with("doc/") {
        out.push((Signal::ProjectDocs, "project documentation"));
    }
    if starts("changelog") || starts("release-notes") || starts("release_notes") {
        out.push((Signal::ChangeLog, "changelog"));
    }
    if contains("threat-model")
        || contains("threat_model")
        || contains("threatmodel")
        || contains("risk-register")
        || contains("risk_register")
        || contains("risk-assessment")
        || contains("risk_assessment")
        || contains("dpia")
    {
        out.push((Signal::ThreatModel, "risk / threat documentation"));
    }
    if contains("incident")
        || contains("runbook")
        || contains("on-call")
        || contains("oncall")
        || contains("postmortem")
        || contains("post-mortem")
        || contains("disaster-recovery")
        || contains("disaster_recovery")
    {
        out.push((Signal::IncidentRunbook, "incident documentation"));
    }
    if starts("privacy") || contains("privacy-policy") || contains("privacy_policy") {
        out.push((Signal::PrivacyNotice, "privacy notice"));
    }
    if contains("data-map")
        || contains("data_map")
        || contains("data-inventory")
        || contains("data_inventory")
        || contains("records-of-processing")
        || contains("data-classification")
        || base == "ropa.md"
    {
        out.push((Signal::DataInventory, "data inventory"));
    }
    if contains("backup") || contains("restore") || contains("recovery") {
        out.push((Signal::BackupProcedure, "backup / recovery artefact"));
    }
    if (rel_lower.starts_with(".github/workflows/")
        && (rel_lower.ends_with(".yml") || rel_lower.ends_with(".yaml")))
        || base == ".gitlab-ci.yml"
        || base == "jenkinsfile"
        || rel_lower == ".circleci/config.yml"
        || base == "azure-pipelines.yml"
        || base == ".drone.yml"
        || base == ".travis.yml"
        || rel_lower.starts_with(".buildkite/")
    {
        out.push((Signal::CiPipeline, "CI pipeline"));
    }
    if contains("pull_request_template") || contains("pull-request-template") {
        out.push((Signal::ReviewGate, "pull-request template"));
    }
    if rel_lower.starts_with("tests/")
        || rel_lower.starts_with("test/")
        || rel_lower.starts_with("spec/")
        || contains("/tests/")
        || contains("/__tests__/")
        || base.starts_with("test_")
        || contains("_test.")
        || contains(".test.")
        || contains(".spec.")
        || contains("_spec.")
    {
        out.push((Signal::AutomatedTests, "test file"));
    }
    if LOCKFILE_BASENAMES.contains(&base) {
        out.push((Signal::DependencyLockfile, "dependency lockfile"));
    }
    if base == "dependabot.yml" || base == "dependabot.yaml" || starts("renovate") {
        out.push((Signal::DependencyAudit, "dependency update automation"));
    }
    if starts(".gitleaks") || base == ".secrets.baseline" || starts("trufflehog") {
        out.push((Signal::SecretScanning, "secret-scanning config"));
    }
    if starts(".eslintrc")
        || starts("eslint.config")
        || base == "clippy.toml"
        || starts(".semgrep")
        || base == "sonar-project.properties"
        || starts(".golangci")
        || base == "ruff.toml"
        || base == "mypy.ini"
        || base == ".pre-commit-config.yaml"
        || contains("codeql")
    {
        out.push((Signal::StaticAnalysis, "linter / analyser config"));
    }
    if ext == Some("tf")
        || ext == Some("tfvars")
        || starts("docker-compose")
        || starts("compose.")
        || base == "pulumi.yaml"
        || base == "serverless.yml"
        || base == "ansible.cfg"
        || contains("/helm/")
        || contains("/charts/")
        || contains("k8s/")
        || contains("kubernetes/")
        || contains("cloudformation")
        || starts("dockerfile")
    {
        out.push((Signal::IacDefinitions, "infrastructure definition"));
    }
    if starts(".env.example")
        || starts(".env.sample")
        || starts(".env.template")
        || base == "env.example"
        || base == ".env.dist"
    {
        out.push((Signal::EnvExample, "environment template"));
    }
    out
}

// ── Scanning ────────────────────────────────────────────────────────────────

/// Output of a `git` subcommand run in `root`, or `None` when git is
/// unavailable or the directory is not a checkout.
fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Files git reports as tracked, relative to `root` and lowercased.
/// `None` when `root` is not a git checkout, or git is unavailable — in which
/// case the committed-credential checks are skipped rather than guessed at.
fn tracked_files(root: &Path) -> Option<HashSet<String>> {
    Some(
        git_output(root, &["ls-files", "-z"])?
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect(),
    )
}

/// Line number containing the byte at `offset`.
fn line_of_offset(text: &str, offset: usize) -> Option<u32> {
    let preceding = text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count();
    u32::try_from(preceding.saturating_add(1)).ok()
}

/// Walk `root` and collect every signal the tree supports.
pub fn scan(root: &Path) -> ProjectFacts {
    let tracked = tracked_files(root);
    // Both are `None` off a checkout: an audit file says "unknown", never a
    // plausible-looking commit nobody read.
    let git_commit = git_output(root, &["rev-parse", "HEAD"]).map(|s| s.trim().to_string());
    let git_dirty =
        git_output(root, &["status", "--porcelain"]).map(|s| !s.trim().is_empty());
    let mut facts = ProjectFacts {
        root: root.display().to_string(),
        git_tracked: tracked.as_ref().map(|t| t.len()),
        git_commit,
        git_dirty,
        ..ProjectFacts::default()
    };

    for path in discover_files(root) {
        facts.files_seen += 1;
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_lower = rel.to_string_lossy().replace('\\', "/").to_lowercase();
        let rel_display = rel.to_string_lossy().replace('\\', "/");
        let base = basename(&rel_lower).to_string();

        for (signal, detail) in path_signals(&rel_lower) {
            if facts.saturated(signal) {
                // Still counted, so the report can say how many matched.
                *facts.counts.entry(signal).or_insert(0) += 1;
                continue;
            }
            facts.record(
                signal,
                Evidence {
                    path: rel_display.clone(),
                    line: None,
                    detail,
                },
            );
        }

        let is_tracked = tracked
            .as_ref()
            .is_some_and(|t| t.contains(rel_lower.as_str()));
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(u64::MAX);

        // Committed credential files: keystores count on sight, PEM-shaped
        // files only when they actually hold a private key.
        if is_tracked {
            let ext = extension(&base);
            let keystore = ext.is_some_and(|e| KEYSTORE_EXTENSIONS.contains(&e));
            let named = SECRET_FILE_BASENAMES.contains(&base.as_str());
            let pem = ext.is_some_and(|e| PEM_EXTENSIONS.contains(&e));
            let pem_private = pem
                && size <= MAX_FILE_BYTES
                && std::fs::read_to_string(&path)
                    .map(|t| t.contains("PRIVATE KEY"))
                    .unwrap_or(false);
            if keystore || named || pem_private {
                facts.record(
                    Signal::CommittedSecretFile,
                    Evidence {
                        path: rel_display.clone(),
                        line: None,
                        detail: "credential file tracked in git",
                    },
                );
            }
        }

        if !is_readable_text(&base) {
            continue;
        }
        if facts.files_read >= MAX_FILES_READ || facts.bytes_read >= MAX_TOTAL_BYTES {
            facts.scan_truncated = true;
            continue;
        }
        if size > MAX_FILE_BYTES {
            facts.files_too_large += 1;
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        facts.files_read += 1;
        facts.bytes_read = facts.bytes_read.saturating_add(text.len() as u64);

        // Matched against once, reused by every rule.
        let haystack = text.to_ascii_lowercase();
        let fixture = is_fixture_path(&rel_lower);
        // One `find` per rule, not one `RegexSet::matches`: the set API answers
        // "which of these matched" with the PikeVM, an NFA simulation that runs
        // orders of magnitude slower than the prefiltered single-pattern search
        // — it turned a scan of this repository into a ten-minute job. `find`
        // also hands back the offset, so the line number costs nothing extra.
        for (rule, re) in COMPILED_RULES.iter() {
            let Some(hit) = re.find(&haystack) else {
                continue;
            };
            // A credential-shaped literal in a fixture or a doc is neither a
            // finding nor evidence; a placeholder value is not a secret.
            if rule.signal == Signal::HardcodedCredential
                && (fixture || !is_plausible_credential(&text, &haystack, re))
            {
                continue;
            }
            if facts.saturated(rule.signal) {
                *facts.counts.entry(rule.signal).or_insert(0) += 1;
                continue;
            }
            facts.record(
                rule.signal,
                Evidence {
                    path: rel_display.clone(),
                    line: line_of_offset(&haystack, hit.start()),
                    detail: rule.detail,
                },
            );
        }

        if base.starts_with("dockerfile") || base == "containerfile" {
            if let Some((line, user)) = docker_non_root_user(&haystack) {
                facts.record(
                    Signal::ContainerNonRoot,
                    Evidence {
                        path: rel_display.clone(),
                        line: Some(line),
                        detail: user,
                    },
                );
            }
        }
    }

    facts
}

/// True when at least one credential-shaped literal holds a value that could
/// actually be a secret.
///
/// The key half of the pattern is matched against `haystack` (lowercased, for
/// speed), but the *value* is read back out of `original` at the same byte
/// offsets — `to_ascii_lowercase` preserves length, so the spans line up. Case
/// is the whole signal here: `ghp_16Cabcdefghijklmnop` is a token,
/// `vibecody.watch.access_token` is a keychain key, and once both are
/// lowercased they are indistinguishable.
fn is_plausible_credential(original: &str, haystack: &str, re: &Regex) -> bool {
    let Some(placeholder) = PLACEHOLDER.as_ref() else {
        return false;
    };
    re.captures_iter(haystack)
        .filter_map(|c| c.get(1))
        .filter(|m| !placeholder.is_match(m.as_str()))
        .filter_map(|m| original.get(m.start()..m.end()))
        .any(looks_like_a_secret)
}

/// Whether a string has the shape of a generated secret rather than a name.
///
/// Two things disqualify a value, and both need the original casing:
///
/// - It reads as an identifier — dot- or underscore-separated lowercase words,
///   like `vibecody.watch.access_token`. That is what a keychain key, a config
///   key or a settings constant looks like, and the codebase is full of them
///   assigned to variables called `accessToken`.
/// - It draws on fewer than two of {lowercase, uppercase, digit}. Generated
///   credentials mix character classes; english-ish names do not.
fn looks_like_a_secret(value: &str) -> bool {
    let identifier = value
        .split(['.', '_', '-'])
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
        && value.contains(['.', '_', '-']);
    if identifier {
        return false;
    }
    let classes = [
        value.chars().any(|c| c.is_ascii_lowercase()),
        value.chars().any(|c| c.is_ascii_uppercase()),
        value.chars().any(|c| c.is_ascii_digit()),
    ]
    .iter()
    .filter(|present| **present)
    .count();
    classes >= 2
}

/// The line of the last `USER` instruction, when it switches away from root.
fn docker_non_root_user(text: &str) -> Option<(u32, &'static str)> {
    let re = DOCKER_USER.as_ref()?;
    let (idx, caps) = text
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| re.captures(line).map(|c| (idx, c)))
        .last()?;
    let user = caps.get(1)?.as_str();
    if user.eq_ignore_ascii_case("root") || user == "0" {
        return None;
    }
    Some(((idx as u32).saturating_add(1), "non-root container user"))
}

// ── Control catalogue ───────────────────────────────────────────────────────

/// How a control is judged from a scan.
pub enum Assessment {
    /// Scored against signals found in the workspace.
    Code(Rule),
    /// Cannot be evidenced by a repository at all. The string says what would
    /// evidence it, so the report tells the reader where to look instead of
    /// silently scoring the control as met.
    Organisational(&'static str),
}

pub struct Rule {
    /// Every one of these must be present for `Implemented`.
    pub required: &'static [Signal],
    /// Reported as evidence and enough for `PartiallyImplemented`, but never
    /// enough on their own for a pass.
    pub supporting: &'static [Signal],
    /// Findings that cap the control at `NotImplemented`.
    pub disqualifying: &'static [Signal],
}

pub struct ControlSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub assessment: Assessment,
}

const fn rule(required: &'static [Signal], supporting: &'static [Signal]) -> Assessment {
    Assessment::Code(Rule {
        required,
        supporting,
        disqualifying: &[],
    })
}

const fn rule_with_findings(
    required: &'static [Signal],
    supporting: &'static [Signal],
    disqualifying: &'static [Signal],
) -> Assessment {
    Assessment::Code(Rule {
        required,
        supporting,
        disqualifying,
    })
}

const SOC2: &[ControlSpec] = &[
    ControlSpec {
        id: "CC1.1",
        name: "Control Environment",
        description: "The entity demonstrates a commitment to integrity and ethical values.",
        assessment: rule(
            &[Signal::License, Signal::ContributionGuide],
            &[Signal::CodeOwners, Signal::ProjectDocs],
        ),
    },
    ControlSpec {
        id: "CC1.4",
        name: "Personnel Competence",
        description: "The entity attracts, develops and retains competent individuals.",
        assessment: Assessment::Organisational(
            "hiring records, role descriptions and training logs — no repository evidence exists",
        ),
    },
    ControlSpec {
        id: "CC2.1",
        name: "Information and Communication",
        description: "Quality information is obtained and communicated to support the controls.",
        assessment: rule(&[Signal::ProjectDocs], &[Signal::ChangeLog]),
    },
    ControlSpec {
        id: "CC3.2",
        name: "Risk Identification",
        description: "The entity identifies and analyses risks to its objectives.",
        assessment: rule(&[Signal::ThreatModel], &[Signal::SecurityPolicy]),
    },
    ControlSpec {
        id: "CC4.1",
        name: "Monitoring Activities",
        description: "Evaluations determine whether the controls are present and functioning.",
        assessment: rule(&[Signal::Telemetry], &[Signal::Logging]),
    },
    ControlSpec {
        id: "CC5.2",
        name: "Technology Control Activities",
        description: "Control activities over technology support the achievement of objectives.",
        assessment: rule(
            &[Signal::CiPipeline],
            &[Signal::StaticAnalysis, Signal::AutomatedTests],
        ),
    },
    ControlSpec {
        id: "CC6.1",
        name: "Logical Access — Authentication",
        description: "Logical access to protected information assets is restricted.",
        assessment: rule(
            &[Signal::Authentication],
            &[Signal::SessionManagement, Signal::MultiFactor],
        ),
    },
    ControlSpec {
        id: "CC6.2",
        name: "Access Authorization",
        description: "Access is authorised prior to issuing credentials and is reviewed.",
        assessment: rule(&[Signal::Authorization], &[Signal::ReviewGate]),
    },
    ControlSpec {
        id: "CC6.3",
        name: "Access Removal",
        description: "Access is removed when no longer required.",
        assessment: rule(&[Signal::SessionManagement], &[Signal::Authorization]),
    },
    ControlSpec {
        id: "CC6.6",
        name: "Encryption in Transit",
        description: "Security measures protect data transmitted beyond system boundaries.",
        assessment: rule(&[Signal::EncryptionInTransit], &[Signal::RateLimiting]),
    },
    ControlSpec {
        id: "CC6.7",
        name: "Encryption at Rest and Secret Handling",
        description: "Data at rest is protected and credentials are held outside the source tree.",
        assessment: rule_with_findings(
            &[Signal::EncryptionAtRest],
            &[Signal::EnvExample, Signal::SecretsManager],
            &[Signal::CommittedSecretFile, Signal::HardcodedCredential],
        ),
    },
    ControlSpec {
        id: "CC6.8",
        name: "Malicious Software Prevention",
        description: "Controls prevent or detect unauthorised or malicious software.",
        assessment: rule(
            &[Signal::DependencyAudit, Signal::DependencyLockfile],
            &[Signal::SecretScanning],
        ),
    },
    ControlSpec {
        id: "CC7.2",
        name: "Security Monitoring",
        description: "The entity monitors system components for anomalies and security events.",
        assessment: rule(
            &[Signal::AuditLogging],
            &[Signal::LogRedaction, Signal::Telemetry],
        ),
    },
    ControlSpec {
        id: "CC7.4",
        name: "Incident Response",
        description: "The entity responds to identified security incidents.",
        assessment: rule(&[Signal::IncidentRunbook], &[Signal::SecurityPolicy]),
    },
    ControlSpec {
        id: "CC8.1",
        name: "Change Management",
        description: "Changes are authorised, designed, tested and approved before deployment.",
        assessment: rule(
            &[Signal::CiPipeline, Signal::ReviewGate],
            &[Signal::AutomatedTests, Signal::ChangeLog],
        ),
    },
    ControlSpec {
        id: "CC9.1",
        name: "Risk Mitigation",
        description: "The entity identifies and mitigates risks from business disruption.",
        assessment: rule(
            &[Signal::InputValidation, Signal::RateLimiting],
            &[Signal::StaticAnalysis],
        ),
    },
    ControlSpec {
        id: "CC9.2",
        name: "Vendor and Business Partner Management",
        description: "The entity assesses and manages risks from vendors and partners.",
        assessment: Assessment::Organisational(
            "vendor contracts, subprocessor lists and due-diligence records held outside the repository",
        ),
    },
    ControlSpec {
        id: "A1.2",
        name: "Availability — Backup and Recovery",
        description: "Environmental protections, backup and recovery support availability.",
        assessment: rule(&[Signal::BackupProcedure], &[Signal::IacDefinitions]),
    },
];

const FEDRAMP: &[ControlSpec] = &[
    ControlSpec {
        id: "AC-2",
        name: "Account Management",
        description: "Accounts are created, enabled, modified and removed under control.",
        assessment: rule(
            &[Signal::Authentication, Signal::Authorization],
            &[Signal::AuditLogging],
        ),
    },
    ControlSpec {
        id: "AC-12",
        name: "Session Termination",
        description: "Sessions are terminated after a defined condition.",
        assessment: rule(&[Signal::SessionManagement], &[]),
    },
    ControlSpec {
        id: "AU-2",
        name: "Auditable Events",
        description: "The system logs the events needed to support after-the-fact investigation.",
        assessment: rule(&[Signal::AuditLogging], &[Signal::Logging]),
    },
    ControlSpec {
        id: "AU-6",
        name: "Audit Review and Reporting",
        description: "Audit records are reviewed and anomalies reported.",
        assessment: rule(&[Signal::Telemetry], &[Signal::AuditLogging]),
    },
    ControlSpec {
        id: "AU-9",
        name: "Protection of Audit Information",
        description: "Audit information is protected from unauthorised disclosure.",
        assessment: rule(&[Signal::LogRedaction], &[Signal::EncryptionAtRest]),
    },
    ControlSpec {
        id: "CM-2",
        name: "Baseline Configuration",
        description: "A current baseline configuration of the system is maintained.",
        assessment: rule(
            &[Signal::IacDefinitions, Signal::DependencyLockfile],
            &[Signal::ContainerNonRoot],
        ),
    },
    ControlSpec {
        id: "CM-3",
        name: "Configuration Change Control",
        description: "Changes are proposed, reviewed, approved and tracked.",
        assessment: rule(
            &[Signal::CiPipeline, Signal::ReviewGate],
            &[Signal::ChangeLog],
        ),
    },
    ControlSpec {
        id: "CP-9",
        name: "System Backup",
        description: "System-level and user-level information is backed up.",
        assessment: rule(&[Signal::BackupProcedure], &[Signal::IacDefinitions]),
    },
    ControlSpec {
        id: "IA-2(1)",
        name: "Multi-Factor Authentication",
        description: "Multi-factor authentication is enforced for privileged accounts.",
        assessment: rule(&[Signal::MultiFactor], &[Signal::Authentication]),
    },
    ControlSpec {
        id: "IR-4",
        name: "Incident Handling",
        description: "An incident-handling capability covers preparation and response.",
        assessment: rule(&[Signal::IncidentRunbook], &[Signal::SecurityPolicy]),
    },
    ControlSpec {
        id: "PE-2",
        name: "Physical Access Authorizations",
        description: "Physical access to the facility hosting the system is authorised.",
        assessment: Assessment::Organisational(
            "data-centre or cloud-provider attestations — no repository evidence exists",
        ),
    },
    ControlSpec {
        id: "PS-3",
        name: "Personnel Screening",
        description: "Individuals are screened before being granted system access.",
        assessment: Assessment::Organisational(
            "background-check records held by HR — no repository evidence exists",
        ),
    },
    ControlSpec {
        id: "RA-3",
        name: "Risk Assessment",
        description: "Risk to operations and assets is assessed and documented.",
        assessment: rule(&[Signal::ThreatModel], &[Signal::SecurityPolicy]),
    },
    ControlSpec {
        id: "RA-5",
        name: "Vulnerability Scanning",
        description: "The system is scanned for vulnerabilities on a defined frequency.",
        assessment: rule(
            &[Signal::DependencyAudit],
            &[Signal::StaticAnalysis, Signal::SecretScanning],
        ),
    },
    ControlSpec {
        id: "SA-11",
        name: "Developer Testing and Evaluation",
        description: "The developer performs security testing on the delivered system.",
        assessment: rule(
            &[Signal::AutomatedTests, Signal::StaticAnalysis],
            &[Signal::CiPipeline],
        ),
    },
    ControlSpec {
        id: "SC-8",
        name: "Transmission Confidentiality and Integrity",
        description: "Transmitted information is protected.",
        assessment: rule(&[Signal::EncryptionInTransit], &[]),
    },
    ControlSpec {
        id: "SC-13",
        name: "Cryptographic Protection",
        description: "Cryptographic mechanisms are used in accordance with policy.",
        assessment: rule(
            &[Signal::EncryptionAtRest],
            &[Signal::EncryptionInTransit, Signal::SecretsManager],
        ),
    },
    ControlSpec {
        id: "SC-28",
        name: "Protection of Information at Rest",
        description: "Information at rest is protected, credentials included.",
        assessment: rule_with_findings(
            &[Signal::EncryptionAtRest],
            &[Signal::SecretsManager, Signal::EnvExample],
            &[Signal::CommittedSecretFile, Signal::HardcodedCredential],
        ),
    },
    ControlSpec {
        id: "SI-2",
        name: "Flaw Remediation",
        description: "Flaws are identified, reported and corrected.",
        assessment: rule(
            &[Signal::DependencyAudit, Signal::DependencyLockfile],
            &[Signal::CiPipeline],
        ),
    },
    ControlSpec {
        id: "SI-10",
        name: "Information Input Validation",
        description: "The validity of information inputs is checked.",
        assessment: rule(&[Signal::InputValidation], &[Signal::RateLimiting]),
    },
];

const HIPAA: &[ControlSpec] = &[
    ControlSpec {
        id: "164.308(a)(1)(ii)(A)",
        name: "Risk Analysis",
        description: "Conduct an assessment of risks to electronic protected health information.",
        assessment: rule(&[Signal::ThreatModel], &[Signal::SecurityPolicy]),
    },
    ControlSpec {
        id: "164.308(a)(1)(ii)(D)",
        name: "Information System Activity Review",
        description: "Regularly review audit logs and access reports.",
        assessment: rule(&[Signal::AuditLogging], &[Signal::Telemetry]),
    },
    ControlSpec {
        id: "164.308(a)(4)",
        name: "Information Access Management",
        description: "Authorise access to health information consistent with least privilege.",
        assessment: rule(
            &[Signal::Authentication, Signal::Authorization],
            &[Signal::SessionManagement],
        ),
    },
    ControlSpec {
        id: "164.308(a)(5)(ii)(B)",
        name: "Protection from Malicious Software",
        description: "Guard against and detect malicious software.",
        assessment: rule(
            &[Signal::DependencyAudit],
            &[Signal::StaticAnalysis, Signal::SecretScanning],
        ),
    },
    ControlSpec {
        id: "164.308(a)(6)",
        name: "Security Incident Procedures",
        description: "Identify and respond to suspected or known security incidents.",
        assessment: rule(&[Signal::IncidentRunbook], &[Signal::AuditLogging]),
    },
    ControlSpec {
        id: "164.308(a)(7)",
        name: "Contingency Plan",
        description: "Establish data backup, disaster recovery and emergency mode operation.",
        assessment: rule(&[Signal::BackupProcedure], &[Signal::IacDefinitions]),
    },
    ControlSpec {
        id: "164.308(b)(1)",
        name: "Business Associate Contracts",
        description: "Obtain satisfactory assurances from business associates.",
        assessment: Assessment::Organisational(
            "signed business-associate agreements — no repository evidence exists",
        ),
    },
    ControlSpec {
        id: "164.310(a)(1)",
        name: "Facility Access Controls",
        description: "Limit physical access to systems holding health information.",
        assessment: Assessment::Organisational(
            "facility access records or a hosting provider's HIPAA attestation",
        ),
    },
    ControlSpec {
        id: "164.312(a)(1)",
        name: "Access Control",
        description: "Allow access only to persons or programs that have been granted rights.",
        assessment: rule(
            &[Signal::Authentication],
            &[Signal::Authorization, Signal::SessionManagement],
        ),
    },
    ControlSpec {
        id: "164.312(a)(2)(iv)",
        name: "Encryption and Decryption",
        description: "Encrypt electronic protected health information at rest.",
        assessment: rule_with_findings(
            &[Signal::EncryptionAtRest],
            &[Signal::SecretsManager],
            &[Signal::CommittedSecretFile, Signal::HardcodedCredential],
        ),
    },
    ControlSpec {
        id: "164.312(b)",
        name: "Audit Controls",
        description: "Record and examine activity in systems holding health information.",
        assessment: rule(&[Signal::AuditLogging], &[Signal::Logging]),
    },
    ControlSpec {
        id: "164.312(c)(1)",
        name: "Integrity",
        description: "Protect health information from improper alteration or destruction.",
        assessment: rule(
            &[Signal::InputValidation],
            &[Signal::AutomatedTests, Signal::AuditLogging],
        ),
    },
    ControlSpec {
        id: "164.312(d)",
        name: "Person or Entity Authentication",
        description: "Verify that a person seeking access is the one claimed.",
        assessment: rule(&[Signal::Authentication, Signal::MultiFactor], &[]),
    },
    ControlSpec {
        id: "164.312(e)(1)",
        name: "Transmission Security",
        description: "Guard against unauthorised access to information in transit.",
        assessment: rule(&[Signal::EncryptionInTransit], &[]),
    },
    ControlSpec {
        id: "164.514(b)",
        name: "De-identification",
        description: "Remove identifiers so information is no longer individually identifiable.",
        assessment: rule(&[Signal::PiiRedaction], &[Signal::DataRetention]),
    },
];

const GDPR: &[ControlSpec] = &[
    ControlSpec {
        id: "Art. 5(1)(e)",
        name: "Storage Limitation",
        description: "Personal data is kept no longer than necessary.",
        assessment: rule(&[Signal::DataRetention], &[Signal::DataErasure]),
    },
    ControlSpec {
        id: "Art. 5(1)(f)",
        name: "Integrity and Confidentiality",
        description: "Personal data is processed with appropriate security.",
        assessment: rule_with_findings(
            &[Signal::EncryptionInTransit, Signal::EncryptionAtRest],
            &[Signal::Authorization],
            &[Signal::CommittedSecretFile, Signal::HardcodedCredential],
        ),
    },
    ControlSpec {
        id: "Art. 7",
        name: "Conditions for Consent",
        description: "Consent is recorded and can be withdrawn as easily as it was given.",
        assessment: rule(&[Signal::ConsentTracking], &[Signal::PrivacyNotice]),
    },
    ControlSpec {
        id: "Art. 13",
        name: "Transparency",
        description: "Data subjects are informed about the processing of their data.",
        assessment: rule(&[Signal::PrivacyNotice], &[Signal::ProjectDocs]),
    },
    ControlSpec {
        id: "Art. 15/20",
        name: "Access and Portability",
        description: "Data subjects can obtain and port their personal data.",
        assessment: rule(&[Signal::DataPortability], &[Signal::Authentication]),
    },
    ControlSpec {
        id: "Art. 17",
        name: "Right to Erasure",
        description: "Data subjects can have their personal data erased.",
        assessment: rule(&[Signal::DataErasure], &[Signal::DataRetention]),
    },
    ControlSpec {
        id: "Art. 25",
        name: "Data Protection by Design and by Default",
        description: "Technical measures such as pseudonymisation implement the principles.",
        assessment: rule(&[Signal::PiiRedaction], &[Signal::EncryptionAtRest]),
    },
    ControlSpec {
        id: "Art. 28",
        name: "Processor Obligations",
        description: "Processing by a processor is governed by a contract.",
        assessment: Assessment::Organisational(
            "data-processing agreements and a subprocessor register — no repository evidence exists",
        ),
    },
    ControlSpec {
        id: "Art. 30",
        name: "Records of Processing Activities",
        description: "A record of processing activities is maintained.",
        assessment: rule(&[Signal::DataInventory], &[Signal::ProjectDocs]),
    },
    ControlSpec {
        id: "Art. 32",
        name: "Security of Processing",
        description: "Appropriate technical measures ensure a level of security appropriate to risk.",
        assessment: rule(
            &[Signal::Authentication, Signal::EncryptionInTransit],
            &[Signal::AuditLogging, Signal::RateLimiting],
        ),
    },
    ControlSpec {
        id: "Art. 33",
        name: "Breach Notification",
        description: "Personal-data breaches are detected and notified within 72 hours.",
        assessment: rule(
            &[Signal::IncidentRunbook],
            &[Signal::AuditLogging, Signal::Telemetry],
        ),
    },
    ControlSpec {
        id: "Art. 35",
        name: "Data Protection Impact Assessment",
        description: "High-risk processing is assessed before it begins.",
        assessment: rule(&[Signal::ThreatModel], &[Signal::DataInventory]),
    },
    ControlSpec {
        id: "Art. 44",
        name: "International Transfers",
        description: "Transfers outside the EEA rely on an adequate safeguard.",
        assessment: Assessment::Organisational(
            "transfer impact assessments and standard contractual clauses held outside the repository",
        ),
    },
];

const ISO27001: &[ControlSpec] = &[
    ControlSpec {
        id: "A.5.1",
        name: "Policies for Information Security",
        description: "Information security policies are defined and published.",
        assessment: rule(
            &[Signal::SecurityPolicy],
            &[Signal::ProjectDocs, Signal::ContributionGuide],
        ),
    },
    ControlSpec {
        id: "A.5.15",
        name: "Access Control",
        description: "Rules for physical and logical access are established.",
        assessment: rule(
            &[Signal::Authorization],
            &[Signal::Authentication, Signal::CodeOwners],
        ),
    },
    ControlSpec {
        id: "A.5.17",
        name: "Authentication Information",
        description: "Allocation of authentication information is controlled.",
        assessment: rule_with_findings(
            &[Signal::Authentication],
            &[Signal::MultiFactor, Signal::SecretsManager],
            &[Signal::CommittedSecretFile, Signal::HardcodedCredential],
        ),
    },
    ControlSpec {
        id: "A.5.24",
        name: "Incident Management Planning",
        description: "Incident management is planned and prepared.",
        assessment: rule(&[Signal::IncidentRunbook], &[Signal::SecurityPolicy]),
    },
    ControlSpec {
        id: "A.5.30",
        name: "ICT Readiness for Business Continuity",
        description: "ICT readiness is planned, implemented and tested.",
        assessment: rule(&[Signal::BackupProcedure], &[Signal::IacDefinitions]),
    },
    ControlSpec {
        id: "A.6.3",
        name: "Information Security Awareness and Training",
        description: "Personnel receive security awareness education.",
        assessment: Assessment::Organisational(
            "training completion records — no repository evidence exists",
        ),
    },
    ControlSpec {
        id: "A.8.2",
        name: "Privileged Access Rights",
        description: "Allocation and use of privileged access is restricted and logged.",
        assessment: rule(
            &[Signal::Authorization, Signal::AuditLogging],
            &[Signal::CodeOwners],
        ),
    },
    ControlSpec {
        id: "A.8.8",
        name: "Management of Technical Vulnerabilities",
        description: "Vulnerabilities are identified and evaluated for exposure.",
        assessment: rule(
            &[Signal::DependencyAudit, Signal::DependencyLockfile],
            &[Signal::StaticAnalysis],
        ),
    },
    ControlSpec {
        id: "A.8.9",
        name: "Configuration Management",
        description: "Configurations are established, documented and monitored.",
        assessment: rule(
            &[Signal::IacDefinitions],
            &[Signal::ContainerNonRoot, Signal::DependencyLockfile],
        ),
    },
    ControlSpec {
        id: "A.8.12",
        name: "Data Leakage Prevention",
        description: "Data leakage prevention measures are applied.",
        assessment: rule_with_findings(
            &[Signal::SecretScanning],
            &[Signal::LogRedaction, Signal::EnvExample],
            &[Signal::CommittedSecretFile, Signal::HardcodedCredential],
        ),
    },
    ControlSpec {
        id: "A.8.15",
        name: "Logging",
        description: "Logs recording activities and security events are produced and kept.",
        assessment: rule(&[Signal::Logging, Signal::AuditLogging], &[]),
    },
    ControlSpec {
        id: "A.8.16",
        name: "Monitoring Activities",
        description: "Systems are monitored for anomalous behaviour.",
        assessment: rule(&[Signal::Telemetry], &[Signal::Logging]),
    },
    ControlSpec {
        id: "A.8.24",
        name: "Use of Cryptography",
        description: "Rules for the effective use of cryptography are implemented.",
        assessment: rule(
            &[Signal::EncryptionInTransit, Signal::EncryptionAtRest],
            &[Signal::SecretsManager],
        ),
    },
    ControlSpec {
        id: "A.8.25",
        name: "Secure Development Life Cycle",
        description: "Rules for secure development are established and applied.",
        assessment: rule(
            &[Signal::CiPipeline, Signal::AutomatedTests],
            &[Signal::ReviewGate],
        ),
    },
    ControlSpec {
        id: "A.8.28",
        name: "Secure Coding",
        description: "Secure coding principles are applied to software development.",
        assessment: rule(
            &[Signal::StaticAnalysis, Signal::InputValidation],
            &[Signal::AutomatedTests],
        ),
    },
    ControlSpec {
        id: "A.8.32",
        name: "Change Management",
        description: "Changes to information processing facilities are subject to change control.",
        assessment: rule(
            &[Signal::ReviewGate, Signal::CiPipeline],
            &[Signal::ChangeLog],
        ),
    },
];

/// The control catalogue scored for a framework.
pub fn catalogue(framework: &ComplianceFramework) -> &'static [ControlSpec] {
    match framework {
        ComplianceFramework::SOC2 => SOC2,
        ComplianceFramework::FedRAMP => FEDRAMP,
        ComplianceFramework::HIPAA => HIPAA,
        ComplianceFramework::GDPR => GDPR,
        ComplianceFramework::ISO27001 => ISO27001,
    }
}

// ── Assessment ──────────────────────────────────────────────────────────────

fn join_phrases(signals: &[Signal]) -> String {
    signals
        .iter()
        .map(|s| s.describe())
        .collect::<Vec<_>>()
        .join(", ")
}

fn evidence_for(facts: &ProjectFacts, signals: &[Signal]) -> Vec<String> {
    signals
        .iter()
        .filter(|s| facts.has(**s))
        .flat_map(|s| {
            let found = facts.evidence(*s);
            let extra = facts.count(*s).saturating_sub(found.len());
            found
                .iter()
                .take(2)
                .map(|e| e.render())
                .chain(
                    (extra > 0)
                        .then(|| format!("+{extra} more matching {}", s.describe()))
                        .into_iter(),
                )
                .collect::<Vec<_>>()
        })
        .take(MAX_EVIDENCE_PER_CONTROL)
        .collect()
}

fn assess_control(spec: &ControlSpec, facts: &ProjectFacts) -> ComplianceControl {
    let (status, evidence, notes) = match &spec.assessment {
        Assessment::Organisational(what) => (
            ControlStatus::NotAssessed,
            Vec::new(),
            format!("Not assessable from source. Evidence lives in {what}."),
        ),
        Assessment::Code(rule) => {
            let findings: Vec<Signal> = rule
                .disqualifying
                .iter()
                .copied()
                .filter(|s| facts.has(*s))
                .collect();
            let missing: Vec<Signal> = rule
                .required
                .iter()
                .copied()
                .filter(|s| !facts.has(*s))
                .collect();
            let present_required = rule.required.len() - missing.len();
            let present_supporting = rule.supporting.iter().filter(|s| facts.has(**s)).count();

            if !findings.is_empty() {
                let evidence = evidence_for(facts, &findings);
                (
                    ControlStatus::NotImplemented,
                    evidence,
                    format!("Blocked by a finding: {}.", join_phrases(&findings)),
                )
            } else if missing.is_empty() && !rule.required.is_empty() {
                let mut shown: Vec<Signal> = rule.required.to_vec();
                shown.extend(rule.supporting.iter().copied().filter(|s| facts.has(*s)));
                (
                    ControlStatus::Implemented,
                    evidence_for(facts, &shown),
                    format!("Found {}.", join_phrases(rule.required)),
                )
            } else if present_required > 0 || present_supporting > 0 {
                let mut shown: Vec<Signal> = rule
                    .required
                    .iter()
                    .copied()
                    .filter(|s| facts.has(*s))
                    .collect();
                shown.extend(rule.supporting.iter().copied().filter(|s| facts.has(*s)));
                (
                    ControlStatus::PartiallyImplemented,
                    evidence_for(facts, &shown),
                    format!("Missing {}.", join_phrases(&missing)),
                )
            } else {
                (
                    ControlStatus::NotImplemented,
                    Vec::new(),
                    format!("No evidence of {}.", join_phrases(&missing)),
                )
            }
        }
    };

    ComplianceControl {
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        description: spec.description.to_string(),
        status,
        evidence,
        notes,
    }
}

/// Score a framework's catalogue against a completed scan.
pub fn assess(framework: &ComplianceFramework, facts: &ProjectFacts) -> Vec<ComplianceControl> {
    catalogue(framework)
        .iter()
        .map(|spec| assess_control(spec, facts))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vibecody-compliance-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, body).expect("write fixture");
    }

    #[test]
    fn content_rules_all_compile() {
        assert_eq!(
            COMPILED_RULES.len(),
            CONTENT_RULES.len(),
            "a content rule failed to compile and was silently dropped"
        );
    }

    #[test]
    fn line_of_offset_counts_from_one() {
        let text = "alpha\nbeta\ngamma\n";
        assert_eq!(line_of_offset(text, 0), Some(1));
        assert_eq!(line_of_offset(text, 6), Some(2));
        assert_eq!(line_of_offset(text, 11), Some(3));
        assert_eq!(line_of_offset(text, 9_999), Some(4));
    }

    #[test]
    fn empty_project_scores_zero_not_a_hundred() {
        let root = tmp("empty");
        let facts = scan(&root);
        let controls = assess(&ComplianceFramework::SOC2, &facts);
        let scored: Vec<_> = controls
            .iter()
            .filter(|c| c.status != ControlStatus::NotAssessed)
            .collect();
        assert!(!scored.is_empty());
        assert!(
            scored
                .iter()
                .all(|c| c.status == ControlStatus::NotImplemented),
            "an empty directory must not satisfy any control"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn organisational_controls_are_not_assessed() {
        let root = tmp("organisational");
        let facts = scan(&root);
        let controls = assess(&ComplianceFramework::FedRAMP, &facts);
        let ps3 = controls
            .iter()
            .find(|c| c.id == "PS-3")
            .expect("PS-3 in catalogue");
        assert_eq!(ps3.status, ControlStatus::NotAssessed);
        assert!(ps3.notes.contains("Not assessable"));
        assert!(ps3.evidence.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn evidence_cites_the_file_that_matched() {
        let root = tmp("evidence");
        write(&root, "LICENSE", "MIT");
        write(&root, "CONTRIBUTING.md", "how to contribute");
        write(
            &root,
            "src/auth.rs",
            "fn check() {\n    let hdr = \"authorization: bearer\";\n}\n",
        );
        let facts = scan(&root);
        assert!(facts.has(Signal::License));
        assert!(facts.has(Signal::Authentication));

        let controls = assess(&ComplianceFramework::SOC2, &facts);
        let cc11 = controls.iter().find(|c| c.id == "CC1.1").expect("CC1.1");
        assert_eq!(cc11.status, ControlStatus::Implemented);
        assert!(cc11.evidence.iter().any(|e| e.contains("LICENSE")));

        let cc61 = controls.iter().find(|c| c.id == "CC6.1").expect("CC6.1");
        assert_eq!(cc61.status, ControlStatus::Implemented);
        assert!(
            cc61.evidence.iter().any(|e| e.contains("src/auth.rs:2")),
            "evidence should cite the matching line, got {:?}",
            cc61.evidence
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn partial_when_only_some_required_signals_are_present() {
        let root = tmp("partial");
        // CC8.1 requires a CI pipeline *and* a review template.
        write(&root, ".github/workflows/ci.yml", "on: push\njobs: {}\n");
        let facts = scan(&root);
        let controls = assess(&ComplianceFramework::SOC2, &facts);
        let cc81 = controls.iter().find(|c| c.id == "CC8.1").expect("CC8.1");
        assert_eq!(cc81.status, ControlStatus::PartiallyImplemented);
        assert!(cc81.notes.contains("pull-request review template"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn placeholder_values_are_not_reported_as_credentials() {
        let root = tmp("placeholder");
        write(
            &root,
            "config/settings.rs",
            "let api_key = \"your-api-key-goes-here\";\n",
        );
        let facts = scan(&root);
        assert!(
            !facts.has(Signal::HardcodedCredential),
            "a documented placeholder is not a credential"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_key_name_assigned_to_a_token_variable_is_not_a_credential() {
        // Reduced from vibewatch: keychain key names, not secrets. Lowercasing
        // the file for speed erases the only thing that separates the two, so
        // the value is re-read from the original text.
        let root = tmp("keyname");
        write(
            &root,
            "src/WatchAuthManager.swift",
            "enum KeychainKey {\n    static let accessToken = \"vibecody.watch.access_token\"\n}\n",
        );
        let facts = scan(&root);
        assert!(
            !facts.has(Signal::HardcodedCredential),
            "a dotted identifier is a key name, not a credential"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn looks_like_a_secret_separates_names_from_tokens() {
        for name in [
            "vibecody.watch.access_token",
            "app_settings_api_key",
            "some-service-token",
        ] {
            assert!(!looks_like_a_secret(name), "{name} is a name");
        }
        for secret in [
            "a83Kd92LmQ0zXvB4tR7yUw11",
            "ghp_16Cabcdefghijklmnop",
            "AKIAIOSFODNN7EXAMPLE",
        ] {
            assert!(looks_like_a_secret(secret), "{secret} is secret-shaped");
        }
    }

    #[test]
    fn mixed_case_token_survives_the_lowercased_scan() {
        let root = tmp("mixedcase");
        write(
            &root,
            "src/config.go",
            "authToken := \"ghp_16CabcdefghijklmnopQRS\"\n",
        );
        let facts = scan(&root);
        assert!(
            facts.has(Signal::HardcodedCredential),
            "a real token must still be found after the haystack is lowercased"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn real_looking_credential_literal_is_a_finding() {
        let root = tmp("credential");
        write(
            &root,
            "src/config.rs",
            "let client_secret = \"a83Kd92LmQ0zXvB4tR7yUw11\";\n",
        );
        let facts = scan(&root);
        assert!(facts.has(Signal::HardcodedCredential));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_finding_blocks_the_control_it_touches() {
        let root = tmp("blocked");
        write(&root, "src/crypto.rs", "use aes_gcm::Aes256Gcm;\n");
        write(
            &root,
            "src/config.rs",
            "let access_token = \"a83Kd92LmQ0zXvB4tR7yUw11\";\n",
        );
        let facts = scan(&root);
        let controls = assess(&ComplianceFramework::SOC2, &facts);
        let cc67 = controls.iter().find(|c| c.id == "CC6.7").expect("CC6.7");
        assert_eq!(
            cc67.status,
            ControlStatus::NotImplemented,
            "encryption at rest must not pass while a credential sits in the source"
        );
        assert!(cc67.notes.contains("finding"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fixtures_do_not_raise_credential_findings() {
        let root = tmp("fixture");
        write(
            &root,
            "tests/data/config.rs",
            "let password = \"a83Kd92LmQ0zXvB4tR7yUw11\";\n",
        );
        let facts = scan(&root);
        assert!(!facts.has(Signal::HardcodedCredential));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn docker_user_root_is_not_evidence() {
        let root = tmp("docker");
        write(&root, "Dockerfile", "FROM alpine\nUSER root\n");
        let facts = scan(&root);
        assert!(!facts.has(Signal::ContainerNonRoot));

        let root2 = tmp("docker-nonroot");
        write(&root2, "Dockerfile", "FROM alpine\nUSER root\nUSER app\n");
        let facts2 = scan(&root2);
        assert!(facts2.has(Signal::ContainerNonRoot));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&root2);
    }

    #[test]
    fn untracked_env_file_is_not_a_committed_secret() {
        let root = tmp("untracked-env");
        write(&root, ".env", "API_KEY=a83Kd92LmQ0zXvB4tR7yUw11\n");
        let facts = scan(&root);
        // Not a git checkout → the tracked-file check cannot run, so it does
        // not fire either way.
        assert!(facts.git_tracked.is_none() || !facts.has(Signal::CommittedSecretFile));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn provenance_is_absent_rather_than_invented_off_a_checkout() {
        let root = tmp("provenance");
        write(&root, "README.md", "hi");
        let facts = scan(&root);
        assert!(facts.git_commit.is_none());
        assert!(facts.git_dirty.is_none());
        assert!(facts.git_tracked.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn every_framework_has_a_non_empty_catalogue() {
        for fw in [
            ComplianceFramework::SOC2,
            ComplianceFramework::FedRAMP,
            ComplianceFramework::HIPAA,
            ComplianceFramework::GDPR,
            ComplianceFramework::ISO27001,
        ] {
            let specs = catalogue(&fw);
            assert!(specs.len() >= 10, "{fw:?} catalogue is too thin");
            let scored = specs
                .iter()
                .filter(|s| matches!(s.assessment, Assessment::Code(_)))
                .count();
            assert!(scored > 0, "{fw:?} has nothing that can be scored");
        }
    }

    #[test]
    fn control_ids_are_unique_per_framework() {
        for fw in [
            ComplianceFramework::SOC2,
            ComplianceFramework::FedRAMP,
            ComplianceFramework::HIPAA,
            ComplianceFramework::GDPR,
            ComplianceFramework::ISO27001,
        ] {
            let specs = catalogue(&fw);
            let unique: HashSet<&str> = specs.iter().map(|s| s.id).collect();
            assert_eq!(unique.len(), specs.len(), "{fw:?} has a duplicate id");
        }
    }

    #[test]
    fn findings_are_never_treated_as_evidence() {
        for fw in [
            ComplianceFramework::SOC2,
            ComplianceFramework::FedRAMP,
            ComplianceFramework::HIPAA,
            ComplianceFramework::GDPR,
            ComplianceFramework::ISO27001,
        ] {
            for spec in catalogue(&fw) {
                let Assessment::Code(rule) = &spec.assessment else {
                    continue;
                };
                for signal in rule.required.iter().chain(rule.supporting.iter()) {
                    assert!(
                        !signal.is_finding(),
                        "{} in {fw:?} counts {:?} as evidence, but it is a finding",
                        spec.id,
                        signal
                    );
                }
                for signal in rule.disqualifying {
                    assert!(
                        signal.is_finding(),
                        "{} in {fw:?} blocks on {:?}, which is not a finding",
                        spec.id,
                        signal
                    );
                }
            }
        }
    }

    #[test]
    fn every_scored_rule_names_at_least_one_required_signal() {
        for fw in [
            ComplianceFramework::SOC2,
            ComplianceFramework::FedRAMP,
            ComplianceFramework::HIPAA,
            ComplianceFramework::GDPR,
            ComplianceFramework::ISO27001,
        ] {
            for spec in catalogue(&fw) {
                if let Assessment::Code(rule) = &spec.assessment {
                    assert!(
                        !rule.required.is_empty(),
                        "{} in {fw:?} would pass on no evidence",
                        spec.id
                    );
                }
            }
        }
    }
}
