import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => (cmd === "daemon_port" ? 7878 : "tok")),
}));

import { VoiceSection } from "@vibe/shared/settings/VoiceSection";

const SETTINGS = {
  engine: "system",
  engines: [
    { id: "system", label: "System", available: true, detail: "The platform voice." },
    { id: "kokoro", label: "Neural (Kokoro)", available: true, detail: "Natural voices." },
  ],
  voice: "af_heart",
  language: "auto",
  voices: [{ id: "af_heart", name: "Heart", lang: "en", quality: "neural" }],
  languages: ["en", "hi"],
};

let fetchMock: ReturnType<typeof vi.fn>;
beforeEach(() => {
  fetchMock = vi.fn(async () => new Response(JSON.stringify(SETTINGS), { status: 200 }));
  vi.stubGlobal("fetch", fetchMock);
});
afterEach(() => {
  vi.unstubAllGlobals();
});

describe("VoiceSection", () => {
  it("renders the engines the daemon reports", async () => {
    render(<VoiceSection />);
    await waitFor(() => expect(screen.getByText("Neural (Kokoro)")).toBeTruthy());
    expect(screen.getByText("Speech engine")).toBeTruthy();
  });

  it("keeps the controls on screen when a save fails", async () => {
    // A failed *save* used to take the whole pane with it: the section returned
    // early on `error`, so one blocked PUT replaced every control with a line
    // of text and a save problem read as a load problem.
    render(<VoiceSection />);
    await waitFor(() => expect(screen.getByText("Neural (Kokoro)")).toBeTruthy());

    fetchMock.mockRejectedValueOnce(new TypeError("Load failed"));
    await act(async () => {
      screen.getByText("Neural (Kokoro)").closest("button")!.click();
    });

    expect(screen.getByRole("status").textContent).toContain("Load failed");
    expect(screen.getByText("Speech engine")).toBeTruthy();
    expect(screen.getByText("Neural (Kokoro)")).toBeTruthy();
  });

  it("shows only the message when there is nothing to show", async () => {
    fetchMock.mockRejectedValue(new TypeError("Load failed"));
    render(<VoiceSection />);
    await waitFor(() => expect(screen.queryByText(/daemon/i)).toBeTruthy());
    expect(screen.queryByText("Speech engine")).toBeNull();
  });
});
