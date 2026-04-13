# Computer Use
Desktop automation action model — represent, validate, and serialize GUI actions (click, type, screenshot, scroll, key press).

## When to Use
- Building desktop automation workflows or agent-driven UI testing
- Validating that action coordinates are within screen bounds before execution
- Serializing action plans for replay or logging

## Commands
- `Action::Click { x, y, button }` — mouse click at coordinates
- `Action::Type { text }` — type text (destructive)
- `Action::Screenshot` — capture the screen (non-destructive)
- `Action::is_destructive()` — detect state-mutating actions
- `Action::description()` — human-readable action summary
- `ScreenBounds::validate_action(action)` — reject out-of-bounds coordinates
- `ActionPlan::new(goal).add(action)` — fluent plan builder
- `ActionPlan::destructive_count()` — count dangerous steps

## Examples
```rust
use vibecli_cli::computer_use::{Action, ActionPlan, MouseButton, Key, ScreenBounds};

let bounds = ScreenBounds::new(1920, 1080);
let plan = ActionPlan::new("Submit login form")
    .add(Action::Click { x: 400, y: 300, button: MouseButton::Left })
    .add(Action::Type { text: "user@example.com".into() })
    .add(Action::KeyPress { key: Key::Enter });

assert!(bounds.validate_action(&plan.steps[0]).is_ok());
assert_eq!(plan.destructive_count(), 2); // Type + Enter
```
