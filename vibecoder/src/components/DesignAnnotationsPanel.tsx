/**
 * DesignAnnotationsPanel — annotate elements in the workspace and turn
 * annotations into actionable design instructions + extracted tokens.
 *
 * (Renamed from DesignModePanel to disambiguate from DesignMode.tsx, which
 * is the larger multi-tab design hub.)
 */
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../hooks/useToast";
import { Toaster } from "./Toaster";

interface Annotation {
  id: string;
  kind: "spacing" | "color" | "typography" | "layout" | "component" | "interaction";
  description: string;
  selector: string | null;
  created_at: string;
}

interface Instruction {
  index: number;
  text: string;
  source_annotation_ids: string[];
}

interface DesignToken {
  name: string;
  value: string;
  category: string;
}

const ANNOTATION_KINDS = ["spacing", "color", "typography", "layout", "component", "interaction"] as const;

// Semantic mapping from annotation kind to a design-system color token.
// (Replaces the previous hard-coded #4a9eff / #e85d8a / #9c6fe0 / #f0a050 / #4caf7d / #50c8e8.)
const KIND_VAR: Record<string, string> = {
  spacing: "var(--accent-blue)",
  color: "var(--accent-color)",
  typography: "var(--info-color)",
  layout: "var(--warning-color)",
  component: "var(--success-color)",
  interaction: "var(--accent-blue)",
};

export function DesignAnnotationsPanel() {
  const { toasts, toast, dismiss } = useToast();
  const [tab, setTab] = useState<"annotate" | "instructions" | "tokens">("annotate");
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [instructions, setInstructions] = useState<Instruction[]>([]);
  const [tokens, setTokens] = useState<DesignToken[]>([]);
  const [loading, setLoading] = useState(true);
  const [newKind, setNewKind] = useState<typeof ANNOTATION_KINDS[number]>("spacing");
  const [newDesc, setNewDesc] = useState("");
  const [newSelector, setNewSelector] = useState("");
  const [generating, setGenerating] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      try {
        const [annRes, instrRes, tokenRes] = await Promise.all([
          invoke<Annotation[]>("design_mode_annotations"),
          invoke<Instruction[]>("design_mode_generate"),
          invoke<DesignToken[]>("design_mode_tokens"),
        ]);
        if (cancelled) return;
        setAnnotations(Array.isArray(annRes) ? annRes : []);
        setInstructions(Array.isArray(instrRes) ? instrRes : []);
        setTokens(Array.isArray(tokenRes) ? tokenRes : []);
      } catch (e) {
        if (!cancelled) toast.error(`Failed to load design annotations: ${e}`);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function addAnnotation() {
    if (!newDesc.trim()) return;
    try {
      const ann = await invoke<Annotation>("design_mode_annotations", {
        action: "add",
        kind: newKind,
        description: newDesc.trim(),
        selector: newSelector.trim() || null,
      });
      if (ann) {
        setAnnotations((prev) => [...prev, ann]);
        toast.success(`Added ${ann.kind} annotation`);
      }
      setNewDesc("");
      setNewSelector("");
    } catch (e) {
      toast.error(`Failed to add annotation: ${e}`);
    }
  }

  async function regenerateInstructions() {
    setGenerating(true);
    try {
      const res = await invoke<Instruction[]>("design_mode_generate");
      setInstructions(Array.isArray(res) ? res : []);
      toast.success(`Generated ${res?.length ?? 0} instruction(s)`);
    } catch (e) {
      toast.error(`Regenerate failed: ${e}`);
    } finally {
      setGenerating(false);
    }
  }

  const tokensByCategory = tokens.reduce<Record<string, DesignToken[]>>((acc, t) => {
    acc[t.category] = acc[t.category] ?? [];
    acc[t.category].push(t);
    return acc;
  }, {});

  return (
    <div className="panel-container" role="region" aria-label="Design Annotations">
      <div className="panel-tab-bar" role="tablist" aria-label="Design Annotations tabs">
        {(["annotate", "instructions", "tokens"] as const).map((t) => (
          <button
            key={t}
            type="button"
            role="tab"
            aria-selected={tab === t}
            className={`panel-tab ${tab === t ? "active" : ""}`}
            onClick={() => setTab(t)}
          >
            {t}
          </button>
        ))}
      </div>

      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {loading && <div className="panel-loading">Loading design annotations…</div>}

        {!loading && tab === "annotate" && (
          <>
            <div className="panel-card" style={{ padding: "var(--space-4)", marginBottom: "var(--space-4)" }}>
              <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: "var(--space-3)" }}>Add Annotation</div>
              <div style={{ display: "flex", gap: "var(--space-2)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
                <div style={{ flex: "0 0 auto" }}>
                  <label htmlFor="ann-kind" style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: "var(--space-1)" }}>Kind</label>
                  <select
                    id="ann-kind"
                    className="panel-input"
                    value={newKind}
                    onChange={(e) => setNewKind(e.target.value as typeof ANNOTATION_KINDS[number])}
                  >
                    {ANNOTATION_KINDS.map((k) => <option key={k} value={k}>{k}</option>)}
                  </select>
                </div>
                <div style={{ flex: 1, minWidth: 150 }}>
                  <label htmlFor="ann-selector" style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: "var(--space-1)" }}>Selector (optional)</label>
                  <input
                    id="ann-selector"
                    className="panel-input"
                    value={newSelector}
                    onChange={(e) => setNewSelector(e.target.value)}
                    placeholder=".btn-primary"
                    style={{ width: "100%", boxSizing: "border-box" }}
                  />
                </div>
              </div>
              <div style={{ marginBottom: "var(--space-3)" }}>
                <label htmlFor="ann-desc" style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: "var(--space-1)" }}>Description</label>
                <textarea
                  id="ann-desc"
                  className="panel-input"
                  value={newDesc}
                  onChange={(e) => setNewDesc(e.target.value)}
                  placeholder="Describe the design annotation..."
                  style={{ width: "100%", height: 60, fontFamily: "var(--font-mono)", resize: "vertical", boxSizing: "border-box" }}
                />
              </div>
              <button
                type="button"
                className="panel-btn panel-btn-primary"
                onClick={addAnnotation}
                disabled={!newDesc.trim()}
              >
                Add Annotation
              </button>
            </div>

            {annotations.length === 0 ? (
              <div className="panel-empty">No annotations yet.</div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                {annotations.map((ann) => {
                  const accent = KIND_VAR[ann.kind] ?? "var(--text-muted)";
                  return (
                    <div
                      key={ann.id}
                      className="panel-card"
                      style={{ borderLeft: `3px solid ${accent}`, padding: "var(--space-3) var(--space-4)" }}
                    >
                      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
                        <span className="panel-tag" style={{ color: accent, fontWeight: 600 }}>{ann.kind}</span>
                        {ann.selector && (
                          <code style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", background: "var(--bg-primary)", padding: "1px 8px", borderRadius: "var(--radius-xs-plus)" }}>
                            {ann.selector}
                          </code>
                        )}
                        <span style={{ marginLeft: "auto", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>{ann.created_at}</span>
                      </div>
                      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{ann.description}</div>
                    </div>
                  );
                })}
              </div>
            )}
          </>
        )}

        {!loading && tab === "instructions" && (
          <>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-3)" }}>
              <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>{instructions.length} instructions generated</span>
              <button
                type="button"
                className="panel-btn panel-btn-secondary panel-btn-sm"
                onClick={regenerateInstructions}
                disabled={generating}
              >
                {generating ? "Regenerating…" : "Regenerate"}
              </button>
            </div>
            {instructions.length === 0 ? (
              <div className="panel-empty">No instructions generated. Add annotations first.</div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                {instructions.map((instr) => (
                  <div key={instr.index} className="panel-card" style={{ padding: "var(--space-3) var(--space-4)", display: "flex", gap: "var(--space-3)" }}>
                    <span style={{ fontSize: "var(--font-size-md)", fontWeight: 700, color: "var(--accent-color)", minWidth: 24 }}>{instr.index}.</span>
                    <div>
                      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", lineHeight: 1.5 }}>{instr.text}</div>
                      {instr.source_annotation_ids.length > 0 && (
                        <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginTop: "var(--space-1)" }}>
                          Sources: {instr.source_annotation_ids.map((id) => id.slice(0, 6)).join(", ")}
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </>
        )}

        {!loading && tab === "tokens" && (
          <>
            {Object.keys(tokensByCategory).length === 0 && (
              <div className="panel-empty">No design tokens extracted.</div>
            )}
            {Object.entries(tokensByCategory).map(([category, categoryTokens]) => (
              <div key={category} style={{ marginBottom: "var(--space-5)" }}>
                <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--text-muted)", marginBottom: "var(--space-2)", textTransform: "uppercase", letterSpacing: "0.05em" }}>{category}</div>
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
                  {categoryTokens.map((t) => (
                    <div key={t.name} className="panel-card" style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "var(--space-2) var(--space-3)" }}>
                      {category === "color" && (
                        <div style={{ width: 20, height: 20, borderRadius: "var(--radius-xs-plus)", background: t.value, border: "1px solid var(--border-color)", flexShrink: 0 }} />
                      )}
                      <code style={{ fontSize: "var(--font-size-base)", color: "var(--accent-color)", flex: 1 }}>{t.name}</code>
                      <code title={t.value} style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{t.value}</code>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </>
        )}
      </div>

      <Toaster toasts={toasts} onDismiss={dismiss} />
    </div>
  );
}
