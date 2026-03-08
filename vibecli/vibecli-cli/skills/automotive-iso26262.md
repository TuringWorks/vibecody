---
triggers: ["automotive software", "ISO 26262", "AUTOSAR", "ASIL", "ECU", "automotive safety", "vehicle software", "ADAS", "autonomous driving", "CAN bus", "automotive Ethernet", "SOTIF"]
tools_allowed: ["read_file", "write_file", "bash"]
category: safety-critical
---

# Automotive Software (ISO 26262 / AUTOSAR)

When developing automotive software under ISO 26262 and AUTOSAR:

1. Follow ISO 26262 lifecycle: Part 3 (Concept Phase: hazard analysis, ASIL assignment) → Part 4 (System Level) → Part 5 (Hardware) → Part 6 (Software: design, implementation, verification, testing) → Part 8 (Supporting processes) → Part 9 (ASIL decomposition).
2. Assign ASIL (Automotive Safety Integrity Level) per hazard: evaluate Severity (S0-S3), Exposure (E0-E4), Controllability (C0-C3) — the combination determines ASIL A through D or QM (Quality Management, no safety requirement).
3. Use AUTOSAR Classic for ECU software: layered architecture with BSW (Basic Software), RTE (Runtime Environment), and SWC (Software Components) — define software component interfaces in ARXML; use the AUTOSAR methodology for code generation and integration.
4. Use AUTOSAR Adaptive for high-performance domains (ADAS, infotainment): POSIX-based, service-oriented architecture with ara::com for communication, ara::exec for execution management — supports dynamic deployment and OTA updates with safety partitioning.
5. Implement CAN communication per ISO 11898: define message IDs and signals in DBC files; use signal packing/unpacking with byte-order awareness (Intel/Motorola); implement message timeout monitoring, CRC protection, and alive counters for safety-relevant messages.
6. For ADAS/AD systems: follow ISO 21448 (SOTIF — Safety of the Intended Functionality) in addition to ISO 26262 — SOTIF addresses hazards from functional insufficiencies (sensor limitations, algorithm failures) rather than systematic or random hardware faults.
7. Apply MISRA C:2012 as the mandatory coding standard: configure static analysis tools (Polyspace, QA-C, PC-lint) for full MISRA compliance; document all deviations with deviation permits approved by the safety manager.
8. Design software with freedom from interference (FFI): ASIL D and QM software on the same ECU requires partitioning (memory protection, temporal protection via OS) — demonstrate that QM faults cannot corrupt ASIL D functions through analysis and testing.
9. Implement diagnostic communication per UDS (ISO 14229): support diagnostic sessions, DTC (Diagnostic Trouble Code) management, read/write memory, routine control, and ECU reset services — integrate with OBD-II for emission-related diagnostics.
10. Use hardware-software interface (HSI) specification: define exactly how software accesses hardware (register maps, timing constraints, interrupt behavior, DMA channels) — HSI mismatches are a common source of systematic faults.
11. Implement safe states and degraded modes: define safe states for each hazard (e.g., limp-home mode for engine management, steering assist reduction for EPS failure); ensure transition to safe state within the Fault Tolerant Time Interval (FTTI).
12. Test according to ISO 26262 Part 6: requirements-based testing with equivalence classes, boundary values, and error guessing (Table 10); structural coverage at statement level (ASIL A/B) or branch level (ASIL C/D); back-to-back testing between model and code.
13. For over-the-air updates: implement A/B partition scheme for rollback capability; verify update integrity with cryptographic signatures; ensure updates maintain safety certification — ISO 24089 defines the process for software update management.
14. Use Automotive Ethernet (100BASE-T1, 1000BASE-T1) for high-bandwidth domains: implement SOME/IP for service discovery and RPC; use AVB/TSN for time-sensitive networking; replace CAN for camera, radar, and lidar data transport.
15. Perform safety analysis at software level: software FMEA analyzing the effects of each software component failure; dependent failure analysis (common cause, cascading) — demonstrate that single-point faults and latent faults meet SPFM and LFM metrics.
