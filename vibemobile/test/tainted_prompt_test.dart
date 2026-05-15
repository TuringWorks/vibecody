import 'package:flutter_test/flutter_test.dart';
import 'package:vibecody_mobile/models/tainted_prompt.dart';

void main() {
  group('TaintedPrompt', () {
    test('parses the daemon PendingPromptEvent JSON shape', () {
      final p = TaintedPrompt.fromJson({
        'request_id': 'prompt-abc',
        'audit_id': 'audit-1234567890abcdef',
        'summary': 'kind=file audit_id=audit-1234567890abcdef '
            'origin=file{path=/repo/README.md}',
        'sink': 'ToolCallArgument',
        'issued_at': 1715700000,
      });
      expect(p.requestId, 'prompt-abc');
      expect(p.auditId, 'audit-1234567890abcdef');
      expect(p.summary, contains('kind=file'));
      expect(p.sink, 'ToolCallArgument');
      expect(p.issuedAt, 1715700000);
    });

    test('tolerates missing fields (daemon-shape evolution)', () {
      final p = TaintedPrompt.fromJson({});
      expect(p.requestId, '');
      expect(p.auditId, '');
      expect(p.summary, '');
      expect(p.sink, 'ToolCallArgument');
      expect(p.issuedAt, 0);
    });

    test('sinkLabel maps every known sink kind', () {
      String labelFor(String sink) => TaintedPrompt(
            requestId: 'r',
            auditId: 'a',
            summary: 's',
            sink: sink,
            issuedAt: 0,
          ).sinkLabel;

      expect(labelFor('ToolCallArgument'), contains('tool'));
      expect(labelFor('McpArgument'), contains('MCP'));
      expect(labelFor('RagDocument'), contains('retrieval'));
      expect(labelFor('WebFetch'), contains('web'));
      expect(labelFor('LlmRequestBody'), contains('LLM'));
      expect(labelFor('LogLine'), contains('log'));
      expect(labelFor('ShellCommand'), contains('shell'));
      // Unknown sink falls back to a generic safe label.
      expect(labelFor('FutureSink'), contains('untrusted'));
    });

    test('summary parsed verbatim — no payload reconstruction', () {
      // Threat-model invariant: the model carries the daemon's
      // audit_summary as-is and never tries to "interpret" it. This
      // test pins the contract so a future refactor can't quietly
      // start parsing summary fields and accidentally surface a
      // payload byte.
      const summary = 'kind=web audit_id=deadbeef00000000 '
          'origin=web{url=https://example.invalid/x}';
      final p = TaintedPrompt.fromJson({
        'request_id': 'r',
        'audit_id': 'deadbeef00000000',
        'summary': summary,
        'sink': 'WebFetch',
        'issued_at': 0,
      });
      expect(p.summary, summary);
    });
  });
}
