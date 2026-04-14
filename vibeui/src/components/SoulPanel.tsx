import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SoulSignals {
  name: string;
  description: string;
  license: string;
  languages: string[];
  frameworks: string[];
  has_tests: boolean;
  has_ci: boolean;
  has_docker: boolean;
  has_readme: boolean;
  is_monorepo: boolean;
  is_open_source: boolean;
  package_manager: string | null;
}

type Tab = "view" | "generate" | "signals";

export function SoulPanel({ workspacePath }: { workspacePath?: string | null }) {
  const [tab, setTab] = useState<Tab>("view");
  const [content, setContent] = useState<string | null>(null);
  const [signals, setSignals] = useState<SoulSignals | null>(null);
  const [customContext, setCustomContext] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [editContent, setEditContent] = useState<string>("");

  const wp = workspacePath || "";

  // Load existing SOUL.md on mount
  useEffect(() => {
    invoke<string | null>("soul_read", { workspacePath: wp })
      .then((result) => {
        setContent(result);
        if (result) setTab("view");
        else setTab("generate");
      })
      .catch(() => setTab("generate"));
  }, [wp]);

  const handleScan = async () => {
    setLoading(true);
    setError("");
    try {
      const result = await invoke<SoulSignals>("soul_scan", { workspacePath: wp });
      setSignals(result);
      setTab("signals");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const startEdit = () => {
    setEditContent(content ?? "");
    setIsEditing(true);
  };

  const cancelEdit = () => {
    setIsEditing(false);
    setEditContent("");
  };

  const handleSave = async () => {
    setLoading(true);
    setError("");
    setSuccess("");
    try {
      await invoke("soul_save", { workspacePath: wp, content: editContent });
      setContent(editContent);
      setSuccess("SOUL.md saved.");
      setIsEditing(false);
      setEditContent("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleGenerate = async (overwrite = false) => {
    setLoading(true);
    setError("");
    setSuccess("");
    try {
      const cmd = overwrite ? "soul_regenerate" : "soul_generate";
      const result = await invoke<string>(cmd, { workspacePath: wp, customContext });
      setContent(result);
      setSuccess(overwrite ? "SOUL.md regenerated." : "SOUL.md created.");
      setTab("view");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-tab-bar">
        <button className={`panel-tab${tab === "view" ? " active" : ""}`} onClick={() => setTab("view")}>View</button>
        <button className={`panel-tab${tab === "generate" ? " active" : ""}`} onClick={() => setTab("generate")}>Generate</button>
        <button className={`panel-tab${tab === "signals" ? " active" : ""}`} onClick={() => { setTab("signals"); if (!signals) handleScan(); }}>Signals</button>
      </div>

      {/* Status messages */}
      {error && (
        <div className="panel-error">{error}</div>
      )}
      {success && (
        <div style={{ padding: "8px 16px", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", color: "var(--text-success)", fontSize: "var(--font-size-base)" }}>
          {success}
        </div>
      )}

      {/* Tab content */}
      <div className="panel-body">
        {tab === "view" && (
          <>
            {content ? (
              isEditing ? (
                <>
                  <textarea
                    value={editContent}
                    onChange={(e) => setEditContent(e.target.value)}
                    className="panel-input panel-textarea panel-input-full"
                    style={{ minHeight: 360, fontFamily: "var(--font-mono)", fontSize: "var(--font-size-md)", lineHeight: "1.6", resize: "vertical" }}
                  />
                  <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end", marginTop: 8 }}>
                    <button className="panel-btn panel-btn-secondary" onClick={cancelEdit}>Cancel</button>
                    <button className="panel-btn panel-btn-primary" onClick={handleSave} disabled={loading}>
                      {loading ? "Saving…" : "Save"}
                    </button>
                  </div>
                </>
              ) : (
                <>
                  <div className="panel-card">
                    <pre style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-md)", lineHeight: "1.6", whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-primary)", margin: 0 }}>
                      {content}
                    </pre>
                  </div>
                  <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
                    <button className="panel-btn panel-btn-secondary" onClick={() => handleGenerate(true)} disabled={loading}>
                      Regenerate
                    </button>
                    <button className="panel-btn panel-btn-primary" onClick={startEdit}>
                      Edit
                    </button>
                  </div>
                </>
              )
            ) : (
              <div className="panel-empty">
                <div style={{ fontSize: "var(--font-size-lg)", marginBottom: "12px" }}>No SOUL.md found in this project.</div>
                <button className="panel-btn panel-btn-primary" onClick={() => setTab("generate")}>
                  Generate One
                </button>
              </div>
            )}
          </>
        )}

        {tab === "generate" && (
          <>
            <div className="panel-card">
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: "8px", color: "var(--text-primary)" }}>
                Generate SOUL.md
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: "16px" }}>
                VibeCody will scan your project structure, detect languages, frameworks, license, and
                testing patterns, then generate a SOUL.md that captures your project's philosophy and design principles.
              </div>

              <label className="panel-label">
                Custom context (optional)
              </label>
              <textarea
                value={customContext}
                onChange={(e) => setCustomContext(e.target.value)}
                placeholder="Describe your project's purpose, motivation, or key values in a few sentences..."
                rows={4}
                className="panel-input panel-textarea panel-input-full"
                style={{ resize: "vertical" }}
              />

              <div style={{ display: "flex", gap: "8px", marginTop: "12px" }}>
                <button
                  className="panel-btn panel-btn-primary"
                  onClick={() => handleGenerate(content !== null)}
                  disabled={loading}
                >
                  {loading ? "Generating..." : content ? "Regenerate SOUL.md" : "Generate SOUL.md"}
                </button>
                <button className="panel-btn panel-btn-secondary" onClick={handleScan} disabled={loading}>
                  Scan Project First
                </button>
              </div>
            </div>

            <div className="panel-card">
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: "8px", color: "var(--text-primary)" }}>
                What gets generated
              </div>
              <ul style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", margin: 0, paddingLeft: "20px", lineHeight: "1.8" }}>
                <li><strong>Why This Project Exists</strong> — The problem and motivation</li>
                <li><strong>Core Beliefs</strong> — 3-6 principles that guide decisions</li>
                <li><strong>Design Principles</strong> — Technical philosophy and patterns</li>
                <li><strong>What This Project Is Not</strong> — Explicit boundaries</li>
                <li><strong>How to Know If a Change Belongs</strong> — Decision framework for contributors</li>
              </ul>
            </div>
          </>
        )}

        {tab === "signals" && (
          <>
            {signals ? (
              <div className="panel-card">
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: "12px", color: "var(--text-primary)" }}>
                  Project Signals: {signals.name}
                </div>
                <table style={{ width: "100%", fontSize: "var(--font-size-base)", borderCollapse: "collapse" }}>
                  <tbody>
                    {[
                      ["Name", signals.name],
                      ["Description", signals.description || "(none detected)"],
                      ["License", signals.license || "(none)"],
                      ["Languages", signals.languages.join(", ") || "(none detected)"],
                      ["Frameworks", signals.frameworks.join(", ") || "(none detected)"],
                      ["Package Manager", signals.package_manager || "(none)"],
                      ["Monorepo", signals.is_monorepo ? "Yes" : "No"],
                      ["Open Source", signals.is_open_source ? "Yes" : "No"],
                      ["Has Tests", signals.has_tests ? "Yes" : "No"],
                      ["Has CI", signals.has_ci ? "Yes" : "No"],
                      ["Has Docker", signals.has_docker ? "Yes" : "No"],
                      ["Has README", signals.has_readme ? "Yes" : "No"],
                    ].map(([label, value]) => (
                      <tr key={label} style={{ borderBottom: "1px solid var(--border-color)" }}>
                        <td style={{ padding: "6px 12px 6px 0", fontWeight: 600, color: "var(--text-primary)", whiteSpace: "nowrap" }}>{label}</td>
                        <td style={{ padding: "6px 0", color: "var(--text-secondary)" }}>{value}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <div style={{ marginTop: "12px" }}>
                  <button className="panel-btn panel-btn-secondary" onClick={handleScan} disabled={loading}>
                    {loading ? "Scanning..." : "Rescan"}
                  </button>
                </div>
              </div>
            ) : (
              <div className="panel-empty">
                {loading ? "Scanning project..." : "Click to scan project signals."}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default SoulPanel;
