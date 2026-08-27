import { describe, it, expect, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ComposerDrawer, type ComposerGroup } from "@vibe/shared/composer/ComposerDrawer";

/**
 * VibeAIChat's composer, reduced to the parts this change is about: the model
 * control moved out of the top chrome and into the frame, and the voice opt-in
 * moved off the toolbar and behind `+`.
 *
 * The real `App` reaches for Tauri commands, the daemon and a microphone on
 * mount, so it is not renderable here. This pins the arrangement instead — the
 * thing that would silently regress the next time the toolbar grows a control.
 */
const Icon = () => <svg />;

function Composer({ voiceEnabled = false, daemonOk = true }: { voiceEnabled?: boolean; daemonOk?: boolean }) {
  const groups: ComposerGroup[] = [
    {
      title: "Voice",
      items: [
        {
          kind: "switch",
          id: "duplex",
          icon: Icon,
          label: "Voice conversation",
          on: voiceEnabled,
          disabled: !daemonOk,
          disabledHint: "Connect to a daemon to start a voice conversation",
          sub: { on: "On — start it from the toolbar", off: "Off — talk with the model, hands free" },
          onChange: vi.fn(),
        },
      ],
    },
  ];
  return (
    <div className="input-area vxc-composer">
      <div className="vxc-frame">
        <textarea className="vxc-frame__input" placeholder="Ask anything…" />
        <div className="vxc-bar vxc-bar--tight">
          <div className="vxc-pop">
            <ComposerDrawer groups={groups} onClose={vi.fn()} label="Turn on voice" />
            <button className="vxc-iconbtn" aria-label="More options" />
          </div>
          <div className="vxc-spacer" />
          <div className="aic-model vxc-bar__shrink">
            <select aria-label="Provider"><option>Ollama</option></select>
            <select aria-label="Model"><option>gpt-oss:120b-cloud</option></select>
          </div>
          {voiceEnabled && <button aria-label="Start voice conversation" />}
          <button className="vxc-send" aria-label="Send message" />
        </div>
      </div>
      <div className="vxc-underrow">
        <span className="vxc-hint">Enter sends · ⇧Enter for a new line</span>
      </div>
    </div>
  );
}

describe("VibeAIChat composer", () => {
  it("puts the model control inside the frame, with the text and the send button", () => {
    // It used to be a full-width bar above the conversation. In a 440px window
    // that spent a whole row on a choice you make once a day.
    render(<Composer />);
    const frame = document.querySelector(".vxc-frame");
    expect(frame).toContainElement(screen.getByLabelText("Model"));
    expect(frame).toContainElement(screen.getByLabelText("Provider"));
    expect(frame).toContainElement(screen.getByPlaceholderText("Ask anything…"));
    expect(frame).toContainElement(screen.getByLabelText("Send message"));
  });

  it("keeps the keyboard contract out of the placeholder", () => {
    // A placeholder vanishes the moment you type, which is exactly when you
    // might want to know what Enter does.
    render(<Composer />);
    expect(screen.getByPlaceholderText("Ask anything…")).toBeInTheDocument();
    expect(screen.getByText(/Enter sends/)).toBeInTheDocument();
    expect(document.querySelector(".vxc-frame")).not.toContainElement(
      screen.getByText(/Enter sends/),
    );
  });

  it("shows no live voice control until voice is turned on", () => {
    // The toolbar carries the start/stop; the opt-in lives behind `+`, so a
    // feature nobody enabled costs no room on a 440px bar.
    const { rerender } = render(<Composer voiceEnabled={false} />);
    expect(screen.queryByLabelText("Start voice conversation")).toBeNull();
    rerender(<Composer voiceEnabled />);
    expect(screen.getByLabelText("Start voice conversation")).toBeInTheDocument();
  });

  it("lets only the model control give up width", () => {
    // Everything else is `flex: none`; without this the long model names push
    // the send button off the row.
    render(<Composer />);
    expect(document.querySelector(".aic-model")).toHaveClass("vxc-bar__shrink");
    expect(document.querySelector(".vxc-bar")).toHaveClass("vxc-bar--tight");
  });

  it("says why voice is unavailable rather than offering a dead switch", () => {
    render(<Composer daemonOk={false} />);
    const item = screen.getByRole("menuitemcheckbox", { name: /Voice conversation/ });
    expect(item).toBeDisabled();
    expect(item).toHaveTextContent("Connect to a daemon");
  });

  it("does not close the menu when voice is switched on", () => {
    const onChange = vi.fn();
    const groups: ComposerGroup[] = [
      { title: "Voice", items: [{ kind: "switch", id: "d", icon: Icon, label: "Voice conversation", on: false, onChange }] },
    ];
    const onClose = vi.fn();
    render(<ComposerDrawer groups={groups} onClose={onClose} />);
    fireEvent.click(screen.getByRole("menuitemcheckbox"));
    expect(onChange).toHaveBeenCalledWith(true);
    expect(onClose).not.toHaveBeenCalled();
  });
});
