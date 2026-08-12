//! Syntax highlighting for code blocks

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight(&self, code: &str, language: Option<&str>) -> String {
        let syntax = if let Some(lang) = language {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.syntax_set.find_syntax_plain_text()
        };

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut highlighted = String::new();
        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            highlighted.push_str(&escaped);
        }

        highlighted
    }

    #[allow(dead_code)]
    pub fn highlight_ranges(&self, code: &str, language: Option<&str>) -> Vec<(Style, String)> {
        let syntax = if let Some(lang) = language {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.syntax_set.find_syntax_plain_text()
        };

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut ranges = Vec::new();
        for line in LinesWithEndings::from(code) {
            let line_ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            for (style, text) in line_ranges {
                ranges.push((style, text.to_string()));
            }
        }

        ranges
    }

    #[allow(dead_code)]
    pub fn detect_language(code: &str) -> Option<String> {
        // Simple heuristics to detect language
        if code.contains("fn ") && code.contains("->") {
            Some("rust".to_string())
        } else if code.contains("def ") || code.contains("import ") {
            Some("python".to_string())
        } else if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            Some("javascript".to_string())
        } else if code.contains("package ") && code.contains("func ") {
            Some("go".to_string())
        } else {
            None
        }
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

// ── ANSI escape codes ────────────────────────────────────────────────────────
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const UNDERLINE: &str = "\x1b[4m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
#[allow(dead_code)]
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";
const WHITE: &str = "\x1b[37m";
const BRIGHT_BLACK: &str = "\x1b[90m"; // gray
const BG_GRAY: &str = "\x1b[48;5;236m"; // dark gray background for code blocks
const BRIGHT_CYAN: &str = "\x1b[96m";
const BRIGHT_GREEN: &str = "\x1b[92m";
const BRIGHT_YELLOW: &str = "\x1b[93m";

/// Render inline markdown formatting within a line (bold, italic, inline code, links).
fn render_inline_markdown(line: &str) -> String {
    let mut result = String::with_capacity(line.len() + 64);
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Inline code: `code`
        if chars[i] == '`' && i + 1 < len {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '`') {
                let code: String = chars[i + 1..i + 1 + end].iter().collect();
                result.push_str(BG_GRAY);
                result.push_str(BRIGHT_CYAN);
                result.push_str(&code);
                result.push_str(RESET);
                i += end + 2;
                continue;
            }
        }
        // Bold + Italic: ***text*** or ___text___
        if i + 2 < len && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*' {
            if let Some(end) = find_closing(&chars, i + 3, &['*', '*', '*']) {
                let text: String = chars[i + 3..end].iter().collect();
                result.push_str(BOLD);
                result.push_str(ITALIC);
                result.push_str(&text);
                result.push_str(RESET);
                i = end + 3;
                continue;
            }
        }
        // Bold: **text** or __text__
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_closing(&chars, i + 2, &['*', '*']) {
                let text: String = chars[i + 2..end].iter().collect();
                result.push_str(BOLD);
                result.push_str(WHITE);
                result.push_str(&text);
                result.push_str(RESET);
                i = end + 2;
                continue;
            }
        }
        // Italic: *text* (single)
        if chars[i] == '*' && i + 1 < len && chars[i + 1] != ' ' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '*') {
                let text: String = chars[i + 1..i + 1 + end].iter().collect();
                result.push_str(ITALIC);
                result.push_str(&text);
                result.push_str(RESET);
                i += end + 2;
                continue;
            }
        }
        // Link: [text](url) — show text underlined, dim the URL
        if chars[i] == '[' {
            if let Some(close_bracket) = chars[i + 1..].iter().position(|&c| c == ']') {
                let text_end = i + 1 + close_bracket;
                if text_end + 1 < len && chars[text_end + 1] == '(' {
                    if let Some(close_paren) = chars[text_end + 2..].iter().position(|&c| c == ')')
                    {
                        let link_text: String = chars[i + 1..text_end].iter().collect();
                        let url: String = chars[text_end + 2..text_end + 2 + close_paren]
                            .iter()
                            .collect();
                        result.push_str(UNDERLINE);
                        result.push_str(CYAN);
                        result.push_str(&link_text);
                        result.push_str(RESET);
                        result.push_str(DIM);
                        result.push(' ');
                        result.push_str(&url);
                        result.push_str(RESET);
                        i = text_end + 2 + close_paren + 1;
                        continue;
                    }
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Find closing delimiter sequence in chars starting at `from`.
fn find_closing(chars: &[char], from: usize, delim: &[char]) -> Option<usize> {
    let dlen = delim.len();
    if from + dlen > chars.len() {
        return None;
    }
    (from..=chars.len() - dlen).find(|&i| chars[i..i + dlen] == *delim)
}

/// Render a single markdown prose line with ANSI colors.
fn render_markdown_line(line: &str) -> String {
    let trimmed = line.trim_start();

    // Headings: # ## ### ####
    if trimmed.starts_with("#### ") {
        return format!(
            "{}{}{}{}\n",
            BOLD,
            BLUE,
            render_inline_markdown(trimmed.trim_start_matches('#').trim()),
            RESET
        );
    }
    if trimmed.starts_with("### ") {
        return format!(
            "{}{}{}{}\n",
            BOLD,
            MAGENTA,
            render_inline_markdown(trimmed.trim_start_matches('#').trim()),
            RESET
        );
    }
    if trimmed.starts_with("## ") {
        return format!(
            "\n{}{}{}{}\n",
            BOLD,
            CYAN,
            render_inline_markdown(trimmed.trim_start_matches('#').trim()),
            RESET
        );
    }
    if trimmed.starts_with("# ") {
        return format!(
            "\n{}{}{}{}\n",
            BOLD,
            BRIGHT_GREEN,
            render_inline_markdown(trimmed.trim_start_matches('#').trim()),
            RESET
        );
    }

    // Horizontal rule: --- or *** or ___
    if trimmed.len() >= 3
        && (trimmed.chars().all(|c| c == '-')
            || trimmed.chars().all(|c| c == '*')
            || trimmed.chars().all(|c| c == '_'))
    {
        return format!(
            "{}{}─────────────────────────────────────────{}\n",
            DIM, BRIGHT_BLACK, RESET
        );
    }

    // Task list: - [ ] or - [x] (must be before unordered list)
    if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        return format!("  {}☐{} {}\n", DIM, RESET, render_inline_markdown(rest));
    }
    if let Some(rest) = trimmed
        .strip_prefix("- [x] ")
        .or_else(|| trimmed.strip_prefix("- [X] "))
    {
        return format!(
            "  {}{}☑{} {}\n",
            GREEN,
            BOLD,
            RESET,
            render_inline_markdown(rest)
        );
    }

    // Blockquote: > text
    if let Some(rest) = trimmed.strip_prefix("> ") {
        return format!(
            "  {}{}│{} {}{}\n",
            DIM,
            GREEN,
            RESET,
            ITALIC,
            render_inline_markdown(rest)
        );
    }
    if trimmed == ">" {
        return format!("  {}{}│{}\n", DIM, GREEN, RESET);
    }

    // Unordered list: - item, * item, + item
    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        let indent = line.len() - trimmed.len();
        let pad: String = " ".repeat(indent);
        return format!(
            "{}{}  •{} {}\n",
            pad,
            BRIGHT_YELLOW,
            RESET,
            render_inline_markdown(rest)
        );
    }

    // Ordered list: 1. item, 2. item, etc.
    if let Some(dot_pos) = trimmed.find(". ") {
        let prefix = &trimmed[..dot_pos];
        if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
            let rest = &trimmed[dot_pos + 2..];
            let indent = line.len() - trimmed.len();
            let pad: String = " ".repeat(indent);
            return format!(
                "{}{}{}{}.{} {}\n",
                pad,
                BRIGHT_YELLOW,
                BOLD,
                prefix,
                RESET,
                render_inline_markdown(rest)
            );
        }
    }

    // Empty line
    if trimmed.is_empty() {
        return "\n".to_string();
    }

    // Regular text with inline formatting
    format!("{}\n", render_inline_markdown(line))
}

/// Extract and highlight code blocks from markdown-style text,
/// and render markdown prose with ANSI colors (headers, bold, italic,
/// inline code, lists, blockquotes, links, horizontal rules).
/// Reasoning tags a model may wrap its private deliberation in. Matched on the
/// local name, so a namespaced `<mm:think>` counts too.
///
/// Longest alternative first: `(think|thinking)` would try `think` against
/// `thinking>` and fail the word boundary.
fn reasoning_open_re() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| {
        regex::Regex::new(r"(?is)<\s*(?:[a-z][\w.-]*:)?(thinking|think)\b[^>]*>")
            .expect("hardcoded regex is valid")
    })
}

/// Byte index just past the closing tag for `name` in `haystack`, if present.
/// Namespace- and case-tolerant, mirroring the opening matcher.
fn reasoning_close_end(haystack: &str, name: &str) -> Option<usize> {
    let lower = haystack.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    let mut from = 0usize;
    while let Some(rel) = lower[from..].find("</") {
        let open = from + rel;
        let close = lower[open..].find('>').map(|i| open + i)?;
        let inner = lower[open + 2..close].trim();
        // Strip a namespace prefix, then compare the local name.
        let local = inner.rsplit(':').next().unwrap_or(inner).trim();
        if local == name {
            return Some(close + 1);
        }
        from = close + 1;
    }
    None
}

/// Dim `<thinking>…</thinking>` spans — the tags and everything between them.
///
/// A model's private deliberation used to print at the same weight as its
/// answer, so there was no way to skip it by eye. Dimming rather than
/// stripping keeps it auditable: you can still read why the model answered as
/// it did, but the answer is what stands out.
///
/// Runs *after* syntax highlighting, so a span can already contain ANSI. A
/// plain `DIM … RESET` wrapper would stop dimming at the span's first interior
/// `RESET`, so each one re-asserts `DIM`.
///
/// An unclosed block dims to end of text — it is still reasoning, and the
/// alternative is leaving a stray `<thinking>` at full weight.
pub fn dim_reasoning_blocks(text: &str) -> String {
    let re = reasoning_open_re();
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(caps) = re.captures(rest) {
        let whole = caps.get(0).expect("group 0 always present");
        let name = caps.get(1).expect("group 1 is in the pattern").as_str();

        out.push_str(&rest[..whole.start()]);
        let from_open = &rest[whole.start()..];

        let (span, tail) = match reasoning_close_end(&from_open[whole.len()..], name) {
            Some(end) => from_open.split_at(whole.len() + end),
            None => (from_open, ""),
        };

        out.push_str(DIM);
        out.push_str(&span.replace(RESET, &format!("{}{}", RESET, DIM)));
        out.push_str(RESET);
        rest = tail;
    }

    out.push_str(rest);
    out
}

pub fn highlight_code_blocks(text: &str) -> String {
    let highlighter = SyntaxHighlighter::new();
    let mut result = String::new();
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut language: Option<String> = None;

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End of code block — syntax highlight the accumulated code
                let lang_label = language.as_deref().unwrap_or("text");
                // Code block header with language label
                result.push_str(&format!(
                    "  {}{}┌─ {} ─{}\n",
                    DIM, BRIGHT_BLACK, lang_label, RESET
                ));
                let highlighted = highlighter.highlight(&code_buffer, language.as_deref());
                let lines: Vec<&str> = highlighted.lines().collect();
                let line_count = lines.len();
                let width = if line_count > 0 {
                    format!("{}", line_count).len()
                } else {
                    1
                };
                // Add left gutter with line numbers + background color
                for (i, hl_line) in lines.iter().enumerate() {
                    let line_num = i + 1;
                    result.push_str(&format!(
                        "  {}{}{:>width$} │{} {}{}{}\n",
                        DIM,
                        BRIGHT_BLACK,
                        line_num,
                        RESET,
                        BG_GRAY,
                        hl_line,
                        RESET,
                        width = width,
                    ));
                }
                result.push_str(&format!("  {}{}└─────────{}\n", DIM, BRIGHT_BLACK, RESET));
                code_buffer.clear();
                language = None;
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                language = line.strip_prefix("```").map(|s| s.trim().to_string());
                if language.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                    language = None;
                }
            }
        } else if in_code_block {
            code_buffer.push_str(line);
            code_buffer.push('\n');
        } else {
            result.push_str(&render_markdown_line(line));
        }
    }

    // Handle unclosed code block
    if in_code_block && !code_buffer.is_empty() {
        let highlighted = highlighter.highlight(&code_buffer, language.as_deref());
        result.push_str(&highlighted);
        result.push_str(RESET);
    }

    // Last, so it sees the ANSI the markdown/code passes emitted and can keep
    // a reasoning span dim across it. Every caller here renders model output.
    dim_reasoning_blocks(&result)
}

// ── Dark background box for tool calls (Claude Code style) ──────────────────
const BG_DARK: &str = "\x1b[48;5;235m"; // dark background for tool call boxes
#[allow(dead_code)] // only referenced by `format_step_result` — see its note
const FG_GREEN_CHECK: &str = "\x1b[38;5;114m"; // muted green for checkmarks
const FG_RED_CROSS: &str = "\x1b[38;5;167m"; // muted red for failures

/// Get terminal width (fallback to 80).
fn terminal_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

/// Render a tool call in a dark background box (Claude Code style).
#[allow(dead_code)]
/// ```text
///   ✓ ls -la src/ | head -20
/// ```
pub fn format_tool_call(tool_name: &str, summary: &str) -> String {
    let width = terminal_width();
    let content = format!(" {} {}", tool_name, summary);
    let padded_len = width.saturating_sub(2);
    let display = if content.len() < padded_len {
        format!("{}{}", content, " ".repeat(padded_len - content.len()))
    } else {
        content[..padded_len].to_string()
    };
    format!(
        "\n{}{}  {}  {}{}\n",
        BG_DARK, BRIGHT_YELLOW, display, RESET, ""
    )
}

/// Format a tool call pending approval (with tool name in dark box).
pub fn format_tool_pending(tool_name: &str, summary: &str) -> String {
    let width = terminal_width();
    let content = format!("  {} {}", tool_name, summary);
    let padded_len = width.saturating_sub(2);
    let display = if content.len() < padded_len {
        format!("{}{}", content, " ".repeat(padded_len - content.len()))
    } else {
        content[..padded_len].to_string()
    };
    format!("\n{}{}{}{}", BG_DARK, WHITE, display, RESET)
}

/// Format a completed step result in a dark background box with checkmark/cross.
///
/// Superseded in the REPL by [`crate::agent_render`], which folds consecutive
/// tool calls into one line instead of printing a box per step. Kept as public
/// API — still the right renderer for a one-step-per-line surface — so this is
/// dead only from the binary's private view of this module.
#[allow(dead_code)]
pub fn format_step_result(_step_num: usize, tool_summary: &str, success: bool) -> String {
    let width = terminal_width();
    let icon = if success { "\u{2713}" } else { "\u{2717}" }; // ✓ or ✗
    let icon_color = if success {
        FG_GREEN_CHECK
    } else {
        FG_RED_CROSS
    };
    let content = format!(" {} {}", icon, tool_summary);
    let padded_len = width.saturating_sub(2);
    let display = if content.len() < padded_len {
        format!("{}{}", content, " ".repeat(padded_len - content.len()))
    } else {
        content[..padded_len].to_string()
    };
    format!("\n{}{} {}{}", BG_DARK, icon_color, display, RESET)
}

/// Format a "thinking" status line (dim gray, like Claude Code).
#[allow(dead_code)]
pub fn format_thinking(duration_secs: u64) -> String {
    format!(
        "{}Thought for {} second{}{}",
        BRIGHT_BLACK,
        duration_secs,
        if duration_secs == 1 { "" } else { "s" },
        RESET
    )
}

/// Format agent task start banner.
pub fn format_agent_start(task: &str, policy: &str) -> String {
    let task_preview = if task.len() > 120 {
        let end = task
            .char_indices()
            .nth(120)
            .map(|(i, _)| i)
            .unwrap_or(task.len());
        format!("{}...", &task[..end])
    } else {
        task.to_string()
    };
    format!(
        "\n{}{} Agent {}{}  {}\n{}  Policy: {}{}  Press Ctrl+C to stop{}\n",
        BOLD, BRIGHT_YELLOW, RESET, BOLD, task_preview, BRIGHT_BLACK, policy, "  |", RESET
    )
}

/// Format a human-readable description of a tool call (for step output).
pub fn describe_tool_action(tool_name: &str, summary: &str) -> String {
    match tool_name {
        "read_file" => {
            let path = summary
                .strip_prefix("read_file(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(summary);
            format!("Reading {}", path)
        }
        "write_file" => {
            // summary: "write_file(path, N lines)"
            let inner = summary
                .strip_prefix("write_file(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(summary);
            if let Some((path, rest)) = inner.split_once(',') {
                format!("Writing {} ({})", path.trim(), rest.trim())
            } else {
                format!("Writing {}", inner)
            }
        }
        "apply_patch" => {
            let inner = summary
                .strip_prefix("apply_patch(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(summary);
            if let Some((path, rest)) = inner.split_once(',') {
                format!("Patching {} ({})", path.trim(), rest.trim())
            } else {
                format!("Patching {}", inner)
            }
        }
        "bash" => {
            let inner = summary
                .strip_prefix("bash(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(summary);
            format!("Running: {}", inner)
        }
        "search_files" => {
            let inner = summary
                .strip_prefix("search_files(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(summary);
            format!("Searching: {}", inner)
        }
        "list_directory" => {
            let inner = summary
                .strip_prefix("list_directory(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(summary);
            format!("Listing {}", inner)
        }
        "web_search" => format!("Searching web: {}", summary),
        "fetch_url" => format!("Fetching URL: {}", summary),
        "think" => "Thinking...".to_string(),
        "task_complete" => "Task complete".to_string(),
        "spawn_agent" => "Spawning sub-agent".to_string(),
        _ => summary.to_string(),
    }
}

/// Format agent completion message with change summary.
pub fn format_agent_complete(summary: &str) -> String {
    format!("\n{}{}Agent complete:{} {}", BOLD, GREEN, RESET, summary)
}

/// Format a change summary showing what files were modified.
pub fn format_change_summary(steps: &[(String, String, bool)]) -> String {
    // steps: [(tool_name, summary, success)]
    let writes: Vec<&str> = steps
        .iter()
        .filter(|(tool, _, success)| *success && (tool == "write_file" || tool == "apply_patch"))
        .filter_map(|(_, summary, _)| summary.split('(').nth(1).and_then(|s| s.split(',').next()))
        .collect();
    let commands: Vec<&str> = steps
        .iter()
        .filter(|(tool, _, success)| *success && tool == "bash")
        .filter_map(|(_, summary, _)| {
            summary
                .strip_prefix("bash(")
                .and_then(|s| s.strip_suffix(')'))
        })
        .collect();

    let mut out = String::new();
    if !writes.is_empty() {
        out.push_str(&format!("\n   {}Files modified:{} ", BOLD, RESET));
        let deduped: Vec<&str> = {
            let mut seen = std::collections::HashSet::new();
            writes.into_iter().filter(|w| seen.insert(*w)).collect()
        };
        out.push_str(&deduped.join(", "));
    }
    if !commands.is_empty() {
        let count = commands.len();
        out.push_str(&format!("\n   {}Commands run:{} {}", BOLD, RESET, count));
    }
    let total = steps.len();
    let succeeded = steps.iter().filter(|(_, _, s)| *s).count();
    out.push_str(&format!(
        "\n   {}Steps:{} {}/{} succeeded",
        BOLD, RESET, succeeded, total
    ));
    out
}

/// Format agent error message.
pub fn format_agent_error(error: &str) -> String {
    format!("\n{}{}Error:{} {}", BOLD, FG_RED_CROSS, RESET, error)
}

/// Format the REPL prompt. Plain text to avoid rustyline ANSI width issues.
pub fn colored_prompt(provider_name: &str, model: Option<&str>) -> String {
    if let Some(m) = model {
        format!("[vibecli {} ({})] > ", provider_name, m)
    } else {
        format!("[vibecli {}] > ", provider_name)
    }
}

/// Format full tool output with dim styling and a max line limit.
pub fn format_tool_output(output: &str, _success: bool) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let max_lines = 30;
    let mut result = String::new();
    let display_lines = if lines.len() > max_lines {
        &lines[..max_lines]
    } else {
        &lines[..]
    };
    for line in display_lines {
        result.push_str(&format!("  {}{}{}\n", DIM, line, RESET));
    }
    if lines.len() > max_lines {
        result.push_str(&format!(
            "  {}... ({} more lines){}\n",
            DIM,
            lines.len() - max_lines,
            RESET
        ));
    }
    result
}

/// Format tool output preview (first line, dimmed).
#[allow(dead_code)]
pub fn format_tool_output_preview(output: &str, success: bool) -> String {
    let first_line = output.lines().next().unwrap_or("");
    let truncated = if first_line.len() > 100 {
        format!("{}...", &first_line[..97])
    } else {
        first_line.to_string()
    };
    if success {
        format!("   {}{}{}", DIM, truncated, RESET)
    } else {
        format!("   {}{}{}{}", FG_RED_CROSS, DIM, truncated, RESET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── dim_reasoning_blocks ─────────────────────────────────────────────────

    #[test]
    fn dims_the_tags_and_the_text_between_them() {
        let out = dim_reasoning_blocks("<thinking>deliberating</thinking>Answer.");
        assert_eq!(
            out,
            format!("{}<thinking>deliberating</thinking>{}Answer.", DIM, RESET)
        );
    }

    #[test]
    fn leaves_text_without_reasoning_untouched() {
        let plain = "Here's a quick Python program.";
        assert_eq!(dim_reasoning_blocks(plain), plain);
    }

    #[test]
    fn dims_short_and_namespaced_tag_names() {
        for (open, close) in [
            ("<think>", "</think>"),
            ("<mm:think>", "</mm:think>"),
            ("<THINKING>", "</THINKING>"),
        ] {
            let out = dim_reasoning_blocks(&format!("{open}x{close}after"));
            assert!(out.starts_with(DIM), "{open} should start a dim span");
            assert!(out.ends_with("after"), "{open} should not dim the tail");
        }
    }

    #[test]
    fn reasserts_dim_after_interior_reset() {
        // The highlighter emits RESET inside prose (inline code, emphasis). A
        // bare DIM…RESET wrapper would stop dimming at the first one.
        let highlighted = format!("<thinking>see {}{}numpy{}{}</thinking>tail", CYAN, "`", RESET, "");
        let out = dim_reasoning_blocks(&highlighted);
        let span_end = out.find("</thinking>").expect("closing tag survives");
        let span = &out[..span_end];
        let resets = span.matches(RESET).count();
        let dims = span.matches(DIM).count();
        assert_eq!(
            dims,
            resets + 1,
            "every interior RESET must be followed by a DIM, plus the opening one: {span:?}"
        );
    }

    #[test]
    fn unclosed_block_dims_to_end_of_text() {
        let out = dim_reasoning_blocks("<thinking>never closed");
        assert!(out.starts_with(DIM));
        assert!(out.ends_with(RESET));
        assert!(out.contains("never closed"));
    }

    #[test]
    fn dims_each_of_several_blocks_independently() {
        let out = dim_reasoning_blocks("<think>a</think>mid<think>b</think>end");
        assert!(out.contains(&format!("{}mid", RESET)), "mid stays undimmed");
        assert!(out.ends_with("end"), "tail stays undimmed: {out:?}");
        assert_eq!(out.matches(DIM).count(), 2, "one DIM per block");
    }

    #[test]
    fn a_lone_angle_bracket_is_not_a_tag() {
        let prose = "if a < b and c > d, return";
        assert_eq!(dim_reasoning_blocks(prose), prose);
    }

    #[test]
    fn highlight_code_blocks_dims_reasoning_in_prose() {
        let out = highlight_code_blocks("<thinking>plan</thinking>\nAnswer.\n");
        assert!(out.contains(DIM), "reasoning should be dimmed end to end");
        assert!(out.contains("plan"), "reasoning text is kept, not stripped");
    }

    // ── detect_language ──────────────────────────────────────────────────────

    #[test]
    fn detect_language_rust() {
        assert_eq!(
            SyntaxHighlighter::detect_language("fn main() -> () {}"),
            Some("rust".to_string())
        );
    }

    #[test]
    fn detect_language_python() {
        assert_eq!(
            SyntaxHighlighter::detect_language("def hello():\n    pass"),
            Some("python".to_string())
        );
    }

    #[test]
    fn detect_language_python_import() {
        assert_eq!(
            SyntaxHighlighter::detect_language("import os\nos.getcwd()"),
            Some("python".to_string())
        );
    }

    #[test]
    fn detect_language_javascript_function() {
        assert_eq!(
            SyntaxHighlighter::detect_language("function greet(name) { return name; }"),
            Some("javascript".to_string())
        );
    }

    #[test]
    fn detect_language_javascript_const() {
        assert_eq!(
            SyntaxHighlighter::detect_language("const x = 42;"),
            Some("javascript".to_string())
        );
    }

    #[test]
    fn detect_language_javascript_let() {
        assert_eq!(
            SyntaxHighlighter::detect_language("let y = 'hello';"),
            Some("javascript".to_string())
        );
    }

    #[test]
    fn detect_language_go() {
        assert_eq!(
            SyntaxHighlighter::detect_language("package main\nfunc main() {}"),
            Some("go".to_string())
        );
    }

    #[test]
    fn detect_language_none_for_prose() {
        assert_eq!(
            SyntaxHighlighter::detect_language("This is just some plain English text."),
            None
        );
    }

    #[test]
    fn detect_language_empty() {
        assert_eq!(SyntaxHighlighter::detect_language(""), None);
    }

    // ── SyntaxHighlighter::new / highlight ───────────────────────────────────

    #[test]
    fn highlighter_new_does_not_panic() {
        let _h = SyntaxHighlighter::new();
    }

    #[test]
    fn highlight_plain_text_returns_non_empty() {
        let h = SyntaxHighlighter::new();
        let out = h.highlight("hello world\n", None);
        assert!(out.contains("hello world"));
    }

    #[test]
    fn highlight_with_language_returns_ansi() {
        let h = SyntaxHighlighter::new();
        let out = h.highlight("fn main() {}\n", Some("rust"));
        // ANSI codes start with ESC
        assert!(out.contains('\x1b'));
    }

    #[test]
    fn highlight_unknown_language_falls_back() {
        let h = SyntaxHighlighter::new();
        // Unknown language should not panic; falls back to plain text
        let out = h.highlight("hello\n", Some("nonexistent_language_xyz"));
        assert!(out.contains("hello"));
    }

    #[test]
    fn highlight_ranges_returns_non_empty() {
        let h = SyntaxHighlighter::new();
        let ranges = h.highlight_ranges("let x = 42;\n", Some("js"));
        assert!(!ranges.is_empty());
    }

    // ── highlight_code_blocks ────────────────────────────────────────────────

    #[test]
    fn highlight_code_blocks_no_code() {
        let input = "Just some text\nand more text\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("Just some text"));
        assert!(out.contains("and more text"));
    }

    #[test]
    fn highlight_code_blocks_with_fenced_block() {
        let input = "Before\n```rust\nfn foo() {}\n```\nAfter\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("Before"));
        assert!(out.contains("After"));
        // The code block should be highlighted (contains ANSI reset at minimum)
        assert!(out.contains("\x1b[0m"));
    }

    #[test]
    fn highlight_code_blocks_no_language_tag() {
        let input = "```\nhello world\n```\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("hello world") || out.contains('\x1b'));
    }

    #[test]
    fn highlight_code_blocks_unclosed_fence_no_panic() {
        let input = "```rust\nfn main() {}\n";
        // Should not panic even if the fence is never closed
        let _out = highlight_code_blocks(input);
    }

    #[test]
    fn highlight_code_blocks_empty_input() {
        let out = highlight_code_blocks("");
        assert!(out.is_empty());
    }

    #[test]
    fn highlight_code_blocks_multiple_blocks() {
        let input = "Text\n```py\nx = 1\n```\nMiddle\n```js\nvar y = 2;\n```\nEnd\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("Text"));
        assert!(out.contains("Middle"));
        assert!(out.contains("End"));
    }

    // ── Markdown rendering ──────────────────────────────────────────────────

    #[test]
    fn render_h1() {
        let out = render_markdown_line("# Hello World");
        assert!(out.contains("Hello World"));
        assert!(out.contains(BOLD));
        assert!(out.contains(BRIGHT_GREEN));
    }

    #[test]
    fn render_h2() {
        let out = render_markdown_line("## Section");
        assert!(out.contains("Section"));
        assert!(out.contains(CYAN));
    }

    #[test]
    fn render_h3() {
        let out = render_markdown_line("### Subsection");
        assert!(out.contains("Subsection"));
        assert!(out.contains(MAGENTA));
    }

    #[test]
    fn render_h4() {
        let out = render_markdown_line("#### Detail");
        assert!(out.contains("Detail"));
        assert!(out.contains(BLUE));
    }

    #[test]
    fn render_unordered_list_dash() {
        let out = render_markdown_line("- item one");
        assert!(out.contains("•"));
        assert!(out.contains("item one"));
    }

    #[test]
    fn render_unordered_list_star() {
        let out = render_markdown_line("* item two");
        assert!(out.contains("•"));
        assert!(out.contains("item two"));
    }

    #[test]
    fn render_ordered_list() {
        let out = render_markdown_line("1. first");
        assert!(out.contains("1."));
        assert!(out.contains("first"));
        assert!(out.contains(BRIGHT_YELLOW));
    }

    #[test]
    fn render_blockquote() {
        let out = render_markdown_line("> quoted text");
        assert!(out.contains("│"));
        assert!(out.contains("quoted text"));
        assert!(out.contains(ITALIC));
    }

    #[test]
    fn render_horizontal_rule() {
        let out = render_markdown_line("---");
        assert!(out.contains("─"));
        assert!(out.contains(DIM));
    }

    #[test]
    fn render_task_list_unchecked() {
        let out = render_markdown_line("- [ ] todo");
        assert!(out.contains("☐"));
        assert!(out.contains("todo"));
    }

    #[test]
    fn render_task_list_checked() {
        let out = render_markdown_line("- [x] done");
        assert!(out.contains("☑"));
        assert!(out.contains("done"));
    }

    #[test]
    fn render_empty_line() {
        let out = render_markdown_line("");
        assert_eq!(out, "\n");
    }

    // ── Inline markdown ─────────────────────────────────────────────────────

    #[test]
    fn inline_code() {
        let out = render_inline_markdown("use `cargo build` here");
        assert!(out.contains("cargo build"));
        assert!(out.contains(BRIGHT_CYAN));
        assert!(out.contains(BG_GRAY));
    }

    #[test]
    fn inline_bold() {
        let out = render_inline_markdown("this is **bold** text");
        assert!(out.contains("bold"));
        assert!(out.contains(BOLD));
    }

    #[test]
    fn inline_italic() {
        let out = render_inline_markdown("this is *italic* text");
        assert!(out.contains("italic"));
        assert!(out.contains(ITALIC));
    }

    #[test]
    fn inline_bold_italic() {
        let out = render_inline_markdown("this is ***both*** styled");
        assert!(out.contains("both"));
        assert!(out.contains(BOLD));
        assert!(out.contains(ITALIC));
    }

    #[test]
    fn inline_link() {
        let out = render_inline_markdown("click [here](https://example.com) now");
        assert!(out.contains("here"));
        assert!(out.contains("https://example.com"));
        assert!(out.contains(UNDERLINE));
        assert!(out.contains(CYAN));
    }

    #[test]
    fn inline_no_formatting() {
        let out = render_inline_markdown("plain text");
        assert_eq!(out, "plain text");
    }

    #[test]
    fn inline_unclosed_backtick() {
        // Should not panic and should include the backtick
        let out = render_inline_markdown("incomplete `code");
        assert!(out.contains('`'));
    }

    // ── Code block rendering with gutter ─────────────────────────────────────

    #[test]
    fn code_block_has_gutter() {
        let input = "```rust\nfn main() {}\n```\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("┌─"));
        assert!(out.contains("│"));
        assert!(out.contains("└─"));
        assert!(out.contains("rust"));
    }

    #[test]
    fn code_block_no_lang_shows_text() {
        let input = "```\nhello\n```\n";
        let out = highlight_code_blocks(input);
        assert!(out.contains("text"));
        assert!(out.contains("hello"));
    }

    // ── Formatting helpers ──────────────────────────────────────────────────

    #[test]
    fn format_tool_call_contains_name() {
        let out = format_tool_call("write_file", "path: main.rs");
        assert!(out.contains("write_file"));
        assert!(out.contains(BG_DARK));
    }

    #[test]
    fn format_tool_pending_contains_name() {
        let out = format_tool_pending("bash", "ls -la");
        assert!(out.contains("bash"));
        assert!(out.contains("ls -la"));
        assert!(out.contains(BG_DARK));
    }

    #[test]
    fn format_step_success() {
        let out = format_step_result(1, "wrote main.rs", true);
        assert!(out.contains("wrote main.rs"));
        assert!(out.contains("\u{2713}")); // checkmark
        assert!(out.contains(BG_DARK));
    }

    #[test]
    fn format_step_failure() {
        let out = format_step_result(2, "compile failed", false);
        assert!(out.contains("compile failed"));
        assert!(out.contains("\u{2717}")); // cross
    }

    #[test]
    fn format_thinking_singular() {
        let out = format_thinking(1);
        assert!(out.contains("1 second"));
        assert!(!out.contains("seconds"));
    }

    #[test]
    fn format_thinking_plural() {
        let out = format_thinking(5);
        assert!(out.contains("5 seconds"));
    }

    #[test]
    fn format_complete_msg() {
        let out = format_agent_complete("all done");
        assert!(out.contains("Agent complete:"));
        assert!(out.contains("all done"));
        assert!(out.contains(GREEN));
    }

    #[test]
    fn format_error_msg() {
        let out = format_agent_error("something broke");
        assert!(out.contains("Error:"));
        assert!(out.contains("something broke"));
    }

    #[test]
    fn colored_prompt_contains_provider() {
        let out = colored_prompt("ollama", None);
        assert!(out.contains("vibecli"));
        assert!(out.contains("ollama"));
        assert!(out.contains(">"));
    }

    #[test]
    fn format_tool_output_preview_success() {
        let out = format_tool_output_preview("file written successfully\nmore details", true);
        assert!(out.contains("file written successfully"));
        assert!(out.contains(DIM));
    }

    #[test]
    fn format_tool_output_preview_truncates_long() {
        let long = "x".repeat(150);
        let out = format_tool_output_preview(&long, true);
        assert!(out.contains("..."));
    }

    #[test]
    fn terminal_width_returns_positive() {
        assert!(terminal_width() > 0);
    }

    // ── Full markdown document ──────────────────────────────────────────────

    #[test]
    fn full_markdown_document() {
        let input = r#"# Title

## Introduction

This is a paragraph with **bold** and *italic* text.

- First item
- Second item with `inline code`

```rust
fn main() {
    println!("hello");
}
```

> A blockquote

1. Ordered item
2. Another item

---

### Conclusion

Visit [the docs](https://vibecody.dev) for more."#;
        let out = highlight_code_blocks(input);
        // Headers are colored
        assert!(out.contains(BRIGHT_GREEN)); // h1
        assert!(out.contains(CYAN)); // h2
        assert!(out.contains(MAGENTA)); // h3
                                        // Lists have bullets
        assert!(out.contains("•"));
        // Code block has gutter
        assert!(out.contains("┌─"));
        // Bold text is styled
        assert!(out.contains(BOLD));
        // Blockquote has pipe
        assert!(out.contains("│"));
        // Horizontal rule
        assert!(out.contains("─────"));
        // Link is underlined
        assert!(out.contains(UNDERLINE));
    }

    // ── find_closing ────────────────────────────────────────────────────────

    #[test]
    fn find_closing_found() {
        let chars: Vec<char> = "hello**world".chars().collect();
        assert_eq!(find_closing(&chars, 0, &['*', '*']), Some(5));
    }

    #[test]
    fn find_closing_not_found() {
        let chars: Vec<char> = "hello world".chars().collect();
        assert_eq!(find_closing(&chars, 0, &['*', '*']), None);
    }

    #[test]
    fn find_closing_at_end() {
        let chars: Vec<char> = "ab**".chars().collect();
        assert_eq!(find_closing(&chars, 0, &['*', '*']), Some(2));
    }
}
