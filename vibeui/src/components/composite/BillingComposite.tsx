import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const CostPanel = lazy(() => import("../CostPanel").then(m => ({ default: m.CostPanel }))) as any;
const UsageMeteringPanel = lazy(() => import("../UsageMeteringPanel").then(m => ({ default: m.UsageMeteringPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  provider: string;
}

export function BillingComposite({ provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "cost", label: "Cost", content: <Suspense fallback={<Loading />}><CostPanel provider={provider} /></Suspense> },
      { id: "usagemetering", label: "Usage", content: <Suspense fallback={<Loading />}><UsageMeteringPanel /></Suspense> },
    ]} />
  );
}
