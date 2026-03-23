import { lazy, Suspense, useRef, type ComponentType } from "react";
import { ErrorBoundary } from "./ErrorBoundary";

const PanelLoading = () => (
  <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading panel...</div>
);

/** Wraps a lazy-loaded panel with its own ErrorBoundary + Suspense. */
function LazyPanel<P extends object>({ Component, props }: { Component: ComponentType<P>; props: P }) {
  return (
    <ErrorBoundary>
      <Suspense fallback={<PanelLoading />}>
        <Component {...props} />
      </Suspense>
    </ErrorBoundary>
  );
}

/**
 * KeepAlivePanel — wraps a panel so it stays mounted (hidden) when inactive.
 * The panel is only rendered once it has been visited at least once.
 */
function KeepAlivePanel({ active, children }: { active: boolean; children: React.ReactNode }) {
  return (
    <div
      style={{
        display: active ? "contents" : "none",
        // "contents" makes this wrapper invisible to layout — the panel renders
        // as if it were a direct child of the parent container.
      }}
    >
      {children}
    </div>
  );
}

// --- Lazy imports (code-split per panel) ---

// Unchanged panels
const ChatTabManager = lazy(() => import("./ChatTabManager").then(m => ({ default: m.ChatTabManager })));
const AgentPanel = lazy(() => import("./AgentPanel").then(m => ({ default: m.AgentPanel })));
const MarketplacePanel = lazy(() => import("./MarketplacePanel").then(m => ({ default: m.MarketplacePanel })));

// Composite panels
const AiTeamsComposite = lazy(() => import("./composite/AiTeamsComposite").then(m => ({ default: m.AiTeamsComposite })));
const AiPlaygroundComposite = lazy(() => import("./composite/AiPlaygroundComposite").then(m => ({ default: m.AiPlaygroundComposite })));
const AiContextComposite = lazy(() => import("./composite/AiContextComposite").then(m => ({ default: m.AiContextComposite })));
const AiGenerationComposite = lazy(() => import("./composite/AiGenerationComposite").then(m => ({ default: m.AiGenerationComposite })));
const ProjectHubComposite = lazy(() => import("./composite/ProjectHubComposite").then(m => ({ default: m.ProjectHubComposite })));
const PlanningComposite = lazy(() => import("./composite/PlanningComposite").then(m => ({ default: m.PlanningComposite })));
const ObservabilityComposite = lazy(() => import("./composite/ObservabilityComposite").then(m => ({ default: m.ObservabilityComposite })));
const DesignComposite = lazy(() => import("./composite/DesignComposite").then(m => ({ default: m.DesignComposite })));
const SecurityComposite = lazy(() => import("./composite/SecurityComposite").then(m => ({ default: m.SecurityComposite })));
const TestingComposite = lazy(() => import("./composite/TestingComposite").then(m => ({ default: m.TestingComposite })));
const CodeAnalysisComposite = lazy(() => import("./composite/CodeAnalysisComposite").then(m => ({ default: m.CodeAnalysisComposite })));
const VersionControlComposite = lazy(() => import("./composite/VersionControlComposite").then(m => ({ default: m.VersionControlComposite })));
const GitHubComposite = lazy(() => import("./composite/GitHubComposite").then(m => ({ default: m.GitHubComposite })));
const CollaborationComposite = lazy(() => import("./composite/CollaborationComposite").then(m => ({ default: m.CollaborationComposite })));
const BuildDeployComposite = lazy(() => import("./composite/BuildDeployComposite").then(m => ({ default: m.BuildDeployComposite })));
const ContainersComposite = lazy(() => import("./composite/ContainersComposite").then(m => ({ default: m.ContainersComposite })));
const CiCdComposite = lazy(() => import("./composite/CiCdComposite").then(m => ({ default: m.CiCdComposite })));
const CloudPlatformComposite = lazy(() => import("./composite/CloudPlatformComposite").then(m => ({ default: m.CloudPlatformComposite })));
const AiMlComposite = lazy(() => import("./composite/AiMlComposite").then(m => ({ default: m.AiMlComposite })));
const DatabaseComposite = lazy(() => import("./composite/DatabaseComposite").then(m => ({ default: m.DatabaseComposite })));
const ApiToolsComposite = lazy(() => import("./composite/ApiToolsComposite").then(m => ({ default: m.ApiToolsComposite })));
const DataPipelineComposite = lazy(() => import("./composite/DataPipelineComposite").then(m => ({ default: m.DataPipelineComposite })));
const SystemMonitorComposite = lazy(() => import("./composite/SystemMonitorComposite").then(m => ({ default: m.SystemMonitorComposite })));
const TerminalComposite = lazy(() => import("./composite/TerminalComposite").then(m => ({ default: m.TerminalComposite })));
const DiagnosticsComposite = lazy(() => import("./composite/DiagnosticsComposite").then(m => ({ default: m.DiagnosticsComposite })));
const ConvertersComposite = lazy(() => import("./composite/ConvertersComposite").then(m => ({ default: m.ConvertersComposite })));
const FormattersComposite = lazy(() => import("./composite/FormattersComposite").then(m => ({ default: m.FormattersComposite })));
const EditorsComposite = lazy(() => import("./composite/EditorsComposite").then(m => ({ default: m.EditorsComposite })));
const ConfigComposite = lazy(() => import("./composite/ConfigComposite").then(m => ({ default: m.ConfigComposite })));
const IntegrationsComposite = lazy(() => import("./composite/IntegrationsComposite").then(m => ({ default: m.IntegrationsComposite })));
const AdministrationComposite = lazy(() => import("./composite/AdministrationComposite").then(m => ({ default: m.AdministrationComposite })));
const BillingComposite = lazy(() => import("./composite/BillingComposite").then(m => ({ default: m.BillingComposite })));
const ToolsSettingsComposite = lazy(() => import("./composite/ToolsSettingsComposite").then(m => ({ default: m.ToolsSettingsComposite })));

// --- Props interfaces ---

interface PanelHostProps {
  tab: string;
  selectedProvider: string;
  availableProviders: string[];
  editorContent: string;
  fileTree: string[];
  currentFile: string | null;
  workspacePath: string | null;
  onPendingWrite: (path: string, content: string) => void;
  onInjectContext: (text: string) => void;
  onOpenFile?: (path: string, line?: number) => void;
  collab: {
    connected: boolean;
    roomId: string | null;
    peerId: string | null;
    peers: Array<{ peerId: string; name: string; color: string }>;
    connect: (...args: any[]) => void;
    disconnect: () => void;
  };
}

/**
 * PanelHost — Keep-alive panel renderer.
 *
 * Instead of unmounting panels on tab switch (which destroys all state),
 * we keep every visited panel mounted but hidden (display:none). This means:
 *   - Panel state (useState, useRef, etc.) survives tab switches
 *   - Lazy-loading still works — panels only load when first visited
 *   - No serialization overhead — React state stays in memory
 *
 * For cross-session persistence (surviving app restarts), individual panels
 * can use the `usePersistentState` hook or `usePanelSettings`.
 */
export function PanelHost(props: PanelHostProps) {
  const { tab, selectedProvider, availableProviders, editorContent, fileTree, currentFile, workspacePath, onPendingWrite } = props;
  const wp = workspacePath;

  // Track which tabs have been visited so we only mount them once they're first opened.
  const visitedRef = useRef<Set<string>>(new Set());
  visitedRef.current.add(tab);
  const visited = visitedRef.current;

  /** Helper: render a panel only if it has been visited. */
  const panel = (id: string, element: React.ReactNode) =>
    visited.has(id) ? (
      <KeepAlivePanel key={id} active={tab === id}>
        {element}
      </KeepAlivePanel>
    ) : null;

  const KNOWN_TABS = [
    "chat", "agent", "marketplace",
    "ai-teams", "ai-playground", "ai-context", "ai-generation",
    "project-hub", "planning", "observability", "design",
    "security", "testing", "code-analysis",
    "version-control", "github", "collaboration",
    "build-deploy", "containers", "ci-cd", "cloud-platform", "ai-ml",
    "database", "api-tools", "data-pipeline",
    "system-monitor", "terminal", "diagnostics",
    "converters", "formatters", "editors",
    "config", "integrations", "administration", "billing", "tools-settings",
  ];

  return (
    <>
      {/* --- AI --- */}
      {panel("chat", <LazyPanel Component={ChatTabManager} props={{ defaultProvider: selectedProvider, availableProviders, context: editorContent, fileTree, currentFile, onPendingWrite }} />)}
      {panel("agent", <LazyPanel Component={AgentPanel} props={{ provider: selectedProvider, workspacePath: wp }} />)}
      {panel("ai-teams", <LazyPanel Component={AiTeamsComposite} props={{ provider: selectedProvider }} />)}
      {panel("ai-playground", <LazyPanel Component={AiPlaygroundComposite} props={{ provider: selectedProvider }} />)}
      {panel("ai-context", <LazyPanel Component={AiContextComposite} props={{ workspacePath: wp || "", provider: selectedProvider }} />)}
      {panel("ai-generation", <LazyPanel Component={AiGenerationComposite} props={{ workspacePath: wp || "", provider: selectedProvider }} />)}
      {panel("marketplace", <LazyPanel Component={MarketplacePanel} props={{}} />)}

      {/* --- Project --- */}
      {panel("project-hub", <LazyPanel Component={ProjectHubComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("planning", <LazyPanel Component={PlanningComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("observability", <LazyPanel Component={ObservabilityComposite} props={{ provider: selectedProvider }} />)}
      {panel("design", <LazyPanel Component={DesignComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}

      {/* --- Code Quality --- */}
      {panel("security", <LazyPanel Component={SecurityComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("testing", <LazyPanel Component={TestingComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("code-analysis", <LazyPanel Component={CodeAnalysisComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}

      {/* --- Source Control --- */}
      {panel("version-control", <LazyPanel Component={VersionControlComposite} props={{ workspacePath: wp }} />)}
      {panel("github", <LazyPanel Component={GitHubComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("collaboration", <LazyPanel Component={CollaborationComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}

      {/* --- Infrastructure --- */}
      {panel("build-deploy", <LazyPanel Component={BuildDeployComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("containers", <LazyPanel Component={ContainersComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("ci-cd", <LazyPanel Component={CiCdComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("cloud-platform", <LazyPanel Component={CloudPlatformComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("ai-ml", <LazyPanel Component={AiMlComposite} props={{ provider: selectedProvider }} />)}

      {/* --- Data & APIs --- */}
      {panel("database", <LazyPanel Component={DatabaseComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("api-tools", <LazyPanel Component={ApiToolsComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("data-pipeline", <LazyPanel Component={DataPipelineComposite} props={{ provider: selectedProvider }} />)}

      {/* --- Developer Tools --- */}
      {panel("system-monitor", <LazyPanel Component={SystemMonitorComposite} props={{ workspacePath: wp }} />)}
      {panel("terminal", <LazyPanel Component={TerminalComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("diagnostics", <LazyPanel Component={DiagnosticsComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}

      {/* --- Toolkit --- */}
      {panel("converters", <LazyPanel Component={ConvertersComposite} props={{}} />)}
      {panel("formatters", <LazyPanel Component={FormattersComposite} props={{}} />)}
      {panel("editors", <LazyPanel Component={EditorsComposite} props={{ workspacePath: wp }} />)}

      {/* --- Settings --- */}
      {panel("config", <LazyPanel Component={ConfigComposite} props={{ workspacePath: wp }} />)}
      {panel("integrations", <LazyPanel Component={IntegrationsComposite} props={{ provider: selectedProvider }} />)}
      {panel("administration", <LazyPanel Component={AdministrationComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("billing", <LazyPanel Component={BillingComposite} props={{ provider: selectedProvider }} />)}
      {panel("tools-settings", <LazyPanel Component={ToolsSettingsComposite} props={{ workspacePath: wp, provider: selectedProvider }} />)}

      {/* Fallback for unknown tabs */}
      {!visited.has(tab) || !KNOWN_TABS.includes(tab) ? (
        <div style={{ padding: 16, color: "var(--text-secondary)" }}>Unknown panel: {tab}</div>
      ) : null}
    </>
  );
}
