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
use futures::{StreamExt, FutureExt};
use tokio::sync::mpsc;

use crate::tui::app::{App, TuiMessage};
use vibecli_core::llm::{LLMProvider, Message, MessageRole, OllamaProvider, OpenAIProvider, AnthropicProvider, GeminiProvider, GrokProvider};
use vibecli_core::Config;

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
                
            Ok(Arc::new(OllamaProvider::new(base_url, model)))
        }
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
                
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "gpt-4-turbo".to_string());
                
            Ok(Arc::new(OpenAIProvider::new(api_key, model)))
        }
        "anthropic" | "claude" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
                
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "claude-3-opus-20240229".to_string());
                
            Ok(Arc::new(AnthropicProvider::new(api_key, model)))
        }
        "gemini" => {
            let api_key = std::env::var("GEMINI_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
                
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "gemini-pro".to_string());
                
            Ok(Arc::new(GeminiProvider::new(api_key, model)))
        }
        "grok" => {
            let api_key = std::env::var("GROK_API_KEY")
                .ok()
                .or_else(|| provider_config.and_then(|c| c.api_key.clone()))
                .unwrap_or_default();
                
            let model = model
                .or_else(|| provider_config.and_then(|c| c.model.clone()))
                .unwrap_or_else(|| "grok-beta".to_string());
                
            Ok(Arc::new(GrokProvider::new(api_key, model)))
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
    if vibecli_core::git::GitOps::is_git_repo(&current_dir) {
        let branch = vibecli_core::git::GitOps::get_current_branch(&current_dir).unwrap_or_default();
        let status = vibecli_core::git::GitOps::get_status(&current_dir).unwrap_or_default();
        let diff = vibecli_core::git::GitOps::get_diff(&current_dir).unwrap_or_default();
        
        let mut context_msg = format!("Git Context (Branch: {}):\n", branch);
        if !status.is_empty() {
            context_msg.push_str("Changed files:\n");
            for (path, state) in status {
                context_msg.push_str(&format!("- {} [{}]\n", path, state));
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
                AppEvent::Input(Event::Key(key)) => {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                        }
                        KeyCode::Tab => {
                            app.current_screen = match app.current_screen {
                                crate::tui::app::CurrentScreen::Chat => crate::tui::app::CurrentScreen::FileTree,
                                crate::tui::app::CurrentScreen::FileTree => crate::tui::app::CurrentScreen::Chat,
                                _ => crate::tui::app::CurrentScreen::Chat,
                            };
                        }
                        KeyCode::Char(c) => {
                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                app.on_key(c);
                            }
                        }
                        KeyCode::Backspace => {
                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                app.on_backspace();
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
                        KeyCode::Enter => {
                            if let crate::tui::app::CurrentScreen::Chat = app.current_screen {
                                if let Some(user_msg) = app.on_enter() {
                                    if user_msg.starts_with('/') {
                                        // Handle slash commands
                                        let parts: Vec<&str> = user_msg.split_whitespace().collect();
                                        match parts[0] {
                                            "/quit" | "/exit" => app.should_quit = true,
                                            "/chat" => app.current_screen = crate::tui::app::CurrentScreen::Chat,
                                            "/files" => app.current_screen = crate::tui::app::CurrentScreen::FileTree,
                                            "/diff" => {
                                                app.current_screen = crate::tui::app::CurrentScreen::DiffView;
                                                
                                                if parts.len() > 1 {
                                                    // Diff specific file
                                                    let path = parts[1];
                                                    if let Ok(content) = std::fs::read_to_string(path) {
                                                        // For now, just show content as we don't have a "previous version" easily available here without git
                                                        // But if it's a git repo, we could try to get HEAD content.
                                                        // Let's just show raw content for now or try to get git diff for this path.
                                                        // Simplified: just show "Loading..." or similar if we can't get diff.
                                                        app.diff_view.set_diff(&content, &content); 
                                                    }
                                                } else {
                                                    // Show full git diff
                                                    let current_dir = std::env::current_dir().unwrap_or_default();
                                                    if let Ok(diff) = vibecli_core::git::GitOps::get_diff(&current_dir) {
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
                                                app.messages.push(TuiMessage::System(format!("Unknown command: {}", parts[0])));
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
                                            match llm.chat(&messages).await {
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
