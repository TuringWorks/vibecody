import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  X, Plug, Server, Sparkles, Bot, Scale, Webhook, Download, Trash2, Power,
  PlugZap, AlertTriangle, Check, Loader2, ExternalLink,
} from "lucide-react";

interface PluginsViewProps {
  daemonUrl: string;
  /** Workspace whose plugin policy applies. */
  path?: string;
  onClose: () => void;
}

/** One enabled component. Hooks additionally carry the event they fire on. */
interface Component {
  plugin: string;
  name: string;
  policy: string;
  event?: string;
}

interface Inventory {
  available: boolean;
  total: number;
  mcp_servers: Component[];
  skills: Component[];
  subagents: Component[];
  rules: Component[];
  hooks: Component[];
}

interface CatalogPlugin {
  name: string;
  title: string;
  version: string;
  description: string;
  components: { kind: string; name: string }[];
  installed: boolean;
  /** `null` until installed; "on" | "off" | "required" after. */
  policy: string | null;
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

/** What a probe found. Mirrors `connectors::ProbeResult` on the daemon. */
type ProbeResult =
  | { state: "ok"; tools: string[] }
  | { state: "failed"; error: string }
  | { state: "timedout"; after_secs: number };

/** A probe's outcome plus when it was taken — a result with no time is a claim
 *  about now made from a measurement of then. */
interface ProbeRecord {
  result: ProbeResult;
  checked_at: number;
}

type Tab = "active" | "catalog" | "connectors";

const SECTIONS: {
  key: keyof Omit<Inventory, "available" | "total">;
  label: string;
  icon: typeof Server;
  blurb: string;
}[] = [
  { key: "mcp_servers", label: "MCP servers", icon: Server, blurb: "Tool servers the agent can call" },
  { key: "skills", label: "Skills", icon: Sparkles, blurb: "Extra skills merged into the catalog" },
  { key: "subagents", label: "Subagents", icon: Bot, blurb: "Delegate agents the loop can spawn" },
  { key: "rules", label: "Rules", icon: Scale, blurb: "Policy injected into every run" },
  { key: "hooks", label: "Hooks", icon: Webhook, blurb: "Run on agent lifecycle events" },
];

/**
 * Plugins and connectors for this workspace: what is active, what can be
 * installed, and what this machine is connected to.
 *
 * This used to be read-only, on the reasoning that a policy change belongs in
 * the CLI where it is audited. That held while the only way to obtain a plugin
 * was to author, sign and pack a bundle by hand — so in practice every
 * workspace showed "no plugin components are enabled" above a sentence naming a
 * command, and nothing could be done about it from here. With a catalog
 * compiled into the daemon there is something to install, and installing it is
 * the whole point of the panel.
 *
 * Two things are still not offered here, deliberately: pinning a plugin as
 * `required` (an admin act the same user could not then undo), and any claim
 * that a connector works. A connector's health comes only from Test, which
 * launches it for real.
 */
export function PluginsView({ daemonUrl, path, onClose }: PluginsViewProps) {
  const [tab, setTab] = useState<Tab>("active");
  const [inv, setInv] = useState<Inventory | null>(null);
  const [catalog, setCatalog] = useState<CatalogPlugin[] | null>(null);
  const [connectors, setConnectors] = useState<Connector[] | null>(null);
  const [connectorCatalog, setConnectorCatalog] = useState<ConnectorSpec[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  /** Ids of rows with an action in flight, so each button disables on its own. */
  const [busy, setBusy] = useState<ReadonlySet<string>>(new Set());
  const [probes, setProbes] = useState<Record<string, ProbeRecord>>({});
  const [reloadTick, setReloadTick] = useState(0);

  const reload = useCallback(() => setReloadTick((t) => t + 1), []);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const [inventory, plugins, conns] = await Promise.all([
          invoke<Inventory>("list_plugins", { url: daemonUrl, path }),
          invoke<{ plugins: CatalogPlugin[] }>("plugin_catalog", { url: daemonUrl, path }),
          invoke<{ connectors: Connector[]; catalog: ConnectorSpec[] }>("list_connectors", {
            url: daemonUrl,
            path,
          }),
        ]);
        if (!alive) return;
        setInv(inventory);
        setCatalog(plugins.plugins);
        setConnectors(conns.connectors);
        setConnectorCatalog(conns.catalog);
        setError(null);
      } catch (e) {
        if (alive) setError(String(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [daemonUrl, path, reloadTick]);

  /** Run one action, tracking its own busy key and surfacing its own failure. */
  const act = useCallback(
    async (key: string, run: () => Promise<string>) => {
      setBusy((prev) => new Set(prev).add(key));
      setNotice(null);
      try {
        setNotice(await run());
        reload();
      } catch (e) {
        // The daemon's message, not a generic one: it names the actual reason.
        setNotice(`Failed: ${String(e)}`);
      } finally {
        setBusy((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
      }
    },
    [reload],
  );

  const installPlugin = (p: CatalogPlugin) =>
    act(`plugin:${p.name}`, async () => {
      const res = await invoke<{ components: number; signing_key_persisted: boolean }>(
        "install_plugin",
        { url: daemonUrl, path, name: p.name },
      );
      // The key warning is rare and easy to miss; say it in the same breath as
      // the success rather than hiding it in a log.
      return res.signing_key_persisted
        ? `Installed ${p.title} — ${res.components} component${res.components === 1 ? "" : "s"} now active.`
        : `Installed ${p.title}, but the signing key could not be stored, so its publisher fingerprint will differ next time.`;
    });

  const setPolicy = (p: CatalogPlugin, policy: "on" | "off") =>
    act(`plugin:${p.name}`, async () => {
      await invoke("set_plugin_policy", { url: daemonUrl, path, name: p.name, policy });
      return policy === "on" ? `${p.title} enabled.` : `${p.title} disabled — its components no longer load.`;
    });

  const uninstallPlugin = (p: CatalogPlugin) =>
    act(`plugin:${p.name}`, async () => {
      const res = await invoke<{ removed: boolean }>("uninstall_plugin", {
        url: daemonUrl,
        path,
        name: p.name,
      });
      return res.removed
        ? `Removed ${p.title}.`
        : `${p.title} had no files on disk; its policy row was cleared.`;
    });

  const addConnector = (spec: ConnectorSpec, credentials: Record<string, string>) =>
    act(`connector:${spec.id}`, async () => {
      await invoke("add_connector", {
        url: daemonUrl,
        path,
        catalogId: spec.id,
        credentials,
      });
      return `Added ${spec.title}. Use Test to check it actually starts.`;
    });

  const toggleConnector = (c: Connector) =>
    act(`connector:${c.id}`, async () => {
      await invoke("toggle_connector", { url: daemonUrl, path, id: c.id, enabled: !c.enabled });
      return c.enabled ? `${c.title} disabled.` : `${c.title} enabled.`;
    });

  const removeConnector = (c: Connector) =>
    act(`connector:${c.id}`, async () => {
      const res = await invoke<{ secrets_deleted: number }>("remove_connector", {
        url: daemonUrl,
        path,
        id: c.id,
      });
      return `Removed ${c.title}${
        res.secrets_deleted > 0
          ? ` and deleted ${res.secrets_deleted} stored credential${res.secrets_deleted === 1 ? "" : "s"}.`
          : "."
      }`;
    });

  const probeConnector = (c: Connector) =>
    act(`connector:${c.id}`, async () => {
      const res = await invoke<ProbeRecord>("probe_connector", { url: daemonUrl, path, id: c.id });
      setProbes((prev) => ({ ...prev, [c.id]: res }));
      switch (res.result.state) {
        case "ok":
          return `${c.title} started and offered ${res.result.tools.length} tool${
            res.result.tools.length === 1 ? "" : "s"
          }.`;
        case "timedout":
          return `${c.title} started but said nothing within ${res.result.after_secs}s.`;
        case "failed":
          return `${c.title} could not start: ${res.result.error}`;
      }
    });

  const catalogById = useMemo(
    () => new Map((connectorCatalog ?? []).map((c) => [c.id, c])),
    [connectorCatalog],
  );
  const configuredIds = useMemo(
    () => new Set((connectors ?? []).map((c) => c.id)),
    [connectors],
  );

  return (
    <div className="vx-skills">
      <div className="vx-skills__head">
        <Plug size={14} />
        <span>Plugins</span>
        {inv && (
          <span className="vx-skills__count">
            {inv.total} enabled component{inv.total === 1 ? "" : "s"}
          </span>
        )}
        <div className="vx-skills__spacer" />
        <button className="vx-icon-btn" aria-label="Close plugins" onClick={onClose}>
          <X size={14} />
        </button>
      </div>

      <div className="vx-plugins__tabs" role="tablist">
        {([
          ["active", "Active"],
          ["catalog", "Plugins"],
          ["connectors", "Connectors"],
        ] as const).map(([key, label]) => (
          <button
            key={key}
            role="tab"
            aria-selected={tab === key}
            className={`vx-right__tab${tab === key ? " is-active" : ""}`}
            onClick={() => setTab(key)}
          >
            {label}
          </button>
        ))}
      </div>

      {notice && <div className="vx-plugins__notice">{notice}</div>}

      <div className="vx-plugins__body">
        {error && <div className="vx-files__empty">Failed to load plugins: {error}</div>}

        {!error && tab === "active" && (
          <ActiveTab inv={inv} onBrowse={() => setTab("catalog")} />
        )}

        {!error && tab === "catalog" && (
          <PluginCatalogTab
            plugins={catalog}
            busy={busy}
            onInstall={installPlugin}
            onEnable={(p) => setPolicy(p, "on")}
            onDisable={(p) => setPolicy(p, "off")}
            onRemove={uninstallPlugin}
          />
        )}

        {!error && tab === "connectors" && (
          <ConnectorsTab
            connectors={connectors}
            catalog={connectorCatalog}
            catalogById={catalogById}
            configuredIds={configuredIds}
            probes={probes}
            busy={busy}
            onAdd={addConnector}
            onToggle={toggleConnector}
            onRemove={removeConnector}
            onProbe={probeConnector}
          />
        )}
      </div>
    </div>
  );
}

// ── Active ───────────────────────────────────────────────────────────────────

function ActiveTab({ inv, onBrowse }: { inv: Inventory | null; onBrowse: () => void }) {
  if (inv === null) return <div className="vx-files__empty">Loading…</div>;
  if (inv.total === 0) {
    return (
      <div className="vx-files__empty">
        Nothing is extending the agent in this workspace yet.
        <div className="vx-plugins__hint">
          The Plugins tab has skills and rules you can install in one click, and Connectors
          adds MCP servers. Bundles built elsewhere still install with{" "}
          <code>vibecli plugin install</code>.
        </div>
        <button className="vx-plugins__cta" onClick={onBrowse}>
          Browse plugins
        </button>
      </div>
    );
  }
  return (
    <>
      {SECTIONS.map(({ key, label, icon: Icon, blurb }) => {
        const rows = inv[key] ?? [];
        if (rows.length === 0) return null;
        return (
          <section key={key} className="vx-plugins__section">
            <div className="vx-plugins__section-head">
              <Icon size={13} />
              <span className="vx-plugins__section-label">{label}</span>
              <span className="vx-plugins__section-count">{rows.length}</span>
            </div>
            <div className="vx-plugins__blurb">{blurb}</div>
            <ul className="vx-plugins__list">
              {rows.map((c, i) => (
                <li key={`${c.plugin}/${c.name}/${i}`} className="vx-plugins__row">
                  <span className="vx-plugins__name">{c.name}</span>
                  {c.event && <span className="vx-plugins__event">{c.event}</span>}
                  <span className="vx-plugins__from">from {c.plugin}</span>
                  {/* "required" is admin-pinned and can't be turned off here. */}
                  <span className={`vx-plugins__policy vx-plugins__policy--${c.policy}`}>
                    {c.policy}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </>
  );
}

// ── Plugin catalog ───────────────────────────────────────────────────────────

function PluginCatalogTab({
  plugins,
  busy,
  onInstall,
  onEnable,
  onDisable,
  onRemove,
}: {
  plugins: CatalogPlugin[] | null;
  busy: ReadonlySet<string>;
  onInstall: (p: CatalogPlugin) => void;
  onEnable: (p: CatalogPlugin) => void;
  onDisable: (p: CatalogPlugin) => void;
  onRemove: (p: CatalogPlugin) => void;
}) {
  if (plugins === null) return <div className="vx-files__empty">Loading…</div>;
  if (plugins.length === 0) return <div className="vx-files__empty">The catalog is empty.</div>;

  return (
    <div className="vx-plugins__cards">
      {plugins.map((p) => {
        const working = busy.has(`plugin:${p.name}`);
        const pinned = p.policy === "required";
        return (
          <article key={p.name} className="vx-plugins__card">
            <header className="vx-plugins__card-head">
              <span className="vx-plugins__card-title">{p.title}</span>
              <span className="vx-plugins__card-version">v{p.version}</span>
              {p.installed && (
                <span className={`vx-plugins__policy vx-plugins__policy--${p.policy}`}>
                  {p.policy}
                </span>
              )}
            </header>
            <p className="vx-plugins__card-desc">{p.description}</p>
            <div className="vx-plugins__chips">
              {p.components.map((c) => (
                <span key={`${c.kind}/${c.name}`} className="vx-plugins__chip">
                  {c.kind} · {c.name}
                </span>
              ))}
            </div>
            <footer className="vx-plugins__card-actions">
              {!p.installed && (
                <button className="vx-plugins__btn" disabled={working} onClick={() => onInstall(p)}>
                  {working ? <Loader2 size={12} className="vx-spin" /> : <Download size={12} />}
                  Install
                </button>
              )}
              {p.installed && p.policy === "on" && (
                <button className="vx-plugins__btn" disabled={working} onClick={() => onDisable(p)}>
                  <Power size={12} />
                  Disable
                </button>
              )}
              {p.installed && p.policy === "off" && (
                <button className="vx-plugins__btn" disabled={working} onClick={() => onEnable(p)}>
                  <Power size={12} />
                  Enable
                </button>
              )}
              {p.installed && !pinned && (
                <button
                  className="vx-plugins__btn vx-plugins__btn--danger"
                  disabled={working}
                  onClick={() => onRemove(p)}
                >
                  <Trash2 size={12} />
                  Remove
                </button>
              )}
              {pinned && (
                <span className="vx-plugins__note">
                  Pinned by an administrator — change it with <code>vibecli plugin</code>.
                </span>
              )}
            </footer>
          </article>
        );
      })}
    </div>
  );
}

// ── Connectors ───────────────────────────────────────────────────────────────

function ConnectorsTab({
  connectors,
  catalog,
  catalogById,
  configuredIds,
  probes,
  busy,
  onAdd,
  onToggle,
  onRemove,
  onProbe,
}: {
  connectors: Connector[] | null;
  catalog: ConnectorSpec[] | null;
  catalogById: Map<string, ConnectorSpec>;
  configuredIds: ReadonlySet<string>;
  probes: Record<string, ProbeRecord>;
  busy: ReadonlySet<string>;
  onAdd: (spec: ConnectorSpec, credentials: Record<string, string>) => void;
  onToggle: (c: Connector) => void;
  onRemove: (c: Connector) => void;
  onProbe: (c: Connector) => void;
}) {
  if (connectors === null || catalog === null) return <div className="vx-files__empty">Loading…</div>;

  const available = catalog.filter((c) => !configuredIds.has(c.id));

  return (
    <>
      <section className="vx-plugins__section">
        <div className="vx-plugins__section-head">
          <PlugZap size={13} />
          <span className="vx-plugins__section-label">Configured</span>
          <span className="vx-plugins__section-count">{connectors.length}</span>
        </div>
        <div className="vx-plugins__blurb">
          MCP servers this workspace can talk to. Credentials are stored encrypted in the
          workspace, never in a file. Reachable from <code>vibecli</code>&rsquo;s{" "}
          <code>/mcp</code> command — agent runs do not use MCP tools yet.
        </div>
        {connectors.length === 0 && (
          <div className="vx-plugins__note">None yet — add one below.</div>
        )}
        <div className="vx-plugins__cards">
          {connectors.map((c) => (
            <ConnectorRow
              key={c.id}
              connector={c}
              spec={c.catalog_id ? catalogById.get(c.catalog_id) : undefined}
              probe={probes[c.id]}
              busy={busy.has(`connector:${c.id}`)}
              onToggle={onToggle}
              onRemove={onRemove}
              onProbe={onProbe}
            />
          ))}
        </div>
      </section>

      <section className="vx-plugins__section">
        <div className="vx-plugins__section-head">
          <Server size={13} />
          <span className="vx-plugins__section-label">Available</span>
          <span className="vx-plugins__section-count">{available.length}</span>
        </div>
        <div className="vx-plugins__blurb">
          Adding one records the command and any credential. It does not check that the
          server runs — press Test afterwards, which launches it for real.
        </div>
        <div className="vx-plugins__cards">
          {available.map((spec) => (
            <ConnectorOffer
              key={spec.id}
              spec={spec}
              busy={busy.has(`connector:${spec.id}`)}
              onAdd={onAdd}
            />
          ))}
        </div>
      </section>
    </>
  );
}

function ConnectorRow({
  connector,
  spec,
  probe,
  busy,
  onToggle,
  onRemove,
  onProbe,
}: {
  connector: Connector;
  spec?: ConnectorSpec;
  probe?: ProbeRecord;
  busy: boolean;
  onToggle: (c: Connector) => void;
  onRemove: (c: Connector) => void;
  onProbe: (c: Connector) => void;
}) {
  const missing = connector.missing_credentials;
  return (
    <article className="vx-plugins__card">
      <header className="vx-plugins__card-head">
        <span className="vx-plugins__card-title">{connector.title}</span>
        <span className={`vx-plugins__policy vx-plugins__policy--${connector.enabled ? "on" : "off"}`}>
          {connector.enabled ? "enabled" : "disabled"}
        </span>
      </header>
      <code className="vx-plugins__cmd">
        {connector.command} {connector.args.join(" ")}
      </code>

      {missing.length > 0 && (
        <div className="vx-plugins__warn">
          <AlertTriangle size={12} />
          Missing credential{missing.length === 1 ? "" : "s"}: {missing.join(", ")} — remove and
          add it again to supply {missing.length === 1 ? "it" : "them"}.
        </div>
      )}

      {spec && !spec.runtime_available && (
        <div className="vx-plugins__warn">
          <AlertTriangle size={12} />
          <code>{spec.runtime_program}</code> is not on this machine&rsquo;s PATH, so this
          connector cannot start until it is installed.
        </div>
      )}

      <ProbeLine probe={probe} />

      <footer className="vx-plugins__card-actions">
        <button className="vx-plugins__btn" disabled={busy} onClick={() => onProbe(connector)}>
          {busy ? <Loader2 size={12} className="vx-spin" /> : <PlugZap size={12} />}
          Test
        </button>
        <button className="vx-plugins__btn" disabled={busy} onClick={() => onToggle(connector)}>
          <Power size={12} />
          {connector.enabled ? "Disable" : "Enable"}
        </button>
        <button
          className="vx-plugins__btn vx-plugins__btn--danger"
          disabled={busy}
          onClick={() => onRemove(connector)}
        >
          <Trash2 size={12} />
          Remove
        </button>
      </footer>
    </article>
  );
}

/** The last measured outcome, or an explicit "not checked" — never a guess
 *  inferred from having a credential. */
function ProbeLine({ probe }: { probe?: ProbeRecord }) {
  if (!probe) {
    return <div className="vx-plugins__note">Not checked yet.</div>;
  }
  const when = new Date(probe.checked_at).toLocaleTimeString();
  switch (probe.result.state) {
    case "ok":
      return (
        <div className="vx-plugins__ok">
          <Check size={12} />
          Started at {when}: {probe.result.tools.length} tool
          {probe.result.tools.length === 1 ? "" : "s"}
          {probe.result.tools.length > 0 && ` — ${probe.result.tools.slice(0, 6).join(", ")}`}
          {probe.result.tools.length > 6 && "…"}
        </div>
      );
    case "timedout":
      return (
        <div className="vx-plugins__warn">
          <AlertTriangle size={12} />
          Started but silent for {probe.result.after_secs}s (checked {when}).
        </div>
      );
    case "failed":
      return (
        <div className="vx-plugins__warn">
          <AlertTriangle size={12} />
          Failed at {when}: {probe.result.error}
        </div>
      );
  }
}

function ConnectorOffer({
  spec,
  busy,
  onAdd,
}: {
  spec: ConnectorSpec;
  busy: boolean;
  onAdd: (spec: ConnectorSpec, credentials: Record<string, string>) => void;
}) {
  const [values, setValues] = useState<Record<string, string>>({});
  const incomplete = spec.credentials.some((f) => !(values[f.env] ?? "").trim());

  return (
    <article className="vx-plugins__card">
      <header className="vx-plugins__card-head">
        <span className="vx-plugins__card-title">{spec.title}</span>
        <span className="vx-plugins__card-version">{spec.runtime}</span>
      </header>
      <p className="vx-plugins__card-desc">{spec.description}</p>
      <code className="vx-plugins__cmd">
        {spec.command} {spec.args.join(" ")}
      </code>

      {!spec.runtime_available && (
        <div className="vx-plugins__warn">
          <AlertTriangle size={12} />
          <code>{spec.runtime_program}</code> is not on PATH. You can still add this, but it
          will not start until that is installed.
        </div>
      )}

      {spec.credentials.map((field) => (
        <label key={field.env} className="vx-plugins__field">
          <span className="vx-plugins__field-label">{field.label}</span>
          <input
            type="password"
            className="vx-plugins__input"
            autoComplete="off"
            value={values[field.env] ?? ""}
            onChange={(e) => setValues((v) => ({ ...v, [field.env]: e.target.value }))}
            placeholder={field.env}
          />
          <span className="vx-plugins__field-help">{field.help}</span>
        </label>
      ))}

      <footer className="vx-plugins__card-actions">
        <button
          className="vx-plugins__btn"
          disabled={busy || incomplete}
          onClick={() => onAdd(spec, values)}
        >
          {busy ? <Loader2 size={12} className="vx-spin" /> : <Download size={12} />}
          Add
        </button>
        <a
          className="vx-plugins__btn vx-plugins__btn--link"
          href={spec.docs_url}
          target="_blank"
          rel="noreferrer"
        >
          <ExternalLink size={12} />
          Docs
        </a>
      </footer>
    </article>
  );
}
