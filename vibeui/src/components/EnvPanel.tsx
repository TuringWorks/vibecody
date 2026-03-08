/**
 * EnvPanel — Environment & Secrets Manager.
 *
 * Features: .env file editor, per-environment switching (dev/staging/prod),
 * secret masking with reveal toggle, inline add/edit/delete, search filter.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12, height: "100%", overflowY: "auto" }}>
      {/* Environment selector */}
      <div>
        <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>Environment</div>
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
                  fontSize: 11,
                  fontWeight: activeEnv === env ? 600 : 400,
                }}
              >
                {env}{file ? ` (${file.var_count})` : ""}
              </button>
            );
          })}
          {/* Create new environment */}
          <div style={{ display: "flex", gap: 4 }}>
            <input
              type="text"
              value={newEnvName}
              onChange={(e) => setNewEnvName(e.target.value)}
              placeholder="new env"
              onKeyDown={(e) => e.key === "Enter" && handleCreateEnv()}
              style={{
                width: 80, padding: "3px 8px", fontSize: 11, background: "var(--bg-secondary)",
                border: "1px solid var(--border-color)", borderRadius: 4,
                color: "var(--text-primary)", outline: "none",
              }}
            />
            <button
              onClick={handleCreateEnv}
              disabled={!newEnvName.trim()}
              style={{
                padding: "3px 8px", fontSize: 11, background: "var(--bg-secondary)",
                border: "1px solid var(--border-color)", borderRadius: 4,
                color: "var(--text-primary)", cursor: "pointer",
              }}
            >
              +
            </button>
          </div>
        </div>
      </div>

      {/* File info */}
      <div style={{ fontSize: 11, opacity: 0.6, fontFamily: "monospace" }}>
        {envToFilename(activeEnv)} &middot; {entries.length} variable{entries.length !== 1 ? "s" : ""}
        {dirty && <span style={{ color: "var(--text-warning, #f9e2af)", marginLeft: 8 }}>Unsaved changes</span>}
      </div>

      {/* Search + Save */}
      <div style={{ display: "flex", gap: 8 }}>
        <input
          type="text"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter by key..."
          style={{
            flex: 1, padding: "6px 10px", fontSize: 12, background: "var(--bg-secondary)",
            border: "1px solid var(--border-color)", borderRadius: 4,
            color: "var(--text-primary)", outline: "none",
          }}
        />
        <button
          onClick={handleSave}
          disabled={!dirty || saving}
          style={{
            padding: "6px 14px", fontSize: 12, fontWeight: 600,
            background: dirty ? "var(--accent-color)" : "var(--bg-secondary)",
            color: dirty ? "var(--text-primary)" : "var(--text-primary)",
            border: "none", borderRadius: 4, cursor: dirty ? "pointer" : "default",
            opacity: dirty ? 1 : 0.5,
          }}
        >
          {saving ? "Saving..." : "Save"}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div style={{ background: "rgba(243,139,168,0.15)", border: "1px solid var(--error-color)", borderRadius: 6, padding: 8, fontSize: 11, color: "var(--text-danger, #f38ba8)" }}>
          {error}
        </div>
      )}

      {/* Variable list */}
      <div style={{ flex: 1, overflowY: "auto" }}>
        {filtered.length === 0 && !filter ? (
          <div style={{ textAlign: "center", padding: 24, opacity: 0.5, fontSize: 12 }}>
            No variables in {envToFilename(activeEnv)}. Add one below.
          </div>
        ) : filtered.length === 0 ? (
          <div style={{ textAlign: "center", padding: 24, opacity: 0.5, fontSize: 12 }}>
            No variables matching "{filter}"
          </div>
        ) : (
          filtered.map((entry) => (
            <div
              key={entry.key}
              style={{
                display: "flex", alignItems: "center", gap: 8, padding: "6px 0",
                borderBottom: "1px solid var(--border-color)", fontSize: 12,
              }}
            >
              {/* Key */}
              <span style={{ fontFamily: "monospace", fontWeight: 600, minWidth: 140, flexShrink: 0 }}>
                {entry.key}
              </span>
              {/* Value */}
              <input
                type={entry.is_secret && !revealedKeys.has(entry.key) ? "password" : "text"}
                value={entry.value}
                onChange={(e) => handleValueChange(entry.key, e.target.value)}
                style={{
                  flex: 1, padding: "3px 8px", fontSize: 11, fontFamily: "monospace",
                  background: "var(--bg-secondary)",
                  border: "1px solid var(--border-color)", borderRadius: 4,
                  color: "var(--text-primary)", outline: "none",
                }}
              />
              {/* Secret badge + reveal toggle */}
              {entry.is_secret && (
                <button
                  onClick={() => handleRevealToggle(entry.key)}
                  title={revealedKeys.has(entry.key) ? "Hide value" : "Reveal value"}
                  style={{
                    background: "none", border: "none", cursor: "pointer", fontSize: 13,
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
                style={{
                  background: "none", border: "none", cursor: "pointer", fontSize: 13,
                  color: "var(--text-danger, #f38ba8)", padding: "2px 4px", flexShrink: 0,
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
        <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>Add Variable</div>
        <div style={{ display: "flex", gap: 6 }}>
          <input
            type="text"
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
            placeholder="KEY_NAME"
            style={{
              width: 140, padding: "6px 8px", fontSize: 11, fontFamily: "monospace",
              background: "var(--bg-secondary)", textTransform: "uppercase",
              border: "1px solid var(--border-color)", borderRadius: 4,
              color: "var(--text-primary)", outline: "none",
            }}
          />
          <input
            type="text"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="value"
            onKeyDown={(e) => e.key === "Enter" && handleAddVar()}
            style={{
              flex: 1, padding: "6px 8px", fontSize: 11, fontFamily: "monospace",
              background: "var(--bg-secondary)",
              border: "1px solid var(--border-color)", borderRadius: 4,
              color: "var(--text-primary)", outline: "none",
            }}
          />
          <button
            onClick={handleAddVar}
            disabled={!newKey.trim() || !newValue.trim()}
            style={{
              padding: "6px 14px", fontSize: 12, fontWeight: 600,
              background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: 4,
              cursor: "pointer", whiteSpace: "nowrap",
            }}
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}
