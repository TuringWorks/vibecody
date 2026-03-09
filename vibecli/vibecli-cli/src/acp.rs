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

    #[test]
    fn capabilities_protocol_version() {
        let caps = default_capabilities();
        assert_eq!(caps.protocol_version, "1.0");
    }

    #[test]
    fn capabilities_agent_name() {
        let caps = default_capabilities();
        assert_eq!(caps.agent_name, "VibeCody");
    }

    #[test]
    fn capabilities_contains_specific_tools() {
        let caps = default_capabilities();
        assert!(caps.supported_tools.contains(&"read_file".to_string()));
        assert!(caps.supported_tools.contains(&"write_file".to_string()));
        assert!(caps.supported_tools.contains(&"bash".to_string()));
        assert!(caps.supported_tools.contains(&"search_files".to_string()));
    }

    #[test]
    fn capabilities_contains_specific_models() {
        let caps = default_capabilities();
        assert!(caps.supported_models.contains(&"ollama".to_string()));
        assert!(caps.supported_models.contains(&"claude".to_string()));
        assert!(caps.supported_models.contains(&"openai".to_string()));
    }

    #[test]
    fn capabilities_contains_specific_features() {
        let caps = default_capabilities();
        assert!(caps.features.contains(&"agent_mode".to_string()));
        assert!(caps.features.contains(&"streaming".to_string()));
        assert!(caps.features.contains(&"mcp_client".to_string()));
    }

    #[test]
    fn task_request_with_approval_policy() {
        let json = r#"{"task":"deploy","approval_policy":"full-auto"}"#;
        let req: AcpTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.approval_policy, Some("full-auto".to_string()));
        assert!(req.model.is_none());
    }

    #[test]
    fn task_request_all_fields() {
        let json = r#"{
            "task": "review code",
            "context": {
                "files": [],
                "workspace_root": "/tmp",
                "language": "rust"
            },
            "model": "openai",
            "approval_policy": "auto-edit"
        }"#;
        let req: AcpTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task, "review code");
        assert_eq!(req.model, Some("openai".to_string()));
        assert_eq!(req.approval_policy, Some("auto-edit".to_string()));
        let ctx = req.context.unwrap();
        assert!(ctx.files.is_empty());
        assert_eq!(ctx.language, Some("rust".to_string()));
    }

    #[test]
    fn context_defaults_empty() {
        let json = r#"{"files":[]}"#;
        let ctx: AcpContext = serde_json::from_str(json).unwrap();
        assert!(ctx.files.is_empty());
        assert!(ctx.workspace_root.is_none());
        assert!(ctx.language.is_none());
    }

    #[test]
    fn file_context_with_selection() {
        let json = r#"{
            "path": "src/lib.rs",
            "content": "fn foo() {}",
            "selection": {"start_line": 1, "start_col": 0, "end_line": 1, "end_col": 11}
        }"#;
        let fc: AcpFileContext = serde_json::from_str(json).unwrap();
        assert_eq!(fc.path, "src/lib.rs");
        assert_eq!(fc.content, Some("fn foo() {}".to_string()));
        let sel = fc.selection.unwrap();
        assert_eq!(sel.start_line, 1);
        assert_eq!(sel.end_col, 11);
    }

    #[test]
    fn file_context_minimal() {
        let json = r#"{"path":"README.md"}"#;
        let fc: AcpFileContext = serde_json::from_str(json).unwrap();
        assert_eq!(fc.path, "README.md");
        assert!(fc.content.is_none());
        assert!(fc.selection.is_none());
    }

    #[test]
    fn task_status_no_summary() {
        let status = AcpTaskStatus {
            id: "task-1".into(),
            status: "pending".into(),
            summary: None,
            files_modified: vec![],
            steps_completed: 0,
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: AcpTaskStatus = serde_json::from_str(&json).unwrap();
        assert!(parsed.summary.is_none());
        assert!(parsed.files_modified.is_empty());
        assert_eq!(parsed.steps_completed, 0);
    }

    #[test]
    fn task_status_complete_with_files() {
        let status = AcpTaskStatus {
            id: "task-99".into(),
            status: "complete".into(),
            summary: Some("All done".into()),
            files_modified: vec!["a.rs".into(), "b.rs".into(), "c.rs".into()],
            steps_completed: 12,
        };
        assert_eq!(status.files_modified.len(), 3);
        assert_eq!(status.steps_completed, 12);
    }

    #[test]
    fn selection_serde_roundtrip() {
        let sel = AcpSelection {
            start_line: 10,
            start_col: 5,
            end_line: 20,
            end_col: 30,
        };
        let json = serde_json::to_string(&sel).unwrap();
        let parsed: AcpSelection = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.start_line, 10);
        assert_eq!(parsed.start_col, 5);
        assert_eq!(parsed.end_line, 20);
        assert_eq!(parsed.end_col, 30);
    }

    #[test]
    fn capabilities_clone() {
        let caps = default_capabilities();
        let cloned = caps.clone();
        assert_eq!(cloned.protocol_version, caps.protocol_version);
        assert_eq!(cloned.supported_tools.len(), caps.supported_tools.len());
    }
}
