---
trigger: "quantum language|quantum programming|Qiskit setup|Cirq setup|Q# setup|OpenQASM|PennyLane|quantum SDK|quantum framework"
category: quantum
allowed_tools: ["read_file", "write_file", "bash"]
---

# Quantum Programming Languages

Best practices for working with quantum programming languages and frameworks:

1. **Qiskit (IBM)**: Use `QuantumCircuit` for circuit construction, `AerSimulator` for local testing, `qiskit-ibm-runtime` for cloud execution. Prefer `SamplerV2`/`EstimatorV2` primitives over legacy `execute()`.
2. **Cirq (Google)**: Use `cirq.LineQubit` for linear topologies, `cirq.GridQubit` for 2D. Leverage `cirq.optimize_for_target_gateset()` for hardware mapping.
3. **Q# (Microsoft)**: Use `@EntryPoint()` for standalone programs, `MResetZ()` for measure-and-reset. Target Azure Quantum via `azure-quantum` Python package.
4. **OpenQASM 3.0**: Prefer v3 over v2 — supports classical control flow, subroutines, and timing. Use `include "stdgates.inc"` for standard gate library.
5. **PennyLane (Xanadu)**: Use `@qml.qnode` decorators for hybrid quantum-classical workflows. Supports automatic differentiation through quantum circuits.
6. **CUDA Quantum (NVIDIA)**: Use for GPU-accelerated quantum simulation. Supports C++ and Python frontends with `cudaq.kernel` decorators.
7. **t|ket⟩ (Quantinuum)**: Use `pytket` for cross-platform circuit optimisation. Supports routing to any backend via `pytket-*` extension packages.
8. **Amazon Braket SDK**: Use `from braket.circuits import Circuit` for vendor-neutral circuits. Supports IonQ, Rigetti, OQC hardware + simulators.
9. **Bloqade (QuEra)**: For neutral-atom quantum computing. Define Hamiltonians with `bloqade.start.add_position()` chains.
10. **Strawberry Fields (Xanadu)**: For photonic/continuous-variable quantum computing. Use `sf.Program()` context manager.
11. **Stim (Google)**: For fast stabiliser simulation and quantum error correction research. Handles millions of qubits for Clifford circuits.
12. **QIR (Quantum Intermediate Representation)**: LLVM-based IR for quantum programs. Use `pyqir` for programmatic generation.
13. **Cross-compilation**: Use pytket or Qiskit transpiler to convert between frameworks. Always verify gate set compatibility with target hardware.
14. **Testing strategy**: Always test on simulators first. Use noise models (`qiskit_aer.noise`, `cirq.ConstantQubitNoiseModel`) before hardware runs. Compare simulator vs hardware results statistically.
