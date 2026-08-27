/**
 * Consent to change a file is a click, not a word.
 *
 * The assistant speaks the question — the user may not be looking at the
 * window — but the answer has to be deliberate: "yes" is a word a microphone
 * can mishear, and the cost of getting it wrong is an overwritten file. So the
 * prompt renders, names the file, and offers both answers with refusal first.
 */
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { VoiceApproval } from "@vibe/shared/voice/VoiceApproval";

describe("voice approval", () => {
  it("shows nothing when nothing was asked", () => {
    const { container } = render(<VoiceApproval approval={null} onRespond={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("asks about the file by name", () => {
    render(
      <VoiceApproval
        approval={{ question: "May I write src/main.rs? It replaces the file with 12 lines." }}
        onRespond={vi.fn()}
      />
    );
    expect(screen.getByText(/src\/main\.rs/)).toBeInTheDocument();
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
  });

  it("sends yes only for the yes button", () => {
    const onRespond = vi.fn();
    render(<VoiceApproval approval={{ question: "May I write a.rs?" }} onRespond={onRespond} />);

    fireEvent.click(screen.getByRole("button", { name: "No" }));
    expect(onRespond).toHaveBeenCalledWith(false);

    fireEvent.click(screen.getByRole("button", { name: /Yes/ }));
    expect(onRespond).toHaveBeenLastCalledWith(true);
    expect(onRespond).toHaveBeenCalledTimes(2);
  });
});
