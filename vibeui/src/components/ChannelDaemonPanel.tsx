/**
 * ChannelDaemonPanel — Manages channel daemon configuration, events, and sessions.
 *
 * Tabs: Channels, Events, Sessions, Settings
 */
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Channels" | "Events" | "Sessions" | "Settings";
const TABS: Tab[] = ["Channels", "Events", "Sessions", "Settings"];

const STATUS_COLORS: Record<string, string> = {
  Connected: "var(--success-color)",
  Disconnected: "var(--error-color)",
  Reconnecting: "var(--warning-color)",
  Idle: "var(--text-secondary)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)", background: color, color: "var(--bg-primary)", fontWeight: 600,
});
const statusBarStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)",
  display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "var(--font-size-base)", flexShrink: 0,
};

interface Channel { name: string; type: string; status: string; events: number }
interface Message { time: string; channel: string; type: string; summary: string }

const ChannelDaemonPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Channels");
  const [channels, setChannels] = useState<Channel[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);

  useEffect(() => {
    invoke<Channel[]>("list_daemon_channels").then(setChannels).catch(() => {});
    invoke<Message[]>("get_channel_messages").then(setMessages).catch(() => {});
  }, []);

  return (
    <div className="panel-container" role="region" aria-label="Channel Daemon Panel">
      <div style={statusBarStyle}>
        <span>Daemon: <span style={{ color: "var(--success-color)", fontWeight: 600 }}>Running</span></span>
        <span>{channels.filter(c => c.status === "Connected").length}/{channels.length} channels connected</span>
      </div>
      <div className="panel-tab-bar" role="tablist" aria-label="Channel Daemon tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {tab === "Channels" && channels.map((ch, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{ch.name}</strong>
              <span style={badgeStyle(STATUS_COLORS[ch.status] || "var(--text-secondary)")}>{ch.status}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Type: {ch.type} &middot; Events processed: {ch.events}</div>
          </div>
        ))}
        {tab === "Events" && messages.map((ev, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between" }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)" }}>{ev.time}</span>
              <span style={badgeStyle("var(--info-color)")}>{ev.type}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", marginTop: 4 }}>[{ev.channel}] {ev.summary}</div>
          </div>
        ))}
        {tab === "Sessions" && (
          <div className="panel-empty">
            <div style={{ fontSize: "var(--font-size-lg)" }}>No active sessions</div>
            <div style={{ fontSize: "var(--font-size-base)", marginTop: 4 }}>Sessions will appear here when channels are connected</div>
          </div>
        )}
        {tab === "Settings" && (
          <div>
            <div className="panel-card"><strong>Auto-reconnect</strong><div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>Automatically reconnect dropped channels after 30s</div></div>
            <div className="panel-card"><strong>Event retention</strong><div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>Keep event logs for 7 days (configurable)</div></div>
            <div className="panel-card"><strong>Session timeout</strong><div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>Idle sessions expire after 15 minutes</div></div>
            <div className="panel-card"><strong>Rate limiting</strong><div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>Max 60 events/min per channel</div></div>
          </div>
        )}
      </div>
    </div>
  );
};

export default ChannelDaemonPanel;
