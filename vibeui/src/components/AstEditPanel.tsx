/**
 * AstEditPanel — AST-aware code editing.
 *
 * Tabs: Files (loaded files with node tree), Edits (pending operations),
 * Preview (diff preview of selected edit).
 * Wired to Tauri backend commands for real AST extraction and edit management.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "files" | "edits" | "preview";
type NodeKind = "function" | "struct" | "enum" | "impl" | "trait" | "module" | "const" | "type";
type EditOp = "rename" | "extract" | "inline" | "delete" | "move" | "wrap";

interface AstNode {
  name: string;
  kind: NodeKind;
  line: number;
  children?: AstNode[];
}

interface AstFile {
  path: string;
  language: string;
  nodes: AstNode[];
}

interface PendingEdit {
  id: string;
  file: string;
  target: string;
  operation: EditOp;
  confidence: number;
  description: string;
  diffBefore: string;
  diffAfter: string;
}

const kindColor: Record<NodeKind, string> = {
  function: "var(--text-info)",
  struct: "var(--text-success)",
  enum: "var(--text-warning)",
  impl: "var(--text-muted)",
  trait: "#cba6f7",
  module: "#fab387",
  const: "#94e2d5",
  type: "#f5c2e7",
};

const tabBtn = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px", fontSize: 11, fontWeight: active ? 600 : 400,
  background: active ? "var(--accent-bg, rgba(99,102,241,0.15))" : "transparent",
  border: "1px solid " + (active ? "var(--accent-primary)" : "var(--border-color)"),
  borderRadius: 4, color: active ? "var(--text-info)" : "var(--text-muted)", cursor: "pointer",
});

function NodeTree({ nodes, depth = 0 }: { nodes: AstNode[]; depth?: number }) {
  return (
    <>
      {nodes.map(n => (
        <div key={n.name + n.line}>
          <div style={{ display: "flex", gap: 8, padding: "3px 8px", paddingLeft: 8 + depth * 16, fontSize: 11, alignItems: "center" }}>
            <span style={{ color: kindColor[n.kind], fontSize: 9, fontWeight: 600, minWidth: 50 }}>{n.kind}</span>
            <span style={{ color: "var(--text-primary)", fontFamily: "monospace" }}>{n.name}</span>
            <span style={{ color: "var(--text-muted)", fontSize: 10, marginLeft: "auto" }}>L{n.line}</span>
          </div>
          {n.children && <NodeTree nodes={n.children} depth={depth + 1} />}
        </div>
      ))}
    </>
  );
}

export default function AstEditPanel() {
  const [tab, setTab] = useState<Tab>("files");
  const [selectedEdit, setSelectedEdit] = useState<string>("");
  const [files, setFiles] = useState<AstFile[]>([]);
  const [edits, setEdits] = useState<PendingEdit[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadFiles = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const workspace = await invoke<string[]>("get_workspace_folders")
        .then(folders => folders[0] || ".")
        .catch(() => ".");
      const result = await invoke<AstFile[]>("get_ast_files", { workspace });
      setFiles(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadEdits = useCallback(async () => {
    try {
      const result = await invoke<PendingEdit[]>("get_ast_edits");
      setEdits(result);
      if (result.length > 0 && !selectedEdit) {
        setSelectedEdit(result[0].id);
      }
    } catch (err) {
      console.error("Failed to load AST edits:", err);
    }
  }, [selectedEdit]);

  useEffect(() => {
    loadFiles();
    loadEdits();
  }, [loadFiles, loadEdits]);

  const applyEdit = async (id: string) => {
    try {
      await invoke("apply_ast_edit", { id });
      setEdits(es => es.filter(e => e.id !== id));
    } catch (err) {
      console.error("Failed to apply AST edit:", err);
    }
  };

  const dismissEdit = async (id: string) => {
    try {
      await invoke("dismiss_ast_edit", { id });
      setEdits(es => es.filter(e => e.id !== id));
    } catch (err) {
      console.error("Failed to dismiss AST edit:", err);
    }
  };

  const selected = edits.find(e => e.id === selectedEdit);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div style={{ display: "flex", gap: 6, padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        {(["files", "edits", "preview"] as Tab[]).map(t => (
          <button key={t} onClick={() => setTab(t)} style={tabBtn(tab === t)}>
            {t[0].toUpperCase() + t.slice(1)}
          </button>
        ))}
        <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-muted)", alignSelf: "center" }}>
          {edits.length} pending
        </span>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
        {loading && (
          <div style={{ textAlign: "center", color: "var(--text-muted)", fontSize: 12, padding: 40 }}>Loading...</div>
        )}
        {error && (
          <div style={{ textAlign: "center", color: "var(--text-danger)", fontSize: 12, padding: 20 }}>{error}</div>
        )}

        {tab === "files" && !loading && files.map(f => (
          <div key={f.path} style={{ background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", overflow: "hidden" }}>
            <div style={{ padding: "6px 10px", borderBottom: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontSize: 12, fontWeight: 600, fontFamily: "monospace", color: "var(--text-primary)" }}>{f.path}</span>
              <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{f.language} | {f.nodes.length} top-level</span>
            </div>
            <NodeTree nodes={f.nodes} />
          </div>
        ))}
        {tab === "files" && !loading && files.length === 0 && !error && (
          <div style={{ textAlign: "center", color: "var(--text-muted)", fontSize: 12, padding: 40 }}>No source files found in workspace</div>
        )}

        {tab === "edits" && edits.map(e => (
          <div key={e.id} role="button" tabIndex={0} onClick={() => { setSelectedEdit(e.id); setTab("preview"); }} onKeyDown={ev => ev.key === "Enter" && (setSelectedEdit(e.id), setTab("preview"))}
            style={{ padding: 10, background: selectedEdit === e.id ? "var(--accent-bg, rgba(99,102,241,0.15))" : "var(--bg-secondary)", borderRadius: 6, border: `1px solid ${selectedEdit === e.id ? "var(--accent-primary)" : "var(--border-color)"}`, cursor: "pointer" }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
              <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 3, background: "rgba(99,102,241,0.15)", color: "var(--text-info)", fontWeight: 600 }}>{e.operation}</span>
              <span style={{ fontSize: 11, fontFamily: "monospace", color: "var(--text-primary)" }}>{e.target}</span>
              <span style={{ fontSize: 10, color: "var(--text-muted)", marginLeft: "auto" }}>{e.file}</span>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 6 }}>{e.description}</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <div style={{ flex: 1, height: 4, background: "var(--bg-primary)", borderRadius: 2, overflow: "hidden" }}>
                <div style={{ width: `${e.confidence * 100}%`, height: "100%", background: e.confidence > 0.85 ? "var(--text-success)" : e.confidence > 0.7 ? "var(--text-warning)" : "var(--text-danger)", borderRadius: 2 }} />
              </div>
              <span style={{ fontSize: 10, color: "var(--text-muted)", minWidth: 30 }}>{(e.confidence * 100).toFixed(0)}%</span>
              <button onClick={(ev) => { ev.stopPropagation(); dismissEdit(e.id); }}
                style={{ padding: "3px 8px", fontSize: 10, borderRadius: 3, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-danger)", cursor: "pointer" }}>Reject</button>
              <button onClick={(ev) => { ev.stopPropagation(); applyEdit(e.id); }}
                style={{ padding: "3px 8px", fontSize: 10, borderRadius: 3, border: "none", background: "var(--text-success)", color: "var(--bg-primary)", cursor: "pointer", fontWeight: 600 }}>Apply</button>
            </div>
          </div>
        ))}
        {tab === "edits" && edits.length === 0 && (
          <div style={{ textAlign: "center", color: "var(--text-muted)", fontSize: 12, padding: 40 }}>No pending AST edits</div>
        )}

        {tab === "preview" && selected && (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{selected.operation}: {selected.target}</div>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>{selected.description}</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
              <div>
                <div style={{ fontSize: 10, fontWeight: 600, color: "var(--text-danger)", marginBottom: 4 }}>Before</div>
                <pre style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", fontSize: 11, fontFamily: "monospace", color: "var(--text-primary)", whiteSpace: "pre-wrap", margin: 0 }}>{selected.diffBefore}</pre>
              </div>
              <div>
                <div style={{ fontSize: 10, fontWeight: 600, color: "var(--text-success)", marginBottom: 4 }}>After</div>
                <pre style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", fontSize: 11, fontFamily: "monospace", color: "var(--text-primary)", whiteSpace: "pre-wrap", margin: 0 }}>{selected.diffAfter}</pre>
              </div>
            </div>
          </div>
        )}
        {tab === "preview" && !selected && (
          <div style={{ textAlign: "center", color: "var(--text-muted)", fontSize: 12, padding: 40 }}>Select an edit from the Edits tab to preview</div>
        )}
      </div>
    </div>
  );
}
