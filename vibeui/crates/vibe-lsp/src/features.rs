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

    // ── Diagnostic tests ────────────────────────────────────────────────────

    #[test]
    fn diagnostic_error_can_be_constructed() {
        let diag = Diagnostic {
            range: Range::new(Position::new(5, 0), Position::new(5, 10)),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("E0308".to_string())),
            source: Some("rustc".to_string()),
            message: "mismatched types".to_string(),
            ..Default::default()
        };
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diag.message, "mismatched types");
        assert_eq!(diag.source.as_deref(), Some("rustc"));
    }

    #[test]
    fn diagnostic_warning_serializes() {
        let diag = Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 5)),
            severity: Some(DiagnosticSeverity::WARNING),
            message: "unused variable".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_value(&diag).unwrap();
        assert_eq!(json["message"], "unused variable");
        assert_eq!(json["severity"], 2); // WARNING = 2
    }

    #[test]
    fn diagnostic_without_severity_defaults_to_none() {
        let diag = Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            message: "info".to_string(),
            ..Default::default()
        };
        assert!(diag.severity.is_none());
    }

    // ── Completion item tests ───────────────────────────────────────────────

    #[test]
    fn completion_item_function_kind() {
        let item = CompletionItem {
            label: "my_function".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("fn my_function() -> bool".to_string()),
            ..Default::default()
        };
        assert_eq!(item.label, "my_function");
        assert_eq!(item.kind, Some(CompletionItemKind::FUNCTION));
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["label"], "my_function");
        assert_eq!(json["kind"], 3); // FUNCTION = 3
    }

    #[test]
    fn completion_item_with_insert_text() {
        let item = CompletionItem {
            label: "println!".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            insert_text: Some("println!(\"$1\")$0".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        };
        assert_eq!(item.insert_text.as_deref(), Some("println!(\"$1\")$0"));
        assert_eq!(item.insert_text_format, Some(InsertTextFormat::SNIPPET));
    }

    // ── Symbol information tests ────────────────────────────────────────────

    #[test]
    fn symbol_information_can_be_constructed() {
        let uri: Uri = "file:///tmp/test.rs".parse().unwrap();
        #[allow(deprecated)]
        let sym = SymbolInformation {
            name: "MyStruct".to_string(),
            kind: SymbolKind::STRUCT,
            tags: None,
            deprecated: None,
            location: Location::new(
                uri,
                Range::new(Position::new(10, 0), Position::new(20, 1)),
            ),
            container_name: Some("my_module".to_string()),
        };
        assert_eq!(sym.name, "MyStruct");
        assert_eq!(sym.kind, SymbolKind::STRUCT);
        assert_eq!(sym.container_name.as_deref(), Some("my_module"));
    }

    // ── Range / Position utility tests ──────────────────────────────────────

    #[test]
    fn range_single_line() {
        let range = Range::new(Position::new(3, 5), Position::new(3, 15));
        assert_eq!(range.start.line, range.end.line);
        assert_eq!(range.end.character - range.start.character, 10);
    }

    #[test]
    fn range_multiline_serialization() {
        let range = Range::new(Position::new(0, 0), Position::new(100, 0));
        let json = serde_json::to_value(&range).unwrap();
        assert_eq!(json["start"]["line"], 0);
        assert_eq!(json["end"]["line"], 100);
    }

    // ── WorkspaceEdit tests ─────────────────────────────────────────────────

    #[test]
    fn workspace_edit_with_changes() {
        let uri: Uri = "file:///tmp/test.rs".parse().unwrap();
        let edit = TextEdit::new(
            Range::new(Position::new(0, 0), Position::new(0, 3)),
            "new_name".to_string(),
        );
        let mut changes = std::collections::HashMap::new();
        changes.insert(uri, vec![edit]);
        let ws_edit = WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        };
        assert!(ws_edit.changes.is_some());
        let c = ws_edit.changes.unwrap();
        assert_eq!(c.len(), 1);
    }

    // ── Location tests ──────────────────────────────────────────────────────

    #[test]
    fn location_roundtrip_serialization() {
        let uri: Uri = "file:///src/main.rs".parse().unwrap();
        let loc = Location::new(
            uri,
            Range::new(Position::new(42, 4), Position::new(42, 20)),
        );
        let json = serde_json::to_value(&loc).unwrap();
        let deserialized: Location = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.uri.as_str(), "file:///src/main.rs");
        assert_eq!(deserialized.range.start.line, 42);
        assert_eq!(deserialized.range.start.character, 4);
    }

    // ── Diagnostic severity level tests ──────────────────────────────────

    #[test]
    fn diagnostic_severity_information() {
        let diag = Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            severity: Some(DiagnosticSeverity::INFORMATION),
            message: "info message".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_value(&diag).unwrap();
        assert_eq!(json["severity"], 3); // INFORMATION = 3
    }

    #[test]
    fn diagnostic_severity_hint() {
        let diag = Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            severity: Some(DiagnosticSeverity::HINT),
            message: "hint".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_value(&diag).unwrap();
        assert_eq!(json["severity"], 4); // HINT = 4
    }

    #[test]
    fn diagnostic_with_related_information() {
        let uri: Uri = "file:///tmp/test.rs".parse().unwrap();
        let related = DiagnosticRelatedInformation {
            location: Location::new(
                uri,
                Range::new(Position::new(1, 0), Position::new(1, 10)),
            ),
            message: "defined here".to_string(),
        };
        let diag = Diagnostic {
            range: Range::new(Position::new(5, 0), Position::new(5, 10)),
            severity: Some(DiagnosticSeverity::ERROR),
            message: "type mismatch".to_string(),
            related_information: Some(vec![related]),
            ..Default::default()
        };
        assert_eq!(diag.related_information.as_ref().unwrap().len(), 1);
        assert_eq!(diag.related_information.unwrap()[0].message, "defined here");
    }

    // ── Completion item kind tests ───────────────────────────────────────

    #[test]
    fn completion_item_variable_kind() {
        let item = CompletionItem {
            label: "my_var".to_string(),
            kind: Some(CompletionItemKind::VARIABLE),
            ..Default::default()
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["kind"], 6); // VARIABLE = 6
    }

    #[test]
    fn completion_item_class_kind() {
        let item = CompletionItem {
            label: "MyClass".to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some("class MyClass".to_string()),
            ..Default::default()
        };
        assert_eq!(item.kind, Some(CompletionItemKind::CLASS));
        assert_eq!(item.detail.as_deref(), Some("class MyClass"));
    }

    // ── TextEdit range tests ─────────────────────────────────────────────

    #[test]
    fn text_edit_multiline_range() {
        let edit = TextEdit::new(
            Range::new(Position::new(5, 0), Position::new(10, 0)),
            "replaced block".to_string(),
        );
        assert_eq!(edit.range.start.line, 5);
        assert_eq!(edit.range.end.line, 10);
        assert_eq!(edit.new_text, "replaced block");
    }

    #[test]
    fn text_edit_empty_replacement() {
        let edit = TextEdit::new(
            Range::new(Position::new(0, 0), Position::new(0, 5)),
            String::new(),
        );
        assert!(edit.new_text.is_empty(), "empty replacement means deletion");
    }

    // ── Completion response variants ─────────────────────────────────────

    #[test]
    fn completion_response_list_variant() {
        let list = CompletionList {
            is_incomplete: true,
            items: vec![
                CompletionItem { label: "item1".to_string(), ..Default::default() },
                CompletionItem { label: "item2".to_string(), ..Default::default() },
            ],
        };
        let resp = CompletionResponse::List(list);
        match resp {
            CompletionResponse::List(l) => {
                assert!(l.is_incomplete);
                assert_eq!(l.items.len(), 2);
            }
            _ => panic!("expected List variant"),
        }
    }

    #[test]
    fn completion_response_array_variant() {
        let items = vec![
            CompletionItem { label: "a".to_string(), ..Default::default() },
        ];
        let resp = CompletionResponse::Array(items);
        match resp {
            CompletionResponse::Array(arr) => assert_eq!(arr.len(), 1),
            _ => panic!("expected Array variant"),
        }
    }
}
