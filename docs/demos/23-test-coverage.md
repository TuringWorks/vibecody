---
layout: page
title: "Demo 23: Test Runner & Coverage"
permalink: /demos/test-coverage/
nav_order: 23
parent: Demos
---


## Overview

VibeCody integrates a unified test runner that works across multiple frameworks (Cargo test, Jest, pytest, and more). The AI can generate test cases from your code, run them, track coverage, and suggest missing tests. The VibeUI Test panel provides a visual overlay showing which lines are covered, which are not, and where the AI recommends adding tests.

**Time to complete:** ~12 minutes

## Prerequisites

- VibeCody installed and configured with at least one AI provider
- A project with testable source code
- Test framework installed for your language (e.g., Rust's built-in `cargo test`, Node.js `jest`, Python `pytest`)
- (Optional) Coverage tool installed (`cargo-tarpaulin`, `nyc`/`c8`, `coverage.py`)
- (Optional) VibeUI installed for the visual Test panel

## Step-by-Step Walkthrough

### Step 1: Run your test suite

Open the VibeCLI REPL and run tests.

```bash
vibecli
```

```
/test run
```

VibeCLI auto-detects your project type and runs the appropriate test command.

```
Detected: Rust project (Cargo.toml found)
Running: cargo test --workspace

test result: ok. 2,686 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
Duration: 34.2s
```

For multi-language projects, VibeCLI runs all detected test frameworks:

```
/test run --all
```

```
Detected frameworks:
  1. Rust (cargo test)     -- 2,686 tests
  2. TypeScript (jest)     -- 142 tests
  3. Python (pytest)       -- 38 tests

Running all...
  [rust]       2,686 passed, 0 failed (34.2s)
  [typescript] 142 passed, 0 failed (4.1s)
  [python]     38 passed, 0 failed (2.3s)

Total: 2,866 passed, 0 failed
```

### Step 2: Run tests with filtering

Run a subset of tests matching a pattern.

```
/test run --filter "preferences"
```

```
Running: cargo test preferences
  test api::preferences::test_create_preferences ... ok
  test api::preferences::test_get_preferences ... ok
  test api::preferences::test_update_preferences ... ok
  test api::preferences::test_delete_preferences ... ok

4 passed, 0 failed (1.1s)
```

### Step 3: Watch mode

Re-run tests automatically when files change.

```
/test watch
```

```
Watching for file changes...
  Patterns: src/**/*.rs, tests/**/*.rs

[12:34:01] File changed: src/api/preferences.rs
[12:34:01] Running affected tests...
  4 passed, 0 failed (0.8s)

[12:35:22] File changed: src/models/user.rs
[12:35:22] Running affected tests...
  12 passed, 0 failed (1.2s)
```

Press `q` to stop watching.

### Step 4: Generate coverage report

```
/test coverage
```

```
Running: cargo tarpaulin --workspace --out json
Generating coverage report...

Coverage Summary:
  Overall:       78.4% (12,340 / 15,738 lines)
  vibecli:       82.1%
  vibe-ai:       76.3%
  vibe-core:     81.9%
  vibe-collab:   69.2%
  vibe-lsp:      71.5%
  vibe-extensions: 74.8%

Uncovered hotspots:
  src/agent.rs           lines 234-267 (error recovery branch)
  src/gateway/slack.rs   lines 89-120 (thread handling)
  src/mcp/protocol.rs    lines 312-345 (edge case parsing)

Coverage report saved to: .vibecli/coverage/report.json
```

### Step 5: AI-generated test suggestions

Ask the AI to analyze coverage gaps and suggest tests.

```
/test generate --file src/agent.rs
```

```
Analyzing src/agent.rs coverage gaps...

Found 3 uncovered regions:
  1. Lines 234-267: Error recovery when provider returns malformed JSON
  2. Lines 312-330: Timeout handling for streaming responses
  3. Lines 401-415: Tool execution with missing permissions

Generated 5 test cases:

  #[test]
  fn test_agent_handles_malformed_provider_json() {
      let agent = Agent::new(MockProvider::returning_invalid_json());
      let result = agent.process("test query");
      assert!(result.is_err());
      assert_eq!(result.unwrap_err().kind(), ErrorKind::ProviderError);
  }

  #[test]
  fn test_agent_timeout_on_streaming_response() { ... }

  #[test]
  fn test_agent_tool_execution_missing_permissions() { ... }

  #[test]
  fn test_agent_retries_on_transient_error() { ... }

  #[test]
  fn test_agent_error_message_includes_provider_name() { ... }

Write tests to tests/agent_coverage_test.rs? [Y/n]
```

Type `Y` to accept, and VibeCLI writes the test file and runs it:

```
Created tests/agent_coverage_test.rs (5 tests)
Running new tests...
  5 passed, 0 failed
Coverage for src/agent.rs: 78.4% -> 91.2% (+12.8%)
```

### Step 6: Run a specific test framework

```
/test run --framework jest
```

```
Running: npx jest --verbose
  PASS src/components/App.test.tsx (14 tests)
  PASS src/components/Panel.test.tsx (8 tests)

22 passed, 0 failed (3.2s)
```

### Step 7: Re-run only failed tests

After a test failure, quickly re-run just the failures.

```
/test run --failed
```

```
Re-running 2 previously failed tests:
  test api::auth::test_expired_token ... ok (was: FAILED)
  test api::auth::test_invalid_signature ... ok (was: FAILED)

2 passed, 0 failed
```

### Step 8: Using the Test panel in VibeUI

Open the **Test** panel from the sidebar.

1. **Run Tab** -- Click "Run All" or select individual test files. Results appear in a tree view with pass/fail indicators.
2. **Coverage Tab** -- Visual overlay on the editor. Green gutters mark covered lines, red gutters mark uncovered lines. Click an uncovered region to ask the AI to generate a test for it.
3. **Generate Tab** -- Select a file and click "Generate Tests". The AI analyzes the code and produces test cases you can review and accept.
4. **Watch Tab** -- Toggle watch mode. Changed files and their test results appear in a live stream.

The coverage overlay integrates directly with the Monaco editor. Hovering over an uncovered line shows a tooltip with the reason it is hard to reach and a suggested test approach.

## Multi-Framework Support

| Framework | Language | Detection | Coverage Tool |
|-----------|----------|-----------|---------------|
| `cargo test` | Rust | `Cargo.toml` | `cargo-tarpaulin` |
| `jest` | JavaScript/TypeScript | `jest.config.*` or `package.json` | `nyc` / `c8` |
| `pytest` | Python | `pytest.ini`, `pyproject.toml` | `coverage.py` |
| `go test` | Go | `go.mod` | built-in `-cover` |
| `dotnet test` | C# | `*.csproj` | `coverlet` |
| `mix test` | Elixir | `mix.exs` | `excoveralls` |

## CLI Command Reference

| Command | Description |
|---------|-------------|
| `/test run` | Run all tests (auto-detect framework) |
| `/test run --all` | Run tests across all detected frameworks |
| `/test run --filter <pattern>` | Run tests matching a name pattern |
| `/test run --framework <name>` | Run tests with a specific framework |
| `/test run --failed` | Re-run only previously failed tests |
| `/test watch` | Watch mode -- re-run tests on file changes |
| `/test coverage` | Generate a coverage report |
| `/test generate --file <path>` | AI-generate tests for uncovered code |

## Demo Recording

```json
{
  "demoRecording": {
    "version": "1.0",
    "title": "Test Runner & Coverage Demo",
    "description": "Run tests across frameworks, track coverage, and generate AI-suggested test cases",
    "duration_seconds": 240,
    "steps": [
      {
        "timestamp": 0,
        "action": "repl_command",
        "command": "/test run",
        "output": "Detected: Rust project\nRunning: cargo test --workspace\n\n2,686 passed, 0 failed (34.2s)",
        "narration": "Run the full test suite with auto-detection"
      },
      {
        "timestamp": 25,
        "action": "repl_command",
        "command": "/test run --filter \"preferences\"",
        "output": "4 passed, 0 failed (1.1s)",
        "narration": "Filter tests by name to run a subset"
      },
      {
        "timestamp": 45,
        "action": "repl_command",
        "command": "/test watch",
        "output": "Watching for file changes...\n[12:34:01] src/api/preferences.rs changed\n  4 passed, 0 failed (0.8s)",
        "narration": "Enable watch mode to auto-run tests on file changes"
      },
      {
        "timestamp": 75,
        "action": "repl_command",
        "command": "/test coverage",
        "output": "Coverage Summary:\n  Overall: 78.4%\n  vibecli: 82.1%\n  vibe-ai: 76.3%\n\nUncovered hotspots:\n  src/agent.rs lines 234-267",
        "narration": "Generate a coverage report with hotspot analysis"
      },
      {
        "timestamp": 110,
        "action": "repl_command",
        "command": "/test generate --file src/agent.rs",
        "output": "Analyzing coverage gaps...\nFound 3 uncovered regions\nGenerated 5 test cases\n\nWrite tests to tests/agent_coverage_test.rs? [Y/n]",
        "narration": "AI analyzes coverage gaps and generates test cases"
      },
      {
        "timestamp": 135,
        "action": "user_input",
        "input": "Y",
        "output": "Created tests/agent_coverage_test.rs (5 tests)\n5 passed, 0 failed\nCoverage: 78.4% -> 91.2% (+12.8%)",
        "narration": "Accept generated tests -- coverage jumps by 12.8%"
      },
      {
        "timestamp": 160,
        "action": "repl_command",
        "command": "/test run --all",
        "output": "Rust: 2,691 passed | TypeScript: 142 passed | Python: 38 passed\nTotal: 2,871 passed, 0 failed",
        "narration": "Run tests across all detected frameworks"
      },
      {
        "timestamp": 185,
        "action": "ui_interaction",
        "panel": "Test",
        "tab": "Coverage",
        "action_detail": "view_overlay",
        "details": "Green gutters on covered lines, red on uncovered. Click to generate tests.",
        "narration": "View the coverage overlay in VibeUI's editor"
      },
      {
        "timestamp": 210,
        "action": "ui_interaction",
        "panel": "Test",
        "tab": "Generate",
        "action_detail": "generate_for_file",
        "file": "src/gateway/slack.rs",
        "narration": "Generate tests for another uncovered file from the UI"
      },
      {
        "timestamp": 230,
        "action": "repl_command",
        "command": "/test run --failed",
        "output": "Re-running 0 previously failed tests.\nAll tests passing.",
        "narration": "Confirm no failures remain"
      }
    ]
  }
}
```

## What's Next

- [Demo 24: Red Team Security](../24-red-team/) -- Automated security scanning with OWASP checks
- Combine test generation with agent teams to have a dedicated Tester agent continuously improving coverage
- Use context bundles to pin test configuration files for consistent test behavior
