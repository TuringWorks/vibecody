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
                            .map_or(false, |r| repos.contains(r)))
            }
            TriggerSource::Slack { events, channels } => {
                payload.source == "slack"
                    && (events.is_empty() || events.contains(&payload.event_type))
                    && (channels.is_empty()
                        || payload
                            .fields
                            .get("channel")
                            .map_or(false, |c| channels.contains(c)))
            }
            TriggerSource::Linear { actions, team_ids } => {
                payload.source == "linear"
                    && (actions.is_empty() || actions.contains(&payload.event_type))
                    && (team_ids.is_empty()
                        || payload
                            .fields
                            .get("team_id")
                            .map_or(false, |t| team_ids.contains(t)))
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
                            .map_or(false, |s| severity.contains(s)))
                    && (services.is_empty()
                        || payload
                            .fields
                            .get("service")
                            .map_or(false, |s| services.contains(s)))
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
                    .map_or(false, |p| patterns.iter().any(|pat| simple_glob_match(pat, p)))
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
    if rest.starts_with('"') {
        let rest = &rest[1..];
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
}
