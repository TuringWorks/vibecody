#![allow(dead_code)]
//! Agent session sharing for VibeCody.
//!
//! Enables sharing agent sessions with team members, closing the gap
//! vs Amp (thread sharing) and Lovable (real-time collab). Sessions
//! can be exported as JSON, Markdown, or HTML, with automatic secret
//! redaction and annotation support.
//!
//! REPL commands: `/session-share share|list|export|annotate|import|delete`

use std::collections::HashMap;

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Private,
    Team,
    Public,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Private => write!(f, "private"),
            Self::Team => write!(f, "team"),
            Self::Public => write!(f, "public"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Renamed => write!(f, "renamed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationTarget {
    ToolCall(String),
    FileChange(String),
    ReasoningStep(u32),
    General,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionOutcome {
    Success,
    Partial,
    Failed,
    InProgress,
}

impl std::fmt::Display for SessionOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Partial => write!(f, "partial"),
            Self::Failed => write!(f, "failed"),
            Self::InProgress => write!(f, "in-progress"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    Json,
    Markdown,
    Html,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SharingError {
    SessionNotFound,
    ExportTooLarge,
    RedactionFailed,
    AnnotationNotFound,
    DuplicateSession,
    InvalidFilter,
    FormatError,
}

impl std::fmt::Display for SharingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionNotFound => write!(f, "session not found"),
            Self::ExportTooLarge => write!(f, "export exceeds maximum size"),
            Self::RedactionFailed => write!(f, "secret redaction failed"),
            Self::AnnotationNotFound => write!(f, "annotation not found"),
            Self::DuplicateSession => write!(f, "duplicate session id"),
            Self::InvalidFilter => write!(f, "invalid filter parameters"),
            Self::FormatError => write!(f, "export format error"),
        }
    }
}

// === Data Structures ===

#[derive(Debug, Clone)]
pub struct SharingConfig {
    pub export_dir: String,
    pub max_export_size_mb: usize,
    pub redact_secrets: bool,
    pub default_visibility: Visibility,
    pub include_file_contents: bool,
    pub include_reasoning: bool,
}

impl Default for SharingConfig {
    fn default() -> Self {
        Self {
            export_dir: ".vibecody/shared-sessions".to_string(),
            max_export_size_mb: 50,
            redact_secrets: true,
            default_visibility: Visibility::Private,
            include_file_contents: false,
            include_reasoning: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallRecord {
    pub id: String,
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileChange {
    pub file_path: String,
    pub change_type: ChangeType,
    pub additions: usize,
    pub deletions: usize,
    pub diff_summary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReasoningStep {
    pub step_number: u32,
    pub description: String,
    pub decision: String,
    pub alternatives_considered: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub id: String,
    pub author: String,
    pub content: String,
    pub target_type: AnnotationTarget,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionMetadata {
    pub model_used: String,
    pub provider: String,
    pub total_tokens: u64,
    pub total_tool_calls: usize,
    pub files_modified: usize,
    pub task_description: String,
    pub outcome: SessionOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SharedSession {
    pub id: String,
    pub title: String,
    pub description: String,
    pub agent_id: String,
    pub created_by: String,
    pub created_at: u64,
    pub visibility: Visibility,
    pub tool_calls: Vec<ToolCallRecord>,
    pub file_changes: Vec<FileChange>,
    pub reasoning_steps: Vec<ReasoningStep>,
    pub annotations: Vec<Annotation>,
    pub metadata: SessionMetadata,
    pub share_url: Option<String>,
    pub duration_secs: u64,
}

#[derive(Debug, Clone)]
pub struct SessionFilter {
    pub author: Option<String>,
    pub visibility: Option<Visibility>,
    pub outcome: Option<SessionOutcome>,
    pub after: Option<u64>,
    pub before: Option<u64>,
    pub keyword: Option<String>,
}

impl Default for SessionFilter {
    fn default() -> Self {
        Self {
            author: None,
            visibility: None,
            outcome: None,
            after: None,
            before: None,
            keyword: None,
        }
    }
}

// === Manager ===

pub struct SessionSharingManager {
    config: SharingConfig,
    sessions: HashMap<String, SharedSession>,
}

impl SessionSharingManager {
    pub fn new(config: SharingConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
        }
    }

    pub fn share_session(&mut self, session: SharedSession) -> Result<String, SharingError> {
        if self.sessions.contains_key(&session.id) {
            return Err(SharingError::DuplicateSession);
        }
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        Ok(id)
    }

    pub fn get_session(&self, id: &str) -> Option<&SharedSession> {
        self.sessions.get(id)
    }

    pub fn list_sessions(&self) -> Vec<&SharedSession> {
        let mut sessions: Vec<&SharedSession> = self.sessions.values().collect();
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        sessions
    }

    pub fn filter_sessions(&self, filter: &SessionFilter) -> Vec<&SharedSession> {
        self.sessions
            .values()
            .filter(|s| {
                if let Some(ref author) = filter.author {
                    if &s.created_by != author {
                        return false;
                    }
                }
                if let Some(ref vis) = filter.visibility {
                    if &s.visibility != vis {
                        return false;
                    }
                }
                if let Some(ref outcome) = filter.outcome {
                    if &s.metadata.outcome != outcome {
                        return false;
                    }
                }
                if let Some(after) = filter.after {
                    if s.created_at < after {
                        return false;
                    }
                }
                if let Some(before) = filter.before {
                    if s.created_at > before {
                        return false;
                    }
                }
                if let Some(ref keyword) = filter.keyword {
                    let kw = keyword.to_lowercase();
                    let matches = s.title.to_lowercase().contains(&kw)
                        || s.description.to_lowercase().contains(&kw)
                        || s.metadata.task_description.to_lowercase().contains(&kw);
                    if !matches {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    pub fn add_annotation(
        &mut self,
        session_id: &str,
        annotation: Annotation,
    ) -> Result<(), SharingError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or(SharingError::SessionNotFound)?;
        session.annotations.push(annotation);
        Ok(())
    }

    pub fn get_annotations(&self, session_id: &str) -> Result<Vec<&Annotation>, SharingError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or(SharingError::SessionNotFound)?;
        Ok(session.annotations.iter().collect())
    }

    pub fn export_session(
        &self,
        id: &str,
        format: ExportFormat,
    ) -> Result<String, SharingError> {
        let session = self
            .sessions
            .get(id)
            .ok_or(SharingError::SessionNotFound)?;

        let size = self.estimate_export_size(session);
        let max_bytes = self.config.max_export_size_mb * 1024 * 1024;
        if size > max_bytes {
            return Err(SharingError::ExportTooLarge);
        }

        let mut output = match format {
            ExportFormat::Json => self.export_to_json(session),
            ExportFormat::Markdown => self.export_to_markdown(session),
            ExportFormat::Html => self.export_to_html(session),
        };

        if self.config.redact_secrets {
            output = self.redact_secrets(&output);
        }

        Ok(output)
    }

    pub fn export_to_json(&self, session: &SharedSession) -> String {
        let tool_calls_json: Vec<String> = session
            .tool_calls
            .iter()
            .map(|tc| {
                format!(
                    r#"    {{"id":"{}","tool_name":"{}","input_summary":"{}","output_summary":"{}","timestamp":{},"duration_ms":{},"success":{}}}"#,
                    tc.id, tc.tool_name, tc.input_summary, tc.output_summary, tc.timestamp, tc.duration_ms, tc.success
                )
            })
            .collect();

        let file_changes_json: Vec<String> = session
            .file_changes
            .iter()
            .map(|fc| {
                format!(
                    r#"    {{"file_path":"{}","change_type":"{}","additions":{},"deletions":{},"diff_summary":"{}"}}"#,
                    fc.file_path, fc.change_type, fc.additions, fc.deletions, fc.diff_summary
                )
            })
            .collect();

        let reasoning_json: Vec<String> = session
            .reasoning_steps
            .iter()
            .map(|rs| {
                let alts: Vec<String> = rs.alternatives_considered.iter().map(|a| format!("\"{}\"", a)).collect();
                format!(
                    r#"    {{"step_number":{},"description":"{}","decision":"{}","alternatives_considered":[{}],"timestamp":{}}}"#,
                    rs.step_number, rs.description, rs.decision, alts.join(","), rs.timestamp
                )
            })
            .collect();

        let annotations_json: Vec<String> = session
            .annotations
            .iter()
            .map(|a| {
                let target = match &a.target_type {
                    AnnotationTarget::ToolCall(id) => format!("{{\"type\":\"tool_call\",\"id\":\"{}\"}}", id),
                    AnnotationTarget::FileChange(path) => format!("{{\"type\":\"file_change\",\"path\":\"{}\"}}", path),
                    AnnotationTarget::ReasoningStep(n) => format!("{{\"type\":\"reasoning_step\",\"step\":{}}}", n),
                    AnnotationTarget::General => "{\"type\":\"general\"}".to_string(),
                };
                format!(
                    r#"    {{"id":"{}","author":"{}","content":"{}","target":{},"timestamp":{}}}"#,
                    a.id, a.author, a.content, target, a.timestamp
                )
            })
            .collect();

        let share_url = match &session.share_url {
            Some(url) => format!("\"{}\"", url),
            None => "null".to_string(),
        };

        format!(
            r#"{{"id":"{}","title":"{}","description":"{}","agent_id":"{}","created_by":"{}","created_at":{},"visibility":"{}","tool_calls":[
{}
],"file_changes":[
{}
],"reasoning_steps":[
{}
],"annotations":[
{}
],"metadata":{{"model_used":"{}","provider":"{}","total_tokens":{},"total_tool_calls":{},"files_modified":{},"task_description":"{}","outcome":"{}"}},"share_url":{},"duration_secs":{}}}"#,
            session.id,
            session.title,
            session.description,
            session.agent_id,
            session.created_by,
            session.created_at,
            session.visibility,
            tool_calls_json.join(",\n"),
            file_changes_json.join(",\n"),
            reasoning_json.join(",\n"),
            annotations_json.join(",\n"),
            session.metadata.model_used,
            session.metadata.provider,
            session.metadata.total_tokens,
            session.metadata.total_tool_calls,
            session.metadata.files_modified,
            session.metadata.task_description,
            session.metadata.outcome,
            share_url,
            session.duration_secs,
        )
    }

    pub fn export_to_markdown(&self, session: &SharedSession) -> String {
        let mut md = String::with_capacity(4096);
        md.push_str(&format!("# {}\n\n", session.title));
        md.push_str(&format!("**Description:** {}\n\n", session.description));
        md.push_str(&format!(
            "**Agent:** {} | **Author:** {} | **Visibility:** {}\n\n",
            session.agent_id, session.created_by, session.visibility
        ));
        md.push_str(&format!(
            "**Model:** {} ({}) | **Tokens:** {} | **Duration:** {}s\n\n",
            session.metadata.model_used,
            session.metadata.provider,
            session.metadata.total_tokens,
            session.duration_secs
        ));
        md.push_str(&format!(
            "**Outcome:** {} | **Task:** {}\n\n",
            session.metadata.outcome, session.metadata.task_description
        ));

        if !session.tool_calls.is_empty() {
            md.push_str("## Tool Calls\n\n");
            md.push_str("| # | Tool | Input | Output | Duration | Status |\n");
            md.push_str("|---|------|-------|--------|----------|--------|\n");
            for (i, tc) in session.tool_calls.iter().enumerate() {
                let status = if tc.success { "OK" } else { "FAIL" };
                md.push_str(&format!(
                    "| {} | {} | {} | {} | {}ms | {} |\n",
                    i + 1,
                    tc.tool_name,
                    tc.input_summary,
                    tc.output_summary,
                    tc.duration_ms,
                    status
                ));
            }
            md.push('\n');
        }

        if !session.file_changes.is_empty() {
            md.push_str("## File Changes\n\n");
            for fc in &session.file_changes {
                md.push_str(&format!(
                    "- **{}** `{}` (+{}/-{}): {}\n",
                    fc.change_type, fc.file_path, fc.additions, fc.deletions, fc.diff_summary
                ));
            }
            md.push('\n');
        }

        if !session.reasoning_steps.is_empty() {
            md.push_str("## Reasoning Steps\n\n");
            for rs in &session.reasoning_steps {
                md.push_str(&format!(
                    "{}. **{}** — {}\n",
                    rs.step_number, rs.description, rs.decision
                ));
                if !rs.alternatives_considered.is_empty() {
                    md.push_str(&format!(
                        "   - Alternatives: {}\n",
                        rs.alternatives_considered.join(", ")
                    ));
                }
            }
            md.push('\n');
        }

        if !session.annotations.is_empty() {
            md.push_str("## Annotations\n\n");
            for a in &session.annotations {
                md.push_str(&format!("- **{}**: {}\n", a.author, a.content));
            }
            md.push('\n');
        }

        md
    }

    pub fn export_to_html(&self, session: &SharedSession) -> String {
        let mut html = String::with_capacity(8192);
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!("<title>{}</title>\n", session.title));
        html.push_str("<style>body{font-family:sans-serif;max-width:900px;margin:0 auto;padding:20px}");
        html.push_str("table{border-collapse:collapse;width:100%}th,td{border:1px solid #ddd;padding:8px;text-align:left}");
        html.push_str("th{background:#f4f4f4}.success{color:green}.fail{color:red}</style>\n");
        html.push_str("</head>\n<body>\n");
        html.push_str(&format!("<h1>{}</h1>\n", session.title));
        html.push_str(&format!("<p>{}</p>\n", session.description));
        html.push_str(&format!(
            "<p><strong>Agent:</strong> {} | <strong>Author:</strong> {} | <strong>Visibility:</strong> {}</p>\n",
            session.agent_id, session.created_by, session.visibility
        ));
        html.push_str(&format!(
            "<p><strong>Model:</strong> {} ({}) | <strong>Outcome:</strong> {}</p>\n",
            session.metadata.model_used, session.metadata.provider, session.metadata.outcome
        ));

        if !session.tool_calls.is_empty() {
            html.push_str("<h2>Tool Calls</h2>\n<table>\n");
            html.push_str("<tr><th>#</th><th>Tool</th><th>Input</th><th>Output</th><th>Duration</th><th>Status</th></tr>\n");
            for (i, tc) in session.tool_calls.iter().enumerate() {
                let class = if tc.success { "success" } else { "fail" };
                let status = if tc.success { "OK" } else { "FAIL" };
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}ms</td><td class=\"{}\">{}</td></tr>\n",
                    i + 1, tc.tool_name, tc.input_summary, tc.output_summary, tc.duration_ms, class, status
                ));
            }
            html.push_str("</table>\n");
        }

        if !session.file_changes.is_empty() {
            html.push_str("<h2>File Changes</h2>\n<ul>\n");
            for fc in &session.file_changes {
                html.push_str(&format!(
                    "<li><strong>{}</strong> <code>{}</code> (+{}/-{}): {}</li>\n",
                    fc.change_type, fc.file_path, fc.additions, fc.deletions, fc.diff_summary
                ));
            }
            html.push_str("</ul>\n");
        }

        if !session.reasoning_steps.is_empty() {
            html.push_str("<h2>Reasoning Steps</h2>\n<ol>\n");
            for rs in &session.reasoning_steps {
                html.push_str(&format!(
                    "<li><strong>{}</strong> — {}</li>\n",
                    rs.description, rs.decision
                ));
            }
            html.push_str("</ol>\n");
        }

        if !session.annotations.is_empty() {
            html.push_str("<h2>Annotations</h2>\n<ul>\n");
            for a in &session.annotations {
                html.push_str(&format!(
                    "<li><strong>{}</strong>: {}</li>\n",
                    a.author, a.content
                ));
            }
            html.push_str("</ul>\n");
        }

        html.push_str("</body>\n</html>");
        html
    }

    pub fn redact_secrets(&self, text: &str) -> String {
        let mut result = text.to_string();
        let redacted = "[REDACTED]";

        // API key patterns: sk-..., key-..., AKIA..., ghp_..., gho_..., glpat-...
        let api_key_prefixes = [
            "sk-", "sk_live_", "sk_test_", "key-", "AKIA", "ghp_", "gho_", "glpat-",
            "xoxb-", "xoxp-", "whsec_",
        ];
        for prefix in &api_key_prefixes {
            let mut offset = 0;
            while offset < result.len() {
                if let Some(pos) = result[offset..].find(prefix) {
                    let start = offset + pos;
                    let end = result[start..]
                        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
                        .map(|e| start + e)
                        .unwrap_or(result.len());
                    result.replace_range(start..end, redacted);
                    offset = start + redacted.len();
                } else {
                    break;
                }
            }
        }

        // Bearer tokens
        let bearer_prefix = "Bearer ";
        {
            let mut offset = 0;
            while offset < result.len() {
                if let Some(pos) = result[offset..].find(bearer_prefix) {
                    let start = offset + pos;
                    let token_start = start + bearer_prefix.len();
                    let end = result[token_start..]
                        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                        .map(|e| token_start + e)
                        .unwrap_or(result.len());
                    result.replace_range(token_start..end, redacted);
                    offset = token_start + redacted.len();
                } else {
                    break;
                }
            }
        }

        // password=... or password:...
        for trigger in &["password=", "password:", "PASSWORD=", "PASSWORD:"] {
            let mut offset = 0;
            while offset < result.len() {
                if let Some(pos) = result[offset..].find(trigger) {
                    let start = offset + pos;
                    let val_start = start + trigger.len();
                    if val_start >= result.len() {
                        break;
                    }
                    // skip optional quotes
                    let (val_start, quote_char) = if result[val_start..].starts_with('"') {
                        (val_start + 1, Some('"'))
                    } else if result[val_start..].starts_with('\'') {
                        (val_start + 1, Some('\''))
                    } else {
                        (val_start, None)
                    };
                    let end = if let Some(q) = quote_char {
                        result[val_start..]
                            .find(q)
                            .map(|e| val_start + e)
                            .unwrap_or(result.len())
                    } else {
                        result[val_start..]
                            .find(|c: char| c.is_whitespace() || c == ',' || c == '}' || c == '"')
                            .map(|e| val_start + e)
                            .unwrap_or(result.len())
                    };
                    result.replace_range(val_start..end, redacted);
                    offset = val_start + redacted.len();
                } else {
                    break;
                }
            }
        }

        // Environment variable assignments: SOME_SECRET=value
        for env_key in &[
            "API_KEY=", "SECRET_KEY=", "ACCESS_TOKEN=", "PRIVATE_KEY=",
            "ANTHROPIC_API_KEY=", "OPENAI_API_KEY=", "GEMINI_API_KEY=",
        ] {
            let mut offset = 0;
            while offset < result.len() {
                if let Some(pos) = result[offset..].find(env_key) {
                    let start = offset + pos;
                    let val_start = start + env_key.len();
                    let end = result[val_start..]
                        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                        .map(|e| val_start + e)
                        .unwrap_or(result.len());
                    result.replace_range(val_start..end, redacted);
                    offset = val_start + redacted.len();
                } else {
                    break;
                }
            }
        }

        result
    }

    pub fn estimate_export_size(&self, session: &SharedSession) -> usize {
        let mut size = 0usize;
        size += session.id.len() + session.title.len() + session.description.len();
        size += session.agent_id.len() + session.created_by.len();
        for tc in &session.tool_calls {
            size += tc.id.len() + tc.tool_name.len() + tc.input_summary.len() + tc.output_summary.len() + 64;
        }
        for fc in &session.file_changes {
            size += fc.file_path.len() + fc.diff_summary.len() + 64;
        }
        for rs in &session.reasoning_steps {
            size += rs.description.len() + rs.decision.len() + 64;
            for alt in &rs.alternatives_considered {
                size += alt.len();
            }
        }
        for a in &session.annotations {
            size += a.id.len() + a.author.len() + a.content.len() + 64;
        }
        size += session.metadata.model_used.len()
            + session.metadata.provider.len()
            + session.metadata.task_description.len()
            + 128;
        size
    }

    pub fn delete_session(&mut self, id: &str) -> Result<(), SharingError> {
        self.sessions
            .remove(id)
            .map(|_| ())
            .ok_or(SharingError::SessionNotFound)
    }

    pub fn update_visibility(
        &mut self,
        id: &str,
        visibility: Visibility,
    ) -> Result<(), SharingError> {
        let session = self
            .sessions
            .get_mut(id)
            .ok_or(SharingError::SessionNotFound)?;
        session.visibility = visibility;
        Ok(())
    }

    pub fn get_session_summary(&self, id: &str) -> Option<String> {
        self.sessions.get(id).map(|s| {
            format!(
                "[{}] {} by {} — {} ({}, {} tool calls, {} files)",
                s.visibility,
                s.title,
                s.created_by,
                s.metadata.outcome,
                s.metadata.provider,
                s.metadata.total_tool_calls,
                s.metadata.files_modified,
            )
        })
    }

    pub fn import_session(&mut self, json: &str) -> Result<SharedSession, SharingError> {
        // Minimal JSON parser — extract top-level string and numeric fields
        let get_str = |key: &str| -> Result<String, SharingError> {
            let pattern = format!("\"{}\":\"", key);
            let start = json.find(&pattern).ok_or(SharingError::FormatError)? + pattern.len();
            let end = json[start..].find('"').ok_or(SharingError::FormatError)? + start;
            Ok(json[start..end].to_string())
        };
        let get_u64 = |key: &str| -> Result<u64, SharingError> {
            let pattern = format!("\"{}\":", key);
            let start = json.find(&pattern).ok_or(SharingError::FormatError)? + pattern.len();
            let num_str: String = json[start..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            num_str.parse().map_err(|_| SharingError::FormatError)
        };

        let id = get_str("id")?;
        let title = get_str("title")?;
        let description = get_str("description")?;
        let agent_id = get_str("agent_id")?;
        let created_by = get_str("created_by")?;
        let created_at = get_u64("created_at")?;
        let visibility_str = get_str("visibility")?;
        let visibility = match visibility_str.as_str() {
            "team" => Visibility::Team,
            "public" => Visibility::Public,
            _ => Visibility::Private,
        };

        let model_used = get_str("model_used")?;
        let provider = get_str("provider")?;
        let total_tokens = get_u64("total_tokens")?;
        let total_tool_calls_val = get_u64("total_tool_calls")? as usize;
        let files_modified_val = get_u64("files_modified")? as usize;
        let task_description = get_str("task_description")?;
        let outcome_str = get_str("outcome")?;
        let outcome = match outcome_str.as_str() {
            "success" => SessionOutcome::Success,
            "partial" => SessionOutcome::Partial,
            "failed" => SessionOutcome::Failed,
            _ => SessionOutcome::InProgress,
        };
        let duration_secs = get_u64("duration_secs")?;

        let session = SharedSession {
            id,
            title,
            description,
            agent_id,
            created_by,
            created_at,
            visibility,
            tool_calls: Vec::new(),
            file_changes: Vec::new(),
            reasoning_steps: Vec::new(),
            annotations: Vec::new(),
            metadata: SessionMetadata {
                model_used,
                provider,
                total_tokens,
                total_tool_calls: total_tool_calls_val,
                files_modified: files_modified_val,
                task_description,
                outcome,
            },
            share_url: None,
            duration_secs,
        };

        Ok(session)
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> SharingConfig {
        SharingConfig::default()
    }

    fn make_metadata(outcome: SessionOutcome) -> SessionMetadata {
        SessionMetadata {
            model_used: "claude-opus-4-20250514".to_string(),
            provider: "anthropic".to_string(),
            total_tokens: 15000,
            total_tool_calls: 3,
            files_modified: 2,
            task_description: "Refactor auth module".to_string(),
            outcome,
        }
    }

    fn make_session(id: &str, author: &str, vis: Visibility, outcome: SessionOutcome) -> SharedSession {
        SharedSession {
            id: id.to_string(),
            title: format!("Session {}", id),
            description: "Test session description".to_string(),
            agent_id: "agent-1".to_string(),
            created_by: author.to_string(),
            created_at: 1700000000,
            visibility: vis,
            tool_calls: vec![
                ToolCallRecord {
                    id: "tc-1".to_string(),
                    tool_name: "read_file".to_string(),
                    input_summary: "src/main.rs".to_string(),
                    output_summary: "200 lines read".to_string(),
                    timestamp: 1700000010,
                    duration_ms: 50,
                    success: true,
                },
            ],
            file_changes: vec![
                FileChange {
                    file_path: "src/auth.rs".to_string(),
                    change_type: ChangeType::Modified,
                    additions: 15,
                    deletions: 3,
                    diff_summary: "Added token validation".to_string(),
                },
            ],
            reasoning_steps: vec![
                ReasoningStep {
                    step_number: 1,
                    description: "Analyze auth flow".to_string(),
                    decision: "Use JWT tokens".to_string(),
                    alternatives_considered: vec!["Session cookies".to_string(), "OAuth only".to_string()],
                    timestamp: 1700000005,
                },
            ],
            annotations: Vec::new(),
            metadata: make_metadata(outcome),
            share_url: None,
            duration_secs: 120,
        }
    }

    // --- Config defaults ---

    #[test]
    fn test_config_defaults() {
        let config = SharingConfig::default();
        assert_eq!(config.export_dir, ".vibecody/shared-sessions");
        assert_eq!(config.max_export_size_mb, 50);
        assert!(config.redact_secrets);
        assert_eq!(config.default_visibility, Visibility::Private);
        assert!(!config.include_file_contents);
        assert!(config.include_reasoning);
    }

    #[test]
    fn test_config_custom() {
        let config = SharingConfig {
            export_dir: "/tmp/sessions".to_string(),
            max_export_size_mb: 100,
            redact_secrets: false,
            default_visibility: Visibility::Public,
            include_file_contents: true,
            include_reasoning: false,
        };
        assert_eq!(config.max_export_size_mb, 100);
        assert!(!config.redact_secrets);
        assert_eq!(config.default_visibility, Visibility::Public);
    }

    // --- Share session ---

    #[test]
    fn test_share_session_success() {
        let mut mgr = SessionSharingManager::new(make_config());
        let session = make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success);
        let result = mgr.share_session(session);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "s-1");
    }

    #[test]
    fn test_share_session_duplicate() {
        let mut mgr = SessionSharingManager::new(make_config());
        let s1 = make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success);
        let s2 = make_session("s-1", "bob", Visibility::Private, SessionOutcome::Failed);
        mgr.share_session(s1).unwrap();
        let result = mgr.share_session(s2);
        assert_eq!(result, Err(SharingError::DuplicateSession));
    }

    // --- Get session ---

    #[test]
    fn test_get_session_found() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let session = mgr.get_session("s-1");
        assert!(session.is_some());
        assert_eq!(session.unwrap().created_by, "alice");
    }

    #[test]
    fn test_get_session_not_found() {
        let mgr = SessionSharingManager::new(make_config());
        assert!(mgr.get_session("nonexistent").is_none());
    }

    // --- List sessions ---

    #[test]
    fn test_list_sessions_empty() {
        let mgr = SessionSharingManager::new(make_config());
        assert!(mgr.list_sessions().is_empty());
    }

    #[test]
    fn test_list_sessions_multiple() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        mgr.share_session(make_session("s-2", "bob", Visibility::Public, SessionOutcome::Failed)).unwrap();
        assert_eq!(mgr.list_sessions().len(), 2);
    }

    // --- Filter sessions ---

    #[test]
    fn test_filter_by_author() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        mgr.share_session(make_session("s-2", "bob", Visibility::Team, SessionOutcome::Success)).unwrap();
        let filter = SessionFilter { author: Some("alice".to_string()), ..Default::default() };
        let results = mgr.filter_sessions(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].created_by, "alice");
    }

    #[test]
    fn test_filter_by_visibility() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        mgr.share_session(make_session("s-2", "bob", Visibility::Public, SessionOutcome::Success)).unwrap();
        let filter = SessionFilter { visibility: Some(Visibility::Public), ..Default::default() };
        let results = mgr.filter_sessions(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-2");
    }

    #[test]
    fn test_filter_by_outcome() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        mgr.share_session(make_session("s-2", "bob", Visibility::Team, SessionOutcome::Failed)).unwrap();
        let filter = SessionFilter { outcome: Some(SessionOutcome::Failed), ..Default::default() };
        let results = mgr.filter_sessions(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-2");
    }

    #[test]
    fn test_filter_by_keyword() {
        let mut mgr = SessionSharingManager::new(make_config());
        let mut s = make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success);
        s.title = "Fix authentication bug".to_string();
        mgr.share_session(s).unwrap();
        mgr.share_session(make_session("s-2", "bob", Visibility::Team, SessionOutcome::Success)).unwrap();
        let filter = SessionFilter { keyword: Some("authentication".to_string()), ..Default::default() };
        let results = mgr.filter_sessions(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-1");
    }

    #[test]
    fn test_filter_by_date_range() {
        let mut mgr = SessionSharingManager::new(make_config());
        let mut s1 = make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success);
        s1.created_at = 1000;
        let mut s2 = make_session("s-2", "bob", Visibility::Team, SessionOutcome::Success);
        s2.created_at = 2000;
        let mut s3 = make_session("s-3", "carol", Visibility::Team, SessionOutcome::Success);
        s3.created_at = 3000;
        mgr.share_session(s1).unwrap();
        mgr.share_session(s2).unwrap();
        mgr.share_session(s3).unwrap();
        let filter = SessionFilter { after: Some(1500), before: Some(2500), ..Default::default() };
        let results = mgr.filter_sessions(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-2");
    }

    #[test]
    fn test_filter_empty() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let filter = SessionFilter::default();
        let results = mgr.filter_sessions(&filter);
        assert_eq!(results.len(), 1);
    }

    // --- Annotations ---

    #[test]
    fn test_add_annotation() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let annotation = Annotation {
            id: "a-1".to_string(),
            author: "bob".to_string(),
            content: "Great approach!".to_string(),
            target_type: AnnotationTarget::General,
            timestamp: 1700001000,
        };
        assert!(mgr.add_annotation("s-1", annotation).is_ok());
    }

    #[test]
    fn test_add_annotation_session_not_found() {
        let mut mgr = SessionSharingManager::new(make_config());
        let annotation = Annotation {
            id: "a-1".to_string(),
            author: "bob".to_string(),
            content: "Comment".to_string(),
            target_type: AnnotationTarget::General,
            timestamp: 1700001000,
        };
        assert_eq!(mgr.add_annotation("nonexistent", annotation), Err(SharingError::SessionNotFound));
    }

    #[test]
    fn test_get_annotations() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let a1 = Annotation {
            id: "a-1".to_string(),
            author: "bob".to_string(),
            content: "Nice!".to_string(),
            target_type: AnnotationTarget::ToolCall("tc-1".to_string()),
            timestamp: 1700001000,
        };
        let a2 = Annotation {
            id: "a-2".to_string(),
            author: "carol".to_string(),
            content: "Why this approach?".to_string(),
            target_type: AnnotationTarget::ReasoningStep(1),
            timestamp: 1700002000,
        };
        mgr.add_annotation("s-1", a1).unwrap();
        mgr.add_annotation("s-1", a2).unwrap();
        let annotations = mgr.get_annotations("s-1").unwrap();
        assert_eq!(annotations.len(), 2);
        assert_eq!(annotations[0].author, "bob");
        assert_eq!(annotations[1].author, "carol");
    }

    #[test]
    fn test_get_annotations_not_found() {
        let mgr = SessionSharingManager::new(make_config());
        assert_eq!(mgr.get_annotations("nope"), Err(SharingError::SessionNotFound));
    }

    // --- Export JSON ---

    #[test]
    fn test_export_json() {
        let mut mgr = SessionSharingManager::new(SharingConfig { redact_secrets: false, ..make_config() });
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let json = mgr.export_session("s-1", ExportFormat::Json).unwrap();
        assert!(json.contains("\"id\":\"s-1\""));
        assert!(json.contains("\"title\":\"Session s-1\""));
        assert!(json.contains("\"created_by\":\"alice\""));
        assert!(json.contains("\"visibility\":\"team\""));
        assert!(json.contains("\"tool_name\":\"read_file\""));
        assert!(json.contains("\"outcome\":\"success\""));
    }

    // --- Export Markdown ---

    #[test]
    fn test_export_markdown() {
        let mut mgr = SessionSharingManager::new(SharingConfig { redact_secrets: false, ..make_config() });
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let md = mgr.export_session("s-1", ExportFormat::Markdown).unwrap();
        assert!(md.contains("# Session s-1"));
        assert!(md.contains("## Tool Calls"));
        assert!(md.contains("## File Changes"));
        assert!(md.contains("## Reasoning Steps"));
        assert!(md.contains("read_file"));
        assert!(md.contains("src/auth.rs"));
        assert!(md.contains("Analyze auth flow"));
    }

    // --- Export HTML ---

    #[test]
    fn test_export_html() {
        let mut mgr = SessionSharingManager::new(SharingConfig { redact_secrets: false, ..make_config() });
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let html = mgr.export_session("s-1", ExportFormat::Html).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<h1>Session s-1</h1>"));
        assert!(html.contains("<h2>Tool Calls</h2>"));
        assert!(html.contains("<h2>File Changes</h2>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn test_export_session_not_found() {
        let mgr = SessionSharingManager::new(make_config());
        assert_eq!(mgr.export_session("nope", ExportFormat::Json), Err(SharingError::SessionNotFound));
    }

    // --- Secret redaction ---

    #[test]
    fn test_redact_api_keys() {
        let mgr = SessionSharingManager::new(make_config());
        let text = "Using key sk-abc123xyz for auth";
        let redacted = mgr.redact_secrets(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("sk-abc123xyz"));
    }

    #[test]
    fn test_redact_bearer_tokens() {
        let mgr = SessionSharingManager::new(make_config());
        let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abc123";
        let redacted = mgr.redact_secrets(text);
        assert!(redacted.contains("Bearer [REDACTED]"));
        assert!(!redacted.contains("eyJhbGciOiJIUzI1NiJ9"));
    }

    #[test]
    fn test_redact_passwords() {
        let mgr = SessionSharingManager::new(make_config());
        let text = "config password=SuperSecret123 done";
        let redacted = mgr.redact_secrets(text);
        assert!(redacted.contains("password=[REDACTED]"));
        assert!(!redacted.contains("SuperSecret123"));
    }

    #[test]
    fn test_redact_env_vars() {
        let mgr = SessionSharingManager::new(make_config());
        let text = "export OPENAI_API_KEY=sk_test_abc123 and ANTHROPIC_API_KEY=clk_live_xyz";
        let redacted = mgr.redact_secrets(text);
        assert!(!redacted.contains("sk_test_abc123"));
        assert!(!redacted.contains("clk_live_xyz"));
    }

    #[test]
    fn test_redact_github_tokens() {
        let mgr = SessionSharingManager::new(make_config());
        let text = "token ghp_abcdef1234567890abcdef1234567890abcd";
        let redacted = mgr.redact_secrets(text);
        assert!(!redacted.contains("ghp_abcdef"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_no_secrets() {
        let mgr = SessionSharingManager::new(make_config());
        let text = "This text has no secrets at all";
        let redacted = mgr.redact_secrets(text);
        assert_eq!(redacted, text);
    }

    // --- Export size estimation ---

    #[test]
    fn test_estimate_export_size() {
        let mgr = SessionSharingManager::new(make_config());
        let session = make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success);
        let size = mgr.estimate_export_size(&session);
        assert!(size > 0);
        assert!(size < 10_000);
    }

    #[test]
    fn test_export_too_large() {
        let mut mgr = SessionSharingManager::new(SharingConfig {
            max_export_size_mb: 0, // 0 bytes max
            ..make_config()
        });
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        assert_eq!(mgr.export_session("s-1", ExportFormat::Json), Err(SharingError::ExportTooLarge));
    }

    // --- Delete session ---

    #[test]
    fn test_delete_session() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        assert!(mgr.delete_session("s-1").is_ok());
        assert!(mgr.get_session("s-1").is_none());
    }

    #[test]
    fn test_delete_session_not_found() {
        let mut mgr = SessionSharingManager::new(make_config());
        assert_eq!(mgr.delete_session("nope"), Err(SharingError::SessionNotFound));
    }

    // --- Visibility updates ---

    #[test]
    fn test_update_visibility() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Private, SessionOutcome::Success)).unwrap();
        assert!(mgr.update_visibility("s-1", Visibility::Public).is_ok());
        assert_eq!(mgr.get_session("s-1").unwrap().visibility, Visibility::Public);
    }

    #[test]
    fn test_update_visibility_not_found() {
        let mut mgr = SessionSharingManager::new(make_config());
        assert_eq!(mgr.update_visibility("nope", Visibility::Team), Err(SharingError::SessionNotFound));
    }

    // --- Session summary ---

    #[test]
    fn test_get_session_summary() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let summary = mgr.get_session_summary("s-1");
        assert!(summary.is_some());
        let s = summary.unwrap();
        assert!(s.contains("team"));
        assert!(s.contains("Session s-1"));
        assert!(s.contains("alice"));
        assert!(s.contains("success"));
        assert!(s.contains("anthropic"));
    }

    #[test]
    fn test_get_session_summary_not_found() {
        let mgr = SessionSharingManager::new(make_config());
        assert!(mgr.get_session_summary("nope").is_none());
    }

    // --- Import from JSON ---

    #[test]
    fn test_import_session() {
        let mut mgr = SessionSharingManager::new(SharingConfig { redact_secrets: false, ..make_config() });
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        let json = mgr.export_session("s-1", ExportFormat::Json).unwrap();
        let mut mgr2 = SessionSharingManager::new(make_config());
        let imported = mgr2.import_session(&json).unwrap();
        assert_eq!(imported.id, "s-1");
        assert_eq!(imported.title, "Session s-1");
        assert_eq!(imported.created_by, "alice");
        assert_eq!(imported.visibility, Visibility::Team);
        assert_eq!(imported.metadata.outcome, SessionOutcome::Success);
        assert_eq!(imported.duration_secs, 120);
    }

    #[test]
    fn test_import_session_invalid_json() {
        let mut mgr = SessionSharingManager::new(make_config());
        let result = mgr.import_session("not json at all");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SharingError::FormatError);
    }

    // --- Error display ---

    #[test]
    fn test_error_display() {
        assert_eq!(SharingError::SessionNotFound.to_string(), "session not found");
        assert_eq!(SharingError::ExportTooLarge.to_string(), "export exceeds maximum size");
        assert_eq!(SharingError::DuplicateSession.to_string(), "duplicate session id");
    }

    // --- Enum display ---

    #[test]
    fn test_visibility_display() {
        assert_eq!(Visibility::Private.to_string(), "private");
        assert_eq!(Visibility::Team.to_string(), "team");
        assert_eq!(Visibility::Public.to_string(), "public");
    }

    #[test]
    fn test_change_type_display() {
        assert_eq!(ChangeType::Created.to_string(), "created");
        assert_eq!(ChangeType::Renamed.to_string(), "renamed");
    }

    #[test]
    fn test_outcome_display() {
        assert_eq!(SessionOutcome::Success.to_string(), "success");
        assert_eq!(SessionOutcome::InProgress.to_string(), "in-progress");
    }

    // --- Annotation target types ---

    #[test]
    fn test_annotation_target_variants() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();

        let targets = vec![
            AnnotationTarget::ToolCall("tc-1".to_string()),
            AnnotationTarget::FileChange("src/lib.rs".to_string()),
            AnnotationTarget::ReasoningStep(1),
            AnnotationTarget::General,
        ];

        for (i, target) in targets.into_iter().enumerate() {
            let a = Annotation {
                id: format!("a-{}", i),
                author: "reviewer".to_string(),
                content: format!("Comment {}", i),
                target_type: target,
                timestamp: 1700001000 + i as u64,
            };
            mgr.add_annotation("s-1", a).unwrap();
        }

        let annotations = mgr.get_annotations("s-1").unwrap();
        assert_eq!(annotations.len(), 4);
    }

    // --- Export with redaction ---

    #[test]
    fn test_export_json_with_redaction() {
        let mut mgr = SessionSharingManager::new(make_config());
        let mut s = make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success);
        s.tool_calls[0].output_summary = "key sk-secret123abc found".to_string();
        mgr.share_session(s).unwrap();
        let json = mgr.export_session("s-1", ExportFormat::Json).unwrap();
        assert!(!json.contains("sk-secret123abc"));
        assert!(json.contains("[REDACTED]"));
    }

    // --- Filter combined ---

    #[test]
    fn test_filter_combined_author_and_outcome() {
        let mut mgr = SessionSharingManager::new(make_config());
        mgr.share_session(make_session("s-1", "alice", Visibility::Team, SessionOutcome::Success)).unwrap();
        mgr.share_session(make_session("s-2", "alice", Visibility::Team, SessionOutcome::Failed)).unwrap();
        mgr.share_session(make_session("s-3", "bob", Visibility::Team, SessionOutcome::Success)).unwrap();
        let filter = SessionFilter {
            author: Some("alice".to_string()),
            outcome: Some(SessionOutcome::Success),
            ..Default::default()
        };
        let results = mgr.filter_sessions(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "s-1");
    }
}
