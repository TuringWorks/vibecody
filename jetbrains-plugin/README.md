# VibeCLI JetBrains Plugin

AI coding assistant powered by [VibeCLI](../README.md), for IntelliJ IDEA and all JetBrains IDEs.

## Features

| Feature | Shortcut |
|---------|----------|
| Open AI panel (Chat / Agent / Jobs) | `Ctrl+Shift+A` |
| Inline AI edit on selection | `Ctrl+Shift+K` |
| Chat — single-turn Q&A | Tool window → Chat tab |
| Agent — multi-step task with live streaming | Tool window → Agent tab |
| Job history (persisted across daemon restarts) | Tool window → Jobs tab |

## Requirements

- IntelliJ IDEA 2024.1+ (or any JetBrains IDE 2024.1+)
- `vibecli` installed and the daemon running:
  ```bash
  vibecli --serve --port 7878 --provider ollama
  ```

## Installation

### From source (development build)

```bash
cd jetbrains-plugin
./gradlew buildPlugin
# Install the ZIP from build/distributions/ via
# IDE Settings → Plugins → ⚙ → Install Plugin from Disk
```

### From JetBrains Marketplace *(coming soon)*

Search for **VibeCLI** in the plugin marketplace.

## Configuration

Open **IDE Settings → Tools → VibeCLI**:

| Setting | Default | Description |
|---------|---------|-------------|
| Daemon URL | `http://localhost:7878` | URL of the running `vibecli serve` daemon |
| Provider | `ollama` | AI provider (`ollama`, `claude`, `openai`, `gemini`, `grok`) |
| Model | `qwen2.5-coder:7b` | Model name passed to the provider |
| Approval mode | `suggest` | `suggest` / `auto-edit` / `full-auto` |

## Architecture

```
IDE plugin  ──HTTP──▶  vibecli serve daemon  ──▶  AI provider
               SSE ◀──  (port 7878)
```

- `VibeCLIService.kt` — thin HTTP client; all network calls run on background threads
- `AgentToolWindow.kt` — Swing panels (Chat / Agent / Jobs)
- `InlineEditAction.kt` — Ctrl+Shift+K action using `WriteCommandAction` (undoable)
- `VibeCLISettings.kt` — persistent settings stored in `vibecli.xml`
- `VibeCLISettingsConfigurable.kt` — IDE settings page (Tools → VibeCLI)

## Build system

Uses the **IntelliJ Platform Gradle Plugin 2.x**:
```bash
./gradlew runIde        # launch IDE with plugin loaded
./gradlew buildPlugin   # produce installable ZIP
./gradlew verifyPlugin  # run plugin verifier
./gradlew publishPlugin # publish to marketplace (needs PUBLISH_TOKEN)
```
