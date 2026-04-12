# Rate Limit Backoff

Provider-aware exponential backoff with jitter and per-provider circuit-breaker logic. Matches Cody 6.0's rate-limit backoff + Copilot Workspace v2's retry strategy.

## When to Use
- Handling 429/529 rate-limit responses from AI providers
- Preventing cascading failures when a provider is degraded
- Automatically opening the circuit after repeated failures
- Reporting per-provider retry state in monitoring dashboards

## Retry Policies
- **Fixed** — constant delay regardless of attempt
- **Exponential** — `base × multiplier^attempt`, capped at `max`
- **ExponentialJitter** — exponential + jitter in [0.5, 1.0] × delay (default)

## Retryable HTTP Codes
`429`, `500`, `502`, `503`, `504`, `529`

## Circuit Breaker States
| State | Meaning |
|---|---|
| Closed | Normal operation |
| HalfOpen | Probe allowed after cool-down |
| Open | All requests rejected until `open_duration` expires |

## Defaults
- Max attempts: 5
- Base delay: 500ms, multiplier: 2×, max: 60s
- Circuit threshold: 5 consecutive failures
- Circuit open duration: 30s

## Commands
- `/retry policy <provider>` — show retry policy for a provider
- `/retry status` — show all provider circuit states
- `/retry circuit <provider> reset` — manually close the circuit

## Examples
```
/retry status
# anthropic: closed  openai: open (24s remaining)  groq: closed

/retry policy anthropic
# strategy: ExponentialJitter  base: 500ms  max: 60s  max_attempts: 5
```
