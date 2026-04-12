import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TeamMember {
  user_id: string;
  name: string;
  email: string;
  sessions: number;
  is_new_member: boolean;
  joined_at: string;
}

interface KnowledgeGap {
  id: string;
  topic: string;
  description: string;
  impact: "low" | "medium" | "high";
  affected_users: string[];
  impact_score: number;
}

interface Hotspot {
  file_path: string;
  access_count: number;
  contributor_count: number;
  complexity: string;
}

export function TeamOnboardingPanel() {
  const [tab, setTab] = useState("members");
  const [members, setMembers] = useState<TeamMember[]>([]);
  const [gaps, setGaps] = useState<KnowledgeGap[]>([]);
  const [guide, setGuide] = useState<string>("");
  const [hotspots, setHotspots] = useState<Hotspot[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedUser, setSelectedUser] = useState("");
  const [loadingGuide, setLoadingGuide] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [membersRes, gapsRes, hotspotsRes] = await Promise.all([
          invoke<TeamMember[]>("team_onboarding_members"),
          invoke<KnowledgeGap[]>("team_onboarding_gaps"),
          invoke<Hotspot[]>("team_onboarding_hotspots"),
        ]);
        const ms = Array.isArray(membersRes) ? membersRes : [];
        setMembers(ms);
        setGaps(Array.isArray(gapsRes) ? gapsRes : []);
        setHotspots(Array.isArray(hotspotsRes) ? hotspotsRes : []);
        if (ms.length > 0) setSelectedUser(ms[0].user_id);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function loadGuide(userId: string) {
    if (!userId) return;
    setLoadingGuide(true);
    try {
      const res = await invoke<string>("team_onboarding_guide", { userId });
      setGuide(typeof res === "string" ? res : "");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingGuide(false);
    }
  }

  useEffect(() => {
    if (tab === "guide" && selectedUser) {
      loadGuide(selectedUser);
    }
  }, [tab, selectedUser]);

  const impactColor = (impact: string) => {
    if (impact === "high") return "var(--error-color)";
    if (impact === "medium") return "var(--warning-color)";
    return "var(--text-muted)";
  };

  const maxAccess = Math.max(...hotspots.map(h => h.access_count), 1);

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: 15, fontWeight: 700, marginBottom: 12 }}>Team Onboarding</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap" }}>
        {["members", "gaps", "guide", "hotspots"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: 6, cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12 }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "members" && (
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)" }}>
                {["User", "Email", "Sessions", "Status", "Joined"].map(h => (
                  <th key={h} style={{ padding: "6px 10px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {members.length === 0 && (
                <tr><td colSpan={5} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No team members found.</td></tr>
              )}
              {members.map(m => (
                <tr key={m.user_id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "6px 10px", fontWeight: 600 }}>{m.name}</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)" }}>{m.email}</td>
                  <td style={{ padding: "6px 10px" }}>{m.sessions}</td>
                  <td style={{ padding: "6px 10px" }}>
                    {m.is_new_member ? (
                      <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 10, background: "var(--accent-color)22", color: "var(--accent-color)", fontWeight: 600 }}>New</span>
                    ) : (
                      <span style={{ fontSize: 11, color: "var(--text-muted)" }}>Member</span>
                    )}
                  </td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{m.joined_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!loading && tab === "gaps" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {gaps.length === 0 && <div style={{ color: "var(--text-muted)" }}>No knowledge gaps identified.</div>}
          {gaps.sort((a, b) => b.impact_score - a.impact_score).map(gap => (
            <div key={gap.id} style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", borderLeft: `3px solid ${impactColor(gap.impact)}`, padding: "12px 14px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontSize: 13, fontWeight: 600 }}>{gap.topic}</span>
                <span style={{ fontSize: 11, padding: "1px 8px", borderRadius: 8, background: impactColor(gap.impact) + "22", color: impactColor(gap.impact), fontWeight: 600 }}>{gap.impact}</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 8 }}>{gap.description}</div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <div style={{ flex: 1, height: 5, background: "var(--bg-primary)", borderRadius: 3 }}>
                  <div style={{ height: "100%", width: `${gap.impact_score}%`, background: impactColor(gap.impact), borderRadius: 3 }} />
                </div>
                <span style={{ fontSize: 11, color: "var(--text-muted)", minWidth: 35 }}>{gap.impact_score}%</span>
              </div>
              {gap.affected_users.length > 0 && (
                <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6 }}>
                  Affects: {gap.affected_users.join(", ")}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {!loading && tab === "guide" && (
        <div>
          <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 14 }}>
            <label style={{ fontSize: 12, color: "var(--text-muted)" }}>User:</label>
            <select value={selectedUser} onChange={e => setSelectedUser(e.target.value)}
              style={{ flex: 1, padding: "5px 10px", borderRadius: 6, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12 }}>
              {members.map(m => <option key={m.user_id} value={m.user_id}>{m.name}</option>)}
            </select>
            <button onClick={() => loadGuide(selectedUser)} disabled={loadingGuide || !selectedUser}
              style={{ padding: "5px 14px", borderRadius: 6, cursor: loadingGuide || !selectedUser ? "not-allowed" : "pointer", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12, opacity: loadingGuide ? 0.6 : 1 }}>
              {loadingGuide ? "Loading…" : "Refresh"}
            </button>
          </div>
          <pre style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", padding: 16, fontSize: 12, lineHeight: 1.7, whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-primary)", margin: 0, minHeight: 200 }}>
            {guide || "Select a user to view their onboarding guide."}
          </pre>
        </div>
      )}

      {!loading && tab === "hotspots" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {hotspots.length === 0 && <div style={{ color: "var(--text-muted)" }}>No hotspots data available.</div>}
          {hotspots.sort((a, b) => b.access_count - a.access_count).map((h, i) => (
            <div key={h.file_path} style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", padding: "10px 14px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontSize: 12, color: "var(--text-muted)", minWidth: 22 }}>#{i + 1}</span>
                <code style={{ fontSize: 12, color: "var(--text-primary)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{h.file_path}</code>
                <span style={{ fontSize: 11, color: "var(--text-muted)", whiteSpace: "nowrap" }}>{h.contributor_count} contributors</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <div style={{ flex: 1, height: 6, background: "var(--bg-primary)", borderRadius: 3 }}>
                  <div style={{ height: "100%", width: `${(h.access_count / maxAccess) * 100}%`, background: "var(--accent-color)", borderRadius: 3, transition: "width 0.3s" }} />
                </div>
                <span style={{ fontSize: 11, color: "var(--text-muted)", minWidth: 60, textAlign: "right" }}>{h.access_count} accesses</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
