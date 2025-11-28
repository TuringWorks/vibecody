//! Tauri commands for frontend-backend communication

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use vibe_ai::{CodeContext, Message, ChatEngine};
use vibe_core::buffer::{Position, Range, Edit, Cursor};
use vibe_core::file_system::FileEntry;

use vibe_core::Workspace;
use vibe_core::terminal::TerminalManager;
use vibe_lsp::manager::LspManager;
use lsp_types::{CompletionParams, CompletionResponse, HoverParams, Hover, GotoDefinitionParams, GotoDefinitionResponse};
use tauri::Emitter;

/// Application state
pub struct AppState {
    pub workspace: Arc<Mutex<Workspace>>,
    pub chat_engine: Arc<Mutex<ChatEngine>>,
    pub terminal_manager: Arc<TerminalManager>,
    pub lsp_manager: Arc<Mutex<LspManager>>,
}

/// File operations

#[tauri::command]
pub async fn read_file(path: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let workspace = state.workspace.lock().await;
    workspace
        .file_system()
        .read_file(&PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_file(
    path: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let workspace = state.workspace.lock().await;
    workspace
        .file_system()
        .write_file(&PathBuf::from(path), &content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_directory(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FileEntry>, String> {
    let workspace = state.workspace.lock().await;
    workspace
        .file_system()
        .list_directory(&PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_directory(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let workspace = state.workspace.lock().await;
    workspace
        .file_system()
        .create_directory(&PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_item(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let workspace = state.workspace.lock().await;
    let path_buf = PathBuf::from(path);
    if path_buf.is_dir() {
        workspace
            .file_system()
            .delete_directory(&path_buf)
            .await
            .map_err(|e| e.to_string())
    } else {
        workspace
            .file_system()
            .delete_file(&path_buf)
            .await
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn rename_item(
    path: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let workspace = state.workspace.lock().await;
    let from_path = PathBuf::from(&path);
    let to_path = from_path.parent().unwrap().join(new_name);
    
    workspace
        .file_system()
        .rename_item(&from_path, &to_path)
        .await
        .map_err(|e| e.to_string())
}

/// Workspace operations

#[tauri::command]
pub async fn add_workspace_folder(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut workspace = state.workspace.lock().await;
    workspace
        .add_folder(PathBuf::from(path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_workspace_folders(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let workspace = state.workspace.lock().await;
    Ok(workspace
        .folders()
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
pub async fn open_file_in_workspace(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let mut workspace = state.workspace.lock().await;
    let buffer = workspace
        .open_file(PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())?;
    Ok(buffer.text())
}

/// Text buffer operations

#[derive(Serialize, Deserialize)]
pub struct InsertTextParams {
    pub path: String,
    pub position: Position,
    pub text: String,
}

#[tauri::command]
pub async fn insert_text(
    params: InsertTextParams,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut workspace = state.workspace.lock().await;
    let buffer = workspace
        .get_buffer_mut(&PathBuf::from(params.path))
        .ok_or("Buffer not found")?;
    buffer
        .insert(params.position, &params.text)
        .map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
pub struct DeleteTextParams {
    pub path: String,
    pub range: Range,
}

#[tauri::command]
pub async fn delete_text(
    params: DeleteTextParams,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut workspace = state.workspace.lock().await;
    let buffer = workspace
        .get_buffer_mut(&PathBuf::from(params.path))
        .ok_or("Buffer not found")?;
    buffer.delete(params.range).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_file(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut workspace = state.workspace.lock().await;
    let buffer = workspace
        .get_buffer_mut(&PathBuf::from(path))
        .ok_or("Buffer not found")?;
    buffer.save().map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
pub struct BatchEditParams {
    pub path: String,
    pub edits: Vec<Edit>,
}

#[tauri::command]
pub async fn apply_batch_edits(
    params: BatchEditParams,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut workspace = state.workspace.lock().await;
    let buffer = workspace
        .get_buffer_mut(&PathBuf::from(params.path))
        .ok_or("Buffer not found")?;
    buffer.apply_edits(params.edits).map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
pub struct UpdateCursorsParams {
    pub path: String,
    pub cursors: Vec<Cursor>,
}

#[tauri::command]
pub async fn update_cursors(
    params: UpdateCursorsParams,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut workspace = state.workspace.lock().await;
    let buffer = workspace
        .get_buffer_mut(&PathBuf::from(params.path))
        .ok_or("Buffer not found")?;
    buffer.set_cursors(params.cursors);
    Ok(())
}

/// Search operations

#[tauri::command]
pub async fn search_files(
    query: String,
    case_sensitive: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<vibe_core::search::SearchResult>, String> {
    let workspace = state.workspace.lock().await;
    // Assuming single root workspace for now, or search all?
    // Let's search the first folder for MVP, or iterate all.
    // Ideally workspace should have a method to search all.
    // For now, let's just search the first folder if available.
    
    if let Some(root) = workspace.folders().first() {
        vibe_core::search::search_files(root, &query, case_sensitive)
            .map_err(|e| e.to_string())
    } else {
        Err("No workspace folder open".to_string())
    }
}

/// Git operations

#[tauri::command]
pub async fn get_git_status(
    state: tauri::State<'_, AppState>,
) -> Result<vibe_core::git::GitStatus, String> {
    let workspace = state.workspace.lock().await;
    
    // For MVP, check status of the first workspace folder
    if let Some(root) = workspace.folders().first() {
        vibe_core::git::get_status(root)
            .map_err(|e| e.to_string())
    } else {
        Err("No workspace folder open".to_string())
    }
}

/// Terminal operations

#[tauri::command]
pub async fn spawn_terminal(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<u32, String> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    
    // Default to bash or sh, maybe configurable later
    let shell = if cfg!(windows) { "powershell" } else { "zsh" };
    
    let id = state.terminal_manager.spawn(shell, tx).map_err(|e| e.to_string())?;
    
    // Spawn a task to forward output to frontend
    tokio::spawn(async move {
        while let Some((id, data)) = rx.recv().await {
            let _ = app_handle.emit("terminal-output", (id, data));
        }
    });
    
    Ok(id)
}

#[tauri::command]
pub async fn write_terminal(
    id: u32,
    data: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.terminal_manager.write(id, &data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resize_terminal(
    id: u32,
    rows: u16,
    cols: u16,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.terminal_manager.resize(id, rows, cols).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_commit(
    path: String,
    message: String,
    files: Vec<String>,
) -> Result<(), String> {
    vibe_core::git::commit(&PathBuf::from(path), &message, files)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_push(
    path: String,
    remote: String,
    branch: String,
) -> Result<(), String> {
    vibe_core::git::push(&PathBuf::from(path), &remote, &branch)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_pull(
    path: String,
    remote: String,
    branch: String,
) -> Result<(), String> {
    vibe_core::git::pull(&PathBuf::from(path), &remote, &branch)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_diff(
    path: String,
    file_path: String,
) -> Result<String, String> {
    vibe_core::git::get_diff(&PathBuf::from(path), &file_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_list_branches(path: String) -> Result<Vec<String>, String> {
    vibe_core::git::list_branches(&PathBuf::from(path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_switch_branch(path: String, branch: String) -> Result<(), String> {
    vibe_core::git::switch_branch(&PathBuf::from(path), &branch)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_get_history(path: String, limit: usize) -> Result<Vec<vibe_core::git::CommitInfo>, String> {
    vibe_core::git::get_history(&PathBuf::from(path), limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_discard_changes(path: String, file_path: String) -> Result<(), String> {
    vibe_core::git::discard_changes(&PathBuf::from(path), &file_path)
        .map_err(|e| e.to_string())
}

/// LSP operations

#[tauri::command]
pub async fn lsp_completion(
    language: String,
    root_path: String,
    params: CompletionParams,
    state: tauri::State<'_, AppState>,
) -> Result<Option<CompletionResponse>, String> {
    let mut manager = state.lsp_manager.lock().await;
    let client = manager.get_client_for_language(&language, &PathBuf::from(root_path))
        .await
        .map_err(|e| e.to_string())?;
    client.completion(params).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_hover(
    language: String,
    root_path: String,
    params: HoverParams,
    state: tauri::State<'_, AppState>,
) -> Result<Option<Hover>, String> {
    let mut manager = state.lsp_manager.lock().await;
    let client = manager.get_client_for_language(&language, &PathBuf::from(root_path))
        .await
        .map_err(|e| e.to_string())?;
    client.hover(params).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_goto_definition(
    language: String,
    root_path: String,
    params: GotoDefinitionParams,
    state: tauri::State<'_, AppState>,
) -> Result<Option<GotoDefinitionResponse>, String> {
    let mut manager = state.lsp_manager.lock().await;
    let client = manager.get_client_for_language(&language, &PathBuf::from(root_path))
        .await
        .map_err(|e| e.to_string())?;
    client.goto_definition(params).await.map_err(|e| e.to_string())
}

/// AI operations

#[derive(Serialize, Deserialize)]
pub struct CompletionRequest {
    pub context: CodeContext,
}

#[tauri::command]
pub async fn request_ai_completion(
    _request: CompletionRequest,
) -> Result<String, String> {
    // TODO: Implement AI completion
    Ok("AI completion not yet implemented".to_string())
}

#[derive(Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub provider: String,
    pub context: Option<String>,
    pub file_tree: Option<Vec<String>>,
    pub current_file: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PendingWrite {
    pub path: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub message: String,
    pub tool_output: String,
    pub pending_write: Option<PendingWrite>,
}

#[tauri::command]
pub async fn send_chat_message(
    mut request: ChatRequest,
    state: tauri::State<'_, AppState>,
) -> Result<ChatResponse, String> {
    let mut chat_engine = state.chat_engine.lock().await;
    
    // Set the active provider based on request
    chat_engine.set_provider_by_name(&request.provider)
        .map_err(|e| e.to_string())?;

    // Inject system prompt with tools and file tree
    let mut system_prompt = String::from(
        "You are an advanced coding assistant with access to the file system.\n\
        You can use the following tools by outputting XML tags:\n\
        - <read_file path=\"path/to/file\" />: Read file content\n\
        - <write_file path=\"path/to/file\">content</write_file>: Write content to file\n\
        - <list_dir path=\"path/to/dir\" />: List directory contents\n\n"
    );

    if let Some(files) = &request.file_tree {
        system_prompt.push_str("Available files:\n");
        for file in files {
            system_prompt.push_str(&format!("- {}\n", file));
        }
        system_prompt.push_str("\n");
    }

    // Prepend system message
    request.messages.insert(0, Message {
        role: vibe_ai::MessageRole::System,
        content: system_prompt,
    });
    
    // Format context with active filename if available
    let context = if let (Some(file), Some(content)) = (&request.current_file, &request.context) {
        Some(format!("Active File: {}\n\nFile Content:\n{}", file, content))
    } else {
        request.context
    };
        
    let response_text = chat_engine
        .chat(&request.messages, context)
        .await
        .map_err(|e| e.to_string())?;

    println!("DEBUG: AI Raw Response: {}", response_text);

    // Process tool calls
    let (tool_output, pending_write) = process_tool_calls(&response_text, &state.workspace).await;
    
    if pending_write.is_some() {
        println!("DEBUG: Pending write detected!");
    } else {
        println!("DEBUG: No pending write detected.");
    }
    
    Ok(ChatResponse {
        message: response_text,
        tool_output,
        pending_write,
    })
}

async fn process_tool_calls(response: &str, workspace_lock: &Arc<Mutex<Workspace>>) -> (String, Option<PendingWrite>) {
    let mut output = String::new();
    let mut pending_write = None;
    let workspace = workspace_lock.lock().await;

    println!("DEBUG: Processing tool calls in response of length {}", response.len());

    // <read_file path="...">
    let read_tag = "<read_file path=\"";
    if let Some(start) = response.find(read_tag) {
        if let Some(end) = response[start..].find("\" />") {
            let path = &response[start + read_tag.len()..start + end];
            match workspace.file_system().read_file(&PathBuf::from(path)).await {
                Ok(content) => output.push_str(&format!("Read file '{}':\n{}\n", path, content)),
                Err(e) => output.push_str(&format!("Failed to read file '{}': {}\n", path, e)),
            }
        }
    }

    // <write_file path="...">content</write_file>
    let write_tag_start = "<write_file path=\"";
    if let Some(start) = response.find(write_tag_start) {
        if let Some(path_end) = response[start..].find("\">") {
            let path = &response[start + write_tag_start.len()..start + path_end];
            if let Some(content_end) = response[start..].find("</write_file>") {
                let content_start = start + path_end + 2;
                let content = &response[content_start..start + content_end];
                
                // Instead of writing immediately, create a pending write
                pending_write = Some(PendingWrite {
                    path: path.to_string(),
                    content: content.to_string(),
                });
                output.push_str(&format!("Proposed write to file '{}'. Waiting for user approval.\n", path));
            }
        }
    }

    // <list_dir path="...">
    let list_tag = "<list_dir path=\"";
    if let Some(start) = response.find(list_tag) {
        if let Some(end) = response[start..].find("\" />") {
            let path = &response[start + list_tag.len()..start + end];
            match workspace.file_system().list_directory(&PathBuf::from(path)).await {
                Ok(entries) => {
                    output.push_str(&format!("Directory '{}':\n", path));
                    for entry in entries {
                        output.push_str(&format!("- {} ({})\n", entry.name, if entry.is_directory { "dir" } else { "file" }));
                    }
                },
                Err(e) => output.push_str(&format!("Failed to list directory '{}': {}\n", path, e)),
            }
        }
    }

    (output, pending_write)
}

#[tauri::command]
pub async fn get_available_ai_providers(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut chat_engine = state.chat_engine.lock().await;

    // 1. Fetch local Ollama models
    if let Ok(models) = vibe_ai::providers::ollama::OllamaProvider::list_models(None).await {
        let existing_names = chat_engine.get_provider_names();
        
        for model in models {
            let display_name = format!("Ollama ({})", model);
            if !existing_names.contains(&display_name) {
                // Register new provider for this model
                let config = vibe_ai::provider::ProviderConfig {
                    provider_type: "ollama".to_string(),
                    api_key: None,
                    model: model.clone(),
                    api_url: Some("http://localhost:11434".to_string()),
                    max_tokens: None,
                    temperature: None,
                };
                let provider = vibe_ai::providers::ollama::OllamaProvider::new(config);
                chat_engine.add_provider(Arc::new(provider));
            }
        }
    }

    // 2. Add standard cloud providers if they are not already present (simplified logic for now)
    // In a real app, we'd check config or availability for these too.
    // For now, we rely on what's registered in lib.rs or added here.
    
    Ok(chat_engine.get_provider_names())
}
