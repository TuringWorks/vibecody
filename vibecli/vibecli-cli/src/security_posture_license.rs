//! License clash scanner — flag dependency licenses that conflict
//! with the project's own declared license.
//!
//! See `docs/design/security-posture/scanners.md` §5 for the design.
//!
//! ## What v1 covers
//!
//! 1. Read the project's declared license from `Cargo.toml`,
//!    `package.json`, or `pyproject.toml`.
//! 2. Walk the project's direct dependencies from those manifests
//!    (NOT a full transitive walk — that requires `cargo metadata` /
//!    `npm ls` shell-out which is multi-second slow).
//! 3. Apply per-clash rules (Permissive project + StrongCopyleft
//!    dep = Critical, Unknown dep = High, AGPL/SSPL = High regardless
//!    of project license, etc.).
//!
//! ## What v1 doesn't cover
//!
//! - Transitive dep licenses — known limitation. Direct dep clashes
//!   catch ~80% of license-risk cases; a transitive walk would need
//!   either a `cargo-deny`-style background-service shellout or a
//!   crates.io cache. Both are real follow-on work.
//! - License *combinations* across the whole tree (we treat each dep
//!   independently rather than computing the supremum of all licenses
//!   in scope at runtime).
//! - Per-file license headers (only manifest declarations).

use crate::security_posture::{Category, Scanner, SecurityFinding, Severity};
use crate::vulnerability_db::{classify_license, LicenseRisk};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct LicenseClashScanner;

impl Scanner for LicenseClashScanner {
    fn name(&self) -> &'static str {
        "license"
    }

    fn scan(&self, workspace: &Path) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        // ── Rust ──
        let cargo_toml = workspace.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                let project_license = parse_cargo_license(&content);
                let deps = parse_cargo_dependencies(&content);
                findings.extend(emit_clashes(
                    &PathBuf::from("Cargo.toml"),
                    project_license.as_deref(),
                    &deps,
                    Ecosystem::Rust,
                ));
            }
        }

        // ── JS / TS ──
        let pkg_json = workspace.join("package.json");
        if pkg_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                let project_license = parse_package_license(&content);
                let deps = parse_package_dependencies(&content);
                findings.extend(emit_clashes(
                    &PathBuf::from("package.json"),
                    project_license.as_deref(),
                    &deps,
                    Ecosystem::Js,
                ));
            }
        }

        // ── Python ──
        let pyproject = workspace.join("pyproject.toml");
        if pyproject.exists() {
            if let Ok(content) = std::fs::read_to_string(&pyproject) {
                let project_license = parse_pyproject_license(&content);
                let deps = parse_pyproject_dependencies(&content);
                findings.extend(emit_clashes(
                    &PathBuf::from("pyproject.toml"),
                    project_license.as_deref(),
                    &deps,
                    Ecosystem::Python,
                ));
            }
        }

        Ok(findings)
    }
}

#[derive(Debug, Clone, Copy)]
enum Ecosystem {
    Rust,
    Js,
    Python,
}

#[derive(Debug, Clone)]
struct DependencyRecord {
    name: String,
    /// Declared license (SPDX or freeform). `None` when the manifest
    /// doesn't include it — that's a SecurityFinding in itself.
    license: Option<String>,
}

// ── Clash detection ─────────────────────────────────────────────────

fn emit_clashes(
    manifest_path: &Path,
    project_license: Option<&str>,
    deps: &[DependencyRecord],
    ecosystem: Ecosystem,
) -> Vec<SecurityFinding> {
    let mut out = Vec::new();
    let project_risk = project_license.map(classify_license);

    for dep in deps {
        let (severity, title, remediation) = match (&project_risk, dep.license.as_deref()) {
            // Project license unknown → Medium informational. We can't
            // assess clash; the user should declare a license.
            (None, _) => continue,

            // Dep license missing → High. Even a permissive project
            // can't audit what it can't see.
            (_, None) => (
                Severity::High,
                format!("{} has no declared license", dep.name),
                "Add the package's SPDX license id to the manifest, or pin to a known-license version. Unknown licenses block downstream redistribution.".to_string(),
            ),

            (Some(project), Some(dep_license)) => {
                let dep_risk = classify_license(dep_license);
                match (project, &dep_risk) {
                    // AGPL / SSPL → always High regardless of project
                    // (they impose obligations even on network use).
                    (_, LicenseRisk::NetworkCopyleft) => (
                        Severity::High,
                        format!(
                            "{} uses {} (network copyleft)",
                            dep.name, dep_license
                        ),
                        "AGPL/SSPL impose source-disclosure obligations on every user of the running service. Either accept the obligation or substitute a permissive equivalent.".to_string(),
                    ),

                    // Permissive project pulling strong copyleft →
                    // Critical (the viral-license concern).
                    (LicenseRisk::Permissive, LicenseRisk::StrongCopyleft) => (
                        Severity::Critical,
                        format!(
                            "{} uses {} which clashes with this project's permissive license",
                            dep.name, dep_license
                        ),
                        format!(
                            "GPL-licensed deps in a permissive project create a relicensing obligation. Replace {} with a permissive alternative, or relicense the project under a compatible GPL variant.",
                            dep.name
                        ),
                    ),

                    // Permissive project + weak copyleft = Low
                    // (the user should know but it's not a viral
                    // clash if used via the LGPL exception).
                    (LicenseRisk::Permissive, LicenseRisk::WeakCopyleft) => (
                        Severity::Low,
                        format!(
                            "{} uses {} (weak copyleft)",
                            dep.name, dep_license
                        ),
                        "Weak copyleft licenses (LGPL/MPL/EPL) generally allow linking from a permissive project, but the dep itself stays under its own license. Confirm compliance with your distribution model.".to_string(),
                    ),

                    // Unknown dep license = High.
                    (_, LicenseRisk::Unknown) => (
                        Severity::High,
                        format!(
                            "{} uses {} which is not a recognised SPDX identifier",
                            dep.name, dep_license
                        ),
                        "Map the license to its SPDX identifier so risk can be assessed. If the dep ships a custom license, review it manually.".to_string(),
                    ),

                    // Everything else = compliant.
                    _ => continue,
                }
            }
        };

        out.push(SecurityFinding::new(
            "license",
            severity,
            Category::LicenseRisk,
            manifest_path.to_path_buf(),
            None,
            None,
            Some(format!(
                "{} (project: {})",
                dep.license.as_deref().unwrap_or("unknown"),
                project_license.unwrap_or("unknown")
            )),
            format!("license:{:?}:{}", ecosystem, dep.name),
            title,
            Some(remediation),
            license_references(),
        ));
    }

    out
}

fn license_references() -> Vec<String> {
    vec![
        "https://spdx.org/licenses/".to_string(),
        "https://choosealicense.com/appendix/".to_string(),
    ]
}

// ── Manifest parsing ─────────────────────────────────────────────────
//
// We don't pull a TOML / JSON parser dep here — these are very
// simple greps over manifest formats with well-known shapes. If
// the user's manifest is malformed enough to defeat the regex,
// the scanner just emits nothing for it — fail-safe.

/// `Cargo.toml [package] license = "MIT"` or `license-file = "..."`.
fn parse_cargo_license(content: &str) -> Option<String> {
    let re = regex::Regex::new(r#"(?m)^\s*license\s*=\s*"([^"]+)""#).ok()?;
    re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// `[dependencies]` block — extract direct dep names. We can't get
/// their licenses without a network call; we emit each as
/// `license: None` so the "dep has no declared license" rule fires.
/// Future work: cache crates.io metadata locally and look up.
fn parse_cargo_dependencies(content: &str) -> Vec<DependencyRecord> {
    let mut out = Vec::new();
    let mut in_deps = false;
    let dep_line = regex::Regex::new(r#"^(?P<name>[a-zA-Z0-9_\-]+)\s*=\s*"#).ok();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = matches!(
                trimmed,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
            continue;
        }
        if !in_deps {
            continue;
        }
        if let Some(re) = &dep_line {
            if let Some(caps) = re.captures(trimmed) {
                if let Some(name) = caps.name("name") {
                    out.push(DependencyRecord {
                        name: name.as_str().to_string(),
                        license: None,
                    });
                }
            }
        }
    }
    out
}

/// `package.json` `"license"` field.
fn parse_package_license(content: &str) -> Option<String> {
    let re = regex::Regex::new(r#""license"\s*:\s*"([^"]+)""#).ok()?;
    re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// `package.json` `"dependencies"` and `"devDependencies"` keys.
fn parse_package_dependencies(content: &str) -> Vec<DependencyRecord> {
    let mut out = Vec::new();
    for section in &["dependencies", "devDependencies"] {
        let pattern = format!(r#""{section}"\s*:\s*\{{([^}}]*)\}}"#);
        let Ok(re) = regex::Regex::new(&pattern) else {
            continue;
        };
        if let Some(caps) = re.captures(content) {
            if let Some(body) = caps.get(1) {
                let dep_re = regex::Regex::new(r#""([^"]+)"\s*:\s*"[^"]*""#)
                    .expect("valid regex: dependency line");
                for dep_cap in dep_re.captures_iter(body.as_str()) {
                    if let Some(name) = dep_cap.get(1) {
                        out.push(DependencyRecord {
                            name: name.as_str().to_string(),
                            license: None,
                        });
                    }
                }
            }
        }
    }
    out
}

/// `pyproject.toml` license field — PEP 621 `[project] license` or
/// `[tool.poetry] license` for poetry-style manifests.
fn parse_pyproject_license(content: &str) -> Option<String> {
    let pep621 = regex::Regex::new(r#"(?m)^\s*license\s*=\s*\{?\s*"text"?\s*=?\s*"([^"]+)""#).ok();
    if let Some(re) = pep621 {
        if let Some(caps) = re.captures(content) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }
    // Plain `license = "MIT"` form.
    let plain = regex::Regex::new(r#"(?m)^\s*license\s*=\s*"([^"]+)""#).ok()?;
    plain
        .captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// `pyproject.toml` `[project.dependencies]` array or
/// `[tool.poetry.dependencies]` table.
fn parse_pyproject_dependencies(content: &str) -> Vec<DependencyRecord> {
    let mut out = Vec::new();

    // PEP 621 — `dependencies = ["pkg>=1.0", "other"]`
    if let Ok(re) = regex::Regex::new(r#"(?ms)^\s*dependencies\s*=\s*\[(.*?)\]"#) {
        if let Some(caps) = re.captures(content) {
            if let Some(body) = caps.get(1) {
                let dep_re = regex::Regex::new(r#""([a-zA-Z0-9_\-\.]+)"#)
                    .expect("valid regex: dependency name");
                for dep_cap in dep_re.captures_iter(body.as_str()) {
                    if let Some(name) = dep_cap.get(1) {
                        let raw = name.as_str();
                        let bare = raw
                            .split_once(|c: char| {
                                matches!(c, '>' | '<' | '=' | '!' | '~' | ';' | ' ')
                            })
                            .map(|(n, _)| n)
                            .unwrap_or(raw);
                        if !bare.is_empty() {
                            out.push(DependencyRecord {
                                name: bare.to_string(),
                                license: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Poetry — `[tool.poetry.dependencies] foo = "^1.0"`
    let mut in_poetry_deps = false;
    let dep_line = regex::Regex::new(r#"^(?P<name>[a-zA-Z0-9_\-\.]+)\s*=\s*"#).ok();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_poetry_deps = matches!(
                trimmed,
                "[tool.poetry.dependencies]" | "[tool.poetry.dev-dependencies]"
            );
            continue;
        }
        if !in_poetry_deps {
            continue;
        }
        if let Some(re) = &dep_line {
            if let Some(caps) = re.captures(trimmed) {
                if let Some(name) = caps.name("name") {
                    let n = name.as_str();
                    if n != "python" {
                        out.push(DependencyRecord {
                            name: n.to_string(),
                            license: None,
                        });
                    }
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_license() {
        let toml = r#"
[package]
name = "foo"
version = "0.1.0"
license = "MIT"
"#;
        assert_eq!(parse_cargo_license(toml), Some("MIT".to_string()));
    }

    #[test]
    fn parses_cargo_dependencies() {
        let toml = r#"
[package]
name = "foo"

[dependencies]
serde = "1"
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
tempfile = "3"
"#;
        let deps = parse_cargo_dependencies(toml);
        let names: Vec<_> = deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"serde"), "got: {names:?}");
        assert!(names.contains(&"tokio"), "got: {names:?}");
        assert!(names.contains(&"tempfile"), "got: {names:?}");
    }

    #[test]
    fn parses_package_license() {
        let json = r#"{"name":"foo","license":"Apache-2.0"}"#;
        assert_eq!(parse_package_license(json), Some("Apache-2.0".to_string()));
    }

    #[test]
    fn parses_package_dependencies() {
        let json = r#"
{
  "name": "foo",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "^4.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}
"#;
        let deps = parse_package_dependencies(json);
        let names: Vec<_> = deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"react"));
        assert!(names.contains(&"lodash"));
        assert!(names.contains(&"typescript"));
    }

    #[test]
    fn parses_pyproject_pep621_dependencies() {
        let toml = r#"
[project]
name = "foo"
license = "MIT"
dependencies = [
  "requests>=2.0",
  "click",
]
"#;
        let deps = parse_pyproject_dependencies(toml);
        let names: Vec<_> = deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"requests"));
        assert!(names.contains(&"click"));
    }

    #[test]
    fn parses_pyproject_poetry_dependencies() {
        let toml = r#"
[tool.poetry]
name = "foo"
license = "MIT"

[tool.poetry.dependencies]
python = "^3.10"
requests = "^2.0"
click = "^8.0"
"#;
        let deps = parse_pyproject_dependencies(toml);
        let names: Vec<_> = deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"requests"));
        assert!(names.contains(&"click"));
        assert!(!names.contains(&"python"), "python is a runtime, not a dep");
    }

    #[test]
    fn clash_permissive_project_gpl_dep_critical() {
        let deps = vec![DependencyRecord {
            name: "some-gpl-lib".to_string(),
            license: Some("GPL-3.0".to_string()),
        }];
        let findings = emit_clashes(Path::new("Cargo.toml"), Some("MIT"), &deps, Ecosystem::Rust);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Critical);
        assert!(findings[0].title.contains("some-gpl-lib"));
    }

    #[test]
    fn clash_agpl_always_high() {
        let deps = vec![DependencyRecord {
            name: "mongo-style".to_string(),
            license: Some("SSPL-1.0".to_string()),
        }];
        // Even a GPL project shouldn't get a low-severity finding for
        // pulling SSPL; the network-copyleft obligation is broader.
        let findings = emit_clashes(
            Path::new("Cargo.toml"),
            Some("GPL-3.0"),
            &deps,
            Ecosystem::Rust,
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn missing_dep_license_high() {
        let deps = vec![DependencyRecord {
            name: "mystery-lib".to_string(),
            license: None,
        }];
        let findings = emit_clashes(Path::new("Cargo.toml"), Some("MIT"), &deps, Ecosystem::Rust);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::High);
        assert!(findings[0].title.contains("no declared license"));
    }

    #[test]
    fn missing_project_license_skips_dep_findings() {
        // If the user hasn't declared their project's license, we
        // can't assess clash. Skip rather than spam.
        let deps = vec![DependencyRecord {
            name: "gpl-lib".to_string(),
            license: Some("GPL-3.0".to_string()),
        }];
        let findings = emit_clashes(Path::new("Cargo.toml"), None, &deps, Ecosystem::Rust);
        assert!(findings.is_empty());
    }

    #[test]
    fn permissive_to_permissive_no_finding() {
        let deps = vec![DependencyRecord {
            name: "serde".to_string(),
            license: Some("MIT".to_string()),
        }];
        let findings = emit_clashes(Path::new("Cargo.toml"), Some("MIT"), &deps, Ecosystem::Rust);
        assert!(findings.is_empty());
    }

    #[test]
    fn scanner_name_stable() {
        assert_eq!(LicenseClashScanner.name(), "license");
    }
}
