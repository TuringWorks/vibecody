---
layout: page
title: "Provider: Ollama"
permalink: /providers/ollama/
---

# Ollama Provider

Run AI models locally on your machine with [Ollama](https://ollama.ai). No API key required. Completely free. Your code never leaves your machine.

## Why Ollama?

- **Free** -- no API costs, no rate limits, no usage tracking
- **Private** -- all inference happens locally; ideal for proprietary code
- **Offline** -- works without an internet connection (air-gapped deployments)
- **Fast startup** -- models load in seconds on modern hardware

## Installation

**macOS:**

```bash
brew install ollama
```

**Linux:**

```bash
curl -fsSL https://ollama.ai/install.sh | sh
```

**Docker:**

```bash
docker run -d -v ollama:/root/.ollama -p 11434:11434 --name ollama ollama/ollama
```

**Verify installation:**

```bash
ollama --version
```

## Pull a Model

```bash
# Recommended for coding
ollama pull qwen3-coder

# Popular alternatives
ollama pull llama3.2
ollama pull codellama:34b
ollama pull deepseek-coder-v2:16b
```

## Recommended Models for Coding

| Model | Size | Quality | Speed | Best for |
|-------|------|---------|-------|----------|
| `qwen3-coder` | 480B (cloud) | Excellent | Fast | General coding (VibeCody default) |
| `llama3.2:70b` | 40 GB | Very good | Medium | Complex reasoning, refactoring |
| `llama3.2:8b` | 4.7 GB | Good | Very fast | Quick tasks, completions |
| `deepseek-coder-v2:16b` | 9 GB | Very good | Fast | Code generation, debugging |
| `codellama:34b` | 19 GB | Good | Medium | Code-specific tasks |
| `codellama:7b` | 3.8 GB | Fair | Very fast | Low-memory machines |
| `qwen2.5-coder:7b` | 4.4 GB | Good | Very fast | Balanced quality/speed |
| `starcoder2:15b` | 9 GB | Good | Fast | Code completion |

**Minimum RAM:** 8 GB for 7B models, 16 GB for 13-16B models, 64 GB for 70B models.

## Configure VibeCody

**Option 1: Environment variable (override API URL)**

```bash
export OLLAMA_HOST="http://localhost:11434"
vibecli --provider ollama
```

**Option 2: Config file** (`~/.vibecli/config.toml`)

```toml
[ollama]
enabled = true
api_url = "http://localhost:11434"
model = "qwen3-coder:480b-cloud"
```

**Option 3: CLI flag**

```bash
vibecli --provider ollama --model llama3.2:8b
```

## Verify Connection

```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Test with VibeCody
vibecli --provider ollama -c "Say hello"
```

## GPU Acceleration

Ollama automatically uses GPU when available:

- **NVIDIA:** Install CUDA drivers. Ollama detects GPUs automatically
- **Apple Silicon:** Metal acceleration is used by default (no setup needed)
- **AMD:** ROCm support on Linux

Check GPU detection:

```bash
ollama run llama3.2:8b "hello"
# Watch the logs: ollama will print which GPU layers are loaded
```

## Air-Gapped Deployment

For environments without internet access:

1. On a machine with internet, pull the model:

   ```bash
   ollama pull llama3.2:8b
   ```

2. Copy the model directory (`~/.ollama/models/`) to the air-gapped machine.

3. Use the provided Docker Compose for a self-contained deployment:

   ```bash
   docker-compose up -d
   ```

   This starts VibeCLI + Ollama as a sidecar with no external network dependencies.

## Troubleshooting

### Connection refused

```
Error: Connection refused (os error 61)
```

Ollama is not running. Start it:

```bash
ollama serve
# or on macOS, open the Ollama app
```

### Model not found

```
Error: model 'xyz' not found
```

Pull the model first:

```bash
ollama pull xyz
```

List available models:

```bash
ollama list
```

### Out of memory (OOM)

If Ollama crashes or becomes unresponsive:

- Use a smaller model (e.g., `llama3.2:8b` instead of `llama3.2:70b`)
- Close other memory-intensive applications
- Set `OLLAMA_MAX_LOADED_MODELS=1` to limit concurrent models
- On Linux, increase swap space

### Slow generation

- Ensure GPU acceleration is active (check `ollama ps`)
- Use a smaller quantization: `ollama pull llama3.2:8b-q4_0`
- Reduce context window: set `num_ctx` in a Modelfile
