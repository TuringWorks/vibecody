/**
 * IdpPanel — Internal Developer Platform management.
 *
 * Tabs: Service Catalog, Golden Paths, Scorecards, Infrastructure, Teams, Platforms, Backstage
 *
 * All data is persisted to ~/.vibecli/idp/ via Tauri commands.
 */
import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type IdpTab = "Service Catalog" | "Golden Paths" | "Scorecards" | "Infrastructure" | "Teams" | "Platforms" | "Backstage";

interface Service {
  id: string;
  name: string;
  owner: string;
  tier: "Tier0" | "Tier1" | "Tier2" | "Tier3";
  status: "Active" | "Deprecated" | "Incubating" | "Sunset";
  language: string;
  framework: string;
  repo_url: string;
  description: string;
}

interface GoldenPath {
  id: string;
  language: string;
  framework: string;
  template_repo: string;
  description: string;
  features: string[];
}

interface ScorecardMetric {
  name: string;
  score: number;
  max_score: number;
  category: string;
}

interface Scorecard {
  service_id: string;
  service_name: string;
  overall_grade: string;
  overall_score: number;
  metrics: ScorecardMetric[];
  recommendations: string[];
}

interface InfraRequest {
  id: string;
  template: string;
  status: "Pending" | "Provisioning" | "Completed" | "Failed";
  requested_by: string;
  created: string;
  config: Record<string, string>;
}

interface Team {
  id: string;
  name: string;
  member_count: number;
  service_count: number;
  onboarding_progress: number;
  onboarding_checklist: ChecklistItem[];
}

interface ChecklistItem {
  label: string;
  completed: boolean;
}

interface IdpPlatform {
  name: string;
  enabled: boolean;
  features: string[];
  config_url: string;
  description: string;
}

const TABS: IdpTab[] = ["Service Catalog", "Golden Paths", "Scorecards", "Infrastructure", "Teams", "Platforms", "Backstage"];

const TIER_COLORS: Record<string, string> = {
  Tier0: "var(--error-color)",
  Tier1: "var(--warning-color)",
  Tier2: "var(--warning-color)",
  Tier3: "var(--success-color)",
};

const STATUS_COLORS: Record<string, string> = {
  Active: "var(--success-color)",
  Deprecated: "var(--error-color)",
  Incubating: "var(--info-color)",
  Sunset: "var(--text-secondary)",
  Pending: "var(--warning-color)",
  Provisioning: "var(--info-color)",
  Completed: "var(--success-color)",
  Failed: "var(--error-color)",
};

const GRADE_COLORS: Record<string, string> = {
  A: "var(--success-color)",
  B: "var(--info-color)",
  C: "var(--warning-color)",
  D: "var(--warning-color)",
  F: "var(--error-color)",
};

const INFRA_TEMPLATES = [
  "PostgreSQL Database",
  "Redis Cache",
  "S3 Bucket",
  "Kubernetes Namespace",
  "API Gateway",
  "CDN Distribution",
  "Message Queue",
  "Monitoring Stack",
  "CI/CD Pipeline",
  "Load Balancer",
];

// ── Styles ──────────────────────────────────────────────────────────────────

const containerStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  height: "100%",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontFamily: "inherit",
  overflow: "hidden",
};

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: 2,
  padding: "8px 12px 0",
  borderBottom: "1px solid var(--border-color)",
  background: "var(--bg-secondary)",
  overflowX: "auto",
  flexShrink: 0,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px",
  cursor: "pointer",
  background: active ? "var(--bg-primary)" : "transparent",
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  border: "none",
  borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  fontSize: 13,
  fontFamily: "inherit",
  whiteSpace: "nowrap",
});

const contentStyle: React.CSSProperties = {
  flex: 1,
  overflow: "auto",
  padding: 16,
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  background: "var(--accent-color)",
  color: "var(--bg-primary)",
  border: "none",
  borderRadius: 4,
  cursor: "pointer",
  fontSize: 12,
  fontFamily: "inherit",
};

const btnSecondary: React.CSSProperties = {
  ...btnStyle,
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
};

const btnDanger: React.CSSProperties = {
  ...btnStyle,
  background: "transparent",
  color: "var(--error-color)",
  border: "1px solid var(--error-color)",
  padding: "2px 8px",
  fontSize: 11,
};

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
  border: "1px solid var(--border-color)",
  borderRadius: 4,
  fontSize: 13,
  fontFamily: "inherit",
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
  borderBottom: "1px solid var(--border-color)",
  color: "var(--text-secondary)",
  fontWeight: 600,
  fontSize: 12,
};

const tdStyle: React.CSSProperties = {
  padding: "8px 10px",
  borderBottom: "1px solid var(--border-color)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: "transparent",
  color,
  border: `1px solid ${color}`,
});

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
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

// ── Component ───────────────────────────────────────────────────────────────

export function IdpPanel() {
  const [activeTab, setActiveTab] = useState<IdpTab>("Service Catalog");
  const [services, setServices] = useState<Service[]>([]);
  const [goldenPaths, setGoldenPaths] = useState<GoldenPath[]>([]);
  const [scorecard, setScorecard] = useState<Scorecard | null>(null);
  const [infraRequests, setInfraRequests] = useState<InfraRequest[]>([]);
  const [teams, setTeams] = useState<Team[]>([]);
  const [platforms, setPlatforms] = useState<IdpPlatform[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  // Service catalog form
  const [serviceSearch, setServiceSearch] = useState("");
  const [showServiceForm, setShowServiceForm] = useState(false);
  const [svcName, setSvcName] = useState("");
  const [svcOwner, setSvcOwner] = useState("");
  const [svcTier, setSvcTier] = useState<Service["tier"]>("Tier2");
  const [svcLanguage, setSvcLanguage] = useState("TypeScript");
  const [svcFramework, setSvcFramework] = useState("React");
  const [svcRepo, setSvcRepo] = useState("");
  const [svcDescription, setSvcDescription] = useState("");

  // Golden Paths
  const [gpLanguageFilter, setGpLanguageFilter] = useState("");

  // Scorecards
  const [scorecardServiceId, setScorecardServiceId] = useState("");

  // Infrastructure
  const [showInfraForm, setShowInfraForm] = useState(false);
  const [infraTemplate, setInfraTemplate] = useState("PostgreSQL Database");
  const [infraEnv, setInfraEnv] = useState("staging");
  const [infraRegion, setInfraRegion] = useState("us-east-1");
  const [infraSize, setInfraSize] = useState("small");

  // Teams
  const [showTeamForm, setShowTeamForm] = useState(false);
  const [teamName, setTeamName] = useState("");
  const [expandedTeam, setExpandedTeam] = useState<string | null>(null);

  // Backstage
  const [backstageServiceId, setBackstageServiceId] = useState("");
  const [catalogYaml, setCatalogYaml] = useState("");

  const showSuccess = useCallback((msg: string) => {
    setSuccessMsg(msg);
    setTimeout(() => setSuccessMsg(null), 3000);
  }, []);

  const showError = useCallback((msg: string) => {
    setError(msg);
    setTimeout(() => setError(null), 8000);
  }, []);

  // ── Data loaders ──────────────────────────────────────────────────────────

  const loadCatalog = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<Service[]>("get_idp_catalog");
      setServices(result);
    } catch (e: unknown) {
      showError(String(e));
    } finally {
      setLoading(false);
    }
  }, [showError]);

  const loadGoldenPaths = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<GoldenPath[]>("get_idp_golden_paths");
      setGoldenPaths(result);
    } catch (e: unknown) {
      showError(String(e));
    } finally {
      setLoading(false);
    }
  }, [showError]);

  const loadTeams = useCallback(async () => {
    try {
      const result = await invoke<Team[]>("get_idp_teams");
      setTeams(result);
    } catch (e: unknown) {
      showError(String(e));
    }
  }, [showError]);

  const loadInfraRequests = useCallback(async () => {
    try {
      const result = await invoke<InfraRequest[]>("get_idp_infra_requests");
      setInfraRequests(result);
    } catch (e: unknown) {
      showError(String(e));
    }
  }, [showError]);

  const loadPlatforms = useCallback(async () => {
    try {
      const result = await invoke<IdpPlatform[]>("get_idp_platforms");
      setPlatforms(result);
    } catch (e: unknown) {
      showError(String(e));
    }
  }, [showError]);

  useEffect(() => {
    loadCatalog();
  }, [loadCatalog]);

  // Load tab-specific data when switching tabs
  useEffect(() => {
    if (activeTab === "Golden Paths" && goldenPaths.length === 0) loadGoldenPaths();
    if (activeTab === "Teams") loadTeams();
    if (activeTab === "Infrastructure") loadInfraRequests();
    if (activeTab === "Platforms" && platforms.length === 0) loadPlatforms();
  }, [activeTab, goldenPaths.length, platforms.length, loadGoldenPaths, loadTeams, loadInfraRequests, loadPlatforms]);

  // ── Actions ───────────────────────────────────────────────────────────────

  async function registerService() {
    if (!svcName.trim() || !svcOwner.trim()) return;
    try {
      await invoke("register_idp_service", {
        name: svcName.trim(),
        owner: svcOwner.trim(),
        tier: svcTier,
        language: svcLanguage,
        framework: svcFramework,
        repoUrl: svcRepo.trim() || null,
        description: svcDescription.trim() || null,
      });
      setShowServiceForm(false);
      setSvcName(""); setSvcOwner(""); setSvcRepo(""); setSvcDescription("");
      setSvcFramework("React"); setSvcLanguage("TypeScript"); setSvcTier("Tier2");
      showSuccess("Service registered successfully.");
      loadCatalog();
    } catch (e: unknown) {
      showError(String(e));
    }
  }

  async function deleteService(id: string) {
    try {
      await invoke("delete_idp_service", { serviceId: id });
      showSuccess("Service removed.");
      loadCatalog();
    } catch (e: unknown) {
      showError(String(e));
    }
  }

  async function loadScorecard(serviceId: string) {
    try {
      setLoading(true);
      const result = await invoke<Scorecard>("get_idp_scorecards", { serviceId });
      setScorecard(result);
    } catch (e: unknown) {
      showError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function evaluateScorecard(serviceId: string) {
    try {
      setLoading(true);
      const result = await invoke<Scorecard>("evaluate_idp_scorecard", { serviceId });
      setScorecard(result);
      showSuccess(`Scorecard evaluated: Grade ${result.overall_grade} (${result.overall_score}/100)`);
    } catch (e: unknown) {
      showError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function submitInfraRequest() {
    try {
      await invoke("request_idp_infra", {
        template: infraTemplate,
        environment: infraEnv,
        region: infraRegion,
        size: infraSize,
      });
      setShowInfraForm(false);
      showSuccess("Infrastructure request submitted.");
      loadInfraRequests();
    } catch (e: unknown) {
      showError(String(e));
    }
  }

  async function createTeam() {
    if (!teamName.trim()) return;
    try {
      await invoke("create_idp_team", { name: teamName.trim() });
      setShowTeamForm(false);
      setTeamName("");
      showSuccess("Team created with onboarding checklist.");
      loadTeams();
    } catch (e: unknown) {
      showError(String(e));
    }
  }

  async function toggleChecklist(teamId: string, itemIndex: number) {
    try {
      const updated = await invoke<Team>("toggle_idp_checklist", { teamId, itemIndex });
      setTeams((prev) => prev.map((t) => (t.id === teamId ? updated : t)));
    } catch (e: unknown) {
      showError(String(e));
    }
  }

  async function togglePlatform(platformName: string, enabled: boolean) {
    try {
      const result = await invoke<IdpPlatform[]>("toggle_idp_platform", { platformName, enabled });
      setPlatforms(result);
    } catch (e: unknown) {
      showError(String(e));
    }
  }

  async function generateCatalogYaml(serviceId: string) {
    try {
      const yaml = await invoke<string>("generate_backstage_catalog", { serviceId });
      setCatalogYaml(yaml);
      setBackstageServiceId(serviceId);
    } catch (e: unknown) {
      showError(String(e));
    }
  }

  // Also support local generation for services (used in Backstage tab inline)
  function generateCatalogInfoLocal(service: Service) {
    const name = service.name.toLowerCase().replace(/\s+/g, "-");
    const yaml = `apiVersion: backstage.io/v1alpha1
kind: Component
metadata:
  name: ${name}
  description: ${service.description || service.name}
  annotations:
    github.com/project-slug: ${service.repo_url ? service.repo_url.replace("https://github.com/", "") : `org/${name}`}
    backstage.io/techdocs-ref: dir:.
  tags:
    - ${service.language.toLowerCase()}${service.framework ? `\n    - ${service.framework.toLowerCase()}` : ""}
    - ${service.tier.toLowerCase()}
spec:
  type: service
  lifecycle: ${service.status === "Active" ? "production" : service.status === "Incubating" ? "experimental" : "deprecated"}
  owner: ${service.owner.toLowerCase().replace(/\s+/g, "-")}
  system: ${name}-system
  providesApis:
    - ${name}-api`;
    setCatalogYaml(yaml);
  }

  const filteredServices = services.filter(
    (s) =>
      !serviceSearch ||
      s.name.toLowerCase().includes(serviceSearch.toLowerCase()) ||
      s.owner.toLowerCase().includes(serviceSearch.toLowerCase()) ||
      s.language.toLowerCase().includes(serviceSearch.toLowerCase())
  );

  const filteredPaths = goldenPaths.filter(
    (gp) => !gpLanguageFilter || gp.language.toLowerCase().includes(gpLanguageFilter.toLowerCase())
  );

  // ── Tab Renderers ─────────────────────────────────────────────────────────

  function renderServiceCatalog() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Service Catalog ({services.length} services)</h3>
          <div style={{ display: "flex", gap: 6 }}>
            <button style={btnSecondary} onClick={loadCatalog}>Refresh</button>
            <button style={btnStyle} onClick={() => setShowServiceForm(!showServiceForm)}>
              {showServiceForm ? "Cancel" : "+ Register Service"}
            </button>
          </div>
        </div>

        <div style={{ marginBottom: 12 }}>
          <input style={inputStyle} value={serviceSearch} onChange={(e) => setServiceSearch(e.target.value)} placeholder="Search services by name, owner, or language..." />
        </div>

        {showServiceForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 2 }}>
                <label style={labelStyle}>Service Name *</label>
                <input style={inputStyle} value={svcName} onChange={(e) => setSvcName(e.target.value)} placeholder="e.g. user-service" />
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Owner *</label>
                <input style={inputStyle} value={svcOwner} onChange={(e) => setSvcOwner(e.target.value)} placeholder="Team or person" />
              </div>
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Tier</label>
                <select style={inputStyle} value={svcTier} onChange={(e) => setSvcTier(e.target.value as Service["tier"])}>
                  <option value="Tier0">Tier 0 - Critical</option>
                  <option value="Tier1">Tier 1 - High</option>
                  <option value="Tier2">Tier 2 - Medium</option>
                  <option value="Tier3">Tier 3 - Low</option>
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Language</label>
                <select style={inputStyle} value={svcLanguage} onChange={(e) => setSvcLanguage(e.target.value)}>
                  {["TypeScript", "Rust", "Go", "Python", "Java", "C#", "Ruby", "Kotlin", "Swift", "Elixir"].map((l) => (
                    <option key={l} value={l}>{l}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Framework</label>
                <input style={inputStyle} value={svcFramework} onChange={(e) => setSvcFramework(e.target.value)} placeholder="e.g. Next.js, Actix" />
              </div>
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Repository URL</label>
              <input style={inputStyle} value={svcRepo} onChange={(e) => setSvcRepo(e.target.value)} placeholder="https://github.com/org/repo" />
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Description</label>
              <textarea style={{ ...inputStyle, height: 50, resize: "vertical" }} value={svcDescription} onChange={(e) => setSvcDescription(e.target.value)} placeholder="Brief description of the service..." />
            </div>
            <button style={{ ...btnStyle, opacity: (!svcName || !svcOwner) ? 0.5 : 1 }} onClick={registerService} disabled={!svcName || !svcOwner}>Register Service</button>
          </div>
        )}

        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Name</th>
              <th style={thStyle}>Owner</th>
              <th style={thStyle}>Tier</th>
              <th style={thStyle}>Status</th>
              <th style={thStyle}>Language</th>
              <th style={thStyle}>Framework</th>
              <th style={thStyle}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {filteredServices.length === 0 && (
              <tr><td colSpan={7} style={{ ...tdStyle, textAlign: "center", color: "var(--text-secondary)" }}>
                {services.length === 0 ? "No services registered. Click \"+ Register Service\" to add your first service." : "No matching services."}
              </td></tr>
            )}
            {filteredServices.map((svc) => (
              <tr key={svc.id}>
                <td style={{ ...tdStyle, fontWeight: 500 }}>{svc.name}</td>
                <td style={tdStyle}>{svc.owner}</td>
                <td style={tdStyle}><span style={badgeStyle(TIER_COLORS[svc.tier] || "var(--text-secondary)")}>{svc.tier}</span></td>
                <td style={tdStyle}><span style={badgeStyle(STATUS_COLORS[svc.status] || "var(--text-secondary)")}>{svc.status}</span></td>
                <td style={tdStyle}>{svc.language}</td>
                <td style={tdStyle}>{svc.framework || "—"}</td>
                <td style={tdStyle}>
                  <div style={{ display: "flex", gap: 4 }}>
                    {svc.repo_url && (
                      <a href={svc.repo_url} target="_blank" rel="noopener noreferrer" style={{ ...btnSecondary, padding: "2px 8px", fontSize: 11, textDecoration: "none" }}>Repo</a>
                    )}
                    <button style={btnDanger} onClick={() => deleteService(svc.id)}>Remove</button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  function renderGoldenPaths() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Golden Paths ({goldenPaths.length} templates)</h3>
          <button style={btnSecondary} onClick={loadGoldenPaths}>Refresh</button>
        </div>

        <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 12px" }}>
          Opinionated, production-ready project templates that encode best practices for each language and framework.
        </p>

        <div style={{ marginBottom: 12 }}>
          <input style={{ ...inputStyle, width: 250 }} value={gpLanguageFilter} onChange={(e) => setGpLanguageFilter(e.target.value)} placeholder="Filter by language..." />
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: 12 }}>
          {filteredPaths.length === 0 && (
            <p style={{ color: "var(--text-secondary)", gridColumn: "1/-1", textAlign: "center" }}>No matching golden paths.</p>
          )}
          {filteredPaths.map((gp) => (
            <div key={gp.id} style={cardStyle}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                <span style={{ fontSize: 20 }}>
                  {gp.language === "TypeScript" ? "TS" : gp.language === "Rust" ? "Rs" : gp.language === "Go" ? "Go" : gp.language === "Python" ? "Py" : gp.language === "Java" ? "Jv" : gp.language === "Kotlin" ? "Kt" : gp.language.slice(0, 2)}
                </span>
                <div>
                  <strong style={{ fontSize: 14 }}>{gp.framework}</strong>
                  <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{gp.language}</div>
                </div>
              </div>
              <p style={{ margin: "0 0 8px", fontSize: 12, color: "var(--text-secondary)" }}>{gp.description}</p>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap", marginBottom: 8 }}>
                {gp.features.map((f) => (
                  <span key={f} style={{ ...badgeStyle("var(--info-color)"), fontSize: 10 }}>{f}</span>
                ))}
              </div>
              <div style={{ fontSize: 11, fontFamily: "inherit", color: "var(--text-secondary)" }}>
                Template: {gp.template_repo}
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  function renderScorecards() {
    return (
      <div>
        <h3 style={{ margin: "0 0 8px", fontSize: 15 }}>Service Scorecards</h3>
        <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 14px" }}>
          Evaluate services against quality, governance, standards, and DORA metrics. Scores are computed from service metadata and can be improved by completing recommendations.
        </p>

        <div style={{ ...cardStyle, marginBottom: 16 }}>
          <div style={{ display: "flex", gap: 10, alignItems: "flex-end" }}>
            <div style={{ ...formGroup, flex: 1, marginBottom: 0 }}>
              <label style={labelStyle}>Select Service</label>
              <select style={inputStyle} value={scorecardServiceId} onChange={(e) => setScorecardServiceId(e.target.value)}>
                <option value="">Choose a service...</option>
                {services.map((svc) => (
                  <option key={svc.id} value={svc.id}>{svc.name}</option>
                ))}
              </select>
            </div>
            <button style={btnSecondary} onClick={() => scorecardServiceId && loadScorecard(scorecardServiceId)} disabled={!scorecardServiceId}>Load</button>
            <button style={btnStyle} onClick={() => scorecardServiceId && evaluateScorecard(scorecardServiceId)} disabled={!scorecardServiceId}>Evaluate</button>
          </div>
          {services.length === 0 && (
            <p style={{ margin: "10px 0 0", fontSize: 11, color: "var(--text-secondary)" }}>Register services in the Service Catalog tab first.</p>
          )}
        </div>

        {scorecard && (
          <div>
            <div style={{ display: "flex", gap: 16, marginBottom: 16 }}>
              <div style={{ ...cardStyle, textAlign: "center", minWidth: 120, marginBottom: 0 }}>
                <div style={{ fontSize: 36, fontWeight: 700, fontFamily: "var(--font-mono)", color: GRADE_COLORS[scorecard.overall_grade] || "var(--text-primary)" }}>
                  {scorecard.overall_grade}
                </div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Overall Grade</div>
                <div style={{ fontSize: 14, fontWeight: 600, marginTop: 4 }}>{scorecard.overall_score}/100</div>
              </div>
              <div style={{ flex: 1 }}>
                <h4 style={{ margin: "0 0 10px", fontSize: 14 }}>{scorecard.service_name} — Metrics</h4>
                {scorecard.metrics.map((metric) => (
                  <div key={metric.name} style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 6 }}>
                    <span style={{ fontSize: 12, minWidth: 180, color: "var(--text-secondary)" }}>
                      <span style={{ ...badgeStyle("var(--text-secondary)"), fontSize: 9, marginRight: 4 }}>{metric.category}</span>
                      {metric.name}
                    </span>
                    <div style={{ flex: 1, height: 8, background: "var(--bg-tertiary)", borderRadius: 4, overflow: "hidden" }}>
                      <div style={{
                        width: `${(metric.score / metric.max_score) * 100}%`,
                        height: "100%",
                        borderRadius: 4,
                        background: metric.score / metric.max_score >= 0.8 ? "var(--success-color)" : metric.score / metric.max_score >= 0.5 ? "var(--warning-color)" : "var(--error-color)",
                      }} />
                    </div>
                    <span style={{ fontSize: 11, minWidth: 50, textAlign: "right", fontFamily: "inherit" }}>
                      {metric.score}/{metric.max_score}
                    </span>
                  </div>
                ))}
              </div>
            </div>

            {scorecard.recommendations.length > 0 && (
              <div style={cardStyle}>
                <h4 style={{ margin: "0 0 8px", fontSize: 14 }}>Recommendations</h4>
                <ul style={{ margin: 0, paddingLeft: 20, fontSize: 12 }}>
                  {scorecard.recommendations.map((rec, i) => (
                    <li key={i} style={{ marginBottom: 4, color: "var(--text-secondary)" }}>{rec}</li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}
      </div>
    );
  }

  function renderInfrastructure() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Self-Service Infrastructure ({infraRequests.length} requests)</h3>
          <div style={{ display: "flex", gap: 6 }}>
            <button style={btnSecondary} onClick={loadInfraRequests}>Refresh</button>
            <button style={btnStyle} onClick={() => setShowInfraForm(!showInfraForm)}>
              {showInfraForm ? "Cancel" : "+ New Request"}
            </button>
          </div>
        </div>

        <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 12px" }}>
          Request pre-approved infrastructure resources. Requests are tracked and provisioned automatically.
        </p>

        {showInfraForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={formGroup}>
              <label style={labelStyle}>Template</label>
              <select style={inputStyle} value={infraTemplate} onChange={(e) => setInfraTemplate(e.target.value)}>
                {INFRA_TEMPLATES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Environment</label>
                <select style={inputStyle} value={infraEnv} onChange={(e) => setInfraEnv(e.target.value)}>
                  <option value="development">Development</option>
                  <option value="staging">Staging</option>
                  <option value="production">Production</option>
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Region</label>
                <select style={inputStyle} value={infraRegion} onChange={(e) => setInfraRegion(e.target.value)}>
                  {["us-east-1", "us-west-2", "eu-west-1", "eu-central-1", "ap-southeast-1"].map((r) => (
                    <option key={r} value={r}>{r}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Size</label>
                <select style={inputStyle} value={infraSize} onChange={(e) => setInfraSize(e.target.value)}>
                  <option value="small">Small</option>
                  <option value="medium">Medium</option>
                  <option value="large">Large</option>
                  <option value="xlarge">XLarge</option>
                </select>
              </div>
            </div>
            <button style={btnStyle} onClick={submitInfraRequest}>Submit Request</button>
          </div>
        )}

        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Request ID</th>
              <th style={thStyle}>Template</th>
              <th style={thStyle}>Status</th>
              <th style={thStyle}>Environment</th>
              <th style={thStyle}>Region</th>
              <th style={thStyle}>Requested By</th>
              <th style={thStyle}>Created</th>
            </tr>
          </thead>
          <tbody>
            {infraRequests.length === 0 && (
              <tr><td colSpan={7} style={{ ...tdStyle, textAlign: "center", color: "var(--text-secondary)" }}>No infrastructure requests yet. Click "+ New Request" to provision resources.</td></tr>
            )}
            {infraRequests.map((req) => (
              <tr key={req.id}>
                <td style={{ ...tdStyle, fontFamily: "inherit", fontSize: 11 }}>{req.id}</td>
                <td style={tdStyle}>{req.template}</td>
                <td style={tdStyle}><span style={badgeStyle(STATUS_COLORS[req.status] || "var(--text-secondary)")}>{req.status}</span></td>
                <td style={tdStyle}>{req.config?.environment || "—"}</td>
                <td style={tdStyle}>{req.config?.region || "—"}</td>
                <td style={tdStyle}>{req.requested_by}</td>
                <td style={tdStyle}>{req.created}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  function renderTeams() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Teams ({teams.length})</h3>
          <div style={{ display: "flex", gap: 6 }}>
            <button style={btnSecondary} onClick={loadTeams}>Refresh</button>
            <button style={btnStyle} onClick={() => setShowTeamForm(!showTeamForm)}>
              {showTeamForm ? "Cancel" : "+ Create Team"}
            </button>
          </div>
        </div>

        <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 12px" }}>
          Create teams and track their onboarding progress through an 8-step checklist. Each item can be toggled as completed.
        </p>

        {showTeamForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={formGroup}>
              <label style={labelStyle}>Team Name</label>
              <input style={inputStyle} value={teamName} onChange={(e) => setTeamName(e.target.value)} placeholder="e.g. Platform Engineering" onKeyDown={(e) => e.key === "Enter" && createTeam()} />
            </div>
            <button style={{ ...btnStyle, opacity: !teamName ? 0.5 : 1 }} onClick={createTeam} disabled={!teamName.trim()}>Create Team</button>
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))", gap: 12 }}>
          {teams.length === 0 && (
            <p style={{ color: "var(--text-secondary)", gridColumn: "1/-1", textAlign: "center" }}>No teams configured. Click "+ Create Team" to get started.</p>
          )}
          {teams.map((team) => (
            <div key={team.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
                <strong style={{ fontSize: 14 }}>{team.name}</strong>
                <button style={{ ...btnSecondary, padding: "2px 8px", fontSize: 11 }} onClick={() => setExpandedTeam(expandedTeam === team.id ? null : team.id)}>
                  {expandedTeam === team.id ? "Hide Checklist" : "Onboarding"}
                </button>
              </div>
              <div style={{ display: "flex", gap: 16, fontSize: 12, color: "var(--text-secondary)", marginBottom: 10 }}>
                <span>{team.member_count} members</span>
                <span>{team.service_count} services</span>
              </div>
              <div style={{ marginBottom: 4 }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, marginBottom: 2 }}>
                  <span style={{ color: "var(--text-secondary)" }}>Onboarding Progress</span>
                  <span style={{ fontWeight: 600, color: team.onboarding_progress === 100 ? "var(--success-color)" : "var(--text-primary)" }}>{team.onboarding_progress}%</span>
                </div>
                <div style={{ height: 6, background: "var(--bg-tertiary)", borderRadius: 3, overflow: "hidden" }}>
                  <div style={{ width: `${team.onboarding_progress}%`, height: "100%", background: team.onboarding_progress === 100 ? "var(--success-color)" : "var(--accent-color)", borderRadius: 3, transition: "width 0.3s" }} />
                </div>
              </div>

              {expandedTeam === team.id && team.onboarding_checklist.length > 0 && (
                <div style={{ marginTop: 10, borderTop: "1px solid var(--border-color)", paddingTop: 8 }}>
                  <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-secondary)" }}>Onboarding Checklist</span>
                  {team.onboarding_checklist.map((item, i) => (
                    <div key={i} role="checkbox" aria-checked={item.completed} tabIndex={0} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", fontSize: 12, cursor: "pointer" }} onClick={() => toggleChecklist(team.id, i)} onKeyDown={e => e.key === "Enter" && toggleChecklist(team.id, i)}>
                      <input type="checkbox" checked={item.completed} readOnly style={{ cursor: "pointer" }} />
                      <span style={{ textDecoration: item.completed ? "line-through" : "none", color: item.completed ? "var(--text-secondary)" : "var(--text-primary)" }}>{item.label}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    );
  }

  function renderPlatforms() {
    return (
      <div>
        <h3 style={{ margin: "0 0 8px", fontSize: 15 }}>IDP Platforms ({platforms.filter(p => p.enabled).length} enabled)</h3>
        <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 14px" }}>
          Enable and configure supported Internal Developer Platforms. Toggle platforms on/off and view their feature sets.
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 12 }}>
          {platforms.map((platform) => (
            <div key={platform.name} style={{ ...cardStyle, opacity: platform.enabled ? 1 : 0.7, borderColor: platform.enabled ? "var(--accent-color)" : "var(--border-color)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <strong style={{ fontSize: 14 }}>{platform.name}</strong>
                <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12 }}>
                  <input type="checkbox" checked={platform.enabled} onChange={() => togglePlatform(platform.name, !platform.enabled)} />
                  {platform.enabled ? "Enabled" : "Disabled"}
                </label>
              </div>
              <p style={{ margin: "0 0 8px", fontSize: 12, color: "var(--text-secondary)" }}>{platform.description}</p>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                {platform.features.map((f) => (
                  <span key={f} style={{ ...badgeStyle(platform.enabled ? "var(--success-color)" : "var(--text-secondary)"), fontSize: 10 }}>{f}</span>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  function renderBackstage() {
    return (
      <div>
        <h3 style={{ margin: "0 0 8px", fontSize: 15 }}>Backstage Integration</h3>
        <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 14px" }}>
          Generate Backstage-compatible <code>catalog-info.yaml</code> files for your registered services. These can be committed to your repos for automatic Backstage discovery.
        </p>

        <div style={cardStyle}>
          <h4 style={{ margin: "0 0 10px", fontSize: 14 }}>Generate catalog-info.yaml</h4>
          <div style={{ display: "flex", gap: 10, alignItems: "flex-end" }}>
            <div style={{ ...formGroup, flex: 1, marginBottom: 0 }}>
              <label style={labelStyle}>Select Service</label>
              <select style={inputStyle} value={backstageServiceId} onChange={(e) => setBackstageServiceId(e.target.value)}>
                <option value="">Choose a service...</option>
                {services.map((svc) => (
                  <option key={svc.id} value={svc.id}>{svc.name}</option>
                ))}
              </select>
            </div>
            <button style={btnStyle} onClick={() => {
              if (backstageServiceId) generateCatalogYaml(backstageServiceId);
            }} disabled={!backstageServiceId}>Generate</button>
          </div>
          {services.length === 0 && (
            <p style={{ margin: "10px 0 0", fontSize: 11, color: "var(--text-secondary)" }}>Register services in the Service Catalog tab first.</p>
          )}
        </div>

        {catalogYaml && (
          <div style={{ marginTop: 16 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <h4 style={{ margin: 0, fontSize: 14 }}>catalog-info.yaml</h4>
              <button style={btnSecondary} onClick={() => { navigator.clipboard.writeText(catalogYaml); showSuccess("YAML copied to clipboard."); }}>Copy</button>
            </div>
            <pre style={{
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              borderRadius: 6,
              padding: 16,
              fontSize: 12,
              fontFamily: "inherit",
              overflow: "auto",
              whiteSpace: "pre-wrap",
              maxHeight: 400,
            }}>
              {catalogYaml}
            </pre>
          </div>
        )}

        <div style={{ marginTop: 20 }}>
          <h4 style={{ margin: "0 0 10px", fontSize: 14 }}>Registered Components</h4>
          {services.length === 0 ? (
            <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No components registered. Register services in the Service Catalog tab first.</p>
          ) : (
            <table style={tableStyle}>
              <thead>
                <tr>
                  <th style={thStyle}>Component</th>
                  <th style={thStyle}>Kind</th>
                  <th style={thStyle}>Owner</th>
                  <th style={thStyle}>Lifecycle</th>
                  <th style={thStyle}>Actions</th>
                </tr>
              </thead>
              <tbody>
                {services.map((svc) => (
                  <tr key={svc.id}>
                    <td style={{ ...tdStyle, fontWeight: 500 }}>{svc.name}</td>
                    <td style={tdStyle}><span style={badgeStyle("var(--info-color)")}>Component</span></td>
                    <td style={tdStyle}>{svc.owner}</td>
                    <td style={tdStyle}>
                      <span style={badgeStyle(svc.status === "Active" ? "var(--success-color)" : svc.status === "Incubating" ? "var(--warning-color)" : "var(--text-secondary)")}>
                        {svc.status === "Active" ? "production" : svc.status === "Incubating" ? "experimental" : "deprecated"}
                      </span>
                    </td>
                    <td style={tdStyle}>
                      <button style={{ ...btnSecondary, padding: "2px 8px", fontSize: 11 }} onClick={() => {
                        setBackstageServiceId(svc.id);
                        generateCatalogInfoLocal(svc);
                      }}>Generate YAML</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    );
  }

  const renderTab = () => {
    switch (activeTab) {
      case "Service Catalog": return renderServiceCatalog();
      case "Golden Paths": return renderGoldenPaths();
      case "Scorecards": return renderScorecards();
      case "Infrastructure": return renderInfrastructure();
      case "Teams": return renderTeams();
      case "Platforms": return renderPlatforms();
      case "Backstage": return renderBackstage();
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
        {successMsg && (
          <div style={{ padding: "8px 12px", marginBottom: 12, background: "color-mix(in srgb, var(--success-color) 13%, transparent)", border: "1px solid var(--success-color)", borderRadius: 4, fontSize: 12, color: "var(--success-color)", display: "flex", justifyContent: "space-between" }}>
            <span>{successMsg}</span>
            <button style={{ background: "none", border: "none", color: "var(--success-color)", cursor: "pointer", fontSize: 14 }} onClick={() => setSuccessMsg(null)}>x</button>
          </div>
        )}
        {error && (
          <div style={{ padding: "8px 12px", marginBottom: 12, background: "color-mix(in srgb, var(--error-color) 13%, transparent)", border: "1px solid var(--error-color)", borderRadius: 4, fontSize: 12, color: "var(--error-color)", display: "flex", justifyContent: "space-between" }}>
            <span>{error}</span>
            <button style={{ background: "none", border: "none", color: "var(--error-color)", cursor: "pointer", fontSize: 14 }} onClick={() => setError(null)}>x</button>
          </div>
        )}
        {loading && <div style={{ textAlign: "center", padding: 20, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>}
        {!loading && renderTab()}
      </div>
    </div>
  );
}
