//! # `graph_index` — the kodegraph bridge into VibeCody
//!
//! VibeCody's token-reduction substrate. The daemon owns a process-global
//! [`GraphHandle`] wrapping a [`kodegraph`] `CodeGraph`, built in the
//! background on startup and refreshed incrementally. Two consumers read it:
//!
//! 1. **Agent system prompt** — [`render_repo_map_summary`] produces a few
//!    hundred tokens of god-node / community / surprising-edge summary that
//!    replaces the flat directory tree in `build_repo_map` (vibe-ai).
//! 2. **TUI `ContextBuilder`** — [`graph_aware_symbols`] seeds
//!    `with_relevant_symbols` from a blast-radius around task terms.
//!
//! The graph is also exposed over HTTP via `/graph/*` (see `serve.rs`) and
//! `/watch/graph/*` (see `watch_bridge.rs`).
//!
//! ## Scope + known limitations
//!
//! - kodegraph is a dependency of `vibecli-cli` only. `vibe-ai` / `vibe-core`
//!   stay kodegraph-free and receive pre-rendered strings / `SymbolInfo` vecs.
//! - Incremental refresh is **on agent spawn** + explicit `/graph/build`.
//!   There is no daemon-wide file-watcher loop driving source files
//!   (`FileWatcher` is poll-based and only used by `security_review_watch`),
//!   so a background debounce poll is deferred to a follow-up.
//! - Only the tree-sitter backbone is enabled (no `lsp`/`cli`/`mcp` features)
//!   to keep the daemon dep tree light.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::SystemTime;

use kodegraph::analyze::{detect_communities, god_nodes, surprising_edges};
use kodegraph::builder::CodeGraphBuilder;
use kodegraph::incremental::FileHashes;
use kodegraph::model::graph::{CodeGraph, NodeData};
use kodegraph::model::symbol::{
    Language as KgLanguage, Symbol as KgSymbol, SymbolKind as KgSymbolKind,
};
use kodegraph::query::{get_neighbors, query_graph, shortest_path as kodegraph_shortest_path};
use kodegraph::report::render_report as kodegraph_render_report;
use kodegraph::store::{SQLiteStore, Store};

use vibe_core::index::{Language as VcLanguage, SymbolInfo, SymbolKind as VcSymbolKind};

/// Hard cap on the rendered repo-map summary (≈ tokens, 4 chars/token).
const SUMMARY_CHAR_CAP: usize = 1600;
/// Max god nodes / communities / surprising edges listed in the summary.
const SUMMARY_GOD_NODES: usize = 8;
const SUMMARY_COMMUNITIES: usize = 6;
const SUMMARY_COMMUNITY_SAMPLES: usize = 3;
const SUMMARY_SURPRISING: usize = 8;
/// Source extensions we bother hashing for the staleness check.
const SOURCE_EXTS: &[&str] = &["rs", "ts", "tsx", "js", "jsx", "mjs", "py", "pyi", "go"];

/// Lifecycle status of the code graph, surfaced in `/health` + the banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphStatus {
    /// Background build in progress.
    Indexing,
    /// Graph loaded (from SQLite or a completed build) and ready to query.
    Ready,
    /// No graph available (init not called / build failed).
    Disabled,
}

impl GraphStatus {
    /// Lowercase one-word status for logs / health / JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Indexing => "indexing",
            Self::Ready => "ready",
            Self::Disabled => "disabled",
        }
    }
}

/// Report surfaced in `/health`, the startup banner, and `/graph/status`.
#[derive(Debug, Clone)]
pub struct GraphProbeReport {
    /// Lifecycle status.
    pub status: GraphStatus,
    /// Total graph nodes.
    pub node_count: usize,
    /// Total backbone edges.
    pub edge_count: usize,
    /// When the graph was last (re)built, if ever.
    pub last_built_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for GraphProbeReport {
    fn default() -> Self {
        Self {
            status: GraphStatus::Disabled,
            node_count: 0,
            edge_count: 0,
            last_built_at: None,
        }
    }
}

/// Process-global handle around the kodegraph graph + its persistence + probe.
pub struct GraphHandle {
    /// The code graph (None until first build completes).
    pub graph: Arc<RwLock<Option<CodeGraph>>>,
    /// File-hash cache (used by the builder's incremental path).
    pub hashes: Arc<RwLock<FileHashes>>,
    /// mtime+size cache for the cheap staleness check (not persisted).
    mtimes: Arc<RwLock<HashMap<String, (SystemTime, u64)>>>,
    /// SQLite store at `<workspace>/.vibecli/codegraph.db`.
    store: Arc<SQLiteStore>,
    /// Workspace root the graph was built for.
    pub workspace_root: PathBuf,
    /// Persisted DB path.
    pub db_path: PathBuf,
    /// Live probe (updated by build/refresh).
    pub probe: Arc<RwLock<GraphProbeReport>>,
}

static GRAPH: OnceLock<GraphHandle> = OnceLock::new();

/// Initialize (or fetch) the process-global graph handle for `workspace_root`.
///
/// Idempotent — once set, subsequent calls return the existing handle (the
/// `workspace_root` argument is only used the first time). Opens the SQLite
/// store at `<workspace>/.vibecli/codegraph.db` and loads any persisted graph
/// + hash cache into the `RwLock`s. Probe is `Ready` if a graph loaded, else
/// `Disabled` (caller should [`spawn_background_build`] to populate it).
pub fn init_graph_handle(workspace_root: &Path) -> &'static GraphHandle {
    GRAPH.get_or_init(|| {
        let workspace_root = workspace_root.to_path_buf();
        let vibecli_dir = workspace_root.join(".vibecli");
        let _ = std::fs::create_dir_all(&vibecli_dir);
        let db_path = vibecli_dir.join("codegraph.db");

        let store = SQLiteStore::open(&db_path).unwrap_or_else(|e| {
            eprintln!("[vibecli graph] failed to open {}: {e}", db_path.display());
            SQLiteStore::open_memory().unwrap_or_else(|e| {
                eprintln!("[vibecli graph] in-memory fallback failed: {e}");
                panic!("kodegraph store unavailable");
            })
        });

        let (graph, hashes) = match store.load_graph() {
            Ok(Some(g)) => {
                let h = store.load_hashes().unwrap_or_default();
                (Some(g), h)
            }
            Ok(None) => (None, FileHashes::new()),
            Err(e) => {
                eprintln!("[vibecli graph] load failed, starting empty: {e}");
                (None, FileHashes::new())
            }
        };

        let (node_count, edge_count) = graph
            .as_ref()
            .map(|g| (g.node_count(), g.edge_count()))
            .unwrap_or((0, 0));
        let status = if graph.is_some() {
            GraphStatus::Ready
        } else {
            GraphStatus::Disabled
        };

        let probe = GraphProbeReport {
            status,
            node_count,
            edge_count,
            last_built_at: None,
        };

        GraphHandle {
            graph: Arc::new(RwLock::new(graph)),
            hashes: Arc::new(RwLock::new(hashes)),
            mtimes: Arc::new(RwLock::new(HashMap::new())),
            store: Arc::new(store),
            workspace_root,
            db_path,
            probe: Arc::new(RwLock::new(probe)),
        }
    })
}

/// Borrow the process-global graph handle, if [`init_graph_handle`] has run.
pub fn graph_handle() -> Option<&'static GraphHandle> {
    GRAPH.get()
}

/// Build (or rebuild) the graph synchronously, persist it, and update the
/// probe. Returns the new probe on success. Errors are logged and the prior
/// graph (if any) is left intact.
pub fn build_graph_blocking(workspace_root: &Path) -> Result<GraphProbeReport, String> {
    let handle = init_graph_handle(workspace_root);
    do_build(handle)
}

fn do_build(handle: &GraphHandle) -> Result<GraphProbeReport, String> {
    // Mark indexing.
    {
        let mut p = handle.probe.write().unwrap();
        p.status = GraphStatus::Indexing;
    }

    let (graph, hashes) = CodeGraphBuilder::new()
        .scan_dir(&handle.workspace_root)
        .map_err(|e| format!("scan_dir: {e}"))?
        .ignore_dirs([".vibecli", "kodegraph-out"])
        .build()
        .map_err(|e| format!("build: {e}"))?;

    // Persist.
    if let Err(e) = handle.store.save_graph(&graph) {
        eprintln!("[vibecli graph] save_graph failed: {e}");
    }
    if let Err(e) = handle.store.save_hashes(&hashes) {
        eprintln!("[vibecli graph] save_hashes failed: {e}");
    }

    let node_count = graph.node_count();
    let edge_count = graph.edge_count();
    *handle.graph.write().unwrap() = Some(graph);
    *handle.hashes.write().unwrap() = hashes;
    refresh_mtimes(handle);

    let probe = GraphProbeReport {
        status: GraphStatus::Ready,
        node_count,
        edge_count,
        last_built_at: Some(chrono::Utc::now()),
    };
    *handle.probe.write().unwrap() = probe.clone();
    eprintln!("[vibecli graph] built: {node_count} nodes, {edge_count} edges");
    Ok(probe)
}

/// Spawn a non-blocking background build on a dedicated OS thread (tree-
/// sitting parsing is CPU-bound — must not run on the tokio runtime). The
/// probe is set to `Indexing` immediately and `Ready`/error on completion.
pub fn spawn_background_build(workspace_root: PathBuf) {
    // Mark indexing eagerly so /health reflects it before the thread starts.
    if let Some(h) = graph_handle() {
        let mut p = h.probe.write().unwrap();
        p.status = GraphStatus::Indexing;
    }
    std::thread::spawn(move || {
        let h = init_graph_handle(&workspace_root);
        if let Err(e) = do_build(h) {
            eprintln!("[vibecli graph] background build failed: {e}");
            let mut p = h.probe.write().unwrap();
            p.status = GraphStatus::Disabled;
        }
    });
}

/// Cheap staleness check: walk source files, compare mtime+size to the cached
/// table. If any file changed/added/removed, rebuild and return `true`.
///
/// On the first call after a cold start (cache empty), this populates the
/// cache and trusts the persisted graph (returns `false`) — the assumption
/// being that `init_graph_handle` just loaded a fresh graph from SQLite.
pub fn refresh_if_stale(workspace_root: &Path) -> bool {
    let Some(handle) = graph_handle() else {
        return false;
    };
    let _ = workspace_root; // handle already knows its root

    let mut current: HashMap<String, (SystemTime, u64)> = HashMap::new();
    for (rel, meta) in walk_source_files(&handle.workspace_root) {
        current.insert(rel, meta);
    }

    let need_rebuild = {
        let cache = handle.mtimes.read().unwrap();
        if cache.is_empty() {
            // Cold cache: populate now, trust the persisted graph.
            false
        } else {
            files_differ(&cache, &current)
        }
    };

    *handle.mtimes.write().unwrap() = current;

    if need_rebuild {
        eprintln!("[vibecli graph] stale files detected — rebuilding");
        if do_build(handle).is_ok() {
            return true;
        }
    }
    false
}

fn refresh_mtimes(handle: &GraphHandle) {
    let mut cache = handle.mtimes.write().unwrap();
    cache.clear();
    for (rel, meta) in walk_source_files(&handle.workspace_root) {
        cache.insert(rel, meta);
    }
}

fn files_differ(
    cache: &HashMap<String, (SystemTime, u64)>,
    current: &HashMap<String, (SystemTime, u64)>,
) -> bool {
    // Added or modified.
    for (path, meta) in current {
        match cache.get(path) {
            Some(c) if c == meta => continue,
            _ => return true,
        }
    }
    // Deleted.
    current.len() != cache.len()
}

/// Walk source files under `root`, returning `(relative_path, (mtime, size))`.
/// Skips the same ignore set as the builder plus `.vibecli`/`kodegraph-out`.
fn walk_source_files(root: &Path) -> Vec<(String, (SystemTime, u64))> {
    const IGNORED: &[&str] = &[
        "target",
        "node_modules",
        ".git",
        ".next",
        "dist",
        "build",
        ".venv",
        "venv",
        "__pycache__",
        ".mypy_cache",
        "kodegraph-out",
        ".vibecli",
    ];
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().filter_entry(|e| {
        if !e.file_type().is_dir() {
            return true;
        }
        e.file_name()
            .to_str()
            .map(|n| !IGNORED.contains(&n))
            .unwrap_or(true)
    }) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !SOURCE_EXTS.contains(&ext) {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| entry.path().to_string_lossy().into_owned());
        let Ok(meta) = std::fs::metadata(entry.path()) else {
            continue;
        };
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        out.push((rel, (mtime, meta.len())));
    }
    out
}

/// Render a compact (~few-hundred-token) repo-map summary: top god nodes,
/// community structure, and surprising cross-file links. Returns `None` when
/// the graph is empty/unavailable — callers fall back to `build_repo_map`.
pub fn render_repo_map_summary(handle: &GraphHandle) -> Option<String> {
    let graph = handle.graph.read().unwrap();
    let graph = graph.as_ref()?;
    if graph.node_count() == 0 {
        return None;
    }

    let mut s = String::new();
    s.push_str(&format!(
        "Code graph: {} symbols, {} topology edges.\n",
        graph.node_count(),
        graph.edge_count(),
    ));

    let gods = god_nodes(graph, SUMMARY_GOD_NODES);
    if !gods.is_empty() {
        s.push_str("God nodes (highest-coupling keystones):\n");
        for (i, g) in gods.iter().enumerate() {
            s.push_str(&format!(
                "  {}. {} — {} (coupling {})\n",
                i + 1,
                g.name,
                short(&g.file),
                g.coupling
            ));
            if s.len() > SUMMARY_CHAR_CAP {
                break;
            }
        }
    }

    let comms = detect_communities(graph);
    if !comms.is_empty() {
        s.push_str(&format!("Communities ({} detected):\n", comms.len()));
        for (i, c) in comms.iter().take(SUMMARY_COMMUNITIES).enumerate() {
            let samples: Vec<String> = c
                .members
                .iter()
                .filter_map(|id| graph.node(*id).map(NodeData::label))
                .take(SUMMARY_COMMUNITY_SAMPLES)
                .collect();
            s.push_str(&format!(
                "  {}. {} ({} nodes): {}\n",
                i + 1,
                c.label,
                c.members.len(),
                samples.join(", "),
            ));
            if s.len() > SUMMARY_CHAR_CAP {
                break;
            }
        }
    }

    let surprising = surprising_edges(graph);
    if !surprising.is_empty() {
        s.push_str(&format!(
            "Surprising cross-file links ({}):\n",
            surprising.len()
        ));
        for e in surprising.iter().take(SUMMARY_SURPRISING) {
            s.push_str(&format!(
                "  - {} → {} [{}] ({} → {})\n",
                e.from,
                e.to,
                e.kind.as_str(),
                short(&e.from_file),
                short(&e.to_file),
            ));
            if s.len() > SUMMARY_CHAR_CAP {
                break;
            }
        }
    }

    if s.len() > SUMMARY_CHAR_CAP {
        s.truncate(SUMMARY_CHAR_CAP);
        s.push_str("…\n");
    }
    Some(s)
}

fn short(path: &str) -> String {
    // Keep only the last two path segments for brevity.
    let mut iter = path.rsplit('/');
    let last = iter.next().unwrap_or(path);
    match iter.next() {
        Some(p) => format!("{p}/{last}"),
        None => last.to_string(),
    }
}

/// Render the full `GRAPH_REPORT.md` for the graph, or `None` if unavailable.
pub fn render_report(handle: &GraphHandle) -> Option<String> {
    let graph = handle.graph.read().unwrap();
    graph.as_ref().map(kodegraph_render_report)
}

/// Render the current repo-map summary without a staleness refresh (cheap).
/// For daemon fallback paths that already did their work this request.
pub fn render_current_summary() -> Option<String> {
    graph_handle().and_then(render_repo_map_summary)
}

/// Refresh the graph if stale (cheap mtime walk), then render the repo-map
/// summary. For the primary daemon agent-spawn path. `None` when no graph is
/// available (caller falls back to the directory-tree repo map).
pub fn current_summary(workspace_root: &std::path::Path) -> Option<String> {
    refresh_if_stale(workspace_root);
    render_current_summary()
}

// ── query helpers (used by /graph/* routes + /semindex) ─────────────────────
//
// JSON views (`query_value` / `node_value` / `neighbors_value` / `path_value`
// / `blast_value` below) back the HTTP routes; `callers` / `callees` back the
// `/semindex` CLI. The richer typed wrappers (`Subgraph` / `NodeData` /
// `BlastRadius` returns) are intentionally not exposed — callers consume either
// the JSON views or the flat `NodeSummary` / `search_symbols` helpers.

/// Names of symbols that call `name`.
pub fn callers(name: &str) -> Vec<String> {
    let Some(handle) = graph_handle() else {
        return Vec::new();
    };
    let graph = handle.graph.read().unwrap();
    let Some(graph) = graph.as_ref() else {
        return Vec::new();
    };
    let mut out: Vec<String> = graph
        .callers(name)
        .iter()
        .map(|e| e.caller.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Names of symbols called by `name`.
pub fn callees(name: &str) -> Vec<String> {
    let Some(handle) = graph_handle() else {
        return Vec::new();
    };
    let graph = handle.graph.read().unwrap();
    let Some(graph) = graph.as_ref() else {
        return Vec::new();
    };
    let mut out: Vec<String> = graph
        .callees(name)
        .iter()
        .map(|e| e.callee.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

// ── JSON views for /graph/* + /watch/graph/* (HTTP layer never sees kodegraph types) ──
//
// `Subgraph` / `BlastRadius` don't derive `Serialize` and `NodeId` is a petgraph
// index, so these helpers acquire the graph read lock once and render
// `serde_json::Value` directly, mapping every `NodeId` to its node label.

/// Live probe as JSON — `{status, node_count, edge_count, last_built_at?}`.
/// `None` (→ caller returns 503/disabled) when no handle is initialized.
pub fn status_value() -> Option<serde_json::Value> {
    let handle = graph_handle()?;
    let probe = handle.probe.read().unwrap();
    let mut v = serde_json::json!({
        "status": probe.status.as_str(),
        "node_count": probe.node_count,
        "edge_count": probe.edge_count,
    });
    if let Some(ts) = probe.last_built_at {
        if let Ok(map) = serde_json::to_value(ts) {
            v.as_object_mut()
                .unwrap()
                .insert("last_built_at".to_string(), map);
        }
    }
    Some(v)
}

/// `query_graph` result as JSON: `{seeds:[label…], nodes:[NodeData…],
/// edges:[{from,to,kind,provenance}], est_tokens}`. `None` if no graph.
pub fn query_value(query: &str, budget: usize) -> Option<serde_json::Value> {
    let handle = graph_handle()?;
    let graph = handle.graph.read().unwrap();
    let graph = graph.as_ref()?;
    let sg = query_graph(graph, query, budget);
    let label_of = |id: kodegraph::model::graph::NodeId| graph.node(id).map(|nd| nd.label());
    let seeds: Vec<String> = sg.seeds.iter().filter_map(|id| label_of(*id)).collect();
    let nodes: Vec<serde_json::Value> = sg
        .nodes
        .iter()
        .filter_map(|nd| serde_json::to_value(nd).ok())
        .collect();
    let edges: Vec<serde_json::Value> = sg
        .edges
        .iter()
        .filter_map(|(from, to, kind, prov)| {
            Some(serde_json::json!({
                "from": label_of(*from)?,
                "to": label_of(*to)?,
                "kind": kind.as_str(),
                "provenance": serde_json::to_value(prov).unwrap_or(serde_json::Value::Null),
            }))
        })
        .collect();
    Some(serde_json::json!({
        "seeds": seeds,
        "nodes": nodes,
        "edges": edges,
        "est_tokens": sg.est_tokens,
    }))
}

/// A single node's payload as JSON, or `None` if no graph / not found.
pub fn node_value(name: &str) -> Option<serde_json::Value> {
    let handle = graph_handle()?;
    let graph = handle.graph.read().unwrap();
    let graph = graph.as_ref()?;
    kodegraph::query::get_node(graph, name).and_then(|nd| serde_json::to_value(&nd).ok())
}

/// Adjacent nodes as a JSON array of `NodeData`. `None` if no graph.
pub fn neighbors_value(name: &str) -> Option<serde_json::Value> {
    let handle = graph_handle()?;
    let graph = handle.graph.read().unwrap();
    let graph = graph.as_ref()?;
    let arr: Vec<serde_json::Value> = get_neighbors(graph, name)
        .into_iter()
        .filter_map(|nd| serde_json::to_value(&nd).ok())
        .collect();
    Some(serde_json::Value::Array(arr))
}

/// Shortest path as `{path:[label…], hops:N}`. `None` if no graph / no path.
pub fn path_value(from: &str, to: &str) -> Option<serde_json::Value> {
    let handle = graph_handle()?;
    let graph = handle.graph.read().unwrap();
    let graph = graph.as_ref()?;
    kodegraph_shortest_path(graph, from, to).map(|(hops, nodes)| {
        serde_json::json!({
            "path": nodes.iter().map(|nd| nd.label()).collect::<Vec<_>>(),
            "hops": hops,
        })
    })
}

/// Blast radius as `{seed, affected, by_hop:{0:[label…],1:[…]}}`. `None` if no graph.
pub fn blast_value(name: &str, max_hops: usize) -> Option<serde_json::Value> {
    let handle = graph_handle()?;
    let graph = handle.graph.read().unwrap();
    let graph = graph.as_ref()?;
    let br = kodegraph::analyze::blast_radius(graph, name, max_hops);
    let label_of = |id: kodegraph::model::graph::NodeId| graph.node(id).map(|nd| nd.label());
    let mut by_hop = serde_json::Map::new();
    for (hop, ids) in &br.by_hop {
        let labels: Vec<String> = ids.iter().filter_map(|id| label_of(*id)).collect();
        by_hop.insert(
            hop.to_string(),
            serde_json::Value::Array(labels.into_iter().map(serde_json::Value::String).collect()),
        );
    }
    Some(serde_json::json!({
        "seed": br.seed_name,
        "affected": br.affected_count,
        "by_hop": serde_json::Value::Object(by_hop),
    }))
}

// ── REPL `/semindex` helpers (typed, kodegraph-free for the caller) ─────────

/// A flat, serializable node summary for the `/semindex` CLI.
#[derive(Debug, Clone)]
pub struct NodeSummary {
    /// Display label (symbol name / module name / file path).
    pub label: String,
    /// `"symbol"` | `"module"` | `"file"`.
    pub kind: &'static str,
    /// Associated file path, if any.
    pub file: String,
    /// 1-based line for symbols; 0 for coarse nodes.
    pub line: usize,
}

fn node_summary_from(nd: &NodeData) -> NodeSummary {
    let (kind, line) = match nd {
        NodeData::Symbol(s) => ("symbol", s.line_start),
        NodeData::Module { .. } => ("module", 0),
        NodeData::File { .. } => ("file", 0),
    };
    NodeSummary {
        label: nd.label(),
        kind,
        file: nd.file_path().unwrap_or("").to_string(),
        line,
    }
}

/// The node at `name` as a flat summary, or `None` if no graph / not found.
pub fn node_summary(name: &str) -> Option<NodeSummary> {
    let handle = graph_handle()?;
    let graph = handle.graph.read().unwrap();
    let graph = graph.as_ref()?;
    kodegraph::query::get_node(graph, name).map(|nd| node_summary_from(&nd))
}

/// Labels of nodes adjacent to `name` (callers + callees + imports/etc.).
/// Empty when no graph. Used by `/semindex hierarchy` as a lossy stand-in.
pub fn neighbor_labels(name: &str) -> Vec<String> {
    let Some(handle) = graph_handle() else {
        return Vec::new();
    };
    let graph = handle.graph.read().unwrap();
    let Some(graph) = graph.as_ref() else {
        return Vec::new();
    };
    get_neighbors(graph, name)
        .into_iter()
        .map(|nd| nd.label())
        .collect()
}

/// Symbols whose name contains any `query` term, as `(label, file, line)`.
/// A lightweight replacement for the retired `SemanticIndex::search_symbols`.
/// Empty when no graph.
pub fn search_symbols(query: &str) -> Vec<(String, String, usize)> {
    let Some(handle) = graph_handle() else {
        return Vec::new();
    };
    let graph = handle.graph.read().unwrap();
    let Some(graph) = graph.as_ref() else {
        return Vec::new();
    };
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|s| s.to_ascii_lowercase())
        .collect();
    let mut out: Vec<(String, String, usize)> = Vec::new();
    for id in graph.backbone().node_indices() {
        if let Some(NodeData::Symbol(s)) = graph.node(id) {
            let name_lc = s.name.to_ascii_lowercase();
            if terms.iter().any(|t| name_lc.contains(t.as_str())) {
                out.push((s.name.clone(), s.file_path.clone(), s.line_start));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

// ── graph-aware symbols for the TUI ContextBuilder ───────────────────────────

/// Select up to `limit` symbols relevant to `task`: seed from a token-budgeted
/// `query_graph` over the task terms, expanding via blast-radius. Returns an
/// empty vec when no graph is available (caller falls back to the index path).
pub fn graph_aware_symbols(task: &str, limit: usize) -> Vec<SymbolInfo> {
    let Some(handle) = graph_handle() else {
        return Vec::new();
    };
    let graph = handle.graph.read().unwrap();
    let Some(graph) = graph.as_ref() else {
        return Vec::new();
    };
    if graph.node_count() == 0 {
        return Vec::new();
    }

    let terms: Vec<&str> = task.split_whitespace().collect();
    let mut out: Vec<SymbolInfo> = Vec::new();
    let mut seen: std::collections::HashSet<(String, String, usize)> =
        std::collections::HashSet::new();

    let push = |out: &mut Vec<SymbolInfo>,
                seen: &mut std::collections::HashSet<(String, String, usize)>,
                sym: &KgSymbol| {
        let key = (sym.name.clone(), sym.file_path.clone(), sym.line_start);
        if seen.insert(key) {
            out.push(map_symbol(sym));
        }
    };

    if !terms.is_empty() {
        let sub = query_graph(graph, &terms.join(" "), 2000);
        for nd in &sub.nodes {
            if let NodeData::Symbol(s) = nd {
                push(&mut out, &mut seen, s);
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }

    // Fall back to / top up with god nodes if the query was too narrow.
    if out.is_empty() {
        for g in god_nodes(graph, limit) {
            if let Some(NodeData::Symbol(s)) = graph.node(g.id) {
                push(&mut out, &mut seen, s);
                if out.len() >= limit {
                    break;
                }
            }
        }
    }

    out.truncate(limit);
    out
}

/// Lossy bridge from a kodegraph `Symbol` to a vibe-core `SymbolInfo`.
///
/// Lossiness: kodegraph has 12 `SymbolKind` / 20 `Language` variants vs
/// vibe-core's 11 / 6. Unknown kinds fall back to a sensible `SymbolKind`;
/// unknown languages → `Unknown`. This only feeds a context string, so the
/// lossiness is acceptable and keeps vibe-core kodegraph-free.
pub fn map_symbol(s: &KgSymbol) -> SymbolInfo {
    SymbolInfo {
        name: s.name.clone(),
        kind: map_symbol_kind(s.kind),
        file: PathBuf::from(&s.file_path),
        line: s.line_start,
        signature: s.signature.clone().unwrap_or_default(),
        language: map_language(s.language),
    }
}

fn map_symbol_kind(k: KgSymbolKind) -> VcSymbolKind {
    match k {
        KgSymbolKind::Function => VcSymbolKind::Function,
        KgSymbolKind::Method => VcSymbolKind::Method,
        KgSymbolKind::Class => VcSymbolKind::Class,
        KgSymbolKind::Struct => VcSymbolKind::Struct,
        KgSymbolKind::Enum => VcSymbolKind::Enum,
        KgSymbolKind::Interface => VcSymbolKind::Interface,
        KgSymbolKind::Trait => VcSymbolKind::Trait,
        KgSymbolKind::Module => VcSymbolKind::Module,
        KgSymbolKind::Constant => VcSymbolKind::Constant,
        KgSymbolKind::Variable => VcSymbolKind::Variable,
        KgSymbolKind::TypeAlias => VcSymbolKind::Type,
        KgSymbolKind::Macro => VcSymbolKind::Constant,
    }
}

fn map_language(l: KgLanguage) -> VcLanguage {
    match l {
        KgLanguage::Rust => VcLanguage::Rust,
        KgLanguage::TypeScript => VcLanguage::TypeScript,
        KgLanguage::JavaScript => VcLanguage::JavaScript,
        KgLanguage::Python => VcLanguage::Python,
        KgLanguage::Go => VcLanguage::Go,
        _ => VcLanguage::Unknown,
    }
}

// (Route handlers in serve.rs serialize graph results via serde_json::Value,
// so the kodegraph analyze result types are not re-exported here. Add a
// `pub use` if a typed surface is needed later.)

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("lib.rs"),
            "pub fn alpha() {}\npub fn beta() { alpha(); }\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("other.rs"), "pub fn gamma() { beta(); }\n").unwrap();
        dir
    }

    #[test]
    fn build_and_query() {
        let dir = fixture_dir();
        let handle = init_graph_handle(dir.path());
        let probe = build_graph_blocking(dir.path()).unwrap();
        assert_eq!(probe.status, GraphStatus::Ready);
        assert!(probe.node_count >= 3, "node_count={}", probe.node_count);

        // alpha is called by beta → callers includes beta.
        let callers_alpha = callers("alpha");
        assert!(
            callers_alpha.iter().any(|c| c.contains("beta")),
            "{:?}",
            callers_alpha
        );

        // summary is non-empty.
        let summary = render_repo_map_summary(handle).unwrap_or_default();
        assert!(summary.contains("Code graph:"));
    }

    #[test]
    fn map_symbol_preserves_core_fields() {
        let s = KgSymbol {
            name: "foo".into(),
            kind: KgSymbolKind::Function,
            qualified_name: "pkg::foo".into(),
            file_path: "src/lib.rs".into(),
            line_start: 42,
            line_end: 50,
            signature: Some("fn foo()".into()),
            doc_comment: None,
            visibility: kodegraph::model::symbol::Visibility::Public,
            language: KgLanguage::Rust,
        };
        let info = map_symbol(&s);
        assert_eq!(info.name, "foo");
        assert!(matches!(info.kind, VcSymbolKind::Function));
        assert_eq!(info.line, 42);
        assert_eq!(info.signature, "fn foo()");
        assert!(matches!(info.language, VcLanguage::Rust));
    }

    #[test]
    fn map_symbol_kind_alias_and_macro_fall_back() {
        assert!(matches!(
            map_symbol_kind(KgSymbolKind::TypeAlias),
            VcSymbolKind::Type
        ));
        assert!(matches!(
            map_symbol_kind(KgSymbolKind::Macro),
            VcSymbolKind::Constant
        ));
    }

    #[test]
    fn graph_aware_symbols_seeds_from_query() {
        let dir = fixture_dir();
        init_graph_handle(dir.path());
        build_graph_blocking(dir.path()).unwrap();
        let syms = graph_aware_symbols("alpha", 10);
        assert!(!syms.is_empty(), "should seed symbols from query 'alpha'");
        assert!(syms
            .iter()
            .any(|s| s.name.contains("alpha") || s.name.contains("beta")));
    }
}
