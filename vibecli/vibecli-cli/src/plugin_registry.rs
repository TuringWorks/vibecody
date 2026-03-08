//! Plugin Registry — publishing, discovery, verification, and dependency resolution.

use crate::plugin_sdk::{PluginManifestV2, PluginKind, PluginDependency};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Registry entry for a published plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub kind: PluginKind,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Vec<String>,
    pub downloads: u64,
    pub rating: f32,        // 0.0 - 5.0
    pub review_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub checksum: String,   // SHA-256 of plugin archive
    pub signature: Option<String>, // GPG/minisign signature
    pub verified: bool,     // verified by VibeCody team
    pub archive_url: String,
    pub archive_size: u64,
    pub platforms: Vec<String>,
    pub dependencies: Vec<PluginDependency>,
    pub all_versions: Vec<VersionEntry>,
}

/// Version history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub version: String,
    pub checksum: String,
    pub archive_url: String,
    pub published_at: String,
    pub changelog: Option<String>,
    pub yanked: bool,
}

/// Publisher identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publisher {
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub verified: bool,
    pub plugins: Vec<String>,
    pub public_key: Option<String>,
}

/// Registry index (cached locally)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub entries: Vec<RegistryEntry>,
    pub publishers: Vec<Publisher>,
    pub updated_at: String,
    pub registry_version: String,
}

/// Plugin registry client
pub struct PluginRegistry {
    pub registry_url: String,
    pub cache_dir: PathBuf,
    pub index: Option<RegistryIndex>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            registry_url: "https://registry.vibecody.dev/api/v1".to_string(),
            cache_dir: home.join(".vibecli").join("registry-cache"),
            index: None,
        }
    }

    pub fn with_url(mut self, url: &str) -> Self {
        self.registry_url = url.to_string();
        self
    }

    /// Load cached index from disk
    pub fn load_cached(&mut self) -> anyhow::Result<&RegistryIndex> {
        let cache_path = self.cache_dir.join("index.json");
        if cache_path.exists() {
            let data = std::fs::read_to_string(&cache_path)?;
            self.index = Some(serde_json::from_str(&data)?);
        } else {
            self.index = Some(Self::built_in_index());
        }
        Ok(self.index.as_ref().unwrap())
    }

    /// Search plugins by query
    pub fn search(&self, query: &str, kind: Option<&PluginKind>) -> Vec<&RegistryEntry> {
        let index = match &self.index {
            Some(i) => i,
            None => return vec![],
        };

        let query_lower = query.to_lowercase();
        let mut results: Vec<(&RegistryEntry, u32)> = index.entries.iter()
            .filter(|e| {
                if let Some(k) = kind {
                    if &e.kind != k { return false; }
                }
                true
            })
            .filter_map(|e| {
                let mut score = 0u32;
                if e.name.to_lowercase().contains(&query_lower) { score += 10; }
                if e.display_name.to_lowercase().contains(&query_lower) { score += 8; }
                if e.description.to_lowercase().contains(&query_lower) { score += 5; }
                if e.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower)) { score += 6; }
                if e.author.to_lowercase().contains(&query_lower) { score += 3; }
                if score > 0 { Some((e, score)) } else { None }
            })
            .collect();

        results.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.downloads.cmp(&a.0.downloads)));
        results.into_iter().map(|(e, _)| e).collect()
    }

    /// Find plugin by exact name
    pub fn find(&self, name: &str) -> Option<&RegistryEntry> {
        self.index.as_ref()?.entries.iter().find(|e| e.name == name)
    }

    /// List all plugins of a specific kind
    pub fn list_by_kind(&self, kind: &PluginKind) -> Vec<&RegistryEntry> {
        match &self.index {
            Some(idx) => idx.entries.iter().filter(|e| &e.kind == kind).collect(),
            None => vec![],
        }
    }

    /// Get top plugins by downloads
    pub fn trending(&self, limit: usize) -> Vec<&RegistryEntry> {
        let mut entries: Vec<&RegistryEntry> = match &self.index {
            Some(idx) => idx.entries.iter().collect(),
            None => return vec![],
        };
        entries.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        entries.into_iter().take(limit).collect()
    }

    /// Verify plugin checksum
    pub fn verify_checksum(archive_path: &Path, expected: &str) -> anyhow::Result<bool> {
        use sha2::{Sha256, Digest};
        let data = std::fs::read(archive_path)?;
        let hash = format!("{:x}", Sha256::digest(&data));
        Ok(hash == expected)
    }

    /// Resolve dependency tree (returns install order)
    pub fn resolve_dependencies(&self, name: &str) -> anyhow::Result<Vec<String>> {
        let mut resolved = vec![];
        let mut visited = std::collections::HashSet::new();
        self.resolve_recursive(name, &mut resolved, &mut visited)?;
        Ok(resolved)
    }

    fn resolve_recursive(
        &self,
        name: &str,
        resolved: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
    ) -> anyhow::Result<()> {
        if visited.contains(name) {
            return Ok(()); // already resolved or circular
        }
        visited.insert(name.to_string());

        if let Some(entry) = self.find(name) {
            for dep in &entry.dependencies {
                if !dep.optional {
                    self.resolve_recursive(&dep.name, resolved, visited)?;
                }
            }
            resolved.push(name.to_string());
        } else {
            anyhow::bail!("Plugin '{}' not found in registry", name);
        }
        Ok(())
    }

    /// Prepare a plugin for publishing (validate, package, checksum)
    pub fn prepare_publish(plugin_dir: &Path) -> anyhow::Result<PublishPackage> {
        let manifest_path = plugin_dir.join("plugin.toml");
        if !manifest_path.exists() {
            anyhow::bail!("No plugin.toml found in {}", plugin_dir.display());
        }

        let manifest_str = std::fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifestV2 = toml::from_str(&manifest_str)?;

        let errors = crate::plugin_sdk::validate_manifest(&manifest);
        if !errors.is_empty() {
            anyhow::bail!("Manifest validation failed:\n{}", errors.join("\n"));
        }

        Ok(PublishPackage {
            manifest,
            plugin_dir: plugin_dir.to_path_buf(),
            archive_path: None,
            checksum: None,
        })
    }

    /// Built-in starter index with example plugins
    fn built_in_index() -> RegistryIndex {
        RegistryIndex {
            entries: vec![
                RegistryEntry {
                    name: "vibecody-jira".to_string(),
                    display_name: "Jira Connector".to_string(),
                    description: "Integrate Jira issues, sprints, and boards with VibeCody agent".to_string(),
                    version: "1.2.0".to_string(),
                    author: "vibecody-team".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::Connector,
                    repository: Some("https://github.com/vibecody/plugin-jira".to_string()),
                    homepage: None,
                    keywords: vec!["jira".into(), "atlassian".into(), "project-management".into()],
                    downloads: 15420,
                    rating: 4.5,
                    review_count: 87,
                    created_at: "2025-06-15".to_string(),
                    updated_at: "2026-02-28".to_string(),
                    checksum: "abc123".to_string(),
                    signature: None,
                    verified: true,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-jira/1.2.0.tar.gz".to_string(),
                    archive_size: 45_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-linear".to_string(),
                    display_name: "Linear Connector".to_string(),
                    description: "Sync Linear issues, cycles, and projects with VibeCody workflows".to_string(),
                    version: "1.0.3".to_string(),
                    author: "vibecody-team".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::Connector,
                    repository: Some("https://github.com/vibecody/plugin-linear".to_string()),
                    homepage: None,
                    keywords: vec!["linear".into(), "issues".into(), "project-management".into()],
                    downloads: 8930,
                    rating: 4.7,
                    review_count: 42,
                    created_at: "2025-09-10".to_string(),
                    updated_at: "2026-03-01".to_string(),
                    checksum: "def456".to_string(),
                    signature: None,
                    verified: true,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-linear/1.0.3.tar.gz".to_string(),
                    archive_size: 38_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-notion".to_string(),
                    display_name: "Notion Connector".to_string(),
                    description: "Read/write Notion pages, databases, and blocks from VibeCody".to_string(),
                    version: "0.9.1".to_string(),
                    author: "community".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::Connector,
                    repository: Some("https://github.com/example/vibecody-notion".to_string()),
                    homepage: None,
                    keywords: vec!["notion".into(), "wiki".into(), "documentation".into()],
                    downloads: 6210,
                    rating: 4.2,
                    review_count: 28,
                    created_at: "2025-11-20".to_string(),
                    updated_at: "2026-01-15".to_string(),
                    checksum: "ghi789".to_string(),
                    signature: None,
                    verified: false,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-notion/0.9.1.tar.gz".to_string(),
                    archive_size: 32_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-prettier".to_string(),
                    display_name: "Prettier Formatter".to_string(),
                    description: "Auto-format code with Prettier on file save and agent edits".to_string(),
                    version: "2.1.0".to_string(),
                    author: "vibecody-team".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::Optimizer,
                    repository: Some("https://github.com/vibecody/plugin-prettier".to_string()),
                    homepage: None,
                    keywords: vec!["prettier".into(), "formatting".into(), "code-style".into()],
                    downloads: 32_100,
                    rating: 4.8,
                    review_count: 156,
                    created_at: "2025-03-01".to_string(),
                    updated_at: "2026-02-20".to_string(),
                    checksum: "jkl012".to_string(),
                    signature: None,
                    verified: true,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-prettier/2.1.0.tar.gz".to_string(),
                    archive_size: 12_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-eslint".to_string(),
                    display_name: "ESLint Integration".to_string(),
                    description: "Run ESLint on TypeScript/JavaScript edits with auto-fix suggestions".to_string(),
                    version: "1.5.0".to_string(),
                    author: "vibecody-team".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::Optimizer,
                    repository: Some("https://github.com/vibecody/plugin-eslint".to_string()),
                    homepage: None,
                    keywords: vec!["eslint".into(), "linting".into(), "typescript".into(), "javascript".into()],
                    downloads: 28_400,
                    rating: 4.6,
                    review_count: 120,
                    created_at: "2025-04-15".to_string(),
                    updated_at: "2026-03-05".to_string(),
                    checksum: "mno345".to_string(),
                    signature: None,
                    verified: true,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-eslint/1.5.0.tar.gz".to_string(),
                    archive_size: 15_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-docker-compose".to_string(),
                    display_name: "Docker Compose Manager".to_string(),
                    description: "Manage Docker Compose stacks with agent-driven orchestration".to_string(),
                    version: "1.1.0".to_string(),
                    author: "vibecody-team".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::Adapter,
                    repository: Some("https://github.com/vibecody/plugin-docker-compose".to_string()),
                    homepage: None,
                    keywords: vec!["docker".into(), "compose".into(), "containers".into()],
                    downloads: 19_800,
                    rating: 4.4,
                    review_count: 73,
                    created_at: "2025-07-01".to_string(),
                    updated_at: "2026-02-10".to_string(),
                    checksum: "pqr678".to_string(),
                    signature: None,
                    verified: true,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-docker-compose/1.1.0.tar.gz".to_string(),
                    archive_size: 22_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-terraform".to_string(),
                    display_name: "Terraform Integration".to_string(),
                    description: "Terraform plan, apply, and drift detection with agent context".to_string(),
                    version: "1.3.0".to_string(),
                    author: "vibecody-team".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::Adapter,
                    repository: Some("https://github.com/vibecody/plugin-terraform".to_string()),
                    homepage: None,
                    keywords: vec!["terraform".into(), "infrastructure".into(), "iac".into()],
                    downloads: 14_200,
                    rating: 4.3,
                    review_count: 55,
                    created_at: "2025-05-20".to_string(),
                    updated_at: "2026-01-30".to_string(),
                    checksum: "stu901".to_string(),
                    signature: None,
                    verified: true,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-terraform/1.3.0.tar.gz".to_string(),
                    archive_size: 18_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-dracula-theme".to_string(),
                    display_name: "Dracula Theme".to_string(),
                    description: "Dracula color theme for VibeUI editor and terminal".to_string(),
                    version: "1.0.0".to_string(),
                    author: "community".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::Theme,
                    repository: Some("https://github.com/example/vibecody-dracula".to_string()),
                    homepage: None,
                    keywords: vec!["theme".into(), "dracula".into(), "dark".into()],
                    downloads: 11_500,
                    rating: 4.9,
                    review_count: 210,
                    created_at: "2025-08-01".to_string(),
                    updated_at: "2025-12-15".to_string(),
                    checksum: "vwx234".to_string(),
                    signature: None,
                    verified: false,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-dracula-theme/1.0.0.tar.gz".to_string(),
                    archive_size: 5_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-devops-pack".to_string(),
                    display_name: "DevOps Skill Pack".to_string(),
                    description: "50+ DevOps skills covering CI/CD, monitoring, IaC, and SRE".to_string(),
                    version: "2.0.0".to_string(),
                    author: "vibecody-team".to_string(),
                    license: "MIT".to_string(),
                    kind: PluginKind::SkillPack,
                    repository: Some("https://github.com/vibecody/plugin-devops-pack".to_string()),
                    homepage: None,
                    keywords: vec!["devops".into(), "ci-cd".into(), "sre".into(), "skills".into()],
                    downloads: 9_700,
                    rating: 4.6,
                    review_count: 35,
                    created_at: "2025-10-01".to_string(),
                    updated_at: "2026-03-01".to_string(),
                    checksum: "yza567".to_string(),
                    signature: None,
                    verified: true,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-devops-pack/2.0.0.tar.gz".to_string(),
                    archive_size: 85_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
                RegistryEntry {
                    name: "vibecody-code-review".to_string(),
                    display_name: "Code Review Workflow".to_string(),
                    description: "Automated code review workflow with configurable rules and PR integration".to_string(),
                    version: "1.0.0".to_string(),
                    author: "community".to_string(),
                    license: "Apache-2.0".to_string(),
                    kind: PluginKind::Workflow,
                    repository: Some("https://github.com/example/vibecody-code-review".to_string()),
                    homepage: None,
                    keywords: vec!["code-review".into(), "workflow".into(), "pull-request".into()],
                    downloads: 7_300,
                    rating: 4.1,
                    review_count: 19,
                    created_at: "2025-12-01".to_string(),
                    updated_at: "2026-02-15".to_string(),
                    checksum: "bcd890".to_string(),
                    signature: None,
                    verified: false,
                    archive_url: "https://registry.vibecody.dev/plugins/vibecody-code-review/1.0.0.tar.gz".to_string(),
                    archive_size: 28_000,
                    platforms: vec!["all".into()],
                    dependencies: vec![],
                    all_versions: vec![],
                },
            ],
            publishers: vec![
                Publisher {
                    username: "vibecody-team".to_string(),
                    display_name: "VibeCody Team".to_string(),
                    email: Some("plugins@vibecody.dev".to_string()),
                    verified: true,
                    plugins: vec![
                        "vibecody-jira".into(), "vibecody-linear".into(),
                        "vibecody-prettier".into(), "vibecody-eslint".into(),
                        "vibecody-docker-compose".into(), "vibecody-terraform".into(),
                        "vibecody-devops-pack".into(),
                    ],
                    public_key: None,
                },
            ],
            updated_at: "2026-03-07".to_string(),
            registry_version: "1.0.0".to_string(),
        }
    }
}

/// Package prepared for publishing
#[derive(Debug)]
pub struct PublishPackage {
    pub manifest: PluginManifestV2,
    pub plugin_dir: PathBuf,
    pub archive_path: Option<PathBuf>,
    pub checksum: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_built_in_index() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        assert!(reg.index.is_some());
        let idx = reg.index.as_ref().unwrap();
        assert!(idx.entries.len() >= 10);
    }

    #[test]
    fn test_search_by_name() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let results = reg.search("jira", None);
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "vibecody-jira");
    }

    #[test]
    fn test_search_by_keyword() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let results = reg.search("formatting", None);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_by_kind() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let results = reg.search("", Some(&PluginKind::Connector));
        // Should find jira, linear, notion
        assert!(results.len() >= 3);
    }

    #[test]
    fn test_find_by_name() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let entry = reg.find("vibecody-prettier");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().version, "2.1.0");
    }

    #[test]
    fn test_find_not_found() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        assert!(reg.find("nonexistent").is_none());
    }

    #[test]
    fn test_list_by_kind() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let connectors = reg.list_by_kind(&PluginKind::Connector);
        assert!(connectors.len() >= 3);
        let themes = reg.list_by_kind(&PluginKind::Theme);
        assert!(themes.len() >= 1);
    }

    #[test]
    fn test_trending() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let top = reg.trending(3);
        assert_eq!(top.len(), 3);
        // Should be sorted by downloads descending
        assert!(top[0].downloads >= top[1].downloads);
        assert!(top[1].downloads >= top[2].downloads);
    }

    #[test]
    fn test_resolve_dependencies_no_deps() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let order = reg.resolve_dependencies("vibecody-jira").unwrap();
        assert_eq!(order, vec!["vibecody-jira"]);
    }

    #[test]
    fn test_resolve_dependencies_not_found() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let result = reg.resolve_dependencies("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_publisher_verified() {
        let mut reg = PluginRegistry::new();
        reg.load_cached().unwrap();
        let idx = reg.index.as_ref().unwrap();
        let team = idx.publishers.iter().find(|p| p.username == "vibecody-team");
        assert!(team.is_some());
        assert!(team.unwrap().verified);
    }
}
