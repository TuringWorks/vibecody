---
layout: page
title: "Demo 10 — Code Transforms"
permalink: /demos/10-code-transforms/
---

# Demo 10 — Code Transforms

## Overview

VibeCody's **Code Transforms** engine performs AST-based (Abstract Syntax Tree) code transformations that go beyond text search-and-replace. Rename symbols across files with full semantic awareness, extract functions and methods, convert between coding patterns (e.g., callbacks to async/await), and apply language-specific transforms. All transforms preserve formatting, comments, and semantics.

---

## Prerequisites

- VibeCody installed (`vibecli --version` returns 0.1+)
- An AI provider configured in `~/.vibecli/config.toml`
- A language server running for your project's language (for symbol resolution)
- For VibeUI: `cd vibeui && npm install && npm run tauri dev`

---

## Step-by-Step Walkthrough

### 1. Rename Refactoring

Rename a symbol (variable, function, type, module) across the entire codebase with semantic awareness. Unlike text replacement, this understands scope and will not rename unrelated identifiers that happen to share the same name.

**VibeUI:**
1. Right-click a symbol in the editor
2. Select **VibeCody > Rename Symbol** (or press `F2`)
3. Type the new name
4. Review the list of affected locations in the preview panel
5. Click **Apply**

**CLI:**

```bash
# Rename a function across the project
vibecli transform rename \
  --symbol "process_request" \
  --new-name "handle_incoming_request" \
  --scope "src/**/*.rs"

# Preview changes without applying
vibecli transform rename \
  --symbol "UserDTO" \
  --new-name "UserResponse" \
  --scope "." \
  --dry-run
```

Example output:

```
Rename: process_request -> handle_incoming_request

  src/handler.rs:15     fn process_request(req: Request)  ->  fn handle_incoming_request(req: Request)
  src/handler.rs:42     process_request(incoming)          ->  handle_incoming_request(incoming)
  src/router.rs:28      .route("/api", process_request)    ->  .route("/api", handle_incoming_request)
  src/tests/api.rs:10   process_request(mock_req)          ->  handle_incoming_request(mock_req)
  src/tests/api.rs:35   assert process_request             ->  assert handle_incoming_request

  5 locations in 3 files. Apply? [y/n]:
```

**REPL:**

```bash
vibecli
/transform rename process_request handle_incoming_request
```

### 2. Extract Function / Method

Select a block of code and extract it into a new function, automatically determining parameters and return types.

**VibeUI:**
1. Select the code block to extract
2. Right-click > **VibeCody > Extract Function** (or `Cmd+Shift+E`)
3. Enter a name for the new function
4. VibeCody determines the parameter list, return type, and lifetimes (for Rust)
5. Review the diff and accept

**CLI:**

```bash
# Extract lines 25-40 of handler.rs into a new function
vibecli transform extract-function \
  --file src/handler.rs \
  --start-line 25 \
  --end-line 40 \
  --name "validate_and_parse_body"
```

Example output:

```
Extracting lines 25-40 from src/handler.rs into `validate_and_parse_body`

Detected parameters:
  - req: &Request (from surrounding scope)
  - max_size: usize (from surrounding scope)

Detected return type: Result<ParsedBody, ValidationError>

Generated function:
  fn validate_and_parse_body(req: &Request, max_size: usize) -> Result<ParsedBody, ValidationError> {
      let body = req.body();
      if body.len() > max_size {
          return Err(ValidationError::TooLarge);
      }
      let parsed: ParsedBody = serde_json::from_slice(body)?;
      validate_fields(&parsed)?;
      Ok(parsed)
  }

Original code replaced with:
  let parsed = validate_and_parse_body(&req, max_size)?;

Apply? [y/n]:
```

### 3. Convert Between Patterns

Transform code between equivalent patterns while preserving behavior.

**CLI:**

```bash
# Convert callback-style code to async/await (JavaScript/TypeScript)
vibecli transform convert \
  --pattern "callbacks-to-async" \
  --file src/api.js

# Convert for loops to iterators (Rust)
vibecli transform convert \
  --pattern "loops-to-iterators" \
  --file src/processing.rs

# Convert class components to functional components (React)
vibecli transform convert \
  --pattern "class-to-functional" \
  --file src/components/UserProfile.tsx

# Convert var to const/let (JavaScript)
vibecli transform convert \
  --pattern "var-to-const-let" \
  --scope "src/**/*.js"
```

Example — callbacks to async/await:

```
Before (src/api.js):
  function fetchUser(id, callback) {
    db.query('SELECT * FROM users WHERE id = ?', [id], (err, rows) => {
      if (err) return callback(err);
      callback(null, rows[0]);
    });
  }

After:
  async function fetchUser(id) {
    const rows = await db.query('SELECT * FROM users WHERE id = ?', [id]);
    return rows[0];
  }
```

**REPL:**

```bash
/transform convert callbacks-to-async src/api.js
```

### 4. Language-Specific Transforms

VibeCody provides transforms tailored to specific languages:

**Rust:**

```bash
# Convert unwrap() calls to proper error handling
vibecli transform convert --pattern "unwrap-to-result" --scope "src/**/*.rs"

# Add missing derive macros
vibecli transform convert --pattern "add-derives" --scope "src/**/*.rs" --derives "Debug,Clone,Serialize"

# Convert string concatenation to format! macro
vibecli transform convert --pattern "concat-to-format" --file src/display.rs
```

**TypeScript:**

```bash
# Convert any types to proper types using inference
vibecli transform convert --pattern "any-to-typed" --scope "src/**/*.ts"

# Convert require() to import
vibecli transform convert --pattern "require-to-import" --scope "src/**/*.ts"
```

**Python:**

```bash
# Convert string formatting to f-strings
vibecli transform convert --pattern "format-to-fstring" --scope "**/*.py"

# Add type annotations to function signatures
vibecli transform convert --pattern "add-type-hints" --scope "src/**/*.py"
```

### 5. AI-Assisted Custom Transforms

For transforms not covered by built-in patterns, describe the transformation in natural language:

```bash
# Describe a custom transform
vibecli transform custom \
  --description "Convert all println! debug statements to use the tracing crate's info! macro" \
  --scope "src/**/*.rs" \
  --dry-run
```

**REPL:**

```bash
/transform custom "Replace all raw SQL strings with sqlx::query! macro calls" --scope src/**/*.rs
```

### 6. Transform Panel in VibeUI

The Transform panel provides a visual interface for all transforms:

**VibeUI:**
1. Open the **Transform** panel (AI Panel > Transform tab)
2. Browse available transforms organized by language
3. Select a transform and configure its parameters
4. Click **Preview** to see all affected locations
5. Review the diff for each change
6. Click **Apply Selected** or **Apply All**

The panel shows:
- A searchable list of built-in transforms
- A custom transform input field for AI-assisted transforms
- A file tree showing affected files with change counts
- A split diff view for before/after comparison

---

## Demo Recording

```json
{
  "id": "demo-code-transforms",
  "title": "Code Transforms",
  "description": "Demonstrates AST-based rename refactoring, function extraction, pattern conversion, and the Transform panel in VibeUI",
  "estimated_duration_s": 160,
  "steps": [
    {
      "action": "Navigate",
      "target": "vibeui://open?folder=/home/user/my-project"
    },
    {
      "action": "Narrate",
      "value": "Let's explore VibeCody's code transform capabilities. We'll start with a semantic rename refactoring."
    },
    {
      "action": "Click",
      "target": ".explorer-file[data-path='src/handler.rs']",
      "description": "Open handler.rs"
    },
    {
      "action": "Wait",
      "duration_ms": 1000
    },
    {
      "action": "Click",
      "target": ".editor-line:nth-child(15) .token-function",
      "description": "Click on the function name process_request"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "F2",
      "description": "Trigger rename refactoring"
    },
    {
      "action": "WaitForSelector",
      "target": ".rename-input",
      "timeout_ms": 1000
    },
    {
      "action": "Screenshot",
      "label": "rename-input-active"
    },
    {
      "action": "Type",
      "target": ".rename-input",
      "value": "handle_incoming_request"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Enter"
    },
    {
      "action": "Wait",
      "duration_ms": 2000
    },
    {
      "action": "Screenshot",
      "label": "rename-preview"
    },
    {
      "action": "Assert",
      "target": ".rename-preview .affected-files",
      "value": "count_greater_than:1"
    },
    {
      "action": "Narrate",
      "value": "VibeCody found all 5 references across 3 files. Unlike text replacement, it correctly skips unrelated identifiers. Let's apply."
    },
    {
      "action": "Click",
      "target": "#apply-rename-btn",
      "description": "Apply the rename"
    },
    {
      "action": "Screenshot",
      "label": "rename-applied"
    },
    {
      "action": "Narrate",
      "value": "Now let's extract a function. We select a block of validation logic and extract it into its own function."
    },
    {
      "action": "Click",
      "target": ".editor-line:nth-child(25)",
      "description": "Click at line 25"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Cmd+Shift+Down Cmd+Shift+Down Cmd+Shift+Down",
      "description": "Select lines 25-40"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Cmd+Shift+E",
      "description": "Trigger extract function"
    },
    {
      "action": "WaitForSelector",
      "target": ".extract-function-dialog",
      "timeout_ms": 2000
    },
    {
      "action": "Type",
      "target": ".extract-function-name-input",
      "value": "validate_and_parse_body"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Enter"
    },
    {
      "action": "Wait",
      "duration_ms": 2000
    },
    {
      "action": "Screenshot",
      "label": "extract-function-preview"
    },
    {
      "action": "Assert",
      "target": ".extract-preview .new-function",
      "value": "contains:validate_and_parse_body"
    },
    {
      "action": "Click",
      "target": "#apply-extract-btn",
      "description": "Apply the extraction"
    },
    {
      "action": "Narrate",
      "value": "Function extracted with correct parameters and return type. Finally, let's use the Transform panel for a pattern conversion."
    },
    {
      "action": "Click",
      "target": ".panel-tab[data-panel='transform']",
      "description": "Open the Transform panel"
    },
    {
      "action": "Screenshot",
      "label": "transform-panel"
    },
    {
      "action": "Click",
      "target": ".transform-list-item[data-pattern='unwrap-to-result']",
      "description": "Select the unwrap-to-result transform"
    },
    {
      "action": "Click",
      "target": "#preview-transform-btn",
      "description": "Preview the transform"
    },
    {
      "action": "Wait",
      "duration_ms": 3000
    },
    {
      "action": "Screenshot",
      "label": "transform-preview-diff"
    },
    {
      "action": "Assert",
      "target": ".transform-diff .removed-line",
      "value": "contains:unwrap()"
    },
    {
      "action": "Assert",
      "target": ".transform-diff .added-line",
      "value": "contains:?"
    },
    {
      "action": "Click",
      "target": "#apply-all-transforms-btn",
      "description": "Apply all transforms"
    },
    {
      "action": "Screenshot",
      "label": "transforms-applied"
    },
    {
      "action": "Narrate",
      "value": "All .unwrap() calls have been converted to proper error handling with the ? operator. The AST-based transform preserved formatting and added the necessary Result return types."
    }
  ],
  "tags": ["transforms", "refactoring", "rename", "extract-function", "ast", "code-quality"]
}
```

---

## What's Next

- [Demo 11 — Docker & Container Management](../11-docker/) — Containerize your refactored project
- [Demo 09 — Autofix & Diagnostics](../09-autofix/) — Fix issues introduced during transforms
- [Demo 07 — Inline Chat & Completions](../07-inline-chat/) — Use inline chat for ad-hoc transforms
