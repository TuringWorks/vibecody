/**
 * BDD: the build-system dropdown has to decide what runs.
 *
 * A Rust workspace with a Makefile detects as two build systems. Picking
 * "Make (Makefile)" and pressing Build ran **cargo** anyway: the panel sent
 * only `{ workspace, command }`, with `command` set from the *custom* field, so
 * it was `undefined` for an ordinary build. `run_build` then re-detected and
 * took `.first()` — cargo, because that is how detection orders a Rust project.
 *
 * The dropdown was decorative. Every scenario below asserts on the payload the
 * backend receives, because that is what decides which compiler runs; asserting
 * that the option is selected in the DOM is exactly the check that passed while
 * the bug shipped.
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

// jsdom implements no layout, so `scrollIntoView` is absent. The panel calls
// it whenever the log grows; without this every scenario dies in an effect
// before reaching its assertion.
beforeEach(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

import { BuildPanel } from "../BuildPanel";

const CARGO = {
  name: "Cargo",
  build_command: "cargo build",
  run_command: "cargo run",
  config_file: "Cargo.toml",
  tool_available: true,
  install_hint: "",
  project_path: "",
};

const MAKE = {
  name: "Make",
  build_command: "make",
  run_command: "make run",
  config_file: "Makefile",
  tool_available: true,
  install_hint: "",
  project_path: "",
};

const OK = { success: true, output: "", errors: [], warnings: [], duration_ms: 1, exit_code: 0 };

function payloadFor(command: string) {
  const call = mockInvoke.mock.calls.find(c => c[0] === command);
  return call?.[1] as { workspace: string; command?: string } | undefined;
}

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case "detect_build_system":
        // Detection order matters: cargo first is what made the bug invisible.
        return [CARGO, MAKE];
      case "list_workspace_subdirs":
        return [];
      case "run_build":
      case "run_app":
        return OK;
      default:
        return null;
    }
  });
});

const props = { workspacePath: "/repo", currentFile: null, onOpenFile: () => {} };

/** Pick a build system by its position in the detected list. */
async function selectSystem(index: number) {
  const select = await screen.findByRole("combobox", { name: /build system/i });
  fireEvent.change(select, { target: { value: String(index) } });
}

describe("Given a project that detects as both Cargo and Make", () => {
  it("When Make is selected and Build pressed, Then make runs — not cargo", async () => {
    render(<BuildPanel {...props} />);
    await selectSystem(1);

    fireEvent.click(screen.getByRole("button", { name: /^build$/i }));

    await waitFor(() => {
      const payload = payloadFor("run_build");
      expect(payload, "run_build should have been invoked").toBeTruthy();
      expect(payload!.command).toBe("make");
    });
  });

  it("When Make is selected and Run pressed, Then make's run command is used", async () => {
    render(<BuildPanel {...props} />);
    await selectSystem(1);

    fireEvent.click(screen.getByRole("button", { name: /^run$/i }));

    await waitFor(() => {
      expect(payloadFor("run_app")?.command).toBe("make run");
    });
  });

  it("When Make is selected and Build & Run pressed, Then both halves use make", async () => {
    render(<BuildPanel {...props} />);
    await selectSystem(1);

    fireEvent.click(screen.getByRole("button", { name: /build & run/i }));

    await waitFor(() => {
      expect(payloadFor("run_build")?.command).toBe("make");
      expect(payloadFor("run_app")?.command).toBe("make run");
    });
  });

  it("When the first system is selected, Then cargo runs", async () => {
    render(<BuildPanel {...props} />);
    // Detection is async, and the selector appears only once it lands. No
    // change event: index 0 is the default, and it must still be sent
    // explicitly rather than left to the backend to guess.
    await screen.findByRole("combobox", { name: /build system/i });
    fireEvent.click(screen.getByRole("button", { name: /^build$/i }));

    await waitFor(() => {
      expect(payloadFor("run_build")?.command).toBe("cargo build");
    });
  });
});

describe("Given a custom command is typed", () => {
  it("Then it wins over the selected system", async () => {
    render(<BuildPanel {...props} />);
    await selectSystem(1);

    // The Custom tab holds the override; it must still take precedence.
    fireEvent.click(screen.getByRole("button", { name: /custom/i }));
    const field = await screen.findByRole("textbox", { name: /custom build command/i });
    fireEvent.change(field, { target: { value: "make release" } });

    fireEvent.click(screen.getByRole("button", { name: /^build$/i }));

    await waitFor(() => {
      expect(payloadFor("run_build")?.command).toBe("make release");
    });
  });
});

describe("Given nothing was detected", () => {
  it("Then Build sends no command and lets the backend say so", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "detect_build_system") return [];
      if (cmd === "list_workspace_subdirs") return [];
      return OK;
    });
    render(<BuildPanel {...props} />);

    fireEvent.click(screen.getByRole("button", { name: /^build$/i }));

    await waitFor(() => {
      const payload = payloadFor("run_build");
      expect(payload).toBeTruthy();
      // Inventing a command here would guess at a project nobody identified.
      expect(payload!.command).toBeUndefined();
    });
  });
});
