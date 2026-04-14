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
    <div className="panel-container">
      <div className="panel-header"><h3>Company Portability</h3></div>
      <div className="panel-body">

      <div style={{ marginBottom: 20 }}>
        <div className="panel-label" style={{ marginBottom: 8 }}>
          Export Company Blueprint
        </div>
        <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 10, lineHeight: 1.5 }}>
          Exports the active company's full structure: org chart, agents, goals, tasks, documents,
          and routines. <strong>Secrets are scrubbed</strong> from the export.
        </p>
        <button onClick={runExport} disabled={loading} className="panel-btn panel-btn-primary">
          {loading ? "Exporting…" : "Export Company"}
        </button>
        {exportOutput && (
          <div style={{ marginTop: 12 }}>
            <div className="panel-label" style={{ marginBottom: 4 }}>
              Export output (copy to save as .json file):
            </div>
            <textarea
              readOnly
              value={exportOutput}
              className="panel-input panel-textarea panel-input-full"
              style={{ height: 200 }}
            />
          </div>
        )}
      </div>

      <div style={{ borderTop: "1px solid var(--border)", paddingTop: 16 }}>
        <div className="panel-label" style={{ marginBottom: 8 }}>
          Import Company Blueprint
        </div>
        <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 10, lineHeight: 1.5 }}>
          Import a previously exported company JSON. A new company is created with re-mapped IDs.
        </p>
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <input
            value={importPath}
            onChange={(e) => setImportPath(e.target.value)}
            placeholder="/path/to/company-export.json"
            className="panel-input"
            style={{ flex: 1 }}
          />
          <button onClick={runImport} disabled={loading} className="panel-btn panel-btn-secondary">
            Import
          </button>
        </div>
        {cmdResult && (
          <div className="panel-card" style={{ fontSize: "var(--font-size-base)" }}>
            {cmdResult}
          </div>
        )}
      </div>
      </div>
    </div>
  );
}
