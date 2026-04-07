/**
 * CompanyAdapterPanel — BYOA adapter registry.
 *
 * Lists registered agent adapters (internal, HTTP, process, Claude, Codex).
 * Supports registering new HTTP/process adapters and testing connectivity.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyAdapterPanelProps {
  workspacePath?: string | null;
}

export function CompanyAdapterPanel({ workspacePath: _wp }: CompanyAdapterPanelProps) {
  const [listOutput, setListOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [name, setName] = useState("");
  const [adapterType, setAdapterType] = useState<"http" | "process">("http");
  const [url, setUrl] = useState("");
  const [command, setCommand] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const out = await invoke<string>("company_cmd", { args: "adapter list" });
      setListOutput(out);
    } catch (e) {
      setListOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const register = async () => {
    if (!name.trim()) return;
    const args = adapterType === "http"
      ? `adapter register ${name.trim()} --type http --url ${url}`
      : `adapter register ${name.trim()} --type process --command ${command}`;
    try {
      const out = await invoke<string>("company_cmd", { args });
      setCmdResult(out);
      setName(""); setUrl(""); setCommand("");
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const remove = async (adapterName: string) => {
    if (!confirm(`Remove adapter "${adapterName}"?`)) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `adapter remove ${adapterName}` });
      setCmdResult(out);
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  return (
    <div className="panel-container">
      <div className="panel-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Adapter Registry</span>
        <button onClick={load} className="panel-btn panel-btn-secondary">
          Refresh
        </button>
      </div>
      <div className="panel-body">

      {/* Register adapter */}
      <div className="panel-card" style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Register Adapter</div>
        <div style={{ display: "flex", gap: 8, marginBottom: 6 }}>
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Name"
            style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }} />
          <select value={adapterType} onChange={(e) => setAdapterType(e.target.value as "http" | "process")}
            style={{ fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }}>
            <option value="http">HTTP</option>
            <option value="process">Process</option>
          </select>
        </div>
        {adapterType === "http" ? (
          <input value={url} onChange={(e) => setUrl(e.target.value)} placeholder="Endpoint URL"
            style={{ width: "100%", fontSize: 12, padding: "4px 8px", marginBottom: 6, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", boxSizing: "border-box" }} />
        ) : (
          <input value={command} onChange={(e) => setCommand(e.target.value)} placeholder="Shell command"
            style={{ width: "100%", fontSize: 12, padding: "4px 8px", marginBottom: 6, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", boxSizing: "border-box" }} />
        )}
        <button onClick={register} className="panel-btn panel-btn-primary">
          Register
        </button>
      </div>

      {cmdResult && (
        <div className="panel-card" style={{ marginBottom: 12, fontSize: 12 }}>
          {cmdResult}
        </div>
      )}

      {/* Remove adapter */}
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <input
          id="remove-adapter-input"
          placeholder="Adapter name to remove"
          style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }}
        />
        <button
          onClick={() => {
            const el = document.getElementById("remove-adapter-input") as HTMLInputElement;
            if (el?.value) remove(el.value.trim());
          }}
          className="panel-btn panel-btn-danger"
        >
          Remove
        </button>
      </div>

      {/* Adapters list */}
      <div className="panel-card">
        {loading ? (
          <span className="panel-loading">Loading…</span>
        ) : (
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap" }}>
            {listOutput || "Built-in adapter: internal (VibeCody AgentPool)"}
          </pre>
        )}
      </div>
      </div>
    </div>
  );
}
