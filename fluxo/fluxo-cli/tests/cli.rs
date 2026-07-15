//! Drives the compiled `fluxo` binary as a subprocess against an in-process server.

use fluxo_engine::Engine;
use fluxo_store::memory::MemoryStore;
use serde_json::Value;
use std::sync::Arc;
use tokio::process::Command;

async fn spawn_server() -> String {
    let engine = Arc::new(Engine::new(MemoryStore::new()));
    let app = fluxo_server::router(engine);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{addr}")
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_fluxo")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_ls_run_get() {
    let url = spawn_server().await;
    let def_path = std::env::temp_dir().join(format!("fluxo_cli_{}.json", std::process::id()));
    std::fs::write(
        &def_path,
        r#"{"name":"demo","tasks":[{"name":"do_work","taskReferenceName":"w"}]}"#,
    )
    .expect("write def");

    // register
    let out = Command::new(bin())
        .args(["--url", &url, "register", def_path.to_str().unwrap()])
        .output()
        .await
        .expect("register");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).contains("\"demo\""));

    // ls
    let out = Command::new(bin()).args(["--url", &url, "ls"]).output().await.expect("ls");
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("demo"));

    // run (no --wait, so it stays RUNNING waiting on the worker task)
    let out = Command::new(bin())
        .args(["--url", &url, "run", "demo"])
        .output()
        .await
        .expect("run");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let started: Value = serde_json::from_slice(&out.stdout).expect("run json");
    let id = started["workflowId"].as_str().expect("workflowId").to_string();

    // get
    let out = Command::new(bin())
        .args(["--url", &url, "get", &id])
        .output()
        .await
        .expect("get");
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("RUNNING"));

    let _ = std::fs::remove_file(&def_path);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn errors_surface_nonzero_exit() {
    let url = spawn_server().await;
    // Getting a nonexistent run should exit non-zero.
    let out = Command::new(bin())
        .args(["--url", &url, "get", "does-not-exist"])
        .output()
        .await
        .expect("get");
    assert!(!out.status.success());
}
