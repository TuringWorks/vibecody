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

const ChatTabManager = lazy(() => import("./ChatTabManager").then(m => ({ default: m.ChatTabManager })));
const AgentPanel = lazy(() => import("./AgentPanel").then(m => ({ default: m.AgentPanel })));
const MemoryPanel = lazy(() => import("./MemoryPanel").then(m => ({ default: m.MemoryPanel })));
const HistoryPanel = lazy(() => import("./HistoryPanel").then(m => ({ default: m.HistoryPanel })));
const CheckpointPanel = lazy(() => import("./CheckpointPanel").then(m => ({ default: m.CheckpointPanel })));
const ArtifactsPanel = lazy(() => import("./ArtifactsPanel").then(m => ({ default: m.ArtifactsPanel })));
const ManagerView = lazy(() => import("./ManagerView").then(m => ({ default: m.ManagerView })));
const HooksPanel = lazy(() => import("./HooksPanel").then(m => ({ default: m.HooksPanel })));
const BackgroundJobsPanel = lazy(() => import("./BackgroundJobsPanel").then(m => ({ default: m.BackgroundJobsPanel })));
const McpPanel = lazy(() => import("./McpPanel").then(m => ({ default: m.McpPanel })));
const SettingsPanel = lazy(() => import("./SettingsPanel").then(m => ({ default: m.SettingsPanel })));
const CascadePanel = lazy(() => import("./CascadePanel").then(m => ({ default: m.CascadePanel })));
const SpecPanel = lazy(() => import("./SpecPanel").then(m => ({ default: m.SpecPanel })));
const WorkflowPanel = lazy(() => import("./WorkflowPanel").then(m => ({ default: m.WorkflowPanel })));
const OrchestrationPanel = lazy(() => import("./OrchestrationPanel").then(m => ({ default: m.OrchestrationPanel })));
const AiMlWorkflowPanel = lazy(() => import("./AiMlWorkflowPanel").then(m => ({ default: m.AiMlWorkflowPanel })));
const ModelWizardPanel = lazy(() => import("./ModelWizardPanel").then(m => ({ default: m.ModelWizardPanel })));
const DesignMode = lazy(() => import("./DesignMode").then(m => ({ default: m.DesignMode })));
const DeployPanel = lazy(() => import("./DeployPanel").then(m => ({ default: m.DeployPanel })));
const DatabasePanel = lazy(() => import("./DatabasePanel").then(m => ({ default: m.DatabasePanel })));
const VibeSqlPanel = lazy(() => import("./VibeSqlPanel").then(m => ({ default: m.VibeSqlPanel })));
const SupabasePanel = lazy(() => import("./SupabasePanel").then(m => ({ default: m.SupabasePanel })));
const AuthPanel = lazy(() => import("./AuthPanel").then(m => ({ default: m.AuthPanel })));
const GitHubSyncPanel = lazy(() => import("./GitHubSyncPanel").then(m => ({ default: m.GitHubSyncPanel })));
const SteeringPanel = lazy(() => import("./SteeringPanel"));
const BugBotPanel = lazy(() => import("./BugBotPanel").then(m => ({ default: m.BugBotPanel })));
const RedTeamPanel = lazy(() => import("./RedTeamPanel").then(m => ({ default: m.RedTeamPanel })));
const TestPanel = lazy(() => import("./TestPanel").then(m => ({ default: m.TestPanel })));
const BuildPanel = lazy(() => import("./BuildPanel").then(m => ({ default: m.BuildPanel })));
const CollabPanel = lazy(() => import("./CollabPanel").then(m => ({ default: m.CollabPanel })));
const CoveragePanel = lazy(() => import("./CoveragePanel").then(m => ({ default: m.CoveragePanel })));
const MultiModelPanel = lazy(() => import("./MultiModelPanel").then(m => ({ default: m.MultiModelPanel })));
const HttpPlayground = lazy(() => import("./HttpPlayground").then(m => ({ default: m.HttpPlayground })));
const ArenaPanel = lazy(() => import("./ArenaPanel").then(m => ({ default: m.ArenaPanel })));
const CostPanel = lazy(() => import("./CostPanel").then(m => ({ default: m.CostPanel })));
const AutofixPanel = lazy(() => import("./AutofixPanel").then(m => ({ default: m.AutofixPanel })));
const ProcessPanel = lazy(() => import("./ProcessPanel"));
const CicdPanel = lazy(() => import("./CicdPanel"));
const K8sPanel = lazy(() => import("./K8sPanel"));
const EnvPanel = lazy(() => import("./EnvPanel").then(m => ({ default: m.EnvPanel })));
const ProfilerPanel = lazy(() => import("./ProfilerPanel").then(m => ({ default: m.ProfilerPanel })));
const DockerPanel = lazy(() => import("./DockerPanel").then(m => ({ default: m.DockerPanel })));
const DepsPanel = lazy(() => import("./DepsPanel").then(m => ({ default: m.DepsPanel })));
const ApiDocsPanel = lazy(() => import("./ApiDocsPanel").then(m => ({ default: m.ApiDocsPanel })));
const MigrationsPanel = lazy(() => import("./MigrationsPanel").then(m => ({ default: m.MigrationsPanel })));
const LogPanel = lazy(() => import("./LogPanel").then(m => ({ default: m.LogPanel })));
const ScriptPanel = lazy(() => import("./ScriptPanel").then(m => ({ default: m.ScriptPanel })));
const NotebookPanel = lazy(() => import("./NotebookPanel").then(m => ({ default: m.NotebookPanel })));
const SshPanel = lazy(() => import("./SshPanel").then(m => ({ default: m.SshPanel })));
const UtilitiesPanel = lazy(() => import("./UtilitiesPanel").then(m => ({ default: m.UtilitiesPanel })));
const BookmarkPanel = lazy(() => import("./BookmarkPanel").then(m => ({ default: m.BookmarkPanel })));
const BisectPanel = lazy(() => import("./BisectPanel").then(m => ({ default: m.BisectPanel })));
const SnippetPanel = lazy(() => import("./SnippetPanel").then(m => ({ default: m.SnippetPanel })));
const MockServerPanel = lazy(() => import("./MockServerPanel").then(m => ({ default: m.MockServerPanel })));
const GraphQLPanel = lazy(() => import("./GraphQLPanel").then(m => ({ default: m.GraphQLPanel })));
const CodeMetricsPanel = lazy(() => import("./CodeMetricsPanel").then(m => ({ default: m.CodeMetricsPanel })));
const LoadTestPanel = lazy(() => import("./LoadTestPanel").then(m => ({ default: m.LoadTestPanel })));
const NetworkPanel = lazy(() => import("./NetworkPanel").then(m => ({ default: m.NetworkPanel })));
const AgentTeamPanel = lazy(() => import("./AgentTeamPanel").then(m => ({ default: m.AgentTeamPanel })));
const CIReviewPanel = lazy(() => import("./CIReviewPanel").then(m => ({ default: m.CIReviewPanel })));
const TraceDashboard = lazy(() => import("./TraceDashboard").then(m => ({ default: m.TraceDashboard })));
const MarketplacePanel = lazy(() => import("./MarketplacePanel").then(m => ({ default: m.MarketplacePanel })));
const TransformPanel = lazy(() => import("./TransformPanel").then(m => ({ default: m.TransformPanel })));
const ScreenshotToApp = lazy(() => import("./ScreenshotToApp").then(m => ({ default: m.ScreenshotToApp })));
const AgentRecordingPanel = lazy(() => import("./AgentRecordingPanel").then(m => ({ default: m.AgentRecordingPanel })));
const VisualTestPanel = lazy(() => import("./VisualTestPanel").then(m => ({ default: m.VisualTestPanel })));
const CloudAgentPanel = lazy(() => import("./CloudAgentPanel").then(m => ({ default: m.CloudAgentPanel })));
const CompliancePanel = lazy(() => import("./CompliancePanel").then(m => ({ default: m.CompliancePanel })));
const ScaffoldPanel = lazy(() => import("./ScaffoldPanel").then(m => ({ default: m.ScaffoldPanel })));
const HealthMonitorPanel = lazy(() => import("./HealthMonitorPanel").then(m => ({ default: m.HealthMonitorPanel })));
const WebSocketPanel = lazy(() => import("./WebSocketPanel").then(m => ({ default: m.WebSocketPanel })));
const ColorPalettePanel = lazy(() => import("./ColorPalettePanel").then(m => ({ default: m.ColorPalettePanel })));
const MarkdownPanel = lazy(() => import("./MarkdownPanel").then(m => ({ default: m.MarkdownPanel })));
const DiffToolPanel = lazy(() => import("./DiffToolPanel").then(m => ({ default: m.DiffToolPanel })));
const CanvasPanel = lazy(() => import("./CanvasPanel"));
const CronPanel = lazy(() => import("./CronPanel").then(m => ({ default: m.CronPanel })));
const RegexPanel = lazy(() => import("./RegexPanel").then(m => ({ default: m.RegexPanel })));
const JwtPanel = lazy(() => import("./JwtPanel").then(m => ({ default: m.JwtPanel })));
const JsonToolsPanel = lazy(() => import("./JsonToolsPanel").then(m => ({ default: m.JsonToolsPanel })));
const EncodingPanel = lazy(() => import("./EncodingPanel").then(m => ({ default: m.EncodingPanel })));
const NumberBasePanel = lazy(() => import("./NumberBasePanel").then(m => ({ default: m.NumberBasePanel })));
const DataGenPanel = lazy(() => import("./DataGenPanel").then(m => ({ default: m.DataGenPanel })));
const TimestampPanel = lazy(() => import("./TimestampPanel").then(m => ({ default: m.TimestampPanel })));
const ColorConverterPanel = lazy(() => import("./ColorConverterPanel").then(m => ({ default: m.ColorConverterPanel })));
const CidrPanel = lazy(() => import("./CidrPanel").then(m => ({ default: m.CidrPanel })));
const CsvPanel = lazy(() => import("./CsvPanel").then(m => ({ default: m.CsvPanel })));
const UnitConverterPanel = lazy(() => import("./UnitConverterPanel").then(m => ({ default: m.UnitConverterPanel })));
const UnicodePanel = lazy(() => import("./UnicodePanel").then(m => ({ default: m.UnicodePanel })));
const SandboxPanel = lazy(() => import("./SandboxPanel").then(m => ({ default: m.SandboxPanel })));
const DashboardPanel = lazy(() => import("./DashboardPanel"));
const WebhookPanel = lazy(() => import("./WebhookPanel").then(m => ({ default: m.WebhookPanel })));
const AdminPanel = lazy(() => import("./AdminPanel").then(m => ({ default: m.AdminPanel })));
const AppBuilderPanel = lazy(() => import("./AppBuilderPanel").then(m => ({ default: m.AppBuilderPanel })));
const InfiniteContextPanel = lazy(() => import("./InfiniteContextPanel").then(m => ({ default: m.InfiniteContextPanel })));
const BatchBuilderPanel = lazy(() => import("./BatchBuilderPanel"));
const StreamingPanel = lazy(() => import("./StreamingPanel").then(m => ({ default: m.StreamingPanel })));
const InferencePanel = lazy(() => import("./InferencePanel").then(m => ({ default: m.InferencePanel })));
const TrainingPanel = lazy(() => import("./TrainingPanel").then(m => ({ default: m.TrainingPanel })));
const DocumentIngestPanel = lazy(() => import("./DocumentIngestPanel").then(m => ({ default: m.DocumentIngestPanel })));
const WebCrawlerPanel = lazy(() => import("./WebCrawlerPanel").then(m => ({ default: m.WebCrawlerPanel })));
const VectorDbPanel = lazy(() => import("./VectorDbPanel").then(m => ({ default: m.VectorDbPanel })));
const QaValidationPanel = lazy(() => import("./QaValidationPanel").then(m => ({ default: m.QaValidationPanel })));
const AstEditPanel = lazy(() => import("./AstEditPanel"));
const CiStatusPanel = lazy(() => import("./CiStatusPanel"));
const CloudSandboxPanel = lazy(() => import("./CloudSandboxPanel"));
const EditPredictionPanel = lazy(() => import("./EditPredictionPanel"));
const PlanDocumentPanel = lazy(() => import("./PlanDocumentPanel"));
const RemoteControlPanel = lazy(() => import("./RemoteControlPanel"));
const SecurityScanPanel = lazy(() => import("./SecurityScanPanel"));
const SessionBrowserPanel = lazy(() => import("./SessionBrowserPanel"));
const SubAgentPanel = lazy(() => import("./SubAgentPanel"));
const ClarifyingQuestionsPanel = lazy(() => import("./ClarifyingQuestionsPanel"));
const ConversationalSearchPanel = lazy(() => import("./ConversationalSearchPanel"));
const DemoPanel = lazy(() => import("./DemoPanel").then(m => ({ default: m.DemoPanel })));
const CloudAutofixPanel = lazy(() => import("./CloudAutofixPanel"));
const FastContextPanel = lazy(() => import("./FastContextPanel"));
const ImageGenPanel = lazy(() => import("./ImageGenPanel"));
const TeamGovernancePanel = lazy(() => import("./TeamGovernancePanel"));
const AgentTeamsPanel = lazy(() => import("./AgentTeamsPanel"));
const DiscussionModePanel = lazy(() => import("./DiscussionModePanel"));
const FullStackGenPanel = lazy(() => import("./FullStackGenPanel"));
const GhActionsPanel = lazy(() => import("./GhActionsPanel"));
const RenderOptimizePanel = lazy(() => import("./RenderOptimizePanel"));
const SoulPanel = lazy(() => import("./SoulPanel").then(m => ({ default: m.SoulPanel })));
// McpLazy and McpDirectory merged into McpPanel (unified MCP panel)
const ContextBundlePanel = lazy(() => import("./ContextBundlePanel").then(m => ({ default: m.ContextBundlePanel })));
const CloudProviderPanel = lazy(() => import("./CloudProviderPanel").then(m => ({ default: m.CloudProviderPanel })));
const AcpPanel = lazy(() => import("./AcpPanel").then(m => ({ default: m.AcpPanel })));
// McpDirectoryPanel merged into McpPanel
const UsageMeteringPanel = lazy(() => import("./UsageMeteringPanel").then(m => ({ default: m.UsageMeteringPanel })));
const SweBenchPanel = lazy(() => import("./SweBenchPanel").then(m => ({ default: m.SweBenchPanel })));
const SessionMemoryPanel = lazy(() => import("./SessionMemoryPanel").then(m => ({ default: m.SessionMemoryPanel })));
const BlueTeamPanel = lazy(() => import("./BlueTeamPanel").then(m => ({ default: m.BlueTeamPanel })));
const PurpleTeamPanel = lazy(() => import("./PurpleTeamPanel").then(m => ({ default: m.PurpleTeamPanel })));
const IdpPanel = lazy(() => import("./IdpPanel").then(m => ({ default: m.IdpPanel })));
const QuantumComputingPanel = lazy(() => import("./QuantumComputingPanel").then(m => ({ default: m.QuantumComputingPanel })));
// AgilePanel is now embedded inside WorkManagementPanel
const DebugModePanel = lazy(() => import("./DebugModePanel"));
const AgentModesPanel = lazy(() => import("./AgentModesPanel"));
const WorkManagementPanel = lazy(() => import("./WorkManagementPanel"));
const AutoResearchPanel = lazy(() => import("./AutoResearchPanel").then(m => ({ default: m.AutoResearchPanel })));
const OpenMemoryPanel = lazy(() => import("./OpenMemoryPanel"));

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
  const { tab, selectedProvider, availableProviders, editorContent, fileTree, currentFile, workspacePath, onPendingWrite, onInjectContext, onOpenFile, collab } = props;
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

  return (
    <>
      {panel("chat", <LazyPanel Component={ChatTabManager} props={{ defaultProvider: selectedProvider, availableProviders, context: editorContent, fileTree, currentFile, onPendingWrite }} />)}
      {panel("agent", <LazyPanel Component={AgentPanel} props={{ provider: selectedProvider, workspacePath: wp }} />)}
      {panel("memory", <LazyPanel Component={MemoryPanel} props={{ workspacePath: wp }} />)}
      {panel("history", <LazyPanel Component={HistoryPanel} props={{}} />)}
      {panel("checkpoints", <LazyPanel Component={CheckpointPanel} props={{ workspacePath: wp }} />)}
      {panel("artifacts", <LazyPanel Component={ArtifactsPanel} props={{ artifacts: [] }} />)}
      {panel("manager", <LazyPanel Component={ManagerView} props={{ provider: selectedProvider }} />)}
      {panel("hooks", <LazyPanel Component={HooksPanel} props={{ workspacePath: wp }} />)}
      {panel("jobs", <LazyPanel Component={BackgroundJobsPanel} props={{}} />)}
      {panel("mcp", <LazyPanel Component={McpPanel} props={{}} />)}
      {panel("settings", <LazyPanel Component={SettingsPanel} props={{}} />)}
      {panel("cascade", <LazyPanel Component={CascadePanel} props={{ onInjectContext }} />)}
      {panel("specs", <LazyPanel Component={SpecPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("workflow", <LazyPanel Component={WorkflowPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("orchestration", <LazyPanel Component={OrchestrationPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("design", <LazyPanel Component={DesignMode} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("deploy", <LazyPanel Component={DeployPanel} props={{ workspacePath: wp }} />)}
      {panel("vibesql", <LazyPanel Component={VibeSqlPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("database", <LazyPanel Component={DatabasePanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("supabase", <LazyPanel Component={SupabasePanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("auth", <LazyPanel Component={AuthPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("github", <LazyPanel Component={GitHubSyncPanel} props={{ workspacePath: wp }} />)}
      {panel("steering", <LazyPanel Component={SteeringPanel} props={{ workspaceRoot: wp || undefined }} />)}
      {panel("bugbot", <LazyPanel Component={BugBotPanel} props={{ workspacePath: wp || undefined, onOpenFile }} />)}
      {panel("redteam", <LazyPanel Component={RedTeamPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("tests", <LazyPanel Component={TestPanel} props={{ workspacePath: wp }} />)}
      {panel("build", <LazyPanel Component={BuildPanel} props={{ workspacePath: wp, currentFile, onOpenFile }} />)}
      {panel("collab", <LazyPanel Component={CollabPanel} props={{ connected: collab.connected, roomId: collab.roomId || "", peerId: collab.peerId || "", peers: collab.peers, onConnect: collab.connect, onDisconnect: collab.disconnect }} />)}
      {panel("coverage", <LazyPanel Component={CoveragePanel} props={{ workspacePath: wp }} />)}
      {panel("compare", <LazyPanel Component={MultiModelPanel} props={{ provider: selectedProvider }} />)}
      {panel("http", <LazyPanel Component={HttpPlayground} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("arena", <LazyPanel Component={ArenaPanel} props={{ provider: selectedProvider }} />)}
      {panel("cost", <LazyPanel Component={CostPanel} props={{ provider: selectedProvider }} />)}
      {panel("autofix", <LazyPanel Component={AutofixPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("processes", <LazyPanel Component={ProcessPanel} props={{}} />)}
      {panel("cicd", <LazyPanel Component={CicdPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("k8s", <LazyPanel Component={K8sPanel} props={{ workspacePath: wp }} />)}
      {panel("env", <LazyPanel Component={EnvPanel} props={{ workspacePath: wp }} />)}
      {panel("profiler", <LazyPanel Component={ProfilerPanel} props={{ workspacePath: wp }} />)}
      {panel("docker", <LazyPanel Component={DockerPanel} props={{ workspacePath: wp }} />)}
      {panel("deps", <LazyPanel Component={DepsPanel} props={{ workspacePath: wp }} />)}
      {panel("apidocs", <LazyPanel Component={ApiDocsPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("migrations", <LazyPanel Component={MigrationsPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("logs", <LazyPanel Component={LogPanel} props={{ workspacePath: wp }} />)}
      {panel("scripts", <LazyPanel Component={ScriptPanel} props={{ workspacePath: wp }} />)}
      {panel("notebook", <LazyPanel Component={NotebookPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("ssh", <LazyPanel Component={SshPanel} props={{ workspacePath: wp }} />)}
      {panel("utils", <LazyPanel Component={UtilitiesPanel} props={{}} />)}
      {panel("markers", <LazyPanel Component={BookmarkPanel} props={{ workspacePath: wp }} />)}
      {panel("bisect", <LazyPanel Component={BisectPanel} props={{ workspacePath: wp }} />)}
      {panel("snippets", <LazyPanel Component={SnippetPanel} props={{ workspacePath: wp }} />)}
      {panel("mock", <LazyPanel Component={MockServerPanel} props={{}} />)}
      {panel("graphql", <LazyPanel Component={GraphQLPanel} props={{ provider: selectedProvider }} />)}
      {panel("metrics", <LazyPanel Component={CodeMetricsPanel} props={{ workspacePath: wp }} />)}
      {panel("loadtest", <LazyPanel Component={LoadTestPanel} props={{ provider: selectedProvider }} />)}
      {panel("network", <LazyPanel Component={NetworkPanel} props={{}} />)}
      {panel("teams", <LazyPanel Component={AgentTeamPanel} props={{ provider: selectedProvider }} />)}
      {panel("cibot", <LazyPanel Component={CIReviewPanel} props={{ provider: selectedProvider }} />)}
      {panel("traces", <LazyPanel Component={TraceDashboard} props={{}} />)}
      {panel("marketplace", <LazyPanel Component={MarketplacePanel} props={{}} />)}
      {panel("transform", <LazyPanel Component={TransformPanel} props={{ provider: selectedProvider }} />)}
      {panel("img2app", <LazyPanel Component={ScreenshotToApp} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("recording", <LazyPanel Component={AgentRecordingPanel} props={{ provider: selectedProvider }} />)}
      {panel("visualtest", <LazyPanel Component={VisualTestPanel} props={{ provider: selectedProvider }} />)}
      {panel("cloud", <LazyPanel Component={CloudAgentPanel} props={{ provider: selectedProvider }} />)}
      {panel("compliance", <LazyPanel Component={CompliancePanel} props={{}} />)}
      {panel("scaffold", <LazyPanel Component={ScaffoldPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("health", <LazyPanel Component={HealthMonitorPanel} props={{}} />)}
      {panel("websocket", <LazyPanel Component={WebSocketPanel} props={{}} />)}
      {panel("colors", <LazyPanel Component={ColorPalettePanel} props={{ workspacePath: wp }} />)}
      {panel("markdown", <LazyPanel Component={MarkdownPanel} props={{ workspacePath: wp }} />)}
      {panel("difftool", <LazyPanel Component={DiffToolPanel} props={{}} />)}
      {panel("canvas", <LazyPanel Component={CanvasPanel} props={{}} />)}
      {panel("cron", <LazyPanel Component={CronPanel} props={{}} />)}
      {panel("regex", <LazyPanel Component={RegexPanel} props={{}} />)}
      {panel("jwt", <LazyPanel Component={JwtPanel} props={{}} />)}
      {panel("jsontools", <LazyPanel Component={JsonToolsPanel} props={{}} />)}
      {panel("encoding", <LazyPanel Component={EncodingPanel} props={{}} />)}
      {panel("numbers", <LazyPanel Component={NumberBasePanel} props={{}} />)}
      {panel("datagen", <LazyPanel Component={DataGenPanel} props={{}} />)}
      {panel("timestamp", <LazyPanel Component={TimestampPanel} props={{}} />)}
      {panel("colorconv", <LazyPanel Component={ColorConverterPanel} props={{}} />)}
      {panel("cidr", <LazyPanel Component={CidrPanel} props={{}} />)}
      {panel("csv", <LazyPanel Component={CsvPanel} props={{}} />)}
      {panel("units", <LazyPanel Component={UnitConverterPanel} props={{}} />)}
      {panel("unicode", <LazyPanel Component={UnicodePanel} props={{}} />)}
      {panel("sandbox", <LazyPanel Component={SandboxPanel} props={{ provider: selectedProvider }} />)}
      {panel("dashboard", <LazyPanel Component={DashboardPanel} props={{ provider: selectedProvider }} />)}
      {panel("webhooks", <LazyPanel Component={WebhookPanel} props={{}} />)}
      {panel("admin", <LazyPanel Component={AdminPanel} props={{}} />)}
      {panel("appbuilder", <LazyPanel Component={AppBuilderPanel} props={{ workspacePath: wp || "", provider: selectedProvider }} />)}
      {panel("icontext", <LazyPanel Component={InfiniteContextPanel} props={{ workspacePath: wp || "", provider: selectedProvider }} />)}
      {panel("batchbuilder", <LazyPanel Component={BatchBuilderPanel} props={{ provider: selectedProvider }} />)}
      {panel("streaming", <LazyPanel Component={StreamingPanel} props={{ provider: selectedProvider }} />)}
      {panel("inference", <LazyPanel Component={InferencePanel} props={{ provider: selectedProvider }} />)}
      {panel("training", <LazyPanel Component={TrainingPanel} props={{ provider: selectedProvider }} />)}
      {panel("ingest", <LazyPanel Component={DocumentIngestPanel} props={{ provider: selectedProvider }} />)}
      {panel("crawler", <LazyPanel Component={WebCrawlerPanel} props={{}} />)}
      {panel("vectordb", <LazyPanel Component={VectorDbPanel} props={{ provider: selectedProvider }} />)}
      {panel("qa-validation", <LazyPanel Component={QaValidationPanel} props={{ provider: selectedProvider }} />)}
      {panel("astedit", <LazyPanel Component={AstEditPanel} props={{ provider: selectedProvider }} />)}
      {panel("cistatus", <LazyPanel Component={CiStatusPanel} props={{ provider: selectedProvider }} />)}
      {panel("cloudsandbox", <LazyPanel Component={CloudSandboxPanel} props={{ provider: selectedProvider }} />)}
      {panel("editpredict", <LazyPanel Component={EditPredictionPanel} props={{ provider: selectedProvider }} />)}
      {panel("plandoc", <LazyPanel Component={PlanDocumentPanel} props={{ provider: selectedProvider }} />)}
      {panel("remotecontrol", <LazyPanel Component={RemoteControlPanel} props={{ provider: selectedProvider }} />)}
      {panel("securityscan", <LazyPanel Component={SecurityScanPanel} props={{ workspacePath: wp || undefined, onOpenFile, provider: selectedProvider }} />)}
      {panel("sessions", <LazyPanel Component={SessionBrowserPanel} props={{}} />)}
      {panel("subagents", <LazyPanel Component={SubAgentPanel} props={{ provider: selectedProvider }} />)}
      {panel("clarify", <LazyPanel Component={ClarifyingQuestionsPanel} props={{ provider: selectedProvider }} />)}
      {panel("codesearch", <LazyPanel Component={ConversationalSearchPanel} props={{ provider: selectedProvider }} />)}
      {panel("demo", <LazyPanel Component={DemoPanel} props={{}} />)}
      {panel("cloudautofix", <LazyPanel Component={CloudAutofixPanel} props={{ provider: selectedProvider }} />)}
      {panel("fastcontext", <LazyPanel Component={FastContextPanel} props={{ provider: selectedProvider }} />)}
      {panel("imagegen", <LazyPanel Component={ImageGenPanel} props={{ provider: selectedProvider }} />)}
      {panel("governance", <LazyPanel Component={TeamGovernancePanel} props={{ provider: selectedProvider }} />)}
      {panel("agentteams", <LazyPanel Component={AgentTeamsPanel} props={{ provider: selectedProvider }} />)}
      {panel("discuss", <LazyPanel Component={DiscussionModePanel} props={{ provider: selectedProvider }} />)}
      {panel("fullstack", <LazyPanel Component={FullStackGenPanel} props={{ provider: selectedProvider }} />)}
      {panel("ghactions", <LazyPanel Component={GhActionsPanel} props={{ provider: selectedProvider }} />)}
      {panel("renderopt", <LazyPanel Component={RenderOptimizePanel} props={{}} />)}
      {panel("soul", <LazyPanel Component={SoulPanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {/* mcplazy merged into mcp panel */}
      {panel("bundles", <LazyPanel Component={ContextBundlePanel} props={{ workspacePath: wp, provider: selectedProvider }} />)}
      {panel("cloudproviders", <LazyPanel Component={CloudProviderPanel} props={{ workspacePath: wp }} />)}
      {panel("acpprotocol", <LazyPanel Component={AcpPanel} props={{ provider: selectedProvider }} />)}
      {/* mcpdirectory merged into mcp panel */}
      {panel("usagemetering", <LazyPanel Component={UsageMeteringPanel} props={{}} />)}
      {panel("swebench", <LazyPanel Component={SweBenchPanel} props={{ provider: selectedProvider }} />)}
      {panel("sessionmemory", <LazyPanel Component={SessionMemoryPanel} props={{}} />)}
      {panel("blueteam", <LazyPanel Component={BlueTeamPanel} props={{ provider: selectedProvider }} />)}
      {panel("purpleteam", <LazyPanel Component={PurpleTeamPanel} props={{ provider: selectedProvider }} />)}
      {panel("idp", <LazyPanel Component={IdpPanel} props={{ provider: selectedProvider }} />)}
      {panel("quantum", <LazyPanel Component={QuantumComputingPanel} props={{ provider: selectedProvider }} />)}
      {/* agile is now embedded inside workmanagement panel */}
      {panel("debugmode", <LazyPanel Component={DebugModePanel} props={{}} />)}
      {panel("agentmodes", <LazyPanel Component={AgentModesPanel} props={{}} />)}
      {panel("workmanagement", <LazyPanel Component={WorkManagementPanel} props={{}} />)}
      {panel("autoresearch", <LazyPanel Component={AutoResearchPanel} props={{ workspacePath: wp || "", provider: selectedProvider }} />)}
      {panel("openmemory", <LazyPanel Component={OpenMemoryPanel} props={{}} />)}
      {panel("aiml", <LazyPanel Component={AiMlWorkflowPanel} props={{}} />)}
      {panel("modelwizard", <LazyPanel Component={ModelWizardPanel} props={{}} />)}
      {/* Fallback for unknown tabs — only render when active and not matched above */}
      {!visited.has(tab) || ![
        "chat","agent","memory","history","checkpoints","artifacts","manager","hooks","jobs","mcp",
        "settings","cascade","specs","workflow","orchestration","design","deploy","database","supabase",
        "auth","github","steering","bugbot","redteam","tests","collab","coverage","compare","http",
        "arena","cost","autofix","processes","cicd","k8s","env","profiler","docker","deps","apidocs",
        "migrations","logs","scripts","notebook","ssh","utils","markers","bisect","snippets","mock",
        "graphql","metrics","loadtest","network","teams","cibot","traces","marketplace","transform",
        "img2app","recording","visualtest","cloud","compliance","scaffold","health","websocket","colors",
        "markdown","difftool","canvas","cron","regex","jwt","jsontools","encoding","numbers","datagen",
        "timestamp","colorconv","cidr","csv","units","unicode","sandbox","dashboard","webhooks","admin",
        "appbuilder","icontext","batchbuilder","streaming","inference","training","ingest","crawler",
        "vectordb","qa-validation","astedit","cistatus","cloudsandbox","editpredict","plandoc",
        "remotecontrol","securityscan","sessions","subagents","clarify","codesearch","demo","cloudautofix",
        "fastcontext","imagegen","governance","agentteams","discuss","fullstack","ghactions","renderopt",
        "soul","bundles","cloudproviders","acpprotocol","usagemetering",
        "swebench","sessionmemory","blueteam","purpleteam","idp","quantum",
        "debugmode","agentmodes","workmanagement","build","autoresearch","openmemory",
      ].includes(tab) ? (
        <div style={{ padding: 16, color: "var(--text-secondary)" }}>Unknown panel: {tab}</div>
      ) : null}
    </>
  );
}
