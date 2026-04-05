---
layout: page
title: Troubleshooting
permalink: /troubleshooting/
---


This guide covers the most common issues you may encounter when installing, configuring, or using VibeCody. Each issue follows the format: **Symptom**, **Cause**, **Solution**.

## Installation Issues

### Rust compilation errors — missing system dependencies

**Symptom:** `cargo build` fails with linker errors or missing header files.

**Cause:** System-level C libraries or build tools are not installed.

**Solution (by OS):**

**macOS:**

```bash
xcode-select --install
brew install cmake pkg-config
```

**Ubuntu / Debian:**

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev cmake \
  libclang-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
  libwebkit2gtk-4.1-dev
```

**Fedora / RHEL:**

```bash
sudo dnf groupinstall "Development Tools"
sudo dnf install openssl-devel cmake clang-devel gtk3-devel \
  libsoup3-devel webkit2gtk4.1-devel javascriptcoregtk4.1-devel
```

**Arch Linux:**

```bash
sudo pacman -S base-devel openssl cmake clang gtk3 libsoup3 webkit2gtk-4.1
```

### Node.js version mismatch

**Symptom:** `npm install` fails with engine compatibility warnings or syntax errors in build scripts.

**Cause:** VibeCody requires Node.js LTS 18 or later.

**Solution:**

```bash
node --version  # Check your current version

# Using nvm
nvm install --lts
nvm use --lts

# Or download directly from https://nodejs.org/
```

### Tauri prerequisites missing (Linux)

**Symptom:** `npm run tauri:dev` fails with errors about `webkit2gtk` or `libappindicator`.

**Cause:** Tauri v2 requires platform-specific WebView libraries.

**Solution:**

```bash
# Ubuntu / Debian
sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf

# Fedora
sudo dnf install webkit2gtk4.1-devel libappindicator-gtk3-devel \
  librsvg2-devel

# See: https://tauri.app/start/prerequisites/
```

On macOS, WebView is bundled with the OS. On Windows, WebView2 is typically pre-installed on Windows 10/11.

### npm install failures

**Symptom:** `npm install` in `vibeui/` fails with permission errors or dependency conflicts.

**Cause:** Stale lockfile, corrupted cache, or permission issues.

**Solution:**

```bash
cd vibeui

# Clean and retry
rm -rf node_modules package-lock.json
npm cache clean --force
npm install

# If permission errors persist (Linux/macOS)
sudo chown -R $(whoami) ~/.npm
npm install
```

## Provider Connection Issues

### "API key not set"

**Symptom:** Error message like `ANTHROPIC_API_KEY not set` when using a provider.

**Cause:** The required environment variable for the selected provider is not exported.

**Solution:**

Set the appropriate environment variable for your provider:

```bash
# Claude (Anthropic)
export ANTHROPIC_API_KEY="sk-ant-..."

# OpenAI / GPT
export OPENAI_API_KEY="sk-..."

# Google Gemini
export GEMINI_API_KEY="AIza..."

# Grok (xAI)
export GROK_API_KEY="xai-..."

# OpenRouter (multi-model gateway)
export OPENROUTER_API_KEY="sk-or-..."
```

Add the export to your shell profile (`~/.bashrc`, `~/.zshrc`) so it persists across sessions. Alternatively, configure the key in `~/.vibecli/config.toml`:

```toml
[provider]
name = "claude"
api_key = "sk-ant-..."
```

### "Connection refused" — Ollama not running

**Symptom:** `Connection refused` when provider is set to `ollama`.

**Cause:** The Ollama service is not running locally.

**Solution:**

```bash
# Start Ollama
ollama serve

# Verify it is listening
curl http://localhost:11434/api/tags

# Pull a model if needed
ollama pull llama3
```

If you changed the Ollama port, update your config:

```toml
[provider]
name = "ollama"
api_url = "http://localhost:11434"
```

### "Rate limited" (HTTP 429)

**Symptom:** Requests fail with `429 Too Many Requests` or `rate_limit_exceeded`.

**Cause:** You have exceeded the API rate limit for your provider or tier.

**Solution:**

1. Wait a few minutes and retry. Most providers reset per-minute quotas.
2. Upgrade your API plan for higher rate limits.
3. Use the **FailoverProvider** to automatically fall back to another provider:

```toml
[provider]
name = "failover"
chain = ["claude", "openai", "gemini"]
```

1. For heavy usage, consider running a local model via Ollama.

### "Model not found"

**Symptom:** Error like `model 'gpt-5-turbo' not found` or `invalid_model`.

**Cause:** The configured model name is incorrect or deprecated.

**Solution:**

Check the correct model name for your provider:

```bash
# List available Ollama models
ollama list

# Check VibeCody's provider defaults
vibecli config show
```

Common correct model names:

- Claude: `claude-sonnet-4-6`
- OpenAI: `gpt-4o`
- Gemini: `gemini-2.5-flash`
- DeepSeek: `deepseek-chat`

### Timeout errors

**Symptom:** Requests fail with `timeout` or `deadline exceeded`.

**Cause:** Slow network, large context window, or overloaded provider.

**Solution:**

Increase the timeout in `~/.vibecli/config.toml`:

```toml
[provider]
timeout_secs = 120  # default is 60
```

For slow connections or large models, values of 120-300 seconds may be needed. Also check your network connectivity and try a different provider to isolate the issue.

### SSL certificate errors

**Symptom:** `certificate verify failed` or `SSL_ERROR_SYSCALL`.

**Cause:** Corporate proxy, outdated CA certificates, or self-signed certificates.

**Solution:**

```bash
# Update CA certificates (Linux)
sudo update-ca-certificates

# If behind a corporate proxy, set the CA bundle
export SSL_CERT_FILE=/path/to/corporate-ca-bundle.crt
export REQUESTS_CA_BUNDLE=/path/to/corporate-ca-bundle.crt
```

For self-hosted providers, you can set a custom API URL with your internal CA.

## Agent Issues

### Agent stuck in a loop

**Symptom:** The agent keeps repeating similar actions without making progress.

**Cause:** Ambiguous instructions, insufficient context, or the model is not capable enough for the task.

**Solution:**

1. **Interrupt** the agent with `Ctrl+C` (CLI) or the Stop button (VibeUI).
2. Set a maximum step limit in config:

```toml
[agent]
max_steps = 25  # default is 50
```

1. Break the task into smaller, more specific sub-tasks.
2. Try a more capable model (e.g., switch from a local model to Claude).

### Agent making unwanted changes

**Symptom:** The agent edits files you did not want modified.

**Cause:** Approval policy is set to `full-auto` or `auto-edit`.

**Solution:**

Switch to `suggest` mode for manual approval of every change:

```toml
[agent]
approval_policy = "suggest"  # Require approval for each edit
```

To undo agent changes:

```bash
# Undo the last set of changes
git checkout -- .

# Or use VibeCody's built-in checkpoint system
vibecli session restore --checkpoint latest
```

### "Tool not available" in sandbox

**Symptom:** Agent reports a tool is unavailable when running inside a sandbox.

**Cause:** The sandbox configuration restricts certain tools for security.

**Solution:**

Check your sandbox configuration in `~/.vibecli/config.toml`:

```toml
[sandbox]
enabled = true
allow_network = true       # Set to true if tools need network access
allowed_tools = ["read", "write", "bash", "search"]
```

Some tools (e.g., `bash`) may be restricted by default in sandbox mode. Add the required tool to `allowed_tools` if you trust the context.

### Session resume not working

**Symptom:** `vibecli session resume` fails or loads an empty session.

**Cause:** Session file corrupted, or the session was from an incompatible version.

**Solution:**

```bash
# List available sessions
vibecli session list

# Try resuming a specific session by ID
vibecli session resume --id <session-id>

# If corrupted, start fresh
vibecli session new
```

Session data is stored in `~/.vibecli/sessions/`. You can inspect or delete individual session files there.

## VibeUI Issues

### Blank screen on launch

**Symptom:** VibeUI window opens but shows a blank white or black screen.

**Cause:** WebView rendering issue, often related to GPU drivers or missing WebView runtime.

**Solution:**

**Linux:**

```bash
# Ensure webkit2gtk is installed
sudo apt install libwebkit2gtk-4.1-dev

# Try disabling GPU acceleration
WEBKIT_DISABLE_COMPOSITING_MODE=1 npm run tauri:dev
```

**Windows:**

- Ensure WebView2 Runtime is installed: [Microsoft WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
- Update your GPU drivers.

**macOS:**

- Update to the latest macOS version. WebView is bundled with the OS.

### Monaco editor slow with large files

**Symptom:** Editor becomes unresponsive when opening files larger than ~1 MB.

**Cause:** Monaco renders the full file in the DOM, which is expensive for large files.

**Solution:**

1. Disable the minimap in VibeUI settings (Settings > Editor > Minimap: off).
2. Disable word wrap for very large files.
3. For files over 5 MB, consider using the terminal-based editor or an external editor.

### Terminal panel not working

**Symptom:** The integrated terminal shows errors or does not accept input.

**Cause:** PTY allocation failure, often a permissions issue on Linux.

**Solution:**

```bash
# Check PTY permissions (Linux)
ls -la /dev/pts/

# Ensure your user is in the tty group
sudo usermod -aG tty $(whoami)

# Restart VibeUI after group change
```

On macOS, ensure Terminal has Full Disk Access in System Settings > Privacy & Security.

### Panels not loading

**Symptom:** A panel tab shows "Loading..." indefinitely or displays an error.

**Cause:** The Tauri command backing the panel may have failed or the panel name is not registered.

**Solution:**

1. Check the developer console: `Cmd+Shift+I` (macOS) or `Ctrl+Shift+I` (Linux/Windows).
2. Look for errors in the console output.
3. Ensure you are running the latest build:

```bash
cd vibeui
npm run tauri:dev
```

1. If a specific panel consistently fails, report the issue with the console error message.

## Build Issues

### cargo build fails

**Symptom:** Workspace build fails with feature or dependency errors.

**Cause:** Some crates have optional dependencies or platform-specific features.

**Solution:**

```bash
# Build only VibeCLI (most common)
cargo build --release -p vibecli

# Check workspace excluding optional crates
cargo check --workspace --exclude vibe-collab

# If lockfile is stale
cargo update
```

### Tauri build fails

**Symptom:** `npm run tauri:build` fails during the Rust compilation or bundling step.

**Cause:** Missing system prerequisites or incorrect Tauri configuration.

**Solution:**

Run through this checklist:

1. Rust toolchain is up to date: `rustup update stable`
2. Node.js is LTS 18+: `node --version`
3. System dependencies are installed (see Installation Issues above)
4. Clean and rebuild:

```bash
cd vibeui
rm -rf node_modules src-tauri/target
npm install
npm run tauri:build
```

### Docker build fails

**Symptom:** `docker build` fails during the multi-stage Rust compilation.

**Cause:** Insufficient memory, missing build context, or base image issues.

**Solution:**

```bash
# Ensure Docker has enough memory (at least 4 GB)
docker build -t vibecody .

# For Apple Silicon Macs, specify platform
docker build --platform linux/amd64 -t vibecody .

# Use docker-compose for the full stack (VibeCLI + Ollama)
docker-compose up
```

## Performance

### High memory usage

**Symptom:** VibeCody process consumes several GB of RAM during long sessions.

**Cause:** Large context windows accumulate tokens over extended sessions.

**Solution:**

1. Limit the context window size:

```toml
[agent]
max_context_tokens = 100000  # Reduce from default
```

1. Clear session history periodically:

```bash
vibecli session new  # Start a fresh session
```

1. Use the `/compact` REPL command to compress context in the current session.
2. Enable session memory profiling to detect leaks: `/metering status`

### Slow responses from AI provider

**Symptom:** Each response takes 10+ seconds.

**Cause:** Large model, long context, or provider latency.

**Solution:**

1. Use a faster model (e.g., `gemini-2.5-flash` or a smaller Ollama model).
2. Reduce context by starting a new session or using context bundles.
3. Check provider status pages for outages.
4. For local models, ensure you have sufficient GPU VRAM:

```bash
# Check GPU availability
nvidia-smi  # NVIDIA
ollama list  # See which models are loaded
```

### Large repository indexing is slow

**Symptom:** Opening a large repository causes long startup delays.

**Cause:** VibeCody indexes the repository for search and context.

**Solution:**

Add exclude patterns to your project configuration:

```toml
# .vibecli/config.toml or project-level config
[index]
exclude = [
  "node_modules",
  "target",
  "dist",
  ".git",
  "vendor",
  "*.min.js",
]
```

For repositories with more than 100,000 files, consider indexing only the directories you are actively working in.

## Productivity Integrations

### Email: "Email not configured"

**Symptom:** `/email unread` returns "Email not configured. Set GMAIL_ACCESS_TOKEN or OUTLOOK_ACCESS_TOKEN."

**Cause:** No OAuth2 token is available.

**Solution:**

```bash
# Gmail — set the access token from Google OAuth2 Playground
export GMAIL_ACCESS_TOKEN="ya29.xxxx"

# Or add to ~/.vibecli/config.toml:
# [email]
# provider = "gmail"
# access_token = "ya29.xxxx"
```

To obtain a Gmail token interactively, use the [Google OAuth2 Playground](https://developers.google.com/oauthplayground) with scope `https://www.googleapis.com/auth/gmail.modify`.

For Outlook, obtain a Microsoft Graph token with scope `https://graph.microsoft.com/Mail.ReadWrite`.

---

### Email: "401 Unauthorized"

**Symptom:** Email commands return a 401 error.

**Cause:** OAuth2 access token has expired (tokens typically expire after 1 hour).

**Solution:** Re-obtain a fresh token using the OAuth2 flow and update `GMAIL_ACCESS_TOKEN` or `[email].access_token` in config.

---

### Calendar: "Calendar not configured"

**Symptom:** `/cal today` returns a "not configured" message.

**Solution:**

```bash
export GOOGLE_CALENDAR_TOKEN="ya29.xxxx"   # Google Calendar
export OUTLOOK_CALENDAR_TOKEN="eyJ0..."    # Outlook Calendar
```

Or add to `~/.vibecli/config.toml`:
```toml
[calendar]
provider = "google"
access_token = "ya29.xxxx"
```

---

### Home Assistant: "Connection refused" or "HA not configured"

**Symptom:** `/ha status` fails to connect.

**Cause:** Home Assistant URL or token is missing/wrong, or HA is not accessible on the network.

**Solution:**

1. Verify your HA instance is reachable: `curl http://homeassistant.local:8123/api/`
2. Check the token: Settings → Profile → Long-Lived Access Tokens → Create token
3. Update config:

```toml
[home_assistant]
url   = "http://homeassistant.local:8123"
token = "eyJ0..."
```

For remote access without VPN, use [Nabu Casa](https://www.nabucasa.com/) or Tailscale and set the remote URL.

For self-signed TLS certificates on local HA: set `insecure = true` in `[home_assistant]`.

---

### Jira: "401 Unauthorized" or "JIRA not configured"

**Symptom:** `/jira mine` returns an auth error.

**Cause:** Missing or incorrect Jira credentials.

**Solution:**

```bash
export JIRA_URL="https://yourorg.atlassian.net"
export JIRA_EMAIL="you@yourorg.com"
export JIRA_API_TOKEN="ATATT3xxx"    # Not your password — API token from id.atlassian.com
```

Or in `~/.vibecli/config.toml`:
```toml
[jira]
url   = "https://yourorg.atlassian.net"
email = "you@yourorg.com"
token = "ATATT3xxx"
```

Generate tokens at: https://id.atlassian.com/manage-profile/security/api-tokens

---

### Notion: "0 results" for known pages

**Symptom:** `/notion search "my page"` returns empty results.

**Cause:** The Notion integration has not been shared with that page/database.

**Solution:**

1. Open the page in Notion
2. Click **Share** (top right)
3. Search for and select your integration (the one with the `NOTION_API_KEY`)
4. Click **Invite**

The API only returns pages explicitly shared with the integration.

---

### Todoist: "401 Unauthorized"

**Symptom:** `/todo list` fails with an auth error.

**Cause:** Invalid or missing Todoist API token.

**Solution:**

1. Open Todoist → Settings → Integrations → Developer → API token
2. Copy the token and set: `export TODOIST_API_KEY="xxxx"` or add `todoist_api_key = "xxxx"` to `~/.vibecli/config.toml`

---

### VibeUI Productivity Panel: "vibecli not found on PATH"

**Symptom:** The Productivity panel in VibeUI shows "vibecli not found on PATH".

**Cause:** The `vibecli` binary is not on the system PATH when VibeUI is launched.

**Solution:**

```bash
# Verify vibecli is installed
vibecli --version

# If not found, install it
curl -fsSL https://vibecody.github.io/install.sh | sh

# Or add to PATH in your shell profile (~/.zshrc / ~/.bashrc):
export PATH="$HOME/.local/bin:$PATH"
```

On macOS, apps launched from Finder or the Dock may not inherit the full shell PATH. Start VibeUI from a terminal: `open -a VibeUI` or `npx tauri dev`.

---

## Still Stuck?

If none of the above solutions resolve your issue:

1. Run the built-in doctor command for automated diagnostics:

```bash
vibecli doctor
```

1. Check [GitHub Issues](https://github.com/TuringWorks/vibecody/issues) for known problems.
2. Open a new issue with:
   - Your OS and version
   - VibeCody version (`vibecli --version`)
   - Full error output
   - Steps to reproduce
