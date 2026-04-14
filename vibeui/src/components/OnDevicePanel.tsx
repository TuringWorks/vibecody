import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LocalModel {
  id: string;
  name: string;
  format: string;
  quant: string;
  size_mb: number;
  downloaded: boolean;
  path: string | null;
}

interface HardwareInfo {
  backend_name: string;
  vram_mb: number | null;
  estimated_tps: number | null;
  cpu_threads: number;
  ram_mb: number;
  gpu_name: string | null;
}

interface BenchmarkResult {
  model_id: string;
  model_name: string;
  tokens_per_second: number;
  time_to_first_token_ms: number;
  total_tokens: number;
  ran_at: string;
}

interface PrivacyConfig {
  local_only: boolean;
  blocked_providers: string[];
}

export function OnDevicePanel() {
  const [tab, setTab] = useState("models");
  const [models, setModels] = useState<LocalModel[]>([]);
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [benchResults, setBenchResults] = useState<BenchmarkResult[]>([]);
  const [privacy, setPrivacy] = useState<PrivacyConfig>({ local_only: false, blocked_providers: [] });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedModel, setSelectedModel] = useState("");
  const [benchRunning, setBenchRunning] = useState(false);
  const [downloading, setDownloading] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState<Set<string>>(new Set());

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [modelsRes, hwRes, benchRes, privRes] = await Promise.all([
          invoke<LocalModel[]>("on_device_list"),
          invoke<HardwareInfo>("on_device_hardware"),
          invoke<BenchmarkResult[]>("on_device_benchmark"),
          invoke<PrivacyConfig>("on_device_enforce"),
        ]);
        const ms = Array.isArray(modelsRes) ? modelsRes : [];
        setModels(ms);
        setHardware(hwRes ?? null);
        setBenchResults(Array.isArray(benchRes) ? benchRes : []);
        setPrivacy(privRes ?? { local_only: false, blocked_providers: [] });
        if (ms.length > 0) setSelectedModel(ms[0].id);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function downloadModel(id: string) {
    setDownloading(prev => new Set([...prev, id]));
    try {
      await invoke("on_device_list", { action: "download", modelId: id });
      const res = await invoke<LocalModel[]>("on_device_list");
      setModels(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(String(e));
    } finally {
      setDownloading(prev => { const n = new Set(prev); n.delete(id); return n; });
    }
  }

  async function deleteModel(id: string) {
    setDeleting(prev => new Set([...prev, id]));
    try {
      await invoke("on_device_list", { action: "delete", modelId: id });
      const res = await invoke<LocalModel[]>("on_device_list");
      setModels(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(String(e));
    } finally {
      setDeleting(prev => { const n = new Set(prev); n.delete(id); return n; });
    }
  }

  async function runBenchmark() {
    if (!selectedModel) return;
    setBenchRunning(true);
    try {
      const res = await invoke<BenchmarkResult[]>("on_device_benchmark", { modelId: selectedModel });
      setBenchResults(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(String(e));
    } finally {
      setBenchRunning(false);
    }
  }

  async function toggleLocalOnly(val: boolean) {
    setPrivacy(p => ({ ...p, local_only: val }));
    try {
      await invoke("on_device_enforce", { localOnly: val, blockedProviders: privacy.blocked_providers });
    } catch (e) {
      setError(String(e));
    }
  }

  const formatMb = (mb: number) => mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`;

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>On-Device Models</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap" }}>
        {["models", "hardware", "benchmark", "privacy"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "models" && (
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)" }}>
                {["Name", "Format", "Quant", "Size", "Actions"].map(h => (
                  <th key={h} style={{ padding: "6px 10px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {models.length === 0 && (
                <tr><td colSpan={5} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No models available.</td></tr>
              )}
              {models.map(m => (
                <tr key={m.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "6px 10px", fontWeight: 600 }}>{m.name}</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)" }}>{m.format}</td>
                  <td style={{ padding: "6px 10px" }}>
                    <span style={{ padding: "1px 7px", borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-sm)", background: "var(--accent-color)22", color: "var(--accent-color)" }}>{m.quant}</span>
                  </td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{formatMb(m.size_mb)}</td>
                  <td style={{ padding: "6px 10px" }}>
                    <div style={{ display: "flex", gap: 6 }}>
                      {!m.downloaded ? (
                        <button onClick={() => downloadModel(m.id)} disabled={downloading.has(m.id)}
                          style={{ padding: "3px 10px", borderRadius: 5, cursor: downloading.has(m.id) ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-sm)", opacity: downloading.has(m.id) ? 0.6 : 1 }}>
                          {downloading.has(m.id) ? "Downloading…" : "Download"}
                        </button>
                      ) : (
                        <button onClick={() => deleteModel(m.id)} disabled={deleting.has(m.id)}
                          style={{ padding: "3px 10px", borderRadius: 5, cursor: deleting.has(m.id) ? "not-allowed" : "pointer", background: "var(--error-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-sm)", opacity: deleting.has(m.id) ? 0.6 : 1 }}>
                          {deleting.has(m.id) ? "Deleting…" : "Delete"}
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!loading && tab === "hardware" && (
        <div style={{ maxWidth: 480 }}>
          {!hardware && <div style={{ color: "var(--text-muted)" }}>No hardware info available.</div>}
          {hardware && (
            <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 18 }}>
              <div style={{ display: "grid", gridTemplateColumns: "150px 1fr", rowGap: 12, fontSize: "var(--font-size-md)" }}>
                {[
                  ["Backend", hardware.backend_name],
                  ["GPU", hardware.gpu_name ?? "None"],
                  ["VRAM", hardware.vram_mb ? formatMb(hardware.vram_mb) : "N/A"],
                  ["Est. TPS", hardware.estimated_tps ? `${hardware.estimated_tps.toFixed(1)} tok/s` : "N/A"],
                  ["CPU Threads", String(hardware.cpu_threads)],
                  ["RAM", formatMb(hardware.ram_mb)],
                ].map(([label, value]) => (
                  <>
                    <span key={`l-${label}`} style={{ color: "var(--text-muted)", fontSize: "var(--font-size-base)" }}>{label}</span>
                    <span key={`v-${label}`} style={{ fontWeight: 600 }}>{value}</span>
                  </>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {!loading && tab === "benchmark" && (
        <div>
          <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 16, flexWrap: "wrap" }}>
            <select value={selectedModel} onChange={e => setSelectedModel(e.target.value)}
              style={{ padding: "6px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", flex: 1, minWidth: 200 }}>
              {models.filter(m => m.downloaded).map(m => (
                <option key={m.id} value={m.id}>{m.name}</option>
              ))}
              {models.filter(m => m.downloaded).length === 0 && <option value="">No downloaded models</option>}
            </select>
            <button onClick={runBenchmark} disabled={benchRunning || !selectedModel}
              style={{ padding: "6px 18px", borderRadius: "var(--radius-sm)", cursor: benchRunning || !selectedModel ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-base)", fontWeight: 600, opacity: benchRunning || !selectedModel ? 0.6 : 1 }}>
              {benchRunning ? "Running…" : "Run Benchmark"}
            </button>
          </div>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)" }}>
                {["Model", "TPS", "TTFT (ms)", "Tokens", "Run At"].map(h => (
                  <th key={h} style={{ padding: "6px 10px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {benchResults.length === 0 && (
                <tr><td colSpan={5} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No benchmark results yet.</td></tr>
              )}
              {benchResults.map((r, i) => (
                <tr key={i} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "6px 10px", fontWeight: 600 }}>{r.model_name}</td>
                  <td style={{ padding: "6px 10px", color: "var(--success-color)" }}>{r.tokens_per_second.toFixed(1)}</td>
                  <td style={{ padding: "6px 10px" }}>{r.time_to_first_token_ms}</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)" }}>{r.total_tokens}</td>
                  <td style={{ padding: "6px 10px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>{r.ran_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!loading && tab === "privacy" && (
        <div style={{ maxWidth: 480 }}>
          <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 18, marginBottom: 16 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 16 }}>
              <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
                <input type="checkbox" checked={privacy.local_only} onChange={e => toggleLocalOnly(e.target.checked)} />
                <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Local-Only Mode</span>
              </label>
              {privacy.local_only && (
                <span style={{ fontSize: "var(--font-size-sm)", padding: "2px 8px", borderRadius: "var(--radius-sm-alt)", background: "var(--success-color)22", color: "var(--success-color)" }}>Active</span>
              )}
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 16 }}>
              When enabled, no data is sent to external providers. Only on-device models are available.
            </div>
            <div>
              <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 8 }}>Blocked Providers</div>
              {privacy.blocked_providers.length === 0 && <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>No providers blocked.</div>}
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {privacy.blocked_providers.map(p => (
                  <span key={p} style={{ padding: "3px 10px", borderRadius: 12, fontSize: "var(--font-size-sm)", background: "var(--error-color)22", color: "var(--error-color)", border: "1px solid var(--error-color)" }}>{p}</span>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
