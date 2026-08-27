import { describe, it, expect, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ComposerDrawer, type ComposerGroup } from "@vibe/shared/composer/ComposerDrawer";

const Icon = () => <svg />;

const groups = (over: Partial<{ on: boolean; supported: boolean }> = {}): ComposerGroup[] => [
  {
    title: "Add to this message",
    items: [
      { id: "attach", icon: Icon, label: "Attach files", sub: "Send file contents", onSelect: vi.fn() },
    ],
  },
  {
    title: "Voice",
    items: [
      {
        kind: "switch",
        id: "duplex",
        icon: Icon,
        label: "Voice conversation",
        on: over.on ?? false,
        disabled: over.supported === false,
        disabledHint: "This webview cannot capture audio",
        sub: { on: "On — start it from the toolbar", off: "Off — talk hands free" },
        onChange: vi.fn(),
      },
    ],
  },
];

describe("ComposerDrawer", () => {
  it("closes before running an action, so the menu is gone when a dialog opens", async () => {
    // Order matters: `onSelect` opens a native file picker, and closing after
    // it leaves the menu floating over the dialog.
    const order: string[] = [];
    const g = groups();
    (g[0].items[0] as { onSelect: () => void }).onSelect = () => order.push("select");
    render(<ComposerDrawer groups={g} onClose={() => order.push("close")} />);
    fireEvent.click(screen.getByRole("menuitem", { name: /Attach files/ }));
    expect(order).toEqual(["close", "select"]);
  });

  it("leaves the menu open when a switch is flipped", async () => {
    const onClose = vi.fn();
    const onChange = vi.fn();
    const g = groups({ on: false });
    (g[1].items[0] as { onChange: (on: boolean) => void }).onChange = onChange;
    render(<ComposerDrawer groups={g} onClose={onClose} />);
    fireEvent.click(screen.getByRole("menuitemcheckbox", { name: /Voice conversation/ }));
    // You flip it to see what it says next; closing the menu hides the answer.
    expect(onChange).toHaveBeenCalledWith(true);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("reports switch state to assistive tech", () => {
    render(<ComposerDrawer groups={groups({ on: true })} onClose={vi.fn()} />);
    expect(screen.getByRole("menuitemcheckbox")).toHaveAttribute("aria-checked", "true");
  });

  it("says why a disabled item is disabled instead of showing a dead row", () => {
    // A greyed-out control with no reason reads as a bug in the app.
    render(<ComposerDrawer groups={groups({ supported: false })} onClose={vi.fn()} />);
    const item = screen.getByRole("menuitemcheckbox");
    expect(item).toBeDisabled();
    expect(item).toHaveTextContent("This webview cannot capture audio");
    // No switch graphic on a control that cannot be switched.
    expect(item.querySelector(".vxc-drawer__switch")).toBeNull();
  });

  it("omits a group heading with nothing under it", () => {
    // Hosts build these arrays conditionally — VibeAIChat has no files to
    // attach — and an empty "Add to this message" heading is worse than none.
    render(
      <ComposerDrawer
        groups={[{ title: "Empty", items: [] }, ...groups()]}
        onClose={vi.fn()}
      />,
    );
    expect(screen.queryByText("Empty")).toBeNull();
    expect(screen.getByText("Voice")).toBeInTheDocument();
  });
});
