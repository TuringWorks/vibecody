/**
 * usePanelSettings — React hook for panel settings persistence.
 *
 * Stores/retrieves settings from the encrypted SQLite backend via Tauri commands.
 * Settings are scoped by profile and panel name.
 *
 * Usage:
 *   const { settings, setSetting, loading, profileId } = usePanelSettings("agile");
 *   // settings is a Record<string, any> of all saved settings for this panel
 *   // setSetting("theme", "dark") persists a value
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface UsePanelSettingsResult {
  /** All settings for this panel as a key-value map */
  settings: Record<string, any>;
  /** Whether settings are still loading */
  loading: boolean;
  /** Current profile ID */
  profileId: string;
  /** Set a single setting value (persists immediately) */
  setSetting: (key: string, value: any) => Promise<void>;
  /** Delete a single setting */
  deleteSetting: (key: string) => Promise<void>;
  /** Delete all settings for this panel */
  resetPanel: () => Promise<void>;
  /** Reload settings from the store */
  reload: () => Promise<void>;
  /** Switch to a different profile */
  switchProfile: (newProfileId: string) => void;
  /** Any error from the last operation */
  error: string | null;
}

export function usePanelSettings(panelName: string): UsePanelSettingsResult {
  const [settings, setSettings] = useState<Record<string, any>>({});
  const [loading, setLoading] = useState(true);
  const [profileId, setProfileId] = useState("default");
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);

  // Load default profile on mount
  useEffect(() => {
    mountedRef.current = true;
    (async () => {
      try {
        const defaultId = await invoke<string>("panel_settings_get_default_profile");
        if (mountedRef.current && defaultId) {
          setProfileId(defaultId);
        }
      } catch {
        // Use "default" if command fails
      }
    })();
    return () => { mountedRef.current = false; };
  }, []);

  // Load settings whenever profileId or panelName changes
  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<Record<string, any>>("panel_settings_get_all", {
        profileId,
        panel: panelName,
      });
      if (mountedRef.current) {
        setSettings(data || {});
      }
    } catch (e: any) {
      if (mountedRef.current) {
        setError(typeof e === "string" ? e : e?.message || "Failed to load settings");
        setSettings({});
      }
    } finally {
      if (mountedRef.current) {
        setLoading(false);
      }
    }
  }, [profileId, panelName]);

  useEffect(() => {
    reload();
  }, [reload]);

  const setSetting = useCallback(
    async (key: string, value: any) => {
      setError(null);
      try {
        await invoke("panel_settings_set", {
          profileId,
          panel: panelName,
          key,
          value,
        });
        setSettings((prev) => ({ ...prev, [key]: value }));
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message || "Failed to save setting");
      }
    },
    [profileId, panelName]
  );

  const deleteSetting = useCallback(
    async (key: string) => {
      setError(null);
      try {
        await invoke("panel_settings_delete", {
          profileId,
          panel: panelName,
          key,
        });
        setSettings((prev) => {
          const next = { ...prev };
          delete next[key];
          return next;
        });
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message || "Failed to delete setting");
      }
    },
    [profileId, panelName]
  );

  const resetPanel = useCallback(async () => {
    setError(null);
    try {
      await invoke("panel_settings_delete_panel", {
        profileId,
        panel: panelName,
      });
      setSettings({});
    } catch (e: any) {
      setError(typeof e === "string" ? e : e?.message || "Failed to reset panel");
    }
  }, [profileId, panelName]);

  const switchProfile = useCallback((newProfileId: string) => {
    setProfileId(newProfileId);
  }, []);

  return {
    settings,
    loading,
    profileId,
    setSetting,
    deleteSetting,
    resetPanel,
    reload,
    switchProfile,
    error,
  };
}

// ── Profile management utilities ──

export interface ProfileInfo {
  id: string;
  name: string;
  created_at: string;
  is_default: boolean;
}

export async function listProfiles(): Promise<ProfileInfo[]> {
  return invoke<ProfileInfo[]>("panel_settings_list_profiles");
}

export async function createProfile(id: string, name: string): Promise<void> {
  return invoke("panel_settings_create_profile", { id, name });
}

export async function deleteProfile(id: string): Promise<void> {
  return invoke("panel_settings_delete_profile", { id });
}

export async function setDefaultProfile(id: string): Promise<void> {
  return invoke("panel_settings_set_default_profile", { id });
}

export async function exportProfile(profileId: string): Promise<Record<string, any>> {
  return invoke("panel_settings_export", { profileId });
}

export async function importProfile(profileId: string, data: Record<string, any>): Promise<number> {
  return invoke("panel_settings_import", { profileId, data });
}
