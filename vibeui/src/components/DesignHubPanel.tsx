/**
 * DesignHubPanel — unified multi-provider design hub.
 *
 * Tabs: Providers | Tokens | Audit | Figma | Settings
 * - Providers: Switch between Figma, Penpot, Pencil, Draw.io, Mermaid, Built-in
 * - Tokens: Cross-provider token browser with CSS/Tailwind/JSON export + filter
 * - Audit: Design system health check and drift detection
 * - Figma: Figma import — token persisted in ProfileStore (NOT localStorage)
 * - Settings: Per-provider credentials and preferences
 *
 * Security: the Figma personal access token is stored via Tauri profile_api_key_*
 * commands, which write through the encrypted ProfileStore. We never touch
 * localStorage for credential material — see AGENTS.md "Secure Settings Storage".
 */
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";
import { useToast } from "../hooks/useToast";
import { usePanelSettings } from "../hooks/usePanelSettings";
import { Toaster } from "./Toaster";

interface DesignHubPanelProps {
  workspacePath: string | null;
  provider: string;
}

type HubTab = "providers" | "tokens" | "audit" | "figma" | "settings";

const PROFILE_ID = "default";
const FIGMA_KEY_PROVIDER = "figma";

const TAB_DEFS: { id: HubTab; label: string }[] = [
  { id: "providers", label: "Providers" },
  { id: "tokens", label: "Tokens" },
  { id: "audit", label: "Audit" },
  { id: "figma", label: "Figma" },
  { id: "settings", label: "Settings" },
];

const PROVIDERS = [
  { id: "penpot", label: "Penpot", icon: "palette", desc: "Open-source Figma alternative" },
  { id: "figma", label: "Figma", icon: "pen-tool", desc: "Figma design import (API token required)" },
  { id: "pencil", label: "Pencil", icon: "edit", desc: "Evolus Pencil .ep wireframes" },
  { id: "drawio", label: "Draw.io", icon: "chart-bar", desc: "Draw.io / diagrams.net editor" },
  { id: "mermaid", label: "Mermaid", icon: "git-graph", desc: "AI-generated Mermaid diagrams" },
  { id: "inhouse", label: "Built-in", icon: "zap", desc: "VibeCody built-in design system" },
] as const;

interface DesignToken { name: string; token_type: string; value: string; provider: string; }
interface AuditIssue { severity: string; code: string; message: string; }
interface AuditReport { score: number; summary: string; issues: AuditIssue[]; }

export function DesignHubPanel({ workspacePath, provider }: DesignHubPanelProps) {
  const { toasts, toast, dismiss } = useToast();
  const { settings, setSetting, loading: settingsLoading } = usePanelSettings("design-hub");
  const [activeTab, setActiveTabState] = useState<HubTab>("providers");
  const [activeProviders, setActiveProvidersState] = useState<string[]>(["inhouse"]);
  const [hydrated, setHydrated] = useState(false);

  // Hydrate from panel_settings_get_all once it has resolved.
  useEffect(() => {
    if (settingsLoading || hydrated) return;
    const tab = settings.activeTab as HubTab | undefined;
    const provs = settings.activeProviders as string[] | undefined;
    if (tab && TAB_DEFS.some((t) => t.id === tab)) setActiveTabState(tab);
    if (Array.isArray(provs) && provs.length > 0) setActiveProvidersState(provs);
    setHydrated(true);
  }, [settings, settingsLoading, hydrated]);

  const setActiveTab = (next: HubTab) => {
    setActiveTabState(next);
    void setSetting("activeTab", next);
  };

  const setActiveProviders = (updater: (prev: string[]) => string[]) => {
    setActiveProvidersState((prev) => {
      const next = updater(prev);
      void setSetting("activeProviders", next);
      return next;
    });
  };
  const [tokens, setTokens] = useState<DesignToken[]>([]);
  const [tokenFilter, setTokenFilter] = useState("");
  const [auditReport, setAuditReport] = useState<AuditReport | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [tokenExportFormat, setTokenExportFormat] = useState("css");
  const [tokenExportResult, setTokenExportResult] = useState("");
  const [figmaUrl, setFigmaUrl] = useState("");
  const [figmaToken, setFigmaToken] = useState("");
  const [figmaSaveToken, setFigmaSaveToken] = useState(false);
  const [figmaResult, setFigmaResult] = useState<Array<{ path: string; content: string }>>([]);
  const [figmaExpandedFile, setFigmaExpandedFile] = useState<string | null>(null);

  // Hydrate the Figma token from the encrypted ProfileStore on mount.
  useEffect(() => {
    let cancelled = false;
    invoke<string | null>("profile_api_key_get", {
      profile_id: PROFILE_ID, profileId: PROFILE_ID,
      provider: FIGMA_KEY_PROVIDER,
    })
      .then((value) => {
        if (cancelled) return;
        if (typeof value === "string" && value.length > 0) {
          setFigmaToken(value);
          setFigmaSaveToken(true);
        }
      })
      .catch((e) => toast.error(`Failed to load Figma token: ${e}`));
    return () => { cancelled = true; };
    // toast is reconstructed each render; we deliberately want this to fire once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
        workspace_path: workspacePath,
      });
      setTokens(result.tokens);
      toast.success(`Loaded ${result.tokens.length} token(s)`);
    } catch (e) {
      toast.error(`Failed to load tokens: ${e}`);
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
        system_name: "VibeCody Design System",
      });
      setTokenExportResult(result);
      toast.success(`Exported ${tokens.length} token(s) as ${tokenExportFormat.toUpperCase()}`);
    } catch (e) {
      toast.error(`Export failed: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };

  const runAudit = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<AuditReport>("audit_design_system_tokens", {
        tokens, systemName: "VibeCody", system_name: "VibeCody",
      });
      setAuditReport(result);
      toast.success(`Audit complete — score: ${result.score}/100`);
    } catch (e) {
      toast.error(`Audit failed: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };

  const persistFigmaToken = async () => {
    try {
      if (figmaSaveToken) {
        await invoke("profile_api_key_set", {
          profile_id: PROFILE_ID, profileId: PROFILE_ID,
          provider: FIGMA_KEY_PROVIDER,
          api_key: figmaToken, apiKey: figmaToken,
        });
      } else {
        await invoke("profile_api_key_delete", {
          profile_id: PROFILE_ID, profileId: PROFILE_ID,
          provider: FIGMA_KEY_PROVIDER,
        });
      }
    } catch (e) {
      toast.error(`Failed to persist Figma token: ${e}`);
    }
  };

  const handleFigmaImport = async () => {
    if (!figmaUrl.trim() || !figmaToken.trim()) return;
    await persistFigmaToken();
    setIsLoading(true);
    setFigmaResult([]);
    setFigmaExpandedFile(null);
    try {
      const files = await invoke<Array<{ path: string; content: string }>>("import_figma", {
        url: figmaUrl, token: figmaToken,
        workspacePath, workspace_path: workspacePath,
        provider,
      });
      setFigmaResult(files);
      toast.success(`${files.length} component(s) generated`);
    } catch (e) {
      toast.error(`Figma import failed: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
      .then(() => toast.info("Copied to clipboard"))
      .catch((e) => toast.error(`Copy failed: ${e}`));
  };

  const filteredTokens = tokenFilter.trim() === ""
    ? tokens
    : tokens.filter((t) => {
        const q = tokenFilter.toLowerCase();
        return t.name.toLowerCase().includes(q) || t.value.toLowerCase().includes(q);
      });

  // ── Render ────────────────────────────────────────────────────────────

  const renderProviders = () => (
    <div style={{ flex: 1, overflow: "auto", padding: "var(--space-4)" }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: "var(--space-1)" }}>
        Design Providers
      </div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: "var(--space-4)", lineHeight: 1.6 }}>
        Enable providers to aggregate tokens and components across design tools.
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))", gap: "var(--space-2)", marginBottom: "var(--space-5)" }}>
        {PROVIDERS.map((p) => {
          const enabled = activeProviders.includes(p.id);
          return (
            <button
              key={p.id}
              type="button"
              onClick={() => toggleProvider(p.id)}
              aria-pressed={enabled}
              aria-label={`${p.label} provider — ${enabled ? "enabled" : "disabled"}`}
              className="panel-card"
              style={{
                padding: "var(--space-3) var(--space-4)",
                background: enabled ? "var(--bg-elevated, var(--bg-secondary))" : "var(--bg-secondary)",
                border: `1px solid ${enabled ? "var(--accent-blue)" : "var(--border-color)"}`,
                borderRadius: "var(--radius-md)",
                cursor: "pointer",
                display: "flex",
                gap: "var(--space-3)",
                alignItems: "flex-start",
                textAlign: "left",
                font: "inherit",
                color: "inherit",
                width: "100%",
              }}
            >
              <Icon name={p.icon} size={20} style={{ flexShrink: 0, marginTop: 2 }} />
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 2 }}>{p.label}</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.4 }}>{p.desc}</div>
              </div>
              <div aria-hidden style={{
                width: 16, height: 16, borderRadius: "50%", border: "2px solid var(--border-color)",
                background: enabled ? "var(--accent-blue)" : "transparent",
                flexShrink: 0, marginTop: 2,
              }} />
            </button>
          );
        })}
      </div>
      <button
        className="panel-btn panel-btn-primary"
        onClick={loadTokens}
        disabled={isLoading || activeProviders.length === 0}
      >
        {isLoading ? "Loading…" : "Load Design Tokens"}
      </button>
      {tokens.length > 0 && (
        <div style={{ marginTop: "var(--space-3)", fontSize: "var(--font-size-base)", color: "var(--text-success)" }}>
          ✓ {tokens.length} token(s) loaded from {activeProviders.length} provider(s)
        </div>
      )}
    </div>
  );

  const renderTokens = () => (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
      <div style={{ padding: "var(--space-2) var(--space-4)", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: "var(--space-2)", alignItems: "center", flexShrink: 0, flexWrap: "wrap" }}>
        <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600 }}>Tokens ({filteredTokens.length}/{tokens.length})</span>
        <input
          aria-label="Filter tokens"
          placeholder="Filter tokens…"
          value={tokenFilter}
          onChange={(e) => setTokenFilter(e.target.value)}
          className="panel-input"
          style={{ flex: 1, minWidth: 160, padding: "4px 8px", fontSize: "var(--font-size-sm)" }}
        />
        <div style={{ display: "flex", gap: "var(--space-1)" }}>
          {["css", "tailwind", "typescript", "json"].map((f) => (
            <button
              key={f}
              type="button"
              onClick={() => setTokenExportFormat(f)}
              className={`panel-btn panel-btn-sm ${tokenExportFormat === f ? "panel-btn-primary" : "panel-btn-secondary"}`}
            >
              {f.toUpperCase()}
            </button>
          ))}
          <button type="button" onClick={exportTokens} disabled={tokens.length === 0} className="panel-btn panel-btn-primary panel-btn-sm">Export</button>
          <button type="button" onClick={runAudit} disabled={tokens.length === 0} className="panel-btn panel-btn-secondary panel-btn-sm">Audit</button>
        </div>
      </div>
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-4)" }}>
        {tokens.length === 0 ? (
          <div className="panel-empty">
            Enable providers and click "Load Design Tokens".
          </div>
        ) : filteredTokens.length === 0 ? (
          <div className="panel-empty">
            No tokens match "{tokenFilter}".
          </div>
        ) : (
          <>
            {filteredTokens.map((t, i) => (
              <div key={`${t.provider}:${t.name}:${i}`} style={{ display: "flex", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-2) 0", borderBottom: "1px solid var(--border-color)" }}>
                {t.token_type === "color" && (
                  <div style={{ width: 20, height: 20, background: t.value, borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", flexShrink: 0 }} />
                )}
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", flex: 1 }}>{t.name}</div>
                <div
                  title={t.value}
                  style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", maxWidth: 220, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                >
                  {t.value}
                </div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", minWidth: 60, textAlign: "right" }}>{t.provider}</div>
              </div>
            ))}
            {tokenExportResult && (
              <div style={{ marginTop: "var(--space-4)" }}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "var(--space-2)" }}>
                  <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Exported ({tokenExportFormat.toUpperCase()})</div>
                  <button type="button" onClick={() => copyToClipboard(tokenExportResult)} className="panel-btn panel-btn-secondary panel-btn-sm">Copy</button>
                </div>
                <pre style={{ fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 400, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: "var(--space-3)", border: "1px solid var(--border-color)", whiteSpace: "pre-wrap" }}>
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
    <div style={{ flex: 1, overflow: "auto", padding: "var(--space-4)" }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: "var(--space-1)" }}>Design System Audit</div>
      {!auditReport ? (
        <div className="panel-empty">
          Load tokens first, then run audit from the Tokens tab.
        </div>
      ) : (
        <>
          <div style={{ display: "flex", gap: "var(--space-4)", marginBottom: "var(--space-5)" }}>
            <div style={{ padding: "var(--space-5)", background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", textAlign: "center", minWidth: 100 }}>
              <div style={{
                fontSize: "var(--font-size-3xl)",
                fontWeight: 800,
                color: auditReport.score >= 80 ? "var(--text-success)" : auditReport.score >= 60 ? "var(--warning-color)" : "var(--error-color)",
              }}>
                {auditReport.score}
              </div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>out of 100</div>
            </div>
            <div style={{ flex: 1, padding: "var(--space-3) 0" }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: "var(--space-1)" }}>Summary</div>
              <div style={{ fontSize: "var(--font-size-md)", lineHeight: 1.6 }}>{auditReport.summary}</div>
            </div>
          </div>
          {auditReport.issues.map((issue, i) => (
            <div key={i} style={{
              marginBottom: "var(--space-2)",
              padding: "var(--space-3) var(--space-4)",
              background: "var(--bg-secondary)",
              borderRadius: "var(--radius-sm-alt)",
              borderLeft: `3px solid ${issue.severity === "Error" ? "var(--error-color)" : issue.severity === "Warning" ? "var(--warning-color)" : "var(--accent-blue)"}`,
            }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 2 }}>{issue.code}</div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{issue.message}</div>
            </div>
          ))}
          {auditReport.issues.length === 0 && (
            <div style={{ padding: "var(--space-5)", textAlign: "center", color: "var(--text-success)", fontSize: "var(--font-size-lg)", fontWeight: 600 }}>
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
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-4)" }}>
        {/* Workflow steps */}
        <div style={{ display: "flex", alignItems: "center", marginBottom: "var(--space-4)" }}>
          {steps.map((s, i) => (
            <div key={s} style={{ display: "flex", alignItems: "center", flex: i < steps.length - 1 ? 1 : undefined }}>
              <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 2 }}>
                <div style={{
                  width: 20, height: 20, borderRadius: "50%", fontSize: "var(--font-size-xs)", fontWeight: 700,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  background: i <= currentStep ? "var(--accent-blue)" : "var(--bg-secondary)",
                  color: i <= currentStep ? "var(--btn-primary-fg, var(--text-primary))" : "var(--text-secondary)",
                  border: `1px solid ${i <= currentStep ? "var(--accent-blue)" : "var(--border-color)"}`,
                }}>{i + 1}</div>
                <div style={{ fontSize: "var(--font-size-xs)", color: i <= currentStep ? "var(--text-primary)" : "var(--text-secondary)", whiteSpace: "nowrap" }}>{s}</div>
              </div>
              {i < steps.length - 1 && (
                <div style={{ flex: 1, height: 1, background: i < currentStep ? "var(--accent-blue)" : "var(--border-color)", margin: "0 4px", marginBottom: 12 }} />
              )}
            </div>
          ))}
        </div>

        {/* Form card */}
        <div className="panel-card" style={{ padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-3)" }}>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: "var(--space-3)", lineHeight: 1.5 }}>
            Get your token from <em>Figma → Settings → Personal access tokens</em>. Stored encrypted in your VibeCody profile.
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
            <div>
              <label htmlFor="figma-url" style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 3 }}>Figma File URL</label>
              <input
                id="figma-url"
                className="panel-input"
                value={figmaUrl}
                onChange={(e) => setFigmaUrl(e.target.value)}
                placeholder="https://www.figma.com/file/…"
                style={{ width: "100%", boxSizing: "border-box" }}
              />
            </div>
            <div>
              <label htmlFor="figma-token" style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 3 }}>Personal Access Token</label>
              <input
                id="figma-token"
                className="panel-input"
                type="password"
                value={figmaToken}
                onChange={(e) => setFigmaToken(e.target.value)}
                placeholder="figd_…"
                style={{ width: "100%", boxSizing: "border-box" }}
              />
            </div>
            <label style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", cursor: "pointer" }}>
              <input
                type="checkbox"
                checked={figmaSaveToken}
                onChange={(e) => setFigmaSaveToken(e.target.checked)}
              />
              Remember token (encrypted in profile)
            </label>
          </div>
        </div>

        <button
          type="button"
          className="panel-btn panel-btn-primary"
          onClick={handleFigmaImport}
          disabled={btnDisabled}
          style={{ width: "100%", marginBottom: "var(--space-4)" }}
        >
          {isLoading ? "Importing…" : "Import & Generate Components"}
        </button>

        {/* Results */}
        {figmaResult.length > 0 && (
          <div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-success)", fontWeight: 600, marginBottom: "var(--space-2)" }}>
              <Icon name="check" size={12} style={{ verticalAlign: "middle", marginRight: 4 }} />
              {figmaResult.length} component{figmaResult.length > 1 ? "s" : ""} generated — click a file to preview
            </div>
            {figmaResult.map((f) => (
              <div key={f.path} style={{ marginBottom: "var(--space-2)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", overflow: "hidden" }}>
                <button
                  type="button"
                  onClick={() => setFigmaExpandedFile(figmaExpandedFile === f.path ? null : f.path)}
                  style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "8px 12px", background: "var(--bg-secondary)", cursor: "pointer", width: "100%", border: "none", color: "inherit", font: "inherit", textAlign: "left" }}
                >
                  <Icon name="file-code" size={12} style={{ flexShrink: 0, color: "var(--text-secondary)" }} />
                  <span style={{ flex: 1, fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.path}</span>
                  <span
                    role="button"
                    aria-label={`Copy ${f.path}`}
                    tabIndex={0}
                    onClick={(e) => { e.stopPropagation(); copyToClipboard(f.content); }}
                    onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.stopPropagation(); copyToClipboard(f.content); } }}
                    style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: 3, color: "var(--text-secondary)" }}
                  >
                    Copy
                  </span>
                </button>
                {figmaExpandedFile === f.path && (
                  <pre style={{ margin: 0, padding: "var(--space-3) var(--space-4)", fontSize: "var(--font-size-xs)", lineHeight: 1.5, overflow: "auto", maxHeight: 220, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>
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
    <div style={{ flex: 1, overflow: "auto", padding: "var(--space-4)" }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: "var(--space-3)" }}>Provider Settings</div>
      {PROVIDERS.map((p) => (
        <div key={p.id} className="panel-card" style={{ marginBottom: "var(--space-3)", padding: "var(--space-3) var(--space-4)" }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: "var(--space-1)", display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
            <Icon name={p.icon} size={14} /> {p.label}
          </div>
          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: "var(--space-2)" }}>{p.desc}</div>
          {p.id === "penpot" && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Configure via the Penpot tab</div>}
          {p.id === "figma" && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Token is stored encrypted in your VibeCody profile.</div>}
          {(p.id === "pencil" || p.id === "drawio" || p.id === "mermaid" || p.id === "inhouse") && (
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-success)" }}>✓ No authentication required</div>
          )}
        </div>
      ))}
    </div>
  );

  return (
    <div className="panel-container">
      <div className="panel-tab-bar" role="tablist" aria-label="Design hub tabs" style={{ flexShrink: 0 }}>
        {TAB_DEFS.map(({ id, label }) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={activeTab === id}
            onClick={() => setActiveTab(id)}
            className={`panel-tab ${activeTab === id ? "active" : ""}`}
          >
            {label}
          </button>
        ))}
      </div>
      <div role="tabpanel" aria-label={activeTab} style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {activeTab === "providers" && renderProviders()}
        {activeTab === "tokens" && renderTokens()}
        {activeTab === "audit" && renderAudit()}
        {activeTab === "figma" && renderFigma()}
        {activeTab === "settings" && renderSettings()}
      </div>
      <Toaster toasts={toasts} onDismiss={dismiss} />
    </div>
  );
}
