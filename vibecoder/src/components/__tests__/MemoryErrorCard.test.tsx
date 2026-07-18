import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryErrorCard } from '../MemoryErrorCard';

describe('MemoryErrorCard', () => {
  it('renders nothing when error is null', () => {
    const { container } = render(<MemoryErrorCard error={null} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders the raw error message under role=alert', () => {
    render(<MemoryErrorCard error="some failure" />);
    const alert = screen.getByRole('alert');
    expect(alert).toBeInTheDocument();
    expect(screen.getByText('some failure')).toBeInTheDocument();
  });

  it('renders the classified hint card when one applies', () => {
    render(<MemoryErrorCard error="Permission denied (os error 13)" />);
    expect(screen.getByText('Permission denied (os error 13)')).toBeInTheDocument();
    expect(screen.getByTestId('memory-error-hint')).toBeInTheDocument();
    expect(screen.getByTestId('memory-error-hint')).toHaveTextContent(/permissions/i);
  });

  it('omits the hint card for unclassified errors', () => {
    render(<MemoryErrorCard error="Some unique never-before-seen failure" />);
    expect(screen.getByText('Some unique never-before-seen failure')).toBeInTheDocument();
    expect(screen.queryByTestId('memory-error-hint')).toBeNull();
  });

  it('routes disk-full errors to the decay-suggestion hint', () => {
    render(<MemoryErrorCard error="ENOSPC: no space left on device" />);
    expect(screen.getByTestId('memory-error-hint')).toHaveTextContent(/decay/i);
  });

  it('routes corrupt-JSON errors to the restore-or-reset hint', () => {
    render(<MemoryErrorCard error="invalid JSON at line 3 column 5" />);
    expect(screen.getByTestId('memory-error-hint')).toHaveTextContent(/corrupt|backup/i);
  });

  it('routes daemon-unreachable errors to the vibecli-serve hint', () => {
    render(<MemoryErrorCard error="connection refused on 127.0.0.1:7878" />);
    expect(screen.getByTestId('memory-error-hint')).toHaveTextContent(/vibecli serve/);
  });
});
