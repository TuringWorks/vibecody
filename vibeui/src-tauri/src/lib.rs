//! VibeUI - AI-Powered Code Editor

mod commands;
mod flow;
mod agent_executor;
mod memory;
pub mod shadow_workspace;

use commands::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;
use vibe_core::Workspace;
use vibe_ai::{ChatEngine, providers, AIConfig};
use vibe_ai::provider::ProviderConfig;
use std::path::PathBuf;
use vibe_core::terminal::TerminalManager;
use vibe_lsp::manager::LspManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize workspace
    let workspace = Arc::new(Mutex::new(Workspace::new("VibeUI Workspace".to_string())));

    // Load AI configuration
    let config_path = PathBuf::from("vibe.toml");
    let ai_config = AIConfig::load_from_file(&config_path).unwrap_or_default();

    // Initialize Chat Engine
    let mut chat_engine = ChatEngine::new();

    // Initialize Ollama if enabled
    if let Some(ollama_conf) = ai_config.ollama {
        if ollama_conf.enabled {
            let config = ProviderConfig {
                provider_type: "ollama".to_string(),
                api_key: ollama_conf.api_key,
                model: ollama_conf.model.unwrap_or_else(|| "codellama".to_string()),
                api_url: ollama_conf.api_url.or_else(|| Some("http://localhost:11434".to_string())),
                max_tokens: ollama_conf.max_tokens,
                temperature: ollama_conf.temperature,
                ..Default::default()
            };
            let provider = providers::ollama::OllamaProvider::new(config);
            chat_engine.add_provider(Arc::new(provider));
        }
    } else {
        // Default fallback if config missing
        let config = ProviderConfig {
            provider_type: "ollama".to_string(),
            api_key: None,
            model: "codellama".to_string(),
            api_url: Some("http://localhost:11434".to_string()),
            max_tokens: None,
            temperature: None,
            ..Default::default()
        };
        let provider = providers::ollama::OllamaProvider::new(config);
        chat_engine.add_provider(Arc::new(provider));
    }

    // Additional providers (OpenAI, Claude, Gemini, etc.) are configured at
    // runtime via the BYOK settings panel and injected through ChatEngine::add_provider().

    let chat_engine = Arc::new(Mutex::new(chat_engine));
    let terminal_manager = Arc::new(TerminalManager::new());
    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));
    let flow = Arc::new(Mutex::new(flow::FlowTracker::new()));
    let agent_pending = Arc::new(Mutex::new(None));
    let terminal_buffer = Arc::new(Mutex::new(Vec::<String>::new()));
    let agent_abort_handle = Arc::new(Mutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            workspace,
            chat_engine,
            terminal_manager,
            lsp_manager,
            flow,
            agent_pending,
            terminal_buffer,
            agent_abort_handle,
        })
        .invoke_handler(tauri::generate_handler![
            commands::read_file,
            commands::write_file,
            commands::list_directory,
            commands::create_directory,
            commands::delete_item,
            commands::rename_item,
            commands::add_workspace_folder,
            commands::get_workspace_folders,
            commands::open_file_in_workspace,
            commands::insert_text,
            commands::delete_text,
            commands::save_file,
            commands::request_ai_completion,
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
            commands::request_inline_completion,
            commands::track_flow_event,
            commands::get_flow_context,
            // Phase 4 commands — Agent Mode
            commands::start_agent_task,
            commands::stop_agent_task,
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
            // Phase 7.3 commands — Next-Edit Prediction
            commands::predict_next_edit,
            commands::inline_edit,
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
            // Rules directory
            commands::list_rule_files,
            commands::get_rule_file,
            commands::save_rule_file,
            commands::delete_rule_file,
            // MCP server manager
            commands::get_mcp_servers,
            commands::save_mcp_servers,
            commands::test_mcp_server,
            // Symbol + codebase search
            commands::search_workspace_symbols,
            commands::semantic_search_codebase,
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
            commands::find_sqlite_files,
            commands::list_db_tables,
            commands::query_db,
            commands::generate_sql_query,
            commands::generate_migration,
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
            // Phase 31: Embedding index
            commands::build_embedding_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
