import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { JsonToolsPanel } from "../JsonToolsPanel";

function inputJson(value: string) {
  fireEvent.change(screen.getByRole("textbox", { name: "JSON input" }), {
    target: { value },
  });
}

describe("JsonToolsPanel — formatting and validation", () => {
  it("Given nested unsorted JSON, when sorting keys, then every object level is sorted", () => {
    render(<JsonToolsPanel />);
    inputJson('{"z":{"b":1,"a":2},"a":0}');

    fireEvent.click(screen.getByRole("button", { name: "Sort Keys" }));

    expect(screen.getByRole("textbox", { name: "JSON input" })).toHaveValue(
      '{\n  "a": 0,\n  "z": {\n    "a": 2,\n    "b": 1\n  }\n}',
    );
  });

  it("Given malformed JSON, then formatting actions are disabled and the parse error is announced", () => {
    render(<JsonToolsPanel />);
    inputJson("{");

    expect(screen.getByRole("button", { name: "Prettify" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Minify" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent(/JSON|property|position|expected/i);
  });

  it("Given valid JSON null, then it remains valid and can be minified", () => {
    render(<JsonToolsPanel />);
    inputJson("null");

    expect(screen.getByText("✓ Valid JSON")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Minify" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Minify" }));
    expect(screen.getByRole("textbox", { name: "JSON input" })).toHaveValue("null");
  });
});

describe("JsonToolsPanel — generated outputs", () => {
  it.each([
    ["false", "false"],
    ["0", "0"],
    ["null", "null"],
  ])("Given valid JSON %s, then YAML emits the primitive", (json, expected) => {
    render(<JsonToolsPanel />);
    inputJson(json);
    fireEvent.click(screen.getByRole("button", { name: "YAML" }));

    const output = screen.getByText("YAML OUTPUT").parentElement!.nextElementSibling!;
    expect(output).toHaveTextContent(expected);
    expect(screen.queryByText(/Fix JSON errors/)).not.toBeInTheDocument();
  });

  it("Given a heterogeneous array, then TypeScript preserves every observed item type", () => {
    render(<JsonToolsPanel />);
    inputJson('{"values":[1,"two",true]}');
    fireEvent.click(screen.getByRole("button", { name: "TypeScript" }));

    expect(screen.getByText(/values: \(number \| string \| boolean\)\[\];/)).toBeInTheDocument();
  });

  it("Given nested objects, then TypeScript generates linked interfaces", () => {
    render(<JsonToolsPanel />);
    inputJson('{"user":{"name":"Ada"}}');
    fireEvent.click(screen.getByRole("button", { name: "TypeScript" }));

    const output = screen.getByText(/export interface Root/);
    expect(output).toHaveTextContent("export interface User");
    expect(output).toHaveTextContent("user: User;");
    expect(output).toHaveTextContent("name: string;");
  });
});

describe("JsonToolsPanel — path queries", () => {
  it("Given a quoted bracket key, then the query returns a nested false value", () => {
    render(<JsonToolsPanel />);
    inputJson('{"feature.flags":{"enabled":false}}');
    fireEvent.click(screen.getByRole("button", { name: "Query" }));
    fireEvent.change(screen.getByRole("textbox", { name: "JSON query path" }), {
      target: { value: '["feature.flags"].enabled' },
    });

    expect(screen.getByText("false")).toBeInTheDocument();
    expect(screen.queryByText(/Path not found/)).not.toBeInTheDocument();
  });

  it("Given a missing path, then it identifies the failing segment", () => {
    render(<JsonToolsPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Query" }));
    fireEvent.change(screen.getByRole("textbox", { name: "JSON query path" }), {
      target: { value: "user.missing.value" },
    });

    expect(screen.getByText('Path not found at "missing"')).toBeInTheDocument();
  });

  it("Given a query result, then its copy control writes the serialized value", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    render(<JsonToolsPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Query" }));

    const result = screen.getByText("RESULT").parentElement!;
    fireEvent.click(within(result).getByRole("button", { name: "Copy query result" }));

    expect(writeText).toHaveBeenCalledWith('"San Francisco"');
  });
});
