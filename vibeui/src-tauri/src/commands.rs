//! Tauri commands for frontend-backend communication

use std::sync::OnceLock;

// ── Lazy-compiled regex patterns ──────────────────────────────────────────────
fn re_html_tag() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"<[^>]+>").unwrap())
}
fn re_whitespace() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"\s{2,}").unwrap())
}
fn re_at_file() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@file:([^\s:]+)(?::(\d+)-(\d+))?").unwrap())
}
fn re_at_folder() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@folder:(\S+)").unwrap())
}
fn re_at_web() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@web:(https?://\S+)").unwrap())
}
fn re_at_symbol() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@symbol:(\S+)").unwrap())
}
fn re_at_docs() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@docs:(\S+)").unwrap())
}
fn re_at_codebase() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@codebase:(\S+)").unwrap())
}
fn re_at_github() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@github:([a-zA-Z0-9_\-]+)/([a-zA-Z0-9_\-]+)#(\d+)").unwrap())
}
fn re_at_jira() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"@jira:([A-Z][A-Z0-9_]+-\d+)").unwrap())
}

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
use vibe_ai::{ToolCall, ToolResult};

// ── GitHub + Jira API types (used by @github: / @jira: context handlers) ──────
#[derive(Deserialize)]
struct GithubIssue {
    #[allow(dead_code)]
    number: u64,
    title: String,
    #[serde(default)]
    body: String,
    state: String,
    labels: Vec<GithubLabel>,
    user: GithubUser,
}
#[derive(Deserialize)]
struct GithubLabel { name: String }
#[derive(Deserialize)]
struct GithubUser { login: String }

#[derive(Deserialize)]
struct JiraIssue {
    fields: JiraFields,
}
#[derive(Deserialize)]
struct JiraFields {
    summary: String,
    #[serde(default)]
    description: Option<String>,
    status: JiraStatus,
    assignee: Option<JiraAssignee>,
}
#[derive(Deserialize)]
struct JiraStatus { name: String }
#[derive(Deserialize)]
struct JiraAssignee { #[serde(rename = "displayName")] display_name: String }

/// Holds a pending tool call awaiting user approval in the agent loop.
pub struct PendingAgentCall {
    pub call: ToolCall,
    pub result_tx: tokio::sync::oneshot::Sender<Option<ToolResult>>,
}

/// Application state
pub struct AppState {
    pub workspace: Arc<Mutex<Workspace>>,
    pub chat_engine: Arc<Mutex<ChatEngine>>,
    pub terminal_manager: Arc<TerminalManager>,
    pub lsp_manager: Arc<Mutex<LspManager>>,
    pub flow: Arc<Mutex<FlowTracker>>,
    /// Slot for a tool call pending user approval during agent execution.
    pub agent_pending: Arc<Mutex<Option<PendingAgentCall>>>,
    /// Rolling buffer of terminal output lines (last MAX_TERMINAL_LINES).
    pub terminal_buffer: Arc<Mutex<Vec<String>>>,
    /// Abort handle for the currently running agent task (if any).
    pub agent_abort_handle: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
}

const MAX_TERMINAL_LINES: usize = 500;

// ── Path safety ─────────────────────────────────────────────────────────────

/// Verify that `path` stays within the workspace root directories.
///
/// Canonicalizes the path and checks it is a descendant of at least one
/// workspace folder.  Returns the validated `PathBuf` on success or a
/// human-readable error string on path-traversal attempts.
fn safe_resolve_path(workspace: &Workspace, path: &str) -> Result<PathBuf, String> {
    let path_buf = PathBuf::from(path);

    // For existing paths: canonicalize directly.
    // For new paths: normalize manually (collapse .. components).
    let canonical = if path_buf.exists() {
        path_buf.canonicalize().map_err(|e| format!("Cannot resolve path '{}': {}", path, e))?
    } else {
        let mut resolved = PathBuf::new();
        for component in path_buf.components() {
            match component {
                std::path::Component::ParentDir => { resolved.pop(); }
                std::path::Component::CurDir => {}
                c => resolved.push(c),
            }
        }
        resolved
    };

    // Check against each workspace folder.
    for folder in workspace.folders() {
        let root = if folder.exists() {
            folder.canonicalize().unwrap_or_else(|_| folder.clone())
        } else {
            folder.clone()
        };
        if canonical.starts_with(&root) {
            return Ok(path_buf);
        }
    }

    Err(format!(
        "Path traversal blocked: '{}' is outside workspace boundaries",
        path
    ))
}

/// File operations

#[tauri::command]
pub async fn read_file(path: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let workspace = state.workspace.lock().await;
    safe_resolve_path(&workspace, &path)?;
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
    safe_resolve_path(&workspace, &path)?;
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
    safe_resolve_path(&workspace, &path)?;
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
    safe_resolve_path(&workspace, &path)?;
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
    safe_resolve_path(&workspace, &path)?;
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
    safe_resolve_path(&workspace, &path)?;
    let from_path = PathBuf::from(&path);
    let parent = from_path.parent()
        .ok_or_else(|| "Cannot rename a root-level path".to_string())?;
    let to_path = parent.join(&new_name);
    // Also verify destination stays in workspace
    safe_resolve_path(&workspace, &to_path.to_string_lossy())?;

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
            let truncated = if diff.len() > 4000 { &diff[..diff.char_indices().nth(4000).map(|(i,_)| i).unwrap_or(diff.len())] } else { &diff };
            ctx.push_str("\n```diff\n");
            ctx.push_str(truncated);
            ctx.push_str("\n```\n");
        }
    }
    Ok(ctx)
}

/// Strip ANSI escape codes from terminal output.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() { break; }
                }
            } else {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
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
    let term_buf = state.terminal_buffer.clone();

    // Spawn a task to forward output to frontend and capture to rolling buffer
    tokio::spawn(async move {
        while let Some((id, data)) = rx.recv().await {
            let _ = app_handle.emit("terminal-output", (id, &data));
            // Append to rolling terminal buffer (strip ANSI, split lines)
            let clean = strip_ansi(&data);
            if !clean.is_empty() {
                let mut buf = term_buf.lock().await;
                for line in clean.lines() {
                    buf.push(line.to_string());
                }
                // Keep only the last MAX_TERMINAL_LINES
                if buf.len() > MAX_TERMINAL_LINES {
                    let drain_to = buf.len() - MAX_TERMINAL_LINES;
                    buf.drain(..drain_to);
                }
            }
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

/// Return the files changed in a given commit (by partial or full SHA hash).
#[tauri::command]
pub async fn git_get_commit_files(path: String, hash: String) -> Result<Vec<String>, String> {
    vibe_core::git::get_commit_files(&PathBuf::from(path), &hash)
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
    request: CompletionRequest,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let ctx = &request.context;
    let language = &ctx.language;

    let provider_name = {
        let engine = state.chat_engine.lock().await;
        engine
            .active_provider()
            .map(|p| p.name().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    };

    // FIM format for Ollama, chat-based for cloud providers
    let prompt = if provider_name.to_lowercase().contains("ollama") {
        format!(
            "<|fim_prefix|>{}<|fim_suffix|>{}<|fim_middle|>",
            ctx.prefix, ctx.suffix
        )
    } else {
        let extra = if ctx.additional_context.is_empty() {
            String::new()
        } else {
            format!("\n\nAdditional context:\n{}", ctx.additional_context.join("\n---\n"))
        };
        format!(
            "Complete the following {} code. Return ONLY the inserted text, no explanations.\n\nPrefix:\n```{}\n{}\n```\n\nSuffix:\n```{}\n{}\n```{}",
            language, language, ctx.prefix, language, ctx.suffix, extra
        )
    };

    let messages = vec![Message {
        role: vibe_ai::MessageRole::User,
        content: prompt,
    }];

    let engine = state.chat_engine.lock().await;
    let result = engine.chat(&messages, None).await.map_err(|e| e.to_string())?;

    // Strip any markdown code fences the model may have added
    let clean = result
        .trim()
        .trim_start_matches("```")
        .trim_start_matches(language.as_str())
        .trim_end_matches("```")
        .trim()
        .to_string();
    Ok(clean)
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

    // Inject project + global AI rules (Phase 4)
    {
        let ws = state.workspace.lock().await;
        if let Some(root) = ws.folders().first() {
            let rules = crate::memory::combined_rules(root);
            if !rules.is_empty() {
                system_prompt.push_str("## AI Rules\n");
                system_prompt.push_str(&rules);
                system_prompt.push('\n');
            }
        }
    }

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
            let at_ctx = resolve_at_references(&last.content, &state.workspace, &state.terminal_buffer).await;
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

    // Process tool calls
    let (tool_output, pending_write) = process_tool_calls(&response_text, &state.workspace).await;

    Ok(ChatResponse {
        message: response_text,
        tool_output,
        pending_write,
    })
}

/// Fetch a URL, strip HTML tags, and return plain text (≤ 6000 chars).
pub(crate) async fn fetch_and_strip(url: &str) -> Result<String, String> {
    let body = reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    // Strip HTML tags
    let stripped = re_html_tag().replace_all(&body, " ");

    // Decode common HTML entities
    let decoded = stripped
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Collapse whitespace
    let collapsed = re_whitespace().replace_all(decoded.trim(), " ");

    let text = if collapsed.len() > 6000 {
        let safe_end = collapsed.char_indices().nth(6000).map(|(i,_)| i).unwrap_or(collapsed.len());
        format!("{}…(truncated)", &collapsed[..safe_end])
    } else {
        collapsed.into_owned()
    };
    Ok(text)
}

/// Tauri command: fetch a URL and return plain-text content for AI context injection.
#[tauri::command]
pub async fn fetch_url_for_context(url: String) -> Result<String, String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Only http:// and https:// URLs are supported.".to_string());
    }
    let text = fetch_and_strip(&url).await?;
    Ok(format!("=== Web content from {} ===\n{}", url, text))
}

/// Scan `content` for `@file:<path>`, `@git`, `@web:<url>`, `@folder:<path>`, and `@terminal`
/// references and return the resolved context string to append to the system prompt.
async fn resolve_at_references(
    content: &str,
    workspace_lock: &Arc<Mutex<Workspace>>,
    terminal_buffer: &Arc<Mutex<Vec<String>>>,
) -> String {
    let mut extra = String::new();

    let workspace = workspace_lock.lock().await;
    let root = workspace.folders().first().cloned();
    drop(workspace);

    // @file:<path> or @file:<path>:<start>-<end> — read file (with optional line range)
    // Matches: @file:src/main.rs  OR  @file:src/main.rs:10-30
    let re = re_at_file();
    for cap in re.captures_iter(content) {
        let rel = &cap[1];
        let line_start: Option<usize> = cap.get(2).and_then(|m| m.as_str().parse().ok());
        let line_end:   Option<usize> = cap.get(3).and_then(|m| m.as_str().parse().ok());
        let abs_path = if let Some(ref r) = root {
            r.join(rel)
        } else {
            PathBuf::from(rel)
        };
        let ext = abs_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match std::fs::read_to_string(&abs_path) {
            Ok(file_content) => {
                let snippet = if let (Some(s), Some(e)) = (line_start, line_end) {
                    // 1-based line range
                    let lines: Vec<&str> = file_content.lines().collect();
                    let from = s.saturating_sub(1).min(lines.len());
                    let to   = e.min(lines.len());
                    lines[from..to].join("\n")
                } else if file_content.len() > 8000 {
                    let safe_end = file_content.char_indices().nth(8000).map(|(i,_)| i).unwrap_or(file_content.len());
                    format!("{}...(truncated)", &file_content[..safe_end])
                } else {
                    file_content
                };
                let range_suffix = if let (Some(s), Some(e)) = (line_start, line_end) {
                    format!(":{}:{}", s, e)
                } else {
                    String::new()
                };
                extra.push_str(&format!(
                    "\n### @file:{}{}\n```{}\n{}\n```\n",
                    rel, range_suffix, ext, snippet
                ));
            }
            Err(_) => {
                extra.push_str(&format!("\n### @file:{}\n(file not found)\n", rel));
            }
        }
    }

    // @folder:<path> — embed a listing of all files under the folder
    for cap in re_at_folder().captures_iter(content) {
        let rel = &cap[1];
        let abs_path = if let Some(ref r) = root { r.join(rel) } else { PathBuf::from(rel) };
        let mut folder_ctx = format!("\n### @folder:{}\n", rel);
        if abs_path.is_dir() {
            let walker = walkdir::WalkDir::new(&abs_path).max_depth(4).into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file());
            let mut count = 0;
            for entry in walker {
                if count >= 200 { folder_ctx.push_str("...(truncated at 200 files)\n"); break; }
                if let Ok(rel_entry) = entry.path().strip_prefix(&abs_path) {
                    folder_ctx.push_str(&format!("  {}\n", rel_entry.display()));
                    count += 1;
                }
            }
            if count == 0 { folder_ctx.push_str("(empty directory)\n"); }
        } else {
            folder_ctx.push_str("(directory not found)\n");
        }
        extra.push_str(&folder_ctx);
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
                    let truncated = if diff.len() > 3000 { &diff[..diff.char_indices().nth(3000).map(|(i,_)| i).unwrap_or(diff.len())] } else { &diff };
                    git_ctx.push_str(&format!("```diff\n{}\n```\n", truncated));
                }
            }
            extra.push_str(&git_ctx);
        }
    }

    // @web:<url> — fetch the URL and embed plain-text content
    for cap in re_at_web().captures_iter(content) {
        let url = &cap[1];
        match fetch_and_strip(url).await {
            Ok(text) => {
                extra.push_str(&format!("\n### @web:{}\n{}\n", url, text));
            }
            Err(e) => {
                extra.push_str(&format!("\n### @web:{}\n(fetch error: {})\n", url, e));
            }
        }
    }

    // @html-selected — context injected from Browser panel element inspector (UI-side)
    if content.contains("@html-selected") {
        extra.push_str("\n### @html-selected\n[HTML element context injected from Browser panel]\n");
    }

    // @terminal — inject last 200 lines from the terminal output buffer
    if content.contains("@terminal") {
        let buf = terminal_buffer.lock().await;
        let lines = buf.len();
        let take = lines.min(200);
        let snippet: Vec<&String> = buf[lines - take..].iter().collect();
        let text = snippet.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
        drop(buf);
        extra.push_str(&format!("\n### @terminal (last {} lines)\n```\n{}\n```\n", take, text));
    }

    // @symbol:<name> — find the symbol in the codebase via regex-based index
    for cap in re_at_symbol().captures_iter(content) {
        let query = &cap[1];
        if let Some(ref r) = root {
            let mut idx = vibe_core::index::CodebaseIndex::new(r.clone());
            let _ = idx.build();
            let hits = idx.search_symbols(query);
            let mut sym_ctx = format!("\n### @symbol:{}\n", query);
            if hits.is_empty() {
                sym_ctx.push_str("(no symbols found)\n");
            } else {
                for sym in hits.iter().take(5) {
                    let rel = sym.file.strip_prefix(r).unwrap_or(&sym.file);
                    sym_ctx.push_str(&format!(
                        "**{}** ({:?}) — {}:{}\n",
                        sym.name,
                        sym.kind,
                        rel.display(),
                        sym.line
                    ));
                    // Embed a few lines of source around the symbol
                    if let Ok(src) = std::fs::read_to_string(&sym.file) {
                        let lines: Vec<&str> = src.lines().collect();
                        let from = sym.line.saturating_sub(1);
                        let to = (from + 10).min(lines.len());
                        let ext = sym.file.extension().and_then(|e| e.to_str()).unwrap_or("");
                        sym_ctx.push_str(&format!("```{}\n{}\n```\n", ext, lines[from..to].join("\n")));
                    }
                }
            }
            extra.push_str(&sym_ctx);
        }
    }

    // @docs:<name> — fetch library documentation from docs.rs / npmjs.com / pypi.org
    for cap in re_at_docs().captures_iter(content) {
        let name_raw = &cap[1];
        // Detect registry: rs:→docs.rs, py:/pypi:→PyPI, npm:→npmjs, else→docs.rs
        let (registry, name) = if name_raw.starts_with("rs:") {
            ("rs", name_raw.trim_start_matches("rs:"))
        } else if name_raw.starts_with("py:") || name_raw.starts_with("pypi:") {
            let n = name_raw.splitn(2, ':').nth(1).unwrap_or(name_raw);
            ("py", n)
        } else if name_raw.starts_with("npm:") {
            ("npm", name_raw.trim_start_matches("npm:"))
        } else {
            ("rs", name_raw) // default: docs.rs for simple names
        };
        let url = match registry {
            "npm" => format!("https://registry.npmjs.org/{}", name),
            "py"  => format!("https://pypi.org/pypi/{}/json", name),
            _     => format!("https://docs.rs/crate/{}/latest/", name),
        };
        let mut docs_ctx = format!("\n### @docs:{}\n", name_raw);
        match fetch_and_strip(&url).await {
            Ok(text) => {
                let truncated: String = text.chars().take(3000).collect();
                docs_ctx.push_str(&truncated);
            }
            Err(e) => {
                docs_ctx.push_str(&format!("(docs fetch error: {})\n", e));
            }
        }
        extra.push_str(&docs_ctx);
    }

    // @codebase:<query> — semantic search over the workspace embedding index
    for cap in re_at_codebase().captures_iter(content) {
        let query = &cap[1];
        if let Some(ref r) = root {
            let index_path = r.join(".vibeui").join("embeddings").join("index.json");
            let mut cb_ctx = format!("\n### @codebase:{}\n", query);

            // Try EmbeddingIndex first (semantic); fall back to CodebaseIndex (keyword)
            let mut used_semantic = false;
            if index_path.exists() {
                use vibe_core::index::embeddings::EmbeddingIndex;
                match EmbeddingIndex::load(&index_path) {
                    Ok(emb_idx) => {
                        match emb_idx.search(query, 5).await {
                            Ok(hits) => {
                                if hits.is_empty() {
                                    cb_ctx.push_str("(no semantically relevant code found)\n");
                                } else {
                                    for hit in &hits {
                                        let rel = hit.file.strip_prefix(r).unwrap_or(&hit.file);
                                        cb_ctx.push_str(&format!(
                                            "{}:{}-{} (score {:.2})\n{}\n",
                                            rel.display(),
                                            hit.chunk_start,
                                            hit.chunk_end,
                                            hit.score,
                                            hit.text.lines().take(4).collect::<Vec<_>>().join("\n")
                                        ));
                                    }
                                }
                                used_semantic = true;
                            }
                            Err(e) => {
                                eprintln!("[vibeui] @codebase: semantic search error: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[vibeui] @codebase: could not load embedding index: {e}");
                    }
                }
            }

            if !used_semantic {
                // Keyword fallback
                let mut idx = vibe_core::index::CodebaseIndex::new(r.clone());
                let _ = idx.build();
                let hits = idx.search_symbols(query);
                if hits.is_empty() {
                    cb_ctx.push_str("(no relevant code found — run /index to build the embedding index)\n");
                } else {
                    for sym in hits.iter().take(5) {
                        let rel = sym.file.strip_prefix(r).unwrap_or(&sym.file);
                        cb_ctx.push_str(&format!("{}:{} — {}\n", rel.display(), sym.line, sym.name));
                    }
                }
            }

            extra.push_str(&cb_ctx);
        }
    }

    // ── @github:owner/repo#N ─────────────────────────────────────────────────
    for cap in re_at_github().captures_iter(content) {
        let owner = cap[1].to_string();
        let repo  = cap[2].to_string();
        let num: u64 = cap[3].parse().unwrap_or(0);
        if num == 0 { continue; }

        let api_url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}",
            owner, repo, num
        );
        let gh_token = std::env::var("GITHUB_TOKEN").ok();
        let mut gh_ctx = format!("\n=== GitHub Issue: {}/{}#{} ===\n", owner, repo, num);
        match fetch_github_issue(&api_url, gh_token).await {
            Ok(issue) => {
                let labels: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();
                gh_ctx.push_str(&format!(
                    "Title: {}\nState: {} | Author: {} | Labels: {}\n\n{}\n",
                    issue.title,
                    issue.state,
                    issue.user.login,
                    if labels.is_empty() { "none".to_string() } else { labels.join(", ") },
                    issue.body.lines().take(30).collect::<Vec<_>>().join("\n"),
                ));
            }
            Err(e) => {
                gh_ctx.push_str(&format!("(GitHub fetch error: {})\n", e));
            }
        }
        extra.push_str(&gh_ctx);
    }

    // ── @jira:PROJECT-123 ─────────────────────────────────────────────────────
    let jira_caps: Vec<String> = re_at_jira()
        .captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect();
    if !jira_caps.is_empty() {
        let base_url = std::env::var("JIRA_BASE_URL").unwrap_or_default();
        let email    = std::env::var("JIRA_EMAIL").unwrap_or_default();
        let token    = std::env::var("JIRA_API_TOKEN").unwrap_or_default();
        for issue_key in jira_caps {
            let mut jira_ctx = format!("\n=== Jira Issue: {} ===\n", issue_key);
            if base_url.is_empty() {
                jira_ctx.push_str("(set JIRA_BASE_URL, JIRA_EMAIL, JIRA_API_TOKEN to fetch Jira issues)\n");
            } else {
                let api_url = format!("{}/rest/api/2/issue/{}", base_url.trim_end_matches('/'), issue_key);
                match fetch_jira_issue(&api_url, &email, &token).await {
                    Ok(issue) => {
                        let assignee = issue.fields.assignee
                            .map(|a| a.display_name)
                            .unwrap_or_else(|| "unassigned".to_string());
                        let desc = issue.fields.description.unwrap_or_default();
                        let snippet: String = desc.lines().take(20).collect::<Vec<_>>().join("\n");
                        jira_ctx.push_str(&format!(
                            "Summary: {}\nStatus: {} | Assignee: {}\n\n{}\n",
                            issue.fields.summary,
                            issue.fields.status.name,
                            assignee,
                            if snippet.is_empty() { "(no description)" } else { &snippet },
                        ));
                    }
                    Err(e) => {
                        jira_ctx.push_str(&format!("(Jira fetch error: {})\n", e));
                    }
                }
            }
            extra.push_str(&jira_ctx);
        }
    }

    extra
}

async fn fetch_jira_issue(url: &str, email: &str, token: &str) -> Result<JiraIssue, String> {
    let client = reqwest::Client::new();
    let mut req = client
        .get(url)
        .header("Accept", "application/json")
        .header("User-Agent", "vibecli/1.0");
    if !email.is_empty() && !token.is_empty() {
        req = req.basic_auth(email, Some(token));
    }
    req.send().await
        .map_err(|e| e.to_string())?
        .json::<JiraIssue>()
        .await
        .map_err(|e| e.to_string())
}

async fn fetch_github_issue(url: &str, token: Option<String>) -> Result<GithubIssue, String> {
    let client = reqwest::Client::new();
    let mut req = client
        .get(url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "vibecli/1.0");
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    req.send().await
        .map_err(|e| e.to_string())?
        .json::<GithubIssue>()
        .await
        .map_err(|e| e.to_string())
}

async fn process_tool_calls(response: &str, workspace_lock: &Arc<Mutex<Workspace>>) -> (String, Option<PendingWrite>) {
    let mut output = String::new();
    let mut pending_write = None;
    let workspace = workspace_lock.lock().await;


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
                    ..Default::default()
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

// ─── Phase 4 Commands ─────────────────────────────────────────────────────────

/// Serializable info sent to the frontend when a tool call needs approval.
#[derive(Serialize, Clone)]
pub struct AgentPendingPayload {
    pub name: String,
    pub summary: String,
    pub is_destructive: bool,
}

/// Serializable step info sent to the frontend after a tool call executes.
#[derive(Serialize, Clone)]
pub struct AgentStepPayload {
    pub step_num: usize,
    pub tool_name: String,
    pub tool_summary: String,
    pub output: String,
    pub success: bool,
    pub approved: bool,
}

/// Start an autonomous agent task. Emits Tauri events:
/// - `agent:chunk`   — streaming LLM text (String)
/// - `agent:pending` — tool call needs approval (AgentPendingPayload)
/// - `agent:step`    — step completed (AgentStepPayload)
/// - `agent:complete`— task done (String summary)
/// - `agent:error`   — error (String)
#[tauri::command]
pub async fn start_agent_task(
    task: String,
    approval_policy: String,
    provider: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use vibe_ai::{AgentLoop, AgentContext, ApprovalPolicy, AgentEvent};
    use crate::agent_executor::TauriToolExecutor;

    // Get the AI provider
    let provider_arc = {
        let mut engine = state.chat_engine.lock().await;
        engine.set_provider_by_name(&provider).map_err(|e| e.to_string())?;
        engine.active_provider().ok_or("No active provider")?.clone()
    };

    // Get workspace root
    let workspace_root = {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned().unwrap_or_else(|| PathBuf::from("."))
    };

    // Build agent context
    let git_branch = vibe_core::git::get_current_branch(&workspace_root).ok();
    let git_diff = vibe_core::git::get_repo_diff(&workspace_root).ok().map(|d| {
        if d.len() > 2000 { d[..2000].to_string() + "\n…(truncated)" } else { d }
    });
    let context = AgentContext {
        workspace_root: workspace_root.clone(),
        open_files: vec![],
        git_branch,
        git_diff_summary: git_diff,
        flow_context: None,
        approved_plan: None,
        extra_skill_dirs: vec![],
        parent_session_id: None,
        depth: 0,
        active_agent_counter: None,
    };

    let executor = Arc::new(TauriToolExecutor::new(workspace_root.clone()));
    let approval = ApprovalPolicy::from_str(&approval_policy);
    let agent = AgentLoop::new(provider_arc, approval, executor)
        .with_policy(&workspace_root);

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<AgentEvent>(64);
    let agent_pending = state.agent_pending.clone();
    let abort_handle_slot = state.agent_abort_handle.clone();

    // Spawn the agent loop and store its abort handle for stop_agent_task
    let join = tokio::spawn(async move {
        let _ = agent.run(&task, context, event_tx).await;
    });
    *abort_handle_slot.lock().await = Some(join.abort_handle());

    // Bridge agent events → Tauri events
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                AgentEvent::StreamChunk(text) => {
                    let _ = app_handle.emit("agent:chunk", text);
                }
                AgentEvent::ToolCallPending { call, result_tx } => {
                    let payload = AgentPendingPayload {
                        name: call.name().to_string(),
                        summary: call.summary(),
                        is_destructive: call.is_destructive(),
                    };
                    // Store for respond_to_agent_approval
                    {
                        let mut slot = agent_pending.lock().await;
                        *slot = Some(PendingAgentCall { call, result_tx });
                    }
                    let _ = app_handle.emit("agent:pending", payload);
                }
                AgentEvent::ToolCallExecuted(step) => {
                    let payload = AgentStepPayload {
                        step_num: step.step_num,
                        tool_name: step.tool_call.name().to_string(),
                        tool_summary: step.tool_call.summary(),
                        output: step.tool_result.output.clone(),
                        success: step.tool_result.success,
                        approved: step.approved,
                    };
                    let _ = app_handle.emit("agent:step", payload);
                }
                AgentEvent::Complete(summary) => {
                    let _ = app_handle.emit("agent:complete", summary);
                    break;
                }
                AgentEvent::Error(msg) => {
                    let _ = app_handle.emit("agent:error", msg);
                    break;
                }
            }
        }
    });

    Ok(())
}

/// Respond to an agent tool-call approval prompt.
/// Abort the currently running agent task (if any).
/// Emits `agent:error` with "Agent stopped by user" so the frontend can reset its state.
#[tauri::command]
pub async fn stop_agent_task(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let handle = {
        let mut slot = state.agent_abort_handle.lock().await;
        slot.take()
    };
    if let Some(h) = handle {
        h.abort();
    }
    // Clear any pending approval so the UI doesn't stay blocked
    {
        let mut slot = state.agent_pending.lock().await;
        *slot = None;
    }
    let _ = app_handle.emit("agent:error", "Agent stopped by user");
    Ok(())
}

/// - `approved = true`  → execute the tool and send result to agent
/// - `approved = false` → reject (agent receives a "rejected" result)
#[tauri::command]
pub async fn respond_to_agent_approval(
    approved: bool,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use crate::agent_executor::TauriToolExecutor;

    let pending = {
        let mut slot = state.agent_pending.lock().await;
        slot.take()
    };

    let Some(PendingAgentCall { call, result_tx }) = pending else {
        return Err("No pending agent approval".to_string());
    };

    if approved {
        let workspace_root = {
            let ws = state.workspace.lock().await;
            ws.folders().first().cloned().unwrap_or_else(|| PathBuf::from("."))
        };
        let executor = TauriToolExecutor::new(workspace_root);
        let result = executor.execute_call(&call).await;

        // Emit a step event so the UI shows what happened
        let payload = AgentStepPayload {
            step_num: 0, // step_num not tracked here
            tool_name: call.name().to_string(),
            tool_summary: call.summary(),
            output: result.output.clone(),
            success: result.success,
            approved: true,
        };
        let _ = app_handle.emit("agent:step", payload);

        let _ = result_tx.send(Some(result));
    } else {
        // Rejection: send None — agent will record "rejected by user"
        let _ = result_tx.send(None);
    }

    Ok(())
}

// ─── Memory / Rules Commands ───────────────────────────────────────────────────

/// Get project-level AI rules from `<workspace>/.vibeui.md`.
#[tauri::command]
pub async fn get_vibeui_rules(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let ws = state.workspace.lock().await;
    let root = ws.folders().first().cloned().ok_or("No workspace folder open")?;
    drop(ws);
    Ok(crate::memory::load_workspace_rules(&root))
}

/// Save project-level AI rules to `<workspace>/.vibeui.md`.
#[tauri::command]
pub async fn save_vibeui_rules(
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let ws = state.workspace.lock().await;
    let root = ws.folders().first().cloned().ok_or("No workspace folder open")?;
    drop(ws);
    crate::memory::save_workspace_rules(&root, &content).map_err(|e| e.to_string())
}

/// Get global AI rules from `~/.vibeui/rules.md`.
#[tauri::command]
pub async fn get_global_rules() -> Result<String, String> {
    Ok(crate::memory::load_global_rules())
}

/// Save global AI rules to `~/.vibeui/rules.md`.
#[tauri::command]
pub async fn save_global_rules(content: String) -> Result<(), String> {
    crate::memory::save_global_rules(&content).map_err(|e| e.to_string())
}

// ─── Rules Directory Commands ──────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RuleFileMeta {
    pub filename: String,
    pub name: String,
    pub path_pattern: Option<String>,
}

fn rules_dir(scope: &str, workspace_root: Option<&std::path::Path>) -> std::path::PathBuf {
    if scope == "workspace" {
        workspace_root
            .unwrap_or(std::path::Path::new("."))
            .join(".vibecli")
            .join("rules")
    } else {
        std::path::PathBuf::from(
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
        )
        .join(".vibecli")
        .join("rules")
    }
}

fn parse_rule_meta(content: &str, filename: &str) -> RuleFileMeta {
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
        .to_string();
    let mut name = stem.clone();
    let mut path_pattern: Option<String> = None;
    if let Some(after_prefix) = content.strip_prefix("---") {
        let after = after_prefix.trim_start_matches('\n');
        if let Some(close) = after.find("\n---") {
            for line in after[..close].lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                    match k.trim() {
                        "name" => name = val,
                        "path_pattern" => path_pattern = Some(val),
                        _ => {}
                    }
                }
            }
        }
    }
    RuleFileMeta { filename: filename.to_string(), name, path_pattern }
}

/// List all `.md` files in the rules directory (scope: "workspace" | "global").
#[tauri::command]
pub async fn list_rule_files(
    scope: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RuleFileMeta>, String> {
    let root = if scope == "workspace" {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned()
    } else {
        None
    };
    let dir = rules_dir(&scope, root.as_deref());
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut metas = vec![];
    let rd = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) == Some("md") {
            let filename = path.file_name().and_then(|x| x.to_str()).unwrap_or("").to_string();
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            metas.push(parse_rule_meta(&content, &filename));
        }
    }
    metas.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(metas)
}

/// Read full content of a rule file.
#[tauri::command]
pub async fn get_rule_file(
    scope: String,
    filename: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let root = if scope == "workspace" {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned()
    } else {
        None
    };
    let path = rules_dir(&scope, root.as_deref()).join(&filename);
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Write (create or overwrite) a rule file.
#[tauri::command]
pub async fn save_rule_file(
    scope: String,
    filename: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let root = if scope == "workspace" {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned()
    } else {
        None
    };
    let dir = rules_dir(&scope, root.as_deref());
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(&filename), content).map_err(|e| e.to_string())
}

/// Delete a rule file permanently.
#[tauri::command]
pub async fn delete_rule_file(
    scope: String,
    filename: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let root = if scope == "workspace" {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned()
    } else {
        None
    };
    let path = rules_dir(&scope, root.as_deref()).join(&filename);
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

// ─── MCP Server Manager Commands ──────────────────────────────────────────────

fn mcp_config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibeui").join("mcp.json")
}

/// Return all configured MCP servers from `~/.vibeui/mcp.json`.
#[tauri::command]
pub async fn get_mcp_servers() -> Result<Vec<serde_json::Value>, String> {
    let path = mcp_config_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str::<Vec<serde_json::Value>>(&text).map_err(|e| e.to_string())
}

/// Persist the MCP server list to `~/.vibeui/mcp.json`.
#[tauri::command]
pub async fn save_mcp_servers(servers: Vec<serde_json::Value>) -> Result<(), String> {
    let path = mcp_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&servers).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
}

/// Spawn a temporary MCP server connection and list its tools.
#[tauri::command]
pub async fn test_mcp_server(server: serde_json::Value) -> Result<Vec<McpToolInfo>, String> {
    let cfg: vibe_ai::mcp::McpServerConfig =
        serde_json::from_value(server).map_err(|e| format!("Invalid server config: {}", e))?;
    tokio::task::spawn_blocking(move || {
        let mut client =
            vibe_ai::mcp::McpClient::connect(&cfg).map_err(|e| format!("Connect failed: {:#}", e))?;
        let tools = client.list_tools().map_err(|e| format!("list_tools failed: {:#}", e))?;
        Ok(tools
            .into_iter()
            .map(|t| McpToolInfo { name: t.name, description: t.description })
            .collect())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

// ─── MCP OAuth Commands ────────────────────────────────────────────────────────

fn mcp_token_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibeui").join("mcp-tokens.json")
}

fn load_mcp_tokens() -> serde_json::Map<String, serde_json::Value> {
    let path = mcp_token_path();
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&text) {
            return map;
        }
    }
    serde_json::Map::new()
}

fn save_mcp_tokens(tokens: &serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
    let path = mcp_token_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(tokens.clone()))
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

/// Build the OAuth authorization URL and open it in the system browser.
/// The caller is responsible for listening to the redirect and passing the code
/// to `complete_mcp_oauth`.
#[tauri::command]
pub async fn initiate_mcp_oauth(
    app: tauri::AppHandle,
    server_name: String,
    client_id: String,
    auth_url: String,
    redirect_uri: String,
    scopes: String,
) -> Result<String, String> {
    use tauri_plugin_opener::OpenerExt;
    // Validate the auth_url starts with https
    if !auth_url.starts_with("https://") && !auth_url.starts_with("http://localhost") {
        return Err("auth_url must start with https:// or http://localhost".to_string());
    }
    let state_token = format!("vibecli-{}-{}", server_name, std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());
    let oauth_url = {
        let params: Vec<(&str, &str)> = vec![
            ("client_id",     &client_id),
            ("redirect_uri",  &redirect_uri),
            ("response_type", "code"),
            ("scope",         &scopes),
            ("state",         &state_token),
        ];
        let qs: String = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();
        format!("{}?{}", auth_url, qs)
    };
    app.opener()
        .open_url(&oauth_url, None::<&str>)
        .map_err(|e| format!("Failed to open browser: {}", e))?;
    Ok(state_token)
}

/// Exchange an authorization code for a token and persist it.
#[tauri::command]
pub async fn complete_mcp_oauth(
    server_name: String,
    code: String,
    token_url: String,
    client_id: String,
    redirect_uri: String,
) -> Result<String, String> {
    if !token_url.starts_with("https://") && !token_url.starts_with("http://localhost") {
        return Err("token_url must start with https:// or http://localhost".to_string());
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(&token_url)
        .form(&[
            ("grant_type",    "authorization_code"),
            ("code",          &code),
            ("client_id",     &client_id),
            ("redirect_uri",  &redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| format!("Token request failed: {}", e))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("Token parse error: {}", e))?;
    let access_token = body["access_token"].as_str()
        .ok_or_else(|| format!("No access_token in response: {}", body))?
        .to_string();
    // Persist token
    let mut tokens = load_mcp_tokens();
    tokens.insert(server_name.clone(), serde_json::json!({
        "access_token": access_token,
        "token_type":   body["token_type"].as_str().unwrap_or("Bearer"),
        "expires_in":   body["expires_in"].as_u64().unwrap_or(3600),
        "obtained_at":  std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
    }));
    save_mcp_tokens(&tokens)?;
    Ok(access_token)
}

/// Return whether a token is stored for the given MCP server name.
#[tauri::command]
pub async fn get_mcp_token_status(server_name: String) -> Result<serde_json::Value, String> {
    let tokens = load_mcp_tokens();
    if let Some(rec) = tokens.get(&server_name) {
        let obtained = rec["obtained_at"].as_u64().unwrap_or(0);
        let expires  = rec["expires_in"].as_u64().unwrap_or(3600);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let expired = now > obtained + expires;
        Ok(serde_json::json!({ "connected": true, "expired": expired }))
    } else {
        Ok(serde_json::json!({ "connected": false, "expired": true }))
    }
}

// ─── Test Runner (Phase 43) ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct TestResult {
    pub name: String,
    pub status: String, // "passed" | "failed" | "ignored" | "running"
    pub duration_ms: Option<u64>,
    pub output: Option<String>,
}

#[derive(Serialize)]
pub struct TestRunResult {
    pub framework: String,
    pub passed: u32,
    pub failed: u32,
    pub ignored: u32,
    pub total: u32,
    pub duration_ms: u64,
    pub tests: Vec<TestResult>,
}

/// Detect which test framework the workspace uses.
#[tauri::command]
pub async fn detect_test_framework(workspace: String) -> String {
    let ws = std::path::Path::new(&workspace);
    if ws.join("Cargo.toml").exists() { return "cargo test".to_string(); }
    // Check package.json for test script
    if let Ok(txt) = std::fs::read_to_string(ws.join("package.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            if v["scripts"]["test"].is_string() {
                let mgr = if ws.join("bun.lockb").exists() { "bun" }
                    else if ws.join("yarn.lock").exists() { "yarn" }
                    else { "npm" };
                return format!("{} test", mgr);
            }
        }
    }
    if ws.join("pytest.ini").exists() || ws.join("pyproject.toml").exists() || ws.join("setup.py").exists() {
        return "pytest".to_string();
    }
    if ws.join("go.mod").exists() { return "go test ./...".to_string(); }
    "unknown".to_string()
}

/// Run tests in the workspace and return parsed results.
///
/// Emits `test:log` events for each output line so the frontend can show a live stream.
#[tauri::command]
pub async fn run_tests(
    app: tauri::AppHandle,
    workspace: String,
    command: Option<String>,
) -> Result<TestRunResult, String> {
    let framework = detect_test_framework(workspace.clone()).await;
    let cmd_str = command.unwrap_or_else(|| framework.clone());
    if cmd_str == "unknown" {
        return Err("Could not detect a test framework. Set a custom command.".to_string());
    }

    let started = std::time::Instant::now();
    let _ = app.emit("test:log", format!("$ {}", cmd_str));

    let (prog, args_str) = if cmd_str.starts_with("cargo") {
        ("cargo", "test --message-format=json --quiet")
    } else if cmd_str.starts_with("bun") {
        ("bun", "test")
    } else if cmd_str.starts_with("yarn") {
        ("yarn", "test --json 2>&1 || true")
    } else if cmd_str.starts_with("npm") {
        ("npm", "test -- --json 2>&1 || true")
    } else if cmd_str.starts_with("pytest") {
        ("python", "-m pytest -v --tb=short --no-header 2>&1 || true")
    } else if cmd_str.starts_with("go test") {
        ("go", "test -v ./... 2>&1 || true")
    } else {
        // custom command: run via sh
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd_str)
            .current_dir(&workspace)
            .output()
            .map_err(|e| format!("Failed to run: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        for line in text.lines() {
            let _ = app.emit("test:log", line.to_string());
        }
        let elapsed = started.elapsed().as_millis() as u64;
        let passed = if output.status.success() { 1 } else { 0 };
        let failed = 1 - passed;
        return Ok(TestRunResult {
            framework: cmd_str,
            passed, failed, ignored: 0, total: 1, duration_ms: elapsed,
            tests: vec![TestResult {
                name: "Test run".to_string(),
                status: if output.status.success() { "passed".to_string() } else { "failed".to_string() },
                duration_ms: Some(elapsed),
                output: if !output.status.success() { Some(text.chars().take(2000).collect()) } else { None },
            }],
        });
    };

    let output = std::process::Command::new(prog)
        .args(args_str.split_whitespace())
        .current_dir(&workspace)
        .output()
        .map_err(|e| format!("Failed to run {} {}: {}", prog, args_str, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    for line in combined.lines().take(500) {
        let _ = app.emit("test:log", line.to_string());
    }

    let elapsed = started.elapsed().as_millis() as u64;

    // Parse results based on framework
    let mut tests: Vec<TestResult> = Vec::new();

    if prog == "cargo" {
        // Parse cargo test JSON events
        for line in stdout.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            if v["type"].as_str() != Some("test") { continue; }
            let event  = v["event"].as_str().unwrap_or("");
            let name   = v["name"].as_str().unwrap_or("?").to_string();
            let dur_ms = v["exec_time"].as_f64().map(|s| (s * 1000.0) as u64);
            let stdout_val = v["stdout"].as_str().map(|s| s.to_string());
            let status = match event {
                "ok"      => "passed",
                "failed"  => "failed",
                "ignored" => "ignored",
                _         => "running",
            };
            tests.push(TestResult { name, status: status.to_string(), duration_ms: dur_ms, output: stdout_val });
        }
    } else {
        // Generic line-by-line parsing for pytest/go/npm
        for line in combined.lines() {
            let trimmed = line.trim();
            if prog == "python" {
                // pytest: "PASSED path/test.py::func_name" or "FAILED path::func"
                if let Some(rest) = trimmed.strip_prefix("PASSED ") {
                    tests.push(TestResult { name: rest.trim().to_string(), status: "passed".to_string(), duration_ms: None, output: None });
                } else if let Some(rest) = trimmed.strip_prefix("FAILED ") {
                    tests.push(TestResult { name: rest.trim().to_string(), status: "failed".to_string(), duration_ms: None, output: None });
                }
            } else if prog == "go" {
                // go test: "--- PASS: TestName (0.00s)"
                if let Some(after_pass) = trimmed.strip_prefix("--- PASS: ") {
                    let parts: Vec<&str> = after_pass.split_whitespace().collect();
                    let name = parts.first().unwrap_or(&"?").to_string();
                    let dur: Option<u64> = parts.get(1).and_then(|s| s.trim_matches(['(','s',')']).parse::<f64>().ok()).map(|s| (s * 1000.0) as u64);
                    tests.push(TestResult { name, status: "passed".to_string(), duration_ms: dur, output: None });
                } else if let Some(after_fail) = trimmed.strip_prefix("--- FAIL: ") {
                    let parts: Vec<&str> = after_fail.split_whitespace().collect();
                    let name = parts.first().unwrap_or(&"?").to_string();
                    tests.push(TestResult { name, status: "failed".to_string(), duration_ms: None, output: None });
                }
            }
        }
        // If we couldn't parse individual tests, synthesize a single result
        if tests.is_empty() {
            tests.push(TestResult {
                name: "Test suite".to_string(),
                status: if output.status.success() { "passed".to_string() } else { "failed".to_string() },
                duration_ms: Some(elapsed),
                output: if !output.status.success() { Some(combined.chars().take(2000).collect()) } else { None },
            });
        }
    }

    let passed  = tests.iter().filter(|t| t.status == "passed").count() as u32;
    let failed  = tests.iter().filter(|t| t.status == "failed").count() as u32;
    let ignored = tests.iter().filter(|t| t.status == "ignored").count() as u32;
    let total   = tests.len() as u32;

    Ok(TestRunResult { framework, passed, failed, ignored, total, duration_ms: elapsed, tests })
}

// ─── AI Commit Message (Phase 43) ─────────────────────────────────────────────

/// Generate a commit message for the current git diff using the active LLM provider.
#[tauri::command]
pub async fn generate_commit_message(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    // Get the workspace path
    let ws = state.workspace.lock().await;
    let ws_path = ws.folders().first()
        .cloned()
        .ok_or_else(|| "No workspace open".to_string())?;
    drop(ws);

    // Run git diff --staged
    let diff_output = std::process::Command::new("git")
        .args(["diff", "--staged", "--stat", "--diff-algorithm=histogram"])
        .current_dir(&ws_path)
        .output()
        .map_err(|e| format!("git diff failed: {}", e))?;
    let stat = String::from_utf8_lossy(&diff_output.stdout);

    let diff_output2 = std::process::Command::new("git")
        .args(["diff", "--staged", "--unified=3"])
        .current_dir(&ws_path)
        .output()
        .map_err(|e| format!("git diff body failed: {}", e))?;
    let diff_body = String::from_utf8_lossy(&diff_output2.stdout);

    if stat.trim().is_empty() && diff_body.trim().is_empty() {
        return Err("No staged changes. Stage files first with git add.".to_string());
    }

    let prompt = format!(
        r#"Write a concise git commit message for the following staged diff.
Rules: imperative mood, ≤72 chars subject line, no trailing period, no "feat:"/"fix:" prefix.
Optionally add a blank line + 1-3 bullet body lines for complex changes.

--- stat ---
{}
--- diff (first 4000 chars) ---
{}
---
Respond with the commit message only, no explanation."#,
        stat.trim(),
        diff_body.chars().take(4000).collect::<String>()
    );

    let engine = state.chat_engine.lock().await;
    let messages = vec![vibe_ai::Message {
        role: vibe_ai::MessageRole::User,
        content: prompt,
    }];
    engine.chat(&messages, None)
        .await
        .map(|r| r.trim().to_string())
        .map_err(|e| e.to_string())
}

// ─── Checkpoint Commands ───────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CheckpointInfo {
    pub index: usize,
    pub message: String,
    pub oid: String,
}

/// Create a git stash checkpoint with a label.
#[tauri::command]
pub async fn create_checkpoint(
    label: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let ws = state.workspace.lock().await;
    let root = ws.folders().first().cloned().ok_or("No workspace folder open")?;
    drop(ws);
    let name = format!("vibeui-checkpoint: {}", label);
    vibe_core::git::create_stash(&root, &name).map_err(|e| e.to_string())
}

/// List all git stash checkpoints.
#[tauri::command]
pub async fn list_checkpoints(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<CheckpointInfo>, String> {
    let ws = state.workspace.lock().await;
    let root = ws.folders().first().cloned().ok_or("No workspace folder open")?;
    drop(ws);
    vibe_core::git::list_stashes(&root)
        .map(|stashes| {
            stashes.into_iter().map(|s| CheckpointInfo {
                index: s.index,
                message: s.message,
                oid: s.oid,
            }).collect()
        })
        .map_err(|e| e.to_string())
}

/// Restore (apply) a checkpoint by index. Does not drop the stash.
#[tauri::command]
pub async fn restore_checkpoint(
    index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let ws = state.workspace.lock().await;
    let root = ws.folders().first().cloned().ok_or("No workspace folder open")?;
    drop(ws);
    vibe_core::git::restore_stash(&root, index).map_err(|e| e.to_string())
}

/// Delete (drop) a checkpoint by index permanently.
#[tauri::command]
pub async fn delete_checkpoint(
    index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let ws = state.workspace.lock().await;
    let root = ws.folders().first().cloned().ok_or("No workspace folder open")?;
    drop(ws);
    vibe_core::git::drop_stash(&root, index).map_err(|e| e.to_string())
}

// ─── Phase 7.3 — Next-Edit Prediction ────────────────────────────────────────

/// A single edit event used as input for next-edit prediction.
#[derive(Deserialize, Debug)]
pub struct EditEvent {
    pub line: u32,
    pub col: u32,
    pub old_text: String,
    pub new_text: String,
    pub elapsed_ms: u64,
}

/// Predicted next location and replacement text.
#[derive(Serialize, Debug)]
pub struct NextEditPrediction {
    pub target_line: u32,
    pub target_col: u32,
    pub suggested_text: String,
    pub confidence: f32,
}

/// Predict the next edit a developer will make after a series of recent changes.
///
/// Sends the recent edit history to the fast model and parses its JSON response.
/// Returns `None` if the model has no high-confidence prediction.
#[tauri::command]
pub async fn predict_next_edit(
    state: tauri::State<'_, AppState>,
    current_file: String,
    content: String,
    cursor_line: u32,
    _cursor_col: u32,
    recent_edits: Vec<EditEvent>,
    provider: String,
) -> Result<Option<NextEditPrediction>, String> {
    // Build the prediction prompt
    let mut edit_lines = String::new();
    for (i, edit) in recent_edits.iter().enumerate() {
        edit_lines.push_str(&format!(
            "{}. Line {}, col {}: {:?} → {:?} ({}ms ago)\n",
            i + 1, edit.line + 1, edit.col + 1,
            edit.old_text, edit.new_text, edit.elapsed_ms
        ));
    }

    // Count occurrences of old text still in content (to show remaining locations)
    let first_old = recent_edits.first().map(|e| e.old_text.as_str()).unwrap_or("");
    let remaining_count = if first_old.is_empty() {
        0
    } else {
        content.matches(first_old).count()
    };

    let prompt = format!(
        "Recent edits in `{}`:\n{}\n\
        The text {:?} still appears {} more time(s) in the file at cursor line {}.\n\
        Predict the next edit the developer will make. \
        Respond ONLY with JSON (no markdown, no explanation):\n\
        {{\"line\": <0-indexed line>, \"col\": <0-indexed col>, \"replacement\": \"<text>\", \"confidence\": <0.0-1.0>}}\n\
        If you have no confident prediction, respond: {{\"confidence\": 0.0}}",
        current_file, edit_lines,
        first_old, remaining_count,
        cursor_line + 1,
    );

    let messages = vec![
        vibe_ai::Message {
            role: vibe_ai::MessageRole::System,
            content: "You are a next-edit prediction engine. Output only valid JSON.".to_string(),
        },
        vibe_ai::Message {
            role: vibe_ai::MessageRole::User,
            content: prompt,
        },
    ];

    let mut engine = state.chat_engine.lock().await;
    if !provider.is_empty() {
        let _ = engine.set_provider_by_name(&provider);
    }
    let response = engine.chat(&messages, None).await.map_err(|e| e.to_string())?;
    drop(engine);

    // Parse the JSON response
    let raw = response.trim();
    // Strip markdown code fences if present
    let json_str = if raw.starts_with("```") {
        raw.lines()
            .filter(|l| !l.starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        raw.to_string()
    };

    #[derive(Deserialize)]
    struct RawPrediction {
        #[serde(default)]
        line: Option<u32>,
        #[serde(default)]
        col: Option<u32>,
        #[serde(default)]
        replacement: Option<String>,
        confidence: f32,
    }

    match serde_json::from_str::<RawPrediction>(&json_str) {
        Ok(pred) if pred.confidence >= 0.5 => {
            if let (Some(line), Some(col), Some(replacement)) = (pred.line, pred.col, pred.replacement) {
                Ok(Some(NextEditPrediction {
                    target_line: line,
                    target_col: col,
                    suggested_text: replacement,
                    confidence: pred.confidence,
                }))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

// ─── Inline Edit (Cmd+K) ──────────────────────────────────────────────────────

/// AI-powered inline edit: given a selected code range and an instruction,
/// return the replacement text to apply.
#[tauri::command]
pub async fn inline_edit(
    state: tauri::State<'_, AppState>,
    file_path: String,
    language: String,
    selected_text: String,
    start_line: u32,
    end_line: u32,
    instruction: String,
    provider: String,
) -> Result<String, String> {
    let prompt = format!(
        "You are an expert code editor. \
         Edit the following {language} code according to the instruction.\n\n\
         File: {file_path}\n\
         Lines: {}-{}\n\n\
         === SELECTED CODE ===\n{selected_text}\n=== END CODE ===\n\n\
         Instruction: {instruction}\n\n\
         Respond ONLY with the replacement code (no markdown fences, no explanation). \
         Preserve the original indentation.",
        start_line + 1,
        end_line + 1,
    );

    let messages = vec![vibe_ai::provider::Message {
        role: vibe_ai::provider::MessageRole::User,
        content: prompt,
    }];

    let mut chat_engine = state.chat_engine.lock().await;
    if !provider.is_empty() {
        let _ = chat_engine.set_provider_by_name(&provider);
    }
    chat_engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

// ─── Phase 5 — Trace / History Commands ───────────────────────────────────────

#[derive(Serialize)]
pub struct TraceSessionInfo {
    pub session_id: String,
    pub timestamp: u64,
    pub step_count: usize,
}

#[derive(Serialize)]
pub struct TraceEntryInfo {
    pub timestamp: u64,
    pub session_id: String,
    pub step: usize,
    pub tool: String,
    pub input_summary: String,
    pub output: String,
    pub success: bool,
    pub duration_ms: u64,
    pub approved_by: String,
}

fn vibeui_trace_dir() -> std::path::PathBuf {
    let base = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join(".vibeui").join("traces")
}

/// List all agent trace sessions for the HistoryPanel.
#[tauri::command]
pub async fn list_trace_sessions() -> Result<Vec<TraceSessionInfo>, String> {
    let dir = vibeui_trace_dir();
    let sessions = vibe_ai::list_traces(&dir);
    Ok(sessions
        .into_iter()
        .map(|s| TraceSessionInfo {
            session_id: s.session_id,
            timestamp: s.timestamp,
            step_count: s.step_count,
        })
        .collect())
}

/// Load all entries from a specific trace session.
#[tauri::command]
pub async fn load_trace_session(session_id: String) -> Result<Vec<TraceEntryInfo>, String> {
    let dir = vibeui_trace_dir();
    let path = dir.join(format!("{}.jsonl", session_id));
    let entries = vibe_ai::load_trace(&path);
    Ok(entries
        .into_iter()
        .map(|e| TraceEntryInfo {
            timestamp: e.timestamp,
            session_id: e.session_id,
            step: e.step,
            tool: e.tool,
            input_summary: e.input_summary,
            output: e.output,
            success: e.success,
            duration_ms: e.duration_ms,
            approved_by: e.approved_by,
        })
        .collect())
}

// ── Phase 8 (extra) — Hooks Config UI ─────────────────────────────────────────

/// A simplified hook config descriptor for the UI (avoids exposing internal enum variants).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfigUi {
    pub event: String,
    #[serde(default)]
    pub tools: Vec<String>,
    /// "command" or "llm"
    pub handler_type: String,
    /// Shell command string (for handler_type == "command")
    pub command: String,
    /// LLM prompt template (for handler_type == "llm")
    pub prompt: String,
    #[serde(default)]
    pub async_exec: bool,
}

fn hooks_config_path(workspace_path: Option<&str>) -> std::path::PathBuf {
    if let Some(ws) = workspace_path {
        if !ws.is_empty() {
            return std::path::PathBuf::from(ws).join(".vibecli").join("hooks.json");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibecli").join("hooks.json")
}

/// Load hooks configuration for the Hooks Config UI panel.
#[tauri::command]
pub async fn get_hooks_config(workspace_path: Option<String>) -> Result<Vec<HookConfigUi>, String> {
    let path = hooks_config_path(workspace_path.as_deref());
    if !path.exists() {
        return Ok(vec![]);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Save hooks configuration from the Hooks Config UI panel.
#[tauri::command]
pub async fn save_hooks_config(
    hooks: Vec<HookConfigUi>,
    workspace_path: Option<String>,
) -> Result<(), String> {
    let path = hooks_config_path(workspace_path.as_deref());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&hooks).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

// ── Phase 9.1 — Manager View (Parallel Agent Orchestration) ──────────────────

/// Describes one running or completed parallel agent instance.
#[derive(Debug, Clone, Serialize)]
pub struct AgentInstanceInfo {
    pub id: String,
    pub task: String,
    /// "pending" | "running" | "done" | "failed"
    pub status: String,
    pub step_count: usize,
    pub branch: String,
    pub worktree_path: String,
}

/// A task spec submitted to the parallel orchestrator.
#[derive(Debug, Deserialize)]
pub struct ParallelAgentTask {
    pub id: String,
    pub task: String,
    /// Optional list of task IDs this one depends on (reserved for future dependency tracking).
    #[serde(default)]
    pub _depends_on: Vec<String>,
}

/// Spawn multiple parallel agents for the Manager View.
///
/// Emits Tauri events:
/// - `manager:agent_update` → `AgentInstanceInfo`   (status change)
/// - `manager:agent_step`   → `{id, step_num, tool}` (per-step progress)
#[tauri::command]
pub async fn start_parallel_agents(
    tasks: Vec<ParallelAgentTask>,
    provider: String,
    approval_policy: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AgentInstanceInfo>, String> {
    use vibe_ai::{AgentLoop, AgentContext, ApprovalPolicy, AgentEvent};
    use crate::agent_executor::TauriToolExecutor;

    if tasks.is_empty() {
        return Ok(vec![]);
    }

    let workspace_root = {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned().unwrap_or_else(|| PathBuf::from("."))
    };

    let provider_arc = {
        let mut engine = state.chat_engine.lock().await;
        engine.set_provider_by_name(&provider).map_err(|e| e.to_string())?;
        engine.active_provider().ok_or("No active provider")?.clone()
    };

    let approval = ApprovalPolicy::from_str(&approval_policy);

    // Build initial status list for the UI
    let mut instances: Vec<AgentInstanceInfo> = tasks
        .iter()
        .map(|t| AgentInstanceInfo {
            id: t.id.clone(),
            task: t.task.clone(),
            status: "pending".to_string(),
            step_count: 0,
            branch: format!("agent/{}", t.id),
            worktree_path: workspace_root
                .join(".agent-worktrees")
                .join(&t.id)
                .to_string_lossy()
                .into_owned(),
        })
        .collect();

    let handle = app_handle.clone();
    let root = workspace_root.clone();
    let prov = provider_arc.clone();

    // Spawn agents concurrently — one independent AgentLoop per task
    for (i, task) in tasks.iter().enumerate() {
        let task_id = task.id.clone();
        let task_desc = task.task.clone();
        let prov2 = prov.clone();
        let root2 = root.clone();
        let approval2 = approval.clone();
        let h2 = handle.clone();

        // Emit "running" status
        instances[i].status = "running".to_string();
        let _ = handle.emit("manager:agent_update", instances[i].clone());

        tokio::spawn(async move {
            let git_branch = vibe_core::git::get_current_branch(&root2).ok();
            let context = AgentContext {
                workspace_root: root2.clone(),
                open_files: vec![],
                git_branch,
                git_diff_summary: None,
                flow_context: None,
                approved_plan: None,
                extra_skill_dirs: vec![],
                parent_session_id: None,
                depth: 0,
                active_agent_counter: None,
            };

            let executor = Arc::new(TauriToolExecutor::new(root2));
            let agent = AgentLoop::new(prov2, approval2, executor);
            let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<AgentEvent>(64);

            let tid = task_id.clone();
            let h3 = h2.clone();

            tokio::spawn(async move {
                let _ = agent.run(&task_desc, context, event_tx).await;
            });

            let mut step_count: usize = 0;
            while let Some(event) = event_rx.recv().await {
                match event {
                    AgentEvent::ToolCallExecuted(step) => {
                        step_count += 1;
                        let payload = serde_json::json!({
                            "id": &tid,
                            "step_num": step.step_num,
                            "tool": step.tool_call.name(),
                            "success": step.tool_result.success,
                        });
                        let _ = h3.emit("manager:agent_step", payload);
                    }
                    AgentEvent::Complete(_) => {
                        let update = AgentInstanceInfo {
                            id: tid.clone(),
                            task: String::new(),
                            status: "done".to_string(),
                            step_count,
                            branch: format!("agent/{}", &tid),
                            worktree_path: String::new(),
                        };
                        let _ = h3.emit("manager:agent_update", update);
                        break;
                    }
                    AgentEvent::Error(msg) => {
                        let update = AgentInstanceInfo {
                            id: tid.clone(),
                            task: msg,
                            status: "failed".to_string(),
                            step_count,
                            branch: format!("agent/{}", &tid),
                            worktree_path: String::new(),
                        };
                        let _ = h3.emit("manager:agent_update", update);
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    Ok(instances)
}

/// Retrieve current status of all spawned parallel agents.
///
/// Since agents run as background tasks, this returns the last-known status
/// from the emitted Tauri events. The frontend maintains the live state;
/// this command provides a snapshot for initial render.
#[tauri::command]
pub async fn get_orchestrator_status(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AgentInstanceInfo>, String> {
    // The live state is maintained by the frontend via manager:agent_update events.
    // Return empty list — the frontend builds its view from events.
    let _ = state;
    Ok(vec![])
}

/// Merge a completed agent's worktree branch into the main branch.
///
/// Strategy: "merge" (default) | "squash" | "rebase"
#[tauri::command]
pub async fn merge_agent_branch(
    agent_id: String,
    strategy: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let workspace_root = {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned().unwrap_or_else(|| PathBuf::from("."))
    };

    let branch = format!("agent/{}", agent_id);

    // Use git CLI to perform the merge
    let merge_args: Vec<&str> = match strategy.as_str() {
        "squash" => vec!["merge", "--squash", &branch],
        "rebase" => vec!["rebase", &branch],
        _ => vec!["merge", "--no-ff", &branch],
    };

    let output = std::process::Command::new("git")
        .args(&merge_args)
        .current_dir(&workspace_root)
        .output()
        .map_err(|e| format!("git error: {e}"))?;

    if output.status.success() {
        Ok(format!(
            "Merged branch '{}' using '{}' strategy",
            branch, strategy
        ))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// ── Code Review ───────────────────────────────────────────────────────────────

/// Run an AI-powered code review and return a structured report.
#[tauri::command]
pub async fn run_code_review(
    state: tauri::State<'_, AppState>,
    workspace_path: String,
    base_ref: Option<String>,
    target_ref: Option<String>,
) -> Result<serde_json::Value, String> {
    let workspace = PathBuf::from(&workspace_path);

    // Get git diff
    let base = base_ref.as_deref().unwrap_or("");
    let diff_args: Vec<&str> = if base.is_empty() {
        vec!["diff", "HEAD"]
    } else {
        vec!["diff", base, target_ref.as_deref().unwrap_or("HEAD")]
    };

    let diff_output = std::process::Command::new("git")
        .args(&diff_args)
        .current_dir(&workspace)
        .output()
        .map_err(|e| format!("git error: {e}"))?;

    let diff = String::from_utf8_lossy(&diff_output.stdout).to_string();
    if diff.trim().is_empty() {
        return Err("No diff found. Make sure there are uncommitted changes or specify a valid base ref.".to_string());
    }

    // Truncate very large diffs
    let diff_for_review = if diff.len() > 20_000 {
        format!("{}\n...(diff truncated at 20k chars)", &diff[..20_000])
    } else {
        diff
    };

    // Get the active AI provider from the chat engine.
    let provider = {
        let engine = state.chat_engine.lock().await;
        engine.active_provider().ok_or("No active AI provider. Set a provider first.")?.clone()
    };

    let review_prompt = format!(
        r#"You are an expert code reviewer. Analyze this git diff and produce a structured review.

Respond ONLY with a JSON object matching this exact schema:
{{
  "summary": "2-3 sentence summary of the changes",
  "issues": [
    {{
      "file": "path/to/file.rs",
      "line": 42,
      "severity": "critical|warning|info",
      "category": "security|performance|correctness|style|testing",
      "description": "What is wrong",
      "suggested_fix": "How to fix it (optional)"
    }}
  ],
  "suggestions": [
    {{ "description": "General suggestion", "file": "optional/file.rs" }}
  ],
  "score": {{
    "overall": 7.5,
    "correctness": 8.0,
    "security": 9.0,
    "performance": 7.0,
    "style": 6.5
  }},
  "files_reviewed": ["list", "of", "files"]
}}

Scores are 0–10 (10 = excellent). Only report real issues.

## Git Diff
```diff
{}
```"#,
        diff_for_review
    );

    let messages = vec![
        vibe_ai::provider::Message {
            role: vibe_ai::provider::MessageRole::User,
            content: review_prompt,
        },
    ];

    let response = provider
        .chat(&messages, None)
        .await
        .map_err(|e| format!("AI provider error: {e}"))?;

    // Extract JSON from the response (strip markdown code fences if present)
    let json_str = extract_json(&response);
    let mut report: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse review JSON: {e}\n\nRaw: {}", &response[..response.len().min(500)]))?;

    // Inject refs for display
    report["base_ref"] = serde_json::Value::String(base_ref.unwrap_or_default());
    report["target_ref"] = serde_json::Value::String(target_ref.unwrap_or_else(|| "HEAD".to_string()));

    Ok(report)
}

/// Open a URL in the system's default browser using the Tauri opener plugin.
#[tauri::command]
pub async fn open_external_url(
    url: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Only http:// and https:// URLs are supported.".to_string());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())
}

// ── Symbol + Codebase Search Commands ─────────────────────────────────────────

#[derive(Serialize)]
pub struct SymbolResult {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
}

/// Search the workspace codebase for symbols matching `query` (substring/fuzzy).
/// Returns up to 20 results, sorted by relevance.
#[tauri::command]
pub async fn search_workspace_symbols(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SymbolResult>, String> {
    let root = {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned().ok_or("No workspace folder open")?
    };
    let mut idx = vibe_core::index::CodebaseIndex::new(root.clone());
    idx.build().map_err(|e| e.to_string())?;
    let hits = idx.search_symbols(&query);
    Ok(hits.into_iter().take(20).map(|s| SymbolResult {
        name: s.name,
        kind: format!("{:?}", s.kind),
        file: s.file.strip_prefix(&root).unwrap_or(&s.file)
            .to_string_lossy().into_owned(),
        line: s.line,
    }).collect())
}

/// Semantic search over the workspace.
///
/// 1. If `.vibeui/embeddings/index.json` exists in the workspace root, loads the
///    `EmbeddingIndex` and performs cosine-similarity search.
/// 2. Otherwise falls back to fast keyword/symbol search via `CodebaseIndex`.
#[tauri::command]
pub async fn semantic_search_codebase(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SymbolResult>, String> {
    let k = limit.unwrap_or(8).min(20);
    let root = {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned().ok_or("No workspace folder open")?
    };

    // ── Try embedding search first ────────────────────────────────────────────
    let index_path = root.join(".vibeui").join("embeddings").join("index.json");
    if index_path.exists() {
        use vibe_core::index::embeddings::EmbeddingIndex;
        match EmbeddingIndex::load(&index_path) {
            Ok(idx) => {
                match idx.search(&query, k).await {
                    Ok(hits) => {
                        return Ok(hits.into_iter().map(|h| {
                            let rel = h.file.strip_prefix(&root).unwrap_or(&h.file)
                                .to_string_lossy().into_owned();
                            SymbolResult {
                                name: h.text.lines().next().unwrap_or("").trim()
                                    .chars().take(80).collect(),
                                kind: format!("snippet (score {:.2})", h.score),
                                file: rel,
                                line: h.chunk_start,
                            }
                        }).collect());
                    }
                    Err(e) => {
                        eprintln!("[vibeui] EmbeddingIndex search failed ({}); falling back to keyword", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[vibeui] Could not load embedding index ({}); falling back to keyword", e);
            }
        }
    }

    // ── Keyword/symbol fallback ───────────────────────────────────────────────
    let mut idx2 = vibe_core::index::CodebaseIndex::new(root.clone());
    idx2.build().map_err(|e| e.to_string())?;
    let hits = idx2.search_symbols(&query);
    Ok(hits.into_iter().take(k).map(|s| SymbolResult {
        name: s.name,
        kind: format!("{:?}", s.kind),
        file: s.file.strip_prefix(&root).unwrap_or(&s.file).to_string_lossy().into_owned(),
        line: s.line,
    }).collect())
}

/// Build (or rebuild) the workspace embedding index.
///
/// Saves to `<workspace>/.vibeui/embeddings/index.json`.
/// `provider`: `"ollama"` (default) or `"openai"`.
/// `model`: embedding model name (default `"nomic-embed-text"` for Ollama).
#[tauri::command]
pub async fn build_embedding_index(
    provider: Option<String>,
    model: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    use vibe_core::index::embeddings::{EmbeddingIndex, EmbeddingProvider};

    let root = {
        let ws = state.workspace.lock().await;
        ws.folders().first().cloned().ok_or("No workspace folder open")?
    };

    let embedding_provider = match provider.as_deref().unwrap_or("ollama") {
        "openai" => {
            let key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| "OPENAI_API_KEY env var not set".to_string())?;
            EmbeddingProvider::openai(key)
        }
        _ => EmbeddingProvider::ollama(
            model.as_deref().unwrap_or("nomic-embed-text"),
        ),
    };

    let idx = EmbeddingIndex::build(&root, &embedding_provider)
        .await
        .map_err(|e| e.to_string())?;

    let chunk_count = idx.chunk_count();
    let file_count = idx.file_count();

    let index_path = root.join(".vibeui").join("embeddings").join("index.json");
    idx.save(&index_path).map_err(|e| e.to_string())?;

    Ok(format!(
        "Built embedding index: {} files, {} chunks → {}",
        file_count, chunk_count, index_path.display()
    ))
}

// ── @docs context ─────────────────────────────────────────────────────────────

/// Fetch library documentation from an online registry.
///
/// `registry`: `"rs"` (docs.rs), `"npm"` (npmjs.com), `"py"` (PyPI).
/// Returns a plain-text summary (max 4000 chars) suitable for AI injection.
#[tauri::command]
pub async fn fetch_doc_content(name: String, registry: String) -> Result<String, String> {
    let url = match registry.as_str() {
        "rs" => format!("https://docs.rs/crate/{}/latest/", name),
        "npm" => format!("https://registry.npmjs.org/{}", name),
        "py" => format!("https://pypi.org/pypi/{}/json", name),
        _ => return Err(format!("Unknown registry: {}", registry)),
    };

    let body = fetch_and_strip(&url).await.map_err(|e| e.to_string())?;

    // For JSON registries, try to extract meaningful fields
    if registry == "npm" || registry == "py" {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            let summary = if registry == "npm" {
                let desc = json["description"].as_str().unwrap_or("").to_string();
                let ver  = json["dist-tags"]["latest"].as_str().unwrap_or("?");
                let kws: Vec<&str> = json["keywords"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();
                format!(
                    "Package: {} (npm v{})\n{}\nKeywords: {}",
                    name, ver, desc, kws.join(", ")
                )
            } else {
                // PyPI JSON
                let info = &json["info"];
                let desc = info["summary"].as_str().unwrap_or("").to_string();
                let ver  = info["version"].as_str().unwrap_or("?");
                format!("Package: {} (PyPI v{})\n{}", name, ver, desc)
            };
            return Ok(summary.chars().take(4000).collect());
        }
    }

    // For docs.rs, return stripped HTML text
    Ok(body.chars().take(4000).collect())
}

// ── Linter integration ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct LintErrorOut {
    pub line: usize,
    pub col: usize,
    pub severity: String,
    pub message: String,
    pub rule: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LintResultOut {
    pub errors: Vec<LintErrorOut>,
    pub warnings: Vec<LintErrorOut>,
    pub raw_output: String,
    pub linter_available: bool,
}

/// Run the appropriate linter for `file_path` and return parsed results.
///
/// Supported linters:
/// - `eslint` — TypeScript/JavaScript
/// - `cargo-check` — Rust (fast, no borrow-checker bypass)
/// - `flake8` — Python
/// - `go-vet` — Go
#[tauri::command]
pub async fn run_linter(file_path: String, linter: String) -> Result<LintResultOut, String> {
    use std::process::Command;

    let path = std::path::Path::new(&file_path);
    let dir  = path.parent().unwrap_or(std::path::Path::new("."));

    let (prog, args, parse_mode) = match linter.as_str() {
        "eslint" => (
            "eslint",
            vec!["--format", "json", &file_path],
            "eslint-json",
        ),
        "cargo-check" => (
            "cargo",
            vec!["check", "--message-format=json", "--quiet"],
            "cargo-json",
        ),
        "flake8" => (
            "flake8",
            vec!["--format=%(path)s:%(row)d:%(col)d:%(code)s:%(text)s", &file_path],
            "flake8-text",
        ),
        "go-vet" => (
            "go",
            vec!["vet", &file_path],
            "go-text",
        ),
        _ => return Ok(LintResultOut {
            errors: vec![], warnings: vec![],
            raw_output: format!("Unknown linter: {}", linter),
            linter_available: false,
        }),
    };

    let output = match Command::new(prog)
        .args(&args)
        .current_dir(dir)
        .output()
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LintResultOut {
                errors: vec![], warnings: vec![],
                raw_output: format!("{} not found in PATH", prog),
                linter_available: false,
            });
        }
        Err(e) => return Err(e.to_string()),
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let raw_output = if stdout.is_empty() { stderr.clone() } else { stdout.clone() };

    let (errors, warnings) = match parse_mode {
        "eslint-json" => parse_eslint_json(&stdout),
        "cargo-json"  => parse_cargo_json(&stdout),
        "flake8-text" => parse_flake8_text(&stdout),
        _             => parse_generic_text(&stderr),
    };

    Ok(LintResultOut { errors, warnings, raw_output, linter_available: true })
}

fn parse_eslint_json(output: &str) -> (Vec<LintErrorOut>, Vec<LintErrorOut>) {
    let mut errors = vec![];
    let mut warnings = vec![];
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(files) = json.as_array() {
            for file in files {
                if let Some(msgs) = file["messages"].as_array() {
                    for msg in msgs {
                        let sev = if msg["severity"].as_u64().unwrap_or(2) == 1 { "warning" } else { "error" };
                        let item = LintErrorOut {
                            line: msg["line"].as_u64().unwrap_or(1) as usize,
                            col: msg["column"].as_u64().unwrap_or(1) as usize,
                            severity: sev.to_string(),
                            message: msg["message"].as_str().unwrap_or("").to_string(),
                            rule: msg["ruleId"].as_str().map(|s| s.to_string()),
                        };
                        if sev == "error" { errors.push(item); } else { warnings.push(item); }
                    }
                }
            }
        }
    }
    (errors, warnings)
}

fn parse_cargo_json(output: &str) -> (Vec<LintErrorOut>, Vec<LintErrorOut>) {
    let mut errors = vec![];
    let mut warnings = vec![];
    for line in output.lines() {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) {
            if msg["reason"].as_str() != Some("compiler-message") { continue; }
            let level = msg["message"]["level"].as_str().unwrap_or("");
            let text  = msg["message"]["message"].as_str().unwrap_or("").to_string();
            let spans = msg["message"]["spans"].as_array();
            let (line_no, col_no) = spans
                .and_then(|s| s.first())
                .map(|sp| (
                    sp["line_start"].as_u64().unwrap_or(1) as usize,
                    sp["column_start"].as_u64().unwrap_or(1) as usize,
                ))
                .unwrap_or((1, 1));
            let item = LintErrorOut {
                line: line_no, col: col_no,
                severity: if level == "error" { "error" } else { "warning" }.to_string(),
                message: text, rule: None,
            };
            if level == "error" { errors.push(item); } else { warnings.push(item); }
        }
    }
    (errors, warnings)
}

fn parse_flake8_text(output: &str) -> (Vec<LintErrorOut>, Vec<LintErrorOut>) {
    let mut errors = vec![];
    let mut warnings = vec![];
    for line in output.lines() {
        // Format: path:row:col:code:text
        let parts: Vec<&str> = line.splitn(5, ':').collect();
        if parts.len() < 5 { continue; }
        let row: usize = parts[1].trim().parse().unwrap_or(1);
        let col: usize = parts[2].trim().parse().unwrap_or(1);
        let code = parts[3].trim();
        let msg  = parts[4].trim().to_string();
        let is_error = code.starts_with('E');
        let item = LintErrorOut {
            line: row, col,
            severity: if is_error { "error" } else { "warning" }.to_string(),
            message: msg,
            rule: Some(code.to_string()),
        };
        if is_error { errors.push(item); } else { warnings.push(item); }
    }
    (errors, warnings)
}

fn parse_generic_text(output: &str) -> (Vec<LintErrorOut>, Vec<LintErrorOut>) {
    let errors: Vec<LintErrorOut> = output.lines()
        .filter(|l| !l.is_empty())
        .take(20)
        .map(|l| LintErrorOut {
            line: 1, col: 1,
            severity: "error".to_string(),
            message: l.to_string(),
            rule: None,
        })
        .collect();
    (errors, vec![])
}

// ── BYOK Settings ─────────────────────────────────────────────────────────────

fn api_keys_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibeui").join("api_keys.json")
}

/// API key settings for cloud providers, stored at `~/.vibeui/api_keys.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApiKeySettings {
    #[serde(default)]
    pub anthropic_api_key: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub gemini_api_key: String,
    #[serde(default)]
    pub grok_api_key: String,
    #[serde(default)]
    pub claude_model: String,
    #[serde(default)]
    pub openai_model: String,
}

/// Load API key settings from `~/.vibeui/api_keys.json`.
#[tauri::command]
pub async fn get_provider_api_keys() -> Result<ApiKeySettings, String> {
    let path = api_keys_path();
    if !path.exists() {
        return Ok(ApiKeySettings::default());
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Save API key settings and re-register cloud providers in the chat engine.
#[tauri::command]
pub async fn save_provider_api_keys(
    settings: ApiKeySettings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Persist to disk
    let path = api_keys_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    // Re-register cloud providers in the chat engine
    let mut engine = state.chat_engine.lock().await;
    engine.clear_cloud_providers();

    if !settings.anthropic_api_key.is_empty() {
        let model = if settings.claude_model.is_empty() {
            "claude-3-5-sonnet-latest".to_string()
        } else {
            settings.claude_model.clone()
        };
        let config = vibe_ai::provider::ProviderConfig {
            provider_type: "claude".to_string(),
            api_key: Some(settings.anthropic_api_key.clone()),
            model,
            api_url: None,
            max_tokens: None,
            temperature: None,
            ..Default::default()
        };
        let provider = vibe_ai::providers::claude::ClaudeProvider::new(config);
        engine.add_provider(Arc::new(provider));
    }

    if !settings.openai_api_key.is_empty() {
        let model = if settings.openai_model.is_empty() {
            "gpt-4o".to_string()
        } else {
            settings.openai_model.clone()
        };
        let config = vibe_ai::provider::ProviderConfig {
            provider_type: "openai".to_string(),
            api_key: Some(settings.openai_api_key.clone()),
            model,
            api_url: None,
            max_tokens: None,
            temperature: None,
            ..Default::default()
        };
        let provider = vibe_ai::providers::openai::OpenAIProvider::new(config);
        engine.add_provider(Arc::new(provider));
    }

    if !settings.gemini_api_key.is_empty() {
        let config = vibe_ai::provider::ProviderConfig {
            provider_type: "gemini".to_string(),
            api_key: Some(settings.gemini_api_key.clone()),
            model: "gemini-2.0-flash".to_string(),
            api_url: None,
            max_tokens: None,
            temperature: None,
            ..Default::default()
        };
        let provider = vibe_ai::providers::gemini::GeminiProvider::new(config);
        engine.add_provider(Arc::new(provider));
    }

    if !settings.grok_api_key.is_empty() {
        let config = vibe_ai::provider::ProviderConfig {
            provider_type: "grok".to_string(),
            api_key: Some(settings.grok_api_key.clone()),
            model: "grok-2-latest".to_string(),
            api_url: None,
            max_tokens: None,
            temperature: None,
            ..Default::default()
        };
        let provider = vibe_ai::providers::grok::GrokProvider::new(config);
        engine.add_provider(Arc::new(provider));
    }

    Ok(())
}

// ── Spec commands ─────────────────────────────────────────────────────────────

/// Serializable spec task for frontend.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpecTaskDto {
    pub id: u32,
    pub description: String,
    pub done: bool,
}

/// Serializable spec for frontend.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpecDto {
    pub name: String,
    pub status: String,
    pub requirements: String,
    pub tasks: Vec<SpecTaskDto>,
    pub body: String,
    pub source: String,
}

fn specs_dir(workspace_path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(workspace_path).join(".vibecli").join("specs")
}

fn parse_spec_file(path: &std::path::Path) -> Option<SpecDto> {
    let raw = std::fs::read_to_string(path).ok()?;
    let name = path.file_stem()?.to_str()?.to_string();
    let mut status = "draft".to_string();
    let mut requirements = String::new();
    let mut body = raw.clone();
    let mut tasks: Vec<SpecTaskDto> = vec![];

    if let Some(after_prefix) = raw.strip_prefix("---") {
        let after_open = after_prefix.trim_start_matches('\n');
        if let Some(close_pos) = after_open.find("\n---") {
            let fm = &after_open[..close_pos];
            body = after_open[close_pos..].trim_start_matches("\n---").trim_start().to_string();
            for line in fm.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let val = v.trim().trim_matches('"').trim_matches('\'');
                    match k.trim() {
                        "status" => status = val.to_string(),
                        "requirements" => requirements = val.to_string(),
                        _ => {}
                    }
                }
            }
        }
    }

    // Parse task list
    for line in body.lines() {
        let line = line.trim();
        let rest_opt = line.strip_prefix("- [x] ").or_else(|| line.strip_prefix("- [ ] "));
        if let Some(rest) = rest_opt {
            let done = line.starts_with("- [x]");
            let rest = rest.trim();
            let (id, desc) = if let Some(stripped) = rest.strip_prefix("**") {
                if let Some(idx) = stripped.find("**:") {
                    let id_str = &stripped[..idx];
                    let desc = stripped[idx + 3..].trim();
                    (id_str.parse::<u32>().unwrap_or(tasks.len() as u32 + 1), desc.to_string())
                } else {
                    (tasks.len() as u32 + 1, rest.to_string())
                }
            } else {
                (tasks.len() as u32 + 1, rest.to_string())
            };
            tasks.push(SpecTaskDto { id, description: desc, done });
        }
    }

    Some(SpecDto {
        name,
        status,
        requirements,
        tasks,
        body,
        source: path.display().to_string(),
    })
}

fn save_spec_file(workspace_path: &str, spec: &SpecDto) -> Result<(), String> {
    let dir = specs_dir(workspace_path);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tasks_md: String = spec.tasks.iter().map(|t| {
        let check = if t.done { "x" } else { " " };
        format!("- [{}] **{}**: {}\n", check, t.id, t.description)
    }).collect();
    let content = format!(
        "---\nname: {}\nstatus: {}\nrequirements: {}\n---\n\n{}\n\n## Tasks\n\n{}",
        spec.name, spec.status, spec.requirements, spec.body, tasks_md
    );
    let path = dir.join(format!("{}.md", spec.name));
    std::fs::write(path, content).map_err(|e| e.to_string())
}

/// List all specs in the workspace `.vibecli/specs/` directory.
#[tauri::command]
pub async fn list_specs(workspace_path: String) -> Result<Vec<SpecDto>, String> {
    let dir = specs_dir(&workspace_path);
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut specs: Vec<SpecDto> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
        .filter_map(|e| parse_spec_file(&e.path()))
        .collect();
    specs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(specs)
}

/// Get a single spec by name.
#[tauri::command]
pub async fn get_spec(workspace_path: String, name: String) -> Result<SpecDto, String> {
    let path = specs_dir(&workspace_path).join(format!("{}.md", name));
    parse_spec_file(&path).ok_or_else(|| format!("Spec '{}' not found", name))
}

/// Generate a new spec from requirements using the LLM.
#[tauri::command]
pub async fn generate_spec(
    workspace_path: String,
    name: String,
    requirements: String,
    provider: String,
    state: tauri::State<'_, AppState>,
) -> Result<SpecDto, String> {
    let prompt = format!(
        r#"You are a software architect. Generate a spec document for:

Requirements: {requirements}

Output a markdown document with these sections:
1. ## User Stories (Given/When/Then format)
2. ## Acceptance Criteria (bullet list)
3. ## Technical Design (architecture decisions, key files to change)
4. ## Tasks (5-10 atomic items formatted as: `- [ ] **N**: task description`)

Be concise and focus on implementable tasks. Start directly with the content (no front-matter)."#
    );

    let messages = vec![vibe_ai::Message {
        role: vibe_ai::MessageRole::User,
        content: prompt,
    }];

    let body = {
        let mut engine = state.chat_engine.lock().await;
        let _ = engine.set_provider_by_name(&provider); // ignore if not found, use active
        engine.chat(&messages, None).await.map_err(|e| e.to_string())?
    };

    let mut spec = SpecDto {
        name: name.clone(),
        status: "draft".to_string(),
        requirements: requirements.clone(),
        tasks: vec![],
        body: body.clone(),
        source: specs_dir(&workspace_path).join(format!("{}.md", name)).display().to_string(),
    };

    // Parse tasks from generated body
    let mut task_id = 1u32;
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with("- [ ]") || line.starts_with("- [x]") {
            let done = line.starts_with("- [x]");
            let rest = &line[5..].trim();
            let desc = if let Some(stripped) = rest.strip_prefix("**") {
                if let Some(idx) = stripped.find("**:") {
                    stripped[idx + 3..].trim().to_string()
                } else {
                    rest.to_string()
                }
            } else {
                rest.to_string()
            };
            spec.tasks.push(SpecTaskDto { id: task_id, description: desc, done });
            task_id += 1;
        }
    }

    save_spec_file(&workspace_path, &spec)?;
    Ok(spec)
}

/// Toggle a task's done state in a spec.
#[tauri::command]
pub async fn update_spec_task(
    workspace_path: String,
    name: String,
    task_id: u32,
    done: bool,
) -> Result<SpecDto, String> {
    let path = specs_dir(&workspace_path).join(format!("{}.md", name));
    let mut spec = parse_spec_file(&path).ok_or_else(|| format!("Spec '{}' not found", name))?;

    if let Some(task) = spec.tasks.iter_mut().find(|t| t.id == task_id) {
        task.done = done;
    }

    // Auto-update status
    let all_done = !spec.tasks.is_empty() && spec.tasks.iter().all(|t| t.done);
    let any_done = spec.tasks.iter().any(|t| t.done);
    if all_done {
        spec.status = "done".to_string();
    } else if any_done && spec.status == "approved" {
        spec.status = "in-progress".to_string();
    }

    save_spec_file(&workspace_path, &spec)?;
    Ok(spec)
}

/// Build an agent task prompt from a spec's pending tasks.
/// The frontend should pass the returned string to `start_agent_task`.
#[tauri::command]
pub async fn run_spec(
    workspace_path: String,
    name: String,
) -> Result<String, String> {
    let path = specs_dir(&workspace_path).join(format!("{}.md", name));
    let spec = parse_spec_file(&path).ok_or_else(|| format!("Spec '{}' not found", name))?;

    let pending: Vec<String> = spec.tasks.iter()
        .filter(|t| !t.done)
        .map(|t| format!("{}. {}", t.id, t.description))
        .collect();

    if pending.is_empty() {
        return Ok(String::new()); // signals "all done" to frontend
    }

    Ok(format!(
        "Spec: {}\nRequirements: {}\n\nWork through these pending tasks in order:\n{}",
        spec.name, spec.requirements, pending.join("\n")
    ))
}

// ── Code Complete Workflow commands ──────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowChecklistItemDto {
    pub id: u32,
    pub description: String,
    pub done: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowStageDto {
    pub stage: String,
    pub label: String,
    pub status: String,
    pub checklist: Vec<WorkflowChecklistItemDto>,
    pub body: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowDto {
    pub name: String,
    pub description: String,
    pub current_stage: usize,
    pub stages: Vec<WorkflowStageDto>,
    pub created_at: String,
    pub overall_progress: f64,
}

const STAGE_LABELS: [&str; 8] = [
    "Requirements",
    "Architecture",
    "Design",
    "Construction Planning",
    "Coding",
    "Quality Assurance",
    "Integration & Testing",
    "Code Complete",
];

fn workflows_dir(workspace_path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(workspace_path)
        .join(".vibecli")
        .join("workflows")
}

fn parse_workflow_file(path: &std::path::Path) -> Option<WorkflowDto> {
    let raw = std::fs::read_to_string(path).ok()?;
    let name = path.file_stem()?.to_str()?.to_string();
    let mut description = String::new();
    let mut current_stage: usize = 0;
    let mut created_at = String::new();
    let mut body = raw.clone();

    // Parse front-matter
    if let Some(after_prefix) = raw.strip_prefix("---") {
        let after_open = after_prefix.trim_start_matches('\n');
        if let Some(close_pos) = after_open.find("\n---") {
            let fm = &after_open[..close_pos];
            body = after_open[close_pos..].trim_start_matches("\n---").trim_start().to_string();
            for line in fm.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim();
                    let val = v.trim().trim_matches('"').trim_matches('\'');
                    match key {
                        "description" => description = val.to_string(),
                        "current_stage" => current_stage = val.parse().unwrap_or(0),
                        "created_at" => created_at = val.to_string(),
                        _ => {}
                    }
                }
            }
        }
    }

    // Parse stages
    let mut stages: Vec<WorkflowStageDto> = STAGE_LABELS
        .iter()
        .map(|label| WorkflowStageDto {
            stage: label.to_string(),
            label: label.to_string(),
            status: "not-started".to_string(),
            checklist: vec![],
            body: String::new(),
        })
        .collect();

    let mut current_section: Option<usize> = None;
    let mut section_lines: Vec<String> = vec![];

    for line in body.lines() {
        if line.starts_with("## Stage: ") {
            if let Some(idx) = current_section {
                flush_workflow_stage_section(&mut stages[idx], &section_lines);
            }
            section_lines.clear();
            let label = line.strip_prefix("## Stage: ").unwrap_or("");
            current_section = STAGE_LABELS.iter().position(|l| *l == label.trim());
        } else if current_section.is_some() {
            section_lines.push(line.to_string());
        }
    }
    if let Some(idx) = current_section {
        flush_workflow_stage_section(&mut stages[idx], &section_lines);
    }

    let total: usize = stages.iter().map(|s| s.checklist.len()).sum();
    let done: usize = stages.iter().map(|s| s.checklist.iter().filter(|c| c.done).count()).sum();
    let overall_progress = if total == 0 { 0.0 } else { (done as f64 / total as f64) * 100.0 };

    Some(WorkflowDto {
        name,
        description,
        current_stage,
        stages,
        created_at,
        overall_progress,
    })
}

fn flush_workflow_stage_section(stage: &mut WorkflowStageDto, lines: &[String]) {
    let mut body_lines: Vec<String> = vec![];
    let mut in_checklist = false;

    for line in lines {
        let trimmed = line.trim();
        if let Some(inner) = trimmed.strip_prefix("<!-- status:").and_then(|s| s.strip_suffix("-->")) {
            stage.status = inner.trim().to_string();
            continue;
        }
        if trimmed == "### Checklist" {
            in_checklist = true;
            continue;
        }
        let checklist_rest = if in_checklist { trimmed.strip_prefix("- [x] ").or_else(|| trimmed.strip_prefix("- [ ] ")) } else { None };
        if let Some(rest) = checklist_rest {
            let done = trimmed.starts_with("- [x]");
            let rest = rest.trim();
            let (id, desc) = if let Some(stripped) = rest.strip_prefix("**") {
                if let Some(idx) = stripped.find("**:") {
                    let id_str = &stripped[..idx];
                    let d = stripped[idx + 3..].trim();
                    (id_str.parse::<u32>().unwrap_or(stage.checklist.len() as u32 + 1), d.to_string())
                } else {
                    (stage.checklist.len() as u32 + 1, rest.to_string())
                }
            } else {
                (stage.checklist.len() as u32 + 1, rest.to_string())
            };
            stage.checklist.push(WorkflowChecklistItemDto { id, description: desc, done });
            continue;
        }
        if in_checklist && trimmed.is_empty() && !stage.checklist.is_empty() {
            in_checklist = false;
            continue;
        }
        if !in_checklist {
            body_lines.push(line.clone());
        }
    }
    stage.body = body_lines.join("\n").trim().to_string();
}

fn save_workflow_file(workspace_path: &str, workflow: &WorkflowDto) -> Result<(), String> {
    let dir = workflows_dir(workspace_path);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.md", workflow.name));

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {}\n", workflow.name));
    out.push_str(&format!("description: {}\n", workflow.description));
    out.push_str(&format!("current_stage: {}\n", workflow.current_stage));
    out.push_str(&format!("created_at: {}\n", workflow.created_at));
    out.push_str("---\n\n");

    for stage in &workflow.stages {
        out.push_str(&format!("## Stage: {}\n", stage.label));
        out.push_str(&format!("<!-- status: {} -->\n\n", stage.status));
        if !stage.body.is_empty() {
            out.push_str(&stage.body);
            out.push_str("\n\n");
        }
        if !stage.checklist.is_empty() {
            out.push_str("### Checklist\n\n");
            for item in &stage.checklist {
                let check = if item.done { "x" } else { " " };
                out.push_str(&format!("- [{}] **{}**: {}\n", check, item.id, item.description));
            }
            out.push('\n');
        }
    }

    std::fs::write(&path, out).map_err(|e| e.to_string())
}

fn workflow_now_ts() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    format!("{}", secs)
}

/// List all workflows in the workspace.
#[tauri::command]
pub async fn list_workflows(workspace_path: String) -> Result<Vec<WorkflowDto>, String> {
    let dir = workflows_dir(&workspace_path);
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut workflows: Vec<WorkflowDto> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
        .filter_map(|e| parse_workflow_file(&e.path()))
        .collect();
    workflows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(workflows)
}

/// Get a single workflow by name.
#[tauri::command]
pub async fn get_workflow(workspace_path: String, name: String) -> Result<WorkflowDto, String> {
    let path = workflows_dir(&workspace_path).join(format!("{}.md", name));
    parse_workflow_file(&path).ok_or_else(|| format!("Workflow '{}' not found", name))
}

/// Create a new workflow with 8 Code Complete stages.
#[tauri::command]
pub async fn create_workflow(
    workspace_path: String,
    name: String,
    description: String,
) -> Result<WorkflowDto, String> {
    let now = workflow_now_ts();
    let workflow = WorkflowDto {
        name: name.clone(),
        description,
        current_stage: 0,
        stages: STAGE_LABELS.iter().enumerate().map(|(i, label)| WorkflowStageDto {
            stage: label.to_string(),
            label: label.to_string(),
            status: if i == 0 { "in-progress".to_string() } else { "not-started".to_string() },
            checklist: vec![],
            body: String::new(),
        }).collect(),
        created_at: now,
        overall_progress: 0.0,
    };
    save_workflow_file(&workspace_path, &workflow)?;
    Ok(workflow)
}

/// Advance workflow to the next stage.
#[tauri::command]
pub async fn advance_workflow_stage(
    workspace_path: String,
    name: String,
) -> Result<WorkflowDto, String> {
    let path = workflows_dir(&workspace_path).join(format!("{}.md", name));
    let mut workflow = parse_workflow_file(&path).ok_or_else(|| format!("Workflow '{}' not found", name))?;

    let idx = workflow.current_stage;
    if idx >= workflow.stages.len() {
        return Err(format!("Invalid current stage index: {}", idx));
    }
    if idx + 1 >= workflow.stages.len() {
        return Err(format!("Already at final stage '{}' — cannot advance further", workflow.stages[idx].label));
    }

    workflow.stages[idx].status = "complete".to_string();
    workflow.current_stage = idx + 1;
    workflow.stages[idx + 1].status = "in-progress".to_string();

    // Recalculate progress
    let total: usize = workflow.stages.iter().map(|s| s.checklist.len()).sum();
    let done: usize = workflow.stages.iter().map(|s| s.checklist.iter().filter(|c| c.done).count()).sum();
    workflow.overall_progress = if total == 0 { 0.0 } else { (done as f64 / total as f64) * 100.0 };

    save_workflow_file(&workspace_path, &workflow)?;
    Ok(workflow)
}

/// Toggle a checklist item in a workflow stage.
#[tauri::command]
pub async fn update_workflow_checklist_item(
    workspace_path: String,
    name: String,
    stage_index: usize,
    item_id: u32,
    done: bool,
) -> Result<WorkflowDto, String> {
    let path = workflows_dir(&workspace_path).join(format!("{}.md", name));
    let mut workflow = parse_workflow_file(&path).ok_or_else(|| format!("Workflow '{}' not found", name))?;

    if stage_index >= workflow.stages.len() {
        return Err(format!("Invalid stage index: {}", stage_index));
    }
    let stage = &mut workflow.stages[stage_index];
    if let Some(item) = stage.checklist.iter_mut().find(|c| c.id == item_id) {
        item.done = done;
    } else {
        return Err(format!("Checklist item {} not found in stage {}", item_id, stage_index));
    }

    // Auto-update stage status based on checklist completion
    if stage.checklist.iter().all(|c| c.done) && !stage.checklist.is_empty() {
        stage.status = "complete".to_string();
    } else if stage.checklist.iter().any(|c| c.done) {
        stage.status = "in-progress".to_string();
    } else if !stage.checklist.is_empty() {
        // All items unchecked — revert to in-progress
        stage.status = "in-progress".to_string();
    }

    // Recalculate progress
    let total: usize = workflow.stages.iter().map(|s| s.checklist.len()).sum();
    let done_count: usize = workflow.stages.iter().map(|s| s.checklist.iter().filter(|c| c.done).count()).sum();
    workflow.overall_progress = if total == 0 { 0.0 } else { (done_count as f64 / total as f64) * 100.0 };

    save_workflow_file(&workspace_path, &workflow)?;
    Ok(workflow)
}

/// AI-generate a checklist for a workflow stage.
#[tauri::command]
pub async fn generate_stage_checklist(
    workspace_path: String,
    name: String,
    stage_index: usize,
    provider: String,
    state: tauri::State<'_, AppState>,
) -> Result<WorkflowDto, String> {
    let path = workflows_dir(&workspace_path).join(format!("{}.md", name));
    let mut workflow = parse_workflow_file(&path).ok_or_else(|| format!("Workflow '{}' not found", name))?;

    if stage_index >= workflow.stages.len() {
        return Err(format!("Invalid stage index: {}", stage_index));
    }

    let stage_label = &STAGE_LABELS[stage_index];
    let stage_guidance = match stage_index {
        0 => "functional requirements, non-functional requirements, user stories, scope boundaries, error handling, data constraints",
        1 => "subsystem decomposition, communication strategy, data storage, error/logging strategy, security, build vs buy, scalability",
        2 => "class/module identification, API design, data structures, design patterns, coupling/cohesion, edge cases, state management",
        3 => "language/framework confirmation, coding standards, dev environment, branching strategy, integration order, CI/CD, task breakdown, risk mitigation",
        4 => "naming conventions, defensive programming, no magic numbers, short functions, DRY, straightforward control structures, WHY comments, input validation",
        5 => "code review, unit tests, integration tests, linter/static analysis, security scan, performance profiling, error handling tests, accessibility",
        6 => "module integration, end-to-end tests, regression tests, load testing, cross-platform testing, migration testing, API validation, logging/monitoring",
        7 => "all features implemented, README updated, API docs, CHANGELOG, license, no TODOs left, externalized config, version tagged, deployment runbook, monitoring plan",
        _ => "",
    };

    let prompt = format!(
        "You are a software construction expert following Steve McConnell's Code Complete methodology.\n\n\
        Generate a checklist for the **{stage_label}** stage of this project:\n\n\
        Project: {desc}\n\n\
        Include items for: {stage_guidance}\n\n\
        Output ONLY a numbered list of 8-12 specific, actionable checklist items. One per line, like:\n\
        1. Description of first item\n\
        2. Description of second item",
        stage_label = stage_label,
        desc = workflow.description,
        stage_guidance = stage_guidance,
    );

    let messages = vec![vibe_ai::Message {
        role: vibe_ai::MessageRole::User,
        content: prompt,
    }];

    let response = {
        let mut engine = state.chat_engine.lock().await;
        let _ = engine.set_provider_by_name(&provider);
        engine.chat(&messages, None).await.map_err(|e| e.to_string())?
    };

    // Parse numbered list
    let mut items: Vec<WorkflowChecklistItemDto> = vec![];
    let mut next_id = 1u32;
    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        // Match "N. text" or "N) text" or "- text"
        let desc = if let Some(rest) = trimmed.strip_prefix("- ") {
            rest.trim().to_string()
        } else {
            let mut chars = trimmed.chars().peekable();
            let mut num_s = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() { num_s.push(c); chars.next(); } else { break; }
            }
            if !num_s.is_empty() {
                if let Some(&c) = chars.peek() {
                    if c == '.' || c == ')' { chars.next(); chars.collect::<String>().trim().to_string() } else { continue; }
                } else { continue; }
            } else { continue; }
        };
        if !desc.is_empty() {
            items.push(WorkflowChecklistItemDto { id: next_id, description: desc, done: false });
            next_id += 1;
        }
    }

    if items.is_empty() {
        return Err("Could not parse checklist from AI response".to_string());
    }

    workflow.stages[stage_index].checklist = items;
    if workflow.stages[stage_index].status == "not-started" {
        workflow.stages[stage_index].status = "in-progress".to_string();
    }

    save_workflow_file(&workspace_path, &workflow)?;
    // Re-read for accurate progress
    let updated = parse_workflow_file(&path).ok_or("Failed to reload workflow")?;
    Ok(updated)
}

// ── Shadow Workspace commands ─────────────────────────────────────────────────

use crate::shadow_workspace::{LintResult, ShadowWorkspace};

/// Write proposed file content to the shadow workspace and run lint.
/// Returns the lint result so the frontend can annotate the diff.
#[tauri::command]
pub async fn shadow_write_and_lint(
    workspace_path: String,
    rel_path: String,
    content: String,
) -> Result<LintResult, String> {
    let root = std::path::Path::new(&workspace_path);
    let shadow = ShadowWorkspace::new(root).map_err(|e| e.to_string())?;
    shadow.sync_file(&rel_path, &content).map_err(|e| e.to_string())?;
    shadow.run_lint(&rel_path).map_err(|e| e.to_string())
}

/// Get a cached lint result for a file path (relative).
/// Returns null if no lint result is cached.
#[tauri::command]
pub async fn shadow_get_lint_result(
    workspace_path: String,
    rel_path: String,
) -> Option<LintResult> {
    let root = std::path::Path::new(&workspace_path);
    ShadowWorkspace::new(root).ok()
        .and_then(|sw| sw.get_lint_result(&rel_path))
}

// ── Visual Editor commands (Phase 19) ────────────────────────────────────────

/// AI-powered visual element edit.
/// Receives selected element info from inspector.js and produces an edited version.
#[tauri::command]
pub async fn visual_edit_element(
    state: tauri::State<'_, AppState>,
    _workspace_path: String,
    selector: String,
    instruction: String,
    current_html: String,
    react_component: Option<String>,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};
    let component_hint = react_component.as_deref()
        .map(|c| format!(" (React component: {})", c))
        .unwrap_or_default();

    let prompt = format!(
        "You are editing a UI element in a web application.\n\
        Selector: {selector}{component_hint}\n\
        Current HTML:\n{current_html}\n\n\
        Instruction: {instruction}\n\n\
        Return ONLY the updated HTML/JSX for this element. No explanations.",
    );

    let messages = vec![
        Message { role: MessageRole::User, content: prompt },
    ];

    let engine = state.chat_engine.lock().await;
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

/// Generate a new React/HTML component from a natural-language description.
#[tauri::command]
pub async fn generate_component(
    state: tauri::State<'_, AppState>,
    _workspace_path: String,
    description: String,
    provider: String,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};

    let prompt = format!(
        "Generate a complete React functional component for: {description}\n\n\
        Requirements:\n\
        - Use TypeScript with proper types\n\
        - Use CSS-in-JS (style prop) or CSS modules\n\
        - Make it self-contained\n\
        - Export as default and named export\n\n\
        Return ONLY the component code, no explanations."
    );

    let messages = vec![
        Message { role: MessageRole::User, content: prompt },
    ];

    let mut engine = state.chat_engine.lock().await;
    if !provider.is_empty() {
        let _ = engine.set_provider_by_name(&provider);
    }
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

/// Import a Figma design and generate React components from it.
#[tauri::command]
pub async fn import_figma(
    state: tauri::State<'_, AppState>,
    url: String,
    token: String,
    workspace_path: String,
    provider: String,
) -> Result<Vec<serde_json::Value>, String> {
    // Extract file key from URL: https://www.figma.com/file/{key}/...
    let key = url.split('/').nth(5).unwrap_or("").to_string();
    if key.is_empty() {
        return Err("Invalid Figma URL — expected https://www.figma.com/file/{key}/...".to_string());
    }

    // Fetch Figma file metadata
    let figma_url = format!("https://api.figma.com/v1/files/{}", key);
    let client = reqwest::Client::new();
    let resp = client.get(&figma_url)
        .header("X-Figma-Token", &token)
        .send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Figma API error: {}", resp.status()));
    }

    let figma_data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    // Extract document name and top-level frames
    let doc_name = figma_data["name"].as_str().unwrap_or("Design").to_string();
    let frames: Vec<String> = figma_data["document"]["children"]
        .as_array()
        .map(|pages| {
            pages.iter()
                .flat_map(|page| {
                    page["children"].as_array()
                        .map(|frames| {
                            frames.iter()
                                .filter_map(|f| f["name"].as_str().map(|s| s.to_string()))
                                .take(5)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .take(10)
                .collect()
        })
        .unwrap_or_default();

    // Use LLM to generate React components for the frames
    use vibe_ai::provider::{Message, MessageRole};
    let prompt = format!(
        "Generate React TypeScript components for a Figma design named '{doc_name}'.\n\
        Frames/screens to implement: {frames}\n\n\
        For each frame, create a simple React component with:\n\
        - Placeholder content matching the frame name\n\
        - Basic layout structure\n\
        - TypeScript types\n\n\
        Return a JSON array:\n\
        [{{\n\
          \"path\": \"src/components/FrameName.tsx\",\n\
          \"content\": \"// Component code\"\n\
        }}]",
        frames = frames.join(", ")
    );

    let messages = vec![Message { role: MessageRole::User, content: prompt }];
    let mut engine = state.chat_engine.lock().await;
    if !provider.is_empty() {
        let _ = engine.set_provider_by_name(&provider);
    }

    let response = engine.chat(&messages, None).await.map_err(|e| e.to_string())?;

    // Parse JSON response
    let json_start = response.find('[').unwrap_or(0);
    let json_end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
    let json_slice = if json_start < json_end { &response[json_start..json_end] } else { "[]" };
    let files: Vec<serde_json::Value> = serde_json::from_str(json_slice)
        .unwrap_or_else(|_| {
            // Fallback: create a single placeholder component
            vec![serde_json::json!({
                "path": format!("src/components/{}.tsx", doc_name.replace(' ', "")),
                "content": format!("// Generated from Figma: {}\nexport function {} () {{\n  return <div>{}</div>;\n}}", doc_name, doc_name.replace(' ', ""), doc_name)
            })]
        });

    // Optionally write files to workspace
    for file in &files {
        if let (Some(path), Some(content)) = (file["path"].as_str(), file["content"].as_str()) {
            let full_path = std::path::Path::new(&workspace_path).join(path);
            if let Some(parent) = full_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&full_path, content);
        }
    }

    Ok(files)
}

// ── Deploy commands (Phase 20) ───────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DeployTarget {
    pub target: String,
    pub build_cmd: String,
    pub out_dir: String,
    pub detected_framework: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DeployRecord {
    pub id: String,
    pub target: String,
    pub url: Option<String>,
    pub timestamp: u64,
    pub status: String,
}

/// Detect project type and recommend a deploy target.
#[tauri::command]
pub async fn detect_deploy_target(workspace: String) -> Result<DeployTarget, String> {
    let pkg_path = std::path::Path::new(&workspace).join("package.json");
    let pkg: serde_json::Value = if pkg_path.exists() {
        std::fs::read_to_string(&pkg_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        serde_json::Value::Null
    };

    let scripts = pkg["scripts"].as_object();
    let deps = pkg["dependencies"].as_object();

    let ws = std::path::Path::new(&workspace);
    let (framework, build_cmd, out_dir) = if deps.map(|d| d.contains_key("next")).unwrap_or(false) {
        ("Next.js", "npm run build", ".next")
    } else if deps.map(|d| d.contains_key("@remix-run/react")).unwrap_or(false) {
        ("Remix", "npm run build", "build")
    } else if deps.map(|d| d.contains_key("@sveltejs/kit")).unwrap_or(false) {
        ("SvelteKit", "npm run build", ".svelte-kit")
    } else if scripts.map(|s| s.contains_key("build")).unwrap_or(false) {
        ("Vite/React", "npm run build", "dist")
    } else if ws.join("firebase.json").exists() {
        ("Firebase", "npm run build 2>/dev/null || true", "public")
    } else if ws.join("app.yaml").exists() || ws.join("Dockerfile").exists() {
        ("GCP Cloud Run", "echo 'Deploying from source'", ".")
    } else if ws.join("Cargo.toml").exists() {
        ("Rust/WASM", "cargo build --release", "target/release")
    } else {
        ("Static", "echo 'Nothing to build'", ".")
    };

    Ok(DeployTarget {
        target: "vercel".to_string(),
        build_cmd: build_cmd.to_string(),
        out_dir: out_dir.to_string(),
        detected_framework: framework.to_string(),
    })
}

/// Run a deployment to the specified target.
#[tauri::command]
pub async fn run_deploy(
    app_handle: tauri::AppHandle,
    target: String,
    workspace: String,
) -> Result<serde_json::Value, String> {
    use tauri::Emitter;

    let deploy_cmd = match target.as_str() {
        "vercel" => "vercel deploy --yes",
        "netlify" => "netlify deploy --prod --dir=dist",
        "railway" => "railway up",
        "github-pages" => "npm run build && npx gh-pages -d dist",
        "gcp-run" => "gcloud run deploy --source . --platform=managed --region=us-central1 --allow-unauthenticated",
        "firebase" => "firebase deploy --only hosting",
        _ => return Err(format!("Unknown deploy target: {}", target)),
    };

    let _ = app_handle.emit("deploy:log", format!("Running: {}", deploy_cmd));

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(deploy_cmd)
        .current_dir(&workspace)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Emit logs line by line
    for line in stdout.lines().chain(stderr.lines()) {
        let _ = app_handle.emit("deploy:log", line.to_string());
    }

    // Try to extract URL from output.
    // Firebase prints "Hosting URL: https://…"; Cloud Run prints "Service URL: https://…";
    // other tools print the URL inline.
    let url = stdout.lines().chain(stderr.lines())
        .find(|line| {
            line.contains("https://")
                || line.to_lowercase().contains("hosting url")
                || line.to_lowercase().contains("service url")
        })
        .and_then(|line| {
            // Prefer the token that starts with https://
            line.split_whitespace()
                .find(|w| w.starts_with("https://"))
        })
        .map(|s| s.trim_end_matches([',', '.', '"']).to_string());

    // Persist record
    let record = DeployRecord {
        id: format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()),
        target,
        url: url.clone(),
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
        status: if output.status.success() { "success".to_string() } else { "failed".to_string() },
    };

    if let Some(home) = std::env::var("HOME").ok().map(PathBuf::from) {
        let history_path = home.join(".vibecli").join("deploy-history.json");
        let mut history: Vec<DeployRecord> = std::fs::read_to_string(&history_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        history.insert(0, record);
        history.truncate(20);
        if let Some(parent) = history_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&history_path, serde_json::to_string_pretty(&history).unwrap_or_default());
    }

    Ok(serde_json::json!({ "url": url, "success": output.status.success() }))
}

/// Get deployment history.
#[tauri::command]
pub async fn get_deploy_history() -> Vec<DeployRecord> {
    std::env::var("HOME").ok().map(PathBuf::from)
        .map(|h| h.join(".vibecli").join("deploy-history.json"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Vec<DeployRecord>>(&s).ok())
        .unwrap_or_default()
}

// ── Custom domain (Phase 42) ──────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct CustomDomainResult {
    pub domain: String,
    pub cname_target: String,
    pub instructions: String,
}

/// Attempt to add a custom domain alias to a deployed project.
///
/// For Vercel it calls the Vercel REST API (requires `VERCEL_TOKEN` + `VERCEL_PROJECT_ID` env vars).
/// For all other targets it returns CNAME record instructions the user can apply manually.
#[tauri::command]
pub async fn set_custom_domain(
    target: String,
    domain: String,
) -> Result<CustomDomainResult, String> {
    // Validate domain (must not be empty, no scheme prefix)
    let domain = domain.trim().trim_start_matches("https://").trim_start_matches("http://").to_string();
    if domain.is_empty() || domain.contains('/') {
        return Err("Invalid domain — provide a bare hostname like myapp.example.com".to_string());
    }

    match target.as_str() {
        "vercel" => {
            let token = std::env::var("VERCEL_TOKEN")
                .map_err(|_| "Set VERCEL_TOKEN environment variable to use Vercel custom domains")?;
            let project_id = std::env::var("VERCEL_PROJECT_ID")
                .unwrap_or_else(|_| "my-project".to_string());
            let client = reqwest::Client::new();
            let resp = client
                .post(format!("https://api.vercel.com/v9/projects/{}/domains", project_id))
                .bearer_auth(&token)
                .json(&serde_json::json!({ "name": domain }))
                .send()
                .await
                .map_err(|e| format!("Vercel API error: {}", e))?;
            if resp.status().is_success() {
                Ok(CustomDomainResult {
                    domain: domain.clone(),
                    cname_target: "cname.vercel-dns.com".to_string(),
                    instructions: format!(
                        "Domain {} added to Vercel.\nAdd a CNAME record:\n  {} → cname.vercel-dns.com",
                        domain, domain
                    ),
                })
            } else {
                let body = resp.text().await.unwrap_or_default();
                Err(format!("Vercel API returned error: {}", body))
            }
        }
        "netlify" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "apex-loadbalancer.netlify.com".to_string(),
            instructions: format!(
                "To point {} to Netlify:\n  Add a CNAME record:\n    {} → apex-loadbalancer.netlify.com\n  Or use Netlify DNS for automatic management.",
                domain, domain
            ),
        }),
        "railway" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "railway.app".to_string(),
            instructions: format!(
                "To use {} with Railway:\n  1. In Railway dashboard → Settings → Domains → Add Domain\n  2. Add a CNAME record:\n     {} → your-app.railway.app",
                domain, domain
            ),
        }),
        "github-pages" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "your-username.github.io".to_string(),
            instructions: format!(
                "To use {} with GitHub Pages:\n  1. Create a CNAME file in your repo root containing: {}\n  2. Add a CNAME record: {} → your-username.github.io\n  3. Enable the domain in repo Settings → Pages",
                domain, domain, domain
            ),
        }),
        "gcp-run" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "ghs.googlehosted.com".to_string(),
            instructions: format!(
                "To map {} to Cloud Run:\n  gcloud beta run domain-mappings create --service SERVICE_NAME --domain {}\n  Then add a CNAME record: {} → ghs.googlehosted.com",
                domain, domain, domain
            ),
        }),
        "firebase" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "firebase-app.web.app".to_string(),
            instructions: format!(
                "To connect {} to Firebase Hosting:\n  firebase hosting:sites:add {}\n  Then follow the DNS instructions shown by the Firebase CLI.",
                domain, domain
            ),
        }),
        _ => Err(format!("Custom domain not supported for target: {}", target)),
    }
}

// ── Database commands (Phase 20) ─────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct TableInfo {
    pub name: String,
    pub row_count: i64,
    pub columns: Vec<ColumnInfo>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub row_count: usize,
    pub error: Option<String>,
}

/// Find SQLite database files in the workspace.
#[tauri::command]
pub async fn find_sqlite_files(workspace_path: String) -> Vec<String> {
    walkdir::WalkDir::new(&workspace_path)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("");
            e.file_type().is_file() && matches!(ext, "db" | "sqlite" | "sqlite3")
        })
        .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        .collect()
}

/// List tables in a database. Only SQLite is supported in the backend; Postgres/Supabase
/// would require additional crates — returns an informative error for those.
#[tauri::command]
pub async fn list_db_tables(connection_string: String, db_type: String) -> Result<Vec<TableInfo>, String> {
    match db_type.as_str() {
        "sqlite" => list_sqlite_tables(&connection_string),
        "postgres" | "supabase" => Err("PostgreSQL/Supabase support requires installing the pg feature. Use vibecli with --db-url for direct SQL access.".to_string()),
        _ => Err(format!("Unknown db type: {}", db_type)),
    }
}

fn list_sqlite_tables(path: &str) -> Result<Vec<TableInfo>, String> {
    // Read SQLite master table directly via raw file parsing using rusqlite-compatible approach
    // We use a simple shell command to avoid adding rusqlite dependency
    let output = std::process::Command::new("sqlite3")
        .arg(path)
        .arg(".tables")
        .output()
        .map_err(|_| "sqlite3 CLI not found. Install sqlite3 to use the Database panel.".to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let tables_raw = String::from_utf8_lossy(&output.stdout).to_string();
    let table_names: Vec<&str> = tables_raw.split_whitespace().collect();

    let mut tables = Vec::new();
    for name in &table_names {
        // Get row count
        let row_count: i64 = std::process::Command::new("sqlite3")
            .arg(path)
            .arg(format!("SELECT COUNT(*) FROM \"{}\";", name))
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().parse().unwrap_or(0))
            .unwrap_or(0);

        // Get columns
        let pragma_str = std::process::Command::new("sqlite3")
            .arg(path)
            .arg(format!("PRAGMA table_info(\"{}\");", name))
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let columns: Vec<ColumnInfo> = pragma_str.lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 6 {
                    Some(ColumnInfo {
                        name: parts[1].to_string(),
                        data_type: parts[2].to_string(),
                        nullable: parts[3] == "0",
                        primary_key: parts[5] != "0",
                    })
                } else { None }
            })
            .collect();

        tables.push(TableInfo { name: name.to_string(), row_count, columns });
    }

    Ok(tables)
}

/// Execute a SQL query and return results as JSON.
#[tauri::command]
pub async fn query_db(
    connection_string: String,
    db_type: String,
    sql: String,
) -> Result<QueryResult, String> {
    match db_type.as_str() {
        "sqlite" => query_sqlite(&connection_string, &sql),
        _ => Err("Only SQLite is currently supported in the GUI. Use vibecli --db-url for other databases.".to_string()),
    }
}

fn query_sqlite(path: &str, sql: &str) -> Result<QueryResult, String> {
    let output = std::process::Command::new("sqlite3")
        .arg("-json")
        .arg(path)
        .arg(sql)
        .output()
        .map_err(|_| "sqlite3 CLI not found".to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Ok(QueryResult { columns: vec![], rows: vec![], row_count: 0, error: Some(err) });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        return Ok(QueryResult { columns: vec![], rows: vec![], row_count: 0, error: None });
    }

    let rows: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .unwrap_or_default();
    let columns: Vec<String> = rows.first()
        .and_then(|r| r.as_object())
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    let row_count = rows.len();

    Ok(QueryResult { columns, rows, row_count, error: None })
}

/// Generate a SQL query from a natural-language description using the LLM.
#[tauri::command]
pub async fn generate_sql_query(
    state: tauri::State<'_, AppState>,
    description: String,
    schema: String,
    provider: String,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};

    let prompt = format!(
        "Generate a SQL query for the following request.\n\
        Schema:\n{schema}\n\n\
        Request: {description}\n\n\
        Return ONLY the SQL query, no explanation, no markdown."
    );

    let messages = vec![Message { role: MessageRole::User, content: prompt }];
    let mut engine = state.chat_engine.lock().await;
    if !provider.is_empty() {
        let _ = engine.set_provider_by_name(&provider);
    }
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

/// Generate a SQL migration script using the LLM.
#[tauri::command]
pub async fn generate_migration(
    state: tauri::State<'_, AppState>,
    connection_string: String,
    db_type: String,
    description: String,
    provider: String,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};

    let prompt = format!(
        "Generate a SQL migration script for a {} database.\n\
        Description: {description}\n\n\
        Return ONLY the SQL, no explanation, no markdown. Include IF NOT EXISTS / IF EXISTS guards.",
        db_type
    );

    let messages = vec![Message { role: MessageRole::User, content: prompt }];
    let mut engine = state.chat_engine.lock().await;
    if !provider.is_empty() {
        let _ = engine.set_provider_by_name(&provider);
    }
    let _ = connection_string;
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

// ── Supabase commands (Phase 26) ──────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct SupabaseConfig {
    pub url: String,
    pub anon_key: String,
}

#[tauri::command]
pub async fn get_supabase_config(workspace_path: String) -> Result<SupabaseConfig, String> {
    let path = std::path::PathBuf::from(&workspace_path).join(".vibeui").join("supabase.json");
    if path.exists() {
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    } else {
        Ok(SupabaseConfig::default())
    }
}

#[tauri::command]
pub async fn save_supabase_config(workspace_path: String, url: String, anon_key: String) -> Result<(), String> {
    let dir = std::path::PathBuf::from(&workspace_path).join(".vibeui");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let cfg = SupabaseConfig { url, anon_key };
    let json = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("supabase.json"), json)
        .map_err(|e| e.to_string())
}

/// List Supabase tables via the PostgREST introspection endpoint.
#[tauri::command]
pub async fn list_supabase_tables(url: String, anon_key: String) -> Result<Vec<TableInfo>, String> {
    // Query pg_tables via RPC or the /rest/v1/ endpoint
    let client = reqwest::Client::new();
    let endpoint = format!("{}/rest/v1/", url.trim_end_matches('/'));
    let resp = client.get(&endpoint)
        .header("apikey", &anon_key)
        .header("Authorization", format!("Bearer {}", anon_key))
        .send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Supabase error {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
    }

    // The root endpoint returns OpenAPI JSON with definitions for each table
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let mut tables = Vec::new();
    if let Some(defs) = json.get("definitions").and_then(|d| d.as_object()) {
        for name in defs.keys() {
            tables.push(TableInfo { name: name.clone(), row_count: 0, columns: vec![] });
        }
    }
    // Sort alphabetically
    tables.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tables)
}

#[derive(serde::Serialize)]
pub struct SupabaseQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub error: Option<String>,
}

/// Run a SELECT query via Supabase PostgREST (table-based URL) or RPC.
#[tauri::command]
pub async fn query_supabase(url: String, anon_key: String, sql: String) -> Result<SupabaseQueryResult, String> {
    // Extract table name and WHERE clause from simple SELECT * FROM <table> LIMIT <n>
    let sql_trimmed = sql.trim();
    let sql_lower = sql_trimmed.to_lowercase();
    if sql_lower.starts_with("select") {
        // Use the lowercased version's index safely — both strings have identical byte lengths for ASCII keywords
        if let Some(from_idx) = sql_lower.find(" from ") {
            let after_from = &sql_trimmed[from_idx + " from ".len()..].trim();
            let table = after_from.split_whitespace().next()
                .unwrap_or("").trim_matches('"');
            let limit = if sql_lower.contains("limit") {
                sql_lower.split("limit").nth(1)
                    .and_then(|s| s.trim().split_whitespace().next())
                    .and_then(|n| n.parse::<u32>().ok())
                    .unwrap_or(50)
            } else { 50 };

            let client = reqwest::Client::new();
            let endpoint = format!("{}/rest/v1/{}", url.trim_end_matches('/'), table);
            let resp = client.get(&endpoint)
                .header("apikey", &anon_key)
                .header("Authorization", format!("Bearer {}", anon_key))
                .header("Prefer", "count=estimated")
                .query(&[("limit", limit.to_string())])
                .send().await.map_err(|e| e.to_string())?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Ok(SupabaseQueryResult { columns: vec![], rows: vec![], error: Some(body) });
            }

            let rows_json: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
            if rows_json.is_empty() {
                return Ok(SupabaseQueryResult { columns: vec![], rows: vec![], error: None });
            }
            let columns: Vec<String> = rows_json[0].as_object()
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            let rows: Vec<Vec<String>> = rows_json.iter().map(|row| {
                columns.iter().map(|col| {
                    row.get(col).map(|v| match v {
                        serde_json::Value::Null => "NULL".to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    }).unwrap_or_default()
                }).collect()
            }).collect();
            return Ok(SupabaseQueryResult { columns, rows, error: None });
        }
    }
    Ok(SupabaseQueryResult { columns: vec![], rows: vec![], error: Some("Only SELECT ... FROM <table> queries are supported via PostgREST".to_string()) })
}

#[tauri::command]
pub async fn generate_supabase_query(
    state: tauri::State<'_, AppState>,
    workspace_path: String,
    provider: String,
    description: String,
    tables: Vec<String>,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};
    let _ = workspace_path;
    let tables_list = tables.join(", ");
    let prompt = format!(
        "Generate a PostgreSQL SELECT query for a Supabase database.\n\
        Available tables: {tables_list}\n\
        Request: {description}\n\n\
        Return ONLY the SQL query, no explanation, no markdown fences."
    );
    let messages = vec![Message { role: MessageRole::User, content: prompt }];
    let mut engine = state.chat_engine.lock().await;
    if !provider.is_empty() { let _ = engine.set_provider_by_name(&provider); }
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

// ── Auth scaffolding commands (Phase 26) ──────────────────────────────────────

#[tauri::command]
pub async fn generate_auth_scaffold(
    state: tauri::State<'_, AppState>,
    workspace_path: String,
    provider: String,
    auth_provider: String,
    framework: String,
    include_middleware: bool,
    include_tests: bool,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};
    let _ = workspace_path;

    let middleware_note = if include_middleware { "Include auth middleware/guard." } else { "" };
    let tests_note = if include_tests { "Include unit tests." } else { "" };

    let prompt = format!(
        "Generate complete authentication code for the following setup:\n\
        - Auth provider: {auth_provider}\n\
        - Framework: {framework}\n\
        - {middleware_note}\n\
        - {tests_note}\n\n\
        Include:\n\
        1. Login / callback / logout route handlers\n\
        2. Session/token management utilities\n\
        3. Environment variable documentation (comment at top)\n\
        {middleware_note}\n\
        {tests_note}\n\n\
        Format as a single file with clear section comments. No markdown fences."
    );

    let messages = vec![Message { role: MessageRole::User, content: prompt }];
    let mut engine = state.chat_engine.lock().await;
    if !provider.is_empty() { let _ = engine.set_provider_by_name(&provider); }
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_auth_scaffold(
    workspace_path: String,
    target_path: String,
    code: String,
    framework: String,
) -> Result<(), String> {
    let workspace_root = std::fs::canonicalize(&workspace_path)
        .map_err(|e| format!("Invalid workspace path: {}", e))?;
    let dir = workspace_root.join(&target_path);
    // Prevent path traversal: ensure the destination stays inside the workspace
    let canonical_dir = dir.to_str()
        .map(|s| std::path::PathBuf::from(s))
        .unwrap_or(dir.clone());
    // Resolve without requiring it to exist yet — strip ".." components
    let mut resolved = std::path::PathBuf::new();
    for component in canonical_dir.components() {
        match component {
            std::path::Component::ParentDir => { resolved.pop(); }
            std::path::Component::CurDir => {}
            c => resolved.push(c),
        }
    }
    if !resolved.starts_with(&workspace_root) {
        return Err("target_path must be inside the workspace".to_string());
    }
    std::fs::create_dir_all(&resolved).map_err(|e| e.to_string())?;
    let dir = resolved;

    let ext = match framework.as_str() {
        "fastapi" => "py",
        "axum" => "rs",
        _ => "ts",
    };
    let file_name = format!("auth.{}", ext);
    std::fs::write(dir.join(&file_name), &code).map_err(|e| e.to_string())?;
    Ok(())
}

// ── GitHub Sync commands (Phase 26) ───────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct GitHubSyncStatus {
    pub repo_url: Option<String>,
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub has_remote: bool,
    pub last_synced: Option<String>,
}

fn load_github_token(workspace_path: &str) -> Option<String> {
    // Check env first, then workspace .vibeui/github_token
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        return Some(t);
    }
    let path = std::path::PathBuf::from(workspace_path).join(".vibeui").join("github_token");
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

#[tauri::command]
pub async fn has_github_token(workspace_path: String) -> Result<bool, String> {
    Ok(load_github_token(&workspace_path).is_some())
}

#[tauri::command]
pub async fn save_github_token(workspace_path: String, token: String) -> Result<(), String> {
    let dir = std::path::PathBuf::from(&workspace_path).join(".vibeui");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("github_token"), token.trim()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_github_sync_status(workspace_path: String) -> Result<GitHubSyncStatus, String> {
    use vibe_core::git;
    let ws = std::path::PathBuf::from(&workspace_path);
    if !git::is_git_repo(&ws) {
        return Ok(GitHubSyncStatus { repo_url: None, branch: "main".to_string(), ahead: 0, behind: 0, has_remote: false, last_synced: None });
    }
    let branch = git::get_current_branch(&ws).unwrap_or_else(|_| "main".to_string());

    // Get remote URL via git command
    let remote_output = std::process::Command::new("git")
        .args(["-C", &workspace_path, "remote", "get-url", "origin"])
        .output().ok();
    let repo_url = remote_output
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    // ahead/behind via git rev-list
    let ahead_output = std::process::Command::new("git")
        .args(["-C", &workspace_path, "rev-list", "--count", "@{u}..HEAD"])
        .output().ok();
    let ahead = ahead_output
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);

    let behind_output = std::process::Command::new("git")
        .args(["-C", &workspace_path, "rev-list", "--count", "HEAD..@{u}"])
        .output().ok();
    let behind = behind_output
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);

    Ok(GitHubSyncStatus {
        has_remote: repo_url.is_some(),
        repo_url,
        branch,
        ahead,
        behind,
        last_synced: None,
    })
}

#[tauri::command]
pub async fn github_sync_push(workspace_path: String, commit_message: String) -> Result<(), String> {
    // git add -A && git commit -m ... && git push
    let status = std::process::Command::new("git")
        .args(["-C", &workspace_path, "add", "-A"])
        .status().map_err(|e| e.to_string())?;
    if !status.success() { return Err("git add failed".to_string()); }

    let status = std::process::Command::new("git")
        .args(["-C", &workspace_path, "commit", "-m", &commit_message])
        .status().map_err(|e| e.to_string())?;
    if !status.success() { return Err("git commit failed (nothing to commit?)".to_string()); }

    let out = std::process::Command::new("git")
        .args(["-C", &workspace_path, "push"])
        .output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn github_sync_pull(workspace_path: String) -> Result<(), String> {
    let out = std::process::Command::new("git")
        .args(["-C", &workspace_path, "pull", "--ff-only"])
        .output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct RepoInfo {
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub default_branch: String,
    pub url: String,
}

#[tauri::command]
pub async fn list_github_repos(workspace_path: String) -> Result<Vec<RepoInfo>, String> {
    let token = load_github_token(&workspace_path)
        .ok_or("GITHUB_TOKEN not set")?;
    let client = reqwest::Client::new();
    let resp = client.get("https://api.github.com/user/repos?per_page=50&sort=updated")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "vibeui/0.1")
        .send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API error {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
    }

    let json: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json.iter().filter_map(|r| {
        Some(RepoInfo {
            name: r["name"].as_str()?.to_string(),
            full_name: r["full_name"].as_str()?.to_string(),
            private: r["private"].as_bool().unwrap_or(false),
            default_branch: r["default_branch"].as_str().unwrap_or("main").to_string(),
            url: r["html_url"].as_str()?.to_string(),
        })
    }).collect())
}

#[tauri::command]
pub async fn github_create_repo(
    workspace_path: String,
    name: String,
    #[allow(non_snake_case)]
    private: bool,
) -> Result<String, String> {
    let token = load_github_token(&workspace_path)
        .ok_or("GITHUB_TOKEN not set")?;
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "name": name, "private": private, "auto_init": false });
    let resp = client.post("https://api.github.com/user/repos")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "vibeui/0.1")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API error {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let clone_url = json["clone_url"].as_str().ok_or("No clone_url in response")?.to_string();
    let html_url = json["html_url"].as_str().unwrap_or(&clone_url).to_string();

    // Add remote and push
    std::process::Command::new("git")
        .args(["-C", &workspace_path, "remote", "add", "origin", &clone_url])
        .status().map_err(|e| e.to_string())?;

    let out = std::process::Command::new("git")
        .args(["-C", &workspace_path, "push", "-u", "origin", "HEAD"])
        .output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        // Don't fail on push errors (workspace might be empty)
        eprintln!("git push warning: {}", stderr);
    }

    Ok(html_url)
}

fn extract_json(text: &str) -> String {
    // Strip ```json ... ``` fences if present
    let trimmed = text.trim();
    if let Some(inner) = trimmed.strip_prefix("```json").and_then(|s| s.strip_suffix("```")) {
        return inner.trim().to_string();
    }
    if let Some(inner) = trimmed.strip_prefix("```").and_then(|s| s.strip_suffix("```")) {
        return inner.trim().to_string();
    }
    // Find first { ... } block
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }
    trimmed.to_string()
}

// ── Auto-Memories ─────────────────────────────────────────────────────────────
//
// Auto-extracted facts from agent sessions, stored at `~/.vibeui/auto-memory.json`.
// Each fact has an id, text, confidence score, tags, and a pinned flag.
// The VibeUI "Auto-Facts" panel reads/writes this store.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFact {
    pub id: String,
    pub fact: String,
    #[serde(default = "default_fact_confidence")]
    pub confidence: f32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub session_id: Option<String>,
}

fn default_fact_confidence() -> f32 { 0.7 }

fn auto_memory_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibeui").join("auto-memory.json")
}

fn load_auto_memories() -> Vec<MemoryFact> {
    std::fs::read_to_string(auto_memory_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_auto_memories(facts: &[MemoryFact]) -> Result<(), String> {
    let path = auto_memory_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(facts).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_auto_memories() -> Result<Vec<MemoryFact>, String> {
    Ok(load_auto_memories())
}

#[tauri::command]
pub async fn delete_auto_memory(id: String) -> Result<bool, String> {
    let mut facts = load_auto_memories();
    let before = facts.len();
    facts.retain(|f| f.id != id);
    if facts.len() < before {
        save_auto_memories(&facts)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn pin_auto_memory(id: String, pinned: bool) -> Result<bool, String> {
    let mut facts = load_auto_memories();
    if let Some(f) = facts.iter_mut().find(|f| f.id == id) {
        f.pinned = pinned;
        save_auto_memories(&facts)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Manually append a fact to the auto-memory store (for VibeUI "Add Fact" UI).
#[tauri::command]
pub async fn add_auto_memory(fact: String, tags: Vec<String>) -> Result<MemoryFact, String> {
    let id = {
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("{:x}", ms)
    };
    let new_fact = MemoryFact {
        id: id.clone(),
        fact: fact.clone(),
        confidence: 1.0, // manually added = certain
        tags,
        pinned: true,
        session_id: None,
    };
    let mut facts = load_auto_memories();
    facts.push(new_fact.clone());
    save_auto_memories(&facts)?;
    Ok(new_fact)
}

// ── BugBot ────────────────────────────────────────────────────────────────────
//
// AI-powered automated code scanner. Analyzes the workspace for bugs, security
// issues, and code smells. Returns structured reports with severity and fixes.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReport {
    pub id: String,
    pub severity: String,    // "critical" | "high" | "medium" | "low" | "info"
    pub category: String,    // "security" | "bug" | "perf" | "style" | "smell"
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub line_hint: Option<u32>,
    pub suggestion: String,
    pub fix_snippet: Option<String>,
}

#[tauri::command]
pub async fn run_bugbot(
    workspace_path: String,
    scan_scope: String, // "workspace" | "file:<path>"
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BugReport>, String> {
    // Collect files to scan
    let root = std::path::PathBuf::from(&workspace_path);
    let mut code_snippets: Vec<(String, String)> = Vec::new(); // (path, snippet)

    let target_file = if scan_scope.starts_with("file:") {
        Some(scan_scope.strip_prefix("file:").unwrap_or("").to_string())
    } else {
        None
    };

    let files_to_scan: Vec<std::path::PathBuf> = if let Some(ref file) = target_file {
        vec![root.join(file)]
    } else {
        // Scan common code files (limit to 20 files for performance)
        walkdir::WalkDir::new(&root)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && matches!(
                    e.path().extension().and_then(|x| x.to_str()),
                    Some("rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go")
                ) && !e.path().to_string_lossy().contains("/target/")
                  && !e.path().to_string_lossy().contains("/node_modules/")
            })
            .map(|e| e.into_path())
            .take(20)
            .collect()
    };

    for path in files_to_scan {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let rel = path.strip_prefix(&root).unwrap_or(&path);
            // Only take first 150 lines per file
            let snippet: String = content.lines().take(150).collect::<Vec<_>>().join("\n");
            code_snippets.push((rel.to_string_lossy().to_string(), snippet));
        }
    }

    if code_snippets.is_empty() {
        return Ok(vec![]);
    }

    // Build prompt
    let files_text: String = code_snippets.iter()
        .map(|(path, code)| format!("=== {} ===\n{}", path, code))
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = format!(
        r#"You are a code security and quality scanner. Analyze the following code and identify bugs, security vulnerabilities, performance issues, and code smells.

Return ONLY a valid JSON array (no markdown, no explanation):
[
  {{
    "severity": "critical|high|medium|low|info",
    "category": "security|bug|perf|style|smell",
    "title": "Short issue title",
    "description": "What the problem is and why it matters",
    "file_path": "relative/path/to/file.rs",
    "line_hint": null,
    "suggestion": "How to fix it",
    "fix_snippet": null
  }}
]

Severity guide: critical=data loss/RCE/auth bypass, high=serious bug, medium=likely bug, low=code smell, info=suggestion.
Return 3–8 issues maximum. Focus on real problems, not style preferences.

Code to analyze:
{}
"#,
        files_text
    );

    let messages = vec![Message { role: vibe_ai::MessageRole::User, content: prompt }];

    let engine = state.chat_engine.lock().await;
    let raw_response = engine.chat(&messages, None).await.map_err(|e| e.to_string())?;
    drop(engine);

    // Parse JSON from response
    let json_start = raw_response.find('[').unwrap_or(0);
    let json_end = raw_response.rfind(']').map(|i| i + 1).unwrap_or(raw_response.len());
    let json_str = if json_start < json_end { &raw_response[json_start..json_end] } else { "[]" };

    #[derive(Deserialize)]
    struct RawReport {
        severity: String,
        category: String,
        title: String,
        description: String,
        file_path: Option<String>,
        line_hint: Option<u32>,
        suggestion: String,
        fix_snippet: Option<String>,
    }

    let raw: Vec<RawReport> = serde_json::from_str(json_str).map_err(|e| format!("Parse error: {e}"))?;

    let mut reports: Vec<BugReport> = raw.into_iter().enumerate().map(|(i, r)| BugReport {
        id: format!("bug-{}", i),
        severity: r.severity,
        category: r.category,
        title: r.title,
        description: r.description,
        file_path: r.file_path,
        line_hint: r.line_hint,
        suggestion: r.suggestion,
        fix_snippet: r.fix_snippet,
    }).collect();

    // Sort by severity
    let sev_order = |s: &str| match s { "critical" => 0, "high" => 1, "medium" => 2, "low" => 3, _ => 4 };
    reports.sort_by_key(|r| sev_order(&r.severity));

    Ok(reports)
}

// ── Steering Files ─────────────────────────────────────────────────────────────
//
// Steering files live in `<workspace>/.vibecli/steering/` (workspace scope) or
// `~/.vibecli/steering/` (global scope). They are Markdown files that inject
// project-wide context at the top of every agent system prompt — no path gating.
// Format mirrors rule files: optional YAML front-matter + body.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringFileMeta {
    pub filename: String,
    pub name: String,
    /// Optional scope label (scope field from front-matter, e.g. "project" or "global")
    pub scope_label: Option<String>,
}

fn steering_dir(scope: &str, workspace_root: Option<&std::path::Path>) -> std::path::PathBuf {
    if scope == "global" {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".vibecli").join("steering")
    } else {
        workspace_root
            .map(|r| r.join(".vibecli").join("steering"))
            .unwrap_or_else(|| std::path::PathBuf::from(".vibecli").join("steering"))
    }
}

fn parse_steering_meta(content: &str, filename: &str) -> SteeringFileMeta {
    let name_default = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("steering")
        .to_string();

    if let Some(after_prefix) = content.strip_prefix("---") {
        let after = after_prefix.trim_start_matches('\n');
        if let Some(close_pos) = after.find("\n---") {
            let fm = &after[..close_pos];
            let mut name: Option<String> = None;
            let mut scope_label: Option<String> = None;
            for line in fm.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                    match k.trim() {
                        "name" => name = Some(val),
                        "scope" => scope_label = Some(val),
                        _ => {}
                    }
                }
            }
            return SteeringFileMeta {
                filename: filename.to_string(),
                name: name.unwrap_or(name_default),
                scope_label,
            };
        }
    }
    SteeringFileMeta { filename: filename.to_string(), name: name_default, scope_label: None }
}

#[tauri::command]
pub async fn get_steering_files(
    scope: String,
    workspace_root: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let root = workspace_root.as_deref().map(std::path::Path::new);
    let dir = steering_dir(&scope, root);
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            let meta = parse_steering_meta(&content, &filename);
            result.push(serde_json::json!({
                "filename": meta.filename,
                "name": meta.name,
                "scope_label": meta.scope_label,
                "content": content,
            }));
        }
    }
    result.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Ok(result)
}

#[tauri::command]
pub async fn save_steering_file(
    scope: String,
    workspace_root: Option<String>,
    filename: String,
    content: String,
) -> Result<(), String> {
    let root = workspace_root.as_deref().map(std::path::Path::new);
    let dir = steering_dir(&scope, root);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Sanitize filename
    let safe = filename.replace(['/', '\\', '\0'], "_");
    let fname = if safe.ends_with(".md") { safe } else { format!("{}.md", safe) };
    std::fs::write(dir.join(&fname), &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_steering_file(
    scope: String,
    workspace_root: Option<String>,
    filename: String,
) -> Result<(), String> {
    let root = workspace_root.as_deref().map(std::path::Path::new);
    let dir = steering_dir(&scope, root);
    let path = dir.join(&filename);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Agent Browser Actions ─────────────────────────────────────────────────────
//
// Provides headless browser-like actions that the agent can invoke:
//   Navigate  — fetch a URL, return stripped text content
//   GetText   — same as Navigate (alias for "read the page text")
//   Screenshot — capture a region of the screen to a temp PNG, return path
//   WaitFor   — sleep N milliseconds (useful between actions)

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BrowserAction {
    Navigate { url: String },
    GetText   { url: String },
    Screenshot { x: Option<i32>, y: Option<i32>, width: Option<i32>, height: Option<i32> },
    WaitFor   { ms: u64 },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserActionResult {
    pub success: bool,
    pub output: String,
}

/// Strip HTML tags and collapse whitespace.  Returns at most `max_chars`.
fn strip_html(raw: &str, max_chars: usize) -> String {
    let no_tags = re_html_tag().replace_all(raw, " ");
    let decoded = no_tags
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    let collapsed = re_whitespace().replace_all(&decoded, " ");
    collapsed.trim().chars().take(max_chars).collect()
}

#[tauri::command]
pub async fn agent_browser_action(action: BrowserAction) -> Result<BrowserActionResult, String> {
    match action {
        BrowserAction::Navigate { url } | BrowserAction::GetText { url } => {
            let client = reqwest::Client::builder()
                .user_agent("VibeCLI/1.0")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?;

            let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                return Ok(BrowserActionResult {
                    success: false,
                    output: format!("HTTP {}: {}", resp.status().as_u16(), url),
                });
            }
            let body = resp.text().await.map_err(|e| e.to_string())?;
            let text = strip_html(&body, 8000);
            Ok(BrowserActionResult {
                success: true,
                output: format!("=== {} ===\n{}", url, text),
            })
        }

        BrowserAction::Screenshot { x, y, width, height } => {
            // Create a temp file for the screenshot
            let tmp = {
                let mut path = std::env::temp_dir().join("vibecli-screenshots");
                let _ = std::fs::create_dir_all(&path);
                path.push(format!("{:032x}.png", rand::random::<u128>()));
                path
            };

            // Build screencapture command (macOS) or scrot (Linux)
            #[cfg(target_os = "macos")]
            let result = {
                let mut cmd = std::process::Command::new("screencapture");
                cmd.arg("-x"); // no sound
                if let (Some(rx), Some(ry), Some(rw), Some(rh)) = (x, y, width, height) {
                    cmd.arg("-R").arg(format!("{},{},{},{}", rx, ry, rw, rh));
                }
                cmd.arg(tmp.to_str().unwrap_or("shot.png")).output()
            };

            #[cfg(not(target_os = "macos"))]
            let result = {
                let mut cmd = std::process::Command::new("scrot");
                if let (Some(rx), Some(ry), Some(rw), Some(rh)) = (x, y, width, height) {
                    cmd.arg("-a").arg(format!("{},{},{},{}", rx, ry, rw, rh));
                }
                cmd.arg(tmp.to_str().unwrap_or("shot.png")).output()
            };

            match result {
                Ok(out) if out.status.success() => Ok(BrowserActionResult {
                    success: true,
                    output: tmp.to_string_lossy().to_string(),
                }),
                Ok(out) => Ok(BrowserActionResult {
                    success: false,
                    output: String::from_utf8_lossy(&out.stderr).to_string(),
                }),
                Err(e) => Ok(BrowserActionResult {
                    success: false,
                    output: format!("Screenshot tool not available: {}", e),
                }),
            }
        }

        BrowserAction::WaitFor { ms } => {
            let clamped = ms.min(30_000); // max 30 s
            tokio::time::sleep(tokio::time::Duration::from_millis(clamped)).await;
            Ok(BrowserActionResult {
                success: true,
                output: format!("Waited {}ms", clamped),
            })
        }
    }
}

// ── Red Team Security Scanning (Phase 41) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamFinding {
    pub id: String,
    pub attack_vector: String,
    pub cvss_score: f64,
    pub severity: String,
    pub url: String,
    pub location: String,
    pub title: String,
    pub description: String,
    pub poc: String,
    pub remediation: String,
    pub source_file: Option<String>,
    pub source_line: Option<u32>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamSessionInfo {
    pub id: String,
    pub target_url: String,
    pub current_stage: String,
    pub findings: Vec<RedTeamFinding>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[tauri::command]
pub async fn start_redteam_scan(
    url: String,
    _config: Option<serde_json::Value>,
) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let session_id = format!("rt-{}", ts);

    // Create session directory.
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let sessions_dir = std::path::PathBuf::from(&home).join(".vibeui").join("redteam");
    std::fs::create_dir_all(&sessions_dir).map_err(|e| e.to_string())?;

    // Save a placeholder session.
    let session = RedTeamSessionInfo {
        id: session_id.clone(),
        target_url: url.clone(),
        current_stage: "Recon".to_string(),
        findings: vec![],
        started_at: format!("{}", ts),
        finished_at: None,
    };
    let path = sessions_dir.join(format!("{}.json", &session_id));
    let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;

    Ok(session_id)
}

#[tauri::command]
pub async fn get_redteam_sessions() -> Result<Vec<RedTeamSessionInfo>, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let sessions_dir = std::path::PathBuf::from(&home).join(".vibeui").join("redteam");
    if !sessions_dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions: Vec<RedTeamSessionInfo> = Vec::new();
    let entries = std::fs::read_dir(&sessions_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(session) = serde_json::from_str::<RedTeamSessionInfo>(&json) {
                    sessions.push(session);
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(sessions)
}

#[tauri::command]
pub async fn get_redteam_findings(session_id: String) -> Result<Vec<RedTeamFinding>, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(&home)
        .join(".vibeui").join("redteam").join(format!("{}.json", session_id));

    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let session: RedTeamSessionInfo = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(session.findings)
}

#[tauri::command]
pub async fn generate_redteam_report(session_id: String) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(&home)
        .join(".vibeui").join("redteam").join(format!("{}.json", session_id));

    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let session: RedTeamSessionInfo = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    let mut report = String::new();
    report.push_str("# Security Assessment Report\n\n");
    report.push_str(&format!("**Target:** {}\n", session.target_url));
    report.push_str(&format!("**Session:** {}\n", session.id));
    report.push_str(&format!("**Date:** {}\n", session.started_at));
    report.push_str("\n---\n\n## Findings\n\n");

    if session.findings.is_empty() {
        report.push_str("No vulnerabilities were identified.\n");
    } else {
        let mut sorted = session.findings.clone();
        sorted.sort_by(|a, b| b.cvss_score.partial_cmp(&a.cvss_score).unwrap_or(std::cmp::Ordering::Equal));

        for (i, f) in sorted.iter().enumerate() {
            report.push_str(&format!("### {}. {} (CVSS: {:.1})\n\n", i + 1, f.title, f.cvss_score));
            report.push_str(&format!("- **Severity:** {}\n", f.severity));
            report.push_str(&format!("- **URL:** `{}`\n", f.url));
            report.push_str(&format!("- **Parameter:** `{}`\n", f.location));
            report.push_str(&format!("- **Confirmed:** {}\n", if f.confirmed { "Yes" } else { "Unconfirmed" }));
            report.push_str(&format!("\n**Description:** {}\n", f.description));
            report.push_str(&format!("\n**PoC:**\n```\n{}\n```\n", f.poc));
            report.push_str(&format!("\n**Remediation:** {}\n\n---\n\n", f.remediation));
        }
    }

    report.push_str("\n*Generated by VibeCody Red Team Module*\n");
    Ok(report)
}

#[tauri::command]
pub async fn cancel_redteam_scan(session_id: String) -> Result<(), String> {
    // Mark session as cancelled.
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(&home)
        .join(".vibeui").join("redteam").join(format!("{}.json", session_id));

    if path.exists() {
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut session: RedTeamSessionInfo = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        session.current_stage = "Cancelled".to_string();
        session.finished_at = Some("cancelled".to_string());
        let updated = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
        std::fs::write(&path, &updated).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Collab (Phase 43) ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabSessionInfo {
    pub room_id: String,
    pub peer_id: String,
    pub ws_url: String,
    pub peers: Vec<CollabPeerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabPeerInfo {
    pub peer_id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabStatus {
    pub connected: bool,
    pub room_id: Option<String>,
    pub peer_count: usize,
}

#[tauri::command]
pub async fn create_collab_session(
    room_id: Option<String>,
    user_name: String,
    daemon_port: Option<u16>,
) -> Result<CollabSessionInfo, String> {
    let port = daemon_port.unwrap_or(7878);
    let room = room_id.unwrap_or_else(|| format!("{:016x}", rand::random::<u64>()));
    let ws_url = format!("ws://127.0.0.1:{port}/ws/collab/{room}");
    Ok(CollabSessionInfo {
        room_id: room,
        peer_id: format!("{:016x}", rand::random::<u64>()),
        ws_url,
        peers: vec![CollabPeerInfo {
            peer_id: format!("{:016x}", rand::random::<u64>()),
            name: user_name,
            color: "#61afef".to_string(),
        }],
    })
}

#[tauri::command]
pub async fn join_collab_session(
    room_id: String,
    user_name: String,
    daemon_port: Option<u16>,
) -> Result<CollabSessionInfo, String> {
    let port = daemon_port.unwrap_or(7878);
    let ws_url = format!("ws://127.0.0.1:{port}/ws/collab/{room_id}");
    Ok(CollabSessionInfo {
        room_id,
        peer_id: format!("{:016x}", rand::random::<u64>()),
        ws_url,
        peers: vec![CollabPeerInfo {
            peer_id: format!("{:016x}", rand::random::<u64>()),
            name: user_name,
            color: "#e06c75".to_string(),
        }],
    })
}

#[tauri::command]
pub async fn leave_collab_session() -> Result<(), String> {
    // The actual disconnect is handled by the frontend closing the WebSocket.
    // This command is a no-op placeholder for cleanup if needed.
    Ok(())
}

#[tauri::command]
pub async fn list_collab_peers(
    room_id: String,
    daemon_port: Option<u16>,
    api_token: Option<String>,
) -> Result<Vec<CollabPeerInfo>, String> {
    let port = daemon_port.unwrap_or(7878);
    let url = format!("http://127.0.0.1:{port}/collab/rooms/{room_id}/peers");
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(token) = api_token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Daemon returned status {}", resp.status()));
    }
    let peers: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(peers
        .into_iter()
        .map(|p| CollabPeerInfo {
            peer_id: p["peer_id"].as_str().unwrap_or("").to_string(),
            name: p["name"].as_str().unwrap_or("").to_string(),
            color: p["color"].as_str().unwrap_or("#888").to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn get_collab_status(
    room_id: Option<String>,
) -> Result<CollabStatus, String> {
    // Status is managed client-side via the useCollab hook;
    // this command provides a bridge for non-React callers.
    Ok(CollabStatus {
        connected: room_id.is_some(),
        room_id,
        peer_count: 0,
    })
}

// ══════════════════════════════════════════════════════════════════════════════
// Phase 44 — Code Coverage Panel
// ══════════════════════════════════════════════════════════════════════════════

/// Per-file coverage entry returned by `run_coverage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: String,
    pub covered: u32,
    pub total: u32,
    pub pct: f32,
    pub uncovered_lines: Vec<u32>,
}

/// Aggregate coverage result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageResult {
    pub framework: String,
    pub total_pct: f32,
    pub files: Vec<FileCoverage>,
    pub raw_output: String,
}

/// Detect which coverage tool the project uses.
#[tauri::command]
pub async fn detect_coverage_tool(workspace: String) -> Result<String, String> {
    let ws = PathBuf::from(&workspace);
    if ws.join("Cargo.toml").exists() {
        return Ok("cargo-llvm-cov".to_string());
    }
    if ws.join("package.json").exists() {
        if let Ok(content) = std::fs::read_to_string(ws.join("package.json")) {
            if content.contains("\"nyc\"") || content.contains("\"c8\"") || content.contains("\"istanbul\"") {
                return Ok("nyc".to_string());
            }
            if content.contains("\"coverage\"") {
                return Ok("npm-coverage".to_string());
            }
        }
        return Ok("npm-coverage".to_string());
    }
    if ws.join("pytest.ini").exists() || ws.join("pyproject.toml").exists() || ws.join("setup.py").exists() {
        return Ok("coverage.py".to_string());
    }
    if ws.join("go.mod").exists() {
        return Ok("go-cover".to_string());
    }
    Err("No coverage tool detected in this workspace".to_string())
}

/// Run coverage for the workspace and return structured results.
#[tauri::command]
pub async fn run_coverage(
    app: tauri::AppHandle,
    workspace: String,
    tool: String,
) -> Result<CoverageResult, String> {
    let ws = PathBuf::from(&workspace);
    let (prog, args): (&str, &[&str]) = match tool.as_str() {
        "cargo-llvm-cov" => ("cargo", &["llvm-cov", "--lcov", "--output-path", "coverage.lcov"]),
        "nyc"            => ("npx",   &["nyc", "--reporter=lcov", "npm", "test"]),
        "npm-coverage"   => ("npm",   &["run", "coverage"]),
        "coverage.py"    => ("python",&["-m", "pytest", "--cov", "--cov-report=lcov:coverage.lcov", "-q"]),
        "go-cover"       => ("go",    &["test", "./...", "-coverprofile=coverage.out"]),
        _                => return Err(format!("Unknown coverage tool: {tool}")),
    };

    let _ = &app; // reserved for future event streaming
    let output = tokio::process::Command::new(prog)
        .args(args)
        .current_dir(&ws)
        .output()
        .await
        .map_err(|e| format!("Failed to run {prog}: {e}"))?;

    let raw_output = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    // Determine LCOV file path
    let lcov_path = if tool == "go-cover" {
        ws.join("coverage.out")
    } else {
        ws.join("coverage.lcov")
    };

    let files = if lcov_path.exists() {
        let content = std::fs::read_to_string(&lcov_path).unwrap_or_default();
        if tool == "go-cover" { parse_go_coverage(&content) } else { parse_lcov(&content) }
    } else {
        Vec::new()
    };

    let (total_covered, total_lines) = files.iter().fold((0u32, 0u32), |(ac, at), f| (ac + f.covered, at + f.total));
    let total_pct = if total_lines > 0 {
        (total_covered as f32 / total_lines as f32) * 100.0
    } else {
        extract_pct_from_raw(&raw_output)
    };

    Ok(CoverageResult { framework: tool, total_pct, files, raw_output })
}

/// Parse LCOV format into FileCoverage entries.
fn parse_lcov(lcov: &str) -> Vec<FileCoverage> {
    let mut files = Vec::new();
    let mut current_file: Option<String> = None;
    let mut covered = 0u32;
    let mut total = 0u32;
    let mut uncovered: Vec<u32> = Vec::new();

    for line in lcov.lines() {
        if let Some(path) = line.strip_prefix("SF:") {
            current_file = Some(path.to_string());
            covered = 0; total = 0; uncovered.clear();
        } else if let Some(da) = line.strip_prefix("DA:") {
            let parts: Vec<&str> = da.splitn(2, ',').collect();
            if parts.len() == 2 {
                if let Ok(ln) = parts[0].parse::<u32>() {
                    total += 1;
                    let count: i64 = parts[1].parse().unwrap_or(0);
                    if count > 0 { covered += 1; } else { uncovered.push(ln); }
                }
            }
        } else if line == "end_of_record" {
            if let Some(path) = current_file.take() {
                let pct = if total > 0 { (covered as f32 / total as f32) * 100.0 } else { 100.0 };
                files.push(FileCoverage { path, covered, total, pct, uncovered_lines: uncovered.clone() });
            }
        }
    }
    files
}

/// Parse `go test -coverprofile` output into FileCoverage entries.
fn parse_go_coverage(cov: &str) -> Vec<FileCoverage> {
    use std::collections::HashMap;
    // Format: "pkg/file.go:start.col,end.col numStmts count"
    let mut data: HashMap<String, (u32, u32, Vec<u32>)> = HashMap::new();

    for line in cov.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 { continue; }
        let count: i64 = parts[2].parse().unwrap_or(0);
        let path = parts[0].split(':').next().unwrap_or("").to_string();
        let start_line: u32 = parts[0].split(':').nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.split('.').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let entry = data.entry(path).or_insert((0, 0, Vec::new()));
        entry.1 += 1;
        if count > 0 { entry.0 += 1; } else { entry.2.push(start_line); }
    }

    data.into_iter().map(|(path, (cov, tot, unc))| {
        let pct = if tot > 0 { (cov as f32 / tot as f32) * 100.0 } else { 100.0 };
        FileCoverage { path, covered: cov, total: tot, pct, uncovered_lines: unc }
    }).collect()
}

/// Extract the first percentage value from raw command output as a fallback.
fn extract_pct_from_raw(raw: &str) -> f32 {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"(\d+(?:\.\d+)?)\s*%").unwrap());
    for cap in re.captures_iter(raw) {
        if let Ok(pct) = cap[1].parse::<f32>() {
            return pct;
        }
    }
    0.0
}

// ══════════════════════════════════════════════════════════════════════════════
// Phase 44 — Multi-Model Comparison
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub provider: String,
    pub model: String,
    pub content: String,
    pub duration_ms: u64,
    pub tokens: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareResult {
    pub a: ModelResponse,
    pub b: ModelResponse,
}

/// Build a temporary provider instance by type name (reads API key from env).
fn build_temp_provider(provider_type: &str, model: &str)
    -> Option<Arc<dyn vibe_ai::provider::AIProvider>>
{
    use vibe_ai::providers;
    use vibe_ai::provider::ProviderConfig;

    let cfg = ProviderConfig {
        provider_type: provider_type.to_string(),
        api_key: match provider_type {
            "claude" | "anthropic" => std::env::var("ANTHROPIC_API_KEY").ok(),
            "openai"               => std::env::var("OPENAI_API_KEY").ok(),
            "gemini"               => std::env::var("GEMINI_API_KEY").ok(),
            "grok"                 => std::env::var("GROK_API_KEY").ok(),
            "groq"                 => std::env::var("GROQ_API_KEY").ok(),
            "ollama"               => Some(String::new()),
            _                      => None,
        },
        model: model.to_string(),
        ..Default::default()
    };

    let p: Arc<dyn vibe_ai::provider::AIProvider> = match provider_type {
        "claude" | "anthropic" => Arc::new(providers::ClaudeProvider::new(cfg)),
        "openai"               => Arc::new(providers::OpenAIProvider::new(cfg)),
        "gemini"               => Arc::new(providers::GeminiProvider::new(cfg)),
        "grok"                 => Arc::new(providers::GrokProvider::new(cfg)),
        "groq"                 => Arc::new(providers::GroqProvider::new(cfg)),
        "ollama"               => Arc::new(providers::OllamaProvider::new(cfg)),
        _                      => return None,
    };
    Some(p)
}

/// Call a single provider with a prompt and return a `ModelResponse`.
async fn call_provider(provider_type: &str, model: &str, prompt: &str) -> ModelResponse {
    use vibe_ai::provider::{Message, MessageRole};
    let start = std::time::Instant::now();
    let messages = vec![Message { role: MessageRole::User, content: prompt.to_string() }];

    let Some(provider) = build_temp_provider(provider_type, model) else {
        return ModelResponse {
            provider: provider_type.to_string(), model: model.to_string(),
            content: String::new(), duration_ms: 0, tokens: None,
            error: Some(format!("Provider '{provider_type}' is not configured")),
        };
    };

    match provider.chat_response(&messages, None).await {
        Ok(resp) => ModelResponse {
            provider: provider_type.to_string(),
            model: model.to_string(),
            content: resp.text,
            duration_ms: start.elapsed().as_millis() as u64,
            tokens: resp.usage.map(|u| u.total()),
            error: None,
        },
        Err(e) => ModelResponse {
            provider: provider_type.to_string(),
            model: model.to_string(),
            content: String::new(),
            duration_ms: start.elapsed().as_millis() as u64,
            tokens: None,
            error: Some(e.to_string()),
        },
    }
}

/// Send the same prompt to two providers in parallel and return both responses.
#[tauri::command]
pub async fn compare_models(
    prompt: String,
    provider_a: String,
    model_a: String,
    provider_b: String,
    model_b: String,
) -> Result<CompareResult, String> {
    let (a, b) = tokio::join!(
        call_provider(&provider_a, &model_a, &prompt),
        call_provider(&provider_b, &model_b, &prompt),
    );
    Ok(CompareResult { a, b })
}

// ══════════════════════════════════════════════════════════════════════════════
// Phase 44 — HTTP Playground
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestHeader {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<HttpRequestHeader>,
    pub body: String,
    pub duration_ms: u64,
}

/// Send an HTTP request and return the response.
#[tauri::command]
pub async fn send_http_request(
    method: String,
    url: String,
    headers: Vec<HttpRequestHeader>,
    body: Option<String>,
) -> Result<HttpResponseData, String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let method_parsed = reqwest::Method::from_bytes(method.to_uppercase().as_bytes())
        .map_err(|e| e.to_string())?;

    let mut req = client.request(method_parsed, &url);
    for h in &headers {
        req = req.header(h.key.as_str(), h.value.as_str());
    }
    if let Some(b) = body {
        req = req.body(b);
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let duration_ms = start.elapsed().as_millis() as u64;
    let status = resp.status();
    let resp_headers: Vec<HttpRequestHeader> = resp.headers().iter()
        .map(|(k, v)| HttpRequestHeader {
            key: k.to_string(),
            value: v.to_str().unwrap_or("").to_string(),
        })
        .collect();
    let body_text = resp.text().await.map_err(|e| e.to_string())?;

    Ok(HttpResponseData {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers: resp_headers,
        body: body_text,
        duration_ms,
    })
}

/// Grep the workspace for common API route patterns.
#[tauri::command]
pub async fn discover_api_endpoints(workspace: String) -> Result<Vec<String>, String> {
    static PATTERNS: &[&str] = &[
        r"app\.(get|post|put|delete|patch)\s*\(",
        r"router\.(get|post|put|delete|patch)\s*\(",
        r#"@(Get|Post|Put|Delete|Patch)\s*\("#,
        r"\.route\s*\(",
        r#"axum::Router::new\(\)"#,
    ];
    let compiled: Vec<regex::Regex> = PATTERNS.iter()
        .filter_map(|p| regex::Regex::new(p).ok())
        .collect();

    let ws = PathBuf::from(&workspace);
    let mut endpoints = Vec::new();

    for entry in walkdir::WalkDir::new(&ws)
        .follow_links(false)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("");
            matches!(ext, "js" | "ts" | "jsx" | "tsx" | "rs" | "py" | "go" | "java")
        })
        .take(500)
    {
        if endpoints.len() >= 60 { break; }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for line in content.lines() {
                let trimmed = line.trim().to_string();
                if compiled.iter().any(|re| re.is_match(&trimmed)) {
                    if !endpoints.contains(&trimmed) {
                        endpoints.push(trimmed);
                        if endpoints.len() >= 60 { break; }
                    }
                }
            }
        }
    }
    Ok(endpoints)
}

// ══════════════════════════════════════════════════════════════════════════════
// Phase 44b — Arena Mode (blind A/B voting)
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaVote {
    pub timestamp: String,
    pub prompt: String,
    pub provider_a: String,
    pub model_a: String,
    pub provider_b: String,
    pub model_b: String,
    pub winner: String, // "a", "b", "tie", "both_bad"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaStats {
    pub provider: String,
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
    pub total: u32,
    pub win_rate: f64,
}

/// Save an arena vote to ~/.vibeui/arena-votes.json
#[tauri::command]
pub async fn save_arena_vote(vote: ArenaVote) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join(".vibeui");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("arena-votes.json");
    let mut votes: Vec<ArenaVote> = if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_else(|_| "[]".into());
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    };
    votes.push(vote);
    let json = serde_json::to_string_pretty(&votes).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load arena vote history and compute per-provider stats.
#[tauri::command]
pub async fn get_arena_history() -> Result<(Vec<ArenaVote>, Vec<ArenaStats>), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(home).join(".vibeui").join("arena-votes.json");
    let votes: Vec<ArenaVote> = if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_else(|_| "[]".into());
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Compute per-provider stats from vote history
    let mut stats_map: std::collections::HashMap<String, (u32, u32, u32)> =
        std::collections::HashMap::new();
    for v in &votes {
        let entry_a = stats_map.entry(v.provider_a.clone()).or_insert((0, 0, 0));
        match v.winner.as_str() {
            "a" => { entry_a.0 += 1; }
            "b" => { entry_a.1 += 1; }
            "tie" => { entry_a.2 += 1; }
            _ => {} // "both_bad" — no score change
        }
        let entry_b = stats_map.entry(v.provider_b.clone()).or_insert((0, 0, 0));
        match v.winner.as_str() {
            "a" => { entry_b.1 += 1; }
            "b" => { entry_b.0 += 1; }
            "tie" => { entry_b.2 += 1; }
            _ => {}
        }
    }
    let stats: Vec<ArenaStats> = stats_map
        .into_iter()
        .map(|(provider, (wins, losses, ties))| {
            let total = wins + losses + ties;
            let win_rate = if total > 0 {
                (wins as f64) / (total as f64)
            } else {
                0.0
            };
            ArenaStats { provider, wins, losses, ties, total, win_rate }
        })
        .collect();

    Ok((votes, stats))
}

// ══════════════════════════════════════════════════════════════════════════════
// Phase 45 — Cost & Performance Observatory
// ══════════════════════════════════════════════════════════════════════════════

/// A single AI call cost record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost_usd: f64,
    pub timestamp_ms: u64,
    pub task_hint: Option<String>,
}

/// Per-provider aggregate for the cost dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCostSummary {
    pub provider: String,
    pub total_cost_usd: f64,
    pub total_tokens: u32,
    pub call_count: u32,
}

/// Full cost metrics payload returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostMetrics {
    pub entries: Vec<CostEntry>,
    pub by_provider: Vec<ProviderCostSummary>,
    pub total_cost_usd: f64,
    pub total_tokens: u32,
    pub budget_limit_usd: Option<f64>,
    pub budget_remaining_usd: Option<f64>,
}

fn cost_log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vibeui").join("cost-log.jsonl")
}

fn cost_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vibeui").join("cost-config.json")
}

/// Append a cost entry to the JSONL log. Called from send_chat_message / agent flow.
#[tauri::command]
pub async fn record_cost_entry(
    session_id: String,
    provider: String,
    model: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    task_hint: Option<String>,
) -> Result<(), String> {
    use vibe_ai::provider::TokenUsage;
    let usage = TokenUsage { prompt_tokens, completion_tokens };
    let cost_usd = usage.estimated_cost_usd(&provider, &model);
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let entry = CostEntry { session_id, provider, model, prompt_tokens, completion_tokens, cost_usd, timestamp_ms, task_hint };
    let line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;

    let path = cost_log_path();
    if let Some(p) = path.parent() { let _ = std::fs::create_dir_all(p); }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    writeln!(file, "{}", line).map_err(|e| e.to_string())
}

/// Load all cost entries and compute aggregates.
#[tauri::command]
pub async fn get_cost_metrics() -> Result<CostMetrics, String> {
    // Load entries
    let log_path = cost_log_path();
    let mut entries: Vec<CostEntry> = Vec::new();
    if log_path.exists() {
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        for line in content.lines() {
            if let Ok(e) = serde_json::from_str::<CostEntry>(line) {
                entries.push(e);
            }
        }
    }

    // Sort newest first
    entries.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));

    // Aggregate by provider
    let mut by_provider: std::collections::HashMap<String, ProviderCostSummary> = std::collections::HashMap::new();
    for e in &entries {
        let s = by_provider.entry(e.provider.clone()).or_insert(ProviderCostSummary {
            provider: e.provider.clone(), total_cost_usd: 0.0, total_tokens: 0, call_count: 0,
        });
        s.total_cost_usd += e.cost_usd;
        s.total_tokens += e.prompt_tokens + e.completion_tokens;
        s.call_count += 1;
    }
    let mut by_provider_vec: Vec<ProviderCostSummary> = by_provider.into_values().collect();
    by_provider_vec.sort_by(|a, b| b.total_cost_usd.partial_cmp(&a.total_cost_usd).unwrap_or(std::cmp::Ordering::Equal));

    let total_cost_usd: f64 = entries.iter().map(|e| e.cost_usd).sum();
    let total_tokens: u32 = entries.iter().map(|e| e.prompt_tokens + e.completion_tokens).sum();

    // Load budget limit
    let budget_limit_usd = if cost_config_path().exists() {
        std::fs::read_to_string(cost_config_path()).ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v["budget_limit_usd"].as_f64())
    } else {
        None
    };

    let budget_remaining_usd = budget_limit_usd.map(|lim| (lim - total_cost_usd).max(0.0));

    Ok(CostMetrics { entries, by_provider: by_provider_vec, total_cost_usd, total_tokens, budget_limit_usd, budget_remaining_usd })
}

/// Set or clear the monthly budget limit.
#[tauri::command]
pub async fn set_cost_limit(limit_usd: Option<f64>) -> Result<(), String> {
    let path = cost_config_path();
    if let Some(p) = path.parent() { let _ = std::fs::create_dir_all(p); }
    let json = serde_json::json!({ "budget_limit_usd": limit_usd });
    std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|e| e.to_string())
}

/// Clear all cost history.
#[tauri::command]
pub async fn clear_cost_history() -> Result<(), String> {
    let path = cost_log_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Phase 45 — AI Git Workflow Enhancements
// ══════════════════════════════════════════════════════════════════════════════

/// Suggest a git branch name for a given task description.
#[tauri::command]
pub async fn suggest_branch_name(
    state: tauri::State<'_, AppState>,
    task_description: String,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};
    let engine = state.chat_engine.lock().await;
    let prompt = format!(
        "Generate a concise, lowercase, hyphen-separated git branch name for this task (no spaces, \
         no special chars except hyphens, max 50 chars, just the name with no explanation):\n\n{}",
        task_description.trim()
    );
    let messages = vec![Message { role: MessageRole::User, content: prompt }];
    let result = engine.chat(&messages, None).await.map_err(|e| e.to_string())?;
    // Clean up: strip quotes, backticks, whitespace
    let name = result.trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .to_lowercase()
        .replace(' ', "-");
    Ok(name)
}

/// AI-assisted merge conflict resolution.
#[tauri::command]
pub async fn resolve_merge_conflict(
    state: tauri::State<'_, AppState>,
    file_path: String,
    conflict_text: String,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};
    let engine = state.chat_engine.lock().await;
    let prompt = format!(
        "You are a code merge conflict resolver. Analyze this merge conflict and return ONLY the \
         resolved code (no explanation, no markdown fences). Choose the best resolution that \
         preserves functionality from both sides, or ours if ambiguous.\n\
         \nFile: {}\n\n```\n{}\n```",
        file_path, conflict_text
    );
    let messages = vec![Message { role: MessageRole::User, content: prompt }];
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

/// Generate a CHANGELOG entry from recent git commits.
#[tauri::command]
pub async fn generate_changelog(
    state: tauri::State<'_, AppState>,
    workspace: String,
    since_ref: Option<String>,
) -> Result<String, String> {
    use vibe_ai::provider::{Message, MessageRole};
    // Get git log
    let since = since_ref.as_deref().unwrap_or("HEAD~20");
    let log_output = tokio::process::Command::new("git")
        .args(["log", &format!("{}..HEAD", since), "--oneline", "--no-merges"])
        .current_dir(&workspace)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let log = String::from_utf8_lossy(&log_output.stdout).to_string();
    if log.trim().is_empty() {
        return Ok("No new commits found since the specified reference.".to_string());
    }

    let engine = state.chat_engine.lock().await;
    let prompt = format!(
        "Convert these git commits into a concise, user-facing CHANGELOG entry in Keep a Changelog \
         format (## [Unreleased] section with ### Added / ### Fixed / ### Changed subsections as \
         appropriate). Group related commits. Use imperative mood. Return only the markdown:\n\n{}",
        log
    );
    let messages = vec![Message { role: MessageRole::User, content: prompt }];
    engine.chat(&messages, None).await.map_err(|e| e.to_string())
}

// ══════════════════════════════════════════════════════════════════════════════
// Phase 45 — Codemod & Lint Auto-Fix
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutofixResult {
    pub framework: String,
    pub files_changed: u32,
    pub diff: String,
    pub stdout: String,
}

/// Run the linter in auto-fix mode and return the resulting diff.
#[tauri::command]
pub async fn run_autofix(workspace: String, framework: Option<String>) -> Result<AutofixResult, String> {
    let ws = PathBuf::from(&workspace);

    // Auto-detect framework if not specified
    let fw = match framework.as_deref() {
        Some(f) => f.to_string(),
        None => {
            if ws.join("Cargo.toml").exists() { "clippy".to_string() }
            else if ws.join("package.json").exists() { "eslint".to_string() }
            else if ws.join("pyproject.toml").exists() || ws.join("setup.py").exists() { "ruff".to_string() }
            else if ws.join("go.mod").exists() { "gofmt".to_string() }
            else { return Err("Cannot detect linter framework".to_string()); }
        }
    };

    // Run fix command
    let (prog, args): (&str, &[&str]) = match fw.as_str() {
        "clippy"  => ("cargo", &["clippy", "--fix", "--allow-dirty", "--allow-staged", "-q"]),
        "eslint"  => ("npx",   &["eslint", "--fix", "."]),
        "ruff"    => ("ruff",  &["check", "--fix", "."]),
        "gofmt"   => ("gofmt", &["-w", "."]),
        "prettier"=> ("npx",   &["prettier", "--write", "."]),
        _         => return Err(format!("Unknown autofix framework: {fw}")),
    };

    let output = tokio::process::Command::new(prog)
        .args(args)
        .current_dir(&ws)
        .output()
        .await
        .map_err(|e| format!("Failed to run autofix: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    // Get the diff of changes
    let diff_stat = tokio::process::Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(&ws)
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let diff = tokio::process::Command::new("git")
        .args(["diff", "--unified=3"])
        .current_dir(&ws)
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Count changed files from stat
    let files_changed = diff_stat.lines()
        .filter(|l| l.contains('|'))
        .count() as u32;

    Ok(AutofixResult { framework: fw, files_changed, diff, stdout })
}

/// Apply or revert an autofix: stage all changes (apply) or restore (revert).
#[tauri::command]
pub async fn apply_autofix(workspace: String, apply: bool) -> Result<(), String> {
    let ws = PathBuf::from(&workspace);
    let args: &[&str] = if apply {
        &["add", "-u"]
    } else {
        &["restore", "--staged", "."]
    };
    tokio::process::Command::new("git")
        .args(args)
        .current_dir(&ws)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !apply {
        // Also restore working tree
        tokio::process::Command::new("git")
            .args(["restore", "."])
            .current_dir(&ws)
            .output()
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_rule_meta ───────────────────────────────────────────────────────

    #[test]
    fn parse_rule_meta_no_frontmatter_uses_stem_as_name() {
        let meta = parse_rule_meta("Just plain content.\n", "rust-safety.md");
        assert_eq!(meta.filename, "rust-safety.md");
        assert_eq!(meta.name, "rust-safety");
        assert!(meta.path_pattern.is_none());
    }

    #[test]
    fn parse_rule_meta_full_frontmatter() {
        let content = "---\nname: my-rule\npath_pattern: \"**/*.rs\"\n---\n\nRule body here.\n";
        let meta = parse_rule_meta(content, "my-rule.md");
        assert_eq!(meta.name, "my-rule");
        assert_eq!(meta.path_pattern.as_deref(), Some("**/*.rs"));
    }

    #[test]
    fn parse_rule_meta_name_only_frontmatter() {
        let content = "---\nname: custom-name\n---\n\nContent.\n";
        let meta = parse_rule_meta(content, "filename.md");
        assert_eq!(meta.name, "custom-name");
        assert!(meta.path_pattern.is_none());
    }

    #[test]
    fn parse_rule_meta_path_pattern_only_uses_stem_name() {
        let content = "---\npath_pattern: '**/*.ts'\n---\n\nContent.\n";
        let meta = parse_rule_meta(content, "typescript.md");
        assert_eq!(meta.name, "typescript");          // falls back to file stem
        assert_eq!(meta.path_pattern.as_deref(), Some("**/*.ts"));
    }

    #[test]
    fn parse_rule_meta_single_quoted_values() {
        let content = "---\nname: 'quoted-name'\npath_pattern: '**/*.py'\n---\n\nBody.\n";
        let meta = parse_rule_meta(content, "q.md");
        assert_eq!(meta.name, "quoted-name");
        assert_eq!(meta.path_pattern.as_deref(), Some("**/*.py"));
    }

    #[test]
    fn parse_rule_meta_empty_content() {
        let meta = parse_rule_meta("", "empty.md");
        assert_eq!(meta.name, "empty");
        assert!(meta.path_pattern.is_none());
    }

    // ── rules_dir ─────────────────────────────────────────────────────────────

    #[test]
    fn rules_dir_workspace_appends_vibecli_rules() {
        let root = std::path::Path::new("/some/project");
        let dir = rules_dir("workspace", Some(root));
        assert_eq!(dir, std::path::PathBuf::from("/some/project/.vibecli/rules"));
    }

    #[test]
    fn rules_dir_global_uses_home_vibecli_rules() {
        let dir = rules_dir("global", None);
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        assert_eq!(dir, std::path::PathBuf::from(&home).join(".vibecli").join("rules"));
    }

    #[test]
    fn rules_dir_workspace_without_root_falls_back_to_dot() {
        let dir = rules_dir("workspace", None);
        assert_eq!(dir, std::path::PathBuf::from("./.vibecli/rules"));
    }

    // ── mcp_config_path ───────────────────────────────────────────────────────

    #[test]
    fn mcp_config_path_is_inside_vibeui_home_dir() {
        let path = mcp_config_path();
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        assert_eq!(path, std::path::PathBuf::from(&home).join(".vibeui").join("mcp.json"));
    }

    #[test]
    fn mcp_config_path_ends_with_json() {
        let path = mcp_config_path();
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("json"));
    }

    // ── RuleFileMeta serialization ────────────────────────────────────────────

    #[test]
    fn rule_file_meta_serializes_to_json() {
        let meta = RuleFileMeta {
            filename: "rust.md".to_string(),
            name: "rust".to_string(),
            path_pattern: Some("**/*.rs".to_string()),
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"filename\":\"rust.md\""));
        assert!(json.contains("\"path_pattern\":\"**/*.rs\""));
    }

    #[test]
    fn rule_file_meta_null_path_pattern_serializes() {
        let meta = RuleFileMeta {
            filename: "always.md".to_string(),
            name: "always".to_string(),
            path_pattern: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"path_pattern\":null"));
    }

    // ── McpToolInfo serialization ─────────────────────────────────────────────

    #[test]
    fn mcp_tool_info_roundtrips_json() {
        let tool = McpToolInfo {
            name: "list_repos".to_string(),
            description: "Lists all repos".to_string(),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let back: McpToolInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "list_repos");
        assert_eq!(back.description, "Lists all repos");
    }
}
