---
layout: page
title: "Model Harness Profiles"
permalink: /harness-profiles/
---

The **harness** is everything a model is given that is not the conversation: whether its tools arrive as machine-readable schemas or as prose, which system prompt is paired with that choice, how many tokens it may spend on a reply, how much reasoning budget it gets, and any instructions that apply to that model alone.

Until v0.5.11 every one of those was a single global. One system prompt, one tool transport, one output cap, for a 3-billion-parameter local model and a frontier model alike. This page describes what replaced it, what VibeCody now ships as defaults, and how to change them.

## The problem this solves

`AIProvider::advertises_native_tools()` used to answer `true` for Ollama and the OpenAI-compatible family and `false` for everything else. In practice that meant eight providers whose APIs offer first-class tool calling — Claude, OpenAI, Gemini, Bedrock, Grok, OpenRouter, Azure OpenAI and Copilot — never received a tool schema. They were handed a 15 KB XML catalogue in the system prompt instead, roughly 4,000 tokens on every turn, and their tool calls were recovered by running a regular expression over the model's prose.

Turning the schemas on was not enough on its own, because several of the reply paths could not have read the answer:

| Provider | What the reply path did |
|---|---|
| Claude | Required a `text` field on every content block. A `tool_use` block has none, so a reply containing one failed to deserialise — an API error for a turn the model completed. |
| Claude, Gemini | Read `content[0]` / `parts[0]` only. A turn that is prose *then* a tool call lost whichever came second. |
| Gemini | `FunctionDeclaration.parameters` was typed as a string, which serialises a JSON Schema as a quoted scalar describing no parameters at all. |
| Bedrock | Filtered reply blocks on `text`, and a `toolUse` block has none. |
| OpenAI-compatible | The non-streaming path deserialised replies into the *request* message type, which has no `tool_calls` field. |

All of those are fixed, and each is covered by a test that fails against the old behaviour.

## What a profile contains

| Field | Meaning | Shipped default | Honoured by |
|---|---|---|---|
| `tool_transport` | `native` puts tool schemas on the wire; `prose` describes them in the system prompt and parses calls out of the reply. | `native` for every provider whose API takes schemas, `prose` otherwise | all |
| `prompt_dialect` | `compact` drops the per-tool XML catalogue; `full` keeps it. | `compact` wherever the transport is `native` | all |
| `context_window_fallback` | Window to assume when the provider's API does not publish one. | **absent** | all |
| `system_prompt_suffix` | Extra instructions for this pair, appended last. | **absent** | all |
| `max_output_tokens` | Cap on the reply. | **absent** | providers with a cap field |
| `temperature` | Sampling temperature. | **absent** | providers with a temperature field |
| `parallel_tool_calls` | Whether the model may emit several tool calls per turn. | **absent** | the OpenAI-shaped family |
| `thinking_budgets` | Per-effort-tier reasoning budget, in **tokens**. | **absent** | Claude, Gemini |
| `prompt_cache` | Ask the API to cache the system prefix. | on for Anthropic, off elsewhere | Claude |

### Not every knob reaches every provider

The last four are provider-specific, and the difference is real rather than
cosmetic: `prompt_cache` is Anthropic's `cache_control`, `parallel_tool_calls`
is an OpenAI-shaped request field, and a thinking budget is a *token count* —
OpenAI's reasoning dial is an effort word, so a number means nothing to it.

The daemon reports which fields apply as `honored_fields` on every
`/harness/profile` response, because it owns the request builders and is the
only thing that knows. The settings panel draws controls only for those.

Offering the rest everywhere would let you turn on a setting that saves, reads
back as changed, and does nothing — the same success-assuming failure this
codebase watches for in return values, arrived at through the UI instead.

### Why so many defaults are absent

Absent means *the provider decides*. It never means zero and never means unlimited.

`vibe_ai::context_window` already refuses to hardcode context windows, because a window is a fact about someone else's product, written from memory, and wrong the moment a vendor ships a revision. The same reasoning applies to output caps and thinking budgets, so VibeCody ships none of them. What the built-in table *does* assert is transport and dialect — facts about VibeCody's own request builders, which it can check.

The knobs are real and settable. The numbers come from whoever measured them.

## Resolution order

Four layers. Each one overrides only the fields it sets; everything else falls through.

1. **Provider family** — keyed on the provider id.
2. **Built-in model rule** — keyed on a model-id prefix, longest match wins. Ships empty.
3. **Your provider-wide override** — `<provider>/*`.
4. **Your override for one model** — `<provider>/<model>`.

So a temperature set provider-wide still applies to a model whose own override changes only the output cap.

Overrides are stored as *patches* — only the fields you changed — in the encrypted `ProfileStore` at `~/.vibecli/profile_settings.db`, never in a `.toml`, a `.json`, or an environment variable. Storing a fully resolved profile would freeze today's defaults into your settings, so an improvement in a later release would never reach you.

## Changing a profile

### In VibeCoder

**Settings → Model Harness**. Pick a provider, then either *All models* or one model. Every row shows whether its value came from VibeCody or from you, and the reset arrow next to a changed row returns it to the shipped default.

Clearing a field removes the override rather than storing an empty one, so the pair goes back to tracking the built-in default.

### Over HTTP

All four routes require the bearer token from `~/.vibecli/daemon.token`. The token is regenerated on every daemon start.

```bash
TOKEN=$(cat ~/.vibecli/daemon.token)

# What will actually be sent to this pair
curl -s -H "Authorization: Bearer $TOKEN" \
  'http://localhost:7878/harness/profile?provider=claude&model=claude-opus-5'

# Everything currently overridden
curl -s -H "Authorization: Bearer $TOKEN" \
  http://localhost:7878/harness/profiles

# Put one model back on the prose tool path
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"tool_transport":"prose","prompt_dialect":"full"}' \
  'http://localhost:7878/harness/profile?provider=ollama&model=qwen3-coder'

# Reset it
curl -s -X DELETE -H "Authorization: Bearer $TOKEN" \
  'http://localhost:7878/harness/profile?provider=ollama&model=qwen3-coder'
```

Omitting `model` addresses the provider-wide entry.

A `GET` returns both halves:

```json
{
  "provider": "claude",
  "model": "claude-opus-5",
  "effective": { "tool_transport": "native", "prompt_dialect": "compact", "prompt_cache": true },
  "builtin":   { "tool_transport": "native", "prompt_dialect": "compact", "prompt_cache": true }
}
```

`effective` is what the harness will use; `builtin` is what VibeCody ships. When they differ, the difference is yours, and `provider_override` / `model_override` say exactly which fields you set.

Overrides are loaded into the resolver when the daemon starts and reinstalled after every write, so a change takes effect on the next request without a restart.

## When to reach for this

**Put a model back on the prose path.** Native tool calling is usually better, but not universally — some models follow a worked XML example more reliably than a schema. Set `tool_transport: prose` and `prompt_dialect: full` together; sending neither schemas nor a catalogue leaves the model nothing to call.

**Raise an output cap.** Claude requests default to 16,384 output tokens, a figure that predates this system and is not a measured limit for any particular model. If your model supports more and you have checked, set it.

**Supply a context window the API will not.** Anthropic and OpenAI do not publish per-model windows, so the agent falls back to its documented default. `context_window_fallback` replaces that default for one pair — and never overrides a window a provider actually reported.

**Give one model an instruction.** `system_prompt_suffix` is appended after everything else the run assembles, so a per-model reminder can correct the general prompt rather than be corrected by it. Only that pair pays the tokens.

## Measuring a change

`evals/suites/models.yaml` asks the narrow question that matters here: can this model be driven through the tool loop at all? Run it before and after a change to the same pair.

```bash
vibecli --eval run --suite models --provider claude --model claude-opus-5
```

Every task is small enough for a 3B model, and each was written from a failure measured against a real model rather than from a guess. Record both numbers. Per `evals/README.md`, `error` and `skipped` stay out of the pass-rate denominator — a rate over zero scored tasks is `n/a`, never `0%`.

If a model does worse with native tools, that is a result, not a defect: set it back to `prose` and the mechanism has done its job.

## A note on `vibecli-mistralrs`

It looks like an OpenAI-shaped provider and is not. It posts Ollama-style NDJSON to the daemon's own `/api/chat`, whose request type carries `model`, `messages`, `stream`, `options` and `backend` — and no `tools`. Unknown fields are dropped silently, so declaring it native would send schemas that never arrive *and* strip the catalogue that was the model's only remaining description of its tools.

It stays on the prose path until the daemon's inference route forwards tool definitions.

## See also

- [API Reference](/vibecody/api-reference/) — the `/harness/*` routes
- [Configuration](/vibecody/configuration/) — where settings live and why
- [Providers](/vibecody/providers/) — per-provider setup
