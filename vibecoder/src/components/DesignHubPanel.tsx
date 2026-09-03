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
 * Reads and writes go through `lib/figmaToken`, shared with DesignMode's Figma
 * tab so the two panels cannot drift apart on where the token lives.
 */
import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { loadFigmaToken, saveFigmaToken, deleteFigmaToken } from "../lib/figmaToken";
import { Icon } from "./Icon";
import { useToast } from "../hooks/useToast";
import { usePanelSettings } from "../hooks/usePanelSettings";
import { Toaster } from "./Toaster";
import { openDesignSubTab } from "../lib/panelDeepLink";
import { GeneratedFileList, type GeneratedFile } from "./design/GeneratedFileList";

interface DesignHubPanelProps {
  workspacePath: string | null;
  provider: string;
  onOpenFile?: (path: string, line?: number) => void;
}

type HubTab = "providers" | "tokens" | "audit" | "figma" | "settings";

const TAB_DEFS: { id: HubTab; label: string }[] = [
  { id: "providers", label: "Providers" },
  { id: "tokens", label: "Tokens" },
  { id: "audit", label: "Audit" },
  { id: "figma", label: "Figma" },
  { id: "settings", label: "Settings" },
];

/** File extension for each export format. */
const EXPORT_EXTENSION: Record<string, string> = {
  css: "css",
  tailwind: "js",
  typescript: "ts",
  json: "json",
};

/**
 * `DesignTokenType`'s serde names, as a reader would say them. Rendering
 * "border_radius" would be showing the wire format.
 */
const TOKEN_TYPE_LABEL: Record<string, string> = {
  color: "Color",
  typography: "Typography",
  spacing: "Spacing",
  border_radius: "Radius",
  shadow: "Shadow",
  animation: "Motion",
  breakpoint: "Breakpoint",
  z_index: "Z-index",
  other: "Other",
};

const PROVIDERS = [
  { id: "penpot", label: "Penpot", icon: "palette", desc: "Open-source Figma alternative" },
  { id: "figma", label: "Figma", icon: "pen-tool", desc: "Figma design import (API token required)" },
  { id: "pencil", label: "Pencil", icon: "edit", desc: "Evolus Pencil .ep wireframes" },
  { id: "drawio", label: "Draw.io", icon: "chart-bar", desc: "Draw.io / diagrams.net editor" },
  { id: "mermaid", label: "Mermaid", icon: "git-graph", desc: "AI-generated Mermaid diagrams" },
  { id: "inhouse", label: "Built-in", icon: "zap", desc: "VibeCody built-in design system" },
] as const;

/**
 * What configuring each provider actually involves. Every line here names a
 * place that exists — the Penpot line used to point at a "Penpot tab" this
 * panel has never had.
 */
const PROVIDER_SETTING_NOTE: Record<string, string> = {
  penpot: "Needs a Penpot instance URL and credentials, entered in the Penpot editor.",
  figma: "Needs a personal access token, stored encrypted in your VibeCody profile.",
  pencil: "Reads and writes .ep wireframes from the workspace — no account needed.",
  drawio: "Edits .drawio files in the workspace — no account needed.",
  mermaid: "Generates diagrams with the toolbar's selected model — no account needed.",
  inhouse: "Reads the CSS custom properties declared in this workspace — no account needed.",
};

/** Providers whose editor lives in Design → its own inner tab. */
const PROVIDER_TAB_LINK: Record<string, string> = {
  penpot: "penpot",
  pencil: "pencil",
  drawio: "drawio",
  mermaid: "diagrams",
};

interface DesignToken { name: string; token_type: string; value: string; provider: string; }
/** Mirrors `design_system_hub::AuditIssue`; severity is serde snake_case. */
interface AuditIssue {
  severity: "error" | "warning" | "info";
  code: string;
  message: string;
  affected: string[];
  suggestion?: string | null;
}

interface AuditReport {
  system_name: string;
  system_version: string;
  score: number;
  summary: string;
  issues: AuditIssue[];
  token_count?: number;
}

const SEVERITY_TONE: Record<AuditIssue["severity"], string> = {
  error: "var(--error-color)",
  warning: "var(--warning-color)",
  info: "var(--accent-blue)",
};

/**
 * What one enabled provider had to say. A provider that contributed nothing
 * reports *why* — an empty result and an unimplemented reader look identical
 * in a flat token list, and only one of them is the user's problem to fix.
 */
interface TokenSource {
  provider: string;
  status: "ok" | "unavailable" | "elsewhere" | "not_applicable" | "unknown";
  reason?: string;
  token_count?: number;
  files_scanned?: number;
  truncated?: boolean;
}

const SOURCE_TONE: Record<TokenSource["status"], string> = {
  ok: "var(--text-success)",
  unavailable: "var(--warning-color)",
  elsewhere: "var(--text-secondary)",
  not_applicable: "var(--text-secondary)",
  unknown: "var(--warning-color)",
};

export function DesignHubPanel({ workspacePath, provider, onOpenFile }: DesignHubPanelProps) {
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
  const [tokenSources, setTokenSources] = useState<TokenSource[]>([]);
  const [tokensLoaded, setTokensLoaded] = useState(false);
  const [tokenFilter, setTokenFilter] = useState("");
  const [auditReport, setAuditReport] = useState<AuditReport | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [tokenExportFormat, setTokenExportFormat] = useState("css");
  const [tokenExportResult, setTokenExportResult] = useState("");
  const [figmaUrl, setFigmaUrl] = useState("");
  const [figmaToken, setFigmaToken] = useState("");
  const [figmaSaveToken, setFigmaSaveToken] = useState(false);
  const [figmaResult, setFigmaResult] = useState<GeneratedFile[]>([]);

  // Hydrate the Figma token from the encrypted ProfileStore on mount. This
  // also drains any plaintext localStorage copy left by an older build.
  useEffect(() => {
    let cancelled = false;
    loadFigmaToken()
      .then((value) => {
        if (cancelled || !value) return;
        setFigmaToken(value);
        setFigmaSaveToken(true);
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
      const result = await invoke<{ tokens: DesignToken[]; sources: TokenSource[] }>(
        "load_design_system_tokens",
        {
          providers: activeProviders,
          workspacePath: workspacePath ?? "",
          workspace_path: workspacePath ?? "",
        },
      );
      setTokens(result.tokens ?? []);
      setTokenSources(result.sources ?? []);
      setTokensLoaded(true);
      // A zero here is a real answer — this workspace declares no custom
      // properties — so it is reported as one rather than as a failure.
      toast.success(`${result.tokens?.length ?? 0} token(s) from ${activeProviders.length} provider(s)`);
      setActiveTab("tokens");
    } catch (e) {
      setTokensLoaded(false);
      toast.error(`Failed to load tokens: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };

  const exportTokens = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<string>("export_design_tokens", {
        tokens: filteredTokens,
        format: tokenExportFormat,
        systemName: "VibeCody Design System",
        system_name: "VibeCody Design System",
      });
      setTokenExportResult(result);
      toast.success(`Exported ${filteredTokens.length} token(s) as ${tokenExportFormat.toUpperCase()}`);
    } catch (e) {
      toast.error(`Export failed: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };

  /** Write the current export to a file the user picks. */
  const saveExport = async () => {
    if (!tokenExportResult) return;
    const ext = EXPORT_EXTENSION[tokenExportFormat] ?? "txt";
    try {
      const path = await save({
        defaultPath: `design-tokens.${ext}`,
        filters: [{ name: tokenExportFormat.toUpperCase(), extensions: [ext] }],
      });
      if (!path) return; // user cancelled — not a failure
      await invoke("fullstack_write_file", { path, content: tokenExportResult });
      toast.success(`Saved ${path}`);
    } catch (e) {
      toast.error(`Save failed: ${e}`);
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
      setActiveTab("audit");
    } catch (e) {
      toast.error(`Audit failed: ${e}`);
    } finally {
      setIsLoading(false);
    }
  };

  const persistFigmaToken = async () => {
    try {
      if (figmaSaveToken) await saveFigmaToken(figmaToken);
      else await deleteFigmaToken();
    } catch (e) {
      toast.error(`Failed to persist Figma token: ${e}`);
    }
  };

  const handleFigmaImport = async () => {
    if (!figmaUrl.trim() || !figmaToken.trim()) return;
    if (!provider) {
      toast.error("No provider selected — pick one in the toolbar dropdown.");
      return;
    }
    await persistFigmaToken();
    setIsLoading(true);
    setFigmaResult([]);
    try {
      const files = await invoke<GeneratedFile[]>("import_figma", {
        url: figmaUrl, token: figmaToken,
        workspacePath: workspacePath ?? "", workspace_path: workspacePath ?? "",
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

  /** Jump to the editor that owns a provider's settings. */
  const openDesignTab = (subTabId: string) => {
    openDesignSubTab(subTabId);
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
      .then(() => toast.info("Copied to clipboard"))
      .catch((e) => toast.error(`Copy failed: ${e}`));
  };

  const filteredTokens = useMemo(() => {
    const q = tokenFilter.trim().toLowerCase();
    if (!q) return tokens;
    return tokens.filter(
      (t) => t.name.toLowerCase().includes(q) || t.value.toLowerCase().includes(q),
    );
  }, [tokens, tokenFilter]);

  /** One line per enabled provider explaining what it contributed. */
  const renderSources = () => {
    if (tokenSources.length === 0) return null;
    return (
      <div style={{ marginTop: "var(--space-3)", display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
        {tokenSources.map((src) => {
          const label = PROVIDERS.find((p) => p.id === src.provider)?.label ?? src.provider;
          const detail =
            src.status === "ok"
              ? `${src.token_count ?? 0} token(s) from ${src.files_scanned ?? 0} stylesheet(s)` +
                (src.truncated ? " — scan hit its file limit, this is a sample" : "")
              : (src.reason ?? "No detail reported.");
          return (
            <div
              key={src.provider}
              style={{ fontSize: "var(--font-size-sm)", color: SOURCE_TONE[src.status] ?? "var(--text-secondary)", lineHeight: 1.5 }}
            >
              <strong style={{ color: "var(--text-primary)" }}>{label}</strong>{" — "}{detail}
            </div>
          );
        })}
      </div>
    );
  };

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
      {tokensLoaded && renderSources()}
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
          <button type="button" onClick={exportTokens} disabled={filteredTokens.length === 0 || isLoading} className="panel-btn panel-btn-primary panel-btn-sm">Export</button>
          <button type="button" onClick={saveExport} disabled={!tokenExportResult} className="panel-btn panel-btn-secondary panel-btn-sm">Save…</button>
          <button type="button" onClick={runAudit} disabled={tokens.length === 0 || isLoading} className="panel-btn panel-btn-secondary panel-btn-sm">Audit</button>
        </div>
      </div>
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-4)" }}>
        {tokens.length === 0 ? (
          <div className="panel-empty">
            {tokensLoaded ? (
              <>
                <div style={{ marginBottom: "var(--space-2)" }}>No design tokens found.</div>
                <div style={{ textAlign: "left", display: "inline-block" }}>{renderSources()}</div>
              </>
            ) : (
              <>Enable providers on the Providers tab, then click “Load Design Tokens”.</>
            )}
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
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", minWidth: 84, textAlign: "right" }}>
                  {TOKEN_TYPE_LABEL[t.token_type] ?? t.token_type}
                </div>
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
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "var(--space-3)", gap: "var(--space-2)" }}>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>Design System Audit</div>
        <button
          type="button"
          className="panel-btn panel-btn-primary panel-btn-sm"
          onClick={runAudit}
          disabled={tokens.length === 0 || isLoading}
          title={tokens.length === 0 ? "Load design tokens first" : `Audit ${tokens.length} token(s)`}
        >
          {isLoading ? "Running…" : auditReport ? "Re-run Audit" : "Run Audit"}
        </button>
      </div>
      {!auditReport ? (
        <div className="panel-empty">
          {tokens.length === 0
            ? "No tokens loaded yet — load them on the Providers tab, then run the audit."
            : `${tokens.length} token(s) ready. Run the audit to score them.`}
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
              borderLeft: `3px solid ${SEVERITY_TONE[issue.severity] ?? "var(--accent-blue)"}`,
            }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 2, display: "flex", gap: "var(--space-2)", alignItems: "baseline" }}>
                <span>{issue.code}</span>
                <span style={{ fontSize: "var(--font-size-xs)", textTransform: "uppercase", color: SEVERITY_TONE[issue.severity] ?? "var(--text-secondary)" }}>
                  {issue.severity}
                </span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{issue.message}</div>
              {issue.affected.length > 0 && (
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4, fontFamily: "var(--font-mono)" }}>
                  {issue.affected.join(", ")}
                </div>
              )}
              {issue.suggestion && (
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>
                  → {issue.suggestion}
                </div>
              )}
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

        {/* Results — the shared review-and-write list, so a Figma import can
            put its components into the workspace instead of only being read.
            Nothing is written until the user picks a file. */}
        <GeneratedFileList
          files={figmaResult}
          workspacePath={workspacePath}
          onError={(m) => toast.error(m)}
          onOpenFile={onOpenFile}
        />
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
          {PROVIDER_SETTING_NOTE[p.id] && (
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
              {PROVIDER_SETTING_NOTE[p.id]}
            </div>
          )}
          {p.id === "figma" ? (
            <button
              type="button"
              className="panel-btn panel-btn-secondary panel-btn-sm"
              style={{ marginTop: "var(--space-2)" }}
              onClick={() => setActiveTab("figma")}
            >
              Open Figma tab
            </button>
          ) : PROVIDER_TAB_LINK[p.id] ? (
            <button
              type="button"
              className="panel-btn panel-btn-secondary panel-btn-sm"
              style={{ marginTop: "var(--space-2)" }}
              onClick={() => openDesignTab(PROVIDER_TAB_LINK[p.id])}
            >
              Open {p.label}
            </button>
          ) : null}
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
