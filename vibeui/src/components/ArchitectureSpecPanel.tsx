/**
 * ArchitectureSpecPanel — Enterprise Architecture dashboard.
 *
 * TOGAF ADM phases, Zachman Framework matrix, C4 Model diagrams,
 * Architecture Decision Records, and governance engine.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, boxSizing: "border-box" };
const tabRow: React.CSSProperties = { display: "flex", gap: 4, marginBottom: 12, flexWrap: "wrap" };
const cellStyle = (filled: boolean): React.CSSProperties => ({ padding: 6, fontSize: 10, textAlign: "center", border: "1px solid var(--border-color)", background: filled ? "var(--bg-secondary)" : "var(--bg-tertiary)", minWidth: 80, color: filled ? "var(--text-primary)" : "var(--text-secondary)" });

type Tab = "togaf" | "zachman" | "c4" | "adr" | "governance";

const togafPhases = ["Preliminary", "Architecture Vision", "Business Architecture", "Information Systems", "Technology Architecture", "Opportunities & Solutions", "Migration Planning", "Implementation Governance", "Change Management"];
const zachmanPerspectives = ["Planner", "Owner", "Designer", "Builder", "Implementer", "Worker"];
const zachmanAspects = ["What", "How", "Where", "Who", "When", "Why"];

export default function ArchitectureSpecPanel() {
  const [tab, setTab] = useState<Tab>("togaf");
  const [report, setReport] = useState("");
  const [adrTitle, setAdrTitle] = useState("");
  const [adrContext, setAdrContext] = useState("");
  const [adrDecision, setAdrDecision] = useState("");
  const [loading, setLoading] = useState(false);

  const loadReport = useCallback(async (type: string) => {
    setLoading(true);
    try {
      const res = await invoke<string>("archspec_report", { reportType: type });
      setReport(res);
    } catch (e) { setReport(`Use /archspec ${type} in terminal for full report.`); }
    setLoading(false);
  }, []);

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Architecture Specification</h2>
      <div style={tabRow}>
        {(["togaf", "zachman", "c4", "adr", "governance"] as Tab[]).map(t => (
          <button key={t} style={{ ...btnStyle, background: tab === t ? "var(--accent-color)" : "var(--bg-tertiary)", color: tab === t ? "#fff" : "var(--text-primary)" }} onClick={() => { setTab(t); if (t !== "adr") loadReport(t); }}>
            {t === "togaf" ? "TOGAF ADM" : t === "zachman" ? "Zachman" : t === "c4" ? "C4 Model" : t === "adr" ? "ADRs" : "Governance"}
          </button>
        ))}
      </div>

      {tab === "togaf" && (
        <>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>TOGAF ADM Phases</div>
            {togafPhases.map((p, i) => (
              <div key={i} style={{ display: "flex", justifyContent: "space-between", padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
                <span>{i + 1}. {p}</span>
                <span style={labelStyle}>0 artifacts</span>
              </div>
            ))}
          </div>
          {report && <div style={cardStyle}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
        </>
      )}

      {tab === "zachman" && (
        <div style={{ overflowX: "auto" }}>
          <table style={{ borderCollapse: "collapse", width: "100%", fontSize: 11 }}>
            <thead>
              <tr>
                <th style={{ ...cellStyle(true), fontWeight: 600 }}></th>
                {zachmanAspects.map(a => <th key={a} style={{ ...cellStyle(true), fontWeight: 600 }}>{a}</th>)}
              </tr>
            </thead>
            <tbody>
              {zachmanPerspectives.map(p => (
                <tr key={p}>
                  <td style={{ ...cellStyle(true), fontWeight: 600 }}>{p}</td>
                  {zachmanAspects.map(a => <td key={a} style={cellStyle(false)}>—</td>)}
                </tr>
              ))}
            </tbody>
          </table>
          <div style={{ ...labelStyle, marginTop: 8 }}>Use <code>/archspec zachman set perspective aspect content</code> to fill cells.</div>
        </div>
      )}

      {tab === "c4" && (
        <div style={cardStyle}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>C4 Model Levels</div>
          {["Context", "Container", "Component", "Code"].map((level, i) => (
            <div key={i} style={{ padding: "8px 0", borderBottom: "1px solid var(--border-color)" }}>
              <div style={{ fontWeight: 600 }}>L{i + 1}: {level}</div>
              <div style={labelStyle}>Use <code>/archspec c4 {level.toLowerCase()}</code> to generate diagram</div>
            </div>
          ))}
          {report && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: 11 }}>{report}</pre>}
        </div>
      )}

      {tab === "adr" && (
        <>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>New Architecture Decision Record</div>
            <div style={labelStyle}>Title</div>
            <input value={adrTitle} onChange={e => setAdrTitle(e.target.value)} style={{ ...inputStyle, marginBottom: 8 }} placeholder="Use PostgreSQL for primary database" />
            <div style={labelStyle}>Context</div>
            <textarea value={adrContext} onChange={e => setAdrContext(e.target.value)} rows={3} style={{ ...inputStyle, marginBottom: 8, resize: "vertical" }} placeholder="We need a reliable RDBMS that supports..." />
            <div style={labelStyle}>Decision</div>
            <textarea value={adrDecision} onChange={e => setAdrDecision(e.target.value)} rows={3} style={{ ...inputStyle, marginBottom: 8, resize: "vertical" }} placeholder="We will use PostgreSQL because..." />
            <button style={btnStyle} disabled={!adrTitle || loading}>Create ADR</button>
          </div>
        </>
      )}

      {tab === "governance" && (
        <div style={cardStyle}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Architecture Governance</div>
          <div style={labelStyle}>Governance rules validate architecture decisions against organizational standards.</div>
          <div style={{ marginTop: 8 }}>
            <button style={btnStyle} onClick={() => loadReport("governance")} disabled={loading}>Run Governance Check</button>
          </div>
          {report && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: 11 }}>{report}</pre>}
        </div>
      )}
    </div>
  );
}
