import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/voice_service.dart';
import '../theme/app_theme.dart';

/// Composer mic button, shared by every VibeMobile chat screen.
///
/// Watches [VoiceService] directly so the screens don't each mirror its state
/// into their own `setState`. Errors surface as a SnackBar rather than being
/// swallowed — a mic that silently does nothing is the failure mode this whole
/// feature is trying to avoid.
class VoiceMicButton extends StatelessWidget {
  const VoiceMicButton({
    super.key,
    required this.baseUrl,
    required this.token,
    required this.onTranscript,
    this.enabled = true,
  });

  /// Daemon to upload to when on-device recognition isn't available.
  /// Null disables the button: there is no fallback to fall back to.
  final String? baseUrl;
  final String? token;
  final void Function(String text) onTranscript;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final voice = context.watch<VoiceService>();
    final snap = voice.snapshot;

    // Report an error once, after this frame — calling ScaffoldMessenger
    // during build is not allowed.
    final error = snap.error;
    if (error != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!context.mounted) return;
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(error)));
        voice.clearError();
      });
    }

    final canRecord = enabled && baseUrl != null && token != null;

    if (snap.isTranscribing) {
      return const Padding(
        padding: EdgeInsets.all(12),
        child: SizedBox(
          width: 20,
          height: 20,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
      );
    }

    return IconButton(
      onPressed: canRecord
          ? () => voice.toggle(
                onTranscript: onTranscript,
                baseUrl: baseUrl!,
                token: token!,
              )
          : null,
      tooltip: snap.isListening ? 'Stop recording' : 'Dictate',
      icon: Icon(
        snap.isListening ? Icons.stop_circle_outlined : Icons.mic_none_rounded,
        color: snap.isListening ? c.accentRed : c.accentBlue,
      ),
    );
  }
}

/// One-line partial transcript, shown above the composer while listening.
class VoicePartialStrip extends StatelessWidget {
  const VoicePartialStrip({super.key});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final snap = context.watch<VoiceService>().snapshot;
    if (!snap.isListening || snap.partial.isEmpty) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 0),
      child: Text(
        snap.partial,
        style: TextStyle(
          color: c.textMuted,
          fontStyle: FontStyle.italic,
          fontSize: 12,
        ),
        maxLines: 2,
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}
