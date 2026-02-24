//! VibeUI - AI-Powered Code Editor

mod commands;
mod flow;
mod agent_executor;
mod memory;

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
        };
        let provider = providers::ollama::OllamaProvider::new(config);
        chat_engine.add_provider(Arc::new(provider));
    }

    // Initialize other providers (placeholders for now, logic would be similar)
    // if let Some(openai_conf) = ai_config.openai { ... }

    let chat_engine = Arc::new(Mutex::new(chat_engine));
    let terminal_manager = Arc::new(TerminalManager::new());
    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));
    let flow = Arc::new(Mutex::new(flow::FlowTracker::new()));
    let agent_pending = Arc::new(Mutex::new(None));

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
