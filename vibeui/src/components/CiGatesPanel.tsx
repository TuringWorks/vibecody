/**
 * CiGatesPanel — CI quality gates: rules, check reports, and GitHub Action YAML generation.
 *
 * Tabs: Rules, Reports, GitHub Action
 */
import React, { useState } from "react";

type Tab = "Rules" | "Reports" | "GitHub Action";
const TABS: Tab[] = ["Rules", "Reports", "GitHub Action"];

const STATUS_COLORS: Record<string, string> = {
  Pass: "var(--success-color)", Fail: "var(--error-color)",
  Warn: "var(--warning-color)", Skip: "var(--text-muted)",
};

const containerStyle: React.CSSProperties = {
  display: "flex", flexDirection: "column", height: "100%",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  fontFamily: "inherit", overflow: "hidden",
};
const tabBarStyle: React.CSSProperties = {
  display: "flex", gap: 2, padding: "8px 12px 0",
  borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)",
  overflowX: "auto", flexShrink: 0,
};
const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px", cursor: "pointer",
  background: active ? "var(--bg-primary)" : "transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  border: "none", borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  fontSize: 13, fontFamily: "inherit", whiteSpace: "nowrap",
});
const contentStyle: React.CSSProperties = { flex: 1, overflow: "auto", padding: 16 };
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 8,
  border: "1px solid var(--border-color)",
};
const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: 10,
  fontSize: 11, background: color, color: "var(--bg-primary)", fontWeight: 600,
});

const RULES = [
  { name: "Test coverage >= 80%", category: "Quality", enabled: true, severity: "Fail" },
  { name: "No security vulnerabilities (High/Critical)", category: "Security", enabled: true, severity: "Fail" },
  { name: "Lint passes with 0 errors", category: "Style", enabled: true, severity: "Fail" },
  { name: "Build completes in < 10 min", category: "Performance", enabled: true, severity: "Warn" },
  { name: "No TODO comments in new code", category: "Style", enabled: false, severity: "Warn" },
  { name: "Docs updated for public API changes", category: "Docs", enabled: true, severity: "Warn" },
  { name: "License headers present", category: "Compliance", enabled: false, severity: "Skip" },
];

const REPORTS = [
  { run: "#142", date: "2026-03-19", branch: "feat/auth-v2", checks: [
    { name: "Test coverage", result: "Pass", detail: "84.2%" },
    { name: "Security scan", result: "Pass", detail: "0 vulnerabilities" },
    { name: "Lint", result: "Fail", detail: "3 errors in routes.ts" },
    { name: "Build time", result: "Pass", detail: "4m 12s" },
  ]},
  { run: "#141", date: "2026-03-18", branch: "fix/perf-regression", checks: [
    { name: "Test coverage", result: "Pass", detail: "81.0%" },
    { name: "Security scan", result: "Pass", detail: "0 vulnerabilities" },
    { name: "Lint", result: "Pass", detail: "0 errors" },
    { name: "Build time", result: "Warn", detail: "9m 48s" },
  ]},
];

const GH_ACTION_YAML = `name: VibeCody CI Gates
on:
  pull_request:
    branches: [main]

jobs:
  quality-gates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests with coverage
        run: cargo test --workspace -- --nocapture
      - name: Check coverage threshold
        run: cargo tarpaulin --out Xml --fail-under 80
      - name: Security audit
        run: cargo audit
      - name: Lint check
        run: cargo clippy -- -D warnings
      - name: Build timing
        run: time cargo build --release`;

const CiGatesPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Rules");
  return (
    <div style={containerStyle} role="region" aria-label="CI Gates Panel">
      <div style={tabBarStyle} role="tablist" aria-label="CI Gates tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Rules" && RULES.map((r, i) => (
          <div key={i} style={{ ...cardStyle, opacity: r.enabled ? 1 : 0.5 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{r.name}</strong>
                <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 2 }}>Category: {r.category}</div>
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <span style={badgeStyle(STATUS_COLORS[r.severity] || "var(--text-muted)")}>{r.severity}</span>
                <span style={{ fontSize: 11, color: r.enabled ? "var(--success-color)" : "var(--text-muted)" }}>{r.enabled ? "ON" : "OFF"}</span>
              </div>
            </div>
          </div>
        ))}
        {tab === "Reports" && REPORTS.map((r, i) => (
          <div key={i} style={{ marginBottom: 16 }}>
            <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Run {r.run} &middot; {r.branch} &middot; {r.date}</div>
            {r.checks.map((c, j) => (
              <div key={j} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span>{c.name}</span>
                  <span style={badgeStyle(STATUS_COLORS[c.result] || "var(--text-muted)")}>{c.result}</span>
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{c.detail}</div>
              </div>
            ))}
          </div>
        ))}
        {tab === "GitHub Action" && (
          <div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>
              Generated GitHub Actions workflow for your CI gates configuration:
            </div>
            <pre style={{
              background: "var(--bg-tertiary)", padding: 12, borderRadius: 6, fontSize: 12,
              fontFamily: "var(--font-mono)", overflow: "auto", border: "1px solid var(--border-color)",
              whiteSpace: "pre-wrap", color: "var(--text-primary)",
            }}>{GH_ACTION_YAML}</pre>
          </div>
        )}
      </div>
    </div>
  );
};

export default CiGatesPanel;
