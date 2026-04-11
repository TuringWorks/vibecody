import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Send, Square, FolderOpen, RefreshCw, Bot, User,
  MessageSquare, AlertCircle, CheckCircle, ChevronDown, Play,
} from "lucide-react";

// ── Types ─────────────────────────────────────────────────────────────────────

type PlatformId =
  | "telegram" | "discord" | "slack" | "whatsapp"
  | "teams" | "googlechat" | "mattermost" | "matrix"
  | "feishu" | "dingtalk" | "wecom" | "line" | "qq" | "zalo"
  | "nextcloud_talk" | "irc" | "webchat" | "synology_chat" | "signal" | "bluebubles"
  | "nostr" | "tlon"
  | "twilio" | "imessage";

type GatewayMode = "polling" | "webhook" | "cli";

interface CredentialField {
  key: string;
  label: string;
  placeholder: string;
  type?: "text" | "password" | "number";
}

interface PlatformDef {
  id: PlatformId;
  label: string;
  mode: GatewayMode;
  group: string;
  fields: CredentialField[];
  helpText: string;
}

interface LogEntry {
  dir: "in" | "out" | "info" | "error";
  platform: string;
  user: string;
  text: string;
  ts: number;
}

interface GatewayStatus {
  active: boolean;
  message_count: number;
}

// ── Platform definitions ───────────────────────────────────────────────────────

const PLATFORMS: PlatformDef[] = [
  // Popular
  {
    id: "telegram", label: "Telegram", mode: "polling", group: "Popular",
    fields: [
      { key: "token", label: "Bot Token", placeholder: "123456:ABCdefGHI...", type: "password" },
    ],
    helpText: "Create a bot with @BotFather and paste the token here.",
  },
  {
    id: "discord", label: "Discord", mode: "polling", group: "Popular",
    fields: [
      { key: "token", label: "Bot Token", placeholder: "MTQx...", type: "password" },
      { key: "channel_id", label: "Channel ID", placeholder: "1234567890" },
    ],
    helpText: "Create a Discord bot and enable Message Content intent. Copy the bot token and target channel ID.",
  },
  {
    id: "slack", label: "Slack", mode: "polling", group: "Popular",
    fields: [
      { key: "bot_token", label: "Bot OAuth Token", placeholder: "xoxb-...", type: "password" },
      { key: "channel", label: "Channel ID", placeholder: "C1234567" },
    ],
    helpText: "Create a Slack app, add chat:write and channels:history scopes, install to workspace.",
  },
  {
    id: "whatsapp", label: "WhatsApp", mode: "webhook", group: "Popular",
    fields: [
      { key: "access_token", label: "Access Token", placeholder: "EAAx...", type: "password" },
      { key: "phone_number_id", label: "Phone Number ID", placeholder: "123456789" },
      { key: "verify_token", label: "Verify Token", placeholder: "my-verify-secret" },
      { key: "port", label: "Webhook Port", placeholder: "8788", type: "number" },
    ],
    helpText: "Set up a Meta for Developers app with WhatsApp product. Configure the webhook to point to your machine:port/webhook.",
  },
  // Enterprise
  {
    id: "teams", label: "Microsoft Teams", mode: "webhook", group: "Enterprise",
    fields: [
      { key: "tenant_id", label: "Tenant ID", placeholder: "xxxxxxxx-...", type: "password" },
      { key: "client_id", label: "Client ID", placeholder: "xxxxxxxx-..." },
      { key: "client_secret", label: "Client Secret", placeholder: "secret", type: "password" },
      { key: "port", label: "Bot Port", placeholder: "3978", type: "number" },
    ],
    helpText: "Register an Azure Bot, configure the messaging endpoint to your machine:port/api/messages.",
  },
  {
    id: "googlechat", label: "Google Chat", mode: "cli", group: "Enterprise",
    fields: [
      { key: "service_account_json", label: "Service Account JSON", placeholder: "{...}", type: "password" },
      { key: "space_id", label: "Space ID", placeholder: "spaces/AAAA..." },
    ],
    helpText: "Google Chat requires service account credentials and a published Google Workspace app. Use vibecli --gateway googlechat for full support.",
  },
  {
    id: "mattermost", label: "Mattermost", mode: "polling", group: "Enterprise",
    fields: [
      { key: "server_url", label: "Server URL", placeholder: "https://mattermost.example.com" },
      { key: "token", label: "Access Token", placeholder: "token...", type: "password" },
      { key: "channel_id", label: "Channel ID", placeholder: "abc123..." },
    ],
    helpText: "Create a Mattermost bot account and generate a personal access token or bot token.",
  },
  {
    id: "matrix", label: "Matrix", mode: "polling", group: "Enterprise",
    fields: [
      { key: "homeserver", label: "Homeserver URL", placeholder: "https://matrix.org" },
      { key: "access_token", label: "Access Token", placeholder: "syt_...", type: "password" },
      { key: "room_id", label: "Room ID", placeholder: "!roomid:matrix.org" },
      { key: "bot_user_id", label: "Bot User ID", placeholder: "@bot:matrix.org" },
    ],
    helpText: "Register a Matrix user, log in to get an access token, then invite the bot to your room.",
  },
  // Asian
  {
    id: "feishu", label: "Feishu / Lark", mode: "webhook", group: "Asian",
    fields: [
      { key: "app_id", label: "App ID", placeholder: "cli_..." },
      { key: "app_secret", label: "App Secret", placeholder: "secret", type: "password" },
      { key: "port", label: "Webhook Port", placeholder: "8791", type: "number" },
    ],
    helpText: "Create a Feishu/Lark app, enable Receive Messages permission, configure event subscription to your machine:port/feishu/event.",
  },
  {
    id: "dingtalk", label: "DingTalk", mode: "webhook", group: "Asian",
    fields: [
      { key: "access_token", label: "Access Token", placeholder: "token...", type: "password" },
      { key: "webhook_secret", label: "Webhook Secret", placeholder: "SEC...", type: "password" },
      { key: "port", label: "Webhook Port", placeholder: "8790", type: "number" },
    ],
    helpText: "Create a DingTalk custom robot, enable outgoing webhook and point it to your machine:port/dingtalk.",
  },
  {
    id: "wecom", label: "WeCom / WeChat Work", mode: "webhook", group: "Asian",
    fields: [
      { key: "corp_id", label: "Corp ID", placeholder: "wx..." },
      { key: "agent_id", label: "Agent ID", placeholder: "1000001" },
      { key: "secret", label: "Secret", placeholder: "secret", type: "password" },
      { key: "port", label: "Webhook Port", placeholder: "8792", type: "number" },
    ],
    helpText: "Create a WeCom custom app, configure the callback URL to your machine:port/wecom/callback.",
  },
  {
    id: "line", label: "LINE", mode: "webhook", group: "Asian",
    fields: [
      { key: "channel_access_token", label: "Channel Access Token", placeholder: "token...", type: "password" },
      { key: "channel_secret", label: "Channel Secret", placeholder: "secret", type: "password" },
      { key: "port", label: "Webhook Port", placeholder: "8789", type: "number" },
    ],
    helpText: "Create a LINE Messaging API channel in LINE Developers. Configure webhook URL to your machine:port/callback.",
  },
  {
    id: "qq", label: "QQ", mode: "cli", group: "Asian",
    fields: [
      { key: "app_id", label: "App ID", placeholder: "12345" },
      { key: "token", label: "Token", placeholder: "token...", type: "password" },
    ],
    helpText: "QQ requires napcat or go-cqhttp running locally. Use vibecli --gateway qq for full support.",
  },
  {
    id: "zalo", label: "Zalo", mode: "cli", group: "Asian",
    fields: [
      { key: "access_token", label: "Access Token", placeholder: "token...", type: "password" },
    ],
    helpText: "Zalo OA API requires app review. Use vibecli --gateway zalo for full support.",
  },
  // Self-hosted
  {
    id: "nextcloud_talk", label: "Nextcloud Talk", mode: "polling", group: "Self-hosted",
    fields: [
      { key: "server_url", label: "Server URL", placeholder: "https://cloud.example.com" },
      { key: "username", label: "Username", placeholder: "bot-user" },
      { key: "password", label: "Password / App Token", placeholder: "token", type: "password" },
      { key: "room_token", label: "Room Token", placeholder: "abc123" },
    ],
    helpText: "Use a Nextcloud user account or app password. The room token appears in the Talk room URL.",
  },
  {
    id: "irc", label: "IRC", mode: "cli", group: "Self-hosted",
    fields: [
      { key: "server", label: "Server", placeholder: "irc.libera.chat" },
      { key: "port", label: "Port", placeholder: "6667", type: "number" },
      { key: "nick", label: "Nickname", placeholder: "vibebot" },
      { key: "channel", label: "Channel", placeholder: "#mychannel" },
    ],
    helpText: "IRC requires a persistent TCP connection. Use vibecli --gateway irc for full support.",
  },
  {
    id: "webchat", label: "WebChat", mode: "cli", group: "Self-hosted",
    fields: [
      { key: "port", label: "Port", placeholder: "8793", type: "number" },
    ],
    helpText: "WebChat is a built-in browser-based chat widget. Use vibecli --gateway webchat for full support.",
  },
  {
    id: "synology_chat", label: "Synology Chat", mode: "webhook", group: "Self-hosted",
    fields: [
      { key: "server_url", label: "Synology Server URL", placeholder: "https://nas.example.com" },
      { key: "outgoing_url", label: "Outgoing Webhook URL (optional)", placeholder: "https://nas.example.com/..." },
      { key: "token", label: "Integration Token", placeholder: "token...", type: "password" },
    ],
    helpText: "Create a Synology Chat integration and configure the outgoing webhook to your machine:8794/synology.",
  },
  {
    id: "signal", label: "Signal", mode: "cli", group: "Self-hosted",
    fields: [
      { key: "api_url", label: "signal-cli REST API URL", placeholder: "http://localhost:8080" },
      { key: "phone_number", label: "Phone Number", placeholder: "+1234567890" },
    ],
    helpText: "Signal requires signal-cli REST API running locally. Use vibecli --gateway signal for full support.",
  },
  {
    id: "bluebubles", label: "BlueBubbles", mode: "polling", group: "Self-hosted",
    fields: [
      { key: "server_url", label: "BlueBubbles Server URL", placeholder: "http://192.168.1.x:1234" },
      { key: "password", label: "Server Password", placeholder: "password", type: "password" },
    ],
    helpText: "Run BlueBubbles server on your Mac. Requires macOS with iMessage. The password is set in BlueBubbles server settings.",
  },
  // Decentralized
  {
    id: "nostr", label: "Nostr", mode: "polling", group: "Decentralized",
    fields: [
      { key: "private_key", label: "Private Key (hex)", placeholder: "nsec or hex...", type: "password" },
      { key: "relay_url", label: "Relay URL", placeholder: "wss://relay.damus.io" },
    ],
    helpText: "Connect to any Nostr relay. The bot will respond to mentions and DMs.",
  },
  {
    id: "tlon", label: "Tlon / Urbit", mode: "cli", group: "Decentralized",
    fields: [
      { key: "ship_url", label: "Ship URL", placeholder: "http://localhost:8080" },
      { key: "ship_code", label: "Ship Code", placeholder: "~sampel-palnet", type: "password" },
    ],
    helpText: "Tlon/Urbit requires a running Urbit ship. Use vibecli --gateway tlon for full support.",
  },
  // SMS/Voice
  {
    id: "twilio", label: "Twilio SMS", mode: "cli", group: "SMS/Voice",
    fields: [
      { key: "account_sid", label: "Account SID", placeholder: "ACxxxx" },
      { key: "auth_token", label: "Auth Token", placeholder: "token...", type: "password" },
      { key: "from_number", label: "From Number", placeholder: "+1234567890" },
    ],
    helpText: "Twilio SMS requires a webhook endpoint and Twilio account. Use vibecli --gateway twilio for full support.",
  },
  {
    id: "imessage", label: "iMessage", mode: "cli", group: "SMS/Voice",
    fields: [
      { key: "db_path", label: "Chat DB Path (optional)", placeholder: "/Users/you/Library/Messages/chat.db" },
    ],
    helpText: "iMessage access requires macOS with Full Disk Access granted. Use vibecli --gateway imessage for full support.",
  },
];

const PLATFORM_BY_ID: Record<PlatformId, PlatformDef> = Object.fromEntries(
  PLATFORMS.map((p) => [p.id, p])
) as Record<PlatformId, PlatformDef>;

const MODE_BADGE: Record<GatewayMode, { label: string; color: string }> = {
  polling: { label: "Live polling", color: "var(--success-color)" },
  webhook: { label: "Webhook", color: "var(--warning-color)" },
  cli:     { label: "CLI only",   color: "#888" },
};

const GROUPS = ["Popular", "Enterprise", "Asian", "Self-hosted", "Decentralized", "SMS/Voice"];

// ── Component ─────────────────────────────────────────────────────────────────

export interface GatewaySandboxPanelProps {
  provider?: string;
}

export function GatewaySandboxPanel({ provider: defaultProvider = "claude" }: GatewaySandboxPanelProps) {
  const [platform, setPlatform] = useState<PlatformId>("telegram");
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [sandboxPath, setSandboxPath] = useState("");
  const [provider, setProvider] = useState(defaultProvider);
  const [allowedUsers, setAllowedUsers] = useState("");
  const [showConfig, setShowConfig] = useState(true);

  const [status, setStatus] = useState<GatewayStatus>({ active: false, message_count: 0 });
  const [log, setLog] = useState<LogEntry[]>([]);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cliOutput, setCliOutput] = useState("");

  const runGatewayCli = useCallback(async (p: string) => {
    setCliOutput(""); setError(null);
    try {
      const res = await invoke<string>("handle_gateway_cli", { platform: p });
      setCliOutput(res);
    } catch (e) { setError(String(e)); }
  }, []);

  const logEndRef = useRef<HTMLDivElement>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refreshLog = useCallback(async () => {
    try {
      const [rawLog, rawStatus] = await Promise.all([
        invoke<LogEntry[]>("get_sandbox_gateway_log"),
        invoke<GatewayStatus>("get_sandbox_gateway_status"),
      ]);
      setLog(rawLog);
      setStatus(rawStatus);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    refreshLog();
    pollRef.current = setInterval(refreshLog, 2000);
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [refreshLog]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [log]);

  // Reset credentials when platform changes
  useEffect(() => {
    setCredentials({});
    setError(null);
  }, [platform]);

  const handlePickFolder = async () => {
    const selected = await open({ directory: true, multiple: false, title: "Select Sandbox Folder" });
    if (typeof selected === "string") setSandboxPath(selected);
  };

  const setField = (key: string, value: string) => {
    setCredentials((prev) => ({ ...prev, [key]: value }));
  };

  const handleStart = async () => {
    const def = PLATFORM_BY_ID[platform];
    if (def.mode === "cli") {
      setError(`Platform '${def.label}' is CLI-only. Run: vibecli --gateway ${platform}`);
      return;
    }
    if (!sandboxPath.trim()) { setError("Sandbox folder is required"); return; }
    setError(null);
    setStarting(true);
    try {
      await invoke("start_sandbox_gateway", {
        platform,
        credentials,
        sandboxPath: sandboxPath.trim(),
        provider,
        allowedUsers: allowedUsers.split(",").map((u) => u.trim()).filter(Boolean),
      });
      setShowConfig(false);
      await refreshLog();
    } catch (e) {
      setError(String(e));
    } finally {
      setStarting(false);
    }
  };

  const handleStop = async () => {
    await invoke("stop_sandbox_gateway").catch(() => {});
    await refreshLog();
  };

  const def = PLATFORM_BY_ID[platform];

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <MessageSquare size={14} style={{ color: "var(--text-secondary)" }} />
        <span style={{ fontWeight: 600, fontSize: 13, flex: 1 }}>Messaging Gateway → Sandbox</span>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ width: 8, height: 8, borderRadius: "50%", background: status.active ? "var(--success-color)" : "#666", display: "inline-block" }} />
          <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
            {status.active ? `Active · ${status.message_count} msgs` : "Stopped"}
          </span>
        </div>
        <button
          onClick={() => setShowConfig((v) => !v)}
          style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 2 }}
          title={showConfig ? "Hide config" : "Show config"}
        >
          <ChevronDown size={14} style={{ transform: showConfig ? "rotate(0deg)" : "rotate(-90deg)", transition: "transform .2s" }} />
        </button>
      </div>

      {/* Config section */}
      {showConfig && (
        <div style={{ padding: 12, borderBottom: "1px solid var(--border-color)", background: "var(--bg-primary)", display: "flex", flexDirection: "column", gap: 10, flexShrink: 0, overflowY: "auto", maxHeight: "60vh" }}>

          {/* Platform grid */}
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {GROUPS.map((group) => {
              const groupPlatforms = PLATFORMS.filter((p) => p.group === group);
              return (
                <div key={group}>
                  <div style={{ fontSize: 10, fontWeight: 700, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: 4 }}>{group}</div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
                    {groupPlatforms.map((p) => {
                      const mb = MODE_BADGE[p.mode];
                      const isSelected = platform === p.id;
                      return (
                        <button
                          key={p.id}
                          onClick={() => !status.active && setPlatform(p.id)}
                          disabled={status.active}
                          style={{
                            padding: "4px 9px",
                            borderRadius: 6,
                            fontSize: 11,
                            cursor: status.active ? "default" : "pointer",
                            background: isSelected ? "var(--accent)" : "var(--bg-secondary)",
                            color: isSelected ? "#fff" : "var(--text-primary)",
                            border: `1px solid ${isSelected ? "var(--accent)" : "var(--border-color)"}`,
                            display: "flex",
                            alignItems: "center",
                            gap: 5,
                          }}
                          title={mb.label}
                        >
                          <span style={{ width: 6, height: 6, borderRadius: "50%", background: mb.color, display: "inline-block", flexShrink: 0 }} />
                          {p.label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>

          {/* Legend */}
          <div style={{ display: "flex", gap: 12, fontSize: 10, color: "var(--text-muted)" }}>
            {(Object.entries(MODE_BADGE) as [GatewayMode, { label: string; color: string }][]).map(([, mb]) => (
              <span key={mb.label} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ width: 6, height: 6, borderRadius: "50%", background: mb.color, display: "inline-block" }} />
                {mb.label}
              </span>
            ))}
          </div>

          {/* Platform help text */}
          <p style={{ fontSize: 11, color: "var(--text-secondary)", margin: 0 }}>{def.helpText}</p>

          {/* CLI-only notice */}
          {def.mode === "cli" && (
            <div style={{ display: "flex", alignItems: "center", gap: 8, background: "rgba(136,136,136,0.1)", border: "1px solid rgba(136,136,136,0.3)", borderRadius: 6, padding: "6px 10px", fontSize: 12, color: "var(--text-secondary)" }}>
              <AlertCircle size={13} />
              <span>CLI-only platform.</span>
              <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runGatewayCli(platform)} title={`vibecli --gateway ${platform}`} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><Play size={12} /> Launch Gateway</button>
            </div>
          )}
          {cliOutput && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: 11, background: "var(--bg-secondary)", padding: 8, borderRadius: 4 }}>{cliOutput}</pre>}

          {/* Credential fields */}
          {def.fields.map((field) => (
            <div key={field.key} style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <label className="panel-label">{field.label}</label>
              <input
                value={credentials[field.key] ?? ""}
                onChange={(e) => setField(field.key, e.target.value)}
                placeholder={field.placeholder}
                disabled={status.active}
                type={field.type === "password" ? "password" : "text"}
                style={inputStyle}
              />
            </div>
          ))}

          {/* Sandbox folder */}
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <label className="panel-label">Sandbox Folder</label>
            <div style={{ display: "flex", gap: 6 }}>
              <input
                value={sandboxPath}
                onChange={(e) => setSandboxPath(e.target.value)}
                placeholder="/Users/you/my-sandbox"
                disabled={status.active}
                style={{ ...inputStyle, flex: 1 }}
              />
              <button
                onClick={handlePickFolder}
                disabled={status.active}
                title="Browse"
                style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "0 8px", cursor: "pointer", color: "var(--text-secondary)", display: "flex", alignItems: "center" }}
              >
                <FolderOpen size={13} />
              </button>
            </div>
          </div>

          {/* Provider + allowed users */}
          <div style={{ display: "flex", gap: 8 }}>
            <div style={{ display: "flex", flexDirection: "column", gap: 4, flex: 1 }}>
              <label className="panel-label">AI Provider</label>
              <input
                value={provider}
                onChange={(e) => setProvider(e.target.value)}
                placeholder="claude"
                disabled={status.active}
                style={inputStyle}
              />
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 4, flex: 1 }}>
              <label className="panel-label">Allowed Users (comma-sep, optional)</label>
              <input
                value={allowedUsers}
                onChange={(e) => setAllowedUsers(e.target.value)}
                placeholder="@alice, @bob"
                disabled={status.active}
                style={inputStyle}
              />
            </div>
          </div>

          {error && (
            <div className="panel-error">
              <AlertCircle size={13} /> {error}
            </div>
          )}

          {/* Start / Stop */}
          <div style={{ display: "flex", gap: 8 }}>
            {!status.active ? (
              <button
                onClick={handleStart}
                disabled={starting || def.mode === "cli"}
                style={{ flex: 1, background: def.mode === "cli" ? "var(--bg-secondary)" : "var(--accent)", color: def.mode === "cli" ? "var(--text-muted)" : "#fff", border: "none", borderRadius: 8, padding: "8px 14px", cursor: def.mode === "cli" ? "default" : "pointer", fontSize: 13, fontWeight: 500, display: "flex", alignItems: "center", justifyContent: "center", gap: 6 }}
              >
                {starting ? <RefreshCw size={14} style={{ animation: "spin 1s linear infinite" }} /> : <Send size={14} />}
                {starting ? "Starting…" : def.mode === "cli" ? "Use vibecli CLI" : "Start Gateway"}
              </button>
            ) : (
              <button
                onClick={handleStop}
                className="panel-btn panel-btn-danger"
                style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", gap: 6 }}
              >
                <Square size={14} /> Stop Gateway
              </button>
            )}
          </div>
        </div>
      )}

      {/* Message log */}
      <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px", display: "flex", flexDirection: "column", gap: 8 }}>
        {log.length === 0 && (
          <div className="panel-empty">
            {status.active
              ? `Waiting for messages… Send a message to your ${def.label} bot.`
              : "Start the gateway to receive messages."}
          </div>
        )}
        {log.map((entry, i) => (
          <LogBubble key={i} entry={entry} />
        ))}
        <div ref={logEndRef} />
      </div>

      {/* Status footer */}
      {status.active && sandboxPath && (
        <div style={{ padding: "6px 12px", borderTop: "1px solid var(--border-color)", fontSize: 11, color: "var(--text-muted)", display: "flex", alignItems: "center", gap: 6, flexShrink: 0 }}>
          <CheckCircle size={12} style={{ color: "var(--success-color)" }} />
          Sandbox: <span style={{ color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{sandboxPath}</span>
          · Provider: {provider}
          · Platform: <span style={{ color: "var(--text-secondary)" }}>{def.label}</span>
        </div>
      )}
    </div>
  );
}

// ── Log bubble ──────────────────────────────────────────────────────────────────

function LogBubble({ entry }: { entry: LogEntry }) {
  const isIncoming = entry.dir === "in";
  const isSystem = entry.dir === "info" || entry.dir === "error";
  const time = new Date(entry.ts * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

  if (isSystem) {
    return (
      <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, color: entry.dir === "error" ? "var(--error-color)" : "var(--text-muted)", padding: "2px 0" }}>
        {entry.dir === "error" ? <AlertCircle size={11} /> : <CheckCircle size={11} />}
        <span>[{entry.platform}] {entry.text}</span>
        <span style={{ marginLeft: "auto", fontSize: 10, opacity: 0.6 }}>{time}</span>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: isIncoming ? "flex-start" : "flex-end", gap: 2 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
        {isIncoming ? <User size={11} style={{ color: "var(--text-muted)" }} /> : <Bot size={11} style={{ color: "#4f9cf9" }} />}
        <span style={{ fontSize: 10, color: "var(--text-muted)" }}>
          {isIncoming ? `@${entry.user}` : "AI"} · {time}
        </span>
      </div>
      <div style={{
        maxWidth: "88%",
        background: isIncoming ? "var(--bg-secondary)" : "var(--bg-tertiary, var(--bg-secondary))",
        border: `1px solid ${isIncoming ? "var(--border-color)" : "rgba(79,156,249,0.3)"}`,
        borderRadius: isIncoming ? "4px 12px 12px 12px" : "12px 4px 12px 12px",
        padding: "6px 10px",
        fontSize: 12,
        lineHeight: 1.5,
        whiteSpace: "pre-wrap",
        wordBreak: "break-word",
        color: "var(--text-primary)",
      }}>
        {entry.text}
      </div>
    </div>
  );
}

// ── Style helpers ───────────────────────────────────────────────────────────────

const inputStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: 6,
  padding: "5px 9px",
  fontSize: 12,
  color: "var(--text-primary)",
  width: "100%",
  boxSizing: "border-box",
};
