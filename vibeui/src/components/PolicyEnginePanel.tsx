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

const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, boxSizing: "border-box" };

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
    <div className="panel-container">
      <h2 style={headingStyle}>Policy Engine</h2>
      <div style={{ display: "flex", gap: 4, marginBottom: 12, flexWrap: "wrap" }}>
        {(["check", "policies", "test", "audit", "conflicts"] as Tab[]).map(t => (
          <button key={t} className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`} onClick={() => { setTab(t); if (t === "conflicts") doConflicts(); }}>
            {t === "check" ? "Check" : t === "policies" ? "Policies" : t === "test" ? "Test" : t === "audit" ? "Audit" : "Conflicts"}
          </button>
        ))}
      </div>

      {tab === "check" && (
        <>
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Authorization Check</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
              <div>
                <div className="panel-label">Principal</div>
                <input value={principal} onChange={e => setPrincipal(e.target.value)} style={inputStyle} />
              </div>
              <div>
                <div className="panel-label">Roles (comma-separated)</div>
                <input value={roles} onChange={e => setRoles(e.target.value)} style={inputStyle} />
              </div>
              <div>
                <div className="panel-label">Resource (kind:id)</div>
                <input value={resource} onChange={e => setResource(e.target.value)} style={inputStyle} />
              </div>
              <div>
                <div className="panel-label">Action</div>
                <input value={action} onChange={e => setAction(e.target.value)} style={inputStyle} />
              </div>
            </div>
            <button className="panel-btn panel-btn-secondary" onClick={doCheck} disabled={loading}>
              {loading ? "..." : "Evaluate"}
            </button>
          </div>
          {result && (
            <div className="panel-card" style={{ borderLeft: `3px solid ${result.effect.includes("ALLOW") ? "var(--success-color)" : "var(--error-color)"}` }}>
              <div style={{ fontSize: 20, fontWeight: 700, color: result.effect.includes("ALLOW") ? "var(--success-color)" : "var(--error-color)" }}>{result.effect}</div>
              <div className="panel-label">Action: {result.action} | Policy: {result.policyId}</div>
              {result.matchedRule && <div className="panel-label">Matched rule: {result.matchedRule}</div>}
            </div>
          )}
        </>
      )}

      {tab === "policies" && (
        <div className="panel-card">
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Add Policy (YAML)</div>
          <textarea value={yamlPolicy} onChange={e => setYamlPolicy(e.target.value)} rows={12} style={{ ...inputStyle, fontFamily: "monospace", resize: "vertical", marginBottom: 8 }} placeholder={'resourcePolicy:\n  resource: "document"\n  rules:\n    - actions: ["read"]\n      effect: ALLOW\n      roles: ["viewer"]'} />
          <button className="panel-btn panel-btn-secondary" disabled={!yamlPolicy || loading}>Add Policy</button>
          <div className="panel-label" style={{ marginTop: 8 }}>Use <code>/policy template &lt;resource&gt;</code> in terminal to generate a starter policy.</div>
        </div>
      )}

      {tab === "test" && (
        <div className="panel-card">
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Policy Test Suite</div>
          <div className="panel-label">Define test cases to verify your policies behave as expected.</div>
          <div style={{ marginTop: 8 }}>Use <code>/policy test suite.yaml</code> in the terminal to run test suites.</div>
        </div>
      )}

      {tab === "audit" && (
        <div className="panel-card">
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Audit Trail</div>
          <div className="panel-label">All authorization decisions are logged with full request/result/policy chain.</div>
          <div style={{ marginTop: 8 }}>Use <code>/policy audit</code> in the terminal to view the audit log.</div>
        </div>
      )}

      {tab === "conflicts" && (
        <>
          {conflicts.length === 0 && !loading && <div className="panel-empty">No policy conflicts detected.</div>}
          {conflicts.map((c, i) => (
            <div key={i} className="panel-card" style={{ borderLeft: "3px solid var(--warning-color)" }}>
              <div style={{ fontWeight: 600 }}>{c.policyA} vs {c.policyB}</div>
              <div className="panel-label">Resource: {c.resource}</div>
              <div>{c.description}</div>
            </div>
          ))}
        </>
      )}
    </div>
  );
}
