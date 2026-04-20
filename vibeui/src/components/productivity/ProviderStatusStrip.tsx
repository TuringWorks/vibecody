import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, CircleSlash, Loader2, RefreshCw } from "lucide-react";
import type { ProviderStatus } from "../../types/productivity";

type Tab = "email" | "calendar" | "tasks" | "notion" | "jira" | "home";

const STATUS_CMD: Partial<Record<Tab, string>> = {
  email: "productivity_email_status",
  calendar: "productivity_cal_status",
  tasks: "productivity_tasks_status",
  notion: "productivity_notion_status",
  jira: "productivity_jira_status",
  home: "productivity_home_status",
};

function providerLabel(p: string | null): string {
  if (!p) return "";
  if (p === "gmail") return "Gmail";
  if (p === "outlook") return "Outlook";
  if (p === "google") return "Google";
  return p.charAt(0).toUpperCase() + p.slice(1);
}

interface Props {
  tab: Tab;
}

export function ProviderStatusStrip({ tab }: Props) {
  const cmd = STATUS_CMD[tab];
  const [status, setStatus] = useState<ProviderStatus | null>(null);
  const [loading, setLoading] = useState(false);

  async function refresh() {
    if (!cmd) return;
    setLoading(true);
    try {
      const s = await invoke<ProviderStatus>(cmd);
      setStatus(s);
    } catch (e) {
      setStatus({
        connected: false,
        provider: null,
        account: null,
        message: String(e),
      });
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab]);

  // Phase-5 tabs: no status command yet → hide the strip to keep UI clean.
  if (!cmd) return null;

  const ok = status?.connected === true;

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "4px 10px",
        borderBottom: "1px solid var(--border-color)",
        background: "var(--bg-secondary)",
        fontSize: "calc(var(--font-size-sm) - 1px)",
        color: "var(--text-secondary)",
        flexShrink: 0,
      }}
    >
      {loading ? (
        <Loader2 size={11} style={{ animation: "spin 1s linear infinite" }} />
      ) : ok ? (
        <CheckCircle2 size={11} color="var(--color-success, #3aa655)" />
      ) : (
        <CircleSlash size={11} color="var(--text-secondary)" />
      )}
      {status && (
        <span>
          {ok ? (
            <>
              Connected
              {status.provider && ` · ${providerLabel(status.provider)}`}
              {status.account && (
                <span style={{ color: "var(--text-primary)" }}>
                  {" "}
                  · {status.account}
                </span>
              )}
            </>
          ) : (
            <>
              Not connected
              {status.message && (
                <span style={{ marginLeft: 6, opacity: 0.75 }}>
                  — {status.message.slice(0, 80)}
                </span>
              )}
            </>
          )}
        </span>
      )}
      <span style={{ flex: 1 }} />
      <button
        className="panel-btn panel-btn-secondary"
        onClick={refresh}
        disabled={loading}
        title="Re-check connection"
        style={{
          display: "flex",
          alignItems: "center",
          padding: "2px 6px",
          fontSize: "calc(var(--font-size-sm) - 1px)",
        }}
      >
        <RefreshCw size={10} />
      </button>
    </div>
  );
}
