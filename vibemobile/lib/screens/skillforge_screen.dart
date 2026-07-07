import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../theme/app_theme.dart';

/// Gap item G6 — Mobile SkillForge screen. Read-only catalogue browse
/// across every paired machine, mirroring `GoalsScreen`. Tapping a skill
/// opens a detail sheet (summary + cached SkillLens report). The heavy
/// `score`/`train`/`promote` mutations stay desktop-only — they need a
/// toolbar-selected LLM (STRICT). The detail sheet has an optional
/// "train status" lookup: paste a job id (from `/skillforge train` on
/// desktop) and see `Running | Done | Failed | Cancelled`.
class SkillforgeScreen extends StatefulWidget {
  const SkillforgeScreen({super.key});

  @override
  State<SkillforgeScreen> createState() => _SkillforgeScreenState();
}

class _SkillforgeScreenState extends State<SkillforgeScreen> {
  List<Map<String, dynamic>> _skills = [];
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
    final all = <Map<String, dynamic>>[];

    for (final cred in auth.machines) {
      try {
        final resp = await api.skilllensSkills(cred.baseUrl, cred.token);
        final skills = (resp['skills'] as List?) ?? const [];
        for (final s in skills) {
          if (s is! Map) continue;
          final map = Map<String, dynamic>.from(s);
          map['_machine_name'] = cred.machineName;
          map['_base_url'] = cred.baseUrl;
          map['_token'] = cred.token;
          all.add(map);
        }
      } catch (_) {
        // Older daemons without /v1/skilllens/* — silent skip per pattern.
      }
    }

    all.sort((a, b) =>
        (a['name'] ?? '').toString().compareTo((b['name'] ?? '').toString()));

    if (mounted) {
      setState(() {
        _skills = all;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(
        title: const Text('SkillForge'),
        actions: [
          IconButton(
              icon: const Icon(Icons.refresh_rounded), onPressed: _refresh),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _skills.isEmpty
              ? Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.school_outlined,
                          size: 48, color: c.textSecondary.withValues(alpha: 0.5)),
                      const SizedBox(height: 16),
                      Text(
                        'No skills surfaced',
                        style: Theme.of(context).textTheme.bodyLarge,
                      ),
                      const SizedBox(height: 4),
                      Text(
                        'Pair a machine running `vibecli serve`, or run /skillforge refresh on desktop.',
                        style: Theme.of(context).textTheme.bodySmall,
                        textAlign: TextAlign.center,
                      ),
                    ],
                  ),
                )
              : RefreshIndicator(
                  onRefresh: _refresh,
                  child: ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: _skills.length,
                    itemBuilder: (_, i) => _SkillCard(
                      skill: _skills[i],
                      onTap: () => _openDetail(_skills[i]),
                    ),
                  ),
                ),
    );
  }

  void _openDetail(Map<String, dynamic> skill) {
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (_) => _SkillDetailSheet(skillSummary: skill),
    );
  }
}

class _SkillCard extends StatelessWidget {
  const _SkillCard({required this.skill, required this.onTap});

  final Map<String, dynamic> skill;
  final VoidCallback onTap;

  String _fmtPct(dynamic v) {
    if (v == null) return '—';
    final d = (v as num).toDouble();
    return d.toStringAsFixed(2);
  }

  Color _evoColor(double? v) {
    if (v == null) return Colors.grey;
    if (v >= 0.7) return Colors.green.shade400;
    if (v >= 0.4) return Colors.orange.shade400;
    return Colors.grey;
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final name = (skill['name'] ?? '(unnamed)') as String;
    final category = (skill['category'] ?? '') as String;
    final machine = (skill['_machine_name'] ?? '') as String;
    final source = (skill['source'] ?? 'builtin') as String;
    final cov = skill['trigger_coverage'];
    final evo = skill['target_evolvability'] is num
        ? (skill['target_evolvability'] as num).toDouble()
        : null;

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
                  Expanded(
                    child: Text(name,
                        style: Theme.of(context).textTheme.titleMedium),
                  ),
                  Container(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: (evo != null ? _evoColor(evo) : Colors.grey)
                          .withValues(alpha: 0.18),
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Text(
                      'evo ${_fmtPct(skill['target_evolvability'])}',
                      style: TextStyle(
                        color: _evoColor(evo),
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 6),
              Row(
                children: [
                  if (category.isNotEmpty)
                    Text(category,
                        style: TextStyle(color: c.textSecondary, fontSize: 12)),
                  if (category.isNotEmpty) const SizedBox(width: 8),
                  Text('cov ${_fmtPct(cov)}',
                      style: TextStyle(color: c.textSecondary, fontSize: 12)),
                  const Spacer(),
                  Text('$source · @ $machine',
                      style: TextStyle(color: c.textSecondary, fontSize: 11)),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SkillDetailSheet extends StatefulWidget {
  const _SkillDetailSheet({required this.skillSummary});

  final Map<String, dynamic> skillSummary;

  @override
  State<_SkillDetailSheet> createState() => _SkillDetailSheetState();
}

class _SkillDetailSheetState extends State<_SkillDetailSheet> {
  Map<String, dynamic>? _detail;
  bool _loading = true;
  final _jobCtrl = TextEditingController();
  Map<String, dynamic>? _jobStatus;
  bool _jobLoading = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _jobCtrl.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final api = context.read<ApiClient>();
    final baseUrl = widget.skillSummary['_base_url'] as String;
    final token = widget.skillSummary['_token'] as String;
    final name = widget.skillSummary['name'] as String;
    try {
      final resp = await api.skilllensSkill(baseUrl, token, name);
      if (mounted) {
        setState(() {
          _detail = resp;
          _loading = false;
        });
      }
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _checkJob() async {
    final jobId = _jobCtrl.text.trim();
    if (jobId.isEmpty) return;
    setState(() => _jobLoading = true);
    final api = context.read<ApiClient>();
    final baseUrl = widget.skillSummary['_base_url'] as String;
    final token = widget.skillSummary['_token'] as String;
    try {
      final resp = await api.skilloptStatus(baseUrl, token, jobId);
      if (mounted) setState(() => _jobStatus = resp);
    } catch (e) {
      if (mounted) {
        setState(() => _jobStatus = {'error': e.toString()});
      }
    } finally {
      if (mounted) setState(() => _jobLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final summary = widget.skillSummary;
    final name = (summary['name'] ?? '(unnamed)') as String;
    final category = (summary['category'] ?? '') as String;
    final report = _detail?['report'] as Map<String, dynamic>?;
    final body = (_detail?['body'] as String?) ?? '';

    final jobState = _jobStatus?['state'] as String?;
    final jobError = _jobStatus?['error'] as String?;

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
              Text(name, style: Theme.of(context).textTheme.titleLarge),
              if (category.isNotEmpty) ...[
                const SizedBox(height: 4),
                Text(category,
                    style: Theme.of(context).textTheme.bodySmall),
              ],
              const SizedBox(height: 12),
              if (_loading)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 12),
                  child: Center(child: CircularProgressIndicator()),
                )
              else ...[
                Text('Cached SkillLens report',
                    style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 6),
                _reportRow('Trigger coverage', summary['trigger_coverage']),
                _reportRow(
                    'Extraction efficacy', summary['extraction_efficacy']),
                _reportRow(
                    'Target evolvability', summary['target_evolvability']),
                if (report != null) ...[
                  const SizedBox(height: 8),
                  _reportRow('Token cost', report['token_cost']),
                ],
                if (body.isNotEmpty) ...[
                  const SizedBox(height: 16),
                  Text('Skill body',
                      style: Theme.of(context).textTheme.titleSmall),
                  const SizedBox(height: 6),
                  Text(
                    body.length > 600 ? '${body.substring(0, 600)}…' : body,
                    style: const TextStyle(
                        fontFamily: 'monospace', fontSize: 11),
                  ),
                ],
              ],
              const SizedBox(height: 20),
              Text('Train status',
                  style: Theme.of(context).textTheme.titleSmall),
              const SizedBox(height: 4),
              Text(
                'Paste a job id from `/skillforge train` on desktop to check its state.',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _jobCtrl,
                      decoration: const InputDecoration(
                        labelText: 'Job id',
                        isDense: true,
                        border: OutlineInputBorder(),
                      ),
                      onSubmitted: (_) => _checkJob(),
                    ),
                  ),
                  const SizedBox(width: 8),
                  FilledButton.icon(
                    onPressed: _jobLoading ? null : _checkJob,
                    icon: const Icon(Icons.search_rounded),
                    label: const Text('Check'),
                  ),
                ],
              ),
              if (_jobLoading)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 8),
                  child: Center(child: CircularProgressIndicator()),
                )
              else if (jobState != null) ...[
                const SizedBox(height: 8),
                Text('State: $jobState',
                    style: TextStyle(
                      color: _jobStateColor(jobState),
                      fontWeight: FontWeight.w600,
                    )),
              ] else if (jobError != null) ...[
                const SizedBox(height: 8),
                Text(jobError,
                    style: TextStyle(color: Colors.red.shade400, fontSize: 12)),
              ],
              const SizedBox(height: 24),
            ],
          ),
        );
      },
    );
  }

  Widget _reportRow(String label, dynamic value) {
    String text;
    if (value == null) {
      text = '—';
    } else if (value is num) {
      text = value.toStringAsFixed(2);
    } else {
      text = value.toString();
    }
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: [
          Text('$label: ',
              style: TextStyle(
                  color: Theme.of(context).textTheme.bodySmall?.color,
                  fontSize: 12)),
          Text(text, style: const TextStyle(fontSize: 12)),
        ],
      ),
    );
  }

  Color _jobStateColor(String state) {
    switch (state) {
      case 'running':
        return Colors.blue.shade400;
      case 'done':
        return Colors.green.shade400;
      case 'failed':
        return Colors.red.shade400;
      case 'cancelled':
        return Colors.orange.shade400;
      default:
        return Theme.of(context).textTheme.bodyMedium?.color ?? Colors.grey;
    }
  }
}