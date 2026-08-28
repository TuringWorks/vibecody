import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

// The hook resolves the daemon's port and token through Tauri, the same way
// useVoiceDuplex does. Neither exists under jsdom.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => (cmd === "daemon_port" ? 7878 : "tok")),
}));

import { useVoiceSettings } from "@vibe/shared/voice/useVoiceSettings";

const SETTINGS = {
  engine: "system",
  engines: [
    { id: "system", label: "System", available: true, detail: "The platform voice." },
    { id: "kokoro", label: "Neural (Kokoro)", available: false, detail: "Not installed — run: make voice-kokoro" },
  ],
  voice: "af_heart",
  language: "auto",
  voices: [{ id: "af_heart", name: "Heart", lang: "en", quality: "neural" }],
  languages: ["en", "hi"],
};

let fetchMock: ReturnType<typeof vi.fn>;
beforeEach(() => {
  fetchMock = vi.fn(async () => new Response(JSON.stringify(SETTINGS), { status: 200 }));
  vi.stubGlobal("fetch", fetchMock);
});
afterEach(() => vi.unstubAllGlobals());

describe("useVoiceSettings", () => {
  it("reads the daemon's settings on mount", async () => {
    const { result } = renderHook(() => useVoiceSettings());
    await waitFor(() => expect(result.current.settings).not.toBeNull());
    expect(result.current.settings?.engine).toBe("system");
    expect(result.current.settings?.engines[1].available).toBe(false);
  });

  it("keeps the daemon's own words when it refuses", async () => {
    // "The neural engine is not installed. Run: make voice-kokoro" is the whole
    // answer. Replacing it with "daemon returned 400" throws away the only part
    // that tells the user what to do.
    const { result } = renderHook(() => useVoiceSettings());
    await waitFor(() => expect(result.current.settings).not.toBeNull());

    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ error: "The neural engine is not installed. Run: make voice-kokoro" }), { status: 400 }),
    );
    await act(async () => { await result.current.update({ engine: "kokoro" }); });
    expect(result.current.error).toContain("make voice-kokoro");
    expect(result.current.error).not.toContain("400");
  });

  it("re-reads after a change rather than trusting the patch", async () => {
    // Changing the engine changes which voices exist, and the daemon is the one
    // that knows them — echoing the patch back would leave a voice list that
    // belongs to the previous engine.
    const { result } = renderHook(() => useVoiceSettings());
    await waitFor(() => expect(result.current.settings).not.toBeNull());
    const before = fetchMock.mock.calls.length;
    await act(async () => { await result.current.update({ language: "hi" }); });
    expect(fetchMock.mock.calls.length).toBe(before + 2); // the PUT, then the re-read
    expect(fetchMock.mock.calls[before][1]?.method).toBe("PUT");
  });

  it("names the daemon when it cannot be reached", async () => {
    // "Failed to fetch" sends people to their network settings for a process
    // that is simply not running.
    fetchMock.mockRejectedValueOnce(new TypeError("Failed to fetch"));
    const { result } = renderHook(() => useVoiceSettings());
    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.error).toMatch(/daemon/i);
  });
});
