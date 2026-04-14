/*!
 * BDD tests for the agent MessageQueue and AgentMessageQueues types.
 * Run with: cargo test --test message_queue_bdd
 */
use cucumber::{given, then, when, World};
use std::sync::Arc;
use std::thread;
use vibecli_cli::message_queue::{AgentMessageQueues, DrainMode, MessageQueue, QueuedMessage};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct MqWorld {
    /// The main queue under test.
    queue: Option<MessageQueue>,
    /// An agent queue pair (steering + follow-up).
    agent_queues: Option<AgentMessageQueues>,
    /// Messages returned by the most recent drain call.
    last_drained: Vec<QueuedMessage>,
    /// Messages drained from the steering queue in the last drain_steering call.
    last_steering_drained: Vec<QueuedMessage>,
    /// Whether the last enqueue attempt succeeded.
    last_enqueue_ok: bool,
    /// The error message from the last failed enqueue.
    last_enqueue_err: String,
}

impl MqWorld {
    fn queue(&self) -> &MessageQueue {
        self.queue.as_ref().expect("queue not initialised")
    }

    fn agent(&self) -> &AgentMessageQueues {
        self.agent_queues.as_ref().expect("agent queues not initialised")
    }
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given("a OneAtATime message queue")]
fn given_one_at_a_time(world: &mut MqWorld) {
    world.queue = Some(MessageQueue::new(DrainMode::OneAtATime));
}

#[given("an All mode message queue")]
fn given_all_mode(world: &mut MqWorld) {
    world.queue = Some(MessageQueue::new(DrainMode::All));
}

#[given(expr = "a OneAtATime message queue with max size {int}")]
fn given_one_at_a_time_capped(world: &mut MqWorld, max: usize) {
    world.queue = Some(MessageQueue::with_max_size(DrainMode::OneAtATime, max));
}

#[given(expr = "an agent message queues pair with OneAtATime drain mode")]
fn given_agent_queues_one_at_a_time(world: &mut MqWorld) {
    world.agent_queues = Some(AgentMessageQueues::new());
}

// Enqueue a comma-separated list of messages in order
#[given(expr = "I enqueue user messages {string}, {string}, {string}")]
fn given_enqueue_three(world: &mut MqWorld, a: String, b: String, c: String) {
    for content in [a, b, c] {
        world.queue().enqueue(QueuedMessage::user(content)).unwrap();
    }
}

#[given(expr = "I enqueue user messages {string}, {string}")]
fn given_enqueue_two(world: &mut MqWorld, a: String, b: String) {
    for content in [a, b] {
        world.queue().enqueue(QueuedMessage::user(content)).unwrap();
    }
}

#[given(expr = "I enqueue user message {string}")]
fn given_enqueue_one(world: &mut MqWorld, content: String) {
    world
        .queue()
        .enqueue(QueuedMessage::user(content))
        .unwrap();
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I drain the queue")]
fn when_drain(world: &mut MqWorld) {
    world.last_drained = world.queue().drain();
}

#[when(expr = "I try to enqueue user message {string}")]
fn when_try_enqueue(world: &mut MqWorld, content: String) {
    match world.queue().enqueue(QueuedMessage::user(content)) {
        Ok(()) => {
            world.last_enqueue_ok = true;
            world.last_enqueue_err.clear();
        }
        Err(e) => {
            world.last_enqueue_ok = false;
            world.last_enqueue_err = e;
        }
    }
}

#[when(expr = "I steer with {string}")]
fn when_steer(world: &mut MqWorld, content: String) {
    world.agent().steer(content).unwrap();
}

#[when(expr = "I follow up with {string}")]
fn when_follow_up(world: &mut MqWorld, content: String) {
    world.agent().follow_up_with(content).unwrap();
}

#[when("I drain the steering queue")]
fn when_drain_steering(world: &mut MqWorld) {
    world.last_steering_drained = world.agent().drain_steering();
}

#[when(expr = "{int} threads each enqueue {int} messages concurrently")]
fn when_concurrent_enqueue(world: &mut MqWorld, thread_count: usize, per_thread: usize) {
    let q = Arc::new(world.queue.take().expect("queue not initialised"));
    let mut handles = Vec::new();
    for t in 0..thread_count {
        let q_clone = Arc::clone(&q);
        handles.push(thread::spawn(move || {
            for i in 0..per_thread {
                let msg = QueuedMessage::user(format!("t{t}-{i}"));
                q_clone.enqueue(msg).ok();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    // Unwrap Arc back into world — safe because all threads finished
    world.queue = Some(Arc::try_unwrap(q).expect("arc still shared after threads joined"));
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then(expr = "I should receive exactly {int} message")]
fn then_receive_count_singular(world: &mut MqWorld, expected: usize) {
    assert_eq!(
        world.last_drained.len(),
        expected,
        "drained message count mismatch"
    );
}

#[then(expr = "I should receive exactly {int} messages")]
fn then_receive_count_plural(world: &mut MqWorld, expected: usize) {
    assert_eq!(
        world.last_drained.len(),
        expected,
        "drained message count mismatch"
    );
}

#[then(expr = "the message content should be {string}")]
fn then_message_content(world: &mut MqWorld, expected: String) {
    assert_eq!(
        world.last_drained[0].content, expected,
        "first drained message content mismatch"
    );
}

#[then(expr = "the queue should have {int} messages remaining")]
fn then_queue_length(world: &mut MqWorld, expected: usize) {
    assert_eq!(world.queue().len(), expected, "remaining queue length mismatch");
}

#[then("the enqueue should fail with a capacity error")]
fn then_enqueue_failed(world: &mut MqWorld) {
    assert!(
        !world.last_enqueue_ok,
        "expected enqueue to fail but it succeeded"
    );
    assert!(
        !world.last_enqueue_err.is_empty(),
        "expected a non-empty error message"
    );
}

#[then(expr = "I should receive exactly {int} steering message")]
fn then_steering_count_singular(world: &mut MqWorld, expected: usize) {
    assert_eq!(
        world.last_steering_drained.len(),
        expected,
        "drained steering message count mismatch"
    );
}

#[then(expr = "I should receive exactly {int} steering messages")]
fn then_steering_count_plural(world: &mut MqWorld, expected: usize) {
    assert_eq!(
        world.last_steering_drained.len(),
        expected,
        "drained steering message count mismatch"
    );
}

#[then(expr = "the follow-up queue should have {int} message remaining")]
fn then_follow_up_length_singular(world: &mut MqWorld, expected: usize) {
    assert_eq!(
        world.agent().follow_up.len(),
        expected,
        "follow-up queue length mismatch"
    );
}

#[then(expr = "the follow-up queue should have {int} messages remaining")]
fn then_follow_up_length_plural(world: &mut MqWorld, expected: usize) {
    assert_eq!(
        world.agent().follow_up.len(),
        expected,
        "follow-up queue length mismatch"
    );
}

#[then(expr = "the steering queue should have {int} message remaining")]
fn then_steering_length_singular(world: &mut MqWorld, expected: usize) {
    assert_eq!(
        world.agent().steering.len(),
        expected,
        "steering queue length mismatch"
    );
}

#[then(expr = "the steering queue should have {int} messages remaining")]
fn then_steering_length_plural(world: &mut MqWorld, expected: usize) {
    assert_eq!(
        world.agent().steering.len(),
        expected,
        "steering queue length mismatch"
    );
}

#[then("the agent queues should be idle")]
fn then_agent_idle(world: &mut MqWorld) {
    assert!(world.agent().is_idle(), "expected agent queues to be idle");
}

#[then("the agent queues should not be idle")]
fn then_agent_not_idle(world: &mut MqWorld) {
    assert!(
        !world.agent().is_idle(),
        "expected agent queues to be non-idle"
    );
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(MqWorld::run("tests/features/message_queue.feature"));
}
