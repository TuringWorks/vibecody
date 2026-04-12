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
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: 15, fontWeight: 700, marginBottom: 12 }}>MCP Governance</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap" }}>
        {["audit", "sso", "gateway", "config"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: 6, cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12 }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "audit" && (
        <div style={{ maxHeight: 500, overflowY: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)", position: "sticky", top: 0 }}>
                {["Timestamp", "Tool", "Caller", "Outcome", "Reason"].map(h => (
                  <th key={h} style={{ padding: "6px 10px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {auditLog.length === 0 && (
                <tr><td colSpan={5} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No audit entries.</td></tr>
              )}
              {auditLog.map(entry => (
                <tr key={entry.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{entry.timestamp}</td>
                  <td style={{ padding: "6px 10px", fontWeight: 600 }}>{entry.tool}</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)" }}>{entry.caller}</td>
                  <td style={{ padding: "6px 10px" }}>
                    <span style={{ padding: "2px 8px", borderRadius: 10, fontSize: 11, background: outcomeColor(entry.outcome) + "22", color: outcomeColor(entry.outcome) }}>{entry.outcome}</span>
                  </td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{entry.reason ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!loading && tab === "sso" && (
        <div style={{ maxWidth: 480 }}>
          <div style={{ marginBottom: 14 }}>
            <label style={{ display: "block", fontSize: 12, color: "var(--text-muted)", marginBottom: 4 }}>Issuer URL</label>
            <input value={issuerUrl} onChange={e => setIssuerUrl(e.target.value)}
              placeholder="https://auth.example.com"
              style={{ width: "100%", padding: "7px 10px", borderRadius: 6, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12, boxSizing: "border-box" }} />
          </div>
          <div style={{ marginBottom: 14 }}>
            <label style={{ display: "block", fontSize: 12, color: "var(--text-muted)", marginBottom: 4 }}>Client ID</label>
            <input value={clientId} onChange={e => setClientId(e.target.value)}
              placeholder="mcp-client-id"
              style={{ width: "100%", padding: "7px 10px", borderRadius: 6, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12, boxSizing: "border-box" }} />
          </div>
          <div style={{ marginBottom: 14 }}>
            <label style={{ display: "block", fontSize: 12, color: "var(--text-muted)", marginBottom: 4 }}>Groups (comma-separated)</label>
            <input value={groups} onChange={e => setGroups(e.target.value)}
              placeholder="admins, developers, readonly"
              style={{ width: "100%", padding: "7px 10px", borderRadius: 6, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12, boxSizing: "border-box" }} />
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 16 }}>
            <input type="checkbox" checked={ssoConfig.enabled} onChange={e => setSsoConfig(s => ({ ...s, enabled: e.target.checked }))} id="sso-enabled" />
            <label htmlFor="sso-enabled" style={{ fontSize: 12, color: "var(--text-muted)" }}>Enable SSO</label>
          </div>
          <button style={{ padding: "8px 20px", borderRadius: 6, cursor: "pointer", background: "var(--accent-color)", color: "#fff", border: "none", fontSize: 13, fontWeight: 600 }}>
            Save SSO Config
          </button>
        </div>
      )}

      {!loading && tab === "gateway" && (
        <div>
          {gatewayRules.length === 0 && <div style={{ color: "var(--text-muted)" }}>No gateway rules configured.</div>}
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {gatewayRules.sort((a, b) => a.priority - b.priority).map(rule => (
              <div key={rule.id} style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 8, padding: "10px 14px", display: "flex", alignItems: "center", gap: 12 }}>
                <span style={{ fontSize: 10, color: "var(--text-muted)", minWidth: 30 }}>#{rule.priority}</span>
                <span style={{ padding: "2px 10px", borderRadius: 10, fontSize: 11, fontWeight: 600, background: rule.action === "allow" ? "var(--success-color)22" : "var(--error-color)22", color: rule.action === "allow" ? "var(--success-color)" : "var(--error-color)", border: `1px solid ${rule.action === "allow" ? "var(--success-color)" : "var(--error-color)"}` }}>{rule.action}</span>
                <code style={{ flex: 1, fontSize: 12, color: "var(--text-primary)", background: "var(--bg-primary)", padding: "2px 8px", borderRadius: 4 }}>{rule.pattern}</code>
                <span style={{ fontSize: 11, color: "var(--text-muted)", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{rule.description}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {!loading && tab === "config" && (
        <div>
          <div style={{ marginBottom: 10, fontSize: 12, color: "var(--text-muted)" }}>Server configuration (JSON export/import)</div>
          <textarea value={configExport} onChange={e => setConfigExport(e.target.value)}
            style={{ width: "100%", height: 300, padding: "10px", borderRadius: 6, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 11, fontFamily: "var(--font-mono)", resize: "vertical", boxSizing: "border-box" }} />
          <div style={{ display: "flex", gap: 10, marginTop: 10 }}>
            <button style={{ padding: "8px 20px", borderRadius: 6, cursor: "pointer", background: "var(--accent-color)", color: "#fff", border: "none", fontSize: 13, fontWeight: 600 }}>Import</button>
            <button onClick={() => navigator.clipboard?.writeText(configExport)}
              style={{ padding: "8px 20px", borderRadius: 6, cursor: "pointer", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 13 }}>Copy</button>
          </div>
        </div>
      )}
    </div>
  );
}
