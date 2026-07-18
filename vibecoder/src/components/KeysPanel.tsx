import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, RefreshCw } from "lucide-react";
import { useModelRegistry } from "../hooks/useModelRegistry";

const DEFAULT_PROFILE = "default";

interface KeyRow {
  provider: string;
  hasKey: boolean;
  masked?: string;
  revealed?: string;
  editing?: boolean;
  draft?: string;
  saving?: boolean;
}

function maskKey(raw: string): string {
  if (!raw) return "";
  if (raw.length <= 8) return "•".repeat(raw.length);
  return `${raw.slice(0, 4)}${"•".repeat(Math.max(4, raw.length - 8))}${raw.slice(-4)}`;
}

export function KeysPanel() {
  const { providers } = useModelRegistry();
  const [profileId, setProfileId] = useState<string>(DEFAULT_PROFILE);
  const [rows, setRows] = useState<Record<string, KeyRow>>({});
  const [filter, setFilter] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const loadProfile = useCallback(async () => {
    try {
      const id = await invoke<string>("panel_settings_get_default_profile");
      if (id) setProfileId(id);
    } catch {
      /* fall back to DEFAULT_PROFILE */
    }
  }, []);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const configured = await invoke<string[]>("profile_api_key_list", { profileId });
      const next: Record<string, KeyRow> = {};
      for (const p of providers) {
        next[p] = { provider: p, hasKey: configured.includes(p) };
      }
      for (const p of configured) {
        if (!next[p]) next[p] = { provider: p, hasKey: true };
      }
      setRows(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [providers, profileId]);

  useEffect(() => {
    loadProfile();
  }, [loadProfile]);

  useEffect(() => {
    reload();
  }, [reload]);

  const startEdit = (provider: string) => {
    setRows((prev) => ({
      ...prev,
      [provider]: { ...prev[provider], editing: true, draft: "" },
    }));
  };

  const cancelEdit = (provider: string) => {
    setRows((prev) => ({
      ...prev,
      [provider]: { ...prev[provider], editing: false, draft: undefined },
    }));
  };

  const save = async (provider: string) => {
    const row = rows[provider];
    const draft = (row?.draft ?? "").trim();
    if (!draft) {
      setError("Key must not be empty");
      return;
    }
    setRows((prev) => ({ ...prev, [provider]: { ...prev[provider], saving: true } }));
    setError(null);
    try {
      await invoke("profile_api_key_set", { profileId, provider, apiKey: draft });
      setStatus(`Saved ${provider} key`);
      setRows((prev) => ({
        ...prev,
        [provider]: {
          provider,
          hasKey: true,
          masked: maskKey(draft),
          editing: false,
          draft: undefined,
          saving: false,
        },
      }));
    } catch (e) {
      setError(String(e));
      setRows((prev) => ({ ...prev, [provider]: { ...prev[provider], saving: false } }));
    }
  };

  const reveal = async (provider: string) => {
    setError(null);
    try {
      const value = await invoke<string | null>("profile_api_key_get", { profileId, provider });
      if (!value) {
        setError(`No key stored for ${provider}`);
        return;
      }
      setRows((prev) => ({
        ...prev,
        [provider]: { ...prev[provider], revealed: value, masked: maskKey(value) },
      }));
    } catch (e) {
      setError(String(e));
    }
  };

  const hide = (provider: string) => {
    setRows((prev) => ({ ...prev, [provider]: { ...prev[provider], revealed: undefined } }));
  };

  const remove = async (provider: string) => {
    if (!window.confirm(`Delete the API key for "${provider}"?`)) return;
    setError(null);
    try {
      await invoke("profile_api_key_delete", { profileId, provider });
      setStatus(`Deleted ${provider} key`);
      setRows((prev) => ({
        ...prev,
        [provider]: { provider, hasKey: false },
      }));
    } catch (e) {
      setError(String(e));
    }
  };

  const list = useMemo(() => {
    const q = filter.trim().toLowerCase();
    return Object.values(rows)
      .filter((r) => !q || r.provider.toLowerCase().includes(q))
      .sort((a, b) => {
        if (a.hasKey !== b.hasKey) return a.hasKey ? -1 : 1;
        return a.provider.localeCompare(b.provider);
      });
  }, [rows, filter]);

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>API Keys</h3>
        <span
          style={{
            fontSize: "var(--font-size-xs)",
            color: "var(--text-secondary)",
            marginLeft: "var(--space-2)",
          }}
        >
          Profile: <code>{profileId}</code>
        </span>
        <div style={{ flex: 1 }} />
        <label htmlFor="keys-filter" style={{ position: "absolute", left: -10000, width: 1, height: 1, overflow: "hidden" }}>
          Filter providers
        </label>
        <input
          id="keys-filter"
          className="panel-input"
          placeholder="Filter providers…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          style={{
            padding: "var(--space-1) var(--space-2)",
            border: "1px solid var(--border-color)",
            borderRadius: "var(--radius-sm)",
            background: "var(--bg-secondary)",
            color: "var(--text-primary)",
            fontSize: "var(--font-size-base)",
          }}
        />
        <button
          className="panel-btn"
          aria-label="Refresh API keys"
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
      {status && !error && (
        <div
          role="status"
          style={{
            padding: "var(--space-2) var(--space-4)",
            background: "var(--success-bg)",
            color: "var(--success-color)",
            fontSize: "var(--font-size-base)",
          }}
        >
          {status}
        </div>
      )}

      <div className="panel-body">
        {list.length === 0 && !loading && (
          <div
            style={{
              padding: "var(--space-4)",
              textAlign: "center",
              color: "var(--text-secondary)",
              fontSize: "var(--font-size-base)",
            }}
          >
            No providers match <code>{filter}</code>.
          </div>
        )}
        {list.map((row) => (
          <KeyCard
            key={row.provider}
            row={row}
            onDraft={(draft) =>
              setRows((prev) => ({
                ...prev,
                [row.provider]: { ...prev[row.provider], draft },
              }))
            }
            onStart={() => startEdit(row.provider)}
            onCancel={() => cancelEdit(row.provider)}
            onSave={() => save(row.provider)}
            onReveal={() => reveal(row.provider)}
            onHide={() => hide(row.provider)}
            onRemove={() => remove(row.provider)}
          />
        ))}
      </div>
    </div>
  );
}

interface KeyCardProps {
  row: KeyRow;
  onDraft: (d: string) => void;
  onStart: () => void;
  onCancel: () => void;
  onSave: () => void;
  onReveal: () => void;
  onHide: () => void;
  onRemove: () => void;
}

function KeyCard({
  row,
  onDraft,
  onStart,
  onCancel,
  onSave,
  onReveal,
  onHide,
  onRemove,
}: KeyCardProps) {
  const inputId = `key-input-${row.provider}`;
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: "var(--space-3)",
        padding: "var(--space-3)",
        marginBottom: "var(--space-2)",
        border: "1px solid var(--border-color)",
        borderRadius: "var(--radius-sm)",
        background: "var(--bg-secondary)",
      }}
    >
      <div style={{ minWidth: 120, fontWeight: 600 }}>{row.provider}</div>
      <div
        style={{
          padding: "var(--space-1) var(--space-2)",
          borderRadius: "var(--radius-xs-plus)",
          fontSize: "var(--font-size-xs)",
          fontWeight: 600,
          background: row.hasKey ? "var(--success-bg)" : "var(--bg-tertiary)",
          color: row.hasKey ? "var(--success-color)" : "var(--text-secondary)",
        }}
      >
        {row.hasKey ? "set" : "unset"}
      </div>
      <div
        style={{
          flex: 1,
          fontFamily: "var(--font-mono, ui-monospace, monospace)",
          color: "var(--text-secondary)",
          fontSize: "var(--font-size-base)",
        }}
      >
        {row.editing ? (
          <>
            <label htmlFor={inputId} style={{ position: "absolute", left: -10000, width: 1, height: 1, overflow: "hidden" }}>
              API key for {row.provider}
            </label>
            <input
              id={inputId}
              type="password"
              autoFocus
              value={row.draft ?? ""}
              onChange={(e) => onDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") onSave();
                if (e.key === "Escape") onCancel();
              }}
              placeholder={`Paste ${row.provider} API key`}
              style={{
                width: "100%",
                padding: "var(--space-1) var(--space-2)",
                background: "var(--bg-primary)",
                color: "var(--text-primary)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-xs-plus)",
                fontSize: "var(--font-size-base)",
              }}
            />
          </>
        ) : row.revealed ? (
          <span>{row.revealed}</span>
        ) : row.hasKey ? (
          <span>{row.masked ?? "••••••••"}</span>
        ) : (
          <span style={{ fontStyle: "italic" }}>No key configured</span>
        )}
      </div>
      <div style={{ display: "flex", gap: "var(--space-1)" }}>
        {row.editing ? (
          <>
            <button
              className="panel-btn"
              onClick={onSave}
              disabled={!!row.saving}
              aria-label={`Save ${row.provider} key`}
            >
              {row.saving ? <Loader2 size={13} className="spin" /> : "Save"}
            </button>
            <button className="panel-btn" onClick={onCancel} aria-label={`Cancel editing ${row.provider} key`}>
              Cancel
            </button>
          </>
        ) : (
          <>
            {row.hasKey && (
              row.revealed ? (
                <button className="panel-btn" onClick={onHide} aria-label={`Hide ${row.provider} key`}>
                  Hide
                </button>
              ) : (
                <button className="panel-btn" onClick={onReveal} aria-label={`Reveal ${row.provider} key`}>
                  Reveal
                </button>
              )
            )}
            <button className="panel-btn" onClick={onStart} aria-label={`${row.hasKey ? "Edit" : "Add"} ${row.provider} key`}>
              {row.hasKey ? "Edit" : "Add"}
            </button>
            {row.hasKey && (
              <button
                className="panel-btn"
                onClick={onRemove}
                aria-label={`Delete ${row.provider} key`}
                style={{ color: "var(--error-color)" }}
              >
                Delete
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default KeysPanel;
