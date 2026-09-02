/**
 * BDD: picking a skill in VibeCoder reaches the composer.
 *
 * The panel does not render the chat, so the only evidence it works is the
 * pair of events it emits — and their order is load-bearing. `inject-context`
 * has to go first: the chat tab's listener is the one that catches it, and a
 * switch-then-inject order would fire into whatever React had rendered by
 * then. Asserting on the events rather than on the button is also what keeps
 * this honest about the seam it actually owns.
 */

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { SkillsPanel } from "../SkillsPanel";

const catalogue = {
  skills: [
    { name: "test-first", category: "engineering", summary: "Pin behaviour.", source: "builtin" },
  ],
};

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockImplementation((cmd: string) =>
    cmd === "skilllens_list_skills"
      ? Promise.resolve(catalogue)
      : Promise.resolve({ name: "test-first", body: "# test-first" }),
  );
});

describe("VibeCoder skill catalogue", () => {
  it("reads the catalogue through the commands this shell registers", async () => {
    render(<SkillsPanel />);
    await screen.findByText("test-first");
    expect(mockInvoke).toHaveBeenCalledWith("skilllens_list_skills");
  });

  it("injects the pick before switching to chat", async () => {
    const events: Array<{ type: string; detail: unknown }> = [];
    const record = (e: Event) =>
      events.push({ type: e.type, detail: (e as CustomEvent).detail });
    window.addEventListener("vibecoder:inject-context", record);
    window.addEventListener("vibecoder:open-tab", record);

    render(<SkillsPanel />);
    fireEvent.click(await screen.findByLabelText("Select test-first"));
    fireEvent.click(screen.getByText("Use skill"));

    window.removeEventListener("vibecoder:inject-context", record);
    window.removeEventListener("vibecoder:open-tab", record);

    expect(events.map((e) => e.type)).toEqual([
      "vibecoder:inject-context",
      "vibecoder:open-tab",
    ]);
    expect(String(events[0].detail)).toContain("test-first");
    expect(events[1].detail).toBe("chat");
  });
});
