/**
 * ContextBundlePanel — Context Bundles / Spaces panel.
 *
 * Manage reusable context bundles that pin files, instructions, and
 * model preferences for quick project switching.
 * Wired to Tauri backend commands for persistent storage.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

const MODEL_OPTIONS = ["claude-opus-4-20250514", "claude-sonnet-4-20250514", "gpt-4o", "gpt-4o-mini", "gemini-2.0-pro", "ollama/llama3"];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "white" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-mono, monospace)", boxSizing: "border-box" };
const textareaStyle: React.CSSProperties = { ...inputStyle, minHeight: 80, resize: "vertical" as const };
const selectStyle: React.CSSProperties = { ...inputStyle, cursor: "pointer" };
const toggleStyle = (on: boolean): React.CSSProperties => ({ display: "inline-block", width: 36, height: 18, borderRadius: 9, background: on ? "var(--success-color)" : "var(--bg-tertiary)", position: "relative", cursor: "pointer", border: "1px solid var(--border-primary)", transition: "background 0.2s" });
const toggleDot = (on: boolean): React.CSSProperties => ({ position: "absolute", top: 2, left: on ? 18 : 2, width: 12, height: 12, borderRadius: "50%", background: "white", transition: "left 0.2s" });

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "bundles" | "create" | "importexport";

export function ContextBundlePanel() {
  const [tab, setTab] = useState<Tab>("bundles");
  const [bundles, setBundles] = useState<ContextBundle[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  // Create form
  const [formName, setFormName] = useState("");
  const [formDesc, setFormDesc] = useState("");
  const [formFiles, setFormFiles] = useState("");
  const [formInstructions, setFormInstructions] = useState("");
  const [formModel, setFormModel] = useState(MODEL_OPTIONS[0]);

  // Import/Export
  const [jsonText, setJsonText] = useState("");
  const [importMsg, setImportMsg] = useState("");

  const loadBundles = useCallback(async () => {
    try {
      setLoading(true);
      setError("");
      const result = await invoke("context_bundle_list") as { bundles: ContextBundle[]; active_count: number };
      setBundles(result.bundles ?? []);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadBundles();
  }, [loadBundles]);

  const toggleActive = async (id: string) => {
    const bundle = bundles.find((b) => b.id === id);
    if (!bundle) return;
    try {
      const updated = await invoke("context_bundle_activate", { id, active: !bundle.active }) as ContextBundle;
      setBundles((prev) => prev.map((b) => (b.id === id ? updated : b)));
    } catch (e) {
      setError(String(e));
    }
  };

  const deleteBundle = async (id: string) => {
    try {
      await invoke("context_bundle_delete", { id });
      setBundles((prev) => prev.filter((b) => b.id !== id));
    } catch (e) {
      setError(String(e));
    }
  };

  const createBundle = async () => {
    if (!formName.trim()) return;
    try {
      const pinnedFiles = formFiles.split("\n").map((f) => f.trim()).filter(Boolean);
      const created = await invoke("context_bundle_create", {
        name: formName.trim(),
        description: formDesc.trim(),
        pinnedFiles,
        instructions: formInstructions.trim(),
        modelPreference: formModel,
      }) as ContextBundle;
      setBundles((prev) => [...prev, created]);
      setFormName("");
      setFormDesc("");
      setFormFiles("");
      setFormInstructions("");
      setTab("bundles");
    } catch (e) {
      setError(String(e));
    }
  };

  const exportBundles = async () => {
    try {
      const result = await invoke("context_bundle_export") as ContextBundle[];
      setJsonText(JSON.stringify(result, null, 2));
    } catch (e) {
      setError(String(e));
    }
  };

  const importBundles = async () => {
    try {
      const parsed = JSON.parse(jsonText);
      if (!Array.isArray(parsed)) { setImportMsg("Error: JSON must be an array of bundles."); return; }
      const result = await invoke("context_bundle_import", { bundlesJson: parsed }) as { imported: number };
      setImportMsg(`Imported ${result.imported} bundle(s) successfully.`);
      await loadBundles();
    } catch {
      setImportMsg("Error: Invalid JSON.");
    }
  };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Context Bundles</h2>

      {error && <div style={{ ...cardStyle, borderColor: "var(--error-color)", color: "var(--error-color)", marginBottom: 12 }}>{error}</div>}

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "bundles")} onClick={() => setTab("bundles")}>My Bundles</button>
        <button style={tabBtnStyle(tab === "create")} onClick={() => setTab("create")}>Create</button>
        <button style={tabBtnStyle(tab === "importexport")} onClick={() => setTab("importexport")}>Import / Export</button>
      </div>

      {tab === "bundles" && (
        <div>
          {loading && <div style={cardStyle}>Loading bundles...</div>}
          {!loading && bundles.length === 0 && <div style={cardStyle}>No bundles yet. Create one to get started.</div>}
          {bundles.map((b) => (
            <div key={b.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <div style={{ fontWeight: 600 }}>{b.name}</div>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <div style={toggleStyle(b.active)} onClick={() => toggleActive(b.id)}>
                    <div style={toggleDot(b.active)} />
                  </div>
                  <span style={{ fontSize: 10, color: b.active ? "var(--success-color)" : "var(--text-secondary)" }}>{b.active ? "Active" : "Inactive"}</span>
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
                  &quot;{b.instructions.slice(0, 100)}{b.instructions.length > 100 ? "..." : ""}&quot;
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
            <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "white" }} onClick={createBundle}>Create Bundle</button>
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
              <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "white" }} onClick={importBundles}>Import from JSON</button>
            </div>
            {importMsg && <div style={{ marginTop: 8, fontSize: 11, color: importMsg.startsWith("Error") ? "var(--error-color)" : "var(--success-color)" }}>{importMsg}</div>}
          </div>
        </div>
      )}
    </div>
  );
}
