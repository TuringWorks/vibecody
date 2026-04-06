/**
 * CompanySecretsPanel — Encrypted company secrets vault.
 *
 * Lists secret keys (values never shown in plain text).
 * Add new secrets via form. Delete with confirmation.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Lock, X } from "lucide-react";

interface CompanySecretsPanelProps {
  workspacePath?: string | null;
}

const btnStyle: React.CSSProperties = {
  fontSize: 11, padding: "3px 10px", cursor: "pointer", borderRadius: 4,
  background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)",
};

const inputStyle: React.CSSProperties = {
  fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)",
};

export function CompanySecretsPanel({ workspacePath: _wp }: CompanySecretsPanelProps) {
  const [listOutput, setListOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");
  const [showAdd, setShowAdd] = useState(false);
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [deleteKey, setDeleteKey] = useState("");
  const [saving, setSaving] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const out = await invoke<string>("company_secret_list");
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
    setSaving(true);
    try {
      const out = await invoke<string>("company_secret_set", { key: newKey.trim(), value: newValue.trim() });
      setCmdResult(out);
      setNewKey("");
      setNewValue("");
      setShowAdd(false);
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const deleteSecret = async (key: string) => {
    if (!key.trim()) return;
    if (!confirm(`Delete secret "${key}"?`)) return;
    try {
      const out = await invoke<string>("company_secret_delete", { key: key.trim() });
      setCmdResult(out);
      setDeleteKey("");
      load();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const isEmpty = !listOutput || listOutput.includes("No secrets");

  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Secrets Vault</span>
        <div style={{ display: "flex", gap: 6 }}>
          <button onClick={() => { setShowAdd(!showAdd); setCmdResult(null); }} style={btnStyle}>
            {showAdd ? "Cancel" : "+ Add Secret"}
          </button>
          <button onClick={load} style={btnStyle}>Refresh</button>
        </div>
      </div>

      {/* Add form */}
      {showAdd && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Add Secret</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <input value={newKey} onChange={(e) => setNewKey(e.target.value)} placeholder="Key name *" autoFocus style={{ ...inputStyle, width: "100%", boxSizing: "border-box" }} />
            <input
              value={newValue}
              onChange={(e) => setNewValue(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addSecret()}
              placeholder="Secret value *"
              type="password"
              style={{ ...inputStyle, width: "100%", boxSizing: "border-box" }}
            />
            <button onClick={addSecret} disabled={saving || !newKey.trim() || !newValue.trim()} style={{ ...btnStyle, padding: "5px 16px", opacity: saving ? 0.6 : 1, alignSelf: "flex-start" }}>
              {saving ? "Saving…" : "Save Secret"}
            </button>
          </div>
        </div>
      )}

      {cmdResult && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 4, padding: 8, marginBottom: 12, fontSize: 12 }}>
          {cmdResult}
          <button onClick={() => setCmdResult(null)} style={{ marginLeft: 8, cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }}><X size={12} /></button>
        </div>
      )}

      {/* Secrets list */}
      {isEmpty && !loading ? (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 24, textAlign: "center" }}>
          <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent, #4a9eff)" }}><Lock size={32} strokeWidth={1.5} /></div>
          <div style={{ fontWeight: 600, marginBottom: 4 }}>No secrets stored</div>
          <div style={{ color: "var(--text-secondary)", fontSize: 12, marginBottom: 16 }}>
            Add API keys, tokens, and credentials
          </div>
          <button onClick={() => setShowAdd(true)} style={{ ...btnStyle, padding: "6px 20px", fontSize: 12 }}>+ Add Secret</button>
        </div>
      ) : (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, marginBottom: 12 }}>
          {loading ? (
            <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
          ) : (
            <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.7 }}>
              {listOutput}
            </pre>
          )}
        </div>
      )}

      {/* Delete */}
      {!isEmpty && (
        <div>
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>DELETE SECRET</div>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              value={deleteKey}
              onChange={(e) => setDeleteKey(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && deleteSecret(deleteKey)}
              placeholder="Key name to delete"
              style={{ ...inputStyle, flex: 1 }}
            />
            <button
              onClick={() => deleteSecret(deleteKey)}
              disabled={!deleteKey.trim()}
              style={{ ...btnStyle, border: "1px solid var(--danger, #e74c3c)", color: "var(--danger, #e74c3c)", opacity: deleteKey.trim() ? 1 : 0.5 }}
            >
              Delete
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
