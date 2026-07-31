import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// The @-mention parsing lives in `src/lib/composerParsing.ts` — free of React
// and Tauri, so it can be unit-tested under plain node. Re-exported here so
// callers have one import for the mention feature.
export { findMention, rankFiles, type MentionQuery } from "../lib/composerParsing";

/**
 * The project's tracked file paths, for @-mention completion in the composer.
 *
 * Sourced from the daemon's `git ls-files` view (gitignore-correct) for the
 * scoped repo. Fetched once per project rather than per keystroke: the list is
 * a few thousand strings at most and filtering is local, so mention completion
 * stays instant and works while the daemon is briefly busy.
 */
export function useProjectFiles(daemonUrl: string, daemonOnline: boolean, path?: string) {
  const [files, setFiles] = useState<string[]>([]);

  useEffect(() => {
    if (!daemonOnline) return;
    let alive = true;
    (async () => {
      try {
        const res = await invoke<{ files: string[] }>("list_files", { url: daemonUrl, path });
        if (alive) setFiles(res.files ?? []);
      } catch {
        if (alive) setFiles([]);
      }
    })();
    return () => {
      alive = false;
    };
  }, [daemonUrl, daemonOnline, path]);

  return files;
}
