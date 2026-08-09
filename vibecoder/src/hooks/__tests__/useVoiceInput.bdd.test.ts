/**
 * BDD tests for the shared `useVoiceInput` hook
 * (packages/vibe-ui-shared/src/voice/useVoiceInput.ts).
 *
 * The hook lives in the shared package but is exercised here because VibeCoder
 * is the app that carries a vitest + jsdom setup. It is the same hook VibeDesk
 * and VibeAIChat mount, so a regression caught here is caught for all three.
 *
 * Scenarios:
 *  1. Initial state: idle, not listening, not transcribing, no interim, no error
 *  2. Web Speech available: toggle() starts recognition and sets isListening
 *  3. onend returns to idle and clears interim text
 *  4. onresult forwards final results to onTranscript
 *  5. onresult exposes non-final results as interimText
 *  6. onerror surfaces an actionable message (the old hook swallowed these)
 *  7. `no-speech` / `aborted` return to idle without an error
 *  8. toggle() while listening stops recognition
 *  9. No Web Speech: toggle() falls back to MediaRecorder
 * 10. On stop, the recorded blob is handed to the supplied transcriber
 * 11. A blob below the silence floor is not transcribed
 * 12. isTranscribing is true in flight and false afterwards
 * 13. A failing transcriber surfaces its message rather than failing silently
 * 14. A denied microphone surfaces a permission message
 * 15. Unmounting aborts in-progress recognition
 * 16. `supported` is false with neither engine available
 */

import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

import { useVoiceInput } from '@vibe/shared/voice/useVoiceInput';

// ── Mock SpeechRecognition ─────────────────────────────────────────────────────

type SpeechHandler = (e?: unknown) => void;
class MockSpeechRecognition {
  continuous = false;
  interimResults = false;
  lang = '';
  maxAlternatives = 1;

  onresult: SpeechHandler | null = null;
  onerror: SpeechHandler | null = null;
  onend: SpeechHandler | null = null;

  started = false;
  stopped = false;
  aborted = false;

  start() { this.started = true; MockSpeechRecognition.lastInstance = this; }
  stop()  { this.stopped = true; this.onend?.(); }
  abort() { this.aborted = true; }

  simulateFinalResult(transcript: string) {
    this.onresult?.({
      resultIndex: 0,
      results: { length: 1, 0: { isFinal: true, 0: { transcript } } },
    });
  }

  simulateInterimResult(transcript: string) {
    this.onresult?.({
      resultIndex: 0,
      results: { length: 1, 0: { isFinal: false, 0: { transcript } } },
    });
  }

  simulateError(error = 'not-allowed') { this.onerror?.({ error }); }
  simulateEnd() { this.onend?.(); }

  static lastInstance: MockSpeechRecognition | null = null;
  static reset() { MockSpeechRecognition.lastInstance = null; }
}

// ── Mock MediaRecorder ─────────────────────────────────────────────────────────

type MediaRecorderHandler = (e?: unknown) => void;
class MockMediaRecorder {
  mimeType: string;
  ondataavailable: MediaRecorderHandler | null = null;
  onstop: MediaRecorderHandler | null = null;
  onerror: MediaRecorderHandler | null = null;
  started = false;
  stopped = false;
  /** Bytes the fake recording produces — drives the silence-floor scenario. */
  static payloadBytes = 4096;

  constructor(_stream: MediaStream, opts: { mimeType?: string } = {}) {
    this.mimeType = opts.mimeType ?? 'audio/webm';
    MockMediaRecorder.lastInstance = this;
  }

  start() { this.started = true; }
  stop()  {
    this.stopped = true;
    const blob = new Blob(['x'.repeat(MockMediaRecorder.payloadBytes)], { type: this.mimeType });
    this.ondataavailable?.({ data: blob });
    this.onstop?.();
  }

  static isTypeSupported = vi.fn().mockReturnValue(true);
  static lastInstance: MockMediaRecorder | null = null;
  static reset() {
    MockMediaRecorder.lastInstance = null;
    MockMediaRecorder.payloadBytes = 4096;
  }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/**
 * Set the Web Speech constructor on the real `window`.
 *
 * Deliberately *not* `vi.stubGlobal('window', {...window})`: spreading a Window
 * copies only own enumerable properties, so `document` — a prototype getter —
 * is lost, and every `waitFor` in this file then dies with "Expected container
 * to be an Element". Mutate the two properties under test instead.
 */
type VoiceWindow = Window & {
  SpeechRecognition?: unknown;
  webkitSpeechRecognition?: unknown;
};

function setSpeechRecognition(ctor: unknown) {
  (window as VoiceWindow).SpeechRecognition = ctor;
  (window as VoiceWindow).webkitSpeechRecognition = undefined;
}

function setupSpeechRecognition() {
  MockSpeechRecognition.reset();
  setSpeechRecognition(MockSpeechRecognition);
}

function setupNoSpeechRecognition(getUserMedia?: () => Promise<unknown>) {
  MockMediaRecorder.reset();
  setSpeechRecognition(undefined);
  vi.stubGlobal('MediaRecorder', MockMediaRecorder);
  vi.stubGlobal('navigator', {
    mediaDevices: {
      getUserMedia:
        getUserMedia ??
        vi.fn().mockResolvedValue({ getTracks: () => [{ stop: vi.fn() }] }),
    },
  });
}

/** Default transcriber: resolves with a fixed transcript. */
function stubTranscriber(text = 'transcribed text') {
  return vi.fn().mockResolvedValue(text);
}

beforeEach(() => {
  vi.clearAllMocks();
  setupSpeechRecognition();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  // `setSpeechRecognition` writes to the real window, so unstubAllGlobals
  // won't undo it — clear it here or the next file inherits a mock engine.
  delete (window as VoiceWindow).SpeechRecognition;
  delete (window as VoiceWindow).webkitSpeechRecognition;
  MockSpeechRecognition.reset();
  MockMediaRecorder.reset();
});

// ── Scenario 1: Initial state ─────────────────────────────────────────────────

describe('Given a fresh useVoiceInput hook', () => {
  it('When it mounts, Then it is idle', () => {
    const { result } = renderHook(() =>
      useVoiceInput({ onTranscript: vi.fn(), transcribe: stubTranscriber() }),
    );
    expect(result.current.state.status).toBe('idle');
    expect(result.current.isListening).toBe(false);
    expect(result.current.isTranscribing).toBe(false);
    expect(result.current.interimText).toBe('');
    expect(result.current.error).toBeNull();
  });

  it('When Web Speech exists, Then it reports itself supported', () => {
    const { result } = renderHook(() => useVoiceInput({ onTranscript: vi.fn() }));
    expect(result.current.supported).toBe(true);
  });
});

// ── Scenarios 2-8: Web Speech path ────────────────────────────────────────────

describe('Given the Web Speech API is available', () => {
  it('When toggle() is called, Then recognition starts and isListening is true', () => {
    const { result } = renderHook(() => useVoiceInput({ onTranscript: vi.fn() }));
    act(() => result.current.toggle());
    expect(MockSpeechRecognition.lastInstance?.started).toBe(true);
    expect(result.current.isListening).toBe(true);
  });

  it('When recognition ends, Then the hook returns to idle', () => {
    const { result } = renderHook(() => useVoiceInput({ onTranscript: vi.fn() }));
    act(() => result.current.toggle());
    act(() => MockSpeechRecognition.lastInstance!.simulateEnd());
    expect(result.current.isListening).toBe(false);
    expect(result.current.interimText).toBe('');
  });

  it('When a final result arrives, Then onTranscript receives it', () => {
    const onTranscript = vi.fn();
    const { result } = renderHook(() => useVoiceInput({ onTranscript }));
    act(() => result.current.toggle());
    act(() => MockSpeechRecognition.lastInstance!.simulateFinalResult('hello world'));
    expect(onTranscript).toHaveBeenCalledWith('hello world');
  });

  it('When an interim result arrives, Then it appears as interimText and not as a transcript', () => {
    const onTranscript = vi.fn();
    const { result } = renderHook(() => useVoiceInput({ onTranscript }));
    act(() => result.current.toggle());
    act(() => MockSpeechRecognition.lastInstance!.simulateInterimResult('partial'));
    expect(result.current.interimText).toBe('partial');
    expect(onTranscript).not.toHaveBeenCalled();
  });

  it('When permission is denied, Then an actionable error is exposed', () => {
    // The pre-shared VibeCoder hook swallowed this, leaving a mic button that
    // silently did nothing — the single most confusing failure mode here.
    const { result } = renderHook(() => useVoiceInput({ onTranscript: vi.fn() }));
    act(() => result.current.toggle());
    act(() => MockSpeechRecognition.lastInstance!.simulateError('not-allowed'));
    expect(result.current.isListening).toBe(false);
    expect(result.current.error).toMatch(/microphone access was denied/i);
  });

  it.each(['no-speech', 'aborted'])(
    'When recognition reports %s, Then it returns to idle without an error',
    (code) => {
      const { result } = renderHook(() => useVoiceInput({ onTranscript: vi.fn() }));
      act(() => result.current.toggle());
      act(() => MockSpeechRecognition.lastInstance!.simulateError(code));
      expect(result.current.error).toBeNull();
      expect(result.current.state.status).toBe('idle');
    },
  );

  it('When toggle() is called while listening, Then recognition stops', () => {
    const { result } = renderHook(() => useVoiceInput({ onTranscript: vi.fn() }));
    act(() => result.current.toggle());
    const first = MockSpeechRecognition.lastInstance!;
    act(() => result.current.toggle());
    expect(first.stopped).toBe(true);
    expect(result.current.isListening).toBe(false);
  });

  it('When the component unmounts mid-recording, Then recognition is aborted', () => {
    const { result, unmount } = renderHook(() => useVoiceInput({ onTranscript: vi.fn() }));
    act(() => result.current.toggle());
    const instance = MockSpeechRecognition.lastInstance!;
    unmount();
    expect(instance.aborted).toBe(true);
  });
});

// ── Scenarios 9-14: MediaRecorder fallback ────────────────────────────────────

describe('Given the Web Speech API is unavailable', () => {
  beforeEach(() => setupNoSpeechRecognition());

  it('When toggle() is called, Then MediaRecorder starts', async () => {
    const { result } = renderHook(() =>
      useVoiceInput({ onTranscript: vi.fn(), transcribe: stubTranscriber() }),
    );
    await act(async () => result.current.toggle());
    expect(MockMediaRecorder.lastInstance?.started).toBe(true);
    expect(result.current.isListening).toBe(true);
  });

  it('When recording stops, Then the blob is handed to the transcriber and the text forwarded', async () => {
    const transcribe = stubTranscriber('spoken words');
    const onTranscript = vi.fn();
    const { result } = renderHook(() => useVoiceInput({ onTranscript, transcribe }));

    await act(async () => result.current.toggle());
    await act(async () => {
      MockMediaRecorder.lastInstance!.stop();
    });

    await waitFor(() => expect(onTranscript).toHaveBeenCalledWith('spoken words'));
    expect(transcribe).toHaveBeenCalledTimes(1);
    expect(transcribe.mock.calls[0][0]).toBeInstanceOf(Blob);
  });

  it('When the clip is below the silence floor, Then nothing is transcribed', async () => {
    MockMediaRecorder.payloadBytes = 32;
    const transcribe = stubTranscriber();
    const { result } = renderHook(() =>
      useVoiceInput({ onTranscript: vi.fn(), transcribe }),
    );

    await act(async () => result.current.toggle());
    await act(async () => {
      MockMediaRecorder.lastInstance!.stop();
    });

    expect(transcribe).not.toHaveBeenCalled();
    expect(result.current.state.status).toBe('idle');
  });

  it('When transcription is in flight, Then isTranscribing is true and then false', async () => {
    let release!: (text: string) => void;
    const transcribe = vi.fn(
      () => new Promise<string>((resolve) => { release = resolve; }),
    );
    const { result } = renderHook(() =>
      useVoiceInput({ onTranscript: vi.fn(), transcribe }),
    );

    await act(async () => result.current.toggle());
    await act(async () => {
      MockMediaRecorder.lastInstance!.stop();
    });
    await waitFor(() => expect(result.current.isTranscribing).toBe(true));

    await act(async () => {
      release('done');
    });
    await waitFor(() => expect(result.current.isTranscribing).toBe(false));
  });

  it('When the transcriber rejects, Then its message is surfaced', async () => {
    const transcribe = vi
      .fn()
      .mockRejectedValue(new Error('Groq API key not set — add it in Settings'));
    const { result } = renderHook(() =>
      useVoiceInput({ onTranscript: vi.fn(), transcribe }),
    );

    await act(async () => result.current.toggle());
    await act(async () => {
      MockMediaRecorder.lastInstance!.stop();
    });

    await waitFor(() => expect(result.current.error).toMatch(/Groq API key not set/));
    expect(result.current.isTranscribing).toBe(false);
  });

  it('When microphone permission is denied, Then a permission message is surfaced', async () => {
    setupNoSpeechRecognition(vi.fn().mockRejectedValue(new Error('NotAllowedError')));
    const { result } = renderHook(() =>
      useVoiceInput({ onTranscript: vi.fn(), transcribe: stubTranscriber() }),
    );

    await act(async () => result.current.toggle());

    await waitFor(() => expect(result.current.error).toMatch(/microphone access was denied/i));
    expect(result.current.isListening).toBe(false);
  });

  it('When no transcriber is supplied, Then the hook reports itself unsupported', () => {
    // No Web Speech and no backend means there is nothing to render a button
    // for — better than a button that always errors.
    const { result } = renderHook(() => useVoiceInput({ onTranscript: vi.fn() }));
    expect(result.current.supported).toBe(false);
  });
});
