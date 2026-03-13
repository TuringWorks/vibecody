//! MCP verified plugin directory for VibeCody.
//!
//! Provides a searchable, reviewable directory of MCP plugins with
//! verification status tracking, permission management, installation
//! lifecycle, and category-based organization.

use std::collections::HashMap;
use std::collections::HashSet;

/// Categories for organizing MCP plugins.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PluginCategory {
    DataSource,
    CodeAnalysis,
    DevOps,
    Communication,
    Database,
    Testing,
    Security,
    Monitoring,
    Documentation,
    Utility,
}

impl PluginCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DataSource => "data_source",
            Self::CodeAnalysis => "code_analysis",
            Self::DevOps => "devops",
            Self::Communication => "communication",
            Self::Database => "database",
            Self::Testing => "testing",
            Self::Security => "security",
            Self::Monitoring => "monitoring",
            Self::Documentation => "documentation",
            Self::Utility => "utility",
        }
    }
}

/// Verification status of a plugin.
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Rejected { reason: String },
}

/// Permissions a plugin may request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PluginPermission {
    FileRead,
    FileWrite,
    NetworkAccess,
    ProcessExec,
    EnvAccess,
    SystemInfo,
}

/// An MCP plugin entry in the directory.
#[derive(Debug, Clone, PartialEq)]
pub struct McpPlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub category: PluginCategory,
    pub verification: VerificationStatus,
    pub downloads: u64,
    pub rating: f64,
    pub rating_count: u32,
    pub tools: Vec<String>,
    pub permissions: Vec<PluginPermission>,
    pub checksum: String,
    pub repository_url: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// A search result entry.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginSearchResult {
    pub plugin_id: String,
    pub name: String,
    pub description: String,
    pub relevance: f64,
    pub verified: bool,
}

/// A user review of a plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginReview {
    pub plugin_id: String,
    pub reviewer: String,
    pub rating: u8,
    pub comment: String,
    pub timestamp: u64,
}

/// Result of an install/uninstall/update operation.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallResult {
    pub plugin_id: String,
    pub success: bool,
    pub message: String,
}

/// Configuration for the plugin directory.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryConfig {
    pub allow_unverified: bool,
    pub auto_update: bool,
    pub max_installed: usize,
}

impl Default for DirectoryConfig {
    fn default() -> Self {
        Self {
            allow_unverified: false,
            auto_update: true,
            max_installed: 50,
        }
    }
}

/// The plugin directory managing all plugins, installations, and reviews.
#[derive(Debug, Clone)]
pub struct PluginDirectory {
    pub plugins: HashMap<String, McpPlugin>,
    pub installed: HashSet<String>,
    pub config: DirectoryConfig,
    reviews: Vec<PluginReview>,
}

impl PluginDirectory {
    pub fn new(config: DirectoryConfig) -> Self {
        Self {
            plugins: HashMap::new(),
            installed: HashSet::new(),
            config,
            reviews: Vec::new(),
        }
    }

    pub fn add_plugin(&mut self, plugin: McpPlugin) -> Result<(), String> {
        if plugin.id.is_empty() {
            return Err("Plugin ID cannot be empty".to_string());
        }
        if plugin.name.is_empty() {
            return Err("Plugin name cannot be empty".to_string());
        }
        if self.plugins.contains_key(&plugin.id) {
            return Err(format!("Plugin '{}' already exists", plugin.id));
        }
        self.plugins.insert(plugin.id.clone(), plugin);
        Ok(())
    }

    pub fn remove_plugin(&mut self, id: &str) -> bool {
        if self.plugins.remove(id).is_some() {
            self.installed.remove(id);
            self.reviews.retain(|r| r.plugin_id != id);
            true
        } else {
            false
        }
    }

    pub fn search(
        &self,
        query: &str,
        category: Option<PluginCategory>,
        verified_only: bool,
        max: usize,
    ) -> Vec<PluginSearchResult> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<PluginSearchResult> = self
            .plugins
            .values()
            .filter(|p| {
                if verified_only && p.verification != VerificationStatus::Verified {
                    return false;
                }
                if let Some(ref cat) = category {
                    if &p.category != cat {
                        return false;
                    }
                }
                true
            })
            .filter_map(|p| {
                let relevance = compute_relevance(p, &query_lower);
                if relevance > 0.0 {
                    Some(PluginSearchResult {
                        plugin_id: p.id.clone(),
                        name: p.name.clone(),
                        description: p.description.clone(),
                        relevance,
                        verified: p.verification == VerificationStatus::Verified,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(max);
        results
    }

    pub fn install(&mut self, id: &str) -> InstallResult {
        let plugin = match self.plugins.get(id) {
            Some(p) => p,
            None => {
                return InstallResult {
                    plugin_id: id.to_string(),
                    success: false,
                    message: format!("Plugin '{}' not found", id),
                };
            }
        };

        if self.installed.contains(id) {
            return InstallResult {
                plugin_id: id.to_string(),
                success: false,
                message: "Plugin already installed".to_string(),
            };
        }

        if self.installed.len() >= self.config.max_installed {
            return InstallResult {
                plugin_id: id.to_string(),
                success: false,
                message: format!(
                    "Maximum installed plugins ({}) reached",
                    self.config.max_installed
                ),
            };
        }

        if !self.config.allow_unverified && plugin.verification != VerificationStatus::Verified {
            return InstallResult {
                plugin_id: id.to_string(),
                success: false,
                message: "Plugin is not verified and unverified plugins are not allowed".to_string(),
            };
        }

        self.installed.insert(id.to_string());
        InstallResult {
            plugin_id: id.to_string(),
            success: true,
            message: format!("Plugin '{}' installed successfully", plugin.name),
        }
    }

    pub fn uninstall(&mut self, id: &str) -> InstallResult {
        if !self.installed.remove(id) {
            return InstallResult {
                plugin_id: id.to_string(),
                success: false,
                message: "Plugin is not installed".to_string(),
            };
        }
        InstallResult {
            plugin_id: id.to_string(),
            success: true,
            message: "Plugin uninstalled successfully".to_string(),
        }
    }

    pub fn update(&mut self, id: &str) -> InstallResult {
        if !self.installed.contains(id) {
            return InstallResult {
                plugin_id: id.to_string(),
                success: false,
                message: "Plugin is not installed, cannot update".to_string(),
            };
        }
        if !self.plugins.contains_key(id) {
            return InstallResult {
                plugin_id: id.to_string(),
                success: false,
                message: "Plugin not found in directory".to_string(),
            };
        }
        InstallResult {
            plugin_id: id.to_string(),
            success: true,
            message: "Plugin updated successfully".to_string(),
        }
    }

    pub fn verify_plugin(&self, id: &str, checksum: &str) -> bool {
        self.plugins
            .get(id)
            .map(|p| p.checksum == checksum)
            .unwrap_or(false)
    }

    pub fn add_review(&mut self, review: PluginReview) -> Result<(), String> {
        if !self.plugins.contains_key(&review.plugin_id) {
            return Err(format!("Plugin '{}' not found", review.plugin_id));
        }
        if review.rating > 5 {
            return Err("Rating must be between 0 and 5".to_string());
        }
        if review.reviewer.is_empty() {
            return Err("Reviewer name cannot be empty".to_string());
        }

        let plugin_id = review.plugin_id.clone();
        self.reviews.push(review);

        // Recalculate average rating
        let plugin_reviews: Vec<&PluginReview> = self
            .reviews
            .iter()
            .filter(|r| r.plugin_id == plugin_id)
            .collect();
        let count = plugin_reviews.len() as u32;
        let sum: u64 = plugin_reviews.iter().map(|r| r.rating as u64).sum();
        let avg = if count > 0 {
            sum as f64 / count as f64
        } else {
            0.0
        };

        if let Some(plugin) = self.plugins.get_mut(&plugin_id) {
            plugin.rating = avg;
            plugin.rating_count = count;
        }

        Ok(())
    }

    pub fn get_reviews(&self, plugin_id: &str) -> Vec<&PluginReview> {
        self.reviews
            .iter()
            .filter(|r| r.plugin_id == plugin_id)
            .collect()
    }

    pub fn list_installed(&self) -> Vec<&McpPlugin> {
        self.installed
            .iter()
            .filter_map(|id| self.plugins.get(id))
            .collect()
    }

    pub fn list_by_category(&self, cat: &PluginCategory) -> Vec<&McpPlugin> {
        self.plugins
            .values()
            .filter(|p| &p.category == cat)
            .collect()
    }

    pub fn get_plugin(&self, id: &str) -> Option<&McpPlugin> {
        self.plugins.get(id)
    }

    pub fn top_rated(&self, limit: usize) -> Vec<&McpPlugin> {
        let mut plugins: Vec<&McpPlugin> = self.plugins.values().filter(|p| p.rating_count > 0).collect();
        plugins.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        plugins.truncate(limit);
        plugins
    }

    pub fn recently_updated(&self, limit: usize) -> Vec<&McpPlugin> {
        let mut plugins: Vec<&McpPlugin> = self.plugins.values().collect();
        plugins.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        plugins.truncate(limit);
        plugins
    }

    pub fn set_verification_status(
        &mut self,
        id: &str,
        status: VerificationStatus,
    ) -> Result<(), String> {
        match self.plugins.get_mut(id) {
            Some(plugin) => {
                plugin.verification = status;
                Ok(())
            }
            None => Err(format!("Plugin '{}' not found", id)),
        }
    }
}

/// Compute search relevance for a plugin against a query.
fn compute_relevance(plugin: &McpPlugin, query_lower: &str) -> f64 {
    if query_lower.is_empty() {
        return 1.0;
    }

    let mut score = 0.0;
    let name_lower = plugin.name.to_lowercase();
    let desc_lower = plugin.description.to_lowercase();

    if name_lower == query_lower {
        score += 1.0;
    } else if name_lower.contains(query_lower) {
        score += 0.8;
    }

    if desc_lower.contains(query_lower) {
        score += 0.4;
    }

    for tool in &plugin.tools {
        if tool.to_lowercase().contains(query_lower) {
            score += 0.2;
            break;
        }
    }

    if plugin.category.as_str().contains(query_lower) {
        score += 0.3;
    }

    // Boost verified plugins
    if plugin.verification == VerificationStatus::Verified {
        score *= 1.1;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plugin(id: &str, name: &str, category: PluginCategory) -> McpPlugin {
        McpPlugin {
            id: id.to_string(),
            name: name.to_string(),
            description: format!("A {} plugin", name),
            version: "1.0.0".to_string(),
            author: "author".to_string(),
            category,
            verification: VerificationStatus::Verified,
            downloads: 100,
            rating: 4.0,
            rating_count: 10,
            tools: vec!["tool_a".to_string()],
            permissions: vec![PluginPermission::FileRead],
            checksum: "abc123".to_string(),
            repository_url: Some("https://github.com/test/plugin".to_string()),
            created_at: 1000,
            updated_at: 2000,
        }
    }

    fn make_directory() -> PluginDirectory {
        let config = DirectoryConfig {
            allow_unverified: true,
            auto_update: true,
            max_installed: 10,
        };
        let mut dir = PluginDirectory::new(config);
        dir.add_plugin(make_plugin("pg-client", "PostgreSQL Client", PluginCategory::Database))
            .unwrap();
        dir.add_plugin(make_plugin("eslint-runner", "ESLint Runner", PluginCategory::CodeAnalysis))
            .unwrap();
        dir.add_plugin(make_plugin("slack-notify", "Slack Notifier", PluginCategory::Communication))
            .unwrap();
        dir
    }

    #[test]
    fn test_add_plugin() {
        let mut dir = PluginDirectory::new(DirectoryConfig::default());
        let plugin = make_plugin("test", "Test Plugin", PluginCategory::Utility);
        assert!(dir.add_plugin(plugin).is_ok());
        assert_eq!(dir.plugins.len(), 1);
    }

    #[test]
    fn test_add_duplicate_plugin() {
        let mut dir = PluginDirectory::new(DirectoryConfig::default());
        let p1 = make_plugin("dup", "Dup", PluginCategory::Utility);
        let p2 = make_plugin("dup", "Dup2", PluginCategory::Utility);
        assert!(dir.add_plugin(p1).is_ok());
        assert!(dir.add_plugin(p2).is_err());
    }

    #[test]
    fn test_add_plugin_empty_id() {
        let mut dir = PluginDirectory::new(DirectoryConfig::default());
        let mut p = make_plugin("x", "Name", PluginCategory::Utility);
        p.id = String::new();
        assert!(dir.add_plugin(p).is_err());
    }

    #[test]
    fn test_add_plugin_empty_name() {
        let mut dir = PluginDirectory::new(DirectoryConfig::default());
        let mut p = make_plugin("x", "Name", PluginCategory::Utility);
        p.name = String::new();
        assert!(dir.add_plugin(p).is_err());
    }

    #[test]
    fn test_remove_plugin() {
        let mut dir = make_directory();
        assert!(dir.remove_plugin("pg-client"));
        assert_eq!(dir.plugins.len(), 2);
        assert!(!dir.remove_plugin("nonexistent"));
    }

    #[test]
    fn test_remove_installed_plugin_cleans_up() {
        let mut dir = make_directory();
        dir.install("pg-client");
        assert!(dir.installed.contains("pg-client"));
        dir.remove_plugin("pg-client");
        assert!(!dir.installed.contains("pg-client"));
    }

    #[test]
    fn test_search_by_name() {
        let dir = make_directory();
        let results = dir.search("PostgreSQL", None, false, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].plugin_id, "pg-client");
    }

    #[test]
    fn test_search_empty_query_returns_all() {
        let dir = make_directory();
        let results = dir.search("", None, false, 10);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_verified_only() {
        let mut dir = make_directory();
        dir.set_verification_status("pg-client", VerificationStatus::Unverified).unwrap();
        let results = dir.search("", None, true, 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_by_category() {
        let dir = make_directory();
        let results = dir.search("", Some(PluginCategory::Database), false, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].plugin_id, "pg-client");
    }

    #[test]
    fn test_search_max_limit() {
        let dir = make_directory();
        let results = dir.search("", None, false, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_install_success() {
        let mut dir = make_directory();
        let result = dir.install("pg-client");
        assert!(result.success);
        assert!(dir.installed.contains("pg-client"));
    }

    #[test]
    fn test_install_not_found() {
        let mut dir = make_directory();
        let result = dir.install("nonexistent");
        assert!(!result.success);
        assert!(result.message.contains("not found"));
    }

    #[test]
    fn test_install_already_installed() {
        let mut dir = make_directory();
        dir.install("pg-client");
        let result = dir.install("pg-client");
        assert!(!result.success);
        assert!(result.message.contains("already installed"));
    }

    #[test]
    fn test_install_max_limit() {
        let config = DirectoryConfig {
            allow_unverified: true,
            auto_update: true,
            max_installed: 1,
        };
        let mut dir = PluginDirectory::new(config);
        dir.add_plugin(make_plugin("a", "A", PluginCategory::Utility)).unwrap();
        dir.add_plugin(make_plugin("b", "B", PluginCategory::Utility)).unwrap();
        dir.install("a");
        let result = dir.install("b");
        assert!(!result.success);
        assert!(result.message.contains("Maximum"));
    }

    #[test]
    fn test_install_unverified_blocked() {
        let config = DirectoryConfig {
            allow_unverified: false,
            auto_update: true,
            max_installed: 50,
        };
        let mut dir = PluginDirectory::new(config);
        let mut p = make_plugin("uv", "Unverified", PluginCategory::Utility);
        p.verification = VerificationStatus::Unverified;
        dir.add_plugin(p).unwrap();
        let result = dir.install("uv");
        assert!(!result.success);
        assert!(result.message.contains("not verified"));
    }

    #[test]
    fn test_uninstall_success() {
        let mut dir = make_directory();
        dir.install("pg-client");
        let result = dir.uninstall("pg-client");
        assert!(result.success);
        assert!(!dir.installed.contains("pg-client"));
    }

    #[test]
    fn test_uninstall_not_installed() {
        let mut dir = make_directory();
        let result = dir.uninstall("pg-client");
        assert!(!result.success);
    }

    #[test]
    fn test_update_success() {
        let mut dir = make_directory();
        dir.install("pg-client");
        let result = dir.update("pg-client");
        assert!(result.success);
    }

    #[test]
    fn test_update_not_installed() {
        let mut dir = make_directory();
        let result = dir.update("pg-client");
        assert!(!result.success);
    }

    #[test]
    fn test_verify_plugin_checksum_match() {
        let dir = make_directory();
        assert!(dir.verify_plugin("pg-client", "abc123"));
    }

    #[test]
    fn test_verify_plugin_checksum_mismatch() {
        let dir = make_directory();
        assert!(!dir.verify_plugin("pg-client", "wrong"));
    }

    #[test]
    fn test_verify_plugin_not_found() {
        let dir = make_directory();
        assert!(!dir.verify_plugin("nonexistent", "abc123"));
    }

    #[test]
    fn test_add_review_updates_rating() {
        let mut dir = make_directory();
        dir.add_review(PluginReview {
            plugin_id: "pg-client".to_string(),
            reviewer: "alice".to_string(),
            rating: 5,
            comment: "Great!".to_string(),
            timestamp: 3000,
        })
        .unwrap();
        dir.add_review(PluginReview {
            plugin_id: "pg-client".to_string(),
            reviewer: "bob".to_string(),
            rating: 3,
            comment: "OK".to_string(),
            timestamp: 3001,
        })
        .unwrap();
        let plugin = dir.get_plugin("pg-client").unwrap();
        assert_eq!(plugin.rating_count, 2);
        assert!((plugin.rating - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_review_plugin_not_found() {
        let mut dir = make_directory();
        let result = dir.add_review(PluginReview {
            plugin_id: "nonexistent".to_string(),
            reviewer: "alice".to_string(),
            rating: 5,
            comment: "x".to_string(),
            timestamp: 0,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_add_review_invalid_rating() {
        let mut dir = make_directory();
        let result = dir.add_review(PluginReview {
            plugin_id: "pg-client".to_string(),
            reviewer: "alice".to_string(),
            rating: 6,
            comment: "x".to_string(),
            timestamp: 0,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_add_review_empty_reviewer() {
        let mut dir = make_directory();
        let result = dir.add_review(PluginReview {
            plugin_id: "pg-client".to_string(),
            reviewer: "".to_string(),
            rating: 4,
            comment: "x".to_string(),
            timestamp: 0,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_reviews() {
        let mut dir = make_directory();
        dir.add_review(PluginReview {
            plugin_id: "pg-client".to_string(),
            reviewer: "alice".to_string(),
            rating: 5,
            comment: "Excellent".to_string(),
            timestamp: 3000,
        })
        .unwrap();
        let reviews = dir.get_reviews("pg-client");
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].reviewer, "alice");
    }

    #[test]
    fn test_list_installed() {
        let mut dir = make_directory();
        dir.install("pg-client");
        dir.install("eslint-runner");
        let installed = dir.list_installed();
        assert_eq!(installed.len(), 2);
    }

    #[test]
    fn test_list_by_category() {
        let dir = make_directory();
        let db_plugins = dir.list_by_category(&PluginCategory::Database);
        assert_eq!(db_plugins.len(), 1);
        assert_eq!(db_plugins[0].id, "pg-client");
    }

    #[test]
    fn test_get_plugin() {
        let dir = make_directory();
        assert!(dir.get_plugin("pg-client").is_some());
        assert!(dir.get_plugin("nonexistent").is_none());
    }

    #[test]
    fn test_top_rated() {
        let mut dir = make_directory();
        if let Some(p) = dir.plugins.get_mut("eslint-runner") {
            p.rating = 4.8;
        }
        if let Some(p) = dir.plugins.get_mut("pg-client") {
            p.rating = 4.2;
        }
        let top = dir.top_rated(2);
        assert_eq!(top.len(), 2);
        assert!(top[0].rating >= top[1].rating);
    }

    #[test]
    fn test_recently_updated() {
        let mut dir = make_directory();
        if let Some(p) = dir.plugins.get_mut("eslint-runner") {
            p.updated_at = 9999;
        }
        let recent = dir.recently_updated(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "eslint-runner");
    }

    #[test]
    fn test_set_verification_status() {
        let mut dir = make_directory();
        dir.set_verification_status("pg-client", VerificationStatus::Pending).unwrap();
        assert_eq!(
            dir.get_plugin("pg-client").unwrap().verification,
            VerificationStatus::Pending
        );
    }

    #[test]
    fn test_set_verification_status_rejected() {
        let mut dir = make_directory();
        let status = VerificationStatus::Rejected {
            reason: "Malicious code detected".to_string(),
        };
        dir.set_verification_status("pg-client", status.clone()).unwrap();
        assert_eq!(dir.get_plugin("pg-client").unwrap().verification, status);
    }

    #[test]
    fn test_set_verification_status_not_found() {
        let mut dir = make_directory();
        let result = dir.set_verification_status("nonexistent", VerificationStatus::Verified);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_relevance_name_exact_match() {
        let mut dir = PluginDirectory::new(DirectoryConfig { allow_unverified: true, ..DirectoryConfig::default() });
        dir.add_plugin(make_plugin("exact", "mytools", PluginCategory::Utility)).unwrap();
        dir.add_plugin(make_plugin("partial", "mytools-extra", PluginCategory::Utility)).unwrap();
        let results = dir.search("mytools", None, false, 10);
        assert!(results.len() >= 2);
        // Exact match should rank higher
        assert_eq!(results[0].plugin_id, "exact");
    }

    #[test]
    fn test_remove_plugin_cleans_reviews() {
        let mut dir = make_directory();
        dir.add_review(PluginReview {
            plugin_id: "pg-client".to_string(),
            reviewer: "alice".to_string(),
            rating: 5,
            comment: "Good".to_string(),
            timestamp: 1000,
        })
        .unwrap();
        assert_eq!(dir.get_reviews("pg-client").len(), 1);
        dir.remove_plugin("pg-client");
        assert_eq!(dir.get_reviews("pg-client").len(), 0);
    }

    #[test]
    fn test_category_as_str() {
        assert_eq!(PluginCategory::DataSource.as_str(), "data_source");
        assert_eq!(PluginCategory::DevOps.as_str(), "devops");
        assert_eq!(PluginCategory::Security.as_str(), "security");
    }

    #[test]
    fn test_default_directory_config() {
        let config = DirectoryConfig::default();
        assert!(!config.allow_unverified);
        assert!(config.auto_update);
        assert_eq!(config.max_installed, 50);
    }

    #[test]
    fn test_plugin_permissions() {
        let p = make_plugin("sec", "Sec Tool", PluginCategory::Security);
        assert_eq!(p.permissions, vec![PluginPermission::FileRead]);
    }

    #[test]
    fn test_search_no_results() {
        let dir = make_directory();
        let results = dir.search("zzzznonexistent", None, false, 10);
        assert!(results.is_empty());
    }
}
