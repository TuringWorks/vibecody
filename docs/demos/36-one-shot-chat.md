---
layout: page
title: "Demo 36: One-Shot Chat Mode"
permalink: /demos/36-one-shot-chat/
---


## Overview

VibeCody supports a one-shot chat mode for quick, non-interactive queries directly from the terminal. Pass your message as a positional argument, and VibeCLI streams the response to stdout then exits immediately. This is ideal for shell scripts, pipelines, and quick lookups where you do not need a persistent REPL session.

**Time to complete:** ~3 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured (API key set via environment variable or `~/.vibecli/config.toml`)
- For provider switching: multiple providers configured

## Step-by-Step Walkthrough

### 1. Basic One-Shot Message

The simplest invocation passes your message as positional arguments after the binary name.

```bash
vibecli "Hello"
```

Example output:

```
Hello! I'm VibeCody, your AI coding assistant. I can help you with:

- Writing and editing code
- Debugging issues
- Explaining concepts
- Running shell commands
- Searching codebases
- And much more

What would you like to work on?
```

The response streams token by token and VibeCLI exits with code 0 when the response is complete.

### 2. Using the `chat` Subcommand

You can also use the explicit `chat` keyword before your message. The behavior is identical to positional arguments.

```bash
vibecli chat "Explain quicksort in 3 sentences"
```

Example output:

```
Quicksort is a divide-and-conquer sorting algorithm that selects a pivot
element and partitions the array into elements less than and greater than
the pivot. It recursively sorts the two partitions, achieving an average
time complexity of O(n log n). While its worst case is O(n^2) with poor
pivot choices, randomized pivot selection makes this unlikely in practice.
```

### 3. Multi-Word Messages

Wrap your entire message in quotes to ensure it is treated as a single prompt.

```bash
vibecli "What is the difference between a mutex and a semaphore?"
```

Example output:

```
A mutex (mutual exclusion) is a locking mechanism that allows only one
thread to access a resource at a time. The thread that locks the mutex
must be the one to unlock it.

A semaphore is a signaling mechanism that maintains a count. It allows
up to N threads to access a resource concurrently (where N is the
semaphore's initial count). Any thread can signal (increment) the
semaphore, not just the one that waited on it.

Key differences:
  - Ownership: mutexes have owner threads; semaphores do not
  - Count: mutexes are binary; semaphores can be counting
  - Use case: mutexes protect critical sections; semaphores control
    access to a pool of resources
```

### 4. Switching Providers with `--provider`

Override the default provider for a single query using the `--provider` flag.

```bash
vibecli --provider claude "Write a haiku about Rust programming"
```

Example output:

```
Ownership is clear,
borrowing without a cost—
safe threads never fear.
```

Try the same prompt with a different provider:

```bash
vibecli --provider openai "Write a haiku about Rust programming"
```

Example output:

```
Lifetimes guard the heap,
the compiler catches all—
zero-cost delight.
```

### 5. Selecting a Specific Model

Combine `--provider` and `--model` for precise control.

```bash
vibecli --provider claude --model claude-sonnet-4-20250514 "Explain the borrow checker"
```

Example output:

```
Rust's borrow checker enforces three rules at compile time:

1. Each value has exactly one owner at a time.
2. You can have either one mutable reference OR any number of immutable
   references to a value, but not both simultaneously.
3. References must always be valid (no dangling pointers).

These rules eliminate data races, use-after-free, and double-free bugs
without requiring a garbage collector. The borrow checker analyzes
lifetimes statically, so there is zero runtime overhead.
```

### 6. Piping Output to Other Commands

Because one-shot mode writes to stdout and exits, you can pipe the output into other tools.

```bash
vibecli "Generate a .gitignore for a Rust project" > .gitignore
```

```bash
vibecli "Write a SQL CREATE TABLE for a users table" | pbcopy
```

```bash
vibecli "List 5 common HTTP status codes as CSV" | column -t -s,
```

Example output from the last command:

```
Code  Name                   Description
200   OK                     Request succeeded
301   Moved Permanently      Resource has moved
404   Not Found              Resource does not exist
500   Internal Server Error  Server encountered an error
503   Service Unavailable    Server temporarily overloaded
```

### 7. Using in Shell Scripts

One-shot mode works well in automation scripts.

```bash
#!/bin/bash
# generate-docs.sh — AI-assisted documentation generator

for file in src/*.rs; do
  echo "Documenting $file..."
  vibecli "Write a one-paragraph doc comment for this file: $(head -20 "$file")" \
    >> docs/generated-comments.md
  echo "---" >> docs/generated-comments.md
done

echo "Done. See docs/generated-comments.md"
```

### 8. Exit Codes

One-shot mode uses standard exit codes for scripting:

| Exit Code | Meaning                                    |
|-----------|--------------------------------------------|
| `0`       | Response completed successfully            |
| `1`       | General error (invalid arguments, etc.)    |
| `2`       | Provider error (auth failure, rate limit)  |
| `3`       | Network error (timeout, DNS failure)       |

```bash
vibecli "Hello" && echo "Success" || echo "Failed with code $?"
```

## Streaming Behavior

One-shot chat streams tokens to the terminal as they arrive from the provider. You will see text appear incrementally rather than waiting for the full response. This provides a responsive experience even for long answers.

If you redirect stdout to a file, streaming still occurs but is buffered by the shell. The file will contain the complete response once VibeCLI exits.

## Demo Recording JSON

```json
{
  "meta": {
    "title": "One-Shot Chat Mode",
    "description": "Send a single message to VibeCLI and get a streamed response without entering the REPL.",
    "duration_seconds": 90,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli \"Hello\"",
      "description": "Send a basic one-shot message",
      "expected_output_contains": "Hello",
      "delay_ms": 3000
    },
    {
      "id": 2,
      "action": "shell",
      "command": "vibecli chat \"Explain quicksort in 3 sentences\"",
      "description": "Use the explicit chat keyword",
      "expected_output_contains": "quicksort",
      "delay_ms": 5000
    },
    {
      "id": 3,
      "action": "shell",
      "command": "vibecli --provider claude \"Write a haiku about Rust programming\"",
      "description": "Override the provider for a single query",
      "expected_output_contains": "Rust",
      "delay_ms": 4000
    },
    {
      "id": 4,
      "action": "shell",
      "command": "vibecli --provider openai \"Write a haiku about Rust programming\"",
      "description": "Compare output from a different provider",
      "expected_output_contains": "Rust",
      "delay_ms": 4000
    },
    {
      "id": 5,
      "action": "shell",
      "command": "vibecli --provider claude --model claude-sonnet-4-20250514 \"Explain the borrow checker\"",
      "description": "Select a specific model",
      "expected_output_contains": "borrow",
      "delay_ms": 6000
    },
    {
      "id": 6,
      "action": "shell",
      "command": "vibecli \"Generate a .gitignore for a Rust project\" > /tmp/demo-gitignore",
      "description": "Pipe one-shot output to a file",
      "delay_ms": 4000
    }
  ]
}
```

## What's Next

- [Demo 1: First Run & Setup](../first-run/) -- Full installation and provider configuration
- [Demo 3: Multi-Provider Chat](../multi-provider-chat/) -- Compare responses across 18 providers
- [Demo 4: Agent Loop](../agent-loop/) -- Let the AI autonomously edit files and run commands
