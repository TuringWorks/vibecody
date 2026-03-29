import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "var(--btn-primary-fg, #fff)",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const badgeStyle = (color: string, bg: string): React.CSSProperties => ({
  padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600, color, background: bg,
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
    if (s === "high") return { color: "var(--error-color)", bg: "#ef444420" };
    if (s === "medium") return { color: "var(--warning-color)", bg: "#eab30820" };
    return { color: "var(--accent-color)", bg: "#3b82f620" };
  };

  const mgrColor = (m: string) => {
    if (m === "npm") return { color: "#cb3837", bg: "#cb383720" };
    if (m === "cargo") return { color: "var(--warning-color)", bg: "#f59e0b20" };
    return { color: "var(--accent-color)", bg: "#3b82f620" };
  };

  const errorBanner = (msg: string) => (
    <div style={{ ...cardStyle, color: "var(--error-color)", borderColor: "var(--error-color)" }}>
      Error: {msg}
    </div>
  );

  const loadingBanner = (msg: string) => (
    <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>
      {msg}
    </div>
  );

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Agentic Package Manager</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["dependencies", "conflicts", "security", "licenses"].map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "dependencies" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <button style={btnStyle} onClick={fetchAnalysis} disabled={loadingAnalysis}>
              {loadingAnalysis ? "Analyzing..." : "Analyze"}
            </button>
          </div>
          {errorAnalysis && errorBanner(errorAnalysis)}
          {loadingAnalysis && loadingBanner("Analyzing dependencies...")}
          {!loadingAnalysis && !errorAnalysis && deps.length === 0 && (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>
              No dependencies found. Click Analyze to scan.
            </div>
          )}
          {deps.map((d) => (
            <div key={d.name} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{d.name}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{d.version}</span>
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
            <button style={btnStyle} onClick={fetchAnalysis} disabled={loadingAnalysis}>
              {loadingAnalysis ? "Analyzing..." : "Refresh"}
            </button>
          </div>
          {errorAnalysis && errorBanner(errorAnalysis)}
          {loadingAnalysis && loadingBanner("Checking conflicts...")}
          {!loadingAnalysis && !errorAnalysis && conflicts.length === 0 && (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>
              No conflicts detected.
            </div>
          )}
          {conflicts.map((c) => (
            <div key={c.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{c.pkg}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{c.versions.join(" vs ")}</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>{c.reason}</div>
              <div style={{ display: "flex", gap: 4 }}>
                {["dedupe", "upgrade", "align", "ignore"].map((s) => (
                  <button key={s} onClick={() => setConflicts((prev) => prev.map((x) => x.id === c.id ? { ...x, strategy: s } : x))}
                    style={{ ...btnStyle, fontSize: 11, padding: "3px 8px", background: c.strategy === s ? "var(--accent-color)" : "transparent", color: c.strategy === s ? "var(--btn-primary-fg, #fff)" : "var(--text-primary)", border: "1px solid var(--border-color)" }}>
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
            <button style={btnStyle} onClick={fetchSecurity} disabled={loadingSecurity}>
              {loadingSecurity ? "Scanning..." : "Scan"}
            </button>
          </div>
          {errorSecurity && errorBanner(errorSecurity)}
          {loadingSecurity && loadingBanner("Running security scan...")}
          {!loadingSecurity && !errorSecurity && advisories.length === 0 && (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>
              No advisories found. Click Scan to check.
            </div>
          )}
          {advisories.map((a, i) => (
            <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                  <span style={{ fontWeight: 600, fontSize: 13 }}>{a.pkg}</span>
                  <span style={badgeStyle(sevColor(a.severity).color, sevColor(a.severity).bg)}>{a.severity}</span>
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{a.cve} - {a.desc}</div>
              </div>
              <span style={{ fontSize: 11, fontWeight: 600, color: a.fixAvailable ? "var(--success-color)" : "var(--error-color)" }}>
                {a.fixAvailable ? "Fix available" : "No fix"}
              </span>
            </div>
          ))}
        </div>
      )}

      {tab === "licenses" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <button style={btnStyle} onClick={fetchLicenses} disabled={loadingLicenses}>
              {loadingLicenses ? "Checking..." : "Check Licenses"}
            </button>
          </div>
          {errorLicenses && errorBanner(errorLicenses)}
          {loadingLicenses && loadingBanner("Checking licenses...")}
          {!loadingLicenses && !errorLicenses && licenses.length === 0 && (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>
              No license data. Click Check Licenses to scan.
            </div>
          )}
          {licenses.map((l) => (
            <div key={l.pkg} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{l.pkg}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 8 }}>{l.license}</span>
              </div>
              <span style={badgeStyle(
                l.status === "compliant" ? "var(--success-color)" : "var(--error-color)",
                l.status === "compliant" ? "#22c55e20" : "#ef444420"
              )}>{l.status === "compliant" ? "Compliant" : "Violation"}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
