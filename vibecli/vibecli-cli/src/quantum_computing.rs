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
    ARTIQ,               // M-Labs — Advanced Real-Time Infrastructure for Quantum
    QCtrl,               // Q-CTRL — firmware-level pulse optimisation
    Mitiq,               // Unitary Fund — quantum error mitigation OS layer
    Qibo,                // TII (UAE) — full-stack quantum OS
    PulseOS,             // Oxford Quantum Circuits — pulse-level control
    Staq,                // Princeton — full-stack quantum compiler OS
    Delft,               // QuTech — quantum network OS (QNodeOS prototype)
    FireOpal,            // Q-CTRL Fire Opal — automated error suppression
    TrueQ,               // Keysight True-Q — characterisation & mitigation
    QCS,                 // Rigetti Quantum Cloud Services
}

impl QuantumOS {
    pub fn name(&self) -> &'static str {
        match self {
            Self::QiskitRuntime => "Qiskit Runtime",
            Self::AzureQuantum => "Azure Quantum",
            Self::AmazonBraket => "Amazon Braket",
            Self::CirqEngine => "Google Quantum Engine",
            Self::QuOS => "QUA / Quantum Machines OPX+",
            Self::ARTIQ => "ARTIQ",
            Self::QCtrl => "Q-CTRL Boulder Opal",
            Self::Mitiq => "Mitiq",
            Self::Qibo => "Qibo",
            Self::PulseOS => "Oxford QC Pulse OS",
            Self::Staq => "staq",
            Self::Delft => "QNodeOS (QuTech)",
            Self::FireOpal => "Q-CTRL Fire Opal",
            Self::TrueQ => "Keysight True-Q",
            Self::QCS => "Rigetti QCS",
        }
    }

    pub fn layer(&self) -> &'static str {
        match self {
            Self::QiskitRuntime | Self::AzureQuantum | Self::AmazonBraket
            | Self::CirqEngine | Self::QCS => "Cloud orchestration",
            Self::QuOS | Self::ARTIQ | Self::PulseOS => "Hardware control plane",
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
            Self::ARTIQ => "M-Labs (NIST / Oxford)",
            Self::QCtrl | Self::FireOpal => "Q-CTRL",
            Self::Mitiq => "Unitary Fund",
            Self::Qibo => "Technology Innovation Institute",
            Self::PulseOS => "Oxford Quantum Circuits",
            Self::Staq => "Princeton / Yale",
            Self::Delft => "QuTech (TU Delft + TNO)",
            Self::TrueQ => "Keysight Technologies",
            Self::QCS => "Rigetti Computing",
        }
    }

    pub fn all() -> Vec<QuantumOS> {
        vec![
            Self::QiskitRuntime, Self::AzureQuantum, Self::AmazonBraket,
            Self::CirqEngine, Self::QuOS, Self::ARTIQ, Self::QCtrl,
            Self::Mitiq, Self::Qibo, Self::PulseOS, Self::Staq,
            Self::Delft, Self::FireOpal, Self::TrueQ, Self::QCS,
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
    VQE,
    QAOA,
    QPE,
    BernsteinVazirani,
    DeutschJozsa,
    SimonProblem,
    HHL,
    QuantumWalk,
    QuantumMonteCarlo,
    QSVM,
    QNN,
    QuantumBoltzmann,
    DMRG,
}

impl QuantumAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Grover => "Grover's search",
            Self::Shor => "Shor's factoring",
            Self::VQE => "Variational Quantum Eigensolver (VQE)",
            Self::QAOA => "Quantum Approximate Optimisation (QAOA)",
            Self::QPE => "Quantum Phase Estimation (QPE)",
            Self::BernsteinVazirani => "Bernstein–Vazirani",
            Self::DeutschJozsa => "Deutsch–Jozsa",
            Self::SimonProblem => "Simon's problem",
            Self::HHL => "HHL (linear systems)",
            Self::QuantumWalk => "Quantum walk",
            Self::QuantumMonteCarlo => "Quantum Monte Carlo",
            Self::QSVM => "Quantum SVM",
            Self::QNN => "Quantum Neural Network",
            Self::QuantumBoltzmann => "Quantum Boltzmann Machine",
            Self::DMRG => "DMRG / tensor-network",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::Grover | Self::BernsteinVazirani | Self::DeutschJozsa
            | Self::SimonProblem => "Oracle / search",
            Self::Shor | Self::QPE => "Number-theoretic",
            Self::VQE | Self::QAOA => "Variational (NISQ-friendly)",
            Self::HHL | Self::QuantumMonteCarlo => "Linear algebra / simulation",
            Self::QuantumWalk => "Graph / combinatorial",
            Self::QSVM | Self::QNN | Self::QuantumBoltzmann => "Quantum machine learning",
            Self::DMRG => "Tensor-network / chemistry",
        }
    }

    pub fn qubit_scaling(&self) -> &'static str {
        match self {
            Self::Grover => "O(√N) queries, N qubits for N-item search",
            Self::Shor => "O(n³) gates for n-bit integer (2n+3 qubits)",
            Self::VQE => "Problem-dependent ansatz depth, typically 4-50 qubits (NISQ)",
            Self::QAOA => "Problem-size + p rounds of alternating unitaries",
            Self::QPE => "O(1/ε) ancilla qubits for precision ε",
            Self::BernsteinVazirani => "n qubits for n-bit secret",
            Self::DeutschJozsa => "n+1 qubits, single oracle query",
            Self::SimonProblem => "n qubits, O(n) queries",
            Self::HHL => "O(log N) qubits for N×N system (exponential speedup)",
            Self::QuantumWalk => "O(log N) qubits for N-vertex graph",
            Self::QuantumMonteCarlo => "Quadratic speedup over classical MC",
            Self::QSVM => "O(log N) qubits for N-dimensional feature space",
            Self::QNN => "Parameterised circuit depth × width",
            Self::QuantumBoltzmann => "Visible + hidden qubit layers",
            Self::DMRG => "Bond dimension dependent, hybrid classical-quantum",
        }
    }

    pub fn all() -> Vec<QuantumAlgorithm> {
        vec![
            Self::Grover, Self::Shor, Self::VQE, Self::QAOA, Self::QPE,
            Self::BernsteinVazirani, Self::DeutschJozsa, Self::SimonProblem,
            Self::HHL, Self::QuantumWalk, Self::QuantumMonteCarlo,
            Self::QSVM, Self::QNN, Self::QuantumBoltzmann, Self::DMRG,
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
    CNOT(usize, usize),            // Controlled-NOT
    CZ(usize, usize),              // Controlled-Z
    SWAP(usize, usize),            // SWAP
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
            Self::CNOT(c, t) => format!("cx q[{}], q[{}];", c, t),
            Self::CZ(c, t) => format!("cz q[{}], q[{}];", c, t),
            Self::SWAP(a, b) => format!("swap q[{}], q[{}];", a, b),
            Self::Toffoli(a, b, t) => format!("ccx q[{}], q[{}], q[{}];", a, b, t),
            Self::Measure(q, c) => format!("c[{}] = measure q[{}];", c, q),
        }
    }

    pub fn max_qubit(&self) -> usize {
        match self {
            Self::H(q) | Self::X(q) | Self::Y(q) | Self::Z(q)
            | Self::S(q) | Self::T(q)
            | Self::Rx(q, _) | Self::Ry(q, _) | Self::Rz(q, _) => *q,
            Self::CNOT(a, b) | Self::CZ(a, b) | Self::SWAP(a, b) => (*a).max(*b),
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
                QuantumGate::CNOT(a, b) | QuantumGate::CZ(a, b) | QuantumGate::SWAP(a, b) => {
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
            QuantumGate::CNOT(..) | QuantumGate::CZ(..) | QuantumGate::SWAP(..)
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
                QuantumGate::CNOT(c, t) => format!("qc.cx({}, {})", c, t),
                QuantumGate::CZ(c, t) => format!("qc.cz({}, {})", c, t),
                QuantumGate::SWAP(a, b) => format!("qc.swap({}, {})", a, b),
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
                QuantumGate::CNOT(c, t) => format!("    cirq.CNOT(qubits[{}], qubits[{}]),", c, t),
                QuantumGate::CZ(c, t) => format!("    cirq.CZ(qubits[{}], qubits[{}]),", c, t),
                QuantumGate::SWAP(a, b) => format!("    cirq.SWAP(qubits[{}], qubits[{}]),", a, b),
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
            (QuantumLanguage::TKet, vec![QuantumOS::QiskitRuntime, QuantumOS::AzureQuantum, QuantumOS::AmazonBraket, QuantumOS::QCS]),
            (QuantumLanguage::CudaQuantum, vec![QuantumOS::QiskitRuntime, QuantumOS::CirqEngine]),
            (QuantumLanguage::Bloqade, vec![QuantumOS::AmazonBraket]),
            (QuantumLanguage::OpenQASM3, vec![QuantumOS::QiskitRuntime, QuantumOS::AzureQuantum, QuantumOS::AmazonBraket]),
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
        let a = QuantumOS::ARTIQ;
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
        let vqe = QuantumAlgorithm::VQE;
        assert_eq!(vqe.category(), "Variational (NISQ-friendly)");
    }

    #[test]
    fn test_qml_algorithms() {
        assert_eq!(QuantumAlgorithm::QSVM.category(), "Quantum machine learning");
        assert_eq!(QuantumAlgorithm::QNN.category(), "Quantum machine learning");
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
        assert_eq!(QuantumGate::CNOT(0, 1).qasm3(), "cx q[0], q[1];");
        assert_eq!(QuantumGate::CZ(1, 2).qasm3(), "cz q[1], q[2];");
        assert_eq!(QuantumGate::SWAP(0, 3).qasm3(), "swap q[0], q[3];");
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
        assert_eq!(QuantumGate::CNOT(1, 4).max_qubit(), 4);
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
        c.add_gate(QuantumGate::CNOT(0, 1));
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
        c.add_gate(QuantumGate::CNOT(0, 1));
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
        c.add_gate(QuantumGate::CNOT(0, 1));
        c.add_gate(QuantumGate::CZ(1, 2));
        c.add_gate(QuantumGate::X(0));
        assert_eq!(c.two_qubit_gate_count(), 2);
    }

    #[test]
    fn test_circuit_to_qasm3() {
        let mut c = QuantumCircuit::new("bell", 2, 2);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::CNOT(0, 1));
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
        c.add_gate(QuantumGate::CNOT(0, 1));
        let code = c.to_qiskit();
        assert!(code.contains("QuantumCircuit(2, 2)"));
        assert!(code.contains("qc.h(0)"));
        assert!(code.contains("qc.cx(0, 1)"));
    }

    #[test]
    fn test_circuit_to_cirq() {
        let mut c = QuantumCircuit::new("bell", 2, 0);
        c.add_gate(QuantumGate::H(0));
        c.add_gate(QuantumGate::CNOT(0, 1));
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
        mgr.add_gate_to_circuit(idx, QuantumGate::CNOT(0, 1));
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
        c.add_gate(QuantumGate::CNOT(0, 1));
        c.add_gate(QuantumGate::CZ(2, 3));
        c.add_gate(QuantumGate::SWAP(0, 4));
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
        c.add_gate(QuantumGate::SWAP(2, 3));
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
        c.add_gate(QuantumGate::SWAP(2, 3));
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
}
