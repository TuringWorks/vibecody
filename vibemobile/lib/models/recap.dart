// Dart mirror of `vibecli/vibecli-cli/src/recap.rs` wire shape.
// Bumped with that file. Spec: docs/design/recap-resume/01-session.md.

class RecapArtifact {
  final String kind; // "file" | "diff" | "job" | "url"
  final String label;
  final String locator;

  const RecapArtifact({required this.kind, required this.label, required this.locator});

  factory RecapArtifact.fromJson(Map<String, dynamic> j) => RecapArtifact(
        kind: j['kind'] as String? ?? 'file',
        label: j['label'] as String? ?? '',
        locator: j['locator'] as String? ?? '',
      );
}

class RecapGenerator {
  /// "heuristic" | "user_edited" | "llm".
  final String type;
  final String? provider;
  final String? model;

  const RecapGenerator({required this.type, this.provider, this.model});

  factory RecapGenerator.fromJson(Map<String, dynamic> j) => RecapGenerator(
        type: j['type'] as String? ?? 'heuristic',
        provider: j['provider'] as String?,
        model: j['model'] as String?,
      );

  String label() {
    if (type == 'llm') return 'LLM · ${provider ?? '?'}/${model ?? '?'}';
    if (type == 'user_edited') return 'user-edited';
    return 'heuristic';
  }
}

class Recap {
  final String id;
  final String kind; // "session" | "job" | "diff_chain"
  final String subjectId;
  final String headline;
  final List<String> bullets;
  final List<String> nextActions;
  final List<RecapArtifact> artifacts;
  final RecapGenerator generator;
  final int schemaVersion;

  const Recap({
    required this.id,
    required this.kind,
    required this.subjectId,
    required this.headline,
    required this.bullets,
    required this.nextActions,
    required this.artifacts,
    required this.generator,
    required this.schemaVersion,
  });

  factory Recap.fromJson(Map<String, dynamic> j) => Recap(
        id: j['id'] as String? ?? '',
        kind: j['kind'] as String? ?? 'session',
        subjectId: j['subject_id'] as String? ?? '',
        headline: j['headline'] as String? ?? '',
        bullets: List<String>.from(j['bullets'] as List? ?? const []),
        nextActions: List<String>.from(j['next_actions'] as List? ?? const []),
        artifacts: ((j['artifacts'] as List?) ?? const [])
            .map((a) => RecapArtifact.fromJson(a as Map<String, dynamic>))
            .toList(),
        generator: RecapGenerator.fromJson(
            (j['generator'] as Map<String, dynamic>?) ?? const {'type': 'heuristic'}),
        schemaVersion: (j['schema_version'] as int?) ?? 1,
      );
}
