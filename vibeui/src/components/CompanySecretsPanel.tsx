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
    <div className="panel-container">
      <div className="panel-header">
        <h3>Secrets Vault</h3>
        <div style={{ display: "flex", gap: 6, marginLeft: "auto" }}>
          <button onClick={() => { setShowAdd(!showAdd); setCmdResult(null); }} className="panel-btn panel-btn-secondary">
            {showAdd ? "Cancel" : "+ Add Secret"}
          </button>
          <button onClick={load} className="panel-btn panel-btn-secondary">Refresh</button>
        </div>
      </div>

      <div className="panel-body">
        {/* Add form */}
        {showAdd && (
          <div className="panel-card" style={{ marginBottom: 12 }}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Add Secret</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <input value={newKey} onChange={(e) => setNewKey(e.target.value)} placeholder="Key name *" autoFocus className="panel-input panel-input-full" />
              <input
                value={newValue}
                onChange={(e) => setNewValue(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && addSecret()}
                placeholder="Secret value *"
                type="password"
                className="panel-input panel-input-full"
              />
              <button onClick={addSecret} disabled={saving || !newKey.trim() || !newValue.trim()} className="panel-btn panel-btn-primary" style={{ opacity: saving ? 0.6 : 1, alignSelf: "flex-start" }}>
                {saving ? "Saving…" : "Save Secret"}
              </button>
            </div>
          </div>
        )}

        {cmdResult && (
          <div className="panel-card" style={{ marginBottom: 12, fontSize: 12 }}>
            {cmdResult}
            <button onClick={() => setCmdResult(null)} style={{ marginLeft: 8, cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }}><X size={12} /></button>
          </div>
        )}

        {/* Secrets list */}
        {isEmpty && !loading ? (
          <div className="panel-empty">
            <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent, #4a9eff)" }}><Lock size={32} strokeWidth={1.5} /></div>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>No secrets stored</div>
            <div style={{ color: "var(--text-secondary)", fontSize: 12, marginBottom: 16 }}>
              Add API keys, tokens, and credentials
            </div>
            <button onClick={() => setShowAdd(true)} className="panel-btn panel-btn-primary">+ Add Secret</button>
          </div>
        ) : (
          <div className="panel-card" style={{ marginBottom: 12 }}>
            {loading ? (
              <div className="panel-loading">Loading…</div>
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
            <div className="panel-label" style={{ marginBottom: 6, fontWeight: 600 }}>DELETE SECRET</div>
            <div style={{ display: "flex", gap: 8 }}>
              <input
                value={deleteKey}
                onChange={(e) => setDeleteKey(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && deleteSecret(deleteKey)}
                placeholder="Key name to delete"
                className="panel-input"
                style={{ flex: 1 }}
              />
              <button
                onClick={() => deleteSecret(deleteKey)}
                disabled={!deleteKey.trim()}
                className="panel-btn panel-btn-danger"
                style={{ opacity: deleteKey.trim() ? 1 : 0.5 }}
              >
                Delete
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
