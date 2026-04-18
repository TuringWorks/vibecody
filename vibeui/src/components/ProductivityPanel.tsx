/**
 * ProductivityPanel — Email, Calendar, Tasks, Notion, Jira, Smart Home
 *
 * Tabs: Email | Calendar | Tasks | Notion | Jira | Home
 *
 * Each tab shows a command palette and output area, backed by Tauri invoke
 * calls to the corresponding vibecli handler (email_client, calendar_client,
 * productivity, home_assistant).
 *
 * Icons: Lucide React — all use currentColor so they respond to CSS theme vars.
 */
import React, { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  // Tab icons
  Mail,
  Calendar,
  ListTodo,
  BookOpen,
  Ticket,
  Home,
  // Email quick actions
  MailOpen,
  Inbox,
  Bot,
  Tag,
  // Calendar quick actions
  CalendarDays,
  CalendarRange,
  SkipForward,
  CalendarCheck,
  // Tasks quick actions
  List,
  // Notion quick actions
  Database,
  Search,
  // Jira quick actions
  User,
  Layers,
  Zap,
  // Home quick actions
  LayoutDashboard,
  Lightbulb,
  Sunset,
  Focus,
  // Action buttons
  Trash2,
  Play,
  Loader2,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

/* ── Shared types ─────────────────────────────────────────────────────────── */
type Tab = "email" | "calendar" | "tasks" | "notion" | "jira" | "home";

interface OutputLine {
  id: number;
  text: string;
  ts: string;
}

/* ── Icon helper ─────────────────────────────────────────────────────────── */
function Ico({
  icon: Icon,
  size = 13,
  style,
}: {
  icon: LucideIcon;
  size?: number;
  style?: React.CSSProperties;
}) {
  return (
    <Icon
      size={size}
      color="currentColor"
      strokeWidth={1.75}
      style={{ flexShrink: 0, ...style }}
    />
  );
}

/* ── Tab definitions ─────────────────────────────────────────────────────── */
const TABS: { key: Tab; label: string; icon: LucideIcon }[] = [
  { key: "email",    label: "Email",    icon: Mail },
  { key: "calendar", label: "Calendar", icon: Calendar },
  { key: "tasks",    label: "Tasks",    icon: ListTodo },
  { key: "notion",   label: "Notion",   icon: BookOpen },
  { key: "jira",     label: "Jira",     icon: Ticket },
  { key: "home",     label: "Home",     icon: Home },
];

/* ── Quick command sets per tab ──────────────────────────────────────────── */
const QUICK: Record<Tab, { label: string; cmd: string; icon: LucideIcon }[]> = {
  email: [
    { label: "Unread",  cmd: "unread",  icon: MailOpen },
    { label: "Inbox",   cmd: "inbox",   icon: Inbox },
    { label: "Triage",  cmd: "triage",  icon: Bot },
    { label: "Labels",  cmd: "labels",  icon: Tag },
  ],
  calendar: [
    { label: "Today",      cmd: "today",      icon: CalendarDays },
    { label: "Week",       cmd: "week",        icon: CalendarRange },
    { label: "Next event", cmd: "next",        icon: SkipForward },
    { label: "Free today", cmd: "free today",  icon: CalendarCheck },
  ],
  tasks: [
    { label: "Today",     cmd: "todo today", icon: CalendarCheck },
    { label: "All tasks", cmd: "todo list",  icon: List },
  ],
  notion: [
    { label: "Databases",  cmd: "notion databases", icon: Database },
    { label: "Search docs", cmd: "notion search ",   icon: Search },
  ],
  jira: [
    { label: "My issues",    cmd: "jira mine",   icon: User },
    { label: "List open",    cmd: "jira list",   icon: Layers },
    { label: "Active sprint", cmd: "jira sprint", icon: Zap },
  ],
  home: [
    { label: "Status",       cmd: "status",       icon: LayoutDashboard },
    { label: "All lights",   cmd: "lights",        icon: Lightbulb },
    { label: "Scene: relax", cmd: "scene relax",   icon: Sunset },
    { label: "Scene: focus", cmd: "scene focus",   icon: Focus },
  ],
};

const PLACEHOLDER: Record<Tab, string> = {
  email:    "unread | inbox | read <id> | send <to> <subject> <body> | search <q> | triage | archive <id>",
  calendar: "today | week | list [days] | create <title> <start> <end> | free [date] | move <id> <start> | next",
  tasks:    "todo today | todo list | todo add <task> due:today p:1 | todo close <id>",
  notion:   "notion search <q> | notion get <page-id> | notion append <page-id> <text>",
  jira:     "jira mine | jira list [project] | jira create <proj> <summary> | jira get <key> | jira comment <key> <text>",
  home:     "status | lights | on <entity> | off <entity> | toggle <entity> | scene <name> | climate <entity> <temp>",
};

/* ── Tauri invoke mapping ─────────────────────────────────────────────────── */
async function runCommand(tab: Tab, cmd: string): Promise<string> {
  const c = cmd.trim();
  try {
    switch (tab) {
      case "email":
        return await invoke<string>("handle_email_command", { args: c });
      case "calendar":
        return await invoke<string>("handle_calendar_command", { args: c });
      case "tasks":
        return await invoke<string>("handle_productivity_command", { args: c });
      case "notion":
        return await invoke<string>("handle_productivity_command", { args: c });
      case "jira":
        return await invoke<string>("handle_productivity_command", { args: c });
      case "home":
        return await invoke<string>("handle_ha_command", { args: c });
    }
  } catch (e: unknown) {
    return `Error: ${e instanceof Error ? e.message : String(e)}`;
  }
}

/* ── Tab content component ─────────────────────────────────────────────────── */
function TabContent({ tab }: { tab: Tab }) {
  const [cmd, setCmd] = useState("");
  const [lines, setLines] = useState<OutputLine[]>([]);
  const [loading, setLoading] = useState(false);
  const counter = useRef(0);
  const outputRef = useRef<HTMLDivElement>(null);

  const pushLine = useCallback((text: string) => {
    const ts = new Date().toLocaleTimeString();
    setLines((prev) => [...prev, { id: counter.current++, text, ts }]);
    setTimeout(() => {
      if (outputRef.current) {
        outputRef.current.scrollTop = outputRef.current.scrollHeight;
      }
    }, 50);
  }, []);

  const run = useCallback(async (c: string) => {
    if (!c.trim()) return;
    setLoading(true);
    pushLine(`$ ${c}`);
    const result = await runCommand(tab, c);
    pushLine(result);
    setLoading(false);
    setCmd("");
  }, [tab, pushLine]);

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden", padding: 12, gap: 10 }}>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
        {QUICK[tab].map((q) => (
          <button
            key={q.label}
            className="panel-btn panel-btn-secondary"
            style={{ display: "flex", alignItems: "center", gap: 5 }}
            onClick={() => run(q.cmd)}
            disabled={loading}
          >
            <Ico icon={q.icon} size={12} />
            {q.label}
          </button>
        ))}
        <button className="panel-btn panel-btn-secondary" style={{ display: "flex", alignItems: "center", gap: 5, marginLeft: "auto", color: "var(--text-secondary)" }} onClick={() => setLines([])}>
          <Ico icon={Trash2} size={12} />
          Clear
        </button>
      </div>

      <div style={{ display: "flex", gap: 6 }}>
        <input
          className="panel-input"
          style={{ flex: 1 }}
          value={cmd}
          onChange={(e) => setCmd(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") run(cmd); }}
          placeholder={PLACEHOLDER[tab]}
          disabled={loading}
        />
        <button className="panel-btn panel-btn-primary" style={{ display: "flex", alignItems: "center", gap: 5 }} onClick={() => run(cmd)} disabled={loading || !cmd.trim()}>
          <Ico icon={loading ? Loader2 : Play} size={12} style={loading ? { animation: "spin 1s linear infinite" } : undefined} />
          {loading ? "Running" : "Run"}
        </button>
      </div>

      <div style={{ flex: 1, overflowY: "auto", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 10, whiteSpace: "pre-wrap", lineHeight: 1.6, fontSize: "var(--font-size-base)" }} ref={outputRef}>
        {lines.length === 0 ? (
          <span style={{ color: "var(--text-secondary)" }}>
            Use the quick buttons or type a command above.
          </span>
        ) : (
          lines.map((l) => (
            <div key={l.id}>
              <span style={{ color: "var(--text-muted, var(--text-secondary))", marginRight: 8, fontSize: "var(--font-size-sm)" }}>{l.ts}</span>
              {l.text}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

/* ── Main export ──────────────────────────────────────────────────────────── */
export function ProductivityPanel() {
  const [activeTab, setActiveTab] = useState<Tab>("email");

  return (
    <div className="panel-container" style={{ fontFamily: "var(--font-mono, monospace)" }}>
      <style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
      <div className="panel-tab-bar">
        {TABS.map((t) => (
          <button
            key={t.key}
            className={`panel-btn panel-tab${activeTab === t.key ? " active" : ""}`}
            style={{ display: "flex", alignItems: "center", gap: 5 }}
            onClick={() => setActiveTab(t.key)}
          >
            <Ico icon={t.icon} size={13} />
            {t.label}
          </button>
        ))}
      </div>
      <TabContent key={activeTab} tab={activeTab} />
    </div>
  );
}

export default ProductivityPanel;
