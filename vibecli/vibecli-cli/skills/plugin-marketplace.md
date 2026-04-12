# Plugin Marketplace

Discovery, metadata browsing, and one-click installation of WASM-based VibeUI extensions. Extends the `vibe-extensions` WASM system.

## Categories
`language-support` | `linting` | `formatting` | `debugging` | `git` | `ai` | `theme` | `productivity` | `testing`

## Built-in Plugins (demo catalogue)
| Plugin | Category | Downloads | Rating |
|---|---|---|---|
| GitLens | git | 211k | ★★★★★ |
| Rust Extras | language-support | 142k | ★★★★★ |
| Prettier Integration | formatting | 95k | ★★★★★ |
| AI Doc Generator | ai | 34k | ★★★★☆ |

## Key Types
- **PluginManifest** — id, name, semver, permissions, sha256, rating, downloads
- **PluginRegistry** — searchable catalogue with `by_category()` and `search()`
- **InstallManager** — install/uninstall/enable with `check_updates()`

## Permissions
Plugins may request: `ReadFiles`, `WriteFiles`, `NetworkAccess`, `ProcessSpawn`, `ClipboardAccess`, `NotificationSend`

High-privilege plugins (ProcessSpawn / NetworkAccess) are flagged.

## Commands
- `/plugin list [category]` — browse available plugins
- `/plugin search <query>` — search by name/description/tags
- `/plugin install <id>` — one-click install
- `/plugin uninstall <id>` — remove plugin
- `/plugin updates` — check for outdated installed plugins

## Examples
```
/plugin search git
# GitLens v2.1.0 — Inline blame, commit history (★★★★★, 211k downloads)

/plugin install vibe-gitlens
# ✓ GitLens installed and enabled
```
