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

#[cfg(test)]
mod tests {
    //! These tests verify the LSP feature module's type compatibility and
    //! constructor helpers.  The actual async functions delegate to `LspClient`
    //! and require a running language server, so they are not called here.

    use lsp_types::*;

    // ── Type construction tests ─────────────────────────────────────────────
    // Ensures the LSP types used in the feature API can be constructed and
    // serialised without panicking, catching any breaking upstream changes.

    fn make_text_document_identifier() -> TextDocumentIdentifier {
        let uri: Uri = "file:///tmp/test.rs".parse().unwrap();
        TextDocumentIdentifier::new(uri)
    }

    fn make_position() -> Position {
        Position::new(10, 5)
    }

    #[test]
    fn completion_params_can_be_constructed() {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams::new(
                make_text_document_identifier(),
                make_position(),
            ),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };
        let json = serde_json::to_value(&params);
        assert!(json.is_ok());
    }

    #[test]
    fn hover_params_can_be_constructed() {
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams::new(
                make_text_document_identifier(),
                make_position(),
            ),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let json = serde_json::to_value(&params);
        assert!(json.is_ok());
    }

    #[test]
    fn goto_definition_params_can_be_constructed() {
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams::new(
                make_text_document_identifier(),
                make_position(),
            ),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let json = serde_json::to_value(&params);
        assert!(json.is_ok());
    }

    #[test]
    fn document_symbol_params_can_be_constructed() {
        let params = DocumentSymbolParams {
            text_document: make_text_document_identifier(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let json = serde_json::to_value(&params);
        assert!(json.is_ok());
    }

    #[test]
    fn formatting_params_can_be_constructed() {
        let params = DocumentFormattingParams {
            text_document: make_text_document_identifier(),
            options: FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let json = serde_json::to_value(&params);
        assert!(json.is_ok());
    }

    #[test]
    fn reference_params_can_be_constructed() {
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams::new(
                make_text_document_identifier(),
                make_position(),
            ),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };
        let json = serde_json::to_value(&params);
        assert!(json.is_ok());
    }

    #[test]
    fn rename_params_can_be_constructed() {
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams::new(
                make_text_document_identifier(),
                make_position(),
            ),
            new_name: "new_name".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let json = serde_json::to_value(&params);
        assert!(json.is_ok());
    }

    #[test]
    fn position_fields_are_correct() {
        let pos = make_position();
        assert_eq!(pos.line, 10);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn text_document_identifier_uri() {
        let tdi = make_text_document_identifier();
        assert_eq!(tdi.uri.as_str(), "file:///tmp/test.rs");
    }

    #[test]
    fn text_edit_can_be_constructed() {
        let edit = TextEdit::new(
            Range::new(Position::new(0, 0), Position::new(0, 5)),
            "replacement".to_string(),
        );
        assert_eq!(edit.new_text, "replacement");
    }
}
