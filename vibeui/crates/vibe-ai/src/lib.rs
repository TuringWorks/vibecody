//! VibeUI AI - AI provider abstraction and integrations

pub mod provider;
pub mod completion;
pub mod chat;
pub mod providers;
pub mod config;
pub mod tools;
pub mod agent;
pub mod trace;
pub mod mcp;

pub use chat::ChatEngine;
pub use provider::{CodeContext, Message, MessageRole, CompletionStream, ImageAttachment};
pub use config::AIConfig;
pub use completion::CompletionEngine;
pub use tools::{ToolCall, ToolResult, parse_tool_calls, format_tool_result, TOOL_SYSTEM_PROMPT};
pub use agent::{
    AgentLoop, AgentEvent, AgentStep, AgentContext, ApprovalPolicy, ToolExecutorTrait,
};
pub use trace::{TraceWriter, TraceEntry, TraceSession, list_traces, load_trace};
pub use mcp::{McpClient, McpTool, McpServerConfig};
