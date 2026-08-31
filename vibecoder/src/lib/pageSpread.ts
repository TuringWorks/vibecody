/**
 * pageSpread — which pages sit on screen together, and where turning a page
 * lands.
 *
 * Kept apart from the viewers because it is the part that is easy to get subtly
 * wrong — off by one at the end of an odd-length document, a "next" that moves
 * one page in a two-page view — and the part that is worth testing without a
 * renderer in the way.
 *
 * Spreads pair from the front: (1, 2), (3, 4), … A book whose cover should sit
 * alone would pair (2, 3) instead, but which convention a PDF wants is not
 * something the file says, and guessing it would move every page number in the
 * toolbar away from the one printed on the page.
 */

/** One page at a time, or two side by side. */
export type Layout = "single" | "spread";

/** How many pages a layout shows at once. */
export function pagesPerView(layout: Layout): number {
  return layout === "spread" ? 2 : 1;
}

/** The first page of the view that `page` belongs to. */
export function viewStart(page: number, layout: Layout): number {
  const per = pagesPerView(layout);
  return page - ((page - 1) % per);
}

/**
 * The pages on screen, in order.
 *
 * The last view of an odd-length document holds one page, not a blank: a
 * placeholder would look like a page the document does not have.
 */
export function pagesInView(page: number, count: number, layout: Layout): number[] {
  if (count < 1) return [];
  const start = viewStart(clampPage(page, count), layout);
  return Array.from({ length: pagesPerView(layout) }, (_, i) => start + i).filter(
    (n) => n <= count,
  );
}

/**
 * Where the previous/next control lands.
 *
 * Always the first page of a view, and never past the last one — clamping to
 * the page count instead would leave the page number sitting in the middle of
 * the view it is already showing.
 */
export function turn(
  page: number,
  count: number,
  layout: Layout,
  direction: 1 | -1,
): number {
  const start = viewStart(clampPage(page, count), layout);
  const next = start + direction * pagesPerView(layout);
  if (next < 1) return 1;
  return next > count ? start : next;
}

/** Whether there is anything before / after the current view. */
export function canTurn(
  page: number,
  count: number,
  layout: Layout,
  direction: 1 | -1,
): boolean {
  const start = viewStart(clampPage(page, count), layout);
  return direction === -1 ? start > 1 : start + pagesPerView(layout) <= count;
}

/** "Page 7 of 120", or "Pages 7–8 of 120" for a spread. */
export function viewLabel(page: number, count: number, layout: Layout): string {
  const pages = pagesInView(page, count, layout);
  if (pages.length === 0) return "No pages";
  if (pages.length === 1) return `Page ${pages[0]} of ${count}`;
  return `Pages ${pages[0]}–${pages[pages.length - 1]} of ${count}`;
}

/** Keep a page number inside the document. */
export function clampPage(page: number, count: number): number {
  if (!Number.isFinite(page)) return 1;
  return Math.min(Math.max(Math.trunc(page), 1), Math.max(count, 1));
}

/** The gap around and between pages of a spread, in CSS pixels. */
export const GUTTER = 24;

/**
 * The scale at which the pages on screen fit the pane they are drawn in.
 *
 * A spread at 100% is two half-pages: a page is taller than any window at its
 * own size, and two of them are wider than most. Fitting is what the view opens
 * at, and what "Fit" goes back to — the zoom controls are for looking closer,
 * not for making the document usable in the first place.
 */
export function fitScale(
  pane: { width: number; height: number },
  page: { width: number; height: number },
  pagesOnScreen: number,
): number {
  const columns = Math.max(pagesOnScreen, 1);
  if (page.width <= 0 || page.height <= 0) return 1;
  const available = pane.width - GUTTER * (columns + 1);
  const width = available / columns / page.width;
  const height = (pane.height - GUTTER * 2) / page.height;
  const fit = Math.min(width, height);
  return Number.isFinite(fit) ? Math.min(Math.max(fit, 0.1), 5) : 1;
}
