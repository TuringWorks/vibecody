// watch_sync_service.dart — Google Docs-style real-time sync via /watch/* endpoints.
//
// Polls daemon every 1 second for new messages in the active session.
// Both regular chat and sandbox chat use the same sessions.db via Bearer auth.

import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

class WatchMessage {
  final int id;
  final String role;
  final String content;
  final int createdAt;

  const WatchMessage({
    required this.id,
    required this.role,
    required this.content,
    required this.createdAt,
  });

  factory WatchMessage.fromJson(Map<String, dynamic> j) => WatchMessage(
        id: j['id'] as int? ?? 0,
        role: j['role'] as String? ?? 'user',
        content: j['content'] as String? ?? '',
        createdAt: j['created_at'] as int? ?? 0,
      );
}

/// Real-time session sync service.
/// Call [startSync] when a session is open, [stopSync] when navigating away.
class WatchSyncService extends ChangeNotifier {
  final http.Client _client;
  String? _baseUrl;
  String? _token;
  String? _sessionId;
  Timer? _timer;

  List<WatchMessage> messages = [];
  int _lastId = 0;
  bool isStreaming = false;

  WatchSyncService({http.Client? client}) : _client = client ?? http.Client();

  void configure(String baseUrl, String token) {
    _baseUrl = baseUrl;
    _token = token;
  }

  /// Start syncing messages for [sessionId] at 1-second intervals.
  void startSync(String sessionId) {
    if (_sessionId == sessionId) return;
    stopSync();
    _sessionId = sessionId;
    _lastId = 0;
    messages = [];

    // Prime cursor with existing messages (don't surface them as new)
    _fetchMessages(primeOnly: true);
    _timer = Timer.periodic(const Duration(seconds: 1), (_) => _fetchMessages());
  }

  void stopSync() {
    _timer?.cancel();
    _timer = null;
    _sessionId = null;
    _lastId = 0;
  }

  Future<void> _fetchMessages({bool primeOnly = false}) async {
    final url = _baseUrl;
    final token = _token;
    final sid = _sessionId;
    if (url == null || token == null || sid == null) return;

    try {
      final uri = Uri.parse('$url/watch/sessions/$sid/messages');
      final resp = await _client.get(uri, headers: {
        'Authorization': 'Bearer $token',
        'Accept': 'application/json',
      }).timeout(const Duration(seconds: 3));

      if (resp.statusCode != 200) return;
      final data = jsonDecode(resp.body) as Map<String, dynamic>;
      final arr = (data['messages'] as List?) ?? [];
      final all = arr.map((m) => WatchMessage.fromJson(m as Map<String, dynamic>)).toList();

      if (primeOnly) {
        _lastId = all.isNotEmpty ? all.map((m) => m.id).reduce((a, b) => a > b ? a : b) : 0;
        return;
      }

      final newMsgs = all.where((m) => m.id > _lastId).toList();
      if (newMsgs.isNotEmpty) {
        _lastId = newMsgs.map((m) => m.id).reduce((a, b) => a > b ? a : b);
        messages = [...messages, ...newMsgs];
        notifyListeners();
      }
    } catch (_) {
      // Daemon not reachable — silently ignore
    }
  }

  // ── Session list ─────────────────────────────────────────────────────────

  Future<List<Map<String, dynamic>>> listSessions(String baseUrl, String token) async {
    try {
      final resp = await _client.get(
        Uri.parse('$baseUrl/watch/sessions'),
        headers: {'Authorization': 'Bearer $token'},
      ).timeout(const Duration(seconds: 5));
      if (resp.statusCode != 200) return [];
      final data = jsonDecode(resp.body) as Map<String, dynamic>;
      return List<Map<String, dynamic>>.from(data['sessions'] ?? []);
    } catch (_) {
      return [];
    }
  }

  // ── Dispatch (send message to session) ───────────────────────────────────

  Future<Map<String, dynamic>?> dispatch(
    String baseUrl,
    String token, {
    required String content,
    String? sessionId,
  }) async {
    try {
      final body = jsonEncode({
        'content': content,
        if (sessionId != null) 'session_id': sessionId,
        'nonce': DateTime.now().millisecondsSinceEpoch.toRadixString(16),
        'timestamp': DateTime.now().millisecondsSinceEpoch ~/ 1000,
      });
      final resp = await _client.post(
        Uri.parse('$baseUrl/watch/dispatch'),
        headers: {
          'Authorization': 'Bearer $token',
          'Content-Type': 'application/json',
        },
        body: body,
      ).timeout(const Duration(seconds: 10));
      if (resp.statusCode != 200) return null;
      return jsonDecode(resp.body) as Map<String, dynamic>;
    } catch (_) {
      return null;
    }
  }

  // ── Active session ────────────────────────────────────────────────────────

  Future<String?> getActiveSession(String baseUrl, String token) async {
    try {
      final resp = await _client.get(
        Uri.parse('$baseUrl/watch/active-session'),
        headers: {'Authorization': 'Bearer $token'},
      ).timeout(const Duration(seconds: 2));
      if (resp.statusCode != 200) return null;
      final data = jsonDecode(resp.body) as Map<String, dynamic>;
      return data['session_id'] as String?;
    } catch (_) {
      return null;
    }
  }

  // ── Sandbox chat session ──────────────────────────────────────────────────

  Future<String?> getSandboxChatSession(String baseUrl, String token) async {
    try {
      final resp = await _client.get(
        Uri.parse('$baseUrl/watch/sandbox/chat-session'),
        headers: {'Authorization': 'Bearer $token'},
      ).timeout(const Duration(seconds: 2));
      if (resp.statusCode != 200) return null;
      final data = jsonDecode(resp.body) as Map<String, dynamic>;
      return data['session_id'] as String?;
    } catch (_) {
      return null;
    }
  }

  // ── Poll for response ─────────────────────────────────────────────────────

  /// Poll every 1s until session has an assistant response and status is done.
  Future<List<WatchMessage>> pollForResponse(
    String baseUrl,
    String token,
    String sessionId, {
    int timeoutSeconds = 60,
  }) async {
    var elapsed = 0;
    while (elapsed < timeoutSeconds) {
      await Future.delayed(const Duration(seconds: 1));
      elapsed++;
      try {
        final resp = await _client.get(
          Uri.parse('$baseUrl/watch/sessions/$sessionId/messages'),
          headers: {'Authorization': 'Bearer $token'},
        ).timeout(const Duration(seconds: 3));
        if (resp.statusCode != 200) continue;
        final data = jsonDecode(resp.body) as Map<String, dynamic>;
        final arr = (data['messages'] as List?) ?? [];
        final msgs = arr.map((m) => WatchMessage.fromJson(m as Map<String, dynamic>)).toList();
        final status = data['status'] as String? ?? 'running';
        final hasAssistant = msgs.any((m) => m.role == 'assistant');
        final isDone = status == 'complete' || status == 'failed';
        if (hasAssistant && isDone) return msgs;
      } catch (_) {}
    }
    return [];
  }

  @override
  void dispose() {
    stopSync();
    _client.close();
    super.dispose();
  }
}
