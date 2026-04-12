# Cost Estimator

Pre-execution token cost estimation — estimates input + output tokens and provider cost in USD before running an agent task. Matches Devin 2.0's pre-execution cost estimation feature.

## When to Use
- Showing users the estimated cost before starting a long agent task
- Comparing costs across providers/models for a given task
- Detecting when a task is likely to be expensive before committing
- Building cost awareness into CI/CD agent pipelines

## Pricing Catalogue (April 2026)
| Provider | Model | Input/1K | Output/1K |
|---|---|---|---|
| anthropic | claude-opus-4-6 | $0.015 | $0.075 |
| anthropic | claude-sonnet-4-6 | $0.003 | $0.015 |
| anthropic | claude-haiku-4-5 | $0.00025 | $0.00125 |
| openai | gpt-4o | $0.005 | $0.015 |
| google | gemini-2.0-flash | $0.0001 | $0.0004 |
| ollama | llama3 | $0 | $0 |

## Confidence Levels
- **High** — no tools expected; simple Q&A
- **Medium** — 1-3 tool rounds expected
- **Low** — 4+ tool rounds; large output variance

## Commands
- `/cost estimate` — estimate cost for current task + provider
- `/cost breakdown` — show per-component token counts
- `/cost compare` — compare cost across all configured providers

## Examples
```
/cost estimate
# anthropic/claude-sonnet-4-6: ~2,400 input + ~800 output ≈ $0.019 (High confidence)

/cost compare
# anthropic/claude-opus-4-6:   $0.096
# openai/gpt-4o:                $0.024
# ollama/llama3:                $0.000 (local)
```
