//! Autonomous agent loop with configurable approval policy.
//!
//! The agent interleaves LLM streaming responses with tool execution,
//! repeating until the model calls `task_complete` or `max_steps` is reached.

use crate::provider::{AIProvider, Message, MessageRole};
use crate::tools::{format_tool_result, parse_tool_calls, ToolCall, ToolResult, TOOL_SYSTEM_PROMPT};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

// ── Approval Policy ───────────────────────────────────────────────────────────

/// Governs how the agent handles potentially destructive tool calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalPolicy {
    /// Show each tool call to the user and wait for y/n/a approval.
    Suggest,
    /// Auto-apply file operations; require approval only for bash commands.
    AutoEdit,
    /// Execute all tool calls automatically without prompting.
    FullAuto,
}

impl ApprovalPolicy {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "full-auto" | "fullauto" => Self::FullAuto,
            "auto-edit" | "autoedit" => Self::AutoEdit,
            _ => Self::Suggest,
        }
    }
}

// ── Agent Step ────────────────────────────────────────────────────────────────

/// A completed step in the agent loop.
#[derive(Debug, Clone)]
pub struct AgentStep {
    pub step_num: usize,
    pub tool_call: ToolCall,
    pub tool_result: ToolResult,
    pub approved: bool,
}

// ── Agent Events ──────────────────────────────────────────────────────────────

/// Events emitted by the agent loop to the UI or REPL.
pub enum AgentEvent {
    /// A streaming chunk from the LLM.
    StreamChunk(String),
    /// A tool call requiring approval.
    /// The caller must execute the tool and send `Some(result)` to approve,
    /// or `None` to reject.
    ToolCallPending {
        call: ToolCall,
        result_tx: oneshot::Sender<Option<ToolResult>>,
    },
    /// A tool call was auto-executed (AutoEdit / FullAuto mode).
    ToolCallExecuted(AgentStep),
    /// The agent has completed the task.
    Complete(String),
    /// An unrecoverable error occurred.
    Error(String),
}

// ── Agent Context ─────────────────────────────────────────────────────────────

/// Environmental context injected at agent startup.
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub workspace_root: PathBuf,
    pub open_files: Vec<String>,
    pub git_branch: Option<String>,
    pub git_diff_summary: Option<String>,
}

// ── Tool Executor Trait ───────────────────────────────────────────────────────

/// Decouples the agent loop (in `vibe-ai`) from the concrete executor
/// (in `vibecli-cli`). Implement this trait and pass it to [`AgentLoop::new`].
#[async_trait]
pub trait ToolExecutorTrait: Send + Sync {
    async fn execute(&self, call: &ToolCall) -> ToolResult;
}

// ── Agent Loop ────────────────────────────────────────────────────────────────

/// Runs the plan→act→observe cycle until the task is complete.
pub struct AgentLoop {
    pub provider: Arc<dyn AIProvider>,
    pub approval: ApprovalPolicy,
    pub max_steps: usize,
    pub executor: Arc<dyn ToolExecutorTrait>,
}

impl AgentLoop {
    pub fn new(
        provider: Arc<dyn AIProvider>,
        approval: ApprovalPolicy,
        executor: Arc<dyn ToolExecutorTrait>,
    ) -> Self {
        Self {
            provider,
            approval,
            max_steps: 30,
            executor,
        }
    }

    /// Run the agent for `task`, emitting [`AgentEvent`]s via `event_tx`.
    pub async fn run(
        &self,
        task: &str,
        context: AgentContext,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<()> {
        let system_content = build_system_prompt(&context);
        let mut messages: Vec<Message> = vec![
            Message { role: MessageRole::System, content: system_content },
            Message { role: MessageRole::User, content: task.to_string() },
        ];

        for step in 0..self.max_steps {
            // ── 1. Stream LLM response ────────────────────────────────────────
            let mut stream = match self.provider.stream_chat(&messages).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = event_tx.send(AgentEvent::Error(e.to_string())).await;
                    return Err(e);
                }
            };

            let mut accumulated = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(text) => {
                        let _ = event_tx.send(AgentEvent::StreamChunk(text.clone())).await;
                        accumulated.push_str(&text);
                    }
                    Err(e) => {
                        let _ = event_tx.send(AgentEvent::Error(e.to_string())).await;
                        return Err(e);
                    }
                }
            }

            // ── 2. Parse tool calls ───────────────────────────────────────────
            let tool_calls = parse_tool_calls(&accumulated);
            if tool_calls.is_empty() {
                // Model responded with prose — treat as final answer.
                let _ = event_tx.send(AgentEvent::Complete(accumulated)).await;
                return Ok(());
            }

            messages.push(Message {
                role: MessageRole::Assistant,
                content: accumulated.clone(),
            });

            // ── 3. Handle first tool call (one tool per turn) ─────────────────
            let call = tool_calls.into_iter().next().unwrap();
            if call.is_terminal() {
                let summary = match &call {
                    ToolCall::TaskComplete { summary } => summary.clone(),
                    _ => "Task complete.".to_string(),
                };
                let _ = event_tx.send(AgentEvent::Complete(summary)).await;
                return Ok(());
            }

            let needs_approval = self.needs_approval(&call);
            let tool_result = if needs_approval {
                let (result_tx, result_rx) = oneshot::channel();
                if event_tx
                    .send(AgentEvent::ToolCallPending { call: call.clone(), result_tx })
                    .await
                    .is_err()
                {
                    return Ok(()); // Receiver dropped — caller gone
                }
                match result_rx.await {
                    Ok(Some(result)) => result,
                    Ok(None) => ToolResult {
                        tool_name: call.name().to_string(),
                        output: "Tool call rejected by user.".to_string(),
                        success: false,
                        truncated: false,
                    },
                    Err(_) => return Ok(()), // Sender dropped
                }
            } else {
                // Auto-execute
                let result = self.executor.execute(&call).await;
                let _ = event_tx
                    .send(AgentEvent::ToolCallExecuted(AgentStep {
                        step_num: step,
                        tool_call: call.clone(),
                        tool_result: result.clone(),
                        approved: true,
                    }))
                    .await;
                result
            };

            // ── 4. Feed result back into conversation ─────────────────────────
            messages.push(Message {
                role: MessageRole::User,
                content: format_tool_result(&call, &tool_result),
            });
        }

        let _ = event_tx
            .send(AgentEvent::Error(format!(
                "Agent reached maximum step limit ({})",
                self.max_steps
            )))
            .await;
        Ok(())
    }

    fn needs_approval(&self, call: &ToolCall) -> bool {
        match &self.approval {
            ApprovalPolicy::FullAuto => false,
            ApprovalPolicy::AutoEdit => matches!(call, ToolCall::Bash { .. }),
            ApprovalPolicy::Suggest => true,
        }
    }
}

fn build_system_prompt(context: &AgentContext) -> String {
    let mut extras = String::new();
    if !context.workspace_root.as_os_str().is_empty() {
        extras.push_str(&format!(
            "\n\n## Environment\nWorkspace root: {}",
            context.workspace_root.display()
        ));
    }
    if let Some(branch) = &context.git_branch {
        extras.push_str(&format!("\nGit branch: {}", branch));
    }
    if let Some(diff) = &context.git_diff_summary {
        extras.push_str(&format!("\nGit diff summary:\n{}", diff));
    }
    format!("{}{}", TOOL_SYSTEM_PROMPT, extras)
}
