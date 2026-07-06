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

**Phase 0 — scaffold.** Module skeleton compiles; the epoch loop lands in
Phase 2. See `notes/skillforge/05 — Implementation Roadmap.md`.

## License

MIT.
