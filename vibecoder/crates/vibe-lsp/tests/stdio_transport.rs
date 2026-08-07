//! End-to-end transport tests against a real stdio language server.
//!
//! The bug these exist to prevent was invisible to unit tests: the client used
//! to funnel every inbound message into a 32-slot channel that only an
//! in-flight request drained, so a server that talks while idle (all of them)
//! filled the channel, wedged the reader, and stopped draining the server's
//! stdout — after which IntelliSense was dead for the rest of the session.
//! Nothing but an actual pipe to an actual process reproduces that.
//!
//! The fixture server is a small Python script written at test time. If
//! `python3` is not available the tests report a skip rather than failing —
//! see `fixture_server()`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use vibe_lsp::client::LspClient;
use vibe_lsp::discovery::{resolve_server, ServerSearchPaths};

/// A scripted LSP server. Only the parts the tests exercise, but the framing
/// and the message flow are the real thing.
const FAKE_SERVER: &str = r#"
import sys, json

def read_message():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        line = line.strip()
        if not line:
            break
        key, _, value = line.decode("utf-8").partition(":")
        headers[key.lower().strip()] = value.strip()
    length = headers.get("content-length")
    if length is None:
        return None
    return json.loads(sys.stdin.buffer.read(int(length)))

def send(obj):
    body = json.dumps(obj).encode("utf-8")
    sys.stdout.buffer.write(b"Content-Length: %d\r\n\r\n" % len(body))
    sys.stdout.buffer.write(body)
    sys.stdout.buffer.flush()

def diagnostic(uri, message, severity=1):
    send({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {
        "uri": uri,
        "diagnostics": [{
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}},
            "severity": severity,
            "message": message,
        }],
    }})

# uri -> latest text the client has synced to us.
documents = {}
capabilities_asked = False

while True:
    msg = read_message()
    if msg is None:
        break
    method = msg.get("method")
    mid = msg.get("id")

    # A reply to a request *we* made.
    if method is None and mid is not None:
        diagnostic("file:///fake/answered", "client answered %s: %s" % (mid, json.dumps(msg.get("result"))), 3)
        continue

    if method == "initialize":
        send({"jsonrpc": "2.0", "id": mid, "result": {"capabilities": {
            "textDocumentSync": 1,
            "completionProvider": {"triggerCharacters": [".", "::"], "resolveProvider": True},
            "signatureHelpProvider": {"triggerCharacters": ["(", ","]},
            "hoverProvider": True,
            "definitionProvider": True,
        }}})
    elif method == "initialized":
        # Talk a lot, with no request in flight. Two thresholds must be crossed
        # for this to reproduce the original deadlock: more messages than any
        # bounded inbound queue holds, AND more bytes than the OS pipe buffer,
        # so a client that stops draining stdout blocks the server's writes.
        padding = "x" * 8192
        for i in range(200):
            send({"jsonrpc": "2.0", "method": "$/progress",
                  "params": {"token": i, "value": {"kind": "report", "message": "indexing " + padding}}})
        # And ask the client something: an unanswered server request stalls
        # servers that gate their work on configuration.
        send({"jsonrpc": "2.0", "id": 9001, "method": "workspace/configuration",
              "params": {"items": [{"section": "fake"}]}})
    elif method == "textDocument/didOpen":
        doc = msg["params"]["textDocument"]
        documents[doc["uri"]] = doc["text"]
        diagnostic(doc["uri"], "opened v%s" % doc["version"])
    elif method == "textDocument/didChange":
        uri = msg["params"]["textDocument"]["uri"]
        version = msg["params"]["textDocument"]["version"]
        documents[uri] = msg["params"]["contentChanges"][-1]["text"]
        diagnostic(uri, "changed to v%s" % version)
    elif method == "textDocument/didClose":
        documents.pop(msg["params"]["textDocument"]["uri"], None)
    elif method == "textDocument/completion":
        uri = msg["params"]["textDocument"]["uri"]
        if uri not in documents:
            send({"jsonrpc": "2.0", "id": mid,
                  "error": {"code": -32602, "message": "document not open"}})
        else:
            # Echo the text we currently hold, so a test can prove that edits
            # reached the server before completion was asked for.
            send({"jsonrpc": "2.0", "id": mid, "result": {"isIncomplete": False, "items": [
                {"label": documents[uri], "kind": 6},
                {"label": "snippet_item", "kind": 3, "insertText": "call(${1:arg})",
                 "insertTextFormat": 2, "detail": "fn call(arg)"},
            ]}})
    elif method == "textDocument/hover":
        # line 999 is the "never answers" probe for the timeout test.
        if msg["params"]["position"]["line"] == 999:
            continue
        send({"jsonrpc": "2.0", "id": mid, "result": {
            "contents": {"kind": "markdown", "value": "hover at line %s" % msg["params"]["position"]["line"]}}})
    elif method == "textDocument/definition":
        send({"jsonrpc": "2.0", "id": mid, "result": {
            "uri": msg["params"]["textDocument"]["uri"],
            "range": {"start": {"line": 4, "character": 2}, "end": {"line": 4, "character": 8}}}})
    elif method == "completionItem/resolve":
        item = dict(msg["params"] if isinstance(msg.get("params"), dict) else {})
        item["documentation"] = {"kind": "markdown", "value": "resolved docs"}
        send({"jsonrpc": "2.0", "id": mid, "result": item})
    elif method == "shutdown":
        send({"jsonrpc": "2.0", "id": mid, "result": None})
    elif method == "exit":
        break
    elif mid is not None:
        send({"jsonrpc": "2.0", "id": mid,
              "error": {"code": -32601, "message": "fake server: %s" % method}})
"#;

/// Locate `python3` and write the fixture script to a temp dir.
/// `None` when python3 is unavailable — the caller reports a skip.
///
/// Written exactly once per process: every test in this file shares the path,
/// and concurrent `fs::write` calls truncate the file another test's `python3`
/// is in the middle of reading. `OnceLock` serialises the setup, so the script
/// is complete before any server starts. (The first version of this file
/// re-wrote it per test and flaked roughly one run in three.)
fn fixture_server() -> Option<(PathBuf, PathBuf)> {
    static FIXTURE: std::sync::OnceLock<Option<(PathBuf, PathBuf)>> = std::sync::OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let search = ServerSearchPaths::from_env();
            let python = resolve_server("python3", &search)?;
            // Process id keeps concurrent `cargo test` runs off each other's file.
            let dir = std::env::temp_dir().join(format!("vibe-lsp-stdio-{}", std::process::id()));
            std::fs::create_dir_all(&dir).ok()?;
            let script = dir.join("fake_lsp_server.py");
            std::fs::write(&script, FAKE_SERVER).ok()?;
            Some((python, script))
        })
        .clone()
}

/// A started, initialized client talking to the fixture server.
async fn connected() -> Option<LspClient> {
    let (python, script) = fixture_server()?;
    let mut client = LspClient::new(
        python.to_string_lossy().to_string(),
        vec![script.to_string_lossy().to_string()],
    );
    client
        .initialize(PathBuf::from("/tmp/fake-workspace"))
        .await
        .expect("fixture server must complete initialize");
    Some(client)
}

macro_rules! client_or_skip {
    () => {
        match connected().await {
            Some(client) => client,
            None => {
                eprintln!("SKIPPED: python3 not available, stdio transport not exercised");
                return;
            }
        }
    };
}

const DOC: &str = "file:///tmp/fake-workspace/main.rs";

/// Poll until `check` passes or the deadline elapses. Notifications are
/// inherently asynchronous; a fixed sleep would be either flaky or slow.
async fn wait_for<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for {what}");
}

#[tokio::test]
async fn initialize_captures_server_capabilities() {
    let client = client_or_skip!();
    assert!(client.is_initialized());
    assert!(client.is_alive());

    let caps = client
        .server_capabilities()
        .await
        .expect("capabilities recorded");
    assert_eq!(caps["hoverProvider"], true);
    assert_eq!(
        client.completion_trigger_characters().await,
        vec![".".to_string(), "::".to_string()],
        "trigger characters drive Monaco's re-trigger behaviour"
    );
    assert_eq!(
        client.signature_help_trigger_characters().await,
        vec!["(".to_string(), ",".to_string()]
    );
}

#[tokio::test]
async fn completion_still_works_after_the_server_floods_notifications() {
    // The regression: 200 unsolicited messages with nothing in flight used to
    // fill a 32-slot queue, wedge the reader, and kill the connection.
    let client = client_or_skip!();

    wait_for("the notification flood to be routed", || async {
        client.diagnostics_for("file:///fake/answered").await.is_some()
    })
    .await;

    client
        .open_document(DOC, "rust", "fn main() {}")
        .await
        .expect("didOpen");

    let response = client
        .completion(completion_params(DOC, 0, 0))
        .await
        .expect("completion must still work after the flood")
        .expect("some completions");

    let items = completion_items(response);
    assert!(
        items.iter().any(|label| label == "fn main() {}"),
        "server echoed its document text: {items:?}"
    );
    assert!(client.is_alive(), "connection survived the flood");
}

#[tokio::test]
async fn server_request_is_answered() {
    // The fixture only publishes this diagnostic after it receives our reply to
    // its `workspace/configuration` request.
    let client = client_or_skip!();
    wait_for("the server's configuration request to be answered", || async {
        client.diagnostics_for("file:///fake/answered").await.is_some()
    })
    .await;

    let published = client
        .diagnostics_for("file:///fake/answered")
        .await
        .expect("answered");
    let message = published[0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("client answered"), "{message}");
    assert!(
        message.contains("null"),
        "one null entry per requested section: {message}"
    );
}

#[tokio::test]
async fn edits_reach_the_server_before_the_next_completion() {
    // This is the user-visible behaviour the feature is for: completion has to
    // see what you just typed, not the file as it was when you opened it.
    let client = client_or_skip!();
    client
        .open_document(DOC, "rust", "let original = 1;")
        .await
        .expect("didOpen");

    for text in ["let edited = 2;", "let edited_again = 3;"] {
        client
            .change_document(DOC, "rust", text)
            .await
            .expect("didChange");

        let items = completion_items(
            client
                .completion(completion_params(DOC, 0, 0))
                .await
                .expect("completion")
                .expect("items"),
        );
        assert!(
            items.iter().any(|label| label == text),
            "completion answered against stale text; wanted {text:?}, got {items:?}"
        );
    }
}

#[tokio::test]
async fn document_versions_increase_monotonically() {
    // Servers discard a change whose version did not advance.
    let client = client_or_skip!();
    client
        .open_document(DOC, "rust", "v1")
        .await
        .expect("didOpen");

    let client = &client;
    for expected_version in 2..=4 {
        client
            .change_document(DOC, "rust", &format!("v{expected_version}"))
            .await
            .expect("didChange");
        let wanted = format!("changed to v{expected_version}");
        wait_for(&wanted.clone(), || {
            let wanted = wanted.clone();
            async move {
                client
                    .diagnostics_for(DOC)
                    .await
                    .and_then(|items| items.first().cloned())
                    .and_then(|item| item["message"].as_str().map(str::to_string))
                    == Some(wanted)
            }
        })
        .await;
    }
}

#[tokio::test]
async fn change_before_open_falls_back_to_did_open() {
    // A server that started after the tab was opened has never seen the
    // document; the fixture errors on completion for an unknown document.
    let client = client_or_skip!();
    client
        .change_document(DOC, "rust", "recovered")
        .await
        .expect("change on an unopened document self-heals");
    assert!(client.is_document_open(DOC).await);

    let items = completion_items(
        client
            .completion(completion_params(DOC, 0, 0))
            .await
            .expect("completion")
            .expect("items"),
    );
    assert!(items.iter().any(|label| label == "recovered"), "{items:?}");
}

#[tokio::test]
async fn reopening_a_document_does_not_duplicate_it() {
    let client = client_or_skip!();
    client.open_document(DOC, "rust", "first").await.expect("open");
    client.open_document(DOC, "rust", "second").await.expect("reopen");

    let items = completion_items(
        client
            .completion(completion_params(DOC, 0, 0))
            .await
            .expect("completion")
            .expect("items"),
    );
    assert!(
        items.iter().any(|label| label == "second"),
        "the reopen resynced instead of re-registering: {items:?}"
    );
}

#[tokio::test]
async fn diagnostics_are_captured_and_cleared_on_close() {
    let client = client_or_skip!();
    client
        .open_document(DOC, "rust", "fn main() {}")
        .await
        .expect("didOpen");

    wait_for("diagnostics for the opened document", || async {
        client.diagnostics_for(DOC).await.is_some()
    })
    .await;
    let items = client.diagnostics_for(DOC).await.expect("published");
    assert_eq!(items[0]["message"], "opened v1");
    assert_eq!(items[0]["severity"], 1);

    client.close_document(DOC).await.expect("didClose");
    assert_eq!(
        client.diagnostics_for(DOC).await,
        None,
        "a closed document's markers must not linger"
    );
    assert!(!client.is_document_open(DOC).await);
}

#[tokio::test]
async fn concurrent_requests_get_their_own_answers() {
    // Out-of-order replies used to be handed to whichever caller was waiting,
    // so hover could return a completion list.
    let client = client_or_skip!();
    client
        .open_document(DOC, "rust", "concurrent")
        .await
        .expect("didOpen");

    let (hover_a, hover_b, definition, completion) = tokio::join!(
        client.hover(hover_params(DOC, 7)),
        client.hover(hover_params(DOC, 11)),
        client.goto_definition(definition_params(DOC, 0)),
        client.completion(completion_params(DOC, 0, 0)),
    );

    assert_eq!(hover_text(hover_a.expect("hover a")), "hover at line 7");
    assert_eq!(hover_text(hover_b.expect("hover b")), "hover at line 11");
    assert!(definition.expect("definition").is_some());
    let items = completion_items(completion.expect("completion").expect("items"));
    assert!(items.iter().any(|label| label == "concurrent"), "{items:?}");
}

#[tokio::test]
async fn snippet_completions_arrive_with_their_insert_text_format() {
    // Monaco needs insertTextFormat==2 to insert `call(${1:arg})` as a snippet
    // instead of typing the placeholder syntax literally.
    let client = client_or_skip!();
    client.open_document(DOC, "rust", "x").await.expect("didOpen");

    let response = client
        .completion(completion_params(DOC, 0, 0))
        .await
        .expect("completion")
        .expect("items");
    let items = match response {
        lsp_types::CompletionResponse::List(list) => list.items,
        lsp_types::CompletionResponse::Array(items) => items,
    };
    let snippet = items
        .iter()
        .find(|item| item.label == "snippet_item")
        .expect("snippet item present");
    assert_eq!(
        snippet.insert_text_format,
        Some(lsp_types::InsertTextFormat::SNIPPET)
    );
    assert_eq!(snippet.insert_text.as_deref(), Some("call(${1:arg})"));
}

#[tokio::test]
async fn completion_items_can_be_resolved_for_documentation() {
    let client = client_or_skip!();
    let resolved = client
        .resolve_completion_item(serde_json::json!({ "label": "push", "kind": 2 }))
        .await
        .expect("resolve");
    assert_eq!(resolved["documentation"]["value"], "resolved docs");
}

#[tokio::test]
async fn an_unanswered_request_times_out_and_the_connection_keeps_working() {
    let client = client_or_skip!();
    client.set_request_timeout(Duration::from_millis(400));
    client.open_document(DOC, "rust", "alive").await.expect("didOpen");

    let started = Instant::now();
    let err = client
        .hover(hover_params(DOC, 999))
        .await
        .expect_err("the fixture never answers line 999");
    assert!(err.to_string().contains("did not answer"), "{err}");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "timeout must bound the wait, took {:?}",
        started.elapsed()
    );

    // The important half: a timed-out request must not desynchronise the
    // stream or leak its pending slot.
    let recovered = hover_text(client.hover(hover_params(DOC, 12)).await.expect("hover"));
    assert_eq!(recovered, "hover at line 12");
    assert!(client.is_alive());
}

#[tokio::test]
async fn requests_fail_fast_once_the_server_exits() {
    let client = client_or_skip!();
    client.set_request_timeout(Duration::from_secs(30));
    client.shutdown().await.expect("shutdown");

    wait_for("the client to notice the server is gone", || async {
        !client.is_alive()
    })
    .await;

    let started = Instant::now();
    let err = client
        .hover(hover_params(DOC, 1))
        .await
        .expect_err("server is gone");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "must not wait out the 30s timeout, took {:?}",
        started.elapsed()
    );
    assert!(
        err.to_string().contains("no longer running") || err.to_string().contains("exited"),
        "{err}"
    );
}

#[tokio::test]
async fn a_path_with_spaces_round_trips_as_a_uri() {
    // `format!("file://{path}")` produced an unparseable URI here, and the
    // server rejected every request for the document.
    let client = client_or_skip!();
    let path = Path::new("/tmp/fake-workspace/My Code/café.rs");
    let uri = vibe_lsp::path_to_uri(path);
    assert!(uri.contains("%20"), "spaces must be encoded: {uri}");

    client
        .open_document(&uri, "rust", "encoded ok")
        .await
        .expect("didOpen");
    let items = completion_items(
        client
            .completion(completion_params(&uri, 0, 0))
            .await
            .expect("completion")
            .expect("items"),
    );
    assert!(items.iter().any(|label| label == "encoded ok"), "{items:?}");

    // And the diagnostics the server published under that URI are findable.
    wait_for("diagnostics for the encoded URI", || async {
        client.diagnostics_for(&uri).await.is_some()
    })
    .await;
}

#[tokio::test]
async fn unknown_method_error_is_reported_not_swallowed() {
    let client = client_or_skip!();
    // The fixture rejects completion for a document it has never seen.
    let err = client
        .completion(completion_params("file:///tmp/never-opened.rs", 0, 0))
        .await
        .expect_err("server returns an LSP error");
    assert!(err.to_string().contains("document not open"), "{err}");
    assert!(err.to_string().contains("-32602"), "code is kept: {err}");
}

// ── Param helpers ───────────────────────────────────────────────────────────

fn text_document(uri: &str) -> lsp_types::TextDocumentIdentifier {
    lsp_types::TextDocumentIdentifier {
        uri: uri.parse().expect("valid URI"),
    }
}

fn completion_params(uri: &str, line: u32, character: u32) -> lsp_types::CompletionParams {
    lsp_types::CompletionParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: text_document(uri),
            position: lsp_types::Position { line, character },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    }
}

fn hover_params(uri: &str, line: u32) -> lsp_types::HoverParams {
    lsp_types::HoverParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: text_document(uri),
            position: lsp_types::Position { line, character: 0 },
        },
        work_done_progress_params: Default::default(),
    }
}

fn definition_params(uri: &str, line: u32) -> lsp_types::GotoDefinitionParams {
    lsp_types::GotoDefinitionParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: text_document(uri),
            position: lsp_types::Position { line, character: 0 },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    }
}

fn completion_items(response: lsp_types::CompletionResponse) -> Vec<String> {
    match response {
        lsp_types::CompletionResponse::Array(items) => {
            items.into_iter().map(|item| item.label).collect()
        }
        lsp_types::CompletionResponse::List(list) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
    }
}

fn hover_text(hover: Option<lsp_types::Hover>) -> String {
    match hover.expect("hover present").contents {
        lsp_types::HoverContents::Markup(markup) => markup.value,
        other => panic!("expected markup contents, got {other:?}"),
    }
}
