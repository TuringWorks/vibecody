# Agent Modes (Smart / Rush / Deep)

Three-mode agent routing system that selects the optimal model and configuration based on task complexity.

## Triggers
- "agent mode", "smart mode", "rush mode", "deep mode"
- "fast mode", "thinking mode", "auto route", "model selection"

## Usage
```
/mode smart                         # Use balanced mode (default)
/mode rush                          # Use fast mode for simple tasks
/mode deep                          # Use deep thinking for complex tasks
/mode auto                          # Auto-select based on task complexity
/mode stats                         # Show usage stats per mode
/mode profile create "my-profile"   # Create custom mode profile
```

## Modes
- **Smart** — Best available model (Claude Opus), balanced speed and quality, default for most tasks
- **Rush** — Fastest model (Haiku), optimized for speed, ideal for simple edits, renames, formatting
- **Deep** — Most capable model + extended thinking budget, for complex multi-file refactors, architecture, debugging

## Features
- Automatic complexity estimation from token count, file count, and keyword analysis
- Keyword-based routing: "refactor"/"architect" → Deep, "fix typo"/"rename" → Rush
- Manual mode override
- Per-mode usage statistics (invocation count, average tokens)
- Custom mode profiles with user-defined configs
- TaskComplexity enum: Simple, Moderate, Complex
