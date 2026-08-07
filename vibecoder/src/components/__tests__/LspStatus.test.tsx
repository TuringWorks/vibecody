import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act, fireEvent } from "@testing-library/react";
import { LspStatus, describeSupport } from "../LspStatus";
import type { LspLanguageSupport } from "../../lib/lsp";

const support = (
  overrides: Partial<LspLanguageSupport> = {},
): LspLanguageSupport => ({
  language: "rust",
  state: "running",
  detail: "",
  supported: true,
  completionTriggerCharacters: [".", "::"],
  signatureHelpTriggerCharacters: ["("],
  ...overrides,
});

describe("describeSupport", () => {
  it("reports a running server as OK and retryable", () => {
    const display = describeSupport(support());
    expect(display.label).toContain("rust");
    expect(display.tone).toBe("ok");
    expect(display.actionable).toBe(true);
  });

  it("surfaces the install hint for a missing server", () => {
    const display = describeSupport(
      support({
        state: "not_installed",
        detail: "rust-analyzer — install: rustup component add rust-analyzer",
      }),
    );
    expect(display.tone).toBe("warn");
    expect(display.title).toContain("rustup component add rust-analyzer");
    expect(display.actionable).toBe(true);
  });

  it("surfaces the failure reason for a server that would not start", () => {
    const display = describeSupport(
      support({ state: "failed", detail: "'gopls' failed to start: broken pipe" }),
    );
    expect(display.tone).toBe("warn");
    expect(display.title).toContain("broken pipe");
  });

  it("shows nothing for a language that has no server at all", () => {
    // Not a problem to report — there is no server to install.
    expect(describeSupport(support({ state: "unconfigured" })).label).toBe("");
  });

  it("does not offer a retry while the server is still starting", () => {
    expect(describeSupport(support({ state: "available" })).actionable).toBe(
      false,
    );
  });
});

describe("<LspStatus />", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  const invokeStub = (
    answers: LspLanguageSupport[] | LspLanguageSupport,
    onRestart?: () => void,
  ) => {
    const queue = Array.isArray(answers) ? [...answers] : [answers];
    let last = queue[0];
    return vi.fn(async (command: string) => {
      if (command === "lsp_restart_language") {
        onRestart?.();
        return null;
      }
      if (command === "lsp_language_support") {
        // Each probe consumes the next canned answer; the last one repeats.
        last = queue.length > 0 ? (queue.shift() as LspLanguageSupport) : last;
        return last;
      }
      return null;
    }) as never;
  };

  it("shows the running server for the active file", async () => {
    render(
      <LspStatus
        filePath="/w/src/main.rs"
        workspaceRoot="/w"
        invoke={invokeStub(support())}
      />,
    );
    expect(await screen.findByRole("button")).toHaveTextContent(
      "IntelliSense: rust",
    );
  });

  it("credits Monaco's own service instead of probing for a server", () => {
    // Probing here would report `typescript-language-server` as missing and
    // warn about a gap that does not exist — Monaco handles TypeScript itself.
    const invoke = invokeStub(support());
    render(
      <LspStatus
        filePath="/w/src/App.tsx"
        workspaceRoot="/w"
        invoke={invoke}
      />,
    );
    expect(screen.getByText("IntelliSense: built-in")).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("offers no retry for a built-in service — there is nothing to restart", () => {
    render(
      <LspStatus
        filePath="/w/a.css"
        workspaceRoot="/w"
        invoke={invokeStub(support())}
      />,
    );
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("probes a server for .vue, which only looks like a built-in language", () => {
    // `.vue` highlights as `html`, and `html` *is* Monaco-serviced. Keying the
    // built-in check on the Monaco language would silently skip Volar — the
    // one thing that understands script blocks and typed templates.
    const invoke = invokeStub(support({ language: "vue" }));
    render(
      <LspStatus filePath="/w/App.vue" workspaceRoot="/w" invoke={invoke} />,
    );
    expect(screen.queryByText("IntelliSense: built-in")).not.toBeInTheDocument();
    expect(invoke).toHaveBeenCalled();
  });

  it("renders nothing for a file with no language server", () => {
    const invoke = invokeStub(support());
    const { container } = render(
      <LspStatus filePath="/w/notes.txt" workspaceRoot="/w" invoke={invoke} />,
    );
    expect(container).toBeEmptyDOMElement();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("renders nothing when no file is open", () => {
    const { container } = render(
      <LspStatus filePath={null} workspaceRoot="/w" invoke={invokeStub(support())} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("re-checks once while the server is still starting, then settles", async () => {
    // The editor starts the server in parallel with the first probe, so the
    // first answer is usually "available" and the second "running".
    const invoke = invokeStub([
      support({ state: "available", detail: "rust-analyzer" }),
      support({ state: "running" }),
    ]);
    render(
      <LspStatus filePath="/w/src/main.rs" workspaceRoot="/w" invoke={invoke} />,
    );

    expect(await screen.findByRole("button")).toHaveTextContent("starting");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2500);
    });
    await waitFor(() =>
      expect(screen.getByRole("button")).toHaveTextContent(
        "IntelliSense: rust",
      ),
    );
  });

  it("does not keep polling after it settles", async () => {
    const invoke = invokeStub(support({ state: "running" }));
    render(
      <LspStatus filePath="/w/src/main.rs" workspaceRoot="/w" invoke={invoke} />,
    );
    await screen.findByRole("button");
    const afterFirstProbe = (invoke as unknown as { mock: { calls: unknown[] } })
      .mock.calls.length;

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });

    expect(
      (invoke as unknown as { mock: { calls: unknown[] } }).mock.calls.length,
    ).toBe(afterFirstProbe);
  });

  it("restarts the language server when clicked, then re-probes", async () => {
    const onRestart = vi.fn();
    const invoke = invokeStub(
      [
        support({ state: "not_installed", detail: "rust-analyzer — install: rustup" }),
        support({ state: "running" }),
      ],
      onRestart,
    );
    render(
      <LspStatus filePath="/w/src/main.rs" workspaceRoot="/w" invoke={invoke} />,
    );

    const button = await screen.findByRole("button");
    expect(button).toHaveTextContent("No IntelliSense");
    await act(async () => {
      fireEvent.click(button);
    });

    expect(onRestart).toHaveBeenCalledTimes(1);
    await waitFor(() =>
      expect(screen.getByRole("button")).toHaveTextContent(
        "IntelliSense: rust",
      ),
    );
  });

  it("keeps the last known state when a probe fails", async () => {
    const invoke = vi.fn(async (command: string) => {
      if (command === "lsp_language_support") return support({ state: "running" });
      throw new Error("daemon down");
    }) as never;
    render(
      <LspStatus filePath="/w/src/main.rs" workspaceRoot="/w" invoke={invoke} />,
    );

    const button = await screen.findByRole("button");
    await act(async () => {
      fireEvent.click(button);
    });

    await waitFor(() =>
      expect(screen.getByRole("button")).toHaveTextContent(
        "IntelliSense: rust",
      ),
    );
  });

  it("re-probes when the active file switches language", async () => {
    const invoke = vi.fn(async (_command: string, args?: Record<string, unknown>) =>
      support({ language: String(args?.language ?? "") }),
    ) as never;
    const { rerender } = render(
      <LspStatus filePath="/w/a.rs" workspaceRoot="/w" invoke={invoke} />,
    );
    expect(await screen.findByRole("button")).toHaveTextContent(
      "IntelliSense: rust",
    );

    rerender(
      <LspStatus filePath="/w/b.py" workspaceRoot="/w" invoke={invoke} />,
    );
    await waitFor(() =>
      expect(screen.getByRole("button")).toHaveTextContent(
        "IntelliSense: python",
      ),
    );
  });

  it("does not re-probe when switching between files of the same language", async () => {
    const invoke = invokeStub(support({ state: "running" }));
    const { rerender } = render(
      <LspStatus filePath="/w/a.rs" workspaceRoot="/w" invoke={invoke} />,
    );
    await screen.findByRole("button");
    const before = (invoke as unknown as { mock: { calls: unknown[] } }).mock
      .calls.length;

    rerender(<LspStatus filePath="/w/b.rs" workspaceRoot="/w" invoke={invoke} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });

    expect(
      (invoke as unknown as { mock: { calls: unknown[] } }).mock.calls.length,
    ).toBe(before);
  });

  it("exposes the reason as an accessible label, not just a tooltip", async () => {
    render(
      <LspStatus
        filePath="/w/src/main.rs"
        workspaceRoot="/w"
        invoke={invokeStub(
          support({
            state: "not_installed",
            detail: "rust-analyzer — install: rustup component add rust-analyzer",
          }),
        )}
      />,
    );
    const button = await screen.findByRole("button");
    expect(button.getAttribute("aria-label")).toContain("rustup");
  });
});
