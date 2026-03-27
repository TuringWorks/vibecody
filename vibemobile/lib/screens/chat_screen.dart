import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../theme/app_theme.dart';

/// Chat/dispatch screen — send messages or tasks to a selected machine.
class ChatScreen extends StatefulWidget {
  const ChatScreen({super.key});

  @override
  State<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends State<ChatScreen> {
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  final List<_ChatMessage> _messages = [];
  String? _selectedMachineId;
  bool _sending = false;

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthService>();
    final machines = auth.machines;
    final c = context.vibeColors;

    // Auto-select first machine if none selected.
    if (_selectedMachineId == null && machines.isNotEmpty) {
      _selectedMachineId = machines.first.machineId;
    }

    return Scaffold(
      appBar: AppBar(
        title: const Text('Chat'),
        actions: [
          if (machines.length > 1)
            PopupMenuButton<String>(
              icon: const Icon(Icons.computer_rounded),
              onSelected: (id) => setState(() => _selectedMachineId = id),
              itemBuilder: (_) => machines.map((m) => PopupMenuItem(
                value: m.machineId,
                child: Row(
                  children: [
                    Icon(
                      m.machineId == _selectedMachineId ? Icons.check_circle : Icons.circle_outlined,
                      size: 18,
                      color: m.machineId == _selectedMachineId ? c.accentBlue : c.textSecondary,
                    ),
                    const SizedBox(width: 8),
                    Text(m.machineName.isNotEmpty ? m.machineName : m.baseUrl),
                  ],
                ),
              )).toList(),
            ),
        ],
      ),
      body: Column(
        children: [
          // Machine indicator bar.
          if (_selectedMachineId != null)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              color: c.bgSecondary,
              child: Row(
                children: [
                  Icon(Icons.circle, size: 8, color: c.accentGreen),
                  const SizedBox(width: 8),
                  Text(
                    _currentCredential(auth)?.machineName ?? 'Unknown',
                    style: TextStyle(fontSize: 12, color: c.textSecondary),
                  ),
                  const Spacer(),
                  _DispatchChip(label: 'Chat', selected: true, onTap: () {}),
                ],
              ),
            ),

          // Messages list.
          Expanded(
            child: _messages.isEmpty
                ? Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(Icons.chat_bubble_outline_rounded, size: 48, color: c.textSecondary.withValues(alpha: 0.5)),
                        const SizedBox(height: 16),
                        Text('Send a message or task to your machine', style: Theme.of(context).textTheme.bodyMedium),
                      ],
                    ),
                  )
                : ListView.builder(
                    controller: _scrollController,
                    padding: const EdgeInsets.all(16),
                    itemCount: _messages.length,
                    itemBuilder: (_, i) => _MessageBubble(message: _messages[i]),
                  ),
          ),

          // Input bar.
          Container(
            padding: const EdgeInsets.fromLTRB(16, 8, 8, 8),
            decoration: BoxDecoration(
              color: c.bgSecondary,
              border: Border(top: BorderSide(color: c.borderColor)),
            ),
            child: SafeArea(
              top: false,
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _controller,
                      maxLines: 4,
                      minLines: 1,
                      decoration: InputDecoration(
                        hintText: 'Message or /command...',
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(20),
                          borderSide: BorderSide.none,
                        ),
                        filled: true,
                        fillColor: c.bgTertiary,
                        contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
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
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  MachineCredential? _currentCredential(AuthService auth) {
    if (_selectedMachineId == null) return null;
    return auth.getCredential(_selectedMachineId!);
  }

  Future<void> _send() async {
    final text = _controller.text.trim();
    if (text.isEmpty) return;

    final auth = context.read<AuthService>();
    final api = context.read<ApiClient>();
    final cred = _currentCredential(auth);
    if (cred == null) return;

    _controller.clear();
    setState(() {
      _messages.add(_ChatMessage(role: 'user', content: text));
      _sending = true;
    });
    _scrollToBottom();

    try {
      if (text.startsWith('/')) {
        final taskId = await api.dispatch(
          cred.baseUrl, cred.token,
          deviceId: auth.deviceId,
          machineId: cred.machineId,
          dispatchType: 'repl_command',
          payload: text,
        );
        setState(() {
          _messages.add(_ChatMessage(role: 'system', content: 'Dispatched as REPL command (task: $taskId)'));
        });
      } else {
        final buffer = StringBuffer();

        setState(() {
          _messages.add(_ChatMessage(role: 'assistant', content: ''));
        });

        await for (final chunk in api.chatStream(cred.baseUrl, cred.token, text)) {
          buffer.write(chunk);
          setState(() {
            _messages.last = _ChatMessage(role: 'assistant', content: buffer.toString());
          });
          _scrollToBottom();
        }
      }
    } catch (e) {
      setState(() {
        _messages.add(_ChatMessage(role: 'error', content: e.toString()));
      });
    } finally {
      setState(() => _sending = false);
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
}

class _ChatMessage {
  final String role;
  final String content;
  _ChatMessage({required this.role, required this.content});
}

class _MessageBubble extends StatelessWidget {
  final _ChatMessage message;
  const _MessageBubble({required this.message});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    final isUser = message.role == 'user';
    final isError = message.role == 'error';
    final isSystem = message.role == 'system';

    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.only(bottom: 8),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        constraints: BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.8),
        decoration: BoxDecoration(
          color: isUser
              ? c.accentBlue.withValues(alpha: 0.2)
              : isError
                  ? c.accentRed.withValues(alpha: 0.15)
                  : isSystem
                      ? c.bgTertiary
                      : c.bgSecondary,
          borderRadius: BorderRadius.circular(16),
          border: Border.all(
            color: isUser ? c.accentBlue.withValues(alpha: 0.3) : c.borderColor,
          ),
        ),
        child: SelectableText(
          message.content,
          style: TextStyle(
            fontSize: 14,
            color: isError ? c.accentRed : c.textPrimary,
            fontFamily: isSystem ? 'JetBrainsMono' : null,
          ),
        ),
      ),
    );
  }
}

class _DispatchChip extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onTap;

  const _DispatchChip({required this.label, required this.selected, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
        decoration: BoxDecoration(
          color: selected ? c.accentBlue.withValues(alpha: 0.2) : c.bgTertiary,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: selected ? c.accentBlue : c.borderColor),
        ),
        child: Text(label, style: TextStyle(fontSize: 11, color: selected ? c.accentBlue : c.textSecondary)),
      ),
    );
  }
}
