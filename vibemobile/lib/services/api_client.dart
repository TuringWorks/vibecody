import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' as http;
import '../models/machine.dart';
import '../models/recap.dart';
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

  // ── Active session (F3.x — cross-device handoff) ───────────

  /// Claim a session as "active on this device" so VibeUI can follow
  /// (mirrors the W1.1 watch path). Best-effort: failures are logged
  /// but do not block opening the chat.
  Future<void> setActiveSession(
    String baseUrl,
    String token, {
    required String sessionId,
    String? deviceId,
    String? deviceLabel,
  }) async {
    try {
      await _client
          .put(
            Uri.parse(_url(baseUrl, '/mobile/active-session')),
            headers: _headers(token),
            body: jsonEncode({
              'session_id': sessionId,
              'device_id': ?deviceId,
              'device_label': ?deviceLabel,
            }),
          )
          .timeout(const Duration(seconds: 5));
    } catch (_) {
      // Best-effort — never block the UI on a failed sync.
    }
  }

  /// Read the daemon's currently-claimed mobile active session. Used
  /// by SessionsScreen to show an "Active on $device" badge so the
  /// user knows which row VibeUI is following.
  Future<Map<String, dynamic>?> getActiveSession(
      String baseUrl, String token) async {
    try {
      final resp = await _client
          .get(
            Uri.parse(_url(baseUrl, '/mobile/active-session')),
            headers: _headers(token),
          )
          .timeout(const Duration(seconds: 5));
      if (resp.statusCode != 200) return null;
      final data = jsonDecode(resp.body);
      if (data is Map<String, dynamic>) {
        final cur = data['active_session'];
        return cur is Map<String, dynamic> ? cur : null;
      }
      return null;
    } catch (_) {
      return null;
    }
  }

  // ── Recap (M1.1 + M1.2) ────────────────────────────────────
  //
  // M1.1 — read-only consumer of /v1/recap (kind=session). M1.2
  // extends this with kind=job and a /v1/resume helper. Mobile
  // never generates recaps; the daemon owns composition.
  //
  // The /v1/recap list endpoint wraps results in
  //   {"recaps": [...], "count": N}
  // so we unwrap `data['recaps']` here. (Earlier M1.1 code expected a
  // bare array; that path always returned null and is fixed below.)

  /// Fetch the most recent session recap for [subjectId].
  /// Returns `null` when the daemon has no recap yet, or when the
  /// route returns a non-2xx — mobile degrades silently rather than
  /// blocking the chat from opening.
  Future<Recap?> getSessionRecap(
      String baseUrl, String token, String subjectId) async {
    return _getRecapByKind(baseUrl, token, kind: 'session', subjectId: subjectId);
  }

  /// M1.2 — Fetch the most recent job recap for [jobId]. Same wire
  /// shape as session recaps; the daemon's J1.1 schema lives on
  /// `jobs.db` and is decrypted on read. Returns `null` for jobs
  /// that have no terminal recap yet (daemon's J1.2 hook is
  /// auto-recap-on-terminal; in-flight jobs land here as null).
  Future<Recap?> getJobRecap(
      String baseUrl, String token, String jobId) async {
    return _getRecapByKind(baseUrl, token, kind: 'job', subjectId: jobId);
  }

  Future<Recap?> _getRecapByKind(
    String baseUrl,
    String token, {
    required String kind,
    required String subjectId,
  }) async {
    try {
      final uri = Uri.parse('$baseUrl/v1/recap').replace(queryParameters: {
        'kind': kind,
        'subject_id': subjectId,
        'limit': '1',
      });
      final resp = await _client
          .get(uri, headers: _headers(token))
          .timeout(const Duration(seconds: 5));
      if (resp.statusCode != 200) return null;
      final data = jsonDecode(resp.body);
      if (data is Map<String, dynamic>) {
        final list = data['recaps'];
        if (list is List && list.isNotEmpty) {
          return Recap.fromJson(list.first as Map<String, dynamic>);
        }
      }
      return null;
    } catch (_) {
      return null;
    }
  }

  /// M1.2 — Trigger a `/v1/resume` from a stored recap. Returns the
  /// new `resumed_session_id` on success or null on any failure.
  /// Mobile only ever surfaces the result via toast / banner; the
  /// actual job execution stays on the daemon side.
  Future<String?> resumeFromRecap(
    String baseUrl,
    String token, {
    required String recapId,
    bool branch = false,
  }) async {
    try {
      final resp = await _client
          .post(
            Uri.parse('$baseUrl/v1/resume'),
            headers: {
              ..._headers(token),
              'Content-Type': 'application/json',
            },
            body: jsonEncode({
              'from_recap_id': recapId,
              'branch': branch,
              'client': 'vibemobile',
            }),
          )
          .timeout(const Duration(seconds: 8));
      if (resp.statusCode != 200) return null;
      final data = jsonDecode(resp.body);
      if (data is Map<String, dynamic>) {
        final sid = data['resumed_session_id'];
        if (sid is String && sid.isNotEmpty) return sid;
      }
      return null;
    } catch (_) {
      return null;
    }
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

  // ── DREAD #1 Slice G part 3 — tainted-argument confirmation bridge ──
  //
  // Same daemon endpoints as the VibeUI WebView modal. Mobile renders
  // through `TaintedConfirmationSheet`; the underlying tainted bytes
  // never leave the daemon — only `audit_summary` (kind / origin /
  // audit_id) crosses the wire. See
  // `docs/security/tainted-data-flow.md` §9 part 3.

  /// Subscribe to `GET /v1/tainted/pending` (SSE) and yield each
  /// `PendingPromptEvent` JSON map as the daemon emits it.
  ///
  /// Daemon-side de-dupes by request_id when the same prompt appears
  /// in both the initial snapshot and a subsequent notification; the
  /// caller is responsible for filtering by `request_id` if they
  /// surface every event.
  Stream<Map<String, dynamic>> taintedPendingStream(
    String baseUrl,
    String token,
  ) async* {
    final request = http.Request(
      'GET',
      Uri.parse(_url(baseUrl, '/v1/tainted/pending')),
    );
    request.headers.addAll(_headers(token));

    final response = await _client.send(request);
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, 'pending stream rejected');
    }

    // Standard SSE framing: blank-line-separated records, each made
    // of `field: value` lines. We forward only `data:` payloads that
    // belong to the `pending` event type.
    String? currentEvent;
    final dataBuffer = StringBuffer();
    await for (final chunk in response.stream
        .transform(utf8.decoder)
        .transform(const LineSplitter())) {
      if (chunk.isEmpty) {
        if (currentEvent == 'pending' && dataBuffer.isNotEmpty) {
          try {
            yield jsonDecode(dataBuffer.toString());
          } catch (_) {
            // Malformed event — skip; daemon owns the schema.
          }
        }
        currentEvent = null;
        dataBuffer.clear();
        continue;
      }
      if (chunk.startsWith('event: ')) {
        currentEvent = chunk.substring(7).trim();
      } else if (chunk.startsWith('data: ')) {
        if (dataBuffer.isNotEmpty) dataBuffer.write('\n');
        dataBuffer.write(chunk.substring(6));
      }
      // `id:` / `retry:` / `:comment` lines ignored — daemon does not
      // currently emit them on this stream.
    }
  }

  /// Resolve a pending tainted-argument prompt. Returns `true` when
  /// the daemon successfully matched and resolved the `request_id`;
  /// `false` when the id is unknown or already-resolved (the daemon
  /// will time the prompt out on its end either way — fail-safe deny).
  Future<bool> taintedRespond(
    String baseUrl,
    String token,
    String requestId,
    bool approve,
  ) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/v1/tainted/respond')),
      headers: _headers(token),
      body: jsonEncode({'request_id': requestId, 'approve': approve}),
    );
    if (resp.statusCode == 200) {
      final data = jsonDecode(resp.body);
      return data['resolved'] == true;
    }
    if (resp.statusCode == 404) return false;
    throw ApiException(resp.statusCode, resp.body);
  }

  // ── /goal — durable execution intent (G1.6) ──────────────────
  //
  // Read-mostly surface. Mobile lists goals across all workspaces
  // (grouped by workspace in the UI) and can start a session bound to
  // a goal. Plan/link/aggregate-recap stay on richer clients.

  /// List goals across all workspaces. Filter by status if given.
  /// Returns the raw JSON payload `{ goals: [...], count: N }` so the
  /// caller can pull out either piece.
  Future<Map<String, dynamic>> listGoals(
    String baseUrl,
    String token, {
    String? status,
    String? workspace,
    int limit = 50,
  }) async {
    final qp = <String, String>{'limit': '$limit'};
    if (status != null) qp['status'] = status;
    if (workspace != null) qp['workspace'] = workspace;
    final uri = Uri.parse(_url(baseUrl, '/v1/goals'))
        .replace(queryParameters: qp);
    final resp = await _client.get(uri, headers: _headers(token));
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// G8.2 — create a new goal. `title` is required (≤120 chars,
  /// enforced server-side; we don't pre-check). `statement` defaults
  /// to empty; `workspace` is the absolute project path the goal
  /// belongs to, or null for the global slot. Returns the created
  /// `Goal` row from the daemon (with `id`, `created_at`, etc.).
  Future<Map<String, dynamic>> createGoal(
    String baseUrl,
    String token, {
    required String title,
    String? statement,
    String? workspace,
  }) async {
    final body = <String, dynamic>{
      'title': title,
      'statement': statement ?? '',
    };
    if (workspace != null && workspace.isNotEmpty) {
      body['workspace'] = workspace;
    }
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/v1/goals')),
      headers: _headers(token),
      body: jsonEncode(body),
    );
    if (resp.statusCode != 201 && resp.statusCode != 200) {
      throw ApiException(resp.statusCode, resp.body);
    }
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// Fetch a single goal with its links.
  Future<Map<String, dynamic>> getGoal(
    String baseUrl,
    String token,
    String goalId,
  ) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/v1/goals/$goalId')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// Start a new session bound to a goal. Returns the new session id.
  Future<String> startGoal(
    String baseUrl,
    String token,
    String goalId, {
    String? task,
  }) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/v1/goals/$goalId/start')),
      headers: _headers(token),
      body: jsonEncode({
        'task': ?task,
      }),
    );
    if (resp.statusCode != 201 && resp.statusCode != 200) {
      throw ApiException(resp.statusCode, resp.body);
    }
    final data = jsonDecode(resp.body);
    return data['session_id'] as String;
  }

  // G5.3 — tree + pin + recap-LLM coverage.

  /// Recursive subtree walk. `depth` is clamped server-side to `[1, 10]`
  /// (default 3). Returns `{ root, depth, tree }` where each node has
  /// `{ goal, children, [truncated, direct_child_count, cycle] }`.
  Future<Map<String, dynamic>> getGoalTree(
    String baseUrl,
    String token,
    String goalId, {
    int? depth,
  }) async {
    final qp = <String, String>{};
    if (depth != null) qp['depth'] = '$depth';
    final uri = Uri.parse(_url(baseUrl, '/v1/goals/$goalId/tree'))
        .replace(queryParameters: qp.isEmpty ? null : qp);
    final resp = await _client.get(uri, headers: _headers(token));
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// Read the pinned goal for a workspace (empty/null = global slot).
  /// Returns `{ workspace, goal_id, pinned_at?, goal? }`; `goal_id`
  /// is `null` when nothing is pinned.
  Future<Map<String, dynamic>> getCurrentGoal(
    String baseUrl,
    String token, {
    String? workspace,
  }) async {
    final qp = <String, String>{};
    if (workspace != null && workspace.isNotEmpty) qp['workspace'] = workspace;
    final uri = Uri.parse(_url(baseUrl, '/v1/goals/current'))
        .replace(queryParameters: qp.isEmpty ? null : qp);
    final resp = await _client.get(uri, headers: _headers(token));
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// Pin a goal as the current execution intent for a workspace
  /// (omit `workspace` to pin in the cross-workspace global slot).
  Future<void> pinGoal(
    String baseUrl,
    String token,
    String goalId, {
    String? workspace,
  }) async {
    final resp = await _client.put(
      Uri.parse(_url(baseUrl, '/v1/goals/current')),
      headers: _headers(token),
      body: jsonEncode({
        'goal_id': goalId,
        'workspace': ?workspace,
      }),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
  }

  /// Clear the pin for a workspace (or the global slot).
  Future<bool> unpinGoal(
    String baseUrl,
    String token, {
    String? workspace,
  }) async {
    final qp = <String, String>{};
    if (workspace != null && workspace.isNotEmpty) qp['workspace'] = workspace;
    final uri = Uri.parse(_url(baseUrl, '/v1/goals/current'))
        .replace(queryParameters: qp.isEmpty ? null : qp);
    final resp = await _client.delete(uri, headers: _headers(token));
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    final data = jsonDecode(resp.body);
    return data['removed'] == true;
  }

  // ── /v1/graph/* — kodegraph code-knowledge-graph (no LLM call) ──────────
  //
  // Thin proxies to the daemon's graph routes. Responses are raw JSON maps
  // (kodegraph shapes are daemon-owned). Mobile polls `graphStatus` to show
  // indexing→ready; `graphBuild` triggers a background rebuild.

  /// `GET /v1/graph/status` — `{status, node_count, edge_count, last_built_at?}`.
  Future<Map<String, dynamic>> graphStatus(
    String baseUrl,
    String token,
  ) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/v1/graph/status')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// `POST /v1/graph/build` — kick off a background build; returns `{status:"indexing"}`.
  Future<Map<String, dynamic>> graphBuild(
    String baseUrl,
    String token,
  ) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/v1/graph/build')),
      headers: _headers(token),
    );
    if (resp.statusCode != 202 && resp.statusCode != 200) {
      throw ApiException(resp.statusCode, resp.body);
    }
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// `POST /v1/graph/query {query, budget?}` — token-budgeted subgraph.
  Future<Map<String, dynamic>> graphQuery(
    String baseUrl,
    String token,
    String query, {
    int budget = 2000,
  }) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/v1/graph/query')),
      headers: _headers(token),
      body: jsonEncode({'query': query, 'budget': budget}),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// `GET /v1/graph/node/:name` — one node payload.
  Future<Map<String, dynamic>> graphNode(
    String baseUrl,
    String token,
    String name,
  ) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/v1/graph/node/${Uri.encodeComponent(name)}')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// `GET /v1/graph/neighbors/:name` — adjacent nodes (a JSON array).
  Future<List<dynamic>> graphNeighbors(
    String baseUrl,
    String token,
    String name,
  ) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/v1/graph/neighbors/${Uri.encodeComponent(name)}')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return jsonDecode(resp.body) as List<dynamic>;
  }

  /// `GET /v1/graph/path/:from/:to` — `{path:[…], hops}`.
  Future<Map<String, dynamic>> graphPath(
    String baseUrl,
    String token,
    String from,
    String to,
  ) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl,
          '/v1/graph/path/${Uri.encodeComponent(from)}/${Uri.encodeComponent(to)}')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// `POST /v1/graph/blast {name, max_hops?}` — blast radius.
  Future<Map<String, dynamic>> graphBlast(
    String baseUrl,
    String token,
    String name, {
    int maxHops = 2,
  }) async {
    final resp = await _client.post(
      Uri.parse(_url(baseUrl, '/v1/graph/blast')),
      headers: _headers(token),
      body: jsonEncode({'name': name, 'max_hops': maxHops}),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
  }

  /// `GET /v1/graph/report` — full `GRAPH_REPORT.md` text (`{report:string}`).
  Future<Map<String, dynamic>> graphReport(
    String baseUrl,
    String token,
  ) async {
    final resp = await _client.get(
      Uri.parse(_url(baseUrl, '/v1/graph/report')),
      headers: _headers(token),
    );
    if (resp.statusCode != 200) throw ApiException(resp.statusCode, resp.body);
    return Map<String, dynamic>.from(jsonDecode(resp.body));
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
