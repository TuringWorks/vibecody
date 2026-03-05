/**
 * TransformPanel — Code Transformation Agent.
 *
 * Automated language/framework upgrades: detect, plan, review, execute.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TransformType {
  id: string;
  label: string;
  description: string;
}

interface TransformPlanItem {
  file: string;
  description: string;
  estimated_changes: number;
}

const TRANSFORMS: TransformType[] = [
  { id: "commonjs_to_esm", label: "CommonJS → ESM", description: "Convert require() to import/export" },
  { id: "react_class_to_hooks", label: "React Class → Hooks", description: "Convert class components to functional with hooks" },
  { id: "python2_to3", label: "Python 2 → 3", description: "Upgrade Python 2 patterns to Python 3" },
  { id: "vue2_to3", label: "Vue 2 → 3", description: "Migrate Vue 2 options API to Vue 3 composition API" },
  { id: "java_upgrade", label: "Java Upgrade", description: "Upgrade Java version patterns and APIs" },
];

export function TransformPanel() {
  const [selected, setSelected] = useState<string | null>(null);
  const [plan, setPlan] = useState<TransformPlanItem[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);

  const handlePlan = async () => {
    if (!selected) return;
    setLoading(true);
    setError(null);
    setPlan(null);
    try {
      const p = await invoke<{ files: TransformPlanItem[] }>("plan_transform", { transformType: selected });
      setPlan(p.files);
    } catch (e) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleExecute = async () => {
    if (!selected || !plan) return;
    setExecuting(true);
    setProgress(0);
    setResult(null);
    setError(null);
    try {
      const res = await invoke<{ files_modified: number; summary: string }>("execute_transform", {
        transformType: selected,
        files: plan.map((p) => p.file),
      });
      setResult(`${res.files_modified} files transformed. ${res.summary}`);
    } catch (e) {
      setError(String(e));
    }
    setExecuting(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div style={{
        padding: "8px 12px", borderBottom: "1px solid var(--border, #2a2a3e)",
        display: "flex", alignItems: "center", gap: 8,
      }}>
        <span style={{ fontSize: 14, fontWeight: 700 }}>Code Transform</span>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px" }}>
        {/* Transform type selector */}
        <div style={{ fontSize: 11, color: "var(--text-secondary, #a6adc8)", marginBottom: 8 }}>
          Select a code transformation to apply across your workspace.
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 12 }}>
          {TRANSFORMS.map((t) => (
            <button key={t.id} onClick={() => { setSelected(t.id); setPlan(null); setResult(null); }} style={{
              padding: "6px 10px", borderRadius: 4, textAlign: "left", cursor: "pointer",
              border: selected === t.id ? "1px solid #6366f1" : "1px solid var(--border, #2a2a3e)",
              background: selected === t.id ? "rgba(99,102,241,0.15)" : "var(--bg-primary, #11111b)",
              color: "var(--text-primary, #cdd6f4)",
            }}>
              <div style={{ fontSize: 11, fontWeight: 600 }}>{t.label}</div>
              <div style={{ fontSize: 10, opacity: 0.6 }}>{t.description}</div>
            </button>
          ))}
        </div>

        {selected && !plan && (
          <button onClick={handlePlan} disabled={loading} style={{
            ...btnStyle, background: "#6366f1", color: "#fff", fontWeight: 700,
            opacity: loading ? 0.5 : 1, width: "100%",
          }}>
            {loading ? "Analyzing..." : "Generate Plan"}
          </button>
        )}

        {error && (
          <div style={{ fontSize: 11, color: "#f38ba8", padding: "4px 8px", background: "rgba(243,139,168,0.05)", borderRadius: 4, marginTop: 8 }}>
            {error}
          </div>
        )}

        {/* Plan view */}
        {plan && (
          <div style={{ marginTop: 8 }}>
            <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>
              Plan: {plan.length} file(s)
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 4, marginBottom: 8 }}>
              {plan.map((p, i) => (
                <div key={i} style={{
                  padding: "4px 8px", fontSize: 10, borderRadius: 3,
                  background: "var(--bg-primary, #11111b)",
                  border: "1px solid var(--border, #2a2a3e)",
                }}>
                  <div style={{ fontWeight: 600, fontFamily: "monospace" }}>{p.file}</div>
                  <div style={{ opacity: 0.6 }}>{p.description} (~{p.estimated_changes} changes)</div>
                </div>
              ))}
            </div>
            <button onClick={handleExecute} disabled={executing} style={{
              ...btnStyle, background: "#a6e3a1", color: "#1e1e2e", fontWeight: 700,
              opacity: executing ? 0.5 : 1, width: "100%",
            }}>
              {executing ? `Transforming... ${progress}%` : "Execute Transform"}
            </button>
          </div>
        )}

        {result && (
          <div style={{ fontSize: 11, color: "#a6e3a1", padding: "6px 8px", background: "rgba(166,227,161,0.05)", borderRadius: 4, marginTop: 8 }}>
            {result}
          </div>
        )}
      </div>
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  padding: "6px 12px", fontSize: 12, fontWeight: 600,
  border: "1px solid var(--border, #2a2a3e)", borderRadius: 4,
  background: "var(--bg-secondary, #1e1e2e)", color: "var(--text-primary, #cdd6f4)",
  cursor: "pointer",
};
