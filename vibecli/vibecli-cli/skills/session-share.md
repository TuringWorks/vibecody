# session-share

Export sessions as self-contained HTML and share them via private GitHub Gist.
Bridges the pi-mono `/export` and `/share` command gap (Phase C3).

## Usage

```
/session export html [--title "My debug session"] [--light]
/session share [--token <GITHUB_TOKEN>] [--title "Bug repro"]
```

`/session export html` writes a `.html` file to the current directory.
`/session share` exports to HTML then uploads as a **private** GitHub Gist and
prints the shareable `https://gist.github.com/…` URL.

## Rules

### Rule 1 — /export generates a timestamped filename
HTML exports are named `session-<sanitized-title>-<YYYYMMDD>.html`.
Special characters in the title are replaced with `-`. Example:
`session-bug-repro-20260413.html`. Never overwrite an existing file;
append `-2`, `-3`, etc. if the path is taken.

### Rule 2 — /share creates private gists by default
The `GistOptions::public` field is `false` by default. Shared sessions
are **not** publicly listed on GitHub. To share publicly a user must
pass `--public` explicitly. Document this in any UI or help text.

### Rule 3 — GitHub token storage
The GitHub token must be read from, in order of preference:
1. `--token <TOKEN>` flag passed at runtime.
2. `GITHUB_TOKEN` environment variable.
3. The `ProfileStore` encrypted database key `github_token`.

Never persist a token to `*.toml`, `*.json`, or any plaintext file.
Reject tokens that do not match the pattern `ghp_[A-Za-z0-9]{36}` or
`github_pat_[A-Za-z0-9_]{82}` with a clear error message.

### Rule 4 — Code block highlighting is CSS-class-based (no JS required)
Syntax highlighting uses only CSS class names (`language-rust`,
`language-python`, etc.) on `<pre><code>` elements. No JavaScript is
bundled, no external CDN is referenced. The file is fully usable
offline. Downstream users may optionally include Highlight.js or Prism
via their own `<link>` tag if live highlighting is desired.

### Rule 5 — HTML export is self-contained
All CSS is inlined inside a `<style>` block in `<head>`. No `<link>`
tags, no `<script>` tags, no external fonts or image URLs. The exported
`.html` file must render identically in any modern browser with
network access disabled.

### Rule 6 — Gist description format
The Gist description must follow the pattern:
`VibeCody session — <title> (<ISO-8601 date>)`
Example: `VibeCody session — Bug repro (2026-04-13)`
This makes Gists discoverable in the user's own gist.github.com list.

### Rule 7 — Sharing etiquette: warn before sharing tool output
If the session contains messages with `role = Tool` whose `content`
exceeds 2 000 characters (e.g. large file reads or command output),
prompt the user:

> "This session includes large tool outputs. Are you sure you want to
> share it? Sensitive file contents may be visible to anyone with the
> link."

Only proceed after explicit confirmation (`y` / `yes`).

## Module

`vibecli/vibecli-cli/src/session_share.rs`

## Key types

| Type | Purpose |
|---|---|
| `ShareMessage` | Single message with role, content, tool name, timestamp |
| `ShareRole` | `User / Assistant / System / Tool` — maps to CSS classes |
| `HtmlExportOptions` | Title, dark/light theme, timestamp, code highlight, truncation |
| `HtmlExporter` | `export()`, `highlight_fences()`, `escape_html()` |
| `GistOptions` | Description, public flag, GitHub token |
| `GistResult` | `gist_id`, `html_url`, `raw_url`, `description` |
| `GistClient` | `build_payload()`, `parse_response()`, `upload()` |

## Related modules

- `session_export.rs` — JSON / Markdown / CSV export (this module adds HTML + Gist)
- `session_tree.rs` — Branching conversation trees
- `profile_store` — Encrypted token storage
