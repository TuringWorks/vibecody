/*!
 * BDD tests for the typed in-process lifecycle event bus.
 * Run with: cargo test --test event_bus_bdd
 */
use cucumber::{given, then, when, World};
use std::sync::{Arc, Mutex};
use vibecli_cli::event_bus::{BusEvent, EventBus, EventFilter, HandlerDecision};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct EbWorld {
    bus: Option<EventBus>,
    last_subscription_id: u64,
    /// Events captured by observational subscribers.
    captured: Arc<Mutex<Vec<BusEvent>>>,
    /// Decision returned by the most recent emit call.
    last_decision: Option<HandlerDecision>,
    /// Priority values recorded by priority-recording handlers, in call order.
    priority_order: Arc<Mutex<Vec<i32>>>,
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn bus(world: &EbWorld) -> &EventBus {
    world.bus.as_ref().expect("EventBus not initialised — use 'Given a fresh EventBus'")
}

// ---------------------------------------------------------------------------
// Given steps
// ---------------------------------------------------------------------------

#[given("a fresh EventBus")]
fn fresh_bus(world: &mut EbWorld) {
    world.bus = Some(EventBus::with_history(64));
    world.captured = Arc::new(Mutex::new(Vec::new()));
    world.priority_order = Arc::new(Mutex::new(Vec::new()));
    world.last_decision = None;
    world.last_subscription_id = 0;
}

#[given(expr = "I subscribe with filter {string} at priority {int}")]
fn subscribe_with_filter(world: &mut EbWorld, filter_spec: String, priority: i32) {
    let filter = parse_filter(&filter_spec);
    let captured = Arc::clone(&world.captured);
    let id = bus(world).subscribe(filter, priority, move |e| {
        captured.lock().unwrap().push(e.clone());
        HandlerDecision::Continue
    });
    world.last_subscription_id = id;
}

#[given(expr = "I subscribe a blocking handler with reason {string} at priority {int}")]
fn subscribe_blocking(world: &mut EbWorld, reason: String, priority: i32) {
    let id = bus(world).subscribe(EventFilter::All, priority, move |_| {
        HandlerDecision::Block { reason: reason.clone() }
    });
    world.last_subscription_id = id;
}

#[given(expr = "I subscribe a priority-recording handler at priority {int}")]
fn subscribe_priority_recorder(world: &mut EbWorld, priority: i32) {
    let order = Arc::clone(&world.priority_order);
    bus(world).subscribe(EventFilter::All, priority, move |_| {
        order.lock().unwrap().push(priority);
        HandlerDecision::Continue
    });
}

// ---------------------------------------------------------------------------
// When steps
// ---------------------------------------------------------------------------

#[when(expr = "I emit a {string} event with turn {int}")]
fn emit_turn_event(world: &mut EbWorld, event_type: String, turn: u32) {
    let event = match event_type.as_str() {
        "agent_start" => BusEvent::AgentStart { turn },
        "agent_end"   => BusEvent::AgentEnd { turn, tool_calls_made: 0 },
        other => panic!("Unknown turn event type: {}", other),
    };
    let decision = bus(world).emit(event);
    world.last_decision = Some(decision);
}


#[when("I emit a \"user_input\" event")]
fn emit_user_input(world: &mut EbWorld) {
    let decision = bus(world).emit(BusEvent::UserInput { content: "test input".into() });
    world.last_decision = Some(decision);
}

#[when(expr = "I emit a {string} event for tool {string}")]
fn emit_tool_event(world: &mut EbWorld, event_type: String, tool: String) {
    let event = match event_type.as_str() {
        "tool_call" => BusEvent::ToolCall {
            call_id: "c1".into(),
            tool_name: tool,
            args_json: "{}".into(),
        },
        "tool_result" => BusEvent::ToolResult {
            call_id: "c1".into(),
            tool_name: tool,
            output: "ok".into(),
            exit_code: 0,
        },
        other => panic!("Unknown tool event type: {}", other),
    };
    let decision = bus(world).emit(event);
    world.last_decision = Some(decision);
}


#[when(expr = "I emit a {string} event with key {string}")]
fn emit_memory_event(world: &mut EbWorld, event_type: String, key: String) {
    let event = match event_type.as_str() {
        "memory_write"  => BusEvent::MemoryWrite  { key, value_preview: "...".into() },
        "memory_read"   => BusEvent::MemoryRead   { key },
        "memory_delete" => BusEvent::MemoryDelete { key },
        other => panic!("Unknown memory event type: {}", other),
    };
    let decision = bus(world).emit(event);
    world.last_decision = Some(decision);
}

#[when("I unsubscribe the last subscription")]
fn unsubscribe_last(world: &mut EbWorld) {
    let id = world.last_subscription_id;
    bus(world).unsubscribe(id);
}

// ---------------------------------------------------------------------------
// Then steps
// ---------------------------------------------------------------------------

#[then(expr = "the subscriber should have received {int} event")]
fn received_n_events_singular(world: &mut EbWorld, n: usize) {
    let got = world.captured.lock().unwrap().len();
    assert_eq!(got, n, "expected {} event(s), got {}", n, got);
}

#[then(expr = "the subscriber should have received {int} events")]
fn received_n_events_plural(world: &mut EbWorld, n: usize) {
    let got = world.captured.lock().unwrap().len();
    assert_eq!(got, n, "expected {} events, got {}", n, got);
}

#[then(expr = "the received event type should be {string}")]
fn received_event_type(world: &mut EbWorld, expected: String) {
    let captured = world.captured.lock().unwrap();
    let last = captured.last().expect("no events captured");
    assert_eq!(
        last.type_name(),
        expected.as_str(),
        "last event type: expected '{}', got '{}'",
        expected,
        last.type_name()
    );
}

#[then(expr = "the emit result should be blocked with reason {string}")]
fn emit_result_blocked(world: &mut EbWorld, reason: String) {
    match world.last_decision.as_ref().expect("no emit decision recorded") {
        HandlerDecision::Block { reason: r } => {
            assert_eq!(r, &reason, "block reason mismatch");
        }
        HandlerDecision::Continue => panic!("expected Block but got Continue"),
    }
}

#[then(expr = "the priority execution order should be {string}")]
fn priority_execution_order(world: &mut EbWorld, expected: String) {
    let got = world.priority_order.lock().unwrap().clone();
    let got_str: Vec<String> = got.iter().map(|p| p.to_string()).collect();
    let got_joined = got_str.join(",");
    assert_eq!(
        got_joined, expected,
        "priority order: expected '{}', got '{}'",
        expected, got_joined
    );
}

// ---------------------------------------------------------------------------
// Filter parser
// ---------------------------------------------------------------------------

fn parse_filter(spec: &str) -> EventFilter {
    if spec == "All" {
        return EventFilter::All;
    }
    if let Some(rest) = spec.strip_prefix("ByType:") {
        let types: Vec<String> = rest.split(',').map(str::trim).map(str::to_owned).collect();
        return EventFilter::ByType(types);
    }
    if let Some(rest) = spec.strip_prefix("ByPrefix:") {
        return EventFilter::ByPrefix(rest.to_owned());
    }
    if let Some(rest) = spec.strip_prefix("Custom:") {
        return EventFilter::Custom(rest.to_owned());
    }
    panic!("Unknown filter spec: '{}'", spec);
}

// ---------------------------------------------------------------------------
// Cucumber entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    EbWorld::run("tests/features/event_bus.feature").await;
}
