/**
 * AudioOutputPanel — Generate audio narrations for changelogs, PRs, status, and custom text.
 *
 * Tabs: Generate, History, Settings
 */
import React, { useState } from "react";

type Tab = "Generate" | "History" | "Settings";
const TABS: Tab[] = ["Generate", "History", "Settings"];

const containerStyle: React.CSSProperties = {
  display: "flex", flexDirection: "column", height: "100%",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  fontFamily: "inherit", overflow: "hidden",
};
const tabBarStyle: React.CSSProperties = {
  display: "flex", gap: 2, padding: "8px 12px 0",
  borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)",
  overflowX: "auto", flexShrink: 0,
};
const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px", cursor: "pointer",
  background: active ? "var(--bg-primary)" : "transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  border: "none", borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  fontSize: 13, fontFamily: "inherit", whiteSpace: "nowrap",
});
const contentStyle: React.CSSProperties = { flex: 1, overflow: "auto", padding: 16 };
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 8,
  border: "1px solid var(--border-color)",
};
const btnStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--accent-color)", color: "var(--bg-primary)",
  border: "none", borderRadius: 4, cursor: "pointer", fontSize: 12, fontFamily: "inherit",
};
const btnSecondary: React.CSSProperties = {
  ...btnStyle, background: "var(--bg-tertiary)", color: "var(--text-primary)",
};
const inputStyle: React.CSSProperties = {
  width: "100%", padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 13, fontFamily: "inherit",
  boxSizing: "border-box",
};
const selectStyle: React.CSSProperties = { ...inputStyle, width: "auto", minWidth: 140 };

const NARRATION_TYPES = [
  { label: "Changelog", desc: "Narrate the latest changelog entries" },
  { label: "PR Summary", desc: "Summarize open pull requests" },
  { label: "Status Report", desc: "Daily project status overview" },
  { label: "Custom", desc: "Enter custom text to narrate" },
];

const HISTORY = [
  { title: "Changelog v0.3.3", type: "Changelog", duration: "1:24", date: "2026-03-19", format: "MP3" },
  { title: "Sprint 12 Status", type: "Status Report", duration: "2:10", date: "2026-03-18", format: "MP3" },
  { title: "PR #142 Summary", type: "PR Summary", duration: "0:45", date: "2026-03-17", format: "WAV" },
];

const AudioOutputPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Generate");
  const [provider, setProvider] = useState("OpenAI TTS");
  const [voice, setVoice] = useState("alloy");
  const [speed, setSpeed] = useState("1.0");
  const [format, setFormat] = useState("mp3");

  return (
    <div style={containerStyle} role="region" aria-label="Audio Output Panel">
      <div style={tabBarStyle} role="tablist" aria-label="Audio Output tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Generate" && (
          <div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginBottom: 16 }}>
              {NARRATION_TYPES.map(n => (
                <button key={n.label} style={btnStyle} aria-label={`Generate ${n.label}`}>{n.label}</button>
              ))}
            </div>
            <textarea style={{ ...inputStyle, height: 80, resize: "vertical" }} placeholder="Or enter custom text to narrate..." aria-label="Custom narration text" />
            <div style={{ marginTop: 8, fontSize: 12, color: "var(--text-muted)" }}>
              Select a narration type above or type custom text, then click Generate.
            </div>
          </div>
        )}
        {tab === "History" && HISTORY.map((h, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{h.title}</strong>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{h.type} &middot; {h.duration} &middot; {h.format} &middot; {h.date}</div>
              </div>
              <button style={btnSecondary} aria-label={`Play ${h.title}`}>Play</button>
            </div>
          </div>
        ))}
        {tab === "Settings" && (
          <div>
            <div style={cardStyle}>
              <label style={{ fontSize: 12, fontWeight: 600 }}>TTS Provider</label>
              <select style={{ ...selectStyle, width: "100%", marginTop: 4 }} value={provider} onChange={e => setProvider(e.target.value)} aria-label="TTS provider">
                <option>OpenAI TTS</option><option>ElevenLabs</option><option>Google Cloud TTS</option><option>Azure Speech</option>
              </select>
            </div>
            <div style={cardStyle}>
              <label style={{ fontSize: 12, fontWeight: 600 }}>Voice</label>
              <select style={{ ...selectStyle, width: "100%", marginTop: 4 }} value={voice} onChange={e => setVoice(e.target.value)} aria-label="Voice selection">
                <option value="alloy">Alloy</option><option value="echo">Echo</option><option value="fable">Fable</option><option value="onyx">Onyx</option><option value="nova">Nova</option><option value="shimmer">Shimmer</option>
              </select>
            </div>
            <div style={cardStyle}>
              <label style={{ fontSize: 12, fontWeight: 600 }}>Speed</label>
              <input style={{ ...inputStyle, marginTop: 4 }} type="number" min="0.5" max="2.0" step="0.1" value={speed} onChange={e => setSpeed(e.target.value)} aria-label="Playback speed" />
            </div>
            <div style={cardStyle}>
              <label style={{ fontSize: 12, fontWeight: 600 }}>Output format</label>
              <select style={{ ...selectStyle, width: "100%", marginTop: 4 }} value={format} onChange={e => setFormat(e.target.value)} aria-label="Output format">
                <option value="mp3">MP3</option><option value="wav">WAV</option><option value="opus">Opus</option><option value="flac">FLAC</option>
              </select>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default AudioOutputPanel;
