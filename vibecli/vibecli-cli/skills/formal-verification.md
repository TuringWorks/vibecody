---
triggers: ["formal verification", "formal methods", "model checking", "theorem proving", "abstract interpretation", "TLA+", "Alloy", "CBMC", "Frama-C", "Kani", "Coq", "Isabelle", "SPIN", "NuSMV", "Z notation", "proof assistant"]
tools_allowed: ["read_file", "write_file", "bash"]
category: safety-critical
---

# Formal Verification & Formal Methods

When applying formal methods to verify safety-critical, security-critical, or high-assurance software:

1. Choose the right technique for the property: model checking for finite-state systems and temporal properties; abstract interpretation for runtime error absence; theorem proving for complex algorithm correctness; static analysis for coding rule compliance.
2. Use TLA+ for distributed system design: `Init == counter = 0; Next == counter' = counter + 1` — specify invariants, liveness properties, and safety properties; run the TLC model checker to exhaustively explore state spaces up to bounded depth.
3. Use Frama-C with ACSL for C code verification: annotate functions with `/*@ requires \valid(buf+(0..len-1)); ensures \result >= 0; assigns buf[0..len-1]; */` — the WP (Weakest Precondition) plugin generates proof obligations discharged by SMT solvers (Alt-Ergo, Z3, CVC5).
4. Use SPARK/GNATprove for Ada: `procedure Sort (A : in out Array_Type) with Pre => A'Length > 0, Post => Is_Sorted (A) and Is_Permutation (A, A'Old);` — GNATprove discharges proofs automatically or generates VCs for manual proof in Coq/Isabelle.
5. Use Kani for Rust verification: `#[kani::proof] fn check_bounds() { let idx: usize = kani::any(); kani::assume(idx < 100); let arr = [0u8; 100]; assert!(arr[idx] == 0); }` — Kani uses CBMC under the hood to verify panic-freedom, overflow, and custom assertions.
6. Use Alloy for lightweight modeling: `sig Process { state: one State } fact { all p: Process | p.state != Error => p.state' in {Running, Waiting} }` — Alloy Analyzer finds counterexamples in small scopes; excellent for protocol and data model design.
7. Apply abstract interpretation with Polyspace or Astree: proves absence of runtime errors (division by zero, overflow, out-of-bounds, uninitialized reads) for all possible inputs without test cases — produces Green (proven safe), Orange (unproven), Red (proven error) annotations.
8. Use SPIN for protocol verification: write models in Promela; specify properties in LTL: `ltl safety { [] (!(P && Q)) }` (mutual exclusion); SPIN exhaustively checks all interleavings for deadlocks, assertion violations, and liveness failures.
9. Use CBMC (C Bounded Model Checker) for C/C++: `cbmc --function main --unwind 10 --unwinding-assertions program.c` — checks assertions, buffer overflows, pointer safety, and arithmetic overflow up to a bounded loop depth.
10. Write loop invariants for automated provers: `/*@ loop invariant 0 <= i <= n; loop invariant sum == i * (i-1) / 2; loop assigns i, sum; loop variant n - i; */` — invariants must hold at loop entry and be preserved by each iteration; variants prove termination.
11. Use refinement for stepwise verification: specify abstract behavior first (e.g., a set), then prove the implementation (e.g., a sorted array with binary search) refines the specification — each refinement step is smaller and more tractable to prove.
12. Combine formal methods with testing: formal verification covers all inputs for proven properties but may miss unspecified properties; testing covers specific scenarios including environmental interactions — the combination provides stronger assurance than either alone.
13. For certification credit: DO-333 (airborne formal methods supplement) defines how formal methods can replace or supplement testing objectives in DO-178C; IEC 61508 recommends formal methods for SIL 3/4 — document the verification tool, the property proved, and any assumptions.
14. Manage proof maintenance: proofs break when code changes — integrate proof checking into CI (`gnatprove` for SPARK, `frama-c -wp` for C, `kani` for Rust); treat proof failures the same as test failures; budget time for proof repair in sprint planning.
15. Start small and pragmatic: prove the most critical properties first (absence of runtime errors, safety invariants, protocol correctness) — don't attempt full functional correctness immediately; even partial formal verification eliminates high-severity defect classes.
