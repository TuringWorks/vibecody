/**
 * Tests for Icon component.
 *
 * Covers:
 *  - Renders an SVG for every documented IconName
 *  - Applies size, color, strokeWidth, className, style, title, aria-label, aria-hidden
 *  - Returns null and warns for unknown names
 *  - ActivityIcon convenience wrapper
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Icon, ActivityIcon } from '../Icon';
import type { IconName } from '../Icon';

afterEach(() => {
  vi.restoreAllMocks();
});

// ── Rendering ─────────────────────────────────────────────────────────────────

describe('Icon — rendering', () => {
  it('renders an <svg> element', () => {
    const { container } = render(<Icon name="search" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('sets default viewBox "0 0 24 24"', () => {
    const { container } = render(<Icon name="search" />);
    expect(container.querySelector('svg')?.getAttribute('viewBox')).toBe('0 0 24 24');
  });

  it('sets fill="none"', () => {
    const { container } = render(<Icon name="search" />);
    expect(container.querySelector('svg')?.getAttribute('fill')).toBe('none');
  });
});

// ── Size prop ─────────────────────────────────────────────────────────────────

describe('Icon — size prop', () => {
  it('defaults to 16px', () => {
    const { container } = render(<Icon name="search" />);
    const svg = container.querySelector('svg')!;
    expect(svg.getAttribute('width')).toBe('16');
    expect(svg.getAttribute('height')).toBe('16');
  });

  it('applies custom size', () => {
    const { container } = render(<Icon name="search" size={32} />);
    const svg = container.querySelector('svg')!;
    expect(svg.getAttribute('width')).toBe('32');
    expect(svg.getAttribute('height')).toBe('32');
  });
});

// ── Color prop ────────────────────────────────────────────────────────────────

describe('Icon — color prop', () => {
  it('defaults to currentColor', () => {
    const { container } = render(<Icon name="search" />);
    expect(container.querySelector('svg')?.getAttribute('stroke')).toBe('currentColor');
  });

  it('applies custom color', () => {
    const { container } = render(<Icon name="search" color="#ff0000" />);
    expect(container.querySelector('svg')?.getAttribute('stroke')).toBe('#ff0000');
  });
});

// ── className and style ───────────────────────────────────────────────────────

describe('Icon — className and style', () => {
  it('forwards className to <svg>', () => {
    const { container } = render(<Icon name="search" className="my-icon" />);
    expect(container.querySelector('svg')?.classList.contains('my-icon')).toBe(true);
  });

  it('forwards style object to <svg>', () => {
    const { container } = render(<Icon name="search" style={{ opacity: 0.5 }} />);
    expect(container.querySelector('svg')?.style.opacity).toBe('0.5');
  });
});

// ── Accessibility ─────────────────────────────────────────────────────────────

describe('Icon — accessibility', () => {
  it('sets aria-hidden="true" when no aria-label provided', () => {
    const { container } = render(<Icon name="search" />);
    expect(container.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
  });

  it('sets role="img" and aria-label when aria-label provided', () => {
    render(<Icon name="search" aria-label="Search" />);
    const svg = screen.getByRole('img', { name: 'Search' });
    expect(svg).toBeDefined();
  });

  it('renders <title> element when title prop is given', () => {
    const { container } = render(<Icon name="search" title="Search icon" />);
    expect(container.querySelector('title')?.textContent).toBe('Search icon');
  });
});

// ── Unknown name ──────────────────────────────────────────────────────────────

describe('Icon — unknown name', () => {
  it('returns null for an unknown icon name', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const { container } = render(<Icon name={'does-not-exist' as IconName} />);
    expect(container.firstChild).toBeNull();
    warnSpy.mockRestore();
  });

  it('warns with the unknown name', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    render(<Icon name={'bad-icon' as IconName} />);
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('bad-icon'));
    warnSpy.mockRestore();
  });
});

// ── All known icons render without throwing ───────────────────────────────────

describe('Icon — all known names render', () => {
  const knownNames: IconName[] = [
    'files', 'search', 'git-branch', 'test-tube', 'clipboard-list',
    'hammer', 'bot', 'shield', 'terminal', 'settings', 'menu',
    'folder-open', 'folder-plus', 'file-plus', 'save', 'panel-left',
    'message-square', 'play', 'eye', 'eye-off', 'file-text', 'file-code',
    'globe', 'image', 'layout-grid', 'graduation-cap', 'sparkles',
    'rocket', 'plug', 'hand', 'puzzle', 'monitor-play',
    'git-pull-request', 'git-commit', 'users', 'user',
    'container', 'refresh-cw', 'cloud-cog', 'workflow', 'cpu',
    'database', 'radio', 'cog', 'terminal-square', 'wrench',
    'binary', 'regex', 'pen-tool', 'user-cog', 'dollar-sign', 'package',
    'store', 'factory', 'infinity', 'swords', 'users-round', 'brain',
    'ruler', 'palette', 'trending-up', 'activity', 'cloud-upload', 'cpu-chip',
    'atom', 'coffee', 'gem', 'braces', 'archive', 'paintbrush',
    'book-open', 'file', 'image-file', 'code2',
    'x', 'check', 'chevron-right', 'chevron-down', 'chevron-up', 'chevron-left',
    'plus', 'minus', 'external-link', 'copy',
    'trash', 'edit', 'info', 'alert-triangle', 'alert-circle',
    'circle-check', 'loader', 'lock', 'unlock', 'key',
    'moon', 'sun', 'arrow-up', 'arrow-down', 'arrow-right', 'arrow-left',
    'panel-right', 'maximize', 'minimize', 'sidebar', 'layers',
    'folder', 'git-graph', 'sparkle', 'zap', 'send', 'mic',
    'stop-circle', 'pause', 'skip-forward', 'list', 'grid',
    'rotate-ccw', 'download', 'upload', 'link', 'unlink',
    'bell', 'bell-off', 'star', 'heart', 'bookmark',
    'filter', 'sort-asc', 'sort-desc', 'expand', 'compress',
    'split', 'merge', 'diff', 'compass', 'map-pin',
    'network', 'wifi', 'bluetooth', 'usb', 'server',
    'microscope', 'flask', 'chart-bar', 'chart-line', 'pie-chart',
  ];

  for (const name of knownNames) {
    it(`renders "${name}" without throwing`, () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      const { container } = render(<Icon name={name} />);
      expect(container.querySelector('svg')).not.toBeNull();
      expect(warnSpy).not.toHaveBeenCalled();
      warnSpy.mockRestore();
    });
  }
});

// ── ActivityIcon ──────────────────────────────────────────────────────────────

describe('ActivityIcon', () => {
  it('renders an SVG at default size 20', () => {
    const { container } = render(<ActivityIcon name="search" />);
    const svg = container.querySelector('svg')!;
    expect(svg.getAttribute('width')).toBe('20');
    expect(svg.getAttribute('height')).toBe('20');
  });

  it('allows overriding size', () => {
    const { container } = render(<ActivityIcon name="search" size={24} />);
    expect(container.querySelector('svg')?.getAttribute('width')).toBe('24');
  });

  it('forwards aria-label', () => {
    render(<ActivityIcon name="search" aria-label="Search" />);
    expect(screen.getByRole('img', { name: 'Search' })).toBeDefined();
  });
});
