//! LSP feature implementations
//!
//! High-level helpers built on top of `LspClient` for common editor features:
//! completion, hover, go-to-definition, document symbols, formatting,
//! references, and rename.
//!
//! All functions delegate directly to the corresponding LSP request methods
//! on `LspClient`; they exist so call-sites read `features::get_completions`
//! rather than `client.completion`.

use anyhow::Result;
use lsp_types::{
    CompletionParams, CompletionResponse,
    DocumentFormattingParams, DocumentSymbolParams, DocumentSymbolResponse,
    GotoDefinitionParams, GotoDefinitionResponse,
    Hover, HoverParams,
    Location, ReferenceParams,
    RenameParams, TextEdit, WorkspaceEdit,
};

use crate::client::LspClient;

// ── Completion ────────────────────────────────────────────────────────────────

/// Request completion items at the given cursor position.
///
/// Returns `None` when the language server provides no completions.
pub async fn get_completions(
    client: &mut LspClient,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    client.completion(params).await
}

// ── Hover ─────────────────────────────────────────────────────────────────────

/// Fetch hover information (type signature, documentation) at a position.
pub async fn get_hover(
    client: &mut LspClient,
    params: HoverParams,
) -> Result<Option<Hover>> {
    client.hover(params).await
}

// ── Go-to-definition ─────────────────────────────────────────────────────────

/// Resolve the definition location(s) for the symbol at a position.
pub async fn goto_definition(
    client: &mut LspClient,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    client.goto_definition(params).await
}

// ── Document symbols ─────────────────────────────────────────────────────────

/// List all symbols (functions, classes, variables, …) in a document.
pub async fn get_document_symbols(
    client: &mut LspClient,
    params: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>> {
    client.document_symbols(params).await
}

// ── Formatting ────────────────────────────────────────────────────────────────

/// Request full-document formatting edits from the language server.
pub async fn format_document(
    client: &mut LspClient,
    params: DocumentFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    client.formatting(params).await
}

// ── References ───────────────────────────────────────────────────────────────

/// Find all references to the symbol at a position.
pub async fn find_references(
    client: &mut LspClient,
    params: ReferenceParams,
) -> Result<Option<Vec<Location>>> {
    client.references(params).await
}

// ── Rename ────────────────────────────────────────────────────────────────────

/// Compute workspace-wide rename edits for the symbol at a position.
pub async fn rename_symbol(
    client: &mut LspClient,
    params: RenameParams,
) -> Result<Option<WorkspaceEdit>> {
    client.rename(params).await
}
