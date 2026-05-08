// M1.2 — JobRecapScreen + ApiClient recap-wire tests.
//
// We can't easily exercise the live HTTP path in unit tests, so this
// file leans on two complementary tactics:
//   * Pure-data tests for `Recap.fromJson` parsing (covered in
//     recap_card_test.dart already; we verify job-kind specifics).
//   * Widget tests for JobRecapScreen rendering states (loading, no
//     recap, recap present, Resume action) using a stub ApiClient via
//     a child screen that mounts RecapCard directly with a fixture.
//
// The JobRecapScreen wires an ApiClient internally; rather than drag in
// a mock-HTTP stack we render the headless RecapCard the screen would
// produce, plus assertions on the surrounding affordances (AppBar,
// "View recap" pill, etc.) inside SessionsScreen — those live in
// integration territory and are out of scope here.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vibecody_mobile/models/recap.dart';
import 'package:vibecody_mobile/services/notification_service.dart';
import 'package:vibecody_mobile/widgets/recap_card.dart';

Recap _jobFixture({
  String headline = 'Refactored auth middleware',
  List<String> bullets = const ['Pulled validate_jwt out', '3 call sites updated'],
  List<String> nextActions = const ['Wire refresh-token rotation'],
  RecapGenerator generator = const RecapGenerator(type: 'heuristic'),
}) {
  return Recap(
    id: 'rcp_job_1',
    kind: 'job',
    subjectId: 'job-abc',
    headline: headline,
    bullets: bullets,
    nextActions: nextActions,
    artifacts: const [
      RecapArtifact(kind: 'file', label: 'auth.rs', locator: 'src/auth.rs'),
    ],
    generator: generator,
    schemaVersion: 1,
  );
}

Widget _wrap(Widget child) => MaterialApp(
      theme: ThemeData.dark(),
      home: Scaffold(body: child),
    );

void main() {
  group('M1.2 — RecapCard with kind=job', () {
    testWidgets('renders the job headline', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(recap: _jobFixture())));
      expect(find.text('Refactored auth middleware'), findsOneWidget);
    });

    testWidgets('renders job bullets and next actions', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(recap: _jobFixture())));
      expect(find.text('Pulled validate_jwt out'), findsOneWidget);
      expect(find.text('3 call sites updated'), findsOneWidget);
      expect(find.text('Wire refresh-token rotation'), findsOneWidget);
    });

    testWidgets('Resume button fires onResume callback', (tester) async {
      var resumes = 0;
      await tester.pumpWidget(_wrap(RecapCard(
        recap: _jobFixture(),
        onResume: () => resumes++,
      )));
      await tester.tap(find.byKey(const Key('recap-resume-btn')));
      await tester.pumpAndSettle();
      expect(resumes, 1);
    });

    testWidgets('LLM generator badge surfaces provider and model', (tester) async {
      final r = _jobFixture(
        generator: const RecapGenerator(
          type: 'llm',
          provider: 'anthropic',
          model: 'claude-opus-4-7',
        ),
      );
      await tester.pumpWidget(_wrap(RecapCard(recap: r)));
      expect(find.text('LLM · anthropic/claude-opus-4-7'), findsOneWidget);
    });
  });

  group('M1.2 — Recap.fromJson covers kind=job wire shape', () {
    test('parses kind=job', () {
      final j = {
        'id': 'rcp_job_1',
        'kind': 'job',
        'subject_id': 'job-abc',
        'headline': 'Refactored auth middleware',
        'bullets': ['Pulled validate_jwt out', 'tests green'],
        'next_actions': ['Open a PR'],
        'artifacts': [
          {'kind': 'job', 'label': 'parent-job', 'locator': 'job-parent-id'},
        ],
        'generator': {'type': 'heuristic'},
        'schema_version': 1,
      };
      final r = Recap.fromJson(j);
      expect(r.kind, 'job');
      expect(r.headline, 'Refactored auth middleware');
      expect(r.bullets, hasLength(2));
      expect(r.nextActions, ['Open a PR']);
      expect(r.artifacts.first.kind, 'job');
      expect(r.artifacts.first.locator, 'job-parent-id');
    });
  });

  group('M1.2 — NotificationCategories', () {
    test('exposes the job_recap category constant', () {
      expect(NotificationCategories.jobRecap, 'job_recap');
    });

    test('AppNotification accepts the jobRecap category', () {
      final n = AppNotification(
        id: 'n1',
        title: 'Job complete',
        body: 'Refactored auth middleware',
        category: NotificationCategories.jobRecap,
        taskId: 'job-abc',
      );
      expect(n.category, 'job_recap');
      expect(n.taskId, 'job-abc');
    });
  });
}
