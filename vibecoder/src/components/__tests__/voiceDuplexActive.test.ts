/**
 * One bad turn must not strand the microphone.
 *
 * The daemon reports a *turn* failure — no speech engine, a provider error, a
 * reply that was all reasoning — as `{type: "error"}` and leaves the socket
 * open, because the conversation is still live and the next thing you say is a
 * new turn. `active` was derived from the status alone, so a single such error
 * told the button the conversation had ended: it offered "start", `start`
 * returned immediately because a socket was already open, and no control could
 * stop the microphone any more.
 *
 * The socket is faked wholesale. The point under test is state bookkeeping, not
 * audio, so the audio stack is stubbed to the minimum the hook touches.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({
  // No shell here: the hook falls back to an unauthenticated local daemon.
  invoke: vi.fn(async () => {
    throw new Error("no tauri host");
  }),
}));

import { useVoiceDuplex } from "@vibe/shared/voice/useVoiceDuplex";

/** The socket the hook opened, so a test can push daemon messages into it. */
let socket: FakeSocket | null = null;

class FakeSocket {
  static OPEN = 1;
  readyState = 1;
  binaryType = "";
  onopen: (() => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  constructor(public url: string) {
    socket = this;
  }
  send = vi.fn();
  close = vi.fn();
  /** Deliver one daemon message, as the socket would. */
  say(v: unknown) {
    this.onmessage?.({ data: JSON.stringify(v) });
  }
}

class FakeAudioContext {
  currentTime = 0;
  destination = {};
  audioWorklet = {
    // Force the ScriptProcessor path: `script-src 'self'` rejects a blob:
    // worklet module in the real shells too.
    addModule: vi.fn(async () => {
      throw new Error("blocked by CSP");
    }),
  };
  resume = vi.fn(async () => {});
  close = vi.fn(async () => {});
  createMediaStreamSource = () => ({ connect: vi.fn() });
  createGain = () => ({ gain: { value: 1 }, connect: vi.fn() });
  createScriptProcessor = () => ({ onaudioprocess: null, connect: vi.fn() });
}

const track = { stop: vi.fn(), getSettings: () => ({ echoCancellation: true }) };

beforeEach(() => {
  socket = null;
  vi.stubGlobal("WebSocket", FakeSocket);
  vi.stubGlobal("AudioContext", FakeAudioContext);
  vi.stubGlobal("AudioWorkletNode", class {});
  vi.stubGlobal("navigator", {
    mediaDevices: {
      getUserMedia: vi.fn(async () => ({
        getAudioTracks: () => [track],
        getTracks: () => [track],
      })),
    },
  });
  vi.stubGlobal("URL", {
    createObjectURL: () => "blob:fake",
    revokeObjectURL: vi.fn(),
  });
});

afterEach(() => vi.unstubAllGlobals());

/** Start a conversation and settle the socket into its open state. */
async function open() {
  const hook = renderHook(() => useVoiceDuplex({ enabled: true, provider: "ollama" }));
  await act(async () => {
    await hook.result.current.start();
  });
  await act(async () => {
    socket?.onopen?.();
  });
  return hook;
}

describe("useVoiceDuplex — a turn-level error keeps the conversation stoppable", () => {
  it("stays active after the daemon reports a failed turn", async () => {
    const hook = await open();
    await waitFor(() => expect(hook.result.current.active).toBe(true));

    await act(async () => {
      socket?.say({
        type: "error",
        message: "The model produced only reasoning and no answer.",
      });
    });

    // The status says what went wrong…
    expect(hook.result.current.state).toEqual({
      status: "error",
      message: "The model produced only reasoning and no answer.",
    });
    // …and the microphone is still open, so the button must still offer Stop.
    expect(hook.result.current.active).toBe(true);
  });

  it("goes inactive once the conversation is stopped", async () => {
    const hook = await open();
    await waitFor(() => expect(hook.result.current.active).toBe(true));

    await act(async () => hook.result.current.stop());

    expect(hook.result.current.active).toBe(false);
    expect(hook.result.current.state).toEqual({ status: "idle" });
  });

  it("is inactive when the start itself failed", async () => {
    // A denied microphone leaves no socket, so the only useful control is the
    // one that tries again.
    (navigator.mediaDevices.getUserMedia as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
      Object.assign(new Error("denied"), { name: "NotAllowedError" }),
    );
    const hook = renderHook(() => useVoiceDuplex({ enabled: true, provider: "ollama" }));
    await act(async () => {
      await hook.result.current.start();
    });

    expect(hook.result.current.state.status).toBe("error");
    expect(hook.result.current.active).toBe(false);
  });
});
