#![allow(dead_code)]
//! Dependency update advisor — analyzes SemVer constraints, identifies outdated
//! dependencies, and assesses update safety (breaking vs non-breaking).
//! Matches Cody 6.0's dependency intelligence feature.
//!
//! Supports Cargo.toml and package.json style dependency specs.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A semantic version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<String>,
}

impl SemVer {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch, pre: None }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim_start_matches(['v', '^', '~', '=', '>', '<', ' ']);
        let parts: Vec<&str> = s.splitn(3, '.').collect();
        if parts.len() < 3 { return None; }
        let patch_pre: Vec<&str> = parts[2].splitn(2, '-').collect();
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: patch_pre[0].parse().ok()?,
            pre: patch_pre.get(1).map(|s| s.to_string()),
        })
    }

    pub fn is_pre_release(&self) -> bool { self.pre.is_some() }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(pre) = &self.pre {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

/// How risky an update is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateRisk {
    /// Patch update — backward compatible bug fixes.
    Patch,
    /// Minor update — new features, backward compatible.
    Minor,
    /// Major update — may contain breaking changes.
    Major,
    /// Pre-release or unstable.
    Unstable,
}

impl std::fmt::Display for UpdateRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateRisk::Patch => write!(f, "patch (safe)"),
            UpdateRisk::Minor => write!(f, "minor (safe)"),
            UpdateRisk::Major => write!(f, "major (breaking possible)"),
            UpdateRisk::Unstable => write!(f, "unstable (pre-release)"),
        }
    }
}

/// A dependency update recommendation.
#[derive(Debug, Clone)]
pub struct UpdateRecommendation {
    pub package: String,
    pub current: SemVer,
    pub latest: SemVer,
    pub risk: UpdateRisk,
    pub notes: Vec<String>,
}

impl UpdateRecommendation {
    pub fn can_auto_update(&self) -> bool {
        matches!(self.risk, UpdateRisk::Patch | UpdateRisk::Minor)
    }
}

// ---------------------------------------------------------------------------
// Advisor
// ---------------------------------------------------------------------------

/// Registry of current (installed) versions vs latest available.
pub struct DepUpdateAdvisor {
    /// package_name → (current, latest)
    registry: HashMap<String, (SemVer, SemVer)>,
    /// package_name → known breaking changes notes
    known_breaking: HashMap<String, Vec<String>>,
}

impl Default for DepUpdateAdvisor {
    fn default() -> Self { Self::new() }
}

impl DepUpdateAdvisor {
    pub fn new() -> Self {
        Self { registry: HashMap::new(), known_breaking: HashMap::new() }
    }

    pub fn add_package(&mut self, name: impl Into<String>, current: SemVer, latest: SemVer) {
        self.registry.insert(name.into(), (current, latest));
    }

    pub fn add_known_breaking(&mut self, package: impl Into<String>, notes: Vec<String>) {
        self.known_breaking.insert(package.into(), notes);
    }

    /// Compute risk level between `current` and `latest`.
    pub fn risk_level(current: &SemVer, latest: &SemVer) -> UpdateRisk {
        if latest.is_pre_release() { return UpdateRisk::Unstable; }
        if latest.major > current.major { return UpdateRisk::Major; }
        if latest.minor > current.minor { return UpdateRisk::Minor; }
        UpdateRisk::Patch
    }

    /// Generate recommendations for all tracked packages.
    pub fn analyze(&self) -> Vec<UpdateRecommendation> {
        let mut recommendations: Vec<UpdateRecommendation> = Vec::new();

        for (package, (current, latest)) in &self.registry {
            if latest <= current { continue; } // already up to date

            let risk = Self::risk_level(current, latest);
            let mut notes = Vec::new();

            if matches!(risk, UpdateRisk::Major) {
                notes.push(format!("Major version bump: {} → {}", current, latest));
            }
            if let Some(breaking) = self.known_breaking.get(package) {
                notes.extend(breaking.iter().cloned());
            }
            if latest.is_pre_release() {
                notes.push("Pre-release version — not recommended for production".into());
            }

            recommendations.push(UpdateRecommendation {
                package: package.clone(),
                current: current.clone(),
                latest: latest.clone(),
                risk,
                notes,
            });
        }

        // Sort: major first (most attention needed), then by package name
        recommendations.sort_by(|a, b| {
            let risk_ord = |r: &UpdateRisk| match r {
                UpdateRisk::Major => 0,
                UpdateRisk::Unstable => 1,
                UpdateRisk::Minor => 2,
                UpdateRisk::Patch => 3,
            };
            risk_ord(&a.risk).cmp(&risk_ord(&b.risk)).then(a.package.cmp(&b.package))
        });

        recommendations
    }

    /// Return only packages that are safe to auto-update.
    pub fn safe_updates(&self) -> Vec<UpdateRecommendation> {
        self.analyze().into_iter().filter(|r| r.can_auto_update()).collect()
    }

    /// Render a human-readable report.
    pub fn report(&self) -> String {
        let recs = self.analyze();
        if recs.is_empty() {
            return "All dependencies are up to date.".to_string();
        }

        let mut out = String::from("# Dependency Update Report\n\n");
        for rec in &recs {
            out.push_str(&format!(
                "- **{}**: {} → {} [{}]\n",
                rec.package, rec.current, rec.latest, rec.risk
            ));
            for note in &rec.notes {
                out.push_str(&format!("  - {}\n", note));
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> SemVer { SemVer::parse(s).unwrap() }

    #[test]
    fn test_semver_parse() {
        let sv = v("1.2.3");
        assert_eq!(sv.major, 1);
        assert_eq!(sv.minor, 2);
        assert_eq!(sv.patch, 3);
    }

    #[test]
    fn test_semver_parse_with_v_prefix() {
        let sv = v("v2.0.0");
        assert_eq!(sv.major, 2);
    }

    #[test]
    fn test_semver_parse_pre_release() {
        let sv = v("1.0.0-alpha.1");
        assert!(sv.is_pre_release());
    }

    #[test]
    fn test_semver_ordering() {
        assert!(v("2.0.0") > v("1.9.9"));
        assert!(v("1.1.0") > v("1.0.9"));
        assert!(v("1.0.1") > v("1.0.0"));
    }

    #[test]
    fn test_risk_patch() {
        assert_eq!(DepUpdateAdvisor::risk_level(&v("1.0.0"), &v("1.0.1")), UpdateRisk::Patch);
    }

    #[test]
    fn test_risk_minor() {
        assert_eq!(DepUpdateAdvisor::risk_level(&v("1.0.0"), &v("1.1.0")), UpdateRisk::Minor);
    }

    #[test]
    fn test_risk_major() {
        assert_eq!(DepUpdateAdvisor::risk_level(&v("1.0.0"), &v("2.0.0")), UpdateRisk::Major);
    }

    #[test]
    fn test_risk_unstable() {
        let latest = SemVer { major: 2, minor: 0, patch: 0, pre: Some("beta.1".into()) };
        assert_eq!(DepUpdateAdvisor::risk_level(&v("1.0.0"), &latest), UpdateRisk::Unstable);
    }

    #[test]
    fn test_analyze_skips_up_to_date() {
        let mut advisor = DepUpdateAdvisor::new();
        advisor.add_package("serde", v("1.0.0"), v("1.0.0")); // same version
        assert!(advisor.analyze().is_empty());
    }

    #[test]
    fn test_analyze_returns_outdated() {
        let mut advisor = DepUpdateAdvisor::new();
        advisor.add_package("tokio", v("1.0.0"), v("1.2.0"));
        let recs = advisor.analyze();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].package, "tokio");
        assert_eq!(recs[0].risk, UpdateRisk::Minor);
    }

    #[test]
    fn test_major_sorted_first() {
        let mut advisor = DepUpdateAdvisor::new();
        advisor.add_package("serde", v("1.0.0"), v("1.0.1"));
        advisor.add_package("tokio", v("1.0.0"), v("2.0.0"));
        let recs = advisor.analyze();
        assert_eq!(recs[0].risk, UpdateRisk::Major);
    }

    #[test]
    fn test_known_breaking_notes() {
        let mut advisor = DepUpdateAdvisor::new();
        advisor.add_package("tokio", v("1.0.0"), v("2.0.0"));
        advisor.add_known_breaking("tokio", vec!["Runtime::new() API changed".into()]);
        let recs = advisor.analyze();
        assert!(recs[0].notes.iter().any(|n| n.contains("Runtime")));
    }

    #[test]
    fn test_safe_updates_excludes_major() {
        let mut advisor = DepUpdateAdvisor::new();
        advisor.add_package("safe-crate", v("1.0.0"), v("1.1.0"));
        advisor.add_package("risky-crate", v("1.0.0"), v("2.0.0"));
        let safe = advisor.safe_updates();
        assert_eq!(safe.len(), 1);
        assert_eq!(safe[0].package, "safe-crate");
    }

    #[test]
    fn test_report_all_up_to_date() {
        let advisor = DepUpdateAdvisor::new();
        assert_eq!(advisor.report(), "All dependencies are up to date.");
    }

    #[test]
    fn test_semver_display() {
        assert_eq!(v("1.2.3").to_string(), "1.2.3");
    }
}
