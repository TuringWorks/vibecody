import { useState, useCallback, useEffect, useMemo } from "react";
import { daemonFetch } from "../lib/daemonFetch";

interface CredentialField {
  env: string;
  label: string;
  help: string;
}

interface ConnectorSpec {
  id: string;
  title: string;
  description: string;
  command: string;
  args: string[];
  runtime: string;
  runtime_program: string;
  runtime_available: boolean;
  credentials: CredentialField[];
  docs_url: string;
}

interface Connector {
  id: string;
  catalog_id: string | null;
  title: string;
  command: string;
  args: string[];
  enabled: boolean;
  added_at: number | null;
  credential_names: string[];
  missing_credentials: string[];
}

/** Mirrors `connectors::ProbeResult` on the daemon. */
type ProbeResult =
  | { state: "ok"; tools: string[] }
  | { state: "failed"; error: string }
  | { state: "timedout"; after_secs: number };

interface ProbeRecord {
  result: ProbeResult;
  checked_at: number;
}

interface ConnectorsPanelProps {
  workspacePath?: string | null;
  daemonUrl?: string;
}

const dotStyle = (color: string): React.CSSProperties => ({
  width: 8,
  height: 8,
  borderRadius: "50%",
  background: color,
  display: "inline-block",
  flex: "none",
});

/**
 * Connectors: MCP servers this workspace can talk to, and their credentials.
 *
 * This panel used to be a facade. `connectors_add` was called with an empty API
 * key — there was no field to type one into — and recorded the connector as
 * `status: "connected"` regardless; `connectors_test` reported healthy when the
 * row existed in a `Vec`; "Auto-Detect Services" returned the hard-coded vendor
 * list minus whatever had been added, having scanned nothing; and the whole
 * thing lived in memory, so it was empty again after a restart. Every green dot
 * on this screen was a claim nobody had checked.
 *
 * It now drives the daemon's `/api/vibedesk/connectors*` routes: definitions in
 * the workspace store, credentials encrypted in `workspace_secrets`, and a Test
 * button that actually launches the server and lists its tools. A connector
 * reads "Not checked" until it has been run — there is no state here that can
 * be inferred from a key being present.
 */
export function ConnectorsPanel({
  workspacePath,
  daemonUrl = "http://localhost:7878",
}: ConnectorsPanelProps) {
  const [tab, setTab] = useState<"connected" | "available">("connected");
  const [connectors, setConnectors] = useState<Connector[] | null>(null);
  const [catalog, setCatalog] = useState<ConnectorSpec[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<ReadonlySet<string>>(new Set());
  const [probes, setProbes] = useState<Record<string, ProbeRecord>>({});
  const [drafts, setDrafts] = useState<Record<string, Record<string, string>>>({});

  const scope = useMemo(
    () => (workspacePath ? `?path=${encodeURIComponent(workspacePath)}` : ""),
    [workspacePath],
  );

  const load = useCallback(async () => {
    try {
      const res = await daemonFetch(`${daemonUrl}/api/vibedesk/connectors${scope}`);
      if (!res.ok) throw new Error(`daemon returned ${res.status}`);
      const body: { connectors: Connector[]; catalog: ConnectorSpec[] } = await res.json();
      setConnectors(body.connectors);
      setCatalog(body.catalog);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [daemonUrl, scope]);

  useEffect(() => {
    void load();
  }, [load]);

  /** POST one action, reporting the daemon's own message when it fails. */
  const post = useCallback(
    async (key: string, path: string, body: Record<string, unknown>, describe: (r: never) => string) => {
      setBusy((prev) => new Set(prev).add(key));
      setNotice(null);
      try {
        const res = await daemonFetch(`${daemonUrl}/api/vibedesk/connectors${path}`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ ...body, path: workspacePath ?? null }),
        });
        const json = await res.json().catch(() => ({}));
        if (!res.ok) {
          // The route's message names the actual reason — a missing
          // credential, a duplicate id. Replacing it with the status code
          // throws away the only useful sentence.
          throw new Error(json?.error ?? `daemon returned ${res.status}`);
        }
        setNotice(describe(json as never));
        await load();
      } catch (e) {
        setNotice(`Failed: ${String(e)}`);
      } finally {
        setBusy((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
      }
    },
    [daemonUrl, workspacePath, load],
  );

  const add = (spec: ConnectorSpec) =>
    post(
      spec.id,
      "",
      { catalog_id: spec.id, credentials: drafts[spec.id] ?? {} },
      () => {
        setDrafts((d) => ({ ...d, [spec.id]: {} }));
        return `Added ${spec.title}. Press Test to check it actually starts.`;
      },
    );

  const toggle = (c: Connector) =>
    post(c.id, "/toggle", { id: c.id, enabled: !c.enabled }, () =>
      c.enabled ? `${c.title} disabled.` : `${c.title} enabled.`,
    );

  const remove = (c: Connector) =>
    post(c.id, "/remove", { id: c.id }, (json: never) => {
      const deleted = (json as { secrets_deleted?: number }).secrets_deleted ?? 0;
      return deleted > 0
        ? `Removed ${c.title} and deleted ${deleted} stored credential${deleted === 1 ? "" : "s"}.`
        : `Removed ${c.title}.`;
    });

  const probe = (c: Connector) =>
    post(c.id, "/probe", { id: c.id }, (json: never) => {
      const record = json as unknown as ProbeRecord;
      setProbes((prev) => ({ ...prev, [c.id]: record }));
      switch (record.result.state) {
        case "ok":
          return `${c.title} started and offered ${record.result.tools.length} tool${
            record.result.tools.length === 1 ? "" : "s"
          }.`;
        case "timedout":
          return `${c.title} started but said nothing within ${record.result.after_secs}s.`;
        case "failed":
          return `${c.title} could not start: ${record.result.error}`;
      }
    });

  const configuredIds = useMemo(
    () => new Set((connectors ?? []).map((c) => c.id)),
    [connectors],
  );
  const available = useMemo(
    () => catalog.filter((c) => !configuredIds.has(c.id)),
    [catalog, configuredIds],
  );

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 4, color: "var(--text-primary)" }}>
        Connectors
      </h2>
      <p style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 12 }}>
        MCP servers this workspace can talk to. Credentials are stored encrypted in the
        workspace, never in a file. Reachable from <code>vibecli</code>&rsquo;s <code>/mcp</code>{" "}
        command — agent runs do not use MCP tools yet.
      </p>

      <div className="panel-tab-bar">
        {(["connected", "available"] as const).map((t) => (
          <button
            key={t}
            className={`panel-tab ${tab === t ? "active" : ""}`}
            onClick={() => setTab(t)}
          >
            {t === "connected" ? "Configured" : "Available"}
          </button>
        ))}
      </div>

      {error && <div className="panel-empty">Cannot reach the daemon: {error}</div>}
      {notice && (
        <div className="panel-card" style={{ marginBottom: 8, fontSize: "var(--font-size-sm)" }}>
          {notice}
        </div>
      )}

      {!error && tab === "connected" && (
        <div>
          <div style={{ marginBottom: 8 }}>
            <button className="panel-btn panel-btn-primary" onClick={() => void load()}>
              Refresh
            </button>
          </div>
          {connectors === null && <div className="panel-empty">Loading…</div>}
          {connectors?.length === 0 && (
            <div className="panel-empty">
              None configured yet. Open Available to add one.
            </div>
          )}
          {connectors?.map((c) => {
            const working = busy.has(c.id);
            const spec = catalog.find((s) => s.id === c.catalog_id);
            const record = probes[c.id];
            return (
              <div key={c.id} className="panel-card" style={{ marginBottom: 8 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span
                    style={dotStyle(
                      c.enabled ? "var(--success-color)" : "var(--text-secondary)",
                    )}
                  />
                  <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{c.title}</span>
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                    {c.enabled ? "enabled" : "disabled"}
                  </span>
                </div>
                <div
                  className="panel-mono"
                  style={{ marginTop: 6, fontSize: "var(--font-size-sm)", wordBreak: "break-all" }}
                >
                  {c.command} {c.args.join(" ")}
                </div>

                {c.missing_credentials.length > 0 && (
                  <div style={{ marginTop: 6, fontSize: "var(--font-size-sm)", color: "var(--warning-color)" }}>
                    Missing credential{c.missing_credentials.length === 1 ? "" : "s"}:{" "}
                    {c.missing_credentials.join(", ")} — remove and add it again to supply{" "}
                    {c.missing_credentials.length === 1 ? "it" : "them"}.
                  </div>
                )}
                {spec && !spec.runtime_available && (
                  <div style={{ marginTop: 6, fontSize: "var(--font-size-sm)", color: "var(--warning-color)" }}>
                    <code>{spec.runtime_program}</code> is not on this machine&rsquo;s PATH, so this
                    connector cannot start.
                  </div>
                )}

                <ProbeLine record={record} />

                <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                  <button
                    className="panel-btn panel-btn-primary"
                    style={{ fontSize: "var(--font-size-sm)", padding: "4px 12px" }}
                    disabled={working}
                    onClick={() => void probe(c)}
                  >
                    {working ? "Working…" : "Test"}
                  </button>
                  <button
                    className="panel-btn panel-btn-secondary"
                    style={{ fontSize: "var(--font-size-sm)", padding: "4px 12px" }}
                    disabled={working}
                    onClick={() => void toggle(c)}
                  >
                    {c.enabled ? "Disable" : "Enable"}
                  </button>
                  <button
                    className="panel-btn panel-btn-secondary"
                    style={{ fontSize: "var(--font-size-sm)", padding: "4px 12px" }}
                    disabled={working}
                    onClick={() => void remove(c)}
                  >
                    Remove
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {!error && tab === "available" && (
        <div>
          {available.length === 0 && (
            <div className="panel-empty">Every catalog connector is already configured.</div>
          )}
          {available.map((spec) => {
            const working = busy.has(spec.id);
            const draft = drafts[spec.id] ?? {};
            const incomplete = spec.credentials.some((f) => !(draft[f.env] ?? "").trim());
            return (
              <div key={spec.id} className="panel-card" style={{ marginBottom: 8 }}>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{spec.title}</div>
                <div
                  style={{
                    fontSize: "var(--font-size-sm)",
                    color: "var(--text-secondary)",
                    marginTop: 4,
                    lineHeight: 1.5,
                  }}
                >
                  {spec.description}
                </div>
                <div
                  className="panel-mono"
                  style={{ marginTop: 6, fontSize: "var(--font-size-sm)", wordBreak: "break-all" }}
                >
                  {spec.command} {spec.args.join(" ")}
                </div>
                {!spec.runtime_available && (
                  <div style={{ marginTop: 6, fontSize: "var(--font-size-sm)", color: "var(--warning-color)" }}>
                    <code>{spec.runtime_program}</code> is not on PATH. You can still add this, but
                    it will not start until that is installed.
                  </div>
                )}

                {spec.credentials.map((field) => (
                  <label key={field.env} style={{ display: "block", marginTop: 8 }}>
                    <span style={{ display: "block", fontSize: "var(--font-size-sm)", marginBottom: 3 }}>
                      {field.label}
                    </span>
                    <input
                      type="password"
                      className="panel-input"
                      autoComplete="off"
                      placeholder={field.env}
                      value={draft[field.env] ?? ""}
                      onChange={(e) =>
                        setDrafts((d) => ({
                          ...d,
                          [spec.id]: { ...(d[spec.id] ?? {}), [field.env]: e.target.value },
                        }))
                      }
                      style={{ width: "100%", boxSizing: "border-box" }}
                    />
                    <span
                      style={{
                        display: "block",
                        marginTop: 3,
                        fontSize: "var(--font-size-sm)",
                        color: "var(--text-secondary)",
                        lineHeight: 1.45,
                      }}
                    >
                      {field.help}
                    </span>
                  </label>
                ))}

                <div style={{ display: "flex", gap: 8, marginTop: 8, alignItems: "center" }}>
                  <button
                    className="panel-btn panel-btn-primary"
                    style={{ fontSize: "var(--font-size-sm)", padding: "4px 12px" }}
                    disabled={working || incomplete}
                    onClick={() => void add(spec)}
                  >
                    {working ? "Adding…" : "Add"}
                  </button>
                  <a
                    href={spec.docs_url}
                    target="_blank"
                    rel="noreferrer"
                    style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}
                  >
                    Docs
                  </a>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** The last measured outcome, or an explicit "not checked". Never a guess. */
function ProbeLine({ record }: { record?: ProbeRecord }) {
  if (!record) {
    return (
      <div style={{ marginTop: 6, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
        Not checked yet.
      </div>
    );
  }
  const when = new Date(record.checked_at).toLocaleTimeString();
  const style = (color: string): React.CSSProperties => ({
    marginTop: 6,
    fontSize: "var(--font-size-sm)",
    color,
    lineHeight: 1.45,
  });
  switch (record.result.state) {
    case "ok":
      return (
        <div style={style("var(--success-color)")}>
          Started at {when}: {record.result.tools.length} tool
          {record.result.tools.length === 1 ? "" : "s"}
          {record.result.tools.length > 0 && ` — ${record.result.tools.slice(0, 6).join(", ")}`}
          {record.result.tools.length > 6 && "…"}
        </div>
      );
    case "timedout":
      return (
        <div style={style("var(--warning-color)")}>
          Started but silent for {record.result.after_secs}s (checked {when}).
        </div>
      );
    case "failed":
      return (
        <div style={style("var(--error-color)")}>
          Failed at {when}: {record.result.error}
        </div>
      );
  }
}
