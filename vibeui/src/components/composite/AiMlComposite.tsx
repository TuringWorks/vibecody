import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const AiMlWorkflowPanel = lazy(() => import("../AiMlWorkflowPanel").then(m => ({ default: m.AiMlWorkflowPanel }))) as any;
const ModelWizardPanel = lazy(() => import("../ModelWizardPanel").then(m => ({ default: m.ModelWizardPanel }))) as any;
const TrainingPanel = lazy(() => import("../TrainingPanel").then(m => ({ default: m.TrainingPanel }))) as any;
const InferencePanel = lazy(() => import("../InferencePanel").then(m => ({ default: m.InferencePanel }))) as any;
const QuantumComputingPanel = lazy(() => import("../QuantumComputingPanel").then(m => ({ default: m.QuantumComputingPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  provider: string;
}

export function AiMlComposite({ provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "workflow", label: "Workflow", content: <Suspense fallback={<Loading />}><AiMlWorkflowPanel /></Suspense> },
      { id: "wizard", label: "Wizard", content: <Suspense fallback={<Loading />}><ModelWizardPanel /></Suspense> },
      { id: "training", label: "Training", content: <Suspense fallback={<Loading />}><TrainingPanel provider={provider} /></Suspense> },
      { id: "inference", label: "Inference", content: <Suspense fallback={<Loading />}><InferencePanel provider={provider} /></Suspense> },
      { id: "quantum", label: "Quantum", content: <Suspense fallback={<Loading />}><QuantumComputingPanel provider={provider} /></Suspense> },
    ]} />
  );
}
