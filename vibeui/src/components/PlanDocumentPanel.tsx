/**
 * PlanDocumentPanel — Plan-as-Document with Feedback.
 *
 * Tabs: Plans (list with status/version/author), Editor (markdown preview),
 * Comments (unresolved comments with resolve button).
 * Wired to Tauri backend commands, persisted to ~/.vibeui/plan-documents.json.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "plans" | "editor" | "comments";
type PlanStatus = "draft" | "review" | "approved" | "archived";

interface Plan {
  id: string;
  title: string;
  status: PlanStatus;
  version: number;
  author: string;
  updatedAt: string;
  markdown: string;
}

interface Comment {
  id: string;
  planId: string;
  author: string;
  timestamp: string;
  text: string;
  resolved: boolean;
  line?: number;
}

const statusColors: Record<PlanStatus, string> = {
  draft: "var(--text-secondary)",
  review: "var(--text-warning)",
  approved: "var(--text-success)",
  archived: "var(--text-secondary)",
};


export default function PlanDocumentPanel() {
  const [tab, setTab] = useState<Tab>("plans");
  const [plans, setPlans] = useState<Plan[]>([]);
  const [comments, setComments] = useState<Comment[]>([]);
  const [selectedPlan, setSelectedPlan] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // New plan form state
  const [showNewPlan, setShowNewPlan] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newAuthor, setNewAuthor] = useState("");
  const [newMarkdown, setNewMarkdown] = useState("");

  // New comment form state
  const [newCommentText, setNewCommentText] = useState("");
  const [newCommentAuthor, setNewCommentAuthor] = useState("");

  const loadPlans = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<Plan[]>("list_plan_documents");
      setPlans(result);
      // Load comments for all plans
      const allComments: Comment[] = [];
      for (const p of result) {
        try {
          const detail = await invoke<{ plan: Plan; comments: Comment[] }>("get_plan_document", { id: p.id });
          allComments.push(...detail.comments);
        } catch {
          // skip individual failures
        }
      }
      setComments(allComments);
      if (result.length > 0 && !selectedPlan) {
        setSelectedPlan(result[0].id);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [selectedPlan]);

  useEffect(() => { loadPlans(); }, [loadPlans]);

  const plan = plans.find(p => p.id === selectedPlan);
  const planComments = comments.filter(c => c.planId === selectedPlan && !c.resolved);
  const allUnresolved = comments.filter(c => !c.resolved);

  const resolveComment = async (id: string) => {
    try {
      await invoke("resolve_plan_comment", { id });
      setComments(cs => cs.map(c => c.id === id ? { ...c, resolved: true } : c));
    } catch (e) {
      setError(String(e));
    }
  };

  const createPlan = async () => {
    if (!newTitle.trim() || !newAuthor.trim()) return;
    try {
      const created = await invoke<Plan>("create_plan_document", {
        title: newTitle.trim(),
        author: newAuthor.trim(),
        markdown: newMarkdown.trim() || `# ${newTitle.trim()}\n\n(New plan)`,
      });
      setPlans(prev => [...prev, created]);
      setSelectedPlan(created.id);
      setShowNewPlan(false);
      setNewTitle("");
      setNewAuthor("");
      setNewMarkdown("");
      setTab("editor");
    } catch (e) {
      setError(String(e));
    }
  };

  const updateStatus = async (id: string, status: PlanStatus) => {
    try {
      const updated = await invoke<Plan>("update_plan_status", { id, status });
      setPlans(prev => prev.map(p => p.id === id ? updated : p));
    } catch (e) {
      setError(String(e));
    }
  };

  const addComment = async () => {
    if (!newCommentText.trim() || !newCommentAuthor.trim() || !selectedPlan) return;
    try {
      const comment = await invoke<Comment>("add_plan_comment", {
        planId: selectedPlan,
        author: newCommentAuthor.trim(),
        text: newCommentText.trim(),
        line: null,
      });
      setComments(prev => [...prev, comment]);
      setNewCommentText("");
    } catch (e) {
      setError(String(e));
    }
  };

  if (loading) {
    return <div style={{ padding: 20, color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>Loading plans...</div>;
  }

  return (
    <div className="panel-container">
      {error && (
        <div role="button" tabIndex={0} className="panel-error" style={{ cursor: "pointer" }} onClick={() => setError(null)}>
          {error} (click to dismiss)
        </div>
      )}
      <div className="panel-header">
        {(["plans", "editor", "comments"] as Tab[]).map(t => (
          <button key={t} onClick={() => setTab(t)} className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`}>
            {t[0].toUpperCase() + t.slice(1)}
            {t === "comments" && allUnresolved.length > 0 && (
              <span style={{ marginLeft: 4, fontSize: 9, padding: "0 4px", borderRadius: "var(--radius-sm)", background: "var(--error-color)", color: "var(--bg-primary)" }}>{allUnresolved.length}</span>
            )}
          </button>
        ))}
      </div>

      <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        {tab === "plans" && (
          <>
            <button onClick={() => setShowNewPlan(!showNewPlan)}
              style={{ alignSelf: "flex-start", padding: "4px 12px", fontSize: "var(--font-size-sm)", background: "var(--accent-bg)", border: "1px solid var(--accent-primary)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-info)", cursor: "pointer", fontWeight: 600 }}>
              {showNewPlan ? "Cancel" : "+ New Plan"}
            </button>
            {showNewPlan && (
              <div style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: 6 }}>
                <input value={newTitle} onChange={e => setNewTitle(e.target.value)} placeholder="Plan title"
                  style={{ padding: "4px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)" }} />
                <input value={newAuthor} onChange={e => setNewAuthor(e.target.value)} placeholder="Author"
                  style={{ padding: "4px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)" }} />
                <textarea value={newMarkdown} onChange={e => setNewMarkdown(e.target.value)} placeholder="Markdown content (optional)" rows={4}
                  style={{ padding: "4px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", fontFamily: "var(--font-mono)", resize: "vertical" }} />
                <button className="panel-btn" onClick={createPlan}
                  style={{ alignSelf: "flex-start", padding: "4px 12px", fontSize: "var(--font-size-sm)", background: "var(--text-success)", border: "none", borderRadius: "var(--radius-xs-plus)", color: "var(--bg-primary)", cursor: "pointer", fontWeight: 600 }}>
                  Create Plan
                </button>
              </div>
            )}
            {plans.length === 0 && !showNewPlan && (
              <div style={{ textAlign: "center", color: "var(--text-secondary)", fontSize: "var(--font-size-base)", padding: 40 }}>No plans yet. Create one to get started.</div>
            )}
            {plans.map(p => (
              <div key={p.id} role="button" tabIndex={0} onClick={() => { setSelectedPlan(p.id); setTab("editor"); }} onKeyDown={e => e.key === "Enter" && (setSelectedPlan(p.id), setTab("editor"))}
                style={{ padding: 10, background: selectedPlan === p.id ? "var(--accent-bg)" : "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: `1px solid ${selectedPlan === p.id ? "var(--accent-primary)" : "var(--border-color)"}`, cursor: "pointer" }}>
                <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 4 }}>
                  <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color: "var(--text-primary)" }}>{p.title}</span>
                  <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 8px", borderRadius: "var(--radius-md)", background: `${statusColors[p.status]}22`, color: statusColors[p.status], fontWeight: 600 }}>{p.status}</span>
                </div>
                <div style={{ display: "flex", gap: 12, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                  <span>v{p.version}</span>
                  <span>{p.author}</span>
                  <span>{p.updatedAt}</span>
                  {comments.filter(c => c.planId === p.id && !c.resolved).length > 0 && (
                    <span style={{ color: "var(--text-warning)" }}>
                      {comments.filter(c => c.planId === p.id && !c.resolved).length} comments
                    </span>
                  )}
                </div>
              </div>
            ))}
          </>
        )}

        {tab === "editor" && plan && (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600, color: "var(--text-primary)" }}>{plan.title}</span>
              <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 8px", borderRadius: "var(--radius-md)", background: `${statusColors[plan.status]}22`, color: statusColors[plan.status] }}>{plan.status}</span>
              <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginLeft: "auto" }}>v{plan.version} by {plan.author}</span>
            </div>
            <div style={{ display: "flex", gap: 4 }}>
              {(["draft", "review", "approved", "archived"] as PlanStatus[]).map(s => (
                <button key={s} onClick={(e) => { e.stopPropagation(); updateStatus(plan.id, s); }}
                  disabled={plan.status === s}
                  style={{ padding: "3px 8px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)",
                    background: plan.status === s ? statusColors[s] : "transparent",
                    color: plan.status === s ? "var(--bg-primary)" : "var(--text-secondary)", cursor: plan.status === s ? "default" : "pointer", opacity: plan.status === s ? 1 : 0.7 }}>
                  {s}
                </button>
              ))}
            </div>
            <pre style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", color: "var(--text-primary)", whiteSpace: "pre-wrap", margin: 0, lineHeight: 1.6 }}>
              {plan.markdown}
            </pre>
            {planComments.length > 0 && (
              <button onClick={() => setTab("comments")}
                style={{ alignSelf: "flex-start", padding: "4px 12px", fontSize: "var(--font-size-sm)", background: "var(--bg-secondary)", border: "1px solid var(--text-warning)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-warning)", cursor: "pointer" }}>
                View {planComments.length} comments
              </button>
            )}
            {/* Add comment inline */}
            <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <input value={newCommentAuthor} onChange={e => setNewCommentAuthor(e.target.value)} placeholder="Your name"
                style={{ padding: "4px 8px", fontSize: "var(--font-size-xs)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", width: 100 }} />
              <input value={newCommentText} onChange={e => setNewCommentText(e.target.value)} placeholder="Add a comment..."
                onKeyDown={e => { if (e.key === "Enter") addComment(); }}
                style={{ flex: 1, padding: "4px 8px", fontSize: "var(--font-size-xs)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)" }} />
              <button className="panel-btn" onClick={addComment}
                style={{ padding: "4px 12px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-xs-plus)", border: "none", background: "var(--accent-primary)", color: "var(--bg-primary)", cursor: "pointer", fontWeight: 600 }}>
                Post
              </button>
            </div>
          </div>
        )}

        {tab === "editor" && !plan && (
          <div style={{ textAlign: "center", color: "var(--text-secondary)", fontSize: "var(--font-size-base)", padding: 40 }}>Select a plan from the Plans tab</div>
        )}

        {tab === "comments" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {allUnresolved.length === 0 && (
              <div style={{ textAlign: "center", color: "var(--text-secondary)", fontSize: "var(--font-size-base)", padding: 40 }}>All comments resolved</div>
            )}
            {allUnresolved.map(c => {
              const p = plans.find(pl => pl.id === c.planId);
              return (
                <div key={c.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
                  <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
                    <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-primary)" }}>{c.author}</span>
                    <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{c.timestamp}</span>
                    {c.line && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>L{c.line}</span>}
                    <span style={{ fontSize: "var(--font-size-xs)", color: "var(--accent-primary)", marginLeft: "auto" }}>{p?.title}</span>
                  </div>
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-primary)", marginBottom: 8 }}>{c.text}</div>
                  <button onClick={() => resolveComment(c.id)}
                    style={{ padding: "4px 12px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-xs-plus)", border: "none", background: "var(--text-success)", color: "var(--bg-primary)", cursor: "pointer", fontWeight: 600 }}>
                    Resolve
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
