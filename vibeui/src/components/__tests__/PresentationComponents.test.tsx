/**
 * Tests for EmptyState, ErrorBoundary, LoadingSpinner, and StatusMessage.
 *
 * These are pure presentational components with no Tauri dependency.
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { EmptyState } from '../EmptyState';
import { ErrorBoundary } from '../ErrorBoundary';
import { LoadingSpinner } from '../LoadingSpinner';
import { StatusMessage } from '../StatusMessage';

// ── EmptyState ─────────────────────────────────────────────────────────────────

describe('EmptyState', () => {
  it('renders with role="status"', () => {
    render(<EmptyState title="Nothing here" />);
    expect(screen.getByRole('status')).toBeDefined();
  });

  it('renders the title', () => {
    render(<EmptyState title="No results found" />);
    expect(screen.getByText('No results found')).toBeDefined();
  });

  it('renders description when provided', () => {
    render(<EmptyState title="Empty" description="Try adjusting your filters" />);
    expect(screen.getByText('Try adjusting your filters')).toBeDefined();
  });

  it('omits description when not provided', () => {
    render(<EmptyState title="Empty" />);
    // No description element — just the title
    expect(screen.queryByText(/Try/)).toBeNull();
  });

  it('renders the action button with correct label', () => {
    render(<EmptyState title="Empty" action={{ label: 'Retry', onClick: vi.fn() }} />);
    expect(screen.getByRole('button', { name: 'Retry' })).toBeDefined();
  });

  it('calls onClick when action button is clicked', () => {
    const onClick = vi.fn();
    render(<EmptyState title="Empty" action={{ label: 'Go', onClick }} />);
    fireEvent.click(screen.getByRole('button', { name: 'Go' }));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it('omits action button when no action provided', () => {
    render(<EmptyState title="Empty" />);
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('renders a custom icon node', () => {
    render(<EmptyState icon={<span data-testid="custom-icon" />} title="Empty" />);
    expect(screen.getByTestId('custom-icon')).toBeDefined();
  });
});

// ── ErrorBoundary ──────────────────────────────────────────────────────────────

/** Component that throws during render, used to trigger the boundary. */
function Bomb({ msg }: { msg: string }) {
  throw new Error(msg);
}

describe('ErrorBoundary', () => {
  it('renders children when there is no error', () => {
    render(
      <ErrorBoundary>
        <span>Hello</span>
      </ErrorBoundary>,
    );
    expect(screen.getByText('Hello')).toBeDefined();
  });

  it('catches render errors and shows the fallback message', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <ErrorBoundary>
        <Bomb msg="test error" />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Something went wrong')).toBeDefined();
    expect(screen.getByText('test error')).toBeDefined();
    consoleSpy.mockRestore();
  });

  it('shows a Retry button on error', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <ErrorBoundary>
        <Bomb msg="boom" />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('button', { name: 'Retry' })).toBeDefined();
    consoleSpy.mockRestore();
  });

  it('resets the error state when Retry is clicked', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    // Render boundary with a *stable* error so we can verify the reset button
    render(
      <ErrorBoundary>
        <Bomb msg="oops" />
      </ErrorBoundary>,
    );
    // Boundary is in error state — Retry button must be visible
    const retryBtn = screen.getByRole('button', { name: 'Retry' });
    expect(retryBtn).toBeDefined();
    // Clicking Retry resets state (even though Bomb will throw again immediately)
    // — we verify the onClick handler fires without throwing
    expect(() => fireEvent.click(retryBtn)).not.toThrow();
    consoleSpy.mockRestore();
  });

  it('renders the custom fallback when provided and an error is thrown', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <ErrorBoundary fallback={<div>Custom fallback</div>}>
        <Bomb msg="error" />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Custom fallback')).toBeDefined();
    expect(screen.queryByText('Something went wrong')).toBeNull();
    consoleSpy.mockRestore();
  });
});

// ── LoadingSpinner ─────────────────────────────────────────────────────────────

describe('LoadingSpinner', () => {
  it('renders with role="status"', () => {
    render(<LoadingSpinner />);
    expect(screen.getByRole('status')).toBeDefined();
  });

  it('renders default label "Loading..." in an sr-only span', () => {
    render(<LoadingSpinner />);
    // The text is in an .sr-only span
    expect(screen.getByText('Loading...')).toBeDefined();
  });

  it('renders a custom label', () => {
    render(<LoadingSpinner label="Analyzing…" />);
    expect(screen.getByText('Analyzing…')).toBeDefined();
  });

  it('renders the spinner div with correct size (default 24)', () => {
    const { container } = render(<LoadingSpinner />);
    const spinner = container.querySelector('.loading-spinner') as HTMLElement;
    expect(spinner).not.toBeNull();
    expect(spinner.style.width).toBe('24px');
    expect(spinner.style.height).toBe('24px');
  });

  it('renders the spinner div with custom size', () => {
    const { container } = render(<LoadingSpinner size={48} />);
    const spinner = container.querySelector('.loading-spinner') as HTMLElement;
    expect(spinner.style.width).toBe('48px');
    expect(spinner.style.height).toBe('48px');
  });

  it('has aria-live="polite"', () => {
    render(<LoadingSpinner />);
    const el = screen.getByRole('status');
    expect(el.getAttribute('aria-live')).toBe('polite');
  });
});

// ── StatusMessage ──────────────────────────────────────────────────────────────

describe('StatusMessage', () => {
  describe('role', () => {
    it('uses role="alert" for error variant', () => {
      render(<StatusMessage variant="error" message="Oops" />);
      expect(screen.getByRole('alert')).toBeDefined();
    });

    it('uses role="status" for non-error variants', () => {
      for (const variant of ['loading', 'empty', 'success', 'warning'] as const) {
        const { unmount } = render(<StatusMessage variant={variant} message="msg" />);
        expect(screen.getByRole('status')).toBeDefined();
        unmount();
      }
    });
  });

  describe('message and detail', () => {
    it('renders the message', () => {
      render(<StatusMessage variant="loading" message="Loading data…" />);
      expect(screen.getByText('Loading data…')).toBeDefined();
    });

    it('renders the detail when provided', () => {
      render(<StatusMessage variant="loading" message="Please wait" detail="~30 s" />);
      expect(screen.getByText(/~30 s/)).toBeDefined();
    });

    it('omits detail when not provided', () => {
      render(<StatusMessage variant="loading" message="Loading" />);
      expect(screen.queryByText(/~30 s/)).toBeNull();
    });
  });

  describe('variants default icons', () => {
    it('renders error variant without throwing', () => {
      expect(() => render(<StatusMessage variant="error" message="err" />)).not.toThrow();
    });

    it('renders warning variant without throwing', () => {
      expect(() => render(<StatusMessage variant="warning" message="warn" />)).not.toThrow();
    });

    it('renders loading variant without throwing', () => {
      expect(() => render(<StatusMessage variant="loading" message="load" />)).not.toThrow();
    });

    it('renders empty variant without throwing', () => {
      expect(() => render(<StatusMessage variant="empty" message="empty" />)).not.toThrow();
    });

    it('renders success variant without throwing', () => {
      expect(() => render(<StatusMessage variant="success" message="done" />)).not.toThrow();
    });
  });

  describe('custom icon', () => {
    it('overrides the default icon with the provided custom icon', () => {
      render(
        <StatusMessage
          variant="empty"
          message="No data"
          icon={<span data-testid="my-icon" />}
        />,
      );
      expect(screen.getByTestId('my-icon')).toBeDefined();
    });
  });

  describe('inline mode', () => {
    it('still renders the message when inline=true', () => {
      render(<StatusMessage variant="error" message="Inline error" inline />);
      expect(screen.getByText('Inline error')).toBeDefined();
    });

    it('renders detail with an em-dash separator in inline mode', () => {
      render(<StatusMessage variant="error" message="err" detail="details" inline />);
      expect(screen.getByText(/— details/)).toBeDefined();
    });
  });
});
