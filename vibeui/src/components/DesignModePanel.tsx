import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

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

const ANNOTATION_KINDS = ["spacing", "color", "typography", "layout", "component", "interaction"];

const KIND_COLORS: Record<string, string> = {
  spacing: "#4a9eff",
  color: "#e85d8a",
  typography: "#9c6fe0",
  layout: "#f0a050",
  component: "#4caf7d",
  interaction: "#50c8e8",
};

export function DesignModePanel() {
  const [tab, setTab] = useState("annotate");
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [instructions, setInstructions] = useState<Instruction[]>([]);
  const [tokens, setTokens] = useState<DesignToken[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [newKind, setNewKind] = useState("spacing");
  const [newDesc, setNewDesc] = useState("");
  const [newSelector, setNewSelector] = useState("");
  const [generating, setGenerating] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [annRes, instrRes, tokenRes] = await Promise.all([
          invoke<Annotation[]>("design_mode_annotations"),
          invoke<Instruction[]>("design_mode_generate"),
          invoke<DesignToken[]>("design_mode_tokens"),
        ]);
        setAnnotations(Array.isArray(annRes) ? annRes : []);
        setInstructions(Array.isArray(instrRes) ? instrRes : []);
        setTokens(Array.isArray(tokenRes) ? tokenRes : []);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
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
      if (ann) setAnnotations(prev => [...prev, ann]);
      setNewDesc("");
      setNewSelector("");
    } catch (e) {
      setError(String(e));
    }
  }

  async function regenerateInstructions() {
    setGenerating(true);
    try {
      const res = await invoke<Instruction[]>("design_mode_generate");
      setInstructions(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(String(e));
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
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>Design Mode</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["annotate", "instructions", "tokens"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "annotate" && (
        <div>
          <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: 14, marginBottom: 16 }}>
            <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 10 }}>Add Annotation</div>
            <div style={{ display: "flex", gap: 8, marginBottom: 10, flexWrap: "wrap" }}>
              <div style={{ flex: "0 0 auto" }}>
                <label style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: 4 }}>Kind</label>
                <select value={newKind} onChange={e => setNewKind(e.target.value)}
                  style={{ padding: "5px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>
                  {ANNOTATION_KINDS.map(k => <option key={k} value={k}>{k}</option>)}
                </select>
              </div>
              <div style={{ flex: 1, minWidth: 150 }}>
                <label style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: 4 }}>Selector (optional)</label>
                <input value={newSelector} onChange={e => setNewSelector(e.target.value)}
                  placeholder=".btn-primary"
                  style={{ width: "100%", padding: "5px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", boxSizing: "border-box" }} />
              </div>
            </div>
            <div style={{ marginBottom: 10 }}>
              <label style={{ display: "block", fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: 4 }}>Description</label>
              <textarea value={newDesc} onChange={e => setNewDesc(e.target.value)}
                placeholder="Describe the design annotation..."
                style={{ width: "100%", height: 60, padding: "6px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", resize: "vertical", boxSizing: "border-box" }} />
            </div>
            <button onClick={addAnnotation} disabled={!newDesc.trim()}
              style={{ padding: "6px 16px", borderRadius: "var(--radius-sm)", cursor: newDesc.trim() ? "pointer" : "not-allowed", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-base)", fontWeight: 600, opacity: newDesc.trim() ? 1 : 0.6 }}>
              Add Annotation
            </button>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {annotations.length === 0 && <div style={{ color: "var(--text-muted)" }}>No annotations yet.</div>}
            {annotations.map(ann => {
              const color = KIND_COLORS[ann.kind] ?? "var(--text-muted)";
              return (
                <div key={ann.id} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: `1px solid var(--border-color)`, borderLeft: `3px solid ${color}`, padding: "10px 14px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                    <span style={{ fontSize: "var(--font-size-sm)", padding: "1px 8px", borderRadius: "var(--radius-sm-alt)", background: color + "22", color, fontWeight: 600 }}>{ann.kind}</span>
                    {ann.selector && <code style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", background: "var(--bg-primary)", padding: "1px 6px", borderRadius: "var(--radius-xs-plus)" }}>{ann.selector}</code>}
                    <span style={{ marginLeft: "auto", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>{ann.created_at}</span>
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{ann.description}</div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {!loading && tab === "instructions" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>{instructions.length} instructions generated</span>
            <button onClick={regenerateInstructions} disabled={generating}
              style={{ padding: "4px 14px", borderRadius: "var(--radius-sm)", cursor: generating ? "not-allowed" : "pointer", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", opacity: generating ? 0.6 : 1 }}>
              {generating ? "Regenerating…" : "Regenerate"}
            </button>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {instructions.length === 0 && <div style={{ color: "var(--text-muted)" }}>No instructions generated. Add annotations first.</div>}
            {instructions.map(instr => (
              <div key={instr.index} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: "10px 14px", display: "flex", gap: 12 }}>
                <span style={{ fontSize: "var(--font-size-md)", fontWeight: 700, color: "var(--accent-color)", minWidth: 24 }}>{instr.index}.</span>
                <div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", lineHeight: 1.5 }}>{instr.text}</div>
                  {instr.source_annotation_ids.length > 0 && (
                    <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginTop: 4 }}>
                      Sources: {instr.source_annotation_ids.map(id => id.slice(0, 6)).join(", ")}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {!loading && tab === "tokens" && (
        <div>
          {Object.keys(tokensByCategory).length === 0 && <div style={{ color: "var(--text-muted)" }}>No design tokens extracted.</div>}
          {Object.entries(tokensByCategory).map(([category, categoryTokens]) => (
            <div key={category} style={{ marginBottom: 20 }}>
              <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--text-muted)", marginBottom: 8, textTransform: "uppercase", letterSpacing: "0.05em" }}>{category}</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                {categoryTokens.map(token => (
                  <div key={token.name} style={{ display: "flex", alignItems: "center", gap: 12, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: "6px 12px", border: "1px solid var(--border-color)" }}>
                    {category === "color" && (
                      <div style={{ width: 20, height: 20, borderRadius: "var(--radius-xs-plus)", background: token.value, border: "1px solid var(--border-color)", flexShrink: 0 }} />
                    )}
                    <code style={{ fontSize: "var(--font-size-base)", color: "var(--accent-color)", flex: 1 }}>{token.name}</code>
                    <code style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>{token.value}</code>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
