import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Plug, Server, Sparkles, Bot, Scale, Webhook } from "lucide-react";

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

const SECTIONS: { key: keyof Omit<Inventory, "available" | "total">; label: string; icon: typeof Server; blurb: string }[] = [
  { key: "mcp_servers", label: "MCP servers", icon: Server, blurb: "Tool servers the agent can call" },
  { key: "skills", label: "Skills", icon: Sparkles, blurb: "Extra skills merged into the catalog" },
  { key: "subagents", label: "Subagents", icon: Bot, blurb: "Delegate agents the loop can spawn" },
  { key: "rules", label: "Rules", icon: Scale, blurb: "Policy injected into every run" },
  { key: "hooks", label: "Hooks", icon: Webhook, blurb: "Run on agent lifecycle events" },
];

/**
 * Read-only view of the plugin components active in this workspace.
 *
 * The daemon has always resolved this inventory — it decides which MCP servers,
 * skills, subagents, rules and hooks a run actually gets — but nothing surfaced
 * it, so "Plugins" was a disabled button and the honest answer to "what is
 * modifying my agent?" was "read the CLI output".
 *
 * Deliberately read-only: enabling or disabling a plugin is a policy change
 * that goes through the CLI and the governance surface, where it's audited.
 * Showing state here without the ability to change it is the correct split.
 */
export function PluginsView({ daemonUrl, path, onClose }: PluginsViewProps) {
  const [inv, setInv] = useState<Inventory | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const res = await invoke<Inventory>("list_plugins", { url: daemonUrl, path });
        if (alive) setInv(res);
      } catch (e) {
        if (alive) setError(String(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [daemonUrl, path]);

  return (
    <div className="vx-skills">
      <div className="vx-skills__head">
        <Plug size={14} />
        <span>Plugins</span>
        {inv && <span className="vx-skills__count">{inv.total} enabled component{inv.total === 1 ? "" : "s"}</span>}
        <div className="vx-skills__spacer" />
        <button className="vx-icon-btn" aria-label="Close plugins" onClick={onClose}>
          <X size={14} />
        </button>
      </div>

      <div className="vx-plugins__body">
        {error && <div className="vx-files__empty">Failed to load plugins: {error}</div>}
        {!error && inv === null && <div className="vx-files__empty">Loading…</div>}
        {!error && inv && inv.total === 0 && (
          <div className="vx-files__empty">
            No plugin components are enabled for this workspace.
            <div className="vx-plugins__hint">
              Plugins are installed and enabled with the <code>vibecli plugin</code> commands; this
              view reflects the resulting policy.
            </div>
          </div>
        )}
        {!error &&
          inv &&
          inv.total > 0 &&
          SECTIONS.map(({ key, label, icon: Icon, blurb }) => {
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
      </div>
    </div>
  );
}
