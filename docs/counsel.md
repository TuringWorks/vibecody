---
layout: page
title: Counsel — Multi-LLM Debate
permalink: /counsel/
---

Counsel is the panel for running structured debates between multiple LLMs on a single topic, then synthesizing the result. It's modeled after a deliberative council: each participant gets a role (Expert, Skeptic, Devil's Advocate, Pragmatist, etc.), responses are recorded round-by-round, and a moderator can synthesize a final answer.

This page documents the desktop panel. Counsel is currently desktop-only; mobile / watch / IDE plugins don't expose it.

---

## How a session works

1. **Setup.** You add 2-N participants — each is a `(provider, model, role, optional persona)` tuple. Pick one as the moderator.
2. **Topic.** Enter a free-form question or proposition for the council to debate.
3. **Round 1.** Click Run Round — every non-moderator participant generates a response in parallel. Responses appear in the round's card with the participant's role badge color-coded.
4. **(Optional) Inject a message.** Type into the inject box and the next round will include your interjection as a system message — useful for redirecting a stuck debate or asking a clarifying question.
5. **(Optional) Vote.** Each response has +/- buttons. Votes don't change the debate flow; they're a UI affordance for highlighting which response best moved the discussion forward.
6. **(Optional) Update participants between rounds.** Swap a participant's provider/model without losing prior rounds — useful for "what would Claude say if GPT-4 had been here for the first round?"
7. **Run more rounds** as needed — typically 2-4 produce useful divergence.
8. **Synthesize.** The moderator reads all rounds and writes a synthesis — the panel's final answer.

---

## Session storage

Sessions live in `~/.vibecoder/counsel-sessions.json` as a JSON array. Each session has:

- `id` — UUID
- `topic` — the debate prompt
- `participants` — provider/model/role list
- `rounds` — array of `{round_number, responses[], user_interjection?}`
- `moderator_index` — which participant synthesizes
- `status` — `Idle` / `Deliberating` / `Synthesized`
- `synthesis` — final answer (only after synthesize)

The file is plain JSON — the topic, responses, and synthesis are stored verbatim. **It's not encrypted.** If you debate sensitive topics, treat the file as the same trust level as a Notes or Docs file on disk.

---

## Roles

Roles are visual + persona affordances; they're prepended to the participant's prompt as "You are the Skeptic — your job is to find weaknesses in arguments." Role names and their suggested behavior:

| Role | Persona |
|---|---|
| Expert | The default — answer with depth and citations |
| Devil's Advocate | Argue against whatever the prior round concluded |
| Skeptic | Probe for weaknesses, demand evidence |
| Creative | Propose unconventional approaches |
| Pragmatist | Focus on what can be implemented this week |
| Researcher | Cite sources, prefer empirical data |
| Custom | Use the `persona` field to write your own role description |

You can mix roles freely — a 4-participant session with Expert / Skeptic / Creative / Pragmatist tends to produce the most divergent debates. Same provider in different roles is fine; it's the role prompt that drives behavior, not the model.

---

## Inject + vote — when to use

**Inject** is the user's voice in the debate. The text becomes a system message visible to every participant in the next round. Good prompts:

- "Wait — round 2 missed the cost dimension. Address it."
- "Let's narrow scope to read-heavy workloads only."
- "The Skeptic's last objection is the strongest — others, respond directly."

**Vote** is a soft signal. Up-votes don't change which participants get called next round; they're a way to mark which responses you'll quote in the eventual synthesis prompt. The vote count appears on each response card; the moderator's synthesize prompt sees the vote totals and is biased toward higher-voted responses.

---

## /health declaration

`features.counsel`:

```json
{
  "available": true,
  "transport": "tauri-desktop",
  "requires": "providers.configured_count >= 2 (for diverse debate)",
  "store_path": "~/.vibecoder/counsel-sessions.json"
}
```

`available` follows the providers count: with 0 providers configured, the panel still renders but every participant card shows "Provider not configured". With 1 provider you can run a session but every participant uses the same model — fine for testing, useless for actual debate. The recommended floor is 2+ different providers.

---

## Observability

Backend operations emit structured tracing events under `vibecody::counsel`:

```bash
RUST_LOG=vibecody::counsel=info vibecli serve
```

Events:

```
INFO vibecody::counsel: counsel.session.create
  session_id=01HXYZ... participant_count=4 moderator_idx=0 providers=["claude", "openai", "gemini", "ollama"]

INFO vibecody::counsel: counsel.round.start
  session_id=01HXYZ... round=2 participant_count=4

INFO vibecody::counsel: counsel.round.complete
  session_id=01HXYZ... round=2 response_count=4 elapsed_ms=14820

INFO vibecody::counsel: counsel.session.delete
  session_id=01HXYZ... remaining=11
```

**Topic, persona, response, and synthesis text are NEVER logged** — only counts, ids, and timing. Operator dashboards aggregate these to spot stuck sessions (rounds taking >60s) and provider failures (round_complete without all responses).

---

## Accessibility

- Session list items use `role="button" tabIndex={0}` AND respond to Enter / Space — the previous implementation had `role="button"` without an `onKeyDown`, which announced as a button to AT but didn't activate.
- Verbose `aria-label` on session list rows: "Open session: <topic> — N rounds, M participants".
- Delete buttons gain an `aria-label` carrying the topic.
- Errors render with `role="alert"` `aria-live="assertive"` so they're announced immediately.
- Participant cards use real `<input type="checkbox">` for selection — no fake-checkbox `role` workaround needed.

---

## Cross-client behaviour

| Client | Counsel |
|---|---|
| **VibeCoder / VibeApp** | Full panel |
| **VibeMobile** | None — debates are inherently a side-by-side comparison UX |
| **VibeWatch** | None |
| **IDE plugins** | None |

Sessions are stored on the daemon machine only; no cross-device sync today. If you start a session on the desktop and want to view it from a phone later, that's a future feature.

---

## Troubleshooting

### "Round failed: provider not configured"

One or more participants reference a provider with no API key. Open Settings → Providers and add the missing key, OR edit the participant's provider via the dropdown to a configured one. The error surfaces inline with `role="alert"` so AT users hear it.

### "Synthesis is just one participant's answer copy-pasted"

The moderator was a small / unaligned model. Try setting the moderator to your strongest model (Claude Sonnet, GPT-4o, etc.) — synthesis is a hard task and benefits from a capable summarizer.

### "Round 3 is identical to round 2"

The participants converged. This is a real outcome; the debate is over. Synthesize and start a new session with sharper roles (more contrarian Devil's Advocate, more skeptical Skeptic) if you want continued divergence.

### "Sessions disappeared after restart"

`~/.vibecoder/counsel-sessions.json` was deleted or corrupted. Sessions aren't backed up to the daemon — if the file goes, the sessions go. Consider a periodic backup to your dotfiles repo.

---

## Related

- **Source:** `vibecoder/src/components/CounselPanel.tsx` (~545 LOC) · backend in `vibecoder/src-tauri/src/commands.rs` (`counsel_create_session`, `counsel_run_round`, `counsel_synthesize`, `counsel_delete_session`, `counsel_inject_message`, `counsel_vote`, `counsel_update_participant`)
- **Counsel runtime:** `vibecli/vibecli-cli/src/counsel.rs` — session model + `add_round`
- **Arena Mode:** [`docs/arena`](./arena.md) — for blind 1-vs-1 comparisons (different use case)
- **SuperBrain:** for parallel multi-provider runs with judge-as-aggregator (different use case)
