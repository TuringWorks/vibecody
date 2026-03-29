import { useState } from "react";

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

export function SmartDepsPanel() {
  const [tab, setTab] = useState("dependencies");
  const [conflicts, setConflicts] = useState([
    { id: 1, pkg: "lodash", versions: ["4.17.21", "3.10.1"], reason: "Transitive via legacy-lib", strategy: "dedupe" },
    { id: 2, pkg: "react", versions: ["18.3.0", "17.0.2"], reason: "Peer dep mismatch", strategy: "upgrade" },
    { id: 3, pkg: "typescript", versions: ["5.4.0", "5.2.0"], reason: "Workspace mismatch", strategy: "align" },
  ]);

  const deps = [
    { name: "react", version: "18.3.0", manager: "npm", dev: false },
    { name: "typescript", version: "5.4.0", manager: "npm", dev: true },
    { name: "serde", version: "1.0.197", manager: "cargo", dev: false },
    { name: "tokio", version: "1.36.0", manager: "cargo", dev: false },
    { name: "tailwindcss", version: "3.4.1", manager: "npm", dev: true },
    { name: "eslint", version: "9.1.0", manager: "npm", dev: true },
    { name: "clap", version: "4.5.2", manager: "cargo", dev: false },
    { name: "pytest", version: "8.1.1", manager: "pip", dev: true },
  ];

  const advisories = [
    { pkg: "lodash", severity: "high", cve: "CVE-2024-1234", desc: "Prototype pollution", fixAvailable: true },
    { pkg: "express", severity: "medium", cve: "CVE-2024-5678", desc: "Open redirect", fixAvailable: true },
    { pkg: "tar", severity: "critical", cve: "CVE-2024-9012", desc: "Path traversal", fixAvailable: false },
    { pkg: "node-fetch", severity: "low", cve: "CVE-2024-3456", desc: "SSRF via redirect", fixAvailable: true },
  ];

  const licenses = [
    { pkg: "react", license: "MIT", status: "compliant" },
    { pkg: "serde", license: "MIT/Apache-2.0", status: "compliant" },
    { pkg: "ffmpeg-sys", license: "LGPL-2.1", status: "violation" },
    { pkg: "openssl", license: "Apache-2.0", status: "compliant" },
    { pkg: "ghostscript", license: "AGPL-3.0", status: "violation" },
    { pkg: "zlib", license: "Zlib", status: "compliant" },
  ];

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
