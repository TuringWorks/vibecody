# Config Layers

Multi-source layered configuration with well-defined precedence, typed values, Cleared semantics, and origin tracking. Also provides three-level JSON deep-merge (user → project → local).

## When to Use
- Merging CLI flags, env vars, project settings, and user defaults
- Tracing which layer set a given config key
- Clearing a key at a higher priority layer
- Deep-merging JSON config files across user/project/local tiers

## Priority Order (lowest → highest)
`System < Project < User < Environment < Cli`

Higher-priority layers win. `Cleared` removes a key set at a lower level.

## Three-Level JSON Merge
`LayeredConfig` merges `~/.vibecli/config.toml` (user) → workspace `.vibecli/settings.json` (project) → `.vibecli/settings.local.json` (local). Deep-merge: objects merge recursively, arrays are replaced.

## Commands
- `/config show` — Display resolved config with origins
- `/config get <key>` — Show value and which layer set it
- `/config set <layer> <key> <value>` — Set a key in a specific layer
- `/config clear <key>` — Remove a key (all layers)
- `/config layers` — List all active layers and their sources
- `/config validate` — Check all layers for schema errors

## Examples
```
/config show
# model: "claude-opus" (from: project)
# timeout: 30 (from: system)

/config get model
# → "claude-opus" [origin: project]

/config set cli model custom-model
# CLI layer now overrides all others for "model"

/config clear timeout
# Removes "timeout" — higher-priority Cleared masks system value
```
