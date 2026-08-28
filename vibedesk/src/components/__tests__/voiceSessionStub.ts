import { vi } from "vitest";
import type { VoiceSession } from "../../hooks/useVoiceSession";

/**
 * An inert voice session for composer tests.
 *
 * The session is owned by the shell rather than the composer — a microphone
 * must survive a chat switch, and the composer's subtree is remounted on every
 * one — so rendering the composer means handing it one.
 */
export function voiceSessionStub(overrides: Partial<VoiceSession> = {}): VoiceSession {
  return {
    duplex: {
      state: { status: "idle" },
      turns: [],
      notice: null,
      activity: null,
      latency: {},
      approval: null,
      respondToApproval: vi.fn(),
      supported: true,
      active: false,
      start: vi.fn(async () => {}),
      stop: vi.fn(),
      setVoice: vi.fn(),
      setLanguage: vi.fn(),
    },
    enabled: false,
    setEnabled: vi.fn(),
    files: [],
    registerSink: vi.fn(),
    ...overrides,
  };
}
