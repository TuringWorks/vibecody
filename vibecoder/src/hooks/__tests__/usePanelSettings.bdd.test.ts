/**
 * BDD tests for usePanelSettings hook and profile management utilities.
 *
 * Scenarios:
 *  - Loading settings on mount
 *  - setSetting persists and updates local state
 *  - deleteSetting removes from local state
 *  - resetPanel clears all settings
 *  - switchProfile triggers a reload
 *  - Error handling for each operation
 *  - Profile utility functions
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import {
  usePanelSettings,
  listProfiles,
  createProfile,
  deleteProfile,
  setDefaultProfile,
  exportProfile,
  importProfile,
} from '../usePanelSettings';

// ── Mock @tauri-apps/api/core ─────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Reset and configure the mock for the standard happy-path. */
function setupHappyPath(savedSettings: Record<string, unknown> = {}) {
  mockInvoke.mockReset();
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'panel_settings_get_default_profile') return 'default';
    if (cmd === 'panel_settings_get_all') return savedSettings;
    return undefined;
  });
}

// ── beforeEach / afterEach ────────────────────────────────────────────────────

beforeEach(() => {
  mockInvoke.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

// ── Loading on mount ──────────────────────────────────────────────────────────

describe('Given the panel mounts', () => {
  it('When the store returns settings, Then loading becomes false and settings are populated', async () => {
    setupHappyPath({ theme: 'dark', fontSize: 14 });

    const { result } = renderHook(() => usePanelSettings('agile'));

    // Initially loading
    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.settings).toEqual({ theme: 'dark', fontSize: 14 });
  });

  it('When the store returns an empty object, Then settings is {}', async () => {
    setupHappyPath({});

    const { result } = renderHook(() => usePanelSettings('empty-panel'));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.settings).toEqual({});
  });

  it('When panel_settings_get_all fails, Then error is set and settings is {}', async () => {
    mockInvoke.mockReset();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'panel_settings_get_default_profile') return 'default';
      if (cmd === 'panel_settings_get_all') throw new Error('DB error');
      return undefined;
    });

    const { result } = renderHook(() => usePanelSettings('bad-panel'));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.settings).toEqual({});
    expect(result.current.error).toBeTruthy();
  });

  it('profileId defaults to "default"', async () => {
    setupHappyPath();

    const { result } = renderHook(() => usePanelSettings('myPanel'));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.profileId).toBe('default');
  });

  it('calls panel_settings_get_all with the correct panel name', async () => {
    setupHappyPath();

    const { result } = renderHook(() => usePanelSettings('cost-panel'));

    await waitFor(() => expect(result.current.loading).toBe(false));

    const getAllCall = mockInvoke.mock.calls.find(([cmd]) => cmd === 'panel_settings_get_all');
    expect(getAllCall).toBeDefined();
    expect(getAllCall![1]).toMatchObject({ panel: 'cost-panel' });
  });
});

// ── setSetting ────────────────────────────────────────────────────────────────

describe('Given the panel is loaded', () => {
  describe('When setSetting is called', () => {
    it('Then it invokes panel_settings_set with correct args', async () => {
      setupHappyPath({ existing: 'value' });
      mockInvoke.mockImplementation(async (cmd: string, _args?: Record<string, unknown>) => {
        if (cmd === 'panel_settings_get_default_profile') return 'default';
        if (cmd === 'panel_settings_get_all') return { existing: 'value' };
        if (cmd === 'panel_settings_set') return undefined;
        return undefined;
      });

      const { result } = renderHook(() => usePanelSettings('settings-panel'));
      await waitFor(() => expect(result.current.loading).toBe(false));

      await act(async () => {
        await result.current.setSetting('myKey', 'myValue');
      });

      const setCall = mockInvoke.mock.calls.find(([cmd]) => cmd === 'panel_settings_set');
      expect(setCall).toBeDefined();
      expect(setCall![1]).toMatchObject({ key: 'myKey', value: 'myValue' });
    });

    it('Then the local settings state is updated immediately', async () => {
      setupHappyPath({ a: 1 });
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'panel_settings_get_default_profile') return 'default';
        if (cmd === 'panel_settings_get_all') return { a: 1 };
        if (cmd === 'panel_settings_set') return undefined;
        return undefined;
      });

      const { result } = renderHook(() => usePanelSettings('test'));
      await waitFor(() => expect(result.current.loading).toBe(false));

      await act(async () => {
        await result.current.setSetting('b', 2);
      });

      expect(result.current.settings).toEqual({ a: 1, b: 2 });
    });

    it('Then if invoke throws, error is set', async () => {
      setupHappyPath({});
      mockInvoke.mockImplementation(async (cmd: string) => {
        if (cmd === 'panel_settings_get_default_profile') return 'default';
        if (cmd === 'panel_settings_get_all') return {};
        if (cmd === 'panel_settings_set') throw new Error('Write failed');
        return undefined;
      });

      const { result } = renderHook(() => usePanelSettings('err-panel'));
      await waitFor(() => expect(result.current.loading).toBe(false));

      await act(async () => {
        await result.current.setSetting('key', 'val');
      });

      expect(result.current.error).toBeTruthy();
    });
  });
});

// ── deleteSetting ─────────────────────────────────────────────────────────────

describe('deleteSetting', () => {
  it('invokes panel_settings_delete and removes key from local state', async () => {
    setupHappyPath({ foo: 'bar', keep: 'me' });
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'panel_settings_get_default_profile') return 'default';
      if (cmd === 'panel_settings_get_all') return { foo: 'bar', keep: 'me' };
      if (cmd === 'panel_settings_delete') return undefined;
      return undefined;
    });

    const { result } = renderHook(() => usePanelSettings('test'));
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.deleteSetting('foo');
    });

    expect(result.current.settings).toEqual({ keep: 'me' });
    expect(result.current.settings).not.toHaveProperty('foo');
  });

  it('sets error when invoke fails', async () => {
    setupHappyPath({ x: 1 });
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'panel_settings_get_default_profile') return 'default';
      if (cmd === 'panel_settings_get_all') return { x: 1 };
      if (cmd === 'panel_settings_delete') throw 'delete failed';
      return undefined;
    });

    const { result } = renderHook(() => usePanelSettings('del-test'));
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.deleteSetting('x');
    });

    expect(result.current.error).toBeTruthy();
  });
});

// ── resetPanel ────────────────────────────────────────────────────────────────

describe('resetPanel', () => {
  it('invokes panel_settings_delete_panel and clears settings', async () => {
    setupHappyPath({ a: 1, b: 2 });
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'panel_settings_get_default_profile') return 'default';
      if (cmd === 'panel_settings_get_all') return { a: 1, b: 2 };
      if (cmd === 'panel_settings_delete_panel') return undefined;
      return undefined;
    });

    const { result } = renderHook(() => usePanelSettings('reset-test'));
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.resetPanel();
    });

    expect(result.current.settings).toEqual({});

    const resetCall = mockInvoke.mock.calls.find(([cmd]) => cmd === 'panel_settings_delete_panel');
    expect(resetCall).toBeDefined();
  });

  it('sets error when invoke fails', async () => {
    setupHappyPath({ a: 1 });
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'panel_settings_get_default_profile') return 'default';
      if (cmd === 'panel_settings_get_all') return { a: 1 };
      if (cmd === 'panel_settings_delete_panel') throw new Error('reset failed');
      return undefined;
    });

    const { result } = renderHook(() => usePanelSettings('err-reset'));
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.resetPanel();
    });

    expect(result.current.error).toBeTruthy();
  });
});

// ── switchProfile ─────────────────────────────────────────────────────────────

describe('switchProfile', () => {
  it('updates profileId and triggers a reload', async () => {
    let callCount = 0;
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'panel_settings_get_default_profile') return 'default';
      if (cmd === 'panel_settings_get_all') {
        callCount++;
        return {};
      }
      return undefined;
    });

    const { result } = renderHook(() => usePanelSettings('profile-test'));
    await waitFor(() => expect(result.current.loading).toBe(false));
    const initialCount = callCount;

    act(() => {
      result.current.switchProfile('work');
    });

    await waitFor(() => expect(result.current.profileId).toBe('work'));
    // Reload should have been triggered
    await waitFor(() => expect(callCount).toBeGreaterThan(initialCount));
  });
});

// ── reload ────────────────────────────────────────────────────────────────────

describe('reload', () => {
  it('re-fetches settings from the store', async () => {
    let callCount = 0;
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'panel_settings_get_default_profile') return 'default';
      if (cmd === 'panel_settings_get_all') {
        callCount++;
        return {};
      }
      return undefined;
    });

    const { result } = renderHook(() => usePanelSettings('reload-test'));
    await waitFor(() => expect(result.current.loading).toBe(false));
    const before = callCount;

    await act(async () => {
      await result.current.reload();
    });

    expect(callCount).toBeGreaterThan(before);
  });
});

// ── Profile management utilities ──────────────────────────────────────────────

describe('Profile management utilities', () => {
  describe('listProfiles', () => {
    it('calls panel_settings_list_profiles and returns result', async () => {
      const profiles = [{ id: 'default', name: 'Default', created_at: '2024-01-01', is_default: true }];
      mockInvoke.mockResolvedValueOnce(profiles);

      const result = await listProfiles();

      expect(mockInvoke).toHaveBeenCalledWith('panel_settings_list_profiles');
      expect(result).toEqual(profiles);
    });
  });

  describe('createProfile', () => {
    it('calls panel_settings_create_profile with id and name', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await createProfile('work', 'Work Profile');

      expect(mockInvoke).toHaveBeenCalledWith('panel_settings_create_profile', { id: 'work', name: 'Work Profile' });
    });
  });

  describe('deleteProfile', () => {
    it('calls panel_settings_delete_profile with id', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await deleteProfile('old-profile');

      expect(mockInvoke).toHaveBeenCalledWith('panel_settings_delete_profile', { id: 'old-profile' });
    });
  });

  describe('setDefaultProfile', () => {
    it('calls panel_settings_set_default_profile with id', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await setDefaultProfile('work');

      expect(mockInvoke).toHaveBeenCalledWith('panel_settings_set_default_profile', { id: 'work' });
    });
  });

  describe('exportProfile', () => {
    it('calls panel_settings_export and returns the data', async () => {
      const exportData = { panel1: { key: 'val' } };
      mockInvoke.mockResolvedValueOnce(exportData);

      const result = await exportProfile('default');

      expect(mockInvoke).toHaveBeenCalledWith('panel_settings_export', { profileId: 'default' });
      expect(result).toEqual(exportData);
    });
  });

  describe('importProfile', () => {
    it('calls panel_settings_import with profileId and data, returns count', async () => {
      mockInvoke.mockResolvedValueOnce(42);

      const count = await importProfile('default', { panel1: { key: 'val' } });

      expect(mockInvoke).toHaveBeenCalledWith('panel_settings_import', {
        profileId: 'default',
        data: { panel1: { key: 'val' } },
      });
      expect(count).toBe(42);
    });
  });
});
