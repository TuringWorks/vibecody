use clap::Parser;
use anyhow::Result;
use crate::config::Config;
use crate::syntax::highlight_code_blocks;
use vibe_ai::provider::{AIProvider as LLMProvider, ImageAttachment, Message, MessageRole, ProviderConfig};
use vibe_ai::providers::ollama::OllamaProvider;
use vibe_ai::agent::{AgentContext, AgentEvent, AgentLoop, ApprovalPolicy};
use vibe_ai::{MultiAgentOrchestrator, OrchestratorEvent, ExecutorFactory};
use vibe_ai::hooks::HookRunner;
use vibe_ai::planner::PlannerAgent;
use vibe_ai::trace::{list_traces, load_session, load_trace, TraceWriter};
use regex::Regex;
use std::io::{self, Write};
use std::sync::Arc;

mod config;
mod syntax;
mod diff_viewer;
mod tool_executor;
mod memory;
mod ci;
use tool_executor::{ToolExecutor, VibeCoreWorktreeManager};
use diff_viewer::DiffViewer;
use memory::ProjectMemory;

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

    /// Run the agent autonomously on a task (non-TUI mode).
    #[arg(long, value_name = "TASK")]
    agent: Option<String>,

    /// Non-interactive CI/exec mode: run agent, write report to stdout/file, exit with code.
    #[arg(long, value_name = "TASK")]
    exec: Option<String>,

    /// Output format for --exec: json (default), markdown, verbose.
    #[arg(long, default_value = "json")]
    output_format: String,

    /// Write --exec report to FILE instead of stdout.
    #[arg(long, value_name = "FILE")]
    output: Option<String>,

    /// Approval policy: prompt before every tool call (default).
    #[arg(long)]
    suggest: bool,

    /// Approval policy: auto-apply file edits, prompt for bash.
    #[arg(long)]
    auto_edit: bool,

    /// Approval policy: execute all tool calls without prompting.
    #[arg(long)]
    full_auto: bool,

    /// Resume a previous agent session by session ID (or ID prefix).
    #[arg(long, value_name = "SESSION_ID")]
    resume: Option<String>,

    /// Enable Plan Mode: generate and show execution plan before running agent.
    #[arg(long)]
    plan: bool,

    /// Run N parallel agents on the same task (requires --agent).
    #[arg(long, value_name = "N")]
    parallel: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Determine approval policy from flags
    let approval_policy = Config::load()
        .map(|c| {
            let from_config = c.safety.approval_policy.clone();
            let from_flags = Config::approval_from_flags(cli.suggest, cli.auto_edit, cli.full_auto);
            // CLI flags override config
            if cli.suggest || cli.auto_edit || cli.full_auto {
                from_flags
            } else {
                from_config
            }
        })
        .unwrap_or_else(|_| {
            Config::approval_from_flags(cli.suggest, cli.auto_edit, cli.full_auto)
        });

    if cli.tui {
        return tui::run(cli.provider, cli.model).await;
    }

    // Non-interactive CI/exec mode: --exec "task"
    if let Some(task) = cli.exec {
        let llm = create_provider(&cli.provider, cli.model.clone())?;
        let cwd = std::env::current_dir()?;
        let config = Config::load().unwrap_or_default();
        let sandbox = config.safety.sandbox;
        let executor: Arc<dyn vibe_ai::agent::ToolExecutorTrait> =
            Arc::new(ToolExecutor::new(cwd.clone(), sandbox));

        let trace_dir = dirs::home_dir()
            .unwrap_or_else(|| cwd.clone())
            .join(".vibecli")
            .join("traces");
        let trace_writer = TraceWriter::new(trace_dir);

        let fmt = ci::CiOutputFormat::from_str(&cli.output_format);
        let verbose = fmt == ci::CiOutputFormat::Verbose;

        let report = ci::run_ci(
            &task,
            ApprovalPolicy::from_str(&approval_policy),
            llm,
            executor,
            Some(trace_writer),
            verbose,
        )
        .await?;

        let output_text = match fmt {
            ci::CiOutputFormat::Markdown => report.to_markdown(),
            _ => serde_json::to_string_pretty(&report)?,
        };

        if let Some(out_path) = cli.output {
            std::fs::write(&out_path, &output_text)?;
            eprintln!("Report written to: {}", out_path);
        } else {
            println!("{}", output_text);
        }

        std::process::exit(report.exit_code());
    }

    // Non-TUI agent mode: --agent "task description"
    if let Some(task) = cli.agent {
        let llm = create_provider(&cli.provider, cli.model.clone())?;

        // Parallel multi-agent mode
        if let Some(n) = cli.parallel {
            return run_parallel_agents(llm, &task, &approval_policy, n).await;
        }

        return run_agent_repl_with_context(
            llm, &task, &approval_policy,
            cli.resume.as_deref(),
            cli.plan,
        ).await;
    }

    // --resume without --agent: list sessions or show usage
    if let Some(sid) = &cli.resume {
        let cwd = std::env::current_dir()?;
        let trace_dir = dirs::home_dir()
            .unwrap_or_else(|| cwd.clone())
            .join(".vibecli")
            .join("traces");
        let sessions = list_traces(&trace_dir);
        if let Some(session) = sessions.iter().find(|s| s.session_id.starts_with(sid.as_str())) {
            println!("📋 Session {} found ({} trace steps)", &session.session_id, session.step_count);
            println!("Use: vibecli --agent \"<task to continue>\" --resume {}", &session.session_id[..8]);
        } else {
            eprintln!("❌ No session found with ID prefix: {}", sid);
        }
        return Ok(());
    }

    println!("🤖 VibeCLI - AI-Powered Coding Assistant");
    println!("Provider: {}", cli.provider);
    println!("\nAvailable commands:");
    println!("  /chat <message>    - Chat with AI");
    println!("  /agent <task>      - Run autonomous coding agent");
    println!("  /generate <prompt> - Generate code");
    println!("  /diff <file>       - Show diff for file");
    println!("  /apply <file>      - Apply changes to file");
    println!("  /exec <command>    - Execute command with AI");
    println!("  /config            - Show configuration");
    println!("  /help              - Show this help");
    println!("  /exit              - Exit VibeCLI");
    println!("\nType a message to chat, or use a command.\n");

    let llm = create_provider(&cli.provider, cli.model)?;

    // Load project memory (VIBECLI.md / AGENTS.md / CLAUDE.md) and inject as system context
    let cwd = std::env::current_dir()?;
    let memory = ProjectMemory::load(&cwd);
    if !memory.is_empty() {
        println!("📚 {}", memory.summary());
    }

    let mut messages: Vec<Message> = Vec::new();
    // Seed system message with memory if available
    if let Some(mem_content) = memory.combined() {
        messages.push(Message {
            role: MessageRole::System,
            content: mem_content,
        });
    }
    let mut conversation_active = false;

    let config = rustyline::Config::builder().auto_add_history(true).build();
    let mut rl = rustyline::Editor::with_config(config)?;
    rl.set_helper(Some(repl::VibeHelper::new()));

    let history_path = dirs::home_dir().map(|h| h.join(".vibecli").join("history.txt"));
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

                // Direct shell command
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
                    }
                    continue;
                }

                if input.starts_with('/') {
                    let parts: Vec<&str> = input.splitn(2, ' ').collect();
                    let command = parts[0];
                    let args = if parts.len() > 1 { parts[1] } else { "" };

                    match command {
                        "/exit" | "/quit" => {
                            println!("👋 Goodbye!");
                            break;
                        }
                        "/help" => show_help(),
                        "/config" => show_config().await?,
                        "/agent" => {
                            if args.is_empty() {
                                println!("Usage: /agent <task description>");
                                continue;
                            }
                            run_agent_repl_with_context(
                                llm.clone(), args, &approval_policy, None, false
                            ).await?;
                        }
                        "/plan" => {
                            if args.is_empty() {
                                println!("Usage: /plan <task description>");
                                continue;
                            }
                            run_agent_repl_with_context(
                                llm.clone(), args, &approval_policy, None, true
                            ).await?;
                        }
                        "/resume" => {
                            let trace_dir = dirs::home_dir()
                                .unwrap_or_else(|| cwd.clone())
                                .join(".vibecli")
                                .join("traces");
                            let sessions = list_traces(&trace_dir);
                            match args {
                                "" => {
                                    // List resumable sessions
                                    let resumable: Vec<_> = sessions.iter()
                                        .filter(|s| {
                                            trace_dir.join(format!("{}-messages.json", s.session_id)).exists()
                                        })
                                        .take(10)
                                        .collect();
                                    if resumable.is_empty() {
                                        println!("📋 No resumable sessions (sessions must have saved messages)");
                                    } else {
                                        println!("\n📋 Resumable sessions:");
                                        for s in resumable {
                                            let elapsed = std::time::Duration::from_secs(
                                                std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap_or_default()
                                                    .as_secs()
                                                    .saturating_sub(s.timestamp)
                                            );
                                            let age = if elapsed.as_secs() < 3600 {
                                                format!("{}m ago", elapsed.as_secs() / 60)
                                            } else {
                                                format!("{}h ago", elapsed.as_secs() / 3600)
                                            };
                                            println!("  {} — {} steps — {}", &s.session_id[..8], s.step_count, age);
                                        }
                                        println!("\nUse: /resume <id_prefix> <task to continue>");
                                    }
                                }
                                _ => {
                                    let parts: Vec<&str> = args.splitn(2, ' ').collect();
                                    let sid = parts[0];
                                    let task = if parts.len() > 1 { parts[1] } else { "continue the previous task" };
                                    run_agent_repl_with_context(
                                        llm.clone(), task, &approval_policy, Some(sid), false
                                    ).await?;
                                }
                            }
                        }
                        "/memory" => {
                            match args {
                                "show" | "" => {
                                    let mem = ProjectMemory::load(&cwd);
                                    println!("{}", mem.summary());
                                    if let Some(content) = mem.combined() {
                                        println!("\n{}", content);
                                    }
                                }
                                "edit" => {
                                    let path = ProjectMemory::default_repo_path(&cwd);
                                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                                    let _ = std::process::Command::new(&editor).arg(&path).status();
                                }
                                _ => println!("Usage: /memory [show|edit]"),
                            }
                        }
                        "/trace" => {
                            let trace_dir = dirs::home_dir()
                                .unwrap_or_else(|| cwd.clone())
                                .join(".vibecli")
                                .join("traces");
                            let sessions = list_traces(&trace_dir);
                            let parts: Vec<&str> = args.splitn(2, ' ').collect();
                            match parts[0] {
                                "view" if parts.len() > 1 => {
                                    // Find session by ID prefix
                                    let id_prefix = parts[1];
                                    if let Some(session) = sessions.iter().find(|s| s.session_id.starts_with(id_prefix)) {
                                        let entries = load_trace(&session.path);
                                        println!("\n📋 Trace: {} ({} steps)\n", session.session_id, entries.len());
                                        for e in &entries {
                                            let icon = if e.success { "✅" } else { "❌" };
                                            println!("{} Step {}: {} — {} ({}ms, {})", icon, e.step + 1, e.tool, e.input_summary, e.duration_ms, e.approved_by);
                                            if !e.output.is_empty() {
                                                let preview: String = e.output.lines().take(3).collect::<Vec<_>>().join("\n");
                                                println!("   {}\n", preview);
                                            }
                                        }
                                    } else {
                                        println!("❌ No trace found with ID prefix: {}", id_prefix);
                                    }
                                }
                                _ => {
                                    // List sessions
                                    if sessions.is_empty() {
                                        println!("📋 No traces found in {}", trace_dir.display());
                                    } else {
                                        println!("\n📋 Recent agent traces ({})\n", trace_dir.display());
                                        for session in sessions.iter().take(10) {
                                            let dt = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(session.timestamp);
                                            let elapsed = std::time::SystemTime::now().duration_since(dt).unwrap_or_default();
                                            let age = if elapsed.as_secs() < 3600 {
                                                format!("{}m ago", elapsed.as_secs() / 60)
                                            } else if elapsed.as_secs() < 86400 {
                                                format!("{}h ago", elapsed.as_secs() / 3600)
                                            } else {
                                                format!("{}d ago", elapsed.as_secs() / 86400)
                                            };
                                            println!("  {} — {} steps — {}", &session.session_id[..8], session.step_count, age);
                                        }
                                        println!("\nUse: /trace view <id_prefix>\n");
                                    }
                                }
                            }
                        }
                        "/mcp" => {
                            let config = Config::load().unwrap_or_default();
                            if config.mcp_servers.is_empty() {
                                println!("No MCP servers configured.\nAdd [[mcp_servers]] to ~/.vibecli/config.toml\n");
                                continue;
                            }
                            let mcp_parts: Vec<&str> = args.splitn(3, ' ').collect();
                            match mcp_parts[0] {
                                "list" | "" => {
                                    println!("\n🔌 Configured MCP servers:");
                                    for srv in &config.mcp_servers {
                                        println!("  {} — {}", srv.name, srv.command);
                                    }
                                    println!("\nUse: /mcp tools <server>  or  /mcp call <server> <tool>\n");
                                }
                                "tools" if mcp_parts.len() > 1 => {
                                    let name = mcp_parts[1];
                                    if let Some(srv_cfg) = config.mcp_servers.iter().find(|s| s.name == name) {
                                        match vibe_ai::mcp::McpClient::connect(srv_cfg) {
                                            Ok(mut client) => {
                                                match client.list_tools() {
                                                    Ok(tools) => {
                                                        println!("\n🔧 Tools from '{}':", name);
                                                        for t in &tools {
                                                            println!("  {} — {}", t.name, t.description);
                                                        }
                                                        println!();
                                                    }
                                                    Err(e) => eprintln!("❌ list_tools failed: {}", e),
                                                }
                                            }
                                            Err(e) => eprintln!("❌ Failed to connect to '{}': {}", name, e),
                                        }
                                    } else {
                                        println!("❌ Unknown MCP server: {}", name);
                                    }
                                }
                                _ => println!("Usage: /mcp [list | tools <server>]\n"),
                            }
                        }
                        "/chat" => {
                            if args.is_empty() {
                                println!("Usage: /chat <message>  or  /chat [image.png] <message>");
                                continue;
                            }
                            conversation_active = true;

                            // Detect [image.png] patterns and load images.
                            let (text_content, images) = extract_images_from_input(args);
                            messages.push(Message {
                                role: MessageRole::User,
                                content: text_content.clone(),
                            });
                            print!("🤖 ");
                            io::stdout().flush()?;
                            let chat_result = if images.is_empty() {
                                llm.chat(&messages, None).await
                            } else {
                                println!("(📷 {} image{})", images.len(), if images.len() > 1 { "s" } else { "" });
                                llm.chat_with_images(&messages, &images, None).await
                            };
                            match chat_result {
                                Ok(response) => {
                                    let highlighted = highlight_code_blocks(&response);
                                    println!("{}\n", highlighted);
                                    messages.push(Message {
                                        role: MessageRole::Assistant,
                                        content: response,
                                    });
                                }
                                Err(e) => eprintln!("❌ Error: {}\n", e),
                            }
                        }
                        "/generate" => {
                            if args.is_empty() {
                                println!("Usage: /generate <prompt>");
                                continue;
                            }
                            println!("🔨 Generating code...");
                            let gen_messages = vec![
                                Message {
                                    role: MessageRole::System,
                                    content: "You are a code generation assistant. Generate clean, well-documented code based on the user's request. Only output the code.".to_string(),
                                },
                                Message { role: MessageRole::User, content: args.to_string() },
                            ];
                            match llm.chat(&gen_messages, None).await {
                                Ok(response) => {
                                    println!("\n{}\n", highlight_code_blocks(&response));
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
                                        let clean = response
                                            .lines()
                                            .filter(|l| !l.starts_with("```"))
                                            .collect::<Vec<_>>()
                                            .join("\n");
                                        std::fs::write(filename, &clean)?;
                                        println!("✅ Saved to: {}\n", filename);
                                    }
                                }
                                Err(e) => eprintln!("❌ Error: {}\n", e),
                            }
                        }
                        "/diff" => {
                            if args.is_empty() {
                                println!("Usage: /diff <file>");
                                continue;
                            }
                            match DiffViewer::show_file_diff(args) {
                                Ok(_) => {}
                                Err(e) => eprintln!("❌ Error showing diff: {}\n", e),
                            }
                        }
                        "/apply" => {
                            let parts: Vec<&str> = args.splitn(2, ' ').collect();
                            if parts.len() < 2 {
                                println!("Usage: /apply <file> <description of changes>");
                                continue;
                            }
                            let file_path = parts[0];
                            let change_description = parts[1];
                            println!("🔨 Generating changes for: {}", file_path);
                            match std::fs::read_to_string(file_path) {
                                Ok(current_content) => {
                                    let apply_messages = vec![
                                        Message {
                                            role: MessageRole::System,
                                            content: "You are a code modification assistant. Given the current file content and a description of changes, output ONLY the modified file content.".to_string(),
                                        },
                                        Message {
                                            role: MessageRole::User,
                                            content: format!("Current file:\n```\n{}\n```\n\nChanges: {}\n\nOutput the complete modified file:", current_content, change_description),
                                        },
                                    ];
                                    match llm.chat(&apply_messages, None).await {
                                        Ok(modified_content) => {
                                            let clean = modified_content
                                                .lines()
                                                .filter(|l| !l.starts_with("```"))
                                                .collect::<Vec<_>>()
                                                .join("\n");
                                            println!("\n📊 Proposed changes:\n");
                                            if let Err(e) = DiffViewer::show_diff(file_path, &current_content, &clean) {
                                                eprintln!("Warning: Could not show diff: {}", e);
                                            }
                                            print!("✅ Apply these changes? (y/N): ");
                                            io::stdout().flush()?;
                                            let mut confirm = String::new();
                                            io::stdin().read_line(&mut confirm)?;
                                            if confirm.trim().to_lowercase() == "y" {
                                                match std::fs::write(file_path, &clean) {
                                                    Ok(_) => println!("✅ Changes applied to: {}\n", file_path),
                                                    Err(e) => eprintln!("❌ Failed to write file: {}\n", e),
                                                }
                                            } else {
                                                println!("❌ Changes cancelled\n");
                                            }
                                        }
                                        Err(e) => eprintln!("❌ Error generating changes: {}\n", e),
                                    }
                                }
                                Err(e) => eprintln!("❌ Failed to read file: {}\n", e),
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
                                    content: "You are a command-line assistant. Generate a single shell command. Only output the command, nothing else.".to_string(),
                                },
                                Message { role: MessageRole::User, content: args.to_string() },
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
                                Err(e) => eprintln!("❌ Error: {}\n", e),
                            }
                        }
                        _ => {
                            println!("❌ Unknown command: {}", command);
                            println!("Type /help for available commands\n");
                        }
                    }
                } else {
                    // Regular chat
                    if !conversation_active {
                        messages.clear();
                        conversation_active = true;
                        messages.push(Message {
                            role: MessageRole::System,
                            content: "You are a helpful coding assistant. If the user asks you to run a command, output it in a ```execute block.".to_string(),
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
                                content: response,
                            });
                        }
                        Err(e) => eprintln!("❌ Error: {}\n", e),
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

    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }
    Ok(())
}

// ── ExecutorFactory implementation ───────────────────────────────────────────

/// An `ExecutorFactory` that creates a `ToolExecutor` for a given workspace root.
struct VibeExecutorFactory {
    sandbox: bool,
    env_policy: tool_executor::ShellEnvPolicy,
}

impl ExecutorFactory for VibeExecutorFactory {
    fn create(&self, workspace_root: std::path::PathBuf) -> Arc<dyn vibe_ai::agent::ToolExecutorTrait> {
        Arc::new(
            ToolExecutor::new(workspace_root, self.sandbox)
                .with_env_policy(self.env_policy.clone()),
        )
    }
}

// ── Parallel multi-agent runner ───────────────────────────────────────────────

async fn run_parallel_agents(
    llm: Arc<dyn LLMProvider>,
    task: &str,
    approval_policy: &str,
    n: usize,
) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let config = Config::load().unwrap_or_default();
    let approval = ApprovalPolicy::from_str(approval_policy);
    let sandbox = config.safety.sandbox;
    let env_policy = config.safety.shell_environment.to_policy();

    let factory = Arc::new(VibeExecutorFactory { sandbox, env_policy });
    let manager = Arc::new(VibeCoreWorktreeManager::new(workspace.clone()));

    let mut orchestrator = MultiAgentOrchestrator::new(llm, approval, factory)
        .with_worktree_manager(manager)
        .with_max_agents(n);

    if !config.hooks.is_empty() {
        orchestrator = orchestrator.with_hooks(HookRunner::new(config.hooks));
    }

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(128);

    println!("🚀 Starting {} parallel agents for task: {}", n, task);
    println!("   Approval: {:?}", approval_policy);
    println!("   Workspace: {}", workspace.display());
    println!();

    let task_str = task.to_string();
    let workspace_clone = workspace.clone();
    tokio::spawn(async move {
        let _ = orchestrator.run_parallel(&workspace_clone, &task_str, n, event_tx).await;
    });

    while let Some(event) = event_rx.recv().await {
        match event {
            OrchestratorEvent::AgentStarted { id, task, worktree } => {
                println!("[Agent {}] Started — worktree: {}", id, worktree.display());
                println!("[Agent {}] Task: {}", id, task);
            }
            OrchestratorEvent::AgentChunk { id, text } => {
                print!("[Agent {}] {}", id, text);
                io::stdout().flush()?;
            }
            OrchestratorEvent::AgentStep { id, step } => {
                let icon = if step.approved { "✅" } else { "❌" };
                println!("\n[Agent {}] {} Step {}: {}", id, icon, step.step_num, step.tool_call.summary());
            }
            OrchestratorEvent::AgentComplete { id, summary, branch } => {
                println!("\n[Agent {}] ✅ Complete — branch: {}", id, branch);
                println!("[Agent {}] Summary: {}", id, summary);
            }
            OrchestratorEvent::AgentError { id, error } => {
                println!("\n[Agent {}] ❌ Error: {}", id, error);
            }
            OrchestratorEvent::AllComplete { results } => {
                println!("\n\n=== All {} agents complete ===\n", results.len());
                let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
                println!("✅ Succeeded: {}/{}", successful.len(), results.len());
                for r in &results {
                    let icon = if r.success { "✅" } else { "❌" };
                    println!("  {} Agent {} — branch: {} ({} steps)", icon, r.id, r.branch, r.steps_taken);
                    if !r.summary.is_empty() {
                        let preview: String = r.summary.lines().next().unwrap_or("").to_string();
                        println!("     {}", preview);
                    }
                }
                if !successful.is_empty() {
                    println!("\nTo merge the best result:");
                    println!("  git merge {} --no-ff", successful[0].branch);
                }
                break;
            }
        }
    }

    Ok(())
}

async fn run_agent_repl_with_context(
    llm: Arc<dyn LLMProvider>,
    task: &str,
    approval_policy: &str,
    resume_session_id: Option<&str>,
    plan_mode: bool,
) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let config = Config::load().unwrap_or_default();
    let approval = ApprovalPolicy::from_str(approval_policy);
    let sandbox = config.safety.sandbox;

    // Apply shell env policy
    let env_policy = config.safety.shell_environment.to_policy();
    let executor: Arc<dyn vibe_ai::agent::ToolExecutorTrait> =
        Arc::new(ToolExecutor::new(workspace.clone(), sandbox).with_env_policy(env_policy));

    // Build hooks from config
    let hook_runner = if config.hooks.is_empty() {
        HookRunner::empty()
    } else {
        HookRunner::new(config.hooks.clone())
    };
    let agent = AgentLoop::new(llm.clone(), approval.clone(), executor.clone())
        .with_hooks(hook_runner);

    let trace_dir = dirs::home_dir()
        .unwrap_or_else(|| workspace.clone())
        .join(".vibecli")
        .join("traces");

    // Plan mode: generate plan before executing
    let approved_plan: Option<String> = if plan_mode {
        println!("🧠 Generating execution plan...\n");
        let planner = PlannerAgent::new(llm.clone());
        let ctx = AgentContext {
            workspace_root: workspace.clone(),
            ..Default::default()
        };
        match planner.plan(task, &ctx).await {
            Ok(plan) => {
                println!("{}", plan.display());
                print!("Execute this plan? (y/N/edit): ");
                io::stdout().flush()?;
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                let answer = confirm.trim().to_lowercase();
                if answer != "y" && answer != "yes" {
                    println!("❌ Plan cancelled");
                    return Ok(());
                }
                Some(plan.display())
            }
            Err(e) => {
                eprintln!("⚠️  Plan generation failed: {} — proceeding without plan", e);
                None
            }
        }
    } else {
        None
    };

    // Session resume: load previous messages if --resume
    let resumed_messages: Vec<Message> = if let Some(sid_prefix) = resume_session_id {
        let sessions = list_traces(&trace_dir);
        if let Some(session) = sessions.iter().find(|s| s.session_id.starts_with(sid_prefix)) {
            match load_session(&session.session_id, &trace_dir) {
                Some(snapshot) if !snapshot.messages.is_empty() => {
                    println!("▶️  Resuming session {} ({} messages, {} trace steps)",
                        &session.session_id[..8.min(session.session_id.len())],
                        snapshot.messages.len(),
                        snapshot.trace.len()
                    );
                    snapshot.messages
                }
                _ => {
                    println!("⚠️  Session found but no saved messages — starting fresh");
                    vec![]
                }
            }
        } else {
            eprintln!("❌ No session found with ID prefix: {}", sid_prefix);
            return Ok(());
        }
    } else {
        vec![]
    };

    let context = AgentContext {
        workspace_root: workspace.clone(),
        approved_plan,
        ..Default::default()
    };

    let trace = TraceWriter::new(trace_dir.clone());

    println!("🤖 Agent starting: {}", task);
    println!("   Approval policy: {:?}", approval);
    println!("   Trace: {}", trace.path().display());
    if !resumed_messages.is_empty() {
        println!("   Resuming {} prior messages", resumed_messages.len());
    }
    println!("   Press Ctrl+C to stop\n");

    // Save messages on completion for future resume
    let trace_for_save = TraceWriter::new(trace_dir);

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<AgentEvent>(50);
    let task_str = task.to_string();
    let context_clone = context.clone();
    tokio::spawn(async move {
        let _ = agent.run(&task_str, context_clone, event_tx).await;
    });

    let mut step_start = std::time::Instant::now();

    while let Some(event) = event_rx.recv().await {
        match event {
            AgentEvent::StreamChunk(text) => {
                print!("{}", text);
                io::stdout().flush()?;
            }
            AgentEvent::ToolCallPending { call, result_tx } => {
                println!("\n\n⚡ Tool call: {}", call.summary());
                print!("   Approve? (y/n/a=approve-all): ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let answer = input.trim().to_lowercase();
                let dur = step_start.elapsed().as_millis() as u64;
                step_start = std::time::Instant::now();

                if answer == "n" {
                    println!("   ❌ Rejected\n");
                    trace.record(0, call.name(), &call.summary(), "rejected by user", false, dur, "rejected");
                    let _ = result_tx.send(None);
                } else {
                    // Execute the tool
                    let result = executor.execute(&call).await;
                    let status = if result.success { "✅" } else { "❌" };
                    println!("   {} {}\n", status, &result.output.lines().next().unwrap_or(""));
                    trace.record(0, call.name(), &call.summary(), &result.output, result.success, dur, "user");
                    let _ = result_tx.send(Some(result));
                }
            }
            AgentEvent::ToolCallExecuted(step) => {
                let dur = step_start.elapsed().as_millis() as u64;
                step_start = std::time::Instant::now();
                let status = if step.tool_result.success { "✅" } else { "❌" };
                println!(
                    "\n{} Step {}: {}",
                    status,
                    step.step_num + 1,
                    step.tool_call.summary()
                );
                trace.record(
                    step.step_num,
                    step.tool_call.name(),
                    &step.tool_call.summary(),
                    &step.tool_result.output,
                    step.tool_result.success,
                    dur,
                    "auto",
                );
            }
            AgentEvent::Complete(summary) => {
                println!("\n\n✅ Agent complete: {}", summary);
                println!("   Trace saved: {}", trace.path().display());
                println!("   Resume with: vibecli --resume {}", trace.session_id());
                // Save context for future resume
                let _ = trace_for_save.save_context(&context);
                break;
            }
            AgentEvent::Error(e) => {
                eprintln!("\n❌ Agent error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

/// Detect `[path/to/image.png]` patterns in `input`, load images, return (clean_text, images).
fn extract_images_from_input(input: &str) -> (String, Vec<ImageAttachment>) {
    let re = Regex::new(r"\[([^\]]+\.(png|jpg|jpeg|gif|webp))\]").unwrap();
    let mut images = Vec::new();

    // First pass: collect images.
    for caps in re.captures_iter(input) {
        let img_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        match ImageAttachment::from_path(std::path::Path::new(img_path)) {
            Ok(img) => images.push(img),
            Err(e) => eprintln!("⚠️  Could not load image '{}': {}", img_path, e),
        }
    }

    // Second pass: strip image markers from text.
    let clean = re.replace_all(input, "").trim().to_string();
    (clean, images)
}

fn create_provider(provider_name: &str, model: Option<String>) -> Result<Arc<dyn LLMProvider>> {
    match provider_name.to_lowercase().as_str() {
        "ollama" => {
            let model = model.unwrap_or_else(|| "qwen3-coder:480b-cloud".to_string());
            Ok(Arc::new(OllamaProvider::new(ProviderConfig {
                provider_type: "ollama".to_string(),
                api_url: Some("http://localhost:11434".to_string()),
                model,
                api_key: None,
                max_tokens: None,
                temperature: None,
            })))
        }
        _ => anyhow::bail!("Unknown provider: {}", provider_name),
    }
}

fn show_help() {
    println!("\n📚 VibeCLI Commands:");
    println!("  /chat <message>          - Chat with AI (supports [image.png] for vision)");
    println!("  /agent <task>            - Run autonomous coding agent on a task");
    println!("  /plan <task>             - Generate execution plan, then run agent");
    println!("  /resume [id] [task]      - List resumable sessions or resume one");
    println!("  /generate <prompt>       - Generate code from a description");
    println!("  /diff <file>             - Show diff for a file");
    println!("  /apply <file>            - Apply AI-suggested changes to a file");
    println!("  /exec <task>             - Generate and execute a shell command");
    println!("  /memory [show]           - Show loaded project memory (VIBECLI.md / AGENTS.md)");
    println!("  /memory edit             - Open project memory in $EDITOR");
    println!("  /trace                   - List recent agent trace sessions");
    println!("  /trace view <id>         - View a trace session timeline");
    println!("  /mcp list                - List configured MCP servers");
    println!("  /mcp tools <server>      - List tools on an MCP server");
    println!("  /config                  - Show current configuration");
    println!("  /help                    - Show this help message");
    println!("  /exit                    - Exit VibeCLI");
    println!("  ! <command>              - Execute shell command directly (e.g. !ls)");
    println!("\nCLI flags:");
    println!("  --agent <task>           - Run agent in REPL mode");
    println!("  --plan                   - Enable plan mode (generate plan before executing)");
    println!("  --resume <session-id>    - Resume a previous agent session");
    println!("  --exec <task>            - CI/non-interactive agent mode");
    println!("  --suggest                - Prompt before every tool call (default)");
    println!("  --auto-edit              - Auto file edits, prompt for bash");
    println!("  --full-auto              - Execute all tool calls autonomously");
    println!("  --output-format json|md  - Report format for --exec");
    println!("  --output <file>          - Write --exec report to file");
    println!("\nMultimodal:");
    println!("  /chat [screenshot.png] What is this error?  - Attach image to chat");
    println!("\n💡 Tip: You can also just type a message to chat\n");
}

async fn show_config() -> Result<()> {
    match Config::load() {
        Ok(config) => {
            println!("\n⚙️  Configuration:");
            println!("  Location: ~/.vibecli/config.toml");
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
            println!("    Approval policy: {}", config.safety.approval_policy);
            println!("    Require approval for commands: {}", config.safety.require_approval_for_commands);
            println!("    Require approval for file changes: {}", config.safety.require_approval_for_file_changes);
            println!("    Sandbox: {}", config.safety.sandbox);
            println!();
        }
        Err(e) => {
            println!("❌ Failed to load config: {}", e);
        }
    }
    Ok(())
}
