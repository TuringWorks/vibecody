import { useMemo, useState } from "react";

// A1 — MCP Apps generic React embedding host (SEP-1865).
//
// Renders `application/vnd.mcp.app+json` payloads inline in chat. The
// backend parser (`mcp_apps_payload.rs`) is the authoritative validator;
// this component is the safe render side.
//
// Safety posture:
//   - Component is named (`react@18`, …), not arbitrary JSX. Unknown
//     component refs render a clear "unsupported" message — the host
//     never executes payload-supplied code.
//   - Props are shown read-only; the user has full visibility into
//     what the app brought.
//   - Actions are simple buttons. Clicking dispatches a window event
//     `vibeui:mcp-app-action` with `{ action_id, label, props }`. The
//     chat layer subscribes and decides what to do (insert into
//     input, send to the agent, etc.). The component itself does NOT
//     touch the network — that's a parent concern.
//   - CSP declarations from the payload are surfaced verbatim so the
//     operator can see what the app would have wanted; we never act
//     on them in this minimal host. A future iframe-sandboxed
//     renderer can promote them to real CSP headers.

export interface McpAppAction {
  id: string;
  label: string;
  description?: string | null;
}

export interface McpAppCsp {
  allowHttp?: string[];
  allowScript?: string[];
}

export interface McpAppPayload {
  type: "mcp.app";
  version: string;
  title: string;
  component: string;
  props?: unknown;
  actions?: McpAppAction[];
  csp?: McpAppCsp | null;
}

/// Component names this host will render. Any value not in this set
/// renders the "unsupported" fallback — no arbitrary JSX execution.
const SUPPORTED_COMPONENTS = new Set<string>([
  "react@18",
  "react@19",
  "json-view",
  "list",
  "card",
]);

interface Props {
  payload: McpAppPayload;
  /// Optional event source identifier so multiple embeds on screen
  /// don't conflate their action events. Defaults to the title.
  sourceId?: string;
}

export function McpAppEmbed({ payload, sourceId }: Props) {
  const [collapsed, setCollapsed] = useState(false);
  const supported = SUPPORTED_COMPONENTS.has(payload.component);
  const propsJson = useMemo(() => {
    try {
      return JSON.stringify(payload.props ?? null, null, 2);
    } catch {
      return "<unrenderable props>";
    }
  }, [payload.props]);

  function fireAction(action: McpAppAction) {
    window.dispatchEvent(
      new CustomEvent("vibeui:mcp-app-action", {
        detail: {
          source_id: sourceId ?? payload.title,
          action_id: action.id,
          label: action.label,
          props: payload.props ?? null,
        },
      }),
    );
  }

  return (
    <div
      style={{
        background: "var(--bg-secondary)",
        border: "1px solid var(--border-color)",
        borderRadius: "var(--radius-sm-alt)",
        padding: 12,
        marginTop: 8,
        marginBottom: 8,
      }}
    >
      <div style={{ display: "flex", alignItems: "baseline", gap: 8, marginBottom: 8 }}>
        <span
          style={{
            fontSize: "var(--font-size-xs)",
            color: "var(--accent-color)",
            fontWeight: 600,
            padding: "2px 6px",
            border: "1px solid var(--accent-color)",
            borderRadius: "var(--radius-md)",
          }}
        >
          MCP App
        </span>
        <span style={{ fontWeight: 600 }}>{payload.title}</span>
        <span style={{ color: "var(--text-muted)", fontSize: "var(--font-size-xs)" }}>
          {payload.component} · v{payload.version}
        </span>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => setCollapsed((c) => !c)}
          style={{
            marginLeft: "auto",
            padding: "2px 8px",
            fontSize: "var(--font-size-xs)",
          }}
        >
          {collapsed ? "Show" : "Hide"}
        </button>
      </div>

      {!collapsed && (
        <>
          {!supported && (
            <div
              style={{
                color: "var(--warning-color)",
                fontSize: "var(--font-size-sm)",
                marginBottom: 8,
                padding: "6px 8px",
                background: "var(--warning-color)11",
                borderRadius: "var(--radius-xs-plus)",
              }}
            >
              Unsupported component: <code>{payload.component}</code>. This MCP App
              requires a renderer not built into VibeUI. Props are shown below for
              inspection but no UI is rendered.
            </div>
          )}

          <details style={{ marginBottom: 8 }}>
            <summary
              style={{
                cursor: "pointer",
                fontSize: "var(--font-size-sm)",
                color: "var(--text-muted)",
              }}
            >
              Props
            </summary>
            <pre
              style={{
                margin: "6px 0 0 0",
                padding: 8,
                background: "var(--bg-primary)",
                borderRadius: "var(--radius-xs-plus)",
                fontSize: "var(--font-size-xs)",
                fontFamily: "var(--font-mono)",
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                maxHeight: 240,
                overflowY: "auto",
              }}
            >
              {propsJson}
            </pre>
          </details>

          {payload.csp && (payload.csp.allowHttp?.length || payload.csp.allowScript?.length) ? (
            <details style={{ marginBottom: 8 }}>
              <summary
                style={{
                  cursor: "pointer",
                  fontSize: "var(--font-size-sm)",
                  color: "var(--text-muted)",
                }}
              >
                CSP declarations (informational)
              </summary>
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", padding: "6px 0" }}>
                {payload.csp.allowHttp?.length ? (
                  <div>allowHttp: <code>{payload.csp.allowHttp.join(", ")}</code></div>
                ) : null}
                {payload.csp.allowScript?.length ? (
                  <div>allowScript: <code>{payload.csp.allowScript.join(", ")}</code></div>
                ) : null}
              </div>
            </details>
          ) : null}

          {payload.actions && payload.actions.length > 0 && (
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {payload.actions.map((a) => (
                <button
                  key={a.id}
                  className="panel-btn"
                  onClick={() => fireAction(a)}
                  title={a.description ?? undefined}
                  style={{
                    padding: "4px 10px",
                    borderRadius: "var(--radius-sm)",
                    fontSize: "var(--font-size-sm)",
                    background: "var(--accent-color)",
                    color: "var(--btn-primary-fg, #fff)",
                    border: "none",
                    fontWeight: 500,
                  }}
                >
                  {a.label}
                </button>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
