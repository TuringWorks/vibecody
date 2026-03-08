---
triggers: ["MISRA C", "MISRA C++", "safety critical C", "automotive C", "CERT C", "IEC 61508 C", "ISO 26262 C", "embedded C safety", "static analysis C", "coding standard C"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gcc"]
category: safety-critical
---

# MISRA C / Safety-Critical C

When writing C code for safety-critical systems under MISRA C:2012 and related standards:

1. Use MISRA C:2012 (with Amendment 2 and 3) as the baseline coding standard — it defines 175 guidelines (mandatory, required, advisory) that restrict C to a safer, more predictable subset; document deviations with formal deviation records.
2. Never use `malloc`, `calloc`, `realloc`, or `free` in safety code (MISRA Rule 21.3) — all memory must be statically allocated or allocated once at initialization; dynamic allocation introduces fragmentation, non-deterministic timing, and allocation failure risks.
3. No recursion (MISRA Rule 17.2) — functions shall not call themselves directly or indirectly; recursion makes stack depth analysis impossible; rewrite recursive algorithms as iterative with explicit stacks of bounded size.
4. All switch statements must have a `default` clause (MISRA Rule 16.4) — even when all enum values are handled; the default should contain a defensive action (error handler, assertion) to catch unexpected values from hardware glitches or memory corruption.
5. No implicit type conversions between signed and unsigned (MISRA Rules 10.1-10.8) — use explicit casts with range checks: `if (signed_val >= 0) { unsigned_result = (uint32_t)signed_val; }` to prevent wrap-around and sign-extension bugs.
6. Restrict pointer arithmetic: no more than one level of pointer indirection (MISRA Rule 18.5); pointer arithmetic only on arrays (MISRA Rule 18.1); no pointer-to-integer casts except for memory-mapped registers with documented justification.
7. Use fixed-width integer types exclusively: `uint8_t`, `int16_t`, `uint32_t`, `int64_t` from `<stdint.h>` — never use `int`, `short`, `long` as their sizes are implementation-defined and vary across compilers/targets.
8. Initialize all variables at declaration (MISRA Rule 9.1) — uninitialized reads are undefined behavior; for arrays, use `= {0}` or explicit memset at initialization; the compiler may not warn about all uninitialized paths.
9. No `goto` statements (MISRA Rule 15.1) — complicates control flow analysis and formal verification; use structured programming with `if/else`, `while`, `for`; single-entry/single-exit functions are preferred for SIL 3/4.
10. Compile with maximum warnings enabled: `-Wall -Wextra -Werror -pedantic -Wconversion -Wsign-conversion -Wcast-align -Wstrict-prototypes` — treat all warnings as errors; no warning suppression without documented justification.
11. Run static analysis tools qualified per the applicable standard: Polyspace (MathWorks), PC-lint/FlexeLint, QA-C (Perforce/Helix), Coverity, CodeSonar, Parasoft C/C++test — configure for MISRA C:2012 rule checking; resolve all mandatory and required violations.
12. Achieve MC/DC (Modified Condition/Decision Coverage) for SIL 3/4 code: every condition in a decision independently affects the outcome — use `gcov` + `lcov` for measurement, but prefer qualified tools (VectorCAST, LDRA, Cantata) for certification evidence.
13. Use `const` and `volatile` correctly: mark read-only data as `const`; mark hardware registers and shared memory as `volatile` (MISRA Rule 2.2) — `volatile` prevents compiler optimization of memory-mapped I/O and interrupt-shared variables.
14. Limit function complexity: cyclomatic complexity <= 10 for safety functions; maximum function length 50-75 lines; maximum 5-7 parameters — complex functions are harder to test, review, and verify; decompose into smaller, single-purpose functions.
15. Document every deviation from MISRA: use a Deviation Record with guideline number, justification, risk assessment, and compensating measures — deviations must be reviewed and approved by the safety assessor; minimize deviations in SIL 3/4 code.
16. Perform unit testing with 100% MC/DC: test boundary values, error paths, and nominal cases; use stubs/mocks for hardware dependencies; execute tests on both host (for speed) and target (for behavioral equivalence) — document target/host behavioral differences.
