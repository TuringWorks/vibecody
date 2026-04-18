/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * CollabPanel — Session management UI for CRDT multiplayer collaboration.
 *
 * Create/join rooms, see connected peers with color indicators, copy invite link, leave session.
 */

import { useState, useCallback, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CollabSessionInfo {
  room_id: string;
  peer_id: string;
  ws_url: string;
  peers: Array<{ peer_id: string; name: string; color: string }>;
}

interface CollabPanelProps {
  connected: boolean;
  roomId: string | null;
  peerId: string | null;
  peers: Array<{ peerId: string; name: string; color: string }>;
  onConnect: (wsUrl: string, userName: string) => void;
  onDisconnect: () => void;
  daemonPort?: number;
  apiToken?: string;
}

export function CollabPanel({
  connected,
  roomId,
  peerId,
  peers,
  onConnect,
  onDisconnect,
  daemonPort = 7878,
  apiToken = "",
}: CollabPanelProps) {
  const [userName, setUserName] = useState("User");
  const [joinRoomId, setJoinRoomId] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    };
  }, []);

  const handleCreate = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const session = await invoke<CollabSessionInfo>("create_collab_session", {
        roomId: null,
        userName,
        daemonPort,
      });
      const wsUrl = `${session.ws_url}?token=${encodeURIComponent(apiToken)}`;
      onConnect(wsUrl, userName);
    } catch (e: any) {
      setError(e?.toString() || "Failed to create session");
    } finally {
      setLoading(false);
    }
  }, [userName, daemonPort, apiToken, onConnect]);

  const handleJoin = useCallback(async () => {
    if (!joinRoomId.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const session = await invoke<CollabSessionInfo>("join_collab_session", {
        roomId: joinRoomId.trim(),
        userName,
        daemonPort,
      });
      const wsUrl = `${session.ws_url}?token=${encodeURIComponent(apiToken)}`;
      onConnect(wsUrl, userName);
    } catch (e: any) {
      setError(e?.toString() || "Failed to join session");
    } finally {
      setLoading(false);
    }
  }, [joinRoomId, userName, daemonPort, apiToken, onConnect]);

  const handleCopyInvite = useCallback(() => {
    if (roomId) {
      navigator.clipboard.writeText(roomId).then(() => {
        setCopied(true);
        if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
        copyTimeoutRef.current = setTimeout(() => {
          setCopied(false);
          copyTimeoutRef.current = null;
        }, 2000);
      }).catch(() => {});
    }
  }, [roomId]);

  const handleLeave = useCallback(async () => {
    try {
      await invoke("leave_collab_session");
    } catch {
      // ignore
    }
    onDisconnect();
  }, [onDisconnect]);

  // Connected state — show room info + peer list
  if (connected && roomId) {
    return (
      <div className="panel-container" style={{ padding: "12px", fontSize: "var(--font-size-md)", flex: 1, minHeight: 0, overflow: "auto" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "12px" }}>
          <span
            style={{
              width: "8px",
              height: "8px",
              borderRadius: "50%",
              background: "var(--success-color)",
              display: "inline-block",
            }}
          />
          <strong>Connected</strong>
        </div>

        <div
          style={{
            background: "var(--bg-tertiary)",
            border: "1px solid var(--border-color)",
            borderRadius: "var(--radius-xs-plus)",
            padding: "8px",
            marginBottom: "12px",
            fontSize: "var(--font-size-base)",
          }}
        >
          <div style={{ color: "var(--text-secondary)", marginBottom: "4px" }}>Room ID</div>
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <code style={{ flex: 1, wordBreak: "break-all" }}>{roomId}</code>
            <button className="panel-btn"
              onClick={handleCopyInvite}
              style={{
                padding: "2px 8px",
                fontSize: "var(--font-size-sm)",
                background: "var(--accent-color)",
                color: "var(--btn-primary-fg)",
                border: "none",
                borderRadius: "3px",
                cursor: "pointer",
              }}
            >
              {copied ? "Copied!" : "Copy"}
            </button>
          </div>
        </div>

        <div style={{ marginBottom: "8px", fontWeight: 600 }}>
          Peers ({peers.length})
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "4px", marginBottom: "16px" }}>
          {peers.map((p) => (
            <div
              key={p.peerId}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "8px",
                padding: "4px 8px",
                background: "var(--bg-tertiary)",
                borderRadius: "3px",
                border: "1px solid var(--border-color)",
              }}
            >
              <span
                style={{
                  width: "10px",
                  height: "10px",
                  borderRadius: "50%",
                  background: p.color,
                  display: "inline-block",
                  flexShrink: 0,
                }}
              />
              <span style={{ flex: 1 }}>{p.name}</span>
              {p.peerId === peerId && (
                <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>(you)</span>
              )}
            </div>
          ))}
          {peers.length === 0 && (
            <div style={{ color: "var(--text-secondary)", fontStyle: "italic" }}>
              No peers connected yet. Share the Room ID to invite collaborators.
            </div>
          )}
        </div>

        <button className="panel-btn"
          onClick={handleLeave}
          style={{
            padding: "8px 16px",
            background: "var(--error-color)",
            color: "var(--btn-primary-fg)",
            border: "none",
            borderRadius: "var(--radius-xs-plus)",
            cursor: "pointer",
            fontSize: "var(--font-size-base)",
          }}
        >
          Leave Session
        </button>
      </div>
    );
  }

  // Disconnected state — create or join
  return (
    <div style={{ padding: "12px", fontSize: "var(--font-size-md)", flex: 1, minHeight: 0, overflow: "auto" }}>
      <div style={{ fontWeight: 600, marginBottom: "12px" }}>Multiplayer Collaboration</div>
      <p style={{ color: "var(--text-secondary)", marginBottom: "16px", fontSize: "var(--font-size-base)" }}>
        Real-time collaborative editing powered by CRDTs. Create a new session or join an existing
        one by Room ID.
      </p>

      {error && (
        <div
          style={{
            background: "rgba(224,108,117,0.1)",
            border: "1px solid var(--error-color)",
            borderRadius: "var(--radius-xs-plus)",
            padding: "8px",
            marginBottom: "12px",
            fontSize: "var(--font-size-base)",
            color: "var(--error-color)",
          }}
        >
          {error}
        </div>
      )}

      <div style={{ marginBottom: "12px" }}>
        <label style={{ display: "block", marginBottom: "4px", fontSize: "var(--font-size-base)" }}>
          Your Name
        </label>
        <input
          value={userName}
          onChange={(e) => setUserName(e.target.value)}
          placeholder="Enter your name..."
          style={{
            width: "100%",
            padding: "8px 8px",
            fontSize: "var(--font-size-base)",
            background: "var(--bg-tertiary)",
            color: "var(--text-primary)",
            border: "1px solid var(--border-color)",
            borderRadius: "var(--radius-xs-plus)",
            boxSizing: "border-box",
          }}
        />
      </div>

      <div style={{ display: "flex", gap: "8px", marginBottom: "20px" }}>
        <button className="panel-btn"
          onClick={handleCreate}
          disabled={loading || !userName.trim()}
          style={{
            flex: 1,
            padding: "8px",
            background: "var(--accent-color)",
            color: "var(--btn-primary-fg)",
            border: "none",
            borderRadius: "var(--radius-xs-plus)",
            cursor: loading ? "wait" : "pointer",
            fontSize: "var(--font-size-base)",
            fontWeight: 600,
            opacity: loading ? 0.6 : 1,
          }}
        >
          {loading ? "Creating..." : "Create Session"}
        </button>
      </div>

      <div
        style={{
          borderTop: "1px solid var(--border-color)",
          paddingTop: "16px",
        }}
      >
        <label style={{ display: "block", marginBottom: "4px", fontSize: "var(--font-size-base)" }}>
          Join Existing Room
        </label>
        <div style={{ display: "flex", gap: "8px" }}>
          <input
            value={joinRoomId}
            onChange={(e) => setJoinRoomId(e.target.value)}
            placeholder="Paste Room ID..."
            style={{
              flex: 1,
              padding: "8px 8px",
              fontSize: "var(--font-size-base)",
              background: "var(--bg-tertiary)",
              color: "var(--text-primary)",
              border: "1px solid var(--border-color)",
              borderRadius: "var(--radius-xs-plus)",
            }}
          />
          <button className="panel-btn"
            onClick={handleJoin}
            disabled={loading || !joinRoomId.trim() || !userName.trim()}
            style={{
              padding: "8px 16px",
              background: "var(--success-color)",
              color: "var(--btn-primary-fg)",
              border: "none",
              borderRadius: "var(--radius-xs-plus)",
              cursor: loading ? "wait" : "pointer",
              fontSize: "var(--font-size-base)",
              fontWeight: 600,
              opacity: loading || !joinRoomId.trim() ? 0.6 : 1,
            }}
          >
            Join
          </button>
        </div>
      </div>

      <div
        style={{
          marginTop: "20px",
          padding: "8px",
          background: "var(--bg-tertiary)",
          borderRadius: "var(--radius-xs-plus)",
          fontSize: "var(--font-size-sm)",
          color: "var(--text-secondary)",
          lineHeight: 1.5,
        }}
      >
        <strong>Prerequisites:</strong>
        <br />
        1. Start the VibeCLI daemon: <code>vibecli --serve --port {daemonPort}</code>
        <br />
        2. Both users connect to the same daemon instance.
        <br />
        3. Changes sync in real-time via WebSocket + CRDT.
      </div>
    </div>
  );
}
