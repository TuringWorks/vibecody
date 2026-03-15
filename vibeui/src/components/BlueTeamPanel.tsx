import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type BlueTeamTab = "Incidents" | "IOCs" | "Detection Rules" | "Forensics" | "SIEM" | "Playbooks" | "Threat Hunt";

interface Incident {
  id: string;
  title: string;
  severity: "P1" | "P2" | "P3" | "P4";
  status: "Open" | "Investigating" | "Contained" | "Resolved" | "Closed";
  category: string;
  assignee: string;
  created: string;
  description: string;
}

interface IOC {
  id: string;
  ioc_type: "IP" | "Domain" | "Hash" | "URL" | "Email" | "File";
  value: string;
  confidence: number;
  source: string;
  first_seen: string;
  tags: string[];
}

interface DetectionRule {
  id: string;
  name: string;
  platform: "Sigma" | "YARA" | "Snort" | "KQL" | "SPL" | "EQL";
  mitre_ids: string[];
  enabled: boolean;
  description: string;
  query: string;
}

interface ForensicsCase {
  id: string;
  incident_id: string;
  incident_title: string;
  artifact_count: number;
  finding_count: number;
  status: "Active" | "Completed" | "Archived";
  created: string;
}

interface SIEMConnection {
  id: string;
  platform: string;
  endpoint: string;
  status: "connected" | "disconnected" | "error";
  last_sync: string;
  event_count: number;
}

interface Playbook {
  id: string;
  name: string;
  category: string;
  steps: PlaybookStep[];
}

interface PlaybookStep {
  order: number;
  name: string;
  description: string;
  automated: boolean;
}

interface ThreatHunt {
  id: string;
  hypothesis: string;
  data_sources: string[];
  query: string;
  status: "Draft" | "Running" | "Completed";
  findings: string[];
}

const TABS: BlueTeamTab[] = ["Incidents", "IOCs", "Detection Rules", "Forensics", "SIEM", "Playbooks", "Threat Hunt"];

const SEVERITY_COLORS: Record<string, string> = {
  P1: "#f38ba8",
  P2: "#fab387",
  P3: "#f9e2af",
  P4: "#89b4fa",
};

const STATUS_COLORS: Record<string, string> = {
  Open: "#f38ba8",
  Investigating: "#fab387",
  Contained: "#f9e2af",
  Resolved: "#a6e3a1",
  Closed: "#6c7086",
  Active: "#a6e3a1",
  Completed: "#89b4fa",
  Archived: "#6c7086",
  Draft: "#6c7086",
  Running: "#fab387",
};

const containerStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  height: "100%",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontFamily: "var(--font-mono)",
  overflow: "hidden",
};

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: 2,
  padding: "8px 12px 0",
  borderBottom: "1px solid var(--border-primary)",
  background: "var(--bg-secondary)",
  overflowX: "auto",
  flexShrink: 0,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px",
  cursor: "pointer",
  background: active ? "var(--bg-primary)" : "transparent",
  color: active ? "var(--accent-primary)" : "var(--text-secondary)",
  border: "none",
  borderBottom: active ? "2px solid var(--accent-primary)" : "2px solid transparent",
  fontSize: 13,
  fontFamily: "var(--font-mono)",
  whiteSpace: "nowrap",
});

const contentStyle: React.CSSProperties = {
  flex: 1,
  overflow: "auto",
  padding: 16,
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  background: "var(--accent-primary)",
  color: "var(--bg-primary)",
  border: "none",
  borderRadius: 4,
  cursor: "pointer",
  fontSize: 12,
  fontFamily: "var(--font-mono)",
};

const btnSecondary: React.CSSProperties = {
  ...btnStyle,
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
};

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
  border: "1px solid var(--border-primary)",
  borderRadius: 4,
  fontSize: 13,
  fontFamily: "var(--font-mono)",
  width: "100%",
  boxSizing: "border-box",
};

const tableStyle: React.CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: 13,
};

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "8px 10px",
  borderBottom: "1px solid var(--border-primary)",
  color: "var(--text-secondary)",
  fontWeight: 600,
  fontSize: 12,
};

const tdStyle: React.CSSProperties = {
  padding: "8px 10px",
  borderBottom: "1px solid var(--border-primary)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: color + "22",
  color,
});

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-primary)",
  borderRadius: 6,
  padding: 14,
  marginBottom: 10,
};

const formGroup: React.CSSProperties = {
  marginBottom: 10,
};

const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: 12,
  color: "var(--text-secondary)",
  marginBottom: 4,
};

export function BlueTeamPanel() {
  const [activeTab, setActiveTab] = useState<BlueTeamTab>("Incidents");
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [iocs, setIOCs] = useState<IOC[]>([]);
  const [rules, setRules] = useState<DetectionRule[]>([]);
  const [cases, _setCases] = useState<ForensicsCase[]>([]);
  const [siemConns, _setSiemConns] = useState<SIEMConnection[]>([]);
  const [playbooks, _setPlaybooks] = useState<Playbook[]>([]);
  const [hunts, _setHunts] = useState<ThreatHunt[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Incident form
  const [showIncidentForm, setShowIncidentForm] = useState(false);
  const [incTitle, setIncTitle] = useState("");
  const [incSeverity, setIncSeverity] = useState<"P1" | "P2" | "P3" | "P4">("P3");
  const [incCategory, setIncCategory] = useState("Malware");
  const [incDescription, setIncDescription] = useState("");

  // IOC form
  const [showIOCForm, setShowIOCForm] = useState(false);
  const [iocType, setIOCType] = useState<IOC["ioc_type"]>("IP");
  const [iocValue, setIOCValue] = useState("");
  const [iocConfidence, setIOCConfidence] = useState(50);
  const [iocSearch, setIOCSearch] = useState("");

  // Detection rule form
  const [showRuleForm, setShowRuleForm] = useState(false);
  const [ruleName, setRuleName] = useState("");
  const [rulePlatform, setRulePlatform] = useState<DetectionRule["platform"]>("Sigma");
  const [ruleMitre, setRuleMitre] = useState("");
  const [ruleQuery, setRuleQuery] = useState("");

  // SIEM form
  const [showSIEMForm, setShowSIEMForm] = useState(false);
  const [siemPlatform, setSiemPlatform] = useState("Splunk");
  const [siemEndpoint, setSiemEndpoint] = useState("");

  // Playbook expansion
  const [expandedPlaybook, setExpandedPlaybook] = useState<string | null>(null);

  // Threat Hunt form
  const [showHuntForm, setShowHuntForm] = useState(false);
  const [huntHypothesis, setHuntHypothesis] = useState("");
  const [huntSources, setHuntSources] = useState("");
  const [huntQuery, setHuntQuery] = useState("");

  useEffect(() => {
    loadIncidents();
  }, []);

  async function loadIncidents() {
    try {
      setLoading(true);
      const result = await invoke<Incident[]>("get_blue_team_incidents");
      setIncidents(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load incidents");
    } finally {
      setLoading(false);
    }
  }

  async function loadIOCs() {
    try {
      setLoading(true);
      const result = await invoke<IOC[]>("get_blue_team_iocs", { search: iocSearch });
      setIOCs(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load IOCs");
    } finally {
      setLoading(false);
    }
  }

  async function createIncident() {
    try {
      await invoke("create_blue_team_incident", {
        title: incTitle,
        severity: incSeverity,
        category: incCategory,
        description: incDescription,
      });
      setShowIncidentForm(false);
      setIncTitle("");
      setIncDescription("");
      loadIncidents();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to create incident");
    }
  }

  async function addIOC() {
    try {
      await invoke("add_blue_team_ioc", {
        iocType: iocType,
        value: iocValue,
        confidence: iocConfidence,
      });
      setShowIOCForm(false);
      setIOCValue("");
      loadIOCs();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to add IOC");
    }
  }

  async function generateReport() {
    try {
      setLoading(true);
      await invoke("generate_blue_team_report");
      setError(null);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to generate report");
    } finally {
      setLoading(false);
    }
  }

  function renderIncidents() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Security Incidents</h3>
          <div style={{ display: "flex", gap: 8 }}>
            <button style={btnSecondary} onClick={generateReport}>Generate Report</button>
            <button style={btnStyle} onClick={() => setShowIncidentForm(!showIncidentForm)}>
              {showIncidentForm ? "Cancel" : "+ New Incident"}
            </button>
          </div>
        </div>

        {showIncidentForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={formGroup}>
              <label style={labelStyle}>Title</label>
              <input style={inputStyle} value={incTitle} onChange={(e) => setIncTitle(e.target.value)} placeholder="Incident title..." />
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Severity</label>
                <select style={inputStyle} value={incSeverity} onChange={(e) => setIncSeverity(e.target.value as any)}>
                  <option value="P1">P1 - Critical</option>
                  <option value="P2">P2 - High</option>
                  <option value="P3">P3 - Medium</option>
                  <option value="P4">P4 - Low</option>
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Category</label>
                <select style={inputStyle} value={incCategory} onChange={(e) => setIncCategory(e.target.value)}>
                  {["Malware", "Phishing", "Ransomware", "Data Breach", "DDoS", "Insider Threat", "Unauthorized Access", "Other"].map((c) => (
                    <option key={c} value={c}>{c}</option>
                  ))}
                </select>
              </div>
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Description</label>
              <textarea style={{ ...inputStyle, height: 60, resize: "vertical" }} value={incDescription} onChange={(e) => setIncDescription(e.target.value)} placeholder="Describe the incident..." />
            </div>
            <button style={btnStyle} onClick={createIncident} disabled={!incTitle}>Create Incident</button>
          </div>
        )}

        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Severity</th>
              <th style={thStyle}>Status</th>
              <th style={thStyle}>Title</th>
              <th style={thStyle}>Category</th>
              <th style={thStyle}>Assignee</th>
              <th style={thStyle}>Created</th>
            </tr>
          </thead>
          <tbody>
            {incidents.length === 0 && (
              <tr><td colSpan={6} style={{ ...tdStyle, textAlign: "center", color: "var(--text-secondary)" }}>No incidents found. Create one to get started.</td></tr>
            )}
            {incidents.map((inc) => (
              <tr key={inc.id}>
                <td style={tdStyle}><span style={badgeStyle(SEVERITY_COLORS[inc.severity] || "#6c7086")}>{inc.severity}</span></td>
                <td style={tdStyle}><span style={badgeStyle(STATUS_COLORS[inc.status] || "#6c7086")}>{inc.status}</span></td>
                <td style={tdStyle}>{inc.title}</td>
                <td style={tdStyle}>{inc.category}</td>
                <td style={tdStyle}>{inc.assignee || "—"}</td>
                <td style={tdStyle}>{inc.created}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  function renderIOCs() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Indicators of Compromise</h3>
          <button style={btnStyle} onClick={() => setShowIOCForm(!showIOCForm)}>
            {showIOCForm ? "Cancel" : "+ Add IOC"}
          </button>
        </div>

        <div style={{ marginBottom: 12, display: "flex", gap: 8 }}>
          <input style={{ ...inputStyle, flex: 1 }} value={iocSearch} onChange={(e) => setIOCSearch(e.target.value)} placeholder="Search IOCs..." />
          <button style={btnSecondary} onClick={loadIOCs}>Search</button>
        </div>

        {showIOCForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Type</label>
                <select style={inputStyle} value={iocType} onChange={(e) => setIOCType(e.target.value as any)}>
                  {["IP", "Domain", "Hash", "URL", "Email", "File"].map((t) => (
                    <option key={t} value={t}>{t}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 2 }}>
                <label style={labelStyle}>Value</label>
                <input style={inputStyle} value={iocValue} onChange={(e) => setIOCValue(e.target.value)} placeholder="e.g. 192.168.1.100 or malware.exe" />
              </div>
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Confidence: {iocConfidence}%</label>
              <input type="range" min={0} max={100} value={iocConfidence} onChange={(e) => setIOCConfidence(Number(e.target.value))} style={{ width: "100%" }} />
            </div>
            <button style={btnStyle} onClick={addIOC} disabled={!iocValue}>Add IOC</button>
          </div>
        )}

        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Type</th>
              <th style={thStyle}>Value</th>
              <th style={thStyle}>Confidence</th>
              <th style={thStyle}>Source</th>
              <th style={thStyle}>First Seen</th>
            </tr>
          </thead>
          <tbody>
            {iocs.length === 0 && (
              <tr><td colSpan={5} style={{ ...tdStyle, textAlign: "center", color: "var(--text-secondary)" }}>No IOCs found.</td></tr>
            )}
            {iocs.map((ioc) => (
              <tr key={ioc.id}>
                <td style={tdStyle}><span style={badgeStyle("var(--accent-primary)")}>{ioc.ioc_type}</span></td>
                <td style={{ ...tdStyle, fontFamily: "var(--font-mono)", fontSize: 12 }}>{ioc.value}</td>
                <td style={tdStyle}>
                  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <div style={{ flex: 1, height: 6, background: "var(--bg-tertiary)", borderRadius: 3, overflow: "hidden" }}>
                      <div style={{ width: `${ioc.confidence}%`, height: "100%", background: ioc.confidence > 75 ? "#a6e3a1" : ioc.confidence > 40 ? "#f9e2af" : "#f38ba8", borderRadius: 3 }} />
                    </div>
                    <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{ioc.confidence}%</span>
                  </div>
                </td>
                <td style={tdStyle}>{ioc.source}</td>
                <td style={tdStyle}>{ioc.first_seen}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  function renderDetectionRules() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Detection Rules</h3>
          <button style={btnStyle} onClick={() => setShowRuleForm(!showRuleForm)}>
            {showRuleForm ? "Cancel" : "+ New Rule"}
          </button>
        </div>

        {showRuleForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={formGroup}>
              <label style={labelStyle}>Rule Name</label>
              <input style={inputStyle} value={ruleName} onChange={(e) => setRuleName(e.target.value)} placeholder="e.g. Suspicious PowerShell Execution" />
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Platform</label>
                <select style={inputStyle} value={rulePlatform} onChange={(e) => setRulePlatform(e.target.value as any)}>
                  {["Sigma", "YARA", "Snort", "KQL", "SPL", "EQL"].map((p) => (
                    <option key={p} value={p}>{p}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>MITRE ATT&CK IDs (comma-separated)</label>
                <input style={inputStyle} value={ruleMitre} onChange={(e) => setRuleMitre(e.target.value)} placeholder="T1059.001, T1027" />
              </div>
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Detection Query</label>
              <textarea style={{ ...inputStyle, height: 80, resize: "vertical" }} value={ruleQuery} onChange={(e) => setRuleQuery(e.target.value)} placeholder="Enter detection query..." />
            </div>
            <button style={btnStyle} onClick={() => { setShowRuleForm(false); setRuleName(""); setRuleQuery(""); }} disabled={!ruleName}>Create Rule</button>
          </div>
        )}

        {rules.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No detection rules configured.</p>}
        {rules.map((rule) => (
          <div key={rule.id} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong style={{ fontSize: 14 }}>{rule.name}</strong>
                <span style={{ ...badgeStyle("#89b4fa"), marginLeft: 8 }}>{rule.platform}</span>
                {rule.mitre_ids.map((mid) => (
                  <span key={mid} style={{ ...badgeStyle("#cba6f7"), marginLeft: 4, fontSize: 10 }}>{mid}</span>
                ))}
              </div>
              <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12 }}>
                <input type="checkbox" checked={rule.enabled} onChange={() => {
                  setRules((prev) => prev.map((r) => r.id === rule.id ? { ...r, enabled: !r.enabled } : r));
                }} />
                {rule.enabled ? "Enabled" : "Disabled"}
              </label>
            </div>
            {rule.description && <p style={{ margin: "6px 0 0", fontSize: 12, color: "var(--text-secondary)" }}>{rule.description}</p>}
          </div>
        ))}
      </div>
    );
  }

  function renderForensics() {
    return (
      <div>
        <h3 style={{ margin: "0 0 14px", fontSize: 15 }}>Forensic Cases</h3>
        {cases.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No forensic cases. Cases are created from incident investigations.</p>}
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Case ID</th>
              <th style={thStyle}>Linked Incident</th>
              <th style={thStyle}>Status</th>
              <th style={thStyle}>Artifacts</th>
              <th style={thStyle}>Findings</th>
              <th style={thStyle}>Created</th>
            </tr>
          </thead>
          <tbody>
            {cases.map((c) => (
              <tr key={c.id}>
                <td style={{ ...tdStyle, fontFamily: "var(--font-mono)", fontSize: 11 }}>{c.id.slice(0, 8)}</td>
                <td style={tdStyle}>{c.incident_title}</td>
                <td style={tdStyle}><span style={badgeStyle(STATUS_COLORS[c.status] || "#6c7086")}>{c.status}</span></td>
                <td style={tdStyle}>{c.artifact_count}</td>
                <td style={tdStyle}>{c.finding_count}</td>
                <td style={tdStyle}>{c.created}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  function renderSIEM() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>SIEM Connections</h3>
          <button style={btnStyle} onClick={() => setShowSIEMForm(!showSIEMForm)}>
            {showSIEMForm ? "Cancel" : "+ Add Connection"}
          </button>
        </div>

        {showSIEMForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Platform</label>
                <select style={inputStyle} value={siemPlatform} onChange={(e) => setSiemPlatform(e.target.value)}>
                  {["Splunk", "Elastic SIEM", "Microsoft Sentinel", "QRadar", "Chronicle", "Sumo Logic", "Wazuh", "Graylog"].map((p) => (
                    <option key={p} value={p}>{p}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 2 }}>
                <label style={labelStyle}>Endpoint URL</label>
                <input style={inputStyle} value={siemEndpoint} onChange={(e) => setSiemEndpoint(e.target.value)} placeholder="https://siem.example.com:8089" />
              </div>
            </div>
            <button style={btnStyle} onClick={() => { setShowSIEMForm(false); setSiemEndpoint(""); }} disabled={!siemEndpoint}>Connect</button>
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 12 }}>
          {siemConns.length === 0 && (
            <p style={{ color: "var(--text-secondary)", gridColumn: "1/-1", textAlign: "center" }}>No SIEM connections configured.</p>
          )}
          {siemConns.map((conn) => (
            <div key={conn.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <strong style={{ fontSize: 14 }}>{conn.platform}</strong>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <div style={{ width: 8, height: 8, borderRadius: "50%", background: conn.status === "connected" ? "#a6e3a1" : conn.status === "error" ? "#f38ba8" : "#6c7086" }} />
                  <span style={{ fontSize: 11, color: "var(--text-secondary)", textTransform: "capitalize" }}>{conn.status}</span>
                </div>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>
                <span style={{ fontFamily: "var(--font-mono)" }}>{conn.endpoint}</span>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-secondary)" }}>
                <span>Last sync: {conn.last_sync}</span>
                <span>{conn.event_count.toLocaleString()} events</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  function renderPlaybooks() {
    return (
      <div>
        <h3 style={{ margin: "0 0 14px", fontSize: 15 }}>Incident Response Playbooks</h3>
        {playbooks.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No playbooks defined.</p>}
        {playbooks.map((pb) => (
          <div key={pb.id} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }} onClick={() => setExpandedPlaybook(expandedPlaybook === pb.id ? null : pb.id)}>
              <div>
                <strong style={{ fontSize: 14 }}>{pb.name}</strong>
                <span style={{ ...badgeStyle("#89b4fa"), marginLeft: 8 }}>{pb.category}</span>
                <span style={{ marginLeft: 8, fontSize: 11, color: "var(--text-secondary)" }}>{pb.steps.length} steps</span>
              </div>
              <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{expandedPlaybook === pb.id ? "▲" : "▼"}</span>
            </div>
            {expandedPlaybook === pb.id && (
              <div style={{ marginTop: 12 }}>
                {pb.steps.map((step) => (
                  <div key={step.order} style={{ display: "flex", alignItems: "flex-start", gap: 10, padding: "8px 0", borderTop: "1px solid var(--border-primary)" }}>
                    <span style={{ fontSize: 12, fontWeight: 600, color: "var(--accent-primary)", minWidth: 24 }}>#{step.order}</span>
                    <div style={{ flex: 1 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <span style={{ fontSize: 13, fontWeight: 500 }}>{step.name}</span>
                        {step.automated && <span style={badgeStyle("#a6e3a1")}>Automated</span>}
                      </div>
                      <p style={{ margin: "4px 0 0", fontSize: 12, color: "var(--text-secondary)" }}>{step.description}</p>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    );
  }

  function renderThreatHunt() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Threat Hunting</h3>
          <button style={btnStyle} onClick={() => setShowHuntForm(!showHuntForm)}>
            {showHuntForm ? "Cancel" : "+ New Hunt"}
          </button>
        </div>

        {showHuntForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={formGroup}>
              <label style={labelStyle}>Hypothesis</label>
              <textarea style={{ ...inputStyle, height: 50, resize: "vertical" }} value={huntHypothesis} onChange={(e) => setHuntHypothesis(e.target.value)} placeholder="e.g. An attacker is using living-off-the-land binaries for lateral movement..." />
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Data Sources (comma-separated)</label>
              <input style={inputStyle} value={huntSources} onChange={(e) => setHuntSources(e.target.value)} placeholder="e.g. EDR, Firewall Logs, DNS Logs" />
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Hunting Query</label>
              <textarea style={{ ...inputStyle, height: 80, resize: "vertical", fontFamily: "var(--font-mono)" }} value={huntQuery} onChange={(e) => setHuntQuery(e.target.value)} placeholder="Enter hunting query..." />
            </div>
            <button style={btnStyle} onClick={() => { setShowHuntForm(false); setHuntHypothesis(""); setHuntSources(""); setHuntQuery(""); }} disabled={!huntHypothesis}>Create Hunt</button>
          </div>
        )}

        {hunts.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No threat hunts. Start a hypothesis-driven hunt.</p>}
        {hunts.map((hunt) => (
          <div key={hunt.id} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <strong style={{ fontSize: 14 }}>{hunt.hypothesis.slice(0, 80)}{hunt.hypothesis.length > 80 ? "..." : ""}</strong>
              <span style={badgeStyle(STATUS_COLORS[hunt.status] || "#6c7086")}>{hunt.status}</span>
            </div>
            <div style={{ display: "flex", gap: 6, marginBottom: 8, flexWrap: "wrap" }}>
              {hunt.data_sources.map((ds) => (
                <span key={ds} style={{ ...badgeStyle("#89b4fa"), fontSize: 10 }}>{ds}</span>
              ))}
            </div>
            {hunt.query && (
              <pre style={{ margin: "8px 0", padding: 10, background: "var(--bg-tertiary)", borderRadius: 4, fontSize: 11, fontFamily: "var(--font-mono)", overflow: "auto", whiteSpace: "pre-wrap" }}>
                {hunt.query}
              </pre>
            )}
            {hunt.findings.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)" }}>Findings:</span>
                <ul style={{ margin: "4px 0 0", paddingLeft: 20, fontSize: 12 }}>
                  {hunt.findings.map((f, i) => <li key={i} style={{ marginBottom: 2 }}>{f}</li>)}
                </ul>
              </div>
            )}
          </div>
        ))}
      </div>
    );
  }

  const renderTab = () => {
    switch (activeTab) {
      case "Incidents": return renderIncidents();
      case "IOCs": return renderIOCs();
      case "Detection Rules": return renderDetectionRules();
      case "Forensics": return renderForensics();
      case "SIEM": return renderSIEM();
      case "Playbooks": return renderPlaybooks();
      case "Threat Hunt": return renderThreatHunt();
    }
  };

  return (
    <div style={containerStyle}>
      <div style={tabBarStyle}>
        {TABS.map((tab) => (
          <button key={tab} style={tabStyle(activeTab === tab)} onClick={() => setActiveTab(tab)}>
            {tab}
          </button>
        ))}
      </div>
      <div style={contentStyle}>
        {error && (
          <div style={{ padding: "8px 12px", marginBottom: 12, background: "#f38ba822", border: "1px solid #f38ba8", borderRadius: 4, fontSize: 12, color: "#f38ba8", display: "flex", justifyContent: "space-between" }}>
            <span>{error}</span>
            <button style={{ background: "none", border: "none", color: "#f38ba8", cursor: "pointer", fontSize: 14 }} onClick={() => setError(null)}>×</button>
          </div>
        )}
        {loading && <div style={{ textAlign: "center", padding: 20, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>}
        {!loading && renderTab()}
      </div>
    </div>
  );
}
