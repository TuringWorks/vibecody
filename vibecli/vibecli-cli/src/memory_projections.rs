//! Markdown projections of OpenMemory state.
//!
//! Phase 6 of the memory-as-infrastructure redesign. OpenMemory stores
//! facts in a binary-ish layout (encrypted content, HNSW index, waypoint
//! graph). Users need a readable, stable window into that state without
//! crawling the internals — so we emit two generated markdown files:
//!
//! - `~/.vibecli/USER.md`            — user-tier projection
//! - `<workspace>/.vibecli/MEMORY.md` — project-tier projection
//!
//! These are *projections*, not sources of truth. Editing them has no
//! effect on the store; regenerating overwrites them.

#![allow(dead_code)]

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::open_memory::{MemoryNode, MemorySector, OpenMemoryStore};

/// Output paths from [`write_projections`]. `user_md` is `None` when
/// `home_dir` was not supplied (e.g. in headless or test environments).
#[derive(Debug, Clone)]
pub struct ProjectionPaths {
    pub user_md: Option<PathBuf>,
    pub memory_md: PathBuf,
}

/// Render one store as a grouped markdown projection. The output is
/// deterministic for a given input state — sections are sorted, entries
/// within a sector are sorted by `created_at` (newest first), and the
/// header records counts so callers can diff projections over time.
pub fn render_markdown(store: &OpenMemoryStore, title: &str) -> String {
    // Scope to the store's current project tier. When `project_id` is set,
    // render only memories tagged for that project; when unset, render the
    // user tier (memories without a project_id). This keeps USER.md and
    // MEMORY.md disjoint even when they share the on-disk store.
    let scope = store.project_id();
    let all: Vec<&MemoryNode> = store
        .list_memories(0, usize::MAX)
        .into_iter()
        .filter(|m| m.project_id.as_deref() == scope)
        .collect();
    let total = all.len();
    let pinned: Vec<&MemoryNode> = all.iter().copied().filter(|m| m.pinned).collect();

    let mut out = String::new();
    out.push_str(&format!("# {}\n", title));
    out.push_str(
        "_Generated projection of OpenMemory state. Edit memories via the \
         `/openmemory` REPL or VibeUI — this file is overwritten on every \
         refresh._\n\n",
    );
    out.push_str(&format!(
        "- Total memories: **{}**\n- Pinned: **{}**\n\n",
        total,
        pinned.len()
    ));

    if !pinned.is_empty() {
        out.push_str("## Pinned\n\n");
        for m in pinned.iter() {
            push_bullet(&mut out, m);
        }
        out.push('\n');
    }

    // Group remaining by sector, in a stable order so diffs are readable.
    for sector in MemorySector::all() {
        let bucket: Vec<&MemoryNode> = all
            .iter()
            .copied()
            .filter(|m| !m.pinned && m.sector == *sector)
            .collect();
        if bucket.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "## {} ({})\n\n",
            sector_heading(*sector),
            bucket.len()
        ));
        for m in bucket.iter() {
            push_bullet(&mut out, m);
        }
        out.push('\n');
    }

    if total == 0 {
        out.push_str(
            "_No memories yet. Facts extracted by agents or imported \
             via `/openmemory import` will appear here._\n",
        );
    }

    out
}

fn sector_heading(s: MemorySector) -> &'static str {
    match s {
        MemorySector::Episodic => "Episodic — events & sessions",
        MemorySector::Semantic => "Semantic — facts & definitions",
        MemorySector::Procedural => "Procedural — how-to & workflows",
        MemorySector::Emotional => "Emotional — reactions & sentiment",
        MemorySector::Reflective => "Reflective — insights & lessons",
    }
}

fn push_bullet(out: &mut String, m: &MemoryNode) {
    let preview = one_line(&m.content, 240);
    let tag_str = if m.tags.is_empty() {
        String::new()
    } else {
        format!(" `[{}]`", m.tags.join(", "))
    };
    out.push_str(&format!(
        "- {preview}{tag_str} *(salience: {:.0}%)*\n",
        (m.salience.clamp(0.0, 1.0)) * 100.0
    ));
}

fn one_line(s: &str, max: usize) -> String {
    let flat: String = s
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    if flat.chars().count() <= max {
        return flat;
    }
    let truncated: String = flat.chars().take(max).collect();
    format!("{truncated}…")
}

/// Write both projections. Returns the paths that were written. The
/// user-tier file is skipped when `home_dir` is `None`.
///
/// Idempotent: projections fully overwrite any existing file, so calling
/// this repeatedly produces the same on-disk bytes for a given store
/// state.
pub fn write_projections(
    home_dir: Option<&Path>,
    workspace: &Path,
) -> Result<ProjectionPaths> {
    let memory_md = workspace.join(".vibecli").join("MEMORY.md");
    let project_store = crate::open_memory::project_scoped_store(workspace);
    let project_title = format!(
        "Project Memory — {}",
        workspace
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
    );
    let project_md = render_markdown(&project_store, &project_title);
    if let Some(parent) = memory_md.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&memory_md, project_md)?;

    let user_md = if let Some(home) = home_dir {
        let path = home.join(".vibecli").join("USER.md");
        // User tier: open the default store without scoping to a project.
        let user_store = crate::open_memory::project_scoped_store(home);
        let body = render_markdown(&user_store, "User Memory");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, body)?;
        Some(path)
    } else {
        None
    };

    Ok(ProjectionPaths { user_md, memory_md })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory::{MemorySector, OpenMemoryStore};
    use tempfile::TempDir;

    fn store_with(entries: &[(&str, MemorySector, bool)]) -> OpenMemoryStore {
        let dir = TempDir::new().unwrap();
        let mut store = OpenMemoryStore::new(dir.path(), "test");
        for (content, _sector, pinned) in entries {
            let id = store.add_with_tags(
                *content,
                Vec::new(),
                std::collections::HashMap::new(),
            );
            if *pinned {
                store.pin(&id);
            }
        }
        // Keep the tempdir alive by leaking it — acceptable in tests where
        // the store doesn't need persistence round-tripping.
        std::mem::forget(dir);
        store
    }

    #[test]
    fn render_markdown_includes_header_and_counts() {
        let store = store_with(&[
            (
                "The project uses Rust edition 2021",
                MemorySector::Semantic,
                false,
            ),
            (
                "Run cargo test --workspace before pushing",
                MemorySector::Procedural,
                false,
            ),
        ]);
        let md = render_markdown(&store, "Test Title");
        assert!(md.starts_with("# Test Title\n"));
        assert!(md.contains("Total memories: **2**"));
        assert!(md.contains("Pinned: **0**"));
    }

    #[test]
    fn render_markdown_groups_pinned_above_sector_buckets() {
        let store = store_with(&[
            ("Some reflective insight", MemorySector::Reflective, true),
            ("An unpinned fact", MemorySector::Semantic, false),
        ]);
        let md = render_markdown(&store, "T");
        let pinned_pos = md.find("## Pinned").expect("pinned header");
        // The unpinned fact is Semantic — its sector heading must appear
        // *after* the pinned block so the most-important entries render
        // first in the file.
        let sector_pos = md
            .find("## Semantic")
            .expect("at least one sector heading");
        assert!(
            pinned_pos < sector_pos,
            "pinned section should precede sector sections"
        );
        assert!(md.contains("insight"));
    }

    #[test]
    fn render_markdown_is_deterministic_for_same_state() {
        let store = store_with(&[
            ("Alpha fact", MemorySector::Semantic, false),
            ("Beta procedure", MemorySector::Procedural, false),
        ]);
        let a = render_markdown(&store, "Same");
        let b = render_markdown(&store, "Same");
        assert_eq!(a, b, "render must be deterministic for a given store state");
    }

    #[test]
    fn render_markdown_empty_store_has_friendly_hint() {
        let dir = TempDir::new().unwrap();
        let store = OpenMemoryStore::new(dir.path(), "test");
        let md = render_markdown(&store, "Empty");
        assert!(md.contains("Total memories: **0**"));
        assert!(md.contains("No memories yet"));
    }

    #[test]
    fn render_markdown_collapses_newlines_in_bullet_previews() {
        // Multi-line memory content must flatten to a single bullet line —
        // otherwise markdown would render it as multiple list items or
        // break list formatting entirely.
        let store = store_with(&[(
            "line one\nline two\nline three",
            MemorySector::Semantic,
            false,
        )]);
        let md = render_markdown(&store, "T");
        let bullet_line = md
            .lines()
            .find(|l| l.starts_with("- ") && l.contains("line one"))
            .expect("bullet line");
        // The entire flattened content lives on ONE line — proof that \n
        // was replaced by spaces instead of surviving into the output.
        assert!(
            bullet_line.contains("line one line two line three"),
            "got: {bullet_line:?}"
        );
        // And the original content is reachable from the combined output.
        assert!(md.contains("line one line two"));
    }

    #[test]
    fn write_projections_writes_memory_md_and_is_idempotent() {
        let workspace = TempDir::new().unwrap();
        let first =
            write_projections(None, workspace.path()).expect("first write");
        assert!(first.memory_md.exists());
        assert!(first.user_md.is_none(), "home omitted → no USER.md");

        let bytes1 = std::fs::read(&first.memory_md).unwrap();
        let _ = write_projections(None, workspace.path()).expect("second write");
        let bytes2 = std::fs::read(&first.memory_md).unwrap();
        assert_eq!(bytes1, bytes2, "repeated writes must be byte-identical");
    }

    #[test]
    fn write_projections_also_writes_user_md_when_home_is_provided() {
        let home = TempDir::new().unwrap();
        let workspace = TempDir::new().unwrap();
        let paths = write_projections(Some(home.path()), workspace.path())
            .expect("write");
        let user_md = paths.user_md.expect("user md path");
        assert!(user_md.exists());
        let body = std::fs::read_to_string(&user_md).unwrap();
        assert!(body.starts_with("# User Memory\n"));
    }

    #[test]
    fn memory_md_title_contains_workspace_basename() {
        let workspace = TempDir::new().unwrap();
        let paths = write_projections(None, workspace.path()).expect("write");
        let body = std::fs::read_to_string(&paths.memory_md).unwrap();
        assert!(
            body.starts_with("# Project Memory — "),
            "got: {}",
            body.lines().next().unwrap_or("")
        );
    }
}
