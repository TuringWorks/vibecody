/**
 * A spoken turn has to end up in the conversation.
 *
 * VibeDesk mounted the duplex hook without an `onTurn`, so a whole voice
 * conversation — the question and the answer — happened with nothing written
 * down anywhere. The two other shells passed one; this is the guard that stops
 * this one from losing it again.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import type { UseVoiceDuplexOptions } from "@vibe/shared/voice/useVoiceDuplex";
import { LOCKED_SANDBOX } from "../../lib/sandbox";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => []) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => null) }));
vi.mock("@vibe/shared/voice/useVoiceInput", () => ({
  useVoiceInput: () => ({
    supported: true,
    isListening: false,
    isTranscribing: false,
    error: null,
    interimText: "",
    toggle: vi.fn(),
  }),
}));

/** The options the composer handed the hook on its last render. */
let lastOpts: UseVoiceDuplexOptions | null = null;
vi.mock("@vibe/shared/voice/useVoiceDuplex", () => ({
  useVoiceDuplex: (opts: UseVoiceDuplexOptions) => {
    lastOpts = opts;
    return {
      state: { status: "idle" },
      turns: [],
      latency: {},
      active: false,
      supported: true,
      start: vi.fn(),
      stop: vi.fn(),
      setVoice: vi.fn(),
      setLanguage: vi.fn(),
    };
  },
}));

import { TaskPrompt } from "../TaskPrompt";

type Props = React.ComponentProps<typeof TaskPrompt>;

function props(overrides: Partial<Props> = {}): Props {
  return {
    daemonUrl: "http://127.0.0.1:7878",
    daemonOnline: true,
    busy: false,
    prefs: {
      provider: "ollama",
      model: "gpt-oss:120b-cloud",
      approval: "default",
      reasoning: "medium",
      mode: "agent",
      sandbox: LOCKED_SANDBOX,
      isolate: false,
    },
    onPref: vi.fn(),
    onProviderModel: vi.fn(),
    draft: "",
    onDraft: vi.fn(),
    attachments: [],
    onAttachments: vi.fn(),
    onSubmit: vi.fn(),
    onStop: vi.fn(),
    onQuickAction: vi.fn(),
    onSlash: vi.fn(),
    onVoiceTurn: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  lastOpts = null;
});

describe("TaskPrompt — voice turns reach the conversation", () => {
  it("forwards both halves of a spoken exchange", () => {
    const onVoiceTurn = vi.fn();
    render(<TaskPrompt {...props({ onVoiceTurn })} />);

    expect(lastOpts?.onTurn).toBeTypeOf("function");
    lastOpts?.onTurn?.({ role: "user", text: "what does this build?" });
    lastOpts?.onTurn?.({ role: "assistant", text: "A Rust daemon and its clients." });

    expect(onVoiceTurn.mock.calls).toEqual([
      ["user", "what does this build?"],
      ["assistant", "A Rust daemon and its clients."],
    ]);
  });

  it("uses the composer's own provider and model, never a default", () => {
    // The provider-agnostic rule: the voice conversation answers on whatever
    // the composer would have used for a typed message.
    render(<TaskPrompt {...props()} />);
    expect(lastOpts?.provider).toBe("ollama");
    expect(lastOpts?.model).toBe("gpt-oss:120b-cloud");
  });
});
