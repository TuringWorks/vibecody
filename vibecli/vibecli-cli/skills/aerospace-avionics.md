---
triggers: ["aerospace", "avionics", "DO-178C", "DO-178B", "DO-254", "flight software", "flight control", "autopilot", "ARINC 429", "ARINC 653", "MIL-STD-1553", "airborne software", "DAL", "Design Assurance Level"]
tools_allowed: ["read_file", "write_file", "bash"]
category: aerospace
---

# Aerospace & Avionics Software

When developing airborne/avionics software under DO-178C and related standards:

1. Classify software by Design Assurance Level (DAL): Level A (catastrophic) requires 100% MC/DC coverage with independence; Level B (hazardous) requires decision coverage; Level C (major) requires statement coverage — DAL drives every process decision.
2. Follow DO-178C objectives rigorously: Plan (PSAC, SDP, SVP, SCMP, SQAP), Develop (SRD→SDD→Source→Object), Verify (reviews, analysis, testing), and Configuration Management — each objective must have evidence of completion.
3. Use ARINC 653-compliant RTOS (VxWorks 653, PikeOS, LynxOS-178, INTEGRITY-178) for partitioned systems — time and space partitioning ensures a fault in one partition cannot corrupt another; define partition schedules in XML configuration.
4. Implement ARINC 429 interfaces with strict word formatting: Label (octal, bits 1-8), SDI (bits 9-10), Data (bits 11-29), SSM (bits 30-31), Parity (bit 32) — validate label/SDI pairs at receive; reject words with parity errors.
5. For MIL-STD-1553B bus communication: implement both Bus Controller (BC) and Remote Terminal (RT) modes; validate command/status word formats; handle mode codes for synchronization and built-in test; timeout at 14μs no-response.
6. Requirements must be traceable bidirectionally: every high-level requirement → low-level requirement → source code → test case — use tools like DOORS, Polarion, or Reqtify; unlinked requirements or untraceable code are certification blockers.
7. Perform structural coverage analysis: statement coverage (DAL C), decision coverage (DAL B), MC/DC (DAL A) — use qualified tools (e.g., VectorCAST, LDRA, Rapita RVS) and document any uncoverable code with deactivated code analysis.
8. Dead code and deactivated code must be justified: dead code (unreachable) must be removed or explained; deactivated code (reachable but not executed in current config) requires analysis showing it cannot be inadvertently activated.
9. Implement Built-In Test (BIT): power-up BIT validates hardware/software initialization; continuous BIT monitors health in-flight; initiated BIT runs on maintenance command — report failures via discrete outputs and maintenance data recording.
10. Handle redundancy and voting: use triple modular redundancy (TMR) with majority voting for DAL A functions; implement cross-channel data link (CCDL) for comparison monitoring; detect and isolate failed channels within one computation frame.
11. Timing must be deterministic: worst-case execution time (WCET) analysis is mandatory — use static analysis tools (aiT, RapiTime) or measurement-based approaches; ensure all tasks complete within their frame period with margin.
12. Use configuration management per DO-178C Section 7: every artifact (source, object, tools, test cases, test results) must be under CM with baselines at each review milestone; problem reports track every defect from detection through verified closure.
13. Qualify development and verification tools: tools that could insert errors (compilers, code generators) require Tool Qualification per DO-330 at TQL-1; tools that could fail to detect errors (coverage analyzers, test frameworks) require TQL-4/5.
14. Apply DO-178C supplements when applicable: DO-332 for object-oriented technology (restrict dynamic dispatch, prove absence of dangling references), DO-333 for formal methods (can replace some testing objectives), DO-331 for model-based development.
