/**
 * useCollab — React hook for CRDT multiplayer collaboration.
 *
 * Manages a Y.Doc, y-websocket provider, awareness state, and connection lifecycle.
 */

import { useState, useEffect, useRef, useCallback } from "react";

// Types for the collab state
export interface CollabPeer {
  peerId: string;
  name: string;
  color: string;
  cursor?: {
    file: string;
    line: number;
    column: number;
    selectionEnd?: [number, number];
  };
}

export interface CollabState {
  connected: boolean;
  roomId: string | null;
  peerId: string | null;
  peers: CollabPeer[];
  wsUrl: string | null;
}

interface CollabMessage {
  type: string;
  room_id?: string;
  peer_id?: string;
  peers?: Array<{ peer_id: string; name: string; color: string }>;
  peer?: { peer_id: string; name: string; color: string };
  message?: string;
  cursor?: { file: string; line: number; column: number; selection_end?: [number, number] };
  timestamp?: number;
}

export function useCollab() {
  const [state, setState] = useState<CollabState>({
    connected: false,
    roomId: null,
    peerId: null,
    peers: [],
    wsUrl: null,
  });

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<number | null>(null);

  const disconnect = useCallback(() => {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    setState({
      connected: false,
      roomId: null,
      peerId: null,
      peers: [],
      wsUrl: null,
    });
  }, []);

  const connect = useCallback(
    (wsUrl: string, userName: string) => {
      // Close existing connection
      if (wsRef.current) {
        wsRef.current.close();
      }

      const ws = new WebSocket(wsUrl + `&name=${encodeURIComponent(userName)}`);
      wsRef.current = ws;

      ws.binaryType = "arraybuffer";

      ws.onopen = () => {
        setState((prev) => ({ ...prev, connected: true, wsUrl }));
      };

      ws.onmessage = (event) => {
        if (typeof event.data === "string") {
          // JSON text message
          try {
            const msg: CollabMessage = JSON.parse(event.data);
            handleTextMessage(msg);
          } catch {
            // Ignore malformed messages
          }
        }
        // Binary messages (Yjs sync) are handled by y-websocket provider directly
        // when integrated with Monaco. For now we just log them.
      };

      ws.onclose = () => {
        // Only update state if this is still the active websocket;
        // a new connect() call may have already replaced it.
        if (wsRef.current === ws) {
          setState((prev) => ({ ...prev, connected: false }));
          wsRef.current = null;
        }
      };

      ws.onerror = () => {
        ws.close();
      };
    },
    []
  );

  const handleTextMessage = useCallback((msg: CollabMessage) => {
    switch (msg.type) {
      case "welcome":
        setState((prev) => ({
          ...prev,
          roomId: msg.room_id ?? null,
          peerId: msg.peer_id ?? null,
          peers:
            msg.peers?.map((p) => ({
              peerId: p.peer_id,
              name: p.name,
              color: p.color,
            })) ?? [],
        }));
        break;

      case "peer_joined":
        if (msg.peer) {
          setState((prev) => ({
            ...prev,
            peers: [
              ...prev.peers.filter((p) => p.peerId !== msg.peer!.peer_id),
              {
                peerId: msg.peer!.peer_id,
                name: msg.peer!.name,
                color: msg.peer!.color,
              },
            ],
          }));
        }
        break;

      case "peer_left":
        if (msg.peer_id) {
          setState((prev) => ({
            ...prev,
            peers: prev.peers.filter((p) => p.peerId !== msg.peer_id),
          }));
        }
        break;

      case "awareness":
        if (msg.peer_id && msg.cursor) {
          setState((prev) => ({
            ...prev,
            peers: prev.peers.map((p) =>
              p.peerId === msg.peer_id
                ? {
                    ...p,
                    cursor: {
                      file: msg.cursor!.file,
                      line: msg.cursor!.line,
                      column: msg.cursor!.column,
                      selectionEnd: msg.cursor!.selection_end,
                    },
                  }
                : p
            ),
          }));
        }
        break;

      case "error":
        console.error("[collab] Server error:", msg.message);
        break;
    }
  }, []);

  const sendAwareness = useCallback(
    (file: string, line: number, column: number, selectionEnd?: [number, number]) => {
      const ws = wsRef.current;
      if (!ws || ws.readyState !== WebSocket.OPEN || !state.peerId) return;
      const msg = {
        type: "awareness",
        peer_id: state.peerId,
        cursor: {
          file,
          line,
          column,
          selection_end: selectionEnd,
        },
        timestamp: Date.now(),
      };
      ws.send(JSON.stringify(msg));
    },
    [state.peerId]
  );

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      disconnect();
    };
  }, [disconnect]);

  return {
    ...state,
    connect,
    disconnect,
    sendAwareness,
    ws: wsRef.current,
  };
}
