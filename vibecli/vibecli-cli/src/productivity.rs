//! Unified productivity integrations: Notion, Todoist, Jira.
//!
//! REPL commands:
//! ```
//! /notion search <query>      — Search Notion pages
//! /notion page <id>           — Read a Notion page
//! /notion create <title>      — Create a page
//! /todo list                  — List Todoist tasks
//! /todo add <content>         — Add a task
//! /todo complete <id>         — Complete a task
//! /todo today                 — Tasks due today
//! /jira list                  — List assigned Jira issues
//! /jira create <proj> <summ>  — Create Jira issue
//! /jira transition <key> <s>  — Move issue status
//! /jira comment <key> <text>  — Add comment
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use vibe_ai::{retry_async, RetryConfig};

const NOTION_API: &str = "https://api.notion.com/v1";
const TODOIST_API: &str = "https://api.todoist.com/rest/v2";

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionPage {
    pub id: String,
    pub title: String,
    pub url: String,
    pub last_edited: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoistTask {
    pub id: String,
    pub content: String,
    pub description: String,
    pub due: Option<String>,
    pub priority: u8,
    pub project_id: Option<String>,
    pub is_completed: bool,
}

impl TodoistTask {
    pub fn priority_icon(&self) -> &str {
        match self.priority {
            4 => "🔥",
            3 => "⚡",
            2 => "📌",
            _ => "·",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub issue_type: String,
    pub url: String,
}

// ── API clients ──────────────────────────────────────────────────────────────

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("VibeCLI/1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

// ── Notion ───────────────────────────────────────────────────────────────────

struct NotionClient {
    api_key: String,
    client: reqwest::Client,
}

impl NotionClient {
    fn from_env_or_config() -> Option<Self> {
        let key = std::env::var("NOTION_API_KEY").ok()
            .or_else(|| {
                crate::config::Config::load().ok()
                    .and_then(|c| c.notion_api_key)
            })?;
        if key.is_empty() { return None; }
        Some(Self { api_key: key, client: build_client() })
    }

    async fn search(&self, query: &str) -> Result<Vec<NotionPage>> {
        let payload = serde_json::json!({ "query": query, "page_size": 20 });
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let payload_c = payload.clone();
        let resp = retry_async(&RetryConfig::default(), "notion-search", || {
            let client = client.clone();
            let api_key = api_key.clone();
            let payload = payload_c.clone();
            async move {
                client.post(format!("{}/search", NOTION_API))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Notion-Version", "2022-06-28")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        let data: serde_json::Value = resp.json().await?;
        let pages = data["results"].as_array()
            .map(|arr| arr.iter().map(|p| {
                let title = p["properties"]["title"]["title"].as_array()
                    .or_else(|| p["properties"]["Name"]["title"].as_array())
                    .and_then(|a| a.first())
                    .and_then(|t| t["plain_text"].as_str())
                    .unwrap_or("Untitled").to_string();
                NotionPage {
                    id: p["id"].as_str().unwrap_or("").to_string(),
                    title,
                    url: p["url"].as_str().unwrap_or("").to_string(),
                    last_edited: p["last_edited_time"].as_str().unwrap_or("").to_string(),
                    icon: p["icon"]["emoji"].as_str().map(String::from),
                }
            }).collect())
            .unwrap_or_default();
        Ok(pages)
    }

    async fn get_page(&self, id: &str) -> Result<String> {
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let url = format!("{}/blocks/{}/children", NOTION_API, id);
        let resp = retry_async(&RetryConfig::default(), "notion-page", || {
            let client = client.clone();
            let api_key = api_key.clone();
            let url = url.clone();
            async move {
                client.get(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Notion-Version", "2022-06-28")
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        let data: serde_json::Value = resp.json().await?;
        let mut content = String::new();
        if let Some(blocks) = data["results"].as_array() {
            for block in blocks {
                let btype = block["type"].as_str().unwrap_or("");
                let texts = block[btype]["rich_text"].as_array();
                if let Some(texts) = texts {
                    for t in texts {
                        content.push_str(t["plain_text"].as_str().unwrap_or(""));
                    }
                    content.push('\n');
                }
            }
        }
        if content.is_empty() { content = "(empty page)".to_string(); }
        Ok(content)
    }

    async fn append_text(&self, page_id: &str, text: &str) -> Result<()> {
        let children: Vec<serde_json::Value> = text
            .split('\n')
            .map(|line| serde_json::json!({
                "object": "block",
                "type": "paragraph",
                "paragraph": {
                    "rich_text": [{
                        "type": "text",
                        "text": { "content": line }
                    }]
                }
            }))
            .collect();
        let payload = serde_json::json!({ "children": children });
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let url = format!("{}/blocks/{}/children", NOTION_API, page_id);
        let payload_c = payload.clone();
        let resp = retry_async(&RetryConfig::default(), "notion-append", || {
            let client = client.clone();
            let api_key = api_key.clone();
            let url = url.clone();
            let payload = payload_c.clone();
            async move {
                client.patch(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Notion-Version", "2022-06-28")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Notion append failed ({}): {}", status, body);
        }
        Ok(())
    }
}

async fn handle_notion(args: &str) -> String {
    let client = match NotionClient::from_env_or_config() {
        Some(c) => c,
        None => return "⚠️  Notion not configured. Set NOTION_API_KEY env var or notion_api_key in config.\n".to_string(),
    };
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let subcmd = parts.first().copied().unwrap_or("");
    let rest = parts.get(1).copied().unwrap_or("");

    match subcmd {
        "search" | "find" => {
            if rest.is_empty() { return "Usage: /notion search <query>\n".to_string(); }
            match client.search(rest).await {
                Ok(pages) => {
                    if pages.is_empty() { return "🔎 No results.\n".to_string(); }
                    let mut out = format!("📓 Notion Search: \"{}\" ({} results)\n", rest, pages.len());
                    for p in &pages {
                        let icon = p.icon.as_deref().unwrap_or("📄");
                        out.push_str(&format!("  {} {}  {}\n    {}\n", icon, p.title, p.last_edited, p.url));
                    }
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "page" | "read" => {
            if rest.is_empty() { return "Usage: /notion page <page_id>\n".to_string(); }
            match client.get_page(rest).await {
                Ok(content) => content,
                Err(e) => format!("❌ {}\n", e),
            }
        }
        _ => "📓 Notion Commands:\n\
              /notion search <query>   — Search pages\n\
              /notion page <id>        — Read page content\n\n\
            Config: Set NOTION_API_KEY env var\n".to_string(),
    }
}

// ── Todoist ──────────────────────────────────────────────────────────────────

struct TodoistClient {
    api_key: String,
    client: reqwest::Client,
}

impl TodoistClient {
    fn from_env_or_config() -> Option<Self> {
        let key = std::env::var("TODOIST_API_KEY").ok()
            .or_else(|| {
                crate::config::Config::load().ok()
                    .and_then(|c| c.todoist_api_key)
            })?;
        if key.is_empty() { return None; }
        Some(Self { api_key: key, client: build_client() })
    }

    async fn list_tasks(&self, filter: &str) -> Result<Vec<TodoistTask>> {
        let url = if filter.is_empty() {
            format!("{}/tasks", TODOIST_API)
        } else {
            format!("{}/tasks?filter={}", TODOIST_API, urlencoding::encode(filter))
        };
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let resp = retry_async(&RetryConfig::default(), "todoist-list", || {
            let client = client.clone();
            let api_key = api_key.clone();
            let url = url.clone();
            async move {
                client.get(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        let data: Vec<serde_json::Value> = resp.json().await?;
        Ok(data.iter().map(|t| TodoistTask {
            id: t["id"].as_str().unwrap_or("").to_string(),
            content: t["content"].as_str().unwrap_or("").to_string(),
            description: t["description"].as_str().unwrap_or("").to_string(),
            due: t["due"]["string"].as_str().map(String::from),
            priority: t["priority"].as_u64().unwrap_or(1) as u8,
            project_id: t["project_id"].as_str().map(String::from),
            is_completed: t["is_completed"].as_bool().unwrap_or(false),
        }).collect())
    }

    async fn add_task(&self, content: &str) -> Result<TodoistTask> {
        self.add_task_full(content, None, None).await
    }

    async fn add_task_full(
        &self,
        content: &str,
        due_string: Option<&str>,
        priority: Option<u8>,
    ) -> Result<TodoistTask> {
        let mut payload = serde_json::json!({ "content": content });
        if let Some(d) = due_string.filter(|s| !s.is_empty()) {
            payload["due_string"] = serde_json::Value::String(d.to_string());
        }
        if let Some(p) = priority.filter(|p| (1..=4).contains(p)) {
            payload["priority"] = serde_json::Value::Number(p.into());
        }
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let resp = retry_async(&RetryConfig::default(), "todoist-add", || {
            let client = client.clone();
            let api_key = api_key.clone();
            let payload = payload.clone();
            async move {
                client.post(format!("{}/tasks", TODOIST_API))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&payload)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        let t: serde_json::Value = resp.json().await?;
        Ok(TodoistTask {
            id: t["id"].as_str().unwrap_or("").to_string(),
            content: t["content"].as_str().unwrap_or("").to_string(),
            description: String::new(),
            due: t["due"]["string"].as_str().map(String::from),
            priority: t["priority"].as_u64().unwrap_or(1) as u8,
            project_id: t["project_id"].as_str().map(String::from),
            is_completed: false,
        })
    }

    async fn close_task(&self, id: &str) -> Result<()> {
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let url = format!("{}/tasks/{}/close", TODOIST_API, id);
        retry_async(&RetryConfig::default(), "todoist-close", || {
            let client = client.clone();
            let api_key = api_key.clone();
            let url = url.clone();
            async move {
                client.post(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;
        Ok(())
    }
}

async fn handle_todoist(args: &str) -> String {
    let client = match TodoistClient::from_env_or_config() {
        Some(c) => c,
        None => return "⚠️  Todoist not configured. Set TODOIST_API_KEY env var.\n".to_string(),
    };
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let subcmd = parts.first().copied().unwrap_or("list");
    let rest = parts.get(1).copied().unwrap_or("");

    match subcmd {
        "" | "list" => {
            match client.list_tasks("").await {
                Ok(tasks) => {
                    if tasks.is_empty() { return "📭 No active tasks!\n".to_string(); }
                    let mut out = format!("📋 Todoist Tasks ({} active)\n{}\n", tasks.len(), "─".repeat(50));
                    for t in &tasks {
                        let due = t.due.as_deref().unwrap_or("");
                        out.push_str(&format!("  {} [{}] {}  {}\n", t.priority_icon(), &t.id[..t.id.len().min(8)], t.content, due));
                    }
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "today" => {
            match client.list_tasks("today").await {
                Ok(tasks) => {
                    if tasks.is_empty() { return "📭 Nothing due today!\n".to_string(); }
                    let mut out = format!("📋 Due Today ({} tasks)\n", tasks.len());
                    for t in &tasks { out.push_str(&format!("  {} {}\n", t.priority_icon(), t.content)); }
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "add" | "new" => {
            if rest.is_empty() { return "Usage: /todo add <task description>\n".to_string(); }
            match client.add_task(rest).await {
                Ok(t) => format!("➕ Task added: {} (id: {})\n", t.content, t.id),
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "complete" | "done" | "close" => {
            if rest.is_empty() { return "Usage: /todo complete <task_id>\n".to_string(); }
            match client.close_task(rest).await {
                Ok(()) => format!("☑️  Task {} completed!\n", rest),
                Err(e) => format!("❌ {}\n", e),
            }
        }
        _ => "📋 Todoist Commands:\n\
              /todo list            — List active tasks\n\
              /todo today           — Tasks due today\n\
              /todo add <task>      — Add a task\n\
              /todo complete <id>   — Complete a task\n\n\
            Config: Set TODOIST_API_KEY env var\n".to_string(),
    }
}

// ── Jira ─────────────────────────────────────────────────────────────────────

struct JiraClient {
    base_url: String,
    auth: String, // base64(email:token)
    client: reqwest::Client,
}

impl JiraClient {
    fn from_env_or_config() -> Option<Self> {
        let url = std::env::var("JIRA_URL").ok()
            .or_else(|| {
                crate::config::Config::load().ok()
                    .and_then(|c| c.jira.as_ref().and_then(|j| j.url.clone()))
            })?;
        let email = std::env::var("JIRA_EMAIL").ok()
            .or_else(|| {
                crate::config::Config::load().ok()
                    .and_then(|c| c.jira.as_ref().and_then(|j| j.email.clone()))
            })?;
        let token = std::env::var("JIRA_API_TOKEN").ok()
            .or_else(|| {
                crate::config::Config::load().ok()
                    .and_then(|c| c.jira.as_ref().and_then(|j| j.api_token.clone()))
            })?;

        use base64::Engine;
        let auth = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", email, token));
        Some(Self { base_url: url.trim_end_matches('/').to_string(), auth, client: build_client() })
    }

    async fn list_issues(&self) -> Result<Vec<JiraIssue>> {
        let jql = "assignee=currentUser() AND statusCategory!=Done ORDER BY updated DESC";
        let url = format!("{}/rest/api/3/search?jql={}&maxResults=20", self.base_url, urlencoding::encode(jql));
        let auth = self.auth.clone();
        let client = self.client.clone();
        let resp = retry_async(&RetryConfig::default(), "jira-list", || {
            let client = client.clone();
            let auth = auth.clone();
            let url = url.clone();
            async move {
                client.get(&url)
                    .header("Authorization", format!("Basic {}", auth))
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        let data: serde_json::Value = resp.json().await?;
        let issues = data["issues"].as_array()
            .map(|arr| arr.iter().map(|i| {
                let fields = &i["fields"];
                JiraIssue {
                    key: i["key"].as_str().unwrap_or("").to_string(),
                    summary: fields["summary"].as_str().unwrap_or("").to_string(),
                    status: fields["status"]["name"].as_str().unwrap_or("").to_string(),
                    priority: fields["priority"]["name"].as_str().unwrap_or("").to_string(),
                    assignee: fields["assignee"]["displayName"].as_str().map(String::from),
                    issue_type: fields["issuetype"]["name"].as_str().unwrap_or("").to_string(),
                    url: format!("{}/browse/{}", self.base_url, i["key"].as_str().unwrap_or("")),
                }
            }).collect())
            .unwrap_or_default();
        Ok(issues)
    }

    async fn create_issue(&self, project: &str, summary: &str) -> Result<JiraIssue> {
        let payload = serde_json::json!({
            "fields": {
                "project": { "key": project },
                "summary": summary,
                "issuetype": { "name": "Task" }
            }
        });
        let url = format!("{}/rest/api/3/issue", self.base_url);
        let auth = self.auth.clone();
        let client = self.client.clone();
        let resp = retry_async(&RetryConfig::default(), "jira-create", || {
            let client = client.clone();
            let auth = auth.clone();
            let url = url.clone();
            let payload = payload.clone();
            async move {
                client.post(&url)
                    .header("Authorization", format!("Basic {}", auth))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;

        let data: serde_json::Value = resp.json().await?;
        let key = data["key"].as_str().unwrap_or("").to_string();
        Ok(JiraIssue {
            key: key.clone(),
            summary: summary.to_string(),
            status: "To Do".to_string(),
            priority: "Medium".to_string(),
            assignee: None,
            issue_type: "Task".to_string(),
            url: format!("{}/browse/{}", self.base_url, key),
        })
    }

    async fn add_comment(&self, key: &str, text: &str) -> Result<()> {
        let payload = serde_json::json!({
            "body": {
                "type": "doc", "version": 1,
                "content": [{"type": "paragraph", "content": [{"type": "text", "text": text}]}]
            }
        });
        let url = format!("{}/rest/api/3/issue/{}/comment", self.base_url, key);
        let auth = self.auth.clone();
        let client = self.client.clone();
        retry_async(&RetryConfig::default(), "jira-comment", || {
            let client = client.clone();
            let auth = auth.clone();
            let url = url.clone();
            let payload = payload.clone();
            async move {
                client.post(&url)
                    .header("Authorization", format!("Basic {}", auth))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(Into::into)
            }
        }).await?;
        Ok(())
    }
}

async fn handle_jira(args: &str) -> String {
    let client = match JiraClient::from_env_or_config() {
        Some(c) => c,
        None => return "⚠️  Jira not configured. Set JIRA_URL, JIRA_EMAIL, JIRA_API_TOKEN env vars.\n".to_string(),
    };
    let parts: Vec<&str> = args.splitn(3, ' ').collect();
    let subcmd = parts.first().copied().unwrap_or("list");
    let arg1 = parts.get(1).copied().unwrap_or("");
    let arg2 = parts.get(2).copied().unwrap_or("");

    match subcmd {
        "" | "list" => {
            match client.list_issues().await {
                Ok(issues) => {
                    if issues.is_empty() { return "🎫 No assigned issues.\n".to_string(); }
                    let mut out = format!("🎫 Jira Issues ({} assigned)\n{}\n", issues.len(), "─".repeat(60));
                    for i in &issues {
                        out.push_str(&format!("  🎫 {:<12} {:<14} {}\n", i.key, i.status, i.summary));
                    }
                    out
                }
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "create" | "new" => {
            if arg1.is_empty() || arg2.is_empty() {
                return "Usage: /jira create <PROJECT_KEY> <summary>\n".to_string();
            }
            match client.create_issue(arg1, arg2).await {
                Ok(issue) => format!("🎫 Created: {} — {}\n   {}\n", issue.key, issue.summary, issue.url),
                Err(e) => format!("❌ {}\n", e),
            }
        }
        "comment" => {
            if arg1.is_empty() || arg2.is_empty() {
                return "Usage: /jira comment <ISSUE_KEY> <text>\n".to_string();
            }
            match client.add_comment(arg1, arg2).await {
                Ok(()) => format!("💬 Comment added to {}\n", arg1),
                Err(e) => format!("❌ {}\n", e),
            }
        }
        _ => "🎫 Jira Commands:\n\
              /jira list                    — List assigned issues\n\
              /jira create <proj> <summary> — Create issue\n\
              /jira comment <key> <text>    — Add comment\n\n\
            Config: Set JIRA_URL, JIRA_EMAIL, JIRA_API_TOKEN env vars\n".to_string(),
    }
}

// ── Unified dispatcher ───────────────────────────────────────────────────────

pub async fn handle_productivity_command(args: &str) -> String {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let tool = parts.first().copied().unwrap_or("");
    let rest = parts.get(1).copied().unwrap_or("");

    match tool {
        "notion" => handle_notion(rest).await,
        "todo" | "todoist" => handle_todoist(rest).await,
        "jira" => handle_jira(rest).await,
        _ => "Productivity Commands:\n\
              /notion search|page     — Notion integration\n\
              /todo list|add|complete — Todoist task management\n\
              /jira list|create       — Jira issue tracking\n".to_string(),
    }
}

// ── UI wrapper API (for typed Tauri commands) ───────────────────────────────

use crate::email_client::ProviderStatus;

// Notion
pub async fn ui_notion_search(query: &str) -> std::result::Result<Vec<NotionPage>, String> {
    let c = NotionClient::from_env_or_config()
        .ok_or_else(|| "Notion not configured. Set NOTION_API_KEY.".to_string())?;
    c.search(query).await.map_err(|e| e.to_string())
}

pub async fn ui_notion_page(id: &str) -> std::result::Result<String, String> {
    let c = NotionClient::from_env_or_config()
        .ok_or_else(|| "Notion not configured. Set NOTION_API_KEY.".to_string())?;
    c.get_page(id).await.map_err(|e| e.to_string())
}

pub async fn ui_notion_append(page_id: &str, text: &str) -> std::result::Result<(), String> {
    let c = NotionClient::from_env_or_config()
        .ok_or_else(|| "Notion not configured. Set NOTION_API_KEY.".to_string())?;
    c.append_text(page_id, text).await.map_err(|e| e.to_string())
}

pub async fn ui_notion_status() -> ProviderStatus {
    let Some(c) = NotionClient::from_env_or_config() else {
        return ProviderStatus {
            connected: false,
            provider: None,
            account: None,
            message: Some("Not signed in".to_string()),
        };
    };
    // Probe /users/me to confirm the token works and fetch an identifier.
    let resp = c
        .client
        .get(format!("{}/users/me", NOTION_API))
        .header("Authorization", format!("Bearer {}", c.api_key))
        .header("Notion-Version", "2022-06-28")
        .send()
        .await;
    match resp {
        Ok(r) if r.status().is_success() => {
            let v: serde_json::Value = r.json().await.unwrap_or_default();
            let name = v["name"]
                .as_str()
                .or_else(|| v["bot"]["owner"]["user"]["name"].as_str())
                .map(String::from);
            ProviderStatus {
                connected: true,
                provider: Some("notion".to_string()),
                account: name,
                message: None,
            }
        }
        Ok(r) => ProviderStatus {
            connected: false,
            provider: Some("notion".to_string()),
            account: None,
            message: Some(format!("HTTP {}", r.status())),
        },
        Err(e) => ProviderStatus {
            connected: false,
            provider: Some("notion".to_string()),
            account: None,
            message: Some(e.to_string()),
        },
    }
}

// Todoist
pub async fn ui_todoist_list(filter: &str) -> std::result::Result<Vec<TodoistTask>, String> {
    let c = TodoistClient::from_env_or_config()
        .ok_or_else(|| "Todoist not configured. Set TODOIST_API_KEY.".to_string())?;
    c.list_tasks(filter).await.map_err(|e| e.to_string())
}

pub async fn ui_todoist_add(content: &str) -> std::result::Result<TodoistTask, String> {
    let c = TodoistClient::from_env_or_config()
        .ok_or_else(|| "Todoist not configured. Set TODOIST_API_KEY.".to_string())?;
    c.add_task(content).await.map_err(|e| e.to_string())
}

pub async fn ui_todoist_add_full(
    content: &str,
    due: Option<&str>,
    priority: Option<u8>,
) -> std::result::Result<TodoistTask, String> {
    let c = TodoistClient::from_env_or_config()
        .ok_or_else(|| "Todoist not configured. Set TODOIST_API_KEY.".to_string())?;
    c.add_task_full(content, due, priority)
        .await
        .map_err(|e| e.to_string())
}

pub async fn ui_todoist_close(id: &str) -> std::result::Result<(), String> {
    let c = TodoistClient::from_env_or_config()
        .ok_or_else(|| "Todoist not configured. Set TODOIST_API_KEY.".to_string())?;
    c.close_task(id).await.map_err(|e| e.to_string())
}

pub async fn ui_todoist_status() -> ProviderStatus {
    let Some(c) = TodoistClient::from_env_or_config() else {
        return ProviderStatus {
            connected: false,
            provider: None,
            account: None,
            message: Some("Not signed in".to_string()),
        };
    };
    // Probe by listing projects (small payload).
    let resp = c
        .client
        .get(format!("{}/projects", TODOIST_API))
        .header("Authorization", format!("Bearer {}", c.api_key))
        .send()
        .await;
    match resp {
        Ok(r) if r.status().is_success() => ProviderStatus {
            connected: true,
            provider: Some("todoist".to_string()),
            account: None,
            message: None,
        },
        Ok(r) => ProviderStatus {
            connected: false,
            provider: Some("todoist".to_string()),
            account: None,
            message: Some(format!("HTTP {}", r.status())),
        },
        Err(e) => ProviderStatus {
            connected: false,
            provider: Some("todoist".to_string()),
            account: None,
            message: Some(e.to_string()),
        },
    }
}

// Jira
pub async fn ui_jira_list() -> std::result::Result<Vec<JiraIssue>, String> {
    let c = JiraClient::from_env_or_config().ok_or_else(|| {
        "Jira not configured. Set JIRA_URL, JIRA_EMAIL, JIRA_API_TOKEN.".to_string()
    })?;
    c.list_issues().await.map_err(|e| e.to_string())
}

pub async fn ui_jira_create(
    project: &str,
    summary: &str,
) -> std::result::Result<JiraIssue, String> {
    let c = JiraClient::from_env_or_config().ok_or_else(|| {
        "Jira not configured. Set JIRA_URL, JIRA_EMAIL, JIRA_API_TOKEN.".to_string()
    })?;
    c.create_issue(project, summary).await.map_err(|e| e.to_string())
}

pub async fn ui_jira_comment(key: &str, text: &str) -> std::result::Result<(), String> {
    let c = JiraClient::from_env_or_config().ok_or_else(|| {
        "Jira not configured. Set JIRA_URL, JIRA_EMAIL, JIRA_API_TOKEN.".to_string()
    })?;
    c.add_comment(key, text).await.map_err(|e| e.to_string())
}

pub async fn ui_jira_status() -> ProviderStatus {
    let Some(c) = JiraClient::from_env_or_config() else {
        return ProviderStatus {
            connected: false,
            provider: None,
            account: None,
            message: Some("Not signed in".to_string()),
        };
    };
    let url = format!("{}/rest/api/3/myself", c.base_url);
    let resp = c
        .client
        .get(&url)
        .header("Authorization", format!("Basic {}", c.auth))
        .send()
        .await;
    match resp {
        Ok(r) if r.status().is_success() => {
            let v: serde_json::Value = r.json().await.unwrap_or_default();
            let account = v["displayName"]
                .as_str()
                .or_else(|| v["emailAddress"].as_str())
                .map(String::from);
            ProviderStatus {
                connected: true,
                provider: Some("jira".to_string()),
                account,
                message: None,
            }
        }
        Ok(r) => ProviderStatus {
            connected: false,
            provider: Some("jira".to_string()),
            account: None,
            message: Some(format!("HTTP {}", r.status())),
        },
        Err(e) => ProviderStatus {
            connected: false,
            provider: Some("jira".to_string()),
            account: None,
            message: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todoist_task_serialize() {
        let task = TodoistTask {
            id: "123".into(), content: "Buy milk".into(),
            description: "".into(), due: Some("2026-04-04".into()),
            priority: 4, project_id: None, is_completed: false,
        };
        let json = serde_json::to_string(&task).unwrap();
        let deser: TodoistTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.content, "Buy milk");
        assert_eq!(deser.priority_icon(), "🔥"); // API priority 4 = p1 (urgent)
    }

    #[test]
    fn test_jira_issue_serialize() {
        let issue = JiraIssue {
            key: "PROJ-123".into(), summary: "Fix bug".into(),
            status: "In Progress".into(), priority: "High".into(),
            assignee: Some("Alice".into()), issue_type: "Bug".into(),
            url: "https://jira.example.com/browse/PROJ-123".into(),
        };
        let json = serde_json::to_string(&issue).unwrap();
        assert!(json.contains("PROJ-123"));
    }

    #[test]
    fn test_todoist_priority_icons() {
        // Todoist REST API priority field: 4=p1 urgent (🔥), 3=p2 high (⚡), 2=p3 medium (📌), 1/other=p4 normal (·)
        for (p, icon) in [(4u8, "🔥"), (3u8, "⚡"), (2u8, "📌"), (1u8, "·")] {
            let t = TodoistTask {
                id: "x".into(), content: "t".into(), description: "".into(),
                due: None, priority: p, project_id: None, is_completed: false,
            };
            assert_eq!(t.priority_icon(), icon, "API priority {} should map to {}", p, icon);
        }
    }

    #[test]
    fn test_jira_issue_no_assignee() {
        let issue = JiraIssue {
            key: "X-1".into(), summary: "Unassigned".into(),
            status: "Open".into(), priority: "Low".into(),
            assignee: None, issue_type: "Task".into(),
            url: "https://jira.example.com/browse/X-1".into(),
        };
        let json = serde_json::to_string(&issue).unwrap();
        let deser: JiraIssue = serde_json::from_str(&json).unwrap();
        assert!(deser.assignee.is_none());
    }

    #[test]
    fn test_todoist_completed_task() {
        let t = TodoistTask {
            id: "done".into(), content: "Finished".into(), description: "".into(),
            due: None, priority: 4, project_id: Some("p1".into()), is_completed: true,
        };
        assert!(t.is_completed);
        let json = serde_json::to_string(&t).unwrap();
        let deser: TodoistTask = serde_json::from_str(&json).unwrap();
        assert!(deser.is_completed);
        assert_eq!(deser.project_id, Some("p1".into()));
    }

    #[tokio::test]
    async fn test_handle_productivity_no_config_notion() {
        let out = handle_productivity_command("notion search test").await;
        assert!(!out.is_empty());
    }

    #[tokio::test]
    async fn test_handle_productivity_no_config_todoist() {
        let out = handle_productivity_command("todoist list").await;
        assert!(!out.is_empty());
    }

    #[tokio::test]
    async fn test_handle_productivity_no_config_jira() {
        let out = handle_productivity_command("jira mine").await;
        assert!(!out.is_empty());
    }

    #[tokio::test]
    async fn test_handle_productivity_unknown_prefix() {
        let out = handle_productivity_command("unknown command").await;
        assert!(!out.is_empty());
    }

    #[tokio::test]
    async fn test_handle_productivity_empty_args() {
        let out = handle_productivity_command("").await;
        assert!(!out.is_empty());
    }
}
