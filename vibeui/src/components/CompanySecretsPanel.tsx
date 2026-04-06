/**
 * CompanySecretsPanel — Encrypted company secrets vault.
 *
 * Lists secret keys (values hidden by default, reveal on click).
 * Supports add, delete. Values never stored in component state
 * longer than needed for display.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanySecretsPanelProps {
  workspacePath?: string | null;
}

export function CompanySecretsPanel({ workspacePath: _wp }: CompanySecretsPanelProps) {
  const [listOutput, setListOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [revealKey, setRevealKey] = useState<string | null>(null);
  const [revealValue, setRevealValue] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const out = await invoke<string>("company_cmd", { args: "secret list" });
      setListOutput(out);
    } catch (e) {
      setListOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const addSecret = async () => {
    if (!newKey.trim() || !newValue.trim()) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `secret set ${newKey.trim()} "${newValue.trim()}"` });
      setCmdResult(out);
      setNewKey("");
      setNewValue("");
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const revealSecret = async (key: string) => {
    if (revealKey === key) {
      setRevealKey(null);
      setRevealValue(null);
      return;
    }
    try {
      const val = await invoke<string>("company_cmd", { args: `secret get ${key}` });
      setRevealKey(key);
      setRevealValue(val);
      // Auto-hide after 10 seconds
      setTimeout(() => { setRevealKey(null); setRevealValue(null); }, 10000);
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const deleteSecret = async (key: string) => {
    if (!confirm(`Delete secret "${key}"?`)) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `secret delete ${key}` });
      setCmdResult(out);
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Secrets Vault</span>
        <button onClick={load} style={{ fontSize: 11, padding: "2px 8px", cursor: "pointer" }}>
          Refresh
        </button>
      </div>

      {/* Add secret */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>Add Secret</div>
        <div style={{ display: "flex", gap: 8, marginBottom: 6 }}>
          <input
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
            placeholder="Key name"
            style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }}
          />
          <input
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="Value"
            type="password"
            style={{ flex: 2, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }}
          />
          <button onClick={addSecret} style={{ fontSize: 11, padding: "4px 12px", cursor: "pointer" }}>
            Set
          </button>
        </div>
      </div>

      {cmdResult && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border)", borderRadius: 4, padding: 8, marginBottom: 12, fontSize: 12 }}>
          {cmdResult}
        </div>
      )}

      {/* Secrets list */}
      <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border)", borderRadius: 6, padding: 12 }}>
        {loading ? (
          <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
        ) : (
          <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap" }}>
            {listOutput || "No secrets. Use the form above to add one."}
          </pre>
        )}
      </div>

      {/* Reveal / Delete by key */}
      <div style={{ marginTop: 12, display: "flex", gap: 8, alignItems: "center" }}>
        <input
          placeholder="Key to reveal or delete"
          id="secret-key-input"
          style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--input-bg, rgba(0,0,0,0.3))", border: "1px solid var(--border)", borderRadius: 4, color: "var(--text-primary)" }}
        />
        <button
          onClick={() => {
            const el = document.getElementById("secret-key-input") as HTMLInputElement;
            if (el?.value) revealSecret(el.value.trim());
          }}
          style={{ fontSize: 11, padding: "4px 10px", cursor: "pointer" }}
        >
          Reveal
        </button>
        <button
          onClick={() => {
            const el = document.getElementById("secret-key-input") as HTMLInputElement;
            if (el?.value) deleteSecret(el.value.trim());
          }}
          style={{ fontSize: 11, padding: "4px 10px", cursor: "pointer", color: "var(--danger, #e74c3c)" }}
        >
          Delete
        </button>
      </div>

      {revealKey && revealValue && (
        <div style={{ marginTop: 12, background: "var(--warning-bg, rgba(255,200,0,0.1))", border: "1px solid var(--warning)", borderRadius: 4, padding: 10, fontSize: 12 }}>
          <strong>{revealKey}:</strong> {revealValue}
          <div style={{ marginTop: 4, fontSize: 11, color: "var(--text-secondary)" }}>Auto-hidden in 10s</div>
        </div>
      )}
    </div>
  );
}
