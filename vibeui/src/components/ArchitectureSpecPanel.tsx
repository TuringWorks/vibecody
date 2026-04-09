/**
 * ArchitectureSpecPanel — Enterprise Architecture dashboard.
 *
 * TOGAF ADM phases, Zachman Framework matrix, C4 Model diagrams,
 * Architecture Decision Records, and governance engine.
 * Each tab has a "Run in CLI" button that invokes vibecli /archspec.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  const [cliError, setCliError] = useState("");

  const runCli = useCallback(async (args: string) => {
    setLoading(true);
    setCliError("");
    setReport("");
    try {
      const res = await invoke<string>("handle_archspec_command", { args });
      setReport(res);
    } catch (e) {
      setCliError(String(e));
    }
    setLoading(false);
  }, []);

  const CliButton = ({ args, label }: { args: string; label?: string }) => (
    <button
      className="panel-btn panel-btn-secondary panel-btn-sm"
      onClick={() => runCli(args)}
      disabled={loading}
      title={`Run: vibecli --cmd "/archspec ${args}"`}
    >
      {loading ? "Running…" : (label ?? `▶ /archspec ${args}`)}
    </button>
  );

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>Architecture Specification</h2>
        <CliButton args="report" label="▶ Full Report" />
      </div>

      <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
        {(["togaf", "zachman", "c4", "adr", "governance"] as Tab[]).map(t => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => { setTab(t); setReport(""); setCliError(""); }}>
            {t === "togaf" ? "TOGAF ADM" : t === "zachman" ? "Zachman" : t === "c4" ? "C4 Model" : t === "adr" ? "ADRs" : "Governance"}
          </button>
        ))}
      </div>

      {tab === "togaf" && (
        <>
          <div className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <span style={{ fontWeight: 600 }}>TOGAF ADM Phases</span>
              <div style={{ display: "flex", gap: 6 }}>
                <CliButton args="togaf" label="▶ Overview" />
                <CliButton args="togaf report" label="▶ Full Report" />
              </div>
            </div>
            {togafPhases.map((p, i) => (
              <div key={i} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
                <span>{i + 1}. {p}</span>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <span className="panel-label">0 artifacts</span>
                  <CliButton args={`togaf ${p.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`} label="▶" />
                </div>
              </div>
            ))}
          </div>
          {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
          {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
        </>
      )}

      {tab === "zachman" && (
        <>
          <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 8 }}>
            <CliButton args="zachman" />
          </div>
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
          </div>
          {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
          {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
        </>
      )}

      {tab === "c4" && (
        <div className="panel-card">
          <div style={{ fontWeight: 600, marginBottom: 8 }}>C4 Model Levels</div>
          {(["context", "container", "component"] as const).map((level) => (
            <div key={level} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 0", borderBottom: "1px solid var(--border-color)" }}>
              <div>
                <div style={{ fontWeight: 600, textTransform: "capitalize" }}>{level}</div>
                <div className="panel-label">Generate {level} diagram</div>
              </div>
              <CliButton args={`c4 ${level}`} label={`▶ ${level}`} />
            </div>
          ))}
          {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
          {report && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: 11 }}>{report}</pre>}
        </div>
      )}

      {tab === "adr" && (
        <>
          <div className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <span style={{ fontWeight: 600 }}>Architecture Decision Records</span>
              <CliButton args="adr list" label="▶ List ADRs" />
            </div>
            <div className="panel-label">Title</div>
            <input value={adrTitle} onChange={e => setAdrTitle(e.target.value)} className="panel-input panel-input-full" style={{ marginBottom: 8 }} placeholder="Use PostgreSQL for primary database" />
            <div className="panel-label">Context</div>
            <textarea value={adrContext} onChange={e => setAdrContext(e.target.value)} rows={3} className="panel-input panel-input-full" style={{ marginBottom: 8, resize: "vertical" }} placeholder="We need a reliable RDBMS that supports..." />
            <div className="panel-label">Decision</div>
            <textarea value={adrDecision} onChange={e => setAdrDecision(e.target.value)} rows={3} className="panel-input panel-input-full" style={{ marginBottom: 8, resize: "vertical" }} placeholder="We will use PostgreSQL because..." />
            <div style={{ display: "flex", gap: 8 }}>
              <button className="panel-btn panel-btn-secondary" disabled={!adrTitle || loading}>Create ADR</button>
              <CliButton args="adr list" label="▶ Refresh List" />
            </div>
          </div>
          {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
          {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
        </>
      )}

      {tab === "governance" && (
        <div className="panel-card">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontWeight: 600 }}>Architecture Governance</span>
            <CliButton args="report" label="▶ Full Report" />
          </div>
          <div className="panel-label">Governance rules validate architecture decisions against organizational standards.</div>
          <div style={{ marginTop: 8 }}>
            <CliButton args="report" label="▶ Run Governance Check" />
          </div>
          {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
          {report && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: 11 }}>{report}</pre>}
        </div>
      )}
    </div>
  );
}
