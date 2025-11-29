pub mod app;
pub mod ui;
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
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::tui::app::{App, TuiMessage};
use vibe_ai::provider::{AIProvider as LLMProvider, Message, MessageRole, ProviderConfig};
use vibe_ai::providers::ollama::OllamaProvider;
use vibe_ai::providers::openai::OpenAIProvider;
use vibe_ai::providers::claude::ClaudeProvider;
use vibe_ai::providers::gemini::GeminiProvider;
use vibe_ai::providers::grok::GrokProvider;
use crate::config::Config;
use vibe_core::git;

use std::sync::Arc;

pub async fn run(provider_name: String, model: Option<String>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Create LLM provider
    let llm = create_provider(&provider_name, model)?;

    // Run app loop
    let res = run_app(&mut terminal, &mut app, llm).await;

    // Restore terminal
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
                
            let config = ProviderConfig {
                provider_type: "ollama".to_string(),
                api_url: Some(base_url),
                model: model,
                api_key: None,
                max_tokens: None,
                temperature: None,
            };
            Ok(Arc::new(OllamaProvider::new(config)))
        }
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
                
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "gpt-4-turbo".to_string());
                
            let config = ProviderConfig {
                provider_type: "openai".to_string(),
                api_url: None,
                model: model,
                api_key: Some(api_key),
                max_tokens: None,
                temperature: None,
            };
            Ok(Arc::new(OpenAIProvider::new(config)))
        }
        "anthropic" | "claude" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
                
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "claude-3-opus-20240229".to_string());
                
            let config = ProviderConfig {
                provider_type: "anthropic".to_string(),
                api_url: None,
                model: model,
                api_key: Some(api_key),
                max_tokens: None,
                temperature: None,
            };
            Ok(Arc::new(ClaudeProvider::new(config)))
        }
        "gemini" => {
            let api_key = std::env::var("GEMINI_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
                
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "gemini-pro".to_string());
                
            let config = ProviderConfig {
                provider_type: "gemini".to_string(),
                api_url: None,
                model: model,
                api_key: Some(api_key),
                max_tokens: None,
                temperature: None,
            };
            Ok(Arc::new(GeminiProvider::new(config)))
        }
        "grok" => {
            let api_key = std::env::var("GROK_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
                
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "grok-beta".to_string());
                
            let config = ProviderConfig {
                provider_type: "grok".to_string(),
                api_url: None,
                model: model,
                api_key: Some(api_key),
                max_tokens: None,
                temperature: None,
            };
            Ok(Arc::new(GrokProvider::new(config)))
        }
        _ => anyhow::bail!("Unknown provider: {}", provider_name),
    }
}

enum AppEvent {
    Input(Event),
    LlmResponse(String),
    Error(String),
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    llm: Arc<dyn LLMProvider>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    let mut event_stream = event::EventStream::new();

    // Inject Git Context
    let current_dir = std::env::current_dir()?;
    if git::is_git_repo(&current_dir) {
        let branch = git::get_current_branch(&current_dir).unwrap_or_default();
        let status = git::get_status(&current_dir).ok();
        let diff = git::get_repo_diff(&current_dir).unwrap_or_default();
        
        let mut context_msg = format!("Git Context (Branch: {}):\n", branch);
        
        if let Some(status) = status {
            if !status.file_statuses.is_empty() {
                context_msg.push_str("Changed files:\n");
                for (path, state) in status.file_statuses {
                    context_msg.push_str(&format!("- {} [{:?}]\n", path, state));
                }
            }
        }
        
        if !diff.is_empty() {
            // Truncate diff if too large to avoid context window issues
            let diff_len = diff.len();
            let max_len = 2000;
            if diff_len > max_len {
                context_msg.push_str(&format!("\nDiff (truncated {}/{} chars):\n{}\n...", max_len, diff_len, &diff[..max_len]));
            } else {
                context_msg.push_str(&format!("\nDiff:\n{}\n", diff));
            }
        }

        app.messages.push(TuiMessage::System(format!("Loaded git context for branch '{}'", branch)));
        // We add this as a hidden system message for the LLM, but for TUI we just showed a notification.
        // To ensure LLM sees it, we need to make sure it's included in the conversion later.
        // Actually, let's add it as a System message that IS visible for now, so user knows what AI sees.
        app.messages.push(TuiMessage::System(context_msg));
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
                AppEvent::Input(Event::Mouse(mouse_event)) => {
                    match mouse_event.kind {
                        event::MouseEventKind::ScrollUp => {
                            match app.current_screen {
                                crate::tui::app::CurrentScreen::Chat => {
                                    app.scroll_offset = app.scroll_offset.saturating_add(3);
                                }
                                crate::tui::app::CurrentScreen::DiffView => {
                                    app.diff_view.scroll_up();
                                }
                                crate::tui::app::CurrentScreen::FileTree => {
                                    app.file_tree.previous();
                                }
                            }
                        }
                        event::MouseEventKind::ScrollDown => {
                            match app.current_screen {
                                crate::tui::app::CurrentScreen::Chat => {
                                    app.scroll_offset = app.scroll_offset.saturating_sub(3);
                                }
                                crate::tui::app::CurrentScreen::DiffView => {
                                    app.diff_view.scroll_down();
                                }
                                crate::tui::app::CurrentScreen::FileTree => {
                                    app.file_tree.next();
                                }
                            }
                        }
                        _ => {}
                    }
                }
                AppEvent::Input(Event::Key(key)) => {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if app.exit_pending {
                                app.should_quit = true;
                            } else {
                                app.exit_pending = true;
                                app.messages.push(TuiMessage::System("⚠️  Press Ctrl+C again to quit".to_string()));
                            }
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                app.scroll_offset = app.scroll_offset.saturating_add(5);
                            }
                        }
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                app.scroll_offset = app.scroll_offset.saturating_sub(5);
                            }
                        }
                        KeyCode::Tab => {
                            app.current_screen = match app.current_screen {
                                crate::tui::app::CurrentScreen::Chat => crate::tui::app::CurrentScreen::FileTree,
                                crate::tui::app::CurrentScreen::FileTree => crate::tui::app::CurrentScreen::Chat,
                                _ => crate::tui::app::CurrentScreen::Chat,
                            };
                        }
                        KeyCode::Esc => {
                            app.current_screen = crate::tui::app::CurrentScreen::Chat;
                        }
                        KeyCode::Char('?') => {
                            let help_text = "📚 VibeCLI TUI Commands

  Navigation:
    /chat              Switch to chat view
    /files             Switch to file tree view
    /diff [file]       View diffs
    PageUp / Ctrl+u    Scroll up
    PageDown / Ctrl+d  Scroll down

  File Tree:
    Enter              Open file/dir
    Backspace          Go up directory
    Up/Down            Navigate

  Actions:
    /exec <cmd>        Execute shell command (via AI)
    ! <cmd>            Execute shell command directly
    /quit              Exit application

  Configuration:
    /init              Initialize default config
    /config            Show current config";

                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                if app.input.is_empty() {
                                    app.messages.push(TuiMessage::System(help_text.to_string()));
                                } else {
                                    app.on_key('?');
                                }
                            } else {
                                app.messages.push(TuiMessage::System(help_text.to_string()));
                                app.current_screen = crate::tui::app::CurrentScreen::Chat;
                            }
                        }
                        KeyCode::Char(c) => {
                            app.exit_pending = false; // Reset exit warning on other input
                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                app.on_key(c);
                            }
                        }
                        KeyCode::Backspace => {
                            match app.current_screen {
                                crate::tui::app::CurrentScreen::Chat => app.on_backspace(),
                                crate::tui::app::CurrentScreen::FileTree => app.file_tree.go_up(),
                                _ => {}
                            }
                        }
                        KeyCode::Up => {
                            match app.current_screen {
                                crate::tui::app::CurrentScreen::FileTree => app.file_tree.previous(),
                                crate::tui::app::CurrentScreen::DiffView => app.diff_view.scroll_up(),
                                _ => {}
                            }
                        }
                        KeyCode::Down => {
                            match app.current_screen {
                                crate::tui::app::CurrentScreen::FileTree => app.file_tree.next(),
                                crate::tui::app::CurrentScreen::DiffView => app.diff_view.scroll_down(),
                                _ => {}
                            }
                        }
                        KeyCode::PageUp => {
                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                app.scroll_offset = app.scroll_offset.saturating_add(5);
                            }
                        }
                        KeyCode::PageDown => {
                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                app.scroll_offset = app.scroll_offset.saturating_sub(5);
                            }
                        }
                        KeyCode::Enter => {
                            match app.current_screen {
                                crate::tui::app::CurrentScreen::FileTree => {
                                    if let Some(file_path) = app.file_tree.enter() {
                                        // Open file in DiffView for now as a simple viewer
                                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                                            app.diff_view.set_diff(&content, &content);
                                            app.current_screen = crate::tui::app::CurrentScreen::DiffView;
                                        }
                                    }
                                }
                                crate::tui::app::CurrentScreen::Chat => {
                                    if let Some(user_msg) = app.on_enter() {
                                    if user_msg.starts_with('/') || user_msg.starts_with('!') {
                                        // Handle commands
                                        let is_direct = user_msg.starts_with('!');
                                        // command_part logic removed as it was unused

                                        if is_direct {
                                            let cmd_to_run = user_msg[1..].trim();
                                            if !cmd_to_run.is_empty() {
                                                // Check safety config
                                                let config = Config::load().unwrap_or_default();
                                                if config.safety.require_approval_for_commands {
                                                    app.messages.push(TuiMessage::System("⚠️  Command execution requires approval. Please disable `require_approval_for_commands` in config to use this feature in TUI.".to_string()));
                                                } else {
                                                    app.messages.push(TuiMessage::System(format!("🚀 Executing: {}", cmd_to_run)));
                                                    
                                                    let output = if cfg!(target_os = "windows") {
                                                        std::process::Command::new("cmd").args(["/C", cmd_to_run]).output()
                                                    } else {
                                                        std::process::Command::new("sh").arg("-c").arg(cmd_to_run).output()
                                                    };

                                                    match output {
                                                        Ok(out) => {
                                                            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                                                            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                                                            let mut result = String::new();
                                                            if !stdout.is_empty() { result.push_str(&stdout); }
                                                            if !stderr.is_empty() { 
                                                                if !result.is_empty() { result.push_str("\n--- stderr ---\n"); }
                                                                result.push_str(&stderr); 
                                                            }
                                                            app.messages.push(TuiMessage::CommandOutput { command: cmd_to_run.to_string(), output: result });
                                                        }
                                                        Err(e) => {
                                                            app.messages.push(TuiMessage::Error(format!("Execution failed: {}", e)));
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            let parts: Vec<&str> = user_msg.splitn(2, ' ').collect();
                                            let command = parts[0];
                                            let args = if parts.len() > 1 { parts[1] } else { "" };

                                            match command {
                                                "/quit" | "/exit" => app.should_quit = true,
                                                "/chat" => app.current_screen = crate::tui::app::CurrentScreen::Chat,
                                                "/files" => app.current_screen = crate::tui::app::CurrentScreen::FileTree,
                                                "/help" => {
                                                    let help_text = "📚 VibeCLI TUI Commands

  Navigation:
    /chat              Switch to chat view
    /files             Switch to file tree view
    /diff [file]       View diffs
    PageUp / Ctrl+u    Scroll up
    PageDown / Ctrl+d  Scroll down

  File Tree:
    Enter              Open file/dir
    Backspace          Go up directory
    Up/Down            Navigate

  Actions:
    /exec <cmd>        Execute shell command (via AI)
    ! <cmd>            Execute shell command directly
    /quit              Exit application

  Configuration:
    /init              Initialize default config
    /config            Show current config";
                                                    app.messages.push(TuiMessage::System(help_text.to_string()));
                                                }
                                                "/init" => {
                                                    match Config::default().save() {
                                                        Ok(_) => app.messages.push(TuiMessage::System("✅ Configuration initialized at ~/.vibecli/config.toml".to_string())),
                                                        Err(e) => app.messages.push(TuiMessage::Error(format!("Failed to save config: {}", e))),
                                                    }
                                                }
                                                "/config" => {
                                                    let config = Config::load().unwrap_or_default();
                                                    let config_str = format!("⚙️  Configuration:\n  Providers:\n    Ollama: {:?}\n  UI: {:?}\n  Safety: {:?}", config.ollama, config.ui, config.safety);
                                                    app.messages.push(TuiMessage::System(config_str));
                                                }
                                                "/exec" => {
                                                    // For TUI, we just pass this to LLM as a request to generate a command
                                                    // The LLM will return a ```execute block, which we need to handle in the response parsing
                                                    // But for now, let's just send it as a user message
                                                    let llm = llm.clone();
                                                    let tx = tx.clone();
                                                    // msg_content removed as it was unused
                                                    
                                                    // ... (standard LLM send logic below)
                                                    // We need to duplicate the send logic here or fall through
                                                    // Let's fall through to the default handler which sends to LLM
                                                    // But we need to modify the content
                                                    app.messages.push(TuiMessage::User(user_msg.clone())); // Show original message
                                                    
                                                    let messages = vec![
                                                        Message { role: MessageRole::System, content: "You are a command generation assistant. Output only the command.".to_string() },
                                                        Message { role: MessageRole::User, content: args.to_string() }
                                                    ];
                                                    
                                                    tokio::spawn(async move {
                                                        match llm.chat(&messages, None).await {
                                                            Ok(response) => { let _ = tx.send(AppEvent::LlmResponse(response)).await; }
                                                            Err(e) => { let _ = tx.send(AppEvent::Error(e.to_string())).await; }
                                                        }
                                                    });
                                                    continue; // Skip the default send
                                                }
                                                "/diff" => {
                                                    app.current_screen = crate::tui::app::CurrentScreen::DiffView;
                                                    if !args.is_empty() {
                                                        if let Ok(content) = std::fs::read_to_string(args) {
                                                            app.diff_view.set_diff(&content, &content); 
                                                        }
                                                    } else {
                                                        let current_dir = std::env::current_dir().unwrap_or_default();
                                                        if let Ok(diff) = git::get_repo_diff(&current_dir) {
                                                            if diff.is_empty() {
                                                                app.diff_view.set_raw_diff("No changes detected.");
                                                            } else {
                                                                app.diff_view.set_raw_diff(&diff);
                                                            }
                                                        } else {
                                                            app.diff_view.set_raw_diff("Failed to load git diff or not a git repository.");
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    // If it starts with slash but not matched, it's unknown
                                                    app.messages.push(TuiMessage::System(format!("Unknown command: {}", command)));
                                                }
                                            }
                                        }
                                    } else {
                                        // Send to LLM
                                        let llm = llm.clone();
                                        let tx = tx.clone();
                                        
                                        // Convert TuiMessage to Message for LLM
                                        let messages: Vec<Message> = app.messages.iter().filter_map(|m| {
                                            match m {
                                                TuiMessage::User(content) => Some(Message { role: MessageRole::User, content: content.clone() }),
                                                TuiMessage::Assistant(content) => Some(Message { role: MessageRole::Assistant, content: content.clone() }),
                                                TuiMessage::System(content) => Some(Message { role: MessageRole::System, content: content.clone() }),
                                                TuiMessage::CommandOutput { command, output } => Some(Message { 
                                                    role: MessageRole::User, 
                                                    content: format!("Command executed: {}\nOutput:\n{}", command, output) 
                                                }),
                                                _ => None,
                                            }
                                        }).collect();
                                        
                                        tokio::spawn(async move {
                                            match llm.chat(&messages, None).await {
                                                Ok(response) => {
                                                    let _ = tx.send(AppEvent::LlmResponse(response)).await;
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(AppEvent::Error(e.to_string())).await;
                                                }
                                            }
                                        });
                                    }
                                }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                AppEvent::LlmResponse(response) => {
                    app.messages.push(TuiMessage::Assistant(response));
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
