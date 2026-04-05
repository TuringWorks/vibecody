---
layout: page
title: "Demo 50: Warp-Style Terminal Features"
permalink: /demos/50-warp-features/
---


## Overview

VibeCody brings Warp-style terminal intelligence to the REPL. Type natural language commands prefixed with `#` and the AI translates them into shell commands. Get automatic corrections when a command fails, secret redaction in output, and contextual next-command suggestions. These features work inside the VibeCLI REPL with any configured AI provider.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI 0.5.1 or later installed and on your PATH
- At least one AI provider configured in `~/.vibecli/config.toml`
- A project directory with files to work with

## Step-by-Step Walkthrough

### Step 1: Natural language to shell commands

Launch the REPL and use the `#` prefix to describe what you want in plain English:

```bash
vibecli
```

```
vibecli 0.5.1 | Provider: claude | Model: claude-sonnet-4-6
Type /help for commands, /quit to exit

> # find all files larger than 10MB in the current directory
```

```
Suggested command:
  find . -type f -size +10M -exec ls -lh {} \;

[R]un  [E]dit  [C]ancel
> R
```

```
-rw-r--r--  1 user  staff    14M Mar 15 09:22 ./data/embeddings.bin
-rw-r--r--  1 user  staff    23M Mar 10 14:05 ./models/classifier.onnx
-rw-r--r--  1 user  staff    11M Feb 28 08:41 ./logs/app-2026-02.log

3 files found.
```

### Step 2: Complex natural language queries

```
> # show git commits from last week that touched any rust files
```

```
Suggested command:
  git log --since="1 week ago" --diff-filter=M -- "*.rs" --oneline

[R]un  [E]dit  [C]ancel
> R
```

```
c7569f6 fix: remove duplicate REPL handlers, create missing module stubs
edc0e4d feat: TurboQuant vector DB integration + panel tests
4ebaa49 feat: Phase 32 P0 — code replay, speculative execution
3d7e159 feat: Phase 32 — context protocol, code review agent
f176b05 feat: TurboQuant panel + REPL benchmark command
```

### Step 3: Automatic command correction

When a command fails, VibeCody detects the error and suggests a fix:

```
> # list docker containers sorted by memory usage
```

```
Suggested command:
  docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}" | sort -k2 -h

[R]un  [E]dit  [C]ancel
> R
```

```
Error: Cannot connect to the Docker daemon. Is docker running?

Auto-correction suggested:
  The Docker daemon is not running. Would you like to start it?
  Corrected command: open -a Docker && sleep 5 && docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}" | sort -k2 -h

[R]un corrected  [C]ancel
> R
```

```
NAME                  MEM USAGE
vibecody-ollama       2.14GiB
postgres-dev          148.5MiB
redis-cache           42.8MiB
```

### Step 4: Typo and flag correction

VibeCody also catches common typos and incorrect flags:

```
> # compress the logs directory
```

```
Suggested command:
  tar -czf logs.tar.gz ./logs/

[R]un  [E]dit  [C]ancel
> E
```

```
Edit command: tar -czf logs-backup.tar.gz ./logs/

[R]un  [C]ancel
> R
```

```
Created logs-backup.tar.gz (4.2 MB)
```

### Step 5: Secret redaction in output

VibeCody automatically detects and redacts secrets in command output:

```
> # show environment variables related to API keys
```

```
Suggested command:
  env | grep -i "api\|key\|secret\|token"

[R]un  [E]dit  [C]ancel
> R
```

```
ANTHROPIC_API_KEY=sk-ant-***REDACTED***
OPENAI_API_KEY=sk-***REDACTED***
GITHUB_TOKEN=ghp_***REDACTED***
AWS_SECRET_ACCESS_KEY=***REDACTED***
DATABASE_URL=postgres://user:***REDACTED***@localhost:5432/mydb
```

Redaction applies automatically to patterns matching API keys, tokens, passwords, and connection strings. No configuration needed.

### Step 6: Next-command suggestions

After each command, VibeCody suggests what you might want to do next based on context:

```
> # check disk usage of the project
```

```
Suggested command:
  du -sh . && du -sh */ | sort -rh | head -10

[R]un  [E]dit  [C]ancel
> R
```

```
 892M    .
 340M    target/
 210M    models/
  98M    data/
  82M    node_modules/
  64M    vibeui/
  48M    docs/
  22M    vibecli/
  14M    logs/
   8M    .git/

Next steps (based on output):
  1. # clean the cargo build cache         → cargo clean
  2. # check what's in the models directory → ls -la models/
  3. # remove node_modules and reinstall    → rm -rf node_modules && npm install

Pick [1-3] or type a new command:
```

### Step 7: Chaining natural language commands

You can chain multiple operations:

```
> # find TODO comments in rust files and count them by file
```

```
Suggested command:
  grep -rn "TODO" --include="*.rs" | cut -d: -f1 | sort | uniq -c | sort -rn

[R]un  [E]dit  [C]ancel
> R
```

```
  12 vibecli/vibecli-cli/src/agent.rs
   8 vibecli/vibecli-cli/src/open_memory.rs
   5 vibeui/crates/vibe-ai/src/provider.rs
   4 vibecli/vibecli-cli/src/auto_research.rs
   3 vibeui/crates/vibe-core/src/buffer.rs

Total: 32 TODO comments across 5 files

Next steps:
  1. # show the TODOs in agent.rs           → grep -n "TODO" vibecli/vibecli-cli/src/agent.rs
  2. # create a tracking issue for the TODOs → gh issue create --title "Address 32 TODOs"
```

## How It Works

- **Natural language parsing**: The `#` prefix sends your description to the AI provider, which generates the appropriate shell command for your OS and available tools.
- **Error detection**: VibeCody intercepts non-zero exit codes and stderr output, then asks the AI to diagnose and suggest a corrected command.
- **Secret redaction**: A regex-based scanner runs on all command output before display. Patterns include API keys (sk-, xai-, ghp_, etc.), tokens, passwords in URLs, and common secret environment variable names.
- **Next suggestions**: The AI receives the command, its output, and the current directory context to generate 2-3 relevant follow-up actions.

## Demo Recording

```json
{
  "meta": {
    "title": "Warp-Style Terminal Features",
    "description": "Natural language commands, automatic corrections, secret redaction, and next-command suggestions.",
    "duration_seconds": 150,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "# find all files larger than 10MB in the current directory", "delay_ms": 4000 },
        { "input": "R", "delay_ms": 3000 }
      ],
      "description": "Natural language to shell command"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "# show git commits from last week that touched any rust files", "delay_ms": 4000 },
        { "input": "R", "delay_ms": 3000 }
      ],
      "description": "Complex natural language query"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "# show environment variables related to API keys", "delay_ms": 4000 },
        { "input": "R", "delay_ms": 2000 }
      ],
      "description": "Secret redaction demonstration"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "# check disk usage of the project", "delay_ms": 4000 },
        { "input": "R", "delay_ms": 3000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Next-command suggestions"
    }
  ]
}
```

## What's Next

- [Demo 48: OpenMemory](../48-open-memory/) -- Persistent cognitive memory engine
- [Demo 51: Profiles & Sessions](../51-profiles-sessions/) -- Profile-based configuration and session management
- [Demo 53: Workflow Orchestration](../53-workflow-orchestration/) -- Task tracking with lessons and todo management
