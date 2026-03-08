---
triggers: ["Ladder Logic", "PLC programming", "programmable logic controller", "IEC 61131-3", "structured text", "function block diagram", "Allen-Bradley", "Siemens PLC", "SCADA"]
tools_allowed: ["read_file", "write_file", "bash"]
category: industrial
---

# Ladder Logic & PLC Programming

When programming PLCs under IEC 61131-3:

1. IEC 61131-3 defines 5 languages: Ladder Diagram (LD), Function Block Diagram (FBD), Structured Text (ST), Instruction List (IL, deprecated), and Sequential Function Chart (SFC) — choose based on task: LD for discrete logic, ST for complex math, FBD for process control, SFC for sequences.
2. In Ladder Logic: left rail = power, right rail = return; contacts (inputs) in series = AND; contacts in parallel = OR; coils (outputs) on the right — `|--[ X1 ]--[ X2 ]--( Y1 )--|` means Y1 = X1 AND X2.
3. Use standard function blocks: `TON` (on-delay timer), `TOF` (off-delay timer), `CTU` (count up), `CTD` (count down), `SR` (set-reset latch), `RS` (reset-set latch) — timers use time base (ms/s); always define preset values.
4. Scan cycle awareness: PLC reads all inputs → executes program top-to-bottom → writes all outputs → repeat — one scan takes 1-50ms; logic must complete within scan time; avoid long loops in Structured Text.
5. Use Structured Text for calculations: `speed := (encoder_count * 60.0) / (time_elapsed * pulses_per_rev);` — ST looks like Pascal; supports `IF/THEN/ELSE`, `CASE`, `FOR/WHILE/REPEAT` loops, `FUNCTION` and `FUNCTION_BLOCK`.
6. Organize with Program Organization Units (POUs): `PROGRAM` for main logic, `FUNCTION_BLOCK` for reusable stateful components (PID controller, motor starter), `FUNCTION` for stateless calculations (unit conversion).
7. Follow naming conventions: prefix I/O with type — `DI_StartButton` (digital input), `DO_MotorRun` (digital output), `AI_Temperature` (analog input), `AO_ValvePosition` (analog output); use structured tags for complex data.
8. Implement safety logic: emergency stop circuits in hardware AND software; use safety-rated PLCs (SIL 3/PLe) for safety functions; dual-channel inputs with discrepancy monitoring; safe state = all outputs off.
9. Handle analog signals: scale raw values (0-32767 for 4-20mA) to engineering units: `temp_degC := (raw_value / 32767.0) * (range_max - range_min) + range_min;` — filter noise with first-order digital filter or moving average.
10. Use Sequential Function Charts for multi-step processes: define steps (states), transitions (conditions), and actions — parallel branches for simultaneous operations; SFC prevents sequence errors by enforcing valid state transitions.
11. Implement HMI communication: define tag databases shared between PLC and SCADA/HMI; use standard protocols (OPC UA, Modbus TCP, EtherNet/IP, PROFINET); minimize HMI poll rates on critical I/O to reduce scan time impact.
12. Version control and testing: use TIA Portal/Studio 5000 project archives; document changes in a changelog; test logic offline with PLC simulator before downloading to hardware; implement factory acceptance test (FAT) procedures with documented test cases.
