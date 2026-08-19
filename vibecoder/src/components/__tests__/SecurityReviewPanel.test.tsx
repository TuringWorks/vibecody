import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// The panel resolves the toolbar's display name to a provider id + model.
vi.mock('../../hooks/useModelRegistry', () => ({
  parseProviderSelection: (display: string) =>
    display ? { provider: 'ollama', model: 'devstral-2' } : { provider: '', model: '' },
}));

vi.mock('../../utils/effort', () => ({ getSelectedEffort: () => 'medium' }));

// ── Import after mocks ────────────────────────────────────────────────────

import { SecurityReviewPanel } from '../SecurityReviewPanel';

// ── Test data ──────────────────────────────────────────────────────────────

const FINDING = {
  severity: 'critical',
  message: 'Command built from unsanitised input',
  file: 'src/auth/login.rs',
  line: 42,
  suggestion: 'Escape the argument or use a parameterised call',
};

/**
 * @param targets what `security_review_targets` resolves to
 * @param findingsByFile findings each file's review returns
 */
function setupMocks(
  targets: { files: string[]; matched: number; limit: number },
  findingsByFile: Record<string, unknown[]> = {}
) {
  mockInvoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
    if (cmd === 'security_review_targets') return targets;
    if (cmd === 'read_file') return '// source';
    if (cmd === 'security_review_file') return findingsByFile[args.file as string] ?? [];
    throw new Error(`unexpected command ${cmd}`);
  });
}

function renderPanel(props: Record<string, unknown> = {}) {
  return render(
    <SecurityReviewPanel workspacePath="/ws" provider="Ollama (devstral-2)" {...props} />
  );
}

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('SecurityReviewPanel — workspace scope', () => {
  /// The panel used to take one file path, so a user typing `src/*` got
  /// "No such file or directory". Scope resolution is the backend's job now.
  it('reviews the whole workspace when no pattern is given', async () => {
    setupMocks({ files: ['src/a.rs', 'src/b.rs'], matched: 2, limit: 40 });
    renderPanel();

    fireEvent.click(screen.getByRole('button', { name: 'Review' }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('security_review_targets', {
        workspace: '/ws',
        pattern: null,
      });
    });
    await waitFor(() => {
      const reviewed = mockInvoke.mock.calls
        .filter(([cmd]) => cmd === 'security_review_file')
        .map(([, args]) => (args as { file: string }).file);
      expect(reviewed).toEqual(['src/a.rs', 'src/b.rs']);
    });
  });

  it('passes a glob through as the pattern', async () => {
    setupMocks({ files: ['src/a.rs'], matched: 1, limit: 40 });
    renderPanel();

    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'src/*' } });
    fireEvent.click(screen.getByRole('button', { name: 'Review' }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('security_review_targets', {
        workspace: '/ws',
        pattern: 'src/*',
      });
    });
  });

  /// A capped run must not read as a clean workspace.
  it('says how many matched files it did not review', async () => {
    setupMocks({ files: ['src/a.rs'], matched: 213, limit: 1 });
    renderPanel();

    fireEvent.click(screen.getByRole('button', { name: 'Review' }));

    await waitFor(() => {
      expect(screen.getByText(/212 were not reviewed/)).toBeTruthy();
    });
  });

  /// One unreadable file must not silently shrink the reported coverage.
  it('reports a file it could not review and keeps going', async () => {
    mockInvoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'security_review_targets') return { files: ['bad.rs', 'src/b.rs'], matched: 2, limit: 40 };
      if (cmd === 'read_file') {
        if (args.path === 'bad.rs') throw new Error('permission denied');
        return '// source';
      }
      if (cmd === 'security_review_file') return [];
      throw new Error(`unexpected command ${cmd}`);
    });
    renderPanel();

    fireEvent.click(screen.getByRole('button', { name: 'Review' }));

    await waitFor(() => {
      expect(screen.getByText(/could not be reviewed/)).toBeTruthy();
    });
    const reviewed = mockInvoke.mock.calls
      .filter(([cmd]) => cmd === 'security_review_file')
      .map(([, a]) => (a as { file: string }).file);
    expect(reviewed).toEqual(['src/b.rs']);
  });

  it('refuses to run without a provider selected', async () => {
    setupMocks({ files: [], matched: 0, limit: 40 });
    renderPanel({ provider: '' });

    fireEvent.click(screen.getByRole('button', { name: 'Review' }));

    await waitFor(() => {
      expect(screen.getByText(/Select a provider/)).toBeTruthy();
    });
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

describe('SecurityReviewPanel — Fix with AI', () => {
  async function renderWithOneFinding() {
    setupMocks({ files: ['src/auth/login.rs'], matched: 1, limit: 40 }, {
      'src/auth/login.rs': [FINDING],
    });
    renderPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Review' }));
    await waitFor(() => expect(screen.getByText(FINDING.message)).toBeTruthy());
  }

  it('hands the change request to chat instead of editing anything', async () => {
    const injected: string[] = [];
    const listener = (e: Event) => injected.push((e as CustomEvent<string>).detail);
    window.addEventListener('vibecoder:inject-context', listener);

    await renderWithOneFinding();
    fireEvent.click(screen.getByRole('button', { name: 'Fix with AI' }));

    expect(injected).toHaveLength(1);
    const request = injected[0];
    // The path must be in the request, or the model writes a second copy of
    // the file instead of fixing the one that has the bug.
    expect(request).toContain('src/auth/login.rs');
    expect(request).toContain('line 42');
    expect(request).toContain(FINDING.message);
    expect(request).toContain(FINDING.suggestion);
    expect(request).toMatch(/do not create a new file/i);

    // Nothing was written.
    const writes = mockInvoke.mock.calls.filter(([cmd]) => String(cmd).includes('write'));
    expect(writes).toEqual([]);

    window.removeEventListener('vibecoder:inject-context', listener);
  });

  it('confirms the request reached chat', async () => {
    await renderWithOneFinding();
    fireEvent.click(screen.getByRole('button', { name: 'Fix with AI' }));
    expect(screen.getByRole('button', { name: /Sent to chat/ })).toBeTruthy();
  });

  it('sends every finding in one request from the header CTA', async () => {
    const injected: string[] = [];
    const listener = (e: Event) => injected.push((e as CustomEvent<string>).detail);
    window.addEventListener('vibecoder:inject-context', listener);

    setupMocks({ files: ['a.rs', 'b.rs'], matched: 2, limit: 40 }, {
      'a.rs': [{ ...FINDING, file: 'a.rs', message: 'first' }],
      'b.rs': [{ ...FINDING, file: 'b.rs', message: 'second', severity: 'warning' }],
    });
    renderPanel();
    fireEvent.click(screen.getByRole('button', { name: 'Review' }));

    await waitFor(() => expect(screen.getByRole('button', { name: /Fix all 2 with AI/ })).toBeTruthy());
    fireEvent.click(screen.getByRole('button', { name: /Fix all 2 with AI/ }));

    expect(injected).toHaveLength(1);
    expect(injected[0]).toContain('a.rs');
    expect(injected[0]).toContain('b.rs');
    expect(injected[0]).toContain('first');
    expect(injected[0]).toContain('second');

    window.removeEventListener('vibecoder:inject-context', listener);
  });
});
