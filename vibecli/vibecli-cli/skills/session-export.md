# session-export

Export and import sessions as portable bundles (JSON, Markdown, CSV).

## Usage

```
/session export [format]     # format: json | markdown | csv (default: markdown)
/session import <file>
```

## Features

- Export to JSON (machine-readable), Markdown (human-readable), or CSV
- Role-aware formatting: user / assistant / system / tool
- Metadata preservation (model, timestamps, tags)
- Import from Markdown or CSV back into session format
- Word count and message count statistics

## Formats

### Markdown
```markdown
# Session Title
**Session**: sess-abc  |  **Model**: claude-sonnet-4-6  |  **Messages**: 4

## USER (1000)
Hello, explain Rust ownership.

## ASSISTANT (2000)
Rust ownership means each value has one owner...
```

### CSV
```csv
id,role,timestamp_ms,content
m1,user,1000,"Hello, explain Rust ownership."
m2,assistant,2000,"Rust ownership means..."
```

## Module

`vibecli/vibecli-cli/src/session_export.rs`
