import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../models/machine.dart';
import '../theme/app_theme.dart';

/// Detailed view of a single paired machine: info, sessions, dispatches, git.
class MachineDetailScreen extends StatefulWidget {
  final MachineCredential credential;
  const MachineDetailScreen({super.key, required this.credential});

  @override
  State<MachineDetailScreen> createState() => _MachineDetailScreenState();
}

class _MachineDetailScreenState extends State<MachineDetailScreen> with SingleTickerProviderStateMixin {
  late TabController _tabController;
  Machine? _machine;
  List<DispatchTask> _dispatches = [];
  List<Map<String, dynamic>> _jobs = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 4, vsync: this);
    _refresh();
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  Future<void> _refresh() async {
    setState(() => _loading = true);
    final api = context.read<ApiClient>();
    final cred = widget.credential;

    try {
      final machines = await api.listMachines(cred.baseUrl, cred.token);
      if (machines.isNotEmpty) _machine = machines.first;
      _dispatches = await api.machineDispatches(cred.baseUrl, cred.token, cred.machineId);
      _jobs = await api.listJobs(cred.baseUrl, cred.token);
    } catch (_) {}

    if (mounted) setState(() => _loading = false);
  }

  @override
  Widget build(BuildContext context) {
    final cred = widget.credential;
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(
        title: Text(cred.machineName.isNotEmpty ? cred.machineName : cred.baseUrl),
        actions: [
          IconButton(icon: const Icon(Icons.refresh_rounded), onPressed: _refresh),
        ],
        bottom: TabBar(
          controller: _tabController,
          indicatorColor: c.accentBlue,
          labelColor: c.accentBlue,
          unselectedLabelColor: c.textSecondary,
          tabs: const [
            Tab(text: 'Overview'),
            Tab(text: 'Sessions'),
            Tab(text: 'Dispatch'),
            Tab(text: 'Actions'),
          ],
        ),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : TabBarView(
              controller: _tabController,
              children: [
                _OverviewTab(machine: _machine, credential: cred),
                _SessionsTab(jobs: _jobs),
                _DispatchTab(dispatches: _dispatches),
                _ActionsTab(credential: cred),
              ],
            ),
    );
  }
}

class _OverviewTab extends StatelessWidget {
  final Machine? machine;
  final MachineCredential credential;
  const _OverviewTab({this.machine, required this.credential});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;

    if (machine == null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.cloud_off_rounded, size: 48, color: c.accentRed),
            const SizedBox(height: 16),
            const Text('Machine unreachable'),
            const SizedBox(height: 8),
            Text(credential.baseUrl, style: TextStyle(fontSize: 12, color: c.textSecondary)),
          ],
        ),
      );
    }

    final m = machine!;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        children: [
          // Status banner.
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: m.isOnline
                  ? c.accentGreen.withValues(alpha: 0.1)
                  : c.accentRed.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(
                color: m.isOnline ? c.accentGreen.withValues(alpha: 0.3) : c.accentRed.withValues(alpha: 0.3),
              ),
            ),
            child: Column(
              children: [
                Text(m.osIcon, style: const TextStyle(fontSize: 40)),
                const SizedBox(height: 8),
                Text(m.name, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600)),
                const SizedBox(height: 4),
                _StatusBadge(status: m.status),
              ],
            ),
          ),
          const SizedBox(height: 16),

          _InfoCard(title: 'System', items: [
            _InfoItem('OS', '${m.os} (${m.arch})'),
            _InfoItem('CPU Cores', '${m.cpuCores}'),
            _InfoItem('Memory', '${m.memoryGb.toStringAsFixed(1)} GB'),
            _InfoItem('Disk Free', '${m.diskFreeGb.toStringAsFixed(1)} GB'),
            _InfoItem('Version', m.daemonVersion),
          ]),
          const SizedBox(height: 12),

          _InfoCard(title: 'Workspace', items: [
            _InfoItem('Root', m.workspaceRoot),
            _InfoItem('Port', '${m.daemonPort}'),
            _InfoItem('Sessions', '${m.activeSessions} / ${m.maxSessions}'),
            if (m.tailscaleIp != null) _InfoItem('Tailscale', m.tailscaleIp!),
          ]),
          const SizedBox(height: 12),

          if (m.capabilities.isNotEmpty) ...[
            const _InfoCard(title: 'Capabilities', items: []),
            Wrap(
              spacing: 6, runSpacing: 6,
              children: m.capabilities.map((cap) => Chip(label: Text(cap))).toList(),
            ),
          ],

          if (m.tags.isNotEmpty) ...[
            const SizedBox(height: 12),
            const _InfoCard(title: 'Tags', items: []),
            Wrap(
              spacing: 6, runSpacing: 6,
              children: m.tags.map((t) => Chip(
                label: Text(t),
                backgroundColor: c.accentBlue.withValues(alpha: 0.15),
              )).toList(),
            ),
          ],
        ],
      ),
    );
  }
}

class _SessionsTab extends StatelessWidget {
  final List<Map<String, dynamic>> jobs;
  const _SessionsTab({required this.jobs});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    if (jobs.isEmpty) {
      return Center(child: Text('No sessions', style: TextStyle(color: c.textSecondary)));
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: jobs.length,
      itemBuilder: (_, i) {
        final j = jobs[i];
        final status = j['status'] ?? '';
        return Card(
          margin: const EdgeInsets.only(bottom: 8),
          child: ListTile(
            leading: Icon(
              status == 'running' ? Icons.play_circle_rounded : Icons.check_circle_rounded,
              color: status == 'running' ? c.accentBlue : c.accentGreen,
            ),
            title: Text(j['task'] ?? '', maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Text('${j['provider'] ?? ''} · $status', style: const TextStyle(fontSize: 12)),
          ),
        );
      },
    );
  }
}

class _DispatchTab extends StatelessWidget {
  final List<DispatchTask> dispatches;
  const _DispatchTab({required this.dispatches});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    if (dispatches.isEmpty) {
      return Center(child: Text('No dispatches', style: TextStyle(color: c.textSecondary)));
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: dispatches.length,
      itemBuilder: (_, i) {
        final d = dispatches[i];
        return Card(
          margin: const EdgeInsets.only(bottom: 8),
          child: ListTile(
            leading: Icon(
              d.isRunning ? Icons.hourglass_top_rounded
                  : d.isComplete ? Icons.check_circle_rounded
                  : Icons.error_rounded,
              color: d.isRunning ? c.accentOrange
                  : d.isComplete ? c.accentGreen
                  : c.accentRed,
            ),
            title: Text(d.payload, maxLines: 1, overflow: TextOverflow.ellipsis),
            subtitle: Text('${d.dispatchType} · ${d.status}', style: const TextStyle(fontSize: 12)),
            trailing: d.result != null
                ? Icon(Icons.arrow_forward_ios_rounded, size: 14, color: c.textSecondary)
                : null,
          ),
        );
      },
    );
  }
}

class _ActionsTab extends StatelessWidget {
  final MachineCredential credential;
  const _ActionsTab({required this.credential});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        _ActionTile(icon: Icons.chat_rounded, color: c.accentBlue, title: 'Send Chat', subtitle: 'Chat with the AI agent', onTap: () => _quickDispatch(context, 'chat')),
        _ActionTile(icon: Icons.smart_toy_rounded, color: c.accentGreen, title: 'Start Agent Task', subtitle: 'Run an autonomous coding task', onTap: () => _quickDispatch(context, 'agent_task')),
        _ActionTile(icon: Icons.terminal_rounded, color: c.accentOrange, title: 'Run Command', subtitle: 'Execute a shell command', onTap: () => _quickDispatch(context, 'command')),
        _ActionTile(icon: Icons.source_rounded, color: Colors.purple, title: 'REPL Command', subtitle: 'Run a /slash command', onTap: () => _quickDispatch(context, 'repl_command')),
        _ActionTile(icon: Icons.folder_rounded, color: Colors.teal, title: 'File Operations', subtitle: 'List, read, or search files', onTap: () => _quickDispatch(context, 'file_op')),
        _ActionTile(icon: Icons.merge_type_rounded, color: Colors.deepOrange, title: 'Git Operations', subtitle: 'Status, commit, push, pull', onTap: () => _quickDispatch(context, 'git_op')),
      ],
    );
  }

  void _quickDispatch(BuildContext context, String type) {
    final c = context.vibeColors;
    final controller = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: c.bgSecondary,
        title: Text('Dispatch: $type'),
        content: TextField(
          controller: controller,
          maxLines: 3,
          decoration: InputDecoration(
            hintText: type == 'repl_command' ? '/status' : 'Enter payload...',
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          ElevatedButton(
            onPressed: () async {
              final auth = context.read<AuthService>();
              final api = context.read<ApiClient>();
              try {
                await api.dispatch(
                  credential.baseUrl, credential.token,
                  deviceId: auth.deviceId,
                  machineId: credential.machineId,
                  dispatchType: type,
                  payload: controller.text,
                );
                if (ctx.mounted) Navigator.pop(ctx);
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: const Text('Dispatched!'), backgroundColor: c.accentGreen),
                  );
                }
              } catch (e) {
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('Error: $e')),
                  );
                }
              }
            },
            child: const Text('Send'),
          ),
        ],
      ),
    );
  }
}

class _ActionTile extends StatelessWidget {
  final IconData icon;
  final Color color;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  const _ActionTile({
    required this.icon,
    required this.color,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        leading: Container(
          width: 40, height: 40,
          decoration: BoxDecoration(
            color: color.withValues(alpha: 0.15),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Icon(icon, color: color, size: 22),
        ),
        title: Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
        subtitle: Text(subtitle, style: const TextStyle(fontSize: 12)),
        trailing: Icon(Icons.arrow_forward_ios_rounded, size: 14, color: c.textSecondary),
        onTap: onTap,
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  final String status;
  const _StatusBadge({required this.status});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final color = switch (status) {
      'online' || 'idle' => c.accentGreen,
      'busy' => c.accentOrange,
      _ => c.accentRed,
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.2),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.5)),
      ),
      child: Text(status.toUpperCase(), style: TextStyle(color: color, fontSize: 11, fontWeight: FontWeight.bold)),
    );
  }
}

class _InfoCard extends StatelessWidget {
  final String title;
  final List<_InfoItem> items;
  const _InfoCard({required this.title, required this.items});

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14)),
            if (items.isNotEmpty) const SizedBox(height: 10),
            ...items.map((item) => Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(item.label, style: TextStyle(color: context.vibeColors.textSecondary, fontSize: 13)),
                  Flexible(
                    child: Text(item.value, style: const TextStyle(fontSize: 13), overflow: TextOverflow.ellipsis),
                  ),
                ],
              ),
            )),
          ],
        ),
      ),
    );
  }
}

class _InfoItem {
  final String label;
  final String value;
  _InfoItem(this.label, this.value);
}
