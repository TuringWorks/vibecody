import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useVoiceDuplex, type UseVoiceDuplex } from "@vibe/shared/voice/useVoiceDuplex";
import { useVoiceDuplexPreference } from "@vibe/shared/voice/useVoiceDuplexPreference";
import { buildVoiceContext, findReadme, VOICE_CONTEXT_LIMITS } from "@vibe/shared/voice/voiceContext";
import { useProjectFiles } from "./useProjectFiles";
import type { Attachment } from "../lib/attachments";

/**
 * The project's README, for the voice context block.
 *
 * Voice answers in one round trip and reads files only when the daemon has a
 * workspace root to jail its tools to, so the file that names the project has
 * to be in the block or the answer is "a collection of directories and files".
 * Failure is silence: a repo without a README is normal.
 */
function useProjectReadme(root: string | undefined, tree: readonly string[]): string | null {
  const [readme, setReadme] = useState<string | null>(null);
  const rel = useMemo(() => (tree.length ? findReadme(tree) : undefined), [tree]);

  useEffect(() => {
    if (!root || !rel) {
      setReadme(null);
      return;
    }
    let alive = true;
    invoke<Attachment>("read_attachment", { path: `${root.replace(/\/+$/, "")}/${rel}` })
      .then((a) => {
        if (alive) setReadme(a.text.slice(0, VOICE_CONTEXT_LIMITS.readme * 2));
      })
      .catch(() => {
        if (alive) setReadme(null);
      });
    return () => {
      alive = false;
    };
  }, [root, rel]);

  return readme;
}

/** Where a completed spoken turn is written down. */
export type VoiceTurnSink = (role: "user" | "assistant", text: string) => void;

export interface VoiceSession {
  duplex: UseVoiceDuplex;
  enabled: boolean;
  setEnabled: (on: boolean) => void;
  /** The scoped project's tracked paths — also what @-mentions complete from. */
  files: string[];
  /**
   * Register the pane that shows the conversation. The pane below is remounted
   * per chat, so it hands its writer in on mount rather than being wired once.
   */
  registerSink: (sink: VoiceTurnSink) => void;
}

export interface VoiceSessionOptions {
  daemonUrl: string;
  daemonOnline: boolean;
  /** The repo a spoken question is about — worktree, project, or nothing. */
  root?: string;
  /** The composer's provider and model, unless the daemon's voice setting
   *  overrides them (it is the daemon that decides; see `/voice/settings`). */
  provider: string;
  model?: string;
}

/**
 * The voice conversation, owned above the chat pane.
 *
 * It lives here rather than inside the composer because the conversation pane
 * is remounted on every chat switch (`key={chatNonce}` in `ShellLayout`) and on
 * every full-screen overlay. Mounted below that line, the hook's teardown ran
 * on each of those: the socket closed, the microphone closed, and the daemon's
 * per-socket history went with it — so clicking a chat mid-sentence ended the
 * conversation silently and the assistant lost every turn it had just had.
 * The composer's run controls were lifted out for the same reason.
 */
export function useVoiceSession({
  daemonUrl,
  daemonOnline,
  root,
  provider,
  model,
}: VoiceSessionOptions): VoiceSession {
  const pref = useVoiceDuplexPreference();
  const files = useProjectFiles(daemonUrl, daemonOnline, root);
  const readme = useProjectReadme(root, files);

  // A ref, not state: the pane that renders the conversation changes identity
  // on every chat switch, and re-rendering the whole shell to record that would
  // be a lot of paint for a callback nobody looks at.
  const sink = useRef<VoiceTurnSink | null>(null);
  const registerSink = useCallback((next: VoiceTurnSink) => {
    sink.current = next;
  }, []);

  const context = useMemo(
    () => buildVoiceContext({ root, readme, tree: files }),
    [root, readme, files],
  );

  const duplex = useVoiceDuplex({
    enabled: pref.enabled,
    daemonUrl,
    provider,
    model,
    // `auto`, not a pinned language. Pinning one does not merely bias the
    // recogniser — it *suppresses* the detection result, so every turn came
    // back labelled English, the reply rule never fired, and a question asked
    // in Hindi was answered in English. Detection runs per turn because
    // code-switching mid-conversation is normal for multilingual speakers.
    language: "auto",
    context,
    workspaceRoot: root ?? null,
    // Fires once per completed turn — the transcription, then the whole reply.
    // A pane that has since been replaced simply is not there to receive it;
    // dropping the turn is better than writing it into a chat the user left.
    onTurn: (turn) => sink.current?.(turn.role, turn.text),
  });

  return { duplex, enabled: pref.enabled, setEnabled: pref.setEnabled, files, registerSink };
}
