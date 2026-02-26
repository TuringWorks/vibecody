//! Messaging gateway for VibeCLI — runs as a 24/7 bot daemon.
//!
//! Supported platforms: Telegram, Discord, Slack
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
//! platform = "telegram"          # or "discord" / "slack"
//! telegram_token = "..."         # or TELEGRAM_BOT_TOKEN env var
//! discord_token = "..."          # or DISCORD_BOT_TOKEN env var
//! slack_bot_token = "..."        # or SLACK_BOT_TOKEN env var
//! slack_app_token = "..."        # or SLACK_APP_TOKEN env var (for Socket Mode)
//! allowed_users = ["@alice", "@bob"]   # optional whitelist
//! max_response_length = 4000     # truncate long agent responses
//! ```

use anyhow::Result;

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
                    eprintln!("[gateway] {} @{}: {}", msg.platform, msg.user, &msg.text[..msg.text.len().min(80)]);

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
}
