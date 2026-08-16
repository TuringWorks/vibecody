import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * One commit author in the open folder's git history.
 *
 * The table used to head these columns "Sessions", "Status" and "Joined", over
 * a commit count, `commits < 5`, and a first-commit date. Open a checkout of
 * someone else's project and it filled with that project's contributors,
 * described as colleagues who had been using the product. Every column now
 * says what it holds, and the table says where the rows came from.
 */
interface Contributor {
  user_id: string;
  name: string;
  email: string;
  commits: number;
  first_commit: string;
}

interface Contributors {
  repo: string;
  contributors: Contributor[];
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
  commits: number;
  contributor_count: number;
}

interface Hotspots {
  repo: string;
  /** How many commits back the scan looked; `commits` is a count within it. */
  scanned_commits: number;
  files: Hotspot[];
}

export function TeamOnboardingPanel() {
  const [tab, setTab] = useState("contributors");
  const [contributors, setContributors] = useState<Contributors | null>(null);
  const [gaps, setGaps] = useState<KnowledgeGap[]>([]);
  const [guide, setGuide] = useState<string>("");
  const [hotspots, setHotspots] = useState<Hotspots | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedUser, setSelectedUser] = useState("");
  const [loadingGuide, setLoadingGuide] = useState(false);
  // Masked by default, and not remembered: revealing is a per-visit decision,
  // so a panel left open on a second monitor does not keep showing addresses
  // because someone clicked once last week.
  const [showEmails, setShowEmails] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [contributorsRes, gapsRes, hotspotsRes] = await Promise.all([
          invoke<Contributors>("team_onboarding_members"),
          invoke<KnowledgeGap[]>("team_onboarding_gaps"),
          invoke<Hotspots>("team_onboarding_hotspots"),
        ]);
        const cs = Array.isArray(contributorsRes?.contributors)
          ? contributorsRes.contributors
          : [];
        setContributors({ repo: contributorsRes?.repo ?? "", contributors: cs });
        setGaps(Array.isArray(gapsRes) ? gapsRes : []);
        setHotspots(
          Array.isArray(hotspotsRes?.files)
            ? hotspotsRes
            : { repo: hotspotsRes?.repo ?? "", scanned_commits: 0, files: [] },
        );
        if (cs.length > 0) setSelectedUser(cs[0].user_id);
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

  /**
   * Hide the local part, keep the domain.
   *
   * These addresses belong to whoever committed to the open folder, which for
   * any checkout of someone else's project is a list of strangers — real names
   * against real personal addresses, on screen by default, in a panel people
   * screenshot. The domain survives because it is the part that carries the
   * signal anyone actually reads a column of addresses for (a colleague at
   * work, a GitHub noreply, an outside contributor); the mailbox does not.
   *
   * Fixed-width, so it does not leak the length of what it hides.
   */
  const maskEmail = (email: string) => {
    const at = email.lastIndexOf("@");
    if (at <= 0) return email ? "•••" : "";
    return `${email[0]}•••${email.slice(at)}`;
  };

  const impactColor = (impact: string) => {
    if (impact === "high") return "var(--error-color)";
    if (impact === "medium") return "var(--warning-color)";
    return "var(--text-muted)";
  };

  const maxCommits = Math.max(...(hotspots?.files ?? []).map(h => h.commits), 1);

  return (
    <div className="panel-container">
      <div className="panel-header"><h3>Team Onboarding</h3></div>
      <div className="panel-tab-bar" style={{ flexWrap: "wrap" }}>
        {["contributors", "gaps", "guide", "hotspots"].map(t => (
          <button className={`panel-tab${tab === t ? " active" : ""}`} key={t} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body">
      {loading && <div className="panel-loading">Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "contributors" && (
        <div style={{ overflowX: "auto" }}>
          {/* Where the rows come from, above the rows. Without this the table
              reads as a roster of people who use this product, which is the
              one thing it has never been. */}
          <div style={{ display: "flex", alignItems: "baseline", gap: 10, marginBottom: 10, flexWrap: "wrap" }}>
            <div style={{ flex: 1, minWidth: 240, fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>
              Commit authors in{" "}
              <code style={{ color: "var(--text-primary)" }}>{contributors?.repo || "the open folder"}</code>
              , from <code>git log</code>. Not product usage.
            </div>
            {(contributors?.contributors.length ?? 0) > 0 && (
              <button
                onClick={() => setShowEmails(v => !v)}
                aria-pressed={showEmails}
                style={{ padding: "2px 10px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: "var(--bg-secondary)", color: "var(--text-muted)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-sm)", whiteSpace: "nowrap" }}
              >
                {showEmails ? "Hide emails" : "Show emails"}
              </button>
            )}
          </div>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)" }}>
                {["Contributor", "Email", "Commits", "First commit"].map(h => (
                  <th key={h} style={{ padding: "8px 12px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {(contributors?.contributors.length ?? 0) === 0 && (
                <tr><td colSpan={4} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No commits found in this folder.</td></tr>
              )}
              {contributors?.contributors.map(c => (
                <tr key={c.user_id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "8px 12px", fontWeight: 600 }}>{c.name}</td>
                  <td style={{ padding: "8px 12px", color: "var(--text-muted)" }}>
                    {showEmails ? c.email : maskEmail(c.email)}
                  </td>
                  <td style={{ padding: "8px 12px" }}>{c.commits}</td>
                  <td style={{ padding: "8px 12px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{c.first_commit}</td>
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
            <div key={gap.id} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", borderLeft: `3px solid ${impactColor(gap.impact)}`, padding: "12px 16px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{gap.topic}</span>
                <span style={{ fontSize: "var(--font-size-sm)", padding: "1px 8px", borderRadius: "var(--radius-sm-alt)", background: impactColor(gap.impact) + "22", color: impactColor(gap.impact), fontWeight: 600 }}>{gap.impact}</span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 8 }}>{gap.description}</div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <div style={{ flex: 1, height: 5, background: "var(--bg-primary)", borderRadius: 3 }}>
                  <div style={{ height: "100%", width: `${gap.impact_score}%`, background: impactColor(gap.impact), borderRadius: 3 }} />
                </div>
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", minWidth: 35 }}>{gap.impact_score}%</span>
              </div>
              {gap.affected_users.length > 0 && (
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginTop: 6 }}>
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
            <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>Contributor:</label>
            <select value={selectedUser} onChange={e => setSelectedUser(e.target.value)}
              style={{ flex: 1, padding: "4px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>
              {contributors?.contributors.map(c => <option key={c.user_id} value={c.user_id}>{c.name}</option>)}
            </select>
            <button onClick={() => loadGuide(selectedUser)} disabled={loadingGuide || !selectedUser}
              style={{ padding: "4px 16px", borderRadius: "var(--radius-sm)", cursor: loadingGuide || !selectedUser ? "not-allowed" : "pointer", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", opacity: loadingGuide ? 0.6 : 1 }}>
              {loadingGuide ? "Loading…" : "Refresh"}
            </button>
          </div>
          <pre style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: 16, fontSize: "var(--font-size-base)", lineHeight: 1.7, whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-primary)", margin: 0, minHeight: 200 }}>
            {guide || "Select a user to view their onboarding guide."}
          </pre>
        </div>
      )}

      {!loading && tab === "hotspots" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {/* A ranking read without its window is misread: these are the most
              changed files *recently*, not of all time. */}
          {(hotspots?.files.length ?? 0) > 0 && (
            <div style={{ marginBottom: 2, fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>
              Most-changed files in the last {hotspots?.scanned_commits.toLocaleString()} commits of{" "}
              <code style={{ color: "var(--text-primary)" }}>{hotspots?.repo}</code>.
            </div>
          )}
          {(hotspots?.files.length ?? 0) === 0 && <div style={{ color: "var(--text-muted)" }}>No commits found in this folder.</div>}
          {[...(hotspots?.files ?? [])].sort((a, b) => b.commits - a.commits).map((h, i) => (
            <div key={h.file_path} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: "12px 16px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", minWidth: 22 }}>#{i + 1}</span>
                <code style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{h.file_path}</code>
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{h.contributor_count} contributors</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <div style={{ flex: 1, height: 6, background: "var(--bg-primary)", borderRadius: 3 }}>
                  <div style={{ height: "100%", width: `${(h.commits / maxCommits) * 100}%`, background: "var(--accent-color)", borderRadius: 3, transition: "width 0.3s" }} />
                </div>
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", minWidth: 60, textAlign: "right" }}>{h.commits} commits</span>
              </div>
            </div>
          ))}
        </div>
      )}
      </div>
    </div>
  );
}
