---
layout: page
title: "Tutorial: Setting Up Your First AI Provider"
permalink: /tutorials/first-provider/
---

# Setting Up Your First AI Provider

Connect VibeCody to an AI model so you can start chatting, generating code, and running the agent loop.

**Prerequisites:** VibeCody installed and on your PATH. See the [Quickstart](../../quickstart/) if you need to install it first.


## Choose a Provider

VibeCody supports 23 AI providers. Here are the most common starting points:

| Provider | Type | Cost | API Key Required | Best For |
|----------|------|------|-----------------|----------|
| **Ollama** | Local | Free | No | Privacy, offline use, experimentation |
| **Claude** | Cloud | Paid | Yes | Best coding quality, extended thinking |
| **OpenAI** | Cloud | Paid | Yes | GPT-4o, broad ecosystem compatibility |
| **Gemini** | Cloud | Free tier | Yes | Large context windows, free tier |
| **Groq** | Cloud | Free tier | Yes | Ultra-fast inference |
| **DeepSeek** | Cloud | Paid | Yes | Cost-effective coding model |

**Recommendation:** Start with Ollama if you want free and local. Start with Claude if you want the best coding results.


## Option 1: Ollama (Free, Local)

Ollama runs models entirely on your machine. No API key, no network access, no data leaves your laptop.

### Step 1: Install Ollama

```bash
# macOS / Linux
curl -fsSL https://ollama.ai/install.sh | sh

# Or visit https://ollama.ai for the desktop installer
```

### Step 2: Pull a Model

```bash
ollama pull qwen3-coder:480b-cloud
```

This downloads the default model VibeCLI expects. Other good options:

```bash
ollama pull codellama:13b        # Smaller, faster
ollama pull deepseek-coder:33b   # Strong coding model
ollama pull llama3:70b           # General purpose
```

### Step 3: Start Ollama

```bash
ollama serve
```

Leave this running in a separate terminal (or use the Ollama desktop app, which runs the server automatically).

### Step 4: Launch VibeCLI

```bash
vibecli
```

That is it. Ollama is the default provider, so no flags are needed. You should see:

```
VibeCLI v0.3.3 — AI coding assistant
Provider: ollama (qwen3-coder:480b-cloud)

vibecli>
```

### Step 5: Verify

```
vibecli> Write a hello world in Rust
```

You should see streamed output with a working Rust program.

### Using a Different Ollama Model

```bash
vibecli --model codellama:13b
```

Or set it permanently in `~/.vibecli/config.toml`:

```toml
[ollama]
enabled = true
model = "codellama:13b"
```


## Option 2: Claude (Cloud, API Key)

Anthropic's Claude models are among the strongest for code generation, reasoning, and agentic tasks.

### Step 1: Get an API Key

1. Go to [console.anthropic.com](https://console.anthropic.com/)
2. Sign up or log in
3. Navigate to **API Keys** and create a new key
4. Copy the key (starts with `sk-ant-`)

### Step 2: Set the Environment Variable

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key-here"
```

Add this to your `~/.bashrc`, `~/.zshrc`, or `~/.profile` to persist across sessions.

### Step 3: Launch with Claude

```bash
vibecli --provider claude
```

Expected output:

```
VibeCLI v0.3.3 — AI coding assistant
Provider: claude (claude-sonnet-4-6)

vibecli>
```

### Step 4: Verify

```
vibecli> Explain the borrow checker in one paragraph
```

You should see a response from Claude.

### Using a Different Claude Model

```bash
vibecli --provider claude --model claude-sonnet-4-6
```

### Enable Extended Thinking

For complex tasks, Claude supports extended thinking mode. Add to `~/.vibecli/config.toml`:

```toml
[claude]
enabled = true
thinking_budget_tokens = 10000
```


## Option 3: OpenAI (Cloud, API Key)

### Step 1: Get an API Key

1. Go to [platform.openai.com](https://platform.openai.com/)
2. Navigate to **API Keys** and create a new key
3. Copy the key (starts with `sk-`)

### Step 2: Set the Environment Variable

```bash
export OPENAI_API_KEY="sk-your-key-here"
```

### Step 3: Launch with OpenAI

```bash
vibecli --provider openai
```

Expected output:

```
VibeCLI v0.3.3 — AI coding assistant
Provider: openai (gpt-4o)

vibecli>
```

### Step 4: Verify

```
vibecli> What model are you?
```


## Switch Between Providers

You do not have to commit to one provider. Switch at any time:

```bash
# Use Claude for a coding task
vibecli --provider claude --exec "refactor auth.rs to use async"

# Use Ollama for private exploration
vibecli --provider ollama

# Use OpenAI for a quick question
vibecli --provider openai
```

Inside the REPL, you can also compare providers head-to-head:

```
vibecli> /arena compare claude openai "Write a binary search in Rust"
```

This shows both responses side by side with hidden identities so you can vote on quality without bias.


## Store Provider Config Permanently

Instead of passing flags every time, edit `~/.vibecli/config.toml`:

```toml
[claude]
enabled = true
# api_key is read from ANTHROPIC_API_KEY env var

[openai]
enabled = true
# api_key is read from OPENAI_API_KEY env var

[ollama]
enabled = true
api_url = "http://localhost:11434"
model = "qwen3-coder:480b-cloud"
```

Then just use the `--provider` flag to switch:

```bash
vibecli --provider claude
```

See the full [Configuration Guide](../../configuration/) for all provider options.


## Troubleshooting

### "Connection refused" (Ollama)

Ollama server is not running. Start it:

```bash
ollama serve
```

### "401 Unauthorized" (Cloud providers)

Your API key is missing or invalid. Check:

```bash
# Is the variable set?
echo $ANTHROPIC_API_KEY

# Is it the right key? (should start with sk-ant- for Claude)
```

### "Model not found" (Ollama)

You need to pull the model first:

```bash
ollama list                          # See what you have
ollama pull qwen3-coder:480b-cloud   # Pull the default
```

### "Rate limited" (Cloud providers)

You have hit the provider's rate limit. Wait a minute and retry, or switch to a different provider. For sustained use, check your plan's rate limits on the provider's dashboard.

### Slow responses with Ollama

Local inference speed depends on your hardware. Options:

- Use a smaller model: `ollama pull codellama:7b`
- Ensure you have enough RAM (13B models need ~8GB, 70B needs ~40GB)
- On macOS with Apple Silicon, Ollama uses the GPU automatically


## Next Steps

- [Using the Agent to Fix Bugs](/vibecody/tutorials/agent-workflow/) -- put your provider to work
- [AI-Powered Code Review](/vibecody/tutorials/code-review/) -- review code with AI
- [Tutorials Index](./) -- browse all tutorials
