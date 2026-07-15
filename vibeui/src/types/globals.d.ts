/**
 * Ambient global augmentations for the browser `window` object.
 *
 * These are the non-standard globals VibeUI attaches or reads at runtime —
 * previously accessed via `(window as any).X`, which erased all type safety.
 * Declaring them here lets call sites use `window.X` directly and narrow
 * against `undefined`.
 */
import type { ExtensionManager } from "../extensions/ExtensionManager";

/**
 * Minimal structural type for the Web Speech API `SpeechRecognition` object.
 * The Web Speech API is not part of TypeScript's default DOM lib, so we model
 * only the surface VibeUI actually uses (voice input in AIChat / useVoiceInput).
 */
export interface SpeechRecognitionLike {
  continuous: boolean;
  interimResults: boolean;
  maxAlternatives: number;
  lang: string;
  onresult:
    | ((event: {
        resultIndex: number;
        results: {
          length: number;
          [i: number]: { isFinal: boolean; [j: number]: { transcript: string } };
        };
      }) => void)
    | null;
  onerror: ((event: { error: string }) => void) | null;
  onend: (() => void) | null;
  start(): void;
  stop(): void;
  abort(): void;
}

declare global {
  interface Window {
    /** Web Speech API constructor (Chromium exposes it prefixed). */
    SpeechRecognition?: new () => SpeechRecognitionLike;
    webkitSpeechRecognition?: new () => SpeechRecognitionLike;
    /** Last message received from a loaded VS Code-style extension (debug hook). */
    lastExtensionMessage?: string;
    /** The active extension host manager, exposed for debugging/tests. */
    extensionManager?: ExtensionManager;
    /** Current red-team scan session id, stashed across async stages. */
    __vibeScanSession?: string;
  }
}
