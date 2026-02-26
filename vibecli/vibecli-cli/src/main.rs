use clap::Parser;
use anyhow::Result;
use crate::config::Config;
use crate::syntax::highlight_code_blocks;
use vibe_ai::provider::{AIProvider as LLMProvider, ImageAttachment, Message, MessageRole, ProviderConfig, TokenUsage};
use vibe_ai::providers::ollama::OllamaProvider;
use vibe_ai::agent::{AgentContext, AgentEvent, AgentLoop, ApprovalPolicy};
use vibe_ai::{MultiAgentOrchestrator, OrchestratorEvent, ExecutorFactory};
use vibe_ai::hooks::HookRunner;
use vibe_ai::planner::PlannerAgent;
use vibe_ai::trace::{list_traces, load_session, load_trace, TraceWriter};
use vibe_core::index::embeddings::{EmbeddingIndex, EmbeddingProvider};
use regex::Regex;
use std::io::{self, Write};
use std::sync::Arc;

mod config;
mod schema;
mod syntax;
mod diff_viewer;
mod tool_executor;
mod memory;
mod memory_recorder;
mod ci;
mod review;
mod serve;
mod mcp_server;
mod otel_init;
mod plugin;
use plugin::PluginLoader;
mod profile;
use profile::{ProfileManager, ProfileOverrides};
use tool_executor::{ToolExecutor, VibeCoreWorktreeManager};
use diff_viewer::DiffViewer;
use memory::ProjectMemory;

mod repl;
mod spec;
mod background_agents;
mod team;
use rustyline::error::ReadlineError;

mod tui;

#[derive(Parser)]
#[command(name = "vibecli")]
#[command(version)]
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

    // ── Review mode ───────────────────────────────────────────────────────────

    /// Run code review on git changes.
    #[arg(long)]
    review: bool,

    /// Base ref for review diff (default: compare uncommitted changes).
    #[arg(long, value_name = "REF")]
    base: Option<String>,

    /// Target ref for review diff (default: working tree).
    #[arg(long, value_name = "REF")]
    branch: Option<String>,

    /// Post review as a comment on a GitHub PR number.
    #[arg(long, value_name = "PR")]
    pr: Option<u32>,

    /// Post the review to GitHub (requires --pr and GITHUB_TOKEN).
    #[arg(long)]
    post_github: bool,

    // ── Daemon mode ───────────────────────────────────────────────────────────

    /// Start the VibeCLI HTTP daemon (for VS Code extension / Agent SDK).
    #[arg(long)]
    serve: bool,

    /// Port for daemon mode (default: 7878).
    #[arg(long, default_value = "7878")]
    port: u16,

    // ── MCP server mode ───────────────────────────────────────────────────────

    /// Run as an MCP (Model Context Protocol) server over stdio.
    /// Exposes read_file, write_file, list_directory, bash, search_files,
    /// and agent_run as MCP tools. Add to Claude Desktop config.json:
    /// { "mcpServers": { "vibecli": { "command": "vibecli", "args": ["--mcp-server"] } } }
    #[arg(long)]
    mcp_server: bool,

    // ── Profile ───────────────────────────────────────────────────────────────

    /// Load a named configuration profile from ~/.vibecli/profiles/<name>.toml.
    /// Profiles override provider, model, and approval_policy from the base config.
    /// Example: --profile work  (loads ~/.vibecli/profiles/work.toml)
    #[arg(long, value_name = "PROFILE")]
    profile: Option<String>,

    // ── Doctor / health check ─────────────────────────────────────────────────

    /// Run a health check of the VibeCLI installation (providers, config, tools).
    #[arg(long)]
    doctor: bool,

    // ── Phase 12 additions ────────────────────────────────────────────────────

    /// Name this session (used as prefix for trace files, e.g. --session-name debug-auth).
    #[arg(long, value_name = "NAME")]
    session_name: Option<String>,

    /// Attach one or more image files to the first chat message (vision).
    /// Example: -i screenshot.png -i diagram.jpg
    #[arg(short = 'i', long = "image", value_name = "FILE", action = clap::ArgAction::Append)]
    image: Vec<String>,

    /// Add an extra directory to the context for the agent (besides cwd).
    /// Can be repeated: --add-dir src --add-dir tests
    #[arg(long, value_name = "DIR", action = clap::ArgAction::Append)]
    add_dir: Vec<String>,

    /// Emit each agent event as a JSON line to stdout (machine-readable mode).
    #[arg(long)]
    json: bool,

    /// Validate the --exec JSON report against a JSON Schema file.
    /// Exits non-zero when the output does not conform to the schema.
    /// Example: --output-schema schema.json
    #[arg(long, value_name = "SCHEMA_FILE")]
    output_schema: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing (with optional OTLP export if [otel] enabled = true).
    let otel_config = Config::load()
        .map(|c| c.otel.clone())
        .unwrap_or_default();
    // Keep the guard alive for the entire program.
    let _otel_guard = otel_init::setup(&otel_config)?;

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

    // ── Doctor mode ───────────────────────────────────────────────────────────
    if cli.doctor {
        return run_doctor().await;
    }

    // ── Profile resolution ────────────────────────────────────────────────────
    // Load a named profile and use it to override provider / model / approval.
    // Priority: CLI flags > profile > base config.
    let (effective_provider, effective_model, approval_policy) =
        if let Some(profile_name) = cli.profile.as_deref() {
            match ProfileOverrides::load(profile_name) {
                Ok(ov) => {
                    // Profile overrides base; CLI explicit flags override profile.
                    let provider = ov.provider.unwrap_or_else(|| cli.provider.clone());
                    let model = cli.model.clone().or(ov.model);
                    let policy = if cli.suggest || cli.auto_edit || cli.full_auto {
                        approval_policy.clone()
                    } else {
                        ov.approval_policy.unwrap_or_else(|| approval_policy.clone())
                    };
                    eprintln!("📋 Profile '{}' → provider={}, approval={}", profile_name, provider, policy);
                    (provider, model, policy)
                }
                Err(e) => {
                    eprintln!("⚠️  Profile '{}' not found: {}", profile_name, e);
                    (cli.provider.clone(), cli.model.clone(), approval_policy.clone())
                }
            }
        } else {
            (cli.provider.clone(), cli.model.clone(), approval_policy.clone())
        };

    // Daemon mode: vibecli serve [--port 7878]
    if cli.serve {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let cwd = std::env::current_dir()?;
        let approval = ApprovalPolicy::from_str(&approval_policy);
        return serve::serve(llm, effective_provider.clone(), approval, cwd, cli.port).await;
    }

    // MCP server mode: vibecli --mcp-server
    if cli.mcp_server {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let cwd = std::env::current_dir()?;
        let approval = ApprovalPolicy::from_str(&approval_policy);
        let config = Config::load().unwrap_or_default();
        return mcp_server::run_server(cwd, llm, approval, config.safety.sandbox).await;
    }

    if cli.tui {
        return tui::run(effective_provider, effective_model).await;
    }

    // Non-interactive CI/exec mode: --exec "task"
    if let Some(task) = cli.exec {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
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

        // Optionally validate the JSON output against a JSON Schema.
        if let Some(schema_path) = &cli.output_schema {
            let schema_bytes = std::fs::read_to_string(schema_path)
                .map_err(|e| anyhow::anyhow!("Cannot read schema file '{}': {}", schema_path, e))?;
            let schema_val: serde_json::Value = serde_json::from_str(&schema_bytes)
                .map_err(|e| anyhow::anyhow!("Invalid JSON in schema file '{}': {}", schema_path, e))?;
            let report_val: serde_json::Value = serde_json::from_str(&output_text)
                .unwrap_or(serde_json::Value::Null);
            if let Err(errs) = schema::validate(&report_val, &schema_val) {
                eprintln!("❌ Output does not conform to schema '{}':", schema_path);
                for e in &errs {
                    eprintln!("   • {}", e);
                }
                std::process::exit(2);
            } else {
                eprintln!("✅ Output conforms to schema '{}'", schema_path);
            }
        }

        if let Some(out_path) = cli.output {
            std::fs::write(&out_path, &output_text)?;
            eprintln!("Report written to: {}", out_path);
        } else {
            println!("{}", output_text);
        }

        std::process::exit(report.exit_code());
    }

    // Code review mode: --review
    if cli.review {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let workspace = std::env::current_dir()?;
        let config = review::ReviewConfig {
            base_ref: cli.base.unwrap_or_default(),
            target_ref: cli.branch.unwrap_or_default(),
            post_to_github: cli.post_github,
            github_pr: cli.pr,
            workspace,
            ..Default::default()
        };

        println!("🔍 Running code review...");
        if !config.base_ref.is_empty() {
            println!("   Base: {}", config.base_ref);
        }
        if !config.target_ref.is_empty() {
            println!("   Target: {}", config.target_ref);
        }

        let report = review::run_review(&config, llm).await?;
        let markdown = report.to_markdown();
        println!("{}", markdown);

        if config.post_to_github {
            if let Some(pr) = config.github_pr {
                print!("📤 Posting review to PR #{}... ", pr);
                io::stdout().flush()?;
                match review::post_to_github_pr(pr, &markdown) {
                    Ok(_) => println!("✅ Posted!"),
                    Err(e) => eprintln!("❌ Failed to post: {}", e),
                }
            } else {
                eprintln!("❌ --post-github requires --pr <number>");
            }
        }

        std::process::exit(report.exit_code());
    }

    // Non-TUI agent mode: --agent "task description"
    if let Some(task) = cli.agent {
        // ── opusplan routing ─────────────────────────────────────────────────
        // If [routing] is configured, use separate providers for planning vs execution.
        let config_for_routing = Config::load().unwrap_or_default();
        let (exec_provider, exec_model) = if config_for_routing.routing.is_configured() {
            let (ep, em) = config_for_routing.routing.resolve_execution(
                &effective_provider,
                effective_model.as_deref().unwrap_or(""),
            );
            if ep != effective_provider || Some(em.as_str()) != effective_model.as_deref() {
                eprintln!("🔀 opusplan routing: execution → {}:{}", ep, em);
            }
            (ep, Some(em))
        } else {
            (effective_provider.clone(), effective_model.clone())
        };
        let llm = create_provider(&exec_provider, exec_model)?;

        // Build planning LLM only when --plan is requested and routing is configured.
        let planning_llm: Option<Arc<dyn LLMProvider>> = if cli.plan && config_for_routing.routing.is_configured() {
            let (pp, pm) = config_for_routing.routing.resolve_planning(
                &effective_provider,
                effective_model.as_deref().unwrap_or(""),
            );
            if pp != effective_provider || Some(pm.as_str()) != effective_model.as_deref() {
                eprintln!("🔀 opusplan routing: planning → {}:{}", pp, pm);
            }
            Some(create_provider(&pp, Some(pm))?)
        } else {
            None
        };

        // Parallel multi-agent mode
        if let Some(n) = cli.parallel {
            return run_parallel_agents(llm, &task, &approval_policy, n).await;
        }

        return run_agent_repl_with_context(
            llm, &task, &approval_policy,
            cli.resume.as_deref(),
            cli.plan,
            cli.json,
            planning_llm,
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
    print!("Provider: {}", effective_provider);
    if let Some(ref m) = effective_model {
        print!(" / {}", m);
    }
    if let Some(ref p) = cli.profile {
        print!("  (profile: {})", p);
    }
    println!();
    println!("\nType a message to chat, use a /command, or /help.\n");

    let mut llm = create_provider(&effective_provider, effective_model.clone())?;
    let mut active_provider = effective_provider.clone();
    let mut active_model = effective_model.clone();
    let session_tokens = TokenUsage::default();

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
                                llm.clone(), args, &approval_policy, None, false, false, None
                            ).await?;
                        }
                        "/plan" => {
                            if args.is_empty() {
                                println!("Usage: /plan <task description>");
                                continue;
                            }
                            run_agent_repl_with_context(
                                llm.clone(), args, &approval_policy, None, true, false, None
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
                                        llm.clone(), task, &approval_policy, Some(sid), false, false, None
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
                        // ── Codebase semantic index ────────────────────────────────────────
                        "/index" => {
                            // Build or refresh the semantic codebase index.
                            let model = if args.is_empty() { "nomic-embed-text" } else { args };
                            println!("🔍 Building semantic index with model '{}' …", model);
                            println!("   (embeds all source files — may take a minute on large repos)");
                            let provider = EmbeddingProvider::ollama(model);
                            match EmbeddingIndex::build(&cwd, &provider).await {
                                Ok(index) => {
                                    let index_path = cwd.join(".vibecli").join("index.json");
                                    if let Some(parent) = index_path.parent() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                    match serde_json::to_string(&index) {
                                        Ok(json) => match std::fs::write(&index_path, json) {
                                            Ok(_) => println!(
                                                "✅ Indexed {} chunks → .vibecli/index.json\n",
                                                index.len()
                                            ),
                                            Err(e) => eprintln!("⚠️  Could not save index: {}\n", e),
                                        },
                                        Err(e) => eprintln!("⚠️  Could not serialise index: {}\n", e),
                                    }
                                }
                                Err(e) => eprintln!("❌ Index build failed: {}\n   Hint: make sure Ollama is running with `ollama pull {}`\n", e, model),
                            }
                        }

                        "/qa" => {
                            // Codebase Q&A using the semantic index.
                            if args.is_empty() {
                                println!("Usage: /qa <question about the codebase>");
                                println!("       Run /index first to build the semantic index.\n");
                                continue;
                            }
                            let index_path = cwd.join(".vibecli").join("index.json");
                            if !index_path.exists() {
                                println!("⚠️  No index found. Run /index first.\n");
                                continue;
                            }
                            let index: EmbeddingIndex = match std::fs::read_to_string(&index_path)
                                .map_err(anyhow::Error::from)
                                .and_then(|s| serde_json::from_str(&s).map_err(anyhow::Error::from))
                            {
                                Ok(i) => i,
                                Err(e) => {
                                    eprintln!("❌ Failed to load index: {}\n", e);
                                    continue;
                                }
                            };
                            println!("🔍 Searching codebase for: {}", args);
                            let hits = match index.search(args, 5).await {
                                Ok(h) => h,
                                Err(e) => {
                                    eprintln!("❌ Search failed: {}\n", e);
                                    continue;
                                }
                            };
                            if hits.is_empty() {
                                println!("No relevant code found. Try re-running /index.\n");
                                continue;
                            }
                            // Build context from top hits.
                            let context = hits
                                .iter()
                                .enumerate()
                                .map(|(i, h)| {
                                    format!(
                                        "=== [{}] {} (lines {}-{}, score={:.2}) ===\n{}",
                                        i + 1,
                                        h.file.display(),
                                        h.chunk_start,
                                        h.chunk_end,
                                        h.score,
                                        h.text
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n\n");
                            let qa_messages = vec![
                                Message {
                                    role: MessageRole::System,
                                    content: format!(
                                        "You are a codebase assistant. Answer questions using \
                                         the following source code sections as context. \
                                         Cite file names and line numbers when relevant.\n\n{}",
                                        context
                                    ),
                                },
                                Message {
                                    role: MessageRole::User,
                                    content: args.to_string(),
                                },
                            ];
                            print!("🤖 ");
                            io::stdout().flush()?;
                            match llm.chat(&qa_messages, None).await {
                                Ok(response) => println!("{}\n", highlight_code_blocks(&response)),
                                Err(e) => eprintln!("❌ Error: {}\n", e),
                            }
                        }

                        // ── Profile management ────────────────────────────────────────────
                        "/profile" => {
                            let mgr = ProfileManager::new();
                            let parts: Vec<&str> = args.splitn(2, ' ').collect();
                            match parts[0] {
                                "list" | "" => {
                                    let profiles = mgr.list();
                                    if profiles.is_empty() {
                                        println!("No profiles installed.");
                                        println!("Create with: /profile create <name> [provider] [approval]\n");
                                    } else {
                                        println!("Profiles ({}):", profiles.len());
                                        for (name, desc) in &profiles {
                                            println!("  {}  {}", name,
                                                if desc.is_empty() { String::new() } else { format!("— {}", desc) });
                                        }
                                        println!();
                                    }
                                }
                                "show" => {
                                    let name = if parts.len() > 1 { parts[1].trim() } else { "" };
                                    if name.is_empty() {
                                        println!("Usage: /profile show <name>\n");
                                        continue;
                                    }
                                    match mgr.load(name) {
                                        Ok(p) => {
                                            println!("Profile: {}", name);
                                            if !p.description.is_empty() {
                                                println!("  {}", p.description);
                                            }
                                            if let Some(prov) = &p.provider {
                                                if let Some(t) = &prov.provider_type { println!("  Provider: {}", t); }
                                                if let Some(m) = &prov.model { println!("  Model: {}", m); }
                                            }
                                            if let Some(s) = &p.safety {
                                                if let Some(a) = &s.approval_policy { println!("  Approval: {}", a); }
                                                if let Some(sb) = s.sandbox { println!("  Sandbox: {}", sb); }
                                            }
                                            println!();
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "create" => {
                                    let rest = if parts.len() > 1 { parts[1].trim() } else { "" };
                                    let mut words = rest.splitn(3, ' ');
                                    let name = words.next().unwrap_or("").trim();
                                    let provider = words.next().unwrap_or("ollama").trim();
                                    let approval = words.next().unwrap_or("suggest").trim();
                                    if name.is_empty() {
                                        println!("Usage: /profile create <name> [provider] [approval]\n");
                                        continue;
                                    }
                                    match mgr.create(name, provider, approval) {
                                        Ok(path) => println!("✅ Created profile '{}' at {}\n", name, path.display()),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "delete" | "remove" => {
                                    let name = if parts.len() > 1 { parts[1].trim() } else { "" };
                                    if name.is_empty() {
                                        println!("Usage: /profile delete <name>\n");
                                        continue;
                                    }
                                    match mgr.delete(name) {
                                        Ok(()) => println!("🗑️  Deleted profile '{}'\n", name),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                _ => println!("Usage: /profile [list|show|create|delete]\n"),
                            }
                        }

                        // ── Plugin management ─────────────────────────────────────────────
                        "/plugin" => {
                            let loader = PluginLoader::new();
                            let parts: Vec<&str> = args.splitn(2, ' ').collect();
                            match parts[0] {
                                "list" | "" => {
                                    let plugins = loader.list();
                                    if plugins.is_empty() {
                                        println!("No plugins installed.");
                                        println!("Install with: /plugin install <path-or-url>\n");
                                    } else {
                                        println!("Installed plugins ({}):", plugins.len());
                                        for (name, version, desc) in &plugins {
                                            println!("  {} v{}  — {}", name, version, desc);
                                        }
                                        println!();
                                    }
                                }
                                "install" => {
                                    let src = if parts.len() > 1 { parts[1].trim() } else { "" };
                                    if src.is_empty() {
                                        println!("Usage: /plugin install <local-path-or-git-url>\n");
                                        continue;
                                    }
                                    print!("📦 Installing plugin from '{}' … ", src);
                                    io::stdout().flush()?;
                                    let result = if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("git@") {
                                        loader.install_from_git(src)
                                    } else {
                                        loader.install_from_path(std::path::Path::new(src))
                                    };
                                    match result {
                                        Ok(p) => println!("✅ Installed '{}' v{}\n", p.manifest.name, p.manifest.version),
                                        Err(e) => eprintln!("❌ Install failed: {}\n", e),
                                    }
                                }
                                "remove" | "uninstall" => {
                                    let name = if parts.len() > 1 { parts[1].trim() } else { "" };
                                    if name.is_empty() {
                                        println!("Usage: /plugin remove <name>\n");
                                        continue;
                                    }
                                    match loader.remove(name) {
                                        Ok(()) => println!("🗑️  Removed plugin '{}'\n", name),
                                        Err(e) => eprintln!("❌ Remove failed: {}\n", e),
                                    }
                                }
                                "info" => {
                                    let name = if parts.len() > 1 { parts[1].trim() } else { "" };
                                    if name.is_empty() {
                                        println!("Usage: /plugin info <name>\n");
                                        continue;
                                    }
                                    let plugin_dir = loader.plugins_dir.join(name);
                                    match loader.load_plugin(&plugin_dir) {
                                        Ok(p) => {
                                            println!("Plugin: {} v{}", p.manifest.name, p.manifest.version);
                                            if !p.manifest.description.is_empty() {
                                                println!("  {}", p.manifest.description);
                                            }
                                            if !p.manifest.author.is_empty() {
                                                println!("  Author: {}", p.manifest.author);
                                            }
                                            println!("  Directory: {}", p.dir.display());
                                            let skills = p.skills_dir();
                                            if skills.exists() {
                                                let count = std::fs::read_dir(&skills)
                                                    .map(|d| d.count())
                                                    .unwrap_or(0);
                                                println!("  Skills: {}", count);
                                            }
                                            if !p.manifest.hooks.is_empty() {
                                                println!("  Hooks: {} configured", p.manifest.hooks.len());
                                            }
                                            println!();
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                _ => println!("Usage: /plugin [list|install|remove|info]\n"),
                            }
                        }

                        // ── Phase 12 REPL commands ────────────────────────────────────────
                        "/model" => {
                            // /model                    — show current
                            // /model <provider>          — switch provider, keep model
                            // /model <provider> <model> — switch both
                            if args.is_empty() {
                                println!("Provider: {}  Model: {}\n",
                                    active_provider,
                                    active_model.as_deref().unwrap_or("(default)"));
                            } else {
                                let model_parts: Vec<&str> = args.splitn(2, ' ').collect();
                                let new_provider = model_parts[0];
                                let new_model = model_parts.get(1).map(|s| s.to_string());
                                match create_provider(new_provider, new_model.clone()) {
                                    Ok(new_llm) => {
                                        llm = new_llm;
                                        active_provider = new_provider.to_string();
                                        active_model = new_model;
                                        println!("✅ Switched to provider: {}  model: {}\n",
                                            active_provider,
                                            active_model.as_deref().unwrap_or("(default)"));
                                    }
                                    Err(e) => eprintln!("❌ Failed to switch provider: {}\n", e),
                                }
                            }
                        }
                        "/cost" => {
                            let total = session_tokens.total();
                            let cost = session_tokens.estimated_cost_usd(
                                &active_provider,
                                active_model.as_deref().unwrap_or(""),
                            );
                            println!("📊 Session token usage:");
                            println!("   Prompt tokens:     {}", session_tokens.prompt_tokens);
                            println!("   Completion tokens: {}", session_tokens.completion_tokens);
                            println!("   Total:             {}", total);
                            if cost > 0.0 {
                                println!("   Estimated cost:    ${:.6}", cost);
                            } else {
                                println!("   Estimated cost:    free (local model)");
                            }
                            println!();
                        }
                        "/context" => {
                            let char_count: usize = messages.iter().map(|m| m.content.len()).sum();
                            let approx_tokens = char_count / 4;
                            println!("📐 Context window:");
                            println!("   Messages:         {}", messages.len());
                            println!("   ~Characters:      {}", char_count);
                            println!("   ~Tokens (est.):   {}", approx_tokens);
                            println!();
                        }
                        "/status" => {
                            println!("ℹ️  Session status:");
                            println!("   Provider:  {}", active_provider);
                            println!("   Model:     {}", active_model.as_deref().unwrap_or("(default)"));
                            println!("   Messages:  {}", messages.len());
                            println!("   Tokens:    {} (prompt) + {} (completion) = {}",
                                session_tokens.prompt_tokens,
                                session_tokens.completion_tokens,
                                session_tokens.total());
                            let cost = session_tokens.estimated_cost_usd(
                                &active_provider,
                                active_model.as_deref().unwrap_or(""),
                            );
                            if cost > 0.0 {
                                println!("   Cost est.: ${:.6}", cost);
                            }
                            println!();
                        }
                        "/fork" => {
                            // Save a named snapshot of current messages for easy resume
                            let fork_name = if args.is_empty() {
                                let ts = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                format!("fork-{}", ts)
                            } else {
                                args.replace(' ', "-")
                            };
                            let trace_dir = dirs::home_dir()
                                .unwrap_or_else(|| cwd.clone())
                                .join(".vibecli")
                                .join("traces");
                            let writer = TraceWriter::new_named(trace_dir, &fork_name);
                            match writer.save_messages(&messages) {
                                Ok(()) => println!("🍴 Forked session as '{}'\n   Resume with: vibecli --resume {}\n",
                                    fork_name, writer.session_id()),
                                Err(e) => eprintln!("❌ Failed to save fork: {}\n", e),
                            }
                        }

                        "/rewind" => {
                            // Checkpoint system: /rewind        → save checkpoint
                            //                   /rewind list    → list checkpoints
                            //                   /rewind <n>     → restore to checkpoint N
                            match args {
                                "" => {
                                    // Save current messages as a checkpoint
                                    let rewind_dir = dirs::home_dir()
                                        .unwrap_or_else(|| cwd.clone())
                                        .join(".vibecli")
                                        .join("rewinds");
                                    let _ = std::fs::create_dir_all(&rewind_dir);
                                    let ts = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs();
                                    let checkpoint_path = rewind_dir.join(format!("{}.json", ts));
                                    let save_result = serde_json::to_string(&messages)
                                        .map_err(|e| e.to_string())
                                        .and_then(|s| std::fs::write(&checkpoint_path, s).map_err(|e| e.to_string()));
                                    match save_result {
                                        Ok(()) => println!("💾 Checkpoint saved ({} messages)\n   Restore with: /rewind {}\n", messages.len(), ts),
                                        Err(e) => eprintln!("❌ Failed to save checkpoint: {}\n", e),
                                    }
                                }
                                "list" => {
                                    let rewind_dir = dirs::home_dir()
                                        .unwrap_or_else(|| cwd.clone())
                                        .join(".vibecli")
                                        .join("rewinds");
                                    let mut entries: Vec<_> = std::fs::read_dir(&rewind_dir)
                                        .map(|d| d.filter_map(|e| e.ok()).collect())
                                        .unwrap_or_default();
                                    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
                                    if entries.is_empty() {
                                        println!("No checkpoints saved. Use /rewind to save one.\n");
                                    } else {
                                        println!("\n💾 Saved checkpoints:");
                                        for entry in entries.iter().take(10) {
                                            let ts_str = entry.file_name().to_string_lossy().replace(".json", "");
                                            let ts_secs: u64 = ts_str.parse().unwrap_or(0);
                                            let elapsed = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default()
                                                .as_secs()
                                                .saturating_sub(ts_secs);
                                            let age = if elapsed < 3600 { format!("{}m ago", elapsed / 60) }
                                                      else { format!("{}h ago", elapsed / 3600) };
                                            // Count messages in checkpoint
                                            let msg_count = std::fs::read_to_string(entry.path())
                                                .ok()
                                                .and_then(|s| serde_json::from_str::<Vec<Message>>(&s).ok())
                                                .map(|m| m.len())
                                                .unwrap_or(0);
                                            println!("  {} — {} messages — {}", ts_str, msg_count, age);
                                        }
                                        println!("\nRestore with: /rewind <timestamp>\n");
                                    }
                                }
                                ts_str => {
                                    let rewind_dir = dirs::home_dir()
                                        .unwrap_or_else(|| cwd.clone())
                                        .join(".vibecli")
                                        .join("rewinds");
                                    let checkpoint_path = rewind_dir.join(format!("{}.json", ts_str));
                                    match std::fs::read_to_string(&checkpoint_path)
                                        .map_err(|e| e.to_string())
                                        .and_then(|s| serde_json::from_str::<Vec<Message>>(&s).map_err(|e| e.to_string()))
                                    {
                                        Ok(restored) => {
                                            let count = restored.len();
                                            messages = restored;
                                            conversation_active = true;
                                            println!("⏪ Rewound to checkpoint {} ({} messages)\n", ts_str, count);
                                        }
                                        Err(e) => eprintln!("❌ Failed to load checkpoint {}: {}\n", ts_str, e),
                                    }
                                }
                            }
                        }

                        // ── Spec-driven development ───────────────────────────────────────
                        "/spec" => {
                            use crate::spec::SpecManager;
                            let cwd = std::env::current_dir()?;
                            let mgr = SpecManager::for_workspace(&cwd);
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["list"]
                            } else {
                                args.splitn(3, ' ').collect()
                            };
                            match parts[0] {
                                "list" | "" => {
                                    let names = mgr.list();
                                    if names.is_empty() {
                                        println!("No specs. Create one with: /spec new <name>\n");
                                    } else {
                                        println!("Specs ({}):", names.len());
                                        for name in &names {
                                            if let Ok(s) = mgr.load(name) {
                                                let done = s.completed();
                                                let total = s.tasks.len();
                                                println!("  [{}/{}] {} — {}", done, total, name, s.status);
                                            } else {
                                                println!("  {}", name);
                                            }
                                        }
                                        println!();
                                    }
                                }
                                "show" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    if name.is_empty() {
                                        println!("Usage: /spec show <name>\n");
                                        continue;
                                    }
                                    match mgr.load(name) {
                                        Ok(spec) => {
                                            println!("\n📋 Spec: {}  [{}]", spec.name, spec.status);
                                            if !spec.requirements.is_empty() {
                                                println!("   Requirements: {}", spec.requirements);
                                            }
                                            println!("\n   Tasks ({}/{} done):", spec.completed(), spec.tasks.len());
                                            for task in &spec.tasks {
                                                let icon = if task.done { "✅" } else { "◻️ " };
                                                println!("   {} [{}] {}", icon, task.id, task.description);
                                            }
                                            println!();
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "new" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    if name.is_empty() {
                                        println!("Usage: /spec new <name> [requirements...]\n");
                                        continue;
                                    }
                                    let requirements = parts.get(2).unwrap_or(&"").trim();
                                    match mgr.init().and_then(|_| mgr.create_empty(name, requirements)) {
                                        Ok(_) => {
                                            println!("✅ Spec '{}' created at {}", name, cwd.join(".vibecli/specs").join(format!("{}.md", name)).display());
                                            println!("   Edit it to add tasks, or use the VibeUI Specs panel to generate one with AI.\n");
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "done" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    let task_id_str = parts.get(2).unwrap_or(&"").trim();
                                    if name.is_empty() || task_id_str.is_empty() {
                                        println!("Usage: /spec done <name> <task-id>\n");
                                        continue;
                                    }
                                    match task_id_str.parse::<u32>() {
                                        Ok(task_id) => match mgr.complete_task(name, task_id) {
                                            Ok(()) => println!("✅ Task {} in '{}' marked done\n", task_id, name),
                                            Err(e) => eprintln!("❌ {}\n", e),
                                        },
                                        Err(_) => eprintln!("❌ Invalid task ID '{}'\n", task_id_str),
                                    }
                                }
                                "run" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    if name.is_empty() {
                                        println!("Usage: /spec run <name>\n");
                                        continue;
                                    }
                                    match mgr.load(name) {
                                        Ok(spec) => {
                                            let ctx = spec.context_string();
                                            println!("🤖 Running agent on spec '{}' ({} pending tasks)…\n", name, spec.pending());
                                            // Inject spec context into the system prompt for the next agent call
                                            println!("{}", ctx);
                                            println!("Use /agent to start the agent with the above spec as context.\n");
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                _ => println!("Usage: /spec [list|show|new|run|done]\n"),
                            }
                        }

                        // ── /agents ────────────────────────────────────────────────────
                        "/agents" => {
                            use crate::background_agents::BackgroundAgentManager;
                            let cwd = std::env::current_dir()?;
                            let mgr = BackgroundAgentManager::for_workspace(&cwd);
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["list"]
                            } else {
                                args.splitn(3, ' ').collect()
                            };
                            match parts[0] {
                                "list" | "" => {
                                    let names = mgr.list_defs();
                                    if names.is_empty() {
                                        println!("No agents defined. Create one in .vibecli/agents/<name>.toml\n");
                                    } else {
                                        println!("Background agents ({}):", names.len());
                                        for name in &names {
                                            if let Ok(def) = mgr.load_def(name) {
                                                println!("  {} — {} [trigger: {}]", def.name, def.task, def.trigger);
                                            } else {
                                                println!("  {}", name);
                                            }
                                        }
                                        println!();
                                    }
                                }
                                "status" => {
                                    let runs = mgr.list_runs();
                                    if runs.is_empty() {
                                        println!("No background agents have run this session.\n");
                                    } else {
                                        println!("Agent runs ({}):", runs.len());
                                        for run in &runs {
                                            let summary = run.summary.as_deref().unwrap_or("—");
                                            println!("  [{}] {} — {} → {}", run.id, run.name, run.status, summary);
                                        }
                                        println!();
                                    }
                                }
                                "new" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    let task = parts.get(2).unwrap_or(&"").trim();
                                    if name.is_empty() {
                                        println!("Usage: /agents new <name> <task description>\n");
                                        continue;
                                    }
                                    let _ = mgr.init();
                                    let task = if task.is_empty() { "Your task here" } else { task };
                                    match mgr.create_template(name, task) {
                                        Ok(_) => println!("✅ Agent '{}' created at .vibecli/agents/{}.toml\n", name, name),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                _ => println!("Usage: /agents [list|status|new <name> <task>]\n"),
                            }
                        }

                        // ── /team ──────────────────────────────────────────────────────
                        "/team" => {
                            use crate::team::TeamManager;
                            let cwd = std::env::current_dir()?;
                            let mgr = TeamManager::for_workspace(&cwd);
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["show"]
                            } else {
                                args.splitn(4, ' ').collect()
                            };
                            match parts[0] {
                                "show" | "" => {
                                    let cfg = mgr.load();
                                    let team_name = cfg.team.name.as_deref().unwrap_or("(unnamed)");
                                    println!("Team: {}", team_name);
                                    if cfg.knowledge.is_empty() {
                                        println!("  No knowledge entries.");
                                    } else {
                                        println!("  Knowledge ({}):", cfg.knowledge.len());
                                        for k in &cfg.knowledge {
                                            println!("    - {}: {}", k.name, k.content);
                                        }
                                    }
                                    if !cfg.shared_commands.is_empty() {
                                        println!("  Shared commands:");
                                        for cmd in &cfg.shared_commands {
                                            println!("    - {} → `{}`", cmd.name, cmd.command);
                                        }
                                    }
                                    println!();
                                }
                                "knowledge" => {
                                    let sub = parts.get(1).unwrap_or(&"").trim();
                                    match sub {
                                        "list" | "" => {
                                            let cfg = mgr.load();
                                            if cfg.knowledge.is_empty() {
                                                println!("No team knowledge entries.\n");
                                            } else {
                                                for k in &cfg.knowledge {
                                                    let tags = if k.tags.is_empty() { String::new() } else { format!(" [{}]", k.tags.join(", ")) };
                                                    println!("  {}{}: {}", k.name, tags, k.content);
                                                }
                                                println!();
                                            }
                                        }
                                        "add" => {
                                            let name = parts.get(2).unwrap_or(&"").trim();
                                            let content = parts.get(3).unwrap_or(&"").trim();
                                            if name.is_empty() || content.is_empty() {
                                                println!("Usage: /team knowledge add <name> <content>\n");
                                                continue;
                                            }
                                            match mgr.add_knowledge(name, content, vec![]) {
                                                Ok(()) => println!("✅ Added team knowledge '{}'.\n", name),
                                                Err(e) => eprintln!("❌ {}\n", e),
                                            }
                                        }
                                        "remove" => {
                                            let name = parts.get(2).unwrap_or(&"").trim();
                                            if name.is_empty() {
                                                println!("Usage: /team knowledge remove <name>\n");
                                                continue;
                                            }
                                            match mgr.remove_knowledge(name) {
                                                Ok(true) => println!("✅ Removed '{}'.\n", name),
                                                Ok(false) => println!("⚠️  '{}' not found.\n", name),
                                                Err(e) => eprintln!("❌ {}\n", e),
                                            }
                                        }
                                        _ => println!("Usage: /team knowledge [list|add|remove]\n"),
                                    }
                                }
                                "sync" => {
                                    match mgr.sync().await {
                                        Ok(msg) => println!("✅ {}\n", msg),
                                        Err(e) => eprintln!("❌ Sync failed: {}\n", e),
                                    }
                                }
                                _ => println!("Usage: /team [show|knowledge [list|add|remove]|sync]\n"),
                            }
                        }

                        _ => {
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
                        Err(e) => eprintln!("❌ Error: {:#}\n", e),
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
    json_output: bool,
    planning_llm: Option<Arc<dyn LLMProvider>>,
) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let config = Config::load().unwrap_or_default();
    let approval = ApprovalPolicy::from_str(approval_policy);
    let sandbox = config.safety.sandbox;

    // Apply shell env policy
    let env_policy = config.safety.shell_environment.to_policy();
    let executor: Arc<dyn vibe_ai::agent::ToolExecutorTrait> =
        Arc::new(ToolExecutor::new(workspace.clone(), sandbox).with_env_policy(env_policy));

    // Build hooks from config; wire LLM provider so `handler = { llm = "..." }` hooks work.
    let hook_runner = if config.hooks.is_empty() {
        HookRunner::empty()
    } else {
        HookRunner::new(config.hooks.clone()).with_llm_provider(llm.clone())
    };
    let agent = AgentLoop::new(llm.clone(), approval.clone(), executor.clone())
        .with_hooks(hook_runner);

    let trace_dir = dirs::home_dir()
        .unwrap_or_else(|| workspace.clone())
        .join(".vibecli")
        .join("traces");

    // Plan mode: generate plan before executing.
    // Uses planning_llm (opusplan routing) when provided, otherwise falls back to llm.
    let approved_plan: Option<String> = if plan_mode {
        println!("🧠 Generating execution plan...\n");
        let plan_provider = planning_llm.unwrap_or_else(|| llm.clone());
        let planner = PlannerAgent::new(plan_provider);
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

    // Collect skill directories from installed plugins.
    let plugin_skill_dirs = PluginLoader::new().all_skill_paths()
        .into_iter()
        .filter_map(|p| p.parent().map(|d| d.to_path_buf()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let context = AgentContext {
        workspace_root: workspace.clone(),
        approved_plan,
        extra_skill_dirs: plugin_skill_dirs,
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
    let mut step_count: usize = 0;

    while let Some(event) = event_rx.recv().await {
        // In --json mode, emit a JSON line for each event and skip pretty printing.
        if json_output {
            let obj = match &event {
                AgentEvent::StreamChunk(t) => serde_json::json!({"type":"chunk","text":t}),
                AgentEvent::ToolCallExecuted(s) => serde_json::json!({
                    "type":"tool_executed",
                    "tool": s.tool_call.name(),
                    "success": s.tool_result.success,
                    "step": s.step_num,
                }),
                AgentEvent::Complete(s) => serde_json::json!({"type":"complete","summary":s}),
                AgentEvent::Error(e) => serde_json::json!({"type":"error","message":e}),
                AgentEvent::ToolCallPending { call, .. } => serde_json::json!({"type":"tool_pending","tool":call.name()}),
            };
            println!("{}", obj);
            io::stdout().flush()?;
            match event {
                AgentEvent::ToolCallPending { result_tx, call } => {
                    // In JSON mode auto-execute all tool calls (full-auto behaviour)
                    let result = executor.execute(&call).await;
                    let _ = result_tx.send(Some(result));
                }
                AgentEvent::Complete(_) | AgentEvent::Error(_) => break,
                _ => {}
            }
            continue;
        }

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
                step_count += 1;
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
                // Auto memory recording
                if config.memory.auto_record && step_count >= config.memory.min_session_steps {
                    let llm2 = llm.clone();
                    let task2 = task.to_string();
                    let summary2 = summary.clone();
                    let steps2 = step_count;
                    tokio::spawn(async move {
                        if let Err(e) = memory_recorder::record_session(llm2, &task2, steps2, &summary2).await {
                            tracing::warn!("Auto memory recording failed: {}", e);
                        }
                    });
                }
                break;
            }
            AgentEvent::Error(e) => {
                eprintln!("\n❌ Agent error: {}", e);
                break;
            }
        }
    }

    // ── Auto-commit offer ────────────────────────────────────────────────────
    // After agent completes, check for uncommitted changes and offer to commit.
    maybe_offer_commit(&workspace, task, llm.as_ref()).await;

    Ok(())
}

/// After an agent task finishes, check for git changes and offer to commit.
async fn maybe_offer_commit(workspace: &std::path::Path, task: &str, llm: &dyn LLMProvider) {
    // Check for changes with `git status --porcelain`
    let status_out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workspace)
        .output();

    let changed = match status_out {
        Ok(o) if !o.stdout.is_empty() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return, // Not a git repo, or no changes
    };

    println!("\n📝 Git changes detected:\n{}", changed.trim_end());
    print!("Commit these changes? (y/N): ");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        return;
    }
    if answer.trim().to_lowercase() != "y" {
        return;
    }

    // Get a short git diff for the LLM commit message
    let diff_out = std::process::Command::new("git")
        .args(["diff", "--stat", "HEAD"])
        .current_dir(workspace)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    println!("🤖 Generating commit message…");
    let prompt = format!(
        "Write a short git commit message (max 72 chars, imperative mood) \
         for these changes made by an AI agent.\n\
         Agent task: {task}\n\
         Changed files:\n{changed}\
         Diff summary:\n{diff_out}\n\
         Output ONLY the commit message, no explanation.",
    );
    let commit_msg = match llm
        .chat(
            &[Message { role: MessageRole::User, content: prompt }],
            None,
        )
        .await
    {
        Ok(msg) => msg.trim().to_string(),
        Err(e) => {
            eprintln!("⚠️  Could not generate commit message: {}", e);
            format!("agent: {}", &task.chars().take(60).collect::<String>())
        }
    };

    println!("   Commit message: {}", commit_msg);
    print!("   Commit? (y/N/e=edit): ");
    let _ = io::stdout().flush();
    let mut confirm = String::new();
    if io::stdin().read_line(&mut confirm).is_err() {
        return;
    }
    let confirm = confirm.trim().to_lowercase();
    if confirm == "n" || confirm.is_empty() {
        return;
    }

    let final_msg = if confirm == "e" {
        print!("   Commit message: ");
        let _ = io::stdout().flush();
        let mut edited = String::new();
        let _ = io::stdin().read_line(&mut edited);
        edited.trim().to_string()
    } else {
        commit_msg
    };

    // Stage all changes and commit
    let add = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(workspace)
        .status();
    let commit = std::process::Command::new("git")
        .args(["commit", "-m", &final_msg])
        .current_dir(workspace)
        .output();

    match (add, commit) {
        (Ok(a), Ok(c)) if a.success() && c.status.success() => {
            let hash = String::from_utf8_lossy(&c.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            println!("✅ Committed: {}\n", hash.trim());
        }
        (_, Err(e)) => eprintln!("❌ Commit failed: {}\n", e),
        (Err(e), _) => eprintln!("❌ git add failed: {}\n", e),
        _ => eprintln!("❌ Commit failed (check git output above)\n"),
    }
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
    use vibe_ai::providers::{claude, openai, gemini, grok, groq, openrouter, azure_openai};

    // Helper: look up API key from config, then env var.
    let cfg = Config::load().unwrap_or_default();

    match provider_name.to_lowercase().as_str() {
        // ── Ollama (local, no API key required) ───────────────────────────────
        "ollama" => {
            let cfg_model = cfg.ollama.as_ref().and_then(|c| c.model.clone());
            let api_url = {
                let raw = cfg.ollama.as_ref().and_then(|c| c.api_url.clone())
                    .or_else(|| std::env::var("OLLAMA_HOST").ok())
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                // Normalize: OLLAMA_HOST is often set without a scheme (e.g. "127.0.0.1:11434")
                if raw.starts_with("http://") || raw.starts_with("https://") {
                    raw
                } else {
                    format!("http://{}", raw)
                }
            };
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "qwen3-coder:480b-cloud".to_string());
            Ok(Arc::new(OllamaProvider::new(ProviderConfig {
                provider_type: "ollama".to_string(),
                api_url: Some(api_url),
                model,
                api_key: None,
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }

        // ── Anthropic Claude ──────────────────────────────────────────────────
        "claude" | "anthropic" => {
            let cfg_key = cfg.claude.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  ANTHROPIC_API_KEY not set (set env var or [claude] api_key in config)");
            }
            let cfg_model = cfg.claude.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "claude-sonnet-4-6".to_string());
            let api_key_helper = cfg.claude.as_ref().and_then(|c| c.api_key_helper.clone());
            let thinking = cfg.claude.as_ref().and_then(|c| c.thinking_budget_tokens);
            Ok(Arc::new(claude::ClaudeProvider::new(ProviderConfig {
                provider_type: "claude".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                thinking_budget_tokens: thinking,
            })))
        }

        // ── OpenAI ────────────────────────────────────────────────────────────
        "openai" | "gpt" => {
            let cfg_key = cfg.openai.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("OPENAI_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  OPENAI_API_KEY not set (set env var or [openai] api_key in config)");
            }
            let cfg_model = cfg.openai.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "gpt-4o".to_string());
            let api_key_helper = cfg.openai.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(openai::OpenAIProvider::new(ProviderConfig {
                provider_type: "openai".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Google Gemini ─────────────────────────────────────────────────────
        "gemini" | "google" => {
            let cfg_key = cfg.gemini.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("GEMINI_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  GEMINI_API_KEY not set (set env var or [gemini] api_key in config)");
            }
            let cfg_model = cfg.gemini.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "gemini-2.0-flash".to_string());
            let api_key_helper = cfg.gemini.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(gemini::GeminiProvider::new(ProviderConfig {
                provider_type: "gemini".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── xAI Grok ──────────────────────────────────────────────────────────
        "grok" | "xai" => {
            let cfg_key = cfg.grok.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("GROK_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  GROK_API_KEY not set (set env var or [grok] api_key in config)");
            }
            let cfg_model = cfg.grok.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "grok-3-mini".to_string());
            let api_key_helper = cfg.grok.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(grok::GrokProvider::new(ProviderConfig {
                provider_type: "grok".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Groq ──────────────────────────────────────────────────────────────
        "groq" => {
            let cfg_key = cfg.groq.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("GROQ_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  GROQ_API_KEY not set (set env var or [groq] api_key in config)");
            }
            let cfg_model = cfg.groq.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "llama-3.3-70b-versatile".to_string());
            let api_key_helper = cfg.groq.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(groq::GroqProvider::new(ProviderConfig {
                provider_type: "groq".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── OpenRouter ────────────────────────────────────────────────────────
        "openrouter" => {
            let cfg_key = cfg.openrouter.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("OPENROUTER_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  OPENROUTER_API_KEY not set (set env var or [openrouter] api_key in config)");
            }
            let cfg_model = cfg.openrouter.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "anthropic/claude-3.5-sonnet".to_string());
            let api_key_helper = cfg.openrouter.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(openrouter::OpenRouterProvider::new(ProviderConfig {
                provider_type: "openrouter".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Azure OpenAI ──────────────────────────────────────────────────────
        "azure" | "azure_openai" => {
            let cfg = cfg.azure_openai.as_ref();
            let api_key = cfg.and_then(|c| c.api_key.clone())
                .or_else(|| std::env::var("AZURE_OPENAI_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  AZURE_OPENAI_API_KEY not set");
            }
            let api_url = cfg.and_then(|c| c.api_url.clone())
                .or_else(|| std::env::var("AZURE_OPENAI_ENDPOINT").ok());
            if api_url.is_none() {
                eprintln!("⚠️  azure_openai.api_url not set (e.g. https://myresource.openai.azure.com)");
            }
            let cfg_model = cfg.and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "gpt-4o".to_string());
            Ok(Arc::new(azure_openai::AzureOpenAIProvider::new(ProviderConfig {
                provider_type: "azure_openai".to_string(),
                api_url,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }

        _ => anyhow::bail!(
            "Unknown provider: '{}'. Available: ollama, claude, openai, gemini, grok, groq, openrouter, azure",
            provider_name
        ),
    }
}

fn show_help() {
    println!("\n📚 VibeCLI Commands:");
    println!("  /chat <message>          - Chat with AI (supports [image.png] for vision)");
    println!("  /agent <task>            - Run autonomous coding agent on a task");
    println!("  /plan <task>             - Generate execution plan, then run agent");
    println!("  /resume [id] [task]      - List resumable sessions or resume one");
    println!("  /rewind                  - Save a conversation checkpoint");
    println!("  /rewind list             - List saved checkpoints");
    println!("  /rewind <timestamp>      - Restore conversation to a checkpoint");
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
    println!("  /index [model]           - Build semantic codebase index (default model: nomic-embed-text)");
    println!("  /qa <question>           - Ask a question about the codebase using semantic search");
    println!("  /plugin list             - List installed plugins");
    println!("  /plugin install <path>   - Install a plugin from a local path or git URL");
    println!("  /plugin remove <name>    - Remove an installed plugin");
    println!("  /plugin info <name>      - Show plugin details");
    println!("  /profile list            - List named profiles (~/.vibecli/profiles/)");
    println!("  /profile show <name>     - Show a profile's settings");
    println!("  /profile create <name>   - Create a new profile interactively");
    println!("  /profile delete <name>   - Delete a profile");
    println!("  /spec                    - Spec-driven development (list|show|new|run|done)");
    println!("  /agents                  - Background agents (list|status|new)");
    println!("  /team                    - Team knowledge store (show|knowledge|sync)");
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
    println!("  --serve                  - Start HTTP daemon (VS Code extension / Agent SDK)");
    println!("  --mcp-server             - Run as MCP server (for Claude Desktop etc.)");
    println!("  --profile <name>         - Load a named config profile (~/.vibecli/profiles/<name>.toml)");
    println!("  --doctor                 - Run health checks on the VibeCLI installation");
    println!("\nProviders (--provider <name>):");
    println!("  ollama                   - Local Ollama (default, no key needed)");
    println!("  claude                   - Anthropic Claude  (ANTHROPIC_API_KEY)");
    println!("  openai                   - OpenAI GPT-4o     (OPENAI_API_KEY)");
    println!("  gemini                   - Google Gemini     (GEMINI_API_KEY)");
    println!("  grok                     - xAI Grok          (GROK_API_KEY)");
    println!("  groq                     - Groq ultra-fast   (GROQ_API_KEY)");
    println!("  openrouter               - OpenRouter 300+   (OPENROUTER_API_KEY)");
    println!("  azure                    - Azure OpenAI      (AZURE_OPENAI_API_KEY + api_url)");
    println!("\nMultimodal:");
    println!("  /chat [screenshot.png] What is this error?  - Attach image to chat");
    println!("\n💡 Tip: You can also just type a message to chat\n");
}

/// Run a health check of the VibeCLI installation: config, providers, git, plugins, profiles.
async fn run_doctor() -> Result<()> {
    println!("\n🩺 VibeCLI Doctor — health check\n");

    // 1. Config file
    let config_path = dirs::home_dir()
        .map(|h| h.join(".vibecli").join("config.toml"))
        .unwrap_or_else(|| std::path::PathBuf::from("~/.vibecli/config.toml"));
    let config = match Config::load() {
        Ok(c) => {
            println!("  ✅ Config     — {} (valid)", config_path.display());
            c
        }
        Err(e) => {
            println!("  ⚠️  Config     — {} (not found: {})", config_path.display(), e);
            Config::default()
        }
    };

    // 2. Ollama reachability (TCP connect — no reqwest needed)
    let api_url = config.ollama.as_ref()
        .and_then(|o| o.api_url.clone())
        .or_else(|| std::env::var("OLLAMA_HOST").ok())
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    // Extract host:port from URL
    let host_port = api_url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or("localhost:11434")
        .to_string();
    // Ensure port is present
    let host_port = if host_port.contains(':') {
        host_port
    } else {
        format!("{}:11434", host_port)
    };
    print!("  ·  Ollama     — checking {}… ", host_port);
    io::stdout().flush()?;
    match host_port.parse::<std::net::SocketAddr>() {
        Ok(addr) => {
            match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(2)) {
                Ok(_) => println!("✅ reachable"),
                Err(_) => println!("❌ not reachable (start with: ollama serve)"),
            }
        }
        Err(_) => println!("⚠️  could not parse address '{}'", host_port),
    }

    // 3. Cloud provider API keys
    for (name, env_var, cfg_key) in [
        ("Claude", "ANTHROPIC_API_KEY", config.claude.as_ref().and_then(|c| c.api_key.clone())),
        ("OpenAI", "OPENAI_API_KEY",    config.openai.as_ref().and_then(|c| c.api_key.clone())),
        ("Gemini", "GEMINI_API_KEY",    config.gemini.as_ref().and_then(|c| c.api_key.clone())),
        ("Grok",   "GROK_API_KEY",      config.grok.as_ref().and_then(|c| c.api_key.clone())),
    ] {
        if std::env::var(env_var).is_ok() {
            println!("  ✅ {:<8} — {} set in environment", name, env_var);
        } else if cfg_key.is_some() {
            println!("  ✅ {:<8} — API key in config.toml", name);
        } else {
            println!("  ○  {:<8} — no key (set {} if needed)", name, env_var);
        }
    }

    // 4. Git binary
    print!("  ·  Git        — checking binary… ");
    io::stdout().flush()?;
    match std::process::Command::new("git").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("✅ {}", ver);
        }
        _ => println!("❌ not found — install git for repository features"),
    }

    // 5. Plugins
    let plugins = PluginLoader::new().list();
    if plugins.is_empty() {
        println!("  ○  Plugins    — none installed (~/.vibecli/plugins/)");
    } else {
        println!("  ✅ Plugins    — {} installed", plugins.len());
        for (name, ver, _desc) in &plugins {
            println!("     • {} v{}", name, ver);
        }
    }

    // 6. Named profiles
    let profiles = ProfileManager::new().list();
    if profiles.is_empty() {
        println!("  ○  Profiles   — none (~/.vibecli/profiles/)");
    } else {
        println!("  ✅ Profiles   — {} available", profiles.len());
        for (name, desc) in &profiles {
            let suffix = if desc.is_empty() {
                String::new()
            } else {
                format!(" — {}", desc)
            };
            println!("     • {}{}", name, suffix);
        }
    }

    // 7. Skills directory
    match dirs::home_dir().map(|h| h.join(".vibecli").join("skills")) {
        Some(dir) if dir.exists() => {
            let count = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
            println!("  ✅ Skills     — {} file(s) in {}", count, dir.display());
        }
        _ => println!("  ○  Skills     — no ~/.vibecli/skills/ directory"),
    }

    // 8. Active profile note
    if let Some(active) = ProfileManager::read_active() {
        println!("  📋 Active profile: {}", active);
    }

    // 9. Sandbox availability
    if config.safety.sandbox {
        #[cfg(target_os = "macos")]
        {
            match std::process::Command::new("sandbox-exec").arg("-n").output() {
                Ok(_) => println!("  ✅ Sandbox    — sandbox-exec available (macOS Seatbelt)"),
                Err(_) => println!("  ❌ Sandbox    — sandbox-exec not found (sandbox mode enabled but tool missing)"),
            }
        }
        #[cfg(target_os = "linux")]
        {
            match std::process::Command::new("bwrap").arg("--version").output() {
                Ok(_) => println!("  ✅ Sandbox    — bwrap available (Linux bubblewrap)"),
                Err(_) => println!("  ❌ Sandbox    — bwrap not found (install: sudo apt install bubblewrap)"),
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        println!("  ⚠️  Sandbox    — enabled but not supported on this OS");
    } else {
        println!("  ○  Sandbox    — disabled (set [safety] sandbox = true to enable)");
    }

    // 10. opusplan model routing
    if config.routing.is_configured() {
        println!("  🔀 Routing    — opusplan routing configured");
        let (pp, pm) = config.routing.resolve_planning("(default)", "(default)");
        let (ep, em) = config.routing.resolve_execution("(default)", "(default)");
        println!("     Planning  → {}:{}", pp, pm);
        println!("     Execution → {}:{}", ep, em);
    } else {
        println!("  ○  Routing    — opusplan routing not configured (uses --provider/--model)");
    }

    println!();
    println!("Config file: ~/.vibecli/config.toml");
    println!("For more information run `vibecli --help`\n");

    Ok(())
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
