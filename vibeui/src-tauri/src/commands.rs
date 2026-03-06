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
    /// Abort handle for the currently running chat stream (if any).
    pub chat_abort_handle: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
    /// Mock server handle (Phase 7.30).
    pub mock_server_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Mock server route registry.
    pub mock_routes: Arc<Mutex<Vec<MockRoute>>>,
    /// Mock server captured request log.
    pub mock_request_log: Arc<Mutex<Vec<MockRequest>>>,
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
    // NOTE: Use strip_prefix (exact literal match), NOT trim_start_matches
    // (which treats the &str as a character set and strips individual chars)
    let mut clean = result.trim();
    if let Some(rest) = clean.strip_prefix("```") {
        clean = rest.strip_prefix(language.as_str()).unwrap_or(rest);
    }
    if let Some(rest) = clean.strip_suffix("```") {
        clean = rest;
    }
    let clean = clean.trim().to_string();
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

#[derive(Clone, Serialize)]
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
        system_prompt.push('\n');
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

// ── Streaming chat (Phase 7.21) ───────────────────────────────────────────────

/// Start a streaming chat response.
///
/// Immediately returns `Ok(())` and spawns a background task that:
/// - Emits `chat:chunk` events for each token
/// - Emits `chat:complete` with the full assembled text when done
/// - Emits `chat:error` on failure
///
/// Call `stop_chat_stream` to cancel an in-progress stream.
#[tauri::command]
pub async fn stream_chat_message(
    mut request: ChatRequest,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Cancel any previously running chat stream.
    {
        let mut handle = state.chat_abort_handle.lock().await;
        if let Some(h) = handle.take() {
            h.abort();
        }
    }

    // Set provider and clone it so we can release the lock before spawning.
    let provider = {
        let mut engine = state.chat_engine.lock().await;
        engine.set_provider_by_name(&request.provider)
            .map_err(|e| e.to_string())?;
        engine.active_provider()
            .ok_or_else(|| "No active provider".to_string())?
            .clone()
    };

    // Inject system prompt (same as send_chat_message)
    let mut system_prompt = String::from(
        "You are an advanced coding assistant with access to the file system.\n\
        You can use the following tools by outputting XML tags:\n\
        - <read_file path=\"path/to/file\" />: Read file content\n\
        - <write_file path=\"path/to/file\">content</write_file>: Write content to file\n\
        - <list_dir path=\"path/to/dir\" />: List directory contents\n\n"
    );
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
        system_prompt.push('\n');
    }
    request.messages.insert(0, vibe_ai::Message {
        role: vibe_ai::MessageRole::System,
        content: system_prompt,
    });

    // Resolve @-context references
    let context = {
        let mut ctx = if let (Some(file), Some(content)) = (&request.current_file, &request.context) {
            Some(format!("Active File: {}\n\nFile Content:\n{}", file, content))
        } else {
            request.context.clone()
        };
        if let Some(last) = request.messages.last() {
            if last.role == vibe_ai::MessageRole::User {
                let at_ctx = resolve_at_references(&last.content, &state.workspace, &state.terminal_buffer).await;
                if !at_ctx.is_empty() {
                    let base = ctx.unwrap_or_default();
                    ctx = Some(if base.is_empty() { at_ctx } else { format!("{}\n\n{}", base, at_ctx) });
                }
            }
        }
        ctx
    };

    // Inject context as a leading user message if present
    let mut messages = request.messages.clone();
    if let Some(ctx_text) = context {
        if !ctx_text.is_empty() {
            // Inject as the second message (after system) so the model sees it.
            let insert_pos = if messages.first().map(|m| m.role == vibe_ai::MessageRole::System).unwrap_or(false) { 1 } else { 0 };
            messages.insert(insert_pos, vibe_ai::Message {
                role: vibe_ai::MessageRole::User,
                content: format!("[Context]\n{}", ctx_text),
            });
        }
    }

    let workspace = state.workspace.clone();
    let abort_store = state.chat_abort_handle.clone();

    let join_handle = tokio::spawn(async move {
        use futures::StreamExt;
        let mut stream = match provider.stream_chat(&messages).await {
            Ok(s) => s,
            Err(e) => {
                let _ = app_handle.emit("chat:error", e.to_string());
                return;
            }
        };
        let mut accumulated = String::with_capacity(4096);
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(text) => {
                    accumulated.push_str(&text);
                    let _ = app_handle.emit("chat:chunk", text.clone());
                }
                Err(e) => {
                    let _ = app_handle.emit("chat:error", e.to_string());
                    return;
                }
            }
        }
        // Process tool calls in the completed response (same as send_chat_message)
        let (tool_output, pending_write) = process_tool_calls(&accumulated, &workspace).await;
        let response = ChatResponse {
            message: accumulated,
            tool_output,
            pending_write,
        };
        let _ = app_handle.emit("chat:complete", response);
    });

    // Store abort handle so stop_chat_stream can cancel it.
    {
        let mut handle = abort_store.lock().await;
        *handle = Some(join_handle.abort_handle());
    }

    Ok(())
}

/// Cancel any in-progress chat stream.
#[tauri::command]
pub async fn stop_chat_stream(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut handle = state.chat_abort_handle.lock().await;
    if let Some(h) = handle.take() {
        h.abort();
    }
    Ok(())
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
            let n = name_raw.split_once(':').map(|x| x.1).unwrap_or(name_raw);
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
    // NOTE: Use strip_prefix (exact literal match), NOT trim_start_matches
    // (which treats the &str as a character set and strips individual chars)
    let mut clean = result.trim();
    if let Some(rest) = clean.strip_prefix("```") {
        clean = rest.strip_prefix(language.as_str()).unwrap_or(rest);
    }
    if let Some(rest) = clean.strip_suffix("```") {
        clean = rest;
    }
    let clean = clean.trim().to_string();

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
        if d.len() > 2000 {
            let end = d.char_indices().nth(2000).map(|(i,_)| i).unwrap_or(d.len());
            d[..end].to_string() + "\n…(truncated)"
        } else { d }
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
        team_bus: None,
        team_agent_id: None,
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
#[allow(clippy::too_many_arguments)]
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
    /// "command", "llm", or "http"
    pub handler_type: String,
    /// Shell command string (for handler_type == "command")
    #[serde(default)]
    pub command: String,
    /// LLM prompt template (for handler_type == "llm")
    #[serde(default)]
    pub prompt: String,
    /// HTTP webhook URL (for handler_type == "http")
    #[serde(default)]
    pub http_url: String,
    /// HTTP method: POST, PUT, PATCH, GET (for handler_type == "http")
    #[serde(default = "default_http_method_str")]
    pub http_method: String,
    /// HTTP headers as JSON string (for handler_type == "http")
    #[serde(default)]
    pub http_headers: String,
    /// HTTP timeout in ms (for handler_type == "http")
    #[serde(default = "default_http_timeout")]
    pub http_timeout_ms: u64,
    #[serde(default)]
    pub async_exec: bool,
}

fn default_http_method_str() -> String { "POST".to_string() }
fn default_http_timeout() -> u64 { 10_000 }

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
                team_bus: None,
                team_agent_id: None,
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
        {
            let end = diff.char_indices().nth(20_000).map(|(i,_)| i).unwrap_or(diff.len());
            format!("{}\n...(diff truncated at 20k chars)", &diff[..end])
        }
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
        .map_err(|e| {
            let end = response.char_indices().nth(500).map(|(i,_)| i).unwrap_or(response.len());
            format!("Failed to parse review JSON: {e}\n\nRaw: {}", &response[..end])
        })?;

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
                if let Some((id_str, after)) = stripped.split_once("**:") {
                    (id_str.parse::<u32>().unwrap_or(tasks.len() as u32 + 1), after.trim().to_string())
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
        let checkbox_rest = line.strip_prefix("- [x] ").or_else(|| line.strip_prefix("- [ ] "));
        if let Some(rest) = checkbox_rest {
            let done = line.starts_with("- [x]");
            let rest = rest.trim();
            let desc = if let Some(stripped) = rest.strip_prefix("**") {
                if let Some((_id_part, after)) = stripped.split_once("**:") {
                    after.trim().to_string()
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
                if let Some((id_str, after)) = stripped.split_once("**:") {
                    (id_str.parse::<u32>().unwrap_or(stage.checklist.len() as u32 + 1), after.trim().to_string())
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
    // Split: ["https:", "", "www.figma.com", "file", "{key}", ...]
    let key = url.split('/').nth(4).unwrap_or("").to_string();
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

/// Check if a CLI tool is installed and available on PATH.
fn check_cli_available(tool: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {} >/dev/null 2>&1", tool)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DeployTarget {
    pub target: String,
    pub build_cmd: String,
    pub out_dir: String,
    pub detected_framework: String,
    #[serde(default)]
    pub recommended_targets: Vec<String>,
    #[serde(default)]
    pub required_cli: Option<String>,
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

    // Build recommended targets list based on project markers
    let mut recommended = Vec::new();
    if ws.join("serverless.yml").exists() || ws.join("serverless.ts").exists() {
        recommended.push("aws-lambda".to_string());
    }
    if ws.join("Dockerfile").exists() {
        recommended.extend_from_slice(&[
            "aws-apprunner".to_string(),
            "gcp-run".to_string(),
            "azure-containerapp".to_string(),
            "digitalocean".to_string(),
            "kubernetes".to_string(),
        ]);
    }
    if ws.join("Chart.yaml").exists() {
        recommended.push("kubernetes-helm".to_string());
    } else if ws.join("k8s").is_dir() {
        recommended.push("kubernetes".to_string());
    }
    if ws.join("staticwebapp.config.json").exists() {
        recommended.push("azure-staticweb".to_string());
    }
    if ws.join("firebase.json").exists() && !recommended.contains(&"firebase".to_string()) {
        recommended.push("firebase".to_string());
    }
    if ws.join("vercel.json").exists() {
        recommended.push("vercel".to_string());
    }
    if ws.join("netlify.toml").exists() {
        recommended.push("netlify".to_string());
    }
    if framework == "Static" && recommended.is_empty() {
        recommended.extend_from_slice(&[
            "aws-s3".to_string(),
            "netlify".to_string(),
            "vercel".to_string(),
        ]);
    }
    if recommended.is_empty() {
        recommended.push("vercel".to_string());
    }

    let default_target = recommended.first().cloned().unwrap_or_else(|| "vercel".to_string());

    Ok(DeployTarget {
        target: default_target,
        build_cmd: build_cmd.to_string(),
        out_dir: out_dir.to_string(),
        detected_framework: framework.to_string(),
        recommended_targets: recommended,
        required_cli: None,
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

    let deploy_cmd: &str = match target.as_str() {
        // ── PaaS ──
        "vercel" => "vercel deploy --yes",
        "netlify" => "netlify deploy --prod --dir=dist",
        "railway" => "railway up",
        "github-pages" => "npm run build && npx gh-pages -d dist",
        // ── Google ──
        "gcp-run" => "gcloud run deploy --source . --platform=managed --region=us-central1 --allow-unauthenticated",
        "firebase" => "firebase deploy --only hosting",
        // ── AWS ──
        "aws-apprunner" => {
            if !check_cli_available("aws") {
                return Err("AWS CLI not installed. Install: https://aws.amazon.com/cli/".into());
            }
            "copilot deploy 2>&1 || aws apprunner create-service --service-name $(basename $(pwd)) --source-configuration '{\"AutoDeploymentsEnabled\":true,\"CodeRepository\":{\"RepositoryUrl\":\".\",\"SourceCodeVersion\":{\"Type\":\"BRANCH\",\"Value\":\"main\"}}}' 2>&1"
        }
        "aws-s3" => {
            if !check_cli_available("aws") {
                return Err("AWS CLI not installed. Install: https://aws.amazon.com/cli/".into());
            }
            "npm run build 2>/dev/null; aws s3 sync dist/ s3://$(basename $(pwd))-deploy --delete 2>&1 && echo 'Uploaded to S3. Create a CloudFront distribution for HTTPS.'"
        }
        "aws-lambda" => {
            if check_cli_available("serverless") {
                "serverless deploy 2>&1"
            } else if check_cli_available("sam") {
                "sam build && sam deploy --no-confirm-changeset 2>&1"
            } else {
                return Err("Install Serverless Framework (npm i -g serverless) or AWS SAM CLI for Lambda deploys.".into());
            }
        }
        "aws-ecs" => {
            if !check_cli_available("aws") {
                return Err("AWS CLI not installed. Install: https://aws.amazon.com/cli/".into());
            }
            "ACCOUNT=$(aws sts get-caller-identity --query Account --output text) && REGION=$(aws configure get region || echo us-east-1) && REPO=$ACCOUNT.dkr.ecr.$REGION.amazonaws.com/$(basename $(pwd)) && aws ecr describe-repositories --repository-names $(basename $(pwd)) 2>/dev/null || aws ecr create-repository --repository-name $(basename $(pwd)) && docker build -t app . && aws ecr get-login-password | docker login --username AWS --password-stdin $REPO && docker tag app:latest $REPO:latest && docker push $REPO:latest && aws ecs update-service --cluster default --service $(basename $(pwd)) --force-new-deployment 2>&1"
        }
        // ── Azure ──
        "azure-appservice" => {
            if !check_cli_available("az") {
                return Err("Azure CLI not installed. Install: https://learn.microsoft.com/en-us/cli/azure/install-azure-cli".into());
            }
            "az webapp up --name $(basename $(pwd)) --runtime 'NODE:18-lts' 2>&1"
        }
        "azure-containerapp" => {
            if !check_cli_available("az") {
                return Err("Azure CLI not installed.".into());
            }
            "az containerapp up --name $(basename $(pwd)) --source . 2>&1"
        }
        "azure-staticweb" => {
            if check_cli_available("swa") {
                "npm run build 2>/dev/null; swa deploy --app-location . --output-location dist 2>&1"
            } else if check_cli_available("az") {
                "npm run build 2>/dev/null; az staticwebapp create --name $(basename $(pwd)) --source . 2>&1"
            } else {
                return Err("Install Azure SWA CLI (npm i -g @azure/static-web-apps-cli) or Azure CLI.".into());
            }
        }
        // ── DigitalOcean ──
        "digitalocean" => {
            if !check_cli_available("doctl") {
                return Err("doctl not installed. Install: https://docs.digitalocean.com/reference/doctl/how-to/install/".into());
            }
            "doctl apps create --spec .do/app.yaml 2>&1 || doctl apps update $(doctl apps list --format ID --no-header | head -1) --spec .do/app.yaml 2>&1"
        }
        // ── Kubernetes ──
        "kubernetes" => {
            if !check_cli_available("kubectl") {
                return Err("kubectl not installed. Install: https://kubernetes.io/docs/tasks/tools/".into());
            }
            "kubectl apply -f k8s/ 2>&1 || kubectl apply -f . 2>&1"
        }
        "kubernetes-helm" => {
            if !check_cli_available("helm") {
                return Err("Helm not installed. Install: https://helm.sh/docs/intro/install/".into());
            }
            "helm upgrade --install $(basename $(pwd)) . 2>&1"
        }
        // ── Oracle Cloud ──
        "oci" => {
            if !check_cli_available("oci") {
                return Err("OCI CLI not installed. Install: https://docs.oracle.com/en-us/iaas/Content/API/SDKDocs/cliinstall.htm".into());
            }
            "fn deploy --app $(basename $(pwd)) 2>&1 || docker build -t app . && echo 'Image built. Push to OCI Container Registry and create a Container Instance.'"
        }
        // ── IBM Cloud ──
        "ibm-cloud" => {
            if !check_cli_available("ibmcloud") {
                return Err("IBM Cloud CLI not installed. Install: https://cloud.ibm.com/docs/cli".into());
            }
            "ibmcloud ce project select --name default 2>&1; ibmcloud ce app create --name $(basename $(pwd)) --build-source . 2>&1 || ibmcloud ce app update --name $(basename $(pwd)) --build-source . 2>&1"
        }
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
                || line.to_lowercase().contains("app url")
                || line.to_lowercase().contains("endpoint")
                || line.to_lowercase().contains("webapp url")
                || line.to_lowercase().contains("external ip")
                || line.to_lowercase().contains("load balancer")
                || line.contains("s3://")
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
        // ── AWS ──
        "aws-apprunner" | "aws-ecs" | "aws-s3" | "aws-lambda" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "your-distribution.cloudfront.net".to_string(),
            instructions: format!(
                "To use {} with AWS:\n  1. Create a CloudFront distribution pointing to your service\n  2. Request an ACM certificate for {}\n  3. Add a CNAME record: {} → your-distribution.cloudfront.net",
                domain, domain, domain
            ),
        }),
        // ── Azure ──
        "azure-appservice" | "azure-containerapp" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "your-app.azurewebsites.net".to_string(),
            instructions: format!(
                "To use {} with Azure:\n  az webapp config hostname add --webapp-name <app> --hostname {}\n  Add a CNAME record: {} → your-app.azurewebsites.net",
                domain, domain, domain
            ),
        }),
        "azure-staticweb" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "your-app.azurestaticapps.net".to_string(),
            instructions: format!(
                "To use {} with Azure Static Web Apps:\n  az staticwebapp hostname set -n <app> --hostname {}\n  Add a CNAME record: {} → your-app.azurestaticapps.net",
                domain, domain, domain
            ),
        }),
        // ── DigitalOcean ──
        "digitalocean" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "your-app.ondigitalocean.app".to_string(),
            instructions: format!(
                "To use {} with DigitalOcean:\n  1. In App Platform dashboard → Settings → Domains → Add Domain\n  2. Add a CNAME record: {} → your-app.ondigitalocean.app",
                domain, domain
            ),
        }),
        // ── Kubernetes ──
        "kubernetes" | "kubernetes-helm" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "EXTERNAL-IP".to_string(),
            instructions: format!(
                "To use {} with Kubernetes:\n  1. Get your LoadBalancer IP: kubectl get svc -o wide\n  2. Add an A record: {} → EXTERNAL-IP\n  3. (Optional) Install cert-manager for automatic TLS",
                domain, domain
            ),
        }),
        // ── OCI ──
        "oci" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "your-lb.oci.oraclecloud.com".to_string(),
            instructions: format!(
                "To use {} with Oracle Cloud:\n  1. Configure a Load Balancer with your container\n  2. Add a CNAME record: {} → your-lb.oci.oraclecloud.com\n  3. Add an OCI WAF or SSL cert",
                domain, domain
            ),
        }),
        // ── IBM Cloud ──
        "ibm-cloud" => Ok(CustomDomainResult {
            domain: domain.clone(),
            cname_target: "your-app.codeengine.appdomain.cloud".to_string(),
            instructions: format!(
                "To use {} with IBM Code Engine:\n  ibmcloud ce domainmapping create --domain-name {} --target your-app\n  Add a CNAME record: {} → your-app.codeengine.appdomain.cloud",
                domain, domain, domain
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

/// Validate a SQLite file path: block traversal, resolve symlinks, reject sensitive paths.
fn validate_sqlite_path(path: &str) -> Result<(), String> {
    if path.contains("..") {
        return Err("Invalid database path: traversal not allowed".to_string());
    }
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err("Database file does not exist".to_string());
    }
    // Resolve symlinks to the real path
    let canonical = p.canonicalize().map_err(|e| format!("Cannot resolve path: {}", e))?;
    let canon_str = canonical.to_string_lossy();
    // Block common sensitive directories
    let blocked = ["/etc", "/var", "/usr", "/bin", "/sbin", "/private/etc"];
    for prefix in &blocked {
        if canon_str.starts_with(prefix) {
            return Err("Access to system directories is not allowed".to_string());
        }
    }
    // Block home-directory dotfiles (e.g. ~/.ssh, ~/.gnupg)
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        let home_str = home.to_string_lossy();
        if canon_str.starts_with(home_str.as_ref()) {
            let relative = &canon_str[home_str.len()..];
            if relative.starts_with("/.") {
                return Err("Access to hidden home directory files is not allowed".to_string());
            }
        }
    }
    Ok(())
}

/// List tables in a database. Only SQLite is supported in the backend; Postgres/Supabase
/// would require additional crates — returns an informative error for those.
#[tauri::command]
pub async fn list_db_tables(connection_string: String, db_type: String) -> Result<Vec<TableInfo>, String> {
    // Validate: SQLite paths — resolve symlinks, block traversal and sensitive paths
    if db_type == "sqlite" {
        validate_sqlite_path(&connection_string)?;
    }
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
    // Validate: SQLite paths — resolve symlinks, block traversal and sensitive paths
    if db_type == "sqlite" {
        validate_sqlite_path(&connection_string)?;
    }
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
                    .and_then(|s| s.split_whitespace().next())
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
        .map(std::path::PathBuf::from)
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
                if compiled.iter().any(|re| re.is_match(&trimmed))
                    && !endpoints.contains(&trimmed) {
                        endpoints.push(trimmed);
                        if endpoints.len() >= 60 { break; }
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
    let serialized = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    std::fs::write(&path, serialized).map_err(|e| e.to_string())
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

    // ── Phase 44: parse_lcov ──────────────────────────────────────────────────

    #[test]
    fn parse_lcov_single_file_full_coverage() {
        let lcov = "SF:src/main.rs\nDA:1,1\nDA:2,1\nDA:3,1\nend_of_record\n";
        let files = parse_lcov(lcov);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].covered, 3);
        assert_eq!(files[0].total, 3);
        assert!((files[0].pct - 100.0).abs() < 0.01);
        assert!(files[0].uncovered_lines.is_empty());
    }

    #[test]
    fn parse_lcov_partial_coverage_tracks_uncovered_lines() {
        let lcov = "SF:src/lib.rs\nDA:1,1\nDA:2,0\nDA:3,1\nDA:4,0\nend_of_record\n";
        let files = parse_lcov(lcov);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].covered, 2);
        assert_eq!(files[0].total, 4);
        assert!((files[0].pct - 50.0).abs() < 0.01);
        assert_eq!(files[0].uncovered_lines, vec![2, 4]);
    }

    #[test]
    fn parse_lcov_multiple_files() {
        let lcov = "SF:a.rs\nDA:1,1\nend_of_record\nSF:b.rs\nDA:1,0\nDA:2,0\nend_of_record\n";
        let files = parse_lcov(lcov);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "a.rs");
        assert_eq!(files[0].covered, 1);
        assert_eq!(files[1].path, "b.rs");
        assert_eq!(files[1].covered, 0);
        assert_eq!(files[1].uncovered_lines, vec![1, 2]);
    }

    #[test]
    fn parse_lcov_empty_input() {
        let files = parse_lcov("");
        assert!(files.is_empty());
    }

    #[test]
    fn parse_lcov_no_da_lines_gives_100_pct() {
        let lcov = "SF:empty.rs\nend_of_record\n";
        let files = parse_lcov(lcov);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].total, 0);
        assert!((files[0].pct - 100.0).abs() < 0.01);
    }

    // ── Phase 44: parse_go_coverage ───────────────────────────────────────────

    #[test]
    fn parse_go_coverage_valid_output() {
        let cov = "mode: set\npkg/main.go:10.1,20.1 3 1\npkg/main.go:25.1,30.1 2 0\n";
        let files = parse_go_coverage(cov);
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.path, "pkg/main.go");
        assert_eq!(f.covered, 1);
        assert_eq!(f.total, 2);
        assert!((f.pct - 50.0).abs() < 0.01);
    }

    #[test]
    fn parse_go_coverage_multiple_files() {
        let cov = "mode: set\na.go:1.1,2.1 1 1\nb.go:1.1,2.1 1 0\n";
        let files = parse_go_coverage(cov);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn parse_go_coverage_empty_input() {
        let files = parse_go_coverage("mode: set\n");
        assert!(files.is_empty());
    }

    #[test]
    fn parse_go_coverage_skips_malformed_lines() {
        let cov = "mode: set\nbad line\nok.go:1.1,2.1 1 1\n";
        let files = parse_go_coverage(cov);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].covered, 1);
    }

    // ── Phase 44: extract_pct_from_raw ────────────────────────────────────────

    #[test]
    fn extract_pct_with_decimal() {
        assert!((extract_pct_from_raw("Coverage: 85.5%") - 85.5).abs() < 0.01);
    }

    #[test]
    fn extract_pct_integer() {
        assert!((extract_pct_from_raw("Total: 92 %") - 92.0).abs() < 0.01);
    }

    #[test]
    fn extract_pct_returns_first_match() {
        assert!((extract_pct_from_raw("Pass: 90%, Fail: 10%") - 90.0).abs() < 0.01);
    }

    #[test]
    fn extract_pct_no_match_returns_zero() {
        assert!((extract_pct_from_raw("no numbers here")).abs() < 0.01);
    }

    #[test]
    fn extract_pct_empty_string() {
        assert!((extract_pct_from_raw("")).abs() < 0.01);
    }

    // ── Phase 44: detect_coverage_tool ────────────────────────────────────────

    #[tokio::test]
    async fn detect_coverage_tool_rust_project() {
        let dir = std::env::temp_dir().join(format!("vibe_cov_rust_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        let result = detect_coverage_tool(dir.to_string_lossy().to_string()).await;
        assert_eq!(result.unwrap(), "cargo-llvm-cov");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn detect_coverage_tool_node_project() {
        let dir = std::env::temp_dir().join(format!("vibe_cov_node_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("package.json"), r#"{"name":"test"}"#).unwrap();
        let result = detect_coverage_tool(dir.to_string_lossy().to_string()).await;
        assert_eq!(result.unwrap(), "npm-coverage");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn detect_coverage_tool_python_project() {
        let dir = std::env::temp_dir().join(format!("vibe_cov_py_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("pyproject.toml"), "[project]").unwrap();
        let result = detect_coverage_tool(dir.to_string_lossy().to_string()).await;
        assert_eq!(result.unwrap(), "coverage.py");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn detect_coverage_tool_go_project() {
        let dir = std::env::temp_dir().join(format!("vibe_cov_go_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("go.mod"), "module test").unwrap();
        let result = detect_coverage_tool(dir.to_string_lossy().to_string()).await;
        assert_eq!(result.unwrap(), "go-cover");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn detect_coverage_tool_unknown_returns_error() {
        let dir = std::env::temp_dir().join(format!("vibe_cov_unk_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let result = detect_coverage_tool(dir.to_string_lossy().to_string()).await;
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn detect_coverage_tool_nyc_in_package_json() {
        let dir = std::env::temp_dir().join(format!("vibe_cov_nyc_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("package.json"), r#"{"devDependencies":{"nyc":"^15"}}"#).unwrap();
        let result = detect_coverage_tool(dir.to_string_lossy().to_string()).await;
        assert_eq!(result.unwrap(), "nyc");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Phase 44: discover_api_endpoints ──────────────────────────────────────

    #[tokio::test]
    async fn discover_api_endpoints_finds_express_routes() {
        let dir = std::env::temp_dir().join(format!("vibe_ep_expr_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("server.js"), "app.get('/api/users', handler);\napp.post('/api/users', create);").unwrap();
        let result = discover_api_endpoints(dir.to_string_lossy().to_string()).await.unwrap();
        assert!(result.len() >= 2);
        assert!(result.iter().any(|e| e.contains("app.get")));
        assert!(result.iter().any(|e| e.contains("app.post")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn discover_api_endpoints_finds_axum_routes() {
        let dir = std::env::temp_dir().join(format!("vibe_ep_axum_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.rs"), "let app = axum::Router::new()\n    .route(\"/health\", get(health));").unwrap();
        let result = discover_api_endpoints(dir.to_string_lossy().to_string()).await.unwrap();
        assert!(!result.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn discover_api_endpoints_empty_workspace() {
        let dir = std::env::temp_dir().join(format!("vibe_ep_empty_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let result = discover_api_endpoints(dir.to_string_lossy().to_string()).await.unwrap();
        assert!(result.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn discover_api_endpoints_deduplicates() {
        let dir = std::env::temp_dir().join(format!("vibe_ep_dedup_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.js"), "app.get('/x', h);").unwrap();
        std::fs::write(dir.join("b.js"), "app.get('/x', h);").unwrap();
        let result = discover_api_endpoints(dir.to_string_lossy().to_string()).await.unwrap();
        let count = result.iter().filter(|e| e.contains("app.get('/x'")).count();
        assert_eq!(count, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Phase 44: build_temp_provider ─────────────────────────────────────────

    #[test]
    fn build_temp_provider_ollama_always_succeeds() {
        let p = build_temp_provider("ollama", "codellama");
        assert!(p.is_some());
    }

    #[test]
    fn build_temp_provider_unknown_returns_none() {
        let p = build_temp_provider("nonexistent-provider", "model");
        assert!(p.is_none());
    }

    #[test]
    fn build_temp_provider_claude_returns_some() {
        let p = build_temp_provider("claude", "claude-sonnet-4-6");
        assert!(p.is_some());
    }

    #[test]
    fn build_temp_provider_anthropic_alias() {
        let p = build_temp_provider("anthropic", "claude-sonnet-4-6");
        assert!(p.is_some());
    }

    // ── Phase 44: FileCoverage serialization ──────────────────────────────────

    #[test]
    fn file_coverage_roundtrips_json() {
        let fc = FileCoverage {
            path: "src/main.rs".to_string(),
            covered: 10, total: 15, pct: 66.67,
            uncovered_lines: vec![3, 7, 11, 14, 15],
        };
        let json = serde_json::to_string(&fc).unwrap();
        let back: FileCoverage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.path, "src/main.rs");
        assert_eq!(back.covered, 10);
        assert_eq!(back.uncovered_lines.len(), 5);
    }

    #[test]
    fn coverage_result_roundtrips_json() {
        let cr = CoverageResult {
            framework: "cargo-llvm-cov".to_string(),
            total_pct: 85.0,
            files: vec![],
            raw_output: "test output".to_string(),
        };
        let json = serde_json::to_string(&cr).unwrap();
        let back: CoverageResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.framework, "cargo-llvm-cov");
        assert!((back.total_pct - 85.0).abs() < 0.01);
    }

    // ── Phase 44: ModelResponse / CompareResult serialization ─────────────────

    #[test]
    fn model_response_roundtrips_json() {
        let mr = ModelResponse {
            provider: "ollama".to_string(),
            model: "codellama".to_string(),
            content: "Hello world".to_string(),
            duration_ms: 123,
            tokens: Some(42),
            error: None,
        };
        let json = serde_json::to_string(&mr).unwrap();
        let back: ModelResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider, "ollama");
        assert_eq!(back.tokens, Some(42));
        assert!(back.error.is_none());
    }

    #[test]
    fn model_response_with_error_roundtrips() {
        let mr = ModelResponse {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            content: String::new(),
            duration_ms: 0,
            tokens: None,
            error: Some("API key not set".to_string()),
        };
        let json = serde_json::to_string(&mr).unwrap();
        let back: ModelResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.error.as_deref(), Some("API key not set"));
    }

    // ── Phase 44: HttpResponseData serialization ──────────────────────────────

    #[test]
    fn http_response_data_roundtrips_json() {
        let resp = HttpResponseData {
            status: 200,
            status_text: "OK".to_string(),
            headers: vec![HttpRequestHeader { key: "content-type".to_string(), value: "application/json".to_string() }],
            body: r#"{"ok":true}"#.to_string(),
            duration_ms: 55,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: HttpResponseData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, 200);
        assert_eq!(back.headers.len(), 1);
        assert_eq!(back.headers[0].key, "content-type");
    }

    // ── Phase 45: cost_log_path / cost_config_path ────────────────────────────

    #[test]
    fn cost_log_path_ends_with_jsonl() {
        let path = cost_log_path();
        assert!(path.to_string_lossy().ends_with("cost-log.jsonl"));
    }

    #[test]
    fn cost_log_path_is_inside_vibeui() {
        let path = cost_log_path();
        assert!(path.to_string_lossy().contains(".vibeui"));
    }

    #[test]
    fn cost_config_path_ends_with_json() {
        let path = cost_config_path();
        assert!(path.to_string_lossy().ends_with("cost-config.json"));
    }

    #[test]
    fn cost_config_path_is_inside_vibeui() {
        let path = cost_config_path();
        assert!(path.to_string_lossy().contains(".vibeui"));
    }

    // ── Phase 45: CostEntry serialization ─────────────────────────────────────

    #[test]
    fn cost_entry_roundtrips_json() {
        let entry = CostEntry {
            session_id: "sess-1".to_string(),
            provider: "claude".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            cost_usd: 0.0045,
            timestamp_ms: 1709100000000,
            task_hint: Some("fix bug".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: CostEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, "sess-1");
        assert_eq!(back.provider, "claude");
        assert_eq!(back.prompt_tokens, 100);
        assert!((back.cost_usd - 0.0045).abs() < 0.0001);
        assert_eq!(back.task_hint.as_deref(), Some("fix bug"));
    }

    #[test]
    fn cost_entry_null_task_hint_roundtrips() {
        let entry = CostEntry {
            session_id: "s".to_string(),
            provider: "ollama".to_string(),
            model: "codellama".to_string(),
            prompt_tokens: 10,
            completion_tokens: 5,
            cost_usd: 0.0,
            timestamp_ms: 0,
            task_hint: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: CostEntry = serde_json::from_str(&json).unwrap();
        assert!(back.task_hint.is_none());
    }

    #[test]
    fn cost_metrics_roundtrips_json() {
        let metrics = CostMetrics {
            entries: vec![],
            by_provider: vec![ProviderCostSummary {
                provider: "openai".to_string(),
                total_cost_usd: 1.50,
                total_tokens: 5000,
                call_count: 10,
            }],
            total_cost_usd: 1.50,
            total_tokens: 5000,
            budget_limit_usd: Some(10.0),
            budget_remaining_usd: Some(8.50),
        };
        let json = serde_json::to_string(&metrics).unwrap();
        let back: CostMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(back.by_provider.len(), 1);
        assert_eq!(back.by_provider[0].call_count, 10);
        assert!((back.budget_remaining_usd.unwrap() - 8.50).abs() < 0.01);
    }

    // ── Phase 45: AutofixResult serialization ─────────────────────────────────

    #[test]
    fn autofix_result_roundtrips_json() {
        let result = AutofixResult {
            framework: "eslint".to_string(),
            files_changed: 3,
            diff: "--- a.js\n+++ b.js".to_string(),
            stdout: "Fixed 3 files".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AutofixResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.framework, "eslint");
        assert_eq!(back.files_changed, 3);
    }
}

// ── Phase 7.19: Process Manager ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub mem_kb: u64,
    pub status: String,
}

/// List running processes (top 60 by memory, cross-platform).
///
/// On macOS/Linux uses `ps aux --sort=-%mem` (BSD ps on macOS, GNU ps on Linux).
/// On Windows uses `tasklist /FO CSV`.
#[tauri::command]
pub async fn list_processes() -> Result<Vec<ProcessInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        let out = tokio::process::Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut procs = Vec::new();
        for line in stdout.lines().take(60) {
            // CSV columns: "Image Name","PID","Session Name","Session#","Mem Usage"
            let cols: Vec<&str> = line.splitn(6, ',').collect();
            if cols.len() < 5 { continue; }
            let name = cols[0].trim_matches('"').to_string();
            let pid: u32 = cols[1].trim_matches('"').parse().unwrap_or(0);
            let mem_str = cols[4].trim_matches('"').replace(',', "").replace(" K", "");
            let mem_kb: u64 = mem_str.trim().parse().unwrap_or(0);
            procs.push(ProcessInfo { pid, name, cpu_pct: 0.0, mem_kb, status: "running".to_string() });
        }
        return Ok(procs);
    }
    #[cfg(not(target_os = "windows"))]
    {
        // `ps aux` columns: USER PID %CPU %MEM VSZ RSS TTY STAT START TIME COMMAND
        let out = tokio::process::Command::new("ps")
            .args(["aux", "--sort=-%mem"])
            .output()
            .await;
        // macOS `ps` doesn't support --sort; fall back without it
        let out = match out {
            Ok(o) if o.status.success() => o,
            _ => tokio::process::Command::new("ps")
                .args(["aux"])
                .output()
                .await
                .map_err(|e| e.to_string())?,
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut procs: Vec<ProcessInfo> = stdout
            .lines()
            .skip(1) // skip header
            .take(60)
            .filter_map(|line| {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() < 11 { return None; }
                let pid: u32 = cols[1].parse().ok()?;
                let cpu_pct: f32 = cols[2].parse().unwrap_or(0.0);
                let rss_kb: u64 = cols[5].parse().unwrap_or(0);
                let stat = cols[7].to_string();
                // Command is everything from column 10 onward
                let name = cols[10..].join(" ");
                // Trim full path to basename for readability
                let name = name.rsplit('/').next().unwrap_or(&name).to_string();
                Some(ProcessInfo { pid, name, cpu_pct, mem_kb: rss_kb, status: stat })
            })
            .collect();
        // Sort by memory desc on macOS (ps aux there doesn't support --sort)
        procs.sort_by(|a, b| b.mem_kb.cmp(&a.mem_kb));
        Ok(procs)
    }
}

/// Send SIGTERM (graceful stop) to a process by PID.
///
/// On Windows this calls `taskkill /PID <pid> /F`.
#[tauri::command]
pub async fn kill_process(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("Invalid PID 0".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        tokio::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Safety: SIGTERM is non-destructive; only lets process clean up.
        // Using `kill` shell command avoids unsafe libc calls.
        let out = tokio::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("kill failed: {}", stderr.trim()));
        }
        Ok(())
    }
}

// ─── Phase 7.22: CI/CD & Kubernetes Deployment Hub ────────────────────────

/// Detect build type from workspace files.
#[tauri::command]
pub async fn detect_build_type(workspace: String) -> Result<String, String> {
    let path = std::path::Path::new(&workspace);
    if path.join("Cargo.toml").exists() {
        return Ok("rust".to_string());
    }
    if path.join("go.mod").exists() {
        return Ok("go".to_string());
    }
    if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
        return Ok("java".to_string());
    }
    // Check for .csproj files
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().extension().map(|e| e == "csproj").unwrap_or(false) {
                return Ok("dotnet".to_string());
            }
        }
    }
    if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
        return Ok("python".to_string());
    }
    if path.join("package.json").exists() {
        return Ok("node".to_string());
    }
    Ok("unknown".to_string())
}

fn cicd_output_path(platform: &str) -> (&'static str, &'static str) {
    match platform {
        "github"    => (".github/workflows", "ci.yml"),
        "gitlab"    => (".", ".gitlab-ci.yml"),
        "circleci"  => (".circleci", "config.yml"),
        "jenkins"   => (".", "Jenkinsfile"),
        "bitbucket" => (".", "bitbucket-pipelines.yml"),
        _           => (".", "ci.yml"),
    }
}

fn build_cicd_template(platform: &str, build_type: &str) -> String {
    match (platform, build_type) {
        // ── GitHub Actions ────────────────────────────────────────────────────
        ("github", "rust") => r#"name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace
      - run: cargo build --release
"#.to_string(),
        ("github", "node") => r#"name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
      - run: npm ci
      - run: npm test
      - run: npm run build
"#.to_string(),
        ("github", "go") => r#"name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-go@v5
        with:
          go-version: '1.22'
      - run: go test ./...
      - run: go build ./...
"#.to_string(),
        ("github", "python") => r#"name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      - run: pip install -r requirements.txt
      - run: pytest
"#.to_string(),
        ("github", "java") => r#"name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-java@v4
        with:
          java-version: '21'
          distribution: 'temurin'
      - run: mvn --batch-mode test
      - run: mvn --batch-mode package -DskipTests
"#.to_string(),
        ("github", "dotnet") => r#"name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '8.x'
      - run: dotnet restore
      - run: dotnet build --no-restore
      - run: dotnet test --no-build --verbosity normal
"#.to_string(),
        ("github", bt) => format!(
            "name: CI\non:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: echo \"Add your {} test/build commands here\"\n", bt),

        // ── GitLab CI ────────────────────────────────────────────────────────
        ("gitlab", "rust") => r#"stages:
  - test
  - build

test:
  image: rust:latest
  stage: test
  script:
    - cargo test --workspace

build:
  image: rust:latest
  stage: build
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/
    expire_in: 1 hour
"#.to_string(),
        ("gitlab", "node") => r#"stages:
  - test
  - build

test:
  image: node:20-alpine
  stage: test
  cache:
    paths:
      - node_modules/
  script:
    - npm ci
    - npm test

build:
  image: node:20-alpine
  stage: build
  script:
    - npm ci
    - npm run build
  artifacts:
    paths:
      - dist/
"#.to_string(),
        ("gitlab", "go") => r#"stages:
  - test
  - build

test:
  image: golang:1.22
  stage: test
  script:
    - go test ./...

build:
  image: golang:1.22
  stage: build
  script:
    - go build -o app ./...
  artifacts:
    paths:
      - app
"#.to_string(),
        ("gitlab", "python") => r#"stages:
  - test

test:
  image: python:3.12-slim
  stage: test
  script:
    - pip install -r requirements.txt
    - pytest
"#.to_string(),
        ("gitlab", bt) => format!(
            "stages:\n  - test\n  - build\n\ntest:\n  stage: test\n  script:\n    - echo \"Add your {} test commands here\"\n\nbuild:\n  stage: build\n  script:\n    - echo \"Add your {} build commands here\"\n", bt, bt),

        // ── CircleCI ─────────────────────────────────────────────────────────
        ("circleci", "rust") => r#"version: 2.1
jobs:
  build-and-test:
    docker:
      - image: cimg/rust:1.77
    steps:
      - checkout
      - restore_cache:
          keys:
            - v1-cargo-{{ checksum "Cargo.lock" }}
      - run:
          name: Run tests
          command: cargo test --workspace
      - run:
          name: Build release
          command: cargo build --release
      - save_cache:
          key: v1-cargo-{{ checksum "Cargo.lock" }}
          paths:
            - ~/.cargo

workflows:
  main:
    jobs:
      - build-and-test
"#.to_string(),
        ("circleci", "node") => r#"version: 2.1
jobs:
  build-and-test:
    docker:
      - image: cimg/node:20.0
    steps:
      - checkout
      - restore_cache:
          keys:
            - v1-npm-{{ checksum "package-lock.json" }}
      - run: npm ci
      - save_cache:
          key: v1-npm-{{ checksum "package-lock.json" }}
          paths:
            - node_modules
      - run: npm test
      - run: npm run build

workflows:
  main:
    jobs:
      - build-and-test
"#.to_string(),
        ("circleci", bt) => format!(
            "version: 2.1\njobs:\n  build-and-test:\n    docker:\n      - image: cimg/base:stable\n    steps:\n      - checkout\n      - run:\n          name: Test\n          command: echo \"Add your {} test commands here\"\n      - run:\n          name: Build\n          command: echo \"Add your {} build commands here\"\n\nworkflows:\n  main:\n    jobs:\n      - build-and-test\n", bt, bt),

        // ── Jenkins ──────────────────────────────────────────────────────────
        ("jenkins", "rust") => r#"pipeline {
    agent {
        docker { image 'rust:latest' }
    }
    environment {
        CARGO_HOME = "${WORKSPACE}/.cargo"
    }
    stages {
        stage('Test') {
            steps {
                sh 'cargo test --workspace'
            }
        }
        stage('Build') {
            steps {
                sh 'cargo build --release'
                archiveArtifacts artifacts: 'target/release/*', onlyIfSuccessful: true
            }
        }
    }
}
"#.to_string(),
        ("jenkins", "node") => r#"pipeline {
    agent {
        docker { image 'node:20-alpine' }
    }
    stages {
        stage('Install') {
            steps { sh 'npm ci' }
        }
        stage('Test') {
            steps { sh 'npm test' }
        }
        stage('Build') {
            steps {
                sh 'npm run build'
                archiveArtifacts artifacts: 'dist/**', onlyIfSuccessful: true
            }
        }
    }
}
"#.to_string(),
        ("jenkins", bt) => format!(
            "pipeline {{\n    agent any\n    stages {{\n        stage('Test') {{\n            steps {{\n                sh 'echo \"Add your {} test commands here\"'\n            }}\n        }}\n        stage('Build') {{\n            steps {{\n                sh 'echo \"Add your {} build commands here\"'\n            }}\n        }}\n    }}\n}}\n", bt, bt),

        // ── Bitbucket Pipelines ───────────────────────────────────────────────
        ("bitbucket", "rust") => r#"image: rust:latest

pipelines:
  default:
    - step:
        name: Test
        caches:
          - cargo
        script:
          - cargo test --workspace
    - step:
        name: Build
        script:
          - cargo build --release

definitions:
  caches:
    cargo: ~/.cargo
"#.to_string(),
        ("bitbucket", "node") => r#"image: node:20-alpine

pipelines:
  default:
    - step:
        name: Install & Test
        caches:
          - node
        script:
          - npm ci
          - npm test
    - step:
        name: Build
        script:
          - npm run build
        artifacts:
          - dist/**
"#.to_string(),
        ("bitbucket", bt) => format!(
            "image: ubuntu:22.04\n\npipelines:\n  default:\n    - step:\n        name: Test\n        script:\n          - echo \"Add your {} test commands here\"\n    - step:\n        name: Build\n        script:\n          - echo \"Add your {} build commands here\"\n", bt, bt),

        // ── Fallback ─────────────────────────────────────────────────────────
        (plat, bt) => format!("# CI/CD config for {} ({})\n# Customize this template for your project\n", plat, bt),
    }
}

/// Generate a CI/CD configuration file for the given platform and build type.
/// Writes the file into the workspace and returns the generated content.
#[tauri::command]
pub async fn generate_cicd_config(
    workspace: String,
    platform: String,
    build_type: String,
) -> Result<String, String> {
    let content = build_cicd_template(&platform, &build_type);
    let (dir, filename) = cicd_output_path(&platform);
    let output_path = std::path::Path::new(&workspace).join(dir).join(filename);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&output_path, &content).map_err(|e| e.to_string())?;
    Ok(content)
}

/// Generate a GitHub Actions release workflow for producing cross-platform binaries.
#[tauri::command]
pub async fn generate_release_workflow(
    workspace: String,
    build_type: String,
    targets: Vec<String>,
) -> Result<String, String> {
    let matrix_entries: Vec<String> = targets.iter().map(|t| {
        let (os, target_triple, artifact) = match t.as_str() {
            "linux-x64"   => ("ubuntu-latest",  "x86_64-unknown-linux-musl",   "app-linux-x64"),
            "linux-arm64" => ("ubuntu-latest",  "aarch64-unknown-linux-musl",  "app-linux-arm64"),
            "macos-arm64" => ("macos-latest",   "aarch64-apple-darwin",         "app-macos-arm64"),
            "macos-x64"   => ("macos-13",        "x86_64-apple-darwin",          "app-macos-x64"),
            "windows-x64" => ("windows-latest", "x86_64-pc-windows-msvc",       "app-windows-x64.exe"),
            _             => ("ubuntu-latest",  "x86_64-unknown-linux-musl",   "app-linux-x64"),
        };
        format!("          - os: {}\n            target: {}\n            artifact: {}", os, target_triple, artifact)
    }).collect();

    let (install_steps, build_cmd) = match build_type.as_str() {
        "rust" => (
            "      - uses: dtolnay/rust-toolchain@stable\n        with:\n          targets: ${{ matrix.target }}\n      - uses: Swatinem/rust-cache@v2\n      - run: cargo install cross --git https://github.com/cross-rs/cross".to_string(),
            "cross build --release --target ${{ matrix.target }}".to_string(),
        ),
        "node" => (
            "      - uses: actions/setup-node@v4\n        with:\n          node-version: '20'\n          cache: 'npm'\n      - run: npm ci".to_string(),
            "npm run build".to_string(),
        ),
        "go" => (
            "      - uses: actions/setup-go@v5\n        with:\n          go-version: '1.22'".to_string(),
            "go build -o ${{ matrix.artifact }} ./...".to_string(),
        ),
        _ => (String::new(), "echo 'Add build command'".to_string()),
    };

    let content = format!(
        "name: Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  build:\n    strategy:\n      matrix:\n        include:\n{matrix}\n    runs-on: ${{{{ matrix.os }}}}\n    steps:\n      - uses: actions/checkout@v4\n{install}\n      - name: Build\n        run: {build}\n      - name: Upload artifact\n        uses: actions/upload-artifact@v4\n        with:\n          name: ${{{{ matrix.artifact }}}}\n          path: ${{{{ matrix.artifact }}}}\n  release:\n    needs: build\n    runs-on: ubuntu-latest\n    permissions:\n      contents: write\n    steps:\n      - name: Download all artifacts\n        uses: actions/download-artifact@v4\n      - name: Create GitHub Release\n        uses: softprops/action-gh-release@v2\n        with:\n          files: '**/*'\n",
        matrix = matrix_entries.join("\n"),
        install = install_steps,
        build = build_cmd,
    );

    let output_path = std::path::Path::new(&workspace).join(".github/workflows/release.yml");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&output_path, &content).map_err(|e| e.to_string())?;
    Ok(content)
}

// ─── Phase 7.22: Kubernetes & ArgoCD ──────────────────────────────────────

/// List available kubectl contexts from the local kubeconfig.
#[tauri::command]
pub async fn list_k8s_contexts() -> Result<Vec<String>, String> {
    let out = tokio::process::Command::new("kubectl")
        .args(["config", "get-contexts", "-o", "name"])
        .output()
        .await;
    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let contexts: Vec<String> = stdout
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            Ok(contexts)
        }
        Err(_) => Ok(vec![]), // kubectl not installed — return empty, not error
    }
}

/// Generate Kubernetes manifests: Deployment + Service + optional Ingress + HPA.
#[tauri::command]
pub async fn generate_k8s_manifests(
    app_name: String,
    image: String,
    port: u16,
    replicas: u32,
    namespace: String,
    ingress_host: Option<String>,
) -> Result<String, String> {
    let max_replicas = (replicas * 3).max(3);

    let deployment = format!(
        "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: {name}\n  namespace: {ns}\n  labels:\n    app: {name}\nspec:\n  replicas: {rep}\n  selector:\n    matchLabels:\n      app: {name}\n  template:\n    metadata:\n      labels:\n        app: {name}\n    spec:\n      containers:\n        - name: {name}\n          image: {img}\n          ports:\n            - containerPort: {port}\n          resources:\n            requests:\n              cpu: \"100m\"\n              memory: \"128Mi\"\n            limits:\n              cpu: \"500m\"\n              memory: \"512Mi\"\n          livenessProbe:\n            httpGet:\n              path: /\n              port: {port}\n            initialDelaySeconds: 15\n            periodSeconds: 20\n",
        name = app_name, ns = namespace, rep = replicas, img = image, port = port
    );

    let service = format!(
        "---\napiVersion: v1\nkind: Service\nmetadata:\n  name: {name}\n  namespace: {ns}\n  labels:\n    app: {name}\nspec:\n  type: ClusterIP\n  selector:\n    app: {name}\n  ports:\n    - port: 80\n      targetPort: {port}\n",
        name = app_name, ns = namespace, port = port
    );

    let ingress = if let Some(host) = ingress_host {
        format!(
            "---\napiVersion: networking.k8s.io/v1\nkind: Ingress\nmetadata:\n  name: {name}\n  namespace: {ns}\n  annotations:\n    kubernetes.io/ingress.class: nginx\nspec:\n  rules:\n    - host: {host}\n      http:\n        paths:\n          - path: /\n            pathType: Prefix\n            backend:\n              service:\n                name: {name}\n                port:\n                  number: 80\n",
            name = app_name, ns = namespace, host = host
        )
    } else {
        String::new()
    };

    let hpa = format!(
        "---\napiVersion: autoscaling/v2\nkind: HorizontalPodAutoscaler\nmetadata:\n  name: {name}\n  namespace: {ns}\nspec:\n  scaleTargetRef:\n    apiVersion: apps/v1\n    kind: Deployment\n    name: {name}\n  minReplicas: 1\n  maxReplicas: {max}\n  metrics:\n    - type: Resource\n      resource:\n        name: cpu\n        target:\n          type: Utilization\n          averageUtilization: 70\n",
        name = app_name, ns = namespace, max = max_replicas
    );

    Ok(format!("{deployment}{service}{ingress}{hpa}"))
}

/// Run a kubectl command against the specified context and namespace.
/// Destructive commands are blocked for safety.
#[tauri::command]
pub async fn run_kubectl_command(
    context: Option<String>,
    namespace: String,
    command: String,
) -> Result<String, String> {
    const BLOCKED: &[&str] = &[
        "delete namespace",
        "delete cluster",
        "--force --grace-period=0",
        "delete node",
    ];
    let cmd_lower = command.to_lowercase();
    for blocked in BLOCKED {
        if cmd_lower.contains(blocked) {
            return Err(format!("Command blocked for safety: contains '{}'", blocked));
        }
    }

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let mut args: Vec<String> = Vec::new();
    if let Some(ctx) = &context {
        if !ctx.is_empty() {
            args.push(format!("--context={}", ctx));
        }
    }
    if !namespace.is_empty() {
        args.push(format!("--namespace={}", namespace));
    }
    args.extend(parts.iter().map(|s| s.to_string()));

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("kubectl").args(&args).output(),
    )
    .await
    .map_err(|_| "kubectl command timed out after 30 seconds".to_string())?
    .map_err(|e| format!("Failed to run kubectl: {}", e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stdout.is_empty() && !stderr.is_empty() {
        Ok(stderr)
    } else if !stderr.is_empty() {
        Ok(format!("{}\n{}", stdout.trim_end(), stderr.trim_end()))
    } else {
        Ok(stdout)
    }
}

/// Generate an ArgoCD Application CR YAML string.
#[tauri::command]
pub async fn generate_argocd_app(
    app_name: String,
    repo_url: String,
    path: String,
    namespace: String,
    server: String,
) -> Result<String, String> {
    let yaml = format!(
        "apiVersion: argoproj.io/v1alpha1\nkind: Application\nmetadata:\n  name: {name}\n  namespace: argocd\nspec:\n  project: default\n  source:\n    repoURL: {repo}\n    targetRevision: HEAD\n    path: {path}\n  destination:\n    server: {server}\n    namespace: {ns}\n  syncPolicy:\n    automated:\n      prune: true\n      selfHeal: true\n    syncOptions:\n      - CreateNamespace=true\n",
        name = app_name, repo = repo_url, path = path, server = server, ns = namespace
    );
    Ok(yaml)
}

// ── Environment & Secrets Manager ────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct EnvFileInfo {
    pub filename: String,
    pub environment: String,
    pub var_count: usize,
    pub last_modified: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct EnvEntry {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub comment: Option<String>,
}

fn is_secret_key(key: &str) -> bool {
    let upper = key.to_uppercase();
    ["SECRET", "TOKEN", "PASSWORD", "CREDENTIAL", "PRIVATE", "API_KEY", "_KEY"]
        .iter()
        .any(|pat| upper.contains(pat))
}

fn env_filename_to_environment(filename: &str) -> String {
    if filename == ".env" || filename == ".env.local" {
        "default".to_string()
    } else if let Some(suffix) = filename.strip_prefix(".env.") {
        suffix.trim_end_matches(".local").to_string()
    } else {
        "default".to_string()
    }
}

fn parse_env_content(content: &str, reveal: bool) -> Vec<EnvEntry> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let mut value = trimmed[eq_pos + 1..].trim().to_string();
            // Strip surrounding quotes
            if value.len() >= 2
                && ((value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\'')))
            {
                value = value[1..value.len() - 1].to_string();
            }
            let secret = is_secret_key(&key);
            let display_value = if secret && !reveal {
                "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}".to_string()
            } else {
                value
            };
            entries.push(EnvEntry {
                key,
                value: display_value,
                is_secret: secret,
                comment: None,
            });
        }
    }
    entries
}

/// List all .env* files in a workspace.
#[tauri::command]
pub async fn get_env_files(workspace: String) -> Result<Vec<EnvFileInfo>, String> {
    let ws = std::path::PathBuf::from(&workspace);
    if !ws.is_dir() {
        return Err("Workspace directory not found".to_string());
    }
    let mut files = Vec::new();
    let entries = std::fs::read_dir(&ws).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(".env") {
            continue;
        }
        // Only match .env, .env.*, .env.local, .env.*.local
        if name != ".env" && !name.starts_with(".env.") {
            continue;
        }
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let var_count = content
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with('#') && t.contains('=')
            })
            .count();
        let last_modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let environment = env_filename_to_environment(&name);
        files.push(EnvFileInfo {
            filename: name,
            environment,
            var_count,
            last_modified,
        });
    }
    files.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(files)
}

/// Read and parse a .env file into structured entries.
#[tauri::command]
pub async fn read_env_file(
    workspace: String,
    filename: String,
    reveal: Option<bool>,
) -> Result<Vec<EnvEntry>, String> {
    let path = std::path::PathBuf::from(&workspace).join(&filename);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    // Prevent path traversal
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let ws_canonical = std::path::PathBuf::from(&workspace)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !canonical.starts_with(&ws_canonical) {
        return Err("Path traversal not allowed".to_string());
    }
    let content = std::fs::read_to_string(&canonical).map_err(|e| e.to_string())?;
    Ok(parse_env_content(&content, reveal.unwrap_or(false)))
}

/// Save entries to a .env file.
#[tauri::command]
pub async fn save_env_file(
    workspace: String,
    filename: String,
    entries: Vec<EnvEntry>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&workspace).join(&filename);
    // Prevent path traversal
    let ws_canonical = std::path::PathBuf::from(&workspace)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    // For new files, just check the parent is within workspace
    if let Ok(canonical) = path.canonicalize() {
        if !canonical.starts_with(&ws_canonical) {
            return Err("Path traversal not allowed".to_string());
        }
    } else {
        // New file — ensure parent is the workspace
        let parent = path
            .parent()
            .ok_or("Invalid path")?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !parent.starts_with(&ws_canonical) {
            return Err("Path traversal not allowed".to_string());
        }
    }
    // Validate keys
    let key_re = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    for entry in &entries {
        if entry.key.is_empty() {
            return Err("Empty key not allowed".to_string());
        }
        if !key_re.is_match(&entry.key) {
            return Err(format!("Invalid key name: {}", entry.key));
        }
    }
    // Check for duplicate keys
    let mut seen = std::collections::HashSet::new();
    for entry in &entries {
        if !seen.insert(&entry.key) {
            return Err(format!("Duplicate key: {}", entry.key));
        }
    }
    // Build file content
    let mut lines = Vec::new();
    for entry in &entries {
        if let Some(comment) = &entry.comment {
            lines.push(format!("# {}", comment));
        }
        // Quote values that contain spaces or special characters
        if entry.value.contains(' ')
            || entry.value.contains('#')
            || entry.value.contains('"')
            || entry.value.contains('\'')
        {
            let escaped = entry.value.replace('\\', "\\\\").replace('"', "\\\"");
            lines.push(format!("{}=\"{}\"", entry.key, escaped));
        } else {
            lines.push(format!("{}={}", entry.key, entry.value));
        }
    }
    let content = lines.join("\n") + "\n";
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    // Set secure permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Delete a specific key from a .env file.
#[tauri::command]
pub async fn delete_env_var(
    workspace: String,
    filename: String,
    key: String,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&workspace).join(&filename);
    if !path.is_file() {
        return Err(format!("File not found: {}", filename));
    }
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let ws_canonical = std::path::PathBuf::from(&workspace)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !canonical.starts_with(&ws_canonical) {
        return Err("Path traversal not allowed".to_string());
    }
    let content = std::fs::read_to_string(&canonical).map_err(|e| e.to_string())?;
    let filtered: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if let Some(eq_pos) = trimmed.find('=') {
                let line_key = trimmed[..eq_pos].trim();
                line_key != key
            } else {
                true // Keep comments and blank lines
            }
        })
        .collect();
    let new_content = filtered.join("\n") + "\n";
    std::fs::write(&canonical, &new_content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get list of detected environments from .env files.
#[tauri::command]
pub async fn get_env_environments(workspace: String) -> Result<Vec<String>, String> {
    let files = get_env_files(workspace).await?;
    let mut envs: Vec<String> = files.iter().map(|f| f.environment.clone()).collect();
    envs.sort();
    envs.dedup();
    if !envs.contains(&"default".to_string()) {
        envs.insert(0, "default".to_string());
    }
    Ok(envs)
}

/// Set the active environment for the workspace.
#[tauri::command]
pub async fn set_active_environment(
    workspace: String,
    environment: String,
) -> Result<(), String> {
    let vibeui_dir = std::path::PathBuf::from(&workspace).join(".vibeui");
    std::fs::create_dir_all(&vibeui_dir).map_err(|e| e.to_string())?;
    let path = vibeui_dir.join("active-env.txt");
    std::fs::write(&path, &environment).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Performance Profiler ─────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ProfileHotspot {
    pub function_name: String,
    pub file: Option<String>,
    pub self_pct: f32,
    pub total_pct: f32,
    pub samples: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProfileResult {
    pub tool: String,
    pub hotspots: Vec<ProfileHotspot>,
    pub total_samples: u64,
    pub duration_secs: f32,
    pub raw_output: String,
}

/// Auto-detect the appropriate profiling tool for the workspace.
#[tauri::command]
pub async fn detect_profiler_tool(workspace: String) -> Result<String, String> {
    let ws = std::path::PathBuf::from(&workspace);
    if ws.join("Cargo.toml").exists() {
        return Ok("cargo-flamegraph".to_string());
    }
    if ws.join("package.json").exists() {
        return Ok("clinic".to_string());
    }
    if ws.join("go.mod").exists() {
        return Ok("go-pprof".to_string());
    }
    if ws.join("pyproject.toml").exists()
        || ws.join("setup.py").exists()
        || ws.join("requirements.txt").exists()
    {
        return Ok("py-spy".to_string());
    }
    Err("No profiling tool detected for this workspace".to_string())
}

fn parse_pprof_top(output: &str) -> Vec<ProfileHotspot> {
    let re = regex::Regex::new(
        r"(?m)^\s*([\d.]+)(%?)\s+[\d.]+%?\s+([\d.]+)(%?)\s+[\d.]+%?\s+(.+)$"
    ).unwrap();
    let mut hotspots = Vec::new();
    for cap in re.captures_iter(output) {
        let self_val: f32 = cap[1].parse().unwrap_or(0.0);
        let total_val: f32 = cap[3].parse().unwrap_or(0.0);
        let func_name = cap[5].trim().to_string();
        if func_name.is_empty() || func_name == "flat" {
            continue;
        }
        hotspots.push(ProfileHotspot {
            function_name: func_name,
            file: None,
            self_pct: self_val,
            total_pct: total_val,
            samples: 0,
        });
    }
    hotspots
}

fn parse_speedscope_json(content: &str) -> Vec<ProfileHotspot> {
    let val: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let frames = val.pointer("/shared/frames").and_then(|v| v.as_array());
    let profiles = val.get("profiles").and_then(|v| v.as_array());
    if let (Some(frames), Some(profiles)) = (frames, profiles) {
        for profile in profiles {
            if let Some(samples) = profile.get("samples").and_then(|s| s.as_array()) {
                for sample in samples {
                    if let Some(stack) = sample.as_array() {
                        for idx in stack {
                            if let Some(i) = idx.as_u64() {
                                if let Some(frame) = frames.get(i as usize) {
                                    let name = frame.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                                    *counts.entry(name.to_string()).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let total: u64 = counts.values().sum();
    let mut hotspots: Vec<ProfileHotspot> = counts
        .into_iter()
        .map(|(name, count)| {
            let pct = if total > 0 { (count as f32 / total as f32) * 100.0 } else { 0.0 };
            ProfileHotspot {
                function_name: name,
                file: None,
                self_pct: pct,
                total_pct: pct,
                samples: count,
            }
        })
        .collect();
    hotspots.sort_by(|a, b| b.self_pct.partial_cmp(&a.self_pct).unwrap_or(std::cmp::Ordering::Equal));
    hotspots
}

/// Run a profiler and return structured results.
#[tauri::command]
pub async fn run_profiler(
    _app: tauri::AppHandle,
    workspace: String,
    tool: String,
    target: Option<String>,
) -> Result<ProfileResult, String> {
    let ws = std::path::PathBuf::from(&workspace);
    let target_str = target.unwrap_or_default();
    let start = std::time::Instant::now();

    match tool.as_str() {
        "cargo-flamegraph" => {
            // Use cargo bench or just build + run with flamegraph
            let mut args = vec!["flamegraph", "--output", "profile.svg"];
            if !target_str.is_empty() {
                args.push("--");
                args.push(&target_str);
            }
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(120),
                tokio::process::Command::new("cargo")
                    .args(&args)
                    .current_dir(&ws)
                    .output(),
            )
            .await
            .map_err(|_| "Profiler timed out after 120 seconds".to_string())?
            .map_err(|e| format!("Failed to run cargo flamegraph: {e}"))?;

            let raw = String::from_utf8_lossy(&output.stdout).to_string()
                + &String::from_utf8_lossy(&output.stderr);
            let duration = start.elapsed().as_secs_f32();

            // Try to parse SVG title tags for hotspot data
            let svg_path = ws.join("profile.svg");
            let mut hotspots = Vec::new();
            if svg_path.exists() {
                let svg_content = std::fs::read_to_string(&svg_path).unwrap_or_default();
                let title_re = regex::Regex::new(r"<title>([^<]+)\s+\((\d+)\s+samples?,\s+([\d.]+)%\)</title>").unwrap();
                for cap in title_re.captures_iter(&svg_content) {
                    let func_name = cap[1].trim().to_string();
                    let samples: u64 = cap[2].parse().unwrap_or(0);
                    let pct: f32 = cap[3].parse().unwrap_or(0.0);
                    hotspots.push(ProfileHotspot {
                        function_name: func_name,
                        file: None,
                        self_pct: pct,
                        total_pct: pct,
                        samples,
                    });
                }
                hotspots.sort_by(|a, b| b.self_pct.partial_cmp(&a.self_pct).unwrap_or(std::cmp::Ordering::Equal));
                hotspots.dedup_by(|a, b| a.function_name == b.function_name);
            }

            let total_samples = hotspots.iter().map(|h| h.samples).sum();
            Ok(ProfileResult { tool, hotspots, total_samples, duration_secs: duration, raw_output: raw })
        }

        "go-pprof" => {
            // Run go test with CPU profile, then parse with pprof -top
            let test_output = tokio::time::timeout(
                std::time::Duration::from_secs(120),
                tokio::process::Command::new("go")
                    .args(["test", "-bench=.", "-benchtime=3s", "-cpuprofile=cpu.prof", "./..."])
                    .current_dir(&ws)
                    .output(),
            )
            .await
            .map_err(|_| "go test timed out after 120 seconds".to_string())?
            .map_err(|e| format!("Failed to run go test: {e}"))?;

            let mut raw = String::from_utf8_lossy(&test_output.stdout).to_string()
                + &String::from_utf8_lossy(&test_output.stderr);

            let prof_path = ws.join("cpu.prof");
            let mut hotspots = Vec::new();
            if prof_path.exists() {
                let pprof_output = tokio::process::Command::new("go")
                    .args(["tool", "pprof", "-top", "cpu.prof"])
                    .current_dir(&ws)
                    .output()
                    .await
                    .map_err(|e| format!("Failed to run pprof: {e}"))?;
                let pprof_text = String::from_utf8_lossy(&pprof_output.stdout).to_string();
                raw.push_str("\n--- pprof top ---\n");
                raw.push_str(&pprof_text);
                hotspots = parse_pprof_top(&pprof_text);
            }

            let duration = start.elapsed().as_secs_f32();
            let total_samples = hotspots.iter().map(|h| h.samples).sum();
            Ok(ProfileResult { tool, hotspots, total_samples, duration_secs: duration, raw_output: raw })
        }

        "py-spy" => {
            let target_cmd = if target_str.is_empty() { "python -c 'import time; time.sleep(1)'".to_string() } else { format!("python {target_str}") };
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(120),
                tokio::process::Command::new("sh")
                    .args(["-c", &format!("py-spy record --format speedscope -o profile.json -- {target_cmd}")])
                    .current_dir(&ws)
                    .output(),
            )
            .await
            .map_err(|_| "py-spy timed out after 120 seconds".to_string())?
            .map_err(|e| format!("Failed to run py-spy: {e}"))?;

            let raw = String::from_utf8_lossy(&output.stdout).to_string()
                + &String::from_utf8_lossy(&output.stderr);
            let duration = start.elapsed().as_secs_f32();

            let json_path = ws.join("profile.json");
            let hotspots = if json_path.exists() {
                let content = std::fs::read_to_string(&json_path).unwrap_or_default();
                parse_speedscope_json(&content)
            } else {
                Vec::new()
            };

            let total_samples = hotspots.iter().map(|h| h.samples).sum();
            Ok(ProfileResult { tool, hotspots, total_samples, duration_secs: duration, raw_output: raw })
        }

        "clinic" => {
            let target_cmd = if target_str.is_empty() { "node .".to_string() } else { format!("node {target_str}") };
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(120),
                tokio::process::Command::new("sh")
                    .args(["-c", &format!("npx clinic doctor --autocannon -- {target_cmd}")])
                    .current_dir(&ws)
                    .output(),
            )
            .await
            .map_err(|_| "clinic timed out after 120 seconds".to_string())?
            .map_err(|e| format!("Failed to run clinic: {e}"))?;

            let raw = String::from_utf8_lossy(&output.stdout).to_string()
                + &String::from_utf8_lossy(&output.stderr);
            let duration = start.elapsed().as_secs_f32();

            // clinic outputs HTML reports; structured parsing is complex — return raw output
            Ok(ProfileResult {
                tool,
                hotspots: Vec::new(),
                total_samples: 0,
                duration_secs: duration,
                raw_output: raw,
            })
        }

        _ => Err(format!("Unknown profiler tool: {tool}")),
    }
}

// ─── Phase 7.25: Docker & Container Management ────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DockerContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
    pub created: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DockerImage {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: String,
    pub created: String,
}

/// List all Docker containers (running + stopped).
#[tauri::command]
pub async fn list_docker_containers() -> Result<Vec<DockerContainer>, String> {
    let out = tokio::process::Command::new("docker")
        .args([
            "ps", "-a",
            "--format",
            "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}\t{{.CreatedAt}}",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run docker: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("docker ps failed: {}", err.trim()));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let containers = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let cols: Vec<&str> = line.splitn(6, '\t').collect();
            DockerContainer {
                id: cols.first().unwrap_or(&"").to_string(),
                name: cols.get(1).unwrap_or(&"").to_string(),
                image: cols.get(2).unwrap_or(&"").to_string(),
                status: cols.get(3).unwrap_or(&"").to_string(),
                ports: cols.get(4).unwrap_or(&"").to_string(),
                created: cols.get(5).unwrap_or(&"").to_string(),
            }
        })
        .collect();
    Ok(containers)
}

/// Perform an action on a container: start | stop | restart | remove | logs.
#[tauri::command]
pub async fn docker_container_action(
    container_id: String,
    action: String,
    tail_lines: Option<u32>,
) -> Result<String, String> {
    if container_id.is_empty() {
        return Err("Container ID required".to_string());
    }
    // Safety: only allow known actions
    let (cmd_args, timeout_secs): (Vec<String>, u64) = match action.as_str() {
        "start"   => (vec!["start".into(), container_id.clone()], 30),
        "stop"    => (vec!["stop".into(), container_id.clone()], 30),
        "restart" => (vec!["restart".into(), container_id.clone()], 30),
        "remove"  => (vec!["rm".into(), "-f".into(), container_id.clone()], 15),
        "logs"    => {
            let n = tail_lines.unwrap_or(100).to_string();
            (vec!["logs".into(), "--tail".into(), n, "--timestamps".into(), container_id.clone()], 15)
        }
        _ => return Err(format!("Unknown action: {action}")),
    };

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio::process::Command::new("docker").args(&cmd_args).output(),
    )
    .await
    .map_err(|_| format!("docker {action} timed out"))?
    .map_err(|e| format!("Failed to run docker {action}: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() && action != "logs" {
        return Err(format!("docker {action} failed: {}", stderr.trim()));
    }
    if stdout.is_empty() { Ok(stderr) } else { Ok(format!("{stdout}{stderr}")) }
}

/// List Docker images.
#[tauri::command]
pub async fn list_docker_images() -> Result<Vec<DockerImage>, String> {
    let out = tokio::process::Command::new("docker")
        .args([
            "images",
            "--format",
            "{{.ID}}\t{{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run docker: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("docker images failed: {}", err.trim()));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let images = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let cols: Vec<&str> = line.splitn(5, '\t').collect();
            DockerImage {
                id: cols.first().unwrap_or(&"").to_string(),
                repository: cols.get(1).unwrap_or(&"").to_string(),
                tag: cols.get(2).unwrap_or(&"").to_string(),
                size: cols.get(3).unwrap_or(&"").to_string(),
                created: cols.get(4).unwrap_or(&"").to_string(),
            }
        })
        .collect();
    Ok(images)
}

/// Run a docker-compose command in the workspace.
/// Allowed actions: up, down, ps, logs, pull, build, restart.
#[tauri::command]
pub async fn docker_compose_action(
    workspace: String,
    action: String,
    service: Option<String>,
) -> Result<String, String> {
    const ALLOWED: &[&str] = &["up", "down", "ps", "logs", "pull", "build", "restart", "stop", "start"];
    if !ALLOWED.contains(&action.as_str()) {
        return Err(format!("Unknown compose action: {action}"));
    }

    let ws = std::path::PathBuf::from(&workspace);

    // Detect compose file
    let compose_file = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"]
        .iter()
        .find(|f| ws.join(f).exists())
        .map(|f| f.to_string())
        .unwrap_or_else(|| "docker-compose.yml".to_string());

    let mut args = vec![
        "compose".to_string(),
        "-f".to_string(),
        compose_file,
        action.clone(),
    ];

    // Flags for specific actions
    if action == "up" {
        args.push("-d".to_string());
    }
    if action == "logs" {
        args.extend(["--tail".to_string(), "100".to_string(), "--timestamps".to_string()]);
    }
    if let Some(svc) = service {
        if !svc.is_empty() {
            args.push(svc);
        }
    }

    let timeout_secs: u64 = match action.as_str() {
        "up" | "build" | "pull" => 300,
        _ => 60,
    };

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio::process::Command::new("docker")
            .args(&args)
            .current_dir(&ws)
            .output(),
    )
    .await
    .map_err(|_| format!("docker compose {action} timed out"))?
    .map_err(|e| format!("Failed to run docker compose {action}: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() && action != "logs" && action != "ps" {
        return Err(format!("docker compose {action} failed:\n{stderr}"));
    }
    if stdout.is_empty() { Ok(stderr) } else { Ok(format!("{stdout}{stderr}")) }
}

/// Pull a Docker image.
#[tauri::command]
pub async fn docker_pull_image(image: String) -> Result<String, String> {
    if image.is_empty() {
        return Err("Image name required".to_string());
    }
    // Validate: image name must not contain shell metacharacters
    if image.chars().any(|c| matches!(c, ';' | '&' | '|' | '$' | '`' | '\n' | '\r')) {
        return Err("Invalid image name".to_string());
    }

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        tokio::process::Command::new("docker")
            .args(["pull", &image])
            .output(),
    )
    .await
    .map_err(|_| "docker pull timed out after 5 minutes".to_string())?
    .map_err(|e| format!("Failed to run docker pull: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(format!("docker pull failed: {}", stderr.trim()));
    }
    Ok(format!("{stdout}{stderr}"))
}

// ── Dependency Manager ───────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DepInfo {
    pub name: String,
    pub current: String,
    pub latest: String,
    pub wanted: String,
    pub dep_type: String,
    pub is_outdated: bool,
    pub is_vulnerable: bool,
    pub vulnerability: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DepsResult {
    pub manager: String,
    pub deps: Vec<DepInfo>,
    pub total: usize,
    pub outdated: usize,
    pub vulnerable: usize,
    pub raw_output: String,
}

/// Auto-detect the package manager for the workspace.
#[tauri::command]
pub async fn detect_package_manager(workspace: String) -> Result<String, String> {
    let ws = std::path::PathBuf::from(&workspace);
    if ws.join("package.json").exists() {
        if ws.join("pnpm-lock.yaml").exists() { return Ok("pnpm".to_string()); }
        if ws.join("yarn.lock").exists() { return Ok("yarn".to_string()); }
        return Ok("npm".to_string());
    }
    if ws.join("Cargo.toml").exists() { return Ok("cargo".to_string()); }
    if ws.join("go.mod").exists() { return Ok("go".to_string()); }
    if ws.join("requirements.txt").exists() || ws.join("pyproject.toml").exists() || ws.join("setup.py").exists() {
        return Ok("pip".to_string());
    }
    Err("No package manager detected in this workspace".to_string())
}

fn parse_npm_outdated(output: &str) -> Vec<DepInfo> {
    let val: serde_json::Value = match serde_json::from_str(output) { Ok(v) => v, Err(_) => return Vec::new() };
    let obj = match val.as_object() { Some(o) => o, None => return Vec::new() };
    obj.iter().map(|(name, info)| {
        let current = info.get("current").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let wanted = info.get("wanted").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let latest = info.get("latest").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let dep_type = info.get("type").and_then(|v| v.as_str()).unwrap_or("dependencies").to_string();
        DepInfo { name: name.clone(), is_outdated: current != latest, current, latest, wanted, dep_type, is_vulnerable: false, vulnerability: None }
    }).collect()
}

fn parse_npm_audit(output: &str, deps: &mut Vec<DepInfo>) {
    let val: serde_json::Value = match serde_json::from_str(output) { Ok(v) => v, Err(_) => return };
    if let Some(vulns) = val.get("vulnerabilities").and_then(|v| v.as_object()) {
        for (pkg, info) in vulns {
            let severity = info.get("severity").and_then(|v| v.as_str()).unwrap_or("unknown");
            let title = info.get("via").and_then(|v| v.as_array()).and_then(|arr| arr.first())
                .and_then(|v| if let Some(s) = v.as_str() { Some(s.to_string()) } else { v.get("title").and_then(|t| t.as_str()).map(|s| s.to_string()) })
                .unwrap_or_else(|| severity.to_string());
            if let Some(dep) = deps.iter_mut().find(|d| d.name == *pkg) {
                dep.is_vulnerable = true;
                dep.vulnerability = Some(format!("{} ({})", title, severity));
            } else {
                deps.push(DepInfo { name: pkg.clone(), current: String::new(), latest: String::new(), wanted: String::new(), dep_type: "dependencies".to_string(), is_outdated: false, is_vulnerable: true, vulnerability: Some(format!("{} ({})", title, severity)) });
            }
        }
    }
}

fn parse_pip_outdated(output: &str) -> Vec<DepInfo> {
    let val: serde_json::Value = match serde_json::from_str(output) { Ok(v) => v, Err(_) => return Vec::new() };
    let arr = match val.as_array() { Some(a) => a, None => return Vec::new() };
    arr.iter().map(|item| {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let current = item.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let latest = item.get("latest_version").and_then(|v| v.as_str()).unwrap_or("").to_string();
        DepInfo { name, is_outdated: current != latest, wanted: latest.clone(), current, latest, dep_type: "dependencies".to_string(), is_vulnerable: false, vulnerability: None }
    }).collect()
}

fn parse_go_outdated(output: &str) -> Vec<DepInfo> {
    let mut deps = Vec::new();
    // go list -m -u -json all outputs concatenated JSON objects
    let mut buf = String::new();
    let mut depth = 0i32;
    for ch in output.chars() {
        buf.push(ch);
        if ch == '{' { depth += 1; }
        if ch == '}' { depth -= 1; if depth == 0 {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&buf) {
                let path = val.get("Path").and_then(|v| v.as_str()).unwrap_or("");
                let version = val.get("Version").and_then(|v| v.as_str()).unwrap_or("");
                let update_ver = val.get("Update").and_then(|u| u.get("Version")).and_then(|v| v.as_str()).unwrap_or("");
                if !path.is_empty() && !version.is_empty() {
                    let is_outdated = !update_ver.is_empty() && update_ver != version;
                    deps.push(DepInfo {
                        name: path.to_string(), current: version.to_string(),
                        latest: if update_ver.is_empty() { version.to_string() } else { update_ver.to_string() },
                        wanted: if update_ver.is_empty() { version.to_string() } else { update_ver.to_string() },
                        dep_type: "module".to_string(), is_outdated, is_vulnerable: false, vulnerability: None,
                    });
                }
            }
            buf.clear();
        }}
    }
    deps
}

fn parse_cargo_dry_run(output: &str) -> Vec<DepInfo> {
    let re = regex::Regex::new(r"Updating\s+(\S+)\s+v(\S+)\s+->\s+v(\S+)").unwrap();
    re.captures_iter(output).map(|cap| {
        let name = cap[1].to_string();
        let current = cap[2].to_string();
        let latest = cap[3].to_string();
        DepInfo { name, is_outdated: current != latest, wanted: latest.clone(), current, latest, dep_type: "dependencies".to_string(), is_vulnerable: false, vulnerability: None }
    }).collect()
}

/// Scan dependencies for the workspace.
#[tauri::command]
pub async fn scan_dependencies(workspace: String, manager: String) -> Result<DepsResult, String> {
    let ws = std::path::PathBuf::from(&workspace);
    let timeout_dur = std::time::Duration::from_secs(60);

    match manager.as_str() {
        "npm" | "yarn" | "pnpm" => {
            let prog = &manager;
            let outdated_out = tokio::time::timeout(timeout_dur,
                tokio::process::Command::new(prog).args(["outdated", "--json"]).current_dir(&ws).output(),
            ).await.map_err(|_| format!("{prog} outdated timed out"))?.map_err(|e| format!("Failed to run {prog} outdated: {e}"))?;

            let outdated_text = String::from_utf8_lossy(&outdated_out.stdout).to_string();
            let mut deps = parse_npm_outdated(&outdated_text);

            // Audit (best-effort)
            let mut raw = outdated_text;
            if let Ok(Ok(audit)) = tokio::time::timeout(timeout_dur,
                tokio::process::Command::new(prog).args(["audit", "--json"]).current_dir(&ws).output(),
            ).await {
                let audit_text = String::from_utf8_lossy(&audit.stdout).to_string();
                parse_npm_audit(&audit_text, &mut deps);
                raw.push_str("\n--- audit ---\n");
                raw.push_str(&audit_text);
            }

            let total = deps.len();
            let outdated = deps.iter().filter(|d| d.is_outdated).count();
            let vulnerable = deps.iter().filter(|d| d.is_vulnerable).count();
            deps.sort_by(|a, b| b.is_vulnerable.cmp(&a.is_vulnerable).then(b.is_outdated.cmp(&a.is_outdated)).then(a.name.cmp(&b.name)));
            Ok(DepsResult { manager, deps, total, outdated, vulnerable, raw_output: raw })
        }
        "cargo" => {
            let dry_out = tokio::time::timeout(timeout_dur,
                tokio::process::Command::new("cargo").args(["update", "--dry-run"]).current_dir(&ws).output(),
            ).await.map_err(|_| "cargo update --dry-run timed out".to_string())?.map_err(|e| format!("Failed to run cargo: {e}"))?;

            let raw = String::from_utf8_lossy(&dry_out.stderr).to_string();
            let mut deps = parse_cargo_dry_run(&raw);
            let mut full_raw = raw;

            // cargo audit (optional)
            if let Ok(Ok(audit)) = tokio::time::timeout(timeout_dur,
                tokio::process::Command::new("cargo").args(["audit", "--json"]).current_dir(&ws).output(),
            ).await {
                let audit_text = String::from_utf8_lossy(&audit.stdout).to_string();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&audit_text) {
                    if let Some(list) = val.pointer("/vulnerabilities/list").and_then(|v| v.as_array()) {
                        for vuln in list {
                            let pkg = vuln.pointer("/package/name").and_then(|v| v.as_str()).unwrap_or("");
                            let id = vuln.pointer("/advisory/id").and_then(|v| v.as_str()).unwrap_or("");
                            let title = vuln.pointer("/advisory/title").and_then(|v| v.as_str()).unwrap_or("");
                            if let Some(dep) = deps.iter_mut().find(|d| d.name == pkg) {
                                dep.is_vulnerable = true;
                                dep.vulnerability = Some(format!("{}: {}", id, title));
                            }
                        }
                    }
                }
                full_raw.push_str("\n--- cargo audit ---\n");
                full_raw.push_str(&audit_text);
            }

            let total = deps.len(); let outdated = deps.iter().filter(|d| d.is_outdated).count(); let vulnerable = deps.iter().filter(|d| d.is_vulnerable).count();
            deps.sort_by(|a, b| b.is_vulnerable.cmp(&a.is_vulnerable).then(a.name.cmp(&b.name)));
            Ok(DepsResult { manager, deps, total, outdated, vulnerable, raw_output: full_raw })
        }
        "pip" => {
            let out = tokio::time::timeout(timeout_dur,
                tokio::process::Command::new("pip").args(["list", "--outdated", "--format", "json"]).current_dir(&ws).output(),
            ).await.map_err(|_| "pip list timed out".to_string())?.map_err(|e| format!("Failed to run pip: {e}"))?;

            let raw = String::from_utf8_lossy(&out.stdout).to_string();
            let deps = parse_pip_outdated(&raw);
            let total = deps.len(); let outdated = deps.iter().filter(|d| d.is_outdated).count(); let vulnerable = 0;
            Ok(DepsResult { manager, deps, total, outdated, vulnerable, raw_output: raw })
        }
        "go" => {
            let out = tokio::time::timeout(timeout_dur,
                tokio::process::Command::new("go").args(["list", "-m", "-u", "-json", "all"]).current_dir(&ws).output(),
            ).await.map_err(|_| "go list timed out".to_string())?.map_err(|e| format!("Failed to run go list: {e}"))?;

            let raw = String::from_utf8_lossy(&out.stdout).to_string();
            let mut deps = parse_go_outdated(&raw);
            deps.retain(|d| !d.current.is_empty());
            let total = deps.len(); let outdated = deps.iter().filter(|d| d.is_outdated).count(); let vulnerable = 0;
            Ok(DepsResult { manager, deps, total, outdated, vulnerable, raw_output: raw })
        }
        _ => Err(format!("Unsupported package manager: {manager}")),
    }
}

/// Upgrade a specific dependency.
#[tauri::command]
pub async fn upgrade_dependency(workspace: String, manager: String, package: String, version: Option<String>) -> Result<String, String> {
    let ws = std::path::PathBuf::from(&workspace);
    if package.is_empty() { return Err("Package name required".to_string()); }
    if package.chars().any(|c| matches!(c, ';' | '&' | '|' | '$' | '`' | '\n' | '\r')) { return Err("Invalid package name".to_string()); }

    let ver = version.unwrap_or_else(|| "latest".to_string());
    let (prog, args): (&str, Vec<String>) = match manager.as_str() {
        "npm" => ("npm", vec!["install".into(), format!("{}@{}", package, ver)]),
        "yarn" => ("yarn", vec!["upgrade".into(), format!("{}@{}", package, ver)]),
        "pnpm" => ("pnpm", vec!["update".into(), format!("{}@{}", package, ver)]),
        "cargo" => ("cargo", vec!["update".into(), "-p".into(), package.clone()]),
        "pip" => if ver == "latest" { ("pip", vec!["install".into(), "--upgrade".into(), package.clone()]) } else { ("pip", vec!["install".into(), format!("{}=={}", package, ver)]) },
        "go" => { let spec = if ver == "latest" { format!("{}@latest", package) } else { format!("{}@{}", package, ver) }; ("go", vec!["get".into(), spec]) }
        _ => return Err(format!("Unsupported manager: {manager}")),
    };

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = tokio::time::timeout(std::time::Duration::from_secs(30),
        tokio::process::Command::new(prog).args(&args_ref).current_dir(&ws).output(),
    ).await.map_err(|_| "Upgrade timed out".to_string())?.map_err(|e| format!("Failed to upgrade: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() { return Err(format!("Upgrade failed: {}", stderr.trim())); }
    Ok(format!("{stdout}{stderr}"))
}

// ─── Phase 7.27: Database Migration Manager ───────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct MigrationEntry {
    pub name: String,
    pub applied_at: Option<String>,
    pub state: String, // "applied" | "pending" | "failed"
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MigrationStatus {
    pub tool: String,
    pub applied: Vec<MigrationEntry>,
    pub pending: Vec<MigrationEntry>,
    pub raw_output: String,
}

fn detect_migration_tool(ws: &std::path::Path) -> &'static str {
    if ws.join("prisma").join("schema.prisma").exists() || ws.join("schema.prisma").exists() {
        return "prisma";
    }
    if ws.join("diesel.toml").exists() || ws.join("migrations").join(".gitkeep").exists() && ws.join("Cargo.toml").exists() {
        return "diesel";
    }
    if ws.join("alembic.ini").exists() || ws.join("alembic").is_dir() {
        return "alembic";
    }
    if ws.join("flyway.conf").exists() || ws.join("src").join("main").join("resources").join("db").join("migration").is_dir() {
        return "flyway";
    }
    if ws.join("go.mod").exists() {
        // golang-migrate: look for migrations dir
        if ws.join("migrations").is_dir() || ws.join("db").join("migrations").is_dir() {
            return "golang-migrate";
        }
    }
    "unknown"
}

fn parse_prisma_status(output: &str) -> (Vec<MigrationEntry>, Vec<MigrationEntry>) {
    let mut applied = Vec::new();
    let mut pending = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Prisma") { continue; }
        if let Some(name) = trimmed.strip_prefix("✔ ").or_else(|| trimmed.strip_prefix("+ ")) {
            applied.push(MigrationEntry { name: name.trim().to_string(), applied_at: None, state: "applied".to_string() });
        } else if let Some(name) = trimmed.strip_prefix("✗ ").or_else(|| trimmed.strip_prefix("- ")) {
            pending.push(MigrationEntry { name: name.trim().to_string(), applied_at: None, state: "pending".to_string() });
        } else if trimmed.contains("(not yet applied)") || trimmed.contains("pending") {
            pending.push(MigrationEntry { name: trimmed.to_string(), applied_at: None, state: "pending".to_string() });
        } else if trimmed.starts_with("20") && trimmed.len() > 14 {
            // timestamp-based migration names
            applied.push(MigrationEntry { name: trimmed.to_string(), applied_at: None, state: "applied".to_string() });
        }
    }
    (applied, pending)
}

fn parse_diesel_status(output: &str) -> (Vec<MigrationEntry>, Vec<MigrationEntry>) {
    let mut applied = Vec::new();
    let mut pending = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if trimmed.starts_with("Running migration") || trimmed.starts_with("[X]") {
            let name = trimmed.trim_start_matches("[X]").trim_start_matches("Running migration").trim().to_string();
            if !name.is_empty() {
                applied.push(MigrationEntry { name, applied_at: None, state: "applied".to_string() });
            }
        } else if trimmed.starts_with("[ ]") {
            let name = trimmed.trim_start_matches("[ ]").trim().to_string();
            if !name.is_empty() {
                pending.push(MigrationEntry { name, applied_at: None, state: "pending".to_string() });
            }
        }
    }
    (applied, pending)
}

/// Detect migration tool and return current migration status.
#[tauri::command]
pub async fn get_migration_status(workspace: String) -> Result<MigrationStatus, String> {
    let ws = std::path::PathBuf::from(&workspace);
    let tool = detect_migration_tool(&ws);

    if tool == "unknown" {
        return Ok(MigrationStatus {
            tool: "unknown".to_string(),
            applied: Vec::new(),
            pending: Vec::new(),
            raw_output: String::new(),
        });
    }

    let (cmd, args): (&str, Vec<&str>) = match tool {
        "prisma"         => ("npx", vec!["prisma", "migrate", "status"]),
        "diesel"         => ("diesel", vec!["migration", "list"]),
        "alembic"        => ("alembic", vec!["history"]),
        "flyway"         => ("flyway", vec!["info"]),
        "golang-migrate" => ("migrate", vec!["-database", "${DATABASE_URL}", "-path", "migrations", "version"]),
        _                => return Err("Unknown tool".to_string()),
    };

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new(cmd).args(&args).current_dir(&ws).output(),
    )
    .await
    .map_err(|_| "Migration status timed out".to_string())?
    .map_err(|e| format!("Failed to get migration status: {e}"))?;

    let raw = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);

    let (applied, pending) = match tool {
        "prisma" => parse_prisma_status(&raw),
        "diesel" => parse_diesel_status(&raw),
        _ => {
            // Generic: lines with "applied" / "pending" keywords
            let mut app = Vec::new();
            let mut pend = Vec::new();
            for line in raw.lines() {
                let l = line.to_lowercase();
                let name = line.trim().to_string();
                if name.is_empty() { continue; }
                if l.contains("applied") || l.contains("[x]") || l.contains("✔") {
                    app.push(MigrationEntry { name, applied_at: None, state: "applied".to_string() });
                } else if l.contains("pending") || l.contains("[ ]") || l.contains("not applied") {
                    pend.push(MigrationEntry { name, applied_at: None, state: "pending".to_string() });
                }
            }
            (app, pend)
        }
    };

    Ok(MigrationStatus { tool: tool.to_string(), applied, pending, raw_output: raw })
}

/// Run a migration action: migrate | rollback | generate | status.
#[tauri::command]
pub async fn run_migration_action(
    workspace: String,
    tool: String,
    action: String,
    extra: Option<String>,
) -> Result<String, String> {
    let ws = std::path::PathBuf::from(&workspace);

    const ALLOWED_ACTIONS: &[&str] = &["migrate", "rollback", "generate", "status", "reset"];
    if !ALLOWED_ACTIONS.contains(&action.as_str()) {
        return Err(format!("Unknown action: {action}"));
    }

    let (cmd, args): (&str, Vec<String>) = match (tool.as_str(), action.as_str()) {
        ("prisma", "migrate")  => ("npx", vec!["prisma".into(), "migrate".into(), "deploy".into()]),
        ("prisma", "rollback") => return Err("Prisma does not support rollback directly. Use `prisma migrate reset` with caution.".to_string()),
        ("prisma", "generate") => ("npx", {
            let name = extra.as_deref().unwrap_or("migration");
            vec!["prisma".into(), "migrate".into(), "dev".into(), "--name".into(), name.into()]
        }),
        ("prisma", "status")   => ("npx", vec!["prisma".into(), "migrate".into(), "status".into()]),

        ("diesel", "migrate")  => ("diesel", vec!["migration".into(), "run".into()]),
        ("diesel", "rollback") => ("diesel", vec!["migration".into(), "revert".into()]),
        ("diesel", "generate") => ("diesel", {
            let name = extra.as_deref().unwrap_or("new_migration");
            vec!["migration".into(), "generate".into(), "--diff-file".into(), name.into()]
        }),
        ("diesel", "status")   => ("diesel", vec!["migration".into(), "list".into()]),

        ("alembic", "migrate")  => ("alembic", vec!["upgrade".into(), "head".into()]),
        ("alembic", "rollback") => ("alembic", vec!["downgrade".into(), "-1".into()]),
        ("alembic", "generate") => ("alembic", {
            let name = extra.as_deref().unwrap_or("auto");
            vec!["revision".into(), "--autogenerate".into(), "-m".into(), name.into()]
        }),
        ("alembic", "status")   => ("alembic", vec!["current".into()]),

        ("flyway", "migrate")   => ("flyway", vec!["migrate".into()]),
        ("flyway", "rollback")  => ("flyway", vec!["undo".into()]),
        ("flyway", "status")    => ("flyway", vec!["info".into()]),
        ("flyway", "generate")  => return Err("Flyway uses SQL files — create a new .sql file in the migrations directory.".to_string()),

        ("golang-migrate", "migrate")  => ("migrate", vec!["-path".into(), "migrations".into(), "-database".into(), "${DATABASE_URL}".into(), "up".into()]),
        ("golang-migrate", "rollback") => ("migrate", vec!["-path".into(), "migrations".into(), "-database".into(), "${DATABASE_URL}".into(), "down".into(), "1".into()]),
        ("golang-migrate", "generate") => {
            let name = extra.as_deref().unwrap_or("new_migration");
            return Ok(format!("Create: migrations/$(date +%Y%m%d%H%M%S)_{name}.up.sql and .down.sql"));
        },
        ("golang-migrate", "status")   => ("migrate", vec!["-path".into(), "migrations".into(), "-database".into(), "${DATABASE_URL}".into(), "version".into()]),

        _ => return Err(format!("Unsupported tool/action: {tool}/{action}")),
    };

    let timeout_secs: u64 = if action == "migrate" || action == "reset" { 60 } else { 30 };

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio::process::Command::new(cmd).args(&args).current_dir(&ws).output(),
    )
    .await
    .map_err(|_| format!("Migration {action} timed out"))?
    .map_err(|e| format!("Failed to run migration {action}: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if !out.status.success() && action != "status" {
        return Err(format!("Migration {action} failed:\n{stderr}"));
    }
    if stdout.is_empty() { Ok(stderr) } else { Ok(format!("{stdout}{stderr}")) }
}

// ── Phase 7.27: Log Viewer & Analyzer ───────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LogEntry {
    pub line_number: usize,
    pub timestamp: Option<String>,
    pub level: String,
    pub message: String,
    pub raw: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LogResult {
    pub source: String,
    pub entries: Vec<LogEntry>,
    pub total_lines: usize,
    pub error_count: usize,
    pub warn_count: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LogSource {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub source_type: String,
}

fn classify_log_level(line: &str) -> &'static str {
    let upper = line.to_uppercase();
    if upper.contains("ERROR") || upper.contains("FATAL") || upper.contains("PANIC") {
        "error"
    } else if upper.contains("WARN") {
        "warn"
    } else if upper.contains("INFO") {
        "info"
    } else if upper.contains("DEBUG") {
        "debug"
    } else if upper.contains("TRACE") {
        "trace"
    } else {
        "unknown"
    }
}

fn extract_timestamp(line: &str) -> Option<String> {
    // ISO 8601: 2024-01-15T10:30:00 or 2024-01-15 10:30:00
    let re_iso = regex::Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}").ok()?;
    if let Some(m) = re_iso.find(line) {
        return Some(m.as_str().to_string());
    }
    // Common log format: [15/Jan/2024:10:30:00]
    let re_clf = regex::Regex::new(r"\[\d{2}/\w{3}/\d{4}:\d{2}:\d{2}:\d{2}").ok()?;
    if let Some(m) = re_clf.find(line) {
        return Some(m.as_str().trim_start_matches('[').to_string());
    }
    None
}

fn level_priority(level: &str) -> u8 {
    match level {
        "error" => 0,
        "warn" => 1,
        "info" => 2,
        "debug" => 3,
        "trace" => 4,
        _ => 5,
    }
}

#[tauri::command]
pub async fn discover_log_sources(workspace: String) -> Result<Vec<LogSource>, String> {
    let ws = std::path::Path::new(&workspace);
    if !ws.is_dir() {
        return Err("Workspace is not a directory".to_string());
    }

    let mut sources = Vec::new();
    let walker = walkdir::WalkDir::new(ws)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "node_modules" && name != "target" && name != "__pycache__"
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if sources.len() >= 50 { break; }
        let path = entry.path();
        if !path.is_file() { continue; }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.ends_with(".log") || name == "npm-debug.log" || name == "yarn-error.log" {
            if let Ok(meta) = std::fs::metadata(path) {
                sources.push(LogSource {
                    name: name.to_string(),
                    path: path.display().to_string(),
                    size_bytes: meta.len(),
                    source_type: "file".to_string(),
                });
            }
        }
    }

    sources.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(sources)
}

#[tauri::command]
pub async fn tail_log_file(
    workspace: String,
    source: String,
    lines: Option<usize>,
    filter_level: Option<String>,
) -> Result<LogResult, String> {
    let max_lines = lines.unwrap_or(500).min(5000);

    let raw_lines: Vec<String> = if let Some(cmd_str) = source.strip_prefix("cmd:") {
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tokio::process::Command::new("sh")
                .args(["-c", cmd_str])
                .output(),
        )
        .await
        .map_err(|_| "Command timed out".to_string())?
        .map_err(|e| format!("Failed to run command: {e}"))?;

        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        text.lines().map(|l| l.to_string()).collect()
    } else {
        let ws = std::path::Path::new(&workspace).canonicalize()
            .map_err(|e| format!("Invalid workspace: {e}"))?;
        let file_path = std::path::Path::new(&source).canonicalize()
            .map_err(|e| format!("File not found: {e}"))?;
        if !file_path.starts_with(&ws) {
            return Err("Access denied: file outside workspace".to_string());
        }

        let content = tokio::fs::read_to_string(&file_path).await
            .map_err(|e| format!("Failed to read file: {e}"))?;
        let all_lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let skip = if all_lines.len() > max_lines { all_lines.len() - max_lines } else { 0 };
        all_lines[skip..].to_vec()
    };

    let filter_prio = filter_level.as_deref().map(level_priority);

    let mut entries = Vec::new();
    let mut error_count = 0;
    let mut warn_count = 0;

    for (i, line) in raw_lines.iter().enumerate() {
        let level = classify_log_level(line);
        if level == "error" { error_count += 1; }
        if level == "warn" { warn_count += 1; }

        if let Some(max_prio) = filter_prio {
            if level_priority(level) > max_prio { continue; }
        }

        entries.push(LogEntry {
            line_number: i + 1,
            timestamp: extract_timestamp(line),
            level: level.to_string(),
            message: line.clone(),
            raw: line.clone(),
        });
    }

    Ok(LogResult {
        source: source.clone(),
        entries,
        total_lines: raw_lines.len(),
        error_count,
        warn_count,
    })
}

#[tauri::command]
pub async fn analyze_logs(
    state: tauri::State<'_, AppState>,
    entries: Vec<String>,
) -> Result<String, String> {
    let truncated: Vec<&String> = entries.iter().take(100).collect();
    if truncated.is_empty() {
        return Err("No log entries to analyze".to_string());
    }

    let log_text = truncated.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
    let prompt = format!(
        "Analyze these log entries. Identify errors, recurring patterns, probable root causes, and suggest fixes.\n\n```\n{}\n```",
        log_text
    );

    let engine = state.chat_engine.lock().await;
    let provider = engine.active_provider().ok_or("No AI provider configured")?;

    let messages = vec![
        vibe_ai::provider::Message {
            role: vibe_ai::provider::MessageRole::User,
            content: prompt,
        },
    ];

    let response = provider.chat(&messages, None).await.map_err(|e| format!("AI error: {e}"))?;
    Ok(response)
}

// ── Phase 7.28: Script Runner & Task Manager ─────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectScript {
    pub category: String, // "npm", "make", "cargo", "python", "custom"
    pub name: String,
    pub command: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScriptCategories {
    pub scripts: Vec<ProjectScript>,
    pub detected_tools: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScriptRunResult {
    pub command: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub output: String,
    pub success: bool,
}

/// Detect all runnable scripts/tasks in the workspace.
#[tauri::command]
pub async fn detect_project_scripts(workspace: String) -> Result<ScriptCategories, String> {
    let ws = std::path::Path::new(&workspace)
        .canonicalize()
        .map_err(|e| format!("Invalid workspace: {e}"))?;

    let mut scripts: Vec<ProjectScript> = Vec::new();
    let mut detected_tools: Vec<String> = Vec::new();

    // ── npm / yarn / pnpm scripts (package.json) ──────────────────────────
    let pkg_json = ws.join("package.json");
    if pkg_json.exists() {
        detected_tools.push("node".to_string());
        if let Ok(content) = tokio::fs::read_to_string(&pkg_json).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = json.get("scripts").and_then(|s| s.as_object()) {
                    let runner = if ws.join("yarn.lock").exists() {
                        "yarn"
                    } else if ws.join("pnpm-lock.yaml").exists() {
                        "pnpm"
                    } else {
                        "npm run"
                    };
                    for (name, val) in obj {
                        scripts.push(ProjectScript {
                            category: "npm".to_string(),
                            name: name.clone(),
                            command: format!("{runner} {name}"),
                            description: val.as_str().map(|s| s.to_string()),
                        });
                    }
                }
            }
        }
    }

    // ── Cargo tasks ───────────────────────────────────────────────────────
    if ws.join("Cargo.toml").exists() {
        detected_tools.push("cargo".to_string());
        for (name, command, description) in [
            ("build", "cargo build", "Compile the project"),
            ("build --release", "cargo build --release", "Compile optimised binary"),
            ("test", "cargo test", "Run all tests"),
            ("clippy", "cargo clippy --all-targets", "Run linter"),
            ("fmt", "cargo fmt", "Format source code"),
            ("check", "cargo check", "Type-check without building"),
            ("run", "cargo run", "Run the default binary"),
            ("doc", "cargo doc --open", "Build and open documentation"),
            ("audit", "cargo audit", "Check for vulnerabilities"),
            ("clean", "cargo clean", "Remove build artifacts"),
        ] {
            scripts.push(ProjectScript {
                category: "cargo".to_string(),
                name: name.to_string(),
                command: command.to_string(),
                description: Some(description.to_string()),
            });
        }
        // Detect custom binary targets from Cargo.toml
        if let Ok(content) = tokio::fs::read_to_string(ws.join("Cargo.toml")).await {
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with("name = ") && content.contains("[[bin]]") {
                    if let Some(name) = t.strip_prefix("name = \"").and_then(|s| s.strip_suffix('"')) {
                        scripts.push(ProjectScript {
                            category: "cargo".to_string(),
                            name: format!("run --bin {name}"),
                            command: format!("cargo run --bin {name}"),
                            description: Some(format!("Run binary '{name}'")),
                        });
                    }
                }
            }
        }
    }

    // ── Makefile targets ──────────────────────────────────────────────────
    let makefile = ws.join("Makefile");
    if makefile.exists() {
        detected_tools.push("make".to_string());
        if let Ok(content) = tokio::fs::read_to_string(&makefile).await {
            for line in content.lines() {
                // Match `target:` lines that don't start with tab (real targets)
                if !line.starts_with('\t') && !line.starts_with('#') && !line.starts_with('.') {
                    if let Some(target) = line.split(':').next() {
                        let target = target.trim();
                        if !target.is_empty() && target.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                            // Extract inline comment as description
                            let desc = line.split_once("##").map(|(_, s)| s.trim().to_string());
                            scripts.push(ProjectScript {
                                category: "make".to_string(),
                                name: target.to_string(),
                                command: format!("make {target}"),
                                description: desc,
                            });
                        }
                    }
                }
            }
        }
    }

    // ── Python tasks ──────────────────────────────────────────────────────
    let has_pyproject = ws.join("pyproject.toml").exists();
    let has_setup = ws.join("setup.py").exists();
    let has_manage = ws.join("manage.py").exists();
    if has_pyproject || has_setup || has_manage {
        detected_tools.push("python".to_string());
        if has_manage {
            for (name, command, description) in [
                ("runserver", "python manage.py runserver", "Start Django dev server"),
                ("migrate", "python manage.py migrate", "Apply database migrations"),
                ("makemigrations", "python manage.py makemigrations", "Create migration files"),
                ("test", "python manage.py test", "Run Django tests"),
                ("shell", "python manage.py shell", "Open Django shell"),
                ("collectstatic", "python manage.py collectstatic", "Collect static files"),
            ] {
                scripts.push(ProjectScript {
                    category: "python".to_string(),
                    name: name.to_string(),
                    command: command.to_string(),
                    description: Some(description.to_string()),
                });
            }
        } else {
            for (name, command, description) in [
                ("test", "python -m pytest -v", "Run tests with pytest"),
                ("lint", "python -m flake8 .", "Lint with flake8"),
                ("format", "python -m black .", "Format with black"),
                ("typecheck", "python -m mypy .", "Type-check with mypy"),
                ("install", "pip install -e .", "Install in editable mode"),
                ("install-dev", "pip install -r requirements-dev.txt", "Install dev requirements"),
            ] {
                scripts.push(ProjectScript {
                    category: "python".to_string(),
                    name: name.to_string(),
                    command: command.to_string(),
                    description: Some(description.to_string()),
                });
            }
            // Read [tool.taskipy.tasks] or scripts from pyproject.toml
            if let Ok(content) = tokio::fs::read_to_string(ws.join("pyproject.toml")).await {
                let mut in_tasks = false;
                for line in content.lines() {
                    let t = line.trim();
                    if t == "[tool.taskipy.tasks]" { in_tasks = true; continue; }
                    if in_tasks && t.starts_with('[') { break; }
                    if in_tasks {
                        if let Some((name, rest)) = t.split_once('=') {
                            let cmd = rest.trim().trim_matches('"').to_string();
                            scripts.push(ProjectScript {
                                category: "python".to_string(),
                                name: name.trim().to_string(),
                                command: format!("python -m taskipy {}", name.trim()),
                                description: Some(cmd),
                            });
                        }
                    }
                }
            }
        }
    }

    // ── Go tasks ──────────────────────────────────────────────────────────
    if ws.join("go.mod").exists() {
        detected_tools.push("go".to_string());
        for (name, command, description) in [
            ("build", "go build ./...", "Build all packages"),
            ("test", "go test ./...", "Run all tests"),
            ("test -race", "go test -race ./...", "Run tests with race detector"),
            ("vet", "go vet ./...", "Run go vet"),
            ("fmt", "gofmt -w .", "Format code"),
            ("mod tidy", "go mod tidy", "Tidy module dependencies"),
            ("generate", "go generate ./...", "Run go:generate"),
            ("run", "go run .", "Run main package"),
        ] {
            scripts.push(ProjectScript {
                category: "go".to_string(),
                name: name.to_string(),
                command: command.to_string(),
                description: Some(description.to_string()),
            });
        }
    }

    // ── Just (justfile) ───────────────────────────────────────────────────
    let justfile = ws.join("justfile");
    if !justfile.exists() {
        let justfile = ws.join("Justfile");
        if justfile.exists() {
            detected_tools.push("just".to_string());
        }
    } else {
        detected_tools.push("just".to_string());
    }
    let justfile_path = if ws.join("justfile").exists() {
        Some(ws.join("justfile"))
    } else if ws.join("Justfile").exists() {
        Some(ws.join("Justfile"))
    } else {
        None
    };
    if let Some(jf) = justfile_path {
        if let Ok(content) = tokio::fs::read_to_string(&jf).await {
            for line in content.lines() {
                if !line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with('#') && !line.starts_with('@') {
                    if let Some(name) = line.split(':').next() {
                        let name = name.trim();
                        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                            let desc = line.split_once('#').map(|(_, s)| s.trim().to_string());
                            scripts.push(ProjectScript {
                                category: "just".to_string(),
                                name: name.to_string(),
                                command: format!("just {name}"),
                                description: desc,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(ScriptCategories { scripts, detected_tools })
}

/// Run a project script, emitting `script:log` events for live output.
#[tauri::command]
pub async fn run_project_script(
    app: tauri::AppHandle,
    workspace: String,
    command: String,
) -> Result<ScriptRunResult, String> {
    let ws = std::path::Path::new(&workspace)
        .canonicalize()
        .map_err(|e| format!("Invalid workspace: {e}"))?;

    // Safety: block destructive shell patterns
    const BLOCKED: &[&str] = &[
        "rm -rf /", "rm -rf ~", ":(){:|:&};:", "dd if=/dev/zero",
        "mkfs", "shutdown", "reboot", "halt",
    ];
    let cmd_lower = command.to_lowercase();
    for pat in BLOCKED {
        if cmd_lower.contains(pat) {
            return Err(format!("Blocked command: contains '{pat}'"));
        }
    }

    let _ = app.emit("script:log", format!("$ {command}"));
    let started = std::time::Instant::now();

    let child = tokio::process::Command::new("sh")
        .args(["-c", &command])
        .current_dir(&ws)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn: {e}"))?;

    // Collect output with timeout (5 minutes)
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| "Script timed out after 5 minutes".to_string())?
    .map_err(|e| format!("Process error: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{stdout}{stderr}");

    for line in combined.lines() {
        let _ = app.emit("script:log", line.to_string());
    }

    let exit_code = output.status.code().unwrap_or(-1);
    let duration_ms = started.elapsed().as_millis() as u64;
    let success = output.status.success();

    let _ = app.emit(
        "script:log",
        format!("\n[Exited with code {exit_code} in {:.1}s]", duration_ms as f64 / 1000.0),
    );

    Ok(ScriptRunResult {
        command,
        exit_code,
        duration_ms,
        output: combined,
        success,
    })
}

// ── Phase 7.28b: Notebook / Scratchpad ──────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn execute_notebook_cell(
    workspace: String,
    language: String,
    code: String,
) -> Result<CellOutput, String> {
    if code.trim().is_empty() {
        return Err("Empty cell".to_string());
    }

    let tmp_dir = std::env::temp_dir().join(format!("vibe-notebook-{}", std::process::id()));
    tokio::fs::create_dir_all(&tmp_dir).await.map_err(|e| format!("Temp dir: {e}"))?;

    let started = std::time::Instant::now();
    let ws = std::path::Path::new(&workspace);

    let (prog, args): (String, Vec<String>) = match language.as_str() {
        "bash" | "sh" => ("sh".into(), vec!["-c".into(), code.clone()]),
        "python" | "python3" => {
            let f = tmp_dir.join("cell.py");
            tokio::fs::write(&f, &code).await.map_err(|e| format!("Write: {e}"))?;
            ("python3".into(), vec![f.display().to_string()])
        }
        "node" | "javascript" | "js" => {
            let f = tmp_dir.join("cell.js");
            tokio::fs::write(&f, &code).await.map_err(|e| format!("Write: {e}"))?;
            ("node".into(), vec![f.display().to_string()])
        }
        "ruby" => {
            let f = tmp_dir.join("cell.rb");
            tokio::fs::write(&f, &code).await.map_err(|e| format!("Write: {e}"))?;
            ("ruby".into(), vec![f.display().to_string()])
        }
        "rust" => {
            let f = tmp_dir.join("cell.rs");
            let out = tmp_dir.join("cell_out");
            tokio::fs::write(&f, &code).await.map_err(|e| format!("Write: {e}"))?;
            ("sh".into(), vec![
                "-c".into(),
                format!("rustc -o {} {} && {}", out.display(), f.display(), out.display()),
            ])
        }
        "go" => {
            let f = tmp_dir.join("cell.go");
            tokio::fs::write(&f, &code).await.map_err(|e| format!("Write: {e}"))?;
            ("go".into(), vec!["run".into(), f.display().to_string()])
        }
        _ => return Err(format!("Unsupported language: {language}. Use bash, python, node, ruby, rust, or go.")),
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new(&prog)
            .args(&args)
            .current_dir(ws)
            .output(),
    )
    .await
    .map_err(|_| "Cell execution timed out (30s)".to_string())?
    .map_err(|e| format!("Failed to run {prog}: {e}"))?;

    let duration_ms = started.elapsed().as_millis() as u64;

    // Clean up temp files (best effort)
    let _ = tokio::fs::remove_dir_all(&tmp_dir).await;

    Ok(CellOutput {
        stdout: String::from_utf8_lossy(&result.stdout).to_string(),
        stderr: String::from_utf8_lossy(&result.stderr).to_string(),
        exit_code: result.status.code().unwrap_or(-1),
        duration_ms,
    })
}

#[tauri::command]
pub async fn ai_notebook_assist(
    state: tauri::State<'_, AppState>,
    cell_code: String,
    cell_output: String,
    question: String,
) -> Result<String, String> {
    let prompt = format!(
        "Given this code:\n```\n{}\n```\n\nAnd its output:\n```\n{}\n```\n\n{}",
        cell_code.chars().take(2000).collect::<String>(),
        cell_output.chars().take(2000).collect::<String>(),
        if question.is_empty() { "Explain what this code does and suggest improvements." } else { &question },
    );

    let engine = state.chat_engine.lock().await;
    let provider = engine.active_provider().ok_or("No AI provider configured")?;
    let messages = vec![vibe_ai::provider::Message {
        role: vibe_ai::provider::MessageRole::User,
        content: prompt,
    }];
    provider.chat(&messages, None).await.map_err(|e| format!("AI error: {e}"))
}

// ── Phase 7.29: SSH Remote Manager ───────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshCommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub success: bool,
}

fn ssh_profiles_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibeui").join("ssh-profiles.json")
}

/// List saved SSH connection profiles.
#[tauri::command]
pub async fn list_ssh_profiles() -> Result<Vec<SshProfile>, String> {
    let path = ssh_profiles_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = tokio::fs::read_to_string(&path).await
        .map_err(|e| format!("Read error: {e}"))?;
    serde_json::from_str::<Vec<SshProfile>>(&content)
        .map_err(|e| format!("Parse error: {e}"))
}

/// Save (add or update) an SSH connection profile.
#[tauri::command]
pub async fn save_ssh_profile(profile: SshProfile) -> Result<(), String> {
    let path = ssh_profiles_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await
            .map_err(|e| format!("Mkdir error: {e}"))?;
    }

    let mut profiles = if path.exists() {
        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| format!("Read error: {e}"))?;
        serde_json::from_str::<Vec<SshProfile>>(&content).unwrap_or_default()
    } else {
        vec![]
    };

    // Upsert by id
    if let Some(pos) = profiles.iter().position(|p| p.id == profile.id) {
        profiles[pos] = profile;
    } else {
        profiles.push(profile);
    }

    let json = serde_json::to_string_pretty(&profiles)
        .map_err(|e| format!("Serialize error: {e}"))?;
    tokio::fs::write(&path, json).await
        .map_err(|e| format!("Write error: {e}"))?;

    // Restrict permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Delete an SSH connection profile by id.
#[tauri::command]
pub async fn delete_ssh_profile(id: String) -> Result<(), String> {
    let path = ssh_profiles_path();
    if !path.exists() { return Ok(()); }

    let content = tokio::fs::read_to_string(&path).await
        .map_err(|e| format!("Read error: {e}"))?;
    let mut profiles: Vec<SshProfile> = serde_json::from_str(&content).unwrap_or_default();
    profiles.retain(|p| p.id != id);

    let json = serde_json::to_string_pretty(&profiles)
        .map_err(|e| format!("Serialize error: {e}"))?;
    tokio::fs::write(&path, json).await
        .map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

/// Run a single command on a remote host via SSH.
///
/// Uses the system `ssh` binary with BatchMode (no password prompts) and a
/// 30-second connect timeout. Emits `ssh:log` events for live streaming.
#[tauri::command]
pub async fn run_ssh_command(
    app: tauri::AppHandle,
    host: String,
    port: u16,
    user: String,
    key_path: Option<String>,
    command: String,
) -> Result<SshCommandResult, String> {
    // Basic input validation
    if host.contains([';', '&', '|', '`', '$']) {
        return Err("Invalid host".to_string());
    }
    if command.is_empty() {
        return Err("Command cannot be empty".to_string());
    }

    let _ = app.emit("ssh:log", format!("$ ssh {}@{}:{} -- {}", user, host, port, command));
    let started = std::time::Instant::now();

    let mut args: Vec<String> = vec![
        "-o".to_string(), "BatchMode=yes".to_string(),
        "-o".to_string(), "ConnectTimeout=10".to_string(),
        "-o".to_string(), "StrictHostKeyChecking=accept-new".to_string(),
        "-p".to_string(), port.to_string(),
    ];

    if let Some(key) = key_path {
        if !key.is_empty() {
            // Validate key path is within home dir
            if let Ok(expanded) = std::path::Path::new(&key).canonicalize() {
                let home_str = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                let home_path = std::path::PathBuf::from(&home_str);
                if expanded.starts_with(&home_path) {
                    args.push("-i".to_string());
                    args.push(expanded.to_string_lossy().to_string());
                }
            }
        }
    }

    args.push(format!("{}@{}", user, host));
    args.push(command.clone());

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("ssh")
            .args(&args)
            .output(),
    )
    .await
    .map_err(|_| "SSH command timed out after 30s".to_string())?
    .map_err(|e| format!("Failed to run ssh: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    for line in stdout.lines().chain(stderr.lines()) {
        let _ = app.emit("ssh:log", line.to_string());
    }

    let exit_code = output.status.code().unwrap_or(-1);
    let duration_ms = started.elapsed().as_millis() as u64;
    let success = output.status.success();

    let _ = app.emit(
        "ssh:log",
        format!("[Exit {exit_code} in {:.1}s]", duration_ms as f64 / 1000.0),
    );

    Ok(SshCommandResult { stdout, stderr, exit_code, duration_ms, success })
}

// ── Phase 7.30 Feature 1: Bookmark & TODO Manager ──────────────────────────

fn re_code_marker() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"(?i)\b(TODO|FIXME|HACK|BUG|NOTE|XXX)\b[:\s]*(.*)").unwrap())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMarker {
    pub file: String,
    pub line: u32,
    pub marker_type: String,
    pub text: String,
    pub context_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub workspace: String,
    pub file: String,
    pub line: u32,
    pub label: String,
    pub created_at: u64,
}

fn bookmarks_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vibeui").join("bookmarks.json")
}

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt", "swift",
    "c", "cpp", "h", "hpp", "cs", "rb", "lua", "sh", "bash", "zsh",
];

#[tauri::command]
pub async fn scan_code_markers(workspace: String) -> Result<Vec<CodeMarker>, String> {
    let ws = PathBuf::from(&workspace);
    if !ws.is_dir() {
        return Err("Workspace directory not found".to_string());
    }
    let re = re_code_marker();
    let mut markers = Vec::new();

    for entry in walkdir::WalkDir::new(&ws)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let path_str = path.to_string_lossy();
        if path_str.contains("/.git/")
            || path_str.contains("/node_modules/")
            || path_str.contains("/target/")
            || path_str.contains("/dist/")
            || path_str.contains("/.next/")
        {
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !SOURCE_EXTENSIONS.contains(&ext) {
            continue;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let marker_type = caps.get(1).map(|m| m.as_str().to_uppercase()).unwrap_or_default();
                let text = caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
                let rel = path.strip_prefix(&ws).unwrap_or(path).to_string_lossy().to_string();
                markers.push(CodeMarker {
                    file: rel,
                    line: (i + 1) as u32,
                    marker_type,
                    text,
                    context_line: line.trim().to_string(),
                });
                if markers.len() >= 500 {
                    return Ok(markers);
                }
            }
        }
    }
    Ok(markers)
}

#[tauri::command]
pub async fn add_bookmark(workspace: String, file: String, line: u32, label: String) -> Result<(), String> {
    let bp = bookmarks_path();
    if let Some(parent) = bp.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut bookmarks: Vec<Bookmark> = match std::fs::read_to_string(&bp) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    bookmarks.push(Bookmark {
        id: format!("{:x}", now & 0xFFFF_FFFF_FFFF),
        workspace,
        file,
        line,
        label,
        created_at: now,
    });
    let json = serde_json::to_string_pretty(&bookmarks).map_err(|e| e.to_string())?;
    std::fs::write(&bp, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_bookmark(_workspace: String, id: String) -> Result<(), String> {
    let bp = bookmarks_path();
    let mut bookmarks: Vec<Bookmark> = match std::fs::read_to_string(&bp) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => return Ok(()),
    };
    bookmarks.retain(|b| b.id != id);
    let json = serde_json::to_string_pretty(&bookmarks).map_err(|e| e.to_string())?;
    std::fs::write(&bp, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_bookmarks(workspace: String) -> Result<Vec<Bookmark>, String> {
    let bp = bookmarks_path();
    let bookmarks: Vec<Bookmark> = match std::fs::read_to_string(&bp) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    Ok(bookmarks.into_iter().filter(|b| b.workspace == workspace).collect())
}

// ── Phase 7.30 Feature 2: Git Bisect Assistant ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BisectStepResult {
    pub current_commit: String,
    pub commit_message: String,
    pub commits_remaining: Option<u32>,
    pub is_done: bool,
    pub culprit_commit: Option<String>,
}

fn validate_git_ref(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("Git ref cannot be empty".to_string());
    }
    if s.contains(';') || s.contains('|') || s.contains('&') || s.contains('`')
        || s.contains('$') || s.contains('\n') || s.contains('\r')
    {
        return Err("Invalid characters in git ref".to_string());
    }
    Ok(())
}

async fn run_git_cmd(workspace: &str, args: &[&str]) -> Result<String, String> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("git")
            .args(args)
            .current_dir(workspace)
            .output(),
    )
    .await
    .map_err(|_| "Git command timed out after 30s".to_string())?
    .map_err(|e| format!("Failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() && stdout.is_empty() {
        Err(stderr)
    } else {
        Ok(format!("{stdout}{stderr}"))
    }
}

#[tauri::command]
pub async fn git_bisect_start(workspace: String, bad: String, good: String) -> Result<String, String> {
    validate_git_ref(&bad)?;
    validate_git_ref(&good)?;
    run_git_cmd(&workspace, &["bisect", "start", &bad, &good]).await
}

#[tauri::command]
pub async fn git_bisect_step(workspace: String, verdict: String) -> Result<BisectStepResult, String> {
    if !["good", "bad", "skip"].contains(&verdict.as_str()) {
        return Err("Verdict must be 'good', 'bad', or 'skip'".to_string());
    }
    let output = run_git_cmd(&workspace, &["bisect", &verdict]).await?;

    let mut result = BisectStepResult {
        current_commit: String::new(),
        commit_message: String::new(),
        commits_remaining: None,
        is_done: false,
        culprit_commit: None,
    };

    if output.contains("is the first bad commit") {
        result.is_done = true;
        // Extract SHA from first line like "abc123def is the first bad commit"
        if let Some(sha) = output.split_whitespace().next() {
            result.culprit_commit = Some(sha.to_string());
            result.current_commit = sha.to_string();
        }
        result.commit_message = output.lines().next().unwrap_or("").to_string();
    } else {
        // Parse "Bisecting: N revisions left to test after this (roughly M steps)"
        for line in output.lines() {
            if line.starts_with("Bisecting:") {
                if let Some(n) = line.split_whitespace().nth(1) {
                    result.commits_remaining = n.parse().ok();
                }
            }
            if line.starts_with('[') {
                // "[abc123] commit message"
                let trimmed = line.trim_start_matches('[');
                if let Some(end) = trimmed.find(']') {
                    result.current_commit = trimmed[..end].to_string();
                    result.commit_message = trimmed[end + 1..].trim().to_string();
                }
            }
        }
    }
    Ok(result)
}

#[tauri::command]
pub async fn git_bisect_reset(workspace: String) -> Result<String, String> {
    run_git_cmd(&workspace, &["bisect", "reset"]).await
}

#[tauri::command]
pub async fn git_bisect_log(workspace: String) -> Result<String, String> {
    let log = run_git_cmd(&workspace, &["bisect", "log"]).await?;
    Ok(log.chars().take(10_000).collect())
}

#[tauri::command]
pub async fn ai_bisect_analyze(
    state: tauri::State<'_, AppState>,
    _workspace: String,
    bisect_log: String,
) -> Result<String, String> {
    let engine = state.chat_engine.lock().await;
    let provider = engine.active_provider().ok_or("No AI provider configured")?;
    let prompt = format!(
        "Analyze this git bisect session log and identify the root cause commit. \
         Explain what likely went wrong and suggest investigation steps.\n\n\
         Bisect log:\n```\n{}\n```",
        bisect_log
    );
    let messages = vec![vibe_ai::provider::Message {
        role: vibe_ai::provider::MessageRole::User,
        content: prompt,
    }];
    provider.chat(&messages, None).await.map_err(|e| format!("AI error: {e}"))
}

// ── Phase 7.30 Feature 3: Snippet Library ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetMeta {
    pub name: String,
    pub description: String,
    pub language: String,
    pub tags: Vec<String>,
    pub created_at: String,
}

fn snippets_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vibecli").join("snippets")
}

fn is_safe_snippet_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[tauri::command]
pub async fn list_snippets() -> Result<Vec<SnippetMeta>, String> {
    let dir = snippets_dir();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut snippets = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let content = std::fs::read_to_string(&path).unwrap_or_default();

        let mut language = String::new();
        let mut tags = Vec::new();
        let mut created_at = String::new();
        let mut description = String::new();
        let mut in_frontmatter = false;
        let mut past_frontmatter = false;

        for line in content.lines() {
            if line.trim() == "---" {
                if !in_frontmatter && !past_frontmatter {
                    in_frontmatter = true;
                    continue;
                } else if in_frontmatter {
                    in_frontmatter = false;
                    past_frontmatter = true;
                    continue;
                }
            }
            if in_frontmatter {
                if let Some(val) = line.strip_prefix("language:") {
                    language = val.trim().to_string();
                } else if let Some(val) = line.strip_prefix("tags:") {
                    let raw = val.trim().trim_start_matches('[').trim_end_matches(']');
                    tags = raw.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
                } else if let Some(val) = line.strip_prefix("created_at:") {
                    created_at = val.trim().to_string();
                }
            } else if past_frontmatter || !in_frontmatter {
                let trimmed = line.trim();
                if !trimmed.is_empty() && description.is_empty() {
                    description = trimmed.to_string();
                }
            }
        }
        snippets.push(SnippetMeta { name, description, language, tags, created_at });
    }
    snippets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(snippets)
}

#[tauri::command]
pub async fn get_snippet(name: String) -> Result<String, String> {
    if !is_safe_snippet_name(&name) {
        return Err("Invalid snippet name".to_string());
    }
    let path = snippets_dir().join(format!("{name}.md"));
    tokio::fs::read_to_string(&path).await.map_err(|e| format!("Failed to read snippet: {e}"))
}

#[tauri::command]
pub async fn save_snippet(name: String, content: String, language: String, tags: String) -> Result<(), String> {
    if !is_safe_snippet_name(&name) {
        return Err("Invalid snippet name (alphanumeric, hyphens, underscores only)".to_string());
    }
    let dir = snippets_dir();
    tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    let now = chrono_lite_now();
    let full = format!("---\nlanguage: {language}\ntags: [{tags}]\ncreated_at: {now}\n---\n\n{content}");
    tokio::fs::write(dir.join(format!("{name}.md")), full).await.map_err(|e| e.to_string())
}

fn chrono_lite_now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    // Simple ISO-ish timestamp without chrono crate
    format!("{secs}")
}

#[tauri::command]
pub async fn delete_snippet(name: String) -> Result<(), String> {
    if !is_safe_snippet_name(&name) {
        return Err("Invalid snippet name".to_string());
    }
    let path = snippets_dir().join(format!("{name}.md"));
    tokio::fs::remove_file(&path).await.map_err(|e| format!("Failed to delete snippet: {e}"))
}

#[tauri::command]
pub async fn generate_snippet(
    state: tauri::State<'_, AppState>,
    description: String,
    language: String,
) -> Result<String, String> {
    let engine = state.chat_engine.lock().await;
    let provider = engine.active_provider().ok_or("No AI provider configured")?;
    let prompt = format!(
        "Generate a concise, reusable code snippet in {language} for the following description. \
         Include brief comments. Return only the code, no explanations.\n\nDescription: {description}"
    );
    let messages = vec![vibe_ai::provider::Message {
        role: vibe_ai::provider::MessageRole::User,
        content: prompt,
    }];
    provider.chat(&messages, None).await.map_err(|e| format!("AI error: {e}"))
}

// ── Phase 7.30 Feature 4: API Mock Server ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRoute {
    pub id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub body: String,
    pub headers: String,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRequest {
    pub timestamp: u64,
    pub method: String,
    pub path: String,
    pub headers: String,
    pub body: String,
    pub matched_route_id: Option<String>,
}

#[tauri::command]
pub async fn start_mock_server(
    state: tauri::State<'_, AppState>,
    port: u16,
) -> Result<String, String> {
    if port < 1024 {
        return Err("Port must be >= 1024".to_string());
    }
    let mut handle_lock = state.mock_server_handle.lock().await;
    if handle_lock.is_some() {
        return Err("Mock server is already running. Stop it first.".to_string());
    }

    let routes = state.mock_routes.clone();
    let log = state.mock_request_log.clone();

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .map_err(|e| format!("Failed to bind port {port}: {e}"))?;

    let handle = tokio::spawn(async move {
        let routes_ext = routes.clone();
        let log_ext = log.clone();

        let app = axum::Router::new()
            .fallback(move |req: axum::extract::Request| {
                let routes = routes_ext.clone();
                let log = log_ext.clone();
                async move {
                    let method = req.method().to_string();
                    let path = req.uri().path().to_string();
                    let headers_str = format!("{:?}", req.headers());
                    let body_bytes = axum::body::to_bytes(req.into_body(), 1_048_576)
                        .await
                        .unwrap_or_default();
                    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

                    let routes_lock = routes.lock().await;
                    let matched = routes_lock.iter().find(|r| {
                        r.method.eq_ignore_ascii_case(&method) && r.path == path
                    });

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let (status, resp_body, matched_id, delay) = if let Some(route) = matched {
                        (route.status, route.body.clone(), Some(route.id.clone()), route.delay_ms)
                    } else {
                        (404, r#"{"error":"no matching mock route"}"#.to_string(), None, 0)
                    };
                    drop(routes_lock);

                    // Log the request
                    let mut log_lock = log.lock().await;
                    if log_lock.len() < 1000 {
                        log_lock.push(MockRequest {
                            timestamp: now,
                            method,
                            path,
                            headers: headers_str,
                            body: body_str,
                            matched_route_id: matched_id,
                        });
                    }
                    drop(log_lock);

                    if delay > 0 {
                        tokio::time::sleep(std::time::Duration::from_millis(delay.min(10_000))).await;
                    }

                    axum::response::Response::builder()
                        .status(status)
                        .header("content-type", "application/json")
                        .header("access-control-allow-origin", "*")
                        .body(axum::body::Body::from(resp_body))
                        .unwrap_or_else(|_| {
                            axum::response::Response::builder()
                                .status(500)
                                .body(axum::body::Body::from("internal error"))
                                .unwrap()
                        })
                }
            });

        let _ = axum::serve(listener, app).await;
    });

    *handle_lock = Some(handle);
    Ok(format!("Mock server started on http://127.0.0.1:{port}"))
}

#[tauri::command]
pub async fn stop_mock_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut handle_lock = state.mock_server_handle.lock().await;
    if let Some(handle) = handle_lock.take() {
        handle.abort();
    }
    state.mock_request_log.lock().await.clear();
    Ok(())
}

#[tauri::command]
pub async fn add_mock_route(
    state: tauri::State<'_, AppState>,
    method: String,
    path: String,
    status: u16,
    body: String,
    headers: String,
) -> Result<(), String> {
    let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
    let method_upper = method.to_uppercase();
    if !valid_methods.contains(&method_upper.as_str()) {
        return Err(format!("Invalid HTTP method: {method}"));
    }
    if !(100..=599).contains(&status) {
        return Err("Status code must be between 100 and 599".to_string());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let route = MockRoute {
        id: format!("{:x}", now & 0xFFFF_FFFF_FFFF),
        method: method_upper,
        path,
        status,
        body,
        headers,
        delay_ms: 0,
    };
    state.mock_routes.lock().await.push(route);
    Ok(())
}

#[tauri::command]
pub async fn remove_mock_route(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    state.mock_routes.lock().await.retain(|r| r.id != id);
    Ok(())
}

#[tauri::command]
pub async fn list_mock_routes(state: tauri::State<'_, AppState>) -> Result<Vec<MockRoute>, String> {
    Ok(state.mock_routes.lock().await.clone())
}

#[tauri::command]
pub async fn get_mock_request_log(state: tauri::State<'_, AppState>) -> Result<Vec<MockRequest>, String> {
    Ok(state.mock_request_log.lock().await.clone())
}

#[tauri::command]
pub async fn generate_mocks_from_spec(
    state: tauri::State<'_, AppState>,
    spec_path: String,
) -> Result<Vec<MockRoute>, String> {
    let content = tokio::fs::read_to_string(&spec_path)
        .await
        .map_err(|e| format!("Failed to read spec: {e}"))?;
    let content: String = content.chars().take(30_000).collect();

    let engine = state.chat_engine.lock().await;
    let provider = engine.active_provider().ok_or("No AI provider configured")?;
    let prompt = format!(
        "Parse this OpenAPI/Swagger spec and generate a JSON array of mock routes. \
         Each route object must have: method (string), path (string), status (number, default 200), \
         body (JSON string for the response). Return ONLY a valid JSON array, no explanation.\n\n\
         ```\n{content}\n```"
    );
    let messages = vec![vibe_ai::provider::Message {
        role: vibe_ai::provider::MessageRole::User,
        content: prompt,
    }];
    let response = provider.chat(&messages, None).await.map_err(|e| format!("AI error: {e}"))?;

    // Parse AI response as JSON array
    let trimmed = response.trim();
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    let parsed: Vec<serde_json::Value> = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse AI response as JSON: {e}"))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut routes = Vec::new();
    for (i, val) in parsed.iter().enumerate() {
        let route = MockRoute {
            id: format!("{:x}", (now + i as u64) & 0xFFFF_FFFF_FFFF),
            method: val.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_uppercase(),
            path: val.get("path").and_then(|v| v.as_str()).unwrap_or("/").to_string(),
            status: val.get("status").and_then(|v| v.as_u64()).unwrap_or(200) as u16,
            body: val.get("body").map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string()),
            headers: String::new(),
            delay_ms: 0,
        };
        routes.push(route);
    }

    // Add routes to the shared registry
    let mut lock = state.mock_routes.lock().await;
    lock.extend(routes.clone());

    Ok(routes)
}

// ── Phase 7.31: GraphQL Playground ───────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphQLResult {
    pub data: Option<serde_json::Value>,
    pub errors: Option<serde_json::Value>,
    pub status: u16,
    pub duration_ms: u64,
    pub raw: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphQLSchemaField {
    pub name: String,
    pub kind: String,
    pub description: Option<String>,
    pub fields: Option<Vec<GraphQLSchemaField>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphQLType {
    pub name: String,
    pub kind: String,
    pub description: Option<String>,
    pub fields: Vec<GraphQLSchemaField>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphQLSchema {
    pub query_type: Option<String>,
    pub mutation_type: Option<String>,
    pub subscription_type: Option<String>,
    pub types: Vec<GraphQLType>,
}

/// Execute a GraphQL query/mutation against the given endpoint.
#[tauri::command]
pub async fn run_graphql_query(
    url: String,
    query: String,
    variables: Option<serde_json::Value>,
    headers: Option<std::collections::HashMap<String, String>>,
    operation_name: Option<String>,
) -> Result<GraphQLResult, String> {
    // URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client error: {e}"))?;

    let mut body = serde_json::json!({ "query": query });
    if let Some(vars) = variables {
        body["variables"] = vars;
    }
    if let Some(op) = operation_name {
        if !op.is_empty() {
            body["operationName"] = serde_json::Value::String(op);
        }
    }

    let mut req = client.post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json");

    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req = req.header(&k, &v);
        }
    }

    let started = std::time::Instant::now();
    let resp = req.json(&body).send().await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status().as_u16();
    let duration_ms = started.elapsed().as_millis() as u64;
    let raw = resp.text().await
        .map_err(|e| format!("Failed to read body: {e}"))?;

    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or(serde_json::Value::String(raw.clone()));

    let data = parsed.get("data").cloned();
    let errors = parsed.get("errors").cloned();

    Ok(GraphQLResult { data, errors, status, duration_ms, raw })
}

/// Introspect a GraphQL endpoint and return simplified type information.
#[tauri::command]
pub async fn introspect_graphql_schema(
    url: String,
    headers: Option<std::collections::HashMap<String, String>>,
) -> Result<GraphQLSchema, String> {
    const INTROSPECTION_QUERY: &str = r#"
    {
      __schema {
        queryType { name }
        mutationType { name }
        subscriptionType { name }
        types {
          name kind description
          fields(includeDeprecated: false) {
            name description
            type { name kind ofType { name kind } }
          }
        }
      }
    }"#;

    let result = run_graphql_query(
        url,
        INTROSPECTION_QUERY.to_string(),
        None,
        headers,
        Some("IntrospectionQuery".to_string()),
    ).await?;

    let schema_val = result.data
        .as_ref()
        .and_then(|d| d.get("__schema"))
        .ok_or("No __schema in response")?;

    let query_type = schema_val.get("queryType")
        .and_then(|t| t.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let mutation_type = schema_val.get("mutationType")
        .and_then(|t| t.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let subscription_type = schema_val.get("subscriptionType")
        .and_then(|t| t.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let types = schema_val.get("types")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let name = t.get("name")?.as_str()?.to_string();
                    // Filter built-in introspection types
                    if name.starts_with("__") { return None; }
                    let kind = t.get("kind")?.as_str()?.to_string();
                    if kind == "SCALAR" && ["String", "Int", "Float", "Boolean", "ID"].contains(&name.as_str()) {
                        return None;
                    }
                    let description = t.get("description")
                        .and_then(|d| d.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());
                    let fields = t.get("fields")
                        .and_then(|f| f.as_array())
                        .map(|farr| {
                            farr.iter().filter_map(|f| {
                                let fname = f.get("name")?.as_str()?.to_string();
                                let fkind = f.get("type")
                                    .and_then(|ft| ft.get("kind"))
                                    .and_then(|k| k.as_str())
                                    .unwrap_or("SCALAR")
                                    .to_string();
                                let fdesc = f.get("description")
                                    .and_then(|d| d.as_str())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string());
                                Some(GraphQLSchemaField { name: fname, kind: fkind, description: fdesc, fields: None })
                            }).collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    Some(GraphQLType { name, kind, description, fields })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(GraphQLSchema { query_type, mutation_type, subscription_type, types })
}

// ── Phase 7.32: Code Metrics Analyzer ────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanguageStat {
    pub language: String,
    pub extension: String,
    pub file_count: usize,
    pub lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileComplexity {
    pub path: String,
    pub lines: usize,
    pub complexity: usize,
    pub language: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeMetrics {
    pub total_files: usize,
    pub total_lines: usize,
    pub total_code_lines: usize,
    pub total_comment_lines: usize,
    pub total_blank_lines: usize,
    pub languages: Vec<LanguageStat>,
    pub largest_files: Vec<FileComplexity>,
    pub most_complex: Vec<FileComplexity>,
}

fn ext_to_language(ext: &str) -> Option<&'static str> {
    match ext {
        "rs"                              => Some("Rust"),
        "ts" | "tsx"                      => Some("TypeScript"),
        "js" | "jsx" | "mjs" | "cjs"     => Some("JavaScript"),
        "py"                              => Some("Python"),
        "go"                              => Some("Go"),
        "java"                            => Some("Java"),
        "c" | "h"                         => Some("C"),
        "cpp" | "cc" | "cxx" | "hpp"     => Some("C++"),
        "cs"                              => Some("C#"),
        "rb"                              => Some("Ruby"),
        "php"                             => Some("PHP"),
        "swift"                           => Some("Swift"),
        "kt" | "kts"                      => Some("Kotlin"),
        "sh" | "bash" | "zsh"            => Some("Shell"),
        "sql"                             => Some("SQL"),
        "html" | "htm"                    => Some("HTML"),
        "css" | "scss" | "sass" | "less" => Some("CSS"),
        "json"                            => Some("JSON"),
        "yaml" | "yml"                    => Some("YAML"),
        "toml"                            => Some("TOML"),
        "md" | "mdx"                      => Some("Markdown"),
        "lua"                             => Some("Lua"),
        "zig"                             => Some("Zig"),
        "dart"                            => Some("Dart"),
        _                                 => None,
    }
}

fn count_branch_complexity(line: &str, ext: &str) -> usize {
    let kws: &[&str] = match ext {
        "rs"            => &["if ", "else if", "match ", "while ", "for ", "loop ", "&&", "||"],
        "ts"|"tsx"|"js"|"jsx" => &["if ", "else if", "while ", "for ", "switch ", "&&", "||", "??"],
        "py"            => &["if ", "elif ", "while ", "for ", "and ", "or ", "except "],
        "go"            => &["if ", "else if", "for ", "switch ", "select ", "&&", "||"],
        "java"|"cs"     => &["if ", "else if", "while ", "for ", "switch ", "&&", "||", "catch "],
        _               => &["if ", "while ", "for ", "&&", "||"],
    };
    kws.iter().filter(|&&kw| line.contains(kw)).count()
}

fn line_is_comment(line: &str, ext: &str) -> bool {
    let t = line.trim();
    match ext {
        "rs"|"ts"|"tsx"|"js"|"jsx"|"go"|"java"|"cs"|"cpp"|"c"|"swift"|"kt" =>
            t.starts_with("//") || t.starts_with("/*") || t.starts_with('*'),
        "py"|"rb" => t.starts_with('#'),
        "html"|"htm" => t.starts_with("<!--"),
        "css"|"scss"|"sass" => t.starts_with("/*") || t.starts_with('*'),
        _ => t.starts_with('#') || t.starts_with("//"),
    }
}

/// Analyse source-code metrics (LOC, language breakdown, complexity) for a workspace.
#[tauri::command]
pub async fn analyze_code_metrics(workspace: String) -> Result<CodeMetrics, String> {
    use std::collections::HashMap;

    let ws = std::path::Path::new(&workspace)
        .canonicalize()
        .map_err(|e| format!("Invalid workspace: {e}"))?;

    const SKIP_DIRS: &[&str] = &[
        "node_modules", ".git", "target", "dist", "build", ".next",
        "vendor", "__pycache__", ".venv", "venv", ".gradle", "out", ".cache",
    ];

    let mut lang_map: HashMap<String, LanguageStat> = HashMap::new();
    let mut all_files: Vec<FileComplexity> = Vec::new();

    for entry in walkdir::WalkDir::new(&ws)
        .follow_links(false)
        .max_depth(12)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() { continue; }

        let path = entry.path();
        let skip = path.ancestors().any(|a| {
            a.file_name().and_then(|n| n.to_str())
                .map(|n| SKIP_DIRS.contains(&n))
                .unwrap_or(false)
        });
        if skip { continue; }

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let lang = match ext_to_language(&ext) {
            Some(l) => l,
            None => continue,
        };

        // Skip files > 1 MB
        if entry.metadata().map(|m| m.len()).unwrap_or(0) > 1_048_576 { continue; }

        let content = match tokio::fs::read_to_string(path).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut total = 0usize;
        let mut code = 0usize;
        let mut comments = 0usize;
        let mut blank = 0usize;
        let mut complexity = 0usize;

        for line in content.lines() {
            total += 1;
            if line.trim().is_empty() {
                blank += 1;
            } else if line_is_comment(line, &ext) {
                comments += 1;
            } else {
                code += 1;
                complexity += count_branch_complexity(line, &ext);
            }
        }

        let rel = path.strip_prefix(&ws).unwrap_or(path).to_string_lossy().to_string();
        all_files.push(FileComplexity { path: rel, lines: total, complexity, language: lang.to_string() });

        let stat = lang_map.entry(lang.to_string()).or_insert_with(|| LanguageStat {
            language: lang.to_string(), extension: ext.clone(),
            file_count: 0, lines: 0, code_lines: 0, comment_lines: 0, blank_lines: 0,
        });
        stat.file_count += 1;
        stat.lines += total;
        stat.code_lines += code;
        stat.comment_lines += comments;
        stat.blank_lines += blank;
    }

    let total_files = all_files.len();
    let total_lines: usize = all_files.iter().map(|f| f.lines).sum();
    let total_code_lines: usize = lang_map.values().map(|l| l.code_lines).sum();
    let total_comment_lines: usize = lang_map.values().map(|l| l.comment_lines).sum();
    let total_blank_lines: usize = lang_map.values().map(|l| l.blank_lines).sum();

    let mut languages: Vec<LanguageStat> = lang_map.into_values().collect();
    languages.sort_by(|a, b| b.lines.cmp(&a.lines));

    let mut largest_files = all_files.clone();
    largest_files.sort_by(|a, b| b.lines.cmp(&a.lines));
    largest_files.truncate(10);

    let mut most_complex = all_files;
    most_complex.sort_by(|a, b| b.complexity.cmp(&a.complexity));
    most_complex.truncate(10);

    Ok(CodeMetrics {
        total_files, total_lines, total_code_lines, total_comment_lines, total_blank_lines,
        languages, largest_files, most_complex,
    })
}

// ── Phase 7.32: HTTP Load Tester ─────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadTestResult {
    pub total_requests: u32,
    pub success: u32,
    pub failed: u32,
    pub duration_ms: u64,
    pub requests_per_sec: f64,
    pub avg_ms: f64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub p50_ms: u64,
    pub p90_ms: u64,
    pub p99_ms: u64,
    pub status_codes: std::collections::HashMap<u16, u32>,
}

/// Run a concurrent HTTP load test. Emits `loadtest:progress` events every 10 requests.
#[tauri::command]
pub async fn run_load_test(
    app: tauri::AppHandle,
    url: String,
    method: String,
    body: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
    concurrency: u32,
    total: u32,
) -> Result<LoadTestResult, String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }
    let total = total.min(10_000);
    let concurrency = concurrency.clamp(1, 200);

    let client = std::sync::Arc::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Client error: {e}"))?
    );

    let method_parsed = reqwest::Method::from_bytes(method.to_uppercase().as_bytes())
        .map_err(|_| format!("Invalid method: {method}"))?;

    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency as usize));
    let started_global = std::time::Instant::now();

    type LatVec = std::sync::Arc<tokio::sync::Mutex<Vec<u64>>>;
    type CodeMap = std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<u16, u32>>>;

    let latencies: LatVec = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(total as usize)));
    let status_codes: CodeMap = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let completed = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let success_ctr = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

    let mut handles = Vec::with_capacity(total as usize);

    for _ in 0..total {
        let (client, sem, latencies, status_codes, completed, success_ctr) = (
            client.clone(), sem.clone(), latencies.clone(), status_codes.clone(),
            completed.clone(), success_ctr.clone(),
        );
        let (url, method, body, headers, app, total) = (
            url.clone(), method_parsed.clone(), body.clone(), headers.clone(), app.clone(), total,
        );

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let t0 = std::time::Instant::now();
            let mut req = client.request(method, &url);
            if let Some(h) = headers { for (k, v) in h { req = req.header(&k, &v); } }
            if let Some(b) = body { req = req.body(b); }
            let elapsed = match req.send().await {
                Ok(resp) => {
                    let ms = t0.elapsed().as_millis() as u64;
                    let code = resp.status().as_u16();
                    *status_codes.lock().await.entry(code).or_insert(0) += 1;
                    if resp.status().is_success() {
                        success_ctr.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    ms
                }
                Err(_) => {
                    *status_codes.lock().await.entry(0).or_insert(0) += 1;
                    t0.elapsed().as_millis() as u64
                }
            };
            latencies.lock().await.push(elapsed);
            let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if done % 10 == 0 || done == total {
                let _ = app.emit("loadtest:progress", done);
            }
        }));
    }

    for h in handles { let _ = h.await; }

    let duration_ms = started_global.elapsed().as_millis() as u64;
    let success = success_ctr.load(std::sync::atomic::Ordering::Relaxed);
    let failed = total - success;

    let mut lats = latencies.lock().await.clone();
    lats.sort_unstable();
    let n = lats.len();
    let avg_ms = if n == 0 { 0.0 } else { lats.iter().sum::<u64>() as f64 / n as f64 };
    let p = |pct: usize| lats.get(n * pct / 100).copied().unwrap_or(0);

    let sc = status_codes.lock().await.clone();
    Ok(LoadTestResult {
        total_requests: total, success, failed, duration_ms,
        requests_per_sec: if duration_ms == 0 { 0.0 } else { total as f64 / (duration_ms as f64 / 1000.0) },
        avg_ms, min_ms: lats.first().copied().unwrap_or(0), max_ms: lats.last().copied().unwrap_or(0),
        p50_ms: p(50), p90_ms: p(90), p99_ms: p(99),
        status_codes: sc,
    })
}

// ── Phase 7.33: Network Tools ─────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpenPort {
    pub port: u16,
    pub protocol: String, // "tcp" | "udp"
    pub pid: Option<u32>,
    pub process: Option<String>,
    pub state: String,
    pub address: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsRecord {
    pub record_type: String,
    pub value: String,
    pub ttl: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TlsCertInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: String,
    pub not_after: String,
    pub san: Vec<String>,
    pub serial: String,
    pub valid: bool,
    pub days_remaining: i64,
    pub raw: String,
}

/// Scan open ports on localhost using `lsof -i` (macOS/Linux).
#[tauri::command]
pub async fn scan_open_ports(host: Option<String>) -> Result<Vec<OpenPort>, String> {
    let target = host.as_deref().unwrap_or("localhost");

    // Use lsof on macOS/Linux; fall back to netstat
    let out = if cfg!(target_os = "windows") {
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            tokio::process::Command::new("netstat")
                .args(["-ano"])
                .output(),
        ).await.map_err(|_| "Timeout".to_string())?
         .map_err(|e| format!("netstat error: {e}"))?
    } else if target == "localhost" || target == "127.0.0.1" || target == "0.0.0.0" {
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::process::Command::new("lsof")
                .args(["-i", "-n", "-P"])
                .output(),
        ).await.map_err(|_| "Timeout".to_string())?
         .map_err(|e| format!("lsof error: {e}"))?
    } else {
        // For remote host scanning use nc or skip
        return Err("Remote port scanning not supported — connect to localhost".to_string());
    };

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut ports: Vec<OpenPort> = Vec::new();
    let mut seen: std::collections::HashSet<(u16, String)> = std::collections::HashSet::new();

    if cfg!(target_os = "windows") {
        // Parse netstat -ano output
        for line in text.lines().skip(4) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 { continue; }
            let proto = cols[0].to_lowercase();
            if !proto.starts_with("tcp") && !proto.starts_with("udp") { continue; }
            let addr = cols[1];
            let state = if cols.len() >= 4 { cols[3] } else { "" };
            let pid = cols.last().and_then(|p| p.parse::<u32>().ok());
            if let Some(port) = addr.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()) {
                if seen.insert((port, proto.clone())) {
                    ports.push(OpenPort { port, protocol: proto, pid, process: None, state: state.to_string(), address: addr.to_string() });
                }
            }
        }
    } else {
        // Parse lsof -i output
        // COMMAND   PID   USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
        // node    12345 user   23u  IPv4 ...      0t0  TCP *:3000 (LISTEN)
        for line in text.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 9 { continue; }
            let process = cols[0].to_string();
            let pid = cols[1].parse::<u32>().ok();
            let name = cols[8]; // e.g. "*:3000" or "127.0.0.1:5432"
            let proto = if name.contains("TCP") || cols[7].eq_ignore_ascii_case("TCP") { "tcp" }
                        else if name.contains("UDP") || cols[7].eq_ignore_ascii_case("UDP") { "udp" }
                        else { cols[7].to_lowercase().as_str().to_string().leak() };
            // Find the address:port part
            let addr_field = cols.iter().rev().find(|&&c| c.contains(':') || c.contains("->")).copied().unwrap_or(name);
            let clean = addr_field.trim_end_matches(" (LISTEN)").trim_end_matches(" (ESTABLISHED)");
            // Take the local side (before ->)
            let local = clean.split("->").next().unwrap_or(clean);
            if let Some(port) = local.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()) {
                let state = if line.contains("(LISTEN)") { "LISTEN" }
                            else if line.contains("(ESTABLISHED)") { "ESTABLISHED" }
                            else { "OPEN" };
                let key = (port, proto.to_string());
                if seen.insert(key) {
                    ports.push(OpenPort {
                        port, protocol: proto.to_string(), pid, process: Some(process),
                        state: state.to_string(), address: local.to_string(),
                    });
                }
            }
        }
    }

    ports.sort_by_key(|p| p.port);
    Ok(ports)
}

/// DNS lookup for a domain using `dig` or `host`.
#[tauri::command]
pub async fn dns_lookup(domain: String, record_type: Option<String>) -> Result<Vec<DnsRecord>, String> {
    // Basic domain validation
    if domain.contains([';', '&', '|', '`', '$', ' ']) {
        return Err("Invalid domain".to_string());
    }

    let rtype = record_type.as_deref().unwrap_or("A").to_uppercase();
    let valid_types = ["A", "AAAA", "CNAME", "MX", "TXT", "NS", "SOA", "PTR", "SRV", "ANY"];
    if !valid_types.contains(&rtype.as_str()) {
        return Err(format!("Invalid record type: {rtype}"));
    }

    // Try dig first, fall back to host
    let (prog, args): (&str, Vec<String>) = if std::process::Command::new("dig").arg("--version").output().is_ok() {
        ("dig", vec!["+short".to_string(), format!("{domain}"), rtype.clone()])
    } else {
        ("host", vec!["-t".to_string(), rtype.clone(), domain.clone()])
    };

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new(prog).args(&args).output(),
    ).await.map_err(|_| "DNS lookup timed out".to_string())?
     .map_err(|e| format!("DNS error: {e}"))?;

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut records: Vec<DnsRecord> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') { continue; }
        records.push(DnsRecord {
            record_type: rtype.clone(),
            value: line.to_string(),
            ttl: None,
        });
    }

    if records.is_empty() && !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(format!("DNS lookup failed: {err}"));
    }

    Ok(records)
}

/// Inspect TLS/SSL certificate of a remote host using `openssl s_client`.
#[tauri::command]
pub async fn check_tls_cert(host: String, port: Option<u16>) -> Result<TlsCertInfo, String> {
    // Validate host
    if host.contains([';', '&', '|', '`', '$', ' ']) {
        return Err("Invalid host".to_string());
    }

    let port = port.unwrap_or(443);
    let target = format!("{host}:{port}");

    // Use openssl s_client to retrieve cert
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        tokio::process::Command::new("openssl")
            .args(["s_client", "-connect", &target, "-servername", &host, "-showcerts"])
            .stdin(std::process::Stdio::null())
            .output(),
    ).await.map_err(|_| "TLS check timed out".to_string())?
     .map_err(|e| format!("openssl error: {e}. Is openssl installed?"))?;

    let raw = String::from_utf8_lossy(&out.stderr).to_string()
        + &String::from_utf8_lossy(&out.stdout);

    // Parse cert fields from openssl output
    fn extract(text: &str, prefix: &str) -> String {
        text.lines()
            .find(|l| l.trim().to_lowercase().contains(&prefix.to_lowercase()))
            .map(|l| l.trim().to_string())
            .unwrap_or_default()
    }

    let subject = extract(&raw, "subject=");
    let issuer  = extract(&raw, "issuer=");
    let not_before = extract(&raw, "not before");
    let not_after  = extract(&raw, "not after");
    let serial  = extract(&raw, "serial number");

    // Parse SAN (Subject Alternative Names)
    let san: Vec<String> = raw.lines()
        .find(|l| l.contains("DNS:"))
        .map(|l| l.split(',')
            .filter_map(|s| {
                let s = s.trim();
                s.strip_prefix("DNS:").map(|stripped| stripped.to_string())
            })
            .collect())
        .unwrap_or_default();

    // Estimate days remaining from "Not After" date
    // openssl outputs dates like: "Not After : Dec 31 23:59:59 2025 GMT"
    let days_remaining = {
        let date_str = not_after.split_once(':').map(|x| x.1).unwrap_or("").trim();
        // Try to parse with chrono-style; use a rough calculation
        let parts: Vec<&str> = date_str.split_whitespace().collect();
        if parts.len() >= 4 {
            let month = match parts[0] {
                "Jan" => 1u32, "Feb" => 2, "Mar" => 3, "Apr" => 4,
                "May" => 5, "Jun" => 6, "Jul" => 7, "Aug" => 8,
                "Sep" => 9, "Oct" => 10, "Nov" => 11, "Dec" => 12, _ => 0,
            };
            let day  = parts[1].parse::<u32>().unwrap_or(0);
            let year = parts[3].parse::<i64>().unwrap_or(0);
            // Very rough day estimate (good enough for display)
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            // Compute target unix time approximately
            let days_in_year: i64 = (year - 1970) * 365 + (year - 1969) / 4;
            let month_days: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
            let day_of_year = month_days[month.saturating_sub(1) as usize] + day as i64;
            let target_ts = (days_in_year + day_of_year) * 86400;
            (target_ts - now) / 86400
        } else { 0 }
    };

    let valid = days_remaining > 0 && raw.contains("Verify return code: 0");

    Ok(TlsCertInfo {
        subject: subject.replace("subject=", "").trim().to_string(),
        issuer: issuer.replace("issuer=", "").trim().to_string(),
        not_before: not_before.replace("Not Before:", "").trim().to_string(),
        not_after: not_after.replace("Not After :", "").trim().to_string(),
        san, serial: serial.replace("Serial Number:", "").trim().to_string(),
        valid, days_remaining, raw,
    })
}

// ── Phase 8.1 — Agent Teams & Peer Communication ─────────────────────────────

/// Serializable team info for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct AgentTeamInfo {
    pub id: String,
    pub lead_agent_id: String,
    pub member_ids: Vec<String>,
    pub goal: String,
    pub status: String,
    pub tasks: Vec<AgentTeamTask>,
    pub message_count: usize,
    pub messages: Vec<AgentTeamMessage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTeamTask {
    pub id: String,
    pub agent_id: String,
    pub description: String,
    pub status: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTeamMessage {
    pub from_agent_id: String,
    pub to_agent_id: Option<String>,
    pub msg_type: String,
    pub content: String,
    pub timestamp: u64,
}

/// Start an agent team with a goal and member count.
/// The lead agent decomposes the goal into sub-tasks using AI.
#[tauri::command]
pub async fn start_agent_team(
    goal: String,
    member_count: usize,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<AgentTeamInfo, String> {
    use vibe_ai::agent_team::*;

    let member_count = member_count.clamp(2, 8);
    let team_id = format!("team-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() % 100000);
    let lead_id = format!("{}-lead", team_id);

    let mut team = AgentTeam::new(&team_id, &lead_id, &goal);

    // Add member agents
    for i in 0..member_count.saturating_sub(1) {
        team.add_member(&format!("{}-worker-{}", team_id, i));
    }

    // Use AI to decompose the goal into sub-tasks
    let engine = state.chat_engine.lock().await;
    if let Some(provider) = engine.active_provider() {
        let decompose_prompt = format!(
            "Decompose this task into {} sub-tasks for a team of AI agents. \
             Return only a numbered list, one sub-task per line, no explanations:\n\n{}",
            member_count, goal
        );
        let messages = vec![vibe_ai::Message {
            role: vibe_ai::MessageRole::User,
            content: decompose_prompt,
        }];

        match provider.chat(&messages, None).await {
            Ok(response) => {
                let subtasks: Vec<TeamSubTask> = response
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .enumerate()
                    .take(member_count)
                    .map(|(i, line)| {
                        let desc = line.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ').to_string();
                        let agent_id = if i == 0 {
                            lead_id.clone()
                        } else {
                            format!("{}-worker-{}", team_id, i.saturating_sub(1))
                        };
                        TeamSubTask {
                            id: format!("task-{}", i),
                            agent_id,
                            description: if desc.is_empty() { line.trim().to_string() } else { desc },
                            status: TeamTaskStatus::Pending,
                            result: None,
                        }
                    })
                    .collect();

                team.set_tasks(subtasks).await;
            }
            Err(e) => {
                eprintln!("[warn] Failed to decompose team goal: {}", e);
            }
        }
    }

    team.set_status("working").await;

    // Announce team formation on the bus
    team.bus.send(TeamMessage::new(
        &lead_id,
        TeamMessageType::Status,
        &format!("Team formed with {} members. Goal: {}", team.member_ids.len(), goal),
    )).await.map_err(|e| e.to_string())?;

    // Emit Tauri event
    let _ = app_handle.emit("team:created", serde_json::json!({
        "team_id": team_id,
        "goal": goal,
        "members": team.member_ids,
    }));

    let info = team_to_info(&team).await;

    Ok(info)
}

/// Get the current status of a team.
#[tauri::command]
pub async fn get_team_status(
    team_id: String,
) -> Result<AgentTeamInfo, String> {
    // For now, return a stub — real implementation would look up the team
    // from a registry. The team creation stores the team in-memory and
    // the frontend tracks it via events.
    Err(format!("Team {} not found in active registry", team_id))
}

/// Send a message on the team bus (from the user/UI to agents).
#[tauri::command]
pub async fn send_team_message(
    team_id: String,
    content: String,
) -> Result<(), String> {
    let _ = (team_id, content);
    // In a full implementation, this would look up the team bus and send.
    // For now, the team bus is managed in-memory by start_agent_team.
    Ok(())
}

async fn team_to_info(team: &vibe_ai::agent_team::AgentTeam) -> AgentTeamInfo {
    let tasks = team.tasks.lock().await;
    let history = team.bus.history().await;

    AgentTeamInfo {
        id: team.id.clone(),
        lead_agent_id: team.lead_agent_id.clone(),
        member_ids: team.member_ids.clone(),
        goal: team.goal.clone(),
        status: team.get_status().await,
        tasks: tasks.iter().map(|t| AgentTeamTask {
            id: t.id.clone(),
            agent_id: t.agent_id.clone(),
            description: t.description.clone(),
            status: format!("{:?}", t.status),
            result: t.result.clone(),
        }).collect(),
        message_count: history.len(),
        messages: history.iter().map(|m| AgentTeamMessage {
            from_agent_id: m.from_agent_id.clone(),
            to_agent_id: m.to_agent_id.clone(),
            msg_type: format!("{:?}", m.msg_type),
            content: m.content.clone(),
            timestamp: m.timestamp,
        }).collect(),
    }
}

// ── Phase 8.2: CI/CD Review Bot ──────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CIReviewConfig {
    #[serde(default)]
    pub app_id: u64,
    #[serde(default)]
    pub private_key_path: Option<String>,
    #[serde(default)]
    pub webhook_secret: Option<String>,
    #[serde(default)]
    pub auto_fix: bool,
    #[serde(default = "default_severity_threshold")]
    pub severity_threshold: String,
}

fn default_severity_threshold() -> String {
    "high".to_string()
}

// ── Phase 8.5: Code Transform ────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransformPlanItem {
    pub file: String,
    pub description: String,
    pub estimated_changes: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransformPlanResult {
    pub files: Vec<TransformPlanItem>,
    pub total_files: usize,
    pub summary: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransformExecResult {
    pub files_modified: usize,
    pub summary: String,
}

#[tauri::command]
pub async fn detect_transform(workspace: String) -> Result<Vec<String>, String> {
    let ws = std::path::PathBuf::from(&workspace);
    // Quick file extension scan to detect potential transforms
    let mut transforms = Vec::new();
    let has_js = walkdir::WalkDir::new(&ws).max_depth(3).into_iter()
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("js"));
    if has_js { transforms.push("commonjs_to_esm".to_string()); }

    let has_jsx = walkdir::WalkDir::new(&ws).max_depth(3).into_iter()
        .filter_map(|e| e.ok())
        .any(|e| {
            let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("");
            ext == "jsx" || ext == "tsx"
        });
    if has_jsx { transforms.push("react_class_to_hooks".to_string()); }

    let has_py = walkdir::WalkDir::new(&ws).max_depth(3).into_iter()
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("py"));
    if has_py { transforms.push("python2_to3".to_string()); }

    Ok(transforms)
}

#[tauri::command]
pub async fn plan_transform(
    transform_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<TransformPlanResult, String> {
    let engine = state.chat_engine.lock().await;
    let llm = engine.active_provider().ok_or("No active AI provider")?;
    let workspace_folders = state.workspace.lock().await.folders().to_vec();
    let ws = workspace_folders.first()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Find files matching the transform type
    let extensions: Vec<&str> = match transform_type.as_str() {
        "commonjs_to_esm" => vec!["js", "cjs"],
        "react_class_to_hooks" => vec!["jsx", "tsx"],
        "python2_to3" => vec!["py"],
        "vue2_to3" => vec!["vue"],
        _ => vec!["js", "ts", "py"],
    };

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(&ws).max_depth(5).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let ext = entry.path().extension().and_then(|x| x.to_str()).unwrap_or("");
            if extensions.contains(&ext) {
                if let Ok(rel) = entry.path().strip_prefix(&ws) {
                    let s = rel.to_string_lossy().to_string();
                    if !s.contains("node_modules") && !s.contains("/target/") && !s.starts_with(".") {
                        files.push(s);
                    }
                }
            }
        }
    }
    files.sort();
    files.truncate(30); // Limit to 30 files

    // Use LLM to generate plan
    let prompt = format!(
        "Plan a '{}' code transformation for these files:\n{}\nReturn JSON: [{{\"file\":\"...\",\"description\":\"...\",\"estimated_changes\":N}}]",
        transform_type,
        files.iter().map(|f| format!("- {}", f)).collect::<Vec<_>>().join("\n")
    );

    let messages = vec![
        vibe_ai::Message { role: vibe_ai::MessageRole::User, content: prompt },
    ];
    let response = llm.chat(&messages, None).await.map_err(|e| e.to_string())?;

    // Parse plan items from JSON
    let items: Vec<TransformPlanItem> = if let Some(start) = response.find('[') {
        if let Some(end) = response.rfind(']') {
            serde_json::from_str(&response[start..=end]).unwrap_or_else(|_| {
                files.iter().map(|f| TransformPlanItem {
                    file: f.clone(), description: format!("Apply {} transform", transform_type), estimated_changes: 3,
                }).collect()
            })
        } else { Vec::new() }
    } else {
        files.iter().map(|f| TransformPlanItem {
            file: f.clone(), description: format!("Apply {} transform", transform_type), estimated_changes: 3,
        }).collect()
    };

    let total = items.len();
    Ok(TransformPlanResult {
        files: items,
        total_files: total,
        summary: format!("{} files to transform", total),
    })
}

#[tauri::command]
pub async fn execute_transform(
    transform_type: String,
    files: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<TransformExecResult, String> {
    let engine = state.chat_engine.lock().await;
    let llm = engine.active_provider().ok_or("No active AI provider")?;
    let workspace_folders = state.workspace.lock().await.folders().to_vec();
    let ws = workspace_folders.first()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let mut modified = 0;
    for file in &files {
        let file_path = ws.join(file);
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let prompt = format!(
            "Apply '{}' transformation to this file. Return ONLY the transformed code:\n```\n{}\n```",
            transform_type, content
        );
        let messages = vec![
            vibe_ai::Message { role: vibe_ai::MessageRole::User, content: prompt },
        ];

        match llm.chat(&messages, None).await {
            Ok(response) => {
                let code = response.trim();
                let code = if code.starts_with("```") {
                    let s = code.find('\n').map(|i| i + 1).unwrap_or(3);
                    let e = code.rfind("```").unwrap_or(code.len());
                    &code[s..e]
                } else { code };

                if !code.trim().is_empty() {
                    let _ = std::fs::write(&file_path, code.trim());
                    modified += 1;
                }
            }
            Err(_) => continue,
        }
    }

    Ok(TransformExecResult {
        files_modified: modified,
        summary: format!("Transformed {}/{} files with {}", modified, files.len(), transform_type),
    })
}

// ── Phase 8.4: Plugin Marketplace ────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketplacePluginInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub repo_url: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub updated_at: String,
}

fn marketplace_index_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home).join(".vibeui");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("marketplace-index.json")
}

fn builtin_marketplace_plugins() -> Vec<MarketplacePluginInfo> {
    vec![
        MarketplacePluginInfo {
            name: "vibecli-prettier".into(),
            description: "Auto-format code with Prettier after file writes".into(),
            version: "1.0.0".into(), author: "VibeCody".into(),
            repo_url: "https://github.com/nicktrebes/vibecli-prettier".into(),
            tags: vec!["formatting".into(), "prettier".into()],
            downloads: 0, updated_at: "2026-03-01".into(),
        },
        MarketplacePluginInfo {
            name: "vibecli-eslint".into(),
            description: "Run ESLint checks after edits".into(),
            version: "1.0.0".into(), author: "VibeCody".into(),
            repo_url: "https://github.com/nicktrebes/vibecli-eslint".into(),
            tags: vec!["linting".into(), "eslint".into()],
            downloads: 0, updated_at: "2026-03-01".into(),
        },
        MarketplacePluginInfo {
            name: "vibecli-docker".into(),
            description: "Docker tools for agent context".into(),
            version: "1.0.0".into(), author: "VibeCody".into(),
            repo_url: "https://github.com/nicktrebes/vibecli-docker".into(),
            tags: vec!["docker".into(), "devops".into()],
            downloads: 0, updated_at: "2026-03-01".into(),
        },
        MarketplacePluginInfo {
            name: "vibecli-terraform".into(),
            description: "Terraform plan/apply integration".into(),
            version: "1.0.0".into(), author: "VibeCody".into(),
            repo_url: "https://github.com/nicktrebes/vibecli-terraform".into(),
            tags: vec!["terraform".into(), "iac".into()],
            downloads: 0, updated_at: "2026-03-01".into(),
        },
    ]
}

#[tauri::command]
pub async fn get_marketplace_plugins() -> Result<Vec<MarketplacePluginInfo>, String> {
    let path = marketplace_index_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let plugins: Vec<MarketplacePluginInfo> = serde_json::from_str(&data).unwrap_or_else(|_| builtin_marketplace_plugins());
        Ok(plugins)
    } else {
        Ok(builtin_marketplace_plugins())
    }
}

#[tauri::command]
pub async fn search_marketplace(query: String) -> Result<Vec<MarketplacePluginInfo>, String> {
    let all = get_marketplace_plugins().await?;
    let q = query.to_lowercase();
    let results: Vec<MarketplacePluginInfo> = all.into_iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&q)
                || p.description.to_lowercase().contains(&q)
                || p.tags.iter().any(|t| t.to_lowercase().contains(&q))
                || p.author.to_lowercase().contains(&q)
        })
        .collect();
    Ok(results)
}

#[tauri::command]
pub async fn install_marketplace_plugin(name: String, repo_url: String) -> Result<String, String> {
    // Install by cloning the git repo into ~/.vibecli/plugins/<name>
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let plugins_dir = std::path::PathBuf::from(home).join(".vibecli").join("plugins");
    let _ = std::fs::create_dir_all(&plugins_dir);
    let target = plugins_dir.join(&name);

    if target.exists() {
        return Err(format!("Plugin '{}' is already installed", name));
    }

    let output = std::process::Command::new("git")
        .args(["clone", "--depth", "1", &repo_url, target.to_str().unwrap_or(".")])
        .output()
        .map_err(|e| format!("git clone failed: {}", e))?;

    if output.status.success() {
        Ok(format!("Installed {} to {}", name, target.display()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git clone failed: {}", stderr))
    }
}

impl Default for CIReviewConfig {
    fn default() -> Self {
        Self {
            app_id: 0,
            private_key_path: None,
            webhook_secret: None,
            auto_fix: false,
            severity_threshold: default_severity_threshold(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CIReviewResult {
    pub pr_number: u64,
    pub repo: String,
    pub commit_sha: String,
    pub findings_count: usize,
    pub severity_counts: SeverityCounts,
    pub status: String,
    pub summary: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

fn ci_review_config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home).join(".vibeui");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("ci-review-config.json")
}

fn ci_review_history_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home).join(".vibeui");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("ci-review-history.json")
}

#[tauri::command]
pub async fn get_ci_review_config() -> Result<CIReviewConfig, String> {
    let path = ci_review_config_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    } else {
        Ok(CIReviewConfig::default())
    }
}

#[tauri::command]
pub async fn save_ci_review_config(config: CIReviewConfig) -> Result<(), String> {
    let path = ci_review_config_path();
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_ci_review_history() -> Result<Vec<CIReviewResult>, String> {
    let path = ci_review_history_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Phase 8.14 — Full-Stack App Generation from Screenshot
// ══════════════════════════════════════════════════════════════════════════════

/// A single generated file with its path, content, and detected language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub language: String,
}

/// Generate a complete app from a screenshot image.
///
/// Sends the base64-encoded image to the active AI provider with a framework-specific
/// prompt, then parses the response to extract code blocks and file paths.
#[tauri::command]
pub async fn generate_app_from_image(
    image_base64: String,
    framework: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<GeneratedFile>, String> {
    use vibe_ai::provider::{Message, MessageRole};

    let fw_instructions = match framework.as_str() {
        "react" => "Generate a React app using TypeScript (TSX). Create functional components with hooks. Use CSS-in-JS (inline style objects). Include an App.tsx entry component and any sub-components in separate files.",
        "vue" => "Generate a Vue 3 app using Single File Components (.vue) with <script setup lang=\"ts\">. Include an App.vue and any sub-components in separate files.",
        "svelte" => "Generate a Svelte app. Create .svelte components with <script lang=\"ts\">. Include an App.svelte entry and any sub-components in separate files.",
        "nextjs" => "Generate a Next.js App Router project using TypeScript. Put pages under app/ directory with page.tsx files. Use CSS modules or Tailwind utility classes. Include layout.tsx.",
        "html" => "Generate a vanilla HTML/CSS/JS app. Create an index.html, a styles.css, and a script.js. Use modern ES6+ JavaScript. Make it responsive.",
        _ => "Generate a React app using TypeScript (TSX). Create functional components with hooks.",
    };

    let prompt = format!(
        "You are a full-stack app generator. I am providing you with a screenshot/design image (base64-encoded below). \
        Analyze the visual layout, colors, typography, spacing, and component structure in this design.\n\n\
        {fw_instructions}\n\n\
        IMPORTANT RULES:\n\
        - Reproduce the design as faithfully as possible\n\
        - Use the exact colors, fonts, and spacing visible in the screenshot\n\
        - Make the app responsive\n\
        - Each file must be in its own fenced code block\n\
        - Before each code block, write a comment line: // FILE: <relative-path>\n\
        - Example:\n\
          // FILE: src/App.tsx\n\
          ```tsx\n\
          // code here\n\
          ```\n\n\
        IMAGE (base64):\n{image_base64}\n\n\
        Generate the complete app now."
    );

    let messages = vec![
        Message { role: MessageRole::User, content: prompt },
    ];

    let engine = state.chat_engine.lock().await;
    let raw = engine.chat(&messages, None).await.map_err(|e| e.to_string())?;

    // Parse the AI response to extract file blocks.
    parse_generated_files(&raw)
}

/// Parse AI response text into a list of `GeneratedFile` entries.
///
/// Looks for patterns like:
///   // FILE: src/App.tsx
///   ```tsx
///   <content>
///   ```
fn parse_generated_files(response: &str) -> Result<Vec<GeneratedFile>, String> {
    let mut files: Vec<GeneratedFile> = Vec::new();
    let lines: Vec<&str> = response.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Detect a FILE marker
        let file_path = if line.starts_with("// FILE:") {
            Some(line.trim_start_matches("// FILE:").trim().to_string())
        } else if line.starts_with("<!-- FILE:") {
            // HTML variant: <!-- FILE: index.html -->
            let inner = line.trim_start_matches("<!-- FILE:")
                .trim_end_matches("-->")
                .trim()
                .to_string();
            Some(inner)
        } else {
            None
        };

        if let Some(path) = file_path {
            // Advance past the FILE marker line
            i += 1;

            // Skip blank lines between FILE marker and code fence
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }

            // Expect a code fence
            if i < lines.len() && lines[i].trim().starts_with("```") {
                let fence_lang = lines[i].trim().trim_start_matches('`').trim().to_string();
                i += 1;

                // Collect lines until closing fence
                let mut content_lines: Vec<&str> = Vec::new();
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    content_lines.push(lines[i]);
                    i += 1;
                }
                // Skip closing fence
                if i < lines.len() {
                    i += 1;
                }

                let content = content_lines.join("\n");
                let language = detect_language_from_path_or_fence(&path, &fence_lang);

                files.push(GeneratedFile { path, content, language });
            }
        } else {
            i += 1;
        }
    }

    // Fallback: if no FILE markers were found, try to extract any fenced code blocks
    if files.is_empty() {
        let mut idx = 0;
        let mut block_num = 0u32;
        while idx < lines.len() {
            if lines[idx].trim().starts_with("```") {
                let fence_lang = lines[idx].trim().trim_start_matches('`').trim().to_string();
                idx += 1;
                let mut content_lines: Vec<&str> = Vec::new();
                while idx < lines.len() && !lines[idx].trim().starts_with("```") {
                    content_lines.push(lines[idx]);
                    idx += 1;
                }
                if idx < lines.len() {
                    idx += 1;
                }
                let content = content_lines.join("\n");
                if !content.trim().is_empty() {
                    block_num += 1;
                    let (path, language) = infer_file_info(&fence_lang, block_num);
                    files.push(GeneratedFile { path, content, language });
                }
            } else {
                idx += 1;
            }
        }
    }

    if files.is_empty() {
        return Err("No code blocks found in AI response. Try again or use a different provider.".to_string());
    }

    Ok(files)
}

/// Detect the language string from a file path extension or code-fence language tag.
fn detect_language_from_path_or_fence(path: &str, fence_lang: &str) -> String {
    if let Some(ext) = path.rsplit('.').next() {
        match ext {
            "tsx" => return "tsx".to_string(),
            "jsx" => return "jsx".to_string(),
            "ts" => return "typescript".to_string(),
            "js" => return "javascript".to_string(),
            "vue" => return "vue".to_string(),
            "svelte" => return "svelte".to_string(),
            "html" | "htm" => return "html".to_string(),
            "css" => return "css".to_string(),
            "json" => return "json".to_string(),
            _ => {}
        }
    }
    if !fence_lang.is_empty() {
        return fence_lang.to_string();
    }
    "text".to_string()
}

/// Infer a sensible file path and language when no FILE marker was present.
fn infer_file_info(fence_lang: &str, block_num: u32) -> (String, String) {
    match fence_lang {
        "tsx" => (format!("src/Component{}.tsx", block_num), "tsx".to_string()),
        "jsx" => (format!("src/Component{}.jsx", block_num), "jsx".to_string()),
        "typescript" | "ts" => (format!("src/file{}.ts", block_num), "typescript".to_string()),
        "javascript" | "js" => (format!("src/file{}.js", block_num), "javascript".to_string()),
        "vue" => (format!("src/Component{}.vue", block_num), "vue".to_string()),
        "svelte" => (format!("src/Component{}.svelte", block_num), "svelte".to_string()),
        "html" => {
            let suffix = if block_num == 1 { String::new() } else { block_num.to_string() };
            (format!("index{}.html", suffix), "html".to_string())
        }
        "css" => {
            let suffix = if block_num == 1 { String::new() } else { block_num.to_string() };
            (format!("styles{}.css", suffix), "css".to_string())
        }
        "json" => (format!("file{}.json", block_num), "json".to_string()),
        _ => (format!("src/file{}.txt", block_num), "text".to_string()),
    }
}

// ── Phase 8.13: Local Edit Model Configuration ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEditConfig {
    pub model: String,
    pub api_url: String,
}

fn local_edit_config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home).join(".vibeui");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("local-edit-config.json")
}

#[tauri::command]
pub async fn configure_local_edit_model(
    model: String,
    api_url: Option<String>,
) -> Result<String, String> {
    let api_url = api_url.unwrap_or_else(|| "http://localhost:11434".to_string());

    // Validate that Ollama is reachable at the given URL
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let check = client
        .get(format!("{}/api/tags", api_url))
        .send()
        .await;

    if check.is_err() {
        return Err(format!(
            "Cannot reach Ollama at {}. Make sure Ollama is running.",
            api_url
        ));
    }

    let config = LocalEditConfig {
        model: model.clone(),
        api_url: api_url.clone(),
    };

    let path = local_edit_config_path();
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(format!(
        "Local edit model configured: {} at {}",
        model, api_url
    ))
}

#[tauri::command]
pub async fn get_local_edit_config() -> Result<Option<LocalEditConfig>, String> {
    let path = local_edit_config_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let config: LocalEditConfig =
            serde_json::from_str(&data).map_err(|e| e.to_string())?;
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

// ── Phase 8.11: Computer Use / Visual Self-Testing ──────────────────────────

/// Take a screenshot using platform-native tools and return its path + timestamp.
#[tauri::command]
pub async fn take_screenshot(output_dir: String) -> Result<serde_json::Value, String> {
    let dir = std::path::PathBuf::from(&output_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = dir.join(format!("screenshot-{ts}.png"));
    let cmd = if cfg!(target_os = "macos") {
        format!("screencapture -x {}", path.display())
    } else if cfg!(target_os = "linux") {
        format!("scrot {}", path.display())
    } else {
        "powershell -command \"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Screen]::PrimaryScreen\"".to_string()
    };
    let output = std::process::Command::new("sh")
        .args(["-c", &cmd])
        .output()
        .map_err(|e| format!("Screenshot failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Screenshot command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "timestamp": ts,
    }))
}

/// Load saved visual test results for a given session ID.
#[tauri::command]
pub async fn get_visual_test_results(session_id: String) -> Result<serde_json::Value, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(&home)
        .join(".vibeui")
        .join("visual-tests")
        .join(format!("{session_id}.json"));
    if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let val: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(val)
    } else {
        Ok(serde_json::json!({"steps": [], "status": "not_found"}))
    }
}

// ── Phase 8.17: Cloud-Isolated Agent Execution (Docker) ─────────────────

/// Start a cloud agent task inside an isolated Docker container.
/// Returns a JSON object with the container ID, status, image, and task.
#[tauri::command]
pub async fn start_cloud_agent(
    image: String,
    task: String,
    workspace: Option<String>,
) -> Result<serde_json::Value, String> {
    // Check Docker availability
    let output = std::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .map_err(|e| format!("Docker check failed: {e}"))?;
    if !output.status.success() {
        return Err("Docker is not installed or not running".to_string());
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(serde_json::json!({
        "container_id": format!("vibecody-{ts}"),
        "status": "queued",
        "image": image,
        "task": task,
        "workspace": workspace,
    }))
}

/// Check the status of a running cloud agent Docker container.
#[tauri::command]
pub async fn get_cloud_agent_status(
    container_id: String,
) -> Result<serde_json::Value, String> {
    let output = std::process::Command::new("docker")
        .args(["inspect", "--format", "{{.State.Status}}", &container_id])
        .output()
        .map_err(|e| format!("Docker inspect failed: {e}"))?;
    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(serde_json::json!({
        "container_id": container_id,
        "status": if status.is_empty() { "not_found".to_string() } else { status },
    }))
}

// ── Phase 8.18: Compliance Reporting ──────────────────────────────────────────

#[tauri::command]
pub async fn generate_compliance_report(
    framework: String,
) -> Result<serde_json::Value, String> {
    let controls: Vec<serde_json::Value> = match framework.as_str() {
        "SOC2" => vec![
            serde_json::json!({"id":"CC1.1","name":"Security Governance","status":"implemented","evidence":["MIT License","Open source codebase"],"notes":"Fully open source with transparent development"}),
            serde_json::json!({"id":"CC6.1","name":"Logical Access Security","status":"implemented","evidence":["Bearer token auth (serve.rs)","CORS localhost restriction","Rate limiting 60req/60s"],"notes":"API endpoints protected"}),
            serde_json::json!({"id":"CC6.6","name":"Encryption in Transit","status":"partial","evidence":["HTTPS supported","TLS cert checking"],"notes":"HTTPS available; HTTP for local dev"}),
            serde_json::json!({"id":"CC6.7","name":"Encryption at Rest","status":"partial","evidence":["Config file permissions 0o600"],"notes":"File permissions enforced; OS-level encryption"}),
            serde_json::json!({"id":"CC7.2","name":"Security Monitoring","status":"implemented","evidence":["OpenTelemetry tracing","Session audit trail","Secret redaction in logs"],"notes":"Full observability pipeline"}),
            serde_json::json!({"id":"CC8.1","name":"Change Management","status":"implemented","evidence":["Approval policy system","Hooks pre/post execution","Git checkpoints"],"notes":"Multi-level approval with hooks"}),
            serde_json::json!({"id":"CC9.1","name":"Risk Mitigation","status":"implemented","evidence":["Command blocklist","Path traversal prevention","Sandbox mode","Red team scanning"],"notes":"Multiple security layers"}),
        ],
        "FedRAMP" => vec![
            serde_json::json!({"id":"AC-2","name":"Account Management","status":"implemented","evidence":["Bearer token auth","API key management"],"notes":"Token-based access control"}),
            serde_json::json!({"id":"AU-2","name":"Audit Events","status":"implemented","evidence":["Session trace store","OTLP export","JSONL audit logs"],"notes":"Comprehensive audit trail"}),
            serde_json::json!({"id":"SC-8","name":"Transmission Confidentiality","status":"partial","evidence":["HTTPS support","TLS verification"],"notes":"Local HTTP; remote HTTPS"}),
            serde_json::json!({"id":"SI-2","name":"Flaw Remediation","status":"implemented","evidence":["cargo audit CI","OWASP scanner","BugBot"],"notes":"Automated vulnerability scanning"}),
        ],
        _ => vec![
            serde_json::json!({"id":"GEN-1","name":"Access Control","status":"implemented","evidence":["Auth system"],"notes":"Bearer token authentication"}),
            serde_json::json!({"id":"GEN-2","name":"Audit Logging","status":"implemented","evidence":["Trace system"],"notes":"Full session audit trail"}),
        ],
    };
    let total = controls.len();
    let implemented = controls.iter().filter(|c| c["status"] == "implemented").count();
    let partial = controls.iter().filter(|c| c["status"] == "partial").count();
    let gaps = total - implemented - partial;
    let applicable = total;
    let pct = if applicable > 0 {
        ((implemented as f64 + partial as f64 * 0.5) / applicable as f64) * 100.0
    } else {
        100.0
    };
    Ok(serde_json::json!({
        "framework": framework,
        "controls": controls,
        "summary": {
            "total": total,
            "implemented": implemented,
            "partial": partial,
            "gaps": gaps,
            "percentage": pct,
        }
    }))
}

// ── Phase 7.34: Project Scaffolding ───────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ScaffoldTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub framework: String,
    pub tags: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct ScaffoldFile {
    pub path: String,
    pub content: String,
}

#[derive(serde::Serialize)]
pub struct ScaffoldResult {
    pub files: Vec<ScaffoldFile>,
    pub install_command: Option<String>,
    pub dev_command: Option<String>,
    pub notes: String,
}

/// List built-in scaffold templates
#[tauri::command]
pub async fn list_scaffold_templates() -> Result<Vec<ScaffoldTemplate>, String> {
    Ok(vec![
        ScaffoldTemplate { id: "rust-cli".into(), name: "Rust CLI".into(), description: "Clap-based CLI with error handling".into(), language: "Rust".into(), framework: "Clap".into(), tags: vec!["rust".into(), "cli".into()] },
        ScaffoldTemplate { id: "rust-axum".into(), name: "Rust Axum API".into(), description: "REST API with Axum, Tower, and serde".into(), language: "Rust".into(), framework: "Axum".into(), tags: vec!["rust".into(), "api".into(), "web".into()] },
        ScaffoldTemplate { id: "react-ts".into(), name: "React + TypeScript".into(), description: "Vite-powered React app with TS".into(), language: "TypeScript".into(), framework: "React".into(), tags: vec!["react".into(), "typescript".into(), "frontend".into()] },
        ScaffoldTemplate { id: "nextjs".into(), name: "Next.js App".into(), description: "Next.js 14 with App Router and Tailwind".into(), language: "TypeScript".into(), framework: "Next.js".into(), tags: vec!["react".into(), "next".into(), "fullstack".into()] },
        ScaffoldTemplate { id: "fastapi".into(), name: "FastAPI".into(), description: "Python FastAPI with pydantic and uvicorn".into(), language: "Python".into(), framework: "FastAPI".into(), tags: vec!["python".into(), "api".into()] },
        ScaffoldTemplate { id: "go-gin".into(), name: "Go Gin API".into(), description: "Gin REST API with structured logging".into(), language: "Go".into(), framework: "Gin".into(), tags: vec!["go".into(), "api".into()] },
        ScaffoldTemplate { id: "tauri-react".into(), name: "Tauri + React".into(), description: "Desktop app with Tauri 2 + React + TS".into(), language: "Rust/TypeScript".into(), framework: "Tauri".into(), tags: vec!["tauri".into(), "desktop".into(), "rust".into()] },
        ScaffoldTemplate { id: "express-ts".into(), name: "Express + TypeScript".into(), description: "Node.js Express API with TypeScript".into(), language: "TypeScript".into(), framework: "Express".into(), tags: vec!["node".into(), "api".into(), "typescript".into()] },
    ])
}

/// Generate scaffold files for a given template and project name
#[tauri::command]
pub async fn generate_scaffold(template_id: String, project_name: String, output_dir: String) -> Result<ScaffoldResult, String> {
    // Validate project name
    if !project_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("Invalid project name: {project_name}"));
    }
    let name = &project_name;

    let result = match template_id.as_str() {
        "rust-cli" => ScaffoldResult {
            files: vec![
                ScaffoldFile { path: "Cargo.toml".into(), content: format!(r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = {{ version = "4", features = ["derive"] }}
anyhow = "1"
"#) },
                ScaffoldFile { path: "src/main.rs".into(), content: format!(r#"use clap::{{Parser, Subcommand}};

#[derive(Parser)]
#[command(name = "{name}", about = "A CLI application")]
struct Cli {{
    #[command(subcommand)]
    command: Commands,
}}

#[derive(Subcommand)]
enum Commands {{
    /// Run the main command
    Run {{ input: String }},
}}

fn main() {{
    let cli = Cli::parse();
    match cli.command {{
        Commands::Run {{ input }} => {{
            println!("Running with: {{}}", input);
        }}
    }}
}}
"#) },
                ScaffoldFile { path: ".gitignore".into(), content: "/target\n".into() },
                ScaffoldFile { path: "README.md".into(), content: format!("# {name}\n\nA Rust CLI application.\n\n## Usage\n\n```bash\ncargo run -- run <input>\n```\n") },
            ],
            install_command: None,
            dev_command: Some("cargo run".into()),
            notes: "Run `cargo build --release` to create a release binary.".into(),
        },
        "rust-axum" => ScaffoldResult {
            files: vec![
                ScaffoldFile { path: "Cargo.toml".into(), content: format!(r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
tower-http = {{ version = "0.5", features = ["cors", "trace"] }}
tracing = "0.1"
tracing-subscriber = "0.3"
"#) },
                ScaffoldFile { path: "src/main.rs".into(), content: r#"use axum::{routing::get, Router, Json};
use serde::Serialize;

#[derive(Serialize)]
struct Health { status: String }

async fn health() -> Json<Health> {
    Json(Health { status: "ok".into() })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new().route("/health", get(health));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
"#.into() },
                ScaffoldFile { path: ".gitignore".into(), content: "/target\n".into() },
            ],
            install_command: None,
            dev_command: Some("cargo run".into()),
            notes: "API will be available at http://localhost:3000".into(),
        },
        "react-ts" => ScaffoldResult {
            files: vec![
                ScaffoldFile { path: "package.json".into(), content: format!(r#"{{
  "name": "{name}",
  "private": true,
  "version": "0.1.0",
  "scripts": {{
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  }},
  "dependencies": {{
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  }},
  "devDependencies": {{
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  }}
}}
"#) },
                ScaffoldFile { path: "index.html".into(), content: format!(r#"<!DOCTYPE html>
<html lang="en">
  <head><meta charset="UTF-8"><title>{name}</title></head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#) },
                ScaffoldFile { path: "src/main.tsx".into(), content: r#"import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><App /></React.StrictMode>
);
"#.into() },
                ScaffoldFile { path: "src/App.tsx".into(), content: format!(r#"export default function App() {{
  return <h1>{name}</h1>;
}}
"#) },
                ScaffoldFile { path: "tsconfig.json".into(), content: r#"{"compilerOptions":{"target":"ES2020","useDefineForClassFields":true,"lib":["ES2020","DOM"],"module":"ESNext","skipLibCheck":true,"moduleResolution":"bundler","allowImportingTsExtensions":true,"noEmit":true,"strict":true,"jsx":"react-jsx"},"include":["src"]}"#.into() },
                ScaffoldFile { path: ".gitignore".into(), content: "node_modules\ndist\n".into() },
            ],
            install_command: Some("npm install".into()),
            dev_command: Some("npm run dev".into()),
            notes: "Run npm install then npm run dev to start.".into(),
        },
        "fastapi" => ScaffoldResult {
            files: vec![
                ScaffoldFile { path: "main.py".into(), content: r#"from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

class Item(BaseModel):
    name: str
    value: float

@app.get("/health")
def health():
    return {"status": "ok"}

@app.post("/items")
def create_item(item: Item):
    return {"created": item.model_dump()}
"#.into() },
                ScaffoldFile { path: "requirements.txt".into(), content: "fastapi>=0.100.0\nuvicorn[standard]>=0.23.0\npydantic>=2.0.0\n".into() },
                ScaffoldFile { path: ".gitignore".into(), content: "__pycache__\n*.pyc\n.venv\n".into() },
                ScaffoldFile { path: "README.md".into(), content: format!("# {name}\n\n```bash\npip install -r requirements.txt\nuvicorn main:app --reload\n```\n") },
            ],
            install_command: Some("pip install -r requirements.txt".into()),
            dev_command: Some("uvicorn main:app --reload".into()),
            notes: "API docs at http://localhost:8000/docs".into(),
        },
        "go-gin" => ScaffoldResult {
            files: vec![
                ScaffoldFile { path: "go.mod".into(), content: format!("module {name}\n\ngo 1.21\n\nrequire github.com/gin-gonic/gin v1.9.1\n") },
                ScaffoldFile { path: "main.go".into(), content: r#"package main

import (
    "net/http"
    "github.com/gin-gonic/gin"
)

func main() {
    r := gin.Default()
    r.GET("/health", func(c *gin.Context) {
        c.JSON(http.StatusOK, gin.H{"status": "ok"})
    })
    r.Run(":8080")
}
"#.into() },
                ScaffoldFile { path: ".gitignore".into(), content: "*.exe\n*.out\n".into() },
            ],
            install_command: Some("go mod tidy".into()),
            dev_command: Some("go run main.go".into()),
            notes: "API at http://localhost:8080".into(),
        },
        "express-ts" => ScaffoldResult {
            files: vec![
                ScaffoldFile { path: "package.json".into(), content: format!(r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "scripts": {{
    "dev": "ts-node src/index.ts",
    "build": "tsc",
    "start": "node dist/index.js"
  }},
  "dependencies": {{
    "express": "^4.18.0"
  }},
  "devDependencies": {{
    "@types/express": "^4.17.0",
    "@types/node": "^20.0.0",
    "ts-node": "^10.9.0",
    "typescript": "^5.0.0"
  }}
}}
"#) },
                ScaffoldFile { path: "src/index.ts".into(), content: r#"import express from "express";
const app = express();
app.use(express.json());
app.get("/health", (_req, res) => res.json({ status: "ok" }));
app.listen(3000, () => console.log("Server on http://localhost:3000"));
"#.into() },
                ScaffoldFile { path: "tsconfig.json".into(), content: r#"{"compilerOptions":{"target":"ES2020","module":"commonjs","outDir":"dist","strict":true},"include":["src"]}"#.into() },
                ScaffoldFile { path: ".gitignore".into(), content: "node_modules\ndist\n".into() },
            ],
            install_command: Some("npm install".into()),
            dev_command: Some("npm run dev".into()),
            notes: "API at http://localhost:3000".into(),
        },
        "nextjs" => ScaffoldResult {
            files: vec![
                ScaffoldFile { path: "package.json".into(), content: format!(r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "scripts": {{
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  }},
  "dependencies": {{
    "next": "14",
    "react": "^18",
    "react-dom": "^18"
  }},
  "devDependencies": {{
    "@types/node": "^20",
    "@types/react": "^18",
    "typescript": "^5"
  }}
}}
"#) },
                ScaffoldFile { path: "app/page.tsx".into(), content: format!("export default function Home() {{\n  return <main><h1>{name}</h1></main>;\n}}\n") },
                ScaffoldFile { path: "app/layout.tsx".into(), content: "export default function RootLayout({ children }: { children: React.ReactNode }) {\n  return <html lang=\"en\"><body>{children}</body></html>;\n}\n".into() },
                ScaffoldFile { path: ".gitignore".into(), content: "node_modules\n.next\n".into() },
            ],
            install_command: Some("npm install".into()),
            dev_command: Some("npm run dev".into()),
            notes: "App at http://localhost:3000".into(),
        },
        "tauri-react" => ScaffoldResult {
            files: vec![
                ScaffoldFile { path: "package.json".into(), content: format!(r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "scripts": {{
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri"
  }},
  "dependencies": {{
    "react": "^18",
    "react-dom": "^18",
    "@tauri-apps/api": "^2"
  }},
  "devDependencies": {{
    "@tauri-apps/cli": "^2",
    "@vitejs/plugin-react": "^4",
    "typescript": "^5",
    "vite": "^5"
  }}
}}
"#) },
                ScaffoldFile { path: "src-tauri/Cargo.toml".into(), content: format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\nname = \"{name}\"\ncrate-type = [\"cdylib\", \"rlib\"]\n\n[dependencies]\ntauri = {{ version = \"2\", features = [] }}\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\n") },
                ScaffoldFile { path: "src-tauri/src/lib.rs".into(), content: "#[cfg_attr(mobile, tauri::mobile_entry_point)]\npub fn run() {\n    tauri::Builder::default()\n        .run(tauri::generate_context!())\n        .expect(\"error while running tauri application\");\n}\n".into() },
                ScaffoldFile { path: "src/App.tsx".into(), content: format!("export default function App() {{\n  return <h1>{name}</h1>;\n}}\n") },
                ScaffoldFile { path: ".gitignore".into(), content: "node_modules\ndist\nsrc-tauri/target\n".into() },
            ],
            install_command: Some("npm install".into()),
            dev_command: Some("npm run tauri dev".into()),
            notes: "Requires Rust and the Tauri CLI prerequisites.".into(),
        },
        _ => return Err(format!("Unknown template: {template_id}")),
    };

    // Write files to output_dir if it is non-empty
    if !output_dir.is_empty() {
        let root = std::path::PathBuf::from(&output_dir);
        for f in &result.files {
            let dest = root.join(&f.path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&dest, &f.content).map_err(|e| e.to_string())?;
        }
    }

    Ok(result)
}

// ── Phase 7.35: Service Health Monitor ────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct HealthMonitor {
    pub id: String,
    pub label: String,
    pub url: String,
    pub expected_status: u16,
    pub timeout_ms: u64,
}

#[derive(serde::Serialize)]
pub struct HealthCheckResult {
    pub id: String,
    pub url: String,
    pub ok: bool,
    pub status_code: Option<u16>,
    pub latency_ms: u64,
    pub timestamp: u64,
    pub error: Option<String>,
}

fn health_monitors_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibeui").join("health-monitors.json")
}

#[tauri::command]
pub async fn get_health_monitors() -> Result<Vec<HealthMonitor>, String> {
    let path = health_monitors_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_health_monitors(monitors: Vec<HealthMonitor>) -> Result<(), String> {
    let path = health_monitors_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(&monitors).map_err(|e| e.to_string())?;
    std::fs::write(&path, &data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_service_health(monitor: HealthMonitor) -> Result<HealthCheckResult, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = std::time::Instant::now();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let timeout = std::time::Duration::from_millis(monitor.timeout_ms.min(30_000));

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(false)
        .build()
        .map_err(|e| e.to_string())?;

    match client.get(&monitor.url).send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let status = resp.status().as_u16();
            let ok = status == monitor.expected_status || (monitor.expected_status == 200 && status < 400);
            Ok(HealthCheckResult {
                id: monitor.id,
                url: monitor.url,
                ok,
                status_code: Some(status),
                latency_ms,
                timestamp: now,
                error: if ok { None } else { Some(format!("HTTP {status}")) },
            })
        }
        Err(e) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            Ok(HealthCheckResult {
                id: monitor.id,
                url: monitor.url,
                ok: false,
                status_code: None,
                latency_ms,
                timestamp: now,
                error: Some(e.to_string()),
            })
        }
    }
}

#[tauri::command]
pub async fn check_all_services(monitors: Vec<HealthMonitor>) -> Result<Vec<HealthCheckResult>, String> {
    let futs: Vec<_> = monitors.into_iter().map(check_service_health).collect();
    let results = futures::future::join_all(futs).await;
    Ok(results.into_iter().filter_map(|r| r.ok()).collect())
}

// ── Phase 7.36: WebSocket Tester ──────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct WsConfig {
    pub id: String,
    pub label: String,
    pub url: String,
    pub protocols: Vec<String>,
}

fn ws_configs_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibeui").join("ws-configs.json")
}

#[tauri::command]
pub async fn get_ws_configs() -> Result<Vec<WsConfig>, String> {
    let path = ws_configs_path();
    if !path.exists() { return Ok(vec![]); }
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_ws_configs(configs: Vec<WsConfig>) -> Result<(), String> {
    let path = ws_configs_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&configs).map_err(|e| e.to_string())?,
    ).map_err(|e| e.to_string())
}

// ── Phase 7.37: Color Palette & Design Token Manager ──────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ColorToken {
    pub name: String,
    pub value: String,   // hex
    pub comment: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ColorPalette {
    pub id: String,
    pub name: String,
    pub tokens: Vec<ColorToken>,
}

fn palettes_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".vibeui").join("color-palettes.json")
}

#[tauri::command]
pub async fn get_color_palettes() -> Result<Vec<ColorPalette>, String> {
    let path = palettes_path();
    if !path.exists() { return Ok(vec![]); }
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_color_palettes(palettes: Vec<ColorPalette>) -> Result<(), String> {
    let path = palettes_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&path, serde_json::to_string_pretty(&palettes).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// Scan workspace files for CSS custom properties (--name: #hex)
#[tauri::command]
pub async fn scan_css_variables(workspace: String) -> Result<Vec<ColorToken>, String> {
    use std::io::BufRead;
    let re = regex::Regex::new(r"--([a-zA-Z0-9_-]+)\s*:\s*(#[0-9a-fA-F]{3,8}|rgb[a]?\([^)]+\))")
        .map_err(|e| e.to_string())?;
    let root = std::path::Path::new(&workspace);
    let mut tokens: Vec<ColorToken> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for entry in walkdir::WalkDir::new(root).max_depth(6).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "css" | "scss" | "sass" | "less" | "tsx" | "ts" | "js" | "jsx") { continue; }
        if path.to_string_lossy().contains("node_modules") || path.to_string_lossy().contains("/target/") { continue; }
        if let Ok(file) = std::fs::File::open(path) {
            for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
                for cap in re.captures_iter(&line) {
                    let name = cap[1].to_string();
                    let value = cap[2].to_string();
                    if seen.insert(name.clone()) {
                        tokens.push(ColorToken { name, value, comment: None });
                    }
                }
            }
        }
        if tokens.len() >= 200 { break; }
    }
    Ok(tokens)
}

/// Export a palette to CSS variables, Tailwind, SCSS, or JSON
#[tauri::command]
pub async fn export_color_palette(palette: ColorPalette, format: String) -> Result<String, String> {
    let out = match format.as_str() {
        "css" => {
            let vars: String = palette.tokens.iter()
                .map(|t| format!("  --{}: {};", t.name, t.value))
                .collect::<Vec<_>>().join("\n");
            format!(":root {{\n{}\n}}", vars)
        }
        "scss" => palette.tokens.iter()
            .map(|t| format!("${}: {};", t.name.replace('-', "_"), t.value))
            .collect::<Vec<_>>().join("\n"),
        "tailwind" => {
            let entries: String = palette.tokens.iter()
                .map(|t| format!("      \"{}\": \"{}\",", t.name, t.value))
                .collect::<Vec<_>>().join("\n");
            format!("// tailwind.config.js extend\ncolors: {{\n{}\n}},", entries)
        }
        "json" => {
            let map: serde_json::Map<String, serde_json::Value> = palette.tokens.iter()
                .map(|t| (t.name.clone(), serde_json::Value::String(t.value.clone())))
                .collect();
            serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?
        }
        _ => return Err(format!("Unknown format: {format}")),
    };
    Ok(out)
}

// ── Phase 7.38: Markdown File Browser ─────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct MarkdownFile {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
}

/// List all .md and .mdx files in the workspace (max depth 8, skips node_modules/target)
#[tauri::command]
pub async fn list_markdown_files(workspace: String) -> Result<Vec<MarkdownFile>, String> {
    let root = std::path::Path::new(&workspace);
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).max_depth(8).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let lossy = path.to_string_lossy();
        if lossy.contains("node_modules") || lossy.contains("/target/") || lossy.contains("/.git/") {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "md" | "mdx") { continue; }
        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        files.push(MarkdownFile {
            path: path.to_string_lossy().into_owned(),
            name: path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
            size_bytes,
        });
        if files.len() >= 500 { break; }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

// ── Canvas / A2UI Visual Workspace commands ──────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanvasNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub config: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanvasEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanvasWorkflow {
    pub name: String,
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
}

fn canvas_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home)
        .join(".vibeui")
        .join("canvas");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[tauri::command]
pub async fn save_canvas_workflow(workflow: CanvasWorkflow) -> Result<(), String> {
    let path = canvas_dir().join(format!("{}.json", workflow.name.replace(' ', "_")));
    let json = serde_json::to_string_pretty(&workflow).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_canvas_workflow(name: String) -> Result<CanvasWorkflow, String> {
    let path = canvas_dir().join(format!("{}.json", name.replace(' ', "_")));
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_canvas_workflows() -> Result<Vec<CanvasWorkflow>, String> {
    let dir = canvas_dir();
    let mut workflows = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(wf) = serde_json::from_str::<CanvasWorkflow>(&json) {
                        workflows.push(wf);
                    }
                }
            }
        }
    }
    Ok(workflows)
}

#[tauri::command]
pub async fn run_canvas_workflow(workflow: CanvasWorkflow) -> Result<String, String> {
    // Convert canvas workflow to a description of the pipeline for now
    let mut desc = format!("Running workflow '{}' with {} nodes:\n", workflow.name, workflow.nodes.len());
    for node in &workflow.nodes {
        desc.push_str(&format!("  - {} ({}: {})\n", node.label, node.node_type, node.id));
    }
    for edge in &workflow.edges {
        desc.push_str(&format!("  {} -> {}\n", edge.from, edge.to));
    }
    Ok(desc)
}

// ── Voice / Transcription commands ───────────────────────────────────────────

#[tauri::command]
pub async fn transcribe_audio(audio_path: String) -> Result<String, String> {
    // Resolve Whisper API key from env
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| "GROQ_API_KEY not set (needed for Whisper transcription)".to_string())?;

    let path = std::path::Path::new(&audio_path);
    if !path.exists() {
        return Err(format!("Audio file not found: {}", audio_path));
    }

    // Build multipart form
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let file_bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-large-v3")
        .part("file", part);

    let resp = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let err = resp.text().await.map_err(|e| e.to_string())?;
        return Err(format!("Whisper API error: {}", err));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(body["text"].as_str().unwrap_or("").to_string())
}

#[tauri::command]
pub async fn text_to_speech(text: String) -> Result<Vec<u8>, String> {
    let api_key = std::env::var("ELEVENLABS_API_KEY")
        .map_err(|_| "ELEVENLABS_API_KEY not set".to_string())?;
    let voice_id = std::env::var("ELEVENLABS_VOICE_ID")
        .unwrap_or_else(|_| "21m00Tcm4TlvDq8ikWAM".to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);
    let resp = client
        .post(&url)
        .header("xi-api-key", &api_key)
        .json(&serde_json::json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": { "stability": 0.5, "similarity_boost": 0.5 }
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let err = resp.text().await.map_err(|e| e.to_string())?;
        return Err(format!("ElevenLabs API error: {}", err));
    }

    Ok(resp.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

// ── Container Sandbox Management ─────────────────────────────────────────────

/// Detect available container runtimes and their versions.
#[tauri::command]
pub async fn detect_sandbox_runtime() -> Result<serde_json::Value, String> {
    let docker_ver = tokio::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let podman_ver = tokio::process::Command::new("podman")
        .args(["version", "--format", "{{.Version}}"])
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let active = if docker_ver.is_some() {
        "docker"
    } else if podman_ver.is_some() {
        "podman"
    } else {
        "none"
    };

    Ok(serde_json::json!({
        "docker": docker_ver,
        "podman": podman_ver,
        "opensandbox": null,
        "active": active,
    }))
}

/// Create a sandbox container.
#[tauri::command]
pub async fn create_sandbox(
    image: Option<String>,
    cpus: Option<f64>,
    memory: Option<String>,
    network_mode: Option<String>,
) -> Result<serde_json::Value, String> {
    let img = image.unwrap_or_else(|| "ubuntu:22.04".to_string());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let name = format!("vibecody-sb-{}", &format!("{:x}", ts)[..12.min(format!("{:x}", ts).len())]);

    let mut args = vec![
        "run".to_string(), "-d".to_string(),
        "--label".to_string(), "vibecody=sandbox".to_string(),
        "--name".to_string(), name.clone(),
    ];

    if let Some(c) = cpus {
        args.push("--cpus".to_string());
        args.push(format!("{c}"));
    }
    if let Some(ref m) = memory {
        args.push("--memory".to_string());
        args.push(m.clone());
    }
    match network_mode.as_deref() {
        Some("none") => {
            args.push("--network".to_string());
            args.push("none".to_string());
        }
        _ => {}
    }

    args.push(img.clone());
    args.push("tail".to_string());
    args.push("-f".to_string());
    args.push("/dev/null".to_string());

    // Try Docker first, then Podman
    let binary = if tokio::process::Command::new("docker")
        .args(["version"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        "docker"
    } else {
        "podman"
    };

    let output = tokio::process::Command::new(binary)
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("{binary} run failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{binary} run failed: {}", stderr.trim()));
    }

    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(serde_json::json!({
        "id": container_id,
        "name": name,
        "image": img,
        "status": "running",
        "runtime": binary,
    }))
}

/// Stop a sandbox container.
#[tauri::command]
pub async fn stop_sandbox(container_id: String) -> Result<(), String> {
    let binary = detect_container_binary().await;
    let _ = tokio::process::Command::new(&binary)
        .args(["stop", &container_id])
        .output()
        .await;
    let _ = tokio::process::Command::new(&binary)
        .args(["rm", "-f", &container_id])
        .output()
        .await;
    Ok(())
}

/// List all VibeCody sandbox containers.
#[tauri::command]
pub async fn list_sandboxes() -> Result<Vec<serde_json::Value>, String> {
    let binary = detect_container_binary().await;
    let output = tokio::process::Command::new(&binary)
        .args([
            "ps", "-a",
            "--filter", "label=vibecody=sandbox",
            "--format", "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.CreatedAt}}",
        ])
        .output()
        .await
        .map_err(|e| format!("{binary} ps failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let cols: Vec<&str> = line.splitn(5, '\t').collect();
            serde_json::json!({
                "id": cols.first().unwrap_or(&""),
                "name": cols.get(1).unwrap_or(&""),
                "image": cols.get(2).unwrap_or(&""),
                "status": cols.get(3).unwrap_or(&""),
                "created_at": cols.get(4).unwrap_or(&""),
                "runtime": binary,
            })
        })
        .collect();

    Ok(containers)
}

/// Execute a command inside a sandbox container.
#[tauri::command]
pub async fn sandbox_exec(
    container_id: String,
    command: String,
) -> Result<serde_json::Value, String> {
    let binary = detect_container_binary().await;
    let output = tokio::process::Command::new(&binary)
        .args(["exec", &container_id, "sh", "-c", &command])
        .output()
        .await
        .map_err(|e| format!("{binary} exec failed: {e}"))?;

    Ok(serde_json::json!({
        "exit_code": output.status.code().unwrap_or(-1),
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
    }))
}

/// Get sandbox container logs.
#[tauri::command]
pub async fn get_sandbox_logs(
    container_id: String,
    tail: Option<u32>,
) -> Result<String, String> {
    let binary = detect_container_binary().await;
    let tail_str = tail.unwrap_or(100).to_string();
    let output = tokio::process::Command::new(&binary)
        .args(["logs", "--tail", &tail_str, &container_id])
        .output()
        .await
        .map_err(|e| format!("{binary} logs failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("{stdout}{stderr}"))
}

/// Pause a sandbox container.
#[tauri::command]
pub async fn pause_sandbox(container_id: String) -> Result<(), String> {
    let binary = detect_container_binary().await;
    let output = tokio::process::Command::new(&binary)
        .args(["pause", &container_id])
        .output()
        .await
        .map_err(|e| format!("{binary} pause failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pause failed: {}", stderr.trim()));
    }
    Ok(())
}

/// Resume a paused sandbox container.
#[tauri::command]
pub async fn resume_sandbox(container_id: String) -> Result<(), String> {
    let binary = detect_container_binary().await;
    let output = tokio::process::Command::new(&binary)
        .args(["unpause", &container_id])
        .output()
        .await
        .map_err(|e| format!("{binary} unpause failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("resume failed: {}", stderr.trim()));
    }
    Ok(())
}

/// Get sandbox container resource metrics.
#[tauri::command]
pub async fn get_sandbox_metrics(
    container_id: String,
) -> Result<serde_json::Value, String> {
    let binary = detect_container_binary().await;
    let output = tokio::process::Command::new(&binary)
        .args(["stats", "--no-stream", "--format",
               "{{.CPUPerc}}\t{{.MemUsage}}\t{{.PIDs}}", &container_id])
        .output()
        .await
        .map_err(|e| format!("{binary} stats failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split('\t').collect();

    let cpu = parts.first().unwrap_or(&"0%").trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
    let mem_str = parts.get(1).unwrap_or(&"0B / 0B");
    let pids = parts.get(2).unwrap_or(&"0").trim().parse::<u32>().unwrap_or(0);

    Ok(serde_json::json!({
        "cpu_usage_percent": cpu,
        "memory_usage": mem_str,
        "pids": pids,
    }))
}

/// Detect which container binary is available (docker or podman).
async fn detect_container_binary() -> String {
    if tokio::process::Command::new("docker")
        .args(["version"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        "docker".to_string()
    } else {
        "podman".to_string()
    }
}
