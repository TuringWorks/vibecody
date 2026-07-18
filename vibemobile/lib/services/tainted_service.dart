// tainted_service.dart — DREAD #1 Slice G part 3 mobile renderer.
//
// Subscribes to the active machine's `GET /v1/tainted/pending` SSE
// stream and exposes the head-of-queue prompt as a ChangeNotifier so
// `HomeScreen` (or any widget) can render a `TaintedConfirmationSheet`.
// On user decision, calls `POST /v1/tainted/respond` to resolve the
// prompt.
//
// Threat-model invariants:
//
// * Payload bytes never leave the daemon — `TaintedPrompt` carries
//   only `audit_summary`. Same invariant as the VibeCoder WebView modal
//   and the CLI prompter banner.
// * Fail-safe deny: any network failure or dismissed sheet that
//   doesn't POST `approve=true` results in the daemon timing the
//   prompt out (5 min) and denying the agent loop. Only explicit
//   user approval executes.
// * Exponential backoff reconnect — a flapping mobile connection
//   does not block the daemon longer than its timeout.

import 'dart:async';
import 'package:flutter/foundation.dart';
import '../models/tainted_prompt.dart';
import 'api_client.dart';

class TaintedService extends ChangeNotifier {
  final ApiClient api;

  String? _baseUrl;
  String? _token;
  StreamSubscription<Map<String, dynamic>>? _sub;
  Timer? _reconnectTimer;

  /// Currently-pending prompts in FIFO order. The UI displays the
  /// head; subsequent prompts surface after resolve().
  final List<TaintedPrompt> _queue = [];

  /// Request_ids the user has already resolved locally — guards
  /// against re-renders when the daemon re-emits the full snapshot.
  final Set<String> _resolved = {};

  /// Request_ids we've already added to the queue — guards against
  /// duplicates within the same stream.
  final Set<String> _seen = {};

  bool _connected = false;
  String? _lastError;
  Duration _backoff = const Duration(seconds: 1);
  static const _maxBackoff = Duration(seconds: 30);

  TaintedService({required this.api});

  /// True while the SSE subscription is live.
  bool get connected => _connected;

  /// Last error message (or null when healthy).
  String? get lastError => _lastError;

  /// Head-of-queue prompt for the modal sheet, or null when idle.
  TaintedPrompt? get headPrompt => _queue.isEmpty ? null : _queue.first;

  /// Number of pending prompts behind the head.
  int get queuedCount => _queue.length;

  /// Begin (or rebind to) a subscription for the given machine.
  /// Calling `configure` with the same baseUrl+token is a no-op;
  /// calling with new credentials closes the previous subscription.
  void configure(String? baseUrl, String? token) {
    if (baseUrl == _baseUrl && token == _token && _sub != null) return;
    _baseUrl = baseUrl;
    _token = token;
    _disconnect();
    if (baseUrl != null && token != null) {
      _connect();
    }
  }

  void _connect() {
    final baseUrl = _baseUrl;
    final token = _token;
    if (baseUrl == null || token == null) return;

    _connected = false;
    _lastError = null;
    notifyListeners();

    try {
      _sub = api.taintedPendingStream(baseUrl, token).listen(
        _onEvent,
        onError: _onStreamError,
        onDone: _onStreamDone,
        cancelOnError: true,
      );
      _connected = true;
      _backoff = const Duration(seconds: 1);
      notifyListeners();
    } catch (e) {
      _onStreamError(e);
    }
  }

  void _disconnect() {
    _sub?.cancel();
    _sub = null;
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    _connected = false;
  }

  void _onEvent(Map<String, dynamic> json) {
    final prompt = TaintedPrompt.fromJson(json);
    if (prompt.requestId.isEmpty) return;
    if (_resolved.contains(prompt.requestId)) return;
    if (_seen.contains(prompt.requestId)) return;
    _seen.add(prompt.requestId);
    _queue.add(prompt);
    notifyListeners();
  }

  void _onStreamError(Object error) {
    _connected = false;
    _lastError = error.toString();
    notifyListeners();
    _scheduleReconnect();
  }

  void _onStreamDone() {
    _connected = false;
    notifyListeners();
    _scheduleReconnect();
  }

  void _scheduleReconnect() {
    if (_baseUrl == null || _token == null) return;
    _reconnectTimer?.cancel();
    final delay = _backoff;
    _backoff = Duration(
      milliseconds: (_backoff.inMilliseconds * 2).clamp(
        1000,
        _maxBackoff.inMilliseconds,
      ),
    );
    _reconnectTimer = Timer(delay, _connect);
  }

  /// Approve or deny the head-of-queue prompt. Optimistically pops
  /// the queue, then POSTs the decision. If the POST fails the
  /// prompt is NOT re-queued — the daemon will time out and deny,
  /// matching the design's fail-safe-deny guarantee.
  Future<void> respond(String requestId, bool approve) async {
    final baseUrl = _baseUrl;
    final token = _token;
    _resolved.add(requestId);
    _queue.removeWhere((p) => p.requestId == requestId);
    notifyListeners();

    if (baseUrl == null || token == null) return;
    try {
      await api.taintedRespond(baseUrl, token, requestId, approve);
    } catch (e) {
      // Decision didn't land — daemon will fall back to its own
      // timeout-deny. Surface a transient error to the user.
      _lastError = 'Decision failed: $e — daemon will deny on timeout.';
      notifyListeners();
    }
  }

  /// Forget any in-memory state. Used when the user un-pairs or
  /// signs out.
  void reset() {
    _disconnect();
    _queue.clear();
    _seen.clear();
    _resolved.clear();
    _baseUrl = null;
    _token = null;
    _lastError = null;
    notifyListeners();
  }

  @override
  void dispose() {
    _disconnect();
    super.dispose();
  }
}
