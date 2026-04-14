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
  const [tab, setTab] = useState("tree");
  const [tree, setTree] = useState<AgentNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [maxDepth, setMaxDepth] = useState(5);
  const [mergeStrategy, setMergeStrategy] = useState("last_write_wins");

  useEffect(() => {
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
    load();
  }, []);

  const statusColor = (s: string) => {
    if (s === "running") return "var(--success-color)";
    if (s === "completed") return "var(--accent-color)";
    if (s === "failed") return "var(--error-color)";
    if (s === "cancelled") return "var(--text-muted)";
    return "var(--warning-color)";
  };

  const flatNodes = flattenTree(tree);

  const renderTreeNode = (node: AgentNode, isLast: boolean, prefix: string) => {
    const connector = isLast ? "└─ " : "├─ ";
    const childPrefix = prefix + (isLast ? "   " : "│  ");
    return (
      <div key={node.id}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "3px 0" }}>
          <span style={{ color: "var(--text-muted)", fontFamily: "var(--font-mono)", whiteSpace: "pre", fontSize: "var(--font-size-base)" }}>{prefix}{connector}</span>
          <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>{node.name}</span>
          <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 7px", borderRadius: "var(--radius-sm-alt)", background: statusColor(node.status) + "22", color: statusColor(node.status), border: `1px solid ${statusColor(node.status)}` }}>{node.status}</span>
          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>{node.model}</span>
        </div>
        {(node.children ?? []).map((child, i) =>
          renderTreeNode(child, i === node.children.length - 1, childPrefix)
        )}
      </div>
    );
  };

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>Nested Agents</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["tree", "nodes", "config"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "tree" && (
        <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: 16, overflowX: "auto" }}>
          {tree.length === 0 && <div style={{ color: "var(--text-muted)" }}>No agent tree available.</div>}
          {tree.map((node, i) => renderTreeNode(node, i === tree.length - 1, ""))}
        </div>
      )}

      {!loading && tab === "nodes" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {flatNodes.length === 0 && <div style={{ color: "var(--text-muted)" }}>No nodes found.</div>}
          {flatNodes.map(node => (
            <div key={node.id} style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: "10px 14px", display: "flex", alignItems: "center", gap: 12 }}>
              <div style={{ width: node.depth * 16, flexShrink: 0 }} />
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{node.name}</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>depth {node.depth} · {node.model} · id: {node.id.slice(0, 10)}…</div>
              </div>
              <span style={{ fontSize: "var(--font-size-sm)", padding: "2px 10px", borderRadius: "var(--radius-md)", background: statusColor(node.status) + "22", color: statusColor(node.status), border: `1px solid ${statusColor(node.status)}` }}>{node.status}</span>
              <div style={{ display: "flex", gap: 6 }}>
                <button onClick={() => invoke("nested_agents_cancel", { nodeId: node.id })}
                  style={{ padding: "3px 10px", borderRadius: 5, cursor: "pointer", background: "var(--error-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-sm)" }}>
                  Cancel
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {!loading && tab === "config" && (
        <div style={{ maxWidth: 400 }}>
          <div style={{ marginBottom: 20 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Max Depth: <strong style={{ color: "var(--text-primary)" }}>{maxDepth}</strong></label>
            <input type="range" min={1} max={20} value={maxDepth} onChange={e => setMaxDepth(Number(e.target.value))}
              style={{ width: "100%", accentColor: "var(--accent-color)" }} />
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>
              <span>1</span><span>20</span>
            </div>
          </div>
          <div style={{ marginBottom: 20 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Merge Strategy</label>
            <select value={mergeStrategy} onChange={e => setMergeStrategy(e.target.value)}
              style={{ width: "100%", padding: "6px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>
              <option value="last_write_wins">Last Write Wins</option>
              <option value="first_write_wins">First Write Wins</option>
              <option value="merge_all">Merge All</option>
              <option value="parent_authority">Parent Authority</option>
            </select>
          </div>
          <button onClick={() => invoke("nested_agents_spawn", { maxDepth, mergeStrategy })}
            style={{ padding: "8px 20px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600 }}>
            Apply Config
          </button>
        </div>
      )}
    </div>
  );
}
