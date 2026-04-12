# Mock AI Provider

A deterministic, zero-network `AIProvider` implementation for CI testing. Provides claw-code parity for reproducible, scenario-driven AI response sequences without hitting live APIs.

## When to Use
- Writing unit or BDD tests that call an `AIProvider` without real credentials
- Simulating exhausted response queues (error injection)
- Testing provider fallback / failover chains
- Verifying call-count semantics and concurrency under load
- Prefix-based scenario dispatch (different prompts → different canned replies)

## Configuration
```rust
// Sequenced responses (returned in order, error when exhausted)
let provider = MockAIProvider::new("test-mock")
    .with_responses(vec![
        Ok("First reply".into()),
        Ok("Second reply".into()),
        Err(anyhow::anyhow!("quota exceeded")),
    ]);

// Scenario prefix matching (checked before the queue)
let provider = MockAIProvider::new("scenario-mock")
    .with_scenario("fix", "I fixed the bug")
    .with_scenario("explain", "Here is the explanation");

// Unavailable provider (simulates a downed endpoint)
let provider = MockAIProvider::new("offline").unavailable();
```

## Key Methods
| Method | Description |
|---|---|
| `call_count()` | Number of `complete` / `chat` calls made |
| `is_available()` | Returns false when `.unavailable()` is set |
| `with_responses(vec)` | Load sequenced reply queue |
| `with_scenario(prefix, reply)` | Add prefix-matched scenario |
| `with_delay(duration)` | Simulate latency (tokio::sleep) |

## Enabling in Tests
The module is gated behind `#[cfg(any(test, feature = "testing"))]`. Add the feature flag to use it from integration tests:

```toml
# Cargo.toml dev-dependencies or test harness
vibe-ai = { path = "...", features = ["testing"] }
```

Or run: `cargo test --features vibe-ai/testing`

## Examples
```rust
use vibe_ai::mock_provider::MockAIProvider;
use vibe_ai::provider::AIProvider;

#[tokio::test]
async fn test_with_mock() {
    let p = MockAIProvider::new("mock")
        .with_responses(vec![Ok("hello".into())]);
    let reply = p.complete("say hello", &[]).await.unwrap();
    assert_eq!(reply, "hello");
    assert_eq!(p.call_count(), 1);
}
```

## BDD Harness
Run: `cargo test --test mock_provider_bdd -p vibe-ai --features vibe-ai/testing`

Scenarios: sequenced order, unavailability, call counting, prefix matching.
