---
triggers: ["dual log", "dual-log", "session log", "log.jsonl", "context.jsonl", "append-only log", "compacted context", "pi-mom", "pi-mono gap", "channel session", "DualLog"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# Dual-Log Session Logging

Rules for working with the `dual_log` module (pi-mono gap bridge, Phase B4).

## Architecture

Two files per session/channel:

| File | Purpose | Compacted? |
|---|---|---|
| `log.jsonl` | Append-only, infinite history | Never |
| `context.jsonl` | Bounded LLM context window | Yes, via `compact()` |

`DualLog` holds both in memory as `full_log` (Vec) and `context` (Vec).  A `sync_watermark` tracks how far into `full_log` the context has already been synced.

---

## Rule 1 — Always call `sync_context()` before an agent turn

The agent may have been idle while new entries were appended (e.g. background tool results, injected system messages).  Call `sync_context()` at the start of every turn to pull those entries into the context window before sending it to the LLM.

```rust
dual_log.sync_context();
let messages = dual_log.context_entries();
llm.chat(messages).await
```

---

## Rule 2 — Use `grep_log()` for history; never re-inject old entries into context

When the agent needs information from earlier in the session that has been evicted from the context window, call `grep_log(pattern)` to retrieve matching entries.  Extract only the relevant fact and inject a *short summary* into the context — do **not** re-add the raw old entries, as that defeats the purpose of compaction and wastes tokens.

```rust
// Good: targeted search, inject summary
let hits = dual_log.grep_log("user's project name");
if let Some(e) = hits.first() {
    // parse fact from e.content, inject a 1-sentence summary
}

// Bad: dumping log entries back into context
dual_log.context.extend(hits.iter().cloned());
```

---

## Rule 3 — Size `max_context_entries` to match the LLM's token budget, not message count

A typical guidance:

| Context window | Suggested `max_context_entries` |
|---|---|
| 8 k tokens | 20–40 |
| 32 k tokens | 80–150 |
| 128 k tokens | 300–500 |
| 200 k+ tokens | 500–1 000 |

Adjust downward if entries are long (code blocks, tool outputs).

---

## Rule 4 — Compact proactively, not reactively

Call `compact()` when `is_context_full()` returns `true` *before* trying to append.  Do not wait until overflow causes entries to be silently dropped from the front.

```rust
if dual_log.is_context_full() {
    let summary = summarise(&dual_log.context_entries()[..keep_recent]);
    dual_log.compact(&summary, keep_recent);
}
dual_log.append(new_entry);
```

---

## Rule 5 — Choose `keep_recent` to preserve the last few turns of dialogue

Preserve at least 3–5 entries (a couple of user/assistant pairs) so the LLM retains immediate conversational context.  A good default is `max_context_entries / 4`.

```rust
let keep_recent = (dual_log.max_context_entries / 4).max(3);
dual_log.compact(&summary, keep_recent);
```

---

## Rule 6 — Write summaries that are dense but faithful

A compaction summary should capture:
- The user's original goal / intent.
- Any decisions or conclusions reached.
- Key facts the LLM will need to continue.

Avoid long paraphrases — a 2–4 sentence summary of 20 messages is adequate.

---

## Rule 7 — Persist after every agent turn, not only on shutdown

Call `persist(log_path, context_path)` at the end of each turn.  The per-file atomic write (write-to-tmp then rename) prevents corruption on crash.  Frequent persists keep `log.jsonl` current so `grep_log()` searches reflect the latest state even across process restarts.

```rust
dual_log.append(assistant_response);
dual_log.sync_context();
dual_log.persist(&log_path, &ctx_path)?;
```

---

## Rule 8 — On restore, verify watermark alignment

After `DualLog::load()`, the `sync_watermark` is set to `full_log.len()`.  If `log.jsonl` was appended to by another process between the last persist and this reload (e.g. a parallel agent), call `sync_context()` immediately after load to pull in any gap entries.

```rust
let mut dl = DualLog::load(&full_src, &ctx_src, max)?;
dl.sync_context(); // catches any entries added since last persist
```

---

## Rule 9 — Never modify `log.jsonl` — only append

`log.jsonl` is the source of truth for auditing, debugging, and historical search.  Any mutation (delete, rewrite) destroys the integrity of the history.  If an entry was sent in error, append a *retraction* entry with `role: System` and content describing the retraction — never remove the original.

---

## Rule 10 — Keep `grep_log()` searches out of the LLM prompt

`grep_log()` is an agent tool executed in Rust.  The search results are consumed programmatically and only a brief synthesis is injected into the context.  Never include the raw JSONL output of `grep_log()` in the LLM prompt — it pollutes the context window with already-known history and wastes tokens.
