// watch_chat_screen.dart — Google Docs-style chat screen for phone app.
//
// Uses /watch/* endpoints (Bearer auth) for bidirectional real-time sync
// with the daemon, Apple Watch, and Android Watch.
// Messages never truncate — full text, scrollable.

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/watch_sync_service.dart';
import '../theme/app_theme.dart';

class WatchChatScreen extends StatefulWidget {
  final String? initialSessionId;
  final String? sessionTitle;

  const WatchChatScreen({super.key, this.initialSessionId, this.sessionTitle});

  @override
  State<WatchChatScreen> createState() => _WatchChatScreenState();
}

class _WatchChatScreenState extends State<WatchChatScreen> {
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  final _sync = WatchSyncService();

  String? _sessionId;
  bool _sending = false;
  String? _streamingText;
  String? _selectedMachineId;

  @override
  void initState() {
    super.initState();
    _sessionId = widget.initialSessionId;
    WidgetsBinding.instance.addPostFrameCallback((_) => _setup());
  }

  void _setup() {
    final auth = context.read<AuthService>();
    if (auth.machines.isEmpty) return;
    final cred = _selectedMachineId != null
        ? auth.getCredential(_selectedMachineId!)
        : auth.machines.first;
    if (cred == null) return;
    _selectedMachineId = cred.machineId;
    _sync.configure(cred.baseUrl, cred.token);
    _sync.addListener(_onSyncUpdate);
    if (_sessionId != null) {
      _sync.startSync(_sessionId!);
    }
  }

  void _onSyncUpdate() {
    if (!mounted) return;
    setState(() {});
    _scrollToBottom();
  }

  void _selectMachine(String machineId) {
    final auth = context.read<AuthService>();
    final cred = auth.getCredential(machineId);
    if (cred == null) return;
    setState(() => _selectedMachineId = machineId);
    _sync.stopSync();
    _sync.configure(cred.baseUrl, cred.token);
    if (_sessionId != null) _sync.startSync(_sessionId!);
  }

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthService>();
    final c = context.vibeColors;
    final allMessages = _sync.messages;

    if (_selectedMachineId == null && auth.machines.isNotEmpty) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _setup());
    }

    return Scaffold(
      appBar: AppBar(
        title: Text(widget.sessionTitle ?? 'Chat'),
        actions: [
          if (auth.machines.length > 1)
            PopupMenuButton<String>(
              icon: const Icon(Icons.computer_rounded),
              onSelected: _selectMachine,
              itemBuilder: (_) => auth.machines.map((m) => PopupMenuItem(
                value: m.machineId,
                child: Row(children: [
                  Icon(
                    m.machineId == _selectedMachineId ? Icons.check_circle : Icons.circle_outlined,
                    size: 18,
                    color: m.machineId == _selectedMachineId ? c.accentBlue : c.textSecondary,
                  ),
                  const SizedBox(width: 8),
                  Text(m.machineName.isNotEmpty ? m.machineName : m.baseUrl),
                ]),
              )).toList(),
            ),
        ],
      ),
      body: Column(
        children: [
          // Status bar
          if (_selectedMachineId != null)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
              color: c.bgSecondary,
              child: Row(children: [
                Icon(Icons.sync_rounded, size: 12, color: c.accentGreen),
                const SizedBox(width: 6),
                Text('Live sync', style: TextStyle(fontSize: 11, color: c.textSecondary)),
                if (_sessionId != null) ...[
                  const SizedBox(width: 8),
                  Text('·', style: TextStyle(color: c.textSecondary)),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _sessionId!,
                      style: TextStyle(fontSize: 10, color: c.textSecondary, fontFamily: 'JetBrainsMono'),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ]),
            ),

          // Messages
          Expanded(
            child: allMessages.isEmpty && _streamingText == null
                ? Center(child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.chat_bubble_outline_rounded, size: 48,
                          color: c.textSecondary.withValues(alpha: 0.4)),
                      const SizedBox(height: 12),
                      Text('Start a conversation', style: TextStyle(color: c.textSecondary)),
                    ],
                  ))
                : ListView.builder(
                    controller: _scrollController,
                    padding: const EdgeInsets.all(12),
                    itemCount: allMessages.length + (_streamingText != null ? 1 : 0),
                    itemBuilder: (ctx, i) {
                      if (i == allMessages.length && _streamingText != null) {
                        return _SyncBubble(
                          role: 'assistant',
                          content: _streamingText!,
                          isStreaming: true,
                        );
                      }
                      final m = allMessages[i];
                      return _SyncBubble(role: m.role, content: m.content);
                    },
                  ),
          ),

          // Input bar
          Container(
            padding: const EdgeInsets.fromLTRB(12, 8, 8, 8),
            decoration: BoxDecoration(
              color: c.bgSecondary,
              border: Border(top: BorderSide(color: c.borderColor)),
            ),
            child: SafeArea(
              top: false,
              child: Row(children: [
                Expanded(
                  child: TextField(
                    controller: _controller,
                    maxLines: 5,
                    minLines: 1,
                    enabled: !_sending,
                    decoration: InputDecoration(
                      hintText: 'Message…',
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(20),
                        borderSide: BorderSide.none,
                      ),
                      filled: true,
                      fillColor: c.bgTertiary,
                      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                    ),
                    onSubmitted: (_) => _send(),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton(
                  onPressed: _sending ? null : _send,
                  icon: _sending
                      ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
                      : Icon(Icons.send_rounded, color: c.accentBlue),
                ),
              ]),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _send() async {
    final text = _controller.text.trim();
    if (text.isEmpty || _selectedMachineId == null) return;
    final auth = context.read<AuthService>();
    final cred = auth.getCredential(_selectedMachineId!)!;

    _controller.clear();
    setState(() => _sending = true);

    try {
      final result = await _sync.dispatch(
        cred.baseUrl, cred.token,
        content: text,
        sessionId: _sessionId,
      );
      if (result != null) {
        final newSid = result['session_id'] as String?;
        if (newSid != null && newSid != _sessionId) {
          setState(() => _sessionId = newSid);
          _sync.startSync(newSid);
        }
        setState(() => _streamingText = '');

        // Poll for response — reliable regardless of SSE
        final msgs = await _sync.pollForResponse(cred.baseUrl, cred.token, _sessionId!);
        setState(() {
          _streamingText = null;
          if (msgs.isNotEmpty) {
            _sync.messages.clear();
            _sync.messages.addAll(msgs);
          }
        });
      }
    } finally {
      setState(() {
        _sending = false;
        _streamingText = null;
      });
      _scrollToBottom();
    }
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      }
    });
  }

  @override
  void dispose() {
    _sync.removeListener(_onSyncUpdate);
    _sync.stopSync();
    _sync.dispose();
    _controller.dispose();
    _scrollController.dispose();
    super.dispose();
  }
}

class _SyncBubble extends StatelessWidget {
  final String role;
  final String content;
  final bool isStreaming;

  const _SyncBubble({required this.role, required this.content, this.isStreaming = false});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final isUser = role == 'user';

    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.only(bottom: 8),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        // maxWidth is 85% of screen — no text truncation, full content shown
        constraints: BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.85),
        decoration: BoxDecoration(
          color: isUser
              ? c.accentBlue.withValues(alpha: 0.2)
              : isStreaming
                  ? c.bgTertiary
                  : c.bgSecondary,
          borderRadius: BorderRadius.circular(16),
          border: Border.all(
            color: isUser
                ? c.accentBlue.withValues(alpha: 0.3)
                : isStreaming
                    ? c.accentBlue.withValues(alpha: 0.2)
                    : c.borderColor,
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (isStreaming) ...[
              Row(children: [
                SizedBox(
                  width: 12, height: 12,
                  child: CircularProgressIndicator(strokeWidth: 1.5, color: c.accentBlue),
                ),
                const SizedBox(width: 6),
                Text('Responding…', style: TextStyle(fontSize: 11, color: c.textSecondary)),
              ]),
              if (content.isNotEmpty) ...[
                const SizedBox(height: 6),
                // Full text — no softWrap restriction, no overflow clipping
                SelectableText(
                  content,
                  style: TextStyle(fontSize: 14, color: c.textPrimary),
                ),
              ],
            ] else
              SelectableText(
                content,
                style: TextStyle(
                  fontSize: 14,
                  color: c.textPrimary,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
