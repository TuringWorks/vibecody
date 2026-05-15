import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:vibecody_mobile/services/api_client.dart';
import 'package:vibecody_mobile/services/auth_service.dart';
import 'package:vibecody_mobile/services/tainted_service.dart';

/// Test double for `ApiClient` — drives a controlled stream of
/// pending events and records every resolve call.
class _FakeApi extends ApiClient {
  final StreamController<Map<String, dynamic>> controller =
      StreamController<Map<String, dynamic>>.broadcast();
  final List<Map<String, dynamic>> responses = [];
  bool nextRespondFails = false;

  _FakeApi() : super(auth: AuthService());

  @override
  Stream<Map<String, dynamic>> taintedPendingStream(
    String baseUrl,
    String token,
  ) => controller.stream;

  @override
  Future<bool> taintedRespond(
    String baseUrl,
    String token,
    String requestId,
    bool approve,
  ) async {
    responses.add({
      'request_id': requestId,
      'approve': approve,
    });
    if (nextRespondFails) {
      throw ApiException(500, 'simulated failure');
    }
    return true;
  }
}

Map<String, dynamic> _event(String id, {String summary = 'kind=file ...'}) => {
      'request_id': id,
      'audit_id': 'audit-$id',
      'summary': summary,
      'sink': 'ToolCallArgument',
      'issued_at': 0,
    };

void main() {
  group('TaintedService', () {
    test('starts disconnected and idle', () {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      expect(svc.connected, false);
      expect(svc.headPrompt, null);
      expect(svc.queuedCount, 0);
    });

    test('configure(null, null) keeps service idle', () {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      svc.configure(null, null);
      expect(svc.connected, false);
    });

    test('forwards stream events as TaintedPrompt entries', () async {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      svc.configure('http://localhost:7878', 'tok');
      // Yield to the event loop so the stream subscription is wired.
      await Future<void>.delayed(Duration.zero);
      api.controller.add(_event('p-1'));
      await Future<void>.delayed(Duration.zero);
      expect(svc.headPrompt?.requestId, 'p-1');
      expect(svc.queuedCount, 1);
    });

    test('de-dupes the same request_id across the stream', () async {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      svc.configure('http://localhost:7878', 'tok');
      await Future<void>.delayed(Duration.zero);
      api.controller.add(_event('p-1'));
      api.controller.add(_event('p-1'));
      api.controller.add(_event('p-1'));
      await Future<void>.delayed(Duration.zero);
      expect(svc.queuedCount, 1);
    });

    test('respond(approve=true) pops the queue and POSTs the decision',
        () async {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      svc.configure('http://localhost:7878', 'tok');
      await Future<void>.delayed(Duration.zero);
      api.controller.add(_event('p-1'));
      await Future<void>.delayed(Duration.zero);

      await svc.respond('p-1', true);
      expect(svc.headPrompt, null);
      expect(svc.queuedCount, 0);
      expect(api.responses, [
        {'request_id': 'p-1', 'approve': true},
      ]);
    });

    test('respond(approve=false) records deny and pops the queue', () async {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      svc.configure('http://localhost:7878', 'tok');
      await Future<void>.delayed(Duration.zero);
      api.controller.add(_event('p-1'));
      await Future<void>.delayed(Duration.zero);

      await svc.respond('p-1', false);
      expect(api.responses, [
        {'request_id': 'p-1', 'approve': false},
      ]);
      expect(svc.headPrompt, null);
    });

    test('already-resolved request_ids never re-render', () async {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      svc.configure('http://localhost:7878', 'tok');
      await Future<void>.delayed(Duration.zero);
      api.controller.add(_event('p-1'));
      await Future<void>.delayed(Duration.zero);
      await svc.respond('p-1', true);
      // Daemon re-emits the same id (snapshot replay window). The
      // service must NOT re-add it to the visible queue.
      api.controller.add(_event('p-1'));
      await Future<void>.delayed(Duration.zero);
      expect(svc.headPrompt, null);
      expect(svc.queuedCount, 0);
    });

    test('respond failure surfaces lastError but still pops the queue',
        () async {
      // Fail-safe deny: a failed POST does NOT re-queue — the daemon
      // will time out and deny. The service surfaces a transient
      // error so the UI can show a banner; the prompt does not come
      // back.
      final api = _FakeApi();
      api.nextRespondFails = true;
      final svc = TaintedService(api: api);
      svc.configure('http://localhost:7878', 'tok');
      await Future<void>.delayed(Duration.zero);
      api.controller.add(_event('p-1'));
      await Future<void>.delayed(Duration.zero);
      await svc.respond('p-1', true);

      expect(svc.headPrompt, null);
      expect(svc.lastError, isNotNull);
      expect(svc.lastError, contains('Decision failed'));
    });

    test('reset clears queue + reconnect state', () async {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      svc.configure('http://localhost:7878', 'tok');
      await Future<void>.delayed(Duration.zero);
      api.controller.add(_event('p-1'));
      await Future<void>.delayed(Duration.zero);
      expect(svc.queuedCount, 1);
      svc.reset();
      expect(svc.queuedCount, 0);
      expect(svc.connected, false);
    });

    test('queue is FIFO — head is the oldest pending prompt', () async {
      final api = _FakeApi();
      final svc = TaintedService(api: api);
      svc.configure('http://localhost:7878', 'tok');
      await Future<void>.delayed(Duration.zero);
      api.controller.add(_event('p-1'));
      api.controller.add(_event('p-2'));
      api.controller.add(_event('p-3'));
      await Future<void>.delayed(Duration.zero);
      expect(svc.queuedCount, 3);
      expect(svc.headPrompt?.requestId, 'p-1');
      await svc.respond('p-1', true);
      expect(svc.headPrompt?.requestId, 'p-2');
      await svc.respond('p-2', false);
      expect(svc.headPrompt?.requestId, 'p-3');
    });
  });
}
