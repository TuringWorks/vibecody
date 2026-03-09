---
triggers: ["infinite context", "context window", "large codebase", "context management", "token budget", "context compression", "codebase understanding", "code context", "unlimited context"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# Infinite Code Context

When working with large codebases that exceed context window limits:

1. Use hierarchical context representation: Level 0 (full file), Level 1 (function summaries), Level 2 (file skeleton — signatures only), Level 3 (module summary — one sentence per file), Level 4 (project architecture overview).
2. Score file relevance using multiple signals: recency (recently modified files score higher), proximity (files in same directory as current file), keyword match (TF-IDF against current query), dependency (import graph distance), and access frequency.
3. Manage the context window with a token budget (default 100K tokens): automatically evict lowest-relevance chunks when the budget is exceeded, or compress existing chunks to higher depth levels (full → summary → skeleton → signatures).
4. Use progressive disclosure: start with Level 3-4 summaries for the entire project, then expand specific files to full content (Level 0) as needed — this gives both breadth and depth within the token budget.
5. Cache processed summaries in an LRU cache (default 1000 entries) — invalidate cache entries when files are modified to ensure freshness.
6. When switching between tasks, use `refresh_context()` to re-score and rebalance the context window for the new query — files relevant to the previous task may be compressed to make room for newly relevant files.
7. Estimate tokens with the word/4 heuristic (1 token ≈ 4 characters) — this is fast and accurate enough for budget management without requiring tokenizer dependencies.
8. For very large monorepos (100K+ files), build a project-level summary first (Level 4), then lazily expand directories and files as the conversation explores different areas of the codebase.
