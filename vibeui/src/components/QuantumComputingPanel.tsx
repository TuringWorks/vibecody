import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ────────────────────────────────────────────────────────────────────

interface QuantumLanguageInfo {
  name: string;
  hostLanguage: string;
  vendor: string;
  installCommand: string;
}

interface QuantumOSInfo {
  name: string;
  layer: string;
  vendor: string;
}

interface QuantumProject {
  id: string;
  name: string;
  language: string;
  targetOs: string | null;
  targetHardware: string;
  algorithm: string | null;
  errorCorrection: string | null;
  numQubits: number;
  description: string;
  estimatedPhysicalQubits: number | null;
}

interface QuantumCircuit {
  index: number;
  name: string;
  numQubits: number;
  numClassical: number;
  gateCount: number;
  depth: number;
  twoQubitGates: number;
}

interface CompatEntry {
  language: string;
  compatibleOs: string[];
}

interface GateInstance {
  type: string;
  targets?: number[];
  target?: number;
  control?: number;
  controls?: number[];
  angle?: number;
  qubit?: number;
  classical?: number;
}

interface CircuitDetail extends QuantumCircuit {
  gates: GateInstance[];
}

interface SimulationResult {
  amplitudes: [string, number, number][];
  probabilities: [string, number][];
  samples: Record<string, number>;
  num_qubits: number;
}

interface OptimizationResult {
  original_gate_count: number;
  optimized_gate_count: number;
  original_depth: number;
  optimized_depth: number;
  rules_applied: string[];
  savings_percent: number;
}

interface CostEstimate {
  provider: string;
  estimated_cost_usd: number;
  breakdown: [string, number][];
  notes: string[];
}

interface ScaffoldFile {
  path: string;
  content: string;
}

interface AlgorithmTemplate {
  name: string;
  description: string;
}

interface HardwareTopology {
  name: string;
  vendor: string;
  qubitCount: number;
  qubits: { id: number; x: number; y: number }[];
  couplings: [number, number][];
}

type QuantumTab =
  | "circuitBuilder"
  | "simulator"
  | "optimizer"
  | "cost"
  | "templates"
  | "scaffold"
  | "topology"
  | "languages"
  | "os"
  | "projects"
  | "algorithms";

const TABS: { id: QuantumTab; label: string }[] = [
  { id: "circuitBuilder", label: "Circuit Builder" },
  { id: "simulator", label: "Simulator" },
  { id: "optimizer", label: "Optimizer" },
  { id: "cost", label: "Cost" },
  { id: "templates", label: "Templates" },
  { id: "scaffold", label: "Scaffold" },
  { id: "topology", label: "Topology" },
  { id: "languages", label: "Languages" },
  { id: "os", label: "Quantum OS" },
  { id: "projects", label: "Projects" },
  { id: "algorithms", label: "Algorithms" },
];

// ── Algorithm Code Examples ─────────────────────────────────────────────────

const ALGORITHM_EXAMPLES: Record<string, Record<string, string>> = {
  "Grover's Search": {
    Qiskit: `from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator

qc = QuantumCircuit(2, 2)
# Superposition
qc.h([0, 1])
# Oracle: mark |11⟩
qc.cz(0, 1)
# Diffusion operator
qc.h([0, 1])
qc.z([0, 1])
qc.cz(0, 1)
qc.h([0, 1])
# Measure
qc.measure([0, 1], [0, 1])

sim = AerSimulator()
result = sim.run(qc, shots=1024).result()
print(result.get_counts())  # |11⟩ dominant`,
    Cirq: `import cirq

q0, q1 = cirq.LineQubit.range(2)
circuit = cirq.Circuit([
    cirq.H(q0), cirq.H(q1),          # Superposition
    cirq.CZ(q0, q1),                  # Oracle: mark |11⟩
    cirq.H(q0), cirq.H(q1),          # Diffusion
    cirq.Z(q0), cirq.Z(q1),
    cirq.CZ(q0, q1),
    cirq.H(q0), cirq.H(q1),
    cirq.measure(q0, q1, key='result')
])

sim = cirq.Simulator()
result = sim.run(circuit, repetitions=1024)
print(result.histogram(key='result'))`,
    PennyLane: `import pennylane as qml
from pennylane import numpy as np

dev = qml.device('default.qubit', wires=2, shots=1024)

@qml.qnode(dev)
def grover():
    # Superposition
    qml.Hadamard(wires=0)
    qml.Hadamard(wires=1)
    # Oracle: mark |11⟩
    qml.CZ(wires=[0, 1])
    # Diffusion
    qml.Hadamard(wires=0)
    qml.Hadamard(wires=1)
    qml.PauliZ(wires=0)
    qml.PauliZ(wires=1)
    qml.CZ(wires=[0, 1])
    qml.Hadamard(wires=0)
    qml.Hadamard(wires=1)
    return qml.counts()

print(grover())  # |11⟩ dominant`,
  },
  "Shor's Factoring": {
    Qiskit: `from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator
import numpy as np

# Simplified Shor's for N=15, a=7
# Uses 4 counting qubits + 4 work qubits
qc = QuantumCircuit(8, 4)

# Initialize counting register in superposition
for i in range(4):
    qc.h(i)

# Modular exponentiation (simplified)
qc.x(4)  # Set work register to |1⟩

# Controlled modular multiplications
# (simplified — full implementation needs modular arithmetic circuits)
qc.cx(0, 4)

# Inverse QFT on counting register
for i in range(2):
    qc.swap(i, 3 - i)
for i in range(4):
    qc.h(i)
    for j in range(i + 1, 4):
        qc.cp(-np.pi / 2**(j - i), j, i)

qc.measure(range(4), range(4))

sim = AerSimulator()
result = sim.run(qc, shots=1024).result()
print(result.get_counts())`,
    Cirq: `import cirq
import numpy as np

# Simplified Shor's period-finding circuit
n_count = 4
qubits = cirq.LineQubit.range(n_count + 4)
circuit = cirq.Circuit()

# Hadamard on counting qubits
circuit.append(cirq.H.on_each(*qubits[:n_count]))
# Initialize work to |1⟩
circuit.append(cirq.X(qubits[n_count]))
# Controlled modular exponentiation (simplified)
circuit.append(cirq.CNOT(qubits[0], qubits[n_count]))
# Inverse QFT
for i in range(n_count // 2):
    circuit.append(cirq.SWAP(qubits[i], qubits[n_count - 1 - i]))
for i in range(n_count):
    circuit.append(cirq.H(qubits[i]))

circuit.append(cirq.measure(*qubits[:n_count], key='result'))
sim = cirq.Simulator()
print(sim.run(circuit, repetitions=1024).histogram(key='result'))`,
  },
  "VQE": {
    Qiskit: `from qiskit import QuantumCircuit
from qiskit.primitives import Estimator
from qiskit.quantum_info import SparsePauliOp
from scipy.optimize import minimize

# H2 Hamiltonian (simplified)
hamiltonian = SparsePauliOp.from_list([
    ("II", -1.05), ("IZ", 0.39), ("ZI", -0.39),
    ("ZZ", -0.01), ("XX", 0.18)
])

def ansatz(params):
    qc = QuantumCircuit(2)
    qc.ry(params[0], 0)
    qc.ry(params[1], 1)
    qc.cx(0, 1)
    qc.ry(params[2], 0)
    qc.ry(params[3], 1)
    return qc

def cost_fn(params):
    qc = ansatz(params)
    estimator = Estimator()
    result = estimator.run(qc, hamiltonian).result()
    return result.values[0]

result = minimize(cost_fn, x0=[0.1]*4, method='COBYLA')
print(f"Ground state energy: {result.fun:.4f} Ha")`,
    PennyLane: `import pennylane as qml
from pennylane import numpy as np

dev = qml.device('default.qubit', wires=2)

# H2 Hamiltonian
coeffs = [-1.05, 0.39, -0.39, -0.01, 0.18]
obs = [qml.Identity(0) @ qml.Identity(1),
       qml.Identity(0) @ qml.PauliZ(1),
       qml.PauliZ(0) @ qml.Identity(1),
       qml.PauliZ(0) @ qml.PauliZ(1),
       qml.PauliX(0) @ qml.PauliX(1)]
H = qml.Hamiltonian(coeffs, obs)

@qml.qnode(dev)
def circuit(params):
    qml.RY(params[0], wires=0)
    qml.RY(params[1], wires=1)
    qml.CNOT(wires=[0, 1])
    qml.RY(params[2], wires=0)
    qml.RY(params[3], wires=1)
    return qml.expval(H)

opt = qml.GradientDescentOptimizer(stepsize=0.4)
params = np.array([0.1, 0.1, 0.1, 0.1], requires_grad=True)
for i in range(100):
    params = opt.step(circuit, params)
print(f"Ground state energy: {circuit(params):.4f} Ha")`,
  },
  "QAOA": {
    Qiskit: `from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator
import numpy as np

n = 4  # qubits for MaxCut
gamma, beta = 0.5, 0.5

qc = QuantumCircuit(n, n)
# Initial superposition
for i in range(n):
    qc.h(i)
# Cost layer: ZZ on edges
edges = [(0,1), (1,2), (2,3), (0,3)]
for (i, j) in edges:
    qc.cx(i, j)
    qc.rz(2 * gamma, j)
    qc.cx(i, j)
# Mixer layer
for i in range(n):
    qc.rx(2 * beta, i)
qc.measure(range(n), range(n))

sim = AerSimulator()
result = sim.run(qc, shots=1024).result()
print(sorted(result.get_counts().items(), key=lambda x: -x[1])[:5])`,
    PennyLane: `import pennylane as qml
from pennylane import numpy as np

n = 4
dev = qml.device('default.qubit', wires=n, shots=1024)
edges = [(0,1), (1,2), (2,3), (0,3)]

def cost_layer(gamma):
    for (i, j) in edges:
        qml.CNOT(wires=[i, j])
        qml.RZ(2 * gamma, wires=j)
        qml.CNOT(wires=[i, j])

def mixer_layer(beta):
    for i in range(n):
        qml.RX(2 * beta, wires=i)

@qml.qnode(dev)
def qaoa(params):
    for i in range(n):
        qml.Hadamard(wires=i)
    cost_layer(params[0])
    mixer_layer(params[1])
    return qml.counts()

print(qaoa([0.5, 0.5]))`,
  },
  "Quantum Phase Estimation": {
    Qiskit: `from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator
import numpy as np

n_count = 3  # precision qubits
qc = QuantumCircuit(n_count + 1, n_count)

# Prepare eigenstate |1⟩ on target
qc.x(n_count)
# Hadamard on counting qubits
for i in range(n_count):
    qc.h(i)
# Controlled unitary powers (U = phase gate with θ=π/4)
for i in range(n_count):
    angle = 2 * np.pi / (2**(n_count - i))
    qc.cp(angle, i, n_count)
# Inverse QFT
for i in range(n_count // 2):
    qc.swap(i, n_count - 1 - i)
for i in range(n_count):
    qc.h(i)
    for j in range(i + 1, n_count):
        qc.cp(-np.pi / 2**(j - i), j, i)
qc.measure(range(n_count), range(n_count))

sim = AerSimulator()
result = sim.run(qc, shots=1024).result()
print(result.get_counts())`,
  },
  "Deutsch-Jozsa": {
    Qiskit: `from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator

n = 3  # input qubits
qc = QuantumCircuit(n + 1, n)

# Prepare ancilla in |1⟩
qc.x(n)
# Hadamard all qubits
qc.h(range(n + 1))
# Oracle: balanced function f(x) = x₀
qc.cx(0, n)
# Hadamard on input qubits
qc.h(range(n))
# Measure input qubits
qc.measure(range(n), range(n))

sim = AerSimulator()
result = sim.run(qc, shots=1024).result()
counts = result.get_counts()
# If all zeros → constant, otherwise → balanced
print("Balanced" if any(k != "0"*n for k in counts) else "Constant")`,
    Cirq: `import cirq

n = 3
qubits = cirq.LineQubit.range(n + 1)
circuit = cirq.Circuit()

circuit.append(cirq.X(qubits[n]))           # Ancilla |1⟩
circuit.append(cirq.H.on_each(*qubits))     # Hadamard all
circuit.append(cirq.CNOT(qubits[0], qubits[n]))  # Oracle
circuit.append(cirq.H.on_each(*qubits[:n])) # Hadamard inputs
circuit.append(cirq.measure(*qubits[:n], key='result'))

sim = cirq.Simulator()
result = sim.run(circuit, repetitions=1024)
print(result.histogram(key='result'))`,
  },
  "Bernstein-Vazirani": {
    Qiskit: `from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator

secret = "1011"  # hidden string to find
n = len(secret)
qc = QuantumCircuit(n + 1, n)

qc.x(n)                    # Ancilla |1⟩
qc.h(range(n + 1))         # Hadamard all
# Oracle: CNOT where secret bit = 1
for i, bit in enumerate(reversed(secret)):
    if bit == "1":
        qc.cx(i, n)
qc.h(range(n))             # Hadamard inputs
qc.measure(range(n), range(n))

sim = AerSimulator()
result = sim.run(qc, shots=1).result()
print(f"Found secret: {list(result.get_counts().keys())[0]}")`,
    Cirq: `import cirq

secret = "1011"
n = len(secret)
qubits = cirq.LineQubit.range(n + 1)
circuit = cirq.Circuit()

circuit.append(cirq.X(qubits[n]))
circuit.append(cirq.H.on_each(*qubits))
for i, bit in enumerate(reversed(secret)):
    if bit == "1":
        circuit.append(cirq.CNOT(qubits[i], qubits[n]))
circuit.append(cirq.H.on_each(*qubits[:n]))
circuit.append(cirq.measure(*qubits[:n], key='s'))

result = cirq.Simulator().run(circuit, repetitions=1)
print(f"Found secret: {result.measurements['s'][0]}")`,
  },
  "HHL Algorithm": {
    Qiskit: `# HHL solves Ax = b for quantum-encoded vectors
# Qiskit provides a built-in HHL implementation
from qiskit.algorithms.linear_solvers import HHL, NumPyLinearSolver
import numpy as np

# 2x2 system: A|x⟩ = |b⟩
A = np.array([[1, -1/3], [-1/3, 1]])
b = np.array([1, 0])

# Classical solution for comparison
classical = NumPyLinearSolver().solve(A, b)

# Quantum HHL solver
hhl = HHL()
quantum_solution = hhl.solve(A, b)
print(f"Classical: {classical.euclidean_norm:.4f}")
print(f"HHL:      {quantum_solution.euclidean_norm:.4f}")`,
  },
  "Quantum Walk": {
    Qiskit: `from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator
import numpy as np

n = 4  # position qubits (2^4 = 16 positions)
qc = QuantumCircuit(n + 1, n)  # +1 coin qubit

# Initial state: coin in superposition, position at 0
qc.h(0)  # coin qubit

# 5 steps of quantum walk
for step in range(5):
    # Coin flip (Hadamard on coin)
    qc.h(0)
    # Conditional shift: increment position if coin=|1⟩
    for i in range(n):
        qc.cx(0, i + 1)

qc.measure(range(1, n + 1), range(n))

sim = AerSimulator()
result = sim.run(qc, shots=1024).result()
print(result.get_counts())`,
  },
  "QSVM": {
    PennyLane: `import pennylane as qml
from pennylane import numpy as np

dev = qml.device('default.qubit', wires=2)

# Quantum kernel: encode data, compute overlap
@qml.qnode(dev)
def kernel_circuit(x1, x2):
    # Encode x1
    qml.RX(x1[0], wires=0)
    qml.RY(x1[1], wires=1)
    # Adjoint encode x2
    qml.adjoint(qml.RY)(x2[1], wires=1)
    qml.adjoint(qml.RX)(x2[0], wires=0)
    return qml.probs(wires=[0, 1])

def kernel(x1, x2):
    return kernel_circuit(x1, x2)[0]  # |00⟩ probability

# Example: compute kernel matrix
X = np.array([[0.1, 0.2], [0.5, 0.8], [1.0, 0.3]])
K = np.array([[kernel(x1, x2) for x2 in X] for x1 in X])
print("Kernel matrix:", K)`,
  },
  "QNN": {
    PennyLane: `import pennylane as qml
from pennylane import numpy as np

n_qubits, n_layers = 4, 3
dev = qml.device('default.qubit', wires=n_qubits)

@qml.qnode(dev)
def qnn(inputs, weights):
    # Encode inputs
    for i in range(n_qubits):
        qml.RX(inputs[i], wires=i)
    # Variational layers
    for l in range(n_layers):
        for i in range(n_qubits):
            qml.RY(weights[l][i], wires=i)
        for i in range(n_qubits - 1):
            qml.CNOT(wires=[i, i+1])
    return qml.expval(qml.PauliZ(0))

weights = np.random.uniform(0, np.pi, (n_layers, n_qubits))
inputs = np.array([0.1, 0.5, 0.3, 0.8])
output = qnn(inputs, weights)
print(f"QNN output: {output:.4f}")`,
  },
};

// ── SVG Constants ────────────────────────────────────────────────────────────

const WIRE_Y0 = 30;
const WIRE_SPACING = 50;
const COL_WIDTH = 55;
const GATE_SIZE = 34;

// ── Gate Definitions ─────────────────────────────────────────────────────────

const SINGLE_QUBIT_GATES = ["H", "X", "Y", "Z", "S", "T"];
const ROTATION_GATES = ["Rx", "Ry", "Rz"];
const MULTI_QUBIT_GATES = ["CNOT", "CZ", "SWAP", "Toffoli"];
const MEASUREMENT_GATES = ["Measure"];

const GATE_COLORS: Record<string, string> = {
  H: "#4a90d9",
  X: "#e74c3c",
  Y: "#27ae60",
  Z: "#8e44ad",
  S: "#f39c12",
  T: "#1abc9c",
  Rx: "#e67e22",
  Ry: "#2ecc71",
  Rz: "#9b59b6",
  CNOT: "#3498db",
  CZ: "#2980b9",
  SWAP: "#e74c3c",
  Toffoli: "#34495e",
  Measure: "#7f8c8d",
};

// ── Topology Generators ──────────────────────────────────────────────────────

function generateGrid(rows: number, cols: number, count: number) {
  const qubits: { id: number; x: number; y: number }[] = [];
  let id = 0;
  for (let r = 0; r < rows && id < count; r++)
    for (let c = 0; c < cols && id < count; c++)
      qubits.push({ id: id++, x: c * 40 + 20, y: r * 40 + 20 });
  return qubits;
}

function generateCircle(count: number) {
  return Array.from({ length: count }, (_, i) => ({
    id: i,
    x: 150 + 120 * Math.cos((2 * Math.PI * i) / count),
    y: 150 + 120 * Math.sin((2 * Math.PI * i) / count),
  }));
}

function generateGridCouplings(rows: number, cols: number, count: number): [number, number][] {
  const couplings: [number, number][] = [];
  for (let r = 0; r < rows; r++)
    for (let c = 0; c < cols; c++) {
      const id = r * cols + c;
      if (id >= count) continue;
      if (c + 1 < cols && id + 1 < count) couplings.push([id, id + 1]);
      if (r + 1 < rows && id + cols < count) couplings.push([id, id + cols]);
    }
  return couplings;
}

function generateAllToAll(count: number): [number, number][] {
  const c: [number, number][] = [];
  for (let i = 0; i < count; i++)
    for (let j = i + 1; j < count; j++) c.push([i, j]);
  return c;
}

function generateHeavyHex(count: number) {
  const qubits: { id: number; x: number; y: number }[] = [];
  const cols = 18;
  const rows = Math.ceil(count / cols);
  let id = 0;
  for (let r = 0; r < rows && id < count; r++) {
    for (let c = 0; c < cols && id < count; c++) {
      const xOff = r % 2 === 1 ? 20 : 0;
      qubits.push({ id: id++, x: c * 38 + 18 + xOff, y: r * 42 + 18 });
    }
  }
  return qubits;
}

function generateHeavyHexCouplings(count: number): [number, number][] {
  const cols = 18;
  const rows = Math.ceil(count / cols);
  const couplings: [number, number][] = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const id = r * cols + c;
      if (id >= count) continue;
      if (c + 1 < cols && id + 1 < count) couplings.push([id, id + 1]);
      if (r + 1 < rows && id + cols < count && (c % 2 === 0 || r % 2 === 0))
        couplings.push([id, id + cols]);
    }
  }
  return couplings;
}

const TOPOLOGIES: HardwareTopology[] = [
  {
    name: "IBM Eagle r3",
    vendor: "IBM",
    qubitCount: 127,
    qubits: generateHeavyHex(127),
    couplings: generateHeavyHexCouplings(127),
  },
  {
    name: "Google Sycamore",
    vendor: "Google",
    qubitCount: 53,
    qubits: generateGrid(6, 9, 53),
    couplings: generateGridCouplings(6, 9, 53),
  },
  {
    name: "IonQ Aria",
    vendor: "IonQ",
    qubitCount: 25,
    qubits: generateCircle(25),
    couplings: generateAllToAll(25),
  },
  {
    name: "Rigetti Ankaa-2",
    vendor: "Rigetti",
    qubitCount: 84,
    qubits: generateGrid(9, 10, 84),
    couplings: generateGridCouplings(9, 10, 84),
  },
  {
    name: "Quantinuum H2",
    vendor: "Quantinuum",
    qubitCount: 32,
    qubits: generateCircle(32),
    couplings: generateAllToAll(32),
  },
];

// ── Bloch Sphere Math ────────────────────────────────────────────────────────

function amplitudesToBloch(re0: number, im0: number, re1: number, im1: number) {
  const r0 = Math.sqrt(re0 * re0 + im0 * im0);
  const theta = 2 * Math.acos(Math.min(r0, 1.0));
  const phi = Math.atan2(im1, re1) - Math.atan2(im0, re0);
  return { theta, phi };
}

// ── Shared Styles ────────────────────────────────────────────────────────────

const inputStyle: React.CSSProperties = {
  padding: "4px 8px",
  borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-secondary)",
  color: "var(--text-primary)",
  fontSize: 12,
};

const btnPrimary: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "none",
  background: "var(--accent-primary)",
  color: "var(--btn-primary-fg)",
  cursor: "pointer",
  fontWeight: 600,
  fontSize: 13,
};

const btnSmall: React.CSSProperties = {
  background: "none",
  border: "1px solid var(--border-color)",
  borderRadius: 4,
  color: "var(--text-secondary)",
  cursor: "pointer",
  padding: "2px 6px",
  fontSize: 11,
};

const cardStyle: React.CSSProperties = {
  padding: 12,
  borderRadius: 8,
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
};

// ── Component ────────────────────────────────────────────────────────────────

export function QuantumComputingPanel() {
  const [tab, setTab] = useState<QuantumTab>("circuitBuilder");

  // Shared data
  const [languages, setLanguages] = useState<QuantumLanguageInfo[]>([]);
  const [osList, setOsList] = useState<QuantumOSInfo[]>([]);
  const [projects, setProjects] = useState<QuantumProject[]>([]);
  const [circuits, setCircuits] = useState<QuantumCircuit[]>([]);
  const [compat, setCompat] = useState<CompatEntry[]>([]);
  const [algorithms, setAlgorithms] = useState<{ name: string; category: string; scaling: string }[]>([]);
  const [hardware, setHardware] = useState<{ type: string; vendors: string[] }[]>([]);

  // Languages tab
  const [helloCode, setHelloCode] = useState<string>("");
  const [selectedLang, setSelectedLang] = useState<string>("");

  // New project form
  const [npName, setNpName] = useState("");
  const [npLang, setNpLang] = useState("Qiskit");
  const [npHw, setNpHw] = useState("Superconducting");
  const [npQubits, setNpQubits] = useState(2);
  const [npDesc, setNpDesc] = useState("");

  // Circuit Builder state
  const [selectedCircuitIdx, setSelectedCircuitIdx] = useState<number | null>(null);
  const [circuitDetail, setCircuitDetail] = useState<CircuitDetail | null>(null);
  const [selectedGate, setSelectedGate] = useState<string | null>(null);
  const [placingControl, setPlacingControl] = useState<number | null>(null);
  const [placingControls, setPlacingControls] = useState<number[]>([]);
  const [cbNewName, setCbNewName] = useState("");
  const [cbNewQubits, setCbNewQubits] = useState(3);
  const [cbNewClassical, setCbNewClassical] = useState(3);

  // Simulator state
  const [simCircuitIdx, setSimCircuitIdx] = useState<number | null>(null);
  const [simShots, setSimShots] = useState(1024);
  const [simResult, setSimResult] = useState<SimulationResult | null>(null);
  const [simRunning, setSimRunning] = useState(false);

  // Optimizer state
  const [optCircuitIdx, setOptCircuitIdx] = useState<number | null>(null);
  const [optResult, setOptResult] = useState<OptimizationResult | null>(null);
  const [optRunning, setOptRunning] = useState(false);

  // Cost state
  const [costCircuitIdx, setCostCircuitIdx] = useState<number | null>(null);
  const [costShots, setCostShots] = useState(1024);
  const [costEstimates, setCostEstimates] = useState<CostEstimate[]>([]);
  const [costRunning, setCostRunning] = useState(false);

  // Templates state
  const [templateList, setTemplateList] = useState<AlgorithmTemplate[]>([]);
  const [tplQubits, setTplQubits] = useState(3);
  const [tplSecret, setTplSecret] = useState("101");
  const [tplLayers, setTplLayers] = useState(2);
  const [tplGamma, setTplGamma] = useState(0.5);
  const [tplBeta, setTplBeta] = useState(0.5);
  const [tplLoading, setTplLoading] = useState<string | null>(null);
  const [algoExpanded, setAlgoExpanded] = useState<string | null>(null);

  // Scaffold state
  const [scafName, setScafName] = useState("my-quantum-project");
  const [scafLang, setScafLang] = useState("Qiskit");
  const [scafQubits, setScafQubits] = useState(4);
  const [scafFiles, setScafFiles] = useState<ScaffoldFile[]>([]);
  const [scafRunning, setScafRunning] = useState(false);
  const [scafExpanded, setScafExpanded] = useState<Set<string>>(new Set());

  // Topology state
  const [selectedTopology, setSelectedTopology] = useState(0);

  useEffect(() => {
    loadAll();
  }, []);

  async function loadAll() {
    try {
      const [langs, oses, projs, circs, comp, algs, hw] = await Promise.all([
        invoke<QuantumLanguageInfo[]>("quantum_get_languages"),
        invoke<QuantumOSInfo[]>("quantum_get_os_list"),
        invoke<QuantumProject[]>("quantum_get_projects"),
        invoke<QuantumCircuit[]>("quantum_get_circuits"),
        invoke<CompatEntry[]>("quantum_get_compatibility"),
        invoke<{ name: string; category: string; scaling: string }[]>("quantum_get_algorithms"),
        invoke<{ type: string; vendors: string[] }[]>("quantum_get_hardware_types"),
      ]);
      setLanguages(langs);
      setOsList(oses);
      setProjects(projs);
      setCircuits(circs);
      setCompat(comp);
      setAlgorithms(algs);
      setHardware(hw);
    } catch {
      // individual loads may fail
    }
  }

  async function loadTemplates() {
    try {
      const tpls = await invoke<AlgorithmTemplate[]>("quantum_list_templates");
      setTemplateList(tpls);
    } catch {
      /* ignore */
    }
  }

  useEffect(() => {
    if (tab === "templates" && templateList.length === 0) {
      loadTemplates();
    }
  }, [tab]);

  // ── Languages helpers ────────────────────────────────────────────────────

  async function loadHelloCircuit(lang: string) {
    setSelectedLang(lang);
    try {
      const code = await invoke<string>("quantum_get_hello_circuit", { language: lang });
      setHelloCode(code);
    } catch {
      setHelloCode("// Not available for this language");
    }
  }

  // ── Projects helpers ─────────────────────────────────────────────────────

  async function createProject() {
    if (!npName.trim()) return;
    try {
      await invoke("quantum_create_project", {
        name: npName,
        language: npLang,
        hardware: npHw,
        numQubits: npQubits,
        description: npDesc,
      });
      setNpName("");
      setNpDesc("");
      const projs = await invoke<QuantumProject[]>("quantum_get_projects");
      setProjects(projs);
    } catch {
      /* ignore */
    }
  }

  async function deleteProject(id: string) {
    try {
      await invoke("quantum_delete_project", { projectId: id });
      const projs = await invoke<QuantumProject[]>("quantum_get_projects");
      setProjects(projs);
    } catch {
      /* ignore */
    }
  }

  // ── Circuit Builder helpers ──────────────────────────────────────────────

  async function loadCircuitDetail(index: number) {
    try {
      const detail = await invoke<CircuitDetail>("quantum_get_circuit_detail", { index });
      setCircuitDetail(detail);
      setSelectedCircuitIdx(index);
    } catch {
      /* ignore */
    }
  }

  async function createNewCircuit() {
    if (!cbNewName.trim()) return;
    try {
      await invoke("quantum_create_circuit", {
        name: cbNewName,
        numQubits: cbNewQubits,
        numClassical: cbNewClassical,
      });
      setCbNewName("");
      const circs = await invoke<QuantumCircuit[]>("quantum_get_circuits");
      setCircuits(circs);
      if (circs.length > 0) {
        await loadCircuitDetail(circs[circs.length - 1].index);
      }
    } catch {
      /* ignore */
    }
  }

  async function addGate(gate: GateInstance) {
    if (selectedCircuitIdx === null) return;
    try {
      const updated = await invoke<CircuitDetail>("quantum_add_gate", {
        index: selectedCircuitIdx,
        gate,
      });
      setCircuitDetail(updated);
      const circs = await invoke<QuantumCircuit[]>("quantum_get_circuits");
      setCircuits(circs);
    } catch {
      /* ignore */
    }
  }

  async function removeGate(gateIndex: number) {
    if (selectedCircuitIdx === null) return;
    try {
      const updated = await invoke<CircuitDetail>("quantum_remove_gate", {
        index: selectedCircuitIdx,
        gateIndex,
      });
      setCircuitDetail(updated);
      const circs = await invoke<QuantumCircuit[]>("quantum_get_circuits");
      setCircuits(circs);
    } catch {
      /* ignore */
    }
  }

  async function deleteCircuit(index: number) {
    try {
      await invoke("quantum_delete_circuit", { index });
      const circs = await invoke<QuantumCircuit[]>("quantum_get_circuits");
      setCircuits(circs);
      if (selectedCircuitIdx === index) {
        setSelectedCircuitIdx(null);
        setCircuitDetail(null);
      }
    } catch {
      /* ignore */
    }
  }

  async function exportCircuit(index: number, format: string) {
    try {
      const code = await invoke<string>("quantum_export_circuit", { index, format });
      setHelloCode(code);
      setSelectedLang(`Circuit export (${format})`);
      setTab("languages");
    } catch {
      /* ignore */
    }
  }

  const handleSvgClick = useCallback(
    (qubit: number) => {
      if (!selectedGate || !circuitDetail) return;

      if (SINGLE_QUBIT_GATES.includes(selectedGate) || selectedGate === "Measure") {
        const gate: GateInstance = { type: selectedGate, target: qubit };
        if (selectedGate === "Measure") {
          gate.qubit = qubit;
          gate.classical = qubit;
        }
        addGate(gate);
        return;
      }

      if (ROTATION_GATES.includes(selectedGate)) {
        const angleStr = prompt(`Enter angle (radians) for ${selectedGate}:`, String(Math.PI / 4));
        if (angleStr === null) return;
        const angle = parseFloat(angleStr);
        if (isNaN(angle)) return;
        addGate({ type: selectedGate, target: qubit, angle });
        return;
      }

      if (selectedGate === "CNOT" || selectedGate === "CZ") {
        if (placingControl === null) {
          setPlacingControl(qubit);
        } else {
          if (qubit !== placingControl) {
            addGate({ type: selectedGate, control: placingControl, target: qubit });
          }
          setPlacingControl(null);
        }
        return;
      }

      if (selectedGate === "SWAP") {
        if (placingControl === null) {
          setPlacingControl(qubit);
        } else {
          if (qubit !== placingControl) {
            addGate({ type: selectedGate, targets: [placingControl, qubit] });
          }
          setPlacingControl(null);
        }
        return;
      }

      if (selectedGate === "Toffoli") {
        if (placingControls.length < 2) {
          const next = [...placingControls, qubit];
          if (next.length < 2) {
            setPlacingControls(next);
          } else {
            setPlacingControls(next);
          }
          if (next.length === 2) {
            // waiting for target on next click
          }
        } else {
          if (!placingControls.includes(qubit)) {
            addGate({ type: "Toffoli", controls: placingControls, target: qubit });
          }
          setPlacingControls([]);
        }
        return;
      }
    },
    [selectedGate, circuitDetail, placingControl, placingControls, selectedCircuitIdx],
  );

  // ── Simulator helpers ────────────────────────────────────────────────────

  async function runSimulation() {
    if (simCircuitIdx === null) return;
    setSimRunning(true);
    setSimResult(null);
    try {
      const result = await invoke<SimulationResult>("quantum_simulate_circuit", {
        index: simCircuitIdx,
        shots: simShots,
      });
      setSimResult(result);
    } catch {
      /* ignore */
    }
    setSimRunning(false);
  }

  // ── Optimizer helpers ────────────────────────────────────────────────────

  async function runOptimizer() {
    if (optCircuitIdx === null) return;
    setOptRunning(true);
    setOptResult(null);
    try {
      const result = await invoke<OptimizationResult>("quantum_optimize_circuit", {
        index: optCircuitIdx,
      });
      setOptResult(result);
    } catch {
      /* ignore */
    }
    setOptRunning(false);
  }

  // ── Cost helpers ─────────────────────────────────────────────────────────

  async function runCostEstimate() {
    if (costCircuitIdx === null) return;
    setCostRunning(true);
    setCostEstimates([]);
    try {
      const result = await invoke<CostEstimate[]>("quantum_estimate_cost", {
        index: costCircuitIdx,
        shots: costShots,
      });
      setCostEstimates(result);
    } catch {
      /* ignore */
    }
    setCostRunning(false);
  }

  // ── Template helpers ─────────────────────────────────────────────────────

  async function loadTemplate(name: string) {
    setTplLoading(name);
    const needsSecret = name.toLowerCase().includes("bernstein-vazirani") || name.toLowerCase().includes("bv");
    const needsLayers = name.toLowerCase().includes("vqe");
    const needsGammaBeta = name.toLowerCase().includes("qaoa");
    const params: Record<string, string | number> = { qubits: tplQubits };
    if (needsSecret) params.secret = tplSecret;
    if (needsLayers) params.layers = tplLayers;
    if (needsGammaBeta) {
      params.gamma = tplGamma;
      params.beta = tplBeta;
    }
    try {
      const detail = await invoke<CircuitDetail>("quantum_get_algorithm_template", {
        name,
        params,
      });
      setCircuitDetail(detail);
      setSelectedCircuitIdx(detail.index);
      const circs = await invoke<QuantumCircuit[]>("quantum_get_circuits");
      setCircuits(circs);
      setTab("circuitBuilder");
    } catch {
      /* ignore */
    }
    setTplLoading(null);
  }

  // ── Scaffold helpers ─────────────────────────────────────────────────────

  async function runScaffold() {
    if (!scafName.trim()) return;
    setScafRunning(true);
    setScafFiles([]);
    try {
      const files = await invoke<ScaffoldFile[]>("quantum_scaffold_project", {
        language: scafLang,
        name: scafName,
        numQubits: scafQubits,
      });
      setScafFiles(files);
    } catch {
      /* ignore */
    }
    setScafRunning(false);
  }

  function toggleScafExpand(path: string) {
    setScafExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }

  // ── Circuit SVG Renderer ─────────────────────────────────────────────────

  function renderCircuitSvg() {
    if (!circuitDetail) return null;
    const { numQubits } = circuitDetail;
    const gates = circuitDetail.gates ?? [];
    const numCols = Math.max(gates.length + 1, 4);
    const svgW = 80 + numCols * COL_WIDTH;
    const svgH = WIRE_Y0 + numQubits * WIRE_SPACING + 20;

    function gateQubit(g: GateInstance): number {
      if (g.target !== undefined) return g.target;
      if (g.qubit !== undefined) return g.qubit;
      if (g.targets && g.targets.length > 0) return g.targets[0];
      return 0;
    }

    return (
      <svg
        width="100%"
        height={svgH}
        viewBox={`0 0 ${svgW} ${svgH}`}
        style={{ background: "var(--bg-tertiary)", borderRadius: 8, border: "1px solid var(--border-color)", cursor: selectedGate ? "crosshair" : "default" }}
      >
        {/* Qubit labels */}
        {Array.from({ length: numQubits }, (_, q) => (
          <text
            key={`ql-${q}`}
            x={12}
            y={WIRE_Y0 + q * WIRE_SPACING + 5}
            fill="var(--text-secondary)"
            fontSize={12}
            fontFamily="monospace"
          >
            |q{q}&#x27E9;
          </text>
        ))}

        {/* Wire lines */}
        {Array.from({ length: numQubits }, (_, q) => (
          <line
            key={`wl-${q}`}
            x1={60}
            y1={WIRE_Y0 + q * WIRE_SPACING}
            x2={svgW - 10}
            y2={WIRE_Y0 + q * WIRE_SPACING}
            stroke="var(--text-tertiary)"
            strokeWidth={1}
            opacity={0.5}
          />
        ))}

        {/* Clickable wire zones */}
        {selectedGate &&
          Array.from({ length: numQubits }, (_, q) => (
            <rect
              key={`cz-${q}`}
              x={60 + gates.length * COL_WIDTH}
              y={WIRE_Y0 + q * WIRE_SPACING - WIRE_SPACING / 2}
              width={COL_WIDTH}
              height={WIRE_SPACING}
              fill="transparent"
              style={{ cursor: "crosshair" }}
              onClick={() => handleSvgClick(q)}
            />
          ))}

        {/* Gate rendering */}
        {gates.map((g, gi) => {
          const col = gi;
          const cx = 80 + col * COL_WIDTH;
          const color = GATE_COLORS[g.type] || "#666";

          // CNOT / CZ
          if ((g.type === "CNOT" || g.type === "CZ") && g.control !== undefined && g.target !== undefined) {
            const cy1 = WIRE_Y0 + g.control * WIRE_SPACING;
            const cy2 = WIRE_Y0 + g.target * WIRE_SPACING;
            return (
              <g key={`g-${gi}`} style={{ cursor: "pointer" }} onClick={() => removeGate(gi)}>
                <title>Click to remove {g.type}</title>
                <line x1={cx} y1={cy1} x2={cx} y2={cy2} stroke={color} strokeWidth={2} />
                <circle cx={cx} cy={cy1} r={5} fill={color} />
                {g.type === "CNOT" ? (
                  <>
                    <circle cx={cx} cy={cy2} r={10} fill="none" stroke={color} strokeWidth={2} />
                    <line x1={cx - 7} y1={cy2} x2={cx + 7} y2={cy2} stroke={color} strokeWidth={2} />
                    <line x1={cx} y1={cy2 - 7} x2={cx} y2={cy2 + 7} stroke={color} strokeWidth={2} />
                  </>
                ) : (
                  <circle cx={cx} cy={cy2} r={5} fill={color} />
                )}
              </g>
            );
          }

          // SWAP
          if (g.type === "SWAP" && g.targets && g.targets.length === 2) {
            const cy1 = WIRE_Y0 + g.targets[0] * WIRE_SPACING;
            const cy2 = WIRE_Y0 + g.targets[1] * WIRE_SPACING;
            return (
              <g key={`g-${gi}`} style={{ cursor: "pointer" }} onClick={() => removeGate(gi)}>
                <title>Click to remove SWAP</title>
                <line x1={cx} y1={cy1} x2={cx} y2={cy2} stroke={color} strokeWidth={2} />
                <text x={cx} y={cy1 + 5} textAnchor="middle" fill={color} fontSize={16} fontWeight="bold">&#xd7;</text>
                <text x={cx} y={cy2 + 5} textAnchor="middle" fill={color} fontSize={16} fontWeight="bold">&#xd7;</text>
              </g>
            );
          }

          // Toffoli
          if (g.type === "Toffoli" && g.controls && g.target !== undefined) {
            const tgtY = WIRE_Y0 + g.target * WIRE_SPACING;
            const allQubits = [...g.controls, g.target];
            const minY = Math.min(...allQubits.map((q) => WIRE_Y0 + q * WIRE_SPACING));
            const maxY = Math.max(...allQubits.map((q) => WIRE_Y0 + q * WIRE_SPACING));
            return (
              <g key={`g-${gi}`} style={{ cursor: "pointer" }} onClick={() => removeGate(gi)}>
                <title>Click to remove Toffoli</title>
                <line x1={cx} y1={minY} x2={cx} y2={maxY} stroke={color} strokeWidth={2} />
                {g.controls.map((c, ci) => (
                  <circle key={ci} cx={cx} cy={WIRE_Y0 + c * WIRE_SPACING} r={5} fill={color} />
                ))}
                <circle cx={cx} cy={tgtY} r={10} fill="none" stroke={color} strokeWidth={2} />
                <line x1={cx - 7} y1={tgtY} x2={cx + 7} y2={tgtY} stroke={color} strokeWidth={2} />
                <line x1={cx} y1={tgtY - 7} x2={cx} y2={tgtY + 7} stroke={color} strokeWidth={2} />
              </g>
            );
          }

          // Measure
          if (g.type === "Measure") {
            const qb = gateQubit(g);
            const cy = WIRE_Y0 + qb * WIRE_SPACING;
            const half = GATE_SIZE / 2;
            return (
              <g key={`g-${gi}`} style={{ cursor: "pointer" }} onClick={() => removeGate(gi)}>
                <title>Click to remove Measure</title>
                <rect x={cx - half} y={cy - half} width={GATE_SIZE} height={GATE_SIZE} rx={4} fill={color} />
                <text x={cx} y={cy + 5} textAnchor="middle" fill="#fff" fontSize={14} fontWeight="bold">
                  M
                </text>
              </g>
            );
          }

          // Single qubit / rotation gates
          const qb = gateQubit(g);
          const cy = WIRE_Y0 + qb * WIRE_SPACING;
          const half = GATE_SIZE / 2;
          return (
            <g key={`g-${gi}`} style={{ cursor: "pointer" }} onClick={() => removeGate(gi)}>
              <title>Click to remove {g.type}{g.angle !== undefined ? ` (${g.angle.toFixed(2)})` : ""}</title>
              <rect x={cx - half} y={cy - half} width={GATE_SIZE} height={GATE_SIZE} rx={4} fill={color} />
              <text x={cx} y={cy + 5} textAnchor="middle" fill="#fff" fontSize={11} fontWeight="bold">
                {g.type}
              </text>
              {g.angle !== undefined && (
                <text x={cx} y={cy + half + 12} textAnchor="middle" fill="var(--text-tertiary)" fontSize={9}>
                  {g.angle.toFixed(2)}
                </text>
              )}
            </g>
          );
        })}

        {/* Placement hint column */}
        {selectedGate && (
          <rect
            x={80 + gates.length * COL_WIDTH - COL_WIDTH / 2}
            y={WIRE_Y0 - WIRE_SPACING / 2}
            width={COL_WIDTH}
            height={numQubits * WIRE_SPACING}
            fill="var(--accent-primary)"
            opacity={0.06}
            rx={4}
          />
        )}
      </svg>
    );
  }

  // ── Bloch Sphere Renderer ────────────────────────────────────────────────

  function renderBlochSphere(amplitudes: [string, number, number][]) {
    if (amplitudes.length !== 2) return null;
    const [, re0, im0] = amplitudes[0];
    const [, re1, im1] = amplitudes[1];
    const { theta, phi } = amplitudesToBloch(re0, im0, re1, im1);

    const R = 80;
    const svgCx = 100;
    const svgCy = 100;

    const sx = R * Math.sin(theta) * Math.cos(phi);
    const sy = R * Math.sin(theta) * Math.sin(phi);
    const sz = R * Math.cos(theta);
    const px = svgCx + sx - sy * 0.3;
    const py = svgCy - sz + sy * 0.3;

    return (
      <div style={{ ...cardStyle, marginTop: 12 }}>
        <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Bloch Sphere</div>
        <svg width={200} height={200} viewBox="0 0 200 200">
          {/* Main circle */}
          <circle cx={svgCx} cy={svgCy} r={R} fill="none" stroke="var(--text-tertiary)" strokeWidth={1} opacity={0.5} />
          {/* Equator ellipse (dashed) */}
          <ellipse cx={svgCx} cy={svgCy} rx={R} ry={R * 0.3} fill="none" stroke="var(--text-tertiary)" strokeWidth={1} strokeDasharray="4 3" opacity={0.4} />
          {/* Vertical axis */}
          <line x1={svgCx} y1={svgCy - R - 5} x2={svgCx} y2={svgCy + R + 5} stroke="var(--text-tertiary)" strokeWidth={0.5} opacity={0.3} />
          {/* Horizontal axis */}
          <line x1={svgCx - R - 5} y1={svgCy} x2={svgCx + R + 5} y2={svgCy} stroke="var(--text-tertiary)" strokeWidth={0.5} opacity={0.3} />

          {/* Axis labels */}
          <text x={svgCx} y={svgCy - R - 10} textAnchor="middle" fill="var(--text-secondary)" fontSize={11}>|0&#x27E9;</text>
          <text x={svgCx} y={svgCy + R + 16} textAnchor="middle" fill="var(--text-secondary)" fontSize={11}>|1&#x27E9;</text>
          <text x={svgCx + R + 10} y={svgCy + 4} textAnchor="start" fill="var(--text-secondary)" fontSize={11}>|+&#x27E9;</text>
          <text x={svgCx - R - 10} y={svgCy + 4} textAnchor="end" fill="var(--text-secondary)" fontSize={11}>|-&#x27E9;</text>

          {/* State arrow */}
          <line x1={svgCx} y1={svgCy} x2={px} y2={py} stroke="var(--accent-primary)" strokeWidth={2} />
          <circle cx={px} cy={py} r={4} fill="var(--accent-primary)" />
        </svg>
        <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>
          theta = {theta.toFixed(3)} rad, phi = {phi.toFixed(3)} rad
        </div>
      </div>
    );
  }

  // ── Probability Bar Chart ────────────────────────────────────────────────

  function renderProbabilityChart(probs: [string, number][]) {
    const nonZero = probs.filter(([, p]) => p > 0.001);
    if (nonZero.length === 0) return <div style={{ color: "var(--text-secondary)", fontSize: 12 }}>No non-zero probabilities.</div>;
    const maxP = Math.max(...nonZero.map(([, p]) => p));

    return (
      <div style={{ ...cardStyle, marginTop: 12 }}>
        <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Probability Distribution</div>
        {nonZero.map(([label, prob]) => (
          <div key={label} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
            <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", width: 60, textAlign: "right", color: "var(--text-secondary)" }}>
              |{label}&#x27E9;
            </span>
            <div style={{ flex: 1, height: 16, background: "var(--bg-tertiary)", borderRadius: 3, overflow: "hidden" }}>
              <div
                style={{
                  width: `${(prob / maxP) * 100}%`,
                  height: "100%",
                  background: "var(--accent-primary)",
                  borderRadius: 3,
                  minWidth: 2,
                }}
              />
            </div>
            <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", width: 55, color: "var(--text-primary)" }}>
              {(prob * 100).toFixed(1)}%
            </span>
          </div>
        ))}
      </div>
    );
  }

  // ── Sample Histogram ─────────────────────────────────────────────────────

  function renderSampleHistogram(samples: Record<string, number>) {
    const entries = Object.entries(samples).sort((a, b) => b[1] - a[1]).slice(0, 16);
    if (entries.length === 0) return null;
    const maxCount = Math.max(...entries.map(([, c]) => c));

    return (
      <div style={{ ...cardStyle, marginTop: 12 }}>
        <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Sample Histogram (top {entries.length})</div>
        {entries.map(([label, count]) => (
          <div key={label} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 3 }}>
            <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", width: 60, textAlign: "right", color: "var(--text-secondary)" }}>
              {label}
            </span>
            <div style={{ flex: 1, height: 14, background: "var(--bg-tertiary)", borderRadius: 3, overflow: "hidden" }}>
              <div
                style={{
                  width: `${(count / maxCount) * 100}%`,
                  height: "100%",
                  background: "var(--success-color)",
                  borderRadius: 3,
                  minWidth: 2,
                }}
              />
            </div>
            <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", width: 45, color: "var(--text-primary)" }}>
              {count}
            </span>
          </div>
        ))}
      </div>
    );
  }

  // ── Topology SVG Renderer ────────────────────────────────────────────────

  function renderTopologySvg(topo: HardwareTopology) {
    const { qubits, couplings } = topo;
    if (qubits.length === 0) return null;

    const degreeMap: Record<number, number> = {};
    couplings.forEach(([a, b]) => {
      degreeMap[a] = (degreeMap[a] || 0) + 1;
      degreeMap[b] = (degreeMap[b] || 0) + 1;
    });
    const maxDeg = Math.max(1, ...Object.values(degreeMap));

    const xs = qubits.map((q) => q.x);
    const ys = qubits.map((q) => q.y);
    const minX = Math.min(...xs) - 20;
    const minY = Math.min(...ys) - 20;
    const maxX = Math.max(...xs) + 20;
    const maxY = Math.max(...ys) + 20;
    const vw = maxX - minX;
    const vh = maxY - minY;

    const qMap: Record<number, { x: number; y: number }> = {};
    qubits.forEach((q) => {
      qMap[q.id] = q;
    });

    function degreeColor(deg: number): string {
      const t = deg / maxDeg;
      const r = Math.round(60 + t * 140);
      const g = Math.round(140 - t * 60);
      const b = Math.round(220 - t * 100);
      return `rgb(${r},${g},${b})`;
    }

    return (
      <svg
        width="100%"
        height={Math.min(vh + 40, 500)}
        viewBox={`${minX} ${minY} ${vw} ${vh}`}
        style={{ background: "var(--bg-tertiary)", borderRadius: 8, border: "1px solid var(--border-color)" }}
      >
        {/* Coupling edges */}
        {couplings.map(([a, b], i) => {
          const qa = qMap[a];
          const qb = qMap[b];
          if (!qa || !qb) return null;
          return (
            <line
              key={`e-${i}`}
              x1={qa.x}
              y1={qa.y}
              x2={qb.x}
              y2={qb.y}
              stroke="var(--text-tertiary)"
              strokeWidth={0.8}
              opacity={0.3}
            />
          );
        })}
        {/* Qubit nodes */}
        {qubits.map((q) => (
          <circle
            key={`n-${q.id}`}
            cx={q.x}
            cy={q.y}
            r={6}
            fill={degreeColor(degreeMap[q.id] || 0)}
            stroke="var(--bg-primary)"
            strokeWidth={1}
          >
            <title>q{q.id} (degree: {degreeMap[q.id] || 0})</title>
          </circle>
        ))}
      </svg>
    );
  }

  // ── Circuit Select Dropdown ──────────────────────────────────────────────

  function CircuitSelect({
    value,
    onChange,
    label,
  }: {
    value: number | null;
    onChange: (v: number | null) => void;
    label?: string;
  }) {
    return (
      <label style={{ fontSize: 12 }}>
        {label || "Circuit"}
        <br />
        <select
          value={value ?? ""}
          onChange={(e) => onChange(e.target.value ? Number(e.target.value) : null)}
          style={{ ...inputStyle, minWidth: 160 }}
        >
          <option value="">-- Select circuit --</option>
          {circuits.map((c) => (
            <option key={c.index} value={c.index}>
              {c.name} ({c.numQubits}q, {c.gateCount}g)
            </option>
          ))}
        </select>
      </label>
    );
  }

  // ── Render ──────────────────────────────────────────────────────────────

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", color: "var(--text-primary)", background: "var(--bg-primary)" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", gap: 2, padding: "8px 12px", borderBottom: "1px solid var(--border-color)", flexWrap: "wrap" }}>
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            style={{
              padding: "6px 14px",
              borderRadius: 6,
              border: "none",
              cursor: "pointer",
              background: tab === t.id ? "var(--accent-primary)" : "var(--bg-secondary)",
              color: tab === t.id ? "#fff" : "var(--text-secondary)",
              fontWeight: tab === t.id ? 600 : 400,
              fontSize: 13,
              borderBottom: tab === t.id ? "2px solid var(--accent-primary)" : "2px solid transparent",
            }}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {/* ── Circuit Builder Tab ─────────────────────────────────────── */}
        {tab === "circuitBuilder" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Circuit Builder</h3>

            {/* Circuit selector + create */}
            <div style={{ display: "flex", gap: 12, alignItems: "flex-end", marginBottom: 12, flexWrap: "wrap" }}>
              <label style={{ fontSize: 12 }}>
                Circuit
                <br />
                <select
                  value={selectedCircuitIdx ?? ""}
                  onChange={(e) => {
                    const v = e.target.value ? Number(e.target.value) : null;
                    if (v !== null) loadCircuitDetail(v);
                    else {
                      setSelectedCircuitIdx(null);
                      setCircuitDetail(null);
                    }
                  }}
                  style={{ ...inputStyle, minWidth: 180 }}
                >
                  <option value="">-- Select circuit --</option>
                  {circuits.map((c) => (
                    <option key={c.index} value={c.index}>
                      {c.name} ({c.numQubits}q)
                    </option>
                  ))}
                </select>
              </label>
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>or</div>
              <label style={{ fontSize: 12 }}>
                Name
                <br />
                <input value={cbNewName} onChange={(e) => setCbNewName(e.target.value)} style={{ ...inputStyle, width: 120 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Qubits
                <br />
                <input type="number" value={cbNewQubits} onChange={(e) => setCbNewQubits(+e.target.value)} min={1} max={20} style={{ ...inputStyle, width: 50 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Classical
                <br />
                <input type="number" value={cbNewClassical} onChange={(e) => setCbNewClassical(+e.target.value)} min={0} max={20} style={{ ...inputStyle, width: 50 }} />
              </label>
              <button onClick={createNewCircuit} style={btnPrimary}>
                New Circuit
              </button>
              {selectedCircuitIdx !== null && (
                <>
                  <button onClick={() => exportCircuit(selectedCircuitIdx, "qasm3")} style={btnSmall}>Export QASM3</button>
                  <button onClick={() => exportCircuit(selectedCircuitIdx, "qiskit")} style={btnSmall}>Export Qiskit</button>
                  <button onClick={() => exportCircuit(selectedCircuitIdx, "cirq")} style={btnSmall}>Export Cirq</button>
                  <button onClick={() => deleteCircuit(selectedCircuitIdx)} style={{ ...btnSmall, color: "var(--error-color)" }}>Delete</button>
                </>
              )}
            </div>

            {circuitDetail && (
              <div style={{ display: "flex", gap: 12 }}>
                {/* Gate palette */}
                <div style={{ width: 100, flexShrink: 0 }}>
                  <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginBottom: 6 }}>SINGLE</div>
                  {SINGLE_QUBIT_GATES.map((g) => (
                    <button
                      key={g}
                      onClick={() => {
                        setSelectedGate(selectedGate === g ? null : g);
                        setPlacingControl(null);
                        setPlacingControls([]);
                      }}
                      style={{
                        display: "block",
                        width: "100%",
                        marginBottom: 3,
                        padding: "4px 8px",
                        borderRadius: 4,
                        border: selectedGate === g ? "2px solid var(--accent-primary)" : "1px solid var(--border-color)",
                        background: selectedGate === g ? "var(--accent-primary-10)" : "var(--bg-secondary)",
                        color: GATE_COLORS[g] || "var(--text-primary)",
                        cursor: "pointer",
                        fontSize: 12,
                        fontWeight: 600,
                        textAlign: "left",
                      }}
                    >
                      {g}
                    </button>
                  ))}
                  <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginTop: 8, marginBottom: 6 }}>ROTATION</div>
                  {ROTATION_GATES.map((g) => (
                    <button
                      key={g}
                      onClick={() => {
                        setSelectedGate(selectedGate === g ? null : g);
                        setPlacingControl(null);
                        setPlacingControls([]);
                      }}
                      style={{
                        display: "block",
                        width: "100%",
                        marginBottom: 3,
                        padding: "4px 8px",
                        borderRadius: 4,
                        border: selectedGate === g ? "2px solid var(--accent-primary)" : "1px solid var(--border-color)",
                        background: selectedGate === g ? "var(--accent-primary-10)" : "var(--bg-secondary)",
                        color: GATE_COLORS[g] || "var(--text-primary)",
                        cursor: "pointer",
                        fontSize: 12,
                        fontWeight: 600,
                        textAlign: "left",
                      }}
                    >
                      {g}
                    </button>
                  ))}
                  <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginTop: 8, marginBottom: 6 }}>MULTI-QUBIT</div>
                  {MULTI_QUBIT_GATES.map((g) => (
                    <button
                      key={g}
                      onClick={() => {
                        setSelectedGate(selectedGate === g ? null : g);
                        setPlacingControl(null);
                        setPlacingControls([]);
                      }}
                      style={{
                        display: "block",
                        width: "100%",
                        marginBottom: 3,
                        padding: "4px 8px",
                        borderRadius: 4,
                        border: selectedGate === g ? "2px solid var(--accent-primary)" : "1px solid var(--border-color)",
                        background: selectedGate === g ? "var(--accent-primary-10)" : "var(--bg-secondary)",
                        color: GATE_COLORS[g] || "var(--text-primary)",
                        cursor: "pointer",
                        fontSize: 12,
                        fontWeight: 600,
                        textAlign: "left",
                      }}
                    >
                      {g}
                    </button>
                  ))}
                  <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", marginTop: 8, marginBottom: 6 }}>MEASURE</div>
                  {MEASUREMENT_GATES.map((g) => (
                    <button
                      key={g}
                      onClick={() => {
                        setSelectedGate(selectedGate === g ? null : g);
                        setPlacingControl(null);
                        setPlacingControls([]);
                      }}
                      style={{
                        display: "block",
                        width: "100%",
                        marginBottom: 3,
                        padding: "4px 8px",
                        borderRadius: 4,
                        border: selectedGate === g ? "2px solid var(--accent-primary)" : "1px solid var(--border-color)",
                        background: selectedGate === g ? "var(--accent-primary-10)" : "var(--bg-secondary)",
                        color: GATE_COLORS[g] || "var(--text-primary)",
                        cursor: "pointer",
                        fontSize: 12,
                        fontWeight: 600,
                        textAlign: "left",
                      }}
                    >
                      {g}
                    </button>
                  ))}
                </div>

                {/* SVG canvas */}
                <div style={{ flex: 1, minWidth: 0 }}>
                  {/* Placement hint */}
                  {selectedGate && (
                    <div
                      style={{
                        padding: "4px 10px",
                        marginBottom: 6,
                        borderRadius: 4,
                        background: "var(--accent-primary-10)",
                        border: "1px solid var(--accent-primary)",
                        fontSize: 12,
                        color: "var(--text-primary)",
                      }}
                    >
                      {placingControl !== null
                        ? `Select target qubit for ${selectedGate} (control: q${placingControl})...`
                        : placingControls.length > 0
                          ? `Select ${placingControls.length < 2 ? "second control" : "target"} qubit for Toffoli (controls: ${placingControls.map((c) => `q${c}`).join(", ")})...`
                          : `Click a qubit wire to place ${selectedGate}`}
                    </div>
                  )}

                  <div style={{ overflowX: "auto" }}>{renderCircuitSvg()}</div>

                  {/* Metrics bar */}
                  <div
                    style={{
                      marginTop: 8,
                      padding: "6px 12px",
                      borderRadius: 6,
                      background: "var(--bg-secondary)",
                      border: "1px solid var(--border-color)",
                      fontSize: 12,
                      color: "var(--text-secondary)",
                      display: "flex",
                      gap: 16,
                      flexWrap: "wrap",
                    }}
                  >
                    <span>Gates: <strong style={{ color: "var(--text-primary)" }}>{circuitDetail.gateCount}</strong></span>
                    <span>Depth: <strong style={{ color: "var(--text-primary)" }}>{circuitDetail.depth}</strong></span>
                    <span>2Q: <strong style={{ color: "var(--text-primary)" }}>{circuitDetail.twoQubitGates}</strong></span>
                    <span>Volume: <strong style={{ color: "var(--text-primary)" }}>{circuitDetail.depth * circuitDetail.numQubits}</strong></span>
                    <span>Qubits: <strong style={{ color: "var(--text-primary)" }}>{circuitDetail.numQubits}</strong></span>
                  </div>
                </div>
              </div>
            )}

            {!circuitDetail && (
              <div style={{ color: "var(--text-secondary)", fontSize: 13, marginTop: 20 }}>
                Select an existing circuit or create a new one to start building.
              </div>
            )}
          </div>
        )}

        {/* ── Simulator Tab ───────────────────────────────────────────── */}
        {tab === "simulator" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Quantum Circuit Simulator</h3>
            <div style={{ display: "flex", gap: 12, alignItems: "flex-end", marginBottom: 16, flexWrap: "wrap" }}>
              <CircuitSelect value={simCircuitIdx} onChange={setSimCircuitIdx} />
              <label style={{ fontSize: 12 }}>
                Shots
                <br />
                <input
                  type="number"
                  value={simShots}
                  onChange={(e) => setSimShots(+e.target.value)}
                  min={1}
                  max={100000}
                  style={{ ...inputStyle, width: 80 }}
                />
              </label>
              <button onClick={runSimulation} disabled={simCircuitIdx === null || simRunning} style={{ ...btnPrimary, opacity: simCircuitIdx === null || simRunning ? 0.5 : 1 }}>
                {simRunning ? "Simulating..." : "Simulate"}
              </button>
            </div>

            {simResult && (
              <div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
                  {simResult.num_qubits} qubit(s) | {simResult.probabilities.length} basis states | {Object.values(simResult.samples).reduce((a, b) => a + b, 0)} samples
                </div>

                {renderProbabilityChart(simResult.probabilities)}

                {simResult.num_qubits === 1 && simResult.amplitudes.length === 2 && renderBlochSphere(simResult.amplitudes)}

                {renderSampleHistogram(simResult.samples)}

                {/* Amplitudes table */}
                <div style={{ ...cardStyle, marginTop: 12 }}>
                  <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>State Amplitudes</div>
                  <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
                    <thead>
                      <tr style={{ borderBottom: "2px solid var(--border-color)" }}>
                        <th style={{ textAlign: "left", padding: 4 }}>State</th>
                        <th style={{ textAlign: "right", padding: 4 }}>Real</th>
                        <th style={{ textAlign: "right", padding: 4 }}>Imag</th>
                        <th style={{ textAlign: "right", padding: 4 }}>Prob</th>
                      </tr>
                    </thead>
                    <tbody>
                      {simResult.amplitudes.map(([label, re, im]) => {
                        const prob = re * re + im * im;
                        return (
                          <tr key={label} style={{ borderBottom: "1px solid var(--border-color)" }}>
                            <td style={{ padding: 4, fontFamily: "var(--font-mono)" }}>|{label}&#x27E9;</td>
                            <td style={{ padding: 4, textAlign: "right", fontFamily: "var(--font-mono)" }}>{re.toFixed(4)}</td>
                            <td style={{ padding: 4, textAlign: "right", fontFamily: "var(--font-mono)" }}>{im.toFixed(4)}</td>
                            <td style={{ padding: 4, textAlign: "right", fontFamily: "var(--font-mono)" }}>{(prob * 100).toFixed(2)}%</td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </div>
        )}

        {/* ── Optimizer Tab ───────────────────────────────────────────── */}
        {tab === "optimizer" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Circuit Optimizer</h3>
            <div style={{ display: "flex", gap: 12, alignItems: "flex-end", marginBottom: 16, flexWrap: "wrap" }}>
              <CircuitSelect value={optCircuitIdx} onChange={setOptCircuitIdx} />
              <button onClick={runOptimizer} disabled={optCircuitIdx === null || optRunning} style={{ ...btnPrimary, opacity: optCircuitIdx === null || optRunning ? 0.5 : 1 }}>
                {optRunning ? "Optimizing..." : "Optimize"}
              </button>
            </div>

            {optResult && (
              <div>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 16 }}>
                  {/* Original card */}
                  <div style={cardStyle}>
                    <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8, color: "var(--text-secondary)" }}>Original</div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                      <div style={{ fontSize: 12 }}>
                        Gates: <strong>{optResult.original_gate_count}</strong>
                      </div>
                      <div style={{ fontSize: 12 }}>
                        Depth: <strong>{optResult.original_depth}</strong>
                      </div>
                    </div>
                  </div>
                  {/* Optimized card */}
                  <div style={{ ...cardStyle, borderColor: "var(--accent-primary)" }}>
                    <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8, color: "var(--accent-primary)" }}>
                      Optimized ({optResult.savings_percent.toFixed(1)}% savings)
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                      <div style={{ fontSize: 12 }}>
                        Gates: <strong>{optResult.optimized_gate_count}</strong>
                      </div>
                      <div style={{ fontSize: 12 }}>
                        Depth: <strong>{optResult.optimized_depth}</strong>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Rules applied */}
                {optResult.rules_applied.length > 0 && (
                  <div style={cardStyle}>
                    <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Optimization Rules Applied</div>
                    <ul style={{ margin: 0, paddingLeft: 20, fontSize: 12, color: "var(--text-secondary)" }}>
                      {optResult.rules_applied.map((r, i) => (
                        <li key={i} style={{ marginBottom: 3 }}>
                          {r}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* ── Cost Tab ────────────────────────────────────────────────── */}
        {tab === "cost" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Provider Cost Estimation</h3>
            <div style={{ display: "flex", gap: 12, alignItems: "flex-end", marginBottom: 16, flexWrap: "wrap" }}>
              <CircuitSelect value={costCircuitIdx} onChange={setCostCircuitIdx} />
              <label style={{ fontSize: 12 }}>
                Shots
                <br />
                <input
                  type="number"
                  value={costShots}
                  onChange={(e) => setCostShots(+e.target.value)}
                  min={1}
                  max={1000000}
                  style={{ ...inputStyle, width: 80 }}
                />
              </label>
              <button onClick={runCostEstimate} disabled={costCircuitIdx === null || costRunning} style={{ ...btnPrimary, opacity: costCircuitIdx === null || costRunning ? 0.5 : 1 }}>
                {costRunning ? "Estimating..." : "Estimate Cost"}
              </button>
            </div>

            {costEstimates.length > 0 && (
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                <thead>
                  <tr style={{ borderBottom: "2px solid var(--border-color)" }}>
                    <th style={{ textAlign: "left", padding: 6 }}>Provider</th>
                    <th style={{ textAlign: "right", padding: 6 }}>Estimated Cost ($)</th>
                    <th style={{ textAlign: "left", padding: 6 }}>Breakdown</th>
                    <th style={{ textAlign: "left", padding: 6 }}>Notes</th>
                  </tr>
                </thead>
                <tbody>
                  {costEstimates.map((est) => (
                    <tr key={est.provider} style={{ borderBottom: "1px solid var(--border-color)" }}>
                      <td style={{ padding: 6, fontWeight: 500 }}>{est.provider}</td>
                      <td style={{ padding: 6, textAlign: "right", fontFamily: "var(--font-mono)" }}>
                        ${est.estimated_cost_usd.toFixed(4)}
                      </td>
                      <td style={{ padding: 6 }}>
                        {est.breakdown.map(([item, cost], i) => (
                          <div key={i} style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                            {item}: ${cost.toFixed(4)}
                          </div>
                        ))}
                      </td>
                      <td style={{ padding: 6, fontSize: 11, color: "var(--text-secondary)" }}>
                        {est.notes.join("; ")}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        )}

        {/* ── Templates Tab ───────────────────────────────────────────── */}
        {tab === "templates" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Algorithm Templates</h3>

            {/* Parameters section */}
            <div style={{ ...cardStyle, marginBottom: 16 }}>
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Template Parameters</div>
              <div style={{ display: "flex", gap: 12, flexWrap: "wrap", alignItems: "flex-end" }}>
                <label style={{ fontSize: 12 }}>
                  Qubits
                  <br />
                  <input type="number" value={tplQubits} onChange={(e) => setTplQubits(+e.target.value)} min={1} max={20} style={{ ...inputStyle, width: 60 }} />
                </label>
                <label style={{ fontSize: 12 }}>
                  Secret (BV)
                  <br />
                  <input value={tplSecret} onChange={(e) => setTplSecret(e.target.value)} style={{ ...inputStyle, width: 80 }} placeholder="101" />
                </label>
                <label style={{ fontSize: 12 }}>
                  Layers (VQE)
                  <br />
                  <input type="number" value={tplLayers} onChange={(e) => setTplLayers(+e.target.value)} min={1} max={10} style={{ ...inputStyle, width: 60 }} />
                </label>
                <label style={{ fontSize: 12 }}>
                  Gamma (QAOA)
                  <br />
                  <input type="number" value={tplGamma} onChange={(e) => setTplGamma(+e.target.value)} step={0.1} style={{ ...inputStyle, width: 60 }} />
                </label>
                <label style={{ fontSize: 12 }}>
                  Beta (QAOA)
                  <br />
                  <input type="number" value={tplBeta} onChange={(e) => setTplBeta(+e.target.value)} step={0.1} style={{ ...inputStyle, width: 60 }} />
                </label>
              </div>
            </div>

            {/* Template cards */}
            {templateList.length === 0 ? (
              <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading templates...</div>
            ) : (
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 12 }}>
                {templateList.map((tpl) => (
                  <div key={tpl.name} style={cardStyle}>
                    <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 4 }}>{tpl.name}</div>
                    <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>{tpl.description}</div>
                    <div style={{ display: "flex", gap: 6 }}>
                      <button
                        onClick={() => loadTemplate(tpl.name)}
                        disabled={tplLoading === tpl.name}
                        style={{ ...btnPrimary, fontSize: 11, padding: "4px 10px", opacity: tplLoading === tpl.name ? 0.5 : 1 }}
                      >
                        {tplLoading === tpl.name ? "Loading..." : "Load Template"}
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ── Scaffold Tab ────────────────────────────────────────────── */}
        {tab === "scaffold" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Project Scaffolding</h3>
            <div style={{ display: "flex", gap: 12, alignItems: "flex-end", marginBottom: 16, flexWrap: "wrap" }}>
              <label style={{ fontSize: 12 }}>
                Project Name
                <br />
                <input value={scafName} onChange={(e) => setScafName(e.target.value)} style={{ ...inputStyle, width: 180 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Language
                <br />
                <select value={scafLang} onChange={(e) => setScafLang(e.target.value)} style={{ ...inputStyle, minWidth: 120 }}>
                  <option value="Qiskit">Qiskit</option>
                  <option value="Cirq">Cirq</option>
                  <option value="PennyLane">PennyLane</option>
                  <option value="Q#">Q#</option>
                </select>
              </label>
              <label style={{ fontSize: 12 }}>
                Qubits
                <br />
                <input type="number" value={scafQubits} onChange={(e) => setScafQubits(+e.target.value)} min={1} max={100} style={{ ...inputStyle, width: 60 }} />
              </label>
              <button onClick={runScaffold} disabled={scafRunning || !scafName.trim()} style={{ ...btnPrimary, opacity: scafRunning || !scafName.trim() ? 0.5 : 1 }}>
                {scafRunning ? "Generating..." : "Generate"}
              </button>
            </div>

            {scafFiles.length > 0 && (
              <div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
                  Generated {scafFiles.length} file(s)
                </div>
                {scafFiles.map((f) => (
                  <div key={f.path} style={{ ...cardStyle, marginBottom: 8 }}>
                    <div
                      style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }}
                      onClick={() => toggleScafExpand(f.path)}
                    >
                      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, color: "var(--accent-primary)" }}>
                        {scafExpanded.has(f.path) ? "v " : "> "}
                        {f.path}
                      </div>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          navigator.clipboard.writeText(f.content);
                        }}
                        style={btnSmall}
                      >
                        Copy
                      </button>
                    </div>
                    {scafExpanded.has(f.path) && (
                      <pre
                        style={{
                          marginTop: 8,
                          padding: 10,
                          background: "var(--bg-tertiary)",
                          borderRadius: 6,
                          fontSize: 11,
                          overflow: "auto",
                          maxHeight: 300,
                          whiteSpace: "pre-wrap",
                          color: "var(--text-primary)",
                        }}
                      >
                        {f.content}
                      </pre>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ── Topology Tab ────────────────────────────────────────────── */}
        {tab === "topology" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Hardware Topology Viewer</h3>
            <div style={{ display: "flex", gap: 12, alignItems: "flex-end", marginBottom: 16, flexWrap: "wrap" }}>
              <label style={{ fontSize: 12 }}>
                Backend
                <br />
                <select
                  value={selectedTopology}
                  onChange={(e) => setSelectedTopology(Number(e.target.value))}
                  style={{ ...inputStyle, minWidth: 200 }}
                >
                  {TOPOLOGIES.map((t, i) => (
                    <option key={i} value={i}>
                      {t.name} ({t.vendor}, {t.qubitCount}q)
                    </option>
                  ))}
                </select>
              </label>
            </div>

            {(() => {
              const topo = TOPOLOGIES[selectedTopology];
              if (!topo) return null;
              return (
                <div>
                  <div style={{ display: "flex", gap: 16, marginBottom: 12, fontSize: 12, color: "var(--text-secondary)" }}>
                    <span>Vendor: <strong style={{ color: "var(--text-primary)" }}>{topo.vendor}</strong></span>
                    <span>Qubits: <strong style={{ color: "var(--text-primary)" }}>{topo.qubitCount}</strong></span>
                    <span>Couplings: <strong style={{ color: "var(--text-primary)" }}>{topo.couplings.length}</strong></span>
                    <span>
                      Connectivity:{" "}
                      <strong style={{ color: "var(--text-primary)" }}>
                        {topo.couplings.length === (topo.qubitCount * (topo.qubitCount - 1)) / 2
                          ? "All-to-all"
                          : `${((2 * topo.couplings.length) / (topo.qubitCount * (topo.qubitCount - 1)) * 100).toFixed(1)}%`}
                      </strong>
                    </span>
                  </div>
                  {renderTopologySvg(topo)}
                  <div style={{ marginTop: 8, fontSize: 11, color: "var(--text-tertiary)" }}>
                    Node brightness indicates connectivity degree. Hover over a qubit to see its ID and degree.
                  </div>
                </div>
              );
            })()}
          </div>
        )}

        {/* ── Languages Tab ───────────────────────────────────────────── */}
        {tab === "languages" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Quantum Programming Languages (20)</h3>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: 12 }}>
              {languages.map((l) => (
                <div
                  key={l.name}
                  onClick={() => loadHelloCircuit(l.name)}
                  style={{
                    padding: 12,
                    borderRadius: 8,
                    cursor: "pointer",
                    background: selectedLang === l.name ? "var(--accent-primary-10)" : "var(--bg-secondary)",
                    border: selectedLang === l.name ? "1px solid var(--accent-primary)" : "1px solid var(--border-color)",
                  }}
                >
                  <div style={{ fontWeight: 600, fontSize: 14 }}>{l.name}</div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>
                    Host: {l.hostLanguage} &middot; {l.vendor}
                  </div>
                  <code style={{ fontSize: 11, color: "var(--text-tertiary)", display: "block", marginTop: 4, wordBreak: "break-all" }}>
                    {l.installCommand}
                  </code>
                </div>
              ))}
            </div>
            {helloCode && (
              <div style={{ marginTop: 16 }}>
                <h4 style={{ margin: "0 0 8px" }}>{selectedLang} — Example</h4>
                <pre
                  style={{
                    background: "var(--bg-tertiary)",
                    padding: 12,
                    borderRadius: 8,
                    fontSize: 12,
                    overflow: "auto",
                    maxHeight: 400,
                    whiteSpace: "pre-wrap",
                  }}
                >
                  {helloCode}
                </pre>
              </div>
            )}

            {compat.length > 0 && (
              <div style={{ marginTop: 20 }}>
                <h4 style={{ margin: "0 0 8px" }}>Language / OS Compatibility Matrix</h4>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                  <thead>
                    <tr style={{ borderBottom: "2px solid var(--border-color)" }}>
                      <th style={{ textAlign: "left", padding: 6 }}>Language</th>
                      <th style={{ textAlign: "left", padding: 6 }}>Compatible Quantum OS</th>
                    </tr>
                  </thead>
                  <tbody>
                    {compat.map((c) => (
                      <tr key={c.language} style={{ borderBottom: "1px solid var(--border-secondary)" }}>
                        <td style={{ padding: 6, fontWeight: 500 }}>{c.language}</td>
                        <td style={{ padding: 6 }}>{c.compatibleOs.join(", ")}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}

        {/* ── Quantum OS Tab ──────────────────────────────────────────── */}
        {tab === "os" && (
          <div>
            <h3 style={{ margin: "0 0 12px" }}>Quantum Operating Systems (15)</h3>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))", gap: 12 }}>
              {osList.map((o) => (
                <div key={o.name} style={{ padding: 12, borderRadius: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-color)" }}>
                  <div style={{ fontWeight: 600, fontSize: 14 }}>{o.name}</div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Layer: {o.layer}</div>
                  <div style={{ fontSize: 12, color: "var(--text-tertiary)", marginTop: 2 }}>Vendor: {o.vendor}</div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ── Projects Tab ────────────────────────────────────────────── */}
        {tab === "projects" && (
          <div>
            <h3 style={{ margin: "0 0 12px" }}>Quantum Projects</h3>
            <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap", alignItems: "flex-end" }}>
              <label style={{ fontSize: 12 }}>
                Name
                <br />
                <input value={npName} onChange={(e) => setNpName(e.target.value)} style={{ ...inputStyle, width: 140 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Language
                <br />
                <select value={npLang} onChange={(e) => setNpLang(e.target.value)} style={inputStyle}>
                  {languages.map((l) => (
                    <option key={l.name} value={l.name}>
                      {l.name}
                    </option>
                  ))}
                </select>
              </label>
              <label style={{ fontSize: 12 }}>
                Hardware
                <br />
                <select value={npHw} onChange={(e) => setNpHw(e.target.value)} style={inputStyle}>
                  {hardware.map((h) => (
                    <option key={h.type} value={h.type}>
                      {h.type}
                    </option>
                  ))}
                </select>
              </label>
              <label style={{ fontSize: 12 }}>
                Qubits
                <br />
                <input type="number" value={npQubits} onChange={(e) => setNpQubits(+e.target.value)} min={1} max={10000} style={{ ...inputStyle, width: 60 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Description
                <br />
                <input value={npDesc} onChange={(e) => setNpDesc(e.target.value)} style={{ ...inputStyle, width: 200 }} />
              </label>
              <button onClick={createProject} style={btnPrimary}>
                Create Project
              </button>
            </div>
            {projects.length === 0 ? (
              <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No quantum projects yet.</div>
            ) : (
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(340px, 1fr))", gap: 12 }}>
                {projects.map((p) => (
                  <div key={p.id} style={{ padding: 12, borderRadius: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-color)" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <div style={{ fontWeight: 600, fontSize: 14 }}>{p.name}</div>
                      <button onClick={() => deleteProject(p.id)} style={{ background: "none", border: "none", color: "var(--text-tertiary)", cursor: "pointer", fontSize: 16 }} title="Delete project">
                        &times;
                      </button>
                    </div>
                    <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>
                      {p.language} &middot; {p.targetHardware} &middot; {p.numQubits} qubits
                    </div>
                    {p.targetOs && <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginTop: 2 }}>OS: {p.targetOs}</div>}
                    {p.algorithm && <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginTop: 2 }}>Algorithm: {p.algorithm}</div>}
                    {p.errorCorrection && (
                      <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginTop: 2 }}>
                        ECC: {p.errorCorrection} &middot; Est. physical qubits: {p.estimatedPhysicalQubits ?? "N/A"}
                      </div>
                    )}
                    {p.description && <div style={{ fontSize: 11, color: "var(--text-tertiary)", marginTop: 4 }}>{p.description}</div>}
                    <div style={{ fontSize: 10, color: "var(--text-tertiary)", marginTop: 4 }}>{p.id}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ── Algorithms Tab ──────────────────────────────────────────── */}
        {tab === "algorithms" && (
          <div>
            <h3 style={{ margin: "0 0 12px" }}>Quantum Algorithms</h3>
            <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 12px" }}>
              Click an algorithm to view code examples in Qiskit, Cirq, and PennyLane.
            </p>
            {algorithms.map((a) => {
              const isExpanded = algoExpanded === a.name;
              const examples = ALGORITHM_EXAMPLES[a.name];
              return (
                <div key={a.name} style={{ ...cardStyle, marginBottom: 8 }}>
                  <div
                    style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }}
                    onClick={() => setAlgoExpanded(isExpanded ? null : a.name)}
                  >
                    <div>
                      <span style={{ fontWeight: 600, fontSize: 13 }}>{a.name}</span>
                      <span style={{ fontSize: 11, color: "var(--text-secondary)", marginLeft: 8 }}>{a.category}</span>
                      <span style={{ fontSize: 11, color: "var(--text-tertiary)", marginLeft: 8 }}>{a.scaling}</span>
                    </div>
                    <span style={{ fontSize: 11, color: "var(--accent-primary)" }}>{isExpanded ? "▼" : "▶"} Code</span>
                  </div>
                  {isExpanded && examples && (
                    <div style={{ marginTop: 10 }}>
                      {(["Qiskit", "Cirq", "PennyLane"] as const).map((lang) => {
                        const code = examples[lang];
                        if (!code) return null;
                        return (
                          <div key={lang} style={{ marginBottom: 10 }}>
                            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                              <span style={{ fontSize: 12, fontWeight: 600, color: "var(--accent-primary)" }}>{lang}</span>
                              <button onClick={() => navigator.clipboard.writeText(code)} style={btnSmall}>Copy</button>
                            </div>
                            <pre style={{ margin: 0, padding: 8, background: "var(--bg-primary)", borderRadius: 4, fontSize: 11, overflowX: "auto", whiteSpace: "pre-wrap", color: "var(--text-primary)", border: "1px solid var(--border-color)" }}>{code}</pre>
                          </div>
                        );
                      })}
                      {!examples && (
                        <div style={{ fontSize: 12, color: "var(--text-secondary)", fontStyle: "italic" }}>Code examples coming soon.</div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

export default QuantumComputingPanel;
