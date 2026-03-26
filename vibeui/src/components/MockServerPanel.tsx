/**
 * MockServerPanel — API Mock Server.
 *
 * Start/stop a local mock HTTP server, define routes, view request log,
 * and import mock routes from OpenAPI specs via AI.
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface MockRoute {
  id: string;
  method: string;
  path: string;
  status: number;
  body: string;
  headers: string;
  delay_ms: number;
}

interface MockRequest {
  timestamp: number;
  method: string;
  path: string;
  headers: string;
  body: string;
  matched_route_id: string | null;
}

type SubTab = "routes" | "log" | "import";

const METHOD_OPTIONS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

const methodColor: Record<string, string> = {
  GET: "var(--success-color)",
  POST: "var(--accent-color)",
  PUT: "var(--warning-color)",
  DELETE: "var(--error-color)",
  PATCH: "var(--text-accent)",
  HEAD: "var(--text-secondary)",
  OPTIONS: "var(--text-secondary)",
};

export function MockServerPanel() {
  const [tab, setTab] = useState<SubTab>("routes");
  const [port, setPort] = useState("3001");
  const [running, setRunning] = useState(false);
  const [routes, setRoutes] = useState<MockRoute[]>([]);
  const [requestLog, setRequestLog] = useState<MockRequest[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Add route form
  const [addMethod, setAddMethod] = useState("GET");
  const [addPath, setAddPath] = useState("");
  const [addStatus, setAddStatus] = useState("200");
  const [addBody, setAddBody] = useState('{"message":"ok"}');

  // Import
  const [specPath, setSpecPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [importResult, setImportResult] = useState<MockRoute[]>([]);

  const loadRoutes = async () => {
    try {
      const r = await invoke<MockRoute[]>("list_mock_routes");
      setRoutes(r);
    } catch (_) { /* ignore */ }
  };

  const loadLog = async () => {
    try {
      const l = await invoke<MockRequest[]>("get_mock_request_log");
      setRequestLog(l);
    } catch (_) { /* ignore */ }
  };

  useEffect(() => { loadRoutes(); }, []);

  // Poll request log when running
  useEffect(() => {
    if (running && tab === "log") {
      pollRef.current = setInterval(loadLog, 2000);
      loadLog();
    }
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [running, tab]);

  const handleStart = async () => {
    setLoading(true);
    setError(null);
    try {
      const msg = await invoke<string>("start_mock_server", { port: parseInt(port, 10) });
      setRunning(true);
      setError(null);
      console.log(msg);
    } catch (e: unknown) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleStop = async () => {
    try {
      await invoke("stop_mock_server");
      setRunning(false);
      setRequestLog([]);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleAddRoute = async () => {
    if (!addPath.trim()) { setError("Path is required"); return; }
    setError(null);
    try {
      await invoke("add_mock_route", {
        method: addMethod,
        path: addPath.startsWith("/") ? addPath : `/${addPath}`,
        status: parseInt(addStatus, 10) || 200,
        body: addBody,
        headers: "",
      });
      setAddPath("");
      await loadRoutes();
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleRemoveRoute = async (id: string) => {
    try {
      await invoke("remove_mock_route", { id });
      await loadRoutes();
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleImport = async () => {
    if (!specPath.trim()) { setError("Spec path is required"); return; }
    setImporting(true);
    setError(null);
    try {
      const result = await invoke<MockRoute[]>("generate_mocks_from_spec", { specPath });
      setImportResult(result);
      await loadRoutes();
    } catch (e: unknown) {
      setError(String(e));
    }
    setImporting(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Server controls */}
      <div style={{
        display: "flex", gap: 6, padding: "8px 12px", alignItems: "center",
        borderBottom: "1px solid var(--border-color)",
      }}>
        <span style={{ fontSize: 11, fontWeight: 600 }}>Port:</span>
        <input
          value={port}
          onChange={(e) => setPort(e.target.value)}
          disabled={running}
          style={{ ...inputStyle, width: 60, textAlign: "center" }}
        />
        {!running ? (
          <button onClick={handleStart} disabled={loading} style={{ ...btnStyle, background: "var(--success-color)", color: "var(--bg-tertiary)" }}>
            {loading ? "..." : "Start"}
          </button>
        ) : (
          <button onClick={handleStop} style={{ ...btnStyle, background: "var(--error-color)", color: "var(--bg-tertiary)" }}>
            Stop
          </button>
        )}
        <span style={{
          fontSize: 10, fontWeight: 600, padding: "2px 8px", borderRadius: 10,
          background: running ? "color-mix(in srgb, var(--accent-green) 15%, transparent)" : "color-mix(in srgb, var(--text-secondary) 15%, transparent)",
          color: running ? "var(--success-color)" : "var(--text-secondary)",
        }}>
          {running ? `Running :${port}` : "Stopped"}
        </span>
        <div style={{ flex: 1 }} />
        <span style={{ fontSize: 10, opacity: 0.5 }}>{routes.length} routes</span>
      </div>

      {/* Sub-tabs */}
      <div style={{ display: "flex", gap: 4, padding: "6px 12px", borderBottom: "1px solid var(--border-color)" }}>
        {(["routes", "log", "import"] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            style={{
              padding: "3px 10px", fontSize: 10, fontWeight: 600, borderRadius: 4, cursor: "pointer",
              border: tab === t ? "1px solid var(--accent-color)" : "1px solid var(--border-color)",
              background: tab === t ? "color-mix(in srgb, var(--accent-blue) 15%, transparent)" : "transparent",
              color: "var(--text-primary)",
            }}
          >
            {t === "routes" ? "Routes" : t === "log" ? "Request Log" : "Import"}
          </button>
        ))}
      </div>

      {error && (
        <div style={{ padding: "6px 12px", fontSize: 11, color: "var(--text-danger)", background: "color-mix(in srgb, var(--accent-rose) 5%, transparent)" }}>
          {error}
        </div>
      )}

      <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px" }}>
        {/* Routes tab */}
        {tab === "routes" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {/* Add route form */}
            <div style={{
              display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap",
              padding: "6px 8px", borderRadius: 4, background: "var(--bg-primary)",
            }}>
              <select value={addMethod} onChange={(e) => setAddMethod(e.target.value)} style={selectStyle}>
                {METHOD_OPTIONS.map((m) => <option key={m} value={m}>{m}</option>)}
              </select>
              <input placeholder="/api/path" value={addPath} onChange={(e) => setAddPath(e.target.value)} style={{ ...inputStyle, flex: 1, minWidth: 120 }} />
              <input placeholder="200" value={addStatus} onChange={(e) => setAddStatus(e.target.value)} style={{ ...inputStyle, width: 50, textAlign: "center" }} />
              <button onClick={handleAddRoute} style={{ ...btnStyle, background: "var(--accent-color)", color: "var(--text-primary)" }}>Add</button>
            </div>
            <input
              placeholder='Response body JSON...'
              value={addBody}
              onChange={(e) => setAddBody(e.target.value)}
              style={{ ...inputStyle, fontFamily: "var(--font-mono)", fontSize: 10 }}
            />

            {/* Route list */}
            {routes.map((r) => (
              <div key={r.id} style={{
                display: "flex", gap: 8, alignItems: "center", padding: "4px 6px",
                borderBottom: "1px solid var(--border-color)", fontSize: 11,
              }}>
                <span style={{
                  padding: "1px 6px", borderRadius: 3, fontWeight: 700, fontSize: 10,
                  color: "var(--bg-tertiary)", background: methodColor[r.method] || "var(--text-secondary)",
                }}>
                  {r.method}
                </span>
                <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-info)" }}>{r.path}</span>
                <span style={{ fontSize: 10, opacity: 0.5 }}>{r.status}</span>
                <div style={{ flex: 1 }} />
                <span style={{ fontSize: 10, opacity: 0.5, maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {r.body.substring(0, 60)}
                </span>
                <button onClick={() => handleRemoveRoute(r.id)} style={{ ...cellBtn, color: "var(--text-danger)" }}>✕</button>
              </div>
            ))}
            {routes.length === 0 && (
              <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
                No routes defined. Add one above or import from OpenAPI.
              </div>
            )}
          </div>
        )}

        {/* Log tab */}
        {tab === "log" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {!running && (
              <div style={{ padding: 12, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
                Start the mock server to capture requests.
              </div>
            )}
            {requestLog.length === 0 && running && (
              <div style={{ padding: 12, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
                Waiting for requests... (auto-refreshes every 2s)
              </div>
            )}
            {requestLog.map((r, i) => (
              <div key={i} style={{
                display: "flex", gap: 8, alignItems: "center", padding: "4px 6px",
                borderBottom: "1px solid var(--border-color)", fontSize: 11,
              }}>
                <span style={{ fontSize: 9, opacity: 0.4, fontFamily: "var(--font-mono)" }}>
                  {new Date(r.timestamp).toLocaleTimeString()}
                </span>
                <span style={{
                  padding: "1px 5px", borderRadius: 3, fontWeight: 700, fontSize: 9,
                  color: "var(--bg-tertiary)", background: methodColor[r.method] || "var(--text-secondary)",
                }}>
                  {r.method}
                </span>
                <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-info)" }}>{r.path}</span>
                <div style={{ flex: 1 }} />
                <span style={{
                  fontSize: 9, padding: "1px 6px", borderRadius: 3,
                  background: r.matched_route_id ? "color-mix(in srgb, var(--accent-green) 15%, transparent)" : "color-mix(in srgb, var(--accent-rose) 15%, transparent)",
                  color: r.matched_route_id ? "var(--success-color)" : "var(--error-color)",
                }}>
                  {r.matched_route_id ? "matched" : "no match"}
                </span>
              </div>
            ))}
          </div>
        )}

        {/* Import tab */}
        {tab === "import" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{ fontSize: 11, opacity: 0.7 }}>
              Import mock routes from an OpenAPI/Swagger spec file. AI will parse the spec and generate routes.
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              <input
                placeholder="Path to OpenAPI spec (JSON/YAML)..."
                value={specPath}
                onChange={(e) => setSpecPath(e.target.value)}
                style={{ ...inputStyle, flex: 1, fontFamily: "var(--font-mono)" }}
              />
              <button onClick={handleImport} disabled={importing} style={{ ...btnStyle, color: "var(--text-info)" }}>
                {importing ? "Importing..." : "Generate Mocks"}
              </button>
            </div>
            {importResult.length > 0 && (
              <div>
                <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 4 }}>
                  Generated {importResult.length} routes:
                </div>
                {importResult.map((r) => (
                  <div key={r.id} style={{ fontSize: 10, padding: "2px 0", fontFamily: "var(--font-mono)" }}>
                    <span style={{ color: methodColor[r.method] || "var(--text-secondary)" }}>{r.method}</span>{" "}
                    <span style={{ color: "var(--text-info)" }}>{r.path}</span>{" "}
                    <span style={{ opacity: 0.5 }}>{r.status}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  padding: "4px 10px", fontSize: 11, fontWeight: 600,
  border: "1px solid var(--border-color)", borderRadius: 4,
  background: "var(--bg-secondary)", color: "var(--text-primary)",
  cursor: "pointer",
};

const inputStyle: React.CSSProperties = {
  padding: "4px 8px", fontSize: 11, borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  outline: "none",
};

const selectStyle: React.CSSProperties = {
  padding: "4px 6px", fontSize: 11, borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)",
};

const cellBtn: React.CSSProperties = {
  background: "none", border: "none", cursor: "pointer",
  fontSize: 12, padding: "0 3px", color: "var(--text-primary)", opacity: 0.7,
};
