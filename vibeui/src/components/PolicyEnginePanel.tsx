/**
 * PolicyEnginePanel — Cerbos-style Authorization Policy dashboard.
 *
 * Define RBAC/ABAC policies, test authorization decisions, view audit trails,
 * detect conflicts, and analyze coverage.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CheckResultDisplay {
  action: string;
  effect: string;
  matchedRule: string | null;
  policyId: string;
}

interface PolicyConflictDisplay {
  policyA: string;
  policyB: string;
  resource: string;
  description: string;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, boxSizing: "border-box" };
const tabRow: React.CSSProperties = { display: "flex", gap: 4, marginBottom: 12, flexWrap: "wrap" };

type Tab = "check" | "policies" | "test" | "audit" | "conflicts";

export default function PolicyEnginePanel() {
  const [tab, setTab] = useState<Tab>("check");
  const [principal, setPrincipal] = useState("user:alice");
  const [roles, setRoles] = useState("editor");
  const [resource, setResource] = useState("document:123");
  const [action, setAction] = useState("read");
  const [result, setResult] = useState<CheckResultDisplay | null>(null);
  const [conflicts, setConflicts] = useState<PolicyConflictDisplay[]>([]);
  const [yamlPolicy, setYamlPolicy] = useState("");
  const [loading, setLoading] = useState(false);

  const doCheck = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<CheckResultDisplay>("policy_check", {
        principalId: principal, roles: roles.split(",").map(r => r.trim()), resourceKind: resource.split(":")[0], resourceId: resource.split(":")[1] || "", action
      });
      setResult(res);
    } catch (e) { setResult({ action, effect: "DENY (error)", matchedRule: null, policyId: "none" }); }
    setLoading(false);
  }, [principal, roles, resource, action]);

  const doConflicts = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<{ conflicts: PolicyConflictDisplay[] }>("policy_conflicts");
      setConflicts(res.conflicts);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Policy Engine</h2>
      <div style={tabRow}>
        {(["check", "policies", "test", "audit", "conflicts"] as Tab[]).map(t => (
          <button key={t} style={{ ...btnStyle, background: tab === t ? "var(--accent-color)" : "var(--bg-tertiary)", color: tab === t ? "#fff" : "var(--text-primary)" }} onClick={() => { setTab(t); if (t === "conflicts") doConflicts(); }}>
            {t === "check" ? "Check" : t === "policies" ? "Policies" : t === "test" ? "Test" : t === "audit" ? "Audit" : "Conflicts"}
          </button>
        ))}
      </div>

      {tab === "check" && (
        <>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Authorization Check</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
              <div>
                <div style={labelStyle}>Principal</div>
                <input value={principal} onChange={e => setPrincipal(e.target.value)} style={inputStyle} />
              </div>
              <div>
                <div style={labelStyle}>Roles (comma-separated)</div>
                <input value={roles} onChange={e => setRoles(e.target.value)} style={inputStyle} />
              </div>
              <div>
                <div style={labelStyle}>Resource (kind:id)</div>
                <input value={resource} onChange={e => setResource(e.target.value)} style={inputStyle} />
              </div>
              <div>
                <div style={labelStyle}>Action</div>
                <input value={action} onChange={e => setAction(e.target.value)} style={inputStyle} />
              </div>
            </div>
            <button style={btnStyle} onClick={doCheck} disabled={loading}>
              {loading ? "..." : "Evaluate"}
            </button>
          </div>
          {result && (
            <div style={{ ...cardStyle, borderLeft: `3px solid ${result.effect.includes("ALLOW") ? "#4caf50" : "#f44336"}` }}>
              <div style={{ fontSize: 20, fontWeight: 700, color: result.effect.includes("ALLOW") ? "#4caf50" : "#f44336" }}>{result.effect}</div>
              <div style={labelStyle}>Action: {result.action} | Policy: {result.policyId}</div>
              {result.matchedRule && <div style={labelStyle}>Matched rule: {result.matchedRule}</div>}
            </div>
          )}
        </>
      )}

      {tab === "policies" && (
        <div style={cardStyle}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Add Policy (YAML)</div>
          <textarea value={yamlPolicy} onChange={e => setYamlPolicy(e.target.value)} rows={12} style={{ ...inputStyle, fontFamily: "monospace", resize: "vertical", marginBottom: 8 }} placeholder={'resourcePolicy:\n  resource: "document"\n  rules:\n    - actions: ["read"]\n      effect: ALLOW\n      roles: ["viewer"]'} />
          <button style={btnStyle} disabled={!yamlPolicy || loading}>Add Policy</button>
          <div style={{ ...labelStyle, marginTop: 8 }}>Use <code>/policy template &lt;resource&gt;</code> in terminal to generate a starter policy.</div>
        </div>
      )}

      {tab === "test" && (
        <div style={cardStyle}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Policy Test Suite</div>
          <div style={labelStyle}>Define test cases to verify your policies behave as expected.</div>
          <div style={{ marginTop: 8 }}>Use <code>/policy test suite.yaml</code> in the terminal to run test suites.</div>
        </div>
      )}

      {tab === "audit" && (
        <div style={cardStyle}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Audit Trail</div>
          <div style={labelStyle}>All authorization decisions are logged with full request/result/policy chain.</div>
          <div style={{ marginTop: 8 }}>Use <code>/policy audit</code> in the terminal to view the audit log.</div>
        </div>
      )}

      {tab === "conflicts" && (
        <>
          {conflicts.length === 0 && !loading && <div style={labelStyle}>No policy conflicts detected.</div>}
          {conflicts.map((c, i) => (
            <div key={i} style={{ ...cardStyle, borderLeft: "3px solid #ff9800" }}>
              <div style={{ fontWeight: 600 }}>{c.policyA} vs {c.policyB}</div>
              <div style={labelStyle}>Resource: {c.resource}</div>
              <div>{c.description}</div>
            </div>
          ))}
        </>
      )}
    </div>
  );
}
