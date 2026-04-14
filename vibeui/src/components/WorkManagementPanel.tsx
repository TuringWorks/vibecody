/**
 * WorkManagementPanel — Unified project management combining enterprise work
 * management (hierarchy, work items, OKRs, risks) with Agile project management
 * (Kanban board, sprints, backlog, ceremonies, SAFe).
 *
 * Tabs: Hierarchy | Agile | Items | Board | Links | OKRs | Risks | Dashboard
 */
import { useState, useEffect, useCallback, lazy, Suspense } from "react";
import { invoke } from "@tauri-apps/api/core";

const AgilePanel = lazy(() => import("./AgilePanel"));
const DiscussionModePanel = lazy(() => import("./DiscussionModePanel"));

/* ── Types ─────────────────────────────────────────────────── */

type TabKey = "hierarchy" | "agile" | "items" | "board" | "relationships" | "okrs" | "risks" | "dashboard" | "discussions";

type WorkItemType = "initiative" | "okr" | "epic" | "feature" | "story" | "task" | "subtask" | "bug" | "risk" | "decision" | "milestone" | "spike";
type Priority = "critical" | "high" | "medium" | "low" | "none";
type RelationType = "parent" | "child" | "blocks" | "blocked_by" | "relates_to" | "duplicates" | "duplicated_by" | "implements" | "implemented_by";

const ITEM_TYPES: WorkItemType[] = ["initiative", "okr", "epic", "feature", "story", "task", "subtask", "bug", "risk", "decision", "milestone", "spike"];
const PRIORITIES: Priority[] = ["critical", "high", "medium", "low", "none"];
const REL_TYPES: RelationType[] = ["parent", "child", "blocks", "blocked_by", "relates_to", "duplicates", "implements"];

interface Org { id: string; name: string; description: string; createdAt: string; }
interface Group { id: string; orgId: string; name: string; description: string; createdAt: string; }
interface Team { id: string; groupId: string; orgId: string; name: string; description: string; createdAt: string; }
interface Workspace { id: string; teamId: string; orgId: string; name: string; description: string; prefix: string; createdAt: string; }

interface Relationship { targetId: string; type: RelationType; }

interface WorkItem {
  id: string;
  displayId: string;
  type: WorkItemType;
  title: string;
  description: string;
  status: string;
  priority: Priority;
  assignee?: string;
  labels: string[];
  storyPoints?: number;
  dueDate?: string;
  orgId: string;
  groupId?: string;
  teamId?: string;
  workspaceId?: string;
  parentId?: string;
  relationships: Relationship[];
  acceptanceCriteria?: string[];
  okrKeyResults?: { id: string; title: string; target: number; current: number }[];
  okrProgress?: number;
  riskLikelihood?: string;
  riskImpact?: string;
  riskMitigation?: string;
  decisionStatus?: string;
  decisionOutcome?: string;
  createdAt: string;
  updatedAt: string;
}

interface Scope { orgId?: string; groupId?: string; teamId?: string; workspaceId?: string; }

/* ── Styles ────────────────────────────────────────────────── */

const cardS: React.CSSProperties = { background: "var(--bg-elevated)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: 12, marginBottom: 8 };
const btnS: React.CSSProperties = { padding: "5px 12px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-elevated)", color: "var(--text-primary)", cursor: "pointer", fontSize: "var(--font-size-base)", fontWeight: 500 };
const btnP: React.CSSProperties = { ...btnS, background: "var(--accent-color)", color: "var(--btn-primary-fg)", borderColor: "var(--accent-color)" };
const inpS: React.CSSProperties = { padding: "5px 8px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: "var(--font-size-base)", width: "100%", boxSizing: "border-box" as const };
const badge = (bg: string, fg = "var(--text-primary)"): React.CSSProperties => ({ display: "inline-block", padding: "1px 6px", borderRadius: 3, fontSize: "var(--font-size-xs)", fontWeight: 600, background: bg, color: fg, marginRight: 4 });

const TYPE_COLORS: Record<string, string> = {
  initiative: "var(--accent-purple)", okr: "var(--accent-gold)", epic: "var(--accent-blue)",
  feature: "var(--accent-green)", story: "var(--accent-blue)", task: "var(--text-secondary)",
  subtask: "var(--text-secondary)", bug: "var(--error-color)", risk: "var(--warning-color)",
  decision: "var(--info-color)", milestone: "var(--accent-rose)", spike: "var(--accent-purple)",
};

const PRIORITY_COLORS: Record<string, string> = {
  critical: "var(--error-color)", high: "var(--warning-color)", medium: "var(--accent-blue)",
  low: "var(--text-secondary)", none: "var(--text-secondary)",
};

const genId = () => Math.random().toString(36).slice(2, 10);

/* ── Main Panel ────────────────────────────────────────────── */

export default function WorkManagementPanel({ provider }: { provider?: string } = {}) {
  const [tab, setTab] = useState<TabKey>("hierarchy");
  const [scope, setScope] = useState<Scope>({});
  const [orgs, setOrgs] = useState<Org[]>([]);
  const [groups, setGroups] = useState<Group[]>([]);
  const [teams, setTeams] = useState<Team[]>([]);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [items, setItems] = useState<WorkItem[]>([]);
  const [error, setError] = useState("");

  const loadOrgs = useCallback(async () => {
    try { const d = await invoke<Org[]>("wm_list_orgs"); setOrgs(d || []); } catch { /* empty */ }
  }, []);

  const loadGroups = useCallback(async () => {
    if (scope.orgId) {
      try { const d = await invoke<Group[]>("wm_list_groups", { orgId: scope.orgId }); setGroups(d || []); } catch { /* empty */ }
    } else { setGroups([]); }
  }, [scope.orgId]);

  const loadTeams = useCallback(async () => {
    if (scope.groupId) {
      try { const d = await invoke<Team[]>("wm_list_teams", { groupId: scope.groupId }); setTeams(d || []); } catch { /* empty */ }
    } else { setTeams([]); }
  }, [scope.groupId]);

  const loadWorkspaces = useCallback(async () => {
    if (scope.teamId) {
      try { const d = await invoke<Workspace[]>("wm_list_workspaces", { teamId: scope.teamId }); setWorkspaces(d || []); } catch { /* empty */ }
    } else { setWorkspaces([]); }
  }, [scope.teamId]);

  const loadItems = useCallback(async () => {
    try { const d = await invoke<WorkItem[]>("wm_list_items", { filter: scope }); setItems(d || []); } catch { /* empty */ }
  }, [scope]);

  const refreshAll = useCallback(async () => {
    await loadOrgs();
    await loadGroups();
    await loadTeams();
    await loadWorkspaces();
    await loadItems();
  }, [loadOrgs, loadGroups, loadTeams, loadWorkspaces, loadItems]);

  useEffect(() => { loadOrgs(); }, [loadOrgs]);
  useEffect(() => { loadGroups(); }, [loadGroups]);
  useEffect(() => { loadTeams(); }, [loadTeams]);
  useEffect(() => { loadWorkspaces(); }, [loadWorkspaces]);
  useEffect(() => { loadItems(); }, [loadItems]);


  const tabs: { id: TabKey; label: string }[] = [
    { id: "hierarchy", label: "Hierarchy" },
    { id: "agile", label: "Agile" },
    { id: "items", label: `Items (${items.length})` },
    { id: "board", label: "Board" },
    { id: "relationships", label: "Links" },
    { id: "okrs", label: "OKRs" },
    { id: "risks", label: "Risks" },
    { id: "dashboard", label: "Dashboard" },
    { id: "discussions", label: "Discussions" },
  ];

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <h3>Projects</h3>
        {scope.orgId && <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          {orgs.find(o => o.id === scope.orgId)?.name}
          {scope.groupId && ` / ${groups.find(g => g.id === scope.groupId)?.name || ""}`}
          {scope.teamId && ` / ${teams.find(t => t.id === scope.teamId)?.name || ""}`}
          {scope.workspaceId && ` / ${workspaces.find(w => w.id === scope.workspaceId)?.name || ""}`}
        </span>}
        {scope.orgId && <button style={{ ...btnS, fontSize: "var(--font-size-xs)", padding: "2px 6px", marginLeft: "auto" }} onClick={() => setScope({})}>Clear scope</button>}
      </div>

      {/* Tabs */}
      <div className="panel-tab-bar" style={{ padding: "0 16px" }}>
        {tabs.map(t => (
          <button key={t.id} onClick={() => setTab(t.id)} className={`panel-tab ${tab === t.id ? "active" : ""}`}>{t.label}</button>
        ))}
      </div>

      {error && <div className="panel-error">{error}<button style={{ float: "right", ...btnS, fontSize: "var(--font-size-xs)", padding: "1px 6px" }} onClick={() => setError("")}>x</button></div>}

      <div className="panel-body">
        {tab === "hierarchy" && <HierarchyTab orgs={orgs} groups={groups} teams={teams} workspaces={workspaces} scope={scope} setScope={setScope} onRefresh={refreshAll} setError={setError} />}
        {tab === "agile" && (
          <Suspense fallback={<div style={{ padding: 16, color: "var(--text-secondary)" }}>Loading Agile...</div>}>
            <AgilePanel provider={provider} />
          </Suspense>
        )}
        {tab === "items" && <ItemsTab items={items} scope={scope} onRefresh={loadItems} setError={setError} provider={provider} />}
        {tab === "board" && <BoardTab items={items} onRefresh={loadItems} setError={setError} />}
        {tab === "relationships" && <RelationshipsTab items={items} onRefresh={loadItems} setError={setError} />}
        {tab === "okrs" && <OkrTab items={items} />}
        {tab === "risks" && <RisksTab items={items} />}
        {tab === "dashboard" && <DashboardTab items={items} />}
        {tab === "discussions" && (
          <Suspense fallback={<div style={{ padding: 16, color: "var(--text-secondary)" }}>Loading Discussions...</div>}>
            <DiscussionModePanel />
          </Suspense>
        )}
      </div>
    </div>
  );
}

/* ── Hierarchy Tab ─────────────────────────────────────────── */

function HierarchyTab({ orgs, groups, teams, workspaces, scope, setScope, onRefresh, setError }: {
  orgs: Org[]; groups: Group[]; teams: Team[]; workspaces: Workspace[]; scope: Scope;
  setScope: (s: Scope) => void; onRefresh: () => void; setError: (e: string) => void;
}) {
  const [creating, setCreating] = useState<"org" | "group" | "team" | "workspace" | null>(null);
  const [editing, setEditing] = useState<{ type: "org" | "group" | "team" | "workspace"; item: any } | null>(null);
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [prefix, setPrefix] = useState("");

  const handleCreate = async () => {
    if (!name.trim()) return;
    try {
      const now = new Date().toISOString();
      if (creating === "org") {
        await invoke("wm_save_org", { org: { id: genId(), name, description: desc, createdAt: now } });
      } else if (creating === "group" && scope.orgId) {
        await invoke("wm_save_group", { group: { id: genId(), orgId: scope.orgId, name, description: desc, createdAt: now } });
      } else if (creating === "team" && scope.groupId) {
        await invoke("wm_save_team", { team: { id: genId(), groupId: scope.groupId, orgId: scope.orgId, name, description: desc, createdAt: now } });
      } else if (creating === "workspace" && scope.teamId) {
        await invoke("wm_save_workspace", { workspace: { id: genId(), teamId: scope.teamId, orgId: scope.orgId, name, description: desc, prefix: prefix.toUpperCase() || "WRK", createdAt: now } });
      }
      setCreating(null); setName(""); setDesc(""); setPrefix("");
      onRefresh();
    } catch (e: any) { setError(String(e)); }
  };

  const startEdit = (type: "org" | "group" | "team" | "workspace", item: any) => {
    setEditing({ type, item });
    setName(item.name || "");
    setDesc(item.description || "");
    setPrefix(item.prefix || "");
    setCreating(null);
  };

  const handleSaveEdit = async () => {
    if (!editing || !name.trim()) return;
    try {
      const updated = { ...editing.item, name, description: desc };
      if (editing.type === "workspace") updated.prefix = prefix.toUpperCase() || updated.prefix;
      const cmd = editing.type === "org" ? "wm_save_org" : editing.type === "group" ? "wm_save_group" : editing.type === "team" ? "wm_save_team" : "wm_save_workspace";
      const param = editing.type === "org" ? { org: updated } : editing.type === "group" ? { group: updated } : editing.type === "team" ? { team: updated } : { workspace: updated };
      await invoke(cmd, param);
      setEditing(null); setName(""); setDesc(""); setPrefix("");
      onRefresh();
    } catch (e: any) { setError(String(e)); }
  };

  const handleDelete = async (type: "org" | "group" | "team" | "workspace", id: string, name: string) => {
    if (!confirm(`Delete ${type} "${name}"? This cannot be undone.`)) return;
    try {
      const cmd = type === "org" ? "wm_delete_org" : type === "group" ? "wm_delete_group" : type === "team" ? "wm_delete_team" : "wm_delete_workspace";
      const param = type === "org" ? { orgId: id } : type === "group" ? { groupId: id } : type === "team" ? { teamId: id } : { workspaceId: id };
      await invoke(cmd, param);
      // Clear scope if the deleted item was selected
      if (type === "org" && scope.orgId === id) setScope({});
      else if (type === "group" && scope.groupId === id) setScope({ orgId: scope.orgId });
      else if (type === "team" && scope.teamId === id) setScope({ orgId: scope.orgId, groupId: scope.groupId });
      else if (type === "workspace" && scope.workspaceId === id) setScope({ orgId: scope.orgId, groupId: scope.groupId, teamId: scope.teamId });
      onRefresh();
    } catch (e: any) { setError(String(e)); }
  };

  const renderLevel = (label: string, items: { id: string; name: string; description: string }[], onSelect: (id: string) => void, selectedId?: string, createType?: "org" | "group" | "team" | "workspace") => (
    <div style={{ marginBottom: 16 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
        <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.05em", color: "var(--text-secondary)" }}>{label} ({items.length})</span>
        {createType && <button style={{ ...btnS, fontSize: "var(--font-size-xs)", padding: "2px 8px" }} onClick={() => setCreating(createType)}>+</button>}
      </div>
      {items.map(item => (
        <div key={item.id} onClick={() => onSelect(item.id)} style={{
          ...cardS, cursor: "pointer", padding: "8px 12px",
          borderLeft: selectedId === item.id ? "3px solid var(--accent-color)" : "3px solid transparent",
          background: selectedId === item.id ? "var(--accent-bg)" : "var(--bg-elevated)",
          display: "flex", alignItems: "center", gap: 8,
        }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{item.name}</div>
            {item.description && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 }}>{item.description}</div>}
          </div>
          {createType && (
            <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
              <button
                onClick={(e) => { e.stopPropagation(); startEdit(createType, item); }}
                style={{ ...btnS, fontSize: 9, padding: "2px 6px" }}
                title={`Edit ${createType}`}
              >
                Edit
              </button>
              <button
                onClick={(e) => { e.stopPropagation(); handleDelete(createType, item.id, item.name); }}
                style={{ ...btnS, fontSize: 9, padding: "2px 6px", color: "var(--error-color)", borderColor: "var(--error-color)", background: "transparent" }}
                title={`Delete ${createType}`}
              >
                Delete
              </button>
            </div>
          )}
        </div>
      ))}
      {items.length === 0 && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", padding: 8 }}>No {label.toLowerCase()} yet. Click + to create one.</div>}
    </div>
  );

  return (
    <div>
      {creating && (
        <div style={{ ...cardS, marginBottom: 12 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8, color: "var(--text-primary)" }}>Create {creating}</div>
          <input style={{ ...inpS, marginBottom: 6 }} placeholder="Name" value={name} onChange={e => setName(e.target.value)} />
          <input style={{ ...inpS, marginBottom: 6 }} placeholder="Description" value={desc} onChange={e => setDesc(e.target.value)} />
          {creating === "workspace" && <input style={{ ...inpS, marginBottom: 6 }} placeholder="ID Prefix (e.g. PROJ)" value={prefix} onChange={e => setPrefix(e.target.value)} />}
          <div style={{ display: "flex", gap: 6 }}>
            <button style={btnP} onClick={handleCreate}>Create</button>
            <button style={btnS} onClick={() => { setCreating(null); setName(""); setDesc(""); setPrefix(""); }}>Cancel</button>
          </div>
        </div>
      )}

      {editing && (
        <div style={{ ...cardS, marginBottom: 12, borderLeft: "3px solid var(--accent-color)" }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8, color: "var(--text-primary)" }}>Edit {editing.type}: {editing.item.name}</div>
          <input style={{ ...inpS, marginBottom: 6 }} placeholder="Name" value={name} onChange={e => setName(e.target.value)} />
          <input style={{ ...inpS, marginBottom: 6 }} placeholder="Description" value={desc} onChange={e => setDesc(e.target.value)} />
          {editing.type === "workspace" && <input style={{ ...inpS, marginBottom: 6 }} placeholder="ID Prefix" value={prefix} onChange={e => setPrefix(e.target.value)} />}
          <div style={{ display: "flex", gap: 6 }}>
            <button style={btnP} onClick={handleSaveEdit}>Save</button>
            <button style={btnS} onClick={() => { setEditing(null); setName(""); setDesc(""); setPrefix(""); }}>Cancel</button>
          </div>
        </div>
      )}

      {/* Getting started guide */}
      {orgs.length === 0 && !creating && (
        <div style={{ ...cardS, textAlign: "center", padding: 24, marginBottom: 16 }}>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 8 }}>Get Started with Projects</div>
          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: 1.6, marginBottom: 12 }}>
            Create an organizational hierarchy to track work items across your teams.<br />
            Organization &rarr; Group/Division &rarr; Team &rarr; Workspace/Project
          </div>
          <button style={btnP} onClick={() => setCreating("org")}>Create Your First Organization</button>
        </div>
      )}

      {/* Breadcrumb flow */}
      {orgs.length > 0 && (
        <div style={{ display: "flex", gap: 6, marginBottom: 12, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", alignItems: "center", flexWrap: "wrap" }}>
          <span style={{ fontWeight: scope.orgId ? 600 : 400, color: scope.orgId ? "var(--accent-color)" : "var(--text-secondary)", cursor: "pointer" }}
            onClick={() => setScope({})}>
            Organizations
          </span>
          {scope.orgId && <>&rarr; <span style={{ fontWeight: scope.groupId ? 600 : 400, color: scope.groupId ? "var(--accent-color)" : "var(--text-secondary)" }}>Groups</span></>}
          {scope.groupId && <>&rarr; <span style={{ fontWeight: scope.teamId ? 600 : 400, color: scope.teamId ? "var(--accent-color)" : "var(--text-secondary)" }}>Teams</span></>}
          {scope.teamId && <>&rarr; <span style={{ fontWeight: scope.workspaceId ? 600 : 400, color: scope.workspaceId ? "var(--accent-color)" : "var(--text-secondary)" }}>Workspaces</span></>}
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
        <div>
          {renderLevel("Organizations", orgs, id => setScope({ orgId: id }), scope.orgId, "org")}
          {scope.orgId && renderLevel("Groups", groups, id => setScope({ ...scope, groupId: id, teamId: undefined, workspaceId: undefined }), scope.groupId, "group")}
        </div>
        <div>
          {scope.groupId && renderLevel("Teams", teams, id => setScope({ ...scope, teamId: id, workspaceId: undefined }), scope.teamId, "team")}
          {scope.teamId && renderLevel("Workspaces", workspaces, id => setScope({ ...scope, workspaceId: id }), scope.workspaceId, "workspace")}
        </div>
      </div>

      {/* Next step hints */}
      {scope.orgId && groups.length === 0 && !creating && (
        <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 8, textAlign: "center" }}>
          Create a Group/Division within your organization to organize teams.
        </div>
      )}
      {scope.groupId && teams.length === 0 && !creating && (
        <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 8, textAlign: "center" }}>
          Create a Team within this group to start managing work.
        </div>
      )}
      {scope.teamId && workspaces.length === 0 && !creating && (
        <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 8, textAlign: "center" }}>
          Create a Workspace/Project to define ID prefixes and start tracking work items.
        </div>
      )}
      {scope.workspaceId && (
        <div style={{ fontSize: "var(--font-size-base)", color: "var(--success-color)", marginTop: 8, textAlign: "center" }}>
          Workspace selected. Switch to the Items tab to create and manage work items.
        </div>
      )}
    </div>
  );
}

/* ── Items Tab ─────────────────────────────────────────────── */

function ItemsTab({ items, scope, onRefresh, setError, provider }: {
  items: WorkItem[]; scope: Scope; onRefresh: () => void; setError: (e: string) => void; provider?: string;
}) {
  const [filterType, setFilterType] = useState<WorkItemType | "">("");
  const [filterStatus, setFilterStatus] = useState("");
  const [search, setSearch] = useState("");
  const [creating, setCreating] = useState(false);
  const [newItem, setNewItem] = useState({ type: "story" as WorkItemType, title: "", description: "", priority: "medium" as Priority, labels: "", storyPoints: 0, parentId: "" });
  const [aiBreaking, setAiBreaking] = useState<string | null>(null);
  const [editingItem, setEditingItem] = useState<WorkItem | null>(null);
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiGenerating, setAiGenerating] = useState(false);
  const [showAiPrompt, setShowAiPrompt] = useState(false);

  const filtered = items.filter(i => {
    if (filterType && i.type !== filterType) return false;
    if (filterStatus && i.status !== filterStatus) return false;
    if (search && !i.title.toLowerCase().includes(search.toLowerCase()) && !i.displayId.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  const handleCreate = async () => {
    if (!newItem.title.trim()) return;
    try {
      await invoke("wm_create_item", {
        item: {
          type: newItem.type,
          title: newItem.title,
          description: newItem.description,
          priority: newItem.priority,
          labels: newItem.labels.split(",").map(s => s.trim()).filter(Boolean),
          storyPoints: newItem.storyPoints || undefined,
          parentId: newItem.parentId || undefined,
          orgId: scope.orgId || "",
          groupId: scope.groupId,
          teamId: scope.teamId,
          workspaceId: scope.workspaceId,
          relationships: [],
        },
      });
      setCreating(false);
      setNewItem({ type: "story", title: "", description: "", priority: "medium", labels: "", storyPoints: 0, parentId: "" });
      onRefresh();
    } catch (e: any) { setError(String(e)); }
  };

  const handleAiBreakdown = async (item: WorkItem) => {
    setAiBreaking(item.displayId);
    try {
      const result = await invoke<{ items: any[] }>("wm_ai_suggest_breakdown", { item, provider });
      if (result.items?.length) {
        for (const child of result.items) {
          await invoke("wm_create_item", {
            item: { ...child, orgId: item.orgId, groupId: item.groupId, teamId: item.teamId, workspaceId: item.workspaceId, parentId: item.displayId, relationships: [] },
          });
        }
        onRefresh();
      }
    } catch (e: any) { setError(String(e)); }
    setAiBreaking(null);
  };

  const handleAiGenerate = async () => {
    if (!aiPrompt.trim()) return;
    setAiGenerating(true);
    try {
      const result = await invoke<any>("wm_ai_generate_item", {
        prompt: aiPrompt,
        itemType: newItem.type,
        provider,
      });
      setNewItem(prev => ({
        ...prev,
        title: result.title || prev.title,
        description: result.description || prev.description,
        type: (result.type as WorkItemType) || prev.type,
        priority: (result.priority as Priority) || prev.priority,
        storyPoints: result.storyPoints ?? prev.storyPoints,
        labels: Array.isArray(result.labels) ? result.labels.join(", ") : prev.labels,
      }));
      setShowAiPrompt(false);
      setAiPrompt("");
    } catch (e: any) { setError(String(e)); }
    setAiGenerating(false);
  };

  const allStatuses = [...new Set(items.map(i => i.status))];

  return (
    <div>
      {/* Create form */}
      {creating ? (
        <div style={{ ...cardS, marginBottom: 12 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Create Work Item</div>
            <button
              style={{ ...btnS, background: showAiPrompt ? "var(--accent-color)" : "var(--bg-secondary)", color: showAiPrompt ? "var(--btn-primary-fg)" : "var(--text-primary)", fontSize: "var(--font-size-sm)", padding: "3px 10px", display: "flex", alignItems: "center", gap: 4 }}
              onClick={() => setShowAiPrompt(v => !v)}
            >
              ✦ AI Generate
            </button>
          </div>

          {/* AI prompt panel */}
          {showAiPrompt && (
            <div style={{ marginBottom: 10, padding: 10, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--accent-color)" }}>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 6 }}>
                Describe what you want to build — AI will suggest a title, description, type &amp; points.
              </div>
              <div style={{ display: "flex", gap: 6 }}>
                <input
                  style={{ ...inpS, flex: 1 }}
                  placeholder="e.g. User login with OAuth, remember me option, and rate limiting..."
                  value={aiPrompt}
                  onChange={e => setAiPrompt(e.target.value)}
                  onKeyDown={e => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); handleAiGenerate(); } }}
                  autoFocus
                />
                <button
                  style={{ ...btnP, opacity: aiGenerating || !aiPrompt.trim() ? 0.5 : 1, whiteSpace: "nowrap" }}
                  onClick={handleAiGenerate}
                  disabled={aiGenerating || !aiPrompt.trim()}
                >
                  {aiGenerating ? "Generating..." : "Generate"}
                </button>
              </div>
            </div>
          )}

          <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
            <select style={{ ...inpS, width: "auto" }} value={newItem.type} onChange={e => setNewItem({ ...newItem, type: e.target.value as WorkItemType })}>
              {ITEM_TYPES.map(t => <option key={t} value={t}>{t}</option>)}
            </select>
            <select style={{ ...inpS, width: "auto" }} value={newItem.priority} onChange={e => setNewItem({ ...newItem, priority: e.target.value as Priority })}>
              {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
            </select>
            <input style={{ ...inpS, width: 60 }} type="number" min={0} placeholder="Pts" value={newItem.storyPoints || ""} onChange={e => setNewItem({ ...newItem, storyPoints: Number(e.target.value) })} />
          </div>
          <input style={{ ...inpS, marginBottom: 6 }} placeholder="Title" value={newItem.title} onChange={e => setNewItem({ ...newItem, title: e.target.value })} />
          <textarea style={{ ...inpS, marginBottom: 6, minHeight: 50, resize: "vertical" }} placeholder="Description" value={newItem.description} onChange={e => setNewItem({ ...newItem, description: e.target.value })} />
          <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
            <input style={{ ...inpS, flex: 1 }} placeholder="Labels (comma-separated)" value={newItem.labels} onChange={e => setNewItem({ ...newItem, labels: e.target.value })} />
            <input style={{ ...inpS, width: 100 }} placeholder="Parent ID" value={newItem.parentId} onChange={e => setNewItem({ ...newItem, parentId: e.target.value })} />
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <button style={btnP} onClick={handleCreate}>Create</button>
            <button style={btnS} onClick={() => { setCreating(false); setShowAiPrompt(false); setAiPrompt(""); }}>Cancel</button>
          </div>
        </div>
      ) : (
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <button style={btnP} onClick={() => setCreating(true)}>+ Create Item</button>
        </div>
      )}

      {/* Edit item form */}
      {editingItem && (
        <div style={{ ...cardS, marginBottom: 12, borderLeft: "3px solid var(--accent-color)" }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8 }}>Edit {editingItem.displayId}: {editingItem.title}</div>
          <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
            <select style={{ ...inpS, width: "auto" }} value={editingItem.type} onChange={e => setEditingItem({ ...editingItem, type: e.target.value as WorkItemType })}>
              {ITEM_TYPES.map(t => <option key={t} value={t}>{t}</option>)}
            </select>
            <select style={{ ...inpS, width: "auto" }} value={editingItem.priority} onChange={e => setEditingItem({ ...editingItem, priority: e.target.value as Priority })}>
              {PRIORITIES.map(p => <option key={p} value={p}>{p}</option>)}
            </select>
            <select style={{ ...inpS, width: "auto" }} value={editingItem.status} onChange={e => setEditingItem({ ...editingItem, status: e.target.value })}>
              {[...new Set(items.map(i => i.status)), "Backlog", "To Do", "In Progress", "In Review", "Done"].filter((v, i, a) => a.indexOf(v) === i).map(s => <option key={s} value={s}>{s}</option>)}
            </select>
            <input style={{ ...inpS, width: 60 }} type="number" min={0} value={editingItem.storyPoints || ""} onChange={e => setEditingItem({ ...editingItem, storyPoints: Number(e.target.value) })} placeholder="Pts" />
          </div>
          <input style={{ ...inpS, marginBottom: 6 }} value={editingItem.title} onChange={e => setEditingItem({ ...editingItem, title: e.target.value })} placeholder="Title" />
          <textarea style={{ ...inpS, marginBottom: 6, minHeight: 50, resize: "vertical" }} value={editingItem.description} onChange={e => setEditingItem({ ...editingItem, description: e.target.value })} placeholder="Description" />
          <input style={{ ...inpS, marginBottom: 6 }} value={editingItem.assignee || ""} onChange={e => setEditingItem({ ...editingItem, assignee: e.target.value })} placeholder="Assignee" />
          <input style={{ ...inpS, marginBottom: 6 }} value={editingItem.labels.join(", ")} onChange={e => setEditingItem({ ...editingItem, labels: e.target.value.split(",").map(s => s.trim()).filter(Boolean) })} placeholder="Labels (comma-separated)" />
          <div style={{ display: "flex", gap: 6 }}>
            <button style={btnP} onClick={async () => {
              try {
                await invoke("wm_update_item", { item: editingItem });
                setEditingItem(null);
                onRefresh();
              } catch (e: any) { setError(String(e)); }
            }}>Save</button>
            <button style={btnS} onClick={() => setEditingItem(null)}>Cancel</button>
          </div>
        </div>
      )}

      {/* Filters */}
      <div style={{ display: "flex", gap: 6, marginBottom: 12, flexWrap: "wrap", alignItems: "center" }}>
        <input style={{ ...inpS, width: 160 }} placeholder="Search by title or ID..." value={search} onChange={e => setSearch(e.target.value)} />
        <select style={{ ...inpS, width: "auto" }} value={filterType} onChange={e => setFilterType(e.target.value as WorkItemType | "")}>
          <option value="">All Types</option>
          {ITEM_TYPES.map(t => <option key={t} value={t}>{t}</option>)}
        </select>
        <select style={{ ...inpS, width: "auto" }} value={filterStatus} onChange={e => setFilterStatus(e.target.value)}>
          <option value="">All Statuses</option>
          {allStatuses.map(s => <option key={s} value={s}>{s}</option>)}
        </select>
        <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginLeft: "auto" }}>{filtered.length} items</span>
      </div>

      {/* Item list */}
      {filtered.map(item => (
        <div key={item.id} style={{ ...cardS, borderLeft: `3px solid ${TYPE_COLORS[item.type] || "var(--border-color)"}` }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", fontWeight: 700, color: "var(--accent-color)" }}>{item.displayId}</span>
            <span style={badge(TYPE_COLORS[item.type] || "var(--bg-tertiary)", "var(--btn-primary-fg)")}>{item.type}</span>
            <span style={badge(PRIORITY_COLORS[item.priority] || "var(--bg-tertiary)", "var(--btn-primary-fg)")}>{item.priority}</span>
            <span style={badge("var(--bg-tertiary)", "var(--text-secondary)")}>{item.status}</span>
            {item.storyPoints !== undefined && item.storyPoints > 0 && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{item.storyPoints} pts</span>}
            {item.parentId && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>parent: {item.parentId}</span>}
            <div style={{ flex: 1 }} />
            <button style={{ ...btnS, fontSize: "var(--font-size-xs)", padding: "2px 6px" }} onClick={() => setEditingItem(item)}>Edit</button>
            {["initiative", "epic", "feature", "story"].includes(item.type) && (
              <button style={{ ...btnS, fontSize: "var(--font-size-xs)", padding: "2px 6px" }} onClick={() => handleAiBreakdown(item)} disabled={aiBreaking === item.displayId}>
                {aiBreaking === item.displayId ? "Breaking down..." : "AI Breakdown"}
              </button>
            )}
            <button style={{ ...btnS, fontSize: "var(--font-size-xs)", padding: "2px 6px", color: "var(--error-color)", borderColor: "var(--error-color)", background: "transparent" }}
              onClick={async () => {
                if (!confirm(`Delete ${item.displayId}?`)) return;
                try { await invoke("wm_delete_item", { displayId: item.displayId }); onRefresh(); }
                catch (e: any) { setError(String(e)); }
              }}>Delete</button>
          </div>
          <div style={{ fontWeight: 500, fontSize: "var(--font-size-md)" }}>{item.title}</div>
          {item.description && <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 2 }}>{item.description.slice(0, 150)}{item.description.length > 150 ? "..." : ""}</div>}
          {item.relationships.length > 0 && (
            <div style={{ display: "flex", gap: 4, marginTop: 4, flexWrap: "wrap" }}>
              {item.relationships.map((r, i) => (
                <span key={i} style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{r.type}: <strong>{r.targetId}</strong></span>
              ))}
            </div>
          )}
          {item.labels.length > 0 && (
            <div style={{ display: "flex", gap: 3, marginTop: 4, flexWrap: "wrap" }}>
              {item.labels.map(l => <span key={l} style={badge("var(--bg-tertiary)", "var(--text-secondary)")}>{l}</span>)}
            </div>
          )}
        </div>
      ))}
      {filtered.length === 0 && (
        <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>
          {scope.orgId
            ? "No work items in this scope. Click + Create Item to get started."
            : "Select an organization from the Hierarchy tab first, then create work items here."}
          <div style={{ marginTop: 12, display: "flex", gap: 8, justifyContent: "center", flexWrap: "wrap" }}>
            {ITEM_TYPES.slice(0, 6).map(t => (
              <span key={t} style={{ ...badge(TYPE_COLORS[t] || "var(--bg-tertiary)", "var(--btn-primary-fg)"), fontSize: "var(--font-size-xs)" }}>{t}</span>
            ))}
            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>+{ITEM_TYPES.length - 6} more types</span>
          </div>
        </div>
      )}
    </div>
  );
}

/* ── Board Tab ─────────────────────────────────────────────── */

function BoardTab({ items, onRefresh, setError }: {
  items: WorkItem[]; onRefresh: () => void; setError: (e: string) => void;
}) {
  const [dragId, setDragId] = useState<string | null>(null);
  const [dragOverCol, setDragOverCol] = useState<string | null>(null);
  const [hoveredCard, setHoveredCard] = useState<string | null>(null);
  const [filterText, setFilterText] = useState("");

  // Derive ordered columns: prefer standard order, fall back to observed statuses
  const STANDARD_COLS = ["Backlog", "To Do", "In Progress", "In Review", "Done"];
  const observedStatuses = Array.from(new Set(items.map(i => i.status)));
  const columns = STANDARD_COLS.some(c => observedStatuses.includes(c))
    ? STANDARD_COLS.filter(c => observedStatuses.includes(c) || items.length === 0)
        .concat(observedStatuses.filter(s => !STANDARD_COLS.includes(s)))
    : observedStatuses.length > 0
      ? observedStatuses
      : STANDARD_COLS;

  const moveItem = async (displayId: string, newStatus: string) => {
    try {
      await invoke("wm_move_item", { displayId, status: newStatus });
      onRefresh();
    } catch (e: any) { setError(String(e)); }
  };

  const onDragStart = (e: React.DragEvent, id: string) => {
    setDragId(id);
    e.dataTransfer.effectAllowed = "move";
  };
  const onDragOver = (e: React.DragEvent, col: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    setDragOverCol(col);
  };
  const onDragLeave = () => setDragOverCol(null);
  const onDrop = async (e: React.DragEvent, col: string) => {
    e.preventDefault();
    setDragOverCol(null);
    if (!dragId) return;
    const item = items.find(i => i.id === dragId);
    if (item && item.status !== col) await moveItem(item.displayId, col);
    setDragId(null);
  };
  const onDragEnd = () => { setDragId(null); setDragOverCol(null); };

  const filtered = filterText
    ? items.filter(i => i.title.toLowerCase().includes(filterText.toLowerCase()) || i.displayId.toLowerCase().includes(filterText.toLowerCase()))
    : items;

  const colIdx = (col: string) => columns.indexOf(col);

  return (
    <div>
      {/* Toolbar */}
      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 10, padding: "6px 0", borderBottom: "1px solid var(--border-color)", flexWrap: "wrap" }}>
        <input
          className="panel-input"
          style={{ width: 160, fontSize: "var(--font-size-sm)", padding: "4px 8px" }}
          placeholder="Search items…"
          value={filterText}
          onChange={e => setFilterText(e.target.value)}
        />
        {filterText && (
          <button className="panel-btn panel-btn-secondary" style={{ padding: "3px 8px", fontSize: "var(--font-size-sm)", color: "var(--error-color)" }} onClick={() => setFilterText("")}>Clear</button>
        )}
        <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginLeft: "auto" }}>{filtered.length} item{filtered.length !== 1 ? "s" : ""}</span>
      </div>

      {/* Kanban columns */}
      <div style={{ display: "flex", gap: 12, overflowX: "auto", paddingBottom: 8 }}>
        {columns.map(col => {
          const colItems = filtered.filter(i => i.status === col);
          const isDragTarget = dragOverCol === col;
          return (
            <div
              key={col}
              onDragOver={e => onDragOver(e, col)}
              onDragLeave={onDragLeave}
              onDrop={e => onDrop(e, col)}
              style={{
                minWidth: 220, flex: 1, borderRadius: "var(--radius-md)", padding: 10,
                transition: "background 0.15s, border 0.15s",
                background: isDragTarget ? "color-mix(in srgb, var(--accent-blue) 8%, transparent)" : "var(--bg-secondary)",
                border: isDragTarget ? "2px dashed var(--accent-blue)" : "1px solid var(--border-color)",
              }}
            >
              {/* Column header */}
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>{col}</span>
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{colItems.length}</span>
              </div>

              {/* Cards */}
              {colItems.map(item => {
                const isDragging = dragId === item.id;
                return (
                  <div
                    key={item.id}
                    draggable
                    onDragStart={e => onDragStart(e, item.id)}
                    onDragEnd={onDragEnd}
                    onMouseEnter={() => setHoveredCard(item.id)}
                    onMouseLeave={() => setHoveredCard(null)}
                    style={{
                      background: "var(--bg-elevated)", border: "1px solid var(--border-color)",
                      borderRadius: "var(--radius-md)", padding: 10, marginBottom: 8,
                      cursor: "grab", opacity: isDragging ? 0.4 : 1,
                      transition: "var(--transition-fast)",
                      transform: hoveredCard === item.id && !isDragging ? "translateY(-2px)" : "none",
                      boxShadow: hoveredCard === item.id ? "var(--elevation-2)" : "var(--card-shadow)",
                    }}
                  >
                    {/* Type + ID row */}
                    <div style={{ display: "flex", gap: 4, marginBottom: 4, alignItems: "center" }}>
                      <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)", color: "var(--accent-color)" }}>{item.displayId}</span>
                      <span style={badge(PRIORITY_COLORS[item.priority] || "var(--bg-tertiary)", "var(--btn-primary-fg)")}>{item.priority}</span>
                      <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", background: "var(--bg-tertiary)", padding: "1px 5px", borderRadius: 3 }}>{item.type}</span>
                    </div>
                    <div style={{ fontWeight: 500, fontSize: "var(--font-size-md)", color: "var(--text-primary)", marginBottom: 6 }}>{item.title}</div>
                    <div style={{ display: "flex", flexWrap: "wrap", gap: 3, marginBottom: item.assignee ? 4 : 0 }}>
                      {item.storyPoints != null && item.storyPoints > 0 && (
                        <span style={badge("var(--accent-purple)", "white")}>{item.storyPoints} pts</span>
                      )}
                      {item.labels.slice(0, 3).map(l => (
                        <span key={l} style={badge("var(--bg-tertiary)", "var(--text-secondary)")}>{l}</span>
                      ))}
                    </div>
                    {item.assignee && (
                      <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 }}>{item.assignee}</div>
                    )}
                    {/* ← → move buttons */}
                    <div style={{ display: "flex", gap: 4, marginTop: 6 }}>
                      {colIdx(col) > 0 && (
                        <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => moveItem(item.displayId, columns[colIdx(col) - 1])}>&larr;</button>
                      )}
                      {colIdx(col) < columns.length - 1 && (
                        <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => moveItem(item.displayId, columns[colIdx(col) + 1])}>&rarr;</button>
                      )}
                    </div>
                  </div>
                );
              })}

              {colItems.length === 0 && (
                <div style={{ textAlign: "center", padding: "20px 8px", color: "var(--text-muted)", fontSize: "var(--font-size-sm)", opacity: 0.5 }}>Drop here</div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

/* ── Relationships Tab ─────────────────────────────────────── */

function RelationshipsTab({ items, onRefresh, setError }: { items: WorkItem[]; onRefresh: () => void; setError: (e: string) => void }) {
  const [sourceId, setSourceId] = useState("");
  const [targetId, setTargetId] = useState("");
  const [relType, setRelType] = useState<RelationType>("relates_to");

  const handleAdd = async () => {
    if (!sourceId.trim() || !targetId.trim()) return;
    try {
      await invoke("wm_add_relationship", { sourceId: sourceId.trim(), targetId: targetId.trim(), relType });
      setSourceId(""); setTargetId("");
      onRefresh();
    } catch (e: any) { setError(String(e)); }
  };

  const linked = items.filter(i => i.relationships.length > 0);

  return (
    <div>
      {/* Add relationship form */}
      <div style={{ ...cardS, marginBottom: 12 }}>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8 }}>Link Work Items</div>
        <div style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
          <input style={{ ...inpS, width: 100 }} placeholder="Source ID" value={sourceId} onChange={e => setSourceId(e.target.value)} />
          <select style={{ ...inpS, width: "auto" }} value={relType} onChange={e => setRelType(e.target.value as RelationType)}>
            {REL_TYPES.map(r => <option key={r} value={r}>{r.replace("_", " ")}</option>)}
          </select>
          <input style={{ ...inpS, width: 100 }} placeholder="Target ID" value={targetId} onChange={e => setTargetId(e.target.value)} />
          <button style={btnP} onClick={handleAdd}>Link</button>
        </div>
      </div>

      {/* Relationship list */}
      {linked.map(item => (
        <div key={item.id} style={{ ...cardS }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)", marginBottom: 4 }}>
            <span style={{ fontFamily: "var(--font-mono)", color: "var(--accent-color)" }}>{item.displayId}</span> — {item.title}
          </div>
          {item.relationships.map((r, i) => (
            <div key={i} style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", padding: "2px 0", display: "flex", alignItems: "center", gap: 6 }}>
              <span style={badge("var(--bg-tertiary)", "var(--text-secondary)")}>{r.type.replace("_", " ")}</span>
              <span style={{ fontFamily: "var(--font-mono)", fontWeight: 600, color: "var(--accent-color)" }}>{r.targetId}</span>
              <button style={{ ...btnS, fontSize: 9, padding: "1px 4px", marginLeft: "auto" }} onClick={async () => {
                try { await invoke("wm_remove_relationship", { sourceId: item.displayId, targetId: r.targetId, relType: r.type }); onRefresh(); } catch (e: any) { setError(String(e)); }
              }}>x</button>
            </div>
          ))}
        </div>
      ))}
      {linked.length === 0 && <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>No linked items yet. Use the form above to create relationships.</div>}
    </div>
  );
}

/* ── OKR Tab ───────────────────────────────────────────────── */

function OkrTab({ items }: { items: WorkItem[] }) {
  const okrs = items.filter(i => i.type === "okr");
  return (
    <div>
      {okrs.map(okr => (
        <div key={okr.id} style={{ ...cardS }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", color: "var(--accent-color)" }}>{okr.displayId}</span>
            <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{okr.title}</span>
            <span style={badge(okr.status === "Achieved" ? "var(--success-color)" : okr.status === "At Risk" ? "var(--warning-color)" : "var(--bg-tertiary)", "var(--btn-primary-fg)")}>{okr.status}</span>
          </div>
          {okr.description && <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>{okr.description}</div>}

          {/* Key Results */}
          {(okr.okrKeyResults || []).map(kr => {
            const pct = kr.target > 0 ? Math.round((kr.current / kr.target) * 100) : 0;
            return (
              <div key={kr.id} style={{ marginBottom: 6 }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-base)", marginBottom: 2 }}>
                  <span>{kr.title}</span>
                  <span style={{ fontWeight: 600, color: pct >= 100 ? "var(--success-color)" : pct >= 70 ? "var(--accent-color)" : "var(--warning-color)" }}>{pct}%</span>
                </div>
                <div style={{ height: 6, background: "var(--bg-tertiary)", borderRadius: 3, overflow: "hidden" }}>
                  <div style={{ width: `${Math.min(pct, 100)}%`, height: "100%", background: pct >= 100 ? "var(--success-color)" : "var(--accent-color)", borderRadius: 3, transition: "width 0.3s" }} />
                </div>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 1 }}>{kr.current} / {kr.target}</div>
              </div>
            );
          })}

          {/* Linked items */}
          {okr.relationships.filter(r => r.type === "implemented_by" || r.type === "child").length > 0 && (
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>
              Linked: {okr.relationships.filter(r => r.type === "implemented_by" || r.type === "child").map(r => r.targetId).join(", ")}
            </div>
          )}
        </div>
      ))}
      {okrs.length === 0 && <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>No OKRs. Create one from the Items tab with type "okr".</div>}
    </div>
  );
}

/* ── Risks Tab ─────────────────────────────────────────────── */

function RisksTab({ items }: { items: WorkItem[] }) {
  const risks = items.filter(i => i.type === "risk");
  const decisions = items.filter(i => i.type === "decision");

  const riskMatrix: Record<string, Record<string, WorkItem[]>> = { high: { high: [], medium: [], low: [] }, medium: { high: [], medium: [], low: [] }, low: { high: [], medium: [], low: [] } };
  risks.forEach(r => {
    const l = r.riskLikelihood || "medium";
    const imp = r.riskImpact || "medium";
    if (riskMatrix[l]?.[imp]) riskMatrix[l][imp].push(r);
  });

  const matrixColor = (l: string, i: string) => {
    if (l === "high" && i === "high") return "var(--error-bg)";
    if ((l === "high" && i === "medium") || (l === "medium" && i === "high")) return "var(--warning-bg)";
    return "var(--bg-secondary)";
  };

  return (
    <div>
      {/* Risk Matrix */}
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8 }}>Risk Matrix</div>
      <table style={{ borderCollapse: "collapse", marginBottom: 16, width: "100%" }}>
        <thead>
          <tr>
            <th style={{ padding: 6, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}></th>
            {["high", "medium", "low"].map(i => <th key={i} style={{ padding: 6, fontSize: "var(--font-size-xs)", textTransform: "uppercase", color: "var(--text-secondary)" }}>Impact: {i}</th>)}
          </tr>
        </thead>
        <tbody>
          {(["high", "medium", "low"] as const).map(l => (
            <tr key={l}>
              <td style={{ padding: 6, fontSize: "var(--font-size-xs)", textTransform: "uppercase", color: "var(--text-secondary)", fontWeight: 600 }}>Likelihood: {l}</td>
              {(["high", "medium", "low"] as const).map(i => (
                <td key={i} style={{ padding: 6, background: matrixColor(l, i), border: "1px solid var(--border-color)", textAlign: "center", minWidth: 80 }}>
                  {riskMatrix[l][i].map(r => (
                    <div key={r.id} style={{ fontSize: "var(--font-size-xs)", marginBottom: 2 }}>
                      <span style={{ fontFamily: "var(--font-mono)", color: "var(--accent-color)" }}>{r.displayId}</span>
                    </div>
                  ))}
                  {riskMatrix[l][i].length === 0 && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>-</span>}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>

      {/* Risk list */}
      {risks.map(r => (
        <div key={r.id} style={{ ...cardS, borderLeft: "3px solid var(--warning-color)" }}>
          <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 4 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", color: "var(--accent-color)" }}>{r.displayId}</span>
            <span style={badge("var(--warning-bg)", "var(--warning-color)")}>{r.status}</span>
            {r.riskLikelihood && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>L:{r.riskLikelihood}</span>}
            {r.riskImpact && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>I:{r.riskImpact}</span>}
          </div>
          <div style={{ fontWeight: 500, fontSize: "var(--font-size-md)" }}>{r.title}</div>
          {r.riskMitigation && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>Mitigation: {r.riskMitigation}</div>}
        </div>
      ))}

      {/* Decision Log */}
      {decisions.length > 0 && (
        <div style={{ marginTop: 16 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8 }}>Decision Log</div>
          {decisions.map(d => (
            <div key={d.id} style={{ ...cardS, borderLeft: "3px solid var(--info-color)" }}>
              <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 4 }}>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", color: "var(--accent-color)" }}>{d.displayId}</span>
                <span style={badge(d.status === "Accepted" ? "var(--success-bg)" : "var(--bg-tertiary)", d.status === "Accepted" ? "var(--success-color)" : "var(--text-secondary)")}>{d.status}</span>
              </div>
              <div style={{ fontWeight: 500, fontSize: "var(--font-size-md)" }}>{d.title}</div>
              {d.decisionOutcome && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>Outcome: {d.decisionOutcome}</div>}
            </div>
          ))}
        </div>
      )}

      {risks.length === 0 && decisions.length === 0 && <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>No risks or decisions tracked yet.</div>}
    </div>
  );
}

/* ── Dashboard Tab ─────────────────────────────────────────── */

function DashboardTab({ items }: { items: WorkItem[] }) {
  const byType = new Map<string, number>();
  const byStatus = new Map<string, number>();
  const byPriority = new Map<string, number>();
  items.forEach(i => {
    byType.set(i.type, (byType.get(i.type) || 0) + 1);
    byStatus.set(i.status, (byStatus.get(i.status) || 0) + 1);
    byPriority.set(i.priority, (byPriority.get(i.priority) || 0) + 1);
  });

  const totalPoints = items.reduce((sum, i) => sum + (i.storyPoints || 0), 0);
  const doneItems = items.filter(i => ["Done", "Closed", "Achieved", "Resolved", "Reached", "Verified"].includes(i.status));
  const okrs = items.filter(i => i.type === "okr");
  const avgOkrProgress = okrs.length > 0 ? Math.round(okrs.reduce((s, o) => s + (o.okrProgress || 0), 0) / okrs.length) : 0;

  const renderBar = (label: string, count: number, total: number, color: string) => (
    <div key={label} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
      <span style={{ fontSize: "var(--font-size-sm)", width: 80, textAlign: "right", color: "var(--text-secondary)" }}>{label}</span>
      <div style={{ flex: 1, height: 14, background: "var(--bg-tertiary)", borderRadius: 3, overflow: "hidden" }}>
        <div style={{ width: total > 0 ? `${(count / total) * 100}%` : "0%", height: "100%", background: color, borderRadius: 3 }} />
      </div>
      <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, width: 30, color: "var(--text-primary)" }}>{count}</span>
    </div>
  );

  return (
    <div>
      {/* Summary cards */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(120px, 1fr))", gap: 8, marginBottom: 16 }}>
        {[
          { label: "Total Items", value: items.length },
          { label: "Done", value: doneItems.length },
          { label: "Story Points", value: totalPoints },
          { label: "OKR Progress", value: `${avgOkrProgress}%` },
          { label: "Open Risks", value: items.filter(i => i.type === "risk" && !["Resolved", "Accepted"].includes(i.status)).length },
          { label: "Open Bugs", value: items.filter(i => i.type === "bug" && !["Closed", "Wont Fix", "Verified"].includes(i.status)).length },
        ].map(({ label, value }) => (
          <div key={label} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: "10px 12px", border: "1px solid var(--border-color)" }}>
            <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.05em" }}>{label}</div>
            <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{value}</div>
          </div>
        ))}
      </div>

      {/* By type */}
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8 }}>By Type</div>
      {[...byType.entries()].sort((a, b) => b[1] - a[1]).map(([type, count]) =>
        renderBar(type, count, items.length, TYPE_COLORS[type] || "var(--accent-color)")
      )}

      {/* By status */}
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginTop: 16, marginBottom: 8 }}>By Status</div>
      {[...byStatus.entries()].sort((a, b) => b[1] - a[1]).map(([status, count]) =>
        renderBar(status, count, items.length, "var(--accent-color)")
      )}

      {/* By priority */}
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginTop: 16, marginBottom: 8 }}>By Priority</div>
      {[...byPriority.entries()].sort((a, b) => PRIORITIES.indexOf(a[0] as Priority) - PRIORITIES.indexOf(b[0] as Priority)).map(([pri, count]) =>
        renderBar(pri, count, items.length, PRIORITY_COLORS[pri] || "var(--accent-color)")
      )}
    </div>
  );
}
