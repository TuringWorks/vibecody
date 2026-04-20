import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CalendarCheck,
  CalendarDays,
  CalendarPlus,
  CalendarRange,
  Loader2,
  RefreshCw,
  SkipForward,
  Terminal,
} from "lucide-react";
import type { CalendarEvent, FreeSlot } from "../../types/productivity";
import { CalendarList } from "./CalendarList";
import { EventDetail } from "./EventDetail";
import { EventComposer } from "./EventComposer";
import { ProviderStatusStrip } from "./ProviderStatusStrip";

type View = "today" | "week" | "upcoming" | "free";

const VIEW_CMD: Record<Exclude<View, "free">, string> = {
  today: "productivity_cal_today",
  week: "productivity_cal_week",
  upcoming: "productivity_cal_upcoming",
};

interface Props {
  initialEventId?: string;
}

export function CalendarTab({ initialEventId }: Props = {}) {
  const [view, setView] = useState<View>("today");
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [free, setFree] = useState<FreeSlot[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(initialEventId ?? null);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [cmd, setCmd] = useState("");
  const [cmdOutput, setCmdOutput] = useState("");
  const [cmdBusy, setCmdBusy] = useState(false);
  const [composing, setComposing] = useState(false);

  const fetchView = useCallback(async (v: View) => {
    setLoading(true);
    setErr(null);
    try {
      if (v === "free") {
        const slots = await invoke<FreeSlot[]>("productivity_cal_free_today");
        setFree(slots);
        setEvents([]);
      } else {
        const list = await invoke<CalendarEvent[]>(VIEW_CMD[v], { max: 20 });
        setEvents(list);
        setFree([]);
      }
    } catch (e) {
      setErr(String(e));
      setEvents([]);
      setFree([]);
    } finally {
      setLoading(false);
    }
  }, []);

  const firstRender = useRef(true);
  useEffect(() => {
    fetchView(view);
    if (firstRender.current) {
      firstRender.current = false;
    } else {
      setSelectedId(null);
    }
  }, [view, fetchView]);

  const selected = useMemo(
    () => events.find((e) => e.id === selectedId) ?? null,
    [events, selectedId],
  );

  const handleDeleted = useCallback((id: string) => {
    setEvents((prev) => prev.filter((e) => e.id !== id));
    setSelectedId(null);
  }, []);

  async function runAdvancedCmd() {
    if (!cmd.trim()) return;
    setCmdBusy(true);
    try {
      const out = await invoke<string>("handle_calendar_command", { args: cmd });
      setCmdOutput(out);
    } catch (e) {
      setCmdOutput(`Error: ${e}`);
    } finally {
      setCmdBusy(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      <ProviderStatusStrip tab="calendar" />
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-color)",
          flexWrap: "wrap",
        }}
      >
        <button
          className={`panel-btn panel-btn-secondary${view === "today" ? " active" : ""}`}
          onClick={() => setView("today")}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <CalendarCheck size={12} />
          Today
        </button>
        <button
          className={`panel-btn panel-btn-secondary${view === "week" ? " active" : ""}`}
          onClick={() => setView("week")}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <CalendarRange size={12} />
          Week
        </button>
        <button
          className={`panel-btn panel-btn-secondary${view === "upcoming" ? " active" : ""}`}
          onClick={() => setView("upcoming")}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <SkipForward size={12} />
          Upcoming
        </button>
        <button
          className={`panel-btn panel-btn-secondary${view === "free" ? " active" : ""}`}
          onClick={() => setView("free")}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <CalendarDays size={12} />
          Free today
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => fetchView(view)}
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
          className="panel-btn panel-btn-primary"
          onClick={() => setComposing(true)}
          title="Create a new event"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <CalendarPlus size={12} />
          New event
        </button>
        <span style={{ flex: 1 }} />
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => setShowAdvanced((s) => !s)}
          title="Advanced: run raw /calendar commands"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <Terminal size={12} />
        </button>
      </div>
      {err && (
        <div
          style={{
            padding: "6px 10px",
            background: "var(--bg-secondary)",
            color: "var(--color-error, #d63e3e)",
            fontSize: "var(--font-size-sm)",
            borderBottom: "1px solid var(--border-color)",
          }}
        >
          {err}
        </div>
      )}
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        <div
          style={{
            width: selected ? "40%" : "100%",
            minWidth: 260,
            borderRight: selected ? "1px solid var(--border-color)" : "none",
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          {view === "free" ? (
            <FreeSlotsView slots={free} loading={loading} />
          ) : loading && events.length === 0 ? (
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
              Loading events…
            </div>
          ) : (
            <CalendarList
              events={events}
              selectedId={selectedId}
              onSelect={setSelectedId}
            />
          )}
        </div>
        {selected && (
          <EventDetail
            event={selected}
            onClose={() => setSelectedId(null)}
            onDeleted={handleDeleted}
          />
        )}
      </div>
      {composing && (
        <EventComposer
          onClose={() => setComposing(false)}
          onCreated={(ev) => {
            setEvents((prev) => [ev, ...prev]);
            fetchView(view);
          }}
        />
      )}
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
              placeholder="today | week | list [days] | create <title> <start> <end> | free [date] | move <id> <start> | next"
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

function FreeSlotsView({ slots, loading }: { slots: FreeSlot[]; loading: boolean }) {
  if (loading) {
    return (
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
        Computing free slots…
      </div>
    );
  }
  if (slots.length === 0) {
    return (
      <div
        style={{
          padding: 20,
          color: "var(--text-secondary)",
          textAlign: "center",
          fontSize: "var(--font-size-sm)",
        }}
      >
        No free slots today during working hours.
      </div>
    );
  }
  return (
    <div style={{ padding: 10, overflowY: "auto", flex: 1 }}>
      <div
        style={{
          fontSize: "calc(var(--font-size-sm) - 1px)",
          color: "var(--text-secondary)",
          marginBottom: 8,
        }}
      >
        {slots.length} open slot{slots.length === 1 ? "" : "s"} (9am–6pm)
      </div>
      {slots.map((s, i) => {
        const start = new Date(s.start);
        const end = new Date(s.end);
        const mins = Math.round((end.getTime() - start.getTime()) / 60000);
        const label = mins >= 60 ? `${Math.round(mins / 60 * 10) / 10}h` : `${mins}m`;
        return (
          <div
            key={i}
            style={{
              display: "flex",
              justifyContent: "space-between",
              padding: "6px 10px",
              borderBottom: "1px solid var(--border-color)",
              fontSize: "var(--font-size-sm)",
            }}
          >
            <span>
              {start.toLocaleTimeString(undefined, {
                hour: "2-digit",
                minute: "2-digit",
              })}{" "}
              –{" "}
              {end.toLocaleTimeString(undefined, {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </span>
            <span style={{ color: "var(--text-secondary)" }}>{label}</span>
          </div>
        );
      })}
    </div>
  );
}
