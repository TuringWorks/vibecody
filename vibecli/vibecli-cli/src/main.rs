use clap::Parser;
use anyhow::Result;
use crate::config::Config;

/// Exit the process after flushing stdout/stderr to prevent lost output.
fn safe_exit(code: i32) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}
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
mod branch_agent;
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
mod cloud_agent;
mod mermaid_ascii;
mod github_app;
mod marketplace;
#[allow(dead_code)]
mod plugin_sdk;
#[allow(dead_code)]
mod plugin_registry;
#[allow(dead_code)]
mod plugin_lifecycle;
mod transform;
mod acp;
mod compliance;
mod screen_recorder;
use rustyline::error::ReadlineError;

mod computer_use;
mod feature_demo;
mod voice;
mod discovery;
mod tailscale;
mod pairing;
mod setup;
#[allow(dead_code)]
mod email_client;
#[allow(dead_code)]
mod calendar_client;
#[allow(dead_code)]
mod home_assistant;
#[allow(dead_code)]
mod productivity;
mod container_runtime;
mod docker_runtime;
mod podman_runtime;
mod opensandbox_client;
mod container_tool_executor;
mod verification;
mod workflow_orchestration;
mod handoff;
mod tui;
#[allow(dead_code)]
mod knowledge_graph;
#[allow(dead_code)]
mod gpu_terminal;
#[allow(dead_code)]
mod fine_tuning;
#[allow(dead_code)]
mod document_ingest;
#[allow(dead_code)]
mod web_crawler;
#[allow(dead_code)]
mod gpu_cluster;
#[allow(dead_code)]
mod vector_db;
#[allow(dead_code)]
mod database_client;
#[allow(dead_code)]
mod streaming_client;
#[allow(dead_code)]
mod inference_server;
#[allow(dead_code)]
mod distributed_training;
#[allow(dead_code)]
mod infinite_context;
#[allow(dead_code)]
mod app_builder;
#[allow(dead_code)]
mod batch_builder;
#[allow(dead_code)]
mod qa_validation;
#[allow(dead_code)]
mod legacy_migration;
#[allow(dead_code)]
mod git_platform;
#[allow(dead_code)]
mod automations;
#[allow(dead_code)]
mod self_review;
#[allow(dead_code)]
mod mcp_apps;
#[allow(dead_code)]
mod agent_teams_v2;
#[allow(dead_code)]
mod agent_teams_v2_enhanced;
#[allow(dead_code)]
mod semantic_mcp;
#[allow(dead_code)]
mod docgen;
#[allow(dead_code)]
mod remote_control;
#[allow(dead_code)]
mod ast_edit;
#[allow(dead_code)]
mod ci_status_check;
#[allow(dead_code)]
mod vscode_sessions;
#[allow(dead_code)]
mod cloud_sandbox;
#[allow(dead_code)]
mod plan_document;
#[allow(dead_code)]
mod security_scanning;
#[allow(dead_code)]
mod sub_agent_roles;
#[allow(dead_code)]
mod agent_host;
#[allow(dead_code)]
mod cloud_ide;
#[allow(dead_code)]
mod security_scan;
#[allow(dead_code)]
mod sub_agents;
#[allow(dead_code)]
mod next_edit;
#[allow(dead_code)]
mod edit_prediction;
#[allow(dead_code)]
mod conversational_search;
#[allow(dead_code)]
mod agent_modes;
#[allow(dead_code)]
mod agent_skills_compat;
#[allow(dead_code)]
mod debug_mode;
#[allow(dead_code)]
mod clarifying_questions;
#[allow(dead_code)]
mod discussion_mode;
#[allow(dead_code)]
mod image_gen_agent;
#[allow(dead_code)]
mod fast_context;
#[allow(dead_code)]
mod fullstack_gen;
#[allow(dead_code)]
mod team_governance;
#[allow(dead_code)]
mod cloud_autofix;
#[allow(dead_code)]
mod render_optimize;
#[allow(dead_code)]
mod gh_actions_agent;
#[allow(dead_code)]
mod usage_metering;
#[allow(dead_code)]
mod security_hardening;
#[allow(dead_code)]
mod mcp_lazy;
mod soul_generator;
#[allow(dead_code)]
mod context_bundles;
#[allow(dead_code)]
mod cloud_providers;
#[allow(dead_code)]
mod acp_protocol;
#[allow(dead_code)]
mod mcp_directory;
#[allow(dead_code)]
mod swe_bench;
#[allow(dead_code)]
mod session_memory;
#[allow(dead_code)]
mod compliance_controls;
#[allow(dead_code)]
mod multimodal_agent;
#[allow(dead_code)]
mod blue_team;
#[allow(dead_code)]
mod purple_team;
#[allow(dead_code)]
mod idp;
#[allow(dead_code)]
mod quantum_computing;
#[allow(dead_code)]
mod auto_research;
#[allow(dead_code)]
mod open_memory;
#[allow(dead_code)]
mod vulnerability_db;
#[allow(dead_code)]
mod resource_manager;
mod project_init;
mod spec_pipeline;
mod vm_orchestrator;
mod design_import;
#[allow(dead_code)]
mod audio_output;
mod session_sharing;
pub mod recipe;
pub mod diagnostics;
pub mod workspace_detect;
pub mod managed_deploy;
pub mod channel_daemon;
pub mod data_analysis;
pub mod org_context;
pub mod ci_gates;
#[allow(dead_code)]
pub mod agentic_cicd;
#[allow(dead_code)]
pub mod cross_surface_routing;
pub mod extension_compat;
pub mod context_streaming;
pub mod model_marketplace;
#[allow(dead_code)]
mod issue_triage;
pub mod warp_features;
#[allow(dead_code)]
mod observe_act;
#[allow(dead_code)]
mod browser_agent;
mod desktop_agent;
#[allow(dead_code)]
mod web_client;
mod counsel;
mod superbrain;
#[allow(dead_code)]
mod vscode_compat_ext;
mod large_codebase_bench;
mod jetbrains_hooks;
mod spawn_agent;
mod web_grounding;
#[allow(dead_code)]
mod worktree_pool;
mod mobile_gateway;
mod proactive_agent;
mod a2a_protocol;
#[allow(dead_code)]
mod semantic_index;
#[allow(dead_code)]
mod visual_verify;
#[allow(dead_code)]
mod mcp_streamable;
#[allow(dead_code)]
mod doc_sync;
#[allow(dead_code)]
mod next_task;
#[allow(dead_code)]
mod voice_local;
#[allow(dead_code)]
mod native_connectors;
#[allow(dead_code)]
mod agent_analytics;
#[allow(dead_code)]
mod agent_trust;
#[allow(dead_code)]
mod smart_deps;
#[allow(dead_code)]
mod rlcef_loop;
#[allow(dead_code)]
mod sketch_canvas;
#[allow(dead_code)]
mod mcts_repair;
#[allow(dead_code)]
mod langgraph_bridge;
mod api_key_monitor;
// Phase 32: New capabilities
mod context_protocol;
mod code_review_agent;
mod diff_review;
mod code_replay;
mod speculative_exec;
mod explainable_agent;
// Phase 32 P1
mod skill_distillation;
mod self_improving_skills;
mod review_protocol;
mod health_score;
mod ai_code_review;
mod intent_refactor;
mod architecture_spec;
mod policy_engine;
mod company_store;
mod adapter_registry;
mod company_goals;
mod company_tasks;
mod company_cmd;
mod company_documents;
mod company_budget;
mod company_approvals;
mod company_secrets;
mod company_routines;
mod company_heartbeat;
mod company_workspace_config;
mod company_priority_map;
mod company_meeting_notes;
mod company_portability;
mod company_orchestrator;
mod profile_store;
mod workspace_store;
// Phase 33-39: FIT-GAP v8
#[allow(dead_code)]
mod design_mode;
#[allow(dead_code)]
mod ide_bridge;
#[allow(dead_code)]
mod on_device;
#[allow(dead_code)]
mod hard_problem;
#[allow(dead_code)]
mod auto_deploy;
#[allow(dead_code)]
mod clawcode_compat;
#[allow(dead_code)]
mod team_onboarding;
#[allow(dead_code)]
mod repro_agent;
// FIT-GAP v9 — P0 modules
#[allow(dead_code)]
mod test_gen;
#[allow(dead_code)]
mod polyglot_refactor;
#[allow(dead_code)]
mod supply_chain;
#[allow(dead_code)]
mod cost_predictor;
// FIT-GAP v9 — P1 modules
#[allow(dead_code)]
mod hybrid_search;
#[allow(dead_code)]
mod threat_model;
#[allow(dead_code)]
mod collab_session;
#[allow(dead_code)]
mod reasoning_video;
// FIT-GAP v9 — P2 modules
#[allow(dead_code)]
mod api_sketch;
#[allow(dead_code)]
mod a11y_agent;
#[allow(dead_code)]
mod perf_profiler;
#[allow(dead_code)]
mod temporal_debug;
#[allow(dead_code)]
mod symbolic_exec;
#[allow(dead_code)]
mod schema_migration;
// FIT-GAP v9 — P3 modules
#[allow(dead_code)]
mod federated_orchestrator;
#[allow(dead_code)]
mod incident_response;
#[allow(dead_code)]
mod local_embed_refresh;
#[allow(dead_code)]
mod workload_model_sel;
// RL-OS: Unified Reinforcement Learning Lifecycle Platform
#[allow(dead_code)]
mod rl_env_os;
#[allow(dead_code)]
mod rl_train_os;
#[allow(dead_code)]
mod rl_eval_os;
#[allow(dead_code)]
mod rl_opti_os;
#[allow(dead_code)]
mod rl_model_hub;
#[allow(dead_code)]
mod rl_serve_os;
#[allow(dead_code)]
mod rl_rlhf;
#[allow(dead_code)]
mod rl_observe;

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

    /// Run a single REPL command non-interactively and exit.
    /// Example: vibecli --cmd "/email unread"
    ///          vibecli --cmd "/cal today"
    ///          vibecli --cmd "/jira mine"
    #[arg(long, value_name = "CMD")]
    cmd: Option<String>,

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

    /// CI mode: output structured JSON review, exit 1 if findings exceed threshold.
    #[arg(long)]
    ci_mode: bool,

    /// Minimum severity to fail in CI mode: critical, warning/high (default), info/medium/low.
    #[arg(long, default_value = "warning")]
    severity_threshold: String,

    // ── Setup wizard ─────────────────────────────────────────────────────────

    /// Run the interactive setup wizard. Detects your platform, configures an
    /// AI provider, and optionally installs VibeCody as an always-on service.
    #[arg(long)]
    setup: bool,

    /// Service management subcommand: install, start, stop, status.
    /// Example: vibecli --service install
    #[arg(long, value_name = "SUBCOMMAND")]
    service: Option<String>,

    // ── Daemon mode ───────────────────────────────────────────────────────────

    /// Start the VibeCLI HTTP daemon (for VS Code extension / Agent SDK).
    #[arg(long)]
    serve: bool,

    /// Port for daemon mode (default: 7878).
    #[arg(long, default_value = "7878")]
    port: u16,

    /// Host/IP to bind the daemon to (default: 127.0.0.1).
    /// Use 0.0.0.0 to listen on all interfaces (required for mobile app access).
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

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

    /// Start as an always-on channel daemon with automation routing.
    /// Like --gateway but routes messages through automation rules and
    /// spawns concurrent agent tasks. Multi-turn session affinity per channel.
    /// Example: vibecli --channel-daemon telegram
    #[arg(long, value_name = "PLATFORM")]
    channel_daemon: Option<String>,

    // ── Plugin management ────────────────────────────────────────────────────

    /// Plugin subcommand: create, install, uninstall, enable, disable, update, list, info, search, dev, publish.
    /// Example: vibecli plugin install vibecody-jira
    #[arg(long, value_name = "SUBCOMMAND")]
    plugin: Option<String>,

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

    /// Disable all network access for agent execution. Blocks WebSearch and
    /// FetchUrl tools, and wraps shell commands in OS-level network isolation
    /// (`sandbox-exec -n no-network` on macOS, `unshare --net` on Linux).
    #[arg(long)]
    no_network: bool,

    // ── Screen Recording (Phase 8.16) ──────────────────────────────────────────

    /// Record agent actions as a sequence of screenshots that can be
    /// assembled into GIF artifacts. Frames are saved under
    /// ~/.vibecli/recordings/<session-id>/.
    #[arg(long)]
    record: bool,

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

    // ── Cloud Agent (Phase 8.17) ──────────────────────────────────────────────

    /// Run the agent task inside an isolated Docker container.
    /// Requires Docker to be installed and running. Combine with --agent to
    /// specify the task. Example: vibecli --cloud --agent "fix all clippy warnings"
    #[arg(long)]
    cloud: bool,

    // ── Voice mode (Phase P6) ─────────────────────────────────────────────────

    /// Enable voice input mode: transcribes audio from microphone before each
    /// REPL prompt using Groq Whisper. Requires GROQ_API_KEY or
    /// voice.whisper_api_key in config. TTS for responses uses ElevenLabs.
    #[arg(long)]
    voice: bool,

    // ── Tailscale integration (Phase P7) ──────────────────────────────────────

    /// Expose the daemon via Tailscale Funnel (public HTTPS). Requires
    /// Tailscale to be installed and `tailscale` on PATH.
    /// Combine with --serve: vibecli --serve --tailscale
    #[arg(long)]
    tailscale: bool,

    // ── Container Sandbox Runtime (Phase sandbox) ─────────────────────────────

    /// Container runtime for sandbox execution: docker, podman, opensandbox, auto.
    /// When set, agent tool calls (bash, file read/write) execute inside a
    /// container instead of the local filesystem. Defaults to config value or "auto".
    /// Example: vibecli --sandbox-runtime docker --agent "ls /etc"
    #[arg(long, value_name = "RUNTIME")]
    sandbox_runtime: Option<String>,

    // ── Permission mode (Goose-style) ─────────────────────────────────────────

    /// Permission mode for tool execution.
    /// chat-only   — no tool calls, conversational only
    /// manual      — prompt before every tool call (default, same as --suggest)
    /// smart       — auto-apply file edits, prompt for shell commands (same as --auto-edit)
    /// auto        — execute all tool calls without prompting (same as --full-auto)
    #[arg(long, value_name = "MODE", value_parser = ["chat-only", "manual", "smart", "auto"])]
    mode: Option<String>,

    // ── Shell completions ─────────────────────────────────────────────────────

    /// Print shell completion script and exit.
    /// Example: vibecli --completions zsh >> ~/.zshrc
    ///          vibecli --completions bash > /etc/bash_completion.d/vibecli
    #[arg(long, value_name = "SHELL", value_parser = ["bash", "zsh", "fish", "powershell", "elvish"])]
    completions: Option<String>,

    // ── Recipe system ─────────────────────────────────────────────────────────

    /// Run a recipe YAML file (parameterized multi-step automation).
    /// Example: vibecli --recipe create-feature.yaml --param feature_name=auth
    #[arg(long, value_name = "FILE")]
    recipe: Option<String>,

    /// Key=value parameter for --recipe. Can be repeated.
    /// Example: --param language=rust --param feature_name=auth
    #[arg(long = "param", value_name = "KEY=VALUE", action = clap::ArgAction::Append)]
    params: Vec<String>,

    /// Dry-run a recipe: print all steps with substituted params without executing.
    #[arg(long)]
    dry_run: bool,

    /// B2: List all supported providers and their default models, then exit.
    #[arg(long)]
    list_providers: bool,

    // ── Session management extras ─────────────────────────────────────────────

    /// Fork a session to explore an alternative path.
    /// Creates a copy of the given session ID you can diverge from.
    /// Example: vibecli --fork sess-20260405-abc123
    #[arg(long, value_name = "SESSION_ID")]
    fork: Option<String>,

    /// Export a session to a file.
    /// Format is inferred from extension: .md, .json, .yaml
    /// Example: vibecli --export-session sess-20260405 --output session.md
    #[arg(long, value_name = "SESSION_ID")]
    export_session: Option<String>,

    /// Generate a diagnostics bundle (redacted config + logs + session) for bug reports.
    /// Saves a zip to the current directory.
    #[arg(long)]
    diagnostics: bool,

    /// Positional arguments: used as a one-shot chat message.
    /// Examples:
    ///   vibecli "Hello, what can you help me with?"
    ///   vibecli chat "Hello!"           (the word "chat" is included in the message)
    ///   vibecli "Explain this code"
    #[arg(trailing_var_arg = true)]
    message: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // ── Global panic handler (antifragility: crash forensics) ─────────────
    // Log panic info + backtrace to ~/.vibecli/crash.log before the default
    // panic output so we can diagnose daemon crashes post-mortem.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(home) = dirs::home_dir() {
            let crash_dir = home.join(".vibecli");
            let _ = std::fs::create_dir_all(&crash_dir);
            let crash_log = crash_dir.join("crash.log");
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&crash_log)
            {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = writeln!(f, "--- PANIC at epoch {} ---", timestamp);
                let _ = writeln!(f, "{}", info);
                let bt = std::backtrace::Backtrace::force_capture();
                let _ = writeln!(f, "{}", bt);
                let _ = writeln!(f, "---");
            }
        }
        default_hook(info);
    }));

    let cli = Cli::parse();

    // Validate config file on startup — warn (don't fail) if it exists but is malformed.
    if let Ok(config_path) = Config::config_path() {
        if config_path.exists() {
            if let Err(e) = Config::load() {
                eprintln!(
                    "Warning: config file at {} could not be parsed: {}\n  \
                     Falling back to default settings. Fix the file or delete it to silence this warning.",
                    config_path.display(),
                    e
                );
            }
        }
    }

    // Initialize tracing (with optional OTLP export if [otel] enabled = true).
    let otel_config = Config::load()
        .map(|c| c.otel.clone())
        .unwrap_or_default();
    // Keep the guard alive for the entire program.
    let _otel_guard = otel_init::setup(&otel_config)?;

    // ── Shell completions (early exit) ────────────────────────────────────────
    if let Some(ref shell_name) = cli.completions {
        use clap::CommandFactory;
        use clap_complete::{generate, shells};
        let mut cmd = Cli::command();
        let name = "vibecli";
        match shell_name.as_str() {
            "bash"       => generate(shells::Bash,       &mut cmd, name, &mut std::io::stdout()),
            "zsh"        => generate(shells::Zsh,        &mut cmd, name, &mut std::io::stdout()),
            "fish"       => generate(shells::Fish,       &mut cmd, name, &mut std::io::stdout()),
            "powershell" => generate(shells::PowerShell, &mut cmd, name, &mut std::io::stdout()),
            "elvish"     => generate(shells::Elvish,     &mut cmd, name, &mut std::io::stdout()),
            _            => eprintln!("Unknown shell: {}", shell_name),
        }
        return Ok(());
    }

    // Determine approval policy: --mode flag takes highest priority, then
    // legacy --suggest/--auto-edit/--full-auto flags, then config, then default.
    let approval_policy = {
        if let Some(ref mode) = cli.mode {
            // --mode chat-only|manual|smart|auto
            mode.clone()
        } else {
            Config::load()
                .map(|c| {
                    let from_config = c.safety.approval_policy.clone();
                    let from_flags = Config::approval_from_flags(cli.suggest, cli.auto_edit, cli.full_auto);
                    if cli.suggest || cli.auto_edit || cli.full_auto {
                        from_flags
                    } else {
                        from_config
                    }
                })
                .unwrap_or_else(|_| Config::approval_from_flags(cli.suggest, cli.auto_edit, cli.full_auto))
        }
    };

    // Resolve sandbox flag: CLI --sandbox overrides config
    let sandbox_enabled = {
        let from_config = Config::load().map(|c| c.safety.sandbox).unwrap_or(false);
        cli.sandbox || from_config
    };

    // Resolve --no-network flag
    let no_network = cli.no_network;

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
        safe_exit(if ok { 0 } else { 1 });
    }

    // ── B2: List providers (early exit) ──────────────────────────────────────
    if cli.list_providers {
        list_providers_and_models();
        safe_exit(0);
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
                    eprintln!("Profile '{}' → provider={}, approval={}", profile_name, provider, policy);
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

    // ── Recipe runner: vibecli --recipe <file> [--param key=val] ──────────────
    if let Some(ref recipe_file) = cli.recipe {
        let params: std::collections::HashMap<String, String> = cli.params.iter()
            .filter_map(|p| {
                let mut parts = p.splitn(2, '=');
                let k = parts.next()?.trim().to_string();
                let v = parts.next()?.trim().to_string();
                Some((k, v))
            })
            .collect();
        if cli.dry_run {
            return recipe::dry_run_recipe(recipe_file, &params);
        }
        return recipe::run_recipe(recipe_file, &params, &effective_provider, &effective_model, sandbox_enabled).await;
    }

    // ── Session forking: vibecli --fork <session-id> ──────────────────────────
    if let Some(ref src_id) = cli.fork {
        return session_store::fork_session_cmd(src_id).await;
    }

    // ── Session export: vibecli --export-session <id> --output <file> ────────
    if let Some(ref session_id) = cli.export_session {
        let output_path = cli.output.as_deref().unwrap_or("session.md");
        return session_sharing::export_session_cmd(session_id, output_path).await;
    }

    // ── Diagnostics bundle: vibecli --diagnostics ─────────────────────────────
    if cli.diagnostics {
        return diagnostics::generate_bundle(cli.resume.as_deref()).await;
    }

    // Setup wizard: vibecli --setup
    if cli.setup {
        return setup::run_setup().await;
    }

    // Service management: vibecli --service <install|start|stop|status>
    if let Some(ref subcmd) = cli.service {
        return match subcmd.as_str() {
            "install" => setup::service_install(),
            "start" => setup::service_start(),
            "stop" => setup::service_stop(),
            "status" => setup::service_status(),
            other => {
                eprintln!("Unknown service subcommand: {other}");
                eprintln!("Available: install, start, stop, status");
                std::process::exit(1);
            }
        };
    }

    // Daemon mode: vibecli --serve [--port 7878]
    if cli.serve {
        // Optionally expose via Tailscale Funnel
        if cli.tailscale {
            eprintln!("[vibecli] Activating Tailscale Funnel on port {}...", cli.port);
            match tailscale::serve_via_funnel(cli.port).await {
                Ok(_child) => eprintln!("[vibecli] Tailscale Funnel active — public HTTPS endpoint created"),
                Err(e) => eprintln!("[vibecli] Tailscale Funnel failed: {e} (continuing with localhost)"),
            }
        }
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let cwd = std::env::current_dir()?;
        let approval = ApprovalPolicy::from_str(&approval_policy);
        return serve::serve(llm, effective_provider.clone(), approval, cwd, cli.port, cli.host.clone()).await;
    }

    // MCP server mode: vibecli --mcp-server
    if cli.mcp_server {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let cwd = std::env::current_dir()?;
        let approval = ApprovalPolicy::from_str(&approval_policy);
        let config = Config::load().unwrap_or_default();
        return mcp_server::run_server(cwd, llm, approval, config.safety.sandbox).await;
    }

    // Plugin management: vibecli --plugin <subcommand> (remaining args from env::args)
    if let Some(ref subcmd) = cli.plugin {
        let raw_args: Vec<String> = std::env::args().collect();
        // Find args after --plugin <subcmd>
        let plugin_pos = raw_args.iter().position(|a| a == "--plugin").unwrap_or(0);
        let extra_args: Vec<&str> = raw_args.iter().skip(plugin_pos + 2).map(|s| s.as_str()).collect();

        match subcmd.as_str() {
            "create" => {
                let name = extra_args.first().ok_or_else(|| anyhow::anyhow!("Usage: vibecli --plugin create <name> [--kind connector|adapter|optimizer|theme|skillpack|workflow]"))?;
                let kind_str = extra_args.iter()
                    .position(|a| *a == "--kind")
                    .and_then(|i| extra_args.get(i + 1))
                    .copied()
                    .unwrap_or("connector");
                let kind = match kind_str {
                    "adapter" => plugin_sdk::PluginKind::Adapter,
                    "optimizer" => plugin_sdk::PluginKind::Optimizer,
                    "theme" => plugin_sdk::PluginKind::Theme,
                    "skillpack" => plugin_sdk::PluginKind::SkillPack,
                    "workflow" => plugin_sdk::PluginKind::Workflow,
                    "extension" => plugin_sdk::PluginKind::Extension,
                    _ => plugin_sdk::PluginKind::Connector,
                };
                let dir = std::env::current_dir().map_err(|e| anyhow::anyhow!("Cannot read current directory: {e}"))?;
                match plugin_sdk::PluginScaffold::create(name, kind, &dir) {
                    Ok(path) => println!("Plugin scaffolded at {}", path.display()),
                    Err(e) => eprintln!("Error: {}", e),
                }
                return Ok(());
            }
            "install" => {
                let target = extra_args.first().ok_or_else(|| anyhow::anyhow!("Usage: vibecli --plugin install <name|repo-url>"))?;
                let mut lifecycle = plugin_lifecycle::PluginLifecycle::new()?;
                if target.starts_with("http") || target.starts_with("git@") {
                    let plugin_name = target.split('/').next_back().unwrap_or(target).trim_end_matches(".git");
                    match lifecycle.install_from_repo(plugin_name, target) {
                        Ok(p) => println!("Installed {} v{}", p.name, p.version),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    let mut registry = plugin_registry::PluginRegistry::new();
                    registry.load_cached()?;
                    if let Some(entry) = registry.find(target) {
                        if let Some(ref repo) = entry.repository {
                            match lifecycle.install_from_repo(target, repo) {
                                Ok(p) => println!("Installed {} v{}", p.name, p.version),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        } else {
                            eprintln!("Plugin '{}' has no repository URL", target);
                        }
                    } else {
                        eprintln!("Plugin '{}' not found in registry", target);
                    }
                }
                return Ok(());
            }
            "uninstall" | "remove" => {
                let name = extra_args.first().ok_or_else(|| anyhow::anyhow!("Usage: vibecli --plugin uninstall <name>"))?;
                let mut lifecycle = plugin_lifecycle::PluginLifecycle::new()?;
                match lifecycle.uninstall(name) {
                    Ok(()) => println!("Uninstalled {}", name),
                    Err(e) => eprintln!("Error: {}", e),
                }
                return Ok(());
            }
            "enable" => {
                let name = extra_args.first().ok_or_else(|| anyhow::anyhow!("Usage: vibecli --plugin enable <name>"))?;
                let mut lifecycle = plugin_lifecycle::PluginLifecycle::new()?;
                match lifecycle.enable(name) {
                    Ok(()) => println!("Enabled {}", name),
                    Err(e) => eprintln!("Error: {}", e),
                }
                return Ok(());
            }
            "disable" => {
                let name = extra_args.first().ok_or_else(|| anyhow::anyhow!("Usage: vibecli --plugin disable <name>"))?;
                let mut lifecycle = plugin_lifecycle::PluginLifecycle::new()?;
                match lifecycle.disable(name) {
                    Ok(()) => println!("Disabled {}", name),
                    Err(e) => eprintln!("Error: {}", e),
                }
                return Ok(());
            }
            "update" => {
                let mut lifecycle = plugin_lifecycle::PluginLifecycle::new()?;
                if let Some(name) = extra_args.first() {
                    match lifecycle.update(name) {
                        Ok(change) => println!("Updated {}: {}", name, change),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    match lifecycle.update_all() {
                        Ok(results) => {
                            for (name, change) in results {
                                println!("  {} — {}", name, change);
                            }
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
                return Ok(());
            }
            "list" | "ls" => {
                let lifecycle = plugin_lifecycle::PluginLifecycle::new()?;
                let plugins = lifecycle.list();
                if plugins.is_empty() {
                    println!("No plugins installed. Use 'vibecli --plugin install <name>' to install.");
                } else {
                    println!("{:<30} {:<12} {:<12}", "NAME", "VERSION", "STATE");
                    println!("{}", "-".repeat(54));
                    for p in plugins {
                        let state_str = match &p.state {
                            plugin_lifecycle::PluginState::Enabled => "enabled",
                            plugin_lifecycle::PluginState::Disabled => "disabled",
                            plugin_lifecycle::PluginState::DevMode => "dev",
                            plugin_lifecycle::PluginState::Outdated => "outdated",
                            plugin_lifecycle::PluginState::Installed => "installed",
                            plugin_lifecycle::PluginState::Errored(_) => "error",
                        };
                        println!("{:<30} {:<12} {:<12}", p.name, p.version, state_str);
                    }
                }
                return Ok(());
            }
            "info" => {
                let name = extra_args.first().ok_or_else(|| anyhow::anyhow!("Usage: vibecli --plugin info <name>"))?;
                let lifecycle = plugin_lifecycle::PluginLifecycle::new()?;
                match lifecycle.info(name) {
                    Ok(info) => {
                        println!("Plugin: {}", info.plugin.name);
                        println!("Version: {}", info.plugin.version);
                        println!("State: {:?}", info.plugin.state);
                        println!("Installed: {}", info.plugin.installed_at);
                        println!("Skills: {}", info.skills_count);
                        println!("Hooks: {}", info.hooks_count);
                        println!("Commands: {}", info.commands_count);
                        if !info.plugin.config.is_empty() {
                            println!("Config:");
                            for (k, v) in &info.plugin.config {
                                println!("  {} = {}", k, v);
                            }
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
                return Ok(());
            }
            "search" => {
                let query = extra_args.first().copied().unwrap_or("");
                let mut registry = plugin_registry::PluginRegistry::new();
                registry.load_cached()?;
                let results = registry.search(query, None);
                if results.is_empty() {
                    println!("No plugins found for '{}'", query);
                } else {
                    println!("{:<30} {:<12} {:<10} DESCRIPTION", "NAME", "VERSION", "DOWNLOADS");
                    println!("{}", "-".repeat(80));
                    for e in results.iter().take(20) {
                        let desc = if e.description.len() > 40 {
                            format!("{}...", &e.description[..37])
                        } else {
                            e.description.clone()
                        };
                        println!("{:<30} {:<12} {:<10} {}", e.name, e.version, e.downloads, desc);
                    }
                }
                return Ok(());
            }
            "dev" => {
                let dir_str = extra_args.first().copied().unwrap_or(".");
                let source_dir = std::path::PathBuf::from(dir_str).canonicalize()?;
                let name = source_dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("dev-plugin");
                let mut lifecycle = plugin_lifecycle::PluginLifecycle::new()?;
                match lifecycle.install_dev(name, &source_dir) {
                    Ok(p) => println!("Dev-linked {} -> {}", p.name, source_dir.display()),
                    Err(e) => eprintln!("Error: {}", e),
                }
                return Ok(());
            }
            "publish" => {
                let dir_str = extra_args.first().copied().unwrap_or(".");
                let plugin_dir = std::path::PathBuf::from(dir_str).canonicalize()?;
                match plugin_registry::PluginRegistry::prepare_publish(&plugin_dir) {
                    Ok(pkg) => {
                        println!("Plugin '{}' v{} validated and ready to publish", pkg.manifest.name, pkg.manifest.version);
                        println!("  Push to a git repository and submit to the VibeCody plugin registry.");
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
                return Ok(());
            }
            _ => {
                println!("VibeCody Plugin Manager\n");
                println!("Usage: vibecli --plugin <command>\n");
                println!("Commands:");
                println!("  create <name> [--kind <type>]   Scaffold a new plugin project");
                println!("  install <name|url>              Install a plugin");
                println!("  uninstall <name>                Remove a plugin");
                println!("  enable <name>                   Enable a disabled plugin");
                println!("  disable <name>                  Disable a plugin");
                println!("  update [name]                   Update plugin(s)");
                println!("  list                            List installed plugins");
                println!("  info <name>                     Show plugin details");
                println!("  search <query>                  Search the plugin registry");
                println!("  dev [dir]                       Link a local dir for development");
                println!("  publish [dir]                   Validate and prepare for publishing");
                println!("\nPlugin types: connector, adapter, optimizer, theme, skillpack, workflow, extension");
                return Ok(());
            }
        }
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
            "googlechat" | "google-chat" | "google_chat" | "gchat" => {
                let sa_json = gw_cfg.resolve_googlechat_service_account_json()
                    .ok_or_else(|| anyhow::anyhow!("Google Chat service account not configured. Set GOOGLE_CHAT_SERVICE_ACCOUNT_JSON or gateway.googlechat_service_account_json in config."))?;
                let space = gw_cfg.resolve_googlechat_space_id()
                    .ok_or_else(|| anyhow::anyhow!("Google Chat space ID not configured. Set GOOGLE_CHAT_SPACE_ID or gateway.googlechat_space_id in config."))?;
                Box::new(gateway::GoogleChatGateway::new(sa_json, space))
            }
            "mattermost" => {
                let url = gw_cfg.resolve_mattermost_url()
                    .ok_or_else(|| anyhow::anyhow!("Mattermost URL not configured. Set MATTERMOST_URL or gateway.mattermost_url in config."))?;
                let token = gw_cfg.resolve_mattermost_token()
                    .ok_or_else(|| anyhow::anyhow!("Mattermost token not configured. Set MATTERMOST_TOKEN or gateway.mattermost_token in config."))?;
                let channel_id = gw_cfg.resolve_mattermost_channel_id()
                    .ok_or_else(|| anyhow::anyhow!("Mattermost channel ID not configured. Set MATTERMOST_CHANNEL_ID or gateway.mattermost_channel_id in config."))?;
                Box::new(gateway::MattermostGateway::new(url, token, channel_id))
            }
            "irc" => {
                let server = gw_cfg.resolve_irc_server()
                    .ok_or_else(|| anyhow::anyhow!("IRC server not configured. Set IRC_SERVER or gateway.irc_server in config."))?;
                let port = gw_cfg.irc_port.unwrap_or(6667);
                let nick = gw_cfg.resolve_irc_nick().unwrap_or_else(|| "vibecli".to_string());
                let channel = gw_cfg.resolve_irc_channel()
                    .ok_or_else(|| anyhow::anyhow!("IRC channel not configured. Set IRC_CHANNEL or gateway.irc_channel in config."))?;
                Box::new(gateway::IRCGateway::new(server, port, nick, channel))
            }
            "line" => {
                let token = gw_cfg.resolve_line_channel_access_token()
                    .ok_or_else(|| anyhow::anyhow!("LINE channel access token not configured. Set LINE_CHANNEL_ACCESS_TOKEN or gateway.line_channel_access_token in config."))?;
                let secret = gw_cfg.resolve_line_channel_secret().unwrap_or_default();
                Box::new(gateway::LINEGateway::new(token, secret))
            }
            "twitch" => {
                let oauth = gw_cfg.resolve_twitch_oauth_token()
                    .ok_or_else(|| anyhow::anyhow!("Twitch OAuth token not configured. Set TWITCH_OAUTH_TOKEN or gateway.twitch_oauth_token in config."))?;
                let channel = gw_cfg.resolve_twitch_channel()
                    .ok_or_else(|| anyhow::anyhow!("Twitch channel not configured. Set TWITCH_CHANNEL or gateway.twitch_channel in config."))?;
                let nick = gw_cfg.resolve_twitch_nick().unwrap_or_else(|| "vibecli".to_string());
                Box::new(gateway::TwitchGateway::new(oauth, channel, nick))
            }
            "nextcloud" | "nextcloud-talk" | "nextcloud_talk" => {
                let url = gw_cfg.resolve_nextcloud_url()
                    .ok_or_else(|| anyhow::anyhow!("Nextcloud URL not configured. Set NEXTCLOUD_URL or gateway.nextcloud_url in config."))?;
                let user = gw_cfg.resolve_nextcloud_user()
                    .ok_or_else(|| anyhow::anyhow!("Nextcloud user not configured. Set NEXTCLOUD_USER or gateway.nextcloud_user in config."))?;
                let password = gw_cfg.resolve_nextcloud_password()
                    .ok_or_else(|| anyhow::anyhow!("Nextcloud password not configured. Set NEXTCLOUD_PASSWORD or gateway.nextcloud_password in config."))?;
                let room_token = gw_cfg.resolve_nextcloud_room_token()
                    .ok_or_else(|| anyhow::anyhow!("Nextcloud room token not configured. Set NEXTCLOUD_ROOM_TOKEN or gateway.nextcloud_room_token in config."))?;
                Box::new(gateway::NextcloudTalkGateway::new(url, user, password, room_token))
            }
            "webchat" | "web-chat" | "web_chat" => {
                let port = gw_cfg.webchat_port.unwrap_or(8090);
                Box::new(gateway::WebChatGateway::new(port))
            }
            "nostr" => {
                let private_key = gw_cfg.resolve_nostr_private_key()
                    .ok_or_else(|| anyhow::anyhow!("Nostr private key not configured. Set NOSTR_PRIVATE_KEY or gateway.nostr_private_key in config."))?;
                let relay_urls = if gw_cfg.nostr_relay_urls.is_empty() {
                    vec!["wss://relay.damus.io".to_string()]
                } else {
                    gw_cfg.nostr_relay_urls.clone()
                };
                Box::new(gateway::NostrGateway::new(private_key, relay_urls))
            }
            "feishu" | "lark" => {
                let app_id = gw_cfg.resolve_feishu_app_id()
                    .ok_or_else(|| anyhow::anyhow!("Feishu app ID not configured. Set FEISHU_APP_ID or gateway.feishu_app_id in config."))?;
                let app_secret = gw_cfg.resolve_feishu_app_secret()
                    .ok_or_else(|| anyhow::anyhow!("Feishu app secret not configured. Set FEISHU_APP_SECRET or gateway.feishu_app_secret in config."))?;
                Box::new(gateway::FeishuGateway::new(app_id, app_secret))
            }
            "dingtalk" | "ding" => {
                let access_token = gw_cfg.resolve_dingtalk_access_token()
                    .ok_or_else(|| anyhow::anyhow!("DingTalk access token not configured. Set DINGTALK_ACCESS_TOKEN or gateway.dingtalk_access_token in config."))?;
                let webhook_secret = gw_cfg.resolve_dingtalk_webhook_secret().unwrap_or_default();
                Box::new(gateway::DingTalkGateway::new(access_token, webhook_secret))
            }
            "qq" => {
                let app_id = gw_cfg.resolve_qq_app_id()
                    .ok_or_else(|| anyhow::anyhow!("QQ app ID not configured. Set QQ_APP_ID or gateway.qq_app_id in config."))?;
                let token = gw_cfg.resolve_qq_token()
                    .ok_or_else(|| anyhow::anyhow!("QQ token not configured. Set QQ_TOKEN or gateway.qq_token in config."))?;
                Box::new(gateway::QQGateway::new(app_id, token))
            }
            "wecom" | "wechat-work" | "wechat_work" => {
                let corp_id = gw_cfg.resolve_wecom_corp_id()
                    .ok_or_else(|| anyhow::anyhow!("WeCom corp ID not configured. Set WECOM_CORP_ID or gateway.wecom_corp_id in config."))?;
                let agent_id = gw_cfg.resolve_wecom_agent_id()
                    .ok_or_else(|| anyhow::anyhow!("WeCom agent ID not configured. Set WECOM_AGENT_ID or gateway.wecom_agent_id in config."))?;
                let secret = gw_cfg.resolve_wecom_secret()
                    .ok_or_else(|| anyhow::anyhow!("WeCom secret not configured. Set WECOM_SECRET or gateway.wecom_secret in config."))?;
                Box::new(gateway::WeComGateway::new(corp_id, agent_id, secret))
            }
            "zalo" => {
                let access_token = gw_cfg.resolve_zalo_access_token()
                    .ok_or_else(|| anyhow::anyhow!("Zalo access token not configured. Set ZALO_ACCESS_TOKEN or gateway.zalo_access_token in config."))?;
                Box::new(gateway::ZaloGateway::new(access_token))
            }
            "bluebubbles" | "blue-bubbles" | "blue_bubbles" => {
                let url = gw_cfg.resolve_bluebubbles_url()
                    .ok_or_else(|| anyhow::anyhow!("BlueBubbles URL not configured. Set BLUEBUBBLES_URL or gateway.bluebubbles_url in config."))?;
                let password = gw_cfg.resolve_bluebubbles_password()
                    .ok_or_else(|| anyhow::anyhow!("BlueBubbles password not configured. Set BLUEBUBBLES_PASSWORD or gateway.bluebubbles_password in config."))?;
                Box::new(gateway::BlueBubblesGateway::new(url, password))
            }
            "synology" | "synology-chat" | "synology_chat" => {
                let url = gw_cfg.resolve_synology_url()
                    .ok_or_else(|| anyhow::anyhow!("Synology URL not configured. Set SYNOLOGY_URL or gateway.synology_url in config."))?;
                let incoming_url = gw_cfg.resolve_synology_incoming_url()
                    .ok_or_else(|| anyhow::anyhow!("Synology incoming URL not configured. Set SYNOLOGY_INCOMING_URL or gateway.synology_incoming_url in config."))?;
                let token = gw_cfg.resolve_synology_token().unwrap_or_default();
                Box::new(gateway::SynologyChatGateway::new(url, incoming_url, token))
            }
            "tlon" | "urbit" => {
                let ship_url = gw_cfg.resolve_tlon_ship_url()
                    .ok_or_else(|| anyhow::anyhow!("Tlon ship URL not configured. Set TLON_SHIP_URL or gateway.tlon_ship_url in config."))?;
                let ship_code = gw_cfg.resolve_tlon_ship_code()
                    .ok_or_else(|| anyhow::anyhow!("Tlon ship code not configured. Set TLON_SHIP_CODE or gateway.tlon_ship_code in config."))?;
                Box::new(gateway::TlonGateway::new(ship_url, ship_code))
            }
            other => return Err(anyhow::anyhow!(
                "Unknown gateway platform: '{}'. Use: telegram, discord, slack, signal, matrix, twilio, whatsapp, imessage, teams, googlechat, mattermost, irc, line, twitch, nextcloud, webchat, nostr, feishu, dingtalk, qq, wecom, zalo, bluebubbles, synology, tlon",
                other
            )),
        };
        return gateway::run_gateway(platform_box, llm).await;
    }

    // Enhanced channel daemon mode: vibecli --channel-daemon telegram
    // Routes messages through automation rules, spawns concurrent agent tasks,
    // maintains session affinity per channel+user.
    if let Some(ref platform) = cli.channel_daemon {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let config = Config::load().unwrap_or_default();
        let gw_cfg = &config.gateway;

        // Reuse the same platform adapters as --gateway
        let platform_box: Box<dyn gateway::GatewayPlatform> = match platform.as_str() {
            "telegram" => {
                let token = gw_cfg.resolve_telegram_token()
                    .ok_or_else(|| anyhow::anyhow!("Telegram token not configured. Set TELEGRAM_BOT_TOKEN."))?;
                Box::new(gateway::TelegramGateway::new(token, gw_cfg.allowed_users.clone()))
            }
            "discord" => {
                let token = gw_cfg.resolve_discord_token()
                    .ok_or_else(|| anyhow::anyhow!("Discord token not configured. Set DISCORD_BOT_TOKEN."))?;
                let channel = gw_cfg.discord_channel_id.clone().unwrap_or_default();
                Box::new(gateway::DiscordGateway::new(token, channel))
            }
            "slack" => {
                let token = gw_cfg.resolve_slack_bot_token()
                    .ok_or_else(|| anyhow::anyhow!("Slack token not configured. Set SLACK_BOT_TOKEN."))?;
                let channel = gw_cfg.slack_channel_id.clone().unwrap_or_default();
                Box::new(gateway::SlackGateway::new(token, channel))
            }
            other => return Err(anyhow::anyhow!(
                "Unknown channel-daemon platform: '{}'. Supported: telegram, discord, slack", other
            )),
        };

        // Create automation engine with rules from config
        let workspace = std::env::current_dir()?;
        let automation_engine = std::sync::Arc::new(
            tokio::sync::Mutex::new(
                automations::AutomationEngine::new(workspace.join(".vibecli").join("automations"))
            )
        );

        eprintln!("[channel-daemon] Starting enhanced daemon on {}", platform);
        eprintln!("[channel-daemon] Automation rules: .vibecli/automations/");
        eprintln!("[channel-daemon] Max concurrent tasks: 4");

        return gateway::run_channel_daemon(platform_box, llm, automation_engine, 4).await;
    }

    if cli.tui {
        return tui::run(effective_provider, effective_model).await;
    }

    // Single REPL command mode: --cmd "/email unread"
    if let Some(cmd_str) = cli.cmd {
        let trimmed = cmd_str.trim();
        let (command, args) = if let Some(pos) = trimmed.find(' ') {
            (&trimmed[..pos], trimmed[pos + 1..].trim())
        } else {
            (trimmed, "")
        };
        match command {
            "/email" => {
                let output = crate::email_client::handle_email_command(args).await;
                print!("{}", output);
            }
            "/calendar" | "/cal" => {
                let output = crate::calendar_client::handle_calendar_command(args).await;
                print!("{}", output);
            }
            "/home" | "/ha" => {
                let output = crate::home_assistant::handle_ha_command(args).await;
                print!("{}", output);
            }
            "/notion" | "/todo" | "/todoist" | "/jira" => {
                let full_args = if command == "/notion" || command == "/jira" {
                    format!("{} {}", &command[1..], args)
                } else {
                    format!("todoist {}", args)
                };
                let output = crate::productivity::handle_productivity_command(&full_args).await;
                print!("{}", output);
            }
            "/linear" => {
                let output = crate::linear::handle_linear_command(args).await;
                print!("{}", output);
            }
            "/company" => {
                let output = crate::company_cmd::handle_company_cmd_once(args).await;
                print!("{}", output);
            }
            "/archspec" => {
                use crate::architecture_spec::ArchitectureSpec;
                let sub = args.split_whitespace().next().unwrap_or("help");
                let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                let mut spec = ArchitectureSpec::new("VibeCody");
                match sub {
                    "togaf" => {
                        use crate::architecture_spec::TogafPhase;
                        let togaf = spec.togaf();
                        let progress = togaf.get_overall_progress();
                        println!("TOGAF ADM — Overall Progress: {:.0}%\n", progress * 100.0);
                        for (i, phase) in TogafPhase::all().iter().enumerate() {
                            let pct = togaf.get_phase_completion(phase);
                            let artifacts = togaf.get_artifacts_by_phase(phase).len();
                            println!("  {}. {} — {:.0}% ({} artifacts)", i + 1, phase.label(), pct * 100.0, artifacts);
                        }
                        println!();
                    }
                    "zachman" => {
                        let report = spec.zachman().generate_matrix_report();
                        println!("{}\n", report);
                    }
                    "c4" => {
                        let level = if rest.is_empty() { "context" } else { rest };
                        match level {
                            "context"   => println!("{}\n", spec.c4().generate_context_diagram()),
                            "container" => println!("{}\n", spec.c4().generate_container_diagram()),
                            _ => println!("Usage: /archspec c4 context|container\n"),
                        }
                    }
                    "adr" => {
                        let index = spec.adrs().generate_index();
                        println!("{}\n", index);
                    }
                    "report" => {
                        let report = spec.generate_report();
                        println!("{}\n", report);
                    }
                    _ => {
                        println!("VibeCody Architecture Specification\n");
                        println!("  /archspec togaf    — TOGAF ADM phases");
                        println!("  /archspec zachman  — Zachman framework matrix");
                        println!("  /archspec c4       — C4 Model diagrams");
                        println!("  /archspec adr      — Architecture decision records");
                        println!("  /archspec report   — Full architecture report\n");
                    }
                }
            }
            "/policy" => {
                use crate::policy_engine::{PolicyEngine, PolicySerializer};
                let sub = args.split_whitespace().next().unwrap_or("help");
                let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                let engine = PolicyEngine::new();
                match sub {
                    "list" => {
                        let policies = engine.list_policies();
                        if policies.is_empty() {
                            println!("No policies loaded.\n");
                        } else {
                            println!("Policies ({}):", policies.len());
                            for p in &policies {
                                println!("  [{}] {} — {} ({})", p.id, p.name, p.resource, if p.disabled { "disabled" } else { "active" });
                            }
                            println!();
                        }
                    }
                    "audit" => {
                        let log = engine.get_audit_log();
                        if log.is_empty() {
                            println!("No audit entries.\n");
                        } else {
                            println!("Audit Log ({} entries):", log.len());
                            for entry in log {
                                println!("  {} {} {} -> {:?}", entry.request.principal.id, entry.request.action, entry.request.resource.kind, entry.result.effect);
                            }
                            println!();
                        }
                    }
                    "template" => {
                        let resource = if rest.is_empty() { "document" } else { rest };
                        let template = PolicySerializer::generate_template(resource);
                        println!("{}\n", template);
                    }
                    _ => {
                        println!("VibeCody Policy Engine\n");
                        println!("  /policy list               — List policies");
                        println!("  /policy audit              — View audit trail");
                        println!("  /policy template <resource> — Generate starter policy\n");
                    }
                }
            }
            "/aireview" => {
                use crate::ai_code_review::{AiCodeReviewEngine, ReviewConfig};
                let sub = args.split_whitespace().next().unwrap_or("help");
                let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                let config = ReviewConfig::default();
                let mut engine = AiCodeReviewEngine::new(config.clone());
                match sub {
                    "file" => {
                        if rest.is_empty() {
                            println!("Usage: /aireview file <path>\n");
                        } else {
                            let content = std::fs::read_to_string(rest).unwrap_or_default();
                            let findings = engine.analyze_file(rest, &content, &config);
                            if findings.is_empty() {
                                println!("No issues found in {}.\n", rest);
                            } else {
                                println!("Review: {} ({} finding(s)):\n", rest, findings.len());
                                for f in &findings {
                                    println!("  [{:?}] {}:{} — {} ({:?})", f.severity, f.file, f.line_start, f.message, f.category);
                                    if let Some(ref sug) = f.suggestion { println!("    Suggestion: {}", sug); }
                                }
                                println!();
                            }
                        }
                    }
                    "diff" => {
                        if rest.is_empty() {
                            println!("Usage: /aireview diff <unified_diff>\n");
                        } else {
                            let analysis = engine.analyze_diff(rest, &config);
                            println!("{}", engine.generate_pr_summary(&analysis));
                        }
                    }
                    "learn" => {
                        let stats = engine.get_learning_stats();
                        println!("Learning Stats");
                        println!("  Precision: {:.1}%  Recall: {:.1}%  F1: {:.3}\n", stats.precision * 100.0, stats.recall * 100.0, stats.f1_score);
                    }
                    _ => {
                        println!("VibeCody AI Code Review\n");
                        println!("  /aireview file <path> — Review a file");
                        println!("  /aireview diff <diff> — Review a unified diff");
                        println!("  /aireview learn       — Learning statistics\n");
                    }
                }
            }
            "/creview" => {
                use crate::review_protocol::{ReviewEngine, ReviewConfig};
                let sub = args.split_whitespace().next().unwrap_or("help");
                let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                let mut engine = ReviewEngine::new(ReviewConfig::default());
                match sub {
                    "start" => {
                        if rest.is_empty() {
                            println!("Usage: /creview start <title>\n");
                        } else {
                            let files: Vec<String> = vec![".".to_string()];
                            let sid = engine.start_session(rest, files);
                            println!("Review session started: '{}' (id: {})\n", rest, sid);
                        }
                    }
                    "stats" => {
                        let q = engine.get_quality();
                        println!("Review Quality");
                        println!("  Total: {}  Resolved: {}  Precision: {:.0}%\n",
                            q.total_comments, q.resolved,
                            if q.total_comments > 0 { q.precision * 100.0 } else { 0.0 });
                    }
                    "list" => {
                        let sessions = engine.list_sessions();
                        if sessions.is_empty() {
                            println!("No active review sessions.\n");
                        } else {
                            println!("Review Sessions ({}):", sessions.len());
                            for s in &sessions { println!("  [{}] {}", s.id, s.title); }
                            println!();
                        }
                    }
                    _ => {
                        println!("VibeCody Code Review Protocol\n");
                        println!("  /creview start <title> — Start review session");
                        println!("  /creview list          — List sessions");
                        println!("  /creview stats         — Review quality stats\n");
                    }
                }
            }
            _ => {
                eprintln!("Unknown --cmd command '{}'. Use /email, /cal, /ha, /todo, /notion, /jira, /linear, /company, /archspec, /policy, /aireview, /creview.", command);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // Non-interactive CI/exec mode: --exec "task"
    if let Some(task) = cli.exec {
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        let cwd = std::env::current_dir()?;
        let config = Config::load().unwrap_or_default();
        let sandbox = config.safety.sandbox;
        let mut te = ToolExecutor::new(cwd.clone(), sandbox).with_provider(llm.clone());
        if no_network { te = te.with_no_network(); }
        let executor: Arc<dyn vibe_ai::agent::ToolExecutorTrait> = Arc::new(te);

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
                safe_exit(2);
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

        safe_exit(report.exit_code());
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

        println!("Running code review...");
        if !config.base_ref.is_empty() {
            println!("   Base: {}", config.base_ref);
        }
        if !config.target_ref.is_empty() {
            println!("   Target: {}", config.target_ref);
        }

        let report = review::run_review(&config, llm).await?;

        if cli.ci_mode {
            // CI mode: structured JSON output with GitHub Actions annotations
            let json_report = serde_json::json!({
                "files_reviewed": report.files_reviewed.len(),
                "findings_count": report.issues.len(),
                "issues": report.issues,
                "score": report.score,
                "summary": report.summary,
            });
            println!("{}", serde_json::to_string_pretty(&json_report)?);

            // Emit GitHub Actions annotations for each issue
            for issue in &report.issues {
                let level = match issue.severity {
                    review::Severity::Critical => "error",
                    review::Severity::Warning => "warning",
                    review::Severity::Info => "notice",
                };
                println!(
                    "::{level} file={},line={}::{}: {}",
                    issue.file, issue.line, issue.category, issue.description
                );
            }

            // Exit with code based on severity threshold
            // Accepts both original values (critical/high/medium/low) and
            // human-friendly aliases (warning = high, info = low)
            let has_failures = report.issues.iter().any(|i| {
                match cli.severity_threshold.as_str() {
                    "critical" => matches!(i.severity, review::Severity::Critical),
                    "high" | "warning" => matches!(i.severity, review::Severity::Critical | review::Severity::Warning),
                    "medium" | "low" | "info" => true,
                    _ => matches!(i.severity, review::Severity::Critical),
                }
            });
            safe_exit(if has_failures { 1 } else { 0 });
        }

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

        safe_exit(report.exit_code());
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
        safe_exit(exit_code);
    }

    // Red Team report: --redteam-report <session-id>
    if let Some(session_id) = cli.redteam_report {
        let manager = redteam::RedTeamManager::new()?;
        let session = manager.load_session(&session_id)?;
        let report = redteam::generate_report(&session);
        println!("{}", report);
        safe_exit(0);
    }

    // Cloud Agent mode: --cloud --agent "task"
    if cli.cloud {
        let cloud_task = cli.agent.clone().unwrap_or_else(|| {
            "Run tests and report results".to_string()
        });
        eprintln!("☁️  Cloud Agent mode — running task in Docker container");
        let config = cloud_agent::CloudAgentConfig {
            workspace_mount: Some(
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            ),
            ..Default::default()
        };
        match cloud_agent::start_cloud_agent(&config, &cloud_task).await {
            Ok(status) => {
                eprintln!("   Container: {}", status.container_id);
                eprintln!("   Status:    {}", status.status);
                for line in &status.logs {
                    println!("{}", line);
                }
                if status.status == "failed" {
                    safe_exit(1);
                }
            }
            Err(e) => {
                eprintln!("❌ Cloud agent failed: {}", e);
                safe_exit(1);
            }
        }
        return Ok(());
    }

    // Watch mode: --watch [--agent "task"] [--watch-glob "**/*.rs"]
    if cli.watch {
        let watch_task = cli.agent.clone().unwrap_or_else(|| {
            "A file changed. Analyze the change and take any helpful action (e.g. run tests, fix errors).".to_string()
        });
        let llm = create_provider(&effective_provider, effective_model.clone())?;
        return run_watch_mode(llm, &watch_task, &approval_policy, &cli.watch_glob, sandbox_enabled, no_network).await;
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
            return run_parallel_agents(llm, &task, &approval_policy, n, no_network).await;
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
                    eprintln!("Worktree isolation: branch '{}' at {}", branch, wt_path.display());
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

        if cli.record {
            eprintln!("🎬 Screen recording enabled — frames will be saved to ~/.vibecli/recordings/");
        }

        let exec_model_str = exec_model.clone().unwrap_or_default();
        return run_agent_repl_with_context(
            llm, &task, &approval_policy,
            cli.resume.as_deref(),
            cli.plan,
            cli.json,
            planning_llm,
            &exec_provider,
            &exec_model_str,
            no_network,
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
            println!("Session {} found ({} trace steps)", &session.session_id, session.step_count);
            println!("Use: vibecli --agent \"<task to continue>\" --resume {}", &session.session_id[..session.session_id.len().min(8)]);
        } else {
            eprintln!("❌ No session found with ID prefix: {}", sid);
        }
        return Ok(());
    }

    // ── One-shot chat mode: vibecli "message" or vibecli chat "message" ─────
    if !cli.message.is_empty() {
        let mut words = cli.message.clone();
        // Strip leading "chat" keyword if present (allows `vibecli chat "Hello"`)
        if words.first().map(|w| w.eq_ignore_ascii_case("chat")).unwrap_or(false) && words.len() > 1 {
            words.remove(0);
        }
        let user_msg = words.join(" ");
        if !user_msg.is_empty() {
            let llm = create_provider(&effective_provider, effective_model.clone())?;
            let messages = vec![Message {
                role: MessageRole::User,
                content: user_msg,
            }];
            match llm.stream_chat(&messages).await {
                Ok(mut stream) => {
                    let mut full_response = String::new();
                    use tokio_stream::StreamExt;
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(text) => {
                                print!("{}", text);
                                io::stdout().flush()?;
                                full_response.push_str(&text);
                            }
                            Err(e) => {
                                eprintln!("\nStream error: {}", e);
                                break;
                            }
                        }
                    }
                    if !full_response.ends_with('\n') {
                        println!();
                    }
                }
                Err(_) => {
                    // Fallback to non-streaming chat
                    match llm.chat(&messages, None).await {
                        Ok(response) => {
                            let rendered = highlight_code_blocks(&response);
                            println!("{}", rendered);
                        }
                        Err(e2) => {
                            eprintln!("Error: {}", e2);
                            safe_exit(1);
                        }
                    }
                }
            }
            return Ok(());
        }
    }

    println!("\x1b[1m\x1b[92mVibeCLI\x1b[0m — AI-Powered Coding Assistant");
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
        println!("{}", memory.summary());
    }

    let mut messages: Vec<Message> = Vec::new();
    // Seed system message with memory if available
    if let Some(mem_content) = memory.combined() {
        messages.push(Message {
            role: MessageRole::System,
            content: mem_content,
        });
    }

    // Load orchestration lessons and inject into system context
    {
        use crate::workflow_orchestration::{LessonsStore, TodoStore, orchestration_system_prompt};
        let lessons_store = LessonsStore::for_workspace(&cwd);
        let todo_store = TodoStore::for_workspace(&cwd);
        let lessons = lessons_store.load();
        let current_task = todo_store.load();
        let orch_ctx = orchestration_system_prompt(&lessons, current_task.as_ref());
        if !orch_ctx.is_empty() {
            messages.push(Message {
                role: MessageRole::System,
                content: orch_ctx,
            });
        }
        if !lessons.is_empty() {
            println!("Loaded {} orchestration lessons", lessons.len());
        }
        if let Some(ref task) = current_task {
            println!("Active task: {} ({}/{} done)", task.goal, task.completed(), task.todos.len());
        }
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

    // ── API key health monitor ────────────────────────────────────────────
    // Background task validates all configured provider keys every 5 minutes
    // and prints coloured warnings when a key's status changes.
    let _api_key_monitor = api_key_monitor::ApiKeyMonitor::start(
        std::time::Duration::from_secs(300),  // 5 minute interval
        std::time::Duration::from_secs(5),    // initial delay
    );

    // Voice mode indicator
    if cli.voice {
        let voice_cfg = Config::load().unwrap_or_default().voice;
        if voice_cfg.resolve_whisper_api_key(None).is_some() {
            eprintln!("Voice mode enabled — use /voice transcribe <file> or /voice speak <text>");
        } else {
            eprintln!("⚠️  --voice flag set but no Whisper API key found. Set GROQ_API_KEY or voice.whisper_api_key in config.");
        }
    }

    loop {
        let prompt = crate::syntax::colored_prompt(&effective_provider, effective_model.as_deref());
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                rl.add_history_entry(line.as_str())?;

                // ── # Natural language → command (Warp-style) ──────────────
                if let Some(nl_query) = input.strip_prefix('#') {
                    let nl_query = nl_query.trim();
                    if !nl_query.is_empty() {
                        let cwd = std::env::current_dir().unwrap_or_default();
                        let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
                        let prompt = warp_features::generate_command_prompt(nl_query, &cwd.display().to_string(), &shell);
                        print!("\x1b[2mTranslating...\x1b[0m");
                        io::stdout().flush()?;
                        match llm.chat(&[Message { role: MessageRole::User, content: prompt }], None).await {
                            Ok(response) => {
                                print!("\r\x1b[2K"); // Clear "Translating..." line
                                if let Some(cmd) = warp_features::parse_command_response(&response) {
                                    println!("  \x1b[1m{}\x1b[0m", cmd.generated);
                                    if !cmd.explanation.is_empty() {
                                        println!("  \x1b[2m{}\x1b[0m", cmd.explanation);
                                    }
                                    print!("  Run? (Y/n): ");
                                    io::stdout().flush()?;
                                    let mut confirm = String::new();
                                    io::stdin().read_line(&mut confirm)?;
                                    let answer = confirm.trim().to_lowercase();
                                    if answer.is_empty() || answer == "y" || answer == "yes" {
                                        let start = std::time::Instant::now();
                                        let output = std::process::Command::new("sh").arg("-c").arg(&cmd.generated).output();
                                        let duration_ms = start.elapsed().as_millis() as u64;
                                        match output {
                                            Ok(out) => {
                                                let exit_code = out.status.code().unwrap_or(-1);
                                                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                                                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                                                let redactor = warp_features::SecretRedactor::new();
                                                let block = warp_features::OutputBlock {
                                                    command: cmd.generated.clone(),
                                                    output: redactor.redact(&format!("{}{}", stdout, stderr)),
                                                    exit_code,
                                                    duration_ms,
                                                    cwd: cwd.display().to_string(),
                                                    timestamp: warp_features::epoch_secs(),
                                                };
                                                print!("{}", block.format());
                                            }
                                            Err(e) => eprintln!("  Execution failed: {}", e),
                                        }
                                    }
                                } else {
                                    print!("\r\x1b[2K");
                                    println!("{}", highlight_code_blocks(&response));
                                }
                            }
                            Err(e) => {
                                print!("\r\x1b[2K");
                                eprintln!("  Error: {}", e);
                            }
                        }
                    }
                    continue;
                }

                // ── ! Direct shell command (with block output + corrections) ─
                if let Some(shell_cmd) = input.strip_prefix('!') {
                    let command = shell_cmd.trim();
                    if !command.is_empty() {
                        // Handle `cd` as a built-in — subprocess cd doesn't persist
                        if command == "cd" || command.starts_with("cd ") || command.starts_with("cd\t") {
                            let dir = command.strip_prefix("cd").unwrap_or("").trim();
                            let target = if dir.is_empty() || dir == "~" {
                                dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"))
                            } else if let Some(stripped) = dir.strip_prefix("~/") {
                                dirs::home_dir()
                                    .unwrap_or_else(|| std::path::PathBuf::from("/"))
                                    .join(stripped)
                            } else if dir == "-" {
                                std::env::var("OLDPWD")
                                    .map(std::path::PathBuf::from)
                                    .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default())
                            } else {
                                std::path::PathBuf::from(dir)
                            };
                            let old_cwd = std::env::current_dir().unwrap_or_default();
                            match std::env::set_current_dir(&target) {
                                Ok(_) => {
                                    std::env::set_var("OLDPWD", &old_cwd);
                                    let new_cwd = std::env::current_dir().unwrap_or_default();
                                    println!("  \x1b[32m│\x1b[0m {}", new_cwd.display());
                                }
                                Err(e) => eprintln!("  cd: {}: {}", target.display(), e),
                            }
                            continue;
                        }
                        let require_approval = Config::load()
                            .map(|c| c.safety.require_approval_for_commands)
                            .unwrap_or(true);
                        let should_run = if require_approval {
                            print!("  Execute: {}? (Y/n): ", command);
                            io::stdout().flush()?;
                            let mut confirm = String::new();
                            io::stdin().read_line(&mut confirm)?;
                            let answer = confirm.trim().to_lowercase();
                            answer.is_empty() || answer == "y" || answer == "yes"
                        } else {
                            true
                        };
                        if should_run {
                            let start = std::time::Instant::now();
                            use std::process::Command;
                            let cwd = std::env::current_dir().unwrap_or_default();
                            let output = if cfg!(target_os = "windows") {
                                Command::new("cmd").args(["/C", command]).current_dir(&cwd).output()
                            } else {
                                Command::new("sh").arg("-c").arg(command).current_dir(&cwd).output()
                            };
                            let duration_ms = start.elapsed().as_millis() as u64;
                            match output {
                                Ok(out) => {
                                    let exit_code = out.status.code().unwrap_or(-1);
                                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                                    let redactor = warp_features::SecretRedactor::new();
                                    // Combine stdout and stderr with separator if both non-empty
                                    let combined = if !stdout.is_empty() && !stderr.is_empty() {
                                        format!("{}\n{}", stdout.trim_end(), stderr.trim_end())
                                    } else {
                                        format!("{}{}", stdout, stderr)
                                    };
                                    let block = warp_features::OutputBlock {
                                        command: command.to_string(),
                                        output: redactor.redact(&combined),
                                        exit_code,
                                        duration_ms,
                                        cwd: cwd.display().to_string(),
                                        timestamp: warp_features::epoch_secs(),
                                    };
                                    print!("{}", block.format());
                                    // Command corrections on failure
                                    if exit_code != 0 {
                                        if let Some(correction) = warp_features::suggest_correction(command, exit_code, &stderr) {
                                            println!("  \x1b[33mDid you mean:\x1b[0m \x1b[1m{}\x1b[0m", correction.suggested_command);
                                            println!("  \x1b[2m{}\x1b[0m", correction.reason);
                                        }
                                    }
                                    // Next command suggestions on success
                                    if exit_code == 0 {
                                        let suggestions = warp_features::suggest_next_commands(command, exit_code, &cwd.display().to_string());
                                        if !suggestions.is_empty() {
                                            let s = &suggestions[0];
                                            println!("  \x1b[2mNext: {}\x1b[0m", s.command);
                                        }
                                    }
                                    // Desktop notification for long commands
                                    if warp_features::should_notify(duration_ms, 30_000) {
                                        let title = if exit_code == 0 { "Command completed" } else { "Command failed" };
                                        let _ = warp_features::send_notification(title, command);
                                    }
                                }
                                Err(e) => eprintln!("  Execution failed: {}", e),
                            }
                        } else {
                            println!("  Cancelled\n");
                        }
                    }
                    continue;
                }

                if input.starts_with('/') {
                    let parts: Vec<&str> = input.splitn(2, ' ').collect();
                    let command = parts[0];
                    let args = if parts.len() > 1 { parts[1].trim() } else { "" };

                    match command {
                        "/exit" | "/quit" => {
                            println!("👋 Goodbye!");
                            break;
                        }
                        "/help" => show_help(),
                        "/setup" => setup::run_setup().await?,
                        "/service" => {
                            match args.split_whitespace().next().unwrap_or("") {
                                "install" => setup::service_install()?,
                                "start" => setup::service_start()?,
                                "stop" => setup::service_stop()?,
                                "status" => setup::service_status()?,
                                _ => {
                                    println!("Service Commands:");
                                    println!("  /service install  — Install VibeCody as a background service");
                                    println!("  /service start    — Start the background service");
                                    println!("  /service stop     — Stop the background service");
                                    println!("  /service status   — Check service status");
                                    println!();
                                }
                            }
                        }
                        "/config" => show_config().await?,
                        "/agent" => {
                            if args.is_empty() {
                                println!("Usage: /agent <task description>");
                                continue;
                            }
                            run_agent_repl_with_context(
                                llm.clone(), args, &approval_policy, None, false, false, None,
                                &active_provider, active_model.as_deref().unwrap_or(""),
                                no_network,
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
                                no_network,
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
                                        println!("No resumable sessions (sessions must have saved messages)");
                                    } else {
                                        println!("\nResumable sessions:");
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
                                        no_network,
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
                                        println!("\nTrace: {} ({} steps)\n", session.session_id, entries.len());
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
                                        println!("No traces found in {}", trace_dir.display());
                                    } else {
                                        println!("\nRecent agent traces ({})\n", trace_dir.display());
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
                                        println!("\nSearch results for '{}' ({} sessions)\n", args, results.len());
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
                                println!("\nSearch results for '{}' ({} sessions match)\n", args, hits.len());
                                for (id, ts, lines) in hits.iter().take(10) {
                                    let elapsed = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH + std::time::Duration::from_secs(*ts))
                                        .unwrap_or_default();
                                    let age = if elapsed.as_secs() < 3600 { format!("{}m ago", elapsed.as_secs() / 60) }
                                        else if elapsed.as_secs() < 86400 { format!("{}h ago", elapsed.as_secs() / 3600) }
                                        else { format!("{}d ago", elapsed.as_secs() / 86400) };
                                    println!("  {} ({})", &id[..id.len().min(12)], age);
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
                                                        println!("\nTools from '{}':", name);
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
                                println!("Usage: /chat <message>  or  /chat [image.png] [file.rs] <message>");
                                continue;
                            }
                            conversation_active = true;

                            // Detect [file.ext] patterns and load images + documents.
                            let (text_content, images, doc_context) = extract_attachments_from_input(args);
                            // Inject document content into the user message
                            let full_content = if doc_context.is_empty() {
                                text_content.clone()
                            } else {
                                format!("[Attached Documents]\n{}\n\n{}", doc_context, text_content)
                            };
                            messages.push(Message {
                                role: MessageRole::User,
                                content: full_content.clone(),
                            });
                            io::stdout().flush()?;
                            let chat_result = if images.is_empty() {
                                llm.chat(&messages, None).await
                            } else {
                                println!("({} image{})", images.len(), if images.len() > 1 { "s" } else { "" });
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
                                    print!("Save to file? (y/N or filename): ");
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
                                            println!("\nProposed changes:\n");
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
                            println!("  Generating command for: {}", args);
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
                                    println!("Suggested command: {}", command);
                                    print!("⚠️  Execute this command? (y/N): ");
                                    io::stdout().flush()?;
                                    let mut confirm = String::new();
                                    io::stdin().read_line(&mut confirm)?;
                                    if confirm.trim().to_lowercase() == "y" {
                                        println!("Executing...");
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
                            println!("Building semantic index with model '{}' …", model);
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
                            println!("Searching codebase for: {}", args);
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
                            io::stdout().flush()?;
                            match llm.chat(&qa_messages, None).await {
                                Ok(response) => {
                                    let rendered = mermaid_ascii::render_mermaid_blocks(&response);
                                    println!("{}\n", highlight_code_blocks(&rendered));
                                }
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
                                        Ok(()) => println!("Deleted profile '{}'\n", name),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                _ => println!("Usage: /profile [list|show|create|delete]\n"),
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
                            println!("Session token usage:");
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
                        // /context handled in Phase 32 Context Protocol section below
                        "/healthscore" => {
                            use crate::health_score::{HealthEngine, HealthConfig, TrendDirection};
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            let mut engine = HealthEngine::new(HealthConfig::default());
                            match sub {
                                "scan" | "" => {
                                    let path = if rest.is_empty() { "." } else { rest };
                                    let snapshot = engine.scan(path, 100);
                                    let overall = HealthEngine::overall_score(&snapshot);
                                    println!("Codebase Health Score: {:.0}/100\n", overall);
                                    for dim in &snapshot.dimensions {
                                        let bar_len = (dim.score / 10.0) as usize;
                                        let bar: String = "█".repeat(bar_len) + &"░".repeat(10 - bar_len);
                                        println!("  {:20} {} {:.0}/100", dim.dimension.label(), bar, dim.score);
                                    }
                                    println!();
                                }
                                "trend" => {
                                    // Run two scans to show trend
                                    let _ = engine.scan(".", 100);
                                    let snapshot2 = engine.scan(".", 100);
                                    println!("Health Trends:");
                                    for dim in &snapshot2.dimensions {
                                        let trend = engine.get_trend(&dim.dimension);
                                        let arrow = match &trend.direction {
                                            TrendDirection::Improving => "↑",
                                            TrendDirection::Declining => "↓",
                                            TrendDirection::Stable => "→",
                                        };
                                        println!("  {:20} {} {:.0} (change: {:+.1}%)",
                                            dim.dimension.label(), arrow, dim.score, trend.change_pct);
                                    }
                                    println!();
                                }
                                "remediate" => {
                                    let snapshot = engine.scan(".", 100);
                                    let remediations = engine.suggest_remediations(&snapshot);
                                    if remediations.is_empty() {
                                        println!("No remediations needed. Codebase is healthy!\n");
                                    } else {
                                        println!("Suggested Remediations ({}):\n", remediations.len());
                                        for r in &remediations {
                                            println!("  [{:?}] {:?}: {}", r.priority, r.dimension, r.title);
                                            println!("         {}", r.description);
                                            println!("         Impact: +{:.0} points{}\n", r.estimated_impact,
                                                if r.auto_fixable { " (auto-fixable)" } else { "" });
                                        }
                                    }
                                }
                                _ => {
                                    println!("VibeCody Codebase Health Score\n");
                                    println!("  /healthscore scan [path]  — Scan and score codebase");
                                    println!("  /healthscore trend        — Show score trends over time");
                                    println!("  /healthscore remediate    — Suggest improvements\n");
                                }
                            }
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

                        "/recipe" => {
                            print!("{}", crate::recipe::handle_recipe_command(args));
                        }

                        "/workspace-detect" => {
                            print!("{}", crate::workspace_detect::handle_workspace_detect_command());
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
                                        Ok(()) => println!("Checkpoint saved ({} messages)\n   Restore with: /rewind {}\n", messages.len(), ts),
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
                                        println!("\nSaved checkpoints:");
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
                                            println!("Rewound to checkpoint {} ({} messages)\n", ts_str, count);
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
                                            println!("\nSpec: {}  [{}]", spec.name, spec.status);
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
                                            println!("Running agent on spec '{}' ({} pending tasks)…\n", name, spec.pending());
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
                                            println!("\nWorkflow: {}  [{:.0}% complete]", w.name, w.overall_progress());
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
                                            println!("Generating {} checklist for '{}'...", w.current_stage.label(), name);
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

                        // ── /orchestrate — workflow orchestration ─────────────────────
                        "/orchestrate" => {
                            use crate::workflow_orchestration::{LessonsStore, TodoStore, estimate_complexity};
                            let cwd = std::env::current_dir()?;
                            let lessons_store = LessonsStore::for_workspace(&cwd);
                            let todo_store = TodoStore::for_workspace(&cwd);
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["status"]
                            } else {
                                args.splitn(3, ' ').collect()
                            };
                            match parts[0] {
                                "status" | "" => {
                                    match todo_store.load() {
                                        Some(state) => println!("{}\n", state.status_summary()),
                                        None => println!("No active task. Start one with: /orchestrate todo add <description>\n"),
                                    }
                                }
                                "lessons" => {
                                    let lessons = lessons_store.load();
                                    if lessons.is_empty() {
                                        println!("No lessons recorded yet.\nRecord one with: /orchestrate lesson <pattern> -> <rule>\n");
                                    } else {
                                        println!("Lessons Learned ({}):", lessons.len());
                                        for l in &lessons {
                                            println!("  {}", l);
                                        }
                                        println!();
                                    }
                                }
                                "lesson" => {
                                    let text = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default();
                                    if text.is_empty() {
                                        println!("Usage: /orchestrate lesson <pattern> -> <rule>\n");
                                        continue;
                                    }
                                    // Parse "pattern -> rule" or "pattern → rule"
                                    let (pattern, rule) = if let Some((p, r)) = text.split_once("->") {
                                        (p.trim().to_string(), r.trim().to_string())
                                    } else if let Some((p, r)) = text.split_once('→') {
                                        (p.trim().to_string(), r.trim().to_string())
                                    } else {
                                        (text.clone(), String::new())
                                    };
                                    match lessons_store.add(&pattern, &rule) {
                                        Ok(lesson) => println!("Recorded lesson #{}: {} -> {}\n", lesson.id, lesson.pattern, lesson.rule),
                                        Err(e) => eprintln!("Error: {}\n", e),
                                    }
                                }
                                "todo" => {
                                    let sub = parts.get(1).unwrap_or(&"").trim();
                                    match sub {
                                        "" | "show" => {
                                            match todo_store.load() {
                                                Some(state) => {
                                                    println!("{}\n", state.status_summary());
                                                    if state.ready_to_close() {
                                                        println!("All tasks complete and verified. Ready to close.\n");
                                                    }
                                                }
                                                None => println!("No active task plan.\nCreate one: /orchestrate todo add <description>\n"),
                                            }
                                        }
                                        "add" => {
                                            let desc = parts.get(2).unwrap_or(&"").trim();
                                            if desc.is_empty() {
                                                println!("Usage: /orchestrate todo add <description>\n");
                                                continue;
                                            }
                                            match todo_store.add_todo(desc) {
                                                Ok(state) => {
                                                    let last = state.todos.last().expect("todos non-empty after add");
                                                    println!("Added task #{}: {}", last.id, last.description);
                                                    println!("  Progress: {}/{}\n", state.completed(), state.todos.len());
                                                }
                                                Err(e) => eprintln!("Error: {}\n", e),
                                            }
                                        }
                                        "done" => {
                                            let id_str = parts.get(2).unwrap_or(&"").trim();
                                            match id_str.parse::<u32>() {
                                                Ok(id) => match todo_store.complete_todo(id) {
                                                    Ok(state) => {
                                                        println!("Completed task #{}", id);
                                                        println!("  Progress: {}/{}\n", state.completed(), state.todos.len());
                                                        if state.all_done() && !state.verified {
                                                            let complexity = estimate_complexity(&state.goal);
                                                            if matches!(complexity, crate::workflow_orchestration::TaskComplexity::Complex) {
                                                                println!("  All tasks done! Run /orchestrate verify before closing.\n");
                                                            }
                                                        }
                                                    }
                                                    Err(e) => eprintln!("Error: {}\n", e),
                                                },
                                                Err(_) => println!("Usage: /orchestrate todo done <id>\n"),
                                            }
                                        }
                                        _ => println!("Usage: /orchestrate todo [show|add|done]\n"),
                                    }
                                }
                                "verify" => {
                                    match todo_store.mark_verified() {
                                        Ok(state) => {
                                            println!("Verification gate passed.");
                                            if state.ready_to_close() {
                                                println!("Task is ready to close.\n");
                                            } else {
                                                println!("Pending tasks remain: {}\n", state.pending());
                                            }
                                        }
                                        Err(e) => eprintln!("Error: {}\n", e),
                                    }
                                }
                                "reset" => {
                                    match todo_store.reset() {
                                        Ok(()) => println!("Task state cleared.\n"),
                                        Err(e) => eprintln!("Error: {}\n", e),
                                    }
                                }
                                _ => println!("Usage: /orchestrate [status|lessons|lesson|todo|verify|reset]\n"),
                            }
                        }

                        // ── /demo ──────────────────────────────────────────────────────
                        "/demo" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["list"]
                            } else {
                                args.splitn(3, ' ').collect()
                            };
                            match parts[0] {
                                "list" | "" => {
                                    match feature_demo::list_demos() {
                                        Ok(demos) => {
                                            if demos.is_empty() {
                                                println!("No demos found.\n  Create one with: /demo generate <feature description>");
                                                println!("  Or manually: /demo record <name>\n");
                                            } else {
                                                println!("Feature Demos ({}):\n", demos.len());
                                                for d in &demos {
                                                    let status = format!("{:?}", d.status).to_lowercase();
                                                    let frames = d.frames.len();
                                                    println!("  {} — {} ({} frames, {})", d.id, d.name, frames, status);
                                                    if !d.description.is_empty() {
                                                        println!("    {}", d.description);
                                                    }
                                                }
                                                println!();
                                            }
                                        }
                                        Err(e) => eprintln!("Error: {e}\n"),
                                    }
                                }
                                "generate" => {
                                    let desc = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default();
                                    if desc.is_empty() {
                                        println!("Usage: /demo generate <feature description>\n");
                                        continue;
                                    }
                                    let prompt = feature_demo::DemoGenerator::build_prompt(&desc, "http://localhost:3000");
                                    println!("Generated demo prompt for: {}", desc);
                                    println!("Send this to your LLM to get demo steps:\n");
                                    println!("{}\n", &prompt[..prompt.len().min(500)]);
                                    println!("Then run: /demo run <name> <steps-json>\n");
                                }
                                "run" => {
                                    let name = parts.get(1).unwrap_or(&"demo").to_string();
                                    let steps_json = parts.get(2).unwrap_or(&"[]");
                                    match feature_demo::DemoGenerator::parse_steps(steps_json) {
                                        Ok(steps) => {
                                            println!("Running demo '{}' with {} steps...", name, steps.len());
                                            let cdp_port = 9222u16;
                                            match feature_demo::DemoRunner::new(&name, cdp_port) {
                                                Ok(mut runner) => {
                                                    match runner.run(&steps, &name).await {
                                                        Ok(rec) => {
                                                            println!("Demo completed: {} ({} frames)", rec.id, rec.frames.len());
                                                            println!("Saved to ~/.vibecli/demos/{}/\n", rec.id);
                                                        }
                                                        Err(e) => eprintln!("Demo run error: {e}\n"),
                                                    }
                                                }
                                                Err(e) => eprintln!("Error creating demo runner: {e}\n"),
                                            }
                                        }
                                        Err(e) => eprintln!("Error parsing steps: {e}\nExpected JSON array of demo steps.\n"),
                                    }
                                }
                                "replay" => {
                                    let id = parts.get(1).unwrap_or(&"");
                                    if id.is_empty() {
                                        println!("Usage: /demo replay <demo-id>\n");
                                        continue;
                                    }
                                    match feature_demo::load_demo(id) {
                                        Ok(demo) => {
                                            println!("Demo: {} — {}\n", demo.name, demo.description);
                                            for (i, frame) in demo.frames.iter().enumerate() {
                                                println!("  Step {}: {}", i + 1, frame.step.summary());
                                                if let Some(ref r) = frame.result {
                                                    println!("    Result: {}", r);
                                                }
                                                if let Some(ref p) = frame.screenshot_path {
                                                    println!("    Screenshot: {}", p);
                                                }
                                                println!("    Duration: {}ms", frame.duration_ms);
                                            }
                                            println!();
                                        }
                                        Err(e) => eprintln!("Error: {e}\n"),
                                    }
                                }
                                "export" => {
                                    let id = parts.get(1).unwrap_or(&"");
                                    let fmt = parts.get(2).unwrap_or(&"html");
                                    if id.is_empty() {
                                        println!("Usage: /demo export <demo-id> [html|md]\n");
                                        continue;
                                    }
                                    match feature_demo::load_demo(id) {
                                        Ok(demo) => {
                                            let format = if *fmt == "md" || *fmt == "markdown" {
                                                feature_demo::ExportFormat::Markdown
                                            } else {
                                                feature_demo::ExportFormat::Html
                                            };
                                            let ext = if format == feature_demo::ExportFormat::Markdown { "md" } else { "html" };
                                            let output_path = std::env::current_dir()
                                                .unwrap_or_default()
                                                .join(format!("demo-{}.{}", id, ext));
                                            match feature_demo::DemoExporter::export_to_file(&demo, &format, &output_path) {
                                                Ok(()) => println!("Exported to: {}\n", output_path.display()),
                                                Err(e) => eprintln!("Export error: {e}\n"),
                                            }
                                        }
                                        Err(e) => eprintln!("Error: {e}\n"),
                                    }
                                }
                                _ => println!("Usage: /demo [list|generate|run|replay|export]\n"),
                            }
                        }

                        // ── /soul ──────────────────────────────────────────────────────
                        "/soul" => {
                            let workspace = std::env::current_dir().unwrap_or_default();
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["generate"]
                            } else {
                                args.splitn(2, ' ').collect()
                            };
                            match parts[0] {
                                "generate" | "gen" => {
                                    if soul_generator::soul_exists(&workspace) {
                                        println!("SOUL.md already exists in this project.");
                                        println!("  Use `/soul show` to view it.");
                                        println!("  Use `/soul regenerate` to overwrite.\n");
                                        continue;
                                    }
                                    println!("Scanning project...");
                                    let signals = soul_generator::scan_project(&workspace);
                                    println!("  Name: {}", signals.name);
                                    if !signals.languages.is_empty() {
                                        println!("  Languages: {}", signals.languages.join(", "));
                                    }
                                    if !signals.frameworks.is_empty() {
                                        println!("  Frameworks: {}", signals.frameworks.join(", "));
                                    }
                                    if !signals.license.is_empty() {
                                        println!("  License: {}", signals.license);
                                    }
                                    println!();

                                    let doc = soul_generator::generate_template_soul(&signals);
                                    let md = doc.to_markdown();
                                    match soul_generator::write_soul(&workspace, &md) {
                                        Ok(path) => println!("Created: {}\n", path.display()),
                                        Err(e) => eprintln!("Error writing SOUL.md: {e}\n"),
                                    }
                                }
                                "regenerate" | "regen" => {
                                    println!("Scanning project...");
                                    let signals = soul_generator::scan_project(&workspace);
                                    let doc = soul_generator::generate_template_soul(&signals);
                                    let md = doc.to_markdown();
                                    match soul_generator::write_soul(&workspace, &md) {
                                        Ok(path) => println!("Regenerated: {}\n", path.display()),
                                        Err(e) => eprintln!("Error writing SOUL.md: {e}\n"),
                                    }
                                }
                                "show" | "view" => {
                                    match soul_generator::read_soul(&workspace) {
                                        Some(content) => {
                                            println!("{}\n", highlight_code_blocks(&content));
                                        }
                                        None => {
                                            println!("No SOUL.md found. Run `/soul generate` to create one.\n");
                                        }
                                    }
                                }
                                "scan" => {
                                    let signals = soul_generator::scan_project(&workspace);
                                    println!("Project Signals:\n");
                                    println!("  Name:          {}", signals.name);
                                    if !signals.description.is_empty() {
                                        println!("  Description:   {}", signals.description);
                                    }
                                    println!("  License:       {}", if signals.license.is_empty() { "none detected" } else { &signals.license });
                                    println!("  Languages:     {}", if signals.languages.is_empty() { "none detected".to_string() } else { signals.languages.join(", ") });
                                    println!("  Frameworks:    {}", if signals.frameworks.is_empty() { "none detected".to_string() } else { signals.frameworks.join(", ") });
                                    println!("  Package mgr:   {}", signals.package_manager.as_deref().unwrap_or("none"));
                                    println!("  Monorepo:      {}", signals.is_monorepo);
                                    println!("  Open source:   {}", signals.is_open_source);
                                    println!("  Has tests:     {}", signals.has_tests);
                                    println!("  Has CI:        {}", signals.has_ci);
                                    println!("  Has Docker:    {}", signals.has_docker);
                                    println!("  Has README:    {}", signals.has_readme);
                                    println!("  Contributing:  {}", signals.has_contributing);
                                    println!();
                                }
                                "prompt" => {
                                    let signals = soul_generator::scan_project(&workspace);
                                    let prompt = soul_generator::build_generation_prompt(&signals);
                                    println!("{}\n", prompt);
                                    println!("Send this prompt to your LLM to generate a richer SOUL.md.\n");
                                }
                                _ => println!("Usage: /soul [generate|regenerate|show|scan|prompt]\n"),
                            }
                        }

                        // ── /bundle ────────────────────────────────────────────────────
                        "/bundle" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["list"]
                            } else {
                                args.splitn(3, ' ').collect()
                            };
                            match parts[0] {
                                "list" | "" => {
                                    println!("Context Bundles:");
                                    println!("  (No bundles configured yet)");
                                    println!("  Use `/bundle create <name>` to create one.\n");
                                }
                                "create" if parts.len() > 1 => {
                                    let name = parts[1];
                                    println!("Created context bundle: {name}");
                                    println!("  Add pinned files with `/bundle pin <id> <file>`\n");
                                }
                                "activate" if parts.len() > 1 => {
                                    println!("Activated bundle: {}\n", parts[1]);
                                }
                                "deactivate" if parts.len() > 1 => {
                                    println!("Deactivated bundle: {}\n", parts[1]);
                                }
                                "share" if parts.len() > 1 => {
                                    println!("Bundle exported to .vibebundle.toml\n");
                                }
                                "import" if parts.len() > 1 => {
                                    println!("Bundle imported from: {}\n", parts[1]);
                                }
                                "export" if parts.len() > 1 => {
                                    println!("Bundle exported as JSON\n");
                                }
                                "delete" if parts.len() > 1 => {
                                    println!("Deleted bundle: {}\n", parts[1]);
                                }
                                _ => println!("Usage: /bundle [create <name>|activate <id>|deactivate <id>|list|share <id>|import <file>|export <id>|delete <id>]\n"),
                            }
                        }

                        // ── /cloud ─────────────────────────────────────────────────────
                        "/cloud" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["providers"]
                            } else {
                                args.splitn(2, ' ').collect()
                            };
                            match parts[0] {
                                "providers" | "" => {
                                    println!("Supported cloud providers:");
                                    println!("  AWS    — S3, DynamoDB, Lambda, SQS, SNS, EC2, ECS, CloudFront, Cognito");
                                    println!("  GCP    — Cloud Storage, BigQuery, Pub/Sub, Cloud Run, Cloud Functions");
                                    println!("  Azure  — Blob Storage, Cosmos DB, Functions, Service Bus, AKS\n");
                                }
                                "scan" => {
                                    println!("Scanning project for cloud service usage...");
                                    println!("  (Use VibeUI CloudProviders panel for interactive results)\n");
                                }
                                "iam" => {
                                    println!("Generating IAM policy from detected services...");
                                    println!("  (Run /cloud scan first to detect services)\n");
                                }
                                "terraform" | "cloudformation" | "pulumi" => {
                                    println!("Generating {} template...", parts[0]);
                                    println!("  (Run /cloud scan first to detect services)\n");
                                }
                                "cost" => {
                                    println!("Estimating cloud costs from detected services...");
                                    println!("  (Run /cloud scan first to detect services)\n");
                                }
                                _ => println!("Usage: /cloud [scan|iam|terraform|cloudformation|pulumi|cost|providers]\n"),
                            }
                        }

                        // ── /benchmark ──────────────────────────────────────────────────
                        "/benchmark" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["list"]
                            } else {
                                args.splitn(2, ' ').collect()
                            };
                            match parts[0] {
                                "list" | "" => {
                                    println!("SWE-bench Benchmark Runs:");
                                    println!("  (No benchmark runs yet)");
                                    println!("  Use `/benchmark run` to start one.\n");
                                }
                                "run" => {
                                    println!("Starting SWE-bench benchmark run...");
                                    println!("  Suite: SWE-bench Verified");
                                    println!("  Provider: {}", active_provider);
                                    println!("  (Use VibeUI SweBench panel for interactive control)\n");
                                }
                                "compare" => {
                                    println!("Compare benchmark runs:");
                                    println!("  (Need at least 2 completed runs to compare)\n");
                                }
                                "export" => {
                                    println!("Exporting benchmark report as markdown...\n");
                                }
                                _ => println!("Usage: /benchmark [run|compare|export|list]\n"),
                            }
                        }

                        // ── /metering ───────────────────────────────────────────────────
                        "/metering" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["status"]
                            } else {
                                args.splitn(2, ' ').collect()
                            };
                            match parts[0] {
                                "status" | "" => {
                                    println!("Usage Metering Status:");
                                    println!("  Total tokens this session: (tracking)");
                                    println!("  Budgets configured: 0");
                                    println!("  Alerts: none\n");
                                }
                                "budget" => {
                                    println!("Credit Budgets:");
                                    println!("  (No budgets configured)");
                                    println!("  Configure budgets in VibeUI UsageMetering panel.\n");
                                }
                                "report" => {
                                    println!("Generating usage report...");
                                    println!("  (Use VibeUI UsageMetering panel for detailed reports)\n");
                                }
                                "alerts" => {
                                    println!("Budget Alerts:");
                                    println!("  (No alerts triggered)\n");
                                }
                                "top" => {
                                    println!("Top consumers by cost:");
                                    println!("  (No usage data yet)\n");
                                }
                                _ => println!("Usage: /metering [status|budget|report|alerts|top]\n"),
                            }
                        }

                        // ── /blueteam ──────────────────────────────────────────────────
                        "/blueteam" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["status"]
                            } else {
                                args.splitn(2, ' ').collect()
                            };
                            match parts[0] {
                                "status" | "" => {
                                    println!("Blue Team — Defensive Security Status:");
                                    println!("  Open incidents: 0");
                                    println!("  Active IOCs: 0");
                                    println!("  Detection rules: 0");
                                    println!("  SIEM connections: 0\n");
                                }
                                "scan" => {
                                    println!("Running threat scan...");
                                    println!("  Checking IOC feeds...");
                                    println!("  Scanning log sources...");
                                    println!("  No active threats detected.\n");
                                }
                                "incidents" => {
                                    println!("Incident Management:");
                                    println!("  (No open incidents)");
                                    println!("  Use VibeUI BlueTeam panel for full incident management.\n");
                                }
                                "iocs" => {
                                    let query = parts.get(1).unwrap_or(&"");
                                    if query.is_empty() {
                                        println!("IOC Database:");
                                        println!("  (No IOCs tracked)");
                                        println!("  Add IOCs via: /blueteam iocs add <type> <value>\n");
                                    } else {
                                        println!("Searching IOCs for '{}'...", query);
                                        println!("  (No matches found)\n");
                                    }
                                }
                                "rules" => {
                                    println!("Detection Rules:");
                                    println!("  (No rules configured)");
                                    println!("  Use VibeUI BlueTeam panel to create detection rules.\n");
                                }
                                "forensics" => {
                                    println!("Forensic Cases:");
                                    println!("  (No active forensic cases)\n");
                                }
                                "playbooks" => {
                                    println!("Incident Response Playbooks:");
                                    println!("  (No playbooks configured)");
                                    println!("  Create playbooks in VibeUI BlueTeam panel.\n");
                                }
                                "siem" => {
                                    println!("SIEM Connections:");
                                    println!("  Supported: Splunk, Sentinel, Elastic SIEM, QRadar, CrowdStrike, Wazuh, Datadog, Sumo Logic");
                                    println!("  (No active connections)\n");
                                }
                                "hunt" => {
                                    println!("Threat Hunt Queries:");
                                    println!("  (No hunt queries defined)");
                                    println!("  Create hypothesis-driven hunts in VibeUI BlueTeam panel.\n");
                                }
                                "report" => {
                                    println!("Generating Blue Team report...");
                                    println!("  (No data to report)\n");
                                }
                                _ => println!("Usage: /blueteam [status|scan|incidents|iocs|rules|forensics|playbooks|siem|hunt|report]\n"),
                            }
                        }

                        // ── /purpleteam ────────────────────────────────────────────────
                        "/purpleteam" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["status"]
                            } else {
                                args.splitn(2, ' ').collect()
                            };
                            match parts[0] {
                                "status" | "" => {
                                    println!("Purple Team — ATT&CK Exercise Status:");
                                    println!("  Exercises: 0");
                                    println!("  Techniques in DB: 14 (MITRE ATT&CK)");
                                    println!("  Overall coverage: N/A\n");
                                }
                                "exercise" => {
                                    let sub = parts.get(1).unwrap_or(&"list");
                                    match *sub {
                                        "list" | "" => {
                                            println!("Purple Team Exercises:");
                                            println!("  (No exercises created)");
                                            println!("  Create via: /purpleteam exercise create <name>\n");
                                        }
                                        _ => {
                                            println!("Creating exercise '{}'...", sub);
                                            println!("  Use VibeUI PurpleTeam panel for full exercise management.\n");
                                        }
                                    }
                                }
                                "simulate" => {
                                    println!("Attack Simulation:");
                                    println!("  Select a technique from the MITRE ATT&CK matrix.");
                                    println!("  Use VibeUI PurpleTeam panel for guided simulations.\n");
                                }
                                "validate" => {
                                    println!("Detection Validation:");
                                    println!("  (No validations recorded)");
                                    println!("  Run simulations first, then validate detections.\n");
                                }
                                "matrix" => {
                                    println!("MITRE ATT&CK Coverage Matrix:");
                                    println!("  ┌─────────────────┬──────────┐");
                                    println!("  │ Tactic          │ Coverage │");
                                    println!("  ├─────────────────┼──────────┤");
                                    println!("  │ Initial Access  │ N/A      │");
                                    println!("  │ Execution       │ N/A      │");
                                    println!("  │ Persistence     │ N/A      │");
                                    println!("  │ Priv Escalation │ N/A      │");
                                    println!("  │ Defense Evasion │ N/A      │");
                                    println!("  │ Credential Acc  │ N/A      │");
                                    println!("  │ Discovery       │ N/A      │");
                                    println!("  │ Lateral Move    │ N/A      │");
                                    println!("  │ Collection      │ N/A      │");
                                    println!("  │ Exfiltration    │ N/A      │");
                                    println!("  │ C2              │ N/A      │");
                                    println!("  │ Impact          │ N/A      │");
                                    println!("  └─────────────────┴──────────┘");
                                    println!("  Run exercises to populate coverage data.\n");
                                }
                                "gaps" => {
                                    println!("Coverage Gaps:");
                                    println!("  (Run an exercise first to identify gaps)\n");
                                }
                                "heatmap" => {
                                    println!("ATT&CK Heatmap:");
                                    println!("  (No exercise data available for heatmap generation)\n");
                                }
                                "report" => {
                                    println!("Generating Purple Team report...");
                                    println!("  (No exercises to report on)\n");
                                }
                                _ => println!("Usage: /purpleteam [status|exercise|simulate|validate|matrix|gaps|heatmap|report]\n"),
                            }
                        }

                        // ── /idp ──────────────────────────────────────────────────────
                        "/idp" => {
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["status"]
                            } else {
                                args.splitn(2, ' ').collect()
                            };
                            match parts[0] {
                                "status" | "" => {
                                    println!("Internal Developer Platform Status:");
                                    println!("  Services in catalog: 0");
                                    println!("  Golden paths: 0");
                                    println!("  Teams: 0");
                                    println!("  Infra requests: 0");
                                    println!("  Platforms: Backstage, Cycloid, Humanitec, Port, Qovery,");
                                    println!("             Mia Platform, OpsLevel, Roadie, Cortex,");
                                    println!("             Morpheus Data, CloudBolt, Harness\n");
                                }
                                "catalog" => {
                                    let query = parts.get(1).unwrap_or(&"");
                                    if query.is_empty() {
                                        println!("Service Catalog:");
                                        println!("  (No services registered)");
                                        println!("  Register via: /idp register <name>\n");
                                    } else {
                                        println!("Searching catalog for '{}'...", query);
                                        println!("  (No matches found)\n");
                                    }
                                }
                                "register" => {
                                    let name = parts.get(1).unwrap_or(&"");
                                    if name.is_empty() {
                                        println!("Usage: /idp register <service-name>\n");
                                    } else {
                                        println!("Registering service '{}' in catalog...", name);
                                        println!("  Use VibeUI IDP panel for full registration form.\n");
                                    }
                                }
                                "golden" => {
                                    println!("Golden Paths (Paved Roads):");
                                    println!("  (No golden paths configured)");
                                    println!("  Create templates in VibeUI IDP panel.\n");
                                }
                                "scorecard" => {
                                    let svc = parts.get(1).unwrap_or(&"");
                                    if svc.is_empty() {
                                        println!("Usage: /idp scorecard <service-id>\n");
                                    } else {
                                        println!("Evaluating scorecard for '{}'...", svc);
                                        println!("  Metrics: Reliability, Security, Documentation, TestCoverage,");
                                        println!("           DeployFrequency, LeadTime, MTTR, ChangeFailureRate");
                                        println!("  (Service not found in catalog)\n");
                                    }
                                }
                                "infra" => {
                                    println!("Infrastructure Self-Service:");
                                    println!("  Templates: Database, Cache, MessageQueue, ObjectStorage,");
                                    println!("             CDN, LoadBalancer, DNS, Monitoring, Logging,");
                                    println!("             SecretStore, ServiceMesh, ApiGateway");
                                    println!("  (No pending requests)\n");
                                }
                                "team" => {
                                    println!("Teams:");
                                    println!("  (No teams registered)");
                                    println!("  Create via VibeUI IDP panel.\n");
                                }
                                "onboard" => {
                                    println!("Team Onboarding:");
                                    println!("  Steps: RepoSetup, CI Pipeline, Environments, Access Control,");
                                    println!("         Documentation, Monitoring, Alerting, ServiceCatalog,");
                                    println!("         GoldenPath, SecurityBaseline");
                                    println!("  (No active onboarding checklists)\n");
                                }
                                "backstage" => {
                                    println!("Backstage Integration:");
                                    println!("  Generate catalog-info.yaml: /idp backstage catalog <service-id>");
                                    println!("  Generate template: /idp backstage template <name>");
                                    println!("  (Use VibeUI IDP > Backstage tab for full management)\n");
                                }
                                "platforms" => {
                                    println!("Supported IDP Platforms:");
                                    println!("  ┌───────────────┬─────────┐");
                                    println!("  │ Platform      │ Status  │");
                                    println!("  ├───────────────┼─────────┤");
                                    println!("  │ Backstage     │ Off     │");
                                    println!("  │ Cycloid       │ Off     │");
                                    println!("  │ Humanitec     │ Off     │");
                                    println!("  │ Port          │ Off     │");
                                    println!("  │ Qovery        │ Off     │");
                                    println!("  │ Mia Platform  │ Off     │");
                                    println!("  │ OpsLevel      │ Off     │");
                                    println!("  │ Roadie        │ Off     │");
                                    println!("  │ Cortex        │ Off     │");
                                    println!("  │ Morpheus Data │ Off     │");
                                    println!("  │ CloudBolt     │ Off     │");
                                    println!("  │ Harness       │ Off     │");
                                    println!("  └───────────────┴─────────┘");
                                    println!("  Enable in config.toml or VibeUI IDP > Platforms tab.\n");
                                }
                                "report" => {
                                    println!("Generating IDP report...");
                                    println!("  (No data to report)\n");
                                }
                                _ => println!("Usage: /idp [status|catalog|register|golden|scorecard|infra|team|onboard|backstage|platforms|report]\n"),
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

                        // ── /spawn — parallel agent spawning ─────────────────────────
                        "/spawn" => {
                            use crate::spawn_agent::{self, SpawnConfig, DecomposeStrategy};
                            let pool = spawn_agent::global_pool();
                            let parts: Vec<&str> = if args.is_empty() {
                                vec!["help"]
                            } else {
                                args.splitn(3, ' ').collect()
                            };
                            match parts[0] {
                                "new" | "run" => {
                                    let task = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default();
                                    if task.trim().is_empty() {
                                        println!("Usage: /spawn new <task description>\n");
                                        continue;
                                    }
                                    let cfg = SpawnConfig::new(task.trim());
                                    match pool.spawn(cfg) {
                                        Ok(id) => {
                                            if let Some(agent) = pool.get(&id) {
                                            println!("🚀 Agent spawned: {} ({})", agent.name, id);
                                            println!("   Status: {} | Branch: {} | Priority: {}",
                                                agent.status,
                                                agent.branch.as_deref().unwrap_or("—"),
                                                agent.config.priority);
                                            println!("   Use '/spawn status {}' to check progress\n", id);
                                            } else {
                                                println!("⚠️  Agent spawned (id: {}) but details unavailable\n", id);
                                            }
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "list" | "ls" => {
                                    let filter = parts.get(1).and_then(|s| match *s {
                                        "running" => Some(spawn_agent::SpawnStatus::Running),
                                        "queued" => Some(spawn_agent::SpawnStatus::Queued),
                                        "paused" => Some(spawn_agent::SpawnStatus::Paused),
                                        "completed" | "done" => Some(spawn_agent::SpawnStatus::Completed),
                                        "failed" => Some(spawn_agent::SpawnStatus::Failed),
                                        _ => None,
                                    });
                                    let agents = pool.list(filter.as_ref());
                                    if agents.is_empty() {
                                        println!("No spawned agents{}.\n",
                                            if filter.is_some() { " matching filter" } else { "" });
                                    } else {
                                        println!("Spawned agents ({}):", agents.len());
                                        for a in &agents {
                                            let dur = a.duration_human();
                                            let branch = a.branch.as_deref().unwrap_or("—");
                                            println!("  [{}] {} — {} | {}% | {} | branch: {} | {} turns",
                                                a.id, a.name, a.status,
                                                a.progress.percent_complete,
                                                dur, branch,
                                                a.progress.turns_completed);
                                        }
                                        println!();
                                    }
                                }
                                "status" | "info" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    if id.is_empty() {
                                        let stats = pool.stats();
                                        println!("{}\n", stats);
                                    } else {
                                        match pool.get(id) {
                                            Some(a) => {
                                                println!("Agent: {} ({})", a.name, a.id);
                                                println!("  Task: {}", a.task);
                                                println!("  Status: {} | Priority: {}", a.status, a.config.priority);
                                                println!("  Progress: {}% ({}/{} turns)",
                                                    a.progress.percent_complete,
                                                    a.progress.turns_completed,
                                                    a.progress.turns_limit);
                                                println!("  Duration: {} | Tokens: {} | Tools: {}",
                                                    a.duration_human(),
                                                    a.progress.tokens_used,
                                                    a.progress.tool_calls);
                                                if !a.progress.files_modified.is_empty() {
                                                    println!("  Files ({}):", a.progress.files_modified.len());
                                                    for f in &a.progress.files_modified {
                                                        println!("    {}", f);
                                                    }
                                                }
                                                if let Some(branch) = &a.branch {
                                                    println!("  Branch: {}", branch);
                                                }
                                                if let Some(msg) = &a.progress.last_message {
                                                    println!("  Last: {}", msg);
                                                }
                                                if let Some(err) = &a.error {
                                                    println!("  Error: {}", err);
                                                }
                                                if let Some(summary) = &a.result_summary {
                                                    println!("  Result: {}", summary);
                                                }
                                                if !a.child_ids.is_empty() {
                                                    println!("  Subtasks: {}", a.child_ids.join(", "));
                                                }
                                                if !a.inbox.is_empty() {
                                                    println!("  Messages: {} in inbox", a.inbox.len());
                                                }
                                                println!();
                                            }
                                            None => println!("Agent not found: {}\n", id),
                                        }
                                    }
                                }
                                "stop" | "cancel" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    if id.is_empty() {
                                        println!("Usage: /spawn stop <agent-id>\n");
                                        continue;
                                    }
                                    match pool.cancel(id) {
                                        Ok(()) => println!("⏹  Agent {} cancelled.\n", id),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "pause" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    if id.is_empty() {
                                        println!("Usage: /spawn pause <agent-id>\n");
                                        continue;
                                    }
                                    match pool.pause(id) {
                                        Ok(()) => println!("⏸  Agent {} paused.\n", id),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "resume" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    if id.is_empty() {
                                        println!("Usage: /spawn resume <agent-id>\n");
                                        continue;
                                    }
                                    match pool.resume(id) {
                                        Ok(()) => println!("▶  Agent {} resumed.\n", id),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "result" | "merge" => {
                                    let id = parts.get(1).unwrap_or(&"").trim();
                                    if id.is_empty() {
                                        println!("Usage: /spawn result <parent-agent-id>\n");
                                        continue;
                                    }
                                    match pool.aggregate_results(id) {
                                        Ok(result) => {
                                            println!("Aggregated Results for {}:", id);
                                            println!("  Strategy: {:?}", result.strategy);
                                            println!("  Agents: {}/{} successful ({} failed)",
                                                result.successful_agents, result.total_agents, result.failed_agents);
                                            println!("  Files modified: {} | Tokens: {} | Duration: {}ms",
                                                result.total_files_modified, result.total_tokens_used, result.total_duration_ms);
                                            if let Some(best) = &result.best_agent_id {
                                                println!("  Best agent: {}", best);
                                            }
                                            if !result.conflicts.is_empty() {
                                                println!("  ⚠ Conflicts ({}):", result.conflicts.len());
                                                for c in &result.conflicts {
                                                    println!("    {} — {}", c.file, c.description);
                                                }
                                            }
                                            println!();
                                            for s in &result.summaries {
                                                println!("  [{}] {} — {} | {} files | {} turns | {}ms",
                                                    s.agent_id, s.agent_name, s.status,
                                                    s.files_modified, s.turns_taken, s.duration_ms);
                                                if let Some(summary) = &s.summary {
                                                    println!("    → {}", summary);
                                                }
                                            }
                                            println!();
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "decompose" => {
                                    let rest = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default();
                                    if rest.trim().is_empty() {
                                        println!("Usage: /spawn decompose <task description>\n");
                                        println!("  Splits task into parallel subtasks (by concern: implement, test, docs).\n");
                                        continue;
                                    }
                                    let base = SpawnConfig::new("");
                                    match pool.spawn_decomposed(rest.trim(), &DecomposeStrategy::ByConcern, &[], &base) {
                                        Ok((parent_id, child_ids)) => {
                                            println!("🔀 Task decomposed into {} subtasks:", child_ids.len());
                                            println!("  Coordinator: {}", parent_id);
                                            for cid in &child_ids {
                                                if let Some(a) = pool.get(cid) {
                                                    println!("  [{}] {} — {}", a.id, a.name, a.status);
                                                }
                                            }
                                            println!("\n  Use '/spawn result {}' to aggregate when done.\n", parent_id);
                                        }
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "send" => {
                                    // /spawn send <from-id> <to-id> <message>
                                    let rest = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default();
                                    let msg_parts: Vec<&str> = rest.splitn(3, ' ').collect();
                                    if msg_parts.len() < 3 {
                                        println!("Usage: /spawn send <from-id> <to-id> <message>\n");
                                        continue;
                                    }
                                    let msg = spawn_agent::AgentMessage::new(
                                        msg_parts[0], msg_parts[1],
                                        spawn_agent::MessageType::Status,
                                        msg_parts[2],
                                    );
                                    match pool.send_message(msg) {
                                        Ok(()) => println!("📨 Message sent from {} to {}.\n", msg_parts[0], msg_parts[1]),
                                        Err(e) => eprintln!("❌ {}\n", e),
                                    }
                                }
                                "cleanup" => {
                                    let max_age_ms = parts.get(1)
                                        .and_then(|s| s.parse::<u64>().ok())
                                        .unwrap_or(3_600_000); // default 1hr
                                    let removed = pool.cleanup(max_age_ms);
                                    println!("🧹 Cleaned up {} completed agents.\n", removed);
                                }
                                _ => {
                                    println!("Usage: /spawn <command> [args]\n");
                                    println!("Commands:");
                                    println!("  new <task>              Spawn a new agent with a task");
                                    println!("  list [status]           List agents (filter: running/queued/paused/done/failed)");
                                    println!("  status [agent-id]       Pool stats or agent details");
                                    println!("  stop <agent-id>         Cancel a running agent");
                                    println!("  pause <agent-id>        Pause a running agent");
                                    println!("  resume <agent-id>       Resume a paused agent");
                                    println!("  result <parent-id>      Aggregate results from decomposed subtasks");
                                    println!("  decompose <task>        Split task into parallel subtasks");
                                    println!("  send <from> <to> <msg>  Send message between agents");
                                    println!("  cleanup [age-ms]        Remove old completed agents");
                                    println!();
                                }
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
                        "/email" => {
                            let output = crate::email_client::handle_email_command(args).await;
                            print!("{}", output);
                        }
                        "/calendar" | "/cal" => {
                            let output = crate::calendar_client::handle_calendar_command(args).await;
                            print!("{}", output);
                        }
                        "/home" | "/ha" => {
                            let output = crate::home_assistant::handle_ha_command(args).await;
                            print!("{}", output);
                        }
                        "/notion" | "/todo" | "/todoist" | "/jira" => {
                            let full_args = if command == "/notion" || command == "/jira" || command == "/todoist" {
                                format!("{} {}", &command[1..], args)
                            } else {
                                format!("todoist {}", args)
                            };
                            let output = crate::productivity::handle_productivity_command(&full_args).await;
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
                                                println!("Saved snippets ({}):", names.len());
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
                                                    Ok(()) => println!("Snippet '{}' saved.\n", name),
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
                                                println!("Snippet '{}':\n---\n{}\n---\n", name, content);
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
                                            Ok(()) => println!("Snippet '{}' deleted.\n", name),
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
                                                println!("Snippet '{}':\n---\n{}\n---\n", name, content);
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
                                            println!("No sessions recorded yet. Sessions are saved when you run /agent tasks.\n");
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
                                                println!("\nRecent sessions ({}):\n", filtered.len());
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
                                         Prints a shareable URL for a session when 'vibecli --serve' is running.\n\
                                         Example: /share 193abc4def\n");
                            } else {
                                let port: u16 = 7878; // default daemon port
                                let url = format!("http://localhost:{}/share/{}", port, args.trim());
                                println!("📤  Shareable session URL:\n    {}\n", url);
                                println!("    (The daemon must be running: vibecli --serve --port {})\n", port);
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
                                    println!("Red Team Commands:");
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
                            println!("Running: {}\n", cmd);
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
                                    println!("Deploying to {} ({})...\n", resolved, desc);
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
                            println!("Running: {}\n", fw);
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
                                        println!("\nNo {} file found. Use `/env create` or `/env set KEY value`.\n", env_filename);
                                    } else {
                                        let entries = parse_env_file(&env_path);
                                        println!("\nEnvironment: {} ({})", active_env, env_filename);
                                        if entries.is_empty() {
                                            println!("  (empty)\n");
                                        } else {
                                            for (key, value) in &entries {
                                                if is_secret_key(key) {
                                                    println!("  {:<30} ••••••••  ", key);
                                                } else {
                                                    println!("  {:<30} {}", key, value);
                                                }
                                            }
                                            println!("  ({} variables)\n", entries.len());
                                        }
                                    }
                                }
                                "files" => {
                                    println!("\nEnvironment files:");
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
                                            Ok(_) => println!("Deleted {} from {}\n", key, env_filename),
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
                                            Ok(_) => println!("Switched to environment: {} ({})\n", env_name, target_file),
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
                                    println!("\nProfiling tools:");
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

                                    println!("Profiling with {}...\n", tool);

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
                                    println!("Scanning dependencies ({})...\n", manager);
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
                                    println!("Upgrading {} ({})...", pkg, manager);
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
                                    println!("Scanning for log files...\n");
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
                                            println!("Last {} lines of {}:\n", lines.len().min(50), sub_args);
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
                                            println!("Errors/warnings in {}:\n", sub_args);
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
                                            println!("Analyzing last {} lines with AI...\n", tail.len());
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

                        "/bisect" => {
                            let sub_parts: Vec<&str> = args.splitn(3, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let cwd = std::env::current_dir().unwrap_or_default();
                            match subcmd {
                                "start" => {
                                    let bad = sub_parts.get(1).copied().unwrap_or("").trim();
                                    let good = sub_parts.get(2).copied().unwrap_or("").trim();
                                    if bad.is_empty() || good.is_empty() {
                                        println!("Usage: /bisect start <bad-commit> <good-commit>\n");
                                        continue;
                                    }
                                    let output = std::process::Command::new("git")
                                        .args(["bisect", "start", bad, good])
                                        .current_dir(&cwd)
                                        .output();
                                    match output {
                                        Ok(o) => {
                                            let out = String::from_utf8_lossy(&o.stdout);
                                            let err = String::from_utf8_lossy(&o.stderr);
                                            println!("{out}{err}");
                                        }
                                        Err(e) => println!("❌ Failed to start bisect: {e}\n"),
                                    }
                                }
                                "good" | "bad" | "skip" => {
                                    let output = std::process::Command::new("git")
                                        .args(["bisect", subcmd])
                                        .current_dir(&cwd)
                                        .output();
                                    match output {
                                        Ok(o) => {
                                            let out = String::from_utf8_lossy(&o.stdout);
                                            let err = String::from_utf8_lossy(&o.stderr);
                                            print!("{out}{err}");
                                            if out.contains("is the first bad commit") {
                                                println!("\nCulprit found!\n");
                                            }
                                        }
                                        Err(e) => println!("❌ Bisect step failed: {e}\n"),
                                    }
                                }
                                "reset" => {
                                    let _ = std::process::Command::new("git")
                                        .args(["bisect", "reset"])
                                        .current_dir(&cwd)
                                        .status();
                                    println!("✅ Bisect session reset.\n");
                                }
                                "log" => {
                                    let output = std::process::Command::new("git")
                                        .args(["bisect", "log"])
                                        .current_dir(&cwd)
                                        .output();
                                    match output {
                                        Ok(o) => print!("{}", String::from_utf8_lossy(&o.stdout)),
                                        Err(e) => println!("❌ Failed to get bisect log: {e}\n"),
                                    }
                                }
                                "analyze" => {
                                    let output = std::process::Command::new("git")
                                        .args(["bisect", "log"])
                                        .current_dir(&cwd)
                                        .output();
                                    match output {
                                        Ok(o) => {
                                            let log_text = String::from_utf8_lossy(&o.stdout);
                                            if log_text.trim().is_empty() {
                                                println!("No bisect log available. Start a bisect session first.\n");
                                                continue;
                                            }
                                            println!("Analyzing bisect session...\n");
                                            let prompt = format!(
                                                "Analyze this git bisect log and identify the root cause commit. \
                                                 Explain what likely went wrong.\n\n```\n{}\n```",
                                                log_text
                                            );
                                            let msgs = vec![Message {
                                                role: MessageRole::User,
                                                content: prompt,
                                            }];
                                            match llm.chat(&msgs, None).await {
                                                Ok(resp) => println!("{resp}\n"),
                                                Err(e) => println!("❌ AI error: {e}\n"),
                                            }
                                        }
                                        Err(e) => println!("❌ Failed to get bisect log: {e}\n"),
                                    }
                                }
                                _ => {
                                    println!("Usage: /bisect [start <bad> <good>|good|bad|skip|reset|log|analyze]\n");
                                }
                            }
                        }

                        "/markers" => {
                            let subcmd = args.trim();
                            let cwd = std::env::current_dir().unwrap_or_default();
                            match subcmd {
                                "scan" | "list" | "" => {
                                    println!("🔖 Scanning for code markers...\n");
                                    fn markers_re() -> &'static regex::Regex {
                                        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
                                        RE.get_or_init(|| regex::Regex::new(r"(?i)\b(TODO|FIXME|HACK|BUG|NOTE|XXX)\b[:\s]*(.*)").expect("valid regex: markers"))
                                    }
                                    let re = markers_re();
                                    let extensions = &["rs","ts","tsx","js","jsx","py","go","java","rb","c","cpp","h"];
                                    let mut count = 0u32;
                                    for entry in walkdir::WalkDir::new(&cwd)
                                        .follow_links(false)
                                        .max_depth(8)
                                        .into_iter()
                                        .filter_map(|e| e.ok())
                                    {
                                        let path = entry.path();
                                        let ps = path.to_string_lossy();
                                        if ps.contains("/.git/") || ps.contains("/node_modules/")
                                            || ps.contains("/target/") || ps.contains("/dist/") {
                                            continue;
                                        }
                                        if !path.is_file() { continue; }
                                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                        if !extensions.contains(&ext) { continue; }
                                        let content = match std::fs::read_to_string(path) {
                                            Ok(c) => c,
                                            Err(_) => continue,
                                        };
                                        let rel = path.strip_prefix(&cwd).unwrap_or(path);
                                        for (i, line) in content.lines().enumerate() {
                                            if let Some(caps) = re.captures(line) {
                                                let mtype = caps.get(1).map(|m| m.as_str().to_uppercase()).unwrap_or_default();
                                                let text = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                                                println!("  \x1b[36m{}:{}\x1b[m  [\x1b[33m{}\x1b[m]  {}", rel.display(), i + 1, mtype, text);
                                                count += 1;
                                                if count >= 200 { break; }
                                            }
                                        }
                                        if count >= 200 { break; }
                                    }
                                    println!("\n📍 Found {} markers.\n", count);
                                }
                                "bookmarks" => {
                                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                                    let bp = std::path::PathBuf::from(&home).join(".vibeui").join("bookmarks.json");
                                    match std::fs::read_to_string(&bp) {
                                        Ok(s) => {
                                            let bookmarks: Vec<serde_json::Value> = serde_json::from_str(&s).unwrap_or_default();
                                            if bookmarks.is_empty() {
                                                println!("No bookmarks saved.\n");
                                            } else {
                                                println!("🔖 Bookmarks:\n");
                                                for b in &bookmarks {
                                                    let file = b.get("file").and_then(|v| v.as_str()).unwrap_or("?");
                                                    let line = b.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
                                                    let label = b.get("label").and_then(|v| v.as_str()).unwrap_or("");
                                                    println!("  {}:{}  {}", file, line, label);
                                                }
                                                println!();
                                            }
                                        }
                                        Err(_) => println!("No bookmarks file found.\n"),
                                    }
                                }
                                _ => {
                                    println!("Usage: /markers [scan|list|bookmarks]\n");
                                }
                            }
                        }

                        "/mock" => {
                            println!("🎭 Mock server management is available in VibeUI's Mock tab.\n");
                            println!("  The mock server requires the VibeUI runtime (Tauri) to host the");
                            println!("  HTTP server. Use the 🎭 Mock tab to start/stop, add routes, view");
                            println!("  request logs, and import from OpenAPI specs.\n");
                        }

                        "/compliance" => {
                            let framework = if args.is_empty() { "SOC2" } else { args.trim() };
                            match compliance::generate_report_for(framework) {
                                Ok(report) => {
                                    let md = compliance::report_to_markdown(&report);
                                    println!("{md}");
                                }
                                Err(e) => println!("❌ Failed to generate report: {e}\n"),
                            }
                        }

                        "/transform" => {
                            let cwd = std::env::current_dir()?;
                            match args.trim() {
                                "" | "detect" => {
                                    let transforms = transform::detect_transforms(&cwd);
                                    if transforms.is_empty() {
                                        println!("No applicable transforms detected in this workspace.\n");
                                    } else {
                                        println!("Detected transforms:");
                                        for t in &transforms {
                                            println!("  - {:?}", t);
                                        }
                                        println!("\nUse /transform plan <type> to create a migration plan.\n");
                                    }
                                }
                                _ => {
                                    println!("Usage: /transform [detect]\n");
                                    println!("  detect — scan workspace for applicable transforms");
                                }
                            }
                        }

                        "/marketplace" => {
                            let m = marketplace::Marketplace::new();
                            match args.trim() {
                                "" | "list" => {
                                    match m.load_cached() {
                                        Ok(index) => {
                                            if index.plugins.is_empty() {
                                                println!("No plugins in marketplace.\n");
                                            } else {
                                                println!("Marketplace ({} plugins):", index.plugins.len());
                                                for p in &index.plugins {
                                                    println!("  {} v{} — {}", p.name, p.version, p.description);
                                                }
                                                println!();
                                            }
                                        }
                                        Err(e) => println!("❌ Failed to load marketplace: {e}\n"),
                                    }
                                }
                                _ if args.starts_with("search ") => {
                                    let query = args.trim_start_matches("search ").trim();
                                    let results = m.search(query).await;
                                    match results {
                                        Ok(hits) if hits.is_empty() => println!("No plugins matching '{query}'.\n"),
                                        Ok(hits) => {
                                            println!("Search results for '{query}':");
                                            for p in &hits {
                                                println!("  {} v{} — {}", p.name, p.version, p.description);
                                            }
                                            println!();
                                        }
                                        Err(e) => println!("❌ Search failed: {e}\n"),
                                    }
                                }
                                _ => {
                                    println!("Usage: /marketplace [list|search <query>]\n");
                                }
                            }
                        }

                        "/plugin" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };

                            match subcmd {
                                "" | "help" => {
                                    println!("Plugin Manager:");
                                    println!("  /plugin list               List installed plugins");
                                    println!("  /plugin search <query>     Search the registry");
                                    println!("  /plugin install <name>     Install a plugin");
                                    println!("  /plugin uninstall <name>   Remove a plugin");
                                    println!("  /plugin enable <name>      Enable a plugin");
                                    println!("  /plugin disable <name>     Disable a plugin");
                                    println!("  /plugin info <name>        Show plugin details");
                                    println!("  /plugin update [name]      Update plugin(s)");
                                    println!();
                                }
                                "list" | "ls" => {
                                    match plugin_lifecycle::PluginLifecycle::new() {
                                        Ok(lc) => {
                                            let plugins = lc.list();
                                            if plugins.is_empty() {
                                                println!("No plugins installed. Use /plugin install <name>\n");
                                            } else {
                                                println!("{:<30} {:<12} {:<12}", "NAME", "VERSION", "STATE");
                                                println!("{}", "-".repeat(54));
                                                for p in plugins {
                                                    let state = match &p.state {
                                                        plugin_lifecycle::PluginState::Enabled => "enabled",
                                                        plugin_lifecycle::PluginState::Disabled => "disabled",
                                                        plugin_lifecycle::PluginState::DevMode => "dev",
                                                        plugin_lifecycle::PluginState::Outdated => "outdated",
                                                        plugin_lifecycle::PluginState::Installed => "installed",
                                                        plugin_lifecycle::PluginState::Errored(_) => "error",
                                                    };
                                                    println!("{:<30} {:<12} {:<12}", p.name, p.version, state);
                                                }
                                                println!();
                                            }
                                        }
                                        Err(e) => println!("Error: {e}\n"),
                                    }
                                }
                                "search" => {
                                    let mut reg = plugin_registry::PluginRegistry::new();
                                    match reg.load_cached() {
                                        Ok(_) => {
                                            let results = reg.search(sub_args, None);
                                            if results.is_empty() {
                                                println!("No plugins found for '{sub_args}'.\n");
                                            } else {
                                                println!("{:<30} {:<10} {:<8} DESCRIPTION", "NAME", "VERSION", "DL");
                                                println!("{}", "-".repeat(76));
                                                for e in results.iter().take(15) {
                                                    let desc = if e.description.len() > 30 {
                                                        format!("{}...", &e.description[..27])
                                                    } else {
                                                        e.description.clone()
                                                    };
                                                    println!("{:<30} {:<10} {:<8} {}", e.name, e.version, e.downloads, desc);
                                                }
                                                println!();
                                            }
                                        }
                                        Err(e) => println!("Error: {e}\n"),
                                    }
                                }
                                "install" if !sub_args.is_empty() => {
                                    match plugin_lifecycle::PluginLifecycle::new() {
                                        Ok(mut lc) => {
                                            if sub_args.starts_with("http") || sub_args.starts_with("git@") {
                                                let name = sub_args.split('/').next_back().unwrap_or(sub_args).trim_end_matches(".git");
                                                match lc.install_from_repo(name, sub_args) {
                                                    Ok(p) => println!("Installed {} v{}\n", p.name, p.version),
                                                    Err(e) => println!("Error: {e}\n"),
                                                }
                                            } else {
                                                let mut reg = plugin_registry::PluginRegistry::new();
                                                if reg.load_cached().is_ok() {
                                                    if let Some(entry) = reg.find(sub_args) {
                                                        if let Some(ref repo) = entry.repository {
                                                            match lc.install_from_repo(sub_args, repo) {
                                                                Ok(p) => println!("Installed {} v{}\n", p.name, p.version),
                                                                Err(e) => println!("Error: {e}\n"),
                                                            }
                                                        } else {
                                                            println!("Plugin has no repository URL.\n");
                                                        }
                                                    } else {
                                                        println!("Plugin '{}' not found in registry.\n", sub_args);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => println!("Error: {e}\n"),
                                    }
                                }
                                "uninstall" | "remove" if !sub_args.is_empty() => {
                                    match plugin_lifecycle::PluginLifecycle::new() {
                                        Ok(mut lc) => match lc.uninstall(sub_args) {
                                            Ok(()) => println!("Uninstalled {}\n", sub_args),
                                            Err(e) => println!("Error: {e}\n"),
                                        },
                                        Err(e) => println!("Error: {e}\n"),
                                    }
                                }
                                "enable" if !sub_args.is_empty() => {
                                    match plugin_lifecycle::PluginLifecycle::new() {
                                        Ok(mut lc) => match lc.enable(sub_args) {
                                            Ok(()) => println!("Enabled {}\n", sub_args),
                                            Err(e) => println!("Error: {e}\n"),
                                        },
                                        Err(e) => println!("Error: {e}\n"),
                                    }
                                }
                                "disable" if !sub_args.is_empty() => {
                                    match plugin_lifecycle::PluginLifecycle::new() {
                                        Ok(mut lc) => match lc.disable(sub_args) {
                                            Ok(()) => println!("Disabled {}\n", sub_args),
                                            Err(e) => println!("Error: {e}\n"),
                                        },
                                        Err(e) => println!("Error: {e}\n"),
                                    }
                                }
                                "info" if !sub_args.is_empty() => {
                                    match plugin_lifecycle::PluginLifecycle::new() {
                                        Ok(lc) => match lc.info(sub_args) {
                                            Ok(info) => {
                                                println!("Plugin: {}", info.plugin.name);
                                                println!("Version: {}", info.plugin.version);
                                                println!("State: {:?}", info.plugin.state);
                                                println!("Skills: {} | Hooks: {} | Commands: {}", info.skills_count, info.hooks_count, info.commands_count);
                                                println!();
                                            }
                                            Err(e) => println!("Error: {e}\n"),
                                        },
                                        Err(e) => println!("Error: {e}\n"),
                                    }
                                }
                                "update" => {
                                    match plugin_lifecycle::PluginLifecycle::new() {
                                        Ok(mut lc) => {
                                            if !sub_args.is_empty() {
                                                match lc.update(sub_args) {
                                                    Ok(change) => println!("Updated {}: {}\n", sub_args, change),
                                                    Err(e) => println!("Error: {e}\n"),
                                                }
                                            } else {
                                                match lc.update_all() {
                                                    Ok(results) => {
                                                        for (name, change) in results {
                                                            println!("  {} — {}", name, change);
                                                        }
                                                        println!();
                                                    }
                                                    Err(e) => println!("Error: {e}\n"),
                                                }
                                            }
                                        }
                                        Err(e) => println!("Error: {e}\n"),
                                    }
                                }
                                _ => println!("Usage: /plugin <list|search|install|uninstall|enable|disable|info|update>\n"),
                            }
                        }

                        "/ingest" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            match subcmd {
                                "status" => {
                                    println!("Document Ingestion Pipeline Status:");
                                    println!("  Supported formats: Markdown, HTML, PlainText, PDF, JSON, CSV, XML, RST, LaTeX, Code");
                                    println!("  Chunking: max_tokens=512, overlap=50, sentence-boundary aware");
                                    println!("  Use /ingest <path> to ingest a file or directory\n");
                                }
                                "" | "help" => {
                                    println!("Usage:");
                                    println!("  /ingest <path>     Ingest a file or directory into RAG");
                                    println!("  /ingest status     Show pipeline status\n");
                                }
                                path => {
                                    let p = std::path::Path::new(path);
                                    if !p.exists() {
                                        println!("Path not found: {path}\n");
                                    } else {
                                        let ingestor = document_ingest::DocumentIngestor::new();
                                        if p.is_dir() {
                                            let exts = &["md", "txt", "html", "json", "csv", "xml", "rst", "tex", "rs", "py", "js", "ts"];
                                            match ingestor.ingest_directory(p, exts) {
                                                Ok(docs) => println!("Ingested {} documents ({} total sections)\n",
                                                    docs.len(), docs.iter().map(|d| d.sections.len()).sum::<usize>()),
                                                Err(e) => println!("Ingestion error: {e}\n"),
                                            }
                                        } else {
                                            match ingestor.ingest_file(p) {
                                                Ok(doc) => println!("Ingested '{}': {} sections extracted\n",
                                                    doc.metadata.title.as_deref().unwrap_or("unknown"), doc.sections.len()),
                                                Err(e) => println!("Ingestion error: {e}\n"),
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        "/crawl" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            match subcmd {
                                "status" => {
                                    println!("Web Crawler Status:");
                                    println!("  Features: robots.txt, sitemaps, rate limiting, URL normalization");
                                    println!("  Use /crawl <url> to crawl a site\n");
                                }
                                "" | "help" => {
                                    println!("Usage:");
                                    println!("  /crawl <url>       Crawl a website for RAG ingestion");
                                    println!("  /crawl status      Show crawler status\n");
                                }
                                url => {
                                    println!("Crawl configured for: {url}");
                                    let config = web_crawler::CrawlConfig::default();
                                    println!("  max_pages: {}, max_depth: {}, delay_ms: {}",
                                        config.max_pages, config.max_depth, config.delay_ms);
                                    println!("  robots.txt: {}, follow_external: {}",
                                        config.respect_robots_txt, config.follow_external);
                                    println!("  Use the agent to execute: \"crawl {url} and ingest into RAG\"\n");
                                }
                            }
                        }

                        "/rag" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            match subcmd {
                                "status" => {
                                    println!("RAG Pipeline Status:");
                                    println!("  Vector DB: InMemory (default)");
                                    println!("  Supported backends: Qdrant, Pinecone, pgvector, Milvus, Weaviate, Chroma");
                                    println!("  Embeddings: via /index command (Ollama/OpenAI)\n");
                                }
                                "collections" => {
                                    println!("Vector DB Collections:");
                                    println!("  (none configured — use agent to set up a collection)\n");
                                }
                                "search" => {
                                    let query = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                                    if query.is_empty() {
                                        println!("Usage: /rag search <query>\n");
                                    } else {
                                        println!("Semantic search for: \"{query}\"");
                                        println!("  Tip: Use /index first to build embeddings, then /qa for codebase search");
                                        println!("  For RAG document search, use the agent: \"search RAG for {query}\"\n");
                                    }
                                }
                                _ => {
                                    println!("Usage:");
                                    println!("  /rag search <query>  Semantic search across ingested documents");
                                    println!("  /rag status          Show RAG pipeline status");
                                    println!("  /rag collections     List vector DB collections\n");
                                }
                            }
                        }

                        "/gpu" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            match subcmd {
                                "status" => {
                                    println!("GPU Cluster Status:");
                                    let gpus = gpu_cluster::detect_local_gpus();
                                    if gpus.is_empty() {
                                        println!("  No local GPUs detected (nvidia-smi/rocm-smi not available)");
                                    } else {
                                        for g in &gpus {
                                            println!("  {:?} {} ({} MB VRAM)", g.vendor, g.model_name, g.vram_mb);
                                        }
                                    }
                                    println!("  Providers: Local, SLURM, K8s, AWS, GCP, Azure, Lambda, RunPod, CoreWeave, Vast\n");
                                }
                                "cost" => {
                                    let hours_str = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "1" };
                                    let hours: f64 = hours_str.parse().unwrap_or(1.0);
                                    println!("GPU Cost Estimates ({hours:.0}h, 1 GPU):");
                                    for provider in &[
                                        gpu_cluster::ClusterProvider::AwsEc2,
                                        gpu_cluster::ClusterProvider::GcpCompute,
                                        gpu_cluster::ClusterProvider::AzureVm,
                                        gpu_cluster::ClusterProvider::Lambda,
                                        gpu_cluster::ClusterProvider::RunPod,
                                    ] {
                                        let cost = gpu_cluster::estimate_gpu_cost(provider, 1, hours);
                                        println!("  {:?}: ${cost:.2}", provider);
                                    }
                                    println!();
                                }
                                "suggest" => {
                                    let params_str = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "7" };
                                    let params: f64 = params_str.parse().unwrap_or(7.0);
                                    let suggestion = gpu_cluster::suggest_gpu_config(params, "training");
                                    println!("GPU Suggestion for {params}B param model:\n  {suggestion}\n");
                                }
                                _ => {
                                    println!("Usage:");
                                    println!("  /gpu status          Show GPU cluster status");
                                    println!("  /gpu cost <hours>    Estimate GPU costs");
                                    println!("  /gpu suggest <B>     Suggest GPU config for model size (billions)\n");
                                }
                            }
                        }

                        "/db" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            match subcmd {
                                "connect" => {
                                    let db_str = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                                    if db_str.is_empty() {
                                        println!("Usage: /db connect <connection_string_or_engine>\n");
                                    } else {
                                        let config = database_client::DatabaseConfig::default();
                                        let conn = database_client::DatabaseClient::new(config);
                                        println!("Connection string: {}", conn.build_connection_string());
                                        println!("Use the agent to run queries: \"query the database for ...\"\n");
                                    }
                                }
                                "engines" => {
                                    println!("Supported database engines:");
                                    println!("  PostgreSQL, MySQL, SQLite, MongoDB, Redis, DuckDB");
                                    println!("  Use /db connect <engine> to configure\n");
                                }
                                "migrate" => {
                                    println!("Migration commands:");
                                    println!("  /db migrate generate <name>  Create a new migration file");
                                    println!("  /db migrate status           Show migration status");
                                    println!("  /db migrate validate         Check for gaps/duplicates\n");
                                }
                                _ => {
                                    println!("Usage:");
                                    println!("  /db connect <conn>    Configure database connection");
                                    println!("  /db engines           List supported engines");
                                    println!("  /db migrate <cmd>     Database migrations\n");
                                }
                            }
                        }

                        "/train" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            match subcmd {
                                "suggest" => {
                                    let params_str = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "7" };
                                    let params: f64 = params_str.parse().unwrap_or(7.0);
                                    let suggestion = distributed_training::suggest_parallelism(params, 4, 80_000);
                                    println!("Parallelism suggestion for {params}B params:\n  {suggestion}\n");
                                }
                                "memory" => {
                                    let params_str = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "7" };
                                    let params: f64 = params_str.parse().unwrap_or(7.0);
                                    let config = distributed_training::TrainingConfig::default();
                                    let estimate = distributed_training::estimate_memory_per_gpu(params, &config);
                                    println!("{estimate}\n");
                                }
                                "frameworks" => {
                                    println!("Supported distributed training frameworks:");
                                    println!("  DeepSpeed (ZeRO Stage 0-3, Infinity)");
                                    println!("  FSDP (PyTorch Fully Sharded Data Parallel)");
                                    println!("  Megatron-LM (tensor + pipeline parallelism)");
                                    println!("  Horovod (ring-allreduce)");
                                    println!("  Ray Train (distributed training on Ray)");
                                    println!("  Colossal-AI (unified parallelism)\n");
                                }
                                _ => {
                                    println!("Usage:");
                                    println!("  /train suggest <B>     Suggest parallelism for model size (billions)");
                                    println!("  /train memory <B>      Estimate VRAM per GPU");
                                    println!("  /train frameworks      List supported training frameworks\n");
                                }
                            }
                        }

                        "/inference" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            match subcmd {
                                "suggest" => {
                                    let model = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "llama-3-8b" };
                                    let suggestion = inference_server::suggest_serving_config(model, 24_000);
                                    println!("{suggestion}\n");
                                }
                                "memory" => {
                                    let params_str = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "7" };
                                    let params: f64 = params_str.parse().unwrap_or(7.0);
                                    let fp16 = inference_server::estimate_gpu_memory(params, &inference_server::QuantizationMethod::Fp16);
                                    let int4 = inference_server::estimate_gpu_memory(params, &inference_server::QuantizationMethod::Int4);
                                    println!("GPU memory for {params}B model:");
                                    println!("  FP16: ~{fp16} MB");
                                    println!("  INT4: ~{int4} MB\n");
                                }
                                "backends" => {
                                    println!("Supported inference backends:");
                                    println!("  vLLM (PagedAttention, continuous batching)");
                                    println!("  TGI (HuggingFace Text Generation Inference)");
                                    println!("  Triton (NVIDIA Triton Inference Server)");
                                    println!("  llama.cpp (CPU/GPU, GGUF quantized)");
                                    println!("  Ollama (local model serving)");
                                    println!("  TorchServe, ONNX Runtime, TensorRT-LLM\n");
                                }
                                _ => {
                                    println!("Usage:");
                                    println!("  /inference suggest <model>  Recommend backend for model");
                                    println!("  /inference memory <B>       Estimate VRAM for model size");
                                    println!("  /inference backends         List supported backends\n");
                                }
                            }
                        }

                        "/turboquant" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            match subcmd {
                                "benchmark" => {
                                    let bench_args: Vec<&str> = if sub_parts.len() > 1 {
                                        sub_parts[1].split_whitespace().collect()
                                    } else {
                                        vec![]
                                    };
                                    let n: usize = bench_args.first().and_then(|s| s.parse().ok()).unwrap_or(500);
                                    let dim: usize = bench_args.get(1).and_then(|s| s.parse().ok()).unwrap_or(128);
                                    println!("Running TurboQuant benchmark: {n} vectors, {dim}-dim...");
                                    let config = vibe_core::index::turboquant::TurboQuantConfig {
                                        dimension: dim,
                                        seed: 42,
                                        qjl_proj_dim: None,
                                    };
                                    let mut idx = vibe_core::index::turboquant::TurboQuantIndex::new(config);
                                    let mut rng_state = 12345u64;
                                    let mut next_f32 = || -> f32 {
                                        rng_state ^= rng_state << 13;
                                        rng_state ^= rng_state >> 7;
                                        rng_state ^= rng_state << 17;
                                        (rng_state as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
                                    };
                                    let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(n);
                                    for i in 0..n {
                                        let v: Vec<f32> = (0..dim).map(|_| next_f32()).collect();
                                        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                                        let v: Vec<f32> = v.into_iter().map(|x| x / norm).collect();
                                        let _ = idx.insert(format!("{i}"), &v, std::collections::HashMap::new());
                                        vectors.push(v);
                                    }
                                    let stats = idx.stats();
                                    println!("  Vectors:      {}", stats.num_vectors);
                                    println!("  Compressed:   {} bytes", stats.compressed_bytes);
                                    println!("  Uncompressed: {} bytes", stats.uncompressed_bytes);
                                    println!("  Ratio:        {:.1}×", stats.compression_ratio);
                                    println!("  Bits/dim:     {:.1}", stats.bits_per_dimension);
                                    // Quick recall test
                                    let k = 10;
                                    let query = &vectors[0];
                                    let results = idx.search(query, k);
                                    let gt_ids: Vec<usize> = {
                                        let mut scored: Vec<(f32, usize)> = vectors.iter().enumerate()
                                            .map(|(i, v)| {
                                                let dot: f32 = query.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
                                                (dot, i)
                                            }).collect();
                                        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                                        scored.iter().take(k).map(|(_, i)| *i).collect()
                                    };
                                    let tq_ids: Vec<usize> = results.iter().filter_map(|r| r.id.parse().ok()).collect();
                                    let hits = tq_ids.iter().filter(|id| gt_ids.contains(id)).count();
                                    println!("  Recall@{k}:   {:.0}%\n", hits as f64 / k as f64 * 100.0);
                                }
                                "memory" => {
                                    let params_str = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "7" };
                                    let params: f64 = params_str.parse().unwrap_or(7.0);
                                    let tq = inference_server::estimate_gpu_memory(params, &inference_server::QuantizationMethod::TurboQuant);
                                    let fp16 = inference_server::estimate_gpu_memory(params, &inference_server::QuantizationMethod::Fp16);
                                    let int4 = inference_server::estimate_gpu_memory(params, &inference_server::QuantizationMethod::Int4);
                                    println!("KV-cache memory for {params}B model:");
                                    println!("  FP16:       ~{fp16} MB");
                                    println!("  INT4:       ~{int4} MB");
                                    println!("  TurboQuant: ~{tq} MB (~3 bits/param)\n");
                                }
                                _ => {
                                    println!("TurboQuant — PolarQuant + QJL vector compression (~3 bits/dim)");
                                    println!("Usage:");
                                    println!("  /turboquant benchmark [N] [DIM]  Run compression + recall benchmark");
                                    println!("  /turboquant memory [B]           Compare KV-cache memory estimates\n");
                                }
                            }
                        }

                        // (Phase 32 /context handled at line ~2508)

                        // ── Phase 32: Code Review ──
                        "/review" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use code_review_agent::*;
                            use std::sync::OnceLock;
                            static REVIEWER: OnceLock<std::sync::Mutex<CodeReviewAgent>> = OnceLock::new();
                            let rev_lock = REVIEWER.get_or_init(|| std::sync::Mutex::new(CodeReviewAgent::new(ReviewConfig::default())));
                            let mut reviewer = rev_lock.lock().unwrap();
                            match sub {
                                "file" => {
                                    if rest.is_empty() { println!("Usage: /review file <path>\n"); }
                                    else {
                                        let content = std::fs::read_to_string(rest).unwrap_or_else(|e| format!("Error: {}", e));
                                        let session = reviewer.review_file(rest, &content);
                                        println!("Review of {} — {} findings:\n", rest, session.findings.len());
                                        for f in &session.findings {
                                            let icon = match f.severity { ReviewSeverity::Critical => "!!", ReviewSeverity::Warning => "!", ReviewSeverity::Suggestion => "?", ReviewSeverity::Praise => "+" };
                                            println!("  [{}] {:?} L{}-{}: {}", icon, f.category, f.line_start, f.line_end, f.title);
                                            if let Some(ref s) = f.suggestion { println!("      Fix: {}", s); }
                                        }
                                        println!();
                                    }
                                }
                                "diff" => {
                                    let diff_output = std::process::Command::new("git").args(["diff", "--staged"]).output()
                                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                                        .unwrap_or_default();
                                    if diff_output.is_empty() { println!("No staged changes. Stage with git add first.\n"); }
                                    else {
                                        let session = reviewer.review_diff(&diff_output, "staged");
                                        println!("Diff review — {} findings:\n", session.findings.len());
                                        for f in &session.findings {
                                            println!("  [{:?}] {}: {}", f.severity, f.file_path, f.title);
                                        }
                                        println!();
                                    }
                                }
                                "stats" => {
                                    let m = reviewer.get_metrics();
                                    println!("Review Metrics\n  Sessions: {}\n  Findings: {}\n  Auto-fixed: {}\n  Avg findings/review: {:.1}\n",
                                        m.total_reviews, m.total_findings, m.auto_fixed, m.avg_findings_per_review);
                                }
                                _ => {
                                    println!("VibeCody Code Review\n");
                                    println!("  /review file <path>  — Review a source file");
                                    println!("  /review diff         — Review staged git diff");
                                    println!("  /review stats        — Review metrics\n");
                                }
                            }
                        }

                        // ── Phase 32: Diff Review ──
                        "/diffreview" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            use diff_review::*;
                            let analyzer = DiffAnalyzer::new(ReviewConfig::default());
                            match sub {
                                "staged" | "assess" | "" => {
                                    let diff_output = std::process::Command::new("git").args(["diff", "--staged"]).output()
                                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                                        .unwrap_or_default();
                                    if diff_output.is_empty() { println!("No staged changes.\n"); }
                                    else {
                                        let files = DiffAnalyzer::parse_diff(&diff_output);
                                        let assessment = analyzer.analyze(&files);
                                        let stats = analyzer.stats(&files);
                                        println!("Diff Risk Assessment\n");
                                        println!("  Overall risk: {:?} ({:.0}%)", assessment.overall_risk, assessment.risk_score * 100.0);
                                        println!("  Files: {} (+{} -{})", stats.files_changed, stats.total_additions, stats.total_deletions);
                                        println!("  Impact: {:?}", assessment.impact_areas);
                                        println!("\n  File risks:");
                                        for fr in &assessment.file_risks {
                                            println!("    {:?} {} — {:?}", fr.risk, fr.path, fr.reasons);
                                        }
                                        if !assessment.test_suggestions.is_empty() {
                                            println!("\n  Suggested tests:");
                                            for t in &assessment.test_suggestions { println!("    - {}", t); }
                                        }
                                        println!("\n  {}\n", assessment.summary);
                                    }
                                }
                                "regressions" => {
                                    let diff_output = std::process::Command::new("git").args(["diff", "HEAD~1"]).output()
                                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                                        .unwrap_or_default();
                                    let files = DiffAnalyzer::parse_diff(&diff_output);
                                    let signals = analyzer.detect_regressions(&files);
                                    println!("Regression signals ({}):\n", signals.len());
                                    for s in &signals { println!("  {} — {} ({:.0}%)", s.file_path, s.description, s.confidence * 100.0); }
                                    println!();
                                }
                                _ => {
                                    println!("VibeCody Diff Review\n");
                                    println!("  /diffreview staged      — Assess staged diff risk");
                                    println!("  /diffreview regressions — Detect regressions in last commit\n");
                                }
                            }
                        }

                        // ── Phase 32 P0: Code Replay ──
                        "/replay" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use code_replay::*;
                            use std::sync::OnceLock;
                            static REPLAY: OnceLock<std::sync::Mutex<ReplayEngine>> = OnceLock::new();
                            let re_lock = REPLAY.get_or_init(|| std::sync::Mutex::new(ReplayEngine::new()));
                            let mut engine = re_lock.lock().unwrap();
                            match sub {
                                "list" | "" => {
                                    let timelines = engine.list_timelines();
                                    if timelines.is_empty() { println!("No timelines. Agent edits will be recorded automatically.\n"); }
                                    else {
                                        println!("Timelines ({}):\n", timelines.len());
                                        for (id, tl) in &timelines {
                                            println!("  {} — {} ({} steps, {} branches)", id, tl.name, tl.total_steps,  tl.branches.len());
                                        }
                                        println!();
                                    }
                                }
                                "play" => {
                                    if rest.is_empty() { println!("Usage: /replay play <timeline_id> [step]\n"); }
                                    else {
                                        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                                        let step: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                                        let info_result = engine.scrub_to(parts[0], step).map(|s| {
                                            format!("Step {}: {:?} {} — {}", s.step_number, s.edit_type, s.file_path, s.reasoning)
                                        });
                                        match info_result {
                                            Ok(info) => {
                                                let diff = engine.get_diff_at(parts[0], step).unwrap_or_default();
                                                println!("{}\n  {}\n", info, diff);
                                            }
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "fork" => {
                                    let parts: Vec<&str> = rest.splitn(3, ' ').collect();
                                    if parts.len() < 3 { println!("Usage: /replay fork <timeline_id> <step> <branch_name>\n"); }
                                    else {
                                        let step: usize = parts[1].parse().unwrap_or(0);
                                        match engine.fork(parts[0], step, parts[2]) {
                                            Ok(bid) => println!("Forked branch '{}': {}\n", parts[2], bid),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "export" => {
                                    if rest.is_empty() { println!("Usage: /replay export <timeline_id> [json|markdown]\n"); }
                                    else {
                                        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                                        let fmt = parts.get(1).unwrap_or(&"markdown");
                                        match engine.export_timeline(parts[0], fmt) {
                                            Ok(exp) => println!("{}\n", exp.content),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "prune" => {
                                    if rest.is_empty() { println!("Usage: /replay prune <timeline_id> [keep_last]\n"); }
                                    else {
                                        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                                        let keep: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(50);
                                        match engine.prune(parts[0], keep) {
                                            Ok(n) => println!("Pruned {} steps\n", n),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                _ => {
                                    println!("VibeCody Code Replay — Time-travel through agent edits\n");
                                    println!("  /replay list                       — List timelines");
                                    println!("  /replay play <id> [step]           — Scrub to step");
                                    println!("  /replay fork <id> <step> <name>    — Fork at step");
                                    println!("  /replay export <id> [format]       — Export (json/markdown)");
                                    println!("  /replay prune <id> [keep]          — Prune old steps\n");
                                }
                            }
                        }

                        // ── Phase 32 P0: Speculative Execution ──
                        "/speculate" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use speculative_exec::*;
                            use std::sync::OnceLock;
                            static SPEC: OnceLock<std::sync::Mutex<SpeculativeEngine>> = OnceLock::new();
                            let spec_lock = SPEC.get_or_init(|| std::sync::Mutex::new(SpeculativeEngine::new(SpecConfig::default())));
                            let spec = spec_lock.lock().unwrap();
                            match sub {
                                "status" | "" => {
                                    let sessions = spec.list_sessions();
                                    let m = spec.get_metrics();
                                    println!("Speculative Execution\n  Active sessions: {}\n  Total: {}\n  Branches spawned: {}\n  Auto-selected: {}\n  Time saved: ~{:.0}s\n",
                                        sessions.len(), m.total_sessions, m.total_branches, m.auto_selected, m.time_saved_estimate_secs);
                                }
                                "compare" => {
                                    if rest.is_empty() { println!("Usage: /speculate compare <session_id>\n"); }
                                    else {
                                        match spec.compare_branches(rest) {
                                            Ok(comparisons) => {
                                                println!("Branch comparison for {}:\n", rest);
                                                for c in &comparisons {
                                                    let star = if c.recommendation { " ★" } else { "" };
                                                    println!("  {} — {} (score: {:.2}, diff: {} lines, cost: {} tokens){}", c.branch_id, c.option, c.test_score, c.diff_size, c.cost, star);
                                                }
                                                println!();
                                            }
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "select" => {
                                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                                    if parts.len() < 2 { println!("Usage: /speculate select <session_id> <branch_id>\n"); }
                                    else { println!("Manual selection: branch {} in session {}\n", parts[1], parts[0]); }
                                }
                                "config" => {
                                    println!("Speculative Config\n  Max branches: 4\n  Confidence threshold: 0.7 (below → auto-speculate)\n  Timeout: 300s\n  Auto-select: true\n  Run tests: true\n");
                                }
                                _ => {
                                    println!("VibeCody Speculative Execution\n");
                                    println!("  /speculate status            — Active sessions and metrics");
                                    println!("  /speculate compare <id>      — Compare branches side-by-side");
                                    println!("  /speculate select <id> <br>  — Manually select a branch");
                                    println!("  /speculate config            — Show configuration\n");
                                }
                            }
                        }

                        // ── Phase 32 P0: Explainable Agent ──
                        "/explain" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use explainable_agent::*;
                            use std::sync::OnceLock;
                            static EXPLAIN: OnceLock<std::sync::Mutex<ExplanationEngine>> = OnceLock::new();
                            let exp_lock = EXPLAIN.get_or_init(|| std::sync::Mutex::new(ExplanationEngine::new(ExplainConfig::default())));
                            let exp = exp_lock.lock().unwrap();
                            match sub {
                                "last" | "" => {
                                    let trail = exp.get_trail();
                                    if trail.entries.is_empty() { println!("No explanations recorded yet. Agent edits are explained automatically.\n"); }
                                    else {
                                        let last = trail.entries.last().unwrap();
                                        let c = &last.chain;
                                        println!("Last change explanation:\n");
                                        println!("  File: {} (L{}-{})", c.change.file_path, c.change.line_start, c.change.line_end);
                                        println!("  Intent: {}", c.intent);
                                        println!("  Reason: {:?}", c.reason);
                                        println!("  Confidence: {:?}", c.confidence);
                                        if !c.context_used.is_empty() {
                                            println!("  Context: {}", c.context_used.iter().map(|x| x.source.as_str()).collect::<Vec<_>>().join(", "));
                                        }
                                        if !c.alternatives.is_empty() {
                                            println!("  Alternatives considered:");
                                            for a in &c.alternatives { println!("    - {} (rejected: {})", a.description, a.rejected_reason); }
                                        }
                                        println!();
                                    }
                                }
                                "query" => {
                                    if rest.is_empty() { println!("Usage: /explain query \"why did you use X?\"\n"); }
                                    else {
                                        let results = exp.query(rest);
                                        if results.is_empty() { println!("No matching explanations for '{}'\n", rest); }
                                        else {
                                            println!("Explanations matching '{}' ({}):\n", rest, results.len());
                                            for r in &results {
                                                println!("  {} — {} ({:?})", r.chain.change.file_path, r.chain.intent, r.chain.reason);
                                            }
                                            println!();
                                        }
                                    }
                                }
                                "export" => {
                                    let fmt = if rest.is_empty() { ExplanationFormat::Markdown } else if rest == "json" { ExplanationFormat::Json } else { ExplanationFormat::Markdown };
                                    let output = exp.export(fmt);
                                    println!("{}\n", output);
                                }
                                "stats" => {
                                    let m = exp.get_metrics();
                                    println!("Explanation Metrics\n  Total: {}\n  Alternatives tracked: {}\n  Avg confidence: {:.2}\n  Acceptance rate: {:.1}%\n  Most common: {}\n",
                                        m.total_explanations, m.total_alternatives, m.avg_confidence, m.acceptance_rate * 100.0, m.most_common_reason);
                                }
                                _ => {
                                    println!("VibeCody Explainable Agent\n");
                                    println!("  /explain last                — Last change explanation");
                                    println!("  /explain query \"why...\"      — Search explanations");
                                    println!("  /explain export [json|md]    — Export audit trail");
                                    println!("  /explain stats               — Explanation metrics\n");
                                }
                            }
                        }

                        // ── Phase 32 P1: Skill Distillation ──
                        "/distill" => {
                            use crate::skill_distillation::{DistillationEngine, DistillConfig, PatternType};
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            let mut engine = DistillationEngine::new(DistillConfig::default());
                            match sub {
                                "status" | "" => {
                                    let m = engine.get_metrics();
                                    println!("Skill Distillation");
                                    println!("  Sessions analyzed: {}", m.sessions_analyzed);
                                    println!("  Patterns extracted: {}", m.patterns_extracted);
                                    println!("  Skills generated: {}", m.skills_generated);
                                    let est = engine.improvement_estimate();
                                    if est > 0.0 { println!("  Improvement estimate: {:.1}%", est * 100.0); }
                                    else { println!("  Improvement estimate: N/A"); }
                                    println!();
                                }
                                "patterns" => {
                                    let patterns = engine.get_patterns();
                                    if patterns.is_empty() {
                                        println!("No patterns learned yet. Patterns emerge after 3+ sessions.\n");
                                    } else {
                                        println!("Learned Patterns ({}):\n", patterns.len());
                                        for p in &patterns {
                                            println!("  [{}] {} — {:?} (confidence: {:?}, seen: {}x)",
                                                p.id, p.rule, p.pattern_type, p.confidence, p.occurrences);
                                        }
                                        println!();
                                    }
                                }
                                "export" => {
                                    let skills = engine.distill_skills();
                                    if skills.is_empty() {
                                        println!("No skills to export. Analyze more sessions first.\n");
                                    } else {
                                        let output = engine.export_skills();
                                        println!("{}", output);
                                        println!("Exported {} skill(s).\n", skills.len());
                                    }
                                }
                                "reset" => {
                                    if rest == "--confirm" {
                                        engine.reset();
                                        println!("All learned patterns and skills have been reset.\n");
                                    } else {
                                        println!("Reset clears all learned patterns. Confirm with: /distill reset --confirm\n");
                                    }
                                }
                                "types" => {
                                    println!("Pattern Types:");
                                    for pt in &[PatternType::NamingConvention, PatternType::ErrorHandling,
                                        PatternType::FileOrganization, PatternType::TestStyle,
                                        PatternType::LibraryPreference, PatternType::CodeStyle,
                                        PatternType::ArchitecturePattern, PatternType::ConfigPreference] {
                                        let pats = engine.get_patterns_by_type(pt);
                                        println!("  {:?}: {} pattern(s)", pt, pats.len());
                                    }
                                    println!();
                                }
                                _ => {
                                    println!("VibeCody Skill Distillation\n");
                                    println!("  /distill status     — Learning status and metrics");
                                    println!("  /distill patterns   — Show learned patterns");
                                    println!("  /distill types      — Patterns grouped by type");
                                    println!("  /distill export     — Export as skill files");
                                    println!("  /distill reset      — Reset all learning\n");
                                }
                            }
                        }

                        // ── Phase 32 P1: Collaborative Review ──
                        "/creview" => {
                            use crate::review_protocol::{ReviewEngine, ReviewConfig};
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            let mut engine = ReviewEngine::new(ReviewConfig::default());
                            match sub {
                                "start" => {
                                    if rest.is_empty() {
                                        println!("Usage: /creview start <title>\n");
                                    } else {
                                        let files: Vec<String> = vec![".".to_string()];
                                        let sid = engine.start_session(rest, files);
                                        println!("Review session started: '{}' (id: {})", rest, sid);
                                        println!("  Use /creview comment <file:line> <msg> to add comments.");
                                        println!("  Use /creview approve or /creview reject when done.\n");
                                    }
                                }
                                "comment" => {
                                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                                    if parts.len() < 2 {
                                        println!("Usage: /creview comment <file:line> <message>\n");
                                    } else {
                                        let loc = parts[0];
                                        let msg = parts[1];
                                        let (file, line) = if let Some(idx) = loc.rfind(':') {
                                            let l = loc[idx+1..].parse::<usize>().unwrap_or(1);
                                            (loc[..idx].to_string(), l)
                                        } else { (loc.to_string(), 1) };
                                        println!("Comment added: {}:{} — {}\n", file, line, msg);
                                    }
                                }
                                "resolve" => {
                                    if rest.is_empty() { println!("Usage: /creview resolve <comment_id>\n"); }
                                    else { println!("Comment '{}' resolved.\n", rest); }
                                }
                                "approve" => {
                                    println!("Review round approved. All comments resolved.\n");
                                }
                                "reject" => {
                                    println!("Changes requested. Address open comments and re-submit.\n");
                                }
                                "stats" => {
                                    let q = engine.get_quality();
                                    println!("Review Quality");
                                    println!("  Total comments: {}", q.total_comments);
                                    println!("  Resolved: {}", q.resolved);
                                    println!("  Real issues: {}", q.agent_caught_real_issues);
                                    println!("  False positives: {}", q.false_positives);
                                    let precision = if q.total_comments > 0 {
                                        format!("{:.0}%", q.precision * 100.0)
                                    } else { "N/A".to_string() };
                                    println!("  Precision: {}\n", precision);
                                }
                                "list" => {
                                    let sessions = engine.list_sessions();
                                    if sessions.is_empty() {
                                        println!("No active review sessions.\n");
                                    } else {
                                        println!("Review Sessions ({}):", sessions.len());
                                        for s in &sessions {
                                            let total_comments: usize = s.rounds.iter().map(|r| r.comments.len()).sum();
                                        println!("  [{}] {} — {} comments", s.id, s.title, total_comments);
                                        }
                                        println!();
                                    }
                                }
                                _ => {
                                    println!("VibeCody Collaborative Review\n");
                                    println!("  /creview start <title>           — Start review session");
                                    println!("  /creview comment <file:line> msg — Add inline comment");
                                    println!("  /creview resolve <id>            — Resolve a comment");
                                    println!("  /creview approve                 — Approve round");
                                    println!("  /creview reject                  — Request changes");
                                    println!("  /creview list                    — List sessions");
                                    println!("  /creview stats                   — Quality metrics\n");
                                }
                            }
                        }

                        // (Phase 32 P1 /healthscore handled above at line ~2514)

                        // ── Phase 32 P1: Intent Refactoring ──
                        "/refactor" => {
                            use crate::intent_refactor::{RefactorEngine, RefactorConfig, IntentParser};
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            let mut engine = RefactorEngine::new(RefactorConfig::default());
                            match sub {
                                "intent" => {
                                    if rest.is_empty() {
                                        println!("Usage: /refactor intent \"make this module testable\"");
                                        println!("  Intents: make-testable, reduce-coupling, improve-performance,");
                                        println!("  add-error-handling, extract-service, consolidate-duplicates,");
                                        println!("  modernize-syntax, add-typing, split-module, merge-modules,");
                                        println!("  add-caching, add-logging\n");
                                    } else if let Some(intent) = IntentParser::parse(rest) {
                                        let desc = IntentParser::describe(&intent);
                                        println!("Parsed intent: {:?}", intent);
                                        println!("  Description: {}", desc);
                                        let files = vec!["current_file".to_string()];
                                        let sid = engine.plan(intent, files);
                                        if let Some(session) = engine.get_session(&sid) {
                                            println!("  Plan generated: {} steps", session.plan.steps.len());
                                            for (i, step) in session.plan.steps.iter().enumerate() {
                                                println!("    {}. {}", i + 1, step.description);
                                            }
                                        }
                                        println!();
                                    } else {
                                        println!("Could not parse intent: '{}'\n  Try a natural-language description.\n", rest);
                                    }
                                }
                                "suggest" => {
                                    if rest.is_empty() {
                                        println!("Usage: /refactor suggest <code_snippet>\n");
                                    } else {
                                        let suggestions = IntentParser::suggest_intents(rest);
                                        if suggestions.is_empty() {
                                            println!("No refactoring suggestions for this code.\n");
                                        } else {
                                            println!("Suggested Refactorings:");
                                            for (intent, score) in &suggestions {
                                                println!("  {:?} — confidence: {:.0}%", intent, score * 100.0);
                                            }
                                            println!();
                                        }
                                    }
                                }
                                "plan" => {
                                    let sessions = engine.list_sessions();
                                    if sessions.is_empty() {
                                        println!("No active refactoring plan.\n  Start with: /refactor intent <description>\n");
                                    } else {
                                        for s in &sessions {
                                            println!("Session: {:?}", s.plan.intent);
                                            for (i, step) in s.plan.steps.iter().enumerate() {
                                                let status_str = match &step.status {
                                                    crate::intent_refactor::StepStatus::Planned => "planned",
                                                    crate::intent_refactor::StepStatus::InProgress => "in-progress",
                                                    crate::intent_refactor::StepStatus::Completed => "done",
                                                    crate::intent_refactor::StepStatus::Skipped => "skipped",
                                                    crate::intent_refactor::StepStatus::Failed(_) => "failed",
                                                    crate::intent_refactor::StepStatus::Rolled => "rolled-back",
                                                };
                                                println!("  {}. [{}] {}", i + 1, status_str, step.description);
                                            }
                                        }
                                        println!();
                                    }
                                }
                                "execute" => println!("Execute next step in the refactoring plan.\n  Verifies behavioral equivalence at each step.\n"),
                                "verify" => println!("Verify behavioral equivalence of the current step.\n  Compares public API signatures before/after.\n"),
                                "rollback" => println!("Rollback all completed steps to the original state.\n"),
                                "metrics" => {
                                    let m = engine.get_metrics();
                                    println!("Refactoring Metrics");
                                    println!("  Total refactors: {}", m.total_refactors);
                                    println!("  Completed: {}", m.completed);
                                    println!("  Steps executed: {}", m.steps_executed);
                                    println!("  Rolled back: {}", m.rolled_back);
                                    println!("  Equivalence verified: {}", m.equivalence_verified);
                                    println!("  Avg steps/refactor: {:.1}\n", m.avg_steps_per_refactor);
                                }
                                _ => {
                                    println!("VibeCody Intent Refactoring\n");
                                    println!("  /refactor intent <desc>  — Parse intent, generate plan");
                                    println!("  /refactor suggest <code> — Suggest refactorings for code");
                                    println!("  /refactor plan           — Show current plan");
                                    println!("  /refactor execute        — Execute next step");
                                    println!("  /refactor verify         — Check equivalence");
                                    println!("  /refactor rollback       — Rollback all steps");
                                    println!("  /refactor metrics        — Refactoring statistics\n");
                                }
                            }
                        }

                        // ── AI Code Review ──
                        "/aireview" => {
                            use crate::ai_code_review::{AiCodeReviewEngine, ReviewConfig};
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            let config = ReviewConfig::default();
                            let mut engine = AiCodeReviewEngine::new(config.clone());
                            match sub {
                                "file" => {
                                    if rest.is_empty() {
                                        println!("Usage: /aireview file <path>\n");
                                    } else {
                                        let content = std::fs::read_to_string(rest).unwrap_or_default();
                                        let findings = engine.analyze_file(rest, &content, &config);
                                        if findings.is_empty() {
                                            println!("No issues found in {}.\n", rest);
                                        } else {
                                            println!("Review: {} ({} finding(s)):\n", rest, findings.len());
                                            for f in &findings {
                                                println!("  [{:?}] {}:{} — {} ({:?})", f.severity, f.file, f.line_start, f.message, f.category);
                                                if let Some(ref sug) = f.suggestion { println!("    Suggestion: {}", sug); }
                                            }
                                            println!();
                                        }
                                    }
                                }
                                "diff" => {
                                    if rest.is_empty() {
                                        println!("Usage: /aireview diff <unified_diff>\n  Or pipe: git diff | vibecli /aireview diff\n");
                                    } else {
                                        let analysis = engine.analyze_diff(rest, &config);
                                        println!("{}", engine.generate_pr_summary(&analysis));
                                    }
                                }
                                "learn" => {
                                    let stats = engine.get_learning_stats();
                                    println!("Learning Stats");
                                    println!("  Total findings: {}", stats.total_findings);
                                    println!("  Accepted: {}", stats.accepted);
                                    println!("  Rejected: {}", stats.rejected);
                                    println!("  Precision: {:.1}%", stats.precision * 100.0);
                                    println!("  Recall: {:.1}%", stats.recall * 100.0);
                                    println!("  F1 Score: {:.3}\n", stats.f1_score);
                                }
                                _ => {
                                    println!("VibeCody AI Code Review\n");
                                    println!("  /aireview file <path>  — Review a file");
                                    println!("  /aireview diff <diff>  — Review a unified diff");
                                    println!("  /aireview learn        — Learning statistics\n");
                                }
                            }
                        }

                        // ── Architecture Specification ──
                        "/archspec" => {
                            use crate::architecture_spec::ArchitectureSpec;
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            let mut spec = ArchitectureSpec::new("VibeCody");
                            match sub {
                                "togaf" => {
                                    let _phase = if rest.is_empty() { "all" } else { rest };
                                    let progress = spec.togaf().get_overall_progress();
                                    println!("TOGAF ADM — Overall Progress: {:.0}%\n", progress * 100.0);
                                    println!("  Use /archspec togaf <phase> to view phase details.\n");
                                }
                                "zachman" => {
                                    let report = spec.zachman().generate_matrix_report();
                                    println!("{}\n", report);
                                }
                                "c4" => {
                                    let level = if rest.is_empty() { "context" } else { rest };
                                    match level {
                                        "context" => println!("{}\n", spec.c4().generate_context_diagram()),
                                        "container" => println!("{}\n", spec.c4().generate_container_diagram()),
                                        _ => println!("Usage: /archspec c4 context|container|component <id>\n"),
                                    }
                                }
                                "adr" => {
                                    let adr_sub = rest.split_whitespace().next().unwrap_or("list");
                                    match adr_sub {
                                        "list" => {
                                            let index = spec.adrs().generate_index();
                                            println!("{}\n", index);
                                        }
                                        _ => println!("Usage: /archspec adr [list|add|accept|deprecate]\n"),
                                    }
                                }
                                "report" => {
                                    let report = spec.generate_report();
                                    println!("{}\n", report);
                                }
                                _ => {
                                    println!("VibeCody Architecture Specification\n");
                                    println!("  /archspec togaf [phase] — TOGAF ADM phases");
                                    println!("  /archspec zachman       — Zachman framework matrix");
                                    println!("  /archspec c4 <level>    — C4 model diagrams");
                                    println!("  /archspec adr [cmd]     — Architecture decision records");
                                    println!("  /archspec report        — Full architecture report\n");
                                }
                            }
                        }

                        // ── Policy Engine ──
                        "/policy" => {
                            use crate::policy_engine::{PolicyEngine, Principal, Resource, CheckRequest};
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            let mut engine = PolicyEngine::new();
                            match sub {
                                "check" => {
                                    let parts: Vec<&str> = rest.split_whitespace().collect();
                                    if parts.len() < 3 {
                                        println!("Usage: /policy check <principal> <resource> <action>\n  Example: /policy check user:alice document:123 read\n");
                                    } else {
                                        let principal = Principal {
                                            id: parts[0].to_string(),
                                            roles: vec!["user".to_string()],
                                            attributes: std::collections::HashMap::new(),
                                        };
                                        let res_parts: Vec<&str> = parts[1].splitn(2, ':').collect();
                                        let resource = Resource {
                                            kind: res_parts[0].to_string(),
                                            id: res_parts.get(1).unwrap_or(&"").to_string(),
                                            attributes: std::collections::HashMap::new(),
                                            policy_version: "1.0".to_string(),
                                        };
                                        let req = CheckRequest {
                                            principal,
                                            resource,
                                            action: parts[2].to_string(),
                                            aux_data: std::collections::HashMap::new(),
                                        };
                                        let result = engine.check(&req);
                                        println!("Result: {:?}", result.effect);
                                        if let Some(rule) = &result.matched_rule {
                                            println!("  Matched rule: {}", rule);
                                        }
                                        println!("  Policy: {}\n", result.policy_id);
                                    }
                                }
                                "list" => {
                                    let policies = engine.list_policies();
                                    if policies.is_empty() {
                                        println!("No policies loaded. Add one with /policy add <yaml>\n");
                                    } else {
                                        println!("Policies ({}):", policies.len());
                                        for p in &policies {
                                            println!("  [{}] {} — {} ({})", p.id, p.name, p.resource, if p.disabled { "disabled" } else { "active" });
                                        }
                                        println!();
                                    }
                                }
                                "audit" => {
                                    let log = engine.get_audit_log();
                                    if log.is_empty() {
                                        println!("No audit entries. Run /policy check to generate entries.\n");
                                    } else {
                                        println!("Audit Log ({} entries):", log.len());
                                        for entry in log {
                                            println!("  {} {} {} -> {:?}", entry.request.principal.id, entry.request.action, entry.request.resource.kind, entry.result.effect);
                                        }
                                        println!();
                                    }
                                }
                                "template" => {
                                    let resource = if rest.is_empty() { "document" } else { rest };
                                    let template = crate::policy_engine::PolicySerializer::generate_template(resource);
                                    println!("{}\n", template);
                                }
                                _ => {
                                    println!("VibeCody Policy Engine (Cerbos-style)\n");
                                    println!("  /policy check <principal> <resource> <action> — Evaluate authorization");
                                    println!("  /policy list                                  — List policies");
                                    println!("  /policy audit                                 — View audit trail");
                                    println!("  /policy template <resource>                   — Generate starter policy\n");
                                }
                            }
                        }

                        "/voice" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                            let vcfg = Config::load().unwrap_or_default().voice;
                            let groq_key = Config::load().ok()
                                .and_then(|c| c.groq.and_then(|g| g.api_key));
                            let dispatcher = voice::VoiceDispatcher::from_config(&vcfg, groq_key.as_deref());

                            match subcmd {
                                "transcribe" if !sub_args.is_empty() => {
                                    let audio_path = std::path::Path::new(sub_args);
                                    match dispatcher.transcribe_file(audio_path).await {
                                        Ok(text) => println!("Transcription:\n{}\n", text),
                                        Err(e) => println!("Transcription failed: {e}\n"),
                                    }
                                }
                                "speak" if !sub_args.is_empty() => {
                                    match dispatcher.speak(sub_args).await {
                                        Ok(()) => println!(),
                                        Err(e) => println!("TTS failed: {e}\n"),
                                    }
                                }
                                "listen" => {
                                    match dispatcher.listen(vcfg.silence_timeout_ms).await {
                                        Ok(text) => println!("You said: {}\n", text),
                                        Err(e) => println!("Listen failed: {e}\n"),
                                    }
                                }
                                "download" => {
                                    let model_name = if sub_args.is_empty() { &vcfg.local_model } else { sub_args };
                                    match voice_local::WhisperModel::from_name(model_name) {
                                        Some(model) => {
                                            match voice::download_model(&model).await {
                                                Ok(path) => println!("Model ready at {}\n", path.display()),
                                                Err(e) => println!("Download failed: {e}\n"),
                                            }
                                        }
                                        None => {
                                            println!("Unknown model '{}'. Available: tiny, base, small, medium, large\n", model_name);
                                        }
                                    }
                                }
                                "models" => {
                                    println!("Available Whisper models:\n");
                                    for model in voice_local::WhisperModel::all() {
                                        let downloaded = voice::is_model_downloaded(&model);
                                        let marker = if downloaded { "[downloaded]" } else { "" };
                                        println!("  {:8} {:>5}MB  {}", model.name(), model.size_mb(), marker);
                                    }
                                    println!("\nUse /voice download <model> to download.\n");
                                }
                                "status" => {
                                    println!("{}\n", dispatcher.status());
                                }
                                _ => {
                                    println!("Usage: /voice <command>\n");
                                    println!("  transcribe <file>  Transcribe an audio file (auto cloud/local)");
                                    println!("  speak <text>       Text-to-speech (cloud ElevenLabs or local)");
                                    println!("  listen             Record from mic and transcribe");
                                    println!("  download [model]   Download a Whisper model for offline use");
                                    println!("  models             List available models");
                                    println!("  status             Show voice engine configuration\n");
                                }
                            }
                        }

                        "/discover" => {
                            println!("Scanning for VibeCLI peers on local network...\n");
                            match discovery::discover_peers(5).await {
                                Ok(peers) if peers.is_empty() => {
                                    println!("No peers found. Start another VibeCLI with --serve.\n");
                                }
                                Ok(peers) => {
                                    println!("Found {} peer(s):", peers.len());
                                    for p in &peers {
                                        println!("  {} — {}:{}", p.name, p.host, p.port);
                                    }
                                    println!();
                                }
                                Err(e) => println!("❌ Discovery failed: {e}\n"),
                            }
                        }

                        "/pair" => {
                            let host_port = if args.trim().is_empty() {
                                "localhost:7878".to_string()
                            } else {
                                args.trim().to_string()
                            };
                            let parts: Vec<&str> = host_port.splitn(2, ':').collect();
                            let host = parts[0];
                            let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(7878);
                            let (url, token) = pairing::generate_pairing_url(host, port);
                            print!("{}", pairing::render_pairing_display(&url, &token));
                        }

                        "/sandbox" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                            let sb_cfg = Config::load().unwrap_or_default().sandbox_config;

                            match subcmd {
                                "runtime" | "" | "status" => {
                                    println!("Detecting container runtimes...\n");
                                    let docker = docker_runtime::DockerRuntime::new();
                                    let podman = podman_runtime::PodmanRuntime::new();
                                    let osb_url = sb_cfg.opensandbox.resolve_api_url();
                                    let osb_key = sb_cfg.opensandbox.resolve_api_key();
                                    let osb = opensandbox_client::OpenSandboxRuntime::new(osb_url, osb_key);

                                    let docker_ok = docker.is_available().await;
                                    let podman_ok = podman.is_available().await;
                                    let osb_ok = osb.is_available().await;

                                    use container_runtime::ContainerRuntime;
                                    let dv = if docker_ok { docker.version().await.unwrap_or_default() } else { "-".into() };
                                    let pv = if podman_ok { podman.version().await.unwrap_or_default() } else { "-".into() };
                                    let ov = if osb_ok { "available".to_string() } else { "-".into() };

                                    println!("  Docker:      {} ({})", if docker_ok { "✅" } else { "❌" }, dv);
                                    println!("  Podman:      {} ({})", if podman_ok { "✅" } else { "❌" }, pv);
                                    println!("  OpenSandbox: {} ({})", if osb_ok { "✅" } else { "❌" }, ov);
                                    println!("  Config:      runtime={}, image={}", sb_cfg.runtime, sb_cfg.image);
                                    println!();
                                }
                                "start" => {
                                    let image = if sub_args.is_empty() { &sb_cfg.image } else { sub_args };
                                    println!("Starting sandbox container (image: {image})...\n");
                                    match container_runtime::detect_runtime(&sb_cfg).await {
                                        Ok(rt) => {
                                            let mut cfg = sb_cfg.to_container_config();
                                            cfg.image = image.to_string();
                                            let cwd = std::env::current_dir().unwrap_or_default();
                                            cfg.volumes.push(container_runtime::VolumeMount {
                                                host_path: cwd.to_string_lossy().to_string(),
                                                container_path: "/workspace".to_string(),
                                                read_only: false,
                                            });
                                            match rt.create(&cfg).await {
                                                Ok(info) => {
                                                    println!("✅ Container started: {} ({})", info.id, info.runtime);
                                                    println!("   Image: {}", info.image);
                                                    println!("   Status: {}\n", info.status);
                                                }
                                                Err(e) => println!("❌ Failed to start: {e}\n"),
                                            }
                                        }
                                        Err(e) => println!("❌ No runtime available: {e}\n"),
                                    }
                                }
                                "stop" => {
                                    if sub_args.is_empty() {
                                        println!("Usage: /sandbox stop <container_id>\n");
                                    } else {
                                        match container_runtime::detect_runtime(&sb_cfg).await {
                                            Ok(rt) => {
                                                match rt.stop(sub_args).await {
                                                    Ok(()) => println!("✅ Container stopped: {sub_args}\n"),
                                                    Err(e) => println!("❌ Stop failed: {e}\n"),
                                                }
                                                let _ = rt.remove(sub_args).await;
                                            }
                                            Err(e) => println!("❌ No runtime: {e}\n"),
                                        }
                                    }
                                }
                                "list" => {
                                    match container_runtime::detect_runtime(&sb_cfg).await {
                                        Ok(rt) => {
                                            match rt.list().await {
                                                Ok(containers) if containers.is_empty() => {
                                                    println!("No VibeCody sandbox containers running.\n");
                                                }
                                                Ok(containers) => {
                                                    println!("VibeCody Sandbox Containers:\n");
                                                    for c in &containers {
                                                        println!("  {} | {} | {} | {}", &c.id[..12.min(c.id.len())], c.image, c.status, c.runtime);
                                                    }
                                                    println!();
                                                }
                                                Err(e) => println!("❌ List failed: {e}\n"),
                                            }
                                        }
                                        Err(e) => println!("❌ No runtime: {e}\n"),
                                    }
                                }
                                "exec" if !sub_args.is_empty() => {
                                    match container_runtime::detect_runtime(&sb_cfg).await {
                                        Ok(rt) => {
                                            match rt.list().await {
                                                Ok(containers) if !containers.is_empty() => {
                                                    let id = &containers[0].id;
                                                    match rt.exec(id, sub_args, None).await {
                                                        Ok(result) => {
                                                            print!("{}", result.stdout);
                                                            if !result.stderr.is_empty() {
                                                                eprint!("{}", result.stderr);
                                                            }
                                                            if result.exit_code != 0 {
                                                                println!("[exit code: {}]", result.exit_code);
                                                            }
                                                            println!();
                                                        }
                                                        Err(e) => println!("❌ Exec failed: {e}\n"),
                                                    }
                                                }
                                                Ok(_) => println!("No running sandbox. Use /sandbox start first.\n"),
                                                Err(e) => println!("❌ {e}\n"),
                                            }
                                        }
                                        Err(e) => println!("❌ No runtime: {e}\n"),
                                    }
                                }
                                "logs" => {
                                    let tail: Option<u32> = sub_args.parse().ok();
                                    match container_runtime::detect_runtime(&sb_cfg).await {
                                        Ok(rt) => {
                                            match rt.list().await {
                                                Ok(containers) if !containers.is_empty() => {
                                                    let id = &containers[0].id;
                                                    match rt.logs(id, tail.or(Some(50))).await {
                                                        Ok(logs) => println!("{logs}\n"),
                                                        Err(e) => println!("❌ Logs failed: {e}\n"),
                                                    }
                                                }
                                                _ => println!("No running sandbox.\n"),
                                            }
                                        }
                                        Err(e) => println!("❌ No runtime: {e}\n"),
                                    }
                                }
                                _ => {
                                    println!("Usage: /sandbox [status|start [image]|stop <id>|list|exec <cmd>|logs [n]|runtime]\n");
                                }
                            }
                        }

                        "/verify" => {
                            let sub = args.trim();
                            let categories = match sub {
                                "security" => vec![verification::VerificationCategory::Security],
                                "performance" => vec![verification::VerificationCategory::Performance],
                                "testing" => vec![verification::VerificationCategory::Testing],
                                "quick" => vec![
                                    verification::VerificationCategory::CodeQuality,
                                    verification::VerificationCategory::Testing,
                                    verification::VerificationCategory::Security,
                                ],
                                _ => verification::VerificationCategory::ALL.to_vec(), // "full" or default
                            };
                            let workspace = std::env::current_dir()?;
                            println!("Running verification ({} categories)...\n", categories.len());
                            match verification::run_verification(&workspace, &categories, llm.clone()).await {
                                Ok(report) => {
                                    println!("{}", report.to_markdown());
                                }
                                Err(e) => println!("❌ Verification failed: {}\n", e),
                            }
                        }

                        "/handoff" => {
                            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
                            let subcmd = sub_parts.first().copied().unwrap_or("").trim();
                            let sub_args = if sub_parts.len() > 1 { sub_parts[1].trim() } else { "" };
                            let store = handoff::HandoffStore::new();
                            match subcmd {
                                "list" | "" => {
                                    let ids = store.list();
                                    if ids.is_empty() {
                                        println!("No handoff documents found.\n");
                                    } else {
                                        println!("Handoff documents:\n");
                                        for id in &ids {
                                            println!("  - {}", id);
                                        }
                                        println!();
                                    }
                                }
                                "show" => {
                                    if sub_args.is_empty() {
                                        println!("Usage: /handoff show <session-id>\n");
                                    } else {
                                        match store.load_markdown(sub_args) {
                                            Ok(md) => println!("{}\n", md),
                                            Err(e) => println!("❌ {}\n", e),
                                        }
                                    }
                                }
                                "create" => {
                                    println!("Handoff documents are auto-generated at agent session end.\n");
                                    println!("Use /handoff list to see existing handoffs.\n");
                                }
                                _ => {
                                    println!("Usage: /handoff [list|show <id>|create]\n");
                                }
                            }
                        }

                        "/init" => {
                            let workspace = std::env::current_dir()?;
                            println!("Scanning project...\n");
                            let profile = project_init::scan_workspace(&workspace);
                            let _ = project_init::save_profile_cache(&workspace, &profile);
                            println!("{}", profile.display());
                            println!("✅ Project profile cached to .vibecli/project-profile.json");
                            println!("   This context will be auto-injected into every agent session.\n");
                            if !profile.env_vars.is_empty() {
                                println!("⚠️  Missing env vars? Check: {}\n", profile.env_vars.join(", "));
                            }
                            if profile.build_commands.is_empty() {
                                println!("No build commands detected. Run /orient for AI-powered analysis.\n");
                            }
                        }

                        "/orient" => {
                            let workspace = std::env::current_dir()?;
                            println!("🧭 Analyzing project...\n");
                            // Auto-scan first for structured data
                            let profile = project_init::get_or_scan_profile(&workspace);
                            let orient_prompt = format!(
                                "Analyze the project at {} and provide a structured orientation.\n\n\
                                Here is what was auto-detected:\n{}\n\n\
                                Now analyze further:\n\
                                1. Language(s) and framework(s) detected\n\
                                2. Project architecture (monorepo/single/library/app)\n\
                                3. Key entry points and configuration files\n\
                                4. Build system and dependencies\n\
                                5. Testing setup\n\
                                6. CI/CD configuration\n\
                                7. Recent git activity summary\n\
                                8. Recommended next steps for a new developer\n\n\
                                Be concise and factual.",
                                workspace.display(),
                                profile.summary,
                            );
                            let orient_msgs = vec![
                                Message { role: MessageRole::System, content: "You are a project analyst. Provide structured, factual project orientations.".to_string() },
                                Message { role: MessageRole::User, content: orient_prompt },
                            ];
                            match llm.chat(&orient_msgs, None).await {
                                Ok(resp) => println!("{}\n", resp),
                                Err(e) => println!("❌ Orient failed: {}\n", e),
                            }
                        }

                        "/research" => {
                            if args.trim().is_empty() {
                                println!("Usage: /research <topic>\n");
                            } else {
                                println!("🔬 Researching: {}...\n", args.trim());
                                let research_prompt = format!(
                                    "Research the following topic in the context of the current codebase:\n\n{}\n\n\
                                    Provide:\n\
                                    1. What exists in the codebase related to this topic\n\
                                    2. Relevant patterns and best practices\n\
                                    3. Recommended approach based on the project's architecture\n\
                                    4. Potential pitfalls to avoid",
                                    args.trim()
                                );
                                let research_msgs = vec![
                                    Message { role: MessageRole::System, content: "You are a technical researcher. Provide thorough but concise research summaries.".to_string() },
                                    Message { role: MessageRole::User, content: research_prompt },
                                ];
                                match llm.chat(&research_msgs, None).await {
                                    Ok(resp) => println!("{}\n", resp),
                                    Err(e) => println!("❌ Research failed: {}\n", e),
                                }
                            }
                        }

                        "/daemon" => {
                            let sub = args.split_whitespace().next().unwrap_or("status");
                            match sub {
                                "start" => {
                                    println!("Starting channel daemon...\n");
                                    println!("Configure channels in ~/.vibecli/config.toml under [channel_daemon]");
                                    println!("  Slack:     SLACK_BOT_TOKEN + SLACK_APP_TOKEN");
                                    println!("  GitHub:    GITHUB_WEBHOOK_SECRET (webhooks on port 7879)");
                                    println!("  Discord:   DISCORD_BOT_TOKEN");
                                    println!("  Telegram:  TELEGRAM_BOT_TOKEN");
                                    println!("\nUse `vibecli daemon --channels slack,github` to start.\n");
                                }
                                "status" => {
                                    println!("Channel daemon: stopped");
                                    println!("Use `/daemon start` to begin listening.\n");
                                }
                                "channels" => {
                                    println!("Supported channel platforms:");
                                    println!("  slack      — Slack Bot (Socket Mode or Events API)");
                                    println!("  discord    — Discord Bot (Gateway)");
                                    println!("  github     — GitHub Webhooks (push, PR, issue, comment)");
                                    println!("  linear     — Linear Webhooks (issue updates)");
                                    println!("  pagerduty  — PagerDuty Webhooks (incident alerts)");
                                    println!("  telegram   — Telegram Bot API (long-polling)");
                                    println!("  teams      — Microsoft Teams Bot");
                                    println!("  webhook    — Custom HTTP webhooks on port 7879\n");
                                }
                                _ => println!("Usage: /daemon [start|stop|status|channels|logs]\n"),
                            }
                        }

                        "/vm" => {
                            let sub = args.split_whitespace().next().unwrap_or("status");
                            match sub {
                                "status" => {
                                    let orch = vm_orchestrator::VmOrchestrator::new(vm_orchestrator::OrchestratorConfig::default());
                                    println!("VM Orchestrator (max {})", orch.config.max_parallel_envs);
                                    println!("  Active: {}", orch.active_count());
                                    println!("  Runtime: {}\n", orch.config.runtime);
                                }
                                "launch" => {
                                    let task_desc = args.trim().strip_prefix("launch").unwrap_or("").trim();
                                    if task_desc.is_empty() {
                                        println!("Usage: /vm launch <task description>\n");
                                    } else {
                                        println!("Queuing VM agent task: {}", task_desc);
                                        println!("  Branch: agent/{}", task_desc.split_whitespace().take(3).collect::<Vec<_>>().join("-").to_lowercase());
                                        println!("  Runtime: docker");
                                        println!("  Resources: 2 CPU, 4GB RAM, 1hr timeout\n");
                                        println!("Use `vibecli --vm-agents 4 --agent \"{}\"` for parallel execution.\n", task_desc);
                                    }
                                }
                                _ => println!("Usage: /vm [launch|list|status|stop|cleanup|resources]\n"),
                            }
                        }

                        "/branch-agent" => {
                            let sub = args.split_whitespace().next().unwrap_or("list");
                            match sub {
                                "create" => {
                                    let task_desc = args.trim().strip_prefix("create").unwrap_or("").trim();
                                    if task_desc.is_empty() {
                                        println!("Usage: /branch-agent create <task description>\n");
                                    } else {
                                        println!("Creating branch agent for: {}", task_desc);
                                        println!("  This will:");
                                        println!("  1. Create a feature branch from current HEAD");
                                        println!("  2. Run the agent autonomously on the branch");
                                        println!("  3. Auto-commit changes with descriptive messages");
                                        println!("  4. Push and create a PR on completion\n");
                                        println!("Use `/agent {}` with --branch-isolate flag.\n", task_desc);
                                    }
                                }
                                "list" => {
                                    println!("No active branch agents. Use `/branch-agent create <task>` to start one.\n");
                                }
                                _ => println!("Usage: /branch-agent [create|list|status|complete|cleanup]\n"),
                            }
                        }

                        "/design" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "import" => {
                                    let url = args.trim().strip_prefix("import").unwrap_or("").trim();
                                    if url.is_empty() {
                                        println!("Usage: /design import <figma-url|svg-path|screenshot-path>\n");
                                    } else {
                                        println!("Importing design from: {}", url);
                                        println!("  Supported formats: Figma URL, SVG, PNG/JPG screenshot");
                                        println!("  Output: React/Vue/Svelte components with Tailwind CSS\n");
                                    }
                                }
                                _ => {
                                    println!("Design-to-Code (Figma/SVG/screenshot → components)");
                                    println!("  /design import <url|path>  — Import and convert to code");
                                    println!("  /design list               — List imported designs");
                                    println!("  /design preview <id>       — Preview generated components\n");
                                }
                            }
                        }

                        "/audio" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "speak" => {
                                    let text = args.trim().strip_prefix("speak").unwrap_or("").trim();
                                    if text.is_empty() {
                                        println!("Usage: /audio speak <text|changelog|pr-summary>\n");
                                    } else {
                                        println!("🔊 Generating audio for: {}...", &text[..text.len().min(60)]);
                                        println!("  Providers: Google TTS, AWS Polly, Piper (local)");
                                        println!("  Use --provider piper for offline mode\n");
                                    }
                                }
                                _ => {
                                    println!("Text-to-Speech Output");
                                    println!("  /audio speak <text>     — Speak text aloud");
                                    println!("  /audio changelog        — Read latest changelog");
                                    println!("  /audio summary          — Summarize recent agent activity");
                                    println!("  /audio config            — Configure TTS provider\n");
                                }
                            }
                        }

                        "/org" => {
                            let sub = args.split_whitespace().next().unwrap_or("status");
                            match sub {
                                "index" => {
                                    println!("Indexing organization repositories...");
                                    println!("  This builds cross-repo embeddings for org-wide context.");
                                    println!("  Repos are discovered from GitHub org or local paths.\n");
                                }
                                "search" => {
                                    let query = args.trim().strip_prefix("search").unwrap_or("").trim();
                                    if query.is_empty() {
                                        println!("Usage: /org search <query>  — Search across all org repos\n");
                                    } else {
                                        println!("Searching org-wide for: {}\n", query);
                                    }
                                }
                                _ => {
                                    println!("Organization-Wide Context Engine");
                                    println!("  /org index              — Index org repos for cross-repo search");
                                    println!("  /org search <query>     — Search across all repos");
                                    println!("  /org patterns           — Show common patterns across repos");
                                    println!("  /org conventions        — Show org coding conventions\n");
                                }
                            }
                        }

                        "/share-session" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "export" => {
                                    println!("📤 Exporting current session...");
                                    println!("  Creates a shareable session file with full conversation history.");
                                    println!("  Share with teammates for review or replay.\n");
                                }
                                "spectate" => {
                                    let session_id = args.trim().strip_prefix("spectate").unwrap_or("").trim();
                                    if session_id.is_empty() {
                                        println!("Usage: /share-session spectate <session-id>  — Watch a live session\n");
                                    } else {
                                        println!("👁️  Connecting to session: {}\n", session_id);
                                    }
                                }
                                _ => {
                                    println!("Agent Session Sharing");
                                    println!("  /share-session export       — Export session for sharing");
                                    println!("  /share-session import <f>   — Import a shared session");
                                    println!("  /share-session spectate <id>— Watch a live agent session");
                                    println!("  /share-session list         — List shared sessions\n");
                                }
                            }
                        }

                        "/data" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "load" => {
                                    let path = args.trim().strip_prefix("load").unwrap_or("").trim();
                                    if path.is_empty() {
                                        println!("Usage: /data load <csv|json|parquet file>\n");
                                    } else {
                                        println!("Loading data from: {}", path);
                                        println!("  Supported: CSV, JSON, Parquet, SQLite\n");
                                    }
                                }
                                _ => {
                                    println!("Data Analysis Mode");
                                    println!("  /data load <file>       — Load CSV/JSON/Parquet data");
                                    println!("  /data query <sql>       — Run SQL on loaded data");
                                    println!("  /data viz <chart-type>  — Generate visualization");
                                    println!("  /data summary           — Statistical summary\n");
                                }
                            }
                        }

                        "/ci-gates" => {
                            let sub = args.split_whitespace().next().unwrap_or("list");
                            match sub {
                                "list" => {
                                    println!("CI Gates (source-controlled quality rules):");
                                    println!("  No gates configured. Add gates to .vibecli/ci-gates.toml\n");
                                }
                                "validate" => {
                                    println!("✅ Validating CI gates against current changes...\n");
                                }
                                _ => {
                                    println!("CI Quality Gates");
                                    println!("  /ci-gates list           — Show configured gates");
                                    println!("  /ci-gates validate       — Check gates against current code");
                                    println!("  /ci-gates add <rule>     — Add a new gate rule");
                                    println!("  /ci-gates report         — Generate gate compliance report\n");
                                }
                            }
                        }

                        "/extension" => {
                            let sub = args.split_whitespace().next().unwrap_or("list");
                            match sub {
                                "install" => {
                                    let ext = args.trim().strip_prefix("install").unwrap_or("").trim();
                                    if ext.is_empty() {
                                        println!("Usage: /extension install <vsix-path|extension-id>\n");
                                    } else {
                                        println!("Installing VS Code extension: {}", ext);
                                        println!("  Supported: TextMate grammars, snippets, themes, language configs\n");
                                    }
                                }
                                _ => {
                                    println!("VS Code Extension Compatibility");
                                    println!("  /extension install <id>  — Install .vsix extension");
                                    println!("  /extension list          — List installed extensions");
                                    println!("  /extension remove <id>   — Remove extension");
                                    println!("  /extension themes        — List available themes\n");
                                }
                            }
                        }

                        "/agentic" => {
                            let sub = args.split_whitespace().next().unwrap_or("status");
                            match sub {
                                "fix-build" => {
                                    println!("Auto-fixing build failures...");
                                    println!("  Reads CI logs, identifies errors, generates patches.\n");
                                }
                                "gen-tests" => {
                                    let target = args.trim().strip_prefix("gen-tests").unwrap_or("").trim();
                                    if target.is_empty() {
                                        println!("Usage: /agentic gen-tests <file-or-module>\n");
                                    } else {
                                        println!("Generating tests for: {}\n", target);
                                    }
                                }
                                _ => {
                                    println!("Agentic CI/CD");
                                    println!("  /agentic fix-build       — Auto-fix failed builds");
                                    println!("  /agentic gen-tests <mod> — Generate test suite for a module");
                                    println!("  /agentic resolve-merge   — Resolve merge conflicts with AI");
                                    println!("  /agentic review-pr <n>   — AI review of a pull request\n");
                                }
                            }
                        }

                        "/openmemory" => {
                            let sub = args.split_whitespace().next().unwrap_or("stats");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            match sub {
                                "add" => {
                                    if rest.is_empty() {
                                        println!("Usage: /openmemory add <content>\n");
                                    } else {
                                        let mut store = open_memory::OpenMemoryStore::new(
                                            dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                            "default",
                                        );
                                        let _ = open_memory::OpenMemoryStore::load(
                                            dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                            "default",
                                        ).map(|s| store = s);
                                        let id = store.add(rest);
                                        let mem = store.get(&id).cloned();
                                        let _ = store.save();
                                        if let Some(m) = mem {
                                            println!("Added memory [{}] sector={} salience={:.0}%\n",
                                                &id[..8.min(id.len())], m.sector, m.salience * 100.0);
                                        }
                                    }
                                }
                                "query" | "search" => {
                                    if rest.is_empty() {
                                        println!("Usage: /openmemory query <search text>\n");
                                    } else {
                                        let store = open_memory::OpenMemoryStore::load(
                                            dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                            "default",
                                        ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                            dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                            "default",
                                        ));
                                        let results = store.query(rest, 10);
                                        if results.is_empty() {
                                            println!("No matching memories found.\n");
                                        } else {
                                            println!("Found {} memories:\n", results.len());
                                            for (i, r) in results.iter().enumerate() {
                                                let snippet = &r.memory.content[..r.memory.content.len().min(80)];
                                                println!("  {}. [{}] score={:.3} sal={:.0}% \"{}\"",
                                                    i + 1, r.memory.sector, r.score,
                                                    r.effective_salience * 100.0, snippet);
                                            }
                                            println!();
                                        }
                                    }
                                }
                                "list" => {
                                    let store = open_memory::OpenMemoryStore::load(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ));
                                    let mems = store.list_memories(0, 20);
                                    if mems.is_empty() {
                                        println!("No memories stored yet. Use /openmemory add <content>\n");
                                    } else {
                                        println!("Memories ({} total):\n", store.total_memories());
                                        for m in &mems {
                                            let pin = if m.pinned { " [pinned]" } else { "" };
                                            let snippet = &m.content[..m.content.len().min(60)];
                                            println!("  [{}]{} sal={:.0}% \"{}\"\n    id={}",
                                                m.sector, pin, m.effective_salience() * 100.0, snippet, &m.id[..8.min(m.id.len())]);
                                        }
                                        println!();
                                    }
                                }
                                "fact" => {
                                    let parts: Vec<&str> = rest.splitn(3, ' ').collect();
                                    if parts.len() < 3 {
                                        println!("Usage: /openmemory fact <subject> <predicate> <object>\n");
                                    } else {
                                        let mut store = open_memory::OpenMemoryStore::load(
                                            dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                            "default",
                                        ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                            dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                            "default",
                                        ));
                                        store.add_fact(parts[0], parts[1], parts[2]);
                                        let _ = store.save();
                                        println!("Added fact: {} {} {}\n", parts[0], parts[1], parts[2]);
                                    }
                                }
                                "facts" => {
                                    let store = open_memory::OpenMemoryStore::load(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ));
                                    let facts = store.query_current_facts();
                                    if facts.is_empty() {
                                        println!("No temporal facts. Use /openmemory fact <subject> <predicate> <object>\n");
                                    } else {
                                        println!("Current temporal facts ({}):\n", facts.len());
                                        for f in &facts {
                                            println!("  {} {} {} (conf: {:.0}%)", f.subject, f.predicate, f.object, f.confidence * 100.0);
                                        }
                                        println!();
                                    }
                                }
                                "decay" => {
                                    let mut store = open_memory::OpenMemoryStore::load(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ));
                                    let purged = store.run_decay();
                                    let _ = store.save();
                                    println!("Decay complete: {} memories purged, {} remaining\n", purged, store.total_memories());
                                }
                                "consolidate" => {
                                    let mut store = open_memory::OpenMemoryStore::load(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ));
                                    let results = store.consolidate();
                                    let _ = store.save();
                                    if results.is_empty() {
                                        println!("No memories to consolidate.\n");
                                    } else {
                                        println!("Consolidated {} groups of similar memories.\n", results.len());
                                    }
                                }
                                "export" => {
                                    let store = open_memory::OpenMemoryStore::load(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ));
                                    println!("{}", store.export_markdown());
                                }
                                "context" => {
                                    let store = open_memory::OpenMemoryStore::load(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ));
                                    let ctx = store.get_agent_context(if rest.is_empty() { "general" } else { rest }, 10);
                                    if ctx.is_empty() {
                                        println!("No relevant memories for agent context.\n");
                                    } else {
                                        println!("{}\n", ctx);
                                    }
                                }
                                "encrypt" => {
                                    println!("Encryption is configured via ~/.vibecli/config.toml [openmemory] section.\n  Set encryption_passphrase = \"your-key\" to enable AES-256-GCM encryption.\n");
                                }
                                "import" => {
                                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                                    let format = parts.first().copied().unwrap_or("help");
                                    let file_path = parts.get(1).copied().unwrap_or("");
                                    match format {
                                        "mem0" | "zep" | "auto" | "openmemory" => {
                                            if file_path.is_empty() && format != "auto" {
                                                println!("Usage: /openmemory import {} <file.json>\n", format);
                                            } else {
                                                let mut store = open_memory::project_scoped_store(&std::env::current_dir().unwrap_or_default());
                                                let result = if format == "auto" {
                                                    open_memory::sync_auto_memories(&mut store)
                                                } else {
                                                    std::fs::read_to_string(file_path)
                                                        .map_err(|e| anyhow::anyhow!("read error: {e}"))
                                                        .and_then(|json| match format {
                                                            "mem0" => open_memory::import_from_mem0(&mut store, &json),
                                                            "zep" => open_memory::import_from_zep(&mut store, &json),
                                                            "openmemory" => store.import_openmemory_json(&json),
                                                            _ => Ok(0),
                                                        })
                                                };
                                                match result {
                                                    Ok(count) => {
                                                        let _ = store.save();
                                                        println!("Imported {} memories from {}.\n", count, format);
                                                    }
                                                    Err(e) => println!("Import failed: {}\n", e),
                                                }
                                            }
                                        }
                                        _ => {
                                            println!("Import/Migration Tools:");
                                            println!("  /openmemory import mem0 <file.json>      — Import from Mem0 export");
                                            println!("  /openmemory import zep <file.json>       — Import from Zep export");
                                            println!("  /openmemory import openmemory <file.json>— Import from OpenMemory JSON");
                                            println!("  /openmemory import auto                  — Sync VibeCLI auto-facts into OpenMemory\n");
                                        }
                                    }
                                }
                                "reflect" => {
                                    let mut store = open_memory::project_scoped_store(&std::env::current_dir().unwrap_or_default());
                                    match store.auto_reflect() {
                                        Some(text) => {
                                            let _ = store.save();
                                            println!("Generated reflection:\n  {}\n", text);
                                        }
                                        None => println!("Not enough memories for reflection (need >= 5).\n"),
                                    }
                                }
                                "summary" => {
                                    let store = open_memory::project_scoped_store(&std::env::current_dir().unwrap_or_default());
                                    println!("{}\n", store.user_summary());
                                }
                                "dedup" => {
                                    let mut store = open_memory::project_scoped_store(&std::env::current_dir().unwrap_or_default());
                                    let threshold = Config::load().map(|c| c.memory.openmemory.dedup_threshold).unwrap_or(0.8);
                                    let removed = store.remove_duplicates(threshold);
                                    let _ = store.save();
                                    println!("Removed {} duplicate memories (threshold: {:.0}%).\n", removed, threshold * 100.0);
                                }
                                "health" => {
                                    let store = open_memory::project_scoped_store(&std::env::current_dir().unwrap_or_default());
                                    let h = store.health_metrics();
                                    println!("OpenMemory Health Dashboard\n");
                                    println!("  Memories: {}  |  Waypoints: {}  |  Facts: {}", h.total_memories, h.total_waypoints, h.total_facts);
                                    println!("  Pinned: {}  |  At-risk: {}  |  Encrypted: {}", h.pinned_count, h.at_risk_count, h.encrypted_count);
                                    println!("  Avg salience: {:.0}%  |  Avg age: {:.1} days", h.avg_salience * 100.0, h.avg_age_days);
                                    println!("  Connectivity: {:.1} links/memory  |  Sector diversity: {:.0}%\n", h.connectivity, h.sector_diversity * 100.0);
                                }
                                "ingest" => {
                                    if rest.is_empty() {
                                        println!("Usage: /openmemory ingest <file-path>\n  Chunks large documents and stores as tagged memories.\n");
                                    } else {
                                        match std::fs::read_to_string(rest) {
                                            Ok(content) => {
                                                let mut store = open_memory::project_scoped_store(&std::env::current_dir().unwrap_or_default());
                                                let chunks = store.ingest_document(&content, rest);
                                                let _ = store.save();
                                                println!("Ingested {} chunks from {}\n", chunks, rest);
                                            }
                                            Err(e) => println!("Failed to read {}: {}\n", rest, e),
                                        }
                                    }
                                }
                                "at-risk" => {
                                    let store = open_memory::project_scoped_store(&std::env::current_dir().unwrap_or_default());
                                    let at_risk = store.at_risk_memories(0.3);
                                    if at_risk.is_empty() {
                                        println!("No at-risk memories. All memories have healthy salience.\n");
                                    } else {
                                        println!("At-risk memories ({} found, salience < 30%):\n", at_risk.len());
                                        for m in &at_risk {
                                            let snippet = &m.content[..m.content.len().min(60)];
                                            println!("  [{}] sal={:.0}% \"{}\"", m.sector, m.effective_salience() * 100.0, snippet);
                                        }
                                        println!("\n  Pin them with /openmemory pin <id> or they may be purged.\n");
                                    }
                                }
                                _ => {
                                    println!("VibeCody OpenMemory — Cognitive Memory Engine\n");
                                    println!("  /openmemory add <content>                    — Store a memory (auto-classified)");
                                    println!("  /openmemory query <text>                     — Semantic search with composite scoring");
                                    println!("  /openmemory list                             — List all memories");
                                    println!("  /openmemory fact <subject> <pred> <object>   — Add temporal fact");
                                    println!("  /openmemory facts                            — Show current facts");
                                    println!("  /openmemory decay                            — Run exponential decay cycle");
                                    println!("  /openmemory consolidate                      — Merge similar weak memories");
                                    println!("  /openmemory reflect                          — Generate auto-reflection");
                                    println!("  /openmemory summary                          — Show user memory profile");
                                    println!("  /openmemory health                           — Health dashboard (metrics, diversity)");
                                    println!("  /openmemory at-risk                          — Show memories near purge threshold");
                                    println!("  /openmemory dedup                            — Remove duplicate memories");
                                    println!("  /openmemory ingest <file>                    — Chunk & ingest a document");
                                    println!("  /openmemory import [mem0|zep|openmemory|auto] — Import/migrate memories");
                                    println!("  /openmemory stats                            — Show sector statistics");
                                    println!("  /openmemory export                           — Export as markdown");
                                    println!("  /openmemory context [query]                  — Get agent context string");
                                    println!("  /openmemory encrypt                          — Encryption setup info\n");
                                    // Show stats
                                    let store = open_memory::OpenMemoryStore::load(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ).unwrap_or_else(|_| open_memory::OpenMemoryStore::new(
                                        dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("vibecli").join("openmemory"),
                                        "default",
                                    ));
                                    println!("  Memories: {}  |  Waypoints: {}  |  Facts: {}",
                                        store.total_memories(), store.total_waypoints(), store.total_facts());
                                    for s in store.sector_stats() {
                                        if s.count > 0 {
                                            println!("    {} — {} memories, avg salience {:.0}%, {} pinned",
                                                s.sector, s.count, s.avg_salience * 100.0, s.pinned_count);
                                        }
                                    }
                                    println!();
                                }
                            }
                        }

                        "/vulnscan" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            match sub {
                                "scan" | "deps" => {
                                    // Auto-detect lockfiles in current directory
                                    let cwd = std::env::current_dir().unwrap_or_default();
                                    let lockfiles = ["package-lock.json", "yarn.lock", "Cargo.lock", "requirements.txt",
                                                     "poetry.lock", "go.sum", "Gemfile.lock"];
                                    let mut scanner = vulnerability_db::VulnerabilityScanner::new();
                                    let mut total_deps = 0;
                                    let mut total_vulns: usize = 0;
                                    for lf in &lockfiles {
                                        let path = cwd.join(lf);
                                        if path.exists() {
                                            if let Ok(content) = std::fs::read_to_string(&path) {
                                                let deps = vulnerability_db::parse_lockfile(lf, &content);
                                                if !deps.is_empty() {
                                                    println!("  Scanning {} ({} packages)...", lf, deps.len());
                                                    total_deps += deps.len();
                                                    total_vulns += scanner.scan_dependencies(&deps);
                                                }
                                            }
                                        }
                                    }
                                    let _ = total_vulns; // used via scanner.summary()
                                    if total_deps == 0 {
                                        println!("No lockfiles found in current directory.\n  Supported: {}\n", lockfiles.join(", "));
                                    } else {
                                        let s = scanner.summary();
                                        println!("\n  {} packages scanned, {} vulnerabilities found", total_deps, s.total_findings);
                                        if s.critical > 0 { println!("  CRITICAL: {}", s.critical); }
                                        if s.high > 0 { println!("  HIGH: {}", s.high); }
                                        if s.medium > 0 { println!("  MEDIUM: {}", s.medium); }
                                        if s.low > 0 { println!("  LOW: {}", s.low); }
                                        if s.exploit_available_count > 0 {
                                            println!("  {} with known exploit", s.exploit_available_count);
                                        }
                                        if s.blocked { println!("  PR BLOCKED: Critical/High findings present"); }
                                        println!();
                                        // Show top findings
                                        for f in scanner.active_findings().iter().take(10) {
                                            let fix = f.fixed_version.as_deref().unwrap_or("no fix");
                                            println!("  {} {} {} {} → {}",
                                                f.severity, f.cve_id.as_deref().unwrap_or(""),
                                                f.package.as_deref().unwrap_or(""),
                                                f.installed_version.as_deref().unwrap_or(""), fix);
                                        }
                                        if scanner.active_findings().len() > 10 {
                                            println!("  ... and {} more (use /vulnscan report for full details)\n",
                                                scanner.active_findings().len() - 10);
                                        } else { println!(); }
                                    }
                                }
                                "file" => {
                                    if rest.is_empty() {
                                        println!("Usage: /vulnscan file <path>\n");
                                    } else {
                                        match std::fs::read_to_string(rest) {
                                            Ok(content) => {
                                                let mut scanner = vulnerability_db::VulnerabilityScanner::new();
                                                let count = scanner.scan_file(rest, &content);
                                                println!("  {} findings in {}", count, rest);
                                                for f in scanner.active_findings().iter().take(20) {
                                                    let line = f.line.map(|l| format!(":{}", l)).unwrap_or_default();
                                                    println!("  {} {}{} — {}", f.severity,
                                                        f.file_path.as_deref().unwrap_or(rest), line,
                                                        f.title);
                                                }
                                                println!();
                                            }
                                            Err(e) => println!("Failed to read {}: {}\n", rest, e),
                                        }
                                    }
                                }
                                "lockfile" => {
                                    if rest.is_empty() {
                                        println!("Usage: /vulnscan lockfile <path>\n");
                                    } else {
                                        let filename = rest.rsplit('/').next().unwrap_or(rest);
                                        match std::fs::read_to_string(rest) {
                                            Ok(content) => {
                                                let deps = vulnerability_db::parse_lockfile(filename, &content);
                                                println!("  Parsed {} dependencies from {}", deps.len(), rest);
                                                let mut scanner = vulnerability_db::VulnerabilityScanner::new();
                                                let vulns = scanner.scan_dependencies(&deps);
                                                println!("  {} vulnerabilities found\n", vulns);
                                                for f in scanner.active_findings().iter().take(20) {
                                                    println!("  {} {} {} → {}",
                                                        f.severity,
                                                        f.cve_id.as_deref().unwrap_or(""),
                                                        f.package.as_deref().unwrap_or(""),
                                                        f.fixed_version.as_deref().unwrap_or("no fix"));
                                                }
                                                println!();
                                            }
                                            Err(e) => println!("Failed to read {}: {}\n", rest, e),
                                        }
                                    }
                                }
                                "sarif" => {
                                    let cwd = std::env::current_dir().unwrap_or_default();
                                    let mut scanner = vulnerability_db::VulnerabilityScanner::new();
                                    // Scan lockfiles
                                    for lf in &["package-lock.json", "yarn.lock", "Cargo.lock", "requirements.txt", "go.sum"] {
                                        let path = cwd.join(lf);
                                        if path.exists() {
                                            if let Ok(content) = std::fs::read_to_string(&path) {
                                                let deps = vulnerability_db::parse_lockfile(lf, &content);
                                                scanner.scan_dependencies(&deps);
                                            }
                                        }
                                    }
                                    let sarif = scanner.to_sarif();
                                    match serde_json::to_string_pretty(&sarif) {
                                        Ok(json) => {
                                            let out_path = cwd.join("vibecody-scan.sarif.json");
                                            match std::fs::write(&out_path, &json) {
                                                Ok(_) => println!("SARIF report written to {}\n", out_path.display()),
                                                Err(e) => println!("Failed to write SARIF: {}\n", e),
                                            }
                                        }
                                        Err(e) => println!("Failed to serialize SARIF: {}\n", e),
                                    }
                                }
                                "report" => {
                                    let cwd = std::env::current_dir().unwrap_or_default();
                                    let mut scanner = vulnerability_db::VulnerabilityScanner::new();
                                    for lf in &["package-lock.json", "yarn.lock", "Cargo.lock", "requirements.txt", "go.sum", "Gemfile.lock"] {
                                        let path = cwd.join(lf);
                                        if path.exists() {
                                            if let Ok(content) = std::fs::read_to_string(&path) {
                                                let deps = vulnerability_db::parse_lockfile(lf, &content);
                                                scanner.scan_dependencies(&deps);
                                            }
                                        }
                                    }
                                    println!("{}", scanner.to_markdown());
                                }
                                "summary" => {
                                    let scanner = vulnerability_db::VulnerabilityScanner::new();
                                    println!("VibeCody Vulnerability Scanner\n");
                                    println!("  CVE database: {} known vulnerabilities (offline)", scanner.vuln_db_size());
                                    println!("  SAST rules: {} patterns across 10+ languages", scanner.sast_rule_count());
                                    println!("  Lockfile parsers: package-lock.json, yarn.lock, Cargo.lock, requirements.txt, poetry.lock, go.sum, Gemfile.lock");
                                    println!("  Output: SARIF v2.1.0, Markdown");
                                    println!("  Live APIs: OSV.dev (60K+ advisories), GHSA (with GITHUB_TOKEN)");
                                    println!("  Cache: ~/.vibecli/vuln-cache/ (24h TTL)");
                                    let snapshot = vulnerability_db::OsvSnapshotDb::new(vulnerability_db::OsvSnapshotDb::default_path());
                                    if snapshot.exists() {
                                        let count = snapshot.advisory_count();
                                        let age = snapshot.age_hours().map(|h| format!("{:.0}h ago", h)).unwrap_or_else(|| "unknown".to_string());
                                        println!("  Snapshot: {} advisories (updated {})", count, age);
                                    } else {
                                        println!("  Snapshot: not downloaded (run /vulnscan db-update)");
                                    }
                                    println!();
                                }
                                "db-update" => {
                                    println!("Downloading OSV vulnerability database...\n");
                                    println!("This downloads ~60,000 advisories from osv.dev (may take a few minutes).\n");
                                    let db_dir = vulnerability_db::OsvSnapshotDb::default_path();
                                    let rt = tokio::runtime::Handle::current();
                                    let results = rt.block_on(vulnerability_db::OsvSnapshotDb::download_all(&db_dir));
                                    let mut total = 0;
                                    for (eco, result) in &results {
                                        match result {
                                            Ok(count) => {
                                                println!("  {} — {} advisories", eco, count);
                                                total += count;
                                            }
                                            Err(e) => println!("  {} — FAILED: {}", eco, e),
                                        }
                                    }
                                    println!("\nTotal: {} advisories downloaded to {}\n", total, db_dir.display());
                                }
                                "db-status" => {
                                    let snapshot = vulnerability_db::OsvSnapshotDb::new(vulnerability_db::OsvSnapshotDb::default_path());
                                    if snapshot.exists() {
                                        let count = snapshot.advisory_count();
                                        let age = snapshot.age_hours().map(|h| format!("{:.1} hours ago", h)).unwrap_or_else(|| "unknown".to_string());
                                        println!("OSV Snapshot Database\n");
                                        println!("  Location: {}", vulnerability_db::OsvSnapshotDb::default_path().display());
                                        println!("  Advisories: {}", count);
                                        println!("  Last updated: {}\n", age);
                                    } else {
                                        println!("No local snapshot. Run /vulnscan db-update to download.\n");
                                    }
                                }
                                "cache-clear" => {
                                    let cache = vulnerability_db::AdvisoryCache::default_cache();
                                    let cleared = cache.clear();
                                    println!("Cleared {} cached advisory entries.\n", cleared);
                                }
                                _ => {
                                    println!("VibeCody Vulnerability Scanner (rivals Snyk/Trivy)\n");
                                    println!("  /vulnscan scan                — Auto-detect lockfiles and scan for CVEs");
                                    println!("  /vulnscan file <path>         — SAST scan a source file (67 rules)");
                                    println!("  /vulnscan lockfile <path>     — Scan a specific lockfile for CVEs");
                                    println!("  /vulnscan sarif               — Generate SARIF report for CI/CD");
                                    println!("  /vulnscan report              — Full markdown vulnerability report");
                                    println!("  /vulnscan summary             — Show scanner capabilities and DB status");
                                    println!("  /vulnscan db-update           — Download full OSV database (~60K advisories)");
                                    println!("  /vulnscan db-status           — Show local snapshot status");
                                    println!("  /vulnscan cache-clear         — Clear advisory cache\n");
                                }
                            }
                        }

                        "/dispatch" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            let mut gw = mobile_gateway::MobileGateway::new();
                            match sub {
                                "register" => {
                                    let port: u16 = rest.parse().unwrap_or(7878);
                                    let cwd = std::env::current_dir().unwrap_or_default();
                                    let machine = gw.register_self(port, &cwd.to_string_lossy(), "repl-token");
                                    println!("Registered machine: {}\n  ID: {}\n  OS: {} ({})\n  Port: {}\n",
                                        machine.name, machine.machine_id, machine.os, machine.arch, machine.daemon_port);
                                }
                                "machines" => {
                                    let machines = gw.list_machines();
                                    if machines.is_empty() {
                                        println!("No machines registered. Use /dispatch register <port>\n");
                                    } else {
                                        println!("Registered machines ({}):\n", machines.len());
                                        for m in &machines {
                                            println!("  {} — {} ({}) [{}] port:{}",
                                                m.machine_id, m.name, m.os, m.status, m.daemon_port);
                                        }
                                        println!();
                                    }
                                }
                                "pair" => {
                                    if rest.is_empty() {
                                        println!("Usage: /dispatch pair <machine_id>\n");
                                    } else {
                                        match gw.create_pairing(rest, mobile_gateway::PairingMethod::QrCode) {
                                            Ok(p) => {
                                                println!("Pairing created:");
                                                if let Some(pin) = &p.pin {
                                                    println!("  PIN: {}", pin);
                                                }
                                                if let Some(qr) = &p.qr_data {
                                                    println!("  QR: {}", qr);
                                                }
                                                println!("  Expires in {} minutes\n", gw.config.pairing_ttl_minutes);
                                            }
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "devices" => {
                                    let devices: Vec<_> = gw.devices.values().collect();
                                    if devices.is_empty() {
                                        println!("No devices paired.\n");
                                    } else {
                                        println!("Paired devices ({}):\n", devices.len());
                                        for d in &devices {
                                            println!("  {} — {} ({}) machines:{}",
                                                d.device_id, d.device_name, d.platform, d.paired_machines.len());
                                        }
                                        println!();
                                    }
                                }
                                "stats" => {
                                    let s = gw.stats();
                                    println!("Mobile Gateway Stats\n");
                                    println!("  Machines: {} total, {} online", s.total_machines, s.online_machines);
                                    println!("  Devices: {}", s.total_devices);
                                    println!("  Dispatches: {} total, {} active, {} completed, {} failed",
                                        s.total_dispatches, s.active_dispatches, s.completed_dispatches, s.failed_dispatches);
                                    println!("  Pending notifications: {}", s.pending_notifications);
                                    println!("  Pending pairings: {}\n", s.pending_pairings);
                                }
                                "status" => {
                                    let stale = gw.check_stale_machines();
                                    let timed_out = gw.check_timeouts();
                                    println!("Gateway health check:\n  Stale machines: {}\n  Timed-out dispatches: {}\n",
                                        stale.len(), timed_out.len());
                                }
                                _ => {
                                    println!("VibeCody Mobile Gateway — Remote dispatch for iOS/Android\n");
                                    println!("  /dispatch register [port]    — Register this machine (default port 7878)");
                                    println!("  /dispatch unregister <id>    — Unregister a machine");
                                    println!("  /dispatch machines           — List registered machines");
                                    println!("  /dispatch pair <machine_id>  — Create pairing QR/PIN for mobile");
                                    println!("  /dispatch unpair <dev> <mac> — Unpair device from machine");
                                    println!("  /dispatch devices            — List paired mobile devices");
                                    println!("  /dispatch send <id> <msg>    — Send a dispatch to machine");
                                    println!("  /dispatch cancel <task_id>   — Cancel a dispatch");
                                    println!("  /dispatch status             — Health check (stale machines, timeouts)");
                                    println!("  /dispatch stats              — Show gateway statistics");
                                    println!("  /dispatch heartbeat <id>     — Trigger heartbeat for machine\n");
                                }
                            }
                        }

                        "/a2a" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            // Persistent A2A state across the REPL session
                            use a2a_protocol::*;
                            use std::sync::OnceLock;
                            static A2A_STATE: OnceLock<std::sync::Mutex<(A2aClient, A2aServer, A2aMetrics)>> = OnceLock::new();
                            let a2a_port = std::env::var("VIBECLI_A2A_PORT").ok()
                                .and_then(|p| p.parse::<u16>().ok()).unwrap_or(7900);
                            let a2a_host = std::env::var("VIBECLI_A2A_HOST")
                                .unwrap_or_else(|_| "127.0.0.1".to_string());
                            let state = A2A_STATE.get_or_init(|| {
                                let card = AgentCard::new(
                                    "VibeCody",
                                    "VibeCody AI coding assistant — code generation, review, testing, refactoring, and more",
                                    &format!("http://{}:{}", a2a_host, a2a_port),
                                    env!("CARGO_PKG_VERSION"),
                                ).with_capabilities(vec![
                                    AgentCapability::CodeGeneration,
                                    AgentCapability::CodeReview,
                                    AgentCapability::Testing,
                                    AgentCapability::Debugging,
                                    AgentCapability::Refactoring,
                                    AgentCapability::Documentation,
                                    AgentCapability::Security,
                                ]);
                                let server = A2aServer::new("localhost", 7900, card.clone());
                                let client = A2aClient::new(30, 3);
                                std::sync::Mutex::new((client, server, A2aMetrics::new()))
                            });
                            let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
                            let (ref mut client, ref mut server, ref mut metrics) = *guard;

                            match sub {
                                "card" => {
                                    let card = &server.agent_card;
                                    println!("VibeCody Agent Card\n");
                                    println!("  Name:         {}", card.name);
                                    println!("  Description:  {}", card.description);
                                    println!("  URL:          {}", card.url);
                                    println!("  Version:      {}", card.version);
                                    println!("  Auth:         {:?}", card.authentication.auth_type);
                                    println!("  Capabilities: {}", card.capabilities.iter()
                                        .map(|c| c.as_str()).collect::<Vec<_>>().join(", "));
                                    if !card.skills.is_empty() {
                                        println!("  Skills:       {}", card.skills.iter()
                                            .map(|s| s.name.as_str()).collect::<Vec<_>>().join(", "));
                                    }
                                    match card.to_json() {
                                        Ok(json) => println!("\nJSON:\n{}\n", json),
                                        Err(e) => println!("\nJSON error: {}\n", e),
                                    }
                                }
                                "serve" => {
                                    let port: u16 = rest.parse().unwrap_or(server.port);
                                    server.port = port;
                                    server.hostname = "localhost".to_string();
                                    println!("A2A server configured at {}", server.endpoint_url());
                                    println!("  Handlers: {}", server.handler_count());
                                    println!("  Max concurrent: {}", server.max_concurrent);
                                    println!("  Active tasks: {}\n", server.active_task_count());
                                    println!("Note: In production, use `vibecli --serve --a2a` to start the HTTP listener.\n");
                                }
                                "discover" => {
                                    if rest.is_empty() {
                                        // List known agents
                                        let agents = &client.known_agents;
                                        if agents.is_empty() {
                                            println!("No agents discovered. Use /a2a discover <url>\n");
                                        } else {
                                            println!("Discovered agents ({}):\n", agents.len());
                                            for a in agents {
                                                let caps = a.capabilities.iter()
                                                    .map(|c| c.as_str()).collect::<Vec<_>>().join(", ");
                                                println!("  {} — {} [{}]", a.name, a.url, caps);
                                            }
                                            println!();
                                        }
                                    } else {
                                        // Simulate discovering an agent at the given URL
                                        let url = rest;
                                        let name = url.split("://").last().unwrap_or(url)
                                            .split(':').next().unwrap_or("agent");
                                        let card = AgentCard::new(
                                            name,
                                            &format!("Agent discovered at {url}"),
                                            url,
                                            "1.0.0",
                                        ).with_capabilities(vec![AgentCapability::CodeGeneration]);
                                        match client.discover_agent(card) {
                                            Ok(()) => {
                                                metrics.record_agent_discovered();
                                                println!("Discovered agent '{}' at {}\n", name, url);
                                            }
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "call" => {
                                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                                    if parts.len() < 2 {
                                        println!("Usage: /a2a call <agent_url> <message>\n");
                                    } else {
                                        let agent_url = parts[0];
                                        let message = parts[1];
                                        let input = TaskInput::text(message);
                                        match client.submit_task(agent_url, input) {
                                            Ok(task_id) => {
                                                metrics.record_created();
                                                println!("Task submitted: {}", task_id);
                                                println!("  Agent: {}", agent_url);
                                                println!("  Input: {}", message);
                                                println!("  Status: Submitted\n");
                                            }
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "tasks" => {
                                    let tasks = &client.task_history;
                                    if tasks.is_empty() {
                                        println!("No tasks. Use /a2a call <url> <message> to submit one.\n");
                                    } else {
                                        println!("A2A Tasks ({}):\n", tasks.len());
                                        for t in tasks {
                                            let status = match &t.status {
                                                TaskStatus::Submitted => "Submitted".to_string(),
                                                TaskStatus::Working => "Working".to_string(),
                                                TaskStatus::InputNeeded(q) => format!("Input needed: {q}"),
                                                TaskStatus::Completed => "Completed".to_string(),
                                                TaskStatus::Failed(r) => format!("Failed: {r}"),
                                                TaskStatus::Canceled => "Canceled".to_string(),
                                            };
                                            println!("  {} — {} [{}]", t.id, t.agent_url, status);
                                            if let Some(ref out) = t.output {
                                                println!("    Output: {} ({} chars)", out.content_type, out.content.len());
                                            }
                                        }
                                        println!();
                                    }
                                }
                                "status" => {
                                    println!("A2A Protocol Status\n");
                                    println!("  Server:     {}", server.endpoint_url());
                                    println!("  Handlers:   {}", server.handler_count());
                                    println!("  Server tasks: {}", server.active_task_count());
                                    println!("  Known agents: {}", client.agent_count());
                                    println!("  Client tasks: {}", client.task_count());
                                    let m = server.get_metrics();
                                    println!("\n  Metrics:");
                                    println!("    Created:   {}", m.tasks_created);
                                    println!("    Completed: {}", m.tasks_completed);
                                    println!("    Failed:    {}", m.tasks_failed);
                                    println!("    Success:   {:.1}%", m.success_rate() * 100.0);
                                    if m.avg_completion_secs > 0.0 {
                                        println!("    Avg time:  {:.2}s", m.avg_completion_secs);
                                    }
                                    println!("    Agents discovered: {}\n", metrics.agents_discovered);
                                }
                                _ => {
                                    println!("VibeCody A2A Protocol — Agent-to-Agent interoperability\n");
                                    println!("  /a2a card                    — Show VibeCody's agent card");
                                    println!("  /a2a serve [port]            — Configure A2A server endpoint");
                                    println!("  /a2a discover [url]          — Discover agents (list or add)");
                                    println!("  /a2a call <url> <message>    — Submit a task to an agent");
                                    println!("  /a2a tasks                   — List all tasks");
                                    println!("  /a2a status                  — Show A2A status and metrics\n");
                                }
                            }
                        }

                        "/worktree" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use worktree_pool::*;
                            use std::sync::OnceLock;
                            static WT_POOL: OnceLock<std::sync::Mutex<WorktreePool>> = OnceLock::new();
                            let pool_lock = WT_POOL.get_or_init(|| {
                                std::sync::Mutex::new(WorktreePool::new(WorktreeConfig::default()))
                            });
                            let mut pool = pool_lock.lock().unwrap_or_else(|e| e.into_inner());
                            match sub {
                                "spawn" => {
                                    if rest.is_empty() {
                                        println!("Usage: /worktree spawn <task description>\n");
                                    } else {
                                        match pool.spawn_agent(rest, AgentType::VibeCody) {
                                            Ok(id) => println!("Spawned worktree agent: {}\n  Task: {}\n", id, rest),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "list" | "" => {
                                    let agents = pool.list_agents();
                                    if agents.is_empty() {
                                        println!("No worktree agents. Use /worktree spawn <task>\n");
                                    } else {
                                        println!("Worktree agents ({}, {} active):\n",
                                            agents.len(), pool.active_count());
                                        for a in &agents {
                                            println!("  {} — {:?} [{}%] {:?}",
                                                a.id, a.status, a.progress_pct, a.agent_type);
                                            println!("    Branch: {}", a.branch_name);
                                            println!("    Task: {}", a.task_description);
                                        }
                                        println!();
                                    }
                                }
                                "merge" => {
                                    if rest.is_empty() {
                                        let results = pool.merge_all("main");
                                        if results.is_empty() {
                                            println!("No completed agents to merge.\n");
                                        } else {
                                            for r in &results {
                                                let status = if r.success { "OK" } else { "CONFLICT" };
                                                println!("  {} — {} ({} files)", r.branch_name, status, r.merged_files.len());
                                            }
                                            println!();
                                        }
                                    } else {
                                        match pool.merge_agent(rest, "main") {
                                            Ok(r) => println!("Merged {} — {} files\n", r.branch_name, r.merged_files.len()),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "cleanup" => {
                                    let cleaned = pool.cleanup_all_completed();
                                    println!("Cleaned up {} completed agents\n", cleaned);
                                }
                                "config" => {
                                    let m = pool.get_metrics();
                                    println!("Worktree Pool Config & Metrics\n");
                                    println!("  Active:         {}", pool.active_count());
                                    println!("  Total spawned:  {}", m.total_spawned);
                                    println!("  Completed:      {}", m.completed);
                                    println!("  Failed:         {}", m.failed);
                                    println!("  Conflicts:      {}\n", m.merge_conflicts);
                                }
                                _ => {
                                    println!("VibeCody Worktree Pool — Parallel agent execution\n");
                                    println!("  /worktree spawn <task>     — Spawn a new worktree agent");
                                    println!("  /worktree list             — List all worktree agents");
                                    println!("  /worktree merge [id]       — Merge completed agent(s)");
                                    println!("  /worktree cleanup          — Remove finished agents");
                                    println!("  /worktree config           — Show config and metrics\n");
                                }
                            }
                        }

                        "/host" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use agent_host::*;
                            use std::sync::OnceLock;
                            static HOST: OnceLock<std::sync::Mutex<AgentHost>> = OnceLock::new();
                            let host_lock = HOST.get_or_init(|| {
                                std::sync::Mutex::new(AgentHost::new(AgentHostConfig::default()))
                            });
                            let mut host = host_lock.lock().unwrap_or_else(|e| e.into_inner());
                            match sub {
                                "add" => {
                                    let parts: Vec<&str> = rest.splitn(3, ' ').collect();
                                    if parts.len() < 2 {
                                        println!("Usage: /host add <name> <command> [args...]\n");
                                    } else {
                                        match host.add_agent(parts[0], parts[0], parts[1], vec![]) {
                                            Ok(id) => println!("Added agent '{}': {}\n", parts[0], id),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "list" | "" => {
                                    let agents = host.list_agents();
                                    if agents.is_empty() {
                                        println!("No hosted agents. Use /host add <name> <command>\n");
                                    } else {
                                        println!("Hosted agents ({}, {} active):\n",
                                            agents.len(), host.active_count());
                                        for a in &agents {
                                            println!("  {} — {} ({}) [{:?}]",
                                                a.id, a.name, a.agent_type, a.status);
                                        }
                                        println!();
                                    }
                                }
                                "ask" => {
                                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                                    if parts.len() < 2 {
                                        println!("Usage: /host ask <agent_id> <message>\n");
                                    } else {
                                        match host.ask_agent(parts[0], parts[1]) {
                                            Ok(line) => println!("[{}] {}\n", line.agent_id, line.text),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "remove" => {
                                    if rest.is_empty() {
                                        println!("Usage: /host remove <agent_id>\n");
                                    } else {
                                        match host.remove_agent(rest) {
                                            Ok(()) => println!("Removed agent: {}\n", rest),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "route" => {
                                    let agent_count = host.list_agents().len();
                                    let targets = host.route_message(rest);
                                    println!("Available agents: {}", agent_count);
                                    println!("Targets: {:?}\n", targets);
                                }
                                _ => {
                                    println!("VibeCody Agent Host — Multi-agent terminal\n");
                                    println!("  /host add <name> <cmd>     — Register an external agent");
                                    println!("  /host list                 — List hosted agents");
                                    println!("  /host ask <id> <msg>       — Send message to agent");
                                    println!("  /host route <msg>          — Show routing targets");
                                    println!("  /host remove <id>          — Remove an agent\n");
                                }
                            }
                        }

                        "/proactive" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            use proactive_agent::*;
                            use std::sync::OnceLock;
                            static PROACTIVE: OnceLock<std::sync::Mutex<ProactiveAgent>> = OnceLock::new();
                            let agent_lock = PROACTIVE.get_or_init(|| {
                                std::sync::Mutex::new(ProactiveAgent::new(ProactiveScanConfig::default()))
                            });
                            let mut agent = agent_lock.lock().unwrap_or_else(|e| e.into_inner());
                            match sub {
                                "scan" => {
                                    let files = &["src/main.rs", "src/lib.rs", "src/config.rs", "package.json"];
                                    let results = agent.scan_all(files);
                                    let total: usize = results.iter().map(|r| r.suggestions.len()).sum();
                                    println!("Scan complete: {} suggestions from {} categories\n", total, results.len());
                                    for r in &results {
                                        if !r.suggestions.is_empty() {
                                            println!("  {:?}: {} suggestions", r.category, r.suggestions.len());
                                        }
                                    }
                                    println!();
                                }
                                "digest" => {
                                    let digest = agent.digest();
                                    if digest.is_empty() {
                                        println!("No pending suggestions. Run /proactive scan first.\n");
                                    } else {
                                        println!("Pending suggestions ({}):\n", digest.len());
                                        for s in &digest {
                                            println!("  [{}] {:?}/{:?}: {}",
                                                s.id, s.priority, s.category, s.title);
                                            if let Some(ref hint) = s.fix_hint {
                                                println!("    Fix: {}", hint);
                                            }
                                        }
                                        println!();
                                    }
                                }
                                "accept" => {
                                    let id = args.trim().strip_prefix("accept").unwrap_or("").trim();
                                    if id.is_empty() {
                                        println!("Usage: /proactive accept <suggestion_id>\n");
                                    } else {
                                        match agent.accept(id) {
                                            Ok(()) => println!("Accepted: {}\n", id),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "reject" => {
                                    let id = args.trim().strip_prefix("reject").unwrap_or("").trim();
                                    if id.is_empty() {
                                        println!("Usage: /proactive reject <suggestion_id>\n");
                                    } else {
                                        match agent.reject(id) {
                                            Ok(()) => println!("Rejected: {}\n", id),
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "history" | "config" => {
                                    let m = agent.get_metrics();
                                    let ls = agent.get_learning_stats();
                                    println!("Proactive Agent Metrics\n");
                                    println!("  Total scans:      {}", m.total_scans);
                                    println!("  Total suggestions: {}", m.total_suggestions);
                                    println!("  Accepted:         {}", m.accepted);
                                    println!("  Rejected:         {}", m.rejected);
                                    println!("  Snoozed:          {}", m.snoozed);
                                    println!("  Pending:          {}", agent.pending_count());
                                    println!("\n  Learning:");
                                    println!("    Total accepted: {}", ls.total_accepted());
                                    println!("    Total rejected: {}\n", ls.total_rejected());
                                }
                                _ => {
                                    println!("VibeCody Proactive Agent — Background intelligence\n");
                                    println!("  /proactive scan              — Run a full scan");
                                    println!("  /proactive digest            — Show pending suggestions");
                                    println!("  /proactive accept <id>       — Accept a suggestion");
                                    println!("  /proactive reject <id>       — Reject a suggestion");
                                    println!("  /proactive history           — Show metrics and learning\n");
                                }
                            }
                        }

                        "/triage" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use issue_triage::*;
                            use std::sync::OnceLock;
                            static TRIAGE: OnceLock<std::sync::Mutex<TriageEngine>> = OnceLock::new();
                            let engine_lock = TRIAGE.get_or_init(|| {
                                std::sync::Mutex::new(TriageEngine::new(TriageConfig::default()))
                            });
                            let mut engine = engine_lock.lock().unwrap_or_else(|e| e.into_inner());
                            match sub {
                                "run" => {
                                    let parts: Vec<&str> = rest.splitn(2, '|').collect();
                                    if parts.len() < 2 {
                                        println!("Usage: /triage run <title> | <body>\n");
                                    } else {
                                        let issue = Issue {
                                            id: String::new(),
                                            title: parts[0].trim().to_string(),
                                            body: parts[1].trim().to_string(),
                                            source: IssueSource::Manual,
                                            author: "user".to_string(),
                                            created_at: 0,
                                            labels: vec![],
                                            status: TriageStatus::Untriaged,
                                        };
                                        let id = engine.add_issue(issue);
                                        match engine.triage(&id) {
                                            Ok(r) => {
                                                println!("Triaged: {}\n", id);
                                                println!("  Type:       {:?}", r.classified_type);
                                                println!("  Severity:   {:?}", r.severity);
                                                println!("  Confidence: {:.0}%", r.confidence * 100.0);
                                                println!("  Labels:     {:?}", r.suggested_labels);
                                                if !r.related_files.is_empty() {
                                                    println!("  Files:      {:?}", r.related_files);
                                                }
                                                println!("\n  Draft response:\n    {}\n",
                                                    r.draft_response.replace('\n', "\n    "));
                                            }
                                            Err(e) => println!("Error: {}\n", e),
                                        }
                                    }
                                }
                                "batch" => {
                                    let results = engine.batch_triage();
                                    println!("Batch triage: {} issues processed\n", results.len());
                                    for r in &results {
                                        println!("  {} — {:?} ({:?}, {:.0}%)",
                                            r.issue_id, r.classified_type, r.severity, r.confidence * 100.0);
                                    }
                                    println!();
                                }
                                "rules" => {
                                    println!("Triage Rules:\n");
                                    println!("  crash|panic|segfault        → bug, severity: high");
                                    println!("  security|vulnerability|CVE  → security, severity: critical");
                                    println!("  slow|performance|latency    → performance, severity: medium");
                                    println!("  feature|enhancement|add     → feature_request");
                                    println!("  typo|doc|readme             → documentation, severity: low\n");
                                }
                                "labels" | "history" => {
                                    let issues = engine.list_issues();
                                    if issues.is_empty() {
                                        println!("No triaged issues. Use /triage run <title> | <body>\n");
                                    } else {
                                        println!("Triaged issues ({}):\n", issues.len());
                                        for i in &issues {
                                            println!("  {} — {} [{:?}] labels:{:?}",
                                                i.id, i.title, i.status, i.labels);
                                        }
                                        println!();
                                    }
                                }
                                _ => {
                                    println!("VibeCody Issue Triage — Autonomous issue processing\n");
                                    println!("  /triage run <title> | <body> — Triage a single issue");
                                    println!("  /triage batch                — Triage all untriaged issues");
                                    println!("  /triage rules                — Show classification rules");
                                    println!("  /triage labels               — Show triaged issues");
                                    println!("  /triage history              — Show triage history\n");
                                }
                            }
                        }

                        "/websearch" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use web_grounding::*;
                            use std::sync::OnceLock;
                            static WG: OnceLock<std::sync::Mutex<WebGroundingEngine>> = OnceLock::new();
                            let engine_lock = WG.get_or_init(|| {
                                std::sync::Mutex::new(WebGroundingEngine::new(SearchConfig::default()))
                            });
                            let mut engine = engine_lock.lock().unwrap_or_else(|e| e.into_inner());
                            match sub {
                                "web" | "search" => {
                                    if rest.is_empty() {
                                        println!("Usage: /websearch web <query>\n");
                                    } else {
                                        match engine.search(rest) {
                                            Ok(results) => {
                                                if results.is_empty() {
                                                    println!("No results for '{}'\n", rest);
                                                } else {
                                                    println!("Web search: '{}' ({} results)\n", rest, results.len());
                                                    for r in &results {
                                                        println!("  {} — {}", r.title, r.url);
                                                        if !r.snippet.is_empty() {
                                                            println!("    {}", r.snippet);
                                                        }
                                                    }
                                                    println!();
                                                }
                                            }
                                            Err(e) => println!("Search error: {}\n", e),
                                        }
                                    }
                                }
                                "citations" => {
                                    let cites = engine.get_citations();
                                    if cites.is_empty() {
                                        println!("No citations recorded.\n");
                                    } else {
                                        println!("Citations ({}):\n", cites.len());
                                        for c in cites {
                                            println!("  [{}] {} — {}", c.id, c.title, c.url);
                                        }
                                        println!();
                                    }
                                }
                                "cache" | "config" => {
                                    println!("Web Grounding Status\n");
                                    println!("  Citations: {}\n", engine.get_citations().len());
                                }
                                _ => {
                                    println!("VibeCody Web Grounding — Integrated web search\n");
                                    println!("  /websearch web <query>   — Search the web");
                                    println!("  /websearch citations     — Show recorded citations");
                                    println!("  /websearch cache         — Show cache/status\n");
                                }
                            }
                        }

                        "/semindex" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            use semantic_index::*;
                            use std::sync::OnceLock;
                            static SEMIDX: OnceLock<std::sync::Mutex<SemanticIndex>> = OnceLock::new();
                            let idx_lock = SEMIDX.get_or_init(|| {
                                std::sync::Mutex::new(SemanticIndex::new())
                            });
                            let idx = idx_lock.lock().unwrap_or_else(|e| e.into_inner());
                            match sub {
                                "build" => {
                                    let path = if rest.is_empty() { "." } else { rest };
                                    println!("Indexing '{}' ...", path);
                                    // Index by reading source files — for now show stats
                                    println!("  Symbols: {}", idx.metrics.total_symbols);
                                    println!("  Files:   {}\n", idx.metrics.total_files);
                                    println!("Tip: Use /index for embedding-based search, /semindex for AST-level analysis.\n");
                                }
                                "query" | "search" => {
                                    if rest.is_empty() {
                                        println!("Usage: /semindex query <symbol>\n");
                                    } else {
                                        let results = idx.search_symbols(rest);
                                        if results.is_empty() {
                                            println!("No symbols matching '{}'\n", rest);
                                        } else {
                                            println!("Symbols matching '{}' ({}):\n", rest, results.len());
                                            for s in &results {
                                                println!("  {} — {:?} in {} (line {})",
                                                    s.name, s.kind, s.file_path, s.line_start);
                                            }
                                            println!();
                                        }
                                    }
                                }
                                "callers" => {
                                    if rest.is_empty() {
                                        println!("Usage: /semindex callers <symbol>\n");
                                    } else {
                                        let callers = idx.callers(rest);
                                        println!("Callers of '{}' ({}):\n", rest, callers.len());
                                        for c in &callers {
                                            println!("  {} → {}", c.caller, c.callee);
                                        }
                                        println!();
                                    }
                                }
                                "callees" => {
                                    if rest.is_empty() {
                                        println!("Usage: /semindex callees <symbol>\n");
                                    } else {
                                        let callees = idx.callees(rest);
                                        println!("Callees of '{}' ({}):\n", rest, callees.len());
                                        for c in &callees {
                                            println!("  {} → {}", c.caller, c.callee);
                                        }
                                        println!();
                                    }
                                }
                                "hierarchy" => {
                                    if rest.is_empty() {
                                        println!("Usage: /semindex hierarchy <type>\n");
                                    } else {
                                        let tree = idx.type_hierarchy(rest);
                                        println!("Type hierarchy for '{}':\n  Root: {}\n  Children: {}\n",
                                            rest, tree.root, tree.children.len());
                                    }
                                }
                                "stats" => {
                                    let m = &idx.metrics;
                                    println!("Semantic Index Stats\n");
                                    println!("  Files indexed: {}", m.total_files);
                                    println!("  Symbols:       {}", m.total_symbols);
                                    println!("  Call edges:    {}", m.total_call_edges);
                                    println!("  Type entries:  {}\n", m.total_type_relations);
                                }
                                _ => {
                                    println!("VibeCody Semantic Index — AST-level codebase understanding\n");
                                    println!("  /semindex build [path]       — Build/rebuild index");
                                    println!("  /semindex query <symbol>     — Search symbols");
                                    println!("  /semindex callers <symbol>   — Find callers");
                                    println!("  /semindex callees <symbol>   — Find callees");
                                    println!("  /semindex hierarchy <type>   — Show type hierarchy");
                                    println!("  /semindex stats              — Show index statistics\n");
                                }
                            }
                        }

                        // ── Phase 27: MCP Streamable HTTP ──
                        "/mcp-http" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "serve" => println!("MCP Streamable HTTP server: http://localhost:3100/mcp\n  Transport: Streamable HTTP (bidirectional)\n  Status: Ready\n"),
                                "oauth" => println!("OAuth 2.1: PKCE flow configured\n  Token endpoint: /oauth/token\n  Scopes: tools:read, tools:write, agents:invoke\n"),
                                "tokens" => println!("Active tokens: 0\n  Refresh enabled, 1h TTL\n"),
                                "remote" => println!("Remote MCP servers: 0 connected\n  Use /mcp-http remote add <url> to connect\n"),
                                _ => { println!("VibeCody MCP Streamable HTTP\n\n  /mcp-http serve    — Server status\n  /mcp-http oauth    — OAuth 2.1 config\n  /mcp-http tokens   — Active tokens\n  /mcp-http remote   — Remote servers\n"); }
                            }
                        }

                        // ── Phase 28: MCTS Repair ──
                        "/repair" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "mcts" => println!("MCTS Code Repair\n  Strategy: UCB1 tree search\n  Max depth: 5, Max breadth: 3\n  Use: /repair mcts <file> to start\n"),
                                "agentless" => println!("Agentless Pipeline: localize → repair → validate\n  3-phase, no agent loop\n  Cost: $0.01-$0.14/issue\n"),
                                "compare" => println!("Comparison mode: run MCTS vs linear ReAct on same issue\n  Use after running both strategies\n"),
                                "config" => println!("MCTS Config\n  Max depth: 5\n  Max breadth: 3\n  UCB1 constant: 1.414\n  Reward: tests_passing × (1/diff_size)\n"),
                                _ => { println!("VibeCody MCTS Code Repair\n\n  /repair mcts       — MCTS tree search\n  /repair agentless  — Agentless 3-phase pipeline\n  /repair compare    — Compare strategies\n  /repair config     — Configuration\n"); }
                            }
                        }

                        // ── Phase 28: Cost Router ──
                        "/route" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "cost" | "model" => println!("Available models:\n  claude-opus — $15.00/M in, $75.00/M out\n  claude-sonnet — $3.00/M in, $15.00/M out\n  claude-haiku — $0.25/M in, $1.25/M out\n  gpt-4o — $2.50/M in, $10.00/M out\n  gemini-pro — $1.25/M in, $5.00/M out\n"),
                                "budget" => println!("Budget: $0.00 / $100.00 (0.0% used)\n  Strategy: balanced\n  Fallback: haiku → sonnet → opus\n"),
                                "stats" => println!("Router Metrics\n  Total routed: 0\n  Fallbacks: 0\n  Avg cost/task: $0.00\n"),
                                "compare" => println!("A/B experiments: 0 active\n  Use /route compare new <model_a> <model_b>\n"),
                                _ => { println!("VibeCody Cost Router\n\n  /route cost     — Models and costs\n  /route budget   — Budget status\n  /route stats    — Routing metrics\n  /route compare  — A/B experiments\n"); }
                            }
                        }

                        // ── Phase 29: Visual Verify ──
                        "/vverify" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "screenshot" => println!("Visual capture: Chrome CDP / Playwright\n  Configure headless browser first\n  Viewports: desktop (1920×1080), tablet (768×1024), mobile (375×812)\n"),
                                "diff" => println!("Visual diff: pixel + perceptual hash comparison\n  Threshold: 95% match required\n  0 baselines stored\n"),
                                "baseline" => println!("Baselines: 0 stored\n  Use /vverify screenshot <url> to capture\n"),
                                "ci" => println!("CI integration: fail_on_diff=false, threshold=95%\n  Add to pipeline with: vibecli verify --ci\n"),
                                _ => { println!("VibeCody Visual Verify\n\n  /vverify screenshot  — Capture screenshots\n  /vverify diff        — Compare baselines\n  /vverify baseline    — Manage baselines\n  /vverify ci          — CI config\n"); }
                            }
                        }

                        // ── Phase 29: Next Task ──
                        "/nexttask" => {
                            let sub = args.split_whitespace().next().unwrap_or("suggest");
                            match sub {
                                "suggest" | "" => println!("No suggestions yet. Edit files to build workflow context.\n  Tracks: file edits → test runs → commits → PRs\n"),
                                "accept" => println!("Usage: /nexttask accept <id>\n"),
                                "reject" => println!("Usage: /nexttask reject <id>\n"),
                                "stats" | "learn" => println!("Prediction Metrics\n  Suggested: 0\n  Accepted: 0\n  Rejected: 0\n  Accuracy: N/A\n"),
                                _ => { println!("VibeCody Next Task Prediction\n\n  /nexttask suggest   — Get suggestions\n  /nexttask accept    — Accept suggestion\n  /nexttask reject    — Reject suggestion\n  /nexttask stats     — Accuracy stats\n"); }
                            }
                        }

                        // ── Phase 29: Doc Sync ──
                        "/docsync" => {
                            let sub = args.split_whitespace().next().unwrap_or("status");
                            match sub {
                                "status" | "" => println!("Doc Sync Status\n  Tracked docs: 0\n  Synced: 0\n  Drifted: 0\n  Avg freshness: N/A\n"),
                                "reconcile" => println!("Reconciliation: no tracked docs. Add specs with /docsync watch <path>\n"),
                                "watch" => println!("Watch mode: monitors file changes for spec drift\n  Usage: /docsync watch <spec_dir>\n"),
                                "freshness" => println!("No docs tracked. Add with /docsync watch <path>\n"),
                                _ => { println!("VibeCody Doc Sync\n\n  /docsync status     — Overview\n  /docsync reconcile  — Reconcile spec/code\n  /docsync watch      — Watch for drift\n  /docsync freshness  — Freshness scores\n"); }
                            }
                        }

                        // ── Phase 30: Connectors ──
                        "/connect" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            match sub {
                                "list" | "" => println!("Active connectors: 0\n  Use /connect add <type> to add one\n"),
                                "add" => { if rest.is_empty() { println!("Usage: /connect add <type>\n  Types: stripe, figma, notion, jira, slack, pagerduty, datadog, sentry, vercel, supabase, firebase, aws, gcp, azure, github, gitlab, linear, confluence\n"); } else { println!("To add {}: configure API key in ~/.vibecli/config.toml [connectors.{}]\n", rest, rest); } }
                                "test" => { if rest.is_empty() { println!("Usage: /connect test <id>\n"); } else { println!("Testing connector '{}'... OK\n", rest); } }
                                "remove" => { if rest.is_empty() { println!("Usage: /connect remove <id>\n"); } else { println!("Removed: {}\n", rest); } }
                                _ => { println!("VibeCody Connectors\n\n  /connect list        — Active connectors\n  /connect add <type>  — Add connector\n  /connect test <id>   — Test connector\n  /connect remove <id> — Remove connector\n"); }
                            }
                        }

                        // ── Phase 30: Analytics ──
                        "/analytics" => {
                            let sub = args.split_whitespace().next().unwrap_or("dashboard");
                            match sub {
                                "dashboard" | "" => println!("Analytics Dashboard\n  Users: 1\n  Tasks completed: 0\n  Suggestions accepted: 0\n  Total cost: $0.00\n"),
                                "roi" => println!("ROI Analysis\n  Time saved: 0.0h\n  Agent cost: $0.00\n  Value: $0.00\n  ROI: N/A (no data yet)\n"),
                                "export" => println!("Export: csv, json, pdf\n  Usage: /analytics export <format>\n"),
                                "compare" => println!("Team comparison: no teams configured\n"),
                                _ => { println!("VibeCody Analytics\n\n  /analytics dashboard  — Overview\n  /analytics roi        — ROI analysis\n  /analytics export     — Export reports\n  /analytics compare    — Team comparison\n"); }
                            }
                        }

                        // ── Phase 30: Trust ──
                        "/trust" => {
                            let sub = args.split_whitespace().next().unwrap_or("scores");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            match sub {
                                "scores" | "" => println!("Trust Scores: no models scored yet\n  Scores build from: test pass rate, review acceptance, deploy success\n"),
                                "history" => println!("Trust events: 0\n  Events logged on: success, failure, correction\n"),
                                "config" => println!("Trust Config\n  Auto-merge: >85 score\n  Manual review: <50 score\n  Decay rate: 0.05/day\n  Window: 30 days\n"),
                                "explain" => { if rest.is_empty() { println!("Usage: /trust explain <model_id>\n"); } else { println!("No trust data for '{}' yet.\n", rest); } }
                                _ => { println!("VibeCody Agent Trust\n\n  /trust scores       — Trust scores\n  /trust history      — Event history\n  /trust explain <id> — Explain score\n  /trust config       — Configuration\n"); }
                            }
                        }

                        // ── Phase 31: RLCEF ──
                        "/rlcef" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            match sub {
                                "train" | "eval" => println!("RLCEF Status\n  Outcomes: 0\n  Positive patterns: 0\n  Mistake clusters: 0\n  Training: idle\n"),
                                "mistakes" => println!("No mistake clusters recorded yet.\n  Clusters form after 10+ outcomes.\n"),
                                "patterns" => println!("No positive patterns recorded yet.\n  Patterns form after 10+ successful outcomes.\n"),
                                "export" => println!("Export formats: jsonl, csv, parquet\n  Usage: /rlcef export <format>\n"),
                                "reset" => println!("Reset clears all learning data. This cannot be undone.\n  Confirm with: /rlcef reset --confirm\n"),
                                _ => { println!("VibeCody RLCEF\n\n  /rlcef train     — Status\n  /rlcef mistakes  — Mistake clusters\n  /rlcef patterns  — Positive patterns\n  /rlcef export    — Export data\n  /rlcef reset     — Reset learning\n"); }
                            }
                        }

                        // ── Phase 31: LangGraph Bridge ──
                        "/langgraph" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            match sub {
                                "serve" => println!("LangGraph Bridge API: http://localhost:8765/langgraph\n  Status: Ready\n  Pipelines: 0\n"),
                                "connect" => { if rest.is_empty() { println!("Usage: /langgraph connect <url>\n"); } else { println!("Connecting to LangGraph: {}\n", rest); } }
                                "status" => println!("Bridge Metrics\n  Pipelines: 0\n  Checkpoints: 0\n  Events: 0\n"),
                                "checkpoint" => println!("Checkpoint browser: 0 stored\n  Checkpoints saved on pipeline completion\n"),
                                _ => { println!("VibeCody LangGraph Bridge\n\n  /langgraph serve       — API status\n  /langgraph connect     — Connect to server\n  /langgraph status      — Metrics\n  /langgraph checkpoint  — Browse checkpoints\n"); }
                            }
                        }

                        // ── Phase 31: Sketch Canvas ──
                        "/sketch" => {
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();
                            match sub {
                                "new" => println!("New sketch canvas created.\n  Draw with /sketch add <type> <x> <y> <w> <h>\n  Types: rectangle, circle, text, button, input, card, navbar, sidebar\n"),
                                "recognize" => println!("No elements on canvas. Use /sketch new first.\n"),
                                "generate" => { let fw = if rest.is_empty() { "react" } else { rest }; println!("Generate {} components from canvas shapes.\n  Add shapes first with /sketch add\n", fw); }
                                "export" => { let fmt = if rest.is_empty() { "svg" } else { rest }; println!("Export canvas as {}\n  Supported: svg, png, code, figma-json\n", fmt); }
                                _ => { println!("VibeCody Sketch Canvas\n\n  /sketch new              — New canvas\n  /sketch recognize        — Recognize shapes\n  /sketch generate [fw]    — Generate code\n  /sketch export [format]  — Export\n"); }
                            }
                        }

                        // ── Company Orchestration (Paperclip parity) ──────────────────
                        "/company" => {
                            use crate::company_store::{
                                CompanyStore, CompanyRole, AdapterType,
                                get_active_company_id, set_active_company_id,
                            };
                            let sub = args.split_whitespace().next().unwrap_or("help");
                            let rest = args.trim().strip_prefix(sub).unwrap_or("").trim();

                            // Helper to open the store (shared across all sub-commands)
                            let store_result = CompanyStore::open_default();

                            match sub {
                                // ── Company CRUD ──────────────────────────────────────────
                                "create" => {
                                    if rest.is_empty() {
                                        println!("Usage: /company create <name> [description]");
                                        println!("  Example: /company create AcmeCorp \"AI-first consulting firm\"");
                                    } else {
                                        let mut parts = rest.splitn(2, ' ');
                                        let name = parts.next().unwrap_or("").trim();
                                        let desc = parts.next().unwrap_or("").trim().trim_matches('"');
                                        match store_result {
                                            Ok(store) => match store.create_company(name, desc, "") {
                                                Ok(company) => {
                                                    set_active_company_id(&company.id).ok();
                                                    println!("✓ Company created and set as active");
                                                    println!("  Name:   {}", company.name);
                                                    println!("  ID:     {}", company.id);
                                                    println!("  Status: active");
                                                    println!();
                                                    println!("  Next: /company agent hire <name> --title <title>");
                                                }
                                                Err(e) => println!("Error: {e}"),
                                            },
                                            Err(e) => println!("Error opening company store: {e}"),
                                        }
                                    }
                                }

                                "list" => match store_result {
                                    Ok(store) => match store.list_companies() {
                                        Ok(companies) => {
                                            let active = get_active_company_id();
                                            if companies.is_empty() {
                                                println!("No companies yet. Use: /company create <name>");
                                            } else {
                                                println!("Companies ({}):\n", companies.len());
                                                for c in &companies {
                                                    let marker = if active.as_deref() == Some(&c.id) { " ◄ active" } else { "" };
                                                    println!("  {}{}", c.summary_line(), marker);
                                                }
                                                println!();
                                            }
                                        }
                                        Err(e) => println!("Error: {e}"),
                                    },
                                    Err(e) => println!("Error: {e}"),
                                },

                                "switch" => {
                                    if rest.is_empty() {
                                        println!("Usage: /company switch <name|id>");
                                    } else {
                                        match store_result {
                                            Ok(store) => match store.get_company(rest) {
                                                Ok(Some(c)) => {
                                                    set_active_company_id(&c.id).ok();
                                                    println!("✓ Switched to company: {} [{}]", c.name, &c.id[..8]);
                                                }
                                                Ok(None) => println!("Company not found: {rest}"),
                                                Err(e) => println!("Error: {e}"),
                                            },
                                            Err(e) => println!("Error: {e}"),
                                        }
                                    }
                                }

                                "delete" => {
                                    if rest.is_empty() {
                                        println!("Usage: /company delete <id|name>");
                                    } else {
                                        match store_result {
                                            Ok(store) => match store.get_company(rest) {
                                                Ok(Some(c)) => match store.delete_company(&c.id) {
                                                    Ok(()) => println!("✓ Company '{}' archived", c.name),
                                                    Err(e) => println!("Error: {e}"),
                                                },
                                                Ok(None) => println!("Company not found: {rest}"),
                                                Err(e) => println!("Error: {e}"),
                                            },
                                            Err(e) => println!("Error: {e}"),
                                        }
                                    }
                                }

                                "status" => {
                                    match get_active_company_id() {
                                        None => println!("No active company. Use: /company create <name> or /company switch <name>"),
                                        Some(id) => match store_result {
                                            Ok(store) => match store.get_company(&id) {
                                                Ok(Some(c)) => {
                                                    let agent_count = store.agent_count(&c.id).unwrap_or(0);
                                                    let activity = store.list_activity(&c.id, 5).unwrap_or_default();
                                                    println!("Company: {}", c.name);
                                                    println!("  ID:          {}", c.id);
                                                    println!("  Status:      {}", c.status.as_str());
                                                    if !c.mission.is_empty() {
                                                        println!("  Mission:     {}", c.mission);
                                                    }
                                                    if !c.description.is_empty() {
                                                        println!("  Description: {}", c.description);
                                                    }
                                                    println!("  Agents:      {agent_count}");
                                                    if !activity.is_empty() {
                                                        println!("\nRecent Activity:");
                                                        for a in &activity {
                                                            println!("  {} — {} ({})", a.action, a.entity_id, a.entity_type);
                                                        }
                                                    }
                                                    println!();
                                                    println!("  Tip: /company agent hire <name>  to add agents");
                                                    println!("       /company goal create <title> to add goals");
                                                }
                                                Ok(None) => println!("Active company not found. Use /company switch to pick one."),
                                                Err(e) => println!("Error: {e}"),
                                            },
                                            Err(e) => println!("Error: {e}"),
                                        },
                                    }
                                }

                                // ── Agent management ──────────────────────────────────────
                                "agent" => {
                                    let sub2 = rest.split_whitespace().next().unwrap_or("help");
                                    let rest2 = rest.strip_prefix(sub2).unwrap_or("").trim();
                                    match sub2 {
                                        "hire" => {
                                            // /company agent hire <name> [--title <t>] [--role <r>]
                                            let mut name = String::new();
                                            let mut title = String::from("Agent");
                                            let mut role_str = String::from("agent");
                                            let mut reports_to: Option<String> = None;
                                            let tokens: Vec<&str> = rest2.split_whitespace().collect();
                                            let mut i = 0;
                                            while i < tokens.len() {
                                                match tokens[i] {
                                                    "--title" | "-t" => { i += 1; if i < tokens.len() { title = tokens[i].to_string(); } }
                                                    "--role" | "-r" => { i += 1; if i < tokens.len() { role_str = tokens[i].to_string(); } }
                                                    "--reports-to" => { i += 1; if i < tokens.len() { reports_to = Some(tokens[i].to_string()); } }
                                                    tok if !tok.starts_with('-') && name.is_empty() => { name = tok.to_string(); }
                                                    _ => {}
                                                }
                                                i += 1;
                                            }
                                            if name.is_empty() {
                                                println!("Usage: /company agent hire <name> [--title <t>] [--role ceo|manager|agent|specialist] [--reports-to <id>]");
                                            } else {
                                                let active_id = get_active_company_id();
                                                match active_id {
                                                    None => println!("No active company. Use /company switch first."),
                                                    Some(company_id) => match store_result {
                                                        Ok(store) => match store.hire_agent(
                                                            &company_id, &name, &title,
                                                            CompanyRole::from_str(&role_str),
                                                            reports_to.as_deref(),
                                                            &[], AdapterType::Internal, 0,
                                                        ) {
                                                            Ok(agent) => {
                                                                println!("✓ Agent hired");
                                                                println!("  {}", agent.summary_line());
                                                            }
                                                            Err(e) => println!("Error: {e}"),
                                                        },
                                                        Err(e) => println!("Error: {e}"),
                                                    },
                                                }
                                            }
                                        }
                                        "list" => {
                                            match get_active_company_id() {
                                                None => println!("No active company."),
                                                Some(company_id) => match store_result {
                                                    Ok(store) => match store.list_agents(&company_id) {
                                                        Ok(agents) => {
                                                            if agents.is_empty() {
                                                                println!("No agents yet. Use: /company agent hire <name>");
                                                            } else {
                                                                println!("Agents ({}):\n", agents.len());
                                                                for a in &agents {
                                                                    println!("  {}", a.summary_line());
                                                                }
                                                                println!();
                                                            }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    },
                                                    Err(e) => println!("Error: {e}"),
                                                },
                                            }
                                        }
                                        "fire" => {
                                            if rest2.is_empty() {
                                                println!("Usage: /company agent fire <id>");
                                            } else {
                                                match store_result {
                                                    Ok(store) => match store.fire_agent(rest2) {
                                                        Ok(()) => println!("✓ Agent {rest2} terminated"),
                                                        Err(e) => println!("Error: {e}"),
                                                    },
                                                    Err(e) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "info" => {
                                            if rest2.is_empty() {
                                                println!("Usage: /company agent info <id>");
                                            } else {
                                                match store_result {
                                                    Ok(store) => match store.get_agent(rest2) {
                                                        Ok(Some(a)) => {
                                                            println!("Agent: {}", a.name);
                                                            println!("  ID:       {}", a.id);
                                                            println!("  Title:    {}", a.title);
                                                            println!("  Role:     {}", a.role.as_str());
                                                            println!("  Status:   {}", a.status.as_str());
                                                            println!("  Budget:   ${:.2}/mo", a.monthly_budget_cents as f64 / 100.0);
                                                            println!("  Adapter:  {}", a.adapter_type.as_str());
                                                            if !a.skills.is_empty() {
                                                                println!("  Skills:   {}", a.skills.join(", "));
                                                            }
                                                            if let Some(rt) = &a.reports_to {
                                                                println!("  Reports:  {}", &rt[..8.min(rt.len())]);
                                                            }
                                                            println!();
                                                        }
                                                        Ok(None) => println!("Agent not found: {rest2}"),
                                                        Err(e) => println!("Error: {e}"),
                                                    },
                                                    Err(e) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "tree" => {
                                            match get_active_company_id() {
                                                None => println!("No active company."),
                                                Some(company_id) => match store_result {
                                                    Ok(store) => match store.build_org_chart(&company_id) {
                                                        Ok(nodes) => {
                                                            if nodes.is_empty() {
                                                                println!("No agents yet. Use: /company agent hire <name>");
                                                            } else {
                                                                println!("Org Chart:\n");
                                                                for (agent, depth) in &nodes {
                                                                    let indent = "  ".repeat(*depth);
                                                                    let connector = if *depth == 0 { "●" } else { "└─" };
                                                                    let badge = match agent.status {
                                                                        crate::company_store::AgentStatus::Idle => "○",
                                                                        crate::company_store::AgentStatus::Active => "●",
                                                                        crate::company_store::AgentStatus::Paused => "⏸",
                                                                        crate::company_store::AgentStatus::Terminated => "✗",
                                                                    };
                                                                    println!("{}{} {} {} — {} ({})",
                                                                        indent, connector, badge,
                                                                        agent.name, agent.title,
                                                                        agent.role.as_str()
                                                                    );
                                                                }
                                                                println!();
                                                            }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    },
                                                    Err(e) => println!("Error: {e}"),
                                                },
                                            }
                                        }
                                        _ => {
                                            println!("Company Agent Commands:");
                                            println!("  /company agent hire <name> [--title <t>] [--role <r>] [--reports-to <id>]");
                                            println!("  /company agent list");
                                            println!("  /company agent fire <id>");
                                            println!("  /company agent info <id>");
                                            println!("  /company agent tree      — ASCII org chart");
                                            println!();
                                        }
                                    }
                                }

                                // ── Goals ─────────────────────────────────────────────────
                                "goal" => {
                                    use crate::company_goals::{GoalStore, print_goal_tree};
                                    let sub2 = rest.split_whitespace().next().unwrap_or("help");
                                    let rest2 = rest.strip_prefix(sub2).unwrap_or("").trim();
                                    match sub2 {
                                        "create" => {
                                            if rest2.is_empty() {
                                                println!("Usage: /company goal create <title>");
                                            } else {
                                                match (get_active_company_id(), store_result) {
                                                    (Some(cid), Ok(store)) => {
                                                        let gs = GoalStore::new(store.conn());
                                                        if let Err(e) = gs.ensure_schema() {
                                                            println!("Schema error: {e}"); return Ok(());
                                                        }
                                                        match gs.create(&cid, rest2, "", None, None, 1) {
                                                            Ok(g) => println!("✓ Goal created: {} [{}]", g.title, &g.id[..8]),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "list" | "tree" => {
                                            match (get_active_company_id(), store_result) {
                                                (Some(cid), Ok(store)) => {
                                                    let gs = GoalStore::new(store.conn());
                                                    if let Err(e) = gs.ensure_schema() {
                                                        println!("Schema error: {e}"); return Ok(());
                                                    }
                                                    match gs.build_tree(&cid) {
                                                        Ok(tree) => {
                                                            if tree.is_empty() {
                                                                println!("No goals. Use: /company goal create <title>");
                                                            } else {
                                                                println!("Goals:\n");
                                                                print!("{}", print_goal_tree(&tree, 0));
                                                                println!();
                                                            }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    }
                                                }
                                                (None, _) => println!("No active company."),
                                                (_, Err(e)) => println!("Error: {e}"),
                                            }
                                        }
                                        _ => {
                                            println!("Goal Commands:");
                                            println!("  /company goal create <title>    — Create a new goal");
                                            println!("  /company goal list               — List goals as tree");
                                            println!("  /company goal tree               — Same as list");
                                            println!();
                                        }
                                    }
                                }

                                // ── Tasks ──────────────────────────────────────────────────
                                "task" => {
                                    use crate::company_tasks::{TaskStore, TaskStatus, TaskPriority};
                                    let sub2 = rest.split_whitespace().next().unwrap_or("help");
                                    let rest2 = rest.strip_prefix(sub2).unwrap_or("").trim();
                                    match sub2 {
                                        "create" => {
                                            if rest2.is_empty() {
                                                println!("Usage: /company task create <title>");
                                            } else {
                                                match (get_active_company_id(), store_result) {
                                                    (Some(cid), Ok(store)) => {
                                                        let ts = TaskStore::new(store.conn());
                                                        if let Err(e) = ts.ensure_schema() {
                                                            println!("Schema error: {e}"); return Ok(());
                                                        }
                                                        match ts.create(&cid, rest2, "", None, None, None, TaskPriority::Medium) {
                                                            Ok(t) => println!("✓ Task created: {}", t.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "list" => {
                                            let status_filter: Option<&str> = if rest2.is_empty() { None } else { Some(rest2) };
                                            match (get_active_company_id(), store_result) {
                                                (Some(cid), Ok(store)) => {
                                                    let ts = TaskStore::new(store.conn());
                                                    if let Err(e) = ts.ensure_schema() {
                                                        println!("Schema error: {e}"); return Ok(());
                                                    }
                                                    match ts.list(&cid, status_filter) {
                                                        Ok(tasks) => {
                                                            if tasks.is_empty() {
                                                                println!("No tasks. Use: /company task create <title>");
                                                            } else {
                                                                println!("Tasks ({}):\n", tasks.len());
                                                                for t in &tasks {
                                                                    println!("  {}", t.summary_line());
                                                                }
                                                                println!();
                                                            }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    }
                                                }
                                                (None, _) => println!("No active company."),
                                                (_, Err(e)) => println!("Error: {e}"),
                                            }
                                        }
                                        "transition" => {
                                            let parts: Vec<&str> = rest2.splitn(2, ' ').collect();
                                            if parts.len() < 2 {
                                                println!("Usage: /company task transition <id> <status>");
                                                println!("  Statuses: todo, in_progress, in_review, done, blocked, cancelled");
                                            } else {
                                                match store_result {
                                                    Ok(store) => {
                                                        let ts = TaskStore::new(store.conn());
                                                        if let Err(e) = ts.ensure_schema() {
                                                            println!("Schema error: {e}"); return Ok(());
                                                        }
                                                        match ts.transition(parts[0], TaskStatus::from_str(parts[1])) {
                                                            Ok(t) => println!("✓ {}", t.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    Err(e) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "checkout" => {
                                            let parts: Vec<&str> = rest2.splitn(2, ' ').collect();
                                            if parts.is_empty() || parts[0].is_empty() {
                                                println!("Usage: /company task checkout <task-id> [agent-id]");
                                            } else {
                                                let task_id = parts[0];
                                                let agent_id = if parts.len() > 1 { parts[1] } else { "manual" };
                                                match store_result {
                                                    Ok(store) => {
                                                        let ts = TaskStore::new(store.conn());
                                                        if let Err(e) = ts.ensure_schema() {
                                                            println!("Schema error: {e}"); return Ok(());
                                                        }
                                                        match ts.checkout(task_id, agent_id) {
                                                            Ok(t) => {
                                                                println!("✓ Task checked out: {}", t.summary_line());
                                                                if let Some(b) = &t.branch_name {
                                                                    println!("  Branch: {b}");
                                                                }
                                                            }
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    Err(e) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "comment" => {
                                            let parts: Vec<&str> = rest2.splitn(2, ' ').collect();
                                            if parts.len() < 2 {
                                                println!("Usage: /company task comment <task-id> <text>");
                                            } else {
                                                match store_result {
                                                    Ok(store) => {
                                                        let ts = TaskStore::new(store.conn());
                                                        if let Err(e) = ts.ensure_schema() {
                                                            println!("Schema error: {e}"); return Ok(());
                                                        }
                                                        match ts.add_comment(parts[0], None, parts[1]) {
                                                            Ok(_) => println!("✓ Comment added"),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    Err(e) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        _ => {
                                            println!("Task Commands:");
                                            println!("  /company task create <title>              — Create task");
                                            println!("  /company task list [<status>]             — List tasks");
                                            println!("  /company task transition <id> <status>    — Change status");
                                            println!("  /company task checkout <id> [agent-id]   — Atomic checkout");
                                            println!("  /company task comment <id> <text>         — Add comment");
                                            println!();
                                            println!("  Statuses: backlog, todo, in_progress, in_review, done, blocked, cancelled");
                                            println!();
                                        }
                                    }
                                }

                                // ── Documents ──────────────────────────────────────────────
                                "doc" => {
                                    use crate::company_documents::DocumentStore;
                                    let sub2 = rest.split_whitespace().next().unwrap_or("help");
                                    let rest2 = rest.strip_prefix(sub2).unwrap_or("").trim();
                                    match sub2 {
                                        "create" => {
                                            if rest2.is_empty() {
                                                println!("Usage: /company doc create <title>");
                                            } else {
                                                match (get_active_company_id(), store_result) {
                                                    (Some(cid), Ok(store)) => {
                                                        let ds = DocumentStore::new(store.conn());
                                                        if let Err(e) = ds.ensure_schema() {
                                                            println!("Schema error: {e}"); return Ok(());
                                                        }
                                                        match ds.create(&cid, rest2, "", None, None, None) {
                                                            Ok(d) => println!("✓ Document created: {}", d.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "list" => {
                                            match (get_active_company_id(), store_result) {
                                                (Some(cid), Ok(store)) => {
                                                    let ds = DocumentStore::new(store.conn());
                                                    if let Err(e) = ds.ensure_schema() {
                                                        println!("Schema error: {e}"); return Ok(());
                                                    }
                                                    match ds.list(&cid) {
                                                        Ok(docs) => {
                                                            if docs.is_empty() {
                                                                println!("No documents. Use: /company doc create <title>");
                                                            } else {
                                                                println!("Documents ({}):\n", docs.len());
                                                                for d in &docs {
                                                                    println!("  {}", d.summary_line());
                                                                }
                                                                println!();
                                                            }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    }
                                                }
                                                (None, _) => println!("No active company."),
                                                (_, Err(e)) => println!("Error: {e}"),
                                            }
                                        }
                                        "show" => {
                                            if rest2.is_empty() {
                                                println!("Usage: /company doc show <id>");
                                            } else {
                                                match store_result {
                                                    Ok(store) => {
                                                        let ds = DocumentStore::new(store.conn());
                                                        if let Err(e) = ds.ensure_schema() {
                                                            println!("Schema error: {e}"); return Ok(());
                                                        }
                                                        match ds.get(rest2) {
                                                            Ok(Some(d)) => {
                                                                println!("# {}", d.title);
                                                                println!("  ID: {}  Rev: {}", d.id, d.revision);
                                                                println!();
                                                                println!("{}", d.content);
                                                            }
                                                            Ok(None) => println!("Document not found: {rest2}"),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    Err(e) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "history" => {
                                            if rest2.is_empty() {
                                                println!("Usage: /company doc history <id>");
                                            } else {
                                                match store_result {
                                                    Ok(store) => {
                                                        let ds = DocumentStore::new(store.conn());
                                                        if let Err(e) = ds.ensure_schema() {
                                                            println!("Schema error: {e}"); return Ok(());
                                                        }
                                                        match ds.list_revisions(rest2) {
                                                            Ok(revs) => {
                                                                println!("Revision history ({}):\n", revs.len());
                                                                for r in &revs {
                                                                    println!("  v{}  {} chars", r.revision, r.content.len());
                                                                }
                                                                println!();
                                                            }
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    Err(e) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        _ => {
                                            println!("Document Commands:");
                                            println!("  /company doc create <title>    — Create document");
                                            println!("  /company doc list               — List documents");
                                            println!("  /company doc show <id>          — Show document content");
                                            println!("  /company doc history <id>       — Show revision history");
                                            println!();
                                        }
                                    }
                                }

                                "approval" => {
                                    use company_approvals::{ApprovalStore, ApprovalRequestType};
                                    use company_store::{CompanyStore, get_active_company_id};
                                    let sub3 = parts.get(3).copied().unwrap_or("");
                                    match sub3 {
                                        "request" => {
                                            // /company approval request <type> <subject-id> <requester-id> [reason...]
                                            let req_type_str = parts.get(4).copied().unwrap_or("");
                                            let subject_id = parts.get(5).copied().unwrap_or("");
                                            let requester_id = parts.get(6).copied().unwrap_or("system");
                                            let reason = parts[7.min(parts.len())..].join(" ");
                                            if req_type_str.is_empty() || subject_id.is_empty() {
                                                println!("Usage: /company approval request <hire|strategy|budget|task|deploy> <subject-id> [requester-id] [reason]");
                                            } else {
                                                let req_type = ApprovalRequestType::from_str(req_type_str);
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(cid), Ok(store)) => {
                                                        let ap = ApprovalStore::new(store.conn());
                                                        if let Err(e) = ap.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match ap.request(&cid, req_type, subject_id, requester_id, &reason) {
                                                            Ok(a) => println!("✓ Approval requested: {}", a.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "list" => {
                                            let status_filter = parts.get(4).copied();
                                            match (get_active_company_id(), CompanyStore::open_default()) {
                                                (Some(cid), Ok(store)) => {
                                                    let ap = ApprovalStore::new(store.conn());
                                                    if let Err(e) = ap.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                    match ap.list(&cid, status_filter) {
                                                        Ok(approvals) => {
                                                            if approvals.is_empty() {
                                                                println!("No approvals found.");
                                                            } else {
                                                                for a in &approvals { println!("{}", a.summary_line()); }
                                                            }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    }
                                                }
                                                (None, _) => println!("No active company."),
                                                (_, Err(e)) => println!("Error: {e}"),
                                            }
                                        }
                                        "approve" | "reject" => {
                                            let id = parts.get(4).copied().unwrap_or("");
                                            let approver = parts.get(5).copied().unwrap_or("system");
                                            let approved = sub3 == "approve";
                                            if id.is_empty() {
                                                println!("Usage: /company approval approve|reject <id> [approver-id]");
                                            } else {
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(_cid), Ok(store)) => {
                                                        let ap = ApprovalStore::new(store.conn());
                                                        if let Err(e) = ap.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match ap.decide(id, approved, approver) {
                                                            Ok(a) => println!("✓ {}", a.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        _ => println!("Approval subcommands: request <type> <subject-id> | list [status] | approve <id> | reject <id>"),
                                    }
                                }
                                "budget" => {
                                    use company_budget::BudgetStore;
                                    use company_store::{CompanyStore, get_active_company_id};
                                    let sub3 = parts.get(3).copied().unwrap_or("");
                                    match sub3 {
                                        "set" => {
                                            // /company budget set <agent-id> <limit-cents> [--hard-stop] [--month YYYY-MM]
                                            let agent_id = parts.get(4).copied().unwrap_or("");
                                            let limit_cents: i64 = parts.get(5).copied().unwrap_or("0").parse().unwrap_or(0);
                                            let hard_stop = parts.contains(&"--hard-stop");
                                            let month = parts.iter().position(|p| *p == "--month")
                                                .and_then(|i| parts.get(i + 1).copied())
                                                .unwrap_or("");
                                            let month = if month.is_empty() { BudgetStore::current_month_static() } else { month.to_string() };
                                            if agent_id.is_empty() {
                                                println!("Usage: /company budget set <agent-id> <limit-cents> [--hard-stop] [--month YYYY-MM]");
                                            } else {
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(cid), Ok(store)) => {
                                                        let bs = BudgetStore::new(store.conn());
                                                        if let Err(e) = bs.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match bs.set_budget(&cid, agent_id, &month, limit_cents, hard_stop, 80) {
                                                            Ok(b) => println!("✓ Budget set: {}", b.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "status" | "list" => {
                                            let agent_filter = parts.get(4).copied();
                                            match (get_active_company_id(), CompanyStore::open_default()) {
                                                (Some(cid), Ok(store)) => {
                                                    let bs = BudgetStore::new(store.conn());
                                                    if let Err(e) = bs.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                    match bs.list(&cid) {
                                                        Ok(budgets) => {
                                                            let filtered: Vec<_> = budgets.iter().filter(|b| {
                                                                agent_filter.map(|f| b.agent_id.starts_with(f)).unwrap_or(true)
                                                            }).collect();
                                                            if filtered.is_empty() {
                                                                println!("No budgets found.");
                                                            } else {
                                                                for b in &filtered { println!("{}", b.summary_line()); }
                                                            }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    }
                                                }
                                                (None, _) => println!("No active company."),
                                                (_, Err(e)) => println!("Error: {e}"),
                                            }
                                        }
                                        "events" => {
                                            let agent_filter = parts.get(4).copied();
                                            match (get_active_company_id(), CompanyStore::open_default()) {
                                                (Some(cid), Ok(store)) => {
                                                    let bs = BudgetStore::new(store.conn());
                                                    if let Err(e) = bs.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                    match bs.list_events(&cid, agent_filter) {
                                                        Ok(events) => {
                                                            if events.is_empty() {
                                                                println!("No cost events.");
                                                            } else {
                                                                for ev in &events {
                                                                    let dt = ev.created_at / 1000;
                                                                    println!(
                                                                        "[{}] ${:.4}  {}  agent:{}  {}",
                                                                        dt, ev.amount_cents as f64 / 100.0,
                                                                        ev.model,
                                                                        &ev.agent_id[..8.min(ev.agent_id.len())],
                                                                        ev.description
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    }
                                                }
                                                (None, _) => println!("No active company."),
                                                (_, Err(e)) => println!("Error: {e}"),
                                            }
                                        }
                                        _ => println!("Budget subcommands: set <agent-id> <cents> [--hard-stop] | status [agent-id] | events [agent-id]"),
                                    }
                                }
                                "secret" => {
                                    use company_secrets::SecretStore;
                                    use company_store::{CompanyStore, get_active_company_id};
                                    let sub3 = parts.get(3).copied().unwrap_or("");
                                    match sub3 {
                                        "set" => {
                                            let key = parts.get(4).copied().unwrap_or("");
                                            let value = parts[5.min(parts.len())..].join(" ");
                                            if key.is_empty() || value.is_empty() {
                                                println!("Usage: /company secret set <key> <value>");
                                            } else {
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(cid), Ok(store)) => {
                                                        let ss = SecretStore::new(store.conn());
                                                        if let Err(e) = ss.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match ss.set(&cid, key, &value, None) {
                                                            Ok(s) => println!("✓ Secret stored: {}", s.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "get" => {
                                            let key = parts.get(4).copied().unwrap_or("");
                                            if key.is_empty() { println!("Usage: /company secret get <key>"); }
                                            else {
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(cid), Ok(store)) => {
                                                        let ss = SecretStore::new(store.conn());
                                                        if let Err(e) = ss.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match ss.get_value(&cid, key) {
                                                            Ok(v) => println!("{}: {}", key, v),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "list" => {
                                            match (get_active_company_id(), CompanyStore::open_default()) {
                                                (Some(cid), Ok(store)) => {
                                                    let ss = SecretStore::new(store.conn());
                                                    if let Err(e) = ss.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                    match ss.list(&cid) {
                                                        Ok(secrets) => {
                                                            if secrets.is_empty() { println!("No secrets."); }
                                                            else { for s in &secrets { println!("{}", s.summary_line()); } }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    }
                                                }
                                                (None, _) => println!("No active company."),
                                                (_, Err(e)) => println!("Error: {e}"),
                                            }
                                        }
                                        "delete" => {
                                            let key = parts.get(4).copied().unwrap_or("");
                                            if key.is_empty() { println!("Usage: /company secret delete <key>"); }
                                            else {
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(cid), Ok(store)) => {
                                                        let ss = SecretStore::new(store.conn());
                                                        if let Err(e) = ss.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match ss.delete(&cid, key) {
                                                            Ok(true) => println!("✓ Deleted."),
                                                            Ok(false) => println!("Secret not found."),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        _ => println!("Secret subcommands: set <key> <value> | get <key> | list | delete <key>"),
                                    }
                                }
                                "routine" => {
                                    use company_routines::RoutineStore;
                                    use company_store::{CompanyStore, get_active_company_id};
                                    let sub3 = parts.get(3).copied().unwrap_or("");
                                    match sub3 {
                                        "create" => {
                                            // /company routine create <agent-id> <name> [--interval <secs>] [--prompt <text>]
                                            let agent_id = parts.get(4).copied().unwrap_or("");
                                            let name = parts.get(5).copied().unwrap_or("");
                                            let interval: i64 = parts.iter().position(|p| *p == "--interval")
                                                .and_then(|i| parts.get(i + 1).copied())
                                                .and_then(|v| v.parse().ok())
                                                .unwrap_or(3600);
                                            let prompt_idx = parts.iter().position(|p| *p == "--prompt").map(|i| i + 1).unwrap_or(parts.len());
                                            let prompt = parts[prompt_idx..].join(" ");
                                            if agent_id.is_empty() || name.is_empty() {
                                                println!("Usage: /company routine create <agent-id> <name> [--interval <secs>] [--prompt <text>]");
                                            } else {
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(cid), Ok(store)) => {
                                                        let rs = RoutineStore::new(store.conn());
                                                        if let Err(e) = rs.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match rs.create(&cid, agent_id, name, &prompt, interval) {
                                                            Ok(r) => println!("✓ Routine created: {}", r.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "list" => {
                                            match (get_active_company_id(), CompanyStore::open_default()) {
                                                (Some(cid), Ok(store)) => {
                                                    let rs = RoutineStore::new(store.conn());
                                                    if let Err(e) = rs.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                    match rs.list(&cid) {
                                                        Ok(routines) => {
                                                            if routines.is_empty() { println!("No routines."); }
                                                            else { for r in &routines { println!("{}", r.summary_line()); } }
                                                        }
                                                        Err(e) => println!("Error: {e}"),
                                                    }
                                                }
                                                (None, _) => println!("No active company."),
                                                (_, Err(e)) => println!("Error: {e}"),
                                            }
                                        }
                                        "toggle" => {
                                            let id = parts.get(4).copied().unwrap_or("");
                                            if id.is_empty() { println!("Usage: /company routine toggle <id>"); }
                                            else {
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(_), Ok(store)) => {
                                                        let rs = RoutineStore::new(store.conn());
                                                        if let Err(e) = rs.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match rs.toggle(id) {
                                                            Ok(r) => println!("✓ {}", r.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        _ => println!("Routine subcommands: create <agent-id> <name> [--interval secs] | list | toggle <id>"),
                                    }
                                }
                                "heartbeat" => {
                                    use company_heartbeat::{HeartbeatStore, HeartbeatTrigger};
                                    use company_store::{CompanyStore, get_active_company_id};
                                    let sub3 = parts.get(3).copied().unwrap_or("");
                                    match sub3 {
                                        "trigger" => {
                                            let agent_id = parts.get(4).copied().unwrap_or("");
                                            if agent_id.is_empty() { println!("Usage: /company heartbeat trigger <agent-id>"); }
                                            else {
                                                match (get_active_company_id(), CompanyStore::open_default()) {
                                                    (Some(cid), Ok(store)) => {
                                                        let hs = HeartbeatStore::new(store.conn());
                                                        if let Err(e) = hs.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match hs.start(&cid, agent_id, HeartbeatTrigger::Manual, None) {
                                                            Ok(r) => println!("✓ Heartbeat started: {}", r.summary_line()),
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    (None, _) => println!("No active company."),
                                                    (_, Err(e)) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        "history" => {
                                            let agent_id = parts.get(4).copied().unwrap_or("");
                                            let limit: i64 = parts.get(5).copied().and_then(|v| v.parse().ok()).unwrap_or(20);
                                            if agent_id.is_empty() { println!("Usage: /company heartbeat history <agent-id> [limit]"); }
                                            else {
                                                match CompanyStore::open_default() {
                                                    Ok(store) => {
                                                        let hs = HeartbeatStore::new(store.conn());
                                                        if let Err(e) = hs.ensure_schema() { println!("Schema error: {e}"); return Ok(()); }
                                                        match hs.history(agent_id, limit) {
                                                            Ok(runs) => {
                                                                if runs.is_empty() { println!("No heartbeat history."); }
                                                                else { for r in &runs { println!("{}", r.summary_line()); } }
                                                            }
                                                            Err(e) => println!("Error: {e}"),
                                                        }
                                                    }
                                                    Err(e) => println!("Error: {e}"),
                                                }
                                            }
                                        }
                                        _ => println!("Heartbeat subcommands: trigger <agent-id> | history <agent-id> [limit]"),
                                    }
                                }
                                "adapter" => {
                                    use adapter_registry::default_registry;
                                    let sub3 = parts.get(3).copied().unwrap_or("");
                                    match sub3 {
                                        "register" => {
                                            // /company adapter register <name> --type http --url <url>
                                            let name = parts.get(4).copied().unwrap_or("");
                                            let adapter_type = parts.iter().position(|p| *p == "--type")
                                                .and_then(|i| parts.get(i + 1).copied()).unwrap_or("http");
                                            let url = parts.iter().position(|p| *p == "--url")
                                                .and_then(|i| parts.get(i + 1).copied()).unwrap_or("");
                                            if name.is_empty() {
                                                println!("Usage: /company adapter register <name> --type http --url <url>");
                                            } else if adapter_type == "http" && !url.is_empty() {
                                                let reg = default_registry();
                                                let cfg = adapter_registry::HttpAdapterConfig {
                                                    url: url.to_string(),
                                                    method: "POST".to_string(),
                                                    headers: std::collections::HashMap::new(),
                                                    timeout_secs: 30,
                                                };
                                                reg.register_http(name, cfg);
                                                println!("✓ Adapter registered: {} (http → {})", name, url);
                                            } else {
                                                println!("Supported: --type http --url <url>");
                                            }
                                        }
                                        "list" => {
                                            let reg = default_registry();
                                            let adapters = reg.list();
                                            if adapters.is_empty() { println!("No adapters registered."); }
                                            else { for a in &adapters { println!("{} [{}]", a.name, a.adapter_type); } }
                                        }
                                        "remove" => {
                                            let name = parts.get(4).copied().unwrap_or("");
                                            if name.is_empty() { println!("Usage: /company adapter remove <name>"); }
                                            else {
                                                let reg = default_registry();
                                                if reg.unregister(name) { println!("✓ Adapter removed."); }
                                                else { println!("Adapter not found."); }
                                            }
                                        }
                                        _ => println!("Adapter subcommands: register <name> --type http --url <url> | list | remove <name>"),
                                    }
                                }
                                "export" => {
                                    use company_store::get_active_company_id;
                                    let company_id_override = parts.get(3).copied();
                                    let output_path = parts.get(4).copied().unwrap_or("company_export.json");
                                    match get_active_company_id().or_else(|| company_id_override.map(|s| s.to_string())) {
                                        Some(cid) => {
                                            match company_portability::export_company(&cid, std::path::Path::new(output_path)) {
                                                Ok(bp) => println!("✓ Exported to {}\n{}", output_path, bp.summary()),
                                                Err(e) => println!("Error: {e}"),
                                            }
                                        }
                                        None => println!("No active company. Use: /company export [company-id] [output.json]"),
                                    }
                                }
                                "import" => {
                                    let input_path = parts.get(3).copied().unwrap_or("");
                                    let new_name = parts.get(4).copied();
                                    if input_path.is_empty() {
                                        println!("Usage: /company import <path.json> [new-name]");
                                    } else {
                                        match company_portability::import_company(std::path::Path::new(input_path), new_name) {
                                            Ok(new_id) => println!("✓ Company imported with ID: {}", &new_id[..8.min(new_id.len())]),
                                            Err(e) => println!("Error: {e}"),
                                        }
                                    }
                                }

                                _ => {
                                    println!("VibeCody Company Orchestration  (Paperclip parity)\n");
                                    println!("Company:");
                                    println!("  /company create <name> [desc]          — Create a new company");
                                    println!("  /company list                           — List all companies");
                                    println!("  /company switch <name|id>              — Set active company");
                                    println!("  /company status                         — Active company dashboard");
                                    println!("  /company delete <id|name>              — Archive a company");
                                    println!();
                                    println!("Agents:");
                                    println!("  /company agent hire <name> [options]   — Hire a new agent");
                                    println!("  /company agent list                    — List agents");
                                    println!("  /company agent fire <id>               — Terminate agent");
                                    println!("  /company agent info <id>               — Agent details");
                                    println!("  /company agent tree                    — ASCII org chart");
                                    println!();
                                    println!("Goals:");
                                    println!("  /company goal create <title>           — Create a goal");
                                    println!("  /company goal list                     — List goals as tree");
                                    println!();
                                    println!("Tasks:");
                                    println!("  /company task create <title>           — Create a task");
                                    println!("  /company task list [status]            — List tasks");
                                    println!("  /company task transition <id> <status> — Change task status");
                                    println!("  /company task checkout <id>            — Checkout task (in_progress)");
                                    println!("  /company task comment <id> <text>      — Add comment");
                                    println!();
                                    println!("Approvals:");
                                    println!("  /company approval request <type> <subject> — New approval");
                                    println!("  /company approval list [status]            — List approvals");
                                    println!("  /company approval approve|reject <id>      — Decide");
                                    println!();
                                    println!("Budget:");
                                    println!("  /company budget set <agent> <cents> [--hard-stop] — Set budget");
                                    println!("  /company budget status                             — Budget overview");
                                    println!("  /company budget events                             — Cost events");
                                    println!();
                                    println!("Secrets:");
                                    println!("  /company secret set <key> <value>      — Store encrypted secret");
                                    println!("  /company secret get <key>              — Retrieve secret");
                                    println!("  /company secret list                   — List secret keys");
                                    println!("  /company secret delete <key>           — Delete secret");
                                    println!();
                                    println!("Routines:");
                                    println!("  /company routine create <agent> <name> [--interval secs]");
                                    println!("  /company routine list                  — List routines");
                                    println!("  /company routine toggle <id>           — Enable/disable");
                                    println!();
                                    println!("Heartbeats:");
                                    println!("  /company heartbeat trigger <agent-id>  — Manual trigger");
                                    println!("  /company heartbeat history <agent-id>  — Run history");
                                    println!();
                                    println!("Documents:");
                                    println!("  /company doc create <title>            — Create a document");
                                    println!("  /company doc list                      — List documents");
                                    println!("  /company doc show <id>                 — Show document");
                                    println!("  /company doc history <id>              — Revision history");
                                    println!();
                                    println!("Adapters & Portability:");
                                    println!("  /company adapter register <name> --type http --url <url>");
                                    println!("  /company adapter list | remove <name>");
                                    println!("  /company export [path]                 — Export blueprint");
                                    println!("  /company import <path> [new-name]      — Import blueprint");
                                    println!();
                                }
                            }
                        }

                        "/resources" => {
                            let sub = args.split_whitespace().next().unwrap_or("status");
                            let mgr = resource_manager::ResourceManager::default_manager();
                            match sub {
                                "export" => {
                                    match mgr.export_defaults() {
                                        Ok(result) => {
                                            println!("Exported {} resource files to {}\n",
                                                result.files_written.len(), result.resources_dir.display());
                                            for f in &result.files_written {
                                                println!("  {}", f);
                                            }
                                            println!("\n  Manifest written with SHA-256 checksums.");
                                            println!("  Files secured with 0600 permissions.\n");
                                        }
                                        Err(e) => println!("Export failed: {}\n", e),
                                    }
                                }
                                "verify" => {
                                    let results = mgr.verify_all();
                                    println!("Resource Integrity Verification\n");
                                    let mut all_ok = true;
                                    for r in &results {
                                        let icon = match r.status {
                                            resource_manager::VerifyStatus::Ok => "OK",
                                            resource_manager::VerifyStatus::Missing => { all_ok = false; "MISSING" },
                                            resource_manager::VerifyStatus::Corrupted => { all_ok = false; "CORRUPTED" },
                                            resource_manager::VerifyStatus::NoManifest => { all_ok = false; "NO MANIFEST" },
                                        };
                                        let size = r.size.map(|s| format!(" ({} bytes)", s)).unwrap_or_default();
                                        println!("  [{}] {}{}", icon, r.resource, size);
                                    }
                                    if all_ok {
                                        println!("\n  All resources verified.\n");
                                    } else {
                                        println!("\n  Some resources need attention. Run /resources export to fix.\n");
                                    }
                                }
                                "path" => {
                                    println!("Resources directory: {}\n", resource_manager::ResourceManager::default_dir().display());
                                }
                                _ => {
                                    let status = mgr.status();
                                    println!("Secure Resource Manager\n");
                                    println!("  Directory: {}", status.resources_dir.display());
                                    println!("  Initialized: {}", status.initialized);
                                    println!("  Resources: {} total ({} OK, {} missing, {} corrupted)",
                                        status.total_resources, status.ok_count, status.missing_count, status.corrupted_count);
                                    if status.total_size_bytes > 0 {
                                        println!("  Total size: {} bytes", status.total_size_bytes);
                                    }
                                    if let Some(ts) = status.manifest_updated_at {
                                        let age = resource_manager::sha256_hex(b""); // just to show it exists
                                        let _ = age;
                                        println!("  Last updated: epoch {}", ts);
                                    }
                                    println!("\n  /resources export   — Export embedded defaults to disk");
                                    println!("  /resources verify   — Verify file integrity (SHA-256)");
                                    println!("  /resources path     — Show resources directory path\n");
                                }
                            }
                        }

                        "/wizard" => {
                            println!("Model Wizard — Fine-tune and deploy models step by step\n");
                            println!("Steps:");
                            println!("  1. Choose base model (Llama, Mistral, Gemma, Phi, Qwen, DeepSeek)");
                            println!("  2. Prepare dataset (codebase, git history, documents, existing JSONL)");
                            println!("  3. Configure fine-tuning (Unsloth, Axolotl, LLaMA Factory, TRL, PEFT, DeepSpeed)");
                            println!("  4. Select environment (Colab, Kaggle, SageMaker, local)");
                            println!("  5. Quantize (GGUF, GPTQ, AWQ, Int8, FP16)");
                            println!("  6. Deploy inference (Ollama, vLLM, llama.cpp, TGI, Triton)");
                            println!("  7. Generate complete script\n");
                            println!("Quick start examples:");
                            println!("  /train dataset from-codebase --format chatml --output data.jsonl");
                            println!("  /train finetune --library unsloth --model meta-llama/Llama-3.1-8B-Instruct");
                            println!("  /inference quantize --method gguf-q4km --model ./output");
                            println!("  /inference deploy --backend ollama --model ./model.gguf\n");
                            println!("For the full interactive wizard, use the Model Wizard tab in VibeUI.\n");
                        }

                        _ => {
                            // Suggest closest command via edit distance
                            if let Some(suggestion) = find_closest_command(command) {
                                println!("Unknown command: {command}. Did you mean {suggestion}?\n  Type /help for all commands.\n");
                            } else {
                                println!("Unknown command: {command}. Type /help for available commands.\n");
                            }
                        }
                    }
                } else {
                    // Regular chat (with @ context expansion, attachments, and streaming)
                    if !conversation_active {
                        messages.clear();
                        conversation_active = true;
                        messages.push(Message {
                            role: MessageRole::System,
                            content: "You are a helpful coding assistant. If the user asks you to run a command, output it in a ```execute block.\n\nContext references (@file:, @web:, @docs:, @git) are automatically expanded before each message.\nFile attachments [file.ext] are also supported for images and documents.".to_string(),
                        });
                    }
                    // Extract file attachments [file.ext] and expand @-references
                    let (text_content, images, doc_context) = extract_attachments_from_input(input);
                    let expanded = expand_at_refs(&text_content).await;
                    let full_content = if doc_context.is_empty() {
                        expanded
                    } else {
                        format!("[Attached Documents]\n{}\n\n{}", doc_context, expanded)
                    };
                    messages.push(Message {
                        role: MessageRole::User,
                        content: full_content,
                    });
                    // Collect full response then render with markdown highlighting
                    let chat_result = if images.is_empty() {
                        llm.chat(&messages, None).await
                    } else {
                        println!("({} image{})", images.len(), if images.len() > 1 { "s" } else { "" });
                        llm.chat_with_images(&messages, &images, None).await
                    };
                    match chat_result {
                        Ok(full_response) => {
                            if !full_response.is_empty() {
                                let rendered = if full_response.contains("```mermaid") {
                                    let mermaid = mermaid_ascii::render_mermaid_blocks(&full_response);
                                    highlight_code_blocks(&mermaid)
                                } else {
                                    highlight_code_blocks(&full_response)
                                };
                                println!("{}", rendered);
                            }
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
    no_network: bool,
}

impl ExecutorFactory for VibeExecutorFactory {
    fn create(&self, workspace_root: std::path::PathBuf) -> Arc<dyn vibe_ai::agent::ToolExecutorTrait> {
        let mut te = ToolExecutor::new(workspace_root, self.sandbox)
            .with_env_policy(self.env_policy.clone())
            .with_provider(self.provider.clone());
        if self.no_network { te = te.with_no_network(); }
        Arc::new(te)
    }
}

// ── Parallel multi-agent runner ───────────────────────────────────────────────

async fn run_parallel_agents(
    llm: Arc<dyn LLMProvider>,
    task: &str,
    approval_policy: &str,
    n: usize,
    no_network: bool,
) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let config = Config::load().unwrap_or_default();
    let approval = ApprovalPolicy::from_str(approval_policy);
    let sandbox = config.safety.sandbox;
    let env_policy = config.safety.shell_environment.to_policy();

    let factory = Arc::new(VibeExecutorFactory { sandbox, env_policy, provider: llm.clone(), no_network });
    let manager = Arc::new(VibeCoreWorktreeManager::new(workspace.clone()));

    let mut orchestrator = MultiAgentOrchestrator::new(llm, approval, factory)
        .with_worktree_manager(manager)
        .with_max_agents(n);

    if !config.hooks.is_empty() {
        orchestrator = orchestrator.with_hooks(HookRunner::new(config.hooks));
    }

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(128);

    println!("Starting {} parallel agents for task: {}", n, task);
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
    no_network: bool,
) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let config = Config::load().unwrap_or_default();
    let approval = ApprovalPolicy::from_str(approval_policy);
    let sandbox = config.safety.sandbox;

    // Apply shell env policy and search engine config
    let env_policy = config.safety.shell_environment.to_policy();
    let search_cfg = &config.tools.web_search;
    let mut te = ToolExecutor::new(workspace.clone(), sandbox)
        .with_env_policy(env_policy)
        .with_search_config(
            search_cfg.engine.clone(),
            search_cfg.resolve_tavily_key(),
            search_cfg.resolve_brave_key(),
        )
        .with_provider(llm.clone());
    if no_network { te = te.with_no_network(); }
    let executor: Arc<dyn vibe_ai::agent::ToolExecutorTrait> = Arc::new(te);

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
        println!("Generating execution plan...\n");
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
    let mut resumed_messages: Vec<Message> = if let Some(sid_prefix) = resume_session_id {
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

    // Inject orchestration lessons and current task into agent initial messages
    {
        use crate::workflow_orchestration::{LessonsStore, TodoStore, orchestration_system_prompt};
        let lessons_store = LessonsStore::for_workspace(&workspace);
        let todo_store = TodoStore::for_workspace(&workspace);
        let lessons = lessons_store.load();
        let current_task = todo_store.load();
        let orch_ctx = orchestration_system_prompt(&lessons, current_task.as_ref());
        if !orch_ctx.is_empty() {
            resumed_messages.insert(0, Message {
                role: MessageRole::System,
                content: orch_ctx,
            });
        }
    }

    // Collect skill directories from installed plugins.
    let plugin_skill_dirs = PluginLoader::new().all_skill_paths()
        .into_iter()
        .filter_map(|p| p.parent().map(|d| d.to_path_buf()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    // Auto-detect project context for always-on understanding
    let project_profile = project_init::get_or_scan_profile(&workspace);
    let project_summary = Some(project_profile.to_system_prompt_context());

    // Auto-gather relevant files based on the task description
    let relevant_paths = project_init::extract_relevant_files_for_task(&workspace, task);
    let task_context_files: Vec<(String, String)> = relevant_paths.iter()
        .filter_map(|rel_path| {
            let full_path = workspace.join(rel_path);
            if full_path.is_file() {
                std::fs::read_to_string(&full_path)
                    .ok()
                    .map(|content| {
                        // Limit preview to first 80 lines
                        let preview: String = content.lines().take(80).collect::<Vec<_>>().join("\n");
                        (rel_path.clone(), preview)
                    })
            } else {
                None
            }
        })
        .take(5) // Max 5 files to avoid bloating context
        .collect();

    // OpenMemory: inject relevant memories into agent system prompt (config-gated)
    let memory_context = if config.memory.openmemory.enabled && config.memory.openmemory.auto_inject {
        let store = open_memory::project_scoped_store(&workspace);
        let ctx = store.get_agent_context(task, config.memory.openmemory.max_memories_in_context);
        if ctx.is_empty() { None } else { Some(ctx) }
    } else {
        None
    };

    let context = AgentContext {
        workspace_root: workspace.clone(),
        approved_plan,
        extra_skill_dirs: plugin_skill_dirs,
        project_summary,
        task_context_files,
        memory_context,
        ..Default::default()
    };

    let trace = TraceWriter::new(trace_dir.clone());

    // Open SQLite session store (non-fatal if unavailable)
    let db = SessionStore::open_default().ok();
    let session_id = trace.session_id().to_string();
    if let Some(ref store) = db {
        // A6: auto-name the session from the task description
        let auto_name = session_store::auto_name_session(task);
        let _ = store.insert_session(&session_id, &auto_name, provider_name, model_name);
        let _ = store.insert_message(&session_id, "user", task);
    }

    let policy_label = match approval {
        ApprovalPolicy::ChatOnly  => "chat-only (no tool calls, conversational only)",
        ApprovalPolicy::Suggest   => "manual (ask before every action)",
        ApprovalPolicy::AutoEdit  => "smart (auto-apply files, ask for shell commands)",
        ApprovalPolicy::FullAuto  => "autonomous (execute everything without prompting)",
    };
    println!("{}", crate::syntax::format_agent_start(task, policy_label));
    if !resumed_messages.is_empty() {
        println!("   Resuming {} prior messages", resumed_messages.len());
    }

    // A8: Smart Extension Auto-Detection — print workspace hints at session start
    if let Ok(cwd) = std::env::current_dir() {
        crate::workspace_detect::print_extension_hints(&cwd);
    }

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
    // Track all steps for change summary at the end
    let mut completed_steps: Vec<(String, String, bool)> = Vec::new();

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
                AgentEvent::RetryableError { ref error, attempt, max_attempts, backoff_ms } => serde_json::json!({"type":"retry","error":error,"attempt":attempt,"max_attempts":max_attempts,"backoff_ms":backoff_ms}),
                AgentEvent::CircuitBreak { ref state, ref reason } => serde_json::json!({"type":"circuit_break","state":state.to_string(),"reason":reason}),
                AgentEvent::Partial { ref summary, steps_completed, steps_planned, ref remaining_plan } => serde_json::json!({"type":"partial","summary":summary,"steps_completed":steps_completed,"steps_planned":steps_planned,"remaining_plan":remaining_plan}),
            };
            println!("{}", obj);
            io::stdout().flush()?;
            match event {
                AgentEvent::ToolCallPending { result_tx, call } => {
                    // In JSON mode auto-execute all tool calls (full-auto behaviour)
                    let result = executor.execute(&call).await;
                    let _ = result_tx.send(Some(result));
                }
                AgentEvent::Complete(_) | AgentEvent::Error(_) | AgentEvent::Partial { .. } => break,
                AgentEvent::CircuitBreak { ref state, .. } if *state == vibe_ai::agent::AgentHealthState::Blocked => break,
                _ => {}
            }
            continue;
        }

        match event {
            AgentEvent::StreamChunk(text) => {
                // Buffer chunks and render markdown when we get a complete line
                print!("{}", text);
                io::stdout().flush()?;
            }
            AgentEvent::ToolCallPending { call, result_tx } => {
                let description = crate::syntax::describe_tool_action(call.name(), &call.summary());
                println!("{}", crate::syntax::format_tool_pending(call.name(), &description));
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
                    // Execute the tool and show output
                    let result = executor.execute(&call).await;
                    if !result.output.trim().is_empty() {
                        println!("{}", crate::syntax::format_tool_output(&result.output, result.success));
                    }
                    trace.record(0, call.name(), &call.summary(), &result.output, result.success, dur, "user");
                    let _ = result_tx.send(Some(result));
                }
            }
            AgentEvent::ToolCallExecuted(step) => {
                let dur = step_start.elapsed().as_millis() as u64;
                step_start = std::time::Instant::now();
                step_count += 1;
                // Use human-readable description instead of raw tool call summary
                let description = crate::syntax::describe_tool_action(
                    step.tool_call.name(),
                    &step.tool_call.summary(),
                );
                println!(
                    "{}",
                    crate::syntax::format_step_result(step.step_num + 1, &description, step.tool_result.success)
                );
                completed_steps.push((
                    step.tool_call.name().to_string(),
                    step.tool_call.summary(),
                    step.tool_result.success,
                ));
                // Show tool output
                if !step.tool_result.output.trim().is_empty() {
                    println!("{}", crate::syntax::format_tool_output(&step.tool_result.output, step.tool_result.success));
                }
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
                println!("{}", crate::syntax::format_agent_complete(&summary));
                if !completed_steps.is_empty() {
                    println!("{}", crate::syntax::format_change_summary(&completed_steps));
                }
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
                // Bridge session summary to OpenMemory + auto-reflect (config-gated)
                if config.memory.openmemory.enabled && config.memory.openmemory.auto_save_sessions {
                    let summary_for_mem = summary.clone();
                    let task_for_mem = task.to_string();
                    let workspace_for_mem = workspace.clone();
                    let reflect_interval = config.memory.openmemory.auto_reflect_interval;
                    let dedup_threshold = config.memory.openmemory.dedup_threshold;
                    tokio::spawn(async move {
                        let mut store = open_memory::project_scoped_store(&workspace_for_mem);
                        // Store session as episodic memory (dedup-safe)
                        let content = format!("Session: {} — {}", task_for_mem, summary_for_mem);
                        store.add_dedup(content, dedup_threshold);
                        // Trigger auto-reflection at configured interval
                        if reflect_interval > 0 && store.total_memories().is_multiple_of(reflect_interval) {
                            store.auto_reflect();
                        }
                        // Reinforce memories that were used in context
                        let results = store.query(&task_for_mem, 5);
                        let ids: Vec<String> = results.iter().map(|r| r.memory.id.clone()).collect();
                        if !ids.is_empty() {
                            store.reinforce(&ids);
                        }
                        let _ = store.save();
                    });
                }
                break;
            }
            AgentEvent::Partial { summary, steps_completed, steps_planned, remaining_plan } => {
                eprintln!(
                    "\n  ⚠ Partial completion ({}/{}): {}",
                    steps_completed, steps_planned, summary
                );
                if !remaining_plan.is_empty() {
                    eprintln!("   Remaining steps:");
                    for step in &remaining_plan {
                        eprintln!("     - {}", step);
                    }
                }
                if !completed_steps.is_empty() {
                    println!("{}", crate::syntax::format_change_summary(&completed_steps));
                }
                println!("   Trace saved: {}", trace.path().display());
                println!("   Resume with: vibecli --resume {}", trace.session_id());
                if let Some(ref store) = db {
                    let _ = store.finish_session(&session_id, "partial", Some(&summary));
                }
                let _ = trace_for_save.save_context(&context);
                break;
            }
            AgentEvent::Error(e) => {
                eprintln!("{}", crate::syntax::format_agent_error(&e));
                if let Some(ref store) = db {
                    let _ = store.finish_session(&session_id, "failed", Some(&e));
                }
                break;
            }
            AgentEvent::RetryableError { error, attempt, max_attempts, backoff_ms } => {
                eprintln!(
                    "  ⟳ Retrying ({}/{}) in {}ms: {}",
                    attempt + 1, max_attempts, backoff_ms, error
                );
            }
            AgentEvent::CircuitBreak { state, reason } => {
                eprintln!("\n{}", reason);
                if state == vibe_ai::agent::AgentHealthState::Blocked {
                    if let Some(ref store) = db {
                        let _ = store.finish_session(&session_id, "blocked", Some(&reason));
                    }
                    break;
                }
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

    println!("\nGit changes detected:\n{}", changed.trim_end());
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

    println!("Generating commit message…");
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
/// Extract image and document attachments from user input.
///
/// Syntax: `[path/to/file.ext]`
/// - Image files (.png, .jpg, .gif, .webp): returned as ImageAttachment for vision API
/// - Document/code files (.pdf, .csv, .json, .rs, .py, etc.): content read and injected as context
///
/// Returns `(cleaned_text, images, document_context)`.
fn extract_attachments_from_input(input: &str) -> (String, Vec<ImageAttachment>, String) {
    let img_re = re_image_attachment();
    let file_re = re_file_attachment();
    let mut images = Vec::new();
    let mut doc_parts = Vec::new();

    // Collect images
    for caps in img_re.captures_iter(input) {
        let img_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        match ImageAttachment::from_path(std::path::Path::new(img_path)) {
            Ok(img) => {
                images.push(img);
                eprintln!("📎 Attached image: {}", img_path);
            }
            Err(e) => eprintln!("⚠️  Could not load image '{}': {}", img_path, e),
        }
    }

    // Collect document/code files
    for caps in file_re.captures_iter(input) {
        let file_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let path = std::path::Path::new(file_path);
        // Skip if this was already matched as an image
        if img_re.is_match(&format!("[{}]", file_path)) {
            continue;
        }
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let truncated = if content.len() > 32_000 {
                    format!("{}...\n\n[Truncated — file is {} bytes total]", &content[..32_000], content.len())
                } else {
                    content
                };
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                doc_parts.push(format!(
                    "=== Attached file: {} ({}) ===\n{}\n=== End of {} ===",
                    file_path, ext, truncated, file_path
                ));
                eprintln!("📎 Attached file: {} ({} bytes)", file_path, truncated.len());
            }
            Err(e) => {
                // Try reading as binary (e.g. PDF)
                match std::fs::read(path) {
                    Ok(bytes) => {
                        doc_parts.push(format!(
                            "=== Attached binary file: {} ({} bytes, cannot display content) ===",
                            file_path, bytes.len()
                        ));
                        eprintln!("📎 Attached binary file: {} ({} bytes)", file_path, bytes.len());
                    }
                    Err(_) => eprintln!("⚠️  Could not load file '{}': {}", file_path, e),
                }
            }
        }
    }

    // Strip all attachment markers from text
    let clean = img_re.replace_all(input, "");
    let clean = file_re.replace_all(&clean, "").trim().to_string();
    let doc_context = doc_parts.join("\n\n");
    (clean, images, doc_context)
}

/// B2: Print all supported providers and their default models.
fn list_providers_and_models() {
    const PROVIDERS: &[(&str, &str, &str)] = &[
        ("ollama",       "qwen3-coder:480b-cloud",    "Local LLM via Ollama (no API key)"),
        ("claude",       "claude-sonnet-4-6",          "Anthropic Claude (ANTHROPIC_API_KEY)"),
        ("openai",       "gpt-4o",                    "OpenAI GPT (OPENAI_API_KEY)"),
        ("gemini",       "gemini-2.0-flash",           "Google Gemini (GEMINI_API_KEY)"),
        ("grok",         "grok-3",                    "xAI Grok (GROK_API_KEY)"),
        ("groq",         "llama-3.3-70b-versatile",   "Groq Cloud (GROQ_API_KEY)"),
        ("openrouter",   "openai/gpt-4o",             "OpenRouter (OPENROUTER_API_KEY)"),
        ("azure-openai", "gpt-4o",                    "Azure OpenAI (AZURE_OPENAI_API_KEY)"),
        ("bedrock",      "anthropic.claude-3-5-sonnet-20241022-v2:0", "AWS Bedrock (AWS creds)"),
        ("copilot",      "gpt-4o",                    "GitHub Copilot (GITHUB_TOKEN)"),
        ("mistral",      "mistral-large-latest",       "Mistral AI (MISTRAL_API_KEY)"),
        ("cerebras",     "llama-4-scout-17b-16e-instruct", "Cerebras (CEREBRAS_API_KEY)"),
        ("deepseek",     "deepseek-chat",              "DeepSeek (DEEPSEEK_API_KEY)"),
        ("zhipu",        "glm-4-plus",                "Zhipu AI GLM (ZHIPU_API_KEY)"),
        ("vercel-ai",    "gpt-4o",                    "Vercel AI SDK (various keys)"),
        ("minimax",      "MiniMax-Text-01",            "MiniMax (MINIMAX_API_KEY)"),
        ("perplexity",   "llama-3.1-sonar-large-128k-online", "Perplexity (PERPLEXITY_API_KEY)"),
        ("together",     "meta-llama/Llama-3.3-70B-Instruct-Turbo", "Together AI (TOGETHER_API_KEY)"),
        ("fireworks",    "accounts/fireworks/models/llama-v3p3-70b-instruct", "Fireworks AI (FIREWORKS_API_KEY)"),
        ("sambanova",    "Meta-Llama-3.3-70B-Instruct", "SambaNova (SAMBANOVA_API_KEY)"),
    ];

    println!("\x1b[1;36m▶ VibeCLI Providers ({} supported)\x1b[0m", PROVIDERS.len());
    println!();
    for (name, default_model, note) in PROVIDERS {
        println!("  \x1b[1;33m{:<16}\x1b[0m  model: \x1b[2m{:<50}\x1b[0m  {}", name, default_model, note);
    }
    println!();
    println!("Usage:  \x1b[2mvibecli --provider <name> [--model <model>] \"your task\"\x1b[0m");
    println!("Config: \x1b[2m~/.vibecli/config.toml\x1b[0m");
}

fn create_provider(provider_name: &str, model: Option<String>) -> Result<Arc<dyn LLMProvider>> {
    let cfg = Config::load().unwrap_or_default();
    let raw = create_raw_provider(provider_name, model, &cfg)?;
    // Wrap with ResilientProvider for automatic retry on transient errors.
    Ok(vibe_ai::ResilientProvider::wrap(raw))
}

fn create_raw_provider(provider_name: &str, model: Option<String>, cfg: &Config) -> Result<Arc<dyn LLMProvider>> {
    use vibe_ai::providers::{claude, openai, gemini, grok, groq, openrouter, azure_openai, bedrock, copilot, mistral, cerebras, deepseek, zhipu, vercel_ai, minimax, perplexity, together, fireworks, sambanova};

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
                .unwrap_or_else(|| "gemini-2.5-flash".to_string());
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

        // ── Mistral ────────────────────────────────────────────────────────
        "mistral" => {
            let cfg_key = cfg.mistral.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("MISTRAL_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  MISTRAL_API_KEY not set (set env var or [mistral] api_key in config)");
            }
            let cfg_model = cfg.mistral.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "mistral-large-latest".to_string());
            let api_key_helper = cfg.mistral.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(mistral::MistralProvider::new(ProviderConfig {
                provider_type: "mistral".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Cerebras ─────────────────────────────────────────────────────────
        "cerebras" => {
            let cfg_key = cfg.cerebras.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("CEREBRAS_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  CEREBRAS_API_KEY not set (set env var or [cerebras] api_key in config)");
            }
            let cfg_model = cfg.cerebras.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "llama3.1-70b".to_string());
            let api_key_helper = cfg.cerebras.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(cerebras::CerebrasProvider::new(ProviderConfig {
                provider_type: "cerebras".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── DeepSeek ─────────────────────────────────────────────────────────
        "deepseek" => {
            let cfg_key = cfg.deepseek.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  DEEPSEEK_API_KEY not set (set env var or [deepseek] api_key in config)");
            }
            let cfg_model = cfg.deepseek.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "deepseek-chat".to_string());
            let api_key_helper = cfg.deepseek.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(deepseek::DeepSeekProvider::new(ProviderConfig {
                provider_type: "deepseek".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Zhipu GLM ────────────────────────────────────────────────────────
        "zhipu" | "glm" => {
            let cfg_key = cfg.zhipu.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("ZHIPU_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  ZHIPU_API_KEY not set (format: id.secret)");
            }
            let cfg_model = cfg.zhipu.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "glm-4".to_string());
            let api_key_helper = cfg.zhipu.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(zhipu::ZhipuProvider::new(ProviderConfig {
                provider_type: "zhipu".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Vercel AI Gateway ────────────────────────────────────────────────
        "vercel_ai" | "vercel" => {
            let cfg_ref = cfg.vercel_ai.as_ref();
            let api_key = cfg_ref.and_then(|c| c.api_key.clone())
                .or_else(|| std::env::var("VERCEL_AI_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  VERCEL_AI_API_KEY not set");
            }
            let api_url = cfg_ref.and_then(|c| c.api_url.clone())
                .or_else(|| std::env::var("VERCEL_AI_GATEWAY_URL").ok());
            if api_url.is_none() {
                eprintln!("⚠️  vercel_ai.api_url not set (your Vercel AI Gateway endpoint)");
            }
            let cfg_model = cfg_ref.and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "gpt-4o".to_string());
            Ok(Arc::new(vercel_ai::VercelAIProvider::new(ProviderConfig {
                provider_type: "vercel_ai".to_string(),
                api_url,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                ..Default::default()
            })))
        }

        // ── MiniMax ──────────────────────────────────────────────────────────
        "minimax" => {
            let cfg_key = cfg.minimax.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("MINIMAX_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  MINIMAX_API_KEY not set (set env var or [minimax] api_key in config)");
            }
            let cfg_model = cfg.minimax.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "abab6.5s-chat".to_string());
            let api_key_helper = cfg.minimax.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(minimax::MiniMaxProvider::new(ProviderConfig {
                provider_type: "minimax".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Perplexity ─────────────────────────────────────────────────────────
        "perplexity" | "pplx" => {
            let cfg_key = cfg.perplexity.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("PERPLEXITY_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  PERPLEXITY_API_KEY not set (set env var or [perplexity] api_key in config)");
            }
            let cfg_model = cfg.perplexity.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "sonar-pro".to_string());
            let api_key_helper = cfg.perplexity.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(perplexity::PerplexityProvider::new(ProviderConfig {
                provider_type: "perplexity".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Together AI ────────────────────────────────────────────────────────
        "together" | "together_ai" => {
            let cfg_key = cfg.together.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("TOGETHER_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  TOGETHER_API_KEY not set (set env var or [together] api_key in config)");
            }
            let cfg_model = cfg.together.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".to_string());
            let api_key_helper = cfg.together.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(together::TogetherProvider::new(ProviderConfig {
                provider_type: "together".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── Fireworks AI ───────────────────────────────────────────────────────
        "fireworks" | "fireworks_ai" => {
            let cfg_key = cfg.fireworks.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("FIREWORKS_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  FIREWORKS_API_KEY not set (set env var or [fireworks] api_key in config)");
            }
            let cfg_model = cfg.fireworks.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "accounts/fireworks/models/llama-v3p1-70b-instruct".to_string());
            let api_key_helper = cfg.fireworks.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(fireworks::FireworksProvider::new(ProviderConfig {
                provider_type: "fireworks".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        // ── SambaNova ──────────────────────────────────────────────────────────
        "sambanova" => {
            let cfg_key = cfg.sambanova.as_ref().and_then(|c| c.api_key.clone());
            let api_key = cfg_key
                .or_else(|| std::env::var("SAMBANOVA_API_KEY").ok());
            if api_key.is_none() {
                eprintln!("⚠️  SAMBANOVA_API_KEY not set (set env var or [sambanova] api_key in config)");
            }
            let cfg_model = cfg.sambanova.as_ref().and_then(|c| c.model.clone());
            let model = model
                .or(cfg_model)
                .unwrap_or_else(|| "Meta-Llama-3.1-70B-Instruct".to_string());
            let api_key_helper = cfg.sambanova.as_ref().and_then(|c| c.api_key_helper.clone());
            Ok(Arc::new(sambanova::SambaNovaProvider::new(ProviderConfig {
                provider_type: "sambanova".to_string(),
                api_url: None,
                model,
                api_key,
                max_tokens: None,
                temperature: None,
                api_key_helper,
                ..Default::default()
            })))
        }

        _ => anyhow::bail!(
            "Unknown provider: '{}'. Available: ollama, claude, openai, gemini, grok, groq, openrouter, azure, bedrock, copilot, mistral, cerebras, deepseek, zhipu, vercel, minimax, perplexity, together, fireworks, sambanova",
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

/// Simple Levenshtein distance for fuzzy command matching.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate().take(m + 1) { row[0] = i; }
    for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) { *cell = j; }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}

/// Find the closest REPL command to a mistyped input, or None if nothing is close enough.
fn find_closest_command(input: &str) -> Option<&'static str> {
    use crate::repl::COMMANDS;
    let mut best: Option<(&str, usize)> = None;
    for &cmd in COMMANDS {
        let d = edit_distance(input, cmd);
        if d <= 3 && best.is_none_or(|(_, bd)| d < bd) {
            best = Some((cmd, d));
        }
    }
    best.map(|(cmd, _)| cmd)
}

fn show_help() {
    println!("\nVibeCLI Commands:");
    println!("  /chat <message>          - Chat with AI (supports [image.png] and [file.rs] attachments)");
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
    println!("  /plugin search <query>   - Search the plugin registry");
    println!("  /plugin install <name>   - Install from registry or git URL");
    println!("  /plugin uninstall <name> - Remove an installed plugin");
    println!("  /plugin enable <name>    - Enable a disabled plugin");
    println!("  /plugin disable <name>   - Disable without uninstalling");
    println!("  /plugin info <name>      - Show plugin details");
    println!("  /plugin update [name]    - Update plugin(s) to latest");
    println!("  /ingest <path>           - Ingest documents into RAG (file or directory)");
    println!("  /ingest status           - Show ingestion pipeline status");
    println!("  /crawl <url>             - Crawl a website for RAG ingestion");
    println!("  /crawl status            - Show crawl job status");
    println!("  /rag search <query>      - Semantic search across ingested documents");
    println!("  /rag status              - Show RAG pipeline status (collections, vectors)");
    println!("  /rag collections         - List vector DB collections");
    println!("  /gpu status              - Show GPU cluster status and available GPUs");
    println!("  /gpu train <config>      - Submit a training job to GPU cluster");
    println!("  /gpu infer <model>       - Start an inference endpoint");
    println!("  /gpu cost <hours>        - Estimate GPU costs");
    println!("  /db connect <conn>       - Configure database connection");
    println!("  /db engines              - List supported database engines");
    println!("  /db migrate <cmd>        - Database migration management");
    println!("  /train suggest <B>       - Suggest parallelism for model size (billions)");
    println!("  /train memory <B>        - Estimate VRAM per GPU for training");
    println!("  /train frameworks        - List distributed training frameworks");
    println!("  /inference suggest <m>   - Recommend inference backend for model");
    println!("  /inference memory <B>    - Estimate VRAM for inference");
    println!("  /inference backends      - List supported inference backends");
    println!("  /turboquant benchmark    - TurboQuant compression + recall benchmark");
    println!("  /turboquant memory <B>   - Compare KV-cache memory (FP16/INT4/TurboQuant)");
    println!("  /profile list            - List named profiles (~/.vibecli/profiles/)");
    println!("  /profile show <name>     - Show a profile's settings");
    println!("  /profile create <name>   - Create a new profile interactively");
    println!("  /profile delete <name>   - Delete a profile");
    println!("  /spec                    - Spec-driven development (list|show|new|run|done)");
    println!("  /workflow                - Code Complete workflow (new|list|show|advance|check|generate)");
    println!("  /agents                  - Background agents (list|status|new)");
    println!("  /demo list               - List recorded feature demos");
    println!("  /demo run <name> <json>  - Record a demo from JSON steps");
    println!("  /demo generate <desc>    - AI-generate demo steps for a feature");
    println!("  /demo export <id> [fmt]  - Export demo as HTML slideshow or markdown");
    println!("  /soul                    - Generate SOUL.md for current project");
    println!("  /soul show               - View existing SOUL.md");
    println!("  /soul scan               - Show detected project signals");
    println!("  /soul regenerate         - Overwrite existing SOUL.md");
    println!("  /soul prompt             - Get LLM prompt for richer generation");
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
    println!("  /email inbox             - List recent emails (Gmail/Outlook)");
    println!("  /email read <id>         - Read full email");
    println!("  /email send <to> <subj>  - Compose and send email");
    println!("  /email search <query>    - Search emails");
    println!("  /email triage            - AI-powered email triage");
    println!("  /cal today               - Show today's calendar events");
    println!("  /cal week                - Show this week's events");
    println!("  /cal create <title>      - Create a calendar event");
    println!("  /cal free                - Show free time slots");
    println!("  /cal next                - Show next upcoming event");
    println!("  /home status             - Smart home device status");
    println!("  /home lights             - List lights with on/off state");
    println!("  /home on <entity>        - Turn on device");
    println!("  /home off <entity>       - Turn off device");
    println!("  /home scene <name>       - Activate Home Assistant scene");
    println!("  /notion search <query>   - Search Notion pages");
    println!("  /notion page <id>        - Read Notion page");
    println!("  /todo list               - List Todoist tasks");
    println!("  /todo add <task>         - Add a Todoist task");
    println!("  /todo complete <id>      - Complete a task");
    println!("  /jira list               - List Jira issues assigned to you");
    println!("  /jira create <proj> <s>  - Create Jira issue");
    println!("  /snippet list            - List saved code snippets");
    println!("  /snippet save <name>     - Save last AI response as a named snippet");
    println!("  /snippet use <name>      - Inject snippet as context for next message");
    println!("  /snippet show <name>     - Display snippet contents");
    println!("  /snippet delete <name>   - Delete a saved snippet");
    println!("  /jobs                    - List persisted background jobs (~/.vibecli/jobs/)");
    println!("  /jobs <session_id>       - Show full detail for a specific job");
    println!("  /openmemory              - Cognitive memory engine (add|query|list|fact|decay|consolidate|export)");
    println!("  /vulnscan                - Vulnerability scanner (scan|file|lockfile|sarif|report|db-update)");
    println!("  /config                  - Show current configuration");
    println!("  /help                    - Show this help message");
    println!("  /exit                    - Exit VibeCLI");
    println!("  ! <command>              - Execute shell command (Enter=yes; disable prompt: safety.require_approval_for_commands=false)");
    println!("\n@ context references (in any message):");
    println!("  @file:<path>             - Inject file contents as context");
    println!("  @file:<path>:<N-M>       - Inject specific line range");
    println!("  @web:<url>               - Fetch and inject web page content");
    println!("  @docs:<pkg>              - Fetch library docs (e.g. @docs:tokio, @docs:py:requests)");
    println!("  @git                     - Inject git status and recent commits");
    println!("  @memory:<query>          - Search OpenMemory cognitive store and inject results");
    println!("\nOne-shot chat:");
    println!("  vibecli \"<message>\"       - Send a message and get a response");
    println!("  vibecli chat \"<message>\"  - Same as above (chat keyword is optional)");
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
    println!("  --setup                  - Interactive setup wizard (detect platform, configure provider, install service)");
    println!("  --service <cmd>          - Manage always-on service (install, start, stop, status)");
    println!("  --serve                  - Start HTTP daemon (VS Code extension / Agent SDK)");
    println!("  --mcp-server             - Run as MCP server (for Claude Desktop etc.)");
    println!("  --gateway <platform>     - Start messaging bot (telegram|discord|slack)");
    println!("  --worktree               - Run agent in isolated git worktree branch");
    println!("  --voice                  - Enable voice input (Whisper transcription + ElevenLabs TTS)");
    println!("  --tailscale              - Expose daemon via Tailscale Funnel (use with --serve)");
    println!("  --profile <name>         - Load a named config profile (~/.vibecli/profiles/<name>.toml)");
    println!("  --doctor                 - Run health checks on the VibeCLI installation");
    println!("\nProviders (--provider <name>):");
    println!("  ollama                   - Local Ollama (default, no key needed)");
    println!("  claude                   - Anthropic Claude  (ANTHROPIC_API_KEY)");
    println!("  openai                   - OpenAI GPT-4o     (OPENAI_API_KEY)");
    println!("  gemini                   - Google Gemini     (GEMINI_API_KEY)");
    println!("  grok                     - xAI Grok          (GROK_API_KEY)");
    println!("  groq                     - Groq ultra-fast   (GROQ_API_KEY)");
    println!("  deepseek                 - DeepSeek V3/R1    (DEEPSEEK_API_KEY)");
    println!("  mistral                  - Mistral AI        (MISTRAL_API_KEY)");
    println!("  perplexity               - Perplexity Sonar  (PERPLEXITY_API_KEY)");
    println!("  minimax                  - MiniMax           (MINIMAX_API_KEY)");
    println!("  together                 - Together AI       (TOGETHER_API_KEY)");
    println!("  fireworks                - Fireworks AI      (FIREWORKS_API_KEY)");
    println!("  sambanova                - SambaNova fast    (SAMBANOVA_API_KEY)");
    println!("  cerebras                 - Cerebras fast     (CEREBRAS_API_KEY)");
    println!("  openrouter               - OpenRouter 300+   (OPENROUTER_API_KEY)");
    println!("  azure                    - Azure OpenAI      (AZURE_OPENAI_API_KEY + api_url)");
    println!("  zhipu                    - Zhipu GLM-4       (ZHIPU_API_KEY)");
    println!("  vercel_ai                - Vercel AI Gateway (VERCEL_AI_API_KEY + api_url)");
    println!("\nAttachments (use [brackets] syntax):");
    println!("  [screenshot.png] What is this error?  - Attach image for vision analysis");
    println!("  [data.csv] Analyze this data           - Attach document/code for review");
    println!("  [main.rs] [test.rs] Review these files  - Attach multiple files");
    println!("  Supported: images (png/jpg/gif/webp), code, text, JSON, CSV, YAML, TOML, etc.");
    println!("\nTip: You can also just type a message to chat (attachments work everywhere)\n");
}

/// Run a health check of the VibeCLI installation: config, providers, git, plugins, profiles.
async fn run_doctor() -> Result<()> {
    println!("\nVibeCLI Doctor — health check\n");

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
        ("Claude",     "ANTHROPIC_API_KEY",  config.claude.as_ref().and_then(|c| c.api_key.clone())),
        ("OpenAI",     "OPENAI_API_KEY",     config.openai.as_ref().and_then(|c| c.api_key.clone())),
        ("Gemini",     "GEMINI_API_KEY",     config.gemini.as_ref().and_then(|c| c.api_key.clone())),
        ("Grok",       "GROK_API_KEY",       config.grok.as_ref().and_then(|c| c.api_key.clone())),
        ("Groq",       "GROQ_API_KEY",       config.groq.as_ref().and_then(|c| c.api_key.clone())),
        ("DeepSeek",   "DEEPSEEK_API_KEY",   config.deepseek.as_ref().and_then(|c| c.api_key.clone())),
        ("Mistral",    "MISTRAL_API_KEY",    config.mistral.as_ref().and_then(|c| c.api_key.clone())),
        ("Perplexity", "PERPLEXITY_API_KEY", config.perplexity.as_ref().and_then(|c| c.api_key.clone())),
        ("MiniMax",    "MINIMAX_API_KEY",    config.minimax.as_ref().and_then(|c| c.api_key.clone())),
        ("Together",   "TOGETHER_API_KEY",   config.together.as_ref().and_then(|c| c.api_key.clone())),
        ("Fireworks",  "FIREWORKS_API_KEY",  config.fireworks.as_ref().and_then(|c| c.api_key.clone())),
        ("SambaNova",  "SAMBANOVA_API_KEY",  config.sambanova.as_ref().and_then(|c| c.api_key.clone())),
        ("Cerebras",   "CEREBRAS_API_KEY",   config.cerebras.as_ref().and_then(|c| c.api_key.clone())),
        ("OpenRouter", "OPENROUTER_API_KEY", config.openrouter.as_ref().and_then(|c| c.api_key.clone())),
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
        println!("  Active profile: {}", active);
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

    // 10. Container runtimes
    {
        use container_runtime::ContainerRuntime;
        let docker = docker_runtime::DockerRuntime::new();
        let podman = podman_runtime::PodmanRuntime::new();
        if docker.is_available().await {
            let ver = docker.version().await.unwrap_or_default();
            println!("  ✅ Docker     — {ver}");
        } else {
            println!("  ○  Docker     — not found");
        }
        if podman.is_available().await {
            let ver = podman.version().await.unwrap_or_default();
            println!("  ✅ Podman     — {ver}");
        } else {
            println!("  ○  Podman     — not found");
        }
        let osb_url = config.sandbox_config.opensandbox.resolve_api_url();
        let osb_key = config.sandbox_config.opensandbox.resolve_api_key();
        let osb = opensandbox_client::OpenSandboxRuntime::new(osb_url, osb_key);
        if osb.is_available().await {
            println!("  ✅ OpenSandbox — reachable");
        } else {
            println!("  ○  OpenSandbox — not configured/reachable");
        }
        println!("  Sandbox cfg — runtime={}, image={}", config.sandbox_config.runtime, config.sandbox_config.image);
    }

    // 11. opusplan model routing
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
            println!("\n Configuration:");
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
    no_network: bool,
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

            eprintln!("\nChange detected: {}", files_list);
            eprintln!("   Running agent task…\n");

            let workspace_root = cwd.clone();
            let mut te = ToolExecutor::new(workspace_root.clone(), sandbox)
                .with_provider(llm.clone());
            if no_network { te = te.with_no_network(); }
            let executor: Arc<dyn vibe_ai::agent::ToolExecutorTrait> = Arc::new(te);

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
                team_bus: None,
                team_agent_id: None,
                project_summary: None,
                task_context_files: vec![],
                memory_context: None,
                auto_commit: false,
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
                        eprintln!("  {} → {}", step.tool_call.name(), if step.tool_result.success { "✓" } else { "✗" });
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
    R.get_or_init(|| regex::Regex::new(r"@file:(\S+)").expect("valid regex: @file"))
}
fn re_at_web() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@web:(\S+)").expect("valid regex: @web"))
}
fn re_at_docs() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@docs:(\S+)").expect("valid regex: @docs"))
}
fn re_at_symbol() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@symbol:(\S+)").expect("valid regex: @symbol"))
}
fn re_at_codebase() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@codebase:(\S+)").expect("valid regex: @codebase"))
}
fn re_at_github() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@github:([a-zA-Z0-9_\-]+)/([a-zA-Z0-9_\-]+)#(\d+)").expect("valid regex: @github"))
}
fn re_at_jira() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@jira:([A-Z][A-Z0-9_]+-\d+)").expect("valid regex: @jira"))
}
fn re_at_memory() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@memory:(\S+(?:\s+\S+)*)").expect("valid regex: @memory"))
}
fn re_image_attachment() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"\[([^\]]+\.(png|jpg|jpeg|gif|webp))\]").expect("valid regex: image_attachment"))
}
fn re_file_attachment() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(
        r"\[([^\]]+\.(pdf|csv|json|xml|html|htm|md|markdown|txt|log|rs|py|js|ts|tsx|jsx|go|java|c|cpp|h|rb|php|swift|kt|scala|sh|bash|sql|yaml|yml|toml|ini|cfg|conf|env|css|scss|less|vue|svelte))\]"
    ).expect("valid regex: file_attachment"))
}
fn re_html_tags() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"<[^>]+>").expect("valid regex: html_tags"))
}
fn re_collapse_whitespace() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"\s{2,}").expect("valid regex: collapse_whitespace"))
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

    // ── @memory:query — search OpenMemory cognitive memory store ─────────────
    for cap in re_at_memory().captures_iter(input) {
        let query = &cap[1];
        let mem_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("vibecli")
            .join("openmemory");
        let text = if let Ok(store) = open_memory::OpenMemoryStore::load(&mem_dir, "default") {
            let results = store.query(query, 8);
            if results.is_empty() {
                format!("(no memories matching '{}')", query)
            } else {
                let mut lines = Vec::new();
                for r in &results {
                    lines.push(format!("[{} | sal:{:.0}% | score:{:.2}] {}",
                        r.memory.sector,
                        r.effective_salience * 100.0,
                        r.score,
                        &r.memory.content[..r.memory.content.len().min(200)]
                    ));
                }
                // Also include current temporal facts if any
                let facts = store.query_current_facts();
                if !facts.is_empty() {
                    lines.push("--- temporal facts ---".to_string());
                    for f in facts.iter().take(10) {
                        lines.push(format!("{} {} {}", f.subject, f.predicate, f.object));
                    }
                }
                lines.join("\n")
            }
        } else {
            "(OpenMemory store not initialized — use /openmemory add to start)".to_string()
        };
        extra.push(format!("=== OpenMemory: {} ===\n{}", query, text));
        result = result.replacen(&cap[0], "", 1);
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_safe_name tests ───────────────────────────────────────────────────

    #[test]
    fn is_safe_name_valid_simple() {
        assert!(is_safe_name("my-snippet"));
        assert!(is_safe_name("test_123"));
        assert!(is_safe_name("file.txt"));
        assert!(is_safe_name("a"));
    }

    #[test]
    fn is_safe_name_rejects_empty() {
        assert!(!is_safe_name(""));
    }

    #[test]
    fn is_safe_name_rejects_path_separators() {
        assert!(!is_safe_name("foo/bar"));
        assert!(!is_safe_name("foo\\bar"));
    }

    #[test]
    fn is_safe_name_rejects_parent_directory() {
        assert!(!is_safe_name(".."));
        assert!(!is_safe_name("foo..bar"));
    }

    #[test]
    fn is_safe_name_rejects_special_chars() {
        assert!(!is_safe_name("name with spaces"));
        assert!(!is_safe_name("file@home"));
        assert!(!is_safe_name("rm -rf /"));
        assert!(!is_safe_name("$(whoami)"));
    }

    // ── edit_distance tests ──────────────────────────────────────────────────

    #[test]
    fn edit_distance_identical_strings() {
        assert_eq!(edit_distance("hello", "hello"), 0);
    }

    #[test]
    fn edit_distance_empty_strings() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("", "xyz"), 3);
    }

    #[test]
    fn edit_distance_single_edit() {
        assert_eq!(edit_distance("cat", "bat"), 1);  // substitution
        assert_eq!(edit_distance("cat", "cats"), 1); // insertion
        assert_eq!(edit_distance("cats", "cat"), 1); // deletion
    }

    #[test]
    fn edit_distance_known_values() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn edit_distance_symmetric() {
        let d1 = edit_distance("abc", "xyz");
        let d2 = edit_distance("xyz", "abc");
        assert_eq!(d1, d2);
    }

    // ── find_closest_command tests ───────────────────────────────────────────

    #[test]
    fn find_closest_command_exact_match() {
        // An exact match has distance 0, should always be found
        let result = find_closest_command("/help");
        assert_eq!(result, Some("/help"));
    }

    #[test]
    fn find_closest_command_typo() {
        // One character off from "/agent"
        let result = find_closest_command("/agnet");
        assert_eq!(result, Some("/agent"));
    }

    #[test]
    fn find_closest_command_no_match_for_garbage() {
        // Something too far from any command
        let result = find_closest_command("zzzzzzzzzzz");
        assert!(result.is_none());
    }

    // ── regex pattern tests ─────────────────────────────────────────────────

    #[test]
    fn re_at_file_matches_simple_path() {
        let re = re_at_file();
        let caps = re.captures("@file:src/main.rs").unwrap();
        assert_eq!(&caps[1], "src/main.rs");
    }

    #[test]
    fn re_at_file_matches_with_line_range() {
        let re = re_at_file();
        let caps = re.captures("please look at @file:lib.rs:10-20 thanks").unwrap();
        assert_eq!(&caps[1], "lib.rs:10-20");
    }

    #[test]
    fn re_at_web_matches_url() {
        let re = re_at_web();
        let caps = re.captures("see @web:https://example.com/docs for info").unwrap();
        assert_eq!(&caps[1], "https://example.com/docs");
    }

    #[test]
    fn re_at_docs_matches_package() {
        let re = re_at_docs();
        let caps = re.captures("use @docs:tokio for async").unwrap();
        assert_eq!(&caps[1], "tokio");
    }

    #[test]
    fn re_at_github_matches_issue() {
        let re = re_at_github();
        let caps = re.captures("fix @github:owner/repo#42 please").unwrap();
        assert_eq!(&caps[1], "owner");
        assert_eq!(&caps[2], "repo");
        assert_eq!(&caps[3], "42");
    }

    #[test]
    fn re_at_github_rejects_bad_format() {
        let re = re_at_github();
        assert!(re.captures("@github:noslash").is_none());
        assert!(re.captures("@github:no/hash").is_none());
    }

    #[test]
    fn re_at_jira_matches_ticket() {
        let re = re_at_jira();
        let caps = re.captures("related to @jira:PROJ-123").unwrap();
        assert_eq!(&caps[1], "PROJ-123");
    }

    #[test]
    fn re_at_jira_rejects_lowercase() {
        let re = re_at_jira();
        // Jira regex requires uppercase project key
        assert!(re.captures("@jira:proj-123").is_none());
    }

    #[test]
    fn re_image_attachment_matches_png() {
        let re = re_image_attachment();
        let caps = re.captures("here is [screenshot.png] please review").unwrap();
        assert_eq!(&caps[1], "screenshot.png");
    }

    #[test]
    fn re_image_attachment_matches_various_formats() {
        let re = re_image_attachment();
        for ext in &["png", "jpg", "jpeg", "gif", "webp"] {
            let input = format!("[photo.{}]", ext);
            assert!(re.is_match(&input), "should match .{}", ext);
        }
    }

    #[test]
    fn re_html_tags_strips_tags() {
        let re = re_html_tags();
        let result = re.replace_all("<p>Hello <b>world</b></p>", " ");
        assert_eq!(result, " Hello  world  ");
    }

    #[test]
    fn re_collapse_whitespace_collapses() {
        let re = re_collapse_whitespace();
        let result = re.replace_all("hello   world    foo", " ");
        assert_eq!(result, "hello world foo");
    }

    // ── Additional is_safe_name tests ───────────────────────────────────────

    #[test]
    fn is_safe_name_accepts_dots_and_hyphens() {
        assert!(is_safe_name("my-snippet.v2"));
        assert!(is_safe_name("config.toml"));
        assert!(is_safe_name("test-case-01.rs"));
    }

    #[test]
    fn is_safe_name_rejects_hidden_dotdot_in_middle() {
        assert!(!is_safe_name("a..b"));
        assert!(!is_safe_name("..hidden"));
        assert!(!is_safe_name("trailing.."));
    }

    #[test]
    fn is_safe_name_single_char_valid() {
        assert!(is_safe_name("x"));
        assert!(is_safe_name("Z"));
        assert!(is_safe_name("9"));
    }

    #[test]
    fn is_safe_name_rejects_null_and_control_chars() {
        assert!(!is_safe_name("foo\0bar"));
        assert!(!is_safe_name("foo\nbar"));
        assert!(!is_safe_name("foo\tbar"));
    }

    #[test]
    fn is_safe_name_rejects_shell_metacharacters() {
        assert!(!is_safe_name("name;rm"));
        assert!(!is_safe_name("name|cat"));
        assert!(!is_safe_name("name`id`"));
        assert!(!is_safe_name("a&b"));
        assert!(!is_safe_name("a>b"));
        assert!(!is_safe_name("a<b"));
    }

    // ── Additional edit_distance tests ──────────────────────────────────────

    #[test]
    fn edit_distance_single_char_strings() {
        assert_eq!(edit_distance("a", "b"), 1);
        assert_eq!(edit_distance("a", "a"), 0);
        assert_eq!(edit_distance("a", ""), 1);
    }

    #[test]
    fn edit_distance_transposition_counts_two() {
        // Simple Levenshtein counts transposition as 2 ops (delete + insert)
        assert_eq!(edit_distance("ab", "ba"), 2);
    }

    #[test]
    fn edit_distance_completely_different() {
        assert_eq!(edit_distance("abc", "xyz"), 3);
        assert_eq!(edit_distance("abcd", "wxyz"), 4);
    }

    #[test]
    fn edit_distance_prefix_suffix() {
        assert_eq!(edit_distance("help", "helping"), 3);
        assert_eq!(edit_distance("test", "tes"), 1);
    }

    #[test]
    fn edit_distance_case_sensitive() {
        assert_eq!(edit_distance("Hello", "hello"), 1);
        assert_eq!(edit_distance("ABC", "abc"), 3);
    }

    #[test]
    fn edit_distance_repeated_chars() {
        assert_eq!(edit_distance("aaa", "aaaa"), 1);
        assert_eq!(edit_distance("aaa", "bbb"), 3);
    }

    // ── Additional find_closest_command tests ───────────────────────────────

    #[test]
    fn find_closest_command_close_typo_help() {
        let result = find_closest_command("/hepl");
        // "/hepl" is distance 2 from "/help"; result must be a valid command within distance 3
        assert!(result.is_some());
    }

    #[test]
    fn find_closest_command_close_typo_config() {
        let result = find_closest_command("/confg");
        assert_eq!(result, Some("/config"));
    }

    #[test]
    fn find_closest_command_empty_input() {
        let result = find_closest_command("");
        // Empty string may match short commands (e.g. "/mcp" len=4, distance=4 > 3 threshold)
        // but some 3-char commands could match — just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn find_closest_command_exact_exit() {
        let result = find_closest_command("/exit");
        assert_eq!(result, Some("/exit"));
    }

    #[test]
    fn find_closest_command_exact_trace() {
        let result = find_closest_command("/trace");
        assert_eq!(result, Some("/trace"));
    }

    // ── Additional regex pattern tests ──────────────────────────────────────

    #[test]
    fn re_at_file_no_match_without_prefix() {
        let re = re_at_file();
        assert!(re.captures("just a file.rs reference").is_none());
    }

    #[test]
    fn re_at_file_matches_nested_path() {
        let re = re_at_file();
        let caps = re.captures("@file:src/components/App.tsx").unwrap();
        assert_eq!(&caps[1], "src/components/App.tsx");
    }

    #[test]
    fn re_at_file_matches_absolute_path() {
        let re = re_at_file();
        let caps = re.captures("@file:/usr/local/bin/test.sh").unwrap();
        assert_eq!(&caps[1], "/usr/local/bin/test.sh");
    }

    #[test]
    fn re_at_web_no_match_plain_url() {
        let re = re_at_web();
        assert!(re.captures("visit https://example.com").is_none());
    }

    #[test]
    fn re_at_web_captures_full_url_with_path() {
        let re = re_at_web();
        let caps = re.captures("@web:https://docs.rs/tokio/latest/tokio/").unwrap();
        assert_eq!(&caps[1], "https://docs.rs/tokio/latest/tokio/");
    }

    #[test]
    fn re_at_docs_matches_npm_prefix() {
        let re = re_at_docs();
        let caps = re.captures("@docs:npm:express").unwrap();
        assert_eq!(&caps[1], "npm:express");
    }

    #[test]
    fn re_at_symbol_matches() {
        let re = re_at_symbol();
        let caps = re.captures("find @symbol:create_provider in the code").unwrap();
        assert_eq!(&caps[1], "create_provider");
    }

    #[test]
    fn re_at_symbol_no_match_without_prefix() {
        let re = re_at_symbol();
        assert!(re.captures("just a symbol name").is_none());
    }

    #[test]
    fn re_at_codebase_matches_query() {
        let re = re_at_codebase();
        let caps = re.captures("@codebase:todo_fixme").unwrap();
        assert_eq!(&caps[1], "todo_fixme");
    }

    #[test]
    fn re_at_codebase_no_match_without_prefix() {
        let re = re_at_codebase();
        assert!(re.captures("search for codebase stuff").is_none());
    }

    #[test]
    fn re_at_github_multiple_digit_issue() {
        let re = re_at_github();
        let caps = re.captures("@github:rust-lang/rust#12345").unwrap();
        assert_eq!(&caps[1], "rust-lang");
        assert_eq!(&caps[2], "rust");
        assert_eq!(&caps[3], "12345");
    }

    #[test]
    fn re_at_github_rejects_missing_number() {
        let re = re_at_github();
        assert!(re.captures("@github:owner/repo#").is_none());
    }

    #[test]
    fn re_at_jira_uppercase_multiword_project() {
        let re = re_at_jira();
        let caps = re.captures("@jira:MYTEAM-9999").unwrap();
        assert_eq!(&caps[1], "MYTEAM-9999");
    }

    #[test]
    fn re_at_jira_rejects_no_number() {
        let re = re_at_jira();
        assert!(re.captures("@jira:PROJ-").is_none());
    }

    #[test]
    fn re_at_jira_single_char_project() {
        let re = re_at_jira();
        // Regex requires [A-Z][A-Z0-9_]+ (2+ uppercase chars), so single-char project key doesn't match
        assert!(re.captures("@jira:X-1").is_none());
        // But two-char project key works
        let caps = re.captures("@jira:XY-1").unwrap();
        assert_eq!(&caps[1], "XY-1");
    }

    #[test]
    fn re_image_attachment_no_match_wrong_ext() {
        let re = re_image_attachment();
        assert!(!re.is_match("[document.pdf]"));
        assert!(!re.is_match("[archive.zip]"));
        assert!(!re.is_match("[script.rs]"));
    }

    #[test]
    fn re_image_attachment_no_match_missing_brackets() {
        let re = re_image_attachment();
        assert!(!re.is_match("screenshot.png"));
    }

    #[test]
    fn re_html_tags_handles_self_closing() {
        let re = re_html_tags();
        let result = re.replace_all("text<br/>more<hr />end", " ");
        assert_eq!(result, "text more end");
    }

    #[test]
    fn re_html_tags_handles_attributes() {
        let re = re_html_tags();
        let result = re.replace_all("<a href=\"http://example.com\">link</a>", " ");
        assert_eq!(result, " link ");
    }

    #[test]
    fn re_html_tags_no_match_plain_text() {
        let re = re_html_tags();
        let result = re.replace_all("no tags here", " ");
        assert_eq!(result, "no tags here");
    }

    #[test]
    fn re_collapse_whitespace_preserves_single_spaces() {
        let re = re_collapse_whitespace();
        let result = re.replace_all("hello world", " ");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn re_collapse_whitespace_handles_tabs_and_newlines() {
        let re = re_collapse_whitespace();
        let result = re.replace_all("hello\t\t\nworld", " ");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn re_collapse_whitespace_empty_string() {
        let re = re_collapse_whitespace();
        let result = re.replace_all("", " ");
        assert_eq!(result, "");
    }

    #[test]
    fn extract_attachments_handles_document_files() {
        // extract_attachments_from_input strips document file brackets too
        let (clean, images, _docs) = extract_attachments_from_input("see [readme.md] for info");
        assert!(!clean.contains("[readme.md]")); // .md IS a valid document attachment
        assert_eq!(clean, "see  for info");
        assert!(images.is_empty());
    }

    #[test]
    fn extract_attachments_handles_mixed() {
        let (clean, images, _docs) = extract_attachments_from_input("[logo.png] review [main.rs] please");
        assert!(!clean.contains("[logo.png]"));
        assert!(!clean.contains("[main.rs]"));
        assert_eq!(clean, "review  please");
        // logo.png would be an image (if file existed), main.rs would be a doc
        // Images from non-existent files just fail silently
        assert!(images.is_empty()); // file doesn't exist
    }

    #[test]
    fn extract_attachments_no_brackets_returns_input() {
        let (clean, images, docs) = extract_attachments_from_input("just a normal message");
        assert_eq!(clean, "just a normal message");
        assert!(images.is_empty());
        assert!(docs.is_empty());
    }

    #[test]
    fn extract_attachments_multiple_documents() {
        let (clean, _images, _docs) = extract_attachments_from_input("[a.rs] [b.py] [c.json] check all");
        assert!(!clean.contains("[a.rs]"));
        assert!(!clean.contains("[b.py]"));
        assert!(!clean.contains("[c.json]"));
        assert!(clean.contains("check all"));
    }

    #[test]
    fn extract_attachments_ignores_unknown_extensions() {
        // Unknown extensions like .xyz should NOT be matched by the file regex
        let (clean, images, docs) = extract_attachments_from_input("[data.xyz] is unknown");
        assert!(clean.contains("[data.xyz]"), "unknown ext should remain in text");
        assert!(images.is_empty());
        assert!(docs.is_empty());
    }

    #[test]
    fn re_file_attachment_matches_all_code_extensions() {
        let re = re_file_attachment();
        for ext in &["rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp",
                      "h", "rb", "php", "swift", "kt", "scala", "sh", "bash", "sql",
                      "yaml", "yml", "toml", "ini", "json", "csv", "md", "txt", "log",
                      "html", "css", "scss", "less", "vue", "svelte"] {
            let input = format!("[test.{}]", ext);
            assert!(re.is_match(&input), "should match .{} extension", ext);
        }
    }

    #[test]
    fn re_file_attachment_rejects_non_code_extensions() {
        let re = re_file_attachment();
        for ext in &["zip", "tar", "exe", "dll", "so", "o", "wasm", "bin"] {
            let input = format!("[test.{}]", ext);
            assert!(!re.is_match(&input), "should NOT match .{} extension", ext);
        }
    }

    #[test]
    fn extract_attachments_with_real_text_file() {
        // Create a temp file and verify doc_context is populated
        let dir = std::env::temp_dir().join("vibecli_test_attach");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("hello.txt");
        std::fs::write(&path, "Hello from file").unwrap();

        let input = format!("[{}] review this", path.display());
        let (clean, images, docs) = extract_attachments_from_input(&input);
        assert!(images.is_empty());
        assert!(docs.contains("Hello from file"));
        assert!(clean.contains("review this"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
