//! Agentic package manager — dependency analysis, conflict resolution,
//! security advisory checks, license compliance, and alternative comparison.
//!
//! Gap 22 — Supports Npm, Cargo, Pip, Go, Maven, and Gradle ecosystems.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported package managers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageManager {
    Npm,
    Cargo,
    Pip,
    Go,
    Maven,
    Gradle,
}

impl PackageManager {
    pub fn lockfile_name(&self) -> &str {
        match self {
            Self::Npm => "package-lock.json",
            Self::Cargo => "Cargo.lock",
            Self::Pip => "requirements.txt",
            Self::Go => "go.sum",
            Self::Maven => "pom.xml",
            Self::Gradle => "gradle.lockfile",
        }
    }

    pub fn manifest_name(&self) -> &str {
        match self {
            Self::Npm => "package.json",
            Self::Cargo => "Cargo.toml",
            Self::Pip => "setup.py",
            Self::Go => "go.mod",
            Self::Maven => "pom.xml",
            Self::Gradle => "build.gradle",
        }
    }
}

/// A single dependency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub manager: PackageManager,
    pub dev_only: bool,
}

impl Dependency {
    pub fn new(name: &str, version: &str, manager: PackageManager) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            manager,
            dev_only: false,
        }
    }

    pub fn dev(mut self) -> Self {
        self.dev_only = true;
        self
    }
}

/// A detected conflict between two requirements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepConflict {
    pub package: String,
    pub required_a: String,
    pub required_b: String,
    pub resolution: Option<String>,
}

/// Strategy for resolving version conflicts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    Newest,
    Oldest,
    Compatible,
    Fork,
}

/// Severity of a security advisory.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// A security advisory for a package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityAdvisory {
    pub id: String,
    pub package: String,
    pub severity: Severity,
    pub fixed_version: Option<String>,
    pub description: String,
}

/// Comparison between package alternatives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageComparison {
    pub name: String,
    pub downloads: u64,
    pub maintenance_score: f64,
    pub security_score: f64,
    pub license: String,
    pub size_kb: u64,
}

/// License compliance policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicensePolicy {
    pub allowed: Vec<String>,
    pub blocked: Vec<String>,
}

impl Default for LicensePolicy {
    fn default() -> Self {
        Self {
            allowed: vec![
                "MIT".to_string(), "Apache-2.0".to_string(), "BSD-2-Clause".to_string(),
                "BSD-3-Clause".to_string(), "ISC".to_string(),
            ],
            blocked: vec![
                "GPL-3.0".to_string(), "AGPL-3.0".to_string(),
            ],
        }
    }
}

/// Configuration for the analyzer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepAnalyzerConfig {
    pub auto_fix: bool,
    pub max_depth: u32,
    pub license_policy: LicensePolicy,
}

impl Default for DepAnalyzerConfig {
    fn default() -> Self {
        Self {
            auto_fix: false,
            max_depth: 10,
            license_policy: LicensePolicy::default(),
        }
    }
}

/// Core dependency analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepAnalyzer {
    pub deps: Vec<Dependency>,
    pub conflicts: Vec<DepConflict>,
    pub advisories: Vec<SecurityAdvisory>,
    pub config: DepAnalyzerConfig,
    usage_counts: HashMap<String, u32>,
}

impl DepAnalyzer {
    pub fn new(config: DepAnalyzerConfig) -> Self {
        Self {
            deps: Vec::new(),
            conflicts: Vec::new(),
            advisories: Vec::new(),
            config,
            usage_counts: HashMap::new(),
        }
    }

    /// Add a dependency.
    pub fn add_dep(&mut self, dep: Dependency) {
        self.deps.push(dep);
    }

    /// Add a security advisory.
    pub fn add_advisory(&mut self, advisory: SecurityAdvisory) {
        self.advisories.push(advisory);
    }

    /// Mark a dependency as "used" (referenced in source).
    pub fn mark_used(&mut self, name: &str) {
        *self.usage_counts.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Analyze dependencies: group by manager, count stats.
    pub fn analyze(&self) -> HashMap<PackageManager, Vec<&Dependency>> {
        let mut groups: HashMap<PackageManager, Vec<&Dependency>> = HashMap::new();
        for dep in &self.deps {
            groups.entry(dep.manager.clone()).or_default().push(dep);
        }
        groups
    }

    /// Detect version conflicts among loaded deps.
    pub fn detect_conflicts(&mut self) -> &[DepConflict] {
        self.conflicts.clear();
        let mut seen: HashMap<String, Vec<&Dependency>> = HashMap::new();
        for dep in &self.deps {
            seen.entry(dep.name.clone()).or_default().push(dep);
        }
        for (name, versions) in &seen {
            if versions.len() > 1 {
                let unique_versions: Vec<&str> = {
                    let mut v: Vec<&str> = versions.iter().map(|d| d.version.as_str()).collect();
                    v.sort();
                    v.dedup();
                    v
                };
                if unique_versions.len() > 1 {
                    self.conflicts.push(DepConflict {
                        package: name.clone(),
                        required_a: unique_versions[0].to_string(),
                        required_b: unique_versions[1].to_string(),
                        resolution: None,
                    });
                }
            }
        }
        &self.conflicts
    }

    /// Resolve a conflict using the given strategy.
    pub fn resolve_conflict(&mut self, package: &str, strategy: &ResolutionStrategy) -> Result<String, String> {
        let conflict = self.conflicts.iter_mut()
            .find(|c| c.package == package)
            .ok_or_else(|| format!("No conflict for {}", package))?;

        let resolved = match strategy {
            ResolutionStrategy::Newest => {
                if conflict.required_a > conflict.required_b {
                    conflict.required_a.clone()
                } else {
                    conflict.required_b.clone()
                }
            }
            ResolutionStrategy::Oldest => {
                if conflict.required_a < conflict.required_b {
                    conflict.required_a.clone()
                } else {
                    conflict.required_b.clone()
                }
            }
            ResolutionStrategy::Compatible => {
                // Pick the higher minor
                conflict.required_b.clone()
            }
            ResolutionStrategy::Fork => {
                format!("{}-fork", package)
            }
        };
        conflict.resolution = Some(resolved.clone());
        Ok(resolved)
    }

    /// Check deps against known advisories.
    pub fn check_security(&self) -> Vec<(&Dependency, &SecurityAdvisory)> {
        let mut results = Vec::new();
        for dep in &self.deps {
            for adv in &self.advisories {
                if adv.package == dep.name {
                    results.push((dep, adv));
                }
            }
        }
        results
    }

    /// Compare alternative packages.
    pub fn compare_alternatives<'a>(&self, alternatives: &'a [PackageComparison]) -> Vec<&'a PackageComparison> {
        let mut sorted: Vec<&PackageComparison> = alternatives.iter().collect();
        sorted.sort_by(|a, b| {
            let score_a = a.maintenance_score * 0.4 + a.security_score * 0.4
                + (a.downloads as f64).log10() * 0.2;
            let score_b = b.maintenance_score * 0.4 + b.security_score * 0.4
                + (b.downloads as f64).log10() * 0.2;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }

    /// Check deps against license policy.
    pub fn check_license_compliance(&self, licenses: &HashMap<String, String>) -> Vec<(String, String, bool)> {
        let mut results = Vec::new();
        for dep in &self.deps {
            if let Some(lic) = licenses.get(&dep.name) {
                let blocked = self.config.license_policy.blocked.iter()
                    .any(|b| lic.contains(b));
                let allowed = self.config.license_policy.allowed.iter()
                    .any(|a| lic.contains(a));
                results.push((dep.name.clone(), lic.clone(), !blocked && allowed));
            }
        }
        results
    }

    /// Detect deps that are never referenced in source.
    pub fn detect_unused(&self) -> Vec<&Dependency> {
        self.deps.iter()
            .filter(|d| !d.dev_only && !self.usage_counts.contains_key(&d.name))
            .collect()
    }

    /// Generate a lockfile entry for a dependency.
    pub fn generate_lockfile_entry(&self, dep: &Dependency) -> String {
        match dep.manager {
            PackageManager::Npm => {
                format!(
                    r#""{}" : {{ "version": "{}", "resolved": "https://registry.npmjs.org/{}/-/{}-{}.tgz" }}"#,
                    dep.name, dep.version, dep.name, dep.name, dep.version
                )
            }
            PackageManager::Cargo => {
                format!(
                    r#"[[package]]
name = "{}"
version = "{}""#,
                    dep.name, dep.version
                )
            }
            PackageManager::Pip => {
                format!("{}=={}", dep.name, dep.version)
            }
            PackageManager::Go => {
                format!("require {} v{}", dep.name, dep.version)
            }
            PackageManager::Maven => {
                format!(
                    "<dependency>\n  <groupId>{}</groupId>\n  <artifactId>{}</artifactId>\n  <version>{}</version>\n</dependency>",
                    dep.name, dep.name, dep.version
                )
            }
            PackageManager::Gradle => {
                format!("implementation '{}:{}'", dep.name, dep.version)
            }
        }
    }

    /// Count total deps.
    pub fn total(&self) -> usize {
        self.deps.len()
    }

    /// Count dev-only deps.
    pub fn dev_count(&self) -> usize {
        self.deps.iter().filter(|d| d.dev_only).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyzer() -> DepAnalyzer {
        DepAnalyzer::new(DepAnalyzerConfig::default())
    }

    #[test]
    fn test_package_manager_lockfile() {
        assert_eq!(PackageManager::Npm.lockfile_name(), "package-lock.json");
        assert_eq!(PackageManager::Cargo.lockfile_name(), "Cargo.lock");
        assert_eq!(PackageManager::Pip.lockfile_name(), "requirements.txt");
    }

    #[test]
    fn test_package_manager_manifest() {
        assert_eq!(PackageManager::Npm.manifest_name(), "package.json");
        assert_eq!(PackageManager::Go.manifest_name(), "go.mod");
    }

    #[test]
    fn test_dependency_new() {
        let d = Dependency::new("serde", "1.0", PackageManager::Cargo);
        assert_eq!(d.name, "serde");
        assert!(!d.dev_only);
    }

    #[test]
    fn test_dependency_dev() {
        let d = Dependency::new("jest", "29.0", PackageManager::Npm).dev();
        assert!(d.dev_only);
    }

    #[test]
    fn test_analyzer_new() {
        let a = analyzer();
        assert!(a.deps.is_empty());
        assert!(a.conflicts.is_empty());
    }

    #[test]
    fn test_add_dep() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("react", "18.0", PackageManager::Npm));
        assert_eq!(a.deps.len(), 1);
    }

    #[test]
    fn test_analyze_groups() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("react", "18.0", PackageManager::Npm));
        a.add_dep(Dependency::new("serde", "1.0", PackageManager::Cargo));
        a.add_dep(Dependency::new("vue", "3.0", PackageManager::Npm));
        let groups = a.analyze();
        assert_eq!(groups[&PackageManager::Npm].len(), 2);
        assert_eq!(groups[&PackageManager::Cargo].len(), 1);
    }

    #[test]
    fn test_detect_conflicts_none() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("react", "18.0", PackageManager::Npm));
        a.detect_conflicts();
        assert!(a.conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_found() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("lodash", "4.17.0", PackageManager::Npm));
        a.add_dep(Dependency::new("lodash", "4.18.0", PackageManager::Npm));
        let conflicts = a.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].package, "lodash");
    }

    #[test]
    fn test_detect_conflicts_same_version() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        a.detect_conflicts();
        assert!(a.conflicts.is_empty());
    }

    #[test]
    fn test_resolve_conflict_newest() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        a.add_dep(Dependency::new("x", "2.0", PackageManager::Npm));
        a.detect_conflicts();
        let r = a.resolve_conflict("x", &ResolutionStrategy::Newest).unwrap();
        assert_eq!(r, "2.0");
    }

    #[test]
    fn test_resolve_conflict_oldest() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        a.add_dep(Dependency::new("x", "2.0", PackageManager::Npm));
        a.detect_conflicts();
        let r = a.resolve_conflict("x", &ResolutionStrategy::Oldest).unwrap();
        assert_eq!(r, "1.0");
    }

    #[test]
    fn test_resolve_conflict_fork() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        a.add_dep(Dependency::new("x", "2.0", PackageManager::Npm));
        a.detect_conflicts();
        let r = a.resolve_conflict("x", &ResolutionStrategy::Fork).unwrap();
        assert!(r.contains("fork"));
    }

    #[test]
    fn test_resolve_conflict_not_found() {
        let mut a = analyzer();
        assert!(a.resolve_conflict("nope", &ResolutionStrategy::Newest).is_err());
    }

    #[test]
    fn test_check_security() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("lodash", "4.17.0", PackageManager::Npm));
        a.add_advisory(SecurityAdvisory {
            id: "CVE-2021-1234".to_string(),
            package: "lodash".to_string(),
            severity: Severity::High,
            fixed_version: Some("4.17.21".to_string()),
            description: "Prototype pollution".to_string(),
        });
        let results = a.check_security();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.severity, Severity::High);
    }

    #[test]
    fn test_check_security_no_match() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("react", "18.0", PackageManager::Npm));
        a.add_advisory(SecurityAdvisory {
            id: "CVE-1".to_string(),
            package: "vue".to_string(),
            severity: Severity::Low,
            fixed_version: None,
            description: "x".to_string(),
        });
        assert!(a.check_security().is_empty());
    }

    #[test]
    fn test_compare_alternatives() {
        let a = analyzer();
        let alts = vec![
            PackageComparison { name: "a".to_string(), downloads: 1000, maintenance_score: 0.5, security_score: 0.5, license: "MIT".to_string(), size_kb: 100 },
            PackageComparison { name: "b".to_string(), downloads: 100000, maintenance_score: 0.9, security_score: 0.9, license: "MIT".to_string(), size_kb: 50 },
        ];
        let sorted = a.compare_alternatives(&alts);
        assert_eq!(sorted[0].name, "b");
    }

    #[test]
    fn test_check_license_compliance_allowed() {
        let a = analyzer();
        let mut a2 = analyzer();
        a2.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        let mut licenses = HashMap::new();
        licenses.insert("x".to_string(), "MIT".to_string());
        let results = a2.check_license_compliance(&licenses);
        assert_eq!(results.len(), 1);
        assert!(results[0].2); // compliant
    }

    #[test]
    fn test_check_license_compliance_blocked() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        let mut licenses = HashMap::new();
        licenses.insert("x".to_string(), "GPL-3.0".to_string());
        let results = a.check_license_compliance(&licenses);
        assert!(!results[0].2); // non-compliant
    }

    #[test]
    fn test_detect_unused() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("used", "1.0", PackageManager::Npm));
        a.add_dep(Dependency::new("unused", "1.0", PackageManager::Npm));
        a.mark_used("used");
        let unused = a.detect_unused();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].name, "unused");
    }

    #[test]
    fn test_detect_unused_ignores_dev() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("jest", "1.0", PackageManager::Npm).dev());
        let unused = a.detect_unused();
        assert!(unused.is_empty()); // dev deps excluded
    }

    #[test]
    fn test_generate_lockfile_npm() {
        let a = analyzer();
        let dep = Dependency::new("react", "18.2.0", PackageManager::Npm);
        let entry = a.generate_lockfile_entry(&dep);
        assert!(entry.contains("react"));
        assert!(entry.contains("18.2.0"));
    }

    #[test]
    fn test_generate_lockfile_cargo() {
        let a = analyzer();
        let dep = Dependency::new("serde", "1.0.193", PackageManager::Cargo);
        let entry = a.generate_lockfile_entry(&dep);
        assert!(entry.contains("[[package]]"));
        assert!(entry.contains("serde"));
    }

    #[test]
    fn test_generate_lockfile_pip() {
        let a = analyzer();
        let dep = Dependency::new("flask", "3.0.0", PackageManager::Pip);
        let entry = a.generate_lockfile_entry(&dep);
        assert_eq!(entry, "flask==3.0.0");
    }

    #[test]
    fn test_generate_lockfile_go() {
        let a = analyzer();
        let dep = Dependency::new("github.com/gin-gonic/gin", "1.9.1", PackageManager::Go);
        let entry = a.generate_lockfile_entry(&dep);
        assert!(entry.contains("require"));
    }

    #[test]
    fn test_generate_lockfile_maven() {
        let a = analyzer();
        let dep = Dependency::new("junit", "5.0", PackageManager::Maven);
        let entry = a.generate_lockfile_entry(&dep);
        assert!(entry.contains("<dependency>"));
    }

    #[test]
    fn test_generate_lockfile_gradle() {
        let a = analyzer();
        let dep = Dependency::new("com.google.guava:guava", "32.0", PackageManager::Gradle);
        let entry = a.generate_lockfile_entry(&dep);
        assert!(entry.contains("implementation"));
    }

    #[test]
    fn test_total_and_dev_count() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("a", "1.0", PackageManager::Npm));
        a.add_dep(Dependency::new("b", "1.0", PackageManager::Npm).dev());
        assert_eq!(a.total(), 2);
        assert_eq!(a.dev_count(), 1);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn test_license_policy_default() {
        let p = LicensePolicy::default();
        assert!(p.allowed.contains(&"MIT".to_string()));
        assert!(p.blocked.contains(&"GPL-3.0".to_string()));
    }

    #[test]
    fn test_dependency_serde() {
        let d = Dependency::new("x", "1.0", PackageManager::Npm);
        let json = serde_json::to_string(&d).unwrap();
        let de: Dependency = serde_json::from_str(&json).unwrap();
        assert_eq!(d, de);
    }

    #[test]
    fn test_dep_conflict_serde() {
        let c = DepConflict {
            package: "x".to_string(),
            required_a: "1.0".to_string(),
            required_b: "2.0".to_string(),
            resolution: None,
        };
        let json = serde_json::to_string(&c).unwrap();
        let de: DepConflict = serde_json::from_str(&json).unwrap();
        assert_eq!(c, de);
    }

    #[test]
    fn test_multiple_advisories_same_package() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        a.add_advisory(SecurityAdvisory {
            id: "CVE-1".to_string(), package: "x".to_string(),
            severity: Severity::High, fixed_version: None, description: "a".to_string(),
        });
        a.add_advisory(SecurityAdvisory {
            id: "CVE-2".to_string(), package: "x".to_string(),
            severity: Severity::Critical, fixed_version: None, description: "b".to_string(),
        });
        assert_eq!(a.check_security().len(), 2);
    }

    #[test]
    fn test_mark_used() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        a.mark_used("x");
        a.mark_used("x");
        assert_eq!(a.usage_counts["x"], 2);
    }

    #[test]
    fn test_check_license_no_license_info() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        let licenses = HashMap::new(); // no license info
        let results = a.check_license_compliance(&licenses);
        assert!(results.is_empty());
    }

    #[test]
    fn test_resolve_sets_resolution() {
        let mut a = analyzer();
        a.add_dep(Dependency::new("x", "1.0", PackageManager::Npm));
        a.add_dep(Dependency::new("x", "2.0", PackageManager::Npm));
        a.detect_conflicts();
        a.resolve_conflict("x", &ResolutionStrategy::Newest).unwrap();
        assert!(a.conflicts[0].resolution.is_some());
    }

    #[test]
    fn test_analyzer_config_default() {
        let cfg = DepAnalyzerConfig::default();
        assert!(!cfg.auto_fix);
        assert_eq!(cfg.max_depth, 10);
    }
}
