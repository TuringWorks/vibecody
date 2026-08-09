/**
 * Microphone capture for the VS Code extension host.
 *
 * The extension host is plain Node — no `getUserMedia`, no `MediaRecorder`.
 * (A webview *sometimes* has them, but permission prompts inside a webview
 * iframe are unreliable across VS Code builds and silently deny on several, so
 * the mic cannot live there.) We shell out to SoX's `rec`, exactly as
 * `VoiceDispatcher::listen` in `vibecli/vibecli-cli/src/voice.rs` has always
 * done for the REPL — one capture strategy for every non-browser client.
 *
 * SoX is an explicit, documented dependency. Its absence is reported as an
 * install hint, never as a generic failure.
 */

import { spawn, type ChildProcess } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

/** Distinct failure states. Each needs different advice, so each has a tag. */
export type CaptureFailure =
  | { kind: "sox-missing"; message: string }
  | { kind: "spawn-failed"; message: string }
  | { kind: "no-audio"; message: string }
  | { kind: "cancelled"; message: string };

export type CaptureResult =
  | { ok: true; wav: Uint8Array; path: string }
  | { ok: false; failure: CaptureFailure };

/** Install guidance, kept in one place because three clients print it. */
export const SOX_INSTALL_HINT =
  "Voice input needs SoX. Install it:\n" +
  "  macOS:   brew install sox\n" +
  "  Linux:   sudo apt install sox\n" +
  "  Windows: choco install sox";

/** True if `rec` (SoX) is on PATH. */
export function hasSox(): Promise<boolean> {
  return new Promise((resolve) => {
    // `rec --version` exits non-zero on some builds, so treat "it ran at all"
    // as present and only a spawn error as absent.
    const probe = spawn("rec", ["--version"], { stdio: "ignore" });
    probe.on("error", () => resolve(false));
    probe.on("close", () => resolve(true));
  });
}

export interface RecordingHandle {
  /** Stop recording and resolve with the captured WAV. */
  stop(): Promise<CaptureResult>;
  /** Abort and discard. */
  cancel(): void;
}

export interface RecordOptions {
  /**
   * Stop automatically after this many seconds of silence. Omit for
   * press-to-stop, which is what the VS Code command uses — a modal
   * "Stop recording" button is clearer than guessing at a silence threshold.
   */
  silenceSeconds?: number;
  /** Hard ceiling so a forgotten recording can't fill the disk. */
  maxSeconds?: number;
}

/**
 * Start recording to a temp WAV. Resolves once `rec` has been spawned; the
 * caller stops it via the returned handle.
 */
export async function startRecording(opts: RecordOptions = {}): Promise<
  { ok: true; handle: RecordingHandle } | { ok: false; failure: CaptureFailure }
> {
  if (!(await hasSox())) {
    return { ok: false, failure: { kind: "sox-missing", message: SOX_INSTALL_HINT } };
  }

  const wavPath = path.join(
    os.tmpdir(),
    `vibecli-voice-${process.pid}-${Date.now()}.wav`,
  );

  // 16 kHz mono is what every whisper backend resamples to anyway; recording
  // it directly avoids an ffmpeg conversion step on the daemon side.
  const args = [wavPath, "rate", "16000", "channels", "1"];
  if (opts.maxSeconds) args.push("trim", "0", String(opts.maxSeconds));
  if (opts.silenceSeconds) {
    args.push("silence", "1", "0.1", "1%", "1", String(opts.silenceSeconds), "1%");
  }

  let child: ChildProcess;
  try {
    child = spawn("rec", args, { stdio: "ignore" });
  } catch (e) {
    return { ok: false, failure: { kind: "spawn-failed", message: String(e) } };
  }

  let spawnError: Error | null = null;
  child.on("error", (e) => {
    spawnError = e;
  });

  const exited = new Promise<void>((resolve) => {
    child.on("close", () => resolve());
  });

  const cleanup = () => {
    try {
      fs.rmSync(wavPath, { force: true });
    } catch {
      /* best effort */
    }
  };

  return {
    ok: true,
    handle: {
      async stop(): Promise<CaptureResult> {
        // SIGINT, not SIGKILL: SoX finalises the WAV header on SIGINT. Killed
        // outright it leaves a header claiming zero frames, which every
        // decoder then reads as an empty file.
        if (child.exitCode === null) child.kill("SIGINT");
        await exited;

        if (spawnError) {
          cleanup();
          return {
            ok: false,
            failure: { kind: "spawn-failed", message: String(spawnError) },
          };
        }

        let wav: Buffer;
        try {
          wav = fs.readFileSync(wavPath);
        } catch {
          return {
            ok: false,
            failure: {
              kind: "no-audio",
              message: "SoX produced no recording. Is an input device selected?",
            },
          };
        }
        cleanup();

        // A bare 44-byte RIFF header and little else is silence, not speech.
        if (wav.length < 2048) {
          return {
            ok: false,
            failure: { kind: "no-audio", message: "No speech was recorded." },
          };
        }
        return { ok: true, wav: new Uint8Array(wav), path: wavPath };
      },
      cancel() {
        if (child.exitCode === null) child.kill("SIGINT");
        cleanup();
      },
    },
  };
}
