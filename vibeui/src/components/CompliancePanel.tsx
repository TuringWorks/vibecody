import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";

interface ComplianceControl {
 id: string;
 name: string;
 status: string;
 evidence: string[];
 notes: string;
}

interface ComplianceReport {
 framework: string;
 controls: ComplianceControl[];
 summary: {
 total: number;
 implemented: number;
 partial: number;
 gaps: number;
 percentage: number;
 };
}

const FRAMEWORKS = ["SOC2", "FedRAMP", "HIPAA", "GDPR", "ISO27001"] as const;

const statusBadge = (s: string) => {
 if (s === "implemented") return { label: "Implemented", color: "var(--success-color)" };
 if (s === "partial") return { label: "Partial", color: "var(--warning-color)" };
 if (s === "not_implemented") return { label: "Gap", color: "var(--error-color)" };
 return { label: "N/A", color: "var(--text-secondary)" };
};

export function CompliancePanel() {
 const [framework, setFramework] = useState<string>("SOC2");
 const [report, setReport] = useState<ComplianceReport | null>(null);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [expanded, setExpanded] = useState<string | null>(null);

 const generate = async () => {
 setLoading(true);
 setError(null);
 try {
 const result = await invoke<ComplianceReport>("generate_compliance_report", { framework });
 setReport(result);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const exportMarkdown = () => {
 if (!report) return;
 let md = `# ${report.framework} Compliance Report\n\n`;
 md += `**Compliance: ${report.summary.percentage.toFixed(1)}%** (${report.summary.implemented} implemented, ${report.summary.partial} partial, ${report.summary.gaps} gaps)\n\n`;
 md += "| ID | Control | Status | Evidence |\n|---|---|---|---|\n";
 for (const c of report.controls) {
 const { label } = statusBadge(c.status);
 md += `| ${c.id} | ${c.name} | ${label} | ${c.evidence.join(", ")} |\n`;
 }
 navigator.clipboard.writeText(md);
 };

 return (
 <div style={{ padding: 16, color: "var(--text-primary)", fontSize: 13 }}>
 <h3 style={{ margin: "0 0 12px 0", fontSize: 15 }}>Compliance Report</h3>

 <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
 <select
 value={framework}
 onChange={(e) => setFramework(e.target.value)}
 style={{ padding: "5px 10px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 12 }}
 >
 {FRAMEWORKS.map((f) => (
 <option key={f} value={f}>{f}</option>
 ))}
 </select>
 <button
 onClick={generate}
 disabled={loading}
 style={{ padding: "5px 14px", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: 4, fontSize: 12, cursor: "pointer" }}
 >
 {loading ? "Generating..." : "Generate Report"}
 </button>
 {report && (
 <button
 onClick={exportMarkdown}
 style={{ padding: "5px 14px", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 12, cursor: "pointer" }}
 >
 Copy Markdown
 </button>
 )}
 </div>

 {error && <div style={{ color: "var(--error-color)", marginBottom: 12 }}>{error}</div>}

 {!report && !loading && !error && (
  <div style={{ textAlign: "center", padding: "40px 16px", color: "var(--text-secondary)", lineHeight: 1.7 }}>
   <Icon name="shield" size={40} style={{ opacity: 0.3, marginBottom: 8 }} />
   <div style={{ fontSize: 13 }}>No compliance report yet</div>
   <div style={{ fontSize: 11, marginTop: 4 }}>
    Select a framework above and click <strong>Generate Report</strong> to audit your codebase.
   </div>
  </div>
 )}

 {loading && (
  <div style={{ textAlign: "center", padding: "40px 16px", color: "var(--text-secondary)", fontSize: 13 }}>
   Generating {framework} compliance report…<br />
   <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>This may take 15–30 seconds</span>
  </div>
 )}

 {report && (
 <>
 {/* Summary bar */}
 <div style={{ marginBottom: 16 }}>
 <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4, fontSize: 12 }}>
 <span>Compliance Score</span>
 <span style={{ fontWeight: 600 }}>{report.summary.percentage.toFixed(1)}%</span>
 </div>
 <div style={{ height: 8, background: "var(--bg-secondary)", borderRadius: 4, overflow: "hidden" }}>
 <div
 style={{
 height: "100%",
 width: `${Math.min(report.summary.percentage, 100)}%`,
 background: report.summary.percentage >= 80 ? "var(--success-color)" : report.summary.percentage >= 50 ? "var(--warning-color)" : "var(--error-color)",
 borderRadius: 4,
 transition: "width 0.3s",
 }}
 />
 </div>
 <div style={{ display: "flex", gap: 16, marginTop: 8, fontSize: 11, color: "var(--text-secondary)" }}>
 <span> {report.summary.implemented} implemented</span>
 <span style={{ color: "var(--warning-color)" }}>{report.summary.partial} partial</span>
 <span> {report.summary.gaps} gaps</span>
 </div>
 </div>

 {/* Controls table */}
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
 <thead>
 <tr>
 <th style={{ textAlign: "left", padding: "6px 8px", borderBottom: "2px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-secondary)", fontSize: 11 }}>ID</th>
 <th style={{ textAlign: "left", padding: "6px 8px", borderBottom: "2px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-secondary)", fontSize: 11 }}>Control</th>
 <th style={{ textAlign: "left", padding: "6px 8px", borderBottom: "2px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-secondary)", fontSize: 11 }}>Status</th>
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
 <td style={{ padding: "5px 8px", borderBottom: "1px solid var(--border-color)", fontFamily: "var(--font-mono)" }}>{ctrl.id}</td>
 <td style={{ padding: "5px 8px", borderBottom: "1px solid var(--border-color)" }}>
 {ctrl.name}
 {expanded === ctrl.id && (
 <div style={{ marginTop: 6, fontSize: 11, color: "var(--text-secondary)" }}>
 <div><strong>Evidence:</strong> {ctrl.evidence.join(", ") || "None"}</div>
 <div style={{ marginTop: 2 }}><strong>Notes:</strong> {ctrl.notes}</div>
 </div>
 )}
 </td>
 <td style={{ padding: "5px 8px", borderBottom: "1px solid var(--border-color)", color: badge.color }}>{badge.label}</td>
 </tr>
 );
 })}
 </tbody>
 </table>
 </>
 )}
 </div>
 );
}
