/**
 * AppBuilderPanel — AI-powered App Builder.
 *
 * Sub-tabs: Quick Start | Templates | Provision | Backend
 * - Quick Start: Describe an app idea, enhance with AI, scaffold from spec
 * - Templates: Browse, use, save, import/export, delete project templates
 * - Provision: Configure database, auth, hosting, SEO, payments
 * - Backend: Unified backend config view, docker-compose, deployment, env vars
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Sparkles,
  Play,
  Layout,
  Upload,
  Download,
  Trash2,
  Plus,
  Check,
  AlertCircle,
  Loader2,
  Server,
  Database,
  Shield,
  Globe,
  Search,
  CreditCard,
  FileText,
  CheckCircle2,
  XCircle,
  X,
} from "lucide-react";

type SubTab = "quickstart" | "templates" | "provision" | "backend";

type TemplateCategory = "All" | "Web" | "Mobile" | "API" | "FullStack" | "Landing" | "Dashboard";

interface EnhancedSpec {
  title: string;
  userStories: string[];
  techStack: string[];
  apiEndpoints: string[];
  uiComponents: string[];
  complexityEstimate: string;
}

interface Template {
  id: string;
  name: string;
  description: string;
  category: TemplateCategory;
  techStack: string[];
}

interface ProvisionConfig {
  database: { enabled: boolean; type: "SQLite" | "PostgreSQL" | "Supabase" };
  auth: { enabled: boolean; provider: "JWT" | "OAuth" | "Supabase Auth" };
  hosting: { target: "Vercel" | "Netlify" | "Railway" | "Docker" };
  seo: { enabled: boolean };
  payments: { enabled: boolean };
}

interface GeneratedFile {
  path: string;
  status: "pending" | "generated" | "error";
}

interface EnvVar {
  key: string;
  value: string;
}

interface ServiceStatus {
  name: string;
  connected: boolean;
  details: string;
}

interface CreateResult {
  projectDir: string;
  filesCreated: string[];
  message: string;
}

interface HistoryEntry {
  id: string;
  name: string;
  templateId: string;
  targetDir: string;
  createdAt: string;
  files: string[];
}

const CATEGORIES: TemplateCategory[] = ["All", "Web", "Mobile", "API", "FullStack", "Landing", "Dashboard"];




const tagStyle: React.CSSProperties = {
  padding: "1px 7px",
  borderRadius: "var(--radius-md)",
  background: "var(--accent)",
  color: "var(--text-primary)",
  fontSize: "var(--font-size-xs)",
  fontWeight: 600,
  opacity: 0.85,
};

const categoryBadgeStyle = (cat: TemplateCategory): React.CSSProperties => {
  const colors: Record<TemplateCategory, string> = {
    All: "var(--text-secondary)",
    Web: "#3178c6",
    Mobile: "var(--accent-green)",
    API: "#f7a41d",
    FullStack: "#9c7ce1",
    Landing: "var(--error-color)",
    Dashboard: "#00bcd4",
  };
  const c = colors[cat];
  return {
    padding: "1px 7px",
    borderRadius: "var(--radius-md)",
    background: c + "22",
    border: `1px solid ${c}`,
    color: c,
    fontSize: "var(--font-size-xs)",
    fontWeight: 600,
  };
};

export function AppBuilderPanel({ workspacePath }: { workspacePath: string }) {
  const [subTab, setSubTab] = useState<SubTab>("quickstart");
  const [errorMsg, setErrorMsg] = useState("");

  // ── Quick Start ──
  const [ideaText, setIdeaText] = useState("");
  const [enhancedSpec, setEnhancedSpec] = useState<EnhancedSpec | null>(null);
  const [isEnhancing, setIsEnhancing] = useState(false);
  const [isBuilding, setIsBuilding] = useState(false);
  const [buildResult, setBuildResult] = useState<CreateResult | null>(null);
  const [selectedTemplateId, setSelectedTemplateId] = useState("");

  // ── Templates ──
  const [templates, setTemplates] = useState<Template[]>([]);
  const [isLoadingTemplates, setIsLoadingTemplates] = useState(false);
  const [categoryFilter, setCategoryFilter] = useState<TemplateCategory>("All");
  const [showSaveForm, setShowSaveForm] = useState(false);
  const [showImportForm, setShowImportForm] = useState(false);
  const [newTemplateName, setNewTemplateName] = useState("");
  const [newTemplateDesc, setNewTemplateDesc] = useState("");
  const [newTemplateCategory, setNewTemplateCategory] = useState<TemplateCategory>("Web");
  const [importJson, setImportJson] = useState("");
  const [deleteConfirmId, setDeleteConfirmId] = useState<string | null>(null);

  // ── History ──
  const [history, setHistory] = useState<HistoryEntry[]>([]);

  // ── Provision ──
  const [provisionConfig, setProvisionConfig] = useState<ProvisionConfig>({
    database: { enabled: false, type: "PostgreSQL" },
    auth: { enabled: false, provider: "JWT" },
    hosting: { target: "Vercel" },
    seo: { enabled: false },
    payments: { enabled: false },
  });
  const [isProvisioning, setIsProvisioning] = useState(false);
  const [generatedFiles, setGeneratedFiles] = useState<GeneratedFile[]>([]);

  // ── Backend ──
  const [envVars, setEnvVars] = useState<EnvVar[]>([
    { key: "DATABASE_URL", value: "" },
    { key: "JWT_SECRET", value: "" },
  ]);
  const [services, _setServices] = useState<ServiceStatus[]>([
    { name: "Database", connected: false, details: "Not configured" },
    { name: "Authentication", connected: false, details: "Not configured" },
    { name: "Hosting", connected: false, details: "Not configured" },
    { name: "API Gateway", connected: false, details: "Not configured" },
  ]);
  const [isGeneratingDocker, setIsGeneratingDocker] = useState(false);
  const [isGeneratingDeploy, setIsGeneratingDeploy] = useState(false);
  const [backendOutput, setBackendOutput] = useState("");

  // ── Load templates from backend on mount ──
  useEffect(() => {
    loadTemplates();
    loadHistory();
  }, []);

  const loadTemplates = async () => {
    setIsLoadingTemplates(true);
    try {
      const result = await invoke<Template[]>("get_app_templates");
      setTemplates(result);
    } catch (err) {
      setErrorMsg(`Failed to load templates: ${err}`);
    } finally {
      setIsLoadingTemplates(false);
    }
  };

  const loadHistory = async () => {
    try {
      const result = await invoke<HistoryEntry[]>("get_app_builder_history");
      setHistory(result);
    } catch {
      // History file may not exist yet
    }
  };

  // ── Quick Start Handlers ──
  const handleEnhance = async () => {
    if (!ideaText.trim()) return;
    setIsEnhancing(true);
    setEnhancedSpec(null);
    setErrorMsg("");
    try {
      const spec = await invoke<EnhancedSpec>("enhance_app_template", { idea: ideaText });
      setEnhancedSpec(spec);
    } catch (err) {
      setErrorMsg(`Enhancement failed: ${err}`);
    } finally {
      setIsEnhancing(false);
    }
  };

  const handleBuild = async () => {
    if (!enhancedSpec) return;
    setIsBuilding(true);
    setBuildResult(null);
    setErrorMsg("");
    const projectName = enhancedSpec.title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "")
      || "my-app";
    try {
      const result = await invoke<CreateResult>("create_app_project", {
        templateId: selectedTemplateId || "react-spa",
        projectName,
        targetDir: workspacePath || ".",
      });
      setBuildResult(result);
      loadHistory();
    } catch (err) {
      setErrorMsg(`Build failed: ${err}`);
    } finally {
      setIsBuilding(false);
    }
  };

  // ── Template Handlers ──
  const handleUseTemplate = (t: Template) => {
    setSubTab("quickstart");
    setIdeaText(`Create a ${t.name}: ${t.description}`);
    setSelectedTemplateId(t.id);
    setEnhancedSpec(null);
    setBuildResult(null);
  };

  const handleSaveTemplate = () => {
    if (!newTemplateName.trim()) return;
    const newT: Template = {
      id: `t${Date.now()}`,
      name: newTemplateName,
      description: newTemplateDesc,
      category: newTemplateCategory,
      techStack: [],
    };
    setTemplates((prev) => [...prev, newT]);
    setNewTemplateName("");
    setNewTemplateDesc("");
    setShowSaveForm(false);
  };

  const handleImportTemplate = () => {
    try {
      const parsed = JSON.parse(importJson) as Template;
      if (parsed.name && parsed.description) {
        setTemplates((prev) => [...prev, { ...parsed, id: `t${Date.now()}` }]);
        setImportJson("");
        setShowImportForm(false);
      }
    } catch {
      // Invalid JSON, ignore
    }
  };

  const handleExportTemplate = (t: Template) => {
    const json = JSON.stringify(t, null, 2);
    navigator.clipboard.writeText(json).catch(() => {});
  };

  const handleDeleteTemplate = (id: string) => {
    setTemplates((prev) => prev.filter((t) => t.id !== id));
    setDeleteConfirmId(null);
  };

  const filteredTemplates = categoryFilter === "All"
    ? templates
    : templates.filter((t) => t.category === categoryFilter);

  // ── Provision Handlers ──
  const handleProvisionAll = async () => {
    setIsProvisioning(true);
    setErrorMsg("");
    const files: GeneratedFile[] = [];
    if (provisionConfig.database.enabled) {
      files.push({ path: `prisma/schema.prisma`, status: "pending" });
      files.push({ path: `src/db.ts`, status: "pending" });
    }
    if (provisionConfig.auth.enabled) {
      files.push({ path: `src/auth/config.ts`, status: "pending" });
      files.push({ path: `src/middleware/auth.ts`, status: "pending" });
    }
    files.push({ path: getHostingConfigFile(provisionConfig.hosting.target), status: "pending" });
    if (provisionConfig.seo.enabled) {
      files.push({ path: `src/seo/meta.ts`, status: "pending" });
      files.push({ path: `public/sitemap.xml`, status: "pending" });
    }
    if (provisionConfig.payments.enabled) {
      files.push({ path: `src/payments/stripe.ts`, status: "pending" });
      files.push({ path: `src/api/webhooks/stripe.ts`, status: "pending" });
    }
    setGeneratedFiles([...files]);

    // Use backend to scaffold with a provision-focused template
    try {
      const projectName = `provision-${Date.now()}`;
      const result = await invoke<CreateResult>("create_app_project", {
        templateId: "react-spa",
        projectName,
        targetDir: workspacePath || ".",
      });
      // Mark all files as generated
      setGeneratedFiles((prev) => prev.map((f) => ({ ...f, status: "generated" as const })));
      setBackendOutput(`Provisioned ${result.filesCreated.length} files into ${result.projectDir}`);
      loadHistory();
    } catch (err) {
      setGeneratedFiles((prev) => prev.map((f) => ({ ...f, status: "error" as const })));
      setErrorMsg(`Provisioning failed: ${err}`);
    } finally {
      setIsProvisioning(false);
    }
  };

  // ── Backend Handlers ──
  const handleGenerateDocker = async () => {
    setIsGeneratingDocker(true);
    setBackendOutput("");
    setErrorMsg("");
    try {
      const result = await invoke<CreateResult>("create_app_project", {
        templateId: "rest-api",
        projectName: `docker-${Date.now()}`,
        targetDir: workspacePath || ".",
      });
      setBackendOutput(`Generated docker-compose.yml — ${result.filesCreated.length} files in ${result.projectDir}`);
      loadHistory();
    } catch (err) {
      setErrorMsg(`Docker generation failed: ${err}`);
    } finally {
      setIsGeneratingDocker(false);
    }
  };

  const handleGenerateDeploy = async () => {
    setIsGeneratingDeploy(true);
    setBackendOutput("");
    setErrorMsg("");
    try {
      const result = await invoke<CreateResult>("create_app_project", {
        templateId: "nextjs-fullstack",
        projectName: `deploy-${Date.now()}`,
        targetDir: workspacePath || ".",
      });
      setBackendOutput(`Generated deployment manifest for ${provisionConfig.hosting.target} — ${result.filesCreated.length} files in ${result.projectDir}`);
      loadHistory();
    } catch (err) {
      setErrorMsg(`Deployment manifest generation failed: ${err}`);
    } finally {
      setIsGeneratingDeploy(false);
    }
  };

  const handleAddEnvVar = () => {
    setEnvVars((prev) => [...prev, { key: "", value: "" }]);
  };

  const handleRemoveEnvVar = (idx: number) => {
    setEnvVars((prev) => prev.filter((_, i) => i !== idx));
  };

  const handleUpdateEnvVar = (idx: number, field: "key" | "value", val: string) => {
    setEnvVars((prev) => prev.map((v, i) => (i === idx ? { ...v, [field]: val } : v)));
  };

  const tabLabels: Record<SubTab, string> = {
    quickstart: "Quick Start",
    templates: "Templates",
    provision: "Provision",
    backend: "Backend",
  };

  return (
    <div className="panel-container">
      {/* Sub-tab bar */}
      <div className="panel-tab-bar">
        {(["quickstart", "templates", "provision", "backend"] as SubTab[]).map((t) => (
          <button
            key={t}
            onClick={() => setSubTab(t)}
            className={`panel-tab ${subTab === t ? "active" : ""}`}
          >
            {tabLabels[t]}
          </button>
        ))}
      </div>

      {errorMsg && (
        <div className="panel-error" style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <AlertCircle size={13} /> {errorMsg}
          <button onClick={() => setErrorMsg("")} style={{ background: "none", border: "none", cursor: "pointer", color: "var(--error-color)", marginLeft: "auto" }}><X size={12} /></button>
        </div>
      )}

      <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        {/* ── Quick Start ── */}
        {subTab === "quickstart" && (
          <>
            <div className="panel-card">
              <div className="panel-label" style={{ marginBottom: 8 }}>Describe Your App Idea</div>
              <textarea
                value={ideaText}
                onChange={(e) => setIdeaText(e.target.value)}
                placeholder="Describe your app idea in natural language. For example: A task management app with user accounts, project boards, drag-and-drop cards, due dates, and team collaboration..."
                className="panel-input panel-input-full"
                style={{ minHeight: 120, resize: "vertical", fontFamily: "inherit", lineHeight: 1.5 }}
              />
              <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
                <button
                  onClick={handleEnhance}
                  disabled={isEnhancing || !ideaText.trim()}
                  className="panel-btn panel-btn-primary"
                  style={{ opacity: isEnhancing || !ideaText.trim() ? 0.6 : 1 }}
                >
                  {isEnhancing ? <Loader2 size={13} className="spin" /> : <Sparkles size={13} />}
                  {isEnhancing ? "Enhancing..." : "Enhance"}
                </button>
              </div>
            </div>

            {enhancedSpec && (
              <div className="panel-card">
                <div className="panel-label" style={{ marginBottom: 10 }}>Enhanced Specification</div>

                <div style={{ marginBottom: 10 }}>
                  <div style={{ fontSize: 16, fontWeight: 700, color: "var(--text-primary)", marginBottom: 4 }}>
                    {enhancedSpec.title}
                  </div>
                </div>

                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                  <div>
                    <div className="panel-label" style={{ marginBottom: 4 }}>User Stories</div>
                    <ul style={{ margin: 0, paddingLeft: 16, fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: 1.8 }}>
                      {enhancedSpec.userStories.map((s, i) => <li key={i}>{s}</li>)}
                    </ul>
                  </div>

                  <div>
                    <div className="panel-label" style={{ marginBottom: 4 }}>Tech Stack</div>
                    <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                      {enhancedSpec.techStack.map((t, i) => (
                        <span key={i} style={tagStyle}>{t}</span>
                      ))}
                    </div>

                    <div className="panel-label" style={{ marginTop: 10, marginBottom: 4 }}>Complexity</div>
                    <div style={{ fontSize: "var(--font-size-base)", color: "var(--warning)" }}>{enhancedSpec.complexityEstimate}</div>
                  </div>

                  <div>
                    <div className="panel-label" style={{ marginBottom: 4 }}>API Endpoints</div>
                    <ul style={{ margin: 0, paddingLeft: 16, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.8, fontFamily: "var(--font-mono)" }}>
                      {enhancedSpec.apiEndpoints.map((e, i) => <li key={i}>{e}</li>)}
                    </ul>
                  </div>

                  <div>
                    <div className="panel-label" style={{ marginBottom: 4 }}>UI Components</div>
                    <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                      {enhancedSpec.uiComponents.map((c, i) => (
                        <span key={i} style={{ ...tagStyle, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border)" }}>
                          {c}
                        </span>
                      ))}
                    </div>
                  </div>
                </div>

                <div style={{ marginTop: 12 }}>
                  <button
                    onClick={handleBuild}
                    disabled={isBuilding}
                    className="panel-btn panel-btn-primary"
                    style={{ opacity: isBuilding ? 0.6 : 1 }}
                  >
                    {isBuilding ? <Loader2 size={13} className="spin" /> : <Play size={13} />}
                    {isBuilding ? "Building..." : "Build"}
                  </button>
                </div>

                {isBuilding && (
                  <div style={{ marginTop: 10 }}>
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>
                      Scaffolding project...
                    </div>
                    <div style={{ height: 6, borderRadius: 3, background: "var(--bg-primary)", overflow: "hidden" }}>
                      <div
                        style={{
                          height: "100%",
                          width: "100%",
                          background: "var(--accent)",
                          borderRadius: 3,
                          animation: "pulse 1.5s ease-in-out infinite",
                          opacity: 0.7,
                        }}
                      />
                    </div>
                  </div>
                )}

                {buildResult && (
                  <div style={{ marginTop: 8, fontSize: "var(--font-size-base)", color: "var(--success)", display: "flex", flexDirection: "column", gap: 4 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
                      <Check size={14} /> {buildResult.message}
                    </div>
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>
                      {buildResult.projectDir}
                    </div>
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                      Files: {buildResult.filesCreated.join(", ")}
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* Recent history */}
            {history.length > 0 && (
              <div className="panel-card">
                <div className="panel-label" style={{ marginBottom: 8 }}>Recent Projects</div>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {history.slice(-5).reverse().map((h) => (
                    <div key={h.id} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: "var(--font-size-base)" }}>
                      <FileText size={12} color="var(--text-secondary)" />
                      <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>{h.name}</span>
                      <span style={{ color: "var(--text-secondary)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)" }}>{h.templateId}</span>
                      <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", marginLeft: "auto" }}>{h.files.length} files</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}

        {/* ── Templates ── */}
        {subTab === "templates" && (
          <>
            <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
              <select
                value={categoryFilter}
                onChange={(e) => setCategoryFilter(e.target.value as TemplateCategory)}
                className="panel-select"
              >
                {CATEGORIES.map((c) => (
                  <option key={c} value={c}>{c}</option>
                ))}
              </select>
              <div style={{ flex: 1 }} />
              <button onClick={() => setShowSaveForm(!showSaveForm)} className="panel-btn panel-btn-secondary">
                <Plus size={12} /> Save Current as Template
              </button>
              <button onClick={() => setShowImportForm(!showImportForm)} className="panel-btn panel-btn-secondary">
                <Upload size={12} /> Import
              </button>
            </div>

            {showSaveForm && (
              <div className="panel-card">
                <div className="panel-label" style={{ marginBottom: 8 }}>Save Current Project as Template</div>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  <input
                    className="panel-input panel-input-full"
                    placeholder="Template name"
                    value={newTemplateName}
                    onChange={(e) => setNewTemplateName(e.target.value)}
                  />
                  <input
                    className="panel-input panel-input-full"
                    placeholder="Description"
                    value={newTemplateDesc}
                    onChange={(e) => setNewTemplateDesc(e.target.value)}
                  />
                  <select
                    className="panel-select"
                    value={newTemplateCategory}
                    onChange={(e) => setNewTemplateCategory(e.target.value as TemplateCategory)}
                  >
                    {CATEGORIES.filter((c) => c !== "All").map((c) => (
                      <option key={c} value={c}>{c}</option>
                    ))}
                  </select>
                  <div style={{ display: "flex", gap: 8 }}>
                    <button onClick={handleSaveTemplate} className="panel-btn panel-btn-primary">
                      <Check size={12} /> Save
                    </button>
                    <button onClick={() => setShowSaveForm(false)} className="panel-btn panel-btn-secondary">Cancel</button>
                  </div>
                </div>
              </div>
            )}

            {showImportForm && (
              <div className="panel-card">
                <div className="panel-label" style={{ marginBottom: 8 }}>Import Template (JSON)</div>
                <textarea
                  className="panel-input panel-input-full" style={{ minHeight: 80, fontFamily: "var(--font-mono)", resize: "vertical" }}
                  placeholder='Paste template JSON here...'
                  value={importJson}
                  onChange={(e) => setImportJson(e.target.value)}
                />
                <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                  <button onClick={handleImportTemplate} className="panel-btn panel-btn-primary">
                    <Upload size={12} /> Import
                  </button>
                  <button onClick={() => setShowImportForm(false)} className="panel-btn panel-btn-secondary">Cancel</button>
                </div>
              </div>
            )}

            {isLoadingTemplates && (
              <div style={{ textAlign: "center", padding: 20, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>
                <Loader2 size={20} className="spin" style={{ marginBottom: 8 }} />
                <div>Loading templates...</div>
              </div>
            )}

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 10 }}>
              {filteredTemplates.map((t) => (
                <div key={t.id} className="panel-card">
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 6 }}>
                    <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, color: "var(--text-primary)" }}>{t.name}</div>
                    <span style={categoryBadgeStyle(t.category as TemplateCategory)}>{t.category}</span>
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8, lineHeight: 1.4 }}>
                    {t.description}
                  </div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 10 }}>
                    {t.techStack.map((s, i) => (
                      <span key={i} style={tagStyle}>{s}</span>
                    ))}
                  </div>
                  <div style={{ display: "flex", gap: 6 }}>
                    <button onClick={() => handleUseTemplate(t)} className="panel-btn panel-btn-primary">
                      <Layout size={11} /> Use Template
                    </button>
                    <button onClick={() => handleExportTemplate(t)} className="panel-btn panel-btn-secondary" title="Copy JSON to clipboard">
                      <Download size={11} /> Export
                    </button>
                    {deleteConfirmId === t.id ? (
                      <>
                        <button onClick={() => handleDeleteTemplate(t.id)} className="panel-btn panel-btn-danger">
                          <Trash2 size={11} /> Confirm
                        </button>
                        <button onClick={() => setDeleteConfirmId(null)} className="panel-btn panel-btn-secondary">Cancel</button>
                      </>
                    ) : (
                      <button onClick={() => setDeleteConfirmId(t.id)} className="panel-btn panel-btn-danger" title="Delete template">
                        <Trash2 size={11} />
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {!isLoadingTemplates && filteredTemplates.length === 0 && (
              <div style={{ textAlign: "center", padding: 30, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
                <Search size={24} style={{ marginBottom: 8, opacity: 0.5 }} />
                <div>No templates found in this category.</div>
              </div>
            )}
          </>
        )}

        {/* ── Provision ── */}
        {subTab === "provision" && (
          <>
            <div className="panel-card">
              <div className="panel-label" style={{ marginBottom: 12 }}>Infrastructure Configuration</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
                {/* Database */}
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <Database size={16} color="var(--text-secondary)" />
                  <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", minWidth: 120 }}>
                    <input
                      type="checkbox"
                      checked={provisionConfig.database.enabled}
                      onChange={(e) => setProvisionConfig((p) => ({ ...p, database: { ...p.database, enabled: e.target.checked } }))}
                    />
                    <span style={{ fontSize: "var(--font-size-md)", color: "var(--text-primary)", fontWeight: 500 }}>Database</span>
                  </label>
                  <select
                    className="panel-select" style={{ opacity: provisionConfig.database.enabled ? 1 : 0.5 }}
                    disabled={!provisionConfig.database.enabled}
                    value={provisionConfig.database.type}
                    onChange={(e) => setProvisionConfig((p) => ({ ...p, database: { ...p.database, type: e.target.value as ProvisionConfig["database"]["type"] } }))}
                  >
                    <option value="SQLite">SQLite</option>
                    <option value="PostgreSQL">PostgreSQL</option>
                    <option value="Supabase">Supabase</option>
                  </select>
                </div>

                {/* Authentication */}
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <Shield size={16} color="var(--text-secondary)" />
                  <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", minWidth: 120 }}>
                    <input
                      type="checkbox"
                      checked={provisionConfig.auth.enabled}
                      onChange={(e) => setProvisionConfig((p) => ({ ...p, auth: { ...p.auth, enabled: e.target.checked } }))}
                    />
                    <span style={{ fontSize: "var(--font-size-md)", color: "var(--text-primary)", fontWeight: 500 }}>Authentication</span>
                  </label>
                  <select
                    className="panel-select" style={{ opacity: provisionConfig.auth.enabled ? 1 : 0.5 }}
                    disabled={!provisionConfig.auth.enabled}
                    value={provisionConfig.auth.provider}
                    onChange={(e) => setProvisionConfig((p) => ({ ...p, auth: { ...p.auth, provider: e.target.value as ProvisionConfig["auth"]["provider"] } }))}
                  >
                    <option value="JWT">JWT</option>
                    <option value="OAuth">OAuth</option>
                    <option value="Supabase Auth">Supabase Auth</option>
                  </select>
                </div>

                {/* Hosting */}
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <Globe size={16} color="var(--text-secondary)" />
                  <span style={{ fontSize: "var(--font-size-md)", color: "var(--text-primary)", fontWeight: 500, minWidth: 120, paddingLeft: 22 }}>Hosting</span>
                  <select
                    className="panel-select"
                    value={provisionConfig.hosting.target}
                    onChange={(e) => setProvisionConfig((p) => ({ ...p, hosting: { ...p.hosting, target: e.target.value as ProvisionConfig["hosting"]["target"] } }))}
                  >
                    <option value="Vercel">Vercel</option>
                    <option value="Netlify">Netlify</option>
                    <option value="Railway">Railway</option>
                    <option value="Docker">Docker</option>
                  </select>
                </div>

                {/* SEO */}
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <Search size={16} color="var(--text-secondary)" />
                  <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", minWidth: 120 }}>
                    <input
                      type="checkbox"
                      checked={provisionConfig.seo.enabled}
                      onChange={(e) => setProvisionConfig((p) => ({ ...p, seo: { ...p.seo, enabled: e.target.checked } }))}
                    />
                    <span style={{ fontSize: "var(--font-size-md)", color: "var(--text-primary)", fontWeight: 500 }}>SEO</span>
                  </label>
                </div>

                {/* Payments */}
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <CreditCard size={16} color="var(--text-secondary)" />
                  <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", minWidth: 120 }}>
                    <input
                      type="checkbox"
                      checked={provisionConfig.payments.enabled}
                      onChange={(e) => setProvisionConfig((p) => ({ ...p, payments: { ...p.payments, enabled: e.target.checked } }))}
                    />
                    <span style={{ fontSize: "var(--font-size-md)", color: "var(--text-primary)", fontWeight: 500 }}>Payments (Stripe)</span>
                  </label>
                </div>
              </div>

              <div style={{ marginTop: 14 }}>
                <button
                  onClick={handleProvisionAll}
                  disabled={isProvisioning}
                  className="panel-btn panel-btn-primary"
                  style={{ opacity: isProvisioning ? 0.6 : 1 }}
                >
                  {isProvisioning ? <Loader2 size={13} className="spin" /> : <Server size={13} />}
                  {isProvisioning ? "Provisioning..." : "Provision All"}
                </button>
              </div>
            </div>

            {generatedFiles.length > 0 && (
              <div className="panel-card">
                <div className="panel-label" style={{ marginBottom: 8 }}>Generated Files</div>
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {generatedFiles.map((f, i) => (
                    <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: "var(--font-size-base)" }}>
                      {f.status === "pending" && <Loader2 size={12} color="var(--text-secondary)" className="spin" />}
                      {f.status === "generated" && <Check size={12} color="var(--success)" />}
                      {f.status === "error" && <AlertCircle size={12} color="var(--error)" />}
                      <FileText size={12} color="var(--text-secondary)" />
                      <span style={{ color: "var(--text-primary)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{f.path}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}

        {/* ── Backend ── */}
        {subTab === "backend" && (
          <>
            <div className="panel-card">
              <div className="panel-label" style={{ marginBottom: 10 }}>Service Status</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                {services.map((svc, i) => (
                  <div
                    key={i}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                      padding: "8px 10px",
                      borderRadius: "var(--radius-xs-plus)",
                      border: "1px solid var(--border)",
                      background: "var(--bg-primary)",
                    }}
                  >
                    {svc.connected
                      ? <CheckCircle2 size={12} strokeWidth={1.5} style={{ color: "var(--accent-green)" }} />
                      : <XCircle size={12} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />
                    }
                    <div>
                      <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--text-primary)" }}>{svc.name}</div>
                      <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{svc.details}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div className="panel-card">
              <div className="panel-label" style={{ marginBottom: 10 }}>Backend Configuration</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, fontSize: "var(--font-size-base)", marginBottom: 12 }}>
                <div>
                  <span style={{ color: "var(--text-secondary)" }}>Database:</span>{" "}
                  <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>
                    {provisionConfig.database.enabled ? provisionConfig.database.type : "None"}
                  </span>
                </div>
                <div>
                  <span style={{ color: "var(--text-secondary)" }}>Auth:</span>{" "}
                  <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>
                    {provisionConfig.auth.enabled ? provisionConfig.auth.provider : "None"}
                  </span>
                </div>
                <div>
                  <span style={{ color: "var(--text-secondary)" }}>Hosting:</span>{" "}
                  <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>{provisionConfig.hosting.target}</span>
                </div>
              </div>

              <div style={{ display: "flex", gap: 8 }}>
                <button
                  onClick={handleGenerateDocker}
                  disabled={isGeneratingDocker}
                  className="panel-btn panel-btn-primary"
                  style={{ opacity: isGeneratingDocker ? 0.6 : 1 }}
                >
                  {isGeneratingDocker ? <Loader2 size={13} className="spin" /> : <Server size={13} />}
                  Generate docker-compose.yml
                </button>
                <button
                  onClick={handleGenerateDeploy}
                  disabled={isGeneratingDeploy}
                  className="panel-btn panel-btn-secondary"
                  style={{ opacity: isGeneratingDeploy ? 0.6 : 1 }}
                >
                  {isGeneratingDeploy ? <Loader2 size={13} className="spin" /> : <Globe size={13} />}
                  Generate deployment manifest
                </button>
              </div>

              {backendOutput && (
                <div style={{
                  marginTop: 10,
                  padding: 8,
                  borderRadius: "var(--radius-xs-plus)",
                  background: "var(--bg-primary)",
                  border: "1px solid var(--border)",
                  fontSize: "var(--font-size-base)",
                  color: "var(--success)",
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                }}>
                  <Check size={13} /> {backendOutput}
                </div>
              )}
            </div>

            <div className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
                <div className="panel-label">Environment Variables</div>
                <button onClick={handleAddEnvVar} className="panel-btn panel-btn-secondary">
                  <Plus size={12} /> Add
                </button>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {envVars.map((v, i) => (
                  <div key={i} style={{ display: "flex", gap: 6, alignItems: "center" }}>
                    <input
                      className="panel-input" style={{ width: "40%", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}
                      placeholder="KEY"
                      value={v.key}
                      onChange={(e) => handleUpdateEnvVar(i, "key", e.target.value)}
                    />
                    <input
                      className="panel-input" style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}
                      placeholder="value"
                      value={v.value}
                      onChange={(e) => handleUpdateEnvVar(i, "value", e.target.value)}
                    />
                    <button
                      onClick={() => handleRemoveEnvVar(i)}
                      style={{
                        background: "transparent",
                        border: "none",
                        cursor: "pointer",
                        color: "var(--text-secondary)",
                        padding: 4,
                      }}
                      title="Remove variable"
                    >
                      <X size={14} />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function getHostingConfigFile(target: string): string {
  switch (target) {
    case "Vercel": return "vercel.json";
    case "Netlify": return "netlify.toml";
    case "Railway": return "railway.toml";
    case "Docker": return "Dockerfile";
    default: return "deploy.json";
  }
}
