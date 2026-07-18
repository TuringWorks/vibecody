/**
 * CompanyWorkspaceConfigPanel — Global workspace configuration settings.
 *
 * Loads and saves owner/assistant/business metadata that is substituted
 * into skill prompts as {{owner_name}}, {{business_name}}, etc.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface WorkspaceConfig {
  owner_name: string;
  assistant_name: string;
  business_name: string;
  timezone: string;
  target_market: string;
  primary_update_channel: string;
  assistant_email: string;
  work_email: string;
}

const EMPTY_CONFIG: WorkspaceConfig = {
  owner_name: "",
  assistant_name: "",
  business_name: "",
  timezone: "",
  target_market: "",
  primary_update_channel: "",
  assistant_email: "",
  work_email: "",
};

const FIELD_LABELS: Record<keyof WorkspaceConfig, string> = {
  owner_name: "Owner Name",
  assistant_name: "Assistant Name",
  business_name: "Business Name",
  timezone: "Timezone",
  target_market: "Target Market",
  primary_update_channel: "Primary Update Channel",
  assistant_email: "Assistant Email",
  work_email: "Work Email",
};


export function CompanyWorkspaceConfigPanel() {
  const [config, setConfig] = useState<WorkspaceConfig>(EMPTY_CONFIG);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const result = await invoke<WorkspaceConfig>("company_workspace_config_get");
      setConfig(result);
    } catch (_e) {
      // leave defaults
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const save = async () => {
    setSaving(true);
    try {
      await invoke("company_workspace_config_set", { config });
      setToast("Configuration saved");
      setTimeout(() => setToast(null), 3000);
    } catch (e) {
      setToast(`Error: ${e}`);
      setTimeout(() => setToast(null), 5000);
    } finally {
      setSaving(false);
    }
  };

  const fields = Object.keys(FIELD_LABELS) as Array<keyof WorkspaceConfig>;

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Workspace Config</h3>
        <button onClick={load} className="panel-btn panel-btn-secondary" style={{ marginLeft: "auto" }}>Refresh</button>
      </div>
      <div className="panel-body">

        {/* Info box */}
        <div style={{
          marginBottom: 16, padding: "12px 16px",
          background: "rgba(74,158,255,0.08)", border: "1px solid rgba(74,158,255,0.25)",
          borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: 1.6,
        }}>
          These values are automatically substituted into skill prompts as{" "}
          <code style={{ fontFamily: "var(--font-mono)", color: "var(--accent-blue)" }}>{"{{owner_name}}"}</code>,{" "}
          <code style={{ fontFamily: "var(--font-mono)", color: "var(--accent-blue)" }}>{"{{business_name}}"}</code>, etc.
        </div>

        {loading ? (
          <div className="panel-loading">Loading…</div>
        ) : (
          <div style={{
            display: "grid", gridTemplateColumns: "1fr 1fr",
            gap: "12px 20px", marginBottom: 20,
          }}>
            {fields.map((key) => (
              <label key={key} style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.04em" }}>
                  {FIELD_LABELS[key]}
                </span>
                <input
                  type="text"
                  value={config[key]}
                  onChange={(e) => setConfig((prev) => ({ ...prev, [key]: e.target.value }))}
                  className="panel-input panel-input-full"
                  placeholder={`{{${key}}}`}
                />
              </label>
            ))}
          </div>
        )}

        <button
          onClick={save}
          disabled={saving || loading}
          className="panel-btn panel-btn-primary"
          style={{ opacity: (saving || loading) ? 0.6 : 1 }}
        >
          {saving ? "Saving…" : "Save"}
        </button>

        {/* Toast */}
        {toast && (
          <div style={{
            marginTop: 12, padding: "8px 16px", borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-base)",
            background: toast.startsWith("Error") ? "rgba(231,76,60,0.15)" : "rgba(39,174,96,0.15)",
            color: toast.startsWith("Error") ? "var(--accent-rose)" : "var(--accent-green)",
            border: `1px solid ${toast.startsWith("Error") ? "var(--accent-rose)" : "var(--accent-green)"}`,
          }}>
            {toast}
          </div>
        )}
      </div>
    </div>
  );
}
