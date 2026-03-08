---
triggers: ["defense", "military", "MIL-STD", "weapons system", "tactical", "C4ISR", "ITAR", "CUI", "NIST 800-171", "CMMC", "defense software", "mission critical", "electronic warfare", "radar software"]
tools_allowed: ["read_file", "write_file", "bash"]
category: defense
---

# Defense & Military Systems Software

When developing defense/military software systems under MIL-STD and DoD regulations:

1. Classify information handling: ITAR-controlled technical data requires US-person-only access; CUI (Controlled Unclassified Information) follows NIST SP 800-171; classified systems follow ICD 503 — never process classified data on unaccredited systems.
2. Follow MIL-STD-498 (or its successor) for software development: SRS (Software Requirements Specification), SDD (Software Design Description), STP (Software Test Plan), STR (Software Test Report) — deliverables are contractually required Data Item Descriptions (DIDs).
3. Implement cybersecurity per NIST RMF (Risk Management Framework): categorize the system (FIPS 199), select controls (NIST 800-53), implement controls, assess (SCA), authorize (ATO), and monitor continuously — every deployed system needs an Authority to Operate.
4. Use cross-domain solutions (CDS) for multi-level security: guard implementations must be evaluated per the Cross Domain Solution Design Guidance; implement content filtering, dirty word searches, and format validation at security boundaries.
5. For real-time tactical systems: use deterministic RTOSes (VxWorks, INTEGRITY, LynxOS); partition safety-critical and mission-critical functions; implement watchdog timers; ensure graceful degradation — a weapon system must fail safe, never fail dangerous.
6. Implement MIL-STD-1553B data bus correctly: dual-redundant bus (A and B); Bus Controller manages traffic; Remote Terminals respond within 12μs; use status word bit 0 (terminal flag) for health indication; implement RT-to-RT transfers for latency-critical paths.
7. Follow FACE (Future Airborne Capability Environment) for portable avionics: use FACE-conformant operating system segments (Safety Base, Security, General Purpose); define Unit of Portability (UoP) interfaces with FACE IDL; validate with FACE Conformance Test Suite.
8. Implement Link 16 / TADIL-J messaging per MIL-STD-6016: use J-series messages (J2.2 for air tracks, J3.2 for point tracks); implement NPG (Network Participation Group) membership management; handle time slot allocation for TDMA access.
9. For radar/EW signal processing: implement FFT-based pulse compression, CFAR (Constant False Alarm Rate) detection, and track-while-scan algorithms; use FPGA offload for front-end processing; keep track management and data fusion in software.
10. Apply DISA STIGs (Security Technical Implementation Guides) to all deployed systems: operating system STIGs, application STIGs, network device STIGs — automate compliance scanning with SCAP tools; document Plan of Action and Milestones (POA&M) for any open findings.
11. Use secure coding practices per CERT C/C++ and CWE: no unbounded buffers, no format string vulnerabilities, no integer overflow in size calculations, no use-after-free — run static analysis (Coverity, Klocwork, CodeSonar) as part of CI.
12. Implement fault tolerance with operational modes: full mission capable (FMC), mission capable (MC), degraded (DEG), and maintenance (MAINT) — define mode transition logic in requirements; test all transitions including abnormal paths.
13. Handle SWAP-C constraints (Size, Weight, Power, and Cost): optimize for embedded targets (ARM, PowerPC, DSP); minimize memory footprint; use fixed-point arithmetic where floating-point hardware is unavailable; profile thermal dissipation.
14. For DevSecOps in defense: follow the DoD Enterprise DevSecOps Reference Design; use hardened container images from Iron Bank (Platform One); implement continuous ATO (cATO) with automated security scanning in the pipeline.
15. Test at every level: unit test with MC/DC where required, integration test at CSC and CSCI levels, system test against SSS requirements, operational test (OT&E) under realistic conditions — maintain a Requirements Traceability Matrix (RTM) linking every requirement to its test.
