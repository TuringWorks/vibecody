---
layout: page
title: "Demo 47: Multi-LLM Deliberation & Routing"
permalink: /demos/47-counsel-superbrain/
---


## Overview

Different LLMs have different strengths. Claude excels at nuanced reasoning, GPT-4o is fast and practical, and Gemini brings strong multimodal capabilities. VibeCody's Counsel and SuperBrain features let you harness multiple models simultaneously. Counsel runs structured debates where models argue different positions on architectural decisions. SuperBrain queries multiple providers in parallel and synthesizes a consensus answer. This demo shows both features in action.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI 0.5.1 installed and on your PATH
- Two or more AI providers configured (e.g., Claude + OpenAI, or Claude + Gemini + Groq)
- For VibeUI: the desktop app running with the **CounselPanel** and **SuperBrainPanel** visible

## Counsel vs SuperBrain

| Feature        | Counsel                          | SuperBrain                         |
|----------------|----------------------------------|------------------------------------|
| Purpose        | Structured debate on decisions   | Consensus answer to questions      |
| Mode           | Adversarial (pro/con/judge)      | Collaborative (parallel + merge)   |
| Output         | Verdict with reasoning           | Unified answer with confidence     |
| Best for       | Architecture, design decisions   | Technical questions, explanations  |
| Provider count | 3 recommended (pro, con, judge)  | 2+ (more = higher confidence)      |

## Step-by-Step Walkthrough

### Step 1: Start a Counsel debate

Launch a structured debate on an architectural question:

```bash
vibecli
```

```
> /counsel new "Should we use microservices or a modular monolith for our payment system?"
```

```
[Counsel] Initializing debate session...
[Counsel] Topic: Should we use microservices or a modular monolith for our payment system?
[Counsel] Assigning roles:
  PRO  (microservices):  claude/claude-sonnet-4
  CON  (monolith):       openai/gpt-4o
  JUDGE:                 claude/claude-opus-4

═══════════════════════════════════════════════════
  COUNSEL DEBATE — Round 1 of 3
═══════════════════════════════════════════════════

── PRO (Claude Sonnet — advocating microservices) ──

Microservices are the right choice for a payment system because:

1. **Independent scaling**: Payment processing has burst traffic patterns.
   Microservices let you scale the transaction processor independently
   from the invoicing service, reducing infrastructure cost by 30-40%.

2. **Fault isolation**: A bug in the notification service should never
   bring down payment processing. Service boundaries enforce this.

3. **Team autonomy**: Payment systems involve PCI compliance, fraud
   detection, and billing — different teams can own different services
   with independent deploy cycles.

4. **Technology diversity**: Fraud detection benefits from Python/ML
   tooling while transaction processing needs low-latency Rust/Go.

── CON (GPT-4o — advocating modular monolith) ──

A modular monolith is the pragmatic choice:

1. **Distributed transactions are hard**: Payments require ACID
   guarantees. Sagas and eventual consistency add enormous complexity
   and subtle failure modes (double charges, lost payments).

2. **Operational overhead**: Microservices require service mesh,
   distributed tracing, API gateways, and container orchestration.
   A monolith needs none of these.

3. **Latency**: Inter-service network calls add 1-5ms per hop. A
   payment flow touching 4 services adds 4-20ms vs near-zero for
   in-process module calls.

4. **Start simple**: You can always extract services later when you
   have proven the domain boundaries. Premature decomposition is
   the #1 cause of failed microservice migrations.

[Counsel] Round 1 complete. Starting Round 2 (rebuttals)...
```

### Step 2: Watch the rebuttal rounds

```
═══════════════════════════════════════════════════
  COUNSEL DEBATE — Round 2 of 3 (Rebuttals)
═══════════════════════════════════════════════════

── PRO rebuttal ──

The distributed transaction concern is valid but solvable. The Outbox
pattern with idempotency keys eliminates double charges. As for
operational overhead — tools like Kubernetes and Istio make service
mesh nearly turnkey. The real risk is a monolith that grows into a
big ball of mud with tangled payment logic.

── CON rebuttal ──

"Nearly turnkey" is doing heavy lifting. Kubernetes alone requires
a dedicated platform team. The Outbox pattern works but adds
complexity that a simple database transaction handles for free.
Module boundaries in a monolith provide the same logical separation
as service boundaries without the network partition tax.

═══════════════════════════════════════════════════
  COUNSEL DEBATE — Round 3 of 3 (Final Arguments)
═══════════════════════════════════════════════════

── PRO final ──
Start with 3 services (Gateway, Processor, Ledger) — not 20.
The boundary is clean and well-understood. Scale from there.

── CON final ──
Start with a modular monolith with enforced module boundaries
(Rust's crate system or Java's module system). Extract services
only when monitoring proves a bottleneck.
```

### Step 3: Read the verdict

```
═══════════════════════════════════════════════════
  VERDICT — Judge (Claude Opus)
═══════════════════════════════════════════════════

**Recommendation: Start with a modular monolith, plan for extraction.**

Reasoning:

Both sides made strong arguments. The PRO case for independent scaling
and fault isolation is technically sound but assumes a team size and
traffic volume that may not exist yet. The CON case correctly identifies
that distributed transactions in payment systems are genuinely dangerous
— a saga that fails mid-flight can charge a customer without recording
the payment.

The optimal path:
1. Build a modular monolith with strict module boundaries (enforced
   by the type system, not just conventions)
2. Design inter-module APIs as if they were service APIs (async
   message passing, no shared mutable state)
3. Extract the fraud detection module first (it has the clearest
   boundary and benefits most from independent scaling)
4. Extract the notification service second (fire-and-forget, low risk)
5. Keep the payment processor and ledger co-located until transaction
   volume exceeds 10K/second

Confidence: 82%
Winning argument: CON (modular monolith) with PRO's extraction roadmap

Debate stats:
  Rounds:       3
  Total tokens: 4,821
  Cost:         $0.089
  Duration:     28s
```

### Step 4: Launch a SuperBrain consensus query

For technical questions where you want a synthesized answer:

```
> /superbrain consensus "Explain monads in programming with a practical Rust example"
```

```
[SuperBrain] Querying 3 providers in parallel...
  ├── claude/claude-sonnet-4  ... ✓ (1.2s)
  ├── openai/gpt-4o           ... ✓ (0.9s)
  └── groq/llama-3.3-70b      ... ✓ (0.4s)

[SuperBrain] Analyzing responses for consensus...

═══════════════════════════════════════════════════
  SUPERBRAIN CONSENSUS
═══════════════════════════════════════════════════

**Consensus answer** (3/3 providers agree on core points):

A monad is a design pattern that wraps a value in a context and provides
two operations:
- `wrap` (called `pure` or `return`): put a value into the context
- `flat_map` (called `bind` or `and_then`): chain operations that
  produce new wrapped values

In Rust, `Option` and `Result` are monads:

```rust
fn parse_port(s: &str) -> Option<u16> {
    s.parse::<u16>().ok()  // Result → Option via .ok()
}

fn lookup_service(port: u16) -> Option<String> {
    match port {
        80 => Some("HTTP".to_string()),
        443 => Some("HTTPS".to_string()),
        _ => None,
    }
}

// Monadic chaining with and_then (flat_map):
let service = Some("443")
    .and_then(|s| parse_port(s))     // Option<u16>
    .and_then(|p| lookup_service(p)); // Option<String>

assert_eq!(service, Some("HTTPS".to_string()));
```

The key insight: `and_then` handles the None/Err case automatically,
so you never write nested match statements.

**Provider-specific insights:**

| Provider   | Unique contribution                                          |
|------------|--------------------------------------------------------------|
| Claude     | Highlighted the monad laws (left identity, right identity,   |
|            | associativity) and why they matter for refactoring safety    |
| GPT-4o     | Provided an async monad example using Future + .await chains |
| Llama 3.3  | Drew the Haskell comparison: do-notation vs Rust's ? operator|

Consensus confidence: 94% (high agreement on definition and example)
Divergence: Low (stylistic differences only, no factual disagreements)

Cost: $0.018 (3 parallel queries)
Tokens: 2,104 total across providers
```

### Step 5: Use SuperBrain for code review

Get multiple perspectives on a code change:

```
> /superbrain review src/auth.rs
```

```
[SuperBrain] Sending src/auth.rs to 3 providers for review...
  ├── claude/claude-sonnet-4  ... ✓
  ├── openai/gpt-4o           ... ✓
  └── groq/llama-3.3-70b      ... ✓

[SuperBrain] Merging review feedback...

Code Review Consensus — src/auth.rs
════════════════════════════════════

Issues found (agreed by 2+ providers):

  [HIGH] Line 34: Password comparison uses == instead of constant-time compare
    Agreed by: Claude ✓, GPT-4o ✓, Llama ✓ (3/3)
    Fix: Use subtle::ConstantTimeEq or ring::constant_time::verify_slices_are_equal

  [MEDIUM] Line 67: Token expiry parsed but timezone not validated
    Agreed by: Claude ✓, GPT-4o ✓ (2/3)
    Fix: Explicitly parse as UTC with chrono::Utc::now()

  [LOW] Line 12: Unused import `std::collections::BTreeMap`
    Agreed by: GPT-4o ✓, Llama ✓ (2/3)
    Fix: Remove the import

Provider-only findings (mentioned by 1 provider):
  Claude: Consider adding rate limiting to validate_user()
  GPT-4o: Document the expected token format in a doc comment

Overall quality: 7/10
Consensus confidence: 89%
```

### Step 6: View deliberation history

```
> /counsel history
```

```
Counsel & SuperBrain History
════════════════════════════

ID                  │ Type       │ Topic                                    │ Cost
────────────────────┼────────────┼──────────────────────────────────────────┼───────
counsel-001         │ Debate     │ Microservices vs monolith                │ $0.089
superbrain-001      │ Consensus  │ Explain monads                           │ $0.018
superbrain-002      │ Review     │ src/auth.rs                              │ $0.024

Total deliberation cost: $0.131
```

### Step 7: Use Counsel and SuperBrain in VibeUI

In the VibeUI desktop app:

- **CounselPanel** -- Visual debate format with PRO/CON columns, round progression, and highlighted verdict. Save debates for future reference.
- **SuperBrainPanel** -- Side-by-side provider responses with consensus highlights. Venn diagram showing agreement and unique insights per provider.

## Demo Recording

```json
{
  "meta": {
    "title": "Multi-LLM Deliberation & Routing",
    "description": "Structured debates with Counsel and parallel consensus with SuperBrain.",
    "duration_seconds": 200,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/counsel new \"Should we use microservices or a modular monolith for our payment system?\"", "delay_ms": 30000 },
        { "input": "/superbrain consensus \"Explain monads in programming with a practical Rust example\"", "delay_ms": 15000 },
        { "input": "/superbrain review src/auth.rs", "delay_ms": 15000 },
        { "input": "/counsel history", "delay_ms": 2000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Run a Counsel debate, SuperBrain consensus, SuperBrain review, and check history"
    }
  ]
}
```

## What's Next

- [Demo 42: MCTS Code Repair](../42-mcts-repair/) -- Fix bugs with tree-search exploration
- [Demo 43: Cost-Optimized Agent Routing](../43-cost-routing/) -- Route tasks to the cheapest viable model
- [Demo 5: Model Arena](../model-arena/) -- Pit models against each other on coding tasks
