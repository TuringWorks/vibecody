# 02 — Documented but Not Implemented

> Features described in documentation that do not exist (or are non-functional) in the codebase.

## Missing CLI Commands (P1)

| Command | Documented In | What Docs Say | What Code Has |
|---------|---------------|---------------|---------------|
| `vibecli service install/start` | docs/setup.md:102-106 | Installs as launchd/systemd service | No `service` module exists. Always-on uses `--serve` flag instead |
| `vibecli setup` | docs/setup.md:128-139 | 5-step interactive wizard | `setup.rs` has `run_setup()` but it's not exposed as a CLI subcommand |
| `vibecli doctor` | docs/glossary.md:49 | Built-in diagnostic command | No `doctor` module or command dispatch |
| `vibecli config set tier <name>` | docs/setup.md:158 | Set pricing tier (lite/pro/max) | No tier concept in config.rs |

## Missing CLI Flags (P1)

| Flag | Documented In | What Docs Say | What Code Has |
|------|---------------|---------------|---------------|
| `--api-token` | docs/api-reference.md:51-54 | Set API auth token for serve mode | `serve.rs` generates tokens internally but flag is not in CLI args |

## Missing Panels (P2)

These panels are listed in `docs/PANEL-AUDIT.md` but no `.tsx` file exists (not a naming mismatch):

| Panel Name | Documented Purpose |
|------------|-------------------|
| ComparePanel | Model comparison |
| FlowPanel | Event flow tracking |
| KeysPanel | API key management |
| ModelManagerPanel | AI model management |

## Naming Mismatches — Panel Audit vs Actual Files (P3)

| PANEL-AUDIT.md Name | Actual File |
|---------------------|-------------|
| ChatPanel | `AIChat.tsx` |
| DiscussionPanel | `DiscussionModePanel.tsx` |
| HttpPanel | `HttpPlayground.tsx` |
| MetricsPanel | `CodeMetricsPanel.tsx` |
| TracesPanel | `TraceDashboard.tsx` |

## Missing Provider (P3)

| Provider | Documented In | Status |
|----------|---------------|--------|
| Gemini Native | MEMORY.md (listed as 1 of 18 providers) | No `gemini_native.rs` exists. Only `gemini.rs` |
