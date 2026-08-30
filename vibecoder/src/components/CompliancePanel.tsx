import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";

interface ComplianceControl {
 id: string;
 name: string;
 description: string;
 status: ControlStatus;
 evidence: string[];
 notes: string;
}

interface ScanScope {
 root: string;
 files_seen: number;
 files_read: number;
 truncated: boolean;
 /** Files skipped for exceeding the per-file size limit. */
 files_too_large: number;
 /** null when the workspace is not a git checkout. */
 git_tracked_files: number | null;
}

interface ComplianceReport {
 framework: string;
 generated_at: number;
 scope: ScanScope;
 controls: ComplianceControl[];
 summary: {
  total: number;
  implemented: number;
  partial: number;
  gaps: number;
  not_applicable: number;
  not_assessed: number;
  /** Controls the scan could decide — the denominator of `percentage`. */
  scored: number;
  /** null when nothing could be scored: a rate over zero controls is n/a. */
  percentage: number | null;
 };
}

const FRAMEWORKS = ["SOC2", "FedRAMP", "HIPAA", "GDPR", "ISO27001"] as const;

type ControlStatus =
 | "implemented"
 | "partial"
 | "not_implemented"
 | "not_applicable"
 | "not_assessed";

interface Badge { label: string; color: string }

const statusBadge = (s: ControlStatus): Badge => {
 switch (s) {
  case "implemented": return { label: "Implemented", color: "var(--success-color)" };
  case "partial": return { label: "Partial", color: "var(--warning-color)" };
  case "not_implemented": return { label: "Gap", color: "var(--error-color)" };
  case "not_applicable": return { label: "N/A", color: "var(--text-secondary)" };
  case "not_assessed": return { label: "Not assessed", color: "var(--text-secondary)" };
  default: {
   // An unknown status is not silently rendered as a pass.
   const exhaustive: never = s;
   return { label: String(exhaustive), color: "var(--text-secondary)" };
  }
 }
};

/** A score over zero scored controls is "n/a", never 0%. */
const formatScore = (pct: number | null) => (pct === null ? "n/a" : `${pct.toFixed(1)}%`);

interface CompliancePanelProps {
 /** The folder to scan. Without one there is nothing to report on. */
 workspacePath?: string | null;
}

export function CompliancePanel({ workspacePath }: CompliancePanelProps) {
 const [framework, setFramework] = useState<string>("SOC2");
 const [report, setReport] = useState<ComplianceReport | null>(null);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [expanded, setExpanded] = useState<string | null>(null);

 const generate = async () => {
  setLoading(true);
  setError(null);
  try {
   const result = await invoke<ComplianceReport>("generate_compliance_report", {
    framework,
    workspacePath,
   });
   setReport(result);
  } catch (e) {
   setError(String(e));
  } finally {
   setLoading(false);
  }
 };

 const exportMarkdown = () => {
  if (!report) return;
  const s = report.summary;
  const lines = [
   `# ${report.framework} Compliance Report`,
   "",
   `**Project:** \`${report.scope.root}\``,
   "",
   `**Compliance: ${formatScore(s.percentage)}** over ${s.scored} scored controls ` +
    `(${s.implemented} implemented, ${s.partial} partial, ${s.gaps} gaps). ` +
    `${s.not_assessed} control(s) could not be assessed from source.`,
   "",
   `Scanned ${report.scope.files_seen} files, read ${report.scope.files_read}.` +
    (report.scope.truncated ? " Scan budget reached — evidence is a lower bound." : "") +
    (report.scope.files_too_large > 0
     ? ` ${report.scope.files_too_large} file(s) were larger than the per-file limit and were not read.`
     : "") +
    (report.scope.git_tracked_files === null
     ? " Not a git checkout, so committed-credential checks did not run."
     : ""),
   "",
   "| ID | Control | Status | Evidence | Notes |",
   "|---|---|---|---|---|",
   ...report.controls.map((c) =>
    `| ${c.id} | ${c.name} | ${statusBadge(c.status).label} | ${c.evidence.join("<br>") || "—"} | ${c.notes} |`
   ),
  ];
  navigator.clipboard.writeText(lines.join("\n"));
 };

 const summary = report?.summary;
 const barPct = summary?.percentage ?? 0;

 return (
  <div className="panel-container">
   <div className="panel-header"><h3>Compliance Report</h3></div>
   <div className="panel-body">

    <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
     <select
      value={framework}
      onChange={(e) => setFramework(e.target.value)}
      style={{ padding: "4px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)" }}
     >
      {FRAMEWORKS.map((f) => (
       <option key={f} value={f}>{f}</option>
      ))}
     </select>
     <button className="panel-btn"
      onClick={generate}
      disabled={loading || !workspacePath}
      title={workspacePath ? undefined : "Open a folder to scan"}
      style={{ padding: "4px 16px", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)", cursor: loading || !workspacePath ? "not-allowed" : "pointer", opacity: workspacePath ? 1 : 0.5 }}
     >
      {loading ? "Scanning..." : "Scan Project"}
     </button>
     {report && (
      <button className="panel-btn"
       onClick={exportMarkdown}
       style={{ padding: "4px 16px", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)", cursor: "pointer" }}
      >
       Copy Markdown
      </button>
     )}
    </div>

    {error && <div style={{ color: "var(--error-color)", marginBottom: 12 }}>{error}</div>}

    {!workspacePath && !error && (
     <div style={{ textAlign: "center", padding: "40px 16px", color: "var(--text-secondary)", lineHeight: 1.7 }}>
      <Icon name="shield" size={40} style={{ opacity: 0.3, marginBottom: 8 }} />
      <div style={{ fontSize: "var(--font-size-md)" }}>No folder open</div>
      <div style={{ fontSize: "var(--font-size-sm)", marginTop: 4 }}>
       A compliance report is a scan of a project — open a folder to run one.
      </div>
     </div>
    )}

    {workspacePath && !report && !loading && !error && (
     <div style={{ textAlign: "center", padding: "40px 16px", color: "var(--text-secondary)", lineHeight: 1.7 }}>
      <Icon name="shield" size={40} style={{ opacity: 0.3, marginBottom: 8 }} />
      <div style={{ fontSize: "var(--font-size-md)" }}>No compliance report yet</div>
      <div style={{ fontSize: "var(--font-size-sm)", marginTop: 4 }}>
       Select a framework and click <strong>Scan Project</strong> to audit
       <code style={{ marginLeft: 4, fontFamily: "var(--font-mono)" }}>{workspacePath}</code>.
      </div>
     </div>
    )}

    {loading && (
     <div style={{ textAlign: "center", padding: "40px 16px", color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
      Scanning for {framework} evidence…<br />
      <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Reading the project tree — larger repositories take longer</span>
     </div>
    )}

    {report && summary && (
     <>
      {/* Summary bar */}
      <div style={{ marginBottom: 16 }}>
       <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4, fontSize: "var(--font-size-base)" }}>
        <span>Compliance Score</span>
        <span style={{ fontWeight: 600 }}>{formatScore(summary.percentage)}</span>
       </div>
       <div style={{ height: 8, background: "var(--bg-secondary)", borderRadius: "var(--radius-xs-plus)", overflow: "hidden" }}>
        <div
         style={{
          height: "100%",
          width: `${Math.min(barPct, 100)}%`,
          background: barPct >= 80 ? "var(--success-color)" : barPct >= 50 ? "var(--warning-color)" : "var(--error-color)",
          borderRadius: "var(--radius-xs-plus)",
          transition: "width 0.3s",
         }}
        />
       </div>
       <div style={{ display: "flex", gap: 16, marginTop: 8, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", flexWrap: "wrap" }}>
        <span style={{ color: "var(--success-color)" }}>{summary.implemented} implemented</span>
        <span style={{ color: "var(--warning-color)" }}>{summary.partial} partial</span>
        <span style={{ color: "var(--error-color)" }}>{summary.gaps} gaps</span>
        <span>{summary.not_assessed} not assessed</span>
        <span>over {summary.scored} scored controls</span>
       </div>
       {/* What the scan actually covered. Without it, a truncated scan and a
           clean project look identical. */}
       <div style={{ marginTop: 8, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.6 }}>
        <div style={{ fontFamily: "var(--font-mono)" }}>{report.scope.root}</div>
        <div>
         {report.scope.files_seen} files scanned, {report.scope.files_read} read.
         {report.scope.truncated && " Scan budget reached — evidence below is a lower bound."}
         {report.scope.files_too_large > 0 &&
          ` ${report.scope.files_too_large} file(s) exceeded the per-file size limit and were not read.`}
         {report.scope.git_tracked_files === null && " Not a git checkout, so committed-credential checks did not run."}
        </div>
        {summary.not_assessed > 0 && (
         <div>
          {summary.not_assessed} control(s) need evidence that does not live in a repository and are
          excluded from the score.
         </div>
        )}
       </div>
      </div>

      {/* Controls table */}
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
       <thead>
        <tr>
         <th style={{ textAlign: "left", padding: "8px 8px", borderBottom: "2px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>ID</th>
         <th style={{ textAlign: "left", padding: "8px 8px", borderBottom: "2px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>Control</th>
         <th style={{ textAlign: "left", padding: "8px 8px", borderBottom: "2px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>Status</th>
        </tr>
       </thead>
       <tbody>
        {report.controls.map((ctrl) => {
         const badge = statusBadge(ctrl.status);
         return (
          <tr
           key={ctrl.id}
           style={{ cursor: "pointer", background: expanded === ctrl.id ? "rgba(124,58,237,0.1)" : undefined }}
           onClick={() => setExpanded(expanded === ctrl.id ? null : ctrl.id)}
          >
           <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border-color)", fontFamily: "var(--font-mono)", verticalAlign: "top", whiteSpace: "nowrap" }}>{ctrl.id}</td>
           <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border-color)" }}>
            {ctrl.name}
            {expanded === ctrl.id && (
             <div style={{ marginTop: 6, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.6 }}>
              <div>{ctrl.description}</div>
              <div style={{ marginTop: 4 }}>
               <strong>Evidence:</strong>{" "}
               {ctrl.evidence.length === 0 ? (
                "None found"
               ) : (
                <ul style={{ margin: "2px 0 0 0", paddingLeft: 18 }}>
                 {ctrl.evidence.map((e) => (
                  <li key={e} style={{ fontFamily: "var(--font-mono)" }}>{e}</li>
                 ))}
                </ul>
               )}
              </div>
              <div style={{ marginTop: 4 }}><strong>Notes:</strong> {ctrl.notes}</div>
             </div>
            )}
           </td>
           <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border-color)", color: badge.color, verticalAlign: "top", whiteSpace: "nowrap" }}>{badge.label}</td>
          </tr>
         );
        })}
       </tbody>
      </table>
     </>
    )}
   </div>
  </div>
 );
}
