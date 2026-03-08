---
triggers: ["safety critical", "IEC 61508", "ISO 26262", "SIL", "ASIL", "functional safety", "fault tree", "FMEA", "hazard analysis", "safety integrity level", "safety case", "EN 50128", "nuclear safety"]
tools_allowed: ["read_file", "write_file", "bash"]
category: safety-critical
---

# Safety-Critical Systems

When developing software for safety-critical domains (automotive, rail, nuclear, industrial, medical devices):

1. Determine the applicable standard by domain: IEC 61508 (generic), ISO 26262 (automotive), EN 50128/50129 (rail), IEC 62304 (medical devices), IEC 61513 (nuclear), DO-178C (airborne) — each defines Safety Integrity Levels and required techniques.
2. Perform hazard analysis upfront: use HAZOP (Hazard and Operability), FTA (Fault Tree Analysis), FMEA (Failure Modes and Effects Analysis), and ETA (Event Tree Analysis) — derive safety requirements from identified hazards with quantified risk levels.
3. Assign Safety Integrity Levels based on risk: IEC 61508 uses SIL 1-4 (4 is highest); ISO 26262 uses ASIL A-D (D is highest) — SIL/ASIL determines required techniques, independence, and coverage metrics at each lifecycle phase.
4. Implement defensive programming at every level: check all inputs at function boundaries, validate return codes, use assertions for invariants, implement plausibility checks on sensor data, and use safe state transitions — never assume correct behavior.
5. Avoid dynamic memory allocation in SIL 3/4 and ASIL C/D systems: all memory must be statically allocated or allocated only at initialization; heap fragmentation and allocation failure are unacceptable in continuous-operation safety functions.
6. Prohibit recursion in safety-critical code: stack depth must be statically determinable — use iterative algorithms; if recursion is unavoidable (SIL 1/2 only), prove bounded depth with formal analysis and allocate sufficient stack.
7. Use safe subsets of programming languages: MISRA C for C code, MISRA C++ for C++ code, SPARK subset for Ada, Ferrocene-qualified Rust — restrict language features that are ambiguous, implementation-defined, or prone to error.
8. Implement watchdog timers and safety monitors: independent hardware watchdog must be serviced within a defined window (not too early, not too late); safety monitor runs on a separate processor and cross-checks the main processor's outputs.
9. Design for fail-safe and fail-operational: fail-safe means the system enters a known safe state on failure (e.g., traffic light goes to all-red); fail-operational means the system continues operating at reduced capability (e.g., automotive steering with one ECU failed).
10. Achieve required structural coverage: SIL 1 → statement coverage; SIL 2 → branch/decision coverage; SIL 3/4 → MC/DC or equivalent — use qualified coverage tools; justify any uncoverable code with dead code analysis.
11. Implement diversity and redundancy: use N-version programming (different teams/languages/algorithms) for SIL 3/4; dissimilar redundancy (hardware + software watchdog) for common-cause failure mitigation; monitor cross-channel agreement.
12. Maintain a Safety Case document: structure per GSN (Goal Structuring Notation) — top goal: "system is acceptably safe"; decompose into sub-goals for each hazard; link evidence (test results, analysis, reviews) to claims via arguments.
13. Use formal verification where required: model checking (SPIN, NuSMV) for state machine properties; abstract interpretation (Polyspace, Astree) for runtime error absence; theorem proving (Isabelle, Coq) for algorithm correctness — IEC 61508 recommends formal methods for SIL 3/4.
14. Conduct independent verification and validation (IV&V): verification activities for SIL 3/4 require personnel independent from the developers — define independence levels (person, team, organization) per the applicable standard.
15. Maintain full lifecycle traceability: safety requirement → design → code → test case → test result → safety case argument — bidirectional traceability with impact analysis; any change triggers regression verification of affected safety functions.
