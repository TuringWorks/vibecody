import { describe, it, expect } from "vitest";
import { reduceTurns, type DuplexEvent, type DuplexTurn } from "@vibe/shared/voice/useVoiceDuplex";

/** Replay a socket transcript through the fold, as the hook does. */
const play = (events: DuplexEvent[]): DuplexTurn[] =>
  events.reduce<DuplexTurn[]>((turns, ev) => reduceTurns(turns, ev), []);

describe("duplex turn list", () => {
  it("renders a multi-sentence reply as exactly one assistant turn", () => {
    // The daemon emits `speaking` per sentence because that is what drives
    // streaming TTS. Appending each one gave the user a chat log where a
    // single answer arrived as three separate bubbles.
    const turns = play([
      { type: "flush" },
      { type: "transcript", text: "what is this project?" },
      { type: "speaking", text: "It is a Rust workspace." },
      { type: "speaking", text: "The daemon lives in vibecli." },
      { type: "speaking", text: "There are three desktop shells." },
      {
        type: "reply",
        text: "It is a Rust workspace. The daemon lives in vibecli. There are three desktop shells.",
      },
    ]);

    expect(turns).toHaveLength(2);
    expect(turns[0]).toEqual({ role: "user", text: "what is this project?", lang: undefined });
    expect(turns[1].role).toBe("assistant");
    expect(turns[1].interim).toBeFalsy();
    expect(turns.filter((t) => t.role === "assistant")).toHaveLength(1);
  });

  it("finalises with the model's own text, not the sentences glued together", () => {
    // The sentence splitter consumes the whitespace it splits on, so
    // re-joining is lossy — punctuation and spacing come back wrong.
    const turns = play([
      { type: "speaking", text: "Yes." },
      { type: "speaking", text: "Two files changed." },
      { type: "reply", text: "Yes.  Two files changed." },
    ]);
    expect(turns[0].text).toBe("Yes.  Two files changed.");
  });

  it("shows sentences as they arrive, marked interim until the reply lands", () => {
    const mid = play([
      { type: "transcript", text: "hello" },
      { type: "speaking", text: "Hi there." },
    ]);
    expect(mid[1]).toEqual({ role: "assistant", text: "Hi there.", interim: true });
  });

  it("keeps a barged-in answer but does not let the next one continue it", () => {
    const turns = play([
      { type: "transcript", text: "explain the daemon" },
      { type: "speaking", text: "The daemon is the source of" },
      // The user talks over it: the daemon bumps its generation, sends
      // `flush`, and no `reply` ever arrives for that turn.
      { type: "flush" },
      { type: "transcript", text: "never mind, what time is it?" },
      { type: "speaking", text: "It is half past four." },
      { type: "reply", text: "It is half past four." },
    ]);

    expect(turns.map((t) => t.role)).toEqual(["user", "assistant", "user", "assistant"]);
    expect(turns[1].text).toBe("The daemon is the source of");
    expect(turns[1].interim).toBe(false);
    expect(turns[3].text).toBe("It is half past four.");
  });

  it("does not leave an empty bubble when the model returns nothing", () => {
    expect(play([{ type: "transcript", text: "hi" }, { type: "reply", text: "" }])).toHaveLength(1);
  });

  it("keeps what was spoken when the reply arrives empty", () => {
    const turns = play([{ type: "speaking", text: "Yes." }, { type: "reply", text: "   " }]);
    expect(turns).toEqual([{ role: "assistant", text: "Yes." }]);
  });

  it("never mutates the list it was given", () => {
    const before: DuplexTurn[] = [{ role: "assistant", text: "One.", interim: true }];
    const snapshot = structuredClone(before);
    reduceTurns(before, { type: "speaking", text: "Two." });
    expect(before).toEqual(snapshot);
  });
});
