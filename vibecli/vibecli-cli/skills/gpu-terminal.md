---
triggers: ["gpu terminal", "GPU rendering", "gpu accelerated", "terminal rendering", "glyph atlas", "wgpu", "frame rate", "terminal performance", "vsync", "subpixel rendering", "ligatures"]
tools_allowed: ["read_file", "write_file", "bash"]
category: terminal
---

# GPU-Accelerated Terminal Rendering

When configuring or optimizing GPU-accelerated terminal rendering:

1. **Select the Right Backend** — VibeCody supports four rendering backends: `Wgpu` (cross-platform, recommended — uses Vulkan/Metal/DX12 under the hood), `OpenGL` (legacy fallback for older hardware), `Metal` (macOS-native, lowest latency on Apple Silicon), and `Software` (CPU-only fallback, always available). Use `detect_best_backend()` to auto-select the optimal backend for the current system. Override via config if needed.

2. **Configure the Glyph Atlas** — The glyph atlas is a texture that caches pre-rasterized font glyphs for instant rendering. Set `font_size` to control rasterization size (default: 14.0). The atlas grows dynamically as new characters are encountered. Monitor `atlas_utilization()` to check how full the atlas is. For CJK or emoji-heavy workloads, consider a larger initial atlas size. Each glyph stores position, dimensions, and advance width for precise text layout.

3. **Understand the Terminal Grid** — The `GpuTerminalGrid` stores terminal cells in a flat row-major array. Each `TerminalCell` contains: character, foreground color (RGBA), background color (RGBA), and style flags (bold, italic, underline). Use `set_cell(row, col, cell)` for single updates and `resize(rows, cols)` when the terminal dimensions change. The grid supports dirty-region detection via `diff()` — only changed cells are re-rendered each frame.

4. **Optimize Frame Rendering** — Enable `vsync: true` to synchronize with display refresh rate and prevent tearing. Set `max_fps` to cap frame rate (default: 120). The renderer uses dirty-region detection to skip unchanged cells, reducing GPU work by 80-95% in typical terminal use. `RenderStats` reports frame time, GPU memory usage, cells rendered, and dirty cell count per frame.

5. **Enable Ligatures and Subpixel Rendering** — Set `enable_ligatures: true` for programming font ligatures (e.g., `->`, `=>`, `!=` rendered as combined glyphs). Enable `subpixel_rendering: true` for sharper text on LCD displays (uses RGB subpixel geometry). Disable subpixel rendering on HiDPI/Retina displays where it's unnecessary. Adjust `cell_padding` (default: 1.0) to control spacing between cells.

6. **Run Benchmarks** — Use `benchmark(frames)` to measure rendering performance. The benchmark renders N frames with full-grid updates and reports: average FPS, min/max/p99 frame times in microseconds, and the backend name. Target: 60+ FPS for smooth scrolling, 120+ FPS for no perceptible lag. Software backend typically achieves 200+ FPS for text-only rendering.

7. **Monitor GPU Memory** — `RenderStats.gpu_memory_bytes` tracks VRAM usage. A typical terminal (120x40) with a 256-glyph atlas uses ~2-4 MB. Heavy Unicode usage or very large terminals may use more. The renderer releases atlas memory when glyphs are evicted.

8. **Handle Fallback Gracefully** — If GPU initialization fails (headless server, SSH session, missing drivers), the renderer automatically falls back to `Software` backend. The `supports_gpu()` method checks for GPU availability without attempting initialization. All rendering APIs are identical regardless of backend — code doesn't need to know which backend is active.

9. **Tune for Different Workloads** — For high-throughput output (build logs, large diffs): maximize `max_fps`, disable ligatures, use larger batch sizes. For interactive editing (TUI apps, Ratatui): enable vsync, enable ligatures, use smaller cell padding. For remote/SSH: use Software backend, disable subpixel rendering, lower max_fps to reduce bandwidth.

10. **Profile Rendering Pipeline** — The rendering pipeline is: grid update → dirty detection → glyph lookup → vertex generation → GPU draw call. Most time is spent in glyph lookup for uncached characters and vertex generation for dirty cells. If frame times spike, check atlas utilization (cache misses) and dirty cell count (excessive redraws).
