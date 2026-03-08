---
triggers: ["LabVIEW", "National Instruments", "NI LabVIEW", "virtual instrument", "VI", "dataflow programming", "DAQ LabVIEW", "FPGA LabVIEW", "G language"]
tools_allowed: ["read_file", "write_file", "bash"]
category: scientific
---

# LabVIEW

When developing LabVIEW (G language) applications for test, measurement, and automation:

1. LabVIEW uses dataflow programming: data flows along wires between nodes — a node executes when all inputs arrive; parallel branches execute concurrently by default; this is fundamentally different from text-based sequential programming.
2. Structure VIs (Virtual Instruments) with front panel (UI) and block diagram (logic): controls = inputs (knobs, buttons, text boxes), indicators = outputs (graphs, LEDs, displays) — keep front panels clean and organized.
3. Use SubVIs for modularity: create SubVIs with defined connector panes (inputs on left, outputs on right) — follow the 4-2-2-4 connector pattern; use icon editor to create meaningful icons; SubVIs are LabVIEW's equivalent of functions.
4. Use proper error handling: wire error clusters (error in → function → error out) through all nodes in sequence — use Case Structure with error wire to handle errors; `Simple Error Handler.vi` for user-facing error dialogs.
5. State machine architecture: use `While Loop` + `Case Structure` + `Enum` for state control — define states: Initialize, Acquire, Process, Display, Shutdown, Error; shift registers carry state between iterations.
6. For data acquisition: use DAQmx VIs — `DAQmx Create Channel` → `DAQmx Timing` → `DAQmx Start Task` → `DAQmx Read` → `DAQmx Clear Task` — configure sample rate, buffer size, and trigger before starting.
7. Use producer-consumer design pattern for real-time systems: producer loop acquires data and enqueues; consumer loop dequeues and processes — Queues decouple acquisition rate from processing rate; prevents data loss.
8. FPGA development: use LabVIEW FPGA Module for deterministic, hardware-timed execution — single-cycle timed loops for maximum throughput; use FIFOs for FPGA-to-host data transfer; compile times can be long (minutes to hours).
9. Data types: use clusters (structs) to group related data; use type definitions (`.ctl` files) for shared cluster types — if you change the typedef, all VIs using it update automatically; use enums for state machines and mode selection.
10. Avoid common mistakes: don't use local variables for data flow (use wires); don't create race conditions with parallel writes to the same variable; don't leave unwired SubVI terminals (can cause unexpected behavior).
11. Use LabVIEW's built-in analysis: `Spectral Measurements`, `Filter`, `Statistics`, `Curve Fitting`, `PID` — Express VIs provide quick configuration dialogs but use standard VIs for production code (better control and performance).
12. Deploy with Application Builder: create standalone executables (.exe) or installers; use LabVIEW Runtime Engine for distribution; use Real-Time Module for deterministic execution on RT targets (CompactRIO, PXI).
