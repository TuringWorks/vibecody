---
triggers: ["embedded real-time", "RTOS", "real-time operating system", "VxWorks", "FreeRTOS", "Zephyr", "WCET", "rate monotonic", "priority inversion", "interrupt handler", "bare metal", "embedded systems"]
tools_allowed: ["read_file", "write_file", "bash"]
category: safety-critical
---

# Embedded Real-Time Systems

When developing embedded real-time software for safety-critical and mission-critical applications:

1. Choose RTOS by certification needs: VxWorks 653/INTEGRITY-178 for DO-178C DAL A; SafeRTOS for IEC 61508 SIL 3; FreeRTOS for non-certified prototyping; Zephyr for IoT with safety aspirations; RTEMS for space missions — the RTOS is part of the safety case.
2. Use Rate Monotonic Scheduling (RMS) for static priority assignment: shorter period = higher priority; schedulability test: `sum(Ci/Ti) <= n(2^(1/n) - 1)` — for n tasks, utilization must be below the bound (69.3% for many tasks); exact analysis uses response time calculation.
3. Prevent priority inversion: use priority inheritance protocol (PIP) or priority ceiling protocol (PCP) on mutexes — PIP temporarily raises the holder's priority; PCP pre-assigns ceiling priorities; both prevent unbounded blocking of high-priority tasks.
4. Keep interrupt handlers minimal: save context, clear interrupt source, set a flag or post to a semaphore, restore context — do NOT perform computation, memory allocation, or I/O in ISRs; defer work to task context via deferred procedure calls or event flags.
5. Perform WCET (Worst-Case Execution Time) analysis: use static analysis tools (aiT, Bound-T, OTAWA) for sound upper bounds, or measurement-based approaches (RapiTime) with coverage evidence — WCET must be less than the task's deadline with defined margin.
6. Avoid unbounded operations in real-time code: no dynamic memory allocation, no unbounded loops, no recursive calls, no priority-uncontrolled blocking — every operation must have a known, bounded execution time; use `O(1)` or `O(log n)` algorithms.
7. Use memory protection units (MPU/MMU): configure regions to isolate tasks from each other and from the kernel; stack overflow detection via MPU guard regions; mark code as read-only/execute-only; mark data as no-execute — containment prevents fault propagation.
8. Design for deterministic communication: use message queues with bounded depth for inter-task communication; avoid shared memory without mutual exclusion; use zero-copy message passing for large payloads; timeout all blocking operations.
9. Implement watchdog timer patterns: `window watchdog` requires kick within a time window (not too early, not too late); `sequence watchdog` verifies task execution order; `logical watchdog` checks program flow with checkpoint tokens — multi-level watchdogs for defense in depth.
10. Handle timing in hardware: use hardware timers for periodic task activation (not software delay loops); read high-resolution timer counters for timestamps; synchronize distributed systems with PTP (IEEE 1588) or GPS PPS signals.
11. Debug with non-intrusive tools: use JTAG/SWD for breakpoints and memory inspection; use trace ports (ETM, ITM) for real-time execution tracing without stopping the processor; use logic analyzers for timing verification on external interfaces.
12. Test real-time properties: measure jitter (variation in period), latency (interrupt-to-response time), and throughput under worst-case load; inject fault conditions (overload, clock drift, communication loss) — timing requirements are functional requirements.
13. Use lock-free data structures where possible: single-producer/single-consumer ring buffers need no locks; use atomic operations (`compare_and_swap`) for shared counters; lock-free designs eliminate priority inversion and deadlock risks.
14. Document the timing architecture: task table (name, priority, period, deadline, WCET, stack size), interrupt table (source, priority, handler, max latency), and communication matrix (producer, consumer, mechanism, max latency) — this is a key review artifact.
