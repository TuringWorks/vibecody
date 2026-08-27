/**
 * BDD + source scan: the chat composer's toolbar wraps rather than clips.
 *
 * `.chat-input-toolbar` shipped as `flex-wrap: nowrap` with `overflow: hidden`.
 * In a narrow sidebar that cut the row off at the card edge — and the thing on
 * the far right is the send button, so the panel's primary action was
 * invisible and unreachable, with the voice control's × the last thing still
 * showing. Wrapping is the only safe direction here: a second line is a
 * cosmetic cost, a missing send button is a dead panel.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn(async () => null);
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => mockInvoke(...(a as [])) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: () => Promise.resolve(() => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../../hooks/useToast", () => ({
  useToast: () => ({ toast: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), success: vi.fn() } }),
}));
vi.mock("../ContextPicker", () => ({ ContextPicker: () => <div /> }));
vi.mock("../../utils/FlowContext", () => ({ flowContext: { add: vi.fn() } }));
vi.mock("@vibe/shared/voice/useVoiceDuplex", () => ({
  useVoiceDuplex: () => ({
    state: { status: "idle" },
    active: false,
    supported: true,
    start: vi.fn(),
    stop: vi.fn(),
  }),
}));

import { AIChat } from "../AIChat";

// jsdom has no layout, so the panel's scroll-to-bottom effect throws on mount.
beforeEach(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

const CSS = readFileSync(resolve(__dirname, "..", "AIChat.css"), "utf8");
/** The declarations of one rule, comments stripped. */
function rule(selector: string): string {
  const body = CSS.replace(/\/\*[\s\S]*?\*\//g, "").split(new RegExp(`(?:^|\\})\\s*${selector.replace(".", "\\.")}\\s*\\{`))[1];
  expect(body, `no rule found for ${selector} — the scan is not reading the file`).toBeTruthy();
  return body.split("}")[0];
}

describe("chat composer toolbar", () => {
  it("wraps instead of clipping", () => {
    const toolbar = rule(".chat-input-toolbar");
    expect(toolbar).toMatch(/flex-wrap:\s*wrap/);
    expect(toolbar, "overflow:hidden here cuts the send button off the card").not.toMatch(
      /overflow:\s*hidden/
    );
  });

  it("responds to the card's own width, not the window's", () => {
    // The card lives in a resizable sidebar: a viewport media query would ask
    // the wrong element how much room there is.
    expect(rule(".chat-input-card")).toMatch(/container-type:\s*inline-size/);
    expect(CSS).toMatch(/@container[^{]*\{\s*\.mode-btn span \{ display: none/);
  });

  it("keeps the mode buttons named once their labels collapse to icons", async () => {
    const { getByLabelText } = render(
      <AIChat provider="ollama" messages={[]} onMessagesChange={vi.fn()} />
    );
    await waitFor(() => expect(getByLabelText("Balanced")).toBeInTheDocument());
    for (const name of ["Fast", "Balanced", "Thorough"]) {
      expect(getByLabelText(name)).toBeInTheDocument();
    }
  });

  it("keeps voice and send together so they wrap as one cluster", async () => {
    const { container, getByLabelText } = render(
      <AIChat provider="ollama" messages={[]} onMessagesChange={vi.fn()} />
    );
    await waitFor(() => expect(container.querySelector(".chat-toolbar-right")).toBeTruthy());
    const right = container.querySelector(".chat-toolbar-right")!;
    expect(right.contains(getByLabelText("Send message"))).toBe(true);
    expect(right.contains(getByLabelText("Start voice input"))).toBe(true);
  });
});
