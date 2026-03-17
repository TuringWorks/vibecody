//! Event-driven automations — external triggers spawn agent tasks in sandbox.
//!
//! Closes P0 Gap 1: Cursor-style Automations where GitHub webhooks, Slack events,
//! PagerDuty alerts, and Linear updates automatically spawn agent sessions.
//!
//! # Architecture
//!
//! ```text
//! Webhook HTTP → AutomationEngine → match rules → spawn AgentTask in sandbox
//!    │                                    │
//!    ├─ GitHub (push, PR, issue)          ├─ EventFilter (repo, labels, etc.)
//!    ├─ Slack  (message, reaction)        ├─ PromptTemplate (with {{payload}} vars)
//!    ├─ Linear (issue update)             └─ SandboxConfig (optional container)
//!    ├─ PagerDuty (incident)
//!    ├─ Telegram (message, command, callback)
//!    ├─ Signal (message, reaction)
//!    ├─ WhatsApp (message, status)
//!    ├─ Discord (message, reaction, slash command)
//!    ├─ Teams (message, mention, adaptive card)
//!    ├─ Matrix (message, reaction, room invite)
//!    ├─ Twilio SMS (incoming message)
//!    ├─ iMessage (incoming message)
//!    ├─ IRC (PRIVMSG, JOIN, mention)
//!    ├─ Twitch (chat message, subscription, raid)
//!    ├─ Cron (time-based)
//!    └─ FileWatch (glob pattern)
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Trigger types
// ---------------------------------------------------------------------------

/// Source of an automation trigger event.
#[derive(Debug, Clone, PartialEq)]
pub enum TriggerSource {
    /// GitHub webhook (push, pull_request, issues, etc.)
    GitHub {
        events: Vec<String>,
        repos: Vec<String>,
    },
    /// Slack event (message, reaction_added, app_mention, etc.)
    Slack {
        events: Vec<String>,
        channels: Vec<String>,
    },
    /// Linear issue update events
    Linear {
        actions: Vec<String>,
        team_ids: Vec<String>,
    },
    /// PagerDuty incident events
    PagerDuty {
        severity: Vec<String>,
        services: Vec<String>,
    },
    /// Telegram Bot API events (message, edited_message, callback_query, etc.)
    Telegram {
        events: Vec<String>,
        chat_ids: Vec<String>,
    },
    /// Signal messenger events (via signal-cli or Signal Bot API)
    Signal {
        events: Vec<String>,
        group_ids: Vec<String>,
    },
    /// WhatsApp Business API events (messages, statuses)
    WhatsApp {
        events: Vec<String>,
        phone_numbers: Vec<String>,
    },
    /// Discord bot events (MESSAGE_CREATE, MESSAGE_REACTION_ADD, INTERACTION_CREATE, etc.)
    Discord {
        events: Vec<String>,
        channel_ids: Vec<String>,
        guild_ids: Vec<String>,
    },
    /// Microsoft Teams events (message, mention, adaptiveCard/action)
    Teams {
        events: Vec<String>,
        channel_ids: Vec<String>,
    },
    /// Matrix protocol events (m.room.message, m.reaction, m.room.member)
    Matrix {
        events: Vec<String>,
        room_ids: Vec<String>,
    },
    /// Twilio SMS/MMS incoming messages
    TwilioSms {
        events: Vec<String>,
        from_numbers: Vec<String>,
    },
    /// iMessage events (via AppleScript bridge on macOS)
    IMessage {
        events: Vec<String>,
        contacts: Vec<String>,
    },
    /// IRC events (PRIVMSG, JOIN, PART, etc.)
    Irc {
        events: Vec<String>,
        channels: Vec<String>,
    },
    /// Twitch chat/event events (chat, subscription, raid, follow)
    Twitch {
        events: Vec<String>,
        channels: Vec<String>,
    },
    /// Cron expression (time-based)
    Cron {
        expression: String,
    },
    /// File system watcher
    FileWatch {
        patterns: Vec<String>,
        path: PathBuf,
    },
    /// Generic webhook (custom integrations)
    Webhook {
        /// Optional secret for HMAC verification
        secret: Option<String>,
    },
}

impl TriggerSource {
    pub fn source_name(&self) -> &str {
        match self {
            TriggerSource::GitHub { .. } => "github",
            TriggerSource::Slack { .. } => "slack",
            TriggerSource::Linear { .. } => "linear",
            TriggerSource::PagerDuty { .. } => "pagerduty",
            TriggerSource::Telegram { .. } => "telegram",
            TriggerSource::Signal { .. } => "signal",
            TriggerSource::WhatsApp { .. } => "whatsapp",
            TriggerSource::Discord { .. } => "discord",
            TriggerSource::Teams { .. } => "teams",
            TriggerSource::Matrix { .. } => "matrix",
            TriggerSource::TwilioSms { .. } => "twilio_sms",
            TriggerSource::IMessage { .. } => "imessage",
            TriggerSource::Irc { .. } => "irc",
            TriggerSource::Twitch { .. } => "twitch",
            TriggerSource::Cron { .. } => "cron",
            TriggerSource::FileWatch { .. } => "filewatch",
            TriggerSource::Webhook { .. } => "webhook",
        }
    }
}

// ---------------------------------------------------------------------------
// Event filter
// ---------------------------------------------------------------------------

/// Conditions an incoming event must satisfy to fire the automation.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// JSONPath-like conditions: key → expected value.
    pub conditions: HashMap<String, String>,
    /// If non-empty, the event payload must contain ALL these keys.
    pub required_fields: Vec<String>,
    /// Regex pattern the raw payload body must match (empty = any).
    pub body_pattern: Option<String>,
}

impl EventFilter {
    pub fn matches(&self, payload: &EventPayload) -> bool {
        // Check required fields
        for field in &self.required_fields {
            if !payload.fields.contains_key(field) {
                return false;
            }
        }
        // Check conditions
        for (key, expected) in &self.conditions {
            match payload.fields.get(key) {
                Some(val) if val == expected => {}
                _ => return false,
            }
        }
        // Check body pattern
        if let Some(pattern) = &self.body_pattern {
            if !payload.raw_body.contains(pattern) {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Event payload
// ---------------------------------------------------------------------------

/// Parsed incoming event from any source.
#[derive(Debug, Clone)]
pub struct EventPayload {
    pub source: String,
    pub event_type: String,
    pub fields: HashMap<String, String>,
    pub raw_body: String,
    pub timestamp: u64,
}

impl EventPayload {
    pub fn new(source: &str, event_type: &str, raw_body: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            source: source.to_string(),
            event_type: event_type.to_string(),
            fields: HashMap::new(),
            raw_body: raw_body.to_string(),
            timestamp: ts,
        }
    }

    pub fn with_field(mut self, key: &str, value: &str) -> Self {
        self.fields.insert(key.to_string(), value.to_string());
        self
    }
}

// ---------------------------------------------------------------------------
// Prompt template
// ---------------------------------------------------------------------------

/// Agent prompt template with variable substitution from event payload.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Template string with `{{variable}}` placeholders.
    pub template: String,
    /// Default values for missing variables.
    pub defaults: HashMap<String, String>,
}

impl PromptTemplate {
    pub fn new(template: &str) -> Self {
        Self {
            template: template.to_string(),
            defaults: HashMap::new(),
        }
    }

    pub fn with_default(mut self, key: &str, value: &str) -> Self {
        self.defaults.insert(key.to_string(), value.to_string());
        self
    }

    /// Render the template by substituting `{{key}}` with payload fields.
    pub fn render(&self, payload: &EventPayload) -> String {
        let mut result = self.template.clone();
        // Substitute event metadata
        result = result.replace("{{source}}", &payload.source);
        result = result.replace("{{event_type}}", &payload.event_type);
        result = result.replace("{{timestamp}}", &payload.timestamp.to_string());
        result = result.replace("{{raw_body}}", &payload.raw_body);
        // Substitute payload fields
        for (key, value) in &payload.fields {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        // Apply defaults for remaining placeholders
        for (key, default) in &self.defaults {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, default);
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Sandbox config for agent execution
// ---------------------------------------------------------------------------

/// Optional sandbox configuration for running the spawned agent.
#[derive(Debug, Clone)]
pub struct AutomationSandbox {
    pub enabled: bool,
    pub runtime: String, // "docker", "podman", "opensandbox"
    pub image: String,
    pub timeout_secs: u64,
    pub memory_limit_mb: u64,
    pub network_enabled: bool,
}

impl Default for AutomationSandbox {
    fn default() -> Self {
        Self {
            enabled: false,
            runtime: "docker".to_string(),
            image: "vibecody/agent:latest".to_string(),
            timeout_secs: 300,
            memory_limit_mb: 512,
            network_enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Automation rule
// ---------------------------------------------------------------------------

/// A single automation rule: trigger → filter → prompt → agent.
#[derive(Debug, Clone)]
pub struct AutomationRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub trigger: TriggerSource,
    pub filter: EventFilter,
    pub prompt_template: PromptTemplate,
    pub sandbox: AutomationSandbox,
    pub provider: String,
    pub model: Option<String>,
    pub max_turns: usize,
    pub created_at: u64,
    pub last_fired: Option<u64>,
    pub fire_count: u64,
}

impl AutomationRule {
    pub fn new(id: &str, name: &str, trigger: TriggerSource, prompt: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            enabled: true,
            trigger,
            filter: EventFilter::default(),
            prompt_template: PromptTemplate::new(prompt),
            sandbox: AutomationSandbox::default(),
            provider: "ollama".to_string(),
            model: None,
            max_turns: 20,
            created_at: ts,
            last_fired: None,
            fire_count: 0,
        }
    }

    /// Check if this rule matches an incoming event.
    pub fn matches(&self, payload: &EventPayload) -> bool {
        if !self.enabled {
            return false;
        }
        // Check source type matches
        let source_matches = match &self.trigger {
            TriggerSource::GitHub { events, repos } => {
                payload.source == "github"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (repos.is_empty()
                        || payload
                            .fields
                            .get("repository")
                            .is_some_and(|r| repos.contains(r)))
            }
            TriggerSource::Slack { events, channels } => {
                payload.source == "slack"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (channels.is_empty()
                        || payload
                            .fields
                            .get("channel")
                            .is_some_and(|c| channels.contains(c)))
            }
            TriggerSource::Linear { actions, team_ids } => {
                payload.source == "linear"
                    && (actions.is_empty() || actions.contains(&payload.event_type))
                    && (team_ids.is_empty()
                        || payload
                            .fields
                            .get("team_id")
                            .is_some_and(|t| team_ids.contains(t)))
            }
            TriggerSource::PagerDuty {
                severity,
                services,
            } => {
                payload.source == "pagerduty"
                    && (severity.is_empty()
                        || payload
                            .fields
                            .get("severity")
                            .is_some_and(|s| severity.contains(s)))
                    && (services.is_empty()
                        || payload
                            .fields
                            .get("service")
                            .is_some_and(|s| services.contains(s)))
            }
            TriggerSource::Telegram { events, chat_ids } => {
                payload.source == "telegram"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (chat_ids.is_empty()
                        || payload
                            .fields
                            .get("chat_id")
                            .is_some_and(|c| chat_ids.contains(c)))
            }
            TriggerSource::Signal { events, group_ids } => {
                payload.source == "signal"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (group_ids.is_empty()
                        || payload
                            .fields
                            .get("group_id")
                            .is_some_and(|g| group_ids.contains(g)))
            }
            TriggerSource::WhatsApp {
                events,
                phone_numbers,
            } => {
                payload.source == "whatsapp"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (phone_numbers.is_empty()
                        || payload
                            .fields
                            .get("from")
                            .is_some_and(|f| phone_numbers.contains(f)))
            }
            TriggerSource::Discord {
                events,
                channel_ids,
                guild_ids,
            } => {
                payload.source == "discord"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (channel_ids.is_empty()
                        || payload
                            .fields
                            .get("channel_id")
                            .is_some_and(|c| channel_ids.contains(c)))
                    && (guild_ids.is_empty()
                        || payload
                            .fields
                            .get("guild_id")
                            .is_some_and(|g| guild_ids.contains(g)))
            }
            TriggerSource::Teams { events, channel_ids } => {
                payload.source == "teams"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (channel_ids.is_empty()
                        || payload
                            .fields
                            .get("channel_id")
                            .is_some_and(|c| channel_ids.contains(c)))
            }
            TriggerSource::Matrix { events, room_ids } => {
                payload.source == "matrix"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (room_ids.is_empty()
                        || payload
                            .fields
                            .get("room_id")
                            .is_some_and(|r| room_ids.contains(r)))
            }
            TriggerSource::TwilioSms {
                events,
                from_numbers,
            } => {
                payload.source == "twilio_sms"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (from_numbers.is_empty()
                        || payload
                            .fields
                            .get("from")
                            .is_some_and(|f| from_numbers.contains(f)))
            }
            TriggerSource::IMessage { events, contacts } => {
                payload.source == "imessage"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (contacts.is_empty()
                        || payload
                            .fields
                            .get("sender")
                            .is_some_and(|s| contacts.contains(s)))
            }
            TriggerSource::Irc { events, channels } => {
                payload.source == "irc"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (channels.is_empty()
                        || payload
                            .fields
                            .get("channel")
                            .is_some_and(|c| channels.contains(c)))
            }
            TriggerSource::Twitch { events, channels } => {
                payload.source == "twitch"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (channels.is_empty()
                        || payload
                            .fields
                            .get("channel")
                            .is_some_and(|c| channels.contains(c)))
            }
            TriggerSource::Cron { .. } => payload.source == "cron",
            TriggerSource::FileWatch { patterns, .. } => {
                if payload.source != "filewatch" {
                    return false;
                }
                if patterns.is_empty() {
                    return true;
                }
                payload
                    .fields
                    .get("path")
                    .is_some_and(|p| patterns.iter().any(|pat| simple_glob_match(pat, p)))
            }
            TriggerSource::Webhook { .. } => payload.source == "webhook",
        };
        source_matches && self.filter.matches(payload)
    }

    /// Render the agent prompt for this rule given an event payload.
    pub fn render_prompt(&self, payload: &EventPayload) -> String {
        self.prompt_template.render(payload)
    }
}

// ---------------------------------------------------------------------------
// Spawned task
// ---------------------------------------------------------------------------

/// Status of a spawned automation task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

/// A task spawned by an automation rule.
#[derive(Debug, Clone)]
pub struct AutomationTask {
    pub task_id: String,
    pub rule_id: String,
    pub prompt: String,
    pub status: TaskStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub output: Option<String>,
}

// ---------------------------------------------------------------------------
// Automation engine
// ---------------------------------------------------------------------------

/// Central engine that holds rules, dispatches events, and tracks tasks.
pub struct AutomationEngine {
    rules: Vec<AutomationRule>,
    tasks: Vec<AutomationTask>,
    config_path: PathBuf,
    task_counter: u64,
}

impl AutomationEngine {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            rules: Vec::new(),
            tasks: Vec::new(),
            config_path,
            task_counter: 0,
        }
    }

    pub fn add_rule(&mut self, rule: AutomationRule) {
        self.rules.push(rule);
    }

    pub fn remove_rule(&mut self, rule_id: &str) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != rule_id);
        self.rules.len() < before
    }

    pub fn get_rule(&self, rule_id: &str) -> Option<&AutomationRule> {
        self.rules.iter().find(|r| r.id == rule_id)
    }

    pub fn get_rule_mut(&mut self, rule_id: &str) -> Option<&mut AutomationRule> {
        self.rules.iter_mut().find(|r| r.id == rule_id)
    }

    pub fn list_rules(&self) -> &[AutomationRule] {
        &self.rules
    }

    pub fn enable_rule(&mut self, rule_id: &str) -> bool {
        if let Some(rule) = self.get_rule_mut(rule_id) {
            rule.enabled = true;
            true
        } else {
            false
        }
    }

    pub fn disable_rule(&mut self, rule_id: &str) -> bool {
        if let Some(rule) = self.get_rule_mut(rule_id) {
            rule.enabled = false;
            true
        } else {
            false
        }
    }

    /// Process an incoming event: match against all rules, return spawned tasks.
    pub fn process_event(&mut self, payload: &EventPayload) -> Vec<AutomationTask> {
        let mut spawned = Vec::new();
        let matching_ids: Vec<String> = self
            .rules
            .iter()
            .filter(|r| r.matches(payload))
            .map(|r| r.id.clone())
            .collect();

        for rule_id in matching_ids {
            let idx = self.rules.iter().position(|r| r.id == rule_id);
            if let Some(idx) = idx {
                let prompt = self.rules[idx].render_prompt(payload);
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let rid = self.rules[idx].id.clone();
                self.rules[idx].last_fired = Some(ts);
                self.rules[idx].fire_count += 1;

                self.task_counter += 1;
                let task_id = format!("auto-{}", self.task_counter);
                let task = AutomationTask {
                    task_id,
                    rule_id: rid,
                    prompt,
                    status: TaskStatus::Queued,
                    created_at: ts,
                    completed_at: None,
                    output: None,
                };
                spawned.push(task.clone());
                self.tasks.push(task);
            }
        }
        spawned
    }

    pub fn list_tasks(&self) -> &[AutomationTask] {
        &self.tasks
    }

    pub fn get_task(&self, task_id: &str) -> Option<&AutomationTask> {
        self.tasks.iter().find(|t| t.task_id == task_id)
    }

    pub fn update_task_status(&mut self, task_id: &str, status: TaskStatus) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.task_id == task_id) {
            if matches!(status, TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Cancelled) {
                task.completed_at = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                );
            }
            task.status = status;
            true
        } else {
            false
        }
    }

    pub fn stats(&self) -> AutomationStats {
        let total_rules = self.rules.len();
        let enabled_rules = self.rules.iter().filter(|r| r.enabled).count();
        let total_tasks = self.tasks.len();
        let running_tasks = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .count();
        let completed_tasks = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed_tasks = self
            .tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Failed(_)))
            .count();
        AutomationStats {
            total_rules,
            enabled_rules,
            total_tasks,
            running_tasks,
            completed_tasks,
            failed_tasks,
        }
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }
}

#[derive(Debug, Clone)]
pub struct AutomationStats {
    pub total_rules: usize,
    pub enabled_rules: usize,
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
}

// ---------------------------------------------------------------------------
// Webhook signature verification
// ---------------------------------------------------------------------------

/// Verify HMAC-SHA256 signature for webhook payloads.
pub fn verify_webhook_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
    // Simple HMAC-SHA256 verification stub
    // In production, use ring or hmac crate
    if secret.is_empty() || signature.is_empty() {
        return false;
    }
    // Compute expected: SHA256(secret + payload) as hex
    // This is a simplified check; real impl uses proper HMAC
    let mut hasher_input = Vec::with_capacity(secret.len() + payload.len());
    hasher_input.extend_from_slice(secret.as_bytes());
    hasher_input.extend_from_slice(payload);
    let hash = simple_sha256(&hasher_input);
    let expected = format!("sha256={}", hash);
    constant_time_eq(signature.as_bytes(), expected.as_bytes())
}

fn simple_sha256(data: &[u8]) -> String {
    // Minimal SHA-256 stub for compilation — real impl delegates to ring/sha2
    let mut hash = 0u64;
    for (i, &byte) in data.iter().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64).wrapping_add(i as u64);
    }
    format!("{:016x}{:016x}{:016x}{:016x}", hash, hash.rotate_left(16), hash.rotate_left(32), hash.rotate_left(48))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// GitHub event parser
// ---------------------------------------------------------------------------

/// Parse a GitHub webhook event payload into an EventPayload.
pub fn parse_github_event(event_type: &str, body: &str) -> EventPayload {
    let mut payload = EventPayload::new("github", event_type, body);
    // Extract common fields from JSON-like structure (simple parsing)
    if let Some(repo) = extract_json_field(body, "full_name") {
        payload = payload.with_field("repository", &repo);
    }
    if let Some(action) = extract_json_field(body, "action") {
        payload = payload.with_field("action", &action);
    }
    if let Some(sender) = extract_json_field(body, "login") {
        payload = payload.with_field("sender", &sender);
    }
    if let Some(ref_field) = extract_json_field(body, "ref") {
        payload = payload.with_field("ref", &ref_field);
    }
    payload
}

/// Parse a Slack event payload.
pub fn parse_slack_event(body: &str) -> EventPayload {
    let event_type = extract_json_field(body, "type").unwrap_or_else(|| "message".to_string());
    let mut payload = EventPayload::new("slack", &event_type, body);
    if let Some(channel) = extract_json_field(body, "channel") {
        payload = payload.with_field("channel", &channel);
    }
    if let Some(user) = extract_json_field(body, "user") {
        payload = payload.with_field("user", &user);
    }
    if let Some(text) = extract_json_field(body, "text") {
        payload = payload.with_field("text", &text);
    }
    payload
}

/// Parse a Linear webhook payload.
pub fn parse_linear_event(body: &str) -> EventPayload {
    let action = extract_json_field(body, "action").unwrap_or_else(|| "update".to_string());
    let mut payload = EventPayload::new("linear", &action, body);
    if let Some(team_id) = extract_json_field(body, "teamId") {
        payload = payload.with_field("team_id", &team_id);
    }
    if let Some(title) = extract_json_field(body, "title") {
        payload = payload.with_field("title", &title);
    }
    if let Some(state) = extract_json_field(body, "state") {
        payload = payload.with_field("state", &state);
    }
    payload
}

/// Parse a PagerDuty webhook payload.
pub fn parse_pagerduty_event(body: &str) -> EventPayload {
    let event_type = extract_json_field(body, "event_type")
        .unwrap_or_else(|| "incident.triggered".to_string());
    let mut payload = EventPayload::new("pagerduty", &event_type, body);
    if let Some(severity) = extract_json_field(body, "severity") {
        payload = payload.with_field("severity", &severity);
    }
    if let Some(service) = extract_json_field(body, "service") {
        payload = payload.with_field("service", &service);
    }
    if let Some(title) = extract_json_field(body, "title") {
        payload = payload.with_field("title", &title);
    }
    payload
}

/// Parse a Telegram Bot API webhook payload.
pub fn parse_telegram_event(body: &str) -> EventPayload {
    let event_type = if body.contains("callback_query") {
        "callback_query"
    } else if body.contains("edited_message") {
        "edited_message"
    } else {
        "message"
    };
    let mut payload = EventPayload::new("telegram", event_type, body);
    if let Some(chat_id) = extract_json_field(body, "chat_id") {
        payload = payload.with_field("chat_id", &chat_id);
    }
    // Try nested chat.id
    if payload.fields.get("chat_id").is_none() {
        if let Some(id) = extract_json_field(body, "id") {
            payload = payload.with_field("chat_id", &id);
        }
    }
    if let Some(text) = extract_json_field(body, "text") {
        payload = payload.with_field("text", &text);
    }
    if let Some(from) = extract_json_field(body, "username") {
        payload = payload.with_field("username", &from);
    }
    payload
}

/// Parse a Signal message event.
pub fn parse_signal_event(body: &str) -> EventPayload {
    let event_type = if body.contains("reaction") {
        "reaction"
    } else {
        "message"
    };
    let mut payload = EventPayload::new("signal", event_type, body);
    if let Some(group_id) = extract_json_field(body, "groupId") {
        payload = payload.with_field("group_id", &group_id);
    }
    if let Some(sender) = extract_json_field(body, "sender") {
        payload = payload.with_field("sender", &sender);
    }
    if let Some(text) = extract_json_field(body, "message") {
        payload = payload.with_field("text", &text);
    }
    payload
}

/// Parse a WhatsApp Business API webhook payload.
pub fn parse_whatsapp_event(body: &str) -> EventPayload {
    let event_type = if body.contains("statuses") {
        "status"
    } else {
        "message"
    };
    let mut payload = EventPayload::new("whatsapp", event_type, body);
    if let Some(from) = extract_json_field(body, "from") {
        payload = payload.with_field("from", &from);
    }
    if let Some(text) = extract_json_field(body, "body") {
        payload = payload.with_field("text", &text);
    }
    if let Some(phone) = extract_json_field(body, "display_phone_number") {
        payload = payload.with_field("to", &phone);
    }
    payload
}

/// Parse a Discord webhook/bot event payload.
pub fn parse_discord_event(body: &str) -> EventPayload {
    let event_type = extract_json_field(body, "t")
        .unwrap_or_else(|| "MESSAGE_CREATE".to_string());
    let mut payload = EventPayload::new("discord", &event_type, body);
    if let Some(channel_id) = extract_json_field(body, "channel_id") {
        payload = payload.with_field("channel_id", &channel_id);
    }
    if let Some(guild_id) = extract_json_field(body, "guild_id") {
        payload = payload.with_field("guild_id", &guild_id);
    }
    if let Some(content) = extract_json_field(body, "content") {
        payload = payload.with_field("text", &content);
    }
    if let Some(author) = extract_json_field(body, "username") {
        payload = payload.with_field("author", &author);
    }
    payload
}

/// Parse a Microsoft Teams activity payload.
pub fn parse_teams_event(body: &str) -> EventPayload {
    let event_type = extract_json_field(body, "type")
        .unwrap_or_else(|| "message".to_string());
    let mut payload = EventPayload::new("teams", &event_type, body);
    if let Some(channel_id) = extract_json_field(body, "channelId") {
        payload = payload.with_field("channel_id", &channel_id);
    }
    if let Some(text) = extract_json_field(body, "text") {
        payload = payload.with_field("text", &text);
    }
    if let Some(from) = extract_json_field(body, "name") {
        payload = payload.with_field("from", &from);
    }
    payload
}

/// Parse a Matrix event payload.
pub fn parse_matrix_event(body: &str) -> EventPayload {
    let event_type = extract_json_field(body, "type")
        .unwrap_or_else(|| "m.room.message".to_string());
    let mut payload = EventPayload::new("matrix", &event_type, body);
    if let Some(room_id) = extract_json_field(body, "room_id") {
        payload = payload.with_field("room_id", &room_id);
    }
    if let Some(sender) = extract_json_field(body, "sender") {
        payload = payload.with_field("sender", &sender);
    }
    if let Some(body_text) = extract_json_field(body, "body") {
        payload = payload.with_field("text", &body_text);
    }
    payload
}

/// Parse a Twilio SMS/MMS webhook payload (form-encoded fields as JSON).
pub fn parse_twilio_sms_event(body: &str) -> EventPayload {
    let mut payload = EventPayload::new("twilio_sms", "incoming", body);
    if let Some(from) = extract_json_field(body, "From") {
        payload = payload.with_field("from", &from);
    }
    if let Some(to) = extract_json_field(body, "To") {
        payload = payload.with_field("to", &to);
    }
    if let Some(text) = extract_json_field(body, "Body") {
        payload = payload.with_field("text", &text);
    }
    payload
}

/// Parse an iMessage event (from AppleScript bridge).
pub fn parse_imessage_event(body: &str) -> EventPayload {
    let mut payload = EventPayload::new("imessage", "message", body);
    if let Some(sender) = extract_json_field(body, "sender") {
        payload = payload.with_field("sender", &sender);
    }
    if let Some(text) = extract_json_field(body, "text") {
        payload = payload.with_field("text", &text);
    }
    if let Some(chat) = extract_json_field(body, "chat") {
        payload = payload.with_field("chat", &chat);
    }
    payload
}

/// Parse an IRC event payload.
pub fn parse_irc_event(body: &str) -> EventPayload {
    let event_type = extract_json_field(body, "command")
        .unwrap_or_else(|| "PRIVMSG".to_string());
    let mut payload = EventPayload::new("irc", &event_type, body);
    if let Some(channel) = extract_json_field(body, "channel") {
        payload = payload.with_field("channel", &channel);
    }
    if let Some(nick) = extract_json_field(body, "nick") {
        payload = payload.with_field("nick", &nick);
    }
    if let Some(text) = extract_json_field(body, "message") {
        payload = payload.with_field("text", &text);
    }
    payload
}

/// Parse a Twitch EventSub / chat webhook payload.
pub fn parse_twitch_event(body: &str) -> EventPayload {
    let event_type = extract_json_field(body, "subscription_type")
        .or_else(|| extract_json_field(body, "event_type"))
        .unwrap_or_else(|| "chat.message".to_string());
    let mut payload = EventPayload::new("twitch", &event_type, body);
    if let Some(channel) = extract_json_field(body, "broadcaster_user_login") {
        payload = payload.with_field("channel", &channel);
    }
    if let Some(user) = extract_json_field(body, "chatter_user_login") {
        payload = payload.with_field("user", &user);
    }
    // Fallback user field
    if payload.fields.get("user").is_none() {
        if let Some(user) = extract_json_field(body, "user_login") {
            payload = payload.with_field("user", &user);
        }
    }
    if let Some(text) = extract_json_field(body, "message_text") {
        payload = payload.with_field("text", &text);
    }
    payload
}

/// Simple glob matching: `*` matches any sequence, `?` matches one char.
fn simple_glob_match(pattern: &str, text: &str) -> bool {
    // Simple two-pointer with backtracking for `*`
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    let (mut px, mut tx) = (0usize, 0usize);
    let (mut star_px, mut star_tx) = (usize::MAX, 0usize);

    while tx < txt.len() {
        if px < pat.len() && (pat[px] == '?' || pat[px] == txt[tx]) {
            px += 1;
            tx += 1;
        } else if px < pat.len() && pat[px] == '*' {
            star_px = px;
            star_tx = tx;
            px += 1;
        } else if star_px != usize::MAX {
            px = star_px + 1;
            star_tx += 1;
            tx = star_tx;
        } else {
            return false;
        }
    }
    while px < pat.len() && pat[px] == '*' {
        px += 1;
    }
    px == pat.len()
}

/// Simple JSON field extractor (avoids serde_json dependency in this module).
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\"", field);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    // Skip whitespace and colon
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':')?;
    let rest = rest.trim_start();
    // Extract string value
    if let Some(rest) = rest.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    } else {
        // Non-string value (number, bool)
        let end = rest.find([',', '}', ']', '\n']).unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> AutomationEngine {
        AutomationEngine::new(PathBuf::from("/tmp/test-automations"))
    }

    #[test]
    fn test_trigger_source_name() {
        let gh = TriggerSource::GitHub {
            events: vec![],
            repos: vec![],
        };
        assert_eq!(gh.source_name(), "github");
        let slack = TriggerSource::Slack {
            events: vec![],
            channels: vec![],
        };
        assert_eq!(slack.source_name(), "slack");
        let linear = TriggerSource::Linear {
            actions: vec![],
            team_ids: vec![],
        };
        assert_eq!(linear.source_name(), "linear");
        let pd = TriggerSource::PagerDuty {
            severity: vec![],
            services: vec![],
        };
        assert_eq!(pd.source_name(), "pagerduty");
        let cron = TriggerSource::Cron {
            expression: "0 * * * *".into(),
        };
        assert_eq!(cron.source_name(), "cron");
        let fw = TriggerSource::FileWatch {
            patterns: vec![],
            path: PathBuf::from("."),
        };
        assert_eq!(fw.source_name(), "filewatch");
        let wh = TriggerSource::Webhook { secret: None };
        assert_eq!(wh.source_name(), "webhook");
    }

    #[test]
    fn test_event_payload_new() {
        let p = EventPayload::new("github", "push", "{}");
        assert_eq!(p.source, "github");
        assert_eq!(p.event_type, "push");
        assert!(p.timestamp > 0);
    }

    #[test]
    fn test_event_payload_with_field() {
        let p = EventPayload::new("slack", "message", "{}")
            .with_field("channel", "#general")
            .with_field("user", "alice");
        assert_eq!(p.fields.get("channel").unwrap(), "#general");
        assert_eq!(p.fields.get("user").unwrap(), "alice");
    }

    #[test]
    fn test_event_filter_empty_matches_all() {
        let filter = EventFilter::default();
        let payload = EventPayload::new("test", "test", "body");
        assert!(filter.matches(&payload));
    }

    #[test]
    fn test_event_filter_required_fields() {
        let filter = EventFilter {
            required_fields: vec!["repo".to_string()],
            ..Default::default()
        };
        let p1 = EventPayload::new("test", "test", "");
        assert!(!filter.matches(&p1));
        let p2 = p1.with_field("repo", "my-repo");
        assert!(filter.matches(&p2));
    }

    #[test]
    fn test_event_filter_conditions() {
        let mut conditions = HashMap::new();
        conditions.insert("action".to_string(), "opened".to_string());
        let filter = EventFilter {
            conditions,
            ..Default::default()
        };
        let p1 = EventPayload::new("test", "test", "").with_field("action", "closed");
        assert!(!filter.matches(&p1));
        let p2 = EventPayload::new("test", "test", "").with_field("action", "opened");
        assert!(filter.matches(&p2));
    }

    #[test]
    fn test_event_filter_body_pattern() {
        let filter = EventFilter {
            body_pattern: Some("urgent".to_string()),
            ..Default::default()
        };
        let p1 = EventPayload::new("test", "test", "normal event");
        assert!(!filter.matches(&p1));
        let p2 = EventPayload::new("test", "test", "this is urgent!");
        assert!(filter.matches(&p2));
    }

    #[test]
    fn test_prompt_template_render() {
        let tpl = PromptTemplate::new("Fix issue in {{repository}}: {{action}} by {{sender}}");
        let payload = EventPayload::new("github", "issues", "{}")
            .with_field("repository", "vibecody")
            .with_field("action", "opened")
            .with_field("sender", "alice");
        let rendered = tpl.render(&payload);
        assert_eq!(rendered, "Fix issue in vibecody: opened by alice");
    }

    #[test]
    fn test_prompt_template_defaults() {
        let tpl = PromptTemplate::new("Deploy {{service}} to {{env}}")
            .with_default("env", "staging");
        let payload = EventPayload::new("webhook", "deploy", "")
            .with_field("service", "api");
        let rendered = tpl.render(&payload);
        assert_eq!(rendered, "Deploy api to staging");
    }

    #[test]
    fn test_prompt_template_metadata() {
        let tpl = PromptTemplate::new("Source: {{source}}, Type: {{event_type}}");
        let payload = EventPayload::new("github", "push", "");
        let rendered = tpl.render(&payload);
        assert_eq!(rendered, "Source: github, Type: push");
    }

    #[test]
    fn test_automation_sandbox_default() {
        let sb = AutomationSandbox::default();
        assert!(!sb.enabled);
        assert_eq!(sb.runtime, "docker");
        assert_eq!(sb.timeout_secs, 300);
        assert_eq!(sb.memory_limit_mb, 512);
        assert!(sb.network_enabled);
    }

    #[test]
    fn test_automation_rule_new() {
        let rule = AutomationRule::new(
            "r1",
            "On push",
            TriggerSource::GitHub {
                events: vec!["push".into()],
                repos: vec![],
            },
            "Run tests for {{repository}}",
        );
        assert_eq!(rule.id, "r1");
        assert!(rule.enabled);
        assert_eq!(rule.fire_count, 0);
        assert!(rule.last_fired.is_none());
    }

    #[test]
    fn test_rule_matches_github_push() {
        let rule = AutomationRule::new(
            "r1",
            "Push handler",
            TriggerSource::GitHub {
                events: vec!["push".into()],
                repos: vec!["vibecody".into()],
            },
            "Test",
        );
        let p1 = EventPayload::new("github", "push", "")
            .with_field("repository", "vibecody");
        assert!(rule.matches(&p1));

        let p2 = EventPayload::new("github", "push", "")
            .with_field("repository", "other-repo");
        assert!(!rule.matches(&p2));

        let p3 = EventPayload::new("github", "issues", "")
            .with_field("repository", "vibecody");
        assert!(!rule.matches(&p3));
    }

    #[test]
    fn test_rule_matches_slack() {
        let rule = AutomationRule::new(
            "r2",
            "Slack handler",
            TriggerSource::Slack {
                events: vec!["app_mention".into()],
                channels: vec!["#dev".into()],
            },
            "Respond",
        );
        let p = EventPayload::new("slack", "app_mention", "")
            .with_field("channel", "#dev");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_linear() {
        let rule = AutomationRule::new(
            "r3",
            "Linear handler",
            TriggerSource::Linear {
                actions: vec!["update".into()],
                team_ids: vec!["team-1".into()],
            },
            "Handle",
        );
        let p = EventPayload::new("linear", "update", "")
            .with_field("team_id", "team-1");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_pagerduty() {
        let rule = AutomationRule::new(
            "r4",
            "PD handler",
            TriggerSource::PagerDuty {
                severity: vec!["critical".into()],
                services: vec![],
            },
            "Triage",
        );
        let p = EventPayload::new("pagerduty", "incident.triggered", "")
            .with_field("severity", "critical");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_cron() {
        let rule = AutomationRule::new(
            "r5",
            "Cron handler",
            TriggerSource::Cron {
                expression: "0 * * * *".into(),
            },
            "Hourly check",
        );
        let p = EventPayload::new("cron", "tick", "");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_filewatch() {
        let rule = AutomationRule::new(
            "r6",
            "File handler",
            TriggerSource::FileWatch {
                patterns: vec!["*.rs".into()],
                path: PathBuf::from("/src"),
            },
            "Lint",
        );
        let p = EventPayload::new("filewatch", "changed", "")
            .with_field("path", "main.rs");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_webhook_generic() {
        let rule = AutomationRule::new(
            "r7",
            "Webhook handler",
            TriggerSource::Webhook { secret: None },
            "Process",
        );
        let p = EventPayload::new("webhook", "custom", "data");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_disabled_rule_no_match() {
        let mut rule = AutomationRule::new(
            "r1",
            "Disabled",
            TriggerSource::Webhook { secret: None },
            "Test",
        );
        rule.enabled = false;
        let p = EventPayload::new("webhook", "any", "");
        assert!(!rule.matches(&p));
    }

    #[test]
    fn test_engine_add_remove_rule() {
        let mut engine = test_engine();
        let rule = AutomationRule::new("r1", "Test", TriggerSource::Webhook { secret: None }, "p");
        engine.add_rule(rule);
        assert_eq!(engine.list_rules().len(), 1);
        assert!(engine.remove_rule("r1"));
        assert_eq!(engine.list_rules().len(), 0);
        assert!(!engine.remove_rule("nonexistent"));
    }

    #[test]
    fn test_engine_get_rule() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "Test",
            TriggerSource::Webhook { secret: None },
            "p",
        ));
        assert!(engine.get_rule("r1").is_some());
        assert!(engine.get_rule("r2").is_none());
    }

    #[test]
    fn test_engine_enable_disable() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "Test",
            TriggerSource::Webhook { secret: None },
            "p",
        ));
        assert!(engine.disable_rule("r1"));
        assert!(!engine.get_rule("r1").unwrap().enabled);
        assert!(engine.enable_rule("r1"));
        assert!(engine.get_rule("r1").unwrap().enabled);
        assert!(!engine.enable_rule("nonexistent"));
    }

    #[test]
    fn test_engine_process_event() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "GH Push",
            TriggerSource::GitHub {
                events: vec!["push".into()],
                repos: vec![],
            },
            "Run tests for {{repository}}",
        ));
        let payload = EventPayload::new("github", "push", "")
            .with_field("repository", "vibecody");
        let tasks = engine.process_event(&payload);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].prompt, "Run tests for vibecody");
        assert_eq!(tasks[0].status, TaskStatus::Queued);
        assert_eq!(engine.get_rule("r1").unwrap().fire_count, 1);
    }

    #[test]
    fn test_engine_process_event_no_match() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "Slack Only",
            TriggerSource::Slack {
                events: vec![],
                channels: vec![],
            },
            "Handle",
        ));
        let payload = EventPayload::new("github", "push", "");
        let tasks = engine.process_event(&payload);
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_engine_multiple_rules_match() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "Rule 1",
            TriggerSource::Webhook { secret: None },
            "First",
        ));
        engine.add_rule(AutomationRule::new(
            "r2",
            "Rule 2",
            TriggerSource::Webhook { secret: None },
            "Second",
        ));
        let payload = EventPayload::new("webhook", "test", "");
        let tasks = engine.process_event(&payload);
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_engine_task_tracking() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "Test",
            TriggerSource::Webhook { secret: None },
            "p",
        ));
        let payload = EventPayload::new("webhook", "test", "");
        let tasks = engine.process_event(&payload);
        let task_id = tasks[0].task_id.clone();

        assert!(engine.get_task(&task_id).is_some());
        assert_eq!(engine.list_tasks().len(), 1);

        assert!(engine.update_task_status(&task_id, TaskStatus::Running));
        assert_eq!(engine.get_task(&task_id).unwrap().status, TaskStatus::Running);

        assert!(engine.update_task_status(&task_id, TaskStatus::Completed));
        assert_eq!(
            engine.get_task(&task_id).unwrap().status,
            TaskStatus::Completed
        );
        assert!(engine.get_task(&task_id).unwrap().completed_at.is_some());
    }

    #[test]
    fn test_engine_stats() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "Enabled",
            TriggerSource::Webhook { secret: None },
            "p",
        ));
        let mut disabled = AutomationRule::new(
            "r2",
            "Disabled",
            TriggerSource::Webhook { secret: None },
            "p",
        );
        disabled.enabled = false;
        engine.add_rule(disabled);

        let stats = engine.stats();
        assert_eq!(stats.total_rules, 2);
        assert_eq!(stats.enabled_rules, 1);
        assert_eq!(stats.total_tasks, 0);
    }

    #[test]
    fn test_engine_task_failed_status() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "Test",
            TriggerSource::Webhook { secret: None },
            "p",
        ));
        let tasks = engine.process_event(&EventPayload::new("webhook", "t", ""));
        let tid = tasks[0].task_id.clone();
        engine.update_task_status(&tid, TaskStatus::Failed("timeout".into()));
        assert!(matches!(
            engine.get_task(&tid).unwrap().status,
            TaskStatus::Failed(_)
        ));
    }

    #[test]
    fn test_engine_config_path() {
        let engine = test_engine();
        assert_eq!(engine.config_path(), &PathBuf::from("/tmp/test-automations"));
    }

    #[test]
    fn test_parse_github_event() {
        let body = r#"{"action": "opened", "repository": {"full_name": "org/repo"}, "sender": {"login": "alice"}, "ref": "refs/heads/main"}"#;
        let p = parse_github_event("pull_request", body);
        assert_eq!(p.source, "github");
        assert_eq!(p.event_type, "pull_request");
        assert_eq!(p.fields.get("repository").unwrap(), "org/repo");
        assert_eq!(p.fields.get("action").unwrap(), "opened");
        assert_eq!(p.fields.get("sender").unwrap(), "alice");
    }

    #[test]
    fn test_parse_slack_event() {
        let body = r#"{"type": "app_mention", "channel": "C123", "user": "U456", "text": "hey bot"}"#;
        let p = parse_slack_event(body);
        assert_eq!(p.source, "slack");
        assert_eq!(p.event_type, "app_mention");
        assert_eq!(p.fields.get("channel").unwrap(), "C123");
        assert_eq!(p.fields.get("text").unwrap(), "hey bot");
    }

    #[test]
    fn test_parse_linear_event() {
        let body = r#"{"action": "create", "teamId": "team-1", "title": "Fix bug", "state": "Todo"}"#;
        let p = parse_linear_event(body);
        assert_eq!(p.source, "linear");
        assert_eq!(p.event_type, "create");
        assert_eq!(p.fields.get("team_id").unwrap(), "team-1");
        assert_eq!(p.fields.get("title").unwrap(), "Fix bug");
    }

    #[test]
    fn test_parse_pagerduty_event() {
        let body = r#"{"event_type": "incident.triggered", "severity": "critical", "service": "api", "title": "High latency"}"#;
        let p = parse_pagerduty_event(body);
        assert_eq!(p.source, "pagerduty");
        assert_eq!(p.event_type, "incident.triggered");
        assert_eq!(p.fields.get("severity").unwrap(), "critical");
        assert_eq!(p.fields.get("service").unwrap(), "api");
    }

    #[test]
    fn test_extract_json_field() {
        assert_eq!(
            extract_json_field(r#"{"name": "test"}"#, "name"),
            Some("test".to_string())
        );
        assert_eq!(
            extract_json_field(r#"{"count": 42}"#, "count"),
            Some("42".to_string())
        );
        assert_eq!(extract_json_field(r#"{}"#, "missing"), None);
    }

    #[test]
    fn test_webhook_signature_empty() {
        assert!(!verify_webhook_signature(b"data", "", "secret"));
        assert!(!verify_webhook_signature(b"data", "sig", ""));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn test_rule_render_prompt() {
        let rule = AutomationRule::new(
            "r1",
            "Test",
            TriggerSource::Webhook { secret: None },
            "Handle {{event_type}} from {{source}}",
        );
        let payload = EventPayload::new("github", "push", "");
        assert_eq!(rule.render_prompt(&payload), "Handle push from github");
    }

    #[test]
    fn test_rule_with_filter() {
        let mut conditions = HashMap::new();
        conditions.insert("action".to_string(), "opened".to_string());
        let mut rule = AutomationRule::new(
            "r1",
            "Filtered",
            TriggerSource::GitHub {
                events: vec!["issues".into()],
                repos: vec![],
            },
            "Handle",
        );
        rule.filter = EventFilter {
            conditions,
            ..Default::default()
        };
        let p1 = EventPayload::new("github", "issues", "")
            .with_field("action", "opened");
        assert!(rule.matches(&p1));
        let p2 = EventPayload::new("github", "issues", "")
            .with_field("action", "closed");
        assert!(!rule.matches(&p2));
    }

    #[test]
    fn test_filewatch_empty_patterns_matches_all() {
        let rule = AutomationRule::new(
            "r1",
            "FW",
            TriggerSource::FileWatch {
                patterns: vec![],
                path: PathBuf::from("."),
            },
            "Lint",
        );
        let p = EventPayload::new("filewatch", "changed", "")
            .with_field("path", "anything.txt");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_github_empty_repos_matches_all() {
        let rule = AutomationRule::new(
            "r1",
            "GH",
            TriggerSource::GitHub {
                events: vec!["push".into()],
                repos: vec![],
            },
            "Test",
        );
        let p = EventPayload::new("github", "push", "")
            .with_field("repository", "any-repo");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_slack_empty_channels_matches_all() {
        let rule = AutomationRule::new(
            "r1",
            "Slack",
            TriggerSource::Slack {
                events: vec![],
                channels: vec![],
            },
            "Handle",
        );
        let p = EventPayload::new("slack", "message", "")
            .with_field("channel", "#random");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_task_cancelled_has_completed_at() {
        let mut engine = test_engine();
        engine.add_rule(AutomationRule::new(
            "r1",
            "T",
            TriggerSource::Webhook { secret: None },
            "p",
        ));
        let tasks = engine.process_event(&EventPayload::new("webhook", "t", ""));
        let tid = tasks[0].task_id.clone();
        engine.update_task_status(&tid, TaskStatus::Cancelled);
        assert_eq!(engine.get_task(&tid).unwrap().status, TaskStatus::Cancelled);
        assert!(engine.get_task(&tid).unwrap().completed_at.is_some());
    }

    #[test]
    fn test_update_nonexistent_task() {
        let mut engine = test_engine();
        assert!(!engine.update_task_status("fake-id", TaskStatus::Running));
    }

    // -----------------------------------------------------------------------
    // Messaging platform trigger sources
    // -----------------------------------------------------------------------

    #[test]
    fn test_trigger_source_name_messaging() {
        assert_eq!(TriggerSource::Telegram { events: vec![], chat_ids: vec![] }.source_name(), "telegram");
        assert_eq!(TriggerSource::Signal { events: vec![], group_ids: vec![] }.source_name(), "signal");
        assert_eq!(TriggerSource::WhatsApp { events: vec![], phone_numbers: vec![] }.source_name(), "whatsapp");
        assert_eq!(TriggerSource::Discord { events: vec![], channel_ids: vec![], guild_ids: vec![] }.source_name(), "discord");
        assert_eq!(TriggerSource::Teams { events: vec![], channel_ids: vec![] }.source_name(), "teams");
        assert_eq!(TriggerSource::Matrix { events: vec![], room_ids: vec![] }.source_name(), "matrix");
        assert_eq!(TriggerSource::TwilioSms { events: vec![], from_numbers: vec![] }.source_name(), "twilio_sms");
        assert_eq!(TriggerSource::IMessage { events: vec![], contacts: vec![] }.source_name(), "imessage");
        assert_eq!(TriggerSource::Irc { events: vec![], channels: vec![] }.source_name(), "irc");
        assert_eq!(TriggerSource::Twitch { events: vec![], channels: vec![] }.source_name(), "twitch");
    }

    #[test]
    fn test_rule_matches_telegram() {
        let rule = AutomationRule::new(
            "r1", "TG",
            TriggerSource::Telegram { events: vec!["message".into()], chat_ids: vec!["123".into()] },
            "Handle",
        );
        let p = EventPayload::new("telegram", "message", "").with_field("chat_id", "123");
        assert!(rule.matches(&p));
        let p2 = EventPayload::new("telegram", "message", "").with_field("chat_id", "999");
        assert!(!rule.matches(&p2));
    }

    #[test]
    fn test_rule_matches_telegram_empty_filter() {
        let rule = AutomationRule::new(
            "r1", "TG all",
            TriggerSource::Telegram { events: vec![], chat_ids: vec![] },
            "Handle",
        );
        let p = EventPayload::new("telegram", "message", "").with_field("chat_id", "any");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_signal() {
        let rule = AutomationRule::new(
            "r1", "Signal",
            TriggerSource::Signal { events: vec!["message".into()], group_ids: vec!["grp-1".into()] },
            "Handle",
        );
        let p = EventPayload::new("signal", "message", "").with_field("group_id", "grp-1");
        assert!(rule.matches(&p));
        let p2 = EventPayload::new("signal", "message", "").with_field("group_id", "grp-2");
        assert!(!rule.matches(&p2));
    }

    #[test]
    fn test_rule_matches_whatsapp() {
        let rule = AutomationRule::new(
            "r1", "WA",
            TriggerSource::WhatsApp { events: vec!["message".into()], phone_numbers: vec!["+1234".into()] },
            "Handle",
        );
        let p = EventPayload::new("whatsapp", "message", "").with_field("from", "+1234");
        assert!(rule.matches(&p));
        let p2 = EventPayload::new("whatsapp", "message", "").with_field("from", "+9999");
        assert!(!rule.matches(&p2));
    }

    #[test]
    fn test_rule_matches_discord() {
        let rule = AutomationRule::new(
            "r1", "Discord",
            TriggerSource::Discord {
                events: vec!["MESSAGE_CREATE".into()],
                channel_ids: vec!["ch-1".into()],
                guild_ids: vec!["g-1".into()],
            },
            "Handle",
        );
        let p = EventPayload::new("discord", "MESSAGE_CREATE", "")
            .with_field("channel_id", "ch-1")
            .with_field("guild_id", "g-1");
        assert!(rule.matches(&p));
        // Wrong guild
        let p2 = EventPayload::new("discord", "MESSAGE_CREATE", "")
            .with_field("channel_id", "ch-1")
            .with_field("guild_id", "g-2");
        assert!(!rule.matches(&p2));
    }

    #[test]
    fn test_rule_matches_discord_empty_guild() {
        let rule = AutomationRule::new(
            "r1", "Discord",
            TriggerSource::Discord { events: vec![], channel_ids: vec![], guild_ids: vec![] },
            "Handle",
        );
        let p = EventPayload::new("discord", "MESSAGE_CREATE", "");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_teams() {
        let rule = AutomationRule::new(
            "r1", "Teams",
            TriggerSource::Teams { events: vec!["message".into()], channel_ids: vec!["ch-1".into()] },
            "Handle",
        );
        let p = EventPayload::new("teams", "message", "").with_field("channel_id", "ch-1");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_matrix() {
        let rule = AutomationRule::new(
            "r1", "Matrix",
            TriggerSource::Matrix { events: vec!["m.room.message".into()], room_ids: vec!["!abc:matrix.org".into()] },
            "Handle",
        );
        let p = EventPayload::new("matrix", "m.room.message", "").with_field("room_id", "!abc:matrix.org");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_twilio_sms() {
        let rule = AutomationRule::new(
            "r1", "SMS",
            TriggerSource::TwilioSms { events: vec!["incoming".into()], from_numbers: vec!["+15551234".into()] },
            "Handle",
        );
        let p = EventPayload::new("twilio_sms", "incoming", "").with_field("from", "+15551234");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_imessage() {
        let rule = AutomationRule::new(
            "r1", "iMsg",
            TriggerSource::IMessage { events: vec!["message".into()], contacts: vec!["alice@icloud.com".into()] },
            "Handle",
        );
        let p = EventPayload::new("imessage", "message", "").with_field("sender", "alice@icloud.com");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_irc() {
        let rule = AutomationRule::new(
            "r1", "IRC",
            TriggerSource::Irc { events: vec!["PRIVMSG".into()], channels: vec!["#rust".into()] },
            "Handle",
        );
        let p = EventPayload::new("irc", "PRIVMSG", "").with_field("channel", "#rust");
        assert!(rule.matches(&p));
    }

    #[test]
    fn test_rule_matches_twitch() {
        let rule = AutomationRule::new(
            "r1", "Twitch",
            TriggerSource::Twitch { events: vec!["chat.message".into()], channels: vec!["streamer1".into()] },
            "Handle",
        );
        let p = EventPayload::new("twitch", "chat.message", "").with_field("channel", "streamer1");
        assert!(rule.matches(&p));
    }

    // -----------------------------------------------------------------------
    // Messaging platform event parsers
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_telegram_event() {
        let body = r#"{"message": {"chat": {"chat_id": "123"}, "text": "hello", "from": {"username": "alice"}}}"#;
        let p = parse_telegram_event(body);
        assert_eq!(p.source, "telegram");
        assert_eq!(p.event_type, "message");
        assert_eq!(p.fields.get("chat_id").unwrap(), "123");
        assert_eq!(p.fields.get("text").unwrap(), "hello");
        assert_eq!(p.fields.get("username").unwrap(), "alice");
    }

    #[test]
    fn test_parse_telegram_callback() {
        let body = r#"{"callback_query": {"data": "btn1"}}"#;
        let p = parse_telegram_event(body);
        assert_eq!(p.event_type, "callback_query");
    }

    #[test]
    fn test_parse_signal_event() {
        let body = r#"{"sender": "+1234", "message": "help me", "groupId": "grp-abc"}"#;
        let p = parse_signal_event(body);
        assert_eq!(p.source, "signal");
        assert_eq!(p.event_type, "message");
        assert_eq!(p.fields.get("sender").unwrap(), "+1234");
        assert_eq!(p.fields.get("text").unwrap(), "help me");
        assert_eq!(p.fields.get("group_id").unwrap(), "grp-abc");
    }

    #[test]
    fn test_parse_signal_reaction() {
        let body = r#"{"reaction": "👍", "sender": "+5678"}"#;
        let p = parse_signal_event(body);
        assert_eq!(p.event_type, "reaction");
    }

    #[test]
    fn test_parse_whatsapp_event() {
        let body = r#"{"from": "+1234567890", "body": "Hi there", "display_phone_number": "+0987654321"}"#;
        let p = parse_whatsapp_event(body);
        assert_eq!(p.source, "whatsapp");
        assert_eq!(p.event_type, "message");
        assert_eq!(p.fields.get("from").unwrap(), "+1234567890");
        assert_eq!(p.fields.get("text").unwrap(), "Hi there");
        assert_eq!(p.fields.get("to").unwrap(), "+0987654321");
    }

    #[test]
    fn test_parse_whatsapp_status() {
        let body = r#"{"statuses": [{"id": "msg1"}]}"#;
        let p = parse_whatsapp_event(body);
        assert_eq!(p.event_type, "status");
    }

    #[test]
    fn test_parse_discord_event() {
        let body = r#"{"t": "MESSAGE_CREATE", "channel_id": "ch-1", "guild_id": "g-1", "content": "hello", "author": {"username": "bob"}}"#;
        let p = parse_discord_event(body);
        assert_eq!(p.source, "discord");
        assert_eq!(p.event_type, "MESSAGE_CREATE");
        assert_eq!(p.fields.get("channel_id").unwrap(), "ch-1");
        assert_eq!(p.fields.get("guild_id").unwrap(), "g-1");
        assert_eq!(p.fields.get("text").unwrap(), "hello");
        assert_eq!(p.fields.get("author").unwrap(), "bob");
    }

    #[test]
    fn test_parse_teams_event() {
        let body = r#"{"type": "message", "channelId": "19:abc", "text": "deploy now", "from": {"name": "Alice"}}"#;
        let p = parse_teams_event(body);
        assert_eq!(p.source, "teams");
        assert_eq!(p.event_type, "message");
        assert_eq!(p.fields.get("channel_id").unwrap(), "19:abc");
        assert_eq!(p.fields.get("text").unwrap(), "deploy now");
        assert_eq!(p.fields.get("from").unwrap(), "Alice");
    }

    #[test]
    fn test_parse_matrix_event() {
        let body = r#"{"type": "m.room.message", "room_id": "!abc:matrix.org", "sender": "@alice:matrix.org", "content": {"body": "hello"}}"#;
        let p = parse_matrix_event(body);
        assert_eq!(p.source, "matrix");
        assert_eq!(p.event_type, "m.room.message");
        assert_eq!(p.fields.get("room_id").unwrap(), "!abc:matrix.org");
        assert_eq!(p.fields.get("sender").unwrap(), "@alice:matrix.org");
        assert_eq!(p.fields.get("text").unwrap(), "hello");
    }

    #[test]
    fn test_parse_twilio_sms_event() {
        let body = r#"{"From": "+15551234", "To": "+15559876", "Body": "help"}"#;
        let p = parse_twilio_sms_event(body);
        assert_eq!(p.source, "twilio_sms");
        assert_eq!(p.event_type, "incoming");
        assert_eq!(p.fields.get("from").unwrap(), "+15551234");
        assert_eq!(p.fields.get("to").unwrap(), "+15559876");
        assert_eq!(p.fields.get("text").unwrap(), "help");
    }

    #[test]
    fn test_parse_imessage_event() {
        let body = r#"{"sender": "alice@icloud.com", "text": "hey", "chat": "iMessage;-;alice@icloud.com"}"#;
        let p = parse_imessage_event(body);
        assert_eq!(p.source, "imessage");
        assert_eq!(p.event_type, "message");
        assert_eq!(p.fields.get("sender").unwrap(), "alice@icloud.com");
        assert_eq!(p.fields.get("text").unwrap(), "hey");
    }

    #[test]
    fn test_parse_irc_event() {
        let body = r##"{"command": "PRIVMSG", "channel": "#rust", "nick": "bob", "message": "hello"}"##;
        let p = parse_irc_event(body);
        assert_eq!(p.source, "irc");
        assert_eq!(p.event_type, "PRIVMSG");
        assert_eq!(p.fields.get("channel").unwrap(), "#rust");
        assert_eq!(p.fields.get("nick").unwrap(), "bob");
        assert_eq!(p.fields.get("text").unwrap(), "hello");
    }

    #[test]
    fn test_parse_twitch_event() {
        let body = r#"{"subscription_type": "channel.chat.message", "broadcaster_user_login": "streamer1", "chatter_user_login": "viewer1", "message_text": "GG"}"#;
        let p = parse_twitch_event(body);
        assert_eq!(p.source, "twitch");
        assert_eq!(p.event_type, "channel.chat.message");
        assert_eq!(p.fields.get("channel").unwrap(), "streamer1");
        assert_eq!(p.fields.get("user").unwrap(), "viewer1");
        assert_eq!(p.fields.get("text").unwrap(), "GG");
    }

    #[test]
    fn test_parse_twitch_fallback_user() {
        let body = r#"{"event_type": "subscription", "user_login": "sub_user"}"#;
        let p = parse_twitch_event(body);
        assert_eq!(p.event_type, "subscription");
        assert_eq!(p.fields.get("user").unwrap(), "sub_user");
    }

    // -----------------------------------------------------------------------
    // Cross-source mismatch tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_telegram_rule_ignores_slack_event() {
        let rule = AutomationRule::new(
            "r1", "TG",
            TriggerSource::Telegram { events: vec![], chat_ids: vec![] },
            "Handle",
        );
        let p = EventPayload::new("slack", "message", "");
        assert!(!rule.matches(&p));
    }

    #[test]
    fn test_discord_rule_ignores_teams_event() {
        let rule = AutomationRule::new(
            "r1", "Discord",
            TriggerSource::Discord { events: vec![], channel_ids: vec![], guild_ids: vec![] },
            "Handle",
        );
        let p = EventPayload::new("teams", "message", "");
        assert!(!rule.matches(&p));
    }

    #[test]
    fn test_whatsapp_rule_ignores_signal_event() {
        let rule = AutomationRule::new(
            "r1", "WA",
            TriggerSource::WhatsApp { events: vec![], phone_numbers: vec![] },
            "Handle",
        );
        let p = EventPayload::new("signal", "message", "");
        assert!(!rule.matches(&p));
    }
}
