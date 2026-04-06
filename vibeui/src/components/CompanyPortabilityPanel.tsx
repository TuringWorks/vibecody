/**
 * CompanyPortabilityPanel — Export/import company blueprints.
 *
 * Export entire company state (agents, goals, tasks, docs — secrets scrubbed)
 * to JSON. Import from a previously exported JSON file.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyPortabilityPanelProps {
  workspacePath?: string | null;
}

export function CompanyPortabilityPanel({ workspacePath: _wp }: CompanyPortabilityPanelProps) {
  const [exportOutput, setExportOutput] = useState<string | null>(null);
  const [importPath, setImportPath] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const runExport = async () => {
    setLoading(true);
    try {
      const out = await invoke<string>("company_cmd", { args: "export" });
      setExportOutput(out);
    } catch (e) {
      setExportOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const runImport = async () => {
    if (!importPath.trim()) return;
    setLoading(true);
    try {
      const out = await invoke<string>("company_cmd", { args: `import ${importPath.trim()}` });
      setCmdResult(out);
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 16 }}>Company Portability</div>

      <div style={{ marginBottom: 20 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
          Export Company Blueprint
        </div>
        <p style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 10, lineHeight: 1.5 }}>
          Exports the active company's full structure: org chart, agents, goals, tasks, documents,
          and routines. <strong>Secrets are scrubbed</strong> from the export.
        </p>
        <button onClick={runExport} disabled={loading} style={{ fontSize: 11, padding: "4px 16px", cursor: "pointer" }}>
          {loading ? "Exporting…" : "Export Company"}
        </button>
        {exportOutput && (
          <div style={{ marginTop: 12 }}>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
              Export output (copy to save as .json file):
            </div>
            <textarea
              readOnly
              value={exportOutput}
              style={{
                width: "100%", height: 200, fontSize: 11, padding: 8,
                background: "var(--panel-bg, rgba(0,0,0,0.2))",
                border: "1px solid var(--border)", borderRadius: 4,
                color: "var(--text-primary)", resize: "vertical", boxSizing: "border-box",
              }}
            />
          </div>
        )}
      </div>

      <div style={{ borderTop: "1px solid var(--border)", paddingTop: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
          Import Company Blueprint
        </div>
        <p style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 10, lineHeight: 1.5 }}>
          Import a previously exported company JSON. A new company is created with re-mapped IDs.
        </p>
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <input
            value={importPath}
            onChange={(e) => setImportPath(e.target.value)}
            placeholder="/path/to/company-export.json"
            style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }}
          />
          <button onClick={runImport} disabled={loading} style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer" }}>
            Import
          </button>
        </div>
        {cmdResult && (
          <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border)", borderRadius: 4, padding: 8, fontSize: 12 }}>
            {cmdResult}
          </div>
        )}
      </div>
    </div>
  );
}
