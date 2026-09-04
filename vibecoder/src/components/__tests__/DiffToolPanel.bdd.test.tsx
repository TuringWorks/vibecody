import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DiffToolPanel } from "../DiffToolPanel";

function editors() {
  return {
    original: screen.getByRole("textbox", { name: "Original (A)" }) as HTMLTextAreaElement,
    modified: screen.getByRole("textbox", { name: "Modified (B)" }) as HTMLTextAreaElement,
  };
}

describe("DiffToolPanel — line diff semantics", () => {
  it("Given one changed line, then it reports one addition and one removal", () => {
    render(<DiffToolPanel />);
    const { original, modified } = editors();
    fireEvent.change(original, { target: { value: "same\nold" } });
    fireEvent.change(modified, { target: { value: "same\nnew" } });

    expect(screen.getByText("+1")).toBeInTheDocument();
    expect(screen.getByText("−1")).toBeInTheDocument();
    expect(screen.getByText("old")).toBeInTheDocument();
    expect(screen.getByText("new")).toBeInTheDocument();
  });

  it("Given identical text, then it reports the identical state", () => {
    render(<DiffToolPanel />);
    const { original, modified } = editors();
    fireEvent.change(original, { target: { value: "same" } });
    fireEvent.change(modified, { target: { value: "same" } });

    expect(screen.getByText("Identical")).toBeInTheDocument();
    expect(screen.queryByText(/unchanged/)).not.toBeInTheDocument();
  });

  it("Given multiline input, then line numbers track each side independently", () => {
    render(<DiffToolPanel />);
    const { original, modified } = editors();
    fireEvent.change(original, { target: { value: "a\nb\nc" } });
    fireEvent.change(modified, { target: { value: "a\nc\nd" } });
    fireEvent.click(screen.getByRole("button", { name: "inline" }));

    expect(screen.getByText("b")).toBeInTheDocument();
    expect(screen.getByText("d")).toBeInTheDocument();
    expect(screen.getAllByText("1").length).toBeGreaterThanOrEqual(2);
  });
});

describe("DiffToolPanel — views and actions", () => {
  it("Given differences, when unified view is selected, then it renders a patch hunk", () => {
    render(<DiffToolPanel />);
    const { original, modified } = editors();
    fireEvent.change(original, { target: { value: "old" } });
    fireEvent.change(modified, { target: { value: "new" } });
    fireEvent.click(screen.getByRole("button", { name: "unified" }));

    expect(screen.getByText(/@@ -1,1 \+1,1 @@/)).toBeInTheDocument();
    expect(screen.getByText(/-old/)).toBeInTheDocument();
    expect(screen.getByText(/\+new/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copy patch" })).toBeInTheDocument();
  });

  it("Given a patch, when copied, then the complete unified text reaches the clipboard", () => {
    const writeText = vi.fn();
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    render(<DiffToolPanel />);
    fireEvent.click(screen.getByRole("button", { name: "unified" }));
    fireEvent.click(screen.getByRole("button", { name: "Copy patch" }));

    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("@@"));
    expect(screen.getByRole("button", { name: /Copied/ })).toBeInTheDocument();
  });

  it("Given two inputs, when swapped, then each editor receives the other side", () => {
    render(<DiffToolPanel />);
    const before = editors();
    const left = before.original.value;
    const right = before.modified.value;
    fireEvent.click(screen.getByRole("button", { name: /Swap/ }));

    expect(editors().original).toHaveValue(right);
    expect(editors().modified).toHaveValue(left);
  });

  it("Given populated inputs, when cleared, then the diff becomes identical and both are empty", () => {
    render(<DiffToolPanel />);
    fireEvent.click(screen.getByRole("button", { name: /Clear/ }));

    expect(editors().original).toHaveValue("");
    expect(editors().modified).toHaveValue("");
    expect(screen.getByText("Identical")).toBeInTheDocument();
  });

  it("Given a blank diff, then unified view explains that there are no differences", () => {
    render(<DiffToolPanel />);
    fireEvent.click(screen.getByRole("button", { name: /Clear/ }));
    fireEvent.click(screen.getByRole("button", { name: "unified" }));

    expect(screen.getByText("(no differences)")).toBeInTheDocument();
  });
});
