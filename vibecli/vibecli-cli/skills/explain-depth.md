# explain-depth

Code explanation at configurable depth levels for different audiences.

## Usage

```
/explain <file_or_selection> [--depth surface|overview|deep|expert] [--audience novice|developer|senior|architect]
/explain --focus "error handling" src/auth.rs
```

## Depth Levels

| Level | Description | Audience |
|-------|-------------|----------|
| `surface` | One-sentence summary | Novice |
| `overview` | Purpose + key components (2-4 sentences) | Developer |
| `deep` | Step-by-step, data flow, edge cases | Senior Engineer |
| `expert` | Complexity, memory, concurrency, security, architecture | Architect |

## Features

- Generates tailored system + user prompts for LLM consumption
- Audience-specific language style (plain English → expert technical)
- Optional focus: `--focus "concurrency"` narrows the explanation
- Follow-up question suggestions per depth level
- Complexity hints (O(n) estimates) at deep/expert levels
- `/explain deeper` upgrades current explanation by one level

## Example

```
> /explain src/session.rs --depth overview
This module manages user sessions via SQLite-backed storage. It provides
CRUD operations for session records with automatic expiry cleanup.
Key data flows: create → store → retrieve → cleanup_expired.
```

## Module

`vibecli/vibecli-cli/src/explain_depth.rs`
