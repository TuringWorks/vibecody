//! Organization-wide cross-repository context engine for VibeCody.
//!
//! Provides org-wide intelligence by indexing multiple repositories,
//! detecting shared patterns and conventions, mapping cross-repo
//! dependencies, and enabling unified search across the organization.
//!
//! Closes the gap vs Tabnine Enterprise Context Engine and Augment's
//! org-wide understanding.
//!
//! # Architecture
//!
//! ```text
//! OrgContextEngine
//!   ├─ OrgConfig (scope, repos, index settings)
//!   ├─ patterns: Vec<OrgPattern>        ─ detected code patterns
//!   ├─ conventions: Vec<Convention>      ─ org-wide conventions
//!   └─ dependencies: Vec<CrossRepoDependency> ─ inter-repo deps
//! ```
//!
//! # Configuration
//!
//! ```toml
//! [org_context]
//! scope = "Organization"
//! index_dir = ".vibecody/org-index"
//! auto_reindex = true
//! reindex_interval_secs = 3600
//! max_repos = 50
//!
//! [[org_context.repos]]
//! name = "backend"
//! path = "/home/user/repos/backend"
//! branch = "main"
//! enabled = true
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur in the org context engine.
#[derive(Debug, Clone, PartialEq)]
pub enum OrgError {
    RepoNotFound(String),
    IndexError(String),
    MaxReposExceeded(usize),
    DuplicateRepo(String),
    PatternNotFound(String),
    ConventionNotFound(String),
    SearchError(String),
    ConfigError(String),
}

impl std::fmt::Display for OrgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RepoNotFound(name) => write!(f, "repository not found: {name}"),
            Self::IndexError(msg) => write!(f, "index error: {msg}"),
            Self::MaxReposExceeded(max) => {
                write!(f, "maximum number of repos exceeded: {max}")
            }
            Self::DuplicateRepo(name) => write!(f, "duplicate repository: {name}"),
            Self::PatternNotFound(id) => write!(f, "pattern not found: {id}"),
            Self::ConventionNotFound(id) => write!(f, "convention not found: {id}"),
            Self::SearchError(msg) => write!(f, "search error: {msg}"),
            Self::ConfigError(msg) => write!(f, "config error: {msg}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, OrgError>;

// ---------------------------------------------------------------------------
// Scope
// ---------------------------------------------------------------------------

/// The breadth of the context engine's scope.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextScope {
    Organization,
    Team,
    ProjectGroup,
}

impl ContextScope {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Organization => "organization",
            Self::Team => "team",
            Self::ProjectGroup => "project_group",
        }
    }
}

// ---------------------------------------------------------------------------
// Repository configuration
// ---------------------------------------------------------------------------

/// Configuration for a single repository within the org scope.
#[derive(Debug, Clone, PartialEq)]
pub struct RepoConfig {
    pub name: String,
    pub path: String,
    pub remote_url: Option<String>,
    pub branch: String,
    pub enabled: bool,
}

impl RepoConfig {
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            remote_url: None,
            branch: "main".to_string(),
            enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Org configuration
// ---------------------------------------------------------------------------

/// Top-level configuration for the org context engine.
#[derive(Debug, Clone)]
pub struct OrgConfig {
    pub scope: ContextScope,
    pub repos: Vec<RepoConfig>,
    pub index_dir: String,
    pub auto_reindex: bool,
    pub reindex_interval_secs: u64,
    pub max_repos: usize,
}

impl Default for OrgConfig {
    fn default() -> Self {
        Self {
            scope: ContextScope::Organization,
            repos: Vec::new(),
            index_dir: ".vibecody/org-index".to_string(),
            auto_reindex: true,
            reindex_interval_secs: 3600,
            max_repos: 50,
        }
    }
}

// ---------------------------------------------------------------------------
// Pattern types
// ---------------------------------------------------------------------------

/// The category of a detected code pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    ArchitecturePattern,
    NamingConvention,
    ErrorHandling,
    TestingPattern,
    ApiDesign,
    DataAccess,
    SecurityPractice,
    BuildConfiguration,
    DependencyManagement,
    CodeOrganization,
}

impl PatternType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ArchitecturePattern => "architecture_pattern",
            Self::NamingConvention => "naming_convention",
            Self::ErrorHandling => "error_handling",
            Self::TestingPattern => "testing_pattern",
            Self::ApiDesign => "api_design",
            Self::DataAccess => "data_access",
            Self::SecurityPractice => "security_practice",
            Self::BuildConfiguration => "build_configuration",
            Self::DependencyManagement => "dependency_management",
            Self::CodeOrganization => "code_organization",
        }
    }
}

/// A single occurrence of a pattern in a specific file.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternOccurrence {
    pub repo_name: String,
    pub file_path: String,
    pub line_number: Option<u32>,
    pub snippet: String,
}

/// A detected code pattern across the organisation.
#[derive(Debug, Clone)]
pub struct OrgPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub pattern_type: PatternType,
    pub occurrences: Vec<PatternOccurrence>,
    pub confidence: f32,
    pub first_seen: u64,
    pub last_seen: u64,
}

impl OrgPattern {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        pattern_type: PatternType,
    ) -> Self {
        let ts = now_secs();
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            pattern_type,
            occurrences: Vec::new(),
            confidence: 0.0,
            first_seen: ts,
            last_seen: ts,
        }
    }
}

// ---------------------------------------------------------------------------
// Conventions
// ---------------------------------------------------------------------------

/// The category a convention belongs to.
#[derive(Debug, Clone, PartialEq)]
pub enum ConventionCategory {
    Naming,
    FileStructure,
    ErrorHandling,
    Testing,
    Documentation,
    Versioning,
    Branching,
    Ci,
    Security,
    Dependency,
}

impl ConventionCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Naming => "naming",
            Self::FileStructure => "file_structure",
            Self::ErrorHandling => "error_handling",
            Self::Testing => "testing",
            Self::Documentation => "documentation",
            Self::Versioning => "versioning",
            Self::Branching => "branching",
            Self::Ci => "ci",
            Self::Security => "security",
            Self::Dependency => "dependency",
        }
    }
}

/// An organisation-wide convention detected or declared.
#[derive(Debug, Clone)]
pub struct Convention {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ConventionCategory,
    pub examples: Vec<String>,
    pub repos_using: Vec<String>,
    pub adoption_rate: f32,
}

impl Convention {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: ConventionCategory,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category,
            examples: Vec::new(),
            repos_using: Vec::new(),
            adoption_rate: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-repo dependencies
// ---------------------------------------------------------------------------

/// The type of dependency between two repositories.
#[derive(Debug, Clone, PartialEq)]
pub enum DepType {
    DirectImport,
    SharedLibrary,
    ApiCall,
    SharedDatabase,
    SharedConfig,
}

impl DepType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::DirectImport => "direct_import",
            Self::SharedLibrary => "shared_library",
            Self::ApiCall => "api_call",
            Self::SharedDatabase => "shared_database",
            Self::SharedConfig => "shared_config",
        }
    }
}

/// A dependency relationship between two repositories.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossRepoDependency {
    pub from_repo: String,
    pub to_repo: String,
    pub dependency_type: DepType,
    pub artifact: String,
    pub version: Option<String>,
}

impl CrossRepoDependency {
    pub fn new(
        from_repo: impl Into<String>,
        to_repo: impl Into<String>,
        dependency_type: DepType,
        artifact: impl Into<String>,
    ) -> Self {
        Self {
            from_repo: from_repo.into(),
            to_repo: to_repo.into(),
            dependency_type,
            artifact: artifact.into(),
            version: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Search types
// ---------------------------------------------------------------------------

/// A single match within an org-wide search.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub repo_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub snippet: String,
    pub relevance_score: f32,
}

/// Results from an org-wide search.
#[derive(Debug, Clone)]
pub struct OrgSearchResult {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub total_repos_searched: usize,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Index status
// ---------------------------------------------------------------------------

/// Snapshot of the current indexing state.
#[derive(Debug, Clone)]
pub struct IndexStatus {
    pub repos_indexed: usize,
    pub total_repos: usize,
    pub patterns_detected: usize,
    pub conventions_detected: usize,
    pub dependencies_mapped: usize,
    pub last_indexed: Option<u64>,
    pub index_size_bytes: u64,
}

// ---------------------------------------------------------------------------
// OrgContextEngine
// ---------------------------------------------------------------------------

/// Organisation-wide cross-repository context engine.
///
/// Indexes multiple repositories, detects shared patterns and conventions,
/// maps cross-repo dependencies, and provides unified search.
pub struct OrgContextEngine {
    config: OrgConfig,
    repositories: Vec<RepoConfig>,
    patterns: Vec<OrgPattern>,
    conventions: Vec<Convention>,
    dependencies: Vec<CrossRepoDependency>,
    last_indexed: Option<u64>,
}

impl OrgContextEngine {
    /// Create a new engine with the given configuration.
    pub fn new(config: OrgConfig) -> Self {
        let repositories = config.repos.clone();
        Self {
            config,
            repositories,
            patterns: Vec::new(),
            conventions: Vec::new(),
            dependencies: Vec::new(),
            last_indexed: None,
        }
    }

    // -- Repo management ---------------------------------------------------

    /// Add a repository to the engine's scope.
    pub fn add_repo(&mut self, repo: RepoConfig) -> Result<()> {
        if self.repositories.len() >= self.config.max_repos {
            return Err(OrgError::MaxReposExceeded(self.config.max_repos));
        }
        if self.repositories.iter().any(|r| r.name == repo.name) {
            return Err(OrgError::DuplicateRepo(repo.name.clone()));
        }
        self.repositories.push(repo);
        Ok(())
    }

    /// Remove a repository by name.
    pub fn remove_repo(&mut self, name: &str) -> Result<()> {
        let idx = self
            .repositories
            .iter()
            .position(|r| r.name == name)
            .ok_or_else(|| OrgError::RepoNotFound(name.to_string()))?;
        self.repositories.remove(idx);
        Ok(())
    }

    /// List all configured repositories.
    pub fn list_repos(&self) -> Vec<&RepoConfig> {
        self.repositories.iter().collect()
    }

    // -- Pattern management ------------------------------------------------

    /// Register a pattern.
    pub fn add_pattern(&mut self, pattern: OrgPattern) -> Result<()> {
        if self.patterns.iter().any(|p| p.id == pattern.id) {
            return Err(OrgError::ConfigError(format!(
                "pattern already exists: {}",
                pattern.id
            )));
        }
        self.patterns.push(pattern);
        Ok(())
    }

    /// Analyse repositories and return detected patterns.
    ///
    /// In a full implementation this would scan source files; here it
    /// returns the currently registered patterns as a starting point.
    pub fn detect_patterns(&self) -> Vec<OrgPattern> {
        self.patterns.clone()
    }

    /// Return all registered patterns.
    pub fn get_patterns(&self) -> Vec<&OrgPattern> {
        self.patterns.iter().collect()
    }

    /// Return patterns filtered by type.
    pub fn get_patterns_by_type(&self, pattern_type: &PatternType) -> Vec<&OrgPattern> {
        self.patterns
            .iter()
            .filter(|p| &p.pattern_type == pattern_type)
            .collect()
    }

    // -- Convention management ---------------------------------------------

    /// Register a convention.
    pub fn add_convention(&mut self, convention: Convention) -> Result<()> {
        if self.conventions.iter().any(|c| c.id == convention.id) {
            return Err(OrgError::ConfigError(format!(
                "convention already exists: {}",
                convention.id
            )));
        }
        self.conventions.push(convention);
        Ok(())
    }

    /// Return all registered conventions.
    pub fn get_conventions(&self) -> Vec<&Convention> {
        self.conventions.iter().collect()
    }

    /// Return conventions filtered by category.
    pub fn get_conventions_by_category(
        &self,
        category: &ConventionCategory,
    ) -> Vec<&Convention> {
        self.conventions
            .iter()
            .filter(|c| &c.category == category)
            .collect()
    }

    // -- Dependency management ---------------------------------------------

    /// Register a cross-repo dependency.
    pub fn add_dependency(&mut self, dep: CrossRepoDependency) -> Result<()> {
        self.dependencies.push(dep);
        Ok(())
    }

    /// Return all registered dependencies.
    pub fn get_dependencies(&self) -> Vec<&CrossRepoDependency> {
        self.dependencies.iter().collect()
    }

    /// Return dependencies involving a specific repository (as source or target).
    pub fn get_repo_dependencies(&self, repo_name: &str) -> Vec<&CrossRepoDependency> {
        self.dependencies
            .iter()
            .filter(|d| d.from_repo == repo_name || d.to_repo == repo_name)
            .collect()
    }

    // -- Search ------------------------------------------------------------

    /// Search across all enabled repositories.
    ///
    /// The current implementation performs a simple substring match against
    /// registered pattern and convention data.  A production build would
    /// integrate with the full-text / vector index.
    pub fn search(&self, query: &str) -> OrgSearchResult {
        let start = now_ms();
        let mut matches = Vec::new();

        // Search pattern occurrences.
        for pattern in &self.patterns {
            for occ in &pattern.occurrences {
                if occ.snippet.contains(query) || pattern.name.contains(query) {
                    matches.push(SearchMatch {
                        repo_name: occ.repo_name.clone(),
                        file_path: occ.file_path.clone(),
                        line_number: occ.line_number.unwrap_or(0),
                        snippet: occ.snippet.clone(),
                        relevance_score: pattern.confidence,
                    });
                }
            }
        }

        let enabled_repos = self.repositories.iter().filter(|r| r.enabled).count();
        let duration = now_ms().saturating_sub(start);

        OrgSearchResult {
            query: query.to_string(),
            matches,
            total_repos_searched: enabled_repos,
            duration_ms: duration,
        }
    }

    // -- Index status ------------------------------------------------------

    /// Return a snapshot of the current index status.
    pub fn get_index_status(&self) -> IndexStatus {
        IndexStatus {
            repos_indexed: self.repositories.iter().filter(|r| r.enabled).count(),
            total_repos: self.repositories.len(),
            patterns_detected: self.patterns.len(),
            conventions_detected: self.conventions.len(),
            dependencies_mapped: self.dependencies.len(),
            last_indexed: self.last_indexed,
            index_size_bytes: 0,
        }
    }

    // -- Analytics ---------------------------------------------------------

    /// Get the adoption rate for a convention by id.
    pub fn get_adoption_rate(&self, convention_id: &str) -> Option<f32> {
        self.conventions
            .iter()
            .find(|c| c.id == convention_id)
            .map(|c| c.adoption_rate)
    }

    /// Return the most common patterns ordered by occurrence count (descending).
    pub fn get_most_common_patterns(&self, limit: usize) -> Vec<&OrgPattern> {
        let mut sorted: Vec<&OrgPattern> = self.patterns.iter().collect();
        sorted.sort_by(|a, b| b.occurrences.len().cmp(&a.occurrences.len()));
        sorted.truncate(limit);
        sorted
    }

    /// Suggest conventions that a repository has not yet adopted.
    pub fn suggest_conventions_for_repo(&self, repo_name: &str) -> Vec<&Convention> {
        self.conventions
            .iter()
            .filter(|c| !c.repos_using.iter().any(|r| r == repo_name))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- helpers -----------------------------------------------------------

    fn default_engine() -> OrgContextEngine {
        OrgContextEngine::new(OrgConfig::default())
    }

    fn sample_repo(name: &str) -> RepoConfig {
        RepoConfig::new(name, format!("/repos/{name}"))
    }

    fn sample_pattern(id: &str, ptype: PatternType) -> OrgPattern {
        OrgPattern::new(id, format!("Pattern {id}"), "A test pattern", ptype)
    }

    fn sample_convention(id: &str, cat: ConventionCategory) -> Convention {
        Convention::new(id, format!("Convention {id}"), "A test convention", cat)
    }

    // -- config defaults ---------------------------------------------------

    #[test]
    fn test_org_config_defaults() {
        let cfg = OrgConfig::default();
        assert_eq!(cfg.scope, ContextScope::Organization);
        assert!(cfg.repos.is_empty());
        assert_eq!(cfg.index_dir, ".vibecody/org-index");
        assert!(cfg.auto_reindex);
        assert_eq!(cfg.reindex_interval_secs, 3600);
        assert_eq!(cfg.max_repos, 50);
    }

    #[test]
    fn test_context_scope_as_str() {
        assert_eq!(ContextScope::Organization.as_str(), "organization");
        assert_eq!(ContextScope::Team.as_str(), "team");
        assert_eq!(ContextScope::ProjectGroup.as_str(), "project_group");
    }

    #[test]
    fn test_scope_team_config() {
        let cfg = OrgConfig {
            scope: ContextScope::Team,
            ..OrgConfig::default()
        };
        let engine = OrgContextEngine::new(cfg);
        assert_eq!(engine.config.scope, ContextScope::Team);
    }

    #[test]
    fn test_scope_project_group_config() {
        let cfg = OrgConfig {
            scope: ContextScope::ProjectGroup,
            ..OrgConfig::default()
        };
        let engine = OrgContextEngine::new(cfg);
        assert_eq!(engine.config.scope, ContextScope::ProjectGroup);
    }

    // -- repo CRUD ---------------------------------------------------------

    #[test]
    fn test_add_repo() {
        let mut engine = default_engine();
        engine.add_repo(sample_repo("backend")).unwrap();
        assert_eq!(engine.list_repos().len(), 1);
        assert_eq!(engine.list_repos()[0].name, "backend");
    }

    #[test]
    fn test_add_multiple_repos() {
        let mut engine = default_engine();
        engine.add_repo(sample_repo("backend")).unwrap();
        engine.add_repo(sample_repo("frontend")).unwrap();
        engine.add_repo(sample_repo("infra")).unwrap();
        assert_eq!(engine.list_repos().len(), 3);
    }

    #[test]
    fn test_add_repo_duplicate() {
        let mut engine = default_engine();
        engine.add_repo(sample_repo("backend")).unwrap();
        let err = engine.add_repo(sample_repo("backend")).unwrap_err();
        assert_eq!(err, OrgError::DuplicateRepo("backend".to_string()));
    }

    #[test]
    fn test_add_repo_max_exceeded() {
        let cfg = OrgConfig {
            max_repos: 2,
            ..OrgConfig::default()
        };
        let mut engine = OrgContextEngine::new(cfg);
        engine.add_repo(sample_repo("a")).unwrap();
        engine.add_repo(sample_repo("b")).unwrap();
        let err = engine.add_repo(sample_repo("c")).unwrap_err();
        assert_eq!(err, OrgError::MaxReposExceeded(2));
    }

    #[test]
    fn test_remove_repo() {
        let mut engine = default_engine();
        engine.add_repo(sample_repo("backend")).unwrap();
        engine.remove_repo("backend").unwrap();
        assert!(engine.list_repos().is_empty());
    }

    #[test]
    fn test_remove_repo_not_found() {
        let mut engine = default_engine();
        let err = engine.remove_repo("nonexistent").unwrap_err();
        assert_eq!(err, OrgError::RepoNotFound("nonexistent".to_string()));
    }

    #[test]
    fn test_list_repos_empty() {
        let engine = default_engine();
        assert!(engine.list_repos().is_empty());
    }

    #[test]
    fn test_repo_config_defaults() {
        let repo = RepoConfig::new("test", "/repos/test");
        assert_eq!(repo.branch, "main");
        assert!(repo.enabled);
        assert!(repo.remote_url.is_none());
    }

    #[test]
    fn test_repos_from_config() {
        let cfg = OrgConfig {
            repos: vec![sample_repo("initial")],
            ..OrgConfig::default()
        };
        let engine = OrgContextEngine::new(cfg);
        assert_eq!(engine.list_repos().len(), 1);
        assert_eq!(engine.list_repos()[0].name, "initial");
    }

    // -- pattern CRUD ------------------------------------------------------

    #[test]
    fn test_add_pattern() {
        let mut engine = default_engine();
        let p = sample_pattern("p1", PatternType::ArchitecturePattern);
        engine.add_pattern(p).unwrap();
        assert_eq!(engine.get_patterns().len(), 1);
    }

    #[test]
    fn test_add_pattern_duplicate_id() {
        let mut engine = default_engine();
        engine
            .add_pattern(sample_pattern("p1", PatternType::ApiDesign))
            .unwrap();
        let err = engine
            .add_pattern(sample_pattern("p1", PatternType::ApiDesign))
            .unwrap_err();
        assert!(matches!(err, OrgError::ConfigError(_)));
    }

    #[test]
    fn test_get_patterns_by_type() {
        let mut engine = default_engine();
        engine
            .add_pattern(sample_pattern("p1", PatternType::ErrorHandling))
            .unwrap();
        engine
            .add_pattern(sample_pattern("p2", PatternType::TestingPattern))
            .unwrap();
        engine
            .add_pattern(sample_pattern("p3", PatternType::ErrorHandling))
            .unwrap();
        let eh = engine.get_patterns_by_type(&PatternType::ErrorHandling);
        assert_eq!(eh.len(), 2);
        let tp = engine.get_patterns_by_type(&PatternType::TestingPattern);
        assert_eq!(tp.len(), 1);
    }

    #[test]
    fn test_get_patterns_by_type_empty() {
        let engine = default_engine();
        assert!(engine
            .get_patterns_by_type(&PatternType::SecurityPractice)
            .is_empty());
    }

    #[test]
    fn test_detect_patterns_returns_registered() {
        let mut engine = default_engine();
        engine
            .add_pattern(sample_pattern("p1", PatternType::DataAccess))
            .unwrap();
        let detected = engine.detect_patterns();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].id, "p1");
    }

    #[test]
    fn test_pattern_type_as_str() {
        assert_eq!(PatternType::ArchitecturePattern.as_str(), "architecture_pattern");
        assert_eq!(PatternType::NamingConvention.as_str(), "naming_convention");
        assert_eq!(PatternType::BuildConfiguration.as_str(), "build_configuration");
        assert_eq!(PatternType::CodeOrganization.as_str(), "code_organization");
    }

    // -- convention CRUD ---------------------------------------------------

    #[test]
    fn test_add_convention() {
        let mut engine = default_engine();
        engine
            .add_convention(sample_convention("c1", ConventionCategory::Naming))
            .unwrap();
        assert_eq!(engine.get_conventions().len(), 1);
    }

    #[test]
    fn test_add_convention_duplicate() {
        let mut engine = default_engine();
        engine
            .add_convention(sample_convention("c1", ConventionCategory::Naming))
            .unwrap();
        let err = engine
            .add_convention(sample_convention("c1", ConventionCategory::Naming))
            .unwrap_err();
        assert!(matches!(err, OrgError::ConfigError(_)));
    }

    #[test]
    fn test_get_conventions_by_category() {
        let mut engine = default_engine();
        engine
            .add_convention(sample_convention("c1", ConventionCategory::Testing))
            .unwrap();
        engine
            .add_convention(sample_convention("c2", ConventionCategory::Security))
            .unwrap();
        engine
            .add_convention(sample_convention("c3", ConventionCategory::Testing))
            .unwrap();
        let testing = engine.get_conventions_by_category(&ConventionCategory::Testing);
        assert_eq!(testing.len(), 2);
    }

    #[test]
    fn test_get_conventions_by_category_empty() {
        let engine = default_engine();
        assert!(engine
            .get_conventions_by_category(&ConventionCategory::Ci)
            .is_empty());
    }

    #[test]
    fn test_convention_category_as_str() {
        assert_eq!(ConventionCategory::Naming.as_str(), "naming");
        assert_eq!(ConventionCategory::FileStructure.as_str(), "file_structure");
        assert_eq!(ConventionCategory::Ci.as_str(), "ci");
        assert_eq!(ConventionCategory::Dependency.as_str(), "dependency");
    }

    // -- dependency CRUD ---------------------------------------------------

    #[test]
    fn test_add_dependency() {
        let mut engine = default_engine();
        let dep = CrossRepoDependency::new("a", "b", DepType::SharedLibrary, "utils-lib");
        engine.add_dependency(dep).unwrap();
        assert_eq!(engine.get_dependencies().len(), 1);
    }

    #[test]
    fn test_get_repo_dependencies() {
        let mut engine = default_engine();
        engine
            .add_dependency(CrossRepoDependency::new(
                "backend",
                "shared",
                DepType::DirectImport,
                "shared-types",
            ))
            .unwrap();
        engine
            .add_dependency(CrossRepoDependency::new(
                "frontend",
                "shared",
                DepType::DirectImport,
                "shared-types",
            ))
            .unwrap();
        engine
            .add_dependency(CrossRepoDependency::new(
                "frontend",
                "api",
                DepType::ApiCall,
                "/v1/users",
            ))
            .unwrap();

        let shared_deps = engine.get_repo_dependencies("shared");
        assert_eq!(shared_deps.len(), 2);

        let frontend_deps = engine.get_repo_dependencies("frontend");
        assert_eq!(frontend_deps.len(), 2);

        let unknown = engine.get_repo_dependencies("unknown");
        assert!(unknown.is_empty());
    }

    #[test]
    fn test_dep_type_as_str() {
        assert_eq!(DepType::DirectImport.as_str(), "direct_import");
        assert_eq!(DepType::SharedLibrary.as_str(), "shared_library");
        assert_eq!(DepType::ApiCall.as_str(), "api_call");
        assert_eq!(DepType::SharedDatabase.as_str(), "shared_database");
        assert_eq!(DepType::SharedConfig.as_str(), "shared_config");
    }

    // -- search ------------------------------------------------------------

    #[test]
    fn test_search_basic() {
        let mut engine = default_engine();
        engine.add_repo(sample_repo("backend")).unwrap();

        let mut pattern = sample_pattern("p1", PatternType::ApiDesign);
        pattern.confidence = 0.9;
        pattern.occurrences.push(PatternOccurrence {
            repo_name: "backend".to_string(),
            file_path: "src/api.rs".to_string(),
            line_number: Some(42),
            snippet: "pub fn create_user(req: Request) -> Response".to_string(),
        });
        engine.add_pattern(pattern).unwrap();

        let result = engine.search("create_user");
        assert_eq!(result.query, "create_user");
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].repo_name, "backend");
        assert_eq!(result.matches[0].line_number, 42);
        assert_eq!(result.total_repos_searched, 1);
    }

    #[test]
    fn test_search_no_results() {
        let engine = default_engine();
        let result = engine.search("nonexistent_query");
        assert!(result.matches.is_empty());
        assert_eq!(result.total_repos_searched, 0);
    }

    #[test]
    fn test_search_by_pattern_name() {
        let mut engine = default_engine();
        engine.add_repo(sample_repo("infra")).unwrap();

        let mut pattern = OrgPattern::new(
            "p1",
            "Repository Pattern",
            "Data access via repository",
            PatternType::DataAccess,
        );
        pattern.occurrences.push(PatternOccurrence {
            repo_name: "infra".to_string(),
            file_path: "src/repo.rs".to_string(),
            line_number: None,
            snippet: "impl UserRepository".to_string(),
        });
        engine.add_pattern(pattern).unwrap();

        let result = engine.search("Repository Pattern");
        assert_eq!(result.matches.len(), 1);
    }

    // -- index status ------------------------------------------------------

    #[test]
    fn test_index_status_empty() {
        let engine = default_engine();
        let status = engine.get_index_status();
        assert_eq!(status.repos_indexed, 0);
        assert_eq!(status.total_repos, 0);
        assert_eq!(status.patterns_detected, 0);
        assert_eq!(status.conventions_detected, 0);
        assert_eq!(status.dependencies_mapped, 0);
        assert!(status.last_indexed.is_none());
    }

    #[test]
    fn test_index_status_populated() {
        let mut engine = default_engine();
        engine.add_repo(sample_repo("a")).unwrap();
        engine.add_repo(sample_repo("b")).unwrap();
        engine
            .add_pattern(sample_pattern("p1", PatternType::ErrorHandling))
            .unwrap();
        engine
            .add_convention(sample_convention("c1", ConventionCategory::Testing))
            .unwrap();
        engine
            .add_dependency(CrossRepoDependency::new(
                "a",
                "b",
                DepType::ApiCall,
                "/health",
            ))
            .unwrap();

        let status = engine.get_index_status();
        assert_eq!(status.repos_indexed, 2);
        assert_eq!(status.total_repos, 2);
        assert_eq!(status.patterns_detected, 1);
        assert_eq!(status.conventions_detected, 1);
        assert_eq!(status.dependencies_mapped, 1);
    }

    #[test]
    fn test_index_status_disabled_repo() {
        let mut engine = default_engine();
        let mut repo = sample_repo("disabled");
        repo.enabled = false;
        engine.add_repo(repo).unwrap();
        engine.add_repo(sample_repo("enabled")).unwrap();

        let status = engine.get_index_status();
        assert_eq!(status.repos_indexed, 1);
        assert_eq!(status.total_repos, 2);
    }

    // -- adoption rate -----------------------------------------------------

    #[test]
    fn test_adoption_rate() {
        let mut engine = default_engine();
        let mut conv = sample_convention("c1", ConventionCategory::Naming);
        conv.adoption_rate = 0.75;
        engine.add_convention(conv).unwrap();

        assert_eq!(engine.get_adoption_rate("c1"), Some(0.75));
    }

    #[test]
    fn test_adoption_rate_not_found() {
        let engine = default_engine();
        assert_eq!(engine.get_adoption_rate("missing"), None);
    }

    // -- most common patterns ----------------------------------------------

    #[test]
    fn test_most_common_patterns() {
        let mut engine = default_engine();

        let mut p1 = sample_pattern("p1", PatternType::ErrorHandling);
        p1.occurrences.push(PatternOccurrence {
            repo_name: "a".to_string(),
            file_path: "f1".to_string(),
            line_number: None,
            snippet: "err".to_string(),
        });

        let mut p2 = sample_pattern("p2", PatternType::TestingPattern);
        p2.occurrences.push(PatternOccurrence {
            repo_name: "a".to_string(),
            file_path: "f2".to_string(),
            line_number: None,
            snippet: "test".to_string(),
        });
        p2.occurrences.push(PatternOccurrence {
            repo_name: "b".to_string(),
            file_path: "f3".to_string(),
            line_number: None,
            snippet: "test2".to_string(),
        });
        p2.occurrences.push(PatternOccurrence {
            repo_name: "c".to_string(),
            file_path: "f4".to_string(),
            line_number: None,
            snippet: "test3".to_string(),
        });

        let p3 = sample_pattern("p3", PatternType::ApiDesign);

        engine.add_pattern(p1).unwrap();
        engine.add_pattern(p2).unwrap();
        engine.add_pattern(p3).unwrap();

        let top = engine.get_most_common_patterns(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].id, "p2"); // 3 occurrences
        assert_eq!(top[1].id, "p1"); // 1 occurrence
    }

    #[test]
    fn test_most_common_patterns_limit() {
        let mut engine = default_engine();
        engine
            .add_pattern(sample_pattern("p1", PatternType::ApiDesign))
            .unwrap();
        engine
            .add_pattern(sample_pattern("p2", PatternType::ApiDesign))
            .unwrap();
        engine
            .add_pattern(sample_pattern("p3", PatternType::ApiDesign))
            .unwrap();

        let top = engine.get_most_common_patterns(1);
        assert_eq!(top.len(), 1);
    }

    // -- convention suggestions --------------------------------------------

    #[test]
    fn test_suggest_conventions_for_repo() {
        let mut engine = default_engine();

        let mut c1 = sample_convention("c1", ConventionCategory::Naming);
        c1.repos_using = vec!["backend".to_string()];
        let mut c2 = sample_convention("c2", ConventionCategory::Testing);
        c2.repos_using = vec!["backend".to_string(), "frontend".to_string()];
        let c3 = sample_convention("c3", ConventionCategory::Security);

        engine.add_convention(c1).unwrap();
        engine.add_convention(c2).unwrap();
        engine.add_convention(c3).unwrap();

        // frontend doesn't use c1 or c3
        let suggestions = engine.suggest_conventions_for_repo("frontend");
        assert_eq!(suggestions.len(), 2);
        let ids: Vec<&str> = suggestions.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"c1"));
        assert!(ids.contains(&"c3"));
    }

    #[test]
    fn test_suggest_conventions_all_adopted() {
        let mut engine = default_engine();
        let mut c = sample_convention("c1", ConventionCategory::Naming);
        c.repos_using = vec!["backend".to_string()];
        engine.add_convention(c).unwrap();

        let suggestions = engine.suggest_conventions_for_repo("backend");
        assert!(suggestions.is_empty());
    }

    // -- error display -----------------------------------------------------

    #[test]
    fn test_error_display() {
        let err = OrgError::RepoNotFound("test".to_string());
        assert_eq!(format!("{err}"), "repository not found: test");

        let err = OrgError::MaxReposExceeded(50);
        assert_eq!(
            format!("{err}"),
            "maximum number of repos exceeded: 50"
        );

        let err = OrgError::DuplicateRepo("dup".to_string());
        assert_eq!(format!("{err}"), "duplicate repository: dup");
    }

    // -- cross-repo dependency with version --------------------------------

    #[test]
    fn test_dependency_with_version() {
        let mut dep = CrossRepoDependency::new("a", "b", DepType::SharedLibrary, "core-lib");
        dep.version = Some("1.2.3".to_string());
        assert_eq!(dep.version.as_deref(), Some("1.2.3"));
    }

    // -- multiple repos with patterns across them --------------------------

    #[test]
    fn test_multiple_repos_with_shared_patterns() {
        let mut engine = default_engine();
        engine.add_repo(sample_repo("svc-a")).unwrap();
        engine.add_repo(sample_repo("svc-b")).unwrap();
        engine.add_repo(sample_repo("svc-c")).unwrap();

        let mut p = sample_pattern("shared-err", PatternType::ErrorHandling);
        p.confidence = 0.85;
        p.occurrences = vec![
            PatternOccurrence {
                repo_name: "svc-a".to_string(),
                file_path: "src/error.rs".to_string(),
                line_number: Some(10),
                snippet: "anyhow::Result".to_string(),
            },
            PatternOccurrence {
                repo_name: "svc-b".to_string(),
                file_path: "src/error.rs".to_string(),
                line_number: Some(5),
                snippet: "anyhow::Result".to_string(),
            },
        ];
        engine.add_pattern(p).unwrap();

        let results = engine.search("anyhow");
        assert_eq!(results.matches.len(), 2);
        assert_eq!(results.total_repos_searched, 3);
    }
}
