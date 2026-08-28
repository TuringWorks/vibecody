---
layout: page
title: "Third-Party Services"
permalink: /services/
---

Everything VibeCody can talk to that is not VibeCody, what each one needs from
you, and — the part that usually goes unsaid — **whether you need it at all**.

**The honest summary: you need one AI provider and nothing else.** Every other
service on this page is optional, and a VibeCody with none of them configured is
a complete coding assistant. Nothing here is required to install, build, run the
daemon, edit code, run the agent loop, or use the 1,144 skills.

## Where credentials live

Every key on this page goes into an **encrypted store**, never a plaintext file:

| Scope | Store | Path |
|---|---|---|
| Yours, all projects | `ProfileStore` | `~/.vibecli/profile_settings.db` |
| One project's secrets | `WorkspaceStore` | `<workspace>/.vibecli/workspace.db` |

Set them in **Settings → API Keys / Integrations** in any desktop shell, or with
`vibecli set-key <provider> <value>`. They are never written to `config.toml`,
and an environment variable is a fallback, not the intended path. See
[Configuration]({{ site.baseurl }}/configuration/).

---

## AI providers — pick at least one

**25 providers are selectable.** All of them stream. The two local ones need no
key and no account.

| Category | Providers |
|---|---|
| **Local, no key** | `ollama`, `lmstudio`, `vllm`, `vibecli-mistralrs` (in-process) |
| **Frontier** | `claude`, `openai`, `gemini`, `grok`, `mistral`, `deepseek`, `zhipu`, `minimax` |
| **Fast inference** | `groq`, `cerebras`, `together`, `fireworks`, `sambanova`, `perplexity` |
| **Cloud platform** | `azure_openai`, `bedrock`, `copilot`, `vercel_ai`, `openrouter` |
| **Specialist** | `poolside`, `claude-code` (local CLI passthrough) |

The canonical list is `vibecoder/src/hooks/useModelRegistry.ts`. Per-provider
setup, models and pricing notes: [Providers]({{ site.baseurl }}/providers/).

> **Fully offline is a supported configuration.** `ollama` plus a downloaded
> model means no account, no key, and nothing leaving the machine. See
> [Sizing]({{ site.baseurl }}/sizing/) for what your hardware can run.

---

## Voice

| Service | Needed for | Key |
|---|---|---|
| **whisper.cpp** (local) | Speech-to-text, on-device | None — a model download |
| **Groq Whisper** | Speech-to-text fallback | Groq API key |
| **Platform TTS** | Spoken replies | None. Built into macOS / Windows / Linux |
| **Kokoro-82M** via MLX | Neural spoken replies | None — `make voice-kokoro`. Apple Silicon |
| **ElevenLabs** | Alternative TTS | `elevenlabs_api_key` + `elevenlabs_voice_id` |

Local whisper is tried first when a model is downloaded; the cloud path is the
fallback, not the default. [Voice]({{ site.baseurl }}/voice-duplex/) ·
[Sizing]({{ site.baseurl }}/sizing/#voice-speech-in-and-out)

---

## Productivity integrations

Configured under **Settings → Integrations**. Each is independent; none is
required.

| Category | Services | What you supply |
|---|---|---|
| **Email** | Gmail, Outlook | OAuth client ID + secret, or access/refresh tokens |
| **Calendar** | Google Calendar, Outlook Calendar | Access token |
| **Project tools** | Linear, Notion, Todoist, Jira | API key; Jira also needs instance URL + email |
| **Search & web** | Tavily, Brave Search | API key |
| **Smart home** | Home Assistant | Instance URL + long-lived access token |
| **Infrastructure** | GitHub, OpenSandbox, any container registry | Token; registry needs URL + user + password |

These are also exposed as MCP tools, so a model can act on them directly.

---

## Messaging gateways

`vibecli --gateway <platform>` runs the agent as a 24/7 bot. **25 adapters** ship
(`vibecli/vibecli-cli/src/gateway.rs`):

Telegram · Discord · Slack · Signal · Matrix · Twilio SMS · WhatsApp · iMessage
(macOS) · Microsoft Teams · Google Chat · Mattermost · IRC · LINE · Twitch ·
Nextcloud Talk · Nostr · Feishu · DingTalk · QQ · WeCom · Zalo · BlueBubbles ·
Synology Chat · Tlon · Web chat

Each takes that platform's own bot token. A message becomes an agent task and the
result comes back as a reply.

---

## Sign-in providers

For signing in to VibeCody itself, not for AI: **Google, GitHub, GitLab,
Bitbucket, Microsoft, Apple** (Settings → OAuth Login). Optional — VibeCody does
not require an account.

---

## Connectivity

How phones, watches and remote clients find a daemon. Clients **race all
reachable paths** rather than being configured with one.

| Transport | Needs | Use |
|---|---|---|
| **mDNS** | Nothing | Same LAN. The zero-config path |
| **Tailscale** | A tailnet | Off-LAN, private, no port forwarding |
| **ngrok** | An ngrok account | A public URL, quickly |
| **Phone relay** | Nothing | The phone relays for the watch |

Details and the ordering: [Connectivity]({{ site.baseurl }}/connectivity/).

---

## Sandboxing

Optional isolation for agent-run commands: **Docker**, **Podman**, and
**OpenSandbox** (hosted, needs an API key), behind one runtime trait.

---

## Model and asset sources

| Source | Pulled for | Credential |
|---|---|---|
| **Ollama registry** | Local LLMs and embedding models | None |
| **Hugging Face** | `mistralrs` models, whisper weights | None, except **gated** repos |

Gated models such as `meta-llama/*` need a Hugging Face token —
`vibecli set-key huggingface hf_...`, or `HF_TOKEN`. Without one the daemon says
so at startup and falls back to an ungated default rather than failing silently.

---

## What none of this is required for

Installing · building · running the daemon · the desktop shells · editing ·
git · the agent loop with a local model · skills · code review · the code index
with a local embedding model · voice with the platform engine.

The only hard requirement is **something that can answer a prompt** — one cloud
key, or Ollama on the machine.
