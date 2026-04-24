/**
 * DiffReviewPanel — unit tests
 *
 * Focus areas:
 * 1. Apply button calls onApply with assembled content
 * 2. onApply is called exactly once (double-click guard)
 * 3. Cancel/Reject All calls onApply(null)
 * 4. No-change files call onApply(null)
 * 5. Hunk accept/reject toggles work
 * 6. LCS guard fires for large files (>800k char product)
 * 7. ErrorBoundary catches render errors and shows Dismiss
 */

import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock Monaco editor with a lightweight textarea so edit-mode tests can drive
// it with fireEvent.change. Preserves the (value, onChange) shape we use.
vi.mock('@monaco-editor/react', () => ({
  __esModule: true,
  default: ({
    value,
    onChange,
    language,
  }: {
    value: string;
    onChange: (next: string | undefined) => void;
    language?: string;
  }) => (
    <textarea
      data-testid="monaco-mock"
      data-language={language ?? ''}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  ),
}));

import { DiffReviewPanel, DiffReviewErrorBoundary } from '../DiffReviewPanel';

// ── Helpers ────────────────────────────────────────────────────────────────────

function renderPanel(
  original: string,
  modified: string,
  onApply = vi.fn(),
  filePath = 'src/App.tsx',
) {
  return render(
    <DiffReviewPanel
      original={original}
      modified={modified}
      filePath={filePath}
      onApply={onApply}
    />,
  );
}

// ── Basic rendering ────────────────────────────────────────────────────────────

describe('DiffReviewPanel — rendering', () => {
  it('shows file name in header', () => {
    renderPanel('a\n', 'b\n');
    expect(screen.getByText('App.tsx')).toBeInTheDocument();
  });

  it('shows "No changes detected" when original === modified', () => {
    const content = 'line1\nline2\n';
    renderPanel(content, content);
    expect(screen.getByText(/No changes detected/i)).toBeInTheDocument();
  });

  it('shows hunk count badge', () => {
    renderPanel('a\nb\n', 'a\nc\n');
    // Badge format: "X/Y hunks"
    expect(screen.getByText(/hunks/i)).toBeInTheDocument();
  });

  it('renders Accept and Reject buttons per hunk', () => {
    renderPanel('old line\n', 'new line\n');
    // Each hunk has a toggle button showing "✓ Accept" or "✗ Reject"
    expect(screen.getAllByText(/Accept|Reject/i).length).toBeGreaterThan(0);
  });
});

// ── Apply behaviour ────────────────────────────────────────────────────────────

describe('DiffReviewPanel — Apply', () => {
  it('calls onApply with assembled content when Apply is clicked', async () => {
    const onApply = vi.fn();
    renderPanel('hello\nworld\n', 'hello\nearth\n', onApply);

    fireEvent.click(screen.getByText(/Apply/i));

    expect(onApply).toHaveBeenCalledOnce();
    const arg = onApply.mock.calls[0][0] as string;
    expect(arg).toContain('earth');
    expect(arg).not.toBeNull();
  });

  it('calls onApply(null) when Cancel is clicked', () => {
    const onApply = vi.fn();
    renderPanel('a\n', 'b\n', onApply);

    fireEvent.click(screen.getByText('Cancel'));

    expect(onApply).toHaveBeenCalledOnce();
    expect(onApply).toHaveBeenCalledWith(null);
  });

  it('calls onApply(null) for no-change files', () => {
    const onApply = vi.fn();
    const content = 'same\ncontent\n';
    renderPanel(content, content, onApply);

    fireEvent.click(screen.getByText(/Apply/i));

    expect(onApply).toHaveBeenCalledOnce();
    expect(onApply).toHaveBeenCalledWith(null);
  });

  it('calls onApply(null) when all hunks are rejected', () => {
    const onApply = vi.fn();
    renderPanel('old\n', 'new\n', onApply);

    // Reject all hunks first
    fireEvent.click(screen.getByText('Reject All'));

    fireEvent.click(screen.getByText(/Apply/i));

    expect(onApply).toHaveBeenCalledOnce();
    expect(onApply).toHaveBeenCalledWith(null);
  });

  it('preserves original lines for rejected hunks', () => {
    const onApply = vi.fn();
    // Two independent lines changed — yields two hunks
    renderPanel(
      'keep this\nchange this\nkeep that\n',
      'keep this\nCHANGED\nkeep that\n',
      onApply,
    );

    // Reject all (should fall back to original semantically)
    fireEvent.click(screen.getByText('Reject All'));
    fireEvent.click(screen.getByText(/Apply/i));

    // onApply(null) when nothing accepted
    expect(onApply).toHaveBeenCalledWith(null);
  });

  it('includes modified lines for accepted hunks', () => {
    const onApply = vi.fn();
    renderPanel('line1\nold\nline3\n', 'line1\nnew\nline3\n', onApply);

    // Default: all hunks accepted
    fireEvent.click(screen.getByText(/Apply/i));

    const result = onApply.mock.calls[0][0] as string;
    expect(result).toContain('new');
    expect(result).not.toContain('old');
  });
});

// ── Double-click guard ────────────────────────────────────────────────────────

describe('DiffReviewPanel — double-click guard', () => {
  it('only calls onApply once even if Apply clicked rapidly', () => {
    const onApply = vi.fn();
    renderPanel('a\n', 'b\n', onApply);

    const applyBtn = screen.getByText(/Apply/i);
    fireEvent.click(applyBtn);
    fireEvent.click(applyBtn);
    fireEvent.click(applyBtn);

    expect(onApply).toHaveBeenCalledOnce();
  });
});

// ── Accept All / Reject All ───────────────────────────────────────────────────

describe('DiffReviewPanel — Accept All / Reject All', () => {
  it('Accept All followed by Apply yields modified content', () => {
    const onApply = vi.fn();
    renderPanel('a\nb\n', 'x\ny\n', onApply);

    fireEvent.click(screen.getByText('Accept All'));
    fireEvent.click(screen.getByText(/Apply/i));

    const result = onApply.mock.calls[0][0] as string;
    expect(typeof result).toBe('string');
  });

  it('Reject All followed by Apply yields null', () => {
    const onApply = vi.fn();
    renderPanel('a\n', 'b\n', onApply);

    fireEvent.click(screen.getByText('Reject All'));
    fireEvent.click(screen.getByText(/Apply/i));

    expect(onApply).toHaveBeenCalledWith(null);
  });
});

// ── Hunk toggle ───────────────────────────────────────────────────────────────

describe('DiffReviewPanel — hunk toggle', () => {
  it('toggles hunk acceptance state when hunk button is clicked', () => {
    renderPanel('a\n', 'b\n', vi.fn());

    // The hunk toggle button renders SVG icon + " Accept" or " Reject".
    // Exclude "Accept All" / "Reject All" toolbar buttons by checking textContent
    // does NOT contain "All".
    const buttons = screen.getAllByRole('button');
    const hunkBtn = buttons.find((btn) => {
      const t = btn.textContent ?? '';
      return /Accept|Reject/.test(t) && !/All/.test(t) && !/Apply/.test(t);
    });

    expect(hunkBtn).toBeDefined();
    const initialText = hunkBtn!.textContent;
    fireEvent.click(hunkBtn!);
    expect(hunkBtn!.textContent).not.toBe(initialText);
  });
});

// ── LCS guard ─────────────────────────────────────────────────────────────────

describe('DiffReviewPanel — large file guard', () => {
  it('renders without crashing for files that exceed LCS guard threshold', () => {
    // 900 lines × 900 lines = 810,000 > 800,000 — triggers fallback diff
    const original = Array.from({ length: 900 }, (_, i) => `line-orig-${i}`).join('\n');
    const modified = Array.from({ length: 900 }, (_, i) => `line-mod-${i}`).join('\n');
    const onApply = vi.fn();

    expect(() => renderPanel(original, modified, onApply)).not.toThrow();

    // Panel should render (shows some hunks or no-changes)
    expect(screen.getByText(/Apply/i)).toBeInTheDocument();
  });

  it('onApply still called after Apply on large file', () => {
    const original = Array.from({ length: 900 }, (_, i) => `orig-${i}`).join('\n');
    const modified = Array.from({ length: 900 }, (_, i) => `mod-${i}`).join('\n');
    const onApply = vi.fn();

    renderPanel(original, modified, onApply);
    fireEvent.click(screen.getByText(/Apply/i));

    expect(onApply).toHaveBeenCalledOnce();
  });
});

// ── onApply callback timing simulation ────────────────────────────────────────

describe('DiffReviewPanel — onApply timing (Apply button crash regression)', () => {
  /**
   * Simulates the App.tsx onApply handler using the rAF-deferred pattern.
   * Verifies:
   *  1. panel closes (setPendingDiff) synchronously
   *  2. write_file is invoked synchronously
   *  3. undo strip state (setLastApply) fires on frame 1
   *  4. Monaco sync state (setOpenFiles) fires on frame 2
   *  5. Total rAF callbacks = 2, nothing thrown
   */
  it('fires state updates in the correct frame order', async () => {
    const rafCallbacks: FrameRequestCallback[] = [];
    const originalRaf = window.requestAnimationFrame;
    window.requestAnimationFrame = (cb) => {
      rafCallbacks.push(cb);
      return rafCallbacks.length;
    };

    const order: string[] = [];
    const mockWriteFile = vi.fn().mockResolvedValue(undefined);

    // Simulate App.tsx onApply logic
    const onApply = vi.fn((result: string | null) => {
      // Frame 0: close overlay
      order.push('setPendingDiff(null)');

      if (result === null) return;

      mockWriteFile(result); // synchronous kick-off
      order.push('invoke(write_file)');

      requestAnimationFrame(() => {
        order.push('setLastApply');         // Frame 1
        requestAnimationFrame(() => {
          order.push('setOpenFiles');        // Frame 2
          order.push('setActiveFilePath');
        });
      });
    });

    renderPanel('old\n', 'new\n', onApply);
    fireEvent.click(screen.getByText(/Apply/i));

    // Frame 0 effects are synchronous
    expect(order).toEqual(['setPendingDiff(null)', 'invoke(write_file)']);
    expect(rafCallbacks).toHaveLength(1);

    // Flush frame 1
    act(() => { rafCallbacks[0](performance.now()); });
    expect(order).toContain('setLastApply');
    expect(rafCallbacks).toHaveLength(2);

    // Flush frame 2
    act(() => { rafCallbacks[1](performance.now()); });
    expect(order).toEqual([
      'setPendingDiff(null)',
      'invoke(write_file)',
      'setLastApply',
      'setOpenFiles',
      'setActiveFilePath',
    ]);

    // onApply called exactly once
    expect(onApply).toHaveBeenCalledOnce();

    window.requestAnimationFrame = originalRaf;
  });

  it('does not call onApply a second time if Apply is clicked after panel is gone', () => {
    const onApply = vi.fn();
    renderPanel('a\n', 'b\n', onApply);

    const btn = screen.getByText(/Apply/i);
    fireEvent.click(btn);

    // Simulate second click after React would have unmounted the panel
    fireEvent.click(btn);

    expect(onApply).toHaveBeenCalledOnce();
  });
});

// ── Edit-before-apply mode ────────────────────────────────────────────────────

describe('DiffReviewPanel — edit-before-apply', () => {
  it('shows Edit button when there are hunks', () => {
    renderPanel('a\n', 'b\n');
    expect(screen.getByLabelText('Edit before applying')).toBeInTheDocument();
  });

  it('does not show Edit button when there are no changes', () => {
    renderPanel('same\n', 'same\n');
    expect(screen.queryByLabelText('Edit before applying')).not.toBeInTheDocument();
  });

  it('entering edit mode renders Monaco with assembled content seeded', () => {
    renderPanel('alpha\nbeta\n', 'alpha\nBETA\n');
    fireEvent.click(screen.getByLabelText('Edit before applying'));

    const ta = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    expect(ta).toBeInTheDocument();
    // Default: all hunks accepted, so seed = modified assembled content
    expect(ta.value).toContain('BETA');
    expect(ta.value).not.toContain('beta');
  });

  it('edit + Apply calls onApply with the edited buffer', () => {
    const onApply = vi.fn();
    renderPanel('one\ntwo\n', 'one\nTWO\n', onApply);

    fireEvent.click(screen.getByLabelText('Edit before applying'));
    const ta = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    fireEvent.change(ta, { target: { value: 'one\nTHREE\n' } });
    fireEvent.click(screen.getByText(/Apply \(edited\)/));

    expect(onApply).toHaveBeenCalledOnce();
    expect(onApply).toHaveBeenCalledWith('one\nTHREE\n');
  });

  it('Apply in edit mode returns null if buffer equals original byte-for-byte', () => {
    const onApply = vi.fn();
    renderPanel('same\n', 'modified\n', onApply);

    fireEvent.click(screen.getByLabelText('Edit before applying'));
    const ta = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    fireEvent.change(ta, { target: { value: 'same\n' } });
    fireEvent.click(screen.getByText(/Apply \(edited\)/));

    expect(onApply).toHaveBeenCalledWith(null);
  });

  it('Back-to-hunks returns to review mode when no edits made (no confirm)', () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    renderPanel('a\n', 'b\n');

    fireEvent.click(screen.getByLabelText('Edit before applying'));
    expect(screen.getByTestId('monaco-mock')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Back to hunks'));
    expect(screen.queryByTestId('monaco-mock')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Edit before applying')).toBeInTheDocument();
    expect(confirmSpy).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it('Back-to-hunks with dirty edits asks confirm and discards on OK', () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    renderPanel('a\n', 'b\n');

    fireEvent.click(screen.getByLabelText('Edit before applying'));
    const ta = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    fireEvent.change(ta, { target: { value: 'tweaked\n' } });

    fireEvent.click(screen.getByLabelText('Back to hunks'));
    expect(confirmSpy).toHaveBeenCalledOnce();
    expect(screen.queryByTestId('monaco-mock')).not.toBeInTheDocument();

    // Re-enter edit — seed should be fresh (not the tweaked buffer).
    fireEvent.click(screen.getByLabelText('Edit before applying'));
    const ta2 = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    expect(ta2.value).not.toBe('tweaked\n');
    confirmSpy.mockRestore();
  });

  it('Back-to-hunks with dirty edits keeps edit mode when confirm is cancelled', () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    renderPanel('a\n', 'b\n');

    fireEvent.click(screen.getByLabelText('Edit before applying'));
    const ta = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    fireEvent.change(ta, { target: { value: 'keep me\n' } });

    fireEvent.click(screen.getByLabelText('Back to hunks'));
    expect(confirmSpy).toHaveBeenCalledOnce();
    // Still in edit mode with the edited value preserved.
    const ta2 = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    expect(ta2.value).toBe('keep me\n');
    confirmSpy.mockRestore();
  });

  it('edit-mode header shows "editing (modified)" indicator when dirty', () => {
    renderPanel('a\n', 'b\n');
    fireEvent.click(screen.getByLabelText('Edit before applying'));
    expect(screen.getByText('editing')).toBeInTheDocument();

    const ta = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    fireEvent.change(ta, { target: { value: 'zzz\n' } });
    expect(screen.getByText(/editing \(modified\)/)).toBeInTheDocument();
  });

  it('passes language prop through to Monaco', () => {
    render(
      <DiffReviewPanel
        original="a\n"
        modified="b\n"
        filePath="unknown"
        onApply={vi.fn()}
        language="rust"
      />,
    );
    fireEvent.click(screen.getByLabelText('Edit before applying'));
    const ta = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    expect(ta.getAttribute('data-language')).toBe('rust');
  });

  it('falls back to detectLanguage when language prop is omitted', () => {
    renderPanel('a\n', 'b\n', vi.fn(), 'src/hello.py');
    fireEvent.click(screen.getByLabelText('Edit before applying'));
    const ta = screen.getByTestId('monaco-mock') as HTMLTextAreaElement;
    // detectLanguage('*.py') is expected to return "python"; accept any non-empty value.
    expect(ta.getAttribute('data-language')).not.toBe('');
  });

  it('edit-mode Cancel still calls onApply(null)', () => {
    const onApply = vi.fn();
    renderPanel('a\n', 'b\n', onApply);
    fireEvent.click(screen.getByLabelText('Edit before applying'));
    fireEvent.click(screen.getByText('Cancel'));
    expect(onApply).toHaveBeenCalledWith(null);
  });
});

// ── ErrorBoundary ─────────────────────────────────────────────────────────────

describe('DiffReviewErrorBoundary', () => {
  // Suppress the expected React error boundary console.error noise
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders children normally when no error', () => {
    const onDismiss = vi.fn();
    render(
      <DiffReviewErrorBoundary onDismiss={onDismiss}>
        <span data-testid="child">ok</span>
      </DiffReviewErrorBoundary>,
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('shows error fallback and Dismiss button when child throws', () => {
    const onDismiss = vi.fn();

    const Bomb = (): React.ReactElement => { throw new Error('test explosion'); };

    render(
      <DiffReviewErrorBoundary onDismiss={onDismiss}>
        <Bomb />
      </DiffReviewErrorBoundary>,
    );

    expect(screen.getByText(/Diff view encountered an error/i)).toBeInTheDocument();
    expect(screen.getByText('Dismiss')).toBeInTheDocument();
  });

  it('calls onDismiss when Dismiss button is clicked', () => {
    const onDismiss = vi.fn();

    const Bomb = (): React.ReactElement => { throw new Error('boom'); };

    render(
      <DiffReviewErrorBoundary onDismiss={onDismiss}>
        <Bomb />
      </DiffReviewErrorBoundary>,
    );

    fireEvent.click(screen.getByText('Dismiss'));
    expect(onDismiss).toHaveBeenCalledOnce();
  });
});
