import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface HookConfig {
  event: string;
  tools: string[];
  handler_type: "command" | "llm" | "http";
  command: string;
  prompt: string;
  http_url: string;
  http_method: string;
  http_headers: string;
  http_timeout_ms: number;
  async_exec: boolean;
}

const HOOK_EVENTS = [
  "PreToolUse",
  "PostToolUse",
  "SessionStart",
  "TaskCompleted",
  "Stop",
] as const;

interface HooksPanelProps {
  workspacePath?: string | null;
}

// ── HookRow ───────────────────────────────────────────────────────────────────

function HookRow({
  hook,
  index,
  onChange,
  onDelete,
}: {
  hook: HookConfig;
  index: number;
  onChange: (i: number, h: HookConfig) => void;
  onDelete: (i: number) => void;
}) {
  const [expanded, setExpanded] = useState(index === 0);

  function update(patch: Partial<HookConfig>) {
    onChange(index, { ...hook, ...patch });
  }

  const labelStyle: React.CSSProperties = {
    fontSize: "10px",
    color: "var(--text-secondary)",
    marginBottom: "3px",
    textTransform: "uppercase",
    letterSpacing: "0.06em",
  };
  const inputStyle: React.CSSProperties = {
    width: "100%",
    padding: "5px 7px",
    fontSize: "11px",
    background: "var(--bg-input, var(--bg-primary))",
    border: "1px solid var(--border-color)",
    borderRadius: "3px",
    color: "var(--text-primary)",
    outline: "none",
    boxSizing: "border-box",
  };

  return (
    <div style={{
      border: "1px solid var(--border-color)",
      borderRadius: "5px",
      marginBottom: "6px",
      background: "var(--bg-secondary)",
      overflow: "hidden",
    }}>
      {/* Header row */}
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          display: "flex",
          alignItems: "center",
          padding: "8px 10px",
          cursor: "pointer",
          gap: "8px",
          userSelect: "none",
        }}
      >
        <span style={{ fontSize: "13px" }}>🪝</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-primary)" }}>
            {hook.event}
            {hook.tools.length > 0 && (
              <span style={{ fontSize: "10px", color: "var(--text-secondary)", marginLeft: "6px" }}>
                [{hook.tools.join(", ")}]
              </span>
            )}
          </div>
          <div style={{ fontSize: "10px", color: "var(--text-secondary)" }}>
            {hook.handler_type === "command"
              ? hook.command || "(no command)"
              : hook.handler_type === "http"
              ? hook.http_url || "(no URL)"
              : hook.prompt.slice(0, 50) || "(no prompt)"}
            {hook.async_exec && <span style={{ marginLeft: "6px", color: "#5af" }}>async</span>}
          </div>
        </div>
        <button
          onClick={(e) => { e.stopPropagation(); onDelete(index); }}
          style={{ padding: "2px 6px", fontSize: "11px", background: "none", border: "1px solid var(--border-color)", borderRadius: "3px", color: "#f44", cursor: "pointer" }}
        >
          ✕
        </button>
        <span style={{ fontSize: "10px", color: "var(--text-secondary)" }}>{expanded ? "▲" : "▼"}</span>
      </div>

      {/* Expanded editor */}
      {expanded && (
        <div style={{ padding: "10px", borderTop: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: "10px" }}>
          {/* Event + Handler type row */}
          <div style={{ display: "flex", gap: "10px" }}>
            <div style={{ flex: 1 }}>
              <div style={labelStyle}>Event</div>
              <select
                value={hook.event}
                onChange={(e) => update({ event: e.target.value })}
                style={inputStyle}
              >
                {HOOK_EVENTS.map((ev) => (
                  <option key={ev} value={ev}>{ev}</option>
                ))}
              </select>
            </div>
            <div style={{ flex: 1 }}>
              <div style={labelStyle}>Handler</div>
              <select
                value={hook.handler_type}
                onChange={(e) => update({ handler_type: e.target.value as "command" | "llm" | "http" })}
                style={inputStyle}
              >
                <option value="command">Shell Command</option>
                <option value="llm">LLM Evaluation</option>
                <option value="http">HTTP Webhook</option>
              </select>
            </div>
          </div>

          {/* Tool filter */}
          <div>
            <div style={labelStyle}>Tool Filter (comma-separated, empty = all)</div>
            <input
              type="text"
              value={hook.tools.join(", ")}
              onChange={(e) => {
                const tools = e.target.value
                  .split(",")
                  .map((t) => t.trim())
                  .filter(Boolean);
                update({ tools });
              }}
              placeholder="write_file, bash, … or leave empty for all"
              style={inputStyle}
            />
          </div>

          {/* Command, LLM prompt, or HTTP webhook */}
          {hook.handler_type === "command" ? (
            <div>
              <div style={labelStyle}>Shell Command</div>
              <input
                type="text"
                value={hook.command}
                onChange={(e) => update({ command: e.target.value })}
                placeholder="sh .vibecli/hooks/lint.sh"
                style={inputStyle}
              />
              <div style={{ fontSize: "10px", color: "var(--text-secondary)", marginTop: "3px" }}>
                Event JSON piped to stdin. Exit 0 = allow, exit 2 = block, stdout = context injection.
              </div>
            </div>
          ) : hook.handler_type === "http" ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
              <div style={{ display: "flex", gap: "10px" }}>
                <div style={{ flex: 1 }}>
                  <div style={labelStyle}>Webhook URL</div>
                  <input
                    type="text"
                    value={hook.http_url}
                    onChange={(e) => update({ http_url: e.target.value })}
                    placeholder="https://example.com/webhook"
                    style={inputStyle}
                  />
                </div>
                <div style={{ width: "100px" }}>
                  <div style={labelStyle}>Method</div>
                  <select
                    value={hook.http_method}
                    onChange={(e) => update({ http_method: e.target.value })}
                    style={inputStyle}
                  >
                    <option value="POST">POST</option>
                    <option value="PUT">PUT</option>
                    <option value="PATCH">PATCH</option>
                    <option value="GET">GET</option>
                  </select>
                </div>
              </div>
              <div>
                <div style={labelStyle}>Headers (JSON, optional)</div>
                <input
                  type="text"
                  value={hook.http_headers}
                  onChange={(e) => update({ http_headers: e.target.value })}
                  placeholder='{"Authorization": "Bearer ..."}'
                  style={{ ...inputStyle, fontFamily: "monospace", fontSize: "10px" }}
                />
              </div>
              <div>
                <div style={labelStyle}>Timeout (ms)</div>
                <input
                  type="number"
                  value={hook.http_timeout_ms}
                  onChange={(e) => update({ http_timeout_ms: parseInt(e.target.value) || 10000 })}
                  style={{ ...inputStyle, width: "100px" }}
                />
              </div>
              <div style={{ fontSize: "10px", color: "var(--text-secondary)" }}>
                POSTs event JSON to URL. Response: {`{"decision": "allow"|"block"|"inject", "reason": "...", "context": "..."}`}
              </div>
            </div>
          ) : (
            <div>
              <div style={labelStyle}>LLM Prompt Template</div>
              <textarea
                value={hook.prompt}
                onChange={(e) => update({ prompt: e.target.value })}
                placeholder='Check if this tool call is safe. Event: {{event}}. Reply {"ok": true} or {"ok": false, "reason": "..."}'
                rows={3}
                style={{ ...inputStyle, resize: "vertical", fontFamily: "inherit" }}
              />
            </div>
          )}

          {/* Async toggle */}
          <label style={{ display: "flex", alignItems: "center", gap: "8px", cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={hook.async_exec}
              onChange={(e) => update({ async_exec: e.target.checked })}
            />
            <span style={{ fontSize: "11px", color: "var(--text-primary)" }}>
              Run asynchronously (never blocks the agent)
            </span>
          </label>
        </div>
      )}
    </div>
  );
}

// ── HooksPanel ────────────────────────────────────────────────────────────────

export function HooksPanel({ workspacePath }: HooksPanelProps) {
  const [hooks, setHooks] = useState<HookConfig[]>([]);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<HookConfig[]>("get_hooks_config", { workspacePath: workspacePath || null })
      .then((h) => { if (!cancelled) setHooks(h); })
      .catch(() => { if (!cancelled) setHooks([]); });
    return () => { cancelled = true; };
  }, [workspacePath]);

  function addHook() {
    setHooks((prev) => [
      ...prev,
      { event: "PreToolUse", tools: [], handler_type: "command", command: "", prompt: "", http_url: "", http_method: "POST", http_headers: "", http_timeout_ms: 10000, async_exec: false },
    ]);
    setDirty(true);
  }

  const updateHook = useCallback((i: number, h: HookConfig) => {
    setHooks((prev) => prev.map((x, idx) => (idx === i ? h : x)));
    setDirty(true);
  }, []);

  const deleteHook = useCallback((i: number) => {
    setHooks((prev) => prev.filter((_, idx) => idx !== i));
    setDirty(true);
  }, []);

  async function save() {
    setSaving(true);
    setSaveMsg(null);
    try {
      await invoke("save_hooks_config", { hooks, workspacePath: workspacePath || null });
      setDirty(false);
      setSaveMsg("Saved.");
    } catch (e: unknown) {
      setSaveMsg(`Error: ${e}`);
    } finally {
      setSaving(false);
    }
  }

  const scope = workspacePath ? "workspace (.vibecli/hooks.json)" : "global (~/.vibecli/hooks.json)";

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div style={{
        padding: "10px 12px",
        borderBottom: "1px solid var(--border-color)",
        display: "flex",
        alignItems: "center",
        gap: "8px",
        flexShrink: 0,
      }}>
        <span style={{ fontSize: "16px" }}>🪝</span>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-primary)" }}>
            Hooks
          </div>
          <div style={{ fontSize: "10px", color: "var(--text-secondary)" }}>
            {scope}
          </div>
        </div>
        <button
          onClick={addHook}
          style={{
            fontSize: "11px",
            padding: "4px 10px",
            background: "none",
            border: "1px dashed var(--border-color)",
            borderRadius: "3px",
            color: "var(--text-secondary)",
            cursor: "pointer",
          }}
        >
          + Add Hook
        </button>
        <button
          onClick={save}
          disabled={!dirty || saving}
          style={{
            fontSize: "11px",
            padding: "4px 10px",
            background: dirty ? "var(--accent-blue, #007acc)" : "var(--bg-secondary)",
            color: dirty ? "#fff" : "var(--text-secondary)",
            border: "none",
            borderRadius: "3px",
            cursor: dirty ? "pointer" : "not-allowed",
            fontWeight: 600,
          }}
        >
          {saving ? "Saving…" : "Save"}
        </button>
      </div>

      {/* Hook list */}
      <div style={{ flex: 1, overflowY: "auto", padding: "10px" }}>
        {saveMsg && (
          <div style={{
            padding: "6px 10px",
            marginBottom: "8px",
            fontSize: "11px",
            color: saveMsg.startsWith("Error") ? "#f44" : "#4c4",
            background: saveMsg.startsWith("Error") ? "#f441" : "#4c41",
            border: `1px solid ${saveMsg.startsWith("Error") ? "#f44" : "#4c4"}`,
            borderRadius: "4px",
          }}>
            {saveMsg}
          </div>
        )}

        {hooks.length === 0 ? (
          <div style={{ padding: "24px 0", textAlign: "center", color: "var(--text-secondary)" }}>
            <div style={{ fontSize: "24px", marginBottom: "8px" }}>🪝</div>
            <div style={{ fontSize: "13px" }}>No hooks configured.</div>
            <div style={{ fontSize: "11px", marginTop: "4px", opacity: 0.7 }}>
              Hooks run shell commands or LLM checks before/after agent tool calls.
            </div>
            <button
              onClick={addHook}
              style={{
                marginTop: "12px",
                fontSize: "12px",
                padding: "6px 16px",
                background: "var(--accent-blue, #007acc)",
                color: "#fff",
                border: "none",
                borderRadius: "4px",
                cursor: "pointer",
              }}
            >
              + Add First Hook
            </button>
          </div>
        ) : (
          hooks.map((hook, i) => (
            <HookRow
              key={i}
              hook={hook}
              index={i}
              onChange={updateHook}
              onDelete={deleteHook}
            />
          ))
        )}

        {/* Hook reference */}
        {hooks.length > 0 && (
          <div style={{ marginTop: "12px", padding: "10px", background: "var(--bg-secondary)", borderRadius: "5px", fontSize: "10px", color: "var(--text-secondary)" }}>
            <div style={{ fontWeight: 600, marginBottom: "4px" }}>Hook Protocol</div>
            <div><b>Shell:</b> Exit 0 → allow, Exit 2 → block, stdout JSON → context injection</div>
            <div><b>LLM:</b> Prompt receives event JSON, reply {`{"ok": true/false}`}</div>
            <div><b>HTTP:</b> POST event JSON to URL, response {`{"decision": "allow"|"block"|"inject"}`}</div>
          </div>
        )}
      </div>
    </div>
  );
}
