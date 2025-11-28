use clap::Parser;
use anyhow::Result;
use crate::config::Config;
use crate::syntax::highlight_code_blocks;
use vibe_ai::provider::{AIProvider as LLMProvider, Message, MessageRole, ProviderConfig};
use vibe_ai::providers::ollama::OllamaProvider;
use std::io::{self, Write};

mod config;
mod syntax;
mod diff_viewer;
use diff_viewer::DiffViewer;

mod repl;
use rustyline::error::ReadlineError;

mod tui;

#[derive(Parser)]
#[command(name = "vibecli")]
#[command(about = "AI-powered coding assistant for the terminal", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "ollama")]
    provider: String,
    
    #[arg(short, long)]
    model: Option<String>,

    #[arg(long)]
    tui: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    if cli.tui {
        return tui::run(cli.provider, cli.model).await;
    }
    
    println!("🤖 VibeCLI - AI-Powered Coding Assistant");
    println!("Provider: {}", cli.provider);
    println!("\nAvailable commands:");
    println!("  /chat <message>    - Chat with AI");
    println!("  /generate <prompt> - Generate code");
    println!("  /diff <file>       - Show diff for file");
    println!("  /apply <file>      - Apply changes to file");
    println!("  /exec <command>    - Execute command with AI");
    println!("  /config            - Show configuration");
    println!("  /help              - Show this help");
    println!("  /exit              - Exit VibeCLI");
    println!("\nType a message to chat, or use a command.\n");
    
    let llm = create_provider(&cli.provider, cli.model)?;
    let mut messages: Vec<Message> = Vec::new();
    let mut conversation_active = false;
    
    let config = rustyline::Config::builder()
        .auto_add_history(true)
        .build();

    let mut rl = rustyline::Editor::with_config(config)?;
    rl.set_helper(Some(repl::VibeHelper::new()));
    
    // Load history
    let history_path = dirs::home_dir()
        .map(|h| h.join(".vibecli").join("history.txt"));
        
    if let Some(ref path) = history_path {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = rl.load_history(path);
    }

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                
                rl.add_history_entry(line.as_str())?;
                
                // Handle direct commands
                if input.starts_with('!') {
                    let command = input[1..].trim();
                    if !command.is_empty() {
                        let require_approval = Config::load()
                            .map(|c| c.safety.require_approval_for_commands)
                            .unwrap_or(true);

                        let should_run = if require_approval {
                            print!("⚡ Execute command: {}? (y/N): ", command);
                            io::stdout().flush()?;
                            
                            let mut confirm = String::new();
                            io::stdin().read_line(&mut confirm)?;
                            confirm.trim().to_lowercase() == "y"
                        } else {
                            true
                        };

                        if should_run {
                            println!("🚀 Executing...");
                            use std::process::Command;
                            let output = if cfg!(target_os = "windows") {
                                Command::new("cmd").args(["/C", command]).output()
                            } else {
                                Command::new("sh").arg("-c").arg(command).output()
                            };
                            
                            match output {
                                Ok(output) => {
                                    println!("{}", String::from_utf8_lossy(&output.stdout));
                                    if !output.stderr.is_empty() {
                                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                                    }
                                }
                                Err(e) => eprintln!("❌ Execution failed: {}", e),
                            }
                            println!();
                        } else {
                            println!("❌ Command execution cancelled\n");
                        }
                        continue;
                    }
                }

                // Handle commands
                if input.starts_with('/') {
                    let parts: Vec<&str> = input.splitn(2, ' ').collect();
                    let command = parts[0];
                    let args = if parts.len() > 1 { parts[1] } else { "" };
                    
                    match command {
                        "/exit" | "/quit" => {
                            println!("👋 Goodbye!");
                            break;
                        }
                        
                        "/help" => {
                            show_help();
                        }
                        
                        "/config" => {
                            show_config().await?;
                        }
                        
                        "/chat" => {
                            if args.is_empty() {
                                println!("Usage: /chat <message>");
                                continue;
                            }
                            
                            conversation_active = true;
                            messages.push(Message {
                                role: MessageRole::User,
                                content: args.to_string(),
                            });
                            
                            print!("🤖 ");
                            io::stdout().flush()?;
                            
                            match llm.chat(&messages, None).await {
                                Ok(response) => {
                                    let highlighted = highlight_code_blocks(&response);
                                    println!("{}\n", highlighted);
                                    messages.push(Message {
                                        role: MessageRole::Assistant,
                                        content: response,
                                    });
                                }
                                Err(e) => {
                                    eprintln!("❌ Error: {}\n", e);
                                }
                            }
                        }
                        
                        "/generate" => {
                            if args.is_empty() {
                                println!("Usage: /generate <prompt> [--output <file>]");
                                continue;
                            }
                            
                            println!("🔨 Generating code...");
                            
                            let gen_messages = vec![
                                Message {
                                    role: MessageRole::System,
                                    content: "You are a code generation assistant. Generate clean, well-documented code based on the user's request. Only output the code, no explanations unless asked.".to_string(),
                                },
                                Message {
                                    role: MessageRole::User,
                                    content: args.to_string(),
                                },
                            ];
                            
                            match llm.chat(&gen_messages, None).await {
                                Ok(response) => {
                                    let highlighted = highlight_code_blocks(&response);
                                    println!("\n{}\n", highlighted);
                                    
                                    // Ask if user wants to save
                                    // Use rustyline for this input too if possible, but simple stdin is fine for y/n
                                    print!("💾 Save to file? (y/N or filename): ");
                                    io::stdout().flush()?;
                                    
                                    let mut save_input = String::new();
                                    io::stdin().read_line(&mut save_input)?;
                                    let save_input = save_input.trim();
                                    
                                    if !save_input.is_empty() && save_input.to_lowercase() != "n" {
                                        let filename = if save_input.to_lowercase() == "y" {
                                            "generated_code.txt"
                                        } else {
                                            save_input
                                        };
                                        
                                        // Strip markdown code blocks if present for saving
                                        let clean_code = response.lines()
                                            .filter(|l| !l.starts_with("```"))
                                            .collect::<Vec<_>>()
                                            .join("\n");
                                            
                                        std::fs::write(filename, &clean_code)?;
                                        println!("✅ Saved to: {}\n", filename);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Error: {}\n", e);
                                }
                            }
                        }
                        
                        "/diff" => {
                            if args.is_empty() {
                                println!("Usage: /diff <file>");
                                continue;
                            }
                            
                            match DiffViewer::show_file_diff(args) {
                                Ok(_) => {},
                                Err(e) => {
                                    eprintln!("❌ Error showing diff: {}\n", e);
                                }
                            }
                        }
                        
                        "/apply" => {
                            if args.is_empty() {
                                println!("Usage: /apply <file> <description of changes>");
                                println!("Example: /apply src/main.rs add error handling");
                                continue;
                            }
                            
                            let parts: Vec<&str> = args.splitn(2, ' ').collect();
                            if parts.len() < 2 {
                                println!("Usage: /apply <file> <description of changes>");
                                continue;
                            }
                            
                            let file_path = parts[0];
                            let change_description = parts[1];
                            
                            println!("🔨 Generating changes for: {}", file_path);
                            
                            // Read current file content
                            match std::fs::read_to_string(file_path) {
                                Ok(current_content) => {
                                    let apply_messages = vec![
                                        Message {
                                            role: MessageRole::System,
                                            content: "You are a code modification assistant. Given the current file content and a description of changes, output ONLY the modified file content. Do not include explanations or markdown formatting.".to_string(),
                                        },
                                        Message {
                                            role: MessageRole::User,
                                            content: format!("Current file content:\n```\n{}\n```\n\nChanges to make: {}\n\nOutput the complete modified file:", current_content, change_description),
                                        },
                                    ];
                                    
                                    match llm.chat(&apply_messages, None).await {
                                        Ok(modified_content) => {
                                            // Clean potential markdown blocks from response
                                            let clean_modified = modified_content.lines()
                                                .filter(|l| !l.starts_with("```"))
                                                .collect::<Vec<_>>()
                                                .join("\n");

                                            // Show diff
                                            println!("\n📊 Proposed changes:\n");
                                            if let Err(e) = DiffViewer::show_diff(file_path, &current_content, &clean_modified) {
                                                eprintln!("Warning: Could not show diff: {}", e);
                                            }
                                            
                                            // Ask for confirmation
                                            print!("✅ Apply these changes? (y/N): ");
                                            io::stdout().flush()?;
                                            
                                            let mut confirm = String::new();
                                            io::stdin().read_line(&mut confirm)?;
                                            
                                            if confirm.trim().to_lowercase() == "y" {
                                                match std::fs::write(file_path, &clean_modified) {
                                                    Ok(_) => println!("✅ Changes applied to: {}\n", file_path),
                                                    Err(e) => eprintln!("❌ Failed to write file: {}\n", e),
                                                }
                                            } else {
                                                println!("❌ Changes cancelled\n");
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("❌ Error generating changes: {}\n", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to read file: {}\n", e);
                                }
                            }
                        }
                        
                        "/exec" => {
                            if args.is_empty() {
                                println!("Usage: /exec <description of what to do>");
                                continue;
                            }
                            
                            println!("⚡ Generating command for: {}", args);
                            
                            let exec_messages = vec![
                                Message {
                                    role: MessageRole::System,
                                    content: "You are a command-line assistant. Generate a single shell command to accomplish the user's request. Only output the command, nothing else.".to_string(),
                                },
                                Message {
                                    role: MessageRole::User,
                                    content: args.to_string(),
                                },
                            ];
                            
                            match llm.chat(&exec_messages, None).await {
                                Ok(command) => {
                                    let command = command.trim();
                                    println!("📝 Suggested command: {}", command);
                                    
                                    print!("⚠️  Execute this command? (y/N): ");
                                    io::stdout().flush()?;
                                    
                                    let mut confirm = String::new();
                                    io::stdin().read_line(&mut confirm)?;
                                    
                                    if confirm.trim().to_lowercase() == "y" {
                                        println!("🚀 Executing...");
                                        
                                        use std::process::Command;
                                        let output = if cfg!(target_os = "windows") {
                                            Command::new("cmd").args(["/C", command]).output()?
                                        } else {
                                            Command::new("sh").arg("-c").arg(command).output()?
                                        };
                                        
                                        println!("{}", String::from_utf8_lossy(&output.stdout));
                                        if !output.stderr.is_empty() {
                                            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                                        }
                                        println!();
                                    } else {
                                        println!("❌ Command execution cancelled\n");
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Error: {}\n", e);
                                }
                            }
                        }
                        
                        _ => {
                            println!("❌ Unknown command: {}", command);
                            println!("Type /help for available commands\n");
                        }
                    }
                } else {
                    // Regular chat message (no command prefix)
                    if !conversation_active {
                        messages.clear(); // Start fresh conversation
                        conversation_active = true;
                        
                        // Add system prompt for command execution
                        messages.push(Message {
                            role: MessageRole::System,
                            content: "You are a helpful coding assistant. If the user asks you to run a command or execute a file, output the command in a markdown code block with the language 'execute'. For example:\n```execute\npython script.py\n```\nOnly use this for commands you want the user to run immediately.".to_string(),
                        });
                    }
                    
                    messages.push(Message {
                        role: MessageRole::User,
                        content: input.to_string(),
                    });
                    
                    print!("🤖 ");
                    io::stdout().flush()?;
                    
                    match llm.chat(&messages, None).await {
                        Ok(response) => {
                            let highlighted = highlight_code_blocks(&response);
                            println!("{}\n", highlighted);
                            messages.push(Message {
                                role: MessageRole::Assistant,
                                content: response.clone(),
                            });

                            // Check for execute blocks
                            if response.contains("```execute") {
                                let lines: Vec<&str> = response.lines().collect();
                                let mut in_exec_block = false;
                                let mut command_to_run = String::new();

                                for line in lines {
                                    if line.trim().starts_with("```execute") {
                                        in_exec_block = true;
                                        continue;
                                    }
                                    if in_exec_block {
                                        if line.trim().starts_with("```") {
                                            break;
                                        }
                                        command_to_run.push_str(line);
                                        command_to_run.push('\n');
                                    }
                                }

                                let command_to_run = command_to_run.trim();
                                if !command_to_run.is_empty() {
                                    println!("⚡ Suggested command: {}", command_to_run);
                                    print!("⚠️  Execute this command? (y/N): ");
                                    io::stdout().flush()?;
                                    
                                    let mut confirm = String::new();
                                    io::stdin().read_line(&mut confirm)?;
                                    
                                    if confirm.trim().to_lowercase() == "y" {
                                        println!("🚀 Executing...");
                                        
                                        use std::process::Command;
                                        let output = if cfg!(target_os = "windows") {
                                            Command::new("cmd").args(["/C", command_to_run]).output()
                                        } else {
                                            Command::new("sh").arg("-c").arg(command_to_run).output()
                                        };
                                        
                                        match output {
                                            Ok(output) => {
                                                println!("{}", String::from_utf8_lossy(&output.stdout));
                                                if !output.stderr.is_empty() {
                                                    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                                                }
                                            }
                                            Err(e) => eprintln!("❌ Execution failed: {}", e),
                                        }
                                        println!();
                                    } else {
                                        println!("❌ Command execution cancelled\n");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error: {}\n", e);
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    
    // Save history
    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }
    
    Ok(())
}

fn create_provider(provider_name: &str, model: Option<String>) -> Result<Box<dyn LLMProvider>> {
    match provider_name.to_lowercase().as_str() {
        "ollama" => {
            let model = model.unwrap_or_else(|| "qwen3-coder:480b-cloud".to_string());
            let config = ProviderConfig {
                provider_type: "ollama".to_string(),
                api_url: Some("http://localhost:11434".to_string()),
                model: model,
                api_key: None,
                max_tokens: None,
                temperature: None,
            };
            Ok(Box::new(OllamaProvider::new(config)))
        }
        _ => anyhow::bail!("Unknown provider: {}", provider_name),
    }
}

fn show_help() {
    println!("\n📚 VibeCLI Commands:");
    println!("  /chat <message>    - Start or continue a conversation with AI");
    println!("  /generate <prompt> - Generate code from a description");
    println!("  /diff <file>       - Show diff for a file");
    println!("  /apply <file>      - Apply AI-suggested changes to a file");
    println!("  /exec <task>       - Generate and execute a shell command");
    println!("  /config            - Show current configuration");
    println!("  /help              - Show this help message");
    println!("  /exit              - Exit VibeCLI");
    println!("  ! <command>        - Execute shell command directly (e.g. !ls)");
    println!("\n💡 Tip: You can also just type a message to chat without using /chat\n");
}

async fn show_config() -> Result<()> {
    match Config::load() {
        Ok(config) => {
            println!("\n⚙️  Configuration:");
            println!("  Location: ~/.vibecli/config.toml");
            
            println!("Current Configuration:");
            println!("  Providers:");
            
            if let Some(ollama) = &config.ollama {
                println!("    Ollama: Enabled={}, Model={}", ollama.enabled, ollama.model.as_deref().unwrap_or("default"));
            }
            
            if let Some(openai) = &config.openai {
                println!("    OpenAI: Enabled={}, Model={}", openai.enabled, openai.model.as_deref().unwrap_or("default"));
            }
            
            if let Some(claude) = &config.claude {
                println!("    Claude: Enabled={}, Model={}", claude.enabled, claude.model.as_deref().unwrap_or("default"));
            }

            println!("  UI:");
            println!("    Theme: {}", config.ui.theme.as_deref().unwrap_or("default"));
            
            println!("  Safety:");
            println!("    Require approval for commands: {}", config.safety.require_approval_for_commands);
            println!("    Require approval for file changes: {}", config.safety.require_approval_for_file_changes);
            println!();
        }
        Err(e) => {
            println!("❌ Failed to load config: {}", e);
            println!("💡 Run: echo 'vibecli init' to create default config\n");
        }
    }
    
    Ok(())
}
