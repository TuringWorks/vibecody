---
triggers: ["session tree", "branch session", "navigate history", "/tree", "branch history", "fork session", "session branch"]
tools_allowed: ["read_file", "write_file", "bash"]
category: session
---
# Session Tree

Rules for working with in-file session tree branching (`session_tree` module).

## Rules

1. **Branch vs. continue**: Continue (append to the active branch) when the conversation is on track. Branch (`branch_from`) when you want to explore an alternative approach, test a hypothesis, or recover from a wrong turn — without discarding prior context.

2. **Single file, multiple branches**: The session tree lives in one JSONL file. Every entry's `parent_id` field encodes the tree structure. Never create a new file just to start a different line of reasoning; use `branch_from` instead.

3. **Label meaningful checkpoints**: After a significant milestone (working prototype, passing tests, confirmed design decision), call `label_entry` with a short, descriptive label (e.g. `"tests-green"`, `"api-design-locked"`). Labels survive serialization and make navigation fast.

4. **Navigate with `/tree path <entry-label>`**: Use `path_to` to reconstruct the full ancestor chain from any labelled checkpoint back to the root. This gives the LLM context exactly the messages that led to that state, with no noise from other branches.

5. **Compaction entries compress context**: When a branch grows long, insert a `Compaction` entry summarising the key conclusions and listing files touched. The LLM reads the compaction summary instead of replaying every prior message, keeping the active context window small.

6. **Custom entries for extension state**: Store plugin or tool state in `Custom { type_name, payload }` entries. This keeps structured data alongside the conversation without polluting the `Message` role stream that gets sent to the LLM.

7. **`BranchSummary` at branch points**: When creating a branch, prepend a `BranchSummary` entry (with a human-readable label) as the first child. This makes branch intent visible in tree-view UIs and in `/tree list`.

8. **Fold before export**: Before exporting or compacting a session, call `fold_subtree` on dead-end branches to exclude their entries. Only the surviving (visible) entries should be included in the exported bundle or sent to the LLM.

9. **Roundtrip safety**: Serialise with `serialize_jsonl` and deserialise with `deserialize_jsonl`. Entry IDs, parent links, labels, and all payload fields are preserved. Never edit the JSONL file manually — the hand-rolled parser is sensitive to structural changes.

10. **Thread safety**: `SessionTree` is not `Send + Sync`. Wrap in `Arc<Mutex<SessionTree>>` when sharing across async tasks or Tauri command handlers.

## Quick Reference

```
/tree list                   # show all branches and leaf labels
/tree branch <entry-label>   # branch from a named checkpoint
/tree path <entry-label>     # print root → entry path
/tree fold <entry-label>     # hide subtree (returns visible count)
/tree label <id> <name>      # attach a label to an entry
/tree compact                # insert a Compaction entry on active branch
```

## Entry Kind Cheat-Sheet

| Kind           | Use case                                      |
|----------------|-----------------------------------------------|
| `Message`      | Normal user/assistant turns                   |
| `ToolCall`     | Tool invocation + result                      |
| `Compaction`   | Context summary; list files changed           |
| `BranchSummary`| Human-readable annotation at a branch point  |
| `Custom`       | Plugin/extension state; arbitrary JSON payload|
