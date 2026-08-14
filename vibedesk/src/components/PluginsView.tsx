import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Search, SlidersHorizontal, Check, Loader2, AlertTriangle } from "lucide-react";

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
  category: string;
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

/** A probe's outcome and when it was taken. A result with no time is a claim
 *  about now made from a measurement of then. */
interface ProbeRecord {
  result: ProbeResult;
  checked_at: number;
}

/**
 * One marketplace row. Plugins and connectors sit in the same list because
 * that is how they are shopped for — the difference between "a rule the agent
 * reads" and "a server the agent calls" matters when you manage it, not when
 * you are looking for Postgres.
 */
type MarketItem =
  | { kind: "plugin"; key: string; title: string; description: string; category: string; plugin: CatalogPlugin }
  | { kind: "connector"; key: string; title: string; description: string; category: string; spec: ConnectorSpec };

/** Each panel loads on its own, so one failure does not blank the others. */
type Loaded<T> = { state: "loading" } | { state: "ready"; value: T } | { state: "error"; message: string };

/** Rows shown per category before "Show N more". */
const COLLAPSED_ROWS = 4;

const SECTION_ORDER = [
  "VibeCody",
  "Engineering Practice",
  "Security",
  "Performance",
  "Operations",
  "Design",
  "Files And Code",
  "Web And Search",
  "Databases",
  "Agent Memory",
  "Inbox And Collaboration",
  "Observability",
  "Utilities",
];

/**
 * Plugins: a marketplace of everything that can extend the agent here, and a
 * manage view for what already does.
 *
 * The panel used to be a single sentence — "No plugin components are enabled
 * for this workspace" — above a pointer to a CLI command, because the only way
 * to obtain a plugin was to author, sign and pack a bundle by hand. There is a
 * catalog now, so this is a shop.
 *
 * Two claims it will not make. A connector is never described as working until
 * Test has actually launched it: there is no state here inferred from a
 * credential being present. And a plugin cannot be pinned `required` from here,
 * because that is an admin act the same user could not then undo.
 */
export function PluginsView({ daemonUrl, path, onClose }: PluginsViewProps) {
  const [tab, setTab] = useState<"marketplace" | "yours">("marketplace");
  const [query, setQuery] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [filterOpen, setFilterOpen] = useState(false);
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [openForm, setOpenForm] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<Record<string, Record<string, string>>>({});

  const [inventory, setInventory] = useState<Loaded<Inventory>>({ state: "loading" });
  const [catalog, setCatalog] = useState<Loaded<CatalogPlugin[]>>({ state: "loading" });
  const [connectors, setConnectors] = useState<
    Loaded<{ connectors: Connector[]; catalog: ConnectorSpec[] }>
  >({ state: "loading" });

  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<ReadonlySet<string>>(new Set());
  const [probes, setProbes] = useState<Record<string, ProbeRecord>>({});
  const [tick, setTick] = useState(0);
  const reload = useCallback(() => setTick((t) => t + 1), []);

  // Three independent loads. Awaiting them together meant a single stale route
  // — a daemon older than the app, 404 on the catalog — blanked the whole
  // panel, including the inventory that had answered fine.
  useEffect(() => {
    let alive = true;
    const run = async <T,>(
      call: () => Promise<T>,
      set: (v: Loaded<T>) => void,
    ) => {
      try {
        const value = await call();
        if (alive) set({ state: "ready", value });
      } catch (e) {
        if (alive) set({ state: "error", message: String(e) });
      }
    };
    void run(() => invoke<Inventory>("list_plugins", { url: daemonUrl, path }), setInventory);
    void run(
      async () => (await invoke<{ plugins: CatalogPlugin[] }>("plugin_catalog", { url: daemonUrl, path })).plugins,
      setCatalog,
    );
    void run(
      () =>
        invoke<{ connectors: Connector[]; catalog: ConnectorSpec[] }>("list_connectors", {
          url: daemonUrl,
          path,
        }),
      setConnectors,
    );
    return () => {
      alive = false;
    };
  }, [daemonUrl, path, tick]);

  const act = useCallback(
    async (key: string, run: () => Promise<string>) => {
      setBusy((prev) => new Set(prev).add(key));
      setNotice(null);
      try {
        setNotice(await run());
        reload();
      } catch (e) {
        // The daemon's own sentence names the reason — a missing credential, a
        // duplicate id, an older binary. A status code would not.
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
      return res.signing_key_persisted
        ? `Added ${p.title} — ${res.components} component${res.components === 1 ? "" : "s"} now active.`
        : `Added ${p.title}, but the signing key could not be stored, so its publisher fingerprint will differ next time.`;
    });

  const setPolicy = (p: CatalogPlugin, policy: "on" | "off") =>
    act(`plugin:${p.name}`, async () => {
      await invoke("set_plugin_policy", { url: daemonUrl, path, name: p.name, policy });
      return policy === "on" ? `${p.title} enabled.` : `${p.title} disabled — its components no longer load.`;
    });

  const removePlugin = (p: CatalogPlugin) =>
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

  const addConnector = (spec: ConnectorSpec) =>
    act(`connector:${spec.id}`, async () => {
      await invoke("add_connector", {
        url: daemonUrl,
        path,
        catalogId: spec.id,
        credentials: drafts[spec.id] ?? {},
      });
      setDrafts((d) => ({ ...d, [spec.id]: {} }));
      setOpenForm(null);
      return `Added ${spec.title}. Use Test in Yours to check it actually starts.`;
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
      return res.secrets_deleted > 0
        ? `Removed ${c.title} and deleted ${res.secrets_deleted} stored credential${res.secrets_deleted === 1 ? "" : "s"}.`
        : `Removed ${c.title}.`;
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

  // ── Marketplace model ─────────────────────────────────────────────────────

  const items = useMemo<MarketItem[]>(() => {
    const plugins: MarketItem[] =
      catalog.state === "ready"
        ? catalog.value.map((p) => ({
            kind: "plugin" as const,
            key: `plugin:${p.name}`,
            title: p.title,
            description: p.description,
            category: p.category,
            plugin: p,
          }))
        : [];
    const specs: MarketItem[] =
      connectors.state === "ready"
        ? connectors.value.catalog.map((s) => ({
            kind: "connector" as const,
            key: `connector:${s.id}`,
            title: s.title,
            description: s.description,
            category: s.category,
            spec: s,
          }))
        : [];
    return [...plugins, ...specs];
  }, [catalog, connectors]);

  const configuredIds = useMemo(
    () => new Set(connectors.state === "ready" ? connectors.value.connectors.map((c) => c.id) : []),
    [connectors],
  );

  const categories = useMemo(() => {
    const present = new Set(items.map((i) => i.category));
    const ordered = SECTION_ORDER.filter((c) => present.has(c));
    // Anything the daemon files under a section this build has not heard of
    // still gets shown, alphabetically after the known ones — a newer daemon
    // must not be able to hide entries from an older panel.
    const extra = [...present].filter((c) => !SECTION_ORDER.includes(c)).sort();
    return [...ordered, ...extra];
  }, [items]);

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return items.filter((i) => {
      if (categoryFilter && i.category !== categoryFilter) return false;
      if (!q) return true;
      return (
        i.title.toLowerCase().includes(q) ||
        i.description.toLowerCase().includes(q) ||
        i.category.toLowerCase().includes(q)
      );
    });
  }, [items, query, categoryFilter]);

  const loading = catalog.state === "loading" || connectors.state === "loading";
  const marketErrors = [
    catalog.state === "error" ? catalog.message : null,
    connectors.state === "error" ? connectors.message : null,
  ].filter((m): m is string => m !== null);

  return (
    <div className="vx-market">
      <div className="vx-market__head">
        <span className="vx-market__title">Plugins</span>
        <button className="vx-icon-btn" aria-label="Close plugins" onClick={onClose}>
          <X size={16} />
        </button>
      </div>

      <div className="vx-market__toolbar">
        <div className="vx-market__tabs" role="tablist">
          {(
            [
              ["marketplace", "Marketplace"],
              ["yours", "Yours"],
            ] as const
          ).map(([key, label]) => (
            <button
              key={key}
              role="tab"
              aria-selected={tab === key}
              className={`vx-market__tab${tab === key ? " is-active" : ""}`}
              onClick={() => setTab(key)}
            >
              {label}
            </button>
          ))}
        </div>

        <div className="vx-market__toolbar-right">
          <div className="vx-market__filter-wrap">
            <button
              className={`vx-market__filter${categoryFilter ? " is-active" : ""}`}
              aria-label="Filter by category"
              aria-expanded={filterOpen}
              onClick={() => setFilterOpen((o) => !o)}
            >
              <SlidersHorizontal size={16} />
            </button>
            {filterOpen && (
              <div className="vx-market__menu" role="menu">
                <button
                  className={`vx-market__menu-item${categoryFilter === null ? " is-active" : ""}`}
                  onClick={() => {
                    setCategoryFilter(null);
                    setFilterOpen(false);
                  }}
                >
                  All categories
                </button>
                {categories.map((c) => (
                  <button
                    key={c}
                    className={`vx-market__menu-item${categoryFilter === c ? " is-active" : ""}`}
                    onClick={() => {
                      setCategoryFilter(c);
                      setFilterOpen(false);
                    }}
                  >
                    {c}
                  </button>
                ))}
              </div>
            )}
          </div>
          <div className="vx-market__search">
            <Search size={15} />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search plugins"
              aria-label="Search plugins"
            />
          </div>
        </div>
      </div>

      {notice && <div className="vx-market__notice">{notice}</div>}

      <div className="vx-market__body">
        {tab === "marketplace" && (
          <>
            {marketErrors.map((m) => (
              <div key={m} className="vx-market__error">
                <AlertTriangle size={14} />
                <span>{m}</span>
              </div>
            ))}
            {loading && marketErrors.length === 0 && <div className="vx-market__empty">Loading…</div>}
            {!loading && visible.length === 0 && marketErrors.length === 0 && (
              <div className="vx-market__empty">
                {query || categoryFilter ? "Nothing matches that." : "The catalog is empty."}
              </div>
            )}

            {categories.map((category) => {
              const rows = visible.filter((i) => i.category === category);
              if (rows.length === 0) return null;
              const isOpen = expanded.has(category) || Boolean(query);
              const shown = isOpen ? rows : rows.slice(0, COLLAPSED_ROWS);
              const hidden = rows.length - shown.length;
              return (
                <section key={category} className="vx-market__section">
                  <div className="vx-market__section-label">{category}</div>
                  <div className="vx-market__grid">
                    {shown.map((item) =>
                      item.kind === "plugin" ? (
                        <PluginRow
                          key={item.key}
                          item={item.plugin}
                          busy={busy.has(`plugin:${item.plugin.name}`)}
                          onAdd={() => installPlugin(item.plugin)}
                        />
                      ) : (
                        <ConnectorRow
                          key={item.key}
                          spec={item.spec}
                          added={configuredIds.has(item.spec.id)}
                          busy={busy.has(`connector:${item.spec.id}`)}
                          formOpen={openForm === item.spec.id}
                          draft={drafts[item.spec.id] ?? {}}
                          onOpenForm={() => setOpenForm(openForm === item.spec.id ? null : item.spec.id)}
                          onDraft={(env, value) =>
                            setDrafts((d) => ({
                              ...d,
                              [item.spec.id]: { ...(d[item.spec.id] ?? {}), [env]: value },
                            }))
                          }
                          onAdd={() => addConnector(item.spec)}
                        />
                      ),
                    )}
                  </div>
                  {hidden > 0 && (
                    <button
                      className="vx-market__more"
                      onClick={() => setExpanded((prev) => new Set(prev).add(category))}
                    >
                      Show {hidden} more
                    </button>
                  )}
                </section>
              );
            })}
          </>
        )}

        {tab === "yours" && (
          <YoursTab
            inventory={inventory}
            catalog={catalog}
            connectors={connectors}
            probes={probes}
            busy={busy}
            onBrowse={() => setTab("marketplace")}
            onEnable={(p) => setPolicy(p, "on")}
            onDisable={(p) => setPolicy(p, "off")}
            onRemovePlugin={removePlugin}
            onToggleConnector={toggleConnector}
            onRemoveConnector={removeConnector}
            onProbeConnector={probeConnector}
          />
        )}
      </div>
    </div>
  );
}

// ── Rows ─────────────────────────────────────────────────────────────────────

/**
 * A monogram tile, not a vendor logo.
 *
 * Shipping Intercom's or AWS's mark would put someone else's trademark in our
 * binary, and fetching it at runtime would make an offline panel depend on the
 * network to render. The letter and a hue derived from the id give each row a
 * stable, recognisable shape without either problem.
 */
function Icon({ id, label }: { id: string; label: string }) {
  const hue = useMemo(() => {
    // Small stable hash — same id, same colour, every launch.
    let h = 0;
    for (let i = 0; i < id.length; i += 1) h = (h * 31 + id.charCodeAt(i)) % 360;
    return h;
  }, [id]);
  return (
    <span
      className="vx-market__icon"
      aria-hidden="true"
      style={{
        background: `hsl(${hue} 45% 22%)`,
        color: `hsl(${hue} 70% 78%)`,
        borderColor: `hsl(${hue} 40% 32%)`,
      }}
    >
      {label.slice(0, 1).toUpperCase()}
    </span>
  );
}

function PluginRow({
  item,
  busy,
  onAdd,
}: {
  item: CatalogPlugin;
  busy: boolean;
  onAdd: () => void;
}) {
  return (
    <div className="vx-market__row">
      <Icon id={item.name} label={item.title} />
      <div className="vx-market__row-text">
        <div className="vx-market__row-title">{item.title}</div>
        <div className="vx-market__row-desc">{item.description}</div>
      </div>
      {item.installed ? (
        <span className="vx-market__added">
          <Check size={14} /> Added
        </span>
      ) : (
        <button className="vx-market__add" disabled={busy} onClick={onAdd}>
          {busy ? <Loader2 size={13} className="vx-spin" /> : null}
          Add
        </button>
      )}
    </div>
  );
}

function ConnectorRow({
  spec,
  added,
  busy,
  formOpen,
  draft,
  onOpenForm,
  onDraft,
  onAdd,
}: {
  spec: ConnectorSpec;
  added: boolean;
  busy: boolean;
  formOpen: boolean;
  draft: Record<string, string>;
  onOpenForm: () => void;
  onDraft: (env: string, value: string) => void;
  onAdd: () => void;
}) {
  const incomplete = spec.credentials.some((f) => !(draft[f.env] ?? "").trim());
  return (
    <div className={`vx-market__row${formOpen ? " is-expanded" : ""}`}>
      <Icon id={spec.id} label={spec.title} />
      <div className="vx-market__row-text">
        <div className="vx-market__row-title">{spec.title}</div>
        <div className="vx-market__row-desc">{spec.description}</div>

        {formOpen && (
          <div className="vx-market__form">
            <code className="vx-market__cmd">
              {spec.command} {spec.args.join(" ")}
            </code>
            {!spec.runtime_available && (
              <div className="vx-market__warn">
                <AlertTriangle size={12} />
                <span>
                  <code>{spec.runtime_program}</code> is not on this machine&rsquo;s PATH. You can
                  add this now, but it will not start until that is installed.
                </span>
              </div>
            )}
            {spec.credentials.map((field) => (
              <label key={field.env} className="vx-market__field">
                <span className="vx-market__field-label">{field.label}</span>
                <input
                  type="password"
                  autoComplete="off"
                  placeholder={field.env}
                  value={draft[field.env] ?? ""}
                  onChange={(e) => onDraft(field.env, e.target.value)}
                />
                <span className="vx-market__field-help">{field.help}</span>
              </label>
            ))}
            <div className="vx-market__form-actions">
              <button className="vx-market__add" disabled={busy || incomplete} onClick={onAdd}>
                {busy ? <Loader2 size={13} className="vx-spin" /> : null}
                {spec.credentials.length > 0 ? "Save and add" : "Add"}
              </button>
              <a className="vx-market__docs" href={spec.docs_url} target="_blank" rel="noreferrer">
                Docs
              </a>
            </div>
          </div>
        )}
      </div>

      {added ? (
        <span className="vx-market__added">
          <Check size={14} /> Added
        </span>
      ) : spec.credentials.length === 0 && !formOpen ? (
        <button className="vx-market__add" disabled={busy} onClick={onAdd}>
          {busy ? <Loader2 size={13} className="vx-spin" /> : null}
          Add
        </button>
      ) : (
        <button className="vx-market__add" onClick={onOpenForm}>
          {formOpen ? "Cancel" : "Add"}
        </button>
      )}
    </div>
  );
}

// ── Yours ────────────────────────────────────────────────────────────────────

function YoursTab({
  inventory,
  catalog,
  connectors,
  probes,
  busy,
  onBrowse,
  onEnable,
  onDisable,
  onRemovePlugin,
  onToggleConnector,
  onRemoveConnector,
  onProbeConnector,
}: {
  inventory: Loaded<Inventory>;
  catalog: Loaded<CatalogPlugin[]>;
  connectors: Loaded<{ connectors: Connector[]; catalog: ConnectorSpec[] }>;
  probes: Record<string, ProbeRecord>;
  busy: ReadonlySet<string>;
  onBrowse: () => void;
  onEnable: (p: CatalogPlugin) => void;
  onDisable: (p: CatalogPlugin) => void;
  onRemovePlugin: (p: CatalogPlugin) => void;
  onToggleConnector: (c: Connector) => void;
  onRemoveConnector: (c: Connector) => void;
  onProbeConnector: (c: Connector) => void;
}) {
  const installed = catalog.state === "ready" ? catalog.value.filter((p) => p.installed) : [];
  const mine = connectors.state === "ready" ? connectors.value.connectors : [];
  const specs = connectors.state === "ready" ? connectors.value.catalog : [];
  const total = inventory.state === "ready" ? inventory.value.total : null;

  if (installed.length === 0 && mine.length === 0 && catalog.state !== "loading") {
    return (
      <div className="vx-market__empty">
        Nothing is extending the agent in this workspace yet.
        <div className="vx-market__empty-sub">
          The Marketplace has skills and rules that install in one click, and MCP servers you
          can connect. Bundles built elsewhere still install with{" "}
          <code>vibecli plugin install</code>.
        </div>
        <button className="vx-market__cta" onClick={onBrowse}>
          Browse the marketplace
        </button>
      </div>
    );
  }

  return (
    <>
      {installed.length > 0 && (
        <section className="vx-market__section">
          <div className="vx-market__section-label">
            Plugins
            {total !== null && (
              <span className="vx-market__section-count">
                {total} live component{total === 1 ? "" : "s"}
              </span>
            )}
          </div>
          <div className="vx-market__list">
            {installed.map((p) => {
              const working = busy.has(`plugin:${p.name}`);
              return (
                <div key={p.name} className="vx-market__manage">
                  <Icon id={p.name} label={p.title} />
                  <div className="vx-market__row-text">
                    <div className="vx-market__row-title">
                      {p.title}
                      <span className={`vx-market__pill vx-market__pill--${p.policy}`}>{p.policy}</span>
                    </div>
                    <div className="vx-market__row-desc">
                      {p.components.map((c) => `${c.kind} · ${c.name}`).join("   ")}
                    </div>
                  </div>
                  <div className="vx-market__actions">
                    {p.policy === "required" ? (
                      <span className="vx-market__note">
                        Pinned by an administrator — change it with <code>vibecli plugin</code>.
                      </span>
                    ) : (
                      <>
                        <button
                          className="vx-market__btn"
                          disabled={working}
                          onClick={() => (p.policy === "on" ? onDisable(p) : onEnable(p))}
                        >
                          {p.policy === "on" ? "Disable" : "Enable"}
                        </button>
                        <button
                          className="vx-market__btn vx-market__btn--danger"
                          disabled={working}
                          onClick={() => onRemovePlugin(p)}
                        >
                          Remove
                        </button>
                      </>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </section>
      )}

      {mine.length > 0 && (
        <section className="vx-market__section">
          <div className="vx-market__section-label">
            Connectors
            <span className="vx-market__section-count">
              reachable from <code>vibecli</code>&rsquo;s <code>/mcp</code>; agent runs do not use
              MCP tools yet
            </span>
          </div>
          <div className="vx-market__list">
            {mine.map((c) => {
              const working = busy.has(`connector:${c.id}`);
              const spec = c.catalog_id ? specs.find((s) => s.id === c.catalog_id) : undefined;
              return (
                <div key={c.id} className="vx-market__manage">
                  <Icon id={c.id} label={c.title} />
                  <div className="vx-market__row-text">
                    <div className="vx-market__row-title">
                      {c.title}
                      <span className={`vx-market__pill vx-market__pill--${c.enabled ? "on" : "off"}`}>
                        {c.enabled ? "enabled" : "disabled"}
                      </span>
                    </div>
                    <code className="vx-market__cmd">
                      {c.command} {c.args.join(" ")}
                    </code>
                    {c.missing_credentials.length > 0 && (
                      <div className="vx-market__warn">
                        <AlertTriangle size={12} />
                        <span>
                          Missing {c.missing_credentials.join(", ")} — remove and add it again to
                          supply {c.missing_credentials.length === 1 ? "it" : "them"}.
                        </span>
                      </div>
                    )}
                    {spec && !spec.runtime_available && (
                      <div className="vx-market__warn">
                        <AlertTriangle size={12} />
                        <span>
                          <code>{spec.runtime_program}</code> is not on PATH, so this cannot start.
                        </span>
                      </div>
                    )}
                    <ProbeLine record={probes[c.id]} />
                  </div>
                  <div className="vx-market__actions">
                    <button
                      className="vx-market__btn"
                      disabled={working}
                      onClick={() => onProbeConnector(c)}
                    >
                      {working ? <Loader2 size={13} className="vx-spin" /> : null}
                      Test
                    </button>
                    <button
                      className="vx-market__btn"
                      disabled={working}
                      onClick={() => onToggleConnector(c)}
                    >
                      {c.enabled ? "Disable" : "Enable"}
                    </button>
                    <button
                      className="vx-market__btn vx-market__btn--danger"
                      disabled={working}
                      onClick={() => onRemoveConnector(c)}
                    >
                      Remove
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        </section>
      )}

      {inventory.state === "error" && (
        <div className="vx-market__error">
          <AlertTriangle size={14} />
          <span>Could not read what is live: {inventory.message}</span>
        </div>
      )}
    </>
  );
}

/** The last measured outcome, or an explicit "not checked". Never a guess. */
function ProbeLine({ record }: { record?: ProbeRecord }) {
  if (!record) return <div className="vx-market__note">Not tested yet.</div>;
  const when = new Date(record.checked_at).toLocaleTimeString();
  switch (record.result.state) {
    case "ok":
      return (
        <div className="vx-market__ok">
          <Check size={12} />
          <span>
            Started at {when}: {record.result.tools.length} tool
            {record.result.tools.length === 1 ? "" : "s"}
            {record.result.tools.length > 0 && ` — ${record.result.tools.slice(0, 6).join(", ")}`}
            {record.result.tools.length > 6 && "…"}
          </span>
        </div>
      );
    case "timedout":
      return (
        <div className="vx-market__warn">
          <AlertTriangle size={12} />
          <span>
            Started but silent for {record.result.after_secs}s (checked {when}).
          </span>
        </div>
      );
    case "failed":
      return (
        <div className="vx-market__warn">
          <AlertTriangle size={12} />
          <span>
            Failed at {when}: {record.result.error}
          </span>
        </div>
      );
  }
}
