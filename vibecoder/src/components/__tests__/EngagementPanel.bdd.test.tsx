/**
 * BDD tests for EngagementPanel.
 *
 * The fixtures in `fixtures-engagement.json` are not invented: they are the
 * verbatim responses of a live `vibecli serve` for a freshly created
 * engagement, captured from `/engagements`, `/engagements/{id}`,
 * `/engagements/{id}/deliverables` and `/engagements/{id}/gates`. A hand-written
 * shape would have passed against the panel and still disagreed with the
 * daemon — which is exactly the failure these tests exist to catch.
 *
 * Scenarios:
 *  - The seeded engagement appears in the selector, named with its client
 *  - Selecting it renders all four phases with their real titles
 *  - Discover publishes no cadence, so it renders a dash rather than a guess
 *  - A seeded engagement is 0% accepted with blockers, and cannot exit a phase
 *  - Gate counts read 3 not measured — not "0 pass" dressed up as a result
 *  - The board lists the phase's own deliverables and gates, and no others
 *  - A daemon 404 (a stale daemon predating /engagements) is surfaced verbatim
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import fixtures from './fixtures-engagement.json';

// The panel talks to the daemon through daemonFetch, which resolves the bearer
// through a Tauri command — unavailable in jsdom. Route paths to the captured
// payloads instead.
const routes = vi.hoisted(() => ({ handler: null as null | ((url: string) => unknown) }));
vi.mock('../../lib/daemonFetch', () => ({
  daemonFetch: vi.fn(async (url: string) => routes.handler!(url)),
}));

import { EngagementPanel } from '../EngagementPanel';

const json = (body: unknown, status = 200) => ({
  ok: status >= 200 && status < 300,
  status,
  json: async () => body,
  text: async () => JSON.stringify(body),
});

const ENGAGEMENT = fixtures.list.engagements[0];

const servesFixtures = (url: string) => {
  const path = url.replace(/^https?:\/\/[^/]+/, '');
  if (path === '/engagements') return json(fixtures.list);
  if (path.endsWith('/deliverables')) return json(fixtures.deliverables);
  if (path.endsWith('/gates')) return json(fixtures.gates);
  if (path.startsWith('/engagements/')) return json(fixtures.report);
  return json({ error: 'Not found' }, 404);
};

/** Render, then pick the seeded engagement out of the selector. */
async function renderSelected() {
  render(<EngagementPanel />);
  const select = await screen.findByLabelText('Select engagement');
  fireEvent.change(select, { target: { value: ENGAGEMENT.id } });
  await screen.findByText('Discover & Assess');
  return select as HTMLSelectElement;
}

beforeEach(() => {
  routes.handler = servesFixtures;
});

// ── The selector ────────────────────────────────────────────────────────────

describe('EngagementPanel — engagement list', () => {
  it('lists the engagement the daemon returned, with client and status', async () => {
    render(<EngagementPanel />);
    const option = await screen.findByRole('option', {
      name: 'Acme Platform Modernization · Acme Corp (draft)',
    });
    expect(option).toBeDefined();
  });

  it('shows the empty-board hint until an engagement is selected', async () => {
    render(<EngagementPanel />);
    await screen.findByRole('option', { name: /Acme Platform/ });
    expect(
      screen.getByText('Select or create an engagement to see its phase board.')
    ).toBeDefined();
  });
});

// ── The phase board ─────────────────────────────────────────────────────────

describe('EngagementPanel — phase board', () => {
  it('renders all four phases of the seeded engagement', async () => {
    await renderSelected();
    for (const title of [
      'Discover & Assess',
      'Prove',
      'Build & Harden',
      'Operate & Transfer',
    ]) {
      expect(screen.getByText(title)).toBeDefined();
    }
  });

  it('renders a dash for Discover, which publishes no cadence', async () => {
    await renderSelected();
    const board = screen.getByRole('group', { name: 'Phase readiness' });
    const discover = within(board).getByText('Discover & Assess')
      .parentElement as HTMLElement;
    expect(within(discover).getByText('01 · —')).toBeDefined();
  });

  it('reports a seeded phase as 0% accepted with blockers, not as ready', async () => {
    await renderSelected();
    const board = screen.getByRole('group', { name: 'Phase readiness' });
    const discover = within(board).getByText('Discover & Assess')
      .parentElement as HTMLElement;
    expect(discover.textContent).toContain('0% accepted');
    expect(discover.textContent).toContain('12 blockers');
    expect(discover.textContent).not.toContain('ready');
  });

  it('cannot close a phase whose blockers are outstanding', async () => {
    await renderSelected();
    const close = screen.getByRole('button', { name: 'Close phase' });
    expect((close as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByRole('button', { name: 'Advance anyway' })).toBeDefined();
  });

  it('renders unmeasured gates as not measured rather than as zero passes', async () => {
    await renderSelected();
    expect(
      screen.getByText(
        /0 pass · 0 fail · 0 pending · 3 not measured · 0 waived/
      )
    ).toBeDefined();
  });
});

// ── Deliverables and gates are filtered by the active phase ─────────────────

describe('EngagementPanel — deliverables and gates', () => {
  it("lists the active phase's deliverables and none from another phase", async () => {
    await renderSelected();
    expect(screen.getByText('Current-state architecture map')).toBeDefined();
    expect(screen.getByText('Technical-debt register')).toBeDefined();
    // Seeded into Prove, so it must not appear on the Discover board.
    expect(
      screen.queryByText('Working pilot deployed in your environment')
    ).toBeNull();
  });

  it('switching phase swaps the board to that phase’s items', async () => {
    await renderSelected();
    fireEvent.click(screen.getByText('Prove'));
    await screen.findByText('Working pilot deployed in your environment');
    expect(screen.queryByText('Current-state architecture map')).toBeNull();
  });

  it('renders each seeded gate with its criterion', async () => {
    await renderSelected();
    expect(screen.getByText('Inventory is complete')).toBeDefined();
    expect(screen.getByText('Requirements are testable')).toBeDefined();
  });
});

// ── The failure this session actually hit ───────────────────────────────────

describe('EngagementPanel — a daemon that does not serve the routes', () => {
  it("surfaces the daemon's own 404 body rather than a generic failure", async () => {
    routes.handler = () => json({ error: 'Not found' }, 404);
    render(<EngagementPanel />);
    await waitFor(() =>
      expect(screen.getByRole('alert').textContent).toBe('Not found')
    );
  });
});
