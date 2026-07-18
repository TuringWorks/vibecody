import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AgentNode {
  id: string;
  name: string;
  parent_id: string | null;
  depth: number;
  status: string;
  model: string;
  children: AgentNode[];
}

interface FlatNode {
  id: string;
  name: string;
  depth: number;
  status: string;
  model: string;
  parent_id: string | null;
}

type TagIntent = "info" | "success" | "warning" | "danger" | "neutral";

function statusIntent(s: string): TagIntent {
  switch (s) {
    case "running": return "success";
    case "completed": return "info";
    case "failed": return "danger";
    case "cancelled": return "neutral";
    default: return "warning";
  }
}

function flattenTree(nodes: AgentNode[]): FlatNode[] {
  const result: FlatNode[] = [];
  function walk(n: AgentNode) {
    result.push({ id: n.id, name: n.name, depth: n.depth, status: n.status, model: n.model, parent_id: n.parent_id });
    (n.children ?? []).forEach(walk);
  }
  nodes.forEach(walk);
  return result;
}

export function NestedAgentsPanel() {
  const [tab, setTab] = useState<"tree" | "nodes" | "config">("tree");
  const [tree, setTree] = useState<AgentNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [maxDepth, setMaxDepth] = useState(5);
  const [mergeStrategy, setMergeStrategy] = useState("last_write_wins");

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<AgentNode[]>("nested_agents_tree");
      setTree(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { load(); }, []);

  const flatNodes = flattenTree(tree);

  const renderTreeNode = (node: AgentNode, isLast: boolean, prefix: string) => {
    const connector = isLast ? "└─ " : "├─ ";
    const childPrefix = prefix + (isLast ? "   " : "│  ");
    return (
      <div key={node.id}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "3px 0" }}>
          <span style={{ color: "var(--text-muted)", fontFamily: "var(--font-mono)", whiteSpace: "pre", fontSize: "var(--font-size-base)" }}>{prefix}{connector}</span>
          <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{node.name}</span>
          <span className={`panel-tag panel-tag-${statusIntent(node.status)}`}>{node.status}</span>
          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>{node.model}</span>
        </div>
        {(node.children ?? []).map((child, i) =>
          renderTreeNode(child, i === node.children.length - 1, childPrefix)
        )}
      </div>
    );
  };

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Nested Agents</h3>
        <button
          className="panel-btn panel-btn-secondary panel-btn-sm"
          style={{ marginLeft: "auto" }}
          onClick={load}
          disabled={loading}
        >
          Refresh
        </button>
      </div>

      <div className="panel-body">
        <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
          {(["tree", "nodes", "config"] as const).map(t => (
            <button
              key={t}
              className={`panel-tab${tab === t ? " active" : ""}`}
              onClick={() => setTab(t)}
            >
              {t}
            </button>
          ))}
        </div>

        {loading && <div className="panel-loading">Loading…</div>}
        {error && (
          <div className="panel-error">
            <span>{error}</span>
            <button onClick={() => setError(null)} aria-label="dismiss">✕</button>
          </div>
        )}

        {!loading && tab === "tree" && (
          tree.length === 0 ? (
            <div className="panel-empty">No agent tree available.</div>
          ) : (
            <div className="panel-card" style={{ overflowX: "auto" }}>
              {tree.map((node, i) => renderTreeNode(node, i === tree.length - 1, ""))}
            </div>
          )
        )}

        {!loading && tab === "nodes" && (
          flatNodes.length === 0 ? (
            <div className="panel-empty">No nodes found.</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {flatNodes.map(node => (
                <div key={node.id} className="panel-card" style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <div style={{ width: node.depth * 16, flexShrink: 0 }} />
                  <div style={{ flex: 1 }}>
                    <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{node.name}</div>
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>
                      depth {node.depth} · {node.model} · id: {node.id.slice(0, 10)}…
                    </div>
                  </div>
                  <span className={`panel-tag panel-tag-${statusIntent(node.status)}`}>{node.status}</span>
                  <button
                    className="panel-btn panel-btn-danger panel-btn-xs"
                    onClick={() => invoke("nested_agents_cancel", { nodeId: node.id })}
                  >
                    Cancel
                  </button>
                </div>
              ))}
            </div>
          )
        )}

        {!loading && tab === "config" && (
          <div style={{ maxWidth: 400 }}>
            <div style={{ marginBottom: 20 }}>
              <label className="panel-label" style={{ display: "block" }}>
                Max Depth: <strong style={{ color: "var(--text-primary)" }}>{maxDepth}</strong>
              </label>
              <input
                type="range"
                min={1}
                max={20}
                value={maxDepth}
                onChange={e => setMaxDepth(Number(e.target.value))}
                style={{ width: "100%", accentColor: "var(--accent-color)" }}
              />
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>
                <span>1</span><span>20</span>
              </div>
            </div>
            <div style={{ marginBottom: 20 }}>
              <label className="panel-label" style={{ display: "block" }}>Merge Strategy</label>
              <select
                className="panel-select"
                value={mergeStrategy}
                onChange={e => setMergeStrategy(e.target.value)}
                style={{ width: "100%" }}
              >
                <option value="last_write_wins">Last Write Wins</option>
                <option value="first_write_wins">First Write Wins</option>
                <option value="merge_all">Merge All</option>
                <option value="parent_authority">Parent Authority</option>
              </select>
            </div>
            <button
              className="panel-btn panel-btn-primary"
              onClick={() => invoke("nested_agents_spawn", { maxDepth, mergeStrategy })}
            >
              Apply Config
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
