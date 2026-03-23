import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const ArtifactsPanel = lazy(() => import("../ArtifactsPanel").then(m => ({ default: m.ArtifactsPanel }))) as any;
const ScreenshotToApp = lazy(() => import("../ScreenshotToApp").then(m => ({ default: m.ScreenshotToApp }))) as any;
const VisualTestPanel = lazy(() => import("../VisualTestPanel").then(m => ({ default: m.VisualTestPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function ToolsSettingsComposite({ workspacePath: wp, provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "artifacts", label: "Artifacts", content: <Suspense fallback={<Loading />}><ArtifactsPanel artifacts={[]} /></Suspense> },
      { id: "img2app", label: "Img2App", content: <Suspense fallback={<Loading />}><ScreenshotToApp workspacePath={wp} provider={provider} /></Suspense> },
      { id: "visualtest", label: "Visual Test", content: <Suspense fallback={<Loading />}><VisualTestPanel provider={provider} /></Suspense> },
    ]} />
  );
}
