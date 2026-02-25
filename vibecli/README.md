# VibeCLI

**AI-powered coding assistant for the terminal.**

VibeCLI is a Rust-based terminal tool designed to bring the power of LLMs directly into your workflow. It offers a rich Terminal UI (TUI) for chatting with AI, generating code, and reviewing changes with context awareness.

## Features

- 🖥️ **Terminal UI (TUI)**: A modern, single-pane interface for seamless interaction.
- 🤖 **Multi-LLM Support**:
  - **Ollama** (Local, Private)
  - **OpenAI** (GPT-4)
  - **Anthropic** (Claude 3.5 Sonnet/Opus)
  - **Google Gemini** (Pro 1.5)
  - **xAI Grok** (Beta)
- 🐙 **Git Context Awareness**: Automatically detects your current branch, status, and diffs to give the AI full context of your work.
- 📊 **Smart Diff Viewing**: Review multi-file changes with syntax highlighting before applying them.
- 💬 **Interactive Chat**: Natural language conversation with code generation capabilities.
- ⚙️ **Flexible Configuration**: TOML-based configuration for providers and UI settings.

## Installation

### Prerequisites

- Rust (latest stable)
- `git`

### Build from Source

```bash
git clone https://github.com/vibecody/vibecli.git
cd vibecli
cargo build --release
```

The binary will be available at `target/release/vibecli`. You can move it to your PATH:

```bash
cp target/release/vibecli /usr/local/bin/
```

## Usage

### Launching the TUI (Recommended)

The Terminal UI provides the best experience with syntax highlighting, scrollable history, and visual diffs.

```bash
vibecli --tui
```

### Command Line Arguments

- `--provider <name>`: Select LLM provider (default: `ollama`). Options: `ollama`, `openai`, `claude`, `gemini`, `grok`.
- `--model <name>`: Override the default model for the selected provider.
- `--tui`: Launch the Terminal UI mode.

**Example:**

```bash
vibecli --tui --provider openai --model gpt-4-turbo
```

### Interactive Commands (TUI)

Once inside the TUI, you can type naturally or use slash commands:

- `/chat <message>` - Start a new chat context (or just type your message).
- `/files` - Switch to File Tree view (Coming Soon).
- `/diff [file]` - View git diffs.
  - `/diff` (no args): Shows the full multi-file git diff of your current workspace.
  - `/diff <file>`: Shows the content of a specific file.
- `/quit` or `/exit` - Exit the application.
- `Tab` - Toggle between Chat and other views.

## Configuration

VibeCLI uses a TOML configuration file located at `~/.vibecli/config.toml`.

**Example Configuration:**

```toml
[ollama]
enabled = true
api_url = "http://localhost:11434"
model = "qwen3-coder:480b-cloud"

[openai]
enabled = false
api_key = "sk-..."
model = "gpt-4-turbo"

[claude]
enabled = false
api_key = "sk-ant-..."
model = "claude-3-opus-20240229"

[gemini]
enabled = false
api_key = "AIza..."
model = "gemini-pro"

[grok]
enabled = false
api_key = "..."
model = "grok-beta"

[ui]
theme = "dark"

[safety]
require_approval_for_commands = true
```

### Environment Variables

You can also use environment variables for API keys (these take precedence over config file if not set in config):

- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`
- `GROK_API_KEY`

## Development

### Project Structure

```text
vibecli/
├── vibecli-core/          # Core logic (LLM, Git, Config, Diff)
│   ├── src/
│   │   ├── llm/          # Provider implementations
│   │   ├── git.rs        # Git operations
│   │   └── config.rs     # Configuration management
└── vibecli-cli/          # CLI & TUI implementation
    ├── src/
    │   ├── tui/          # Ratatui-based interface
    │   │   ├── components/ # UI Components (Chat, DiffView)
    │   │   └── app.rs    # State management
    │   └── main.rs       # Entry point
```

### Running Tests

```bash
cargo test
```

## License

MIT
