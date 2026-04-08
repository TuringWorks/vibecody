//! Quantum computing programming languages, quantum operating systems,
//! circuit design helpers, and hardware backend management.

use std::collections::HashMap;

// ── Quantum Programming Languages ────────────────────────────────────────────

/// Supported quantum programming languages and frameworks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QuantumLanguage {
    Qiskit,        // Python — IBM Quantum
    Cirq,          // Python — Google Quantum AI
    PennyLane,     // Python — Xanadu (differentiable quantum)
    QSharp,        // Q# — Microsoft Azure Quantum
    Quipper,       // Haskell-embedded — scalable quantum
    Silq,          // High-level quantum language (ETH Zürich)
    OpenQASM2,     // Open Quantum Assembly Language v2
    OpenQASM3,     // Open Quantum Assembly Language v3
    Scaffold,      // C-like quantum language (Princeton)
    ProjectQ,      // Python — ETH Zürich
    Strawberry,    // Strawberry Fields — Xanadu photonic
    TKet,          // Quantinuum t|ket⟩ SDK
    BraketSDK,     // Amazon Braket SDK
    CudaQuantum,   // NVIDIA CUDA Quantum (C++ / Python)
    Qulacs,        // C++/Python high-performance simulator
    Stim,          // Google — stabilizer circuit simulator
    Bloqade,       // QuEra — neutral atom quantum
    IonQ,          // IonQ native SDK
    QirAlliance,   // Quantum Intermediate Representation
    Twist,         // MIT — purity-based quantum language
}

impl QuantumLanguage {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Qiskit => "Qiskit",
            Self::Cirq => "Cirq",
            Self::PennyLane => "PennyLane",
            Self::QSharp => "Q#",
            Self::Quipper => "Quipper",
            Self::Silq => "Silq",
            Self::OpenQASM2 => "OpenQASM 2.0",
            Self::OpenQASM3 => "OpenQASM 3.0",
            Self::Scaffold => "Scaffold",
            Self::ProjectQ => "ProjectQ",
            Self::Strawberry => "Strawberry Fields",
            Self::TKet => "t|ket⟩",
            Self::BraketSDK => "Amazon Braket SDK",
            Self::CudaQuantum => "CUDA Quantum",
            Self::Qulacs => "Qulacs",
            Self::Stim => "Stim",
            Self::Bloqade => "Bloqade",
            Self::IonQ => "IonQ SDK",
            Self::QirAlliance => "QIR (Quantum Intermediate Representation)",
            Self::Twist => "Twist",
        }
    }

    pub fn host_language(&self) -> &'static str {
        match self {
            Self::Qiskit | Self::Cirq | Self::PennyLane | Self::ProjectQ
            | Self::Strawberry | Self::BraketSDK | Self::Qulacs
            | Self::Bloqade | Self::IonQ => "Python",
            Self::QSharp => "Q# (standalone / Python host)",
            Self::Quipper => "Haskell",
            Self::Silq => "Silq (standalone)",
            Self::OpenQASM2 | Self::OpenQASM3 => "QASM (standalone)",
            Self::Scaffold => "C-like (standalone)",
            Self::TKet => "Python / C++",
            Self::CudaQuantum => "C++ / Python",
            Self::Stim => "Python / C++",
            Self::QirAlliance => "LLVM IR",
            Self::Twist => "Twist (standalone)",
        }
    }

    pub fn vendor(&self) -> &'static str {
        match self {
            Self::Qiskit => "IBM Quantum",
            Self::Cirq | Self::Stim => "Google Quantum AI",
            Self::PennyLane | Self::Strawberry => "Xanadu",
            Self::QSharp => "Microsoft",
            Self::Quipper => "Dalhousie / IARPA",
            Self::Silq => "ETH Zürich",
            Self::OpenQASM2 | Self::OpenQASM3 => "IBM / OpenQASM Spec",
            Self::Scaffold => "Princeton",
            Self::ProjectQ => "ETH Zürich",
            Self::TKet => "Quantinuum",
            Self::BraketSDK => "Amazon Web Services",
            Self::CudaQuantum => "NVIDIA",
            Self::Qulacs => "QunaSys / Osaka Univ",
            Self::Bloqade => "QuEra Computing",
            Self::IonQ => "IonQ",
            Self::QirAlliance => "QIR Alliance (Microsoft + others)",
            Self::Twist => "MIT CSAIL",
        }
    }

    pub fn install_command(&self) -> &'static str {
        match self {
            Self::Qiskit => "pip install qiskit qiskit-aer qiskit-ibm-runtime",
            Self::Cirq => "pip install cirq",
            Self::PennyLane => "pip install pennylane pennylane-qiskit",
            Self::QSharp => "dotnet new -i Microsoft.Quantum.ProjectTemplates && pip install qsharp",
            Self::Quipper => "cabal install quipper",
            Self::Silq => "# Download from https://silq.ethz.ch",
            Self::OpenQASM2 | Self::OpenQASM3 => "pip install openqasm3",
            Self::Scaffold => "# Build from https://github.com/epiqc/ScaffCC",
            Self::ProjectQ => "pip install projectq",
            Self::Strawberry => "pip install strawberryfields",
            Self::TKet => "pip install pytket",
            Self::BraketSDK => "pip install amazon-braket-sdk",
            Self::CudaQuantum => "pip install cuda-quantum",
            Self::Qulacs => "pip install qulacs",
            Self::Stim => "pip install stim",
            Self::Bloqade => "pip install bloqade",
            Self::IonQ => "pip install qiskit-ionq",
            Self::QirAlliance => "pip install pyqir",
            Self::Twist => "# Build from https://github.com/psg-mit/twist-popl22",
        }
    }

    pub fn hello_circuit(&self) -> &'static str {
        match self {
            Self::Qiskit => r#"from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator

qc = QuantumCircuit(2, 2)
qc.h(0)           # Hadamard on qubit 0
qc.cx(0, 1)       # CNOT — creates Bell state |Φ+⟩
qc.measure([0, 1], [0, 1])

sim = AerSimulator()
result = sim.run(qc, shots=1024).result()
print(result.get_counts())
"#,
            Self::Cirq => r#"import cirq

q0, q1 = cirq.LineQubit.range(2)
circuit = cirq.Circuit([
    cirq.H(q0),
    cirq.CNOT(q0, q1),
    cirq.measure(q0, q1, key='result'),
])

sim = cirq.Simulator()
result = sim.run(circuit, repetitions=1024)
print(result.histogram(key='result'))
"#,
            Self::QSharp => r#"namespace BellState {
    open Microsoft.Quantum.Canon;
    open Microsoft.Quantum.Intrinsic;
    open Microsoft.Quantum.Measurement;

    @EntryPoint()
    operation Main() : (Result, Result) {
        use (q0, q1) = (Qubit(), Qubit());
        H(q0);
        CNOT(q0, q1);
        let r0 = MResetZ(q0);
        let r1 = MResetZ(q1);
        return (r0, r1);
    }
}
"#,
            Self::OpenQASM3 => r#"OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;
bit[2] c;

h q[0];
cx q[0], q[1];

c = measure q;
"#,
            Self::PennyLane => r#"import pennylane as qml
from pennylane import numpy as np

dev = qml.device("default.qubit", wires=2)

@qml.qnode(dev)
def bell_state():
    qml.Hadamard(wires=0)
    qml.CNOT(wires=[0, 1])
    return qml.probs(wires=[0, 1])

print(bell_state())
"#,
            _ => "// See official documentation for this language's hello-world circuit.",
        }
    }

    pub fn all() -> Vec<QuantumLanguage> {
        vec![
            Self::Qiskit, Self::Cirq, Self::PennyLane, Self::QSharp,
            Self::Quipper, Self::Silq, Self::OpenQASM2, Self::OpenQASM3,
            Self::Scaffold, Self::ProjectQ, Self::Strawberry, Self::TKet,
            Self::BraketSDK, Self::CudaQuantum, Self::Qulacs, Self::Stim,
            Self::Bloqade, Self::IonQ, Self::QirAlliance, Self::Twist,
        ]
    }
}

// ── Quantum Operating Systems ────────────────────────────────────────────────

/// Known quantum operating systems and control-plane software.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QuantumOS {
    QiskitRuntime,       // IBM Qiskit Runtime (cloud control plane)
    AzureQuantum,        // Microsoft Azure Quantum resource manager
    AmazonBraket,        // AWS Braket orchestration layer
    CirqEngine,          // Google Quantum Engine (Cirq backend)
    QuOS,                // Quantum Machines — QUA language runtime
    Artiq,               // M-Labs — Advanced Real-Time Infrastructure for Quantum
    QCtrl,               // Q-CTRL — firmware-level pulse optimisation
    Mitiq,               // Unitary Fund — quantum error mitigation OS layer
    Qibo,                // TII (UAE) — full-stack quantum OS
    PulseOS,             // Oxford Quantum Circuits — pulse-level control
    Staq,                // Princeton — full-stack quantum compiler OS
    Delft,               // QuTech — quantum network OS (QNodeOS prototype)
    FireOpal,            // Q-CTRL Fire Opal — automated error suppression
    TrueQ,               // Keysight True-Q — characterisation & mitigation
    Qcs,                 // Rigetti Quantum Cloud Services
}

impl QuantumOS {
    pub fn name(&self) -> &'static str {
        match self {
            Self::QiskitRuntime => "Qiskit Runtime",
            Self::AzureQuantum => "Azure Quantum",
            Self::AmazonBraket => "Amazon Braket",
            Self::CirqEngine => "Google Quantum Engine",
            Self::QuOS => "QUA / Quantum Machines OPX+",
            Self::Artiq => "ARTIQ",
            Self::QCtrl => "Q-CTRL Boulder Opal",
            Self::Mitiq => "Mitiq",
            Self::Qibo => "Qibo",
            Self::PulseOS => "Oxford QC Pulse OS",
            Self::Staq => "staq",
            Self::Delft => "QNodeOS (QuTech)",
            Self::FireOpal => "Q-CTRL Fire Opal",
            Self::TrueQ => "Keysight True-Q",
            Self::Qcs => "Rigetti QCS",
        }
    }

    pub fn layer(&self) -> &'static str {
        match self {
            Self::QiskitRuntime | Self::AzureQuantum | Self::AmazonBraket
            | Self::CirqEngine | Self::Qcs => "Cloud orchestration",
            Self::QuOS | Self::Artiq | Self::PulseOS => "Hardware control plane",
            Self::QCtrl | Self::FireOpal | Self::TrueQ => "Error mitigation / firmware",
            Self::Mitiq => "Error mitigation (software)",
            Self::Qibo => "Full-stack (compiler + runtime)",
            Self::Staq => "Compiler + optimiser",
            Self::Delft => "Quantum network OS",
        }
    }

    pub fn vendor(&self) -> &'static str {
        match self {
            Self::QiskitRuntime => "IBM Quantum",
            Self::AzureQuantum => "Microsoft",
            Self::AmazonBraket => "Amazon Web Services",
            Self::CirqEngine => "Google Quantum AI",
            Self::QuOS => "Quantum Machines",
            Self::Artiq => "M-Labs (NIST / Oxford)",
            Self::QCtrl | Self::FireOpal => "Q-CTRL",
            Self::Mitiq => "Unitary Fund",
            Self::Qibo => "Technology Innovation Institute",
            Self::PulseOS => "Oxford Quantum Circuits",
            Self::Staq => "Princeton / Yale",
            Self::Delft => "QuTech (TU Delft + TNO)",
            Self::TrueQ => "Keysight Technologies",
            Self::Qcs => "Rigetti Computing",
        }
    }

    pub fn all() -> Vec<QuantumOS> {
        vec![
            Self::QiskitRuntime, Self::AzureQuantum, Self::AmazonBraket,
            Self::CirqEngine, Self::QuOS, Self::Artiq, Self::QCtrl,
            Self::Mitiq, Self::Qibo, Self::PulseOS, Self::Staq,
            Self::Delft, Self::FireOpal, Self::TrueQ, Self::Qcs,
        ]
    }
}

// ── Quantum Hardware Backends ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QuantumHardwareType {
    Superconducting,
    TrappedIon,
    Photonic,
    NeutralAtom,
    TopologicalQubit,
    NVCenter,
    QuantumDot,
    AnnealingProcessor,
}

impl QuantumHardwareType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Superconducting => "Superconducting transmon",
            Self::TrappedIon => "Trapped-ion",
            Self::Photonic => "Photonic",
            Self::NeutralAtom => "Neutral atom",
            Self::TopologicalQubit => "Topological qubit",
            Self::NVCenter => "Nitrogen-vacancy (NV) center",
            Self::QuantumDot => "Quantum dot / spin qubit",
            Self::AnnealingProcessor => "Quantum annealing processor",
        }
    }

    pub fn leading_vendors(&self) -> &'static [&'static str] {
        match self {
            Self::Superconducting => &["IBM", "Google", "Rigetti", "Oxford QC", "IQM"],
            Self::TrappedIon => &["IonQ", "Quantinuum (Honeywell)", "Alpine Quantum"],
            Self::Photonic => &["Xanadu", "PsiQuantum", "Quandela"],
            Self::NeutralAtom => &["QuEra", "Pasqal", "Atom Computing"],
            Self::TopologicalQubit => &["Microsoft (Station Q)"],
            Self::NVCenter => &["Element Six / De Beers", "Quantum Brilliance"],
            Self::QuantumDot => &["Intel", "Silicon Quantum Computing"],
            Self::AnnealingProcessor => &["D-Wave"],
        }
    }
}

// ── Quantum Algorithms ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QuantumAlgorithm {
    Grover,
    Shor,
    Vqe,
    Qaoa,
    Qpe,
    BernsteinVazirani,
    DeutschJozsa,
    SimonProblem,
    Hhl,
    QuantumWalk,
    QuantumMonteCarlo,
    Qsvm,
    Qnn,
    QuantumBoltzmann,
    Dmrg,
}

impl QuantumAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Grover => "Grover's search",
            Self::Shor => "Shor's factoring",
            Self::Vqe => "Variational Quantum Eigensolver (VQE)",
            Self::Qaoa => "Quantum Approximate Optimisation (QAOA)",
            Self::Qpe => "Quantum Phase Estimation (QPE)",
            Self::BernsteinVazirani => "Bernstein–Vazirani",
            Self::DeutschJozsa => "Deutsch–Jozsa",
            Self::SimonProblem => "Simon's problem",
            Self::Hhl => "HHL (linear systems)",
            Self::QuantumWalk => "Quantum walk",
            Self::QuantumMonteCarlo => "Quantum Monte Carlo",
            Self::Qsvm => "Quantum SVM",
            Self::Qnn => "Quantum Neural Network",
            Self::QuantumBoltzmann => "Quantum Boltzmann Machine",
            Self::Dmrg => "DMRG / tensor-network",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::Grover | Self::BernsteinVazirani | Self::DeutschJozsa
            | Self::SimonProblem => "Oracle / search",
            Self::Shor | Self::Qpe => "Number-theoretic",
            Self::Vqe | Self::Qaoa => "Variational (NISQ-friendly)",
            Self::Hhl | Self::QuantumMonteCarlo => "Linear algebra / simulation",
            Self::QuantumWalk => "Graph / combinatorial",
            Self::Qsvm | Self::Qnn | Self::QuantumBoltzmann => "Quantum machine learning",
            Self::Dmrg => "Tensor-network / chemistry",
        }
    }

    pub fn qubit_scaling(&self) -> &'static str {
        match self {
            Self::Grover => "O(√N) queries, N qubits for N-item search",
            Self::Shor => "O(n³) gates for n-bit integer (2n+3 qubits)",
            Self::Vqe => "Problem-dependent ansatz depth, typically 4-50 qubits (NISQ)",
            Self::Qaoa => "Problem-size + p rounds of alternating unitaries",
            Self::Qpe => "O(1/ε) ancilla qubits for precision ε",
            Self::BernsteinVazirani => "n qubits for n-bit secret",
            Self::DeutschJozsa => "n+1 qubits, single oracle query",
            Self::SimonProblem => "n qubits, O(n) queries",
            Self::Hhl => "O(log N) qubits for N×N system (exponential speedup)",
            Self::QuantumWalk => "O(log N) qubits for N-vertex graph",
            Self::QuantumMonteCarlo => "Quadratic speedup over classical MC",
            Self::Qsvm => "O(log N) qubits for N-dimensional feature space",
            Self::Qnn => "Parameterised circuit depth × width",
            Self::QuantumBoltzmann => "Visible + hidden qubit layers",
            Self::Dmrg => "Bond dimension dependent, hybrid classical-quantum",
        }
    }

    pub fn all() -> Vec<QuantumAlgorithm> {
        vec![
            Self::Grover, Self::Shor, Self::Vqe, Self::Qaoa, Self::Qpe,
            Self::BernsteinVazirani, Self::DeutschJozsa, Self::SimonProblem,
            Self::Hhl, Self::QuantumWalk, Self::QuantumMonteCarlo,
            Self::Qsvm, Self::Qnn, Self::QuantumBoltzmann, Self::Dmrg,
        ]
    }
}

// ── Error Correction Codes ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCorrectionCode {
    SurfaceCode,
    SteaneCode,
    ShorCode,
    ColorCode,
    ToricCode,
    BaconShorCode,
    CatCode,
    GKPCode,
    FlaggedFTEC,
}

impl ErrorCorrectionCode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SurfaceCode => "Surface code",
            Self::SteaneCode => "Steane [[7,1,3]] code",
            Self::ShorCode => "Shor [[9,1,3]] code",
            Self::ColorCode => "Color code",
            Self::ToricCode => "Toric code",
            Self::BaconShorCode => "Bacon–Shor code",
            Self::CatCode => "Cat code (bosonic)",
            Self::GKPCode => "GKP (Gottesman–Kitaev–Preskill)",
            Self::FlaggedFTEC => "Flagged FTEC (flag qubits)",
        }
    }

    pub fn physical_per_logical(&self) -> &'static str {
        match self {
            Self::SurfaceCode => "~1000:1 at code distance d=17 (threshold ~1%)",
            Self::SteaneCode => "7:1 (distance 3)",
            Self::ShorCode => "9:1 (distance 3)",
            Self::ColorCode => "~d² : 1 (similar to surface code)",
            Self::ToricCode => "2d²:1 on torus geometry",
            Self::BaconShorCode => "m×n subsystem for distance min(m,n)",
            Self::CatCode => "Hardware-efficient (continuous-variable)",
            Self::GKPCode => "Single bosonic mode per logical qubit",
            Self::FlaggedFTEC => "Code-dependent, reduces ancilla overhead",
        }
    }

    pub fn all() -> Vec<ErrorCorrectionCode> {
        vec![
            Self::SurfaceCode, Self::SteaneCode, Self::ShorCode,
            Self::ColorCode, Self::ToricCode, Self::BaconShorCode,
            Self::CatCode, Self::GKPCode, Self::FlaggedFTEC,
        ]
    }
}

// ── Quantum Circuit ──────────────────────────────────────────────────────────

/// A minimal quantum gate representation for circuit building.
#[derive(Debug, Clone, PartialEq)]
pub enum QuantumGate {
    H(usize),                      // Hadamard
    X(usize),                      // Pauli-X (NOT)
    Y(usize),                      // Pauli-Y
    Z(usize),                      // Pauli-Z
    S(usize),                      // Phase (√Z)
    T(usize),                      // π/8 gate (√S)
    Rx(usize, f64),                // Rotation around X
    Ry(usize, f64),                // Rotation around Y
    Rz(usize, f64),                // Rotation around Z
    Cnot(usize, usize),            // Controlled-NOT
    CZ(usize, usize),              // Controlled-Z
    Swap(usize, usize),            // SWAP
    Toffoli(usize, usize, usize),  // CCX (Toffoli)
    Measure(usize, usize),         // Measure qubit -> classical bit
}

impl QuantumGate {
    pub fn qasm3(&self) -> String {
        match self {
            Self::H(q) => format!("h q[{}];", q),
            Self::X(q) => format!("x q[{}];", q),
            Self::Y(q) => format!("y q[{}];", q),
            Self::Z(q) => format!("z q[{}];", q),
            Self::S(q) => format!("s q[{}];", q),
            Self::T(q) => format!("t q[{}];", q),
            Self::Rx(q, theta) => format!("rx({:.6}) q[{}];", theta, q),
            Self::Ry(q, theta) => format!("ry({:.6}) q[{}];", theta, q),
            Self::Rz(q, theta) => format!("rz({:.6}) q[{}];", theta, q),
            Self::Cnot(c, t) => format!("cx q[{}], q[{}];", c, t),
            Self::CZ(c, t) => format!("cz q[{}], q[{}];", c, t),
            Self::Swap(a, b) => format!("swap q[{}], q[{}];", a, b),
            Self::Toffoli(a, b, t) => format!("ccx q[{}], q[{}], q[{}];", a, b, t),
            Self::Measure(q, c) => format!("c[{}] = measure q[{}];", c, q),
        }
    }

    pub fn max_qubit(&self) -> usize {
        match self {
            Self::H(q) | Self::X(q) | Self::Y(q) | Self::Z(q)
            | Self::S(q) | Self::T(q)
            | Self::Rx(q, _) | Self::Ry(q, _) | Self::Rz(q, _) => *q,
            Self::Cnot(a, b) | Self::CZ(a, b) | Self::Swap(a, b) => (*a).max(*b),
            Self::Toffoli(a, b, c) => (*a).max(*b).max(*c),
            Self::Measure(q, _) => *q,
        }
    }
}

/// A quantum circuit composed of a sequence of gates.
#[derive(Debug, Clone)]
pub struct QuantumCircuit {
    pub name: String,
    pub num_qubits: usize,
    pub num_classical: usize,
    pub gates: Vec<QuantumGate>,
}

impl QuantumCircuit {
    pub fn new(name: &str, num_qubits: usize, num_classical: usize) -> Self {
        Self {
            name: name.to_string(),
            num_qubits,
            num_classical,
            gates: Vec::new(),
        }
    }

    pub fn add_gate(&mut self, gate: QuantumGate) {
        self.gates.push(gate);
    }

    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    pub fn depth(&self) -> usize {
        if self.gates.is_empty() {
            return 0;
        }
        let mut qubit_depth = vec![0usize; self.num_qubits];
        for gate in &self.gates {
            match gate {
                QuantumGate::H(q) | QuantumGate::X(q) | QuantumGate::Y(q)
                | QuantumGate::Z(q) | QuantumGate::S(q) | QuantumGate::T(q)
                | QuantumGate::Rx(q, _) | QuantumGate::Ry(q, _) | QuantumGate::Rz(q, _)
                | QuantumGate::Measure(q, _) => {
                    if *q < self.num_qubits {
                        qubit_depth[*q] += 1;
                    }
                }
                QuantumGate::Cnot(a, b) | QuantumGate::CZ(a, b) | QuantumGate::Swap(a, b) => {
                    if *a < self.num_qubits && *b < self.num_qubits {
                        let d = qubit_depth[*a].max(qubit_depth[*b]) + 1;
                        qubit_depth[*a] = d;
                        qubit_depth[*b] = d;
                    }
                }
                QuantumGate::Toffoli(a, b, c) => {
                    if *a < self.num_qubits && *b < self.num_qubits && *c < self.num_qubits {
                        let d = qubit_depth[*a].max(qubit_depth[*b]).max(qubit_depth[*c]) + 1;
                        qubit_depth[*a] = d;
                        qubit_depth[*b] = d;
                        qubit_depth[*c] = d;
                    }
                }
            }
        }
        qubit_depth.into_iter().max().unwrap_or(0)
    }

    pub fn two_qubit_gate_count(&self) -> usize {
        self.gates.iter().filter(|g| matches!(g,
            QuantumGate::Cnot(..) | QuantumGate::CZ(..) | QuantumGate::Swap(..)
        )).count()
    }

    pub fn to_qasm3(&self) -> String {
        let mut out = String::with_capacity(256);
        out.push_str("OPENQASM 3.0;\ninclude \"stdgates.inc\";\n\n");
        out.push_str(&format!("qubit[{}] q;\n", self.num_qubits));
        if self.num_classical > 0 {
            out.push_str(&format!("bit[{}] c;\n", self.num_classical));
        }
        out.push('\n');
        for gate in &self.gates {
            out.push_str(&gate.qasm3());
            out.push('\n');
        }
        out
    }

    /// Generate Qiskit Python code for this circuit.
    pub fn to_qiskit(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("from qiskit import QuantumCircuit\n\n");
        out.push_str(&format!(
            "qc = QuantumCircuit({}, {})\n",
            self.num_qubits, self.num_classical
        ));
        for gate in &self.gates {
            let line = match gate {
                QuantumGate::H(q) => format!("qc.h({})", q),
                QuantumGate::X(q) => format!("qc.x({})", q),
                QuantumGate::Y(q) => format!("qc.y({})", q),
                QuantumGate::Z(q) => format!("qc.z({})", q),
                QuantumGate::S(q) => format!("qc.s({})", q),
                QuantumGate::T(q) => format!("qc.t({})", q),
                QuantumGate::Rx(q, t) => format!("qc.rx({:.6}, {})", t, q),
                QuantumGate::Ry(q, t) => format!("qc.ry({:.6}, {})", t, q),
                QuantumGate::Rz(q, t) => format!("qc.rz({:.6}, {})", t, q),
                QuantumGate::Cnot(c, t) => format!("qc.cx({}, {})", c, t),
                QuantumGate::CZ(c, t) => format!("qc.cz({}, {})", c, t),
                QuantumGate::Swap(a, b) => format!("qc.swap({}, {})", a, b),
                QuantumGate::Toffoli(a, b, t) => format!("qc.ccx({}, {}, {})", a, b, t),
                QuantumGate::Measure(q, c) => format!("qc.measure({}, {})", q, c),
            };
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    /// Generate Cirq Python code for this circuit.
    pub fn to_cirq(&self) -> String {
        let mut out = String::with_capacity(512);
        out.push_str("import cirq\n\n");
        out.push_str(&format!(
            "qubits = cirq.LineQubit.range({})\n",
            self.num_qubits
        ));
        out.push_str("circuit = cirq.Circuit([\n");
        for gate in &self.gates {
            let line = match gate {
                QuantumGate::H(q) => format!("    cirq.H(qubits[{}]),", q),
                QuantumGate::X(q) => format!("    cirq.X(qubits[{}]),", q),
                QuantumGate::Y(q) => format!("    cirq.Y(qubits[{}]),", q),
                QuantumGate::Z(q) => format!("    cirq.Z(qubits[{}]),", q),
                QuantumGate::S(q) => format!("    cirq.S(qubits[{}]),", q),
                QuantumGate::T(q) => format!("    cirq.T(qubits[{}]),", q),
                QuantumGate::Rx(q, t) => format!("    cirq.rx({:.6})(qubits[{}]),", t, q),
                QuantumGate::Ry(q, t) => format!("    cirq.ry({:.6})(qubits[{}]),", t, q),
                QuantumGate::Rz(q, t) => format!("    cirq.rz({:.6})(qubits[{}]),", t, q),
                QuantumGate::Cnot(c, t) => format!("    cirq.CNOT(qubits[{}], qubits[{}]),", c, t),
                QuantumGate::CZ(c, t) => format!("    cirq.CZ(qubits[{}], qubits[{}]),", c, t),
                QuantumGate::Swap(a, b) => format!("    cirq.SWAP(qubits[{}], qubits[{}]),", a, b),
                QuantumGate::Toffoli(a, b, t) => format!("    cirq.CCX(qubits[{}], qubits[{}], qubits[{}]),", a, b, t),
                QuantumGate::Measure(q, _) => format!("    cirq.measure(qubits[{}], key='m{}'),", q, q),
            };
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str("])\n");
        out
    }
}

// ── Quantum Computing Manager ────────────────────────────────────────────────

pub struct QuantumComputingManager {
    next_id: u64,
    pub circuits: Vec<QuantumCircuit>,
    pub projects: Vec<QuantumProject>,
    pub hardware_prefs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct QuantumProject {
    pub id: String,
    pub name: String,
    pub language: QuantumLanguage,
    pub target_os: Option<QuantumOS>,
    pub target_hardware: QuantumHardwareType,
    pub algorithm: Option<QuantumAlgorithm>,
    pub error_correction: Option<ErrorCorrectionCode>,
    pub num_qubits: usize,
    pub description: String,
}

impl Default for QuantumComputingManager {
    fn default() -> Self { Self::new() }
}

impl QuantumComputingManager {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            circuits: Vec::new(),
            projects: Vec::new(),
            hardware_prefs: HashMap::new(),
        }
    }

    fn gen_id(&mut self, prefix: &str) -> String {
        let id = format!("{}-{:04}", prefix, self.next_id);
        self.next_id += 1;
        id
    }

    pub fn create_project(
        &mut self,
        name: &str,
        language: QuantumLanguage,
        target_hardware: QuantumHardwareType,
        num_qubits: usize,
        description: &str,
    ) -> String {
        let id = self.gen_id("QP");
        self.projects.push(QuantumProject {
            id: id.clone(),
            name: name.to_string(),
            language,
            target_os: None,
            target_hardware,
            algorithm: None,
            error_correction: None,
            num_qubits,
            description: description.to_string(),
        });
        id
    }

    pub fn set_project_os(&mut self, project_id: &str, os: QuantumOS) -> bool {
        if let Some(p) = self.projects.iter_mut().find(|p| p.id == project_id) {
            p.target_os = Some(os);
            true
        } else {
            false
        }
    }

    pub fn set_project_algorithm(&mut self, project_id: &str, alg: QuantumAlgorithm) -> bool {
        if let Some(p) = self.projects.iter_mut().find(|p| p.id == project_id) {
            p.algorithm = Some(alg);
            true
        } else {
            false
        }
    }

    pub fn set_project_ecc(&mut self, project_id: &str, ecc: ErrorCorrectionCode) -> bool {
        if let Some(p) = self.projects.iter_mut().find(|p| p.id == project_id) {
            p.error_correction = Some(ecc);
            true
        } else {
            false
        }
    }

    pub fn get_project(&self, project_id: &str) -> Option<&QuantumProject> {
        self.projects.iter().find(|p| p.id == project_id)
    }

    pub fn list_projects(&self) -> &[QuantumProject] {
        &self.projects
    }

    pub fn delete_project(&mut self, project_id: &str) -> bool {
        let len = self.projects.len();
        self.projects.retain(|p| p.id != project_id);
        self.projects.len() < len
    }

    pub fn create_circuit(&mut self, name: &str, num_qubits: usize, num_classical: usize) -> usize {
        let circuit = QuantumCircuit::new(name, num_qubits, num_classical);
        self.circuits.push(circuit);
        self.circuits.len() - 1
    }

    pub fn add_gate_to_circuit(&mut self, idx: usize, gate: QuantumGate) -> bool {
        if idx < self.circuits.len() {
            self.circuits[idx].add_gate(gate);
            true
        } else {
            false
        }
    }

    pub fn get_circuit(&self, idx: usize) -> Option<&QuantumCircuit> {
        self.circuits.get(idx)
    }

    pub fn list_circuits(&self) -> &[QuantumCircuit] {
        &self.circuits
    }

    pub fn export_circuit_qasm3(&self, idx: usize) -> Option<String> {
        self.circuits.get(idx).map(|c| c.to_qasm3())
    }

    pub fn export_circuit_qiskit(&self, idx: usize) -> Option<String> {
        self.circuits.get(idx).map(|c| c.to_qiskit())
    }

    pub fn export_circuit_cirq(&self, idx: usize) -> Option<String> {
        self.circuits.get(idx).map(|c| c.to_cirq())
    }

    pub fn set_hardware_pref(&mut self, key: &str, value: &str) {
        self.hardware_prefs.insert(key.to_string(), value.to_string());
    }

    pub fn get_hardware_pref(&self, key: &str) -> Option<&String> {
        self.hardware_prefs.get(key)
    }

    /// Estimate physical qubits needed for a project (rough heuristic).
    pub fn estimate_physical_qubits(&self, project_id: &str) -> Option<usize> {
        let p = self.projects.iter().find(|p| p.id == project_id)?;
        let overhead = match &p.error_correction {
            Some(ErrorCorrectionCode::SurfaceCode) => 1000,
            Some(ErrorCorrectionCode::SteaneCode) => 7,
            Some(ErrorCorrectionCode::ShorCode) => 9,
            Some(ErrorCorrectionCode::ColorCode) => 500,
            Some(ErrorCorrectionCode::ToricCode) => 800,
            Some(ErrorCorrectionCode::BaconShorCode) => 15,
            Some(ErrorCorrectionCode::CatCode) => 3,
            Some(ErrorCorrectionCode::GKPCode) => 2,
            Some(ErrorCorrectionCode::FlaggedFTEC) => 12,
            None => 1,
        };
        Some(p.num_qubits * overhead)
    }

    /// Generate a compatibility matrix: which languages work on which OS.
    pub fn compatibility_matrix() -> Vec<(QuantumLanguage, Vec<QuantumOS>)> {
        vec![
            (QuantumLanguage::Qiskit, vec![QuantumOS::QiskitRuntime, QuantumOS::AzureQuantum, QuantumOS::AmazonBraket]),
            (QuantumLanguage::Cirq, vec![QuantumOS::CirqEngine, QuantumOS::AzureQuantum, QuantumOS::AmazonBraket]),
            (QuantumLanguage::PennyLane, vec![QuantumOS::QiskitRuntime, QuantumOS::AmazonBraket, QuantumOS::CirqEngine]),
            (QuantumLanguage::QSharp, vec![QuantumOS::AzureQuantum]),
            (QuantumLanguage::BraketSDK, vec![QuantumOS::AmazonBraket]),
            (QuantumLanguage::TKet, vec![QuantumOS::QiskitRuntime, QuantumOS::AzureQuantum, QuantumOS::AmazonBraket, QuantumOS::Qcs]),
            (QuantumLanguage::CudaQuantum, vec![QuantumOS::QiskitRuntime, QuantumOS::CirqEngine]),
            (QuantumLanguage::Bloqade, vec![QuantumOS::AmazonBraket]),
            (QuantumLanguage::OpenQASM3, vec![QuantumOS::QiskitRuntime, QuantumOS::AzureQuantum, QuantumOS::AmazonBraket]),
        ]
    }
}

// ── Complex Number ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };
    pub const I: Self = Self { re: 0.0, im: 1.0 };

    pub fn new(re: f64, im: f64) -> Self { Self { re, im } }
    pub fn from_polar(r: f64, theta: f64) -> Self { Self { re: r * theta.cos(), im: r * theta.sin() } }
    pub fn conj(self) -> Self { Self { re: self.re, im: -self.im } }
    pub fn norm_sq(self) -> f64 { self.re * self.re + self.im * self.im }
    pub fn norm(self) -> f64 { self.norm_sq().sqrt() }
}

impl std::ops::Add for Complex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Self { re: self.re + rhs.re, im: self.im + rhs.im } }
}

impl std::ops::Sub for Complex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Self { re: self.re - rhs.re, im: self.im - rhs.im } }
}

impl std::ops::Mul for Complex {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl std::ops::Mul<f64> for Complex {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self { Self { re: self.re * rhs, im: self.im * rhs } }
}

impl std::ops::Neg for Complex {
    type Output = Self;
    fn neg(self) -> Self { Self { re: -self.re, im: -self.im } }
}

// ── Statevector Simulator ─────────────────────────────────────────────────────

pub type GateMatrix = [[Complex; 2]; 2];

/// Gate matrices for the standard quantum gate set.
pub fn gate_h() -> GateMatrix {
    let s = 1.0 / std::f64::consts::SQRT_2;
    let c = Complex::new(s, 0.0);
    [[c, c], [c, Complex::new(-s, 0.0)]]
}
pub fn gate_x() -> GateMatrix { [[Complex::ZERO, Complex::ONE], [Complex::ONE, Complex::ZERO]] }
pub fn gate_y() -> GateMatrix { [[Complex::ZERO, -Complex::I], [Complex::I, Complex::ZERO]] }
pub fn gate_z() -> GateMatrix { [[Complex::ONE, Complex::ZERO], [Complex::ZERO, Complex::new(-1.0, 0.0)]] }
pub fn gate_s() -> GateMatrix { [[Complex::ONE, Complex::ZERO], [Complex::ZERO, Complex::I]] }
pub fn gate_t() -> GateMatrix {
    [[Complex::ONE, Complex::ZERO], [Complex::ZERO, Complex::from_polar(1.0, std::f64::consts::FRAC_PI_4)]]
}
pub fn gate_rx(theta: f64) -> GateMatrix {
    let c = Complex::new((theta / 2.0).cos(), 0.0);
    let s = Complex::new(0.0, -(theta / 2.0).sin());
    [[c, s], [s, c]]
}
pub fn gate_ry(theta: f64) -> GateMatrix {
    let c = Complex::new((theta / 2.0).cos(), 0.0);
    let s = Complex::new((theta / 2.0).sin(), 0.0);
    [[c, -s], [s, c]]
}
pub fn gate_rz(theta: f64) -> GateMatrix {
    [[Complex::from_polar(1.0, -theta / 2.0), Complex::ZERO],
     [Complex::ZERO, Complex::from_polar(1.0, theta / 2.0)]]
}

/// Simulation result returned to the frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimulationResult {
    pub amplitudes: Vec<(String, f64, f64)>,  // (basis_label, re, im)
    pub probabilities: Vec<(String, f64)>,
    pub samples: std::collections::HashMap<String, usize>,
    pub num_qubits: usize,
}

pub struct StatevectorSimulator {
    pub num_qubits: usize,
    pub state: Vec<Complex>,
}

impl StatevectorSimulator {
    pub fn new(num_qubits: usize) -> Result<Self, String> {
        if num_qubits == 0 || num_qubits > 16 {
            return Err(format!("Qubit count must be 1-16, got {}", num_qubits));
        }
        let dim = 1 << num_qubits;
        let mut state = vec![Complex::ZERO; dim];
        state[0] = Complex::ONE; // |0...0⟩
        Ok(Self { num_qubits, state })
    }

    /// Apply a 2x2 unitary to a single qubit.
    pub fn apply_single_qubit(&mut self, qubit: usize, m: &GateMatrix) {
        let dim = self.state.len();
        let bit = 1 << qubit;
        let mut i = 0;
        while i < dim {
            // Process pairs of indices differing in bit `qubit`
            if i & bit == 0 {
                let j = i | bit;
                let a = self.state[i];
                let b = self.state[j];
                self.state[i] = m[0][0] * a + m[0][1] * b;
                self.state[j] = m[1][0] * a + m[1][1] * b;
            }
            i += 1;
        }
    }

    /// Apply a controlled-U gate (control, target, 2x2 matrix on target).
    pub fn apply_controlled(&mut self, control: usize, target: usize, m: &GateMatrix) {
        let dim = self.state.len();
        let ctrl_bit = 1 << control;
        let tgt_bit = 1 << target;
        for i in 0..dim {
            if i & ctrl_bit != 0 && i & tgt_bit == 0 {
                let j = i | tgt_bit;
                let a = self.state[i];
                let b = self.state[j];
                self.state[i] = m[0][0] * a + m[0][1] * b;
                self.state[j] = m[1][0] * a + m[1][1] * b;
            }
        }
    }

    /// Apply a doubly-controlled-U gate (Toffoli-like).
    pub fn apply_double_controlled(&mut self, c1: usize, c2: usize, target: usize, m: &GateMatrix) {
        let dim = self.state.len();
        let c1_bit = 1 << c1;
        let c2_bit = 1 << c2;
        let tgt_bit = 1 << target;
        for i in 0..dim {
            if i & c1_bit != 0 && i & c2_bit != 0 && i & tgt_bit == 0 {
                let j = i | tgt_bit;
                let a = self.state[i];
                let b = self.state[j];
                self.state[i] = m[0][0] * a + m[0][1] * b;
                self.state[j] = m[1][0] * a + m[1][1] * b;
            }
        }
    }

    /// Apply SWAP gate.
    pub fn apply_swap(&mut self, a: usize, b: usize) {
        let dim = self.state.len();
        let a_bit = 1 << a;
        let b_bit = 1 << b;
        for i in 0..dim {
            // Only swap when bits differ: a=1,b=0
            if i & a_bit != 0 && i & b_bit == 0 {
                let j = (i & !a_bit) | b_bit;
                self.state.swap(i, j);
            }
        }
    }

    /// Apply a QuantumGate to the statevector.
    pub fn apply_gate(&mut self, gate: &QuantumGate) {
        match gate {
            QuantumGate::H(q) => self.apply_single_qubit(*q, &gate_h()),
            QuantumGate::X(q) => self.apply_single_qubit(*q, &gate_x()),
            QuantumGate::Y(q) => self.apply_single_qubit(*q, &gate_y()),
            QuantumGate::Z(q) => self.apply_single_qubit(*q, &gate_z()),
            QuantumGate::S(q) => self.apply_single_qubit(*q, &gate_s()),
            QuantumGate::T(q) => self.apply_single_qubit(*q, &gate_t()),
            QuantumGate::Rx(q, theta) => self.apply_single_qubit(*q, &gate_rx(*theta)),
            QuantumGate::Ry(q, theta) => self.apply_single_qubit(*q, &gate_ry(*theta)),
            QuantumGate::Rz(q, theta) => self.apply_single_qubit(*q, &gate_rz(*theta)),
            QuantumGate::Cnot(c, t) => self.apply_controlled(*c, *t, &gate_x()),
            QuantumGate::CZ(c, t) => self.apply_controlled(*c, *t, &gate_z()),
            QuantumGate::Swap(a, b) => self.apply_swap(*a, *b),
            QuantumGate::Toffoli(c1, c2, t) => self.apply_double_controlled(*c1, *c2, *t, &gate_x()),
            QuantumGate::Measure(_, _) => {} // measurement handled separately
        }
    }

    /// Get probability of each computational basis state.
    pub fn probabilities(&self) -> Vec<(String, f64)> {
        self.state.iter().enumerate()
            .map(|(i, amp)| {
                let label = format!("{:0>width$b}", i, width = self.num_qubits);
                (label, amp.norm_sq())
            })
            .filter(|(_, p)| *p > 1e-12)
            .collect()
    }

    /// Get full amplitudes as (label, re, im) triples.
    pub fn amplitudes(&self) -> Vec<(String, f64, f64)> {
        self.state.iter().enumerate()
            .map(|(i, amp)| {
                let label = format!("{:0>width$b}", i, width = self.num_qubits);
                (label, amp.re, amp.im)
            })
            .filter(|(_, re, im)| re.abs() > 1e-12 || im.abs() > 1e-12)
            .collect()
    }

    /// Sample measurement outcomes.
    pub fn sample(&self, shots: usize) -> std::collections::HashMap<String, usize> {
        use std::collections::HashMap;
        let probs = self.probabilities();
        let mut counts: HashMap<String, usize> = HashMap::new();
        // Simple sampling using cumulative distribution
        let cumulative: Vec<f64> = {
            let mut cum = Vec::with_capacity(probs.len());
            let mut acc = 0.0;
            for (_, p) in &probs {
                acc += p;
                cum.push(acc);
            }
            cum
        };
        // Pseudo-random sampling using hash-based RNG
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        for shot in 0..shots {
            let mut h = DefaultHasher::new();
            shot.hash(&mut h);
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .hash(&mut h);
            let r = (h.finish() % 1_000_000) as f64 / 1_000_000.0;
            let idx = cumulative.iter().position(|c| r < *c).unwrap_or(probs.len() - 1);
            *counts.entry(probs[idx].0.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Simulate an entire circuit and return results.
    pub fn simulate_circuit(circuit: &QuantumCircuit, shots: usize) -> Result<SimulationResult, String> {
        let mut sim = Self::new(circuit.num_qubits)?;
        for gate in &circuit.gates {
            sim.apply_gate(gate);
        }
        Ok(SimulationResult {
            amplitudes: sim.amplitudes(),
            probabilities: sim.probabilities(),
            samples: sim.sample(shots),
            num_qubits: circuit.num_qubits,
        })
    }
}

// ── Circuit Optimizer ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OptimizationResult {
    pub original_gate_count: usize,
    pub optimized_gate_count: usize,
    pub original_depth: usize,
    pub optimized_depth: usize,
    pub rules_applied: Vec<String>,
    pub savings_percent: f64,
}

pub struct CircuitOptimizer;

impl CircuitOptimizer {
    /// Optimize a circuit by applying gate cancellation and merging rules.
    pub fn optimize(circuit: &QuantumCircuit) -> (QuantumCircuit, OptimizationResult) {
        let original_count = circuit.gate_count();
        let original_depth = circuit.depth();
        let mut gates = circuit.gates.clone();
        let mut rules: Vec<String> = Vec::new();

        // Iterate passes until stable
        for _ in 0..10 {
            let before = gates.len();
            gates = Self::cancel_identities(&gates, &mut rules);
            gates = Self::merge_rotations(&gates, &mut rules);
            if gates.len() == before { break; }
        }

        let optimized = QuantumCircuit {
            name: format!("{}_optimized", circuit.name),
            num_qubits: circuit.num_qubits,
            num_classical: circuit.num_classical,
            gates,
        };
        let opt_count = optimized.gate_count();
        let savings = if original_count > 0 {
            ((original_count - opt_count) as f64 / original_count as f64) * 100.0
        } else { 0.0 };

        let result = OptimizationResult {
            original_gate_count: original_count,
            optimized_gate_count: opt_count,
            original_depth,
            optimized_depth: optimized.depth(),
            rules_applied: rules,
            savings_percent: savings,
        };
        (optimized, result)
    }

    fn cancel_identities(gates: &[QuantumGate], rules: &mut Vec<String>) -> Vec<QuantumGate> {
        let mut result: Vec<QuantumGate> = Vec::new();
        let mut i = 0;
        while i < gates.len() {
            if i + 1 < gates.len() {
                let (a, b) = (&gates[i], &gates[i + 1]);
                // Self-inverse gates: HH, XX, YY, ZZ, CNOT·CNOT
                let cancelled = match (a, b) {
                    (QuantumGate::H(q1), QuantumGate::H(q2)) if q1 == q2 => { rules.push(format!("HH→I on q{}", q1)); true }
                    (QuantumGate::X(q1), QuantumGate::X(q2)) if q1 == q2 => { rules.push(format!("XX→I on q{}", q1)); true }
                    (QuantumGate::Y(q1), QuantumGate::Y(q2)) if q1 == q2 => { rules.push(format!("YY→I on q{}", q1)); true }
                    (QuantumGate::Z(q1), QuantumGate::Z(q2)) if q1 == q2 => { rules.push(format!("ZZ→I on q{}", q1)); true }
                    (QuantumGate::Cnot(c1, t1), QuantumGate::Cnot(c2, t2)) if c1 == c2 && t1 == t2 => { rules.push(format!("CNOT·CNOT→I on q{},q{}", c1, t1)); true }
                    _ => false,
                };
                if cancelled { i += 2; continue; }
                // SS→Z, TT→S
                match (a, b) {
                    (QuantumGate::S(q1), QuantumGate::S(q2)) if q1 == q2 => {
                        rules.push(format!("SS→Z on q{}", q1));
                        result.push(QuantumGate::Z(*q1));
                        i += 2; continue;
                    }
                    (QuantumGate::T(q1), QuantumGate::T(q2)) if q1 == q2 => {
                        rules.push(format!("TT→S on q{}", q1));
                        result.push(QuantumGate::S(*q1));
                        i += 2; continue;
                    }
                    _ => {}
                }
            }
            result.push(gates[i].clone());
            i += 1;
        }
        result
    }

    fn merge_rotations(gates: &[QuantumGate], rules: &mut Vec<String>) -> Vec<QuantumGate> {
        let mut result: Vec<QuantumGate> = Vec::new();
        let mut i = 0;
        while i < gates.len() {
            if i + 1 < gates.len() {
                let merged = match (&gates[i], &gates[i + 1]) {
                    (QuantumGate::Rx(q1, a), QuantumGate::Rx(q2, b)) if q1 == q2 => {
                        let sum = a + b;
                        rules.push(format!("Rx merge on q{}: {:.3}+{:.3}={:.3}", q1, a, b, sum));
                        if sum.abs() < 1e-10 || (sum - 2.0 * std::f64::consts::PI).abs() < 1e-10 { None }
                        else { Some(QuantumGate::Rx(*q1, sum)) }
                    }
                    (QuantumGate::Ry(q1, a), QuantumGate::Ry(q2, b)) if q1 == q2 => {
                        let sum = a + b;
                        rules.push(format!("Ry merge on q{}: {:.3}+{:.3}={:.3}", q1, a, b, sum));
                        if sum.abs() < 1e-10 || (sum - 2.0 * std::f64::consts::PI).abs() < 1e-10 { None }
                        else { Some(QuantumGate::Ry(*q1, sum)) }
                    }
                    (QuantumGate::Rz(q1, a), QuantumGate::Rz(q2, b)) if q1 == q2 => {
                        let sum = a + b;
                        rules.push(format!("Rz merge on q{}: {:.3}+{:.3}={:.3}", q1, a, b, sum));
                        if sum.abs() < 1e-10 || (sum - 2.0 * std::f64::consts::PI).abs() < 1e-10 { None }
                        else { Some(QuantumGate::Rz(*q1, sum)) }
                    }
                    _ => { result.push(gates[i].clone()); i += 1; continue; }
                };
                if let Some(g) = merged { result.push(g); }
                i += 2;
                continue;
            }
            result.push(gates[i].clone());
            i += 1;
        }
        result
    }
}

// ── Algorithm Circuit Templates ───────────────────────────────────────────────

pub struct AlgorithmTemplates;

impl AlgorithmTemplates {
    pub fn bell_state() -> QuantumCircuit {
        let mut c = QuantumCircuit::new("Bell State", 2, 2);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        c.add_gate(QuantumGate::Measure(0, 0));
        c.add_gate(QuantumGate::Measure(1, 1));
        c
    }

    pub fn ghz_state(n: usize) -> QuantumCircuit {
        let n = n.clamp(2, 16);
        let mut c = QuantumCircuit::new(&format!("GHZ-{}", n), n, n);
        c.add_gate(QuantumGate::H(0));
        for i in 0..n - 1 {
            c.add_gate(QuantumGate::Cnot(0, i + 1));
        }
        for i in 0..n { c.add_gate(QuantumGate::Measure(i, i)); }
        c
    }

    pub fn qft(n: usize) -> QuantumCircuit {
        let n = n.clamp(2, 16);
        let mut c = QuantumCircuit::new(&format!("QFT-{}", n), n, 0);
        for i in 0..n {
            c.add_gate(QuantumGate::H(i));
            for j in (i + 1)..n {
                let angle = std::f64::consts::PI / (1 << (j - i)) as f64;
                // Controlled-Rz implemented as: CZ-like with rotation
                // For simplicity, we use Rz as an approximation in the template
                c.add_gate(QuantumGate::Rz(j, angle));
            }
        }
        // SWAP to reverse qubit order
        for i in 0..n / 2 {
            c.add_gate(QuantumGate::Swap(i, n - 1 - i));
        }
        c
    }

    pub fn grover_2qubit() -> QuantumCircuit {
        let mut c = QuantumCircuit::new("Grover 2-qubit", 2, 2);
        // Superposition
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::H(1));
        // Oracle (mark |11⟩)
        c.add_gate(QuantumGate::CZ(0, 1));
        // Diffusion
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::H(1));
        c.add_gate(QuantumGate::Z(0));
        c.add_gate(QuantumGate::Z(1));
        c.add_gate(QuantumGate::CZ(0, 1));
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::H(1));
        // Measure
        c.add_gate(QuantumGate::Measure(0, 0));
        c.add_gate(QuantumGate::Measure(1, 1));
        c
    }

    pub fn deutsch_jozsa(n: usize) -> QuantumCircuit {
        let n = n.clamp(1, 15);
        let mut c = QuantumCircuit::new(&format!("Deutsch-Jozsa {}", n), n + 1, n);
        // Prepare ancilla in |1⟩
        c.add_gate(QuantumGate::X(n));
        // Hadamard all qubits
        for i in 0..=n { c.add_gate(QuantumGate::H(i)); }
        // Oracle: balanced function f(x) = x_0 (CNOT from q0 to ancilla)
        c.add_gate(QuantumGate::Cnot(0, n));
        // Hadamard on input qubits
        for i in 0..n { c.add_gate(QuantumGate::H(i)); }
        // Measure input qubits
        for i in 0..n { c.add_gate(QuantumGate::Measure(i, i)); }
        c
    }

    pub fn bernstein_vazirani(secret: &str) -> QuantumCircuit {
        let bits: Vec<bool> = secret.chars().rev().map(|ch| ch == '1').collect();
        let n = bits.len().clamp(1, 15);
        let mut c = QuantumCircuit::new(&format!("BV s={}", secret), n + 1, n);
        // Prepare ancilla
        c.add_gate(QuantumGate::X(n));
        // Hadamard all
        for i in 0..=n { c.add_gate(QuantumGate::H(i)); }
        // Oracle: CNOT from q_i to ancilla where secret bit is 1
        for (i, &bit) in bits.iter().enumerate() {
            if bit { c.add_gate(QuantumGate::Cnot(i, n)); }
        }
        // Hadamard on input qubits
        for i in 0..n { c.add_gate(QuantumGate::H(i)); }
        // Measure
        for i in 0..n { c.add_gate(QuantumGate::Measure(i, i)); }
        c
    }

    pub fn vqe_ansatz(n: usize, layers: usize) -> QuantumCircuit {
        let n = n.clamp(2, 16);
        let layers = layers.clamp(1, 10);
        let mut c = QuantumCircuit::new(&format!("VQE {}-qubit {}-layer", n, layers), n, n);
        for layer in 0..layers {
            // Ry rotation layer
            for q in 0..n {
                let angle = std::f64::consts::PI * (layer as f64 + 1.0) / (layers as f64 + 1.0);
                c.add_gate(QuantumGate::Ry(q, angle));
            }
            // Entangling layer
            for q in 0..n - 1 {
                c.add_gate(QuantumGate::Cnot(q, q + 1));
            }
        }
        for i in 0..n { c.add_gate(QuantumGate::Measure(i, i)); }
        c
    }

    pub fn qaoa_layer(n: usize, gamma: f64, beta: f64) -> QuantumCircuit {
        let n = n.clamp(2, 16);
        let mut c = QuantumCircuit::new(&format!("QAOA {}-qubit", n), n, n);
        // Initial superposition
        for q in 0..n { c.add_gate(QuantumGate::H(q)); }
        // Cost layer: ZZ interactions on nearest-neighbor pairs
        for q in 0..n - 1 {
            c.add_gate(QuantumGate::Cnot(q, q + 1));
            c.add_gate(QuantumGate::Rz(q + 1, 2.0 * gamma));
            c.add_gate(QuantumGate::Cnot(q, q + 1));
        }
        // Mixer layer: Rx on all qubits
        for q in 0..n { c.add_gate(QuantumGate::Rx(q, 2.0 * beta)); }
        for i in 0..n { c.add_gate(QuantumGate::Measure(i, i)); }
        c
    }

    /// Generic entry point: get a template by name with optional params.
    pub fn get_template(name: &str, params: &std::collections::HashMap<String, String>) -> Result<QuantumCircuit, String> {
        let n = params.get("qubits").and_then(|v| v.parse().ok()).unwrap_or(3);
        match name.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "bell" | "bellstate" => Ok(Self::bell_state()),
            "ghz" | "ghzstate" => Ok(Self::ghz_state(n)),
            "qft" | "quantumfouriertransform" => Ok(Self::qft(n)),
            "grover" | "grover2qubit" => Ok(Self::grover_2qubit()),
            "deutschjozsa" => Ok(Self::deutsch_jozsa(n)),
            "bernsteinvazirani" | "bv" => {
                let secret = params.get("secret").map(|s| s.as_str()).unwrap_or("101");
                Ok(Self::bernstein_vazirani(secret))
            }
            "vqe" | "vqeansatz" => {
                let layers = params.get("layers").and_then(|v| v.parse().ok()).unwrap_or(2);
                Ok(Self::vqe_ansatz(n, layers))
            }
            "qaoa" => {
                let gamma = params.get("gamma").and_then(|v| v.parse().ok()).unwrap_or(0.5);
                let beta = params.get("beta").and_then(|v| v.parse().ok()).unwrap_or(0.5);
                Ok(Self::qaoa_layer(n, gamma, beta))
            }
            _ => Err(format!("Unknown algorithm template: {}", name)),
        }
    }

    /// List available templates with descriptions.
    pub fn list() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Bell State", "2-qubit entangled pair: H + CNOT"),
            ("GHZ", "N-qubit GHZ state: maximally entangled"),
            ("QFT", "Quantum Fourier Transform"),
            ("Grover 2-qubit", "Grover's search on 2 qubits"),
            ("Deutsch-Jozsa", "Determines if function is constant or balanced"),
            ("Bernstein-Vazirani", "Finds hidden bit string in one query"),
            ("VQE Ansatz", "Variational Quantum Eigensolver ansatz circuit"),
            ("QAOA", "Quantum Approximate Optimization Algorithm"),
        ]
    }
}

// ── Cost Estimator ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CostEstimate {
    pub provider: String,
    pub estimated_cost_usd: f64,
    pub breakdown: Vec<(String, f64)>,
    pub notes: Vec<String>,
}

pub struct CostEstimator;

impl CostEstimator {
    /// IBM Quantum: $1.60/second runtime
    pub fn estimate_ibm(circuit: &QuantumCircuit, shots: usize) -> CostEstimate {
        // Gate times: single ~35ns, CNOT ~300ns, readout ~1μs
        let single_ns: f64 = circuit.gates.iter().filter(|g| matches!(g, QuantumGate::H(_) | QuantumGate::X(_) | QuantumGate::Y(_) | QuantumGate::Z(_) | QuantumGate::S(_) | QuantumGate::T(_) | QuantumGate::Rx(..) | QuantumGate::Ry(..) | QuantumGate::Rz(..))).count() as f64 * 35.0;
        let two_q_ns = circuit.two_qubit_gate_count() as f64 * 300.0;
        let measure_ns = circuit.gates.iter().filter(|g| matches!(g, QuantumGate::Measure(..))).count() as f64 * 1000.0;
        let total_ns_per_shot = single_ns + two_q_ns + measure_ns;
        let total_seconds = total_ns_per_shot * shots as f64 / 1e9;
        let cost = total_seconds * 1.60;
        CostEstimate {
            provider: "IBM Quantum".to_string(),
            estimated_cost_usd: cost,
            breakdown: vec![
                ("Runtime (seconds)".to_string(), total_seconds),
                ("Rate ($/second)".to_string(), 1.60),
            ],
            notes: vec!["Based on IBM Quantum pay-as-you-go pricing ($1.60/sec)".to_string()],
        }
    }

    /// Amazon Braket: $0.30/task + per-shot pricing
    pub fn estimate_braket(_circuit: &QuantumCircuit, shots: usize) -> CostEstimate {
        let task_cost = 0.30;
        let shot_cost = 0.00145 * shots as f64; // IonQ Aria pricing
        let total = task_cost + shot_cost;
        CostEstimate {
            provider: "Amazon Braket (IonQ)".to_string(),
            estimated_cost_usd: total,
            breakdown: vec![
                ("Task fee".to_string(), task_cost),
                (format!("{} shots × $0.00145", shots), shot_cost),
            ],
            notes: vec!["IonQ Aria via Braket. Rigetti: $0.00035/shot, Simulators: $0.075/min".to_string()],
        }
    }

    /// IonQ: per-gate pricing
    pub fn estimate_ionq(circuit: &QuantumCircuit, shots: usize) -> CostEstimate {
        let single_gates = circuit.gate_count() - circuit.two_qubit_gate_count();
        let single_cost = single_gates as f64 * 0.00003 * shots as f64;
        let two_q_cost = circuit.two_qubit_gate_count() as f64 * 0.0003 * shots as f64;
        let shot_cost = shots as f64 * 0.01;
        let total = single_cost + two_q_cost + shot_cost;
        CostEstimate {
            provider: "IonQ (direct)".to_string(),
            estimated_cost_usd: total,
            breakdown: vec![
                (format!("{} 1Q gates × {} shots × $0.00003", single_gates, shots), single_cost),
                (format!("{} 2Q gates × {} shots × $0.0003", circuit.two_qubit_gate_count(), shots), two_q_cost),
                (format!("{} shots × $0.01", shots), shot_cost),
            ],
            notes: vec!["IonQ Aria direct access pricing".to_string()],
        }
    }

    /// Estimate across all providers.
    pub fn estimate_all(circuit: &QuantumCircuit, shots: usize) -> Vec<CostEstimate> {
        vec![
            Self::estimate_ibm(circuit, shots),
            Self::estimate_braket(circuit, shots),
            Self::estimate_ionq(circuit, shots),
        ]
    }
}

// ── Project Scaffolder ────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScaffoldFile {
    pub path: String,
    pub content: String,
}

pub struct ProjectScaffolder;

impl ProjectScaffolder {
    pub fn scaffold(language: &str, name: &str, num_qubits: usize) -> Result<Vec<ScaffoldFile>, String> {
        match language.to_lowercase().as_str() {
            "qiskit" => Ok(Self::scaffold_qiskit(name, num_qubits)),
            "cirq" => Ok(Self::scaffold_cirq(name, num_qubits)),
            "pennylane" => Ok(Self::scaffold_pennylane(name, num_qubits)),
            "q#" | "qsharp" => Ok(Self::scaffold_qsharp(name, num_qubits)),
            _ => Err(format!("Unsupported language for scaffolding: {}", language)),
        }
    }

    fn scaffold_qiskit(name: &str, qubits: usize) -> Vec<ScaffoldFile> {
        vec![
            ScaffoldFile { path: "requirements.txt".to_string(), content: "qiskit>=1.0\nqiskit-aer>=0.13\nqiskit-ibm-runtime>=0.20\npytest>=7.0\n".to_string() },
            ScaffoldFile { path: "main.py".to_string(), content: format!(
                "\"\"\"Quantum circuit: {name}\"\"\"\nfrom qiskit import QuantumCircuit\nfrom qiskit_aer import AerSimulator\n\ndef create_circuit() -> QuantumCircuit:\n    qc = QuantumCircuit({qubits}, {qubits})\n    # Add gates here\n    qc.h(0)\n    qc.measure_all()\n    return qc\n\ndef main():\n    qc = create_circuit()\n    sim = AerSimulator()\n    result = sim.run(qc, shots=1024).result()\n    counts = result.get_counts()\n    print(f\"Results: {{counts}}\")\n\nif __name__ == \"__main__\":\n    main()\n"
            )},
            ScaffoldFile { path: "test_circuit.py".to_string(), content: format!(
                "from main import create_circuit\nfrom qiskit_aer import AerSimulator\n\ndef test_circuit_runs():\n    qc = create_circuit()\n    assert qc.num_qubits == {qubits}\n    sim = AerSimulator()\n    result = sim.run(qc, shots=100).result()\n    assert result.success\n\ndef test_circuit_depth():\n    qc = create_circuit()\n    assert qc.depth() > 0\n"
            )},
            ScaffoldFile { path: ".github/workflows/test.yml".to_string(), content: "name: Test\non: [push, pull_request]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: actions/setup-python@v5\n        with:\n          python-version: '3.11'\n      - run: pip install -r requirements.txt\n      - run: pytest -v\n".to_string() },
            ScaffoldFile { path: "README.md".to_string(), content: format!("# {name}\n\nQuantum circuit project using Qiskit ({qubits} qubits).\n\n## Setup\n```bash\npip install -r requirements.txt\npython main.py\n```\n\n## Test\n```bash\npytest -v\n```\n") },
        ]
    }

    fn scaffold_cirq(name: &str, qubits: usize) -> Vec<ScaffoldFile> {
        vec![
            ScaffoldFile { path: "requirements.txt".to_string(), content: "cirq>=1.3\npytest>=7.0\n".to_string() },
            ScaffoldFile { path: "main.py".to_string(), content: format!(
                "\"\"\"Quantum circuit: {name}\"\"\"\nimport cirq\n\ndef create_circuit() -> cirq.Circuit:\n    qubits = cirq.LineQubit.range({qubits})\n    circuit = cirq.Circuit()\n    circuit.append(cirq.H(qubits[0]))\n    circuit.append(cirq.measure(*qubits, key='result'))\n    return circuit\n\ndef main():\n    circuit = create_circuit()\n    sim = cirq.Simulator()\n    result = sim.run(circuit, repetitions=1024)\n    print(f\"Results: {{result.histogram(key='result')}}\")\n\nif __name__ == \"__main__\":\n    main()\n"
            )},
            ScaffoldFile { path: "test_circuit.py".to_string(), content: format!(
                "import cirq\nfrom main import create_circuit\n\ndef test_circuit_runs():\n    circuit = create_circuit()\n    sim = cirq.Simulator()\n    result = sim.run(circuit, repetitions=100)\n    assert len(result.measurements) > 0\n\ndef test_qubit_count():\n    circuit = create_circuit()\n    assert len(circuit.all_qubits()) <= {qubits}\n"
            )},
            ScaffoldFile { path: "README.md".to_string(), content: format!("# {name}\n\nQuantum circuit project using Cirq ({qubits} qubits).\n\n## Setup\n```bash\npip install -r requirements.txt\npython main.py\n```\n") },
        ]
    }

    fn scaffold_pennylane(name: &str, qubits: usize) -> Vec<ScaffoldFile> {
        vec![
            ScaffoldFile { path: "requirements.txt".to_string(), content: "pennylane>=0.35\npytest>=7.0\n".to_string() },
            ScaffoldFile { path: "main.py".to_string(), content: format!(
                "\"\"\"Quantum circuit: {name}\"\"\"\nimport pennylane as qml\nfrom pennylane import numpy as np\n\ndev = qml.device('default.qubit', wires={qubits})\n\n@qml.qnode(dev)\ndef circuit(params):\n    for i in range({qubits}):\n        qml.RY(params[i], wires=i)\n    for i in range({qubits} - 1):\n        qml.CNOT(wires=[i, i + 1])\n    return qml.probs(wires=range({qubits}))\n\ndef main():\n    params = np.random.uniform(0, np.pi, {qubits})\n    probs = circuit(params)\n    print(f\"Probabilities: {{probs}}\")\n\nif __name__ == \"__main__\":\n    main()\n"
            )},
            ScaffoldFile { path: "test_circuit.py".to_string(), content: format!(
                "import pennylane as qml\nfrom pennylane import numpy as np\nfrom main import circuit\n\ndef test_circuit_output():\n    params = np.zeros({qubits})\n    probs = circuit(params)\n    assert abs(sum(probs) - 1.0) < 1e-6\n"
            )},
            ScaffoldFile { path: "README.md".to_string(), content: format!("# {name}\n\nQuantum circuit project using PennyLane ({qubits} qubits).\n\n## Setup\n```bash\npip install -r requirements.txt\npython main.py\n```\n") },
        ]
    }

    fn scaffold_qsharp(name: &str, qubits: usize) -> Vec<ScaffoldFile> {
        vec![
            ScaffoldFile { path: format!("{}.csproj", name), content: "<Project Sdk=\"Microsoft.Quantum.Sdk/0.28.302812\">\n  <PropertyGroup>\n    <OutputType>Exe</OutputType>\n    <TargetFramework>net6.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n".to_string() },
            ScaffoldFile { path: "Program.qs".to_string(), content: format!(
                "namespace {name} {{\n    open Microsoft.Quantum.Canon;\n    open Microsoft.Quantum.Intrinsic;\n    open Microsoft.Quantum.Measurement;\n\n    @EntryPoint()\n    operation Main() : Result[] {{\n        use qubits = Qubit[{qubits}];\n        H(qubits[0]);\n        let results = MeasureEachZ(qubits);\n        Message($\"Results: {{results}}\");\n        return results;\n    }}\n}}\n"
            )},
            ScaffoldFile { path: "README.md".to_string(), content: format!("# {name}\n\nQuantum project using Q# ({qubits} qubits).\n\n## Run\n```bash\ndotnet run\n```\n") },
        ]
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> QuantumComputingManager {
        QuantumComputingManager::new()
    }

    // --- Language tests ---

    #[test]
    fn test_all_languages_listed() {
        let all = QuantumLanguage::all();
        assert_eq!(all.len(), 20);
    }

    #[test]
    fn test_language_properties() {
        let qiskit = QuantumLanguage::Qiskit;
        assert_eq!(qiskit.name(), "Qiskit");
        assert_eq!(qiskit.host_language(), "Python");
        assert_eq!(qiskit.vendor(), "IBM Quantum");
        assert!(qiskit.install_command().contains("pip install qiskit"));
    }

    #[test]
    fn test_qsharp_properties() {
        let qs = QuantumLanguage::QSharp;
        assert_eq!(qs.name(), "Q#");
        assert_eq!(qs.vendor(), "Microsoft");
    }

    #[test]
    fn test_hello_circuit_qiskit() {
        let code = QuantumLanguage::Qiskit.hello_circuit();
        assert!(code.contains("QuantumCircuit"));
        assert!(code.contains("qc.h(0)"));
        assert!(code.contains("qc.cx(0, 1)"));
    }

    #[test]
    fn test_hello_circuit_cirq() {
        let code = QuantumLanguage::Cirq.hello_circuit();
        assert!(code.contains("cirq.H"));
        assert!(code.contains("cirq.CNOT"));
    }

    #[test]
    fn test_hello_circuit_qsharp() {
        let code = QuantumLanguage::QSharp.hello_circuit();
        assert!(code.contains("H(q0)"));
        assert!(code.contains("CNOT(q0, q1)"));
    }

    #[test]
    fn test_hello_circuit_openqasm3() {
        let code = QuantumLanguage::OpenQASM3.hello_circuit();
        assert!(code.contains("OPENQASM 3.0"));
        assert!(code.contains("h q[0]"));
    }

    #[test]
    fn test_hello_circuit_pennylane() {
        let code = QuantumLanguage::PennyLane.hello_circuit();
        assert!(code.contains("pennylane"));
        assert!(code.contains("qml.Hadamard"));
    }

    #[test]
    fn test_hello_circuit_default() {
        let code = QuantumLanguage::Silq.hello_circuit();
        assert!(code.contains("official documentation"));
    }

    // --- OS tests ---

    #[test]
    fn test_all_os_listed() {
        let all = QuantumOS::all();
        assert_eq!(all.len(), 15);
    }

    #[test]
    fn test_os_properties() {
        let qr = QuantumOS::QiskitRuntime;
        assert_eq!(qr.name(), "Qiskit Runtime");
        assert_eq!(qr.layer(), "Cloud orchestration");
        assert_eq!(qr.vendor(), "IBM Quantum");
    }

    #[test]
    fn test_os_artiq() {
        let a = QuantumOS::Artiq;
        assert_eq!(a.layer(), "Hardware control plane");
        assert_eq!(a.vendor(), "M-Labs (NIST / Oxford)");
    }

    #[test]
    fn test_os_delft() {
        let d = QuantumOS::Delft;
        assert_eq!(d.layer(), "Quantum network OS");
    }

    // --- Hardware tests ---

    #[test]
    fn test_hardware_type_name() {
        assert_eq!(QuantumHardwareType::Superconducting.name(), "Superconducting transmon");
        assert_eq!(QuantumHardwareType::TrappedIon.name(), "Trapped-ion");
    }

    #[test]
    fn test_hardware_vendors() {
        let vendors = QuantumHardwareType::Superconducting.leading_vendors();
        assert!(vendors.contains(&"IBM"));
        assert!(vendors.contains(&"Google"));
    }

    #[test]
    fn test_trapped_ion_vendors() {
        let vendors = QuantumHardwareType::TrappedIon.leading_vendors();
        assert!(vendors.contains(&"IonQ"));
    }

    #[test]
    fn test_annealing_vendors() {
        let vendors = QuantumHardwareType::AnnealingProcessor.leading_vendors();
        assert!(vendors.contains(&"D-Wave"));
    }

    // --- Algorithm tests ---

    #[test]
    fn test_all_algorithms_listed() {
        let all = QuantumAlgorithm::all();
        assert_eq!(all.len(), 15);
    }

    #[test]
    fn test_algorithm_properties() {
        let grover = QuantumAlgorithm::Grover;
        assert_eq!(grover.name(), "Grover's search");
        assert_eq!(grover.category(), "Oracle / search");
        assert!(grover.qubit_scaling().contains("O(√N)"));
    }

    #[test]
    fn test_shor_algorithm() {
        let shor = QuantumAlgorithm::Shor;
        assert_eq!(shor.category(), "Number-theoretic");
        assert!(shor.qubit_scaling().contains("2n+3"));
    }

    #[test]
    fn test_vqe_algorithm() {
        let vqe = QuantumAlgorithm::Vqe;
        assert_eq!(vqe.category(), "Variational (NISQ-friendly)");
    }

    #[test]
    fn test_qml_algorithms() {
        assert_eq!(QuantumAlgorithm::Qsvm.category(), "Quantum machine learning");
        assert_eq!(QuantumAlgorithm::Qnn.category(), "Quantum machine learning");
    }

    // --- Error correction tests ---

    #[test]
    fn test_all_ecc_listed() {
        let all = ErrorCorrectionCode::all();
        assert_eq!(all.len(), 9);
    }

    #[test]
    fn test_surface_code() {
        let sc = ErrorCorrectionCode::SurfaceCode;
        assert_eq!(sc.name(), "Surface code");
        assert!(sc.physical_per_logical().contains("1000:1"));
    }

    #[test]
    fn test_steane_code() {
        let sc = ErrorCorrectionCode::SteaneCode;
        assert!(sc.physical_per_logical().contains("7:1"));
    }

    // --- Gate tests ---

    #[test]
    fn test_gate_qasm3_single() {
        assert_eq!(QuantumGate::H(0).qasm3(), "h q[0];");
        assert_eq!(QuantumGate::X(1).qasm3(), "x q[1];");
        assert_eq!(QuantumGate::Z(2).qasm3(), "z q[2];");
    }

    #[test]
    fn test_gate_qasm3_two_qubit() {
        assert_eq!(QuantumGate::Cnot(0, 1).qasm3(), "cx q[0], q[1];");
        assert_eq!(QuantumGate::CZ(1, 2).qasm3(), "cz q[1], q[2];");
        assert_eq!(QuantumGate::Swap(0, 3).qasm3(), "swap q[0], q[3];");
    }

    #[test]
    fn test_gate_qasm3_toffoli() {
        assert_eq!(QuantumGate::Toffoli(0, 1, 2).qasm3(), "ccx q[0], q[1], q[2];");
    }

    #[test]
    fn test_gate_qasm3_rotation() {
        let rx = QuantumGate::Rx(0, 1.5707963);
        assert!(rx.qasm3().starts_with("rx("));
    }

    #[test]
    fn test_gate_qasm3_measure() {
        assert_eq!(QuantumGate::Measure(0, 0).qasm3(), "c[0] = measure q[0];");
    }

    #[test]
    fn test_gate_max_qubit() {
        assert_eq!(QuantumGate::H(3).max_qubit(), 3);
        assert_eq!(QuantumGate::Cnot(1, 4).max_qubit(), 4);
        assert_eq!(QuantumGate::Toffoli(2, 5, 3).max_qubit(), 5);
    }

    // --- Circuit tests ---

    #[test]
    fn test_circuit_new() {
        let c = QuantumCircuit::new("test", 3, 3);
        assert_eq!(c.name, "test");
        assert_eq!(c.num_qubits, 3);
        assert_eq!(c.gate_count(), 0);
    }

    #[test]
    fn test_circuit_add_gate() {
        let mut c = QuantumCircuit::new("bell", 2, 2);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        assert_eq!(c.gate_count(), 2);
    }

    #[test]
    fn test_circuit_depth_simple() {
        let mut c = QuantumCircuit::new("test", 2, 0);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::H(1));
        // Parallel gates: depth = 1 each
        assert_eq!(c.depth(), 1);
    }

    #[test]
    fn test_circuit_depth_serial() {
        let mut c = QuantumCircuit::new("test", 2, 0);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        c.add_gate(QuantumGate::Measure(0, 0));
        // H on q0 (depth 1), CNOT on q0,q1 (depth 2), Measure on q0 (depth 3)
        assert_eq!(c.depth(), 3);
    }

    #[test]
    fn test_circuit_depth_empty() {
        let c = QuantumCircuit::new("empty", 2, 0);
        assert_eq!(c.depth(), 0);
    }

    #[test]
    fn test_circuit_two_qubit_count() {
        let mut c = QuantumCircuit::new("test", 3, 0);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        c.add_gate(QuantumGate::CZ(1, 2));
        c.add_gate(QuantumGate::X(0));
        assert_eq!(c.two_qubit_gate_count(), 2);
    }

    #[test]
    fn test_circuit_to_qasm3() {
        let mut c = QuantumCircuit::new("bell", 2, 2);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        c.add_gate(QuantumGate::Measure(0, 0));
        c.add_gate(QuantumGate::Measure(1, 1));
        let qasm = c.to_qasm3();
        assert!(qasm.contains("OPENQASM 3.0;"));
        assert!(qasm.contains("qubit[2] q;"));
        assert!(qasm.contains("bit[2] c;"));
        assert!(qasm.contains("h q[0];"));
        assert!(qasm.contains("cx q[0], q[1];"));
    }

    #[test]
    fn test_circuit_to_qiskit() {
        let mut c = QuantumCircuit::new("bell", 2, 2);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        let code = c.to_qiskit();
        assert!(code.contains("QuantumCircuit(2, 2)"));
        assert!(code.contains("qc.h(0)"));
        assert!(code.contains("qc.cx(0, 1)"));
    }

    #[test]
    fn test_circuit_to_cirq() {
        let mut c = QuantumCircuit::new("bell", 2, 0);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        let code = c.to_cirq();
        assert!(code.contains("import cirq"));
        assert!(code.contains("cirq.H(qubits[0])"));
        assert!(code.contains("cirq.CNOT(qubits[0], qubits[1])"));
    }

    // --- Manager tests ---

    #[test]
    fn test_manager_new_empty() {
        let mgr = make_manager();
        assert!(mgr.circuits.is_empty());
        assert!(mgr.projects.is_empty());
    }

    #[test]
    fn test_create_project() {
        let mut mgr = make_manager();
        let id = mgr.create_project(
            "Bell State Demo",
            QuantumLanguage::Qiskit,
            QuantumHardwareType::Superconducting,
            2,
            "Create a Bell state on IBM hardware",
        );
        assert!(id.starts_with("QP-"));
        assert_eq!(mgr.projects.len(), 1);
        assert_eq!(mgr.projects[0].name, "Bell State Demo");
    }

    #[test]
    fn test_get_project() {
        let mut mgr = make_manager();
        let id = mgr.create_project("test", QuantumLanguage::Cirq, QuantumHardwareType::Superconducting, 5, "test");
        let p = mgr.get_project(&id).expect("project should exist");
        assert_eq!(p.language, QuantumLanguage::Cirq);
        assert_eq!(p.num_qubits, 5);
    }

    #[test]
    fn test_get_project_not_found() {
        let mgr = make_manager();
        assert!(mgr.get_project("QP-9999").is_none());
    }

    #[test]
    fn test_set_project_os() {
        let mut mgr = make_manager();
        let id = mgr.create_project("test", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 2, "test");
        assert!(mgr.set_project_os(&id, QuantumOS::QiskitRuntime));
        let p = mgr.get_project(&id).unwrap();
        assert_eq!(p.target_os, Some(QuantumOS::QiskitRuntime));
    }

    #[test]
    fn test_set_project_os_not_found() {
        let mut mgr = make_manager();
        assert!(!mgr.set_project_os("QP-9999", QuantumOS::AzureQuantum));
    }

    #[test]
    fn test_set_project_algorithm() {
        let mut mgr = make_manager();
        let id = mgr.create_project("test", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 2, "test");
        assert!(mgr.set_project_algorithm(&id, QuantumAlgorithm::Grover));
        let p = mgr.get_project(&id).unwrap();
        assert_eq!(p.algorithm, Some(QuantumAlgorithm::Grover));
    }

    #[test]
    fn test_set_project_ecc() {
        let mut mgr = make_manager();
        let id = mgr.create_project("test", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 10, "test");
        assert!(mgr.set_project_ecc(&id, ErrorCorrectionCode::SurfaceCode));
        let p = mgr.get_project(&id).unwrap();
        assert_eq!(p.error_correction, Some(ErrorCorrectionCode::SurfaceCode));
    }

    #[test]
    fn test_delete_project() {
        let mut mgr = make_manager();
        let id = mgr.create_project("test", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 2, "test");
        assert!(mgr.delete_project(&id));
        assert!(mgr.projects.is_empty());
    }

    #[test]
    fn test_delete_project_not_found() {
        let mut mgr = make_manager();
        assert!(!mgr.delete_project("QP-9999"));
    }

    #[test]
    fn test_list_projects() {
        let mut mgr = make_manager();
        mgr.create_project("a", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 2, "a");
        mgr.create_project("b", QuantumLanguage::Cirq, QuantumHardwareType::TrappedIon, 5, "b");
        assert_eq!(mgr.list_projects().len(), 2);
    }

    #[test]
    fn test_create_circuit() {
        let mut mgr = make_manager();
        let idx = mgr.create_circuit("bell", 2, 2);
        assert_eq!(idx, 0);
        assert_eq!(mgr.circuits.len(), 1);
    }

    #[test]
    fn test_add_gate_to_circuit() {
        let mut mgr = make_manager();
        let idx = mgr.create_circuit("test", 2, 0);
        assert!(mgr.add_gate_to_circuit(idx, QuantumGate::H(0)));
        assert_eq!(mgr.get_circuit(idx).unwrap().gate_count(), 1);
    }

    #[test]
    fn test_add_gate_invalid_index() {
        let mut mgr = make_manager();
        assert!(!mgr.add_gate_to_circuit(99, QuantumGate::H(0)));
    }

    #[test]
    fn test_get_circuit() {
        let mut mgr = make_manager();
        let idx = mgr.create_circuit("test", 3, 3);
        let c = mgr.get_circuit(idx).unwrap();
        assert_eq!(c.num_qubits, 3);
    }

    #[test]
    fn test_get_circuit_not_found() {
        let mgr = make_manager();
        assert!(mgr.get_circuit(0).is_none());
    }

    #[test]
    fn test_export_qasm3() {
        let mut mgr = make_manager();
        let idx = mgr.create_circuit("bell", 2, 2);
        mgr.add_gate_to_circuit(idx, QuantumGate::H(0));
        mgr.add_gate_to_circuit(idx, QuantumGate::Cnot(0, 1));
        let qasm = mgr.export_circuit_qasm3(idx).unwrap();
        assert!(qasm.contains("OPENQASM 3.0"));
    }

    #[test]
    fn test_export_qiskit() {
        let mut mgr = make_manager();
        let idx = mgr.create_circuit("bell", 2, 2);
        mgr.add_gate_to_circuit(idx, QuantumGate::H(0));
        let code = mgr.export_circuit_qiskit(idx).unwrap();
        assert!(code.contains("qc.h(0)"));
    }

    #[test]
    fn test_export_cirq() {
        let mut mgr = make_manager();
        let idx = mgr.create_circuit("bell", 2, 0);
        mgr.add_gate_to_circuit(idx, QuantumGate::H(0));
        let code = mgr.export_circuit_cirq(idx).unwrap();
        assert!(code.contains("cirq.H"));
    }

    #[test]
    fn test_export_not_found() {
        let mgr = make_manager();
        assert!(mgr.export_circuit_qasm3(0).is_none());
    }

    #[test]
    fn test_hardware_pref() {
        let mut mgr = make_manager();
        mgr.set_hardware_pref("backend", "ibm_sherbrooke");
        assert_eq!(mgr.get_hardware_pref("backend").unwrap(), "ibm_sherbrooke");
    }

    #[test]
    fn test_hardware_pref_not_found() {
        let mgr = make_manager();
        assert!(mgr.get_hardware_pref("missing").is_none());
    }

    #[test]
    fn test_estimate_physical_qubits_no_ecc() {
        let mut mgr = make_manager();
        let id = mgr.create_project("test", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 10, "test");
        assert_eq!(mgr.estimate_physical_qubits(&id), Some(10));
    }

    #[test]
    fn test_estimate_physical_qubits_surface_code() {
        let mut mgr = make_manager();
        let id = mgr.create_project("test", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 10, "test");
        mgr.set_project_ecc(&id, ErrorCorrectionCode::SurfaceCode);
        assert_eq!(mgr.estimate_physical_qubits(&id), Some(10_000));
    }

    #[test]
    fn test_estimate_physical_qubits_steane() {
        let mut mgr = make_manager();
        let id = mgr.create_project("test", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 10, "test");
        mgr.set_project_ecc(&id, ErrorCorrectionCode::SteaneCode);
        assert_eq!(mgr.estimate_physical_qubits(&id), Some(70));
    }

    #[test]
    fn test_estimate_physical_qubits_not_found() {
        let mgr = make_manager();
        assert!(mgr.estimate_physical_qubits("QP-9999").is_none());
    }

    #[test]
    fn test_compatibility_matrix() {
        let matrix = QuantumComputingManager::compatibility_matrix();
        assert!(!matrix.is_empty());
        // Qiskit should be first
        assert_eq!(matrix[0].0, QuantumLanguage::Qiskit);
        assert!(matrix[0].1.contains(&QuantumOS::QiskitRuntime));
    }

    #[test]
    fn test_id_generation_unique() {
        let mut mgr = make_manager();
        let id1 = mgr.create_project("a", QuantumLanguage::Qiskit, QuantumHardwareType::Superconducting, 1, "a");
        let id2 = mgr.create_project("b", QuantumLanguage::Cirq, QuantumHardwareType::TrappedIon, 2, "b");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_multiple_circuits() {
        let mut mgr = make_manager();
        let i1 = mgr.create_circuit("c1", 2, 2);
        let i2 = mgr.create_circuit("c2", 4, 4);
        assert_eq!(i1, 0);
        assert_eq!(i2, 1);
        assert_eq!(mgr.list_circuits().len(), 2);
    }

    #[test]
    fn test_circuit_all_gate_types_qasm() {
        let mut c = QuantumCircuit::new("all_gates", 5, 2);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::X(1));
        c.add_gate(QuantumGate::Y(2));
        c.add_gate(QuantumGate::Z(3));
        c.add_gate(QuantumGate::S(0));
        c.add_gate(QuantumGate::T(1));
        c.add_gate(QuantumGate::Rx(0, 1.0));
        c.add_gate(QuantumGate::Ry(1, 2.0));
        c.add_gate(QuantumGate::Rz(2, 3.0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        c.add_gate(QuantumGate::CZ(2, 3));
        c.add_gate(QuantumGate::Swap(0, 4));
        c.add_gate(QuantumGate::Toffoli(0, 1, 2));
        c.add_gate(QuantumGate::Measure(0, 0));
        let qasm = c.to_qasm3();
        assert!(qasm.contains("h q[0];"));
        assert!(qasm.contains("x q[1];"));
        assert!(qasm.contains("y q[2];"));
        assert!(qasm.contains("z q[3];"));
        assert!(qasm.contains("s q[0];"));
        assert!(qasm.contains("t q[1];"));
        assert!(qasm.contains("ccx q[0], q[1], q[2];"));
    }

    #[test]
    fn test_circuit_all_gate_types_qiskit() {
        let mut c = QuantumCircuit::new("all_gates", 5, 2);
        c.add_gate(QuantumGate::Y(0));
        c.add_gate(QuantumGate::S(1));
        c.add_gate(QuantumGate::T(2));
        c.add_gate(QuantumGate::CZ(0, 1));
        c.add_gate(QuantumGate::Swap(2, 3));
        c.add_gate(QuantumGate::Toffoli(0, 1, 2));
        let code = c.to_qiskit();
        assert!(code.contains("qc.y(0)"));
        assert!(code.contains("qc.s(1)"));
        assert!(code.contains("qc.t(2)"));
        assert!(code.contains("qc.cz(0, 1)"));
        assert!(code.contains("qc.swap(2, 3)"));
        assert!(code.contains("qc.ccx(0, 1, 2)"));
    }

    #[test]
    fn test_circuit_all_gate_types_cirq() {
        let mut c = QuantumCircuit::new("all_gates", 5, 2);
        c.add_gate(QuantumGate::Y(0));
        c.add_gate(QuantumGate::Z(1));
        c.add_gate(QuantumGate::S(2));
        c.add_gate(QuantumGate::T(3));
        c.add_gate(QuantumGate::Rx(0, 1.0));
        c.add_gate(QuantumGate::Ry(1, 2.0));
        c.add_gate(QuantumGate::Rz(2, 3.0));
        c.add_gate(QuantumGate::CZ(0, 1));
        c.add_gate(QuantumGate::Swap(2, 3));
        c.add_gate(QuantumGate::Toffoli(0, 1, 2));
        c.add_gate(QuantumGate::Measure(0, 0));
        let code = c.to_cirq();
        assert!(code.contains("cirq.Y(qubits[0])"));
        assert!(code.contains("cirq.Z(qubits[1])"));
        assert!(code.contains("cirq.S(qubits[2])"));
        assert!(code.contains("cirq.T(qubits[3])"));
        assert!(code.contains("cirq.CZ(qubits[0], qubits[1])"));
        assert!(code.contains("cirq.SWAP(qubits[2], qubits[3])"));
        assert!(code.contains("cirq.CCX(qubits[0], qubits[1], qubits[2])"));
        assert!(code.contains("cirq.measure(qubits[0], key='m0')"));
    }

    #[test]
    fn test_circuit_depth_toffoli() {
        let mut c = QuantumCircuit::new("tof", 3, 0);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::H(1));
        c.add_gate(QuantumGate::H(2));
        c.add_gate(QuantumGate::Toffoli(0, 1, 2));
        assert_eq!(c.depth(), 2);
    }

    // --- Statevector simulator tests ---

    #[test]
    fn test_simulator_init() {
        let sim = StatevectorSimulator::new(2).unwrap();
        assert_eq!(sim.state.len(), 4);
        assert!((sim.state[0].re - 1.0).abs() < 1e-10);
        assert!(sim.state[1].norm_sq() < 1e-10);
    }

    #[test]
    fn test_simulator_max_qubits() {
        assert!(StatevectorSimulator::new(17).is_err());
        assert!(StatevectorSimulator::new(0).is_err());
        assert!(StatevectorSimulator::new(16).is_ok());
    }

    #[test]
    fn test_h_gate_superposition() {
        let mut sim = StatevectorSimulator::new(1).unwrap();
        sim.apply_gate(&QuantumGate::H(0));
        let probs = sim.probabilities();
        assert_eq!(probs.len(), 2);
        for (_, p) in &probs {
            assert!((p - 0.5).abs() < 1e-10);
        }
    }

    #[test]
    fn test_x_gate_flip() {
        let mut sim = StatevectorSimulator::new(1).unwrap();
        sim.apply_gate(&QuantumGate::X(0));
        assert!(sim.state[0].norm_sq() < 1e-10);
        assert!((sim.state[1].re - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bell_state_simulation() {
        let circuit = AlgorithmTemplates::bell_state();
        let result = StatevectorSimulator::simulate_circuit(&circuit, 1000).unwrap();
        // Bell state should have 2 non-zero probabilities: |00⟩ and |11⟩
        assert_eq!(result.probabilities.len(), 2);
        for (label, p) in &result.probabilities {
            assert!(label == "00" || label == "11");
            assert!((p - 0.5).abs() < 1e-10);
        }
    }

    #[test]
    fn test_cnot_entanglement() {
        let mut sim = StatevectorSimulator::new(2).unwrap();
        sim.apply_gate(&QuantumGate::H(0));
        sim.apply_gate(&QuantumGate::Cnot(0, 1));
        let probs = sim.probabilities();
        assert_eq!(probs.len(), 2);
    }

    #[test]
    fn test_z_gate() {
        let mut sim = StatevectorSimulator::new(1).unwrap();
        sim.apply_gate(&QuantumGate::H(0));
        sim.apply_gate(&QuantumGate::Z(0));
        // Should produce |−⟩ state: (|0⟩ − |1⟩)/√2
        let s = 1.0 / std::f64::consts::SQRT_2;
        assert!((sim.state[0].re - s).abs() < 1e-10);
        assert!((sim.state[1].re + s).abs() < 1e-10);
    }

    #[test]
    fn test_swap_gate() {
        let mut sim = StatevectorSimulator::new(2).unwrap();
        sim.apply_gate(&QuantumGate::X(0)); // bit 0 set -> index 0b01
        sim.apply_gate(&QuantumGate::Swap(0, 1)); // swap bits -> index 0b10
        assert!((sim.state[0b10].re - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_toffoli_gate() {
        let mut sim = StatevectorSimulator::new(3).unwrap();
        sim.apply_gate(&QuantumGate::X(0)); // |001⟩ -> bit 0 = 1
        sim.apply_gate(&QuantumGate::X(1)); // |011⟩ -> bit 1 = 1
        sim.apply_gate(&QuantumGate::Toffoli(0, 1, 2)); // both controls on -> flip target
        // State should be |111⟩ = index 0b111 = 7
        assert!((sim.state[7].re - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sampling() {
        let mut sim = StatevectorSimulator::new(1).unwrap();
        sim.apply_gate(&QuantumGate::H(0));
        let samples = sim.sample(1000);
        assert!(samples.contains_key("0"));
        assert!(samples.contains_key("1"));
        let count_0 = *samples.get("0").unwrap_or(&0);
        let count_1 = *samples.get("1").unwrap_or(&0);
        assert_eq!(count_0 + count_1, 1000);
        // Should be roughly 50/50 (within 20% tolerance)
        assert!(count_0 > 300 && count_0 < 700);
    }

    #[test]
    fn test_rotation_gates() {
        let mut sim = StatevectorSimulator::new(1).unwrap();
        // Rx(pi) should act like X gate
        sim.apply_gate(&QuantumGate::Rx(0, std::f64::consts::PI));
        assert!(sim.state[0].norm_sq() < 1e-8);
        assert!(sim.state[1].norm_sq() > 0.99);
    }

    // --- Optimizer tests ---

    #[test]
    fn test_optimizer_hh_cancel() {
        let mut c = QuantumCircuit::new("test", 1, 0);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::H(0));
        let (opt, result) = CircuitOptimizer::optimize(&c);
        assert_eq!(opt.gate_count(), 0);
        assert!(result.rules_applied.iter().any(|r| r.contains("HH→I")));
    }

    #[test]
    fn test_optimizer_ss_to_z() {
        let mut c = QuantumCircuit::new("test", 1, 0);
        c.add_gate(QuantumGate::S(0));
        c.add_gate(QuantumGate::S(0));
        let (opt, _) = CircuitOptimizer::optimize(&c);
        assert_eq!(opt.gate_count(), 1);
        assert!(matches!(opt.gates[0], QuantumGate::Z(0)));
    }

    #[test]
    fn test_optimizer_rotation_merge() {
        let mut c = QuantumCircuit::new("test", 1, 0);
        c.add_gate(QuantumGate::Rx(0, 0.5));
        c.add_gate(QuantumGate::Rx(0, 0.3));
        let (opt, result) = CircuitOptimizer::optimize(&c);
        assert_eq!(opt.gate_count(), 1);
        if let QuantumGate::Rx(_, angle) = opt.gates[0] {
            assert!((angle - 0.8).abs() < 1e-10);
        } else { panic!("Expected Rx gate"); }
        assert!(result.rules_applied.iter().any(|r| r.contains("Rx merge")));
    }

    #[test]
    fn test_optimizer_rotation_to_zero() {
        let mut c = QuantumCircuit::new("test", 1, 0);
        c.add_gate(QuantumGate::Rz(0, 1.0));
        c.add_gate(QuantumGate::Rz(0, -1.0));
        let (opt, _) = CircuitOptimizer::optimize(&c);
        assert_eq!(opt.gate_count(), 0);
    }

    #[test]
    fn test_optimizer_cnot_cancel() {
        let mut c = QuantumCircuit::new("test", 2, 0);
        c.add_gate(QuantumGate::Cnot(0, 1));
        c.add_gate(QuantumGate::Cnot(0, 1));
        let (opt, _) = CircuitOptimizer::optimize(&c);
        assert_eq!(opt.gate_count(), 0);
    }

    #[test]
    fn test_optimizer_no_change() {
        let mut c = QuantumCircuit::new("test", 2, 2);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::Cnot(0, 1));
        let (opt, result) = CircuitOptimizer::optimize(&c);
        assert_eq!(opt.gate_count(), 2);
        assert_eq!(result.savings_percent, 0.0);
    }

    // --- Algorithm template tests ---

    #[test]
    fn test_bell_state_template() {
        let c = AlgorithmTemplates::bell_state();
        assert_eq!(c.num_qubits, 2);
        assert_eq!(c.gate_count(), 4); // H + CNOT + 2 Measure
    }

    #[test]
    fn test_ghz_template() {
        let c = AlgorithmTemplates::ghz_state(4);
        assert_eq!(c.num_qubits, 4);
        // H(0) + 3 CNOTs + 4 Measures = 8
        assert_eq!(c.gate_count(), 8);
    }

    #[test]
    fn test_qft_template() {
        let c = AlgorithmTemplates::qft(3);
        assert_eq!(c.num_qubits, 3);
        assert!(c.gate_count() > 0);
    }

    #[test]
    fn test_bv_template() {
        let c = AlgorithmTemplates::bernstein_vazirani("110");
        assert_eq!(c.num_qubits, 4); // 3 + 1 ancilla
    }

    #[test]
    fn test_vqe_template() {
        let c = AlgorithmTemplates::vqe_ansatz(3, 2);
        assert_eq!(c.num_qubits, 3);
        assert!(c.gate_count() > 0);
    }

    #[test]
    fn test_template_lookup() {
        let params = std::collections::HashMap::new();
        assert!(AlgorithmTemplates::get_template("bell", &params).is_ok());
        assert!(AlgorithmTemplates::get_template("unknown", &params).is_err());
    }

    #[test]
    fn test_template_list() {
        let list = AlgorithmTemplates::list();
        assert_eq!(list.len(), 8);
    }

    // --- Cost estimator tests ---

    #[test]
    fn test_cost_estimator_empty_circuit() {
        let c = QuantumCircuit::new("empty", 2, 2);
        let estimates = CostEstimator::estimate_all(&c, 1000);
        assert_eq!(estimates.len(), 3);
        for e in &estimates {
            assert!(e.estimated_cost_usd >= 0.0);
        }
    }

    #[test]
    fn test_cost_estimator_bell_state() {
        let c = AlgorithmTemplates::bell_state();
        let estimates = CostEstimator::estimate_all(&c, 1000);
        assert_eq!(estimates.len(), 3);
        // All should have non-zero cost
        for e in &estimates {
            assert!(e.estimated_cost_usd > 0.0, "{} cost was 0", e.provider);
        }
    }

    #[test]
    fn test_cost_scales_with_shots() {
        let c = AlgorithmTemplates::bell_state();
        let low = CostEstimator::estimate_ibm(&c, 100);
        let high = CostEstimator::estimate_ibm(&c, 10000);
        assert!(high.estimated_cost_usd > low.estimated_cost_usd);
    }

    // --- Scaffolder tests ---

    #[test]
    fn test_scaffold_qiskit() {
        let files = ProjectScaffolder::scaffold("qiskit", "myproject", 3).unwrap();
        assert!(files.len() >= 4);
        assert!(files.iter().any(|f| f.path == "main.py"));
        assert!(files.iter().any(|f| f.path == "requirements.txt"));
        assert!(files.iter().any(|f| f.path.contains("test_")));
        let main = files.iter().find(|f| f.path == "main.py").unwrap();
        assert!(main.content.contains("QuantumCircuit(3"));
    }

    #[test]
    fn test_scaffold_cirq() {
        let files = ProjectScaffolder::scaffold("cirq", "myproject", 4).unwrap();
        assert!(files.len() >= 3);
        let main = files.iter().find(|f| f.path == "main.py").unwrap();
        assert!(main.content.contains("cirq"));
    }

    #[test]
    fn test_scaffold_pennylane() {
        let files = ProjectScaffolder::scaffold("pennylane", "myproject", 2).unwrap();
        assert!(files.len() >= 3);
    }

    #[test]
    fn test_scaffold_qsharp() {
        let files = ProjectScaffolder::scaffold("q#", "myproject", 5).unwrap();
        assert!(files.len() >= 2);
    }

    #[test]
    fn test_scaffold_unknown() {
        assert!(ProjectScaffolder::scaffold("unknown", "p", 2).is_err());
    }
}
