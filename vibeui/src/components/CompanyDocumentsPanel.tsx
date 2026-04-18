/**
 * CompanyDocumentsPanel — Markdown documents with revision history.
 *
 * Shows company documents linked to tasks/goals. Supports creating,
 * editing (full markdown), viewing revision history, role assignment,
 * and meeting notes ingestion.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CompanyDocumentsPanelProps {
  workspacePath?: string | null;
}

type DocRole = 'policy' | 'source-of-truth' | 'reference' | 'template';

interface Document {
  id: string;
  title: string;
  role: DocRole;
  created_at: number;
  updated_at: number;
}

interface MeetingTask {
  title: string;
  owner: string;
  due_date: string | null;
  checked: boolean;
}

interface MeetingApproval {
  subject: string;
  decision_text: string;
}

interface MeetingFollowup {
  text: string;
  due_date: string | null;
}

interface MeetingIngestResult {
  tasks: Array<{ title: string; owner: string; due_date: string | null }>;
  approvals: Array<{ subject: string; decision_text: string }>;
  followups: Array<{ text: string; due_date: string | null }>;
}

function roleBadgeStyle(role: DocRole): React.CSSProperties {
  const map: Record<DocRole, { color: string; bg: string }> = {
    'policy': { color: 'var(--accent-gold)', bg: 'rgba(255,193,7,0.15)' },
    'source-of-truth': { color: 'var(--accent-rose)', bg: 'rgba(231,76,60,0.15)' },
    'reference': { color: 'var(--text-secondary)', bg: 'rgba(128,128,128,0.12)' },
    'template': { color: 'var(--accent-blue)', bg: 'rgba(74,158,255,0.15)' },
  };
  const { color, bg } = map[role] ?? map['reference'];
  return {
    display: 'inline-block', padding: '1px 7px', borderRadius: "var(--radius-md)", fontSize: "var(--font-size-xs)",
    fontWeight: 600, color, background: bg, border: `1px solid ${color}`,
  };
}


export function CompanyDocumentsPanel({ workspacePath: _wp }: CompanyDocumentsPanelProps) {
  // Tabs: list | create | view | meeting
  const [tab, setTab] = useState<"list" | "create" | "view" | "meeting">("list");
  const [docs, setDocs] = useState<Document[]>([]);
  const [listOutput, setListOutput] = useState<string>("");
  const [docOutput, setDocOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [content, setContent] = useState("");
  const [docId, setDocId] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);
  const [useStructured, setUseStructured] = useState(false);
  const [roleDropdownId, setRoleDropdownId] = useState<string | null>(null);

  // Meeting notes state
  const [meetingTitle, setMeetingTitle] = useState("");
  const [meetingContent, setMeetingContent] = useState("");
  const [meetingLoading, setMeetingLoading] = useState(false);
  const [meetingTasks, setMeetingTasks] = useState<MeetingTask[]>([]);
  const [meetingApprovals, setMeetingApprovals] = useState<MeetingApproval[]>([]);
  const [meetingFollowups, setMeetingFollowups] = useState<MeetingFollowup[]>([]);
  const [meetingIngested, setMeetingIngested] = useState(false);

  const loadList = async () => {
    setLoading(true);
    try {
      const result = await invoke<Document[]>("company_doc_list_json").catch(async () => {
        setUseStructured(false);
        const out = await invoke<string>("company_cmd", { args: "doc list" });
        setListOutput(out);
        return null;
      });
      if (result !== null) {
        setUseStructured(true);
        setDocs(result);
      }
    } catch (e) {
      setListOutput(`Error: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadList(); }, []);

  const createDoc = async () => {
    if (!newTitle.trim()) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `doc create "${newTitle.trim()}"` });
      setCmdResult(out);
      setNewTitle("");
      setContent("");
      setTab("list");
      loadList();
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const viewDoc = async () => {
    if (!docId.trim()) return;
    try {
      const out = await invoke<string>("company_cmd", { args: `doc show ${docId.trim()}` });
      setDocOutput(out);
      setTab("view");
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const changeRole = async (dId: string, role: DocRole) => {
    try {
      await invoke("company_doc_set_role", { docId: dId, role });
      setDocs((prev) => prev.map((d) => d.id === dId ? { ...d, role } : d));
      setRoleDropdownId(null);
    } catch { /* ignore */ }
  };

  const ingestMeeting = async () => {
    if (!meetingContent.trim()) return;
    setMeetingLoading(true);
    setMeetingIngested(false);
    try {
      const result = await invoke<MeetingIngestResult>("company_ingest_meeting_notes", {
        content: meetingContent,
        sourceTitle: meetingTitle || null,
      });
      setMeetingTasks(result.tasks.map((t) => ({ ...t, checked: false })));
      setMeetingApprovals(result.approvals);
      setMeetingFollowups(result.followups);
      setMeetingIngested(true);
    } catch (e) {
      setCmdResult(`Ingest error: ${e}`);
    } finally {
      setMeetingLoading(false);
    }
  };

  const addTaskToBoard = async (task: MeetingTask) => {
    try {
      await invoke("company_task_create_v2", {
        title: task.title,
        status: "backlog",
        owner: task.owner || "agent",
        program: "Other",
        recurrence: null,
      });
      setCmdResult(`Task added: "${task.title}"`);
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const createApproval = async (approval: MeetingApproval) => {
    try {
      await invoke("company_approval_request", {
        subject: approval.subject,
        decisionText: approval.decision_text,
      });
      setCmdResult(`Approval created: "${approval.subject}"`);
    } catch (e) {
      setCmdResult(`Error: ${e}`);
    }
  };

  const roles: DocRole[] = ['policy', 'source-of-truth', 'reference', 'template'];

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Agent Docs</h3>
        <div style={{ display: "flex", gap: 6, marginLeft: "auto" }}>
          {(["list", "create", "meeting"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`}
              style={{ padding: "2px 8px" }}
            >
              {t === "list" ? "List" : t === "create" ? "+ New" : "Meeting Notes"}
            </button>
          ))}
          <button onClick={loadList} className="panel-btn panel-btn-secondary">
            Refresh
          </button>
        </div>
      </div>
      <div className="panel-body">

      {cmdResult && (
        <div className="panel-card" style={{ marginBottom: 12, fontSize: "var(--font-size-base)" }}>
          {cmdResult}
        </div>
      )}

      {/* LIST TAB */}
      {tab === "list" && (
        <>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <input value={docId} onChange={(e) => setDocId(e.target.value)} onKeyDown={(e) => e.key === "Enter" && viewDoc()} placeholder="Document ID to view"
              className="panel-input" style={{ flex: 1 }} />
            <button onClick={viewDoc} className="panel-btn panel-btn-primary">View</button>
          </div>
          <div className="panel-card" style={{ minHeight: 200, padding: useStructured ? 0 : undefined, overflow: "hidden" }}>
            {loading ? (
              <span className="panel-loading" style={{ padding: 12, display: "block" }}>Loading…</span>
            ) : useStructured ? (
              docs.length === 0 ? (
                <div style={{ padding: 16, fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>No documents. Click + New to create one.</div>
              ) : (
                <div>
                  {docs.map((doc) => (
                    <div key={doc.id} style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 12px", borderBottom: "1px solid var(--border-color)" }}>
                      <span style={{ fontSize: "var(--font-size-base)", flex: 1 }}>{doc.title}</span>
                      {/* Role badge with change dropdown */}
                      <div style={{ position: "relative" }}>
                        <span
                          style={{ ...roleBadgeStyle(doc.role), cursor: "pointer" }}
                          onClick={() => setRoleDropdownId(roleDropdownId === doc.id ? null : doc.id)}
                        >
                          {doc.role}
                        </span>
                        {roleDropdownId === doc.id && (
                          <div style={{
                            position: "absolute", right: 0, top: "110%", zIndex: 50, minWidth: 160,
                            background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
                            borderRadius: "var(--radius-sm)", boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
                          }}>
                            {roles.map((r) => (
                              <button
                                key={r}
                                onClick={() => changeRole(doc.id, r)}
                                style={{
                                  display: "block", width: "100%", textAlign: "left",
                                  padding: "8px 12px", background: r === doc.role ? "rgba(128,128,128,0.1)" : "transparent",
                                  border: "none", cursor: "pointer", fontSize: "var(--font-size-base)",
                                }}
                              >
                                <span style={roleBadgeStyle(r)}>{r}</span>
                              </button>
                            ))}
                          </div>
                        )}
                      </div>
                      <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>{doc.id}</span>
                    </div>
                  ))}
                </div>
              )
            ) : (
              <pre style={{ margin: 0, fontSize: "var(--font-size-base)", whiteSpace: "pre-wrap" }}>
                {listOutput || "No documents. Click + New to create one."}
              </pre>
            )}
          </div>
        </>
      )}

      {/* CREATE TAB */}
      {tab === "create" && (
        <div>
          <input value={newTitle} onChange={(e) => setNewTitle(e.target.value)} placeholder="Document title"
            className="panel-input panel-input-full" style={{ marginBottom: 8 }} />
          <textarea value={content} onChange={(e) => setContent(e.target.value)} placeholder="Document content (Markdown)"
            className="panel-input panel-textarea panel-input-full" style={{ height: 300, marginBottom: 8 }} />
          <button onClick={createDoc} className="panel-btn panel-btn-primary">
            Create Document
          </button>
        </div>
      )}

      {/* VIEW TAB */}
      {tab === "view" && (
        <div>
          <button onClick={() => setTab("list")} className="panel-btn panel-btn-secondary" style={{ marginBottom: 12 }}>← Back</button>
          <div className="panel-card">
            <pre style={{ margin: 0, fontSize: "var(--font-size-base)", whiteSpace: "pre-wrap", lineHeight: 1.6 }}>
              {docOutput}
            </pre>
          </div>
        </div>
      )}

      {/* MEETING NOTES TAB */}
      {tab === "meeting" && (
        <div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 16 }}>
            <input
              value={meetingTitle}
              onChange={(e) => setMeetingTitle(e.target.value)}
              placeholder="Title (optional)"
              className="panel-input panel-input-full"
            />
            <textarea
              value={meetingContent}
              onChange={(e) => setMeetingContent(e.target.value)}
              placeholder="Paste meeting notes, transcript, or summary..."
              rows={8}
              className="panel-input panel-textarea panel-input-full"
            />
            <button
              onClick={ingestMeeting}
              disabled={!meetingContent.trim() || meetingLoading}
              className="panel-btn panel-btn-primary"
              style={{ alignSelf: "flex-start", opacity: (!meetingContent.trim() || meetingLoading) ? 0.5 : 1 }}
            >
              {meetingLoading ? "Ingesting…" : "Ingest"}
            </button>
          </div>

          {meetingIngested && (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", borderTop: "1px solid var(--border-color)", paddingTop: 12, fontWeight: 600 }}>── Results ──</div>

              {/* Tasks */}
              <div>
                <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 6 }}>
                  Tasks extracted: <span style={{ fontWeight: 400, color: "var(--text-secondary)" }}>{meetingTasks.length}</span>
                </div>
                {meetingTasks.length === 0 ? (
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>None</div>
                ) : (
                  <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    {meetingTasks.map((t, i) => (
                      <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)" }}>
                        <input
                          type="checkbox"
                          checked={t.checked}
                          onChange={() => setMeetingTasks((prev) => prev.map((x, j) => j === i ? { ...x, checked: !x.checked } : x))}
                        />
                        <span style={{ flex: 1, fontSize: "var(--font-size-base)" }}>{t.title}</span>
                        {t.owner && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{t.owner}</span>}
                        {t.due_date && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{t.due_date}</span>}
                        <button
                          onClick={() => addTaskToBoard(t)}
                          className="panel-btn panel-btn-secondary"
                          style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px" }}
                        >
                          Add as Task
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Decisions / Approvals */}
              <div>
                <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 6 }}>
                  Decisions/Approvals: <span style={{ fontWeight: 400, color: "var(--text-secondary)" }}>{meetingApprovals.length}</span>
                </div>
                {meetingApprovals.length === 0 ? (
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>None</div>
                ) : (
                  <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    {meetingApprovals.map((a, i) => (
                      <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)" }}>
                        <span style={{ flex: 1, fontSize: "var(--font-size-base)" }}><strong>{a.subject}</strong> — {a.decision_text}</span>
                        <button
                          onClick={() => createApproval(a)}
                          className="panel-btn panel-btn-secondary"
                          style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px" }}
                        >
                          Create Approval
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Follow-ups */}
              <div>
                <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 6 }}>
                  Follow-ups: <span style={{ fontWeight: 400, color: "var(--text-secondary)" }}>{meetingFollowups.length}</span>
                </div>
                {meetingFollowups.length === 0 ? (
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>None</div>
                ) : (
                  <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    {meetingFollowups.map((f, i) => (
                      <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)" }}>
                        <span style={{ flex: 1, fontSize: "var(--font-size-base)" }}>{f.text}</span>
                        {f.due_date && (
                          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--accent-gold)", padding: "1px 8px", background: "rgba(255,193,7,0.12)", borderRadius: "var(--radius-sm)" }}>
                            due: {f.due_date}
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      )}
      </div>
    </div>
  );
}
