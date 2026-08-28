/**
 * A spoken turn has to end up in the conversation — and the conversation has to
 * survive being switched.
 *
 * Two failures, one test file. VibeDesk first mounted the duplex hook with no
 * `onTurn`, so a whole spoken exchange happened with nothing written down. Then
 * the hook sat inside the composer, which `ShellLayout` remounts on every chat
 * switch (`key={chatNonce}`) and replaces for every full-screen overlay: its
 * teardown closed the socket and the microphone, and the daemon's history —
 * which is per socket — went with them. Clicking a chat ended the conversation
 * mid-sentence, silently.
 *
 * So the session lives in `useVoiceSession`, above that line, and the pane that
 * shows the conversation registers itself as where turns are written. These pin
 * both halves.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { renderHook, act } from "@testing-library/react";
import type { UseVoiceDuplexOptions } from "@vibe/shared/voice/useVoiceDuplex";
import { LOCKED_SANDBOX } from "../../lib/sandbox";
import { voiceSessionStub } from "./voiceSessionStub";

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
vi.mock("../../hooks/useProjectFiles", async (orig) => ({
  ...(await orig<typeof import("../../hooks/useProjectFiles")>()),
  useProjectFiles: () => [],
}));

/** The options the session handed the hook on its last render. */
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
import { useVoiceSession } from "../../hooks/useVoiceSession";

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
    projectFiles: [],
    voice: voiceSessionStub(),
    ...overrides,
  };
}

const session = () =>
  renderHook(() =>
    useVoiceSession({
      daemonUrl: "http://127.0.0.1:7878",
      daemonOnline: true,
      root: "/repo",
      provider: "ollama",
      model: "gpt-oss:120b-cloud",
    }),
  );

beforeEach(() => {
  lastOpts = null;
});

describe("the voice session", () => {
  it("forwards both halves of a spoken exchange to the registered pane", () => {
    const { result } = session();
    const sink = vi.fn();
    act(() => result.current.registerSink(sink));

    lastOpts?.onTurn?.({ role: "user", text: "what does this build?" });
    lastOpts?.onTurn?.({ role: "assistant", text: "A Rust daemon and its clients." });

    expect(sink.mock.calls).toEqual([
      ["user", "what does this build?"],
      ["assistant", "A Rust daemon and its clients."],
    ]);
  });

  it("writes to whichever pane registered last, not the one that has gone", () => {
    // A chat switch replaces the pane. The turn belongs to the conversation on
    // screen; delivering it to the unmounted one would put a spoken answer in
    // a chat the user has left.
    const { result } = session();
    const first = vi.fn();
    const second = vi.fn();
    act(() => result.current.registerSink(first));
    act(() => result.current.registerSink(second));

    lastOpts?.onTurn?.({ role: "assistant", text: "still here" });

    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledWith("assistant", "still here");
  });

  it("uses the composer's own provider and model, never a default", () => {
    // The provider-agnostic rule: the voice conversation answers on whatever
    // the composer would have used for a typed message — unless the daemon's
    // own voice setting overrides it, which is the daemon's decision to make.
    session();
    expect(lastOpts?.provider).toBe("ollama");
    expect(lastOpts?.model).toBe("gpt-oss:120b-cloud");
  });

  it("survives the composer being remounted", () => {
    // The composer is what a chat switch throws away. Nothing about the
    // conversation may be inside it.
    const { result } = session();
    const sink = vi.fn();
    act(() => result.current.registerSink(sink));

    const view = render(<TaskPrompt {...props({ voice: result.current })} />);
    view.unmount();

    lastOpts?.onTurn?.({ role: "assistant", text: "still talking" });
    expect(sink).toHaveBeenCalledWith("assistant", "still talking");
  });
});
