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

**Phase 0 — scaffold.** Re-exports both member crates. Logic lands in Phases 1–2.

## License

MIT.
