import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/notification_service.dart';
import '../theme/app_theme.dart';

/// Settings screen for managing connections and app preferences.
class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthService>();
    final notif = context.watch<NotificationService>();
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Device info.
          _SectionHeader(title: 'Device'),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _InfoRow(label: 'Device ID', value: auth.deviceId),
                  const SizedBox(height: 8),
                  _InfoRow(label: 'Push Token', value: notif.pushToken != null ? '${notif.pushToken!.substring(0, 20)}...' : 'Not set'),
                  const SizedBox(height: 8),
                  _InfoRow(label: 'Notifications', value: notif.permissionGranted ? 'Enabled' : 'Disabled'),
                ],
              ),
            ),
          ),
          const SizedBox(height: 24),

          // Paired machines.
          _SectionHeader(title: 'Paired Machines (${auth.machines.length})'),
          ...auth.machines.map((cred) => Card(
            margin: const EdgeInsets.only(bottom: 8),
            child: ListTile(
              leading: Icon(Icons.computer_rounded, color: c.accentBlue),
              title: Text(cred.machineName.isNotEmpty ? cred.machineName : cred.baseUrl),
              subtitle: Text(cred.baseUrl, style: const TextStyle(fontSize: 12)),
              trailing: IconButton(
                icon: Icon(Icons.delete_outline_rounded, color: c.accentRed),
                onPressed: () => _confirmRemove(context, auth, cred),
              ),
            ),
          )),
          const SizedBox(height: 24),

          // App info.
          _SectionHeader(title: 'About'),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const _InfoRow(label: 'App Version', value: '1.0.0'),
                  const SizedBox(height: 8),
                  const _InfoRow(label: 'Protocol', value: 'VibeCody Mobile Gateway v1'),
                  const SizedBox(height: 8),
                  const _InfoRow(label: 'License', value: 'MIT'),
                ],
              ),
            ),
          ),
          const SizedBox(height: 24),

          // Clear all data.
          Center(
            child: TextButton.icon(
              onPressed: () => _confirmClearAll(context, auth),
              icon: Icon(Icons.delete_forever_rounded, color: c.accentRed),
              label: Text('Clear All Data', style: TextStyle(color: c.accentRed)),
            ),
          ),
        ],
      ),
    );
  }

  void _confirmRemove(BuildContext context, AuthService auth, MachineCredential cred) {
    final c = context.vibeColors;
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: c.bgSecondary,
        title: const Text('Remove Machine?'),
        content: Text('Unpair from ${cred.machineName.isNotEmpty ? cred.machineName : cred.baseUrl}?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          ElevatedButton(
            onPressed: () {
              auth.removeMachine(cred.machineId);
              Navigator.pop(ctx);
            },
            style: ElevatedButton.styleFrom(backgroundColor: c.accentRed),
            child: const Text('Remove'),
          ),
        ],
      ),
    );
  }

  void _confirmClearAll(BuildContext context, AuthService auth) {
    final c = context.vibeColors;
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: c.bgSecondary,
        title: const Text('Clear All Data?'),
        content: const Text('This will remove all paired machines and stored credentials.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          ElevatedButton(
            onPressed: () {
              for (final m in auth.machines.toList()) {
                auth.removeMachine(m.machineId);
              }
              Navigator.pop(ctx);
            },
            style: ElevatedButton.styleFrom(backgroundColor: c.accentRed),
            child: const Text('Clear'),
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8, left: 4),
      child: Text(title, style: Theme.of(context).textTheme.headlineMedium?.copyWith(fontSize: 16)),
    );
  }
}

class _InfoRow extends StatelessWidget {
  final String label;
  final String value;
  const _InfoRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: TextStyle(color: c.textSecondary, fontSize: 13)),
        Flexible(
          child: Text(
            value,
            style: const TextStyle(fontSize: 13, fontFamily: 'JetBrainsMono'),
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }
}
