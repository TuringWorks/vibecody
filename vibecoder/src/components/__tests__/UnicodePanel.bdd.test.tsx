import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UnicodePanel } from "../UnicodePanel";

const writeText = vi.fn().mockResolvedValue(undefined);

beforeEach(() => {
  writeText.mockClear();
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
});

function detailValue(label: string): HTMLElement {
  return screen.getByText(label).nextElementSibling as HTMLElement;
}

describe("UnicodePanel — browse and character details", () => {
  it("Given a selected BMP character, then it shows interoperable escape forms", () => {
    render(<UnicodePanel />);
    fireEvent.click(screen.getByTitle("LATIN CAPITAL LETTER A U+0041"));

    expect(screen.getByText("LATIN CAPITAL LETTER A")).toBeInTheDocument();
    expect(screen.getByText("U+0041 · dec 65")).toBeInTheDocument();
    expect(detailValue("HTML entity")).toHaveTextContent("&#x0041;");
    expect(detailValue("CSS escape")).toHaveTextContent("\\0041");
    expect(detailValue("JS escape")).toHaveTextContent("\\u0041");
    expect(detailValue("UTF-8 hex")).toHaveTextContent("41");
  });

  it("Given a supplementary character, then it shows its surrogate pair and UTF-8 bytes", () => {
    render(<UnicodePanel />);
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    fireEvent.change(screen.getByPlaceholderText(/Name \(e\.g\. ARROW\)/), { target: { value: "U+1F680" } });
    fireEvent.click(screen.getByTitle("ROCKET U+1F680"));

    expect(detailValue("JS escape")).toHaveTextContent("\\uD83D\\uDE80");
    expect(detailValue("UTF-8 hex")).toHaveTextContent("F0 9F 9A 80");
    expect(detailValue("Percent-encoded")).toHaveTextContent("%F0%9F%9A%80");
  });
});

describe("UnicodePanel — search and favorites", () => {
  it("Given a name fragment, then it returns matching named characters", () => {
    render(<UnicodePanel />);
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    fireEvent.change(screen.getByPlaceholderText(/Name \(e\.g\. ARROW\)/), { target: { value: "RIGHTWARDS ARROW" } });

    expect(screen.getByText(/results/)).not.toHaveTextContent("0 results");
    expect(screen.getByTitle("RIGHTWARDS ARROW U+2192")).toBeInTheDocument();
  });

  it("Given a surrogate code point, then it rejects the non-scalar value", () => {
    render(<UnicodePanel />);
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    fireEvent.change(screen.getByPlaceholderText(/Name \(e\.g\. ARROW\)/), { target: { value: "U+D800" } });

    expect(screen.getByText("0 results")).toBeInTheDocument();
    expect(screen.getByText("No characters found")).toBeInTheDocument();
  });

  it("Given a favorited character, then Favorites exposes it and Copy all copies the character", () => {
    render(<UnicodePanel />);
    fireEvent.click(screen.getByRole("button", { name: "Latin-1 Supplement" }));
    fireEvent.click(screen.getByTitle("COPYRIGHT SIGN U+00A9"));
    fireEvent.click(screen.getByRole("button", { name: "Add COPYRIGHT SIGN to favorites" }));

    fireEvent.click(screen.getByRole("button", { name: "Favorites (1)" }));
    expect(screen.getByText("1 saved")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Copy all" }));
    expect(writeText).toHaveBeenCalledWith("©");
  });
});

describe("UnicodePanel — input analyzer", () => {
  it("Given mixed BMP and supplementary text, then it reports code points and UTF-8 bytes", () => {
    render(<UnicodePanel />);
    fireEvent.click(screen.getByRole("button", { name: "Analyze" }));
    fireEvent.change(screen.getByPlaceholderText("Paste or type text to analyze each character…"), { target: { value: "A🚀é" } });

    expect(screen.getByText("3 code points · 7 UTF-8 bytes")).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "U+1F680" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "F0 9F 9A 80" })).toBeInTheDocument();
  });

  it("Given an analyzed row, when selected, then the same details are available", () => {
    render(<UnicodePanel />);
    fireEvent.click(screen.getByRole("button", { name: "Analyze" }));
    fireEvent.change(screen.getByPlaceholderText("Paste or type text to analyze each character…"), { target: { value: "→" } });

    fireEvent.click(within(screen.getByRole("row", { name: /U\+2192/ })).getByRole("cell", { name: "→" }));
    expect(screen.getAllByText("RIGHTWARDS ARROW")).toHaveLength(2);
    expect(detailValue("HTML entity")).toHaveTextContent("&#x2192;");
  });

  it("Given a lone surrogate from pasted text, then selecting it does not crash percent encoding", () => {
    render(<UnicodePanel />);
    fireEvent.click(screen.getByRole("button", { name: "Analyze" }));
    fireEvent.change(screen.getByPlaceholderText("Paste or type text to analyze each character…"), { target: { value: "\uD800" } });

    fireEvent.click(screen.getByRole("row", { name: /U\+D800/ }));

    expect(detailValue("Percent-encoded")).toHaveTextContent("invalid Unicode scalar");
    expect(detailValue("UTF-8 hex")).toHaveTextContent("EF BF BD");
  });
});
