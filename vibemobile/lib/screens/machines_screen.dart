import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../models/machine.dart';
import '../theme/app_theme.dart';
import 'machine_detail_screen.dart';
import 'pair_screen.dart';
import 'manual_connect_screen.dart';

/// Lists all paired machines with status indicators.
class MachinesScreen extends StatefulWidget {
  const MachinesScreen({super.key});

  @override
  State<MachinesScreen> createState() => _MachinesScreenState();
}

class _MachinesScreenState extends State<MachinesScreen> {
  final Map<String, Machine?> _machineDetails = {};
  final Map<String, bool> _reachable = {};
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

    for (final cred in auth.machines) {
      try {
        final healthy = await api.healthCheck(cred.baseUrl);
        _reachable[cred.machineId] = healthy;
        if (healthy) {
          final machines = await api.listMachines(cred.baseUrl, cred.token);
          if (machines.isNotEmpty) {
            _machineDetails[cred.machineId] = machines.first;
          }
        }
      } catch (_) {
        _reachable[cred.machineId] = false;
      }
    }
    if (mounted) setState(() => _loading = false);
  }

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthService>();
<<<<<<< HEAD
=======
    final c = context.vibeColors;
>>>>>>> c137a77b261988cafc873fac496424c3f2c18d3d

    return Scaffold(
      appBar: AppBar(
        title: const Text('Machines'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh_rounded),
            onPressed: _refresh,
          ),
        ],
      ),
      body: _loading && auth.machines.isEmpty
          ? const Center(child: CircularProgressIndicator())
          : auth.machines.isEmpty
              ? _buildEmpty(context)
              : RefreshIndicator(
                  onRefresh: _refresh,
                  child: ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: auth.machines.length,
                    itemBuilder: (context, index) {
                      final cred = auth.machines[index];
                      final machine = _machineDetails[cred.machineId];
                      final reachable = _reachable[cred.machineId] ?? false;
                      return _MachineCard(
                        cred: cred,
                        machine: machine,
                        reachable: reachable,
                        onTap: () => Navigator.push(
                          context,
                          MaterialPageRoute(builder: (_) => MachineDetailScreen(credential: cred)),
                        ),
                      );
                    },
                  ),
                ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => _showAddMenu(context),
        child: const Icon(Icons.add_rounded),
      ),
    );
  }

  Widget _buildEmpty(BuildContext context) {
    final c = context.vibeColors;
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.computer_rounded, size: 64, color: c.textSecondary.withValues(alpha: 0.5)),
          const SizedBox(height: 16),
          Text('No machines paired', style: Theme.of(context).textTheme.bodyLarge),
          const SizedBox(height: 8),
          Text('Tap + to connect to a VibeCody daemon', style: Theme.of(context).textTheme.bodyMedium),
        ],
      ),
    );
  }

  void _showAddMenu(BuildContext context) {
    final c = context.vibeColors;
    showModalBottomSheet(
      context: context,
      backgroundColor: c.bgSecondary,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox(height: 8),
            Container(width: 40, height: 4, decoration: BoxDecoration(color: c.borderColor, borderRadius: BorderRadius.circular(2))),
            ListTile(
              leading: Icon(Icons.qr_code_scanner_rounded, color: c.accentBlue),
              title: const Text('Scan QR Code'),
              subtitle: const Text('Scan from terminal'),
              onTap: () {
                Navigator.pop(ctx);
                Navigator.push(context, MaterialPageRoute(builder: (_) => const PairScreen()));
              },
            ),
            ListTile(
              leading: Icon(Icons.link_rounded, color: c.accentGreen),
              title: const Text('Connect Manually'),
              subtitle: const Text('Enter URL and token'),
              onTap: () {
                Navigator.pop(ctx);
                Navigator.push(context, MaterialPageRoute(builder: (_) => const ManualConnectScreen()));
              },
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }
}

class _MachineCard extends StatelessWidget {
  final MachineCredential cred;
  final Machine? machine;
  final bool reachable;
  final VoidCallback onTap;

  const _MachineCard({
    required this.cred,
    this.machine,
    required this.reachable,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final status = reachable ? (machine?.status ?? 'online') : 'offline';
    final statusColor = switch (status) {
      'online' || 'idle' => c.accentGreen,
      'busy' => c.accentOrange,
      _ => c.accentRed,
    };

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              // OS icon.
              Container(
                width: 48, height: 48,
                decoration: BoxDecoration(
                  color: c.bgTertiary,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Center(
                  child: Text(
                    machine?.osIcon ?? '💻',
                    style: const TextStyle(fontSize: 24),
                  ),
                ),
              ),
              const SizedBox(width: 16),
              // Info.
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      cred.machineName.isNotEmpty ? cred.machineName : cred.baseUrl,
                      style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      machine != null
                          ? '${machine!.os} · ${machine!.workspaceRoot}'
                          : cred.baseUrl,
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(fontSize: 12),
                      maxLines: 1, overflow: TextOverflow.ellipsis,
                    ),
                    if (machine != null && machine!.activeSessions > 0)
                      Padding(
                        padding: const EdgeInsets.only(top: 4),
                        child: Text(
                          '${machine!.activeSessions} active session${machine!.activeSessions > 1 ? 's' : ''}',
                          style: TextStyle(color: c.accentOrange, fontSize: 12),
                        ),
                      ),
                  ],
                ),
              ),
              // Status dot.
              Container(
                width: 10, height: 10,
                decoration: BoxDecoration(
                  color: statusColor,
                  shape: BoxShape.circle,
                  boxShadow: [BoxShadow(color: statusColor.withValues(alpha: 0.4), blurRadius: 6)],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
