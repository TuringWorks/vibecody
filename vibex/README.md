# VibeX

> Task-first, conversation-driven companion app in the VibeCody ecosystem.
> Tauri 2 + React/TS. Shares all VibeCLI Rust crates and the VibeUI design system; owns only its React tree + a thin daemon bridge.

VibeX is **not** VibeUI v2. It is the fast path — type a task, watch it happen — built to mirror the Codex desktop UX: a three-column shell (left project/chat nav · center conversation · right Environment inspector) with all run controls inline in the composer.

Spec & plan live in the PDM vault:
`obsidian/.../Projects/VibeCody/pdm/` → `06` spec · `07` Codex-alignment critique · `08` build plan · `09` task backlog.

## Locked decisions

1. **No Cmd+K / no inline-completion.** AI edits go through conversation+Review or the existing **⌘. `DiffCompleteModal`** surface only (patent-distance). Enforced by `scripts/check-no-inline-edit.mjs` (task VX-013).
2. **Worktree-native** environments; sandbox tiers are opt-in escalation.
3. The **VibeCLI daemon is the source of truth** — VibeX talks to it over HTTP/SSE (`src-tauri/src/commands.rs`), never re-implementing agent logic.

## Develop

```bash
cd vibex
npm install
npm run tauri:dev          # opens the VibeX window (needs `vibecli serve` running)
npm run lint:no-inline-edit # VX-013 gate
npm run build              # tsc + vite production build
```

Dev server runs on **:1422** (vibeui=1420, vibeapp=1421). The Rust crate is `vibex` in the workspace; `cargo check -p vibex`.

## Status

Phase 0 (scaffold) + Phase 1 shell are in place: three-column layout, project nav rail, session stream with structured tool-use blocks, composer with provider/approval/reasoning pills, quick-action drawer (Files/Terminal first), Environment inspector, daemon status banner. Live data wiring (`/api/tasks`, worktree-per-task, streaming) is the next slice — see `pdm/09` backlog `VX-105/111/112/113`.
