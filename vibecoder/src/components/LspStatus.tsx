/**
 * LspStatus — status-bar indicator for the active file's language server.
 *
 * IntelliSense fails silently by nature: a server that is not installed, or
 * that crashed on startup, looks exactly like a language with no completions.
 * This is the one place that says which it is, and offers the fix.
 *
 * It does not poll. It probes when the file (or its language) changes, once
 * more shortly after if the server was still starting, and whenever the user
 * clicks to retry — a standing interval would burn CPU to re-learn a fact that
 * only changes when someone installs a server.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import {
  hasBuiltinLanguageService,
  lspLanguageForPath,
  parseInstallHint,
  type LspLanguageSupport,
  type InvokeFn,
} from "../lib/lsp";

export interface LspStatusProps {
  /** Absolute path of the file in the editor, or null when none is open. */
  filePath: string | null;
  /** Workspace root; the backend needs it to know where to root a server. */
  workspaceRoot: string;
  invoke: InvokeFn;
}

/** How long after a probe to look again while a server is still starting. */
const STARTUP_RECHECK_MS = 2000;

interface Display {
  label: string;
  title: string;
  tone: "ok" | "warn" | "muted";
  actionable: boolean;
}

export function describeSupport(support: LspLanguageSupport): Display {
  switch (support.state) {
    case "running":
      return {
        label: `IntelliSense: ${support.language}`,
        title: `Language server running for ${support.language}`,
        tone: "ok",
        actionable: true,
      };
    case "available":
      return {
        label: `IntelliSense: starting ${support.language}`,
        title: `Starting ${support.detail || support.language}`,
        tone: "muted",
        actionable: false,
      };
    case "not_installed":
      return {
        label: `No IntelliSense: ${support.language}`,
        title: `${support.detail}\n\nClick to retry after installing.`,
        tone: "warn",
        actionable: true,
      };
    case "failed":
      return {
        label: `IntelliSense failed: ${support.language}`,
        title: `${support.detail}\n\nClick to retry.`,
        tone: "warn",
        actionable: true,
      };
    case "unconfigured":
      return {
        label: "",
        title: "",
        tone: "muted",
        actionable: false,
      };
  }
}

const TONE_COLOR: Record<Display["tone"], string> = {
  ok: "var(--success-color, #22c55e)",
  warn: "var(--warning-color, #f59e0b)",
  muted: "var(--text-secondary, #9ca3af)",
};

export function LspStatus({ filePath, workspaceRoot, invoke }: LspStatusProps) {
  const [support, setSupport] = useState<LspLanguageSupport | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const recheckRef = useRef<number | null>(null);

  const fileLanguage = filePath === null ? null : lspLanguageForPath(filePath);
  // Monaco services these itself, so no server is started and none is missing.
  // Probing would report `typescript-language-server` as "not installed" and
  // warn about a gap that does not exist. Keyed on the LSP language so `.vue`
  // and `.svelte` — which highlight as `html` — are not mistaken for it.
  const builtin = fileLanguage !== null && hasBuiltinLanguageService(fileLanguage);
  const language = builtin ? null : fileLanguage;
  // A single-file window has no folder; the backend roots the server at the
  // file's directory, so an empty root is still a valid probe.
  const rootPath = workspaceRoot;

  const probe = useCallback(
    async (targetLanguage: string): Promise<LspLanguageSupport | null> => {
      try {
        return await invoke<LspLanguageSupport>("lsp_language_support", {
          language: targetLanguage,
          rootPath,
        });
      } catch {
        return null;
      }
    },
    [invoke, rootPath],
  );

  useEffect(() => {
    if (recheckRef.current !== null) {
      window.clearTimeout(recheckRef.current);
      recheckRef.current = null;
    }
    if (language === null) {
      setSupport(null);
      return;
    }

    let cancelled = false;
    void (async () => {
      const first = await probe(language);
      if (cancelled) return;
      setSupport(first);
      // The editor starts the server in parallel with this probe, so a
      // just-opened file usually reports "available" before it reports
      // "running". One follow-up look settles it without a poll.
      if (first && first.state !== "running" && first.state !== "unconfigured") {
        recheckRef.current = window.setTimeout(() => {
          recheckRef.current = null;
          void probe(language).then((again) => {
            if (!cancelled && again) setSupport(again);
          });
        }, STARTUP_RECHECK_MS);
      }
    })();

    return () => {
      cancelled = true;
      if (recheckRef.current !== null) {
        window.clearTimeout(recheckRef.current);
        recheckRef.current = null;
      }
    };
  }, [language, probe]);

  const retry = useCallback(async () => {
    if (language === null || busy) return;
    setBusy(true);
    try {
      // Clears the "not installed" memo too, so this is the action to take
      // right after installing a server — no app restart needed.
      await invoke("lsp_restart_language", { language });
      const refreshed = await probe(language);
      if (refreshed) setSupport(refreshed);
    } catch {
      // Leave the previous state on screen; the label is still accurate.
    } finally {
      setBusy(false);
    }
  }, [busy, invoke, language, probe]);

  if (builtin) {
    return (
      <span className="status-item" style={{ color: TONE_COLOR.ok }}
        title={`Monaco's built-in ${fileLanguage} service provides IntelliSense for this file`}>
        IntelliSense: built-in
      </span>
    );
  }
  if (language === null || support === null) return null;
  const display = describeSupport(support);
  if (display.label === "") return null;

  // A hint the user has to retype is barely better than no hint, so when the
  // server is missing we offer the command itself.
  const install = support.installHint
    ? parseInstallHint(support.installHint)
    : {};

  return (
    <>
      <button
        type="button"
        className="status-item"
        onClick={display.actionable ? retry : undefined}
        disabled={!display.actionable || busy}
        title={display.title}
        aria-label={display.title || display.label}
        style={{ color: TONE_COLOR[display.tone] }}
      >
        {busy ? `Restarting ${language}…` : display.label}
      </button>
      {install.command !== undefined && (
        <button
          type="button"
          className="status-item"
          onClick={() => {
            setCopied(false);
            // Copy rather than run: installing software on someone's machine
            // is their decision, and several hints need a platform choice.
            void navigator.clipboard
              ?.writeText(install.command as string)
              .then(() => setCopied(true))
              .catch(() => setCopied(false));
          }}
          title={`Copy to clipboard:\n${install.command}`}
          aria-label={`Copy install command for ${language}: ${install.command}`}
          style={{ color: TONE_COLOR.muted }}
        >
          {copied ? "Copied ✓" : "Copy install"}
        </button>
      )}
    </>
  );
}
