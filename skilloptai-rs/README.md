# skilloptai-rs

> Train agent-skill documents — a Rust port of
> [TuringWorks/SkillOpt](https://github.com/TuringWorks/SkillOpt).

Treats a skill markdown doc as the trainable state of a frozen agent. Scored
**rollouts** drive bounded **add/delete/replace** edits, accepted **only when a
held-out validation score strictly improves** — epoch after epoch, with a
rejected-edit buffer and a textual learning rate for stability. Output is a
`best_skill.md` deployed with **zero inference-time overhead**.

Part of the **SkillForge** workspace. Depends on
[`skilllensai-rs`](../skilllensai-rs) for the shared `Trajectory` schema and the
metrics its validation gate calls. Provider-agnostic via the `SkillLlm` seam.

Design & roadmap: `notes/skillforge/` (start at `SkillForge — MOC.md`).

## Status

**Phases 0–2 done.** `edit` / `buffer` / `report` compile without the LLM
feature; the `llm` feature adds `env` (`Env` + `StaticEnv` + seeded split),
`rollout`, `propose`, `gate`, and `trainer::train` (strict gate, rejected-edit
buffer, textual-LR, patience early-stop, fully seeded). CLI: `skilloptai train`
/ `propose` over an OpenAI-compatible `SkillLlm`. `cargo test -p skilloptai-rs`
green (30 tests, incl. a deterministic val-curve test + a strict-gate
rejection test). Wired into the VibeCody daemon via
`vibecli/vibecli-cli/src/skillforge_index.rs` (Phase 3 —
`/v1/skillopt/train` + `status`/`cancel`/`promote` + `RepoAgentEnv` over the
catalog), into VibeUI via `SkillForgePanel.tsx` (Phase 4 — Optimize tab with
live val-curve + guarded Promote), and out to every client (Phase 5 —
Flutter/Watch/Wear read-only status + VS Code/Agent SDK full surface). Next:
per-epoch SSE streaming + `RepoAgentEnv` from real agent-job history. See
`notes/skillforge/05 — Implementation Roadmap.md`.

## License

MIT.
