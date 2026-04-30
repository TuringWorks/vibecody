import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';

// ── Mock lucide-react icons ────────────────────────────────────────────────
vi.mock('lucide-react', () => {
  const icon = (name: string) => {
    const Component = (props: Record<string, unknown>) => <span data-testid={`icon-${name}`} {...props} />;
    Component.displayName = name;
    return Component;
  };
  const names = ['ChevronDown', 'ChevronRight', 'Play', 'FileText', 'Link', 'Briefcase', 'GitBranch'];
  return Object.fromEntries(names.map(n => [n, icon(n)]));
});

import { RecapCard } from '../RecapCard';
import type { Recap } from '../../types/recap';

function makeRecap(overrides: Partial<Recap> = {}): Recap {
  return {
    id: 'rcp_abc123',
    kind: 'session',
    subject_id: 'sess_xyz',
    last_message_id: 42,
    workspace: '/repo',
    generated_at: '2026-04-29T12:00:00Z',
    generator: { type: 'heuristic' },
    headline: 'Wired auth refresh-token rotation',
    bullets: ['Ran `cargo test` (3×)', 'Edited src/auth.rs', 'Stopped: rate limit hit'],
    next_actions: ['Wire refresh token to frontend', 'Add expiry e2e test'],
    artifacts: [
      { kind: 'file', label: 'auth.rs', locator: 'src/auth.rs' },
      { kind: 'job', label: 'cargo test', locator: 'job_99' },
    ],
    resume_hint: {
      target: { type: 'session', id: 'sess_xyz' },
      from_message: 42,
      seed_instruction: 'Wire refresh token to frontend',
      branch_on_resume: false,
    },
    schema_version: 1,
    ...overrides,
  };
}

describe('RecapCard', () => {
  it('renders the headline', () => {
    render(<RecapCard recap={makeRecap()} />);
    expect(screen.getByText('Wired auth refresh-token rotation')).toBeInTheDocument();
  });

  it('renders all bullets', () => {
    render(<RecapCard recap={makeRecap()} />);
    expect(screen.getByText('Ran `cargo test` (3×)')).toBeInTheDocument();
    expect(screen.getByText('Edited src/auth.rs')).toBeInTheDocument();
    expect(screen.getByText('Stopped: rate limit hit')).toBeInTheDocument();
  });

  it('renders next actions under a "Next" label', () => {
    render(<RecapCard recap={makeRecap()} />);
    expect(screen.getByText('Next')).toBeInTheDocument();
    expect(screen.getByText('Wire refresh token to frontend')).toBeInTheDocument();
    expect(screen.getByText('Add expiry e2e test')).toBeInTheDocument();
  });

  it('renders each artifact label and locator', () => {
    render(<RecapCard recap={makeRecap()} />);
    expect(screen.getByText('Artifacts')).toBeInTheDocument();
    expect(screen.getByText('auth.rs')).toBeInTheDocument();
    expect(screen.getByText('src/auth.rs')).toBeInTheDocument();
    expect(screen.getByText('cargo test')).toBeInTheDocument();
    expect(screen.getByText('job_99')).toBeInTheDocument();
  });

  it('shows the heuristic generator badge by default', () => {
    render(<RecapCard recap={makeRecap()} />);
    expect(screen.getByLabelText('Generator')).toHaveTextContent('heuristic');
  });

  it('shows an LLM generator badge with provider and model', () => {
    const recap = makeRecap({
      generator: { type: 'llm', provider: 'anthropic', model: 'claude-opus-4-7' },
    });
    render(<RecapCard recap={recap} />);
    expect(screen.getByLabelText('Generator')).toHaveTextContent('LLM · anthropic/claude-opus-4-7');
  });

  it('shows the user-edited generator badge when applicable', () => {
    const recap = makeRecap({ generator: { type: 'user_edited' } });
    render(<RecapCard recap={recap} />);
    expect(screen.getByLabelText('Generator')).toHaveTextContent('user-edited');
  });

  it('omits the bullets section when bullets is empty', () => {
    render(<RecapCard recap={makeRecap({ bullets: [] })} />);
    expect(screen.queryByText('What happened')).toBeNull();
  });

  it('omits the next-actions section when next_actions is empty', () => {
    render(<RecapCard recap={makeRecap({ next_actions: [] })} />);
    expect(screen.queryByText('Next')).toBeNull();
  });

  it('omits the artifacts section when artifacts is empty', () => {
    render(<RecapCard recap={makeRecap({ artifacts: [] })} />);
    expect(screen.queryByText('Artifacts')).toBeNull();
  });

  it('"Resume from here" fires onResume with the recap', () => {
    const onResume = vi.fn();
    const recap = makeRecap();
    render(<RecapCard recap={recap} onResume={onResume} />);
    fireEvent.click(screen.getByLabelText('Resume from here'));
    expect(onResume).toHaveBeenCalledTimes(1);
    expect(onResume).toHaveBeenCalledWith(recap);
  });

  it('does not throw when "Resume from here" is clicked without an onResume handler', () => {
    render(<RecapCard recap={makeRecap()} />);
    expect(() => fireEvent.click(screen.getByLabelText('Resume from here'))).not.toThrow();
  });

  it('starts open by default and aria-expanded is true', () => {
    render(<RecapCard recap={makeRecap()} />);
    expect(screen.getByLabelText('Toggle recap')).toHaveAttribute('aria-expanded', 'true');
  });

  it('starts collapsed when defaultCollapsed=true; toggle expands it', () => {
    render(<RecapCard recap={makeRecap()} defaultCollapsed />);
    const toggle = screen.getByLabelText('Toggle recap');
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    // Body content (bullets) should be hidden when collapsed
    expect(screen.queryByText('Ran `cargo test` (3×)')).toBeNull();
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('Ran `cargo test` (3×)')).toBeInTheDocument();
  });

  it('hides the Resume button when collapsed', () => {
    render(<RecapCard recap={makeRecap()} defaultCollapsed />);
    expect(screen.queryByLabelText('Resume from here')).toBeNull();
  });
});
