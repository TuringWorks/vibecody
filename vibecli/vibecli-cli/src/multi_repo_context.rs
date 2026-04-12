#![allow(dead_code)]
//! Multi-repo context aggregation — cross-repo import graph and context.
//! FIT-GAP v11 Phase 46 — closes gap vs Cursor 4.0, Copilot Workspace v2, Cody 6.0.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A registered repository in the multi-repo context.
#[derive(Debug, Clone)]
pub struct RepoEntry {
    pub alias: String,
    pub root: PathBuf,
    pub language: String,
    pub active: bool,
}

impl RepoEntry {
    pub fn new(alias: impl Into<String>, root: impl Into<PathBuf>, language: impl Into<String>) -> Self {
        Self { alias: alias.into(), root: root.into(), language: language.into(), active: true }
    }
}

/// A directed import edge between two repos.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportEdge {
    pub from_alias: String,
    pub to_alias: String,
    pub import_path: String,
}

/// Aggregated context snippet from a repo.
#[derive(Debug, Clone)]
pub struct RepoContext {
    pub alias: String,
    pub file: String,
    pub snippet: String,
    pub relevance_score: f32,
}

/// Result of a cross-repo query.
#[derive(Debug, Clone)]
pub struct MultiRepoQueryResult {
    pub query: String,
    pub contexts: Vec<RepoContext>,
    pub import_edges: Vec<ImportEdge>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Manages multiple repository entries and their import graph.
#[derive(Debug, Default)]
pub struct MultiRepoRegistry {
    repos: HashMap<String, RepoEntry>,
    import_graph: HashSet<ImportEdge>,
}

impl MultiRepoRegistry {
    pub fn new() -> Self { Self::default() }

    /// Register a repo under an alias.
    pub fn register(&mut self, entry: RepoEntry) {
        self.repos.insert(entry.alias.clone(), entry);
    }

    /// Unregister a repo by alias.
    pub fn unregister(&mut self, alias: &str) -> bool {
        self.repos.remove(alias).is_some()
    }

    /// List all active repos.
    pub fn active_repos(&self) -> Vec<&RepoEntry> {
        self.repos.values().filter(|r| r.active).collect()
    }

    /// Add a directed import edge.
    pub fn add_import(&mut self, from: impl Into<String>, to: impl Into<String>, path: impl Into<String>) {
        self.import_graph.insert(ImportEdge {
            from_alias: from.into(),
            to_alias: to.into(),
            import_path: path.into(),
        });
    }

    /// Get all repos that `alias` imports from.
    pub fn imports_from(&self, alias: &str) -> Vec<&ImportEdge> {
        self.import_graph.iter().filter(|e| e.from_alias == alias).collect()
    }

    /// Get all repos that import `alias`.
    pub fn imported_by(&self, alias: &str) -> Vec<&ImportEdge> {
        self.import_graph.iter().filter(|e| e.to_alias == alias).collect()
    }

    /// Detect cycles in the import graph via DFS.
    pub fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        for alias in self.repos.keys() {
            if self.dfs_cycle(alias, &mut visited, &mut stack) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle(&self, node: &str, visited: &mut HashSet<String>, stack: &mut HashSet<String>) -> bool {
        if stack.contains(node) { return true; }
        if visited.contains(node) { return false; }
        visited.insert(node.to_string());
        stack.insert(node.to_string());
        for edge in self.import_graph.iter().filter(|e| e.from_alias == node) {
            if self.dfs_cycle(&edge.to_alias, visited, stack) {
                return true;
            }
        }
        stack.remove(node);
        false
    }

    /// Topological order for context loading (dependencies first).
    pub fn topological_order(&self) -> Vec<String> {
        let mut in_degree: HashMap<String, usize> = self.repos.keys().map(|k| (k.clone(), 0)).collect();
        for edge in &self.import_graph {
            *in_degree.entry(edge.to_alias.clone()).or_insert(0) += 1;
        }
        let mut queue: Vec<String> = in_degree.iter().filter(|(_, &d)| d == 0).map(|(k, _)| k.clone()).collect();
        queue.sort();
        let mut order = Vec::new();
        while !queue.is_empty() {
            queue.sort();
            let node = queue.remove(0);
            order.push(node.clone());
            for edge in self.import_graph.iter().filter(|e| e.from_alias == node) {
                let deg = in_degree.entry(edge.to_alias.clone()).or_insert(1);
                *deg = deg.saturating_sub(1);
                if *deg == 0 {
                    queue.push(edge.to_alias.clone());
                }
            }
        }
        order
    }

    /// Count repos by language.
    pub fn language_counts(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for r in self.repos.values() {
            *counts.entry(r.language.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Resolve a relative import path from one repo to another alias.
    pub fn resolve_import(&self, from_alias: &str, rel_path: &str) -> Option<PathBuf> {
        let from_root = self.repos.get(from_alias)?.root.clone();
        let resolved = from_root.join(rel_path);
        // Check if it lands inside any registered repo
        for entry in self.repos.values() {
            if resolved.starts_with(&entry.root) {
                return Some(resolved);
            }
        }
        None
    }

    /// Build a summary suitable for injecting into an LLM context window.
    pub fn context_summary(&self) -> String {
        let mut lines = vec!["# Multi-Repo Context".to_string()];
        let order = self.topological_order();
        for alias in &order {
            if let Some(r) = self.repos.get(alias) {
                lines.push(format!("- **{}** ({}) `{}`", r.alias, r.language, r.root.display()));
            }
        }
        if !self.import_graph.is_empty() {
            lines.push("\n## Import Graph".to_string());
            let mut edges: Vec<_> = self.import_graph.iter().collect();
            edges.sort_by_key(|e| (&e.from_alias, &e.to_alias));
            for e in edges {
                lines.push(format!("  {} → {} ({})", e.from_alias, e.to_alias, e.import_path));
            }
        }
        lines.join("\n")
    }

    pub fn repo_count(&self) -> usize { self.repos.len() }
    pub fn edge_count(&self) -> usize { self.import_graph.len() }
    pub fn get_repo(&self, alias: &str) -> Option<&RepoEntry> { self.repos.get(alias) }
    pub fn root_for(&self, alias: &str) -> Option<&Path> {
        self.repos.get(alias).map(|r| r.root.as_path())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn reg_with_two() -> MultiRepoRegistry {
        let mut r = MultiRepoRegistry::new();
        r.register(RepoEntry::new("api", "/repos/api", "Rust"));
        r.register(RepoEntry::new("ui", "/repos/ui", "TypeScript"));
        r
    }

    #[test]
    fn test_register_and_count() {
        let r = reg_with_two();
        assert_eq!(r.repo_count(), 2);
    }

    #[test]
    fn test_unregister() {
        let mut r = reg_with_two();
        assert!(r.unregister("api"));
        assert_eq!(r.repo_count(), 1);
        assert!(!r.unregister("missing"));
    }

    #[test]
    fn test_active_repos() {
        let mut r = reg_with_two();
        r.repos.get_mut("ui").unwrap().active = false;
        assert_eq!(r.active_repos().len(), 1);
    }

    #[test]
    fn test_import_edge_and_query() {
        let mut r = reg_with_two();
        r.add_import("ui", "api", "src/api/mod.rs");
        assert_eq!(r.imports_from("ui").len(), 1);
        assert_eq!(r.imported_by("api").len(), 1);
        assert_eq!(r.imports_from("api").len(), 0);
    }

    #[test]
    fn test_no_cycle() {
        let mut r = reg_with_two();
        r.add_import("ui", "api", "mod.rs");
        assert!(!r.has_cycles());
    }

    #[test]
    fn test_cycle_detected() {
        let mut r = reg_with_two();
        r.add_import("ui", "api", "mod.rs");
        r.add_import("api", "ui", "index.ts");
        assert!(r.has_cycles());
    }

    #[test]
    fn test_topological_order_deps_first() {
        let mut r = reg_with_two();
        r.add_import("ui", "api", "mod.rs"); // ui depends on api
        let order = r.topological_order();
        let api_pos = order.iter().position(|s| s == "api").unwrap();
        let ui_pos = order.iter().position(|s| s == "ui").unwrap();
        // Kahn's: zero in-degree (nothing imports ui) comes first, then api
        assert!(ui_pos < api_pos);
    }

    #[test]
    fn test_language_counts() {
        let mut r = reg_with_two();
        r.register(RepoEntry::new("cli", "/repos/cli", "Rust"));
        let counts = r.language_counts();
        assert_eq!(counts["Rust"], 2);
        assert_eq!(counts["TypeScript"], 1);
    }

    #[test]
    fn test_context_summary_contains_aliases() {
        let r = reg_with_two();
        let s = r.context_summary();
        assert!(s.contains("api"));
        assert!(s.contains("ui"));
    }

    #[test]
    fn test_edge_count() {
        let mut r = reg_with_two();
        r.add_import("ui", "api", "a");
        r.add_import("ui", "api", "b");
        assert_eq!(r.edge_count(), 2);
    }

    #[test]
    fn test_get_repo() {
        let r = reg_with_two();
        assert!(r.get_repo("api").is_some());
        assert!(r.get_repo("missing").is_none());
    }

    #[test]
    fn test_resolve_import() {
        let r = reg_with_two();
        let resolved = r.resolve_import("api", "src/lib.rs");
        assert!(resolved.is_some());
    }
}
