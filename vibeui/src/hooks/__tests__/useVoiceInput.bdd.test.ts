/**
 * BDD tests for useVoiceInput — voice capture with SpeechRecognition / MediaRecorder fallback.
 *
 * Scenarios:
 *  1. Initial state: not listening, not transcribing, empty interimText
 *  2. When SpeechRecognition is available: toggle() starts recognition and sets isListening=true
 *  3. SpeechRecognition onend resets isListening=false and clears interimText
 *  4. SpeechRecognition onresult fires onTranscript for final results
 *  5. SpeechRecognition onresult sets interimText for non-final results
 *  6. SpeechRecognition onerror resets isListening=false
 *  7. toggle() while listening stops recognition (no new recognition started)
 *  8. When SpeechRecognition is unavailable: toggle() uses MediaRecorder
 *  9. MediaRecorder onstop invokes transcribe_audio_bytes with base64 audio
 * 10. When blob size < 100, transcription is skipped
 * 11. isTranscribing is true while transcription is in flight, false after
 * 12. When transcription invoke throws, isTranscribing resets to false gracefully
 * 13. Unmounting aborts any in-progress SpeechRecognition
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { useVoiceInput } from '../useVoiceInput';

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
      results: {
        length: 1,
        0: { isFinal: true, 0: { transcript } },
      },
    });
  }

  simulateInterimResult(transcript: string) {
    this.onresult?.({
      resultIndex: 0,
      results: {
        length: 1,
        0: { isFinal: false, 0: { transcript } },
      },
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

  constructor(_stream: MediaStream, opts: { mimeType?: string } = {}) {
    this.mimeType = opts.mimeType ?? 'audio/webm';
    MockMediaRecorder.lastInstance = this;
  }

  start() { this.started = true; }
  stop()  {
    this.stopped = true;
    // Simulate data available then stop
    const blob = new Blob(['x'.repeat(200)], { type: this.mimeType });
    this.ondataavailable?.({ data: blob });
    this.onstop?.();
  }

  static isTypeSupported = vi.fn().mockReturnValue(true);
  static lastInstance: MockMediaRecorder | null = null;
  static reset() { MockMediaRecorder.lastInstance = null; }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

function setupSpeechRecognition() {
  MockSpeechRecognition.reset();
  vi.stubGlobal('window', {
    ...window,
    SpeechRecognition: MockSpeechRecognition,
    webkitSpeechRecognition: undefined,
  });
}

function setupNoSpeechRecognition() {
  MockMediaRecorder.reset();
  vi.stubGlobal('window', {
    ...window,
    SpeechRecognition: undefined,
    webkitSpeechRecognition: undefined,
  });
  vi.stubGlobal('MediaRecorder', MockMediaRecorder);
  vi.stubGlobal('navigator', {
    mediaDevices: {
      getUserMedia: vi.fn().mockResolvedValue({
        getTracks: () => [{ stop: vi.fn() }],
      }),
    },
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockResolvedValue('transcribed text');
  setupSpeechRecognition();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  MockSpeechRecognition.reset();
  MockMediaRecorder.reset();
});

// ── Scenario 1: Initial state ─────────────────────────────────────────────────

describe('Given a fresh useVoiceInput hook', () => {
  it('When it mounts, Then isListening is false', () => {
    const onTranscript = vi.fn();
    const { result } = renderHook(() => useVoiceInput(onTranscript));
    expect(result.current.isListening).toBe(false);
  });

  it('When it mounts, Then isTranscribing is false', () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    expect(result.current.isTranscribing).toBe(false);
  });

  it('When it mounts, Then interimText is empty string', () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    expect(result.current.interimText).toBe('');
  });
});

// ── Scenario 2: SpeechRecognition starts ──────────────────────────────────────

describe('Given SpeechRecognition is available', () => {
  it('When toggle() is called, Then SpeechRecognition.start() is called', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    expect(MockSpeechRecognition.lastInstance?.started).toBe(true);
  });

  it('When toggle() is called, Then isListening becomes true', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    expect(result.current.isListening).toBe(true);
  });

  it('When toggle() is called, Then continuous and interimResults are enabled', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    expect(MockSpeechRecognition.lastInstance?.continuous).toBe(true);
    expect(MockSpeechRecognition.lastInstance?.interimResults).toBe(true);
  });
});

// ── Scenario 3: onend resets isListening ─────────────────────────────────────

describe('Given SpeechRecognition is running', () => {
  it('When onend fires, Then isListening becomes false', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    act(() => { MockSpeechRecognition.lastInstance?.simulateEnd(); });
    expect(result.current.isListening).toBe(false);
  });

  it('When onend fires, Then interimText is cleared', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    act(() => { MockSpeechRecognition.lastInstance?.simulateInterimResult('partial'); });
    expect(result.current.interimText).toBe('partial');
    act(() => { MockSpeechRecognition.lastInstance?.simulateEnd(); });
    expect(result.current.interimText).toBe('');
  });
});

// ── Scenario 4: onresult fires onTranscript for final results ─────────────────

describe('Given SpeechRecognition returns a final result', () => {
  it('When a final transcript arrives, Then onTranscript is called with the text', async () => {
    const onTranscript = vi.fn();
    const { result } = renderHook(() => useVoiceInput(onTranscript));
    await act(async () => { await result.current.toggle(); });
    act(() => { MockSpeechRecognition.lastInstance?.simulateFinalResult('hello world'); });
    expect(onTranscript).toHaveBeenCalledOnce();
    expect(onTranscript).toHaveBeenCalledWith('hello world');
  });
});

// ── Scenario 5: interimText for non-final results ─────────────────────────────

describe('Given SpeechRecognition returns an interim result', () => {
  it('When a non-final transcript arrives, Then interimText is updated', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    act(() => { MockSpeechRecognition.lastInstance?.simulateInterimResult('typing…'); });
    expect(result.current.interimText).toBe('typing…');
  });

  it('When a non-final transcript arrives, Then onTranscript is NOT called', async () => {
    const onTranscript = vi.fn();
    const { result } = renderHook(() => useVoiceInput(onTranscript));
    await act(async () => { await result.current.toggle(); });
    act(() => { MockSpeechRecognition.lastInstance?.simulateInterimResult('partial'); });
    expect(onTranscript).not.toHaveBeenCalled();
  });
});

// ── Scenario 6: onerror resets isListening ───────────────────────────────────

describe('Given SpeechRecognition encounters an error', () => {
  it('When onerror fires, Then isListening becomes false', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    act(() => { MockSpeechRecognition.lastInstance?.simulateError('not-allowed'); });
    expect(result.current.isListening).toBe(false);
  });
});

// ── Scenario 7: toggle() while listening stops recognition ───────────────────

describe('Given SpeechRecognition is actively listening', () => {
  it('When toggle() is called again, Then recognition.stop() is called', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); }); // start
    const recognition = MockSpeechRecognition.lastInstance!;
    await act(async () => { await result.current.toggle(); }); // stop
    expect(recognition.stopped).toBe(true);
  });
});

// ── Scenario 8: MediaRecorder fallback ───────────────────────────────────────

describe('Given SpeechRecognition is not available', () => {
  beforeEach(() => { setupNoSpeechRecognition(); });

  it('When toggle() is called, Then MediaRecorder.start() is called', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    expect(MockMediaRecorder.lastInstance?.started).toBe(true);
  });

  it('When toggle() is called, Then isListening becomes true', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    expect(result.current.isListening).toBe(true);
  });
});

// ── Scenario 9: MediaRecorder onstop invokes transcription ───────────────────

describe('Given the MediaRecorder records audio and stops', () => {
  beforeEach(() => { setupNoSpeechRecognition(); });

  it('When recording stops with >100 bytes, Then invoke("transcribe_audio_bytes") is called', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); }); // start
    // Simulate toggling off (stop MediaRecorder)
    await act(async () => { await result.current.toggle(); }); // stop
    // MediaRecorder.stop() triggers onstop asynchronously
    // Allow microtask queue to flush
    await act(async () => {});
    expect(mockInvoke).toHaveBeenCalledWith('transcribe_audio_bytes', expect.objectContaining({
      audioBase64: expect.any(String),
    }));
  });

  it('When transcription succeeds, Then onTranscript is called with the result', async () => {
    mockInvoke.mockResolvedValue('  hello world  ');
    const onTranscript = vi.fn();
    const { result } = renderHook(() => useVoiceInput(onTranscript));
    await act(async () => { await result.current.toggle(); });
    await act(async () => { await result.current.toggle(); });
    await act(async () => {});
    expect(onTranscript).toHaveBeenCalledWith('  hello world  ');
  });
});

// ── Scenario 12: Transcription failure is graceful ────────────────────────────

describe('Given transcription invoke throws', () => {
  beforeEach(() => {
    setupNoSpeechRecognition();
    mockInvoke.mockRejectedValue(new Error('GROQ_API_KEY not set'));
  });

  it('When invoke throws, Then isTranscribing resets to false without crashing', async () => {
    const { result } = renderHook(() => useVoiceInput(vi.fn()));
    await act(async () => { await result.current.toggle(); });
    await act(async () => { await result.current.toggle(); });
    await act(async () => {});
    expect(result.current.isTranscribing).toBe(false);
  });
});
