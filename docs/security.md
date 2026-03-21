---
layout: page
title: Security
permalink: /security/
---

# Security

This document describes VibeCody's security model, data privacy practices, and hardening recommendations for production deployments.

---

## Security Model Overview

VibeCody follows a **defense-in-depth** approach with multiple independent layers of protection:

1. **Approval policies** gate what the agent can do.
2. **Sandbox isolation** restricts where agent commands execute.
3. **Input validation** prevents path traversal and SSRF attacks.
4. **Rate limiting** protects against abuse.
5. **Audit trails** record every action for review.
6. **Admin policies** enforce organizational restrictions.

No single layer is relied upon in isolation. A failure in one layer is contained by the others.

---

## Data Privacy

VibeCody itself does not collect telemetry, analytics, or usage data. What leaves your machine depends entirely on your provider configuration:

| Configuration | Data Sent Externally |
|---|---|
| **Ollama (local)** | None. All inference runs on your hardware. |
| **Cloud provider** (Claude, OpenAI, Gemini, etc.) | Prompt text and code context are sent to the provider's API. |
| **OpenRouter** | Prompt text is sent to OpenRouter, which routes to the selected model provider. |
| **Air-gapped Docker** | None. The container has no network access. |

When using cloud providers, review their data retention policies. Most major providers (Anthropic, OpenAI, Google) offer API usage with no training on your data, but terms vary by provider and plan.

**Recommendations:**
- For sensitive codebases, use Ollama or an air-gapped deployment.
- Use environment variables or `api_key_helper` for API keys rather than hardcoding them in config files.
- Avoid committing `.vibecli/config.toml` to version control if it contains API keys.

---

## Approval Policies

Approval policies are the primary mechanism for controlling agent autonomy:

### suggest (default)

The agent proposes every action and waits for explicit user approval before proceeding. This is the safest mode and is recommended when working with unfamiliar codebases or untrusted prompts.

### auto-edit

The agent can read and write files automatically but must request approval before executing shell commands. This balances productivity with safety for routine coding tasks.

### full-auto

The agent can perform all actions — file edits, command execution, tool invocations — without approval. **Use only with sandbox enabled.** This mode is designed for CI pipelines and batch processing where human interaction is not available.

```toml
[agent]
approval_policy = "suggest"  # "suggest", "auto-edit", or "full-auto"
```

---

## Sandbox Isolation

The sandbox executes agent commands inside a container, preventing access to the host system.

### Container Runtimes

VibeCody supports three container runtimes through the unified `ContainerRuntime` trait:

- **Docker** — Most common. Requires Docker Engine or Docker Desktop.
- **Podman** — Rootless alternative to Docker. No daemon required.
- **OpenSandbox** — Lightweight isolation for environments without Docker.

### Configuration

```toml
[sandbox]
enabled = true
runtime = "docker"           # "docker", "podman", or "opensandbox"
image = "ubuntu:22.04"       # Base image for the sandbox
allow_network = false        # Block all outbound network access
mount_workspace = true       # Mount the current project directory (read-write)
allowed_tools = ["read", "write", "bash", "search"]
timeout_secs = 300           # Kill the container after this duration
```

### Network Controls

When `allow_network = false`, the container is started with `--network=none`, preventing all inbound and outbound connections. This is critical for air-gapped deployments and prevents the agent from exfiltrating data or downloading malicious payloads.

---

## API Key Management

### Storage Options

API keys can be provided through multiple mechanisms, listed from most secure to least:

1. **api_key_helper** — A command that returns the key on stdout. Integrates with system keychains, Vault, or AWS Secrets Manager:

```toml
[provider]
api_key_helper = "security find-generic-password -s vibecody-anthropic -w"
```

2. **Environment variables** — Set in your shell profile or CI environment:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

3. **Config file** — Stored in `~/.vibecli/config.toml`. Ensure the file has restrictive permissions:

```bash
chmod 600 ~/.vibecli/config.toml
```

### Key Rotation

When rotating API keys:

1. Generate a new key from your provider's dashboard.
2. Update the environment variable or key helper.
3. Verify with `vibecli doctor` to confirm the new key works.
4. Revoke the old key from the provider's dashboard.

---

## SSRF Prevention

The tool executor validates all URLs before making requests. Only the following URL schemes are permitted:

- `http://`
- `https://`

Schemes such as `file://`, `ftp://`, `gopher://`, and others are blocked. Additionally, requests to private IP ranges (127.0.0.0/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16) are rejected by default when operating in sandbox mode.

---

## Path Traversal Prevention

All file operations performed by the agent go through a safe-name validation layer that:

- Rejects paths containing `..` components.
- Resolves symlinks and verifies the resolved path is within the allowed workspace.
- Blocks access to sensitive system paths (`/etc/passwd`, `~/.ssh/`, etc.).
- Normalizes path separators to prevent bypass via mixed separators on Windows.

---

## Rate Limiting

The HTTP daemon (`vibecli serve`) enforces per-endpoint rate limits:

| Endpoint | Default Limit |
|---|---|
| `/api/chat` | 60 requests/minute |
| `/api/agent` | 20 requests/minute |
| `/api/tools/*` | 120 requests/minute |
| `/health` | Unlimited |

Rate limits are configurable in the serve configuration. Exceeding the limit returns HTTP 429 with a `Retry-After` header.

```toml
[serve]
rate_limit_chat = 60
rate_limit_agent = 20
```

---

## Admin Policy

Organizations can enforce restrictions through a policy file at `.vibecli/policy.toml` in the project root or a global policy at `~/.vibecli/policy.toml`:

```toml
[policy]
# Restrict which providers can be used
allowed_providers = ["claude", "ollama"]

# Block specific shell commands
blocked_commands = ["rm -rf /", "curl", "wget", "nc"]

# Require sandbox for full-auto mode
require_sandbox_for_auto = true

# Disable specific tools
disabled_tools = ["bash"]

# Maximum context tokens per session
max_context_tokens = 200000
```

Policy files are read at startup and cannot be overridden by user configuration.

---

## Audit Trail

Every agent action is recorded in JSONL trace files:

```
~/.vibecli/traces/
  session-abc123.jsonl          # Tool calls, model requests, timing
  session-abc123-messages.json  # Full message history
  session-abc123-context.json   # Context window snapshots
```

Each trace entry includes:
- Timestamp (ISO 8601)
- Action type (tool_call, model_request, model_response, user_input)
- Input and output data
- Duration in milliseconds
- Token usage (prompt and completion)

Traces can be reviewed in the VibeUI Traces panel or exported for external analysis. The compliance controls module supports configurable retention policies and automatic PII redaction.

---

## Air-Gapped Mode

For environments with no internet access:

### Setup

1. Pre-pull the required Docker images and Ollama models on a connected machine.
2. Transfer them to the air-gapped host via `docker save` / `docker load` and `ollama cp`.
3. Use `docker-compose.yml` to run VibeCLI with the Ollama sidecar:

```bash
docker-compose up
```

### Verification

Confirm no network egress:

```bash
# Inside the container
curl https://example.com  # Should fail: network is disabled
```

The Docker Compose configuration sets `network_mode: none` for the VibeCLI container and creates an internal-only network between VibeCLI and Ollama.

---

## Reporting Vulnerabilities

If you discover a security vulnerability in VibeCody, please report it responsibly:

1. **Do not** open a public GitHub issue for security vulnerabilities.
2. Email **security@vibecody.dev** with:
   - A description of the vulnerability.
   - Steps to reproduce.
   - Potential impact assessment.
3. You will receive an acknowledgment within 48 hours.
4. We aim to release a fix within 7 days for critical issues.

We appreciate responsible disclosure and will credit reporters (with permission) in the release notes.

---

## Security Hardening Checklist

Use this checklist when deploying VibeCody in production or sensitive environments:

- [ ] Set approval policy to `suggest` or `auto-edit` (not `full-auto` without sandbox).
- [ ] Enable sandbox with `allow_network = false`.
- [ ] Use `api_key_helper` instead of plaintext API keys in config files.
- [ ] Set `chmod 600` on `~/.vibecli/config.toml`.
- [ ] Configure an admin policy file with `allowed_providers` and `blocked_commands`.
- [ ] Review trace logs regularly for unexpected agent behavior.
- [ ] Set `max_context_tokens` to limit memory usage per session.
- [ ] Enable rate limiting on the HTTP daemon if exposed to a network.
- [ ] Keep VibeCody updated to receive security patches.
- [ ] For air-gapped deployments, verify network isolation with connectivity tests.
- [ ] Enable PII redaction in compliance controls if handling sensitive data.
- [ ] Configure data retention policies to automatically purge old traces.
- [ ] Run `vibecli doctor` periodically to verify configuration integrity.
