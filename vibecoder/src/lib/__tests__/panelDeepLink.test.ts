/**
 * Deep links into a panel's subtab.
 *
 * The event alone cannot carry these: panels are lazy, so the first link into
 * one fires while its `TabbedPanel` is still unmounted and there is nobody
 * listening. The parked request is what covers that gap, which makes "who is
 * allowed to claim it" the whole correctness question — a panel that claims a
 * request meant for another one navigates the user somewhere they did not ask
 * to go, and the panel they did ask for never opens.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { openPanelTab, takePendingTab } from "../panelDeepLink";

describe("panel deep links", () => {
  beforeEach(() => {
    // Drain anything a previous scenario parked.
    takePendingTab("ai-ml");
    takePendingTab("security");
  });

  it("dispatches the key it was given", () => {
    const seen: unknown[] = [];
    const handler = (e: Event) => seen.push((e as CustomEvent).detail);
    window.addEventListener("vibecoder:open-tab", handler);
    openPanelTab("ai-ml/skills");
    window.removeEventListener("vibecoder:open-tab", handler);
    expect(seen).toEqual(["ai-ml/skills"]);
  });

  it("parks a subtab request for its own panel and nobody else's", () => {
    openPanelTab("ai-ml/skills");
    expect(takePendingTab("security")).toBeNull();
    expect(takePendingTab("ai-ml")).toBe("ai-ml/skills");
  });

  it("hands the request over once — a later mount must not re-navigate", () => {
    openPanelTab("ai-ml/skills");
    expect(takePendingTab("ai-ml")).toBe("ai-ml/skills");
    expect(takePendingTab("ai-ml")).toBeNull();
  });

  it("parks nothing for a bare panel id, which App acts on and is always listening", () => {
    openPanelTab("chat");
    expect(takePendingTab("chat")).toBeNull();
  });

  it("does not let a stale request outlive the one that replaced it", () => {
    openPanelTab("ai-ml/skills");
    openPanelTab("security/scan");
    expect(takePendingTab("ai-ml")).toBeNull();
    expect(takePendingTab("security")).toBe("security/scan");
  });

  it("clears the parked request when a bare panel id follows it", () => {
    openPanelTab("ai-ml/skills");
    openPanelTab("chat");
    expect(takePendingTab("ai-ml")).toBeNull();
  });
});

describe("prefix matching", () => {
  it("does not treat a panel as a prefix of a longer panel name", () => {
    openPanelTab("ai-ml-extra/thing");
    expect(takePendingTab("ai-ml")).toBeNull();
    expect(takePendingTab("ai-ml-extra")).toBe("ai-ml-extra/thing");
    vi.restoreAllMocks();
  });
});
