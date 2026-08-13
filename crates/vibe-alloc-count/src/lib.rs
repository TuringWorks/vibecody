//! A counting global allocator, so allocation claims are **measured**.
//!
//! AGENTS.md ranks allocation last among performance wins, and for a reason:
//! it is the easiest thing to "optimise" without evidence. This crate exists so
//! that a claim like "this path no longer allocates per chunk" is a number a
//! test can assert on, and a regression is a failing test rather than a slow
//! afternoon six months later.
//!
//! ```ignore
//! use vibe_alloc_count::{CountingAllocator, measure};
//!
//! #[global_allocator]
//! static ALLOC: CountingAllocator = CountingAllocator::new();
//!
//! #[test]
//! fn parsing_a_chunk_does_not_recompile_regexes() {
//!     let first = measure(|| parse_tool_calls(CHUNK));
//!     let second = measure(|| parse_tool_calls(CHUNK));
//!     // Steady-state cost, after any one-time lazy init.
//!     assert!(second.allocations < 100, "{second:?}");
//! }
//! ```
//!
//! ## Caveats, so the numbers are read honestly
//!
//! * **A binary has exactly one global allocator.** Only one crate in a test
//!   binary may install this, so measurement lives in dedicated test targets.
//! * **Counts are process-wide**, not per-thread. Measure on a quiet thread, or
//!   accept that background tokio work is included in the number.
//! * **The first call to a lazily-initialised path is not the steady state.**
//!   A `LazyLock<Regex>` allocates once, on whichever call happens to be first.
//!   Measure the *second* call when you care about per-unit cost, which is what
//!   [`measure_steady_state`] does for you.
//! * This counts calls to the allocator, not peak RSS. A single 10 MB
//!   allocation counts as one allocation. Use `bytes_allocated` for footprint.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static BYTES_ALLOCATED: AtomicU64 = AtomicU64::new(0);

/// A `System` allocator that counts what passes through it.
///
/// Overhead is two relaxed atomic adds per allocation, so it is fine in a test
/// or bench build. Do not ship it as the allocator of a release binary.
pub struct CountingAllocator;

impl CountingAllocator {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CountingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: every method forwards to `System`, which upholds the `GlobalAlloc`
// contract; the only additions are relaxed counter updates, which cannot
// affect memory validity. `Relaxed` is sufficient because the counters are
// statistics, never used to guard access to memory.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // A realloc that grows in place still copies in the general case, and
        // is the signature of a `Vec` that was built without `with_capacity`.
        // Counting it as an allocation is what makes that visible.
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        BYTES_ALLOCATED.fetch_add(
            new_size.saturating_sub(layout.size()) as u64,
            Ordering::Relaxed,
        );
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// What happened to the heap during one measured call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllocStats {
    /// Number of `alloc` + `realloc` calls.
    pub allocations: u64,
    /// Number of `dealloc` calls.
    pub deallocations: u64,
    /// Total bytes requested (not peak RSS).
    pub bytes_allocated: u64,
}

impl AllocStats {
    /// Allocations that were not freed during the measured window. Negative
    /// balances (freeing something allocated earlier) clamp to zero, so this
    /// is a hint about retention, not a leak detector.
    pub fn retained(&self) -> u64 {
        self.allocations.saturating_sub(self.deallocations)
    }
}

fn snapshot() -> AllocStats {
    AllocStats {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
        bytes_allocated: BYTES_ALLOCATED.load(Ordering::Relaxed),
    }
}

/// Run `f` and report what it allocated.
///
/// The return value of `f` is dropped *inside* the measured window, so its
/// deallocations are counted. If you need the value, use [`measure_with`].
pub fn measure<T>(f: impl FnOnce() -> T) -> AllocStats {
    let (stats, value) = measure_with(f);
    drop(value);
    stats
}

/// Run `f`, reporting both its allocation stats and its return value.
///
/// The value is still alive when the stats are taken, so its allocations are
/// counted but its deallocations are not — which is the honest reading of
/// "what did producing this cost".
pub fn measure_with<T>(f: impl FnOnce() -> T) -> (AllocStats, T) {
    let before = snapshot();
    let value = f();
    let after = snapshot();
    (
        AllocStats {
            allocations: after.allocations - before.allocations,
            deallocations: after.deallocations - before.deallocations,
            bytes_allocated: after.bytes_allocated - before.bytes_allocated,
        },
        value,
    )
}

/// Measure the *steady-state* cost of `f` by running it twice and reporting the
/// second run.
///
/// Anything behind a `LazyLock`/`OnceLock` initialises on the first call, and
/// counting that against per-call cost overstates it — often by orders of
/// magnitude, which is exactly the mistake that makes an optimisation look like
/// it did nothing. Use this whenever the code under test has lazy statics.
pub fn measure_steady_state<T>(mut f: impl FnMut() -> T) -> AllocStats {
    drop(f());
    measure(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests only exercise the counting logic; the allocator itself is
    // installed by the test targets that measure real code.

    #[test]
    fn retained_never_goes_negative() {
        let s = AllocStats {
            allocations: 2,
            deallocations: 9,
            bytes_allocated: 0,
        };
        assert_eq!(s.retained(), 0);
    }

    #[test]
    fn retained_reports_the_unfreed_balance() {
        let s = AllocStats {
            allocations: 10,
            deallocations: 4,
            bytes_allocated: 0,
        };
        assert_eq!(s.retained(), 6);
    }
}
