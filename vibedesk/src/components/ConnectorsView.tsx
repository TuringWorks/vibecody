import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Check, Loader2, Plug, Trash2, X, Zap } from "lucide-react";

/**
 * Connectors — the MCP servers this workspace can talk to.
 *
 * Split out of the Plugins marketplace, where connectors shared a list with
 * plugin bundles. They are not the same thing: a plugin ships skills and hooks
 * inside a signed bundle, a connector is a *process* that either starts and
 * offers tools or does not. That difference is the whole content of this view —
 * every row answers "can the agent actually use this", and the answer comes
 * from having launched it, not from it being installed.
 */

interface ConnectorsViewProps {
  daemonUrl: string;
  /** Workspace whose connectors these are. Same value the shell hands PluginsView. */
  path?: string;
  onClose: () => void;
}

interface CredentialField {
  env: string;
  label: string;
  help: string;
}

interface ConnectorSpec {
  id: string;
  title: string;
  description: string;
  category: string;
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

type Loaded<T> =
  | { state: "loading" }
  | { state: "ready"; value: T }
  | { state: "error"; message: string };

interface Listing {
  connectors: Connector[];
  catalog: ConnectorSpec[];
}

/**
 * How a probe reads to a human.
 *
 * `ok` with an empty tool list is the case this exists for. It is a successful
 * handshake with a server that offers the agent nothing, and rendering it as
 * plain "ok" is how a broken connector looked healthy: an MCP client bug made
 * `server-everything` report 0 of its 13 tools, and the panel showed a tick.
 * Zero tools is reported as its own outcome, never as success.
 */
function describeProbe(result: ProbeResult): { tone: "ok" | "warn" | "bad"; text: string } {
  switch (result.state) {
    case "ok":
      return result.tools.length === 0
        ? { tone: "warn", text: "Started, but offered no tools" }
        : { tone: "ok", text: `${result.tools.length} tools` };
    case "failed":
      return { tone: "bad", text: result.error };
    case "timedout":
      return { tone: "bad", text: `No answer after ${result.after_secs}s` };
    default: {
      // Exhaustive: a new state must be handled, not fall through as working.
      const never: never = result;
      return { tone: "bad", text: `Unknown probe state: ${JSON.stringify(never)}` };
    }
  }
}

export function ConnectorsView({ daemonUrl, path, onClose }: ConnectorsViewProps) {
  const [listing, setListing] = useState<Loaded<Listing>>({ state: "loading" });
  const [busy, setBusy] = useState<string | null>(null);
  const [probes, setProbes] = useState<Record<string, ProbeRecord | "running">>({});
  const [drafts, setDrafts] = useState<Record<string, Record<string, string>>>({});
  const [openForm, setOpenForm] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const value = await invoke<Listing>("list_connectors", { url: daemonUrl, path });
      setListing({ state: "ready", value });
    } catch (e) {
      // An empty list would read as "this workspace has no connectors", which
      // is a different and wrong statement from "we could not ask".
      setListing({ state: "error", message: e instanceof Error ? e.message : String(e) });
    }
  }, [daemonUrl, path]);

  useEffect(() => {
    void load();
  }, [load]);

  const configured = useMemo(() => {
    const map = new Map<string, Connector>();
    if (listing.state === "ready") {
      for (const c of listing.value.connectors) map.set(c.catalog_id ?? c.id, c);
    }
    return map;
  }, [listing]);

  const act = useCallback(
    async (key: string, run: () => Promise<string | null>) => {
      setBusy(key);
      setNotice(null);
      try {
        const message = await run();
        if (message) setNotice(message);
        await load();
      } catch (e) {
        setNotice(e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(null);
      }
    },
    [load]
  );

  const addConnector = (spec: ConnectorSpec) =>
    act(`add:${spec.id}`, async () => {
      // `catalogId`, never a bare `id`: the daemon reads an id with no command
      // as a *custom* connector and answers 400 "a command is required".
      await invoke("add_connector", {
        url: daemonUrl,
        path,
        catalogId: spec.id,
        credentials: drafts[spec.id] ?? {},
      });
      setDrafts(d => ({ ...d, [spec.id]: {} }));
      setOpenForm(null);
      return `Added ${spec.title}. Use Test to check it actually starts.`;
    });

  const toggleConnector = (c: Connector) =>
    act(`toggle:${c.id}`, async () => {
      await invoke("toggle_connector", { url: daemonUrl, path, id: c.id, enabled: !c.enabled });
      return null;
    });

  const removeConnector = (c: Connector) =>
    act(`remove:${c.id}`, async () => {
      const res = await invoke<{ secrets_deleted: number }>("remove_connector", {
        url: daemonUrl,
        path,
        id: c.id,
      });
      setProbes(p => {
        const next = { ...p };
        delete next[c.id];
        return next;
      });
      return res.secrets_deleted > 0
        ? `Removed ${c.title} and ${res.secrets_deleted} stored credential(s).`
        : `Removed ${c.title}.`;
    });

  const probeConnector = async (c: Connector) => {
    setProbes(p => ({ ...p, [c.id]: "running" }));
    try {
      const record = await invoke<ProbeRecord>("probe_connector", {
        url: daemonUrl,
        path,
        id: c.id,
      });
      setProbes(p => ({ ...p, [c.id]: record }));
    } catch (e) {
      setProbes(p => ({
        ...p,
        [c.id]: {
          result: { state: "failed", error: e instanceof Error ? e.message : String(e) },
          checked_at: Date.now(),
        },
      }));
    }
  };

  const catalog = listing.state === "ready" ? listing.value.catalog : [];
  const categories = useMemo(() => {
    const groups = new Map<string, ConnectorSpec[]>();
    for (const spec of catalog) {
      const list = groups.get(spec.category) ?? [];
      list.push(spec);
      groups.set(spec.category, list);
    }
    return [...groups.entries()];
  }, [catalog]);

  return (
    <div className="vx-market">
      <div className="vx-market__head">
        <span className="vx-market__title">Connectors</span>
        <button className="vx-icon-btn" aria-label="Close connectors" onClick={onClose}>
          <X size={16} />
        </button>
      </div>

      {notice && <div className="vx-market__notice">{notice}</div>}

      {listing.state === "error" && (
        <div className="vx-market__error" role="alert">
          Could not reach the daemon: {listing.message}
        </div>
      )}

      {listing.state === "loading" && <div className="vx-market__empty">Loading connectors…</div>}

      {listing.state === "ready" &&
        categories.map(([category, specs]) => (
          <section key={category} className="vx-market__section">
            <div className="vx-market__section-label">{category}</div>
            {specs.map(spec => {
              const existing = configured.get(spec.id);
              const probe = existing ? probes[existing.id] : undefined;
              const verdict =
                probe && probe !== "running" ? describeProbe(probe.result) : null;

              return (
                <div key={spec.id} className="vx-market__row" data-testid={`connector-${spec.id}`}>
                  <div className="vx-market__row-main">
                    <span className="vx-market__row-icon">
                      <Plug size={14} />
                    </span>
                    <div>
                      <div className="vx-market__row-title">{spec.title}</div>
                      <div className="vx-market__row-desc">{spec.description}</div>
                      <div className="vx-market__row-meta">
                        <span>{spec.runtime}</span>
                        {!spec.runtime_available && (
                          <span
                            className="vx-market__warn"
                            data-testid="connector-runtime-warning"
                            title={`${spec.runtime_program} is not on PATH`}
                          >
                            <AlertTriangle size={12} /> {spec.runtime_program} not installed
                          </span>
                        )}
                        {existing && (
                          <span data-testid="connector-state">
                            {existing.enabled ? "Added · enabled" : "Added · disabled"}
                          </span>
                        )}
                        {existing && existing.missing_credentials.length > 0 && (
                          <span className="vx-market__warn">
                            <AlertTriangle size={12} /> missing:{" "}
                            {existing.missing_credentials.join(", ")}
                          </span>
                        )}
                      </div>

                      {verdict && (
                        <div
                          className={`vx-market__probe is-${verdict.tone}`}
                          data-testid="connector-probe"
                        >
                          {verdict.tone === "ok" ? <Check size={12} /> : <AlertTriangle size={12} />}{" "}
                          {verdict.text}
                        </div>
                      )}
                      {probe === "running" && (
                        <div className="vx-market__probe" data-testid="connector-probe">
                          <Loader2 size={12} className="spin" /> Testing…
                        </div>
                      )}
                      {verdict?.tone === "ok" &&
                        probe !== "running" &&
                        probe &&
                        probe.result.state === "ok" && (
                          <div className="vx-market__tools">
                            {probe.result.tools.slice(0, 12).join(", ")}
                          </div>
                        )}
                    </div>
                  </div>

                  <div className="vx-market__row-actions">
                    {!existing && spec.credentials.length > 0 && openForm !== spec.id && (
                      <button
                        className="vx-btn"
                        onClick={() => setOpenForm(spec.id)}
                        disabled={busy !== null}
                      >
                        Add…
                      </button>
                    )}
                    {!existing && spec.credentials.length === 0 && (
                      <button
                        className="vx-btn"
                        onClick={() => addConnector(spec)}
                        disabled={busy !== null}
                      >
                        Add
                      </button>
                    )}
                    {existing && (
                      <>
                        <button
                          className="vx-btn"
                          onClick={() => probeConnector(existing)}
                          disabled={probe === "running"}
                        >
                          <Zap size={12} /> Test
                        </button>
                        <button
                          className="vx-btn"
                          onClick={() => toggleConnector(existing)}
                          disabled={busy !== null}
                        >
                          {existing.enabled ? "Disable" : "Enable"}
                        </button>
                        <button
                          className="vx-btn vx-btn--danger"
                          aria-label={`Remove ${spec.title}`}
                          onClick={() => removeConnector(existing)}
                          disabled={busy !== null}
                        >
                          <Trash2 size={12} />
                        </button>
                      </>
                    )}
                  </div>

                  {openForm === spec.id && !existing && (
                    <form
                      className="vx-market__form"
                      onSubmit={e => {
                        e.preventDefault();
                        void addConnector(spec);
                      }}
                    >
                      {spec.credentials.map(field => (
                        <label key={field.env} className="vx-market__field">
                          <span>{field.label}</span>
                          <input
                            type="password"
                            autoComplete="off"
                            value={drafts[spec.id]?.[field.env] ?? ""}
                            onChange={e =>
                              setDrafts(d => ({
                                ...d,
                                [spec.id]: { ...(d[spec.id] ?? {}), [field.env]: e.target.value },
                              }))
                            }
                          />
                          {field.help && <small>{field.help}</small>}
                        </label>
                      ))}
                      <div className="vx-market__form-actions">
                        <button className="vx-btn" type="submit" disabled={busy !== null}>
                          Add {spec.title}
                        </button>
                        <button className="vx-btn" type="button" onClick={() => setOpenForm(null)}>
                          Cancel
                        </button>
                      </div>
                    </form>
                  )}
                </div>
              );
            })}
          </section>
        ))}
    </div>
  );
}
