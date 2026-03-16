import { useState, useEffect } from "react";
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

type QuantumTab = "languages" | "os" | "circuits" | "projects" | "algorithms" | "hardware";

const TABS: { id: QuantumTab; label: string }[] = [
  { id: "languages", label: "Languages" },
  { id: "os", label: "Quantum OS" },
  { id: "circuits", label: "Circuits" },
  { id: "projects", label: "Projects" },
  { id: "algorithms", label: "Algorithms" },
  { id: "hardware", label: "Hardware" },
];

// ── Component ────────────────────────────────────────────────────────────────

export function QuantumComputingPanel() {
  const [tab, setTab] = useState<QuantumTab>("languages");
  const [languages, setLanguages] = useState<QuantumLanguageInfo[]>([]);
  const [osList, setOsList] = useState<QuantumOSInfo[]>([]);
  const [projects, setProjects] = useState<QuantumProject[]>([]);
  const [circuits, setCircuits] = useState<QuantumCircuit[]>([]);
  const [compat, setCompat] = useState<CompatEntry[]>([]);
  const [algorithms, setAlgorithms] = useState<{ name: string; category: string; scaling: string }[]>([]);
  const [hardware, setHardware] = useState<{ type: string; vendors: string[] }[]>([]);
  const [helloCode, setHelloCode] = useState<string>("");
  const [selectedLang, setSelectedLang] = useState<string>("");

  // New project form
  const [npName, setNpName] = useState("");
  const [npLang, setNpLang] = useState("Qiskit");
  const [npHw, setNpHw] = useState("Superconducting");
  const [npQubits, setNpQubits] = useState(2);
  const [npDesc, setNpDesc] = useState("");

  // New circuit form
  const [ncName, setNcName] = useState("");
  const [ncQubits, setNcQubits] = useState(2);
  const [ncClassical, setNcClassical] = useState(2);

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
      // individual loads may fail, that's ok
    }
  }

  async function loadHelloCircuit(lang: string) {
    setSelectedLang(lang);
    try {
      const code = await invoke<string>("quantum_get_hello_circuit", { language: lang });
      setHelloCode(code);
    } catch {
      setHelloCode("// Not available for this language");
    }
  }

  async function createProject() {
    if (!npName.trim()) return;
    try {
      await invoke("quantum_create_project", {
        name: npName, language: npLang, hardware: npHw,
        numQubits: npQubits, description: npDesc,
      });
      setNpName(""); setNpDesc("");
      const projs = await invoke<QuantumProject[]>("quantum_get_projects");
      setProjects(projs);
    } catch { /* ignore */ }
  }

  async function deleteProject(id: string) {
    try {
      await invoke("quantum_delete_project", { projectId: id });
      const projs = await invoke<QuantumProject[]>("quantum_get_projects");
      setProjects(projs);
    } catch { /* ignore */ }
  }

  async function createCircuit() {
    if (!ncName.trim()) return;
    try {
      await invoke("quantum_create_circuit", {
        name: ncName, numQubits: ncQubits, numClassical: ncClassical,
      });
      setNcName("");
      const circs = await invoke<QuantumCircuit[]>("quantum_get_circuits");
      setCircuits(circs);
    } catch { /* ignore */ }
  }

  async function exportCircuit(index: number, format: string) {
    try {
      const code = await invoke<string>("quantum_export_circuit", { index, format });
      setHelloCode(code);
      setSelectedLang(`Circuit export (${format})`);
      setTab("languages"); // show in the code view area
    } catch { /* ignore */ }
  }

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", color: "var(--text-primary)", background: "var(--bg-primary)" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", gap: 2, padding: "8px 12px", borderBottom: "1px solid var(--border-primary)", flexWrap: "wrap" }}>
        {TABS.map(t => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            style={{
              padding: "6px 14px", borderRadius: 6, border: "none", cursor: "pointer",
              background: tab === t.id ? "var(--accent-primary)" : "var(--bg-secondary)",
              color: tab === t.id ? "#fff" : "var(--text-secondary)",
              fontWeight: tab === t.id ? 600 : 400, fontSize: 13,
            }}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {/* ── Languages Tab ─────────────────────────────────────────── */}
        {tab === "languages" && (
          <div>
            <h3 style={{ margin: "0 0 12px", color: "var(--text-primary)" }}>Quantum Programming Languages (20)</h3>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: 12 }}>
              {languages.map(l => (
                <div
                  key={l.name}
                  onClick={() => loadHelloCircuit(l.name)}
                  style={{
                    padding: 12, borderRadius: 8, cursor: "pointer",
                    background: selectedLang === l.name ? "var(--accent-primary-10)" : "var(--bg-secondary)",
                    border: selectedLang === l.name ? "1px solid var(--accent-primary)" : "1px solid var(--border-primary)",
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
                <pre style={{
                  background: "var(--bg-tertiary)", padding: 12, borderRadius: 8, fontSize: 12,
                  overflow: "auto", maxHeight: 400, whiteSpace: "pre-wrap",
                }}>{helloCode}</pre>
              </div>
            )}

            {compat.length > 0 && (
              <div style={{ marginTop: 20 }}>
                <h4 style={{ margin: "0 0 8px" }}>Language / OS Compatibility Matrix</h4>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                  <thead>
                    <tr style={{ borderBottom: "2px solid var(--border-primary)" }}>
                      <th style={{ textAlign: "left", padding: 6 }}>Language</th>
                      <th style={{ textAlign: "left", padding: 6 }}>Compatible Quantum OS</th>
                    </tr>
                  </thead>
                  <tbody>
                    {compat.map(c => (
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

        {/* ── Quantum OS Tab ────────────────────────────────────────── */}
        {tab === "os" && (
          <div>
            <h3 style={{ margin: "0 0 12px" }}>Quantum Operating Systems (15)</h3>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))", gap: 12 }}>
              {osList.map(o => (
                <div key={o.name} style={{ padding: 12, borderRadius: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-primary)" }}>
                  <div style={{ fontWeight: 600, fontSize: 14 }}>{o.name}</div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>
                    Layer: {o.layer}
                  </div>
                  <div style={{ fontSize: 12, color: "var(--text-tertiary)", marginTop: 2 }}>
                    Vendor: {o.vendor}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ── Circuits Tab ──────────────────────────────────────────── */}
        {tab === "circuits" && (
          <div>
            <h3 style={{ margin: "0 0 12px" }}>Quantum Circuits</h3>
            <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap", alignItems: "flex-end" }}>
              <label style={{ fontSize: 12 }}>
                Name<br />
                <input value={ncName} onChange={e => setNcName(e.target.value)} style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", width: 140 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Qubits<br />
                <input type="number" value={ncQubits} onChange={e => setNcQubits(+e.target.value)} min={1} max={100} style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", width: 60 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Classical bits<br />
                <input type="number" value={ncClassical} onChange={e => setNcClassical(+e.target.value)} min={0} max={100} style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", width: 60 }} />
              </label>
              <button onClick={createCircuit} style={{ padding: "6px 14px", borderRadius: 6, border: "none", background: "var(--accent-primary)", color: "var(--text-primary)", cursor: "pointer", fontWeight: 600, fontSize: 13 }}>
                Create Circuit
              </button>
            </div>
            {circuits.length === 0 ? (
              <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No circuits yet. Create one above.</div>
            ) : (
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                <thead>
                  <tr style={{ borderBottom: "2px solid var(--border-primary)" }}>
                    <th style={{ textAlign: "left", padding: 6 }}>Name</th>
                    <th style={{ textAlign: "right", padding: 6 }}>Qubits</th>
                    <th style={{ textAlign: "right", padding: 6 }}>Gates</th>
                    <th style={{ textAlign: "right", padding: 6 }}>Depth</th>
                    <th style={{ textAlign: "right", padding: 6 }}>2Q Gates</th>
                    <th style={{ textAlign: "center", padding: 6 }}>Export</th>
                  </tr>
                </thead>
                <tbody>
                  {circuits.map(c => (
                    <tr key={c.index} style={{ borderBottom: "1px solid var(--border-secondary)" }}>
                      <td style={{ padding: 6 }}>{c.name}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{c.numQubits}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{c.gateCount}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{c.depth}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{c.twoQubitGates}</td>
                      <td style={{ padding: 6, textAlign: "center" }}>
                        <button onClick={() => exportCircuit(c.index, "qasm3")} style={{ marginRight: 4, background: "none", border: "1px solid var(--border-primary)", borderRadius: 4, color: "var(--text-secondary)", cursor: "pointer", padding: "2px 6px", fontSize: 11 }}>QASM3</button>
                        <button onClick={() => exportCircuit(c.index, "qiskit")} style={{ marginRight: 4, background: "none", border: "1px solid var(--border-primary)", borderRadius: 4, color: "var(--text-secondary)", cursor: "pointer", padding: "2px 6px", fontSize: 11 }}>Qiskit</button>
                        <button onClick={() => exportCircuit(c.index, "cirq")} style={{ background: "none", border: "1px solid var(--border-primary)", borderRadius: 4, color: "var(--text-secondary)", cursor: "pointer", padding: "2px 6px", fontSize: 11 }}>Cirq</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        )}

        {/* ── Projects Tab ──────────────────────────────────────────── */}
        {tab === "projects" && (
          <div>
            <h3 style={{ margin: "0 0 12px" }}>Quantum Projects</h3>
            <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap", alignItems: "flex-end" }}>
              <label style={{ fontSize: 12 }}>
                Name<br />
                <input value={npName} onChange={e => setNpName(e.target.value)} style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", width: 140 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Language<br />
                <select value={npLang} onChange={e => setNpLang(e.target.value)} style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)" }}>
                  {languages.map(l => <option key={l.name} value={l.name}>{l.name}</option>)}
                </select>
              </label>
              <label style={{ fontSize: 12 }}>
                Hardware<br />
                <select value={npHw} onChange={e => setNpHw(e.target.value)} style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)" }}>
                  {hardware.map(h => <option key={h.type} value={h.type}>{h.type}</option>)}
                </select>
              </label>
              <label style={{ fontSize: 12 }}>
                Qubits<br />
                <input type="number" value={npQubits} onChange={e => setNpQubits(+e.target.value)} min={1} max={10000} style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", width: 60 }} />
              </label>
              <label style={{ fontSize: 12 }}>
                Description<br />
                <input value={npDesc} onChange={e => setNpDesc(e.target.value)} style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", width: 200 }} />
              </label>
              <button onClick={createProject} style={{ padding: "6px 14px", borderRadius: 6, border: "none", background: "var(--accent-primary)", color: "var(--text-primary)", cursor: "pointer", fontWeight: 600, fontSize: 13 }}>
                Create Project
              </button>
            </div>
            {projects.length === 0 ? (
              <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No quantum projects yet.</div>
            ) : (
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(340px, 1fr))", gap: 12 }}>
                {projects.map(p => (
                  <div key={p.id} style={{ padding: 12, borderRadius: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-primary)" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <div style={{ fontWeight: 600, fontSize: 14 }}>{p.name}</div>
                      <button onClick={() => deleteProject(p.id)} style={{ background: "none", border: "none", color: "var(--text-tertiary)", cursor: "pointer", fontSize: 16 }} title="Delete project">&times;</button>
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

        {/* ── Algorithms Tab ────────────────────────────────────────── */}
        {tab === "algorithms" && (
          <div>
            <h3 style={{ margin: "0 0 12px" }}>Quantum Algorithms (15)</h3>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
              <thead>
                <tr style={{ borderBottom: "2px solid var(--border-primary)" }}>
                  <th style={{ textAlign: "left", padding: 6 }}>Algorithm</th>
                  <th style={{ textAlign: "left", padding: 6 }}>Category</th>
                  <th style={{ textAlign: "left", padding: 6 }}>Qubit Scaling</th>
                </tr>
              </thead>
              <tbody>
                {algorithms.map(a => (
                  <tr key={a.name} style={{ borderBottom: "1px solid var(--border-secondary)" }}>
                    <td style={{ padding: 6, fontWeight: 500 }}>{a.name}</td>
                    <td style={{ padding: 6 }}>{a.category}</td>
                    <td style={{ padding: 6, fontSize: 11, color: "var(--text-secondary)" }}>{a.scaling}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* ── Hardware Tab ──────────────────────────────────────────── */}
        {tab === "hardware" && (
          <div>
            <h3 style={{ margin: "0 0 12px" }}>Quantum Hardware Types</h3>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))", gap: 12 }}>
              {hardware.map(h => (
                <div key={h.type} style={{ padding: 12, borderRadius: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-primary)" }}>
                  <div style={{ fontWeight: 600, fontSize: 14 }}>{h.type}</div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 6 }}>
                    {h.vendors.map(v => (
                      <span key={v} style={{ display: "inline-block", padding: "2px 8px", marginRight: 4, marginBottom: 4, borderRadius: 4, background: "var(--bg-tertiary)", fontSize: 11 }}>{v}</span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default QuantumComputingPanel;
