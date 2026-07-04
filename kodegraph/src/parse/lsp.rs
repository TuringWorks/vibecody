//! LSP enrichment tier — Tier 2, optional (`lsp` feature).
//!
//! Upgrades call-graph + type-hierarchy edges to compiler-grade precision by asking
//! a real language server (rust-analyzer, pyright, gopls, …) for
//! `textDocument/prepareCallHierarchy` + `callHierarchy/incoming|outgoing` and
//! `textDocument/prepareTypeHierarchy` + `typeHierarchy/supertypes|subtypes`.
//!
//! `kodegraph` ships its **own** minimal blocking JSON-RPC stdio client so the crate
//! stays independent of any editor's LSP wrapper. VibeCody may instead implement
//! [`super::EdgeProvider`] on top of `vibe-lsp` and feed `kodegraph` — see Part B of
//! the plan.
//!
//! Edges produced here are tagged `Lsp` / `Extracted` at confidence `0.95`.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    PartialResultParams, Position, TextDocumentIdentifier, TextDocumentPositionParams,
    TypeHierarchyItem, TypeHierarchyPrepareParams, TypeHierarchySubtypesParams,
    TypeHierarchySupertypesParams, Uri, WorkDoneProgressParams,
};
use serde_json::{json, Value};
use std::str::FromStr;
use url::Url;

/// Build an `lsp_types::Uri` (a `file://` URI) from a filesystem path.
fn file_uri(path: &Path) -> Result<Uri> {
    let url = Url::from_file_path(path).map_err(|_| anyhow!("bad path: {:?}", path))?;
    Uri::from_str(url.as_str()).map_err(|e| anyhow!("bad uri: {e}"))
}

use crate::model::edge::{CallEdge, CallType, EdgeSource, Provenance, TypeRelation, TypeRelationType};
use crate::model::symbol::{Language, Symbol};
use crate::parse::EdgeProvider;

const LSP_PROVENANCE: Provenance = Provenance {
    source: EdgeSource::Lsp,
    tag: crate::model::edge::ProvenanceTag::Extracted,
    confidence: 0.95,
};

/// Minimal blocking JSON-RPC stdio client for a single language server.
struct LspClientInner {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: AtomicU64,
}

/// Thread-safe wrapper around the stdio client.
pub struct LspClient {
    inner: Mutex<LspClientInner>,
    root: PathBuf,
    language: Language,
}

impl LspClient {
    /// Spawn a language server `cmd` with `args` rooted at `root` and send `initialize`.
    pub fn spawn(cmd: &str, args: &[&str], root: &Path, language: Language) -> Result<Self> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("failed to spawn {cmd}: {e}"))?;
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let inner = LspClientInner { stdin, stdout, next_id: AtomicU64::new(1) };
        let client = Self { inner: Mutex::new(inner), root: root.to_path_buf(), language };

        // initialize
        let root_uri = Url::from_file_path(&client.root).map_err(|_| anyhow!("bad root path"))?;
        let _init: Value = client.request(
            "initialize",
            json!({
                "capabilities": {},
                "processId": std::process::id(),
                "rootUri": root_uri.as_str(),
            }),
        )?;
        client.notify("initialized", json!({}))?;
        Ok(client)
    }

    fn request(&self, method: &str, params: Value) -> Result<Value> {
        let mut inner = self.inner.lock().unwrap();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let body = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let serialized = serde_json::to_string(&body)?;
        write!(inner.stdin, "Content-Length: {}\r\n\r\n{}", serialized.len(), serialized)?;
        inner.stdin.flush()?;

        // Read frames until we get a response with our id.
        loop {
            let frame = read_frame(&mut inner.stdout)?;
            let val: Value = serde_json::from_str(&frame)?;
            if val.get("id") == Some(&Value::from(id)) {
                if let Some(err) = val.get("error") {
                    return Err(anyhow!("LSP error: {err}"));
                }
                return Ok(val.get("result").cloned().unwrap_or(Value::Null));
            }
            // Otherwise it's a notification / someone else's response; ignore.
        }
    }

    fn notify(&self, method: &str, params: Value) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let body = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        let serialized = serde_json::to_string(&body)?;
        write!(inner.stdin, "Content-Length: {}\r\n\r\n{}", serialized.len(), serialized)?;
        inner.stdin.flush()?;
        Ok(())
    }

    /// Open a text document in the server (required before hierarchy queries).
    pub fn did_open(&self, path: &Path, text: &str) -> Result<()> {
        let uri = Url::from_file_path(path).map_err(|_| anyhow!("bad path"))?;
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": { "uri": uri.as_str(), "languageId": self.language.as_str(), "version": 1, "text": text }
            }),
        )
    }
}

fn read_frame<R: BufRead>(reader: &mut R) -> Result<String> {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Err(anyhow!("LSP server closed stdout"));
        }
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(rest.trim().parse::<usize>()?);
        }
    }
    let len = content_length.ok_or_else(|| anyhow!("missing Content-Length"))?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// An [`EdgeProvider`] backed by an LSP server. Upgrades call/type edges to
/// confidence `0.95` / `Extracted`.
pub struct LspEdgeProvider {
    client: LspClient,
}

impl LspEdgeProvider {
    /// Construct from an already-spawned [`LspClient`].
    pub fn new(client: LspClient) -> Self {
        Self { client }
    }

    fn prepare_call_hierarchy(&self, sym: &Symbol) -> Result<Vec<CallHierarchyItem>> {
        let uri = file_uri(Path::new(&sym.file_path))?;
        let params = CallHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: (sym.line_start.saturating_sub(1)) as u32,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let res = self.client.request("textDocument/prepareCallHierarchy", serde_json::to_value(&params)?)?;
        let items: Vec<CallHierarchyItem> = serde_json::from_value(res).unwrap_or_default();
        Ok(items)
    }

    fn prepare_type_hierarchy(&self, sym: &Symbol) -> Result<Vec<TypeHierarchyItem>> {
        let uri = file_uri(Path::new(&sym.file_path))?;
        let params = TypeHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line: (sym.line_start.saturating_sub(1)) as u32, character: 0 },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let res = self.client.request("textDocument/prepareTypeHierarchy", serde_json::to_value(&params)?)?;
        let items: Vec<TypeHierarchyItem> = serde_json::from_value(res).unwrap_or_default();
        Ok(items)
    }
}

impl EdgeProvider for LspEdgeProvider {
    fn name(&self) -> &'static str {
        "lsp"
    }

    fn incoming_calls(&self, sym: &Symbol) -> Result<Vec<CallEdge>> {
        let items = self.prepare_call_hierarchy(sym)?;
        let mut out = Vec::new();
        for item in items {
            let params = CallHierarchyIncomingCallsParams {
                item: item.clone(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            let res = self
                .client
                .request("callHierarchy/incomingCalls", serde_json::to_value(&params)?)?;
            let calls: Vec<CallHierarchyIncomingCall> = serde_json::from_value(res).unwrap_or_default();
            for c in calls {
                let caller_name = c.from.name;
                let from_uri = c.from.uri.to_string();
                out.push(CallEdge {
                    caller: caller_name,
                    callee: sym.name.clone(),
                    file: from_uri,
                    line: c.from_ranges.first().map(|r| r.start.line as usize + 1).unwrap_or(0),
                    call_type: CallType::Direct,
                    provenance: LSP_PROVENANCE,
                });
            }
        }
        Ok(out)
    }

    fn outgoing_calls(&self, sym: &Symbol) -> Result<Vec<CallEdge>> {
        let items = self.prepare_call_hierarchy(sym)?;
        let mut out = Vec::new();
        for item in items {
            let params = CallHierarchyOutgoingCallsParams {
                item: item.clone(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            let res = self
                .client
                .request("callHierarchy/outgoingCalls", serde_json::to_value(&params)?)?;
            let calls: Vec<CallHierarchyOutgoingCall> = serde_json::from_value(res).unwrap_or_default();
            for c in calls {
                let callee_name = c.to.name;
                let to_uri = c.to.uri.to_string();
                out.push(CallEdge {
                    caller: sym.name.clone(),
                    callee: callee_name,
                    file: to_uri,
                    line: c.from_ranges.first().map(|r| r.start.line as usize + 1).unwrap_or(0),
                    call_type: CallType::Direct,
                    provenance: LSP_PROVENANCE,
                });
            }
        }
        Ok(out)
    }

    fn supertypes(&self, sym: &Symbol) -> Result<Vec<TypeRelation>> {
        let items = self.prepare_type_hierarchy(sym)?;
        let mut out = Vec::new();
        for item in items {
            let params = TypeHierarchySupertypesParams {
                item: item.clone(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            let res = self
                .client
                .request("typeHierarchy/supertypes", serde_json::to_value(&params)?)?;
            let supers: Vec<TypeHierarchyItem> = serde_json::from_value(res).unwrap_or_default();
            for sup in supers {
                out.push(TypeRelation {
                    parent: sup.name,
                    child: sym.name.clone(),
                    relation: TypeRelationType::Implements,
                    provenance: LSP_PROVENANCE,
                });
            }
        }
        Ok(out)
    }

    fn subtypes(&self, sym: &Symbol) -> Result<Vec<TypeRelation>> {
        let items = self.prepare_type_hierarchy(sym)?;
        let mut out = Vec::new();
        for item in items {
            let params = TypeHierarchySubtypesParams {
                item: item.clone(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };
            let res = self
                .client
                .request("typeHierarchy/subtypes", serde_json::to_value(&params)?)?;
            let subs: Vec<TypeHierarchyItem> = serde_json::from_value(res).unwrap_or_default();
            for sub in subs {
                out.push(TypeRelation {
                    parent: sym.name.clone(),
                    child: sub.name,
                    relation: TypeRelationType::Implements,
                    provenance: LSP_PROVENANCE,
                });
            }
        }
        Ok(out)
    }
}