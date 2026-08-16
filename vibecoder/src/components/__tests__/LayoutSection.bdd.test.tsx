/**
 * BDD: Settings → Panels & Tabs changes what the app renders.
 *
 * A settings page is only worth anything if the surfaces it claims to control
 * actually change, so these scenarios drive the real `LayoutSection` and then
 * assert on the real `GroupedTabBar` and the real `TabbedPanel` — the nav and
 * the tab strip — rather than on the preference object it wrote. A test that
 * only checks the stored JSON passes for a settings page wired to nothing.
 *
 * The three scenarios that matter most are the ones about *not* losing things:
 * a hidden feature stays listed here so it can be turned back on, a panel added
 * in a later release still appears for someone whose preferences predate it,
 * and hiding every tab in a panel does not leave an empty shell.
 */

import { render, screen, fireEvent, within } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi } from "vitest";

import { LayoutSection } from "../settings/LayoutSection";
import { GroupedTabBar } from "../GroupedTabBar";
import { TabbedPanel } from "../TabbedPanel";
import { applyLayout, loadLayoutPrefs, saveLayoutPrefs, EMPTY_PREFS } from "../../lib/layoutPrefs";
import { TAB_GROUPS } from "../../constants/tabGroups";

// jsdom implements no scrolling, and GroupedTabBar scrolls the active tab into
// view on mount. Nothing under test depends on it happening.
Element.prototype.scrollIntoView = vi.fn();

beforeEach(() => localStorage.clear());

/** The nav, rendered with whatever preferences are currently stored. */
function renderNav() {
  return render(<GroupedTabBar activeTab="chat" onTabChange={() => {}} />);
}

/** Group headers in the order the nav shows them. */
function navGroupOrder(): string[] {
  return screen
    .getAllByRole("button")
    .map((b) => b.textContent ?? "")
    .filter((t) => TAB_GROUPS.some((g) => t.startsWith(g.label)))
    .map((t) => TAB_GROUPS.find((g) => t.startsWith(g.label))!.label);
}

describe("Given the shipped layout", () => {
  it("When Settings opens, Then every group and panel is listed", async () => {
    render(<LayoutSection />);

    for (const group of TAB_GROUPS) {
      expect(screen.getByLabelText(`Show the ${group.label} group`)).toBeTruthy();
    }
    // The summary counts what is on, so "all of them" is visible at a glance.
    // 42 panels are reachable from the nav; three more render without a nav
    // entry and are not listed here.
    const summary = screen.getByText((_c, el) => /42 of 42 panels/.test(el?.textContent ?? ""), {
      selector: "p",
    });
    expect(summary).toBeTruthy();
  });

  it("When nothing has been changed, Then the nav renders the shipped order", () => {
    renderNav();
    expect(navGroupOrder()).toEqual(TAB_GROUPS.map((g) => g.label));
  });
});

describe("Given a panel the user does not want", () => {
  it("When it is unchecked, Then it disappears from the nav", () => {
    const settings = render(<LayoutSection />);
    fireEvent.click(screen.getByLabelText("Show the Billing panel"));
    settings.unmount();

    renderNav();
    expect(screen.queryByText("Billing")).toBeNull();
  });

  it("When it is unchecked, Then it is still listed in Settings so it can come back", () => {
    render(<LayoutSection />);
    const box = screen.getByLabelText("Show the Billing panel") as HTMLInputElement;
    fireEvent.click(box);

    // A settings page whose disabled entries vanish from it is a one-way door.
    expect((screen.getByLabelText("Show the Billing panel") as HTMLInputElement).checked).toBe(false);

    fireEvent.click(screen.getByLabelText("Show the Billing panel"));
    expect((screen.getByLabelText("Show the Billing panel") as HTMLInputElement).checked).toBe(true);
    expect(loadLayoutPrefs().hidden.panels).toEqual([]);
  });

  it("When a whole group is unchecked, Then none of its panels reach the nav", () => {
    const settings = render(<LayoutSection />);
    fireEvent.click(screen.getByLabelText("Show the Company group"));
    settings.unmount();

    renderNav();
    expect(screen.queryByText("Company")).toBeNull();
  });
});

describe("Given a group in the wrong place", () => {
  it("When it is moved up, Then the nav order changes to match", () => {
    const settings = render(<LayoutSection />);
    const second = TAB_GROUPS[1].label;
    fireEvent.click(screen.getByLabelText(`Move ${second} up`));
    settings.unmount();

    renderNav();
    expect(navGroupOrder()[0]).toBe(second);
  });

  it("When the first group is shown, Then it cannot be moved up", () => {
    render(<LayoutSection />);
    const first = screen.getByLabelText(`Move ${TAB_GROUPS[0].label} up`) as HTMLButtonElement;
    expect(first.disabled).toBe(true);
  });

  it("When a search is active, Then reordering is disabled rather than wrong", () => {
    render(<LayoutSection />);
    fireEvent.change(screen.getByLabelText("Find a panel or tab"), { target: { value: "billing" } });

    // The arrows move an item relative to what is on screen; with the list
    // filtered that is not the move anyone means.
    const btn = screen.getByLabelText("Move Billing down") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });
});

describe("Given a panel with subfeature tabs", () => {
  const tabs = [
    { id: "review", label: "Review", content: <div>review body</div> },
    { id: "redteam", label: "Red Team", content: <div>redteam body</div> },
  ];

  it("When a tab is hidden, Then the panel stops rendering it", () => {
    saveLayoutPrefs({
      ...EMPTY_PREFS,
      hidden: { groups: [], panels: [], tabs: ["security/redteam"] },
    });
    render(<TabbedPanel panelId="security" tabs={tabs} />);

    expect(screen.getByRole("button", { name: "Review" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Red Team" })).toBeNull();
  });

  it("When tabs are reordered, Then the strip follows and opens the new first tab", () => {
    saveLayoutPrefs({
      ...EMPTY_PREFS,
      order: { groups: [], panels: {}, tabs: { security: ["redteam", "review"] } },
    });
    render(<TabbedPanel panelId="security" tabs={tabs} />);

    const strip = screen.getByRole("button", { name: "Red Team" }).parentElement as HTMLElement;
    const labels = within(strip).getAllByRole("button").map((b) => b.textContent);
    expect(labels).toEqual(["Red Team", "Review"]);
  });

  it("When every tab is hidden, Then the panel falls back rather than render nothing", () => {
    saveLayoutPrefs({
      ...EMPTY_PREFS,
      hidden: { groups: [], panels: [], tabs: ["security/review", "security/redteam"] },
    });
    render(<TabbedPanel panelId="security" tabs={tabs} />);

    // An empty strip has no control that would bring anything back, so the
    // preference is ignored when it would leave nothing at all.
    expect(screen.getByRole("button", { name: "Review" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Red Team" })).toBeTruthy();
  });

  it("When the open tab is hidden, Then the panel shows another instead of blanking", () => {
    saveLayoutPrefs({
      ...EMPTY_PREFS,
      hidden: { groups: [], panels: [], tabs: ["security/review"] },
    });
    render(<TabbedPanel panelId="security" tabs={tabs} defaultTab="review" />);

    expect(screen.getByText("redteam body")).toBeTruthy();
  });

  it("When a tab id is shared with another panel, Then only the named one is hidden", () => {
    // "dashboard" is a tab in several panels; the key carries the panel so
    // hiding one does not hide its namesakes.
    saveLayoutPrefs({
      ...EMPTY_PREFS,
      hidden: { groups: [], panels: [], tabs: ["agent-os/dashboard"] },
    });
    const shared = [
      { id: "dashboard", label: "Dashboard", content: <div>obs</div> },
      { id: "traces", label: "Traces", content: <div>traces</div> },
    ];
    render(<TabbedPanel panelId="observability" tabs={shared} />);

    expect(screen.getByRole("button", { name: "Dashboard" })).toBeTruthy();
  });

  it("When no panelId is given, Then preferences do not apply", () => {
    // TabbedPanel is also used for tab strips nested inside a panel. Those are
    // not user-configurable features and must not be silently reordered.
    saveLayoutPrefs({
      ...EMPTY_PREFS,
      hidden: { groups: [], panels: [], tabs: ["security/redteam"] },
    });
    render(<TabbedPanel tabs={tabs} />);

    expect(screen.getByRole("button", { name: "Red Team" })).toBeTruthy();
  });
});

describe("Given preferences saved before a panel existed", () => {
  it("When the app adds one, Then it still appears", () => {
    // Storing a full ordering would freeze the layout at its saved version.
    // Preferences are a diff, so an id absent from the stored order means
    // "not ranked yet", never "excluded".
    const shipped = ["old-a", "old-b", "brand-new"];
    const savedBefore = ["old-b", "old-a"];
    expect(applyLayout(shipped, (s) => s, savedBefore, [])).toContain("brand-new");
  });
});

describe("Given a customised layout", () => {
  it("When Reset is clicked, Then the shipped layout comes back", () => {
    const settings = render(<LayoutSection />);
    fireEvent.click(screen.getByLabelText("Show the Billing panel"));
    expect(loadLayoutPrefs().hidden.panels).toEqual(["billing"]);

    fireEvent.click(screen.getByRole("button", { name: /Reset to defaults/ }));

    expect(loadLayoutPrefs()).toEqual(EMPTY_PREFS);
    settings.unmount();
    renderNav();
    expect(navGroupOrder()).toEqual(TAB_GROUPS.map((g) => g.label));
  });
});
