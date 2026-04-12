# Tool Pair Compaction

Compact conversation context while preserving tool call/result pairs and critical semantic boundaries. Generates structured summaries with key decisions, file changes, and tool usage stats.

## When to Use
- Reducing token count before hitting context limits
- Preserving tool call/result pairs during compaction
- Generating a structured summary of session progress
- Identifying safe compaction boundaries in the message history
- Capping summary length for injection into fresh context

## Compaction Strategy
1. Identify "safe boundaries" — points where a tool call/result pair is complete
2. Summarize pairs between boundaries into a `CompactionSummary`
3. Preserve the most recent N pairs verbatim (configurable)
4. Render the summary as a compact markdown block for re-injection

## CompactionSummary Fields
- `total_tokens_saved` — tokens freed by compaction
- `pairs_compacted` — number of tool pairs summarized
- `key_decisions` — important choices made during the session
- `files_changed` — list of modified files
- `tool_stats` — count of each tool used

## Commands
- `/compact now` — Compact conversation using default settings
- `/compact preview` — Show what would be compacted
- `/compact summary` — Display the current compaction summary
- `/compact boundary` — Find the next safe compaction point
- `/compact config` — Show/set compaction thresholds

## Examples
```
/compact preview
# Safe boundary at message 42 (after Edit result)
# Would compact 38 messages → ~12,000 tokens saved

/compact now
# Compacted: 38 messages, 12,450 tokens saved
# Key decisions: switched to async I/O, added clippy::pedantic
# Files changed: src/lib.rs, src/agent.rs, Cargo.toml
```
