import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MiddleTruncate } from '../MiddleTruncate';

describe('MiddleTruncate', () => {
  it('keeps the whole string readable to a screen reader', () => {
    // The ellipsis is CSS. Splitting the text must not lose any of it, or the
    // accessible name becomes a different path from the one on screen.
    const path = 'docs/content/ai/AI_FEATURES_SUMMARY.md';
    const { container } = render(<MiddleTruncate text={path} />);
    expect(container.textContent).toBe(path);
  });

  it('puts the end of the path in a span that cannot shrink', () => {
    // This is the whole mechanism: the head ellipsises, the tail survives. If
    // the tail could shrink, this would be an ordinary end-truncation with
    // extra steps.
    const { container } = render(
      <MiddleTruncate text="docs/PYTHON_UDF_COMPLETE_DOCUMENTATION.md" tail={12} />,
    );
    const spans = container.querySelectorAll('span > span');
    expect(spans).toHaveLength(2);
    expect(spans[1]).toHaveTextContent('CUMENTATION.md'.slice(-12));
    expect(spans[1]).toHaveStyle({ flex: 'none' });
    expect(spans[0]).toHaveStyle({ textOverflow: 'ellipsis' });
  });

  it('does not split a string short enough to fit', () => {
    // "README.md" split into head + tail would show an ellipsis in front of
    // text that was never truncated.
    const { container } = render(<MiddleTruncate text="README.md" tail={12} />);
    expect(container.querySelectorAll('span > span')).toHaveLength(1);
  });

  it('exposes the full path on hover for the truncated case', () => {
    const path = 'docs/content/ai/AI_FEATURES_SUMMARY.md';
    render(<MiddleTruncate text={path} />);
    expect(screen.getByTitle(path)).toBeInTheDocument();
  });

  it('can shrink below its content width', () => {
    // `min-width: 0` is the load-bearing property. A flex item defaults to
    // `min-width: auto`, so without this the long path refuses to shrink and
    // pushes the controls beside it out of the row instead of truncating.
    const { container } = render(<MiddleTruncate text={'a'.repeat(200)} />);
    expect(container.firstChild).toHaveStyle({ minWidth: '0px' });
  });
});
