---
layout: page
title: MCP — Model Context Protocol
permalink: /mcp/
---

VibeCody is a fully-featured MCP host. The MCP panel inside VibeUI/VibeApp is the unified surface for managing servers, browsing the plugin directory, viewing the lazy-loaded tool registry, and inspecting cache metrics. This page documents the user-facing surface; the underlying client lives in `crates/vibe-ai/src/mcp.rs`.

---

## What's in the panel

The panel has five tabs:

| Tab | Purpose |
|---|---|
| **Servers** | Configure MCP servers — name, command, args, env, OAuth tokens. Test connectivity and see live tool lists. |
| **Tools** | The lazy-loaded tool registry. Search across servers, load / unload individual tools to manage agent-context budget. |
| **Directory** | Browse the curated plugin catalog by category, install or uninstall with one click. |
| **Installed** | Just the installed plugins, with View Tools navigation back to the Tools tab. |
| **Metrics** | Cache hit rate, average load time, context-savings percentage, and a per-tool load-time chart. |

---

## Server configuration

Server records live in `~/.vibeui/mcp.json` as a plain JSON array. Each entry:

```json
{
  "name": "filesystem",
  "command": "mcp-fs",
  "args": ["--root", "/workspace"],
  "env": { "MCP_LOG_LEVEL": "info" }
}
```

The panel writes this file via the `save_mcp_servers` Tauri command. **OAuth tokens** for servers that require auth go to `~/.vibeui/mcp-tokens.json` separately — they are *not* stored alongside the config because tokens are sensitive material.

### Test before save

The Servers tab has a **Test** button that spins up a short-lived MCP client connection, calls `list_tools`, and shows what the server would expose. Run this before saving — a server that can't be reached or doesn't speak MCP will surface the failure inline (with `aria-live="assertive"`) rather than silently land in the config.

### OAuth flow

For servers that need OAuth (e.g. cloud APIs):

1. Click **Connect via OAuth** on the server row.
2. Fill in client_id, auth_url, token_url, scopes, and the redirect URI (which the daemon listens on).
3. Click **Initiate** — the system browser opens to the authorization page.
4. After authorizing, the IDP redirects to the daemon's callback. Paste the resulting `code` back into the panel.
5. The panel exchanges the code for a token via `complete_mcp_oauth` and stores the token under the server's name in `mcp-tokens.json`.

The token-status badge on each server row reflects connectivity *and* expiration — a stale token shows as disconnected even if the row's config looks fine.

### Delete confirmation

Like the Sessions panel, deleting a server is a two-click confirmation: first click arms, second click commits. There's no recycle bin — the row is removed from `mcp.json` immediately on commit.

---

## Tool registry (lazy loading)

The Tools tab reflects the **lazy-loading runtime** — tools aren't held in agent context until you load them. The columns:

- **Status** — `loaded`, `loading`, or `unloaded`. Loading is animated; the actual load happens in the background.
- **Size** — KB cost when loaded.
- **Last used** — timestamp from the daemon's tool-call history.
- **Load time** — first-load wall-clock from when load was issued.

Click the row toggle to load or unload. The Tools tab shows live tools per *configured* server when reachable; servers that don't respond to test_mcp_server within the timeout simply collapse to "no tools listed" rather than blocking the whole tab.

### Search

The search box filters the registry across all servers in real time (200ms debounce). Matches against tool name and description. Use this when you can't remember which server a tool came from.

---

## Plugin directory

The Directory tab is a curated catalog of MCP plugins. Each entry has:

- **Author / Author rating** — surfaced for user trust signals.
- **Category** — filterable from the sidebar.
- **Install / Uninstall** — two-click confirmation. Install adds the plugin id to `~/.vibeui/mcp-installed.json`; uninstall removes it.

**Install does not auto-add a server entry to `mcp.json`.** The directory is metadata-only — actually wiring a plugin into your active MCP host happens in the Servers tab, with the plugin's documented `command` / `args`. This separation is deliberate so that "installed" tracks intent (the user wants this plugin available) without conflating it with active runtime state.

---

## /health declaration

`features.mcp` is a runtime probe of `~/.vibeui/mcp.json`:

```json
{
  "available": true,
  "transport": "tauri-desktop",
  "config_path": "~/.vibeui/mcp.json",
  "server_count": 3,
  "configured": true
}
```

Operators can hit `/health.features.mcp.server_count` to confirm an environment has MCP servers wired up. The boolean `configured` is `server_count > 0` — clients gating MCP-dependent features should read this single field.

---

## Observability

Backend operations emit structured tracing events under `vibecody::mcp`:

```bash
RUST_LOG=vibecody::mcp=info vibecli serve
```

Events:

```
INFO vibecody::mcp: mcp.servers.save server_count=3
INFO vibecody::mcp: mcp.server.test.start server_name=filesystem
INFO vibecody::mcp: mcp.server.test.ok   server_name=filesystem tool_count=12 elapsed_ms=420
WARN vibecody::mcp: mcp.server.test.failed server_name=git elapsed_ms=2080
INFO vibecody::mcp: mcp.plugin.install   plugin_id=terraform installed_total=4
INFO vibecody::mcp: mcp.plugin.uninstall plugin_id=terraform installed_total=3
WARN vibecody::mcp: mcp.plugin.install.not_found plugin_id=banana
```

Server commands and arguments are **not** logged — only the server name. Tool descriptions, OAuth tokens, and plugin metadata are never logged.

---

## Accessibility

- The panel's tab bar uses `role="tablist"` (set by the existing `panel-tab-bar` class) — keyboard navigation between tabs is available.
- The shared error banner is `role="alert"` with `aria-live="assertive"`, so failures from any subsystem (servers, tools, directory) are announced immediately.
- Two-click delete on servers and plugins carries the confirmation state in `aria-label` so AT users hear "second click commits" rather than just "Delete".
- The OAuth flow is a multi-step form with clear step labels; each input has an associated `<label>` and the **busy** state disables the submit button rather than just hiding it.

---

## Cross-client behaviour

| Client | MCP UI |
|---|---|
| **VibeUI / VibeApp** | Full panel (servers, tools, directory, metrics) |
| **VibeMobile** | None — MCP is a desktop-host concern |
| **VibeWatch** | None |
| **IDE plugins** | Read-only — surface configured server names for tool-completion contexts |

The panel runs in the Tauri host. Servers are subprocess-spawned by the host — they inherit the host's user but run in fresh process trees. Sandbox enforcement (see [`docs/sandbox`](./sandbox.md)) does not apply to MCP server subprocesses today; treat them as trusted helpers and use the OAuth flow rather than long-lived environment-variable tokens where possible.

---

## Troubleshooting

### "Connect failed: Connection refused" when testing a server

Either the binary isn't on PATH or the server isn't speaking MCP on stdio. Check the `command` field — it must resolve via `which` from the daemon's environment, not just the user's shell. Absolute paths are safer.

### "list_tools failed: invalid handshake"

The server connected but isn't returning a valid MCP `tools/list` response. Likely an old or non-MCP binary mistakenly named like an MCP server. Cross-check against the server's documentation — newer servers should support the `2024-11-05` protocol revision.

### "Plugin already installed" when nothing seems to be installed

Clear `~/.vibeui/mcp-installed.json` to reset the install ledger. The directory tab tracks intent only — it doesn't probe the file system.

### "Token exchange failed" during OAuth

Most often caused by a redirect URI mismatch between the panel and the IDP's registered application. The panel defaults to `http://localhost:7879/oauth/callback` — register that exact URL with your IDP, including the path.

### "Servers tab shows no live tools after Test"

The Tools tab probes all servers in parallel, but rendering is gated by `serverToolsLoading`. If a server hangs (no response, no error within the timeout), the panel resolves it to an empty list. Check `RUST_LOG=vibecody::mcp=info` for the corresponding `mcp.server.test.failed` event.

---

## Related

- **MCP client core:** `crates/vibe-ai/src/mcp.rs` — connect, list_tools, call_tool
- **Source:** `vibeui/src/components/McpPanel.tsx` (902 LOC) · backend in `vibeui/src-tauri/src/commands.rs`
- **Tests:** `vibeui/src/components/__tests__/McpPanel.test.tsx`
- **Sandbox:** [`docs/sandbox`](./sandbox.md) — note that MCP server subprocesses are NOT sandboxed in the current Tier-0
