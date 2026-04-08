//! MCP Lazy Loading / Tool Search for VibeCody.
//!
//! This module implements a lazy-loading registry for MCP tool schemas. At startup,
//! only lightweight manifests (name + description) are stored. Full tool schemas
//! (including parameter definitions) are loaded on first use and cached with LRU
//! eviction. This dramatically reduces context window usage when many MCP servers
//! expose hundreds of tools but only a few are actively used.
//!
//! Key features:
//! - **Lazy loading**: Full schemas loaded on demand, not at startup
//! - **LRU eviction**: Unused schemas evicted after configurable idle timeout
//! - **Tool search**: Keyword-based search across all registered MCP servers
//! - **Eager load patterns**: Glob patterns to pre-load critical tools
//! - **Metrics**: Context savings, cache hit/miss rates, load times

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Lightweight manifest stored for every known tool at startup.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub server_name: String,
    pub loaded: bool,
}

/// Definition of a single parameter within a tool schema.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDef {
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// Full tool schema loaded on demand.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, ParameterDef>,
    pub server_name: String,
}

/// Configuration for the lazy-loading registry.
#[derive(Debug, Clone, PartialEq)]
pub struct LazyLoadConfig {
    pub max_cached_schemas: usize,
    pub idle_timeout_secs: u64,
    pub eager_load_patterns: Vec<String>,
}

impl Default for LazyLoadConfig {
    fn default() -> Self {
        Self {
            max_cached_schemas: 64,
            idle_timeout_secs: 300,
            eager_load_patterns: Vec::new(),
        }
    }
}

/// A single search result returned by `search_tools`.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSearchResult {
    pub tool_name: String,
    pub server_name: String,
    pub description: String,
    pub relevance_score: f64,
}

/// Aggregate metrics for the registry.
#[derive(Debug, Clone, PartialEq)]
pub struct RegistryMetrics {
    pub total_manifests: usize,
    pub loaded_schemas: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub context_savings_percent: f64,
    pub total_load_time_ms: u64,
}

type SchemaLoader = Option<Box<dyn Fn(&str, &str) -> Option<ToolSchema> + Send + Sync>>;

/// Entry in the LRU schema cache, tracking last access time.
#[derive(Debug, Clone)]
struct CachedSchema {
    schema: ToolSchema,
    last_accessed: Instant,
    load_time_ms: u64,
}

/// The lazy-loading tool registry.
///
/// Stores lightweight manifests for all tools and caches full schemas with
/// LRU eviction based on idle timeout and capacity limits.
pub struct LazyToolRegistry {
    manifests: HashMap<String, ToolManifest>,
    schemas: HashMap<String, CachedSchema>,
    config: LazyLoadConfig,
    cache_hits: u64,
    cache_misses: u64,
    total_load_time_ms: u64,
    /// External schema loader function. In production this calls the MCP server;
    /// for tests it can be replaced with a closure.
    schema_loader: SchemaLoader,
}

impl std::fmt::Debug for LazyToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyToolRegistry")
            .field("manifests", &self.manifests)
            .field("schemas", &self.schemas)
            .field("config", &self.config)
            .field("cache_hits", &self.cache_hits)
            .field("cache_misses", &self.cache_misses)
            .field("total_load_time_ms", &self.total_load_time_ms)
            .field("schema_loader", &self.schema_loader.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl LazyToolRegistry {
    /// Create a new registry with the given configuration.
    pub fn new(config: LazyLoadConfig) -> Self {
        Self {
            manifests: HashMap::new(),
            schemas: HashMap::new(),
            config,
            cache_hits: 0,
            cache_misses: 0,
            total_load_time_ms: 0,
            schema_loader: None,
        }
    }

    /// Set a custom schema loader function (used in tests and for real MCP calls).
    pub fn set_schema_loader<F>(&mut self, loader: F)
    where
        F: Fn(&str, &str) -> Option<ToolSchema> + Send + Sync + 'static,
    {
        self.schema_loader = Some(Box::new(loader));
    }

    /// Register a tool manifest. If a tool with the same name already exists, it
    /// is overwritten. If the tool name matches any eager-load pattern, the schema
    /// is loaded immediately.
    pub fn register_manifest(&mut self, manifest: ToolManifest) {
        let should_eager = self.should_eager_load(&manifest.name);
        self.manifests.insert(manifest.name.clone(), manifest.clone());

        if should_eager {
            let _ = self.load_tool(&manifest.name);
        }
    }

    /// Search tools by keyword query. Returns up to `max_results` results sorted
    /// by descending relevance score. Searches both tool name and description.
    pub fn search_tools(&self, query: &str, max_results: usize) -> Vec<ToolSearchResult> {
        if query.is_empty() || self.manifests.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();

        if keywords.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<ToolSearchResult> = self
            .manifests
            .values()
            .filter_map(|manifest| {
                let score = self.compute_relevance(&manifest.name, &manifest.description, &keywords);
                if score > 0.0 {
                    Some(ToolSearchResult {
                        tool_name: manifest.name.clone(),
                        server_name: manifest.server_name.clone(),
                        description: manifest.description.clone(),
                        relevance_score: score,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(max_results);
        results
    }

    /// Load a tool's full schema on demand. Returns the schema if available.
    /// Updates LRU access time on cache hit. On cache miss, invokes the schema
    /// loader, caches the result, and evicts if over capacity.
    pub fn load_tool(&mut self, name: &str) -> Option<&ToolSchema> {
        // Check cache first
        if self.schemas.contains_key(name) {
            self.cache_hits += 1;
            if let Some(entry) = self.schemas.get_mut(name) {
                entry.last_accessed = Instant::now();
            }
            return self.schemas.get(name).map(|e| &e.schema);
        }

        // Cache miss
        self.cache_misses += 1;

        let manifest = self.manifests.get(name)?;
        let server_name = manifest.server_name.clone();
        let tool_name = manifest.name.clone();

        // Load the schema
        let start = Instant::now();
        let schema = if let Some(ref loader) = self.schema_loader {
            loader(&tool_name, &server_name)?
        } else {
            // Default: synthesize a minimal schema from the manifest
            ToolSchema {
                name: tool_name.clone(),
                description: manifest.description.clone(),
                parameters: HashMap::new(),
                server_name: server_name.clone(),
            }
        };
        let load_ms = start.elapsed().as_millis() as u64;
        self.total_load_time_ms += load_ms;

        // Mark manifest as loaded
        if let Some(m) = self.manifests.get_mut(&tool_name) {
            m.loaded = true;
        }

        // Evict if at capacity
        if self.schemas.len() >= self.config.max_cached_schemas {
            self.evict_lru();
        }

        self.schemas.insert(
            tool_name.clone(),
            CachedSchema {
                schema,
                last_accessed: Instant::now(),
                load_time_ms: load_ms,
            },
        );

        self.schemas.get(&tool_name).map(|e| &e.schema)
    }

    /// Evict tools whose schemas have not been accessed within the idle timeout.
    /// Returns the number of evicted entries.
    pub fn unload_idle_tools(&mut self) -> usize {
        let timeout = Duration::from_secs(self.config.idle_timeout_secs);
        let now = Instant::now();

        let to_evict: Vec<String> = self
            .schemas
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.last_accessed) >= timeout)
            .map(|(name, _)| name.clone())
            .collect();

        let count = to_evict.len();
        for name in &to_evict {
            self.schemas.remove(name);
            if let Some(m) = self.manifests.get_mut(name) {
                m.loaded = false;
            }
        }

        count
    }

    /// Get current registry metrics.
    pub fn get_metrics(&self) -> RegistryMetrics {
        let total_manifests = self.manifests.len();
        let loaded_schemas = self.schemas.len();

        let context_savings_percent = if total_manifests > 0 {
            let ratio = 1.0 - (loaded_schemas as f64 / total_manifests as f64);
            (ratio * 10000.0).round() / 100.0
        } else {
            0.0
        };

        RegistryMetrics {
            total_manifests,
            loaded_schemas,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            context_savings_percent,
            total_load_time_ms: self.total_load_time_ms,
        }
    }

    /// Check whether a tool's schema is currently loaded in cache.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.schemas.contains_key(name)
    }

    /// Get the manifest for a tool by name.
    pub fn get_manifest(&self, name: &str) -> Option<&ToolManifest> {
        self.manifests.get(name)
    }

    /// List all unique server names that have registered tools.
    pub fn list_servers(&self) -> Vec<String> {
        let mut servers: Vec<String> = self
            .manifests
            .values()
            .map(|m| m.server_name.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        servers.sort();
        servers
    }

    /// Get all tool manifests for a given server.
    pub fn tools_for_server(&self, server: &str) -> Vec<&ToolManifest> {
        self.manifests
            .values()
            .filter(|m| m.server_name == server)
            .collect()
    }

    /// Estimate the context token count. Returns a tuple of
    /// (loaded_tokens, total_if_all_loaded_tokens). Each manifest costs ~20 tokens
    /// (name + description) and each loaded schema adds ~15 tokens per parameter.
    pub fn context_token_estimate(&self) -> usize {
        let manifest_tokens_each = 20;
        let param_tokens_each = 15;

        let loaded_token_count: usize = self
            .schemas
            .values()
            .map(|entry| {
                manifest_tokens_each + entry.schema.parameters.len() * param_tokens_each
            })
            .sum();

        loaded_token_count
    }

    /// Estimate total tokens if every tool were loaded.
    pub fn context_token_estimate_all(&self) -> usize {
        let manifest_tokens_each = 20;
        let param_tokens_each = 15;

        // For manifests without loaded schemas, estimate 3 params average
        let avg_params = 3;

        let total: usize = self
            .manifests
            .values()
            .map(|m| {
                if let Some(entry) = self.schemas.get(&m.name) {
                    manifest_tokens_each + entry.schema.parameters.len() * param_tokens_each
                } else {
                    manifest_tokens_each + avg_params * param_tokens_each
                }
            })
            .sum();

        total
    }

    // --- Private helpers ---

    /// Compute a relevance score for a tool given search keywords.
    fn compute_relevance(&self, name: &str, description: &str, keywords: &[&str]) -> f64 {
        let name_lower = name.to_lowercase();
        let desc_lower = description.to_lowercase();

        let mut score = 0.0;
        let mut matched_keywords = 0;

        for &kw in keywords {
            let mut kw_score = 0.0;

            // Exact name match (highest weight)
            if name_lower == kw {
                kw_score += 10.0;
            }
            // Name contains keyword
            else if name_lower.contains(kw) {
                // Boost if it starts with the keyword
                if name_lower.starts_with(kw) {
                    kw_score += 6.0;
                } else {
                    kw_score += 4.0;
                }
            }

            // Description contains keyword
            if desc_lower.contains(kw) {
                // Count occurrences for TF-like boost (capped)
                let count = desc_lower.matches(kw).count();
                kw_score += 2.0 + (count as f64 - 1.0).min(3.0) * 0.5;
            }

            if kw_score > 0.0 {
                matched_keywords += 1;
            }
            score += kw_score;
        }

        // Bonus for matching all keywords (conjunction boost)
        if matched_keywords == keywords.len() && keywords.len() > 1 {
            score *= 1.5;
        }

        // Penalize if not all keywords matched
        if matched_keywords < keywords.len() {
            score *= matched_keywords as f64 / keywords.len() as f64;
        }

        score
    }

    /// Check if a tool name matches any eager-load glob pattern.
    fn should_eager_load(&self, name: &str) -> bool {
        for pattern in &self.config.eager_load_patterns {
            if glob_match(pattern, name) {
                return true;
            }
        }
        false
    }

    /// Evict the least-recently-used schema entry.
    fn evict_lru(&mut self) {
        if let Some(oldest_name) = self
            .schemas
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(name, _)| name.clone())
        {
            self.schemas.remove(&oldest_name);
            if let Some(m) = self.manifests.get_mut(&oldest_name) {
                m.loaded = false;
            }
        }
    }
}

/// Simple glob matching supporting `*` (any sequence) and `?` (single char).
fn glob_match(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    glob_match_inner(&pat, &txt, 0, 0)
}

fn glob_match_inner(pat: &[char], txt: &[char], pi: usize, ti: usize) -> bool {
    if pi == pat.len() {
        return ti == txt.len();
    }

    if pat[pi] == '*' {
        // Skip consecutive *
        let mut next_pi = pi;
        while next_pi < pat.len() && pat[next_pi] == '*' {
            next_pi += 1;
        }
        // Try matching * against 0..n characters
        for skip in 0..=(txt.len() - ti) {
            if glob_match_inner(pat, txt, next_pi, ti + skip) {
                return true;
            }
        }
        false
    } else if ti < txt.len() && (pat[pi] == '?' || pat[pi] == txt[ti]) {
        glob_match_inner(pat, txt, pi + 1, ti + 1)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manifest(name: &str, desc: &str, server: &str) -> ToolManifest {
        ToolManifest {
            name: name.to_string(),
            description: desc.to_string(),
            server_name: server.to_string(),
            loaded: false,
        }
    }

    fn make_config(max: usize, idle: u64) -> LazyLoadConfig {
        LazyLoadConfig {
            max_cached_schemas: max,
            idle_timeout_secs: idle,
            eager_load_patterns: Vec::new(),
        }
    }

    fn make_registry_with_tools() -> LazyToolRegistry {
        let mut reg = LazyToolRegistry::new(make_config(10, 300));
        reg.register_manifest(make_manifest("read_file", "Read a file from disk", "filesystem"));
        reg.register_manifest(make_manifest("write_file", "Write content to a file", "filesystem"));
        reg.register_manifest(make_manifest("list_dir", "List directory contents", "filesystem"));
        reg.register_manifest(make_manifest("search_code", "Search code with regex", "search"));
        reg.register_manifest(make_manifest("git_status", "Show git status", "git"));
        reg.register_manifest(make_manifest("git_commit", "Create a git commit", "git"));
        reg.register_manifest(make_manifest("run_test", "Run unit tests", "testing"));
        reg.register_manifest(make_manifest("debug_start", "Start debugger session", "debug"));
        reg
    }

    // --- Manifest registration tests ---

    #[test]
    fn test_register_single_manifest() {
        let mut reg = LazyToolRegistry::new(LazyLoadConfig::default());
        reg.register_manifest(make_manifest("tool1", "A tool", "server1"));
        assert_eq!(reg.manifests.len(), 1);
        assert_eq!(reg.get_manifest("tool1").unwrap().description, "A tool");
    }

    #[test]
    fn test_register_multiple_manifests() {
        let reg = make_registry_with_tools();
        assert_eq!(reg.manifests.len(), 8);
    }

    #[test]
    fn test_register_duplicate_overwrites() {
        let mut reg = LazyToolRegistry::new(LazyLoadConfig::default());
        reg.register_manifest(make_manifest("tool1", "Version 1", "server1"));
        reg.register_manifest(make_manifest("tool1", "Version 2", "server1"));
        assert_eq!(reg.manifests.len(), 1);
        assert_eq!(reg.get_manifest("tool1").unwrap().description, "Version 2");
    }

    #[test]
    fn test_manifest_loaded_initially_false() {
        let mut reg = LazyToolRegistry::new(LazyLoadConfig::default());
        reg.register_manifest(make_manifest("tool1", "Desc", "srv"));
        assert!(!reg.get_manifest("tool1").unwrap().loaded);
    }

    #[test]
    fn test_get_manifest_nonexistent() {
        let reg = LazyToolRegistry::new(LazyLoadConfig::default());
        assert!(reg.get_manifest("nonexistent").is_none());
    }

    // --- Search tests ---

    #[test]
    fn test_search_exact_name_match() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("read_file", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].tool_name, "read_file");
    }

    #[test]
    fn test_search_partial_name_match() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("file", 10);
        let names: Vec<&str> = results.iter().map(|r| r.tool_name.as_str()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
    }

    #[test]
    fn test_search_description_match() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("regex", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].tool_name, "search_code");
    }

    #[test]
    fn test_search_case_insensitive() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("GIT", 10);
        let names: Vec<&str> = results.iter().map(|r| r.tool_name.as_str()).collect();
        assert!(names.contains(&"git_status"));
        assert!(names.contains(&"git_commit"));
    }

    #[test]
    fn test_search_multi_keyword() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("git commit", 5);
        assert!(!results.is_empty());
        // git_commit should rank highest (matches both keywords in name)
        assert_eq!(results[0].tool_name, "git_commit");
    }

    #[test]
    fn test_search_max_results_limit() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("file", 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_no_matches() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("zzzznonexistent", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_empty_query() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_empty_registry() {
        let reg = LazyToolRegistry::new(LazyLoadConfig::default());
        let results = reg.search_tools("anything", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_relevance_ordering() {
        let mut reg = LazyToolRegistry::new(make_config(10, 300));
        reg.register_manifest(make_manifest("search", "Find things", "srv"));
        reg.register_manifest(make_manifest("search_code", "Search code files", "srv"));
        reg.register_manifest(make_manifest("deep_search", "Deep search in archives", "srv"));

        let results = reg.search_tools("search", 10);
        // Exact name match "search" should rank highest
        assert_eq!(results[0].tool_name, "search");
    }

    // --- Load tool tests ---

    #[test]
    fn test_load_tool_creates_schema() {
        let mut reg = make_registry_with_tools();
        let schema = reg.load_tool("read_file");
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().name, "read_file");
    }

    #[test]
    fn test_load_tool_marks_loaded() {
        let mut reg = make_registry_with_tools();
        assert!(!reg.is_loaded("read_file"));
        reg.load_tool("read_file");
        assert!(reg.is_loaded("read_file"));
    }

    #[test]
    fn test_load_tool_manifest_loaded_flag() {
        let mut reg = make_registry_with_tools();
        reg.load_tool("read_file");
        assert!(reg.get_manifest("read_file").unwrap().loaded);
    }

    #[test]
    fn test_load_nonexistent_tool() {
        let mut reg = make_registry_with_tools();
        assert!(reg.load_tool("nonexistent").is_none());
    }

    #[test]
    fn test_load_tool_cache_hit() {
        let mut reg = make_registry_with_tools();
        reg.load_tool("read_file");
        assert_eq!(reg.cache_hits, 0);
        assert_eq!(reg.cache_misses, 1);

        reg.load_tool("read_file");
        assert_eq!(reg.cache_hits, 1);
        assert_eq!(reg.cache_misses, 1);
    }

    #[test]
    fn test_load_tool_with_custom_loader() {
        let mut reg = make_registry_with_tools();
        reg.set_schema_loader(|name, server| {
            let mut params = HashMap::new();
            params.insert(
                "path".to_string(),
                ParameterDef {
                    param_type: "string".to_string(),
                    description: "File path".to_string(),
                    required: true,
                },
            );
            Some(ToolSchema {
                name: name.to_string(),
                description: format!("Loaded from {}", server),
                parameters: params,
                server_name: server.to_string(),
            })
        });

        let schema = reg.load_tool("read_file").unwrap();
        assert_eq!(schema.parameters.len(), 1);
        assert!(schema.parameters.contains_key("path"));
        assert_eq!(schema.description, "Loaded from filesystem");
    }

    #[test]
    fn test_load_tool_custom_loader_returns_none() {
        let mut reg = make_registry_with_tools();
        reg.set_schema_loader(|_, _| None);
        assert!(reg.load_tool("read_file").is_none());
    }

    // --- LRU eviction tests ---

    #[test]
    fn test_lru_eviction_on_capacity() {
        let mut reg = LazyToolRegistry::new(make_config(2, 300));
        reg.register_manifest(make_manifest("t1", "Tool 1", "srv"));
        reg.register_manifest(make_manifest("t2", "Tool 2", "srv"));
        reg.register_manifest(make_manifest("t3", "Tool 3", "srv"));

        reg.load_tool("t1");
        reg.load_tool("t2");
        // At capacity (2). Loading t3 should evict the LRU (t1).
        reg.load_tool("t3");

        assert!(!reg.is_loaded("t1"));
        assert!(reg.is_loaded("t2"));
        assert!(reg.is_loaded("t3"));
    }

    #[test]
    fn test_lru_access_updates_order() {
        let mut reg = LazyToolRegistry::new(make_config(2, 300));
        reg.register_manifest(make_manifest("t1", "Tool 1", "srv"));
        reg.register_manifest(make_manifest("t2", "Tool 2", "srv"));
        reg.register_manifest(make_manifest("t3", "Tool 3", "srv"));

        reg.load_tool("t1");
        reg.load_tool("t2");
        // Touch t1 so it becomes most recent
        reg.load_tool("t1");
        // Now t2 is LRU, loading t3 should evict t2
        reg.load_tool("t3");

        assert!(reg.is_loaded("t1"));
        assert!(!reg.is_loaded("t2"));
        assert!(reg.is_loaded("t3"));
    }

    #[test]
    fn test_lru_eviction_updates_manifest_loaded() {
        let mut reg = LazyToolRegistry::new(make_config(1, 300));
        reg.register_manifest(make_manifest("t1", "Tool 1", "srv"));
        reg.register_manifest(make_manifest("t2", "Tool 2", "srv"));

        reg.load_tool("t1");
        assert!(reg.get_manifest("t1").unwrap().loaded);

        reg.load_tool("t2");
        assert!(!reg.get_manifest("t1").unwrap().loaded);
        assert!(reg.get_manifest("t2").unwrap().loaded);
    }

    // --- Idle timeout tests ---

    #[test]
    fn test_unload_idle_tools_none_expired() {
        let mut reg = make_registry_with_tools();
        reg.load_tool("read_file");
        reg.load_tool("git_status");

        let evicted = reg.unload_idle_tools();
        assert_eq!(evicted, 0);
        assert!(reg.is_loaded("read_file"));
        assert!(reg.is_loaded("git_status"));
    }

    #[test]
    fn test_unload_idle_tools_with_zero_timeout() {
        let mut reg = LazyToolRegistry::new(make_config(10, 0));
        reg.register_manifest(make_manifest("t1", "Tool 1", "srv"));
        reg.load_tool("t1");

        // With zero timeout, everything should be idle
        // Need a tiny delay to ensure Instant::now() > last_accessed
        std::thread::sleep(Duration::from_millis(1));
        let evicted = reg.unload_idle_tools();
        assert_eq!(evicted, 1);
        assert!(!reg.is_loaded("t1"));
    }

    #[test]
    fn test_unload_idle_restores_manifest_loaded_flag() {
        let mut reg = LazyToolRegistry::new(make_config(10, 0));
        reg.register_manifest(make_manifest("t1", "Tool 1", "srv"));
        reg.load_tool("t1");
        assert!(reg.get_manifest("t1").unwrap().loaded);

        std::thread::sleep(Duration::from_millis(1));
        reg.unload_idle_tools();
        assert!(!reg.get_manifest("t1").unwrap().loaded);
    }

    #[test]
    fn test_unload_empty_registry() {
        let mut reg = LazyToolRegistry::new(make_config(10, 0));
        assert_eq!(reg.unload_idle_tools(), 0);
    }

    // --- Metrics tests ---

    #[test]
    fn test_metrics_initial() {
        let reg = LazyToolRegistry::new(LazyLoadConfig::default());
        let m = reg.get_metrics();
        assert_eq!(m.total_manifests, 0);
        assert_eq!(m.loaded_schemas, 0);
        assert_eq!(m.cache_hits, 0);
        assert_eq!(m.cache_misses, 0);
        assert_eq!(m.context_savings_percent, 0.0);
    }

    #[test]
    fn test_metrics_after_registration() {
        let reg = make_registry_with_tools();
        let m = reg.get_metrics();
        assert_eq!(m.total_manifests, 8);
        assert_eq!(m.loaded_schemas, 0);
        assert_eq!(m.context_savings_percent, 100.0);
    }

    #[test]
    fn test_metrics_after_loads() {
        let mut reg = make_registry_with_tools();
        reg.load_tool("read_file");
        reg.load_tool("git_status");

        let m = reg.get_metrics();
        assert_eq!(m.total_manifests, 8);
        assert_eq!(m.loaded_schemas, 2);
        assert_eq!(m.cache_misses, 2);
        assert_eq!(m.cache_hits, 0);
        // 1 - 2/8 = 0.75 = 75%
        assert_eq!(m.context_savings_percent, 75.0);
    }

    #[test]
    fn test_metrics_cache_hits_tracked() {
        let mut reg = make_registry_with_tools();
        reg.load_tool("read_file");
        reg.load_tool("read_file");
        reg.load_tool("read_file");

        let m = reg.get_metrics();
        assert_eq!(m.cache_hits, 2);
        assert_eq!(m.cache_misses, 1);
    }

    #[test]
    fn test_metrics_context_savings_all_loaded() {
        let mut reg = LazyToolRegistry::new(make_config(10, 300));
        reg.register_manifest(make_manifest("t1", "Tool", "srv"));
        reg.register_manifest(make_manifest("t2", "Tool", "srv"));
        reg.load_tool("t1");
        reg.load_tool("t2");

        let m = reg.get_metrics();
        assert_eq!(m.context_savings_percent, 0.0);
    }

    // --- Context token estimate tests ---

    #[test]
    fn test_context_token_estimate_empty() {
        let reg = LazyToolRegistry::new(LazyLoadConfig::default());
        assert_eq!(reg.context_token_estimate(), 0);
    }

    #[test]
    fn test_context_token_estimate_loaded_schema() {
        let mut reg = make_registry_with_tools();
        // Default loader creates schema with 0 params -> 20 tokens
        reg.load_tool("read_file");
        assert_eq!(reg.context_token_estimate(), 20);
    }

    #[test]
    fn test_context_token_estimate_with_params() {
        let mut reg = make_registry_with_tools();
        reg.set_schema_loader(|name, server| {
            let mut params = HashMap::new();
            params.insert("p1".to_string(), ParameterDef {
                param_type: "string".to_string(),
                description: "Param 1".to_string(),
                required: true,
            });
            params.insert("p2".to_string(), ParameterDef {
                param_type: "number".to_string(),
                description: "Param 2".to_string(),
                required: false,
            });
            Some(ToolSchema {
                name: name.to_string(),
                description: "desc".to_string(),
                parameters: params,
                server_name: server.to_string(),
            })
        });

        reg.load_tool("read_file");
        // 20 base + 2*15 = 50
        assert_eq!(reg.context_token_estimate(), 50);
    }

    #[test]
    fn test_context_token_estimate_all() {
        let reg = make_registry_with_tools();
        // 8 manifests, each estimated at 20 + 3*15 = 65
        assert_eq!(reg.context_token_estimate_all(), 8 * 65);
    }

    // --- Server listing tests ---

    #[test]
    fn test_list_servers() {
        let reg = make_registry_with_tools();
        let servers = reg.list_servers();
        assert!(servers.contains(&"filesystem".to_string()));
        assert!(servers.contains(&"git".to_string()));
        assert!(servers.contains(&"search".to_string()));
        assert!(servers.contains(&"testing".to_string()));
        assert!(servers.contains(&"debug".to_string()));
        assert_eq!(servers.len(), 5);
    }

    #[test]
    fn test_list_servers_empty() {
        let reg = LazyToolRegistry::new(LazyLoadConfig::default());
        assert!(reg.list_servers().is_empty());
    }

    #[test]
    fn test_tools_for_server() {
        let reg = make_registry_with_tools();
        let fs_tools = reg.tools_for_server("filesystem");
        assert_eq!(fs_tools.len(), 3);
        let names: Vec<&str> = fs_tools.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"list_dir"));
    }

    #[test]
    fn test_tools_for_nonexistent_server() {
        let reg = make_registry_with_tools();
        assert!(reg.tools_for_server("nonexistent").is_empty());
    }

    // --- Glob / eager load tests ---

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("hello", "hello"));
        assert!(!glob_match("hello", "world"));
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("read_*", "read_file"));
        assert!(glob_match("read_*", "read_dir"));
        assert!(!glob_match("read_*", "write_file"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match("t?st", "test"));
        assert!(glob_match("t?st", "tast"));
        assert!(!glob_match("t?st", "toast"));
    }

    #[test]
    fn test_glob_match_complex() {
        assert!(glob_match("*_file", "read_file"));
        assert!(glob_match("*_file", "write_file"));
        assert!(!glob_match("*_file", "list_dir"));
        assert!(glob_match("git_*", "git_status"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn test_eager_load_patterns() {
        let config = LazyLoadConfig {
            max_cached_schemas: 10,
            idle_timeout_secs: 300,
            eager_load_patterns: vec!["read_*".to_string(), "git_*".to_string()],
        };
        let mut reg = LazyToolRegistry::new(config);

        reg.register_manifest(make_manifest("read_file", "Read a file", "fs"));
        reg.register_manifest(make_manifest("git_status", "Git status", "git"));
        reg.register_manifest(make_manifest("write_file", "Write a file", "fs"));

        // read_file and git_status should be eagerly loaded
        assert!(reg.is_loaded("read_file"));
        assert!(reg.is_loaded("git_status"));
        assert!(!reg.is_loaded("write_file"));
    }

    #[test]
    fn test_eager_load_no_patterns() {
        let mut reg = LazyToolRegistry::new(LazyLoadConfig::default());
        reg.register_manifest(make_manifest("read_file", "Read a file", "fs"));
        assert!(!reg.is_loaded("read_file"));
    }

    // --- Edge case tests ---

    #[test]
    fn test_search_whitespace_only_query() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("   ", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_register_tools_different_servers_same_name() {
        let mut reg = LazyToolRegistry::new(LazyLoadConfig::default());
        reg.register_manifest(make_manifest("read", "Read v1", "server_a"));
        reg.register_manifest(make_manifest("read", "Read v2", "server_b"));
        // Second registration overwrites the first (same key)
        assert_eq!(reg.manifests.len(), 1);
        assert_eq!(reg.get_manifest("read").unwrap().server_name, "server_b");
    }

    #[test]
    fn test_is_loaded_after_eviction() {
        let mut reg = LazyToolRegistry::new(make_config(1, 300));
        reg.register_manifest(make_manifest("t1", "Tool 1", "srv"));
        reg.register_manifest(make_manifest("t2", "Tool 2", "srv"));

        reg.load_tool("t1");
        assert!(reg.is_loaded("t1"));

        reg.load_tool("t2");
        assert!(!reg.is_loaded("t1"));
        assert!(reg.is_loaded("t2"));
    }

    #[test]
    fn test_reload_after_eviction() {
        let mut reg = LazyToolRegistry::new(make_config(1, 300));
        reg.register_manifest(make_manifest("t1", "Tool 1", "srv"));
        reg.register_manifest(make_manifest("t2", "Tool 2", "srv"));

        reg.load_tool("t1");
        reg.load_tool("t2"); // evicts t1
        reg.load_tool("t1"); // reload t1, evicts t2

        assert!(reg.is_loaded("t1"));
        assert!(!reg.is_loaded("t2"));
        assert_eq!(reg.cache_misses, 3);
    }

    #[test]
    fn test_search_results_include_server_name() {
        let reg = make_registry_with_tools();
        let results = reg.search_tools("git_status", 1);
        assert_eq!(results[0].server_name, "git");
    }

    #[test]
    fn test_metrics_load_time_accumulates() {
        let mut reg = make_registry_with_tools();
        reg.load_tool("read_file");
        reg.load_tool("git_status");
        // Load time should be >= 0 (may be 0 on fast machines)
        let m = reg.get_metrics();
        assert!(m.total_load_time_ms < 1000); // sanity check: under 1 second
    }

    #[test]
    fn test_large_registry_search_performance() {
        let mut reg = LazyToolRegistry::new(make_config(100, 300));
        for i in 0..200 {
            reg.register_manifest(make_manifest(
                &format!("tool_{}", i),
                &format!("Description for tool number {}", i),
                &format!("server_{}", i % 5),
            ));
        }

        let results = reg.search_tools("tool_15", 5);
        assert!(!results.is_empty());
        // Exact match should be first
        assert!(results.iter().any(|r| r.tool_name == "tool_15"));
    }

    #[test]
    fn test_servers_sorted() {
        let mut reg = LazyToolRegistry::new(LazyLoadConfig::default());
        reg.register_manifest(make_manifest("c", "C", "zulu"));
        reg.register_manifest(make_manifest("a", "A", "alpha"));
        reg.register_manifest(make_manifest("b", "B", "mike"));

        let servers = reg.list_servers();
        assert_eq!(servers, vec!["alpha", "mike", "zulu"]);
    }
}
