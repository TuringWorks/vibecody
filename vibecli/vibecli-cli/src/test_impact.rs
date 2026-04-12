#![allow(dead_code)]
//! Test impact analysis — maps changed source files to affected test targets,
//! enabling "run only affected tests" mode.
//!
//! Works language-agnostically using symbol-import graph traversal.
//! Matches GitHub Copilot Workspace v2's test impact analysis.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Language detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Other(String),
}

impl Language {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => Language::Rust,
            Some("ts") | Some("tsx") => Language::TypeScript,
            Some("js") | Some("jsx") | Some("mjs") => Language::JavaScript,
            Some("py") => Language::Python,
            Some("go") => Language::Go,
            Some(ext) => Language::Other(ext.to_string()),
            None => Language::Other(String::new()),
        }
    }

    pub fn test_patterns(&self) -> Vec<&'static str> {
        match self {
            Language::Rust => vec!["#[test]", "#[cfg(test)]", "mod tests"],
            Language::TypeScript | Language::JavaScript => {
                vec!["describe(", "it(", "test(", ".spec.", ".test."]
            }
            Language::Python => vec!["def test_", "class Test", "unittest.TestCase"],
            Language::Go => vec!["func Test", "_test.go"],
            Language::Other(_) => vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Import graph
// ---------------------------------------------------------------------------

/// A node in the import graph.
#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub path: PathBuf,
    pub language: Language,
    /// Modules this file imports / depends on.
    pub imports: Vec<PathBuf>,
    /// Whether this file contains tests.
    pub has_tests: bool,
}

impl ModuleNode {
    pub fn new(path: impl Into<PathBuf>, language: Language) -> Self {
        Self {
            path: path.into(),
            language,
            imports: vec![],
            has_tests: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Import graph (adjacency list)
// ---------------------------------------------------------------------------

/// Directed import graph: edge A → B means A imports B.
pub struct ImportGraph {
    /// path → node
    nodes: HashMap<PathBuf, ModuleNode>,
    /// Reverse edges: B → {A | A imports B}
    reverse: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl ImportGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Add a module node to the graph.
    pub fn add_node(&mut self, node: ModuleNode) {
        self.nodes.insert(node.path.clone(), node);
    }

    /// Declare that `importer` imports `imported`.
    pub fn add_edge(&mut self, importer: &Path, imported: &Path) {
        if let Some(node) = self.nodes.get_mut(importer) {
            if !node.imports.contains(&imported.to_path_buf()) {
                node.imports.push(imported.to_path_buf());
            }
        }
        self.reverse
            .entry(imported.to_path_buf())
            .or_default()
            .insert(importer.to_path_buf());
    }

    /// Mark a path as containing tests.
    pub fn mark_has_tests(&mut self, path: &Path) {
        if let Some(node) = self.nodes.get_mut(path) {
            node.has_tests = true;
        }
    }

    /// BFS: return all files that transitively import `changed`.
    pub fn reverse_reachable(&self, changed: &Path) -> HashSet<PathBuf> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(changed.to_path_buf());

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            if let Some(importers) = self.reverse.get(&current) {
                for importer in importers {
                    queue.push_back(importer.clone());
                }
            }
        }

        visited.remove(changed);
        visited
    }

    /// Return all test files that are reachable from a changed file.
    pub fn affected_tests(&self, changed: &Path) -> HashSet<PathBuf> {
        let reachable = self.reverse_reachable(changed);
        let mut tests = HashSet::new();

        // The changed file itself might be a test file.
        if let Some(node) = self.nodes.get(changed) {
            if node.has_tests {
                tests.insert(changed.to_path_buf());
            }
        }

        for path in &reachable {
            if let Some(node) = self.nodes.get(path) {
                if node.has_tests {
                    tests.insert(path.clone());
                }
            }
        }

        tests
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.nodes.values().map(|n| n.imports.len()).sum()
    }
}

impl Default for ImportGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Test impact analyser
// ---------------------------------------------------------------------------

/// Result of a test impact analysis run.
#[derive(Debug, Clone)]
pub struct ImpactReport {
    /// Changed source files that were analysed.
    pub changed_files: Vec<PathBuf>,
    /// Test files that need to be re-run.
    pub affected_tests: Vec<PathBuf>,
    /// Files that were not reachable from any changed file.
    pub unaffected_tests: Vec<PathBuf>,
    /// All test files in the graph.
    pub total_tests: usize,
}

impl ImpactReport {
    pub fn reduction_pct(&self) -> f64 {
        if self.total_tests == 0 {
            return 100.0;
        }
        let unaffected = self.total_tests - self.affected_tests.len();
        unaffected as f64 / self.total_tests as f64 * 100.0
    }

    pub fn needs_full_run(&self) -> bool {
        self.affected_tests.len() == self.total_tests
    }
}

pub struct TestImpactAnalyser {
    graph: ImportGraph,
}

impl TestImpactAnalyser {
    pub fn new(graph: ImportGraph) -> Self {
        Self { graph }
    }

    /// Analyse the impact of `changed_files` and return an `ImpactReport`.
    pub fn analyse(&self, changed_files: &[PathBuf]) -> ImpactReport {
        let all_test_files: HashSet<PathBuf> = self
            .graph
            .nodes
            .values()
            .filter(|n| n.has_tests)
            .map(|n| n.path.clone())
            .collect();

        let mut affected: HashSet<PathBuf> = HashSet::new();
        for f in changed_files {
            let tests = self.graph.affected_tests(f);
            affected.extend(tests);
        }

        let unaffected: Vec<PathBuf> = all_test_files
            .iter()
            .filter(|t| !affected.contains(*t))
            .cloned()
            .collect();

        let mut affected_sorted: Vec<PathBuf> = affected.into_iter().collect();
        affected_sorted.sort();

        ImpactReport {
            changed_files: changed_files.to_vec(),
            affected_tests: affected_sorted,
            unaffected_tests: unaffected,
            total_tests: all_test_files.len(),
        }
    }

    pub fn graph(&self) -> &ImportGraph {
        &self.graph
    }
}

// ---------------------------------------------------------------------------
// Simple Rust import extractor (heuristic, no full parse)
// ---------------------------------------------------------------------------

/// Extract `use crate::...` and `mod ...;` style imports from Rust source.
pub fn extract_rust_imports(source: &str) -> Vec<String> {
    let mut imports = vec![];
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("use ") {
            let path = rest.trim_end_matches(';').trim();
            imports.push(path.to_string());
        } else if let Some(rest) = trimmed.strip_prefix("mod ") {
            let name = rest.trim_end_matches(';').trim_end_matches('{').trim();
            imports.push(format!("mod::{name}"));
        }
    }
    imports
}

/// Check whether a Rust source file contains test code.
pub fn has_rust_tests(source: &str) -> bool {
    source.contains("#[test]") || source.contains("#[cfg(test)]")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn build_graph() -> ImportGraph {
        // lib.rs → utils.rs
        // main.rs → lib.rs
        // tests/lib_test.rs imports lib.rs (has tests)
        // tests/util_test.rs imports utils.rs (has tests)
        let mut g = ImportGraph::new();

        g.add_node(ModuleNode::new("src/utils.rs", Language::Rust));
        g.add_node(ModuleNode::new("src/lib.rs", Language::Rust));
        g.add_node(ModuleNode::new("src/main.rs", Language::Rust));
        let mut test_lib = ModuleNode::new("tests/lib_test.rs", Language::Rust);
        test_lib.has_tests = true;
        let mut test_util = ModuleNode::new("tests/util_test.rs", Language::Rust);
        test_util.has_tests = true;
        g.add_node(test_lib);
        g.add_node(test_util);

        g.add_edge(&p("src/lib.rs"), &p("src/utils.rs"));
        g.add_edge(&p("src/main.rs"), &p("src/lib.rs"));
        g.add_edge(&p("tests/lib_test.rs"), &p("src/lib.rs"));
        g.add_edge(&p("tests/util_test.rs"), &p("src/utils.rs"));

        g
    }

    #[test]
    fn test_graph_node_and_edge_count() {
        let g = build_graph();
        assert_eq!(g.node_count(), 5);
        assert_eq!(g.edge_count(), 4);
    }

    #[test]
    fn test_reverse_reachable_utils() {
        let g = build_graph();
        let reach = g.reverse_reachable(&p("src/utils.rs"));
        assert!(reach.contains(&p("src/lib.rs")));
        assert!(reach.contains(&p("tests/util_test.rs")));
    }

    #[test]
    fn test_affected_tests_utils_changed() {
        let g = build_graph();
        let tests = g.affected_tests(&p("src/utils.rs"));
        assert!(tests.contains(&p("tests/util_test.rs")));
        assert!(tests.contains(&p("tests/lib_test.rs"))); // lib.rs imports utils
    }

    #[test]
    fn test_affected_tests_lib_changed() {
        let g = build_graph();
        let tests = g.affected_tests(&p("src/lib.rs"));
        assert!(tests.contains(&p("tests/lib_test.rs")));
        // util_test doesn't import lib.rs
        assert!(!tests.contains(&p("tests/util_test.rs")));
    }

    #[test]
    fn test_impact_analyser_reduction() {
        let g = build_graph();
        let analyser = TestImpactAnalyser::new(g);
        let report = analyser.analyse(&[p("src/utils.rs")]);
        assert!(!report.affected_tests.is_empty());
        assert_eq!(report.total_tests, 2);
        assert!(report.reduction_pct() >= 0.0);
    }

    #[test]
    fn test_impact_analyser_unaffected_main() {
        let g = build_graph();
        let analyser = TestImpactAnalyser::new(g);
        let report = analyser.analyse(&[p("src/main.rs")]);
        // Nothing imports main.rs, so no tests are affected — both test files unaffected.
        assert_eq!(report.affected_tests.len(), 0);
        assert_eq!(report.unaffected_tests.len(), 2);
    }

    #[test]
    fn test_language_from_path_rust() {
        assert_eq!(Language::from_path(Path::new("src/lib.rs")), Language::Rust);
    }

    #[test]
    fn test_language_from_path_ts() {
        assert_eq!(
            Language::from_path(Path::new("src/app.tsx")),
            Language::TypeScript
        );
    }

    #[test]
    fn test_rust_test_patterns() {
        let patterns = Language::Rust.test_patterns();
        assert!(patterns.contains(&"#[test]"));
    }

    #[test]
    fn test_extract_rust_imports() {
        let src = "use crate::utils::helper;\nmod sub;\n";
        let imports = extract_rust_imports(src);
        assert!(imports.iter().any(|i| i.contains("utils::helper")));
        assert!(imports.iter().any(|i| i.contains("sub")));
    }

    #[test]
    fn test_has_rust_tests_true() {
        let src = "fn main() {}\n#[test]\nfn test_foo() {}";
        assert!(has_rust_tests(src));
    }

    #[test]
    fn test_has_rust_tests_false() {
        let src = "fn main() {}";
        assert!(!has_rust_tests(src));
    }

    #[test]
    fn test_no_affected_tests_for_unknown_file() {
        let g = build_graph();
        let analyser = TestImpactAnalyser::new(g);
        let report = analyser.analyse(&[p("src/totally_new.rs")]);
        assert!(report.affected_tests.is_empty());
    }

    #[test]
    fn test_mark_has_tests() {
        let mut g = ImportGraph::new();
        g.add_node(ModuleNode::new("src/foo.rs", Language::Rust));
        g.mark_has_tests(&p("src/foo.rs"));
        assert!(g.nodes[&p("src/foo.rs")].has_tests);
    }
}
