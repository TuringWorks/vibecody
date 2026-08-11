import { describe, it, expect, vi } from "vitest";
import {
  fitCompletionToLine,
  windowContext,
  registerGhostText,
  PREFIX_LINES,
  SUFFIX_LINES,
  type GhostTextDeps,
} from "../ghostText";

// ── Monaco stand-ins ──────────────────────────────────────────────────────
// Only the surface `registerGhostText` actually touches. Mirrors the real
// enum's numbering: Automatic = 0, Explicit = 1. The VS Code enum is the
// other way round, which is exactly why the gate must compare against a
// named member — see ghostText.ts's header.
const TRIGGER = { Automatic: 0, Explicit: 1 } as const;

function fakeModel(lines: string[], languageId = "typescript") {
  const lineAt = (n: number) => lines[n - 1] ?? "";
  return {
    getLineCount: () => lines.length,
    getLanguageId: () => languageId,
    getLineMaxColumn: (n: number) => lineAt(n).length + 1,
    getValueInRange: (range: {
      startLineNumber: number;
      startColumn: number;
      endLineNumber: number;
      endColumn: number;
    }) => {
      const slice: string[] = [];
      for (let n = range.startLineNumber; n <= range.endLineNumber; n++) {
        const text = lineAt(n);
        const from = n === range.startLineNumber ? range.startColumn - 1 : 0;
        const to = n === range.endLineNumber ? range.endColumn - 1 : text.length;
        slice.push(text.slice(from, to));
      }
      return slice.join("\n");
    },
  };
}

function fakeMonaco() {
  const dispose = vi.fn();
  let registered: {
    provideInlineCompletions: (
      model: unknown,
      position: unknown,
      context: { triggerKind: number },
      token: { isCancellationRequested: boolean },
    ) => Promise<{ items: unknown[] }>;
  } | null = null;
  return {
    dispose,
    get provider() {
      if (!registered) throw new Error("provider was never registered");
      return registered;
    },
    monaco: {
      languages: {
        InlineCompletionTriggerKind: TRIGGER,
        registerInlineCompletionsProvider: (_selector: string, provider: never) => {
          registered = provider;
          return { dispose };
        },
      },
    } as never,
  };
}

function deps(overrides: Partial<GhostTextDeps> = {}): GhostTextDeps {
  return {
    invoke: vi.fn().mockResolvedValue({
      completion: "return a + b;",
      model_name: "test-model",
      truncated: false,
    }),
    getProvider: () => "anthropic",
    getModel: () => "claude-opus-5",
    getFilePath: () => "/w/src/lib.ts",
    onError: vi.fn(),
    ...overrides,
  };
}

const POSITION = { lineNumber: 2, column: 5 };
const LIVE_TOKEN = { isCancellationRequested: false };

describe("windowContext", () => {
  it("splits the buffer exactly at the cursor", () => {
    const model = fakeModel(["function f() {", "    ", "}"]);
    const { prefix, suffix } = windowContext(model, POSITION);
    expect(prefix).toBe("function f() {\n    ");
    expect(suffix).toBe("\n}");
  });

  it("reports only the text between the cursor and end of line as restOfLine", () => {
    const model = fakeModel(["const x = 1;", "    foo(bar)", "}"]);
    // Columns are 1-based: column 9 of "    foo(bar)" sits just after the `(`.
    const { restOfLine } = windowContext(model, { lineNumber: 2, column: 9 });
    expect(restOfLine).toBe("bar)");
  });

  it("bounds the window rather than sending the whole file", () => {
    const long = Array.from({ length: 1000 }, (_, i) => `line ${i}`);
    const model = fakeModel(long);
    const { prefix, suffix } = windowContext(model, { lineNumber: 500, column: 1 });
    expect(prefix.split("\n").length).toBe(PREFIX_LINES + 1);
    expect(suffix.split("\n").length).toBe(SUFFIX_LINES + 1);
  });

  it("clamps at the start and end of the buffer", () => {
    const model = fakeModel(["a", "b"]);
    const { prefix, suffix } = windowContext(model, { lineNumber: 1, column: 1 });
    expect(prefix).toBe("");
    expect(suffix).toBe("a\nb");
  });
});

describe("fitCompletionToLine", () => {
  it("keeps a multi-line completion when only whitespace follows the cursor", () => {
    const fitted = fitCompletionToLine("if (x) {\n  y();\n}", "   ");
    expect(fitted).toEqual({ text: "if (x) {\n  y();\n}", extendToEndOfLine: true });
  });

  it("does not extend the range when the cursor is already at end of line", () => {
    const fitted = fitCompletionToLine("a + b", "");
    expect(fitted).toEqual({ text: "a + b", extendToEndOfLine: false });
  });

  it("clips to one line when real code follows the cursor", () => {
    // Monaco requires a multi-line insertText to end its range at a line end,
    // and we cannot extend over `)` without eating it.
    const fitted = fitCompletionToLine("a + b\nmore()", ")");
    expect(fitted).toEqual({ text: "a + b", extendToEndOfLine: false });
  });

  it("returns null when nothing renderable remains", () => {
    expect(fitCompletionToLine("", "")).toBeNull();
    expect(fitCompletionToLine("\nfoo()", ")")).toBeNull();
  });
});

describe("registerGhostText — the explicit-trigger gate", () => {
  it("returns nothing and calls no backend for an automatic trigger", async () => {
    const fake = fakeMonaco();
    const d = deps();
    registerGhostText(fake.monaco, d);

    const result = await fake.provider.provideInlineCompletions(
      fakeModel(["a", "bcde", "f"]),
      POSITION,
      { triggerKind: TRIGGER.Automatic },
      LIVE_TOKEN,
    );

    expect(result.items).toEqual([]);
    expect(d.invoke).not.toHaveBeenCalled();
  });

  it("answers an explicit trigger with a suggestion at the cursor", async () => {
    const fake = fakeMonaco();
    const d = deps();
    registerGhostText(fake.monaco, d);

    const result = await fake.provider.provideInlineCompletions(
      fakeModel(["function f() {", "    ", "}"]),
      POSITION,
      { triggerKind: TRIGGER.Explicit },
      LIVE_TOKEN,
    );

    expect(d.invoke).toHaveBeenCalledWith("ghost_complete", {
      filePath: "/w/src/lib.ts",
      language: "typescript",
      prefix: "function f() {\n    ",
      suffix: "\n}",
      provider: "anthropic",
      model: "claude-opus-5",
    });
    expect(result.items).toHaveLength(1);
    expect(result.items[0]).toMatchObject({ insertText: "return a + b;" });
  });

  it("refuses to guess a provider when the toolbar has no selection", async () => {
    const fake = fakeMonaco();
    const d = deps({ getProvider: () => "", getModel: () => "" });
    registerGhostText(fake.monaco, d);

    const result = await fake.provider.provideInlineCompletions(
      fakeModel(["a", "bcde"]),
      POSITION,
      { triggerKind: TRIGGER.Explicit },
      LIVE_TOKEN,
    );

    expect(result.items).toEqual([]);
    expect(d.invoke).not.toHaveBeenCalled();
    expect(d.onError).toHaveBeenCalledWith(
      expect.stringContaining("Select a provider"),
    );
  });

  it("treats an empty completion as the model declining, not an error", async () => {
    const fake = fakeMonaco();
    const d = deps({
      invoke: vi.fn().mockResolvedValue({
        completion: "",
        model_name: "test-model",
        truncated: false,
      }),
    });
    registerGhostText(fake.monaco, d);

    const result = await fake.provider.provideInlineCompletions(
      fakeModel(["a", "bcde"]),
      POSITION,
      { triggerKind: TRIGGER.Explicit },
      LIVE_TOKEN,
    );

    expect(result.items).toEqual([]);
    expect(d.onError).not.toHaveBeenCalled();
  });

  it("surfaces a backend failure to the user", async () => {
    const fake = fakeMonaco();
    const d = deps({ invoke: vi.fn().mockRejectedValue(new Error("no API key")) });
    registerGhostText(fake.monaco, d);

    const result = await fake.provider.provideInlineCompletions(
      fakeModel(["a", "bcde"]),
      POSITION,
      { triggerKind: TRIGGER.Explicit },
      LIVE_TOKEN,
    );

    expect(result.items).toEqual([]);
    expect(d.onError).toHaveBeenCalledWith(expect.stringContaining("no API key"));
  });

  it("drops a result whose request was cancelled mid-flight", async () => {
    const fake = fakeMonaco();
    registerGhostText(fake.monaco, deps());

    const result = await fake.provider.provideInlineCompletions(
      fakeModel(["a", "bcde"]),
      POSITION,
      { triggerKind: TRIGGER.Explicit },
      { isCancellationRequested: true },
    );

    expect(result.items).toEqual([]);
  });

  it("reports truncation without claiming a specific line count", async () => {
    const fake = fakeMonaco();
    const onTruncated = vi.fn();
    const d = deps({
      onTruncated,
      invoke: vi.fn().mockResolvedValue({
        completion: "a();",
        model_name: "test-model",
        truncated: true,
      }),
    });
    registerGhostText(fake.monaco, d);

    await fake.provider.provideInlineCompletions(
      fakeModel(["a", "bcde"]),
      POSITION,
      { triggerKind: TRIGGER.Explicit },
      LIVE_TOKEN,
    );

    expect(onTruncated).toHaveBeenCalledOnce();
  });

  it("disposes its registration", () => {
    const fake = fakeMonaco();
    registerGhostText(fake.monaco, deps()).dispose();
    expect(fake.dispose).toHaveBeenCalledOnce();
  });
});
