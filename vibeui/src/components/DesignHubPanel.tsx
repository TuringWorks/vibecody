/**
 * DesignHubPanel — unified multi-provider design hub.
 *
 * Replaces and extends the Figma-only tab in DesignMode.
 * Tabs: Providers | Tokens | Audit | Figma (legacy) | Settings
 * - Providers: Switch between Figma, Penpot, Pencil, Draw.io, Mermaid, Built-in
 * - Tokens: Cross-provider token browser with CSS/Tailwind/JSON export
 * - Audit: Design system health check and drift detection
 * - Figma: Preserved Figma import (legacy)
 * - Settings: Per-provider credentials and preferences
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";

interface DesignHubPanelProps {
  workspacePath: string | null;
  provider: string;
}

type HubTab = "providers" | "tokens" | "audit" | "figma" | "settings";

const TAB_DEFS: { id: HubTab; label: string }[] = [
  { id: "providers", label: "Providers" },
  { id: "tokens", label: "Tokens" },
  { id: "audit", label: "Audit" },
  { id: "figma", label: "Figma" },
  { id: "settings", label: "Settings" },
];

const PROVIDERS = [
  { id: "penpot", label: "Penpot", icon: "palette", desc: "Open-source Figma alternative", status: "active" },
  { id: "figma", label: "Figma", icon: "pen-tool", desc: "Figma design import (API token required)", status: "active" },
  { id: "pencil", label: "Pencil", icon: "edit", desc: "Evolus Pencil .ep wireframes", status: "active" },
  { id: "drawio", label: "Draw.io", icon: "chart-bar", desc: "Draw.io / diagrams.net editor", status: "active" },
  { id: "mermaid", label: "Mermaid", icon: "git-graph", desc: "AI-generated Mermaid diagrams", status: "active" },
  { id: "inhouse", label: "Built-in", icon: "zap", desc: "VibeCody built-in design system", status: "active" },
] as const;

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "7px 14px",
  fontSize: 12,
  fontWeight: active ? 600 : 400,
  cursor: "pointer",
  border: "none",
  borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  background: "transparent",
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  whiteSpace: "nowrap",
});

interface DesignToken { name: string; token_type: string; value: string; provider: string; }
interface AuditIssue { severity: string; code: string; message: string; }
interface AuditReport { score: number; summary: string; issues: AuditIssue[]; }

export function DesignHubPanel({ workspacePath, provider }: DesignHubPanelProps) {
  const [activeTab, setActiveTab] = useState<HubTab>("providers");
  const [activeProviders, setActiveProviders] = useState<string[]>(["inhouse"]);
  const [tokens, setTokens] = useState<DesignToken[]>([]);
  const [auditReport, setAuditReport] = useState<AuditReport | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [tokenExportFormat, setTokenExportFormat] = useState("css");
  const [tokenExportResult, setTokenExportResult] = useState("");
  const [figmaUrl, setFigmaUrl] = useState("");
  const [figmaToken, setFigmaToken] = useState(() => localStorage.getItem("figma_token") ?? "");
  const [figmaSaveToken, setFigmaSaveToken] = useState(() => !!localStorage.getItem("figma_token"));
  const [figmaResult, setFigmaResult] = useState<Array<{ path: string; content: string }>>([]);
  const [figmaExpandedFile, setFigmaExpandedFile] = useState<string | null>(null);
  const [statusMsg, setStatusMsg] = useState("");

  const showStatus = (msg: string) => {
    setStatusMsg(msg);
    setTimeout(() => setStatusMsg(""), 3000);
  };

  const toggleProvider = (id: string) => {
    setActiveProviders((prev) =>
      prev.includes(id) ? prev.filter((p) => p !== id) : [...prev, id]
    );
  };

  const loadTokens = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<{ tokens: DesignToken[] }>("load_design_system_tokens", {
        providers: activeProviders,
        workspacePath,
      }).catch(() => ({ tokens: [] }));
      setTokens(result.tokens);
      showStatus(`Loaded ${result.tokens.length} token(s)`);
    } finally {
      setIsLoading(false);
    }
  };

  const exportTokens = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<string>("export_design_tokens", {
        tokens,
        format: tokenExportFormat,
        systemName: "VibeCody Design System",
      }).catch((e: unknown) => String(e));
      setTokenExportResult(result);
    } finally {
      setIsLoading(false);
    }
  };

  const runAudit = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<AuditReport>("audit_design_system_tokens", {
        tokens,
        systemName: "VibeCody",
      }).catch(() => null);
      if (result) {
        setAuditReport(result);
        showStatus(`Audit complete — score: ${result.score}/100`);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleFigmaImport = async () => {
    if (!figmaUrl.trim() || !figmaToken.trim()) return;
    if (figmaSaveToken) localStorage.setItem("figma_token", figmaToken);
    else localStorage.removeItem("figma_token");
    setIsLoading(true);
    setFigmaResult([]);
    setFigmaExpandedFile(null);
    try {
      const files = await invoke<Array<{ path: string; content: string }>>("import_figma", {
        url: figmaUrl, token: figmaToken, workspacePath, provider,
      }).catch(() => []);
      setFigmaResult(files);
      showStatus(`${files.length} component(s) generated`);
    } finally {
      setIsLoading(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text).then(() => showStatus("Copied!")).catch(() => {});
  };

  // ── Render ────────────────────────────────────────────────────────────

  const renderProviders = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 4 }}>Design Providers</div>
      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.6 }}>
        Enable providers to aggregate tokens and components across design tools.
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))", gap: 10, marginBottom: 20 }}>
        {PROVIDERS.map((p) => {
          const enabled = activeProviders.includes(p.id);
          return (
            <div
              key={p.id}
              onClick={() => toggleProvider(p.id)}
              style={{
                padding: "14px 16px",
                background: enabled ? "var(--bg-elevated)" : "var(--bg-secondary)",
                border: `1px solid ${enabled ? "var(--accent-blue)" : "var(--border-color)"}`,
                borderRadius: 10,
                cursor: "pointer",
                display: "flex",
                gap: 12,
                alignItems: "flex-start",
                transition: "all 0.15s",
              }}
            >
              <Icon name={p.icon} size={20} style={{ flexShrink: 0, marginTop: 2 }} />
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 2 }}>{p.label}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", lineHeight: 1.4 }}>{p.desc}</div>
              </div>
              <div style={{
                width: 16, height: 16, borderRadius: "50%", border: "2px solid var(--border-color)",
                background: enabled ? "var(--accent-blue)" : "transparent",
                flexShrink: 0, marginTop: 2,
              }} />
            </div>
          );
        })}
      </div>
      <button
        onClick={loadTokens}
        disabled={isLoading || activeProviders.length === 0}
        style={{ background: "var(--accent-blue)", color: "#fff", border: "none", borderRadius: 6, padding: "10px 24px", cursor: "pointer", fontWeight: 600, fontSize: 14, opacity: isLoading || activeProviders.length === 0 ? 0.5 : 1 }}
      >
        {isLoading ? "Loading…" : "Load Design Tokens"}
      </button>
      {tokens.length > 0 && (
        <div style={{ marginTop: 12, fontSize: 12, color: "var(--text-success)" }}>
          ✓ {tokens.length} token(s) loaded from {activeProviders.length} provider(s)
        </div>
      )}
    </div>
  );

  const renderTokens = () => (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
      <div style={{ padding: "8px 16px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 6, alignItems: "center", flexShrink: 0 }}>
        <span style={{ fontSize: 12, fontWeight: 600 }}>Tokens ({tokens.length})</span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 4 }}>
          {["css", "tailwind", "typescript", "json"].map((f) => (
            <button key={f} onClick={() => setTokenExportFormat(f)}
              style={{ background: tokenExportFormat === f ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 4, padding: "3px 8px", cursor: "pointer", color: tokenExportFormat === f ? "#fff" : "inherit", fontSize: 11, fontWeight: tokenExportFormat === f ? 600 : 400 }}
            >{f.toUpperCase()}</button>
          ))}
          <button onClick={exportTokens} disabled={tokens.length === 0}
            style={{ background: "var(--accent-blue)", border: "none", borderRadius: 4, padding: "3px 8px", cursor: "pointer", color: "#fff", fontSize: 11, fontWeight: 600 }}
          >Export</button>
          <button onClick={runAudit} disabled={tokens.length === 0}
            style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 4, padding: "3px 8px", cursor: "pointer", color: "inherit", fontSize: 11 }}
          >Audit</button>
        </div>
      </div>
      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {tokens.length === 0 ? (
          <div style={{ color: "var(--text-secondary)", fontSize: 13, textAlign: "center", padding: 32 }}>
            Enable providers and click "Load Design Tokens".
          </div>
        ) : (
          <>
            {tokens.slice(0, 50).map((t, i) => (
              <div key={i} style={{ display: "flex", gap: 12, alignItems: "center", padding: "6px 0", borderBottom: "1px solid var(--border-color)" }}>
                {t.token_type === "color" && (
                  <div style={{ width: 20, height: 20, background: t.value, borderRadius: 4, border: "1px solid var(--border-color)", flexShrink: 0 }} />
                )}
                <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, flex: 1 }}>{t.name}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{t.value.slice(0, 30)}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", minWidth: 60, textAlign: "right" }}>{t.provider}</div>
              </div>
            ))}
            {tokens.length > 50 && <div style={{ fontSize: 12, color: "var(--text-secondary)", padding: "8px 0" }}>…and {tokens.length - 50} more</div>}
            {tokenExportResult && (
              <div style={{ marginTop: 16 }}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                  <div style={{ fontWeight: 600, fontSize: 13 }}>Exported ({tokenExportFormat.toUpperCase()})</div>
                  <button onClick={() => copyToClipboard(tokenExportResult)}
                    style={{ background: "none", border: "1px solid var(--border-color)", borderRadius: 4, padding: "2px 8px", cursor: "pointer", color: "inherit", fontSize: 11 }}>Copy</button>
                </div>
                <pre style={{ fontSize: 11, overflow: "auto", maxHeight: 400, background: "var(--bg-secondary)", borderRadius: 6, padding: 12, border: "1px solid var(--border-color)", whiteSpace: "pre-wrap" }}>
                  {tokenExportResult}
                </pre>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );

  const renderAudit = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 4 }}>Design System Audit</div>
      {!auditReport ? (
        <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>
          Load tokens first, then run audit from the Tokens tab.
        </div>
      ) : (
        <>
          <div style={{ display: "flex", gap: 16, marginBottom: 20 }}>
            <div style={{ padding: 20, background: "var(--bg-secondary)", borderRadius: 10, border: "1px solid var(--border-color)", textAlign: "center", minWidth: 100 }}>
              <div style={{ fontSize: 36, fontWeight: 800, color: auditReport.score >= 80 ? "var(--text-success)" : auditReport.score >= 60 ? "var(--warning-color)" : "var(--error-color, #f85149)" }}>
                {auditReport.score}
              </div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>out of 100</div>
            </div>
            <div style={{ flex: 1, padding: "12px 0" }}>
              <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 4 }}>Summary</div>
              <div style={{ fontSize: 13, lineHeight: 1.6 }}>{auditReport.summary}</div>
            </div>
          </div>
          {auditReport.issues.map((issue, i) => (
            <div key={i} style={{ marginBottom: 8, padding: "10px 14px", background: "var(--bg-secondary)", borderRadius: 8, borderLeft: `3px solid ${issue.severity === "Error" ? "var(--error-color, #f85149)" : issue.severity === "Warning" ? "var(--warning-color)" : "var(--accent-blue)"}` }}>
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 2 }}>{issue.code}</div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{issue.message}</div>
            </div>
          ))}
          {auditReport.issues.length === 0 && (
            <div style={{ padding: 20, textAlign: "center", color: "var(--text-success)", fontSize: 14, fontWeight: 600 }}>
              ✓ All checks passed!
            </div>
          )}
        </>
      )}
    </div>
  );

  const renderFigma = () => {
    const steps = ["Connect", "Generate", "Review"];
    const currentStep = figmaResult.length > 0 ? 2 : isLoading ? 1 : 0;
    const btnDisabled = isLoading || !figmaUrl.trim() || !figmaToken.trim();
    return (
      <div style={{ flex: 1, overflow: "auto", padding: "14px 16px" }}>
        {/* Workflow steps */}
        <div style={{ display: "flex", alignItems: "center", marginBottom: 14 }}>
          {steps.map((s, i) => (
            <div key={s} style={{ display: "flex", alignItems: "center", flex: i < steps.length - 1 ? 1 : undefined }}>
              <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 2 }}>
                <div style={{
                  width: 20, height: 20, borderRadius: "50%", fontSize: 10, fontWeight: 700,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  background: i <= currentStep ? "var(--accent-blue)" : "var(--bg-secondary)",
                  color: i <= currentStep ? "#fff" : "var(--text-secondary)",
                  border: `1px solid ${i <= currentStep ? "var(--accent-blue)" : "var(--border-color)"}`,
                }}>{i + 1}</div>
                <div style={{ fontSize: 9, color: i <= currentStep ? "var(--text-primary)" : "var(--text-secondary)", whiteSpace: "nowrap" }}>{s}</div>
              </div>
              {i < steps.length - 1 && (
                <div style={{ flex: 1, height: 1, background: i < currentStep ? "var(--accent-blue)" : "var(--border-color)", margin: "0 4px", marginBottom: 12 }} />
              )}
            </div>
          ))}
        </div>

        {/* Form card */}
        <div style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", padding: "12px 14px", marginBottom: 10 }}>
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 10, lineHeight: 1.5 }}>
            Get your token from <em>Figma → Settings → Personal access tokens</em>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 3 }}>Figma File URL</div>
              <input
                value={figmaUrl}
                onChange={(e) => setFigmaUrl(e.target.value)}
                placeholder="https://www.figma.com/file/…"
                style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 5, color: "inherit", padding: "5px 8px", fontSize: 12, boxSizing: "border-box" as const }}
              />
            </div>
            <div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 3 }}>Personal Access Token</div>
              <input
                type="password"
                value={figmaToken}
                onChange={(e) => setFigmaToken(e.target.value)}
                placeholder="figd_…"
                style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 5, color: "inherit", padding: "5px 8px", fontSize: 12, boxSizing: "border-box" as const }}
              />
            </div>
            <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--text-secondary)", cursor: "pointer" }}>
              <input
                type="checkbox"
                checked={figmaSaveToken}
                onChange={(e) => setFigmaSaveToken(e.target.checked)}
              />
              Remember token on this device
            </label>
          </div>
        </div>

        <button
          onClick={handleFigmaImport}
          disabled={btnDisabled}
          style={{ width: "100%", background: "var(--accent-blue)", color: "#fff", border: "none", borderRadius: 6, padding: "8px 0", cursor: btnDisabled ? "not-allowed" : "pointer", fontWeight: 600, fontSize: 13, opacity: btnDisabled ? 0.5 : 1, marginBottom: 14 }}
        >
          {isLoading ? "Importing…" : "Import & Generate Components"}
        </button>

        {/* Results */}
        {figmaResult.length > 0 && (
          <div>
            <div style={{ fontSize: 12, color: "var(--text-success)", fontWeight: 600, marginBottom: 8 }}>
              <Icon name="check" size={12} style={{ verticalAlign: "middle", marginRight: 4 }} />
              {figmaResult.length} component{figmaResult.length > 1 ? "s" : ""} generated — click a file to preview
            </div>
            {figmaResult.map((f) => (
              <div key={f.path} style={{ marginBottom: 6, borderRadius: 6, border: "1px solid var(--border-color)", overflow: "hidden" }}>
                <div
                  onClick={() => setFigmaExpandedFile(figmaExpandedFile === f.path ? null : f.path)}
                  style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 10px", background: "var(--bg-secondary)", cursor: "pointer" }}
                >
                  <Icon name="file-code" size={12} style={{ flexShrink: 0, color: "var(--text-secondary)" }} />
                  <span style={{ flex: 1, fontSize: 11, fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.path}</span>
                  <button
                    onClick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(f.content); showStatus("Copied!"); }}
                    title="Copy code"
                    style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: 10, padding: "2px 6px", borderRadius: 3 }}
                  >
                    Copy
                  </button>
                  <button
                    onClick={(e) => { e.stopPropagation(); copyToClipboard(f.content); }}
                    title="Preview component"
                    style={{ background: "none", border: "1px solid var(--accent-blue)", color: "var(--accent-blue)", cursor: "pointer", fontSize: 10, padding: "2px 6px", borderRadius: 3 }}
                  >
                    Open
                  </button>
                </div>
                {figmaExpandedFile === f.path && (
                  <pre style={{ margin: 0, padding: "10px 12px", fontSize: 10, lineHeight: 1.5, overflow: "auto", maxHeight: 220, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>
                    <code>{f.content}</code>
                  </pre>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    );
  };

  const renderSettings = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>Provider Settings</div>
      {PROVIDERS.map((p) => (
        <div key={p.id} style={{ marginBottom: 12, padding: "12px 14px", background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)" }}>
          <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 4, display: "flex", alignItems: "center", gap: 6 }}><Icon name={p.icon} size={14} /> {p.label}</div>
          <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>{p.desc}</div>
          {p.id === "penpot" && <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Configure via the Penpot tab</div>}
          {p.id === "figma" && <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Configure via the Figma tab (API token per-import)</div>}
          {(p.id === "pencil" || p.id === "drawio" || p.id === "mermaid" || p.id === "inhouse") && (
            <div style={{ fontSize: 11, color: "var(--text-success)" }}>✓ No authentication required</div>
          )}
        </div>
      ))}
    </div>
  );

  return (
    <div className="panel-container">
      <div className="panel-header" style={{ padding: 0, overflow: "auto", flexShrink: 0 }}>
        {TAB_DEFS.map(({ id, label }) => (
          <button key={id} onClick={() => setActiveTab(id)} style={tabStyle(activeTab === id)}>{label}</button>
        ))}
        {statusMsg && <span style={{ marginLeft: "auto", marginRight: 12, fontSize: 11, color: "var(--text-success)", lineHeight: "30px" }}>✓ {statusMsg}</span>}
      </div>
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {activeTab === "providers" && renderProviders()}
        {activeTab === "tokens" && renderTokens()}
        {activeTab === "audit" && renderAudit()}
        {activeTab === "figma" && renderFigma()}
        {activeTab === "settings" && renderSettings()}
      </div>
    </div>
  );
}
