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
use crate::flow::FlowTracker;

/// Application state
pub struct AppState {
    pub workspace: Arc<Mutex<Workspace>>,
    pub chat_engine: Arc<Mutex<ChatEngine>>,
    pub terminal_manager: Arc<TerminalManager>,
    pub lsp_manager: Arc<Mutex<LspManager>>,
    pub flow: Arc<Mutex<FlowTracker>>,
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

/// Context search for @ picker

#[derive(Serialize)]
pub struct ContextFileEntry {
    pub path: String,
    pub name: String,
}

/// Return file paths matching `query` within the first workspace folder.
/// Limited to 20 results for use in the @ picker dropdown.
#[tauri::command]
pub async fn search_files_for_context(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ContextFileEntry>, String> {
    use walkdir::WalkDir;

    let workspace = state.workspace.lock().await;
    let root = workspace
        .folders()
        .first()
        .cloned()
        .ok_or("No workspace folder open")?;
    drop(workspace);

    let q = query.to_lowercase();
    let mut results = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let path_str = path.to_string_lossy();
        // Skip common non-source directories
        if path_str.contains("/target/")
            || path_str.contains("/node_modules/")
            || path_str.contains("/.git/")
            || path_str.contains("/dist/")
        {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if q.is_empty()
            || name.to_lowercase().contains(&q)
            || path_str.to_lowercase().contains(&q)
        {
            let rel = path.strip_prefix(&root).unwrap_or(path);
            results.push(ContextFileEntry {
                path: rel.to_string_lossy().to_string(),
                name: name.to_string(),
            });
            if results.len() >= 20 {
                break;
            }
        }
    }

    Ok(results)
}

/// Return formatted git context (branch + changed files + diff excerpt).
#[tauri::command]
pub async fn get_git_context(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let workspace = state.workspace.lock().await;
    let root = workspace
        .folders()
        .first()
        .cloned()
        .ok_or("No workspace folder open")?;
    drop(workspace);

    let mut ctx = String::new();
    if let Ok(status) = vibe_core::git::get_status(&root) {
        ctx.push_str(&format!("Branch: {}\n", status.branch));
        if !status.file_statuses.is_empty() {
            ctx.push_str("Changed files:\n");
            for (file, state) in &status.file_statuses {
                ctx.push_str(&format!("  {:?} {}\n", state, file));
            }
        }
    }
    if let Ok(diff) = vibe_core::git::get_repo_diff(&root) {
        if !diff.is_empty() {
            let truncated = if diff.len() > 4000 { &diff[..4000] } else { &diff };
            ctx.push_str("\n```diff\n");
            ctx.push_str(truncated);
            ctx.push_str("\n```\n");
        }
    }
    Ok(ctx)
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
    let mut context = if let (Some(file), Some(content)) = (&request.current_file, &request.context) {
        Some(format!("Active File: {}\n\nFile Content:\n{}", file, content))
    } else {
        request.context.clone()
    };

    // Resolve @file:<path> and @git references from the last user message
    if let Some(last) = request.messages.last() {
        if last.role == vibe_ai::MessageRole::User {
            let at_ctx = resolve_at_references(&last.content, &state.workspace).await;
            if !at_ctx.is_empty() {
                let base = context.unwrap_or_default();
                context = Some(if base.is_empty() {
                    at_ctx
                } else {
                    format!("{}\n\n{}", base, at_ctx)
                });
            }
        }
    }

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

/// Scan `content` for `@file:<path>` and `@git` references and return the
/// resolved context string to append to the system prompt.
async fn resolve_at_references(content: &str, workspace_lock: &Arc<Mutex<Workspace>>) -> String {
    use regex::Regex;
    let mut extra = String::new();

    let workspace = workspace_lock.lock().await;
    let root = workspace.folders().first().cloned();
    drop(workspace);

    // @file:<path> — read the file and embed its content
    let re = Regex::new(r"@file:(\S+)").unwrap();
    for cap in re.captures_iter(content) {
        let rel = &cap[1];
        let abs_path = if let Some(ref r) = root {
            r.join(rel)
        } else {
            PathBuf::from(rel)
        };
        let ext = abs_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match std::fs::read_to_string(&abs_path) {
            Ok(file_content) => {
                let snippet = if file_content.len() > 8000 {
                    format!("{}...(truncated)", &file_content[..8000])
                } else {
                    file_content
                };
                extra.push_str(&format!(
                    "\n### @file:{}\n```{}\n{}\n```\n",
                    rel, ext, snippet
                ));
            }
            Err(_) => {
                extra.push_str(&format!("\n### @file:{}\n(file not found)\n", rel));
            }
        }
    }

    // @git — inject current branch, changed files, and diff
    if content.contains("@git") {
        if let Some(ref r) = root {
            let mut git_ctx = String::from("\n### @git\n");
            if let Ok(status) = vibe_core::git::get_status(r) {
                git_ctx.push_str(&format!("Branch: {}\n", status.branch));
                for (file, state) in &status.file_statuses {
                    git_ctx.push_str(&format!("  {:?} {}\n", state, file));
                }
            }
            if let Ok(diff) = vibe_core::git::get_repo_diff(r) {
                if !diff.is_empty() {
                    let truncated = if diff.len() > 3000 { &diff[..3000] } else { &diff };
                    git_ctx.push_str(&format!("```diff\n{}\n```\n", truncated));
                }
            }
            extra.push_str(&git_ctx);
        }
    }

    extra
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

// ─── Phase 3 Commands ─────────────────────────────────────────────────────────

/// Git stash: push all current changes as a named stash. Returns the stash OID.
#[tauri::command]
pub async fn git_stash_create(
    path: String,
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let workspace = state.workspace.lock().await;
    let root = workspace.folders().first().cloned().unwrap_or_else(|| PathBuf::from(&path));
    drop(workspace);
    vibe_core::git::create_stash(&root, &name).map_err(|e| e.to_string())
}

/// Git stash pop: apply + drop the most recent stash.
#[tauri::command]
pub async fn git_stash_pop(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let workspace = state.workspace.lock().await;
    let root = workspace.folders().first().cloned().unwrap_or_else(|| PathBuf::from(&path));
    drop(workspace);
    vibe_core::git::pop_stash(&root).map_err(|e| e.to_string())
}

/// LSP: notify that a document was opened.
#[tauri::command]
pub async fn lsp_did_open(
    language: String,
    root_path: String,
    uri: String,
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use lsp_types::{DidOpenTextDocumentParams, TextDocumentItem};
    let mut manager = state.lsp_manager.lock().await;
    let client = manager
        .get_client_for_language(&language, &PathBuf::from(&root_path))
        .await
        .map_err(|e| e.to_string())?;
    let doc_uri: lsp_types::Uri = uri.parse().map_err(|_| "Invalid URI".to_string())?;
    client
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: doc_uri,
                language_id: language.clone(),
                version: 1,
                text,
            },
        })
        .await
        .map_err(|e| e.to_string())
}

/// LSP: notify that a document's content changed.
#[tauri::command]
pub async fn lsp_did_change(
    language: String,
    root_path: String,
    uri: String,
    text: String,
    version: i32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use lsp_types::{DidChangeTextDocumentParams, VersionedTextDocumentIdentifier, TextDocumentContentChangeEvent};
    let mut manager = state.lsp_manager.lock().await;
    let client = manager
        .get_client_for_language(&language, &PathBuf::from(&root_path))
        .await
        .map_err(|e| e.to_string())?;
    let doc_uri: lsp_types::Uri = uri.parse().map_err(|_| "Invalid URI".to_string())?;
    client
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri: doc_uri, version },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text,
            }],
        })
        .await
        .map_err(|e| e.to_string())
}

/// LSP: notify that a document was saved.
#[tauri::command]
pub async fn lsp_did_save(
    language: String,
    root_path: String,
    uri: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use lsp_types::{DidSaveTextDocumentParams, TextDocumentIdentifier};
    let mut manager = state.lsp_manager.lock().await;
    let client = manager
        .get_client_for_language(&language, &PathBuf::from(&root_path))
        .await
        .map_err(|e| e.to_string())?;
    let doc_uri: lsp_types::Uri = uri.parse().map_err(|_| "Invalid URI".to_string())?;
    client
        .did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: doc_uri },
            text: None,
        })
        .await
        .map_err(|e| e.to_string())
}

/// Inline AI completion using FIM format for Ollama or chat prompt for others.
/// Returns the completion text (suffix to insert at cursor).
#[tauri::command]
pub async fn request_inline_completion(
    prefix: String,
    suffix: String,
    language: String,
    provider: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let mut chat_engine = state.chat_engine.lock().await;
    chat_engine.set_provider_by_name(&provider).map_err(|e| e.to_string())?;

    // Use FIM format for Ollama (fill-in-the-middle), chat for others
    let prompt = if provider.to_lowercase().contains("ollama") {
        format!(
            "<|fim_prefix|>{}<|fim_suffix|>{}<|fim_middle|>",
            &prefix, &suffix
        )
    } else {
        format!(
            "Complete the following {} code. Return ONLY the code to insert at the cursor, nothing else.\n\nPrefix:\n```{}\n{}\n```\n\nSuffix:\n```{}\n{}\n```\n\nCompletion:",
            language, language, prefix, language, suffix
        )
    };

    let messages = vec![Message {
        role: vibe_ai::MessageRole::User,
        content: prompt,
    }];
    let result = chat_engine.chat(&messages, None).await.map_err(|e| e.to_string())?;

    // Strip any markdown code fences from the response
    let clean = result
        .trim()
        .trim_start_matches("```")
        .trim_start_matches(&language)
        .trim_end_matches("```")
        .trim()
        .to_string();

    Ok(clean)
}

/// Flow tracking: record a developer activity event.
#[tauri::command]
pub async fn track_flow_event(
    kind: String,
    data: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.flow.lock().await.record(&kind, &data);
    Ok(())
}

/// Flow context: return recent developer activity as a formatted context string.
#[tauri::command]
pub async fn get_flow_context(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.flow.lock().await.context_string(20))
}
