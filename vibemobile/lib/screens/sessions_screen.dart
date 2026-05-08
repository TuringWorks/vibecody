import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../theme/app_theme.dart';
import 'job_recap_screen.dart';

/// Shows active and historical agent sessions across all paired machines.
class SessionsScreen extends StatefulWidget {
  const SessionsScreen({super.key});

  @override
  State<SessionsScreen> createState() => _SessionsScreenState();
}

class _SessionsScreenState extends State<SessionsScreen> {
  List<Map<String, dynamic>> _jobs = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    setState(() => _loading = true);
    final auth = context.read<AuthService>();
    final api = context.read<ApiClient>();
    final allJobs = <Map<String, dynamic>>[];

    for (final cred in auth.machines) {
      try {
        final sessions = await api.listSessions(cred.baseUrl, cred.token);
        for (final session in sessions) {
          session['_machine_name'] = cred.machineName;
          session['_base_url'] = cred.baseUrl;
          session['_token'] = cred.token;
          session['_recap_kind'] = 'session';
        }
        allJobs.addAll(sessions);
      } catch (_) {
        // Fall back to listJobs if /mobile/sessions not available.
        // Rows from /jobs are background-agent jobs and use the
        // job-recap path (M1.2).
        try {
          final jobs = await api.listJobs(cred.baseUrl, cred.token);
          for (final job in jobs) {
            job['_machine_name'] = cred.machineName;
            job['_base_url'] = cred.baseUrl;
            job['_token'] = cred.token;
            job['_recap_kind'] = 'job';
          }
          allJobs.addAll(jobs);
        } catch (_) {}
      }
    }

    // M1.1 + M1.2 — fetch the latest recap headline per row and
    // stitch it in. Session rows go to /v1/recap?kind=session, job
    // rows to kind=job. Best-effort: a missing or 4xx recap leaves
    // the row's existing preview untouched. Calls run in parallel.
    await Future.wait(allJobs.map((job) async {
      final sid = job['session_id'] ?? job['id'];
      if (sid is! String || sid.isEmpty) return;
      final baseUrl = job['_base_url'] as String?;
      final token = job['_token'] as String?;
      if (baseUrl == null || token == null) return;
      final kind = job['_recap_kind'] as String? ?? 'session';
      final recap = kind == 'job'
          ? await api.getJobRecap(baseUrl, token, sid)
          : await api.getSessionRecap(baseUrl, token, sid);
      if (recap != null && recap.headline.isNotEmpty) {
        job['_recap_headline'] = recap.headline;
      }
    }));

    allJobs.sort((a, b) => (b['started_at'] ?? 0).compareTo(a['started_at'] ?? 0));

    if (mounted) {
      setState(() {
        _jobs = allJobs;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Sessions'),
        actions: [
          IconButton(icon: const Icon(Icons.refresh_rounded), onPressed: _refresh),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _jobs.isEmpty
              ? Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.history_rounded, size: 48, color: c.textSecondary.withValues(alpha: 0.5)),
                      const SizedBox(height: 16),
                      Text('No sessions yet', style: Theme.of(context).textTheme.bodyLarge),
                    ],
                  ),
                )
              : RefreshIndicator(
                  onRefresh: _refresh,
                  child: ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: _jobs.length,
                    itemBuilder: (_, i) => _JobCard(
                      job: _jobs[i],
                      onCancel: () => _cancelJob(_jobs[i]),
                      onOpenRecap: () => _openJobRecap(_jobs[i]),
                    ),
                  ),
                ),
    );
  }

  /// M1.2 — open the JobRecapScreen for a terminal background-agent
  /// job. Only fires for rows tagged `_recap_kind == 'job'`; session
  /// rows ignore the gesture (their tap-target is the chat screen,
  /// not implemented here).
  void _openJobRecap(Map<String, dynamic> job) {
    if (job['_recap_kind'] != 'job') return;
    final sid = job['session_id'] ?? job['id'];
    final baseUrl = job['_base_url'];
    final token = job['_token'];
    if (sid is! String || baseUrl is! String || token is! String) return;
    Navigator.of(context).push(MaterialPageRoute(
      builder: (_) => JobRecapScreen(
        baseUrl: baseUrl,
        token: token,
        jobId: sid,
        taskPreview: job['task'] ?? job['_recap_headline'] ?? '',
      ),
    ));
  }

  Future<void> _cancelJob(Map<String, dynamic> job) async {
    final api = context.read<ApiClient>();
    try {
      await api.cancelJob(job['_base_url'], job['_token'], job['session_id']);
      _refresh();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to cancel: $e')),
        );
      }
    }
  }
}

class _JobCard extends StatelessWidget {
  final Map<String, dynamic> job;
  final VoidCallback onCancel;
  final VoidCallback onOpenRecap;

  const _JobCard({
    required this.job,
    required this.onCancel,
    required this.onOpenRecap,
  });

  bool get _isJobKind => job['_recap_kind'] == 'job';
  bool get _isTerminal {
    final s = job['status'];
    return s == 'complete' || s == 'failed' || s == 'cancelled';
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final status = job['status'] ?? 'unknown';
    final statusColor = switch (status) {
      'running' => c.accentBlue,
      'complete' => c.accentGreen,
      'failed' => c.accentRed,
      'cancelled' => c.textSecondary,
      _ => c.accentOrange,
    };
    final statusIcon = switch (status) {
      'running' => Icons.play_circle_rounded,
      'complete' => Icons.check_circle_rounded,
      'failed' => Icons.error_rounded,
      'cancelled' => Icons.cancel_rounded,
      _ => Icons.hourglass_top_rounded,
    };

    // M1.2 — terminal job rows are tappable; the tap opens the
    // JobRecapScreen so the user can review what happened and Resume.
    final tappable = _isJobKind && _isTerminal;

    return Card(
      key: Key('job-card-${job['session_id'] ?? job['id'] ?? ''}'),
      margin: const EdgeInsets.only(bottom: 10),
      child: InkWell(
        onTap: tappable ? onOpenRecap : null,
        borderRadius: BorderRadius.circular(8),
        child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(statusIcon, size: 18, color: statusColor),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    job['task'] ?? 'Unknown task',
                    style: const TextStyle(fontWeight: FontWeight.w600),
                    maxLines: 2, overflow: TextOverflow.ellipsis,
                  ),
                ),
                if (status == 'running')
                  IconButton(
                    icon: Icon(Icons.stop_circle_rounded, color: c.accentRed, size: 20),
                    onPressed: onCancel,
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                  ),
              ],
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                _InfoChip(label: job['_machine_name'] ?? '', icon: Icons.computer_rounded),
                const SizedBox(width: 8),
                _InfoChip(label: job['provider'] ?? '', icon: Icons.smart_toy_rounded),
                const Spacer(),
                Text(
                  _formatTime(job['started_at']),
                  style: TextStyle(fontSize: 11, color: c.textSecondary),
                ),
              ],
            ),
            if ((job['_recap_headline'] ?? job['last_message_preview'] ?? job['summary']) != null) ...[
              const SizedBox(height: 8),
              Text(
                // M1.1 — recap headline wins when present; falls back
                // to the previous preview/summary so rows from older
                // daemons keep working.
                job['_recap_headline'] ?? job['last_message_preview'] ?? job['summary'],
                style: TextStyle(fontSize: 12, color: c.textSecondary),
                maxLines: 2, overflow: TextOverflow.ellipsis,
              ),
            ],
            if (tappable) ...[
              const SizedBox(height: 6),
              Row(
                children: [
                  Icon(Icons.chevron_right_rounded, size: 14, color: c.textSecondary),
                  const SizedBox(width: 2),
                  Text(
                    'View recap',
                    style: TextStyle(fontSize: 11, color: c.accentBlue),
                  ),
                ],
              ),
            ],
          ],
        ),
        ),
      ),
    );
  }

  String _formatTime(dynamic ts) {
    if (ts == null || ts == 0) return '';
    final dt = DateTime.fromMillisecondsSinceEpoch((ts as int) * 1000);
    final diff = DateTime.now().difference(dt);
    if (diff.inMinutes < 1) return 'just now';
    if (diff.inHours < 1) return '${diff.inMinutes}m ago';
    if (diff.inDays < 1) return '${diff.inHours}h ago';
    return '${diff.inDays}d ago';
  }
}

class _InfoChip extends StatelessWidget {
  final String label;
  final IconData icon;
  const _InfoChip({required this.label, required this.icon});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    if (label.isEmpty) return const SizedBox.shrink();
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: c.bgTertiary,
        borderRadius: BorderRadius.circular(10),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 12, color: c.textSecondary),
          const SizedBox(width: 4),
          Text(label, style: TextStyle(fontSize: 11, color: c.textSecondary)),
        ],
      ),
    );
  }
}
