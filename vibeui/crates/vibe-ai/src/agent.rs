//! Autonomous agent loop with configurable approval policy.
//!
//! The agent interleaves LLM streaming responses with tool execution,
//! repeating until the model calls `task_complete` or `max_steps` is reached.

use crate::hooks::{HookDecision, HookEvent, HookRunner};
use crate::otel;
use crate::policy::AdminPolicy;
use crate::provider::{AIProvider, Message, MessageRole};
use crate::skills::SkillLoader;
use crate::tools::{format_tool_result, parse_tool_calls, ToolCall, ToolResult, TOOL_SYSTEM_PROMPT};
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::Instrument;

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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentContext {
    pub workspace_root: PathBuf,
    pub open_files: Vec<String>,
    pub git_branch: Option<String>,
    pub git_diff_summary: Option<String>,
    /// Recent developer activity (from flow tracker) — injected into system prompt.
    pub flow_context: Option<String>,
    /// Pre-approved plan text — injected into system prompt when Plan Mode is used.
    pub approved_plan: Option<String>,
    /// Extra skill directories to search (e.g. from installed plugins).
    #[serde(default)]
    pub extra_skill_dirs: Vec<std::path::PathBuf>,
    /// Session ID of the parent agent (`None` for root agents).
    #[serde(default)]
    pub parent_session_id: Option<String>,
    /// Current nesting depth (0 for root agents).
    #[serde(default)]
    pub depth: u32,
    /// Shared counter of total active agents across the tree (runtime only).
    #[serde(skip)]
    pub active_agent_counter: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
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
    /// Optional hook runner for intercepting agent events.
    pub hooks: Option<Arc<HookRunner>>,
    /// Admin policy loaded from `.vibecli/policy.toml`.
    pub policy: AdminPolicy,
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
            hooks: None,
            policy: AdminPolicy::default(),
        }
    }

    /// Attach a hook runner to this agent loop.
    pub fn with_hooks(mut self, runner: HookRunner) -> Self {
        self.hooks = Some(Arc::new(runner));
        self
    }

    /// Load and apply an admin policy from the workspace root.
    pub fn with_policy(mut self, workspace_root: &std::path::Path) -> Self {
        self.policy = AdminPolicy::load(workspace_root);
        // Policy can restrict max_steps
        self.max_steps = self.policy.effective_max_steps(self.max_steps);
        self
    }

    /// Apply a pre-built admin policy.
    pub fn with_policy_direct(mut self, policy: AdminPolicy) -> Self {
        self.max_steps = policy.effective_max_steps(self.max_steps);
        self.policy = policy;
        self
    }

    /// Run the agent for `task`, emitting [`AgentEvent`]s via `event_tx`.
    pub async fn run(
        &self,
        task: &str,
        context: AgentContext,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<()> {
        let session_id = format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        // ── Root session span ─────────────────────────────────────────────────
        let session_span = tracing::info_span!(
            "agent.session",
            session_id = %session_id,
            task = %otel::truncate_task(task, 200),
        );

        self.run_inner(task, context, event_tx, session_id)
            .instrument(session_span)
            .await
    }

    async fn run_inner(
        &self,
        task: &str,
        context: AgentContext,
        event_tx: mpsc::Sender<AgentEvent>,
        session_id: String,
    ) -> Result<()> {
        // Fire SessionStart hook (non-blocking, best-effort)
        if let Some(hooks) = &self.hooks {
            let _hook_span = tracing::info_span!(
                "agent.hook",
                event = "SessionStart",
                session_id = %session_id,
            );
            hooks.run(&HookEvent::SessionStart { session_id: session_id.clone() }).await;
        }

        let system_content = build_system_prompt(&context);
        let mut messages: Vec<Message> = vec![
            Message { role: MessageRole::System, content: system_content },
        ];

        // Fire UserPromptSubmit hook — can block or inject extra context.
        let user_content = if let Some(hooks) = &self.hooks {
            match hooks.run(&HookEvent::UserPromptSubmit {
                prompt: task.to_string(),
                session_id: session_id.clone(),
            }).await {
                HookDecision::Block { reason } => {
                    tracing::info!(reason = %reason, "UserPromptSubmit blocked by hook");
                    let _ = event_tx.send(AgentEvent::Error(
                        format!("Task blocked by hook: {}", reason)
                    )).await;
                    return Ok(());
                }
                HookDecision::InjectContext { text } => {
                    format!("{}\n\n[Hook context]\n{}", task, text)
                }
                HookDecision::Allow => task.to_string(),
            }
        } else {
            task.to_string()
        };
        messages.push(Message { role: MessageRole::User, content: user_content });

        for step in 0..self.max_steps {
            // ── 1. Stream LLM response ────────────────────────────────────────
            let llm_span = tracing::info_span!(
                "agent.llm_call",
                step = step,
                message_count = messages.len(),
            );
            let mut accumulated = String::new();
            {
                let _guard = llm_span.enter();
                let mut stream = match self.provider.stream_chat(&messages).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(error = %e, "LLM call failed");
                        let _ = event_tx.send(AgentEvent::Error(e.to_string())).await;
                        return Err(e);
                    }
                };

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(text) => {
                            let _ = event_tx.send(AgentEvent::StreamChunk(text.clone())).await;
                            accumulated.push_str(&text);
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "LLM stream error");
                            let _ = event_tx.send(AgentEvent::Error(e.to_string())).await;
                            return Err(e);
                        }
                    }
                }
                tracing::debug!(response_len = accumulated.len(), "LLM response complete");
            }

            // ── 2. Parse tool calls ───────────────────────────────────────────
            let tool_calls = parse_tool_calls(&accumulated);
            if tool_calls.is_empty() {
                // Model responded with prose — treat as final answer.
                // Fire Stop hook
                if let Some(hooks) = &self.hooks {
                    let _hook_span = tracing::info_span!(
                        "agent.hook",
                        event = "Stop",
                        reason = "prose_response",
                        session_id = %session_id,
                    );
                    hooks.run(&HookEvent::Stop {
                        reason: "prose_response".to_string(),
                        session_id: session_id.clone(),
                    }).await;
                }
                let _ = event_tx.send(AgentEvent::Complete(accumulated)).await;
                return Ok(());
            }

            messages.push(Message {
                role: MessageRole::Assistant,
                content: accumulated.clone(),
            });

            // ── 3. Handle first tool call (one tool per turn) ─────────────────
            let call = match tool_calls.into_iter().next() {
                Some(c) => c,
                None => {
                    let _ = event_tx.send(AgentEvent::Complete(accumulated)).await;
                    return Ok(());
                }
            };
            if call.is_terminal() {
                let summary = match &call {
                    ToolCall::TaskComplete { summary } => summary.clone(),
                    _ => "Task complete.".to_string(),
                };
                // Fire TaskCompleted hook
                if let Some(hooks) = &self.hooks {
                    let _hook_span = tracing::info_span!(
                        "agent.hook",
                        event = "TaskCompleted",
                        session_id = %session_id,
                    );
                    hooks.run(&HookEvent::TaskCompleted {
                        summary: summary.clone(),
                        session_id: session_id.clone(),
                    }).await;
                }
                tracing::info!(step = step, summary = %summary, "Agent task complete");
                let _ = event_tx.send(AgentEvent::Complete(summary)).await;
                return Ok(());
            }

            // ── 3a. Admin policy check ────────────────────────────────────────
            match self.policy.check_tool(call.name()) {
                crate::policy::PolicyDecision::Block(reason) => {
                    tracing::warn!(tool = %call.name(), reason = %reason, "Tool call blocked by admin policy");
                    messages.push(Message {
                        role: MessageRole::User,
                        content: format!("❌ Tool call blocked by admin policy: {}", reason),
                    });
                    continue;
                }
                crate::policy::PolicyDecision::RequireApproval => {
                    // Policy overrides approval policy for this tool
                    tracing::info!(tool = %call.name(), "Admin policy requires approval for this tool");
                }
                crate::policy::PolicyDecision::Allow => {}
            }

            // ── 3b. PreToolUse hook ───────────────────────────────────────────
            if let Some(hooks) = &self.hooks {
                let _hook_span = tracing::info_span!(
                    "agent.hook",
                    event = "PreToolUse",
                    tool = %call.name(),
                    session_id = %session_id,
                );
                let decision = hooks.run(&HookEvent::PreToolUse {
                    call: call.clone(),
                    session_id: session_id.clone(),
                }).await;
                match decision {
                    HookDecision::Block { reason } => {
                        tracing::warn!(tool = %call.name(), reason = %reason, "Tool call blocked by hook");
                        // Tell the model the tool was blocked
                        messages.push(Message {
                            role: MessageRole::User,
                            content: format!("❌ Tool call blocked by hook: {}", reason),
                        });
                        continue;
                    }
                    HookDecision::InjectContext { text } => {
                        messages.push(Message {
                            role: MessageRole::User,
                            content: format!("[Hook context] {}", text),
                        });
                    }
                    HookDecision::Allow => {}
                }
            }

            // ── 3b. Execute tool call ─────────────────────────────────────────
            let step_span = tracing::info_span!(
                "agent.step",
                step_num = step,
                tool = %call.name(),
            );
            let needs_approval = self.needs_approval(&call);
            let tool_result = {
                let _guard = step_span.enter();
                if needs_approval {
                    let (result_tx, result_rx) = oneshot::channel();
                    if event_tx
                        .send(AgentEvent::ToolCallPending { call: call.clone(), result_tx })
                        .await
                        .is_err()
                    {
                        return Ok(()); // Receiver dropped — caller gone
                    }
                    match result_rx.await {
                        Ok(Some(result)) => {
                            tracing::debug!(
                                tool = %call.name(),
                                success = result.success,
                                "Tool call approved and executed",
                            );
                            result
                        }
                        Ok(None) => {
                            tracing::info!(tool = %call.name(), "Tool call rejected by user");
                            ToolResult {
                                tool_name: call.name().to_string(),
                                output: "Tool call rejected by user.".to_string(),
                                success: false,
                                truncated: false,
                            }
                        }
                        Err(_) => return Ok(()), // Sender dropped
                    }
                } else {
                    // Auto-execute
                    let result = self.executor.execute(&call).await;
                    tracing::debug!(
                        tool = %call.name(),
                        success = result.success,
                        truncated = result.truncated,
                        "Tool call auto-executed",
                    );
                    let _ = event_tx
                        .send(AgentEvent::ToolCallExecuted(AgentStep {
                            step_num: step,
                            tool_call: call.clone(),
                            tool_result: result.clone(),
                            approved: true,
                        }))
                        .await;
                    result
                }
            };

            // ── 3c. PostToolUse hook ──────────────────────────────────────────
            if let Some(hooks) = &self.hooks {
                let _hook_span = tracing::info_span!(
                    "agent.hook",
                    event = "PostToolUse",
                    tool = %call.name(),
                    tool_success = tool_result.success,
                    session_id = %session_id,
                );
                let decision = hooks.run(&HookEvent::PostToolUse {
                    call: call.clone(),
                    result: tool_result.clone(),
                    session_id: session_id.clone(),
                }).await;
                if let HookDecision::InjectContext { text } = decision {
                    messages.push(Message {
                        role: MessageRole::User,
                        content: format!("[Post-hook context] {}", text),
                    });
                }

                // ── 3d. File-event hooks (FileSaved / FileCreated) ────────────
                // Fire these after a successful WriteFile so hooks can react to
                // specific file patterns (e.g. auto-format on save, run tests).
                if tool_result.success {
                    let file_event = match &call {
                        ToolCall::WriteFile { path, content } => {
                            // Detect creation vs update by checking if file was
                            // readable before this write (best-effort: non-blocking).
                            let lang = path.rsplit('.').next().unwrap_or("").to_string();
                            Some(HookEvent::FileSaved {
                                path: path.clone(),
                                content: content.clone(),
                                language: lang,
                            })
                        }
                        ToolCall::Bash { command } if command.contains("mkdir") || command.contains("touch") => {
                            // Best-effort: if bash creates a file, fire FileCreated.
                            None // too ambiguous to infer path reliably
                        }
                        _ => None,
                    };

                    if let Some(ev) = file_event {
                        let _ = hooks.run(&ev).await;
                    }
                }
            }

            // ── 4. Feed result back into conversation ─────────────────────────
            messages.push(Message {
                role: MessageRole::User,
                content: format_tool_result(&call, &tool_result),
            });
        }

        tracing::warn!(max_steps = self.max_steps, "Agent reached maximum step limit");
        let _ = event_tx
            .send(AgentEvent::Error(format!(
                "Agent reached maximum step limit ({})",
                self.max_steps
            )))
            .await;
        Ok(())
    }

    fn needs_approval(&self, call: &ToolCall) -> bool {
        // Policy can force approval even in FullAuto mode
        if self.policy.requires_approval(call.name()) {
            return true;
        }
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

    // 6.5: Inject recent developer activity (flow context)
    if let Some(flow) = &context.flow_context {
        if !flow.is_empty() {
            extras.push_str(&format!("\n\n## Recent Developer Activity\n{}", flow));
        }
    }

    // 6.2: Inject approved execution plan if plan mode was used
    if let Some(plan) = &context.approved_plan {
        if !plan.is_empty() {
            extras.push_str(&format!(
                "\n\n## Approved Execution Plan\nThe user has reviewed and approved this plan. Follow it step by step:\n{}",
                plan
            ));
        }
    }

    // 8.1: Auto-activate skills whose triggers match the task or open files
    if !context.workspace_root.as_os_str().is_empty() {
        // Build a loader that covers workspace, global, and plugin skill dirs.
        let mut skill_dirs = vec![
            context.workspace_root.join(".vibecli").join("skills"),
        ];
        if let Ok(home) = std::env::var("HOME") {
            skill_dirs.push(std::path::PathBuf::from(home).join(".vibecli").join("skills"));
        }
        skill_dirs.extend(context.extra_skill_dirs.iter().cloned());
        let loader = SkillLoader::with_dirs(skill_dirs);
        // Match against open files list and any context text
        let context_text = context.open_files.join(" ")
            + context.git_branch.as_deref().unwrap_or("")
            + context.flow_context.as_deref().unwrap_or("");
        let skills = loader.matching(&context_text);
        if !skills.is_empty() {
            extras.push_str("\n\n## Active Skills");
            for skill in &skills {
                extras.push_str(&format!("\n\n### Skill: {}", skill.name));
                if !skill.description.is_empty() {
                    extras.push_str(&format!(" — {}", skill.description));
                }
                extras.push('\n');
                extras.push_str(&skill.content);
            }
        }
    }

    // 13.1: Inject matching rules from `.vibecli/rules/` directory
    if !context.workspace_root.as_os_str().is_empty() {
        let rules = crate::rules::RulesLoader::load_for_workspace(&context.workspace_root);
        let matching: Vec<_> = rules.iter()
            .filter(|r| r.matches_open_files(&context.open_files))
            .collect();
        if !matching.is_empty() {
            extras.push_str("\n\n## Active Rules");
            for rule in &matching {
                extras.push_str(&format!("\n\n### Rule: {}\n", rule.name));
                extras.push_str(&rule.content);
            }
        }
    }

    format!("{}{}", TOOL_SYSTEM_PROMPT, extras)
}
