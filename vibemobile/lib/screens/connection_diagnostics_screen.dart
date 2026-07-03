import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/handoff_service.dart';
import '../services/discovery_service.dart';
import '../theme/app_theme.dart';

/// Screen showing connection diagnostics for paired machines.
class ConnectionDiagnosticsScreen extends StatelessWidget {
  const ConnectionDiagnosticsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthService>();
    final handoff = context.watch<HandoffService>();
    final discovery = context.watch<DiscoveryService>();
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Connection Diagnostics'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => handoff.probe(),
            tooltip: 'Refresh connection status',
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: () => handoff.probe(),
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            // Connection status overview.
            _ConnectionStatusCard(handoff: handoff, c: c),

            const SizedBox(height: 24),

            // Paired machines details.
            if (auth.machines.isEmpty)
              const Center(
                child: Text(
                  'No paired machines. Pair a machine to see connection details.',
                  textAlign: TextAlign.center,
                  style: TextStyle(color: Colors.grey),
                ),
              )
            else
              ...auth.machines.map((cred) => MachineDiagnosticsCard(
                    credential: cred,
                    handoff: handoff,
                    discovery: discovery,
                    c: c,
                  )),
          ],
        ),
      ),
    );
  }
}

/// Card showing overall connection status.
class _ConnectionStatusCard extends StatelessWidget {
  const _ConnectionStatusCard({
    required this.handoff,
    required this.c,
  });

  final HandoffService handoff;
  final ColorScheme c;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Connection Status',
              style: Theme.of(context)
                  .textTheme
                  .titleLarge
                  ?.copyWith(color: c.onBackground),
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                const Icon(Icons.network_check, color: Colors.green),
                const SizedBox(width: 8),
                Text(
                  handoff.probing ? 'Probing...' : 'Idle',
                  style: TextStyle(
                    fontWeight: FontWeight.bold,
                    color: handoff.probing
                        ? c.accentBlue
                        : c.textSecondary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Text(
              'Last probe: ${handoff.hasHandoff ? 'Success' : 'None'}',
              style: TextStyle(color: c.textSecondary, fontSize: 12),
            ),
          ],
        ),
      ),
    );
  }
}

/// Card showing diagnostics for a single machine.
class MachineDiagnosticsCard extends StatelessWidget {
  const MachineDiagnosticsCard({
    required this.credential,
    required this.handoff,
    required this.discovery,
    required this.c,
  });

  final MachineCredential credential;
  final HandoffService handoff;
  final DiscoveryService discovery;
  final ColorScheme c;

  @override
  Widget build(BuildContext context) {
    final beacon = handoff._beacons[credential.machineId];
    final resolvedUrl = handoff.resolvedUrl(credential.machineId);
    final candidates = handoff.candidates
        .where((cand) => cand.machineId == credential.machineId)
        .toList();

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Machine header.
            Row(
              children: [
                Icon(
                  Icons.computer_rounded,
                  color: c.accentBlue,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    credential.machineName.isNotEmpty
                        ? credential.machineName
                        : credential.machineId,
                    style: Theme.of(context)
                        .textTheme
                        .titleMedium
                        ?.copyWith(color: c.onBackground),
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.launch),
                  onPressed: () {
                    // TODO: Navigate to machine detail screen?
                  },
                  tooltip: 'View machine details',
                ),
              ],
            ),
            const SizedBox(height: 12),

            // Stored credential.
            _InfoRow(
              label: 'Stored URL',
              value: credential.baseUrl,
              c: c,
            ),

            // Beacon information.
            if (beacon != null) ...[
              const SizedBox(height: 8),
              Text(
                'Last Beacon',
                style: Theme.of(context)
                    .textTheme
                    .titleSmall
                    ?.copyWith(fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 4),
              _InfoRow(
                label: 'Hostname',
                value: beacon.hostname,
                c: c,
              ),
              _InfoRow(
                label: 'LAN IPs',
                value: beacon.lanIsEmpty ? 'None' : beacon.lanIps.join(', '),
                c: c,
              ),
              _InfoRow(
                label: 'Tailscale IP',
                value: beacon.tailscaleIp ?? 'Not available',
                c: c,
              ),
              _InfoRow(
                label: 'Public URL',
                value: beacon.publicUrl ?? 'Not available',
                c: c,
              ),
              _InfoRow(
                label: 'Uptime',
                value: '${beacon.uptimeSecs ~/ 3600}h ${(beacon.uptimeSecs % 3600) ~/ 60}m',
                c: c,
              ),
              if (beacon.activeSession != null) ...[
                const SizedBox(height: 4),
                Text(
                  'Active Session',
                  style: Theme.of(context)
                      .textTheme
                      .titleSmall
                      ?.copyWith(fontWeight: FontWeight.bold),
                ),
                const SizedBox(height: 4),
                _InfoRow(
                  label: 'Task',
                  value: beacon.activeSession!.task,
                  c: c,
                ),
                _InfoRow(
                  label: 'Provider',
                  value: beacon.activeSession!.provider,
                  c: c,
                ),
                _InfoRow(
                  label: 'Status',
                  value: beacon.activeSession!.status,
                  c: c,
                ),
              ],
            ] else
              const Padding(
                padding: EdgeInsets.only(top: 8),
                child: Text(
                  'No beacon data available',
                  style: TextStyle(color: Colors.grey, fontSize: 12),
                ),
              ),

            // Discovered URLs from mDNS.
            if (discovery.daemons.isNotEmpty) ...[
              const SizedBox(height: 8),
              Text(
                'mDNS Discovered',
                style: Theme.of(context)
                    .textTheme
                    .titleSmall
                    ?.copyWith(fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 4),
              ...discovery.daemons
                  .where((daemon) => daemon.machineId == credential.machineId ||
                      daemon.host.contains(credential.machineId))
                  .take(3)
                  .map((daemon) => _InfoRow(
                        label: 'mDNS Host',
                        value: '${daemon.host}:${daemon.port}',
                        c: c,
                      )),
            ],

            // Connection candidates and resolved URL.
            const SizedBox(height: 8),
            Text(
              'Connection Race',
              style: Theme.of(context)
                  .textTheme
                  .titleSmall
                  ?.copyWith(fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 4),
            _InfoRow(
              label: 'Resolved URL',
              value: resolvedUrl ?? 'None',
              c: c,
            ),
            _InfoRow(
              label: 'Candidates Count',
              value: '${candidates.length}',
              c: c,
            ),
            if (candidates.isNotEmpty) ...[
              const SizedBox(height: 4),
              Wrap(
                spacing: 8,
                children: candidates
                    .map((cand) => Chip(
                          label: Text(cand.resolvedUrl),
                          avatar: Icon(
                            Icons.check_circle,
                            color: cand.resolvedUrl == resolvedUrl
                                ? Colors.green
                                : Colors.grey,
                            size: 16,
                          ),
                        ))
                    .toList(),
              ),
            ],

            // Last probe time.
            const SizedBox(height: 8),
            _InfoRow(
              label: 'Last Probed',
              value: handoff.probing
                  ? 'Just now...'
                  : '${_formatDuration(handoff._lastProbeTime ?? 0)} ago',
              c: c,
            ),
          ],
        ),
      ),
    );
  }

  String _formatDuration(int secondsAgo) {
    if (secondsAgo == 0) return 'just now';
    if (secondsAgo < 60) return '$secondsAgo sec';
    if (secondsAgo < 3600) return '${secondsAgo ~/ 60} min';
    return '${secondsAgo ~/ 3600} h';
  }
}

/// Helper widget for info rows.
class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.label,
    required this.value,
    required this.c,
  });

  final String label;
  final String value;
  final ColorScheme c;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            label,
            style: TextStyle(color: c.textSecondary, fontSize: 12),
          ),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(
                fontSize: 12,
                fontFamily: 'JetBrainsMono',
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

// Extension to check if lanIps is empty.
extension on List<String> {
  bool get lanIsEmpty => this.isEmpty || (this.length == 1 && this.first.isEmpty);
}