import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useModelRegistry } from "../hooks/useModelRegistry";

// ── Types ────────────────────────────────────────────────────────────────────

interface Participant {
  provider: string;
  model: string;
  role: string;
  persona?: string;
}

interface CounselResponse {
  participant_index: number;
  content: string;
  duration_ms: number;
  tokens?: number;
  votes: number;
}

interface CounselRound {
  round_number: number;
  responses: CounselResponse[];
  user_interjection?: string;
}

interface CounselSession {
  id: string;
  topic: string;
  participants: Participant[];
  rounds: CounselRound[];
  moderator_index: number;
  status: string;
  synthesis?: string;
}

interface SessionSummary {
  id: string;
  topic: string;
  status: string;
  round_count: number;
  participant_count: number;
}

// ── Constants ────────────────────────────────────────────────────────────────

const ROLES = ["Expert", "Devil's Advocate", "Skeptic", "Creative", "Pragmatist", "Researcher", "Custom"];
// PROVIDERS now loaded dynamically via useModelRegistry hook

const ROLE_COLORS: Record<string, string> = {
  Expert: "var(--accent-blue)",
  "Devil's Advocate": "#ff4a4a",
  Skeptic: "#ff9f43",
  Creative: "#b94aff",
  Pragmatist: "#4aff7f",
  Researcher: "#4adcff",
  Custom: "var(--text-secondary)",
};

const DEFAULT_PARTICIPANTS: Participant[] = [
  { provider: "claude", model: "claude-sonnet-4-6", role: "Expert" },
  { provider: "openai", model: "gpt-4o", role: "Skeptic" },
  { provider: "gemini", model: "gemini-2.5-flash", role: "Creative" },
];

// ── Styles ───────────────────────────────────────────────────────────────────

const S = {
  container: { display: "flex", flex: 1, minHeight: 0, fontFamily: "var(--font-family, system-ui, sans-serif)", color: "var(--text-primary)", background: "var(--bg-primary)" } as const,
  sidebar: { width: 220, borderRight: "1px solid var(--border-color)", padding: 12, overflowY: "auto", flexShrink: 0 } as const,
  main: { flex: 1, padding: 20, overflowY: "auto" } as const,
  btn: { padding: "8px 16px", border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13, fontWeight: 600, background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)" } as const,
  btnSecondary: { padding: "6px 12px", border: "1px solid var(--border-color)", borderRadius: 6, cursor: "pointer", fontSize: 12, background: "transparent", color: "var(--text-primary)" } as const,
  input: { width: "100%", padding: "8px 10px", borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 13, boxSizing: "border-box" } as const,
  textarea: { width: "100%", padding: "10px 12px", borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 13, resize: "vertical", minHeight: 80, boxSizing: "border-box", fontFamily: "inherit" } as const,
  select: { padding: "6px 8px", borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 13 } as const,
  card: { border: "1px solid var(--border-color)", borderRadius: 8, padding: 14, marginBottom: 12, background: "var(--bg-secondary)" } as const,
  badge: (color: string) => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 11, fontWeight: 600, background: color + "22", color, marginRight: 6 }),
  sidebarItem: (active: boolean) => ({ padding: "8px 10px", borderRadius: 6, cursor: "pointer", fontSize: 12, marginBottom: 4, background: active ? "var(--accent-blue)22" : "transparent", color: active ? "var(--accent-blue)" : "var(--text-secondary)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" } as const),
  h2: { fontSize: 16, fontWeight: 700, margin: "0 0 16px 0" } as const,
  h3: { fontSize: 14, fontWeight: 600, margin: "16px 0 8px 0" } as const,
  label: { fontSize: 12, color: "var(--text-secondary)", marginBottom: 4, display: "block" } as const,
  grid: { display: "grid", gap: 12 } as const,
  voteBtn: { padding: "2px 6px", border: "1px solid var(--border-color)", borderRadius: 4, cursor: "pointer", fontSize: 12, background: "transparent", color: "var(--text-secondary)", lineHeight: 1 } as const,
  synthesisCard: { border: "2px solid var(--accent-blue)", borderRadius: 8, padding: 16, background: "var(--accent-blue)08", marginBottom: 16 } as const,
};

// ── Component ────────────────────────────────────────────────────────────────

export function CounselPanel() {
  const { providers, modelsForProvider } = useModelRegistry();
  const [sessionList, setSessionList] = useState<SessionSummary[]>([]);
  const [activeSession, setActiveSession] = useState<CounselSession | null>(null);
  const [showSetup, setShowSetup] = useState(true);
  const [selectedParticipants, setSelectedParticipants] = useState<Set<number>>(new Set());

  // Setup state
  const [topic, setTopic] = useState("");
  const [participants, setParticipants] = useState<Participant[]>([...DEFAULT_PARTICIPANTS]);
  const [moderatorIdx, setModeratorIdx] = useState(0);

  // Toggle participant selection
  const toggleParticipantSelection = (idx: number) => {
    setSelectedParticipants(prev => {
      const next = new Set(prev);
      if (next.has(idx)) next.delete(idx); else next.add(idx);
      return next;
    });
  };
  const removeSelectedParticipants = () => {
    setParticipants(prev => prev.filter((_, i) => !selectedParticipants.has(i)));
    setSelectedParticipants(new Set());
    if (selectedParticipants.has(moderatorIdx)) setModeratorIdx(0);
  };

  // Runtime state
  const [deliberating, setDeliberating] = useState(false);
  const [synthesizing, setSynthesizing] = useState(false);
  const [interjection, setInterjection] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Load sessions on mount
  useEffect(() => {
    loadSessions();
  }, []);

  // Listen for counsel events
  useEffect(() => {
    const unlisten = listen("counsel:chunk", () => {});
    return () => { unlisten.then(f => f()); };
  }, []);

  const loadSessions = useCallback(async () => {
    try {
      const list = await invoke<SessionSummary[]>("counsel_list_sessions");
      setSessionList(Array.isArray(list) ? list : []);
    } catch { /* ignore */ }
  }, []);

  const loadSession = useCallback(async (id: string) => {
    try {
      const s = await invoke<CounselSession>("counsel_get_session", { sessionId: id });
      setActiveSession(s);
      setShowSetup(false);
    } catch { /* ignore */ }
  }, []);

  const startSession = useCallback(async () => {
    if (!topic.trim() || participants.length === 0) return;
    setError(null);
    try {
      const s = await invoke<CounselSession>("counsel_create_session", {
        topic: topic.trim(),
        participants: participants.map(p => ({
          provider: p.provider,
          model: p.model,
          role: p.role,
          persona: p.persona || undefined,
        })),
        moderatorIdx: moderatorIdx,
      });
      setActiveSession(s);
      setShowSetup(false);
      loadSessions();
    } catch (e: any) {
      setError(`Failed to create session: ${e?.toString() || "unknown error"}`);
    }
  }, [topic, participants, moderatorIdx, loadSessions]);

  const runRound = useCallback(async () => {
    if (!activeSession) return;
    setDeliberating(true);
    try {
      await invoke("counsel_run_round", { sessionId: activeSession.id });
      await loadSession(activeSession.id);
    } catch (e: any) {
      setError(`Round failed: ${e?.toString() || "unknown error"}`);
    } finally {
      setDeliberating(false);
    }
  }, [activeSession, loadSession]);

  const synthesize = useCallback(async () => {
    if (!activeSession) return;
    setSynthesizing(true);
    try {
      await invoke("counsel_synthesize", { sessionId: activeSession.id });
      await loadSession(activeSession.id);
    } catch (e: any) {
      setError(`Synthesis failed: ${e?.toString() || "unknown error"}`);
    } finally {
      setSynthesizing(false);
    }
  }, [activeSession, loadSession]);

  const injectMessage = useCallback(async () => {
    if (!activeSession || !interjection.trim()) return;
    try {
      await invoke("counsel_inject_message", { sessionId: activeSession.id, message: interjection.trim() });
      setInterjection("");
      await loadSession(activeSession.id);
    } catch { /* ignore */ }
  }, [activeSession, interjection, loadSession]);

  const vote = useCallback(async (roundIdx: number, participantIdx: number, delta: number) => {
    if (!activeSession) return;
    try {
      await invoke("counsel_vote", { sessionId: activeSession.id, roundIdx, participantIdx, delta });
      await loadSession(activeSession.id);
    } catch { /* ignore */ }
  }, [activeSession, loadSession]);

  const addParticipant = () => {
    setParticipants(prev => [...prev, { provider: "ollama", model: "llama3.2", role: "Expert" }]);
  };

  const removeParticipant = (idx: number) => {
    setParticipants(prev => {
      const next = prev.filter((_, i) => i !== idx);
      if (moderatorIdx >= next.length) setModeratorIdx(Math.max(0, next.length - 1));
      return next;
    });
  };

  const updateParticipant = (idx: number, field: keyof Participant, value: string) => {
    setParticipants(prev => prev.map((p, i) => i === idx ? { ...p, [field]: value } : p));
  };

  const deleteSession = useCallback(async (id: string) => {
    try {
      await invoke("counsel_delete_session", { sessionId: id });
      if (activeSession?.id === id) {
        setActiveSession(null);
        setShowSetup(true);
      }
      loadSessions();
    } catch { /* ignore */ }
  }, [activeSession, loadSessions]);

  const updateSessionParticipant = useCallback(async (idx: number, provider: string, model: string) => {
    if (!activeSession) return;
    try {
      const updated = await invoke<CounselSession>("counsel_update_participant", {
        sessionId: activeSession.id,
        participantIdx: idx,
        provider,
        model,
      });
      setActiveSession(updated);
    } catch (e: any) {
      setError(`Failed to update participant: ${e?.toString()}`);
    }
  }, [activeSession]);

  const newSession = () => {
    setActiveSession(null);
    setShowSetup(true);
    setTopic("");
    setParticipants([...DEFAULT_PARTICIPANTS]);
    setModeratorIdx(0);
  };

  // ── Render ───

  return (
    <div style={S.container}>
      {/* Sidebar */}
      <div style={S.sidebar}>
        <div style={{ marginBottom: 12 }}>
          <button style={{ ...S.btn, width: "100%", fontSize: 12 }} onClick={newSession}>
            + New Session
          </button>
        </div>
        <div style={S.label}>Past Sessions</div>
        {sessionList.map(s => (
          <div
            key={s.id}
            style={{ ...S.sidebarItem(activeSession?.id === s.id), display: "flex", alignItems: "center", gap: 4 }}
            title={s.topic}
          >
            <div style={{ flex: 1, minWidth: 0, cursor: "pointer" }} onClick={() => loadSession(s.id)}>
              <div style={{ fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis" }}>{s.topic.slice(0, 28)}{s.topic.length > 28 ? "..." : ""}</div>
              <div style={{ fontSize: 10, opacity: 0.7 }}>
                {s.round_count} rounds, {s.participant_count} participants
              </div>
            </div>
            <button
              onClick={(e) => { e.stopPropagation(); deleteSession(s.id); }}
              title="Delete session"
              style={{ flexShrink: 0, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: 12, padding: "2px 4px", opacity: 0.6 }}
            >
              x
            </button>
          </div>
        ))}
        {sessionList.length === 0 && (
          <div style={{ fontSize: 11, color: "var(--text-secondary)", padding: 8 }}>
            No sessions yet
          </div>
        )}
      </div>

      {/* Main content */}
      <div style={S.main}>
        {showSetup && !activeSession ? (
          /* ── Setup Section ── */
          <div>
            <h2 style={S.h2}>New Counsel Session</h2>

            {error && (
              <div className="panel-error" style={{ marginBottom: 12 }}>
                <span>{error}</span>
                <button onClick={() => setError(null)} style={{ float: "right", background: "none", border: "none", color: "inherit", cursor: "pointer", fontWeight: 600 }}>x</button>
              </div>
            )}

            <div style={{ marginBottom: 16 }}>
              <label className="panel-label">Topic</label>
              <textarea
                style={S.textarea}
                placeholder="What should the AI panel debate? e.g., 'Best approach to implement a microservices migration'"
                value={topic}
                onChange={e => setTopic(e.target.value)}
              />
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <h3 style={S.h3}>Participants</h3>
              {selectedParticipants.size > 0 && (
                <button style={{ ...S.btnSecondary, fontSize: 11, padding: "3px 8px", color: "var(--accent-rose, var(--error-color))" }} onClick={removeSelectedParticipants}>
                  Remove {selectedParticipants.size} selected
                </button>
              )}
            </div>
            {participants.map((p, i) => {
              const models = modelsForProvider(p.provider);
              return (
              <div key={i} style={{ ...S.card, display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", borderColor: selectedParticipants.has(i) ? "var(--accent-blue)" : undefined }}>
                <input
                  type="checkbox"
                  checked={selectedParticipants.has(i)}
                  onChange={() => toggleParticipantSelection(i)}
                  title="Select participant"
                  style={{ flexShrink: 0 }}
                />
                <input
                  type="radio"
                  name="moderator"
                  checked={moderatorIdx === i}
                  onChange={() => setModeratorIdx(i)}
                  title="Set as moderator"
                  style={{ flexShrink: 0 }}
                />
                <select style={{ ...S.select, flex: "1 1 100px", minWidth: 80 }} value={p.provider} onChange={e => {
                  updateParticipant(i, "provider", e.target.value);
                  const newModels = modelsForProvider(e.target.value);
                  if (newModels.length > 0) updateParticipant(i, "model", newModels[0]);
                }}>
                  {providers.map(pr => <option key={pr} value={pr}>{pr}</option>)}
                </select>
                <select style={{ ...S.select, flex: "2 1 120px", minWidth: 100 }} value={p.model} onChange={e => updateParticipant(i, "model", e.target.value)}>
                  {models.map(m => <option key={m} value={m}>{m}</option>)}
                  {models.length === 0 && <option value="">No models available</option>}
                  {models.length > 0 && !models.includes(p.model) && p.model && (
                    <option value={p.model}>{p.model} (custom)</option>
                  )}
                </select>
                <select style={{ ...S.select, flex: "1 1 90px", minWidth: 80 }} value={p.role} onChange={e => updateParticipant(i, "role", e.target.value)}>
                  {ROLES.map(r => <option key={r} value={r}>{r}</option>)}
                </select>
                <button style={{ ...S.voteBtn, flexShrink: 0 }} onClick={() => removeParticipant(i)} title="Remove">x</button>
              </div>
              );
            })}
            <div style={{ display: "flex", gap: 10, marginTop: 12 }}>
              <button style={S.btnSecondary} onClick={addParticipant}>+ Add Participant</button>
              <button
                style={{ ...S.btn, opacity: (!topic.trim() || participants.length === 0) ? 0.5 : 1 }}
                disabled={!topic.trim() || participants.length === 0}
                onClick={startSession}
              >
                Start Counsel
              </button>
            </div>
          </div>
        ) : activeSession ? (
          /* ── Deliberation Section ── */
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
              <h2 style={{ ...S.h2, margin: 0 }}>{activeSession.topic}</h2>
              <span style={S.badge("var(--accent-blue)")}>{activeSession.status}</span>
            </div>

            {error && (
              <div className="panel-error" style={{ marginBottom: 12 }}>
                <span>{error}</span>
                <button onClick={() => setError(null)} style={{ float: "right", background: "none", border: "none", color: "inherit", cursor: "pointer", fontWeight: 600 }}>x</button>
              </div>
            )}

            {/* Participant cards with editable provider/model */}
            <div style={{ display: "flex", gap: 6, marginBottom: 16, flexWrap: "wrap", alignItems: "center" }}>
              {activeSession.participants.map((p, i) => {
                const roleColor = ROLE_COLORS[p.role] || "var(--text-secondary)";
                const pModels = modelsForProvider(p.provider);
                return (
                  <div key={i} style={{ ...S.badge(roleColor), display: "inline-flex", alignItems: "center", gap: 4, padding: "4px 8px" }}>
                    <select
                      style={{ background: "transparent", border: "none", color: "inherit", fontSize: 11, fontWeight: 600, cursor: "pointer", outline: "none", padding: 0 }}
                      value={p.provider}
                      onChange={e => {
                        const newProv = e.target.value;
                        const newModels = modelsForProvider(newProv);
                        updateSessionParticipant(i, newProv, newModels[0] || "");
                      }}
                    >
                      {providers.map(pr => <option key={pr} value={pr} style={{ background: "var(--bg-secondary)", color: "var(--text-primary)" }}>{pr}</option>)}
                    </select>
                    <span style={{ opacity: 0.5 }}>/</span>
                    <select
                      style={{ background: "transparent", border: "none", color: "inherit", fontSize: 11, cursor: "pointer", outline: "none", padding: 0, maxWidth: 140 }}
                      value={p.model}
                      onChange={e => updateSessionParticipant(i, p.provider, e.target.value)}
                    >
                      {pModels.map(m => <option key={m} value={m} style={{ background: "var(--bg-secondary)", color: "var(--text-primary)" }}>{m}</option>)}
                      {pModels.length > 0 && !pModels.includes(p.model) && p.model && (
                        <option value={p.model} style={{ background: "var(--bg-secondary)", color: "var(--text-primary)" }}>{p.model}</option>
                      )}
                    </select>
                    <span style={{ opacity: 0.7, fontSize: 10 }}>({p.role})</span>
                    {i === activeSession.moderator_index && <span style={{ fontSize: 9, opacity: 0.6 }}>[Mod]</span>}
                  </div>
                );
              })}
            </div>

            {/* Synthesis */}
            {activeSession.synthesis && (
              <div style={S.synthesisCard}>
                <div style={{ fontSize: 12, fontWeight: 700, color: "var(--accent-blue)", marginBottom: 8 }}>
                  Synthesis by {activeSession.participants[activeSession.moderator_index]?.provider}/
                  {activeSession.participants[activeSession.moderator_index]?.model}
                </div>
                <div style={{ fontSize: 13, lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
                  {activeSession.synthesis}
                </div>
              </div>
            )}

            {/* Rounds */}
            {activeSession.rounds.map((round, ri) => (
              <div key={ri} style={{ marginBottom: 20 }}>
                <h3 style={S.h3}>Round {round.round_number + 1}</h3>
                <div style={{ ...S.grid, gridTemplateColumns: `repeat(${Math.min(activeSession.participants.length, 3)}, 1fr)` }}>
                  {round.responses.map((resp, rsi) => {
                    const participant = activeSession.participants[resp.participant_index];
                    const roleColor = ROLE_COLORS[participant?.role] || "var(--text-secondary)";
                    return (
                      <div key={rsi} style={{ ...S.card, borderTop: `3px solid ${roleColor}` }}>
                        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                          <div>
                            <span style={{ fontSize: 12, fontWeight: 600 }}>{participant?.provider}/{participant?.model}</span>
                            <span style={S.badge(roleColor)}>{participant?.role}</span>
                          </div>
                          <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>{resp.duration_ms}ms</span>
                        </div>
                        <div style={{ fontSize: 13, lineHeight: 1.5, whiteSpace: "pre-wrap", maxHeight: 300, overflowY: "auto", marginBottom: 8 }}>
                          {resp.content}
                        </div>
                        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                          <button style={S.voteBtn} onClick={() => vote(ri, resp.participant_index, 1)}>&#9650;</button>
                          <span style={{ fontSize: 13, fontWeight: 600, minWidth: 20, textAlign: "center" }}>{resp.votes}</span>
                          <button style={S.voteBtn} onClick={() => vote(ri, resp.participant_index, -1)}>&#9660;</button>
                          {resp.tokens != null && (
                            <span style={{ fontSize: 10, color: "var(--text-secondary)", marginLeft: 8 }}>{resp.tokens} tok</span>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
                {round.user_interjection && (
                  <div style={{ ...S.card, borderLeft: "3px solid var(--accent-blue)", marginTop: 8 }}>
                    <div style={{ fontSize: 11, fontWeight: 600, color: "var(--accent-blue)", marginBottom: 4 }}>User Context</div>
                    <div style={{ fontSize: 13 }}>{round.user_interjection}</div>
                  </div>
                )}
              </div>
            ))}

            {/* User interjection input (between rounds) */}
            {activeSession.rounds.length > 0 && !activeSession.synthesis && (
              <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
                <input
                  style={{ ...S.input, flex: 1 }}
                  placeholder="Add context or redirect the discussion..."
                  value={interjection}
                  onChange={e => setInterjection(e.target.value)}
                  onKeyDown={e => e.key === "Enter" && injectMessage()}
                />
                <button style={S.btnSecondary} onClick={injectMessage} disabled={!interjection.trim()}>
                  Add Context
                </button>
              </div>
            )}

            {/* Action buttons */}
            {!activeSession.synthesis && (
              <div style={{ display: "flex", gap: 10 }}>
                <button
                  style={{ ...S.btn, opacity: deliberating ? 0.5 : 1 }}
                  disabled={deliberating}
                  onClick={runRound}
                >
                  {deliberating ? "Deliberating..." : `Run Round ${(activeSession.rounds.length || 0) + 1}`}
                </button>
                {activeSession.rounds.length > 0 && (
                  <button
                    style={{ ...S.btn, background: "#b94aff", opacity: synthesizing ? 0.5 : 1 }}
                    disabled={synthesizing}
                    onClick={synthesize}
                  >
                    {synthesizing ? "Synthesizing..." : "Synthesize"}
                  </button>
                )}
              </div>
            )}
          </div>
        ) : null}
      </div>
    </div>
  );
}

export default CounselPanel;
