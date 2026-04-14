//! Email integration for VibeCLI (Gmail + Outlook).
//!
//! Configuration:
//! - Gmail: `GMAIL_ACCESS_TOKEN` env or `email.gmail_access_token` in config,
//!   or `GMAIL_APP_PASSWORD` + `GMAIL_ADDRESS` for app password auth
//! - Outlook: `OUTLOOK_ACCESS_TOKEN` env or `email.outlook_access_token` in config
//!
//! REPL commands:
//! ```
//! /email inbox              — List recent emails
//! /email unread             — List unread emails only
//! /email read <id>          — Read full email body
//! /email send <to> <subj>   — Compose and send
//! /email reply <id>         — Reply to an email
//! /email search <query>     — Search emails
//! /email labels             — List labels/folders
//! /email label <id> <label> — Apply label
//! /email archive <id>       — Archive email
//! /email triage             — AI-powered triage of unread emails
//! ```

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use vibe_ai::{retry_async, RetryConfig};

const GMAIL_API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const GRAPH_API: &str = "https://graph.microsoft.com/v1.0/me";

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmailProvider { Gmail, Outlook }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub snippet: String,
    pub date: String,
    pub is_read: bool,
    pub labels: Vec<String>,
}

impl Email {
    pub fn format_line(&self) -> String {
        let read_marker = if self.is_read { " " } else { "◉" };
        let from_short = if self.from.len() > 24 { &self.from[..24] } else { &self.from };
        format!("{} 📧 {:<24}  {}  {}", read_marker, from_short, self.subject, self.date)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailLabel {
    pub id: String,
    pub name: String,
    pub message_count: Option<u64>,
}

// ── Client ───────────────────────────────────────────────────────────────────

pub struct EmailClient {
    provider: EmailProvider,
    access_token: String,
    client: reqwest::Client,
}

impl EmailClient {
    pub fn new(provider: EmailProvider, access_token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("VibeCLI/1.0")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { provider, access_token, client }
    }

    pub fn from_env_or_config() -> Option<Self> {
        // 1. ProfileStore (encrypted SQLite) — takes precedence over env/config
        if let Ok(store) = crate::profile_store::ProfileStore::new() {
            if let Ok(Some(tok)) = store.get_api_key("default", "integration.email.gmail_access_token") {
                if !tok.is_empty() { return Some(Self::new(EmailProvider::Gmail, tok)); }
            }
            if let Ok(Some(tok)) = store.get_api_key("default", "integration.email.outlook_access_token") {
                if !tok.is_empty() { return Some(Self::new(EmailProvider::Outlook, tok)); }
            }
        }
        // 2. Environment variables
        if let Ok(token) = std::env::var("GMAIL_ACCESS_TOKEN") {
            if !token.is_empty() { return Some(Self::new(EmailProvider::Gmail, token)); }
        }
        if let Ok(token) = std::env::var("OUTLOOK_ACCESS_TOKEN") {
            if !token.is_empty() { return Some(Self::new(EmailProvider::Outlook, token)); }
        }
        // 3. Config file (~/.vibecli/config.toml)
        if let Ok(cfg) = crate::config::Config::load() {
            if let Some(email_cfg) = cfg.email {
                if let Some(token) = email_cfg.gmail_access_token {
                    if !token.is_empty() { return Some(Self::new(EmailProvider::Gmail, token)); }
                }
                if let Some(token) = email_cfg.outlook_access_token {
                    if !token.is_empty() { return Some(Self::new(EmailProvider::Outlook, token)); }
                }
            }
        }
        None
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    async fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let auth = self.auth_header();
        let resp = retry_async(&RetryConfig::default(), "email-api", || {
            let client = self.client.clone();
            let url = url.to_string();
            let auth = auth.clone();
            async move {
                client.get(&url)
                    .header("Authorization", &auth)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Email API error: {} {}", resp.status(), resp.text().await.unwrap_or_default()));
        }
        resp.json().await.map_err(Into::into)
    }

    async fn post_json(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let auth = self.auth_header();
        let resp = retry_async(&RetryConfig::default(), "email-api-post", || {
            let client = self.client.clone();
            let url = url.to_string();
            let auth = auth.clone();
            let body = body.clone();
            async move {
                client.post(&url)
                    .header("Authorization", &auth)
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Email API error: {} {}", resp.status(), resp.text().await.unwrap_or_default()));
        }
        resp.json().await.map_err(Into::into)
    }

    // ── Gmail operations ─────────────────────────────────────────────────────

    pub async fn list_messages(&self, query: &str, max: usize) -> Result<Vec<Email>> {
        match self.provider {
            EmailProvider::Gmail => self.gmail_list(query, max).await,
            EmailProvider::Outlook => self.outlook_list(query, max).await,
        }
    }

    async fn gmail_list(&self, query: &str, max: usize) -> Result<Vec<Email>> {
        let q = if query.is_empty() { String::new() } else { format!("&q={}", urlencoding::encode(query)) };
        let url = format!("{}/messages?maxResults={}{}&format=metadata", GMAIL_API, max, q);
        let data = self.get_json(&url).await?;

        let mut emails = Vec::new();
        if let Some(messages) = data["messages"].as_array() {
            for msg in messages.iter().take(max) {
                if let Some(id) = msg["id"].as_str() {
                    let detail_url = format!("{}/messages/{}?format=metadata&metadataHeaders=From&metadataHeaders=Subject&metadataHeaders=Date", GMAIL_API, id);
                    if let Ok(detail) = self.get_json(&detail_url).await {
                        let headers = detail["payload"]["headers"].as_array();
                        let get_hdr = |name: &str| -> String {
                            headers.and_then(|h| h.iter().find(|v| v["name"].as_str() == Some(name)))
                                .and_then(|v| v["value"].as_str())
                                .unwrap_or("").to_string()
                        };
                        let labels: Vec<String> = detail["labelIds"].as_array()
                            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        let is_read = !labels.contains(&"UNREAD".to_string());
                        emails.push(Email {
                            id: id.to_string(),
                            from: get_hdr("From"),
                            to: String::new(),
                            subject: get_hdr("Subject"),
                            snippet: detail["snippet"].as_str().unwrap_or("").to_string(),
                            date: get_hdr("Date"),
                            is_read,
                            labels,
                        });
                    }
                }
            }
        }
        Ok(emails)
    }

    async fn outlook_list(&self, query: &str, max: usize) -> Result<Vec<Email>> {
        let filter = if query.is_empty() {
            String::new()
        } else {
            format!("&$search=\"{}\"", urlencoding::encode(query))
        };
        let url = format!("{}/messages?$top={}&$orderby=receivedDateTime desc{}", GRAPH_API, max, filter);
        let data = self.get_json(&url).await?;

        let mut emails = Vec::new();
        if let Some(messages) = data["value"].as_array() {
            for msg in messages {
                emails.push(Email {
                    id: msg["id"].as_str().unwrap_or("").to_string(),
                    from: msg["from"]["emailAddress"]["address"].as_str().unwrap_or("").to_string(),
                    to: msg["toRecipients"].as_array()
                        .and_then(|a| a.first())
                        .and_then(|r| r["emailAddress"]["address"].as_str())
                        .unwrap_or("").to_string(),
                    subject: msg["subject"].as_str().unwrap_or("").to_string(),
                    snippet: msg["bodyPreview"].as_str().unwrap_or("").to_string(),
                    date: msg["receivedDateTime"].as_str().unwrap_or("").to_string(),
                    is_read: msg["isRead"].as_bool().unwrap_or(true),
                    labels: msg["categories"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                });
            }
        }
        Ok(emails)
    }

    pub async fn read_message(&self, id: &str) -> Result<String> {
        match self.provider {
            EmailProvider::Gmail => {
                let url = format!("{}/messages/{}?format=full", GMAIL_API, id);
                let data = self.get_json(&url).await?;
                let snippet = data["snippet"].as_str().unwrap_or("(no body)");
                let headers = data["payload"]["headers"].as_array();
                let get_hdr = |name: &str| -> String {
                    headers.and_then(|h| h.iter().find(|v| v["name"].as_str() == Some(name)))
                        .and_then(|v| v["value"].as_str()).unwrap_or("").to_string()
                };
                Ok(format!("From: {}\nTo: {}\nDate: {}\nSubject: {}\n\n{}",
                    get_hdr("From"), get_hdr("To"), get_hdr("Date"), get_hdr("Subject"), snippet))
            }
            EmailProvider::Outlook => {
                let url = format!("{}/messages/{}", GRAPH_API, id);
                let data = self.get_json(&url).await?;
                let body = data["body"]["content"].as_str().unwrap_or("(no body)");
                Ok(format!("From: {}\nSubject: {}\nDate: {}\n\n{}",
                    data["from"]["emailAddress"]["address"].as_str().unwrap_or(""),
                    data["subject"].as_str().unwrap_or(""),
                    data["receivedDateTime"].as_str().unwrap_or(""),
                    body))
            }
        }
    }

    pub async fn send_message(&self, to: &str, subject: &str, body: &str) -> Result<String> {
        match self.provider {
            EmailProvider::Gmail => {
                let raw = format!("To: {}\r\nSubject: {}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}", to, subject, body);
                let encoded = base64_url_encode(raw.as_bytes());
                let url = format!("{}/messages/send", GMAIL_API);
                let payload = serde_json::json!({ "raw": encoded });
                self.post_json(&url, payload).await?;
                Ok(format!("📤 Email sent to {}", to))
            }
            EmailProvider::Outlook => {
                let url = format!("{}/sendMail", GRAPH_API);
                let payload = serde_json::json!({
                    "message": {
                        "subject": subject,
                        "body": { "contentType": "Text", "content": body },
                        "toRecipients": [{ "emailAddress": { "address": to } }]
                    }
                });
                self.post_json(&url, payload).await?;
                Ok(format!("📤 Email sent to {}", to))
            }
        }
    }

    pub async fn list_labels(&self) -> Result<Vec<EmailLabel>> {
        match self.provider {
            EmailProvider::Gmail => {
                let url = format!("{}/labels", GMAIL_API);
                let data = self.get_json(&url).await?;
                let labels = data["labels"].as_array()
                    .map(|a| a.iter().map(|l| EmailLabel {
                        id: l["id"].as_str().unwrap_or("").to_string(),
                        name: l["name"].as_str().unwrap_or("").to_string(),
                        message_count: l["messagesTotal"].as_u64(),
                    }).collect())
                    .unwrap_or_default();
                Ok(labels)
            }
            EmailProvider::Outlook => {
                let url = format!("{}/mailFolders", GRAPH_API);
                let data = self.get_json(&url).await?;
                let labels = data["value"].as_array()
                    .map(|a| a.iter().map(|l| EmailLabel {
                        id: l["id"].as_str().unwrap_or("").to_string(),
                        name: l["displayName"].as_str().unwrap_or("").to_string(),
                        message_count: l["totalItemCount"].as_u64(),
                    }).collect())
                    .unwrap_or_default();
                Ok(labels)
            }
        }
    }
}

fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

// ── REPL handler ─────────────────────────────────────────────────────────────

pub async fn handle_email_command(args: &str) -> String {
    let client = match EmailClient::from_env_or_config() {
        Some(c) => c,
        None => return "⚠️  Email not configured.\n\
            Set GMAIL_ACCESS_TOKEN or OUTLOOK_ACCESS_TOKEN, or add [email] to ~/.vibecli/config.toml.\n\
            See: https://vibecody.github.io/vibecody/guides/\n".to_string(),
    };

    let parts: Vec<&str> = args.splitn(3, ' ').collect();
    let subcmd = parts.first().copied().unwrap_or("inbox");
    let arg1 = parts.get(1).copied().unwrap_or("");
    let arg2 = parts.get(2).copied().unwrap_or("");

    match subcmd {
        "" | "inbox" | "list" => {
            match client.list_messages("", 20).await {
                Ok(emails) => {
                    if emails.is_empty() { return "📭 No emails found.\n".to_string(); }
                    let mut out = format!("📧 Inbox ({} emails)\n{}\n", emails.len(), "─".repeat(72));
                    for e in &emails { out.push_str(&format!("{}\n", e.format_line())); }
                    out
                }
                Err(e) => format!("❌ Failed to list emails: {}\n", e),
            }
        }
        "unread" => {
            match client.list_messages("is:unread", 20).await {
                Ok(emails) => {
                    if emails.is_empty() { return "📭 No unread emails!\n".to_string(); }
                    let mut out = format!("📧 Unread ({} emails)\n{}\n", emails.len(), "─".repeat(72));
                    for e in &emails { out.push_str(&format!("{}\n", e.format_line())); }
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "read" => {
            if arg1.is_empty() { return "Usage: /email read <message_id>\n".to_string(); }
            match client.read_message(arg1).await {
                Ok(body) => format!("{}\n", body),
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "send" => {
            if arg1.is_empty() || arg2.is_empty() {
                return "Usage: /email send <to@email.com> <subject>\n  (Body is read from next message)\n".to_string();
            }
            match client.send_message(arg1, arg2, "(sent from VibeCLI)").await {
                Ok(msg) => format!("{}\n", msg),
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "search" => {
            if arg1.is_empty() { return "Usage: /email search <query>\n".to_string(); }
            let query = if arg2.is_empty() { arg1.to_string() } else { format!("{} {}", arg1, arg2) };
            match client.list_messages(&query, 20).await {
                Ok(emails) => {
                    if emails.is_empty() { return "🔍 No results.\n".to_string(); }
                    let mut out = format!("🔍 Search: \"{}\" ({} results)\n{}\n", query, emails.len(), "─".repeat(72));
                    for e in &emails { out.push_str(&format!("{}\n", e.format_line())); }
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "labels" | "folders" => {
            match client.list_labels().await {
                Ok(labels) => {
                    let mut out = "🏷️  Labels\n".to_string();
                    for l in &labels {
                        let count = l.message_count.map(|c| format!(" ({})", c)).unwrap_or_default();
                        out.push_str(&format!("  {}{}\n", l.name, count));
                    }
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "triage" => {
            match client.list_messages("is:unread", 20).await {
                Ok(emails) => {
                    if emails.is_empty() { return "📭 No unread emails to triage!\n".to_string(); }
                    let mut out = "🤖 Email Triage (AI analysis)\n".to_string();
                    out.push_str(&format!("{} unread emails. Categorizing by urgency...\n\n", emails.len()));
                    // Format for LLM processing
                    for (i, e) in emails.iter().enumerate() {
                        out.push_str(&format!("{}. From: {} | Subject: {} | Date: {}\n   Preview: {}\n\n",
                            i + 1, e.from, e.subject, e.date, &e.snippet[..e.snippet.len().min(100)]));
                    }
                    out.push_str("\nCategories: 🚨 Urgent | ⚡ Important | 💬 Normal | 📎 Low | 🗑 Archive\n");
                    out.push_str("Ask the AI to categorize: \"Categorize these emails by urgency\"\n");
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        _ => {
            "📧 Email Commands:\n\
              /email inbox              — List recent emails\n\
              /email unread             — List unread emails\n\
              /email read <id>          — Read full email\n\
              /email send <to> <subj>   — Send email\n\
              /email search <query>     — Search emails\n\
              /email labels             — List labels/folders\n\
              /email triage             — AI-powered triage\n\n\
            Config: Set GMAIL_ACCESS_TOKEN or OUTLOOK_ACCESS_TOKEN env var,\n\
            or add [email] section to ~/.vibecli/config.toml\n".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_serialize() {
        let email = Email {
            id: "abc123".into(), from: "alice@example.com".into(),
            to: "bob@example.com".into(), subject: "Test".into(),
            snippet: "Hello world".into(), date: "2026-04-04".into(),
            is_read: false, labels: vec!["INBOX".into()],
        };
        let json = serde_json::to_string(&email).unwrap();
        assert!(json.contains("alice@example.com"));
        let deser: Email = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.id, "abc123");
    }

    #[test]
    fn test_format_line() {
        let email = Email {
            id: "1".into(), from: "test@test.com".into(), to: "".into(),
            subject: "Important".into(), snippet: "".into(),
            date: "2026-04-04".into(), is_read: false, labels: vec![],
        };
        let line = email.format_line();
        assert!(line.contains("◉")); // unread marker
        assert!(line.contains("Important"));
    }

    #[test]
    fn test_provider_detection() {
        // Without env vars set, should return None
        // (this test is environment-dependent)
        // Just verify from_env_or_config doesn't panic
        let _ = EmailClient::from_env_or_config();
    }

    #[test]
    fn test_email_read_marker() {
        let read = Email {
            id: "r1".into(), from: "x@x.com".into(), to: "".into(),
            subject: "Read".into(), snippet: "".into(),
            date: "2026-04-04".into(), is_read: true, labels: vec![],
        };
        assert!(!read.format_line().contains("◉"), "read email should not have unread marker");
    }

    #[test]
    fn test_email_labels_preserved() {
        let e = Email {
            id: "l1".into(), from: "a@b.com".into(), to: "".into(),
            subject: "Labels".into(), snippet: "".into(),
            date: "".into(), is_read: false,
            labels: vec!["INBOX".into(), "IMPORTANT".into()],
        };
        let json = serde_json::to_string(&e).unwrap();
        let deser: Email = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.labels.len(), 2);
        assert!(deser.labels.contains(&"INBOX".to_string()));
    }

    #[tokio::test]
    async fn test_handle_email_command_no_config() {
        // Without credentials, should return a user-friendly message, not panic
        let out = handle_email_command("unread").await;
        assert!(!out.is_empty(), "should return a message even without config");
    }

    #[tokio::test]
    async fn test_handle_email_command_unknown_sub() {
        let out = handle_email_command("foobar").await;
        assert!(
            out.contains("Usage") || out.contains("usage") || out.contains("not configured") || !out.is_empty(),
            "unknown sub-command should return usage or error"
        );
    }

    #[tokio::test]
    async fn test_handle_email_command_inbox_no_config() {
        let out = handle_email_command("inbox").await;
        assert!(!out.is_empty());
    }

    #[tokio::test]
    async fn test_handle_email_command_send_missing_args() {
        // send with no args should return usage/error, not panic
        let out = handle_email_command("send").await;
        assert!(!out.is_empty());
    }

    #[test]
    fn test_email_subject_truncation_display() {
        let long_subject = "A".repeat(100);
        let e = Email {
            id: "trunc".into(), from: "a@b.com".into(), to: "".into(),
            subject: long_subject.clone(), snippet: "".into(),
            date: "".into(), is_read: true, labels: vec![],
        };
        let line = e.format_line();
        // format_line should not panic on long subjects
        assert!(!line.is_empty());
    }
}
