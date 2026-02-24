# VibeCLI GitHub Action

Run VibeCLI as a CI agent to automatically fix bugs, add features, or review code.

## Usage

### Auto-fix failing tests
```yaml
- uses: ./.github/actions/vibecli
  with:
    task: "Fix the failing tests reported in CI"
    provider: claude
    approval: full-auto
    anthropic-api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

### Code review on PR
```yaml
- uses: ./.github/actions/vibecli
  with:
    task: "Review the changes in this PR for correctness and security"
    provider: claude
    approval: suggest
    output-format: markdown
    output-file: review-report.md
    anthropic-api-key: ${{ secrets.ANTHROPIC_API_KEY }}
```

### Add error handling
```yaml
- uses: ./.github/actions/vibecli
  with:
    task: "Add proper error handling to all public API functions"
    provider: openai
    approval: auto-edit
    output-format: json
    openai-api-key: ${{ secrets.OPENAI_API_KEY }}
```

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `task` | ✅ | — | Task description for the agent |
| `provider` | — | `claude` | AI provider: `ollama`, `claude`, `openai`, `gemini`, `grok` |
| `approval` | — | `auto-edit` | Policy: `suggest`, `auto-edit`, `full-auto` |
| `output-format` | — | `markdown` | `json`, `markdown`, or `verbose` |
| `output-file` | — | stdout | Write report to this file path |
| `anthropic-api-key` | — | — | Required for `provider: claude` |
| `openai-api-key` | — | — | Required for `provider: openai` |

## Outputs

| Output | Description |
|--------|-------------|
| `report` | Path to the report file (if `output-file` was set) |
| `exit-code` | `0` = success, `1` = critical issues found |
