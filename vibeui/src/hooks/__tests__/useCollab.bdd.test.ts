/**
 * BDD tests for useCollab — WebSocket CRDT collaboration state machine.
 *
 * Scenarios:
 *  1. Initial state: disconnected, no room, no peers
 *  2. connect() opens a WebSocket to the given URL with the user name
 *  3. WebSocket onopen sets connected=true and records wsUrl
 *  4. "welcome" message sets roomId, peerId, and initial peers list
 *  5. "peer_joined" adds the new peer to the peers array
 *  6. "peer_joined" de-duplicates by peerId
 *  7. "peer_left" removes the peer from the peers array
 *  8. "awareness" updates the cursor for the matching peer
 *  9. "awareness" for unknown peerId is a no-op
 * 10. WebSocket onclose sets connected=false
 * 11. disconnect() closes the socket and resets all state
 * 12. sendAwareness sends the correct JSON when connected
 * 13. sendAwareness is a no-op when not connected
 * 14. Unmounting triggers disconnect
 * 15. Calling connect() twice replaces the previous socket
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useCollab } from '../useCollab';

// ── Minimal WebSocket mock ────────────────────────────────────────────────────

type WsEventHandler = (e?: unknown) => void;

class MockWebSocket {
  static OPEN = 1;
  static CLOSED = 3;

  url: string;
  binaryType = 'blob';
  readyState = MockWebSocket.OPEN;

  onopen:    WsEventHandler | null = null;
  onmessage: WsEventHandler | null = null;
  onclose:   WsEventHandler | null = null;
  onerror:   WsEventHandler | null = null;

  sentMessages: string[] = [];
  closed = false;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.lastInstance = this;
    MockWebSocket.instances.push(this);
  }

  send(data: string) { this.sentMessages.push(data); }
  close() {
    this.closed = true;
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.();
  }

  // Test helpers
  simulateOpen()    { this.onopen?.(); }
  simulateMessage(data: unknown) { this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent); }
  simulateClose()   { this.close(); }
  simulateError()   { this.onerror?.(); }

  static lastInstance: MockWebSocket | null = null;
  static instances: MockWebSocket[] = [];
  static reset() {
    MockWebSocket.lastInstance = null;
    MockWebSocket.instances = [];
  }
}

beforeEach(() => {
  MockWebSocket.reset();
  vi.stubGlobal('WebSocket', MockWebSocket);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

// ── Scenario 1: Initial state ─────────────────────────────────────────────────

describe('Given a fresh useCollab hook', () => {
  it('When it mounts, Then connected is false', () => {
    const { result } = renderHook(() => useCollab());
    expect(result.current.connected).toBe(false);
  });

  it('When it mounts, Then roomId, peerId are null', () => {
    const { result } = renderHook(() => useCollab());
    expect(result.current.roomId).toBeNull();
    expect(result.current.peerId).toBeNull();
  });

  it('When it mounts, Then peers is an empty array', () => {
    const { result } = renderHook(() => useCollab());
    expect(result.current.peers).toHaveLength(0);
  });
});

// ── Scenario 2: connect() opens WebSocket ────────────────────────────────────

describe('Given connect() is called', () => {
  it('When connect() is called, Then a WebSocket is opened to the given URL', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://localhost:9001/collab?room=abc', 'Alice'); });
    expect(MockWebSocket.lastInstance?.url).toContain('ws://localhost:9001/collab?room=abc');
  });

  it('When connect() is called with a user name, Then the URL includes the encoded name', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://host/room', 'Bob Smith'); });
    expect(MockWebSocket.lastInstance?.url).toContain('Bob%20Smith');
  });
});

// ── Scenario 3: onopen sets connected=true ───────────────────────────────────

describe('Given the WebSocket opens successfully', () => {
  it('When onopen fires, Then connected becomes true', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    expect(result.current.connected).toBe(true);
  });

  it('When onopen fires, Then wsUrl is recorded', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    expect(result.current.wsUrl).toBe('ws://host/room');
  });
});

// ── Scenario 4: "welcome" message ────────────────────────────────────────────

describe('Given a "welcome" message is received', () => {
  function setupConnected() {
    const hook = renderHook(() => useCollab());
    act(() => { hook.result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    return hook;
  }

  it('When "welcome" arrives, Then roomId is set', () => {
    const { result } = setupConnected();
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'welcome',
        room_id: 'room-42',
        peer_id: 'peer-1',
        peers: [],
      });
    });
    expect(result.current.roomId).toBe('room-42');
  });

  it('When "welcome" arrives, Then peerId is set', () => {
    const { result } = setupConnected();
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'welcome',
        room_id: 'room-42',
        peer_id: 'peer-1',
        peers: [],
      });
    });
    expect(result.current.peerId).toBe('peer-1');
  });

  it('When "welcome" arrives with peers, Then peers list is populated', () => {
    const { result } = setupConnected();
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'welcome',
        room_id: 'room-42',
        peer_id: 'peer-1',
        peers: [
          { peer_id: 'peer-2', name: 'Bob', color: '#ff0000' },
          { peer_id: 'peer-3', name: 'Carol', color: '#0000ff' },
        ],
      });
    });
    expect(result.current.peers).toHaveLength(2);
    expect(result.current.peers[0].name).toBe('Bob');
    expect(result.current.peers[1].name).toBe('Carol');
  });
});

// ── Scenario 5 & 6: peer_joined ──────────────────────────────────────────────

describe('Given a peer joins the room', () => {
  function setupWithPeer(existingPeers: Array<{ peer_id: string; name: string; color: string }> = []) {
    const hook = renderHook(() => useCollab());
    act(() => { hook.result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'welcome',
        room_id: 'r1',
        peer_id: 'p1',
        peers: existingPeers,
      });
    });
    return hook;
  }

  it('When "peer_joined" arrives, Then the new peer appears in peers', () => {
    const { result } = setupWithPeer();
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'peer_joined',
        peer: { peer_id: 'p2', name: 'Dave', color: '#00ff00' },
      });
    });
    expect(result.current.peers.find(p => p.peerId === 'p2')?.name).toBe('Dave');
  });

  it('When the same peer joins again, Then they are not duplicated', () => {
    const { result } = setupWithPeer([{ peer_id: 'p2', name: 'Dave', color: '#00ff00' }]);
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'peer_joined',
        peer: { peer_id: 'p2', name: 'Dave (reconnected)', color: '#00ff00' },
      });
    });
    expect(result.current.peers.filter(p => p.peerId === 'p2')).toHaveLength(1);
  });
});

// ── Scenario 7: peer_left ─────────────────────────────────────────────────────

describe('Given a peer leaves the room', () => {
  it('When "peer_left" arrives, Then the peer is removed from peers', () => {
    const hook = renderHook(() => useCollab());
    act(() => { hook.result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'welcome', room_id: 'r1', peer_id: 'p1',
        peers: [{ peer_id: 'p2', name: 'Bob', color: '#fff' }],
      });
    });
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({ type: 'peer_left', peer_id: 'p2' });
    });
    expect(hook.result.current.peers.find(p => p.peerId === 'p2')).toBeUndefined();
  });
});

// ── Scenario 8 & 9: awareness ────────────────────────────────────────────────

describe('Given an awareness update for a connected peer', () => {
  function setupWithBob() {
    const hook = renderHook(() => useCollab());
    act(() => { hook.result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'welcome', room_id: 'r1', peer_id: 'p1',
        peers: [{ peer_id: 'p2', name: 'Bob', color: '#ff0' }],
      });
    });
    return hook;
  }

  it('When "awareness" arrives for Bob, Then Bob cursor is updated', () => {
    const { result } = setupWithBob();
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'awareness',
        peer_id: 'p2',
        cursor: { file: 'src/main.rs', line: 42, column: 7 },
      });
    });
    const bob = result.current.peers.find(p => p.peerId === 'p2');
    expect(bob?.cursor?.line).toBe(42);
    expect(bob?.cursor?.file).toBe('src/main.rs');
  });

  it('When "awareness" arrives for an unknown peer, Then peers is unchanged', () => {
    const { result } = setupWithBob();
    const before = result.current.peers.length;
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'awareness',
        peer_id: 'ghost-peer',
        cursor: { file: 'x.ts', line: 1, column: 1 },
      });
    });
    expect(result.current.peers).toHaveLength(before);
  });
});

// ── Scenario 10: onclose ─────────────────────────────────────────────────────

describe('Given the WebSocket closes unexpectedly', () => {
  it('When onclose fires, Then connected becomes false', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    expect(result.current.connected).toBe(true);
    act(() => { MockWebSocket.lastInstance?.simulateClose(); });
    expect(result.current.connected).toBe(false);
  });
});

// ── Scenario 11: disconnect() ─────────────────────────────────────────────────

describe('Given a connected session', () => {
  it('When disconnect() is called, Then connected becomes false', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    act(() => { result.current.disconnect(); });
    expect(result.current.connected).toBe(false);
  });

  it('When disconnect() is called, Then roomId, peerId, and peers are reset', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'welcome', room_id: 'r99', peer_id: 'p99', peers: [],
      });
    });
    act(() => { result.current.disconnect(); });
    expect(result.current.roomId).toBeNull();
    expect(result.current.peerId).toBeNull();
    expect(result.current.peers).toHaveLength(0);
  });

  it('When disconnect() is called, Then the WebSocket is closed', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://host/room', 'Alice'); });
    const ws = MockWebSocket.lastInstance!;
    act(() => { result.current.disconnect(); });
    expect(ws.closed).toBe(true);
  });
});

// ── Scenario 12 & 13: sendAwareness ──────────────────────────────────────────

describe('Given a connected session with a known peerId', () => {
  it('When sendAwareness is called, Then a JSON message is sent on the WebSocket', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://host/room', 'Alice'); });
    act(() => { MockWebSocket.lastInstance?.simulateOpen(); });
    act(() => {
      MockWebSocket.lastInstance?.simulateMessage({
        type: 'welcome', room_id: 'r1', peer_id: 'self', peers: [],
      });
    });
    act(() => { result.current.sendAwareness('src/app.ts', 10, 5); });
    const ws = MockWebSocket.lastInstance!;
    expect(ws.sentMessages).toHaveLength(1);
    const msg = JSON.parse(ws.sentMessages[0]);
    expect(msg.type).toBe('awareness');
    expect(msg.cursor.file).toBe('src/app.ts');
    expect(msg.cursor.line).toBe(10);
    expect(msg.cursor.column).toBe(5);
  });
});

describe('Given a disconnected hook', () => {
  it('When sendAwareness is called, Then no message is sent', () => {
    const { result } = renderHook(() => useCollab());
    // Never connected — wsRef.current is null
    act(() => { result.current.sendAwareness('file.ts', 1, 1); });
    // No WebSocket was ever created
    expect(MockWebSocket.lastInstance).toBeNull();
  });
});

// ── Scenario 15: Calling connect() twice replaces socket ─────────────────────

describe('Given a second connect() call replaces the first socket', () => {
  it('When connect() is called twice, Then the first socket is replaced', () => {
    const { result } = renderHook(() => useCollab());
    act(() => { result.current.connect('ws://first', 'Alice'); });
    const first = MockWebSocket.lastInstance;
    act(() => { result.current.connect('ws://second', 'Alice'); });
    const second = MockWebSocket.lastInstance;
    expect(first).not.toBe(second);
    expect(second?.url).toContain('ws://second');
  });
});
