---
trigger: "quantum OS|quantum operating system|Qiskit Runtime|Azure Quantum|Amazon Braket|ARTIQ|quantum control plane|quantum cloud|QNodeOS"
category: quantum
allowed_tools: ["read_file", "write_file", "bash"]
---

# Quantum Operating Systems

Best practices for quantum control planes and operating systems:

1. **Qiskit Runtime (IBM)**: Use Sessions for iterative algorithms (VQE, QAOA). Prefer `Estimator`/`Sampler` primitives. Enable error mitigation with `resilience_level=1+`.
2. **Azure Quantum (Microsoft)**: Use the `azure.quantum` Python SDK. Submit Q#, Qiskit, or Cirq jobs. Set `target` to specific hardware (ionq.simulator, quantinuum.qpu.h1-1).
3. **Amazon Braket**: Use `AwsDevice` for hardware, `LocalSimulator` for development. Braket Hybrid Jobs for long-running variational algorithms. S3 stores results automatically.
4. **Google Quantum Engine**: Access via Cirq's `cirq_google.Engine`. Requires Google Cloud project. Use `cirq_google.optimized_for_sycamore()` for hardware.
5. **ARTIQ (M-Labs)**: Real-time instrument control for trapped-ion and neutral-atom labs. Write kernels in ARTIQ Python subset. Sub-nanosecond timing via FPGA.
6. **QUA / Quantum Machines OPX+**: Pulse-level control language. Define sequences with `with program()` context. Real-time classical feedback within quantum loops.
7. **Q-CTRL Boulder Opal / Fire Opal**: Firmware-level pulse optimisation. Use `qctrl.create_optimization()` for automated error suppression. Fire Opal wraps existing Qiskit circuits.
8. **Mitiq (Unitary Fund)**: Software-level error mitigation. Supports ZNE (zero-noise extrapolation), PEC, CDR, DDD. Wraps any executor function.
9. **Qibo (TII)**: Full-stack: compiler + simulator + hardware drivers. Use `qibo.models.Circuit` for portable circuits. Supports GPU acceleration.
10. **QNodeOS (QuTech)**: Prototype quantum network operating system. Enables entanglement distribution across network nodes. Research-stage.
11. **Rigetti QCS**: Access Aspen-M processors via `pyquil`. Use Quil language or compile from Qiskit/Cirq. Quilc compiler handles routing.
12. **staq (Princeton/Yale)**: Full quantum compiler toolchain. Parses OpenQASM, optimises, and maps to hardware. CLI-based workflow.
13. **Keysight True-Q**: Hardware characterisation and error mitigation. Generate randomised benchmarking, process tomography, and noise reconstruction protocols.
14. **Multi-cloud strategy**: Use pytket or PennyLane as abstraction layers to target multiple quantum cloud providers from a single codebase.
15. **Job management**: Always implement retry logic and result caching for cloud quantum jobs — hardware queues can be hours long. Store job IDs for async retrieval.
