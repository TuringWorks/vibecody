import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Settings key holding the JSON array of explicitly-added project paths. */
const PROJECTS_KEY = "vibex.projects";

/**
 * Persisted list of project folders the user has added via "New project"
 * (VX bug-1). The Projects rail is otherwise derived purely from live tasks, so
 * a freshly-picked project with no chats would be invisible and would vanish on
 * restart. This hook stores the picked paths in the encrypted settings DB
 * (`setting_get`/`setting_set`) so empty projects appear immediately and
 * survive relaunch. The rail renders the union of these paths and task paths.
 */
export function useProjects() {
  const [projectPaths, setProjectPaths] = useState<string[]>([]);

  // Load the persisted list on mount.
  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const raw = await invoke<string | null>("setting_get", { key: PROJECTS_KEY });
        if (!alive || !raw) return;
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          setProjectPaths(parsed.filter((p): p is string => typeof p === "string"));
        }
      } catch (e) {
        console.error("load projects failed", e);
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  const persist = useCallback((paths: string[]) => {
    invoke("setting_set", { key: PROJECTS_KEY, value: JSON.stringify(paths) }).catch((e) =>
      console.error("persist projects failed", e)
    );
  }, []);

  /** Add a project path (dedup) and persist. No-op if already present. */
  const addProject = useCallback(
    (path: string) => {
      setProjectPaths((prev) => {
        if (!path || prev.includes(path)) return prev;
        const next = [...prev, path];
        persist(next);
        return next;
      });
    },
    [persist]
  );

  /** Forget a project path and persist (does not touch its tasks/worktrees). */
  const removeProject = useCallback(
    (path: string) => {
      setProjectPaths((prev) => {
        if (!prev.includes(path)) return prev;
        const next = prev.filter((p) => p !== path);
        persist(next);
        return next;
      });
    },
    [persist]
  );

  return { projectPaths, addProject, removeProject };
}
