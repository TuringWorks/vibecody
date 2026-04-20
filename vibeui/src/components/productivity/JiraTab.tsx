import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertCircle,
  ExternalLink,
  Loader2,
  MessageSquare,
  Plus,
  RefreshCw,
  Terminal,
} from "lucide-react";
import type { JiraIssue } from "../../types/productivity";
import { ProviderStatusStrip } from "./ProviderStatusStrip";

function priorityColor(p: string): string {
  const s = p.toLowerCase();
  if (s.includes("highest") || s.includes("critical")) return "var(--color-error, #d63e3e)";
  if (s.includes("high")) return "var(--color-warn, #c69023)";
  if (s.includes("medium")) return "var(--text-primary)";
  return "var(--text-secondary)";
}

function statusColor(s: string): string {
  const k = s.toLowerCase();
  if (k.includes("done") || k.includes("closed") || k.includes("resolved"))
    return "var(--color-success, #3aa655)";
  if (k.includes("progress") || k.includes("review")) return "var(--color-warn, #c69023)";
  return "var(--text-secondary)";
}

export function JiraTab() {
  const [issues, setIssues] = useState<JiraIssue[]>([]);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const [showCreate, setShowCreate] = useState(false);
  const [project, setProject] = useState("");
  const [summary, setSummary] = useState("");
  const [creating, setCreating] = useState(false);

  const [selected, setSelected] = useState<JiraIssue | null>(null);
  const [comment, setComment] = useState("");
  const [commenting, setCommenting] = useState(false);
  const [commentOk, setCommentOk] = useState<string | null>(null);

  const [showAdvanced, setShowAdvanced] = useState(false);
  const [cmd, setCmd] = useState("");
  const [cmdOutput, setCmdOutput] = useState("");
  const [cmdBusy, setCmdBusy] = useState(false);

  const fetchIssues = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const list = await invoke<JiraIssue[]>("productivity_jira_list");
      setIssues(list);
    } catch (e) {
      setErr(String(e));
      setIssues([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchIssues();
  }, [fetchIssues]);

  async function createIssue() {
    if (!project.trim() || !summary.trim()) return;
    setCreating(true);
    setErr(null);
    try {
      const issue = await invoke<JiraIssue>("productivity_jira_create", {
        project: project.trim(),
        summary: summary.trim(),
      });
      setIssues((prev) => [issue, ...prev]);
      setSummary("");
      setShowCreate(false);
    } catch (e) {
      setErr(String(e));
    } finally {
      setCreating(false);
    }
  }

  async function postComment() {
    if (!selected || !comment.trim()) return;
    setCommenting(true);
    setCommentOk(null);
    try {
      await invoke("productivity_jira_comment", {
        key: selected.key,
        text: comment.trim(),
      });
      setComment("");
      setCommentOk("Comment added.");
    } catch (e) {
      setErr(String(e));
    } finally {
      setCommenting(false);
    }
  }

  async function runAdvancedCmd() {
    if (!cmd.trim()) return;
    setCmdBusy(true);
    try {
      const out = await invoke<string>("handle_productivity_command", {
        args: `jira ${cmd}`,
      });
      setCmdOutput(out);
    } catch (e) {
      setCmdOutput(`Error: ${e}`);
    } finally {
      setCmdBusy(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      <ProviderStatusStrip tab="jira" />
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-color)",
          flexWrap: "wrap",
        }}
      >
        <button
          className="panel-btn panel-btn-secondary"
          onClick={fetchIssues}
          disabled={loading}
          title="Refresh"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {loading ? (
            <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
          ) : (
            <RefreshCw size={12} />
          )}
          Refresh
        </button>
        <button
          className={`panel-btn panel-btn-secondary${showCreate ? " active" : ""}`}
          onClick={() => setShowCreate((s) => !s)}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <Plus size={12} />
          New
        </button>
        <span style={{ flex: 1 }} />
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => setShowAdvanced((s) => !s)}
          title="Advanced: raw /jira commands"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <Terminal size={12} />
        </button>
      </div>
      {showCreate && (
        <div
          style={{
            display: "flex",
            gap: 6,
            padding: "8px 10px",
            borderBottom: "1px solid var(--border-color)",
            background: "var(--bg-secondary)",
          }}
        >
          <input
            className="panel-input"
            style={{ width: 120 }}
            placeholder="Project key"
            value={project}
            onChange={(e) => setProject(e.target.value.toUpperCase())}
            disabled={creating}
          />
          <input
            className="panel-input"
            style={{ flex: 1 }}
            placeholder="Issue summary"
            value={summary}
            onChange={(e) => setSummary(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") createIssue();
            }}
            disabled={creating}
          />
          <button
            className="panel-btn panel-btn-primary"
            onClick={createIssue}
            disabled={creating || !project.trim() || !summary.trim()}
          >
            {creating ? "Creating…" : "Create"}
          </button>
        </div>
      )}
      {err && (
        <div
          style={{
            padding: "6px 10px",
            color: "var(--color-error, #d63e3e)",
            background: "var(--bg-secondary)",
            fontSize: "var(--font-size-sm)",
            borderBottom: "1px solid var(--border-color)",
          }}
        >
          {err}
        </div>
      )}
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        <div
          style={{
            width: selected ? "50%" : "100%",
            overflowY: "auto",
            borderRight: selected ? "1px solid var(--border-color)" : undefined,
          }}
        >
          {loading && issues.length === 0 ? (
            <div
              style={{
                padding: 20,
                display: "flex",
                alignItems: "center",
                gap: 6,
                color: "var(--text-secondary)",
                fontSize: "var(--font-size-sm)",
              }}
            >
              <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
              Loading issues…
            </div>
          ) : issues.length === 0 ? (
            <div
              style={{
                padding: 20,
                color: "var(--text-secondary)",
                textAlign: "center",
                fontSize: "var(--font-size-sm)",
              }}
            >
              No issues.
            </div>
          ) : (
            issues.map((i) => (
              <button
                key={i.key}
                onClick={() => {
                  setSelected(i);
                  setComment("");
                  setCommentOk(null);
                }}
                className={`panel-card panel-card--clickable${selected?.key === i.key ? " active" : ""}`}
                style={{
                  display: "grid",
                  gridTemplateColumns: "72px 1fr auto",
                  gap: 8,
                  alignItems: "center",
                  width: "100%",
                  textAlign: "left",
                  background: selected?.key === i.key ? "var(--bg-tertiary)" : "transparent",
                  border: "none",
                  borderBottom: "1px solid var(--border-color)",
                  padding: "8px 10px",
                  cursor: "pointer",
                  color: "inherit",
                  fontSize: "var(--font-size-sm)",
                }}
              >
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    color: "var(--text-secondary)",
                    fontSize: "calc(var(--font-size-sm) - 1px)",
                  }}
                >
                  {i.key}
                </span>
                <span
                  style={{
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {i.summary}
                </span>
                <span
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    fontSize: "calc(var(--font-size-sm) - 1px)",
                  }}
                >
                  <span style={{ color: statusColor(i.status) }}>{i.status}</span>
                  <AlertCircle size={10} color={priorityColor(i.priority)} />
                </span>
              </button>
            ))
          )}
        </div>
        {selected && (
          <div style={{ flex: 1, overflowY: "auto", padding: 12 }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                marginBottom: 10,
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  color: "var(--text-secondary)",
                  fontSize: "var(--font-size-sm)",
                }}
              >
                {selected.key}
              </span>
              <strong style={{ flex: 1 }}>{selected.summary}</strong>
              <a
                href={selected.url}
                target="_blank"
                rel="noreferrer"
                className="panel-btn panel-btn-secondary"
                style={{
                  textDecoration: "none",
                  fontSize: "calc(var(--font-size-sm) - 1px)",
                  display: "flex",
                  alignItems: "center",
                  gap: 4,
                }}
              >
                <ExternalLink size={11} />
                Open
              </a>
              <button
                className="panel-btn panel-btn-secondary"
                onClick={() => setSelected(null)}
                style={{ fontSize: "calc(var(--font-size-sm) - 1px)" }}
              >
                Close
              </button>
            </div>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "auto 1fr",
                columnGap: 12,
                rowGap: 4,
                marginBottom: 12,
                fontSize: "var(--font-size-sm)",
              }}
            >
              <span style={{ color: "var(--text-secondary)" }}>Type</span>
              <span>{selected.issue_type}</span>
              <span style={{ color: "var(--text-secondary)" }}>Status</span>
              <span style={{ color: statusColor(selected.status) }}>{selected.status}</span>
              <span style={{ color: "var(--text-secondary)" }}>Priority</span>
              <span style={{ color: priorityColor(selected.priority) }}>{selected.priority}</span>
              <span style={{ color: "var(--text-secondary)" }}>Assignee</span>
              <span>{selected.assignee ?? <em style={{ color: "var(--text-secondary)" }}>Unassigned</em>}</span>
            </div>
            <div
              style={{
                borderTop: "1px solid var(--border-color)",
                paddingTop: 10,
                display: "flex",
                flexDirection: "column",
                gap: 6,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <MessageSquare size={12} color="var(--text-secondary)" />
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                  Add comment
                </span>
              </div>
              <textarea
                className="panel-input"
                style={{ minHeight: 60, resize: "vertical", fontFamily: "inherit" }}
                placeholder="Write a comment…"
                value={comment}
                onChange={(e) => setComment(e.target.value)}
                disabled={commenting}
              />
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <button
                  className="panel-btn panel-btn-primary"
                  onClick={postComment}
                  disabled={commenting || !comment.trim()}
                >
                  {commenting ? "Posting…" : "Post comment"}
                </button>
                {commentOk && (
                  <span
                    style={{
                      color: "var(--color-success, #3aa655)",
                      fontSize: "var(--font-size-sm)",
                    }}
                  >
                    {commentOk}
                  </span>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
      {showAdvanced && (
        <div
          style={{
            borderTop: "1px solid var(--border-color)",
            padding: 10,
            background: "var(--bg-secondary)",
            display: "flex",
            flexDirection: "column",
            gap: 6,
            maxHeight: "35%",
          }}
        >
          <div style={{ display: "flex", gap: 6 }}>
            <input
              className="panel-input"
              style={{ flex: 1 }}
              placeholder="jira mine | jira list [project] | jira get <key> | jira sprint"
              value={cmd}
              onChange={(e) => setCmd(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") runAdvancedCmd();
              }}
              disabled={cmdBusy}
            />
            <button
              className="panel-btn panel-btn-primary"
              onClick={runAdvancedCmd}
              disabled={cmdBusy || !cmd.trim()}
            >
              {cmdBusy ? "Running…" : "Run"}
            </button>
          </div>
          {cmdOutput && (
            <pre
              style={{
                margin: 0,
                padding: 8,
                background: "var(--bg-primary)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-xs-plus)",
                fontSize: "var(--font-size-sm)",
                whiteSpace: "pre-wrap",
                overflowY: "auto",
                flex: 1,
              }}
            >
              {cmdOutput}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}
