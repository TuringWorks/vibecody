# token-dashboard

Track and visualize token usage and costs across LLM calls.

## Usage

```
/token-dashboard [session-id]
/tokens                          # alias
```

## Features

- Real-time prompt/completion/cached token tracking
- Per-model cost estimation with built-in pricing (Claude, GPT-4o, Gemini)
- Budget limits with soft warn (80%) and hard block (100%)
- Session-scoped and wildcard `*` budgets
- Time-window filtering and history (up to 1000 records)
- Text dashboard render with model breakdown

## Commands

| Command | Description |
|---------|-------------|
| `/tokens stats` | Show overall token stats for the current session |
| `/tokens budget <model> <max>` | Set a token budget for a model |
| `/tokens cost` | Show estimated cost breakdown |
| `/tokens reset` | Clear the current session's usage history |

## Pricing (per 1M tokens)

| Model | Prompt | Completion |
|-------|--------|------------|
| claude-opus-4-6 | $15 | $75 |
| claude-sonnet-4-6 | $3 | $15 |
| claude-haiku-4-5 | $0.25 | $1.25 |
| gpt-4o | $5 | $15 |
| gemini-2.5-pro | $1.25 | $5 |

## Module

`vibecli/vibecli-cli/src/token_dashboard.rs`
