## Summary

<!-- Brief description of what this PR does and why. -->

## Change surface

<!-- Which parts of the Product Matrix does this touch? Check all that apply. -->

- [ ] VibeCLI (daemon / TUI / REPL)
- [ ] VibeCoder (desktop editor)
- [ ] VibeApp (secondary Tauri shell)
- [ ] VibeMobile (Flutter)
- [ ] VibeCodyWatch (watchOS)
- [ ] VibeCodyWear (Wear OS)
- [ ] Watch companions (iOS / Android)
- [ ] VS Code extension
- [ ] JetBrains plugin
- [ ] Neovim plugin
- [ ] Agent SDK
- [ ] vibe-indexer

## Checklist

<!-- Delete items that don't apply. -->

- [ ] Tested locally (`make test` or `cargo test`)
- [ ] Design-system classes used for any new UI (`panel-container`, `panel-btn`, etc.)
- [ ] Provider-agnostic: no hard-coded provider/model in panels or Tauri commands
- [ ] Sensitive values stored in ProfileStore / WorkspaceStore (never plaintext)
- [ ] Feature is zero-config by default (sane defaults, no env-var-only setup)
- [ ] Required config surfaced in daemon startup log + `/health`
- [ ] Cross-referenced: FEATURE-MATRIX.md / FEATURE-REFERENCE.md / FIT-GAP-ANALYSIS.md updated if this closes a gap
- [ ] BDD tests added or updated (`*.bdd.test.tsx` for panels, `*.feature` for daemon routes)
- [ ] Documentation updated (`docs/` or AGENTS.md if the Change-Surface Cookbook changed)

## Related

<!-- Links to issues, discussions, or other PRs. -->
