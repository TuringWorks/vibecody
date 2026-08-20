/**
 * BDD: the panel has to show a long test run *while it runs*.
 *
 * `run_tests` collected the whole output with a blocking `Command::output()`
 * and emitted every `test:log` line after the process exited, so a `make
 * test-verbose` that takes minutes left the panel showing one echoed command
 * line and nothing else — indistinguishable from a hung run.
 *
 * Every scenario below emits `test:log` events *while the `run_tests` promise
 * is still pending, and asserts on what is on screen at that moment. Asserting
 * after the promise resolves is exactly the check that passed while the bug
 * shipped.
 */

import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

type Handler = (e: { payload: string }) => void;
const handlers: Record<string, Handler[]> = {};
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (event: string, handler: Handler) => {
    (handlers[event] ||= []).push(handler);
    return () => {
      handlers[event] = (handlers[event] || []).filter((h) => h !== handler);
    };
  },
}));

import { TestPanel } from "../TestPanel";

/** Emit a `test:log` line the way the backend does, mid-run. */
async function emitLog(line: string) {
  await act(async () => {
    (handlers["test:log"] || []).forEach((h) => h({ payload: line }));
  });
}

/** A `run_tests` call the test controls the completion of. */
function pendingRun() {
  let finish: (value: unknown) => void = () => {};
  const promise = new Promise((resolve) => { finish = resolve; });
  return { promise, finish };
}

const EMPTY_RESULT = {
  framework: "cargo test",
  passed: 0, failed: 0, ignored: 0, total: 0,
  duration_ms: 10,
  tests: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  for (const key of Object.keys(handlers)) delete handlers[key];
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === "detect_test_framework") return "cargo test";
    return null;
  });
});

async function startRun(run: ReturnType<typeof pendingRun>) {
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === "detect_test_framework") return "cargo test";
    if (cmd === "run_tests") return run.promise;
    return null;
  });
  render(<TestPanel workspacePath="/repo" />);
  // The listener is registered in an effect; without awaiting it the first
  // emitted line lands before anything is subscribed.
  await waitFor(() => expect(handlers["test:log"]?.length).toBe(1));
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: /run tests/i }));
  });
}

describe("Given a test run that is still going", () => {
  it("When lines arrive, Then they are on screen before the run finishes", async () => {
    const run = pendingRun();
    await startRun(run);

    await emitLog("$ make test-verbose");
    await emitLog("   Compiling vibe-ai v0.5.5");
    await emitLog("test tools::strip_thinking ... ok");

    const console_ = screen.getByTestId("test-console");
    expect(console_.textContent).toContain("Compiling vibe-ai");
    expect(console_.textContent).toContain("test tools::strip_thinking ... ok");

    await act(async () => { run.finish(EMPTY_RESULT); });
  });

  it("When it is running, Then the panel says so and counts what it has seen", async () => {
    const run = pendingRun();
    await startRun(run);
    await emitLog("$ make test-verbose");
    await emitLog("   Compiling vibe-ai v0.5.5");

    expect(screen.getByRole("status").textContent).toMatch(/Running/);
    expect(screen.getByRole("status").textContent).toMatch(/2 lines/);

    await act(async () => { run.finish(EMPTY_RESULT); });
  });

  it("When Suspend is pressed, Then the backend is asked to kill the run", async () => {
    const run = pendingRun();
    await startRun(run);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /suspend/i }));
    });

    expect(mockInvoke.mock.calls.some((c) => c[0] === "stop_tests")).toBe(true);

    await act(async () => { run.finish(EMPTY_RESULT); });
  });
});

describe("Given a run that has finished", () => {
  it("Then the console it produced is still readable", async () => {
    const run = pendingRun();
    await startRun(run);
    await emitLog("test tools::strip_thinking ... FAILED");
    await emitLog("assertion failed: left == right");

    await act(async () => { run.finish(EMPTY_RESULT); });

    // The old panel hid the log the moment `running` went false, taking the
    // failure detail with it.
    expect(screen.getByTestId("test-console").textContent).toContain("assertion failed");
  });
});
