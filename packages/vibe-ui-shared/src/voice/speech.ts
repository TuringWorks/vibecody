/**
 * Minimal structural types for the Web Speech API.
 *
 * The Web Speech API is not in TypeScript's default DOM lib, so this models
 * only the surface the voice hook uses. Kept structural (rather than importing
 * a `@types` package) so the shared package stays dependency-free — every host
 * app compiles these files with its own toolchain.
 */

/** A single alternative for one recognition result. */
export interface SpeechAlternativeLike {
  transcript: string;
}

/** One result slot, which may still be revised (`isFinal === false`). */
export interface SpeechResultLike {
  isFinal: boolean;
  [index: number]: SpeechAlternativeLike;
}

export interface SpeechResultListLike {
  length: number;
  [index: number]: SpeechResultLike;
}

export interface SpeechResultEventLike {
  resultIndex: number;
  results: SpeechResultListLike;
}

export interface SpeechErrorEventLike {
  error: string;
}

export interface SpeechRecognitionLike {
  continuous: boolean;
  interimResults: boolean;
  maxAlternatives: number;
  lang: string;
  onresult: ((event: SpeechResultEventLike) => void) | null;
  onerror: ((event: SpeechErrorEventLike) => void) | null;
  onend: (() => void) | null;
  start(): void;
  stop(): void;
  abort(): void;
}

export type SpeechRecognitionCtor = new () => SpeechRecognitionLike;

/**
 * The Web Speech constructor, if this webview has one.
 *
 * Chromium exposes it prefixed. Notably it is *absent* from most Tauri/WKWebView
 * builds, which is why the hook always needs a recorder-based fallback.
 */
export function getSpeechRecognition(): SpeechRecognitionCtor | undefined {
  const w = window as unknown as {
    SpeechRecognition?: SpeechRecognitionCtor;
    webkitSpeechRecognition?: SpeechRecognitionCtor;
  };
  return w.SpeechRecognition ?? w.webkitSpeechRecognition;
}

/**
 * Human-readable explanation for a Web Speech `error` code.
 *
 * These codes reach the user as-is otherwise (`"not-allowed"` in a toast is
 * not an explanation), and two of them — `not-allowed` and `service-not-allowed`
 * — are the ones a user can actually act on.
 */
export function describeSpeechError(code: string): string {
  switch (code) {
    case "not-allowed":
    case "service-not-allowed":
      return "Microphone access was denied. Allow it in your system settings and try again.";
    case "no-speech":
      return "No speech detected.";
    case "audio-capture":
      return "No microphone was found.";
    case "network":
      return "Speech recognition is unavailable — the network request failed.";
    case "aborted":
      return "Recording was cancelled.";
    default:
      return `Speech recognition failed (${code}).`;
  }
}
