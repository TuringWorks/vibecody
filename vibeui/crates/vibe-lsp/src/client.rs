//! LSP client implementation

use anyhow::{Result, anyhow, Context};
use lsp_types::*;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use serde_json::Value;

/// LSP client for communicating with language servers
pub struct LspClient {
    server_cmd: String,
    server_args: Vec<String>,
    process: Option<Child>,
    request_tx: Option<mpsc::Sender<Value>>,
    response_rx: Option<mpsc::Receiver<Value>>,
    request_id: i64,
    initialized: bool,
}

impl LspClient {
    /// Create a new LSP client
    pub fn new(server_cmd: String, server_args: Vec<String>) -> Self {
        Self {
            server_cmd,
            server_args,
            process: None,
            request_tx: None,
            response_rx: None,
            request_id: 0,
            initialized: false,
        }
    }

    /// Start the language server
    pub async fn start(&mut self) -> Result<()> {
        let mut child = Command::new(&self.server_cmd)
            .args(&self.server_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn language server")?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("Failed to open stderr"))?;

        let (req_tx, mut req_rx) = mpsc::channel::<Value>(32);
        let (res_tx, res_rx) = mpsc::channel::<Value>(32);

        // Writer task
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = req_rx.recv().await {
                let body = match serde_json::to_string(&msg) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::error!("Failed to serialize LSP message: {}", e);
                        continue;
                    }
                };
                let header = format!("Content-Length: {}\r\n\r\n", body.len());
                if let Err(e) = stdin.write_all(header.as_bytes()).await {
                    tracing::error!("Failed to write header: {}", e);
                    break;
                }
                if let Err(e) = stdin.write_all(body.as_bytes()).await {
                    tracing::error!("Failed to write body: {}", e);
                    break;
                }
            }
        });

        // Reader task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if line.starts_with("Content-Length: ") {
                            if let Ok(len) = line.trim_start_matches("Content-Length: ").trim().parse::<usize>() {
                                // Read empty line
                                let mut empty = String::new();
                                let _ = reader.read_line(&mut empty).await;
                                
                                // Read body
                                let mut body = vec![0; len];
                                if (reader.read_exact(&mut body).await).is_ok() {
                                    if let Ok(val) = serde_json::from_slice::<Value>(&body) {
                                        let _ = res_tx.send(val).await;
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Stderr logger
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 { break; }
                tracing::error!("LSP Stderr: {}", line.trim());
                line.clear();
            }
        });

        self.process = Some(child);
        self.request_tx = Some(req_tx);
        self.response_rx = Some(res_rx);

        Ok(())
    }

    /// Initialize the language server
    pub async fn initialize(&mut self, root_path: PathBuf) -> Result<()> {
        if self.process.is_none() {
            self.start().await?;
        }

        let uri_string = format!("file://{}", root_path.display());
        let root_uri: Uri = uri_string.parse().map_err(|_| anyhow!("Invalid root path"))?;
        
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            #[allow(deprecated)]
            root_uri: Some(root_uri),
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };

        let _response = self.send_request("initialize", serde_json::to_value(params)?).await?;
        
        // Send initialized notification
        self.send_notification("initialized", serde_json::json!({})).await?;
        
        self.initialized = true;
        Ok(())
    }

    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.request_id;
        self.request_id += 1;

        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        if let Some(tx) = &self.request_tx {
            tx.send(req).await.map_err(|_| anyhow!("Failed to send request"))?;
        }

        // Wait for response
        // Note: This is a simplified synchronous wait. In a real implementation, 
        // we'd need a map of pending requests to handle out-of-order responses.
        // For MVP, we assume sequential processing or just wait for the matching ID.
        if let Some(rx) = &mut self.response_rx {
            while let Some(msg) = rx.recv().await {
                if let Some(msg_id) = msg.get("id").and_then(|i| i.as_i64()) {
                    if msg_id == id {
                        if let Some(result) = msg.get("result") {
                            return Ok(result.clone());
                        } else if let Some(error) = msg.get("error") {
                            return Err(anyhow!("LSP Error: {:?}", error));
                        }
                    }
                }
            }
        }

        Err(anyhow!("No response received"))
    }

    async fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        if let Some(tx) = &self.request_tx {
            tx.send(notif).await.map_err(|_| anyhow!("Failed to send notification"))?;
        }
        Ok(())
    }

    /// Shutdown the language server
    pub async fn shutdown(&mut self) -> Result<()> {
        if self.initialized {
            self.send_request("shutdown", serde_json::json!(null)).await?;
            self.send_notification("exit", serde_json::json!(null)).await?;
        }
        
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }

    /// Send a completion request
    pub async fn completion(&mut self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let res = self.send_request("textDocument/completion", serde_json::to_value(params)?).await?;
        Ok(serde_json::from_value(res).ok())
    }

    /// Send a hover request
    pub async fn hover(&mut self, params: HoverParams) -> Result<Option<Hover>> {
        let res = self.send_request("textDocument/hover", serde_json::to_value(params)?).await?;
        Ok(serde_json::from_value(res).ok())
    }

    /// Send a goto definition request
    pub async fn goto_definition(&mut self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let res = self.send_request("textDocument/definition", serde_json::to_value(params)?).await?;
        Ok(serde_json::from_value(res).ok())
    }

    /// Notify document opened
    pub async fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<()> {
        self.send_notification("textDocument/didOpen", serde_json::to_value(params)?).await
    }

    /// Notify document changed
    pub async fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Result<()> {
        self.send_notification("textDocument/didChange", serde_json::to_value(params)?).await
    }

    /// Notify document saved
    pub async fn did_save(&mut self, params: DidSaveTextDocumentParams) -> Result<()> {
        self.send_notification("textDocument/didSave", serde_json::to_value(params)?).await
    }

    /// Request document symbols (outline view).
    pub async fn document_symbols(
        &mut self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let res = self
            .send_request("textDocument/documentSymbol", serde_json::to_value(params)?)
            .await?;
        Ok(serde_json::from_value(res).ok())
    }

    /// Request full-document formatting edits.
    pub async fn formatting(
        &mut self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let res = self
            .send_request("textDocument/formatting", serde_json::to_value(params)?)
            .await?;
        Ok(serde_json::from_value(res).ok())
    }

    /// Request references for the symbol at a position.
    pub async fn references(
        &mut self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>> {
        let res = self
            .send_request("textDocument/references", serde_json::to_value(params)?)
            .await?;
        Ok(serde_json::from_value(res).ok())
    }

    /// Request rename edits for the symbol at a position.
    pub async fn rename(
        &mut self,
        params: RenameParams,
    ) -> Result<Option<WorkspaceEdit>> {
        let res = self
            .send_request("textDocument/rename", serde_json::to_value(params)?)
            .await?;
        Ok(serde_json::from_value(res).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_client_is_not_initialized() {
        let client = LspClient::new("rust-analyzer".to_string(), vec![]);
        assert!(!client.initialized);
    }

    #[test]
    fn new_client_has_no_process() {
        let client = LspClient::new("rust-analyzer".to_string(), vec![]);
        assert!(client.process.is_none());
    }

    #[test]
    fn new_client_has_no_channels() {
        let client = LspClient::new("test-server".to_string(), vec!["--stdio".to_string()]);
        assert!(client.request_tx.is_none());
        assert!(client.response_rx.is_none());
    }

    #[test]
    fn new_client_starts_with_request_id_zero() {
        let client = LspClient::new("test".to_string(), vec![]);
        assert_eq!(client.request_id, 0);
    }

    #[test]
    fn new_client_stores_server_cmd() {
        let client = LspClient::new("pylsp".to_string(), vec!["--arg1".to_string()]);
        assert_eq!(client.server_cmd, "pylsp");
        assert_eq!(client.server_args, vec!["--arg1"]);
    }

    #[test]
    fn new_client_preserves_multiple_args() {
        let args = vec!["--stdio".to_string(), "--log-level".to_string(), "debug".to_string()];
        let client = LspClient::new("ts-server".to_string(), args.clone());
        assert_eq!(client.server_args, args);
    }

    #[test]
    fn new_client_with_empty_args() {
        let client = LspClient::new("rust-analyzer".to_string(), vec![]);
        assert!(client.server_args.is_empty());
    }

    #[tokio::test]
    async fn shutdown_without_start_is_ok() {
        let mut client = LspClient::new("nonexistent-server".to_string(), vec![]);
        // Shutdown on an un-started, un-initialized client should be a no-op
        let result = client.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn start_nonexistent_server_fails() {
        let mut client = LspClient::new(
            "this-server-does-not-exist-12345".to_string(),
            vec![],
        );
        let result = client.start().await;
        assert!(result.is_err());
    }
}
