import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ConfirmationDialog } from "../ConfirmationDialog";

describe("ConfirmationDialog", () => {
  it("renders an accessible destructive confirmation and invokes the selected action", () => {
    const onConfirm = vi.fn();
    render(
      <ConfirmationDialog
        open
        title="Delete key?"
        message="This cannot be undone."
        confirmLabel="Delete key"
        danger
        onConfirm={onConfirm}
        onCancel={vi.fn()}
      />,
    );
    expect(screen.getByRole("dialog")).toHaveAttribute("aria-modal", "true");
    fireEvent.click(screen.getByRole("button", { name: "Delete key" }));
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it("cancels with Escape unless an operation is busy", () => {
    const onCancel = vi.fn();
    const { rerender } = render(
      <ConfirmationDialog open title="Terminate?" message="Confirm" onConfirm={vi.fn()} onCancel={onCancel} />,
    );
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onCancel).toHaveBeenCalledOnce();
    rerender(
      <ConfirmationDialog open busy title="Terminate?" message="Confirm" onConfirm={vi.fn()} onCancel={onCancel} />,
    );
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onCancel).toHaveBeenCalledOnce();
  });
});
