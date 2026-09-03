#![allow(dead_code, clippy::upper_case_acronyms)]
//! Readiness reports generated from a scan of the user's project.
//!
//! What this produces is a **gap / readiness assessment**, not an audit report.
//! The distinction is the whole point of the artefact: an attestation comes
//! from an independent licensed CPA, and a SOC 2 Type II in particular is
//! evidence that controls were *consistently operating* across a three-to-
//! twelve-month observation window. A scan of a source tree is a point-in-time
//! look at control **design** — closer to a Type I question, and even then only
//! at the part of the design that is visible in code. Nothing here shortens an
//! observation window or substitutes for one.
//!
//! The report model lives here; the evidence gathering and the per-framework
//! control catalogues live in [`crate::compliance_scan`]. A report is only ever
//! as good as what the scanner actually saw, so every report carries its
//! [`ScanScope`] — the directory scanned, how many files were read, and whether
//! a scan budget was hit — and the score is computed over scored controls only.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compliance_scan;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceFramework {
    SOC2,
    FedRAMP,
    HIPAA,
    GDPR,
    ISO27001,
}

impl ComplianceFramework {
    pub fn label(&self) -> &'static str {
        match self {
            ComplianceFramework::SOC2 => "SOC 2",
            ComplianceFramework::FedRAMP => "FedRAMP",
            ComplianceFramework::HIPAA => "HIPAA",
            ComplianceFramework::GDPR => "GDPR",
            ComplianceFramework::ISO27001 => "ISO 27001",
        }
    }

    /// Every framework the scanner can score, in the order a bundle files them.
    pub fn all() -> &'static [ComplianceFramework] {
        use ComplianceFramework::*;
        &[SOC2, FedRAMP, HIPAA, GDPR, ISO27001]
    }

    /// The name the CLI accepts and the stem a bundled report is filed under.
    /// Round-trips through [`ComplianceFramework::parse`] — a test pins that,
    /// because a slug that does not parse produces a bundle no one can re-run.
    pub fn slug(&self) -> &'static str {
        match self {
            ComplianceFramework::SOC2 => "soc2",
            ComplianceFramework::FedRAMP => "fedramp",
            ComplianceFramework::HIPAA => "hipaa",
            ComplianceFramework::GDPR => "gdpr",
            ComplianceFramework::ISO27001 => "iso27001",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input
            .trim()
            .to_lowercase()
            .replace([' ', '-', '_'], "")
            .as_str()
        {
            "soc2" => Some(ComplianceFramework::SOC2),
            "fedramp" | "nist80053" => Some(ComplianceFramework::FedRAMP),
            "hipaa" => Some(ComplianceFramework::HIPAA),
            "gdpr" => Some(ComplianceFramework::GDPR),
            "iso27001" | "iso" | "iso27k" => Some(ComplianceFramework::ISO27001),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceControl {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: ControlStatus,
    pub evidence: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ControlStatus {
    Implemented,
    PartiallyImplemented,
    NotImplemented,
    NotApplicable,
    /// The control cannot be evidenced by a repository — personnel screening,
    /// vendor contracts, physical access. Kept out of the score's denominator
    /// so the percentage describes only what was measured.
    NotAssessed,
}

impl ControlStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ControlStatus::Implemented => "Implemented",
            ControlStatus::PartiallyImplemented => "Partial",
            ControlStatus::NotImplemented => "Gap",
            ControlStatus::NotApplicable => "N/A",
            ControlStatus::NotAssessed => "Not assessed",
        }
    }

    /// Whether this control counts toward the compliance percentage.
    pub fn is_scored(&self) -> bool {
        matches!(
            self,
            ControlStatus::Implemented
                | ControlStatus::PartiallyImplemented
                | ControlStatus::NotImplemented
        )
    }
}

/// What the scan actually covered. A report without this cannot be trusted:
/// a truncated scan and a clean project produce the same absent evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanScope {
    pub root: String,
    pub files_seen: usize,
    pub files_read: usize,
    /// The whole-scan budget was exhausted — findings are a lower bound.
    pub truncated: bool,
    /// Files skipped for exceeding the per-file size limit.
    pub files_too_large: usize,
    /// Files tracked by git, or `None` when the root is not a git checkout
    /// (in which case the committed-credential checks did not run).
    pub git_tracked_files: Option<usize>,
    /// The commit scanned, when there was one.
    pub git_commit: Option<String>,
    /// Whether that tree had uncommitted changes. A report over a dirty tree is
    /// not reproducible from its commit, and must say so to be filed.
    pub git_dirty: Option<bool>,
    /// The tool that produced the report, so a filed copy can be re-run.
    pub tool_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub framework: ComplianceFramework,
    pub generated_at: u64,
    pub scope: ScanScope,
    pub controls: Vec<ComplianceControl>,
    pub summary: ComplianceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSummary {
    pub total_controls: usize,
    pub implemented: usize,
    pub partial: usize,
    pub not_implemented: usize,
    pub not_applicable: usize,
    pub not_assessed: usize,
    /// Controls the scan could actually decide — the percentage's denominator.
    pub scored: usize,
    /// `None` when nothing was scored: a rate over zero controls is not 0%.
    pub compliance_percentage: Option<f64>,
}

/// One walk of a project, reusable across frameworks.
///
/// The walk is the expensive half of a report and every framework asks the same
/// questions of it, so scoring a project against all five costs one scan rather
/// than five. Reports built from the same `ProjectScan` therefore share a scope
/// and a timestamp, which is also what makes them comparable: five reports taken
/// seconds apart over a tree that changed in between are five different facts.
pub struct ProjectScan {
    pub facts: compliance_scan::ProjectFacts,
    pub scope: ScanScope,
    /// Unix seconds at which the scan ran, stamped on every report built from it.
    pub scanned_at: u64,
}

impl ProjectScan {
    /// Score this one scan against `framework`.
    pub fn report(&self, framework: &ComplianceFramework) -> ComplianceReport {
        let controls = compliance_scan::assess(framework, &self.facts);
        build_report(
            framework.clone(),
            self.scope.clone(),
            controls,
            self.scanned_at,
        )
    }
}

/// Resolve a framework name, naming the accepted values on failure.
pub fn parse_framework(name: &str) -> Result<ComplianceFramework> {
    ComplianceFramework::parse(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported framework: {}. Supported: {}",
            name,
            ComplianceFramework::all()
                .iter()
                .map(|f| f.slug())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// Walk `root` once and record what the scan covered.
pub fn scan_project(root: &Path) -> Result<ProjectScan> {
    if !root.is_dir() {
        anyhow::bail!("Not a directory: {}", root.display());
    }
    // Report the path the user can act on, not the `.` they typed.
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let facts = compliance_scan::scan(&root);
    let scope = ScanScope {
        root: facts.root.clone(),
        files_seen: facts.files_seen,
        files_read: facts.files_read,
        truncated: facts.scan_truncated,
        files_too_large: facts.files_too_large,
        git_tracked_files: facts.git_tracked,
        git_commit: facts.git_commit.clone(),
        git_dirty: facts.git_dirty,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Ok(ProjectScan {
        facts,
        scope,
        scanned_at: now_unix(),
    })
}

/// Scan `root` and score it against `framework`.
pub fn generate_report_for_path(framework: &str, root: &Path) -> Result<ComplianceReport> {
    let framework = parse_framework(framework)?;
    Ok(scan_project(root)?.report(&framework))
}

/// Scan the current working directory.
pub fn generate_report_for(framework: &str) -> Result<ComplianceReport> {
    let cwd = std::env::current_dir()?;
    generate_report_for_path(framework, &cwd)
}

fn build_report(
    framework: ComplianceFramework,
    scope: ScanScope,
    controls: Vec<ComplianceControl>,
    generated_at: u64,
) -> ComplianceReport {
    let count = |want: ControlStatus| controls.iter().filter(|c| c.status == want).count();
    let implemented = count(ControlStatus::Implemented);
    let partial = count(ControlStatus::PartiallyImplemented);
    let not_impl = count(ControlStatus::NotImplemented);
    let na = count(ControlStatus::NotApplicable);
    let not_assessed = count(ControlStatus::NotAssessed);
    let scored = implemented + partial + not_impl;
    // A partially implemented control is half a control, and nothing the scan
    // could not decide is allowed to inflate the denominator.
    let pct = (scored > 0)
        .then(|| ((implemented as f64) + (partial as f64) * 0.5) / (scored as f64) * 100.0);

    ComplianceReport {
        framework,
        generated_at,
        scope,
        summary: ComplianceSummary {
            total_controls: controls.len(),
            implemented,
            partial,
            not_implemented: not_impl,
            not_applicable: na,
            not_assessed,
            scored,
            compliance_percentage: pct,
        },
        controls,
    }
}

/// Unix seconds now. A clock before the epoch is not a reason to fail a scan,
/// but it is also not a timestamp — the report says 0 rather than inventing one.
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// RFC 3339 rendering of a Unix timestamp, in UTC.
///
/// Hand-rolled rather than pulling `chrono` in for one line: the report needs a
/// timestamp a human filing it can read, not date arithmetic.
fn format_timestamp(secs: u64) -> String {
    let days_total = secs / 86_400;
    let time = secs % 86_400;
    let (mut year, mut day) = (1970_u64, days_total);
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let len = if leap { 366 } else { 365 };
        if day < len {
            break;
        }
        day -= len;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while month < 12 && day >= months[month] {
        day -= months[month];
        month += 1;
    }
    format!(
        "{year:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        month + 1,
        day + 1,
        time / 3600,
        (time % 3600) / 60,
        time % 60,
    )
}

/// Export a report as markdown.
pub fn report_to_markdown(report: &ComplianceReport) -> String {
    let s = &report.summary;
    let score = match s.compliance_percentage {
        Some(p) => format!("{p:.1}%"),
        None => "n/a (no control could be scored)".to_string(),
    };
    let mut md = format!("# {} Readiness Assessment\n\n", report.framework.label());
    md.push_str(
        "> **Gap assessment, not an audit report.** This is a point-in-time scan of \
control design as it appears in source. An attestation comes from an independent \
licensed CPA, and a Type II report additionally requires evidence that controls \
operated consistently across a three-to-twelve-month observation window — which no \
source scan can demonstrate.\n\n",
    );
    md.push_str(&format!("**Project:** `{}`\n\n", report.scope.root));
    md.push_str(&format!(
        "**Scanned:** {} · vibecli {} · commit {}{}\n\n",
        format_timestamp(report.generated_at),
        report.scope.tool_version,
        report.scope.git_commit.as_deref().unwrap_or("unknown (not a git checkout)"),
        match report.scope.git_dirty {
            Some(true) => " — **uncommitted changes present**, this report is not reproducible from the commit alone",
            Some(false) => " (clean tree)",
            None => "",
        },
    ));
    md.push_str(&format!(
        "**Compliance: {score}** over {} scored controls ({} implemented, {} partial, {} gaps). \
{} control(s) could not be assessed from source.\n\n",
        s.scored, s.implemented, s.partial, s.not_implemented, s.not_assessed,
    ));
    md.push_str(&format!(
        "Scanned {} files, read {}.{}{}{}\n\n",
        report.scope.files_seen,
        report.scope.files_read,
        if report.scope.truncated {
            " Scan budget reached — evidence below is a lower bound."
        } else {
            ""
        },
        if report.scope.files_too_large > 0 {
            format!(
                " {} file(s) were larger than the per-file limit and were not read.",
                report.scope.files_too_large
            )
        } else {
            String::new()
        },
        if report.scope.git_tracked_files.is_none() {
            " Not a git checkout, so committed-credential checks did not run."
        } else {
            ""
        },
    ));
    md.push_str("| ID | Control | Status | Evidence | Notes |\n");
    md.push_str("|---|---|---|---|---|\n");
    for c in &report.controls {
        let evidence = if c.evidence.is_empty() {
            "—".to_string()
        } else {
            c.evidence.join("<br>")
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            c.id,
            c.name,
            c.status.label(),
            evidence,
            c.notes,
        ));
    }
    md
}

// ── Bundles: one scan, every framework, filed with checksums ──────────────────

/// How a single report is rendered. `both` expands to every variant, so no
/// caller has to keep its own list of the formats that exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Markdown,
    Json,
}

impl ReportFormat {
    pub fn all() -> &'static [ReportFormat] {
        &[ReportFormat::Markdown, ReportFormat::Json]
    }

    pub fn id(&self) -> &'static str {
        match self {
            ReportFormat::Markdown => "markdown",
            ReportFormat::Json => "json",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ReportFormat::Markdown => "md",
            ReportFormat::Json => "json",
        }
    }

    /// Parse a `--format` value into the formats to emit.
    pub fn parse_list(input: &str) -> Result<Vec<ReportFormat>> {
        match input.trim().to_lowercase().as_str() {
            "markdown" | "md" => Ok(vec![ReportFormat::Markdown]),
            "json" => Ok(vec![ReportFormat::Json]),
            "both" | "all" => Ok(ReportFormat::all().to_vec()),
            other => anyhow::bail!("Unsupported format: {other}. Supported: markdown, json, both"),
        }
    }

    fn render(&self, report: &ComplianceReport) -> Result<String> {
        match self {
            ReportFormat::Markdown => Ok(report_to_markdown(report)),
            ReportFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        }
    }
}

/// A rendered report, with the digest that will be filed next to it.
#[derive(Debug, Clone)]
pub struct BundledReport {
    pub framework: ComplianceFramework,
    pub format: ReportFormat,
    /// File name relative to the output directory.
    pub file: String,
    pub contents: String,
    pub sha256: String,
    pub summary: ComplianceSummary,
}

/// The index filed with a bundle: provenance, the scope of the one scan that
/// produced every report, and a digest per file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub tool: String,
    pub tool_version: String,
    pub generated_at: u64,
    pub generated_at_utc: String,
    /// Named rather than assumed, so a verifier does not have to guess.
    pub checksum_algorithm: String,
    /// The single scan behind every report listed below.
    pub scope: ScanScope,
    pub reports: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub framework: String,
    pub framework_id: String,
    pub format: String,
    pub file: String,
    pub bytes: usize,
    pub sha256: String,
    pub total_controls: usize,
    pub scored: usize,
    /// `None` when nothing could be scored. A rate over zero controls is not 0%.
    pub compliance_percentage: Option<f64>,
}

/// Everything a bundle writes to disk. Rendering and hashing happen here;
/// the caller does the IO, which is what makes this testable without one.
#[derive(Debug, Clone)]
pub struct ReportBundle {
    pub scope: ScanScope,
    pub generated_at: u64,
    pub reports: Vec<BundledReport>,
    /// `manifest.json`.
    pub manifest_file: String,
    pub manifest_contents: String,
    /// `SHA256SUMS`, in the format `shasum -a 256 -c` reads.
    pub checksums_file: String,
    pub checksums_contents: String,
}

pub const MANIFEST_FILE: &str = "manifest.json";
pub const CHECKSUMS_FILE: &str = "SHA256SUMS";

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Scan `root` once and render every (framework, format) pair from that one scan.
pub fn build_bundle(
    root: &Path,
    frameworks: &[ComplianceFramework],
    formats: &[ReportFormat],
) -> Result<ReportBundle> {
    if frameworks.is_empty() {
        anyhow::bail!("No framework selected");
    }
    if formats.is_empty() {
        anyhow::bail!("No output format selected");
    }
    let scan = scan_project(root)?;
    let reports = frameworks
        .iter()
        .map(|framework| {
            let report = scan.report(framework);
            formats
                .iter()
                .map(|format| {
                    let contents = format.render(&report)?;
                    Ok(BundledReport {
                        framework: framework.clone(),
                        format: *format,
                        file: format!("{}.{}", framework.slug(), format.extension()),
                        sha256: sha256_hex(contents.as_bytes()),
                        contents,
                        summary: report.summary.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let manifest = Manifest {
        tool: "vibecli".to_string(),
        tool_version: scan.scope.tool_version.clone(),
        generated_at: scan.scanned_at,
        generated_at_utc: format_timestamp(scan.scanned_at),
        checksum_algorithm: "sha256".to_string(),
        scope: scan.scope.clone(),
        reports: reports
            .iter()
            .map(|r| ManifestEntry {
                framework: r.framework.label().to_string(),
                framework_id: r.framework.slug().to_string(),
                format: r.format.id().to_string(),
                file: r.file.clone(),
                bytes: r.contents.len(),
                sha256: r.sha256.clone(),
                total_controls: r.summary.total_controls,
                scored: r.summary.scored,
                compliance_percentage: r.summary.compliance_percentage,
            })
            .collect(),
    };
    let checksums = reports
        .iter()
        .map(|r| format!("{}  {}\n", r.sha256, r.file))
        .collect::<String>();

    Ok(ReportBundle {
        scope: scan.scope,
        generated_at: scan.scanned_at,
        reports,
        manifest_file: MANIFEST_FILE.to_string(),
        manifest_contents: serde_json::to_string_pretty(&manifest)? + "\n",
        checksums_file: CHECKSUMS_FILE.to_string(),
        checksums_contents: checksums,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vibecody-compliance-report-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create fixture dir");
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
    fn every_framework_slug_parses_back() {
        for framework in ComplianceFramework::all() {
            assert_eq!(
                ComplianceFramework::parse(framework.slug()).as_ref(),
                Some(framework),
                "slug {} must round-trip, or a bundled report cannot be re-run",
                framework.slug()
            );
        }
    }

    #[test]
    fn format_parse_list_rejects_what_it_cannot_render() {
        assert_eq!(
            ReportFormat::parse_list("both").expect("both"),
            ReportFormat::all().to_vec()
        );
        assert_eq!(
            ReportFormat::parse_list("md").expect("md"),
            vec![ReportFormat::Markdown]
        );
        assert!(ReportFormat::parse_list("pdf").is_err());
    }

    #[test]
    fn a_bundle_files_every_framework_in_every_format() {
        let root = fixture("bundle-coverage");
        write(&root, "LICENSE", "MIT");
        let bundle =
            build_bundle(&root, ComplianceFramework::all(), ReportFormat::all()).expect("bundle");
        assert_eq!(
            bundle.reports.len(),
            ComplianceFramework::all().len() * ReportFormat::all().len()
        );
        for framework in ComplianceFramework::all() {
            for format in ReportFormat::all() {
                let want = format!("{}.{}", framework.slug(), format.extension());
                assert!(
                    bundle.reports.iter().any(|r| r.file == want),
                    "{want} missing from the bundle"
                );
            }
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn manifest_and_checksums_match_the_rendered_bytes() {
        let root = fixture("bundle-checksums");
        write(&root, "LICENSE", "MIT");
        let bundle = build_bundle(&root, ComplianceFramework::all(), &[ReportFormat::Markdown])
            .expect("bundle");
        let manifest: Manifest =
            serde_json::from_str(&bundle.manifest_contents).expect("manifest is json");
        assert_eq!(manifest.checksum_algorithm, "sha256");
        assert_eq!(manifest.reports.len(), bundle.reports.len());

        for report in &bundle.reports {
            let digest = sha256_hex(report.contents.as_bytes());
            assert_eq!(digest, report.sha256, "digest of {}", report.file);
            let entry = manifest
                .reports
                .iter()
                .find(|e| e.file == report.file)
                .unwrap_or_else(|| panic!("{} missing from manifest", report.file));
            assert_eq!(entry.sha256, digest);
            assert_eq!(entry.bytes, report.contents.len());
            assert_eq!(
                entry.compliance_percentage,
                report.summary.compliance_percentage
            );
            // `shasum -a 256 -c` reads exactly this shape.
            assert!(
                bundle
                    .checksums_contents
                    .contains(&format!("{digest}  {}\n", report.file)),
                "{} missing from SHA256SUMS",
                report.file
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn every_report_in_a_bundle_comes_from_the_same_scan() {
        let root = fixture("bundle-one-scan");
        write(&root, "LICENSE", "MIT");
        let bundle =
            build_bundle(&root, ComplianceFramework::all(), &[ReportFormat::Json]).expect("bundle");
        let reports: Vec<ComplianceReport> = bundle
            .reports
            .iter()
            .map(|r| serde_json::from_str(&r.contents).expect("report is json"))
            .collect();
        let first = reports.first().expect("at least one report");
        for report in &reports {
            assert_eq!(
                report.generated_at, first.generated_at,
                "reports from one scan must share its timestamp"
            );
            assert_eq!(report.scope.root, first.scope.root);
            assert_eq!(report.scope.files_seen, first.scope.files_seen);
            assert_eq!(report.scope.files_read, first.scope.files_read);
        }
        assert_eq!(bundle.generated_at, first.generated_at);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_bundle_with_no_framework_is_an_error_not_an_empty_manifest() {
        let root = fixture("bundle-empty");
        assert!(build_bundle(&root, &[], ReportFormat::all()).is_err());
        assert!(build_bundle(&root, ComplianceFramework::all(), &[]).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn framework_parse_accepts_the_names_the_ui_sends() {
        for name in ["SOC2", "soc 2", "FedRAMP", "HIPAA", "GDPR", "ISO27001"] {
            assert!(
                ComplianceFramework::parse(name).is_some(),
                "{name} should parse"
            );
        }
        assert!(ComplianceFramework::parse("PCI-DSS").is_none());
    }

    #[test]
    fn unsupported_framework_is_an_error_not_a_placeholder_report() {
        let root = fixture("unsupported");
        let err = generate_report_for_path("pci-dss", &root);
        assert!(err.is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_directory_is_an_error() {
        let missing = std::env::temp_dir().join("vibecody-compliance-does-not-exist");
        assert!(generate_report_for_path("soc2", &missing).is_err());
    }

    #[test]
    fn empty_project_does_not_score_a_hundred_percent() {
        let root = fixture("empty");
        let report = generate_report_for_path("soc2", &root).expect("report");
        assert_eq!(report.summary.compliance_percentage, Some(0.0));
        assert!(report.summary.not_implemented > 0);
        assert!(report.summary.not_assessed > 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn unassessable_controls_stay_out_of_the_denominator() {
        let root = fixture("denominator");
        let report = generate_report_for_path("fedramp", &root).expect("report");
        let s = &report.summary;
        assert_eq!(s.scored, s.implemented + s.partial + s.not_implemented);
        assert_eq!(
            s.total_controls,
            s.scored + s.not_assessed + s.not_applicable
        );
        assert!(
            s.scored < s.total_controls,
            "FedRAMP has organisational controls"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn report_reflects_the_scanned_project_not_vibecody() {
        let root = fixture("reflects");
        write(&root, "LICENSE", "MIT");
        write(&root, "CONTRIBUTING.md", "contribute");
        write(&root, "README.md", "a project");
        let report = generate_report_for_path("soc2", &root).expect("report");
        // The report canonicalises the root, and on macOS `/var` resolves to
        // `/private/var` — compare canonical paths, not the string passed in.
        let expected = root.canonicalize().unwrap_or_else(|_| root.clone());
        assert_eq!(report.scope.root, expected.display().to_string());
        assert!(report.scope.files_seen >= 3);
        let cc11 = report
            .controls
            .iter()
            .find(|c| c.id == "CC1.1")
            .expect("CC1.1");
        assert_eq!(cc11.status, ControlStatus::Implemented);
        assert!(cc11.evidence.iter().any(|e| e.contains("LICENSE")));
        // Nothing in this fixture proves encryption at rest.
        let cc67 = report
            .controls
            .iter()
            .find(|c| c.id == "CC6.7")
            .expect("CC6.7");
        assert_eq!(cc67.status, ControlStatus::NotImplemented);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn percentage_rises_only_with_evidence() {
        let bare = fixture("bare");
        let bare_report = generate_report_for_path("soc2", &bare).expect("report");

        let equipped = fixture("equipped");
        write(&equipped, "LICENSE", "MIT");
        write(&equipped, "CONTRIBUTING.md", "contribute");
        write(&equipped, "README.md", "docs");
        write(&equipped, "SECURITY.md", "report issues here");
        write(&equipped, ".github/workflows/ci.yml", "run: cargo test\n");
        write(&equipped, ".github/pull_request_template.md", "checklist");
        write(&equipped, "Cargo.lock", "# lock");
        write(&equipped, "src/auth.rs", "fn f() { require_auth(); }\n");
        let equipped_report = generate_report_for_path("soc2", &equipped).expect("report");

        let bare_pct = bare_report.summary.compliance_percentage.unwrap_or(0.0);
        let equipped_pct = equipped_report.summary.compliance_percentage.unwrap_or(0.0);
        assert!(
            equipped_pct > bare_pct,
            "evidence should move the score: {bare_pct} -> {equipped_pct}"
        );
        assert!(
            equipped_pct < 100.0,
            "a fixture with no crypto or backups must not be fully compliant"
        );
        let _ = fs::remove_dir_all(&bare);
        let _ = fs::remove_dir_all(&equipped);
    }

    #[test]
    fn markdown_carries_the_scope_and_the_gaps() {
        let root = fixture("markdown");
        write(&root, "README.md", "hello");
        let report = generate_report_for_path("gdpr", &root).expect("report");
        let md = report_to_markdown(&report);
        assert!(md.contains("# GDPR Readiness Assessment"));
        assert!(
            md.contains("not an audit report"),
            "the artefact must not be mistakable for an attestation"
        );
        assert!(md.contains(&root.display().to_string()));
        assert!(md.contains("could not be assessed from source"));
        assert!(md.contains("| ID | Control | Status | Evidence | Notes |"));
        assert!(md.contains("Art. 17"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn every_framework_produces_a_report() {
        let root = fixture("all-frameworks");
        write(&root, "README.md", "hello");
        for fw in ["soc2", "fedramp", "hipaa", "gdpr", "iso27001"] {
            let report = generate_report_for_path(fw, &root)
                .unwrap_or_else(|e| panic!("{fw} should produce a report: {e}"));
            assert!(
                report.controls.len() >= 10,
                "{fw} returned only {} controls",
                report.controls.len()
            );
            assert!(
                report.summary.scored > 0,
                "{fw} scored nothing at all — the percentage would be meaningless"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn format_timestamp_renders_utc() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_timestamp(1_000_000_000), "2001-09-09T01:46:40Z");
        // A leap day, to catch the february branch.
        assert_eq!(format_timestamp(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn markdown_states_provenance_or_says_it_is_unknown() {
        let root = fixture("provenance-md");
        write(&root, "README.md", "hi");
        let report = generate_report_for_path("soc2", &root).expect("report");
        let md = report_to_markdown(&report);
        assert!(md.contains("vibecli "));
        assert!(
            md.contains("unknown (not a git checkout)"),
            "a fixture directory has no commit, and the report must say so"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn report_serde_roundtrip() {
        let root = fixture("serde");
        write(&root, "README.md", "hello");
        let report = generate_report_for_path("soc2", &root).expect("report");
        let json = serde_json::to_string(&report).expect("serialize");
        let parsed: ComplianceReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.framework, ComplianceFramework::SOC2);
        assert_eq!(parsed.controls.len(), report.controls.len());
        assert_eq!(parsed.summary.scored, report.summary.scored);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn summary_counts_partition_the_catalogue() {
        let root = fixture("partition");
        write(&root, "README.md", "hello");
        let report = generate_report_for_path("iso27001", &root).expect("report");
        let s = &report.summary;
        assert_eq!(
            s.total_controls,
            s.implemented + s.partial + s.not_implemented + s.not_applicable + s.not_assessed
        );
        let _ = fs::remove_dir_all(&root);
    }
}
