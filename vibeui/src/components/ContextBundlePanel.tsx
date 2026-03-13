/**
 * ContextBundlePanel — Context Bundles / Spaces panel.
 *
 * Manage reusable context bundles that pin files, instructions, and
 * model preferences for quick project switching.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState } from "react";

// ── Types ─────────────────────────────────────────────────────────────────────

interface ContextBundle {
  id: string;
  name: string;
  description: string;
  pinnedFiles: string[];
  instructions: string;
  modelPreference: string;
  active: boolean;
  createdAt: string;
}

// ── Mock Data ─────────────────────────────────────────────────────────────────

const INITIAL_BUNDLES: ContextBundle[] = [
  { id: "b1", name: "Backend API", description: "Rust backend service context", pinnedFiles: ["src/main.rs", "src/lib.rs", "Cargo.toml"], instructions: "Focus on performance and error handling. Use async/await patterns.", modelPreference: "claude-opus-4-20250514", active: true, createdAt: "2026-03-10T10:00:00Z" },
  { id: "b2", name: "Frontend UI", description: "React TypeScript frontend context", pinnedFiles: ["src/App.tsx", "src/index.tsx", "package.json"], instructions: "Use functional components with hooks. Follow existing CSS variable theming.", modelPreference: "claude-sonnet-4-20250514", active: false, createdAt: "2026-03-11T14:00:00Z" },
  { id: "b3", name: "Infrastructure", description: "DevOps and deployment configs", pinnedFiles: ["Dockerfile", "docker-compose.yml", ".github/workflows/ci.yml", "terraform/main.tf"], instructions: "Prefer declarative configuration. Keep images minimal.", modelPreference: "gpt-4o", active: false, createdAt: "2026-03-12T09:00:00Z" },
  { id: "b4", name: "Testing Suite", description: "Test files and coverage config", pinnedFiles: ["tests/", "jest.config.ts", "vitest.config.ts"], instructions: "Write comprehensive unit tests. Aim for >90% coverage.", modelPreference: "claude-opus-4-20250514", active: false, createdAt: "2026-03-12T16:00:00Z" },
];

const MODEL_OPTIONS = ["claude-opus-4-20250514", "claude-sonnet-4-20250514", "gpt-4o", "gpt-4o-mini", "gemini-2.0-pro", "ollama/llama3"];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "#fff" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-mono, monospace)", boxSizing: "border-box" };
const textareaStyle: React.CSSProperties = { ...inputStyle, minHeight: 80, resize: "vertical" as const };
const selectStyle: React.CSSProperties = { ...inputStyle, cursor: "pointer" };
const toggleStyle = (on: boolean): React.CSSProperties => ({ display: "inline-block", width: 36, height: 18, borderRadius: 9, background: on ? "#22c55e" : "var(--bg-tertiary)", position: "relative", cursor: "pointer", border: "1px solid var(--border-primary)", transition: "background 0.2s" });
const toggleDot = (on: boolean): React.CSSProperties => ({ position: "absolute", top: 2, left: on ? 18 : 2, width: 12, height: 12, borderRadius: "50%", background: "#fff", transition: "left 0.2s" });

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "bundles" | "create" | "importexport";

export function ContextBundlePanel() {
  const [tab, setTab] = useState<Tab>("bundles");
  const [bundles, setBundles] = useState<ContextBundle[]>(INITIAL_BUNDLES);

  // Create form
  const [formName, setFormName] = useState("");
  const [formDesc, setFormDesc] = useState("");
  const [formFiles, setFormFiles] = useState("");
  const [formInstructions, setFormInstructions] = useState("");
  const [formModel, setFormModel] = useState(MODEL_OPTIONS[0]);

  // Import/Export
  const [jsonText, setJsonText] = useState("");
  const [importMsg, setImportMsg] = useState("");

  const toggleActive = (id: string) => {
    setBundles((prev) => prev.map((b) => (b.id === id ? { ...b, active: !b.active } : b)));
  };

  const deleteBundle = (id: string) => {
    setBundles((prev) => prev.filter((b) => b.id !== id));
  };

  const createBundle = () => {
    if (!formName.trim()) return;
    const newBundle: ContextBundle = {
      id: `b${Date.now()}`,
      name: formName.trim(),
      description: formDesc.trim(),
      pinnedFiles: formFiles.split("\n").map((f) => f.trim()).filter(Boolean),
      instructions: formInstructions.trim(),
      modelPreference: formModel,
      active: false,
      createdAt: new Date().toISOString(),
    };
    setBundles((prev) => [...prev, newBundle]);
    setFormName("");
    setFormDesc("");
    setFormFiles("");
    setFormInstructions("");
    setTab("bundles");
  };

  const exportBundles = () => {
    setJsonText(JSON.stringify(bundles, null, 2));
  };

  const importBundles = () => {
    try {
      const parsed = JSON.parse(jsonText);
      if (!Array.isArray(parsed)) { setImportMsg("Error: JSON must be an array of bundles."); return; }
      setBundles(parsed);
      setImportMsg(`Imported ${parsed.length} bundle(s) successfully.`);
    } catch {
      setImportMsg("Error: Invalid JSON.");
    }
  };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Context Bundles</h2>

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "bundles")} onClick={() => setTab("bundles")}>My Bundles</button>
        <button style={tabBtnStyle(tab === "create")} onClick={() => setTab("create")}>Create</button>
        <button style={tabBtnStyle(tab === "importexport")} onClick={() => setTab("importexport")}>Import / Export</button>
      </div>

      {tab === "bundles" && (
        <div>
          {bundles.length === 0 && <div style={cardStyle}>No bundles yet. Create one to get started.</div>}
          {bundles.map((b) => (
            <div key={b.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <div style={{ fontWeight: 600 }}>{b.name}</div>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <div style={toggleStyle(b.active)} onClick={() => toggleActive(b.id)}>
                    <div style={toggleDot(b.active)} />
                  </div>
                  <span style={{ fontSize: 10, color: b.active ? "#22c55e" : "var(--text-secondary)" }}>{b.active ? "Active" : "Inactive"}</span>
                </div>
              </div>
              <div style={labelStyle}>{b.description}</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
                Model: {b.modelPreference} | Files: {b.pinnedFiles.length} pinned
              </div>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 6 }}>
                {b.pinnedFiles.join(", ")}
              </div>
              {b.instructions && (
                <div style={{ fontSize: 11, color: "var(--text-secondary)", fontStyle: "italic", marginBottom: 6 }}>
                  "{b.instructions.slice(0, 100)}{b.instructions.length > 100 ? "..." : ""}"
                </div>
              )}
              <div style={{ display: "flex", gap: 6 }}>
                <button style={btnStyle} onClick={() => deleteBundle(b.id)}>Delete</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "create" && (
        <div>
          <div style={cardStyle}>
            <div style={{ marginBottom: 10 }}>
              <div style={labelStyle}>Bundle Name *</div>
              <input style={inputStyle} value={formName} onChange={(e) => setFormName(e.target.value)} placeholder="e.g. Backend API" />
            </div>
            <div style={{ marginBottom: 10 }}>
              <div style={labelStyle}>Description</div>
              <input style={inputStyle} value={formDesc} onChange={(e) => setFormDesc(e.target.value)} placeholder="Short description of this context bundle" />
            </div>
            <div style={{ marginBottom: 10 }}>
              <div style={labelStyle}>Pinned Files (one per line)</div>
              <textarea style={textareaStyle} value={formFiles} onChange={(e) => setFormFiles(e.target.value)} placeholder={"src/main.rs\nsrc/lib.rs\nCargo.toml"} />
            </div>
            <div style={{ marginBottom: 10 }}>
              <div style={labelStyle}>Instructions</div>
              <textarea style={textareaStyle} value={formInstructions} onChange={(e) => setFormInstructions(e.target.value)} placeholder="Custom instructions for the AI when this bundle is active..." />
            </div>
            <div style={{ marginBottom: 12 }}>
              <div style={labelStyle}>Model Preference</div>
              <select style={selectStyle} value={formModel} onChange={(e) => setFormModel(e.target.value)}>
                {MODEL_OPTIONS.map((m) => <option key={m} value={m}>{m}</option>)}
              </select>
            </div>
            <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "#fff" }} onClick={createBundle}>Create Bundle</button>
          </div>
        </div>
      )}

      {tab === "importexport" && (
        <div>
          <div style={cardStyle}>
            <div style={labelStyle}>Bundle JSON</div>
            <textarea style={{ ...textareaStyle, minHeight: 200 }} value={jsonText} onChange={(e) => setJsonText(e.target.value)} placeholder="Paste bundle JSON here to import, or click Export to populate..." />
            <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
              <button style={btnStyle} onClick={exportBundles}>Export Current Bundles</button>
              <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "#fff" }} onClick={importBundles}>Import from JSON</button>
            </div>
            {importMsg && <div style={{ marginTop: 8, fontSize: 11, color: importMsg.startsWith("Error") ? "#ef4444" : "#22c55e" }}>{importMsg}</div>}
          </div>
        </div>
      )}
    </div>
  );
}
