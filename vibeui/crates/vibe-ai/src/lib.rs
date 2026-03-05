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
pub mod hooks;
pub mod planner;
pub mod multi_agent;
pub mod skills;
pub mod artifacts;
pub mod otel;
pub mod policy;
pub mod rules;
pub mod agent_team;

pub use chat::ChatEngine;
pub use provider::{CodeContext, Message, MessageRole, CompletionStream, ImageAttachment};
pub use providers::{
    BedrockProvider, CopilotProvider, AzureOpenAIProvider, OpenRouterProvider, GroqProvider,
    LocalEditProvider, MistralProvider, CerebrasProvider, DeepSeekProvider, ZhipuProvider,
    VercelAIProvider, FailoverProvider,
};
pub use config::AIConfig;
pub use completion::CompletionEngine;
pub use tools::{ToolCall, ToolResult, parse_tool_calls, format_tool_result, TOOL_SYSTEM_PROMPT};
pub use agent::{
    AgentLoop, AgentEvent, AgentStep, AgentContext, ApprovalPolicy, ToolExecutorTrait,
};
pub use trace::{TraceWriter, TraceEntry, TraceSession, SessionSnapshot, list_traces, load_trace, load_session};
pub use mcp::{McpClient, McpTool, McpServerConfig};
pub use hooks::{HookConfig, HookDecision, HookEvent, HookHandler, HookRunner};
pub use planner::{ExecutionPlan, PlanStep, PlanStepStatus, PlannerAgent};
pub use multi_agent::{
    MultiAgentOrchestrator, AgentTask, AgentResult, AgentInstance,
    AgentStatus, OrchestratorEvent, ExecutorFactory, WorktreeManager, IsolatedWorktree,
};
pub use skills::{Skill, SkillLoader, SkillWatcher, SkillInstaller};
pub use artifacts::{Artifact, AgentArtifact, ArtifactStore, Annotation, TaskItem, ReviewIssueRef};
pub use policy::{AdminPolicy, PolicyDecision};
pub use rules::{Rule, RulesLoader};
pub use agent_team::{
    AgentTeam, TeamMessage, TeamMessageBus, TeamMessageType,
    TeamSubTask, TeamTaskStatus, TeamInfo,
};
