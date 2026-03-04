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
mod workflow;
mod background_agents;
mod team;
mod memory_auto;
mod bugbot;
mod redteam;
mod scheduler;
mod gateway;
mod linear;
mod session_store;
use session_store::SessionStore;
mod notebook;
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

    // ── Gateway mode (Phase 21) ────────────────────────────────────────────────

    /// Start as a messaging gateway bot.
    /// Supported: telegram, discord, slack, signal, matrix (element),
    ///            twilio (sms), whatsapp, imessage (macOS), teams.
    /// Requires the corresponding token/config. Example: vibecli --gateway signal
    #[arg(long, value_name = "PLATFORM")]
    gateway: Option<String>,

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

    // ── Notebook runner ───────────────────────────────────────────────────────

    /// Run a .vibe notebook file (executable Markdown with code cells).
    /// Supported languages: bash, sh, python, python3, ruby, node, js, deno, rust
    /// Example: vibecli --notebook script.vibe
    #[arg(long, value_name = "FILE")]
    notebook: Option<String>,

    /// Continue running notebook cells even if one fails (default: stop on first error).
    #[arg(long)]
    continue_on_error: bool,

    // ── Copilot OAuth ─────────────────────────────────────────────────────────

    /// Authenticate with GitHub Copilot via device flow.
    /// Prints a GITHUB_TOKEN to save to your shell profile.
    #[arg(long)]
    copilot_login: bool,

    // ── Worktree isolation ─────────────────────────────────────────────────────

    /// Use git worktree isolation for this agent session.
    /// The agent runs in a fresh worktree branch; changes are not applied to
    /// the current branch unless explicitly merged. Mirrors Claude Code's -w flag.
    #[arg(long)]
    worktree: bool,

    // ── Watch mode ─────────────────────────────────────────────────────────────

    /// Watch the current directory for file changes and run the agent task
    /// automatically whenever a file matching --watch-glob changes.
    /// Example: vibecli --watch --agent "Run tests after changes" --watch-glob "**/*.rs"
    #[arg(long)]
    watch: bool,

    /// Glob pattern for --watch mode (default: "**/*.{rs,ts,tsx,py,go,js,jsx}").
    #[arg(long, value_name = "GLOB", default_value = "**/*.{rs,ts,tsx,py,go,js,jsx}")]
    watch_glob: String,

    // ── Sandbox ────────────────────────────────────────────────────────────────

    /// Enable sandbox isolation for shell commands executed by the agent.
    /// On macOS, wraps bash calls in `sandbox-exec` (Seatbelt).
    /// On Linux, wraps in `bwrap` if available.
    /// Overrides config.safety.sandbox = true/false.
    #[arg(long)]
    sandbox: bool,

    // ── Red Team mode (Phase 41) ──────────────────────────────────────────────

    /// Run autonomous red team security scan against a target URL.
    /// Requires explicit user consent. Only test applications you own/control.
    /// Example: vibecli --redteam http://localhost:3000
    #[arg(long, value_name = "URL")]
    redteam: Option<String>,

    /// YAML configuration file for red team scan (auth flows, scope, depth).
    /// Example: vibecli --redteam http://localhost:3000 --redteam-config auth.yaml
    #[arg(long, value_name = "FILE")]
    redteam_config: Option<String>,

    /// Generate a report from a previous red team session ID.
    /// Example: vibecli --redteam-report rt-20260226T143025
    #[arg(long, value_name = "SESSION_ID")]
    redteam_report: Option<String>,
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

    // Resolve sandbox flag: CLI --sandbox overrides config
    let sandbox_enabled = {
        let from_config = Config::load().map(|c| c.safety.sandbox).unwrap_or(false);
        cli.sandbox || from_config
    };

    // ── Doctor mode ───────────────────────────────────────────────────────────
    if cli.doctor {
        return run_doctor().await;
    }

    // ── Copilot device-flow login ─────────────────────────────────────────────
    if cli.copilot_login {
        match vibe_ai::providers::copilot::run_device_flow().await {
            Ok(token) => {
                println!("✅ GitHub token obtained!");
                println!("Add to your shell profile:");
                println!("  export GITHUB_TOKEN={}", token);
            }
            Err(e) => eprintln!("❌ Copilot login failed: {}", e),
        }
        return Ok(());
    }

    // ── Notebook runner ───────────────────────────────────────────────────────
    if let Some(notebook_path) = cli.notebook.as_deref() {
        let path = std::path::Path::new(notebook_path);
        let ok = notebook::run_notebook(path, cli.continue_on_error)?;
        std::process::exit(if ok { 0 } else { 1 });
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

    // Gateway mode: vibecli --gateway telegram|discord|slack|signal|matrix|twilio|whatsapp|imessage|teams
    if let Some(ref platform) = cli.gateway {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let config = Config::load().unwrap_or_default();
        let gw_cfg = &config.gateway;
        let platform_box: Box<dyn gateway::GatewayPlatform> = match platform.as_str() {
            "telegram" => {
                let token = gw_cfg.resolve_telegram_token()
                    .ok_or_else(|| anyhow::anyhow!("Telegram token not configured. Set TELEGRAM_BOT_TOKEN or gateway.telegram_token in config."))?;
                Box::new(gateway::TelegramGateway::new(token, gw_cfg.allowed_users.clone()))
            }
            "discord" => {
                let token = gw_cfg.resolve_discord_token()
                    .ok_or_else(|| anyhow::anyhow!("Discord token not configured. Set DISCORD_BOT_TOKEN or gateway.discord_token in config."))?;
                let channel = gw_cfg.discord_channel_id.clone().unwrap_or_default();
                Box::new(gateway::DiscordGateway::new(token, channel))
            }
            "slack" => {
                let token = gw_cfg.resolve_slack_bot_token()
                    .ok_or_else(|| anyhow::anyhow!("Slack token not configured. Set SLACK_BOT_TOKEN or gateway.slack_bot_token in config."))?;
                let channel = gw_cfg.slack_channel_id.clone().unwrap_or_default();
                Box::new(gateway::SlackGateway::new(token, channel))
            }
            "signal" => {
                let api_url = gw_cfg.resolve_signal_api_url()
                    .ok_or_else(|| anyhow::anyhow!("Signal API URL not configured. Set SIGNAL_API_URL or gateway.signal_api_url in config."))?;
                let phone = gw_cfg.resolve_signal_phone_number()
                    .ok_or_else(|| anyhow::anyhow!("Signal phone number not configured. Set SIGNAL_PHONE_NUMBER or gateway.signal_phone_number in config."))?;
                Box::new(gateway::SignalGateway::new(api_url, phone))
            }
            "matrix" | "element" => {
                let hs = gw_cfg.resolve_matrix_homeserver_url()
                    .ok_or_else(|| anyhow::anyhow!("Matrix homeserver URL not configured. Set MATRIX_HOMESERVER_URL or gateway.matrix_homeserver_url in config."))?;
                let token = gw_cfg.resolve_matrix_access_token()
                    .ok_or_else(|| anyhow::anyhow!("Matrix access token not configured. Set MATRIX_ACCESS_TOKEN or gateway.matrix_access_token in config."))?;
                let room = gw_cfg.resolve_matrix_room_id()
                    .ok_or_else(|| anyhow::anyhow!("Matrix room ID not configured. Set MATRIX_ROOM_ID or gateway.matrix_room_id in config."))?;
                let user = gw_cfg.resolve_matrix_user_id().unwrap_or_default();
                Box::new(gateway::MatrixGateway::new(hs, token, room, user))
            }
            "twilio" | "sms" => {
                let sid = gw_cfg.resolve_twilio_account_sid()
                    .ok_or_else(|| anyhow::anyhow!("Twilio Account SID not configured. Set TWILIO_ACCOUNT_SID or gateway.twilio_account_sid in config."))?;
                let auth = gw_cfg.resolve_twilio_auth_token()
                    .ok_or_else(|| anyhow::anyhow!("Twilio Auth Token not configured. Set TWILIO_AUTH_TOKEN or gateway.twilio_auth_token in config."))?;
                let from = gw_cfg.resolve_twilio_from_number()
                    .ok_or_else(|| anyhow::anyhow!("Twilio From number not configured. Set TWILIO_FROM_NUMBER or gateway.twilio_from_number in config."))?;
                Box::new(gateway::TwilioGateway::new(sid, auth, from))
            }
            "whatsapp" => {
                let token = gw_cfg.resolve_whatsapp_access_token()
                    .ok_or_else(|| anyhow::anyhow!("WhatsApp access token not configured. Set WHATSAPP_ACCESS_TOKEN or gateway.whatsapp_access_token in config."))?;
                let phone_id = gw_cfg.resolve_whatsapp_phone_number_id()
                    .ok_or_else(|| anyhow::anyhow!("WhatsApp Phone Number ID not configured. Set WHATSAPP_PHONE_NUMBER_ID or gateway.whatsapp_phone_number_id in config."))?;
                let verify = gw_cfg.resolve_whatsapp_verify_token().unwrap_or_else(|| "vibecli".to_string());
                let port = gw_cfg.whatsapp_webhook_port.unwrap_or(8443);
                Box::new(gateway::WhatsAppGateway::new(token, phone_id, verify, port).await)
            }
            #[cfg(target_os = "macos")]
            "imessage" => {
                let db_path = gw_cfg.resolve_imessage_db_path();
                Box::new(gateway::IMessageGateway::new(db_path))
            }
            #[cfg(not(target_os = "macos"))]
            "imessage" => {
                return Err(anyhow::anyhow!("iMessage gateway is only available on macOS."));
            }
            "teams" => {
                let tenant = gw_cfg.resolve_teams_tenant_id()
                    .ok_or_else(|| anyhow::anyhow!("Teams Tenant ID not configured. Set TEAMS_TENANT_ID or gateway.teams_tenant_id in config."))?;
                let client_id = gw_cfg.resolve_teams_client_id()
                    .ok_or_else(|| anyhow::anyhow!("Teams Client ID not configured. Set TEAMS_CLIENT_ID or gateway.teams_client_id in config."))?;
                let secret = gw_cfg.resolve_teams_client_secret()
                    .ok_or_else(|| anyhow::anyhow!("Teams Client Secret not configured. Set TEAMS_CLIENT_SECRET or gateway.teams_client_secret in config."))?;
                let port = gw_cfg.teams_webhook_port.unwrap_or(3978);
                Box::new(gateway::TeamsGateway::new(tenant, client_id, secret, port).await)
            }
            other => return Err(anyhow::anyhow!(
                "Unknown gateway platform: '{}'. Use: telegram, discord, slack, signal, matrix, twilio, whatsapp, imessage, teams",
                other
            )),
        };
        return gateway::run_gateway(platform_box, llm).await;
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
            Arc::new(ToolExecutor::new(cwd.clone(), sandbox).with_provider(llm.clone()));

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

    // Red Team mode: --redteam <url>
    if let Some(target_url) = cli.redteam {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let mut rt_config = redteam::RedTeamConfig {
            target_url,
            source_path: Some(std::env::current_dir()?),
            ..Default::default()
        };

        // Load YAML config if provided.
        if let Some(config_path) = cli.redteam_config {
            let yaml_str = std::fs::read_to_string(&config_path)?;
            rt_config = serde_yaml::from_str(&yaml_str).unwrap_or(rt_config);
        }

        let session = redteam::run_redteam_pipeline(rt_config, llm).await?;
        let exit_code = if session.findings.iter().any(|f| f.severity == redteam::CvssSeverity::Critical) {
            2
        } else if session.findings.iter().any(|f| f.severity == redteam::CvssSeverity::High) {
            1
        } else {
            0
        };
        std::process::exit(exit_code);
    }

    // Red Team report: --redteam-report <session-id>
    if let Some(session_id) = cli.redteam_report {
        let manager = redteam::RedTeamManager::new()?;
        let session = manager.load_session(&session_id)?;
        let report = redteam::generate_report(&session);
        println!("{}", report);
        std::process::exit(0);
    }

    // Watch mode: --watch [--agent "task"] [--watch-glob "**/*.rs"]
    if cli.watch {
        let watch_task = cli.agent.clone().unwrap_or_else(|| {
            "A file changed. Analyze the change and take any helpful action (e.g. run tests, fix errors).".to_string()
        });
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        return run_watch_mode(llm, &watch_task, &approval_policy, &cli.watch_glob, sandbox_enabled).await;
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
        let llm = create_provider(&exec_provider, exec_model.clone())?;

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

        // Worktree isolation mode (--worktree flag)
        let worktree_branch_hint: Option<String> = if cli.worktree {
            use vibe_ai::WorktreeManager;
            let cwd = std::env::current_dir()?;
            let manager = tool_executor::VibeCoreWorktreeManager::new(cwd.clone());
            let agent_id = format!("wt-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs());
            match manager.create_isolated_worktree(&agent_id) {
                Ok(wt) => {
                    let wt_path = wt.path.clone();
                    let branch = wt.branch.clone();
                    eprintln!("🌿 Worktree isolation: branch '{}' at {}", branch, wt_path.display());
                    eprintln!("   After the agent completes, merge with:");
                    eprintln!("   git merge {}", branch);
                    // Change CWD to the worktree so the agent runs there
                    let _ = std::env::set_current_dir(&wt_path);
                    // Don't drop the wt handle — keep worktree alive for the session
                    std::mem::forget(wt);
                    Some(branch)
                }
                Err(e) => {
                    eprintln!("⚠️  Could not create worktree ({}). Running in current directory.", e);
                    None
                }
            }
        } else {
            None
        };
        let _ = worktree_branch_hint; // used for display only

        let exec_model_str = exec_model.clone().unwrap_or_default();
        return run_agent_repl_with_context(
            llm, &task, &approval_policy,
            cli.resume.as_deref(),
            cli.plan,
            cli.json,
            planning_llm,
            &exec_provider,
            &exec_model_str,
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
            println!("Use: vibecli --agent \"<task to continue>\" --resume {}", &session.session_id[..session.session_id.len().min(8)]);
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
                if let Some(shell_cmd) = input.strip_prefix('!') {
                    let command = shell_cmd.trim();
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
                                llm.clone(), args, &approval_policy, None, false, false, None,
                                &active_provider, active_model.as_deref().unwrap_or(""),
                            ).await?;
                        }
                        "/plan" => {
                            if args.is_empty() {
                                println!("Usage: /plan <task description>");
                                continue;
                            }
                            run_agent_repl_with_context(
                                llm.clone(), args, &approval_policy, None, true, false, None,
                                &active_provider, active_model.as_deref().unwrap_or(""),
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
                                            println!("  {} — {} steps — {}", &s.session_id[..s.session_id.len().min(8)], s.step_count, age);
                                        }
                                        println!("\nUse: /resume <id_prefix> <task to continue>");
                                    }
                                }
                                _ => {
                                    let parts: Vec<&str> = args.splitn(2, ' ').collect();
                                    let sid = parts[0];
                                    let task = if parts.len() > 1 { parts[1] } else { "continue the previous task" };
                                    run_agent_repl_with_context(
                                        llm.clone(), task, &approval_policy, Some(sid), false, false, None,
                                        &active_provider, active_model.as_deref().unwrap_or(""),
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
                                            println!("  {} — {} steps — {}", &session.session_id[..session.session_id.len().min(8)], session.step_count, age);
                                        }
                                        println!("\nUse: /trace view <id_prefix>\n");
                                    }
                                }
                            }
                        }
                        // ── /search ────────────────────────────────────────────────────────
                        // Full-text search across all session traces and messages.
                        // Usage: /search <query>
                        "/search" => {
                            if args.is_empty() {
                                println!("Usage: /search <query>\n");
                                continue;
                            }
                            // Fast SQLite search when DB is available
                            if let Ok(store) = SessionStore::open_default() {
                                match store.search(args) {
                                    Ok(results) if !results.is_empty() => {
                                        println!("\n🔍 Search results for '{}' ({} sessions)\n", args, results.len());
                                        let now_ms = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis() as u64;
                                        for s in results.iter().take(10) {
                                            let age_s = now_ms.saturating_sub(s.started_at) / 1000;
                                            let age = if age_s < 3600 { format!("{}m ago", age_s/60) }
                                                       else if age_s < 86400 { format!("{}h ago", age_s/3600) }
                                                       else { format!("{}d ago", age_s/86400) };
                                            println!("  {} [{}] {} — {} — {} steps",
                                                &s.id[..8.min(s.id.len())],
                                                s.status, s.task, age, s.step_count);
                                        }
                                        println!();
                                        continue;
                                    }
                                    Ok(_) => {
                                        println!("No sessions found matching '{}'\n", args);
                                        continue;
                                    }
                                    Err(_) => {} // fall through to JSONL search
                                }
                            }
                            // Fallback: JSONL search
                            let query = args.to_lowercase();
                            let trace_dir = dirs::home_dir()
                                .unwrap_or_else(|| cwd.clone())
                                .join(".vibecli")
                                .join("traces");
                            let sessions = list_traces(&trace_dir);

                            if sessions.is_empty() {
                                println!("No traces found. Run an agent first.\n");
                                continue;
                            }

                            let mut hits: Vec<(String, u64, Vec<String>)> = Vec::new(); // (session_id, ts, matching_lines)

                            for session in &sessions {
                                let mut matching: Vec<String> = Vec::new();

                                // Search JSONL trace entries
                                let entries = load_trace(&session.path);
                                for entry in &entries {
                                    let haystack = format!("{} {}", entry.tool, entry.input_summary).to_lowercase();
                                    if query.split_whitespace().all(|w| haystack.contains(w)) {
                                        let summary_end = entry.input_summary.char_indices().nth(80).map(|(i,_)| i).unwrap_or(entry.input_summary.len());
                                        matching.push(format!("[step {}] {}: {}", entry.step + 1, entry.tool, &entry.input_summary[..summary_end]));
                                    }
                                }

                                // Search messages sidecar
                                let msgs_path = session.path.with_extension("").to_string_lossy().to_string() + "-messages.json";
                                if let Ok(msgs_raw) = std::fs::read_to_string(&msgs_path) {
                                    if let Ok(msgs) = serde_json::from_str::<Vec<serde_json::Value>>(&msgs_raw) {
                                        for msg in &msgs {
                                            let content = msg["content"].as_str().unwrap_or("").to_lowercase();
                                            if query.split_whitespace().all(|w| content.contains(w)) {
                                                let role = msg["role"].as_str().unwrap_or("?");
                                                let preview: String = content.chars().take(120).collect();
                                                matching.push(format!("[{}] {}", role, preview));
                                                if matching.len() >= 3 { break; }
                                            }
                                        }
                                    }
                                }

                                if !matching.is_empty() {
                                    hits.push((session.session_id.clone(), session.timestamp, matching));
                                }
                            }

                            if hits.is_empty() {
                                println!("No sessions found matching '{}'\n", args);
                            } else {
                                println!("\n🔍 Search results for '{}' ({} sessions match)\n", args, hits.len());
                                for (id, ts, lines) in hits.iter().take(10) {
                                    let elapsed = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH + std::time::Duration::from_secs(*ts))
                                        .unwrap_or_default();
                                    let age = if elapsed.as_secs() < 3600 { format!("{}m ago", elapsed.as_secs() / 60) }
                                        else if elapsed.as_secs() < 86400 { format!("{}h ago", elapsed.as_secs() / 3600) }
                                        else { format!("{}d ago", elapsed.as_secs() / 86400) };
                                    println!("  📋 {} ({})", &id[..id.len().min(12)], age);
                                    for line in lines.iter().take(2) {
                                        println!("     {}", line);
                                    }
                                    println!("  → /trace view {} | /resume {}\n", &id[..id.len().min(8)], &id[..id.len().min(8)]);
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
                                let new_provider = match model_parts.first() {
                                    Some(p) if !p.is_empty() => p,
                                    _ => {
                                        println!("Usage: /model <provider> [model]\n");
                                        continue;
                                    }
                                };
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
                                            let msg_info = match std::fs::read_to_string(entry.path()) {
                                                Err(e) => format!("(unreadable: {})", e),
                                                Ok(s) => match serde_json::from_str::<Vec<Message>>(&s) {
                                                    Ok(m) => format!("{} messages", m.len()),
                                                    Err(e) => format!("(corrupt: {})", e),
                                                },
                                            };
                                            println!("  {} — {} — {}", ts_str, msg_info, age);
                                        }
                                        println!("\nRestore with: /rewind <timestamp>\n");
                                    }
                                }
                                ts_str => {
                                    // Validate: timestamps are numeric only (prevents path traversal)
                                    if !ts_str.chars().all(|c| c.is_ascii_digit()) {
                                        eprintln!("❌ Invalid checkpoint ID '{}'. Expected a numeric timestamp.\n", ts_str);
                                    } else {
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
                                    } // end else (valid timestamp)
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

                        // ── Code Complete workflow ──────────────────────────────────────
                        "/workflow" => {
                            use crate::workflow::WorkflowManager;
                            let cwd = std::env::current_dir()?;
                            let mgr = WorkflowManager::for_workspace(&cwd);
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["list"]
                            } else {
                                args.splitn(3, ' ').collect()
                            };
                            match parts[0] {
                                "list" | "" => {
                                    let names = mgr.list();
                                    if names.is_empty() {
                                        println!("No workflows. Create one with: /workflow new <name> <description>\n");
                                    } else {
                                        println!("Workflows ({}):", names.len());
                                        for name in &names {
                                            if let Ok(w) = mgr.load(name) {
                                                let pct = w.overall_progress();
                                                println!("  {} — {} [{:.0}%]", name, w.current_stage.label(), pct);
                                            }
                                        }
                                        println!();
                                    }
                                }
                                "show" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    if name.is_empty() {
                                        println!("Usage: /workflow show <name>\n");
                                        continue;
                                    }
                                    match mgr.load(name) {
                                        Ok(w) => {
                                            println!("\n🏗️  Workflow: {}  [{:.0}% complete]", w.name, w.overall_progress());
                                            println!("   {}\n", w.description);
                                            for stage in &w.stages {
                                                let marker = if stage.stage == w.current_stage { "▶" }
                                                    else if stage.status == crate::workflow::StageStatus::Complete { "✅" }
                                                    else if stage.status == crate::workflow::StageStatus::Skipped { "⏭" }
                                                    else { "○" };
                                                let pct = if stage.checklist.is_empty() { String::new() }
                                                    else { format!(" ({}/{} — {:.0}%)", stage.completed_count(), stage.total_count(), stage.progress_pct()) };
                                                println!("   {} {}. {}{}", marker, stage.stage.index() + 1, stage.stage.label(), pct);
                                            }
                                            println!();
                                            // Show current stage checklist
                                            let current = w.current_stage_data();
                                            if !current.checklist.is_empty() {
                                                println!("   Current stage checklist:");
                                                for item in &current.checklist {
                                                    let check = if item.done { "✓" } else { " " };
                                                    println!("     [{}] {}: {}", check, item.id, item.description);
                                                }
                                                println!();
                                            }
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "new" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    if name.is_empty() {
                                        println!("Usage: /workflow new <name> <description>\n");
                                        continue;
                                    }
                                    let description = parts.get(2).unwrap_or(&"").trim();
                                    match mgr.create(name, description) {
                                        Ok(_) => {
                                            println!("✅ Workflow '{}' created with 8 stages (Code Complete methodology)", name);
                                            println!("   Current stage: Requirements");
                                            println!("   Use /workflow generate {} to AI-generate a checklist for the current stage.\n", name);
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "advance" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    if name.is_empty() {
                                        println!("Usage: /workflow advance <name>\n");
                                        continue;
                                    }
                                    match mgr.advance_stage(name) {
                                        Ok(w) => {
                                            println!("✅ Advanced to stage: {}\n", w.current_stage.label());
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "check" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    let item_id_str = parts.get(2).unwrap_or(&"").trim();
                                    if name.is_empty() || item_id_str.is_empty() {
                                        println!("Usage: /workflow check <name> <item-id>\n");
                                        continue;
                                    }
                                    match item_id_str.parse::<u32>() {
                                        Ok(item_id) => {
                                            if let Ok(w) = mgr.load(name) {
                                                let stage_idx = w.current_stage.index();
                                                if let Some(item) = w.stages[stage_idx]
                                                    .checklist.iter()
                                                    .find(|c| c.id == item_id)
                                                {
                                                    let currently_done = item.done;
                                                    match mgr.toggle_checklist_item(name, stage_idx, item_id, !currently_done) {
                                                        Ok(_) => println!("✅ Toggled item {} in '{}'\n", item_id, name),
                                                        Err(e) => eprintln!("❌ {}\n", e),
                                                    }
                                                } else {
                                                    eprintln!("❌ Checklist item {} not found in current stage\n", item_id);
                                                }
                                            } else {
                                                eprintln!("❌ Workflow '{}' not found\n", name);
                                            }
                                        }
                                        Err(_) => eprintln!("❌ Invalid item ID: {}\n", item_id_str),
                                    }
                                }
                                "generate" => {
                                    let name = parts.get(1).unwrap_or(&"").trim();
                                    if name.is_empty() {
                                        println!("Usage: /workflow generate <name>\n");
                                        continue;
                                    }
                                    match mgr.load(name) {
                                        Ok(w) => {
                                            let prompt = crate::workflow::stage_checklist_prompt(
                                                &w.current_stage,
                                                &w.description,
                                            );
                                            println!("🤖 Generating {} checklist for '{}'...", w.current_stage.label(), name);
                                            match llm.chat(&[], Some(prompt)).await {
                                                Ok(response) => {
                                                    let items = crate::workflow::parse_checklist_response(&response);
                                                    if items.is_empty() {
                                                        println!("⚠️  Could not parse checklist from response.\n");
                                                    } else {
                                                        let stage_idx = w.current_stage.index();
                                                        match mgr.set_stage_checklist(name, stage_idx, items.clone()) {
                                                            Ok(_) => {
                                                                println!("✅ Generated {} checklist items:", items.len());
                                                                for item in &items {
                                                                    println!("   [ ] {}: {}", item.id, item.description);
                                                                }
                                                                println!();
                                                            }
                                                            Err(e) => eprintln!("❌ {}\n", e),
                                                        }
                                                    }
                                                }
                                                Err(e) => eprintln!("❌ LLM error: {}\n", e),
                                            }
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                _ => println!("Usage: /workflow [new|list|show|advance|check|generate]\n"),
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

                        // ── /remind ────────────────────────────────────────────────────────
                        // Usage: /remind in <duration> "<task>"
                        //        /remind list
                        //        /remind cancel <id>
                        "/remind" => {
                            use crate::scheduler::{Scheduler, parse_duration, format_relative, format_interval};
                            let sched = Scheduler::new();
                            let parts: Vec<&str> = args.splitn(3, ' ').collect();
                            match parts.first().copied().unwrap_or("") {
                                "list" => {
                                    let jobs = sched.list();
                                    if jobs.is_empty() {
                                        println!("No scheduled reminders.\n");
                                    } else {
                                        println!("Scheduled reminders:");
                                        for j in &jobs {
                                            let when = match &j.schedule {
                                                crate::scheduler::ScheduleExpr::Once { at_ms } => format_relative(*at_ms),
                                                crate::scheduler::ScheduleExpr::Recurring { interval_secs, next_at_ms } =>
                                                    format!("{} (next: {})", format_interval(*interval_secs), format_relative(*next_at_ms)),
                                            };
                                            println!("  [{}] {} — {}", j.id, when, j.task);
                                        }
                                        println!();
                                    }
                                }
                                "cancel" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    match sched.cancel(id) {
                                        Ok(Some(j)) => println!("✅ Cancelled reminder [{}]: {}\n", j.id, j.task),
                                        Ok(None) => println!("⚠️  No reminder found with id prefix '{}'\n", id),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "in" => {
                                    let dur_str = parts.get(1).unwrap_or(&"").trim();
                                    let task = parts.get(2).unwrap_or(&"").trim().trim_matches('"');
                                    if let Some(secs) = parse_duration(dur_str) {
                                        match sched.add_in(task, secs) {
                                            Ok(j) => println!("✅ Reminder [{}] set: '{}' in {}\n", j.id, j.task, dur_str),
                                            Err(e) => eprintln!("❌ {}\n", e),
                                        }
                                    } else {
                                        println!("⚠️  Invalid duration '{}'. Use: 30s, 10m, 2h, 1d\n", dur_str);
                                    }
                                }
                                _ => println!("Usage: /remind in <dur> \"<task>\" | /remind list | /remind cancel <id>\n"),
                            }
                        }

                        // ── /schedule ──────────────────────────────────────────────────────
                        // Usage: /schedule every <duration> "<task>"
                        //        /schedule list
                        //        /schedule cancel <id>
                        "/linear" => {
                            let output = crate::linear::handle_linear_command(args).await;
                            print!("{}", output);
                        }

                        "/schedule" => {
                            use crate::scheduler::{Scheduler, parse_duration, format_relative, format_interval};
                            let sched = Scheduler::new();
                            let parts: Vec<&str> = args.splitn(3, ' ').collect();
                            match parts.first().copied().unwrap_or("") {
                                "list" => {
                                    let jobs = sched.list();
                                    if jobs.is_empty() {
                                        println!("No scheduled jobs.\n");
                                    } else {
                                        println!("Scheduled jobs:");
                                        for j in &jobs {
                                            let when = match &j.schedule {
                                                crate::scheduler::ScheduleExpr::Once { at_ms } => format_relative(*at_ms),
                                                crate::scheduler::ScheduleExpr::Recurring { interval_secs, next_at_ms } =>
                                                    format!("{} (next: {})", format_interval(*interval_secs), format_relative(*next_at_ms)),
                                            };
                                            println!("  [{}] {} — {} (triggered {} times)", j.id, when, j.task, j.triggered_count);
                                        }
                                        println!();
                                    }
                                }
                                "cancel" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    match sched.cancel(id) {
                                        Ok(Some(j)) => println!("✅ Cancelled job [{}]: {}\n", j.id, j.task),
                                        Ok(None) => println!("⚠️  No job found with id prefix '{}'\n", id),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "every" => {
                                    let dur_str = parts.get(1).unwrap_or(&"").trim();
                                    let task = parts.get(2).unwrap_or(&"").trim().trim_matches('"');
                                    if let Some(secs) = parse_duration(dur_str) {
                                        match sched.add_recurring(task, secs) {
                                            Ok(j) => println!("✅ Recurring job [{}]: '{}' every {}\n", j.id, j.task, dur_str),
                                            Err(e) => eprintln!("❌ {}\n", e),
                                        }
                                    } else {
                                        println!("⚠️  Invalid interval '{}'. Use: 30s, 10m, 2h, 1d\n", dur_str);
                                    }
                                }
                                _ => println!("Usage: /schedule every <interval> \"<task>\" | /schedule list | /schedule cancel <id>\n"),
                            }
                        }

                        // ── /snippet — save/list/use named code snippets ──────────────
                        "/snippet" => {
                            let snippet_dir = dirs::home_dir()
                                .unwrap_or_else(|| cwd.clone())
                                .join(".vibecli")
                                .join("snippets");
                            let _ = std::fs::create_dir_all(&snippet_dir);

                            let sub_parts: Vec<&str> = args.splitn(3, ' ').collect();
                            let sub = sub_parts.first().copied().unwrap_or("").trim();

                            match sub {
                                "list" | "" => {
                                    match std::fs::read_dir(&snippet_dir) {
                                        Ok(entries) => {
                                            let mut names: Vec<String> = entries
                                                .filter_map(|e| e.ok())
                                                .filter_map(|e| {
                                                    let p = e.path();
                                                    if p.extension().and_then(|x| x.to_str()) == Some("md") {
                                                        p.file_stem().map(|s| s.to_string_lossy().to_string())
                                                    } else { None }
                                                })
                                                .collect();
                                            names.sort();
                                            if names.is_empty() {
                                                println!("No snippets saved yet. Use: /snippet save <name>\n");
                                            } else {
                                                println!("📌 Saved snippets ({}):", names.len());
                                                for n in &names { println!("  - {}", n); }
                                                println!();
                                            }
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "save" => {
                                    let name = sub_parts.get(1).copied().unwrap_or("").trim();
                                    if name.is_empty() {
                                        println!("Usage: /snippet save <name>\n       Saves the last AI response as a named snippet.\n");
                                    } else if !is_safe_name(name) {
                                        eprintln!("❌ Invalid snippet name '{}'. Use only letters, digits, hyphens and underscores.\n", name);
                                    } else {
                                        // Find the last assistant message
                                        let last_assistant = messages.iter().rev()
                                            .find(|m| m.role == MessageRole::Assistant)
                                            .map(|m| m.content.clone());
                                        match last_assistant {
                                            Some(content) => {
                                                let path = snippet_dir.join(format!("{}.md", name));
                                                match std::fs::write(&path, &content) {
                                                    Ok(()) => println!("💾 Snippet '{}' saved.\n", name),
                                                    Err(e) => eprintln!("❌ Failed to save: {}\n", e),
                                                }
                                            }
                                            None => println!("⚠️  No assistant message in current session to save.\n"),
                                        }
                                    }
                                }
                                "use" | "insert" => {
                                    let name = sub_parts.get(1).copied().unwrap_or("").trim();
                                    if name.is_empty() {
                                        println!("Usage: /snippet use <name>\n       Injects snippet as context in the next message.\n");
                                    } else if !is_safe_name(name) {
                                        eprintln!("❌ Invalid snippet name '{}'.\n", name);
                                    } else {
                                        let path = snippet_dir.join(format!("{}.md", name));
                                        match std::fs::read_to_string(&path) {
                                            Ok(content) => {
                                                println!("📌 Snippet '{}':\n---\n{}\n---\n", name, content);
                                                // Inject as a user context message
                                                messages.push(Message {
                                                    role: MessageRole::User,
                                                    content: format!("Here is the '{}' snippet for reference:\n\n{}", name, content),
                                                });
                                                messages.push(Message {
                                                    role: MessageRole::Assistant,
                                                    content: format!("Got it — I've noted the '{}' snippet.", name),
                                                });
                                            }
                                            Err(_) => println!("⚠️  Snippet '{}' not found.\n", name),
                                        }
                                    }
                                }
                                "delete" | "rm" => {
                                    let name = sub_parts.get(1).copied().unwrap_or("").trim();
                                    if name.is_empty() {
                                        println!("Usage: /snippet delete <name>\n");
                                    } else if !is_safe_name(name) {
                                        eprintln!("❌ Invalid snippet name '{}'.\n", name);
                                    } else {
                                        let path = snippet_dir.join(format!("{}.md", name));
                                        match std::fs::remove_file(&path) {
                                            Ok(()) => println!("🗑️  Snippet '{}' deleted.\n", name),
                                            Err(_) => println!("⚠️  Snippet '{}' not found.\n", name),
                                        }
                                    }
                                }
                                "show" | "cat" => {
                                    let name = sub_parts.get(1).copied().unwrap_or("").trim();
                                    if name.is_empty() {
                                        println!("Usage: /snippet show <name>\n");
                                    } else if !is_safe_name(name) {
                                        eprintln!("❌ Invalid snippet name '{}'.\n", name);
                                    } else {
                                        let path = snippet_dir.join(format!("{}.md", name));
                                        match std::fs::read_to_string(&path) {
                                            Ok(content) => {
                                                println!("📌 Snippet '{}':\n---\n{}\n---\n", name, content);
                                            }
                                            Err(_) => println!("⚠️  Snippet '{}' not found.\n", name),
                                        }
                                    }
                                }
                                _ => println!("Usage: /snippet [list|save <name>|use <name>|show <name>|delete <name>]\n"),
                            }
                        }

                        // ── /jobs ──────────────────────────────────────────────────────────
                        // Usage: /jobs           → list recent jobs
                        //        /jobs <id>      → show full detail for a job
                        "/jobs" => {
                            let jobs_dir = {
                                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                                std::path::PathBuf::from(home).join(".vibecli").join("jobs")
                            };

                            if !args.is_empty() {
                                // Show detail for a single job
                                let job_path = jobs_dir.join(format!("{}.json", args));
                                if !job_path.exists() {
                                    eprintln!("❌ Job not found: {}\n", args);
                                } else {
                                    match std::fs::read_to_string(&job_path)
                                        .map_err(|e| e.to_string())
                                        .and_then(|s| serde_json::from_str::<crate::serve::JobRecord>(&s).map_err(|e| e.to_string()))
                                    {
                                        Ok(rec) => {
                                            let icon = match rec.status.as_str() {
                                                "complete"  => "✅",
                                                "running"   => "🟡",
                                                "failed"    => "❌",
                                                "cancelled" => "⛔",
                                                _           => "❓",
                                            };
                                            println!("\n{} Job: {}", icon, rec.session_id);
                                            println!("  Status  : {}", rec.status);
                                            println!("  Provider: {}", rec.provider);
                                            println!("  Task    : {}", rec.task);
                                            let started = rec.started_at / 1000;
                                            let elapsed_now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default()
                                                .as_secs()
                                                .saturating_sub(started);
                                            println!("  Started : {}s ago", elapsed_now);
                                            if let Some(fin) = rec.finished_at {
                                                let duration_ms = fin.saturating_sub(rec.started_at);
                                                println!("  Duration: {:.1}s", duration_ms as f64 / 1000.0);
                                            }
                                            if let Some(summary) = &rec.summary {
                                                println!("  Summary : {}", summary);
                                            }
                                            println!();
                                        }
                                        Err(e) => eprintln!("❌ Failed to read job record: {}\n", e),
                                    }
                                }
                            } else if !jobs_dir.exists() {
                                println!("No background jobs found (jobs directory does not exist).\n");
                            } else {
                                let mut records: Vec<crate::serve::JobRecord> = Vec::new();
                                if let Ok(rd) = std::fs::read_dir(&jobs_dir) {
                                    for entry in rd.flatten() {
                                        let p = entry.path();
                                        if p.extension().and_then(|e| e.to_str()) == Some("json") {
                                            if let Ok(raw) = std::fs::read_to_string(&p) {
                                                if let Ok(rec) = serde_json::from_str::<crate::serve::JobRecord>(&raw) {
                                                    records.push(rec);
                                                }
                                            }
                                        }
                                    }
                                }
                                records.sort_by(|a, b| b.started_at.cmp(&a.started_at));
                                if records.is_empty() {
                                    println!("No background jobs found.\n");
                                } else {
                                    println!("{:<38} {:<10} TASK", "SESSION ID", "STATUS");
                                    println!("{}", "-".repeat(80));
                                    for rec in records.iter().take(20) {
                                        let icon = match rec.status.as_str() {
                                            "complete"  => "✅",
                                            "running"   => "🟡",
                                            "failed"    => "❌",
                                            "cancelled" => "⛔",
                                            _           => "❓",
                                        };
                                        let preview: String = rec.task.chars().take(50).collect();
                                        let preview = if rec.task.len() > 50 { format!("{}…", preview) } else { preview };
                                        println!("{:<38} {} {:<9} {}", rec.session_id, icon, rec.status, preview);
                                    }
                                    println!("  (use /jobs <session_id> for full detail)\n");
                                }
                            }
                        }

                        "/sessions" => {
                            // List recent agent sessions from SQLite store.
                            // With no args: show last 15 root sessions.
                            // With a prefix: filter by session ID prefix.
                            match SessionStore::open_default() {
                                Ok(store) => {
                                    match store.list_root_sessions(15) {
                                        Ok(sessions) if sessions.is_empty() => {
                                            println!("📋 No sessions recorded yet. Sessions are saved when you run /agent tasks.\n");
                                        }
                                        Ok(sessions) => {
                                            let filter = args.trim().to_lowercase();
                                            let filtered: Vec<_> = sessions.iter()
                                                .filter(|s| {
                                                    filter.is_empty() || s.id.starts_with(&filter)
                                                })
                                                .collect();
                                            if filtered.is_empty() {
                                                println!("No sessions matching '{}'.\n", args.trim());
                                            } else {
                                                println!("\n📋 Recent sessions ({}):\n", filtered.len());
                                                println!("  {:<10}  {:<8}  {:<7}  {:<5}  Task",
                                                    "ID", "Status", "Steps", "Model");
                                                println!("  {}", "─".repeat(72));
                                                for s in &filtered {
                                                    // Human-readable elapsed time
                                                    let now_ms = std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap_or_default()
                                                        .as_millis() as u64;
                                                    let elapsed_s = now_ms.saturating_sub(s.started_at) / 1000;
                                                    let age = if elapsed_s < 60 {
                                                        format!("{}s", elapsed_s)
                                                    } else if elapsed_s < 3600 {
                                                        format!("{}m", elapsed_s / 60)
                                                    } else {
                                                        format!("{}h", elapsed_s / 3600)
                                                    };
                                                    let status_icon = match s.status.as_str() {
                                                        "complete" => "✅",
                                                        "running"  => "🟡",
                                                        "failed"   => "❌",
                                                        _          => "⚪",
                                                    };
                                                    let task_preview = if s.task.len() > 45 {
                                                        format!("{}…", &s.task[..45])
                                                    } else {
                                                        s.task.clone()
                                                    };
                                                    let model_short = s.model.rsplit('/').next()
                                                        .unwrap_or(&s.model)
                                                        .chars().take(12).collect::<String>();
                                                    println!("  {:<10}  {} {:<7}  {:>5}  {} — {}",
                                                        &s.id[..s.id.len().min(10)],
                                                        status_icon, s.status,
                                                        s.step_count,
                                                        task_preview,
                                                        model_short,
                                                    );
                                                    println!("              ({} ago)  /resume {} \"continue the task\"",
                                                        age, &s.id[..s.id.len().min(10)]);
                                                }
                                                println!();
                                            }
                                        }
                                        Err(e) => {
                                            println!("⚠️  Could not read session store: {}\n", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("⚠️  Session store unavailable: {}\n", e);
                                }
                            }
                        }

                        "/share" => {
                            if args.is_empty() {
                                println!("Usage: /share <session_id>\n\
                                         Prints a shareable URL for a session when 'vibecli serve' is running.\n\
                                         Example: /share 193abc4def\n");
                            } else {
                                let port: u16 = 7878; // default daemon port
                                let url = format!("http://localhost:{}/share/{}", port, args.trim());
                                println!("📤  Shareable session URL:\n    {}\n", url);
                                println!("    (The daemon must be running: vibecli serve --port {})\n", port);
                            }
                        }

                        "/redteam" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["help"]
                            } else {
                                args.splitn(3, ' ').collect()
                            };
                            match parts[0] {
                                "scan" => {
                                    let target = parts.get(1).unwrap_or(&"").trim();
                                    if target.is_empty() {
                                        println!("Usage: /redteam scan <url> [--repo <path>]\n");
                                        continue;
                                    }
                                    let llm = create_provider(&effective_provider, effective_model.clone())?;
                                    let mut rt_config = redteam::RedTeamConfig {
                                        target_url: target.to_string(),
                                        source_path: Some(std::env::current_dir()?),
                                        ..Default::default()
                                    };
                                    // Check for --repo flag in remaining args.
                                    if let Some(rest) = parts.get(2) {
                                        if rest.contains("--repo") {
                                            let repo_path = rest.replace("--repo", "").trim().to_string();
                                            if !repo_path.is_empty() {
                                                rt_config.source_path = Some(std::path::PathBuf::from(repo_path));
                                            }
                                        }
                                    }
                                    match redteam::run_redteam_pipeline(rt_config, llm).await {
                                        Ok(session) => {
                                            println!("{}", redteam::format_findings(&session.findings));
                                        }
                                        Err(e) => eprintln!("❌ Red team scan failed: {}\n", e),
                                    }
                                }
                                "list" => {
                                    match redteam::RedTeamManager::new().and_then(|m| m.list_sessions()) {
                                        Ok(sessions) => {
                                            if sessions.is_empty() {
                                                println!("No red team sessions. Start one with: /redteam scan <url>\n");
                                            } else {
                                                println!("Red Team Sessions ({}):", sessions.len());
                                                for s in &sessions {
                                                    println!("  {}", s.summary_line());
                                                }
                                                println!();
                                            }
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "show" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    if id.is_empty() {
                                        println!("Usage: /redteam show <session-id>\n");
                                        continue;
                                    }
                                    match redteam::RedTeamManager::new().and_then(|m| m.load_session(id)) {
                                        Ok(session) => {
                                            println!("{}", redteam::format_findings(&session.findings));
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "report" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    if id.is_empty() {
                                        println!("Usage: /redteam report <session-id>\n");
                                        continue;
                                    }
                                    match redteam::RedTeamManager::new().and_then(|m| m.load_session(id)) {
                                        Ok(session) => {
                                            println!("{}", redteam::generate_report(&session));
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "config" => {
                                    let rt_cfg = Config::load().unwrap_or_default().redteam;
                                    println!("Red Team Configuration (from ~/.vibecli/config.toml):");
                                    println!("  max_depth: {}", rt_cfg.max_depth);
                                    println!("  timeout_secs: {}", rt_cfg.timeout_secs);
                                    println!("  parallel_agents: {}", rt_cfg.parallel_agents);
                                    println!("  auto_report: {}", rt_cfg.auto_report);
                                    println!();
                                }
                                _ => {
                                    println!("🛡️  Red Team Commands:");
                                    println!("  /redteam scan <url> [--repo <path>]  — run security scan");
                                    println!("  /redteam list                        — list all sessions");
                                    println!("  /redteam show <id>                   — show findings");
                                    println!("  /redteam report <id>                 — generate full report");
                                    println!("  /redteam config                      — show configuration");
                                    println!();
                                }
                            }
                        }

                        // ── /arena ─────────────────────────────────────────────────────
                        "/arena" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["help"]
                            } else {
                                args.splitn(2, ' ').collect()
                            };
                            match parts[0] {
                                "compare" => {
                                    let rest = parts.get(1).unwrap_or(&"");
                                    let tokens: Vec<&str> = rest.split_whitespace().collect();
                                    if tokens.len() < 2 {
                                        println!("Usage: /arena compare <provider1> <provider2> [prompt]\n");
                                        continue;
                                    }
                                    let p1 = tokens[0];
                                    let p2 = tokens[1];
                                    let prompt_text = if tokens.len() > 2 {
                                        tokens[2..].join(" ")
                                    } else {
                                        println!("Enter prompt: ");
                                        let mut buf = String::new();
                                        std::io::stdin().read_line(&mut buf).unwrap_or(0);
                                        buf.trim().to_string()
                                    };
                                    if prompt_text.is_empty() {
                                        println!("No prompt provided.\n");
                                        continue;
                                    }
                                    println!("Arena: {} vs {} ...", p1, p2);
                                    let llm_a = create_provider(p1, None);
                                    let llm_b = create_provider(p2, None);
                                    match (llm_a, llm_b) {
                                        (Ok(a), Ok(b)) => {
                                            use vibe_ai::provider::{Message, MessageRole};
                                            let msgs = vec![Message { role: MessageRole::User, content: prompt_text.clone() }];
                                            let (r_a, r_b) = tokio::join!(
                                                a.chat_response(&msgs, None),
                                                b.chat_response(&msgs, None),
                                            );
                                            println!("\n-- Model A ------------------------------------------");
                                            match &r_a {
                                                Ok(r) => println!("{}\n", r.text),
                                                Err(e) => println!("Error: {}\n", e),
                                            }
                                            println!("-- Model B ------------------------------------------");
                                            match &r_b {
                                                Ok(r) => println!("{}\n", r.text),
                                                Err(e) => println!("Error: {}\n", e),
                                            }
                                            println!("Which is better?  [a / b / tie / skip]");
                                            let mut vote = String::new();
                                            std::io::stdin().read_line(&mut vote).unwrap_or(0);
                                            let v = vote.trim().to_lowercase();
                                            if matches!(v.as_str(), "a" | "b" | "tie") {
                                                println!("Voted: {}  -- Reveal: A={}, B={}\n", v, p1, p2);
                                            } else {
                                                println!("Skipped.\n");
                                            }
                                        }
                                        _ => println!("Failed to create one or both providers.\n"),
                                    }
                                }
                                "stats" => {
                                    println!("Arena stats: use the VibeUI Arena tab for full leaderboard.\n");
                                }
                                "history" => {
                                    println!("Arena history: use the VibeUI Arena tab for full history.\n");
                                }
                                _ => {
                                    println!("Usage:");
                                    println!("  /arena compare <p1> <p2> [prompt]  -- blind A/B comparison");
                                    println!("  /arena stats                       -- show leaderboard");
                                    println!("  /arena history                     -- show vote history");
                                    println!();
                                }
                            }
                        }

                        // ── /test ──────────────────────────────────────────────────────
                        "/test" => {
                            let cwd = std::env::current_dir()?;
                            let ws = cwd.to_string_lossy().to_string();
                            // Auto-detect or use custom command
                            let cmd = if args.trim().is_empty() {
                                // Auto-detect
                                if cwd.join("Cargo.toml").exists() {
                                    "cargo test".to_string()
                                } else if cwd.join("package.json").exists() {
                                    "npm test".to_string()
                                } else if cwd.join("pytest.ini").exists() || cwd.join("pyproject.toml").exists() || cwd.join("setup.py").exists() {
                                    "python -m pytest -v".to_string()
                                } else if cwd.join("go.mod").exists() {
                                    "go test ./...".to_string()
                                } else {
                                    println!("❌ Cannot detect test framework. Use: /test <command>\n");
                                    continue;
                                }
                            } else {
                                args.trim().to_string()
                            };
                            let _ = ws;
                            println!("🧪 Running: {}\n", cmd);
                            let (prog, cmd_args) = if cmd.starts_with("cargo") {
                                ("cargo", vec!["test"])
                            } else if cmd.starts_with("npm") {
                                ("npm", vec!["test"])
                            } else if cmd.starts_with("python") || cmd.starts_with("pytest") {
                                ("python", vec!["-m", "pytest", "-v"])
                            } else if cmd.starts_with("go test") {
                                ("go", vec!["test", "./..."])
                            } else {
                                ("sh", vec!["-c", &cmd])
                            };
                            let status = std::process::Command::new(prog)
                                .args(&cmd_args)
                                .current_dir(&cwd)
                                .status();
                            match status {
                                Ok(s) if s.success() => println!("✅ Tests passed\n"),
                                Ok(_) => println!("❌ Tests failed\n"),
                                Err(e) => println!("❌ Failed to run tests: {}\n", e),
                            }
                        }

                        // ── /autofix ───────────────────────────────────────────────────
                        "/deploy" => {
                            let cwd = std::env::current_dir()?;

                            // Deploy target table: (id, cli_tool, deploy_cmd, description)
                            let deploy_targets: &[(&str, &str, &str, &str)] = &[
                                ("vercel",          "vercel",     "vercel deploy --yes",                                   "Vercel"),
                                ("netlify",         "netlify",    "netlify deploy --prod --dir=dist",                      "Netlify"),
                                ("railway",         "railway",    "railway up",                                            "Railway"),
                                ("github-pages",    "gh",         "npm run build && npx gh-pages -d dist",                 "GitHub Pages"),
                                ("gcp",             "gcloud",     "gcloud run deploy --source . --platform=managed --region=us-central1 --allow-unauthenticated", "GCP Cloud Run"),
                                ("firebase",        "firebase",   "firebase deploy --only hosting",                        "Firebase"),
                                ("aws-apprunner",   "aws",        "copilot deploy 2>&1 || aws apprunner create-service --service-name $(basename $(pwd)) --source-configuration '{}'", "AWS App Runner"),
                                ("aws-s3",          "aws",        "npm run build 2>/dev/null; aws s3 sync dist/ s3://$(basename $(pwd))-deploy --delete", "AWS S3 + CloudFront"),
                                ("aws-lambda",      "serverless", "serverless deploy",                                     "AWS Lambda (Serverless)"),
                                ("aws-ecs",         "aws",        "docker build -t app . && aws ecs update-service --cluster default --service $(basename $(pwd)) --force-new-deployment", "AWS ECS/Fargate"),
                                ("azure-appservice", "az",        "az webapp up --name $(basename $(pwd))",                "Azure App Service"),
                                ("azure-container", "az",         "az containerapp up --name $(basename $(pwd)) --source .", "Azure Container Apps"),
                                ("azure-static",    "swa",        "swa deploy --app-location . --output-location dist",   "Azure Static Web Apps"),
                                ("digitalocean",    "doctl",      "doctl apps create --spec .do/app.yaml",                 "DigitalOcean App Platform"),
                                ("kubernetes",      "kubectl",    "kubectl apply -f k8s/ 2>&1 || kubectl apply -f .",      "Kubernetes"),
                                ("helm",            "helm",       "helm upgrade --install $(basename $(pwd)) .",           "Kubernetes (Helm)"),
                                ("oci",             "oci",        "fn deploy --app $(basename $(pwd))",                    "Oracle Cloud"),
                                ("ibm",             "ibmcloud",   "ibmcloud ce app create --name $(basename $(pwd)) --build-source .", "IBM Code Engine"),
                            ];

                            fn cli_available(tool: &str) -> bool {
                                std::process::Command::new("sh")
                                    .args(["-c", &format!("command -v {} >/dev/null 2>&1", tool)])
                                    .status()
                                    .map(|s| s.success())
                                    .unwrap_or(false)
                            }

                            let target_arg = args.trim().to_lowercase();

                            if target_arg == "list" || target_arg == "help" {
                                println!("Available deploy targets:\n");
                                for (key, cli, _, desc) in deploy_targets {
                                    let mark = if cli_available(cli) { "✅" } else { "❌" };
                                    println!("  {mark} {key:<18} {desc} (requires: {cli})");
                                }
                                println!("\nUsage: /deploy <target>  or  /deploy  (auto-detect)\n");
                                continue;
                            }

                            // Auto-detect target if none given
                            let resolved = if target_arg.is_empty() {
                                if cwd.join("serverless.yml").exists() || cwd.join("serverless.ts").exists() {
                                    "aws-lambda"
                                } else if cwd.join("Chart.yaml").exists() {
                                    "helm"
                                } else if cwd.join("k8s").is_dir() {
                                    "kubernetes"
                                } else if cwd.join("Dockerfile").exists() {
                                    if cli_available("aws") { "aws-apprunner" }
                                    else if cli_available("az") { "azure-container" }
                                    else if cli_available("doctl") { "digitalocean" }
                                    else if cli_available("gcloud") { "gcp" }
                                    else { "vercel" }
                                } else if cwd.join("firebase.json").exists() {
                                    "firebase"
                                } else if cwd.join("vercel.json").exists() {
                                    "vercel"
                                } else if cwd.join("netlify.toml").exists() {
                                    "netlify"
                                } else if cwd.join("package.json").exists() {
                                    "vercel"
                                } else {
                                    println!("❌ Cannot auto-detect deploy target. Use: /deploy <target>");
                                    println!("Run /deploy list to see available targets.\n");
                                    continue;
                                }
                            } else {
                                // Resolve aliases
                                match target_arg.as_str() {
                                    "aws" => {
                                        if cwd.join("serverless.yml").exists() || cwd.join("serverless.ts").exists() { "aws-lambda" }
                                        else if cwd.join("Dockerfile").exists() { "aws-apprunner" }
                                        else { "aws-s3" }
                                    }
                                    "azure" => {
                                        if cwd.join("Dockerfile").exists() { "azure-container" }
                                        else if cwd.join("staticwebapp.config.json").exists() { "azure-static" }
                                        else { "azure-appservice" }
                                    }
                                    "k8s" | "kube" => "kubernetes",
                                    "do" => "digitalocean",
                                    "oracle" => "oci",
                                    "gcp" | "google" => "gcp",
                                    other => other,
                                }
                            };

                            let entry = deploy_targets.iter().find(|(k, _, _, _)| *k == resolved);
                            match entry {
                                Some((_, cli, cmd, desc)) => {
                                    if !cli_available(cli) {
                                        println!("❌ {} CLI not found (required for {}). Install it first.\n", cli, desc);
                                        continue;
                                    }
                                    println!("🚀 Deploying to {} ({})...\n", resolved, desc);
                                    println!("Running: {}\n", cmd);
                                    let status = std::process::Command::new("sh")
                                        .args(["-c", cmd])
                                        .current_dir(&cwd)
                                        .status();
                                    match status {
                                        Ok(s) if s.success() => println!("\n✅ Deployment succeeded!\n"),
                                        Ok(_) => println!("\n❌ Deployment failed. Check output above.\n"),
                                        Err(e) => println!("\n❌ Failed to run deploy: {}\n", e),
                                    }
                                }
                                None => {
                                    println!("❌ Unknown target: {}. Run /deploy list for options.\n", resolved);
                                }
                            }
                        }

                        "/autofix" => {
                            let cwd = std::env::current_dir()?;
                            let fw = if args.trim().is_empty() {
                                // Auto-detect
                                if cwd.join("Cargo.toml").exists() { "cargo clippy --fix --allow-dirty --allow-staged -q" }
                                else if cwd.join("package.json").exists() { "npx eslint --fix ." }
                                else if cwd.join("pyproject.toml").exists() || cwd.join("setup.py").exists() { "ruff check --fix ." }
                                else if cwd.join("go.mod").exists() { "gofmt -w ." }
                                else { println!("❌ Cannot detect linter. Use: /autofix <clippy|eslint|ruff|gofmt|prettier>\n"); continue; }
                            } else {
                                match args.trim() {
                                    "clippy"   => "cargo clippy --fix --allow-dirty --allow-staged -q",
                                    "eslint"   => "npx eslint --fix .",
                                    "ruff"     => "ruff check --fix .",
                                    "gofmt"    => "gofmt -w .",
                                    "prettier" => "npx prettier --write .",
                                    other      => { println!("❌ Unknown framework: {}. Use clippy|eslint|ruff|gofmt|prettier\n", other); continue; }
                                }
                            };
                            println!("🔧 Running: {}\n", fw);
                            let status = std::process::Command::new("sh")
                                .args(["-c", fw])
                                .current_dir(&cwd)
                                .status();
                            match status {
                                Ok(s) if s.success() => {
                                    // Show diff
                                    let diff = std::process::Command::new("git")
                                        .args(["diff", "--stat"])
                                        .current_dir(&cwd)
                                        .output()
                                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                                        .unwrap_or_default();
                                    if diff.trim().is_empty() {
                                        println!("✅ No issues found — code is already clean!\n");
                                    } else {
                                        println!("✅ Fixed! Changes:\n{}\nUse `git add -u && git commit` to apply.\n", diff);
                                    }
                                }
                                Ok(_) => println!("⚠️  Autofix ran with warnings/errors. Check output above.\n"),
                                Err(e) => println!("❌ Failed to run autofix: {}\n", e),
                            }
                        }

                        "/env" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                            let cwd = std::env::current_dir().unwrap_or_default();

                            fn is_secret_key(key: &str) -> bool {
                                let upper = key.to_uppercase();
                                ["SECRET", "TOKEN", "PASSWORD", "CREDENTIAL", "PRIVATE", "API_KEY", "_KEY"]
                                    .iter()
                                    .any(|pat| upper.contains(pat))
                            }

                            fn parse_env_file(path: &std::path::Path) -> Vec<(String, String)> {
                                let content = match std::fs::read_to_string(path) {
                                    Ok(c) => c,
                                    Err(_) => return Vec::new(),
                                };
                                let mut entries = Vec::new();
                                for line in content.lines() {
                                    let trimmed = line.trim();
                                    if trimmed.is_empty() || trimmed.starts_with('#') {
                                        continue;
                                    }
                                    if let Some(eq_pos) = trimmed.find('=') {
                                        let key = trimmed[..eq_pos].trim().to_string();
                                        let mut value = trimmed[eq_pos + 1..].trim().to_string();
                                        if value.len() >= 2
                                            && ((value.starts_with('"') && value.ends_with('"'))
                                                || (value.starts_with('\'') && value.ends_with('\'')))
                                        {
                                            value = value[1..value.len() - 1].to_string();
                                        }
                                        entries.push((key, value));
                                    }
                                }
                                entries
                            }

                            // Determine active env file
                            let active_env_path = cwd.join(".vibeui").join("active-env.txt");
                            let active_env = std::fs::read_to_string(&active_env_path)
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| "default".to_string());
                            let env_filename = if active_env == "default" { ".env".to_string() } else { format!(".env.{}", active_env) };
                            let env_path = cwd.join(&env_filename);

                            match subcmd {
                                "" | "list" => {
                                    if !env_path.exists() {
                                        println!("\n📋 No {} file found. Use `/env create` or `/env set KEY value`.\n", env_filename);
                                    } else {
                                        let entries = parse_env_file(&env_path);
                                        println!("\n🔑 Environment: {} ({})", active_env, env_filename);
                                        if entries.is_empty() {
                                            println!("  (empty)\n");
                                        } else {
                                            for (key, value) in &entries {
                                                if is_secret_key(key) {
                                                    println!("  {:<30} ••••••••  🔒", key);
                                                } else {
                                                    println!("  {:<30} {}", key, value);
                                                }
                                            }
                                            println!("  ({} variables)\n", entries.len());
                                        }
                                    }
                                }
                                "files" => {
                                    println!("\n📁 Environment files:");
                                    let mut found = false;
                                    if let Ok(dir) = std::fs::read_dir(&cwd) {
                                        let mut files: Vec<_> = dir
                                            .flatten()
                                            .filter(|e| {
                                                let name = e.file_name().to_string_lossy().to_string();
                                                name == ".env" || name.starts_with(".env.")
                                            })
                                            .collect();
                                        files.sort_by_key(|e| e.file_name());
                                        for entry in &files {
                                            found = true;
                                            let name = entry.file_name().to_string_lossy().to_string();
                                            let entries = parse_env_file(&entry.path());
                                            let marker = if name == env_filename { " ← active" } else { "" };
                                            println!("  {} ({} vars){}", name, entries.len(), marker);
                                        }
                                    }
                                    if !found {
                                        println!("  (no .env files found)");
                                    }
                                    println!();
                                }
                                "get" => {
                                    let key = sub_args.trim();
                                    if key.is_empty() {
                                        println!("Usage: /env get <KEY>\n");
                                    } else {
                                        let entries = parse_env_file(&env_path);
                                        match entries.iter().find(|(k, _)| k == key) {
                                            Some((k, v)) => println!("\n  {}={}\n", k, v),
                                            None => println!("\n  Key \"{}\" not found in {}\n", key, env_filename),
                                        }
                                    }
                                }
                                "set" => {
                                    let set_parts: Vec<&str> = sub_args.splitn(2, ' ').collect();
                                    let key = set_parts.first().copied().unwrap_or("").trim().to_uppercase();
                                    let value = if set_parts.len() > 1 { set_parts[1].trim() } else { "" };
                                    if key.is_empty() || value.is_empty() {
                                        println!("Usage: /env set <KEY> <value>\n");
                                    } else {
                                        // Read existing content (or empty)
                                        let content = std::fs::read_to_string(&env_path).unwrap_or_default();
                                        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
                                        let mut found = false;
                                        for line in &mut lines {
                                            let trimmed = line.trim();
                                            if let Some(eq_pos) = trimmed.find('=') {
                                                let line_key = trimmed[..eq_pos].trim();
                                                if line_key == key {
                                                    *line = format!("{}={}", key, value);
                                                    found = true;
                                                    break;
                                                }
                                            }
                                        }
                                        if !found {
                                            lines.push(format!("{}={}", key, value));
                                        }
                                        let new_content = lines.join("\n") + "\n";
                                        match std::fs::write(&env_path, &new_content) {
                                            Ok(_) => {
                                                #[cfg(unix)]
                                                {
                                                    use std::os::unix::fs::PermissionsExt;
                                                    let _ = std::fs::set_permissions(&env_path, std::fs::Permissions::from_mode(0o600));
                                                }
                                                let action = if found { "Updated" } else { "Added" };
                                                println!("✅ {} {}={} in {}\n", action, key, value, env_filename);
                                            }
                                            Err(e) => println!("❌ Failed to write {}: {}\n", env_filename, e),
                                        }
                                    }
                                }
                                "delete" => {
                                    let key = sub_args.trim();
                                    if key.is_empty() {
                                        println!("Usage: /env delete <KEY>\n");
                                    } else if !env_path.exists() {
                                        println!("❌ {} not found\n", env_filename);
                                    } else {
                                        let content = std::fs::read_to_string(&env_path).unwrap_or_default();
                                        let filtered: Vec<&str> = content
                                            .lines()
                                            .filter(|line| {
                                                let trimmed = line.trim();
                                                if let Some(eq_pos) = trimmed.find('=') {
                                                    trimmed[..eq_pos].trim() != key
                                                } else {
                                                    true
                                                }
                                            })
                                            .collect();
                                        let new_content = filtered.join("\n") + "\n";
                                        match std::fs::write(&env_path, &new_content) {
                                            Ok(_) => println!("🗑️  Deleted {} from {}\n", key, env_filename),
                                            Err(e) => println!("❌ Failed to write {}: {}\n", env_filename, e),
                                        }
                                    }
                                }
                                "switch" => {
                                    let env_name = sub_args.trim();
                                    if env_name.is_empty() {
                                        println!("Usage: /env switch <environment>\n  Current: {} ({})\n", active_env, env_filename);
                                    } else {
                                        let vibeui_dir = cwd.join(".vibeui");
                                        let _ = std::fs::create_dir_all(&vibeui_dir);
                                        let target_file = if env_name == "default" { ".env".to_string() } else { format!(".env.{}", env_name) };
                                        match std::fs::write(vibeui_dir.join("active-env.txt"), env_name) {
                                            Ok(_) => println!("🔄 Switched to environment: {} ({})\n", env_name, target_file),
                                            Err(e) => println!("❌ Failed to switch: {}\n", e),
                                        }
                                    }
                                }
                                "create" => {
                                    let env_name = sub_args.trim().to_lowercase();
                                    if env_name.is_empty() {
                                        println!("Usage: /env create <environment>\n  Example: /env create staging\n");
                                    } else {
                                        let new_file = cwd.join(format!(".env.{}", env_name));
                                        if new_file.exists() {
                                            println!("⚠️  .env.{} already exists\n", env_name);
                                        } else {
                                            match std::fs::write(&new_file, "") {
                                                Ok(_) => {
                                                    #[cfg(unix)]
                                                    {
                                                        use std::os::unix::fs::PermissionsExt;
                                                        let _ = std::fs::set_permissions(&new_file, std::fs::Permissions::from_mode(0o600));
                                                    }
                                                    println!("✅ Created .env.{}\n  Use `/env switch {}` to activate it.\n", env_name, env_name);
                                                }
                                                Err(e) => println!("❌ Failed to create .env.{}: {}\n", env_name, e),
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    println!("Usage: /env [list|get <key>|set <key> <val>|delete <key>|switch <env>|files|create <env>]\n");
                                }
                            }
                        }

                        "/profiler" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                            let cwd = std::env::current_dir().unwrap_or_default();

                            fn detect_prof_tool(cwd: &std::path::Path) -> Option<&'static str> {
                                if cwd.join("Cargo.toml").exists() { Some("cargo-flamegraph") }
                                else if cwd.join("package.json").exists() { Some("clinic") }
                                else if cwd.join("go.mod").exists() { Some("go-pprof") }
                                else if cwd.join("pyproject.toml").exists() || cwd.join("setup.py").exists() { Some("py-spy") }
                                else { None }
                            }

                            fn prof_cli_available(tool: &str) -> bool {
                                std::process::Command::new("sh")
                                    .args(["-c", &format!("command -v {} >/dev/null 2>&1", tool)])
                                    .status()
                                    .map(|s| s.success())
                                    .unwrap_or(false)
                            }

                            match subcmd {
                                "list-tools" => {
                                    println!("\n🔥 Profiling tools:");
                                    let tools = [
                                        ("cargo-flamegraph", "flamegraph", "Rust CPU profiling (perf/dtrace + flamegraph)"),
                                        ("clinic",           "clinic",     "Node.js performance diagnostics"),
                                        ("py-spy",           "py-spy",     "Python sampling profiler"),
                                        ("go-pprof",         "go",         "Go built-in CPU profiling"),
                                    ];
                                    for (name, cli, desc) in &tools {
                                        let mark = if prof_cli_available(cli) { "✅" } else { "❌" };
                                        println!("  {mark} {name:<20} {desc} (requires: {cli})");
                                    }
                                    println!();
                                }
                                "" | "run" => {
                                    let tool = match detect_prof_tool(&cwd) {
                                        Some(t) => t,
                                        None => { println!("❌ No profiler detected. Use `/profiler list-tools`.\n"); continue; }
                                    };
                                    let cli_name = match tool {
                                        "cargo-flamegraph" => "flamegraph",
                                        "clinic" => "clinic",
                                        "py-spy" => "py-spy",
                                        "go-pprof" => "go",
                                        _ => tool,
                                    };
                                    if !prof_cli_available(cli_name) {
                                        println!("❌ {} not found. Install it first.\n", cli_name);
                                        continue;
                                    }
                                    let target_arg = if subcmd == "run" && !sub_args.is_empty() { sub_args } else { "" };

                                    println!("🔥 Profiling with {}...\n", tool);

                                    let cmd_str = match tool {
                                        "cargo-flamegraph" => {
                                            if target_arg.is_empty() {
                                                "cargo flamegraph --output profile.svg 2>&1".to_string()
                                            } else {
                                                format!("cargo flamegraph --output profile.svg -- {} 2>&1", target_arg)
                                            }
                                        }
                                        "go-pprof" => {
                                            "go test -bench=. -benchtime=3s -cpuprofile=cpu.prof ./... 2>&1 && go tool pprof -top cpu.prof 2>&1".to_string()
                                        }
                                        "py-spy" => {
                                            let t = if target_arg.is_empty() { "python -c 'import time; time.sleep(1)'" } else { target_arg };
                                            format!("py-spy record --format speedscope -o profile.json -- {} 2>&1", t)
                                        }
                                        "clinic" => {
                                            let t = if target_arg.is_empty() { "node ." } else { target_arg };
                                            format!("npx clinic doctor -- {} 2>&1", t)
                                        }
                                        _ => { println!("❌ Unknown tool\n"); continue; }
                                    };

                                    let status = std::process::Command::new("sh")
                                        .args(["-c", &cmd_str])
                                        .current_dir(&cwd)
                                        .status();
                                    match status {
                                        Ok(s) if s.success() => {
                                            // For go-pprof, parse the top output
                                            if tool == "go-pprof" {
                                                let pprof_out = std::process::Command::new("go")
                                                    .args(["tool", "pprof", "-top", "cpu.prof"])
                                                    .current_dir(&cwd)
                                                    .output();
                                                if let Ok(out) = pprof_out {
                                                    let text = String::from_utf8_lossy(&out.stdout);
                                                    println!("\n{}", text);
                                                }
                                            }
                                            println!("✅ Profiling complete.\n");
                                        }
                                        Ok(_) => println!("⚠️  Profiler exited with warnings. Check output above.\n"),
                                        Err(e) => println!("❌ Failed to run profiler: {}\n", e),
                                    }
                                }
                                _ => {
                                    println!("Usage: /profiler [run [target]|list-tools]\n");
                                }
                            }
                        }

                        "/deps" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                            let cwd = std::env::current_dir().unwrap_or_default();

                            // Detect package manager
                            let manager = if cwd.join("pnpm-lock.yaml").exists() {
                                "pnpm"
                            } else if cwd.join("yarn.lock").exists() {
                                "yarn"
                            } else if cwd.join("package.json").exists() {
                                "npm"
                            } else if cwd.join("Cargo.toml").exists() {
                                "cargo"
                            } else if cwd.join("go.mod").exists() {
                                "go"
                            } else if cwd.join("requirements.txt").exists()
                                || cwd.join("pyproject.toml").exists()
                                || cwd.join("setup.py").exists()
                            {
                                "pip"
                            } else {
                                println!("❌ No package manager detected in current directory.\n");
                                continue;
                            };

                            match subcmd {
                                "" | "scan" | "outdated" | "vulnerable" | "list" => {
                                    println!("📦 Scanning dependencies ({})...\n", manager);
                                    let outdated_cmd = match manager {
                                        "npm" => "npm outdated --json",
                                        "yarn" => "yarn outdated --json",
                                        "pnpm" => "pnpm outdated --format json",
                                        "cargo" => "cargo update --dry-run",
                                        "pip" => "pip list --outdated --format json",
                                        "go" => "go list -m -u -json all",
                                        _ => { println!("❌ Unsupported manager: {}\n", manager); continue; }
                                    };

                                    let output = std::process::Command::new("sh")
                                        .args(["-c", outdated_cmd])
                                        .current_dir(&cwd)
                                        .output();

                                    match output {
                                        Ok(o) => {
                                            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                                            let stderr = String::from_utf8_lossy(&o.stderr).to_string();

                                            // Parse results based on manager
                                            let mut deps: Vec<(String, String, String, bool)> = Vec::new(); // (name, current, latest, is_outdated)

                                            match manager {
                                                "npm" | "pnpm" => {
                                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stdout) {
                                                        if let Some(obj) = val.as_object() {
                                                            for (name, info) in obj {
                                                                let current = info.get("current").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                                                                let latest = info.get("latest").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                                                                let outdated = current != latest;
                                                                deps.push((name.clone(), current, latest, outdated));
                                                            }
                                                        }
                                                    }
                                                }
                                                "pip" => {
                                                    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                                                        for item in &arr {
                                                            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                                                            let current = item.get("version").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                                                            let latest = item.get("latest_version").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                                                            deps.push((name, current, latest, true));
                                                        }
                                                    }
                                                }
                                                "go" => {
                                                    for line in stdout.lines() {
                                                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                                                            let path = val.get("Path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                            let version = val.get("Version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                            if let Some(update) = val.get("Update") {
                                                                let new_ver = update.get("Version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                                if !new_ver.is_empty() && new_ver != version {
                                                                    deps.push((path, version, new_ver, true));
                                                                }
                                                            } else if subcmd == "list" || subcmd.is_empty() {
                                                                deps.push((path, version.clone(), version, false));
                                                            }
                                                        }
                                                    }
                                                }
                                                "cargo" => {
                                                    // Parse cargo update --dry-run text output
                                                    let text = if !stderr.is_empty() { &stderr } else { &stdout };
                                                    for line in text.lines() {
                                                        if line.contains("Updating") && line.contains("->") {
                                                            let parts: Vec<&str> = line.split_whitespace().collect();
                                                            if parts.len() >= 5 {
                                                                let name = parts[1].to_string();
                                                                let current = parts[2].trim_start_matches('v').to_string();
                                                                let latest = parts[4].trim_start_matches('v').to_string();
                                                                deps.push((name, current, latest, true));
                                                            }
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }

                                            // Filter based on subcommand
                                            let show_deps: Vec<&(String, String, String, bool)> = match subcmd {
                                                "outdated" => deps.iter().filter(|d| d.3).collect(),
                                                _ => deps.iter().collect(),
                                            };

                                            if show_deps.is_empty() {
                                                println!("  ✅ All dependencies are up to date!\n");
                                            } else {
                                                // Print table header
                                                println!("  {:<30} {:>12} {:>12}   Status", "Package", "Current", "Latest");
                                                println!("  {}", "-".repeat(72));
                                                let mut outdated_count = 0;
                                                for (name, current, latest, is_outdated) in &show_deps {
                                                    let status = if *is_outdated { "⬆ outdated" } else { "✓" };
                                                    if *is_outdated { outdated_count += 1; }
                                                    let display_name = if name.len() > 28 { &name[..28] } else { name };
                                                    println!("  {:<30} {:>12} {:>12}   {}", display_name, current, latest, status);
                                                }
                                                println!("\n  Total: {} | Outdated: {}\n", show_deps.len(), outdated_count);
                                            }
                                        }
                                        Err(e) => {
                                            println!("❌ Failed to run {}: {}\n", outdated_cmd, e);
                                        }
                                    }
                                }
                                "upgrade" => {
                                    if sub_args.is_empty() {
                                        println!("Usage: /deps upgrade <package>\n");
                                        continue;
                                    }
                                    let pkg = sub_args.split_whitespace().next().unwrap_or("");
                                    // Validate package name
                                    if pkg.chars().any(|c| ";|&$`\\\"'(){}[]<>!".contains(c)) {
                                        println!("❌ Invalid package name.\n");
                                        continue;
                                    }
                                    let upgrade_cmd = match manager {
                                        "npm" => format!("npm install {}@latest", pkg),
                                        "yarn" => format!("yarn upgrade {}@latest", pkg),
                                        "pnpm" => format!("pnpm update {}@latest", pkg),
                                        "cargo" => format!("cargo update -p {}", pkg),
                                        "pip" => format!("pip install --upgrade {}", pkg),
                                        "go" => format!("go get {}@latest", pkg),
                                        _ => { println!("❌ Unsupported manager.\n"); continue; }
                                    };
                                    println!("📦 Upgrading {} ({})...", pkg, manager);
                                    let status = std::process::Command::new("sh")
                                        .args(["-c", &upgrade_cmd])
                                        .current_dir(&cwd)
                                        .status();
                                    match status {
                                        Ok(s) if s.success() => println!("✅ {} upgraded successfully.\n", pkg),
                                        Ok(_) => println!("⚠️  Upgrade completed with warnings. Check output above.\n"),
                                        Err(e) => println!("❌ Failed to upgrade {}: {}\n", pkg, e),
                                    }
                                }
                                _ => {
                                    println!("Usage: /deps [scan|outdated|vulnerable|upgrade <pkg>|list]\n");
                                }
                            }
                        }

                        "/logs" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                            let cwd = std::env::current_dir().unwrap_or_default();

                            match subcmd {
                                "" | "sources" => {
                                    println!("📋 Scanning for log files...\n");
                                    let mut found: Vec<(String, String)> = Vec::new();
                                    for entry in walkdir::WalkDir::new(&cwd)
                                        .max_depth(4)
                                        .follow_links(false)
                                        .into_iter()
                                        .filter_entry(|e| {
                                            let n = e.file_name().to_string_lossy();
                                            !n.starts_with('.') && n != "node_modules" && n != "target" && n != "__pycache__"
                                        })
                                        .filter_map(|e| e.ok())
                                    {
                                        if found.len() >= 30 { break; }
                                        let path = entry.path();
                                        if path.is_file() {
                                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                                            if name.ends_with(".log") {
                                                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                                                let size_str = if size >= 1_048_576 {
                                                    format!("{:.1} MB", size as f64 / 1_048_576.0)
                                                } else if size >= 1_024 {
                                                    format!("{:.1} KB", size as f64 / 1_024.0)
                                                } else {
                                                    format!("{} B", size)
                                                };
                                                found.push((path.display().to_string(), size_str));
                                            }
                                        }
                                    }
                                    if found.is_empty() {
                                        println!("  No log files found.\n");
                                    } else {
                                        println!("  {:<50} {:>10}", "File", "Size");
                                        println!("  {}", "-".repeat(62));
                                        for (path, size) in &found {
                                            let display = if path.len() > 48 { &path[path.len()-48..] } else { path.as_str() };
                                            println!("  {:<50} {:>10}", display, size);
                                        }
                                        println!("\n  Found {} log file(s). Use `/logs tail <path>` to view.\n", found.len());
                                    }
                                }
                                "tail" => {
                                    if sub_args.is_empty() {
                                        println!("Usage: /logs tail <file_path>\n");
                                        continue;
                                    }
                                    let file_path = std::path::Path::new(sub_args);
                                    let abs_path = if file_path.is_absolute() { file_path.to_path_buf() } else { cwd.join(file_path) };
                                    match std::fs::read_to_string(&abs_path) {
                                        Ok(content) => {
                                            let lines: Vec<&str> = content.lines().collect();
                                            let skip = if lines.len() > 50 { lines.len() - 50 } else { 0 };
                                            println!("📋 Last {} lines of {}:\n", lines.len().min(50), sub_args);
                                            for line in &lines[skip..] {
                                                let upper = line.to_uppercase();
                                                if upper.contains("ERROR") || upper.contains("FATAL") {
                                                    println!("  \x1b[31m{}\x1b[0m", line);
                                                } else if upper.contains("WARN") {
                                                    println!("  \x1b[33m{}\x1b[0m", line);
                                                } else {
                                                    println!("  {}", line);
                                                }
                                            }
                                            println!();
                                        }
                                        Err(e) => println!("❌ Failed to read {}: {}\n", sub_args, e),
                                    }
                                }
                                "errors" => {
                                    if sub_args.is_empty() {
                                        println!("Usage: /logs errors <file_path>\n");
                                        continue;
                                    }
                                    let file_path = std::path::Path::new(sub_args);
                                    let abs_path = if file_path.is_absolute() { file_path.to_path_buf() } else { cwd.join(file_path) };
                                    match std::fs::read_to_string(&abs_path) {
                                        Ok(content) => {
                                            let mut error_count = 0usize;
                                            let mut warn_count = 0usize;
                                            println!("📋 Errors/warnings in {}:\n", sub_args);
                                            for line in content.lines() {
                                                let upper = line.to_uppercase();
                                                if upper.contains("ERROR") || upper.contains("FATAL") || upper.contains("PANIC") {
                                                    println!("  \x1b[31m{}\x1b[0m", line);
                                                    error_count += 1;
                                                } else if upper.contains("WARN") {
                                                    println!("  \x1b[33m{}\x1b[0m", line);
                                                    warn_count += 1;
                                                }
                                            }
                                            if error_count == 0 && warn_count == 0 {
                                                println!("  ✅ No errors or warnings found.\n");
                                            } else {
                                                println!("\n  Errors: {} | Warnings: {}\n", error_count, warn_count);
                                            }
                                        }
                                        Err(e) => println!("❌ Failed to read {}: {}\n", sub_args, e),
                                    }
                                }
                                "analyze" => {
                                    if sub_args.is_empty() {
                                        println!("Usage: /logs analyze <file_path>\n");
                                        continue;
                                    }
                                    let file_path = std::path::Path::new(sub_args);
                                    let abs_path = if file_path.is_absolute() { file_path.to_path_buf() } else { cwd.join(file_path) };
                                    match std::fs::read_to_string(&abs_path) {
                                        Ok(content) => {
                                            let lines: Vec<&str> = content.lines().collect();
                                            let tail: Vec<&str> = lines.iter().rev().take(100).copied().collect::<Vec<_>>().into_iter().rev().collect();
                                            let log_text = tail.join("\n");
                                            println!("📋 Analyzing last {} lines with AI...\n", tail.len());
                                            let prompt = format!(
                                                "Analyze these log entries. Identify errors, recurring patterns, probable root causes, and suggest fixes.\n\n```\n{}\n```",
                                                log_text
                                            );
                                            let msgs = vec![vibe_ai::provider::Message {
                                                role: vibe_ai::provider::MessageRole::User,
                                                content: prompt,
                                            }];
                                            match llm.chat(&msgs, None).await {
                                                Ok(response) => println!("{}\n", response),
                                                Err(e) => println!("❌ AI analysis failed: {}\n", e),
                                            }
                                        }
                                        Err(e) => println!("❌ Failed to read {}: {}\n", sub_args, e),
                                    }
                                }
                                _ => {
                                    println!("Usage: /logs [tail <file>|sources|errors <file>|analyze <file>]\n");
                                }
                            }
                        }

                        "/notebook" => {
                            let file_arg = args.trim();
                            if file_arg.is_empty() {
                                println!("Usage: /notebook <file.vibe>\n");
                                println!("  Run a .vibe notebook file with executable code cells.\n");
                                println!("  Example: /notebook demo.vibe\n");
                                continue;
                            }
                            let cwd = std::env::current_dir().unwrap_or_default();
                            let path = if std::path::Path::new(file_arg).is_absolute() {
                                std::path::PathBuf::from(file_arg)
                            } else {
                                cwd.join(file_arg)
                            };
                            if !path.exists() {
                                println!("❌ File not found: {}\n", path.display());
                                continue;
                            }
                            println!("📓 Running notebook: {}\n", path.display());
                            match notebook::run_notebook(&path, false) {
                                Ok(success) => {
                                    if success {
                                        println!("\n✅ All cells passed.\n");
                                    } else {
                                        println!("\n⚠️  Some cells failed.\n");
                                    }
                                }
                                Err(e) => println!("❌ Notebook error: {}\n", e),
                            }
                        }

                        "/migration" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                            let cwd = std::env::current_dir().unwrap_or_default();

                            // Detect migration tool
                            let tool = if cwd.join("prisma").join("schema.prisma").exists()
                                || cwd.join("schema.prisma").exists()
                            {
                                "prisma"
                            } else if cwd.join("diesel.toml").exists() {
                                "diesel"
                            } else if cwd.join("alembic.ini").exists() {
                                "alembic"
                            } else if cwd.join("flyway.conf").exists() {
                                "flyway"
                            } else if cwd.join("go.mod").exists() && cwd.join("migrations").exists() {
                                "golang-migrate"
                            } else {
                                println!("❌ No migration tool detected. Supported: Prisma, Diesel, Alembic, Flyway, golang-migrate.\n");
                                continue;
                            };

                            match subcmd {
                                "" | "status" => {
                                    println!("🔷 Migration status ({})...\n", tool);
                                    let status_cmd = match tool {
                                        "prisma" => "npx prisma migrate status",
                                        "diesel" => "diesel migration list",
                                        "alembic" => "alembic current",
                                        "flyway" => "flyway info",
                                        "golang-migrate" => "migrate -path migrations version",
                                        _ => { println!("❌ Unsupported tool.\n"); continue; }
                                    };
                                    let output = std::process::Command::new("sh")
                                        .args(["-c", status_cmd])
                                        .current_dir(&cwd)
                                        .output();
                                    match output {
                                        Ok(o) => {
                                            let stdout = String::from_utf8_lossy(&o.stdout);
                                            let stderr = String::from_utf8_lossy(&o.stderr);
                                            if !stdout.trim().is_empty() { println!("{}", stdout); }
                                            if !stderr.trim().is_empty() { println!("{}", stderr); }
                                            if stdout.trim().is_empty() && stderr.trim().is_empty() {
                                                println!("  (no output)\n");
                                            }
                                        }
                                        Err(e) => println!("❌ Failed to run {}: {}\n", status_cmd, e),
                                    }
                                }
                                "migrate" => {
                                    println!("🔷 Running migrations ({})...\n", tool);
                                    let migrate_cmd = match tool {
                                        "prisma" => "npx prisma migrate deploy",
                                        "diesel" => "diesel migration run",
                                        "alembic" => "alembic upgrade head",
                                        "flyway" => "flyway migrate",
                                        "golang-migrate" => "migrate -path migrations -database $DATABASE_URL up",
                                        _ => { println!("❌ Unsupported tool.\n"); continue; }
                                    };
                                    let status = std::process::Command::new("sh")
                                        .args(["-c", migrate_cmd])
                                        .current_dir(&cwd)
                                        .status();
                                    match status {
                                        Ok(s) if s.success() => println!("✅ Migrations applied successfully.\n"),
                                        Ok(_) => println!("⚠️  Migration completed with warnings. Check output above.\n"),
                                        Err(e) => println!("❌ Failed to run migrations: {}\n", e),
                                    }
                                }
                                "rollback" => {
                                    println!("🔷 Rolling back last migration ({})...\n", tool);
                                    let rollback_cmd = match tool {
                                        "prisma" => "npx prisma migrate reset --skip-seed",
                                        "diesel" => "diesel migration revert",
                                        "alembic" => "alembic downgrade -1",
                                        "flyway" => "flyway undo",
                                        "golang-migrate" => "migrate -path migrations -database $DATABASE_URL down 1",
                                        _ => { println!("❌ Unsupported tool.\n"); continue; }
                                    };
                                    let status = std::process::Command::new("sh")
                                        .args(["-c", rollback_cmd])
                                        .current_dir(&cwd)
                                        .status();
                                    match status {
                                        Ok(s) if s.success() => println!("✅ Rollback completed.\n"),
                                        Ok(_) => println!("⚠️  Rollback completed with warnings.\n"),
                                        Err(e) => println!("❌ Failed to rollback: {}\n", e),
                                    }
                                }
                                "generate" => {
                                    if sub_args.is_empty() {
                                        println!("Usage: /migration generate <name>\n");
                                        continue;
                                    }
                                    let name = sub_args.split_whitespace().next().unwrap_or("");
                                    // Validate name (alphanumeric + underscores/hyphens)
                                    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                                        println!("❌ Invalid migration name. Use alphanumeric, hyphens, and underscores only.\n");
                                        continue;
                                    }
                                    println!("🔷 Generating migration '{}' ({})...\n", name, tool);
                                    let gen_cmd = match tool {
                                        "prisma" => format!("npx prisma migrate dev --name {}", name),
                                        "diesel" => format!("diesel migration generate {}", name),
                                        "alembic" => format!("alembic revision -m \"{}\"", name),
                                        "flyway" => {
                                            println!("  Flyway migrations are created manually. Create a new file:\n  sql/V<version>__{}.sql\n", name);
                                            continue;
                                        }
                                        "golang-migrate" => format!("migrate create -ext sql -dir migrations -seq {}", name),
                                        _ => { println!("❌ Unsupported tool.\n"); continue; }
                                    };
                                    let status = std::process::Command::new("sh")
                                        .args(["-c", &gen_cmd])
                                        .current_dir(&cwd)
                                        .status();
                                    match status {
                                        Ok(s) if s.success() => println!("✅ Migration '{}' generated.\n", name),
                                        Ok(_) => println!("⚠️  Generation completed with warnings.\n"),
                                        Err(e) => println!("❌ Failed to generate migration: {}\n", e),
                                    }
                                }
                                _ => {
                                    println!("Usage: /migration [status|migrate|rollback|generate <name>]\n");
                                }
                            }
                        }

                        _ => {
                            println!("Type /help for available commands\n");
                        }
                    }
                } else {
                    // Regular chat (with @ context expansion and streaming)
                    if !conversation_active {
                        messages.clear();
                        conversation_active = true;
                        messages.push(Message {
                            role: MessageRole::System,
                            content: "You are a helpful coding assistant. If the user asks you to run a command, output it in a ```execute block.\n\nContext references (@file:, @web:, @docs:, @git) are automatically expanded before each message.".to_string(),
                        });
                    }
                    // Expand @file:, @web:, @docs:, @git references
                    let expanded = expand_at_refs(input).await;
                    messages.push(Message {
                        role: MessageRole::User,
                        content: expanded,
                    });
                    print!("🤖 ");
                    io::stdout().flush()?;
                    // Stream the response token by token
                    match llm.stream_chat(&messages).await {
                        Ok(mut stream) => {
                            use futures::StreamExt;
                            let mut full_response = String::new();
                            while let Some(chunk) = stream.next().await {
                                match chunk {
                                    Ok(text) => {
                                        print!("{}", text);
                                        io::stdout().flush().ok();
                                        full_response.push_str(&text);
                                    }
                                    Err(e) => {
                                        eprintln!("\n❌ Stream error: {:#}", e);
                                        break;
                                    }
                                }
                            }
                            println!("\n");
                            if !full_response.is_empty() {
                                messages.push(Message {
                                    role: MessageRole::Assistant,
                                    content: full_response,
                                });
                            }
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
    provider: Arc<dyn LLMProvider>,
}

impl ExecutorFactory for VibeExecutorFactory {
    fn create(&self, workspace_root: std::path::PathBuf) -> Arc<dyn vibe_ai::agent::ToolExecutorTrait> {
        Arc::new(
            ToolExecutor::new(workspace_root, self.sandbox)
                .with_env_policy(self.env_policy.clone())
                .with_provider(self.provider.clone()),
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

    let factory = Arc::new(VibeExecutorFactory { sandbox, env_policy, provider: llm.clone() });
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

#[allow(clippy::too_many_arguments)]
async fn run_agent_repl_with_context(
    llm: Arc<dyn LLMProvider>,
    task: &str,
    approval_policy: &str,
    resume_session_id: Option<&str>,
    plan_mode: bool,
    json_output: bool,
    planning_llm: Option<Arc<dyn LLMProvider>>,
    provider_name: &str,
    model_name: &str,
) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let config = Config::load().unwrap_or_default();
    let approval = ApprovalPolicy::from_str(approval_policy);
    let sandbox = config.safety.sandbox;

    // Apply shell env policy and search engine config
    let env_policy = config.safety.shell_environment.to_policy();
    let search_cfg = &config.tools.web_search;
    let executor: Arc<dyn vibe_ai::agent::ToolExecutorTrait> =
        Arc::new(ToolExecutor::new(workspace.clone(), sandbox)
            .with_env_policy(env_policy)
            .with_search_config(
                search_cfg.engine.clone(),
                search_cfg.resolve_tavily_key(),
                search_cfg.resolve_brave_key(),
            )
            .with_provider(llm.clone()));

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
        // 1. Try JSONL traces first (fastest, preserves full message objects)
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
                    // JSONL trace exists but no messages sidecar — try SQLite
                    println!("⚠️  No JSONL messages for session — trying SQLite history …");
                    if let Ok(store) = SessionStore::open_default() {
                        let full_id = session.session_id.clone();
                        match store.get_messages(&full_id) {
                            Ok(rows) if !rows.is_empty() => {
                                let msgs: Vec<Message> = rows.into_iter()
                                    .filter_map(|r| {
                                        let role = match r.role.as_str() {
                                            "user"      => Some(MessageRole::User),
                                            "assistant" => Some(MessageRole::Assistant),
                                            "system"    => Some(MessageRole::System),
                                            _           => None,
                                        };
                                        role.map(|role| Message { role, content: r.content })
                                    })
                                    .collect();
                                println!("▶️  Restored {} messages from SQLite for session {}",
                                    msgs.len(), &full_id[..8.min(full_id.len())]);
                                msgs
                            }
                            _ => {
                                println!("⚠️  Session found but no saved messages — starting fresh");
                                vec![]
                            }
                        }
                    } else {
                        println!("⚠️  Session found but no saved messages — starting fresh");
                        vec![]
                    }
                }
            }
        } else {
            // 2. No JSONL trace — fall back to pure SQLite lookup
            match SessionStore::open_default() {
                Ok(store) => {
                    // Find session by ID prefix
                    let all = store.list_root_sessions(50).unwrap_or_default();
                    if let Some(row) = all.iter().find(|r| r.id.starts_with(sid_prefix)) {
                        match store.get_messages(&row.id) {
                            Ok(msgs) if !msgs.is_empty() => {
                                let messages: Vec<Message> = msgs.into_iter()
                                    .filter_map(|r| {
                                        let role = match r.role.as_str() {
                                            "user"      => Some(MessageRole::User),
                                            "assistant" => Some(MessageRole::Assistant),
                                            "system"    => Some(MessageRole::System),
                                            _           => None,
                                        };
                                        role.map(|role| Message { role, content: r.content })
                                    })
                                    .collect();
                                println!("▶️  Restored {} messages from SQLite for session {}",
                                    messages.len(), &row.id[..row.id.len().min(10)]);
                                messages
                            }
                            _ => {
                                eprintln!("❌ Session '{}' found in SQLite but has no messages.", sid_prefix);
                                return Ok(());
                            }
                        }
                    } else {
                        eprintln!("❌ No session found with ID prefix: {}", sid_prefix);
                        return Ok(());
                    }
                }
                Err(_) => {
                    eprintln!("❌ No session found with ID prefix: {}", sid_prefix);
                    return Ok(());
                }
            }
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

    // Open SQLite session store (non-fatal if unavailable)
    let db = SessionStore::open_default().ok();
    let session_id = trace.session_id().to_string();
    if let Some(ref store) = db {
        let _ = store.insert_session(&session_id, task, provider_name, model_name);
        let _ = store.insert_message(&session_id, "user", task);
    }

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
                if let Some(ref store) = db {
                    let _ = store.insert_step(
                        &session_id,
                        step.step_num,
                        step.tool_call.name(),
                        &step.tool_call.summary(),
                        &step.tool_result.output,
                        step.tool_result.success,
                    );
                }
            }
            AgentEvent::Complete(summary) => {
                println!("\n\n✅ Agent complete: {}", summary);
                println!("   Trace saved: {}", trace.path().display());
                println!("   Resume with: vibecli --resume {}", trace.session_id());
                if let Some(ref store) = db {
                    let _ = store.insert_message(&session_id, "assistant", &summary);
                    let _ = store.finish_session(&session_id, "complete", Some(&summary));
                }
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
                if let Some(ref store) = db {
                    let _ = store.finish_session(&session_id, "failed", Some(&e));
                }
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
    let re = re_image_attachment();
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
    use vibe_ai::providers::{claude, openai, gemini, grok, groq, openrouter, azure_openai, bedrock, copilot};

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

        // ── AWS Bedrock ───────────────────────────────────────────────────────
        "bedrock" | "aws" | "aws-bedrock" => {
            let region = std::env::var("AWS_REGION")
                .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                .ok();
            let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
            if std::env::var("AWS_ACCESS_KEY_ID").is_err() {
                eprintln!("⚠️  AWS_ACCESS_KEY_ID not set");
            }
            if secret_key.is_none() {
                eprintln!("⚠️  AWS_SECRET_ACCESS_KEY not set");
            }
            let model = model.unwrap_or_else(|| "anthropic.claude-3-sonnet-20240229-v1:0".to_string());
            Ok(Arc::new(bedrock::BedrockProvider::new(ProviderConfig {
                provider_type: "bedrock".to_string(),
                api_url: region, // reuse api_url field for region
                model,
                api_key: secret_key,
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }

        // ── GitHub Copilot ────────────────────────────────────────────────────
        "copilot" | "github-copilot" => {
            let api_key = std::env::var("GITHUB_TOKEN").ok();
            if api_key.is_none() {
                eprintln!("⚠️  GITHUB_TOKEN not set (required for GitHub Copilot)");
                eprintln!("   Run: vibecli --copilot-login  to authenticate via device flow");
            }
            let model = model.unwrap_or_else(|| "gpt-4o".to_string());
            Ok(Arc::new(copilot::CopilotProvider::new(ProviderConfig {
                provider_type: "copilot".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }

        _ => anyhow::bail!(
            "Unknown provider: '{}'. Available: ollama, claude, openai, gemini, grok, groq, openrouter, azure, bedrock, copilot",
            provider_name
        ),
    }
}

/// Validate a user-supplied name used to build file paths (snippets, etc.).
/// Rejects anything containing path separators or parent-directory components.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
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
    println!("  /workflow                - Code Complete workflow (new|list|show|advance|check|generate)");
    println!("  /agents                  - Background agents (list|status|new)");
    println!("  /team                    - Team knowledge store (show|knowledge|sync)");
    println!("  /remind in <dur> \"task\"  - Set a one-time reminder (30s, 10m, 2h, 1d)");
    println!("  /remind list             - List active reminders");
    println!("  /remind cancel <id>      - Cancel a reminder");
    println!("  /schedule every <dur> \"t\"- Set a recurring task (every 10m, 1h, 1d)");
    println!("  /schedule list           - List scheduled jobs");
    println!("  /schedule cancel <id>    - Cancel a scheduled job");
    println!("  /linear list             - List Linear issues assigned to you");
    println!("  /linear new \"title\"      - Create a new Linear issue");
    println!("  /linear open <id>        - Open a Linear issue in the browser");
    println!("  /linear attach <id>      - Link current session to a Linear issue");
    println!("  /snippet list            - List saved code snippets");
    println!("  /snippet save <name>     - Save last AI response as a named snippet");
    println!("  /snippet use <name>      - Inject snippet as context for next message");
    println!("  /snippet show <name>     - Display snippet contents");
    println!("  /snippet delete <name>   - Delete a saved snippet");
    println!("  /jobs                    - List persisted background jobs (~/.vibecli/jobs/)");
    println!("  /jobs <session_id>       - Show full detail for a specific job");
    println!("  /config                  - Show current configuration");
    println!("  /help                    - Show this help message");
    println!("  /exit                    - Exit VibeCLI");
    println!("  ! <command>              - Execute shell command directly (e.g. !ls)");
    println!("\n@ context references (in any message):");
    println!("  @file:<path>             - Inject file contents as context");
    println!("  @file:<path>:<N-M>       - Inject specific line range");
    println!("  @web:<url>               - Fetch and inject web page content");
    println!("  @docs:<pkg>              - Fetch library docs (e.g. @docs:tokio, @docs:py:requests)");
    println!("  @git                     - Inject git status and recent commits");
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
    println!("  --gateway <platform>     - Start messaging bot (telegram|discord|slack)");
    println!("  --worktree               - Run agent in isolated git worktree branch");
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

// ── Watch Mode ────────────────────────────────────────────────────────────────

/// Watch the CWD for changes and run an agent task on each change.
async fn run_watch_mode(
    llm: Arc<dyn LLMProvider>,
    task_template: &str,
    approval_policy: &str,
    watch_glob: &str,
    sandbox: bool,
) -> Result<()> {
    use notify::{Event, EventKind, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::Duration as StdDuration;

    let cwd = std::env::current_dir()?;
    let glob_pattern = watch_glob.to_string();
    let task_template = task_template.to_string();

    eprintln!("👁  Watching {} for changes (glob: {})…", cwd.display(), glob_pattern);
    let task_end = task_template.char_indices().nth(80).map(|(i,_)| i).unwrap_or(task_template.len());
    eprintln!("   Task on change: {}", &task_template[..task_end]);
    eprintln!("   Press Ctrl+C to stop.\n");

    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();

    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    watcher.watch(&cwd, RecursiveMode::Recursive)?;

    // Debounce: wait 500ms after last event before triggering
    let debounce = StdDuration::from_millis(500);
    let mut last_trigger = std::time::Instant::now()
        .checked_sub(StdDuration::from_secs(10))
        .unwrap_or_else(std::time::Instant::now);

    loop {
        // Collect all pending events
        let mut changed_paths: Vec<String> = Vec::new();
        loop {
            match rx.recv_timeout(StdDuration::from_millis(100)) {
                Ok(Ok(event)) => {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)) {
                        for path in &event.paths {
                            let rel = path.strip_prefix(&cwd).unwrap_or(path);
                            let rel_str = rel.to_string_lossy().to_string();
                            // Skip hidden dirs and target/
                            if rel_str.starts_with('.') || rel_str.contains("/target/") || rel_str.contains("/node_modules/") {
                                continue;
                            }
                            // Apply glob filter (simple: check extension match)
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            let exts = glob_pattern.trim_matches(|c| c == '{' || c == '}')
                                .split(',')
                                .filter_map(|p| p.rsplit('.').next().map(|s| s.to_string()))
                                .collect::<Vec<_>>();
                            if exts.is_empty() || exts.iter().any(|e| e == ext || e == "*") {
                                changed_paths.push(rel_str);
                            }
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("[watch] Error: {}", e),
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }

        if !changed_paths.is_empty() && last_trigger.elapsed() >= debounce {
            last_trigger = std::time::Instant::now();
            changed_paths.dedup();
            let files_list = changed_paths.join(", ");
            let task = format!("{}\n\nChanged files: {}", task_template, files_list);

            eprintln!("\n🔄 Change detected: {}", files_list);
            eprintln!("   Running agent task…\n");

            let workspace_root = cwd.clone();
            let executor: Arc<dyn vibe_ai::agent::ToolExecutorTrait> =
                Arc::new(ToolExecutor::new(workspace_root.clone(), sandbox)
                    .with_provider(llm.clone()));

            let approval = ApprovalPolicy::from_str(approval_policy);
            let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(64);
            let agent = AgentLoop::new(llm.clone(), approval, executor);

            let task_clone = task.clone();
            let ctx = vibe_ai::AgentContext {
                workspace_root: workspace_root.clone(),
                open_files: vec![],
                git_branch: None,
                git_diff_summary: None,
                flow_context: None,
                approved_plan: None,
                extra_skill_dirs: vec![],
                parent_session_id: None,
                depth: 0,
                active_agent_counter: None,
            };
            tokio::spawn(async move {
                let _ = agent.run(&task_clone, ctx, event_tx).await;
            });

            while let Some(event) = event_rx.recv().await {
                match event {
                    AgentEvent::StreamChunk(text) => {
                        print!("{}", text);
                    }
                    AgentEvent::ToolCallExecuted(step) => {
                        eprintln!("  🔧 {} → {}", step.tool_call.name(), if step.tool_result.success { "✓" } else { "✗" });
                    }
                    AgentEvent::Complete(summary) => {
                        eprintln!("\n✅ Agent complete: {}\n", summary);
                        break;
                    }
                    AgentEvent::Error(e) => {
                        eprintln!("\n❌ Agent error: {}\n", e);
                        break;
                    }
                    _ => {}
                }
            }
        }

        // Small yield to prevent busy-loop
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

// ── Lazy-compiled regex patterns for @-ref expansion ─────────────────────────

fn re_at_file() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@file:(\S+)").unwrap())
}
fn re_at_web() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@web:(\S+)").unwrap())
}
fn re_at_docs() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@docs:(\S+)").unwrap())
}
fn re_at_symbol() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@symbol:(\S+)").unwrap())
}
fn re_at_codebase() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@codebase:(\S+)").unwrap())
}
fn re_at_github() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@github:([a-zA-Z0-9_\-]+)/([a-zA-Z0-9_\-]+)#(\d+)").unwrap())
}
fn re_at_jira() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@jira:([A-Z][A-Z0-9_]+-\d+)").unwrap())
}
fn re_image_attachment() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"\[([^\]]+\.(png|jpg|jpeg|gif|webp))\]").unwrap())
}
fn re_html_tags() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"<[^>]+>").unwrap())
}
fn re_collapse_whitespace() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"\s{2,}").unwrap())
}

// ── REPL @ context expansion ──────────────────────────────────────────────────
//
// Resolves `@file:path`, `@web:url`, `@docs:name`, and `@git` references in
// a user message, injecting their content as context before the message.
// Returns the expanded message string.

pub async fn expand_at_refs(input: &str) -> String {
    let mut extra = Vec::<String>::new();
    let mut result = input.to_string();

    // ── @file:path ────────────────────────────────────────────────────────────
    for cap in re_at_file().captures_iter(input) {
        let raw_path = &cap[1];
        // Support line-range  @file:path:N-M
        let (file_path, line_range) = if let Some(idx) = raw_path.rfind(':') {
            let candidate = &raw_path[idx + 1..];
            if candidate.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                (&raw_path[..idx], Some(candidate))
            } else {
                (raw_path, None)
            }
        } else {
            (raw_path, None)
        };

        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                let text = if let Some(range) = line_range {
                    let parts: Vec<&str> = range.splitn(2, '-').collect();
                    let start: usize = match parts[0].parse() {
                        Ok(n) if n > 0 => n,
                        _ => {
                            eprintln!("⚠️  Invalid line range '{}', showing full file", range);
                            1
                        }
                    };
                    let end: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(start);
                    content
                        .lines()
                        .enumerate()
                        .filter(|(i, _)| *i + 1 >= start && *i < end)
                        .map(|(_, l)| l)
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    content.chars().take(8000).collect()
                };
                extra.push(format!(
                    "=== File: {} ===\n```\n{}\n```",
                    file_path, text
                ));
            }
            Err(e) => {
                extra.push(format!("(Could not read {}: {})", file_path, e));
            }
        }
        result = result.replacen(&cap[0], "", 1);
    }

    // ── @git ──────────────────────────────────────────────────────────────────
    if result.contains("@git") {
        let stat = std::process::Command::new("git")
            .args(["diff", "--stat", "HEAD"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let log = std::process::Command::new("git")
            .args(["log", "--oneline", "-5"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        extra.push(format!("=== Git status ===\n{}\n=== Recent commits ===\n{}", stat.trim(), log.trim()));
        result = result.replace("@git", "");
    }

    // ── @web:url ──────────────────────────────────────────────────────────────
    for cap in re_at_web().captures_iter(input) {
        let url = &cap[1];
        let text = fetch_and_strip_url(url, 6000).await;
        extra.push(format!("=== Web: {} ===\n{}", url, text));
        result = result.replacen(&cap[0], "", 1);
    }

    // ── @docs:name ────────────────────────────────────────────────────────────
    for cap in re_at_docs().captures_iter(input) {
        let name_raw = &cap[1];
        // Detect registry: rs: → docs.rs, py:/pypi: → PyPI, npm: → npmjs, default → docs.rs
        let (registry, clean_name) = if name_raw.starts_with("rs:") {
            ("docs.rs", name_raw.trim_start_matches("rs:"))
        } else if name_raw.starts_with("py:") || name_raw.starts_with("pypi:") {
            ("pypi", name_raw.split(':').next_back().unwrap_or(name_raw))
        } else if name_raw.starts_with("npm:") {
            ("npm", name_raw.trim_start_matches("npm:"))
        } else {
            ("docs.rs", name_raw)
        };

        let url = match registry {
            "pypi" => format!("https://pypi.org/pypi/{}/json", clean_name),
            "npm"  => format!("https://registry.npmjs.org/{}", clean_name),
            _      => format!("https://docs.rs/{}/latest/{}/index.html", clean_name, clean_name.replace('-', "_")),
        };
        let text = fetch_and_strip_url(&url, 4000).await;
        extra.push(format!("=== Docs: {} ({}) ===\n{}", clean_name, registry, text));
        result = result.replacen(&cap[0], "", 1);
    }

    // ── @symbol:name — search for a symbol across the codebase ───────────────
    for cap in re_at_symbol().captures_iter(input) {
        let sym_name = &cap[1];
        // Quick grep-based symbol search: find function/class/struct/const definitions
        let output = std::process::Command::new("grep")
            .args([
                "-rn",
                "--include=*.rs",
                "--include=*.ts",
                "--include=*.tsx",
                "--include=*.js",
                "--include=*.py",
                "--include=*.go",
                "-E",
                &format!(
                    r"(fn|function|class|struct|enum|const|type|def|interface)\s+{}(\s|[(<{{]|$)",
                    regex::escape(sym_name)
                ),
                ".",
            ])
            .output()
            .ok();
        let text = output
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        if text.trim().is_empty() {
            extra.push(format!("=== Symbol: {} ===\n(not found)", sym_name));
        } else {
            let lines: String = text.lines().take(20).collect::<Vec<_>>().join("\n");
            extra.push(format!("=== Symbol: {} ===\n{}", sym_name, lines));
        }
        result = result.replacen(&cap[0], "", 1);
    }

    // ── @codebase:query — keyword search across the workspace ────────────────
    for cap in re_at_codebase().captures_iter(input) {
        let query = &cap[1];
        let output = std::process::Command::new("grep")
            .args([
                "-rn",
                "--include=*.rs",
                "--include=*.ts",
                "--include=*.tsx",
                "--include=*.js",
                "--include=*.py",
                "--include=*.go",
                "-i",
                query,
                ".",
            ])
            .output()
            .ok();
        let text = output
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        if text.trim().is_empty() {
            extra.push(format!("=== Codebase search: {} ===\n(no matches)", query));
        } else {
            let lines: String = text.lines().take(20).collect::<Vec<_>>().join("\n");
            extra.push(format!("=== Codebase search: {} ===\n{}", query, lines));
        }
        result = result.replacen(&cap[0], "", 1);
    }

    // ── @github:owner/repo#N — inject GitHub issue / PR content ──────────────
    // Collect matches first to avoid mutable/immutable borrow conflict.
    let github_caps: Vec<(String, String, String, String)> = re_at_github()
        .captures_iter(&result.clone())
        .map(|cap| (
            cap[0].to_string(),
            cap[1].to_string(),
            cap[2].to_string(),
            cap[3].to_string(),
        ))
        .collect();
    for (matched, owner, repo, num) in github_caps {
        let api_url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}",
            owner, repo, num
        );
        let text = fetch_github_issue_text(&api_url).await;
        extra.push(format!("=== GitHub Issue: {}/{}#{} ===\n{}", owner, repo, num, text));
        result = result.replacen(&matched, "", 1);
    }

    // ── @jira:PROJECT-123 — inject Jira issue content ────────────────────────
    let jira_caps: Vec<(String, String)> = re_at_jira()
        .captures_iter(&result.clone())
        .map(|cap| (cap[0].to_string(), cap[1].to_string()))
        .collect();
    if !jira_caps.is_empty() {
        let base_url = std::env::var("JIRA_BASE_URL").unwrap_or_default();
        let token    = std::env::var("JIRA_API_TOKEN").unwrap_or_default();
        let email    = std::env::var("JIRA_EMAIL").unwrap_or_default();
        for (matched, issue_key) in jira_caps {
            let text = if base_url.is_empty() {
                "(set JIRA_BASE_URL, JIRA_EMAIL, JIRA_API_TOKEN to fetch Jira issues)".to_string()
            } else {
                let api_url = format!("{}/rest/api/2/issue/{}", base_url.trim_end_matches('/'), issue_key);
                fetch_jira_issue_text(&api_url, &email, &token).await
            };
            extra.push(format!("=== Jira Issue: {} ===\n{}", issue_key, text));
            result = result.replacen(&matched, "", 1);
        }
    }

    // ── Assemble ──────────────────────────────────────────────────────────────
    let result = result.trim().to_string();
    if extra.is_empty() {
        return result;
    }
    format!("{}\n\n{}", extra.join("\n\n"), result)
}

/// Fetch a Jira issue via the REST API v2 and return a plain-text summary.
async fn fetch_jira_issue_text(api_url: &str, email: &str, token: &str) -> String {
    let client = match reqwest::Client::builder()
        .user_agent("VibeCLI/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("(could not build HTTP client: {})", e),
    };
    let mut req = client.get(api_url).header("Accept", "application/json");
    if !email.is_empty() && !token.is_empty() {
        req = req.basic_auth(email, Some(token));
    }
    let body = match req.send().await {
        Ok(resp) => match resp.text().await {
            Ok(b) => b,
            Err(e) => return format!("(Jira response read error: {})", e),
        },
        Err(e) => return format!("(Jira fetch error: {})", e),
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
        let summary  = v["fields"]["summary"].as_str().unwrap_or("(no summary)");
        let status   = v["fields"]["status"]["name"].as_str().unwrap_or("unknown");
        let assignee = v["fields"]["assignee"]["displayName"].as_str().unwrap_or("unassigned");
        let desc_raw = v["fields"]["description"].as_str().unwrap_or("");
        let snippet: String = desc_raw.lines().take(20).collect::<Vec<_>>().join("\n");
        format!(
            "Summary: {}\nStatus: {} | Assignee: {}\n\n{}",
            summary, status, assignee,
            if snippet.is_empty() { "(no description)".to_string() } else { snippet },
        )
    } else {
        body.chars().take(3000).collect()
    }
}

/// Fetch a GitHub issue/PR JSON from the API and return a plain-text summary.
async fn fetch_github_issue_text(api_url: &str) -> String {
    let client = match reqwest::Client::builder()
        .user_agent("VibeCLI/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("(could not build HTTP client: {})", e),
    };
    let mut req = client
        .get(api_url)
        .header("Accept", "application/vnd.github.v3+json");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    let body = match req.send().await {
        Ok(resp) => match resp.text().await {
            Ok(b) => b,
            Err(e) => return format!("(GitHub response read error: {})", e),
        },
        Err(e) => return format!("(GitHub fetch error: {})", e),
    };
    // Parse minimal summary from GitHub issue JSON.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
        let title  = v["title"].as_str().unwrap_or("(no title)");
        let state  = v["state"].as_str().unwrap_or("unknown");
        let author = v["user"]["login"].as_str().unwrap_or("unknown");
        let labels: Vec<&str> = v["labels"]
            .as_array()
            .map(|a| a.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();
        let body_text = v["body"].as_str().unwrap_or("").to_string();
        let snippet: String = body_text.lines().take(20).collect::<Vec<_>>().join("\n");
        format!(
            "Title: {}\nState: {} | Author: {} | Labels: {}\n\n{}",
            title,
            state,
            author,
            if labels.is_empty() { "none".to_string() } else { labels.join(", ") },
            snippet,
        )
    } else {
        body.chars().take(3000).collect()
    }
}

/// Fetch a URL and return stripped plain text, capped at `max_chars`.
async fn fetch_and_strip_url(url: &str, max_chars: usize) -> String {
    let client = match reqwest::Client::builder()
        .user_agent("VibeCLI/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("(HTTP client error: {})", e),
    };

    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.text().await {
                Ok(body) => {
                    // Simple HTML strip: remove tags, decode entities
                    let no_tags = re_html_tags().replace_all(&body, " ");
                    let decoded = no_tags
                        .replace("&amp;", "&").replace("&lt;", "<")
                        .replace("&gt;", ">").replace("&quot;", "\"")
                        .replace("&nbsp;", " ").replace("&#39;", "'");
                    let collapsed = re_collapse_whitespace().replace_all(decoded.trim(), " ");
                    collapsed.chars().take(max_chars).collect()
                }
                Err(e) => format!("(Read body error: {})", e),
            }
        }
        Ok(resp) => format!("(HTTP {})", resp.status()),
        Err(e) => format!("(Request failed: {})", e),
    }
}
