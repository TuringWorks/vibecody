# 03 — Diffcomplete Recap & Resume

**Scope:** the regenerate-with-refinement chain on a single diffcomplete invocation
**Parent:** [`README.md`](./README.md)
**Status:** Draft · 2026-04-26

---

> **⚠ Patent-distance constraint.**
> Diffcomplete is the project's deliberately patent-distant alternative to inline AI editing. Any feature on this surface — including recap and resume — is required to walk the five-element claim checklist before merging.
>
> All five elements are summarized in `notes/PATENT_AUDIT_INLINE.md` (gitignored). Cross-reference: [README.md → Patent-distance posture](./README.md#patent-distance-posture).
>
> If this doc seems to over-constrain the design, that is intentional — the constraint is the point.

---

## What's there today

Grounded in `b1e28ad1` (Phase 2 slice 3, just shipped):

- **Backend:** `vibeui/crates/vibe-ai/src/diffcomplete.rs` — `DiffCompleteRequest` includes `previous_diff: Option<String>` and `refinement: Option<String>`; the prompt renders `=== Previous attempt ===` then `Refinement:` then `Instruction:`. Slice 3 confirmed.
- **Tauri command:** `diffcomplete_generate` in `vibeui/src-tauri/src/commands.rs:6522` accepts the two optional fields.
- **Modal:** `vibeui/src/components/DiffCompleteModal.tsx` — keeps `lastDiff`, `refinement`, `phase`, `instruction`, `additionalFiles`, `modified` in **component state only**. Closing the modal loses the chain.
- **Tests:** `DiffCompleteModal.test.tsx` covers the regenerate flow.
- **No persistence**, no chain history, no resume of a chain across modal opens.
- **Memory artifact:** `feedback_patent_distance_priority.md` — user explicitly prefers patent-distance moves over ergonomics polish on this surface.

The surface today is: open modal → instruction → diff → review → optionally refine + regenerate → apply or cancel. **All chain state evaporates on close.**

## Goals

1. **Persist the chain** for the lifetime of a workspace, so a closed-then-reopened modal can resume the same diff chain.
2. **Generate a recap** of a chain when the user applies, cancels, or closes the modal — capturing the iteration trail.
3. **Resume** lets the user re-open any prior chain and continue from any step in it (chain forking is fine; chain rewriting is not).
4. Keep all five patent-distance elements distant. Recap is a **side panel artifact, never inlined**, **never auto-generated during typing**, **never decorated with accept/reject affordances on code**.

## Non-goals

- Auto-suggesting diffs as the user types. **Forbidden by patent-distance posture.**
- Surfacing the chain in a hover popup or inline ghost-text. **Forbidden.**
- Mobile / watch composition of new diff chains. Mobile and watch can **read** chain recaps (handoff use case), but composition stays on desktop.
- Cross-workspace chain visibility. Chains live in the workspace they were created in.

## Patent-distance walk-through

Before describing the design, walk the five claim elements explicitly. Each row says how the proposed feature stays distant.

| Element | Standard inline-suggestion claim | Diffcomplete recap/resume posture |
|---|---|---|
| **1. Continuous monitoring of typing** | The system monitors keystrokes and selection state continuously to predict completions. | Recap is generated *only* on explicit modal close / apply / cancel. No keyboard listener, no idle scanner. Chain persistence is a *result-of-action* write, not a continuous capture. |
| **2. Predictive completion** | The system predicts code the user is *about to type*. | Recap and resume look *backward* at user-directed diffs. Resuming a chain replays *prior* user instructions; no forward prediction. |
| **3. Inline UI presentation** | Suggestions render in the editor buffer as ghost text or overlays. | Recap and chain history live in a side panel and the modal footer — never in the editor buffer. The applied diff is the only thing that lands in the buffer, and only on explicit user click. |
| **4. Accept/reject on code** | Tab-to-accept / Esc-to-reject decorate suggested code. | The chain history shows *what was done*, not *what could be done*. There is no accept button on a chain entry — only "View" and "Resume", which open the existing modal flow with its existing explicit Apply gesture. |
| **5. Model-context-window expansion** | The patent posture extends prompt context to drive better predictions. | Resume *replays* prior context that was already user-supplied; recap *summarizes* it. Neither expands context beyond what the user already directed. |

If any future change to this surface would soften any of these rows, the change is required to be re-audited via `notes/PATENT_AUDIT_INLINE.md` *before* implementation.

## Triggers

| Trigger | Default |
|---|---|
| User clicks **Apply** in the modal | Auto-recap (heuristic) |
| User clicks **Cancel** or closes the modal with chain length ≥ 2 | Auto-recap (heuristic) |
| User clicks "Save & close" with chain length ≥ 2 | Auto-recap (heuristic) |
| User clicks "Recap" in the chain history view | On-demand (heuristic or LLM) |

**Chains of length 1** (no regeneration occurred) do not produce a recap. The recap value is in the iteration trail.

**No idle-timer trigger.** Per patent-distance Element 1, recap generation is never time-based.

## Data model

A chain is a sequence of (instruction, refinement, diff, applied?) tuples on a single (file, selection) target.

```rust
// vibeui/crates/vibe-ai/src/diffcomplete_chain.rs (new)
pub struct DiffChain {
    pub id: ChainId,                        // ULID
    pub workspace: PathBuf,
    pub file_path: String,
    pub language: String,
    pub selection_start: u32,               // line numbers of the original selection
    pub selection_end: u32,
    pub original_text: String,              // the selection at first invocation
    pub steps: Vec<DiffChainStep>,
    pub final_state: DiffChainFinal,        // Applied | Cancelled | Open
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schema_version: u16,
}

pub struct DiffChainStep {
    pub index: u32,                         // 0-based
    pub instruction: String,                // first step has the original instruction
    pub refinement: Option<String>,         // None on step 0; Some on regenerations
    pub additional_files: Vec<AdditionalFile>,
    pub diff: String,                       // unified diff text
    pub provider: String,
    pub model: String,
    pub tokens_input: u32,
    pub tokens_output: u32,
    pub generated_at: DateTime<Utc>,
}

pub enum DiffChainFinal {
    Applied { applied_step: u32, applied_at: DateTime<Utc> },
    Cancelled { reason: CancellationReason, cancelled_at: DateTime<Utc> },
    Open,                                   // modal still open, persisted as autosave
}

pub enum CancellationReason { UserCancel, ModalClosed, Error }
```

### Recap shape

Inherits the cross-cutting `Recap` shape. Diffcomplete-specific fields:

```rust
pub struct ResumeHint {
    pub target: ResumeTarget::DiffChain(ChainId),
    pub from_diff_index: Option<u32>,       // step to seed the resumed modal at; default = last
    pub seed_refinement: Option<String>,    // pre-fill refinement input
}
```

A `from_diff_index` of `N` means: open the modal in `review` phase showing `steps[N].diff`, ready to refine or apply. **Forking** (resuming from index N when later steps exist) creates a *new* chain; the old chain is preserved unchanged. Chain rewriting is not supported.

## Storage

Diff chains are workspace-scoped, so they live on `workspace.db`:

```sql
CREATE TABLE IF NOT EXISTS diff_chains (
    id TEXT PRIMARY KEY,                    -- ULID
    file_path TEXT NOT NULL,
    language TEXT NOT NULL,
    selection_start INTEGER NOT NULL,
    selection_end INTEGER NOT NULL,
    original_text_enc BLOB NOT NULL,        -- ChaCha20-Poly1305
    steps_enc BLOB NOT NULL,                -- ChaCha20-Poly1305(JSON of Vec<DiffChainStep>)
    final_state TEXT NOT NULL,              -- 'applied' | 'cancelled' | 'open'
    final_meta_json TEXT,                   -- JSON with applied_step / cancelled_at / reason
    parent_chain_id TEXT,                   -- when a chain was forked from another
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_chains_file ON diff_chains(file_path);
CREATE INDEX IF NOT EXISTS idx_chains_updated ON diff_chains(updated_at);
CREATE INDEX IF NOT EXISTS idx_chains_parent ON diff_chains(parent_chain_id);

CREATE TABLE IF NOT EXISTS diff_chain_recaps (
    id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL,
    last_step_index INTEGER NOT NULL,       -- idempotency cursor
    generated_at TEXT NOT NULL,
    generator_kind TEXT NOT NULL,
    generator_provider TEXT,
    generator_model TEXT,
    headline_enc BLOB NOT NULL,
    body_enc BLOB NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (chain_id) REFERENCES diff_chains(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_chainrecaps_step ON diff_chain_recaps(chain_id, last_step_index);
```

Encryption uses the existing `WorkspaceStore` ChaCha20-Poly1305 key (machine + workspace bound). No new key derivation. Selections and diffs can leak intent and source — encrypted at rest is the right default.

**Storage-rule check** (per `CLAUDE.md`):
- ✅ Project secrets / chain content → `WorkspaceStore` (`<workspace>/.vibecli/workspace.db`).
- ✅ No plaintext writes.

## RPC contract

Standard cross-cutting routes apply with `kind: "diff_chain"`. Plus diffcomplete-specific:

### `POST /v1/diffcomplete/chains` — autosave a chain

Called by the modal each time a step is appended (regenerate succeeds) or the final state changes.

```jsonc
// request
{
  "chain_id": "01HCH...",          // omit on first call; daemon assigns
  "file_path": "src/auth.rs",
  "language": "rust",
  "selection_start": 12,
  "selection_end": 28,
  "original_text": "fn validate(...) { ... }",
  "step": {
    "index": 2,
    "instruction": "rename x to count",
    "refinement": "tighten the error path",
    "additional_files": [],
    "diff": "--- a/...",
    "provider": "anthropic",
    "model": "claude-opus-4-7",
    "tokens_input": 1200,
    "tokens_output": 240
  },
  "final_state": "open"   // or "applied" / "cancelled" with extra fields
}

// response
{ "chain_id": "01HCH...", "step_index": 2 }
```

The endpoint is idempotent on `(chain_id, step.index)`. A repeated POST with the same step index returns the existing record.

### `GET /v1/diffcomplete/chains?file_path=&limit=`

List chains in the workspace, newest-first. Default `limit=20`.

### `GET /v1/diffcomplete/chains/:id`

Full chain detail, decrypted.

### `POST /v1/recap` (kind=diff_chain)

Generates a chain recap. `force` and `generator` semantics same as session.

### `POST /v1/resume` (kind=diff_chain)

```jsonc
// request
{
  "from_recap_id": "01HMRECAP...",
  "kind": "diff_chain",
  "from_diff_index": null,         // overrides recap.resume_hint.from_diff_index
  "seed_refinement": null,
  "client": "vibeui"
}

// response
{
  "handle": "01HRESUME...",
  "resumed_chain_id": "01HCH_NEW...",   // a NEW chain (forking is the only mode)
  "parent_chain_id": "01HCH...",
  "primed_step_index": 2,                // modal opens at this step in review phase
  "ready": true
}
```

The frontend translates this into: open `DiffCompleteModal` with `phase = 'review'`, `lastDiff = step.diff`, `instruction = step.instruction`, and (if `seed_refinement` set) pre-filled refinement input.

## Per-surface UX

### vibeui — DiffCompleteModal

This is the primary surface.

- **Modal header gains a chain breadcrumb** when chain length > 1: `Step 2 of 3 · auth.rs L12-28`. The breadcrumb is read-only — clicking a step opens a side panel listing all steps in the chain (each row: instruction, refinement, generated_at, "View diff" link).
- **Modal footer gains a "History" link** when there are prior chains for the same `(file_path, selection_start, selection_end)`. Clicking opens the chain history panel.
- **Autosave on every successful regenerate** — `POST /v1/diffcomplete/chains` after each `runGenerate` call. If the daemon is unreachable, autosave is queued and retried; the modal does not block on persistence.
- **On Apply:** existing apply flow runs (write to file). Then `final_state = applied`, recap is generated heuristically.
- **On Cancel / X-close with chain length ≥ 2:** `final_state = cancelled (UserCancel|ModalClosed)`, recap generated heuristically.
- **On reopening a previously-cancelled chain:** the side panel offers "Resume" (forks a new chain). Original chain is preserved.
- **No keyboard accept-style affordance is added.** The existing Cmd+. trigger remains the explicit-action gateway. The chain breadcrumb does not have hotkeys to "step forward / back" — that would creep toward Element-4 territory.

### vibeui — Chain History panel

A new panel reachable via:
- The modal footer "History" link (filtered to current selection)
- The Activity Bar (icon: stacked-papers, label "Diff Chains") (filtered to current file or workspace)

Layout: a list of chain cards, each showing:
- File path + selection range
- Chain length, final state, updated_at
- Recap headline (if a recap exists)
- "View" → opens a read-only side panel with the full chain
- "Resume from last step" → calls `/v1/resume`

The panel **never** auto-opens. **Never** decorates the editor with chain markers.

### vibecli REPL / TUI

Diffcomplete is desktop-only today (it's an editor flow). The CLI gets a *read-only* surface:

- **`/diff-chains [--file PATH]`** — list chains in the current workspace.
- **`/diff-chain show <chain_id>`** — print all steps inline (instruction, refinement, diff).
- **`/diff-chain recap <chain_id>`** — print recap.
- **No CLI resume.** Resuming requires the modal UI; the CLI prints "Open in vibeui to resume." This is a deliberate restriction — running a "regenerate from refinement" loop in a terminal would push toward an inline-suggestion shape that we want to stay distant from.

### vibemobile

- **JobRecapView (from `02-job.md`) extends to render `kind: diff_chain` recaps** via the same shape. Tap a chain recap → readonly view of steps. **No mobile resume** — tapping "Continue" hands off to desktop via `/mobile/sessions`, exactly as session resume does.
- This is intentionally minimal. Mobile is a status / handoff surface for diffcomplete, not a composition surface.

### vibewatch

- **Chain recaps are not surfaced on watch by default.** They're file-scoped, code-heavy, and headline-only would lose the value.
- An advanced setting in the desktop Settings panel ("Surface diff-chain recaps on watch") can opt in. Off by default.

## Heuristic recap algorithm (chain-specific)

1. **Headline:** `<verb> <file>:<lines> — <step-0 instruction trimmed>`.
   - verb: "Refined" if applied, "Drafted" if cancelled.
   - Example: `Refined auth.rs L12-28 — rename x to count`.
2. **Bullets** (3–5):
   - Step count + final state: `3 iterations, applied step 3`.
   - Refinement summary: `Refinements: tighten error path → add doc comment`. (joined with ` → `, max 3.)
   - Provider/model: `claude-opus-4-7 via anthropic`.
   - Token total: `~3.4k input, ~600 output across the chain`.
   - File context: `Additional files: 2 (router.rs, mod.rs)` if `additional_files` was used in any step.
3. **next_actions:** empty by default. (Diffcomplete is one selection at a time; "next action" is genuinely unclear without LLM judgment.) If LLM generator is used, it can fill `next_actions` from chain content.
4. **artifacts:**
   - One File artifact for `file_path`.
   - One Diff artifact per step (label = `step N`, locator = `chain:<id>#step<N>`).
5. **resume_hint:** `from_diff_index = applied_step` if applied, else `steps.len() - 1`. `seed_refinement = None`.

## Failure modes

| Failure | Behavior |
|---|---|
| Daemon down during autosave | Queue locally in component state; retry every 30s. Modal continues to function. |
| Workspace key derivation fails (workspace renamed?) | Old chains unreadable; modal logs a warning but starts a fresh chain. (Same trade-off as `WorkspaceStore` today.) |
| Resume of a chain whose underlying file/selection no longer matches | Show a warning banner: "File or selection has changed since this chain was created. Resume anyway?" — explicit user confirm required. |
| Recap generation fails | Chain remains intact; recap is absent (`headline = null` UI shows "(no recap yet, click to generate)"). |

## Slicing plan

| Slice | What | Surfaces | Patent re-audit |
|---|---|---|---|
| **D1.1** | `diff_chains` table on `workspace.db` + `DiffChain` types + autosave RPC | daemon + Tauri command | Required: confirm autosave doesn't run on idle/typing. |
| **D1.2** | `DiffCompleteModal` autosave on regenerate / final-state | vibeui | Required: confirm no editor-buffer decoration added. |
| **D1.3** | Chain History panel (read-only) | vibeui | Required: confirm panel does not auto-open. |
| **D1.4** | `diff_chain_recaps` table + heuristic generator + `/v1/recap` (kind=diff_chain) | daemon | Required: confirm recap is not generated on a timer. |
| **D1.5** | "Resume from last step" button in chain history; `/v1/resume` (kind=diff_chain); modal opens at primed step | vibeui + daemon | Required: confirm resume requires explicit click and uses existing modal-Apply gesture. |
| **D1.6** | REPL `/diff-chains` and `/diff-chain show` (read-only) | vibecli | Required: confirm no CLI loop affordance added. |
| **D2.x** | LLM recap generator option | daemon + vibeui Settings | Required: confirm generator is opt-in per call. |
| **M1.3** | Mobile read-only chain recap view via `/v1/recap` shared shape | vibemobile | (Read-only — minimal patent surface.) |

Each slice is required to commit a one-line note in its PR description: `Patent re-audit: PASS (elements 1–5 unchanged)`. If any element is touched, the audit is the merge blocker.

## Open questions

1. **Should "Save & close" be a distinct action from Cancel?** Cancel today discards. A new "Save & close" preserves the chain in `final_state: open` for later resume, without applying. Probably yes — the chain has value even unapplied. To confirm in v1.
2. **Chain TTL / cleanup.** Do `final_state: cancelled` chains stick around forever? Suggest: keep 90 days, then garbage-collect. Configurable in Settings → Diffcomplete.
3. **Cross-file chains.** A user might naturally regenerate after editing surrounding files. The chain is still anchored to the original `(file, selection)`. We do *not* try to track edits to surrounding files — the chain is a record of model output for one selection. Confirmed: out of scope to chase cross-file invalidation.
4. **Same selection, two chains in flight.** Allowed. The chain history panel sorts by `updated_at`; both are visible.
