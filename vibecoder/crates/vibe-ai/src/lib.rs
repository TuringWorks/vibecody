//! VibeCoder AI - AI provider abstraction and integrations

pub mod agent;
pub mod agent_team;
pub mod artifacts;
pub mod catalog;
pub mod chat;
pub mod config;
pub mod diffcomplete;
pub mod hooks;
pub mod mcp;
pub mod multi_agent;
pub mod otel;
pub mod planner;
pub mod policy;
pub mod provider;
pub mod providers;
pub mod resilience;
pub mod resilient;
pub mod rules;
pub mod skills;
pub mod tools;
pub mod trace;

pub use agent::{
    AgentContext, AgentEvent, AgentLoop, AgentStep, ApprovalPolicy, ToolExecutorTrait,
};
pub use artifacts::{AgentArtifact, Annotation, Artifact, ArtifactStore, ReviewIssueRef, TaskItem};
pub use chat::ChatEngine;
pub use config::AIConfig;
pub use hooks::{HookConfig, HookDecision, HookEvent, HookHandler, HookRunner};
pub use mcp::{McpClient, McpServerConfig, McpTool};
pub use multi_agent::{
    AgentInstance, AgentResult, AgentStatus, AgentTask, ExecutorFactory, IsolatedWorktree,
    MultiAgentOrchestrator, OrchestratorEvent, WorktreeManager,
};
pub use planner::{ExecutionPlan, PlanStep, PlanStepStatus, PlannerAgent};
pub use policy::{AdminPolicy, PolicyDecision};
pub use provider::{CodeContext, CompletionStream, Effort, ImageAttachment, Message, MessageRole};
pub use providers::{
    AzureOpenAIProvider, BedrockProvider, CerebrasProvider, CopilotProvider, DeepSeekProvider,
    FailoverProvider, GroqProvider, LocalEditProvider, MistralProvider, OpenRouterProvider,
    VercelAIProvider, ZhipuProvider,
};
pub use resilience::{
    add_jitter, classify_error, FailureCategory, FailureJournal, FailureJournalSummary,
    FailurePattern, FailureRecord, ProviderCallOutcome, ProviderHealth, ProviderHealthTracker,
    RecoveryPolicy, ResilienceConfig,
};
pub use resilient::{is_retryable, retry_async, ResilientProvider, RetryConfig};
pub use rules::{Rule, RulesLoader};
pub use skills::{Skill, SkillInstaller, SkillLoader, SkillWatcher};
pub use tools::{format_tool_result, parse_tool_calls, ToolCall, ToolResult, TOOL_SYSTEM_PROMPT};
pub use trace::{
    list_traces, load_eval_records, load_session, load_trace, DecisionTraceEntry, DecisionWriter,
    SessionSnapshot, SkillEvalRecord, TraceEntry, TraceSession, TraceWriter,
};
// Claw-code parity: reusable mock provider for deterministic CI testing
#[cfg(any(test, feature = "testing"))]
pub mod mock_provider;

pub use agent_team::{
    AgentTeam, TeamInfo, TeamMessage, TeamMessageBus, TeamMessageType, TeamSubTask, TeamTaskStatus,
};
