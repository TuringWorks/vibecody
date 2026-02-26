pub mod app;
pub mod ui;
pub mod theme;
pub mod components;
mod tests;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::memory::ProjectMemory;
use crate::tool_executor::ToolExecutor;
use crate::tui::app::{App, CurrentScreen, PendingApproval, TuiMessage};
use crate::tui::components::agent_view::AgentStatus;
use vibe_ai::agent::{AgentContext, AgentEvent, AgentLoop, AgentStep, ApprovalPolicy, ToolExecutorTrait};
use vibe_ai::provider::{AIProvider as LLMProvider, Message, MessageRole, ProviderConfig};
use vibe_ai::providers::ollama::OllamaProvider;
use vibe_ai::providers::openai::OpenAIProvider;
use vibe_ai::providers::claude::ClaudeProvider;
use vibe_ai::providers::gemini::GeminiProvider;
use vibe_ai::providers::grok::GrokProvider;
use vibe_ai::tools::ToolResult;
use crate::config::Config;
use vibe_core::git;

pub async fn run(provider_name: String, model: Option<String>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let llm = create_provider(&provider_name, model)?;

    let res = run_app(&mut terminal, &mut app, llm).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn create_provider(provider_name: &str, model: Option<String>) -> Result<Arc<dyn LLMProvider>> {
    let config = Config::load().unwrap_or_default();
    let provider_config = config.get_provider_config(provider_name);

    match provider_name.to_lowercase().as_str() {
        "ollama" => {
            let base_url = provider_config
                .and_then(|c| c.api_url.clone())
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "qwen3-coder:480b-cloud".to_string());
            Ok(Arc::new(OllamaProvider::new(ProviderConfig {
                provider_type: "ollama".to_string(),
                api_url: Some(base_url),
                model,
                api_key: None,
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "gpt-4-turbo".to_string());
            Ok(Arc::new(OpenAIProvider::new(ProviderConfig {
                provider_type: "openai".to_string(),
                api_url: None,
                model,
                api_key: Some(api_key),
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }
        "anthropic" | "claude" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "claude-3-opus-20240229".to_string());
            Ok(Arc::new(ClaudeProvider::new(ProviderConfig {
                provider_type: "anthropic".to_string(),
                api_url: None,
                model,
                api_key: Some(api_key),
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }
        "gemini" => {
            let api_key = std::env::var("GEMINI_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "gemini-pro".to_string());
            Ok(Arc::new(GeminiProvider::new(ProviderConfig {
                provider_type: "gemini".to_string(),
                api_url: None,
                model,
                api_key: Some(api_key),
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }
        "grok" => {
            let api_key = std::env::var("GROK_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "grok-beta".to_string());
            Ok(Arc::new(GrokProvider::new(ProviderConfig {
                provider_type: "grok".to_string(),
                api_url: None,
                model,
                api_key: Some(api_key),
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }
        _ => anyhow::bail!("Unknown provider: {}", provider_name),
    }
}

enum AppEvent {
    Input(Event),
    /// Streaming chunk for regular chat.
    Chunk(String),
    /// Regular chat stream finished.
    StreamDone,
    /// Non-streaming response fallback.
    LlmResponse(String),
    /// Agent streaming chunk.
    AgentChunk(String),
    /// Agent tool call needs approval.
    AgentToolCallPending {
        call: vibe_ai::tools::ToolCall,
        result_tx: tokio::sync::oneshot::Sender<Option<ToolResult>>,
    },
    /// Agent auto-executed a tool call.
    AgentToolCallExecuted(AgentStep),
    /// Agent finished.
    AgentComplete(String),
    /// Agent error.
    AgentError(String),
    Error(String),
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    llm: Arc<dyn LLMProvider>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<AppEvent>(200);
    let mut event_stream = event::EventStream::new();

    let workspace = std::env::current_dir()?;
    let tool_executor = Arc::new(ToolExecutor::new(workspace.clone(), false));

    // Inject project memory (VIBECLI.md / AGENTS.md / CLAUDE.md)
    let memory = ProjectMemory::load(&workspace);
    if !memory.is_empty() {
        app.messages.push(TuiMessage::System(format!("📚 {}", memory.summary())));
        if let Some(mem_content) = memory.combined() {
            app.messages.push(TuiMessage::System(mem_content));
        }
    }

    // Inject smart context via ContextBuilder (git branch + diff, respects token budget)
    if git::is_git_repo(&workspace) {
        let branch = git::get_current_branch(&workspace).unwrap_or_default();
        let diff = git::get_repo_diff(&workspace).unwrap_or_default();
        let changed_files: Vec<String> = git::get_status(&workspace)
            .map(|s| s.file_statuses.into_keys().collect())
            .unwrap_or_default();

        let context = vibe_core::ContextBuilder::new()
            .with_git_branch(&branch)
            .with_git_diff(&diff)
            .with_git_changed_files(changed_files)
            .with_token_budget(4_000)
            .build_for_task("general coding assistance");

        if !context.is_empty() {
            app.messages.push(TuiMessage::System(format!(
                "Loaded git context for branch '{}'", branch
            )));
            app.messages.push(TuiMessage::System(context));
        }
    }

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let event = tokio::select! {
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(event)) => Some(AppEvent::Input(event)),
                    Some(Err(e)) => Some(AppEvent::Error(e.to_string())),
                    None => None,
                }
            },
            Some(app_event) = rx.recv() => Some(app_event),
        };

        if let Some(event) = event {
            match event {
                // ── Mouse Events ──────────────────────────────────────────────
                AppEvent::Input(Event::Mouse(mouse_event)) => {
                    match mouse_event.kind {
                        event::MouseEventKind::ScrollUp => match app.current_screen {
                            CurrentScreen::Chat => {
                                app.scroll_offset = app.scroll_offset.saturating_add(3);
                            }
                            CurrentScreen::DiffView => app.diff_view.scroll_up(),
                            CurrentScreen::FileTree => app.file_tree.previous(),
                            CurrentScreen::Agent => app.agent_view.scroll_up(),
                            CurrentScreen::VimEditor => {}
                        },
                        event::MouseEventKind::ScrollDown => match app.current_screen {
                            CurrentScreen::Chat => {
                                app.scroll_offset = app.scroll_offset.saturating_sub(3);
                            }
                            CurrentScreen::DiffView => app.diff_view.scroll_down(),
                            CurrentScreen::FileTree => app.file_tree.next(),
                            CurrentScreen::Agent => app.agent_view.scroll_down(),
                            CurrentScreen::VimEditor => {}
                        },
                        _ => {}
                    }
                }

                // ── Key Events ────────────────────────────────────────────────
                AppEvent::Input(Event::Key(key)) => {
                    // VimEditor gets all keys when active
                    if matches!(app.current_screen, CurrentScreen::VimEditor) {
                        // Get viewport height for scroll calculations (approximate)
                        let viewport_h = terminal.size().map(|r| r.height.saturating_sub(4)).unwrap_or(24);
                        let wants_close = app.vim_editor.handle_key(key, viewport_h);
                        if wants_close {
                            app.current_screen = CurrentScreen::Chat;
                            app.messages.push(TuiMessage::System("Editor closed.".to_string()));
                        }
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if app.exit_pending {
                                app.should_quit = true;
                            } else {
                                app.exit_pending = true;
                                app.messages.push(TuiMessage::System(
                                    "⚠️  Press Ctrl+C again to quit".to_string(),
                                ));
                            }
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let CurrentScreen::Chat = app.current_screen {
                                app.scroll_offset = app.scroll_offset.saturating_add(5);
                            }
                        }
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let CurrentScreen::Chat = app.current_screen {
                                app.scroll_offset = app.scroll_offset.saturating_sub(5);
                            }
                        }
                        KeyCode::Tab => {
                            app.current_screen = match app.current_screen {
                                CurrentScreen::Chat => CurrentScreen::FileTree,
                                CurrentScreen::FileTree => CurrentScreen::Chat,
                                CurrentScreen::Agent => CurrentScreen::Chat,
                                _ => CurrentScreen::Chat,
                            };
                        }
                        KeyCode::Esc => {
                            app.current_screen = CurrentScreen::Chat;
                        }

                        // ── Agent approval keys ───────────────────────────────
                        KeyCode::Char('y') if matches!(app.current_screen, CurrentScreen::Agent) => {
                            if let Some(PendingApproval { call, result_tx }) =
                                app.pending_approval.take()
                            {
                                app.agent_view.pending_call = None;
                                app.agent_view.status = AgentStatus::Running;
                                let executor = tool_executor.clone();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let result = executor.execute(&call).await;
                                    let step = AgentStep {
                                        step_num: 0,
                                        tool_call: call,
                                        tool_result: result.clone(),
                                        approved: true,
                                    };
                                    let _ = result_tx.send(Some(result));
                                    let _ = tx_clone
                                        .send(AppEvent::AgentToolCallExecuted(step))
                                        .await;
                                });
                            }
                        }
                        KeyCode::Char('n') if matches!(app.current_screen, CurrentScreen::Agent) => {
                            if let Some(PendingApproval { result_tx, .. }) =
                                app.pending_approval.take()
                            {
                                app.agent_view.pending_call = None;
                                app.agent_view.status = AgentStatus::Running;
                                let _ = result_tx.send(None);
                            }
                        }
                        KeyCode::Char('a') if matches!(app.current_screen, CurrentScreen::Agent) => {
                            // Approve-all: approve current and switch executor to FullAuto
                            if let Some(PendingApproval { call, result_tx }) =
                                app.pending_approval.take()
                            {
                                app.agent_view.pending_call = None;
                                app.agent_view.status = AgentStatus::Running;
                                let executor = tool_executor.clone();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let result = executor.execute(&call).await;
                                    let step = AgentStep {
                                        step_num: 0,
                                        tool_call: call,
                                        tool_result: result.clone(),
                                        approved: true,
                                    };
                                    let _ = result_tx.send(Some(result));
                                    let _ = tx_clone
                                        .send(AppEvent::AgentToolCallExecuted(step))
                                        .await;
                                });
                            }
                        }

                        KeyCode::Char('?') => {
                            let help_text = "📚 VibeCLI TUI Commands

  Navigation:
    /chat              Switch to chat view
    /files             Switch to file tree view
    /diff [file]       View diffs
    PageUp / Ctrl+u    Scroll up
    PageDown / Ctrl+d  Scroll down

  Agent:
    /agent <task>      Run autonomous coding agent
    y / n / a          Approve / reject / approve-all (in agent view)
    Tab                Return to chat from agent view

  File Tree:
    Enter              Open file/dir
    Backspace          Go up directory
    Up/Down            Navigate

  Actions:
    /exec <cmd>        Execute shell command (via AI)
    ! <cmd>            Execute shell command directly
    /edit <file>       Open file in vim-like modal editor
    /quit              Exit application

  Memory:
    /memory            Show loaded project memory (VIBECLI.md / AGENTS.md)
    /memory edit       Open project memory file for editing

  Configuration:
    /init              Initialize default config
    /config            Show current config";

                            if let CurrentScreen::Chat = app.current_screen {
                                if app.input.is_empty() {
                                    app.messages.push(TuiMessage::System(help_text.to_string()));
                                } else {
                                    app.on_key('?');
                                }
                            } else {
                                app.messages.push(TuiMessage::System(help_text.to_string()));
                                app.current_screen = CurrentScreen::Chat;
                            }
                        }
                        KeyCode::Char(c) => {
                            app.exit_pending = false;
                            if let CurrentScreen::Chat = app.current_screen {
                                app.on_key(c);
                            }
                        }
                        KeyCode::Backspace => match app.current_screen {
                            CurrentScreen::Chat => app.on_backspace(),
                            CurrentScreen::FileTree => app.file_tree.go_up(),
                            _ => {}
                        },
                        KeyCode::Up => match app.current_screen {
                            CurrentScreen::FileTree => app.file_tree.previous(),
                            CurrentScreen::DiffView => app.diff_view.scroll_up(),
                            CurrentScreen::Agent => app.agent_view.scroll_up(),
                            _ => {}
                        },
                        KeyCode::Down => match app.current_screen {
                            CurrentScreen::FileTree => app.file_tree.next(),
                            CurrentScreen::DiffView => app.diff_view.scroll_down(),
                            CurrentScreen::Agent => app.agent_view.scroll_down(),
                            _ => {}
                        },
                        KeyCode::PageUp => {
                            if let CurrentScreen::Chat = app.current_screen {
                                app.scroll_offset = app.scroll_offset.saturating_add(5);
                            }
                        }
                        KeyCode::PageDown => {
                            if let CurrentScreen::Chat = app.current_screen {
                                app.scroll_offset = app.scroll_offset.saturating_sub(5);
                            }
                        }
                        KeyCode::Enter => match app.current_screen {
                            CurrentScreen::FileTree => {
                                if let Some(file_path) = app.file_tree.enter() {
                                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                                        app.diff_view.set_diff(&content, &content);
                                        app.current_screen = CurrentScreen::DiffView;
                                    }
                                }
                            }
                            CurrentScreen::Chat => {
                                if let Some(user_msg) = app.on_enter() {
                                    handle_chat_input(
                                        user_msg,
                                        app,
                                        llm.clone(),
                                        tool_executor.clone(),
                                        tx.clone(),
                                        &workspace,
                                    )
                                    .await;
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }

                // ── LLM / Streaming events ────────────────────────────────────
                AppEvent::Chunk(text) => {
                    if let Some(TuiMessage::AssistantStreaming(ref mut s)) =
                        app.messages.last_mut()
                    {
                        s.push_str(&text);
                    } else {
                        app.messages.push(TuiMessage::AssistantStreaming(text));
                    }
                    app.scroll_offset = 0;
                }
                AppEvent::StreamDone => {
                    if let Some(last) = app.messages.last_mut() {
                        if let TuiMessage::AssistantStreaming(s) = last {
                            *last = TuiMessage::Assistant(s.clone());
                        }
                    }
                }
                AppEvent::LlmResponse(response) => {
                    app.messages.push(TuiMessage::Assistant(response));
                }

                // ── Agent events ──────────────────────────────────────────────
                AppEvent::AgentChunk(text) => {
                    app.agent_view.append_stream(&text);
                }
                AppEvent::AgentToolCallPending { call, result_tx } => {
                    app.agent_view.streaming_text.clear();
                    app.agent_view.pending_call = Some(call.clone());
                    app.agent_view.status = AgentStatus::WaitingApproval;
                    app.pending_approval = Some(PendingApproval { call, result_tx });
                    app.current_screen = CurrentScreen::Agent;
                }
                AppEvent::AgentToolCallExecuted(step) => {
                    app.agent_view.add_step(step);
                    app.agent_view.status = AgentStatus::Running;
                }
                AppEvent::AgentComplete(summary) => {
                    app.agent_view.streaming_text.clear();
                    app.agent_view.status = AgentStatus::Complete(summary.clone());
                    app.messages.push(TuiMessage::System(format!(
                        "✅ Agent complete: {}",
                        if summary.len() > 80 { &summary[..80] } else { &summary }
                    )));
                }
                AppEvent::AgentError(e) => {
                    app.agent_view.status = AgentStatus::Error(e.clone());
                    app.messages.push(TuiMessage::Error(format!("Agent: {}", e)));
                }

                AppEvent::Error(e) => {
                    app.messages.push(TuiMessage::Error(e));
                }
                _ => {}
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Handle user input in the Chat screen.
async fn handle_chat_input(
    user_msg: String,
    app: &mut App,
    llm: Arc<dyn LLMProvider>,
    tool_executor: Arc<ToolExecutor>,
    tx: mpsc::Sender<AppEvent>,
    workspace: &std::path::Path,
) {
    if user_msg.starts_with('!') {
        // Direct shell command
        let cmd = user_msg[1..].trim();
        if !cmd.is_empty() {
            let config = Config::load().unwrap_or_default();
            if config.safety.require_approval_for_commands {
                app.messages.push(TuiMessage::System(
                    "⚠️  Command execution requires approval. Disable `require_approval_for_commands` in config."
                        .to_string(),
                ));
            } else {
                app.messages.push(TuiMessage::System(format!("🚀 Executing: {}", cmd)));
                let output = if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd").args(["/C", cmd]).output()
                } else {
                    std::process::Command::new("sh").arg("-c").arg(cmd).output()
                };
                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        let mut result = stdout;
                        if !stderr.is_empty() {
                            if !result.is_empty() {
                                result.push_str("\n--- stderr ---\n");
                            }
                            result.push_str(&stderr);
                        }
                        app.messages.push(TuiMessage::CommandOutput {
                            command: cmd.to_string(),
                            output: result,
                        });
                    }
                    Err(e) => app.messages.push(TuiMessage::Error(format!("Execution failed: {}", e))),
                }
            }
        }
        return;
    }

    if user_msg.starts_with('/') {
        let parts: Vec<&str> = user_msg.splitn(2, ' ').collect();
        let command = parts[0];
        let args = if parts.len() > 1 { parts[1] } else { "" };

        match command {
            "/quit" | "/exit" => app.should_quit = true,
            "/chat" => app.current_screen = CurrentScreen::Chat,
            "/files" => app.current_screen = CurrentScreen::FileTree,
            "/help" => {
                app.messages.push(TuiMessage::System("Use ? for shortcuts".to_string()));
            }
            "/init" => match Config::default().save() {
                Ok(_) => app.messages.push(TuiMessage::System(
                    "✅ Configuration initialized at ~/.vibecli/config.toml".to_string(),
                )),
                Err(e) => app.messages
                    .push(TuiMessage::Error(format!("Failed to save config: {}", e))),
            },
            "/config" => {
                let config = Config::load().unwrap_or_default();
                app.messages.push(TuiMessage::System(format!(
                    "⚙️  Configuration:\n  Providers:\n    Ollama: {:?}\n  UI: {:?}\n  Safety: {:?}",
                    config.ollama, config.ui, config.safety
                )));
            }
            "/agent" => {
                if args.is_empty() {
                    app.messages.push(TuiMessage::System(
                        "Usage: /agent <task description>".to_string(),
                    ));
                    return;
                }
                let task = args.to_string();
                app.agent_view.reset();
                app.current_screen = CurrentScreen::Agent;
                app.messages.push(TuiMessage::System(format!("🤖 Starting agent: {}", task)));

                let config = Config::load().unwrap_or_default();
                let approval = ApprovalPolicy::from_str(&config.safety.approval_policy);
                let sandbox = config.safety.sandbox;
                let executor_for_agent: Arc<dyn vibe_ai::agent::ToolExecutorTrait> =
                    Arc::new(ToolExecutor::new(workspace.to_path_buf(), sandbox));
                let agent = AgentLoop::new(llm, approval, executor_for_agent);
                let ws = workspace.to_path_buf();
                let tx_clone = tx.clone();

                tokio::spawn(async move {
                    let (agent_tx, mut agent_rx) = mpsc::channel::<AgentEvent>(50);
                    let agent_tx_for_loop = agent_tx.clone();
                    let context = AgentContext {
                        workspace_root: ws,
                        ..Default::default()
                    };
                    tokio::spawn(async move {
                        let _ = agent.run(&task, context, agent_tx_for_loop).await;
                    });
                    drop(agent_tx); // close extra sender so rx drains when agent done

                    while let Some(ev) = agent_rx.recv().await {
                        let app_ev = match ev {
                            AgentEvent::StreamChunk(t) => AppEvent::AgentChunk(t),
                            AgentEvent::ToolCallPending { call, result_tx } => {
                                AppEvent::AgentToolCallPending { call, result_tx }
                            }
                            AgentEvent::ToolCallExecuted(step) => {
                                AppEvent::AgentToolCallExecuted(step)
                            }
                            AgentEvent::Complete(s) => AppEvent::AgentComplete(s),
                            AgentEvent::Error(e) => AppEvent::AgentError(e),
                        };
                        if tx_clone.send(app_ev).await.is_err() {
                            break;
                        }
                    }
                });
            }
            "/exec" => {
                if args.is_empty() {
                    app.messages.push(TuiMessage::System(
                        "Usage: /exec <description of what to do>".to_string(),
                    ));
                    return;
                }
                let messages = vec![
                    Message {
                        role: MessageRole::System,
                        content: "You are a command generation assistant. Output only the command."
                            .to_string(),
                    },
                    Message { role: MessageRole::User, content: args.to_string() },
                ];
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    match llm.chat(&messages, None).await {
                        Ok(response) => {
                            let _ = tx_clone.send(AppEvent::LlmResponse(response)).await;
                        }
                        Err(e) => {
                            let _ = tx_clone.send(AppEvent::Error(e.to_string())).await;
                        }
                    }
                });
            }
            "/memory" => {
                let cwd = std::env::current_dir().unwrap_or_default();
                match args {
                    "show" | "" => {
                        let mem = ProjectMemory::load(&cwd);
                        app.messages.push(TuiMessage::System(mem.summary()));
                        if let Some(content) = mem.combined() {
                            app.messages.push(TuiMessage::System(content));
                        }
                    }
                    "edit" => {
                        let path = ProjectMemory::default_repo_path(&cwd);
                        app.messages.push(TuiMessage::System(format!(
                            "Edit project memory at: {}",
                            path.display()
                        )));
                    }
                    _ => {
                        app.messages.push(TuiMessage::System(
                            "Usage: /memory [show|edit]".to_string(),
                        ));
                    }
                }
            }
            "/edit" => {
                if args.is_empty() {
                    app.messages.push(TuiMessage::System(
                        "Usage: /edit <file>  — open file in vim-like editor".to_string(),
                    ));
                } else {
                    let path = std::path::PathBuf::from(args);
                    app.vim_editor.open(path);
                    app.current_screen = CurrentScreen::VimEditor;
                }
            }
            "/diff" => {
                app.current_screen = CurrentScreen::DiffView;
                if !args.is_empty() {
                    if let Ok(content) = std::fs::read_to_string(args) {
                        app.diff_view.set_diff(&content, &content);
                    }
                } else {
                    let current_dir = std::env::current_dir().unwrap_or_default();
                    match vibe_core::git::get_repo_diff(&current_dir) {
                        Ok(diff) if !diff.is_empty() => app.diff_view.set_raw_diff(&diff),
                        _ => app.diff_view.set_raw_diff("No changes detected."),
                    }
                }
            }
            "/theme" => {
                use crate::tui::theme::{available_themes, get_theme};
                if args.is_empty() {
                    let list = available_themes().join(", ");
                    app.messages.push(TuiMessage::System(format!(
                        "Available themes: {}\nCurrent: {}\nUsage: /theme <name>",
                        list, app.theme.name
                    )));
                } else {
                    let new_theme = get_theme(args);
                    let name = new_theme.name;
                    app.theme = new_theme;
                    app.messages.push(TuiMessage::System(format!("✅ Theme switched to '{}'", name)));
                }
            }
            "/check" => {
                // Run cargo check (or eslint if package.json present) and populate
                // the diagnostics panel.
                let cwd = std::env::current_dir().unwrap_or_default();
                let use_npm = cwd.join("package.json").exists() && !cwd.join("Cargo.toml").exists();
                let (prog, prog_args): (&str, &[&str]) = if use_npm {
                    ("npx", &["eslint", "--format", "json", "."])
                } else {
                    ("cargo", &["check", "--message-format=json", "--quiet"])
                };
                app.messages.push(TuiMessage::System(format!(
                    "Running {} {}…", prog, prog_args.join(" ")
                )));
                match std::process::Command::new(prog)
                    .args(prog_args)
                    .current_dir(&cwd)
                    .output()
                {
                    Err(e) => {
                        app.messages.push(TuiMessage::Error(format!(
                            "Failed to run {}: {}", prog, e
                        )));
                    }
                    Ok(out) => {
                        use crate::tui::components::diagnostics::parse_cargo_check;
                        let combined = format!(
                            "{}\n{}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr),
                        );
                        let diags = parse_cargo_check(&combined);
                        let summary = app.diagnostics_panel.status.clone();
                        app.diagnostics_panel.set(diags);
                        let _ = summary; // used inside set()
                        app.messages.push(TuiMessage::System(format!(
                            "Diagnostics: {}", app.diagnostics_panel.status
                        )));
                    }
                }
            }
            "/share" => {
                if args.is_empty() {
                    app.messages.push(TuiMessage::System(
                        "Usage: /share <session_id>  — print shareable URL (requires vibecli serve)".to_string()
                    ));
                } else {
                    let url = format!("http://localhost:7878/share/{}", args.trim());
                    app.messages.push(TuiMessage::System(format!(
                        "📤 Share URL: {}", url
                    )));
                }
            }
            _ => {
                app.messages.push(TuiMessage::System(format!(
                    "Unknown command: {}",
                    command
                )));
            }
        }
        return;
    }

    // Regular chat — stream response
    let messages: Vec<Message> = app.messages.iter().filter_map(|m| match m {
        TuiMessage::User(c) => Some(Message { role: MessageRole::User, content: c.clone() }),
        TuiMessage::Assistant(c) | TuiMessage::AssistantStreaming(c) => {
            Some(Message { role: MessageRole::Assistant, content: c.clone() })
        }
        TuiMessage::System(c) => Some(Message { role: MessageRole::System, content: c.clone() }),
        TuiMessage::CommandOutput { command, output } => Some(Message {
            role: MessageRole::User,
            content: format!("Command executed: {}\nOutput:\n{}", command, output),
        }),
        _ => None,
    }).collect();

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        match llm.stream_chat(&messages).await {
            Ok(mut stream) => {
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(text) => {
                            if tx_clone.send(AppEvent::Chunk(text)).await.is_err() {
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = tx_clone.send(AppEvent::Error(e.to_string())).await;
                            return;
                        }
                    }
                }
                let _ = tx_clone.send(AppEvent::StreamDone).await;
            }
            Err(e) => {
                let _ = tx_clone.send(AppEvent::Error(e.to_string())).await;
            }
        }
    });

    // Suppress unused warning
    let _ = tool_executor;
}
