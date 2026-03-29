/**
 * CiGatesPanel — CI quality gates: rules, check reports, and GitHub Action YAML generation.
 *
 * Tabs: Rules, Reports, GitHub Action
 */
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Rules" | "Reports" | "GitHub Action";
const TABS: Tab[] = ["Rules", "Reports", "GitHub Action"];

const STATUS_COLORS: Record<string, string> = {
  Pass: "var(--success-color)", Fail: "var(--error-color)",
  Warn: "var(--warning-color)", Skip: "var(--text-secondary)",
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
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  border: "none", borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
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

interface Gate { name: string; category: string; enabled: boolean; severity: string }

const CiGatesPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Rules");
  const [gates, setGates] = useState<Gate[]>([]);

  useEffect(() => {
    invoke<Gate[]>("list_ci_gates").then(setGates).catch(() => {});
  }, []);

  const handleToggle = async (name: string) => {
    try {
      const updated = await invoke<Gate>("toggle_ci_gate", { name });
      setGates(prev => prev.map(g => g.name === name ? updated : g));
    } catch (_) { /* ignore */ }
  };

  return (
    <div style={containerStyle} role="region" aria-label="CI Gates Panel">
      <div style={tabBarStyle} role="tablist" aria-label="CI Gates tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Rules" && gates.map((r, i) => (
          <div key={i} style={{ ...cardStyle, opacity: r.enabled ? 1 : 0.5 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{r.name}</strong>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>Category: {r.category}</div>
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <span style={badgeStyle(STATUS_COLORS[r.severity] || "var(--text-secondary)")}>{r.severity}</span>
                <button
                  style={{ fontSize: 11, color: r.enabled ? "var(--success-color)" : "var(--text-secondary)", background: "none", border: "none", cursor: "pointer" }}
                  onClick={() => handleToggle(r.name)}
                >{r.enabled ? "ON" : "OFF"}</button>
              </div>
            </div>
          </div>
        ))}
        {tab === "Reports" && (
          <div style={{ textAlign: "center", padding: 40, color: "var(--text-secondary)" }}>
            <div style={{ fontSize: 14 }}>No CI reports yet</div>
            <div style={{ fontSize: 12, marginTop: 4 }}>Reports will appear after CI runs complete</div>
          </div>
        )}
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
