import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useVoiceDuplexPreference } from "@vibe/shared/voice/useVoiceDuplexPreference";

/**
 * A feature that opens a microphone must be something a person turned on.
 * "Idle until clicked" is not the same promise — it leaves a live control one
 * misclick from an open mic.
 */
describe("useVoiceDuplexPreference", () => {
  beforeEach(() => localStorage.clear());

  it("is off when nothing has been stored", () => {
    const { result } = renderHook(() => useVoiceDuplexPreference());
    expect(result.current.enabled).toBe(false);
  });

  it("is off for any stored value that is not exactly true", () => {
    // Anything ambiguous must read as off. A microphone is not the place to be
    // generous about what counts as consent.
    for (const v of ["", "false", "1", "yes", "TRUE", "null"]) {
      localStorage.setItem("vibe.voice.duplexEnabled", v);
      const { result } = renderHook(() => useVoiceDuplexPreference());
      expect(result.current.enabled, `stored ${JSON.stringify(v)}`).toBe(false);
    }
  });

  it("persists an explicit opt-in and reads it back", () => {
    const { result } = renderHook(() => useVoiceDuplexPreference());
    act(() => result.current.setEnabled(true));
    expect(result.current.enabled).toBe(true);
    expect(localStorage.getItem("vibe.voice.duplexEnabled")).toBe("true");

    const second = renderHook(() => useVoiceDuplexPreference());
    expect(second.result.current.enabled).toBe(true);
  });

  it("propagates a change to another hook in the same window", () => {
    // `storage` only fires in *other* windows, so a chat panel and a composer
    // in one window would otherwise disagree about whether voice is on.
    const a = renderHook(() => useVoiceDuplexPreference());
    const b = renderHook(() => useVoiceDuplexPreference());
    act(() => a.result.current.setEnabled(true));
    expect(b.result.current.enabled).toBe(true);
    act(() => a.result.current.setEnabled(false));
    expect(b.result.current.enabled).toBe(false);
  });
});
