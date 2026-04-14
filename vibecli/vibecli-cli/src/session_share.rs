#![allow(dead_code)]
//! Session HTML export + GitHub Gist sharing.
//! Pi-mono gap bridge: Phase C3.
//!
//! Adds HTML rendering with syntax-highlighted code blocks and private
//! GitHub Gist upload on top of the existing session_export.rs bundle.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

/// Role of a message in a shared session view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareRole {
    User,
    Assistant,
    System,
    Tool,
}

impl ShareRole {
    /// Lowercase role name used in data attributes and aria labels.
    pub fn as_str(&self) -> &str {
        match self {
            ShareRole::User => "user",
            ShareRole::Assistant => "assistant",
            ShareRole::System => "system",
            ShareRole::Tool => "tool",
        }
    }

    /// CSS class applied to the message wrapper `<div>`.
    pub fn css_class(&self) -> &str {
        match self {
            ShareRole::User => "msg-user",
            ShareRole::Assistant => "msg-assistant",
            ShareRole::System => "msg-system",
            ShareRole::Tool => "msg-tool",
        }
    }
}

// ---------------------------------------------------------------------------
// ShareMessage
// ---------------------------------------------------------------------------

/// A single message ready for HTML rendering.
#[derive(Debug, Clone)]
pub struct ShareMessage {
    pub role: ShareRole,
    pub content: String,
    pub tool_name: Option<String>,
    pub timestamp_ms: Option<u64>,
}

impl ShareMessage {
    /// Convenience constructor for a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ShareRole::User,
            content: content.into(),
            tool_name: None,
            timestamp_ms: None,
        }
    }

    /// Convenience constructor for an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ShareRole::Assistant,
            content: content.into(),
            tool_name: None,
            timestamp_ms: None,
        }
    }

    /// Convenience constructor for a tool-result message.
    pub fn tool(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: ShareRole::Tool,
            content: content.into(),
            tool_name: Some(name.into()),
            timestamp_ms: None,
        }
    }

    /// Convenience constructor with explicit timestamp.
    pub fn with_timestamp(mut self, ts_ms: u64) -> Self {
        self.timestamp_ms = Some(ts_ms);
        self
    }
}

// ---------------------------------------------------------------------------
// HtmlExportOptions
// ---------------------------------------------------------------------------

/// Configuration for the standalone HTML export.
#[derive(Debug, Clone)]
pub struct HtmlExportOptions {
    /// `<title>` element and visible heading text.
    pub title: String,
    /// Use dark-theme CSS variables (default: `true`).
    pub dark_theme: bool,
    /// Render `timestamp_ms` next to each message header.
    pub include_timestamps: bool,
    /// Detect ` ```lang … ``` ` fences and wrap with syntax-class `<pre>`.
    pub highlight_code_blocks: bool,
    /// Render tool call name badge in tool messages.
    pub include_tool_details: bool,
    /// Truncate message content to this byte length (None = unlimited).
    pub max_content_length: Option<usize>,
}

impl Default for HtmlExportOptions {
    fn default() -> Self {
        Self {
            title: "VibeCody Session".to_string(),
            dark_theme: true,
            include_timestamps: true,
            highlight_code_blocks: true,
            include_tool_details: true,
            max_content_length: None,
        }
    }
}

// ---------------------------------------------------------------------------
// HtmlExporter
// ---------------------------------------------------------------------------

/// Renders a slice of `ShareMessage`s to a self-contained HTML document.
///
/// All CSS is inlined; no external fonts, scripts, or stylesheets are
/// referenced so the file is fully offline-viewable.
pub struct HtmlExporter;

impl HtmlExporter {
    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Render `messages` to a standalone HTML string.
    pub fn export(messages: &[ShareMessage], opts: &HtmlExportOptions) -> String {
        let title_escaped = Self::escape_html(&opts.title);
        let body_html = Self::render_messages(messages, opts);
        let css = Self::build_css(opts.dark_theme);

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
{css}
</style>
</head>
<body>
<div class="session-container">
<h1 class="session-title">{title}</h1>
{body}
</div>
</body>
</html>"#,
            title = title_escaped,
            css = css,
            body = body_html,
        )
    }

    /// Replace ` ```lang … ``` ` fences with `<pre><code class="language-lang">…</code></pre>`.
    ///
    /// Supports multi-line blocks. Falls back gracefully when no language tag
    /// is present (uses `language-text`).
    pub fn highlight_fences(content: &str) -> String {
        let mut result = String::with_capacity(content.len() + 64);

        // Simple line-oriented parser: scan for lines starting with ```
        let mut output_lines: Vec<String> = Vec::new();
        let mut in_block = false;
        let mut lang = String::new();
        let mut block_lines: Vec<String> = Vec::new();

        for raw_line in content.lines() {
            let trimmed = raw_line.trim_start();
            if !in_block {
                if trimmed.starts_with("```") {
                    in_block = true;
                    lang = trimmed[3..].trim().to_string();
                    if lang.is_empty() {
                        lang = "text".to_string();
                    }
                    block_lines.clear();
                } else {
                    output_lines.push(Self::escape_html(raw_line));
                }
            } else if trimmed.trim() == "```" {
                // Closing fence
                let code_body = block_lines.join("\n");
                output_lines.push(format!(
                    r#"<pre><code class="language-{lang}">{code}</code></pre>"#,
                    lang = Self::escape_html(&lang),
                    code = Self::escape_html(&code_body),
                ));
                in_block = false;
                lang.clear();
                block_lines.clear();
            } else {
                block_lines.push(raw_line.to_string());
            }
        }

        // Un-closed fence — emit as plain pre block
        if in_block && !block_lines.is_empty() {
            let code_body = block_lines.join("\n");
            output_lines.push(format!(
                r#"<pre><code class="language-{lang}">{code}</code></pre>"#,
                lang = Self::escape_html(&lang),
                code = Self::escape_html(&code_body),
            ));
        }

        result.push_str(&output_lines.join("\n"));
        result
    }

    /// Escape `&`, `<`, `>`, `"`, and `'` for safe HTML embedding.
    pub fn escape_html(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for ch in s.chars() {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#39;"),
                c => out.push(c),
            }
        }
        out
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn render_messages(messages: &[ShareMessage], opts: &HtmlExportOptions) -> String {
        messages
            .iter()
            .map(|msg| Self::render_message(msg, opts))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_message(msg: &ShareMessage, opts: &HtmlExportOptions) -> String {
        let css_class = msg.role.css_class();
        let role_label = msg.role.as_str().to_uppercase();

        // Timestamp badge
        let ts_badge = if opts.include_timestamps {
            match msg.timestamp_ms {
                Some(ts) => format!(r#" <span class="msg-ts">{ts}</span>"#, ts = ts),
                None => String::new(),
            }
        } else {
            String::new()
        };

        // Tool name badge
        let tool_badge = if opts.include_tool_details {
            match &msg.tool_name {
                Some(name) => format!(
                    r#" <span class="tool-name">{}</span>"#,
                    Self::escape_html(name)
                ),
                None => String::new(),
            }
        } else {
            String::new()
        };

        // Possibly truncate content
        let raw_content = match opts.max_content_length {
            Some(max) if msg.content.len() > max => {
                let truncated = &msg.content[..max];
                format!("{truncated}… [truncated]")
            }
            _ => msg.content.clone(),
        };

        // Possibly highlight fences
        let rendered_content = if opts.highlight_code_blocks {
            Self::highlight_fences(&raw_content)
        } else {
            Self::escape_html(&raw_content)
        };

        // Expandable tool messages use <details>
        if msg.role == ShareRole::Tool && opts.include_tool_details {
            let summary_label = match &msg.tool_name {
                Some(n) => format!("Tool: {}", Self::escape_html(n)),
                None => "Tool output".to_string(),
            };
            format!(
                r#"<div class="{css_class}">
<details>
<summary><span class="msg-role">{role_label}</span>{tool_badge}{ts_badge} — {summary}</summary>
<div class="msg-content">{content}</div>
</details>
</div>"#,
                css_class = css_class,
                role_label = role_label,
                tool_badge = tool_badge,
                ts_badge = ts_badge,
                summary = summary_label,
                content = rendered_content,
            )
        } else {
            format!(
                r#"<div class="{css_class}">
<div class="msg-header"><span class="msg-role">{role_label}</span>{tool_badge}{ts_badge}</div>
<div class="msg-content">{content}</div>
</div>"#,
                css_class = css_class,
                role_label = role_label,
                tool_badge = tool_badge,
                ts_badge = ts_badge,
                content = rendered_content,
            )
        }
    }

    fn build_css(dark: bool) -> &'static str {
        if dark {
            DARK_CSS
        } else {
            LIGHT_CSS
        }
    }
}

// ---------------------------------------------------------------------------
// Embedded CSS themes
// ---------------------------------------------------------------------------

const DARK_CSS: &str = r#":root {
  --bg: #1e1e2e;
  --bg-card: #252537;
  --fg: #cdd6f4;
  --fg-muted: #a6adc8;
  --accent-user: #89b4fa;
  --accent-assistant: #a6e3a1;
  --accent-system: #f9e2af;
  --accent-tool: #cba6f7;
  --border: #313244;
  --code-bg: #181825;
  --ts-fg: #585b70;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body { background: var(--bg); color: var(--fg); font-family: system-ui, sans-serif; line-height: 1.6; }
.session-container { max-width: 860px; margin: 0 auto; padding: 2rem 1rem; }
.session-title { font-size: 1.5rem; margin-bottom: 1.5rem; color: var(--fg); border-bottom: 1px solid var(--border); padding-bottom: .5rem; }
.msg-user, .msg-assistant, .msg-system, .msg-tool { background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 1rem; margin-bottom: 1rem; }
.msg-user   { border-left: 3px solid var(--accent-user); }
.msg-assistant { border-left: 3px solid var(--accent-assistant); }
.msg-system { border-left: 3px solid var(--accent-system); }
.msg-tool   { border-left: 3px solid var(--accent-tool); }
.msg-header { margin-bottom: .5rem; }
.msg-role { font-weight: 700; font-size: .8rem; letter-spacing: .08em; text-transform: uppercase; }
.msg-user    .msg-role { color: var(--accent-user); }
.msg-assistant .msg-role { color: var(--accent-assistant); }
.msg-system  .msg-role { color: var(--accent-system); }
.msg-tool    .msg-role { color: var(--accent-tool); }
.msg-ts { font-size: .75rem; color: var(--ts-fg); margin-left: .5rem; }
.tool-name { font-size: .75rem; background: var(--code-bg); color: var(--accent-tool); border-radius: 4px; padding: .1em .4em; margin-left: .4rem; }
.msg-content { white-space: pre-wrap; word-break: break-word; color: var(--fg); }
pre { background: var(--code-bg); border-radius: 6px; padding: .8rem 1rem; overflow-x: auto; margin: .5rem 0; }
code { font-family: "JetBrains Mono", "Fira Code", monospace; font-size: .875rem; }
details summary { cursor: pointer; list-style: none; padding: .2rem 0; }
details summary::-webkit-details-marker { display: none; }"#;

const LIGHT_CSS: &str = r#":root {
  --bg: #f9f9fb;
  --bg-card: #ffffff;
  --fg: #24292f;
  --fg-muted: #57606a;
  --accent-user: #0969da;
  --accent-assistant: #1a7f37;
  --accent-system: #9a6700;
  --accent-tool: #8250df;
  --border: #d0d7de;
  --code-bg: #f6f8fa;
  --ts-fg: #8c959f;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body { background: var(--bg); color: var(--fg); font-family: system-ui, sans-serif; line-height: 1.6; }
.session-container { max-width: 860px; margin: 0 auto; padding: 2rem 1rem; }
.session-title { font-size: 1.5rem; margin-bottom: 1.5rem; color: var(--fg); border-bottom: 1px solid var(--border); padding-bottom: .5rem; }
.msg-user, .msg-assistant, .msg-system, .msg-tool { background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 1rem; margin-bottom: 1rem; }
.msg-user   { border-left: 3px solid var(--accent-user); }
.msg-assistant { border-left: 3px solid var(--accent-assistant); }
.msg-system { border-left: 3px solid var(--accent-system); }
.msg-tool   { border-left: 3px solid var(--accent-tool); }
.msg-header { margin-bottom: .5rem; }
.msg-role { font-weight: 700; font-size: .8rem; letter-spacing: .08em; text-transform: uppercase; }
.msg-user    .msg-role { color: var(--accent-user); }
.msg-assistant .msg-role { color: var(--accent-assistant); }
.msg-system  .msg-role { color: var(--accent-system); }
.msg-tool    .msg-role { color: var(--accent-tool); }
.msg-ts { font-size: .75rem; color: var(--ts-fg); margin-left: .5rem; }
.tool-name { font-size: .75rem; background: var(--code-bg); color: var(--accent-tool); border-radius: 4px; padding: .1em .4em; margin-left: .4rem; }
.msg-content { white-space: pre-wrap; word-break: break-word; color: var(--fg); }
pre { background: var(--code-bg); border-radius: 6px; padding: .8rem 1rem; overflow-x: auto; margin: .5rem 0; }
code { font-family: "JetBrains Mono", "Fira Code", monospace; font-size: .875rem; }
details summary { cursor: pointer; list-style: none; padding: .2rem 0; }
details summary::-webkit-details-marker { display: none; }"#;

// ---------------------------------------------------------------------------
// Gist sharing
// ---------------------------------------------------------------------------

/// Options for GitHub Gist upload.
#[derive(Debug, Clone)]
pub struct GistOptions {
    /// Human-readable description shown on the Gist page.
    pub description: String,
    /// Whether the gist is publicly listed (default `false` — private).
    pub public: bool,
    /// GitHub personal access token with `gist` scope.
    pub github_token: Option<String>,
}

impl Default for GistOptions {
    fn default() -> Self {
        Self {
            description: "VibeCody session share".to_string(),
            public: false, // private by default for session shares
            github_token: None,
        }
    }
}

/// A successfully created GitHub Gist.
#[derive(Debug, Clone)]
pub struct GistResult {
    pub gist_id: String,
    /// `https://gist.github.com/<user>/<id>`
    pub html_url: String,
    /// Raw content URL for the first file.
    pub raw_url: String,
    pub description: String,
}

/// Errors that can occur during Gist upload.
#[derive(Debug, Clone)]
pub enum GistError {
    /// No GitHub token was provided and the API rejected the request.
    AuthRequired,
    /// A network-level error (e.g. DNS failure, TLS error).
    NetworkError(String),
    /// The GitHub API returned an HTTP error status.
    ApiError { status: u16, message: String },
    /// The response body could not be parsed as expected JSON.
    InvalidResponse(String),
}

impl std::fmt::Display for GistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GistError::AuthRequired => write!(
                f,
                "GitHub token required — set GITHUB_TOKEN or provide via GistOptions"
            ),
            GistError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            GistError::ApiError { status, message } => {
                write!(f, "GitHub API error {status}: {message}")
            }
            GistError::InvalidResponse(msg) => write!(f, "Invalid API response: {msg}"),
        }
    }
}

/// Client for creating GitHub Gists via the REST API.
pub struct GistClient {
    token: Option<String>,
    /// Override for testing — defaults to `"https://api.github.com"`.
    base_url: String,
}

impl GistClient {
    /// Create a client pointing at `https://api.github.com`.
    pub fn new(token: Option<String>) -> Self {
        Self {
            token,
            base_url: "https://api.github.com".to_string(),
        }
    }

    /// Create a client with a custom base URL (useful for test servers).
    pub fn with_base_url(token: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            token,
            base_url: base_url.into(),
        }
    }

    /// Build the JSON request body for gist creation **without** making any
    /// network call. `filename` should end in `.html`.
    pub fn build_payload(filename: &str, content: &str, opts: &GistOptions) -> String {
        let escaped_desc = json_escape_str(&opts.description);
        let escaped_content = json_escape_str(content);
        let escaped_filename = json_escape_str(filename);
        let public_val = if opts.public { "true" } else { "false" };

        format!(
            r#"{{"description":"{desc}","public":{public},"files":{{"{file}":{{"content":"{content}"}}}}}}"#,
            desc = escaped_desc,
            public = public_val,
            file = escaped_filename,
            content = escaped_content,
        )
    }

    /// Parse a GitHub API gist-creation response body into a `GistResult`.
    ///
    /// Expects at minimum: `id`, `html_url`, `description`, and a `files`
    /// object with at least one entry that has a `raw_url` field.
    pub fn parse_response(json: &str) -> Result<GistResult, GistError> {
        // Minimal hand-rolled parser — no serde dependency needed here.
        let gist_id = extract_json_string(json, "\"id\"")
            .ok_or_else(|| GistError::InvalidResponse("missing 'id' field".to_string()))?;

        let html_url = extract_json_string(json, "\"html_url\"")
            .ok_or_else(|| GistError::InvalidResponse("missing 'html_url' field".to_string()))?;

        let description = extract_json_string(json, "\"description\"").unwrap_or_default();

        // raw_url lives nested inside files.<filename>.raw_url
        let raw_url =
            extract_json_string(json, "\"raw_url\"").unwrap_or_else(|| format!("{html_url}/raw"));

        Ok(GistResult {
            gist_id,
            html_url,
            raw_url,
            description,
        })
    }

    /// Upload `html_content` as a private Gist and return the result.
    ///
    /// Requires a GitHub token with the `gist` OAuth scope either in
    /// `opts.github_token` or on `self.token`.
    pub async fn upload(
        &self,
        html_content: &str,
        session_title: &str,
        opts: &GistOptions,
    ) -> Result<GistResult, GistError> {
        let token = opts
            .github_token
            .as_deref()
            .or(self.token.as_deref())
            .ok_or(GistError::AuthRequired)?;

        let safe_title: String = session_title
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let filename = format!("session-{}.html", safe_title);
        let payload = Self::build_payload(&filename, html_content, opts);
        let url = format!("{}/gists", self.base_url);

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", format!("token {token}"))
            .header("Accept", "application/vnd.github.v3+json")
            .header("Content-Type", "application/json")
            .header("User-Agent", "vibecody/session-share")
            .body(payload)
            .send()
            .await
            .map_err(|e| GistError::NetworkError(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| GistError::NetworkError(e.to_string()))?;

        if status == 401 || status == 403 {
            return Err(GistError::AuthRequired);
        }
        if status < 200 || status >= 300 {
            let msg = extract_json_string(&body, "\"message\"")
                .unwrap_or_else(|| body.chars().take(200).collect());
            return Err(GistError::ApiError {
                status,
                message: msg,
            });
        }

        Self::parse_response(&body)
    }
}

// ---------------------------------------------------------------------------
// JSON helpers (no external deps)
// ---------------------------------------------------------------------------

/// Escape a string value for embedding inside a JSON string literal.
fn json_escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// Very minimal JSON string extractor: finds `key: "value"` patterns.
///
/// Scans for `key` (e.g. `"id"`), then finds the next `"…"` value.
/// Does not handle escaped quotes inside values (sufficient for Gist IDs
/// and URLs which never contain them).
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let key_pos = json.find(key)?;
    // Skip past the key and the colon+whitespace
    let after_key = &json[key_pos + key.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let inner = &after_colon[1..];
    // Find closing quote, respecting \\ escape sequences
    let mut result = String::new();
    let mut chars = inner.chars();
    loop {
        match chars.next()? {
            '"' => break,
            '\\' => match chars.next()? {
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                c => {
                    result.push('\\');
                    result.push(c);
                }
            },
            c => result.push(c),
        }
    }
    Some(result)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- HtmlExportOptions default ---

    #[test]
    fn test_default_options() {
        let opts = HtmlExportOptions::default();
        assert_eq!(opts.title, "VibeCody Session");
        assert!(opts.dark_theme);
        assert!(opts.include_timestamps);
        assert!(opts.highlight_code_blocks);
        assert!(opts.include_tool_details);
        assert!(opts.max_content_length.is_none());
    }

    // --- escape_html ---

    #[test]
    fn test_escape_html_special_chars() {
        assert_eq!(HtmlExporter::escape_html("&"), "&amp;");
        assert_eq!(HtmlExporter::escape_html("<"), "&lt;");
        assert_eq!(HtmlExporter::escape_html(">"), "&gt;");
        assert_eq!(HtmlExporter::escape_html("\""), "&quot;");
        assert_eq!(HtmlExporter::escape_html("'"), "&#39;");
    }

    #[test]
    fn test_escape_html_combined() {
        let input = r#"<script>alert("XSS & fun")</script>"#;
        let out = HtmlExporter::escape_html(input);
        assert!(out.contains("&lt;script&gt;"));
        assert!(out.contains("&amp;"));
        assert!(out.contains("&quot;"));
        assert!(!out.contains('<'));
    }

    #[test]
    fn test_escape_html_plain_passthrough() {
        assert_eq!(HtmlExporter::escape_html("Hello world"), "Hello world");
    }

    // --- highlight_fences ---

    #[test]
    fn test_highlight_fences_rust_block() {
        let input = "Look at this:\n```rust\nfn main() {}\n```\nDone.";
        let out = HtmlExporter::highlight_fences(input);
        assert!(out.contains(r#"class="language-rust""#));
        assert!(out.contains("<pre><code"));
        // Curly braces are not HTML-special; they appear as-is in the output
        assert!(out.contains("fn main() {}"));
        assert!(out.contains("Done."));
    }

    #[test]
    fn test_highlight_fences_no_lang_defaults_to_text() {
        let input = "```\nsome code\n```";
        let out = HtmlExporter::highlight_fences(input);
        assert!(out.contains(r#"class="language-text""#));
    }

    #[test]
    fn test_highlight_fences_no_blocks_unchanged() {
        let input = "Just plain text\nno fences here";
        let out = HtmlExporter::highlight_fences(input);
        // No pre/code tags added
        assert!(!out.contains("<pre>"));
        assert!(out.contains("Just plain text"));
    }

    #[test]
    fn test_highlight_fences_multiple_blocks() {
        let input = "```python\nprint('hi')\n```\nsome text\n```js\nconsole.log(1)\n```";
        let out = HtmlExporter::highlight_fences(input);
        assert!(out.contains(r#"class="language-python""#));
        assert!(out.contains(r#"class="language-js""#));
    }

    // --- HtmlExporter::export ---

    #[test]
    fn test_export_produces_valid_html_structure() {
        let msgs = vec![
            ShareMessage::user("Hello"),
            ShareMessage::assistant("Hi there!"),
        ];
        let opts = HtmlExportOptions::default();
        let html = HtmlExporter::export(&msgs, &opts);

        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<title>VibeCody Session</title>"));
        assert!(html.contains("session-container"));
    }

    #[test]
    fn test_export_correct_css_classes() {
        let msgs = vec![
            ShareMessage::user("Question"),
            ShareMessage::assistant("Answer"),
            ShareMessage::tool("read_file", "contents"),
        ];
        let opts = HtmlExportOptions::default();
        let html = HtmlExporter::export(&msgs, &opts);

        assert!(html.contains("msg-user"));
        assert!(html.contains("msg-assistant"));
        assert!(html.contains("msg-tool"));
    }

    #[test]
    fn test_export_dark_theme_css_vars() {
        let msgs = vec![ShareMessage::user("hi")];
        let opts = HtmlExportOptions {
            dark_theme: true,
            ..Default::default()
        };
        let html = HtmlExporter::export(&msgs, &opts);
        assert!(html.contains("--bg: #1e1e2e"));
    }

    #[test]
    fn test_export_light_theme_css_vars() {
        let msgs = vec![ShareMessage::user("hi")];
        let opts = HtmlExportOptions {
            dark_theme: false,
            ..Default::default()
        };
        let html = HtmlExporter::export(&msgs, &opts);
        assert!(html.contains("--bg: #f9f9fb"));
    }

    #[test]
    fn test_export_timestamps_included() {
        let msg = ShareMessage::user("hi").with_timestamp(1_700_000_000_000);
        let opts = HtmlExportOptions {
            include_timestamps: true,
            ..Default::default()
        };
        let html = HtmlExporter::export(&[msg], &opts);
        assert!(html.contains("1700000000000"));
    }

    #[test]
    fn test_export_timestamps_excluded() {
        let msg = ShareMessage::user("hi").with_timestamp(9_999_999);
        let opts = HtmlExportOptions {
            include_timestamps: false,
            ..Default::default()
        };
        let html = HtmlExporter::export(&[msg], &opts);
        assert!(!html.contains("9999999"));
    }

    #[test]
    fn test_export_truncates_long_content() {
        let long_msg = ShareMessage::user("a".repeat(500));
        let opts = HtmlExportOptions {
            max_content_length: Some(10),
            highlight_code_blocks: false,
            ..Default::default()
        };
        let html = HtmlExporter::export(&[long_msg], &opts);
        assert!(html.contains("[truncated]"));
    }

    #[test]
    fn test_export_tool_message_uses_details_element() {
        let msg = ShareMessage::tool("bash", "$ ls -la");
        let opts = HtmlExportOptions::default();
        let html = HtmlExporter::export(&[msg], &opts);
        assert!(html.contains("<details>"));
        assert!(html.contains("<summary>"));
    }

    #[test]
    fn test_export_title_is_html_escaped() {
        let msgs: Vec<ShareMessage> = vec![];
        let opts = HtmlExportOptions {
            title: "Alert: <script>".to_string(),
            ..Default::default()
        };
        let html = HtmlExporter::export(&msgs, &opts);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    // --- ShareRole ---

    #[test]
    fn test_share_role_as_str() {
        assert_eq!(ShareRole::User.as_str(), "user");
        assert_eq!(ShareRole::Assistant.as_str(), "assistant");
        assert_eq!(ShareRole::System.as_str(), "system");
        assert_eq!(ShareRole::Tool.as_str(), "tool");
    }

    #[test]
    fn test_share_role_css_class() {
        assert_eq!(ShareRole::User.css_class(), "msg-user");
        assert_eq!(ShareRole::Assistant.css_class(), "msg-assistant");
        assert_eq!(ShareRole::System.css_class(), "msg-system");
        assert_eq!(ShareRole::Tool.css_class(), "msg-tool");
    }

    // --- GistClient::build_payload ---

    #[test]
    fn test_build_payload_json_structure() {
        let opts = GistOptions {
            description: "My session".to_string(),
            public: false,
            github_token: None,
        };
        let payload = GistClient::build_payload("session-test.html", "<html/>", &opts);

        assert!(payload.contains(r#""description":"My session""#));
        assert!(payload.contains(r#""public":false"#));
        assert!(payload.contains(r#""session-test.html""#));
        assert!(payload.contains(r#""content""#));
        assert!(payload.contains("&lt;html/&gt;") == false); // content is JSON-escaped, not HTML-escaped
    }

    #[test]
    fn test_build_payload_public_true() {
        let opts = GistOptions {
            public: true,
            ..Default::default()
        };
        let payload = GistClient::build_payload("f.html", "x", &opts);
        assert!(payload.contains(r#""public":true"#));
    }

    #[test]
    fn test_build_payload_escapes_quotes_in_content() {
        let opts = GistOptions::default();
        let content = r#"say "hello""#;
        let payload = GistClient::build_payload("f.html", content, &opts);
        // JSON-escaped double quotes
        assert!(payload.contains(r#"say \"hello\""#));
    }

    #[test]
    fn test_build_payload_escapes_newlines_in_content() {
        let opts = GistOptions::default();
        let content = "line1\nline2";
        let payload = GistClient::build_payload("f.html", content, &opts);
        assert!(payload.contains(r#"line1\nline2"#));
    }

    // --- GistClient::parse_response ---

    #[test]
    fn test_parse_response_valid_json() {
        let json = r#"{
            "id": "abc123",
            "html_url": "https://gist.github.com/user/abc123",
            "description": "VibeCody session share",
            "files": {
                "session-test.html": {
                    "raw_url": "https://gist.githubusercontent.com/user/abc123/raw/session-test.html"
                }
            }
        }"#;

        let result = GistClient::parse_response(json).expect("parse should succeed");
        assert_eq!(result.gist_id, "abc123");
        assert_eq!(result.html_url, "https://gist.github.com/user/abc123");
        assert_eq!(result.description, "VibeCody session share");
        assert!(!result.raw_url.is_empty());
    }

    #[test]
    fn test_parse_response_missing_id_returns_error() {
        let json = r#"{"html_url": "https://gist.github.com/x"}"#;
        let err = GistClient::parse_response(json).unwrap_err();
        assert!(matches!(err, GistError::InvalidResponse(_)));
    }

    #[test]
    fn test_parse_response_missing_html_url_returns_error() {
        let json = r#"{"id": "xyz"}"#;
        let err = GistClient::parse_response(json).unwrap_err();
        assert!(matches!(err, GistError::InvalidResponse(_)));
    }

    // --- GistOptions default ---

    #[test]
    fn test_gist_options_default() {
        let opts = GistOptions::default();
        assert!(!opts.public, "sessions should be private by default");
        assert!(opts.github_token.is_none());
        assert!(!opts.description.is_empty());
    }

    // --- GistError display ---

    #[test]
    fn test_gist_error_display_auth_required() {
        let msg = format!("{}", GistError::AuthRequired);
        assert!(msg.contains("token"));
    }

    #[test]
    fn test_gist_error_display_api_error() {
        let err = GistError::ApiError {
            status: 422,
            message: "validation failed".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("422"));
        assert!(msg.contains("validation failed"));
    }

    // --- json helpers ---

    #[test]
    fn test_json_escape_str_newline_and_quote() {
        let s = "hello\nworld\"quote";
        let escaped = json_escape_str(s);
        assert!(escaped.contains("\\n"));
        assert!(escaped.contains("\\\""));
    }

    #[test]
    fn test_extract_json_string_basic() {
        let json = r#"{"id":"abc123","other":"val"}"#;
        assert_eq!(
            extract_json_string(json, "\"id\""),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_extract_json_string_missing_key() {
        let json = r#"{"other":"val"}"#;
        assert_eq!(extract_json_string(json, "\"id\""), None);
    }
}
