import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const mockOpen = vi.fn();
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: (...args: unknown[]) => mockOpen(...args),
}));

import { DiffCompleteModal, applyUnifiedDiff, classifyDiffCompleteError, trapFocusInside } from '../DiffCompleteModal';

// ── Pure-function unit tests ────────────────────────────────────────────

describe('classifyDiffCompleteError', () => {
  it('attaches a Settings-pointer hint for "no active AI provider"', () => {
    const r = classifyDiffCompleteError("No active AI provider configured");
    expect(r.hint).toMatch(/Settings → API Keys/i);
  });

  it('attaches a format-hint for "did not contain a diff"', () => {
    const r = classifyDiffCompleteError("Model response did not contain a diff block");
    expect(r.hint).toMatch(/unified diff/i);
  });

  it('attaches a regenerate-hint for "could not be applied cleanly"', () => {
    const r = classifyDiffCompleteError("Model returned a diff that could not be applied cleanly.");
    expect(r.hint).toMatch(/Regenerate/i);
    // Avoid the literal "try again" copy — collides with the button label.
    expect(r.hint).not.toMatch(/try again/i);
  });

  it('attaches a connection-hint for network-class errors', () => {
    expect(classifyDiffCompleteError("network unreachable").hint).toMatch(/internet/i);
    expect(classifyDiffCompleteError("connection refused").hint).toMatch(/internet/i);
    expect(classifyDiffCompleteError("request timeout").hint).toMatch(/internet/i);
  });

  it('attaches a rate-limit hint for 429s and quota errors', () => {
    expect(classifyDiffCompleteError("429 Too Many Requests").hint).toMatch(/rate limit/i);
    expect(classifyDiffCompleteError("quota exceeded").hint).toMatch(/rate limit/i);
  });

  it('attaches an api-key hint for 401 / unauthorized', () => {
    expect(classifyDiffCompleteError("401 Unauthorized").hint).toMatch(/API key/i);
    expect(classifyDiffCompleteError("Invalid API key").hint).toMatch(/API key/i);
  });

  it('returns no hint for unclassified errors', () => {
    const r = classifyDiffCompleteError("Some unique never-seen failure");
    expect(r.message).toBe("Some unique never-seen failure");
    expect(r.hint).toBeUndefined();
  });

  it('always echoes the original message verbatim', () => {
    const raw = "Model response did not contain a diff block";
    expect(classifyDiffCompleteError(raw).message).toBe(raw);
  });
});

describe('trapFocusInside', () => {
  function setupContainer(html: string): HTMLDivElement {
    const c = document.createElement('div');
    c.innerHTML = html;
    document.body.appendChild(c);
    return c;
  }

  function makeKey(key: string, shift = false): KeyboardEvent {
    const e = new KeyboardEvent("keydown", { key, shiftKey: shift, bubbles: true, cancelable: true });
    return e;
  }

  it('does nothing for non-Tab keys', () => {
    const c = setupContainer('<button id="a">A</button><button id="b">B</button>');
    const a = c.querySelector<HTMLElement>('#a')!;
    a.focus();
    const e = makeKey("Enter");
    expect(trapFocusInside(c, e)).toBe(false);
    expect(e.defaultPrevented).toBe(false);
    c.remove();
  });

  it('cycles forward from last to first focusable', () => {
    const c = setupContainer('<button id="a">A</button><button id="b">B</button>');
    const a = c.querySelector<HTMLElement>('#a')!;
    const b = c.querySelector<HTMLElement>('#b')!;
    b.focus();
    const e = makeKey("Tab");
    expect(trapFocusInside(c, e)).toBe(true);
    expect(e.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(a);
    c.remove();
  });

  it('cycles backward from first to last on Shift+Tab', () => {
    const c = setupContainer('<button id="a">A</button><button id="b">B</button>');
    const a = c.querySelector<HTMLElement>('#a')!;
    const b = c.querySelector<HTMLElement>('#b')!;
    a.focus();
    const e = makeKey("Tab", true);
    expect(trapFocusInside(c, e)).toBe(true);
    expect(document.activeElement).toBe(b);
    c.remove();
  });

  it('does not trap when there are no focusables', () => {
    const c = setupContainer('<span>no buttons</span>');
    const e = makeKey("Tab");
    expect(trapFocusInside(c, e)).toBe(false);
    c.remove();
  });

  it('skips disabled buttons when computing focus targets', () => {
    const c = setupContainer('<button id="a">A</button><button id="b" disabled>B</button>');
    const a = c.querySelector<HTMLElement>('#a')!;
    a.focus();
    // Only `a` is focusable; Tab should cycle back to itself.
    const e = makeKey("Tab");
    expect(trapFocusInside(c, e)).toBe(true);
    expect(document.activeElement).toBe(a);
    c.remove();
  });
});

describe('applyUnifiedDiff', () => {
  it('applies a simple one-line change', () => {
    const original = "line 1\nline 2\nline 3\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,3 +1,3 @@",
      " line 1",
      "-line 2",
      "+LINE TWO",
      " line 3",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBe("line 1\nLINE TWO\nline 3\n");
  });

  it('applies an insertion', () => {
    const original = "a\nc\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,2 +1,3 @@",
      " a",
      "+b",
      " c",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBe("a\nb\nc\n");
  });

  it('applies a deletion', () => {
    const original = "a\nb\nc\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,3 +1,2 @@",
      " a",
      "-b",
      " c",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBe("a\nc\n");
  });

  it('applies two hunks in order', () => {
    const original = "a\nb\nc\nd\ne\nf\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,2 +1,2 @@",
      "-a",
      "+A",
      " b",
      "@@ -5,2 +5,2 @@",
      " e",
      "-f",
      "+F",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBe("A\nb\nc\nd\ne\nF\n");
  });

  it('returns null when context does not match', () => {
    const original = "a\nb\nc\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,3 +1,3 @@",
      " a",
      "-WRONG",
      "+new",
      " c",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBeNull();
  });

  it('returns null when there are no hunks', () => {
    expect(applyUnifiedDiff("a\n", "just prose, no diff")).toBeNull();
  });

  it('ignores "No newline at end of file" marker', () => {
    const original = "a\nb";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,2 +1,2 @@",
      " a",
      "-b",
      "\\ No newline at end of file",
      "+B",
      "\\ No newline at end of file",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBe("a\nB");
  });

  it('preserves trailing content after last hunk', () => {
    const original = "a\nb\nc\nd\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,2 +1,2 @@",
      "-a",
      "+A",
      " b",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBe("A\nb\nc\nd\n");
  });

  it('absorbs line-number drift (model writes wrong @@ line)', () => {
    // The hunk claims to start at line 1, but the matching context is at
    // line 3. A strict patch tool would reject this; we widen the search.
    const original = "lead 1\nlead 2\nlead 3\nbody A\nbody B\nbody C\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,3 +1,3 @@",
      " body A",
      "-body B",
      "+BODY B",
      " body C",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBe("lead 1\nlead 2\nlead 3\nbody A\nBODY B\nbody C\n");
  });

  it('absorbs trailing-whitespace drift in context lines', () => {
    // Model added a trailing space to the context line; we should still
    // apply, AND the output should preserve the original (no trailing space).
    const original = "alpha\nbeta\ngamma\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,3 +1,3 @@",
      " alpha  ",          // <-- trailing whitespace on context
      "-beta",
      "+BETA",
      " gamma",
    ].join("\n");
    expect(applyUnifiedDiff(original, diff)).toBe("alpha\nBETA\ngamma\n");
  });

  it('still preserves Python-significant internal indentation', () => {
    // 4 spaces vs tab in the context line should NOT match — we deliberately
    // don't collapse internal whitespace because Python / YAML / Markdown
    // tables would silently corrupt.
    const original = "def f():\n    return 1\n";
    const diff = [
      "--- a/f",
      "+++ b/f",
      "@@ -1,2 +1,2 @@",
      "\tdef f():",          // <-- tab where file has spaces
      "-    return 1",
      "+    return 2",
    ].join("\n");
    // First line doesn't fuzz-match (we only fuzz trailing whitespace),
    // so the whole diff is rejected. Better than silently misapplying.
    expect(applyUnifiedDiff(original, diff)).toBeNull();
  });
});

describe('DiffCompleteModal — flow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const baseProps = {
    open: true,
    onClose: vi.fn(),
    filePath: "src/lib.rs",
    language: "rust",
    originalContent: "line 1\nline 2\nline 3\n",
    selectionText: "",
    selectionStartLine: 0,
    selectionEndLine: 0,
    provider: "mock",
    onApply: vi.fn(),
  };

  it('renders instruction prompt when opened', () => {
    render(<DiffCompleteModal {...baseProps} />);
    expect(screen.getByPlaceholderText(/Describe the change/i)).toBeInTheDocument();
    expect(screen.getByText(/Generate diff/i)).toBeInTheDocument();
  });

  it('disables submit when instruction is empty', () => {
    render(<DiffCompleteModal {...baseProps} />);
    const submit = screen.getByText(/Generate diff/i).closest('button')!;
    expect(submit).toBeDisabled();
  });

  it('calls diffcomplete_generate and then enters review phase on success', async () => {
    mockInvoke.mockResolvedValueOnce({
      unified_diff: [
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,3 +1,3 @@",
        " line 1",
        "-line 2",
        "+LINE TWO",
        " line 3",
      ].join("\n"),
      explanation: "Renamed line 2",
      model_name: "mock",
    });

    render(<DiffCompleteModal {...baseProps} />);
    const input = screen.getByPlaceholderText(/Describe the change/i);
    fireEvent.change(input, { target: { value: "rename line 2" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("diffcomplete_generate", expect.objectContaining({
        filePath: "src/lib.rs",
        language: "rust",
        instruction: "rename line 2",
        provider: "mock",
      }));
    });

    await waitFor(() => {
      expect(screen.getByText(/Renamed line 2/)).toBeInTheDocument();
    });
  });

  it('shows error phase when the backend returns an unclean diff', async () => {
    mockInvoke.mockResolvedValueOnce({
      unified_diff: [
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,3 +1,3 @@",
        " line 1",
        "-WRONG CONTEXT",
        "+new",
        " line 3",
      ].join("\n"),
      explanation: null,
      model_name: "mock",
    });

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      expect(screen.getByText(/could not be applied cleanly/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/Try again/i)).toBeInTheDocument();
  });

  it('shows error phase when the backend throws', async () => {
    mockInvoke.mockRejectedValueOnce("No provider configured");

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      expect(screen.getByText(/No provider configured/)).toBeInTheDocument();
    });
  });

  it('returns null from onApply path when user dismisses via Close', () => {
    const onClose = vi.fn();
    render(<DiffCompleteModal {...baseProps} onClose={onClose} />);
    fireEvent.click(screen.getByLabelText('Close'));
    expect(onClose).toHaveBeenCalled();
  });

  it('attaches picked files as additionalFiles in the invoke payload', async () => {
    mockOpen.mockResolvedValueOnce(["/abs/src/helper.rs", "/abs/src/types.rs"]);
    mockInvoke.mockImplementation((cmd: string, args: { path?: string }) => {
      if (cmd === "read_file_sandbox") {
        if (args.path === "/abs/src/helper.rs") return Promise.resolve("pub fn helper() {}\n");
        if (args.path === "/abs/src/types.rs") return Promise.resolve("pub struct Foo;\n");
      }
      if (cmd === "diffcomplete_generate") {
        return Promise.resolve({
          unified_diff: [
            "--- a/src/lib.rs",
            "+++ b/src/lib.rs",
            "@@ -1,3 +1,3 @@",
            " line 1",
            "-line 2",
            "+LINE TWO",
            " line 3",
          ].join("\n"),
          explanation: null,
          model_name: "mock",
        });
      }
      return Promise.reject(new Error(`unexpected cmd: ${cmd}`));
    });

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.click(screen.getByLabelText('Add files as context'));
    await waitFor(() => {
      expect(screen.getByText(/2 files attached/)).toBeInTheDocument();
    });

    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      const call = mockInvoke.mock.calls.find(c => c[0] === "diffcomplete_generate");
      expect(call).toBeTruthy();
      expect(call![1]).toMatchObject({
        additionalFiles: [
          { path: "/abs/src/helper.rs", content: "pub fn helper() {}\n" },
          { path: "/abs/src/types.rs", content: "pub struct Foo;\n" },
        ],
      });
    });
  });

  it('omits additionalFiles when none attached (sends null)', async () => {
    mockInvoke.mockResolvedValueOnce({
      unified_diff: [
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,3 +1,3 @@",
        " line 1",
        "-line 2",
        "+LINE TWO",
        " line 3",
      ].join("\n"),
      explanation: null,
      model_name: "mock",
    });

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("diffcomplete_generate", expect.objectContaining({
        additionalFiles: null,
      }));
    });
  });

  it('sends previousDiff and refinement as null on the first generate call', async () => {
    mockInvoke.mockResolvedValueOnce({
      unified_diff: [
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,3 +1,3 @@",
        " line 1",
        "-line 2",
        "+LINE TWO",
        " line 3",
      ].join("\n"),
      explanation: null,
      model_name: "mock",
    });

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("diffcomplete_generate", expect.objectContaining({
        previousDiff: null,
        refinement: null,
      }));
    });
  });

  it('refinement input is not shown until a diff arrives', () => {
    render(<DiffCompleteModal {...baseProps} />);
    expect(screen.queryByLabelText(/Regenerate with refinement/i)).not.toBeInTheDocument();
    expect(screen.queryByPlaceholderText(/tighten the error path/i)).not.toBeInTheDocument();
  });

  it('Regenerate sends previousDiff + refinement + the original instruction', async () => {
    const firstDiff = [
      "--- a/src/lib.rs",
      "+++ b/src/lib.rs",
      "@@ -1,3 +1,3 @@",
      " line 1",
      "-line 2",
      "+LINE TWO",
      " line 3",
    ].join("\n");
    const secondDiff = [
      "--- a/src/lib.rs",
      "+++ b/src/lib.rs",
      "@@ -1,3 +1,3 @@",
      " line 1",
      "-line 2",
      "+SECOND PASS",
      " line 3",
    ].join("\n");

    mockInvoke
      .mockResolvedValueOnce({ unified_diff: firstDiff, explanation: null, model_name: "mock" })
      .mockResolvedValueOnce({ unified_diff: secondDiff, explanation: null, model_name: "mock" });

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), {
      target: { value: "rename line 2" },
    });
    fireEvent.click(screen.getByText(/Generate diff/i));

    const refineBox = await screen.findByPlaceholderText(/tighten the error path/i);
    fireEvent.change(refineBox, { target: { value: "  use SECOND PASS instead  " } });
    fireEvent.click(screen.getByLabelText(/Regenerate with refinement/i));

    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter(c => c[0] === "diffcomplete_generate");
      expect(calls.length).toBe(2);
      expect(calls[1][1]).toMatchObject({
        instruction: "rename line 2",
        previousDiff: firstDiff,
        refinement: "use SECOND PASS instead",
      });
    });
  });

  it('clears the refinement field after Regenerate fires', async () => {
    const firstDiff = [
      "--- a/src/lib.rs",
      "+++ b/src/lib.rs",
      "@@ -1,3 +1,3 @@",
      " line 1",
      "-line 2",
      "+LINE TWO",
      " line 3",
    ].join("\n");

    mockInvoke
      .mockResolvedValueOnce({ unified_diff: firstDiff, explanation: null, model_name: "mock" })
      .mockResolvedValueOnce({ unified_diff: firstDiff, explanation: null, model_name: "mock" });

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), {
      target: { value: "x" },
    });
    fireEvent.click(screen.getByText(/Generate diff/i));

    const refineBox = await screen.findByPlaceholderText(/tighten the error path/i) as HTMLTextAreaElement;
    fireEvent.change(refineBox, { target: { value: "more concise" } });
    expect(refineBox.value).toBe("more concise");

    fireEvent.click(screen.getByLabelText(/Regenerate with refinement/i));

    await waitFor(() => {
      const refreshed = screen.getByPlaceholderText(/tighten the error path/i) as HTMLTextAreaElement;
      expect(refreshed.value).toBe("");
    });
  });

  it('Regenerate button is disabled when refinement is empty', async () => {
    mockInvoke.mockResolvedValueOnce({
      unified_diff: [
        "--- a/src/lib.rs",
        "+++ b/src/lib.rs",
        "@@ -1,3 +1,3 @@",
        " line 1",
        "-line 2",
        "+LINE TWO",
        " line 3",
      ].join("\n"),
      explanation: null,
      model_name: "mock",
    });

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    const regen = await screen.findByLabelText(/Regenerate with refinement/i);
    expect(regen).toBeDisabled();
  });

  // ── Empty state when no provider is configured ─────────────────────────

  it('shows the no-provider empty state when /health reports no providers', async () => {
    const fetchMock = vi.fn(async () => new Response(
      JSON.stringify({ features: { diffcomplete: { available: false } } }),
      { status: 200, headers: { "content-type": "application/json" } },
    ));
    const origFetch = globalThis.fetch;
    globalThis.fetch = fetchMock as unknown as typeof globalThis.fetch;
    try {
      // Empty provider prop forces the modal to probe /health.
      render(<DiffCompleteModal {...baseProps} provider="" />);
      await waitFor(() => {
        expect(screen.getByText(/No AI provider configured/i)).toBeInTheDocument();
      });
      // The Generate button should not be on screen — empty state replaces
      // the prompt UI, not augments it.
      expect(screen.queryByText(/Generate diff/i)).toBeNull();
      // Exact, user-facing pointer to the fix.
      expect(screen.getByText(/Settings → API Keys/i)).toBeInTheDocument();
    } finally {
      globalThis.fetch = origFetch;
    }
  });

  it('renders prompt UI when provider prop is set, even before /health fires', () => {
    // baseProps has provider="mock" — the modal must trust the parent
    // and not gate the prompt UI on a /health roundtrip.
    render(<DiffCompleteModal {...baseProps} />);
    expect(screen.getByPlaceholderText(/Describe the change/i)).toBeInTheDocument();
    expect(screen.queryByText(/No AI provider configured/i)).toBeNull();
  });

  // ── Error classification + hints ────────────────────────────────────────

  it('renders the error hint card alongside the error message', async () => {
    mockInvoke.mockRejectedValueOnce("No active AI provider configured");
    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));
    await waitFor(() => {
      expect(screen.getByText(/No active AI provider configured/i)).toBeInTheDocument();
    });
    // The hint matches what classifyDiffCompleteError emits for this fragment.
    expect(screen.getByTestId("diffcomplete-error-hint")).toBeInTheDocument();
    expect(screen.getByTestId("diffcomplete-error-hint")).toHaveTextContent(/Settings → API Keys/i);
  });

  it('shows no hint card for an unclassified error', async () => {
    mockInvoke.mockRejectedValueOnce("Some weird never-seen-before fault");
    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));
    await waitFor(() => {
      expect(screen.getByText(/Some weird never-seen-before fault/i)).toBeInTheDocument();
    });
    expect(screen.queryByTestId("diffcomplete-error-hint")).toBeNull();
  });

  it('removes a chip via its × button and drops it from the payload', async () => {
    mockOpen.mockResolvedValueOnce(["/abs/src/helper.rs"]);
    mockInvoke.mockImplementation((cmd: string, args: { path?: string }) => {
      if (cmd === "read_file_sandbox" && args.path === "/abs/src/helper.rs") {
        return Promise.resolve("pub fn helper() {}\n");
      }
      if (cmd === "diffcomplete_generate") {
        return Promise.resolve({
          unified_diff: [
            "--- a/src/lib.rs",
            "+++ b/src/lib.rs",
            "@@ -1,3 +1,3 @@",
            " line 1",
            "-line 2",
            "+LINE TWO",
            " line 3",
          ].join("\n"),
          explanation: null,
          model_name: "mock",
        });
      }
      return Promise.reject(new Error(`unexpected cmd: ${cmd}`));
    });

    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.click(screen.getByLabelText('Add files as context'));
    await waitFor(() => {
      expect(screen.getByText(/1 file attached/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByLabelText('Remove /abs/src/helper.rs'));
    await waitFor(() => {
      expect(screen.queryByText(/1 file attached/)).not.toBeInTheDocument();
    });

    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "x" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      const call = mockInvoke.mock.calls.find(c => c[0] === "diffcomplete_generate");
      expect(call).toBeTruthy();
      expect(call![1]).toMatchObject({ additionalFiles: null });
    });
  });
});

// ── D1.2: chain autosave on regenerate / Apply / Cancel ────────────────────

describe('DiffCompleteModal — D1.2 autosave hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const baseProps = {
    open: true,
    onClose: vi.fn(),
    filePath: "src/lib.rs",
    language: "rust",
    originalContent: "line 1\nline 2\nline 3\n",
    selectionText: "line 2",
    selectionStartLine: 2,
    selectionEndLine: 2,
    provider: "mock",
    onApply: vi.fn(),
  };

  function mockGenerateOnce(diffBody: string, model = "mock") {
    return mockInvoke.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd === "diffcomplete_generate") {
        return {
          unified_diff: diffBody,
          explanation: "ok",
          model_name: model,
        };
      }
      if (cmd === "diffcomplete_chain_autosave") {
        // Return a deterministic chain id so subsequent posts are
        // recognized as appends rather than fresh chains.
        const req = (args as { request: { chain_id: string | null } }).request;
        return {
          chain_id: req.chain_id ?? "chain-mock-1",
          step_index: 0,
        };
      }
      return null;
    });
  }

  const cleanDiff = [
    "--- a/src/lib.rs",
    "+++ b/src/lib.rs",
    "@@ -1,3 +1,3 @@",
    " line 1",
    "-line 2",
    "+LINE TWO",
    " line 3",
  ].join("\n");

  it('posts diffcomplete_chain_autosave with step on first successful generate', async () => {
    mockGenerateOnce(cleanDiff);
    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "rename" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      const call = mockInvoke.mock.calls.find(c => c[0] === "diffcomplete_chain_autosave");
      expect(call).toBeTruthy();
      const req = call![1] as { request: { chain_id: string | null; step: { index: number; instruction: string; refinement: string | null } } };
      expect(req.request.chain_id).toBeNull();
      expect(req.request.step.index).toBe(0);
      expect(req.request.step.instruction).toBe("rename");
      expect(req.request.step.refinement).toBeNull();
    });
  });

  it('writes final_state=applied when the user clicks Apply', async () => {
    mockGenerateOnce(cleanDiff);
    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "rename" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    // Wait for autosave to land so chainId is populated before Apply.
    await waitFor(() => {
      expect(mockInvoke.mock.calls.some(c => c[0] === "diffcomplete_chain_autosave")).toBe(true);
    });

    // DiffReviewPanel labels its primary action "Apply (N)".
    const applyBtn = await screen.findByRole('button', { name: /^Apply\s*\(\d+\)/i });
    fireEvent.click(applyBtn);

    await waitFor(() => {
      const finalCall = mockInvoke.mock.calls
        .filter(c => c[0] === "diffcomplete_chain_autosave")
        .map(c => c[1] as { request: { final_state: { type?: string } | null } })
        .find(c => c.request.final_state?.type === "applied");
      expect(finalCall).toBeTruthy();
    });
  });

  it('writes final_state=cancelled when Cancel is clicked in prompt phase', async () => {
    mockGenerateOnce(cleanDiff);
    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "rename" } });
    fireEvent.click(screen.getByText(/Generate diff/i));
    await waitFor(() => {
      expect(mockInvoke.mock.calls.some(c => c[0] === "diffcomplete_chain_autosave")).toBe(true);
    });
    // Reset to prompt phase via the Try-again-style path is complex;
    // instead, simulate a Close/Cancel from review by pressing Esc.
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });

    await waitFor(() => {
      const finalCall = mockInvoke.mock.calls
        .filter(c => c[0] === "diffcomplete_chain_autosave")
        .map(c => c[1] as { request: { final_state: { type?: string; reason?: string } | null } })
        .find(c => c.request.final_state?.type === "cancelled");
      expect(finalCall).toBeTruthy();
      expect(finalCall!.request.final_state!.reason).toMatch(/modal_closed|user_cancel/);
    });
  });

  it('does NOT write final_state when modal is closed before any generate', async () => {
    mockGenerateOnce(cleanDiff);
    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.click(screen.getByText(/^Cancel$/));

    // Give the (non-)autosave call a chance to land.
    await new Promise(r => setTimeout(r, 50));
    const autosaveCalls = mockInvoke.mock.calls.filter(c => c[0] === "diffcomplete_chain_autosave");
    expect(autosaveCalls).toHaveLength(0);
  });

  it('reuses chain_id across regenerations (idempotent chain)', async () => {
    mockGenerateOnce(cleanDiff);
    render(<DiffCompleteModal {...baseProps} />);
    fireEvent.change(screen.getByPlaceholderText(/Describe the change/i), { target: { value: "first" } });
    fireEvent.click(screen.getByText(/Generate diff/i));

    await waitFor(() => {
      expect(mockInvoke.mock.calls.some(c => c[0] === "diffcomplete_chain_autosave")).toBe(true);
    });

    // Second autosave (refinement) — drive it through the refinement path.
    const refineInput = screen.getByPlaceholderText(/tighten the error path/i);
    fireEvent.change(refineInput, { target: { value: "tighten error path" } });
    const regenBtn = screen.getByLabelText('Regenerate with refinement') as HTMLButtonElement;
    fireEvent.click(regenBtn);

    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter(c => c[0] === "diffcomplete_chain_autosave");
      expect(calls.length).toBeGreaterThanOrEqual(2);
      const second = calls[1][1] as { request: { chain_id: string | null; step: { index: number; refinement: string | null } } };
      expect(second.request.chain_id).toBe("chain-mock-1");
      expect(second.request.step.index).toBe(1);
      expect(second.request.step.refinement).toBe("tighten error path");
    });
  });
});
