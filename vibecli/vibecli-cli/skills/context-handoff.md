# Context Handoff

Serialize and transfer a live AI conversation — system prompt, messages, and tool definitions — verbatim to a different provider mid-session. Bridges the pi-mono gap: a portable `HandoffContext` that every provider can consume without adaptation.

## When to Hand Off

1. **Cost routing** — Switch from a large/expensive model (Sonnet, GPT-4o) to a cheaper one (Haiku, GPT-4o-mini) once the user's intent is clear and the context is warm. Trigger after the first successful assistant turn or when token spend exceeds a configurable threshold.

2. **Provider fallback** — When the primary provider returns a 5xx error, rate-limit (429), or fails to respond within the configured timeout, hand off to a backup provider transparently. Preserve the full message history so the backup can continue without a cold start.

3. **Capability routing** — Route turns that require native tool calling to a provider that supports it (e.g. Gemini or GPT-4o) while keeping reasoning-heavy turns on Claude. Inspect `HandoffMessage::has_tool_calls()` to detect when the previous assistant turn issued tool calls.

4. **Context-length routing** — When `HandoffContext::token_estimate()` approaches a provider's context window (e.g. 190 k tokens for a 200 k window), switch to a model with a larger window before the session breaks.

5. **User-requested switch** — Honor explicit `/model switch <provider>` commands without resetting the conversation. Wrap the current context with `for_provider()` and re-submit.

## Preserving Tool Definitions

- Always include `ToolDefinition` entries in the `HandoffContext::tools` list even if no tool has been called yet. Different providers use different schemas (OpenAI function-calling JSON vs. Anthropic tool use vs. Gemini function declarations), but `parameters_json` stores the canonical JSON Schema string — the adapter layer translates it per provider.
- Keep `parameters_json` in strict JSON Schema draft-7 format so that every provider adapter can map it without guessing.
- Do not strip tools when trimming the token budget (`trim_to_token_budget` preserves `tools` and `system_prompt` by design — only messages are dropped).

## Token Budget Trimming Strategy

- Call `trim_to_token_budget(max)` with `max` set to ~80 % of the target provider's context window, leaving room for the new turn and the model's response.
- Trimming removes the **oldest** messages first (from index 0), preserving the most recent conversational context. The system prompt and tool definitions are never dropped.
- After trimming, verify `last_user_message()` is still present. If it was dropped (extremely aggressive budget), re-add it before submitting.
- For sessions that must retain the full history, switch to a larger-context model instead of trimming — use capability routing rather than discarding turns.

## History Audit

- Always call `HandoffHistory::record()` at every provider switch, including the `HandoffReason`. This creates an immutable audit trail for cost attribution, incident replay, and debugging.
- Expose `providers_used()` in session metadata so users can see which models contributed to a response.
- Store `message_count_at_handoff` to reconstruct exactly which messages each provider saw if a response needs to be attributed or re-generated.
- Log the `HandoffEvent::timestamp_ms` alongside response latency to diagnose whether a fallback switch introduced user-visible delay.

## Cost Routing Patterns

```
# Pattern 1: warm-start then switch
ctx = HandoffContext::new("claude-sonnet-4-5").with_system(system_prompt)
ctx.push_message(HandoffMessage::user(first_turn))
first_response = call_provider("claude-sonnet-4-5", &ctx)
ctx.push_message(HandoffMessage::assistant(first_response))
history.record("claude-sonnet-4-5", "claude-haiku-3", HandoffReason::CostRouting, ctx.message_count())
ctx = ctx.for_provider("claude-haiku-3")
# all subsequent turns use haiku

# Pattern 2: budget-gated switch
if cost_so_far_usd > 0.10 {
    ctx = ctx.for_provider("gpt-4o-mini")
    history.record(prev, "gpt-4o-mini", HandoffReason::CostRouting, ctx.message_count())
}

# Pattern 3: fallback on error
match call_provider("openai", &ctx) {
    Err(_) => {
        history.record("openai", "groq", HandoffReason::Fallback, ctx.message_count())
        call_provider("groq", &ctx.for_provider("groq"))
    }
    Ok(r) => r,
}
```

## Summary Logging

Use `HandoffContext::summary()` to log a one-line digest at every handoff point:

```
[handoff] claude-sonnet → claude-haiku | 12 messages, 3 tools, ~4200 tokens | reason: CostRouting
```

This single line captures enough state for cost dashboards and support tickets without serializing the full context.

## REPL Commands

- `/handoff status` — Print `ctx.summary()` for the active context
- `/handoff to <provider>` — Trigger a `UserRequested` handoff to the named provider
- `/handoff history` — List all `HandoffEvent`s in the current session
- `/handoff trim <tokens>` — Apply `trim_to_token_budget` and show the new summary
- `/handoff serialize` — Dump the current context as JSON (useful for debugging or portability)
