import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:record/record.dart';
import 'package:speech_to_text/speech_to_text.dart' as stt;

import 'api_client.dart';

/// What the mic is doing.
///
/// A sum type, not parallel `isListening` / `isTranscribing` / `error` fields:
/// those admit impossible combinations and every widget then has to decide
/// which flag wins. Mirrors `VoiceState` in the shared web hook so the two
/// platforms describe the same states in the same words.
enum VoiceStatus { idle, listening, transcribing, error }

/// Which engine produced (or would produce) a transcript.
enum VoiceEngine {
  /// The platform recogniser — iOS `SFSpeechRecognizer`, Android
  /// `SpeechRecognizer`. Streams partial results; free; usually on-device.
  onDevice,

  /// Recorded clip uploaded to the daemon's `/voice/transcribe`.
  daemon,
}

/// Immutable snapshot handed to the UI on every change.
@immutable
class VoiceSnapshot {
  final VoiceStatus status;

  /// Partial text while listening. Empty in every other state.
  final String partial;

  /// Message for the user. Non-null only when [status] is [VoiceStatus.error].
  final String? error;

  /// Engine backing the current session, once one has been chosen.
  final VoiceEngine? engine;

  const VoiceSnapshot({
    this.status = VoiceStatus.idle,
    this.partial = '',
    this.error,
    this.engine,
  });

  bool get isListening => status == VoiceStatus.listening;
  bool get isTranscribing => status == VoiceStatus.transcribing;
  bool get isBusy => isListening || isTranscribing;
}

/// Microphone input for the VibeMobile composers.
///
/// Two engines, tried in this order:
///
/// 1. **On-device recognition** (`speech_to_text`) — partial results as you
///    speak, no upload, no API key. Unavailable on some devices and on
///    emulators without Google app / speech services.
/// 2. **Record then upload** (`record` → `ApiClient.transcribeAudio`) — the
///    daemon runs whisper locally or falls back to Groq.
///
/// Every failure is reported through [snapshot] with something the user can
/// act on. Nothing here silently no-ops: a mic button that does nothing on tap
/// is the worst outcome of the three.
class VoiceService extends ChangeNotifier {
  VoiceService({required ApiClient api}) : _api = api;

  final ApiClient _api;
  final stt.SpeechToText _speech = stt.SpeechToText();
  final AudioRecorder _recorder = AudioRecorder();

  VoiceSnapshot _snapshot = const VoiceSnapshot();
  VoiceSnapshot get snapshot => _snapshot;

  /// Whether on-device recognition initialised. `null` until first probed —
  /// absent is not the same as unavailable, and the UI shows them differently.
  bool? _onDeviceAvailable;
  bool? get onDeviceAvailable => _onDeviceAvailable;

  String? _recordingPath;
  bool _disposed = false;

  void _emit(VoiceSnapshot next) {
    if (_disposed) return;
    _snapshot = next;
    notifyListeners();
  }

  void _fail(String message) =>
      _emit(VoiceSnapshot(status: VoiceStatus.error, error: message));

  /// Clear an error without starting a new recording.
  void clearError() {
    if (_snapshot.status == VoiceStatus.error) _emit(const VoiceSnapshot());
  }

  /// Probe on-device recognition once. Safe to call repeatedly.
  Future<bool> initOnDevice() async {
    final cached = _onDeviceAvailable;
    if (cached != null) return cached;
    try {
      final ok = await _speech.initialize(
        // Errors and status changes are surfaced through the snapshot, not
        // printed — a debug print is invisible to the person holding the phone.
        onError: (e) => _fail(_describeSpeechError(e.errorMsg)),
        onStatus: (status) {
          if (status == 'done' || status == 'notListening') {
            if (_snapshot.status == VoiceStatus.listening) {
              _emit(const VoiceSnapshot());
            }
          }
        },
      );
      _onDeviceAvailable = ok;
      return ok;
    } catch (_) {
      _onDeviceAvailable = false;
      return false;
    }
  }

  /// Start listening, or stop if already listening.
  ///
  /// [onTranscript] receives each finalised chunk. [baseUrl] and [token]
  /// are only used by the upload fallback; pass them even when on-device
  /// recognition is expected, since the fallback may still be chosen.
  Future<void> toggle({
    required void Function(String text) onTranscript,
    required String baseUrl,
    required String token,
    String? localeId,
    bool preferDaemon = false,
  }) async {
    if (_snapshot.status == VoiceStatus.transcribing) return;

    if (_snapshot.isListening) {
      await _stop(onTranscript: onTranscript, baseUrl: baseUrl, token: token);
      return;
    }

    if (!preferDaemon && await initOnDevice()) {
      await _startOnDevice(onTranscript: onTranscript, localeId: localeId);
      return;
    }
    await _startRecording();
  }

  Future<void> _startOnDevice({
    required void Function(String text) onTranscript,
    String? localeId,
  }) async {
    try {
      await _speech.listen(
        onResult: (result) {
          if (result.finalResult) {
            final text = result.recognizedWords.trim();
            if (text.isNotEmpty) onTranscript(text);
            _emit(const VoiceSnapshot());
          } else {
            _emit(VoiceSnapshot(
              status: VoiceStatus.listening,
              partial: result.recognizedWords,
              engine: VoiceEngine.onDevice,
            ));
          }
        },
        listenOptions: stt.SpeechListenOptions(
          localeId: localeId,
          partialResults: true,
          cancelOnError: true,
        ),
      );
      _emit(const VoiceSnapshot(
        status: VoiceStatus.listening,
        engine: VoiceEngine.onDevice,
      ));
    } catch (e) {
      _fail('Could not start speech recognition: $e');
    }
  }

  Future<void> _startRecording() async {
    if (!await _recorder.hasPermission()) {
      _fail('Microphone access was denied. Allow it in Settings and try again.');
      return;
    }
    try {
      final dir = await getTemporaryDirectory();
      // Timestamped so a second recording can't collide with an upload still
      // reading the first.
      final path =
          '${dir.path}/vibecody-voice-${DateTime.now().millisecondsSinceEpoch}.m4a';
      await _recorder.start(
        // AAC in an m4a container: supported by both platforms' encoders and
        // by ffmpeg on the daemon side, and roughly 10× smaller than WAV.
        const RecordConfig(encoder: AudioEncoder.aacLc, sampleRate: 16000, numChannels: 1),
        path: path,
      );
      _recordingPath = path;
      _emit(const VoiceSnapshot(
        status: VoiceStatus.listening,
        engine: VoiceEngine.daemon,
      ));
    } catch (e) {
      _fail('Could not start recording: $e');
    }
  }

  Future<void> _stop({
    required void Function(String text) onTranscript,
    required String baseUrl,
    required String token,
  }) async {
    if (_snapshot.engine == VoiceEngine.onDevice) {
      await _speech.stop();
      _emit(const VoiceSnapshot());
      return;
    }

    String? path;
    try {
      path = await _recorder.stop();
    } catch (e) {
      _fail('Recording failed: $e');
      return;
    }
    path ??= _recordingPath;
    _recordingPath = null;
    if (path == null) {
      _emit(const VoiceSnapshot());
      return;
    }

    final file = File(path);
    final bytes = await file.exists() ? await file.readAsBytes() : null;
    // Delete before the upload so a failed request can't leave audio behind.
    if (await file.exists()) {
      unawaited(file.delete().catchError((_) => file));
    }
    // Below this the clip is silence or a double-tap, not speech.
    if (bytes == null || bytes.length < 1024) {
      _emit(const VoiceSnapshot());
      return;
    }

    _emit(const VoiceSnapshot(
      status: VoiceStatus.transcribing,
      engine: VoiceEngine.daemon,
    ));
    try {
      final text = await _api.transcribeAudio(
        baseUrl,
        token,
        bytes,
        mimeType: 'audio/mp4',
      );
      _emit(const VoiceSnapshot());
      if (text.trim().isNotEmpty) onTranscript(text.trim());
    } on ApiException catch (e) {
      // The daemon's voice errors are setup guidance ("run /voice download",
      // "set GROQ_API_KEY"); surfacing them verbatim is the point.
      _fail(_describeApiError(e));
    } catch (e) {
      _fail('Transcription failed: $e');
    }
  }

  /// Cancel without transcribing — used when a screen is popped mid-recording.
  Future<void> cancel() async {
    if (_snapshot.engine == VoiceEngine.onDevice) {
      await _speech.cancel();
    } else if (_snapshot.isListening) {
      final path = await _recorder.stop() ?? _recordingPath;
      _recordingPath = null;
      if (path != null) {
        final file = File(path);
        if (await file.exists()) unawaited(file.delete().catchError((_) => file));
      }
    }
    _emit(const VoiceSnapshot());
  }

  @override
  void dispose() {
    _disposed = true;
    unawaited(_speech.cancel());
    unawaited(_recorder.dispose());
    super.dispose();
  }
}

/// Turn a platform speech error code into something a user can act on.
String _describeSpeechError(String code) {
  switch (code) {
    case 'error_permission':
    case 'error_speech_timeout':
      return code == 'error_permission'
          ? 'Microphone access was denied. Allow it in Settings and try again.'
          : 'No speech detected.';
    case 'error_no_match':
      return 'Could not make out any words.';
    case 'error_audio':
      return 'The microphone is unavailable.';
    case 'error_network':
      return 'Speech recognition needs a network connection.';
    default:
      return 'Speech recognition failed ($code).';
  }
}

/// Pull the daemon's `{"error": "..."}` message out of a failed response.
String _describeApiError(ApiException e) {
  try {
    final decoded = jsonDecodeSafe(e.body);
    final message = decoded?['error'];
    if (message is String && message.isNotEmpty) return message;
  } catch (_) {
    /* fall through to the generic message */
  }
  return 'Transcription failed (HTTP ${e.statusCode}).';
}

Map<String, dynamic>? jsonDecodeSafe(String body) {
  try {
    final value = const JsonDecoder().convert(body);
    return value is Map<String, dynamic> ? value : null;
  } catch (_) {
    return null;
  }
}
