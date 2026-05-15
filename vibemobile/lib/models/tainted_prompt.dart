/// Mobile-side mirror of the daemon's `PendingPromptEvent` JSON shape
/// (defined in `vibecli-cli/src/tainted_http_bridge.rs`).
///
/// Carries only the audit summary — the underlying tainted bytes
/// never appear here. See `docs/security/tainted-data-flow.md` §8
/// for the threat-model invariant.
class TaintedPrompt {
  /// Daemon-generated id used to resolve via `POST /v1/tainted/respond`.
  final String requestId;

  /// Stable identifier matching `Tainted::audit_id()` of the value
  /// being gated. Same bytes appearing through multiple origins
  /// produce the same `auditId` — useful for client-side correlation
  /// and de-dup.
  final String auditId;

  /// `kind=… audit_id=… origin={fields}` — payload-free banner the
  /// user sees in the modal. Bounded length (256 chars per provenance
  /// field at the daemon).
  final String summary;

  /// Which sink fired the gate (`ToolCallArgument`, `McpArgument`,
  /// `RagDocument`, …). Surfaces as "About to run shell command" /
  /// "About to call MCP tool" in the sheet header.
  final String sink;

  /// Unix-seconds the daemon queued the prompt.
  final int issuedAt;

  const TaintedPrompt({
    required this.requestId,
    required this.auditId,
    required this.summary,
    required this.sink,
    required this.issuedAt,
  });

  factory TaintedPrompt.fromJson(Map<String, dynamic> j) => TaintedPrompt(
        requestId: j['request_id'] as String? ?? '',
        auditId: j['audit_id'] as String? ?? '',
        summary: j['summary'] as String? ?? '',
        sink: j['sink'] as String? ?? 'ToolCallArgument',
        issuedAt: j['issued_at'] as int? ?? 0,
      );

  /// Human-friendly sink label for the sheet header.
  String get sinkLabel {
    switch (sink) {
      case 'ToolCallArgument':
        return 'Run tool with untrusted argument';
      case 'McpArgument':
        return 'Call MCP tool with untrusted argument';
      case 'RagDocument':
        return 'Use retrieval-augmented document';
      case 'WebFetch':
        return 'Fetch web URL';
      case 'LlmRequestBody':
        return 'Send to LLM provider';
      case 'LogLine':
        return 'Emit log line';
      case 'ShellCommand':
        return 'Run shell command';
      default:
        return 'Confirm action with untrusted data';
    }
  }
}
