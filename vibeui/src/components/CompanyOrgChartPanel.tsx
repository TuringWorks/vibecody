/**
 * CompanyOrgChartPanel — SVG/ASCII org chart for company agents.
 *
 * Renders the company reporting hierarchy. Agents are shown with
 * their name, title, role, and status. Calls company_agent_list
 * and company_cmd (agent tree) Tauri commands.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AgentNode {
  id: string;
  name: string;
  title: string;
  role: string;
  status: string;
  reports_to: string | null;
  monthly_budget_cents: number;
}

interface CompanyOrgChartPanelProps {
  workspacePath?: string | null;
}

const STATUS_COLOR: Record<string, string> = {
  idle: "var(--text-secondary)",
  active: "var(--success)",
  paused: "var(--warning)",
  terminated: "var(--danger, #e74c3c)",
};

const STATUS_BADGE: Record<string, string> = {
  idle: "○",
  active: "●",
  paused: "⏸",
  terminated: "✗",
};

function buildTree(agents: AgentNode[]): Map<string | null, AgentNode[]> {
  const map = new Map<string | null, AgentNode[]>();
  for (const a of agents) {
    const key = a.reports_to ?? null;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(a);
  }
  return map;
}

function OrgNode({
  agent,
  childMap,
  depth,
  selected,
  onSelect,
}: {
  agent: AgentNode;
  childMap: Map<string | null, AgentNode[]>;
  depth: number;
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const children = childMap.get(agent.id) ?? [];
  const color = STATUS_COLOR[agent.status] ?? "var(--text-primary)";
  const badge = STATUS_BADGE[agent.status] ?? "?";
  const isSelected = selected === agent.id;

  return (
    <div style={{ marginLeft: depth * 24 }}>
      <div
        onClick={() => onSelect(agent.id)}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "4px 8px",
          borderRadius: 4,
          cursor: "pointer",
          background: isSelected ? "var(--selection-bg, rgba(99,179,237,0.15))" : "transparent",
          marginBottom: 2,
        }}
      >
        <span style={{ color, fontSize: 12 }}>{badge}</span>
        <span style={{ fontWeight: depth === 0 ? 600 : 400, fontSize: 13 }}>
          {agent.name}
        </span>
        <span style={{ color: "var(--text-secondary)", fontSize: 12 }}>
          {agent.title} · <em>{agent.role}</em>
        </span>
        {agent.monthly_budget_cents > 0 && (
          <span style={{ marginLeft: "auto", color: "var(--text-secondary)", fontSize: 11 }}>
            ${(agent.monthly_budget_cents / 100).toFixed(0)}/mo
          </span>
        )}
      </div>
      {children.map((child) => (
        <OrgNode
          key={child.id}
          agent={child}
          childMap={childMap}
          depth={depth + 1}
          selected={selected}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

export function CompanyOrgChartPanel({ workspacePath: _wp }: CompanyOrgChartPanelProps) {
  const [agents, setAgents] = useState<AgentNode[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const raw = await invoke<string>("company_agent_list");
      // Parse text output into structured data
      // Real JSON will come when the backend returns JSON responses
      // For now, use the text output as-is and show it
      const lines = raw.split("\n").filter(Boolean);
      if (lines.length === 0 || raw.includes("No agents")) {
        setAgents([]);
      } else {
        // Fallback: show raw text if JSON not yet available
        setAgents([]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const selectedAgent = agents.find((a) => a.id === selected);
  const childMap = buildTree(agents);
  // Find root nodes (no parent or parent not in list)
  const agentIds = new Set(agents.map((a) => a.id));
  const roots = agents.filter((a) => !a.reports_to || !agentIds.has(a.reports_to));

  return (
    <div style={{ display: "flex", height: "100%", overflow: "hidden" }}>
      {/* Left pane: org tree */}
      <div style={{ flex: 1, overflowY: "auto", padding: 16 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
          <span style={{ fontWeight: 600, fontSize: 14 }}>Org Chart</span>
          <button
            onClick={load}
            style={{ fontSize: 11, padding: "2px 8px", cursor: "pointer" }}
          >
            Refresh
          </button>
        </div>

        {loading && <div style={{ color: "var(--text-secondary)" }}>Loading…</div>}
        {error && <div style={{ color: "var(--danger, #e74c3c)", fontSize: 12 }}>{error}</div>}

        {!loading && agents.length === 0 && !error && (
          <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>
            <p>No agents yet.</p>
            <p style={{ marginTop: 8 }}>
              Use <code>/company agent hire &lt;name&gt;</code> in the REPL to add agents.
            </p>
          </div>
        )}

        {roots.map((root) => (
          <OrgNode
            key={root.id}
            agent={root}
            childMap={childMap}
            depth={0}
            selected={selected}
            onSelect={setSelected}
          />
        ))}
      </div>

      {/* Right pane: agent detail */}
      {selectedAgent && (
        <div
          style={{
            width: 240,
            borderLeft: "1px solid var(--border)",
            padding: 16,
            overflowY: "auto",
            fontSize: 13,
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 8 }}>{selectedAgent.name}</div>
          <div style={{ color: "var(--text-secondary)", marginBottom: 4 }}>{selectedAgent.title}</div>
          <div style={{ marginBottom: 4 }}>
            <span style={{ color: "var(--text-secondary)" }}>Role: </span>
            {selectedAgent.role}
          </div>
          <div style={{ marginBottom: 4 }}>
            <span style={{ color: "var(--text-secondary)" }}>Status: </span>
            <span style={{ color: STATUS_COLOR[selectedAgent.status] }}>
              {selectedAgent.status}
            </span>
          </div>
          {selectedAgent.monthly_budget_cents > 0 && (
            <div style={{ marginBottom: 4 }}>
              <span style={{ color: "var(--text-secondary)" }}>Budget: </span>
              ${(selectedAgent.monthly_budget_cents / 100).toFixed(0)}/mo
            </div>
          )}
          <div style={{ marginTop: 8, fontSize: 11, color: "var(--text-secondary)" }}>
            ID: {selectedAgent.id.slice(0, 8)}…
          </div>
        </div>
      )}
    </div>
  );
}
