import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MarkdownPanel } from "../MarkdownPanel";

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function arrangeFiles() {
  invokeMock.mockImplementation((command: string, args?: { path?: string }) => {
    if (command === "list_markdown_files") {
      return Promise.resolve([
        { path: "/work/alpha.md", name: "alpha.md", size_bytes: 1024 },
        { path: "/work/beta.mdx", name: "beta.mdx", size_bytes: 2048 },
      ]);
    }
    if (command === "read_file" && args?.path === "/work/alpha.md") return Promise.resolve("# Alpha\n\nBody");
    if (command === "write_file") return Promise.resolve(undefined);
    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}

beforeEach(() => {
  invokeMock.mockReset();
  arrangeFiles();
});

describe("MarkdownPanel — workspace files", () => {
  it("Given a workspace, then it lists and filters Markdown files", async () => {
    render(<MarkdownPanel workspacePath="/work" />);

    expect(await screen.findByRole("button", { name: /alpha\.md/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /beta\.mdx/ })).toBeInTheDocument();

    fireEvent.change(screen.getByRole("textbox", { name: "Filter Markdown files" }), { target: { value: "beta" } });
    expect(screen.queryByRole("button", { name: /alpha\.md/ })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /beta\.mdx/ })).toBeInTheDocument();
  });

  it("Given a listed file, when opened, then its contents replace the editor without marking it dirty", async () => {
    render(<MarkdownPanel workspacePath="/work" />);
    fireEvent.click(await screen.findByRole("button", { name: /alpha\.md/ }));

    await waitFor(() => expect(screen.getByRole("textbox", { name: "Markdown editor" })).toHaveValue("# Alpha\n\nBody"));
    expect(screen.getByText("alpha.md")).toBeInTheDocument();
    expect(screen.queryByText("alpha.md •")).not.toBeInTheDocument();
  });

  it("Given a file read failure, then the error is announced", async () => {
    invokeMock.mockImplementation((command: string) => command === "list_markdown_files"
      ? Promise.resolve([{ path: "/work/bad.md", name: "bad.md", size_bytes: 1 }])
      : Promise.reject(new Error("permission denied")));
    render(<MarkdownPanel workspacePath="/work" />);
    fireEvent.click(await screen.findByRole("button", { name: /bad\.md/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent("permission denied");
  });
});

describe("MarkdownPanel — editing and saving", () => {
  it("Given a caret position, when Tab is pressed, then two spaces are inserted and the caret advances by two", () => {
    render(<MarkdownPanel workspacePath="/work" />);
    const editor = screen.getByRole("textbox", { name: "Markdown editor" }) as HTMLTextAreaElement;
    fireEvent.change(editor, { target: { value: "ab" } });
    editor.setSelectionRange(1, 1);

    fireEvent.keyDown(editor, { key: "Tab" });

    expect(editor).toHaveValue("a  b");
    expect(editor.selectionStart).toBe(3);
  });

  it("Given a new note, then its filename can be changed before saving", async () => {
    render(<MarkdownPanel workspacePath="/work" />);
    fireEvent.click(screen.getByRole("button", { name: "New file" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Markdown filename" }), { target: { value: "notes.md" } });
    fireEvent.change(screen.getByRole("textbox", { name: "Markdown editor" }), { target: { value: "# Notes" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("write_file", {
      path: "/work/notes.md",
      content: "# Notes",
    }));
    expect(await screen.findByRole("status")).toHaveTextContent("Saved");
  });

  it("Given no workspace, when saving, then the actionable failure is announced", async () => {
    render(<MarkdownPanel workspacePath={null} />);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("No workspace — cannot save");
    expect(invokeMock).not.toHaveBeenCalledWith("write_file", expect.anything());
  });

  it("Given a write failure, then it is announced as an error", async () => {
    invokeMock.mockImplementation((command: string) => command === "list_markdown_files"
      ? Promise.resolve([])
      : Promise.reject(new Error("disk full")));
    render(<MarkdownPanel workspacePath="/work" />);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Save failed: Error: disk full");
  });

  it("Given an edited existing file, then Command-S saves back to the same path", async () => {
    render(<MarkdownPanel workspacePath="/work" />);
    fireEvent.click(await screen.findByRole("button", { name: /alpha\.md/ }));
    const editor = screen.getByRole("textbox", { name: "Markdown editor" });
    await waitFor(() => expect(editor).toHaveValue("# Alpha\n\nBody"));
    fireEvent.change(editor, { target: { value: "updated" } });
    fireEvent.keyDown(editor, { key: "s", metaKey: true });

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("write_file", {
      path: "/work/alpha.md",
      content: "updated",
    }));
  });
});

describe("MarkdownPanel — preview and export", () => {
  it("Given 201 words, then reading time rounds up to two minutes", () => {
    render(<MarkdownPanel workspacePath={null} />);
    fireEvent.change(screen.getByRole("textbox", { name: "Markdown editor" }), {
      target: { value: Array.from({ length: 201 }, () => "word").join(" ") },
    });

    expect(screen.getByText("2 min read")).toBeInTheDocument();
  });

  it("Given Markdown content, when exporting, then it downloads a standalone HTML file", () => {
    const createObjectURL = vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:markdown");
    const revokeObjectURL = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => undefined);
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);
    render(<MarkdownPanel workspacePath="/work" />);
    fireEvent.change(screen.getByRole("textbox", { name: "Markdown editor" }), { target: { value: "# Export Me" } });

    fireEvent.click(screen.getByRole("button", { name: "Export HTML" }));

    expect(createObjectURL).toHaveBeenCalledWith(expect.any(Blob));
    expect(click).toHaveBeenCalledOnce();
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:markdown");
    createObjectURL.mockRestore();
    revokeObjectURL.mockRestore();
    click.mockRestore();
  });
});
