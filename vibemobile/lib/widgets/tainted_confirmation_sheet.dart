// tainted_confirmation_sheet.dart — DREAD #1 Slice G part 3 mobile UI.
//
// Modal bottom sheet that surfaces a pending tainted-argument prompt
// from `TaintedService` and lets the user approve or deny. Mirrors
// the VibeCoder WebView `TaintedConfirmationModal` UX: head-of-queue
// render, payload-free summary, Deny-by-default visual weight.
//
// The widget only renders when `TaintedService.headPrompt != null`.
// It does not need any of its own state — `TaintedService` owns the
// queue, this is a pure projection.

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/tainted_prompt.dart';
import '../services/tainted_service.dart';

class TaintedConfirmationSheet extends StatelessWidget {
  const TaintedConfirmationSheet({super.key});

  @override
  Widget build(BuildContext context) {
    final svc = context.watch<TaintedService>();
    final head = svc.headPrompt;
    if (head == null) return const SizedBox.shrink();
    return _Sheet(prompt: head, queuedBehind: svc.queuedCount - 1);
  }
}

class _Sheet extends StatelessWidget {
  final TaintedPrompt prompt;
  final int queuedBehind;

  const _Sheet({required this.prompt, required this.queuedBehind});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    // Render as a modal-style banner pinned to the bottom of the
    // current screen. We don't use `showModalBottomSheet` because the
    // service emits asynchronously — the imperative API would require
    // tracking shown-sheets and dismissing them on respond, which is
    // exactly the bookkeeping the declarative renderer avoids.
    return Positioned(
      left: 0,
      right: 0,
      bottom: 0,
      child: Material(
        color: Colors.transparent,
        child: Container(
          decoration: BoxDecoration(
            color: theme.colorScheme.surface,
            border: Border(
              top: BorderSide(color: theme.colorScheme.error, width: 2),
            ),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withValues(alpha: 0.25),
                blurRadius: 12,
                offset: const Offset(0, -2),
              ),
            ],
          ),
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 16),
          child: SafeArea(
            top: false,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  children: [
                    Icon(
                      Icons.shield_outlined,
                      color: theme.colorScheme.error,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        prompt.sinkLabel,
                        style: theme.textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  'The agent wants to use data that originated outside the '
                  'trust boundary. Review the audit summary before approving.',
                  style: theme.textTheme.bodySmall,
                ),
                const SizedBox(height: 12),
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  constraints: const BoxConstraints(maxHeight: 160),
                  child: SingleChildScrollView(
                    child: SelectableText(
                      prompt.summary,
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontFamily: 'monospace',
                        fontSize: 11,
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  queuedBehind > 0
                      ? 'audit_id: ${prompt.auditId}  ·  $queuedBehind more pending'
                      : 'audit_id: ${prompt.auditId}',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurface.withValues(alpha: 0.6),
                    fontSize: 10,
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton(
                        onPressed: () => context
                            .read<TaintedService>()
                            .respond(prompt.requestId, false),
                        child: const Text('Deny'),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: FilledButton(
                        style: FilledButton.styleFrom(
                          backgroundColor: theme.colorScheme.error,
                          foregroundColor: theme.colorScheme.onError,
                        ),
                        onPressed: () => context
                            .read<TaintedService>()
                            .respond(prompt.requestId, true),
                        child: const Text('Approve'),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
