//! BDD coverage for A2A over real HTTP + SSE (US-002).

use cucumber::{World, given, then, when};
use std::sync::Arc;
use tokio::sync::Mutex;
use vibecli_cli::a2a_http::{A2aHttpClient, ServerHandle, serve_agent};
use vibecli_cli::a2a_protocol::{
    A2aEventType, AgentCapability, AgentCard, A2aServer, TaskInput,
};

#[derive(Default, World)]
pub struct A2aWorld {
    server: Option<ServerHandle>,
    server_state: Option<Arc<Mutex<A2aServer>>>,
    fetched_card: Option<AgentCard>,
    last_task_id: Option<String>,
    events: Vec<(String, String)>,
}

impl std::fmt::Debug for A2aWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("A2aWorld")
            .field("server_addr", &self.server.as_ref().map(|s| s.addr))
            .field("fetched_card", &self.fetched_card.as_ref().map(|c| c.name.clone()))
            .field("last_task_id", &self.last_task_id)
            .field("events_count", &self.events.len())
            .finish()
    }
}

fn card_for(name: &str) -> AgentCard {
    AgentCard::new(name, "test agent", "http://placeholder", "1.0.0")
        .with_capabilities(vec![AgentCapability::CodeGeneration])
}

// ── Given ───────────────────────────────────────────────────────────────────

#[given(regex = r#"^an A2A HTTP server hosting an agent named "([^"]+)" on a random port$"#)]
async fn given_server(w: &mut A2aWorld, name: String) {
    let inner = A2aServer::new("127.0.0.1", 0, card_for(&name));
    let state = Arc::new(Mutex::new(inner));
    let handle = serve_agent(state.clone()).await.expect("serve");
    w.server_state = Some(state);
    w.server = Some(handle);
}

// ── When ────────────────────────────────────────────────────────────────────

#[when(regex = r#"^a client fetches the agent card from that server$"#)]
async fn when_fetch_card(w: &mut A2aWorld) {
    let server = w.server.as_ref().expect("server");
    let client = A2aHttpClient::new(reqwest::Client::new());
    let card = client.fetch_card(&server.base_url()).await.expect("fetch");
    w.fetched_card = Some(card);
}

#[when(regex = r#"^a client submits a text task with content "([^"]+)"$"#)]
async fn when_submit_task(w: &mut A2aWorld, content: String) {
    let server = w.server.as_ref().expect("server");
    let client = A2aHttpClient::new(reqwest::Client::new());
    let id = client
        .submit_task(&server.base_url(), TaskInput::text(&content))
        .await
        .expect("submit");
    w.last_task_id = Some(id);
}

#[when(regex = r#"^the client reads at most (\d+) SSE events from the server$"#)]
async fn when_read_events(w: &mut A2aWorld, n: usize) {
    let server = w.server.as_ref().expect("server");
    let client = A2aHttpClient::new(reqwest::Client::new());
    let events = client
        .read_events(&server.base_url(), n)
        .await
        .expect("read events");
    w.events = events;
}

// ── Then ────────────────────────────────────────────────────────────────────

#[then(regex = r#"^the fetched card name is "([^"]+)"$"#)]
fn then_card_name(w: &mut A2aWorld, name: String) {
    let card = w.fetched_card.as_ref().expect("card");
    assert_eq!(card.name, name);
}

#[then(regex = r#"^the fetched card has capability "([^"]+)"$"#)]
fn then_card_capability(w: &mut A2aWorld, cap: String) {
    let card = w.fetched_card.as_ref().expect("card");
    assert!(
        card.capabilities.iter().any(|c| c.as_str() == cap),
        "capabilities {:?} missing {cap}",
        card.capabilities
    );
}

#[then(regex = r#"^the returned task id starts with "([^"]+)"$"#)]
fn then_task_id_prefix(w: &mut A2aWorld, prefix: String) {
    let id = w.last_task_id.as_ref().expect("task id");
    assert!(id.starts_with(&prefix), "id {id} !startswith {prefix}");
}

#[then(regex = r#"^the client can GET the task and its status is "([^"]+)"$"#)]
async fn then_task_status(w: &mut A2aWorld, status: String) {
    let server = w.server.as_ref().expect("server");
    let client = A2aHttpClient::new(reqwest::Client::new());
    let id = w.last_task_id.as_ref().expect("task id");
    let task = client
        .get_task(&server.base_url(), id)
        .await
        .expect("get_task");
    let got = format!("{:?}", task.status);
    assert!(
        got.contains(&status),
        "expected status {status}, got {got}"
    );
}

#[then(regex = r#"^the received events include a "([^"]+)" event$"#)]
fn then_events_include(w: &mut A2aWorld, kind: String) {
    assert!(
        w.events.iter().any(|(k, _)| k == &kind),
        "events {:?} missing kind {kind}",
        w.events
    );
    // sanity: kind should be a real A2aEventType
    let _ = match kind.as_str() {
        "TaskCreated" => A2aEventType::TaskCreated,
        "StatusChanged" => A2aEventType::StatusChanged,
        "OutputReady" => A2aEventType::OutputReady,
        "Error" => A2aEventType::Error,
        other => panic!("unknown A2a event kind in feature: {other}"),
    };
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    A2aWorld::run("tests/features/a2a_http.feature").await;
}
