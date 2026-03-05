//! Agent Client Protocol (ACP) Support — open protocol for IDE-agnostic agent integration.
//!
//! ACP enables any IDE (Zed, JetBrains, VS Code, etc.) to use VibeCody as an agent backend.
//!
//! # Endpoints
//!
//! | Method | Path                        | Description                    |
//! |--------|-----------------------------|--------------------------------|
//! | GET    | `/acp/v1/capabilities`      | List supported capabilities    |
//! | POST   | `/acp/v1/tasks`             | Submit a new task              |
//! | GET    | `/acp/v1/tasks/:id`         | Get task status                |
//! | GET    | `/acp/v1/tasks/:id/events`  | SSE stream of task events      |

use serde::{Deserialize, Serialize};

/// ACP capability advertisement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpCapabilities {
    pub protocol_version: String,
    pub agent_name: String,
    pub agent_version: String,
    pub supported_tools: Vec<String>,
    pub supported_models: Vec<String>,
    pub features: Vec<String>,
}

/// ACP task submission request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpTaskRequest {
    pub task: String,
    #[serde(default)]
    pub context: Option<AcpContext>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub approval_policy: Option<String>,
}

/// Optional context for ACP tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpContext {
    #[serde(default)]
    pub files: Vec<AcpFileContext>,
    #[serde(default)]
    pub workspace_root: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
}

/// File context for ACP tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpFileContext {
    pub path: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub selection: Option<AcpSelection>,
}

/// Text selection range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSelection {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// ACP task status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpTaskStatus {
    pub id: String,
    pub status: String, // "pending" | "running" | "complete" | "failed"
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub files_modified: Vec<String>,
    #[serde(default)]
    pub steps_completed: usize,
}

/// Build the default capabilities.
pub fn default_capabilities() -> AcpCapabilities {
    AcpCapabilities {
        protocol_version: "1.0".to_string(),
        agent_name: "VibeCody".to_string(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        supported_tools: vec![
            "read_file".into(),
            "write_file".into(),
            "list_directory".into(),
            "bash".into(),
            "search_files".into(),
            "web_search".into(),
            "fetch_url".into(),
        ],
        supported_models: vec![
            "ollama".into(),
            "openai".into(),
            "claude".into(),
            "gemini".into(),
            "grok".into(),
            "groq".into(),
            "openrouter".into(),
            "bedrock".into(),
            "copilot".into(),
        ],
        features: vec![
            "agent_mode".into(),
            "code_review".into(),
            "multi_agent".into(),
            "plan_mode".into(),
            "streaming".into(),
            "session_resume".into(),
            "mcp_client".into(),
            "hooks".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_serde() {
        let caps = default_capabilities();
        let json = serde_json::to_string(&caps).unwrap();
        let parsed: AcpCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.protocol_version, "1.0");
        assert_eq!(parsed.agent_name, "VibeCody");
    }

    #[test]
    fn task_request_minimal() {
        let json = r#"{"task":"fix the bug"}"#;
        let req: AcpTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "fix the bug");
        assert!(req.context.is_none());
    }

    #[test]
    fn task_request_with_context() {
        let json = r#"{
            "task": "refactor",
            "context": {
                "files": [{"path": "src/main.rs", "content": "fn main() {}"}],
                "workspace_root": "/tmp/project"
            },
            "model": "claude"
        }"#;
        let req: AcpTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "refactor");
        assert_eq!(req.context.unwrap().files.len(), 1);
        assert_eq!(req.model, Some("claude".into()));
    }

    #[test]
    fn task_status_serde() {
        let status = AcpTaskStatus {
            id: "abc-123".into(),
            status: "running".into(),
            summary: Some("Working on it".into()),
            files_modified: vec!["main.rs".into()],
            steps_completed: 5,
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: AcpTaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.steps_completed, 5);
    }

    #[test]
    fn default_capabilities_has_tools() {
        let caps = default_capabilities();
        assert!(caps.supported_tools.len() >= 5);
        assert!(caps.supported_models.len() >= 5);
        assert!(caps.features.len() >= 5);
    }
}
