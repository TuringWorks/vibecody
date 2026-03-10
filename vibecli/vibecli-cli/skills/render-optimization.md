# Terminal Render Optimization

Optimized TUI rendering with frame diffing, dirty region detection, and cache-based re-render reduction.

## Triggers
- "render optimization", "TUI performance", "re-render reduction"
- "frame diff", "render cache", "dirty region"

## Usage
```
/render stats                             # Show render cache stats
/render force                             # Force full re-render
/render debug                             # Show dirty regions
/render benchmark                         # Benchmark render performance
```

## Features
- Line-level content hashing for fast comparison
- Frame diffing: only re-render changed lines
- Dirty region merging: adjacent dirty lines grouped into contiguous regions
- Render cache with hit/miss tracking
- ~74% re-render reduction through incremental updates
- OptimizedFrame with only changed regions (not full frame)
- Cache statistics: hit rate, average reduction percentage, total frames
- Force full repaint when needed
