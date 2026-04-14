# TUI Images

Inline image rendering in terminal emulators — Kitty Graphics Protocol and iTerm2 inline images. Pi-mono gap bridge (Phase C1).

## When to Use
- Displaying image previews, generated diagrams, or screenshots directly in the CLI output
- Implementing a `render` REPL command that shows images without leaving the terminal
- Streaming generated images (AI image output) into TUI panels that support Kitty or iTerm2
- Providing graceful degradation to text placeholders for SSH or dumb-terminal sessions

## Rules

### 1. Protocol detection is environment-driven — never hard-code
Always call `ImageProtocol::detect()` at runtime. The order of precedence is:
1. `$KITTY_WINDOW_ID` — present in every Kitty child process.
2. `$TERM` containing "kitty" — for embedded Kitty sessions.
3. `$TERM_PROGRAM` containing "iterm", "wezterm", or "hyper" — iTerm2-compatible.
4. Fall through to `ImageProtocol::None`.

Override detection only when the caller explicitly passes `RenderOptions { protocol: … }`.

### 2. Always provide a text fallback
When `ImageProtocol::None` is active (SSH, basic xterm, CI), `render_image_bytes()` returns a `RenderResult` with `fallback = true` and a human-readable `placeholder_text` in the form `[image: WxH FORMAT SIZE]`. Never silently drop an image or emit raw binary to a non-supporting terminal.

### 3. Respect column and row limits
Kitty's `c=` (columns) and `r=` (rows) parameters control the cell grid the image occupies. Default to `max_width_cols = 80` and `max_height_rows = 24`. For full-screen panels, query `crossterm::terminal::size()` and pass the live dimensions via `RenderOptions`.

### 4. Kitty vs iTerm2 protocol differences
| Property | Kitty | iTerm2 |
|---|---|---|
| Escape wrapper | APC (`\x1b_G…\x1b\\`) | OSC 1337 (`\x1b]1337;…\x07`) |
| Size hint | columns × rows (cell grid) | pixels |
| Chunking | Supported (`m=1` / `m=0`) | Not applicable |
| Animation | Yes (via `a=f`) | GIF only |
| Raw format | Any (PNG recommended, `f=100`) | Any; file extension inferred |

For large images (> 256 KB) on Kitty, split into chunks using `m=1` for all but the final chunk and `m=0` on the last. This module's `kitty_escape` emits a single-chunk payload (`m=0`) suitable for images up to ~1 MB.

### 5. SVG handling requires rasterisation before display
`ImageFormat::Svg` is a vector format; Kitty and iTerm2 expect raster data. When `format == ImageFormat::Svg`:
- If an external rasteriser (e.g., `resvg`, `inkscape --pipe`) is available, pipe the SVG through it before calling `render_image_bytes`.
- Otherwise fall back to the text placeholder; never pass raw SVG bytes to a terminal protocol that expects PNG/JPEG.

### 6. Dimension parsing does not require an image crate
PNG: read 4-byte BE width/height from IHDR at offset 16/20.
JPEG: scan for SOF markers (`0xFFC0`–`0xFFCB`) and read height/width from the frame header.
GIF: read 2-byte LE width/height at bytes 6 and 8.
If parsing fails, use `(0, 0)` — the placeholder will still render correctly and the terminal protocol will fall back to auto-sizing.

## Commands
- `ImageProtocol::detect()` — detect terminal capability from env vars
- `render_image_bytes(data, opts)` — render raw bytes; returns `RenderResult`
- `render_image_file(path, opts)` — read file then render; returns `Result<RenderResult, String>`
- `parse_image_dimensions(data)` — extract (width, height) from magic bytes
- `kitty_escape(data, cols, rows)` — low-level Kitty APC sequence builder
- `iterm2_escape(data, width_px, height_px)` — low-level OSC 1337 sequence builder
- `image_placeholder(meta, path_hint)` — text description for unsupported terminals
- `RenderResult::output()` — returns escape sequence or placeholder, whichever is appropriate
- `RenderResult::is_visual()` — `true` when a real visual sequence was produced

## Examples
```rust
use vibecli_cli::tui_images::{render_image_bytes, RenderOptions, ImageProtocol};

// Auto-detect and render a PNG to stdout.
let data = std::fs::read("screenshot.png").unwrap();
let opts = RenderOptions::default(); // auto-detects protocol
let result = render_image_bytes(&data, &opts);
print!("{}", result.output()); // visual or placeholder

// Force iTerm2 regardless of environment.
let opts = RenderOptions {
    protocol: ImageProtocol::ITerm2,
    max_width_cols: 120,
    max_height_rows: 40,
    ..Default::default()
};
let result = render_image_bytes(&data, &opts);
if result.is_visual() {
    print!("{}", result.escape_sequence);
} else {
    println!("{}", result.placeholder_text);
}
```
