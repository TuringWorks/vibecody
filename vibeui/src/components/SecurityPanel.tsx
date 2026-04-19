import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, RefreshCw } from "lucide-react";

interface SandboxStatus {
  enabled: boolean;
  supported: boolean;
  backend: string;
  profile_path?: string | null;
}

export function SecurityPanel() {
  const [status, setStatus] = useState<SandboxStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const next = await invoke<SandboxStatus>("get_sandbox_status");
      setStatus(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const toggle = async (next: boolean) => {
    if (!status) return;
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      const updated = await invoke<SandboxStatus>("set_sandbox_enabled", { enabled: next });
      setStatus(updated);
      setMessage(next ? "Agent sandbox enabled" : "Agent sandbox disabled");
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const backendLabel = (name: string) => {
    switch (name) {
      case "sandbox-exec":
        return "macOS sandbox-exec (Seatbelt)";
      case "bwrap":
        return "Linux bubblewrap (bwrap)";
      case "unsupported":
        return "Not supported on this OS";
      default:
        return name;
    }
  };

  return (
    <div
      className="panel"
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        color: "var(--text-primary)",
        background: "var(--bg-primary)",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "var(--space-2)",
          padding: "var(--space-3) var(--space-4)",
          borderBottom: "1px solid var(--border-color)",
        }}
      >
        <h3 style={{ margin: 0, fontSize: "var(--font-size-lg)" }}>Security</h3>
        <div style={{ flex: 1 }} />
        <button
          className="panel-btn"
          aria-label="Refresh sandbox status"
          onClick={reload}
          disabled={loading}
          style={{ padding: "var(--space-1) var(--space-2)" }}
        >
          {loading ? <Loader2 size={13} className="spin" /> : <RefreshCw size={13} />}
        </button>
      </div>

      {error && (
        <div
          role="alert"
          style={{
            padding: "var(--space-2) var(--space-4)",
            background: "var(--error-bg)",
            color: "var(--error-color)",
            fontSize: "var(--font-size-base)",
          }}
        >
          {error}
        </div>
      )}
      {message && !error && (
        <div
          role="status"
          style={{
            padding: "var(--space-2) var(--space-4)",
            background: "var(--success-bg)",
            color: "var(--success-color)",
            fontSize: "var(--font-size-base)",
          }}
        >
          {message}
        </div>
      )}

      <div style={{ flex: 1, overflowY: "auto", padding: "var(--space-4)" }}>
        <section
          style={{
            border: "1px solid var(--border-color)",
            borderRadius: "var(--radius-sm)",
            background: "var(--bg-secondary)",
            padding: "var(--space-4)",
            marginBottom: "var(--space-4)",
          }}
        >
          <h4 style={{ margin: 0, marginBottom: "var(--space-2)", fontSize: "var(--font-size-base)" }}>
            Agent OS sandbox
          </h4>
          <p
            style={{
              margin: 0,
              marginBottom: "var(--space-3)",
              color: "var(--text-secondary)",
              fontSize: "var(--font-size-base)",
              lineHeight: 1.5,
            }}
          >
            When enabled, VibeCLI runs agent-spawned shell commands inside the host OS
            sandbox to restrict filesystem and network access. The setting is written to
            <code> ~/.vibecli/config.toml</code> under <code>safety.sandbox</code> and is
            picked up on the next agent run.
          </p>

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "auto 1fr",
              gap: "var(--space-2) var(--space-4)",
              fontSize: "var(--font-size-base)",
              marginBottom: "var(--space-4)",
            }}
          >
            <span style={{ color: "var(--text-secondary)" }}>Backend</span>
            <span>
              <code>{status?.backend ?? "…"}</code>
              <span style={{ marginLeft: "var(--space-2)", color: "var(--text-tertiary)" }}>
                {status ? backendLabel(status.backend) : ""}
              </span>
            </span>
            <span style={{ color: "var(--text-secondary)" }}>Supported</span>
            <span
              style={{
                color: status?.supported ? "var(--success-color)" : "var(--warning-color)",
                fontWeight: 600,
              }}
            >
              {status ? (status.supported ? "Yes" : "No") : "…"}
            </span>
            <span style={{ color: "var(--text-secondary)" }}>Custom profile</span>
            <span>
              <code>{status?.profile_path ?? "(default)"}</code>
            </span>
            <span style={{ color: "var(--text-secondary)" }}>State</span>
            <span
              style={{
                color: status?.enabled ? "var(--success-color)" : "var(--text-tertiary)",
                fontWeight: 600,
              }}
            >
              {status ? (status.enabled ? "Enabled" : "Disabled") : "…"}
            </span>
          </div>

          <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
            <button
              className="panel-btn"
              onClick={() => toggle(true)}
              disabled={saving || !status || status.enabled}
              aria-label="Enable agent sandbox"
              style={{
                padding: "var(--space-1) var(--space-3)",
                background: "var(--btn-primary-bg)",
                color: "var(--btn-primary-fg)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-xs-plus)",
              }}
            >
              {saving ? <Loader2 size={13} className="spin" /> : "Enable"}
            </button>
            <button
              className="panel-btn"
              onClick={() => toggle(false)}
              disabled={saving || !status || !status.enabled}
              aria-label="Disable agent sandbox"
              style={{
                padding: "var(--space-1) var(--space-3)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-xs-plus)",
              }}
            >
              {saving ? <Loader2 size={13} className="spin" /> : "Disable"}
            </button>
            {status && !status.supported && (
              <span
                role="note"
                style={{
                  marginLeft: "var(--space-2)",
                  color: "var(--warning-color)",
                  fontSize: "var(--font-size-xs)",
                }}
              >
                No sandbox backend on this OS — toggle persists but has no effect until run on macOS or Linux.
              </span>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}

export default SecurityPanel;
