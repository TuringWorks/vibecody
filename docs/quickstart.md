---
layout: page
title: Quickstart
permalink: /quickstart/
---


**Zero to productive in 5 minutes.**


## What is VibeCody?

VibeCody is an AI-powered developer toolchain built in Rust. It gives you many ways to work: **VibeCLI** (terminal), **VibeCoder** (desktop editor, 246 panels), **VibeAIChat** (desktop chat), **VibeMobile** (Flutter, 6 platforms), and **VibeWatch** (native Apple Watch + Wear OS). All surfaces share the same backend crates, supporting 25 AI providers, an autonomous agent loop, code review, multi-agent orchestration, MCP integration, and 1,144 built-in skills. Start with a local model and zero API keys — and with zero-config mDNS / Tailscale / ngrok connectivity, your phone and watch find your desktop automatically.


## Choose Your Surface

| | **VibeCLI** | **VibeCoder** | **VibeAIChat** |
|---|---|---|---|
| **Best for** | Terminal, CI/CD, scripting | Visual editing, panel-rich workflows | Quick chat beside your work |
| **Interface** | TUI (Ratatui) or REPL | Desktop app (Tauri + Monaco) | Small desktop window / menu bar |
| **Install** | One-liner or a tarball | Download a `.dmg` / `.msi` / `.AppImage` | Same |
| **Works headless** | Yes | No | No |
| **AI features** | All 25 providers, agent, review, skills | All of it, plus visual panels | Chat, voice, providers |

**Recommendation:** if you live in a terminal, start with VibeCLI. Otherwise
download VibeCoder — it starts the daemon for you, so there is nothing else to
install and nothing to configure.


## Install

Two paths, and which one you want depends on whether you intend to change the
code.

### For everyone: install the release build

Signed, notarized, and built by CI with SHA-256 checksums. **No toolchain, no
Rust, no Node.js.**

**Desktop apps** — download from the [Releases page]({{ site.baseurl }}/release/):

| Platform | Take |
|---|---|
| macOS (Apple Silicon / Intel) | the `.dmg` for your architecture |
| Windows | `-setup.exe` (installer) or `_en-US.msi` |
| Linux | `.AppImage` (portable) or `.deb` |

The same three shells are published each release — **VibeCoder** (editor),
**VibeDesk** (task shell), **VibeAIChat** (chat). Each starts the VibeCLI daemon
on launch and reuses one already running, so installing more than one is fine.

**VibeCLI** — one line, macOS and Linux, x86_64 and ARM:

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

It resolves the latest release, verifies the checksum, and installs to
`~/.local/bin/vibecli`. Override the location with `INSTALL_DIR=/usr/local/bin`.
Windows and anyone who prefers to see what they are running can take the
`vibecli-*` archive from the Releases page directly.

**Mobile and watch** — `.ipa`, `.apk`/`.aab`, watchOS `.app.zip` and Wear OS
builds are on the same Releases page. iOS and watchOS are unsigned; sideload via
AltStore / Sideloadly / Xcode.

Check what you got:

```bash
vibecli --version
```

### For developers: git and make

Building from source is for changing VibeCody, not for using it. It needs Rust
stable and Node.js, and `make setup` installs both along with the system
libraries:

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody
make setup      # Rust, Node.js, system libs, npm deps
make doctor     # verify the toolchain before you build anything
```

Then build or run whichever surface you are working on:

```bash
make cli                    # build target/release/vibecli
make ui                     # run VibeCoder in dev mode
make vibedesk               # run VibeDesk
make aichat                 # run VibeAIChat
make build-all              # everything CI builds
```

`make help` lists every target; `make help-surfaces` prints the
`build-<surface>` / `test-<surface>` matrix. See the [Development
Guide]({{ site.baseurl }}/development/).

> **A source build is not a release build.** It is unsigned, unnotarized, and
> built with your local toolchain. On macOS that means Gatekeeper will complain,
> and `codesign -dv` will say `adhoc`. That is expected — it is not the artifact
> the Releases page publishes.

### Either way: Docker

Runs the daemon with Ollama alongside it, no host dependencies and no API key:

```bash
git clone https://github.com/TuringWorks/vibecody.git
cd vibecody
docker-compose up
```

A prebuilt image tarball (`vibecli-docker-*.tar.gz`) ships with each release for
air-gapped installs.

### How much machine you need

A cloud provider needs almost nothing — 2 GB of RAM will do. Running a model
locally is a different question, and
[Sizing & Hardware]({{ site.baseurl }}/sizing/) answers it: VRAM by model size
and quantisation, what actually uses a GPU, and what the setup wizard decides
for you.


## Your First Chat

Launch VibeCLI with no arguments to enter REPL mode:

```bash
vibecli
```

You will see the prompt:

```
VibeCLI v0.5.12 — AI coding assistant
Provider: ollama (glm-5.2:cloud)
Type a message or /help for commands.

vibecli>
```

Type a question:

```sh
vibecli> What does the #[derive(Debug)] macro do in Rust?
```

Expected output (streamed):

```sh
The #[derive(Debug)] attribute macro automatically implements the
`Debug` trait for a struct or enum, allowing you to print it with
`{:?}` formatting in println!, dbg!, or format!.

Example:
  #[derive(Debug)]
  struct Point { x: f64, y: f64 }

  let p = Point { x: 1.0, y: 2.0 };
  println!("{:?}", p);  // Point { x: 1.0, y: 2.0 }
```

That is it -- you are chatting with an AI. Press `Ctrl+C` or type `/quit` to exit.


## Your First Agent Task

The agent loop lets VibeCody autonomously read files, write code, and run commands. Use `--agent` for interactive mode or `--exec` for non-interactive (CI) mode:

```bash
# Interactive mode (asks for approval on each step)
vibecli --agent "add error handling to main.rs"

# Non-interactive mode (full-auto, JSON output)
vibecli --exec "add error handling to main.rs"
```

Example output (interactive mode with default `suggest` policy):

```sh
 Agent   add error handling to main.rs
  Policy: suggest (ask before every action)  |  Press Ctrl+C to stop

 ✓ Reading src/main.rs
 ✓ Searching: "error handling"

  bash  Running: cargo check
   Approve? (y/n/a=approve-all): y

 ✓ Running: cargo check
 ✓ Patching src/main.rs (3 hunks)

Agent complete: Added Result<()> return type, wrapped I/O in match blocks.
   Files modified: src/main.rs
   Commands run: 1
   Steps: 4/4 succeeded
   Trace saved: ~/.vibecli/traces/1711234567.jsonl
   Resume with: vibecli --resume 1711234567
```

In `suggest` mode (default), the agent asks before shell commands and file writes. Type `y` to approve, `n` to reject, or `a` to auto-approve all remaining steps.

### Approval Policies

| Flag | Behavior |
|------|----------|
| *(default)* | Ask before every edit and command |
| `--auto-edit` | Auto-apply file edits; ask before shell commands |
| `--full-auto` | Auto-execute everything (use with `--sandbox`) |

You can also use `/agent <task>` from the REPL to start agent tasks interactively, and `/plan <task>` to review a plan before executing.


## Connect a Cloud Provider

Local Ollama works with no key at all. Cloud providers give you larger models.

**Step 1:** Get an API key — for Claude, from
[console.anthropic.com](https://console.anthropic.com/).

**Step 2:** Store it. `set-key` writes to the **encrypted ProfileStore**
(`~/.vibecli/profile_settings.db`), which every surface reads — CLI, the three
desktop shells, mobile, watch:

```bash
vibecli set-key claude sk-ant-your-key-here
```

> **Prefer this over an environment variable.** `ANTHROPIC_API_KEY` and
> `OPENAI_API_KEY` are still honoured as a fallback, but an exported variable
> reaches only the shell that exported it — the desktop apps, launched from
> Finder or a launcher, will not see it. And a key in `~/.bashrc` is a plaintext
> secret on disk. Nothing should ever put a key in `config.toml`.

**Step 3:** Launch with it:

```bash
vibecli --provider claude
```

Expected output:

```sh
VibeCLI v0.5.12 — AI coding assistant
Provider: claude (claude-opus-5)

vibecli>
```

**Step 4:** Verify:

```sh
vibecli> Hello, which model am I talking to?
```

Other providers follow the same pattern — `vibecli set-key <provider> <key>`,
then `--provider <provider>`. The default model per provider comes from the
registry, so you only pass `--model` to override it:

| Provider | `set-key` name | Default model |
|---|---|---|
| Claude | `claude` | `claude-opus-5` |
| OpenAI | `openai` | `gpt-5.6-sol` |
| Gemini | `gemini` | `gemini-3.6-flash` |
| Grok | `grok` | `grok-4.5` |
| Ollama | *(none needed)* | whatever you have pulled |

All 25 providers and what each needs: [Third-Party
Services]({{ site.baseurl }}/services/).


## Your First Code Review

Navigate to any Git repository with uncommitted changes and run:

```bash
vibecli --review
```

Or from inside the REPL:

```sh
vibecli> /review
```

Expected output:

```sh
[review] Analyzing diff (3 files, +47 -12 lines)...

## Code Review Summary

### src/auth.rs (2 issues)
  [HIGH] Line 34: Unwrap on network call will panic in production.
         Suggestion: Use `?` operator or handle the error explicitly.
  [MED]  Line 51: Password comparison is not constant-time.
         Suggestion: Use `subtle::ConstantTimeEq` to prevent timing attacks.

### src/main.rs (1 issue)
  [LOW]  Line 12: Unused import `std::collections::HashMap`.
         Suggestion: Remove the import.

3 issues found (1 high, 1 medium, 1 low).
```

You can also review a GitHub PR directly:

```sh
vibecli> /review --pr 42
```

See the [Code Review Tutorial](/vibecody/tutorials/code-review/) for more options.


## Next Steps

You are up and running. Here is where to go next:

| Goal | Link |
|------|------|
| Set up more AI providers | [First Provider Tutorial](/vibecody/tutorials/first-provider/) |
| Learn the agent workflow | [Agent Workflow Tutorial](/vibecody/tutorials/agent-workflow/) |
| Deep-dive on code review | [Code Review Tutorial](/vibecody/tutorials/code-review/) |
| Browse all tutorials | [Tutorials Index](/vibecody/tutorials/) |
| Configure VibeCLI fully | [Configuration Guide](/vibecody/configuration/) |
| Set up the desktop editor | [VibeCoder Reference](/vibecody/vibecoder/) |
| Full CLI reference | [VibeCLI Reference](/vibecody/vibecli/) |
| Pair your phone | [VibeMobile](/vibecody/vibemobile/) |
| Pair your Apple Watch | [watchOS guide](/vibecody/watchos/) |
| Pair your Wear OS watch | [Wear OS guide](/vibecody/wearos/) |
| Zero-config LAN / Internet | [Connectivity](/vibecody/connectivity/) |


## Common Issues

### "Connection refused" when using Ollama

Ollama has to be running before VibeCLI can reach it:

```bash
ollama serve                 # in one terminal
ollama pull qwen2.5-coder:7b # a local model — see /sizing/ for what fits
vibecli --provider ollama
```

### "API key not found" for a cloud provider

Store the key rather than exporting it:

```bash
vibecli set-key claude sk-ant-...
```

An exported `ANTHROPIC_API_KEY` works for the shell you exported it in, and not
for a desktop app launched from Finder or a launcher — which is the usual reason
this appears to be set and still fails. Never put a key in `config.toml`.

### The desktop app says the daemon is unavailable

The shells autostart the daemon on `127.0.0.1:7878`. Three distinct causes,
which look identical on screen:

```bash
curl -s http://127.0.0.1:7878/health
```

- **No answer** — the daemon is not running, or is still starting. A cold
  daemon has been **measured at ~16 s** to first answer; give it that long
  before concluding anything.
- **Answers, but without `"service": "vibecli"`** — something else holds port
  7878. Free the port or set `VIBECLI_DAEMON_PORT`.
- **Answers correctly, but panels still fail with 401** — a stale token. It is
  regenerated on **every** daemon start; restart the app so it re-reads.

### The voice reply sounds robotic, or nothing is spoken

Speech has an install step, and for most of this feature's life nothing ran it:

```bash
make voice-sidecar   # streaming platform speech (macOS)
make voice-kokoro    # the neural engine (Apple Silicon)
make voice-status    # which engine will ACTUALLY run
```

Ask `voice-status` before concluding the voice is broken. On macOS, also check
System Settings → Accessibility → Spoken Content → Manage Voices — Apple's
Enhanced and Premium voices are neural, free, and not installed by default.

**Voice input does not work on Linux at all** in any of the three shells:
WebKitGTK denies microphone capture unless the embedder enables it, and neither
wry nor Tauri does. Not a configuration problem — there is no setting that fixes
it today.

### A source build fails on missing system libraries

Only relevant if you are building from source. On Linux:

```bash
sudo apt install pkg-config libssl-dev     # Ubuntu/Debian
sudo dnf install openssl-devel             # Fedora
```

`make doctor` checks the whole toolchain and is faster than discovering these
one compile error at a time.

### Something else

[Troubleshooting]({{ site.baseurl }}/troubleshooting/) covers the rest; each
[deployment guide]({{ site.baseurl }}/guides/) has a section for its own
platform.
