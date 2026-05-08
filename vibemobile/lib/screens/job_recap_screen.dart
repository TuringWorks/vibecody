// M1.2 — JobRecapScreen
//
// Read-only view of a background-job recap, reachable from the Sessions
// list when a job has reached terminal state (complete | failed |
// cancelled). Renders the shared RecapCard plus a Resume action that
// hits POST /v1/resume on the daemon. Mobile never generates job
// recaps; the daemon's J1.2 terminal-state hook owns generation.
//
// Spec: docs/design/recap-resume/02-job.md § Per-surface UX → vibemobile.

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/recap.dart';
import '../services/api_client.dart';
import '../theme/app_theme.dart';
import '../widgets/recap_card.dart';

class JobRecapScreen extends StatefulWidget {
  /// Daemon base URL (already resolved by AuthService).
  final String baseUrl;

  /// Bearer token for the paired daemon.
  final String token;

  /// Job session_id (subject_id of the recap).
  final String jobId;

  /// Original job task — shown as a fallback when the daemon has no
  /// recap yet so the screen always has something to display.
  final String taskPreview;

  const JobRecapScreen({
    super.key,
    required this.baseUrl,
    required this.token,
    required this.jobId,
    required this.taskPreview,
  });

  @override
  State<JobRecapScreen> createState() => _JobRecapScreenState();
}

class _JobRecapScreenState extends State<JobRecapScreen> {
  Recap? _recap;
  bool _loading = true;
  bool _resuming = false;
  String? _errorMessage;

  ApiClient get _api => context.read<ApiClient>();

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    final r = await _api.getJobRecap(widget.baseUrl, widget.token, widget.jobId);
    if (!mounted) return;
    setState(() {
      _recap = r;
      _loading = false;
    });
  }

  Future<void> _resume() async {
    final r = _recap;
    if (r == null || _resuming) return;
    setState(() => _resuming = true);
    final newId = await _api.resumeFromRecap(
      widget.baseUrl,
      widget.token,
      recapId: r.id,
      branch: false,
    );
    if (!mounted) return;
    setState(() => _resuming = false);
    if (newId == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Resume failed — daemon may be offline')),
      );
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Resumed as $newId')),
    );
    Navigator.of(context).pop(newId);
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Job recap'),
        actions: [
          IconButton(
            key: const Key('job-recap-refresh'),
            icon: const Icon(Icons.refresh_rounded),
            onPressed: _loading ? null : _load,
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _recap == null
              ? _NoRecap(taskPreview: widget.taskPreview, color: c.textSecondary)
              : ListView(
                  children: [
                    RecapCard(
                      recap: _recap!,
                      onResume: _resuming ? null : _resume,
                    ),
                    if (_errorMessage != null)
                      Padding(
                        padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
                        child: Text(
                          _errorMessage!,
                          style: TextStyle(fontSize: 12, color: c.accentRed),
                        ),
                      ),
                  ],
                ),
    );
  }
}

class _NoRecap extends StatelessWidget {
  final String taskPreview;
  final Color color;

  const _NoRecap({required this.taskPreview, required this.color});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.history_toggle_off_rounded, size: 48, color: color.withValues(alpha: 0.5)),
            const SizedBox(height: 12),
            Text(
              'No recap yet',
              style: Theme.of(context).textTheme.bodyLarge,
            ),
            const SizedBox(height: 4),
            Text(
              'The daemon auto-recaps when a job ends (complete / failed / cancelled).',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 12, color: color),
            ),
            const SizedBox(height: 16),
            Text(
              taskPreview,
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 12, color: color),
            ),
          ],
        ),
      ),
    );
  }
}
