# @vibecody/ui-shared

UI that both Tauri shells use — **VibeDesk** and **VibeAIChat**.

```
src/settings/   Providers · Appearance · Account screens (+ their CSS)
src/hooks/      useTheme, useProviderSettings
src/theme/      theme definitions
src/lib/        thinking.ts — splits model reasoning out of a turn
```

## Why it exists

These files were copied between the two apps, and the copies drifted. A fix to
one left the other rendering raw `<thinking>` markup on screen. Anything two
shells would otherwise keep in sync by hand belongs here.

## How it is consumed

As **source**, not a built package. Each host app aliases it:

```ts
// vite.config.ts
resolve: { alias: { "@vibe/shared": resolve(__dirname, "../packages/vibe-ui-shared/src") } }
```

```jsonc
// tsconfig.json
"paths":   { "@vibe/shared/*": ["../packages/vibe-ui-shared/src/*"] },
"include": ["src", "../packages/vibe-ui-shared/src"]
```

No build step, no `dist`, no version to bump — each app compiles these files
with its own toolchain, and `tsc --noEmit` in either app type-checks them.

## Requirements on the host app

1. Import the design-system tokens **before** `settings.css`; it uses those
   variables and nothing else.
2. Register the settings Tauri commands its screens invoke — `setting_get`,
   `setting_set`, `provider_key_set/has/list/delete`, `provider_config_set`,
   `oauth_client_set/has`. Both shells have them in `src-tauri/src/settings.rs`.

Miss either and the screens compile, render, and quietly do nothing — so check
both when wiring a new shell.
