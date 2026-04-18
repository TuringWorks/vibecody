/**
 * EnvPanel — Environment & Secrets Manager.
 *
 * Features: .env file editor, per-environment switching (dev/staging/prod),
 * secret masking with reveal toggle, inline add/edit/delete, search filter.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";

interface EnvFileInfo {
  filename: string;
  environment: string;
  var_count: number;
  last_modified: number;
}

interface EnvEntry {
  key: string;
  value: string;
  is_secret: boolean;
  comment: string | null;
}

interface EnvPanelProps {
  workspacePath: string | null;
}

export function EnvPanel({ workspacePath }: EnvPanelProps) {
  const [envFiles, setEnvFiles] = useState<EnvFileInfo[]>([]);
  const [activeEnv, setActiveEnv] = useState("default");
  const [entries, setEntries] = useState<EnvEntry[]>([]);
  const [revealedKeys, setRevealedKeys] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState("");
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newEnvName, setNewEnvName] = useState("");

  const envToFilename = useCallback((env: string) => {
    return env === "default" ? ".env" : `.env.${env}`;
  }, []);

  const loadFiles = useCallback(async () => {
    if (!workspacePath) return;
    try {
      const files = await invoke<EnvFileInfo[]>("get_env_files", { workspace: workspacePath });
      setEnvFiles(files);
    } catch {
      setEnvFiles([]);
    }
  }, [workspacePath]);

  const loadEntries = useCallback(async (env: string) => {
    if (!workspacePath) return;
    const filename = env === "default" ? ".env" : `.env.${env}`;
    try {
      const result = await invoke<EnvEntry[]>("read_env_file", {
        workspace: workspacePath,
        filename,
        reveal: false,
      });
      setEntries(result);
      setDirty(false);
      setError(null);
    } catch {
      setEntries([]);
      setDirty(false);
    }
  }, [workspacePath]);

  useEffect(() => {
    loadFiles();
    loadEntries(activeEnv);
  }, [workspacePath, loadFiles, loadEntries, activeEnv]);

  if (!workspacePath) {
    return (
      <div style={{ padding: 16, opacity: 0.6, textAlign: "center" }}>
        <p>Open a workspace folder to manage environment variables.</p>
      </div>
    );
  }

  const handleSwitchEnv = (env: string) => {
    if (dirty && !confirm("Unsaved changes will be lost. Switch environment?")) return;
    setActiveEnv(env);
    setRevealedKeys(new Set());
    setFilter("");
    loadEntries(env);
  };

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      // Need real values for secrets — re-read with reveal=true, merge edits
      const revealed = await invoke<EnvEntry[]>("read_env_file", {
        workspace: workspacePath,
        filename: envToFilename(activeEnv),
        reveal: true,
      });
      // Merge: for each entry in our state, if it was a secret and not revealed in UI, use the original value
      const toSave = entries.map((e) => {
        if (e.is_secret && !revealedKeys.has(e.key) && e.value.includes("\u2022")) {
          const orig = revealed.find((r) => r.key === e.key);
          return orig ? { ...e, value: orig.value } : e;
        }
        return e;
      });
      await invoke("save_env_file", {
        workspace: workspacePath,
        filename: envToFilename(activeEnv),
        entries: toSave,
      });
      setDirty(false);
      await loadFiles();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleRevealToggle = async (key: string) => {
    if (revealedKeys.has(key)) {
      setRevealedKeys((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
      // Re-mask the value
      setEntries((prev) =>
        prev.map((e) =>
          e.key === key && e.is_secret
            ? { ...e, value: "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022" }
            : e
        )
      );
    } else {
      // Fetch real value
      try {
        const revealed = await invoke<EnvEntry[]>("read_env_file", {
          workspace: workspacePath,
          filename: envToFilename(activeEnv),
          reveal: true,
        });
        const real = revealed.find((e) => e.key === key);
        if (real) {
          setRevealedKeys((prev) => new Set(prev).add(key));
          setEntries((prev) =>
            prev.map((e) => (e.key === key ? { ...e, value: real.value } : e))
          );
        }
      } catch {
        // ignore
      }
    }
  };

  const handleAddVar = () => {
    const trimmedKey = newKey.trim().toUpperCase();
    if (!trimmedKey || !newValue.trim()) return;
    if (entries.some((e) => e.key === trimmedKey)) {
      setError(`Key "${trimmedKey}" already exists`);
      return;
    }
    const entry: EnvEntry = {
      key: trimmedKey,
      value: newValue.trim(),
      is_secret: false, // User just typed it, show it
      comment: null,
    };
    setEntries((prev) => [...prev, entry]);
    setNewKey("");
    setNewValue("");
    setDirty(true);
    setError(null);
  };

  const handleDeleteVar = async (key: string) => {
    try {
      await invoke("delete_env_var", {
        workspace: workspacePath,
        filename: envToFilename(activeEnv),
        key,
      });
      setEntries((prev) => prev.filter((e) => e.key !== key));
      await loadFiles();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleValueChange = (key: string, newVal: string) => {
    setEntries((prev) =>
      prev.map((e) => (e.key === key ? { ...e, value: newVal } : e))
    );
    setDirty(true);
  };

  const handleCreateEnv = async () => {
    const name = newEnvName.trim().toLowerCase();
    if (!name) return;
    const filename = `.env.${name}`;
    try {
      await invoke("save_env_file", {
        workspace: workspacePath,
        filename,
        entries: [],
      });
      setNewEnvName("");
      await loadFiles();
      handleSwitchEnv(name);
    } catch (e) {
      setError(String(e));
    }
  };

  const environments = [...new Set(["default", ...envFiles.map((f) => f.environment)])];
  const filtered = filter
    ? entries.filter((e) => e.key.toLowerCase().includes(filter.toLowerCase()))
    : entries;

  return (
    <div className="panel-container">
    <div className="panel-body" style={{ overflowY: "auto", display: "flex", flexDirection: "column", gap: 12 }}>
      {/* Environment selector */}
      <div>
        <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 6 }}>Environment</div>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", alignItems: "center" }}>
          {environments.map((env) => {
            const file = envFiles.find((f) => f.environment === env);
            return (
              <button
                key={env}
                onClick={() => handleSwitchEnv(env)}
                style={{
                  background: activeEnv === env ? "var(--accent-color)" : "var(--bg-secondary)",
                  border: `1px solid ${activeEnv === env ? "var(--accent-color)" : "var(--border-color)"}`,
                  borderRadius: 12,
                  padding: "4px 12px",
                  cursor: "pointer",
                  color: "var(--text-primary)",
                  fontSize: "var(--font-size-sm)",
                  fontWeight: activeEnv === env ? 600 : 400,
                }}
              >
                {env}{file ? ` (${file.var_count})` : ""}
              </button>
            );
          })}
          {/* Create new environment */}
          <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
            <label htmlFor="new-env-name" style={{ position: "absolute", width: 1, height: 1, overflow: "hidden", clip: "rect(0,0,0,0)", whiteSpace: "nowrap" }}>
              New environment name
            </label>
            <input
              id="new-env-name"
              type="text"
              value={newEnvName}
              onChange={(e) => setNewEnvName(e.target.value)}
              placeholder="new env"
              onKeyDown={(e) => e.key === "Enter" && handleCreateEnv()}
              className="panel-input"
              style={{ width: 80 }}
            />
            <button
              onClick={handleCreateEnv}
              disabled={!newEnvName.trim()}
              aria-label="Create new environment"
              className="panel-btn panel-btn-secondary panel-btn-xs"
            >
              +
            </button>
          </div>
        </div>
      </div>

      {/* File info */}
      <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.6, fontFamily: "var(--font-mono)" }}>
        {envToFilename(activeEnv)} &middot; {entries.length} variable{entries.length !== 1 ? "s" : ""}
        {dirty && <span style={{ color: "var(--text-warning)", marginLeft: 8 }}>Unsaved changes</span>}
      </div>

      {/* Search + Save */}
      <div style={{ display: "flex", gap: 8 }}>
        <label htmlFor="env-filter" style={{ position: "absolute", width: 1, height: 1, overflow: "hidden", clip: "rect(0,0,0,0)", whiteSpace: "nowrap" }}>
          Filter by key
        </label>
        <input
          id="env-filter"
          type="text"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter by key..."
          className="panel-input panel-input-full"
          style={{ flex: 1 }}
        />
        <button
          onClick={handleSave}
          disabled={!dirty || saving}
          className="panel-btn panel-btn-primary panel-btn-sm"
        >
          {saving ? <><Loader2 size={13} className="spin" /> Saving...</> : "Save"}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="panel-error">
          {error}
        </div>
      )}

      {/* Variable list */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {filtered.length === 0 && !filter ? (
          <div className="panel-empty">
            No variables in {envToFilename(activeEnv)}. Add one below.
          </div>
        ) : filtered.length === 0 ? (
          <div className="panel-empty">
            No variables matching &ldquo;{filter}&rdquo;
          </div>
        ) : (
          filtered.map((entry) => (
            <div
              key={entry.key}
              style={{
                display: "flex", alignItems: "center", gap: 8, padding: "8px 0",
                borderBottom: "1px solid var(--border-color)", fontSize: "var(--font-size-base)",
              }}
            >
              {/* Key */}
              <span style={{ fontFamily: "var(--font-mono)", fontWeight: 600, minWidth: 140, flexShrink: 0 }}>
                {entry.key}
              </span>
              {/* Value */}
              <input
                type={entry.is_secret && !revealedKeys.has(entry.key) ? "password" : "text"}
                value={entry.value}
                onChange={(e) => handleValueChange(entry.key, e.target.value)}
                aria-label={`Value for ${entry.key}`}
                className="panel-input panel-input-full"
                style={{ flex: 1, fontFamily: "var(--font-mono)" }}
              />
              {/* Secret badge + reveal toggle */}
              {entry.is_secret && (
                <button
                  onClick={() => handleRevealToggle(entry.key)}
                  title={revealedKeys.has(entry.key) ? "Hide value" : "Reveal value"}
                  aria-label={revealedKeys.has(entry.key) ? "Hide secret value" : "Reveal secret value"}
                  role="switch"
                  aria-checked={revealedKeys.has(entry.key)}
                  style={{
                    background: "none", border: "none", cursor: "pointer", fontSize: "var(--font-size-md)",
                    color: "var(--text-primary)", padding: "2px 4px", flexShrink: 0,
                  }}
                >
                  {revealedKeys.has(entry.key) ? "Hide" : "Show"}
                </button>
              )}
              {/* Delete */}
              <button
                onClick={() => handleDeleteVar(entry.key)}
                title="Delete variable"
                aria-label="Delete environment variable"
                style={{
                  background: "none", border: "none", cursor: "pointer", fontSize: "var(--font-size-md)",
                  color: "var(--text-danger)", padding: "2px 4px", flexShrink: 0,
                }}
              >
                ✕
              </button>
            </div>
          ))
        )}
      </div>

      {/* Add new variable */}
      <div style={{ borderTop: "1px solid var(--border-color)", paddingTop: 10 }}>
        <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 6 }}>Add Variable</div>
        <div style={{ display: "flex", gap: 6 }}>
          <label htmlFor="new-var-key" style={{ position: "absolute", width: 1, height: 1, overflow: "hidden", clip: "rect(0,0,0,0)", whiteSpace: "nowrap" }}>
            Variable key name
          </label>
          <input
            id="new-var-key"
            type="text"
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
            placeholder="KEY_NAME"
            className="panel-input"
            style={{ width: 140, fontFamily: "var(--font-mono)", textTransform: "uppercase" }}
          />
          <label htmlFor="new-var-value" style={{ position: "absolute", width: 1, height: 1, overflow: "hidden", clip: "rect(0,0,0,0)", whiteSpace: "nowrap" }}>
            Variable value
          </label>
          <input
            id="new-var-value"
            type="text"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="value"
            onKeyDown={(e) => e.key === "Enter" && handleAddVar()}
            className="panel-input panel-input-full"
            style={{ flex: 1, fontFamily: "var(--font-mono)" }}
          />
          <button
            onClick={handleAddVar}
            disabled={!newKey.trim() || !newValue.trim()}
            className="panel-btn panel-btn-primary panel-btn-sm"
            style={{ whiteSpace: "nowrap" }}
          >
            Add
          </button>
        </div>
      </div>
    </div>
    </div>
  );
}
