#![allow(dead_code)]
//! Typed in-process lifecycle event bus for extensions.
//! Pi-mono gap bridge: Phase C6.
//!
//! Provides 30+ typed event variants covering session, agent, tool, provider,
//! streaming, memory, file, cost, and extension lifecycles. Extensions subscribe
//! with filters and priority; blocking handlers can veto `ToolCall` and
//! `BeforeProviderRequest` events before execution proceeds.

use std::sync::{Arc, Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

/// All lifecycle event variants emitted on the bus.
#[derive(Debug, Clone, PartialEq)]
pub enum BusEvent {
    // ── Session lifecycle ────────────────────────────────────────────────────
    SessionInit          { session_id: String },
    SessionEnd           { session_id: String },
    SessionBeforeCompact { session_id: String, message_count: usize },
    SessionAfterCompact  { session_id: String, summary: String },

    // ── Agent lifecycle ──────────────────────────────────────────────────────
    AgentStart { turn: u32 },
    AgentEnd   { turn: u32, tool_calls_made: usize },
    AgentError { turn: u32, message: String },

    // ── Tool lifecycle ───────────────────────────────────────────────────────
    ToolCall    { call_id: String, tool_name: String, args_json: String },
    ToolResult  { call_id: String, tool_name: String, output: String, exit_code: i32 },
    ToolBlocked { call_id: String, tool_name: String, reason: String },

    // ── Model / provider ─────────────────────────────────────────────────────
    ModelChange           { old_model: String, new_model: String },
    ProviderChange        { old_provider: String, new_provider: String },
    BeforeProviderRequest { provider: String, message_count: usize },
    AfterProviderResponse { provider: String, input_tokens: u32, output_tokens: u32 },

    // ── Token streaming ──────────────────────────────────────────────────────
    TokenDelta   { text: String, turn: u32 },
    ThinkingDelta { text: String, turn: u32 },
    StreamEnd    { turn: u32 },

    // ── Input ────────────────────────────────────────────────────────────────
    UserInput    { content: String },
    CommandInput { command: String, args: String },

    // ── Memory ───────────────────────────────────────────────────────────────
    MemoryWrite  { key: String, value_preview: String },
    MemoryRead   { key: String },
    MemoryDelete { key: String },

    // ── File operations (from tool calls) ───────────────────────────────────
    FileRead   { path: String },
    FileWrite  { path: String, bytes: usize },
    FileDelete { path: String },

    // ── Cost / quota ─────────────────────────────────────────────────────────
    CostThreshold { provider: String, cost_usd: f64, threshold_usd: f64 },
    QuotaWarning  { provider: String, used_pct: f64 },

    // ── Extension ────────────────────────────────────────────────────────────
    ExtensionLoaded   { name: String },
    ExtensionUnloaded { name: String },
    ExtensionError    { name: String, error: String },

    // ── Custom (extension-defined events) ───────────────────────────────────
    Custom { event_type: String, payload: String },
}

impl BusEvent {
    /// Returns the snake_case type name used for filtering.
    pub fn type_name(&self) -> &str {
        match self {
            Self::SessionInit          { .. } => "session_init",
            Self::SessionEnd           { .. } => "session_end",
            Self::SessionBeforeCompact { .. } => "session_before_compact",
            Self::SessionAfterCompact  { .. } => "session_after_compact",
            Self::AgentStart           { .. } => "agent_start",
            Self::AgentEnd             { .. } => "agent_end",
            Self::AgentError           { .. } => "agent_error",
            Self::ToolCall             { .. } => "tool_call",
            Self::ToolResult           { .. } => "tool_result",
            Self::ToolBlocked          { .. } => "tool_blocked",
            Self::ModelChange          { .. } => "model_change",
            Self::ProviderChange       { .. } => "provider_change",
            Self::BeforeProviderRequest{ .. } => "before_provider_request",
            Self::AfterProviderResponse{ .. } => "after_provider_response",
            Self::TokenDelta           { .. } => "token_delta",
            Self::ThinkingDelta        { .. } => "thinking_delta",
            Self::StreamEnd            { .. } => "stream_end",
            Self::UserInput            { .. } => "user_input",
            Self::CommandInput         { .. } => "command_input",
            Self::MemoryWrite          { .. } => "memory_write",
            Self::MemoryRead           { .. } => "memory_read",
            Self::MemoryDelete         { .. } => "memory_delete",
            Self::FileRead             { .. } => "file_read",
            Self::FileWrite            { .. } => "file_write",
            Self::FileDelete           { .. } => "file_delete",
            Self::CostThreshold        { .. } => "cost_threshold",
            Self::QuotaWarning         { .. } => "quota_warning",
            Self::ExtensionLoaded      { .. } => "extension_loaded",
            Self::ExtensionUnloaded    { .. } => "extension_unloaded",
            Self::ExtensionError       { .. } => "extension_error",
            Self::Custom               { .. } => "custom",
        }
    }

    /// Returns `true` for events where a `Block` decision has real effect.
    /// Only `ToolCall` and `BeforeProviderRequest` are meaningful to block;
    /// all other events are observational.
    pub fn is_blocking_candidate(&self) -> bool {
        matches!(self, Self::ToolCall { .. } | Self::BeforeProviderRequest { .. })
    }

    /// Returns the session_id if this event carries one.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::SessionInit          { session_id }
            | Self::SessionEnd         { session_id }
            | Self::SessionBeforeCompact { session_id, .. }
            | Self::SessionAfterCompact  { session_id, .. } => Some(session_id.as_str()),
            _ => None,
        }
    }

    /// Returns the tool_name if this event is tool-related.
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            Self::ToolCall    { tool_name, .. }
            | Self::ToolResult  { tool_name, .. }
            | Self::ToolBlocked { tool_name, .. } => Some(tool_name.as_str()),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Subscription
// ---------------------------------------------------------------------------

/// Opaque identifier for a subscription, returned by [`EventBus::subscribe`].
pub type SubscriberId = u64;

/// Filter that controls which events are delivered to a subscriber.
#[derive(Debug, Clone)]
pub enum EventFilter {
    /// Receive every event.
    All,
    /// Receive only events whose [`BusEvent::type_name`] is in the list.
    ByType(Vec<String>),
    /// Receive events whose [`BusEvent::type_name`] starts with the given prefix.
    /// E.g., `"tool_"` matches `tool_call`, `tool_result`, `tool_blocked`.
    ByPrefix(String),
    /// Opaque tag for custom dispatch logic (extension-managed).
    Custom(String),
}

impl EventFilter {
    /// Returns `true` if this filter matches `event`.
    pub fn matches(&self, event: &BusEvent) -> bool {
        match self {
            Self::All => true,
            Self::ByType(names) => names.iter().any(|n| n == event.type_name()),
            Self::ByPrefix(prefix) => event.type_name().starts_with(prefix.as_str()),
            // Custom filters never auto-match; the subscriber is responsible for
            // checking via the handler closure.
            Self::Custom(_) => true,
        }
    }
}

/// Decision returned by a subscription handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerDecision {
    /// Allow the operation to proceed (default for observational handlers).
    Continue,
    /// Block the operation (only meaningful for blocking-candidate events).
    Block { reason: String },
}

/// Internal subscription record.
pub struct Subscription {
    pub id: SubscriberId,
    pub filter: EventFilter,
    pub handler: Box<dyn Fn(&BusEvent) -> HandlerDecision + Send + Sync>,
    /// Higher priority runs first. Ties are broken by insertion order.
    pub priority: i32,
}

// We implement Debug manually because the handler closure is not Debug.
impl std::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription")
            .field("id", &self.id)
            .field("filter", &self.filter)
            .field("priority", &self.priority)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// EventBus
// ---------------------------------------------------------------------------

/// Typed in-process lifecycle event bus.
///
/// # Thread safety
/// All fields are wrapped in `Arc<Mutex<_>>` so the bus can be cloned and
/// shared across threads. Handlers are required to be `Send + Sync`.
///
/// # Blocking
/// When [`EventBus::emit`] is called with a blocking-candidate event (e.g.
/// `ToolCall`), if *any* handler returns `HandlerDecision::Block`, that
/// decision is returned immediately and remaining handlers are **not** called.
/// For non-blocking-candidate events all handlers are always called.
pub struct EventBus {
    subscriptions: Arc<Mutex<Vec<Subscription>>>,
    next_id: Arc<Mutex<SubscriberId>>,
    history: Arc<Mutex<Vec<BusEvent>>>,
    max_history: usize,
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("max_history", &self.max_history)
            .finish_non_exhaustive()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new bus with no history retention (max_history = 0).
    pub fn new() -> Self {
        Self::with_history(256)
    }

    /// Create a new bus keeping up to `max_history` recent events.
    /// Pass `0` to disable history.
    pub fn with_history(max_history: usize) -> Self {
        Self {
            subscriptions: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
            history: Arc::new(Mutex::new(Vec::new())),
            max_history,
        }
    }

    /// Subscribe with a filter and handler. Returns the subscription ID.
    ///
    /// `priority` controls ordering — higher values run first. Use `0` for
    /// ordinary observers and negative values for low-priority logging.
    pub fn subscribe<F>(&self, filter: EventFilter, priority: i32, handler: F) -> SubscriberId
    where
        F: Fn(&BusEvent) -> HandlerDecision + Send + Sync + 'static,
    {
        let id = {
            let mut n = self.next_id.lock().unwrap();
            let id = *n;
            *n += 1;
            id
        };
        let sub = Subscription {
            id,
            filter,
            handler: Box::new(handler),
            priority,
        };
        let mut subs = self.subscriptions.lock().unwrap();
        subs.push(sub);
        // Keep sorted: highest priority first, then insertion order (stable sort).
        subs.sort_by(|a, b| b.priority.cmp(&a.priority));
        id
    }

    /// Unsubscribe by ID. Returns `true` if a subscription was removed.
    pub fn unsubscribe(&self, id: SubscriberId) -> bool {
        let mut subs = self.subscriptions.lock().unwrap();
        let before = subs.len();
        subs.retain(|s| s.id != id);
        subs.len() < before
    }

    /// Emit an event.
    ///
    /// Handlers that match the event filter are called in priority order.
    /// For blocking-candidate events the first `Block` decision short-circuits
    /// remaining handlers and is returned. For non-blocking events all matching
    /// handlers are called and `Continue` is always returned.
    pub fn emit(&self, event: BusEvent) -> HandlerDecision {
        // Record in history first (even if blocked, for auditability).
        if self.max_history > 0 {
            let mut hist = self.history.lock().unwrap();
            if hist.len() >= self.max_history {
                hist.remove(0);
            }
            hist.push(event.clone());
        }

        let is_blocking = event.is_blocking_candidate();
        let subs = self.subscriptions.lock().unwrap();

        for sub in subs.iter() {
            if !sub.filter.matches(&event) {
                continue;
            }
            let decision = (sub.handler)(&event);
            if is_blocking {
                if let HandlerDecision::Block { .. } = &decision {
                    return decision;
                }
            }
        }

        HandlerDecision::Continue
    }

    /// Return a snapshot of the event history (oldest first).
    pub fn history(&self) -> Vec<BusEvent> {
        self.history.lock().unwrap().clone()
    }

    /// Number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.lock().unwrap().len()
    }

    /// Number of events currently stored in history.
    pub fn history_count(&self) -> usize {
        self.history.lock().unwrap().len()
    }

    /// Clear the event history.
    pub fn clear_history(&self) {
        self.history.lock().unwrap().clear();
    }
}

// ---------------------------------------------------------------------------
// Global bus
// ---------------------------------------------------------------------------

/// Returns the process-wide shared [`EventBus`].
///
/// The global bus is initialized with a 512-event history window on first
/// call and reused for all subsequent calls. Use this for production code;
/// construct a local [`EventBus`] instance in unit tests to avoid cross-test
/// interference.
pub fn global_bus() -> Arc<EventBus> {
    static GLOBAL: OnceLock<Arc<EventBus>> = OnceLock::new();
    Arc::clone(GLOBAL.get_or_init(|| Arc::new(EventBus::with_history(512))))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_bus() -> EventBus {
        EventBus::with_history(16)
    }

    // ── subscribe + emit received ────────────────────────────────────────────

    #[test]
    fn subscribe_and_emit_received() {
        let bus = make_bus();
        let received: Arc<Mutex<Vec<BusEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let recv = Arc::clone(&received);

        bus.subscribe(EventFilter::All, 0, move |e| {
            recv.lock().unwrap().push(e.clone());
            HandlerDecision::Continue
        });

        bus.emit(BusEvent::AgentStart { turn: 1 });
        bus.emit(BusEvent::AgentEnd { turn: 1, tool_calls_made: 3 });

        let got = received.lock().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], BusEvent::AgentStart { turn: 1 });
    }

    // ── unsubscribe stops delivery ───────────────────────────────────────────

    #[test]
    fn unsubscribe_stops_delivery() {
        let bus = make_bus();
        let count: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
        let c = Arc::clone(&count);

        let id = bus.subscribe(EventFilter::All, 0, move |_| {
            *c.lock().unwrap() += 1;
            HandlerDecision::Continue
        });

        bus.emit(BusEvent::UserInput { content: "hello".into() });
        assert_eq!(*count.lock().unwrap(), 1);

        let removed = bus.unsubscribe(id);
        assert!(removed);

        bus.emit(BusEvent::UserInput { content: "world".into() });
        assert_eq!(*count.lock().unwrap(), 1, "should not increment after unsubscribe");
    }

    // ── unsubscribe unknown id returns false ─────────────────────────────────

    #[test]
    fn unsubscribe_unknown_id_returns_false() {
        let bus = make_bus();
        assert!(!bus.unsubscribe(9999));
    }

    // ── Block decision propagates ────────────────────────────────────────────

    #[test]
    fn block_decision_propagates() {
        let bus = make_bus();
        bus.subscribe(EventFilter::All, 0, |_| HandlerDecision::Block {
            reason: "denied".into(),
        });

        let decision = bus.emit(BusEvent::ToolCall {
            call_id: "c1".into(),
            tool_name: "Bash".into(),
            args_json: "{}".into(),
        });

        assert_eq!(
            decision,
            HandlerDecision::Block { reason: "denied".into() }
        );
    }

    // ── Block on non-blocking candidate is ignored ───────────────────────────

    #[test]
    fn block_on_non_blocking_candidate_returns_continue() {
        let bus = make_bus();
        // AgentStart is NOT a blocking candidate; even if a handler returns Block,
        // the bus must return Continue.
        bus.subscribe(EventFilter::All, 0, |_| HandlerDecision::Block {
            reason: "this should be ignored".into(),
        });

        let decision = bus.emit(BusEvent::AgentStart { turn: 0 });
        assert_eq!(decision, HandlerDecision::Continue);
    }

    // ── EventFilter::ByType ──────────────────────────────────────────────────

    #[test]
    fn filter_by_type_matches_only_matching_events() {
        let bus = make_bus();
        let count: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
        let c = Arc::clone(&count);

        bus.subscribe(
            EventFilter::ByType(vec!["tool_call".into()]),
            0,
            move |_| {
                *c.lock().unwrap() += 1;
                HandlerDecision::Continue
            },
        );

        bus.emit(BusEvent::AgentStart { turn: 1 });
        bus.emit(BusEvent::ToolCall {
            call_id: "c1".into(),
            tool_name: "Edit".into(),
            args_json: "{}".into(),
        });
        bus.emit(BusEvent::AgentEnd { turn: 1, tool_calls_made: 1 });

        assert_eq!(*count.lock().unwrap(), 1);
    }

    // ── EventFilter::ByPrefix ────────────────────────────────────────────────

    #[test]
    fn filter_by_prefix_matches_prefix_events() {
        let bus = make_bus();
        let names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let n = Arc::clone(&names);

        bus.subscribe(EventFilter::ByPrefix("tool_".into()), 0, move |e| {
            n.lock().unwrap().push(e.type_name().to_owned());
            HandlerDecision::Continue
        });

        bus.emit(BusEvent::SessionInit { session_id: "s1".into() });
        bus.emit(BusEvent::ToolCall {
            call_id: "c1".into(),
            tool_name: "Read".into(),
            args_json: "{}".into(),
        });
        bus.emit(BusEvent::ToolResult {
            call_id: "c1".into(),
            tool_name: "Read".into(),
            output: "ok".into(),
            exit_code: 0,
        });
        bus.emit(BusEvent::ToolBlocked {
            call_id: "c2".into(),
            tool_name: "Bash".into(),
            reason: "no".into(),
        });
        bus.emit(BusEvent::AgentEnd { turn: 1, tool_calls_made: 2 });

        let got = names.lock().unwrap();
        assert_eq!(got.as_slice(), &["tool_call", "tool_result", "tool_blocked"]);
    }

    // ── Priority ordering ────────────────────────────────────────────────────

    #[test]
    fn priority_ordering_higher_runs_first() {
        let bus = make_bus();
        let order: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));

        for &p in &[0i32, 10, -5, 5] {
            let o = Arc::clone(&order);
            bus.subscribe(EventFilter::All, p, move |_| {
                o.lock().unwrap().push(p);
                HandlerDecision::Continue
            });
        }

        bus.emit(BusEvent::UserInput { content: "x".into() });

        let got = order.lock().unwrap().clone();
        assert_eq!(got, vec![10, 5, 0, -5]);
    }

    // ── History records events ───────────────────────────────────────────────

    #[test]
    fn history_records_emitted_events() {
        let bus = make_bus();
        bus.emit(BusEvent::SessionInit { session_id: "s1".into() });
        bus.emit(BusEvent::AgentStart { turn: 1 });

        let hist = bus.history();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].type_name(), "session_init");
        assert_eq!(hist[1].type_name(), "agent_start");
    }

    // ── History max_history eviction ─────────────────────────────────────────

    #[test]
    fn history_max_history_eviction() {
        let bus = EventBus::with_history(3);

        for i in 0..5u32 {
            bus.emit(BusEvent::AgentStart { turn: i });
        }

        let hist = bus.history();
        assert_eq!(hist.len(), 3, "should only keep last 3 events");
        // Oldest surviving should be turn=2
        assert_eq!(hist[0], BusEvent::AgentStart { turn: 2 });
        assert_eq!(hist[2], BusEvent::AgentStart { turn: 4 });
    }

    // ── history_count and clear_history ─────────────────────────────────────

    #[test]
    fn history_count_and_clear() {
        let bus = make_bus();
        bus.emit(BusEvent::UserInput { content: "hi".into() });
        bus.emit(BusEvent::UserInput { content: "bye".into() });
        assert_eq!(bus.history_count(), 2);
        bus.clear_history();
        assert_eq!(bus.history_count(), 0);
    }

    // ── Custom event ─────────────────────────────────────────────────────────

    #[test]
    fn custom_event_delivered() {
        let bus = make_bus();
        let payload: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let p = Arc::clone(&payload);

        bus.subscribe(EventFilter::ByType(vec!["custom".into()]), 0, move |e| {
            if let BusEvent::Custom { payload, .. } = e {
                *p.lock().unwrap() = Some(payload.clone());
            }
            HandlerDecision::Continue
        });

        bus.emit(BusEvent::Custom {
            event_type: "my_extension_event".into(),
            payload: r#"{"foo":42}"#.into(),
        });

        assert_eq!(
            payload.lock().unwrap().as_deref(),
            Some(r#"{"foo":42}"#)
        );
    }

    // ── type_name spot checks ────────────────────────────────────────────────

    #[test]
    fn type_name_spot_checks() {
        let cases: &[(BusEvent, &str)] = &[
            (BusEvent::SessionInit { session_id: "s".into() }, "session_init"),
            (BusEvent::SessionBeforeCompact { session_id: "s".into(), message_count: 10 }, "session_before_compact"),
            (BusEvent::ToolCall { call_id: "c".into(), tool_name: "t".into(), args_json: "{}".into() }, "tool_call"),
            (BusEvent::BeforeProviderRequest { provider: "claude".into(), message_count: 5 }, "before_provider_request"),
            (BusEvent::TokenDelta { text: "hi".into(), turn: 1 }, "token_delta"),
            (BusEvent::MemoryWrite { key: "k".into(), value_preview: "v".into() }, "memory_write"),
            (BusEvent::CostThreshold { provider: "p".into(), cost_usd: 1.0, threshold_usd: 5.0 }, "cost_threshold"),
            (BusEvent::ExtensionLoaded { name: "ext".into() }, "extension_loaded"),
        ];
        for (event, expected) in cases {
            assert_eq!(event.type_name(), *expected, "mismatch for {:?}", event);
        }
    }

    // ── is_blocking_candidate ────────────────────────────────────────────────

    #[test]
    fn is_blocking_candidate_correct() {
        assert!(BusEvent::ToolCall {
            call_id: "c".into(), tool_name: "t".into(), args_json: "{}".into()
        }.is_blocking_candidate());

        assert!(BusEvent::BeforeProviderRequest {
            provider: "p".into(), message_count: 1
        }.is_blocking_candidate());

        assert!(!BusEvent::AgentStart { turn: 0 }.is_blocking_candidate());
        assert!(!BusEvent::UserInput { content: "x".into() }.is_blocking_candidate());
        assert!(!BusEvent::SessionEnd { session_id: "s".into() }.is_blocking_candidate());
    }

    // ── session_id and tool_name accessors ───────────────────────────────────

    #[test]
    fn session_id_accessor() {
        let e = BusEvent::SessionInit { session_id: "abc".into() };
        assert_eq!(e.session_id(), Some("abc"));

        let e2 = BusEvent::AgentStart { turn: 1 };
        assert_eq!(e2.session_id(), None);
    }

    #[test]
    fn tool_name_accessor() {
        let e = BusEvent::ToolCall {
            call_id: "c".into(),
            tool_name: "Bash".into(),
            args_json: "{}".into(),
        };
        assert_eq!(e.tool_name(), Some("Bash"));

        let e2 = BusEvent::AgentStart { turn: 0 };
        assert_eq!(e2.tool_name(), None);
    }

    // ── subscription_count ───────────────────────────────────────────────────

    #[test]
    fn subscription_count_tracks_adds_and_removes() {
        let bus = make_bus();
        assert_eq!(bus.subscription_count(), 0);

        let id1 = bus.subscribe(EventFilter::All, 0, |_| HandlerDecision::Continue);
        let id2 = bus.subscribe(EventFilter::All, 0, |_| HandlerDecision::Continue);
        assert_eq!(bus.subscription_count(), 2);

        bus.unsubscribe(id1);
        assert_eq!(bus.subscription_count(), 1);

        bus.unsubscribe(id2);
        assert_eq!(bus.subscription_count(), 0);
    }

    // ── global_bus is a singleton ────────────────────────────────────────────

    #[test]
    fn global_bus_is_same_instance() {
        let a = global_bus();
        let b = global_bus();
        // Both Arcs must point to the same allocation.
        assert!(Arc::ptr_eq(&a, &b));
    }

    // ── Block short-circuits remaining handlers ───────────────────────────────

    #[test]
    fn block_short_circuits_lower_priority_handlers() {
        let bus = make_bus();
        let second_called: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
        let sc = Arc::clone(&second_called);

        // High priority blocker
        bus.subscribe(EventFilter::All, 10, |_| HandlerDecision::Block {
            reason: "stop".into(),
        });

        // Lower priority — should NOT be called
        bus.subscribe(EventFilter::All, 0, move |_| {
            *sc.lock().unwrap() = true;
            HandlerDecision::Continue
        });

        let decision = bus.emit(BusEvent::ToolCall {
            call_id: "c".into(),
            tool_name: "Bash".into(),
            args_json: "{}".into(),
        });

        assert_eq!(decision, HandlerDecision::Block { reason: "stop".into() });
        assert!(!*second_called.lock().unwrap(), "lower-priority handler must not run after block");
    }
}
