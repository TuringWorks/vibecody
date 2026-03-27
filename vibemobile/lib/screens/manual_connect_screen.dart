import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../theme/app_theme.dart';
import 'home_screen.dart';

/// Manual connection screen — enter daemon URL and API token.
class ManualConnectScreen extends StatefulWidget {
  const ManualConnectScreen({super.key});

  @override
  State<ManualConnectScreen> createState() => _ManualConnectScreenState();
}

class _ManualConnectScreenState extends State<ManualConnectScreen> {
  final _urlController = TextEditingController(text: 'http://');
  final _tokenController = TextEditingController();
  final _nameController = TextEditingController();
  bool _connecting = false;
  String? _error;
  bool _reachable = false;

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(title: const Text('Connect Manually')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Enter the URL and API token shown when you run:',
              style: TextStyle(color: c.textSecondary),
            ),
            const SizedBox(height: 8),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: c.bgTertiary,
                borderRadius: BorderRadius.circular(8),
              ),
              child: SelectableText(
                'vibecli --serve --host 0.0.0.0 --port 7878',
                style: TextStyle(fontFamily: 'JetBrainsMono', fontSize: 14, color: c.accentGreen),
              ),
            ),
            const SizedBox(height: 24),

            // Name.
            const Text('Name (optional)', style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
            const SizedBox(height: 8),
            TextField(
              controller: _nameController,
              decoration: const InputDecoration(hintText: 'My Mac Pro'),
            ),
            const SizedBox(height: 20),

            // URL.
            const Text('Daemon URL', style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _urlController,
                    keyboardType: TextInputType.url,
                    decoration: const InputDecoration(hintText: 'http://10.0.2.2:7878 or local IP'),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton(
                  onPressed: _checkHealth,
                  icon: Icon(
                    _reachable ? Icons.check_circle_rounded : Icons.wifi_find_rounded,
                    color: _reachable ? c.accentGreen : c.textSecondary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 20),

            // API Token.
            const Text('API Token', style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
            const SizedBox(height: 8),
            TextField(
              controller: _tokenController,
              obscureText: true,
              decoration: const InputDecoration(hintText: 'Bearer token from terminal output'),
            ),
            const SizedBox(height: 8),

            if (_error != null)
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: c.accentRed.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: c.accentRed.withValues(alpha: 0.3)),
                ),
                child: Row(
                  children: [
                    Icon(Icons.error_outline_rounded, color: c.accentRed, size: 18),
                    const SizedBox(width: 8),
                    Expanded(child: Text(_error!, style: TextStyle(color: c.accentRed, fontSize: 13))),
                  ],
                ),
              ),

            const SizedBox(height: 32),

            // Connect button.
            SizedBox(
              width: double.infinity,
              child: ElevatedButton(
                onPressed: _connecting ? null : _connect,
                child: _connecting
                    ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                    : const Text('Connect'),
              ),
            ),

            const SizedBox(height: 24),

            // Help section.
            ExpansionTile(
              title: const Text('Connection Help', style: TextStyle(fontSize: 13)),
              tilePadding: EdgeInsets.zero,
              children: [
                _HelpItem(icon: Icons.phone_android_rounded, color: c.accentBlue, title: 'Android Emulator', desc: 'Use http://10.0.2.2:7878 (maps to host\'s localhost)'),
                _HelpItem(icon: Icons.phone_iphone_rounded, color: c.accentBlue, title: 'iOS Simulator', desc: 'Use http://localhost:7878 (shares host network)'),
                _HelpItem(icon: Icons.wifi_rounded, color: c.accentBlue, title: 'Physical Device', desc: 'Use your machine\'s local IP (e.g., 192.168.1.x:7878)'),
                _HelpItem(icon: Icons.vpn_key_rounded, color: c.accentBlue, title: 'Tailscale', desc: 'Use your Tailscale IP (100.x.y.z:7878) for secure remote access'),
                _HelpItem(icon: Icons.public_rounded, color: c.accentBlue, title: 'Tailscale Funnel', desc: 'Use the public HTTPS URL for access from anywhere'),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _checkHealth() async {
    final api = context.read<ApiClient>();
    final url = _urlController.text.trim();
    if (url.isEmpty) return;

    final healthy = await api.healthCheck(url);
    setState(() {
      _reachable = healthy;
      _error = healthy ? null : 'Cannot reach daemon at $url';
    });
  }

  Future<void> _connect() async {
    final url = _urlController.text.trim();
    final token = _tokenController.text.trim();
    final name = _nameController.text.trim();

    if (url.isEmpty || token.isEmpty) {
      setState(() => _error = 'URL and token are required');
      return;
    }

    setState(() { _connecting = true; _error = null; });

    try {
      final api = context.read<ApiClient>();
      final auth = context.read<AuthService>();
      final c = context.vibeColors;

      final healthy = await api.healthCheck(url);
      if (!healthy) {
        throw Exception('Cannot reach daemon at $url');
      }

      await auth.addManual(url, token, name.isNotEmpty ? name : url);

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: const Text('Connected!'), backgroundColor: c.accentGreen),
        );
        Navigator.pushAndRemoveUntil(
          context,
          MaterialPageRoute(builder: (_) => const HomeScreen()),
          (route) => false,
        );
      }
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      setState(() => _connecting = false);
    }
  }
}

class _HelpItem extends StatelessWidget {
  final IconData icon;
  final Color color;
  final String title;
  final String desc;
  const _HelpItem({required this.icon, required this.color, required this.title, required this.desc});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 18, color: color),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13)),
                Text(desc, style: TextStyle(fontSize: 12, color: context.vibeColors.textSecondary)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
