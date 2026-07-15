//! End-to-end HTTP tests driving the router in-process with `tower::oneshot`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use fluxo_engine::Engine;
use fluxo_store::memory::MemoryStore;
use fluxo_server::router;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(match body {
            Some(v) => Body::from(v.to_string()),
            None => Body::empty(),
        })
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body");
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

#[tokio::test]
async fn register_execute_poll_complete() {
    let app = router(Arc::new(Engine::new(MemoryStore::new())));

    // Register.
    let (status, _) = send(
        &app,
        "POST",
        "/workflow",
        Some(json!({
            "name": "demo",
            "tasks": [ { "name": "do_work", "taskReferenceName": "w" } ],
            "outputParameters": { "done": "${w.output.ok}" }
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Execute.
    let (status, body) = send(&app, "POST", "/workflow/demo/execute", Some(json!({ "input": {} }))).await;
    assert_eq!(status, StatusCode::OK);
    let workflow_id = body["workflowId"].as_str().expect("workflowId").to_string();

    // Poll the worker task by name.
    let (status, body) = send(&app, "GET", "/tasks/poll/do_work?workerId=w1", None).await;
    assert_eq!(status, StatusCode::OK);
    let task_id = body["task"]["taskId"].as_str().expect("taskId").to_string();
    assert_eq!(body["task"]["referenceName"], json!("w"));

    // A second poll finds nothing.
    let (status, _) = send(&app, "GET", "/tasks/poll/do_work?workerId=w1", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Complete the task.
    let (status, _) = send(
        &app,
        "POST",
        &format!("/tasks/{task_id}/complete"),
        Some(json!({ "workflowId": workflow_id, "output": { "ok": true } })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The run is now complete with the mapped output.
    let (status, body) = send(&app, "GET", &format!("/workflow/run/{workflow_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], json!("COMPLETED"));
    assert_eq!(body["output"], json!({ "done": true }));
}

#[tokio::test]
async fn missing_run_is_404() {
    let app = router(Arc::new(Engine::new(MemoryStore::new())));
    let (status, _) = send(&app, "GET", "/workflow/run/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
