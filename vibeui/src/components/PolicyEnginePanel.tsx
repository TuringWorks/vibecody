/**
 * PolicyEnginePanel — Cerbos-style Authorization Policy dashboard.
 *
 * Define RBAC/ABAC policies, test authorization decisions, view audit trails,
 * detect conflicts, and analyze coverage.
 */
import React, { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Play } from "lucide-react";

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

const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: "var(--font-size-xl)", fontWeight: 600, color: "var(--text-primary)" };

type Tab = "check" | "policies" | "test" | "audit" | "conflicts";

const CliBtn = ({ args, label, runCli }: { args: string; label: React.ReactNode; runCli: (args: string) => void }) => (
  <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli(args)} title={`vibecli --cmd "/policy ${args}"`} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>{label}</button>
);

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
  const [cliOutput, setCliOutput] = useState("");
  const [cliError, setCliError] = useState("");

  const runCli = useCallback(async (args: string) => {
    setCliOutput(""); setCliError("");
    try {
      const res = await invoke<string>("handle_policy_command", { args });
      setCliOutput(res);
    } catch (e) { setCliError(String(e)); }
  }, []);

  const doCheck = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<CheckResultDisplay>("policy_check", {
        principalId: principal, roles: roles.split(",").map(r => r.trim()), resourceKind: resource.split(":")[0], resourceId: resource.split(":")[1] || "", action
      });
      setResult(res);
    } catch (_e) { setResult({ action, effect: "DENY (error)", matchedRule: null, policyId: "none" }); }
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
      {/* panel-body gives all 4-sided padding per design-system/components/panel.md */}
      <div className="panel-body">
      <h2 style={headingStyle}>Policy Engine</h2>
      <div style={{ display: "flex", gap: 4, marginBottom: 12, flexWrap: "wrap" }}>
        {(["check", "policies", "test", "audit", "conflicts"] as Tab[]).map(t => (
          <button key={t} className={`panel-tab panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`} onClick={() => { setTab(t); if (t === "conflicts") doConflicts(); }}>
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
                <input value={principal} onChange={e => setPrincipal(e.target.value)} className="panel-input panel-input-full" />
              </div>
              <div>
                <div className="panel-label">Roles (comma-separated)</div>
                <input value={roles} onChange={e => setRoles(e.target.value)} className="panel-input panel-input-full" />
              </div>
              <div>
                <div className="panel-label">Resource (kind:id)</div>
                <input value={resource} onChange={e => setResource(e.target.value)} className="panel-input panel-input-full" />
              </div>
              <div>
                <div className="panel-label">Action</div>
                <input value={action} onChange={e => setAction(e.target.value)} className="panel-input panel-input-full" />
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
          <textarea value={yamlPolicy} onChange={e => setYamlPolicy(e.target.value)} rows={12} className="panel-input panel-textarea panel-input-full" style={{ fontFamily: "monospace", resize: "vertical", marginBottom: 8 }} placeholder={'resourcePolicy:\n  resource: "document"\n  rules:\n    - actions: ["read"]\n      effect: ALLOW\n      roles: ["viewer"]'} />
          <div style={{ display: "flex", gap: 8, marginTop: 8, flexWrap: "wrap" }}>
            <button className="panel-btn panel-btn-secondary" disabled={!yamlPolicy || loading}>Add Policy</button>
            <CliBtn args={`template ${resource.split(":")[0] || "document"}`} label={<><Play size={12} /> Generate Template</>} runCli={runCli} />
          </div>
        </div>
      )}

      {tab === "test" && (
        <div className="panel-card">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontWeight: 600 }}>Policy Test Suite</span>
            <CliBtn args="test suite.yaml" label={<><Play size={12} /> Run Tests</>} runCli={runCli} />
          </div>
          <div className="panel-label">Define test cases to verify your policies behave as expected.</div>
        </div>
      )}

      {tab === "audit" && (
        <div className="panel-card">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontWeight: 600 }}>Audit Trail</span>
            <CliBtn args="audit" label={<><Play size={12} /> View Audit Log</>} runCli={runCli} />
          </div>
          <div className="panel-label">All authorization decisions are logged with full request/result/policy chain.</div>
        </div>
      )}

      {(cliOutput || cliError) && (
        <div className={`panel-card ${cliError ? "panel-error" : ""}`} style={{ marginTop: 8 }}>
          <pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: "var(--font-size-sm)" }}>{cliError || cliOutput}</pre>
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
    </div>
  );
}
