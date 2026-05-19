//! TS / Python intra-procedural taint scanner (v1 stub).
//!
//! See `docs/design/security-posture/scanners.md` §6 for the full
//! design. This file lands the **module skeleton + Scanner trait
//! impl** so the aggregator registration is complete and the
//! design contracts are locked in code; the actual tree-sitter
//! AST walk + source→sink dataflow ships in a follow-on slice.
//!
//! ## Why a stub now
//!
//! Tree-sitter integration is a multi-day build that has to:
//! 1. Pull the `tree-sitter`, `tree-sitter-typescript`, and
//!    `tree-sitter-python` crates as new workspace deps.
//! 2. Define the per-language source / sink / sanitizer node
//!    queries.
//! 3. Implement the intra-procedural flow analysis (which is
//!    bounded and non-trivial — see scanners.md §6 for the
//!    algorithm sketch).
//! 4. Add fuzz tests so a malformed source file can't panic the
//!    scanner.
//!
//! The stub is **fail-safe**: returns an empty findings vec rather
//! than emitting placeholder findings. The panel surfaces no
//! confusion about whether the scanner ran — it just lists no
//! taint findings until the real implementation lands.
//!
//! ## When the real implementation arrives
//!
//! The Scanner trait impl already takes the right shape — the
//! aggregator wiring (in `commands.rs::security_posture_scan`)
//! doesn't change. Only the body of `scan()` gets rewritten.

use crate::security_posture::{Scanner, SecurityFinding};
use anyhow::Result;
use std::path::Path;

pub struct TaintScanner;

impl Scanner for TaintScanner {
    fn name(&self) -> &'static str {
        "taint"
    }

    /// Stub — returns an empty findings vec. The aggregator's
    /// per-scanner error chip is *not* triggered because returning
    /// no findings is a valid scanner outcome (it just means
    /// "nothing tainted detected"). Full implementation arrives in
    /// the next slice per the scanners.md §6 algorithm.
    fn scan(&self, _workspace: &Path) -> Result<Vec<SecurityFinding>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_name_stable() {
        assert_eq!(TaintScanner.name(), "taint");
    }

    #[test]
    fn stub_returns_empty_findings() {
        let scanner = TaintScanner;
        let findings = scanner.scan(Path::new("/tmp")).unwrap();
        assert!(
            findings.is_empty(),
            "stub must not emit placeholder findings — the panel \
             surfaces the empty result as 'no taint findings'"
        );
    }
}
