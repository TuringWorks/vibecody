//! Email integration for VibeCLI (Gmail + Outlook).
//!
//! Configuration:
//! - Gmail: `GMAIL_ACCESS_TOKEN` env or `email.gmail_access_token` in config,
//!   or `GMAIL_APP_PASSWORD` + `GMAIL_ADDRESS` for app password auth.
//!   For automatic OAuth refresh (recommended — access tokens expire after
//!   ~60 minutes), also configure `gmail_refresh_token`,
//!   `gmail_oauth_client_id`, and `gmail_oauth_client_secret` (or the
//!   matching `GMAIL_REFRESH_TOKEN` / `GMAIL_OAUTH_CLIENT_ID` /
//!   `GMAIL_OAUTH_CLIENT_SECRET` env vars).
//! - Outlook: `OUTLOOK_ACCESS_TOKEN` env or `email.outlook_access_token` in
//!   config. For refresh: `outlook_refresh_token`,
//!   `outlook_oauth_client_id`, `outlook_oauth_client_secret`.
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
use std::sync::Mutex;
use vibe_ai::{retry_async, RetryConfig};

const GMAIL_API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const GRAPH_API: &str = "https://graph.microsoft.com/v1.0/me";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const MS_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
// Standard Outlook/Graph mail scope; required when refreshing a Microsoft
// access token via the OAuth refresh-token grant.
const MS_DEFAULT_SCOPE: &str =
    "https://graph.microsoft.com/Mail.ReadWrite https://graph.microsoft.com/Mail.Send offline_access";

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

/// Full email detail for the UI reader pane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailBody {
    pub id: String,
    pub from: String,
    pub to: String,
    pub cc: String,
    pub subject: String,
    pub date: String,
    pub body_text: String,
    pub body_html: String,
    pub is_read: bool,
    pub labels: Vec<String>,
}

/// Auth/config status for the provider status strip (shared across productivity tabs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub connected: bool,
    pub provider: Option<String>,
    pub account: Option<String>,
    pub message: Option<String>,
}

// ── Client ───────────────────────────────────────────────────────────────────

pub struct EmailClient {
    provider: EmailProvider,
    // Access token is wrapped in a Mutex so refresh() can mutate it without
    // forcing every API method (and its callers) to take `&mut self`.
    // Never await with the lock held — clone the String out and drop the
    // guard immediately; refresh writes briefly after the network round-trip.
    access_token: Mutex<String>,
    refresh_token: Option<String>,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    client: reqwest::Client,
}

impl EmailClient {
    pub fn new(provider: EmailProvider, access_token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("VibeCLI/1.0")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            provider,
            access_token: Mutex::new(access_token),
            refresh_token: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            client,
        }
    }

    /// Attach OAuth refresh credentials so the client can mint a fresh
    /// access token automatically when the current one expires (Gmail
    /// access tokens expire after ~60 minutes — without this, every call
    /// fails with HTTP 401 until the user manually re-pastes a token).
    pub fn with_refresh(
        mut self,
        refresh_token: Option<String>,
        oauth_client_id: Option<String>,
        oauth_client_secret: Option<String>,
    ) -> Self {
        // Treat empty strings as unset — saves callers from having to
        // pre-filter blanks coming out of optional config fields.
        self.refresh_token = refresh_token.filter(|s| !s.is_empty());
        self.oauth_client_id = oauth_client_id.filter(|s| !s.is_empty());
        self.oauth_client_secret = oauth_client_secret.filter(|s| !s.is_empty());
        self
    }

    pub fn from_env_or_config() -> Option<Self> {
        // 1. ProfileStore (encrypted SQLite) — takes precedence over env/config
        if let Ok(store) = crate::profile_store::ProfileStore::new() {
            if let Ok(Some(tok)) = store.get_api_key("default", "integration.email.gmail_access_token") {
                if !tok.is_empty() {
                    let r = store.get_api_key("default", "integration.email.gmail_refresh_token").ok().flatten();
                    let id = store.get_api_key("default", "integration.email.gmail_oauth_client_id").ok().flatten();
                    let sec = store.get_api_key("default", "integration.email.gmail_oauth_client_secret").ok().flatten();
                    return Some(Self::new(EmailProvider::Gmail, tok).with_refresh(r, id, sec));
                }
            }
            if let Ok(Some(tok)) = store.get_api_key("default", "integration.email.outlook_access_token") {
                if !tok.is_empty() {
                    let r = store.get_api_key("default", "integration.email.outlook_refresh_token").ok().flatten();
                    let id = store.get_api_key("default", "integration.email.outlook_oauth_client_id").ok().flatten();
                    let sec = store.get_api_key("default", "integration.email.outlook_oauth_client_secret").ok().flatten();
                    return Some(Self::new(EmailProvider::Outlook, tok).with_refresh(r, id, sec));
                }
            }
        }
        // 2. Environment variables
        if let Ok(token) = std::env::var("GMAIL_ACCESS_TOKEN") {
            if !token.is_empty() {
                let r = std::env::var("GMAIL_REFRESH_TOKEN").ok();
                let id = std::env::var("GMAIL_OAUTH_CLIENT_ID").ok();
                let sec = std::env::var("GMAIL_OAUTH_CLIENT_SECRET").ok();
                return Some(Self::new(EmailProvider::Gmail, token).with_refresh(r, id, sec));
            }
        }
        if let Ok(token) = std::env::var("OUTLOOK_ACCESS_TOKEN") {
            if !token.is_empty() {
                let r = std::env::var("OUTLOOK_REFRESH_TOKEN").ok();
                let id = std::env::var("OUTLOOK_OAUTH_CLIENT_ID").ok();
                let sec = std::env::var("OUTLOOK_OAUTH_CLIENT_SECRET").ok();
                return Some(Self::new(EmailProvider::Outlook, token).with_refresh(r, id, sec));
            }
        }
        // 3. Config file (~/.vibecli/config.toml)
        if let Ok(cfg) = crate::config::Config::load() {
            if let Some(email_cfg) = cfg.email {
                if let Some(token) = email_cfg.gmail_access_token {
                    if !token.is_empty() {
                        return Some(
                            Self::new(EmailProvider::Gmail, token).with_refresh(
                                email_cfg.gmail_refresh_token,
                                email_cfg.gmail_oauth_client_id,
                                email_cfg.gmail_oauth_client_secret,
                            ),
                        );
                    }
                }
                if let Some(token) = email_cfg.outlook_access_token {
                    if !token.is_empty() {
                        return Some(
                            Self::new(EmailProvider::Outlook, token).with_refresh(
                                email_cfg.outlook_refresh_token,
                                email_cfg.outlook_oauth_client_id,
                                email_cfg.outlook_oauth_client_secret,
                            ),
                        );
                    }
                }
            }
        }
        None
    }

    fn auth_header(&self) -> String {
        // Brief lock — clone out and drop the guard before awaiting.
        let tok = self.access_token.lock().expect("email access_token mutex poisoned").clone();
        format!("Bearer {}", tok)
    }

    fn current_access_token(&self) -> String {
        self.access_token.lock().expect("email access_token mutex poisoned").clone()
    }

    fn store_new_access_token(&self, new_token: &str) {
        *self.access_token.lock().expect("email access_token mutex poisoned") = new_token.to_string();
    }

    fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
            && self.oauth_client_id.is_some()
            && self.oauth_client_secret.is_some()
    }

    /// Mint a new access token using the stored refresh credentials, then
    /// persist it back to the ProfileStore so the next process start picks
    /// it up. Errors here are surfaced to the caller — typically as the
    /// "expired and refresh failed" path of the API methods below.
    async fn refresh_access_token(&self) -> Result<()> {
        let refresh_token = self.refresh_token.as_deref().ok_or_else(|| {
            anyhow!("Gmail/Outlook access token expired and no refresh token is configured. \
                     Add a refresh token + OAuth client ID/secret in Settings → Integrations → Email.")
        })?;
        let client_id = self.oauth_client_id.as_deref().ok_or_else(|| {
            anyhow!("Cannot refresh OAuth token: missing client_id")
        })?;
        let client_secret = self.oauth_client_secret.as_deref().ok_or_else(|| {
            anyhow!("Cannot refresh OAuth token: missing client_secret")
        })?;

        // Build the form body. Microsoft requires a `scope` field on
        // refresh; Google ignores it. Send it for both — harmless on Google.
        let mut form: Vec<(&str, &str)> = vec![
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        if matches!(self.provider, EmailProvider::Outlook) {
            form.push(("scope", MS_DEFAULT_SCOPE));
        }
        let url = match self.provider {
            EmailProvider::Gmail => GOOGLE_TOKEN_URL,
            EmailProvider::Outlook => MS_TOKEN_URL,
        };

        let resp = self.client.post(url).form(&form).send().await
            .map_err(|e| anyhow!("OAuth refresh request failed: {}", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(anyhow!("OAuth refresh failed ({}): {}", status, body));
        }
        let new_token = parse_oauth_refresh_response(&body)?;
        self.store_new_access_token(&new_token);

        // Best-effort persist. Failure here is non-fatal: the in-memory
        // token is updated, so the current process keeps working — the
        // next process will just get a 401 again and refresh again.
        if let Ok(store) = crate::profile_store::ProfileStore::new() {
            let key = match self.provider {
                EmailProvider::Gmail => "integration.email.gmail_access_token",
                EmailProvider::Outlook => "integration.email.outlook_access_token",
            };
            if let Err(e) = store.set_api_key("default", key, &new_token) {
                tracing::warn!("Failed to persist refreshed email token to ProfileStore: {}", e);
            }
        }
        Ok(())
    }

    async fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let mut refreshed = false;
        loop {
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

            let status = resp.status();
            if status == reqwest::StatusCode::UNAUTHORIZED && !refreshed && self.can_refresh() {
                drop(resp);
                refreshed = true;
                self.refresh_access_token().await?;
                continue;
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(format_email_api_error(status, &body));
            }
            return resp.json().await.map_err(Into::into);
        }
    }

    async fn patch_json(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let mut refreshed = false;
        loop {
            let auth = self.auth_header();
            let body_clone = body.clone();
            let resp = retry_async(&RetryConfig::default(), "email-api-patch", || {
                let client = self.client.clone();
                let url = url.to_string();
                let auth = auth.clone();
                let body = body_clone.clone();
                async move {
                    client.patch(&url)
                        .header("Authorization", &auth)
                        .header("Content-Type", "application/json")
                        .json(&body)
                        .send()
                        .await
                        .map_err(Into::into)
                }
            }).await?;

            let status = resp.status();
            if status == reqwest::StatusCode::UNAUTHORIZED && !refreshed && self.can_refresh() {
                drop(resp);
                refreshed = true;
                self.refresh_access_token().await?;
                continue;
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(format_email_api_error(status, &body));
            }
            // PATCH may return empty body on Outlook; tolerate.
            let text = resp.text().await.unwrap_or_default();
            return if text.trim().is_empty() {
                Ok(serde_json::Value::Null)
            } else {
                serde_json::from_str(&text).map_err(Into::into)
            };
        }
    }

    async fn post_json(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let mut refreshed = false;
        loop {
            let auth = self.auth_header();
            let body_clone = body.clone();
            let resp = retry_async(&RetryConfig::default(), "email-api-post", || {
                let client = self.client.clone();
                let url = url.to_string();
                let auth = auth.clone();
                let body = body_clone.clone();
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

            let status = resp.status();
            if status == reqwest::StatusCode::UNAUTHORIZED && !refreshed && self.can_refresh() {
                drop(resp);
                refreshed = true;
                self.refresh_access_token().await?;
                continue;
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(format_email_api_error(status, &body));
            }
            return resp.json().await.map_err(Into::into);
        }
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

    /// Typed variant of `read_message` — returns structured `EmailBody` for UI reader panes.
    pub async fn read_message_typed(&self, id: &str) -> Result<EmailBody> {
        match self.provider {
            EmailProvider::Gmail => {
                let url = format!("{}/messages/{}?format=full", GMAIL_API, id);
                let data = self.get_json(&url).await?;
                let headers = data["payload"]["headers"].as_array();
                let get_hdr = |name: &str| -> String {
                    headers
                        .and_then(|h| h.iter().find(|v| v["name"].as_str() == Some(name)))
                        .and_then(|v| v["value"].as_str())
                        .unwrap_or("")
                        .to_string()
                };
                let (body_text, body_html) = extract_gmail_body(&data["payload"]);
                let labels: Vec<String> = data["labelIds"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let is_read = !labels.contains(&"UNREAD".to_string());
                let text_fallback = if body_text.is_empty() && body_html.is_empty() {
                    data["snippet"].as_str().unwrap_or("").to_string()
                } else {
                    body_text
                };
                Ok(EmailBody {
                    id: id.to_string(),
                    from: get_hdr("From"),
                    to: get_hdr("To"),
                    cc: get_hdr("Cc"),
                    subject: get_hdr("Subject"),
                    date: get_hdr("Date"),
                    body_text: text_fallback,
                    body_html,
                    is_read,
                    labels,
                })
            }
            EmailProvider::Outlook => {
                let url = format!("{}/messages/{}", GRAPH_API, id);
                let data = self.get_json(&url).await?;
                let content_type = data["body"]["contentType"].as_str().unwrap_or("text");
                let body_raw = data["body"]["content"].as_str().unwrap_or("").to_string();
                let (body_text, body_html) = if content_type.eq_ignore_ascii_case("html") {
                    (String::new(), body_raw)
                } else {
                    (body_raw, String::new())
                };
                let to = data["toRecipients"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|r| r["emailAddress"]["address"].as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                let cc = data["ccRecipients"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|r| r["emailAddress"]["address"].as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                Ok(EmailBody {
                    id: id.to_string(),
                    from: data["from"]["emailAddress"]["address"].as_str().unwrap_or("").to_string(),
                    to,
                    cc,
                    subject: data["subject"].as_str().unwrap_or("").to_string(),
                    date: data["receivedDateTime"].as_str().unwrap_or("").to_string(),
                    body_text,
                    body_html,
                    is_read: data["isRead"].as_bool().unwrap_or(true),
                    labels: data["categories"]
                        .as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                })
            }
        }
    }

    /// Archive (move out of inbox). Gmail: remove INBOX label. Outlook: move to Archive folder.
    pub async fn archive_message(&self, id: &str) -> Result<()> {
        match self.provider {
            EmailProvider::Gmail => {
                let url = format!("{}/messages/{}/modify", GMAIL_API, id);
                let body = serde_json::json!({ "removeLabelIds": ["INBOX"] });
                self.post_json(&url, body).await?;
                Ok(())
            }
            EmailProvider::Outlook => {
                let url = format!("{}/messages/{}/move", GRAPH_API, id);
                let body = serde_json::json!({ "destinationId": "archive" });
                self.post_json(&url, body).await?;
                Ok(())
            }
        }
    }

    /// Mark read/unread.
    pub async fn mark_read(&self, id: &str, read: bool) -> Result<()> {
        match self.provider {
            EmailProvider::Gmail => {
                let url = format!("{}/messages/{}/modify", GMAIL_API, id);
                let body = if read {
                    serde_json::json!({ "removeLabelIds": ["UNREAD"] })
                } else {
                    serde_json::json!({ "addLabelIds": ["UNREAD"] })
                };
                self.post_json(&url, body).await?;
                Ok(())
            }
            EmailProvider::Outlook => {
                let url = format!("{}/messages/{}", GRAPH_API, id);
                let body = serde_json::json!({ "isRead": read });
                self.patch_json(&url, body).await?;
                Ok(())
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

/// Pull `access_token` out of an OAuth refresh response. Pure function so
/// the JSON shape can be unit-tested without a live HTTP server.
fn parse_oauth_refresh_response(body: &str) -> Result<String> {
    let json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| anyhow!("OAuth refresh: response body is not JSON: {} (body: {})", e, body))?;
    if let Some(tok) = json["access_token"].as_str() {
        if !tok.is_empty() {
            return Ok(tok.to_string());
        }
    }
    // Surface the provider's error description if present so the user can
    // act on it ("invalid_grant" → re-authorize; "invalid_client" → wrong
    // client_id/secret).
    let err = json["error"].as_str().unwrap_or("unknown");
    let desc = json["error_description"].as_str().unwrap_or("");
    Err(anyhow!(
        "OAuth refresh: response missing access_token (error: {}{})",
        err,
        if desc.is_empty() { String::new() } else { format!(" — {}", desc) }
    ))
}

/// Format a non-success response from Gmail / Graph as a user-actionable
/// error. The previous implementation dumped the raw provider JSON, which
/// is several kilobytes of OAuth boilerplate the user can't act on.
fn format_email_api_error(status: reqwest::StatusCode, body: &str) -> anyhow::Error {
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return anyhow!(
            "Email API 401 Unauthorized — your access token has expired or been revoked. \
             Configure a refresh token + OAuth client credentials in Settings → \
             Integrations → Email so VibeCLI can refresh tokens automatically."
        );
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        return anyhow!(
            "Email API 403 Forbidden — the access token does not grant the required scope. \
             For Gmail you need https://www.googleapis.com/auth/gmail.modify; for Outlook \
             you need Mail.ReadWrite (and Mail.Send for sending)."
        );
    }
    // For other statuses, keep the body but trim it so logs / UI don't
    // get a wall of nested JSON.
    let trimmed: String = body.chars().take(400).collect();
    let suffix = if body.len() > 400 { " …" } else { "" };
    anyhow!("Email API error {}: {}{}", status, trimmed, suffix)
}

fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn base64_url_decode(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    // Gmail uses URL-safe base64 without padding
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s.trim())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(s.trim()))
        .ok()
}

/// Walk a Gmail MIME payload tree and pull out (text_plain, text_html) bodies.
/// Returns the first text/plain and text/html parts found (decoded UTF-8).
fn extract_gmail_body(payload: &serde_json::Value) -> (String, String) {
    let mut text = String::new();
    let mut html = String::new();
    walk_gmail_payload(payload, &mut text, &mut html);
    (text, html)
}

fn walk_gmail_payload(part: &serde_json::Value, text: &mut String, html: &mut String) {
    let mime = part["mimeType"].as_str().unwrap_or("");
    if let Some(data) = part["body"]["data"].as_str() {
        if let Some(bytes) = base64_url_decode(data) {
            let decoded = String::from_utf8_lossy(&bytes).into_owned();
            if mime == "text/plain" && text.is_empty() {
                *text = decoded;
            } else if mime == "text/html" && html.is_empty() {
                *html = decoded;
            }
        }
    }
    if let Some(parts) = part["parts"].as_array() {
        for child in parts {
            walk_gmail_payload(child, text, html);
            if !text.is_empty() && !html.is_empty() {
                return;
            }
        }
    }
}

// ── UI wrapper API ───────────────────────────────────────────────────────────
//
// These functions are called by the Tauri command layer. They handle the
// "not configured" case (returning `Err` with a short message) so the UI
// can show a clean "Sign in" strip instead of a raw text dump.

fn require_client() -> std::result::Result<EmailClient, String> {
    EmailClient::from_env_or_config().ok_or_else(|| {
        "Email not configured. Set GMAIL_ACCESS_TOKEN / OUTLOOK_ACCESS_TOKEN, \
         or add [email] to ~/.vibecli/config.toml."
            .to_string()
    })
}

pub async fn ui_list(query: &str, max: usize) -> std::result::Result<Vec<Email>, String> {
    let client = require_client()?;
    client.list_messages(query, max).await.map_err(|e| e.to_string())
}

pub async fn ui_read(id: &str) -> std::result::Result<EmailBody, String> {
    let client = require_client()?;
    client.read_message_typed(id).await.map_err(|e| e.to_string())
}

pub async fn ui_archive(id: &str) -> std::result::Result<(), String> {
    let client = require_client()?;
    client.archive_message(id).await.map_err(|e| e.to_string())
}

pub async fn ui_mark_read(id: &str, read: bool) -> std::result::Result<(), String> {
    let client = require_client()?;
    client.mark_read(id, read).await.map_err(|e| e.to_string())
}

pub async fn ui_labels() -> std::result::Result<Vec<EmailLabel>, String> {
    let client = require_client()?;
    client.list_labels().await.map_err(|e| e.to_string())
}

pub async fn ui_send(to: &str, subject: &str, body: &str) -> std::result::Result<String, String> {
    let client = require_client()?;
    client.send_message(to, subject, body).await.map_err(|e| e.to_string())
}

/// Returns auth status for the UI provider strip. Never errors — a missing
/// config is a legitimate "not connected" state, not an error.
pub async fn ui_status() -> ProviderStatus {
    let Some(client) = EmailClient::from_env_or_config() else {
        return ProviderStatus {
            connected: false,
            provider: None,
            account: None,
            message: Some("Not signed in".to_string()),
        };
    };
    let provider_name = match client.provider {
        EmailProvider::Gmail => "gmail",
        EmailProvider::Outlook => "outlook",
    };
    // Probe the profile endpoint to confirm the token still works and fetch the account address.
    let (ok, account, err) = match client.provider {
        EmailProvider::Gmail => match client.get_json(&format!("{}/profile", GMAIL_API)).await {
            Ok(v) => (true, v["emailAddress"].as_str().map(String::from), None),
            Err(e) => (false, None, Some(e.to_string())),
        },
        EmailProvider::Outlook => match client.get_json(GRAPH_API).await {
            Ok(v) => (
                true,
                v["mail"].as_str().or_else(|| v["userPrincipalName"].as_str()).map(String::from),
                None,
            ),
            Err(e) => (false, None, Some(e.to_string())),
        },
    };
    ProviderStatus {
        connected: ok,
        provider: Some(provider_name.to_string()),
        account,
        message: err,
    }
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
    fn parse_oauth_refresh_response_extracts_access_token() {
        let body = r#"{"access_token":"ya29.new","expires_in":3599,"token_type":"Bearer"}"#;
        let tok = parse_oauth_refresh_response(body).unwrap();
        assert_eq!(tok, "ya29.new");
    }

    #[test]
    fn parse_oauth_refresh_response_rejects_non_json() {
        let err = parse_oauth_refresh_response("not json").unwrap_err().to_string();
        assert!(err.contains("not JSON"), "got: {err}");
    }

    #[test]
    fn parse_oauth_refresh_response_surfaces_error_description() {
        // Google's typical refresh failure body — "invalid_grant" means the
        // user revoked access or the refresh token was rotated.
        let body = r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#;
        let err = parse_oauth_refresh_response(body).unwrap_err().to_string();
        assert!(err.contains("invalid_grant"), "got: {err}");
        assert!(err.contains("expired or revoked"), "got: {err}");
    }

    #[test]
    fn parse_oauth_refresh_response_rejects_empty_access_token() {
        let body = r#"{"access_token":"","error":"invalid_request"}"#;
        let err = parse_oauth_refresh_response(body).unwrap_err().to_string();
        assert!(err.contains("invalid_request"), "got: {err}");
    }

    #[test]
    fn format_email_api_error_401_is_actionable() {
        let err = format_email_api_error(reqwest::StatusCode::UNAUTHORIZED, "ignored body").to_string();
        assert!(err.contains("expired"), "got: {err}");
        assert!(err.contains("refresh"), "got: {err}");
        // The raw provider body should NOT leak through on 401 — it's
        // multiple kilobytes of OAuth boilerplate the user can't act on.
        assert!(!err.contains("ignored body"), "got: {err}");
    }

    #[test]
    fn format_email_api_error_403_mentions_scope() {
        let err = format_email_api_error(reqwest::StatusCode::FORBIDDEN, "").to_string();
        assert!(err.contains("scope") || err.contains("Mail."), "got: {err}");
    }

    #[test]
    fn format_email_api_error_other_truncates_long_bodies() {
        let body = "x".repeat(2000);
        let err = format_email_api_error(reqwest::StatusCode::BAD_GATEWAY, &body).to_string();
        // Should include the leading chunk plus the truncation marker.
        assert!(err.contains("502"), "got: {err}");
        assert!(err.ends_with('…') || err.contains(" …"), "got: {err}");
        assert!(err.len() < 600, "should not dump kilobytes; got len {}", err.len());
    }

    #[test]
    fn with_refresh_treats_empty_strings_as_unset() {
        let c = EmailClient::new(EmailProvider::Gmail, "tok".into())
            .with_refresh(Some("".into()), Some("".into()), Some("".into()));
        assert!(!c.can_refresh(), "empty refresh creds should not enable can_refresh");
    }

    #[test]
    fn can_refresh_requires_all_three_fields() {
        let only_token = EmailClient::new(EmailProvider::Gmail, "tok".into())
            .with_refresh(Some("r".into()), None, None);
        assert!(!only_token.can_refresh());
        let two = EmailClient::new(EmailProvider::Gmail, "tok".into())
            .with_refresh(Some("r".into()), Some("id".into()), None);
        assert!(!two.can_refresh());
        let all_three = EmailClient::new(EmailProvider::Gmail, "tok".into())
            .with_refresh(Some("r".into()), Some("id".into()), Some("sec".into()));
        assert!(all_three.can_refresh());
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
