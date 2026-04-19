/**
 * Unit test for the shared <PanelError> primitive — US-015 (A-2).
 *
 * Error containers must carry `role="alert"` + `aria-live="assertive"`
 * so that screen readers announce the failure immediately. Panels that
 * render `<div className="panel-error">{err}</div>` inline do NOT get
 * that announcement — the text just appears silently. This primitive
 * enforces the contract at the usage site.
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PanelError } from "../shared/PanelError";

describe("US-015 — PanelError primitive (A-2)", () => {
  it("renders role=alert + aria-live=assertive", () => {
    render(<PanelError>Connection failed</PanelError>);
    const el = screen.getByRole("alert");
    expect(el).toHaveAttribute("aria-live", "assertive");
    expect(el).toHaveTextContent("Connection failed");
  });

  it("keeps the panel-error class so existing styling applies", () => {
    render(<PanelError>x</PanelError>);
    expect(screen.getByRole("alert")).toHaveClass("panel-error");
  });

  it("renders nothing when children are empty/null/false", () => {
    const { container, rerender } = render(<PanelError>{null}</PanelError>);
    expect(container.firstChild).toBeNull();
    rerender(<PanelError>{false}</PanelError>);
    expect(container.firstChild).toBeNull();
    rerender(<PanelError>{""}</PanelError>);
    expect(container.firstChild).toBeNull();
  });

  it("invokes onDismiss when the dismiss button is clicked", async () => {
    let dismissed = false;
    render(<PanelError onDismiss={() => (dismissed = true)}>x</PanelError>);
    const btn = screen.getByRole("button", { name: /dismiss/i });
    btn.click();
    expect(dismissed).toBe(true);
  });

  it("has no dismiss button when onDismiss is not provided", () => {
    render(<PanelError>x</PanelError>);
    expect(screen.queryByRole("button")).toBeNull();
  });
});
