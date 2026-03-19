/**
 * CiStatusPanel — CI/CD AI Status Checks.
 *
 * Tabs: Suites (check suites with state badge), Checks (individual checks
 * with annotations), Config (enabled/required checks, thresholds).
 * Wired to Tauri backend commands: get_ci_status, get_ci_checks,
 * get_ci_config, trigger_ci_check.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "suites" | "checks" | "config";
type SuiteState = "success" | "failure" | "pending" | "running";
type CheckState = "pass" | "fail" | "warn" | "skip" | "running" | "pending";

interface CheckSuite {
  id: string;
  name: string;
  state: SuiteState;
  branch: string;
  commit: string;
  duration: string;
  checksCount: number;
  passCount: number;
}

interface Check {
  id: string;
  suiteId: string;
  name: string;
  state: CheckState;
  annotations: number;
  duration: string;
  message: string;
}

interface CheckConfig {
  id: string;
  name: string;
  enabled: boolean;
  required: boolean;
  threshold: number;
}

const stateColors: Record<string, string> = {
  success: "var(--text-success)", pass: "var(--text-success)",
  failure: "var(--text-danger)", fail: "var(--text-danger)",
  pending: "var(--text-muted)", skip: "var(--text-muted)",
  running: "var(--text-warning)", warn: "var(--text-warning)",
};

const tabBtn = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px", fontSize: 11, fontWeight: active ? 600 : 400,
  background: active ? "var(--accent-bg)" : "transparent",
  border: "1px solid " + (active ? "var(--accent-primary)" : "var(--border-color)"),
  borderRadius: 4, color: active ? "var(--text-info)" : "var(--text-muted)", cursor: "pointer",
});

export default function CiStatusPanel() {
  const [tab, setTab] = useState<Tab>("suites");
  const [selectedSuite, setSelectedSuite] = useState<string | null>(null);
  const [suites, setSuites] = useState<CheckSuite[]>([]);
  const [checks, setChecks] = useState<Check[]>([]);
  const [config, setConfig] = useState<CheckConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [runningCheck, setRunningCheck] = useState<string | null>(null);

  const workspace = ".";

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [s, ch, cfg] = await Promise.all([
        invoke<CheckSuite[]>("get_ci_status", { workspace }),
        invoke<Check[]>("get_ci_checks", { workspace }),
        invoke<CheckConfig[]>("get_ci_config", { workspace }),
      ]);
      setSuites(s);
      setChecks(ch);
      setConfig(cfg);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [workspace]);

  useEffect(() => { loadData(); }, [loadData]);

  const toggleConfig = (id: string, field: "enabled" | "required") => {
    setConfig(cs => cs.map(c => c.id === id ? { ...c, [field]: !c[field] } : c));
  };

  const triggerCheck = async (checkName: string) => {
    setRunningCheck(checkName);
    try {
      const [stdout, stderr, code] = await invoke<[string, string, number]>("trigger_ci_check", {
        workspace,
        checkName,
      });
      // Update the matching config check or check item with result
      setChecks(prev => prev.map(c => {
        if (c.name === checkName || c.name.toLowerCase().includes(checkName.toLowerCase())) {
          return {
            ...c,
            state: code === 0 ? "pass" as CheckState : "fail" as CheckState,
            message: code === 0
              ? (stdout.split('\n').filter(l => l.trim()).pop() || "Passed")
              : (stderr.split('\n').filter(l => l.trim()).pop() || stdout.split('\n').filter(l => l.trim()).pop() || "Failed"),
          };
        }
        return c;
      }));
    } catch (e) {
      setError(`Check failed: ${e}`);
    } finally {
      setRunningCheck(null);
    }
  };

  const filteredChecks = selectedSuite ? checks.filter(c => c.suiteId === selectedSuite) : checks;

  if (loading) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "var(--text-muted)", fontSize: 12 }}>
        Loading CI status...
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div style={{ display: "flex", gap: 6, padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", alignItems: "center" }}>
        {(["suites", "checks", "config"] as Tab[]).map(t => (
          <button key={t} onClick={() => setTab(t)} style={tabBtn(tab === t)}>
            {t[0].toUpperCase() + t.slice(1)}
          </button>
        ))}
        <button onClick={loadData} style={{ marginLeft: "auto", padding: "4px 10px", fontSize: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>
          Refresh
        </button>
      </div>

      {error && (
        <div style={{ padding: "6px 10px", fontSize: 11, color: "var(--text-danger)", background: "rgba(243,139,168,0.1)", borderBottom: "1px solid var(--border-color)" }}>
          {error}
        </div>
      )}

      <div style={{ flex: 1, overflowY: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 8 }}>
        {tab === "suites" && suites.length === 0 && (
          <div style={{ fontSize: 12, color: "var(--text-muted)", textAlign: "center", padding: 24 }}>
            No CI configurations detected in this workspace.
            <br /><span style={{ fontSize: 10 }}>Add .github/workflows/, .gitlab-ci.yml, Jenkinsfile, etc.</span>
          </div>
        )}

        {tab === "suites" && suites.map(s => (
          <div key={s.id} role="button" tabIndex={0} onClick={() => { setSelectedSuite(s.id); setTab("checks"); }} onKeyDown={e => e.key === "Enter" && (setSelectedSuite(s.id), setTab("checks"))}
            style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", cursor: "pointer" }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
              <span style={{ width: 8, height: 8, borderRadius: "50%", background: stateColors[s.state] }} />
              <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{s.name}</span>
              <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: `${stateColors[s.state]}22`, color: stateColors[s.state] }}>{s.state}</span>
              <span style={{ fontSize: 10, color: "var(--text-muted)", marginLeft: "auto" }}>{s.duration}</span>
            </div>
            <div style={{ display: "flex", gap: 12, fontSize: 10, color: "var(--text-muted)" }}>
              <span>Branch: <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-info)" }}>{s.branch}</span></span>
              <span>Commit: <span style={{ fontFamily: "var(--font-mono)" }}>{s.commit}</span></span>
              <span style={{ marginLeft: "auto" }}>{s.passCount}/{s.checksCount} passed</span>
            </div>
            <div style={{ marginTop: 6, height: 3, background: "var(--bg-primary)", borderRadius: 2, overflow: "hidden" }}>
              <div style={{ width: s.checksCount > 0 ? `${(s.passCount / s.checksCount) * 100}%` : "0%", height: "100%", background: stateColors[s.state], borderRadius: 2 }} />
            </div>
          </div>
        ))}

        {tab === "checks" && (
          <>
            {selectedSuite && (
              <button onClick={() => setSelectedSuite(null)}
                style={{ alignSelf: "flex-start", padding: "4px 10px", fontSize: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer", marginBottom: 4 }}>
                Show all checks
              </button>
            )}
            {filteredChecks.length === 0 && (
              <div style={{ fontSize: 12, color: "var(--text-muted)", textAlign: "center", padding: 24 }}>
                No checks found.
              </div>
            )}
            {filteredChecks.map(c => (
              <div key={c.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", display: "flex", gap: 10, alignItems: "center" }}>
                <span style={{ width: 8, height: 8, borderRadius: "50%", background: stateColors[c.state], flexShrink: 0 }} />
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{c.name}</div>
                  <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 2 }}>{c.message}</div>
                </div>
                {c.annotations > 0 && (
                  <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: "rgba(243,139,168,0.15)", color: "var(--text-danger)" }}>
                    {c.annotations} annotations
                  </span>
                )}
                <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{c.duration}</span>
              </div>
            ))}
          </>
        )}

        {tab === "config" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {config.length === 0 && (
              <div style={{ fontSize: 12, color: "var(--text-muted)", textAlign: "center", padding: 24 }}>
                No CI configuration detected.
              </div>
            )}
            {config.length > 0 && (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 60px 60px 80px 70px", gap: 4, padding: "6px 10px", fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>
                <span>Check</span><span>Enabled</span><span>Required</span><span>Threshold</span><span>Run</span>
              </div>
            )}
            {config.map(c => (
              <div key={c.id} style={{ display: "grid", gridTemplateColumns: "1fr 60px 60px 80px 70px", gap: 4, padding: "8px 10px", background: "var(--bg-secondary)", borderRadius: 4, border: "1px solid var(--border-color)", alignItems: "center" }}>
                <span style={{ fontSize: 11, color: c.enabled ? "var(--text-primary)" : "var(--text-muted)" }}>{c.name}</span>
                <input type="checkbox" checked={c.enabled} onChange={() => toggleConfig(c.id, "enabled")} style={{ cursor: "pointer" }} />
                <input type="checkbox" checked={c.required} onChange={() => toggleConfig(c.id, "required")} disabled={!c.enabled} style={{ cursor: "pointer" }} />
                <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--text-muted)" }}>{c.threshold}</span>
                <button
                  disabled={!c.enabled || runningCheck === c.name}
                  onClick={() => triggerCheck(c.name)}
                  style={{
                    padding: "2px 8px", fontSize: 10, borderRadius: 3, cursor: c.enabled ? "pointer" : "default",
                    background: c.enabled ? "var(--accent-bg)" : "var(--bg-primary)",
                    border: "1px solid " + (c.enabled ? "var(--accent-primary)" : "var(--border-color)"),
                    color: c.enabled ? "var(--text-info)" : "var(--text-muted)",
                    opacity: runningCheck === c.name ? 0.6 : 1,
                  }}>
                  {runningCheck === c.name ? "..." : "Run"}
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
