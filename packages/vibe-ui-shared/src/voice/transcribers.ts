import { invoke } from "@tauri-apps/api/core";

/**
 * Turns recorded audio into text.
 *
 * The hook takes one of these rather than reaching for a backend itself: the
 * Tauri shells go through their own `transcribe_audio` command (which owns the
 * daemon URL and bearer token), while a plain browser build talks to
 * `/voice/transcribe` directly. Both are below.
 */
export type Transcriber = (audio: Blob) => Promise<string>;

/** Thrown when transcription fails for a reason worth showing the user. */
export class TranscriptionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TranscriptionError";
  }
}

/**
 * Base64-encode a Blob without building a giant intermediate string.
 *
 * `FileReader` gives us a `data:` URL whose payload is already base64; slicing
 * past the comma is cheaper and safer than a per-byte `String.fromCharCode`
 * loop, which is O(n) string concatenation and blows up on long recordings.
 */
export function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new TranscriptionError("Could not read the recorded audio."));
    reader.onload = () => {
      const result = typeof reader.result === "string" ? reader.result : "";
      const comma = result.indexOf(",");
      if (comma < 0) {
        reject(new TranscriptionError("Could not encode the recorded audio."));
        return;
      }
      resolve(result.slice(comma + 1));
    };
    reader.readAsDataURL(blob);
  });
}

/**
 * Transcribe through the host shell's `transcribe_audio` Tauri command.
 *
 * VibeDesk and VibeAIChat both register a command of that name that forwards to
 * the daemon's `POST /voice/transcribe`, so the shells keep their rule of never
 * re-implementing daemon logic in the frontend — and the frontend never has to
 * know the rotating bearer token.
 */
export function tauriTranscriber(
  /** Daemon base URL. Omit (or pass `""`) to let the Rust side use its default. */
  daemonUrl?: string,
  opts: {
    /**
     * Explicit bearer token. Omit for the zero-config path — the Rust side
     * reads `~/.vibecli/daemon.token`, which is where `vibecli --serve` writes
     * it, so a locally-autostarted daemon needs no token in the UI at all.
     */
    token?: string;
    /** Force the local whisper model so audio never leaves the machine. */
    preferLocal?: boolean;
    /** Language hint for the local model (e.g. `de`). */
    language?: string;
  } = {},
): Transcriber {
  return async (audio: Blob) => {
    const audioBase64 = await blobToBase64(audio);
    return invoke<string>("transcribe_audio", {
      url: daemonUrl || null,
      audioBase64,
      mimeType: audio.type || "audio/webm",
      language: opts.language ?? null,
      preferLocal: opts.preferLocal ?? null,
      token: opts.token || null,
    });
  };
}

/**
 * Transcribe by POSTing straight to the daemon. For non-Tauri browser hosts,
 * which must supply the bearer token themselves.
 */
export function daemonTranscriber(
  daemonUrl: string,
  token: string,
  preferLocal = false,
): Transcriber {
  return async (audio: Blob) => {
    const res = await fetch(`${daemonUrl.replace(/\/$/, "")}/voice/transcribe`, {
      method: "POST",
      headers: {
        "content-type": audio.type || "audio/webm",
        authorization: `Bearer ${token}`,
        ...(preferLocal ? { "x-voice-prefer-local": "true" } : {}),
      },
      body: audio,
    });
    const body = (await res.json().catch(() => null)) as
      | { text?: string; error?: string }
      | null;
    if (!res.ok) {
      throw new TranscriptionError(
        body?.error ?? `Transcription failed (HTTP ${res.status}).`,
      );
    }
    return body?.text ?? "";
  };
}
