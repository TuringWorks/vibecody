//! LSP client — a full-duplex JSON-RPC connection to one language server.
//!
//! ## Why this is a dispatcher and not a request/response pair
//!
//! LSP is not request/response. While one `textDocument/completion` is in
//! flight a server also sends notifications (`$/progress`,
//! `textDocument/publishDiagnostics`, `window/logMessage`) and *its own
//! requests* (`client/registerCapability`, `workspace/configuration`). Three
//! consequences shape this module:
//!
//! 1. **The reader must never block.** An earlier version forwarded every
//!    inbound message into a bounded `mpsc` that only `send_request` drained.
//!    `rust-analyzer` emits far more than the channel's 32 messages while
//!    indexing, so the reader wedged on a full channel, stopped draining the
//!    server's stdout, the pipe filled, and the server blocked on write —
//!    IntelliSense died a few seconds after opening a file and never came back.
//!    Here the reader owns the routing and never awaits anything that a
//!    consumer has to pump.
//! 2. **Responses are routed by id**, through a pending-request map, so
//!    concurrent and out-of-order requests can't consume each other's replies.
//! 3. **Server→client requests are always answered** (see [`auto_reply`]).
//!    A server that is still waiting on `workspace/configuration` will happily
//!    sit there forever and answer nothing else.
//!
//! Everything after `initialize` takes `&self`, so the manager can hand out an
//! `Arc<LspClient>` and callers can await a slow request *without* holding the
//! manager lock — one file's cold rust-analyzer must not stall hover in
//! another language.

use crate::discovery::{augmented_path, resolve_server, ServerSearchPaths};
use anyhow::{anyhow, Context, Result};
use lsp_types::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};

/// Budget for a feature request (completion / hover / definition). A wedged
/// server must surface as one failed completion, not a frozen editor.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// `initialize` gets longer: cold servers do real work before replying.
pub const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(45);

/// Outbound queue depth. Notifications (didChange on every keystroke) are the
/// bulk of traffic; deep enough that typing never awaits the writer.
const OUTBOUND_CAPACITY: usize = 256;

type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, String>>>>>;

/// Diagnostics as the server published them, keyed by document URI. Kept as
/// raw JSON: the WebView needs the LSP shape, and re-encoding through
/// `lsp_types` only adds a way to drop fields we don't model.
type DiagnosticMap = Arc<Mutex<HashMap<String, Vec<Value>>>>;

/// Live transport for a spawned server.
struct Transport {
    outbound: mpsc::Sender<Value>,
    pending: PendingMap,
    alive: Arc<AtomicBool>,
}

/// LSP client for communicating with one language server.
pub struct LspClient {
    server_cmd: String,
    server_args: Vec<String>,
    /// `None` until [`LspClient::start`] succeeds.
    transport: Option<Transport>,
    /// Held only so `shutdown` can reap the process.
    process: Mutex<Option<Child>>,
    /// `initialize` result → drives trigger characters and feature gating.
    server_capabilities: Mutex<Option<Value>>,
    diagnostics: DiagnosticMap,
    /// Documents we have sent `didOpen` for → URI → last version we sent.
    open_docs: Mutex<HashMap<String, i32>>,
    /// Answer to `workspace/workspaceFolders`, set during `initialize`.
    workspace_folders: Arc<Mutex<Value>>,
    /// Per-request budget in milliseconds. Adjustable because "slow" is a
    /// property of the repository, not of the protocol.
    request_timeout_ms: AtomicU64,
    next_id: AtomicI64,
    initialized: AtomicBool,
}

impl std::fmt::Debug for LspClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspClient")
            .field("command", &self.server_cmd)
            .field("args", &self.server_args)
            .field("started", &self.transport.is_some())
            .field("alive", &self.is_alive())
            .field("initialized", &self.is_initialized())
            .finish()
    }
}

impl LspClient {
    /// Create a client. Nothing is spawned until [`Self::initialize`].
    pub fn new(server_cmd: String, server_args: Vec<String>) -> Self {
        Self {
            server_cmd,
            server_args,
            transport: None,
            process: Mutex::new(None),
            server_capabilities: Mutex::new(None),
            diagnostics: Arc::new(Mutex::new(HashMap::new())),
            open_docs: Mutex::new(HashMap::new()),
            workspace_folders: Arc::new(Mutex::new(Value::Null)),
            request_timeout_ms: AtomicU64::new(REQUEST_TIMEOUT.as_millis() as u64),
            next_id: AtomicI64::new(1),
            initialized: AtomicBool::new(false),
        }
    }

    /// Change the per-request budget.
    pub fn set_request_timeout(&self, timeout: Duration) {
        self.request_timeout_ms
            .store(timeout.as_millis() as u64, Ordering::Relaxed);
    }

    fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.request_timeout_ms.load(Ordering::Relaxed))
    }

    /// The command this client runs (for diagnostics and error messages).
    pub fn command(&self) -> &str {
        &self.server_cmd
    }

    /// Is the connection still usable? False once the server exits or its
    /// stdout closes — the manager drops dead clients and respawns on demand.
    pub fn is_alive(&self) -> bool {
        self.transport
            .as_ref()
            .is_some_and(|t| t.alive.load(Ordering::Acquire))
    }

    /// Has `initialize` completed?
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    /// Start the language server process and its reader/writer tasks.
    pub async fn start(&mut self) -> Result<()> {
        let search = ServerSearchPaths::from_env();
        // Spawn by absolute path: a GUI-launched app's PATH does not contain
        // ~/.cargo/bin or a Homebrew prefix, so the bare name would not resolve.
        let program = resolve_server(&self.server_cmd, &search).ok_or_else(|| {
            anyhow!(
                "language server '{}' was not found on PATH or in any standard install directory",
                self.server_cmd
            )
        })?;

        let mut child = Command::new(&program)
            .args(&self.server_args)
            // Servers shell out (rust-analyzer → cargo, tsserver → node), so
            // give them the same augmented PATH we used to find them.
            .env("PATH", augmented_path(&search))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("failed to spawn language server '{}'", program.display()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("language server stdin was not piped"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("language server stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("language server stderr was not piped"))?;

        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let alive = Arc::new(AtomicBool::new(true));

        // ── Writer: frame and write everything we send. ──
        let writer_alive = Arc::clone(&alive);
        let writer_cmd = self.server_cmd.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = outbound_rx.recv().await {
                let body = match serde_json::to_vec(&msg) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::error!("{writer_cmd}: failed to serialize LSP message: {e}");
                        continue;
                    }
                };
                let header = format!("Content-Length: {}\r\n\r\n", body.len());
                if stdin.write_all(header.as_bytes()).await.is_err()
                    || stdin.write_all(&body).await.is_err()
                    || stdin.flush().await.is_err()
                {
                    tracing::warn!("{writer_cmd}: language server stdin closed");
                    writer_alive.store(false, Ordering::Release);
                    break;
                }
            }
        });

        // ── Reader: route responses, answer server requests, keep diagnostics. ──
        let reader_pending = Arc::clone(&pending);
        let reader_diagnostics = Arc::clone(&self.diagnostics);
        let reader_outbound = outbound_tx.clone();
        let reader_folders = Arc::clone(&self.workspace_folders);
        let reader_alive = Arc::clone(&alive);
        let reader_cmd = self.server_cmd.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader).await {
                    Ok(Some(msg)) => {
                        route_message(
                            msg,
                            &reader_cmd,
                            &reader_pending,
                            &reader_diagnostics,
                            &reader_outbound,
                            &reader_folders,
                        )
                        .await;
                    }
                    Ok(None) => {
                        tracing::info!("{reader_cmd}: language server closed stdout");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("{reader_cmd}: LSP framing error: {e}");
                        break;
                    }
                }
            }
            // The server is gone. Fail every in-flight request now rather than
            // making each one wait out its timeout.
            reader_alive.store(false, Ordering::Release);
            let waiters = std::mem::take(&mut *reader_pending.lock().await);
            for (_, tx) in waiters {
                let _ = tx.send(Err(format!("language server '{reader_cmd}' exited")));
            }
        });

        // ── Stderr: servers put install hints and crashes here. ──
        let stderr_cmd = self.server_cmd.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let text = line.trim_end();
                        if !text.is_empty() {
                            tracing::debug!("{stderr_cmd} stderr: {text}");
                        }
                    }
                }
            }
        });

        *self.process.lock().await = Some(child);
        self.transport = Some(Transport {
            outbound: outbound_tx,
            pending,
            alive,
        });
        Ok(())
    }

    /// Start (if needed) and perform the LSP `initialize` handshake.
    pub async fn initialize(&mut self, root_path: PathBuf) -> Result<()> {
        if self.transport.is_none() {
            self.start().await?;
        }

        let root_uri = path_to_uri(&root_path);
        let folders = json!([{
            "uri": root_uri,
            "name": root_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "workspace".to_string()),
        }]);
        *self.workspace_folders.lock().await = folders.clone();

        // Hand-written rather than `ClientCapabilities::default()`, which
        // declares nothing: servers gate features on what the client claims, so
        // an empty object is how you get a server that answers completion with
        // plain labels, no snippets, no docs — or refuses outright.
        let params = json!({
            "processId": std::process::id(),
            "clientInfo": { "name": "VibeCoder", "version": env!("CARGO_PKG_VERSION") },
            "rootUri": root_uri,
            "workspaceFolders": folders,
            "capabilities": client_capabilities(),
        });

        let result = self
            .request("initialize", params, INITIALIZE_TIMEOUT)
            .await?;
        *self.server_capabilities.lock().await = result.get("capabilities").cloned();

        self.notify("initialized", json!({})).await?;
        // Servers that read settings (pyright, yaml-language-server) wait for
        // this before doing anything; empty settings means "use your defaults".
        self.notify("workspace/didChangeConfiguration", json!({ "settings": {} }))
            .await?;

        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    /// The server's advertised capabilities, as reported by `initialize`.
    pub async fn server_capabilities(&self) -> Option<Value> {
        self.server_capabilities.lock().await.clone()
    }

    /// Characters that should re-trigger completion (`.`, `::`, `->`, …), as
    /// advertised by the server. Monaco needs these registered up front — with
    /// none, completion only fires mid-identifier and `foo.` shows nothing,
    /// which reads exactly like "IntelliSense is broken".
    pub async fn completion_trigger_characters(&self) -> Vec<String> {
        self.string_array_capability(&["completionProvider", "triggerCharacters"])
            .await
    }

    /// Characters that open signature help (`(`, `,`).
    pub async fn signature_help_trigger_characters(&self) -> Vec<String> {
        self.string_array_capability(&["signatureHelpProvider", "triggerCharacters"])
            .await
    }

    async fn string_array_capability(&self, path: &[&str]) -> Vec<String> {
        let caps = self.server_capabilities.lock().await;
        let Some(mut node) = caps.as_ref() else {
            return Vec::new();
        };
        for key in path {
            match node.get(key) {
                Some(next) => node = next,
                None => return Vec::new(),
            }
        }
        node.as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ── Transport ────────────────────────────────────────────────────────────

    fn transport(&self) -> Result<&Transport> {
        self.transport
            .as_ref()
            .ok_or_else(|| anyhow!("language server '{}' has not been started", self.server_cmd))
    }

    /// Send a request and await its reply, keyed by id.
    async fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value> {
        let transport = self.transport()?;
        if !transport.alive.load(Ordering::Acquire) {
            return Err(anyhow!(
                "language server '{}' is no longer running",
                self.server_cmd
            ));
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        transport.pending.lock().await.insert(id, tx);

        let envelope = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        if transport.outbound.send(envelope).await.is_err() {
            transport.pending.lock().await.remove(&id);
            return Err(anyhow!(
                "language server '{}' is no longer accepting requests",
                self.server_cmd
            ));
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(result))) => Ok(result),
            Ok(Ok(Err(message))) => Err(anyhow!("{} ({})", message, method)),
            Ok(Err(_)) => Err(anyhow!(
                "language server '{}' dropped the reply to {method}",
                self.server_cmd
            )),
            Err(_) => {
                transport.pending.lock().await.remove(&id);
                // Tell the server to stop working on it; it may still be busy.
                let _ = transport
                    .outbound
                    .send(json!({
                        "jsonrpc": "2.0",
                        "method": "$/cancelRequest",
                        "params": { "id": id }
                    }))
                    .await;
                Err(anyhow!(
                    "language server '{}' did not answer {method} within {}s",
                    self.server_cmd,
                    timeout.as_secs()
                ))
            }
        }
    }

    async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let transport = self.transport()?;
        transport
            .outbound
            .send(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
            .await
            .map_err(|_| {
                anyhow!(
                    "language server '{}' is no longer accepting notifications",
                    self.server_cmd
                )
            })
    }

    /// Shut the server down cleanly, then reap it.
    pub async fn shutdown(&self) -> Result<()> {
        if self.initialized.load(Ordering::Acquire) && self.is_alive() {
            // Best-effort: a wedged server should not block app teardown.
            let _ = self
                .request("shutdown", Value::Null, Duration::from_secs(2))
                .await;
            let _ = self.notify("exit", Value::Null).await;
        }
        if let Some(mut child) = self.process.lock().await.take() {
            let _ = child.kill().await;
        }
        self.initialized.store(false, Ordering::Release);
        Ok(())
    }

    // ── Document synchronisation ─────────────────────────────────────────────

    /// Open a document, or resync it if it is already open.
    ///
    /// Idempotent on purpose: the frontend may reopen the same tab, and a
    /// duplicate `didOpen` makes servers report the document twice.
    pub async fn open_document(&self, uri: &str, language_id: &str, text: &str) -> Result<()> {
        if self.open_docs.lock().await.contains_key(uri) {
            return self.send_did_change(uri, text).await;
        }
        self.send_did_open(uri, language_id, text).await
    }

    /// Push the document's current text to the server (full-text sync).
    ///
    /// Falls back to `didOpen` for a document the server has not seen —
    /// otherwise a server that started *after* the tab was opened (or was
    /// restarted after a crash) rejects every request for that file, and
    /// completion silently returns nothing for the rest of the session.
    pub async fn change_document(&self, uri: &str, language_id: &str, text: &str) -> Result<()> {
        if self.open_docs.lock().await.contains_key(uri) {
            return self.send_did_change(uri, text).await;
        }
        self.send_did_open(uri, language_id, text).await
    }

    async fn send_did_open(&self, uri: &str, language_id: &str, text: &str) -> Result<()> {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }),
        )
        .await?;
        self.open_docs.lock().await.insert(uri.to_string(), 1);
        Ok(())
    }

    async fn send_did_change(&self, uri: &str, text: &str) -> Result<()> {
        // Versions must increase monotonically per document or servers discard
        // the change and keep answering against stale text.
        let version = {
            let mut docs = self.open_docs.lock().await;
            let version = docs.entry(uri.to_string()).or_insert(1);
            *version += 1;
            *version
        };
        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }],
            }),
        )
        .await
    }

    /// Notify the server that a document was saved.
    pub async fn save_document(&self, uri: &str, text: Option<&str>) -> Result<()> {
        if !self.open_docs.lock().await.contains_key(uri) {
            return Ok(());
        }
        let mut params = json!({ "textDocument": { "uri": uri } });
        if let (Some(text), Some(obj)) = (text, params.as_object_mut()) {
            obj.insert("text".into(), Value::String(text.to_string()));
        }
        self.notify("textDocument/didSave", params).await
    }

    /// Notify the server that a document was closed, and drop its diagnostics.
    pub async fn close_document(&self, uri: &str) -> Result<()> {
        if self.open_docs.lock().await.remove(uri).is_none() {
            return Ok(());
        }
        self.diagnostics.lock().await.remove(uri);
        self.notify(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri } }),
        )
        .await
    }

    /// Is this document currently open on the server?
    pub async fn is_document_open(&self, uri: &str) -> bool {
        self.open_docs.lock().await.contains_key(uri)
    }

    /// Latest diagnostics the server published for `uri`, if any.
    ///
    /// `None` means "the server has not published for this document", which is
    /// different from `Some(vec![])` — "it published, and the file is clean".
    /// Callers clear markers on the latter and leave them alone on the former.
    pub async fn diagnostics_for(&self, uri: &str) -> Option<Vec<Value>> {
        self.diagnostics.lock().await.get(&normalize_uri(uri)).cloned()
    }

    // ── Requests ─────────────────────────────────────────────────────────────

    /// Send a completion request.
    pub async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let res = self
            .request(
                "textDocument/completion",
                serde_json::to_value(params)?,
                self.request_timeout(),
            )
            .await?;
        parse_optional(res, "completion")
    }

    /// Resolve extra detail (documentation, additional edits) for one item.
    pub async fn resolve_completion_item(&self, item: Value) -> Result<Value> {
        self.request("completionItem/resolve", item, self.request_timeout())
            .await
    }

    /// Send a hover request.
    pub async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let res = self
            .request(
                "textDocument/hover",
                serde_json::to_value(params)?,
                self.request_timeout(),
            )
            .await?;
        parse_optional(res, "hover")
    }

    /// Send a goto definition request.
    pub async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let res = self
            .request(
                "textDocument/definition",
                serde_json::to_value(params)?,
                self.request_timeout(),
            )
            .await?;
        parse_optional(res, "definition")
    }

    /// Signature help (parameter hints) at a position.
    pub async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let res = self
            .request(
                "textDocument/signatureHelp",
                serde_json::to_value(params)?,
                self.request_timeout(),
            )
            .await?;
        parse_optional(res, "signatureHelp")
    }

    /// Request document symbols (outline view).
    pub async fn document_symbols(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let res = self
            .request(
                "textDocument/documentSymbol",
                serde_json::to_value(params)?,
                self.request_timeout(),
            )
            .await?;
        parse_optional(res, "documentSymbol")
    }

    /// Request full-document formatting edits.
    pub async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let res = self
            .request(
                "textDocument/formatting",
                serde_json::to_value(params)?,
                self.request_timeout(),
            )
            .await?;
        parse_optional(res, "formatting")
    }

    /// Request references for the symbol at a position.
    pub async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let res = self
            .request(
                "textDocument/references",
                serde_json::to_value(params)?,
                self.request_timeout(),
            )
            .await?;
        parse_optional(res, "references")
    }

    /// Request rename edits for the symbol at a position.
    pub async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let res = self
            .request(
                "textDocument/rename",
                serde_json::to_value(params)?,
                self.request_timeout(),
            )
            .await?;
        parse_optional(res, "rename")
    }
}

/// Decode a result, distinguishing "the server said null" from "we could not
/// read what it said". Swallowing the parse error (the old `.ok()`) turned a
/// schema mismatch into a silent empty completion list.
fn parse_optional<T: serde::de::DeserializeOwned>(res: Value, what: &str) -> Result<Option<T>> {
    if res.is_null() {
        return Ok(None);
    }
    serde_json::from_value(res)
        .map(Some)
        .with_context(|| format!("could not decode {what} response"))
}

/// `file://` URI for a filesystem path, percent-encoding what must be encoded.
///
/// `format!("file://{}", path.display())` — the previous approach — produces an
/// invalid URI for any path containing a space or `#`, and the server then
/// rejects every request for that document.
pub fn path_to_uri(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    let mut out = String::with_capacity(text.len() + 8);
    out.push_str("file://");
    #[cfg(windows)]
    {
        // C:\src\a.rs → file:///C:/src/a.rs
        out.push('/');
    }
    for ch in text.chars() {
        match ch {
            '/' | '-' | '_' | '.' | '~' | ':' => out.push(ch),
            #[cfg(windows)]
            '\\' => out.push('/'),
            c if c.is_ascii_alphanumeric() => out.push(c),
            c => {
                let mut buf = [0u8; 4];
                for byte in c.encode_utf8(&mut buf).as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

/// A comparable form of a `file://` URI: percent-decoded, so the same document
/// keys the same entry no matter which side encoded it.
pub fn normalize_uri(uri: &str) -> String {
    let bytes = uri.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match (bytes[i], bytes.get(i + 1), bytes.get(i + 2)) {
            (b'%', Some(hi), Some(lo)) => {
                match u8::from_str_radix(&format!("{}{}", *hi as char, *lo as char), 16) {
                    Ok(byte) => {
                        decoded.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        decoded.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            _ => {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).to_string()
}

/// Read one `Content-Length`-framed message. `Ok(None)` is a clean EOF.
async fn read_message<R: AsyncBufReadExt + AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<Option<Value>> {
    let mut content_length: Option<usize> = None;
    // Headers, terminated by a blank line. Servers may send `Content-Type` too,
    // so we scan until the blank line instead of assuming a fixed layout.
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            return Ok(None);
        }
        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse::<usize>().ok();
            }
        }
    }

    let length = content_length
        .ok_or_else(|| anyhow!("LSP message header had no usable Content-Length"))?;
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).await?;
    serde_json::from_slice(&body)
        .map(Some)
        .context("LSP message body was not valid JSON")
}

/// Send one inbound message where it belongs.
async fn route_message(
    msg: Value,
    server_cmd: &str,
    pending: &PendingMap,
    diagnostics: &DiagnosticMap,
    outbound: &mpsc::Sender<Value>,
    workspace_folders: &Arc<Mutex<Value>>,
) {
    let method = msg.get("method").and_then(Value::as_str).map(str::to_string);
    let id = msg.get("id").cloned();

    match (method, id) {
        // Response to one of our requests.
        (None, Some(id)) => {
            let Some(id) = id.as_i64() else { return };
            let Some(waiter) = pending.lock().await.remove(&id) else {
                tracing::debug!("{server_cmd}: reply to unknown or cancelled request {id}");
                return;
            };
            let payload = match (msg.get("result"), msg.get("error")) {
                (Some(result), _) => Ok(result.clone()),
                (None, Some(error)) => Err(lsp_error_message(error)),
                // A response with neither is malformed; treat as null result so
                // the caller sees "no data" rather than hanging to its timeout.
                (None, None) => Ok(Value::Null),
            };
            let _ = waiter.send(payload);
        }

        // The server is asking us something. It blocks until we answer.
        (Some(method), Some(id)) => {
            let reply = match auto_reply(&method, msg.get("params"), workspace_folders).await {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err(message) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": message },
                }),
            };
            if outbound.send(reply).await.is_err() {
                tracing::debug!("{server_cmd}: could not answer server request {method}");
            }
        }

        // Notification.
        (Some(method), None) => {
            handle_notification(&method, msg.get("params"), server_cmd, diagnostics).await;
        }

        (None, None) => tracing::debug!("{server_cmd}: ignoring malformed LSP message"),
    }
}

fn lsp_error_message(error: &Value) -> String {
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("language server reported an error");
    match error.get("code").and_then(Value::as_i64) {
        Some(code) => format!("{message} (LSP error {code})"),
        None => message.to_string(),
    }
}

async fn handle_notification(
    method: &str,
    params: Option<&Value>,
    server_cmd: &str,
    diagnostics: &DiagnosticMap,
) {
    match method {
        "textDocument/publishDiagnostics" => {
            let Some(params) = params else { return };
            let Some(uri) = params.get("uri").and_then(Value::as_str) else {
                return;
            };
            let items = params
                .get("diagnostics")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            // Normalised key: servers are free to re-encode the URI they
            // publish under (`a%20b.rs` vs `a b.rs`), and a raw-string key
            // would then never match the lookup for that exact document.
            diagnostics.lock().await.insert(normalize_uri(uri), items);
        }
        "window/logMessage" | "window/showMessage" => {
            let text = params
                .and_then(|p| p.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("");
            tracing::debug!("{server_cmd}: {text}");
        }
        _ => tracing::trace!("{server_cmd}: notification {method}"),
    }
}

/// What to answer a server→client request with.
///
/// Every arm matters: a server that asked and got no reply stops making
/// progress. `Err` becomes a `MethodNotFound` error response — still an answer.
async fn auto_reply(
    method: &str,
    params: Option<&Value>,
    workspace_folders: &Arc<Mutex<Value>>,
) -> Result<Value, String> {
    match method {
        // One entry per requested section. `null` = "not configured", which
        // every server reads as "use your defaults".
        "workspace/configuration" => {
            let count = params
                .and_then(|p| p.get("items"))
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(1);
            Ok(Value::Array(vec![Value::Null; count.max(1)]))
        }
        "workspace/workspaceFolders" => Ok(workspace_folders.lock().await.clone()),
        // We register everything statically, so dynamic (un)registration is a
        // no-op we simply acknowledge.
        "client/registerCapability" | "client/unregisterCapability" => Ok(Value::Null),
        "window/workDoneProgress/create" => Ok(Value::Null),
        "workspace/semanticTokens/refresh"
        | "workspace/codeLens/refresh"
        | "workspace/inlayHint/refresh"
        | "workspace/diagnostic/refresh"
        | "workspace/inlineValue/refresh" => Ok(Value::Null),
        // Refuse rather than silently rewriting the user's files from a
        // background server request.
        "workspace/applyEdit" => Ok(json!({
            "applied": false,
            "failureReason": "VibeCoder does not apply server-initiated edits",
        })),
        // No UI to show a modal from here; "no button chosen" is a valid answer.
        "window/showMessageRequest" | "window/showDocument" => Ok(Value::Null),
        other => Err(format!("VibeCoder does not implement {other}")),
    }
}

/// What we tell the server we can do.
///
/// Kept as JSON rather than `lsp_types::ClientCapabilities` so it survives
/// `lsp-types` upgrades unchanged, and so every claim here is visible next to
/// the code that has to honour it.
fn client_capabilities() -> Value {
    json!({
        "workspace": {
            "workspaceFolders": true,
            "configuration": true,
            "didChangeConfiguration": { "dynamicRegistration": false },
            "applyEdit": false,
            "symbol": { "dynamicRegistration": false },
        },
        "textDocument": {
            "synchronization": {
                "dynamicRegistration": false,
                "willSave": false,
                "willSaveWaitUntil": false,
                "didSave": true,
            },
            "completion": {
                "dynamicRegistration": false,
                "contextSupport": true,
                "completionItem": {
                    // Monaco inserts snippets, so ask for the good candidates.
                    "snippetSupport": true,
                    "commitCharactersSupport": true,
                    "documentationFormat": ["markdown", "plaintext"],
                    "deprecatedSupport": true,
                    "preselectSupport": true,
                    "insertReplaceSupport": true,
                    "labelDetailsSupport": true,
                    "resolveSupport": {
                        "properties": ["documentation", "detail", "additionalTextEdits"]
                    },
                    "tagSupport": { "valueSet": [1] },
                },
                // LSP CompletionItemKind 1..=25. Declaring the full set stops
                // servers from down-mapping everything to Text.
                "completionItemKind": {
                    "valueSet": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
                                 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
                },
            },
            "hover": {
                "dynamicRegistration": false,
                "contentFormat": ["markdown", "plaintext"],
            },
            "signatureHelp": {
                "dynamicRegistration": false,
                "signatureInformation": {
                    "documentationFormat": ["markdown", "plaintext"],
                    "parameterInformation": { "labelOffsetSupport": true },
                    "activeParameterSupport": true,
                },
                "contextSupport": true,
            },
            // linkSupport false: we consume plain Locations on the Monaco side.
            "definition": { "dynamicRegistration": false, "linkSupport": false },
            "typeDefinition": { "dynamicRegistration": false, "linkSupport": false },
            "implementation": { "dynamicRegistration": false, "linkSupport": false },
            "references": { "dynamicRegistration": false },
            "documentHighlight": { "dynamicRegistration": false },
            "documentSymbol": {
                "dynamicRegistration": false,
                "hierarchicalDocumentSymbolSupport": true,
            },
            "formatting": { "dynamicRegistration": false },
            "rangeFormatting": { "dynamicRegistration": false },
            "rename": { "dynamicRegistration": false, "prepareSupport": false },
            "publishDiagnostics": {
                "relatedInformation": true,
                "versionSupport": true,
                "codeDescriptionSupport": true,
                "tagSupport": { "valueSet": [1, 2] },
            },
            "codeAction": {
                "dynamicRegistration": false,
                "codeActionLiteralSupport": {
                    "codeActionKind": {
                        "valueSet": ["quickfix", "refactor", "refactor.extract",
                                     "refactor.inline", "refactor.rewrite", "source",
                                     "source.organizeImports"]
                    }
                },
            },
        },
        "window": { "workDoneProgress": true },
        "general": { "positionEncodings": ["utf-16"] },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_client_is_not_initialized() {
        let client = LspClient::new("rust-analyzer".to_string(), vec![]);
        assert!(!client.is_initialized());
    }

    #[test]
    fn new_client_has_no_transport() {
        let client = LspClient::new("rust-analyzer".to_string(), vec![]);
        assert!(client.transport.is_none());
        assert!(!client.is_alive());
    }

    #[test]
    fn new_client_stores_command_and_args() {
        let client = LspClient::new("pylsp".to_string(), vec!["--arg1".to_string()]);
        assert_eq!(client.command(), "pylsp");
        assert_eq!(client.server_args, vec!["--arg1"]);
    }

    #[tokio::test]
    async fn requests_before_start_fail_with_a_clear_message() {
        let client = LspClient::new("rust-analyzer".to_string(), vec![]);
        let err = client
            .notify("textDocument/didOpen", json!({}))
            .await
            .expect_err("no transport yet");
        assert!(err.to_string().contains("has not been started"), "{err}");
    }

    #[tokio::test]
    async fn shutdown_without_start_is_ok() {
        let client = LspClient::new("nonexistent-server".to_string(), vec![]);
        assert!(client.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn start_nonexistent_server_fails_with_not_found() {
        let mut client = LspClient::new("this-server-does-not-exist-12345".to_string(), vec![]);
        let err = client.start().await.expect_err("should not resolve");
        assert!(err.to_string().contains("was not found"), "{err}");
    }

    // ── Framing ─────────────────────────────────────────────────────────────

    async fn frame(raw: &str) -> Result<Option<Value>> {
        let mut reader = BufReader::new(raw.as_bytes());
        read_message(&mut reader).await
    }

    #[tokio::test]
    async fn reads_a_simple_framed_message() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
        let raw = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let msg = frame(&raw).await.expect("parse").expect("message");
        assert_eq!(msg["id"], 1);
    }

    #[tokio::test]
    async fn reads_a_message_with_extra_headers() {
        // Some servers send Content-Type; assuming a fixed two-line header
        // (the old reader) desynchronises the stream permanently.
        let body = r#"{"jsonrpc":"2.0","method":"$/progress","params":{}}"#;
        let raw = format!(
            "Content-Length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{body}",
            body.len()
        );
        let msg = frame(&raw).await.expect("parse").expect("message");
        assert_eq!(msg["method"], "$/progress");
    }

    #[tokio::test]
    async fn header_name_is_case_insensitive() {
        let body = r#"{"jsonrpc":"2.0","id":7,"result":null}"#;
        let raw = format!("content-length: {}\r\n\r\n{body}", body.len());
        let msg = frame(&raw).await.expect("parse").expect("message");
        assert_eq!(msg["id"], 7);
    }

    #[tokio::test]
    async fn reads_two_messages_back_to_back() {
        let a = r#"{"jsonrpc":"2.0","id":1,"result":1}"#;
        let b = r#"{"jsonrpc":"2.0","id":2,"result":2}"#;
        let raw = format!(
            "Content-Length: {}\r\n\r\n{a}Content-Length: {}\r\n\r\n{b}",
            a.len(),
            b.len()
        );
        let mut reader = BufReader::new(raw.as_bytes());
        let first = read_message(&mut reader).await.expect("a").expect("a");
        let second = read_message(&mut reader).await.expect("b").expect("b");
        assert_eq!(first["result"], 1);
        assert_eq!(second["result"], 2);
    }

    #[tokio::test]
    async fn utf8_body_length_is_counted_in_bytes() {
        let body = r#"{"jsonrpc":"2.0","id":3,"result":"héllo → ok"}"#;
        let raw = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let msg = frame(&raw).await.expect("parse").expect("message");
        assert_eq!(msg["result"], "héllo → ok");
    }

    #[tokio::test]
    async fn eof_is_none_not_an_error() {
        assert!(frame("").await.expect("clean eof").is_none());
    }

    #[tokio::test]
    async fn missing_content_length_is_an_error() {
        assert!(frame("X-Nonsense: 1\r\n\r\n{}").await.is_err());
    }

    // ── Routing ─────────────────────────────────────────────────────────────

    fn routing_fixture() -> (PendingMap, DiagnosticMap, mpsc::Receiver<Value>, mpsc::Sender<Value>, Arc<Mutex<Value>>) {
        let (tx, rx) = mpsc::channel(16);
        (
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
            rx,
            tx,
            Arc::new(Mutex::new(json!([{ "uri": "file:///w", "name": "w" }]))),
        )
    }

    #[tokio::test]
    async fn response_is_routed_to_its_waiter() {
        let (pending, diags, _rx, tx, folders) = routing_fixture();
        let (done_tx, done_rx) = oneshot::channel();
        pending.lock().await.insert(42, done_tx);

        route_message(
            json!({"jsonrpc":"2.0","id":42,"result":{"ok":true}}),
            "test",
            &pending,
            &diags,
            &tx,
            &folders,
        )
        .await;

        let result = done_rx.await.expect("delivered").expect("ok result");
        assert_eq!(result["ok"], true);
        assert!(pending.lock().await.is_empty(), "waiter should be consumed");
    }

    #[tokio::test]
    async fn out_of_order_responses_reach_the_right_waiter() {
        // The old client returned the first reply it saw to whoever was
        // waiting, so a slow completion and a fast hover swapped answers.
        let (pending, diags, _rx, tx, folders) = routing_fixture();
        let (tx_a, rx_a) = oneshot::channel();
        let (tx_b, rx_b) = oneshot::channel();
        pending.lock().await.insert(1, tx_a);
        pending.lock().await.insert(2, tx_b);

        for msg in [
            json!({"jsonrpc":"2.0","id":2,"result":"second"}),
            json!({"jsonrpc":"2.0","id":1,"result":"first"}),
        ] {
            route_message(msg, "test", &pending, &diags, &tx, &folders).await;
        }

        assert_eq!(rx_a.await.expect("a").expect("ok"), json!("first"));
        assert_eq!(rx_b.await.expect("b").expect("ok"), json!("second"));
    }

    #[tokio::test]
    async fn error_response_becomes_an_error_for_the_waiter() {
        let (pending, diags, _rx, tx, folders) = routing_fixture();
        let (done_tx, done_rx) = oneshot::channel();
        pending.lock().await.insert(5, done_tx);

        route_message(
            json!({"jsonrpc":"2.0","id":5,"error":{"code":-32602,"message":"bad params"}}),
            "test",
            &pending,
            &diags,
            &tx,
            &folders,
        )
        .await;

        let err = done_rx.await.expect("delivered").expect_err("error");
        assert!(err.contains("bad params"), "{err}");
        assert!(err.contains("-32602"), "{err}");
    }

    #[tokio::test]
    async fn notification_does_not_consume_a_pending_waiter() {
        // A server request/notification sharing an id with one of ours must not
        // be mistaken for its response.
        let (pending, diags, _rx, tx, folders) = routing_fixture();
        let (done_tx, _done_rx) = oneshot::channel();
        pending.lock().await.insert(1, done_tx);

        route_message(
            json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":1}}),
            "test",
            &pending,
            &diags,
            &tx,
            &folders,
        )
        .await;

        assert_eq!(pending.lock().await.len(), 1, "waiter must survive");
    }

    #[tokio::test]
    async fn publish_diagnostics_is_stored_by_uri() {
        let (pending, diags, _rx, tx, folders) = routing_fixture();
        route_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": {
                    "uri": "file:///w/src/main.rs",
                    "diagnostics": [{
                        "range": {"start":{"line":3,"character":4},"end":{"line":3,"character":9}},
                        "severity": 1,
                        "message": "cannot find value `x`"
                    }]
                }
            }),
            "test",
            &pending,
            &diags,
            &tx,
            &folders,
        )
        .await;

        let stored = diags.lock().await;
        let items = stored.get("file:///w/src/main.rs").expect("stored");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["message"], "cannot find value `x`");
    }

    #[tokio::test]
    async fn empty_publish_diagnostics_clears_to_an_empty_list() {
        // Distinguishable from "never published": Some(vec![]) vs None.
        let (pending, diags, _rx, tx, folders) = routing_fixture();
        route_message(
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": { "uri": "file:///w/a.rs", "diagnostics": [] }
            }),
            "test",
            &pending,
            &diags,
            &tx,
            &folders,
        )
        .await;
        assert_eq!(diags.lock().await.get("file:///w/a.rs"), Some(&vec![]));
    }

    #[tokio::test]
    async fn server_request_gets_answered() {
        let (pending, diags, mut rx, tx, folders) = routing_fixture();
        route_message(
            json!({"jsonrpc":"2.0","id":1,"method":"client/registerCapability","params":{}}),
            "test",
            &pending,
            &diags,
            &tx,
            &folders,
        )
        .await;

        let reply = rx.recv().await.expect("reply sent");
        assert_eq!(reply["id"], 1);
        assert!(reply.get("result").is_some(), "must be a success reply");
    }

    #[tokio::test]
    async fn unknown_server_request_still_gets_an_error_reply() {
        let (pending, diags, mut rx, tx, folders) = routing_fixture();
        route_message(
            json!({"jsonrpc":"2.0","id":9,"method":"weird/thing"}),
            "test",
            &pending,
            &diags,
            &tx,
            &folders,
        )
        .await;

        let reply = rx.recv().await.expect("reply sent");
        assert_eq!(reply["id"], 9);
        assert_eq!(reply["error"]["code"], -32601);
    }

    // ── auto_reply ──────────────────────────────────────────────────────────

    fn folders_fixture() -> Arc<Mutex<Value>> {
        Arc::new(Mutex::new(json!([{ "uri": "file:///w", "name": "w" }])))
    }

    #[tokio::test]
    async fn configuration_reply_has_one_entry_per_item() {
        let params = json!({ "items": [{"section":"rust-analyzer"},{"section":"editor"}] });
        let reply = auto_reply("workspace/configuration", Some(&params), &folders_fixture())
            .await
            .expect("ok");
        assert_eq!(reply.as_array().map(Vec::len), Some(2));
    }

    #[tokio::test]
    async fn configuration_reply_is_never_empty() {
        let reply = auto_reply("workspace/configuration", None, &folders_fixture())
            .await
            .expect("ok");
        assert_eq!(reply.as_array().map(Vec::len), Some(1));
    }

    #[tokio::test]
    async fn workspace_folders_reply_echoes_the_root() {
        let reply = auto_reply("workspace/workspaceFolders", None, &folders_fixture())
            .await
            .expect("ok");
        assert_eq!(reply[0]["uri"], "file:///w");
    }

    #[tokio::test]
    async fn apply_edit_is_refused_not_ignored() {
        let reply = auto_reply("workspace/applyEdit", None, &folders_fixture())
            .await
            .expect("ok");
        assert_eq!(reply["applied"], false);
    }

    // ── Capabilities ────────────────────────────────────────────────────────

    #[test]
    fn capabilities_declare_snippet_and_markdown_support() {
        let caps = client_capabilities();
        assert_eq!(
            caps["textDocument"]["completion"]["completionItem"]["snippetSupport"],
            true
        );
        assert_eq!(
            caps["textDocument"]["hover"]["contentFormat"][0],
            "markdown"
        );
    }

    #[test]
    fn capabilities_declare_every_completion_item_kind() {
        let caps = client_capabilities();
        let kinds = caps["textDocument"]["completion"]["completionItemKind"]["valueSet"]
            .as_array()
            .expect("valueSet");
        assert_eq!(kinds.len(), 25, "LSP defines kinds 1..=25");
    }

    #[test]
    fn capabilities_ask_for_did_save() {
        let caps = client_capabilities();
        assert_eq!(caps["textDocument"]["synchronization"]["didSave"], true);
    }

    #[tokio::test]
    async fn trigger_characters_come_from_server_capabilities() {
        let client = LspClient::new("test".into(), vec![]);
        *client.server_capabilities.lock().await = Some(json!({
            "completionProvider": { "triggerCharacters": [".", ":"] },
            "signatureHelpProvider": { "triggerCharacters": ["(", ","] },
        }));
        assert_eq!(
            client.completion_trigger_characters().await,
            vec![".".to_string(), ":".to_string()]
        );
        assert_eq!(
            client.signature_help_trigger_characters().await,
            vec!["(".to_string(), ",".to_string()]
        );
    }

    #[tokio::test]
    async fn trigger_characters_are_empty_when_unadvertised() {
        let client = LspClient::new("test".into(), vec![]);
        assert!(client.completion_trigger_characters().await.is_empty());
        *client.server_capabilities.lock().await = Some(json!({ "hoverProvider": true }));
        assert!(client.completion_trigger_characters().await.is_empty());
    }

    // ── URIs ────────────────────────────────────────────────────────────────

    #[test]
    fn plain_path_uri() {
        assert_eq!(
            path_to_uri(std::path::Path::new("/home/dev/project/src/main.rs")),
            "file:///home/dev/project/src/main.rs"
        );
    }

    #[test]
    fn spaces_are_percent_encoded() {
        // `format!("file://{path}")` yields an unparseable URI here, and the
        // server rejects every request for the document.
        assert_eq!(
            path_to_uri(std::path::Path::new("/Users/dev/My Code/a.rs")),
            "file:///Users/dev/My%20Code/a.rs"
        );
    }

    #[test]
    fn hash_and_question_marks_are_encoded() {
        let uri = path_to_uri(std::path::Path::new("/tmp/we#ird?name.rs"));
        assert_eq!(uri, "file:///tmp/we%23ird%3Fname.rs");
    }

    #[test]
    fn non_ascii_is_percent_encoded_as_utf8() {
        assert_eq!(
            path_to_uri(std::path::Path::new("/tmp/café.rs")),
            "file:///tmp/caf%C3%A9.rs"
        );
    }

    #[test]
    fn uri_round_trips_through_lsp_types() {
        for path in [
            "/home/dev/a.rs",
            "/Users/dev/My Code/b.rs",
            "/tmp/café.rs",
            "/tmp/we#ird.rs",
        ] {
            let uri = path_to_uri(std::path::Path::new(path));
            assert!(
                uri.parse::<lsp_types::Uri>().is_ok(),
                "{uri} must be a valid LSP Uri"
            );
        }
    }

    // ── Optional-result decoding ────────────────────────────────────────────

    #[test]
    fn null_result_is_none() {
        let parsed: Option<Hover> = parse_optional(Value::Null, "hover").expect("ok");
        assert!(parsed.is_none());
    }

    #[test]
    fn undecodable_result_is_an_error_not_silent_none() {
        let res: Result<Option<Vec<Location>>> =
            parse_optional(json!({"not":"a location list"}), "references");
        assert!(res.is_err(), "a schema mismatch must not read as 'no data'");
    }

    #[test]
    fn completion_list_result_decodes() {
        let value = json!({
            "isIncomplete": false,
            "items": [{ "label": "push", "kind": 2 }]
        });
        let parsed: Option<CompletionResponse> = parse_optional(value, "completion").expect("ok");
        match parsed {
            Some(CompletionResponse::List(list)) => {
                assert_eq!(list.items.len(), 1);
                assert_eq!(list.items[0].label, "push");
            }
            other => panic!("expected a CompletionList, got {other:?}"),
        }
    }

    // ── Document bookkeeping ────────────────────────────────────────────────

    #[tokio::test]
    async fn document_is_not_open_before_did_open() {
        let client = LspClient::new("test".into(), vec![]);
        assert!(!client.is_document_open("file:///w/a.rs").await);
    }

    #[tokio::test]
    async fn save_and_close_of_an_unopened_document_are_no_ops() {
        // No transport, so a real notify would fail — these must short-circuit.
        let client = LspClient::new("test".into(), vec![]);
        assert!(client.save_document("file:///w/a.rs", None).await.is_ok());
        assert!(client.close_document("file:///w/a.rs").await.is_ok());
    }

    #[tokio::test]
    async fn diagnostics_absent_and_empty_are_distinguishable() {
        let client = LspClient::new("test".into(), vec![]);
        assert_eq!(client.diagnostics_for("file:///w/a.rs").await, None);
        client
            .diagnostics
            .lock()
            .await
            .insert("file:///w/a.rs".into(), vec![]);
        assert_eq!(client.diagnostics_for("file:///w/a.rs").await, Some(vec![]));
    }

    #[tokio::test]
    async fn diagnostics_lookup_survives_uri_re_encoding() {
        // We ask with the encoded URI Monaco produced; the server published
        // under a decoded one (or the reverse). Same document either way.
        let client = LspClient::new("test".into(), vec![]);
        {
            let mut store = client.diagnostics.lock().await;
            store.insert(
                normalize_uri("file:///w/My%20Code/a.rs"),
                vec![json!({"message": "oops"})],
            );
        }
        let found = client
            .diagnostics_for("file:///w/My Code/a.rs")
            .await
            .expect("same document");
        assert_eq!(found[0]["message"], "oops");
    }

    #[test]
    fn normalize_uri_decodes_percent_escapes() {
        assert_eq!(
            normalize_uri("file:///tmp/a%20b%23c.rs"),
            "file:///tmp/a b#c.rs"
        );
    }

    #[test]
    fn normalize_uri_is_idempotent() {
        let once = normalize_uri("file:///tmp/caf%C3%A9.rs");
        assert_eq!(once, "file:///tmp/café.rs");
        assert_eq!(normalize_uri(&once), once);
    }

    #[test]
    fn normalize_uri_leaves_a_stray_percent_alone() {
        assert_eq!(normalize_uri("file:///tmp/100%.rs"), "file:///tmp/100%.rs");
    }
}
