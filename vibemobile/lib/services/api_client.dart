import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' as http;
import '../models/machine.dart';
import 'auth_service.dart';

/// HTTP client for the VibeCody daemon REST API.
class ApiClient {
  final AuthService auth;
  final http.Client _client = http.Client();

  ApiClient({required this.auth});

  /// Build full URL for a given machine and path.
  String _url(String baseUrl, String path) => '$baseUrl$path';

  Map<String, String> _headers(String token) => {
    'Authorization': 'Bearer $token',
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  };

  // ── Machine Management ──────────────────────────────────────

  /// List all registered machines on a daemon.
  Future<List<Machine>> listMachines(String baseUrl, String token) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/mobile/machines')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    final data = jsonDecode(resp.body);
    return (data['machines'] as List).map((m) => Machine.fromJson(m)).toList();
  }

  /// Get detailed machine info.
  Future<Machine> getMachine(String baseUrl, String token, String machineId) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/mobile/machines/$machineId')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Machine.fromJson(jsonDecode(resp.body));
  }

  /// Register this machine with the gateway.
  Future<String> registerMachine(String baseUrl, String token, Map<String, dynamic> registration) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/mobile/machines')),
      headers: _headers(token),
      body: jsonEncode(registration),
    );
    if (resp.statusCode != 201) throw ApiException(resp.statusCode, resp.body);
    final data = jsonDecode(resp.body);
    return data['machine_id'];
  }

  /// Unregister a machine.
  Future<void> unregisterMachine(String baseUrl, String token, String machineId) async {
    final resp = await _client.delete(
      Uri.parse(_url(baseUrl, '/mobile/machines/$machineId')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
  }

  /// Send heartbeat for a machine.
  Future<int> heartbeat(String baseUrl, String token, String machineId, Map<String, dynamic> metrics) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/mobile/machines/$machineId/heartbeat')),
      headers: _headers(token),
      body: jsonEncode(metrics),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    final data = jsonDecode(resp.body);
    return data['pending_dispatches'] ?? 0;
  }

  // ── Pairing ────────────────────────────────────────────────

  /// Create a pairing request.
  Future<Map<String, dynamic>> createPairing(String baseUrl, String token, String machineId, {String method = 'qr_code'}) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/mobile/pairing')),
      headers: _headers(token),
      body: jsonEncode({'machine_id': machineId, 'method': method}),
    );
    if (resp.statusCode != 201) throw ApiException(resp.statusCode, resp.body);
    return jsonDecode(resp.body);
  }

  /// Accept a pairing from mobile side.
  Future<void> acceptPairing(String baseUrl, String token, String pairingId, Map<String, dynamic> deviceInfo) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/mobile/pairing/$pairingId/accept')),
      headers: _headers(token),
      body: jsonEncode(deviceInfo),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
  }

  /// Verify a 6-digit PIN.
  Future<bool> verifyPin(String baseUrl, String token, String pairingId, String pin) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/mobile/pairing/$pairingId/verify')),
      headers: _headers(token),
      body: jsonEncode({'pin': pin}),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return jsonDecode(resp.body)['valid'] == true;
  }

  // ── Dispatch ───────────────────────────────────────────────

  /// Dispatch a task to a machine.
  Future<String> dispatch(String baseUrl, String token, {
    required String deviceId,
    required String machineId,
    String dispatchType = 'chat',
    required String payload,
  }) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/mobile/dispatch')),
      headers: _headers(token),
      body: jsonEncode({
        'device_id': deviceId,
        'machine_id': machineId,
        'dispatch_type': dispatchType,
        'payload': payload,
      }),
    );
    if (resp.statusCode != 201) throw ApiException(resp.statusCode, resp.body);
    return jsonDecode(resp.body)['task_id'];
  }

  /// Get dispatch task details.
  Future<DispatchTask> getDispatch(String baseUrl, String token, String taskId) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/mobile/dispatch/$taskId')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return DispatchTask.fromJson(jsonDecode(resp.body));
  }

  /// Cancel a dispatch.
  Future<void> cancelDispatch(String baseUrl, String token, String taskId) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/mobile/dispatch/$taskId/cancel')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
  }

  /// List dispatches for a machine.
  Future<List<DispatchTask>> machineDispatches(String baseUrl, String token, String machineId) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/mobile/dispatches/machine/$machineId')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    final data = jsonDecode(resp.body);
    return (data['dispatches'] as List).map((t) => DispatchTask.fromJson(t)).toList();
  }

  // ── Chat (direct to daemon) ────────────────────────────────

  /// Send a chat message directly to a daemon.
  Future<String> chat(String baseUrl, String token, String message) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/chat')),
      headers: _headers(token),
      body: jsonEncode({
        'messages': [{'role': 'user', 'content': message}],
      }),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return jsonDecode(resp.body)['reply'] ?? '';
  }

  /// Stream chat responses via SSE.
  Stream<String> chatStream(String baseUrl, String token, String message) async* {
    final request = http.Request(
      'POST',
      Uri.parse(_url(baseUrl, '/chat/stream')),
    );
    request.headers.addAll(_headers(token));
    request.body = jsonEncode({
      'messages': [{'role': 'user', 'content': message}],
    });

    final response = await _client.send(request);
    await for (final chunk in response.stream.transform(utf8.decoder).transform(const LineSplitter())) {
      if (chunk.startsWith('data: ')) {
        yield chunk.substring(6);
      }
    }
  }

  // ── Agent ──────────────────────────────────────────────────

  /// Start an agent task on a daemon.
  Future<String> startAgent(String baseUrl, String token, String task) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/agent')),
      headers: _headers(token),
      body: jsonEncode({'task': task}),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return jsonDecode(resp.body)['session_id'];
  }

  /// Stream agent events via SSE.
  Stream<Map<String, dynamic>> agentStream(String baseUrl, String token, String sessionId) async* {
    final request = http.Request(
      'GET',
      Uri.parse(_url(baseUrl, '/stream/$sessionId')),
    );
    request.headers.addAll(_headers(token));

    final response = await _client.send(request);
    await for (final chunk in response.stream.transform(utf8.decoder).transform(const LineSplitter())) {
      if (chunk.startsWith('data: ')) {
        try {
          yield jsonDecode(chunk.substring(6));
        } catch (_) {
          yield {'type': 'chunk', 'content': chunk.substring(6)};
        }
      }
    }
  }

  // ── Jobs ───────────────────────────────────────────────────

  /// List all jobs on a daemon.
  Future<List<Map<String, dynamic>>> listJobs(String baseUrl, String token) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/jobs')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return List<Map<String, dynamic>>.from(jsonDecode(resp.body));
  }

  /// Cancel a running job.
  Future<void> cancelJob(String baseUrl, String token, String jobId) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/jobs/$jobId/cancel')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
  }

  // ── Gateway Stats ──────────────────────────────────────────

  /// Get mobile gateway statistics.
  Future<GatewayStats> stats(String baseUrl, String token) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/mobile/stats')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return GatewayStats.fromJson(jsonDecode(resp.body));
  }

  // ── Handoff / Sessions ─────────────────────────────────────

  /// Fetch beacon (no auth required).
  Future<Map<String, dynamic>> beacon(String baseUrl) async {
    final resp = await _client
        .get(Uri.parse('$baseUrl/mobile/beacon'))
        .timeout(const Duration(seconds: 5));
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return jsonDecode(resp.body);
  }

  /// List sessions for handoff — uses /mobile/sessions endpoint.
  Future<List<Map<String, dynamic>>> listSessions(String baseUrl, String token) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/mobile/sessions')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    final data = jsonDecode(resp.body);
    return List<Map<String, dynamic>>.from(data['sessions'] ?? []);
  }

  /// Fetch handoff context for a specific session.
  Future<Map<String, dynamic>> sessionContext(
      String baseUrl, String token, String sessionId) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/mobile/sessions/$sessionId/context')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return jsonDecode(resp.body);
  }

  // ── Health Check ───────────────────────────────────────────

  /// Check if a daemon is reachable.
  Future<bool> healthCheck(String baseUrl) async {
    try {
      final resp = await _client.get(Uri.parse('$baseUrl/health')).timeout(
        const Duration(seconds: 5),
      );
      return resp.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  void dispose() {
    _client.close();
  }
}

class ApiException implements Exception {
  final int statusCode;
  final String body;
  ApiException(this.statusCode, this.body);

  @override
  String toString() => 'ApiException($statusCode): $body';
}
