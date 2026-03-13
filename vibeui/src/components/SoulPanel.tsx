import React, { useState, useEffect } from "react";
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

  const tabStyle = (t: Tab): React.CSSProperties => ({
    padding: "6px 16px",
    fontSize: "12px",
    border: "none",
    cursor: "pointer",
    background: tab === t ? "var(--bg-primary)" : "transparent",
    color: tab === t ? "var(--text-primary)" : "var(--text-muted, var(--text-secondary))",
    borderBottom: tab === t ? "2px solid var(--accent-color, #60a5fa)" : "2px solid transparent",
  });

  const cardStyle: React.CSSProperties = {
    background: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
    borderRadius: "8px",
    padding: "16px",
    marginBottom: "12px",
  };

  const btnStyle = (variant: "primary" | "secondary" = "primary"): React.CSSProperties => ({
    padding: "8px 20px",
    fontSize: "13px",
    border: "none",
    borderRadius: "6px",
    cursor: loading ? "not-allowed" : "pointer",
    opacity: loading ? 0.6 : 1,
    background: variant === "primary" ? "var(--accent-color, #60a5fa)" : "var(--bg-secondary)",
    color: variant === "primary" ? "#fff" : "var(--text-primary)",
  });

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", overflow: "hidden" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
        <button style={tabStyle("view")} onClick={() => setTab("view")}>View</button>
        <button style={tabStyle("generate")} onClick={() => setTab("generate")}>Generate</button>
        <button style={tabStyle("signals")} onClick={() => { setTab("signals"); if (!signals) handleScan(); }}>Signals</button>
      </div>

      {/* Status messages */}
      {error && (
        <div style={{ padding: "8px 16px", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", color: "var(--text-danger, #ef4444)", fontSize: "12px" }}>
          {error}
        </div>
      )}
      {success && (
        <div style={{ padding: "8px 16px", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", color: "var(--text-success, #22c55e)", fontSize: "12px" }}>
          {success}
        </div>
      )}

      {/* Tab content */}
      <div style={{ flex: 1, overflow: "auto", padding: "16px" }}>
        {tab === "view" && (
          <>
            {content ? (
              <div style={cardStyle}>
                <pre style={{ fontFamily: "monospace", fontSize: "13px", lineHeight: "1.6", whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-primary)", margin: 0 }}>
                  {content}
                </pre>
              </div>
            ) : (
              <div style={{ textAlign: "center", padding: "40px 20px", color: "var(--text-muted, var(--text-secondary))" }}>
                <div style={{ fontSize: "14px", marginBottom: "12px" }}>No SOUL.md found in this project.</div>
                <button style={btnStyle("primary")} onClick={() => setTab("generate")}>
                  Generate One
                </button>
              </div>
            )}
            {content && (
              <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
                <button style={btnStyle("secondary")} onClick={() => handleGenerate(true)} disabled={loading}>
                  Regenerate
                </button>
              </div>
            )}
          </>
        )}

        {tab === "generate" && (
          <>
            <div style={cardStyle}>
              <div style={{ fontWeight: 600, fontSize: "14px", marginBottom: "8px", color: "var(--text-primary)" }}>
                Generate SOUL.md
              </div>
              <div style={{ fontSize: "12px", color: "var(--text-muted, var(--text-secondary))", marginBottom: "16px" }}>
                VibeCody will scan your project structure, detect languages, frameworks, license, and
                testing patterns, then generate a SOUL.md that captures your project's philosophy and design principles.
              </div>

              <label style={{ display: "block", fontSize: "12px", fontWeight: 600, marginBottom: "4px", color: "var(--text-primary)" }}>
                Custom context (optional)
              </label>
              <textarea
                value={customContext}
                onChange={(e) => setCustomContext(e.target.value)}
                placeholder="Describe your project's purpose, motivation, or key values in a few sentences..."
                rows={4}
                style={{
                  width: "100%",
                  padding: "8px 12px",
                  fontSize: "13px",
                  fontFamily: "inherit",
                  background: "var(--bg-primary)",
                  color: "var(--text-primary)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "6px",
                  resize: "vertical",
                  boxSizing: "border-box",
                }}
              />

              <div style={{ display: "flex", gap: "8px", marginTop: "12px" }}>
                <button
                  style={btnStyle("primary")}
                  onClick={() => handleGenerate(content !== null)}
                  disabled={loading}
                >
                  {loading ? "Generating..." : content ? "Regenerate SOUL.md" : "Generate SOUL.md"}
                </button>
                <button style={btnStyle("secondary")} onClick={handleScan} disabled={loading}>
                  Scan Project First
                </button>
              </div>
            </div>

            <div style={cardStyle}>
              <div style={{ fontWeight: 600, fontSize: "13px", marginBottom: "8px", color: "var(--text-primary)" }}>
                What gets generated
              </div>
              <ul style={{ fontSize: "12px", color: "var(--text-muted, var(--text-secondary))", margin: 0, paddingLeft: "20px", lineHeight: "1.8" }}>
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
              <div style={cardStyle}>
                <div style={{ fontWeight: 600, fontSize: "14px", marginBottom: "12px", color: "var(--text-primary)" }}>
                  Project Signals: {signals.name}
                </div>
                <table style={{ width: "100%", fontSize: "12px", borderCollapse: "collapse" }}>
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
                        <td style={{ padding: "6px 0", color: "var(--text-muted, var(--text-secondary))" }}>{value}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <div style={{ marginTop: "12px" }}>
                  <button style={btnStyle("secondary")} onClick={handleScan} disabled={loading}>
                    {loading ? "Scanning..." : "Rescan"}
                  </button>
                </div>
              </div>
            ) : (
              <div style={{ textAlign: "center", padding: "40px", color: "var(--text-muted, var(--text-secondary))" }}>
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
