// This module is compiled into both the library and the `vibecli` binary (see
// CLAUDE.md → Module declaration pattern). The binary uses a subset of the
// public API, so the rest reads as dead there; `bugbot.rs` carries the same
// allow for the same reason.
#![allow(dead_code)]
//! BugBot autofix — turn findings into *committable* suggestions.
//!
//! [`bugbot`](crate::bugbot) tells a reviewer what is wrong in prose. This
//! module takes the next step competitors already take: it produces a
//! GitHub ```` ```suggestion ```` block the reviewer commits with one click, or
//! that `--bugbot --fix` applies to the working tree.
//!
//! # Why anchoring is the whole problem
//!
//! GitHub applies a suggestion by **replacing the exact lines the comment is
//! anchored to** in the head commit. A suggestion anchored at the wrong line
//! silently destroys code. So a proposal is only ever built from lines this
//! module can *see* in the diff's post-image — never from a line number the
//! model asserted. When the anchor cannot be located, no proposal is emitted.
//! Absent stays absent.
//!
//! # What "verified" means here
//!
//! [`FixProposal`] carries a [`Verification`] that says exactly what was
//! checked. `AnchorVerified` means the target lines were located in the diff
//! and the replacement is non-empty and different — it does **not** mean the
//! result compiles. Nothing in this module claims a fix was tested.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vibe_ai::provider::{AIProvider as LLMProvider, Message, MessageRole};

use crate::bugbot::{BugReport, Severity};

/// Longest span of lines a single suggestion may replace.
///
/// A suggestion that rewrites half a file is not reviewable, and the wider the
/// span the likelier the anchor drifts against the head commit.
pub const MAX_SPAN_LINES: u32 = 20;

/// Lines of post-image context shown to the model on each side of a finding.
pub const CONTEXT_RADIUS: u32 = 8;

// ── Post-image index ─────────────────────────────────────────────────────────

/// The new-file side of a unified diff, addressable by line number.
///
/// Built from context (` `) and added (`+`) lines only — removed lines do not
/// exist in the head commit and cannot be anchored to.
#[derive(Debug, Clone, Default)]
pub struct PostImage {
    files: HashMap<String, BTreeMap<u32, String>>,
}

impl PostImage {
    /// Parse a unified diff into a `path -> new_line -> text` index.
    ///
    /// Accepts both `diff --git a/x b/x` output and bare `+++ b/x` headers, so
    /// it works on `git diff`, `git format-patch`, and the GitHub
    /// `application/vnd.github.v3.diff` media type alike.
    pub fn from_diff(diff: &str) -> Self {
        let mut files: HashMap<String, BTreeMap<u32, String>> = HashMap::new();
        let mut path: Option<String> = None;
        let mut new_line: u32 = 0;

        // An added line reading `++ x` renders as `+++ x`, indistinguishable from
        // a file header on its own. A real `+++` header is always the line right
        // after a `---` header, so the pair is matched together — never `+++`
        // alone, which would silently repoint every anchor that follows.
        let mut lines = diff.lines().peekable();

        while let Some(raw) = lines.next() {
            if let Some(rest) = raw.strip_prefix("diff --git ") {
                path = parse_git_header_path(rest);
                new_line = 0;
                continue;
            }
            if raw.starts_with("--- ")
                && lines.peek().is_some_and(|next| next.starts_with("+++ "))
            {
                let header = lines.next().unwrap_or_default();
                // `+++ /dev/null` is a deletion — nothing to anchor to.
                path = strip_diff_prefix(header[4..].trim()).filter(|p| p.as_str() != "/dev/null");
                new_line = 0;
                continue;
            }
            if let Some(rest) = raw.strip_prefix("@@") {
                new_line = parse_hunk_new_start(rest).unwrap_or(0);
                continue;
            }
            // Only inside a hunk of a known file do body lines mean anything.
            let (Some(p), true) = (path.as_ref(), new_line > 0) else {
                continue;
            };
            match raw.as_bytes().first() {
                // Added or context: both exist in the head commit.
                Some(b'+') | Some(b' ') => {
                    files
                        .entry(p.clone())
                        .or_default()
                        .insert(new_line, raw[1..].to_string());
                    new_line += 1;
                }
                // Removed: consumes an old-file line, not a new-file one.
                Some(b'-') => {}
                // `\ No newline at end of file`.
                Some(b'\\') => {}
                // A blank context line that lost its leading space in transit.
                None => {
                    files
                        .entry(p.clone())
                        .or_default()
                        .insert(new_line, String::new());
                    new_line += 1;
                }
                _ => {}
            }
        }

        Self { files }
    }

    /// Text of a single post-image line, if the diff shows it.
    pub fn line(&self, path: &str, line: u32) -> Option<&str> {
        self.files.get(path)?.get(&line).map(String::as_str)
    }

    /// Every path the diff touches on the new side.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// Contiguous post-image lines `start..=end`, or `None` if any is missing.
    ///
    /// A gap means the range straddles a hunk boundary; the unseen lines could
    /// be anything, so the range is not safe to replace.
    pub fn span(&self, path: &str, start: u32, end: u32) -> Option<Vec<&str>> {
        let file = self.files.get(path)?;
        if start == 0 || end < start {
            return None;
        }
        (start..=end)
            .map(|n| file.get(&n).map(String::as_str))
            .collect()
    }

    /// Numbered window around `line`, clamped to what the diff actually shows.
    ///
    /// Used to give the model real code to rewrite instead of asking it to
    /// recall the file from the finding's prose.
    pub fn window(&self, path: &str, line: u32, radius: u32) -> Vec<(u32, &str)> {
        let Some(file) = self.files.get(path) else {
            return Vec::new();
        };
        let lo = line.saturating_sub(radius).max(1);
        let hi = line.saturating_add(radius);
        file.range(lo..=hi).map(|(n, t)| (*n, t.as_str())).collect()
    }
}

/// Extract the b-side path from the tail of a `diff --git ` line.
fn parse_git_header_path(rest: &str) -> Option<String> {
    // "a/src/foo.rs b/src/foo.rs" — take everything after the last " b/".
    rest.rsplit_once(" b/")
        .map(|(_, b)| b.to_string())
        .filter(|p| !p.is_empty())
}

/// Strip the `b/` (or `a/`) prefix and any trailing tab-separated metadata.
fn strip_diff_prefix(spec: &str) -> Option<String> {
    let path = spec.split('\t').next().unwrap_or(spec).trim();
    if path.is_empty() {
        return None;
    }
    Some(
        path.strip_prefix("b/")
            .or_else(|| path.strip_prefix("a/"))
            .unwrap_or(path)
            .to_string(),
    )
}

/// Parse the new-file start line out of `@@ -12,7 +34,9 @@`.
fn parse_hunk_new_start(rest: &str) -> Option<u32> {
    let plus = rest.split('+').nth(1)?;
    let digits: String = plus.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

// ── Proposals ────────────────────────────────────────────────────────────────

/// What was actually checked about a proposal. Nothing more is implied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verification {
    /// Target lines located in the diff post-image; replacement is non-empty
    /// and differs from the original. The result is **not** known to compile.
    AnchorVerified,
}

/// A committable replacement for a contiguous run of post-image lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixProposal {
    /// Index into the `reports` slice this proposal fixes.
    pub report_index: usize,
    pub path: String,
    /// First post-image line replaced (1-based, inclusive).
    pub start_line: u32,
    /// Last post-image line replaced (1-based, inclusive).
    pub end_line: u32,
    /// The lines as they exist in the head commit.
    pub original: Vec<String>,
    /// The lines that replace them.
    pub replacement: Vec<String>,
    /// One sentence on why the replacement is correct.
    pub rationale: String,
    pub verification: Verification,
}

impl FixProposal {
    /// True when the proposal replaces more than one line.
    pub fn is_multiline(&self) -> bool {
        self.end_line > self.start_line
    }

    /// Render the GitHub suggestion block on its own.
    pub fn suggestion_block(&self) -> String {
        format!("```suggestion\n{}\n```", self.replacement.join("\n"))
    }

    /// Render the full review-comment body for a finding plus its fix.
    pub fn comment_body(&self, report: &BugReport) -> String {
        let mut body = format!(
            "**{}** {}: {}",
            report.icon(),
            report.severity,
            report.message
        );
        if !self.rationale.is_empty() {
            body.push_str(&format!("\n\n🔧 **Proposed fix:** {}", self.rationale));
        }
        body.push_str(&format!("\n\n{}", self.suggestion_block()));
        body.push_str(
            "\n\n<sub>Suggested by VibeCody BugBot. The anchor was verified against this diff; \
             the fix has not been compiled or tested.</sub>",
        );
        body
    }

    /// The `comments[]` entry for `POST /pulls/{n}/reviews`.
    ///
    /// Multi-line suggestions need `start_line` + `start_side`; single-line
    /// ones must omit them or GitHub rejects the review.
    pub fn review_comment_json(&self, report: &BugReport) -> serde_json::Value {
        let mut comment = serde_json::json!({
            "path": self.path,
            "line": self.end_line,
            "side": "RIGHT",
            "body": self.comment_body(report),
        });
        if self.is_multiline() {
            if let Some(obj) = comment.as_object_mut() {
                obj.insert("start_line".into(), self.start_line.into());
                obj.insert("start_side".into(), "RIGHT".into());
            }
        }
        comment
    }
}

/// Why a candidate fix was not turned into a proposal.
///
/// Every variant is a refusal to guess. They are surfaced rather than swallowed
/// so `--bugbot` can tell a user *why* a finding has no suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// The model declined to propose a fix.
    ModelDeclined,
    /// The model's reply was not the requested JSON object.
    Unparseable,
    /// `start_line..=end_line` is not fully present in the diff post-image.
    AnchorMissing { path: String, start: u32, end: u32 },
    /// The span exceeds [`MAX_SPAN_LINES`].
    SpanTooLarge { lines: u32 },
    /// The replacement is empty — deletions are not proposed automatically.
    EmptyReplacement,
    /// The replacement is byte-identical to the original.
    Unchanged,
    /// The replacement contains a code fence, which would break the block.
    FenceInReplacement,
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rejection::ModelDeclined => write!(f, "model declined to propose a fix"),
            Rejection::Unparseable => write!(f, "model reply was not valid fix JSON"),
            Rejection::AnchorMissing { path, start, end } => write!(
                f,
                "lines {}-{} of {} are not in the diff — cannot anchor a suggestion",
                start, end, path
            ),
            Rejection::SpanTooLarge { lines } => write!(
                f,
                "span of {} lines exceeds the {}-line suggestion limit",
                lines, MAX_SPAN_LINES
            ),
            Rejection::EmptyReplacement => write!(f, "replacement was empty"),
            Rejection::Unchanged => write!(f, "replacement is identical to the original"),
            Rejection::FenceInReplacement => write!(f, "replacement contains a code fence"),
        }
    }
}

/// A fix the model proposed, before validation.
#[derive(Debug, Clone, Deserialize)]
struct RawFix {
    #[serde(default)]
    skip: bool,
    #[serde(default)]
    start_line: u32,
    #[serde(default)]
    end_line: u32,
    #[serde(default)]
    replacement: String,
    #[serde(default)]
    rationale: String,
}

/// Validate a candidate replacement against the post-image and build a proposal.
///
/// This is the only constructor of [`FixProposal`]; every rule that keeps a
/// suggestion from corrupting a file lives here.
pub fn build_proposal(
    post: &PostImage,
    report_index: usize,
    path: &str,
    start_line: u32,
    end_line: u32,
    replacement: &str,
    rationale: &str,
) -> Result<FixProposal, Rejection> {
    if start_line == 0 || end_line < start_line {
        return Err(Rejection::AnchorMissing {
            path: path.to_string(),
            start: start_line,
            end: end_line,
        });
    }

    let span = end_line - start_line + 1;
    if span > MAX_SPAN_LINES {
        return Err(Rejection::SpanTooLarge { lines: span });
    }

    let original: Vec<String> = post
        .span(path, start_line, end_line)
        .ok_or_else(|| Rejection::AnchorMissing {
            path: path.to_string(),
            start: start_line,
            end: end_line,
        })?
        .into_iter()
        .map(str::to_string)
        .collect();

    let replacement = strip_code_fence(replacement);
    if replacement.contains("```") {
        return Err(Rejection::FenceInReplacement);
    }

    let replacement_lines: Vec<String> = replacement
        .strip_suffix('\n')
        .unwrap_or(&replacement)
        .split('\n')
        .map(str::to_string)
        .collect();

    if replacement_lines.iter().all(|l| l.trim().is_empty()) {
        return Err(Rejection::EmptyReplacement);
    }
    if replacement_lines == original {
        return Err(Rejection::Unchanged);
    }

    Ok(FixProposal {
        report_index,
        path: path.to_string(),
        start_line,
        end_line,
        original,
        replacement: replacement_lines,
        rationale: rationale.trim().to_string(),
        verification: Verification::AnchorVerified,
    })
}

/// Drop a surrounding ```` ``` ```` fence the model wrapped its answer in.
fn strip_code_fence(text: &str) -> String {
    let trimmed = text.trim_matches('\n');
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed.to_string();
    };
    // Skip the info string on the opening fence.
    let body = rest.split_once('\n').map(|(_, b)| b).unwrap_or("");
    body.strip_suffix("```")
        .unwrap_or(body)
        .trim_end_matches('\n')
        .to_string()
}

// ── Generation ───────────────────────────────────────────────────────────────

/// How many findings to attempt a fix for in one review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutofixLimits {
    /// Upper bound on model round-trips. Each finding costs one.
    pub max_fixes: usize,
}

impl Default for AutofixLimits {
    fn default() -> Self {
        Self { max_fixes: 10 }
    }
}

/// The outcome of attempting a fix for one finding.
pub type Attempt = Result<FixProposal, Rejection>;

/// Attempt a committable fix for each actionable finding.
///
/// Only `error` and `warning` findings are attempted — `info` findings are
/// observations, not defects. Returns one entry per attempted finding, paired
/// with its index in `reports`, so callers can report refusals as well as fixes.
pub async fn propose_fixes(
    llm: &Arc<dyn LLMProvider>,
    post: &PostImage,
    reports: &[BugReport],
    limits: AutofixLimits,
) -> Vec<(usize, Attempt)> {
    let actionable = reports
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r.severity, Severity::Error | Severity::Warning))
        .take(limits.max_fixes);

    let mut out = Vec::new();
    for (index, report) in actionable {
        out.push((index, propose_one(llm, post, index, report).await));
    }
    out
}

/// Attempt a fix for a single finding.
async fn propose_one(
    llm: &Arc<dyn LLMProvider>,
    post: &PostImage,
    index: usize,
    report: &BugReport,
) -> Attempt {
    let window = post.window(&report.file, report.line, CONTEXT_RADIUS);
    if window.is_empty() {
        return Err(Rejection::AnchorMissing {
            path: report.file.clone(),
            start: report.line,
            end: report.line,
        });
    }

    let reply = llm
        .chat(
            &[Message {
                role: MessageRole::User,
                content: fix_prompt(report, &window),
            }],
            None,
        )
        .await
        .map_err(|_| Rejection::ModelDeclined)?;

    let raw = parse_raw_fix(&reply).ok_or(Rejection::Unparseable)?;
    if raw.skip {
        return Err(Rejection::ModelDeclined);
    }

    build_proposal(
        post,
        index,
        &report.file,
        raw.start_line,
        raw.end_line,
        &raw.replacement,
        &raw.rationale,
    )
}

/// Build the single-finding fix prompt from real post-image lines.
fn fix_prompt(report: &BugReport, window: &[(u32, &str)]) -> String {
    let numbered = window
        .iter()
        .map(|(n, text)| format!("{:>6} | {}", n, text))
        .collect::<Vec<_>>()
        .join("\n");

    let first = window.first().map(|(n, _)| *n).unwrap_or(report.line);
    let last = window.last().map(|(n, _)| *n).unwrap_or(report.line);

    format!(
        r#"You are BugBot's fix author. Rewrite the smallest possible run of lines that resolves this finding.

File: {file}
Finding (line {line}, {severity}): {message}

Numbered lines from the file (only these line numbers exist — do not reference any other):
{numbered}

Rules:
- `start_line` and `end_line` MUST both be within {first}..{last} and name lines shown above.
- Replace at most {max} lines. Prefer one.
- `replacement` is the literal new text for those lines, newline-separated, with the file's exact indentation. No line numbers, no diff markers, no code fence.
- If you cannot fix this from the lines shown, return {{"skip": true}}.

Return ONLY a JSON object:
{{"start_line": {line}, "end_line": {line}, "replacement": "...", "rationale": "one sentence"}}
"#,
        file = report.file,
        line = report.line,
        severity = report.severity,
        message = report.message,
        numbered = numbered,
        first = first,
        last = last,
        max = MAX_SPAN_LINES,
    )
}

/// Pull the first JSON object out of a model reply.
fn parse_raw_fix(reply: &str) -> Option<RawFix> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')? + 1;
    if start >= end {
        return None;
    }
    serde_json::from_str(&reply[start..end]).ok()
}

// ── Local application ────────────────────────────────────────────────────────

/// Apply a proposal to in-memory file content.
///
/// Returns `None` when the file's current lines at the anchor differ from the
/// `original` recorded in the proposal — the file moved under us, and applying
/// anyway would corrupt it.
pub fn apply_to_content(content: &str, proposal: &FixProposal) -> Option<String> {
    let trailing_newline = content.ends_with('\n');
    let lines: Vec<&str> = content.split('\n').collect();
    // `split` on a trailing newline yields a final empty element that is not a line.
    let lines = if trailing_newline {
        &lines[..lines.len().saturating_sub(1)]
    } else {
        &lines[..]
    };

    let start = proposal.start_line.checked_sub(1)? as usize;
    let end = proposal.end_line as usize;
    if end > lines.len() || start >= end {
        return None;
    }
    if lines[start..end] != proposal.original[..] {
        return None;
    }

    let patched: Vec<&str> = lines[..start]
        .iter()
        .copied()
        .chain(proposal.replacement.iter().map(String::as_str))
        .chain(lines[end..].iter().copied())
        .collect();

    let mut out = patched.join("\n");
    if trailing_newline {
        out.push('\n');
    }
    Some(out)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "\
diff --git a/src/math.rs b/src/math.rs
index 1111111..2222222 100644
--- a/src/math.rs
+++ b/src/math.rs
@@ -1,4 +1,6 @@
 fn divide(a: i32, b: i32) -> i32 {
-    a / b
+    let q = a / b;
+    q
 }

+// trailing
";

    fn report(file: &str, line: u32, severity: Severity) -> BugReport {
        BugReport {
            file: file.to_string(),
            line,
            severity,
            message: "Division by zero when b is 0".into(),
            suggestion: None,
            fix_command: None,
            category: Some("logic".into()),
        }
    }

    // ── PostImage ────────────────────────────────────────────────────────────

    #[test]
    fn indexes_context_and_added_lines_by_new_line_number() {
        let post = PostImage::from_diff(DIFF);
        assert_eq!(post.line("src/math.rs", 1), Some("fn divide(a: i32, b: i32) -> i32 {"));
        assert_eq!(post.line("src/math.rs", 2), Some("    let q = a / b;"));
        assert_eq!(post.line("src/math.rs", 3), Some("    q"));
        assert_eq!(post.line("src/math.rs", 4), Some("}"));
        assert_eq!(post.line("src/math.rs", 5), Some(""));
        assert_eq!(post.line("src/math.rs", 6), Some("// trailing"));
    }

    #[test]
    fn removed_lines_do_not_consume_a_new_line_number() {
        // `-    a / b` must not shift the numbering of what follows.
        let post = PostImage::from_diff(DIFF);
        assert_eq!(post.line("src/math.rs", 2), Some("    let q = a / b;"));
    }

    #[test]
    fn unknown_path_and_line_are_none_not_guesses() {
        let post = PostImage::from_diff(DIFF);
        assert_eq!(post.line("src/other.rs", 1), None);
        assert_eq!(post.line("src/math.rs", 99), None);
    }

    #[test]
    fn span_returns_none_when_any_line_is_unseen() {
        let post = PostImage::from_diff(DIFF);
        assert!(post.span("src/math.rs", 1, 3).is_some());
        assert!(post.span("src/math.rs", 5, 7).is_none());
        assert!(post.span("src/math.rs", 0, 2).is_none());
        assert!(post.span("src/math.rs", 3, 2).is_none());
    }

    #[test]
    fn span_across_a_hunk_gap_is_rejected() {
        let gapped = "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,1 +1,1 @@
+one
@@ -50,1 +50,1 @@
+fifty
";
        let post = PostImage::from_diff(gapped);
        assert_eq!(post.line("a.rs", 1), Some("one"));
        assert_eq!(post.line("a.rs", 50), Some("fifty"));
        assert!(post.span("a.rs", 1, 50).is_none());
    }

    #[test]
    fn window_is_clamped_to_lines_the_diff_shows() {
        let post = PostImage::from_diff(DIFF);
        let w = post.window("src/math.rs", 2, 100);
        assert_eq!(w.len(), 6);
        assert_eq!(w[0].0, 1);
        assert_eq!(w[5].0, 6);
    }

    #[test]
    fn window_on_unknown_path_is_empty() {
        let post = PostImage::from_diff(DIFF);
        assert!(post.window("nope.rs", 1, 5).is_empty());
    }

    #[test]
    fn parses_bare_plusplusplus_headers_without_git_header() {
        let plain = "--- a/x.py\n+++ b/x.py\n@@ -1,2 +1,2 @@\n-old\n+new\n ctx\n";
        let post = PostImage::from_diff(plain);
        assert_eq!(post.line("x.py", 1), Some("new"));
        assert_eq!(post.line("x.py", 2), Some("ctx"));
    }

    #[test]
    fn deleted_file_contributes_no_anchors() {
        let del = "diff --git a/gone.rs b/gone.rs\n--- a/gone.rs\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-bye\n";
        let post = PostImage::from_diff(del);
        assert_eq!(post.paths().count(), 0);
    }

    #[test]
    fn hunk_new_start_is_parsed_from_the_plus_range() {
        assert_eq!(parse_hunk_new_start(" -12,7 +34,9 @@"), Some(34));
        assert_eq!(parse_hunk_new_start(" -1 +1 @@"), Some(1));
        assert_eq!(parse_hunk_new_start(" nonsense"), None);
    }

    // ── build_proposal ───────────────────────────────────────────────────────

    #[test]
    fn builds_a_single_line_proposal() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 2, "    let q = checked(a, b)?;", "guard b")
            .expect("anchor is present");
        assert_eq!(p.start_line, 2);
        assert_eq!(p.end_line, 2);
        assert_eq!(p.original, vec!["    let q = a / b;"]);
        assert_eq!(p.replacement, vec!["    let q = checked(a, b)?;"]);
        assert!(!p.is_multiline());
        assert_eq!(p.verification, Verification::AnchorVerified);
    }

    #[test]
    fn builds_a_multiline_proposal() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 3, "    let q = a / b;\n    q + 1", "")
            .expect("anchor is present");
        assert!(p.is_multiline());
        assert_eq!(p.original.len(), 2);
        assert_eq!(p.replacement.len(), 2);
    }

    #[test]
    fn rejects_an_anchor_the_diff_does_not_show() {
        let post = PostImage::from_diff(DIFF);
        let err = build_proposal(&post, 0, "src/math.rs", 40, 40, "x", "").unwrap_err();
        assert!(matches!(err, Rejection::AnchorMissing { .. }));
    }

    #[test]
    fn rejects_an_anchor_in_an_untouched_file() {
        let post = PostImage::from_diff(DIFF);
        let err = build_proposal(&post, 0, "src/elsewhere.rs", 1, 1, "x", "").unwrap_err();
        assert!(matches!(err, Rejection::AnchorMissing { .. }));
    }

    #[test]
    fn rejects_a_zero_or_inverted_line_range() {
        let post = PostImage::from_diff(DIFF);
        assert!(matches!(
            build_proposal(&post, 0, "src/math.rs", 0, 1, "x", "").unwrap_err(),
            Rejection::AnchorMissing { .. }
        ));
        assert!(matches!(
            build_proposal(&post, 0, "src/math.rs", 3, 2, "x", "").unwrap_err(),
            Rejection::AnchorMissing { .. }
        ));
    }

    #[test]
    fn rejects_a_span_over_the_limit_before_touching_the_index() {
        let post = PostImage::from_diff(DIFF);
        let err = build_proposal(&post, 0, "src/math.rs", 1, 1 + MAX_SPAN_LINES, "x", "").unwrap_err();
        assert_eq!(err, Rejection::SpanTooLarge { lines: MAX_SPAN_LINES + 1 });
    }

    #[test]
    fn rejects_an_empty_replacement_rather_than_proposing_a_deletion() {
        let post = PostImage::from_diff(DIFF);
        assert_eq!(
            build_proposal(&post, 0, "src/math.rs", 2, 2, "   \n  ", "").unwrap_err(),
            Rejection::EmptyReplacement
        );
    }

    #[test]
    fn rejects_a_replacement_identical_to_the_original() {
        let post = PostImage::from_diff(DIFF);
        assert_eq!(
            build_proposal(&post, 0, "src/math.rs", 2, 2, "    let q = a / b;", "").unwrap_err(),
            Rejection::Unchanged
        );
    }

    #[test]
    fn rejects_a_replacement_containing_an_inner_fence() {
        let post = PostImage::from_diff(DIFF);
        assert_eq!(
            build_proposal(&post, 0, "src/math.rs", 2, 2, "a\n```\nb", "").unwrap_err(),
            Rejection::FenceInReplacement
        );
    }

    #[test]
    fn strips_a_wrapping_fence_the_model_added() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 2, "```rust\n    let q = 1;\n```", "")
            .expect("fence is stripped, not rejected");
        assert_eq!(p.replacement, vec!["    let q = 1;"]);
    }

    // ── Rendering ────────────────────────────────────────────────────────────

    #[test]
    fn suggestion_block_is_a_github_suggestion_fence() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 3, "    one\n    two", "").unwrap();
        assert_eq!(p.suggestion_block(), "```suggestion\n    one\n    two\n```");
    }

    #[test]
    fn comment_body_states_what_was_not_verified() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 2, "    let q = 1;", "guard").unwrap();
        let body = p.comment_body(&report("src/math.rs", 2, Severity::Error));
        assert!(body.contains("```suggestion"));
        assert!(body.contains("guard"));
        assert!(body.contains("has not been compiled or tested"));
    }

    #[test]
    fn single_line_comment_json_omits_start_line() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 2, "    let q = 1;", "").unwrap();
        let json = p.review_comment_json(&report("src/math.rs", 2, Severity::Error));
        assert_eq!(json["line"], 2);
        assert_eq!(json["side"], "RIGHT");
        assert!(json.get("start_line").is_none());
    }

    #[test]
    fn multiline_comment_json_carries_start_line_and_side() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 3, "    a\n    b", "").unwrap();
        let json = p.review_comment_json(&report("src/math.rs", 3, Severity::Error));
        assert_eq!(json["start_line"], 2);
        assert_eq!(json["start_side"], "RIGHT");
        assert_eq!(json["line"], 3);
    }

    // ── apply_to_content ─────────────────────────────────────────────────────

    #[test]
    fn applies_a_proposal_to_matching_content() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 2, "    let q = 0;", "").unwrap();
        let content = "fn divide(a: i32, b: i32) -> i32 {\n    let q = a / b;\n    q\n}\n";
        let out = apply_to_content(content, &p).expect("original matches");
        assert_eq!(out, "fn divide(a: i32, b: i32) -> i32 {\n    let q = 0;\n    q\n}\n");
    }

    #[test]
    fn refuses_to_apply_when_the_file_moved_under_us() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 2, "    let q = 0;", "").unwrap();
        let drifted = "fn divide(a: i32, b: i32) -> i32 {\n    SOMETHING ELSE\n    q\n}\n";
        assert!(apply_to_content(drifted, &p).is_none());
    }

    #[test]
    fn refuses_to_apply_past_the_end_of_the_file() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 6, 6, "// changed", "").unwrap();
        assert!(apply_to_content("only one line\n", &p).is_none());
    }

    #[test]
    fn preserves_absence_of_a_trailing_newline() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 1, 1, "fn divide2() {", "").unwrap();
        let content = "fn divide(a: i32, b: i32) -> i32 {";
        let out = apply_to_content(content, &p).expect("single line matches");
        assert_eq!(out, "fn divide2() {");
    }

    #[test]
    fn applying_a_multiline_proposal_changes_line_count() {
        let post = PostImage::from_diff(DIFF);
        let p = build_proposal(&post, 0, "src/math.rs", 2, 3, "    q(a, b)", "").unwrap();
        let content = "fn divide(a: i32, b: i32) -> i32 {\n    let q = a / b;\n    q\n}\n";
        let out = apply_to_content(content, &p).expect("original matches");
        assert_eq!(out, "fn divide(a: i32, b: i32) -> i32 {\n    q(a, b)\n}\n");
    }

    // ── Prompt / parsing ─────────────────────────────────────────────────────

    #[test]
    fn prompt_contains_only_real_line_numbers() {
        let post = PostImage::from_diff(DIFF);
        let window = post.window("src/math.rs", 2, 2);
        let prompt = fix_prompt(&report("src/math.rs", 2, Severity::Error), &window);
        assert!(prompt.contains("     1 | fn divide"));
        assert!(prompt.contains("do not reference any other"));
        assert!(prompt.contains("Replace at most 20 lines"));
    }

    #[test]
    fn parses_a_fix_object_out_of_surrounding_prose() {
        let raw = parse_raw_fix("Sure!\n{\"start_line\":2,\"end_line\":2,\"replacement\":\"x\",\"rationale\":\"y\"}\nDone.")
            .expect("object is found");
        assert_eq!(raw.start_line, 2);
        assert_eq!(raw.replacement, "x");
        assert!(!raw.skip);
    }

    #[test]
    fn parses_a_skip_reply() {
        let raw = parse_raw_fix("{\"skip\": true}").expect("object is found");
        assert!(raw.skip);
    }

    #[test]
    fn unparseable_reply_yields_none() {
        assert!(parse_raw_fix("no json here").is_none());
        assert!(parse_raw_fix("{not json}").is_none());
    }

    #[test]
    fn strip_code_fence_leaves_unfenced_text_alone() {
        assert_eq!(strip_code_fence("plain\ntext"), "plain\ntext");
        assert_eq!(strip_code_fence("```\nfenced\n```"), "fenced");
        assert_eq!(strip_code_fence("```rust\nfenced\n```"), "fenced");
    }

    #[test]
    fn rejection_messages_name_the_anchor() {
        let r = Rejection::AnchorMissing {
            path: "a.rs".into(),
            start: 3,
            end: 5,
        };
        let msg = r.to_string();
        assert!(msg.contains("3-5"));
        assert!(msg.contains("a.rs"));
    }

    #[test]
    fn autofix_limits_default_is_bounded() {
        assert_eq!(AutofixLimits::default().max_fixes, 10);
    }
}
