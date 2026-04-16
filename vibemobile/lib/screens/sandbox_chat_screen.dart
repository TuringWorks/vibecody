// sandbox_chat_screen.dart — Sandbox AI chat tab for the phone app.
//
// Polls /watch/sandbox/chat-session to discover the active sandbox session,
// then syncs messages bidirectionally with VibeUI and all watches.

import 'dart:async';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/watch_sync_service.dart';
import '../theme/app_theme.dart';
import 'watch_chat_screen.dart';

class SandboxChatScreen extends StatefulWidget {
  const SandboxChatScreen({super.key});

  @override
  State<SandboxChatScreen> createState() => _SandboxChatScreenState();
}

class _SandboxChatScreenState extends State<SandboxChatScreen> {
  final _pollSync = WatchSyncService();
  Timer? _pollTimer;
  String? _sandboxSessionId;
  String? _selectedMachineId;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _setup());
  }

  void _setup() {
    final auth = context.read<AuthService>();
    if (auth.machines.isEmpty) return;
    final cred = auth.machines.first;
    _selectedMachineId = cred.machineId;
    _pollSync.configure(cred.baseUrl, cred.token);
    _startPolling();
  }

  void _startPolling() {
    _pollTimer?.cancel();
    _pollTimer = Timer.periodic(const Duration(seconds: 10), (_) => _refreshSandboxSession());
    _refreshSandboxSession();
  }

  Future<void> _refreshSandboxSession() async {
    final auth = context.read<AuthService>();
    final cred = _selectedMachineId != null
        ? auth.getCredential(_selectedMachineId!)
        : auth.machines.firstOrNull;
    if (cred == null) return;
    final sid = await _pollSync.getSandboxChatSession(cred.baseUrl, cred.token);
    if (sid != _sandboxSessionId && mounted) {
      setState(() => _sandboxSessionId = sid);
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final auth = context.watch<AuthService>();

    if (!auth.hasMachines) {
      return Scaffold(
        appBar: AppBar(title: const Text('Sandbox Chat')),
        body: Center(
          child: Text('No machines configured', style: TextStyle(color: c.textSecondary)),
        ),
      );
    }

    final sid = _sandboxSessionId;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Sandbox Chat'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh_rounded),
            tooltip: 'Refresh',
            onPressed: _refreshSandboxSession,
          ),
        ],
      ),
      body: sid == null
          ? Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.terminal_rounded, size: 56,
                      color: c.textSecondary.withValues(alpha: 0.4)),
                  const SizedBox(height: 16),
                  Text(
                    'No active sandbox chat',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Open VibeUI → Sandbox tab → pick a folder\nto start the sandbox AI chat.',
                    style: TextStyle(color: c.textSecondary, fontSize: 13),
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 24),
                  OutlinedButton.icon(
                    onPressed: _refreshSandboxSession,
                    icon: const Icon(Icons.refresh_rounded),
                    label: const Text('Check again'),
                  ),
                ],
              ),
            )
          : WatchChatScreen(
              key: ValueKey(sid),
              initialSessionId: sid,
              sessionTitle: 'Sandbox Chat',
            ),
    );
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    _pollSync.dispose();
    super.dispose();
  }
}

extension on List<dynamic> {
  dynamic get firstOrNull => isEmpty ? null : first;
}
