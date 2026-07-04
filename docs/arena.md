---
layout: page
title: Multi-Model Arena
permalink: /arena/
---

Arena is the panel for blind A/B model comparisons. You configure two providers and one prompt; the panel sends the prompt to both in parallel, hides which side is which, and asks you to vote. The leaderboard tracks per-provider win rates over time.

This page documents the user-facing surface. Arena is a desktop-only feature (VibeUI / VibeApp) — mobile and watch clients don't ship it.

---

## How a battle works

1. **Pick A and B.** Each side has a provider dropdown (the registered providers) and a model field (autocomplete from the provider's known model list).
2. **Enter a prompt** and click **Battle** (or press `⌘/Ctrl+Enter` from the textarea).
3. **Both providers run in parallel.** The daemon's `compare_models` command joins them via `tokio::join!`, so total wall-clock latency is `max(provider_a, provider_b)`, not the sum.
4. **Identities are randomized.** Internally, the panel flips a coin and may swap which side is labelled "A" and which is "B" — so position bias is broken.
5. **Both responses render side-by-side**, each as a `BlindResponseCard` with `(identity hidden)` until you vote.
6. **You vote** A / B / Tie / Both bad.
7. **Reveal panel** unveils which model was on each side, plus latency and token counts. The reveal is announced to AT users via `aria-live="polite"`.
8. **Vote saved** to `~/.vibeui/arena-votes.json` and the leaderboard refreshes.
9. **Send winner to Chat** (optional) injects the winning response into the active chat tab via the `vibeui:inject-context` event.

---

## Vote storage

Votes are stored in `~/.vibeui/arena-votes.json` as a plain JSON array. They're not encrypted — the file contains no API keys, only:

- timestamp
- prompt
- both `(provider, model)` pairs
- winner (`a`, `b`, `tie`, `both_bad`)

Each vote has an entry per side. For per-provider stats, votes where both sides are the same provider self-cancel for that provider's win/loss columns (the leaderboard is informative when comparing across providers, less so within one).

If you want to reset the leaderboard, delete `~/.vibeui/arena-votes.json` — there's no in-app "Reset" button by design (this is a write-once history).

---

## Leaderboard

The Leaderboard table renders only when there's at least one vote. Columns:

| Column | Meaning |
|---|---|
| Provider | Provider name (model not aggregated — same provider, different models, share rows) |
| Wins | Times this provider was the user's vote |
| Losses | Times the opposing provider was voted winner |
| Ties | Tie votes count for both sides as a tie |
| Total | Wins + losses + ties (excludes "both bad") |
| Win Rate | Wins / Total — null-safe; sorted descending |

`both_bad` votes are intentionally excluded from the per-provider score columns: a "both bad" outcome doesn't tell us which provider was *less* bad.

---

## Empty / error states

- **No prompt** → Battle button is disabled.
- **Provider not configured** → both `compare_models` and the per-side response card surface an inline error. The panel-level alert (`role="alert"`) shows the daemon's reply verbatim; the inline card highlights its border in red and shows `error: <message>`.
- **Provider returns nothing** → "(empty response)" placeholder rather than a blank card, so it's obvious the side ran.
- **Vote save fails** → reveal still renders; a non-blocking inline alert (`Couldn't save your vote — the leaderboard won't reflect this battle. Try voting again.`) appears under the vote group. The error does *not* block the reveal because the user already cast their vote — losing it from history is just a silent data-only failure.

---

## /health declaration

`features.arena` declares the surface:

```json
{
  "available": true,
  "transport": "tauri-desktop",
  "requires": "providers.configured_count >= 2 (for non-trivial battles)",
  "votes_path": "~/.vibeui/arena-votes.json"
}
```

Arena is technically usable with 1 configured provider (you can battle two of its models against each other), but is most useful with ≥2. The `requires` string is informational — clients gating UI on this should check `providers.configured_count >= 1`, not 2.

---

## Observability

Backend operations emit structured tracing events under `vibecody::arena`:

```bash
RUST_LOG=vibecody::arena=info vibecli serve
```

Events:

```
INFO vibecody::arena: arena.battle.start
  provider_a=ollama model_a=llama3 provider_b=openai model_b=gpt-4 prompt_len=42

INFO vibecody::arena: arena.battle.complete
  provider_a=ollama provider_b=openai a_ok=true b_ok=true elapsed_ms=820

INFO vibecody::arena: arena.vote.save
  winner=a provider_a=ollama provider_b=openai total_votes=14
```

Prompts are **never** logged — only the length. Response content is never logged.

---

## Accessibility

- The vote buttons live in `role="group" aria-label="Cast your vote"` and each carries a verbose `aria-label` ("Vote: A is better") so screen readers don't have to infer from "A is better" alone.
- The reveal panel is `role="region" aria-label="Battle reveal" aria-live="polite"` so the winner announcement is read out without yanking focus.
- The error alert and vote-save error use `role="alert"` so they're announced immediately when they appear.
- All interactive elements are keyboard-reachable; no focus traps.

---

## Cross-client behaviour

| Client | Arena |
|---|---|
| **VibeUI / VibeApp** | ✅ |
| **VibeMobile** | ❌ — no plans (battles are inherently a side-by-side desktop UX) |
| **VibeWatch** | ❌ |
| **IDE plugins** | ❌ |
| **Agent SDK** | n/a — the SDK is for invoking models, not comparing them |

---

## Troubleshooting

### "Battle returns immediately with two errors"

Both providers are unconfigured. Open Settings → Providers, add at least one API key, and restart the panel. Provider availability is reported in `/health.providers.configured_count`.

### "Same model wins every time"

Position bias: the panel randomizes which side gets which model on every battle, so this would manifest as winning whichever side is currently *labelled* A or B regardless of model. If you see that, file an issue with a screenshot.

### "Send winner to Chat does nothing"

The button dispatches a `vibeui:inject-context` window event. The Chat panel listens for this event when it's mounted and a tab is active. If no tab is active (e.g., Chat panel is closed), the event has no effect — switch to Chat first, then click Send winner to Chat.

---

## Related

- **Providers:** [`docs/providers/`](./providers/) — what counts as a configured provider
- **Source:** `vibeui/src/components/ArenaPanel.tsx` (399 LOC) · backend in `vibeui/src-tauri/src/commands.rs` (`compare_models`, `save_arena_vote`, `get_arena_history`)
- **Tests:** `vibeui/src/components/__tests__/ArenaPanel.bdd.test.tsx`
