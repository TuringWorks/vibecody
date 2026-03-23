/**
 * SessionSharingPanel — Share, annotate, and export agent sessions.
 *
 * Tabs: Shared Sessions, Annotations, Export
 */
import React, { useState } from "react";

type Tab = "Shared Sessions" | "Annotations" | "Export";
const TABS: Tab[] = ["Shared Sessions", "Annotations", "Export"];

const VIS_COLORS: Record<string, string> = {
  Public: "var(--success-color)", Team: "var(--info-color)",
  Private: "var(--text-muted)", "Link Only": "var(--warning-color)",
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
const btnStyle: React.CSSProperties = {
  padding: "6px 14px", background: "var(--accent-color)", color: "var(--bg-primary)",
  border: "none", borderRadius: 4, cursor: "pointer", fontSize: 12, fontFamily: "inherit",
};
const selectStyle: React.CSSProperties = {
  padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 13, fontFamily: "inherit",
  width: "100%", boxSizing: "border-box" as const,
};

const SESSIONS = [
  { id: "sess-a1b2", title: "Auth refactor session", owner: "alice", visibility: "Team", messages: 24, date: "2026-03-19", views: 12 },
  { id: "sess-c3d4", title: "Bug investigation #142", owner: "bob", visibility: "Public", messages: 38, date: "2026-03-18", views: 45 },
  { id: "sess-e5f6", title: "Performance tuning", owner: "carol", visibility: "Link Only", messages: 15, date: "2026-03-17", views: 8 },
  { id: "sess-g7h8", title: "Architecture review", owner: "dave", visibility: "Private", messages: 52, date: "2026-03-16", views: 0 },
];
const ANNOTATIONS = [
  { session: "sess-a1b2", author: "bob", text: "Great approach to middleware injection", line: 12, date: "2026-03-19" },
  { session: "sess-c3d4", author: "alice", text: "Root cause was the unchecked unwrap on line 45", line: 8, date: "2026-03-18" },
  { session: "sess-c3d4", author: "carol", text: "Could also add a regression test here", line: 22, date: "2026-03-18" },
  { session: "sess-e5f6", author: "dave", text: "The caching strategy is spot on", line: 5, date: "2026-03-17" },
];

const FORMATS = ["Markdown", "JSON", "HTML", "PDF"];

const SessionSharingPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Shared Sessions");
  const [exportFormat, setExportFormat] = useState("Markdown");

  return (
    <div style={containerStyle} role="region" aria-label="Session Sharing Panel">
      <div style={tabBarStyle} role="tablist" aria-label="Session Sharing tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Shared Sessions" && SESSIONS.map((s, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{s.title}</strong>
              <span style={badgeStyle(VIS_COLORS[s.visibility] || "var(--text-muted)")}>{s.visibility}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {s.owner} &middot; {s.messages} messages &middot; {s.views} views &middot; {s.date}
            </div>
          </div>
        ))}
        {tab === "Annotations" && ANNOTATIONS.map((a, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{a.author}</strong>
              <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{a.session} line {a.line}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{a.text}</div>
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>{a.date}</div>
          </div>
        ))}
        {tab === "Export" && (
          <div>
            <div style={cardStyle}>
              <label style={{ fontSize: 12, fontWeight: 600, display: "block", marginBottom: 4 }}>Export format</label>
              <select style={selectStyle} value={exportFormat} onChange={e => setExportFormat(e.target.value)} aria-label="Export format">
                {FORMATS.map(f => <option key={f} value={f}>{f}</option>)}
              </select>
            </div>
            <div style={{ ...cardStyle, background: "var(--bg-tertiary)", fontFamily: "var(--font-mono)", fontSize: 12, whiteSpace: "pre-wrap" }}>
              {exportFormat === "Markdown" && "# Session Export\n\n## Auth refactor session\n**Owner:** alice | **Messages:** 24\n\n---\n> Message 1: ...\n> Message 2: ..."}
              {exportFormat === "JSON" && '{\n  "session_id": "sess-a1b2",\n  "title": "Auth refactor session",\n  "messages": [...]\n}'}
              {exportFormat === "HTML" && "<html>\n<body>\n  <h1>Session Export</h1>\n  <div class=\"message\">...</div>\n</body>\n</html>"}
              {exportFormat === "PDF" && "[PDF preview not available - click Export to download]"}
            </div>
            <button style={{ ...btnStyle, marginTop: 8 }} aria-label="Export session">Export</button>
          </div>
        )}
      </div>
    </div>
  );
};

export default SessionSharingPanel;
