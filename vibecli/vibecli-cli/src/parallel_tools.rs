//! Parallel tool executor — concurrent dispatch with preflight hooks.
//! Pi-mono gap bridge: Phase A2.
//!
//! Wraps `parallel_tool_scheduler` (dependency-tracked scheduler) with a
//! higher-level execution layer: sequential preflight → concurrent dispatch →
//! results emitted in original call order.

use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::thread;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Result of a single tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Name of the tool that was invoked.
    pub tool_name: String,
    /// Identifier matching the originating `ToolCall`.
    pub call_id: String,
    /// Output produced by the tool (empty string when blocked).
    pub output: String,
    /// Wall-clock milliseconds spent executing this single tool.
    pub elapsed_ms: u64,
    /// `true` when preflight returned `Block` — tool was never executed.
    pub blocked: bool,
    /// Non-`None` when the tool raised an error during execution.
    pub error: Option<String>,
}

/// Preflight decision returned by a `beforeToolCall` hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightDecision {
    /// Allow the tool call to proceed.
    Allow,
    /// Prevent the tool call with an explanatory reason.
    Block { reason: String },
}

/// A single tool invocation request.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Unique identifier for this call within the assistant turn.
    pub call_id: String,
    /// Name of the tool to invoke (e.g. `"Read"`, `"Bash"`).
    pub tool_name: String,
    /// JSON-serialised arguments for the tool.
    pub args_json: String,
}

impl ToolCall {
    /// Convenience constructor.
    pub fn new(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        args_json: impl Into<String>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            args_json: args_json.into(),
        }
    }
}

/// Execution mode for the dispatcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Default — all allowed tools run concurrently via OS threads.
    Parallel,
    /// Backward-compatible — tools run one after another in call order.
    Sequential,
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Aggregate statistics from a single `dispatch` run.
#[derive(Debug, Clone, Default)]
pub struct DispatchStats {
    /// Total number of calls submitted.
    pub total_calls: usize,
    /// Calls that passed preflight and were executed.
    pub executed: usize,
    /// Calls blocked by preflight.
    pub blocked: usize,
    /// Calls that raised an error during execution.
    pub failed: usize,
    /// Wall-clock milliseconds for the entire dispatch (parallel savings visible here).
    pub wall_time_ms: u64,
    /// Sum of individual `elapsed_ms` values (serial equivalent time).
    pub sum_serial_ms: u64,
}

impl DispatchStats {
    /// Theoretical speedup: `sum_serial_ms / wall_time_ms`.
    ///
    /// Returns `1.0` when `wall_time_ms` is zero to avoid division-by-zero.
    pub fn speedup_ratio(&self) -> f64 {
        if self.wall_time_ms == 0 {
            1.0
        } else {
            self.sum_serial_ms as f64 / self.wall_time_ms as f64
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// High-level tool executor.
///
/// # Lifecycle for one agent turn
/// 1. **Preflight** — all `ToolCall`s are passed sequentially to the
///    `preflight` closure (mirrors `beforeToolCall` hooks).
/// 2. **Dispatch** — calls that were *allowed* execute concurrently (Parallel
///    mode) or in order (Sequential mode).
/// 3. **Ordering** — results are returned in the original call-list order
///    regardless of completion order.
pub struct ParallelToolDispatcher {
    mode: ExecutionMode,
    max_concurrency: usize,
}

impl ParallelToolDispatcher {
    /// Create a dispatcher with the given mode and default concurrency (8).
    pub fn new(mode: ExecutionMode) -> Self {
        Self {
            mode,
            max_concurrency: 8,
        }
    }

    /// Create a dispatcher with explicit concurrency limit.
    pub fn with_concurrency(mode: ExecutionMode, max: usize) -> Self {
        Self {
            mode,
            max_concurrency: max.max(1),
        }
    }

    /// Return the configured execution mode.
    pub fn mode(&self) -> &ExecutionMode {
        &self.mode
    }

    /// Return the configured maximum concurrency.
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    /// Preflight all calls sequentially (beforeToolCall), then execute allowed
    /// ones according to the execution mode, and return results in the
    /// **original call order**.
    ///
    /// # Type parameters
    /// - `F` — sync preflight hook: `(&ToolCall) -> PreflightDecision`
    /// - `G` — sync tool executor: `(&ToolCall) -> ToolResult` (called
    ///   concurrently via OS threads in Parallel mode)
    pub fn dispatch<F, G>(
        &self,
        calls: Vec<ToolCall>,
        preflight: F,
        execute: G,
    ) -> Vec<ToolResult>
    where
        F: Fn(&ToolCall) -> PreflightDecision,
        G: Fn(&ToolCall) -> ToolResult + Send + Sync + 'static,
    {
        if calls.is_empty() {
            return vec![];
        }

        // --- Step 1: sequential preflight -----------------------------------
        // Each call gets a PreflightDecision; blocked calls are converted to
        // stub ToolResults immediately.
        let mut preflight_decisions: Vec<(ToolCall, PreflightDecision)> =
            Vec::with_capacity(calls.len());
        for call in calls {
            let decision = preflight(&call);
            preflight_decisions.push((call, decision));
        }

        // --- Step 2: dispatch allowed calls ---------------------------------
        match self.mode {
            ExecutionMode::Sequential => {
                Self::dispatch_sequential(preflight_decisions, execute)
            }
            ExecutionMode::Parallel => {
                self.dispatch_parallel(preflight_decisions, execute)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Internal: sequential dispatch
    // -----------------------------------------------------------------------
    fn dispatch_sequential<G>(
        decisions: Vec<(ToolCall, PreflightDecision)>,
        execute: G,
    ) -> Vec<ToolResult>
    where
        G: Fn(&ToolCall) -> ToolResult,
    {
        decisions
            .into_iter()
            .map(|(call, decision)| match decision {
                PreflightDecision::Allow => execute(&call),
                PreflightDecision::Block { reason } => ToolResult {
                    tool_name: call.tool_name,
                    call_id: call.call_id,
                    output: String::new(),
                    elapsed_ms: 0,
                    blocked: true,
                    error: Some(reason),
                },
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Internal: parallel dispatch (threaded, capped at max_concurrency)
    // -----------------------------------------------------------------------
    fn dispatch_parallel<G>(
        &self,
        decisions: Vec<(ToolCall, PreflightDecision)>,
        execute: G,
    ) -> Vec<ToolResult>
    where
        G: Fn(&ToolCall) -> ToolResult + Send + Sync + 'static,
    {
        let total = decisions.len();

        // Allocate result slots indexed by original position.
        let results: Arc<Mutex<Vec<Option<ToolResult>>>> =
            Arc::new(Mutex::new(vec![None; total]));

        let execute = Arc::new(execute);

        // Chunk allowed calls into batches of `max_concurrency`.
        // Blocked calls are inserted inline without spawning threads.
        let thread_handles: Vec<thread::JoinHandle<()>> = Vec::new();

        // We process calls in original order but may run them in parallel
        // batches. A semaphore-like approach: collect all allowed indices, then
        // spawn threads in windows of `max_concurrency`.
        let allowed_indices: Vec<usize> = decisions
            .iter()
            .enumerate()
            .filter(|(_, (_, d))| *d == PreflightDecision::Allow)
            .map(|(i, _)| i)
            .collect();

        // Pre-populate blocked results immediately (no thread needed).
        {
            let mut guard = results.lock().unwrap_or_else(|e| e.into_inner());
            for (i, (call, decision)) in decisions.iter().enumerate() {
                if let PreflightDecision::Block { reason } = decision {
                    guard[i] = Some(ToolResult {
                        tool_name: call.tool_name.clone(),
                        call_id: call.call_id.clone(),
                        output: String::new(),
                        elapsed_ms: 0,
                        blocked: true,
                        error: Some(reason.clone()),
                    });
                }
            }
        }

        // Move calls into an Arc<Vec<_>> so threads can borrow by index.
        let calls_arc: Arc<Vec<(ToolCall, PreflightDecision)>> = Arc::new(decisions);

        // Spawn threads for allowed calls, respecting max_concurrency window.
        for chunk in allowed_indices.chunks(self.max_concurrency) {
            let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();
            for &idx in chunk {
                let calls_ref = Arc::clone(&calls_arc);
                let results_ref = Arc::clone(&results);
                let exec_ref = Arc::clone(&execute);
                let handle = thread::spawn(move || {
                    let call = &calls_ref[idx].0;
                    let result = exec_ref(call);
                    let mut guard = results_ref.lock().unwrap_or_else(|e| e.into_inner());
                    guard[idx] = Some(result);
                });
                handles.push(handle);
            }
            // Join this batch before starting the next (respects max_concurrency).
            for h in handles {
                let _ = h.join();
            }
        }

        // Wait for any stray handles (shouldn't be any after batch joins).
        for h in thread_handles {
            let _ = h.join();
        }

        // Collect in original order.
        let mut guard = results.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .iter_mut()
            .map(|slot| slot.take().expect("result slot must be filled"))
            .collect()
    }

    /// Convenience wrapper: dispatch and also return `DispatchStats`.
    pub fn dispatch_with_stats<F, G>(
        &self,
        calls: Vec<ToolCall>,
        preflight: F,
        execute: G,
    ) -> (Vec<ToolResult>, DispatchStats)
    where
        F: Fn(&ToolCall) -> PreflightDecision,
        G: Fn(&ToolCall) -> ToolResult + Send + Sync + 'static,
    {
        let total_calls = calls.len();
        let wall_start = Instant::now();
        let results = self.dispatch(calls, preflight, execute);
        let wall_time_ms = wall_start.elapsed().as_millis() as u64;

        let mut stats = DispatchStats {
            total_calls,
            wall_time_ms,
            ..Default::default()
        };
        for r in &results {
            if r.blocked {
                stats.blocked += 1;
            } else if r.error.is_some() {
                stats.failed += 1;
                stats.executed += 1;
                stats.sum_serial_ms += r.elapsed_ms;
            } else {
                stats.executed += 1;
                stats.sum_serial_ms += r.elapsed_ms;
            }
        }

        (results, stats)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn allow_all(call: &ToolCall) -> PreflightDecision {
        let _ = call;
        PreflightDecision::Allow
    }

    fn block_named<'a>(blocked_name: &'a str) -> impl Fn(&ToolCall) -> PreflightDecision + 'a {
        move |call: &ToolCall| {
            if call.tool_name == blocked_name {
                PreflightDecision::Block {
                    reason: format!("{} is not permitted", blocked_name),
                }
            } else {
                PreflightDecision::Allow
            }
        }
    }

    fn instant_executor(call: &ToolCall) -> ToolResult {
        ToolResult {
            tool_name: call.tool_name.clone(),
            call_id: call.call_id.clone(),
            output: format!("ok:{}", call.call_id),
            elapsed_ms: 0,
            blocked: false,
            error: None,
        }
    }

    /// Sleep-based executor: sleeps for `elapsed_ms` encoded in args_json as a
    /// plain integer string (milliseconds).
    fn sleep_executor(call: &ToolCall) -> ToolResult {
        let ms: u64 = call.args_json.trim().parse().unwrap_or(50);
        let start = Instant::now();
        thread::sleep(Duration::from_millis(ms));
        ToolResult {
            tool_name: call.tool_name.clone(),
            call_id: call.call_id.clone(),
            output: format!("slept:{}ms", ms),
            elapsed_ms: start.elapsed().as_millis() as u64,
            blocked: false,
            error: None,
        }
    }

    fn make_calls(names: &[(&str, &str)]) -> Vec<ToolCall> {
        names
            .iter()
            .enumerate()
            .map(|(i, (name, args))| ToolCall::new(format!("id{i}"), *name, *args))
            .collect()
    }

    // ── Test 1: sequential preflight order is preserved ─────────────────────
    #[test]
    fn test_preflight_order_preserved() {
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let log2 = Arc::clone(&log);

        let calls = make_calls(&[("Read", "{}"), ("Write", "{}"), ("Bash", "{}")]);
        let dispatcher = ParallelToolDispatcher::new(ExecutionMode::Sequential);
        dispatcher.dispatch(
            calls,
            move |call| {
                log2.lock()
                    .unwrap()
                    .push(call.tool_name.clone());
                PreflightDecision::Allow
            },
            instant_executor,
        );

        let seen = log.lock().unwrap().clone();
        assert_eq!(seen, vec!["Read", "Write", "Bash"],
            "preflight must visit calls in original order");
    }

    // ── Test 2: parallel mode is faster than sum of serial times ────────────
    #[test]
    fn test_parallel_faster_than_serial() {
        // 4 calls each sleeping 50 ms → serial sum ~200 ms, parallel ~50 ms
        let calls: Vec<ToolCall> = (0..4)
            .map(|i| ToolCall::new(format!("id{i}"), "Sleep", "50"))
            .collect();

        let dispatcher = ParallelToolDispatcher::with_concurrency(ExecutionMode::Parallel, 4);
        let (results, stats) =
            dispatcher.dispatch_with_stats(calls, allow_all, sleep_executor);

        assert_eq!(results.len(), 4);
        assert!(
            stats.sum_serial_ms >= 150,
            "sum_serial should be ~200 ms, got {}",
            stats.sum_serial_ms
        );
        assert!(
            stats.wall_time_ms < stats.sum_serial_ms,
            "parallel wall time ({} ms) should be < serial sum ({} ms)",
            stats.wall_time_ms,
            stats.sum_serial_ms
        );
        assert!(
            stats.speedup_ratio() > 1.5,
            "expected speedup > 1.5×, got {:.2}",
            stats.speedup_ratio()
        );
    }

    // ── Test 3: blocked calls are not executed ───────────────────────────────
    #[test]
    fn test_blocked_calls_not_executed() {
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let executed2 = Arc::clone(&executed);

        let calls = make_calls(&[("Read", "{}"), ("Bash", "{}"), ("Write", "{}")]);
        let dispatcher = ParallelToolDispatcher::new(ExecutionMode::Sequential);
        let results = dispatcher.dispatch(
            calls,
            block_named("Bash"),
            move |call| {
                executed2.lock().unwrap().push(call.call_id.clone());
                instant_executor(call)
            },
        );

        // Bash (index 1) must be blocked
        assert!(results[1].blocked, "Bash call should be blocked");
        assert_eq!(results[1].output, "", "blocked call should have empty output");
        assert!(results[1].error.is_some(), "blocked call should carry reason");

        // Read and Write must have executed
        assert!(!results[0].blocked);
        assert!(!results[2].blocked);

        let exec_log = executed.lock().unwrap();
        assert!(!exec_log.contains(&"id1".to_string()), "Bash must not be executed");
        assert!(exec_log.contains(&"id0".to_string()));
        assert!(exec_log.contains(&"id2".to_string()));
    }

    // ── Test 4: results returned in original call order (parallel) ──────────
    #[test]
    fn test_results_in_original_order_parallel() {
        // Calls sleep for decreasing durations so the fastest finishes last in
        // the call list — yet results must still be in submission order.
        let calls = vec![
            ToolCall::new("id0", "SlowTool", "60"),
            ToolCall::new("id1", "MedTool", "30"),
            ToolCall::new("id2", "FastTool", "10"),
        ];

        let dispatcher = ParallelToolDispatcher::with_concurrency(ExecutionMode::Parallel, 3);
        let results = dispatcher.dispatch(calls, allow_all, sleep_executor);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].call_id, "id0", "first result must be id0");
        assert_eq!(results[1].call_id, "id1", "second result must be id1");
        assert_eq!(results[2].call_id, "id2", "third result must be id2");
    }

    // ── Test 5: Sequential mode falls back to sequential execution ───────────
    #[test]
    fn test_sequential_mode_single_thread() {
        // 3 calls each sleeping 20 ms → sequential total ≥ 60 ms
        let calls: Vec<ToolCall> = (0..3)
            .map(|i| ToolCall::new(format!("id{i}"), "Sleep", "20"))
            .collect();

        let dispatcher = ParallelToolDispatcher::new(ExecutionMode::Sequential);
        let (results, stats) =
            dispatcher.dispatch_with_stats(calls, allow_all, sleep_executor);

        assert_eq!(results.len(), 3);
        assert_eq!(*dispatcher.mode(), ExecutionMode::Sequential);
        // In sequential mode wall_time ≈ sum_serial (no true overlap)
        // We just check all calls completed.
        assert_eq!(stats.executed, 3);
        assert_eq!(stats.blocked, 0);
    }

    // ── Additional: empty call list returns empty results ────────────────────
    #[test]
    fn test_empty_calls_returns_empty() {
        let dispatcher = ParallelToolDispatcher::new(ExecutionMode::Parallel);
        let results = dispatcher.dispatch(vec![], allow_all, instant_executor);
        assert!(results.is_empty());
    }

    // ── Additional: max_concurrency getter ──────────────────────────────────
    #[test]
    fn test_max_concurrency_getter() {
        let d = ParallelToolDispatcher::with_concurrency(ExecutionMode::Parallel, 4);
        assert_eq!(d.max_concurrency(), 4);
    }

    // ── Additional: speedup_ratio avoids divide-by-zero ─────────────────────
    #[test]
    fn test_speedup_ratio_zero_wall_time() {
        let stats = DispatchStats {
            wall_time_ms: 0,
            sum_serial_ms: 100,
            ..Default::default()
        };
        assert_eq!(stats.speedup_ratio(), 1.0);
    }

    // ── Additional: dispatch_with_stats counts blocked/failed/executed ───────
    #[test]
    fn test_dispatch_stats_counts() {
        let calls = make_calls(&[("Read", "{}"), ("Bash", "{}"), ("Write", "{}")]);
        let dispatcher = ParallelToolDispatcher::new(ExecutionMode::Sequential);

        let error_exec = |call: &ToolCall| -> ToolResult {
            if call.tool_name == "Write" {
                ToolResult {
                    tool_name: call.tool_name.clone(),
                    call_id: call.call_id.clone(),
                    output: String::new(),
                    elapsed_ms: 1,
                    blocked: false,
                    error: Some("disk full".into()),
                }
            } else {
                instant_executor(call)
            }
        };

        let (_, stats) = dispatcher.dispatch_with_stats(calls, block_named("Bash"), error_exec);
        assert_eq!(stats.total_calls, 3);
        assert_eq!(stats.blocked, 1);  // Bash
        assert_eq!(stats.failed, 1);   // Write
        assert_eq!(stats.executed, 2); // Read + Write (both ran, Write errored)
    }
}
