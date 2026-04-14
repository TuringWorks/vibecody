import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, ChevronDown, ChevronUp } from "lucide-react";

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
  name?: string;
  action?: string;
  description: string;
  automated?: boolean;
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
  P1: "var(--accent-rose)",
  P2: "var(--accent-gold)",
  P3: "var(--accent-gold)",
  P4: "var(--accent-blue)",
};

const STATUS_COLORS: Record<string, string> = {
  Open: "var(--accent-rose)",
  Investigating: "var(--accent-gold)",
  Contained: "var(--accent-gold)",
  Resolved: "var(--accent-green)",
  Closed: "var(--text-secondary)",
  Active: "var(--accent-green)",
  Completed: "var(--accent-blue)",
  Archived: "var(--text-secondary)",
  Draft: "var(--text-secondary)",
  Running: "var(--accent-gold)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  background: color + "22",
  color,
});

const formGroup: React.CSSProperties = {
  marginBottom: 10,
};

export function BlueTeamPanel() {
  const [activeTab, setActiveTab] = useState<BlueTeamTab>("Incidents");
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [iocs, setIOCs] = useState<IOC[]>([]);
  const [rules, setRules] = useState<DetectionRule[]>([]);
  const [cases] = useState<ForensicsCase[]>([]);
  const [siemConns, setSiemConns] = useState<SIEMConnection[]>([]);
  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [hunts, setHunts] = useState<ThreatHunt[]>([]);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
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

  const showSuccess = (msg: string) => { setSuccessMsg(msg); setTimeout(() => setSuccessMsg(null), 3000); };

  useEffect(() => {
    loadIncidents();
  }, []);

  useEffect(() => {
    if (activeTab === "IOCs" && iocs.length === 0) loadIOCs();
    if (activeTab === "Detection Rules" && rules.length === 0) loadRules();
    if (activeTab === "SIEM" && siemConns.length === 0) loadSIEM();
    if (activeTab === "Playbooks" && playbooks.length === 0) loadPlaybooks();
    if (activeTab === "Threat Hunt") loadHunts();
  }, [activeTab]);

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

  async function loadRules() {
    try {
      setLoading(true);
      const result = await invoke<DetectionRule[]>("get_blue_team_rules");
      setRules(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load rules");
    } finally {
      setLoading(false);
    }
  }

  async function loadSIEM() {
    try {
      setLoading(true);
      const result = await invoke<SIEMConnection[]>("get_blue_team_siem_connections");
      setSiemConns(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load SIEM connections");
    } finally {
      setLoading(false);
    }
  }

  async function loadPlaybooks() {
    try {
      setLoading(true);
      const result = await invoke<Playbook[]>("get_blue_team_playbooks");
      setPlaybooks(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load playbooks");
    } finally {
      setLoading(false);
    }
  }

  async function loadHunts() {
    try {
      setLoading(true);
      const result = await invoke<ThreatHunt[]>("get_blue_team_hunts");
      setHunts(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load hunts");
    } finally {
      setLoading(false);
    }
  }

  async function createRule() {
    try {
      await invoke("create_blue_team_rule", {
        name: ruleName,
        platform: rulePlatform,
        mitreIds: ruleMitre.split(",").map((s) => s.trim()).filter(Boolean),
        query: ruleQuery,
        description: null,
      });
      setShowRuleForm(false);
      setRuleName("");
      setRuleMitre("");
      setRuleQuery("");
      showSuccess("Detection rule created");
      loadRules();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to create rule");
    }
  }

  async function toggleRule(ruleId: string, enabled: boolean) {
    try {
      await invoke("toggle_blue_team_rule", { ruleId, enabled });
      loadRules();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to toggle rule");
    }
  }

  async function addSIEM() {
    try {
      await invoke("add_blue_team_siem", {
        platform: siemPlatform,
        endpoint: siemEndpoint,
      });
      setShowSIEMForm(false);
      setSiemEndpoint("");
      showSuccess("SIEM connection added");
      loadSIEM();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to add SIEM connection");
    }
  }

  async function createHunt() {
    try {
      await invoke("create_blue_team_hunt", {
        hypothesis: huntHypothesis,
        dataSources: huntSources.split(",").map((s) => s.trim()).filter(Boolean),
        query: huntQuery,
      });
      setShowHuntForm(false);
      setHuntHypothesis("");
      setHuntSources("");
      setHuntQuery("");
      showSuccess("Threat hunt created");
      loadHunts();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to create hunt");
    }
  }

  async function generateReport() {
    try {
      setLoading(true);
      const report = await invoke<string>("generate_blue_team_report");
      const blob = new Blob([report], { type: "text/markdown" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `blue-team-report-${new Date().toISOString().slice(0, 10)}.md`;
      a.click();
      URL.revokeObjectURL(url);
      showSuccess("Report downloaded");
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
          <h3 style={{ margin: 0, fontSize: "var(--font-size-xl)" }}>Security Incidents</h3>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="panel-btn panel-btn-secondary" onClick={generateReport}>Generate Report</button>
            <button className="panel-btn panel-btn-primary" onClick={() => setShowIncidentForm(!showIncidentForm)}>
              {showIncidentForm ? "Cancel" : "+ New Incident"}
            </button>
          </div>
        </div>

        {showIncidentForm && (
          <div className="panel-card" style={{ marginBottom: 16 }}>
            <div style={formGroup}>
              <label className="panel-label">Title</label>
              <input className="panel-input panel-input-full" value={incTitle} onChange={(e) => setIncTitle(e.target.value)} placeholder="Incident title..." />
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Severity</label>
                <select className="panel-input panel-input-full" value={incSeverity} onChange={(e) => setIncSeverity(e.target.value as "P1" | "P2" | "P3" | "P4")}>
                  <option value="P1">P1 - Critical</option>
                  <option value="P2">P2 - High</option>
                  <option value="P3">P3 - Medium</option>
                  <option value="P4">P4 - Low</option>
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Category</label>
                <select className="panel-input panel-input-full" value={incCategory} onChange={(e) => setIncCategory(e.target.value)}>
                  {["Malware", "Phishing", "Ransomware", "Data Breach", "DDoS", "Insider Threat", "Unauthorized Access", "Other"].map((c) => (
                    <option key={c} value={c}>{c}</option>
                  ))}
                </select>
              </div>
            </div>
            <div style={formGroup}>
              <label className="panel-label">Description</label>
              <textarea className="panel-input panel-input-full" style={{ height: 60, resize: "vertical" }} value={incDescription} onChange={(e) => setIncDescription(e.target.value)} placeholder="Describe the incident..." />
            </div>
            <button className="panel-btn panel-btn-primary" onClick={createIncident} disabled={!incTitle}>Create Incident</button>
          </div>
        )}

        <table className="panel-table">
          <thead>
            <tr>
              <th >Severity</th>
              <th >Status</th>
              <th >Title</th>
              <th >Category</th>
              <th >Assignee</th>
              <th >Created</th>
            </tr>
          </thead>
          <tbody>
            {incidents.length === 0 && (
              <tr><td colSpan={6} style={{ textAlign: "center", color: "var(--text-secondary)" }}>No incidents found. Create one to get started.</td></tr>
            )}
            {incidents.map((inc) => (
              <tr key={inc.id}>
                <td ><span style={badgeStyle(SEVERITY_COLORS[inc.severity] || "var(--text-secondary)")}>{inc.severity}</span></td>
                <td ><span style={badgeStyle(STATUS_COLORS[inc.status] || "var(--text-secondary)")}>{inc.status}</span></td>
                <td >{inc.title}</td>
                <td >{inc.category}</td>
                <td >{inc.assignee || "—"}</td>
                <td >{inc.created}</td>
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
          <h3 style={{ margin: 0, fontSize: "var(--font-size-xl)" }}>Indicators of Compromise</h3>
          <button className="panel-btn panel-btn-primary" onClick={() => setShowIOCForm(!showIOCForm)}>
            {showIOCForm ? "Cancel" : "+ Add IOC"}
          </button>
        </div>

        <div style={{ marginBottom: 12, display: "flex", gap: 8 }}>
          <input className="panel-input" style={{ flex: 1 }} value={iocSearch} onChange={(e) => setIOCSearch(e.target.value)} placeholder="Search IOCs..." />
          <button className="panel-btn panel-btn-secondary" onClick={loadIOCs}>Search</button>
        </div>

        {showIOCForm && (
          <div className="panel-card" style={{ marginBottom: 16 }}>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Type</label>
                <select className="panel-input panel-input-full" value={iocType} onChange={(e) => setIOCType(e.target.value as "IP" | "Domain" | "Hash" | "URL" | "Email" | "File")}>
                  {["IP", "Domain", "Hash", "URL", "Email", "File"].map((t) => (
                    <option key={t} value={t}>{t}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 2 }}>
                <label className="panel-label">Value</label>
                <input className="panel-input panel-input-full" value={iocValue} onChange={(e) => setIOCValue(e.target.value)} placeholder="e.g. 192.168.1.100 or malware.exe" />
              </div>
            </div>
            <div style={formGroup}>
              <label className="panel-label">Confidence: {iocConfidence}%</label>
              <input type="range" min={0} max={100} value={iocConfidence} onChange={(e) => setIOCConfidence(Number(e.target.value))} style={{ width: "100%" }} />
            </div>
            <button className="panel-btn panel-btn-primary" onClick={addIOC} disabled={!iocValue}>Add IOC</button>
          </div>
        )}

        <table className="panel-table">
          <thead>
            <tr>
              <th >Type</th>
              <th >Value</th>
              <th >Confidence</th>
              <th >Source</th>
              <th >First Seen</th>
            </tr>
          </thead>
          <tbody>
            {iocs.length === 0 && (
              <tr><td colSpan={5} style={{ textAlign: "center", color: "var(--text-secondary)" }}>No IOCs found.</td></tr>
            )}
            {iocs.map((ioc) => (
              <tr key={ioc.id}>
                <td ><span style={badgeStyle("var(--accent-blue)")}>{ioc.ioc_type}</span></td>
                <td style={{ fontSize: "var(--font-size-base)" }}>{ioc.value}</td>
                <td >
                  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <div style={{ flex: 1, height: 6, background: "var(--bg-tertiary)", borderRadius: 3, overflow: "hidden" }}>
                      <div style={{ width: `${ioc.confidence}%`, height: "100%", background: ioc.confidence > 75 ? "var(--accent-green)" : ioc.confidence > 40 ? "var(--accent-gold)" : "var(--accent-rose)", borderRadius: 3 }} />
                    </div>
                    <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{ioc.confidence}%</span>
                  </div>
                </td>
                <td >{ioc.source}</td>
                <td >{ioc.first_seen}</td>
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
          <h3 style={{ margin: 0, fontSize: "var(--font-size-xl)" }}>Detection Rules</h3>
          <button className="panel-btn panel-btn-primary" onClick={() => setShowRuleForm(!showRuleForm)}>
            {showRuleForm ? "Cancel" : "+ New Rule"}
          </button>
        </div>

        {showRuleForm && (
          <div className="panel-card" style={{ marginBottom: 16 }}>
            <div style={formGroup}>
              <label className="panel-label">Rule Name</label>
              <input className="panel-input panel-input-full" value={ruleName} onChange={(e) => setRuleName(e.target.value)} placeholder="e.g. Suspicious PowerShell Execution" />
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Platform</label>
                <select className="panel-input panel-input-full" value={rulePlatform} onChange={(e) => setRulePlatform(e.target.value as "Sigma" | "YARA" | "Snort" | "KQL" | "SPL" | "EQL")}>
                  {["Sigma", "YARA", "Snort", "KQL", "SPL", "EQL"].map((p) => (
                    <option key={p} value={p}>{p}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">MITRE ATT&CK IDs (comma-separated)</label>
                <input className="panel-input panel-input-full" value={ruleMitre} onChange={(e) => setRuleMitre(e.target.value)} placeholder="T1059.001, T1027" />
              </div>
            </div>
            <div style={formGroup}>
              <label className="panel-label">Detection Query</label>
              <textarea className="panel-input panel-input-full" style={{ height: 80, resize: "vertical" }} value={ruleQuery} onChange={(e) => setRuleQuery(e.target.value)} placeholder="Enter detection query..." />
            </div>
            <button className="panel-btn panel-btn-primary" onClick={createRule} disabled={!ruleName}>Create Rule</button>
          </div>
        )}

        {rules.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No detection rules configured.</p>}
        {rules.map((rule) => (
          <div key={rule.id} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong style={{ fontSize: "var(--font-size-lg)" }}>{rule.name}</strong>
                <span style={{ ...badgeStyle("var(--info-color)"), marginLeft: 8 }}>{rule.platform}</span>
                {rule.mitre_ids.map((mid) => (
                  <span key={mid} style={{ ...badgeStyle("var(--accent-purple)"), marginLeft: 4, fontSize: "var(--font-size-xs)" }}>{mid}</span>
                ))}
              </div>
              <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: "var(--font-size-base)" }}>
                <input type="checkbox" checked={rule.enabled} onChange={() => {
                  toggleRule(rule.id, !rule.enabled);
                }} />
                {rule.enabled ? "Enabled" : "Disabled"}
              </label>
            </div>
            {rule.description && <p style={{ margin: "6px 0 0", fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{rule.description}</p>}
          </div>
        ))}
      </div>
    );
  }

  function renderForensics() {
    return (
      <div>
        <h3 style={{ margin: "0 0 14px", fontSize: "var(--font-size-xl)" }}>Forensic Cases</h3>
        {cases.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No forensic cases. Cases are created from incident investigations.</p>}
        <table className="panel-table">
          <thead>
            <tr>
              <th >Case ID</th>
              <th >Linked Incident</th>
              <th >Status</th>
              <th >Artifacts</th>
              <th >Findings</th>
              <th >Created</th>
            </tr>
          </thead>
          <tbody>
            {cases.map((c) => (
              <tr key={c.id}>
                <td style={{ fontSize: "var(--font-size-sm)" }}>{c.id.slice(0, 8)}</td>
                <td >{c.incident_title}</td>
                <td ><span style={badgeStyle(STATUS_COLORS[c.status] || "var(--text-secondary)")}>{c.status}</span></td>
                <td >{c.artifact_count}</td>
                <td >{c.finding_count}</td>
                <td >{c.created}</td>
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
          <h3 style={{ margin: 0, fontSize: "var(--font-size-xl)" }}>SIEM Connections</h3>
          <button className="panel-btn panel-btn-primary" onClick={() => setShowSIEMForm(!showSIEMForm)}>
            {showSIEMForm ? "Cancel" : "+ Add Connection"}
          </button>
        </div>

        {showSIEMForm && (
          <div className="panel-card" style={{ marginBottom: 16 }}>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Platform</label>
                <select className="panel-input panel-input-full" value={siemPlatform} onChange={(e) => setSiemPlatform(e.target.value)}>
                  {["Splunk", "Elastic SIEM", "Microsoft Sentinel", "QRadar", "Chronicle", "Sumo Logic", "Wazuh", "Graylog"].map((p) => (
                    <option key={p} value={p}>{p}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 2 }}>
                <label className="panel-label">Endpoint URL</label>
                <input className="panel-input panel-input-full" value={siemEndpoint} onChange={(e) => setSiemEndpoint(e.target.value)} placeholder="https://siem.example.com:8089" />
              </div>
            </div>
            <button className="panel-btn panel-btn-primary" onClick={addSIEM} disabled={!siemEndpoint}>Connect</button>
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 12 }}>
          {siemConns.length === 0 && (
            <p style={{ color: "var(--text-secondary)", gridColumn: "1/-1", textAlign: "center" }}>No SIEM connections configured.</p>
          )}
          {siemConns.map((conn) => (
            <div key={conn.id} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <strong style={{ fontSize: "var(--font-size-lg)" }}>{conn.platform}</strong>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <div style={{ width: 8, height: 8, borderRadius: "50%", background: conn.status === "connected" ? "var(--accent-green)" : conn.status === "error" ? "var(--accent-rose)" : "var(--text-secondary)" }} />
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", textTransform: "capitalize" }}>{conn.status}</span>
                </div>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>
                <span style={{ fontFamily: "inherit" }}>{conn.endpoint}</span>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
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
        <h3 style={{ margin: "0 0 14px", fontSize: "var(--font-size-xl)" }}>Incident Response Playbooks</h3>
        {playbooks.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No playbooks defined.</p>}
        {playbooks.map((pb) => (
          <div key={pb.id} className="panel-card">
            <div role="button" tabIndex={0} aria-expanded={expandedPlaybook === pb.id} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }} onClick={() => setExpandedPlaybook(expandedPlaybook === pb.id ? null : pb.id)} onKeyDown={e => e.key === "Enter" && setExpandedPlaybook(expandedPlaybook === pb.id ? null : pb.id)}>
              <div>
                <strong style={{ fontSize: "var(--font-size-lg)" }}>{pb.name}</strong>
                <span style={{ ...badgeStyle("var(--info-color)"), marginLeft: 8 }}>{pb.category}</span>
                <span style={{ marginLeft: 8, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{pb.steps.length} steps</span>
              </div>
              <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{expandedPlaybook === pb.id ? <ChevronUp size={12} /> : <ChevronDown size={12} />}</span>
            </div>
            {expandedPlaybook === pb.id && (
              <div style={{ marginTop: 12 }}>
                {pb.steps.map((step) => (
                  <div key={step.order} style={{ display: "flex", alignItems: "flex-start", gap: 10, padding: "8px 0", borderTop: "1px solid var(--border-color)" }}>
                    <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--accent-blue)", minWidth: 24 }}>#{step.order}</span>
                    <div style={{ flex: 1 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <span style={{ fontSize: "var(--font-size-md)", fontWeight: 500 }}>{step.name || step.action}</span>
                        {step.automated && <span style={badgeStyle("var(--success-color)")}>Auto</span>}
                      </div>
                      <p style={{ margin: "4px 0 0", fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{step.description}</p>
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
          <h3 style={{ margin: 0, fontSize: "var(--font-size-xl)" }}>Threat Hunting</h3>
          <button className="panel-btn panel-btn-primary" onClick={() => setShowHuntForm(!showHuntForm)}>
            {showHuntForm ? "Cancel" : "+ New Hunt"}
          </button>
        </div>

        {showHuntForm && (
          <div className="panel-card" style={{ marginBottom: 16 }}>
            <div style={formGroup}>
              <label className="panel-label">Hypothesis</label>
              <textarea className="panel-input panel-input-full" style={{ height: 50, resize: "vertical" }} value={huntHypothesis} onChange={(e) => setHuntHypothesis(e.target.value)} placeholder="e.g. An attacker is using living-off-the-land binaries for lateral movement..." />
            </div>
            <div style={formGroup}>
              <label className="panel-label">Data Sources (comma-separated)</label>
              <input className="panel-input panel-input-full" value={huntSources} onChange={(e) => setHuntSources(e.target.value)} placeholder="e.g. EDR, Firewall Logs, DNS Logs" />
            </div>
            <div style={formGroup}>
              <label className="panel-label">Hunting Query</label>
              <textarea className="panel-input panel-input-full" style={{ height: 80, resize: "vertical" }} value={huntQuery} onChange={(e) => setHuntQuery(e.target.value)} placeholder="Enter hunting query..." />
            </div>
            <button className="panel-btn panel-btn-primary" onClick={createHunt} disabled={!huntHypothesis}>Create Hunt</button>
          </div>
        )}

        {hunts.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No threat hunts. Start a hypothesis-driven hunt.</p>}
        {hunts.map((hunt) => (
          <div key={hunt.id} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <strong style={{ fontSize: "var(--font-size-lg)" }}>{hunt.hypothesis.slice(0, 80)}{hunt.hypothesis.length > 80 ? "..." : ""}</strong>
              <span style={badgeStyle(STATUS_COLORS[hunt.status] || "var(--text-secondary)")}>{hunt.status}</span>
            </div>
            <div style={{ display: "flex", gap: 6, marginBottom: 8, flexWrap: "wrap" }}>
              {hunt.data_sources.map((ds) => (
                <span key={ds} style={{ ...badgeStyle("var(--info-color)"), fontSize: "var(--font-size-xs)" }}>{ds}</span>
              ))}
            </div>
            {hunt.query && (
              <pre style={{ margin: "8px 0", padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-sm)", fontFamily: "inherit", overflow: "auto", whiteSpace: "pre-wrap" }}>
                {hunt.query}
              </pre>
            )}
            {hunt.findings.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--text-secondary)" }}>Findings:</span>
                <ul style={{ margin: "4px 0 0", paddingLeft: 20, fontSize: "var(--font-size-base)" }}>
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
    <div className="panel-container">
      <div className="panel-tab-bar">
        {TABS.map((tab) => (
          <button key={tab} className={`panel-tab ${activeTab === tab ? "active" : ""}`} onClick={() => setActiveTab(tab)}>
            {tab}
          </button>
        ))}
      </div>
      <div className="panel-body">
        {successMsg && (
          <div style={{ padding: "8px 12px", marginBottom: 12, background: "var(--success-bg)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)", color: "var(--success-color)" }}>
            {successMsg}
          </div>
        )}
        {error && (
          <div className="panel-error" style={{ marginBottom: 12, display: "flex", justifyContent: "space-between" }}>
            <span>{error}</span>
            <button aria-label="Dismiss error" style={{ background: "none", border: "none", color: "var(--error-color)", cursor: "pointer", display: "flex", alignItems: "center" }} onClick={() => setError(null)}><X size={14} /></button>
          </div>
        )}
        {loading && <div className="panel-loading">Loading...</div>}
        {!loading && renderTab()}
      </div>
    </div>
  );
}
