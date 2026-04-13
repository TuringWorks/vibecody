# Focus View
Distraction-free UI session mode with configurable notification suppression, auto-exit timers, and distraction tracking.

## When to Use
- Entering deep work / flow state in the editor (hide panels, silence notifications)
- Tracking how many distractions occurred during a session
- Automatically exiting focus mode after a configured time limit

## Commands
- `FocusConfig::default_deep()` — maximal focus: hide all panels, silent notifications, dim unfocused
- `FocusConfig::default_light()` — light focus: panels visible, minimal notifications
- `FocusManager::new()` — create manager with no active session
- `manager.enter_focus(config, now_secs)` — start a focus session; returns `&FocusSession`
- `manager.exit_focus(now_secs)` — end active session; moves it to history
- `manager.record_distraction()` — increment distraction counter on the active session
- `manager.is_in_focus()` — whether a session is currently active
- `manager.session_count()` — number of completed historical sessions
- `manager.should_auto_exit(now_secs)` — true if active session has exceeded its `auto_exit_after_secs`
- `session.duration_secs(now_secs)` — elapsed seconds (uses `ended_at` if session is closed)

## Examples
```rust
let mut mgr = FocusManager::new();
let mut cfg = FocusConfig::default_deep();
cfg.auto_exit_after_secs = Some(3600); // 1 hour

mgr.enter_focus(cfg, unix_now());

// During work
mgr.record_distraction(); // user switched away

// Poll auto-exit
if mgr.should_auto_exit(unix_now()) {
    let session = mgr.exit_focus(unix_now()).unwrap();
    println!("session had {} distractions", session.distraction_count);
}
```
