/**
 * AudioOutputPanel — Generate audio narrations for changelogs, PRs, status, and custom text.
 *
 * Tabs: Generate, History, Settings
 */
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Generate" | "History" | "Settings";
const TABS: Tab[] = ["Generate", "History", "Settings"];


const NARRATION_TYPES = [
  { label: "Changelog", desc: "Narrate the latest changelog entries" },
  { label: "PR Summary", desc: "Summarize open pull requests" },
  { label: "Status Report", desc: "Daily project status overview" },
  { label: "Custom", desc: "Enter custom text to narrate" },
];

interface Narration { id: number; title: string; type: string; duration: string; date: string; format: string }

const AudioOutputPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Generate");
  const [provider, setProvider] = useState("OpenAI TTS");
  const [voice, setVoice] = useState("alloy");
  const [speed, setSpeed] = useState("1.0");
  const [format, setFormat] = useState("mp3");
  const [customText, setCustomText] = useState("");
  const [history, setHistory] = useState<Narration[]>([]);

  useEffect(() => {
    invoke<Narration[]>("list_narrations").then(setHistory).catch(() => {});
  }, []);

  const handleGenerate = async (type_: string) => {
    try {
      const result = await invoke<Narration>("create_narration", { narrationType: type_, text: customText || type_ });
      setHistory(prev => [result, ...prev]);
      setTab("History");
    } catch (_) { /* ignore */ }
  };

  return (
    <div className="panel-container" role="region" aria-label="Audio Output Panel">
      <div className="panel-tab-bar" role="tablist" aria-label="Audio Output tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {tab === "Generate" && (
          <div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginBottom: 16 }}>
              {NARRATION_TYPES.map(n => (
                <button key={n.label} className="panel-btn panel-btn-primary" aria-label={`Generate ${n.label}`} onClick={() => handleGenerate(n.label)}>{n.label}</button>
              ))}
            </div>
            <textarea style={{ height: 80, resize: "vertical", width: "100%", padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-md)", fontFamily: "inherit", boxSizing: "border-box" }} placeholder="Or enter custom text to narrate..." aria-label="Custom narration text" value={customText} onChange={e => setCustomText(e.target.value)} />
            <div style={{ marginTop: 8, fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
              Select a narration type above or type custom text, then click Generate.
            </div>
          </div>
        )}
        {tab === "History" && history.map((h, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{h.title}</strong>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{h.type} &middot; {h.duration} &middot; {h.format} &middot; {h.date}</div>
              </div>
              <button className="panel-btn panel-btn-secondary" aria-label={`Play ${h.title}`}>Play</button>
            </div>
          </div>
        ))}
        {tab === "Settings" && (
          <div>
            <div className="panel-card">
              <label className="panel-label">TTS Provider</label>
              <select style={{ width: "100%", marginTop: 4, padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-md)", fontFamily: "inherit", boxSizing: "border-box" }} value={provider} onChange={e => setProvider(e.target.value)} aria-label="TTS provider">
                <option>OpenAI TTS</option><option>ElevenLabs</option><option>Google Cloud TTS</option><option>Azure Speech</option>
              </select>
            </div>
            <div className="panel-card">
              <label className="panel-label">Voice</label>
              <select style={{ width: "100%", marginTop: 4, padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-md)", fontFamily: "inherit", boxSizing: "border-box" }} value={voice} onChange={e => setVoice(e.target.value)} aria-label="Voice selection">
                <option value="alloy">Alloy</option><option value="echo">Echo</option><option value="fable">Fable</option><option value="onyx">Onyx</option><option value="nova">Nova</option><option value="shimmer">Shimmer</option>
              </select>
            </div>
            <div className="panel-card">
              <label className="panel-label">Speed</label>
              <input style={{ marginTop: 4, width: "100%", padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-md)", fontFamily: "inherit", boxSizing: "border-box" }} type="number" min="0.5" max="2.0" step="0.1" value={speed} onChange={e => setSpeed(e.target.value)} aria-label="Playback speed" />
            </div>
            <div className="panel-card">
              <label className="panel-label">Output format</label>
              <select style={{ width: "100%", marginTop: 4, padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-md)", fontFamily: "inherit", boxSizing: "border-box" }} value={format} onChange={e => setFormat(e.target.value)} aria-label="Output format">
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
