import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Fan,
  Lightbulb,
  Loader2,
  Lock,
  Plug,
  Power,
  RefreshCw,
  Sparkles,
  Terminal,
  Thermometer,
  ToggleLeft,
  ToggleRight,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { HaEntity } from "../../types/productivity";
import { ProviderStatusStrip } from "./ProviderStatusStrip";

const DOMAIN_ORDER = [
  "light",
  "switch",
  "scene",
  "climate",
  "fan",
  "lock",
  "media_player",
  "sensor",
  "binary_sensor",
];

const DOMAIN_ICONS: Record<string, LucideIcon> = {
  light: Lightbulb,
  switch: Plug,
  scene: Sparkles,
  climate: Thermometer,
  fan: Fan,
  lock: Lock,
  media_player: Power,
  sensor: ToggleLeft,
  binary_sensor: ToggleLeft,
};

function entityDomain(id: string): string {
  const dot = id.indexOf(".");
  return dot > 0 ? id.slice(0, dot) : "other";
}

function friendlyName(e: HaEntity): string {
  const n = e.attributes["friendly_name"];
  return typeof n === "string" ? n : e.entity_id;
}

function isOn(state: string): boolean {
  return state === "on" || state === "home" || state === "playing" || state === "unlocked";
}

export function HomeTab() {
  const [entities, setEntities] = useState<HaEntity[]>([]);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [cmd, setCmd] = useState("");
  const [cmdOutput, setCmdOutput] = useState("");
  const [cmdBusy, setCmdBusy] = useState(false);

  const fetchStates = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const list = await invoke<HaEntity[]>("productivity_home_list");
      setEntities(list);
    } catch (e) {
      setErr(String(e));
      setEntities([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStates();
  }, [fetchStates]);

  async function callService(
    domain: string,
    service: string,
    entityId: string,
  ) {
    setBusy(entityId);
    try {
      await invoke("productivity_home_call_service", {
        domain,
        service,
        entityId,
        data: null,
      });
      setTimeout(fetchStates, 300);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function toggleEntity(e: HaEntity) {
    const d = entityDomain(e.entity_id);
    if (d === "scene" || d === "script") {
      await callService(d, "turn_on", e.entity_id);
      return;
    }
    const service = isOn(e.state) ? "turn_off" : "turn_on";
    await callService(d, service, e.entity_id);
  }

  async function runAdvancedCmd() {
    if (!cmd.trim()) return;
    setCmdBusy(true);
    try {
      const out = await invoke<string>("handle_ha_command", { args: cmd });
      setCmdOutput(out);
    } catch (e) {
      setCmdOutput(`Error: ${e}`);
    } finally {
      setCmdBusy(false);
    }
  }

  const grouped = useMemo(() => {
    const f = filter.trim().toLowerCase();
    const filtered = f
      ? entities.filter(
          (e) =>
            e.entity_id.toLowerCase().includes(f) ||
            friendlyName(e).toLowerCase().includes(f),
        )
      : entities;
    const groups: Record<string, HaEntity[]> = {};
    for (const e of filtered) {
      const d = entityDomain(e.entity_id);
      (groups[d] ||= []).push(e);
    }
    const keys = Object.keys(groups);
    keys.sort((a, b) => {
      const ai = DOMAIN_ORDER.indexOf(a);
      const bi = DOMAIN_ORDER.indexOf(b);
      if (ai === -1 && bi === -1) return a.localeCompare(b);
      if (ai === -1) return 1;
      if (bi === -1) return -1;
      return ai - bi;
    });
    return keys.map((k) => [k, groups[k]] as const);
  }, [entities, filter]);

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      <ProviderStatusStrip tab="home" />
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-color)",
        }}
      >
        <input
          className="panel-input"
          style={{ flex: 1 }}
          placeholder="Filter entities…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <button
          className="panel-btn panel-btn-secondary"
          onClick={fetchStates}
          disabled={loading}
          title="Refresh"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {loading ? (
            <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
          ) : (
            <RefreshCw size={12} />
          )}
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => setShowAdvanced((s) => !s)}
          title="Advanced: raw /home commands"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <Terminal size={12} />
        </button>
      </div>
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
      <div style={{ flex: 1, overflowY: "auto" }}>
        {loading && entities.length === 0 ? (
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
            Loading entities…
          </div>
        ) : entities.length === 0 ? (
          <div
            style={{
              padding: 20,
              color: "var(--text-secondary)",
              textAlign: "center",
              fontSize: "var(--font-size-sm)",
            }}
          >
            No entities.
          </div>
        ) : (
          grouped.map(([domain, list]) => {
            const Icon = DOMAIN_ICONS[domain] ?? ToggleLeft;
            return (
              <div key={domain}>
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    padding: "6px 10px",
                    background: "var(--bg-secondary)",
                    borderBottom: "1px solid var(--border-color)",
                    fontSize: "calc(var(--font-size-sm) - 1px)",
                    color: "var(--text-secondary)",
                    textTransform: "uppercase",
                    letterSpacing: 0.5,
                    position: "sticky",
                    top: 0,
                    zIndex: 1,
                  }}
                >
                  <Icon size={11} />
                  {domain} · {list.length}
                </div>
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
                    gap: 8,
                    padding: 10,
                  }}
                >
                  {list.map((e) => {
                    const d = entityDomain(e.entity_id);
                    const on = isOn(e.state);
                    const actionable =
                      d === "light" ||
                      d === "switch" ||
                      d === "scene" ||
                      d === "fan" ||
                      d === "media_player" ||
                      d === "script";
                    return (
                      <button
                        key={e.entity_id}
                        onClick={() => actionable && toggleEntity(e)}
                        disabled={!actionable || busy === e.entity_id}
                        className="panel-card panel-card--clickable"
                        style={{
                          display: "flex",
                          flexDirection: "column",
                          gap: 4,
                          padding: 10,
                          border: "1px solid var(--border-color)",
                          borderRadius: "var(--radius-xs-plus)",
                          background: on
                            ? "var(--bg-tertiary)"
                            : "var(--bg-secondary)",
                          color: "inherit",
                          cursor: actionable ? "pointer" : "default",
                          textAlign: "left",
                          fontSize: "var(--font-size-sm)",
                          opacity: busy === e.entity_id ? 0.6 : 1,
                        }}
                        title={e.entity_id}
                      >
                        <div
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                          }}
                        >
                          <Icon
                            size={13}
                            color={on ? "var(--color-success, #3aa655)" : "var(--text-secondary)"}
                          />
                          <span
                            style={{
                              flex: 1,
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                              fontWeight: on ? 600 : 400,
                            }}
                          >
                            {friendlyName(e)}
                          </span>
                          {busy === e.entity_id ? (
                            <Loader2
                              size={12}
                              style={{ animation: "spin 1s linear infinite" }}
                            />
                          ) : actionable ? (
                            on ? (
                              <ToggleRight
                                size={14}
                                color="var(--color-success, #3aa655)"
                              />
                            ) : (
                              <ToggleLeft size={14} color="var(--text-secondary)" />
                            )
                          ) : null}
                        </div>
                        <div
                          style={{
                            fontSize: "calc(var(--font-size-sm) - 2px)",
                            color: "var(--text-secondary)",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                        >
                          {e.state}
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
            );
          })
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
              placeholder="status | lights | on <entity> | off <entity> | scene <name>"
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
