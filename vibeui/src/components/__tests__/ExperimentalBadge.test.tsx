import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ExperimentalBadge } from '../ExperimentalBadge';

describe('ExperimentalBadge', () => {
  it('renders the visible "Experimental" label as an inline pill by default', () => {
    render(<ExperimentalBadge feature="Test feature" />);
    expect(screen.getByText('Experimental')).toBeInTheDocument();
  });

  it('exposes the feature name in the screen-reader label', () => {
    render(<ExperimentalBadge feature="RL-OS dashboard" />);
    const note = screen.getByRole('note');
    expect(note).toHaveAttribute('aria-label', 'Experimental — RL-OS dashboard may change or break.');
  });

  it('falls back to a generic SR label when no feature is given', () => {
    render(<ExperimentalBadge />);
    const note = screen.getByRole('note');
    expect(note.getAttribute('aria-label')).toMatch(/Experimental/);
  });

  it('honors a custom tooltip', () => {
    render(<ExperimentalBadge tooltip="Stub backend, off by default." />);
    const note = screen.getByRole('note');
    expect(note).toHaveAttribute('title', 'Stub backend, off by default.');
  });

  it('renders the banner shape when as="banner"', () => {
    render(<ExperimentalBadge feature="Voice" tooltip="Local voice models are experimental." as="banner" />);
    // Banner shows the inline-pill text + the tooltip text inline.
    expect(screen.getByText('Experimental')).toBeInTheDocument();
    expect(screen.getByText(/Local voice models are experimental/i)).toBeInTheDocument();
  });

  it('renders trailing children (e.g. a help link) at the end', () => {
    render(
      <ExperimentalBadge>
        <span data-testid="badge-trailing"> · learn more</span>
      </ExperimentalBadge>
    );
    expect(screen.getByTestId('badge-trailing')).toBeInTheDocument();
  });
});
