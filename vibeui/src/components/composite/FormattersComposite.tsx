import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const RegexPanel = lazy(() => import("../RegexPanel").then(m => ({ default: m.RegexPanel }))) as any;
const JwtPanel = lazy(() => import("../JwtPanel").then(m => ({ default: m.JwtPanel }))) as any;
const JsonToolsPanel = lazy(() => import("../JsonToolsPanel").then(m => ({ default: m.JsonToolsPanel }))) as any;
const CronPanel = lazy(() => import("../CronPanel").then(m => ({ default: m.CronPanel }))) as any;
const CsvPanel = lazy(() => import("../CsvPanel").then(m => ({ default: m.CsvPanel }))) as any;
const CidrPanel = lazy(() => import("../CidrPanel").then(m => ({ default: m.CidrPanel }))) as any;
const DataGenPanel = lazy(() => import("../DataGenPanel").then(m => ({ default: m.DataGenPanel }))) as any;
const UtilitiesPanel = lazy(() => import("../UtilitiesPanel").then(m => ({ default: m.UtilitiesPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

export function FormattersComposite() {
  return (
    <TabbedPanel tabs={[
      { id: "regex", label: "Regex", content: <Suspense fallback={<Loading />}><RegexPanel /></Suspense> },
      { id: "jwt", label: "JWT", content: <Suspense fallback={<Loading />}><JwtPanel /></Suspense> },
      { id: "jsontools", label: "JSON", content: <Suspense fallback={<Loading />}><JsonToolsPanel /></Suspense> },
      { id: "cron", label: "Cron", content: <Suspense fallback={<Loading />}><CronPanel /></Suspense> },
      { id: "csv", label: "CSV", content: <Suspense fallback={<Loading />}><CsvPanel /></Suspense> },
      { id: "cidr", label: "CIDR", content: <Suspense fallback={<Loading />}><CidrPanel /></Suspense> },
      { id: "datagen", label: "Data Gen", content: <Suspense fallback={<Loading />}><DataGenPanel /></Suspense> },
      { id: "utils", label: "Utils", content: <Suspense fallback={<Loading />}><UtilitiesPanel /></Suspense> },
    ]} />
  );
}
