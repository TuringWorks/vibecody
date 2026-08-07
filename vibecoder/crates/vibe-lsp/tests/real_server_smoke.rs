//! Smoke tests against **real** language servers.
//!
//! `#[ignore]`d: they need `rust-analyzer` / `clangd` installed and take
//! seconds, so they stay out of `cargo test`. They are the only tests that
//! prove the handshake, the capabilities we declare, and full-text sync work
//! against a production server rather than a fixture.
//!
//! ```sh
//! cargo test -p vibe-lsp --test real_server_smoke -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;
use vibe_lsp::client::LspClient;
use vibe_lsp::discovery::{server_available, ServerSearchPaths};
use vibe_lsp::path_to_uri;

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vibe-lsp-real-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(path, contents).expect("write fixture");
}

fn completion_params(uri: &str, line: u32, character: u32) -> lsp_types::CompletionParams {
    lsp_types::CompletionParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: uri.parse().expect("valid uri"),
            },
            position: lsp_types::Position { line, character },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: Some(lsp_types::CompletionContext {
            trigger_kind: lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    }
}

fn labels(response: Option<lsp_types::CompletionResponse>) -> Vec<String> {
    match response {
        Some(lsp_types::CompletionResponse::Array(items)) => {
            items.into_iter().map(|item| item.label).collect()
        }
        Some(lsp_types::CompletionResponse::List(list)) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
        None => Vec::new(),
    }
}

/// Retry completion until `ready` accepts the result, or attempts run out.
///
/// A cold server answers *something* long before it can answer *well*:
/// rust-analyzer returns only postfix snippets until the crate graph is loaded,
/// and clangd falls back to identifier matching until its preamble is built.
/// Waiting for a specific expected label is the only way to tell "indexing" from
/// "type resolution is broken".
async fn completion_until(
    client: &LspClient,
    uri: &str,
    line: u32,
    character: u32,
    attempts: usize,
    ready: impl Fn(&[String]) -> bool,
) -> Vec<String> {
    let mut last = Vec::new();
    for attempt in 0..attempts {
        match client
            .completion(completion_params(uri, line, character))
            .await
        {
            Ok(response) => {
                last = labels(response);
                if ready(&last) {
                    return last;
                }
            }
            Err(e) => eprintln!("attempt {attempt}: {e}"),
        }
        tokio::time::sleep(Duration::from_millis(750)).await;
    }
    last
}

/// Servers dress labels up: clangd pads for column alignment (`" x"`) and
/// rust-analyzer appends a call hint (`"push_str(…)"`). Compare on the name.
fn has_label(labels: &[String], wanted: &str) -> bool {
    labels.iter().any(|label| {
        let name = label.trim();
        name == wanted || name.starts_with(&format!("{wanted}("))
    })
}

#[tokio::test]
#[ignore = "needs rust-analyzer installed; takes ~10-30s"]
async fn rust_analyzer_completes_after_an_edit() {
    let search = ServerSearchPaths::from_env();
    if !server_available("rust-analyzer", &search) {
        eprintln!("SKIPPED: rust-analyzer not installed");
        return;
    }

    let root = scratch_dir("rust");
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let main = root.join("src/main.rs");
    let original = "fn main() {\n    let text = String::new();\n}\n";
    write(&main, original);

    let mut client = LspClient::new("rust-analyzer".to_string(), vec![]);
    client
        .initialize(root.clone())
        .await
        .expect("rust-analyzer initialize");
    client.set_request_timeout(Duration::from_secs(20));

    let uri = path_to_uri(&main);
    client
        .open_document(&uri, "rust", original)
        .await
        .expect("didOpen");

    // The edit only exists in the editor — it was never written to disk. This
    // is the case that used to fail: without didChange the server answers
    // against the on-disk text and knows nothing about `text.`.
    let edited = "fn main() {\n    let text = String::new();\n    text.\n}\n";
    client
        .change_document(&uri, "rust", edited)
        .await
        .expect("didChange");

    // Line 2 (0-based), just after `text.`
    let found = completion_until(&client, &uri, 2, 9, 60, |labels| {
        has_label(labels, "push_str")
    })
    .await;
    println!("rust-analyzer returned {} items", found.len());
    assert!(
        has_label(&found, "push_str"),
        "expected String methods for `text.` — the server is answering, but not \
         against the edited text. Got: {:?}",
        &found.iter().take(20).collect::<Vec<_>>()
    );

    let caps = client
        .server_capabilities()
        .await
        .expect("capabilities recorded");
    assert!(caps.get("completionProvider").is_some());
    let triggers = client.completion_trigger_characters().await;
    println!("rust-analyzer trigger characters: {triggers:?}");
    assert!(
        triggers.iter().any(|t| t == "."),
        "`.` must re-trigger completion: {triggers:?}"
    );

    client.shutdown().await.expect("shutdown");
}

#[tokio::test]
#[ignore = "needs rust-analyzer installed; takes ~10-30s"]
async fn rust_analyzer_publishes_diagnostics() {
    let search = ServerSearchPaths::from_env();
    if !server_available("rust-analyzer", &search) {
        eprintln!("SKIPPED: rust-analyzer not installed");
        return;
    }

    let root = scratch_dir("diag");
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let main = root.join("src/main.rs");
    let broken = "fn main() {\n    let x: i32 = \"not an integer\";\n}\n";
    write(&main, broken);

    let mut client = LspClient::new("rust-analyzer".to_string(), vec![]);
    client.initialize(root.clone()).await.expect("initialize");
    let uri = path_to_uri(&main);
    client
        .open_document(&uri, "rust", broken)
        .await
        .expect("didOpen");

    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(items) = client.diagnostics_for(&uri).await {
            if !items.is_empty() {
                println!("diagnostics: {:?}", items[0]["message"]);
                break;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "rust-analyzer published no diagnostics for an obvious type error"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    client.shutdown().await.expect("shutdown");
}

#[tokio::test]
#[ignore = "needs clangd installed"]
async fn clangd_completes_after_an_edit() {
    let search = ServerSearchPaths::from_env();
    if !server_available("clangd", &search) {
        eprintln!("SKIPPED: clangd not installed");
        return;
    }

    let root = scratch_dir("c");
    let main = root.join("main.c");
    let original = "#include <stdio.h>\nint main(void) {\n  return 0;\n}\n";
    write(&main, original);

    let mut client = LspClient::new("clangd".to_string(), vec![]);
    client.initialize(root.clone()).await.expect("initialize");
    client.set_request_timeout(Duration::from_secs(20));

    let uri = path_to_uri(&main);
    client
        .open_document(&uri, "c", original)
        .await
        .expect("didOpen");

    let edited =
        "#include <stdio.h>\nstruct Point { int x; int y; };\nint main(void) {\n  struct Point p;\n  p.\n  return 0;\n}\n";
    client
        .change_document(&uri, "c", edited)
        .await
        .expect("didChange");

    // Line 4 (0-based), just after `p.`. Only member completion can produce a
    // list this short — clangd's identifier fallback returns every token in the
    // file, so a small list is itself the evidence that the type resolved.
    let found = completion_until(&client, &uri, 4, 4, 40, |labels| {
        has_label(labels, "x") && has_label(labels, "y") && labels.len() <= 4
    })
    .await;
    println!("clangd returned {} items: {found:?}", found.len());
    assert!(
        has_label(&found, "x") && has_label(&found, "y") && found.len() <= 4,
        "expected member completion for `p.` (just the struct's fields), got: {:?}",
        &found.iter().take(20).collect::<Vec<_>>()
    );

    client.shutdown().await.expect("shutdown");
}
