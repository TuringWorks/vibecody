import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

// Plugin Governance panel.
//
// Design invariants:
//   - No "for-you" surface, no usage telemetry, no recommendations —
//     this panel shows the installed plugins for THIS workspace and
//     nothing else.
//   - Every policy change is a deliberate admin click; the `As admin`
//     toggle is the only privilege gate, mirroring the server-side
//     `PolicySetter::Admin` semantics.
//   - Each row shows the publisher key fingerprint (truncated
//     x-coordinate) so trust is anchored to a key the user can see
//     and recognise on re-install.

interface InstalledPlugin {
  name: string;
  version: string;
  publisher: {
    name: string;
    url: string | null;
    key_fingerprint: string;
  };
  description: string;
  install_dir: string;
  components: {
    mcp_servers: number;
    skills: number;
    subagents: number;
    rules: number;
    hooks: number;
  };
  policy: "off" | "on" | "required";
  signature: {
    kid: string;
    algorithm: string;
    manifest_digest: string;
  };
}

/** A core plugin compiled into the binary, with its state in this workspace. */
interface CatalogPlugin {
  name: string;
  title: string;
  version: string;
  description: string;
  category: string;
  components: { kind: string; name: string }[];
  /** Other catalog plugins a bundle installs with it. */
  includes: string[];
  /** Connector ids this setup expects. */
  connectors: string[];
  installed: boolean;
  policy: Policy | null;
}

/** What became of one connector a bundle expects. */
interface ConnectorOutcome {
  id: string;
  title: string;
  state: "already_configured" | "added" | "needs_credentials" | "unknown" | "failed";
  fields?: string[];
  error?: string;
}

type Policy = "off" | "on" | "required";

const POLICY_COLORS: Record<Policy, { bg: string; fg: string }> = {
  off: { bg: "var(--text-muted)22", fg: "var(--text-muted)" },
  on: { bg: "var(--success-color)22", fg: "var(--success-color)" },
  required: { bg: "var(--warning-color)22", fg: "var(--warning-color)" },
};

interface Props {
  // Nullable to match the composite wrapper's contract — when no
  // workspace is open we render an empty state instead of crashing.
  workspacePath?: string | null;
}

export function PluginGovernancePanel({ workspacePath }: Props) {
  const [plugins, setPlugins] = useState<InstalledPlugin[]>([]);
  const [catalog, setCatalog] = useState<CatalogPlugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [busyPlugin, setBusyPlugin] = useState<string | null>(null);
  const [installPath, setInstallPath] = useState("");
  const [installForce, setInstallForce] = useState(false);
  const [installMsg, setInstallMsg] = useState<string | null>(null);
  const [installMode, setInstallMode] = useState<"file" | "url">("file");
  const [installUrl, setInstallUrl] = useState("");

  const load = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    setError(null);
    try {
      const [list, core] = await Promise.all([
        invoke<InstalledPlugin[]>("plugin_list_installed", { workspacePath }),
        invoke<CatalogPlugin[]>("plugin_catalog_list", { workspacePath }),
      ]);
      setPlugins(Array.isArray(list) ? list : []);
      setCatalog(Array.isArray(core) ? core : []);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [workspacePath]);

  useEffect(() => {
    load();
  }, [load]);

  async function changePolicy(name: string, policy: Policy) {
    setBusyPlugin(name);
    try {
      await invoke("plugin_set_policy", {
        workspacePath,
        name,
        policy,
        isAdmin,
      });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusyPlugin(null);
    }
  }

  async function uninstall(name: string) {
    if (!confirm(`Uninstall plugin "${name}"?`)) return;
    setBusyPlugin(name);
    try {
      await invoke<boolean>("plugin_uninstall", {
        workspacePath,
        name,
        isAdmin,
      });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusyPlugin(null);
    }
  }

  async function pickBundle() {
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "MCPB bundle", extensions: ["mcpb"] }],
      });
      if (typeof picked === "string") setInstallPath(picked);
    } catch (e) {
      setError(String(e));
    }
  }

  async function installFromFile() {
    if (!installPath.trim()) return;
    setInstallMsg(null);
    try {
      const installed = await invoke<InstalledPlugin>("plugin_install_from_file", {
        workspacePath,
        bundlePath: installPath.trim(),
        force: installForce,
      });
      setInstallMsg(
        `Installed ${installed.name} v${installed.version} from ${installed.publisher.name} ` +
          `(key ${installed.publisher.key_fingerprint}…) — policy: ${installed.policy}`,
      );
      setInstallPath("");
      setInstallForce(false);
      await load();
    } catch (e) {
      setInstallMsg(`Error: ${e}`);
    }
  }

  async function installFromCatalog(entry: CatalogPlugin) {
    setBusyPlugin(entry.name);
    setInstallMsg(null);
    try {
      const installed = await invoke<
        InstalledPlugin & {
          signing_key_persisted: boolean;
          included: string[];
          connectors: ConnectorOutcome[];
        }
      >("plugin_install_from_catalog", { workspacePath, name: entry.name, force: false });

      // A bundle brings its members and wires up the connectors it can. The
      // ones needing a credential are named rather than counted as done —
      // this panel has no field to enter one, so the user is told where to.
      const wired = (installed.connectors ?? []).filter(
        (c) => c.state === "added" || c.state === "already_configured",
      );
      const pending = (installed.connectors ?? []).filter((c) => c.state === "needs_credentials");
      const brought = [
        (installed.included ?? []).length > 0
          ? `${installed.included.length} plugin${installed.included.length === 1 ? "" : "s"}`
          : null,
        wired.length > 0 ? `${wired.length} connector${wired.length === 1 ? "" : "s"}` : null,
      ]
        .filter((x): x is string => x !== null)
        .join(", ");

      setInstallMsg(
        `Installed ${installed.name} v${installed.version} — policy: ${installed.policy}` +
          (brought ? ` — brought ${brought}` : "") +
          (pending.length > 0
            ? `. Still needs a credential: ${pending
                .map((c) => c.title)
                .join(", ")} — add ${pending.length === 1 ? "it" : "them"} in Connectors.`
            : "") +
          // Rare, and worth saying in the same breath: the fingerprint shown on
          // the row below will differ next install if the key did not persist.
          (installed.signing_key_persisted
            ? ` (key ${installed.publisher.key_fingerprint}…)`
            : " — the signing key could not be stored, so its fingerprint will change next install"),
      );
      await load();
    } catch (e) {
      setInstallMsg(`Error: ${e}`);
    } finally {
      setBusyPlugin(null);
    }
  }

  async function installFromUrl() {
    if (!installUrl.trim()) return;
    setInstallMsg(null);
    try {
      const installed = await invoke<InstalledPlugin>("plugin_install_from_url", {
        workspacePath,
        url: installUrl.trim(),
        force: installForce,
      });
      setInstallMsg(
        `Installed ${installed.name} v${installed.version} from ${installed.publisher.name} ` +
          `(key ${installed.publisher.key_fingerprint}…) — policy: ${installed.policy}`,
      );
      setInstallUrl("");
      setInstallForce(false);
      await load();
    } catch (e) {
      setInstallMsg(`Error: ${e}`);
    }
  }

  // Policy options the current row can transition to. Required ↔
  // anything-else needs the `As admin` toggle; the policy buttons
  // below are still rendered, but the click is gated by `isAdmin`
  // (the workspace-store enforces this regardless).
  const policyOptions: Policy[] = ["off", "on", "required"];

  if (!workspacePath) {
    return (
      <div className="panel-container">
        <div className="panel-header"><h3>Plugin Governance</h3></div>
        <div className="panel-body">
          <div style={{ color: "var(--text-muted)", fontStyle: "italic" }}>
            Open a workspace to view installed plugins.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Plugin Governance</h3>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>
            <input
              type="checkbox"
              checked={isAdmin}
              onChange={(e) => setIsAdmin(e.target.checked)}
            />
            As admin
          </label>
          <button className="panel-btn panel-btn-secondary" onClick={load} disabled={loading}>
            {loading ? "Loading…" : "Refresh"}
          </button>
        </div>
      </div>

      <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 18 }}>
        {error && <div className="panel-error"><span>{error}</span></div>}

        {/* Core catalog — the only plugins available without authoring a
            bundle first, which is why the empty state below used to be a dead
            end for anyone who did not already have one. */}
        <section>
          <div style={{ fontWeight: 600, marginBottom: 4 }}>
            Core plugins
            <span style={{ color: "var(--text-muted)", fontWeight: 400, marginLeft: 8, fontSize: "var(--font-size-sm)" }}>
              ({catalog.filter((c) => !c.installed).length} available)
            </span>
          </div>
          <div style={{ color: "var(--text-muted)", fontSize: "var(--font-size-sm)", marginBottom: 8 }}>
            Skills and rules that ship with VibeCody. Installed through the same verified
            path as a downloaded bundle, but signed by this machine — the signature proves
            the install has not been altered, not who wrote it.
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {catalog.map((c) => (
              <div
                key={c.name}
                style={{
                  background: "var(--bg-secondary)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "var(--radius-sm-alt)",
                  padding: 12,
                  display: "flex",
                  alignItems: "flex-start",
                  gap: 12,
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div>
                    <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{c.title}</span>
                    <span style={{ color: "var(--text-muted)", marginLeft: 8, fontSize: "var(--font-size-sm)" }}>
                      v{c.version}
                    </span>
                  </div>
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4, lineHeight: 1.5 }}>
                    {c.description}
                  </div>
                  {(c.includes?.length > 0 || c.connectors?.length > 0) && (
                    <div style={{ marginTop: 4, fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>
                      Includes{" "}
                      {[
                        c.includes?.length > 0
                          ? `${c.includes.length} plugin${c.includes.length === 1 ? "" : "s"}`
                          : null,
                        c.connectors?.length > 0 ? c.connectors.join(", ") : null,
                      ]
                        .filter(Boolean)
                        .join(" · ")}
                    </div>
                  )}
                  <div style={{ marginTop: 6, display: "flex", flexWrap: "wrap", gap: 4 }}>
                    {c.components.map((comp) => (
                      <span
                        key={`${comp.kind}/${comp.name}`}
                        style={{
                          fontSize: "var(--font-size-sm)",
                          padding: "1px 8px",
                          border: "1px solid var(--border-color)",
                          borderRadius: "var(--radius-md)",
                          color: "var(--text-muted)",
                        }}
                      >
                        {comp.kind} · {comp.name}
                      </span>
                    ))}
                  </div>
                </div>
                {c.installed && c.policy ? (
                  <span
                    style={{
                      padding: "2px 12px",
                      borderRadius: "var(--radius-md)",
                      fontSize: "var(--font-size-sm)",
                      fontWeight: 600,
                      background: POLICY_COLORS[c.policy].bg,
                      color: POLICY_COLORS[c.policy].fg,
                      flex: "none",
                    }}
                  >
                    {c.policy}
                  </span>
                ) : (
                  <button
                    className="panel-btn"
                    disabled={busyPlugin === c.name}
                    onClick={() => installFromCatalog(c)}
                    style={{ flex: "none", padding: "4px 14px", fontSize: "var(--font-size-sm)", fontWeight: 600 }}
                  >
                    {busyPlugin === c.name ? "Installing…" : "Install"}
                  </button>
                )}
              </div>
            ))}
          </div>
        </section>

        {/* Install signed MCPB bundle — local file or HTTPS URL. */}
        <section style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: 14 }}>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Install signed MCPB bundle</div>
          <div style={{ display: "flex", gap: 6, marginBottom: 10 }}>
            {(["file", "url"] as const).map((m) => (
              <button
                key={m}
                className={`panel-btn ${installMode === m ? "" : "panel-btn-secondary"}`}
                onClick={() => { setInstallMode(m); setInstallMsg(null); }}
                style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-sm)", fontWeight: installMode === m ? 600 : 400 }}
              >
                {m === "file" ? "Local file" : "HTTPS URL"}
              </button>
            ))}
          </div>
          {installMode === "file" ? (
            <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
              <input
                value={installPath}
                onChange={(e) => setInstallPath(e.target.value)}
                placeholder="/path/to/plugin.mcpb"
                style={{ flex: 1, padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}
              />
              <button className="panel-btn panel-btn-secondary" onClick={pickBundle}>Browse…</button>
            </div>
          ) : (
            <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
              <input
                value={installUrl}
                onChange={(e) => setInstallUrl(e.target.value)}
                placeholder="https://publisher.example/plugin.mcpb"
                style={{ flex: 1, padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}
              />
            </div>
          )}
          <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 10 }}>
            <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>
              <input type="checkbox" checked={installForce} onChange={(e) => setInstallForce(e.target.checked)} />
              Force re-install
            </label>
            <button
              className="panel-btn"
              onClick={installMode === "file" ? installFromFile : installFromUrl}
              disabled={installMode === "file" ? !installPath.trim() : !installUrl.trim()}
              style={{ padding: "8px 16px", borderRadius: "var(--radius-sm)", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontWeight: 600 }}
            >
              Install
            </button>
          </div>
          {installMsg && (
            <div style={{ fontSize: "var(--font-size-sm)", color: installMsg.startsWith("Error") ? "var(--error-color)" : "var(--success-color)" }}>
              {installMsg}
            </div>
          )}
        </section>

        {/* Installed-plugins list. */}
        <section>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>
            Installed plugins
            <span style={{ color: "var(--text-muted)", fontWeight: 400, marginLeft: 8, fontSize: "var(--font-size-sm)" }}>
              ({plugins.length})
            </span>
          </div>
          {!loading && plugins.length === 0 && (
            <div style={{ color: "var(--text-muted)", fontStyle: "italic" }}>
              None installed. Install a core plugin above, or a signed MCPB bundle from a
              file or URL.
            </div>
          )}
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {plugins.map((p) => (
              <div
                key={p.name}
                style={{
                  background: "var(--bg-secondary)",
                  border: "1px solid var(--border-color)",
                  borderRadius: "var(--radius-sm-alt)",
                  padding: 14,
                }}
              >
                <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", gap: 12, marginBottom: 6 }}>
                  <div>
                    <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{p.name}</span>
                    <span style={{ color: "var(--text-muted)", marginLeft: 8 }}>v{p.version}</span>
                    <span style={{ color: "var(--text-muted)", marginLeft: 12, fontSize: "var(--font-size-sm)" }}>
                      by {p.publisher.name}
                    </span>
                  </div>
                  <span
                    style={{
                      padding: "2px 12px",
                      borderRadius: "var(--radius-md)",
                      fontSize: "var(--font-size-sm)",
                      fontWeight: 600,
                      background: POLICY_COLORS[p.policy].bg,
                      color: POLICY_COLORS[p.policy].fg,
                    }}
                  >
                    {p.policy}
                  </span>
                </div>
                {p.description && (
                  <div style={{ color: "var(--text-muted)", fontSize: "var(--font-size-sm)", marginBottom: 8 }}>
                    {p.description}
                  </div>
                )}
                <div style={{ display: "flex", gap: 12, fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginBottom: 10, flexWrap: "wrap" }}>
                  <span>{p.components.mcp_servers} MCP</span>
                  <span>·</span>
                  <span>{p.components.skills} skills</span>
                  <span>·</span>
                  <span>{p.components.subagents} subagents</span>
                  <span>·</span>
                  <span>{p.components.rules} rules</span>
                  <span>·</span>
                  <span>{p.components.hooks} hooks</span>
                  <span style={{ marginLeft: "auto" }}>
                    key: <code>{p.publisher.key_fingerprint}…</code>
                  </span>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginRight: 4 }}>
                    Policy:
                  </span>
                  {policyOptions.map((opt) => (
                    <button
                      key={opt}
                      className="panel-btn panel-btn-secondary"
                      onClick={() => changePolicy(p.name, opt)}
                      disabled={busyPlugin === p.name || p.policy === opt}
                      style={{
                        padding: "4px 10px",
                        borderRadius: "var(--radius-sm)",
                        fontSize: "var(--font-size-sm)",
                        opacity: p.policy === opt ? 0.5 : 1,
                        fontWeight: p.policy === opt ? 600 : 400,
                      }}
                    >
                      {opt}
                    </button>
                  ))}
                  <button
                    className="panel-btn"
                    onClick={() => uninstall(p.name)}
                    disabled={busyPlugin === p.name}
                    style={{
                      marginLeft: "auto",
                      padding: "4px 12px",
                      borderRadius: "var(--radius-sm)",
                      fontSize: "var(--font-size-sm)",
                      background: "var(--error-color)22",
                      color: "var(--error-color)",
                      border: "1px solid var(--error-color)",
                    }}
                  >
                    Uninstall
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
