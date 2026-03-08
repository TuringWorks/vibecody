---
triggers: ["PLC", "SCADA", "industrial automation", "ladder logic", "HMI", "Allen Bradley", "Siemens PLC", "DCS"]
tools_allowed: ["read_file", "write_file", "bash"]
category: engineering
---

# PLC & SCADA Industrial Automation

When working with PLC programming and SCADA systems:

1. Follow IEC 61131-3 standard for PLC programming languages — use Ladder Diagram (LD) for discrete logic familiar to electricians, Structured Text (ST) for complex algorithms and math, Function Block Diagram (FBD) for analog process control, Sequential Function Chart (SFC) for sequential processes, and Instruction List (IL) only for legacy maintenance. Choose the language that best matches the problem domain and the maintenance team's skill set.

2. Design SCADA system architecture with clear separation of layers — field devices and sensors at Level 0, PLCs and RTUs at Level 1, SCADA servers and HMI at Level 2, MES/historian at Level 3, and ERP at Level 4 (Purdue model). Use redundant communication paths between levels, implement hot-standby servers for critical systems, and size the SCADA server for at least 2x the expected tag count to allow for future expansion.

3. Design HMI screens following ISA-101 high-performance principles — use gray backgrounds with color reserved for abnormal conditions, avoid gratuitous animation and 3D effects, display process variables with units and engineering ranges, show equipment status with clear state indicators, organize screens in a logical hierarchy (overview → area → detail), and ensure all critical information is visible within 3 clicks from the main screen.

4. Configure I/O properly with correct wiring practices — verify sensor types (NPN/PNP, 2-wire/3-wire/4-wire) match input card specifications, use shielded cables for analog signals with single-point grounding, separate power wiring from signal wiring in cable trays, label all terminal blocks to match I/O lists, document channel assignments in a spreadsheet cross-referenced with electrical drawings, and always include 10-15% spare I/O capacity for future expansion.

5. Implement PID control loops methodically — start with manual mode to verify the control valve and sensor work correctly, use step-test or bump-test methods to characterize process dynamics (dead time, time constant, gain), apply Ziegler-Nichols or Cohen-Coon tuning rules as a starting point, tune in the order of P then I then D, implement anti-windup on the integral term, set reasonable output limits and rate-of-change limits, and document all tuning parameters with the date tuned and process conditions.

6. Select and configure communication protocols appropriate to the application — use Modbus RTU/TCP for simple, vendor-neutral serial or Ethernet communication; Profinet for Siemens ecosystems with real-time requirements; EtherNet/IP for Allen-Bradley and Rockwell environments; OPC UA for cross-vendor interoperability and IT/OT convergence; and HART for smart analog instrument configuration. Always document network architecture including IP addresses, node IDs, baud rates, and timeout settings.

7. Design safety systems in compliance with IEC 61508 and IEC 61511 — determine required Safety Integrity Level (SIL 1-4) through risk analysis (LOPA or risk graph), use certified safety PLCs (e.g., Allen-Bradley GuardLogix, Siemens F-CPU) for safety functions, keep safety logic separate from standard control logic, implement proof testing at intervals specified by the SIL verification, never bypass safety systems without a formal Management of Change (MOC) process, and maintain complete documentation of all safety instrumented functions (SIFs).

8. Implement alarm management following ISA-18.2 and IEC 62682 — rationalize alarms to eliminate nuisance and standing alarms (target fewer than 6 alarms per operator per hour), assign proper priorities (critical, high, medium, low) based on consequence and response time, configure appropriate deadbands to prevent alarm chattering, implement alarm shelving and suppression with audit trails, create first-out alarm logic for critical trip sequences, and regularly review alarm performance metrics (flood rate, stale alarms, distribution by priority).

9. Integrate data historians for long-term process data storage — configure appropriate scan rates (typically 1-10 seconds for process variables, sub-second for high-speed events), use compression settings that preserve data fidelity while managing storage (swinging door or boxcar compression), set up meaningful tag naming conventions (e.g., Area.Unit.Measurement.Type), create calculated tags for KPIs and derived values, establish data retention policies, and provide operator and engineering access through trend displays and reporting tools (e.g., OSIsoft PI, Honeywell PHD, AVEVA Historian).

10. Secure OT networks following IEC 62443 and NIST guidelines — segment IT and OT networks with industrial firewalls and DMZs, disable unnecessary ports and services on PLCs and HMIs, change default passwords on all devices, implement role-based access control, use encrypted protocols where available (OPC UA with certificates, HTTPS for web-based HMIs), maintain an asset inventory of all connected devices with firmware versions, apply patches through a tested and approved process, and monitor network traffic for anomalies using OT-specific intrusion detection.

11. Conduct thorough commissioning with Factory Acceptance Testing (FAT) and Site Acceptance Testing (SAT) — create detailed test procedures for each I/O point (loop checks), verify all interlocks and safety functions, simulate process conditions to test control logic, validate HMI displays against P&IDs, test communication failover and redundancy switchover, perform load testing on SCADA servers, document all test results with pass/fail criteria, and maintain a punch list for any outstanding items with assigned owners and deadlines.

12. Establish systematic troubleshooting and maintenance practices — use PLC diagnostic LEDs and status registers to identify faults, leverage built-in diagnostic tools (Siemens TIA Portal diagnostics, Rockwell FactoryTalk Diagnostics), maintain online and offline program backups with version control, implement preventive maintenance schedules for hardware (battery replacement, fan cleaning, firmware updates), keep spare parts inventory for critical components, create troubleshooting guides for common failure modes, and train operations and maintenance staff on system architecture and basic fault-finding procedures.
