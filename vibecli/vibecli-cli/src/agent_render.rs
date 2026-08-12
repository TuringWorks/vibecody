//! Structured console rendering for agent runs.
//!
//! The old renderer printed one line per tool call plus that tool's full
//! output, interleaved with whatever prose the model emitted between calls. A
//! twenty-step scaffold became hundreds of lines in which the actual edits were
//! invisible.
//!
//! This module collapses that into two kinds of block:
//!
//! - **Activity** — consecutive non-editing calls folded into one line:
//!   `Read 1 file, listed 1 directory, ran 3 shell commands`.
//! - **Edit** — every file-modifying call gets its own header and line count,
//!   because edits are the thing a user actually scans for.
//!
//! An edit flushes any pending activity first, so ordering is preserved: the
//! reads that led to an edit print above it.
//!
//! ## What the counts mean
//!
//! Line counts are derived only from data we hold. `apply_patch` carries its
//! own `+`/`-` lines, so added/removed are exact. `write_file` carries the new
//! body, so the written line count is exact — but nothing in [`ToolResult`]
//! records what the file held *before*, so no "removed" figure is claimed. A
//! fabricated `removed 0 lines` would read as "this file was new" on every
//! overwrite, which is exactly the kind of confident-but-unchecked number this
//! codebase treats as a defect.

use std::path::Path;
use vibe_ai::tools::ToolCall;

/// One rendered unit of agent activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// Folded read-only activity, e.g. `Read 2 files, ran 1 shell command`.
    Activity(String),
    /// A file-modifying call.
    Edit {
        verb: String,
        path: String,
        detail: String,
    },
}

impl Block {
    /// Render to the console form, without colour.
    pub fn to_plain(&self) -> String {
        match self {
            Block::Activity(s) => format!("  {s}"),
            Block::Edit { verb, path, detail } => {
                format!("⏺ {verb}({path})\n  {detail}")
            }
        }
    }
}

/// Counters for read-only tools, in the order they are rendered.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Activity {
    searched: usize,
    read: usize,
    listed: usize,
    ran: usize,
    fetched: usize,
    web_searched: usize,
    spawned: usize,
    /// Tools with no dedicated phrasing yet — counted so they cannot vanish
    /// from the summary just because nobody wrote a verb for them.
    other: usize,
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("{n} {one}")
    } else {
        format!("{n} {many}")
    }
}

/// Uppercase the first character of `s`.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

impl Activity {
    fn record(&mut self, call: &ToolCall) {
        match call {
            ToolCall::SearchFiles { .. } => self.searched += 1,
            ToolCall::ReadFile { .. } => self.read += 1,
            ToolCall::ListDirectory { .. } => self.listed += 1,
            ToolCall::Bash { .. } => self.ran += 1,
            ToolCall::FetchUrl { .. } => self.fetched += 1,
            ToolCall::WebSearch { .. } => self.web_searched += 1,
            ToolCall::SpawnAgent { .. } => self.spawned += 1,
            _ => self.other += 1,
        }
    }

    /// `Searched for 1 pattern, read 4 files, ran 6 shell commands`
    fn render(&self) -> Option<String> {
        let parts: Vec<String> = [
            (
                self.searched,
                plural(self.searched, "pattern", "patterns"),
                "searched for ",
            ),
            (self.read, plural(self.read, "file", "files"), "read "),
            (
                self.listed,
                plural(self.listed, "directory", "directories"),
                "listed ",
            ),
            (
                self.ran,
                plural(self.ran, "shell command", "shell commands"),
                "ran ",
            ),
            (
                self.fetched,
                plural(self.fetched, "URL", "URLs"),
                "fetched ",
            ),
            (
                self.web_searched,
                plural(self.web_searched, "web search", "web searches"),
                "ran ",
            ),
            (
                self.spawned,
                plural(self.spawned, "agent", "agents"),
                "spawned ",
            ),
            (
                self.other,
                plural(self.other, "other tool", "other tools"),
                "used ",
            ),
        ]
        .into_iter()
        .filter(|(n, _, _)| *n > 0)
        .map(|(_, counted, verb)| format!("{verb}{counted}"))
        .collect();

        parts
            .split_first()
            .map(|(first, rest)| match rest.is_empty() {
                true => capitalize(first),
                false => format!("{}, {}", capitalize(first), rest.join(", ")),
            })
    }
}

/// Added/removed line counts parsed from a unified diff body.
///
/// `+++`/`---` file headers are excluded — counting them inflates every patch
/// by one line in each direction.
fn patch_line_delta(patch: &str) -> (usize, usize) {
    patch.lines().fold((0, 0), |(add, del), line| {
        if line.starts_with("+++") || line.starts_with("---") {
            (add, del)
        } else if line.starts_with('+') {
            (add + 1, del)
        } else if line.starts_with('-') {
            (add, del + 1)
        } else {
            (add, del)
        }
    })
}

/// Shorten an absolute path against the workspace so the line stays readable.
fn display_path(path: &str, workspace: Option<&Path>) -> String {
    workspace
        .and_then(|ws| Path::new(path).strip_prefix(ws).ok())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// Folds a stream of tool calls into [`Block`]s.
#[derive(Debug, Default)]
pub struct Renderer {
    pending: Activity,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one executed call. Returns the blocks ready to print — empty
    /// while read-only calls are still accumulating.
    pub fn record(
        &mut self,
        call: &ToolCall,
        success: bool,
        workspace: Option<&Path>,
    ) -> Vec<Block> {
        match Self::edit_block(call, success, workspace) {
            // An edit ends the current activity run, so the reads that led to
            // it print above it rather than after.
            Some(edit) => self
                .flush()
                .into_iter()
                .chain(std::iter::once(edit))
                .collect(),
            None => {
                self.pending.record(call);
                Vec::new()
            }
        }
    }

    /// Emit any half-accumulated activity. Call at end of turn, and before
    /// printing a final summary, or the last reads are silently dropped.
    pub fn flush(&mut self) -> Vec<Block> {
        let rendered = self.pending.render();
        self.pending = Activity::default();
        rendered.map(Block::Activity).into_iter().collect()
    }

    fn edit_block(call: &ToolCall, success: bool, workspace: Option<&Path>) -> Option<Block> {
        let (verb, path, detail) = match call {
            ToolCall::WriteFile { path, content } => (
                "Write",
                path.clone(),
                // No "removed" figure: nothing here knows the previous body.
                format!("Wrote {}", plural(content.lines().count(), "line", "lines")),
            ),
            ToolCall::ApplyPatch { path, patch } => {
                let (added, removed) = patch_line_delta(patch);
                (
                    "Patch",
                    path.clone(),
                    format!(
                        "Added {}, removed {}",
                        plural(added, "line", "lines"),
                        plural(removed, "line", "lines")
                    ),
                )
            }
            _ => return None,
        };
        let detail = if success {
            detail
        } else {
            format!("{detail} — failed")
        };
        Some(Block::Edit {
            verb: verb.to_string(),
            path: display_path(&path, workspace),
            detail,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(p: &str) -> ToolCall {
        ToolCall::ReadFile {
            path: p.to_string(),
        }
    }
    fn bash(c: &str) -> ToolCall {
        ToolCall::Bash {
            command: c.to_string(),
        }
    }
    fn write(p: &str, c: &str) -> ToolCall {
        ToolCall::WriteFile {
            path: p.to_string(),
            content: c.to_string(),
        }
    }

    #[test]
    fn read_only_calls_fold_into_one_line() {
        let mut r = Renderer::new();
        assert!(r.record(&read("a.rs"), true, None).is_empty());
        assert!(r.record(&bash("ls"), true, None).is_empty());
        assert!(r.record(&bash("pwd"), true, None).is_empty());
        assert_eq!(
            r.flush(),
            vec![Block::Activity("Read 1 file, ran 2 shell commands".into())]
        );
    }

    #[test]
    fn singular_and_plural_are_both_right() {
        let mut r = Renderer::new();
        r.record(&read("a"), true, None);
        assert_eq!(r.flush(), vec![Block::Activity("Read 1 file".into())]);

        let mut r = Renderer::new();
        r.record(&read("a"), true, None);
        r.record(&read("b"), true, None);
        assert_eq!(r.flush(), vec![Block::Activity("Read 2 files".into())]);
    }

    #[test]
    fn an_edit_flushes_pending_activity_first() {
        // Ordering matters: the reads that led to the edit belong above it.
        let mut r = Renderer::new();
        r.record(&read("a.rs"), true, None);
        let blocks = r.record(&write("db.rs", "one\ntwo\nthree"), true, None);
        assert_eq!(
            blocks,
            vec![
                Block::Activity("Read 1 file".into()),
                Block::Edit {
                    verb: "Write".into(),
                    path: "db.rs".into(),
                    detail: "Wrote 3 lines".into(),
                },
            ]
        );
    }

    #[test]
    fn activity_after_an_edit_starts_a_fresh_run() {
        let mut r = Renderer::new();
        r.record(&write("a.rs", "x"), true, None);
        r.record(&bash("cargo test"), true, None);
        assert_eq!(
            r.flush(),
            vec![Block::Activity("Ran 1 shell command".into())]
        );
    }

    #[test]
    fn patch_counts_exclude_file_headers() {
        let patch =
            "--- a/x.rs\n+++ b/x.rs\n@@ -1,2 +1,3 @@\n context\n-old line\n+new line\n+extra";
        assert_eq!(patch_line_delta(patch), (2, 1));
    }

    #[test]
    fn patch_block_reports_both_directions() {
        let mut r = Renderer::new();
        let call = ToolCall::ApplyPatch {
            path: "src/x.rs".into(),
            patch: "--- a/x\n+++ b/x\n-gone\n+added\n+added2".into(),
        };
        assert_eq!(
            r.record(&call, true, None),
            vec![Block::Edit {
                verb: "Patch".into(),
                path: "src/x.rs".into(),
                detail: "Added 2 lines, removed 1 line".into(),
            }]
        );
    }

    #[test]
    fn a_failed_edit_says_so() {
        let mut r = Renderer::new();
        let blocks = r.record(&write("a.rs", "x"), false, None);
        match &blocks[..] {
            [Block::Edit { detail, .. }] => assert!(detail.ends_with("— failed"), "{detail}"),
            other => panic!("expected one edit block, got {other:?}"),
        }
    }

    #[test]
    fn flush_on_an_empty_run_emits_nothing() {
        assert!(Renderer::new().flush().is_empty());
    }

    #[test]
    fn unknown_tools_are_counted_not_dropped() {
        // A tool with no verb must still appear, or the summary quietly
        // under-reports what the agent did.
        let mut r = Renderer::new();
        r.record(
            &ToolCall::TaskComplete {
                summary: "done".into(),
            },
            true,
            None,
        );
        assert_eq!(r.flush(), vec![Block::Activity("Used 1 other tool".into())]);
    }

    #[test]
    fn paths_render_relative_to_the_workspace() {
        let mut r = Renderer::new();
        let blocks = r.record(
            &write("/repo/src/db.rs", "a"),
            true,
            Some(Path::new("/repo")),
        );
        match &blocks[..] {
            [Block::Edit { path, .. }] => assert_eq!(path, "src/db.rs"),
            other => panic!("expected one edit block, got {other:?}"),
        }
    }

    #[test]
    fn plain_rendering_shapes_the_two_block_kinds() {
        assert_eq!(
            Block::Activity("Read 1 file".into()).to_plain(),
            "  Read 1 file"
        );
        assert_eq!(
            Block::Edit {
                verb: "Write".into(),
                path: "a.rs".into(),
                detail: "Wrote 3 lines".into()
            }
            .to_plain(),
            "⏺ Write(a.rs)\n  Wrote 3 lines"
        );
    }
}
