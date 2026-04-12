#![allow(dead_code)]
//! Session export/import — portable session bundles.
//! FIT-GAP v11 Phase 48 — closes gap vs Claude Code 1.x.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Role of a message in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "assistant" => Role::Assistant,
            "system" => Role::System,
            "tool" => Role::Tool,
            _ => Role::User,
        }
    }
}

/// A single message in the session.
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: String,
    pub timestamp_ms: u64,
    pub metadata: HashMap<String, String>,
}

impl Message {
    pub fn new(id: impl Into<String>, role: Role, content: impl Into<String>, ts: u64) -> Self {
        Self { id: id.into(), role, content: content.into(), timestamp_ms: ts, metadata: HashMap::new() }
    }
}

/// Format for the exported session bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Markdown,
    Csv,
}

/// A portable session bundle.
#[derive(Debug, Clone)]
pub struct SessionBundle {
    pub session_id: String,
    pub title: String,
    pub created_ms: u64,
    pub exported_ms: u64,
    pub model: String,
    pub messages: Vec<Message>,
    pub tags: Vec<String>,
    pub extra: HashMap<String, String>,
}

impl SessionBundle {
    pub fn message_count(&self) -> usize { self.messages.len() }
    pub fn word_count(&self) -> usize {
        self.messages.iter().map(|m| m.content.split_whitespace().count()).sum()
    }
}

// ---------------------------------------------------------------------------
// Exporter
// ---------------------------------------------------------------------------

/// Serializes a session bundle to various formats.
pub struct SessionExporter;

impl SessionExporter {
    pub fn export(bundle: &SessionBundle, format: ExportFormat) -> String {
        match format {
            ExportFormat::Json => Self::to_json(bundle),
            ExportFormat::Markdown => Self::to_markdown(bundle),
            ExportFormat::Csv => Self::to_csv(bundle),
        }
    }

    fn to_json(b: &SessionBundle) -> String {
        let msgs: Vec<String> = b.messages.iter().map(|m| {
            format!(
                "{{\"id\":\"{}\",\"role\":\"{}\",\"content\":{},\"ts\":{}}}",
                m.id,
                m.role.as_str(),
                serde_escape(&m.content),
                m.timestamp_ms
            )
        }).collect();
        format!(
            "{{\"session_id\":\"{}\",\"title\":{},\"model\":\"{}\",\"created\":{},\"messages\":[{}]}}",
            b.session_id,
            serde_escape(&b.title),
            b.model,
            b.created_ms,
            msgs.join(",")
        )
    }

    fn to_markdown(b: &SessionBundle) -> String {
        let mut lines = vec![
            format!("# {}", b.title),
            format!("**Session**: {}  |  **Model**: {}  |  **Messages**: {}",
                b.session_id, b.model, b.message_count()),
            String::new(),
        ];
        for msg in &b.messages {
            lines.push(format!("## {} ({})", msg.role.as_str().to_uppercase(), msg.timestamp_ms));
            lines.push(msg.content.clone());
            lines.push(String::new());
        }
        lines.join("\n")
    }

    fn to_csv(b: &SessionBundle) -> String {
        let mut lines = vec!["id,role,timestamp_ms,content".to_string()];
        for m in &b.messages {
            let content_esc = m.content.replace('"', "\"\"");
            lines.push(format!("{},{},{},\"{}\"", m.id, m.role.as_str(), m.timestamp_ms, content_esc));
        }
        lines.join("\n")
    }
}

fn serde_escape(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    format!("\"{}\"", escaped)
}

// ---------------------------------------------------------------------------
// Importer
// ---------------------------------------------------------------------------

/// Minimal JSON deserializer for session bundles.
pub struct SessionImporter;

impl SessionImporter {
    /// Import from Markdown format (re-parse headers + content).
    pub fn from_markdown(src: &str) -> SessionBundle {
        let mut messages = Vec::new();
        let mut title = "Imported Session".to_string();
        let mut current_role: Option<Role> = None;
        let mut current_content = Vec::new();
        let mut msg_counter = 0u64;

        for line in src.lines() {
            if line.starts_with("# ") {
                title = line[2..].to_string();
            } else if line.starts_with("## ") {
                // Flush previous
                if let Some(role) = current_role.take() {
                    let content = current_content.join("\n").trim().to_string();
                    messages.push(Message::new(format!("m{}", msg_counter), role, content, msg_counter));
                    msg_counter += 1;
                    current_content.clear();
                }
                let header = &line[3..];
                let role_str = header.split('(').next().unwrap_or("USER").trim().to_lowercase();
                current_role = Some(Role::from_str(&role_str));
            } else if current_role.is_some() {
                current_content.push(line.to_string());
            }
        }
        if let Some(role) = current_role {
            let content = current_content.join("\n").trim().to_string();
            if !content.is_empty() {
                messages.push(Message::new(format!("m{}", msg_counter), role, content, msg_counter));
            }
        }

        SessionBundle {
            session_id: "imported".to_string(),
            title,
            created_ms: 0,
            exported_ms: 0,
            model: "unknown".to_string(),
            messages,
            tags: Vec::new(),
            extra: HashMap::new(),
        }
    }

    /// Import from CSV (id,role,timestamp_ms,content).
    pub fn from_csv(src: &str) -> SessionBundle {
        let mut messages = Vec::new();
        let mut lines = src.lines();
        lines.next(); // skip header
        for line in lines {
            let mut parts = line.splitn(4, ',');
            let id = parts.next().unwrap_or("").to_string();
            let role = Role::from_str(parts.next().unwrap_or("user"));
            let ts: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let content_raw = parts.next().unwrap_or("");
            let content = content_raw.trim_matches('"').replace("\"\"", "\"");
            messages.push(Message::new(id, role, content, ts));
        }
        SessionBundle {
            session_id: "imported-csv".to_string(),
            title: "Imported CSV Session".to_string(),
            created_ms: 0,
            exported_ms: 0,
            model: "unknown".to_string(),
            messages,
            tags: Vec::new(),
            extra: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bundle() -> SessionBundle {
        let mut msgs = Vec::new();
        msgs.push(Message::new("m1", Role::User, "Hello, explain Rust ownership.", 1000));
        msgs.push(Message::new("m2", Role::Assistant, "Rust ownership means each value has one owner.", 2000));
        SessionBundle {
            session_id: "sess-abc".to_string(),
            title: "Rust Ownership Q&A".to_string(),
            created_ms: 900,
            exported_ms: 3000,
            model: "claude-sonnet-4-6".to_string(),
            messages: msgs,
            tags: vec!["rust".to_string()],
            extra: HashMap::new(),
        }
    }

    #[test]
    fn test_message_count() {
        let b = sample_bundle();
        assert_eq!(b.message_count(), 2);
    }

    #[test]
    fn test_word_count() {
        let b = sample_bundle();
        assert!(b.word_count() > 5);
    }

    #[test]
    fn test_export_json_contains_session_id() {
        let b = sample_bundle();
        let json = SessionExporter::export(&b, ExportFormat::Json);
        assert!(json.contains("sess-abc"));
    }

    #[test]
    fn test_export_json_contains_messages() {
        let b = sample_bundle();
        let json = SessionExporter::export(&b, ExportFormat::Json);
        assert!(json.contains("user"));
        assert!(json.contains("assistant"));
    }

    #[test]
    fn test_export_markdown_contains_title() {
        let b = sample_bundle();
        let md = SessionExporter::export(&b, ExportFormat::Markdown);
        assert!(md.contains("# Rust Ownership Q&A"));
    }

    #[test]
    fn test_export_markdown_has_roles() {
        let b = sample_bundle();
        let md = SessionExporter::export(&b, ExportFormat::Markdown);
        assert!(md.contains("USER"));
        assert!(md.contains("ASSISTANT"));
    }

    #[test]
    fn test_export_csv_header() {
        let b = sample_bundle();
        let csv = SessionExporter::export(&b, ExportFormat::Csv);
        assert!(csv.starts_with("id,role,timestamp_ms,content"));
    }

    #[test]
    fn test_export_csv_rows() {
        let b = sample_bundle();
        let csv = SessionExporter::export(&b, ExportFormat::Csv);
        assert!(csv.contains("m1,user,1000"));
    }

    #[test]
    fn test_import_from_csv() {
        let b = sample_bundle();
        let csv = SessionExporter::export(&b, ExportFormat::Csv);
        let imported = SessionImporter::from_csv(&csv);
        assert_eq!(imported.messages.len(), 2);
        assert_eq!(imported.messages[0].role, Role::User);
    }

    #[test]
    fn test_import_from_markdown() {
        let b = sample_bundle();
        let md = SessionExporter::export(&b, ExportFormat::Markdown);
        let imported = SessionImporter::from_markdown(&md);
        assert_eq!(imported.title, "Rust Ownership Q&A");
        assert!(!imported.messages.is_empty());
    }

    #[test]
    fn test_role_round_trip() {
        for r in &["user", "assistant", "system", "tool"] {
            assert_eq!(Role::from_str(r).as_str(), *r);
        }
    }

    #[test]
    fn test_json_escapes_quotes() {
        let mut b = sample_bundle();
        b.messages[0].content = r#"He said "hello""#.to_string();
        let json = SessionExporter::export(&b, ExportFormat::Json);
        assert!(json.contains(r#"\""#));
    }

    #[test]
    fn test_csv_escapes_commas_in_content() {
        let mut b = sample_bundle();
        b.messages[0].content = "one, two, three".to_string();
        let csv = SessionExporter::export(&b, ExportFormat::Csv);
        // Content should be quoted
        assert!(csv.contains("\"one, two, three\""));
    }
}
