//! VibeUI - AI-Powered Code Editor
#![recursion_limit = "512"]

mod commands;
mod flow;
mod agent_executor;
mod memory;
mod panel_store;
pub mod shadow_workspace;
pub mod sonar_rules;

use commands::AppState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;
use vibe_core::Workspace;
use vibe_ai::{ChatEngine, providers, AIConfig};
use vibe_ai::provider::ProviderConfig;
use std::path::PathBuf;
use vibe_core::terminal::TerminalManager;
use vibe_lsp::manager::LspManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── Fix PATH for macOS .app bundles ──────────────────────────────────
    // When launched from Finder/Launchpad, macOS gives apps a minimal PATH
    // (/usr/bin:/bin:/usr/sbin:/sbin) that excludes Homebrew, ~/.cargo/bin,
    // nvm, etc. We source the user's login shell to get the real PATH.
    #[cfg(target_os = "macos")]
    {
        if let Ok(shell) = std::env::var("SHELL").or_else(|_| Ok::<String, std::env::VarError>("/bin/zsh".to_string())) {
            if let Ok(output) = std::process::Command::new(&shell)
                .args(["-l", "-c", "echo __PATH_START__${PATH}__PATH_END__"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let (Some(start), Some(end)) = (stdout.find("__PATH_START__"), stdout.find("__PATH_END__")) {
                    let shell_path = &stdout[start + 14..end];
                    // Merge: prepend shell-derived paths to current PATH
                    let current = std::env::var("PATH").unwrap_or_default();
                    let merged = if current.is_empty() {
                        shell_path.to_string()
                    } else {
                        format!("{shell_path}:{current}")
                    };
                    // SAFETY: Called before the async runtime starts, so no other threads are running.
                    unsafe { std::env::set_var("PATH", &merged); }
                }
            }
        }
    }

    // Initialize workspace
    let workspace = Arc::new(Mutex::new(Workspace::new("VibeUI Workspace".to_string())));

    // Load AI configuration — resolve from ~/.vibeui/vibe.toml so it works
    // when the app is launched from Finder (where cwd is /).
    let config_path = dirs::home_dir()
        .map(|h| h.join(".vibeui").join("vibe.toml"))
        .unwrap_or_else(|| PathBuf::from("vibe.toml"));
    let ai_config = AIConfig::load_from_file(&config_path).unwrap_or_default();

    // Initialize Chat Engine
    let mut chat_engine = ChatEngine::new();

    // Initialize Ollama from config if present.
    // No hardcoded model fallback — real models are discovered dynamically
    // by get_available_ai_providers() which queries Ollama's /api/tags endpoint
    // and registers each installed model as a separate provider.
    if let Some(ollama_conf) = ai_config.ollama {
        if ollama_conf.enabled {
            if let Some(model) = ollama_conf.model {
                let config = ProviderConfig {
                    provider_type: "ollama".to_string(),
                    api_key: ollama_conf.api_key,
                    model,
                    api_url: ollama_conf.api_url.or_else(|| Some("http://127.0.0.1:11434".to_string())),
                    max_tokens: ollama_conf.max_tokens,
                    temperature: ollama_conf.temperature,
                    ..Default::default()
                };
                let provider = providers::ollama::OllamaProvider::new(config);
                chat_engine.add_provider(Arc::new(provider));
            }
            // If no model specified in config, skip — models will be auto-discovered
        }
    }
    // No else-branch: don't register a hardcoded fallback model.
    // get_available_ai_providers() discovers all locally installed Ollama models.

    // Load saved API keys from ~/.vibecli/profile_settings.db and register cloud providers at startup.
    {
        let settings = commands::load_api_key_settings();
        commands::register_cloud_providers(&mut chat_engine, &settings);
    }

    let chat_engine = Arc::new(Mutex::new(chat_engine));
    let terminal_manager = Arc::new(TerminalManager::new());
    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));
    let flow = Arc::new(Mutex::new(flow::FlowTracker::new()));
    let agent_pending = Arc::new(Mutex::new(None));
    let terminal_buffer = Arc::new(Mutex::new(Vec::<String>::new()));
    let agent_abort_handle = Arc::new(Mutex::new(None));
    let chat_abort_handle = Arc::new(Mutex::new(None));
    let provider_health = Arc::new(vibe_ai::ProviderHealthTracker::new(100, std::time::Duration::from_secs(3600)));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Replace the default native menu with a minimal app-only menu.
            // On macOS this keeps Cmd+Q / Cmd+H / Cmd+M working while removing
            // the duplicate File/Edit/View/Window/Help menus (handled in React).
            // On Windows/Linux there is no native menu bar by default, so this
            // is effectively a no-op.
            use tauri::menu::{MenuBuilder, SubmenuBuilder, PredefinedMenuItem};
            let app_submenu = SubmenuBuilder::new(app, "VibeUI")
                .about(None)
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;
            let edit_submenu = SubmenuBuilder::new(app, "Edit")
                .item(&PredefinedMenuItem::undo(app, None)?)
                .item(&PredefinedMenuItem::redo(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::cut(app, None)?)
                .item(&PredefinedMenuItem::copy(app, None)?)
                .item(&PredefinedMenuItem::paste(app, None)?)
                .item(&PredefinedMenuItem::select_all(app, None)?)
                .build()?;
            let menu = MenuBuilder::new(app)
                .item(&app_submenu)
                .item(&edit_submenu)
                .build()?;
            app.set_menu(menu)?;

            // Auto-spawn the vibecli daemon in the background if it isn't already running.
            // This makes BackgroundJobs, Collab, and other HTTP-based panels work without
            // the user needing to manually start `vibecli --serve`.
            {
                let app_handle2 = app.handle().clone();
                let daemon_proc = app.state::<AppState>().daemon_process.clone();
                tauri::async_runtime::spawn(async move {
                    use tauri::Emitter;
                    // Give the main window time to render before we try to spawn.
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    // Skip if something is already listening on 7878.
                    let already_up = tokio::net::TcpStream::connect("127.0.0.1:7878").await.is_ok();
                    if already_up { return; }
                    let Some(binary) = crate::commands::find_vibecli_binary() else {
                        eprintln!("[daemon] vibecli binary not found — daemon will not auto-start");
                        let _ = app_handle2.emit(
                            "daemon:error",
                            serde_json::json!({
                                "stage": "find-binary",
                                "message": "vibecli binary not found on PATH or in ~/.cargo/bin. \
                                            Build with `cargo install --path vibecli/vibecli-cli` or set PATH.",
                            }),
                        );
                        return;
                    };
                    let spawn_result = tokio::process::Command::new(&binary)
                        .args(["--serve", "--port", "7878"])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::piped())
                        .kill_on_drop(true)
                        .spawn();
                    let mut child = match spawn_result {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("[daemon] spawn failed for {}: {e}", binary.display());
                            let _ = app_handle2.emit(
                                "daemon:error",
                                serde_json::json!({
                                    "stage": "spawn",
                                    "binary": binary.to_string_lossy(),
                                    "message": e.to_string(),
                                }),
                            );
                            return;
                        }
                    };
                    // Capture stderr so a crash-on-startup is visible in the
                    // event stream instead of being swallowed.
                    if let Some(stderr) = child.stderr.take() {
                        let app_handle3 = app_handle2.clone();
                        tauri::async_runtime::spawn(async move {
                            use tokio::io::{AsyncBufReadExt, BufReader};
                            let mut lines = BufReader::new(stderr).lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                eprintln!("[daemon] {line}");
                                let _ = app_handle3.emit(
                                    "daemon:stderr",
                                    serde_json::json!({ "line": line }),
                                );
                            }
                        });
                    }
                    *daemon_proc.lock().await = Some(child);
                    // Wait for it to come up, then notify the frontend.
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    if tokio::net::TcpStream::connect("127.0.0.1:7878").await.is_ok() {
                        let _ = app_handle2.emit(
                            "daemon:online",
                            serde_json::json!({ "port": 7878, "auto": true }),
                        );
                    } else {
                        let _ = app_handle2.emit(
                            "daemon:error",
                            serde_json::json!({
                                "stage": "wait-for-listen",
                                "message": "spawned vibecli but it did not start listening on 127.0.0.1:7878 within 2s",
                            }),
                        );
                    }
                });
            }

            Ok(())
        })
        .manage(AppState {
            workspace,
            chat_engine,
            terminal_manager,
            lsp_manager,
            flow,
            agent_pending,
            terminal_buffer,
            agent_abort_handle,
            chat_abort_handle,
            provider_health,
            mock_server_handle: Arc::new(Mutex::new(None)),
            mock_routes: Arc::new(Mutex::new(Vec::new())),
            mock_request_log: Arc::new(Mutex::new(Vec::new())),
            sub_agents: Arc::new(Mutex::new(Vec::new())),
            active_team: Arc::new(Mutex::new(None)),
            a2a_agents: Arc::new(Mutex::new(Vec::new())),
            a2a_tasks: Arc::new(Mutex::new(Vec::new())),
            a2a_metrics: Arc::new(Mutex::new(serde_json::json!({
                "tasks_created": 0,
                "tasks_completed": 0,
                "tasks_failed": 0,
                "tasks_cancelled": 0,
                "agents_discovered": 0
            }))),
            a2a_local_card: Arc::new(Mutex::new(serde_json::json!({
                "name": "VibeCody",
                "description": "VibeCody AI coding assistant — A2A-compatible agent",
                "version": "1.0.0",
                "capabilities": ["code_generation", "code_review", "debugging", "refactoring", "testing"],
                "endpoint": "http://localhost:9876/a2a",
                "protocol": "a2a/1.0"
            }))),
            // Phase 24
            worktree_agents: Arc::new(Mutex::new(Vec::new())),
            worktree_metrics: Arc::new(Mutex::new(serde_json::json!({ "total_spawned": 0, "completed": 0, "failed": 0, "merge_conflicts": 0 }))),
            hosted_agents: Arc::new(Mutex::new(Vec::new())),
            host_output: Arc::new(Mutex::new(Vec::new())),
            host_processes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            host_clipboard: Arc::new(Mutex::new(Vec::new())),
            // Phase 25
            proactive_suggestions: Arc::new(Mutex::new(Vec::new())),
            proactive_metrics: Arc::new(Mutex::new(serde_json::json!({ "total_scans": 0, "total_suggestions": 0, "accepted": 0, "rejected": 0 }))),
            triage_results: Arc::new(Mutex::new(Vec::new())),
            triage_metrics: Arc::new(Mutex::new(serde_json::json!({ "total_triaged": 0, "auto_labeled": 0, "avg_confidence": 0.0 }))),
            // Phase 26
            web_search_results: Arc::new(Mutex::new(Vec::new())),
            web_citations: Arc::new(Mutex::new(Vec::new())),
            web_cache: Arc::new(Mutex::new(serde_json::json!({ "total_entries": 0, "hit_count": 0, "miss_count": 0 }))),
            web_grounding_engine: Arc::new(Mutex::new(
                vibecli_cli::web_grounding::WebGroundingEngine::new(
                    commands::initial_search_config(),
                ),
            )),
            web_http_client: reqwest::Client::new(),
            semindex_symbols: Arc::new(Mutex::new(Vec::new())),
            semindex_stats: Arc::new(Mutex::new(serde_json::json!({ "total_symbols": 0, "total_call_edges": 0, "total_files": 0 }))),
            // Phase 27-28: MCP HTTP + MCTS Repair + Cost Router
            mcp_http_state: Arc::new(Mutex::new(serde_json::json!({ "server_running": false, "connections": 0, "oauth_configured": false }))),
            repair_sessions: Arc::new(Mutex::new(Vec::new())),
            route_decisions: Arc::new(Mutex::new(Vec::new())),
            route_budget: Arc::new(Mutex::new(serde_json::json!({ "total": 100.0, "spent": 0.0, "remaining": 100.0, "period": "monthly" }))),
            // Phase 29: Visual Verify + NextTask + DocSync
            vverify_baselines: Arc::new(Mutex::new(Vec::new())),
            nexttask_suggestions: Arc::new(Mutex::new(Vec::new())),
            docsync_state: Arc::new(Mutex::new(serde_json::json!({ "total_sections": 0, "avg_freshness": 100.0, "stale_count": 0, "alerts": 0 }))),
            // Phase 30: Connectors + Analytics + Trust + SmartDeps
            connector_instances: Arc::new(Mutex::new(Vec::new())),
            analytics_data: Arc::new(Mutex::new(serde_json::json!({ "total_tasks": 0, "total_cost": 0.0, "time_saved_mins": 0, "roi": 0.0 }))),
            trust_scores: Arc::new(Mutex::new(Vec::new())),
            smartdeps_analysis: Arc::new(Mutex::new(serde_json::json!({ "dependencies": [], "conflicts": [], "advisories": [] }))),
            // Phase 31: RLCEF + LangGraph + Sketch
            rlcef_outcomes: Arc::new(Mutex::new(Vec::new())),
            langgraph_pipelines: Arc::new(Mutex::new(Vec::new())),
            sketch_elements: Arc::new(Mutex::new(Vec::new())),
            // Data Analysis
            da_datasets: Arc::new(Mutex::new(Vec::new())),
            da_charts: Arc::new(Mutex::new(Vec::new())),
            da_widgets: Arc::new(Mutex::new(Vec::new())),
            da_queries: Arc::new(Mutex::new(Vec::new())),
            // Branch Agent
            branch_agents: Arc::new(Mutex::new(Vec::new())),
            branch_prs: Arc::new(Mutex::new(Vec::new())),
            branch_conflicts: Arc::new(Mutex::new(Vec::new())),
            // Audio Output
            narrations: Arc::new(Mutex::new(Vec::new())),
            // Channel Daemon
            daemon_channels: Arc::new(Mutex::new(Vec::new())),
            channel_messages: Arc::new(Mutex::new(Vec::new())),
            sandbox_gateway_abort: Arc::new(Mutex::new(None)),
            sandbox_gateway_log: Arc::new(Mutex::new(Vec::new())),
            sandbox_gateway_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            // CI Gates
            ci_gates: Arc::new(Mutex::new(Vec::new())),
            // Design Import
            design_imports: Arc::new(Mutex::new(Vec::new())),
            desktop_windows: Arc::new(Mutex::new(Vec::new())),
            datagen_schemas: Arc::new(Mutex::new(Vec::new())),
            wizard_configs: Arc::new(Mutex::new(Vec::new())),
            training_jobs: Arc::new(Mutex::new(Vec::new())),
            browser_sessions: Arc::new(Mutex::new(Vec::new())),
            turboquant_index: Arc::new(Mutex::new(None)),
            daemon_process: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::assemble_context,
            commands::read_file,
            commands::read_file_base64,
            commands::write_file,
            commands::list_directory,
            commands::list_directory_sandbox,
            commands::read_file_sandbox,
            commands::write_file_sandbox,
            commands::start_sandbox_gateway,
            commands::stop_sandbox_gateway,
            commands::get_sandbox_gateway_log,
            commands::get_sandbox_gateway_status,
            commands::create_directory,
            commands::delete_item,
            commands::rename_item,
            commands::add_workspace_folder,
            commands::get_workspace_folders,
            commands::open_file_in_workspace,
            commands::insert_text,
            commands::delete_text,
            commands::save_file,
            commands::send_chat_message,
            commands::get_available_ai_providers,
            commands::search_files,
            commands::get_git_status,
            commands::git_commit,
            commands::git_push,
            commands::git_pull,
            commands::git_diff,
            commands::git_list_branches,
            commands::git_switch_branch,
            commands::get_git_config,
            commands::set_git_config,
            commands::store_git_credentials,
            commands::git_get_history,
            commands::git_get_commit_files,
            commands::git_discard_changes,
            commands::spawn_terminal,
            commands::write_terminal,
            commands::resize_terminal,
            commands::apply_batch_edits,
            commands::update_cursors,
            commands::lsp_completion,
            commands::lsp_hover,
            commands::lsp_goto_definition,
            commands::search_files_for_context,
            commands::get_git_context,
            // Phase 3 commands
            commands::git_stash_create,
            commands::git_stash_pop,
            commands::lsp_did_open,
            commands::lsp_did_change,
            commands::lsp_did_save,
            commands::lsp_list_servers,
            commands::track_flow_event,
            commands::get_flow_context,
            // Phase 4 commands — Agent Mode
            commands::start_agent_task,
            commands::stop_agent_task,
            commands::resume_agent_task,
            commands::list_agent_checkpoints,
            commands::start_parallel_agent_task,
            commands::respond_to_agent_approval,
            // Phase 4 commands — Memory / Rules
            commands::get_vibeui_rules,
            commands::save_vibeui_rules,
            commands::get_global_rules,
            commands::save_global_rules,
            // Phase 4 commands — Checkpoints
            commands::create_checkpoint,
            commands::list_checkpoints,
            commands::restore_checkpoint,
            commands::delete_checkpoint,
            // Phase 5 commands — Trace / History
            commands::list_trace_sessions,
            commands::load_trace_session,
            commands::get_all_trace_entries,
            // Attachment commands
            commands::read_attachment,
            // Session Browser commands
            commands::list_sessions,
            commands::get_session_detail,
            commands::delete_session,
            commands::fork_session,
            commands::get_sandbox_status,
            commands::set_sandbox_enabled,
            commands::diffcomplete_generate,
            // Build System commands
            commands::list_workspace_subdirs,
            commands::detect_build_system,
            commands::run_build,
            commands::run_app,
            // Hooks Config UI commands
            commands::get_hooks_config,
            commands::save_hooks_config,
            // Phase 9.1 commands — Manager View (Parallel Agents)
            commands::start_parallel_agents,
            commands::get_orchestrator_status,
            commands::merge_agent_branch,
            // ReviewPanel
            commands::run_code_review,
            // @web context + browser opener
            commands::fetch_url_for_context,
            commands::open_external_url,
            // BYOK Settings
            commands::get_provider_api_keys,
            commands::save_provider_api_keys,
            commands::validate_api_key,
            commands::validate_all_api_keys,
            // Rules directory
            commands::list_rule_files,
            commands::get_rule_file,
            commands::save_rule_file,
            commands::delete_rule_file,
            // MCP server manager
            commands::get_mcp_servers,
            commands::save_mcp_servers,
            commands::test_mcp_server,
            commands::initiate_mcp_oauth,
            commands::complete_mcp_oauth,
            commands::get_mcp_token_status,
            // Test runner (Phase 43)
            commands::detect_test_framework,
            commands::run_tests,
            commands::generate_commit_message,
            // Symbol + codebase search
            commands::search_workspace_symbols,
            // @docs context
            commands::fetch_doc_content,
            // Linter integration
            commands::run_linter,
            // Spec-driven development (Phase 16)
            commands::list_specs,
            commands::get_spec,
            commands::generate_spec,
            commands::update_spec_task,
            commands::run_spec,
            // Code Complete Workflow
            commands::list_workflows,
            commands::get_workflow,
            commands::create_workflow,
            commands::advance_workflow_stage,
            commands::update_workflow_checklist_item,
            commands::generate_stage_checklist,
            // Shadow workspace / lint preview (Phase 18)
            commands::shadow_write_and_lint,
            commands::shadow_get_lint_result,
            // Visual Builder (Phase 19)
            commands::visual_edit_element,
            commands::generate_component,
            commands::import_figma,
            // Deploy + Database (Phase 20)
            commands::detect_deploy_target,
            commands::run_deploy,
            commands::get_deploy_history,
            commands::set_custom_domain,
            commands::find_sqlite_files,
            commands::list_db_tables,
            commands::query_db,
            commands::generate_sql_query,
            commands::generate_migration,
            commands::get_migration_status,
            commands::run_migration_action,
            // Multi-database DataGrip-parity
            commands::db_test_connection,
            commands::db_schema,
            commands::db_query,
            commands::db_save_profile,
            commands::db_list_profiles,
            commands::db_delete_profile,
            commands::db_find_local_files,
            // Phase 26: Supabase
            commands::get_supabase_config,
            commands::save_supabase_config,
            commands::list_supabase_tables,
            commands::query_supabase,
            commands::generate_supabase_query,
            // Phase 26: Auth scaffolding
            commands::generate_auth_scaffold,
            commands::write_auth_scaffold,
            // Phase 26: GitHub Sync
            commands::has_github_token,
            commands::save_github_token,
            commands::get_github_sync_status,
            commands::github_sync_push,
            commands::github_sync_pull,
            commands::list_github_repos,
            commands::github_create_repo,
            // Phase 27: Steering Files
            commands::get_steering_files,
            commands::save_steering_file,
            commands::delete_steering_file,
            // Phase 28: Auto-Memories
            commands::get_auto_memories,
            commands::delete_auto_memory,
            commands::pin_auto_memory,
            commands::add_auto_memory,
            // Phase 28: BugBot
            commands::run_bugbot,
            // Phase 29: Agent Browser Actions
            commands::agent_browser_action,
            // Phase 41: Red Team
            commands::start_redteam_scan,
            commands::get_redteam_sessions,
            commands::get_redteam_findings,
            commands::generate_redteam_report,
            commands::cancel_redteam_scan,
            // Phase 43: CRDT Collab
            commands::create_collab_session,
            commands::join_collab_session,
            commands::leave_collab_session,
            commands::list_collab_peers,
            commands::get_collab_status,
            // Phase 44: Code Coverage
            commands::detect_coverage_tool,
            commands::run_coverage,
            // Phase 44: Multi-Model Comparison
            commands::compare_models,
            // Phase 44: HTTP Playground
            commands::send_http_request,
            commands::discover_api_endpoints,
            // Phase 44b: Arena Mode
            commands::save_arena_vote,
            commands::get_arena_history,
            // Phase VI: Project Dashboard
            commands::get_project_dashboard,
            // Phase 45: Cost Observatory
            commands::record_cost_entry,
            commands::get_cost_metrics,
            commands::set_cost_limit,
            commands::clear_cost_history,
            // Phase 45: AI Git Workflow
            commands::suggest_branch_name,
            commands::resolve_merge_conflict,
            commands::generate_changelog,
            // Phase 45: Codemod & Auto-Fix
            commands::run_autofix,
            commands::apply_autofix,
            // Phase 7.19: Process Manager
            commands::list_processes,
            commands::kill_process,
            // Phase 7.21: Chat Streaming
            commands::stream_chat_message,
            commands::stop_chat_stream,
            // Phase 7.22: CI/CD & Kubernetes Deployment Hub
            commands::detect_build_type,
            commands::generate_cicd_config,
            commands::generate_release_workflow,
            commands::list_k8s_contexts,
            commands::generate_k8s_manifests,
            commands::run_kubectl_command,
            commands::generate_argocd_app,
            // Phase 7.22b: Extended Kubernetes & DevOps Commands
            commands::list_k8s_namespaces,
            commands::get_cluster_info,
            commands::run_helm_command,
            commands::run_argocd_command,
            commands::generate_argo_workflow,
            commands::generate_argo_rollout,
            commands::generate_argo_event_source,
            commands::generate_argo_sensor,
            commands::generate_applicationset,
            commands::generate_pipeline,
            // Phase 7.22b: Extended K8s Commands (continued)
            commands::scale_k8s_deployment,
            commands::get_k8s_events,
            commands::get_k8s_resource_yaml,
            commands::restart_k8s_deployment,
            commands::get_k8s_pod_logs,
            commands::get_k8s_services,
            commands::get_k8s_ingresses,
            commands::describe_k8s_resource,
            commands::get_k8s_configmaps,
            commands::get_k8s_secrets,
            // Phase 7.23: Environment & Secrets Manager
            commands::get_env_files,
            commands::read_env_file,
            commands::save_env_file,
            commands::delete_env_var,
            commands::get_env_environments,
            commands::set_active_environment,
            // Phase 7.24: Performance Profiler
            commands::detect_profiler_tool,
            commands::run_profiler,
            // Phase 7.25: Docker & Container Management
            commands::list_docker_containers,
            commands::docker_container_action,
            commands::list_docker_images,
            commands::docker_compose_action,
            commands::docker_pull_image,
            // Phase 7.25: Dependency Manager
            commands::detect_package_manager,
            commands::scan_dependencies,
            commands::upgrade_dependency,
            // Phase 7.27: Log Viewer & Analyzer
            commands::discover_log_sources,
            commands::tail_log_file,
            commands::analyze_logs,
            // Phase 7.28: Script Runner & Task Manager
            commands::detect_project_scripts,
            commands::run_project_script,
            // Phase 7.28b: Notebook / Scratchpad
            commands::execute_notebook_cell,
            commands::ai_notebook_assist,
            // Phase 7.29: SSH Remote Manager
            commands::list_ssh_profiles,
            commands::save_ssh_profile,
            commands::delete_ssh_profile,
            commands::run_ssh_command,
            // Phase 7.30: Bookmark & TODO Manager
            commands::scan_code_markers,
            commands::add_bookmark,
            commands::remove_bookmark,
            commands::get_bookmarks,
            // Phase 7.30: Git Bisect Assistant
            commands::git_bisect_start,
            commands::git_bisect_step,
            commands::git_bisect_reset,
            commands::git_bisect_log,
            commands::ai_bisect_analyze,
            // Phase 7.30: Snippet Library
            commands::list_snippets,
            commands::get_snippet,
            commands::save_snippet,
            commands::delete_snippet,
            commands::generate_snippet,
            // Phase 7.30: API Mock Server
            commands::start_mock_server,
            commands::stop_mock_server,
            commands::add_mock_route,
            commands::remove_mock_route,
            commands::list_mock_routes,
            commands::get_mock_request_log,
            commands::generate_mocks_from_spec,
            // Phase 7.31: GraphQL Playground
            commands::run_graphql_query,
            commands::introspect_graphql_schema,
            // Phase 7.32: Code Metrics + Load Tester
            commands::analyze_code_metrics,
            commands::run_load_test,
            // Phase 7.33: Network Tools
            commands::scan_open_ports,
            commands::dns_lookup,
            commands::check_tls_cert,
            // Phase 8.1: Agent Teams
            commands::start_agent_team,
            commands::get_team_status,
            commands::send_team_message,
            commands::dismiss_team,
            commands::get_team_history,
            // Phase 8.2: CI/CD Review Bot
            commands::get_ci_review_config,
            commands::save_ci_review_config,
            commands::get_ci_review_history,
            // Phase 8.4: Plugin Marketplace
            commands::get_marketplace_plugins,
            commands::search_marketplace,
            commands::install_marketplace_plugin,
            commands::list_installed_plugins,
            // Phase 8.5: Code Transform
            commands::detect_transform,
            commands::plan_transform,
            commands::execute_transform,
            // Phase 8.13: Local Edit Model
            commands::configure_local_edit_model,
            commands::get_local_edit_config,
            // Phase 8.14: Screenshot to App
            commands::generate_app_from_image,
            // Phase 8.11: Computer Use / Visual Self-Testing
            commands::take_screenshot,
            commands::get_visual_test_results,
            // Phase 8.17: Cloud-Isolated Agent Execution (Docker)
            commands::start_cloud_agent,
            commands::get_cloud_agent_status,
            // Container Sandbox Management (Docker/Podman/OpenSandbox)
            commands::detect_sandbox_runtime,
            commands::create_sandbox,
            commands::stop_sandbox,
            commands::list_sandboxes,
            commands::sandbox_exec,
            commands::get_sandbox_logs,
            commands::pause_sandbox,
            commands::resume_sandbox,
            commands::get_sandbox_metrics,
            // Phase 8.18: Compliance Reporting
            commands::generate_compliance_report,
            // Phase 7.34: Project Scaffolding
            commands::list_scaffold_templates,
            commands::generate_scaffold,
            // Phase 7.35: Service Health Monitor
            commands::get_health_monitors,
            commands::save_health_monitors,
            commands::check_service_health,
            commands::check_all_services,
            // Phase 7.36: WebSocket Tester
            commands::get_ws_configs,
            commands::save_ws_configs,
            // Phase 7.37: Color Palette & Design Token Manager
            commands::get_color_palettes,
            commands::save_color_palettes,
            commands::scan_css_variables,
            commands::export_color_palette,
            // Phase 7.38: Markdown Editor
            commands::list_markdown_files,
            // Canvas / A2UI Visual Workspace
            commands::save_canvas_workflow,
            commands::load_canvas_workflow,
            commands::list_canvas_workflows,
            commands::run_canvas_workflow,
            // Voice & Media
            commands::transcribe_audio,
            commands::transcribe_audio_bytes,
            commands::text_to_speech,
            // Gap Closure: Webhook Automations
            commands::get_webhooks,
            commands::save_webhook,
            commands::delete_webhook,
            commands::test_webhook,
            commands::get_webhook_logs,
            commands::replay_webhook,
            // Gap Closure: Enterprise Admin (RBAC, Audit, Team)
            commands::get_team_members,
            commands::save_team_member,
            commands::remove_team_member,
            commands::get_audit_log,
            commands::get_rbac_policies,
            commands::save_rbac_policy,
            commands::delete_rbac_policy,
            // Gap Closure: Chrome DevTools Protocol
            commands::cdp_capture_page,
            commands::cdp_get_version,
            commands::cdp_list_targets,
            commands::cdp_open_tab,
            commands::cdp_screenshot,
            // Feature Demo System
            commands::demo_list,
            commands::demo_get,
            commands::demo_run,
            commands::demo_generate_steps,
            commands::demo_export,
            commands::soul_scan,
            commands::soul_generate,
            commands::soul_regenerate,
            commands::soul_read,
            commands::soul_save,
            commands::generate_vibeui_rules,
            // Phase 10-14: Futureproofing
            commands::mcp_lazy_status,
            commands::mcp_lazy_list_tools,
            commands::mcp_lazy_search,
            commands::mcp_lazy_load_tool,
            commands::mcp_lazy_unload_tool,
            commands::mcp_lazy_metrics,
            commands::context_bundle_list,
            commands::context_bundle_create,
            commands::context_bundle_delete,
            commands::context_bundle_activate,
            commands::context_bundle_export,
            commands::context_bundle_import,
            commands::cloud_provider_scan,
            commands::cloud_provider_iam,
            commands::cloud_provider_iac,
            commands::cloud_provider_cost,
            commands::acp_server_status,
            commands::get_acp_status,
            commands::get_acp_capabilities,
            commands::get_acp_messages,
            commands::register_acp_capability,
            commands::send_acp_message,
            commands::toggle_acp_server,
            commands::mcp_directory_search,
            commands::mcp_directory_installed,
            commands::list_mcp_plugins,
            commands::search_mcp_plugins,
            commands::install_mcp_plugin,
            commands::uninstall_mcp_plugin,
            commands::usage_metering_status,
            commands::get_usage_kpis,
            commands::get_usage_budgets,
            commands::get_usage_by_provider,
            commands::get_usage_by_model,
            commands::get_usage_alerts,
            commands::create_usage_budget,
            commands::dismiss_usage_alert,
            commands::swe_bench_list_runs,
            commands::swe_bench_get_suites,
            commands::swe_bench_start_run,
            commands::swe_bench_get_results,
            commands::get_session_memory_health,
            commands::get_session_memory_samples,
            commands::get_session_memory_alerts,
            commands::run_session_memory_compact,
            commands::dismiss_session_memory_alert,
            // Blue Team — Defensive Security
            commands::get_blue_team_incidents,
            commands::create_blue_team_incident,
            commands::get_blue_team_iocs,
            commands::add_blue_team_ioc,
            commands::get_blue_team_rules,
            commands::create_blue_team_rule,
            commands::toggle_blue_team_rule,
            commands::get_blue_team_siem_connections,
            commands::add_blue_team_siem,
            commands::get_blue_team_playbooks,
            commands::get_blue_team_hunts,
            commands::create_blue_team_hunt,
            commands::generate_blue_team_report,
            // Purple Team — ATT&CK Exercises
            commands::list_purple_team_exercises,
            commands::create_purple_team_exercise,
            commands::purple_team_ai_generate_exercise,
            commands::get_purple_team_matrix,
            commands::record_purple_team_simulation,
            commands::get_purple_team_simulations,
            commands::get_purple_team_gaps,
            commands::generate_purple_team_report,
            // IDP — Internal Developer Platform
            commands::get_idp_catalog,
            commands::register_idp_service,
            commands::delete_idp_service,
            commands::get_idp_scorecards,
            commands::evaluate_idp_scorecard,
            commands::get_idp_golden_paths,
            commands::get_idp_platforms,
            commands::toggle_idp_platform,
            commands::generate_backstage_catalog,
            commands::get_idp_teams,
            commands::create_idp_team,
            commands::toggle_idp_checklist,
            commands::request_idp_infra,
            commands::get_idp_infra_requests,
            commands::fullstack_generate,
            commands::fullstack_read_file,
            commands::fullstack_write_file,
            // Clarifying Questions
            commands::get_clarify_questions,
            commands::get_clarify_plan,
            commands::get_clarify_risks,
            commands::save_clarify_questions,
            commands::save_clarify_plan,
            // Orchestration
            commands::get_orch_state,
            commands::save_orch_state,
            commands::get_orch_lessons,
            commands::save_orch_lessons,
            // AI/ML Workflow
            commands::get_aiml_pipeline_config,
            commands::save_aiml_pipeline_config,
            // Next-Task ML
            commands::get_nexttask_predictions,
            commands::get_nexttask_history,
            commands::get_nexttask_transitions,
            commands::get_nexttask_rules,
            commands::accept_nexttask,
            commands::toggle_nexttask_rule,
            // QA Validation
            commands::run_qa_validation,
            commands::get_qa_history,
            // Vector DB
            commands::list_vector_collections,
            commands::create_vector_collection,
            commands::delete_vector_collection,
            commands::vector_search,
            // Org Context
            commands::get_org_repos,
            commands::get_org_patterns,
            commands::get_org_conventions,
            commands::get_org_dependencies,
            commands::save_org_repo,
            // Spec Pipeline
            commands::get_spec_requirements,
            commands::get_spec_designs,
            commands::get_spec_tasks,
            commands::save_spec_requirement,
            commands::save_spec_design,
            commands::save_spec_task_item,
            // VM Orchestrator
            commands::get_vm_environments,
            commands::get_vm_pull_requests,
            commands::get_vm_conflicts,
            commands::get_vm_config,
            commands::save_vm_config,
            // Session Sharing
            commands::get_shared_sessions,
            commands::get_session_annotations,
            commands::share_session,
            commands::add_session_annotation,
            // Self-Review
            commands::get_selfreview_iterations,
            commands::get_selfreview_config,
            commands::save_selfreview_config,
            // Streaming
            commands::get_streaming_topics,
            commands::save_streaming_topic,
            commands::delete_streaming_topic,
            // Observe-Act
            commands::get_observeact_steps,
            commands::get_observeact_config,
            commands::save_observeact_config,
            // Web Crawler
            commands::run_web_crawl,
            commands::get_crawl_results,
            commands::parse_sitemap,
            commands::check_robots_txt,
            // Visual Verify
            commands::get_visual_baselines,
            commands::save_visual_baseline,
            commands::get_visual_diffs,
            commands::delete_visual_baseline,
            // Automations
            commands::get_automation_rules,
            commands::get_automation_tasks,
            commands::get_automation_stats,
            commands::get_automation_logs,
            commands::create_automation_rule,
            commands::update_automation_rule,
            commands::delete_automation_rule,
            commands::toggle_automation_rule,
            // Resilience
            commands::get_provider_health,
            commands::get_circuit_breaker_state,
            commands::get_failure_records,
            commands::get_failure_patterns,
            commands::get_resilience_config,
            commands::save_resilience_config,
            commands::record_failure,
            // Security Scan
            commands::run_security_scan,
            commands::get_security_scan_results,
            commands::get_security_scan_history,
            commands::suppress_security_finding,
            commands::suppress_security_cwe,
            commands::get_security_suppressions,
            // Agile Project Management
            commands::agile_get_board,
            commands::agile_update_card,
            commands::agile_move_card,
            commands::agile_delete_card,
            commands::agile_get_sprints,
            commands::agile_create_sprint,
            commands::agile_update_sprint,
            commands::agile_get_backlog,
            commands::agile_create_story,
            commands::agile_update_story,
            commands::agile_delete_story,
            commands::agile_get_ceremonies,
            commands::agile_save_ceremony,
            commands::agile_get_metrics,
            commands::agile_ai_analyze,
            commands::agile_update_wip_limits,
            commands::agile_get_safe,
            commands::agile_save_safe,
            commands::agile_ai_split_story,
            commands::agile_ai_generate_subtasks,
            commands::agile_ai_generate_ac,
            commands::agile_ai_estimate_points,
            commands::agile_ai_retro_generate,
            commands::agile_ai_generate_backlog,
            // Work Management
            commands::wm_get_config,
            commands::wm_save_config,
            commands::wm_list_orgs,
            commands::wm_save_org,
            commands::wm_delete_org,
            commands::wm_list_groups,
            commands::wm_save_group,
            commands::wm_delete_group,
            commands::wm_list_teams,
            commands::wm_save_team,
            commands::wm_delete_team,
            commands::wm_list_workspaces,
            commands::wm_save_workspace,
            commands::wm_delete_workspace,
            commands::wm_create_item,
            commands::wm_update_item,
            commands::wm_delete_item,
            commands::wm_list_items,
            commands::wm_get_item,
            commands::wm_move_item,
            commands::wm_add_relationship,
            commands::wm_remove_relationship,
            commands::wm_get_item_tree,
            commands::wm_get_dashboard,
            commands::wm_ai_generate_item,
            commands::wm_ai_suggest_breakdown,
            commands::wm_ai_assess_risk,
            // Quantum Computing
            commands::quantum_get_languages,
            commands::quantum_get_os_list,
            commands::quantum_get_algorithms,
            commands::quantum_get_hardware_types,
            commands::quantum_get_hello_circuit,
            commands::quantum_get_compatibility,
            commands::quantum_get_projects,
            commands::quantum_create_project,
            commands::quantum_delete_project,
            commands::quantum_get_circuits,
            commands::quantum_create_circuit,
            commands::quantum_export_circuit,
            commands::quantum_add_gate,
            commands::quantum_remove_gate,
            commands::quantum_get_circuit_detail,
            commands::quantum_simulate_circuit,
            commands::quantum_optimize_circuit,
            commands::quantum_estimate_cost,
            commands::quantum_get_algorithm_template,
            commands::quantum_list_templates,
            commands::quantum_scaffold_project,
            commands::quantum_delete_circuit,
            commands::quantum_clear_circuit_gates,
            // Counsel
            commands::counsel_create_session,
            commands::counsel_list_sessions,
            commands::counsel_get_session,
            commands::counsel_run_round,
            commands::counsel_synthesize,
            commands::counsel_inject_message,
            commands::counsel_vote,
            commands::counsel_delete_session,
            commands::counsel_update_participant,
            // SuperBrain
            commands::superbrain_route,
            commands::superbrain_query,
            commands::superbrain_get_modes,
            // Batch Builder
            commands::batch_create_run,
            commands::batch_list_runs,
            commands::batch_get_run,
            commands::batch_update_run,
            commands::batch_delete_run,
            commands::batch_simulate_progress,
            commands::batch_save_findings,
            commands::batch_save_migration,
            // Cloud OAuth
            commands::cloud_oauth_initiate,
            commands::cloud_oauth_complete,
            commands::cloud_oauth_status,
            commands::cloud_oauth_list_connected,
            commands::cloud_oauth_disconnect,
            commands::cloud_oauth_refresh,
            commands::cloud_oauth_get_token,
            commands::cloud_oauth_google_profile,
            commands::cloud_oauth_save_client_config,
            commands::cloud_oauth_get_client_config,
            // Panel Settings Store
            commands::panel_settings_get,
            commands::panel_settings_get_all,
            commands::panel_settings_set,
            commands::panel_settings_delete,
            commands::panel_settings_delete_panel,
            commands::panel_settings_list_profiles,
            commands::panel_settings_create_profile,
            commands::panel_settings_delete_profile,
            commands::panel_settings_set_default_profile,
            commands::panel_settings_get_default_profile,
            commands::panel_settings_export,
            commands::panel_settings_import,
            // Profile Store — API Keys, Provider Configs, Global Settings
            commands::profile_api_key_set,
            commands::profile_api_key_get,
            commands::profile_api_key_list,
            commands::profile_api_key_delete,
            commands::integration_tokens_get,
            commands::integration_token_set,
            commands::integration_token_delete,
            commands::get_watch_pairing_info,
            commands::list_watch_devices,
            commands::revoke_watch_device,
            commands::watch_get_session_messages,
            commands::watch_get_active_session,
            commands::watch_get_sandbox_chat_session,
            commands::watch_set_sandbox_chat_session,
            commands::recap_get_for_session,
            commands::recap_resume_session,
            commands::recap_generate,
            commands::profile_provider_config_set,
            commands::profile_provider_config_get,
            commands::profile_provider_config_get_all,
            commands::profile_global_set,
            commands::profile_global_get,
            commands::profile_global_get_all,
            commands::profile_global_delete,
            // Workspace Store — Per-Project Encrypted Settings & Secrets
            commands::workspace_setting_get,
            commands::workspace_setting_set,
            commands::workspace_setting_delete,
            commands::workspace_setting_list,
            commands::workspace_secret_set,
            commands::workspace_secret_get,
            commands::workspace_secret_delete,
            commands::workspace_secret_list,
            // Sub-Agent Management
            commands::list_sub_agents,
            commands::spawn_sub_agent,
            commands::dismiss_sub_agent,
            commands::clear_completed_sub_agents,
            // CI Status Checks
            commands::get_ci_status,
            commands::get_ci_checks,
            commands::get_ci_config,
            commands::trigger_ci_check,
            // Edit Prediction
            commands::get_edit_predictions,
            commands::get_edit_patterns,
            commands::get_edit_model_stats,
            commands::accept_prediction,
            commands::dismiss_prediction,
            // Plan Document Commands
            commands::list_plan_documents,
            commands::get_plan_document,
            commands::create_plan_document,
            commands::update_plan_status,
            commands::add_plan_comment,
            commands::resolve_plan_comment,
            // Remote Control
            commands::get_remote_control_status,
            commands::list_remote_clients,
            commands::get_remote_events,
            commands::start_remote_server,
            commands::stop_remote_server,
            commands::disconnect_remote_client,
            // Cloud Sandbox Management
            commands::list_cloud_sandboxes,
            commands::get_cloud_sandbox_templates,
            commands::create_cloud_sandbox,
            commands::stop_cloud_sandbox,
            commands::delete_cloud_sandbox,
            commands::get_cloud_sandbox_logs,
            // Knowledge Graph
            commands::get_knowledge_graph,
            commands::get_knowledge_graph_stats,
            commands::search_knowledge_graph,
            commands::refresh_knowledge_graph,
            // Adventure tab names
            commands::get_adventure_names,
            commands::add_adventure_name,
            commands::remove_adventure_name,
            // Agent Modes
            commands::get_agent_modes,
            commands::get_agent_mode_stats,
            commands::set_active_agent_mode,
            commands::get_agent_mode_profiles,
            commands::create_agent_mode_profile,
            // Debug Mode
            commands::list_debug_sessions,
            commands::create_debug_session,
            commands::add_debug_breakpoint,
            commands::remove_debug_breakpoint,
            commands::run_debug_analysis,
            commands::delete_debug_session,
            // Discussion Mode
            commands::list_discussion_threads,
            commands::create_discussion_thread,
            commands::add_discussion_message,
            commands::get_discussion_thread,
            commands::delete_discussion_thread,
            // Render Optimization
            commands::get_render_stats,
            commands::get_dirty_regions,
            commands::run_render_optimization,
            commands::reset_render_stats,
            // Image Generation
            commands::list_generated_images,
            commands::generate_image,
            commands::delete_generated_image,
            commands::get_image_gen_stats,
            commands::get_generated_image_data,
            commands::get_available_image_providers,
            // Conversational Search
            commands::conversational_search,
            commands::get_search_history,
            commands::get_search_suggestions,
            commands::clear_search_history,
            // Fast Context / SWE-grep
            commands::fast_context_search,
            commands::fast_context_index_stats,
            commands::fast_context_cache_stats,
            commands::fast_context_reindex,
            // Fine-Tuning Panel
            commands::get_fine_tuning_stats,
            commands::list_fine_tuning_jobs,
            commands::create_fine_tuning_job,
            commands::list_fine_tuning_evals,
            commands::list_fine_tuning_adapters,
            commands::create_fine_tuning_adapter,
            // GPU Terminal Panel
            commands::get_gpu_terminal_stats,
            commands::run_gpu_terminal_benchmark,
            commands::get_gpu_fps_history,
            commands::get_gpu_glyph_atlas,
            // AST Edit Panel
            commands::get_ast_files,
            commands::get_ast_edits,
            commands::create_ast_edit,
            commands::apply_ast_edit,
            commands::dismiss_ast_edit,
            // Infinite Context Manager
            commands::get_context_chunks,
            commands::get_project_file_tree,
            commands::get_context_window_stats,
            commands::evict_context_chunk,
            commands::pin_context_chunk,
            // App Builder
            commands::get_app_templates,
            commands::create_app_project,
            commands::get_app_builder_history,
            commands::enhance_app_template,
            // Cloud Autofix
            commands::list_autofix_attempts,
            commands::get_autofix_stats,
            commands::create_autofix_attempt,
            commands::update_autofix_status,
            commands::get_autofix_config,
            commands::save_autofix_config,
            // Team Governance
            commands::list_governance_plugins,
            commands::submit_plugin_for_approval,
            commands::approve_plugin,
            commands::reject_plugin,
            commands::get_governance_audit_log,
            commands::get_governance_policies,
            // GitHub Actions Panel
            commands::list_gh_workflow_templates,
            commands::generate_gh_workflow,
            commands::list_gh_secrets,
            commands::save_gh_workflow,
            commands::get_gh_actions_history,
            // VibeSQL Server
            commands::vibesql_list_connections,
            commands::vibesql_save_connection,
            commands::vibesql_delete_connection,
            commands::vibesql_connect,
            commands::vibesql_list_tables,
            commands::vibesql_execute_query,
            commands::vibesql_server_info,
            commands::vibesql_generate_sql,
            // AutoResearch — Autonomous Iterative Research Agent
            commands::autoresearch_list_sessions,
            commands::autoresearch_get_session,
            commands::autoresearch_create_session,
            commands::autoresearch_record_experiment,
            commands::autoresearch_get_memory,
            commands::autoresearch_save_lesson,
            commands::autoresearch_export_tsv,
            commands::autoresearch_delete_session,
            // OpenMemory — Cognitive Memory Engine
            commands::openmemory_stats,
            commands::openmemory_index_stats,
            commands::openmemory_list,
            commands::openmemory_add,
            commands::openmemory_delete,
            commands::openmemory_pin,
            commands::openmemory_unpin,
            commands::openmemory_query,
            commands::openmemory_facts,
            commands::openmemory_add_fact,
            commands::openmemory_run_decay,
            commands::openmemory_consolidate,
            commands::openmemory_export,
            commands::openmemory_enable_encryption,
            commands::openmemory_layered_context,
            commands::openmemory_drawer_stats,
            commands::openmemory_benchmark,
            commands::memory_projections_refresh,
            // FIT-GAP v10 — Phase 40-43 commands
            commands::parallel_tool_scheduler_status,
            commands::context_budget_status,
            commands::smart_diff_render,
            commands::agent_state_get,
            commands::file_watcher_status,
            commands::cost_estimate,
            commands::rate_limit_status,
            commands::test_impact_analyse,
            // FIT-GAP v10 — Phase 44 (P3 gaps closed)
            commands::ai_merge_resolve,
            commands::symbol_rename_preview,
            commands::code_templates_list,
            commands::code_template_render,
            commands::cache_advisory_analyze,
            commands::voice_history_record,
            commands::plugin_marketplace_list,
            commands::plugin_marketplace_install,
            // FIT-GAP v11 — Phase 45-47 commands
            commands::agent_registry_list,
            commands::agent_registry_register,
            commands::agent_quota_usage,
            commands::agent_autoscale_status,
            commands::agent_checkpoint_save,
            commands::workspace_snapshot_capture,
            commands::inline_diff_hunks,
            commands::changelog_generate,
            commands::pr_description_generate,
            commands::dep_update_analyze,
            // FIT-GAP v11 — Phase 48: P3 Commands
            commands::token_dashboard_stats,
            commands::session_export,
            commands::capability_discover,
            commands::explain_code_depth,
            commands::perf_regression_check,
            commands::prompt_vcs_commit,
            commands::repl_macro_expand,
            commands::semantic_search_v2,
            // MCP Plugin Tools
            commands::get_mcp_plugin_tools,
            // Vulnerability Scanner
            commands::vulnscan_scan_deps,
            commands::vulnscan_scan_file,
            commands::vulnscan_status,
            // SpawnAgent — Parallel Agent Spawning & Lifecycle Management
            commands::spawn_agent_new,
            commands::spawn_agent_list,
            commands::spawn_agent_stats,
            commands::spawn_agent_pause,
            commands::spawn_agent_resume,
            commands::spawn_agent_cancel,
            commands::spawn_agent_decompose,
            commands::spawn_agent_aggregate,
            // Agent Recordings
            commands::list_agent_recordings,
            // Mobile Gateway — machine registration, pairing, dispatch
            commands::dispatch_list_machines,
            commands::dispatch_register_machine,
            commands::dispatch_unregister_machine,
            commands::dispatch_create_pairing,
            commands::dispatch_accept_pairing,
            commands::dispatch_list_devices,
            commands::dispatch_send,
            commands::dispatch_cancel,
            commands::dispatch_stats,
            commands::dispatch_heartbeat,
            // FIT-GAP v7 commands
            commands::a2a_list_agents,
            commands::a2a_discover,
            commands::a2a_submit_task,
            commands::a2a_get_metrics,
            commands::a2a_get_agent_card,
            commands::a2a_update_agent_card,
            commands::a2a_list_tasks,
            commands::a2a_cancel_task,
            commands::skills_list,
            commands::skills_import,
            commands::skills_validate,
            commands::worktree_list,
            commands::worktree_spawn,
            commands::worktree_merge,
            commands::worktree_cleanup,
            commands::host_list_agents,
            commands::host_register,
            commands::host_start,
            commands::host_stop,
            commands::host_get_output,
            commands::host_get_clipboard,
            commands::host_set_clipboard,
            commands::host_clear_clipboard,
            commands::proactive_scan,
            commands::proactive_get_suggestions,
            commands::proactive_accept,
            commands::proactive_reject,
            commands::proactive_get_digest,
            commands::triage_issue,
            commands::triage_get_rules,
            commands::triage_get_history,
            commands::triage_get_metrics,
            commands::web_search,
            commands::web_get_citations,
            commands::web_cache_stats,
            commands::web_clear_cache,
            commands::semindex_build,
            commands::semindex_search,
            commands::semindex_callers,
            commands::semindex_callees,
            commands::semindex_stats,
            commands::mcp_http_status,
            commands::mcp_http_connections,
            commands::repair_list_sessions,
            commands::repair_new_session,
            commands::repair_get_tree,
            commands::repair_compare,
            commands::route_list_models,
            commands::route_get_decisions,
            commands::route_get_budget,
            commands::route_ab_experiments,
            commands::vverify_capture,
            commands::vverify_list_baselines,
            commands::vverify_compare,
            commands::nexttask_suggest,
            commands::nexttask_accept,
            commands::nexttask_reject,
            commands::nexttask_accuracy,
            commands::docsync_status,
            commands::docsync_reconcile,
            commands::docsync_get_alerts,
            commands::docsync_get_links,
            commands::docsync_get_sections,
            commands::voice_list_models,
            commands::voice_start_recording,
            commands::voice_stop_recording,
            commands::connectors_list,
            commands::connectors_available,
            commands::connectors_add,
            commands::connectors_test,
            commands::connectors_discover,
            commands::analytics_dashboard,
            commands::analytics_users,
            commands::analytics_teams,
            commands::analytics_export,
            commands::trust_get_scores,
            commands::trust_get_events,
            commands::trust_explain,
            commands::smartdeps_analyze,
            commands::smartdeps_check_security,
            commands::smartdeps_check_licenses,
            commands::rlcef_get_outcomes,
            commands::rlcef_get_mistakes,
            commands::rlcef_get_strategies,
            commands::rlcef_export,
            commands::langgraph_list_pipelines,
            commands::langgraph_create_pipeline,
            commands::langgraph_get_checkpoints,
            commands::langgraph_get_events,
            commands::sketch_recognize,
            commands::sketch_generate,
            commands::sketch_export,
            // Data Analysis
            commands::da_list_datasets,
            commands::da_add_dataset,
            commands::da_remove_dataset,
            commands::da_list_charts,
            commands::da_add_chart,
            commands::da_list_widgets,
            commands::da_add_widget,
            commands::da_execute_query,
            commands::da_list_queries,
            // Branch Agent
            commands::list_branch_agents,
            commands::get_branch_prs,
            commands::get_branch_conflicts,
            // Audio Output
            commands::list_narrations,
            commands::create_narration,
            // Channel Daemon
            commands::list_daemon_channels,
            commands::get_channel_messages,
            // CI Gates
            commands::list_ci_gates,
            commands::toggle_ci_gate,
            // Design Import
            commands::list_design_imports,
            commands::create_design_import,
            commands::desktop_list_windows,
            commands::desktop_run_action,
            commands::datagen_list_schemas,
            commands::datagen_save_schema,
            commands::wizard_list_configs,
            commands::wizard_save_config,
            commands::training_list_jobs,
            commands::training_create_job,
            // Cost Router
            commands::cost_router_list_models,
            commands::cost_router_get_budget,
            // MCTS Repair
            commands::mcts_list_sessions,
            commands::mcts_create_session,
            commands::mcts_get_tree,
            // Browser Agent
            commands::browser_list_sessions,
            commands::browser_create_session,
            commands::browser_close_session,
            // TurboQuant
            commands::turboquant_stats,
            commands::turboquant_insert,
            commands::turboquant_search,
            commands::turboquant_benchmark,
            commands::turboquant_clear,
            // Health Score
            commands::healthscore_scan,
            commands::healthscore_remediate,
            // Intent Refactoring
            commands::refactor_parse_intent,
            commands::refactor_plan,
            commands::refactor_suggest,
            // Collaborative Review
            commands::creview_start,
            commands::creview_stats,
            // Skill Distillation
            commands::distill_status,
            commands::distill_patterns,
            commands::distill_export,
            // Self-Improving Skills
            commands::sis_record_activation,
            commands::sis_record_outcome,
            commands::sis_record_session_outcome,
            commands::sis_get_metrics,
            commands::sis_propose_evolutions,
            commands::sis_apply_evolution,
            commands::sis_extract_new_skill,
            commands::sis_prune_candidates,
            commands::sis_status,
            // RL-OS
            commands::rl_create_training_run,
            commands::rl_list_training_runs,
            commands::rl_get_training_metrics,
            commands::rl_start_training,
            commands::rl_stop_training,
            commands::rl_list_environments,
            commands::rl_get_environment,
            commands::rl_deploy_environment,
            commands::rl_register_custom_environment,
            commands::rl_refresh_environments,
            commands::rl_delete_environment,
            commands::rl_list_policies,
            commands::rl_register_policy,
            commands::rl_delete_policy,
            commands::rl_get_policy_card,
            commands::rl_compare_policies,
            commands::rl_list_eval_suites,
            commands::rl_create_eval_suite,
            commands::rl_delete_eval_suite,
            commands::rl_get_eval_results,
            commands::rl_get_optimization_report,
            commands::rl_run_optimization,
            commands::rl_list_deployments,
            commands::rl_create_deployment,
            commands::rl_promote_deployment,
            commands::rl_rollback_deployment,
            commands::rl_stop_deployment,
            commands::rl_get_deployment_health,
            commands::rl_get_model_lineage,
            commands::rl_get_multi_agent_metrics,
            commands::rl_get_reward_decomposition,
            commands::rl_get_alignment_metrics,
            commands::rl_create_preference,
            commands::rl_judge_preference,
            commands::rl_list_preferences,
            // Productivity integrations
            commands::handle_email_command,
            commands::handle_calendar_command,
            commands::handle_productivity_command,
            commands::handle_ha_command,
            // Typed email commands (in-process, for interactive UI)
            commands::productivity_email_list,
            commands::productivity_email_read,
            commands::productivity_email_archive,
            commands::productivity_email_mark_read,
            commands::productivity_email_labels,
            commands::productivity_email_send,
            commands::productivity_cal_create,
            commands::productivity_email_status,
            // Typed calendar commands
            commands::productivity_cal_today,
            commands::productivity_cal_week,
            commands::productivity_cal_upcoming,
            commands::productivity_cal_range,
            commands::productivity_cal_delete,
            commands::productivity_cal_move,
            commands::productivity_cal_next,
            commands::productivity_cal_free_today,
            commands::productivity_cal_status,
            // Typed tasks / notion / jira commands
            commands::productivity_tasks_list,
            commands::productivity_tasks_add,
            commands::productivity_tasks_add_full,
            commands::productivity_tasks_close,
            commands::productivity_tasks_status,
            commands::productivity_notion_search,
            commands::productivity_notion_page,
            commands::productivity_notion_append,
            commands::productivity_notion_status,
            commands::productivity_jira_list,
            commands::productivity_jira_create,
            commands::productivity_jira_comment,
            commands::productivity_jira_status,
            commands::productivity_home_list,
            commands::productivity_home_call_service,
            commands::productivity_home_status,
            commands::productivity_plan_my_day,
            commands::productivity_draft_reply,
            commands::handle_archspec_command,
            commands::archspec_load,
            commands::archspec_set_artifact_status,
            commands::archspec_generate,
            commands::archspec_set_adr_status,
            commands::archspec_create_adr,
            commands::archspec_save,
            commands::archspec_list_scans,
            commands::archspec_load_scan,
            commands::archspec_clear_history,
            commands::handle_policy_command,
            commands::sonar_load_rules,
            commands::sonar_get_rules,
            commands::sonar_scan_file,
            commands::handle_aireview_command,
            commands::handle_creview_command,
            commands::handle_gateway_cli,
            // Company Orchestration (Paperclip parity)
            commands::company_cmd,
            commands::company_create,
            commands::company_list,
            commands::company_status,
            commands::company_switch,
            commands::company_delete,
            commands::company_agent_hire,
            commands::company_agent_list,
            commands::company_agent_info,
            commands::company_agent_fire,
            commands::company_budget_set,
            commands::company_budget_status,
            commands::company_budget_events,
            commands::company_approval_request,
            commands::company_approval_list,
            commands::company_approval_approve,
            commands::company_approval_reject,
            commands::company_secret_set,
            commands::company_secret_get,
            commands::company_secret_list,
            commands::company_secret_delete,
            commands::company_routine_create,
            commands::company_routine_list,
            commands::company_routine_toggle,
            commands::company_heartbeat_trigger,
            commands::company_heartbeat_history,
            commands::company_export,
            commands::company_import,
            commands::company_adapter_list,
            commands::company_dashboard,
            commands::company_workspace_config_get,
            commands::company_workspace_config_set,
            commands::company_priority_map_get,
            commands::company_priority_map_set,
            commands::company_heartbeat_history_json,
            commands::company_routine_create_v2,
            commands::company_routine_list_json,
            commands::company_routine_set_delivery_mode,
            commands::company_task_create_v2,
            commands::company_task_list_json,
            commands::company_doc_set_role,
            commands::company_doc_list_json,
            commands::company_ingest_meeting_notes,
            commands::set_automation_resolution_mode,
            commands::company_list_skills,
            // Daemon management
            commands::get_daemon_status,
            commands::start_daemon,
            commands::stop_daemon,
            // FIT-GAP v8: Phase 33-39 — Agent Intelligence panels
            commands::env_dispatch_list,
            commands::env_dispatch_task,
            commands::env_dispatch_status,
            commands::nested_agents_tree,
            commands::nested_agents_spawn,
            commands::nested_agents_cancel,
            commands::thought_stream_live,
            commands::thought_stream_history,
            commands::thought_stream_export,
            commands::hard_problem_decompose,
            commands::hard_problem_confirm_assumption,
            commands::hard_problem_questions,
            commands::repro_agent_snapshots,
            commands::repro_agent_diff,
            commands::repro_agent_verify,
            commands::on_device_list,
            commands::on_device_enforce,
            // Design platform — Draw.io, Pencil, Penpot, AI diagram generator, design system hub
            commands::parse_drawio_xml,
            commands::generate_drawio_xml,
            commands::get_drawio_template,
            commands::save_drawio_file,
            commands::execute_drawio_mcp,
            commands::parse_pencil_ep,
            commands::generate_pencil_wireframe,
            commands::execute_pencil_mcp,
            commands::connect_penpot,
            commands::list_penpot_files,
            commands::import_penpot_file,
            commands::export_penpot_component,
            commands::export_penpot_tokens,
            commands::generate_diagram,
            commands::save_diagram_file,
            commands::load_design_system_tokens,
            commands::export_design_tokens,
            commands::audit_design_system_tokens,
            // Enterprise Governance panels — MCP Governance / MSAF / Team Onboarding
            commands::mcp_audit_query,
            commands::mcp_sso_config,
            commands::mcp_sso_config_save,
            commands::mcp_gateway_rules,
            commands::mcp_config_export,
            commands::mcp_config_import,
            commands::msaf_manifest,
            commands::msaf_catalog_list,
            commands::msaf_validate_token,
            commands::team_onboarding_members,
            commands::team_onboarding_gaps,
            commands::team_onboarding_hotspots,
            commands::team_onboarding_guide,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use vibe_ai::provider::ProviderConfig;

    // ── ProviderConfig defaults used in run() ─────────────────────────────

    #[test]
    fn default_ollama_config_has_codellama_model() {
        let config = ProviderConfig {
            provider_type: "ollama".to_string(),
            api_key: None,
            model: "codellama".to_string(),
            api_url: Some("http://localhost:11434".to_string()),
            max_tokens: None,
            temperature: None,
            ..Default::default()
        };
        assert_eq!(config.model, "codellama");
        assert_eq!(config.api_url.as_deref(), Some("http://localhost:11434"));
    }

    #[test]
    fn provider_config_default_trait() {
        let config = ProviderConfig::default();
        assert!(config.api_key.is_none());
        assert!(config.max_tokens.is_none());
        assert!(config.temperature.is_none());
    }

    #[test]
    fn provider_config_with_custom_api_url() {
        let config = ProviderConfig {
            provider_type: "ollama".to_string(),
            api_url: Some("http://custom:9999".to_string()),
            model: "llama3".to_string(),
            ..Default::default()
        };
        assert_eq!(config.api_url.as_deref(), Some("http://custom:9999"));
        assert_eq!(config.model, "llama3");
    }

    #[test]
    fn provider_config_with_api_key() {
        let config = ProviderConfig {
            provider_type: "openai".to_string(),
            api_key: Some("sk-test-key".to_string()),
            model: "gpt-4".to_string(),
            ..Default::default()
        };
        assert_eq!(config.api_key.as_deref(), Some("sk-test-key"));
        assert_eq!(config.provider_type, "openai");
    }

    #[test]
    fn provider_config_temperature_and_max_tokens() {
        let config = ProviderConfig {
            provider_type: "claude".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            model: "claude-3-opus".to_string(),
            ..Default::default()
        };
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.max_tokens, Some(4096));
    }

    // ── AIConfig fallback logic ───────────────────────────────────────────

    #[test]
    fn ai_config_load_missing_file_returns_default() {
        use std::path::PathBuf;
        use vibe_ai::AIConfig;
        let config = AIConfig::load_from_file(&PathBuf::from("/nonexistent/vibe.toml"))
            .unwrap_or_default();
        // Default config should have no ollama section
        assert!(config.ollama.is_none());
    }

    #[test]
    fn ai_config_default_has_no_providers() {
        use vibe_ai::AIConfig;
        let config = AIConfig::default();
        assert!(config.ollama.is_none());
    }

    // ── FlowTracker (used in run()) ───────────────────────────────────────

    #[test]
    fn flow_tracker_new_is_empty() {
        let tracker = crate::flow::FlowTracker::new();
        assert_eq!(tracker.context_string(10), "");
    }

    #[test]
    fn flow_tracker_record_and_context() {
        let mut tracker = crate::flow::FlowTracker::new();
        tracker.record("file_open", "src/lib.rs");
        let ctx = tracker.context_string(10);
        assert!(ctx.contains("src/lib.rs"));
    }

    // ── Workspace creation ────────────────────────────────────────────────

    #[test]
    fn workspace_creation_with_name() {
        let ws = vibe_core::Workspace::new("Test Workspace".to_string());
        assert_eq!(ws.name(), "Test Workspace");
    }

    #[test]
    fn chat_engine_starts_empty() {
        let engine = vibe_ai::ChatEngine::new();
        // ChatEngine should be constructable without panic
        drop(engine);
    }
}
