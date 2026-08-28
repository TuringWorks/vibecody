/**
 * BDD: the composer groups its controls instead of listing them.
 *
 * The toolbar grew to eight equal-weight pills in one row — quick actions,
 * attach, mic, duplex voice, approval tier, branch, model, mode, effort — and
 * wrapped onto two lines at normal window widths, with nothing to say which of
 * them described the *message* and which described the *run*. These pin the
 * grouping that replaced it, because "which row is this control on" is exactly
 * the kind of thing a refactor silently undoes:
 *
 *   in the box   — what you are sending, and who answers it
 *   context row  — the run's standing conditions (approval, branch, sandbox)
 *   "+" menu     — everything summoned on demand, incl. the voice opt-in
 */

import { render, screen, fireEvent, within } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => null) }));
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


import { TaskPrompt, type ComposerSubmit } from "../TaskPrompt";
import { LOCKED_SANDBOX } from "../../lib/sandbox";
import { voiceSessionStub } from "./voiceSessionStub";

type Props = React.ComponentProps<typeof TaskPrompt>;

function props(overrides: Partial<Props> = {}): Props {
  return {
    daemonUrl: "http://127.0.0.1:7878",
    daemonOnline: true,
    busy: false,
    prefs: {
      provider: "anthropic",
      model: "claude-opus-5",
      approval: "default",
      reasoning: "medium",
      mode: "agent",
      sandbox: LOCKED_SANDBOX,
      isolate: false,
    },
    projectFiles: [],
    voice: voiceSessionStub(),
    onPref: vi.fn(),
    onProviderModel: vi.fn(),
    draft: "",
    onDraft: vi.fn(),
    attachments: [],
    onAttachments: vi.fn(),
    onSubmit: vi.fn<(p: ComposerSubmit) => void>(),
    onStop: vi.fn(),
    onQuickAction: vi.fn(),
    onSlash: vi.fn(),
    ...overrides,
  };
}

const box = () => document.querySelector(".vx-composer__box") as HTMLElement;
const contextRow = () => document.querySelector(".vx-composer__context") as HTMLElement;

describe("composer layout", () => {
  it("keeps the message and the model that answers it inside one frame", () => {
    render(<TaskPrompt {...props()} />);
    const frame = box();
    expect(frame).toBeInTheDocument();
    expect(within(frame).getByRole("textbox")).toBeInTheDocument();
    expect(within(frame).getByLabelText("Provider and model")).toBeInTheDocument();
    expect(within(frame).getByLabelText("Reasoning effort")).toBeInTheDocument();
    expect(within(frame).getByLabelText("Submit task")).toBeInTheDocument();
  });

  it("shows the run mode as a segmented switch, not a menu to open", () => {
    render(<TaskPrompt {...props()} />);
    const group = screen.getByRole("radiogroup", { name: "Run mode" });
    expect(within(group).getAllByRole("radio").map((b) => b.textContent)).toEqual([
      "Agent",
      "Chat",
      "Sandbox",
    ]);
    expect(within(group).getByRole("radio", { name: "Agent" })).toBeChecked();
  });

  it("puts the run's standing conditions on the context row, not the toolbar", () => {
    render(<TaskPrompt {...props()} />);
    const row = contextRow();
    expect(within(row).getByLabelText("Approval tier")).toBeInTheDocument();
    expect(within(row).getByText("In place")).toBeInTheDocument();
    // …and not in the frame with the send button.
    expect(within(box()).queryByLabelText("Approval tier")).toBeNull();
  });

  it("colours only the approval tier that gave something away", () => {
    const { rerender } = render(<TaskPrompt {...props()} />);
    expect(screen.getByLabelText("Approval tier").className).not.toContain("vx-chip--warn");
    rerender(<TaskPrompt {...props({ prefs: { ...props().prefs, approval: "full-access" } })} />);
    expect(screen.getByLabelText("Approval tier").className).toContain("vx-chip--warn");
  });

  it("offers the sandbox grants only in sandbox mode", () => {
    const { rerender } = render(<TaskPrompt {...props()} />);
    expect(screen.queryByText(/^Access:/)).toBeNull();
    rerender(<TaskPrompt {...props({ prefs: { ...props().prefs, mode: "sandbox" } })} />);
    expect(screen.getByText(/^Access:/)).toBeInTheDocument();
  });

  it("folds attach, panels and the voice opt-in into the one + menu", () => {
    render(<TaskPrompt {...props()} />);
    // Nothing but "+" for these until it is opened.
    expect(screen.queryByRole("menu", { name: "Quick actions" })).toBeNull();
    fireEvent.click(screen.getByLabelText("Attach files, open a panel, or turn on voice"));
    const menu = screen.getByRole("menu", { name: "Quick actions" });
    expect(within(menu).getByText("Attach files")).toBeInTheDocument();
    expect(within(menu).getByText("Files")).toBeInTheDocument();
    expect(within(menu).getByText("Terminal")).toBeInTheDocument();
    expect(within(menu).getByRole("menuitemcheckbox", { name: /Voice conversation/ })).toHaveAttribute(
      "aria-checked",
      "false"
    );
  });

  it("closes the + menu on a click outside it", () => {
    render(<TaskPrompt {...props()} />);
    fireEvent.click(screen.getByLabelText("Attach files, open a panel, or turn on voice"));
    expect(screen.getByRole("menu", { name: "Quick actions" })).toBeInTheDocument();
    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole("menu", { name: "Quick actions" })).toBeNull();
  });

  it("hides the duplex start button until voice is switched on", () => {
    render(<TaskPrompt {...props()} />);
    expect(screen.queryByLabelText("Start voice conversation")).toBeNull();
    // Dictation is always there — it is a different control with a different job.
    expect(screen.getByLabelText("Dictate (voice input)")).toBeInTheDocument();
  });
});
