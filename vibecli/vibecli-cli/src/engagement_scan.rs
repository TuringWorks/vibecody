#![allow(dead_code)]
//! Workspace scan — find artifacts that might back an engagement deliverable.
//!
//! The gap this closes: VibeCody's panels *produce* things, and the engagement
//! record has to be told about them by hand. A scan walks the workspace and
//! proposes which existing files could serve as evidence for which deliverable.
//!
//! ## What a scan is allowed to conclude
//!
//! **A file existing is not a deliverable.** `docs/threat-model.md` may be a
//! finished threat model, an empty heading, or a template somebody copied in
//! three years ago. So the scan:
//!
//! * proposes **candidates**, never facts;
//! * **never changes a deliverable's status** — attaching evidence is as far as
//!   it goes, and even that is opt-in;
//! * labels what it attaches `detected: <rule>`, so a reviewer can tell a
//!   machine's guess from a human's assertion;
//! * has **no rule at all** for deliverables it cannot honestly detect. There
//!   is no filename that means "the production system works" or "we paired with
//!   your engineers". A missing rule is the correct answer there, and inventing
//!   a weak one would convert "unknown" into "probably fine" — the exact
//!   substitution [AGENTS.md](../../../AGENTS.md#modelling-honesty--a-model-that-cannot-be-wrong-is-not-a-model)
//!   forbids.
//!
//! Caps are reported, never silent: a scan that stopped early says so, because
//! a truncated result that looks complete reads as "there is nothing else".

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// Directory names never worth walking. Cheap to skip, expensive to descend.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".nuxt",
    ".gradle",
    ".idea",
    ".mypy_cache",
    ".pytest_cache",
    "Pods",
    "DerivedData",
];

/// Bounds. A scan is interactive, so it must finish; when it stops early the
/// report says so rather than presenting a partial answer as a whole one.
const MAX_DEPTH: usize = 7;
const MAX_FILES: usize = 40_000;
const MAX_CANDIDATES: usize = 400;

/// One detection rule.
///
/// `needles` match against the lowercased, forward-slashed path relative to the
/// workspace root. `extensions` narrows by suffix; empty means any suffix.
pub struct Rule {
    /// Deliverable key this rule proposes evidence for.
    pub key: &'static str,
    /// Human-readable reason, shown to the reviewer and stored on the evidence.
    pub label: &'static str,
    pub needles: &'static [&'static str],
    pub extensions: &'static [&'static str],
}

/// Document suffixes — the deliverables that are written down.
const DOCS: &[&str] = &[".md", ".markdown", ".adoc", ".rst", ".txt", ".pdf", ".docx"];

macro_rules! r {
    ($key:literal, $label:literal, [$($needle:literal),* $(,)?], $ext:expr) => {
        Rule {
            key: $key,
            label: $label,
            needles: &[$($needle),*],
            extensions: $ext,
        }
    };
}

/// Detection rules.
///
/// Deliberately absent: `production-system`, `pairing-log`,
/// `enablement-sessions` attendance, and `sla-managed-operation` signature.
/// No path pattern is evidence that a system runs in production, that two
/// engineers sat together, or that a client signed something. Those stay
/// human-attested.
pub static RULES: &[Rule] = &[
    // ── Discover ──────────────────────────────────────────────────────────
    r!(
        "current-state-architecture-map",
        "architecture document or diagram",
        ["architecture", "system-design", "c4-model", "context-diagram"],
        &[".md", ".markdown", ".adoc", ".drawio", ".mmd", ".svg", ".puml", ".pdf"]
    ),
    r!(
        "system-inventory",
        "service inventory or catalog",
        ["inventory", "service-catalog", "catalog-info", "system-catalog"],
        &[".md", ".yaml", ".yml", ".csv", ".json"]
    ),
    r!(
        "stakeholder-interviews",
        "interview notes",
        ["interview", "stakeholder-notes", "discovery-notes"],
        DOCS
    ),
    r!(
        "functional-requirements",
        "requirements or user stories",
        ["functional-requirement", "user-stories", "user-story", "/prd", "prd-"],
        DOCS
    ),
    r!(
        "non-functional-requirements",
        "NFR document",
        ["non-functional", "nfr.", "nfr-", "/nfr", "quality-attributes"],
        DOCS
    ),
    r!(
        "risk-register",
        "risk register",
        ["risk-register", "risk-log", "risks."],
        &[".md", ".csv", ".xlsx", ".yaml", ".yml"]
    ),
    r!(
        "dependency-register",
        "dependency inventory or SBOM",
        ["sbom", "cyclonedx", "spdx", "dependency-register", "dependencies."],
        &[".md", ".json", ".xml", ".csv", ".yaml", ".yml"]
    ),
    r!(
        "tech-debt-register",
        "technical-debt register",
        ["tech-debt", "technical-debt", "debt-register"],
        &[".md", ".csv", ".yaml", ".yml"]
    ),
    r!(
        "prioritized-roadmap",
        "roadmap",
        ["roadmap", "delivery-plan", "sequencing"],
        DOCS
    ),
    // ── Prove ─────────────────────────────────────────────────────────────
    r!(
        "success-criteria",
        "success or acceptance criteria",
        ["success-criteria", "acceptance-criteria", "exit-criteria"],
        DOCS
    ),
    r!(
        "measured-results",
        "measured results",
        ["benchmark", "eval-results", "measured-results", "pilot-results"],
        &[".md", ".json", ".csv", ".html"]
    ),
    r!(
        "cost-model",
        "cost model",
        // "tco" bare matches "ou-tco-mes". Delimit it.
        ["cost-model", "cost-estimate", "tco.", "tco-", "/tco", "pricing-model"],
        &[".md", ".csv", ".xlsx", ".json"]
    ),
    r!(
        "go-no-go",
        "go / no-go recommendation",
        ["go-no-go", "go_no_go", "recommendation"],
        DOCS
    ),
    // ── Build ─────────────────────────────────────────────────────────────
    r!(
        "infrastructure-as-code",
        "infrastructure as code",
        [
            "terraform/",
            "pulumi",
            "cloudformation",
            "/helm/",
            "charts/",
            "ansible",
            "main.tf",
            "kustomization"
        ],
        &[".tf", ".tfvars", ".yaml", ".yml", ".json", ".ts", ".py"]
    ),
    r!(
        "ci-cd-pipelines",
        "CI/CD pipeline definition",
        [
            ".github/workflows/",
            ".gitlab-ci",
            "jenkinsfile",
            "azure-pipelines",
            ".circleci/",
            "buildkite",
            ".drone"
        ],
        &[]
    ),
    r!(
        "test-coverage",
        "coverage report",
        ["lcov.info", "coverage.xml", "cobertura", "tarpaulin-report", "coverage-summary"],
        &[]
    ),
    r!(
        "observability",
        "telemetry or dashboard configuration",
        ["opentelemetry", "otel-", "otel.", "grafana", "dashboards/", "jaeger"],
        &[".yaml", ".yml", ".json", ".toml", ".md"]
    ),
    r!(
        "alerting-and-slos",
        "SLO or alert definition",
        // "slo" bare is too loose: scanning this repository it matched
        // `warehouse-slotting-agent.md`. Every needle here ends at a
        // delimiter, so "slot", "slope" and "slog" no longer qualify.
        [
            "slo.",
            "slos.",
            "slo/",
            "slos/",
            "slo-",
            "slos-",
            "alerting",
            "alertmanager",
            "alerts.",
            "prometheus"
        ],
        &[".yaml", ".yml", ".json", ".md", ".rules"]
    ),
    r!(
        "threat-model",
        "threat model",
        ["threat-model", "threatmodel", "stride", "attack-tree"],
        &[".md", ".json", ".yaml", ".yml", ".pdf"]
    ),
    r!(
        "compliance-evidence",
        "compliance artifact",
        ["compliance", "soc2", "soc-2", "iso27001", "hipaa", "pci-dss", "audit-evidence"],
        &[".md", ".pdf", ".csv", ".json", ".yaml", ".yml"]
    ),
    r!(
        "architecture-decision-records",
        "architecture decision record",
        ["/adr/", "adr-", "decision-record", "/decisions/"],
        &[".md", ".markdown", ".adoc"]
    ),
    // ── Operate ───────────────────────────────────────────────────────────
    r!("runbooks", "runbook", ["runbook", "playbook"], DOCS),
    r!(
        "escalation-paths",
        "escalation path",
        ["escalation"],
        DOCS
    ),
    r!(
        "oncall-handbook",
        "on-call handbook",
        ["on-call", "oncall", "rotation"],
        DOCS
    ),
    r!(
        "exit-plan",
        "exit or offboarding plan",
        ["exit-plan", "offboarding", "transition-plan", "handover"],
        DOCS
    ),
];

// ── Results ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceCandidate {
    /// Deliverable this file might back. A proposal, not a finding.
    pub deliverable_key: String,
    /// Why the scan thinks so.
    pub rule: String,
    /// Path relative to the scan root, forward-slashed.
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub root: String,
    pub files_scanned: usize,
    pub candidates: Vec<EvidenceCandidate>,
    /// True when a cap stopped the walk. A truncated list that looked complete
    /// would read as "there is nothing else here".
    pub truncated: bool,
    /// Deliverable keys that have no detection rule at all, so their absence
    /// from `candidates` means nothing. Reported so a reader does not conclude
    /// "the scan found nothing, therefore nothing exists".
    pub undetectable: Vec<String>,
    pub notes: Vec<String>,
}

/// Deliverable keys the scan is structurally unable to judge.
pub fn undetectable_keys() -> Vec<String> {
    let detected: std::collections::HashSet<&str> = RULES.iter().map(|r| r.key).collect();
    crate::engagement::TEMPLATE
        .iter()
        .map(|t| t.key)
        .filter(|k| !detected.contains(k))
        .map(str::to_string)
        .collect()
}

fn normalise(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/").to_lowercase()
}

/// Does `rule` match this relative path?
pub fn rule_matches(rule: &Rule, normalised_path: &str) -> bool {
    let needle_hit = rule.needles.iter().any(|n| normalised_path.contains(n));
    if !needle_hit {
        return false;
    }
    rule.extensions.is_empty()
        || rule
            .extensions
            .iter()
            .any(|e| normalised_path.ends_with(e))
}

/// Walk `root` and propose evidence.
pub fn scan(root: impl AsRef<Path>) -> Result<ScanReport> {
    let root = root.as_ref();
    let root_display = root.to_string_lossy().to_string();
    if !root.is_dir() {
        anyhow::bail!("scan root {root_display:?} is not a directory");
    }

    let mut candidates: Vec<EvidenceCandidate> = Vec::new();
    let mut files_scanned = 0usize;
    let mut truncated = false;

    let walker = WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Keep the root itself; skip known-noise directories by name.
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !(e.file_type().is_dir() && SKIP_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d)))
        });

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            // An unreadable directory is not a reason to abandon the scan; it
            // is a reason to say the scan was incomplete.
            Err(_) => {
                truncated = true;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        files_scanned += 1;
        if files_scanned > MAX_FILES {
            truncated = true;
            break;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let normalised = normalise(rel);
        let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        for rule in RULES {
            if rule_matches(rule, &normalised) {
                candidates.push(EvidenceCandidate {
                    deliverable_key: rule.key.to_string(),
                    rule: rule.label.to_string(),
                    path: normalised.clone(),
                    bytes,
                });
                if candidates.len() >= MAX_CANDIDATES {
                    truncated = true;
                    break;
                }
            }
        }
        if truncated {
            break;
        }
    }

    candidates.sort_by(|a, b| {
        a.deliverable_key
            .cmp(&b.deliverable_key)
            .then_with(|| a.path.cmp(&b.path))
    });
    candidates.dedup();

    let undetectable = undetectable_keys();
    let mut notes = vec![
        "A file existing is not a deliverable. These are candidates for review, \
         not findings."
            .to_string(),
    ];
    if truncated {
        notes.push(format!(
            "Scan stopped early (depth {MAX_DEPTH}, {MAX_FILES} files, or \
             {MAX_CANDIDATES} candidates). The list is incomplete."
        ));
    }
    if !undetectable.is_empty() {
        notes.push(format!(
            "{} deliverable(s) have no detection rule — their absence here means \
             nothing was looked for, not that nothing exists.",
            undetectable.len()
        ));
    }

    Ok(ScanReport {
        root: root_display,
        files_scanned,
        candidates,
        truncated,
        undetectable,
        notes,
    })
}

/// Attach a scan's candidates as evidence.
///
/// Deliverable statuses are untouched — attaching a file says "here is
/// something related", not "this is done". The label carries `detected:` so a
/// reviewer can always tell a machine's guess from a human's assertion.
pub fn attach(
    store: &crate::engagement::EngagementStore,
    engagement_id: &str,
    report: &ScanReport,
) -> Result<usize> {
    let mut attached = 0usize;
    for c in &report.candidates {
        let Some(d) = store
            .deliverable_by_key(engagement_id, &c.deliverable_key)
            .with_context(|| format!("look up deliverable {}", c.deliverable_key))?
        else {
            continue;
        };
        // Don't attach the same path twice across repeated scans.
        let existing = store.evidence(&d.id)?;
        if existing.iter().any(|e| e.reference == c.path) {
            continue;
        }
        store.add_evidence(
            &d.id,
            crate::engagement::EvidenceKind::File,
            &format!("detected: {}", c.rule),
            &c.path,
        )?;
        attached += 1;
    }
    Ok(attached)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(&p, b"x").expect("write");
    }

    #[test]
    fn every_rule_targets_a_real_deliverable_key() {
        // A rule pointing at a key the template does not have would propose
        // evidence for a deliverable that cannot receive it — silently, since
        // `attach` skips unknown keys.
        let keys: std::collections::HashSet<&str> = crate::engagement::TEMPLATE
            .iter()
            .map(|t| t.key)
            .collect();
        for r in RULES {
            assert!(
                keys.contains(r.key),
                "rule '{}' targets unknown deliverable key '{}'",
                r.label,
                r.key
            );
            assert!(!r.needles.is_empty(), "rule '{}' matches nothing", r.label);
        }
    }

    #[test]
    fn undetectable_keys_are_named_not_hidden() {
        let u = undetectable_keys();
        // These are the ones no filename can honestly evidence.
        assert!(u.iter().any(|k| k == "production-system"));
        assert!(u.iter().any(|k| k == "pairing-log"));
        // And the ones that do have rules must not appear.
        assert!(!u.iter().any(|k| k == "threat-model"));
    }

    #[test]
    fn finds_the_obvious_artifacts() {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        touch(root, "docs/architecture.md");
        touch(root, "docs/threat-model.md");
        touch(root, "docs/adr/0001-use-postgres.md");
        touch(root, ".github/workflows/ci.yml");
        touch(root, "terraform/main.tf");
        touch(root, "docs/runbook-database.md");
        touch(root, "docs/roadmap.md");

        let report = scan(root).expect("scan");
        let keys: Vec<&str> = report
            .candidates
            .iter()
            .map(|c| c.deliverable_key.as_str())
            .collect();
        for expected in [
            "current-state-architecture-map",
            "threat-model",
            "architecture-decision-records",
            "ci-cd-pipelines",
            "infrastructure-as-code",
            "runbooks",
            "prioritized-roadmap",
        ] {
            assert!(keys.contains(&expected), "missed {expected}: {keys:?}");
        }
        assert!(!report.truncated);
    }

    #[test]
    fn skips_noise_directories() {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        touch(root, "node_modules/pkg/docs/architecture.md");
        touch(root, "target/debug/threat-model.md");
        touch(root, ".git/roadmap.md");

        let report = scan(root).expect("scan");
        assert!(
            report.candidates.is_empty(),
            "vendored files must not be proposed as this engagement's evidence: {:?}",
            report.candidates
        );
    }

    #[test]
    fn loose_acronyms_do_not_match_longer_words() {
        // Found by running the scan over this repository: "slo" matched
        // `warehouse-slotting-agent.md` and proposed a warehouse skill as an
        // SLO definition. Acronym needles must end at a delimiter.
        let slo = RULES
            .iter()
            .find(|r| r.key == "alerting-and-slos")
            .expect("slo rule");
        assert!(!rule_matches(
            slo,
            "skills/transportation-warehouse-slotting-agent.md"
        ));
        assert!(rule_matches(slo, "monitoring/slo.yaml"));
        assert!(rule_matches(slo, "docs/slo-definitions.md"));

        let cost = RULES
            .iter()
            .find(|r| r.key == "cost-model")
            .expect("cost rule");
        assert!(!rule_matches(cost, "docs/outcomes.md"));
        assert!(rule_matches(cost, "docs/tco-2026.md"));
    }

    #[test]
    fn extension_filter_keeps_source_files_out() {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        // A source file about cost is not a cost model.
        touch(root, "src/cost-model.rs");
        touch(root, "docs/cost-model.md");

        let report = scan(root).expect("scan");
        let paths: Vec<&str> = report.candidates.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains(&"docs/cost-model.md"));
        assert!(!paths.contains(&"src/cost-model.rs"));
    }

    #[test]
    fn a_report_always_says_a_file_is_not_a_deliverable() {
        let dir = TempDir::new().expect("tempdir");
        let report = scan(dir.path()).expect("scan");
        assert!(report
            .notes
            .iter()
            .any(|n| n.contains("not a deliverable")));
        // And it always names what it could not look for.
        assert!(!report.undetectable.is_empty());
    }

    #[test]
    fn scanning_a_file_instead_of_a_directory_is_an_error() {
        let dir = TempDir::new().expect("tempdir");
        let f = dir.path().join("x.md");
        fs::write(&f, b"x").expect("write");
        assert!(scan(&f).is_err());
    }

    #[test]
    fn attach_creates_evidence_but_never_changes_status() {
        use crate::engagement::{DeliverableStatus, EngagementStore};
        let store_dir = TempDir::new().expect("tempdir");
        let store =
            EngagementStore::open(store_dir.path().join("e.db")).expect("open");
        let e = store.create("Acme", "Acme", None, "").expect("create");

        let ws = TempDir::new().expect("tempdir");
        touch(ws.path(), "docs/threat-model.md");
        let report = scan(ws.path()).expect("scan");

        let attached = attach(&store, &e.id, &report).expect("attach");
        assert!(attached > 0);

        let d = store
            .deliverable_by_key(&e.id, "threat-model")
            .expect("query")
            .expect("present");
        assert_eq!(
            d.status,
            DeliverableStatus::NotStarted,
            "a detected file must not mark a deliverable as done"
        );
        assert_eq!(d.evidence_count, 1);
        let ev = store.evidence(&d.id).expect("evidence");
        assert!(
            ev[0].label.starts_with("detected: "),
            "a machine's guess must be labelled as one"
        );

        // Re-scanning must not pile up duplicates.
        let again = attach(&store, &e.id, &report).expect("attach again");
        assert_eq!(again, 0);
    }
}
