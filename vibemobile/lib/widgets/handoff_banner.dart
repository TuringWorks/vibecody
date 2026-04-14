import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/handoff_service.dart';
import '../theme/app_theme.dart';
import '../screens/chat_screen.dart';

/// iOS-style Handoff suggestion banner — shown when a recent session is
/// detected on a paired machine.
class HandoffBanner extends StatelessWidget {
  const HandoffBanner({super.key});

  @override
  Widget build(BuildContext context) {
    final handoff = context.watch<HandoffService>();
    if (!handoff.hasHandoff) return const SizedBox.shrink();

    final candidate = handoff.candidates.first;
    final c = context.vibeColors;

    return GestureDetector(
      onTap: () => _continueSession(context, candidate),
      child: Container(
        margin: const EdgeInsets.fromLTRB(16, 0, 16, 8),
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        decoration: BoxDecoration(
          color: c.bgSecondary,
          borderRadius: BorderRadius.circular(16),
          border: Border.all(color: c.accentBlue.withValues(alpha: 0.4)),
          boxShadow: [
            BoxShadow(
              color: c.accentBlue.withValues(alpha: 0.08),
              blurRadius: 12,
              offset: const Offset(0, 2),
            ),
          ],
        ),
        child: Row(
          children: [
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: c.accentBlue.withValues(alpha: 0.15),
                borderRadius: BorderRadius.circular(10),
              ),
              child: Icon(Icons.computer_rounded, color: c.accentBlue, size: 20),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    children: [
                      Flexible(
                        child: Text(
                          'Continue on ${candidate.machineName}',
                          style: TextStyle(
                            fontSize: 13,
                            fontWeight: FontWeight.w600,
                            color: c.textPrimary,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      const SizedBox(width: 6),
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 6, vertical: 2),
                        decoration: BoxDecoration(
                          color: candidate.session.status == 'running'
                              ? c.accentGreen.withValues(alpha: 0.15)
                              : c.bgTertiary,
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: Text(
                          candidate.session.status == 'running'
                              ? 'Live'
                              : 'Recent',
                          style: TextStyle(
                            fontSize: 10,
                            fontWeight: FontWeight.w600,
                            color: candidate.session.status == 'running'
                                ? c.accentGreen
                                : c.textSecondary,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 2),
                  Text(
                    candidate.session.task,
                    style: TextStyle(fontSize: 12, color: c.textSecondary),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Icon(Icons.arrow_forward_ios_rounded,
                size: 14, color: c.textSecondary),
          ],
        ),
      ),
    );
  }

  void _continueSession(BuildContext context, HandoffCandidate candidate) {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => ChatScreen(
          resumeMachineId: candidate.machineId,
          resumeSessionId: candidate.session.sessionId,
          resumeTask: candidate.session.task,
        ),
      ),
    );
  }
}
