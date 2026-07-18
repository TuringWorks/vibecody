/**
 * SecurityReviewPanel (gap B3) — §18.B3 cleared shape.
 *
 * On-demand, provider-agnostic security review of a single file. The user names
 * a file; the panel reads it and runs `security_review_file`, which returns
 * standard `Finding` records (the same schema clippy/eslint/semgrep produce).
 * Findings are surfaced here for review — acting on one is an explicit user step
 * (open the file, invoke diffcomplete), never an auto-applied fix. This is the
 * user-invoked entry point; the daemon's opt-in file-watcher loop calls the same
 * backend for the always-on path.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PROVIDER_DEFAULT_MODEL } from "../hooks/useModelRegistry";
import { getSelectedEffort } from "../utils/effort";

interface SecurityReviewPanelProps {
  workspacePath?: string | null;
  provider?: string;
  onOpenFile?: (path: string, line?: number) => void;
}

interface SecurityFinding {
  severity: string;
  message: string;
  file: string | null;
  line: number | null;
  suggestion: string | null;
}

const SEVERITY_COLOR: Record<string, string> = {
  critical: "#e5484d",
  error: "#e5734d",
  warning: "#e2a64d",
  info: "var(--text-secondary)",
};

export function SecurityReviewPanel({ workspacePath, provider, onOpenFile }: SecurityReviewPanelProps) {
  const [filePath, setFilePath] = useState("");
  const [findings, setFindings] = useState<SecurityFinding[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runReview = async () => {
    const path = filePath.trim();
    if (!path) return;
    if (!provider) {
      setError("Select a provider in the toolbar first.");
      return;
    }
    setLoading(true);
    setError(null);
    setFindings(null);
    try {
      const contents = await invoke<string>("read_file", { path });
      const result = await invoke<SecurityFinding[]>("security_review_file", {
        provider,
        model: PROVIDER_DEFAULT_MODEL[provider] ?? "",
        file: path,
        contents,
        effort: getSelectedEffort(),
      });
      setFindings(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
      <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
        Opt-in security review of a single file. Findings use the standard review schema;
        acting on one is an explicit step — open the file and apply via diffcomplete.
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <input
          type="text"
          value={filePath}
          onChange={(e) => setFilePath(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") runReview(); }}
          placeholder={workspacePath ? "path/to/file.rs (relative to workspace)" : "file path"}
          style={{ flex: 1, padding: 8, fontSize: "var(--font-size-md)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", boxSizing: "border-box" }}
        />
        <button
          onClick={runReview}
          disabled={loading || !filePath.trim()}
          style={{ padding: "6px 14px", fontSize: "var(--font-size-md)", borderRadius: "var(--radius-sm)", border: "none", background: "var(--accent-color)", color: "#fff", cursor: loading ? "default" : "pointer", opacity: loading || !filePath.trim() ? 0.6 : 1, whiteSpace: "nowrap" }}
        >
          {loading ? "Reviewing…" : "Review"}
        </button>
      </div>

      {error && <div style={{ fontSize: "var(--font-size-sm)", color: SEVERITY_COLOR.critical }}>{error}</div>}

      {findings && findings.length === 0 && (
        <div style={{ fontSize: "var(--font-size-md)", color: "var(--text-success, #4caf50)" }}>No security findings.</div>
      )}

      {findings && findings.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {findings.map((f, i) => (
            <div
              key={i}
              onClick={() => f.file && onOpenFile?.(f.file, f.line ?? undefined)}
              style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", borderLeft: `3px solid ${SEVERITY_COLOR[f.severity] ?? "var(--text-secondary)"}`, cursor: f.file ? "pointer" : "default" }}
            >
              <div style={{ display: "flex", gap: 8, alignItems: "baseline" }}>
                <span style={{ fontSize: "var(--font-size-xs)", textTransform: "uppercase", fontWeight: 700, color: SEVERITY_COLOR[f.severity] ?? "var(--text-secondary)" }}>{f.severity}</span>
                {f.line != null && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>line {f.line}</span>}
              </div>
              <div style={{ fontSize: "var(--font-size-md)", marginTop: 4 }}>{f.message}</div>
              {f.suggestion && (
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>↳ {f.suggestion}</div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default SecurityReviewPanel;
