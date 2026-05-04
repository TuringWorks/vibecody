---
layout: page
title: Diffcomplete (⌘.)
permalink: /diffcomplete/
---

# Diffcomplete (⌘.)

> AI editing without ghost-text. Press **⌘.** anywhere in the editor, describe the change you want, review the unified diff hunk-by-hunk, accept or reject what you like.

Diffcomplete is VibeCody's only AI code-editing surface. It deliberately replaces keystroke-driven inline completions with an explicit-trigger, diff-output flow — better for review, better for safety, and patent-distant from FIM / next-edit / ghost-text systems.

---

## Quick start

1. Open any file in **VibeUI** (the desktop editor).
2. (Optional) Select a region of code. If nothing is selected, the whole file is the editing target.
3. Press **⌘.** (Cmd-Period on macOS, Ctrl-Period on Linux/Windows). Or open the command palette and run **AI edit (diff)**.
4. Type the change you want — e.g. *"extract the validation into a helper function"*.
5. Press **⌘⏎** to generate. The model returns a unified diff which is rendered in a side-by-side review.
6. Per hunk: **Accept** keeps the change, **Reject** drops it, **Edit** lets you tweak the proposed text in place.
7. Click **Apply** to write the accepted hunks back to the file.

That's it. There is no other AI editing surface in VibeCody. If something looks like inline completion, it isn't — it's a static lint or formatting hint.

---

## How it works

Diffcomplete sends a single deliberate request per ⌘. press:

```
[ ⌘. press ]
   │
   ▼
DiffCompleteModal  ── user types instruction ──┐
   │                                            │
   ▼                                            ▼
diffcomplete_generate (Tauri command)     [ Enter / ⌘⏎ ]
   │
   ▼
vibe_ai::diffcomplete::generate()
   │
   ├─► Builds a system prompt that demands unified-diff output
   ├─► Includes selection + 200-line context window each side
   ├─► Includes any user-attached "additional files" (explicit picker only)
   ├─► Includes project_memory from VIBECLI.md / AGENTS.md / CLAUDE.md
   │   (author-authored only; never auto-extracted state)
   └─► Calls the active provider's chat() surface — never FIM
   │
   ▼
Unified-diff text  →  applyUnifiedDiff()  →  modified file content
   │
   ▼
DiffReviewPanel  ── user accepts / rejects per hunk ──►  file written
```

**Key design rules:**

- **Trigger is explicit.** Never on keystroke, never on idle, never on selection-change. Only ⌘. (or the equivalent palette command).
- **Output is a unified diff.** Not a code suggestion, not a completion. The model is instructed to emit *exactly one fenced ` ```diff ` block* and the daemon extracts it.
- **Review is mandatory.** The modified content is never silently applied — `DiffReviewPanel` always renders the hunks for review.
- **No hidden state.** The only context shipped to the model is what's visible in the modal: selection + 200-line bounded window, attached files, project memory, and the previous diff if you're refining.

---

## Refinement

After a diff is generated, the modal shows a **Refine this diff** field below the review.

- Type a refinement instruction (e.g. *"tighten the error path"*) and press **⌘⏎** to regenerate.
- The previous diff is included in the next request as a *"Previous attempt"* block, so the model refines instead of starting over.
- The refinement is layered *on top of* your original instruction — you can iterate without losing intent.
- You can repeat refinement as many times as needed; only the most recent diff is kept as context.

The refinement chain is visible to you and to the model — there is no auto-retry or shadow-context.

---

## Additional files (manual context)

The **+ Add file…** button under the instruction box opens a native file picker. Selected files are read by the daemon (sandboxed) and included in the prompt under an *"Additional files (user-supplied context)"* section.

This is **human-in-the-loop retrieval** by design:

- Files are added by you, never by automatic embedding search or call-graph traversal.
- No usage telemetry is collected from this picker.
- You can remove an attached file via its **×** button before submitting.

If you want a file in scope, attach it. Diffcomplete will not guess.

---

## Configuration

### Default provider

Diffcomplete uses the AI provider that's currently active in VibeUI's Settings → API Keys. Any provider with a `chat()` surface works — Anthropic, OpenAI, Gemini, Groq, OpenRouter, Mistral, DeepSeek, Cerebras, Together, Fireworks, SambaNova, Perplexity, and the local mistralrs / Ollama paths are all supported.

To change the active provider mid-session: Settings → API Keys → click the provider you want.

### Required configuration

**At least one provider key.** No provider configured = no diffcomplete.

Set a key one of two ways:

1. **In-app:** Settings → API Keys → choose provider → paste key → Save.
2. **From the terminal:**
   ```bash
   vibecli set-key anthropic sk-ant-...
   vibecli set-key openai    sk-...
   vibecli set-key huggingface hf_...   # for gated mistralrs models
   ```

Keys are stored in the encrypted ProfileStore at `~/.vibecli/profile_settings.db` — never in `.env` files, never in plaintext.

### Verifying readiness

The daemon's `/health` endpoint exposes the canonical readiness signal:

```bash
curl http://127.0.0.1:7878/health | jq '.features.diffcomplete'
```

```json
{
  "available": true,
  "requires": "providers.configured_count > 0",
  "transport": "tauri-desktop"
}
```

`available` is `true` whenever `/health.providers.configured_count > 0`. Diffcomplete inherits its readiness from the canonical providers signal — there's no separate per-feature probe.

---

## Troubleshooting

### "No active AI provider configured"

You don't have any provider keys set. Open Settings → API Keys and add one, or run `vibecli set-key <provider> <value>` in your terminal.

### "Model response did not contain a diff block"

The model didn't follow the unified-diff format. This happens occasionally with smaller open-weights models. Workarounds:

- Switch to a stronger provider (Claude, GPT, Gemini Pro tier).
- Simplify the instruction — vague asks ("make this better") confuse the format.
- Try Regenerate with a refinement.

### "Model returned a diff that could not be applied cleanly"

The model invented context lines that don't match your file. This is typically a model-quality issue, not a Diffcomplete bug. Workarounds:

- Click **Regenerate** with a refinement that points at the real code (e.g. *"the variable is called `userId` not `user_id`"*).
- Close the modal, re-select a smaller region (200-line context window applies around the selection), and try again.

### "Provider rate limit hit" / 429

Your provider has throttled you. Wait, or switch to a different provider via Settings.

### "401 Unauthorized" / "Invalid API key"

The provider rejected the key. Re-check it in Settings → API Keys. Some providers (Anthropic, OpenAI) rotate trial keys quickly.

### Network timeout

Slow connection or the provider's region is unhealthy. Switch providers, or wait it out.

---

## Scope

Diffcomplete is currently **desktop-only** — it ships as a Tauri command in VibeUI, not as a daemon HTTP route. Mobile / Watch / IDE plugins do not surface it today.

If you need diffcomplete from another client surface, file an issue describing the use case. The backend (`vibe_ai::diffcomplete`) is already a clean library and can be exposed via daemon HTTP without protocol changes.

---

## Observability

Every diffcomplete generation emits structured `tracing` events on the daemon under the `vibecody::diffcomplete` target:

- `debug` on request entry — provider, language, file_path, instruction_len, context_lines, has_selection, is_refinement, extra_files
- `warn` when the provider is unavailable, the chat call fails, or no diff block was returned
- `info` on success — provider, diff_len, had_explanation

Tail the daemon log with:

```bash
journalctl --user -u vibecli  # systemd
# or
RUST_LOG=vibecody::diffcomplete=info vibecli serve
```

User content (instruction text, file paths) is **not** logged at any level — only shapes (length, count) and stable enums (provider name, language). No telemetry leaves your machine without explicit opt-in.

---

## Security

- **No autocomplete from history.** The model never sees your shell history, edit history, or any state you didn't explicitly attach.
- **Project memory is author-authored only.** The `project_memory` field carries content from `VIBECLI.md` / `AGENTS.md` / `CLAUDE.md` only — never auto-extracted user state, scratchpads, or orchestration logs.
- **Sandboxed file reads.** Files attached via **+ Add file…** are read through `read_file_sandbox`, which respects the active workspace boundary.
- **No eval, no exec.** The applier is a pure unified-diff parser — `applyUnifiedDiff()` in the modal source. It will refuse to apply hunks whose context doesn't match the current file content.

---

## What Diffcomplete is NOT

For clarity (and because we removed these on purpose):

- **Not** keystroke-driven ghost text. There is no `registerInlineCompletionsProvider` path in VibeCody.
- **Not** FIM (fill-in-middle). The model receives prefix + selection + suffix as discrete labeled regions, not a single FIM template.
- **Not** next-edit prediction. The model only sees the current file state, never your past edits.
- **Not** auto-retrieval. Files in context come from your explicit picker. There is no embedding search, no call-graph walker, no symbol-server probe.

Diffcomplete is a deliberate, claim-distant alternative to those patterns. If you want a different shape of AI editing, that is a feature request — but the answer will not be re-introducing ghost text.

---

## Related

- **Design doc:** [`docs/design/recap-resume/03-diffcomplete.md`](https://github.com/TuringWorks/vibecody/blob/main/docs/design/recap-resume/03-diffcomplete.md)
- **Source:** [`vibeui/crates/vibe-ai/src/diffcomplete.rs`](https://github.com/TuringWorks/vibecody/blob/main/vibeui/crates/vibe-ai/src/diffcomplete.rs) · [`vibeui/src/components/DiffCompleteModal.tsx`](https://github.com/TuringWorks/vibecody/blob/main/vibeui/src/components/DiffCompleteModal.tsx)
- **Patent audit working doc:** `notes/PATENT_AUDIT_INLINE.md` (gitignored — local only)
