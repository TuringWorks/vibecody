---
triggers: ["industrial automation", "PLC", "SCADA", "OPC-UA", "MES", "digital twin", "cobot", "pick and place", "conveyor", "industrial IoT", "factory automation"]
tools_allowed: ["read_file", "write_file", "bash"]
category: robotics
---

# Industrial Automation and Factory Robotics

When working with industrial automation, PLC programming, and factory systems:

1. Structure PLC programs following IEC 61131-3 using a modular architecture: organize code into Program Organization Units (POUs) — Function Blocks for reusable equipment modules (valve, motor, axis), Functions for stateless calculations, and Programs for top-level orchestration — use Structured Text (ST) for complex logic and Sequential Function Charts (SFC) for step-based sequences, and maintain a consistent naming convention (e.g., ISA-88 equipment module naming) across the entire project.

2. Implement OPC-UA servers on controllers and edge devices by defining a well-structured information model (address space) that reflects the physical and logical hierarchy of the plant — expose real-time process data, alarms, historical data access, and method calls through standard OPC-UA services, use security profiles (Sign & Encrypt with X.509 certificates), and implement the Pub/Sub transport for high-throughput telemetry to cloud or SCADA systems.

3. Design SCADA system architecture in tiered layers: field devices and PLCs at Level 1, SCADA servers (data acquisition, alarming, historian) at Level 2, and MES/HMI clients at Level 3 — implement redundant SCADA servers (hot standby) for critical processes, use a centralized historian with compression (swinging door or boxcar) for long-term trend storage, and design alarm management following ISA-18.2 (rationalization, shelving, suppression, metrics like alarm rate per operator per hour).

4. Integrate with MES (Manufacturing Execution System) by implementing ISA-95 (B2MML) interfaces between Level 3 (MES) and Level 2 (control) — exchange work orders, production schedules, material consumption, quality results, and equipment status using standardized message formats, and implement store-and-forward buffering so production continues uninterrupted during MES connectivity outages.

5. Build digital twin models that mirror physical assets at the appropriate fidelity level: physics-based simulation models for process optimization and what-if analysis, real-time synchronized state models for monitoring and anomaly detection, and reduced-order models for edge deployment — feed twins with live sensor data via OPC-UA or MQTT, maintain bidirectional synchronization, and use the twin for virtual commissioning of control logic before deployment to physical equipment.

6. Implement cobot (collaborative robot) safety following ISO/TS 15066: configure safety-rated monitored stop, hand guiding, speed and separation monitoring (SSM), or power and force limiting (PFL) modes based on the application risk assessment — define safety zones with configurable boundaries using safety-rated laser scanners or 3D cameras, reduce speed/force as the human approaches, and stop before contact forces exceed biomechanical pain/injury thresholds defined in the standard.

7. Optimize pick-and-place operations by integrating vision-guided robotics (2D or 3D camera for part localization), computing grasp poses based on part geometry and gripper capabilities, planning collision-free paths with cycle time minimization, and implementing robust error recovery for common failures (missed picks, dropped parts, conveyor jams) — use bin-picking algorithms with point cloud segmentation for unstructured environments and conveyor tracking for moving-line applications.

8. Design conveyor control systems with proper motion coordination: implement accumulation logic (zero-pressure, minimum-pressure) to prevent product damage, zone-based tracking that maintains product identity and sequence through merges/diverts, speed synchronization between conveyor sections and robotic stations, and jam detection with automatic recovery — interface with sortation systems using barcode/RFID readers and divert-confirm sensors.

9. Build industrial IoT data pipelines that collect high-frequency machine data (vibration, temperature, power, cycle counts) from sensors via industrial protocols (Modbus, EtherNet/IP, PROFINET, MQTT Sparkplug B), perform edge preprocessing (downsampling, feature extraction, anomaly detection) to reduce bandwidth, and transmit to cloud platforms (AWS IoT, Azure IoT Hub) using store-and-forward agents that handle intermittent connectivity — always timestamp data at the source with synchronized clocks (PTP/IEEE 1588).

10. Implement predictive maintenance ML models by collecting labeled training data from historians (normal operation, degradation patterns, failure events), engineering features from raw sensor data (RMS vibration, spectral peaks, temperature trends, cycle time drift), training models (gradient boosting, LSTM, autoencoders for anomaly detection) with proper train/test splits respecting temporal ordering, and deploying inference at the edge with model versioning — output remaining useful life (RUL) estimates and maintenance work order triggers integrated with CMMS.

11. Design cell controllers as the orchestration layer between MES work orders and individual equipment controllers: manage recipe/parameter download to PLCs, coordinate multi-robot and multi-station sequences, track work-in-process through the cell, collect per-part quality and process data (genealogy/traceability), and handle mode transitions (automatic, manual, maintenance) with proper interlocking — implement the PackML (ISA-TR88.00.02) state model for consistent equipment behavior across the plant.

12. Implement production scheduling algorithms that optimize for throughput, changeover minimization, due date adherence, and energy cost — use constraint-based scheduling (job-shop or flow-shop models) with heuristics (dispatching rules, genetic algorithms) for NP-hard problems, integrate with MES for real-time schedule adherence monitoring, and support dynamic rescheduling when disruptions occur (machine breakdown, rush orders, material shortages) by re-optimizing the remaining schedule horizon.

13. Secure industrial control systems following IEC 62443: segment networks into zones and conduits with firewalls at boundaries (Purdue model levels), implement role-based access control on all HMI and engineering workstations, disable unnecessary services and ports on PLCs and network switches, deploy intrusion detection systems that understand industrial protocols, and maintain a patching strategy that balances security with production uptime — never expose control system networks directly to the internet.

14. Implement machine vision quality inspection systems using industrial cameras (area scan or line scan based on throughput requirements), proper lighting design (backlighting, structured light, diffuse dome) to maximize defect contrast, and image processing pipelines (traditional CV for dimensional measurement, deep learning for surface defect classification) — calibrate cameras for metric accuracy, validate detection rates (sensitivity/specificity) against golden sample sets, and integrate pass/fail results with PLC reject mechanisms and MES quality records.
