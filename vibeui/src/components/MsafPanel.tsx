import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AgentManifest {
  agent_id: string;
  name: string;
  version: string;
  capabilities: string[];
  description: string;
  author: string;
}

interface CatalogEntry {
  agent_id: string;
  name: string;
  version: string;
  status: string;
  heartbeat_at: string;
  endpoint: string;
}

interface TokenResult {
  valid: boolean;
  agent_id: string | null;
  scopes: string[];
  expires_at: string | null;
  error: string | null;
}

export function MsafPanel() {
  const [tab, setTab] = useState("manifest");
  const [manifest, setManifest] = useState<AgentManifest | null>(null);
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tokenInput, setTokenInput] = useState("");
  const [tokenResult, setTokenResult] = useState<TokenResult | null>(null);
  const [validating, setValidating] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [manifestRes, catalogRes] = await Promise.all([
          invoke<AgentManifest>("msaf_manifest"),
          invoke<CatalogEntry[]>("msaf_catalog_list"),
        ]);
        setManifest(manifestRes ?? null);
        setCatalog(Array.isArray(catalogRes) ? catalogRes : []);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function validateToken() {
    if (!tokenInput.trim()) return;
    setValidating(true);
    setTokenResult(null);
    try {
      const res = await invoke<TokenResult>("msaf_validate_token", { token: tokenInput.trim() });
      setTokenResult(res ?? null);
    } catch (e) {
      setTokenResult({ valid: false, agent_id: null, scopes: [], expires_at: null, error: String(e) });
    } finally {
      setValidating(false);
    }
  }

  const statusColor = (s: string) => {
    if (s === "online") return "var(--success-color)";
    if (s === "offline") return "var(--text-muted)";
    if (s === "degraded") return "var(--warning-color)";
    return "var(--error-color)";
  };

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>MSAF — Multi-Agent Standard Framework</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["manifest", "catalog", "tokens"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "manifest" && (
        <div>
          {!manifest && <div style={{ color: "var(--text-muted)" }}>No manifest available.</div>}
          {manifest && (
            <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: 16 }}>
              <div style={{ display: "grid", gridTemplateColumns: "120px 1fr", rowGap: 10, fontSize: "var(--font-size-md)" }}>
                {[
                  ["Agent ID", manifest.agent_id],
                  ["Name", manifest.name],
                  ["Version", manifest.version],
                  ["Author", manifest.author],
                  ["Description", manifest.description],
                ].map(([label, value]) => (
                  <>
                    <span key={`l-${label}`} style={{ color: "var(--text-muted)", fontSize: "var(--font-size-base)" }}>{label}</span>
                    <span key={`v-${label}`} style={{ color: "var(--text-primary)" }}>{value}</span>
                  </>
                ))}
              </div>
              <div style={{ marginTop: 16 }}>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 8 }}>Capabilities</div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                  {(manifest.capabilities ?? []).map(cap => (
                    <span key={cap} style={{ padding: "3px 10px", borderRadius: 12, fontSize: "var(--font-size-sm)", background: "var(--accent-color)22", color: "var(--accent-color)", border: "1px solid var(--accent-color)" }}>{cap}</span>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {!loading && tab === "catalog" && (
        <div>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)" }}>
                {["Name", "Version", "Status", "Heartbeat", "Endpoint"].map(h => (
                  <th key={h} style={{ padding: "6px 10px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {catalog.length === 0 && (
                <tr><td colSpan={5} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No agents registered.</td></tr>
              )}
              {catalog.map(entry => (
                <tr key={entry.agent_id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "6px 10px", fontWeight: 600 }}>{entry.name}</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)" }}>{entry.version}</td>
                  <td style={{ padding: "6px 10px" }}>
                    <span style={{ padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-sm)", background: statusColor(entry.status) + "22", color: statusColor(entry.status) }}>{entry.status}</span>
                  </td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{entry.heartbeat_at}</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)", maxWidth: 180, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{entry.endpoint}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!loading && tab === "tokens" && (
        <div style={{ maxWidth: 500 }}>
          <div style={{ marginBottom: 12 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Paste agent token to validate</label>
            <textarea value={tokenInput} onChange={e => setTokenInput(e.target.value)}
              placeholder="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."
              style={{ width: "100%", height: 80, padding: "8px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", resize: "vertical", boxSizing: "border-box" }} />
          </div>
          <button onClick={validateToken} disabled={validating}
            style={{ padding: "8px 20px", borderRadius: "var(--radius-sm)", cursor: validating ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600, opacity: validating ? 0.6 : 1, marginBottom: 16 }}>
            {validating ? "Validating…" : "Validate Token"}
          </button>
          {tokenResult && (
            <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: `1px solid ${tokenResult.valid ? "var(--success-color)" : "var(--error-color)"}`, padding: 14 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
                <span style={{ fontWeight: 700, fontSize: "var(--font-size-lg)", color: tokenResult.valid ? "var(--success-color)" : "var(--error-color)" }}>
                  {tokenResult.valid ? "Valid" : "Invalid"}
                </span>
              </div>
              {tokenResult.error && <div style={{ color: "var(--error-color)", fontSize: "var(--font-size-base)", marginBottom: 8 }}>{tokenResult.error}</div>}
              {tokenResult.valid && (
                <div style={{ display: "grid", gridTemplateColumns: "100px 1fr", rowGap: 6, fontSize: "var(--font-size-base)" }}>
                  <span style={{ color: "var(--text-muted)" }}>Agent ID</span>
                  <span>{tokenResult.agent_id ?? "—"}</span>
                  <span style={{ color: "var(--text-muted)" }}>Expires</span>
                  <span>{tokenResult.expires_at ?? "never"}</span>
                  <span style={{ color: "var(--text-muted)" }}>Scopes</span>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                    {(tokenResult.scopes ?? []).map(s => (
                      <span key={s} style={{ padding: "1px 8px", borderRadius: "var(--radius-sm-alt)", fontSize: "var(--font-size-sm)", background: "var(--accent-color)22", color: "var(--accent-color)" }}>{s}</span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
