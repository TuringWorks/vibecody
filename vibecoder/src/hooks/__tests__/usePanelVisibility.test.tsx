/**
 * `useVisibleInterval` decides when nineteen panels are allowed to poll, so the
 * cases that matter are the ones that would silently reintroduce the bug it
 * fixes: polling while hidden, and *not* polling while shown.
 */
import { render, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  PanelVisibilityContext,
  useVisibleInterval,
} from "../usePanelVisibility";

function Poller({
  fn,
  ms,
  runOnShow,
}: {
  fn: () => void;
  ms: number | null;
  runOnShow?: boolean;
}) {
  useVisibleInterval(fn, ms, runOnShow === undefined ? {} : { runOnShow });
  return null;
}

/** Render `Poller` inside a visibility context we control. */
function renderIn(visible: boolean, props: Parameters<typeof Poller>[0]) {
  return render(
    <PanelVisibilityContext.Provider value={visible}>
      <Poller {...props} />
    </PanelVisibilityContext.Provider>,
  );
}

describe("useVisibleInterval", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("polls on the interval while visible", () => {
    const fn = vi.fn();
    renderIn(true, { fn, ms: 1000, runOnShow: false });
    expect(fn).not.toHaveBeenCalled();
    act(() => { vi.advanceTimersByTime(3000); });
    expect(fn).toHaveBeenCalledTimes(3);
  });

  it("does not poll at all while the tab is hidden", () => {
    const fn = vi.fn();
    renderIn(false, { fn, ms: 1000 });
    act(() => { vi.advanceTimersByTime(10_000); });
    expect(fn).not.toHaveBeenCalled();
  });

  it("stops polling when the tab becomes hidden", () => {
    const fn = vi.fn();
    const { rerender } = renderIn(true, { fn, ms: 1000, runOnShow: false });
    act(() => { vi.advanceTimersByTime(2000); });
    expect(fn).toHaveBeenCalledTimes(2);

    rerender(
      <PanelVisibilityContext.Provider value={false}>
        <Poller fn={fn} ms={1000} runOnShow={false} />
      </PanelVisibilityContext.Provider>,
    );
    act(() => { vi.advanceTimersByTime(10_000); });
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("refreshes immediately on becoming visible, so shown data is not stale", () => {
    const fn = vi.fn();
    const { rerender } = renderIn(false, { fn, ms: 5000 });
    expect(fn).not.toHaveBeenCalled();

    rerender(
      <PanelVisibilityContext.Provider value={true}>
        <Poller fn={fn} ms={5000} />
      </PanelVisibilityContext.Provider>,
    );
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("runOnShow:false suppresses the catch-up call", () => {
    const fn = vi.fn();
    renderIn(true, { fn, ms: 5000, runOnShow: false });
    expect(fn).not.toHaveBeenCalled();
  });

  it("a null interval disables the poll, which is how callers gate it", () => {
    const fn = vi.fn();
    renderIn(true, { fn, ms: null });
    act(() => { vi.advanceTimersByTime(60_000); });
    expect(fn).not.toHaveBeenCalled();
  });

  it("defaults to visible with no provider, so non-tabbed panels are unchanged", () => {
    const fn = vi.fn();
    render(<Poller fn={fn} ms={1000} runOnShow={false} />);
    act(() => { vi.advanceTimersByTime(2000); });
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("a changing callback identity does not restart the timer", () => {
    // The usual regression: an inline arrow re-created each render resets the
    // interval, and a 5s poll silently becomes a poll on every render.
    const calls: number[] = [];
    function Host({ tick }: { tick: number }) {
      useVisibleInterval(() => calls.push(tick), 1000, { runOnShow: false });
      return null;
    }
    const { rerender } = render(<Host tick={1} />);
    act(() => { vi.advanceTimersByTime(900); });
    rerender(<Host tick={2} />);
    act(() => { vi.advanceTimersByTime(200); });
    // One elapsed second total => exactly one call, using the newest callback.
    expect(calls).toEqual([2]);
  });
});
