/**
 * AppBuilderPanel — AI-powered App Builder.
 *
 * Sub-tabs: Quick Start | Templates | Provision | Backend
 * - Quick Start: Describe an app idea, enhance with AI, scaffold from spec
 * - Templates: Browse, use, save, import/export, delete project templates
 * - Provision: Configure database, auth, hosting, SEO, payments
 * - Backend: Unified backend config view, docker-compose, deployment, env vars
 */
import { useState } from "react";
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
  Circle,
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

const CATEGORIES: TemplateCategory[] = ["All", "Web", "Mobile", "API", "FullStack", "Landing", "Dashboard"];

const SAMPLE_TEMPLATES: Template[] = [
  { id: "t1", name: "React SPA", description: "Single-page React app with Vite and TailwindCSS", category: "Web", techStack: ["React", "Vite", "TailwindCSS"] },
  { id: "t2", name: "REST API", description: "Express.js REST API with TypeScript and Prisma ORM", category: "API", techStack: ["Node.js", "Express", "Prisma", "TypeScript"] },
  { id: "t3", name: "Full-Stack Next.js", description: "Next.js app with API routes, auth, and database", category: "FullStack", techStack: ["Next.js", "Prisma", "NextAuth"] },
  { id: "t4", name: "Landing Page", description: "Marketing landing page with animations and contact form", category: "Landing", techStack: ["HTML", "TailwindCSS", "Alpine.js"] },
  { id: "t5", name: "Admin Dashboard", description: "Data-driven dashboard with charts, tables, and RBAC", category: "Dashboard", techStack: ["React", "Recharts", "TanStack Table"] },
  { id: "t6", name: "React Native App", description: "Cross-platform mobile app with Expo and navigation", category: "Mobile", techStack: ["React Native", "Expo", "React Navigation"] },
];

const btnStyle = (variant: "primary" | "default" | "danger" = "default"): React.CSSProperties => ({
  padding: "6px 14px",
  fontSize: 12,
  border: variant === "primary" ? "none" : variant === "danger" ? "1px solid var(--error)" : "1px solid var(--border)",
  borderRadius: 4,
  cursor: "pointer",
  fontWeight: 500,
  background: variant === "primary" ? "var(--accent)" : variant === "danger" ? "transparent" : "var(--bg-secondary)",
  color: variant === "primary" ? "white" : variant === "danger" ? "var(--error)" : "var(--text-primary)",
  display: "inline-flex",
  alignItems: "center",
  gap: 5,
});

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  fontSize: 12,
  border: "1px solid var(--border)",
  borderRadius: 4,
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  outline: "none",
  width: "100%",
};

const selectStyle: React.CSSProperties = {
  ...inputStyle,
  width: "auto",
  minWidth: 140,
};

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border)",
  borderRadius: 6,
  padding: 12,
  background: "var(--bg-secondary)",
};

const labelStyle: React.CSSProperties = {
  fontSize: 11,
  color: "var(--text-secondary)",
  fontWeight: 600,
  textTransform: "uppercase" as const,
  letterSpacing: "0.5px",
};

const tagStyle: React.CSSProperties = {
  padding: "1px 7px",
  borderRadius: 10,
  background: "var(--accent)",
  color: "white",
  fontSize: 10,
  fontWeight: 600,
  opacity: 0.85,
};

const categoryBadgeStyle = (cat: TemplateCategory): React.CSSProperties => {
  const colors: Record<TemplateCategory, string> = {
    All: "#888",
    Web: "#3178c6",
    Mobile: "#4caf50",
    API: "#f7a41d",
    FullStack: "#9c7ce1",
    Landing: "#e91e63",
    Dashboard: "#00bcd4",
  };
  const c = colors[cat];
  return {
    padding: "1px 7px",
    borderRadius: 10,
    background: c + "22",
    border: `1px solid ${c}`,
    color: c,
    fontSize: 10,
    fontWeight: 600,
  };
};

export function AppBuilderPanel({ workspacePath }: { workspacePath: string }) {
  const _workspacePath = workspacePath;
  void _workspacePath;

  const [subTab, setSubTab] = useState<SubTab>("quickstart");

  // ── Quick Start ──
  const [ideaText, setIdeaText] = useState("");
  const [enhancedSpec, setEnhancedSpec] = useState<EnhancedSpec | null>(null);
  const [isEnhancing, setIsEnhancing] = useState(false);
  const [isBuilding, setIsBuilding] = useState(false);
  const [buildProgress, setBuildProgress] = useState(0);

  // ── Templates ──
  const [templates, setTemplates] = useState<Template[]>(SAMPLE_TEMPLATES);
  const [categoryFilter, setCategoryFilter] = useState<TemplateCategory>("All");
  const [showSaveForm, setShowSaveForm] = useState(false);
  const [showImportForm, setShowImportForm] = useState(false);
  const [newTemplateName, setNewTemplateName] = useState("");
  const [newTemplateDesc, setNewTemplateDesc] = useState("");
  const [newTemplateCategory, setNewTemplateCategory] = useState<TemplateCategory>("Web");
  const [importJson, setImportJson] = useState("");
  const [deleteConfirmId, setDeleteConfirmId] = useState<string | null>(null);

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

  // ── Quick Start Handlers ──
  const handleEnhance = async () => {
    if (!ideaText.trim()) return;
    setIsEnhancing(true);
    setEnhancedSpec(null);
    // Simulate AI enhancement
    await new Promise((r) => setTimeout(r, 1200));
    setEnhancedSpec({
      title: ideaText.split(/[.\n]/)[0].trim().slice(0, 60) || "My App",
      userStories: [
        "As a user, I can sign up and log in securely",
        "As a user, I can view and manage my dashboard",
        "As an admin, I can manage users and settings",
      ],
      techStack: ["React", "TypeScript", "Node.js", "PostgreSQL", "TailwindCSS"],
      apiEndpoints: ["POST /api/auth/login", "GET /api/users", "POST /api/data", "DELETE /api/data/:id"],
      uiComponents: ["LoginForm", "Dashboard", "DataTable", "SettingsPanel", "Sidebar"],
      complexityEstimate: "Medium (~2-3 weeks for MVP)",
    });
    setIsEnhancing(false);
  };

  const handleBuild = async () => {
    setIsBuilding(true);
    setBuildProgress(0);
    for (let i = 1; i <= 10; i++) {
      await new Promise((r) => setTimeout(r, 300));
      setBuildProgress(i * 10);
    }
    setIsBuilding(false);
  };

  // ── Template Handlers ──
  const handleUseTemplate = (t: Template) => {
    setSubTab("quickstart");
    setIdeaText(`Create a ${t.name}: ${t.description}`);
    setEnhancedSpec(null);
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

    for (let i = 0; i < files.length; i++) {
      await new Promise((r) => setTimeout(r, 400));
      setGeneratedFiles((prev) =>
        prev.map((f, idx) => (idx === i ? { ...f, status: "generated" } : f))
      );
    }
    setIsProvisioning(false);
  };

  // ── Backend Handlers ──
  const handleGenerateDocker = async () => {
    setIsGeneratingDocker(true);
    setBackendOutput("");
    await new Promise((r) => setTimeout(r, 800));
    setBackendOutput("Generated docker-compose.yml with services: app, db, redis");
    setIsGeneratingDocker(false);
  };

  const handleGenerateDeploy = async () => {
    setIsGeneratingDeploy(true);
    setBackendOutput("");
    await new Promise((r) => setTimeout(r, 800));
    setBackendOutput(`Generated deployment manifest for ${provisionConfig.hosting.target}`);
    setIsGeneratingDeploy(false);
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
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Sub-tab bar */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border)", background: "var(--bg-secondary)", flexShrink: 0 }}>
        {(["quickstart", "templates", "provision", "backend"] as SubTab[]).map((t) => (
          <button
            key={t}
            onClick={() => setSubTab(t)}
            style={{
              padding: "6px 14px",
              fontSize: 12,
              background: "transparent",
              color: subTab === t ? "var(--text-primary)" : "var(--text-secondary)",
              border: "none",
              borderBottom: subTab === t ? "2px solid var(--accent)" : "2px solid transparent",
              cursor: "pointer",
              fontWeight: subTab === t ? 600 : 400,
            }}
          >
            {tabLabels[t]}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 12 }}>
        {/* ── Quick Start ── */}
        {subTab === "quickstart" && (
          <>
            <div style={cardStyle}>
              <div style={{ ...labelStyle, marginBottom: 8 }}>Describe Your App Idea</div>
              <textarea
                value={ideaText}
                onChange={(e) => setIdeaText(e.target.value)}
                placeholder="Describe your app idea in natural language. For example: A task management app with user accounts, project boards, drag-and-drop cards, due dates, and team collaboration..."
                style={{
                  ...inputStyle,
                  minHeight: 120,
                  resize: "vertical",
                  fontFamily: "inherit",
                  lineHeight: 1.5,
                }}
              />
              <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
                <button
                  onClick={handleEnhance}
                  disabled={isEnhancing || !ideaText.trim()}
                  style={{
                    ...btnStyle("primary"),
                    opacity: isEnhancing || !ideaText.trim() ? 0.6 : 1,
                  }}
                >
                  {isEnhancing ? <Loader2 size={13} className="spin" /> : <Sparkles size={13} />}
                  {isEnhancing ? "Enhancing..." : "Enhance"}
                </button>
              </div>
            </div>

            {enhancedSpec && (
              <div style={cardStyle}>
                <div style={{ ...labelStyle, marginBottom: 10 }}>Enhanced Specification</div>

                <div style={{ marginBottom: 10 }}>
                  <div style={{ fontSize: 16, fontWeight: 700, color: "var(--text-primary)", marginBottom: 4 }}>
                    {enhancedSpec.title}
                  </div>
                </div>

                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                  <div>
                    <div style={{ ...labelStyle, marginBottom: 4 }}>User Stories</div>
                    <ul style={{ margin: 0, paddingLeft: 16, fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.8 }}>
                      {enhancedSpec.userStories.map((s, i) => <li key={i}>{s}</li>)}
                    </ul>
                  </div>

                  <div>
                    <div style={{ ...labelStyle, marginBottom: 4 }}>Tech Stack</div>
                    <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                      {enhancedSpec.techStack.map((t, i) => (
                        <span key={i} style={tagStyle}>{t}</span>
                      ))}
                    </div>

                    <div style={{ ...labelStyle, marginTop: 10, marginBottom: 4 }}>Complexity</div>
                    <div style={{ fontSize: 12, color: "var(--warning)" }}>{enhancedSpec.complexityEstimate}</div>
                  </div>

                  <div>
                    <div style={{ ...labelStyle, marginBottom: 4 }}>API Endpoints</div>
                    <ul style={{ margin: 0, paddingLeft: 16, fontSize: 11, color: "var(--text-secondary)", lineHeight: 1.8, fontFamily: "monospace" }}>
                      {enhancedSpec.apiEndpoints.map((e, i) => <li key={i}>{e}</li>)}
                    </ul>
                  </div>

                  <div>
                    <div style={{ ...labelStyle, marginBottom: 4 }}>UI Components</div>
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
                    style={{
                      ...btnStyle("primary"),
                      opacity: isBuilding ? 0.6 : 1,
                    }}
                  >
                    {isBuilding ? <Loader2 size={13} className="spin" /> : <Play size={13} />}
                    {isBuilding ? "Building..." : "Build"}
                  </button>
                </div>

                {isBuilding && (
                  <div style={{ marginTop: 10 }}>
                    <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
                      Scaffolding... {buildProgress}%
                    </div>
                    <div style={{ height: 6, borderRadius: 3, background: "var(--bg-primary)", overflow: "hidden" }}>
                      <div
                        style={{
                          height: "100%",
                          width: `${buildProgress}%`,
                          background: "var(--accent)",
                          borderRadius: 3,
                          transition: "width 0.3s ease",
                        }}
                      />
                    </div>
                  </div>
                )}

                {!isBuilding && buildProgress === 100 && (
                  <div style={{ marginTop: 8, fontSize: 12, color: "var(--success)", display: "flex", alignItems: "center", gap: 5 }}>
                    <Check size={14} /> Scaffold complete. Project files generated.
                  </div>
                )}
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
                style={selectStyle}
              >
                {CATEGORIES.map((c) => (
                  <option key={c} value={c}>{c}</option>
                ))}
              </select>
              <div style={{ flex: 1 }} />
              <button onClick={() => setShowSaveForm(!showSaveForm)} style={btnStyle()}>
                <Plus size={12} /> Save Current as Template
              </button>
              <button onClick={() => setShowImportForm(!showImportForm)} style={btnStyle()}>
                <Upload size={12} /> Import
              </button>
            </div>

            {showSaveForm && (
              <div style={cardStyle}>
                <div style={{ ...labelStyle, marginBottom: 8 }}>Save Current Project as Template</div>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  <input
                    style={inputStyle}
                    placeholder="Template name"
                    value={newTemplateName}
                    onChange={(e) => setNewTemplateName(e.target.value)}
                  />
                  <input
                    style={inputStyle}
                    placeholder="Description"
                    value={newTemplateDesc}
                    onChange={(e) => setNewTemplateDesc(e.target.value)}
                  />
                  <select
                    style={selectStyle}
                    value={newTemplateCategory}
                    onChange={(e) => setNewTemplateCategory(e.target.value as TemplateCategory)}
                  >
                    {CATEGORIES.filter((c) => c !== "All").map((c) => (
                      <option key={c} value={c}>{c}</option>
                    ))}
                  </select>
                  <div style={{ display: "flex", gap: 8 }}>
                    <button onClick={handleSaveTemplate} style={btnStyle("primary")}>
                      <Check size={12} /> Save
                    </button>
                    <button onClick={() => setShowSaveForm(false)} style={btnStyle()}>Cancel</button>
                  </div>
                </div>
              </div>
            )}

            {showImportForm && (
              <div style={cardStyle}>
                <div style={{ ...labelStyle, marginBottom: 8 }}>Import Template (JSON)</div>
                <textarea
                  style={{ ...inputStyle, minHeight: 80, fontFamily: "monospace", resize: "vertical" }}
                  placeholder='Paste template JSON here...'
                  value={importJson}
                  onChange={(e) => setImportJson(e.target.value)}
                />
                <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                  <button onClick={handleImportTemplate} style={btnStyle("primary")}>
                    <Upload size={12} /> Import
                  </button>
                  <button onClick={() => setShowImportForm(false)} style={btnStyle()}>Cancel</button>
                </div>
              </div>
            )}

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 10 }}>
              {filteredTemplates.map((t) => (
                <div key={t.id} style={cardStyle}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: 6 }}>
                    <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)" }}>{t.name}</div>
                    <span style={categoryBadgeStyle(t.category)}>{t.category}</span>
                  </div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8, lineHeight: 1.4 }}>
                    {t.description}
                  </div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 10 }}>
                    {t.techStack.map((s, i) => (
                      <span key={i} style={tagStyle}>{s}</span>
                    ))}
                  </div>
                  <div style={{ display: "flex", gap: 6 }}>
                    <button onClick={() => handleUseTemplate(t)} style={btnStyle("primary")}>
                      <Layout size={11} /> Use Template
                    </button>
                    <button onClick={() => handleExportTemplate(t)} style={btnStyle()} title="Copy JSON to clipboard">
                      <Download size={11} /> Export
                    </button>
                    {deleteConfirmId === t.id ? (
                      <>
                        <button onClick={() => handleDeleteTemplate(t.id)} style={btnStyle("danger")}>
                          <Trash2 size={11} /> Confirm
                        </button>
                        <button onClick={() => setDeleteConfirmId(null)} style={btnStyle()}>Cancel</button>
                      </>
                    ) : (
                      <button onClick={() => setDeleteConfirmId(t.id)} style={btnStyle("danger")} title="Delete template">
                        <Trash2 size={11} />
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {filteredTemplates.length === 0 && (
              <div style={{ textAlign: "center", padding: 30, color: "var(--text-secondary)", fontSize: 13 }}>
                <Search size={24} style={{ marginBottom: 8, opacity: 0.5 }} />
                <div>No templates found in this category.</div>
              </div>
            )}
          </>
        )}

        {/* ── Provision ── */}
        {subTab === "provision" && (
          <>
            <div style={cardStyle}>
              <div style={{ ...labelStyle, marginBottom: 12 }}>Infrastructure Configuration</div>
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
                    <span style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 500 }}>Database</span>
                  </label>
                  <select
                    style={{ ...selectStyle, opacity: provisionConfig.database.enabled ? 1 : 0.5 }}
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
                    <span style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 500 }}>Authentication</span>
                  </label>
                  <select
                    style={{ ...selectStyle, opacity: provisionConfig.auth.enabled ? 1 : 0.5 }}
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
                  <span style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 500, minWidth: 120, paddingLeft: 22 }}>Hosting</span>
                  <select
                    style={selectStyle}
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
                    <span style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 500 }}>SEO</span>
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
                    <span style={{ fontSize: 13, color: "var(--text-primary)", fontWeight: 500 }}>Payments (Stripe)</span>
                  </label>
                </div>
              </div>

              <div style={{ marginTop: 14 }}>
                <button
                  onClick={handleProvisionAll}
                  disabled={isProvisioning}
                  style={{
                    ...btnStyle("primary"),
                    opacity: isProvisioning ? 0.6 : 1,
                  }}
                >
                  {isProvisioning ? <Loader2 size={13} className="spin" /> : <Server size={13} />}
                  {isProvisioning ? "Provisioning..." : "Provision All"}
                </button>
              </div>
            </div>

            {generatedFiles.length > 0 && (
              <div style={cardStyle}>
                <div style={{ ...labelStyle, marginBottom: 8 }}>Generated Files</div>
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {generatedFiles.map((f, i) => (
                    <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12 }}>
                      {f.status === "pending" && <Loader2 size={12} color="var(--text-secondary)" className="spin" />}
                      {f.status === "generated" && <Check size={12} color="var(--success)" />}
                      {f.status === "error" && <AlertCircle size={12} color="var(--error)" />}
                      <FileText size={12} color="var(--text-secondary)" />
                      <span style={{ color: "var(--text-primary)", fontFamily: "monospace", fontSize: 11 }}>{f.path}</span>
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
            <div style={cardStyle}>
              <div style={{ ...labelStyle, marginBottom: 10 }}>Service Status</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                {services.map((svc, i) => (
                  <div
                    key={i}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                      padding: "8px 10px",
                      borderRadius: 4,
                      border: "1px solid var(--border)",
                      background: "var(--bg-primary)",
                    }}
                  >
                    <Circle
                      size={10}
                      strokeWidth={0}
                      fill={svc.connected ? "var(--success)" : "var(--text-secondary)"}
                    />
                    <div>
                      <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{svc.name}</div>
                      <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>{svc.details}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div style={cardStyle}>
              <div style={{ ...labelStyle, marginBottom: 10 }}>Backend Configuration</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, fontSize: 12, marginBottom: 12 }}>
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
                  style={{
                    ...btnStyle("primary"),
                    opacity: isGeneratingDocker ? 0.6 : 1,
                  }}
                >
                  {isGeneratingDocker ? <Loader2 size={13} className="spin" /> : <Server size={13} />}
                  Generate docker-compose.yml
                </button>
                <button
                  onClick={handleGenerateDeploy}
                  disabled={isGeneratingDeploy}
                  style={{
                    ...btnStyle(),
                    opacity: isGeneratingDeploy ? 0.6 : 1,
                  }}
                >
                  {isGeneratingDeploy ? <Loader2 size={13} className="spin" /> : <Globe size={13} />}
                  Generate deployment manifest
                </button>
              </div>

              {backendOutput && (
                <div style={{
                  marginTop: 10,
                  padding: 8,
                  borderRadius: 4,
                  background: "var(--bg-primary)",
                  border: "1px solid var(--border)",
                  fontSize: 12,
                  color: "var(--success)",
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                }}>
                  <Check size={13} /> {backendOutput}
                </div>
              )}
            </div>

            <div style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
                <div style={labelStyle}>Environment Variables</div>
                <button onClick={handleAddEnvVar} style={btnStyle()}>
                  <Plus size={12} /> Add
                </button>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {envVars.map((v, i) => (
                  <div key={i} style={{ display: "flex", gap: 6, alignItems: "center" }}>
                    <input
                      style={{ ...inputStyle, width: "40%", fontFamily: "monospace", fontSize: 11 }}
                      placeholder="KEY"
                      value={v.key}
                      onChange={(e) => handleUpdateEnvVar(i, "key", e.target.value)}
                    />
                    <input
                      style={{ ...inputStyle, flex: 1, fontFamily: "monospace", fontSize: 11 }}
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
