import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const EncodingPanel = lazy(() => import("../EncodingPanel").then(m => ({ default: m.EncodingPanel }))) as any;
const NumberBasePanel = lazy(() => import("../NumberBasePanel").then(m => ({ default: m.NumberBasePanel }))) as any;
const ColorConverterPanel = lazy(() => import("../ColorConverterPanel").then(m => ({ default: m.ColorConverterPanel }))) as any;
const UnitConverterPanel = lazy(() => import("../UnitConverterPanel").then(m => ({ default: m.UnitConverterPanel }))) as any;
const UnicodePanel = lazy(() => import("../UnicodePanel").then(m => ({ default: m.UnicodePanel }))) as any;
const TimestampPanel = lazy(() => import("../TimestampPanel").then(m => ({ default: m.TimestampPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

export function ConvertersComposite() {
  return (
    <TabbedPanel tabs={[
      { id: "encoding", label: "Encoding", content: <Suspense fallback={<Loading />}><EncodingPanel /></Suspense> },
      { id: "numbers", label: "Numbers", content: <Suspense fallback={<Loading />}><NumberBasePanel /></Suspense> },
      { id: "colorconv", label: "Colors", content: <Suspense fallback={<Loading />}><ColorConverterPanel /></Suspense> },
      { id: "units", label: "Units", content: <Suspense fallback={<Loading />}><UnitConverterPanel /></Suspense> },
      { id: "unicode", label: "Unicode", content: <Suspense fallback={<Loading />}><UnicodePanel /></Suspense> },
      { id: "timestamp", label: "Timestamp", content: <Suspense fallback={<Loading />}><TimestampPanel /></Suspense> },
    ]} />
  );
}
