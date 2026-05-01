import 'package:flutter/material.dart';
import '../models/recap.dart';
import '../theme/app_theme.dart';

/// M1.1 — pinned card that summarises a session at a glance. Reads
/// only; mobile never generates recaps. Spec:
/// docs/design/recap-resume/01-session.md § Per-surface UX → vibemobile.
class RecapCard extends StatefulWidget {
  final Recap recap;

  /// Fired when the user taps the "Resume" affordance. Mobile leaves
  /// the actual /v1/resume call to the parent so this widget stays
  /// presentational and easy to widget-test.
  final VoidCallback? onResume;

  /// When true the body (bullets, next, artifacts) starts hidden; the
  /// user expands by tapping the header.
  final bool defaultCollapsed;

  const RecapCard({
    super.key,
    required this.recap,
    this.onResume,
    this.defaultCollapsed = false,
  });

  @override
  State<RecapCard> createState() => _RecapCardState();
}

class _RecapCardState extends State<RecapCard> {
  late bool _collapsed;

  @override
  void initState() {
    super.initState();
    _collapsed = widget.defaultCollapsed;
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final r = widget.recap;

    return Container(
      margin: const EdgeInsets.fromLTRB(12, 8, 12, 4),
      decoration: BoxDecoration(
        color: c.bgSecondary,
        border: Border.all(color: c.borderColor),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header — always visible, tappable to collapse/expand.
          InkWell(
            onTap: () => setState(() => _collapsed = !_collapsed),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
              child: Row(
                children: [
                  Icon(
                    _collapsed ? Icons.chevron_right : Icons.expand_more,
                    size: 18,
                    color: c.textSecondary,
                    semanticLabel: _collapsed ? 'Expand recap' : 'Collapse recap',
                  ),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      r.headline,
                      style: TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                        color: c.textPrimary,
                      ),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 8),
                  _GeneratorBadge(generator: r.generator),
                ],
              ),
            ),
          ),
          if (!_collapsed) _Body(recap: r, onResume: widget.onResume),
        ],
      ),
    );
  }
}

class _Body extends StatelessWidget {
  final Recap recap;
  final VoidCallback? onResume;

  const _Body({required this.recap, this.onResume});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (recap.bullets.isNotEmpty) ...[
            _SectionLabel(label: 'What happened'),
            for (final b in recap.bullets) _Bullet(text: b),
          ],
          if (recap.nextActions.isNotEmpty) ...[
            const SizedBox(height: 8),
            _SectionLabel(label: 'Next'),
            for (final a in recap.nextActions) _Bullet(text: a),
          ],
          if (recap.artifacts.isNotEmpty) ...[
            const SizedBox(height: 8),
            _SectionLabel(label: 'Artifacts'),
            for (final a in recap.artifacts) _ArtifactRow(artifact: a),
          ],
          const SizedBox(height: 12),
          Align(
            alignment: Alignment.centerRight,
            child: TextButton.icon(
              key: const Key('recap-resume-btn'),
              onPressed: onResume,
              icon: const Icon(Icons.play_arrow_rounded, size: 16),
              label: const Text('Resume from here'),
              style: TextButton.styleFrom(
                foregroundColor: c.accentBlue,
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionLabel extends StatelessWidget {
  final String label;
  const _SectionLabel({required this.label});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Text(
        label.toUpperCase(),
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.6,
          color: c.textSecondary,
        ),
      ),
    );
  }
}

class _Bullet extends StatelessWidget {
  final String text;
  const _Bullet({required this.text});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    return Padding(
      padding: const EdgeInsets.only(left: 4, top: 1, bottom: 1),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('• ', style: TextStyle(color: c.textSecondary, fontSize: 13)),
          Expanded(
            child: Text(text, style: TextStyle(fontSize: 13, color: c.textPrimary)),
          ),
        ],
      ),
    );
  }
}

class _ArtifactRow extends StatelessWidget {
  final RecapArtifact artifact;
  const _ArtifactRow({required this.artifact});

  IconData _iconFor(String kind) {
    switch (kind) {
      case 'file':
        return Icons.insert_drive_file_outlined;
      case 'job':
        return Icons.work_outline;
      case 'diff':
        return Icons.compare_arrows_rounded;
      case 'url':
        return Icons.link;
      default:
        return Icons.circle_outlined;
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 1),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Icon(_iconFor(artifact.kind), size: 13, color: c.textSecondary),
          const SizedBox(width: 6),
          Text(artifact.label, style: TextStyle(fontSize: 13, color: c.textPrimary)),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              artifact.locator,
              style: TextStyle(fontSize: 11, color: c.textSecondary),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

class _GeneratorBadge extends StatelessWidget {
  final RecapGenerator generator;
  const _GeneratorBadge({required this.generator});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final bg = switch (generator.type) {
      'llm' => c.accentBlue.withValues(alpha: 0.18),
      'user_edited' => c.accentOrange.withValues(alpha: 0.18),
      _ => c.bgTertiary,
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text(
        generator.label(),
        style: TextStyle(fontSize: 10, color: c.textSecondary),
      ),
    );
  }
}
