import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../theme/app_theme.dart';

/// G2.2 — Mobile Goals screen. Read-mostly view of `/v1/goals` across
/// every paired machine. Tapping a goal opens a detail sheet with a
/// "Start session" action; the heavy lifting (plan generation, links,
/// criteria editing) stays on VibeUI and the daemon REPL.
class GoalsScreen extends StatefulWidget {
  const GoalsScreen({super.key});

  @override
  State<GoalsScreen> createState() => _GoalsScreenState();
}

class _GoalsScreenState extends State<GoalsScreen> {
  List<Map<String, dynamic>> _goals = [];
  bool _loading = true;
  String _statusFilter = 'active';

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    setState(() => _loading = true);
    final auth = context.read<AuthService>();
    final api = context.read<ApiClient>();
    final all = <Map<String, dynamic>>[];

    for (final cred in auth.machines) {
      try {
        final resp = await api.listGoals(
          cred.baseUrl,
          cred.token,
          status: _statusFilter == 'all' ? null : _statusFilter,
        );
        final goals = (resp['goals'] as List?) ?? const [];
        // G6.2 — gather the set of workspaces present in this machine's
        // goal list so we can fetch one pin per workspace + the global
        // slot, and tag every goal with `_pinned: true/false`.
        final wsSet = <String?>{};
        for (final g in goals) {
          if (g is! Map) continue;
          wsSet.add(g['workspace'] as String?);
        }
        final pinned = <String?, String?>{}; // workspace → pinned goal_id
        for (final ws in wsSet) {
          try {
            final pin = await api.getCurrentGoal(cred.baseUrl, cred.token, workspace: ws);
            pinned[ws] = pin['goal_id'] as String?;
          } catch (_) {
            pinned[ws] = null;
          }
        }
        for (final g in goals) {
          if (g is! Map) continue;
          final map = Map<String, dynamic>.from(g);
          map['_machine_name'] = cred.machineName;
          map['_base_url'] = cred.baseUrl;
          map['_token'] = cred.token;
          final ws = map['workspace'] as String?;
          map['_pinned'] = pinned[ws] == map['id'];
          all.add(map);
        }
      } catch (_) {
        // Older daemons without /v1/goals — silent skip per pattern.
      }
    }

    all.sort((a, b) =>
        (b['updated_at'] ?? '').toString().compareTo((a['updated_at'] ?? '').toString()));

    if (mounted) {
      setState(() {
        _goals = all;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Goals'),
        actions: [
          IconButton(icon: const Icon(Icons.refresh_rounded), onPressed: _refresh),
        ],
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(48),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
            child: Row(
              children: [
                for (final s in ['active', 'paused', 'done', 'abandoned', 'all'])
                  Padding(
                    padding: const EdgeInsets.only(right: 6),
                    child: ChoiceChip(
                      label: Text(s),
                      selected: _statusFilter == s,
                      onSelected: (_) {
                        setState(() => _statusFilter = s);
                        _refresh();
                      },
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _goals.isEmpty
              ? Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.track_changes_rounded,
                          size: 48, color: c.textSecondary.withValues(alpha: 0.5)),
                      const SizedBox(height: 16),
                      Text(
                        'No goals${_statusFilter == 'all' ? '' : ' (status: $_statusFilter)'}',
                        style: Theme.of(context).textTheme.bodyLarge,
                      ),
                      const SizedBox(height: 4),
                      Text(
                        'Create one from VibeUI or the CLI (`/goal new`).',
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                )
              : RefreshIndicator(
                  onRefresh: _refresh,
                  child: ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: _goals.length,
                    itemBuilder: (_, i) => _GoalCard(
                      goal: _goals[i],
                      onTap: () => _openDetail(_goals[i]),
                    ),
                  ),
                ),
    );
  }

  void _openDetail(Map<String, dynamic> goal) {
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (_) => _GoalDetailSheet(
        goalSummary: goal,
        onStarted: () {
          Navigator.of(context).pop();
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('Session started on the goal')),
          );
        },
      ),
    );
  }
}

class _GoalCard extends StatelessWidget {
  const _GoalCard({required this.goal, required this.onTap});

  final Map<String, dynamic> goal;
  final VoidCallback onTap;

  Color _statusColor(BuildContext ctx, String status) {
    switch (status) {
      case 'active':
        return Colors.green.shade400;
      case 'paused':
        return Colors.blue.shade400;
      case 'done':
        return Colors.grey;
      case 'abandoned':
        return Colors.orange.shade400;
      default:
        return ctx.vibeColors.textSecondary;
    }
  }

  String _workspaceLabel(String? ws) {
    if (ws == null || ws.isEmpty) return 'global';
    final parts = ws.split('/').where((s) => s.isNotEmpty).toList();
    return parts.isEmpty ? ws : parts.last;
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final status = (goal['status'] ?? 'active') as String;
    final title = (goal['title'] ?? '(untitled)') as String;
    final machine = (goal['_machine_name'] ?? '') as String;
    final ws = goal['workspace'] as String?;
    final id = (goal['id'] ?? '') as String;
    final short = id.length > 8 ? id.substring(0, 8) : id;
    final isPinned = goal['_pinned'] == true;

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: _statusColor(context, status).withValues(alpha: 0.18),
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Text(
                      status,
                      style: TextStyle(
                        color: _statusColor(context, status),
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    _workspaceLabel(ws),
                    style: TextStyle(color: c.textSecondary, fontSize: 12),
                  ),
                  const Spacer(),
                  Text(
                    short,
                    style: TextStyle(
                      color: c.textSecondary,
                      fontSize: 11,
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 6),
              Row(
                children: [
                  if (isPinned) ...[
                    Icon(
                      Icons.star_rounded,
                      size: 14,
                      color: Theme.of(context).colorScheme.primary,
                      semanticLabel: 'pinned current goal',
                    ),
                    const SizedBox(width: 4),
                  ],
                  Expanded(
                    child: Text(title, style: Theme.of(context).textTheme.titleMedium),
                  ),
                ],
              ),
              if (machine.isNotEmpty) ...[
                const SizedBox(height: 6),
                Text(
                  '@ $machine',
                  style: TextStyle(color: c.textSecondary, fontSize: 11),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _GoalDetailSheet extends StatefulWidget {
  const _GoalDetailSheet({required this.goalSummary, required this.onStarted});

  final Map<String, dynamic> goalSummary;
  final VoidCallback onStarted;

  @override
  State<_GoalDetailSheet> createState() => _GoalDetailSheetState();
}

class _GoalDetailSheetState extends State<_GoalDetailSheet> {
  Map<String, dynamic>? _goal;
  List<Map<String, dynamic>> _links = [];
  bool _loading = true;
  bool _starting = false;
  // G6.2 — pin state for this goal. `_isPinned` is the workspace-local
  // pin (matching the goal's own `workspace` field); the global slot
  // isn't exposed in the mobile detail sheet today.
  bool _isPinned = false;
  bool _pinning = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final api = context.read<ApiClient>();
    final baseUrl = widget.goalSummary['_base_url'] as String;
    final token = widget.goalSummary['_token'] as String;
    final id = widget.goalSummary['id'] as String;
    try {
      final resp = await api.getGoal(baseUrl, token, id);
      // Best-effort pin lookup — failure leaves `_isPinned = false`.
      String? pinnedId;
      try {
        final ws = widget.goalSummary['workspace'] as String?;
        final pin = await api.getCurrentGoal(baseUrl, token, workspace: ws);
        pinnedId = pin['goal_id'] as String?;
      } catch (_) {}
      if (mounted) {
        setState(() {
          _goal = resp['goal'] as Map<String, dynamic>?;
          _links = ((resp['links'] as List?) ?? const [])
              .whereType<Map>()
              .map((m) => Map<String, dynamic>.from(m))
              .toList();
          _isPinned = pinnedId == id;
          _loading = false;
        });
      }
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _togglePin() async {
    setState(() => _pinning = true);
    final api = context.read<ApiClient>();
    final baseUrl = widget.goalSummary['_base_url'] as String;
    final token = widget.goalSummary['_token'] as String;
    final id = widget.goalSummary['id'] as String;
    final ws = widget.goalSummary['workspace'] as String?;
    try {
      if (_isPinned) {
        await api.unpinGoal(baseUrl, token, workspace: ws);
      } else {
        await api.pinGoal(baseUrl, token, id, workspace: ws);
      }
      if (mounted) {
        setState(() {
          _isPinned = !_isPinned;
          _pinning = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(_isPinned ? 'Pinned as current goal' : 'Pin cleared'),
          ),
        );
      }
    } catch (e) {
      if (mounted) {
        setState(() => _pinning = false);
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Pin update failed: $e')),
        );
      }
    }
  }

  Future<void> _start() async {
    setState(() => _starting = true);
    final api = context.read<ApiClient>();
    final baseUrl = widget.goalSummary['_base_url'] as String;
    final token = widget.goalSummary['_token'] as String;
    final id = widget.goalSummary['id'] as String;
    try {
      await api.startGoal(baseUrl, token, id);
      widget.onStarted();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to start: $e')),
        );
        setState(() => _starting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final goal = _goal ?? widget.goalSummary;
    final title = (goal['title'] ?? '(untitled)') as String;
    final statement = (goal['statement'] ?? '') as String;
    final status = (goal['status'] ?? 'active') as String;
    final criteria = ((goal['success_criteria'] as List?) ?? const [])
        .whereType<String>()
        .toList();

    return DraggableScrollableSheet(
      initialChildSize: 0.6,
      minChildSize: 0.3,
      maxChildSize: 0.95,
      expand: false,
      builder: (_, scrollController) {
        return Padding(
          padding: const EdgeInsets.all(16),
          child: ListView(
            controller: scrollController,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(title, style: Theme.of(context).textTheme.titleLarge),
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                    decoration: BoxDecoration(
                      color: Theme.of(context).colorScheme.primary.withValues(alpha: 0.12),
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Text(status, style: const TextStyle(fontSize: 11)),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              if (statement.isNotEmpty)
                Text(statement, style: Theme.of(context).textTheme.bodyMedium),
              if (criteria.isNotEmpty) ...[
                const SizedBox(height: 16),
                Text('Success criteria', style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 6),
                for (final c in criteria) Text('• $c'),
              ],
              const SizedBox(height: 16),
              Text(
                'Linked sessions, jobs & recaps (${_links.length})',
                style: Theme.of(context).textTheme.titleSmall,
              ),
              const SizedBox(height: 6),
              if (_loading)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 12),
                  child: Center(child: CircularProgressIndicator()),
                )
              else if (_links.isEmpty)
                const Text('— none yet —')
              else
                for (final l in _links)
                  ListTile(
                    dense: true,
                    contentPadding: EdgeInsets.zero,
                    leading: Text((l['kind'] ?? '') as String,
                        style: const TextStyle(fontSize: 11)),
                    title: Text(
                      ((l['target_id'] ?? '') as String).substring(
                        0,
                        ((l['target_id'] ?? '') as String).length.clamp(0, 12),
                      ),
                      style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                    ),
                    subtitle: l['note'] != null
                        ? Text((l['note'] ?? '') as String,
                            style: const TextStyle(fontSize: 11))
                        : null,
                  ),
              const SizedBox(height: 16),
              Row(
                children: [
                  Expanded(
                    child: FilledButton.icon(
                      onPressed: _starting ? null : _start,
                      icon: const Icon(Icons.play_arrow_rounded),
                      label: Text(_starting ? 'Starting…' : 'Start a session on this goal'),
                    ),
                  ),
                  const SizedBox(width: 8),
                  OutlinedButton.icon(
                    onPressed: _pinning ? null : _togglePin,
                    icon: Icon(_isPinned ? Icons.star_rounded : Icons.star_outline_rounded),
                    label: Text(_isPinned ? 'Pinned' : 'Pin'),
                  ),
                ],
              ),
              const SizedBox(height: 24),
            ],
          ),
        );
      },
    );
  }
}
