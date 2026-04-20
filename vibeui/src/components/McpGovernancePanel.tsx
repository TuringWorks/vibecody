import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AuditEntry {
  id: string;
  timestamp: string;
  tool: string;
  caller: string;
  outcome: string;
  reason: string | null;
}

interface SsoConfig {
  issuer_url: string;
  client_id: string;
  groups: string[];
  enabled: boolean;
}

interface GatewayRule {
  id: string;
  pattern: string;
  action: "allow" | "deny";
  description: string;
  priority: number;
}

export function McpGovernancePanel() {
  const [tab, setTab] = useState("audit");
  const [auditLog, setAuditLog] = useState<AuditEntry[]>([]);
  const [ssoConfig, setSsoConfig] = useState<SsoConfig>({ issuer_url: "", client_id: "", groups: [], enabled: false });
  const [gatewayRules, setGatewayRules] = useState<GatewayRule[]>([]);
  const [configExport, setConfigExport] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [issuerUrl, setIssuerUrl] = useState("");
  const [clientId, setClientId] = useState("");
  const [groups, setGroups] = useState("");
  const [ssoSaving, setSsoSaving] = useState(false);
  const [ssoSavedMsg, setSsoSavedMsg] = useState<string | null>(null);
  const [configImporting, setConfigImporting] = useState(false);
  const [configMsg, setConfigMsg] = useState<string | null>(null);

  async function saveSso() {
    setSsoSaving(true);
    setSsoSavedMsg(null);
    try {
      const next: SsoConfig = {
        issuer_url: issuerUrl.trim(),
        client_id: clientId.trim(),
        groups: groups.split(",").map(g => g.trim()).filter(Boolean),
        enabled: ssoConfig.enabled,
      };
      await invoke("mcp_sso_config_save", { config: next });
      setSsoConfig(next);
      setSsoSavedMsg("Saved.");
    } catch (e) {
      setSsoSavedMsg(`Error: ${e}`);
    } finally {
      setSsoSaving(false);
    }
  }

  async function importConfig() {
    setConfigImporting(true);
    setConfigMsg(null);
    try {
      const count = await invoke<number>("mcp_config_import", { json: configExport });
      setConfigMsg(`Imported ${count} server(s).`);
    } catch (e) {
      setConfigMsg(`Error: ${e}`);
    } finally {
      setConfigImporting(false);
    }
  }

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [auditRes, ssoRes, rulesRes, configRes] = await Promise.all([
          invoke<AuditEntry[]>("mcp_audit_query"),
          invoke<SsoConfig>("mcp_sso_config"),
          invoke<GatewayRule[]>("mcp_gateway_rules"),
          invoke<string>("mcp_config_export"),
        ]);
        setAuditLog(Array.isArray(auditRes) ? auditRes : []);
        if (ssoRes) {
          setSsoConfig(ssoRes);
          setIssuerUrl(ssoRes.issuer_url);
          setClientId(ssoRes.client_id);
          setGroups((ssoRes.groups ?? []).join(", "));
        }
        setGatewayRules(Array.isArray(rulesRes) ? rulesRes : []);
        setConfigExport(typeof configRes === "string" ? configRes : "");
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  const outcomeColor = (o: string) => {
    if (o === "allowed") return "var(--success-color)";
    if (o === "denied") return "var(--error-color)";
    return "var(--warning-color)";
  };

  return (
    <div className="panel-container">
      <div className="panel-header"><h3>MCP Governance</h3></div>
      <div className="panel-tab-bar" style={{ flexWrap: "wrap" }}>
        {["audit", "sso", "gateway", "config"].map(t => (
          <button className={`panel-tab${tab === t ? " active" : ""}`} key={t} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body">
      {loading && <div className="panel-loading">Loading...</div>}
      {error && <div className="panel-error"><span>{error}</span></div>}

      {!loading && tab === "audit" && (
        <div style={{ maxHeight: 500, overflowY: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)", position: "sticky", top: 0 }}>
                {["Timestamp", "Tool", "Caller", "Outcome", "Reason"].map(h => (
                  <th key={h} style={{ padding: "8px 12px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {auditLog.length === 0 && (
                <tr><td colSpan={5} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No audit entries.</td></tr>
              )}
              {auditLog.map(entry => (
                <tr key={entry.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "8px 12px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{entry.timestamp}</td>
                  <td style={{ padding: "8px 12px", fontWeight: 600 }}>{entry.tool}</td>
                  <td style={{ padding: "8px 12px", color: "var(--text-muted)" }}>{entry.caller}</td>
                  <td style={{ padding: "8px 12px" }}>
                    <span style={{ padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-sm)", background: outcomeColor(entry.outcome) + "22", color: outcomeColor(entry.outcome) }}>{entry.outcome}</span>
                  </td>
                  <td style={{ padding: "8px 12px", color: "var(--text-muted)", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{entry.reason ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!loading && tab === "sso" && (
        <div style={{ maxWidth: 480 }}>
          <div style={{ marginBottom: 14 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 4 }}>Issuer URL</label>
            <input value={issuerUrl} onChange={e => setIssuerUrl(e.target.value)}
              placeholder="https://auth.example.com"
              style={{ width: "100%", padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", boxSizing: "border-box" }} />
          </div>
          <div style={{ marginBottom: 14 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 4 }}>Client ID</label>
            <input value={clientId} onChange={e => setClientId(e.target.value)}
              placeholder="mcp-client-id"
              style={{ width: "100%", padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", boxSizing: "border-box" }} />
          </div>
          <div style={{ marginBottom: 14 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 4 }}>Groups (comma-separated)</label>
            <input value={groups} onChange={e => setGroups(e.target.value)}
              placeholder="admins, developers, readonly"
              style={{ width: "100%", padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", boxSizing: "border-box" }} />
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 16 }}>
            <input type="checkbox" checked={ssoConfig.enabled} onChange={e => setSsoConfig(s => ({ ...s, enabled: e.target.checked }))} id="sso-enabled" />
            <label htmlFor="sso-enabled" style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>Enable SSO</label>
          </div>
          <button className="panel-btn" onClick={saveSso} disabled={ssoSaving}
            style={{ padding: "8px 20px", borderRadius: "var(--radius-sm)", cursor: ssoSaving ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600, opacity: ssoSaving ? 0.6 : 1 }}>
            {ssoSaving ? "Saving…" : "Save SSO Config"}
          </button>
          {ssoSavedMsg && (
            <div style={{ marginTop: 10, fontSize: "var(--font-size-sm)", color: ssoSavedMsg.startsWith("Error") ? "var(--error-color)" : "var(--success-color)" }}>
              {ssoSavedMsg}
            </div>
          )}
        </div>
      )}

      {!loading && tab === "gateway" && (
        <div>
          {gatewayRules.length === 0 && <div style={{ color: "var(--text-muted)" }}>No gateway rules configured.</div>}
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {gatewayRules.sort((a, b) => a.priority - b.priority).map(rule => (
              <div key={rule.id} style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: "12px 16px", display: "flex", alignItems: "center", gap: 12 }}>
                <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", minWidth: 30 }}>#{rule.priority}</span>
                <span style={{ padding: "2px 12px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-sm)", fontWeight: 600, background: rule.action === "allow" ? "var(--success-color)22" : "var(--error-color)22", color: rule.action === "allow" ? "var(--success-color)" : "var(--error-color)", border: `1px solid ${rule.action === "allow" ? "var(--success-color)" : "var(--error-color)"}` }}>{rule.action}</span>
                <code style={{ flex: 1, fontSize: "var(--font-size-base)", color: "var(--text-primary)", background: "var(--bg-primary)", padding: "2px 8px", borderRadius: "var(--radius-xs-plus)" }}>{rule.pattern}</code>
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{rule.description}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {!loading && tab === "config" && (
        <div>
          <div style={{ marginBottom: 10, fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>Server configuration (JSON export/import)</div>
          <textarea value={configExport} onChange={e => setConfigExport(e.target.value)}
            style={{ width: "100%", height: 300, padding: "12px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", resize: "vertical", boxSizing: "border-box" }} />
          <div style={{ display: "flex", gap: 10, marginTop: 10 }}>
            <button className="panel-btn" onClick={importConfig} disabled={configImporting}
              style={{ padding: "8px 20px", borderRadius: "var(--radius-sm)", cursor: configImporting ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600, opacity: configImporting ? 0.6 : 1 }}>
              {configImporting ? "Importing…" : "Import"}
            </button>
            <button className="panel-btn panel-btn-secondary" onClick={() => navigator.clipboard?.writeText(configExport)}>Copy</button>
          </div>
          {configMsg && (
            <div style={{ marginTop: 10, fontSize: "var(--font-size-sm)", color: configMsg.startsWith("Error") ? "var(--error-color)" : "var(--success-color)" }}>
              {configMsg}
            </div>
          )}
        </div>
      )}
      </div>
    </div>
  );
}
