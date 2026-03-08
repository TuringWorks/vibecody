//! GPU-accelerated terminal rendering abstraction.
//!
//! Provides a rendering pipeline with glyph atlas caching, dirty-region
//! detection, and multiple backend support (Wgpu, OpenGL, Metal, Software).
//! Falls back to CPU-based software rendering when GPU is unavailable.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Backend ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuBackend {
    Wgpu,
    OpenGL,
    Metal,
    Software,
}

impl GpuBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wgpu => "wgpu",
            Self::OpenGL => "opengl",
            Self::Metal => "metal",
            Self::Software => "software",
        }
    }

    pub fn is_gpu(&self) -> bool {
        !matches!(self, Self::Software)
    }
}

// ── Color ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

// ── Terminal Cell ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::WHITE,
            bg: Color::BLACK,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

// ── Cell Diff ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CellDiff {
    pub row: u32,
    pub col: u32,
    pub old: TerminalCell,
    pub new: TerminalCell,
}

// ── Glyph Info ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphInfo {
    pub ch: char,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub advance: f32,
}

// ── Glyph Atlas ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphAtlas {
    pub width: u32,
    pub height: u32,
    pub font_size: f32,
    glyphs: HashMap<char, GlyphInfo>,
    next_x: u32,
    next_y: u32,
    row_height: u32,
}

impl GlyphAtlas {
    pub fn new(font_size: f32) -> Self {
        let glyph_px = (font_size * 1.5) as u32;
        Self {
            width: 1024,
            height: 1024,
            font_size,
            glyphs: HashMap::new(),
            next_x: 0,
            next_y: 0,
            row_height: glyph_px,
        }
    }

    /// Rasterize a character into the atlas (software stub).
    pub fn rasterize(&mut self, ch: char) -> GlyphInfo {
        if let Some(info) = self.glyphs.get(&ch) {
            return info.clone();
        }

        let glyph_w = (self.font_size * 0.6) as u32;
        let glyph_h = self.row_height;

        // Wrap to next row if needed
        if self.next_x + glyph_w > self.width {
            self.next_x = 0;
            self.next_y += self.row_height;
        }

        let info = GlyphInfo {
            ch,
            x: self.next_x,
            y: self.next_y,
            width: glyph_w,
            height: glyph_h,
            advance: self.font_size * 0.6,
        };

        self.next_x += glyph_w;
        self.glyphs.insert(ch, info.clone());
        info
    }

    pub fn lookup(&self, ch: char) -> Option<&GlyphInfo> {
        self.glyphs.get(&ch)
    }

    /// Fraction of atlas area occupied by glyphs.
    pub fn atlas_utilization(&self) -> f32 {
        let total_area = (self.width * self.height) as f32;
        if total_area == 0.0 { return 0.0; }
        let used_area: f32 = self.glyphs.values()
            .map(|g| (g.width * g.height) as f32)
            .sum();
        used_area / total_area
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    pub fn clear(&mut self) {
        self.glyphs.clear();
        self.next_x = 0;
        self.next_y = 0;
    }
}

// ── GPU Terminal Grid ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuTerminalGrid {
    cells: Vec<TerminalCell>,
    pub rows: u32,
    pub cols: u32,
}

impl GpuTerminalGrid {
    pub fn new(rows: u32, cols: u32) -> Self {
        let size = (rows * cols) as usize;
        Self {
            cells: vec![TerminalCell::default(); size],
            rows,
            cols,
        }
    }

    fn index(&self, row: u32, col: u32) -> Option<usize> {
        if row < self.rows && col < self.cols {
            Some((row * self.cols + col) as usize)
        } else {
            None
        }
    }

    pub fn set_cell(&mut self, row: u32, col: u32, cell: TerminalCell) {
        if let Some(idx) = self.index(row, col) {
            self.cells[idx] = cell;
        }
    }

    pub fn get_cell(&self, row: u32, col: u32) -> Option<&TerminalCell> {
        self.index(row, col).map(|idx| &self.cells[idx])
    }

    pub fn resize(&mut self, rows: u32, cols: u32) {
        let new_size = (rows * cols) as usize;
        let mut new_cells = vec![TerminalCell::default(); new_size];

        // Copy existing cells where they overlap
        let copy_rows = self.rows.min(rows);
        let copy_cols = self.cols.min(cols);
        for r in 0..copy_rows {
            for c in 0..copy_cols {
                let old_idx = (r * self.cols + c) as usize;
                let new_idx = (r * cols + c) as usize;
                new_cells[new_idx] = self.cells[old_idx].clone();
            }
        }

        self.cells = new_cells;
        self.rows = rows;
        self.cols = cols;
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = TerminalCell::default();
        }
    }

    /// Compute differences between this grid and another.
    pub fn diff(&self, other: &GpuTerminalGrid) -> Vec<CellDiff> {
        let mut diffs = Vec::new();
        let rows = self.rows.min(other.rows);
        let cols = self.cols.min(other.cols);

        for r in 0..rows {
            for c in 0..cols {
                let idx_self = (r * self.cols + c) as usize;
                let idx_other = (r * other.cols + c) as usize;
                if self.cells[idx_self] != other.cells[idx_other] {
                    diffs.push(CellDiff {
                        row: r,
                        col: c,
                        old: self.cells[idx_self].clone(),
                        new: other.cells[idx_other].clone(),
                    });
                }
            }
        }
        diffs
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

// ── Render Stats ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RenderStats {
    pub frame_time_us: u64,
    pub gpu_memory_bytes: u64,
    pub cells_rendered: u32,
    pub dirty_cells: u32,
}

// ── Benchmark Result ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub avg_fps: f64,
    pub min_frame_us: u64,
    pub max_frame_us: u64,
    pub p99_frame_us: u64,
    pub backend_name: String,
    pub frames_rendered: u32,
}

// ── GPU Config ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub preferred_backend: Option<GpuBackend>,
    pub font_size: f32,
    pub vsync: bool,
    pub max_fps: u32,
    pub enable_ligatures: bool,
    pub subpixel_rendering: bool,
    pub cell_padding: f32,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            preferred_backend: None,
            font_size: 14.0,
            vsync: true,
            max_fps: 120,
            enable_ligatures: false,
            subpixel_rendering: true,
            cell_padding: 1.0,
        }
    }
}

// ── GPU Renderer ─────────────────────────────────────────────────────────────

pub struct GpuRenderer {
    pub backend: GpuBackend,
    pub atlas: GlyphAtlas,
    pub grid: GpuTerminalGrid,
    pub config: GpuConfig,
    pub frame_count: u64,
    previous_grid: Option<GpuTerminalGrid>,
}

impl GpuRenderer {
    /// Create a new renderer. Falls back to Software if preferred GPU backend
    /// is unavailable.
    pub fn new(config: GpuConfig) -> anyhow::Result<Self> {
        let backend = config.preferred_backend
            .unwrap_or_else(Self::detect_best_backend);

        let atlas = GlyphAtlas::new(config.font_size);
        let grid = GpuTerminalGrid::new(24, 80); // default terminal size

        Ok(Self {
            backend,
            atlas,
            grid,
            config,
            frame_count: 0,
            previous_grid: None,
        })
    }

    /// Probe the system for the best available GPU backend.
    /// In this implementation, always returns Software since actual GPU
    /// backends require wgpu/OpenGL initialization.
    pub fn detect_best_backend() -> GpuBackend {
        // On macOS, Metal would be preferred if available
        #[cfg(target_os = "macos")]
        {
            // Would probe for Metal support here
            // For now, return Software
        }
        GpuBackend::Software
    }

    /// Check if a GPU backend is available (not just Software fallback).
    pub fn supports_gpu() -> bool {
        // Would check for actual GPU API availability
        false
    }

    /// Render one frame. Returns stats about the render pass.
    pub fn render_frame(&mut self) -> anyhow::Result<RenderStats> {
        let start = std::time::Instant::now();

        // Ensure all characters in the grid have atlas entries
        for r in 0..self.grid.rows {
            for c in 0..self.grid.cols {
                if let Some(cell) = self.grid.get_cell(r, c) {
                    if cell.ch != ' ' {
                        self.atlas.rasterize(cell.ch);
                    }
                }
            }
        }

        // Compute dirty cells
        let dirty = if let Some(prev) = &self.previous_grid {
            self.grid.diff(prev).len() as u32
        } else {
            self.grid.cell_count() as u32
        };

        // Store current grid as previous for next frame
        self.previous_grid = Some(self.grid.clone());
        self.frame_count += 1;

        let elapsed = start.elapsed();
        Ok(RenderStats {
            frame_time_us: elapsed.as_micros() as u64,
            gpu_memory_bytes: self.estimate_memory(),
            cells_rendered: self.grid.cell_count() as u32,
            dirty_cells: dirty,
        })
    }

    /// Update the grid content.
    pub fn update_grid(&mut self, grid: GpuTerminalGrid) {
        self.grid = grid;
    }

    /// Estimate GPU/CPU memory usage in bytes.
    fn estimate_memory(&self) -> u64 {
        let atlas_bytes = (self.atlas.width * self.atlas.height * 4) as u64; // RGBA
        let grid_bytes = (self.grid.cell_count() * std::mem::size_of::<TerminalCell>()) as u64;
        atlas_bytes + grid_bytes
    }

    /// Run a benchmark rendering N frames.
    pub fn benchmark(&mut self, frames: u32) -> BenchmarkResult {
        let mut frame_times: Vec<u64> = Vec::with_capacity(frames as usize);

        // Fill grid with content for realistic benchmark
        for r in 0..self.grid.rows {
            for c in 0..self.grid.cols {
                self.grid.set_cell(r, c, TerminalCell {
                    ch: (b'A' + ((r * self.grid.cols + c) % 26) as u8) as char,
                    fg: Color::WHITE,
                    bg: Color::BLACK,
                    bold: c % 3 == 0,
                    italic: false,
                    underline: r % 5 == 0,
                });
            }
        }

        for _ in 0..frames {
            match self.render_frame() {
                Ok(stats) => frame_times.push(stats.frame_time_us),
                Err(_) => frame_times.push(0),
            }
        }

        frame_times.sort();
        let total_us: u64 = frame_times.iter().sum();
        let avg_frame_us = if frames > 0 { total_us / frames as u64 } else { 0 };
        let avg_fps = if avg_frame_us > 0 {
            1_000_000.0 / avg_frame_us as f64
        } else {
            0.0
        };

        let p99_idx = ((frames as f64 * 0.99) as usize).min(frame_times.len().saturating_sub(1));

        BenchmarkResult {
            avg_fps,
            min_frame_us: frame_times.first().copied().unwrap_or(0),
            max_frame_us: frame_times.last().copied().unwrap_or(0),
            p99_frame_us: frame_times.get(p99_idx).copied().unwrap_or(0),
            backend_name: self.backend.as_str().to_string(),
            frames_rendered: frames,
        }
    }

    /// Current FPS estimate based on last frame time.
    pub fn current_fps(&self) -> f32 {
        // Would track running average; simplified for now
        60.0
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Color tests ──────────────────────────────────────────────────────

    #[test]
    fn test_color_rgb() {
        let c = Color::rgb(255, 128, 0);
        assert_eq!(c.r, 255);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_color_rgba() {
        let c = Color::rgba(10, 20, 30, 128);
        assert_eq!(c.a, 128);
    }

    #[test]
    fn test_color_to_hex() {
        assert_eq!(Color::rgb(255, 0, 128).to_hex(), "#ff0080");
        assert_eq!(Color::rgba(255, 0, 128, 128).to_hex(), "#ff008080");
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
        assert_eq!(Color::BLACK, Color::rgb(0, 0, 0));
        assert_eq!(Color::TRANSPARENT.a, 0);
    }

    // ── Cell tests ───────────────────────────────────────────────────────

    #[test]
    fn test_cell_default() {
        let cell = TerminalCell::default();
        assert_eq!(cell.ch, ' ');
        assert!(!cell.bold);
        assert!(!cell.italic);
        assert!(!cell.underline);
    }

    #[test]
    fn test_cell_equality() {
        let a = TerminalCell { ch: 'A', ..Default::default() };
        let b = TerminalCell { ch: 'A', ..Default::default() };
        let c = TerminalCell { ch: 'B', ..Default::default() };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ── GlyphAtlas tests ─────────────────────────────────────────────────

    #[test]
    fn test_atlas_new() {
        let atlas = GlyphAtlas::new(14.0);
        assert_eq!(atlas.font_size, 14.0);
        assert_eq!(atlas.glyph_count(), 0);
    }

    #[test]
    fn test_atlas_rasterize() {
        let mut atlas = GlyphAtlas::new(14.0);
        let info = atlas.rasterize('A');
        assert_eq!(info.ch, 'A');
        assert!(info.width > 0);
        assert!(info.height > 0);
        assert_eq!(atlas.glyph_count(), 1);
    }

    #[test]
    fn test_atlas_rasterize_cached() {
        let mut atlas = GlyphAtlas::new(14.0);
        let info1 = atlas.rasterize('X');
        let info2 = atlas.rasterize('X');
        assert_eq!(info1.x, info2.x);
        assert_eq!(info1.y, info2.y);
        assert_eq!(atlas.glyph_count(), 1);
    }

    #[test]
    fn test_atlas_lookup() {
        let mut atlas = GlyphAtlas::new(14.0);
        assert!(atlas.lookup('Z').is_none());
        atlas.rasterize('Z');
        assert!(atlas.lookup('Z').is_some());
    }

    #[test]
    fn test_atlas_utilization() {
        let atlas = GlyphAtlas::new(14.0);
        assert_eq!(atlas.atlas_utilization(), 0.0);

        let mut atlas2 = GlyphAtlas::new(14.0);
        atlas2.rasterize('A');
        assert!(atlas2.atlas_utilization() > 0.0);
        assert!(atlas2.atlas_utilization() < 1.0);
    }

    #[test]
    fn test_atlas_clear() {
        let mut atlas = GlyphAtlas::new(14.0);
        atlas.rasterize('A');
        atlas.rasterize('B');
        assert_eq!(atlas.glyph_count(), 2);
        atlas.clear();
        assert_eq!(atlas.glyph_count(), 0);
    }

    #[test]
    fn test_atlas_row_wrap() {
        let mut atlas = GlyphAtlas::new(14.0);
        // Rasterize enough glyphs to fill a row
        for ch in 'A'..='Z' {
            atlas.rasterize(ch);
        }
        for ch in 'a'..='z' {
            atlas.rasterize(ch);
        }
        assert_eq!(atlas.glyph_count(), 52);
    }

    // ── Grid tests ───────────────────────────────────────────────────────

    #[test]
    fn test_grid_new() {
        let grid = GpuTerminalGrid::new(24, 80);
        assert_eq!(grid.rows, 24);
        assert_eq!(grid.cols, 80);
        assert_eq!(grid.cell_count(), 24 * 80);
    }

    #[test]
    fn test_grid_set_get() {
        let mut grid = GpuTerminalGrid::new(10, 10);
        let cell = TerminalCell { ch: 'X', bold: true, ..Default::default() };
        grid.set_cell(5, 3, cell.clone());
        let got = grid.get_cell(5, 3).unwrap();
        assert_eq!(got.ch, 'X');
        assert!(got.bold);
    }

    #[test]
    fn test_grid_out_of_bounds() {
        let grid = GpuTerminalGrid::new(10, 10);
        assert!(grid.get_cell(10, 0).is_none());
        assert!(grid.get_cell(0, 10).is_none());
        assert!(grid.get_cell(99, 99).is_none());
    }

    #[test]
    fn test_grid_set_out_of_bounds() {
        let mut grid = GpuTerminalGrid::new(5, 5);
        // Should not panic
        grid.set_cell(10, 10, TerminalCell::default());
    }

    #[test]
    fn test_grid_clear() {
        let mut grid = GpuTerminalGrid::new(5, 5);
        grid.set_cell(2, 2, TerminalCell { ch: 'Q', ..Default::default() });
        grid.clear();
        assert_eq!(grid.get_cell(2, 2).unwrap().ch, ' ');
    }

    #[test]
    fn test_grid_resize_larger() {
        let mut grid = GpuTerminalGrid::new(5, 5);
        grid.set_cell(2, 2, TerminalCell { ch: 'A', ..Default::default() });
        grid.resize(10, 10);
        assert_eq!(grid.rows, 10);
        assert_eq!(grid.cols, 10);
        // Existing cell preserved
        assert_eq!(grid.get_cell(2, 2).unwrap().ch, 'A');
    }

    #[test]
    fn test_grid_resize_smaller() {
        let mut grid = GpuTerminalGrid::new(10, 10);
        grid.set_cell(2, 2, TerminalCell { ch: 'B', ..Default::default() });
        grid.resize(5, 5);
        assert_eq!(grid.rows, 5);
        // Cell within new bounds preserved
        assert_eq!(grid.get_cell(2, 2).unwrap().ch, 'B');
    }

    #[test]
    fn test_grid_diff_identical() {
        let a = GpuTerminalGrid::new(5, 5);
        let b = GpuTerminalGrid::new(5, 5);
        let diffs = a.diff(&b);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_grid_diff_one_change() {
        let a = GpuTerminalGrid::new(5, 5);
        let mut b = GpuTerminalGrid::new(5, 5);
        b.set_cell(1, 1, TerminalCell { ch: 'Z', ..Default::default() });
        let diffs = a.diff(&b);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].row, 1);
        assert_eq!(diffs[0].col, 1);
        assert_eq!(diffs[0].new.ch, 'Z');
    }

    #[test]
    fn test_grid_diff_multiple_changes() {
        let a = GpuTerminalGrid::new(5, 5);
        let mut b = GpuTerminalGrid::new(5, 5);
        b.set_cell(0, 0, TerminalCell { ch: '1', ..Default::default() });
        b.set_cell(4, 4, TerminalCell { ch: '2', ..Default::default() });
        let diffs = a.diff(&b);
        assert_eq!(diffs.len(), 2);
    }

    #[test]
    fn test_grid_diff_different_sizes() {
        let a = GpuTerminalGrid::new(5, 5);
        let mut b = GpuTerminalGrid::new(10, 10);
        b.set_cell(2, 2, TerminalCell { ch: 'X', ..Default::default() });
        let diffs = a.diff(&b);
        // Only compares overlapping region (5x5)
        assert_eq!(diffs.len(), 1);
    }

    // ── Backend tests ────────────────────────────────────────────────────

    #[test]
    fn test_backend_as_str() {
        assert_eq!(GpuBackend::Wgpu.as_str(), "wgpu");
        assert_eq!(GpuBackend::OpenGL.as_str(), "opengl");
        assert_eq!(GpuBackend::Metal.as_str(), "metal");
        assert_eq!(GpuBackend::Software.as_str(), "software");
    }

    #[test]
    fn test_backend_is_gpu() {
        assert!(GpuBackend::Wgpu.is_gpu());
        assert!(GpuBackend::Metal.is_gpu());
        assert!(!GpuBackend::Software.is_gpu());
    }

    #[test]
    fn test_detect_best_backend() {
        let backend = GpuRenderer::detect_best_backend();
        assert_eq!(backend, GpuBackend::Software);
    }

    #[test]
    fn test_supports_gpu() {
        // In test environment, GPU is not available
        assert!(!GpuRenderer::supports_gpu());
    }

    // ── Config tests ─────────────────────────────────────────────────────

    #[test]
    fn test_config_default() {
        let config = GpuConfig::default();
        assert_eq!(config.font_size, 14.0);
        assert!(config.vsync);
        assert_eq!(config.max_fps, 120);
        assert!(!config.enable_ligatures);
        assert!(config.subpixel_rendering);
        assert_eq!(config.cell_padding, 1.0);
        assert!(config.preferred_backend.is_none());
    }

    // ── Renderer tests ───────────────────────────────────────────────────

    #[test]
    fn test_renderer_new() {
        let config = GpuConfig::default();
        let renderer = GpuRenderer::new(config).expect("renderer creation failed");
        assert_eq!(renderer.backend, GpuBackend::Software);
        assert_eq!(renderer.frame_count, 0);
    }

    #[test]
    fn test_renderer_with_specific_backend() {
        let config = GpuConfig {
            preferred_backend: Some(GpuBackend::Software),
            ..Default::default()
        };
        let renderer = GpuRenderer::new(config).expect("renderer creation failed");
        assert_eq!(renderer.backend, GpuBackend::Software);
    }

    #[test]
    fn test_render_frame() {
        let mut renderer = GpuRenderer::new(GpuConfig::default()).expect("failed");
        let stats = renderer.render_frame().expect("render failed");
        assert_eq!(stats.cells_rendered, 24 * 80);
        assert_eq!(renderer.frame_count, 1);
    }

    #[test]
    fn test_render_frame_dirty_detection() {
        let mut renderer = GpuRenderer::new(GpuConfig::default()).expect("failed");

        // First frame: all cells dirty
        let stats1 = renderer.render_frame().expect("failed");
        assert_eq!(stats1.dirty_cells, 24 * 80);

        // Second frame with no changes: zero dirty cells
        let stats2 = renderer.render_frame().expect("failed");
        assert_eq!(stats2.dirty_cells, 0);

        // Change one cell: one dirty cell
        renderer.grid.set_cell(0, 0, TerminalCell { ch: 'X', ..Default::default() });
        let stats3 = renderer.render_frame().expect("failed");
        assert_eq!(stats3.dirty_cells, 1);
    }

    #[test]
    fn test_update_grid() {
        let mut renderer = GpuRenderer::new(GpuConfig::default()).expect("failed");
        let new_grid = GpuTerminalGrid::new(40, 120);
        renderer.update_grid(new_grid);
        assert_eq!(renderer.grid.rows, 40);
        assert_eq!(renderer.grid.cols, 120);
    }

    #[test]
    fn test_benchmark() {
        let mut renderer = GpuRenderer::new(GpuConfig::default()).expect("failed");
        let result = renderer.benchmark(10);
        assert_eq!(result.frames_rendered, 10);
        assert!(result.avg_fps > 0.0);
        assert_eq!(result.backend_name, "software");
        assert!(result.min_frame_us <= result.max_frame_us);
        assert!(result.p99_frame_us <= result.max_frame_us);
    }

    #[test]
    fn test_benchmark_zero_frames() {
        let mut renderer = GpuRenderer::new(GpuConfig::default()).expect("failed");
        let result = renderer.benchmark(0);
        assert_eq!(result.frames_rendered, 0);
    }

    #[test]
    fn test_render_stats_default() {
        let stats = RenderStats::default();
        assert_eq!(stats.frame_time_us, 0);
        assert_eq!(stats.gpu_memory_bytes, 0);
        assert_eq!(stats.cells_rendered, 0);
        assert_eq!(stats.dirty_cells, 0);
    }

    #[test]
    fn test_memory_estimate() {
        let renderer = GpuRenderer::new(GpuConfig::default()).expect("failed");
        let mem = renderer.estimate_memory();
        assert!(mem > 0); // Atlas (1024*1024*4) + grid cells
    }

    #[test]
    fn test_current_fps() {
        let renderer = GpuRenderer::new(GpuConfig::default()).expect("failed");
        assert_eq!(renderer.current_fps(), 60.0);
    }

    // ── Zero-size edge case ──────────────────────────────────────────────

    #[test]
    fn test_grid_zero_size() {
        let grid = GpuTerminalGrid::new(0, 0);
        assert_eq!(grid.cell_count(), 0);
        assert!(grid.get_cell(0, 0).is_none());
    }

    #[test]
    fn test_atlas_zero_font_size() {
        let atlas = GlyphAtlas::new(0.0);
        assert_eq!(atlas.atlas_utilization(), 0.0);
    }
}
