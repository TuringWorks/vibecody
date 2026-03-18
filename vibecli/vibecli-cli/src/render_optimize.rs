
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderLine {
    pub content: String,
    pub style_hash: u64,
    pub dirty: bool,
}

impl RenderLine {
    pub fn new(content: &str, style_hash: u64) -> Self {
        Self {
            content: content.to_string(),
            style_hash,
            dirty: false,
        }
    }

    /// Fast identity hash combining content + style.
    fn identity_hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.content.hash(&mut h);
        self.style_hash.hash(&mut h);
        h.finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderFrame {
    pub lines: Vec<RenderLine>,
    pub width: usize,
    pub height: usize,
    pub cursor_pos: (usize, usize),
}

impl RenderFrame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            lines: Vec::new(),
            width,
            height,
            cursor_pos: (0, 0),
        }
    }

    fn line_hashes(&self) -> Vec<u64> {
        self.lines.iter().map(|l| l.identity_hash()).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub changed_lines: Vec<usize>,
    pub total_lines: usize,
    pub unchanged_lines: usize,
    pub reduction_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirtyRegion {
    pub start_line: usize,
    pub end_line: usize,
    pub lines: Vec<RenderLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedFrame {
    pub regions: Vec<DirtyRegion>,
    pub total_lines: usize,
    pub rendered_lines: usize,
    pub reduction_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub avg_reduction_pct: f64,
    pub total_frames: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderCache {
    pub previous_frame: Option<RenderFrame>,
    pub hit_count: u64,
    pub miss_count: u64,
}

impl RenderCache {
    fn new() -> Self {
        Self {
            previous_frame: None,
            hit_count: 0,
            miss_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// RenderOptimizer
// ---------------------------------------------------------------------------

pub struct RenderOptimizer {
    width: usize,
    height: usize,
    cache: RenderCache,
    force_full: bool,
    reduction_history: Vec<f64>,
    /// Pre-computed hashes for the cached frame, keyed by line index.
    hash_cache: HashMap<usize, u64>,
}

impl RenderOptimizer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cache: RenderCache::new(),
            force_full: false,
            reduction_history: Vec::new(),
            hash_cache: HashMap::new(),
        }
    }

    // -- Diff computation ---------------------------------------------------

    pub fn compute_diff(old_frame: &RenderFrame, new_frame: &RenderFrame) -> DiffResult {
        let old_hashes = old_frame.line_hashes();
        let new_hashes = new_frame.line_hashes();

        let total_lines = new_hashes.len();
        let mut changed_lines = Vec::new();

        for (i, new_hash) in new_hashes.iter().enumerate() {
            let changed = match old_hashes.get(i) {
                Some(oh) => oh != new_hash,
                None => true, // new line has no old counterpart
            };
            if changed {
                changed_lines.push(i);
            }
        }
        // Lines that existed in old but not in new are implicitly "removed".
        // We also count lines beyond old len as changed (already handled above).

        let unchanged_lines = total_lines.saturating_sub(changed_lines.len());
        let reduction_pct = if total_lines == 0 {
            100.0
        } else {
            (unchanged_lines as f64 / total_lines as f64) * 100.0
        };

        DiffResult {
            changed_lines,
            total_lines,
            unchanged_lines,
            reduction_pct,
        }
    }

    // -- Cache-aware queries ------------------------------------------------

    pub fn should_rerender(&mut self, new_frame: &RenderFrame) -> bool {
        if self.force_full {
            self.cache.miss_count += 1;
            return true;
        }

        match &self.cache.previous_frame {
            None => {
                self.cache.miss_count += 1;
                true
            }
            Some(old) => {
                let diff = Self::compute_diff(old, new_frame);
                if diff.changed_lines.is_empty()
                    && old.width == new_frame.width
                    && old.height == new_frame.height
                    && old.cursor_pos == new_frame.cursor_pos
                {
                    self.cache.hit_count += 1;
                    false
                } else {
                    self.cache.miss_count += 1;
                    true
                }
            }
        }
    }

    pub fn get_dirty_lines(&self, new_frame: &RenderFrame) -> Vec<usize> {
        if self.force_full {
            return (0..new_frame.lines.len()).collect();
        }
        match &self.cache.previous_frame {
            None => (0..new_frame.lines.len()).collect(),
            Some(old) => {
                let diff = Self::compute_diff(old, new_frame);
                diff.changed_lines
            }
        }
    }

    // -- Cache management ---------------------------------------------------

    pub fn update_cache(&mut self, frame: &RenderFrame) {
        // Rebuild hash cache for fast future lookups.
        self.hash_cache.clear();
        for (i, line) in frame.lines.iter().enumerate() {
            self.hash_cache.insert(i, line.identity_hash());
        }

        if let Some(old) = &self.cache.previous_frame {
            let diff = Self::compute_diff(old, frame);
            self.reduction_history.push(diff.reduction_pct);
        }

        self.cache.previous_frame = Some(frame.clone());
        self.force_full = false;
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let total = self.cache.hit_count + self.cache.miss_count;
        let hit_rate = if total == 0 {
            0.0
        } else {
            self.cache.hit_count as f64 / total as f64
        };
        let avg_reduction_pct = if self.reduction_history.is_empty() {
            0.0
        } else {
            self.reduction_history.iter().sum::<f64>() / self.reduction_history.len() as f64
        };
        CacheStats {
            hits: self.cache.hit_count,
            misses: self.cache.miss_count,
            hit_rate,
            avg_reduction_pct,
            total_frames: total,
        }
    }

    pub fn set_full_rerender(&mut self) {
        self.force_full = true;
    }

    // -- Frame optimization -------------------------------------------------

    pub fn optimize_frame(&self, frame: &RenderFrame) -> OptimizedFrame {
        let dirty_indices = self.get_dirty_lines(frame);
        let total_lines = frame.lines.len();
        let rendered_lines = dirty_indices.len();

        let regions = Self::merge_regions(&dirty_indices, &frame.lines);

        let reduction_pct = if total_lines == 0 {
            100.0
        } else {
            ((total_lines.saturating_sub(rendered_lines)) as f64 / total_lines as f64) * 100.0
        };

        OptimizedFrame {
            regions,
            total_lines,
            rendered_lines,
            reduction_pct,
        }
    }

    /// Merge contiguous dirty line indices into `DirtyRegion`s.
    fn merge_regions(dirty: &[usize], all_lines: &[RenderLine]) -> Vec<DirtyRegion> {
        if dirty.is_empty() {
            return Vec::new();
        }

        let mut regions: Vec<DirtyRegion> = Vec::new();
        let mut start = dirty[0];
        let mut end = dirty[0];

        for &idx in &dirty[1..] {
            if idx == end + 1 {
                end = idx;
            } else {
                regions.push(Self::build_region(start, end, all_lines));
                start = idx;
                end = idx;
            }
        }
        regions.push(Self::build_region(start, end, all_lines));
        regions
    }

    fn build_region(start: usize, end: usize, all_lines: &[RenderLine]) -> DirtyRegion {
        let lines: Vec<RenderLine> = (start..=end)
            .filter_map(|i| all_lines.get(i).cloned())
            .collect();
        DirtyRegion {
            start_line: start,
            end_line: end,
            lines,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(lines: &[&str], width: usize, height: usize) -> RenderFrame {
        let render_lines: Vec<RenderLine> = lines
            .iter()
            .enumerate()
            .map(|(i, s)| RenderLine::new(s, i as u64))
            .collect();
        RenderFrame {
            lines: render_lines,
            width,
            height,
            cursor_pos: (0, 0),
        }
    }

    fn make_frame_styled(entries: &[(&str, u64)], width: usize, height: usize) -> RenderFrame {
        let render_lines: Vec<RenderLine> = entries
            .iter()
            .map(|(s, h)| RenderLine::new(s, *h))
            .collect();
        RenderFrame {
            lines: render_lines,
            width,
            height,
            cursor_pos: (0, 0),
        }
    }

    // -- Diff computation ---------------------------------------------------

    #[test]
    fn test_diff_identical_frames() {
        let f = make_frame(&["hello", "world"], 80, 24);
        let diff = RenderOptimizer::compute_diff(&f, &f.clone());
        assert!(diff.changed_lines.is_empty());
        assert_eq!(diff.unchanged_lines, 2);
        assert!((diff.reduction_pct - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_diff_completely_different() {
        let a = make_frame(&["aaa", "bbb"], 80, 24);
        let b = make_frame(&["xxx", "yyy"], 80, 24);
        let diff = RenderOptimizer::compute_diff(&a, &b);
        assert_eq!(diff.changed_lines.len(), 2);
        assert_eq!(diff.unchanged_lines, 0);
        assert!((diff.reduction_pct - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_diff_partial_change() {
        let a = make_frame_styled(&[("line1", 0), ("line2", 1), ("line3", 2)], 80, 24);
        let mut b = a.clone();
        b.lines[1] = RenderLine::new("CHANGED", 99);
        let diff = RenderOptimizer::compute_diff(&a, &b);
        assert_eq!(diff.changed_lines, vec![1]);
        assert_eq!(diff.unchanged_lines, 2);
    }

    #[test]
    fn test_diff_new_lines_added() {
        let a = make_frame(&["a"], 80, 24);
        let b = make_frame(&["a", "b", "c"], 80, 24);
        let diff = RenderOptimizer::compute_diff(&a, &b);
        // line 0 changed because style_hash differs (index-based), but content "a" same
        // Actually style_hash = index, so a has hash(a,0), b has hash(a,0) — identical for line 0
        assert_eq!(diff.changed_lines, vec![1, 2]);
        assert_eq!(diff.total_lines, 3);
    }

    #[test]
    fn test_diff_empty_frames() {
        let a = RenderFrame::new(80, 24);
        let b = RenderFrame::new(80, 24);
        let diff = RenderOptimizer::compute_diff(&a, &b);
        assert!(diff.changed_lines.is_empty());
        assert_eq!(diff.total_lines, 0);
        assert!((diff.reduction_pct - 100.0).abs() < f64::EPSILON);
    }

    // -- should_rerender ----------------------------------------------------

    #[test]
    fn test_should_rerender_no_cache() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["hello"], 80, 24);
        assert!(opt.should_rerender(&f));
    }

    #[test]
    fn test_should_rerender_identical() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["hello"], 80, 24);
        opt.update_cache(&f);
        assert!(!opt.should_rerender(&f));
    }

    #[test]
    fn test_should_rerender_changed() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f1 = make_frame(&["hello"], 80, 24);
        opt.update_cache(&f1);
        let f2 = make_frame(&["world"], 80, 24);
        assert!(opt.should_rerender(&f2));
    }

    #[test]
    fn test_should_rerender_cursor_moved() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f1 = make_frame(&["hello"], 80, 24);
        opt.update_cache(&f1);
        let mut f2 = f1.clone();
        f2.cursor_pos = (5, 0);
        assert!(opt.should_rerender(&f2));
    }

    // -- get_dirty_lines ----------------------------------------------------

    #[test]
    fn test_dirty_lines_no_cache() {
        let opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["a", "b", "c"], 80, 24);
        let dirty = opt.get_dirty_lines(&f);
        assert_eq!(dirty, vec![0, 1, 2]);
    }

    #[test]
    fn test_dirty_lines_one_change() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f1 = make_frame_styled(&[("a", 0), ("b", 1), ("c", 2)], 80, 24);
        opt.update_cache(&f1);
        let mut f2 = f1.clone();
        f2.lines[2] = RenderLine::new("C", 2);
        let dirty = opt.get_dirty_lines(&f2);
        assert_eq!(dirty, vec![2]);
    }

    // -- Cache stats --------------------------------------------------------

    #[test]
    fn test_cache_stats_initial() {
        let opt = RenderOptimizer::new(80, 24);
        let stats = opt.get_cache_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_frames, 0);
        assert!((stats.hit_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_stats_after_hits_and_misses() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["x"], 80, 24);
        opt.should_rerender(&f); // miss (no cache)
        opt.update_cache(&f);
        opt.should_rerender(&f); // hit
        opt.should_rerender(&f); // hit
        let stats = opt.get_cache_stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_cache_stats_avg_reduction() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f1 = make_frame_styled(&[("a", 0), ("b", 1)], 80, 24);
        opt.update_cache(&f1);
        // Change one of two lines → 50% reduction
        let mut f2 = f1.clone();
        f2.lines[0] = RenderLine::new("X", 99);
        opt.update_cache(&f2);
        let stats = opt.get_cache_stats();
        assert!(stats.avg_reduction_pct > 0.0);
    }

    // -- set_full_rerender --------------------------------------------------

    #[test]
    fn test_full_rerender_flag() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["x"], 80, 24);
        opt.update_cache(&f);
        assert!(!opt.should_rerender(&f));
        opt.set_full_rerender();
        assert!(opt.should_rerender(&f));
    }

    #[test]
    fn test_full_rerender_dirty_lines_all() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["a", "b", "c"], 80, 24);
        opt.update_cache(&f);
        opt.set_full_rerender();
        let dirty = opt.get_dirty_lines(&f);
        assert_eq!(dirty, vec![0, 1, 2]);
    }

    #[test]
    fn test_full_rerender_clears_after_update() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["a"], 80, 24);
        opt.update_cache(&f);
        opt.set_full_rerender();
        opt.update_cache(&f); // clears force_full
        assert!(!opt.should_rerender(&f));
    }

    // -- optimize_frame / region merging ------------------------------------

    #[test]
    fn test_optimize_frame_no_cache_all_dirty() {
        let opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["a", "b", "c"], 80, 24);
        let of = opt.optimize_frame(&f);
        assert_eq!(of.total_lines, 3);
        assert_eq!(of.rendered_lines, 3);
        assert_eq!(of.regions.len(), 1); // all contiguous
        assert_eq!(of.regions[0].start_line, 0);
        assert_eq!(of.regions[0].end_line, 2);
    }

    #[test]
    fn test_optimize_frame_contiguous_merge() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f1 = make_frame_styled(
            &[("a", 0), ("b", 1), ("c", 2), ("d", 3), ("e", 4)],
            80,
            24,
        );
        opt.update_cache(&f1);

        let mut f2 = f1.clone();
        f2.lines[1] = RenderLine::new("B", 99);
        f2.lines[2] = RenderLine::new("C", 98);
        f2.lines[4] = RenderLine::new("E", 97);

        let of = opt.optimize_frame(&f2);
        // dirty: [1, 2, 4] → two regions: [1..2] and [4..4]
        assert_eq!(of.regions.len(), 2);
        assert_eq!(of.regions[0].start_line, 1);
        assert_eq!(of.regions[0].end_line, 2);
        assert_eq!(of.regions[0].lines.len(), 2);
        assert_eq!(of.regions[1].start_line, 4);
        assert_eq!(of.regions[1].end_line, 4);
        assert_eq!(of.rendered_lines, 3);
    }

    #[test]
    fn test_optimize_frame_empty() {
        let opt = RenderOptimizer::new(80, 24);
        let f = RenderFrame::new(80, 24);
        let of = opt.optimize_frame(&f);
        assert_eq!(of.total_lines, 0);
        assert_eq!(of.rendered_lines, 0);
        assert!(of.regions.is_empty());
        assert!((of.reduction_pct - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_optimize_frame_reduction_pct() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f1 = make_frame_styled(
            &[("a", 0), ("b", 1), ("c", 2), ("d", 3)],
            80,
            24,
        );
        opt.update_cache(&f1);
        let mut f2 = f1.clone();
        f2.lines[0] = RenderLine::new("X", 77);
        let of = opt.optimize_frame(&f2);
        // 1 of 4 dirty → 75% reduction
        assert!((of.reduction_pct - 75.0).abs() < f64::EPSILON);
    }

    // -- RenderLine hashing -------------------------------------------------

    #[test]
    fn test_render_line_identity_hash_stable() {
        let a = RenderLine::new("hello", 42);
        let b = RenderLine::new("hello", 42);
        assert_eq!(a.identity_hash(), b.identity_hash());
    }

    #[test]
    fn test_render_line_identity_hash_differs() {
        let a = RenderLine::new("hello", 42);
        let b = RenderLine::new("hello", 43);
        assert_ne!(a.identity_hash(), b.identity_hash());
    }

    // -- Edge cases ---------------------------------------------------------

    #[test]
    fn test_single_line_frame() {
        let mut opt = RenderOptimizer::new(80, 24);
        let f = make_frame(&["only"], 80, 24);
        opt.update_cache(&f);
        assert!(!opt.should_rerender(&f));
        let dirty = opt.get_dirty_lines(&f);
        assert!(dirty.is_empty());
    }

    #[test]
    fn test_frame_shrinks() {
        let a = make_frame_styled(&[("a", 0), ("b", 1), ("c", 2)], 80, 24);
        let b = make_frame_styled(&[("a", 0)], 80, 24);
        let diff = RenderOptimizer::compute_diff(&a, &b);
        assert_eq!(diff.total_lines, 1);
        assert!(diff.changed_lines.is_empty());
        assert_eq!(diff.unchanged_lines, 1);
    }
}
