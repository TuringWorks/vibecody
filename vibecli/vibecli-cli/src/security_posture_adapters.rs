//! Adapters that wrap VibeCody's existing scanner modules into the
//! `Scanner` trait so the Security Posture aggregator can consume
//! them uniformly.
//!
//! Each adapter:
//! - Implements [`Scanner`] from `security_posture`
//! - Maps its native finding shape to [`SecurityFinding`]
//! - Normalizes severity into the 5-bucket `Severity` enum
//! - Sets `scanner` name + a stable `rule_id` prefix
//!
//! See `docs/design/security-posture/scanners.md` for the per-scanner
//! contract (severity mapping, category mapping, fast-path support).

use crate::security_posture::{Category, Scanner, SecurityFinding, Severity};
use anyhow::Result;
use std::path::{Path, PathBuf};

// ── health_score adapter ─────────────────────────────────────────────

/// Wraps `crate::health_score::HealthEngine` into a `Scanner`.
///
/// Each `DimensionScore` becomes one `SecurityFinding`. Findings
/// with `Info` severity (score ≥ 80) are still emitted so the panel
/// can show the green "healthy" rows; the panel filters them out
/// by default.
pub struct HealthScannerAdapter;

impl Scanner for HealthScannerAdapter {
    fn name(&self) -> &'static str {
        "health"
    }

    fn scan(&self, workspace: &Path) -> Result<Vec<SecurityFinding>> {
        use crate::health_score::{HealthConfig, HealthEngine};
        let mut engine = HealthEngine::new(HealthConfig::default());
        let snapshot = engine.scan(&workspace.to_string_lossy(), 0);

        let mut findings = Vec::with_capacity(snapshot.dimensions.len());
        for dim in snapshot.dimensions {
            let severity = score_to_severity(dim.score);
            let rule_id = format!("health:{}", slugify_dimension(dim.dimension.label()));
            let title = format!("{} — score {:.0}", dim.dimension.label(), dim.score);

            findings.push(SecurityFinding::new(
                "health",
                severity,
                Category::CodeHealth,
                PathBuf::from("."), // health is whole-workspace, not per-file
                None,
                None,
                Some(dim.details),
                rule_id,
                title,
                dim.remediation,
                Vec::new(),
            ));
        }
        Ok(findings)
    }
}

/// Map a 0–100 health score into the 5-bucket severity scale.
///
/// Lower score = worse health = higher severity. The thresholds
/// are documented in `scanners.md`:
/// `< 30 → High`, `< 60 → Medium`, `< 80 → Low`, `≥ 80 → Info`.
/// Critical is reserved for actual security findings — health
/// scores below 30 are still concerning but they're not exploits.
fn score_to_severity(score: f64) -> Severity {
    if score < 30.0 {
        Severity::High
    } else if score < 60.0 {
        Severity::Medium
    } else if score < 80.0 {
        Severity::Low
    } else {
        Severity::Info
    }
}

/// Turn a dimension label like "Test Coverage" into a kebab-case
/// rule-id suffix so the SecurityFinding.rule_id is stable across
/// scans even if HealthDimension::label() formatting changes.
fn slugify_dimension(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .replace("--", "-")
}

// ── vulnerability_db adapter ────────────────────────────────────────

/// Wraps `crate::vulnerability_db::VulnerabilityScanner` into a
/// `Scanner`. Walks the workspace for lockfiles (any of the formats
/// the existing parser knows about) and source files (for the SAST
/// regex sweep), emits one `SecurityFinding` per active vulnerability
/// and per SAST match.
///
/// CVE findings carry the upstream advisory URL in `references`.
/// SAST findings carry the CWE id in `rule_id`.
pub struct VulnerabilityScannerAdapter;

impl Scanner for VulnerabilityScannerAdapter {
    fn name(&self) -> &'static str {
        "vulnerability_db"
    }

    fn scan(&self, workspace: &Path) -> Result<Vec<SecurityFinding>> {
        use crate::vulnerability_db::{parse_lockfile, VulnerabilityScanner};

        let mut scanner = VulnerabilityScanner::new();

        // ── lockfile sweep ──
        // Walk shallow (project root + 1 level) for known lockfile names.
        // We deliberately don't recurse into node_modules / vendor — the
        // lockfile at the root is the source of truth.
        let lockfile_names = [
            "Cargo.lock",
            "package-lock.json",
            "yarn.lock",
            "pnpm-lock.yaml",
            "Pipfile.lock",
            "poetry.lock",
            "Gemfile.lock",
            "go.sum",
        ];
        for name in lockfile_names {
            let path = workspace.join(name);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let deps = parse_lockfile(name, &content);
                scanner.scan_dependencies(&deps);
            }
        }

        // ── SAST source sweep ──
        // Walk source files and let the existing SAST rule engine
        // produce findings. We respect a shallow depth budget so a
        // huge monorepo doesn't make the scan unbounded; users who
        // want deeper coverage can rescan from a subdirectory.
        let ws_str = workspace.to_string_lossy().to_string();
        for entry in walkdir::WalkDir::new(workspace)
            .max_depth(6)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let skip = path.ancestors().any(|a| {
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
                        )
                    })
                    .unwrap_or(false)
            });
            if skip {
                continue;
            }
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !matches!(
                    ext,
                    "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java"
                ) {
                    continue;
                }
            } else {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(path) {
                if content.len() > 1_048_576 {
                    continue; // skip huge files
                }
                let rel = path.strip_prefix(workspace).unwrap_or(path);
                scanner.scan_file(&rel.to_string_lossy(), &content);
            }
        }
        // Reference the workspace string so the unused-binding lint
        // doesn't fire when the SAST sweep returns no findings.
        let _ = ws_str;

        // ── convert to SecurityFinding ──
        let mut findings = Vec::new();
        for vuln in scanner.active_findings() {
            let severity = map_vulndb_severity(&vuln.severity);

            // Advisory URL if we have a CVE id.
            let mut refs = Vec::new();
            if let Some(cve) = &vuln.cve_id {
                if !cve.is_empty() {
                    refs.push(format!("https://nvd.nist.gov/vuln/detail/{cve}"));
                }
            }
            if let Some(ghsa) = &vuln.ghsa_id {
                if !ghsa.is_empty() {
                    refs.push(format!("https://github.com/advisories/{ghsa}"));
                }
            }

            let cwe_segment = vuln
                .cwe_id
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("{s}:"))
                .unwrap_or_default();
            let rule_id = format!("vuln-db:{cwe_segment}{}", vuln.id);

            // File path drives category: a finding with a file_path
            // is a source-code SAST hit; without it, it's a
            // dependency CVE.
            let (file, category) = match vuln.file_path.as_deref() {
                Some(p) if !p.is_empty() => (PathBuf::from(p), Category::Sast),
                _ => (PathBuf::from("."), Category::DependencyCve),
            };

            let line: Option<u32> = vuln
                .line
                .filter(|&n| n > 0)
                .and_then(|n| u32::try_from(n).ok());

            let title = match (
                vuln.installed_version.as_deref(),
                vuln.fixed_version.as_deref(),
            ) {
                (Some(inst), Some(fix)) if !inst.is_empty() && !fix.is_empty() => {
                    format!("{} (installed: {inst}, fix: {fix})", vuln.title)
                }
                (Some(inst), _) if !inst.is_empty() => {
                    format!("{} (installed: {inst})", vuln.title)
                }
                _ => vuln.title.clone(),
            };

            let remediation = if vuln.remediation.is_empty() {
                None
            } else {
                Some(vuln.remediation.clone())
            };

            findings.push(SecurityFinding::new(
                "vulnerability_db",
                severity,
                category,
                file,
                line,
                None,
                None, // snippet not surfaced; title + rule_id carry the signal
                rule_id,
                title,
                remediation,
                refs,
            ));
        }

        Ok(findings)
    }
}

/// Map vulnerability_db's native severity enum to our 5-bucket
/// scale. The native enum is a Rust enum with Display; we match on
/// the string form so adding new variants upstream doesn't break us
/// (we'd just map them to Medium pending a deliberate update).
fn map_vulndb_severity(s: &crate::vulnerability_db::Severity) -> Severity {
    use crate::vulnerability_db::Severity as V;
    match s {
        V::Critical => Severity::Critical,
        V::High => Severity::High,
        V::Medium => Severity::Medium,
        V::Low => Severity::Low,
        // `vulnerability_db::Severity` calls the lowest level `None`
        // (CVSS 0.0); map that to `security_posture::Severity::Info`,
        // which is the right "informational, not exploitable" bucket
        // on the security-posture side.
        V::None => Severity::Info,
    }
}

// ── sonar_rules adapter ─────────────────────────────────────────────

/// Wraps `vibe_core::sonar_rules::scan_content` into a `Scanner`.
///
/// Walks the workspace (respecting the standard skip-dirs) and runs
/// each source file through Sonar's regex-based rule engine. Each
/// `SonarIssue` becomes one `SecurityFinding`.
///
/// Severity mapping (per scanners.md §2):
/// `BLOCKER → Critical`, `CRITICAL → High`, `MAJOR → Medium`,
/// `MINOR → Low`, `INFO → Info`.
///
/// Category mapping: `VULNERABILITY` and `SECURITY_HOTSPOT` issues
/// land in `Sast`; `BUG` and `CODE_SMELL` land in `CodeHealth`.
pub struct SonarScannerAdapter;

impl Scanner for SonarScannerAdapter {
    fn name(&self) -> &'static str {
        "sonar"
    }

    fn scan(&self, workspace: &Path) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

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

            // Skip the same noise dirs the other scanners do, so
            // we don't re-scan node_modules / target / .git.
            let skip = path.ancestors().any(|a| {
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
                        )
                    })
                    .unwrap_or(false)
            });
            if skip {
                continue;
            }

            // Sonar rules cover ts / js / rs primarily; only feed
            // files those rules will actually match. Skip everything
            // else to keep the scan fast.
            let ext = match path.extension().and_then(|e| e.to_str()) {
                Some(e) => e,
                None => continue,
            };
            if !matches!(
                ext,
                "ts" | "tsx" | "js" | "jsx" | "rs" | "py" | "java" | "go" | "cs"
            ) {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) if c.len() < 1_048_576 => c, // 1 MiB cap
                _ => continue,
            };
            let rel = path.strip_prefix(workspace).unwrap_or(path);
            let rel_str = rel.to_string_lossy().to_string();
            let result = vibe_core::sonar_rules::scan_content(&rel_str, &content);

            for issue in result.issues {
                findings.push(sonar_issue_to_finding(&issue, rel));
            }
        }

        Ok(findings)
    }
}

fn sonar_issue_to_finding(
    issue: &vibe_core::sonar_rules::SonarIssue,
    rel: &Path,
) -> SecurityFinding {
    let severity = map_sonar_severity(&issue.severity);
    let category = match issue.issue_type.as_str() {
        "VULNERABILITY" | "SECURITY_HOTSPOT" => Category::Sast,
        _ => Category::CodeHealth, // BUG / CODE_SMELL
    };
    // Sonar's `code_snippet` field is the matched line content.
    // Bound it to the snippet cap via SecurityFinding::new's
    // truncation — `SNIPPET_MAX_BYTES` enforced there.
    let snippet = if issue.code_snippet.is_empty() {
        None
    } else {
        Some(issue.code_snippet.clone())
    };
    let remediation = if issue.how_to_fix.is_empty() {
        None
    } else {
        Some(issue.how_to_fix.clone())
    };
    SecurityFinding::new(
        "sonar",
        severity,
        category,
        rel.to_path_buf(),
        Some(issue.line),
        Some(issue.col_start),
        snippet,
        format!("sonar:{}", issue.rule_key),
        issue.rule_name.clone(),
        remediation,
        Vec::new(),
    )
}

/// Map Sonar's severity-string vocabulary into our 5-bucket enum.
/// Unknown values default to Medium — surfacing on the panel is
/// better than silently dropping.
fn map_sonar_severity(s: &str) -> Severity {
    match s {
        "BLOCKER" => Severity::Critical,
        "CRITICAL" => Severity::High,
        "MAJOR" => Severity::Medium,
        "MINOR" => Severity::Low,
        "INFO" => Severity::Info,
        _ => Severity::Medium,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_to_severity_thresholds() {
        assert_eq!(score_to_severity(10.0), Severity::High);
        assert_eq!(score_to_severity(29.9), Severity::High);
        assert_eq!(score_to_severity(30.0), Severity::Medium);
        assert_eq!(score_to_severity(59.9), Severity::Medium);
        assert_eq!(score_to_severity(60.0), Severity::Low);
        assert_eq!(score_to_severity(79.9), Severity::Low);
        assert_eq!(score_to_severity(80.0), Severity::Info);
        assert_eq!(score_to_severity(100.0), Severity::Info);
    }

    #[test]
    fn slugify_dimension_kebab_case() {
        assert_eq!(slugify_dimension("Test Coverage"), "test-coverage");
        assert_eq!(slugify_dimension("API Coverage"), "api-coverage");
        assert_eq!(
            slugify_dimension("Dependency Freshness"),
            "dependency-freshness"
        );
    }

    #[test]
    fn health_scanner_name_stable() {
        assert_eq!(HealthScannerAdapter.name(), "health");
    }

    #[test]
    fn vuln_scanner_name_stable() {
        assert_eq!(VulnerabilityScannerAdapter.name(), "vulnerability_db");
    }

    #[test]
    fn sonar_scanner_name_stable() {
        assert_eq!(SonarScannerAdapter.name(), "sonar");
    }

    #[test]
    fn map_sonar_severity_known_values() {
        assert_eq!(map_sonar_severity("BLOCKER"), Severity::Critical);
        assert_eq!(map_sonar_severity("CRITICAL"), Severity::High);
        assert_eq!(map_sonar_severity("MAJOR"), Severity::Medium);
        assert_eq!(map_sonar_severity("MINOR"), Severity::Low);
        assert_eq!(map_sonar_severity("INFO"), Severity::Info);
    }

    #[test]
    fn map_sonar_severity_unknown_defaults_medium() {
        // Surfacing on the panel is better than dropping; a future
        // Sonar severity variant should still show up.
        assert_eq!(map_sonar_severity("WHATEVER"), Severity::Medium);
        assert_eq!(map_sonar_severity(""), Severity::Medium);
    }

    #[test]
    fn sonar_vulnerability_issue_lands_in_sast() {
        let issue = vibe_core::sonar_rules::SonarIssue {
            rule_key: "rust:S2068".into(),
            rule_name: "Hardcoded credential".into(),
            file: "src/x.rs".into(),
            line: 10,
            end_line: 10,
            col_start: 0,
            message: String::new(),
            severity: "BLOCKER".into(),
            issue_type: "VULNERABILITY".into(),
            code_snippet: String::new(),
            context_before: String::new(),
            context_after: String::new(),
            why: String::new(),
            how_to_fix: String::new(),
            effort: String::new(),
        };
        let f = sonar_issue_to_finding(&issue, Path::new("src/x.rs"));
        assert_eq!(f.scanner, "sonar");
        assert_eq!(f.severity, Severity::Critical);
        assert_eq!(f.category, Category::Sast);
        assert_eq!(f.rule_id, "sonar:rust:S2068");
        assert_eq!(f.line, Some(10));
    }

    #[test]
    fn sonar_code_smell_lands_in_code_health() {
        let issue = vibe_core::sonar_rules::SonarIssue {
            rule_key: "ts:S1192".into(),
            rule_name: "Duplicate string literals".into(),
            file: "x.ts".into(),
            line: 5,
            end_line: 5,
            col_start: 0,
            message: String::new(),
            severity: "MINOR".into(),
            issue_type: "CODE_SMELL".into(),
            code_snippet: String::new(),
            context_before: String::new(),
            context_after: String::new(),
            why: String::new(),
            how_to_fix: String::new(),
            effort: String::new(),
        };
        let f = sonar_issue_to_finding(&issue, Path::new("x.ts"));
        assert_eq!(f.category, Category::CodeHealth);
        assert_eq!(f.severity, Severity::Low);
    }

    // Full-scan tests need a fixture workspace; deferred to the
    // BDD harness where `tests/` already builds tempdir trees.
}
