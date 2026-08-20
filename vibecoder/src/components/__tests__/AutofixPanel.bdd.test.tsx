/**
 * BDD: which element App.css stretches to fill the panel.
 *
 * `.panel-container > div:last-child:not(.panel-header):not(.panel-tab-bar)`
 * gets `flex: 1` — the catch-all that gives ~78 panels a scroll area. It lands
 * on whatever renders last, so a panel built from sibling blocks hands that
 * rule a different element in every state. While an autofix run was in flight
 * the last block was the framework-selector row, which duly stretched: a
 * full-panel-height `<select>` next to a full-panel-height Suspend button.
 *
 * jsdom does no layout, so the assertion is on the structure the rule keys off
 * — one `.panel-body` wrapper, last, in every state — rather than on pixels.
 */

import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { AutofixPanel } from "../AutofixPanel";

const RESULT = {
  framework: "clippy",
  files_changed: 2,
  diff: "diff --git a/x.rs b/x.rs\n+fixed\n",
  stdout: "",
};

function pendingRun() {
  let finish: (value: unknown) => void = () => {};
  let fail: (reason: unknown) => void = () => {};
  const promise = new Promise((resolve, reject) => { finish = resolve; fail = reject; });
  return { promise, finish, fail };
}

/** The element App.css will stretch. */
function stretchedChild(): Element {
  const container = document.querySelector(".panel-container")!;
  return container.lastElementChild!;
}

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === "detect_coverage_tool") return "cargo-llvm-cov";
    return null;
  });
});

async function renderPanel() {
  render(<AutofixPanel workspacePath="/repo" />);
  await waitFor(() => expect(screen.getByText(/detected: clippy/)).toBeTruthy());
}

describe("Given the Codemod & Auto-Fix panel", () => {
  it("When idle, Then the stretched child is the scroll wrapper", async () => {
    await renderPanel();
    expect(stretchedChild().className).toContain("panel-body");
  });

  it("When a run is in flight, Then the selector row is not the stretched child", async () => {
    const run = pendingRun();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "detect_coverage_tool") return "cargo-llvm-cov";
      if (cmd === "run_autofix") return run.promise;
      return null;
    });
    await renderPanel();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /run autofix/i }));
    });

    // The reported screenshot: everything else was conditional on *not*
    // running, leaving the row holding the select and Suspend last in the DOM.
    const stretched = stretchedChild();
    expect(stretched.className).toContain("panel-body");
    expect(stretched.querySelector("select"), "the select belongs inside the wrapper").toBeTruthy();

    await act(async () => { run.finish(RESULT); });
  });

  it("When a run is in flight, Then the panel says it is running", async () => {
    const run = pendingRun();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "detect_coverage_tool") return "cargo-llvm-cov";
      if (cmd === "run_autofix") return run.promise;
      return null;
    });
    await renderPanel();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /run autofix/i }));
    });

    // Previously the panel went blank for the whole run.
    expect(screen.getByRole("status").textContent).toMatch(/Running clippy/);

    await act(async () => { run.finish(RESULT); });
  });

  it("When a run fails, Then the error state keeps the same wrapper", async () => {
    const run = pendingRun();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "detect_coverage_tool") return "cargo-llvm-cov";
      if (cmd === "run_autofix") return run.promise;
      return null;
    });
    await renderPanel();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /run autofix/i }));
    });
    await act(async () => { run.fail("clippy exploded"); });

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("clippy exploded"));
    expect(stretchedChild().className).toContain("panel-body");
  });

  it("When results arrive, Then the diff still lives inside the wrapper", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "detect_coverage_tool") return "cargo-llvm-cov";
      if (cmd === "run_autofix") return RESULT;
      return null;
    });
    await renderPanel();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /run autofix/i }));
    });

    await waitFor(() => expect(screen.getByText(/2 files changed/)).toBeTruthy());
    expect(stretchedChild().className).toContain("panel-body");
  });
});
