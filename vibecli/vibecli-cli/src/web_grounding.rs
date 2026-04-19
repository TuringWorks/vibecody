#![allow(dead_code)]
//! Web search grounding for the agent loop.
//!
//! Provides web search integration with multiple providers (Google, Bing, Brave,
//! SearXNG, Tavily, DuckDuckGo), result ranking, caching, rate limiting, citation
//! tracking, and XML tool definitions for agent system prompt injection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Search Provider ─────────────────────────────────────────────────────────

/// Supported web search backend providers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchProvider {
    Google,
    Bing,
    Brave,
    SearXNG,
    Tavily,
    DuckDuckGo,
}

impl SearchProvider {
    /// Default API endpoint for this provider (where applicable).
    pub fn default_endpoint(&self) -> &str {
        match self {
            Self::Google => "https://www.googleapis.com/customsearch/v1",
            Self::Bing => "https://api.bing.microsoft.com/v7.0/search",
            Self::Brave => "https://api.search.brave.com/res/v1/web/search",
            Self::SearXNG => "http://localhost:8888/search",
            Self::Tavily => "https://api.tavily.com/search",
            Self::DuckDuckGo => "https://api.duckduckgo.com/",
        }
    }

    /// Whether this provider requires an API key.
    pub fn requires_api_key(&self) -> bool {
        match self {
            Self::Google | Self::Bing | Self::Brave | Self::Tavily => true,
            Self::SearXNG | Self::DuckDuckGo => false,
        }
    }
}

// ── Search Config ───────────────────────────────────────────────────────────

/// Configuration for the web search engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchConfig {
    pub provider: SearchProvider,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_results: u32,
    pub cache_ttl_secs: u64,
    pub rate_limit_per_min: u32,
    pub privacy_mode: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            provider: SearchProvider::DuckDuckGo,
            api_key: None,
            base_url: None,
            max_results: 5,
            cache_ttl_secs: 3600,
            rate_limit_per_min: 30,
            privacy_mode: false,
        }
    }
}

// ── Search Query ────────────────────────────────────────────────────────────

/// A search query with optional context and filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub context: Option<String>,
    pub language: Option<String>,
    pub filters: Vec<SearchFilter>,
}

impl SearchQuery {
    /// Build the effective query string including applied filters.
    pub fn effective_query(&self) -> String {
        let mut parts = vec![self.query.clone()];
        for filter in &self.filters {
            match filter {
                SearchFilter::Site(s) => parts.push(format!("site:{s}")),
                SearchFilter::FileType(ft) => parts.push(format!("filetype:{ft}")),
                SearchFilter::DateRange(_, _) => {} // handled by API params
                SearchFilter::ExcludeSite(s) => parts.push(format!("-site:{s}")),
            }
        }
        if let Some(lang) = &self.language {
            parts.push(format!("lang:{lang}"));
        }
        parts.join(" ")
    }
}

// ── Search Filter ───────────────────────────────────────────────────────────

/// Filters that can be applied to a search query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchFilter {
    Site(String),
    FileType(String),
    DateRange(u64, u64),
    ExcludeSite(String),
}

// ── Result Content Type ─────────────────────────────────────────────────────

/// Classification of a search result's content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResultContentType {
    Documentation,
    StackOverflow,
    BlogPost,
    OfficialDocs,
    GitHubRepo,
    Tutorial,
    Forum,
    Unknown,
}

impl ResultContentType {
    /// Relevance boost factor for this content type.
    fn boost(&self) -> f64 {
        match self {
            Self::OfficialDocs => 0.15,
            Self::Documentation => 0.12,
            Self::StackOverflow => 0.10,
            Self::GitHubRepo => 0.08,
            Self::Tutorial => 0.06,
            Self::BlogPost => 0.03,
            Self::Forum => 0.02,
            Self::Unknown => 0.0,
        }
    }
}

// ── Search Result ───────────────────────────────────────────────────────────

/// A single web search result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: SearchProvider,
    pub relevance_score: f64,
    pub fetched_at: u64,
    pub content_type: ResultContentType,
}

// ── Citation ────────────────────────────────────────────────────────────────

/// A citation referencing a search result used by the agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Citation {
    pub id: String,
    pub url: String,
    pub title: String,
    pub used_in_context: String,
    pub result_index: usize,
    pub timestamp: u64,
}

// ── Cache ───────────────────────────────────────────────────────────────────

/// A single cache entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEntry {
    pub results: Vec<SearchResult>,
    pub inserted_at: u64,
}

/// Statistics about cache usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
}

/// LRU-ish cache for search results, keyed by query string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCache {
    pub entries: HashMap<String, CacheEntry>,
    pub max_entries: usize,
    pub ttl_secs: u64,
    hit_count: u64,
    miss_count: u64,
}

impl SearchCache {
    pub fn new(max_entries: usize, ttl_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            ttl_secs,
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Look up cached results for a query. Returns None if missing or expired.
    pub fn get(&mut self, query: &str, now: u64) -> Option<&[SearchResult]> {
        // Check expiry first
        if let Some(entry) = self.entries.get(query) {
            if now.saturating_sub(entry.inserted_at) > self.ttl_secs {
                self.entries.remove(query);
                self.miss_count += 1;
                return None;
            }
        }
        if self.entries.contains_key(query) {
            self.hit_count += 1;
            Some(&self.entries[query].results)
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// Insert results into the cache. Evicts oldest entry if full.
    pub fn put(&mut self, query: &str, results: Vec<SearchResult>, now: u64) {
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(query) {
            // Evict the oldest entry
            if let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, v)| v.inserted_at)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&oldest_key);
            }
        }
        self.entries.insert(
            query.to_string(),
            CacheEntry {
                results,
                inserted_at: now,
            },
        );
    }

    /// Remove all entries older than TTL. Returns count of evicted entries.
    pub fn evict_expired(&mut self, now: u64) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|_, v| now.saturating_sub(v.inserted_at) <= self.ttl_secs);
        before - self.entries.len()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hit_count = 0;
        self.miss_count = 0;
    }

    /// Return usage statistics.
    pub fn stats(&self) -> CacheStats {
        let total = self.hit_count + self.miss_count;
        CacheStats {
            total_entries: self.entries.len(),
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            hit_rate: if total == 0 {
                0.0
            } else {
                self.hit_count as f64 / total as f64
            },
        }
    }
}

// ── Rate Limiter ────────────────────────────────────────────────────────────

/// Simple sliding-window rate limiter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    pub requests: Vec<u64>,
    pub max_per_minute: u32,
}

impl RateLimiter {
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            requests: Vec::new(),
            max_per_minute,
        }
    }

    /// Whether a request is allowed at the given timestamp.
    pub fn can_request(&self, now: u64) -> bool {
        let window_start = now.saturating_sub(60);
        let count = self.requests.iter().filter(|&&t| t > window_start).count();
        (count as u32) < self.max_per_minute
    }

    /// Record a request timestamp.
    pub fn record_request(&mut self, now: u64) {
        self.requests.push(now);
        // Prune old entries outside the window
        let window_start = now.saturating_sub(60);
        self.requests.retain(|&t| t > window_start);
    }

    /// How many requests remain in the current window.
    pub fn remaining(&self, now: u64) -> u32 {
        let window_start = now.saturating_sub(60);
        let count = self.requests.iter().filter(|&&t| t > window_start).count() as u32;
        self.max_per_minute.saturating_sub(count)
    }
}

// ── Search Record ───────────────────────────────────────────────────────────

/// Log of a single search execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchRecord {
    pub query: String,
    pub provider: SearchProvider,
    pub results_count: usize,
    pub cached: bool,
    pub timestamp: u64,
}

// ── Grounding Metrics ───────────────────────────────────────────────────────

/// Aggregate metrics for web grounding usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundingMetrics {
    pub total_searches: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub total_citations: u32,
    pub avg_results_per_search: f64,
    pub rate_limited_count: u32,
}

impl Default for GroundingMetrics {
    fn default() -> Self {
        Self {
            total_searches: 0,
            cache_hits: 0,
            cache_misses: 0,
            total_citations: 0,
            avg_results_per_search: 0.0,
            rate_limited_count: 0,
        }
    }
}

// ── WebGroundingEngine ──────────────────────────────────────────────────────

/// Core engine that orchestrates web search, caching, rate limiting, and citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebGroundingEngine {
    pub config: SearchConfig,
    pub cache: SearchCache,
    pub rate_limiter: RateLimiter,
    pub citations: Vec<Citation>,
    pub search_history: Vec<SearchRecord>,
    pub metrics: GroundingMetrics,
    /// Monotonic clock substitute for testing — callers set this before calls.
    pub now: u64,
}

impl WebGroundingEngine {
    /// Create a new engine from the given configuration.
    pub fn new(config: SearchConfig) -> Self {
        let cache = SearchCache::new(500, config.cache_ttl_secs);
        let rate_limiter = RateLimiter::new(config.rate_limit_per_min);
        Self {
            config,
            cache,
            rate_limiter,
            citations: Vec::new(),
            search_history: Vec::new(),
            metrics: GroundingMetrics::default(),
            now: 0,
        }
    }

    /// Execute a basic search (no context-aware ranking).
    pub fn search(&mut self, query: &str) -> Result<Vec<SearchResult>, String> {
        if query.trim().is_empty() {
            return Err("Search query must not be empty".to_string());
        }

        let now = self.now;

        // Rate limit check
        if !self.rate_limiter.can_request(now) {
            self.metrics.rate_limited_count += 1;
            return Err("Rate limit exceeded — try again shortly".to_string());
        }

        // Cache check
        if let Some(cached) = self.cache.get(query, now) {
            let results = cached.to_vec();
            self.metrics.cache_hits += 1;
            self.metrics.total_searches += 1;
            self.update_avg(results.len());
            self.search_history.push(SearchRecord {
                query: self.maybe_redact(query),
                provider: self.config.provider.clone(),
                results_count: results.len(),
                cached: true,
                timestamp: now,
            });
            return Ok(results);
        }

        self.metrics.cache_misses += 1;
        self.rate_limiter.record_request(now);

        // Simulate search execution (real impl would HTTP call the provider)
        let mut results = self.execute_provider_search(query)?;

        // Classify results
        for r in &mut results {
            r.content_type = Self::classify_result(&r.url, &r.title);
        }

        // Truncate to max_results
        results.truncate(self.config.max_results as usize);

        // Cache the results
        self.cache.put(query, results.clone(), now);

        self.metrics.total_searches += 1;
        self.update_avg(results.len());
        self.search_history.push(SearchRecord {
            query: self.maybe_redact(query),
            provider: self.config.provider.clone(),
            results_count: results.len(),
            cached: false,
            timestamp: now,
        });

        Ok(results)
    }

    /// Execute a search through a live [`SearchBackend`] over the network.
    ///
    /// Reuses the cache, rate limiter, classifier, and metrics pipeline from the
    /// sync `search` method. This is the entry point real callers (Tauri, REPL)
    /// should use once a backend is configured.
    pub async fn search_async(
        &mut self,
        query: &str,
        backend: &dyn crate::web_grounding_backend::SearchBackend,
    ) -> Result<Vec<SearchResult>, String> {
        if query.trim().is_empty() {
            return Err("Search query must not be empty".to_string());
        }

        let now = self.now;

        if !self.rate_limiter.can_request(now) {
            self.metrics.rate_limited_count += 1;
            return Err("Rate limit exceeded — try again shortly".to_string());
        }

        if let Some(cached) = self.cache.get(query, now) {
            let results = cached.to_vec();
            self.metrics.cache_hits += 1;
            self.metrics.total_searches += 1;
            self.update_avg(results.len());
            self.search_history.push(SearchRecord {
                query: self.maybe_redact(query),
                provider: self.config.provider.clone(),
                results_count: results.len(),
                cached: true,
                timestamp: now,
            });
            return Ok(results);
        }

        self.metrics.cache_misses += 1;
        self.rate_limiter.record_request(now);

        let mut results = backend.search(query, &self.config).await?;

        for r in &mut results {
            r.content_type = Self::classify_result(&r.url, &r.title);
        }
        results.truncate(self.config.max_results as usize);
        self.cache.put(query, results.clone(), now);

        self.metrics.total_searches += 1;
        self.update_avg(results.len());
        self.search_history.push(SearchRecord {
            query: self.maybe_redact(query),
            provider: self.config.provider.clone(),
            results_count: results.len(),
            cached: false,
            timestamp: now,
        });

        Ok(results)
    }

    /// Execute a search with context-aware relevance ranking.
    pub fn search_with_context(
        &mut self,
        query: &str,
        context: &str,
    ) -> Result<Vec<SearchResult>, String> {
        let mut results = self.search(query)?;
        Self::rank_results(&mut results, context);
        Ok(results)
    }

    /// Adjust relevance scores based on how well results relate to the context.
    pub fn rank_results(results: &mut [SearchResult], context: &str) {
        let ctx_lower = context.to_lowercase();
        let ctx_words: Vec<&str> = ctx_lower.split_whitespace().collect();

        for result in results.iter_mut() {
            let title_lower = result.title.to_lowercase();
            let snippet_lower = result.snippet.to_lowercase();

            // Word overlap scoring
            let mut overlap = 0usize;
            for word in &ctx_words {
                if word.len() < 3 {
                    continue;
                }
                if title_lower.contains(word) {
                    overlap += 2;
                }
                if snippet_lower.contains(word) {
                    overlap += 1;
                }
            }
            let word_score = (overlap as f64 * 0.03).min(0.4);

            // Content type boost
            let type_boost = result.content_type.boost();

            result.relevance_score = (result.relevance_score + word_score + type_boost).min(1.0);
        }

        // Sort descending by relevance
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Record a citation for a search result used by the agent.
    /// Returns the citation ID.
    pub fn add_citation(&mut self, result: &SearchResult, used_for: &str) -> String {
        let id = format!("cite-{}", self.citations.len() + 1);
        self.citations.push(Citation {
            id: id.clone(),
            url: result.url.clone(),
            title: result.title.clone(),
            used_in_context: used_for.to_string(),
            result_index: self.citations.len(),
            timestamp: self.now,
        });
        self.metrics.total_citations += 1;
        id
    }

    /// Get all recorded citations.
    pub fn get_citations(&self) -> &[Citation] {
        &self.citations
    }

    /// Format citations as a markdown reference list.
    pub fn format_citations(&self) -> String {
        if self.citations.is_empty() {
            return String::from("No citations recorded.");
        }
        let mut out = String::from("## References\n\n");
        for (i, c) in self.citations.iter().enumerate() {
            out.push_str(&format!(
                "{}. [{}]({}) — {}\n",
                i + 1,
                c.title,
                c.url,
                c.used_in_context
            ));
        }
        out
    }

    /// Classify a search result by URL and title heuristics.
    pub fn classify_result(url: &str, title: &str) -> ResultContentType {
        let url_lower = url.to_lowercase();
        let title_lower = title.to_lowercase();

        if url_lower.contains("stackoverflow.com") || url_lower.contains("stackexchange.com") {
            return ResultContentType::StackOverflow;
        }
        if url_lower.contains("github.com") {
            return ResultContentType::GitHubRepo;
        }
        if url_lower.contains("readthedocs") || url_lower.contains("devdocs.io") {
            return ResultContentType::Documentation;
        }
        if url_lower.contains("docs.") || url_lower.contains("/docs/") || url_lower.contains("/documentation/") || url_lower.contains("doc.rust-lang.org") {
            return ResultContentType::OfficialDocs;
        }
        if title_lower.contains("tutorial") || title_lower.contains("getting started") || title_lower.contains("how to") {
            return ResultContentType::Tutorial;
        }
        if url_lower.contains("reddit.com")
            || url_lower.contains("discourse")
            || url_lower.contains("forum")
        {
            return ResultContentType::Forum;
        }
        if url_lower.contains("medium.com")
            || url_lower.contains("dev.to")
            || url_lower.contains("blog")
            || url_lower.contains("hashnode")
        {
            return ResultContentType::BlogPost;
        }

        ResultContentType::Unknown
    }

    // ── internal helpers ────────────────────────────────────────────────

    /// Simulated provider search. In production this would HTTP-call the
    /// configured provider. Here we return an empty vec so the engine is
    /// fully testable without network access — tests inject results via cache.
    fn execute_provider_search(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        // Validate API key requirement
        if self.config.provider.requires_api_key() && self.config.api_key.is_none() {
            return Err(format!(
                "{:?} provider requires an API key",
                self.config.provider
            ));
        }

        // In a real implementation, build HTTP request per-provider here.
        // For now, return synthetic results that include the query for testing.
        let _endpoint = self
            .config
            .base_url
            .as_deref()
            .unwrap_or_else(|| self.config.provider.default_endpoint());

        // Return empty — callers should use cache pre-population for tests
        // or plug in a real HTTP backend.
        let _ = query;
        Ok(Vec::new())
    }

    fn maybe_redact(&self, query: &str) -> String {
        if self.config.privacy_mode {
            format!("[redacted-{}chars]", query.len())
        } else {
            query.to_string()
        }
    }

    fn update_avg(&mut self, count: usize) {
        let total = self.metrics.total_searches as f64;
        let prev = self.metrics.avg_results_per_search;
        // Running average
        if total <= 1.0 {
            self.metrics.avg_results_per_search = count as f64;
        } else {
            self.metrics.avg_results_per_search =
                prev + (count as f64 - prev) / total;
        }
    }
}

// ── Agent tool XML ──────────────────────────────────────────────────────────

/// Generate the XML tool definition for `search_web` that can be injected into
/// agent system prompts.
pub fn search_web_tool_xml() -> String {
    r#"<tool name="search_web">
  <description>
    Search the web for current information, documentation, and code examples.
    Use this when the user asks about recent events, needs up-to-date documentation,
    or when your training data may be outdated. Results include titles, URLs, and
    snippets. Citations are automatically tracked.
  </description>
  <parameters>
    <parameter name="query" type="string" required="true">
      The search query string. Be specific and include relevant keywords.
    </parameter>
    <parameter name="context" type="string" required="false">
      Optional context about what the agent is working on, used to improve
      relevance ranking of results.
    </parameter>
    <parameter name="filters" type="array" required="false">
      Optional filters: site:example.com, filetype:pdf, exclude:-site:example.com
    </parameter>
  </parameters>
</tool>"#
        .to_string()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> SearchConfig {
        SearchConfig {
            provider: SearchProvider::DuckDuckGo,
            api_key: None,
            base_url: None,
            max_results: 5,
            cache_ttl_secs: 3600,
            rate_limit_per_min: 30,
            privacy_mode: false,
        }
    }

    fn sample_result(title: &str, url: &str, snippet: &str, score: f64) -> SearchResult {
        SearchResult {
            title: title.to_string(),
            url: url.to_string(),
            snippet: snippet.to_string(),
            source: SearchProvider::DuckDuckGo,
            relevance_score: score,
            fetched_at: 1000,
            content_type: ResultContentType::Unknown,
        }
    }

    // ── SearchProvider tests ────────────────────────────────────────────

    #[test]
    fn test_provider_requires_api_key() {
        assert!(SearchProvider::Google.requires_api_key());
        assert!(SearchProvider::Bing.requires_api_key());
        assert!(SearchProvider::Brave.requires_api_key());
        assert!(SearchProvider::Tavily.requires_api_key());
        assert!(!SearchProvider::DuckDuckGo.requires_api_key());
        assert!(!SearchProvider::SearXNG.requires_api_key());
    }

    #[test]
    fn test_provider_default_endpoints() {
        assert!(SearchProvider::Google.default_endpoint().contains("googleapis"));
        assert!(SearchProvider::Bing.default_endpoint().contains("bing.microsoft"));
        assert!(SearchProvider::Brave.default_endpoint().contains("brave.com"));
        assert!(SearchProvider::SearXNG.default_endpoint().contains("localhost"));
        assert!(SearchProvider::Tavily.default_endpoint().contains("tavily"));
        assert!(SearchProvider::DuckDuckGo.default_endpoint().contains("duckduckgo"));
    }

    // ── SearchConfig tests ──────────────────────────────────────────────

    #[test]
    fn test_config_defaults() {
        let cfg = SearchConfig::default();
        assert_eq!(cfg.provider, SearchProvider::DuckDuckGo);
        assert_eq!(cfg.max_results, 5);
        assert_eq!(cfg.cache_ttl_secs, 3600);
        assert_eq!(cfg.rate_limit_per_min, 30);
        assert!(!cfg.privacy_mode);
        assert!(cfg.api_key.is_none());
        assert!(cfg.base_url.is_none());
    }

    // ── SearchQuery tests ───────────────────────────────────────────────

    #[test]
    fn test_query_effective_simple() {
        let q = SearchQuery {
            query: "rust async".to_string(),
            context: None,
            language: None,
            filters: vec![],
        };
        assert_eq!(q.effective_query(), "rust async");
    }

    #[test]
    fn test_query_effective_with_filters() {
        let q = SearchQuery {
            query: "tokio runtime".to_string(),
            context: None,
            language: Some("en".to_string()),
            filters: vec![
                SearchFilter::Site("docs.rs".to_string()),
                SearchFilter::ExcludeSite("reddit.com".to_string()),
                SearchFilter::FileType("pdf".to_string()),
            ],
        };
        let eff = q.effective_query();
        assert!(eff.contains("site:docs.rs"));
        assert!(eff.contains("-site:reddit.com"));
        assert!(eff.contains("filetype:pdf"));
        assert!(eff.contains("lang:en"));
    }

    #[test]
    fn test_query_date_range_filter_not_in_string() {
        let q = SearchQuery {
            query: "test".to_string(),
            context: None,
            language: None,
            filters: vec![SearchFilter::DateRange(100, 200)],
        };
        // DateRange is API-level, not in the query string
        assert_eq!(q.effective_query(), "test");
    }

    // ── Classification tests ────────────────────────────────────────────

    #[test]
    fn test_classify_stackoverflow() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://stackoverflow.com/q/123", "Some question"),
            ResultContentType::StackOverflow
        );
    }

    #[test]
    fn test_classify_github() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://github.com/tokio-rs/tokio", "Tokio repo"),
            ResultContentType::GitHubRepo
        );
    }

    #[test]
    fn test_classify_official_docs() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://docs.python.org/3/", "Python docs"),
            ResultContentType::OfficialDocs
        );
    }

    #[test]
    fn test_classify_documentation() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://serde.readthedocs.io/en/latest/", "Serde docs"),
            ResultContentType::Documentation
        );
    }

    #[test]
    fn test_classify_tutorial() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://example.com/learn", "How to build a REST API"),
            ResultContentType::Tutorial
        );
    }

    #[test]
    fn test_classify_forum() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://reddit.com/r/rust", "Rust subreddit"),
            ResultContentType::Forum
        );
    }

    #[test]
    fn test_classify_blog() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://medium.com/@author/post", "My article"),
            ResultContentType::BlogPost
        );
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://example.com/page", "Stuff"),
            ResultContentType::Unknown
        );
    }

    // ── Cache tests ─────────────────────────────────────────────────────

    #[test]
    fn test_cache_put_and_get() {
        let mut cache = SearchCache::new(10, 3600);
        let results = vec![sample_result("T", "http://x.com", "snip", 0.5)];
        cache.put("rust", results.clone(), 1000);

        let got = cache.get("rust", 1000);
        assert!(got.is_some());
        assert_eq!(got.unwrap().len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = SearchCache::new(10, 3600);
        assert!(cache.get("nope", 1000).is_none());
    }

    #[test]
    fn test_cache_ttl_expiry() {
        let mut cache = SearchCache::new(10, 100);
        cache.put("q", vec![], 1000);
        // Within TTL
        assert!(cache.get("q", 1050).is_some());
        // Past TTL
        assert!(cache.get("q", 1200).is_none());
    }

    #[test]
    fn test_cache_evict_expired() {
        let mut cache = SearchCache::new(10, 100);
        cache.put("old", vec![], 500);
        cache.put("new", vec![], 1000);
        let evicted = cache.evict_expired(1050);
        assert_eq!(evicted, 1); // "old" evicted
        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = SearchCache::new(10, 3600);
        cache.put("a", vec![], 100);
        cache.put("b", vec![], 100);
        cache.clear();
        assert_eq!(cache.entries.len(), 0);
    }

    #[test]
    fn test_cache_max_entries_eviction() {
        let mut cache = SearchCache::new(2, 3600);
        cache.put("first", vec![], 100);
        cache.put("second", vec![], 200);
        cache.put("third", vec![], 300);
        // "first" should have been evicted (oldest)
        assert_eq!(cache.entries.len(), 2);
        assert!(cache.get("first", 300).is_none());
        assert!(cache.get("second", 300).is_some());
        assert!(cache.get("third", 300).is_some());
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = SearchCache::new(10, 3600);
        cache.put("q", vec![], 1000);
        let _ = cache.get("q", 1000); // hit
        let _ = cache.get("miss", 1000); // miss
        let stats = cache.stats();
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(stats.total_entries, 1);
    }

    #[test]
    fn test_cache_stats_no_requests() {
        let cache = SearchCache::new(10, 3600);
        let stats = cache.stats();
        assert_eq!(stats.hit_rate, 0.0);
    }

    // ── Rate limiter tests ──────────────────────────────────────────────

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(5);
        assert!(limiter.can_request(1000));
    }

    #[test]
    fn test_rate_limiter_blocks_when_exhausted() {
        let mut limiter = RateLimiter::new(2);
        limiter.record_request(1000);
        limiter.record_request(1001);
        assert!(!limiter.can_request(1002));
    }

    #[test]
    fn test_rate_limiter_window_slides() {
        let mut limiter = RateLimiter::new(2);
        limiter.record_request(1000);
        limiter.record_request(1001);
        assert!(!limiter.can_request(1002));
        // After 60s the window slides
        assert!(limiter.can_request(1062));
    }

    #[test]
    fn test_rate_limiter_remaining() {
        let mut limiter = RateLimiter::new(5);
        assert_eq!(limiter.remaining(1000), 5);
        limiter.record_request(1000);
        assert_eq!(limiter.remaining(1000), 4);
        limiter.record_request(1001);
        limiter.record_request(1002);
        assert_eq!(limiter.remaining(1003), 2);
    }

    // ── Engine: search tests ────────────────────────────────────────────

    #[test]
    fn test_engine_empty_query_error() {
        let mut engine = WebGroundingEngine::new(sample_config());
        assert!(engine.search("").is_err());
        assert!(engine.search("   ").is_err());
    }

    #[test]
    fn test_engine_search_basic() {
        let mut engine = WebGroundingEngine::new(sample_config());
        engine.now = 1000;
        let results = engine.search("rust async").unwrap();
        // DuckDuckGo doesn't need API key; returns empty from stub
        assert!(results.is_empty());
        assert_eq!(engine.metrics.total_searches, 1);
    }

    #[test]
    fn test_engine_api_key_required() {
        let mut cfg = sample_config();
        cfg.provider = SearchProvider::Google;
        cfg.api_key = None;
        let mut engine = WebGroundingEngine::new(cfg);
        engine.now = 1000;
        let err = engine.search("test").unwrap_err();
        assert!(err.contains("API key"));
    }

    #[test]
    fn test_engine_search_returns_cached() {
        let mut engine = WebGroundingEngine::new(sample_config());
        engine.now = 1000;
        let results = vec![sample_result("Cached", "http://c.com", "s", 0.9)];
        engine.cache.put("rust", results.clone(), 1000);
        let got = engine.search("rust").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "Cached");
        assert_eq!(engine.metrics.cache_hits, 1);
    }

    #[test]
    fn test_engine_rate_limited() {
        let mut cfg = sample_config();
        cfg.rate_limit_per_min = 1;
        let mut engine = WebGroundingEngine::new(cfg);
        engine.now = 1000;
        // First search OK
        let _ = engine.search("a");
        // Second should be rate limited
        let err = engine.search("b").unwrap_err();
        assert!(err.contains("Rate limit"));
        assert_eq!(engine.metrics.rate_limited_count, 1);
    }

    // ── Ranking tests ───────────────────────────────────────────────────

    #[test]
    fn test_rank_results_context_boost() {
        let mut results = vec![
            sample_result("Unrelated page", "http://a.com", "nothing relevant", 0.5),
            sample_result(
                "Rust async guide",
                "http://b.com",
                "learn about async and await in rust",
                0.5,
            ),
        ];
        WebGroundingEngine::rank_results(&mut results, "rust async programming");
        // The second result should be ranked higher
        assert!(results[0].relevance_score >= results[1].relevance_score);
        assert!(results[0].title.contains("Rust"));
    }

    #[test]
    fn test_rank_results_content_type_boost() {
        let mut results = vec![
            SearchResult {
                content_type: ResultContentType::Unknown,
                ..sample_result("A", "http://a.com", "info", 0.5)
            },
            SearchResult {
                content_type: ResultContentType::OfficialDocs,
                ..sample_result("B", "http://b.com", "info", 0.5)
            },
        ];
        WebGroundingEngine::rank_results(&mut results, "something");
        assert!(results[0].relevance_score > results[1].relevance_score);
        assert_eq!(results[0].content_type, ResultContentType::OfficialDocs);
    }

    #[test]
    fn test_rank_results_capped_at_one() {
        let mut results = vec![SearchResult {
            content_type: ResultContentType::OfficialDocs,
            ..sample_result(
                "Rust Rust Rust",
                "http://a.com",
                "rust rust rust rust rust rust rust",
                0.95,
            )
        }];
        WebGroundingEngine::rank_results(&mut results, "rust rust rust rust rust");
        assert!(results[0].relevance_score <= 1.0);
    }

    // ── Citation tests ──────────────────────────────────────────────────

    #[test]
    fn test_add_citation() {
        let mut engine = WebGroundingEngine::new(sample_config());
        engine.now = 2000;
        let result = sample_result("Title", "http://x.com", "snip", 0.8);
        let id = engine.add_citation(&result, "used for context");
        assert_eq!(id, "cite-1");
        assert_eq!(engine.citations.len(), 1);
        assert_eq!(engine.citations[0].url, "http://x.com");
        assert_eq!(engine.citations[0].used_in_context, "used for context");
        assert_eq!(engine.metrics.total_citations, 1);
    }

    #[test]
    fn test_multiple_citations() {
        let mut engine = WebGroundingEngine::new(sample_config());
        let r1 = sample_result("A", "http://a.com", "a", 0.5);
        let r2 = sample_result("B", "http://b.com", "b", 0.6);
        let id1 = engine.add_citation(&r1, "first");
        let id2 = engine.add_citation(&r2, "second");
        assert_eq!(id1, "cite-1");
        assert_eq!(id2, "cite-2");
        assert_eq!(engine.get_citations().len(), 2);
    }

    #[test]
    fn test_format_citations_empty() {
        let engine = WebGroundingEngine::new(sample_config());
        assert_eq!(engine.format_citations(), "No citations recorded.");
    }

    #[test]
    fn test_format_citations_markdown() {
        let mut engine = WebGroundingEngine::new(sample_config());
        let r = sample_result("Docs", "https://docs.rs", "snip", 0.9);
        engine.add_citation(&r, "API reference");
        let md = engine.format_citations();
        assert!(md.contains("## References"));
        assert!(md.contains("[Docs](https://docs.rs)"));
        assert!(md.contains("API reference"));
    }

    // ── Privacy mode tests ──────────────────────────────────────────────

    #[test]
    fn test_privacy_mode_redacts_history() {
        let mut cfg = sample_config();
        cfg.privacy_mode = true;
        let mut engine = WebGroundingEngine::new(cfg);
        engine.now = 1000;
        let _ = engine.search("secret query");
        assert!(engine.search_history[0].query.contains("[redacted-"));
        assert!(!engine.search_history[0].query.contains("secret"));
    }

    #[test]
    fn test_non_privacy_mode_preserves_query() {
        let mut engine = WebGroundingEngine::new(sample_config());
        engine.now = 1000;
        let _ = engine.search("visible query");
        assert_eq!(engine.search_history[0].query, "visible query");
    }

    // ── Search history & metrics tests ──────────────────────────────────

    #[test]
    fn test_search_history_recorded() {
        let mut engine = WebGroundingEngine::new(sample_config());
        engine.now = 500;
        let _ = engine.search("first");
        engine.now = 600;
        let _ = engine.search("second");
        assert_eq!(engine.search_history.len(), 2);
        assert_eq!(engine.search_history[0].timestamp, 500);
        assert_eq!(engine.search_history[1].timestamp, 600);
    }

    #[test]
    fn test_metrics_tracking() {
        let mut engine = WebGroundingEngine::new(sample_config());
        engine.now = 1000;
        // Pre-populate cache for one query
        engine.cache.put("cached", vec![sample_result("X", "http://x.com", "s", 0.5)], 1000);
        let _ = engine.search("cached"); // cache hit
        let _ = engine.search("fresh"); // cache miss
        assert_eq!(engine.metrics.total_searches, 2);
        assert_eq!(engine.metrics.cache_hits, 1);
        assert_eq!(engine.metrics.cache_misses, 1);
    }

    #[test]
    fn test_grounding_metrics_default() {
        let m = GroundingMetrics::default();
        assert_eq!(m.total_searches, 0);
        assert_eq!(m.cache_hits, 0);
        assert_eq!(m.total_citations, 0);
        assert_eq!(m.avg_results_per_search, 0.0);
    }

    // ── Tool XML generation ─────────────────────────────────────────────

    #[test]
    fn test_tool_xml_contains_name() {
        let xml = search_web_tool_xml();
        assert!(xml.contains("search_web"));
    }

    #[test]
    fn test_tool_xml_contains_parameters() {
        let xml = search_web_tool_xml();
        assert!(xml.contains("query"));
        assert!(xml.contains("context"));
        assert!(xml.contains("filters"));
        assert!(xml.contains("required=\"true\""));
    }

    #[test]
    fn test_tool_xml_well_formed() {
        let xml = search_web_tool_xml();
        assert!(xml.starts_with("<tool"));
        assert!(xml.ends_with("</tool>"));
    }

    // ── Search with context ─────────────────────────────────────────────

    #[test]
    fn test_search_with_context() {
        let mut engine = WebGroundingEngine::new(sample_config());
        engine.now = 1000;
        engine.cache.put(
            "tokio",
            vec![
                sample_result("Tokio Docs", "https://docs.rs/tokio", "async runtime", 0.5),
                sample_result("Random post", "https://blog.com/x", "unrelated content", 0.5),
            ],
            1000,
        );
        let results = engine.search_with_context("tokio", "async runtime rust").unwrap();
        assert_eq!(results.len(), 2);
        // The docs result should be ranked higher due to keyword overlap
        assert!(results[0].title.contains("Tokio"));
    }

    // ── Serialization round-trip ────────────────────────────────────────

    #[test]
    fn test_search_result_serde_roundtrip() {
        let r = sample_result("Test", "http://t.com", "snip", 0.7);
        let json = serde_json::to_string(&r).unwrap();
        let r2: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let cfg = sample_config();
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: SearchConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, cfg2);
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[test]
    fn test_content_type_boost_ordering() {
        // OfficialDocs should have the highest boost
        assert!(ResultContentType::OfficialDocs.boost() > ResultContentType::BlogPost.boost());
        assert!(ResultContentType::StackOverflow.boost() > ResultContentType::Forum.boost());
        assert_eq!(ResultContentType::Unknown.boost(), 0.0);
    }

    #[test]
    fn test_classify_stackexchange() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://math.stackexchange.com/q/1", "Math"),
            ResultContentType::StackOverflow
        );
    }

    #[test]
    fn test_classify_devto() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://dev.to/user/post", "My Post"),
            ResultContentType::BlogPost
        );
    }

    #[test]
    fn test_classify_discourse_forum() {
        assert_eq!(
            WebGroundingEngine::classify_result("https://discourse.example.com/t/123", "Topic"),
            ResultContentType::Forum
        );
    }

    #[test]
    fn test_rate_limiter_prunes_old_entries() {
        let mut limiter = RateLimiter::new(100);
        for i in 0..50 {
            limiter.record_request(i);
        }
        // After recording at t=1000, old entries should be pruned
        limiter.record_request(1000);
        assert!(limiter.requests.len() < 50);
    }

    #[test]
    fn test_engine_custom_base_url() {
        let cfg = SearchConfig {
            provider: SearchProvider::SearXNG,
            base_url: Some("https://my-searx.local/search".to_string()),
            ..sample_config()
        };
        let engine = WebGroundingEngine::new(cfg);
        assert_eq!(
            engine.config.base_url.as_deref(),
            Some("https://my-searx.local/search")
        );
    }
}
