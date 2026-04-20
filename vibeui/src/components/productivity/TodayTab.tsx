import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Bot,
  Calendar as CalendarIcon,
  CheckCircle2,
  Circle,
  ChevronRight,
  Clock,
  Flame,
  Inbox,
  ListTodo,
  Loader2,
  Mail,
  MapPin,
  RefreshCw,
  Sparkles,
  X,
  Zap,
} from "lucide-react";
import type {
  CalendarEvent,
  Email,
  ProviderStatus,
  TodoistTask,
} from "../../types/productivity";

export type TodayNavTarget =
  | { tab: "email"; emailId?: string }
  | { tab: "calendar"; eventId?: string }
  | { tab: "tasks" };

interface Props {
  onNavigate: (target: TodayNavTarget) => void;
}

interface PlanMyDayResult {
  plan: string;
  provider: string;
  model: string;
  duration_ms: number;
  context_summary: string;
}

interface TodayData {
  emails: Email[];
  events: CalendarEvent[];
  tasks: TodoistTask[];
  emailStatus: ProviderStatus | null;
  calStatus: ProviderStatus | null;
  taskStatus: ProviderStatus | null;
  emailErr: string | null;
  calErr: string | null;
  taskErr: string | null;
}

function fmtTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  } catch {
    return iso;
  }
}

function priorityIcon(p: number) {
  if (p === 4) return <Flame size={11} color="var(--color-error, #d63e3e)" />;
  if (p === 3) return <Zap size={11} color="var(--color-warn, #c69023)" />;
  if (p === 2) return <Circle size={9} color="var(--text-secondary)" strokeWidth={2} />;
  return <span style={{ color: "var(--text-secondary)", width: 11, textAlign: "center" }}>·</span>;
}

export function TodayTab({ onNavigate }: Props) {
  const [data, setData] = useState<TodayData>({
    emails: [],
    events: [],
    tasks: [],
    emailStatus: null,
    calStatus: null,
    taskStatus: null,
    emailErr: null,
    calErr: null,
    taskErr: null,
  });
  const [loading, setLoading] = useState(false);
  const [completing, setCompleting] = useState<string | null>(null);
  const [plan, setPlan] = useState<PlanMyDayResult | null>(null);
  const [planLoading, setPlanLoading] = useState(false);
  const [planErr, setPlanErr] = useState<string | null>(null);

  async function planMyDay() {
    setPlanLoading(true);
    setPlanErr(null);
    try {
      const r = await invoke<PlanMyDayResult>("productivity_plan_my_day");
      setPlan(r);
    } catch (e) {
      setPlanErr(String(e));
    } finally {
      setPlanLoading(false);
    }
  }

  const refresh = useCallback(async () => {
    setLoading(true);
    const [
      emailStatusR,
      calStatusR,
      taskStatusR,
      emailsR,
      eventsR,
      tasksR,
    ] = await Promise.allSettled([
      invoke<ProviderStatus>("productivity_email_status"),
      invoke<ProviderStatus>("productivity_cal_status"),
      invoke<ProviderStatus>("productivity_tasks_status"),
      invoke<Email[]>("productivity_email_list", { query: "is:unread", max: 5 }),
      invoke<CalendarEvent[]>("productivity_cal_today"),
      invoke<TodoistTask[]>("productivity_tasks_list", { filter: "today" }),
    ]);
    setData({
      emailStatus: emailStatusR.status === "fulfilled" ? emailStatusR.value : null,
      calStatus: calStatusR.status === "fulfilled" ? calStatusR.value : null,
      taskStatus: taskStatusR.status === "fulfilled" ? taskStatusR.value : null,
      emails: emailsR.status === "fulfilled" ? emailsR.value : [],
      events: eventsR.status === "fulfilled" ? eventsR.value : [],
      tasks: tasksR.status === "fulfilled" ? tasksR.value : [],
      emailErr: emailsR.status === "rejected" ? String(emailsR.reason) : null,
      calErr: eventsR.status === "rejected" ? String(eventsR.reason) : null,
      taskErr: tasksR.status === "rejected" ? String(tasksR.reason) : null,
    });
    setLoading(false);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function completeTask(id: string) {
    setCompleting(id);
    try {
      await invoke("productivity_tasks_close", { id });
      setData((d) => ({ ...d, tasks: d.tasks.filter((t) => t.id !== id) }));
    } catch {
      // ignore — the user can retry from the Tasks tab with a full error view
    } finally {
      setCompleting(null);
    }
  }

  const today = new Date().toLocaleDateString([], {
    weekday: "long",
    month: "long",
    day: "numeric",
  });

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        flex: 1,
        overflow: "hidden",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "10px 12px",
          borderBottom: "1px solid var(--border-color)",
          background: "var(--bg-secondary)",
        }}
      >
        <strong style={{ fontSize: "var(--font-size-base)" }}>{today}</strong>
        <span style={{ flex: 1 }} />
        <button
          className="panel-btn panel-btn-primary"
          onClick={planMyDay}
          disabled={planLoading}
          title="Let AI synthesize a plan from your emails, events, and tasks"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {planLoading ? (
            <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
          ) : (
            <Sparkles size={12} />
          )}
          Plan my day
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={refresh}
          disabled={loading}
          title="Refresh all"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {loading ? (
            <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
          ) : (
            <RefreshCw size={12} />
          )}
          Refresh
        </button>
      </div>
      {(plan || planErr || planLoading) && (
        <div
          style={{
            padding: 12,
            borderBottom: "1px solid var(--border-color)",
            background: "var(--bg-primary)",
            display: "flex",
            flexDirection: "column",
            gap: 6,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Bot size={13} color="var(--color-accent, var(--text-primary))" />
            <strong style={{ flex: 1 }}>Plan my day</strong>
            {plan && (
              <span
                style={{
                  color: "var(--text-secondary)",
                  fontSize: "calc(var(--font-size-sm) - 1px)",
                }}
              >
                {plan.provider} · {plan.model} · {(plan.duration_ms / 1000).toFixed(1)}s
              </span>
            )}
            <button
              onClick={() => {
                setPlan(null);
                setPlanErr(null);
              }}
              title="Dismiss"
              style={{
                background: "none",
                border: "none",
                padding: 2,
                cursor: "pointer",
                color: "var(--text-secondary)",
                display: "flex",
              }}
            >
              <X size={12} />
            </button>
          </div>
          {planLoading ? (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                color: "var(--text-secondary)",
                fontSize: "var(--font-size-sm)",
              }}
            >
              <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
              Synthesizing plan from today's events, tasks, and email…
            </div>
          ) : planErr ? (
            <div
              style={{
                color: "var(--color-error, #d63e3e)",
                fontSize: "var(--font-size-sm)",
              }}
            >
              {planErr}
            </div>
          ) : plan ? (
            <>
              <span
                style={{
                  color: "var(--text-secondary)",
                  fontSize: "calc(var(--font-size-sm) - 1px)",
                }}
              >
                {plan.context_summary}
              </span>
              <pre
                style={{
                  margin: 0,
                  whiteSpace: "pre-wrap",
                  fontFamily: "var(--font-family)",
                  fontSize: "var(--font-size-sm)",
                  lineHeight: 1.5,
                }}
              >
                {plan.plan}
              </pre>
            </>
          ) : null}
        </div>
      )}
      <div
        style={{
          flex: 1,
          overflowY: "auto",
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))",
          gap: 12,
          padding: 12,
          alignContent: "start",
        }}
      >
        <Section
          title="Unread email"
          icon={Mail}
          count={data.emails.length}
          status={data.emailStatus}
          err={data.emailErr}
          onOpen={() => onNavigate({ tab: "email" })}
          empty="No unread email."
        >
          {data.emails.map((m) => (
            <button
              key={m.id}
              onClick={() => onNavigate({ tab: "email", emailId: m.id })}
              className="panel-card panel-card--clickable"
              style={{
                display: "grid",
                gridTemplateColumns: "16px 1fr",
                gap: 8,
                alignItems: "center",
                width: "100%",
                textAlign: "left",
                background: "transparent",
                border: "none",
                borderBottom: "1px solid var(--border-color)",
                padding: "6px 8px",
                cursor: "pointer",
                color: "inherit",
                fontSize: "var(--font-size-sm)",
              }}
            >
              <Inbox size={12} color="var(--text-secondary)" />
              <span
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 2,
                  overflow: "hidden",
                }}
              >
                <span
                  style={{
                    fontWeight: 600,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {m.subject || "(no subject)"}
                </span>
                <span
                  style={{
                    color: "var(--text-secondary)",
                    fontSize: "calc(var(--font-size-sm) - 1px)",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {m.from} — {m.snippet}
                </span>
              </span>
            </button>
          ))}
        </Section>

        <Section
          title="Today's events"
          icon={CalendarIcon}
          count={data.events.length}
          status={data.calStatus}
          err={data.calErr}
          onOpen={() => onNavigate({ tab: "calendar" })}
          empty="Nothing on the calendar."
        >
          {data.events.map((e) => (
            <button
              key={e.id}
              onClick={() => onNavigate({ tab: "calendar", eventId: e.id })}
              className="panel-card panel-card--clickable"
              style={{
                display: "grid",
                gridTemplateColumns: "72px 1fr",
                gap: 8,
                alignItems: "center",
                width: "100%",
                textAlign: "left",
                background: "transparent",
                border: "none",
                borderBottom: "1px solid var(--border-color)",
                padding: "6px 8px",
                cursor: "pointer",
                color: "inherit",
                fontSize: "var(--font-size-sm)",
                opacity: e.status === "cancelled" ? 0.6 : 1,
                textDecoration: e.status === "cancelled" ? "line-through" : "none",
              }}
            >
              <span
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 4,
                  color: "var(--text-secondary)",
                  fontSize: "calc(var(--font-size-sm) - 1px)",
                }}
              >
                <Clock size={10} />
                {fmtTime(e.start)}
              </span>
              <span
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 2,
                  overflow: "hidden",
                }}
              >
                <span
                  style={{
                    fontWeight: 600,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {e.summary || "(untitled)"}
                </span>
                {e.location && (
                  <span
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 3,
                      color: "var(--text-secondary)",
                      fontSize: "calc(var(--font-size-sm) - 1px)",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    <MapPin size={9} />
                    {e.location}
                  </span>
                )}
              </span>
            </button>
          ))}
        </Section>

        <Section
          title="Today's tasks"
          icon={ListTodo}
          count={data.tasks.length}
          status={data.taskStatus}
          err={data.taskErr}
          onOpen={() => onNavigate({ tab: "tasks" })}
          empty="Nothing due today."
        >
          {data.tasks.map((t) => (
            <div
              key={t.id}
              style={{
                display: "grid",
                gridTemplateColumns: "20px 14px 1fr auto",
                gap: 8,
                alignItems: "center",
                padding: "6px 8px",
                borderBottom: "1px solid var(--border-color)",
                fontSize: "var(--font-size-sm)",
              }}
            >
              <button
                onClick={() => completeTask(t.id)}
                disabled={completing === t.id}
                title="Complete task"
                style={{
                  background: "none",
                  border: "none",
                  padding: 0,
                  cursor: "pointer",
                  color: "inherit",
                  display: "flex",
                  alignItems: "center",
                }}
              >
                {completing === t.id ? (
                  <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
                ) : (
                  <Circle size={12} strokeWidth={1.5} color="var(--text-secondary)" />
                )}
              </button>
              <span style={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
                {priorityIcon(t.priority)}
              </span>
              <span
                style={{
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {t.content}
              </span>
              {t.due && (
                <span
                  style={{
                    color: "var(--text-secondary)",
                    fontSize: "calc(var(--font-size-sm) - 1px)",
                    whiteSpace: "nowrap",
                  }}
                >
                  {t.due}
                </span>
              )}
            </div>
          ))}
        </Section>
      </div>
    </div>
  );
}

function Section({
  title,
  icon: Icon,
  count,
  status,
  err,
  onOpen,
  empty,
  children,
}: {
  title: string;
  icon: typeof Mail;
  count: number;
  status: ProviderStatus | null;
  err: string | null;
  onOpen: () => void;
  empty: string;
  children: React.ReactNode;
}) {
  const notConnected = status && !status.connected;
  return (
    <div
      style={{
        border: "1px solid var(--border-color)",
        borderRadius: "var(--radius-sm)",
        background: "var(--bg-primary)",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <button
        onClick={onOpen}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "8px 10px",
          background: "var(--bg-secondary)",
          borderBottom: "1px solid var(--border-color)",
          border: "none",
          cursor: "pointer",
          color: "inherit",
          textAlign: "left",
        }}
      >
        <Icon size={13} color="var(--text-secondary)" />
        <strong style={{ flex: 1 }}>{title}</strong>
        <span
          style={{
            color: "var(--text-secondary)",
            fontSize: "calc(var(--font-size-sm) - 1px)",
          }}
        >
          {count}
        </span>
        <ChevronRight size={12} color="var(--text-secondary)" />
      </button>
      {notConnected ? (
        <div
          style={{
            padding: 12,
            color: "var(--text-secondary)",
            fontSize: "var(--font-size-sm)",
            display: "flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          Not connected{status?.message ? ` — ${status.message.slice(0, 80)}` : ""}
        </div>
      ) : err ? (
        <div
          style={{
            padding: 12,
            color: "var(--color-error, #d63e3e)",
            fontSize: "var(--font-size-sm)",
          }}
        >
          {err}
        </div>
      ) : count === 0 ? (
        <div
          style={{
            padding: 12,
            color: "var(--text-secondary)",
            fontSize: "var(--font-size-sm)",
            textAlign: "center",
          }}
        >
          <CheckCircle2
            size={14}
            color="var(--color-success, #3aa655)"
            style={{ verticalAlign: "middle", marginRight: 4 }}
          />
          {empty}
        </div>
      ) : (
        <div>{children}</div>
      )}
    </div>
  );
}
