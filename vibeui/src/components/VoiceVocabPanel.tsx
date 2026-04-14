import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface VocabSymbol {
  name: string;
  kind: string;
  frequency: number;
  phonetic: string | null;
  file_path: string | null;
}

interface WhisperConfig {
  initial_prompt: string;
  hotwords: string[];
  language: string;
  model_size: string;
  temperature: number;
}

interface VocabMetrics {
  wer_before: number;
  wer_after: number;
  improvement_pct: number;
  total_utterances: number;
  hotword_hit_rate: number;
  last_evaluated_at: string | null;
}

const KIND_COLORS: Record<string, string> = {
  function: "#4a9eff",
  struct: "#9c6fe0",
  enum: "#f0a050",
  trait: "#4caf7d",
  variable: "#50c8e8",
  module: "#e85d8a",
  type: "#8b8b9e",
};

export function VoiceVocabPanel() {
  const [tab, setTab] = useState("vocab");
  const [symbols, setSymbols] = useState<VocabSymbol[]>([]);
  const [whisperConfig, setWhisperConfig] = useState<WhisperConfig | null>(null);
  const [metrics, setMetrics] = useState<VocabMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [building, setBuilding] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [vocabRes, statsRes, metricsRes] = await Promise.all([
          invoke<VocabSymbol[]>("voice_vocab_build"),
          invoke<WhisperConfig>("voice_vocab_stats"),
          invoke<VocabMetrics>("voice_vocab_metrics"),
        ]);
        setSymbols(Array.isArray(vocabRes) ? vocabRes : []);
        setWhisperConfig(statsRes ?? null);
        setMetrics(metricsRes ?? null);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function rebuildVocab() {
    setBuilding(true);
    try {
      const res = await invoke<VocabSymbol[]>("voice_vocab_build", { rebuild: true });
      setSymbols(Array.isArray(res) ? res : []);
    } catch (e) {
      setError(String(e));
    } finally {
      setBuilding(false);
    }
  }

  const maxFreq = Math.max(...symbols.map(s => s.frequency), 1);

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>Voice Vocab Injector</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["vocab", "inject", "metrics"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "vocab" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>{symbols.length} symbols indexed</span>
            <button onClick={rebuildVocab} disabled={building}
              style={{ padding: "4px 14px", borderRadius: "var(--radius-sm)", cursor: building ? "not-allowed" : "pointer", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", opacity: building ? 0.6 : 1 }}>
              {building ? "Building…" : "Rebuild"}
            </button>
          </div>
          <div style={{ overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
              <thead>
                <tr style={{ background: "var(--bg-secondary)" }}>
                  {["Symbol", "Kind", "Frequency", "Phonetic", "File"].map(h => (
                    <th key={h} style={{ padding: "6px 10px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {symbols.length === 0 && (
                  <tr><td colSpan={5} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No symbols found.</td></tr>
                )}
                {symbols.slice(0, 100).map((s, i) => {
                  const kindColor = KIND_COLORS[s.kind] ?? "var(--text-muted)";
                  return (
                    <tr key={i} style={{ borderBottom: "1px solid var(--border-color)" }}>
                      <td style={{ padding: "6px 10px" }}>
                        <code style={{ color: "var(--text-primary)" }}>{s.name}</code>
                      </td>
                      <td style={{ padding: "6px 10px" }}>
                        <span style={{ fontSize: "var(--font-size-sm)", padding: "1px 8px", borderRadius: "var(--radius-sm-alt)", background: kindColor + "22", color: kindColor }}>{s.kind}</span>
                      </td>
                      <td style={{ padding: "6px 10px" }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                          <div style={{ width: 50, height: 4, background: "var(--bg-primary)", borderRadius: 2 }}>
                            <div style={{ height: "100%", width: `${(s.frequency / maxFreq) * 100}%`, background: "var(--accent-color)", borderRadius: 2 }} />
                          </div>
                          <span style={{ color: "var(--text-muted)" }}>{s.frequency}</span>
                        </div>
                      </td>
                      <td style={{ padding: "6px 10px", color: "var(--text-muted)" }}>{s.phonetic ?? "—"}</td>
                      <td style={{ padding: "6px 10px", color: "var(--text-muted)", maxWidth: 160, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.file_path ?? "—"}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            {symbols.length > 100 && (
              <div style={{ padding: "8px 10px", fontSize: "var(--font-size-sm)", color: "var(--text-muted)", textAlign: "center" }}>
                Showing top 100 of {symbols.length} symbols
              </div>
            )}
          </div>
        </div>
      )}

      {!loading && tab === "inject" && (
        <div style={{ maxWidth: 560 }}>
          {!whisperConfig && <div style={{ color: "var(--text-muted)" }}>No Whisper config available.</div>}
          {whisperConfig && (
            <div>
              <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 16, marginBottom: 16 }}>
                <div style={{ display: "grid", gridTemplateColumns: "130px 1fr", rowGap: 10, fontSize: "var(--font-size-base)", marginBottom: 14 }}>
                  {[
                    ["Model Size", whisperConfig.model_size],
                    ["Language", whisperConfig.language],
                    ["Temperature", String(whisperConfig.temperature)],
                    ["Hotword Count", String(whisperConfig.hotwords.length)],
                  ].map(([label, value]) => (
                    <>
                      <span key={`l-${label}`} style={{ color: "var(--text-muted)" }}>{label}</span>
                      <span key={`v-${label}`} style={{ fontWeight: 600 }}>{value}</span>
                    </>
                  ))}
                </div>
                <div style={{ marginBottom: 10 }}>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Initial Prompt</div>
                  <div style={{ background: "var(--bg-primary)", borderRadius: "var(--radius-sm)", padding: "8px 12px", fontSize: "var(--font-size-base)", color: "var(--text-primary)", lineHeight: 1.5, maxHeight: 100, overflowY: "auto" }}>
                    {whisperConfig.initial_prompt || "(empty)"}
                  </div>
                </div>
                <div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Hotwords ({whisperConfig.hotwords.length})</div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 6, maxHeight: 120, overflowY: "auto" }}>
                    {whisperConfig.hotwords.slice(0, 50).map(hw => (
                      <code key={hw} style={{ fontSize: "var(--font-size-sm)", padding: "2px 8px", borderRadius: "var(--radius-sm)", background: "var(--accent-color)22", color: "var(--accent-color)", border: "1px solid var(--accent-color)44" }}>{hw}</code>
                    ))}
                    {whisperConfig.hotwords.length > 50 && (
                      <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", padding: "2px 8px" }}>+{whisperConfig.hotwords.length - 50} more</span>
                    )}
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {!loading && tab === "metrics" && (
        <div style={{ maxWidth: 500 }}>
          {!metrics && <div style={{ color: "var(--text-muted)" }}>No metrics available.</div>}
          {metrics && (
            <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12 }}>
                {[
                  { label: "WER Before", value: `${(metrics.wer_before * 100).toFixed(1)}%`, color: "var(--error-color)" },
                  { label: "WER After", value: `${(metrics.wer_after * 100).toFixed(1)}%`, color: "var(--warning-color)" },
                  { label: "Improvement", value: `${metrics.improvement_pct.toFixed(1)}%`, color: "var(--success-color)" },
                ].map(stat => (
                  <div key={stat.label} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: "14px 12px", textAlign: "center" }}>
                    <div style={{ fontSize: 22, fontWeight: 700, color: stat.color }}>{stat.value}</div>
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginTop: 4 }}>{stat.label}</div>
                  </div>
                ))}
              </div>
              <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 16 }}>
                <div style={{ display: "grid", gridTemplateColumns: "160px 1fr", rowGap: 10, fontSize: "var(--font-size-base)" }}>
                  {[
                    ["Total Utterances", String(metrics.total_utterances)],
                    ["Hotword Hit Rate", `${(metrics.hotword_hit_rate * 100).toFixed(1)}%`],
                    ["Last Evaluated", metrics.last_evaluated_at ?? "Never"],
                  ].map(([label, value]) => (
                    <>
                      <span key={`l-${label}`} style={{ color: "var(--text-muted)" }}>{label}</span>
                      <span key={`v-${label}`}>{value}</span>
                    </>
                  ))}
                </div>
              </div>
              <div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 8 }}>WER Improvement Visual</div>
                <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: "12px 16px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: 4 }}>
                    <span>Before: {(metrics.wer_before * 100).toFixed(1)}%</span>
                    <span>After: {(metrics.wer_after * 100).toFixed(1)}%</span>
                  </div>
                  <div style={{ position: "relative", height: 20, background: "var(--bg-primary)", borderRadius: "var(--radius-xs-plus)" }}>
                    <div style={{ position: "absolute", top: 0, left: 0, height: "100%", width: `${metrics.wer_before * 100}%`, background: "var(--error-color)44", borderRadius: "var(--radius-xs-plus)" }} />
                    <div style={{ position: "absolute", top: 0, left: 0, height: "100%", width: `${metrics.wer_after * 100}%`, background: "var(--warning-color)", borderRadius: "var(--radius-xs-plus)" }} />
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
