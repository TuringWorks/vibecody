/**
 * BDD tests for useNotifications — centralized notification state.
 *
 * Scenarios:
 *  1. Adding a notification inserts it at the front (most-recent-first)
 *  2. Notifications start as unread; unreadCount reflects this
 *  3. markRead sets read=true for the specified id
 *  4. markAllRead sets read=true for every notification
 *  5. dismiss removes a notification by id
 *  6. clearCategory removes all notifications matching that category
 *  7. MAX_NOTIFICATIONS (100) cap is enforced
 *  8. Action payload is preserved on add
 *  9. add() returns the created notification object
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useNotifications } from '../useNotifications';
import type { AddNotificationOpts } from '../useNotifications';

// ── Fixture helpers ───────────────────────────────────────────────────────────

function apiKeyNotif(overrides: Partial<AddNotificationOpts> = {}): AddNotificationOpts {
  return {
    title: 'OpenAI key invalid',
    body: 'HTTP 401 Unauthorized',
    severity: 'error',
    category: 'api-keys',
    ...overrides,
  };
}

function systemNotif(overrides: Partial<AddNotificationOpts> = {}): AddNotificationOpts {
  return {
    title: 'Workspace loaded',
    body: '/home/user/project',
    severity: 'info',
    category: 'system',
    ...overrides,
  };
}

beforeEach(() => vi.clearAllMocks());

// ── Scenario 1: Most-recent-first ordering ────────────────────────────────────

describe('Given a fresh notifications queue', () => {
  it('When a notification is added, Then it appears in the list', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => { result.current.add(apiKeyNotif()); });
    expect(result.current.notifications).toHaveLength(1);
    expect(result.current.notifications[0].title).toBe('OpenAI key invalid');
  });

  it('When two notifications are added, Then the most recent is first', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      result.current.add(systemNotif({ title: 'First' }));
      result.current.add(systemNotif({ title: 'Second' }));
    });
    expect(result.current.notifications[0].title).toBe('Second');
    expect(result.current.notifications[1].title).toBe('First');
  });
});

// ── Scenario 2: Unread count ──────────────────────────────────────────────────

describe('Given three notifications are added', () => {
  it('When none are read, Then unreadCount equals 3', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      result.current.add(apiKeyNotif());
      result.current.add(apiKeyNotif());
      result.current.add(systemNotif());
    });
    expect(result.current.unreadCount).toBe(3);
  });

  it('When the queue is empty, Then unreadCount is 0', () => {
    const { result } = renderHook(() => useNotifications());
    expect(result.current.unreadCount).toBe(0);
  });
});

// ── Scenario 3: markRead ──────────────────────────────────────────────────────

describe('Given two unread notifications', () => {
  it('When markRead is called for the first, Then only that notification has read=true', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      result.current.add(apiKeyNotif({ title: 'A' }));
      result.current.add(apiKeyNotif({ title: 'B' }));
    });
    // notifications are newest-first: [B, A]
    const idA = result.current.notifications[1].id;
    act(() => { result.current.markRead(idA); });

    expect(result.current.notifications[1].read).toBe(true);
    expect(result.current.notifications[0].read).toBe(false);
    expect(result.current.unreadCount).toBe(1);
  });

  it('When markRead is called with an unknown id, Then nothing changes', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => { result.current.add(systemNotif()); });
    act(() => { result.current.markRead(99999); });
    expect(result.current.notifications[0].read).toBe(false);
  });
});

// ── Scenario 4: markAllRead ───────────────────────────────────────────────────

describe('Given 5 unread notifications', () => {
  it('When markAllRead is called, Then unreadCount becomes 0', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      for (let i = 0; i < 5; i++) {
        result.current.add(systemNotif({ title: `Notification ${i}` }));
      }
    });
    expect(result.current.unreadCount).toBe(5);
    act(() => { result.current.markAllRead(); });
    expect(result.current.unreadCount).toBe(0);
  });

  it('When markAllRead is called, Then every notification has read=true', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      result.current.add(apiKeyNotif());
      result.current.add(systemNotif());
    });
    act(() => { result.current.markAllRead(); });
    expect(result.current.notifications.every(n => n.read)).toBe(true);
  });
});

// ── Scenario 5: dismiss ───────────────────────────────────────────────────────

describe('Given a notification in the queue', () => {
  it('When dismiss(id) is called, Then that notification is removed', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      result.current.add(apiKeyNotif({ title: 'Removable' }));
      result.current.add(systemNotif({ title: 'Keeper' }));
    });
    const removableId = result.current.notifications[1].id; // oldest (added first) is at index 1
    act(() => { result.current.dismiss(removableId); });
    expect(result.current.notifications).toHaveLength(1);
    expect(result.current.notifications[0].title).toBe('Keeper');
  });

  it('When dismiss is called with an unknown id, Then the queue is unchanged', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => { result.current.add(systemNotif()); });
    act(() => { result.current.dismiss(99999); });
    expect(result.current.notifications).toHaveLength(1);
  });
});

// ── Scenario 6: clearCategory ─────────────────────────────────────────────────

describe('Given a mix of api-keys and system notifications', () => {
  it('When clearCategory("api-keys") is called, Then only api-keys notifications are removed', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      result.current.add(apiKeyNotif({ title: 'Key 1' }));
      result.current.add(apiKeyNotif({ title: 'Key 2' }));
      result.current.add(systemNotif({ title: 'System' }));
    });
    act(() => { result.current.clearCategory('api-keys'); });
    expect(result.current.notifications).toHaveLength(1);
    expect(result.current.notifications[0].category).toBe('system');
  });

  it('When clearCategory is called for a category with no matches, Then nothing changes', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => { result.current.add(apiKeyNotif()); });
    act(() => { result.current.clearCategory('build'); });
    expect(result.current.notifications).toHaveLength(1);
  });
});

// ── Scenario 7: MAX_NOTIFICATIONS cap ────────────────────────────────────────

describe('Given 101 notifications are added', () => {
  it('When the 101st is added, Then the queue is capped at 100 entries', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      for (let i = 0; i < 101; i++) {
        result.current.add(systemNotif({ title: `N${i}` }));
      }
    });
    expect(result.current.notifications).toHaveLength(100);
  });

  it('When capped, Then the most recent 100 entries are retained (oldest dropped)', () => {
    const { result } = renderHook(() => useNotifications());
    act(() => {
      for (let i = 0; i < 101; i++) {
        result.current.add(systemNotif({ title: `N${i}` }));
      }
    });
    // Newest is N100 (first in the list), oldest N0 should be gone
    expect(result.current.notifications[0].title).toBe('N100');
    expect(result.current.notifications.find(n => n.title === 'N0')).toBeUndefined();
  });
});

// ── Scenario 8: Action payload ────────────────────────────────────────────────

describe('Given a notification added with an action', () => {
  it('When the notification is retrieved, Then its action label and onClick are preserved', () => {
    const { result } = renderHook(() => useNotifications());
    const onClick = vi.fn();
    act(() => {
      result.current.add({
        ...apiKeyNotif(),
        action: { label: 'Open Settings', onClick },
      });
    });
    const notif = result.current.notifications[0];
    expect(notif.action?.label).toBe('Open Settings');
    notif.action?.onClick();
    expect(onClick).toHaveBeenCalledOnce();
  });
});

// ── Scenario 9: add() return value ───────────────────────────────────────────

describe('Given add() is called', () => {
  it('Then it returns the AppNotification object with an assigned id and timestamp', () => {
    const { result } = renderHook(() => useNotifications());
    let returned: ReturnType<typeof result.current.add>;
    act(() => {
      returned = result.current.add(systemNotif());
    });
    expect(returned!.id).toBeGreaterThan(0);
    expect(returned!.read).toBe(false);
    expect(returned!.timestamp).toBeGreaterThan(0);
  });
});
