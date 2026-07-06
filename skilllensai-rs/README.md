# skilllensai-rs

> Analyse & measure agent-skill utility — a Rust port of
> [TuringWorks/SkillLens](https://github.com/TuringWorks/SkillLens).

Normalises agent runs into a unified **Trajectory** schema, extracts candidate
skills (sequential baseline / parallel mode-merge), and scores skill utility via
**Extraction Efficacy** and **Target Evolvability**.

Part of the **SkillForge** workspace (with [`skilloptai-rs`](../skilloptai-rs)).
Provider-agnostic: LLM access goes through the crate-local `SkillLlm` trait, so
the crate depends on neither `vibecli` nor `vibe-ai`.

Design & roadmap: `notes/skillforge/` (start at `SkillForge — MOC.md`).

## Status

**Phase 0 — scaffold.** Module skeleton compiles; concrete logic lands in
Phase 1. See `notes/skillforge/05 — Implementation Roadmap.md`.

## License

MIT.
