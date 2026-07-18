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

**Phases 0–2 done.** Core (`model` / `convert` / `store` /
`metrics::trigger_coverage`) needs no LLM; the `llm` feature adds
`extract::{SequentialExtractor, ParallelExtractor}` and `metrics::eval::{
EvalTask, Grader, target_evolvability, extraction_efficacy}`. CLI:
`skilllensai convert` / `report` (no key). `cargo test -p skilllensai-rs` green
(23 tests + golden parse over ~710 shipped skills). Wired into the VibeCody
daemon via `vibecli/vibecli-cli/src/skillforge_index.rs` (Phase 3 —
`/v1/skilllens/*` routes + `/health.skillforge` + watch mirror), into VibeCoder
via `SkillForgePanel.tsx` (Phase 4 — Catalog / Lens), and out to every client
(Phase 5 — Flutter/Watch/Wear read-only + VS Code/Agent SDK full surface).
See `notes/skillforge/05 — Implementation Roadmap.md`.

## License

MIT.
