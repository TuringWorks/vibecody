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

import { DiffCompleteModal, applyUnifiedDiff } from '../DiffCompleteModal';

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
