import { render, screen, waitFor, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('lucide-react', () => ({
  FolderOpen: () => <span />,
  AlertTriangle: () => <span />,
  X: () => <span />,
  ChevronDown: () => <span />,
}));

// The review body is a panel of its own; this test is about which tab shows it.
// `useCodeReview` and `ReviewControls` are real enough to place: the controls
// render an input and a button, and the hook holds the run state that ties the
// two tabs together.
vi.mock('../ReviewPanel', () => ({
  ReviewPanel: () => <div data-testid="review-panel">review findings</div>,
  ReviewControls: ({ onRun }: { onRun?: () => void }) => (
    <div>
      <input placeholder="Base ref (e.g. main, HEAD~1) — leave blank for uncommitted" />
      <button onClick={onRun}>Run Review</button>
    </div>
  ),
  useCodeReview: () => ({
    baseRef: '',
    setBaseRef: vi.fn(),
    isLoading: false,
    report: null,
    error: null,
    runId: 0,
    runReview: vi.fn(),
  }),
}));

vi.mock('../Toaster', () => ({ Toaster: () => null }));
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    toast: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), success: vi.fn() },
    dismiss: vi.fn(),
  }),
}));

import { GitPanel } from '../GitPanel';

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'get_git_status':
        return Promise.resolve({ branch: 'main', file_statuses: { 'src/a.ts': 'M' } });
      case 'git_list_branches':
        return Promise.resolve(['main']);
      case 'get_github_sync_status':
        return Promise.resolve({ has_remote: false });
      default:
        return Promise.resolve(null);
    }
  });
});

async function renderPanel() {
  render(<GitPanel workspacePath="/repo" selectedProvider="ollama" />);
  // Wait for the Changes *body*, not the tab bar. The panel renders its tab bar
  // during the "Loading git status…" state too, so waiting on a tab returned
  // before `get_git_status` resolved — and under a loaded suite the body was
  // still absent when the assertions ran. The heading only appears once the
  // status has landed.
  await waitFor(() =>
    expect(screen.getByRole('heading', { name: 'Changes' })).toBeInTheDocument(),
  );
}

const openTab = async (name: string) => {
  await act(async () => { fireEvent.click(screen.getByRole('tab', { name })); });
};

/**
 * No JSX-children comment leaked into the rendered text.
 *
 * A `/* … *\/` comment is only a comment in JSX while it is wrapped in braces.
 * Lifted into children position — which is what happens when a block is moved
 * out of a `{cond && ( … )}` wrapper — it becomes literal text, and neither
 * `tsc` nor eslint has anything to say about it: JSX text is valid.
 *
 * This shipped. The Review tab rendered five lines of a comment about
 * `overflow: hidden` above its controls. The tab tests passed throughout,
 * because they asserted what *should* be present and never that nothing else
 * was.
 */
function expectNoRawComments() {
  const text = document.body.textContent ?? '';
  expect(text, `rendered text contains comment syntax:\n${text.slice(0, 400)}`)
    .not.toMatch(/\/\*|\*\//);
}

describe('history diff', () => {
  /**
   * A history entry asks what *that commit* changed, which is its parent
   * against itself — not HEAD against the working tree.
   *
   * Getting it wrong is invisible rather than loud: while browsing history the
   * working tree usually matches HEAD, so the two sides come back identical
   * and the viewer renders an empty diff. The commit looks like it changed
   * nothing, and there is no error anywhere to say otherwise.
   */
  it('asks for the commit and its parent, not the working tree', async () => {
    const onCompareFile = vi.fn();
    mockInvoke.mockImplementation((cmd: string) => {
      switch (cmd) {
        case 'get_git_status':
          return Promise.resolve({ branch: 'main', file_statuses: {} });
        case 'git_list_branches':
          return Promise.resolve(['main']);
        case 'git_get_history':
          return Promise.resolve([
            { hash: 'abcdef1234567890', author: 'me', timestamp: 1, message: 'a commit' },
          ]);
        case 'git_get_commit_files':
          return Promise.resolve(['src/cli.ts']);
        default:
          return Promise.resolve(null);
      }
    });

    render(<GitPanel workspacePath="/repo" onCompareFile={onCompareFile} />);
    await waitFor(() =>
      expect(screen.getByRole('heading', { name: 'Changes' })).toBeInTheDocument(),
    );
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'History' })); });
    await act(async () => { fireEvent.click(screen.getByText('a commit')); });
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Diff' })); });

    expect(onCompareFile).toHaveBeenCalledWith('src/cli.ts', 'abcdef1234567890');
  });
});

describe('GitPanel tabs', () => {
  it('offers Changes, Review, Tools and GitHub as tabs — and only those', async () => {
    await renderPanel();
    for (const name of ['Changes', 'Review', 'Tools', 'GitHub']) {
      expect(screen.getByRole('tab', { name })).toBeInTheDocument();
    }
    // Changelog and Settings folded into Tools; a tab bar that keeps growing
    // is the thing this panel was moved away from.
    expect(screen.getAllByRole('tab')).toHaveLength(4);
  });

  /**
   * The point of the change: these three used to be collapsible sections
   * stacked under the changes list, sharing its one scroll region. Each now
   * owns the panel, so none of them may render while Changes is showing.
   */
  it('does not render the three bodies inside the Changes view', async () => {
    await renderPanel();
    // The heading, not the tab — both say "Changes".
    expect(screen.getByRole('heading', { name: 'Changes' })).toBeInTheDocument();
    // The review *findings* stay on their tab; only the control that starts a
    // run lives here.
    expect(screen.queryByTestId('review-panel')).toBeNull();
    expect(screen.queryByPlaceholderText(/since \(e\.g\. HEAD~10/)).toBeNull();
    expect(screen.queryByPlaceholderText('User name')).toBeNull();
    // The AI git tools are neither a property of the working tree nor a review
    // of it, so they are not here either.
    expect(screen.queryByPlaceholderText('Describe the task…')).toBeNull();
    expect(screen.queryByPlaceholderText(/File path/)).toBeNull();
  });

  it('shows the AI git tools on their own tab', async () => {
    await renderPanel();
    await openTab('Tools');
    expect(screen.getByPlaceholderText('Describe the task…')).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/File path/)).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Changes' })).toBeNull();
  });

  it('shows the review findings on their own tab', async () => {
    await renderPanel();
    await openTab('Review');
    expect(screen.getByTestId('review-panel')).toBeInTheDocument();
    // …and the changes list is no longer competing for the height.
    expect(screen.queryByText('No changes')).toBeNull();
  });

  /** Every tab, because the leak was in one of them and nothing said so. */
  it('renders no comment syntax as text on any tab', async () => {
    await renderPanel();
    for (const name of ['Changes', 'Review', 'Tools']) {
      // eslint-disable-next-line no-await-in-loop
      await openTab(name);
      expectNoRawComments();
    }
  });

  /**
   * Starting a review is an action on the changes being reviewed, so the
   * control sits with them. The findings do not follow it back.
   */
  it('puts the Run Review control on the Changes tab, not the Review tab', async () => {
    await renderPanel();
    expect(screen.getByRole('button', { name: 'Run Review' })).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/Base ref/)).toBeInTheDocument();
    expect(screen.queryByTestId('review-panel')).toBeNull();

    await openTab('Review');
    expect(screen.queryByRole('button', { name: 'Run Review' })).toBeNull();
    expect(screen.queryByPlaceholderText(/Base ref/)).toBeNull();
  });

  /**
   * The report lands on the Review tab, so pressing Run from Changes must go
   * there. Otherwise the button reads as one that did nothing.
   */
  it('switches to the Review tab when a run starts', async () => {
    await renderPanel();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Run Review' }));
    });
    const selected = screen.getAllByRole('tab').filter(
      (t) => t.getAttribute('aria-selected') === 'true',
    );
    expect(selected[0]).toHaveTextContent('Review');
    expect(screen.getByTestId('review-panel')).toBeInTheDocument();
  });

  it('shows the changelog generator under Tools', async () => {
    await renderPanel();
    await openTab('Tools');
    expect(screen.getByPlaceholderText(/since \(e\.g\. HEAD~10/)).toBeInTheDocument();
    expect(screen.queryByTestId('review-panel')).toBeNull();
  });

  it('shows git settings under Tools', async () => {
    await renderPanel();
    await openTab('Tools');
    expect(screen.getByPlaceholderText('User name')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Email')).toBeInTheDocument();
    expect(screen.queryByTestId('review-panel')).toBeNull();
  });

  /** All four, on one tab, each labelled. */
  it('gathers every git tool under Tools', async () => {
    await renderPanel();
    await openTab('Tools');
    for (const heading of ['AI Branch Name', 'Resolve Merge Conflict', 'Generate Changelog', 'Git Settings']) {
      expect(screen.getByText(heading)).toBeInTheDocument();
    }
  });

  it('marks exactly one tab selected at a time', async () => {
    await renderPanel();
    await openTab('Tools');
    const selected = screen.getAllByRole('tab').filter(
      (t) => t.getAttribute('aria-selected') === 'true',
    );
    expect(selected).toHaveLength(1);
    expect(selected[0]).toHaveTextContent('Tools');
  });

  /** Switching away and back must not lose what was typed. */
  it('keeps changelog input across a tab switch', async () => {
    await renderPanel();
    await openTab('Tools');
    const input = screen.getByPlaceholderText(/since \(e\.g\. HEAD~10/) as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'v1.2.0' } });
    await openTab('Changes');
    await openTab('Tools');
    expect((screen.getByPlaceholderText(/since \(e\.g\. HEAD~10/) as HTMLInputElement).value)
      .toBe('v1.2.0');
  });
});
