/**
 * The live caption over the composer.
 *
 * The bug it exists for: a spoken conversation showed nothing on screen. The
 * microphone heard a question, the speaker answered it, and the window was
 * blank the whole time — so a failure anywhere in the chain (no speech engine,
 * a model that never replied, audio playing at the wrong pitch) looked exactly
 * like a working feature with nothing to say.
 */
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { VoiceTranscript } from "@vibe/shared/voice/VoiceTranscript";
import type { DuplexTurn } from "@vibe/shared/voice/useVoiceDuplex";

const turns = (...t: DuplexTurn[]) => t;

describe("VoiceTranscript", () => {
  it("shows nothing while no conversation is running", () => {
    const { container } = render(
      <VoiceTranscript state={{ status: "idle" }} turns={[]} active={false} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("says it is listening before anything has been said", () => {
    render(<VoiceTranscript state={{ status: "listening" }} turns={[]} active />);
    expect(screen.getByText(/just start talking/i)).toBeTruthy();
  });

  it("keeps the question on screen while the model is thinking", () => {
    render(
      <VoiceTranscript
        state={{ status: "thinking" }}
        turns={turns({ role: "user", text: "What does this repo build?" })}
        active
      />,
    );
    expect(screen.getByText("What does this repo build?")).toBeTruthy();
    expect(screen.getByText(/thinking/i)).toBeTruthy();
  });

  it("shows the answer under the question it answers", () => {
    render(
      <VoiceTranscript
        state={{ status: "speaking" }}
        turns={turns(
          { role: "user", text: "What does this repo build?" },
          { role: "assistant", text: "It builds a Rust daemon and several clients.", interim: true },
        )}
        active
      />,
    );
    expect(screen.getByText("What does this repo build?")).toBeTruthy();
    expect(screen.getByText("It builds a Rust daemon and several clients.")).toBeTruthy();
    // Not still claiming to be thinking once there are words to show.
    expect(screen.queryByText(/thinking…/i)).toBeNull();
  });

  it("shows only the newest turn, not the whole conversation", () => {
    // Everything older has already been handed to the chat log via `onTurn`;
    // repeating it here would render every turn twice.
    render(
      <VoiceTranscript
        state={{ status: "speaking" }}
        turns={turns(
          { role: "user", text: "first question" },
          { role: "assistant", text: "first answer" },
          { role: "user", text: "second question" },
          { role: "assistant", text: "second answer", interim: true },
        )}
        active
      />,
    );
    expect(screen.queryByText("first question")).toBeNull();
    expect(screen.queryByText("first answer")).toBeNull();
    expect(screen.getByText("second question")).toBeTruthy();
    expect(screen.getByText("second answer")).toBeTruthy();
  });

  it("reports an error even when the conversation has already stopped", () => {
    // The hook tears down on a failed start, so `active` is false at exactly
    // the moment the user most needs to be told why.
    render(
      <VoiceTranscript
        state={{ status: "error", message: "Microphone access was denied." }}
        turns={[]}
        active={false}
      />,
    );
    expect(screen.getByText("Microphone access was denied.")).toBeTruthy();
  });
});
