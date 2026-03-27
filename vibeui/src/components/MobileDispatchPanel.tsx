/**
 * MobileDispatchPanel — Manage registered mobile devices, machine registration,
 * and view dispatch status for the VibeCody Mobile Gateway.
 *
 * Tabs: Machines, Devices, Dispatches, Pairing, Stats
 */
import React, { useState } from "react";

type Tab = "Machines" | "Devices" | "Dispatches" | "Pairing" | "Stats";
const TABS: Tab[] = ["Machines", "Devices", "Dispatches", "Pairing", "Stats"];

const STATUS_COLORS: Record<string, string> = {
  online: "var(--success-color)",
  idle: "var(--success-color)",
  busy: "var(--warning-color)",
  offline: "var(--error-color)",
  unreachable: "var(--text-secondary)",
};

const DISPATCH_STATUS_COLORS: Record<string, string> = {
  queued: "var(--text-secondary)",
  sent: "var(--accent-blue)",
  running: "var(--accent-blue)",
  completed: "var(--success-color)",
  failed: "var(--error-color)",
  cancelled: "var(--text-secondary)",
  timed_out: "var(--warning-color)",
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
const metricCardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 8, padding: 16, textAlign: "center" as const,
  border: "1px solid var(--border-color)", flex: 1, minWidth: 120,
};
const metricValue: React.CSSProperties = { fontSize: 28, fontWeight: 700 };
const metricLabel: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginTop: 4 };

const MACHINES = [
  { machine_id: "mach-a1", name: "Mac Studio (macOS)", os: "macOS", arch: "aarch64", status: "online", daemon_port: 7878, workspace: "/Users/dev/project", cpu_cores: 12, memory_gb: 64, active_sessions: 2, tags: ["prod", "gpu"] },
  { machine_id: "mach-b2", name: "Dev Server (Linux)", os: "Linux", arch: "x86_64", status: "busy", daemon_port: 7878, workspace: "/home/dev/workspace", cpu_cores: 32, memory_gb: 128, active_sessions: 5, tags: ["ci"] },
  { machine_id: "mach-c3", name: "Docker Builder", os: "Docker", arch: "x86_64", status: "idle", daemon_port: 7879, workspace: "/app", cpu_cores: 8, memory_gb: 16, active_sessions: 0, tags: [] },
  { machine_id: "mach-d4", name: "Windows Workstation", os: "Windows", arch: "x86_64", status: "offline", daemon_port: 7878, workspace: "C:\\Users\\dev\\code", cpu_cores: 16, memory_gb: 32, active_sessions: 0, tags: ["test"] },
];

const DEVICES = [
  { device_id: "dev-001", device_name: "iPhone 16 Pro", platform: "apns", paired_machines: ["mach-a1", "mach-b2"], app_version: "1.0.0", os_version: "18.3", last_seen: "2 min ago" },
  { device_id: "dev-002", device_name: "Pixel 9", platform: "fcm", paired_machines: ["mach-a1"], app_version: "1.0.0", os_version: "15.0", last_seen: "15 min ago" },
  { device_id: "dev-003", device_name: "iPad Pro", platform: "apns", paired_machines: ["mach-a1", "mach-b2", "mach-c3"], app_version: "1.0.0", os_version: "18.3", last_seen: "1 hr ago" },
];

const DISPATCHES = [
  { task_id: "dsp-1", machine: "Mac Studio", type: "chat", payload: "What's the status of the auth refactor?", status: "completed", result: "The auth refactor is 80% complete...", time: "2 min ago" },
  { task_id: "dsp-2", machine: "Dev Server", type: "agent_task", payload: "Fix the failing test in payment_test.rs", status: "running", result: null, time: "5 min ago" },
  { task_id: "dsp-3", machine: "Mac Studio", type: "git_op", payload: "status", status: "completed", result: "On branch main, 3 files modified", time: "12 min ago" },
  { task_id: "dsp-4", machine: "Dev Server", type: "repl_command", payload: "/coverage", status: "queued", result: null, time: "1 min ago" },
  { task_id: "dsp-5", machine: "Docker Builder", type: "command", payload: "cargo build --release", status: "failed", result: "error[E0308]: mismatched types", time: "30 min ago" },
];

const PAIRINGS = [
  { id: "pair-1", machine: "Mac Studio", method: "qr_code", pin: "482917", status: "pending", expires_in: "8:42" },
];

const OS_ICONS: Record<string, string> = { macOS: "🍎", Linux: "🐧", Windows: "🪟", Docker: "🐳", WSL: "🐧" };

export default function MobileDispatchPanel() {
  const [tab, setTab] = useState<Tab>("Machines");

  return (
    <div style={containerStyle}>
      <div style={statusBarStyle}>
        <span>📱 Mobile Gateway</span>
        <span>3 machines online · 3 devices paired · 2 active dispatches</span>
      </div>
      <div style={tabBarStyle}>
        {TABS.map(t => (
          <button key={t} style={tabStyle(t === tab)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle}>
        {tab === "Machines" && <MachinesTab />}
        {tab === "Devices" && <DevicesTab />}
        {tab === "Dispatches" && <DispatchesTab />}
        {tab === "Pairing" && <PairingTab />}
        {tab === "Stats" && <StatsTab />}
      </div>
    </div>
  );
}

function MachinesTab() {
  return (
    <>
      {MACHINES.map(m => (
        <div key={m.machine_id} style={cardStyle}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
              <span style={{ fontSize: 24 }}>{OS_ICONS[m.os] || "💻"}</span>
              <div>
                <div style={{ fontWeight: 600 }}>{m.name}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{m.workspace} · port {m.daemon_port}</div>
              </div>
            </div>
            <span style={badgeStyle(STATUS_COLORS[m.status] || "var(--text-secondary)")}>{m.status}</span>
          </div>
          <div style={{ display: "flex", gap: 16, marginTop: 10, fontSize: 12, color: "var(--text-secondary)" }}>
            <span>🔧 {m.cpu_cores} cores</span>
            <span>💾 {m.memory_gb} GB</span>
            <span>📊 {m.active_sessions} sessions</span>
            <span>{m.arch}</span>
          </div>
          {m.tags.length > 0 && (
            <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
              {m.tags.map(t => <span key={t} style={{ padding: "1px 8px", borderRadius: 10, fontSize: 10, background: "var(--accent-blue)", color: "var(--bg-primary)", opacity: 0.8 }}>{t}</span>)}
            </div>
          )}
        </div>
      ))}
    </>
  );
}

function DevicesTab() {
  return (
    <>
      {DEVICES.map(d => (
        <div key={d.device_id} style={cardStyle}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontWeight: 600 }}>{d.device_name}</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                {d.platform.toUpperCase()} · {d.os_version} · v{d.app_version}
              </div>
            </div>
            <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{d.last_seen}</span>
          </div>
          <div style={{ marginTop: 8, fontSize: 12 }}>
            <span style={{ color: "var(--text-secondary)" }}>Paired with: </span>
            {d.paired_machines.map((mid, i) => (
              <span key={mid}>{i > 0 ? ", " : ""}{MACHINES.find(m => m.machine_id === mid)?.name || mid}</span>
            ))}
          </div>
        </div>
      ))}
    </>
  );
}

function DispatchesTab() {
  return (
    <>
      {DISPATCHES.map(d => (
        <div key={d.task_id} style={cardStyle}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={badgeStyle(DISPATCH_STATUS_COLORS[d.status] || "var(--text-secondary)")}>{d.status}</span>
              <span style={{ fontSize: 11, padding: "2px 6px", borderRadius: 4, background: "var(--bg-tertiary)" }}>{d.type}</span>
            </div>
            <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{d.time}</span>
          </div>
          <div style={{ marginTop: 8, fontFamily: "monospace", fontSize: 13 }}>{d.payload}</div>
          <div style={{ marginTop: 6, fontSize: 12, color: "var(--text-secondary)" }}>→ {d.machine}</div>
          {d.result && (
            <div style={{ marginTop: 8, padding: 8, borderRadius: 4, background: "var(--bg-tertiary)", fontSize: 12, fontFamily: "monospace" }}>{d.result}</div>
          )}
        </div>
      ))}
    </>
  );
}

function PairingTab() {
  return (
    <>
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontWeight: 600, marginBottom: 8 }}>Active Pairing Requests</div>
        {PAIRINGS.map(p => (
          <div key={p.id} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600 }}>{p.machine}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Method: {p.method} · Expires in {p.expires_in}</div>
              </div>
              <span style={badgeStyle("var(--warning-color)")}>{p.status}</span>
            </div>
            {p.pin && (
              <div style={{ marginTop: 12, textAlign: "center" }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>PIN Code</div>
                <div style={{ fontSize: 36, fontFamily: "monospace", fontWeight: 700, letterSpacing: 8, color: "var(--accent-blue)" }}>{p.pin}</div>
              </div>
            )}
          </div>
        ))}
      </div>
      <div style={{ padding: 16, background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)" }}>
        <div style={{ fontWeight: 600, marginBottom: 12 }}>How to Pair</div>
        <ol style={{ margin: 0, paddingLeft: 20, fontSize: 13, lineHeight: 1.8 }}>
          <li>Install <strong>VibeCody Mobile</strong> from App Store or Play Store</li>
          <li>Run <code style={{ padding: "2px 6px", borderRadius: 4, background: "var(--bg-tertiary)" }}>vibecli serve --port 7878</code> on your machine</li>
          <li>Open the mobile app and tap <strong>Scan QR Code</strong> or enter the 6-digit PIN</li>
          <li>Start managing your sessions remotely!</li>
        </ol>
      </div>
    </>
  );
}

function StatsTab() {
  const stats = {
    total_machines: 4, online_machines: 3, total_devices: 3,
    total_dispatches: 47, active_dispatches: 2, completed_dispatches: 38,
    failed_dispatches: 5, pending_notifications: 1, pending_pairings: 1,
  };

  return (
    <>
      <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 20 }}>
        <div style={metricCardStyle}>
          <div style={{ ...metricValue, color: "var(--accent-blue)" }}>{stats.online_machines}/{stats.total_machines}</div>
          <div style={metricLabel}>Machines Online</div>
        </div>
        <div style={metricCardStyle}>
          <div style={{ ...metricValue, color: "var(--success-color)" }}>{stats.total_devices}</div>
          <div style={metricLabel}>Paired Devices</div>
        </div>
        <div style={metricCardStyle}>
          <div style={{ ...metricValue, color: "var(--warning-color)" }}>{stats.active_dispatches}</div>
          <div style={metricLabel}>Active Dispatches</div>
        </div>
      </div>
      <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
        <div style={metricCardStyle}>
          <div style={{ ...metricValue, color: "var(--text-primary)" }}>{stats.total_dispatches}</div>
          <div style={metricLabel}>Total Dispatches</div>
        </div>
        <div style={metricCardStyle}>
          <div style={{ ...metricValue, color: "var(--success-color)" }}>{stats.completed_dispatches}</div>
          <div style={metricLabel}>Completed</div>
        </div>
        <div style={metricCardStyle}>
          <div style={{ ...metricValue, color: "var(--error-color)" }}>{stats.failed_dispatches}</div>
          <div style={metricLabel}>Failed</div>
        </div>
      </div>
    </>
  );
}
