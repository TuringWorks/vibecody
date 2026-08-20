/**
 * The shared "Fix with AI" hand-off, and the code reviewer that now uses it.
 *
 * The panel edits nothing: it writes a change request into the chat composer
 * and the user presses send. The composer is usually behind another tab, so the
 * button has to say the request was written — a hand-off that silently
 * succeeded read as broken and got clicked again.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { FixWithAIButton } from '../FixWithAIButton';
import { ReviewPanel } from '../ReviewPanel';
import type { FixItem } from '../../lib/fixWithAI';

const injected: string[] = [];
const listener = (e: Event) => injected.push((e as CustomEvent<string>).detail);

beforeEach(() => {
  mockInvoke.mockReset();
  injected.length = 0;
  window.addEventListener('vibecoder:inject-context', listener);
});
afterEach(() => window.removeEventListener('vibecoder:inject-context', listener));

const item = (over: Partial<FixItem> = {}): FixItem => ({
  file: 'src/a.rs',
  line: 42,
  severity: 'critical',
  message: 'Query built by concatenation.',
  ...over,
});

describe('FixWithAIButton', () => {
  it('writes the request into the composer and then says it did', () => {
    render(<FixWithAIButton items={[item()]} source="code review" />);

    fireEvent.click(screen.getByRole('button', { name: 'Fix with AI' }));

    expect(injected).toHaveLength(1);
    expect(injected[0]).toContain('src/a.rs:42');
    expect(screen.getByRole('button', { name: /Sent to chat/ })).toBeTruthy();
  });

  it('caps a batch and names the cap in the label and the request', () => {
    const many = Array.from({ length: 40 }, (_, i) => item({ line: 100 + i }));
    render(<FixWithAIButton items={many} source="code review" />);

    fireEvent.click(screen.getByRole('button', { name: 'Fix first 25 of 40 with AI' }));

    expect(injected[0]).toContain('first 25 of 40');
    expect(injected[0]).toContain('line 124');     // the 25th
    expect(injected[0]).not.toContain('line 125'); // the 26th, dropped
  });

  it('stops claiming the last run was sent when a new run arrives', () => {
    const { rerender } = render(<FixWithAIButton items={[item()]} source="code review" resetKey={1} />);
    fireEvent.click(screen.getByRole('button', { name: 'Fix with AI' }));
    expect(screen.getByRole('button', { name: /Sent to chat/ })).toBeTruthy();

    rerender(<FixWithAIButton items={[item()]} source="code review" resetKey={2} />);
    expect(screen.getByRole('button', { name: 'Fix with AI' })).toBeTruthy();
  });

  it('is disabled with nothing to hand over', () => {
    render(<FixWithAIButton items={[]} source="code review" />);
    const button = screen.getByRole('button');
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(injected).toHaveLength(0);
  });
});

/** Run a review whose reply is `report`, and wait for it to land. */
async function review(report: unknown) {
  mockInvoke.mockImplementation((cmd: string) =>
    cmd === 'run_code_review' ? Promise.resolve(report) : Promise.resolve(null),
  );
  render(<ReviewPanel workspacePath="/ws" />);
  fireEvent.click(screen.getByRole('button', { name: /Run Review/ }));
  await waitFor(() => expect(screen.getByText(/Quality Score/)).toBeTruthy());
}

const issue = (over: Record<string, unknown> = {}) => ({
  file: 'src/a.ts',
  line: 3,
  severity: 'high',
  category: 'correctness',
  description: 'oid:0 breaks the stable-across-restarts guarantee',
  suggested_fix: 'Map the persisted OID through the conversion.',
  ...over,
});

describe('ReviewPanel — Fix with AI', () => {
  it('hands one issue to chat instead of editing anything', async () => {
    await review({ summary: 's', issues: [issue()], score: {} });

    fireEvent.click(screen.getAllByRole('button', { name: 'Fix with AI' })[0]);

    expect(injected).toHaveLength(1);
    // The path and line must be in the request, or the model writes a second
    // copy of the file instead of fixing the one that has the bug.
    expect(injected[0]).toContain('src/a.ts');
    expect(injected[0]).toContain('line 3');
    expect(injected[0]).toContain('oid:0 breaks');
    expect(injected[0]).toContain('Map the persisted OID through the conversion.');
    expect(injected[0]).toMatch(/do not create a new file/i);

    // Nothing was written: the only command a review runs is the review.
    expect(mockInvoke.mock.calls.map(([cmd]) => cmd)).toEqual(['run_code_review']);
  });

  it("sends the reviewer's own severity word, not the bucket it was coloured by", async () => {
    await review({ summary: 's', issues: [issue({ severity: 'blocker' })], score: {} });

    fireEvent.click(screen.getAllByRole('button', { name: 'Fix with AI' })[0]);

    expect(injected[0]).toContain('[blocker]');
  });

  it('hands over only what the severity filter leaves visible', async () => {
    await review({
      summary: 's',
      issues: [
        issue({ description: 'a critical one', severity: 'critical' }),
        issue({ description: 'an info one', severity: 'info' }),
      ],
      score: {},
    });

    fireEvent.click(screen.getByRole('button', { name: /^Info \(1\)$/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Fix all 1 with AI' }));

    expect(injected).toHaveLength(1);
    expect(injected[0]).toContain('an info one');
    expect(injected[0]).not.toContain('a critical one');
  });
});
