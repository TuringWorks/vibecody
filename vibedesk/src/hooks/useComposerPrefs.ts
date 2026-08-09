import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ApprovalTier } from "../components/ApprovalPill";
import type { ReasoningEffort } from "../components/ReasoningPill";
import type { RunMode } from "../components/ModePill";
import { LOCKED_SANDBOX, type SandboxPolicy } from "../lib/sandbox";

/** Settings key holding the composer's run controls as a JSON blob. */
const PREFS_KEY = "vibedesk.composer";

export interface ComposerPrefs {
  provider: string;
  model?: string;
  approval: ApprovalTier;
  reasoning: ReasoningEffort;
  /** Agent (tool loop) or Chat (single reply, no tools). */
  mode: RunMode;
  /** Sandbox-mode grants. Ignored in Agent and Chat modes. */
  sandbox: SandboxPolicy;
  /** Whether the next run forks its own git worktree branch. */
  isolate: boolean;
}

const DEFAULTS: ComposerPrefs = {
  provider: "ollama",
  model: undefined,
  approval: "default",
  // Off by default: extended thinking is opt-in. It costs latency and
  // tokens on every turn, and for most prompts the user wants an answer.
  reasoning: "off",
  // Agent stays the default: VibeDesk is a task runner first, and
  // silently answering instead of acting would be the worse surprise.
  mode: "agent",
  // Denies everything outside the workspace: selecting Sandbox mode grants
  // nothing until the user opens the settings and says what to allow.
  sandbox: LOCKED_SANDBOX,
  isolate: false,
};

function parsePrefs(raw: string): Partial<ComposerPrefs> {
  try {
    const parsed: unknown = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? (parsed as Partial<ComposerPrefs>) : {};
  } catch {
    return {};
  }
}

/**
 * The composer's run controls (provider · model · approval · reasoning ·
 * branch), owned above the conversation pane and persisted to the encrypted
 * settings DB.
 *
 * These used to be `useState` inside TaskPrompt. Because the conversation pane
 * is remounted on every chat switch, that reset the model to the hardcoded
 * default each time the user clicked a chat or "New chat" — so a deliberately
 * chosen model silently reverted, and the run went to something else. Lifting
 * them out of the remounted subtree fixes that within a session; persisting
 * them carries the choice across restarts.
 */
export function useComposerPrefs() {
  const [prefs, setPrefs] = useState<ComposerPrefs>(DEFAULTS);
  // Don't persist the initial defaults back over a stored value before the
  // load resolves — that would erase the user's saved model on every launch.
  const loaded = useRef(false);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const raw = await invoke<string | null>("setting_get", { key: PREFS_KEY });
        if (alive && raw) setPrefs((prev) => ({ ...prev, ...parsePrefs(raw) }));
      } catch (e) {
        console.error("load composer prefs failed", e);
      } finally {
        if (alive) loaded.current = true;
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    if (!loaded.current) return;
    invoke("setting_set", { key: PREFS_KEY, value: JSON.stringify(prefs) }).catch((e) =>
      console.error("persist composer prefs failed", e)
    );
  }, [prefs]);

  const update = useCallback(<K extends keyof ComposerPrefs>(key: K, value: ComposerPrefs[K]) => {
    setPrefs((prev) => (prev[key] === value ? prev : { ...prev, [key]: value }));
  }, []);

  /** Provider and model move together — a model always belongs to a provider. */
  const setProviderModel = useCallback((provider: string, model: string | undefined) => {
    setPrefs((prev) =>
      prev.provider === provider && prev.model === model ? prev : { ...prev, provider, model }
    );
  }, []);

  return { prefs, update, setProviderModel };
}
