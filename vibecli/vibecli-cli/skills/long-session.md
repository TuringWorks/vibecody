# Long Session Manager
Manage autonomous 7+ hour sessions by budgeting tokens, turns, and wall-time, then deciding to continue, compact, or halt.

## When to Use
- Running unattended overnight or multi-hour agent sessions
- Detecting when context compaction is needed before a session degrades
- Enforcing hard resource caps to avoid runaway token spend

## Commands
- `SessionManager::with_defaults()` — create a manager with 2M tokens / 500 turns / 7h budget
- `SessionManager::new(budget)` — custom budget
- `manager.decide(&state, now_secs)` — returns `Continue`, `CompactAndContinue`, or `Halt(reason)`
- `manager.should_compact(&state, now_secs)` — true when any dimension exceeds 75%
- `manager.budget_status(&state, now_secs)` — fractional consumption per dimension
- `manager.budget_remaining(&state, now_secs)` — absolute remaining capacity
- `state.record_turn(tokens, tool_calls)` — update counters after each turn

## Examples
```rust
let mut state = SessionState::new("my-session", unix_now());
let mgr = SessionManager::with_defaults();

loop {
    state.record_turn(run_turn(), tool_calls);
    match mgr.decide(&state, unix_now()) {
        ContinuationDecision::Continue => continue,
        ContinuationDecision::CompactAndContinue => {
            compact_context();
            state.compactions += 1;
        }
        ContinuationDecision::Halt(reason) => {
            eprintln!("session halted: {reason}");
            break;
        }
    }
}
```
