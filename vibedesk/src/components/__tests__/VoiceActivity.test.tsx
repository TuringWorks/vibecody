/**
 * The caption explains a pause the user can otherwise only hear as silence.
 *
 * A spoken turn that stops to read a file is several seconds of nothing, and
 * nothing is exactly what a dead microphone sounds like. When the daemon says
 * what it is doing ("Reading README.md"), that outranks the generic
 * "Thinking…" — it is the difference between a working feature and a broken
 * one, from the only side the user can see.
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VoiceTranscript } from "@vibe/shared/voice/VoiceTranscript";

describe("voice caption during a tool pause", () => {
  it("says what is being read instead of just Thinking", () => {
    render(
      <VoiceTranscript
        state={{ status: "thinking" }}
        turns={[{ role: "user", text: "summarise the project" }]}
        activity="Reading README.md"
        active
      />
    );
    expect(screen.getByText("Reading README.md")).toBeInTheDocument();
    expect(screen.queryByText("Thinking…")).toBeNull();
  });

  it("falls back to the state when nothing is being read", () => {
    render(
      <VoiceTranscript
        state={{ status: "thinking" }}
        turns={[{ role: "user", text: "hello" }]}
        activity={null}
        active
      />
    );
    expect(screen.getByText("Thinking…")).toBeInTheDocument();
  });
});
