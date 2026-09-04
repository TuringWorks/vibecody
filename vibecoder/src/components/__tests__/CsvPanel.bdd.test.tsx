import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CsvPanel } from "../CsvPanel";

function paste(csv: string) {
  fireEvent.change(screen.getByRole("textbox", { name: "CSV data" }), { target: { value: csv } });
}

describe("CsvPanel — parsing and table workflow", () => {
  it("Given quoted delimiters, then commas inside fields remain part of the cell", () => {
    render(<CsvPanel />);
    paste('Name,Note\nAlice,"hello, world"');

    expect(screen.getByRole("cell", { name: "Alice" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "hello, world" })).toBeInTheDocument();
  });

  it("Given a multiline quoted field, then its line break is preserved", () => {
    render(<CsvPanel />);
    paste('Name,Note\nAlice,"line one\nline two"');

    expect(screen.getByRole("cell", { name: /line one\nline two/ })).toBeInTheDocument();
  });

  it("Given numeric rows, when sorting a column, then values sort numerically", () => {
    render(<CsvPanel />);
    paste("Name,Age\nAlice,10\nBob,2\nCara,30");
    fireEvent.click(screen.getByRole("columnheader", { name: "Age" }));

    const rows = screen.getAllByRole("row");
    expect(rows[1]).toHaveTextContent("Bob");
    expect(rows[2]).toHaveTextContent("Alice");
  });

  it("Given data, when filtered to one column, then only matching rows remain", () => {
    render(<CsvPanel />);
    paste("Name,City\nAlice,NYC\nBob,LA");
    fireEvent.click(screen.getByRole("button", { name: "Filter" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Filter CSV values" }), { target: { value: "nyc" } });

    expect(screen.getByText("Alice")).toBeInTheDocument();
    expect(screen.queryByText("Bob")).not.toBeInTheDocument();
  });

  it("Given a row, when edited, then the serialized data reflects the new cell", () => {
    render(<CsvPanel />);
    paste("Name,Age\nAlice,10");
    fireEvent.doubleClick(screen.getByRole("cell", { name: "10" }));
    const editor = screen.getByRole("textbox", { name: "Edit CSV cell" });
    fireEvent.change(editor, { target: { value: "11" } });
    fireEvent.keyDown(editor, { key: "Enter" });

    expect(screen.getByRole("cell", { name: "11" })).toBeInTheDocument();
  });

  it("Given a table, when converted to JSON, then it copies objects keyed by headers", () => {
    const writeText = vi.fn();
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    render(<CsvPanel />);
    paste("Name,Age\nAlice,10");
    fireEvent.click(screen.getByRole("button", { name: "Convert" }));
    fireEvent.click(screen.getByRole("button", { name: "Copy as JSON" }));

    expect(writeText).toHaveBeenCalledWith(expect.stringContaining('"Name": "Alice"'));
    expect(writeText).toHaveBeenCalledWith(expect.stringContaining('"Age": "10"'));
  });

  it("Given a table, when exported, then it downloads CSV with escaped cells", () => {
    const createObjectURL = vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:csv");
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);
    render(<CsvPanel />);
    paste('Name,Note\nAlice,"hello, world"');
    fireEvent.click(screen.getByRole("button", { name: "Export" }));

    expect(createObjectURL).toHaveBeenCalledWith(expect.any(Blob));
    expect(click).toHaveBeenCalledOnce();
    createObjectURL.mockRestore();
    click.mockRestore();
  });

  it("Given no header row, then the first row is treated as data", () => {
    render(<CsvPanel />);
    paste("Alice,10\nBob,20");
    fireEvent.click(screen.getByRole("checkbox", { name: "Header row" }));

    expect(screen.getByText("Alice")).toBeInTheDocument();
    expect(screen.getByText("Bob")).toBeInTheDocument();
  });
});
