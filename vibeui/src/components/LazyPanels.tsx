import { lazy, Suspense, type ComponentType } from "react";
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
const DesignMode = lazy(() => import("./DesignMode").then(m => ({ default: m.DesignMode })));
const DeployPanel = lazy(() => import("./DeployPanel").then(m => ({ default: m.DeployPanel })));
const DatabasePanel = lazy(() => import("./DatabasePanel").then(m => ({ default: m.DatabasePanel })));
const SupabasePanel = lazy(() => import("./SupabasePanel").then(m => ({ default: m.SupabasePanel })));
const AuthPanel = lazy(() => import("./AuthPanel").then(m => ({ default: m.AuthPanel })));
const GitHubSyncPanel = lazy(() => import("./GitHubSyncPanel").then(m => ({ default: m.GitHubSyncPanel })));
const SteeringPanel = lazy(() => import("./SteeringPanel"));
const BugBotPanel = lazy(() => import("./BugBotPanel").then(m => ({ default: m.BugBotPanel })));
const RedTeamPanel = lazy(() => import("./RedTeamPanel").then(m => ({ default: m.RedTeamPanel })));
const TestPanel = lazy(() => import("./TestPanel").then(m => ({ default: m.TestPanel })));
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
const McpLazyPanel = lazy(() => import("./McpLazyPanel").then(m => ({ default: m.McpLazyPanel })));
const ContextBundlePanel = lazy(() => import("./ContextBundlePanel").then(m => ({ default: m.ContextBundlePanel })));
const CloudProviderPanel = lazy(() => import("./CloudProviderPanel").then(m => ({ default: m.CloudProviderPanel })));
const AcpPanel = lazy(() => import("./AcpPanel").then(m => ({ default: m.AcpPanel })));
const McpDirectoryPanel = lazy(() => import("./McpDirectoryPanel").then(m => ({ default: m.McpDirectoryPanel })));
const UsageMeteringPanel = lazy(() => import("./UsageMeteringPanel").then(m => ({ default: m.UsageMeteringPanel })));
const SweBenchPanel = lazy(() => import("./SweBenchPanel").then(m => ({ default: m.SweBenchPanel })));
const SessionMemoryPanel = lazy(() => import("./SessionMemoryPanel").then(m => ({ default: m.SessionMemoryPanel })));
const BlueTeamPanel = lazy(() => import("./BlueTeamPanel").then(m => ({ default: m.BlueTeamPanel })));
const PurpleTeamPanel = lazy(() => import("./PurpleTeamPanel").then(m => ({ default: m.PurpleTeamPanel })));
const IdpPanel = lazy(() => import("./IdpPanel").then(m => ({ default: m.IdpPanel })));

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

/** Renders the active panel with lazy loading + per-panel ErrorBoundary. */
export function PanelHost(props: PanelHostProps) {
  const { tab, selectedProvider, availableProviders, editorContent, fileTree, currentFile, workspacePath, onPendingWrite, onInjectContext, onOpenFile, collab } = props;
  const wp = workspacePath;

  switch (tab) {
    case "chat":
      return <LazyPanel Component={ChatTabManager} props={{ defaultProvider: selectedProvider, availableProviders, context: editorContent, fileTree, currentFile, onPendingWrite }} />;
    case "agent":
      return <LazyPanel Component={AgentPanel} props={{ provider: selectedProvider, workspacePath: wp }} />;
    case "memory":
      return <LazyPanel Component={MemoryPanel} props={{ workspacePath: wp }} />;
    case "history":
      return <LazyPanel Component={HistoryPanel} props={{}} />;
    case "checkpoints":
      return <LazyPanel Component={CheckpointPanel} props={{ workspacePath: wp }} />;
    case "artifacts":
      return <LazyPanel Component={ArtifactsPanel} props={{ artifacts: [] }} />;
    case "manager":
      return <LazyPanel Component={ManagerView} props={{ provider: selectedProvider }} />;
    case "hooks":
      return <LazyPanel Component={HooksPanel} props={{ workspacePath: wp }} />;
    case "jobs":
      return <LazyPanel Component={BackgroundJobsPanel} props={{}} />;
    case "mcp":
      return <LazyPanel Component={McpPanel} props={{}} />;
    case "settings":
      return <LazyPanel Component={SettingsPanel} props={{}} />;
    case "cascade":
      return <LazyPanel Component={CascadePanel} props={{ onInjectContext }} />;
    case "specs":
      return <LazyPanel Component={SpecPanel} props={{ workspacePath: wp, provider: selectedProvider }} />;
    case "workflow":
      return <LazyPanel Component={WorkflowPanel} props={{ workspacePath: wp, provider: selectedProvider }} />;
    case "orchestration":
      return <LazyPanel Component={OrchestrationPanel} props={{ workspacePath: wp }} />;
    case "design":
      return <LazyPanel Component={DesignMode} props={{ workspacePath: wp, provider: selectedProvider }} />;
    case "deploy":
      return <LazyPanel Component={DeployPanel} props={{ workspacePath: wp }} />;
    case "database":
      return <LazyPanel Component={DatabasePanel} props={{ workspacePath: wp, provider: selectedProvider }} />;
    case "supabase":
      return <LazyPanel Component={SupabasePanel} props={{ workspacePath: wp, provider: selectedProvider }} />;
    case "auth":
      return <LazyPanel Component={AuthPanel} props={{ workspacePath: wp, provider: selectedProvider }} />;
    case "github":
      return <LazyPanel Component={GitHubSyncPanel} props={{ workspacePath: wp }} />;
    case "steering":
      return <LazyPanel Component={SteeringPanel} props={{ workspaceRoot: wp || undefined }} />;
    case "bugbot":
      return <LazyPanel Component={BugBotPanel} props={{ workspacePath: wp || undefined, onOpenFile }} />;
    case "redteam":
      return <LazyPanel Component={RedTeamPanel} props={{ workspacePath: wp, provider: selectedProvider }} />;
    case "tests":
      return <LazyPanel Component={TestPanel} props={{ workspacePath: wp }} />;
    case "collab":
      return <LazyPanel Component={CollabPanel} props={{ connected: collab.connected, roomId: collab.roomId || "", peerId: collab.peerId || "", peers: collab.peers, onConnect: collab.connect, onDisconnect: collab.disconnect }} />;
    case "coverage":
      return <LazyPanel Component={CoveragePanel} props={{ workspacePath: wp }} />;
    case "compare":
      return <LazyPanel Component={MultiModelPanel} props={{}} />;
    case "http":
      return <LazyPanel Component={HttpPlayground} props={{ workspacePath: wp }} />;
    case "arena":
      return <LazyPanel Component={ArenaPanel} props={{}} />;
    case "cost":
      return <LazyPanel Component={CostPanel} props={{}} />;
    case "autofix":
      return <LazyPanel Component={AutofixPanel} props={{ workspacePath: wp }} />;
    case "processes":
      return <LazyPanel Component={ProcessPanel} props={{}} />;
    case "cicd":
      return <LazyPanel Component={CicdPanel} props={{ workspacePath: wp }} />;
    case "k8s":
      return <LazyPanel Component={K8sPanel} props={{ workspacePath: wp }} />;
    case "env":
      return <LazyPanel Component={EnvPanel} props={{ workspacePath: wp }} />;
    case "profiler":
      return <LazyPanel Component={ProfilerPanel} props={{ workspacePath: wp }} />;
    case "docker":
      return <LazyPanel Component={DockerPanel} props={{ workspacePath: wp }} />;
    case "deps":
      return <LazyPanel Component={DepsPanel} props={{ workspacePath: wp }} />;
    case "apidocs":
      return <LazyPanel Component={ApiDocsPanel} props={{ workspacePath: wp }} />;
    case "migrations":
      return <LazyPanel Component={MigrationsPanel} props={{ workspacePath: wp }} />;
    case "logs":
      return <LazyPanel Component={LogPanel} props={{ workspacePath: wp }} />;
    case "scripts":
      return <LazyPanel Component={ScriptPanel} props={{ workspacePath: wp }} />;
    case "notebook":
      return <LazyPanel Component={NotebookPanel} props={{ workspacePath: wp }} />;
    case "ssh":
      return <LazyPanel Component={SshPanel} props={{ workspacePath: wp }} />;
    case "utils":
      return <LazyPanel Component={UtilitiesPanel} props={{}} />;
    case "markers":
      return <LazyPanel Component={BookmarkPanel} props={{ workspacePath: wp }} />;
    case "bisect":
      return <LazyPanel Component={BisectPanel} props={{ workspacePath: wp }} />;
    case "snippets":
      return <LazyPanel Component={SnippetPanel} props={{ workspacePath: wp }} />;
    case "mock":
      return <LazyPanel Component={MockServerPanel} props={{}} />;
    case "graphql":
      return <LazyPanel Component={GraphQLPanel} props={{}} />;
    case "metrics":
      return <LazyPanel Component={CodeMetricsPanel} props={{ workspacePath: wp }} />;
    case "loadtest":
      return <LazyPanel Component={LoadTestPanel} props={{}} />;
    case "network":
      return <LazyPanel Component={NetworkPanel} props={{}} />;
    case "teams":
      return <LazyPanel Component={AgentTeamPanel} props={{}} />;
    case "cibot":
      return <LazyPanel Component={CIReviewPanel} props={{}} />;
    case "traces":
      return <LazyPanel Component={TraceDashboard} props={{}} />;
    case "marketplace":
      return <LazyPanel Component={MarketplacePanel} props={{}} />;
    case "transform":
      return <LazyPanel Component={TransformPanel} props={{}} />;
    case "img2app":
      return <LazyPanel Component={ScreenshotToApp} props={{ workspacePath: wp }} />;
    case "recording":
      return <LazyPanel Component={AgentRecordingPanel} props={{}} />;
    case "visualtest":
      return <LazyPanel Component={VisualTestPanel} props={{}} />;
    case "cloud":
      return <LazyPanel Component={CloudAgentPanel} props={{}} />;
    case "compliance":
      return <LazyPanel Component={CompliancePanel} props={{}} />;
    case "scaffold":
      return <LazyPanel Component={ScaffoldPanel} props={{ workspacePath: wp }} />;
    case "health":
      return <LazyPanel Component={HealthMonitorPanel} props={{}} />;
    case "websocket":
      return <LazyPanel Component={WebSocketPanel} props={{}} />;
    case "colors":
      return <LazyPanel Component={ColorPalettePanel} props={{ workspacePath: wp }} />;
    case "markdown":
      return <LazyPanel Component={MarkdownPanel} props={{ workspacePath: wp }} />;
    case "difftool":
      return <LazyPanel Component={DiffToolPanel} props={{}} />;
    case "canvas":
      return <LazyPanel Component={CanvasPanel} props={{}} />;
    case "cron":
      return <LazyPanel Component={CronPanel} props={{}} />;
    case "regex":
      return <LazyPanel Component={RegexPanel} props={{}} />;
    case "jwt":
      return <LazyPanel Component={JwtPanel} props={{}} />;
    case "jsontools":
      return <LazyPanel Component={JsonToolsPanel} props={{}} />;
    case "encoding":
      return <LazyPanel Component={EncodingPanel} props={{}} />;
    case "numbers":
      return <LazyPanel Component={NumberBasePanel} props={{}} />;
    case "datagen":
      return <LazyPanel Component={DataGenPanel} props={{}} />;
    case "timestamp":
      return <LazyPanel Component={TimestampPanel} props={{}} />;
    case "colorconv":
      return <LazyPanel Component={ColorConverterPanel} props={{}} />;
    case "cidr":
      return <LazyPanel Component={CidrPanel} props={{}} />;
    case "csv":
      return <LazyPanel Component={CsvPanel} props={{}} />;
    case "units":
      return <LazyPanel Component={UnitConverterPanel} props={{}} />;
    case "unicode":
      return <LazyPanel Component={UnicodePanel} props={{}} />;
    case "sandbox":
      return <LazyPanel Component={SandboxPanel} props={{}} />;
    case "dashboard":
      return <LazyPanel Component={DashboardPanel} props={{}} />;
    case "webhooks":
      return <LazyPanel Component={WebhookPanel} props={{}} />;
    case "admin":
      return <LazyPanel Component={AdminPanel} props={{}} />;
    case "appbuilder":
      return <LazyPanel Component={AppBuilderPanel} props={{ workspacePath: wp || "" }} />;
    case "icontext":
      return <LazyPanel Component={InfiniteContextPanel} props={{ workspacePath: wp || "" }} />;
    case "batchbuilder":
      return <LazyPanel Component={BatchBuilderPanel} props={{}} />;
    case "streaming":
      return <LazyPanel Component={StreamingPanel} props={{}} />;
    case "inference":
      return <LazyPanel Component={InferencePanel} props={{}} />;
    case "training":
      return <LazyPanel Component={TrainingPanel} props={{}} />;
    case "ingest":
      return <LazyPanel Component={DocumentIngestPanel} props={{}} />;
    case "crawler":
      return <LazyPanel Component={WebCrawlerPanel} props={{}} />;
    case "vectordb":
      return <LazyPanel Component={VectorDbPanel} props={{}} />;
    case "qa-validation":
      return <LazyPanel Component={QaValidationPanel} props={{}} />;
    case "astedit":
      return <LazyPanel Component={AstEditPanel} props={{}} />;
    case "cistatus":
      return <LazyPanel Component={CiStatusPanel} props={{}} />;
    case "cloudsandbox":
      return <LazyPanel Component={CloudSandboxPanel} props={{}} />;
    case "editpredict":
      return <LazyPanel Component={EditPredictionPanel} props={{}} />;
    case "plandoc":
      return <LazyPanel Component={PlanDocumentPanel} props={{}} />;
    case "remotecontrol":
      return <LazyPanel Component={RemoteControlPanel} props={{}} />;
    case "securityscan":
      return <LazyPanel Component={SecurityScanPanel} props={{ workspacePath: wp || undefined, onOpenFile }} />;
    case "sessions":
      return <LazyPanel Component={SessionBrowserPanel} props={{}} />;
    case "subagents":
      return <LazyPanel Component={SubAgentPanel} props={{}} />;
    case "clarify":
      return <LazyPanel Component={ClarifyingQuestionsPanel} props={{}} />;
    case "codesearch":
      return <LazyPanel Component={ConversationalSearchPanel} props={{}} />;
    case "demo":
      return <LazyPanel Component={DemoPanel} props={{}} />;
    case "cloudautofix":
      return <LazyPanel Component={CloudAutofixPanel} props={{}} />;
    case "fastcontext":
      return <LazyPanel Component={FastContextPanel} props={{}} />;
    case "imagegen":
      return <LazyPanel Component={ImageGenPanel} props={{}} />;
    case "governance":
      return <LazyPanel Component={TeamGovernancePanel} props={{}} />;
    case "agentteams":
      return <LazyPanel Component={AgentTeamsPanel} props={{}} />;
    case "discuss":
      return <LazyPanel Component={DiscussionModePanel} props={{}} />;
    case "fullstack":
      return <LazyPanel Component={FullStackGenPanel} props={{}} />;
    case "ghactions":
      return <LazyPanel Component={GhActionsPanel} props={{}} />;
    case "renderopt":
      return <LazyPanel Component={RenderOptimizePanel} props={{}} />;
    case "soul":
      return <LazyPanel Component={SoulPanel} props={{ workspacePath: wp }} />;
    case "mcplazy":
      return <LazyPanel Component={McpLazyPanel} props={{}} />;
    case "bundles":
      return <LazyPanel Component={ContextBundlePanel} props={{ workspacePath: wp }} />;
    case "cloudproviders":
      return <LazyPanel Component={CloudProviderPanel} props={{ workspacePath: wp }} />;
    case "acpprotocol":
      return <LazyPanel Component={AcpPanel} props={{}} />;
    case "mcpdirectory":
      return <LazyPanel Component={McpDirectoryPanel} props={{}} />;
    case "usagemetering":
      return <LazyPanel Component={UsageMeteringPanel} props={{}} />;
    case "swebench":
      return <LazyPanel Component={SweBenchPanel} props={{}} />;
    case "sessionmemory":
      return <LazyPanel Component={SessionMemoryPanel} props={{}} />;
    case "blueteam":
      return <LazyPanel Component={BlueTeamPanel} props={{}} />;
    case "purpleteam":
      return <LazyPanel Component={PurpleTeamPanel} props={{}} />;
    case "idp":
      return <LazyPanel Component={IdpPanel} props={{}} />;
    default:
      return <div style={{ padding: 16, color: "var(--text-secondary)" }}>Unknown panel: {tab}</div>;
  }
}
