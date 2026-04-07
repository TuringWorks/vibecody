import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface VoiceModel {
  name: string;
  size: string;
  downloaded: boolean;
  selected: boolean;
}

interface HistoryEntry {
  text: string;
  time: string;
  confidence: number;
}

export function VoiceLocalPanel() {
  const [tab, setTab] = useState("record");
  const [recording, setRecording] = useState(false);
  const [transcription, setTranscription] = useState("");
  const [confidence, setConfidence] = useState(0);
  const [language, setLanguage] = useState("en");
  const [vad, setVad] = useState(true);
  const [sampleRate, setSampleRate] = useState("16000");
  const [models, setModels] = useState<VoiceModel[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [loadingModels, setLoadingModels] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);

  useEffect(() => {
    async function loadModels() {
      setLoadingModels(true);
      try {
        const modelList = await invoke<VoiceModel[]>("voice_list_models");
        setModels(modelList);
      } catch (e) {
        console.error("Failed to load voice models:", e);
      }
      setLoadingModels(false);
    }
    loadModels();
  }, []);

  const toggleRecording = useCallback(async () => {
    if (!recording) {
      setRecording(true);
      setTranscription("Listening...");
      setConfidence(0);
      setActionLoading(true);
      try {
        await invoke("voice_start_recording");
      } catch (e) {
        console.error("Failed to start recording:", e);
        setRecording(false);
        setTranscription("");
        setActionLoading(false);
      }
    } else {
      try {
        const result = await invoke<{ text: string; confidence: number }>("voice_stop_recording");
        setTranscription(result.text);
        setConfidence(Math.round(result.confidence * 100));
        setHistory((prev) => [
          { text: result.text, time: new Date().toISOString().replace("T", " ").slice(0, 19), confidence: result.confidence },
          ...prev,
        ]);
      } catch (e) {
        console.error("Failed to stop recording:", e);
      }
      setRecording(false);
      setActionLoading(false);
    }
  }, [recording]);

  const confColor = confidence > 80 ? "var(--success-color)" : confidence > 50 ? "var(--warning-color)" : "var(--error-color)";

  return (
    <div className="panel-container">
      <div className="panel-tab-bar">
        {["record", "models", "history", "config"].map((t) => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      <div className="panel-body">
        {tab === "record" && (
          <div style={{ textAlign: "center" }}>
            <button onClick={toggleRecording} disabled={actionLoading && !recording} style={{
              width: 72, height: 72, borderRadius: "50%", border: "none", cursor: "pointer",
              background: recording ? "var(--error-color)" : "var(--error-color)", boxShadow: recording ? "0 0 0 8px #ef444440" : "none",
              marginBottom: 20, transition: "box-shadow 0.3s",
            }}>
              <div style={{ width: 24, height: 24, borderRadius: recording ? 4 : 12, background: "var(--btn-primary-fg, #fff)", margin: "auto" }} />
            </button>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 16 }}>
              {recording ? "Recording... click to stop" : "Click to start recording"}
            </div>
            <div className="panel-card" style={{ minHeight: 60, textAlign: "left" }}>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Transcription</div>
              <div style={{ fontSize: 14 }}>{transcription || "No transcription yet"}</div>
            </div>
            {confidence > 0 && (
              <div className="panel-card" style={{ textAlign: "left" }}>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Confidence</div>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <div style={{ flex: 1, height: 8, borderRadius: 4, background: "var(--border-color)" }}>
                    <div style={{ width: `${confidence}%`, height: 8, borderRadius: 4, background: confColor }} />
                  </div>
                  <span style={{ color: confColor, fontWeight: 600, fontSize: 13 }}>{confidence}%</span>
                </div>
              </div>
            )}
          </div>
        )}

        {tab === "models" && (
          <div>
            {loadingModels && <div className="panel-loading">Loading models...</div>}
            {!loadingModels && models.length === 0 && <div className="panel-empty">No voice models available.</div>}
            {models.map((m) => (
              <div key={m.name} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <span style={{ fontWeight: 600, fontSize: 13 }}>whisper-{m.name}</span>
                  <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 8 }}>{m.size}</span>
                  {m.selected && <span style={{ marginLeft: 8, fontSize: 11, color: "var(--success-color)", fontWeight: 600 }}>SELECTED</span>}
                </div>
                {m.downloaded ? (
                  <button className="panel-btn panel-btn-primary" style={{ background: m.selected ? "var(--success-color)" : undefined }}>
                    {m.selected ? "Active" : "Select"}
                  </button>
                ) : (
                  <button className="panel-btn panel-btn-secondary">Download</button>
                )}
              </div>
            ))}
          </div>
        )}

        {tab === "history" && (
          <div>
            {history.length === 0 && <div className="panel-empty">No transcription history yet. Record something to get started.</div>}
            {history.map((h, i) => (
              <div key={i} className="panel-card">
                <div style={{ fontSize: 13, marginBottom: 4 }}>{h.text}</div>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-secondary)" }}>
                  <span>{h.time}</span>
                  <span style={{ color: h.confidence > 0.9 ? "var(--success-color)" : "var(--warning-color)" }}>{(h.confidence * 100).toFixed(0)}%</span>
                </div>
              </div>
            ))}
          </div>
        )}

        {tab === "config" && (
          <div>
            <div className="panel-card">
              <div className="panel-label">Language</div>
              <select value={language} onChange={(e) => setLanguage(e.target.value)}
                style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }}>
                {["en", "es", "fr", "de", "ja", "zh", "ko", "ru"].map((l) => <option key={l} value={l}>{l}</option>)}
              </select>
            </div>
            <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontWeight: 600, fontSize: 13 }}>Voice Activity Detection</span>
              <button className="panel-btn panel-btn-primary" style={{ background: vad ? "var(--success-color)" : "var(--border-color)" }}
                onClick={() => setVad(!vad)}>{vad ? "ON" : "OFF"}</button>
            </div>
            <div className="panel-card">
              <div className="panel-label">Sample Rate</div>
              <select value={sampleRate} onChange={(e) => setSampleRate(e.target.value)}
                style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }}>
                {["8000", "16000", "22050", "44100", "48000"].map((r) => <option key={r} value={r}>{r} Hz</option>)}
              </select>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
