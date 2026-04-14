import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

const badgeStyle = (color: string, bg: string): React.CSSProperties => ({
  padding: "2px 8px", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-sm)", fontWeight: 600, color, background: bg,
});

interface Dep {
  name: string;
  version: string;
  manager: string;
  dev: boolean;
}

interface Conflict {
  id: number;
  pkg: string;
  versions: string[];
  reason: string;
  strategy: string;
}

interface Advisory {
  pkg: string;
  severity: string;
  cve: string;
  desc: string;
  fixAvailable: boolean;
}

interface License {
  pkg: string;
  license: string;
  status: string;
}

interface AnalysisResult {
  deps: Dep[];
  conflicts: Conflict[];
}

export function SmartDepsPanel() {
  const [tab, setTab] = useState("dependencies");

  const [deps, setDeps] = useState<Dep[]>([]);
  const [conflicts, setConflicts] = useState<Conflict[]>([]);
  const [advisories, setAdvisories] = useState<Advisory[]>([]);
  const [licenses, setLicenses] = useState<License[]>([]);

  const [loadingAnalysis, setLoadingAnalysis] = useState(false);
  const [loadingSecurity, setLoadingSecurity] = useState(false);
  const [loadingLicenses, setLoadingLicenses] = useState(false);

  const [errorAnalysis, setErrorAnalysis] = useState<string | null>(null);
  const [errorSecurity, setErrorSecurity] = useState<string | null>(null);
  const [errorLicenses, setErrorLicenses] = useState<string | null>(null);

  const fetchAnalysis = async () => {
    setLoadingAnalysis(true);
    setErrorAnalysis(null);
    try {
      const result = await invoke<AnalysisResult>("smartdeps_analyze");
      setDeps(result.deps ?? []);
      setConflicts(result.conflicts ?? []);
    } catch (err: unknown) {
      setErrorAnalysis(err instanceof Error ? err.message : String(err));
    } finally {
      setLoadingAnalysis(false);
    }
  };

  const fetchSecurity = async () => {
    setLoadingSecurity(true);
    setErrorSecurity(null);
    try {
      const result = await invoke<Advisory[]>("smartdeps_check_security");
      setAdvisories(result ?? []);
    } catch (err: unknown) {
      setErrorSecurity(err instanceof Error ? err.message : String(err));
    } finally {
      setLoadingSecurity(false);
    }
  };

  const fetchLicenses = async () => {
    setLoadingLicenses(true);
    setErrorLicenses(null);
    try {
      const result = await invoke<License[]>("smartdeps_check_licenses");
      setLicenses(result ?? []);
    } catch (err: unknown) {
      setErrorLicenses(err instanceof Error ? err.message : String(err));
    } finally {
      setLoadingLicenses(false);
    }
  };

  useEffect(() => {
    fetchAnalysis();
  }, []);

  const sevColor = (s: string) => {
    if (s === "critical") return { color: "var(--btn-primary-fg, #fff)", bg: "var(--error-color)" };
    if (s === "high") return { color: "var(--error-color)", bg: "var(--error-bg)" };
    if (s === "medium") return { color: "var(--warning-color)", bg: "#eab30820" };
    return { color: "var(--accent-color)", bg: "#3b82f620" };
  };

  const mgrColor = (m: string) => {
    if (m === "npm") return { color: "#cb3837", bg: "#cb383720" };
    if (m === "cargo") return { color: "var(--warning-color)", bg: "#f59e0b20" };
    return { color: "var(--accent-color)", bg: "#3b82f620" };
  };

  const errorBanner = (msg: string) => (
    <div className="panel-error"><span>Error: {msg}</span></div>
  );

  const loadingBanner = (msg: string) => (
    <div className="panel-loading">{msg}</div>
  );

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>Agentic Package Manager</h2>
      <div className="panel-tab-bar" style={{ marginBottom: 16 }}>
        {["dependencies", "conflicts", "security", "licenses"].map((t) => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "dependencies" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <button className="panel-btn panel-btn-primary" onClick={fetchAnalysis} disabled={loadingAnalysis}>
              {loadingAnalysis ? "Analyzing..." : "Analyze"}
            </button>
          </div>
          {errorAnalysis && errorBanner(errorAnalysis)}
          {loadingAnalysis && loadingBanner("Analyzing dependencies...")}
          {!loadingAnalysis && !errorAnalysis && deps.length === 0 && (
            <div className="panel-empty">No dependencies found. Click Analyze to scan.</div>
          )}
          {deps.map((d) => (
            <div key={d.name} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{d.name}</span>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{d.version}</span>
                {d.dev && <span style={badgeStyle("var(--text-secondary)", "var(--border-color)")}>dev</span>}
              </div>
              <span style={badgeStyle(mgrColor(d.manager).color, mgrColor(d.manager).bg)}>{d.manager}</span>
            </div>
          ))}
        </div>
      )}

      {tab === "conflicts" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <button className="panel-btn panel-btn-primary" onClick={fetchAnalysis} disabled={loadingAnalysis}>
              {loadingAnalysis ? "Analyzing..." : "Refresh"}
            </button>
          </div>
          {errorAnalysis && errorBanner(errorAnalysis)}
          {loadingAnalysis && loadingBanner("Checking conflicts...")}
          {!loadingAnalysis && !errorAnalysis && conflicts.length === 0 && (
            <div className="panel-empty">No conflicts detected.</div>
          )}
          {conflicts.map((c) => (
            <div key={c.id} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{c.pkg}</span>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{c.versions.join(" vs ")}</span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>{c.reason}</div>
              <div style={{ display: "flex", gap: 4 }}>
                {["dedupe", "upgrade", "align", "ignore"].map((s) => (
                  <button key={s} onClick={() => setConflicts((prev) => prev.map((x) => x.id === c.id ? { ...x, strategy: s } : x))}
                    className={c.strategy === s ? "panel-btn panel-btn-primary" : "panel-btn panel-btn-secondary"}
                    style={{ fontSize: "var(--font-size-sm)", padding: "3px 8px" }}>
                    {s}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "security" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <button className="panel-btn panel-btn-primary" onClick={fetchSecurity} disabled={loadingSecurity}>
              {loadingSecurity ? "Scanning..." : "Scan"}
            </button>
          </div>
          {errorSecurity && errorBanner(errorSecurity)}
          {loadingSecurity && loadingBanner("Running security scan...")}
          {!loadingSecurity && !errorSecurity && advisories.length === 0 && (
            <div className="panel-empty">No advisories found. Click Scan to check.</div>
          )}
          {advisories.map((a, i) => (
            <div key={i} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                  <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{a.pkg}</span>
                  <span style={badgeStyle(sevColor(a.severity).color, sevColor(a.severity).bg)}>{a.severity}</span>
                </div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{a.cve} - {a.desc}</div>
              </div>
              <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: a.fixAvailable ? "var(--success-color)" : "var(--error-color)" }}>
                {a.fixAvailable ? "Fix available" : "No fix"}
              </span>
            </div>
          ))}
        </div>
      )}

      {tab === "licenses" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <button className="panel-btn panel-btn-primary" onClick={fetchLicenses} disabled={loadingLicenses}>
              {loadingLicenses ? "Checking..." : "Check Licenses"}
            </button>
          </div>
          {errorLicenses && errorBanner(errorLicenses)}
          {loadingLicenses && loadingBanner("Checking licenses...")}
          {!loadingLicenses && !errorLicenses && licenses.length === 0 && (
            <div className="panel-empty">No license data. Click Check Licenses to scan.</div>
          )}
          {licenses.map((l) => (
            <div key={l.pkg} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{l.pkg}</span>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginLeft: 8 }}>{l.license}</span>
              </div>
              <span style={badgeStyle(
                l.status === "compliant" ? "var(--success-color)" : "var(--error-color)",
                l.status === "compliant" ? "var(--success-bg)" : "var(--error-bg)"
              )}>{l.status === "compliant" ? "Compliant" : "Violation"}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
