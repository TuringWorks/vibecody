//! Self-verifying agent loop — bounded retry around a `verify → fix`
//! cycle.
//!
//! Phase 53 P1 (A8 from v13 fitgap, Devin 2.2). Wraps any verifier
//! that returns `Verdict::Pass | Verdict::Fail(diagnostic)` and any
//! repair function that takes the diagnostic and produces an updated
//! candidate. The loop runs at most `max_iterations` times (default 3
//! per the Phase 53 spec) and returns the terminal outcome.
//!
//! Designed to plug into `visual_verify.rs` (the existing screenshot-
//! diff module): the verifier is `visual_verify`, the repair function
//! is the agent's `apply_diff` against the LLM's response. This module
//! ships only the bounded-loop scaffold so it can be unit-tested
//! independently.
//!
//! Loop shape:
//!
//!   loop iteration 1..=max:
//!     verdict = verify(candidate)
//!     if Pass:                           → Success
//!     if iteration == max:               → MaxIterations(diag)
//!     repair_outcome = repair(candidate, diag)
//!     if Cancelled:                      → Cancelled
//!     candidate = repair_outcome
//!
//! Errors from verify or repair propagate as `anyhow::Error`.

use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Pass,
    Fail(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome<T> {
    /// Verifier returned `Pass` after `iterations_used` rounds.
    Success {
        candidate: T,
        iterations_used: usize,
    },
    /// Hit the iteration cap — last candidate + last diagnostic so the
    /// caller can surface the failure honestly.
    MaxIterations {
        candidate: T,
        last_diagnostic: String,
        iterations_used: usize,
    },
    /// External cancellation signal fired during `repair`.
    Cancelled {
        candidate: T,
        iterations_used: usize,
    },
}

#[derive(Debug, Clone)]
pub struct LoopConfig {
    /// Maximum number of (verify, repair) rounds. Phase 53 spec is 3.
    pub max_iterations: usize,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self { max_iterations: 3 }
    }
}

/// Run a bounded verify → repair loop.
///
/// `verify` runs first; if Pass, returns `Success` immediately with
/// the unchanged candidate. Otherwise `repair` is called with the
/// failing diagnostic, producing a new candidate, and the next
/// iteration begins.
pub fn run_loop<T, V, R>(
    initial: T,
    config: &LoopConfig,
    verify: V,
    mut repair: R,
) -> Result<Outcome<T>>
where
    V: Fn(&T) -> Result<Verdict>,
    R: FnMut(&T, &str) -> Result<RepairOutcome<T>>,
{
    let max = config.max_iterations.max(1);
    let mut candidate = initial;
    let mut iterations_used = 0usize;
    let mut last_diagnostic = String::new();

    loop {
        iterations_used += 1;
        let verdict = verify(&candidate)?;
        match verdict {
            Verdict::Pass => {
                return Ok(Outcome::Success {
                    candidate,
                    iterations_used,
                });
            }
            Verdict::Fail(d) => {
                last_diagnostic = d;
            }
        }
        if iterations_used >= max {
            return Ok(Outcome::MaxIterations {
                candidate,
                last_diagnostic,
                iterations_used,
            });
        }
        match repair(&candidate, &last_diagnostic)? {
            RepairOutcome::Updated(next) => {
                candidate = next;
            }
            RepairOutcome::Cancelled(c) => {
                return Ok(Outcome::Cancelled {
                    candidate: c,
                    iterations_used,
                });
            }
        }
    }
}

/// What the repair function returned.
#[derive(Debug, Clone)]
pub enum RepairOutcome<T> {
    /// New candidate to verify.
    Updated(T),
    /// External cancellation — abort the loop.
    Cancelled(T),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_on_first_iteration_returns_success_with_zero_repairs() {
        let out: Outcome<i32> = run_loop(
            42,
            &LoopConfig::default(),
            |_| Ok(Verdict::Pass),
            |c, _| Ok(RepairOutcome::Updated(*c)),
        )
        .unwrap();
        assert_eq!(
            out,
            Outcome::Success {
                candidate: 42,
                iterations_used: 1
            }
        );
    }

    #[test]
    fn fails_then_repairs_then_passes_within_budget() {
        let mut iter = 0;
        let out: Outcome<i32> = run_loop(
            0,
            &LoopConfig { max_iterations: 3 },
            |c| {
                iter += 1;
                if *c >= 2 {
                    Ok(Verdict::Pass)
                } else {
                    Ok(Verdict::Fail(format!("too small: {c}")))
                }
            },
            |c, _diag| Ok(RepairOutcome::Updated(c + 1)),
        )
        .unwrap();
        match out {
            Outcome::Success { candidate, iterations_used } => {
                assert_eq!(candidate, 2);
                assert!(iterations_used >= 2);
            }
            other => panic!("expected Success, got {other:?}"),
        }
    }

    #[test]
    fn exceeds_iteration_cap_returns_max_iterations_with_last_diag() {
        let out: Outcome<i32> = run_loop(
            0,
            &LoopConfig { max_iterations: 2 },
            |c| Ok(Verdict::Fail(format!("still bad: {c}"))),
            |c, _| Ok(RepairOutcome::Updated(c + 1)),
        )
        .unwrap();
        match out {
            Outcome::MaxIterations {
                last_diagnostic,
                iterations_used,
                ..
            } => {
                assert_eq!(iterations_used, 2);
                assert!(last_diagnostic.contains("still bad"));
            }
            other => panic!("expected MaxIterations, got {other:?}"),
        }
    }

    #[test]
    fn repair_cancellation_short_circuits_with_cancelled() {
        let out: Outcome<i32> = run_loop(
            0,
            &LoopConfig { max_iterations: 5 },
            |_| Ok(Verdict::Fail("nope".into())),
            |c, _| Ok(RepairOutcome::Cancelled(*c)),
        )
        .unwrap();
        assert!(matches!(out, Outcome::Cancelled { .. }));
    }

    #[test]
    fn verify_error_propagates_as_anyhow_err() {
        let res: Result<Outcome<i32>> = run_loop(
            0,
            &LoopConfig::default(),
            |_| Err(anyhow::anyhow!("verifier exploded")),
            |c, _| Ok(RepairOutcome::Updated(*c)),
        );
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("verifier exploded"));
    }
}
