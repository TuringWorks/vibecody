# Prompt Cache Advisor

Analyzes prompt structure and recommends optimal `cache_control` breakpoints to minimize cost. Matches Claude Code 1.x's prompt caching guidance.

## Cache Types
| Type | TTL | When to Use |
|---|---|---|
| `persistent` | Up to 1 hour | System prompt, tool definitions, static files |
| `ephemeral` | 5 minutes | RAG context, conversation history, session-scoped data |
| `none` | — | Per-turn user messages, dynamic content |

## Efficiency Labels
- **excellent** — ≥ 70% cacheable tokens
- **good** — 40–70%
- **moderate** — 20–40%
- **poor** — < 20%

## Key Types
- **PromptSegment** — typed segment with `ChangeFreq`
- **CacheAdvisor** — `analyze(segments)` → `CacheAdvisorySummary`
- **CacheRecommendation** — per-segment cache type + estimated savings

## Commands
- `/cache analyze` — analyze current session's prompt structure
- `/cache report` — print full advisory report
- `/cache savings` — show estimated cost savings per 1M requests

## Examples
```
/cache analyze
# Total tokens: 32,768 | Cacheable: 24,576 (75%) | Efficiency: excellent
# Savings: $0.0675/1M requests
# Recommendations:
#   system_prompt → persistent
#   tool_definitions → persistent
#   conversation_history → ephemeral
```
