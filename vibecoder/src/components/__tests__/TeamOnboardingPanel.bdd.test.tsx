/**
 * BDD: the Team Onboarding panel says what it measured, and where from.
 *
 * This panel invented three facts. It ran `git log` on whatever folder was
 * open, then headed the commit count "Sessions", labelled anyone under five
 * commits "New" and everyone else "Member", and called the first-commit date
 * "Joined". Open a checkout of an unrelated open-source project and the panel
 * reported that project's 129 contributors as colleagues who had been using
 * the product — names, personal email addresses and all. The numbers were
 * right; every word around them was wrong.
 *
 * So these scenarios assert on wording as much as data. Two of them assert
 * that specific words are *absent*, which is the only way to keep a fabricated
 * metric from growing back.
 */

import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { TeamOnboardingPanel } from '../TeamOnboardingPanel';

// Shaped after the real thing: a checkout of someone else's repository, which
// is the case that produced the bug.
const CONTRIBUTORS = {
  repo: '/src/git-oss/gbrain',
  contributors: [
    {
      user_id: 'garrytan@gmail.com',
      name: 'Garry Tan',
      email: 'garrytan@gmail.com',
      commits: 417,
      first_commit: '2026-04-05',
    },
    {
      user_id: 'someone@users.noreply.github.com',
      name: 'Someone Else',
      email: 'someone@users.noreply.github.com',
      commits: 4,
      first_commit: '2026-07-17',
    },
  ],
};

const GUIDE = {
  repo: '/src/git-oss/gbrain',
  contributor: 'Garry Tan',
  scanned_commits: 2000,
  error: null as string | null,
  markdown: '# Onboarding guide for Garry Tan\n\n1. `src/main.rs` — 12 commits\n',
};

const HOTSPOTS = {
  repo: '/src/git-oss/gbrain',
  scanned_commits: 1000,
  files: [{ file_path: 'src/main.rs', commits: 42, contributor_count: 7 }],
};

function daemon(over: Record<string, unknown> = {}) {
  const answers: Record<string, unknown> = {
    team_onboarding_members: CONTRIBUTORS,
    team_onboarding_gaps: [],
    team_onboarding_hotspots: HOTSPOTS,
    team_onboarding_guide: GUIDE,
    ...over,
  };
  // Resolve rather than reject on an unknown command: a stray call from a
  // still-mounted panel in a finished test would otherwise surface as a
  // file-level unhandled rejection and mask every real assertion.
  mockInvoke.mockImplementation((cmd: string) => Promise.resolve(answers[cmd]));
}

/** Deepest element whose whole text matches — the provenance line is built
 *  from text plus a `<code>`, which the default node-at-a-time matcher misses. */
function findWholeText(re: RegExp) {
  return screen.findByText((_c, el) => {
    if (!el || !re.test(el.textContent ?? '')) return false;
    return !Array.from(el.children).some(c => re.test(c.textContent ?? ''));
  });
}

beforeEach(() => mockInvoke.mockReset());

describe('Given a folder whose git history is not this team', () => {
  it('When the contributors tab renders, Then it names the repository the rows came from', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    // The question this panel could not answer was "who are these people".
    // The answer is the folder, so the folder is on screen with them.
    const line = await findWholeText(/Commit authors in .*gbrain/);
    expect(line.textContent).toContain('git log');
    expect(line.textContent).toMatch(/Not product usage/);
  });

  it('When the columns render, Then each names what it holds', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    await screen.findByText('Garry Tan');
    for (const header of ['Contributor', 'Email', 'Commits', 'First commit']) {
      expect(screen.getByRole('columnheader', { name: header })).toBeTruthy();
    }
  });

  it('When the columns render, Then no column claims a session or a join', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    await screen.findByText('Garry Tan');
    // `Sessions` was a commit count and `Joined` was a first-commit date.
    // Neither was ever observed, so neither word may reappear.
    expect(screen.queryByRole('columnheader', { name: 'Sessions' })).toBeNull();
    expect(screen.queryByRole('columnheader', { name: 'Joined' })).toBeNull();
    expect(screen.queryByRole('columnheader', { name: 'Status' })).toBeNull();
  });

  it('When a contributor has few commits, Then they are not labelled a new member', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    await screen.findByText('Someone Else');
    // Four commits used to render a "New" badge and 417 a "Member" one, from
    // `commits < 5`. Membership of anything was never in the data.
    expect(screen.queryByText('New')).toBeNull();
    expect(screen.queryByText('Member')).toBeNull();
    expect(screen.getByText('4')).toBeTruthy();
    expect(screen.getByText('417')).toBeTruthy();
  });

  it('When the counts render, Then they are the commit counts as given', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    const row = (await screen.findByText('Garry Tan')).closest('tr') as HTMLElement;
    expect(row.textContent).toContain('417');
    expect(row.textContent).toContain('2026-04-05');
  });
});

describe("Given contributors who are strangers to this machine's owner", () => {
  it('When the table renders, Then no full address is on screen', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    await screen.findByText('Garry Tan');
    // The row still identifies the person — name, and enough of the address to
    // tell a colleague from an outside contributor — without publishing the
    // mailbox to anyone who screenshots the panel.
    expect(screen.getByText('g•••@gmail.com')).toBeTruthy();
    expect(screen.queryByText('garrytan@gmail.com')).toBeNull();
  });

  it('When emails are revealed, Then the full address is shown and can be hidden again', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    const toggle = await screen.findByRole('button', { name: 'Show emails' });
    expect(toggle.getAttribute('aria-pressed')).toBe('false');
    fireEvent.click(toggle);

    expect(await screen.findByText('garrytan@gmail.com')).toBeTruthy();
    // Revealing must be reversible in place; a one-way door is a worse default
    // than no toggle at all.
    fireEvent.click(screen.getByRole('button', { name: 'Hide emails' }));
    await waitFor(() => expect(screen.queryByText('garrytan@gmail.com')).toBeNull());
  });

  it('When an address is masked, Then the mask does not leak its length', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    await screen.findByText('Someone Else');
    // A GitHub noreply local part is long and a gmail one short; both mask to
    // the same width, so the mask says nothing about what it hides.
    expect(screen.getByText('s•••@users.noreply.github.com')).toBeTruthy();
  });
});

describe('Given a repository with hotspots', () => {
  it('When they render, Then the count is commits and the window is stated', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    // The tab bar is the only way in; the panel opens on contributors.
    (await screen.findByRole('button', { name: 'hotspots' })).click();

    expect(await screen.findByText('42 commits')).toBeTruthy();
    // A ranking over the last 1,000 commits is misread as all-time unless the
    // window travels with it.
    const line = await findWholeText(/last 1,000 commits/);
    expect(line.textContent).toContain('gbrain');
    // "accesses" implied the file was read. Nothing observed a read.
    expect(screen.queryByText(/accesses/)).toBeNull();
  });
});

describe("Given a contributor's guide", () => {
  it('When it is built, Then it is titled by name, not by address', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    fireEvent.click(await screen.findByRole('button', { name: 'guide' }));

    // The address was masked one tab over, and this block exists to be copied
    // and pasted somewhere else.
    const pre = await screen.findByText(/Onboarding guide for Garry Tan/);
    expect(pre.textContent).not.toContain('garrytan@gmail.com');
  });

  it('When git cannot be read, Then it says so instead of blaming the contributor', async () => {
    daemon({
      team_onboarding_guide: {
        ...GUIDE,
        error: 'could not run git: No such file or directory',
        markdown: '',
      },
    });
    render(<TeamOnboardingPanel />);

    fireEvent.click(await screen.findByRole('button', { name: 'guide' }));

    const line = await findWholeText(/Could not read this repository/);
    expect(line.textContent).toContain('could not run git');
    // The old code returned "No git history available for this user" for a
    // failed spawn, which asserts a fact about the person from a fact about
    // the machine.
    expect(screen.queryByText(/No git history available/)).toBeNull();
  });
});

describe('Given a knowledge-gaps engine with no data source', () => {
  it('When the panel renders, Then there is no gaps tab to mislead anyone', async () => {
    daemon();
    render(<TeamOnboardingPanel />);

    await screen.findByText('Garry Tan');
    // `team_onboarding_gaps` returns an empty list unconditionally, so the tab
    // could only ever say "No knowledge gaps identified" — a finding, not the
    // absence of one.
    expect(screen.queryByRole('button', { name: 'gaps' })).toBeNull();
    expect(screen.queryByText(/knowledge gaps/i)).toBeNull();
  });
});

describe('Given a folder with no commits', () => {
  it('When the panel renders, Then it says so without inventing a team', async () => {
    daemon({
      team_onboarding_members: { repo: '/src/empty', contributors: [] },
      team_onboarding_hotspots: { repo: '/src/empty', scanned_commits: 1000, files: [] },
    });
    render(<TeamOnboardingPanel />);

    await waitFor(() =>
      expect(screen.getByText('No commits found in this folder.')).toBeTruthy(),
    );
    // The old copy was "No team members found", which asserts a team exists
    // and is empty. There is no team here either way.
    expect(screen.queryByText(/team members/i)).toBeNull();
  });
});
