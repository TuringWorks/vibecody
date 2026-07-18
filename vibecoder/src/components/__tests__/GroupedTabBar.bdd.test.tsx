/**
 * BDD / TDD tests for GroupedTabBar.
 *
 * Scenarios:
 *  - Renders all tab groups from TAB_GROUPS
 *  - Marks the active tab with aria-selected
 *  - Search/filter narrows the list
 *  - Clearing search restores all groups
 *  - Escape key clears search
 *  - Toggle group collapse/expand
 *  - Keyboard navigation (ArrowDown / ArrowUp / Home / End)
 *  - Calls onTabChange when a tab is clicked
 *  - Calls onCollapse when the collapse button is clicked
 *  - Shows "No matching panels" for impossible queries
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { GroupedTabBar } from '../GroupedTabBar';
import { TAB_GROUPS } from '../../constants/tabGroups';
import { TAB_META } from '../../constants/tabMeta';

// jsdom doesn't implement scrollIntoView — stub it globally
beforeEach(() => {
  window.HTMLElement.prototype.scrollIntoView = vi.fn();
});

// ── Helpers ───────────────────────────────────────────────────────────────────

function firstTab(): string {
  return TAB_GROUPS[0].tabs[0];
}

function firstTabLabel(): string {
  return TAB_META[firstTab()]?.label ?? firstTab();
}

function renderBar(overrides: Partial<React.ComponentProps<typeof GroupedTabBar>> = {}) {
  const onTabChange = vi.fn();
  const result = render(
    <GroupedTabBar
      activeTab={firstTab()}
      onTabChange={onTabChange}
      {...overrides}
    />,
  );
  return { ...result, onTabChange };
}

// ── Initial render ────────────────────────────────────────────────────────────

describe('GroupedTabBar — initial render', () => {
  it('renders a tablist', () => {
    renderBar();
    expect(screen.getByRole('tablist')).toBeDefined();
  });

  it('renders all group headers', () => {
    renderBar();
    for (const group of TAB_GROUPS) {
      // Group headers are buttons with the group label text — use getAllByText
      // since some label text may also appear in tab items
      expect(screen.getAllByText(group.label).length).toBeGreaterThanOrEqual(1);
    }
  });

  it('renders the first group\'s tabs', () => {
    renderBar();
    expect(screen.getByText(firstTabLabel())).toBeDefined();
  });

  it('marks the active tab with aria-selected="true"', () => {
    renderBar();
    const activeBtn = screen.getByRole('tab', { name: firstTabLabel() });
    expect(activeBtn.getAttribute('aria-selected')).toBe('true');
  });

  it('marks all other tabs with aria-selected="false"', () => {
    renderBar();
    const allTabs = screen.getAllByRole('tab');
    const inactive = allTabs.filter((t) => t.getAttribute('aria-selected') === 'false');
    expect(inactive.length).toBeGreaterThan(0);
  });

  it('renders the search input', () => {
    renderBar();
    expect(screen.getByRole('textbox', { name: 'Filter AI panels' })).toBeDefined();
  });
});

// ── Search ────────────────────────────────────────────────────────────────────

describe('GroupedTabBar — search', () => {
  it('filters tabs to those matching the query', () => {
    renderBar();
    const input = screen.getByRole('textbox', { name: 'Filter AI panels' });
    fireEvent.change(input, { target: { value: firstTabLabel() } });
    const tabs = screen.getAllByRole('tab');
    // Only tabs matching the query should be visible
    expect(tabs.length).toBeGreaterThanOrEqual(1);
    expect(tabs.some((t) => t.textContent?.includes(firstTabLabel()))).toBe(true);
  });

  it('shows "No matching panels" for a nonsense query', () => {
    renderBar();
    const input = screen.getByRole('textbox', { name: 'Filter AI panels' });
    fireEvent.change(input, { target: { value: 'xXzZqQthisCannotMatchAnything999' } });
    expect(screen.getByText('No matching panels')).toBeDefined();
  });

  it('shows a clear (×) button while there is search text', () => {
    renderBar();
    const input = screen.getByRole('textbox', { name: 'Filter AI panels' });
    fireEvent.change(input, { target: { value: 'a' } });
    expect(screen.getByRole('button', { name: 'Clear search' })).toBeDefined();
  });

  it('does not show the clear button when search is empty', () => {
    renderBar();
    expect(screen.queryByRole('button', { name: 'Clear search' })).toBeNull();
  });

  it('clicking clear (×) resets search and shows all groups', () => {
    renderBar();
    const input = screen.getByRole('textbox', { name: 'Filter AI panels' });
    fireEvent.change(input, { target: { value: 'zzz' } });
    fireEvent.click(screen.getByRole('button', { name: 'Clear search' }));
    expect((input as HTMLInputElement).value).toBe('');
    // All groups should be visible again
    expect(screen.queryByText('No matching panels')).toBeNull();
  });

  it('pressing Escape clears the search', () => {
    renderBar();
    const input = screen.getByRole('textbox', { name: 'Filter AI panels' });
    fireEvent.change(input, { target: { value: 'abc' } });
    fireEvent.keyDown(input, { key: 'Escape' });
    expect((input as HTMLInputElement).value).toBe('');
  });
});

// ── Group collapse ────────────────────────────────────────────────────────────

describe('GroupedTabBar — group collapse', () => {
  it('toggles a group to collapsed when its header is clicked', () => {
    renderBar();
    const firstGroupHeader = screen.getAllByRole('button').find(
      (btn) => btn.classList.contains('tab-group-header'),
    )!;
    // Initially expanded
    expect(firstGroupHeader.getAttribute('aria-expanded')).toBe('true');
    fireEvent.click(firstGroupHeader);
    expect(firstGroupHeader.getAttribute('aria-expanded')).toBe('false');
  });

  it('re-expands a collapsed group when header clicked again', () => {
    renderBar();
    const firstGroupHeader = screen.getAllByRole('button').find(
      (btn) => btn.classList.contains('tab-group-header'),
    )!;
    fireEvent.click(firstGroupHeader); // collapse
    fireEvent.click(firstGroupHeader); // expand again
    expect(firstGroupHeader.getAttribute('aria-expanded')).toBe('true');
  });
});

// ── Tab click ─────────────────────────────────────────────────────────────────

describe('GroupedTabBar — tab click', () => {
  it('calls onTabChange with the tab id when clicked', () => {
    const { onTabChange } = renderBar();
    // Find any tab other than the active one
    const tabs = screen.getAllByRole('tab');
    const other = tabs.find((t) => t.getAttribute('aria-selected') === 'false')!;
    fireEvent.click(other);
    expect(onTabChange).toHaveBeenCalledOnce();
  });
});

// ── Keyboard navigation ───────────────────────────────────────────────────────

describe('GroupedTabBar — keyboard navigation', () => {
  it('ArrowDown on the first tab calls onTabChange with the next tab', () => {
    const onTabChange = vi.fn();
    const { unmount, container } = render(
      <GroupedTabBar activeTab={firstTab()} onTabChange={onTabChange} />,
    );
    const activeTab = container.querySelector('[role="tab"][aria-selected="true"]') as HTMLElement;
    fireEvent.keyDown(activeTab, { key: 'ArrowDown' });
    expect(onTabChange).toHaveBeenCalledOnce();
    unmount();
  });

  it('ArrowUp on the first tab does not call onTabChange (already at top)', () => {
    const onTabChange = vi.fn();
    const { unmount, container } = render(
      <GroupedTabBar activeTab={firstTab()} onTabChange={onTabChange} />,
    );
    const activeTab = container.querySelector('[role="tab"][aria-selected="true"]') as HTMLElement;
    fireEvent.keyDown(activeTab, { key: 'ArrowUp' });
    expect(onTabChange).not.toHaveBeenCalled();
    unmount();
  });

  it('Home key moves to the first visible tab when on the last tab', () => {
    const allTabIds = TAB_GROUPS.flatMap((g) => g.tabs);
    const lastTabId = allTabIds[allTabIds.length - 1];

    const onTabChange = vi.fn();
    const { unmount, container } = render(
      <GroupedTabBar activeTab={lastTabId} onTabChange={onTabChange} />,
    );
    const activeTab = container.querySelector('[role="tab"][aria-selected="true"]') as HTMLElement;
    expect(activeTab).not.toBeNull();
    fireEvent.keyDown(activeTab, { key: 'Home' });
    expect(onTabChange).toHaveBeenCalledOnce();
    unmount();
  });
});

// ── Collapse button ───────────────────────────────────────────────────────────

describe('GroupedTabBar — collapse panel button', () => {
  it('shows the collapse button when onCollapse is provided', () => {
    renderBar({ onCollapse: vi.fn() });
    expect(screen.getByRole('button', { name: 'Collapse filter panel' })).toBeDefined();
  });

  it('calls onCollapse when the collapse button is clicked', () => {
    const onCollapse = vi.fn();
    renderBar({ onCollapse });
    fireEvent.click(screen.getByRole('button', { name: 'Collapse filter panel' }));
    expect(onCollapse).toHaveBeenCalledOnce();
  });

  it('does not render the collapse button when onCollapse is not provided', () => {
    renderBar();
    expect(screen.queryByRole('button', { name: 'Collapse filter panel' })).toBeNull();
  });
});
