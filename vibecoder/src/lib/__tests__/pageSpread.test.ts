/**
 * The arithmetic of a two-page spread. Cheap to get subtly wrong, and wrong in
 * ways a screenshot does not show: a "next" that moves one page in a two-page
 * view, or a last spread that claims a page the document does not have.
 */
import { describe, it, expect } from 'vitest';

import {
  canTurn,
  clampPage,
  fitScale,
  pagesInView,
  turn,
  viewLabel,
  viewStart,
} from '../pageSpread';

describe('pagesInView', () => {
  it('shows one page at a time on its own', () => {
    expect(pagesInView(1, 10, 'single')).toEqual([1]);
    expect(pagesInView(7, 10, 'single')).toEqual([7]);
  });

  it('pairs from the front', () => {
    expect(pagesInView(1, 10, 'spread')).toEqual([1, 2]);
    expect(pagesInView(2, 10, 'spread')).toEqual([1, 2]);
    expect(pagesInView(3, 10, 'spread')).toEqual([3, 4]);
  });

  it('does not invent a page to fill the last spread', () => {
    expect(pagesInView(5, 5, 'spread')).toEqual([5]);
    expect(pagesInView(1, 1, 'spread')).toEqual([1]);
  });

  it('has nothing to show for a document with no pages', () => {
    expect(pagesInView(1, 0, 'spread')).toEqual([]);
    expect(viewLabel(1, 0, 'spread')).toBe('No pages');
  });
});

describe('turning a page', () => {
  it('moves by a whole view, not by one page', () => {
    expect(turn(1, 10, 'spread', 1)).toBe(3);
    expect(turn(4, 10, 'spread', -1)).toBe(1);
    expect(turn(4, 10, 'single', 1)).toBe(5);
  });

  it('stops at both ends', () => {
    expect(turn(1, 10, 'spread', -1)).toBe(1);
    expect(turn(9, 10, 'spread', 1)).toBe(9);
    expect(canTurn(1, 10, 'spread', -1)).toBe(false);
    expect(canTurn(9, 10, 'spread', 1)).toBe(false);
    expect(canTurn(9, 10, 'spread', -1)).toBe(true);
  });

  it('will not turn onto a half-empty last spread that is already showing', () => {
    // Pages 5 of 5 is the last view; there is no sixth page to turn to.
    expect(canTurn(5, 5, 'spread', 1)).toBe(false);
    expect(canTurn(3, 5, 'spread', 1)).toBe(true);
  });
});

describe('viewLabel', () => {
  it('names what is actually on screen', () => {
    expect(viewLabel(7, 120, 'single')).toBe('Page 7 of 120');
    expect(viewLabel(7, 120, 'spread')).toBe('Pages 7–8 of 120');
    expect(viewLabel(119, 120, 'spread')).toBe('Pages 119–120 of 120');
    // An odd-length document ends on a single page, and says so.
    expect(viewLabel(121, 121, 'spread')).toBe('Page 121 of 121');
  });
});

describe('clamping', () => {
  it('keeps a page inside the document', () => {
    expect(clampPage(0, 10)).toBe(1);
    expect(clampPage(99, 10)).toBe(10);
    expect(clampPage(Number.NaN, 10)).toBe(1);
    expect(clampPage(3.7, 10)).toBe(3);
  });

  it('leaves an empty document on page one rather than page zero', () => {
    expect(clampPage(1, 0)).toBe(1);
  });
});

describe('switching layout', () => {
  it('keeps you on the page you were reading', () => {
    // Reading page 8 one-up, then switching to a spread, shows 7–8 — not 8–9,
    // which would be a spread no printed book has.
    expect(viewStart(8, 'spread')).toBe(7);
    expect(pagesInView(viewStart(8, 'spread'), 120, 'spread')).toEqual([7, 8]);
    expect(viewStart(8, 'single')).toBe(8);
  });
});

describe('fitScale', () => {
  const page = { width: 612, height: 792 };

  it('fits one page to the shorter of the two dimensions', () => {
    // 900 tall pane, 792pt page: height decides, not width.
    const scale = fitScale({ width: 1400, height: 900 }, page, 1);
    // Two gutters and the strip that carries the page number.
    expect(scale).toBeCloseTo((900 - 48 - 22) / 792, 5);
  });

  it('fits a spread to two pages and the gutters between them', () => {
    const scale = fitScale({ width: 1400, height: 2000 }, page, 2);
    // Three gutters: outside, middle, outside.
    expect(scale).toBeCloseTo((1400 - 24 * 3) / 2 / 612, 5);
  });

  it('is smaller for two pages than for one when width is what binds', () => {
    const wide = { width: 1400, height: 2000 };
    expect(fitScale(wide, page, 2)).toBeLessThan(fitScale(wide, page, 1));
    // In a short window the height decides for both, and a spread costs
    // nothing — which is exactly when a spread is worth having.
    const short = { width: 1400, height: 900 };
    expect(fitScale(short, page, 2)).toBe(fitScale(short, page, 1));
  });

  it('does not divide by a page that has no size', () => {
    expect(fitScale({ width: 1400, height: 900 }, { width: 0, height: 0 }, 1)).toBe(1);
    expect(fitScale({ width: 0, height: 0 }, page, 1)).toBe(0.1);
  });
});
