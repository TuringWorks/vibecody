import { useCallback, useEffect, useRef, useState } from "react";
import { getDaemonToken, daemonUrl as resolveDaemonUrl } from "../lib/daemonFetch";

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
  /**
   * What the user is looking at — file tree, open file, pinned notes.
   *
   * Sent on connect and whenever it changes. A host that omits it gets an
   * assistant that knows nothing about the open project, which is how this
   * shipped: the typed chat path injected all of this and the voice path
   * injected none of it.
   */
  context?: string;
  /**
   * Absolute path of the open project, when the host has one.
   *
   * This is what lets a spoken turn *look* at a file rather than answer from
   * whatever the host happened to preload. The daemon jails its read-only
   * tools to this directory and refuses the tools entirely without it — a
   * model naming any path it likes is not a feature.
   */
  workspaceRoot?: string | null;
  /**
   * Whether the feature is enabled at all. Defaults to `false`.
   *
   * Checked here as well as in the UI: hiding a control is not the same as
   * refusing to open a microphone, and this hook is what actually opens one.
   */
  enabled?: boolean;
  /**
   * A *completed* turn, for the host's own chat log.
   *
   * Fires once for the user (on transcription) and once for the assistant
   * (with the whole reply). Deliberately not per sentence: `speaking` fires
   * per sentence because that is what drives streaming TTS, and a host that
   * appended each one rendered a two-sentence answer as two chat bubbles.
   * Live, sentence-by-sentence text is on `turns`.
   */
  onTurn?: (turn: DuplexTurn) => void;
  /**
   * Show a file to the user, by absolute path.
   *
   * Providing this is what tells the daemon the assistant may offer to open
   * files at all — the capability is declared from the presence of the handler,
   * not configured separately, so the two cannot drift apart. A host without an
   * editor (VibeDesk, VibeAIChat) leaves it out and the tool is never
   * advertised, rather than being offered and silently doing nothing.
   */
  onOpenFile?: (path: string) => void;
}

export interface UseVoiceDuplex {
  state: DuplexState;
  turns: DuplexTurn[];
  /**
   * Something the user should know that did not stop the conversation — the
   * configured speech engine failing to start, so the platform voice is
   * speaking instead. A working fallback reported as an error reads as a broken
   * feature; reported as nothing at all, it is a voice that silently is not the
   * one that was configured.
   */
  notice: string | null;
  /**
   * What the assistant is doing during a pause — "Reading README.md".
   *
   * A spoken turn that stops to look at a file is several seconds of silence,
   * and silence is what a broken microphone sounds like. Cleared by the next
   * state change, so it never outlives the pause it explains.
   */
  activity: string | null;
  latency: DuplexLatency;
  /**
   * A change the assistant wants to make, waiting on the user.
   *
   * Spoken *and* shown: the question is read aloud because the user may not be
   * looking, and answered by a click because agreeing to overwrite a file must
   * be deliberate rather than a word the microphone thought it heard.
   */
  approval: { question: string } | null;
  /** Answer the pending question. Anything but `true` is a refusal. */
  respondToApproval: (approved: boolean) => void;
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
/**
 * Turn a start failure into something a user can act on.
 *
 * "connection failed" told nobody anything. Each of these has a different
 * remedy, and the first two are the ones that actually happen.
 */
function describeStartFailure(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  const name = e instanceof Error ? e.name : "";
  if (name === "NotAllowedError" || /permission|denied|not allowed/i.test(msg)) {
    return "Microphone access was denied. Allow it in System Settings → Privacy & Security → Microphone.";
  }
  if (name === "NotFoundError" || /no.*(device|microphone)/i.test(msg)) {
    return "No microphone was found.";
  }
  if (/csp|content security/i.test(msg)) {
    return `Audio capture was blocked by this app's content security policy (${msg}).`;
  }
  return `Could not start voice: ${msg}`;
}

/** A socket message, as far as the visible turn list is concerned. */
export type DuplexEvent =
  | { type: "transcript"; text: string; lang?: string }
  | { type: "speaking"; text: string }
  | { type: "reply"; text: string }
  | { type: "flush" };

/**
 * Fold one socket event into the turn list.
 *
 * Pure, so the property this exists to hold — one reply is one turn — can be
 * tested without a microphone, a socket or an AudioContext.
 *
 * An assistant turn is *open* while it is the last turn and still `interim`.
 * Sentences extend the open turn instead of starting a new one; `reply` closes
 * it with the model's own text. `speaking` fires per sentence because that is
 * what drives streaming TTS, and appending each one rendered a two-sentence
 * answer as two chat bubbles.
 */
export function reduceTurns(turns: readonly DuplexTurn[], ev: DuplexEvent): DuplexTurn[] {
  const last = turns[turns.length - 1];
  const open = last?.role === "assistant" && last.interim === true ? last : null;
  const kept = open ? turns.slice(0, -1) : turns.slice();

  switch (ev.type) {
    case "transcript":
      return [...turns, { role: "user", text: ev.text, lang: ev.lang }];

    case "speaking":
      return open
        ? [...kept, { ...open, text: `${open.text} ${ev.text}`.trim() }]
        : [...turns, { role: "assistant", text: ev.text, interim: true }];

    case "reply": {
      // The model's own text, not the sentences glued back together — the
      // splitter consumes the whitespace it splits on. An empty reply keeps
      // whatever was already spoken rather than blanking the turn.
      const text = ev.text.trim() || open?.text.trim() || "";
      return text ? [...kept, { role: "assistant", text }] : kept;
    }

    case "flush":
      // Barge-in: what was being said was cut off mid-word. Keep it — the
      // assistant did say it — but close it so the next turn's sentences do
      // not land inside it.
      return open ? [...kept, { ...open, interim: false }] : kept;

    default: {
      const never: never = ev;
      return never;
    }
  }
}

export function duplexSupported(): boolean {
  return (
    typeof AudioWorkletNode !== "undefined" &&
    typeof WebSocket !== "undefined" &&
    !!navigator.mediaDevices?.getUserMedia
  );
}

export function useVoiceDuplex(opts: UseVoiceDuplexOptions): UseVoiceDuplex {
  const [state, setState] = useState<DuplexState>({ status: "idle" });
  const [activity, setActivity] = useState<string | null>(null);
  const [approval, setApproval] = useState<{ question: string } | null>(null);
  const [turns, setTurns] = useState<DuplexTurn[]>([]);
  const [latency, setLatency] = useState<DuplexLatency>({});
  const [notice, setNotice] = useState<string | null>(null);
  /**
   * Whether a socket is open, tracked separately from `state`.
   *
   * The daemon reports a turn-level failure — no speech engine, a provider
   * error, a reply that was all reasoning — as `error` without closing the
   * conversation. Deriving `active` from the status alone then told the button
   * the conversation had stopped, so it offered "start" on a socket that was
   * still open, `start` returned early because `ws.current` was set, and there
   * was no longer any control that could stop it. One bad turn stranded the
   * microphone open with no way back.
   */
  const [connected, setConnected] = useState(false);

  const ws = useRef<WebSocket | null>(null);
  /// Read by the capture callback, which is created before the socket opens.
  const sockRef = useRef<WebSocket | null>(null);
  const capturePath = useRef<"worklet" | "scriptprocessor" | null>(null);
  const capCtx = useRef<AudioContext | null>(null);
  const playCtx = useRef<AudioContext | null>(null);
  const stream = useRef<MediaStream | null>(null);
  const nextAt = useRef(0);
  const live = useRef<AudioBufferSourceNode[]>([]);
  const onTurn = useRef(opts.onTurn);
  onTurn.current = opts.onTurn;
  const onOpenFile = useRef(opts.onOpenFile);
  onOpenFile.current = opts.onOpenFile;
  /// Read when the socket opens, which happens after `start` was created.
  const contextRef = useRef(opts.context);
  contextRef.current = opts.context;
  const rootRef = useRef(opts.workspaceRoot);
  rootRef.current = opts.workspaceRoot;

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

  /// Release every resource. Safe to call on a half-started attempt.
  const teardown = useCallback(() => {
    flush();
    setConnected(false);
    ws.current?.close();
    ws.current = null;
    sockRef.current = null;
    stream.current?.getTracks().forEach((t) => t.stop());
    stream.current = null;
    void capCtx.current?.close().catch(() => {});
    void playCtx.current?.close().catch(() => {});
    capCtx.current = null;
    playCtx.current = null;
    capturePath.current = null;
  }, [flush]);

  const stop = useCallback(() => {
    teardown();
    setState({ status: "idle" });
  }, [teardown]);

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
    if (opts.enabled === false) {
      setState({ status: "error", message: "Voice is turned off." });
      return;
    }
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

      // Capture, two ways. An AudioWorklet is the right tool — it runs off the
      // main thread — but its module is fetched under `script-src`, and the
      // shells ship `script-src 'self'`, which rejects a blob: URL outright
      // ("Not allowed by CSP", measured). `worker-src 'self' blob:` does not
      // cover worklets. Rather than depend on every host's CSP, fall back to a
      // ScriptProcessorNode, which needs no module fetch at all.
      const src = cap.createMediaStreamSource(s);
      // A capture node with no downstream connection is not pulled by some
      // engines, so both paths terminate in a muted gain.
      const mute = cap.createGain();
      mute.gain.value = 0;
      mute.connect(cap.destination);

      const onPcm = (pcm: Int16Array<ArrayBufferLike>) => {
        if (sockRef.current?.readyState !== WebSocket.OPEN) return;
        // A transferred worklet buffer is typed `ArrayBufferLike`, which admits
        // SharedArrayBuffer and so is not assignable to a WebSocket payload.
        // It cannot be shared here — both producers allocate a plain
        // Int16Array — so narrow rather than copy 4 KB per frame.
        sockRef.current.send(pcm as Int16Array<ArrayBuffer>);
      };

      let usedWorklet = false;
      try {
        const url = URL.createObjectURL(
          new Blob([CAPTURE_WORKLET], { type: "application/javascript" }),
        );
        await cap.audioWorklet.addModule(url);
        URL.revokeObjectURL(url);
        const node = new AudioWorkletNode(cap, "cap");
        src.connect(node);
        node.connect(mute);
        node.port.onmessage = (e) => onPcm(e.data as Int16Array);
        usedWorklet = true;
      } catch {
        // Deprecated, and still the only capture path that works under a
        // `script-src 'self'` CSP. 2048 frames at 16 kHz is 128 ms — well
        // inside the 600 ms the daemon needs to detect end of turn.
        const sp = cap.createScriptProcessor(2048, 1, 1);
        sp.onaudioprocess = (e) => {
          const ch = e.inputBuffer.getChannelData(0);
          const pcm = new Int16Array(ch.length);
          for (let i = 0; i < ch.length; i++) {
            const v = Math.max(-1, Math.min(1, ch[i]));
            pcm[i] = v < 0 ? v * 0x8000 : v * 0x7fff;
          }
          onPcm(pcm);
        };
        src.connect(sp);
        sp.connect(mute);
      }
      capturePath.current = usedWorklet ? "worklet" : "scriptprocessor";

      // Resolve late: the daemon rotates its token on every restart, and the
      // shells restart it themselves. A socket cannot retry a 401 the way
      // `daemonFetch` does — the handshake either authenticates or it doesn't —
      // so the token is read fresh here and never from a mount-time cache.
      const token = opts.token || (await getDaemonToken(true)) || "";
      const base = (opts.daemonUrl ?? (await resolveDaemonUrl())).replace(/^http/, "ws");
      const q = new URLSearchParams({ token });
      if (opts.provider) q.set("provider", opts.provider);
      if (opts.model) q.set("model", opts.model);
      if (opts.language) q.set("language", opts.language);
      if (opts.voice) q.set("voice", opts.voice);
      const sock = new WebSocket(`${base}/ws/voice/duplex?${q}`);
      sock.binaryType = "arraybuffer";
      ws.current = sock;
      sockRef.current = sock;

      // Before the first turn, not after it: a question asked in the first
      // two seconds is the one most likely to be about what is on screen.
      sock.onopen = () => {
        setConnected(true);
        const c = contextRef.current?.trim();
        if (c) sock.send(JSON.stringify({ type: "set_context", context: c }));
        const r = rootRef.current;
        if (r) sock.send(JSON.stringify({ type: "set_workspace", root: r }));
        // What this host can do, as opposed to what it has open. The daemon
        // only teaches the assistant a tool the client answered for, so a shell
        // with no editor never offers to open a file it cannot show.
        if (onOpenFile.current) {
          sock.send(JSON.stringify({ type: "set_capabilities", open_file: true }));
        }
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
            setActivity(null);
            break;
          // The daemon looked something up. Say so — see `activity`.
          case "tool":
            setActivity(typeof m.text === "string" ? m.text : null);
            break;
          // The one thing the daemon asks the *host* to do rather than doing
          // itself. It has already resolved the path against the workspace and
          // confirmed the file exists, so an unknown action is a version skew,
          // not a bad path — ignore it rather than guessing.
          case "ui":
            if (m.action === "open_file" && typeof m.path === "string") {
              onOpenFile.current?.(m.path);
            }
            break;
          case "approval_request":
            setApproval({ question: String(m.question ?? "May I make this change?") });
            break;
          // Answered, timed out, or the turn was abandoned — either way the
          // question is no longer live and must leave the screen.
          case "approval_resolved":
            setApproval(null);
            break;
          case "flush":
            flush();
            setTurns((t) => reduceTurns(t, { type: "flush" }));
            break;
          case "transcript": {
            setTurns((t) => reduceTurns(t, { type: "transcript", text: m.text, lang: m.lang }));
            setLatency((l) => ({ ...l, asrMs: m.asr_ms }));
            onTurn.current?.({ role: "user", text: m.text, lang: m.lang });
            break;
          }
          case "speaking":
            // Live text only. The host's chat log hears about the turn once,
            // on `reply` — see the `onTurn` contract.
            setTurns((t) => reduceTurns(t, { type: "speaking", text: m.text }));
            break;
          case "latency":
            setLatency((l) => ({ ...l, firstAudioMs: m.first_audio_ms }));
            break;
          case "reply":
            setTurns((t) => reduceTurns(t, { type: "reply", text: m.text }));
            setLatency({
              asrMs: m.asr_ms,
              llmTtftMs: m.llm_ttft_ms,
              firstAudioMs: m.first_audio_ms ?? undefined,
              totalMs: m.total_ms,
            });
            if (m.text?.trim()) onTurn.current?.({ role: "assistant", text: m.text });
            break;
          case "notice":
            // Worth telling the user, and *not* a failure — a configured speech
            // engine that did not start, so the platform voice is speaking
            // instead. Deliberately not `setState(error)`: that is a terminal
            // session state, and the conversation is working.
            setNotice(m.message);
            break;
          case "error":
            setState({ status: "error", message: m.message });
            break;
        }
      };
      sock.onerror = () => {
        teardown();
        setState({
          status: "error",
          message:
            "Could not reach the daemon's voice route. Check the daemon is running and " +
            "recent enough to serve /ws/voice/duplex.",
        });
      };
      sock.onclose = () => {
        if (ws.current === sock) stop();
      };
    } catch (e) {
      // Release everything before reporting. A half-started attempt used to
      // keep the microphone open, which then made *push-to-talk* fail too —
      // one failure taking out a feature that was working.
      teardown();
      setState({
        status: "error",
        message: describeStartFailure(e),
      });
    }
  }, [opts.daemonUrl, opts.token, opts.provider, opts.model, opts.language, opts.voice,
      opts.enabled, enqueue, flush, teardown]);

  const send = useCallback((v: unknown) => {
    if (ws.current?.readyState === WebSocket.OPEN) ws.current.send(JSON.stringify(v));
  }, []);

  // Following the workspace while the mic stays open: switching project or
  // file mid-conversation must not leave the assistant answering about the
  // last one. An empty block clears it rather than pinning what is now stale.
  useEffect(() => {
    send({ type: "set_context", context: opts.context ?? "" });
  }, [opts.context, send]);

  // Same contract for the root: switching project mid-conversation must move
  // the tools with it, and closing one must take them away rather than leave
  // them pointed at a directory the user has walked away from.
  useEffect(() => {
    send({ type: "set_workspace", root: opts.workspaceRoot ?? "" });
  }, [opts.workspaceRoot, send]);

  useEffect(() => () => teardown(), [teardown]);

  // Switching the preference off mid-conversation closes the microphone. A
  // disabled feature that keeps holding the device is the failure this whole
  // opt-in exists to prevent.
  useEffect(() => {
    if (opts.enabled === false && ws.current) stop();
  }, [opts.enabled, stop]);

  const respondToApproval = useCallback(
    (approved: boolean) => {
      setApproval(null);
      send({ type: "approval", approved });
    },
    [send],
  );

  return {
    state,
    turns,
    notice,
    activity,
    approval,
    respondToApproval,
    latency,
    supported: duplexSupported(),
    // A failed *start* leaves no socket, so `connected` is false and the button
    // correctly offers to start again. A failed *turn* keeps the socket, and
    // the only useful control is the one that stops it.
    active: connected || (state.status !== "idle" && state.status !== "error"),
    start,
    stop,
    setVoice: (id) => send({ type: "set_voice", id }),
    setLanguage: (lang) => send({ type: "set_language", lang }),
  };
}
