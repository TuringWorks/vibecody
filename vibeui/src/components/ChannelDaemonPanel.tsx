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
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  border: "none", borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  fontSize: 13, fontFamily: "inherit", whiteSpace: "nowrap",
});
const contentStyle: React.CSSProperties = { flex: 1, overflow: "auto", padding: 16 };
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 8,
  border: "1px solid var(--border-color)",
};
const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: 10,
  fontSize: 11, background: color, color: "var(--bg-primary)", fontWeight: 600,
});
const statusBarStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)",
  display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12, flexShrink: 0,
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
    <div style={containerStyle} role="region" aria-label="Channel Daemon Panel">
      <div style={statusBarStyle}>
        <span>Daemon: <span style={{ color: "var(--success-color)", fontWeight: 600 }}>Running</span></span>
        <span>{channels.filter(c => c.status === "Connected").length}/{channels.length} channels connected</span>
      </div>
      <div style={tabBarStyle} role="tablist" aria-label="Channel Daemon tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Channels" && channels.map((ch, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{ch.name}</strong>
              <span style={badgeStyle(STATUS_COLORS[ch.status] || "var(--text-secondary)")}>{ch.status}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Type: {ch.type} &middot; Events processed: {ch.events}</div>
          </div>
        ))}
        {tab === "Events" && messages.map((ev, i) => (
          <div key={i} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between" }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>{ev.time}</span>
              <span style={badgeStyle("var(--info-color)")}>{ev.type}</span>
            </div>
            <div style={{ fontSize: 12, marginTop: 4 }}>[{ev.channel}] {ev.summary}</div>
          </div>
        ))}
        {tab === "Sessions" && (
          <div style={{ textAlign: "center", padding: 40, color: "var(--text-secondary)" }}>
            <div style={{ fontSize: 14 }}>No active sessions</div>
            <div style={{ fontSize: 12, marginTop: 4 }}>Sessions will appear here when channels are connected</div>
          </div>
        )}
        {tab === "Settings" && (
          <div>
            <div style={cardStyle}><strong>Auto-reconnect</strong><div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Automatically reconnect dropped channels after 30s</div></div>
            <div style={cardStyle}><strong>Event retention</strong><div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Keep event logs for 7 days (configurable)</div></div>
            <div style={cardStyle}><strong>Session timeout</strong><div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Idle sessions expire after 15 minutes</div></div>
            <div style={cardStyle}><strong>Rate limiting</strong><div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>Max 60 events/min per channel</div></div>
          </div>
        )}
      </div>
    </div>
  );
};

export default ChannelDaemonPanel;
