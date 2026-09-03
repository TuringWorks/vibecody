/**
 * DesignAnnotationsPanel — annotate elements in the workspace and turn
 * annotations into actionable design instructions + extracted tokens.
 *
 * (Renamed from DesignModePanel to disambiguate from DesignMode.tsx, which
 * is the larger multi-tab design hub.)
 */
import { useCallback, useEffect, useState } from "react";
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

/** `DesignTokenType`'s serde names, as a reader would say them. */
const TOKEN_CATEGORY_LABEL: Record<string, string> = {
  color: "Color",
  typography: "Typography",
  spacing: "Spacing",
  border_radius: "Radius",
  shadow: "Shadow",
  animation: "Motion",
  breakpoint: "Breakpoint",
  z_index: "Z-index",
  other: "Other",
};

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

/** Render an RFC3339 timestamp in the viewer's locale. Older records stored a
 *  bare unix-seconds string; those are shown as they are rather than as a date
 *  in 1970. */
function formatCreatedAt(raw: string): string {
  if (!raw) return "";
  const at = new Date(/^\d+$/.test(raw) ? Number(raw) * 1000 : raw);
  return Number.isNaN(at.getTime()) ? raw : at.toLocaleString();
}

export function DesignAnnotationsPanel() {
  const { toasts, toast, dismiss } = useToast();
  const [tab, setTab] = useState<"annotate" | "instructions" | "tokens">("annotate");
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [instructions, setInstructions] = useState<Instruction[]>([]);
  const [tokens, setTokens] = useState<DesignToken[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [newKind, setNewKind] = useState<typeof ANNOTATION_KINDS[number]>("spacing");
  const [newDesc, setNewDesc] = useState("");
  const [newSelector, setNewSelector] = useState("");
  const [generating, setGenerating] = useState(false);

  /** Re-read everything the backend derives from the annotation list. */
  const reload = useCallback(async () => {
    try {
      const [annRes, instrRes, tokenRes] = await Promise.all([
        invoke<Annotation[]>("design_mode_annotations"),
        invoke<Instruction[]>("design_mode_generate"),
        invoke<DesignToken[]>("design_mode_tokens"),
      ]);
      setAnnotations(Array.isArray(annRes) ? annRes : []);
      setInstructions(Array.isArray(instrRes) ? instrRes : []);
      setTokens(Array.isArray(tokenRes) ? tokenRes : []);
      setLoadError(null);
    } catch (e) {
      // An unreadable store is not an empty one — the panel used to show
      // "No annotations yet." either way. The banner persists; the toast is
      // what makes the failure noticeable on a tab the user is not looking at.
      setLoadError(String(e));
      toast.error(`Failed to load design annotations: ${e}`);
    }
  }, [toast]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    reload().finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [reload]);

  async function addAnnotation() {
    if (!newDesc.trim()) return;
    try {
      const ann = await invoke<Annotation>("design_mode_annotations", {
        action: "add",
        kind: newKind,
        description: newDesc.trim(),
        selector: newSelector.trim() || null,
      });
      if (ann) toast.success(`Added ${ann.kind} annotation`);
      setNewDesc("");
      setNewSelector("");
      // Instructions are derived from the annotations, so adding one changes
      // them. Appending locally left the Instructions tab a step behind.
      await reload();
    } catch (e) {
      toast.error(`Failed to add annotation: ${e}`);
    }
  }

  async function deleteAnnotation(id: string) {
    setDeleting(id);
    try {
      await invoke("design_mode_annotations", { action: "delete", id });
      toast.success("Annotation deleted");
      await reload();
    } catch (e) {
      toast.error(`Failed to delete annotation: ${e}`);
    } finally {
      setDeleting(null);
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

        {!loading && loadError && (
          <div role="alert" style={{ color: "var(--error-color)", fontSize: "var(--font-size-base)", marginBottom: "var(--space-3)" }}>
            Could not read design annotations: {loadError}
          </div>
        )}

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
                        <span
                          title={ann.created_at}
                          style={{ marginLeft: "auto", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}
                        >
                          {formatCreatedAt(ann.created_at)}
                        </span>
                        <button
                          type="button"
                          className="panel-btn panel-btn-secondary panel-btn-sm"
                          aria-label={`Delete ${ann.kind} annotation`}
                          onClick={() => deleteAnnotation(ann.id)}
                          disabled={deleting === ann.id}
                        >
                          {deleting === ann.id ? "…" : "Delete"}
                        </button>
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
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: "var(--space-3)" }}>
              CSS custom properties declared in this workspace's stylesheets.
            </div>
            {Object.keys(tokensByCategory).length === 0 && (
              <div className="panel-empty">
                No CSS custom properties found in this workspace's stylesheets.
              </div>
            )}
            {Object.entries(tokensByCategory).map(([category, categoryTokens]) => (
              <div key={category} style={{ marginBottom: "var(--space-5)" }}>
                <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--text-muted)", marginBottom: "var(--space-2)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                  {TOKEN_CATEGORY_LABEL[category] ?? category}
                </div>
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
