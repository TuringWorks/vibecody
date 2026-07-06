# skillforgeai-rs

> The **SkillForge** facade — one crate that re-exports
> [`skilllensai-rs`](../skilllensai-rs) (analyse skills) and
> [`skilloptai-rs`](../skilloptai-rs) (train skills).

```rust
use skillforgeai::{lens, opt};
```

They compose: **lens picks/measures → opt optimises → lens re-measures.**

Design & roadmap: `notes/skillforge/` (start at `SkillForge — MOC.md`).

## Status

**Phases 0–5 done.** Re-exports both member crates (`lens` = `skilllensai`,
`opt` = `skilloptai`), complete through Phase 2; the VibeCody daemon bridge
(`vibecli/vibecli-cli/src/skillforge_index.rs`, Phase 3) adapts both onto
`vibe_ai::AIProvider` and exposes `/v1/skilllens/*` + `/v1/skillopt/*`; the
VibeUI panel (`SkillForgePanel.tsx`, Phase 4) drives them from the desktop;
the client fan-out (Phase 5) ships read-only catalogue/status to
Flutter/Watch/Wear and the full surface to VS Code + the Agent SDK. See
`notes/skillforge/05 — Implementation Roadmap.md`.

## License

MIT.
