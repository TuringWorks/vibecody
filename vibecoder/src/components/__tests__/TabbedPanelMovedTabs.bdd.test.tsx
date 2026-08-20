/**
 * BDD: a tab moved into a panel is actually rendered by that panel.
 *
 * The destination panel has never imported the tab it is now hosting, so it
 * loads it by key from the generated `tabRegistry`. That indirection is the
 * whole feature, and it is the part a preference test cannot reach: storing
 * `moves.tabs` proves nothing if the host never looks it up.
 *
 * The registry is mocked rather than real so the scenario exercises the
 * hosting path without pulling a genuine panel — and its heavy dependencies —
 * into jsdom. The real registry's *shape* is guarded by the catalog contract
 * test, which reads it back out of the composite sources.
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("../../constants/tabRegistry", () => ({
  TAB_REGISTRY: {
    "design/sketch": {
      panelId: "design",
      tabId: "sketch",
      label: "Sketch",
      load: async () => ({ default: () => <div>sketch body</div> }),
    },
    "design/named": {
      panelId: "design",
      tabId: "named",
      label: "Named",
      exportName: "NamedPanel",
      load: async () => ({ NamedPanel: () => <div>named body</div> }),
    },
  },
  isMovableTab: (key: string) => key.startsWith("design/"),
}));

import { TabbedPanel } from "../TabbedPanel";
import { saveLayoutPrefs, EMPTY_PREFS } from "../../lib/layoutPrefs";

const ownTabs = [
  { id: "scan", label: "Scan", content: <div>scan body</div> },
  { id: "redteam", label: "Red Team", content: <div>redteam body</div> },
];

const withMove = (tabs: Record<string, string>) =>
  saveLayoutPrefs({ ...EMPTY_PREFS, moves: { panels: {}, tabs } });

beforeEach(() => localStorage.clear());

describe("Given a tab moved into this panel", () => {
  it("When the panel renders, Then the moved tab appears alongside its own", async () => {
    withMove({ "design/sketch": "security" });
    render(<TabbedPanel panelId="security" tabs={ownTabs} />);

    await waitFor(() => expect(screen.getByRole("button", { name: "Sketch" })).toBeTruthy());
    expect(screen.getByRole("button", { name: "Scan" })).toBeTruthy();
  });

  it("When the moved tab is opened, Then its content is what renders", async () => {
    withMove({ "design/sketch": "security" });
    render(<TabbedPanel panelId="security" tabs={ownTabs} />);

    fireEvent.click(await screen.findByRole("button", { name: "Sketch" }));
    expect(await screen.findByText("sketch body")).toBeTruthy();
  });

  it("When the tab's module has no default export, Then the named one is used", async () => {
    // Most panels in this codebase are named exports; a host that only handled
    // `default` would render a blank tab for the majority of them.
    withMove({ "design/named": "security" });
    render(<TabbedPanel panelId="security" tabs={ownTabs} />);

    fireEvent.click(await screen.findByRole("button", { name: "Named" }));
    expect(await screen.findByText("named body")).toBeTruthy();
  });

  it("When it is hidden, Then it stays hidden under its original key", async () => {
    // Hiding is addressed by where a tab ships, not where it landed — that is
    // what lets it be hidden, moved, and un-hidden without losing track.
    saveLayoutPrefs({
      ...EMPTY_PREFS,
      hidden: { groups: [], panels: [], tabs: ["design/sketch"] },
      moves: { panels: {}, tabs: { "design/sketch": "security" } },
    });
    render(<TabbedPanel panelId="security" tabs={ownTabs} />);

    await waitFor(() => expect(screen.getByRole("button", { name: "Scan" })).toBeTruthy());
    expect(screen.queryByRole("button", { name: "Sketch" })).toBeNull();
  });

  it("When a moved tab shares an id with one already here, Then both are shown", async () => {
    // Tab ids repeat across panels. Keying the strip by id alone would drop one
    // of them, or worse, show one tab's label over the other's content.
    withMove({ "design/sketch": "security" });
    render(
      <TabbedPanel
        panelId="security"
        tabs={[{ id: "sketch", label: "Own Sketch", content: <div>own sketch body</div> }]}
      />,
    );

    await waitFor(() => expect(screen.getByRole("button", { name: "Sketch" })).toBeTruthy());
    expect(screen.getByRole("button", { name: "Own Sketch" })).toBeTruthy();
  });

  it("When two tabs share an id, Then clicking one opens that one", async () => {
    // Resolved by key rather than id: with ids alone the second tab's click
    // would land on the first, and the panel would look broken in a way that
    // only shows up for the handful of ids that repeat.
    withMove({ "design/sketch": "security" });
    render(
      <TabbedPanel
        panelId="security"
        tabs={[{ id: "sketch", label: "Own Sketch", content: <div>own sketch body</div> }]}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Sketch" }));
    expect(await screen.findByText("sketch body")).toBeTruthy();
    expect(screen.queryByText("own sketch body")?.parentElement?.style.display).toBe("none");
  });

  it("When nothing was moved here, Then only this panel's own tabs render", () => {
    // The lookup is skipped entirely in that case — the effect returns before
    // importing the registry — so a panel hosting nothing foreign does not pay
    // for the map of every movable tab in the app. That guard lives in
    // TabbedPanel; what is observable from here is its result.
    render(<TabbedPanel panelId="security" tabs={ownTabs} />);

    expect(screen.getByRole("button", { name: "Scan" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Red Team" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Sketch" })).toBeNull();
  });
});
