/**
 * CompanyDocumentsPanel — Markdown documents with revision history.
 *
 * Shows company documents linked to tasks/goals. Supports creating,
 * editing (full markdown), and viewing revision history.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyDocumentsPanelProps {
  workspacePath?: string | null;
}

export function CompanyDocumentsPanel({ workspacePath: _wp }: CompanyDocumentsPanelProps) {
  const [listOutput, setListOutput] = useState<string>("");
  const [docOutput, setDocOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [content, setContent] = useState("");
  const [docId, setDocId] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [mode, setMode] = useState<"list" | "create" | "view">("list");

  const loadList = async () => {
    setLoading(true);
    try {
      const out = await invoke<string>("company_cmd", { args: "doc list" });
      setListOutput(out);
    } catch (e) {
      setListOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadList(); }, []);

  const createDoc = async () => {
    if (!newTitle.trim()) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `doc create "${newTitle.trim()}"` });
      setCmdResult(out);
      setNewTitle("");
      setContent("");
      setMode("list");
      loadList();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const viewDoc = async () => {
    if (!docId.trim()) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `doc show ${docId.trim()}` });
      setDocOutput(out);
      setMode("view");
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const btnStyle: React.CSSProperties = {
    fontSize: 11, padding: "3px 10px", cursor: "pointer", borderRadius: 4,
    background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)",
  };
  return (
    <div style={{ padding: 16, fontSize: 13, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Documents</span>
        <div style={{ display: "flex", gap: 6 }}>
          <button onClick={() => setMode("list")} style={{ ...btnStyle, padding: "2px 8px", background: mode === "list" ? "var(--accent, #4a9eff)" : "var(--bg-tertiary)", color: mode === "list" ? "#fff" : "var(--text-primary)", border: `1px solid ${mode === "list" ? "var(--accent, #4a9eff)" : "var(--border-color)"}` }}>
            List
          </button>
          <button onClick={() => setMode("create")} style={{ ...btnStyle, padding: "2px 8px", background: mode === "create" ? "var(--accent, #4a9eff)" : "var(--bg-tertiary)", color: mode === "create" ? "#fff" : "var(--text-primary)", border: `1px solid ${mode === "create" ? "var(--accent, #4a9eff)" : "var(--border-color)"}` }}>
            + New
          </button>
          <button onClick={loadList} style={btnStyle}>
            Refresh
          </button>
        </div>
      </div>

      {cmdResult && (
        <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 4, padding: 8, marginBottom: 12, fontSize: 12 }}>
          {cmdResult}
        </div>
      )}

      {mode === "list" && (
        <>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <input value={docId} onChange={(e) => setDocId(e.target.value)} onKeyDown={(e) => e.key === "Enter" && viewDoc()} placeholder="Document ID to view"
              style={{ flex: 1, fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }} />
            <button onClick={viewDoc} style={{...btnStyle, padding: "4px 12px"}}>View</button>
          </div>
          <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12, minHeight: 200 }}>
            {loading ? (
              <span style={{ color: "var(--text-secondary)" }}>Loading…</span>
            ) : (
              <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap" }}>
                {listOutput || "No documents. Click + New to create one."}
              </pre>
            )}
          </div>
        </>
      )}

      {mode === "create" && (
        <div>
          <input value={newTitle} onChange={(e) => setNewTitle(e.target.value)} placeholder="Document title"
            style={{ width: "100%", fontSize: 13, padding: "6px 10px", marginBottom: 8, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", boxSizing: "border-box" }} />
          <textarea value={content} onChange={(e) => setContent(e.target.value)} placeholder="Document content (Markdown)"
            style={{ width: "100%", height: 300, fontSize: 12, padding: "8px", marginBottom: 8, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", resize: "vertical", boxSizing: "border-box" }} />
          <button onClick={createDoc} style={{...btnStyle, padding: "4px 16px"}}>
            Create Document
          </button>
        </div>
      )}

      {mode === "view" && (
        <div>
          <button onClick={() => setMode("list")} style={{...btnStyle, marginBottom: 12}}>← Back</button>
          <div style={{ background: "var(--panel-bg, rgba(0,0,0,0.2))", border: "1px solid var(--border-color)", borderRadius: 6, padding: 12 }}>
            <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.6 }}>
              {docOutput}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
