import { useState, useCallback } from "react";

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "#fff",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

export function VoiceLocalPanel() {
  const [tab, setTab] = useState("record");
  const [recording, setRecording] = useState(false);
  const [transcription, setTranscription] = useState("");
  const [confidence, setConfidence] = useState(0);
  const [language, setLanguage] = useState("en");
  const [vad, setVad] = useState(true);
  const [sampleRate, setSampleRate] = useState("16000");
  const [models] = useState([
    { name: "tiny", size: "75 MB", downloaded: true, selected: false },
    { name: "base", size: "142 MB", downloaded: true, selected: true },
    { name: "small", size: "466 MB", downloaded: false, selected: false },
    { name: "medium", size: "1.5 GB", downloaded: false, selected: false },
    { name: "large-v3", size: "2.9 GB", downloaded: false, selected: false },
  ]);
  const [history] = useState([
    { text: "Create a new function called parse config", time: "2026-03-26 10:15:32", confidence: 0.94 },
    { text: "Add error handling to the HTTP client", time: "2026-03-26 10:12:07", confidence: 0.89 },
    { text: "Run the test suite for the auth module", time: "2026-03-26 10:08:44", confidence: 0.97 },
    { text: "Refactor the database connection pool", time: "2026-03-26 09:55:21", confidence: 0.82 },
  ]);

  const toggleRecording = useCallback(() => {
    if (!recording) {
      setRecording(true);
      setTranscription("Listening...");
      setConfidence(0);
      setTimeout(() => {
        setTranscription("Add a new endpoint for user authentication");
        setConfidence(92);
        setRecording(false);
      }, 2000);
    } else {
      setRecording(false);
    }
  }, [recording]);

  const confColor = confidence > 80 ? "#22c55e" : confidence > 50 ? "#eab308" : "#ef4444";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Offline Voice Coding</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["record", "models", "history", "config"].map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "record" && (
        <div style={{ textAlign: "center" }}>
          <button onClick={toggleRecording} style={{
            width: 72, height: 72, borderRadius: "50%", border: "none", cursor: "pointer",
            background: recording ? "#ef4444" : "#dc2626", boxShadow: recording ? "0 0 0 8px #ef444440" : "none",
            marginBottom: 20, transition: "box-shadow 0.3s",
          }}>
            <div style={{ width: 24, height: 24, borderRadius: recording ? 4 : 12, background: "#fff", margin: "auto" }} />
          </button>
          <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 16 }}>
            {recording ? "Recording... click to stop" : "Click to start recording"}
          </div>
          <div style={{ ...cardStyle, minHeight: 60, textAlign: "left" }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Transcription</div>
            <div style={{ fontSize: 14 }}>{transcription || "No transcription yet"}</div>
          </div>
          {confidence > 0 && (
            <div style={{ ...cardStyle, textAlign: "left" }}>
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
          {models.map((m) => (
            <div key={m.name} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600, fontSize: 13 }}>whisper-{m.name}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 8 }}>{m.size}</span>
                {m.selected && <span style={{ marginLeft: 8, fontSize: 11, color: "#22c55e", fontWeight: 600 }}>SELECTED</span>}
              </div>
              {m.downloaded ? (
                <button style={{ ...btnStyle, background: m.selected ? "#22c55e" : "var(--accent-color)" }}>
                  {m.selected ? "Active" : "Select"}
                </button>
              ) : (
                <button style={btnStyle}>Download</button>
              )}
            </div>
          ))}
        </div>
      )}

      {tab === "history" && (
        <div>
          {history.map((h, i) => (
            <div key={i} style={cardStyle}>
              <div style={{ fontSize: 13, marginBottom: 4 }}>{h.text}</div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-secondary)" }}>
                <span>{h.time}</span>
                <span style={{ color: h.confidence > 0.9 ? "#22c55e" : "#eab308" }}>{(h.confidence * 100).toFixed(0)}%</span>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Language</div>
            <select value={language} onChange={(e) => setLanguage(e.target.value)}
              style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }}>
              {["en", "es", "fr", "de", "ja", "zh", "ko", "ru"].map((l) => <option key={l} value={l}>{l}</option>)}
            </select>
          </div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontWeight: 600, fontSize: 13 }}>Voice Activity Detection</span>
            <button style={{ ...btnStyle, background: vad ? "#22c55e" : "var(--border-color)" }}
              onClick={() => setVad(!vad)}>{vad ? "ON" : "OFF"}</button>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Sample Rate</div>
            <select value={sampleRate} onChange={(e) => setSampleRate(e.target.value)}
              style={{ padding: "4px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }}>
              {["8000", "16000", "22050", "44100", "48000"].map((r) => <option key={r} value={r}>{r} Hz</option>)}
            </select>
          </div>
        </div>
      )}
    </div>
  );
}
