/**
 * BDD: the draw.io panel behaves like a file editor, not a scratch canvas.
 *
 * Every scenario here is a thing the panel got wrong, reported by a user
 * looking at the screen:
 *
 *  - **"Save & Exit doesn't really exit."** It didn't. The embed URL passed
 *    `noSaveBtn=1`, which makes draw.io show *Save & Exit* in place of *Save*,
 *    and nothing on this side handled the `exit` event — so the button saved
 *    and then sat there. A control that does half of what it says is worse than
 *    one that isn't offered.
 *  - **"Where does the file get saved?"** To `<workspace>/diagrams/diagram.drawio`,
 *    always, for every diagram — so the second one silently replaced the first.
 *    The toolbar said "Saved to workspace" and never named the file. And the
 *    call was `.catch(() => {})` followed by an unconditional success message,
 *    so a failed write reported as a save.
 *  - **"Existing .drawio files should preview and load in an editor."** There
 *    was no way to reach one: the editor opened blank every time.
 *  - **"Export should save to the open workspace."** There was no export.
 */
import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { DrawioEditorPanel } from "../DrawioEditorPanel";

/** One row as `list_drawio_files` returns it. */
function file(over: Partial<Record<string, unknown>> = {}) {
  return {
    path: "docs/architecture.drawio",
    name: "architecture.drawio",
    size_bytes: 4096,
    modified_unix: Math.floor(Date.now() / 1000) - 120,
    pages: 2,
    vertices: 10,
    edges: 6,
    is_embedded_export: false,
    ...over,
  };
}

const TEMPLATES = [
  { id: "c4_context", label: "C4 — System Context", kind: "c4_context", summary: "One system and its people" },
];

/**
 * Default backend: one diagram on disk, one template, saves succeed.
 * Individual scenarios override the one command they are about.
 */
function backend(over: Record<string, (args: Record<string, unknown>) => unknown> = {}) {
  mockInvoke.mockImplementation((cmd: string, args: Record<string, unknown> = {}) => {
    if (over[cmd]) return Promise.resolve(over[cmd](args));
    switch (cmd) {
      case "list_drawio_files":
        return Promise.resolve([file()]);
      case "list_drawio_templates":
        return Promise.resolve(TEMPLATES);
      case "read_drawio_file":
        return Promise.resolve("<mxfile><diagram><mxGraphModel><root/></mxGraphModel></diagram></mxfile>");
      case "get_drawio_template":
        return Promise.resolve("<mxfile><diagram><mxGraphModel><root/></mxGraphModel></diagram></mxfile>");
      case "save_drawio_file":
        return Promise.resolve({
          path: String(args.relativePath),
          absolute_path: `/ws/${args.relativePath}`,
          size_bytes: 2048,
          created: true,
        });
      case "export_drawio_file":
        return Promise.resolve({
          path: String(args.relativePath),
          absolute_path: `/ws/${args.relativePath}`,
          size_bytes: 91_000,
          created: true,
        });
      default:
        return Promise.resolve(null);
    }
  });
}

/** How many times `cmd` was invoked, and with what. */
function callsTo(cmd: string): Record<string, unknown>[] {
  return mockInvoke.mock.calls
    .filter(([c]) => c === cmd)
    .map(([, args]) => (args ?? {}) as Record<string, unknown>);
}

/** The editor iframe, with a stubbed `contentWindow` we can assert against. */
function editorIframe(): { el: HTMLIFrameElement; posted: ReturnType<typeof vi.fn> } {
  const el = document.querySelector('iframe[title="Draw.io Editor"]') as HTMLIFrameElement;
  const posted = vi.fn();
  Object.defineProperty(el, "contentWindow", {
    value: { postMessage: posted },
    configurable: true,
  });
  return { el, posted };
}

/**
 * Deliver an embed-protocol message as if it came from the editor iframe.
 *
 * `source` is defined, not assigned: on a `MessageEvent` it is a getter-only
 * property, and the panel checks it to tell its own iframe's messages from
 * every other frame's.
 */
function fromEditor(el: HTMLIFrameElement, payload: unknown) {
  const evt = new MessageEvent("message", { data: JSON.stringify(payload) });
  Object.defineProperty(evt, "source", { value: el.contentWindow, configurable: true });
  fireEvent(window, evt);
}

/** Open the Editor tab with a diagram loaded from the Diagrams list. */
async function openTheDiagram() {
  render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
  const card = await screen.findByTitle("Open docs/architecture.drawio");
  fireEvent.click(card);
  await screen.findByTitle("docs/architecture.drawio");
}

beforeEach(() => {
  mockInvoke.mockReset();
  backend();
  vi.spyOn(window, "confirm").mockReturnValue(true);
});

afterEach(() => {
  vi.restoreAllMocks();
});

// ─────────────────────────────────────────────────────────────────────────────

describe('Given the embedded editor is configured', () => {
  it("When the Editor tab opens, Then no Save & Exit button is asked for", async () => {
    await openTheDiagram();
    const src = (document.querySelector('iframe[title="Draw.io Editor"]') as HTMLIFrameElement).src;

    // `noSaveBtn=1` is what made draw.io render *Save & Exit* instead of *Save*.
    expect(src).not.toContain("noSaveBtn");
    expect(src).toContain("noExitBtn=1");
    expect(src).toContain("saveAndExit=0");
  });

  it("When the Editor tab opens, Then autosave is on so Save writes what is on screen", async () => {
    // The handler for `autosave` already existed; the URL never asked for the
    // event, so Save could write a diagram several edits stale.
    await openTheDiagram();
    const src = (document.querySelector('iframe[title="Draw.io Editor"]') as HTMLIFrameElement).src;
    expect(src).toContain("autosave=1");
  });

  it("When the editor sends exit anyway, Then the document closes rather than nothing happening", async () => {
    await openTheDiagram();
    const { el } = editorIframe();

    fromEditor(el, { event: "exit" });

    // Back to the diagram list, with no document open — the honest meaning of
    // "exit" for a panel that is a tab. Doing nothing is what the user saw.
    await waitFor(() => expect(screen.getByPlaceholderText("Filter diagrams…")).toBeInTheDocument());
  });
});

describe("Given a diagram is open", () => {
  it("When it is saved, Then the message names the file that was written", async () => {
    await openTheDiagram();
    const { el } = editorIframe();
    fromEditor(el, { event: "autosave", xml: "<mxfile>edited</mxfile>" });

    fireEvent.click(await screen.findByRole("button", { name: "Save" }));

    // "Saved to workspace" was true and useless: it is exactly the question the
    // user was left holding.
    const status = await screen.findByRole("status");
    expect(status).toHaveTextContent("docs/architecture.drawio");
  });

  it("When it is saved, Then it goes to its own path — not a shared diagram.drawio", async () => {
    await openTheDiagram();
    const { el } = editorIframe();
    fromEditor(el, { event: "autosave", xml: "<mxfile>edited</mxfile>" });
    fireEvent.click(await screen.findByRole("button", { name: "Save" }));

    await waitFor(() => expect(callsTo("save_drawio_file")).toHaveLength(1));
    expect(callsTo("save_drawio_file")[0].relativePath).toBe("docs/architecture.drawio");
  });

  it("When the write fails, Then the failure is shown — not a success message", async () => {
    backend({
      save_drawio_file: () => {
        throw new Error("Permission denied");
      },
    });
    await openTheDiagram();
    const { el } = editorIframe();
    fromEditor(el, { event: "autosave", xml: "<mxfile>edited</mxfile>" });

    fireEvent.click(await screen.findByRole("button", { name: "Save" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Permission denied");
    expect(screen.queryByText(/Saved to workspace/)).toBeNull();
  });

  it("When the editor's own Save fires, Then it reaches the disk", async () => {
    // Ctrl+S and File → Save both arrive as this event. Updating local state
    // and stopping there is what makes the button a lie.
    await openTheDiagram();
    const { el } = editorIframe();

    fromEditor(el, { event: "save", xml: "<mxfile>from-ctrl-s</mxfile>" });

    await waitFor(() => expect(callsTo("save_drawio_file")).toHaveLength(1));
    expect(callsTo("save_drawio_file")[0].xml).toBe("<mxfile>from-ctrl-s</mxfile>");
  });

  it("When there are unsaved edits, Then the toolbar says so", async () => {
    await openTheDiagram();
    const { el } = editorIframe();
    expect(screen.queryByTitle("Unsaved changes")).toBeNull();

    fromEditor(el, { event: "autosave", xml: "<mxfile>edited</mxfile>" });

    await waitFor(() => expect(screen.getAllByTitle("Unsaved changes").length).toBeGreaterThan(0));
  });
});

describe("Given a diagram that has never been saved", () => {
  it("When Save is pressed, Then it asks for a name instead of picking one", async () => {
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
    fireEvent.click(await screen.findByRole("button", { name: "New diagram" }));

    fireEvent.click(await screen.findByRole("button", { name: "Save…" }));

    expect(await screen.findByLabelText("Save as")).toBeInTheDocument();
    expect(callsTo("save_drawio_file")).toHaveLength(0);
  });

  it("When a bare name is given, Then it lands in diagrams/ with a .drawio extension", async () => {
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
    fireEvent.click(await screen.findByRole("button", { name: "New diagram" }));
    fireEvent.click(await screen.findByRole("button", { name: "Save…" }));

    fireEvent.change(await screen.findByLabelText("Save as"), { target: { value: "auth-flow" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(callsTo("save_drawio_file")).toHaveLength(1));
    expect(callsTo("save_drawio_file")[0].relativePath).toBe("diagrams/auth-flow.drawio");
  });

  it("When a full path is given, Then it is used exactly as typed", async () => {
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
    fireEvent.click(await screen.findByRole("button", { name: "New diagram" }));
    fireEvent.click(await screen.findByRole("button", { name: "Save…" }));

    fireEvent.change(await screen.findByLabelText("Save as"), {
      target: { value: "docs/adr/0007-topology.drawio" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(callsTo("save_drawio_file")).toHaveLength(1));
    expect(callsTo("save_drawio_file")[0].relativePath).toBe("docs/adr/0007-topology.drawio");
  });
});

describe("Given the workspace already has diagrams", () => {
  it("When the panel opens, Then they are listed", async () => {
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
    expect(await screen.findByText("architecture.drawio")).toBeInTheDocument();
    expect(screen.getByText("docs/architecture.drawio")).toBeInTheDocument();
  });

  it("When one is chosen, Then it loads into the editor and Save writes back to it", async () => {
    await openTheDiagram();
    expect(callsTo("read_drawio_file")[0].relativePath).toBe("docs/architecture.drawio");
    // The path is on screen at all times, which is the answer to "where does
    // this get saved".
    expect(screen.getByTitle("docs/architecture.drawio")).toBeInTheDocument();
  });

  it("When the listing fails, Then it says so rather than showing an empty workspace", async () => {
    backend({
      list_drawio_files: () => {
        throw new Error("EACCES");
      },
    });
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);

    // An empty list is indistinguishable from "no diagrams here", which sends
    // the user looking for the wrong problem.
    expect(await screen.findByRole("alert")).toHaveTextContent("EACCES");
  });

  it("When a file was too large to count, Then it says so instead of showing zero", async () => {
    backend({
      list_drawio_files: () => [file({ pages: null, vertices: null, edges: null, size_bytes: 5_000_000 })],
    });
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);

    // `0 pages · 0 shapes` would describe the largest diagram in the repo as
    // an empty one.
    expect(await screen.findByText(/not counted \(large file\)/)).toBeInTheDocument();
    expect(screen.queryByText(/0 pages/)).toBeNull();
  });

  it("When the filesystem reported no mtime, Then no time is shown at all", async () => {
    backend({ list_drawio_files: () => [file({ modified_unix: null })] });
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);

    const card = await screen.findByTitle("Open docs/architecture.drawio");
    // Never "just now" — that would assert a fact nobody checked.
    expect(within(card).queryByText(/ago|just now/)).toBeNull();
  });

  it("When an editable SVG export is opened, Then it will not silently overwrite the picture", async () => {
    backend({
      list_drawio_files: () => [
        file({ path: "docs/flow.drawio.svg", name: "flow.drawio.svg", is_embedded_export: true }),
      ],
    });
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
    fireEvent.click(await screen.findByTitle("Open docs/flow.drawio.svg"));

    // Its diagram opens, but it has no path — Save has to ask for a name,
    // because writing XML back into the SVG would leave the picture showing
    // the previous version.
    expect(await screen.findByText("untitled — not saved")).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Save…" })).toBeInTheDocument();
  });
});

describe("Given the user wants a picture out of the editor", () => {
  it("When PNG is chosen, Then the editor is asked to export it", async () => {
    await openTheDiagram();
    const { posted } = editorIframe();

    fireEvent.click(screen.getByRole("button", { name: "PNG" }));

    const request = posted.mock.calls.map(([m]) => JSON.parse(String(m))).find((m) => m.action === "export");
    expect(request).toMatchObject({ action: "export", format: "png" });
  });

  it("When the export comes back, Then it is written into the workspace beside the diagram", async () => {
    await openTheDiagram();
    const { el } = editorIframe();
    fireEvent.click(screen.getByRole("button", { name: "PNG" }));

    fromEditor(el, { event: "export", format: "png", data: "data:image/png;base64,iVBORw0KGgo=" });

    await waitFor(() => expect(callsTo("export_drawio_file")).toHaveLength(1));
    // draw.io's own export menu downloads through the browser, which the Tauri
    // webview does not surface — the file simply never appeared anywhere.
    expect(callsTo("export_drawio_file")[0].relativePath).toBe("docs/architecture.png");
  });

  it("When the export is written, Then the message names the file", async () => {
    await openTheDiagram();
    const { el } = editorIframe();
    fireEvent.click(screen.getByRole("button", { name: "SVG" }));
    fromEditor(el, { event: "export", format: "svg", data: "data:image/svg+xml;base64,PHN2Zy8+" });

    expect(await screen.findByRole("status")).toHaveTextContent("docs/architecture.svg");
  });

  it("When the write fails, Then the failure is shown", async () => {
    backend({
      export_drawio_file: () => {
        throw new Error("No space left on device");
      },
    });
    await openTheDiagram();
    const { el } = editorIframe();
    fireEvent.click(screen.getByRole("button", { name: "PNG" }));
    fromEditor(el, { event: "export", format: "png", data: "data:image/png;base64,iVBORw0KGgo=" });

    expect(await screen.findByRole("alert")).toHaveTextContent("No space left on device");
  });
});

describe("Given the Templates tab", () => {
  it("When it opens, Then it offers only templates the backend actually has", async () => {
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
    fireEvent.click(screen.getByRole("button", { name: "Templates" }));

    // The list used to be hard-coded here while the backend had none of them —
    // eight cards, every one returning a single labelled rectangle.
    expect(await screen.findByText("C4 — System Context")).toBeInTheDocument();
    expect(screen.queryByText("Microservices Architecture")).toBeNull();
  });

  it("When a template is missing, Then it says so rather than opening a placeholder", async () => {
    backend({
      get_drawio_template: () => {
        throw new Error("No template named `c4_context`");
      },
    });
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
    fireEvent.click(screen.getByRole("button", { name: "Templates" }));
    fireEvent.click(await screen.findByText("C4 — System Context"));

    expect(await screen.findByRole("alert")).toHaveTextContent("No template named");
  });
});

describe("Given the MCP bridge", () => {
  it("When a command runs, Then it is scoped to the open workspace", async () => {
    render(<DrawioEditorPanel workspacePath="/ws" provider="anthropic" />);
    fireEvent.click(screen.getByRole("button", { name: "MCP Bridge" }));
    fireEvent.change(await screen.findByLabelText(/File path/), {
      target: { value: "docs/architecture.drawio" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Execute/ }));

    // Without the workspace the bridge is a general-purpose read/write
    // primitive for anywhere the deny-list happens not to cover.
    await waitFor(() => expect(callsTo("execute_drawio_mcp")).toHaveLength(1));
    expect(callsTo("execute_drawio_mcp")[0].workspacePath).toBe("/ws");
  });
});
