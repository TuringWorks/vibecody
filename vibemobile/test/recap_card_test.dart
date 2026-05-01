import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vibecody_mobile/models/recap.dart';
import 'package:vibecody_mobile/widgets/recap_card.dart';

Recap _fixture({
  String headline = 'Wired auth refresh-token rotation',
  List<String> bullets = const ['Ran cargo test (3x)', 'Edited src/auth.rs'],
  List<String> nextActions = const ['Wire refresh token to frontend'],
  List<RecapArtifact> artifacts = const [
    RecapArtifact(kind: 'file', label: 'auth.rs', locator: 'src/auth.rs'),
  ],
  RecapGenerator generator = const RecapGenerator(type: 'heuristic'),
}) {
  return Recap(
    id: 'rcp_abc',
    kind: 'session',
    subjectId: 'sess_xyz',
    headline: headline,
    bullets: bullets,
    nextActions: nextActions,
    artifacts: artifacts,
    generator: generator,
    schemaVersion: 1,
  );
}

Widget _wrap(Widget child) => MaterialApp(
      theme: ThemeData.dark(),
      home: Scaffold(body: child),
    );

void main() {
  group('RecapCard', () {
    testWidgets('renders the headline', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(recap: _fixture())));
      expect(find.text('Wired auth refresh-token rotation'), findsOneWidget);
    });

    testWidgets('renders bullets and next actions when expanded', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(recap: _fixture())));
      expect(find.text('Ran cargo test (3x)'), findsOneWidget);
      expect(find.text('Edited src/auth.rs'), findsOneWidget);
      expect(find.text('Wire refresh token to frontend'), findsOneWidget);
    });

    testWidgets('renders artifacts with label and locator', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(recap: _fixture())));
      expect(find.text('auth.rs'), findsOneWidget);
      expect(find.text('src/auth.rs'), findsOneWidget);
    });

    testWidgets('shows the heuristic generator badge by default', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(recap: _fixture())));
      expect(find.text('heuristic'), findsOneWidget);
    });

    testWidgets('shows an LLM badge with provider/model', (tester) async {
      final r = _fixture(
        generator: const RecapGenerator(
            type: 'llm', provider: 'anthropic', model: 'claude-opus-4-7'),
      );
      await tester.pumpWidget(_wrap(RecapCard(recap: r)));
      expect(find.text('LLM · anthropic/claude-opus-4-7'), findsOneWidget);
    });

    testWidgets('shows a user-edited badge', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(
          recap: _fixture(generator: const RecapGenerator(type: 'user_edited')))));
      expect(find.text('user-edited'), findsOneWidget);
    });

    testWidgets('omits sections when their lists are empty', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(
          recap: _fixture(bullets: const [], nextActions: const [], artifacts: const []))));
      expect(find.text('WHAT HAPPENED'), findsNothing);
      expect(find.text('NEXT'), findsNothing);
      expect(find.text('ARTIFACTS'), findsNothing);
    });

    testWidgets('"Resume from here" fires onResume', (tester) async {
      var pressed = 0;
      await tester.pumpWidget(_wrap(RecapCard(
        recap: _fixture(),
        onResume: () => pressed++,
      )));
      await tester.tap(find.byKey(const Key('recap-resume-btn')));
      await tester.pumpAndSettle();
      expect(pressed, 1);
    });

    testWidgets('starts collapsed when defaultCollapsed=true; tap header expands', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(recap: _fixture(), defaultCollapsed: true)));
      // Body content (bullets) is hidden when collapsed.
      expect(find.text('Ran cargo test (3x)'), findsNothing);
      // Tap header (anywhere on the headline row) to expand.
      await tester.tap(find.text('Wired auth refresh-token rotation'));
      await tester.pumpAndSettle();
      expect(find.text('Ran cargo test (3x)'), findsOneWidget);
    });

    testWidgets('Resume button is hidden when collapsed', (tester) async {
      await tester.pumpWidget(_wrap(RecapCard(recap: _fixture(), defaultCollapsed: true)));
      expect(find.byKey(const Key('recap-resume-btn')), findsNothing);
    });
  });

  group('Recap.fromJson', () {
    test('parses the canonical wire shape', () {
      final j = {
        'id': 'rcp_1',
        'kind': 'session',
        'subject_id': 'sess_xyz',
        'headline': 'Wired auth',
        'bullets': ['b1', 'b2'],
        'next_actions': ['n1'],
        'artifacts': [
          {'kind': 'file', 'label': 'auth.rs', 'locator': 'src/auth.rs'},
        ],
        'generator': {'type': 'heuristic'},
        'schema_version': 1,
      };
      final r = Recap.fromJson(j);
      expect(r.headline, 'Wired auth');
      expect(r.bullets, ['b1', 'b2']);
      expect(r.nextActions, ['n1']);
      expect(r.artifacts, hasLength(1));
      expect(r.artifacts.first.label, 'auth.rs');
      expect(r.generator.type, 'heuristic');
    });

    test('falls back gracefully on missing optional fields', () {
      final r = Recap.fromJson({
        'id': 'rcp_1',
        'kind': 'session',
        'subject_id': 'sess_xyz',
        'headline': 'X',
        'generator': {'type': 'heuristic'},
        'schema_version': 1,
      });
      expect(r.bullets, isEmpty);
      expect(r.nextActions, isEmpty);
      expect(r.artifacts, isEmpty);
    });

    test('parses LLM generator with provider/model', () {
      final r = Recap.fromJson({
        'id': 'rcp_1',
        'kind': 'session',
        'subject_id': 'sess_xyz',
        'headline': 'X',
        'generator': {'type': 'llm', 'provider': 'anthropic', 'model': 'claude-opus-4-7'},
        'schema_version': 1,
      });
      expect(r.generator.type, 'llm');
      expect(r.generator.provider, 'anthropic');
      expect(r.generator.model, 'claude-opus-4-7');
      expect(r.generator.label(), 'LLM · anthropic/claude-opus-4-7');
    });
  });
}
