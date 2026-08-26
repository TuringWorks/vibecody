import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Full-duplex voice, against the daemon's `/ws/voice/duplex` route.
 *
 * The microphone stays open while the assistant speaks, so the user can
 * interrupt mid-sentence. That is only possible because the browser's echo
 * canceller removes our own playback from the capture stream — measured at
 * ≥40 dB in a WKWebView, and it covers WebAudio-rendered output, which is what
 * this hook uses (`tools/webview-probe --arm aec`).
 *
 * **A host without echo cancellation must not use this hook.** With an open mic
 * and no AEC the agent hears itself and interrupts itself on every sentence;
 * such surfaces should stay on push-to-talk `useVoiceInput`.
 *
 * All pipeline logic — VAD, turn-taking, ASR, the model call, TTS — lives in
 * the daemon. This is a microphone and a speaker.
 */

/** What the conversation is doing right now. */
export type DuplexState =
  | { status: "idle" }
  | { status: "connecting" }
  /** Mic open, nothing being said. */
  | { status: "listening" }
  /** The user is speaking. */
  | { status: "hearing" }
  /** Transcribed; waiting on the model. */
  | { status: "thinking" }
  /** Audio is playing back. */
  | { status: "speaking" }
  | { status: "error"; message: string };

export interface DuplexTurn {
  role: "user" | "assistant";
  text: string;
  /** Whisper language code, when the daemon detected one. */
  lang?: string;
  /** Still being revised — an interim hypothesis. */
  interim?: boolean;
}

export interface DuplexLatency {
  asrMs?: number;
  llmTtftMs?: number;
  firstAudioMs?: number;
  totalMs?: number;
}

export interface UseVoiceDuplexOptions {
  /** Daemon base URL, e.g. `http://127.0.0.1:7878`. Omit and the hook asks the
   *  shell for the port it actually bound — `VIBECLI_DAEMON_PORT` moves it, so
   *  a hardcoded 7878 is wrong on any non-default setup. */
  daemonUrl?: string;
  /** Bearer token. A WebSocket cannot set headers, so it goes in the query.
   *  Leave empty and the hook resolves the effective one from the shell —
   *  a local daemon mints a fresh token on every start and stores it nowhere
   *  the frontend can see, so asking the settings store returns null and the
   *  socket 401s against the user's own daemon. */
  token?: string;
  /** Provider + model for the reply, from the host's own selector. Required by
   *  the provider-agnostic rule — never let the daemon pick. */
  provider?: string;
  model?: string;
  /** `en` for the fast path, `auto` to detect per turn, or a language code. */
  language?: string;
  voice?: string;
  onTurn?: (turn: DuplexTurn) => void;
}

export interface UseVoiceDuplex {
  state: DuplexState;
  turns: DuplexTurn[];
  latency: DuplexLatency;
  /** Whether this host can do duplex at all — drive the button's presence. */
  supported: boolean;
  active: boolean;
  start: () => Promise<void>;
  stop: () => void;
  setVoice: (id: string) => void;
  setLanguage: (lang: string) => void;
}

/** Capture worklet: downsampled by the AudioContext, converted to Int16 here. */
const CAPTURE_WORKLET = `
class Cap extends AudioWorkletProcessor {
  process(inputs) {
    const ch = inputs[0][0];
    if (ch) {
      const pcm = new Int16Array(ch.length);
      for (let i = 0; i < ch.length; i++) {
        const s = Math.max(-1, Math.min(1, ch[i]));
        pcm[i] = s < 0 ? s * 0x8000 : s * 0x7FFF;
      }
      this.port.postMessage(pcm, [pcm.buffer]);
    }
    return true;
  }
}
registerProcessor('cap', Cap);`;

/** Whether this webview can plausibly run duplex. */
export function duplexSupported(): boolean {
  return (
    typeof AudioWorkletNode !== "undefined" &&
    typeof WebSocket !== "undefined" &&
    !!navigator.mediaDevices?.getUserMedia
  );
}

export function useVoiceDuplex(opts: UseVoiceDuplexOptions): UseVoiceDuplex {
  const [state, setState] = useState<DuplexState>({ status: "idle" });
  const [turns, setTurns] = useState<DuplexTurn[]>([]);
  const [latency, setLatency] = useState<DuplexLatency>({});

  const ws = useRef<WebSocket | null>(null);
  const capCtx = useRef<AudioContext | null>(null);
  const playCtx = useRef<AudioContext | null>(null);
  const stream = useRef<MediaStream | null>(null);
  const nextAt = useRef(0);
  const live = useRef<AudioBufferSourceNode[]>([]);
  const onTurn = useRef(opts.onTurn);
  onTurn.current = opts.onTurn;

  /** Stop every scheduled source and reset the cursor. */
  const flush = useCallback(() => {
    live.current.forEach((s) => {
      try {
        s.stop();
      } catch {
        /* already ended */
      }
    });
    live.current = [];
    nextAt.current = 0;
  }, []);

  const stop = useCallback(() => {
    flush();
    ws.current?.close();
    ws.current = null;
    stream.current?.getTracks().forEach((t) => t.stop());
    stream.current = null;
    void capCtx.current?.close();
    void playCtx.current?.close();
    capCtx.current = null;
    playCtx.current = null;
    setState({ status: "idle" });
  }, [flush]);

  /**
   * Queue a chunk back-to-back on the AudioContext clock.
   *
   * Frames are self-describing — a `u32` sample rate then f32 samples — because
   * the daemon's streaming and batch engines produce different rates and a
   * wrong rate does not fail, it just plays at the wrong pitch.
   */
  const enqueue = useCallback((data: ArrayBuffer) => {
    const ctx = playCtx.current;
    if (!ctx || data.byteLength < 8) return;
    const rate = new DataView(data).getUint32(0, true);
    const pcm = new Float32Array(data, 4);
    const buf = ctx.createBuffer(1, pcm.length, rate);
    buf.copyToChannel(pcm, 0);
    const src = ctx.createBufferSource();
    src.buffer = buf;
    src.connect(ctx.destination);
    const now = ctx.currentTime;
    if (nextAt.current < now + 0.02) nextAt.current = now + 0.02;
    src.start(nextAt.current);
    nextAt.current += buf.duration;
    live.current.push(src);
    src.onended = () => {
      live.current = live.current.filter((s) => s !== src);
    };
  }, []);

  const start = useCallback(async () => {
    if (ws.current) return;
    setState({ status: "connecting" });
    try {
      // Ask for all three; WebKit only honours echoCancellation, and reporting
      // what was *applied* rather than what was requested is the difference
      // between knowing and assuming.
      const s = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, autoGainControl: true, noiseSuppression: true },
      });
      stream.current = s;
      const applied = s.getAudioTracks()[0]?.getSettings?.().echoCancellation;
      if (applied !== true) {
        // Not fatal — some engines omit the field entirely — but an open mic
        // with no echo cancellation will make the assistant interrupt itself.
        console.warn("[voice-duplex] echoCancellation not confirmed:", applied);
      }

      // 16 kHz for capture (what the daemon's ASR wants); playback gets its own
      // context so neither has to resample by hand.
      const cap = new AudioContext({ sampleRate: 16000 });
      await cap.resume();
      capCtx.current = cap;
      const play = new AudioContext();
      await play.resume();
      playCtx.current = play;

      const url = URL.createObjectURL(
        new Blob([CAPTURE_WORKLET], { type: "application/javascript" }),
      );
      await cap.audioWorklet.addModule(url);
      URL.revokeObjectURL(url);
      const node = new AudioWorkletNode(cap, "cap");
      cap.createMediaStreamSource(s).connect(node);
      // A worklet with no downstream connection is not pulled in some engines.
      const mute = cap.createGain();
      mute.gain.value = 0;
      node.connect(mute).connect(cap.destination);

      // Resolve late: the daemon rotates its token on every restart, and the
      // shells restart it themselves.
      let token = opts.token ?? "";
      if (!token) {
        try {
          token = (await invoke<string | null>("daemon_token_effective", { explicit: null })) ?? "";
        } catch {
          /* a host without the command falls back to an unauthenticated daemon */
        }
      }
      let daemonUrl = opts.daemonUrl;
      if (!daemonUrl) {
        try {
          const port = await invoke<number>("daemon_port");
          daemonUrl = `http://127.0.0.1:${port}`;
        } catch {
          daemonUrl = "http://127.0.0.1:7878";
        }
      }
      const base = daemonUrl.replace(/^http/, "ws");
      const q = new URLSearchParams({ token });
      if (opts.provider) q.set("provider", opts.provider);
      if (opts.model) q.set("model", opts.model);
      if (opts.language) q.set("language", opts.language);
      if (opts.voice) q.set("voice", opts.voice);
      const sock = new WebSocket(`${base}/ws/voice/duplex?${q}`);
      sock.binaryType = "arraybuffer";
      ws.current = sock;

      node.port.onmessage = (e) => {
        if (sock.readyState === WebSocket.OPEN) sock.send(e.data.buffer ?? e.data);
      };

      sock.onmessage = (ev) => {
        if (ev.data instanceof ArrayBuffer) {
          enqueue(ev.data);
          return;
        }
        const m = JSON.parse(ev.data as string);
        switch (m.type) {
          case "state":
            setState({ status: m.state });
            break;
          case "flush":
            flush();
            break;
          case "transcript": {
            const turn: DuplexTurn = { role: "user", text: m.text, lang: m.lang };
            setTurns((t) => [...t, turn]);
            setLatency((l) => ({ ...l, asrMs: m.asr_ms }));
            onTurn.current?.(turn);
            break;
          }
          case "speaking": {
            const turn: DuplexTurn = { role: "assistant", text: m.text };
            setTurns((t) => [...t, turn]);
            onTurn.current?.(turn);
            break;
          }
          case "latency":
            setLatency((l) => ({ ...l, firstAudioMs: m.first_audio_ms }));
            break;
          case "reply":
            setLatency({
              asrMs: m.asr_ms,
              llmTtftMs: m.llm_ttft_ms,
              firstAudioMs: m.first_audio_ms ?? undefined,
              totalMs: m.total_ms,
            });
            break;
          case "error":
            setState({ status: "error", message: m.message });
            break;
        }
      };
      sock.onerror = () => setState({ status: "error", message: "connection failed" });
      sock.onclose = () => {
        if (ws.current === sock) stop();
      };
    } catch (e) {
      setState({
        status: "error",
        message: e instanceof Error ? e.message : "could not start voice",
      });
    }
  }, [opts.daemonUrl, opts.token, opts.provider, opts.model, opts.language, opts.voice, enqueue, flush, stop]);

  const send = useCallback((v: unknown) => {
    if (ws.current?.readyState === WebSocket.OPEN) ws.current.send(JSON.stringify(v));
  }, []);

  useEffect(() => () => stop(), [stop]);

  return {
    state,
    turns,
    latency,
    supported: duplexSupported(),
    active: state.status !== "idle" && state.status !== "error",
    start,
    stop,
    setVoice: (id) => send({ type: "set_voice", id }),
    setLanguage: (lang) => send({ type: "set_language", lang }),
  };
}
