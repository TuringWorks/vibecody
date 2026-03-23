import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const SettingsPanel = lazy(() => import("../SettingsPanel").then(m => ({ default: m.SettingsPanel }))) as any;
const HooksPanel = lazy(() => import("../HooksPanel").then(m => ({ default: m.HooksPanel }))) as any;
const BookmarkPanel = lazy(() => import("../BookmarkPanel").then(m => ({ default: m.BookmarkPanel }))) as any;
const BackgroundJobsPanel = lazy(() => import("../BackgroundJobsPanel").then(m => ({ default: m.BackgroundJobsPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
}

export function ConfigComposite({ workspacePath: wp }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "settings", label: "Keys", content: <Suspense fallback={<Loading />}><SettingsPanel /></Suspense> },
      { id: "hooks", label: "Hooks", content: <Suspense fallback={<Loading />}><HooksPanel workspacePath={wp} /></Suspense> },
      { id: "markers", label: "Bookmarks", content: <Suspense fallback={<Loading />}><BookmarkPanel workspacePath={wp} /></Suspense> },
      { id: "jobs", label: "Jobs", content: <Suspense fallback={<Loading />}><BackgroundJobsPanel /></Suspense> },
    ]} />
  );
}
