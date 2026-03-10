import React, { useState } from "react";

// -- Types --------------------------------------------------------------------

type SessionStatus = "Active" | "Completed" | "Failed" | "Paused";
type TabName = "Sessions" | "Replay" | "Stats";

interface Session {
  id: string;
  name: string;
  provider: string;
  model: string;
  status: SessionStatus;
  messageCount: number;
  startedAt: string;
  duration: string;
}

interface ReplayStep {
  index: number;
  role: "user" | "assistant" | "tool";
  content: string;
  timestamp: string;
  tokenCount: number;
}

interface ProviderStat {
  provider: string;
  count: number;
  pct: number;
}

// -- Mock Data ----------------------------------------------------------------

const MOCK_SESSIONS: Session[] = [
  { id: "sess-a1b2c3", name: "Refactor auth module", provider: "Claude", model: "opus-4", status: "Completed", messageCount: 24, startedAt: "2026-03-09 10:15", duration: "12m" },
  { id: "sess-d4e5f6", name: "Fix CI pipeline", provider: "OpenAI", model: "gpt-4o", status: "Active", messageCount: 8, startedAt: "2026-03-09 14:30", duration: "3m" },
  { id: "sess-g7h8i9", name: "Add unit tests", provider: "Gemini", model: "gemini-2.0", status: "Failed", messageCount: 15, startedAt: "2026-03-08 09:00", duration: "7m" },
  { id: "sess-j0k1l2", name: "Database migration", provider: "Claude", model: "sonnet-4", status: "Paused", messageCount: 31, startedAt: "2026-03-07 16:45", duration: "22m" },
  { id: "sess-m3n4o5", name: "API endpoint design", provider: "Ollama", model: "llama3", status: "Completed", messageCount: 12, startedAt: "2026-03-07 11:20", duration: "5m" },
];

const MOCK_REPLAY_STEPS: ReplayStep[] = [
  { index: 0, role: "user", content: "Refactor the auth module to use JWT tokens", timestamp: "10:15:00", tokenCount: 18 },
  { index: 1, role: "assistant", content: "I'll analyze the current auth module and plan the refactoring...", timestamp: "10:15:02", tokenCount: 342 },
  { index: 2, role: "tool", content: "Read file: src/auth/mod.rs (245 lines)", timestamp: "10:15:03", tokenCount: 1200 },
  { index: 3, role: "assistant", content: "Here's my plan for the JWT refactoring:\n1. Add jsonwebtoken dependency\n2. Create JwtConfig struct\n3. Update login handler", timestamp: "10:15:08", tokenCount: 580 },
  { index: 4, role: "user", content: "Looks good, proceed with the implementation", timestamp: "10:16:30", tokenCount: 9 },
  { index: 5, role: "assistant", content: "I'll start by updating Cargo.toml and creating the JWT module...", timestamp: "10:16:32", tokenCount: 890 },
];

const MOCK_PROVIDER_STATS: ProviderStat[] = [
  { provider: "Claude", count: 42, pct: 45 },
  { provider: "OpenAI", count: 28, pct: 30 },
  { provider: "Gemini", count: 12, pct: 13 },
  { provider: "Ollama", count: 8, pct: 9 },
  { provider: "Groq", count: 3, pct: 3 },
];

// -- Helpers ------------------------------------------------------------------

const statusColor = (s: SessionStatus): string => {
  switch (s) {
    case "Active": return "var(--vscode-charts-green, #4caf50)";
    case "Completed": return "var(--vscode-charts-blue, #007acc)";
    case "Failed": return "var(--vscode-errorForeground, #f44336)";
    case "Paused": return "var(--vscode-charts-yellow, #ff9800)";
  }
};

const roleColor = (r: string): string => {
  switch (r) {
    case "user": return "var(--vscode-charts-blue, #007acc)";
    case "assistant": return "var(--vscode-charts-green, #4caf50)";
    case "tool": return "var(--vscode-charts-yellow, #ff9800)";
    default: return "var(--vscode-foreground, #ccc)";
  }
};

// -- Component ----------------------------------------------------------------

const SessionBrowserPanel: React.FC = () => {
  const [tab, setTab] = useState<TabName>("Sessions");
  const [search, setSearch] = useState("");
  const [selectedSession, setSelectedSession] = useState<Session | null>(null);
  const [replayIndex, setReplayIndex] = useState(0);

  const filteredSessions = MOCK_SESSIONS.filter(
    (s) => s.name.toLowerCase().includes(search.toLowerCase()) || s.provider.toLowerCase().includes(search.toLowerCase())
  );

  const tabs: TabName[] = ["Sessions", "Replay", "Stats"];

  return (
    <div style={{ padding: 12, fontFamily: "var(--vscode-font-family, sans-serif)", fontSize: 13, height: "100%", overflowY: "auto", color: "var(--vscode-foreground, #ccc)", background: "var(--vscode-editor-background, #1e1e1e)" }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>Session Browser</div>

      {/* Tab bar */}
      <div style={{ display: "flex", gap: 0, marginBottom: 12, borderBottom: "1px solid var(--vscode-panel-border, #444)" }}>
        {tabs.map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "6px 16px", fontSize: 12, background: "none", border: "none", borderBottom: tab === t ? "2px solid var(--vscode-focusBorder, #007acc)" : "2px solid transparent", color: tab === t ? "var(--vscode-foreground, #fff)" : "var(--vscode-disabledForeground, #888)", cursor: "pointer", fontWeight: tab === t ? 600 : 400 }}>
            {t}
          </button>
        ))}
      </div>

      {/* Sessions Tab */}
      {tab === "Sessions" && (
        <div>
          <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search sessions..." style={{ width: "100%", padding: "6px 10px", fontSize: 12, background: "var(--vscode-input-background, #333)", color: "var(--vscode-input-foreground, #fff)", border: "1px solid var(--vscode-input-border, #555)", borderRadius: 4, marginBottom: 10, boxSizing: "border-box" }} />
          {filteredSessions.map((s) => (
            <div key={s.id} onClick={() => { setSelectedSession(s); setTab("Replay"); setReplayIndex(0); }} style={{ padding: "8px 10px", marginBottom: 6, borderRadius: 4, background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", cursor: "pointer", borderLeft: `3px solid ${statusColor(s.status)}` }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontWeight: 600, fontSize: 12 }}>{s.name}</span>
                <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 10, background: statusColor(s.status), color: "#fff", fontWeight: 600 }}>{s.status}</span>
              </div>
              <div style={{ display: "flex", gap: 12, marginTop: 4, fontSize: 11, color: "var(--vscode-disabledForeground, #888)" }}>
                <span>{s.provider} / {s.model}</span>
                <span>{s.messageCount} msgs</span>
                <span>{s.duration}</span>
                <span style={{ marginLeft: "auto" }}>{s.startedAt}</span>
              </div>
            </div>
          ))}
          {filteredSessions.length === 0 && (
            <div style={{ textAlign: "center", padding: 30, color: "var(--vscode-disabledForeground, #888)" }}>No sessions match your search.</div>
          )}
        </div>
      )}

      {/* Replay Tab */}
      {tab === "Replay" && (
        <div>
          {selectedSession ? (
            <>
              <div style={{ marginBottom: 10, fontSize: 12, color: "var(--vscode-disabledForeground, #888)" }}>
                Replaying: <strong style={{ color: "var(--vscode-foreground, #fff)" }}>{selectedSession.name}</strong> ({selectedSession.id})
              </div>
              <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
                <button onClick={() => setReplayIndex(Math.max(0, replayIndex - 1))} disabled={replayIndex === 0} style={{ padding: "4px 12px", fontSize: 11, borderRadius: 4, border: "1px solid var(--vscode-panel-border, #555)", background: "none", color: "var(--vscode-foreground, #ccc)", cursor: replayIndex === 0 ? "not-allowed" : "pointer" }}>Prev</button>
                <button onClick={() => setReplayIndex(Math.min(MOCK_REPLAY_STEPS.length - 1, replayIndex + 1))} disabled={replayIndex >= MOCK_REPLAY_STEPS.length - 1} style={{ padding: "4px 12px", fontSize: 11, borderRadius: 4, border: "1px solid var(--vscode-panel-border, #555)", background: "none", color: "var(--vscode-foreground, #ccc)", cursor: replayIndex >= MOCK_REPLAY_STEPS.length - 1 ? "not-allowed" : "pointer" }}>Next</button>
                <span style={{ fontSize: 11, color: "var(--vscode-disabledForeground, #888)", lineHeight: "28px" }}>Step {replayIndex + 1} / {MOCK_REPLAY_STEPS.length}</span>
                <button onClick={() => alert("Snapshot saved")} style={{ marginLeft: "auto", padding: "4px 12px", fontSize: 11, borderRadius: 4, border: "none", background: "var(--vscode-button-background, #007acc)", color: "var(--vscode-button-foreground, #fff)", cursor: "pointer" }}>Snapshot</button>
              </div>
              {MOCK_REPLAY_STEPS.slice(0, replayIndex + 1).map((step) => (
                <div key={step.index} style={{ padding: "8px 10px", marginBottom: 6, borderRadius: 4, background: step.index === replayIndex ? "var(--vscode-editor-selectionBackground, #264f78)" : "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", borderLeft: `3px solid ${roleColor(step.role)}` }}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, marginBottom: 4 }}>
                    <span style={{ fontWeight: 600, color: roleColor(step.role), textTransform: "capitalize" }}>{step.role}</span>
                    <span style={{ color: "var(--vscode-disabledForeground, #888)" }}>{step.timestamp} - {step.tokenCount} tokens</span>
                  </div>
                  <div style={{ fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.5 }}>{step.content}</div>
                </div>
              ))}
            </>
          ) : (
            <div style={{ textAlign: "center", padding: 30, color: "var(--vscode-disabledForeground, #888)" }}>Select a session from the Sessions tab to replay it.</div>
          )}
        </div>
      )}

      {/* Stats Tab */}
      {tab === "Stats" && (
        <div>
          <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 16 }}>
            {[
              { label: "Total Sessions", value: String(MOCK_SESSIONS.length) },
              { label: "Completed", value: String(MOCK_SESSIONS.filter((s) => s.status === "Completed").length) },
              { label: "Acceptance Rate", value: "87%" },
            ].map(({ label, value }) => (
              <div key={label} style={{ background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", padding: "10px 16px", borderRadius: 6, textAlign: "center", minWidth: 90 }}>
                <div style={{ fontSize: 20, fontWeight: 700, color: "var(--vscode-focusBorder, #007acc)" }}>{value}</div>
                <div style={{ fontSize: 11, color: "var(--vscode-disabledForeground, #888)", marginTop: 2 }}>{label}</div>
              </div>
            ))}
          </div>
          <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Sessions by Provider</div>
          {MOCK_PROVIDER_STATS.map((p) => (
            <div key={p.provider} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
              <span style={{ minWidth: 60, fontSize: 12 }}>{p.provider}</span>
              <div style={{ flex: 1, background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", borderRadius: 3, height: 10, overflow: "hidden" }}>
                <div style={{ width: `${p.pct}%`, height: "100%", background: "var(--vscode-focusBorder, #007acc)", borderRadius: 3 }} />
              </div>
              <span style={{ minWidth: 30, textAlign: "right", fontSize: 11, color: "var(--vscode-disabledForeground, #888)" }}>{p.count}</span>
              <span style={{ minWidth: 35, textAlign: "right", fontSize: 11, color: "var(--vscode-disabledForeground, #888)" }}>{p.pct}%</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SessionBrowserPanel;
