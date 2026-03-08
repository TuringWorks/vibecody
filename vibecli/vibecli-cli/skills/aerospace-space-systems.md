---
triggers: ["space systems", "satellite software", "spacecraft", "ECSS", "NASA", "cFS", "CCSDS", "space flight software", "CubeSat", "radiation hardening", "RTOS space", "launch vehicle"]
tools_allowed: ["read_file", "write_file", "bash"]
category: aerospace
---

# Space Systems & Satellite Software

When developing software for spacecraft, satellites, and launch vehicles:

1. Follow NASA/ESA standards: NASA-STD-8719.13 (Software Safety), NPR 7150.2 (Software Engineering), ECSS-E-ST-40C (ESA Software Engineering) — these define lifecycle processes, safety analysis, and V&V requirements; classification A-D drives rigor level.
2. Use NASA cFS (core Flight System) as the flight software framework: layered architecture with OSAL (OS Abstraction Layer), PSP (Platform Support Package), cFE (core Flight Executive with ES, SB, EVS, TBL, TIME services) — applications communicate via Software Bus messages.
3. Implement CCSDS (Consultative Committee for Space Data Systems) protocols: use Space Packet Protocol for telemetry/telecommand framing; implement CCSDS File Delivery Protocol (CFDP) for reliable file transfer; use AOS/TM/TC transfer frames for link layer.
4. Handle radiation effects in software: implement EDAC (Error Detection and Correction) scrubbing for memory; use TMR (Triple Modular Redundancy) for critical variables; periodically refresh FPGA configuration; detect and recover from Single-Event Upsets (SEU) and latchups.
5. Design for autonomous operation: ground contact windows may be minutes per orbit — implement fault detection, isolation, and recovery (FDIR) that handles anomalies autonomously; use state-based safe modes with progressive degradation levels.
6. Implement command and telemetry handling: validate command checksums, sequence counters, and authorization before execution; generate telemetry at defined rates; implement housekeeping, diagnostic, and science telemetry streams with priority-based downlink scheduling.
7. Use VxWorks, RTEMS, or FreeRTOS for flight RTOS: RTEMS is open-source and has heritage on multiple missions; configure tick rate for mission timing requirements; use rate monotonic scheduling for deterministic task execution.
8. Thermal management in software: read temperature sensors, control heaters with hysteresis bands, adjust duty cycles based on orbital position (eclipse/sunlit) — implement thermal safe mode that activates survival heaters if primary control fails.
9. Implement attitude determination and control (ADCS): read star trackers, sun sensors, magnetometers, and gyros; fuse measurements with extended Kalman filter; command reaction wheels and magnetorquers; handle wheel desaturation using magnetic torquers or thrusters.
10. Power management: monitor bus voltage, battery state-of-charge, and solar array current; implement load shedding schedules for eclipse periods; protect against over-discharge with automatic non-essential load disconnection.
11. Use CubeSat standards (CDS) for small satellites: PC/104 bus format, CCSDS or AX.25 for amateur-band communication, UHF/VHF for telemetry, S-band or X-band for payload data — implement deployment sequencers for antenna and solar panel release with timer redundancy.
12. Test with hardware-in-the-loop (HITL) and software-in-the-loop (SITL): simulate orbital dynamics, sensor models, actuator models, thermal environment, and ground station contact — run full mission scenarios including anomaly injection and safe mode transitions.
13. Handle timekeeping precisely: use GPS time or spacecraft clock with ground-correlated corrections; implement leap second handling; maintain a correlation between spacecraft time and UTC for science data timestamping — clock drift management is critical for formation flying.
14. Configuration management per ECSS-M-ST-40C or NASA NPR 7150.2: maintain flight software baselines with formal change control; use identical build environments for flight images; archive build tools, source, and test artifacts for the entire mission lifetime (potentially decades).
