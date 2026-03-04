//! Messaging gateway for VibeCLI — runs as a 24/7 bot daemon.
//!
//! Supported platforms: Telegram, Discord, Slack, Signal, Matrix,
//! Twilio SMS, WhatsApp, iMessage (macOS), Microsoft Teams
//!
//! Start with: `vibecli --gateway telegram`
//!
//! The gateway bridges chat messages to the VibeCLI agent loop.
//! Each message is processed as an agent task; the result is sent
//! back as a reply in the chat platform.
//!
//! Configuration via environment variables or ~/.vibecli/config.toml:
//!
//! ```toml
//! [gateway]
//! platform = "telegram"          # or "discord" / "slack" / "signal" / "matrix" / "twilio" / "whatsapp" / "imessage" / "teams"
//! telegram_token = "..."         # or TELEGRAM_BOT_TOKEN env var
//! discord_token = "..."          # or DISCORD_BOT_TOKEN env var
//! slack_bot_token = "..."        # or SLACK_BOT_TOKEN env var
//! slack_app_token = "..."        # or SLACK_APP_TOKEN env var (for Socket Mode)
//! signal_api_url = "..."         # or SIGNAL_API_URL env var
//! signal_phone_number = "+1..."  # or SIGNAL_PHONE_NUMBER env var
//! matrix_homeserver_url = "..."  # or MATRIX_HOMESERVER_URL env var
//! matrix_access_token = "..."    # or MATRIX_ACCESS_TOKEN env var
//! matrix_room_id = "!abc:..."    # or MATRIX_ROOM_ID env var
//! matrix_user_id = "@bot:..."    # or MATRIX_USER_ID env var
//! twilio_account_sid = "AC..."   # or TWILIO_ACCOUNT_SID env var
//! twilio_auth_token = "..."      # or TWILIO_AUTH_TOKEN env var
//! twilio_from_number = "+1..."   # or TWILIO_FROM_NUMBER env var
//! whatsapp_access_token = "..."  # or WHATSAPP_ACCESS_TOKEN env var
//! whatsapp_phone_number_id = "." # or WHATSAPP_PHONE_NUMBER_ID env var
//! whatsapp_verify_token = "..."  # or WHATSAPP_VERIFY_TOKEN env var
//! imessage_db_path = "~/..."     # or IMESSAGE_DB_PATH env var (macOS only)
//! teams_tenant_id = "..."        # or TEAMS_TENANT_ID env var
//! teams_client_id = "..."        # or TEAMS_CLIENT_ID env var
//! teams_client_secret = "..."    # or TEAMS_CLIENT_SECRET env var
//! allowed_users = ["@alice", "@bob"]   # optional whitelist
//! max_response_length = 4000     # truncate long agent responses
//! ```

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// An incoming message from any gateway platform.
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub platform: String,
    pub chat_id: String,
    pub user: String,
    pub text: String,
    pub message_id: Option<String>,
}

/// A response to send back to the platform.
#[derive(Debug, Clone)]
pub struct GatewayResponse {
    pub chat_id: String,
    pub text: String,
    pub reply_to: Option<String>,
}

/// Trait that each platform adapter implements.
#[async_trait::async_trait]
pub trait GatewayPlatform: Send + Sync {
    /// Poll for new incoming messages.
    async fn poll(&mut self) -> Result<Vec<IncomingMessage>>;
    /// Send a response.
    async fn send(&self, response: GatewayResponse) -> Result<()>;
    /// Platform name for logging.
    fn name(&self) -> &str;
}

/// Telegram adapter — uses the Telegram Bot API (long-polling).
pub struct TelegramGateway {
    token: String,
    offset: i64,
    client: reqwest::Client,
    allowed_users: Vec<String>,
}

impl TelegramGateway {
    pub fn new(token: String, allowed_users: Vec<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(35))
            .user_agent("VibeCLI-Gateway/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { token, offset: 0, client, allowed_users }
    }

    fn base_url(&self) -> String {
        format!("https://api.telegram.org/bot{}", self.token)
    }
}

#[async_trait::async_trait]
impl GatewayPlatform for TelegramGateway {
    fn name(&self) -> &str { "telegram" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        let url = format!("{}/getUpdates?timeout=30&offset={}", self.base_url(), self.offset);
        let resp = self.client.get(&url).send().await?.json::<serde_json::Value>().await?;

        let mut messages = Vec::new();
        if let Some(updates) = resp["result"].as_array() {
            for update in updates {
                let update_id = update["update_id"].as_i64().unwrap_or(0);
                self.offset = update_id + 1;

                if let Some(msg) = update.get("message") {
                    let chat_id = msg["chat"]["id"].to_string();
                    let text = msg["text"].as_str().unwrap_or("").to_string();
                    let user = msg["from"]["username"].as_str()
                        .or_else(|| msg["from"]["first_name"].as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let message_id = msg["message_id"].as_i64().map(|id| id.to_string());

                    if text.is_empty() { continue; }

                    // Check whitelist
                    if !self.allowed_users.is_empty()
                        && !self.allowed_users.iter().any(|u| u == &user || u == &format!("@{}", user)) {
                        continue;
                    }

                    messages.push(IncomingMessage {
                        platform: "telegram".to_string(),
                        chat_id,
                        user,
                        text,
                        message_id,
                    });
                }
            }
        }
        Ok(messages)
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        let url = format!("{}/sendMessage", self.base_url());
        let mut payload = serde_json::json!({
            "chat_id": response.chat_id,
            "text": truncate_text(&response.text, 4096),
            "parse_mode": "Markdown",
        });
        if let Some(reply_id) = response.reply_to {
            payload["reply_to_message_id"] = serde_json::Value::String(reply_id);
        }
        self.client.post(&url).json(&payload).send().await?;
        Ok(())
    }
}

/// Discord adapter — uses the Discord HTTP API + Gateway WebSocket.
/// For simplicity, we use webhook-based sends and HTTP polling for messages
/// (a production implementation would use the real Discord Gateway WS protocol).
pub struct DiscordGateway {
    token: String,
    client: reqwest::Client,
    last_message_id: Option<String>,
    channel_id: String,
}

impl DiscordGateway {
    pub fn new(token: String, channel_id: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("VibeCLI-Gateway/1.0 (https://github.com/vibecody/vibecody)")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { token, client, last_message_id: None, channel_id }
    }
}

#[async_trait::async_trait]
impl GatewayPlatform for DiscordGateway {
    fn name(&self) -> &str { "discord" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        let mut url = format!(
            "https://discord.com/api/v10/channels/{}/messages?limit=5",
            self.channel_id
        );
        if let Some(after) = &self.last_message_id {
            url.push_str(&format!("&after={}", after));
        }

        let resp = self.client.get(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send().await?.json::<serde_json::Value>().await?;

        let mut messages = Vec::new();
        if let Some(msgs) = resp.as_array() {
            for msg in msgs.iter().rev() {
                // Skip bot messages
                if msg["author"]["bot"].as_bool().unwrap_or(false) { continue; }

                let id = msg["id"].as_str().unwrap_or("").to_string();
                let text = msg["content"].as_str().unwrap_or("").to_string();
                let user = msg["author"]["username"].as_str().unwrap_or("unknown").to_string();

                if text.is_empty() || id.is_empty() { continue; }

                self.last_message_id = Some(id.clone());
                messages.push(IncomingMessage {
                    platform: "discord".to_string(),
                    chat_id: self.channel_id.clone(),
                    user,
                    text,
                    message_id: Some(id),
                });
            }
        }
        Ok(messages)
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            self.channel_id
        );
        let payload = serde_json::json!({
            "content": truncate_text(&response.text, 2000),
        });
        self.client.post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&payload)
            .send().await?;
        Ok(())
    }
}

/// Slack adapter — uses the Slack Web API (RTM/Events via polling).
pub struct SlackGateway {
    bot_token: String,
    client: reqwest::Client,
    channel: String,
    last_ts: Option<String>,
}

impl SlackGateway {
    pub fn new(bot_token: String, channel: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("VibeCLI-Gateway/1.0")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { bot_token, client, channel, last_ts: None }
    }
}

#[async_trait::async_trait]
impl GatewayPlatform for SlackGateway {
    fn name(&self) -> &str { "slack" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        let mut url = format!(
            "https://slack.com/api/conversations.history?channel={}&limit=5",
            self.channel
        );
        if let Some(ts) = &self.last_ts {
            url.push_str(&format!("&oldest={}", ts));
        }
        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .send().await?.json::<serde_json::Value>().await?;

        let mut messages = Vec::new();
        if let Some(msgs) = resp["messages"].as_array() {
            for msg in msgs.iter().rev() {
                if msg["bot_id"].is_string() { continue; } // skip bot messages
                let ts = msg["ts"].as_str().unwrap_or("").to_string();
                let text = msg["text"].as_str().unwrap_or("").to_string();
                let user = msg["user"].as_str().unwrap_or("unknown").to_string();

                if text.is_empty() || ts.is_empty() { continue; }
                if Some(&ts) == self.last_ts.as_ref() { continue; }
                self.last_ts = Some(ts.clone());

                messages.push(IncomingMessage {
                    platform: "slack".to_string(),
                    chat_id: self.channel.clone(),
                    user,
                    text,
                    message_id: Some(ts),
                });
            }
        }
        Ok(messages)
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        let payload = serde_json::json!({
            "channel": self.channel,
            "text": truncate_text(&response.text, 40000),
        });
        self.client.post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&payload)
            .send().await?;
        Ok(())
    }
}

// ── Signal adapter ────────────────────────────────────────────────────────────
/// Signal adapter — uses the signal-cli REST API.
///
/// Requires signal-cli REST running (e.g. via Docker):
///   docker run -p 8080:8080 bbernhard/signal-cli-rest-api
///
/// `poll()` does a destructive read via `GET /v1/receive/{number}`.
/// `send()` posts to `POST /v2/send`.
pub struct SignalGateway {
    api_url: String,
    phone_number: String,
    client: reqwest::Client,
}

impl SignalGateway {
    pub fn new(api_url: String, phone_number: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("VibeCLI-Gateway/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { api_url, phone_number, client }
    }
}

#[async_trait::async_trait]
impl GatewayPlatform for SignalGateway {
    fn name(&self) -> &str { "signal" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        // signal-cli REST: GET /v1/receive/{number} returns messages and consumes them
        let url = format!("{}/v1/receive/{}", self.api_url, self.phone_number);
        let resp = self.client.get(&url).send().await?.json::<serde_json::Value>().await?;

        let mut messages = Vec::new();
        if let Some(entries) = resp.as_array() {
            for entry in entries {
                let envelope = &entry["envelope"];
                let data_msg = &envelope["dataMessage"];

                let text = data_msg["message"].as_str().unwrap_or("").to_string();
                if text.is_empty() { continue; }

                let source = envelope["source"].as_str().unwrap_or("unknown").to_string();
                let ts = data_msg["timestamp"].as_u64().unwrap_or(0).to_string();

                messages.push(IncomingMessage {
                    platform: "signal".to_string(),
                    chat_id: source.clone(),
                    user: source,
                    text,
                    message_id: Some(ts),
                });
            }
        }
        Ok(messages)
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        let url = format!("{}/v2/send", self.api_url);
        let payload = serde_json::json!({
            "message": truncate_text(&response.text, 30000),
            "number": self.phone_number,
            "recipients": [response.chat_id],
        });
        self.client.post(&url).json(&payload).send().await?;
        Ok(())
    }
}

// ── Matrix adapter ───────────────────────────────────────────────────────────
/// Matrix adapter — uses the Matrix Client-Server API with `/sync` long-polling.
pub struct MatrixGateway {
    homeserver: String,
    access_token: String,
    room_id: String,
    bot_user_id: String,
    client: reqwest::Client,
    since_token: Option<String>,
}

impl MatrixGateway {
    pub fn new(homeserver: String, access_token: String, room_id: String, bot_user_id: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(35))
            .user_agent("VibeCLI-Gateway/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { homeserver, access_token, room_id, bot_user_id, client, since_token: None }
    }
}

#[async_trait::async_trait]
impl GatewayPlatform for MatrixGateway {
    fn name(&self) -> &str { "matrix" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        let mut url = format!(
            "{}/_matrix/client/v3/sync?timeout=30000&filter={{\"room\":{{\"rooms\":[\"{}\"],\"timeline\":{{\"limit\":10}}}}}}",
            self.homeserver, self.room_id
        );
        if let Some(since) = &self.since_token {
            url.push_str(&format!("&since={}", since));
        }

        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send().await?.json::<serde_json::Value>().await?;

        // Update since token for next poll
        if let Some(next) = resp["next_batch"].as_str() {
            self.since_token = Some(next.to_string());
        }

        let mut messages = Vec::new();

        // Navigate: rooms → join → <room_id> → timeline → events
        if let Some(events) = resp["rooms"]["join"][&self.room_id]["timeline"]["events"].as_array() {
            for event in events {
                if event["type"].as_str() != Some("m.room.message") { continue; }

                let sender = event["sender"].as_str().unwrap_or("").to_string();
                // Skip our own messages
                if sender == self.bot_user_id { continue; }

                let body = event["content"]["body"].as_str().unwrap_or("").to_string();
                if body.is_empty() { continue; }

                let event_id = event["event_id"].as_str().unwrap_or("").to_string();

                messages.push(IncomingMessage {
                    platform: "matrix".to_string(),
                    chat_id: self.room_id.clone(),
                    user: sender,
                    text: body,
                    message_id: Some(event_id),
                });
            }
        }
        Ok(messages)
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        // Use PUT with a transaction ID to avoid duplicates
        let txn_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver,
            response.chat_id,
            txn_id
        );
        let payload = serde_json::json!({
            "msgtype": "m.text",
            "body": truncate_text(&response.text, 60000),
        });
        self.client.put(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&payload)
            .send().await?;
        Ok(())
    }
}

// ── Twilio SMS adapter ───────────────────────────────────────────────────────
/// Twilio SMS adapter — polls for inbound messages and sends via the Messages API.
pub struct TwilioGateway {
    account_sid: String,
    auth_token: String,
    from_number: String,
    client: reqwest::Client,
    last_message_sid: Option<String>,
}

impl TwilioGateway {
    pub fn new(account_sid: String, auth_token: String, from_number: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("VibeCLI-Gateway/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { account_sid, auth_token, from_number, client, last_message_sid: None }
    }
}

#[async_trait::async_trait]
impl GatewayPlatform for TwilioGateway {
    fn name(&self) -> &str { "twilio" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        // Fetch recent inbound messages
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json?To={}&PageSize=5",
            self.account_sid, self.from_number
        );
        let resp = self.client.get(&url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .send().await?.json::<serde_json::Value>().await?;

        let mut messages = Vec::new();
        if let Some(msgs) = resp["messages"].as_array() {
            let mut found_last = self.last_message_sid.is_none();
            // Messages come newest-first; collect and reverse for chronological order
            let mut batch: Vec<IncomingMessage> = Vec::new();
            for msg in msgs {
                let sid = msg["sid"].as_str().unwrap_or("").to_string();
                let direction = msg["direction"].as_str().unwrap_or("");
                if direction != "inbound" { continue; }

                // Skip messages we've already seen
                if !found_last {
                    if Some(&sid) == self.last_message_sid.as_ref() {
                        found_last = true;
                    }
                    continue;
                }

                let body = msg["body"].as_str().unwrap_or("").to_string();
                let from = msg["from"].as_str().unwrap_or("unknown").to_string();
                if body.is_empty() { continue; }

                self.last_message_sid = Some(sid.clone());
                batch.push(IncomingMessage {
                    platform: "twilio".to_string(),
                    chat_id: from.clone(),
                    user: from,
                    text: body,
                    message_id: Some(sid),
                });
            }

            // If bookmark SID was not found in this batch (stale/rotated off page),
            // treat all inbound messages as new to avoid silently dropping them.
            if !found_last && self.last_message_sid.is_some() {
                batch.clear();
                for msg in msgs {
                    let sid = msg["sid"].as_str().unwrap_or("").to_string();
                    let direction = msg["direction"].as_str().unwrap_or("");
                    if direction != "inbound" { continue; }
                    let body = msg["body"].as_str().unwrap_or("").to_string();
                    let from = msg["from"].as_str().unwrap_or("unknown").to_string();
                    if body.is_empty() { continue; }
                    self.last_message_sid = Some(sid.clone());
                    batch.push(IncomingMessage {
                        platform: "twilio".to_string(),
                        chat_id: from.clone(),
                        user: from,
                        text: body,
                        message_id: Some(sid),
                    });
                }
            }

            batch.reverse();
            messages.extend(batch);
        }
        Ok(messages)
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        );
        let params = [
            ("To", response.chat_id.as_str()),
            ("From", self.from_number.as_str()),
            ("Body", &truncate_text(&response.text, 1600)),
        ];
        self.client.post(&url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&params)
            .send().await?;
        Ok(())
    }
}

// ── iMessage adapter (macOS only) ────────────────────────────────────────────
/// iMessage adapter — reads from `~/Library/Messages/chat.db` and sends via AppleScript.
///
/// Requires Full Disk Access on macOS for chat.db access.
#[cfg(target_os = "macos")]
pub struct IMessageGateway {
    db_path: String,
    last_rowid: i64,
}

#[cfg(target_os = "macos")]
impl IMessageGateway {
    pub fn new(db_path: Option<String>) -> Self {
        let path = db_path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{}/Library/Messages/chat.db", home)
        });
        // Get the current max ROWID so we only see new messages
        let last_rowid = Self::max_rowid(&path).unwrap_or(0);
        Self { db_path: path, last_rowid }
    }

    fn max_rowid(db_path: &str) -> Option<i64> {
        let conn = rusqlite::Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ).ok()?;
        conn.query_row("SELECT MAX(ROWID) FROM message", [], |row| row.get(0)).ok()
    }

    /// Escape text for AppleScript string literals.
    fn escape_applescript(text: &str) -> String {
        text.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

#[cfg(target_os = "macos")]
#[async_trait::async_trait]
impl GatewayPlatform for IMessageGateway {
    fn name(&self) -> &str { "imessage" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        let conn = rusqlite::Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        let mut stmt = conn.prepare(
            "SELECT m.ROWID, m.text, h.id, m.is_from_me
             FROM message m
             LEFT JOIN handle h ON m.handle_id = h.ROWID
             WHERE m.ROWID > ?1 AND m.is_from_me = 0 AND m.text IS NOT NULL
             ORDER BY m.ROWID ASC
             LIMIT 10"
        )?;

        let mut messages = Vec::new();
        let rows = stmt.query_map([self.last_rowid], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        for row in rows {
            let (rowid, text, handle_id) = row?;
            self.last_rowid = rowid; // always advance past this row
            if text.is_empty() { continue; }

            messages.push(IncomingMessage {
                platform: "imessage".to_string(),
                chat_id: handle_id.clone(),
                user: handle_id,
                text,
                message_id: Some(rowid.to_string()),
            });
        }
        Ok(messages)
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        let escaped = Self::escape_applescript(&truncate_text(&response.text, 10000));
        let script = format!(
            r#"tell application "Messages"
    set targetService to 1st account whose service type = iMessage
    set targetBuddy to participant "{}" of targetService
    send "{}" to targetBuddy
end tell"#,
            Self::escape_applescript(&response.chat_id),
            escaped
        );

        tokio::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .await?;
        Ok(())
    }
}

// ── WhatsApp adapter (Meta Cloud API) ────────────────────────────────────────
/// WhatsApp adapter — receives messages via a webhook and sends via the Cloud API.
///
/// Spawns a lightweight Axum HTTP server to receive webhook events from Meta.
/// Messages are buffered in an `Arc<Mutex<Vec>>` and drained on `poll()`.
pub struct WhatsAppGateway {
    access_token: String,
    phone_number_id: String,
    client: reqwest::Client,
    buffer: Arc<Mutex<Vec<IncomingMessage>>>,
}

impl WhatsAppGateway {
    pub async fn new(
        access_token: String,
        phone_number_id: String,
        verify_token: String,
        port: u16,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("VibeCLI-Gateway/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let buffer: Arc<Mutex<Vec<IncomingMessage>>> = Arc::new(Mutex::new(Vec::new()));

        // Spawn webhook receiver
        let buf_clone = buffer.clone();
        let vt = verify_token.clone();
        tokio::spawn(async move {
            Self::run_webhook(port, vt, buf_clone).await;
        });

        Self { access_token, phone_number_id, client, buffer }
    }

    async fn run_webhook(port: u16, verify_token: String, buffer: Arc<Mutex<Vec<IncomingMessage>>>) {
        use axum::{Router, extract::Query, routing::get, routing::post};

        let vt = verify_token.clone();
        let verify_handler = move |Query(params): Query<std::collections::HashMap<String, String>>| async move {
            if params.get("hub.verify_token").map(|s| s.as_str()) == Some(&vt) {
                params.get("hub.challenge").cloned().unwrap_or_default()
            } else {
                "invalid".to_string()
            }
        };

        let buf = buffer.clone();
        let post_handler = move |axum::extract::Json(body): axum::extract::Json<serde_json::Value>| async move {
            // Parse WhatsApp webhook payload
            if let Some(entries) = body["entry"].as_array() {
                for entry in entries {
                    if let Some(changes) = entry["changes"].as_array() {
                        for change in changes {
                            if let Some(msgs) = change["value"]["messages"].as_array() {
                                for msg in msgs {
                                    let text = msg["text"]["body"].as_str().unwrap_or("").to_string();
                                    let from = msg["from"].as_str().unwrap_or("").to_string();
                                    let msg_id = msg["id"].as_str().unwrap_or("").to_string();
                                    if text.is_empty() || from.is_empty() { continue; }

                                    buf.lock().await.push(IncomingMessage {
                                        platform: "whatsapp".to_string(),
                                        chat_id: from.clone(),
                                        user: from,
                                        text,
                                        message_id: Some(msg_id),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "OK".to_string()
        };

        let app = Router::new()
            .route("/webhook", get(verify_handler))
            .route("/webhook", post(post_handler));

        let addr: std::net::SocketAddr = ([0, 0, 0, 0], port).into();
        eprintln!("[whatsapp] Webhook listening on :{}", port);
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[whatsapp] Failed to bind port {}: {}", port, e);
                return;
            }
        };
        let _ = axum::serve(listener, app).await;
    }
}

#[async_trait::async_trait]
impl GatewayPlatform for WhatsAppGateway {
    fn name(&self) -> &str { "whatsapp" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        let mut buf = self.buffer.lock().await;
        let msgs = buf.drain(..).collect();
        Ok(msgs)
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            self.phone_number_id
        );
        let payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": response.chat_id,
            "type": "text",
            "text": {
                "body": truncate_text(&response.text, 4096)
            }
        });
        self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&payload)
            .send().await?;
        Ok(())
    }
}

// ── Microsoft Teams adapter ──────────────────────────────────────────────────
/// Microsoft Teams adapter — uses the Bot Framework protocol.
///
/// Receives activities via an Axum webhook on `teams_webhook_port` (default 3978).
/// Sends replies via the Bot Framework Activity API using OAuth2 client credentials.
pub struct TeamsGateway {
    client_id: String,
    client_secret: String,
    tenant_id: String,
    client: reqwest::Client,
    buffer: Arc<Mutex<Vec<(IncomingMessage, String)>>>, // (msg, service_url)
    token_cache: Arc<Mutex<Option<(String, std::time::Instant)>>>,
}

impl TeamsGateway {
    pub async fn new(tenant_id: String, client_id: String, client_secret: String, port: u16) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("VibeCLI-Gateway/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let buffer: Arc<Mutex<Vec<(IncomingMessage, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let token_cache: Arc<Mutex<Option<(String, std::time::Instant)>>> = Arc::new(Mutex::new(None));

        // Spawn webhook receiver
        let buf_clone = buffer.clone();
        tokio::spawn(async move {
            Self::run_webhook(port, buf_clone).await;
        });

        Self { client_id, client_secret, tenant_id, client, buffer, token_cache }
    }

    async fn run_webhook(port: u16, buffer: Arc<Mutex<Vec<(IncomingMessage, String)>>>) {
        use axum::{Router, routing::post};

        let buf = buffer.clone();
        let handler = move |axum::extract::Json(activity): axum::extract::Json<serde_json::Value>| {
            let buf = buf.clone();
            async move {
                let activity_type = activity["type"].as_str().unwrap_or("");
                if activity_type != "message" { return "OK".to_string(); }

                let text = activity["text"].as_str().unwrap_or("").to_string();
                let from_name = activity["from"]["name"].as_str().unwrap_or("unknown").to_string();
                let conversation_id = activity["conversation"]["id"].as_str().unwrap_or("").to_string();
                let activity_id = activity["id"].as_str().unwrap_or("").to_string();
                let service_url = activity["serviceUrl"].as_str().unwrap_or("").to_string();

                if text.is_empty() || conversation_id.is_empty() { return "OK".to_string(); }

                buf.lock().await.push((
                    IncomingMessage {
                        platform: "teams".to_string(),
                        chat_id: conversation_id,
                        user: from_name,
                        text,
                        message_id: Some(activity_id),
                    },
                    service_url,
                ));
                "OK".to_string()
            }
        };

        let app = Router::new().route("/api/messages", post(handler));
        let addr: std::net::SocketAddr = ([0, 0, 0, 0], port).into();
        eprintln!("[teams] Bot Framework webhook on :{}", port);
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[teams] Failed to bind port {}: {}", port, e);
                return;
            }
        };
        let _ = axum::serve(listener, app).await;
    }

    async fn get_access_token(&self) -> Result<String> {
        // Check cache (tokens last ~3600s; refresh at 3000s)
        {
            let cache = self.token_cache.lock().await;
            if let Some((token, acquired)) = cache.as_ref() {
                if acquired.elapsed().as_secs() < 3000 {
                    return Ok(token.clone());
                }
            }
        }

        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        );
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("scope", "https://api.botframework.com/.default"),
        ];
        let resp = self.client.post(&url).form(&params).send().await?
            .json::<serde_json::Value>().await?;

        let token = resp["access_token"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Teams OAuth2 failed: no access_token in response"))?
            .to_string();

        let mut cache = self.token_cache.lock().await;
        *cache = Some((token.clone(), std::time::Instant::now()));
        Ok(token)
    }
}

#[async_trait::async_trait]
impl GatewayPlatform for TeamsGateway {
    fn name(&self) -> &str { "teams" }

    async fn poll(&mut self) -> Result<Vec<IncomingMessage>> {
        let mut buf = self.buffer.lock().await;
        let pairs: Vec<_> = buf.drain(..).collect();
        // We store (IncomingMessage, service_url) but poll() returns just IncomingMessage.
        // The service_url is needed for send() — we store it in a side map.
        // For simplicity, we embed it in the message_id as "activity_id|service_url".
        Ok(pairs.into_iter().map(|(mut msg, svc_url)| {
            if let Some(aid) = &msg.message_id {
                msg.message_id = Some(format!("{}|{}", aid, svc_url));
            }
            msg
        }).collect())
    }

    async fn send(&self, response: GatewayResponse) -> Result<()> {
        // Extract service_url from message_id (packed by poll())
        let (_, service_url) = response.reply_to.as_deref()
            .and_then(|s| s.split_once('|'))
            .unwrap_or(("", "https://smba.trafficmanager.net/teams/"));

        let token = self.get_access_token().await?;
        let url = format!(
            "{}v3/conversations/{}/activities",
            if service_url.ends_with('/') { service_url.to_string() } else { format!("{}/", service_url) },
            response.chat_id
        );
        let payload = serde_json::json!({
            "type": "message",
            "text": truncate_text(&response.text, 28000),
        });
        self.client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send().await?;
        Ok(())
    }
}

/// Run the gateway loop.
///
/// `llm` is the AI provider used to respond.
/// `gateway` is the platform adapter.
///
/// This function runs indefinitely, polling for messages every ~2 seconds,
/// running the agent for each message, and sending the response back.
pub async fn run_gateway(
    mut gateway: Box<dyn GatewayPlatform>,
    llm: std::sync::Arc<dyn vibe_ai::provider::AIProvider>,
) -> Result<()> {
    use vibe_ai::provider::{Message, MessageRole};

    eprintln!("[gateway] Starting {} gateway", gateway.name());

    loop {
        match gateway.poll().await {
            Ok(incoming) => {
                for msg in incoming {
                    let text_end = msg.text.char_indices().nth(80).map(|(i,_)| i).unwrap_or(msg.text.len());
                    eprintln!("[gateway] {} @{}: {}", msg.platform, msg.user, &msg.text[..text_end]);

                    // Simple direct LLM response (non-agent for speed)
                    let messages = vec![
                        Message { role: MessageRole::System, content: "You are VibeCLI, an AI coding assistant running as a bot. Be concise and helpful.".to_string() },
                        Message { role: MessageRole::User, content: msg.text.clone() },
                    ];

                    let response_text = match llm.chat(&messages, None).await {
                        Ok(text) => text,
                        Err(e) => format!("❌ Error: {}", e),
                    };

                    let chat_id = msg.chat_id.clone();
                    let reply_to = msg.message_id.clone();
                    let _ = gateway.send(GatewayResponse {
                        chat_id,
                        text: response_text,
                        reply_to,
                    }).await;
                }
            }
            Err(e) => {
                eprintln!("[gateway] Poll error: {}", e);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

/// Truncate text to max_len bytes, appending "…" if truncated.
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    // "…" is 3 UTF-8 bytes; leave room for it.
    let ellipsis = "…";
    let mut cut = max_len.saturating_sub(ellipsis.len());
    // Walk back to a valid UTF-8 char boundary.
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}{}", &text[..cut], ellipsis)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_text_short() {
        assert_eq!(truncate_text("hello", 100), "hello");
    }

    #[test]
    fn truncate_text_long() {
        let long = "a".repeat(200);
        let truncated = truncate_text(&long, 100);
        assert!(truncated.len() <= 100);
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn incoming_message_fields() {
        let msg = IncomingMessage {
            platform: "telegram".to_string(),
            chat_id: "12345".to_string(),
            user: "alice".to_string(),
            text: "hello".to_string(),
            message_id: Some("1".to_string()),
        };
        assert_eq!(msg.platform, "telegram");
        assert_eq!(msg.user, "alice");
    }

    #[test]
    fn gateway_response_fields() {
        let resp = GatewayResponse {
            chat_id: "12345".to_string(),
            text: "World".to_string(),
            reply_to: None,
        };
        assert_eq!(resp.chat_id, "12345");
    }

    // ── Signal tests ──

    #[test]
    fn signal_gateway_constructor() {
        let gw = SignalGateway::new("http://localhost:8080".to_string(), "+15551234567".to_string());
        assert_eq!(gw.name(), "signal");
        assert_eq!(gw.api_url, "http://localhost:8080");
        assert_eq!(gw.phone_number, "+15551234567");
    }

    #[test]
    fn signal_truncation() {
        let text = "x".repeat(40000);
        let truncated = truncate_text(&text, 30000);
        assert!(truncated.len() <= 30000);
    }

    // ── Matrix tests ──

    #[test]
    fn matrix_gateway_constructor() {
        let gw = MatrixGateway::new(
            "https://matrix.org".to_string(),
            "syt_token".to_string(),
            "!room:matrix.org".to_string(),
            "@bot:matrix.org".to_string(),
        );
        assert_eq!(gw.name(), "matrix");
        assert_eq!(gw.homeserver, "https://matrix.org");
        assert!(gw.since_token.is_none());
    }

    #[test]
    fn matrix_truncation() {
        let text = "x".repeat(70000);
        let truncated = truncate_text(&text, 60000);
        assert!(truncated.len() <= 60000);
    }

    // ── Twilio tests ──

    #[test]
    fn twilio_gateway_constructor() {
        let gw = TwilioGateway::new(
            "AC1234567890".to_string(),
            "auth_token".to_string(),
            "+15559876543".to_string(),
        );
        assert_eq!(gw.name(), "twilio");
        assert_eq!(gw.from_number, "+15559876543");
        assert!(gw.last_message_sid.is_none());
    }

    #[test]
    fn twilio_sms_truncation() {
        let text = "x".repeat(2000);
        let truncated = truncate_text(&text, 1600);
        assert!(truncated.len() <= 1600);
    }

    // ── iMessage tests (macOS only) ──

    #[cfg(target_os = "macos")]
    #[test]
    fn imessage_applescript_escaping() {
        assert_eq!(
            IMessageGateway::escape_applescript(r#"hello "world" \ test"#),
            r#"hello \"world\" \\ test"#
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn imessage_truncation() {
        let text = "x".repeat(12000);
        let truncated = truncate_text(&text, 10000);
        assert!(truncated.len() <= 10000);
    }

    // ── Teams tests ──

    #[test]
    fn teams_service_url_packing() {
        // Verify the service_url is packed into message_id with pipe separator
        let msg = IncomingMessage {
            platform: "teams".to_string(),
            chat_id: "conv_123".to_string(),
            user: "alice".to_string(),
            text: "hello".to_string(),
            message_id: Some("act_1".to_string()),
        };
        // Simulate what poll() does
        let packed = format!("{}|{}", msg.message_id.unwrap(), "https://smba.trafficmanager.net/teams/");
        let (activity_id, service_url) = packed.split_once('|').unwrap();
        assert_eq!(activity_id, "act_1");
        assert!(service_url.starts_with("https://"));
    }

    #[test]
    fn teams_truncation() {
        let text = "x".repeat(30000);
        let truncated = truncate_text(&text, 28000);
        assert!(truncated.len() <= 28000);
    }
}
