/**
 * Unit test for the shared <ToggleSwitch> primitive — US-012 (A-4).
 *
 * Custom-styled toggles must expose `role="switch"` + `aria-checked` +
 * keyboard activation (Space/Enter), otherwise AT users see a generic
 * clickable div. This primitive exists so panels can't forget.
 */
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ToggleSwitch } from "../shared/ToggleSwitch";

describe("US-012 — ToggleSwitch primitive (A-4)", () => {
  it("renders with role=switch and aria-checked reflects state", () => {
    render(<ToggleSwitch checked={true} onChange={() => {}} label="Dark mode" />);
    const el = screen.getByRole("switch", { name: "Dark mode" });
    expect(el).toHaveAttribute("aria-checked", "true");
  });

  it("aria-checked=false when unchecked", () => {
    render(<ToggleSwitch checked={false} onChange={() => {}} label="Notifications" />);
    const el = screen.getByRole("switch", { name: "Notifications" });
    expect(el).toHaveAttribute("aria-checked", "false");
  });

  it("click invokes onChange with the toggled value", () => {
    const onChange = vi.fn();
    render(<ToggleSwitch checked={false} onChange={onChange} label="X" />);
    fireEvent.click(screen.getByRole("switch"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("Space activates the switch", () => {
    const onChange = vi.fn();
    render(<ToggleSwitch checked={true} onChange={onChange} label="X" />);
    const el = screen.getByRole("switch");
    fireEvent.keyDown(el, { key: " " });
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("Enter activates the switch", () => {
    const onChange = vi.fn();
    render(<ToggleSwitch checked={false} onChange={onChange} label="X" />);
    fireEvent.keyDown(screen.getByRole("switch"), { key: "Enter" });
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("is focusable via tabIndex=0", () => {
    render(<ToggleSwitch checked={false} onChange={() => {}} label="X" />);
    expect(screen.getByRole("switch")).toHaveAttribute("tabindex", "0");
  });

  it("when disabled, onChange is not called and aria-disabled is set", () => {
    const onChange = vi.fn();
    render(<ToggleSwitch checked={false} onChange={onChange} label="X" disabled />);
    const el = screen.getByRole("switch");
    expect(el).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(el);
    fireEvent.keyDown(el, { key: "Enter" });
    expect(onChange).not.toHaveBeenCalled();
  });
});
