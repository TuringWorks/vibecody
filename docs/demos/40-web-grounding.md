---
layout: page
title: "Demo 40: Web Search Grounding"
permalink: /demos/40-web-grounding/
---


## Overview

VibeCody can ground AI responses in live web search results, ensuring answers reflect current documentation, release notes, and community knowledge rather than relying solely on the model's training data. Configure one or more search providers (Google, Bing, Brave, SearXNG, Tavily), and the agent automatically searches the web when a query benefits from up-to-date information. Results include inline citations so you can verify sources.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured
- A search provider API key (or a self-hosted SearXNG instance for offline use)
- For VibeUI: the desktop app running with the **WebGrounding** panel visible

## Supported Search Providers

| Provider   | API Key Env Var         | Free Tier     | Notes                                    |
|------------|-------------------------|---------------|------------------------------------------|
| **Google**   | `GOOGLE_SEARCH_API_KEY` | 100 queries/day | Requires Custom Search Engine ID       |
| **Bing**     | `BING_SEARCH_API_KEY`   | 1,000/month   | Azure Cognitive Services                 |
| **Brave**    | `BRAVE_SEARCH_API_KEY`  | 2,000/month   | Privacy-focused, no tracking             |
| **Tavily**   | `TAVILY_API_KEY`        | 1,000/month   | Optimized for AI grounding               |
| **SearXNG**  | None (self-hosted)      | Unlimited     | Docker: `docker run -p 8080:8080 searxng/searxng` |

## Step-by-Step Walkthrough

### 1. Configure a Search Provider

Add your search provider to `~/.vibecli/config.toml`:

```toml
[websearch]
enabled = true
default_provider = "brave"
max_results = 5
cache_ttl = "1h"

[websearch.brave]
api_key = "BSA_your_key_here"

[websearch.searxng]
url = "http://localhost:8080"
```

Or set the API key as an environment variable:

```bash
export BRAVE_SEARCH_API_KEY="BSA_your_key_here"
```

### 2. Run a Web Search

Search the web directly from the REPL.

**REPL:**

```bash
vibecli
> /websearch "how to use tokio channels in Rust"
```

Example output:

```
Web Search Results (Brave, 5 results):

  1. Tokio Tutorial — Channels
     https://tokio.rs/tokio/tutorial/channels
     "Tokio provides a number of channels, each serving a different
     purpose: mpsc, oneshot, broadcast, and watch..."
     Updated: 2026-02-15

  2. Rust by Example — Channels with Tokio
     https://doc.rust-lang.org/rust-by-example/std_misc/channels.html
     "The mpsc::channel function returns a (Sender, Receiver) tuple.
     Multiple producers can clone the sender..."
     Updated: 2026-01-20

  3. Stack Overflow — Tokio mpsc vs broadcast channel
     https://stackoverflow.com/questions/78901234/tokio-mpsc-vs-broadcast
     "Use mpsc when you have multiple producers and one consumer. Use
     broadcast when every consumer needs every message..."
     Votes: 47 | Updated: 2026-03-10

  4. Tokio API Docs — tokio::sync::mpsc
     https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html
     "A multi-producer, single-consumer channel. Each message sent is
     delivered to exactly one receiver..."
     Updated: 2026-03-01

  5. Blog — Practical Tokio Channel Patterns
     https://blog.example.com/tokio-channel-patterns
     "In this post we cover fan-out, fan-in, and pipeline patterns
     using tokio channels..."
     Updated: 2026-02-28
```

### 3. Grounded Agent Responses

When you ask the agent a question and web grounding is enabled, the agent automatically searches the web and incorporates findings with citations.

**REPL:**

```bash
vibecli
> How do I set up tokio channels for a work-stealing pattern?
```

Example output:

```
To implement a work-stealing pattern with Tokio channels, you can use
an `mpsc` channel where multiple worker tasks pull from a shared
receiver. Here's a practical approach:

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::channel::<Task>(100);
    let rx = Arc::new(Mutex::new(rx));

    // Spawn worker pool
    for id in 0..4 {
        let rx = rx.clone();
        tokio::spawn(async move {
            while let Some(task) = rx.lock().await.recv().await {
                println!("Worker {id} processing: {task:?}");
                process(task).await;
            }
        });
    }

    // Send work items
    for i in 0..20 {
        tx.send(Task::new(i)).await.unwrap();
    }
}
```

Note: Wrapping the receiver in `Arc<Mutex<_>>` allows multiple workers
to compete for messages, effectively creating a work-stealing pool [1].
For higher throughput, consider the `async-channel` crate which provides
a multi-consumer channel without the mutex overhead [3].

**Key considerations:**
- Channel buffer size affects backpressure behavior [4]
- Use `tokio::sync::Semaphore` alongside channels to limit concurrency [1]
- For fan-out (sending to all workers), use `broadcast` instead [2]

---
Sources:
[1] https://tokio.rs/tokio/tutorial/channels
[2] https://stackoverflow.com/questions/78901234/tokio-mpsc-vs-broadcast
[3] https://blog.example.com/tokio-channel-patterns
[4] https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html
```

The numbered citations link back to the web search results. You can click them in VibeUI or copy the URLs from the terminal.

### 4. Search with a Specific Provider

Override the default search provider for a single query.

**REPL:**

```bash
vibecli
> /websearch --provider google "rust async trait stabilization"
```

Example output:

```
Web Search Results (Google, 5 results):

  1. Rust Blog — Async Traits Stabilized in 1.85
     https://blog.rust-lang.org/2026/01/09/async-trait-stabilized.html
     "We are excited to announce that async fn in traits is now stable
     as of Rust 1.85..."
     Updated: 2026-01-09

  2. GitHub — rust-lang/rust PR #12345
     https://github.com/rust-lang/rust/pull/12345
     "Stabilize async fn in traits for Rust 1.85..."
     Updated: 2025-12-20
  ...
```

### 5. Control When Grounding Activates

By default, the agent decides when a query benefits from web search. You can control this behavior.

**REPL:**

```bash
vibecli
> /websearch config --mode auto
```

Available modes:

| Mode       | Behavior                                                      |
|------------|---------------------------------------------------------------|
| `auto`     | Agent decides when to search (default)                        |
| `always`   | Every query triggers a web search                             |
| `manual`   | Only search when you explicitly use `/websearch`              |
| `ask`      | Agent asks permission before searching                        |

### 6. View Search Cache

Web search results are cached to avoid redundant API calls.

**REPL:**

```bash
vibecli
> /websearch cache
```

Example output:

```
Search Cache (12 entries, 45 KB):

  Query                                Provider  Age     Results
  tokio channels in Rust               brave     2m      5
  rust async trait stabilization       google    15m     5
  serde rename_all options             brave     1h      5
  axum middleware examples             brave     3h      5
  ...

Cache TTL: 1 hour
To clear: /websearch cache --clear
```

### 7. SearXNG for Offline/Private Search

For air-gapped or privacy-sensitive environments, use a self-hosted SearXNG instance.

```bash
# Start SearXNG locally
docker run -d --name searxng -p 8080:8080 searxng/searxng
```

Configure VibeCLI to use it:

```toml
[websearch]
default_provider = "searxng"

[websearch.searxng]
url = "http://localhost:8080"
```

Then search as normal:

```bash
vibecli
> /websearch "tokio select macro usage"
```

SearXNG aggregates results from multiple search engines without sending your queries to a third-party API.

### 8. VibeUI WebGrounding Panel

Open the **WebGrounding** panel in VibeUI to see:

- **Search** tab: run searches, view results with previews, click to open in browser
- **Config** tab: manage providers, set API keys, adjust cache TTL and mode
- **History** tab: past searches with timestamps, providers, and result counts
- **Citations** tab: all citations from grounded agent responses, linked to conversations

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Web Search Grounding",
    "description": "Ground AI responses in live web search results with inline citations.",
    "duration_seconds": 160,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/websearch \"how to use tokio channels in Rust\"", "delay_ms": 5000 }
      ],
      "description": "Run a direct web search"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "How do I set up tokio channels for a work-stealing pattern?", "delay_ms": 8000 }
      ],
      "description": "Ask a question with automatic web grounding"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/websearch --provider google \"rust async trait stabilization\"", "delay_ms": 5000 }
      ],
      "description": "Search with a specific provider"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/websearch config --mode auto", "delay_ms": 2000 }
      ],
      "description": "Configure grounding activation mode"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/websearch cache", "delay_ms": 2000 }
      ],
      "description": "View search result cache"
    },
    {
      "id": 6,
      "action": "vibeui_interaction",
      "panel": "WebGrounding",
      "tab": "Search",
      "description": "Run searches and view results in VibeUI"
    },
    {
      "id": 7,
      "action": "vibeui_interaction",
      "panel": "WebGrounding",
      "tab": "Citations",
      "description": "Browse all citations from grounded responses"
    }
  ]
}
```

## What's Next

- [Demo 8: Code Search & Embeddings](../08-code-search/) -- Semantic search within your codebase
- [Demo 41: Semantic Index](../41-semantic-index/) -- AST-level codebase understanding
- [Demo 39: Proactive Agent](../39-proactive-agent/) -- Background suggestions powered by analysis
