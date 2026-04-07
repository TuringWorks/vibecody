/**
 * CompanyPriorityMapPanel — Per-program urgency and routing rules.
 *
 * Configures P0/P1/P2/P3 urgency and routing for 7 company programs.
 * Values are persisted via company_priority_map_set Tauri command.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ProgramEntry {
  program: string;
  urgency: 0 | 1 | 2 | 3;
  routing_rules: string;
}

const PROGRAMS = ["Revenue", "EA", "Legal", "BizDev", "Marketing", "Product", "Personal"] as const;

const URGENCY_CONFIG: Record<0 | 1 | 2 | 3, { label: string; color: string; bg: string; desc: string }> = {
  0: { label: "P0", color: "var(--accent-rose)", bg: "rgba(231,76,60,0.18)", desc: "Critical" },
  1: { label: "P1", color: "var(--accent-gold)", bg: "rgba(255,193,7,0.18)", desc: "High" },
  2: { label: "P2", color: "var(--accent-blue)", bg: "rgba(74,158,255,0.18)", desc: "Medium" },
  3: { label: "P3", color: "var(--text-secondary)", bg: "rgba(128,128,128,0.12)", desc: "Low" },
};

function defaultMap(): ProgramEntry[] {
  return PROGRAMS.map((program) => ({ program, urgency: 2 as const, routing_rules: "" }));
}

const inputStyle: React.CSSProperties = {
  fontSize: 12, padding: "4px 8px",
  background: "var(--bg-primary)", border: "1px solid var(--border-color)",
  borderRadius: 4, color: "var(--text-primary)", flex: 1, minWidth: 0,
};

export function CompanyPriorityMapPanel() {
  const [map, setMap] = useState<ProgramEntry[]>(defaultMap());
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const result = await invoke<ProgramEntry[]>("company_priority_map_get");
      // Merge with default so all 7 programs always present
      const merged = PROGRAMS.map((prog) => {
        const existing = result.find((e) => e.program === prog);
        return existing ?? { program: prog, urgency: 2 as const, routing_rules: "" };
      });
      setMap(merged);
    } catch (_e) {
      // leave defaults
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const setUrgency = (index: number, urgency: 0 | 1 | 2 | 3) => {
    setMap((prev) => prev.map((e, i) => i === index ? { ...e, urgency } : e));
  };

  const setRouting = (index: number, routing_rules: string) => {
    setMap((prev) => prev.map((e, i) => i === index ? { ...e, routing_rules } : e));
  };

  const save = async () => {
    setSaving(true);
    try {
      await invoke("company_priority_map_set", { map });
      setToast("Priority map saved");
      setTimeout(() => setToast(null), 3000);
    } catch (e) {
      setToast(`Error: ${e}`);
      setTimeout(() => setToast(null), 5000);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="panel-container">
      <div className="panel-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Priority Map</span>
        <button onClick={load} className="panel-btn panel-btn-secondary">Refresh</button>
      </div>
      <div className="panel-body">

        {/* Legend */}
        <div style={{ display: "flex", gap: 12, marginBottom: 14, flexWrap: "wrap" }}>
          {([0, 1, 2, 3] as const).map((u) => {
            const cfg = URGENCY_CONFIG[u];
            return (
              <span key={u} style={{
                padding: "2px 10px", borderRadius: 10, fontSize: 11, fontWeight: 600,
                color: cfg.color, background: cfg.bg, border: `1px solid ${cfg.color}`,
              }}>
                {cfg.label} {cfg.desc}
              </span>
            );
          })}
        </div>

        {loading ? (
          <div className="panel-loading">Loading…</div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 16 }}>
            {/* Header row */}
            <div style={{ display: "grid", gridTemplateColumns: "110px 1fr 1fr", gap: 8, alignItems: "center", padding: "0 4px" }}>
              <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase" }}>Program</span>
              <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase" }}>Urgency</span>
              <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase" }}>Routing Rules</span>
            </div>
            {map.map((entry, i) => {
              const urgencyCfg = URGENCY_CONFIG[entry.urgency];
              return (
                <div
                  key={entry.program}
                  style={{
                    display: "grid", gridTemplateColumns: "110px 1fr 1fr", gap: 8,
                    alignItems: "center", padding: "8px 10px",
                    background: "var(--bg-secondary)", borderRadius: 6,
                    border: "1px solid var(--border-color)",
                  }}
                >
                  {/* Program name */}
                  <span style={{ fontSize: 13, fontWeight: 600 }}>{entry.program}</span>

                  {/* Urgency button group */}
                  <div style={{ display: "flex", gap: 4 }}>
                    {([0, 1, 2, 3] as const).map((u) => {
                      const cfg = URGENCY_CONFIG[u];
                      const selected = entry.urgency === u;
                      return (
                        <button
                          key={u}
                          onClick={() => setUrgency(i, u)}
                          style={{
                            padding: "2px 8px", borderRadius: 6, fontSize: 11, fontWeight: 700,
                            cursor: "pointer",
                            background: selected ? cfg.bg : "transparent",
                            color: selected ? cfg.color : "var(--text-secondary)",
                            border: `1px solid ${selected ? cfg.color : "var(--border-color)"}`,
                          }}
                          title={cfg.desc}
                        >
                          {cfg.label}
                        </button>
                      );
                    })}
                    <span style={{ fontSize: 10, color: urgencyCfg.color, alignSelf: "center", marginLeft: 4 }}>
                      {urgencyCfg.desc}
                    </span>
                  </div>

                  {/* Routing rules */}
                  <input
                    type="text"
                    value={entry.routing_rules}
                    onChange={(e) => setRouting(i, e.target.value)}
                    placeholder="e.g. cfo, legal-team"
                    style={inputStyle}
                  />
                </div>
              );
            })}
          </div>
        )}

        <button
          onClick={save}
          disabled={saving || loading}
          className="panel-btn panel-btn-primary"
          style={{ opacity: (saving || loading) ? 0.6 : 1 }}
        >
          {saving ? "Saving…" : "Save Map"}
        </button>

        {toast && (
          <div style={{
            marginTop: 12, padding: "8px 14px", borderRadius: 6, fontSize: 12,
            background: toast.startsWith("Error") ? "rgba(231,76,60,0.15)" : "rgba(39,174,96,0.15)",
            color: toast.startsWith("Error") ? "var(--accent-rose)" : "var(--accent-green)",
            border: `1px solid ${toast.startsWith("Error") ? "var(--accent-rose)" : "var(--accent-green)"}`,
          }}>
            {toast}
          </div>
        )}
      </div>
    </div>
  );
}
