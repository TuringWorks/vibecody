/**
 * BDD: Connectors is a menu entry, like Plugins.
 *
 * The nav rail is the app's only durable way in. Connectors previously had no
 * entry at all — you reached them by opening Plugins and scrolling a mixed
 * marketplace, which is why "does my GitHub connector work" had no answer you
 * could navigate to.
 *
 * These drive the rail itself rather than the shell, so they stay honest about
 * what they cover: the button exists, is labelled, and calls its handler. The
 * wiring from that handler to the overlay is asserted separately by
 * `ShellLayout` rendering `ConnectorsView` — see ConnectorsView.bdd.test.tsx
 * for the view's own behaviour.
 */

import type React from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => null) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => null) }));

import { ProjectNavRail } from "../ProjectNavRail";

type RailProps = React.ComponentProps<typeof ProjectNavRail>;

function railProps(overrides: Partial<RailProps> = {}): RailProps {
  return {
    tasks: [],
    tasksError: null,
    projectPaths: [],
    activeChatId: null,
    activeProject: null,
    collapsed: false,
    onNewChat: () => {},
    onNewProject: () => {},
    onSelectProject: () => {},
    onDeleteProject: () => {},
    onSelectChat: () => {},
    onRenameChat: () => {},
    onDeleteChat: () => {},
    onArchiveChat: () => {},
    onOpenSearch: () => {},
    onOpenSkills: () => {},
    onOpenPlugins: () => {},
    onOpenConnectors: () => {},
    onOpenAutomations: () => {},
    onOpenTrash: () => {},
    onOpenSettings: () => {},
    onToggle: () => {},
    ...overrides,
  } as RailProps;
}

describe("Given the navigation rail", () => {
  it("When it renders, Then Connectors sits alongside Plugins", () => {
    render(<ProjectNavRail {...railProps()} />);

    expect(screen.getByRole("button", { name: /^connectors$/i })).toBeInTheDocument();
    // Parity with Plugins is the requirement, so assert both rather than
    // asserting Connectors alone and leaving the comparison implicit.
    expect(screen.getByRole("button", { name: /^plugins$/i })).toBeInTheDocument();
  });

  it("When Connectors is clicked, Then it opens Connectors and not Plugins", () => {
    const onOpenConnectors = vi.fn();
    const onOpenPlugins = vi.fn();
    render(<ProjectNavRail {...railProps({ onOpenConnectors, onOpenPlugins })} />);

    fireEvent.click(screen.getByRole("button", { name: /^connectors$/i }));

    expect(onOpenConnectors).toHaveBeenCalledTimes(1);
    // The two entries are adjacent and share styling; wiring one to the other's
    // handler would look right on screen and open the wrong panel.
    expect(onOpenPlugins).not.toHaveBeenCalled();
  });
});
