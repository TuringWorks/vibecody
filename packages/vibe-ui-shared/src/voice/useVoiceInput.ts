import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  describeSpeechError,
  getSpeechRecognition,
  type SpeechRecognitionLike,
} from "./speech";
import { TranscriptionError, type Transcriber } from "./transcribers";

/**
 * What the mic is doing right now.
 *
 * Modelled as a sum type rather than parallel `isListening` / `isTranscribing` /
 * `error` fields, because those admit states that cannot happen (listening *and*
 * transcribing, an error while still recording) and every consumer then has to
 * decide which flag wins.
 */
export type VoiceState =
  | { status: "idle" }
  /** Mic is open. `interim` is the not-yet-final text, if the engine streams it. */
  | { status: "listening"; interim: string }
  /** Mic is closed and the clip is being transcribed. */
  | { status: "transcribing" }
  /** Something the user should be told about. Cleared on the next `toggle()`. */
  | { status: "error"; message: string };

export interface UseVoiceInputOptions {
  /** Called with each finalised chunk of recognised text. */
  onTranscript: (text: string) => void;
  /**
   * How to turn a recorded clip into text when the Web Speech API is absent.
   * Omit to disable the fallback — the hook then reports itself unsupported on
   * webviews without Web Speech, rather than opening a mic it can't use.
   */
  transcribe?: Transcriber;
  /** BCP-47 language tag for Web Speech recognition. */
  lang?: string;
}

export interface UseVoiceInput {
  state: VoiceState;
  /** Start listening, or stop if already listening. */
  toggle: () => void;
  /** Whether this host can capture voice at all — drive the button's presence. */
  supported: boolean;
  isListening: boolean;
  isTranscribing: boolean;
  /** Interim (non-final) text while listening; `""` otherwise. */
  interimText: string;
  /** Current error message, or `null`. */
  error: string | null;
  /** Dismiss an error without starting a new recording. */
  clearError: () => void;
}

/** Pick a container the browser will actually record into. */
function pickMimeType(): string {
  const candidates = [
    "audio/webm;codecs=opus",
    "audio/webm",
    "audio/mp4",
    "audio/ogg;codecs=opus",
  ];
  return candidates.find((t) => MediaRecorder.isTypeSupported(t)) ?? "";
}

/**
 * Microphone input for a chat composer, shared by every VibeCody webview.
 *
 * Two engines, in order:
 *
 * 1. **Web Speech API** — free, streams interim text, no audio ever leaves the
 *    machine on Chromium+Google. Absent from most Tauri/WKWebView builds.
 * 2. **`MediaRecorder` + `transcribe`** — records a clip and hands it to the
 *    supplied backend (the daemon's `/voice/transcribe`, which itself prefers a
 *    local whisper model and falls back to Groq).
 *
 * Every failure path sets `state.status === "error"` with something a user can
 * act on. The earlier VibeCoder-only version swallowed all of them, so a denied
 * mic permission, a missing API key and an unsupported webview were
 * indistinguishable from a button that simply did nothing.
 */
export function useVoiceInput({
  onTranscript,
  transcribe,
  lang = "en-US",
}: UseVoiceInputOptions): UseVoiceInput {
  const [state, setState] = useState<VoiceState>({ status: "idle" });
  const recognitionRef = useRef<SpeechRecognitionLike | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const streamRef = useRef<MediaStream | null>(null);

  // `toggle` must not be re-created when the consumer passes a new inline
  // callback, or a button's onClick identity churns on every keystroke.
  const onTranscriptRef = useRef(onTranscript);
  useEffect(() => {
    onTranscriptRef.current = onTranscript;
  }, [onTranscript]);

  const transcribeRef = useRef(transcribe);
  useEffect(() => {
    transcribeRef.current = transcribe;
  }, [transcribe]);

  const supported = useMemo(() => {
    const hasWebSpeech = typeof window !== "undefined" && !!getSpeechRecognition();
    const hasRecorder =
      typeof window !== "undefined" &&
      typeof MediaRecorder !== "undefined" &&
      !!navigator.mediaDevices?.getUserMedia &&
      !!transcribe;
    return hasWebSpeech || hasRecorder;
    // `transcribe` is compared by identity on purpose: a consumer that only
    // has a backend once the daemon is up should re-evaluate support then.
  }, [transcribe]);

  /** Release the mic. Safe to call from any state. */
  const teardown = useCallback(() => {
    recognitionRef.current = null;
    recorderRef.current = null;
    streamRef.current?.getTracks().forEach((t) => t.stop());
    streamRef.current = null;
  }, []);

  // Unmounting mid-recording must not leave the OS mic indicator on.
  useEffect(
    () => () => {
      try {
        recognitionRef.current?.abort();
      } catch {
        /* already dead */
      }
      teardown();
    },
    [teardown],
  );

  const startWebSpeech = useCallback(
    (Ctor: NonNullable<ReturnType<typeof getSpeechRecognition>>): boolean => {
      try {
        const recognition = new Ctor();
        recognition.continuous = true;
        recognition.interimResults = true;
        recognition.lang = lang;
        recognition.maxAlternatives = 1;

        recognition.onresult = (event) => {
          let interim = "";
          let final = "";
          for (let i = event.resultIndex; i < event.results.length; i++) {
            const result = event.results[i];
            if (result.isFinal) final += result[0].transcript;
            else interim += result[0].transcript;
          }
          if (final) onTranscriptRef.current(final);
          setState((prev) =>
            prev.status === "listening" ? { status: "listening", interim } : prev,
          );
        };

        recognition.onerror = (event) => {
          // `no-speech` and `aborted` are how a user cancels; they are not
          // failures worth a red message.
          const benign = event.error === "no-speech" || event.error === "aborted";
          setState(
            benign
              ? { status: "idle" }
              : { status: "error", message: describeSpeechError(event.error) },
          );
          teardown();
        };

        recognition.onend = () => {
          setState((prev) => (prev.status === "listening" ? { status: "idle" } : prev));
          teardown();
        };

        recognition.start();
        recognitionRef.current = recognition;
        setState({ status: "listening", interim: "" });
        return true;
      } catch {
        return false;
      }
    },
    [lang, teardown],
  );

  const startRecorder = useCallback(async () => {
    const backend = transcribeRef.current;
    if (!backend) {
      setState({
        status: "error",
        message: "Voice input isn't available in this window.",
      });
      return;
    }

    let stream: MediaStream;
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    } catch {
      setState({
        status: "error",
        message: "Microphone access was denied. Allow it in your system settings and try again.",
      });
      return;
    }
    streamRef.current = stream;

    const mimeType = pickMimeType();
    let recorder: MediaRecorder;
    try {
      recorder = mimeType ? new MediaRecorder(stream, { mimeType }) : new MediaRecorder(stream);
    } catch {
      teardown();
      setState({ status: "error", message: "This window can't record audio." });
      return;
    }
    chunksRef.current = [];

    recorder.ondataavailable = (e) => {
      if (e.data.size > 0) chunksRef.current.push(e.data);
    };

    recorder.onerror = () => {
      teardown();
      setState({ status: "error", message: "Recording failed." });
    };

    recorder.onstop = async () => {
      const type = recorder.mimeType || mimeType || "audio/webm";
      const blob = new Blob(chunksRef.current, { type });
      chunksRef.current = [];
      teardown();

      // A blob this small is silence or an immediate second click, not speech.
      if (blob.size < 1024) {
        setState({ status: "idle" });
        return;
      }

      setState({ status: "transcribing" });
      try {
        const text = await backend(blob);
        setState({ status: "idle" });
        if (text.trim()) onTranscriptRef.current(text);
      } catch (e) {
        setState({
          status: "error",
          message:
            e instanceof TranscriptionError || e instanceof Error
              ? e.message
              : "Transcription failed.",
        });
      }
    };

    recorder.start();
    recorderRef.current = recorder;
    setState({ status: "listening", interim: "" });
  }, [teardown]);

  const toggle = useCallback(() => {
    if (state.status === "transcribing") return;

    if (state.status === "listening") {
      if (recognitionRef.current) recognitionRef.current.stop();
      else recorderRef.current?.stop();
      return;
    }

    const Ctor = getSpeechRecognition();
    if (Ctor && startWebSpeech(Ctor)) return;
    void startRecorder();
  }, [state.status, startWebSpeech, startRecorder]);

  const clearError = useCallback(
    () => setState((prev) => (prev.status === "error" ? { status: "idle" } : prev)),
    [],
  );

  return {
    state,
    toggle,
    supported,
    isListening: state.status === "listening",
    isTranscribing: state.status === "transcribing",
    interimText: state.status === "listening" ? state.interim : "",
    error: state.status === "error" ? state.message : null,
    clearError,
  };
}
