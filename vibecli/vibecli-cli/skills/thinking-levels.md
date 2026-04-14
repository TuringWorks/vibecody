---
triggers: ["thinking level", "reasoning budget", "token budget", "extended thinking", "model:level", "thinking tokens", "sonnet:high", "xhigh", "reasoning effort"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# Thinking Levels

Rules for selecting and configuring the 6-level thinking abstraction (off/minimal/low/medium/high/xhigh).

1. **Start at the lowest sufficient level** — thinking tokens cost the same as output tokens. Use `off` for retrieval-only tasks and `minimal` for simple single-line edits. Escalate only when the task actually requires multi-step reasoning.

2. **Level-to-budget mapping** — never exceed the default budget without an explicit override. Default budgets: `off`=0, `minimal`=200, `low`=1 000, `medium`=5 000, `high`=10 000, `xhigh`=32 000 tokens. Budgets above 10k significantly increase cost and latency.

3. **Model shorthand syntax** — pass `--model <name>:<level>` on the CLI, e.g. `--model sonnet:high` or `--model gpt-4o:medium`. Unrecognised level suffixes silently fall back to `off` — always validate the level string before invoking the model.

4. **Auto-select via TaskHint** — when no explicit level is given, call `ThinkingLevel::default_for_task(&hint)` to pick a sensible default: `SimpleEdit`→minimal, `CodeGeneration`→low, `Debugging`→medium, `Architecture`→high, `ComplexReasoning`→xhigh, `Unknown`→low.

5. **Provider-specific behaviour**
   - **Anthropic / Claude**: uses the `interleaved-thinking-2025-05-14` beta; budget is sent as `thinking.budget_tokens` in the request body. Requires model `claude-3-5-sonnet`, `claude-3-7-sonnet`, or `claude-opus-4-x`.
   - **OpenAI o1/o3**: maps Minimal+Low → `reasoning_effort: low`, Medium → `medium`, High+XHigh → `high`. Budget parameter is not used — effort tier controls internal compute.
   - **Gemini**: sends `thinkingConfig.thinkingBudget` with the integer token value. Requires `gemini-2.0-flash-thinking` or `gemini-2.5-*` models.
   - **Other providers**: thinking is disabled automatically — `ThinkingConfig::disabled()` is used and the provider receives no extra params.

6. **Cost implications per level** — approximate extra cost multipliers on top of base output cost: `minimal` ×1.0 (negligible), `low` ×1.1, `medium` ×1.5, `high` ×2×, `xhigh` ×4×. Communicate estimated cost to the user before invoking `xhigh` on long contexts.

7. **Session budget overrides** — use `ThinkingBudgetOverride::set(level, tokens)` to allow power users to fine-tune budgets without changing the level enum. Overrides are per-session and do not persist across restarts. Validate that `tokens > 0` before storing.

8. **When to increase the thinking level**
   - Debugging produces incorrect or incomplete diffs → escalate from `low` to `medium`.
   - Architecture reviews return shallow analysis → escalate from `medium` to `high`.
   - Complex multi-file refactors, security proofs, or formal reasoning → use `xhigh`.
   - Simple renames, whitespace fixes, or doc-string updates → stay at `off` or `minimal`.

9. **Model compatibility checklist** — before sending extended-thinking requests, verify: (a) the model supports extended reasoning, (b) the level is not `off`, (c) the budget does not exceed the model's maximum (32 768 for Claude, provider-dependent for others). Fail gracefully with `ThinkingConfig::disabled()` if any check fails.

10. **Streaming with thinking tokens** — when streaming, thinking-token chunks arrive before the visible answer. Strip them before rendering in the TUI using the `reasoning_provider::strip_thinking_from` helper, or pass `strip_thinking: true` to `ReasoningConfig`. Always surface the raw thinking blocks in debug/verbose mode.
