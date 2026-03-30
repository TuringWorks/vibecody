---
layout: page
title: "Demo 08 — Code Search & Embeddings"
permalink: /demos/08-code-search/
---


## Overview

VibeCody provides **semantic code search** powered by embeddings, going beyond simple text matching to understand the meaning of your queries. Ask "function that validates email addresses" and VibeCody finds the relevant code even if it never contains the word "validate" or "email" in its identifiers.

The system supports file-level and function-level indexing, cross-repository search, and a full RAG (Retrieval-Augmented Generation) pipeline that feeds search results directly into AI conversations for context-aware answers.


## Prerequisites

- VibeCody installed (`vibecli --version` returns 0.5.1+)
- An AI provider with embedding support configured in `~/.vibecli/config.toml`:

```toml
[openai]
enabled = true
api_key = "sk-..."

[embeddings]
model = "text-embedding-3-small"
dimensions = 1536
```

- A project to index (the demo uses a sample Rust project)
- For VibeUI: `cd vibeui && npm install && npm run tauri dev`


## Step-by-Step Walkthrough

### 1. Generate Embeddings for Your Project

Index your codebase to generate vector embeddings for all source files and functions.

**CLI:**

```bash
# Index the current project
vibecli embeddings index .

# Index with specific file patterns
vibecli embeddings index . --include "*.rs,*.ts,*.py" --exclude "target/,node_modules/"

# Check indexing status
vibecli embeddings status
```

Example output:

```
Indexing project: /home/user/my-project
  Files scanned:    342
  Functions found:  1,284
  Chunks created:   2,108
  Embeddings generated: 2,108 (model: text-embedding-3-small)
  Index size:       14.2 MB
  Duration:         8.3s
```

**VibeUI:**
1. Open the **Code Search** panel (Cmd+Shift+F or click the magnifying glass icon)
2. Click **Build Index** in the panel toolbar
3. A progress bar shows indexing status

### 2. Semantic Search in the REPL

Use the `/search` command to query your codebase semantically.

**CLI:**

```bash
vibecli

# In the REPL:
/search function that handles HTTP authentication
```

Example output:

```
Search results for: "function that handles HTTP authentication"

1. src/auth/middleware.rs:authenticate_request (score: 0.94)
   Lines 23-58 | Extracts Bearer token from headers and validates JWT

2. src/auth/jwt.rs:verify_token (score: 0.89)
   Lines 12-45 | Decodes and verifies JWT claims against stored keys

3. src/handlers/login.rs:handle_login (score: 0.85)
   Lines 30-72 | POST /login handler that issues tokens

4. src/middleware/cors.rs:check_origin (score: 0.71)
   Lines 5-20 | CORS origin validation (related to auth headers)

5. tests/auth_test.rs:test_auth_flow (score: 0.68)
   Lines 10-55 | Integration test for the full auth flow
```

### 3. File-Level vs. Function-Level Search

Control the granularity of search results:

```bash
# Search at file level (broader matches)
/search --level file database connection pooling

# Search at function level (precise matches)
/search --level function retry logic with exponential backoff

# Search with a result limit
/search --limit 3 error handling patterns
```

### 4. Cross-Repository Search

Search across multiple indexed repositories simultaneously:

```bash
# Index additional repositories
vibecli embeddings index ~/projects/frontend-app
vibecli embeddings index ~/projects/shared-lib

# Search across all indexed repos
/search --all-repos shared utility for date formatting
```

Example output:

```
Search results across 3 repositories:

1. [shared-lib] src/utils/date.rs:format_iso8601 (score: 0.96)
2. [frontend-app] src/helpers/dateFormat.ts:formatDate (score: 0.91)
3. [my-project] src/api/serializers.rs:serialize_timestamp (score: 0.78)
```

### 5. Vector Similarity Search via API

For programmatic access, use the HTTP API:

```bash
vibecli --serve --port 7878 --provider openai

# Query the vector index
curl -X POST http://localhost:7878/api/v1/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "function that handles HTTP authentication",
    "limit": 5,
    "level": "function",
    "min_score": 0.6,
    "repositories": ["my-project"]
  }'
```

Response:

```json
{
  "results": [
    {
      "file": "src/auth/middleware.rs",
      "symbol": "authenticate_request",
      "line_start": 23,
      "line_end": 58,
      "score": 0.94,
      "snippet": "pub async fn authenticate_request(req: &Request) -> Result<Claims> { ... }",
      "repository": "my-project"
    }
  ],
  "query_embedding_ms": 45,
  "search_ms": 12,
  "total_results": 5
}
```

### 6. RAG Pipeline Integration

Feed search results directly into an AI conversation for context-aware answers:

```bash
# In the REPL — RAG mode automatically retrieves relevant code
/rag How does authentication work in this project?
```

The RAG pipeline:
1. Generates an embedding for your question
2. Retrieves the top-k most relevant code chunks
3. Injects them as context into the AI prompt
4. Returns an answer grounded in your actual code

**CLI direct command:**

```bash
vibecli rag --query "How does authentication work?" --top-k 5
```

Example output:

```
RAG Answer (grounded in 5 code chunks):

Authentication in this project follows a JWT-based flow:

1. The `handle_login` function in `src/handlers/login.rs` accepts
   credentials and calls `issue_token` to create a signed JWT.

2. Incoming requests pass through `authenticate_request` middleware
   in `src/auth/middleware.rs`, which extracts the Bearer token
   from the Authorization header.

3. Token verification happens in `verify_token` (`src/auth/jwt.rs`),
   which checks expiry, issuer, and audience claims.

Sources: middleware.rs:23-58, jwt.rs:12-45, login.rs:30-72
```

**VibeUI:**
1. Open the **Code Search** panel
2. Toggle **RAG Mode** in the panel toolbar
3. Type your question in the search bar
4. Results appear with AI-synthesized answers and linked source locations

### 7. Managing the Index

```bash
# Rebuild the index (after major refactors)
vibecli embeddings rebuild .

# Show index statistics
vibecli embeddings stats

# Delete the index for a project
vibecli embeddings drop .

# Export embeddings for use in external tools
vibecli embeddings export . --format jsonl --output embeddings.jsonl
```


## Demo Recording

```json
{
  "id": "demo-code-search",
  "title": "Code Search & Embeddings",
  "description": "Demonstrates semantic code search, cross-repo search, and RAG pipeline integration",
  "estimated_duration_s": 150,
  "steps": [
    {
      "action": "Navigate",
      "target": "vibeui://open?folder=/home/user/my-project"
    },
    {
      "action": "Narrate",
      "value": "First, we index the project to generate vector embeddings for all source files and functions."
    },
    {
      "action": "Click",
      "target": ".panel-tab[data-panel='search']",
      "description": "Open the Code Search panel"
    },
    {
      "action": "Click",
      "target": "#build-index-btn",
      "description": "Start building the embeddings index"
    },
    {
      "action": "WaitForSelector",
      "target": ".index-progress-bar",
      "timeout_ms": 3000
    },
    {
      "action": "Screenshot",
      "label": "indexing-in-progress"
    },
    {
      "action": "Narrate",
      "value": "Indexing scans source files, extracts functions and classes, generates embeddings with the configured model, and stores them in a local vector database."
    },
    {
      "action": "WaitForSelector",
      "target": ".index-complete-badge",
      "timeout_ms": 30000
    },
    {
      "action": "Screenshot",
      "label": "indexing-complete"
    },
    {
      "action": "Assert",
      "target": ".index-stats .file-count",
      "value": "greater_than:0"
    },
    {
      "action": "Narrate",
      "value": "Index is built. Now let's perform a semantic search. We'll ask for authentication logic even though we don't know the exact function names."
    },
    {
      "action": "Type",
      "target": ".search-input",
      "value": "function that handles HTTP authentication"
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
      "label": "semantic-search-results"
    },
    {
      "action": "Assert",
      "target": ".search-result:first-child .score",
      "value": "greater_than:0.8"
    },
    {
      "action": "Click",
      "target": ".search-result:first-child",
      "description": "Click the top result to navigate to it in the editor"
    },
    {
      "action": "Screenshot",
      "label": "navigate-to-result"
    },
    {
      "action": "Narrate",
      "value": "Clicking a result jumps to the exact location in the editor. Now let's try RAG mode to get an AI-synthesized answer grounded in our code."
    },
    {
      "action": "Click",
      "target": "#rag-mode-toggle",
      "description": "Enable RAG mode"
    },
    {
      "action": "Type",
      "target": ".search-input",
      "value": "How does authentication work in this project?"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Enter"
    },
    {
      "action": "Wait",
      "duration_ms": 4000
    },
    {
      "action": "Screenshot",
      "label": "rag-answer"
    },
    {
      "action": "Assert",
      "target": ".rag-answer",
      "value": "contains:JWT"
    },
    {
      "action": "Narrate",
      "value": "The RAG pipeline retrieved the most relevant code chunks, injected them as context, and the AI produced a grounded explanation with source references. Each source link is clickable."
    },
    {
      "action": "Click",
      "target": ".rag-source-link:first-child",
      "description": "Click a source reference to navigate to it"
    },
    {
      "action": "Screenshot",
      "label": "rag-source-navigation"
    }
  ],
  "tags": ["search", "embeddings", "rag", "semantic-search", "vector-db"]
}
```


## What's Next

- [Demo 09 — Autofix & Diagnostics](../09-autofix/) — AI-powered fixes for issues found during search
- [Demo 07 — Inline Chat & Completions](../07-inline-chat/) — Edit code directly from search results
- [Demo 10 — Code Transforms](../10-code-transforms/) — Refactor code found via semantic search
