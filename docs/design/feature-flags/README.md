# Feature Flags — Design Index

**Status:** Draft · 2026-04-30
**Scope:** vibecli daemon (Rust) + vibecoder (Tauri/React) + vibeapp + vibemobile (Flutter) + vibewatch (SwiftUI / Kotlin Compose) + vscode-extension + jetbrains-plugin + neovim-plugin + agent-sdk
**Owner:** TBD
**Related docs:** [AGENTS.md → Zero-Config First](../../../AGENTS.md), [vibecoder/design-system/README.md](../../../vibecoder/design-system/README.md), [docs/design/sandbox-tiers/README.md](../sandbox-tiers/README.md), [docs/design/recap-resume/README.md](../recap-resume/README.md), [docs/design/rl-os/README.md](../rl-os/README.md)

---

## TL;DR

VibeCody currently exposes **285 React panels** across **41 composites** through a single side-rail navigation. Of those, the most recent production-readiness audit identified **only ~7 surfaces as GA-quality** (Diffcomplete, Memory, Settings, Recap-heuristic, BackgroundJobs, ChatTabManager, SessionsList, plus the Mobile/Watch recap viewers). The remaining ~278 panels range from "real but rough" through "scaffold/experimental" through "pure illustration with `// SLICE_N_MOCK` markers".

This doc specifies a **feature-flag system** that lets us:

1. Hide experimental and developer-only surfaces from the default user experience by tier (GA / Beta / Experimental / Internal).
2. Give users a **Settings → Features** matrix to opt in to Beta and Experimental surfaces themselves, with clear visual labelling and a one-click "go back to defaults" affordance.
3. Keep the daemon as the **single source of truth** for which flags exist and what their defaults are, with all 13 clients (desktop, mobile, watch, IDE plugins, SDK) reading from the same `/v1/flags` endpoint and falling back gracefully when offline.
4. Persist user overrides in the **encrypted ProfileStore** (per AGENTS.md Zero-Config First), with no env-var or plaintext requirement, surfacing the resolved flag set in the startup banner, `/health`, and `docs/`.

The system is deliberately **not** an A/B testing harness, **not** a kill-switch service for breakage in the field, **not** a permissioning system, and **not** a gradual-rollout / cohort tool. Those are all valid problems but they are different problems and conflating them produces a worse implementation of all four.

---

## Table of contents

1. [Goals + non-goals](#1-goals--non-goals)
2. [Flag taxonomy](#2-flag-taxonomy)
3. [Naming convention](#3-naming-convention)
4. [Storage](#4-storage)
5. [Runtime evaluation](#5-runtime-evaluation)
6. [The Settings matrix UI](#6-the-settings-matrix-ui)
7. [Composite-level vs panel-level vs feature-level flags](#7-composite-level-vs-panel-level-vs-feature-level-flags)
8. [Day-1 flag matrix](#8-day-1-flag-matrix)
9. [Telemetry](#9-telemetry)
10. [Rollout plan](#10-rollout-plan)
11. [Acceptance criteria](#11-acceptance-criteria)
12. [Cross-client consistency](#12-cross-client-consistency)
13. [Worked example: `composite.rl_os` end to end](#13-worked-example-compositerl_os-end-to-end)
14. [Open questions](#14-open-questions)

---

## 1. Goals + non-goals

### Goals

1. **Hide experimental surfaces by default.** A first-time user opening VibeCody should see only GA-quality surfaces. Beta surfaces are visible too (because they're solid enough to ship as defaults) but clearly labelled. Experimental surfaces are completely hidden until the user opts in. Internal/dev panels are completely hidden unless "Developer mode" is on.
2. **Single source of truth in the daemon.** Flag definitions, defaults, and tier metadata live in one Rust file in `vibecli`. Every other surface (Tauri command, HTTP route, mobile, watch, IDE plugin, SDK) reads from the daemon. Drift between clients is impossible by construction.
3. **Zero-config compliant.** Per AGENTS.md, the system must work out of the box with no required user action. Defaults are compiled in, user overrides are stored encrypted, env-var overrides are dev-only fallbacks, and the resolved set is surfaced in the startup banner, `/health`, and the docs.
4. **One Settings UX.** Users get a single, searchable, well-labelled matrix in Settings → Features. They never have to edit a JSON file, set an env var, or learn a flag name to enable an experimental surface — everything is discoverable from the UI.
5. **Cheap to add a flag.** Adding a new flag must be a 1–3 line change in one file plus a single line in the panel's render guard. If gating a panel takes more than 5 minutes, adoption will collapse and the system will be useless.
6. **Composite-aware.** Most flags will gate composites (which group panels into tabs), not individual panels. The system has to express composite/panel/feature scoping cleanly.
7. **Production-grade itself.** The flag system is not allowed to be the next "scaffold" panel. It must hit the user's 10-point definition-of-done before it's considered shipped (see §11).

### Non-goals

1. **Not an A/B testing platform.** No traffic-splitting, no variant selection, no statistical rollout. If we need A/B testing later, it goes in a separate system.
2. **Not a kill switch.** The flag system is not the right place to disable a feature in the field after a regression — that's a release/hotfix problem, not a flag problem. Flags here are *user preferences over which surfaces to show*, not *operational levers over what code paths execute in production*.
3. **Not a permissioning system.** No per-user, per-org, per-tenant rules. Every user on a given install sees the same flag set unless they personally toggle it.
4. **Not a gradual rollout tool.** No "10% of users get this on by default". Defaults are global and compiled in.
5. **Not a remote config service.** The daemon does not phone home to fetch flags. The compiled defaults plus the user's encrypted overrides are everything.
6. **Not a feature-discovery surface.** The Settings matrix lists *flagged* features. It is not a marketing page or a feature catalog. Users looking for "what can VibeCody do?" should consult the docs / command palette / sidebar, not Settings → Features.
7. **Not a per-workspace flag store.** Flags are a profile concern (the *user's* preference), not a workspace concern. They live in `ProfileStore`, never `WorkspaceStore`. A user opening five different workspaces sees the same flagged surfaces in all five.

### Why these non-goals matter

Every feature-flag system in the wild eventually grows into one of these other things and at that point it becomes either bloated or unsafe (or both). Drawing the line at "user preference for showing experimental surfaces" keeps the system small, the UX legible, and the code path one-dimensional: *was the flag on or off when the panel rendered?*

---

## 2. Flag taxonomy

Every flag has exactly one of four **tiers**:

| Tier | Default | Visible in Settings? | User can toggle? | Use for |
|---|---|---|---|---|
| **GA** | on | no (hidden — there's nothing to toggle) | no (compiled-in `true`) | Core surfaces that have hit the 10-point definition-of-done |
| **Beta** | on | yes — under "Beta" sub-tab, with a "BETA" pill | yes (opt out) | Surfaces that work but have rough edges; we want default exposure but the user can hide them |
| **Experimental** | off | yes — under "Experimental" sub-tab, with an amber "EXPERIMENTAL" pill | yes (opt in) | Surfaces that may break, may change shape, may produce wrong output. User-elective only. |
| **Internal** | off | no — unless `developer_mode` flag is on, then visible under "Developer" sub-tab | yes (opt in) when developer mode is on | Debug panels, telemetry inspectors, internal performance counters, dev-only test harnesses |

### Tier semantics

* **GA flags exist as records in the registry** but `useFeatureFlag()` for a GA flag always returns `true`. They appear in `/v1/flags` so clients can introspect "what is this build's GA surface?" but they are not toggle-able. The rationale for keeping GA flags as records (rather than not defining them at all) is that it makes the migration GA → Beta or Beta → GA a one-line change in the registry rather than a code-wide search-and-delete.

* **Beta flags** have `default: true`. The user can flip them off. When they do, the panel/composite/feature disappears from the UI (or becomes a no-op on the backend, in the case of feature-level flags). When they flip it back on, it reappears.

* **Experimental flags** have `default: false`. The user must explicitly opt in. Behind the toggle, the surface comes with an amber "EXPERIMENTAL" pill and a tooltip "This feature may change or break." The user never gets to opt in *and* hide the warning — the warning is part of the contract.

* **Internal flags** have `default: false` and are gated behind a meta-flag `feature.developer_mode`. When `developer_mode` is off (the default), Internal flags are not shown in Settings at all and `useFeatureFlag()` returns `false` for them. When `developer_mode` is on, they appear under a "Developer" sub-tab and can be toggled.

### Tier promotion / demotion

Flags move tiers as the underlying surface matures:

```
Internal → Experimental → Beta → GA
                ↑                  ↓
                └── (regression) ──┘
```

A tier change is a one-line edit in `defaults.json` and a corresponding update in the docs. No data migration is required because user overrides are stored by **flag name**, not by **tier** — if a user had `panel.foo` enabled while it was Experimental and we promote it to GA, their override silently becomes a no-op (GA flags are always on) and we discard the override on next write.

### Anti-pattern: "Beta-but-actually-default-off"

Resist the temptation to have a Beta flag with `default: false`. A Beta flag with `default: false` is just an Experimental flag with worse labelling. If the surface isn't ready to be on by default, it is not Beta yet — it is Experimental.

---

## 3. Naming convention

Flag names are **lowercase**, **dotted**, **hierarchical**, and **stable across versions**. The first segment is one of three scopes:

| Scope prefix | Meaning | Example |
|---|---|---|
| `panel.` | Single React panel under `vibecoder/src/components/` | `panel.rl_training`, `panel.diffcomplete_history` |
| `composite.` | A composite under `vibecoder/src/components/composite/` (groups multiple panels into tabs) | `composite.rl_os`, `composite.ai_playground` |
| `feature.` | Backend or cross-cutting capability that does not map 1:1 to a panel | `feature.recap_llm_generator`, `feature.sandbox_tier_firecracker`, `feature.developer_mode` |

### Rules

1. **Lowercase only.** No `Panel.RLTraining` — always `panel.rl_training`.
2. **`snake_case` within a segment.** No `panel.rl-training` (kebab) or `panel.rlTraining` (camel).
3. **Match the panel/composite/feature ID.** `panel.rl_training` corresponds to `RLTrainingPanel.tsx`. The mapping is mechanical (`PascalCase` → `snake_case`, drop the trailing `Panel` / `Composite` suffix).
4. **No version numbers in the name.** `feature.recap_llm_generator` — not `feature.recap_llm_generator_v2`. If we need to break-change a feature, we ship a new flag and deprecate the old one over one release.
5. **Stable across releases.** A flag name, once shipped, is contractual. Renaming a flag requires a deprecation alias for at least one minor version.
6. **No environment in the name.** No `feature.recap_llm_generator_dev`. Environment selection is not what flags are for.

### Suffixes

Composite flags imply panel flags: setting `composite.rl_os = false` hides every panel in that composite, *unless* a child panel also has its own flag set to `true` (panel-level override wins — see §7).

If a composite has child-specific flags, they live under the composite's name with the panel suffix: `panel.rl_os.rl_training`, `panel.rl_os.rl_rlhf`, etc. This makes the parent/child relationship explicit in the registry.

### Examples

```
panel.diffcomplete                       # GA — always on
panel.diffcomplete_history               # Beta — opt out
panel.recap_llm_generator                # Experimental — opt in
composite.rl_os                          # Experimental — opt in (covers 10 child panels)
panel.rl_os.rl_training                  # (auto-derived from composite.rl_os)
feature.sandbox_tier_firecracker         # Experimental — backend; gates Tier-3 in sandbox-tiers
feature.recap_llm_generator              # Experimental — backend gate for LLM-driven recap
feature.developer_mode                   # Meta-flag — gates Internal tier visibility
panel.debug_telemetry_inspector          # Internal — only with developer_mode on
```

---

## 4. Storage

### Layered resolution

Flag values are resolved in priority order, *most-specific wins*:

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. Env-var override   VIBE_FLAG_<NAME>=on|off  (dev-only)        │  ← highest priority
├──────────────────────────────────────────────────────────────────┤
│ 2. User override      ProfileStore["feature_flags"][name]        │
├──────────────────────────────────────────────────────────────────┤
│ 3. Compiled default   defaults.json.tier-implied default         │  ← lowest priority
└──────────────────────────────────────────────────────────────────┘
```

A GA-tier flag short-circuits all of this and always returns `true`. Internal flags additionally gate on `feature.developer_mode` evaluating to `true`.

### Where each layer lives

#### 4.1 Compiled defaults — `vibecoder/src/featureFlags/defaults.json`

Authoritative copy of the registry, generated from the Rust source of truth at build time (see §4.4). Read by the React frontend so panel render guards can synchronously evaluate "does this flag exist? what's its tier? what's its default?" without a daemon round-trip on first paint.

```json
{
  "version": 1,
  "flags": [
    {
      "name": "composite.rl_os",
      "tier": "experimental",
      "default": false,
      "label": "RL Operating System",
      "description": "10-panel suite for reinforcement-learning workflows. Slice 1–7 implementations exist but most surfaces are illustrative.",
      "covers": [
        "panel.rl_os.rl_training",
        "panel.rl_os.rl_rlhf",
        "panel.rl_os.rl_eval",
        "panel.rl_os.rl_rollouts",
        "panel.rl_os.rl_rewards",
        "panel.rl_os.rl_policies",
        "panel.rl_os.rl_replay",
        "panel.rl_os.rl_inference",
        "panel.rl_os.rl_artifacts",
        "panel.rl_os.rl_governance"
      ],
      "owner": "rl-os",
      "since": "v0.42.0"
    }
  ]
}
```

The frontend ships this file in the bundle. It is regenerated on every build from the Rust registry — the frontend `defaults.json` and the Rust `Registry::compiled()` are guaranteed identical because the JSON is the dump of the Rust value.

#### 4.2 User overrides — `ProfileStore["feature_flags"]`

Per AGENTS.md, the encrypted `ProfileStore` (`~/.vibecli/profile_settings.db`) is the single legal place for user preferences that aren't workspace-scoped. Feature-flag overrides live there under the namespace `feature_flags`:

```rust
// in vibecli/vibecli-cli/src/profile_store.rs (extending the existing store)

const FEATURE_FLAGS_NAMESPACE: &str = "feature_flags";

impl ProfileStore {
    pub fn get_flag_override(&self, name: &str) -> Result<Option<bool>> {
        self.get_kv(FEATURE_FLAGS_NAMESPACE, name)
            .map(|opt| opt.and_then(|s| s.parse::<bool>().ok()))
    }

    pub fn set_flag_override(&self, name: &str, value: bool) -> Result<()> {
        self.set_kv(FEATURE_FLAGS_NAMESPACE, name, &value.to_string())
    }

    pub fn clear_flag_override(&self, name: &str) -> Result<()> {
        self.delete_kv(FEATURE_FLAGS_NAMESPACE, name)
    }

    pub fn all_flag_overrides(&self) -> Result<HashMap<String, bool>> {
        self.list_namespace(FEATURE_FLAGS_NAMESPACE)
            .map(|kv| {
                kv.into_iter()
                    .filter_map(|(k, v)| v.parse::<bool>().ok().map(|b| (k, b)))
                    .collect()
            })
    }
}
```

**Critical:** flag overrides are **encrypted at rest** like any other ProfileStore value. They are never written to `*.toml`, `*.json`, or any plaintext file. They are never written to `~/.vibecoder/`.

#### 4.3 Env-var override — `VIBE_FLAG_<NAME>=on|off`

Dev-only fallback. The transformation `name → env var` is mechanical:

```
panel.rl_training         → VIBE_FLAG_PANEL_RL_TRAINING
composite.rl_os           → VIBE_FLAG_COMPOSITE_RL_OS
feature.developer_mode    → VIBE_FLAG_FEATURE_DEVELOPER_MODE
```

(Uppercase, dots → underscores, prefix `VIBE_FLAG_`.)

Accepted values: `on`, `off`, `true`, `false`, `1`, `0`. Anything else logs a warning and is ignored.

Per AGENTS.md, **env vars are not allowed to be required for production**. They exist only to let an engineer flip a flag during local development without going through the Settings UI. Production users never set these.

The startup banner (`vibecli serve` / `vibecli daemon`) prints which env-var overrides are active so the user can see why a flag is in an unexpected state:

```
[startup] VibeCody daemon v0.43.0
[startup] Profile: ravbod@gmail.com
[startup] Workspace: /Volumes/fast01/source/other/git/vibecody
[startup] Feature flags: 38 GA, 12 Beta, 14 Experimental, 9 Internal (developer_mode=off)
[startup]   env-var overrides active:
[startup]     composite.rl_os = on  (VIBE_FLAG_COMPOSITE_RL_OS=on)
[startup]   user overrides active: 3 (use `vibecli flags list --user` to inspect)
```

#### 4.4 Source of truth — `vibecli/vibecli-cli/src/feature_flags/registry.rs`

```rust
// vibecli/vibecli-cli/src/feature_flags/registry.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Ga,
    Beta,
    Experimental,
    Internal,
}

impl Tier {
    pub fn default_value(self) -> bool {
        matches!(self, Tier::Ga | Tier::Beta)
    }
    pub fn user_visible(self, dev_mode: bool) -> bool {
        match self {
            Tier::Ga => false,             // nothing to toggle
            Tier::Beta | Tier::Experimental => true,
            Tier::Internal => dev_mode,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagSpec {
    pub name: String,
    pub tier: Tier,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub covers: Vec<String>,    // child flag names (composite-level only)
    pub owner: String,           // team / area
    pub since: String,           // version added
}

pub struct Registry {
    flags: Vec<FlagSpec>,
}

impl Registry {
    pub fn compiled() -> &'static Registry { /* generated build.rs */ }
    pub fn get(&self, name: &str) -> Option<&FlagSpec> { /* ... */ }
    pub fn iter(&self) -> impl Iterator<Item = &FlagSpec> { self.flags.iter() }
    pub fn export_json(&self) -> String { /* dumps to defaults.json shape */ }
}
```

A `build.rs` step in `vibecli-cli` writes `vibecoder/src/featureFlags/defaults.json` from `Registry::compiled().export_json()` so the two are always in sync. CI fails the build if `defaults.json` is committed out of date with the Rust source.

### Storage summary table

| Layer | Location | Format | Mutability | Scope |
|---|---|---|---|---|
| Compiled defaults | `vibecoder/src/featureFlags/defaults.json` (generated from Rust `Registry::compiled()`) | JSON | Build-time only | Per build |
| User overrides | `~/.vibecli/profile_settings.db` namespace `feature_flags` | Encrypted KV (string→bool) | Runtime, via Settings UI or `vibecli flags set` | Per profile |
| Env-var override | Process env `VIBE_FLAG_<NAME>` | String (`on`/`off`) | Per-process | Dev-only |

---

## 5. Runtime evaluation

### 5.1 Daemon resolution

The daemon resolves the full flag set at startup and on every Settings change:

```rust
// vibecli/vibecli-cli/src/feature_flags/resolver.rs

pub struct ResolvedFlags {
    map: HashMap<String, bool>,
    tier: HashMap<String, Tier>,
}

impl ResolvedFlags {
    pub fn resolve(
        registry: &Registry,
        profile_overrides: &HashMap<String, bool>,
        env: &HashMap<String, String>,
    ) -> Self {
        let mut map = HashMap::new();
        let mut tier = HashMap::new();
        let dev_mode = Self::resolve_one(
            registry,
            profile_overrides,
            env,
            "feature.developer_mode",
        );

        for spec in registry.iter() {
            tier.insert(spec.name.clone(), spec.tier);
            let value = match spec.tier {
                Tier::Ga => true,
                Tier::Internal if !dev_mode => false,
                _ => Self::resolve_one(registry, profile_overrides, env, &spec.name),
            };
            map.insert(spec.name.clone(), value);
        }
        Self { map, tier }
    }

    fn resolve_one(
        registry: &Registry,
        profile: &HashMap<String, bool>,
        env: &HashMap<String, String>,
        name: &str,
    ) -> bool {
        // 1. env override
        let env_key = format!(
            "VIBE_FLAG_{}",
            name.to_uppercase().replace('.', "_"),
        );
        if let Some(v) = env.get(&env_key) {
            match v.to_ascii_lowercase().as_str() {
                "on" | "true" | "1" => return true,
                "off" | "false" | "0" => return false,
                _ => log::warn!("invalid VIBE_FLAG value for {name}: {v}"),
            }
        }
        // 2. user override
        if let Some(&v) = profile.get(name) {
            return v;
        }
        // 3. compiled default
        registry.get(name)
            .map(|s| s.tier.default_value())
            .unwrap_or(false)
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        // Composite-aware lookup: if name is `panel.rl_os.rl_training`,
        // and panel.rl_os.rl_training has no override, fall back to composite.rl_os.
        if let Some(&v) = self.map.get(name) {
            return v;
        }
        if let Some(parent) = composite_parent_of(name) {
            return self.map.get(&parent).copied().unwrap_or(false);
        }
        false
    }
}
```

### 5.2 Tauri command

```rust
// vibecoder/src-tauri/src/commands.rs

#[tauri::command]
pub async fn feature_flags(
    state: tauri::State<'_, AppState>,
) -> Result<FeatureFlagsResponse, String> {
    let resolved = state.feature_flags.read().await.snapshot();
    Ok(FeatureFlagsResponse {
        version: 1,
        flags: resolved.into_iter().map(|(name, value, tier, spec)| {
            FlagDto { name, value, tier, label: spec.label, description: spec.description, ... }
        }).collect(),
    })
}

#[tauri::command]
pub async fn feature_flag_set(
    state: tauri::State<'_, AppState>,
    name: String,
    value: Option<bool>,    // None = clear override
) -> Result<(), String> {
    let store = state.profile_store.read().await;
    match value {
        Some(v) => store.set_flag_override(&name, v).map_err(|e| e.to_string())?,
        None    => store.clear_flag_override(&name).map_err(|e| e.to_string())?,
    }
    state.feature_flags.write().await.reload().await;
    state.event_bus.emit("feature-flags-changed", ()).ok();
    Ok(())
}
```

Both commands are registered in `vibecoder/src-tauri/src/lib.rs`'s `tauri::generate_handler!` block, and (mirroring) in `vibeapp/src-tauri/src/lib.rs`.

### 5.3 HTTP route

```
GET  /v1/flags
GET  /v1/flags/:name
POST /v1/flags/:name      body: { "value": true | false | null }
```

Response shape:

```json
{
  "version": 1,
  "developer_mode": false,
  "flags": [
    {
      "name": "composite.rl_os",
      "tier": "experimental",
      "value": false,
      "default": false,
      "label": "RL Operating System",
      "description": "...",
      "covers": ["panel.rl_os.rl_training", "..."],
      "user_overridden": false,
      "env_overridden": false
    }
  ]
}
```

`POST /v1/flags/:name` requires a paired client (mobile / watch / IDE plugin), bearer-authenticated like the rest of the `/v1/*` surface.

### 5.4 React hook

```ts
// vibecoder/src/featureFlags/useFeatureFlag.ts

import { useContext } from "react";
import { FeatureFlagsContext } from "./FeatureFlagsProvider";

export function useFeatureFlag(name: string): boolean {
  const ctx = useContext(FeatureFlagsContext);
  if (!ctx) {
    // Fail closed for Experimental/Internal, fail open for Beta/GA
    // when the provider hasn't mounted yet (initial paint).
    return defaultsForName(name);
  }
  return ctx.isEnabled(name);
}

export function useFeatureFlagSpec(name: string): FlagSpec | null {
  const ctx = useContext(FeatureFlagsContext);
  return ctx?.spec(name) ?? null;
}

export function useAllFlags(): FlagDto[] {
  const ctx = useContext(FeatureFlagsContext);
  return ctx?.all() ?? [];
}
```

The provider lives at the app root, fetches `feature_flags` once on mount, and re-fetches on the `feature-flags-changed` Tauri event. The compiled `defaults.json` is bundled so first paint is synchronous.

### 5.5 Render guards

A panel that's flag-gated wraps its export:

```tsx
// vibecoder/src/components/RLTrainingPanel.tsx

import { withFeatureFlag } from "../featureFlags/withFeatureFlag";

function RLTrainingPanel() {
  // ... existing implementation
}

export default withFeatureFlag("panel.rl_os.rl_training", RLTrainingPanel);
```

`withFeatureFlag` returns `null` when the flag is off, so the panel is invisible (no DOM, no React subtree, no side effects). The sidebar / composite / command palette consumes the flag set directly via `useAllFlags()` to decide which entries to show.

### 5.6 `/health` integration

The existing `/health` endpoint gains a `feature_flags` block:

```json
{
  "status": "ok",
  "version": "0.43.0",
  ...
  "feature_flags": {
    "registry_version": 1,
    "counts": { "ga": 38, "beta": 12, "experimental": 14, "internal": 9 },
    "developer_mode": false,
    "user_overrides": 3,
    "env_overrides": 1
  }
}
```

This is the AGENTS.md Zero-Config First requirement: the resolved feature-flag state must be observable from `/health` so an operator can answer "what does this user actually see?" without crawling the encrypted store.

---

## 6. The Settings matrix UI

### 6.1 Where it lives

A new section in `vibecoder/src/components/SettingsPanel.tsx`. The existing `SettingsSection` type is extended:

```ts
type SettingsSection =
  | "profile"
  | "appearance"
  | "oauth"
  | "customizations"
  | "apikeys"
  | "integrations"
  | "sessions"
  | "features";   // new
```

The "Features" section renders three sub-tabs: **Beta** / **Experimental** / **Developer**. (No "GA" sub-tab because there's nothing to toggle.) The "Developer" sub-tab is hidden unless `feature.developer_mode` is on; when shown, it lists Internal-tier flags **and** the `feature.developer_mode` toggle itself (so the user can turn it off again).

### 6.2 Layout

```
┌────────────────────────────────────────────────────────────────────────┐
│ Settings → Features                                                    │
├────────────────────────────────────────────────────────────────────────┤
│ [Beta] [Experimental] [Developer]                  Search: [_______]   │
├────────────────────────────────────────────────────────────────────────┤
│  Bulk: [Enable all GA + Beta]  [Hide everything Experimental]          │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│ ┌───┐  Multi-Model Compare          [BETA]               (reset)       │
│ │ ✓ │  panel.multi_model                                               │
│ └───┘  Run a single prompt against multiple models in parallel.        │
│ ────────────────────────────────────────────────────────────────────── │
│ ┌───┐  Counsel                       [BETA]              (reset)       │
│ │ ✓ │  composite.counsel                                               │
│ └───┘  Multi-agent deliberation panel. Some sub-tools still scaffold.  │
│ ────────────────────────────────────────────────────────────────────── │
│ ...                                                                    │
└────────────────────────────────────────────────────────────────────────┘
```

(Dotted name in monospace, label bold, description in secondary text. Tier pill (`BETA` / `EXPERIMENTAL`) sits inline next to the label. "(reset)" link only appears when the row has a user override, and clears it on click.)

### 6.3 Row anatomy

| Element | Source | Interaction |
|---|---|---|
| Checkbox | Current resolved value | Click → toggle → persist via `feature_flag_set` Tauri command |
| Label | `FlagSpec.label` | — |
| Dotted name | `FlagSpec.name` | Monospace; click to copy |
| Tier pill | `FlagSpec.tier` | Beta = neutral grey-blue; Experimental = amber with tooltip "This feature may change or break."; Internal = magenta with tooltip "Internal tool — exposed because Developer mode is on." |
| Description | `FlagSpec.description` | One line, truncate with ellipsis on overflow + tooltip |
| Reset link | Visible iff `user_overridden == true` | Click → `feature_flag_set(name, null)` → row jumps back to default |
| Env-override badge | Visible iff `env_overridden == true` | Read-only, with tooltip "Set by `VIBE_FLAG_FOO=on` — clear the env var to use the Settings value." Toggle is disabled. |

### 6.4 Bulk actions

* **"Enable all GA + Beta"** — clears every user override on Beta-tier flags (so they go back to their default-on state) and on Experimental-tier flags (so they go back to default-off). Equivalent to "go back to recommended defaults".
* **"Hide everything Experimental"** — sets every Experimental-tier flag to `false` explicitly (writes user overrides, even for ones that are already off, so flag-tier promotions don't surprise the user).

Both actions show a confirmation dialog with a count of flags about to change.

### 6.5 Search / filter

A single text input filters rows by case-insensitive substring against {label, dotted name, description}. No advanced query language. If a sub-tab has zero matches after filtering, show an empty-state message: "No Beta features match \"foo\"."

### 6.6 Visual treatment of pills

```
[BETA]            background: var(--surface-info-soft),  text: var(--text-info)
[EXPERIMENTAL]    background: var(--surface-warning-soft), text: var(--text-warning)
[INTERNAL]        background: var(--surface-magenta-soft), text: var(--text-magenta)
```

Per `vibecoder/design-system/README.md` token system. No raw hex codes.

### 6.7 Empty states

* If a sub-tab has no flags (e.g., Developer when `developer_mode` is off), show an empty state with a one-line explanation:
  * Beta empty: "No Beta features in this build."
  * Experimental empty: "No Experimental features in this build."
  * Developer empty (visible only when developer_mode is on): "No Internal flags in this build."

* If `developer_mode` is off, the Developer tab itself is hidden — there is no empty state for it.

### 6.8 Settings persistence path

Flag toggles do **not** go through `localStorage`. They are persisted exclusively to ProfileStore via the Tauri command, so they survive a `localStorage.clear()` and are encrypted at rest. The `SettingsPanel` reads the resolved set via `useAllFlags()` — no local state caching beyond what the React provider already does.

### 6.9 Sketch

```tsx
// vibecoder/src/components/settings/FeaturesSection.tsx

import React, { useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAllFlags } from "../../featureFlags/useFeatureFlag";
import type { FlagDto, Tier } from "../../featureFlags/types";

type SubTab = "beta" | "experimental" | "developer";

export function FeaturesSection() {
  const flags = useAllFlags();
  const devMode = useMemo(
    () => flags.find(f => f.name === "feature.developer_mode")?.value ?? false,
    [flags],
  );
  const [tab, setTab] = useState<SubTab>("beta");
  const [query, setQuery] = useState("");

  const tierForTab: Record<SubTab, Tier> = {
    beta: "beta",
    experimental: "experimental",
    developer: "internal",
  };

  const visibleFlags = useMemo(() => {
    const tier = tierForTab[tab];
    const q = query.trim().toLowerCase();
    return flags
      .filter(f => f.tier === tier)
      .filter(f =>
        !q ||
        f.label.toLowerCase().includes(q) ||
        f.name.toLowerCase().includes(q) ||
        f.description.toLowerCase().includes(q),
      );
  }, [flags, tab, query]);

  const toggle = async (name: string, value: boolean) => {
    await invoke("feature_flag_set", { name, value });
  };
  const reset = async (name: string) => {
    await invoke("feature_flag_set", { name, value: null });
  };

  return (
    <div className="features-section">
      <Tabs>
        <Tab active={tab === "beta"} onClick={() => setTab("beta")}>Beta</Tab>
        <Tab active={tab === "experimental"} onClick={() => setTab("experimental")}>
          Experimental
        </Tab>
        {devMode && (
          <Tab active={tab === "developer"} onClick={() => setTab("developer")}>
            Developer
          </Tab>
        )}
        <SearchInput value={query} onChange={setQuery} />
      </Tabs>
      <BulkActions />
      {visibleFlags.length === 0 ? (
        <EmptyState tab={tab} hasQuery={!!query} />
      ) : (
        <ul className="flag-list">
          {visibleFlags.map(f => (
            <FlagRow key={f.name} flag={f} onToggle={toggle} onReset={reset} />
          ))}
        </ul>
      )}
    </div>
  );
}
```

(Concrete `FlagRow`, `Tabs`, `BulkActions`, `EmptyState` components left as exercises for the implementor. They follow the design-system token rules in `vibecoder/design-system/README.md`.)

---

## 7. Composite-level vs panel-level vs feature-level flags

Three scopes, with layered evaluation:

### 7.1 Composite-level flags

`composite.<id>` flags gate an entire composite (a tab group). When off, the composite disappears from the sidebar entirely and every child panel is unreachable through navigation. The composite-level flag also exposes a `covers` array listing every child panel's name, so the Settings UI can show a tooltip "Hides 10 child panels".

Composite flags are the **primary lever** — the Day-1 matrix uses composite-level flags wherever possible because (a) the user thinks at the composite level (it's the unit of navigation), and (b) one composite-level flag is cheaper than 10 panel flags to maintain.

### 7.2 Panel-level flags

`panel.<id>` flags gate a single panel. Used when:

* A panel is in a GA composite but the panel itself isn't ready (e.g., `panel.recap_llm_generator` inside an otherwise-GA Recap composite).
* A panel is in an Experimental composite but happens to be GA-quality and we want it visible regardless (e.g., `panel.rl_os.rl_inference` could be promoted to GA while the rest of the composite stays Experimental).

**Override rule:** if both `composite.X` and `panel.X.Y` exist as registry entries, `panel.X.Y`'s evaluation wins. Specifically: the panel renders iff `useFeatureFlag("panel.X.Y") && useFeatureFlag("composite.X")` resolves true *unless* the panel has its own user override, in which case the panel's override is used directly without consulting the composite.

This sounds confusing in prose but the resolver code (§5.1) handles it cleanly: panel-level lookup with composite fallback, plus an explicit override slot per name.

### 7.3 Feature-level flags

`feature.<id>` flags gate backend or cross-cutting capabilities that don't have a 1:1 panel correspondence. Examples:

* `feature.recap_llm_generator` — when off, the daemon's recap subsystem uses only the heuristic generator and never invokes the LLM path, even if the LLM-recap panel is enabled. (The panel becomes a no-op in that combination.)
* `feature.sandbox_tier_firecracker` — when off, the daemon refuses to launch Tier-3 Firecracker microVMs and the sandbox config UI hides the Tier-3 option.
* `feature.developer_mode` — meta-flag, gates Internal-tier visibility everywhere (Settings UI, sidebar, command palette).

Feature flags are checked **server-side** in the daemon (`ResolvedFlags::is_enabled()` in Rust) as well as client-side, because they typically protect a backend capability that should not be reachable even if a malicious client lies about its UI state.

### 7.4 Combination matrix

| `composite.X` value | `panel.X.Y` value | `panel.X.Y` user override? | Renders? |
|---|---|---|---|
| on | on | — | yes |
| on | off | — | no |
| off | on | — | yes (panel-level wins) |
| off | off | — | no |
| on | (no entry) | — | yes (composite controls) |
| off | (no entry) | — | no (composite controls) |
| any | any | yes (any value) | the override value (override always wins) |

### 7.5 When to use which

* **Default to `composite.X`.** Ship one flag for the whole composite.
* **Add `panel.X.Y` only** when one child panel needs to deviate from the composite's tier.
* **Use `feature.X` only** for backend gates or cross-cutting toggles that don't map to a single render guard.

---

## 8. Day-1 flag matrix

Concrete proposed flags for the current state of the codebase, based on the most recent production-readiness audit. **Counts at the bottom** (~73 flags total). All of these become live in Phase A → B → C → D over four ships.

### 8.1 GA — always on, hidden from Settings (38 flags)

These are surfaces that have hit the 10-point definition-of-done. The user can't turn them off because there's no toggle for them in Settings.

| Flag | Surface |
|---|---|
| `panel.diffcomplete` | Diffcomplete inline AI editing surface |
| `panel.diffcomplete_history` | Diffcomplete history view |
| `panel.memory` | Memory panel (CRUD over OpenMemory store) |
| `panel.settings` | Settings panel itself |
| `panel.recap` | Recap-heuristic panel |
| `panel.background_jobs` | Background jobs panel |
| `panel.chat_tab_manager` | Chat tab manager |
| `panel.sessions_list` | Sessions list |
| `panel.session_detail` | Single-session detail |
| `panel.api_keys` | API keys panel inside Settings |
| `panel.profile` | Profile panel inside Settings |
| `panel.appearance` | Appearance panel inside Settings |
| `panel.oauth` | OAuth panel inside Settings |
| `panel.integrations` | Integrations panel inside Settings |
| `panel.customizations` | Customizations panel inside Settings |
| `panel.command_palette` | Cmd-K command palette |
| `panel.editor` | Monaco editor surface |
| `panel.terminal` | Terminal panel |
| `panel.file_tree` | File tree |
| `panel.git_status` | Git status panel |
| `panel.diff_viewer` | Diff viewer |
| `panel.search` | Workspace search |
| `panel.error_log` | Error log surface |
| `panel.notifications` | Notification center |
| `feature.session_persistence` | Backend: session persistence to workspace store |
| `feature.workspace_indexing` | Backend: vibe-indexer on the workspace |
| `feature.profile_store` | Backend: encrypted profile store |
| `feature.workspace_store` | Backend: encrypted workspace store |
| `feature.health_endpoint` | Backend: `/health` |
| `feature.flags_endpoint` | Backend: `/v1/flags` (this system itself) |
| `feature.recap_heuristic` | Backend: heuristic recap generator |
| `feature.diffcomplete` | Backend: diffcomplete generator |
| `feature.memory_store` | Backend: OpenMemory single-store |
| `feature.openai_provider` | Backend: OpenAI provider |
| `feature.anthropic_provider` | Backend: Anthropic provider |
| `feature.local_provider` | Backend: local provider (Ollama-compat) |
| `feature.pairing_p256` | Backend: P-256 device pairing |
| `feature.lan_mdns` | Backend: mDNS LAN announcement |

### 8.2 Beta — on by default, user can opt out (12 flags)

These work but have rough edges. Default-on so users see them, but they get a "BETA" pill and can hide them.

| Flag | Surface |
|---|---|
| `panel.multi_model` | Multi-Model Compare |
| `panel.arena` | Arena (head-to-head model comparison) |
| `composite.counsel` | Counsel (multi-agent deliberation composite) |
| `composite.super_brain` | SuperBrain composite |
| `panel.mobile_recap_viewer` | Mobile recap header (M1.1, just shipped) |
| `panel.watch_recap_viewer` | Watch recap viewer |
| `composite.ai_playground` | AI Playground composite |
| `composite.ai_teams` | AI Teams composite |
| `panel.background_agents` | Background agents inspector |
| `panel.skills_browser` | Skills browser (711 skill files) |
| `panel.model_hub` | Model hub (Slice 5 of RL-OS, but standalone-usable) |
| `feature.recap_session_kind` | Backend: session-kind recap generator (vs heuristic-only) |

### 8.3 Experimental — off by default, opt-in (14 flags)

Off out of the box. User must explicitly opt in via Settings → Features → Experimental. Amber pill, "may change or break" tooltip.

| Flag | Surface |
|---|---|
| `composite.rl_os` | RL-OS composite (covers all 10 RL panels) |
| `feature.recap_llm_generator` | Backend: LLM-driven recap generator (vs heuristic) |
| `feature.recap_kind_diffcomplete` | Backend: diffcomplete-kind recap (per recap-resume design) |
| `feature.sandbox_tier_native` | Sandbox Tier-0 (existing dead-code `BwrapProfile` / `sandbox-exec` / AppContainer) |
| `feature.sandbox_tier_firecracker` | Sandbox Tier-3 (Firecracker microVM) |
| `feature.sandbox_tier_hyperlight` | Sandbox Tier-2 (Hyperlight hypervisor partition) |
| `feature.sandbox_egress_broker` | Backend: egress broker for sandbox tiers |
| `feature.rl_os_native_onnx` | RL-OS Slice 7d native ONNX runtime (Path C step) |
| `panel.collab_session` | Multi-user collaboration panel |
| `panel.voice_input` | Voice input panel |
| `composite.enterprise_governance` | Enterprise governance composite |
| `panel.cloud_deploy` | Cloud deployment panel |
| `panel.observability` | Observability panel (slice mocks) |
| `panel.data_pipeline` | Data pipeline composite (mostly scaffold) |

(Anything currently behind a `// SLICE_N_MOCK` source-code marker should be flagged Experimental as a blanket rule. Audit pass during Phase C is responsible for finding those and adding the appropriate flag entries.)

### 8.4 Internal — developer-mode only (9 flags)

Hidden unless `feature.developer_mode` is on. Magenta pill.

| Flag | Surface |
|---|---|
| `feature.developer_mode` | Meta-flag — gates Internal visibility |
| `panel.debug_telemetry_inspector` | Telemetry inspector |
| `panel.debug_event_bus` | Event bus inspector |
| `panel.debug_flag_inspector` | Flag inspector (raw resolved-flag dump for testing) |
| `panel.debug_profile_store` | ProfileStore raw KV viewer |
| `panel.debug_workspace_store` | WorkspaceStore raw KV viewer |
| `panel.debug_lsp_inspector` | LSP message inspector |
| `panel.debug_provider_log` | Per-provider request/response log |
| `panel.debug_perf_counters` | Performance counters dashboard |

### 8.5 Day-1 totals

| Tier | Count |
|---|---|
| GA | 38 |
| Beta | 12 |
| Experimental | 14 |
| Internal | 9 |
| **Total** | **73** |

---

## 9. Telemetry

Per AGENTS.md, **no external telemetry without explicit user opt-in**. The flag system itself produces local structured log lines and nothing else.

### 9.1 What we log

Every flag flip emits a single structured log line:

```
{
  "ts": "2026-04-30T12:34:56.789Z",
  "event": "feature_flag.changed",
  "name": "composite.rl_os",
  "from": false,
  "to": true,
  "source": "settings_ui" | "cli" | "env_var_startup" | "bulk_action",
  "tier": "experimental"
}
```

Logged to the daemon's existing structured logger (the same one that captures session events). Visible in `panel.debug_telemetry_inspector` when developer mode is on.

The startup banner already announces the flag counts (§4.3); no need to re-log them per flag at startup.

### 9.2 What we don't log

* No sending to any remote endpoint. Period.
* No correlation IDs that could be used to track a user across flag flips.
* No flag-flip frequency aggregation. If we want "how often do users opt into Experimental?" we get it from a future opt-in survey, not from passive logging.

### 9.3 Local introspection

`vibecli flags log` (CLI subcommand) tails the local log filtered to `feature_flag.*` events. Useful for debugging "why did this flag flip?" without spelunking through the global log.

---

## 10. Rollout plan

Four ships, each independently releasable. No phase requires breaking changes from a prior phase.

### Phase A — Infrastructure (no flags wired yet)

**Goal:** land the system with zero behavior change for users.

**Deliverables:**
1. `vibecli/vibecli-cli/src/feature_flags/` module (`registry.rs`, `resolver.rs`, `mod.rs`).
2. Initial `Registry::compiled()` containing **only** the GA flags from §8.1 plus `feature.developer_mode`. (No Beta/Experimental/Internal entries yet — those land in later phases.)
3. ProfileStore extension: `feature_flags` namespace + `get/set/clear/all_flag_overrides`.
4. Tauri commands: `feature_flags`, `feature_flag_set`. Registered in both `vibecoder/src-tauri/src/lib.rs` and `vibeapp/src-tauri/src/lib.rs`.
5. HTTP routes: `GET /v1/flags`, `GET /v1/flags/:name`, `POST /v1/flags/:name`. Wired in `serve.rs`.
6. `build.rs` step that regenerates `vibecoder/src/featureFlags/defaults.json` from the Rust registry.
7. `vibecoder/src/featureFlags/`: `FeatureFlagsProvider.tsx`, `useFeatureFlag.ts`, `withFeatureFlag.ts`, `types.ts`.
8. SettingsPanel extension: new "Features" section with Beta / Experimental / Developer sub-tabs. **All three sub-tabs render their empty state on day one** because no Beta/Experimental/Internal flags exist yet.
9. CLI subcommand: `vibecli flags list [--user|--env|--all]`, `vibecli flags set NAME VALUE`, `vibecli flags clear NAME`, `vibecli flags log`.
10. `/health` `feature_flags` block.
11. Startup banner update: print the flag counts and active overrides.
12. Docs: this file is shipped, plus a one-paragraph entry in `AGENTS.md` Zero-Config First section pointing here.

**Acceptance:** existing UI is byte-identical to pre-Phase-A. `vibecli flags list` returns 38 GA flags + 1 meta-flag (developer_mode). Settings → Features shows three empty sub-tabs (Developer is hidden because developer_mode defaults to false).

### Phase B — RL-OS composite behind one flag

**Goal:** the single biggest readiness win — hide 10 illustrative panels behind one flag with one Settings entry.

**Deliverables:**
1. Add `composite.rl_os` to the registry as Experimental, default false, with `covers: [panel.rl_os.rl_training, panel.rl_os.rl_rlhf, ...]` listing all 10 panels.
2. Wrap `RLOSComposite.tsx` with `withFeatureFlag("composite.rl_os", ...)`.
3. Update sidebar / composite registry to filter out `composite.rl_os` when the flag is off.
4. Update `RLOSComposite`'s `SimulationModeBadge` so when the user *is* opted in, the badge stays in place but its `covers` prop populates accurately.
5. Verify: on a fresh install, the RL-OS sidebar entry is gone. Opting in via Settings → Features → Experimental → RL Operating System makes it appear within one frame (no reload).

**Acceptance:** Settings → Features → Experimental shows exactly 1 row (`composite.rl_os`) with the amber pill. Default-off. Toggling shows / hides the entire composite plus all 10 child panels in the sidebar.

### Phase C — rest of Experimental tier

**Goal:** flag every other Experimental-tier surface from §8.3.

**Deliverables:**
1. Add the remaining 13 Experimental flags to the registry.
2. Wrap each panel/composite with `withFeatureFlag(...)`.
3. For each `feature.*` flag, add the server-side `if !flags.is_enabled(...) { return Err(...) }` guards in the daemon code paths they protect.
4. Audit pass: grep for `// SLICE_N_MOCK` in the codebase, file each instance under its corresponding flag (or open an issue if the audit finds an unflagged mock).
5. Verify: fresh install + default settings + click around the entire app. Should be impossible to reach an Experimental surface without going through Settings.

**Acceptance:** `vibecli flags list --tier experimental` returns 14 flags. Settings → Features → Experimental lists all 14 with their pills, descriptions, and toggles. Bulk action "Hide everything Experimental" is a no-op on a fresh install (because they're already off) but the dialog correctly shows a count of 0 changes.

### Phase D — Beta tier

**Goal:** flag the 12 Beta-tier surfaces from §8.2.

**Deliverables:**
1. Add the 12 Beta flags to the registry.
2. Wrap each panel/composite with `withFeatureFlag(...)`.
3. Verify: defaults are unchanged (Beta flags are on by default, so the UI looks the same).
4. The Settings → Features → Beta tab now shows 12 rows with BETA pills and "(reset)" links for any user who has opted out.

**Acceptance:** A user who never visits Settings sees exactly the same UI before and after Phase D. A user who visits Settings → Features → Beta sees 12 rows and can opt out of any of them.

### Phase E (optional, post-rollout) — Internal tier

**Goal:** add the developer-mode panels from §8.4.

Can ship at any point after Phase A. Only valuable internally; user-facing impact is zero unless they discover and enable `feature.developer_mode`.

### Out of scope for this rollout

* Composite-level flags for non-Experimental composites (`composite.ai_playground`, `composite.ai_teams` etc.) — these are listed under Beta but should ship as panel-level flags first if the composite contains any GA-quality panels we don't want to pull. The triage decision happens during Phase D implementation.
* Backend gating beyond what's listed. Server-side guards for `feature.recap_llm_generator`, `feature.sandbox_tier_*`, `feature.rl_os_native_onnx` are mandatory in Phase C; other backend gates are added on demand.

---

## 11. Acceptance criteria

The flag system itself must hit the 10-point definition-of-done. This list is the "is the flag system shipped?" checklist; each item is binary.

| # | Criterion | What "done" looks like |
|---|---|---|
| 1 | **Real backend** | `Registry::compiled()` is populated from `vibecli/vibecli-cli/src/feature_flags/registry.rs`. `ResolvedFlags::resolve()` honors all three layers (env > user > default). No stub returns. |
| 2 | **Empty-state UX** | All three Settings sub-tabs render an empty-state message when no flags match. The Developer sub-tab is correctly hidden when `feature.developer_mode` is off. |
| 3 | **Error UX** | `feature_flag_set` failure shows a toast with the error message. `/v1/flags` 5xx on the mobile/watch client falls back to the cached set with a banner "Flags last synced: 5 min ago". |
| 4 | **Tests** | Unit tests for `ResolvedFlags::resolve` covering all three layers, the GA short-circuit, the Internal+dev-mode interaction, and the composite-fallback rule. Integration test for `feature_flag_set` round-trip through Tauri. UI snapshot test for the Settings matrix. CLI `vibecli flags` smoke test. |
| 5 | **Docs** | This file. Plus a one-paragraph entry in `AGENTS.md` and a row in `docs/release.md`'s "what's new" for the release that ships Phase A. |
| 6 | **`/health` field** | `feature_flags` block present and populated. `vibecli health --json` includes it. |
| 7 | **Structured logging** | `feature_flag.changed` events emitted on every flip with `from`, `to`, `source`, `tier`. Visible in `vibecli flags log`. |
| 8 | **ProfileStore for persistence** | All user overrides stored encrypted in `~/.vibecli/profile_settings.db` namespace `feature_flags`. No `localStorage`. No plaintext file. No env-var requirement. |
| 9 | **Accessibility** | Settings matrix is keyboard-navigable. Each checkbox has an accessible label that includes the dotted name and tier. Tier pills have `aria-label`. Search input has `<label>`. Reset links are real `<button>`s with focus styles. Color is not the only signal for tier (the pill text says "BETA" / "EXPERIMENTAL"). |
| 10 | **Cross-client consistency** | `/v1/flags` returns the same shape on every client. Mobile, watch, and IDE plugins consume it (see §12). The flag registry compiled into the daemon is the only source of truth — no client has its own flag list. |

A flag system that fails any of these is a half-shipped flag system. The whole point of the system is to mark *other* features as half-shipped; it cannot itself be half-shipped or it loses authority.

---

## 12. Cross-client consistency

VibeCody is 13 clients × 1 daemon. Flags are defined in the daemon. Each client reads from `/v1/flags` and falls back gracefully when offline.

### 12.1 The data flow

```
┌─────────────────────┐
│ ProfileStore        │ ← user overrides (encrypted)
│ feature_flags ns    │
└──────────┬──────────┘
           │ get_flag_overrides()
           ▼
┌─────────────────────┐    ┌─────────────────────┐
│ Compiled defaults   │ ── │ Env-var overrides   │ (dev-only)
│ Registry::compiled()│    │ VIBE_FLAG_*         │
└──────────┬──────────┘    └──────────┬──────────┘
           │                          │
           └────────┬─────────────────┘
                    ▼
          ┌──────────────────┐
          │ ResolvedFlags    │
          │ (in-memory)      │
          └────────┬─────────┘
                   │
        ┌──────────┼─────────────────────────────┐
        ▼          ▼                             ▼
   ┌────────┐  ┌─────────┐                 ┌──────────┐
   │ Tauri  │  │ HTTP    │                 │ CLI      │
   │ cmd    │  │ /v1/    │                 │ vibecli  │
   │        │  │ flags   │                 │ flags    │
   └───┬────┘  └────┬────┘                 └──────────┘
       │            │
       ▼            └────────┬─────────┬────────┬────────┬───────┐
   ┌────────┐                ▼         ▼        ▼        ▼       ▼
   │ vibecoder │           ┌────────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐
   │ vibeapp│           │mobile  │ │watch │ │vscode│ │jetbr.│ │SDK   │
   └────────┘           │Flutter │ │Swift │ │ext   │ │plugin│ │      │
                        │+ Kotlin│ │+ Wear│ │      │ │      │ │      │
                        └────────┘ └──────┘ └──────┘ └──────┘ └──────┘
```

### 12.2 Per-client integration

| Client | Path | Reads via | Cache | Offline fallback |
|---|---|---|---|---|
| **vibecli daemon (self)** | `vibecli/vibecli-cli/src/feature_flags/` | `ResolvedFlags` in-memory | n/a (source of truth) | n/a |
| **vibecoder (Tauri)** | `vibecoder/src/featureFlags/FeatureFlagsProvider.tsx` | `feature_flags` Tauri command | React provider, refetch on `feature-flags-changed` event | bundled `defaults.json` (compiled-in) for first paint |
| **vibeapp (Tauri)** | mirror of vibecoder | same as vibecoder | same | same |
| **vibemobile (Flutter)** | `vibemobile/lib/services/feature_flags_service.dart` (new) | `GET /v1/flags` via existing `api_client.dart` | sqflite cache `feature_flags_cache` | last-known-good cached set + banner "synced N min ago" |
| **vibewatch (SwiftUI)** | `vibewatch/VibeCodyWatch/FeatureFlagsManager.swift` (new) | `GET /v1/flags` via existing `WatchNetworkManager.swift` | UserDefaults `featureFlagsCache` | last-known-good or, on cold start with no cache, "GA-only" mode (only GA-tier surfaces shown) |
| **vibewatch (Wear OS)** | `vibewatch/VibeCodyWear/FeatureFlagsRepository.kt` (new) | `GET /v1/flags` via existing Retrofit client | DataStore `feature_flags_cache` | same as Swift |
| **vscode-extension** | `vscode-extension/src/feature-flags.ts` (new) | `GET /v1/flags` via `api-client.ts` | extension `globalState` | bundled snapshot of GA-only flags |
| **jetbrains-plugin** | `jetbrains-plugin/src/main/kotlin/.../FeatureFlags.kt` (new) | `GET /v1/flags` via existing HTTP client | `PropertiesComponent` | GA-only fallback |
| **neovim-plugin** | `neovim-plugin/lua/vibecody/feature_flags.lua` (new) | `GET /v1/flags` via existing curl wrapper | `vim.fn.stdpath('cache')/vibecody/flags.json` | GA-only fallback |
| **agent-sdk** | `packages/agent-sdk/src/feature-flags.ts` (new) | `GET /v1/flags` via SDK HTTP client | in-process cache | the SDK is offline-by-design — its fallback is "all flags off" because no UI is gated by them |

### 12.3 Refresh semantics

* **Online clients** poll `/v1/flags` on connect, then subscribe to a server-sent event (`SSE: feature-flags-changed`) for live updates. No SSE → 60-second poll.
* **Offline clients** use the cached set without time-out (a flag should not flip behind the user's back just because they lost network).
* **Cold-start with no cache** → GA-only fallback per the table above. Better to under-show than to flash an Experimental panel that shouldn't be there.

### 12.4 Write path from non-daemon clients

Writes (toggling a flag) are accepted via `POST /v1/flags/:name` on any paired client. The daemon validates the bearer token and applies the change to ProfileStore. Other clients receive the update via the SSE channel and refresh their local cache.

This means the user can flip a flag from their phone (e.g., enable RL-OS in Settings → Features on mobile) and the change is visible on their desktop on next render.

### 12.5 Invariants

1. **One registry.** No client ships its own flag list. All clients query `/v1/flags`.
2. **One default fallback shape.** When offline with no cache, every client falls back to "GA-only" — never to "all on", never to "all off", never to "the build's last-shipped defaults". GA-only is the safe minimum because GA is the only tier that can't break the user.
3. **No client writes to ProfileStore directly.** All writes go through `POST /v1/flags/:name` on the daemon. The daemon is the only process that ever touches the encrypted store for flag values.

---

## 13. Worked example: `composite.rl_os` end to end

A complete walk-through of one flag, from registry definition through user toggle through daemon resolution through render.

### 13.1 Registry definition

In `vibecli/vibecli-cli/src/feature_flags/registry.rs`:

```rust
FlagSpec {
    name: "composite.rl_os".into(),
    tier: Tier::Experimental,
    label: "RL Operating System".into(),
    description: "10-panel suite for reinforcement-learning workflows. \
                  Slice 1–7 implementations exist but most surfaces are \
                  illustrative.".into(),
    covers: vec![
        "panel.rl_os.rl_training".into(),
        "panel.rl_os.rl_rlhf".into(),
        "panel.rl_os.rl_eval".into(),
        "panel.rl_os.rl_rollouts".into(),
        "panel.rl_os.rl_rewards".into(),
        "panel.rl_os.rl_policies".into(),
        "panel.rl_os.rl_replay".into(),
        "panel.rl_os.rl_inference".into(),
        "panel.rl_os.rl_artifacts".into(),
        "panel.rl_os.rl_governance".into(),
    ],
    owner: "rl-os".into(),
    since: "v0.42.0".into(),
}
```

### 13.2 Build-time JSON dump

`build.rs` regenerates `vibecoder/src/featureFlags/defaults.json` to include:

```json
{
  "name": "composite.rl_os",
  "tier": "experimental",
  "default": false,
  "label": "RL Operating System",
  "description": "10-panel suite for reinforcement-learning workflows. Slice 1–7 implementations exist but most surfaces are illustrative.",
  "covers": ["panel.rl_os.rl_training", "panel.rl_os.rl_rlhf", "..."],
  "owner": "rl-os",
  "since": "v0.42.0"
}
```

### 13.3 Daemon startup resolution

A user starts `vibecli serve`. The daemon:

1. Loads `Registry::compiled()` (which contains `composite.rl_os` with default `false`).
2. Opens `ProfileStore` and calls `all_flag_overrides()` — for a fresh user this returns `{}`.
3. Reads env vars — `VIBE_FLAG_COMPOSITE_RL_OS` is unset.
4. Resolves: tier = Experimental, no env, no override, default = `false`. → `composite.rl_os = false`.
5. Prints to startup banner: "Feature flags: 38 GA, 12 Beta, 14 Experimental, 9 Internal (developer_mode=off)".

### 13.4 Frontend first paint

vibecoder mounts. `FeatureFlagsProvider` synchronously loads `defaults.json` from the bundle so the app can render *something* before the Tauri round-trip completes. For the RL-OS composite, the bundled default says `false`, so `RLOSComposite` (wrapped with `withFeatureFlag("composite.rl_os", ...)`) returns `null` and never enters the DOM.

200ms later the `feature_flags` Tauri command resolves and the provider updates with the daemon's authoritative answer. For this user, the answer matches the bundled default (`false`), so no re-render is needed.

### 13.5 User opts in

User opens Settings → Features → Experimental, finds "RL Operating System" with the amber EXPERIMENTAL pill, ticks the checkbox. The handler fires:

```ts
await invoke("feature_flag_set", { name: "composite.rl_os", value: true });
```

Tauri command:

1. Calls `ProfileStore::set_flag_override("composite.rl_os", true)`.
2. Calls `ResolvedFlags::reload()` — re-runs the three-layer resolution. New value: `true`.
3. Emits `feature-flags-changed` event over Tauri IPC.
4. Logs `{"event":"feature_flag.changed","name":"composite.rl_os","from":false,"to":true,"source":"settings_ui","tier":"experimental"}`.
5. Sends `/v1/flags` SSE notification to any other paired clients (mobile, watch, IDE).

### 13.6 Frontend re-render

The `FeatureFlagsProvider` listens for `feature-flags-changed`, refetches the resolved set, updates its context value. Every component that called `useFeatureFlag("composite.rl_os")` re-renders. `RLOSComposite`'s `withFeatureFlag` wrapper now returns the actual composite. The sidebar adds the RL-OS entry. All 10 child panels become reachable through navigation.

### 13.7 Mobile sees the change

The mobile client's open SSE connection receives the `feature-flags-changed` notification, calls `GET /v1/flags`, updates its local cache, re-renders any flag-aware UI (mobile doesn't currently surface RL-OS, but the cached state changes for consistency).

### 13.8 User opts back out

User unchecks the box. Same flow in reverse:

1. `feature_flag_set("composite.rl_os", false)` → ProfileStore now has `{ composite.rl_os: false }` (note: the override is *false*, not absent — the user has explicitly opted out, not "reverted to default").
2. Re-resolve, re-render, RL-OS disappears from the sidebar.
3. Log: `{"event":"feature_flag.changed","name":"composite.rl_os","from":true,"to":false,"source":"settings_ui","tier":"experimental"}`.

### 13.9 User clicks "(reset)"

User clicks the "(reset)" link next to the row. Handler fires:

```ts
await invoke("feature_flag_set", { name: "composite.rl_os", value: null });
```

This clears the override entirely. The resolver falls through to the compiled default (`false` for Experimental). The row's checkbox now matches the default, the "(reset)" link disappears. The user's ProfileStore no longer has an entry for `composite.rl_os`.

### 13.10 Engineer toggles via env var

A developer runs `VIBE_FLAG_COMPOSITE_RL_OS=on vibecli serve`. The daemon:

1. Detects env override at startup, resolves `composite.rl_os = true` regardless of the user's ProfileStore value.
2. Prints to startup banner: "env-var overrides active: composite.rl_os = on (VIBE_FLAG_COMPOSITE_RL_OS=on)".
3. Settings UI shows the row with the checkbox in the on state but **disabled**, with a tooltip "Set by VIBE_FLAG_COMPOSITE_RL_OS=on — clear the env var to use the Settings value." The "(reset)" link is hidden because the user override (if any) is being shadowed by the env var.

User stops the daemon, clears the env var, restarts → resolution falls back to ProfileStore (or default if no override). UI re-enables the checkbox.

### 13.11 Tier promotion later

Sometime later, RL-OS matures and gets promoted to Beta. A one-line edit in `registry.rs`:

```rust
- tier: Tier::Experimental,
+ tier: Tier::Beta,
```

Result on next ship:
* The flag's default flips from `false` to `true`.
* The Settings row moves from the Experimental sub-tab to the Beta sub-tab.
* The pill changes from amber EXPERIMENTAL to blue-grey BETA.
* The tooltip text changes accordingly.
* Existing user overrides survive: a user who had explicitly opted in stays opted in (override = true matches new default = true, no visible change), a user who had explicitly opted out stays opted out (override = false overrides new default = true, the user's choice is honored).

No data migration. No deprecation alias. The override key is the same.

### 13.12 Eventual GA promotion

Even later, RL-OS hits GA. Edit:

```rust
- tier: Tier::Beta,
+ tier: Tier::Ga,
```

Result:
* The Settings row disappears (GA has no toggle).
* `useFeatureFlag("composite.rl_os")` short-circuits to `true` regardless of any user override.
* On next ProfileStore write, the now-meaningless override is silently discarded.

A user who had specifically opted out is now forced to see the composite again. This is intentional — promoting to GA is a statement that the surface is core enough that the user no longer gets to hide it from themselves.

---

## 14. Open questions

These are the decisions left to make during Phase A implementation. None of them blocks design approval; all of them need an answer before code lands.

1. **Where exactly does `defaults.json` live?** The doc proposes `vibecoder/src/featureFlags/defaults.json`. Alternative: `vibecoder/src/generated/featureFlags.json` to make the "this is generated, don't edit by hand" warning more obvious. Decision punted to Phase A reviewer.
2. **Composite.rl_os covers list — auto-derived or manual?** Auto-derived from a `RegisterPanel`-style declaration would be cleaner but doesn't exist today. Manual list is fine for Day 1; auto-derivation is a Phase F nice-to-have if anyone ever wants it.
3. **SSE channel: new endpoint or piggy-back on existing event stream?** vibecli already has `/v1/events` for session updates. Adding a `feature-flags-changed` event type is cheaper than a new endpoint. Recommend piggy-back.
4. **`vibecli flags list` output format.** Default to a human table. `--json` for machine. `--tier=experimental` to filter. Standard CLI hygiene; decide once during implementation.
5. **Should the Internal/Developer sub-tab require a confirmation when toggling `feature.developer_mode` on?** Probably yes — a "this exposes internal-only tools that may show sensitive state. Continue?" dialog. Belongs in the implementation, not the spec.
6. **Tier of the feature-flags panel itself in the debug section (`panel.debug_flag_inspector`).** Listed as Internal in §8.4. Ensure the inspector renders even when the registry has zero flags, since that's its job to surface.
7. **Migration story for users mid-Phase-D when a previously-unflagged surface becomes Beta and they were relying on always-visible behavior.** Mitigation: Beta is on by default, so this is a no-op for the user — but if they had a script driving the UI, the surface stays visible because Beta defaults to on. No migration is needed unless we promote something *down* a tier (which the spec doesn't currently support cleanly — open question).
8. **SDK behavior when a `useFeatureFlag` call references a flag that doesn't exist in the registry.** Spec says "fail closed" (return `false`). Confirm with a unit test in Phase A.
9. **Env-var override of `feature.developer_mode`.** Should `VIBE_FLAG_FEATURE_DEVELOPER_MODE=on` be honored? Yes, because the env-var layer is "highest priority" and consistency matters. Document this in `vibecli flags --help` so it's discoverable.
10. **Versioning of `defaults.json`.** If we ever ship a breaking change (e.g., add a `category` field to FlagSpec), the `version: 1` envelope lets consumers detect and degrade gracefully. Bump the version on any non-additive change.

---

## Appendix A: file-level change list for Phase A

For an engineer implementing Phase A, these are every file that needs to be touched. Use this as the implementation checklist.

### New files

* `vibecli/vibecli-cli/src/feature_flags/mod.rs`
* `vibecli/vibecli-cli/src/feature_flags/registry.rs`
* `vibecli/vibecli-cli/src/feature_flags/resolver.rs`
* `vibecli/vibecli-cli/src/feature_flags/cli.rs` (`vibecli flags ...` subcommands)
* `vibecli/vibecli-cli/build.rs` extension (regenerate `defaults.json`)
* `vibecoder/src/featureFlags/FeatureFlagsProvider.tsx`
* `vibecoder/src/featureFlags/useFeatureFlag.ts`
* `vibecoder/src/featureFlags/withFeatureFlag.ts`
* `vibecoder/src/featureFlags/types.ts`
* `vibecoder/src/featureFlags/defaults.json` (generated)
* `vibecoder/src/components/settings/FeaturesSection.tsx`
* `vibecoder/src/components/settings/FlagRow.tsx`
* `vibecoder/src/components/settings/BulkActions.tsx`
* `vibecoder/src/components/settings/EmptyState.tsx`
* `docs/design/feature-flags/README.md` (this file)

### Modified files

* `vibecli/vibecli-cli/src/lib.rs` — `pub mod feature_flags;`
* `vibecli/vibecli-cli/src/main.rs` — `mod feature_flags;` + register `flags` subcommand
* `vibecli/vibecli-cli/src/profile_store.rs` — add `*_flag_override` methods
* `vibecli/vibecli-cli/src/serve.rs` (or `watch_bridge.rs` per current routing convention) — `/v1/flags`, `/v1/flags/:name` routes
* `vibecli/vibecli-cli/src/health.rs` — add `feature_flags` block to `/health` response
* `vibecoder/src-tauri/src/commands.rs` — `feature_flags` and `feature_flag_set` commands
* `vibecoder/src-tauri/src/lib.rs` — register both in `tauri::generate_handler!`
* `vibeapp/src-tauri/src/commands.rs` — mirror
* `vibeapp/src-tauri/src/lib.rs` — mirror in `tauri::generate_handler!`
* `vibecoder/src/components/SettingsPanel.tsx` — extend `SettingsSection` with `"features"`, render `FeaturesSection`
* `vibecoder/src/App.tsx` (or wherever the root provider chain lives) — wrap with `FeatureFlagsProvider`
* `AGENTS.md` — one-paragraph mention under Zero-Config First pointing to this doc
* `docs/release.md` — entry in the next release's "what's new"
* `CLAUDE.md` — a "feature flags" line under Quick Reference (`vibecli flags list`)

### Test files

* `vibecli/vibecli-cli/tests/feature_flags_resolver.rs`
* `vibecli/vibecli-cli/tests/feature_flags_http.rs`
* `vibecli/vibecli-cli/tests/feature_flags_cli.rs`
* `vibecoder/src/featureFlags/__tests__/useFeatureFlag.test.ts`
* `vibecoder/src/components/settings/__tests__/FeaturesSection.test.tsx`

---

## Appendix B: detailed Phase A code sketches

This section contains fully-fleshed code sketches that an implementor can crib from directly. They are not load-bearing for the design — the spec above is — but they save an hour of "what does this actually look like in code?".

### B.1 `vibecli/vibecli-cli/src/feature_flags/mod.rs`

```rust
//! Feature flags — see docs/design/feature-flags/README.md
//!
//! Three layers, lowest to highest priority:
//!   1. Compiled defaults from `Registry::compiled()`
//!   2. User overrides from ProfileStore namespace `feature_flags`
//!   3. Env-var overrides via `VIBE_FLAG_<NAME>`
//!
//! GA-tier flags short-circuit to `true`. Internal-tier flags additionally
//! gate on `feature.developer_mode`.

mod registry;
mod resolver;
mod cli;

pub use registry::{FlagSpec, Registry, Tier};
pub use resolver::ResolvedFlags;
pub use cli::FlagsSubcommand;

/// Convenience: compute the env-var name for a flag.
/// `composite.rl_os` -> `VIBE_FLAG_COMPOSITE_RL_OS`.
pub fn env_var_for(name: &str) -> String {
    format!("VIBE_FLAG_{}", name.to_uppercase().replace('.', "_"))
}

/// Given `panel.rl_os.rl_training`, returns Some("composite.rl_os").
/// Returns None if there is no composite parent.
pub fn composite_parent_of(name: &str) -> Option<String> {
    let stripped = name.strip_prefix("panel.")?;
    let mut parts = stripped.splitn(2, '.');
    let composite = parts.next()?;
    let _child = parts.next()?;   // require there to be a child segment
    Some(format!("composite.{composite}"))
}
```

### B.2 `vibecli/vibecli-cli/src/feature_flags/cli.rs`

```rust
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct FlagsSubcommand {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// List flags.
    List {
        /// Filter to a single tier.
        #[arg(long)]
        tier: Option<String>,
        /// Show only flags with a user override.
        #[arg(long)]
        user: bool,
        /// Show only flags with an env override.
        #[arg(long)]
        env: bool,
        /// Show all (default).
        #[arg(long)]
        all: bool,
        /// JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Set a user override.
    Set { name: String, value: String },
    /// Clear a user override (revert to default).
    Clear { name: String },
    /// Tail the structured log filtered to feature_flag.* events.
    Log,
}

impl FlagsSubcommand {
    pub fn run(self, ctx: &CliContext) -> anyhow::Result<()> {
        match self.cmd {
            Cmd::List { tier, user, env, all: _, json } => {
                /* implementation */
                Ok(())
            }
            Cmd::Set { name, value } => {
                let v = parse_bool(&value)?;
                ctx.profile_store.set_flag_override(&name, v)?;
                ctx.notify_flag_change(&name)?;
                println!("OK: {name} = {v}");
                Ok(())
            }
            Cmd::Clear { name } => {
                ctx.profile_store.clear_flag_override(&name)?;
                ctx.notify_flag_change(&name)?;
                println!("OK: cleared {name}");
                Ok(())
            }
            Cmd::Log => {
                /* tail structured logger filtered to feature_flag.* */
                Ok(())
            }
        }
    }
}

fn parse_bool(s: &str) -> anyhow::Result<bool> {
    match s.to_ascii_lowercase().as_str() {
        "on" | "true" | "1" | "yes" => Ok(true),
        "off" | "false" | "0" | "no" => Ok(false),
        _ => anyhow::bail!("invalid boolean: {s} (use on/off/true/false/1/0)"),
    }
}
```

### B.3 `vibecoder/src/featureFlags/types.ts`

```ts
export type Tier = "ga" | "beta" | "experimental" | "internal";

export interface FlagSpec {
  name: string;
  tier: Tier;
  default: boolean;
  label: string;
  description: string;
  covers?: string[];
  owner: string;
  since: string;
}

export interface FlagDto extends FlagSpec {
  value: boolean;
  userOverridden: boolean;
  envOverridden: boolean;
}

export interface FeatureFlagsResponse {
  version: number;
  developerMode: boolean;
  flags: FlagDto[];
}
```

### B.4 `vibecoder/src/featureFlags/FeatureFlagsProvider.tsx`

```tsx
import React, {
  createContext,
  useEffect,
  useMemo,
  useState,
  ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import defaultsJson from "./defaults.json";
import type { FlagSpec, FlagDto, FeatureFlagsResponse } from "./types";

interface ContextValue {
  isEnabled: (name: string) => boolean;
  spec: (name: string) => FlagSpec | null;
  all: () => FlagDto[];
  refresh: () => Promise<void>;
}

export const FeatureFlagsContext = createContext<ContextValue | null>(null);

const BUNDLED_DEFAULTS: FlagSpec[] = defaultsJson.flags;

function bundledFallback(): FlagDto[] {
  return BUNDLED_DEFAULTS.map(spec => ({
    ...spec,
    value: spec.tier === "ga" || (spec.tier === "beta" && spec.default),
    userOverridden: false,
    envOverridden: false,
  }));
}

export function FeatureFlagsProvider({ children }: { children: ReactNode }) {
  const [flags, setFlags] = useState<FlagDto[]>(bundledFallback);

  const refresh = async () => {
    try {
      const resp = await invoke<FeatureFlagsResponse>("feature_flags");
      setFlags(resp.flags);
    } catch (e) {
      console.warn("feature_flags fetch failed; keeping cached set", e);
    }
  };

  useEffect(() => {
    refresh();
    const unlisten = listen("feature-flags-changed", () => { refresh(); });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const value = useMemo<ContextValue>(() => {
    const byName = new Map(flags.map(f => [f.name, f]));
    return {
      isEnabled: (name) => {
        const exact = byName.get(name);
        if (exact) return exact.value;
        // composite-fallback for `panel.<composite>.<child>` → `composite.<composite>`
        const m = name.match(/^panel\.([^.]+)\.(.+)$/);
        if (m) {
          const parent = byName.get(`composite.${m[1]}`);
          if (parent) return parent.value;
        }
        return false;
      },
      spec: (name) => byName.get(name) ?? null,
      all: () => flags,
      refresh,
    };
  }, [flags]);

  return (
    <FeatureFlagsContext.Provider value={value}>
      {children}
    </FeatureFlagsContext.Provider>
  );
}
```

### B.5 `vibecoder/src/featureFlags/withFeatureFlag.ts`

```tsx
import React from "react";
import { useFeatureFlag } from "./useFeatureFlag";

export function withFeatureFlag<P extends object>(
  flagName: string,
  Component: React.ComponentType<P>,
): React.ComponentType<P> {
  const Wrapped: React.FC<P> = (props) => {
    const enabled = useFeatureFlag(flagName);
    if (!enabled) return null;
    return <Component {...props} />;
  };
  Wrapped.displayName = `withFeatureFlag(${Component.displayName || Component.name || "Component"})`;
  return Wrapped;
}
```

### B.6 Sidebar / composite registry filter

Wherever the sidebar consumes the composite list, add a flag-aware filter:

```tsx
// vibecoder/src/components/Sidebar.tsx (or wherever the composite registry is enumerated)

import { useAllFlags } from "../featureFlags/useFeatureFlag";

const ALL_COMPOSITES: CompositeEntry[] = [/* … */];

export function Sidebar() {
  const flags = useAllFlags();
  const enabled = useMemo(() => {
    const map = new Map(flags.map(f => [f.name, f.value]));
    return ALL_COMPOSITES.filter(c => {
      const flagName = `composite.${c.id}`;
      // If there's no flag entry, the composite is implicitly GA → show.
      if (!map.has(flagName)) return true;
      return map.get(flagName) === true;
    });
  }, [flags]);

  return <nav>{enabled.map(c => <CompositeLink key={c.id} entry={c} />)}</nav>;
}
```

### B.7 Mobile fetch (Flutter sketch)

```dart
// vibemobile/lib/services/feature_flags_service.dart

class FeatureFlagsService {
  final ApiClient api;
  final SharedPreferences prefs;

  Future<List<FlagDto>> fetch() async {
    try {
      final resp = await api.get('/v1/flags');
      final flags = (resp['flags'] as List).map(FlagDto.fromJson).toList();
      await _cache(flags);
      return flags;
    } catch (e) {
      return _loadCache() ?? _gaOnlyFallback();
    }
  }

  Future<void> set(String name, bool? value) async {
    await api.post('/v1/flags/$name', body: {'value': value});
  }
}
```

### B.8 watchOS fetch (Swift sketch)

```swift
// vibewatch/VibeCodyWatch/FeatureFlagsManager.swift

@MainActor
final class FeatureFlagsManager: ObservableObject {
    @Published private(set) var flags: [FlagDto] = []

    func refresh() async {
        do {
            let resp: FeatureFlagsResponse = try await network.get("/v1/flags")
            self.flags = resp.flags
            persistToUserDefaults(resp.flags)
        } catch {
            self.flags = loadCachedOrGAOnly()
        }
    }

    func isEnabled(_ name: String) -> Bool {
        flags.first { $0.name == name }?.value ?? false
    }
}
```

---

## Appendix C: tier rationale (why these four, why not more)

We considered six tiers and rejected two of them. For the historical record:

### Considered but rejected

* **Alpha** (between Internal and Experimental). Rejected because the line between Alpha and Experimental is fuzzy — "even less ready than Experimental" doesn't add a useful axis for the user. If a surface is so unready it can't be on the Experimental list, it should not be flag-gated at all yet; it should live in a feature branch.
* **Deprecated** (a tier for sunset surfaces). Rejected because deprecation is a release-management concern, not a feature-flag concern. A surface being phased out should be removed from the build over one or two releases, not left around behind a "Deprecated" flag forever. Removal date in a release note > flag entry that grows stale.

### Kept

* **GA, Beta, Experimental** — the standard three-tier readiness ladder. Mirrors the language users already understand from browser releases (Chrome stable / beta / dev), database systems (PostgreSQL preview / beta / GA), etc.
* **Internal** — kept because there's a genuine "developer-only" use case (telemetry inspectors, raw store viewers, performance counters) that should not be visible to a normal user even if they want to opt in. Putting these behind `feature.developer_mode` is the right scope.

### What about cohort / percentage rollouts?

Explicitly out of scope per §1 non-goals. If we ever want them, they go in a separate file as `RolloutPolicy` orthogonal to `Tier`. The flag name stays the same; the rollout policy changes how the daemon resolves it. We are not designing for that today and we are not going to leave hooks for it that would constrain the simple case.

---

## Appendix D: failure modes and what happens

A list of "what if X breaks?" cases and the spec'd behavior. Not exhaustive but covers the obvious ones.

| Failure | Behavior |
|---|---|
| ProfileStore corrupt / missing | Resolver falls through to compiled defaults. No crash. Startup banner notes "ProfileStore unavailable, flag overrides not loaded". |
| `defaults.json` out of date with Rust registry | Build fails (CI gate). Cannot ship. |
| User flag override references a flag name no longer in the registry | Override is ignored at resolution time. Logged as a warning at startup. Optional: `vibecli flags clean` removes orphaned overrides. |
| Daemon down, mobile client reaches `/v1/flags` | Mobile client uses last-known cached set with banner "synced N min ago". |
| Daemon down, mobile client cold start with no cache | GA-only mode. Banner "offline — only stable features visible". |
| User flips a flag on mobile while desktop is offline | Change persists to ProfileStore (via daemon, when both are connected). Desktop sees the change next time it connects. There is no flag-flip queue on the client; the client cannot flip flags while the daemon is unreachable. |
| Env var with invalid value (`VIBE_FLAG_FOO=banana`) | Logged as warning at startup, ignored, falls through to user override / default. |
| Two env vars conflict (impossible per the format, but defensively) | First-resolved wins; `Vec<(String, String)>` from `std::env::vars()` is deterministic per process. |
| Tier promoted from Experimental to GA while user has explicit `false` override | Override silently discarded on next ProfileStore write. User sees the surface again. Documented behavior; not surprising. |
| User toggles a flag thousands of times in a debugging session | ProfileStore writes are durable; no rate limit. Structured log captures each event. No remote telemetry to spam. |
| `feature.developer_mode` toggled off while user has Internal-tier overrides | Overrides remain in ProfileStore but resolver returns `false` for all Internal flags. Toggling developer_mode back on restores them. |
| Flag name collision (two `FlagSpec` entries with the same `name`) | Build fails — `Registry::compiled()` validates uniqueness at construction time and `panic!`s if violated. |
| Composite flag `covers` references a panel name that has no panel | Build warning, not a hard fail. Lets us ship the composite flag before all child panels exist. |

---

## Appendix E: relationship to other VibeCody design docs

This system interacts with several existing design docs. The interactions are summarized here so an engineer reading any one of them knows where the boundary lives.

### vs. [docs/design/sandbox-tiers/README.md](../sandbox-tiers/README.md)

The sandbox-tiers spec defines four backends (Tier-0 native, Tier-1 WASI, Tier-2 Hyperlight, Tier-3 Firecracker). Each tier-2 and tier-3 backend is gated by a feature flag from this system: `feature.sandbox_tier_hyperlight` and `feature.sandbox_tier_firecracker`. Tier-0 is GA (it's the default path) and Tier-1 is also GA (existing hardened code). The flag values are checked **in the daemon's sandbox dispatcher** (`vibecli/vibecli-cli/src/sandbox/dispatch.rs`) — if the flag is off, the dispatcher refuses to construct the corresponding backend even if a client requests it.

### vs. [docs/design/recap-resume/README.md](../recap-resume/README.md)

The recap system has a heuristic generator (GA, always on, `feature.recap_heuristic`) and an LLM-driven generator (Experimental, off by default, `feature.recap_llm_generator`). The recap subsystem checks `feature.recap_llm_generator` at request time — when off, it falls through to the heuristic. The diffcomplete-kind recap (`feature.recap_kind_diffcomplete`) requires a per-slice patent re-audit per the recap-resume design; the feature flag adds a UI gate but does not substitute for the patent audit.

### vs. [docs/design/rl-os/README.md](../rl-os/README.md)

The RL-OS spec describes 7 slices, with `rl_*_os.rs` modules wired panel-by-panel as the simulation-mode disclaimer falls. This system's `composite.rl_os` flag (Experimental, default off) hides the entire 10-panel composite until the user opts in. As individual panels mature, they can be promoted to GA via panel-level flags (`panel.rl_os.<name>` set to GA tier) while the composite stays Experimental — see §7 for the override semantics. Slice 7d's native ONNX runtime gets its own backend gate: `feature.rl_os_native_onnx`, default off, even when the composite is on.

### vs. [docs/design/multi-agent-chat.md](../multi-agent-chat.md)

The multi-agent chat surfaces (Counsel, MultiModel, Arena, SuperBrain) are Beta-tier in the Day-1 matrix (§8.2). They are on by default but the user can hide them via Settings → Features → Beta. No backend feature flag needed — the panels are already self-contained.

### vs. [AGENTS.md → Zero-Config First](../../../AGENTS.md)

This system is itself bound by Zero-Config First. To check the box:

* No required env vars — env vars are dev-only fallbacks.
* User overrides stored in encrypted ProfileStore.
* Resolved set surfaced in startup banner, `/health`, and `docs/` (this file plus the `vibecli flags --help` text).
* Works out of the box: a fresh install with no ProfileStore, no env vars, and no Settings interaction, sees exactly the GA + Beta surfaces. No setup required.

The flag system is therefore not just *compliant* with Zero-Config First — it is the *enabler* of it for new features. The contract for adding a new panel becomes: "if it's not GA-quality yet, ship it Experimental and Zero-Config First is satisfied because the user never sees it without opting in."

---

## Appendix F: glossary

| Term | Meaning |
|---|---|
| **Flag** | A boolean toggle with a stable name like `panel.rl_training` |
| **Tier** | One of GA / Beta / Experimental / Internal |
| **GA** | "Generally Available" — production-quality, no toggle |
| **Beta** | On by default, user can opt out |
| **Experimental** | Off by default, user must opt in |
| **Internal** | Hidden unless `feature.developer_mode` is on |
| **Composite** | A grouping of related panels under one sidebar entry; lives in `vibecoder/src/components/composite/` |
| **Panel** | A single React component under `vibecoder/src/components/` that the user navigates to |
| **Feature** | A backend or cross-cutting capability that doesn't 1:1 map to a panel |
| **Override** | A user-set value that takes precedence over the compiled default |
| **Registry** | The compiled-in list of all known flags + their tiers + their metadata |
| **Resolver** | The code that combines defaults + overrides + env vars into the actual evaluated value |
| **ProfileStore** | The encrypted per-user KV store at `~/.vibecli/profile_settings.db` |
| **Zero-Config First** | The AGENTS.md contract that no setup should be required for a feature to work |
| **defaults.json** | The build-time JSON dump of the Rust `Registry`, shipped to the React bundle for synchronous first paint |

---

*End of design doc. See §10 for what to build first.*
