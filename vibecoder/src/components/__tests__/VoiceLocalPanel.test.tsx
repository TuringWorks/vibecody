/**
 * Tests for VoiceLocalPanel — the panel whose buttons used to be wired to
 * nothing.
 *
 * Every case here pins a control to the command it must reach: the panel
 * shipped with a Download button that had no `onClick` and a record button
 * calling a stub that returned an empty transcript, so it rendered as a working
 * feature while doing nothing at all.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

type EventCallback = (event: { payload: unknown }) => void;
const eventListeners: Record<string, EventCallback[]> = {};
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, cb: EventCallback) => {
    (eventListeners[event] ??= []).push(cb);
    return Promise.resolve(() => {
      const idx = eventListeners[event].indexOf(cb);
      if (idx >= 0) eventListeners[event].splice(idx, 1);
    });
  },
}));

// The mic itself is the shared hook's business and has its own BDD suite;
// stub it so these tests are about the panel's wiring.
const voiceHook = {
  toggle: vi.fn(),
  supported: true,
  isListening: false,
  isTranscribing: false,
  interimText: '',
  error: null as string | null,
  clearError: vi.fn(),
  state: { status: 'idle' as const },
};
vi.mock('@vibe/shared/voice/useVoiceInput', () => ({
  useVoiceInput: (opts: { onTranscript: (t: string) => void }) => {
    transcriptSink = opts.onTranscript;
    return voiceHook;
  },
}));
let transcriptSink: (t: string) => void = () => {};

const transcriberOptions = vi.fn();
vi.mock('@vibe/shared/voice/transcribers', () => ({
  tauriTranscriber: (url: unknown, opts: unknown) => {
    transcriberOptions(url, opts);
    return vi.fn();
  },
}));

vi.mock('../ExperimentalBadge', () => ({
  ExperimentalBadge: () => <div data-testid="experimental-badge" />,
}));

import { VoiceLocalPanel } from '../VoiceLocalPanel';

const MODELS = [
  { id: 'tiny', label: 'Whisper Tiny', size_mb: 75, downloaded: false, selected: false, path: null },
  { id: 'base', label: 'Whisper Base', size_mb: 142, downloaded: true, selected: true, path: '/models/ggml-base.bin' },
];

const READY_STATUS = {
  cloud_stt_configured: true,
  local_model: 'base',
  local_model_downloaded: true,
  prefer_local: false,
  language: 'en',
  whisper_cpp_installed: true,
  whisper_python_installed: false,
  ffmpeg_installed: true,
};

/** Default happy-path backend. Individual tests override single commands. */
function backend(overrides: Record<string, unknown> = {}) {
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd in overrides) {
      const value = overrides[cmd];
      return typeof value === 'function' ? (value as () => unknown)() : Promise.resolve(value);
    }
    switch (cmd) {
      case 'voice_list_models':
        return Promise.resolve(MODELS);
      case 'voice_status':
        return Promise.resolve(READY_STATUS);
      case 'voice_get_settings':
        return Promise.resolve({ local_model: 'base', language: 'en', prefer_local: false });
      default:
        return Promise.resolve(null);
    }
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  transcriberOptions.mockReset();
  voiceHook.toggle.mockReset();
  for (const key of Object.keys(eventListeners)) delete eventListeners[key];
  backend();
});

describe('Given the Models tab', () => {
  it('When Download is clicked, Then the download command runs for that model', async () => {
    render(<VoiceLocalPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Models' }));
    const button = await screen.findByRole('button', { name: 'Download' });

    fireEvent.click(button);

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('voice_download_model', { id: 'tiny' }),
    );
  });

  it('Then the model label is rendered once, not prefixed with its own backend', async () => {
    render(<VoiceLocalPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Models' }));

    expect(await screen.findByText('Whisper Tiny')).toBeInTheDocument();
    expect(screen.queryByText(/whisper-Whisper/)).not.toBeInTheDocument();
  });

  it('When a progress event arrives, Then measured bytes are shown', async () => {
    backend({ voice_download_model: () => new Promise(() => {}) });
    render(<VoiceLocalPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Models' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Download' }));
    await waitFor(() => expect(eventListeners['voice-model-progress']?.length).toBe(1));

    eventListeners['voice-model-progress'][0]({
      payload: { id: 'tiny', downloaded_bytes: 37_500_000, total_bytes: 75_000_000 },
    });

    expect(await screen.findByText(/50% — 38 MB \/ 75 MB/)).toBeInTheDocument();
  });

  it('When the server sends no Content-Length, Then the total is called unknown, not 0%', async () => {
    backend({ voice_download_model: () => new Promise(() => {}) });
    render(<VoiceLocalPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Models' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Download' }));
    await waitFor(() => expect(eventListeners['voice-model-progress']?.length).toBe(1));

    eventListeners['voice-model-progress'][0]({
      payload: { id: 'tiny', downloaded_bytes: 1_000_000, total_bytes: 0 },
    });

    expect(await screen.findByText(/total size unknown/)).toBeInTheDocument();
    expect(screen.queryByText(/0%/)).not.toBeInTheDocument();
  });

  it('When a download fails, Then the failure is shown instead of being swallowed', async () => {
    backend({ voice_download_model: () => Promise.reject('Download failed: HTTP 404') });
    render(<VoiceLocalPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Models' }));

    fireEvent.click(await screen.findByRole('button', { name: 'Download' }));

    expect(await screen.findByText(/Could not download Whisper Tiny.*404/)).toBeInTheDocument();
  });

  it('When Select is clicked on a downloaded model, Then the selection is persisted', async () => {
    backend({
      voice_list_models: [
        { ...MODELS[1], selected: false },
        { ...MODELS[0], downloaded: true, path: '/models/ggml-tiny.bin' },
      ],
    });
    render(<VoiceLocalPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Models' }));

    fireEvent.click((await screen.findAllByRole('button', { name: 'Select' }))[0]);

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('voice_select_model', { id: 'base' }),
    );
  });
});

describe('Given the setup banner', () => {
  it('When no whisper runtime is installed, Then a downloaded model is not called ready', async () => {
    backend({
      voice_status: {
        ...READY_STATUS,
        cloud_stt_configured: false,
        whisper_cpp_installed: false,
        whisper_python_installed: false,
      },
    });
    render(<VoiceLocalPanel />);

    expect(await screen.findByText('Transcription cannot run yet')).toBeInTheDocument();
    expect(screen.getByText(/no whisper runtime on PATH/)).toBeInTheDocument();
  });

  it('When everything is in place, Then no blocker is claimed', async () => {
    render(<VoiceLocalPanel />);
    await screen.findByText('Click to start recording');

    expect(screen.queryByText('Transcription cannot run yet')).not.toBeInTheDocument();
  });

  it('When the daemon is unreachable, Then that is stated rather than logged silently', async () => {
    backend({ voice_status: () => Promise.reject('connection refused on port 7878') });
    render(<VoiceLocalPanel />);

    expect(await screen.findByText('Cannot reach the daemon')).toBeInTheDocument();
    expect(screen.getByText(/connection refused on port 7878/)).toBeInTheDocument();
  });
});

describe('Given the Record tab', () => {
  it('When the mic button is clicked, Then the recorder is toggled', async () => {
    render(<VoiceLocalPanel />);
    await screen.findByText('Click to start recording');

    fireEvent.click(screen.getByRole('button', { name: 'Start recording' }));

    expect(voiceHook.toggle).toHaveBeenCalled();
  });

  it('When a transcript arrives, Then it lands in the panel and its history', async () => {
    render(<VoiceLocalPanel />);
    await screen.findByText('Click to start recording');

    transcriptSink('  hello there  ');

    expect(await screen.findByText('hello there')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'History' }));
    expect(screen.getByText('hello there')).toBeInTheDocument();
  });
});

describe('Given the Config tab', () => {
  it('When prefer-local is toggled, Then it is persisted and reaches the transcriber', async () => {
    render(<VoiceLocalPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Config' }));

    fireEvent.click(await screen.findByRole('button', { name: 'OFF' }));

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('voice_set_settings', {
        language: null,
        preferLocal: true,
      }),
    );
    await waitFor(() =>
      expect(transcriberOptions).toHaveBeenLastCalledWith(undefined, {
        preferLocal: true,
        language: 'en',
      }),
    );
  });

  it('When a language is chosen, Then it is persisted', async () => {
    render(<VoiceLocalPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'Config' }));

    fireEvent.change(await screen.findByRole('combobox'), { target: { value: 'de' } });

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('voice_set_settings', {
        language: 'de',
        preferLocal: null,
      }),
    );
  });
});
