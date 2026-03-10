# Feature Demo Recording

Record, generate, and export interactive feature demos using browser automation (CDP).

## Commands

- `/demo list` — List all recorded demos
- `/demo run <name> <steps-json>` — Record a demo by executing JSON steps
- `/demo generate <description>` — AI-generate demo steps from a feature description
- `/demo export <id> [html|md]` — Export a demo as an HTML slideshow or markdown

## Demo Steps

Steps are JSON arrays with these action types:

| Action | Fields | Description |
|--------|--------|-------------|
| `navigate` | `url` | Navigate browser to URL |
| `click` | `selector` | Click a CSS-selected element |
| `type` | `selector`, `text` | Type text into an input |
| `wait` | `ms` | Wait N milliseconds |
| `screenshot` | `label` | Capture a labeled screenshot |
| `assert` | `selector`, `expected` | Assert element text matches |
| `narrate` | `text` | Add narration to the demo |
| `eval_js` | `script` | Execute JavaScript in the page |
| `scroll` | `selector`, `direction` | Scroll an element (up/down) |
| `wait_for_selector` | `selector`, `timeout_ms` | Wait for element to appear |

## Example

```json
[
  {"action": "navigate", "url": "http://localhost:3000"},
  {"action": "screenshot", "label": "Home page"},
  {"action": "click", "selector": "#login-btn"},
  {"action": "type", "selector": "#email", "text": "user@example.com"},
  {"action": "screenshot", "label": "Login form filled"},
  {"action": "narrate", "text": "User logs in with email"}
]
```

## Export Formats

- **HTML** — Self-contained slideshow with keyboard navigation and frame thumbnails
- **Markdown** — Document with embedded screenshots and step descriptions

## Architecture

- `feature_demo.rs` — Core: DemoStep, DemoRunner, DemoGenerator, DemoExporter, BrowserSession (CDP)
- `DemoPanel.tsx` — VibeUI panel with list, create, and AI-generate tabs
- Demos stored at `~/.vibecli/demos/<name>-<timestamp>/demo.json`
- Screenshots saved as PNG frames alongside each demo
