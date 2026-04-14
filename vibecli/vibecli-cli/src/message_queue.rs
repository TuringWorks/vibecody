#![allow(dead_code)]
//! Message queues for agent steering and follow-up injection.
//! Pi-mono gap bridge: Phase A4.
//!
//! Two independent queues per agent turn:
//! - **Steering queue** — injected between tool calls while the turn is active.
//! - **Follow-up queue** — injected after all tool calls in a turn complete.
//!
//! Both queues are thread-safe and support two drain modes: `OneAtATime`
//! (one message per opportunity) or `All` (flush everything at once).

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// DrainMode
// ---------------------------------------------------------------------------

/// Controls how many messages are dequeued per opportunity.
///
/// - `OneAtATime` — dequeue a single message per tool-call gap or turn-end.
///   Ideal for fine-grained steering that needs one message at a time.
/// - `All` — flush the entire queue at once.
///   Useful when all pending follow-ups should be delivered together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrainMode {
    /// Dequeue one message per tool-call gap / turn-end.
    OneAtATime,
    /// Dequeue all queued messages at once.
    All,
}

// ---------------------------------------------------------------------------
// QueuedMessage
// ---------------------------------------------------------------------------

/// A message waiting to be injected into an agent turn.
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    /// Unique message identifier (UUID or caller-supplied string).
    pub id: String,
    /// The message text to inject.
    pub content: String,
    /// Role for the injected message: `"user"` or `"system"`.
    pub role: String,
    /// Where the message will be injected: `"between_tools"` or `"after_turn"`.
    pub injected_at: Option<String>,
    /// Arbitrary key/value metadata (e.g. source, priority hint).
    pub metadata: HashMap<String, String>,
}

impl QueuedMessage {
    /// Create a user-role message with a generated id.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            content: content.into(),
            role: "user".to_string(),
            injected_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a system-role message with a generated id.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            content: content.into(),
            role: "system".to_string(),
            injected_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Attach a metadata key/value pair (builder-style).
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Generate a simple monotonic id (avoids pulling in uuid for this module).
fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("msg-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

// ---------------------------------------------------------------------------
// MessageQueue
// ---------------------------------------------------------------------------

/// A thread-safe FIFO queue for [`QueuedMessage`]s with a configurable drain
/// mode and optional capacity cap.
#[derive(Debug, Clone)]
pub struct MessageQueue {
    inner: Arc<Mutex<VecDeque<QueuedMessage>>>,
    mode: DrainMode,
    max_size: usize,
}

impl MessageQueue {
    /// Default maximum queue depth when none is specified.
    const DEFAULT_MAX: usize = 1_000;

    /// Create a new queue with the given drain mode and the default capacity.
    pub fn new(mode: DrainMode) -> Self {
        Self::with_max_size(mode, Self::DEFAULT_MAX)
    }

    /// Create a new queue with an explicit maximum depth.
    ///
    /// `enqueue` returns `Err` once the queue reaches `max` messages.
    pub fn with_max_size(mode: DrainMode, max: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
            mode,
            max_size: max,
        }
    }

    /// Push a message onto the back of the queue.
    ///
    /// Returns `Err` if the queue is at capacity.
    pub fn enqueue(&self, msg: QueuedMessage) -> Result<(), String> {
        let mut guard = self.inner.lock().expect("message_queue lock poisoned");
        if guard.len() >= self.max_size {
            return Err(format!(
                "queue is full ({} / {} messages)",
                guard.len(),
                self.max_size
            ));
        }
        guard.push_back(msg);
        Ok(())
    }

    /// Dequeue messages according to the configured [`DrainMode`].
    ///
    /// - `OneAtATime` — returns a `Vec` with at most one message.
    /// - `All` — drains the entire queue and returns all messages.
    pub fn drain(&self) -> Vec<QueuedMessage> {
        let mut guard = self.inner.lock().expect("message_queue lock poisoned");
        match self.mode {
            DrainMode::OneAtATime => {
                if let Some(msg) = guard.pop_front() {
                    vec![msg]
                } else {
                    vec![]
                }
            }
            DrainMode::All => guard.drain(..).collect(),
        }
    }

    /// Peek at the front message without removing it.
    pub fn peek(&self) -> Option<QueuedMessage> {
        let guard = self.inner.lock().expect("message_queue lock poisoned");
        guard.front().cloned()
    }

    /// Return the number of messages currently in the queue.
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("message_queue lock poisoned")
            .len()
    }

    /// Return `true` when the queue contains no messages.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Discard all queued messages.
    pub fn clear(&self) {
        self.inner
            .lock()
            .expect("message_queue lock poisoned")
            .clear();
    }

    /// Return the drain mode for this queue.
    pub fn mode(&self) -> DrainMode {
        self.mode.clone()
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new(DrainMode::OneAtATime)
    }
}

// ---------------------------------------------------------------------------
// AgentMessageQueues
// ---------------------------------------------------------------------------

/// A paired set of queues — one for mid-turn steering, one for post-turn
/// follow-up — that mirror pi-mono's `agent.steer()` / `agent.followUp()` API.
#[derive(Debug, Clone)]
pub struct AgentMessageQueues {
    /// Messages injected between tool calls while the agent turn is active.
    pub steering: MessageQueue,
    /// Messages injected after all tool calls in a turn have completed.
    pub follow_up: MessageQueue,
}

impl AgentMessageQueues {
    /// Create queues with the default `OneAtATime` drain mode for both sides.
    pub fn new() -> Self {
        Self::with_modes(DrainMode::OneAtATime, DrainMode::OneAtATime)
    }

    /// Create queues with explicit drain modes for each side.
    pub fn with_modes(steering: DrainMode, follow_up: DrainMode) -> Self {
        Self {
            steering: MessageQueue::new(steering),
            follow_up: MessageQueue::new(follow_up),
        }
    }

    /// Push a user-role guidance message onto the **steering** queue.
    ///
    /// Mirrors `agent.steer(msg)` from the pi-mono API.
    pub fn steer(&self, content: impl Into<String>) -> Result<(), String> {
        let msg = QueuedMessage::user(content).with_metadata("injected_at", "between_tools");
        self.steering.enqueue(msg)
    }

    /// Push a user-role message onto the **follow-up** queue.
    ///
    /// Mirrors `agent.followUp(msg)` from the pi-mono API.
    pub fn follow_up_with(&self, content: impl Into<String>) -> Result<(), String> {
        let msg = QueuedMessage::user(content).with_metadata("injected_at", "after_turn");
        self.follow_up.enqueue(msg)
    }

    /// Drain the steering queue according to its configured [`DrainMode`].
    pub fn drain_steering(&self) -> Vec<QueuedMessage> {
        self.steering.drain()
    }

    /// Drain the follow-up queue according to its configured [`DrainMode`].
    pub fn drain_follow_up(&self) -> Vec<QueuedMessage> {
        self.follow_up.drain()
    }

    /// Return `true` when both queues are empty (agent is idle).
    pub fn is_idle(&self) -> bool {
        self.steering.is_empty() && self.follow_up.is_empty()
    }
}

impl Default for AgentMessageQueues {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    // ── DrainMode::OneAtATime ────────────────────────────────────────────────

    #[test]
    fn one_at_a_time_drains_single_message() {
        let q = MessageQueue::new(DrainMode::OneAtATime);
        q.enqueue(QueuedMessage::user("first")).unwrap();
        q.enqueue(QueuedMessage::user("second")).unwrap();
        q.enqueue(QueuedMessage::user("third")).unwrap();

        let batch = q.drain();
        assert_eq!(batch.len(), 1, "OneAtATime must return exactly one message");
        assert_eq!(batch[0].content, "first");
        assert_eq!(q.len(), 2, "two messages must remain");
    }

    #[test]
    fn one_at_a_time_drain_order_is_fifo() {
        let q = MessageQueue::new(DrainMode::OneAtATime);
        for i in 0..5 {
            q.enqueue(QueuedMessage::user(format!("msg-{i}"))).unwrap();
        }
        for i in 0..5 {
            let batch = q.drain();
            assert_eq!(batch.len(), 1);
            assert_eq!(batch[0].content, format!("msg-{i}"));
        }
        assert!(q.is_empty());
    }

    #[test]
    fn one_at_a_time_drain_on_empty_returns_empty_vec() {
        let q = MessageQueue::new(DrainMode::OneAtATime);
        assert_eq!(q.drain().len(), 0);
    }

    // ── DrainMode::All ───────────────────────────────────────────────────────

    #[test]
    fn all_mode_drains_entire_queue() {
        let q = MessageQueue::new(DrainMode::All);
        q.enqueue(QueuedMessage::user("a")).unwrap();
        q.enqueue(QueuedMessage::user("b")).unwrap();
        q.enqueue(QueuedMessage::user("c")).unwrap();

        let batch = q.drain();
        assert_eq!(batch.len(), 3, "All mode must return all three messages");
        assert!(q.is_empty(), "queue must be empty after All drain");
    }

    #[test]
    fn all_mode_drain_preserves_order() {
        let q = MessageQueue::new(DrainMode::All);
        for i in 0..4 {
            q.enqueue(QueuedMessage::user(format!("item-{i}"))).unwrap();
        }
        let batch = q.drain();
        for (i, msg) in batch.iter().enumerate() {
            assert_eq!(msg.content, format!("item-{i}"));
        }
    }

    // ── max_size enforcement ─────────────────────────────────────────────────

    #[test]
    fn enqueue_rejects_when_at_max_size() {
        let q = MessageQueue::with_max_size(DrainMode::OneAtATime, 2);
        q.enqueue(QueuedMessage::user("one")).unwrap();
        q.enqueue(QueuedMessage::user("two")).unwrap();

        let result = q.enqueue(QueuedMessage::user("three"));
        assert!(result.is_err(), "third enqueue must fail at capacity 2");
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn enqueue_succeeds_after_drain_frees_space() {
        let q = MessageQueue::with_max_size(DrainMode::OneAtATime, 1);
        q.enqueue(QueuedMessage::user("first")).unwrap();
        assert!(q.enqueue(QueuedMessage::user("overflow")).is_err());

        // Drain the single slot free
        q.drain();
        assert!(q.enqueue(QueuedMessage::user("fits-now")).is_ok());
    }

    // ── peek ────────────────────────────────────────────────────────────────

    #[test]
    fn peek_returns_front_without_removing() {
        let q = MessageQueue::new(DrainMode::OneAtATime);
        q.enqueue(QueuedMessage::user("peek-me")).unwrap();
        q.enqueue(QueuedMessage::user("second")).unwrap();

        let peeked = q.peek().expect("peek on non-empty queue");
        assert_eq!(peeked.content, "peek-me");
        assert_eq!(q.len(), 2, "peek must not remove the message");
    }

    #[test]
    fn peek_on_empty_queue_returns_none() {
        let q = MessageQueue::new(DrainMode::All);
        assert!(q.peek().is_none());
    }

    // ── clear ────────────────────────────────────────────────────────────────

    #[test]
    fn clear_removes_all_messages() {
        let q = MessageQueue::new(DrainMode::All);
        for _ in 0..5 {
            q.enqueue(QueuedMessage::user("x")).unwrap();
        }
        q.clear();
        assert!(q.is_empty());
    }

    // ── is_idle ──────────────────────────────────────────────────────────────

    #[test]
    fn agent_queues_is_idle_when_both_empty() {
        let aq = AgentMessageQueues::new();
        assert!(aq.is_idle());
    }

    #[test]
    fn agent_queues_not_idle_when_steering_has_message() {
        let aq = AgentMessageQueues::new();
        aq.steer("adjust tone").unwrap();
        assert!(!aq.is_idle());
    }

    #[test]
    fn agent_queues_not_idle_when_follow_up_has_message() {
        let aq = AgentMessageQueues::new();
        aq.follow_up_with("summarise results").unwrap();
        assert!(!aq.is_idle());
    }

    #[test]
    fn agent_queues_idle_after_both_drained() {
        let aq = AgentMessageQueues::new();
        aq.steer("steer").unwrap();
        aq.follow_up_with("follow-up").unwrap();
        aq.drain_steering();
        aq.drain_follow_up();
        assert!(aq.is_idle());
    }

    // ── steer + follow_up API ────────────────────────────────────────────────

    #[test]
    fn steer_enqueues_into_steering_queue() {
        let aq = AgentMessageQueues::new();
        aq.steer("be more concise").unwrap();
        assert_eq!(aq.steering.len(), 1);
        assert_eq!(aq.follow_up.len(), 0);
    }

    #[test]
    fn follow_up_with_enqueues_into_follow_up_queue() {
        let aq = AgentMessageQueues::new();
        aq.follow_up_with("now summarise").unwrap();
        assert_eq!(aq.follow_up.len(), 1);
        assert_eq!(aq.steering.len(), 0);
    }

    #[test]
    fn steer_message_has_injected_at_metadata() {
        let aq = AgentMessageQueues::new();
        aq.steer("redirect").unwrap();
        let msgs = aq.drain_steering();
        assert_eq!(
            msgs[0].metadata.get("injected_at").map(String::as_str),
            Some("between_tools")
        );
    }

    #[test]
    fn follow_up_message_has_injected_at_metadata() {
        let aq = AgentMessageQueues::new();
        aq.follow_up_with("wrap up").unwrap();
        let msgs = aq.drain_follow_up();
        assert_eq!(
            msgs[0].metadata.get("injected_at").map(String::as_str),
            Some("after_turn")
        );
    }

    #[test]
    fn steering_and_follow_up_are_independent() {
        let aq = AgentMessageQueues::new();
        aq.steer("steer-1").unwrap();
        aq.steer("steer-2").unwrap();
        aq.follow_up_with("followup-1").unwrap();

        // Drain steering only drains steering queue
        let steered = aq.drain_steering();
        assert_eq!(steered.len(), 1); // OneAtATime
        assert_eq!(aq.steering.len(), 1);
        assert_eq!(aq.follow_up.len(), 1, "follow-up queue must be untouched");
    }

    // ── QueuedMessage builders ───────────────────────────────────────────────

    #[test]
    fn user_message_has_user_role() {
        let m = QueuedMessage::user("hello");
        assert_eq!(m.role, "user");
        assert_eq!(m.content, "hello");
    }

    #[test]
    fn system_message_has_system_role() {
        let m = QueuedMessage::system("sys prompt");
        assert_eq!(m.role, "system");
    }

    #[test]
    fn with_metadata_builder_attaches_kv() {
        let m = QueuedMessage::user("test")
            .with_metadata("priority", "high")
            .with_metadata("source", "ui");
        assert_eq!(m.metadata["priority"], "high");
        assert_eq!(m.metadata["source"], "ui");
    }

    // ── thread safety ────────────────────────────────────────────────────────

    #[test]
    fn concurrent_enqueue_from_multiple_threads() {
        let q = Arc::new(MessageQueue::with_max_size(DrainMode::All, 1_000));
        let mut handles = Vec::new();

        for t in 0..8 {
            let q_clone = Arc::clone(&q);
            handles.push(thread::spawn(move || {
                for i in 0..10 {
                    let msg = QueuedMessage::user(format!("t{t}-msg{i}"));
                    q_clone.enqueue(msg).ok(); // ignore max-size errors if any
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        // All 80 messages should be present (capacity is 1000)
        assert_eq!(q.len(), 80, "all 80 messages must be enqueued");
    }

    #[test]
    fn concurrent_drain_returns_non_overlapping_messages() {
        // Load 100 messages then drain from two threads simultaneously.
        let q = Arc::new(MessageQueue::new(DrainMode::OneAtATime));
        for i in 0..100 {
            q.enqueue(QueuedMessage::user(format!("m{i}"))).unwrap();
        }

        let q1 = Arc::clone(&q);
        let q2 = Arc::clone(&q);
        let h1 = thread::spawn(move || {
            let mut collected = Vec::new();
            loop {
                let batch = q1.drain();
                if batch.is_empty() {
                    break;
                }
                collected.extend(batch.into_iter().map(|m| m.content));
            }
            collected
        });
        let h2 = thread::spawn(move || {
            let mut collected = Vec::new();
            loop {
                let batch = q2.drain();
                if batch.is_empty() {
                    break;
                }
                collected.extend(batch.into_iter().map(|m| m.content));
            }
            collected
        });

        let mut all: Vec<_> = h1.join().unwrap();
        all.extend(h2.join().unwrap());
        assert_eq!(
            all.len(),
            100,
            "each message must be delivered exactly once"
        );
        // No duplicates
        let mut sorted = all.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 100, "no duplicate deliveries");
    }
}
