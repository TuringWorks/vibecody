import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../theme/app_theme.dart';
import 'home_screen.dart';

/// QR code scanner + manual URL entry screen for pairing with a VibeCody daemon.
class PairScreen extends StatefulWidget {
  const PairScreen({super.key});

  @override
  State<PairScreen> createState() => _PairScreenState();
}

class _PairScreenState extends State<PairScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  final MobileScannerController _scannerController = MobileScannerController();
  bool _processing = false;
  String? _error;

  // Manual entry controllers
  final _urlController = TextEditingController();
  final _tokenController = TextEditingController();
  final _nameController = TextEditingController(text: 'My Machine');
  bool _manualLoading = false;
  String? _manualError;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    // Stop/start scanner when switching tabs to save resources
    _tabController.addListener(() {
      if (_tabController.index == 0) {
        _scannerController.start();
      } else {
        _scannerController.stop();
      }
    });
  }

  @override
  void dispose() {
    _tabController.dispose();
    _scannerController.dispose();
    _urlController.dispose();
    _tokenController.dispose();
    _nameController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;

    return DefaultTabController(
      length: 2,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Pair with Daemon'),
          backgroundColor: Colors.transparent,
          elevation: 0,
          bottom: TabBar(
            controller: _tabController,
            indicatorColor: c.accentBlue,
            labelColor: c.accentBlue,
            unselectedLabelColor: Colors.white60,
            tabs: const [
              Tab(text: 'Scan QR'),
              Tab(text: 'Manual URL'),
            ],
          ),
        ),
        extendBodyBehindAppBar: true,
        body: TabBarView(
          controller: _tabController,
          children: [
            _buildQrTab(c),
            _buildManualTab(c),
          ],
        ),
      ),
    );
  }

  // ── QR tab (unchanged logic) ────────────────────────────────────────────────

  Widget _buildQrTab(dynamic c) {
    return Stack(
      children: [
        // Scanner.
        MobileScanner(
          controller: _scannerController,
          onDetect: _onDetect,
        ),

        // Overlay.
        Container(
          decoration: BoxDecoration(
            color: Colors.black.withValues(alpha: 0.5),
          ),
        ),

        // Viewfinder frame.
        Center(
          child: Container(
            width: 280, height: 280,
            decoration: BoxDecoration(
              border: Border.all(color: c.accentBlue, width: 3),
              borderRadius: BorderRadius.circular(20),
            ),
          ),
        ),

        // Instructions.
        Positioned(
          bottom: 100,
          left: 0, right: 0,
          child: Column(
            children: [
              if (_processing)
                CircularProgressIndicator(color: c.accentBlue)
              else if (_error != null) ...[
                Icon(Icons.error_rounded, color: c.accentRed, size: 40),
                const SizedBox(height: 8),
                Text(_error!, style: TextStyle(color: c.accentRed), textAlign: TextAlign.center),
                const SizedBox(height: 16),
                ElevatedButton(
                  onPressed: () => setState(() { _error = null; _processing = false; }),
                  child: const Text('Try Again'),
                ),
              ] else
                const Text(
                  'Point your camera at the QR code\nshown in your terminal',
                  textAlign: TextAlign.center,
                  style: TextStyle(color: Colors.white70, fontSize: 15),
                ),
            ],
          ),
        ),
      ],
    );
  }

  // ── Manual URL tab ──────────────────────────────────────────────────────────

  Widget _buildManualTab(dynamic c) {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(24, 120, 24, 40),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Enter daemon connection details manually.\nUseful when QR scanning is unavailable (e.g. emulator).',
            style: TextStyle(color: Colors.white60, fontSize: 13),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),

          // Daemon URL
          _buildField(
            controller: _urlController,
            label: 'Daemon URL',
            hint: 'http://192.168.1.x:7878',
            keyboardType: TextInputType.url,
            c: c,
          ),
          const SizedBox(height: 16),

          // API Token
          _buildField(
            controller: _tokenController,
            label: 'API Token',
            hint: 'Bearer token from daemon',
            obscureText: true,
            c: c,
          ),
          const SizedBox(height: 16),

          // Machine Name
          _buildField(
            controller: _nameController,
            label: 'Machine Name (optional)',
            hint: 'My Machine',
            c: c,
          ),
          const SizedBox(height: 24),

          // Error display
          if (_manualError != null) ...[
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: c.accentRed.withValues(alpha: 0.15),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: c.accentRed.withValues(alpha: 0.5)),
              ),
              child: Row(
                children: [
                  Icon(Icons.error_outline, color: c.accentRed, size: 18),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _manualError!,
                      style: TextStyle(color: c.accentRed, fontSize: 13),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
          ],

          // Connect button
          SizedBox(
            height: 48,
            child: ElevatedButton(
              onPressed: _manualLoading ? null : _connectManual,
              style: ElevatedButton.styleFrom(
                backgroundColor: c.accentBlue,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(10),
                ),
              ),
              child: _manualLoading
                  ? const SizedBox(
                      width: 22, height: 22,
                      child: CircularProgressIndicator(
                        color: Colors.white, strokeWidth: 2.5,
                      ),
                    )
                  : const Text(
                      'Connect',
                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
                    ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildField({
    required TextEditingController controller,
    required String label,
    required String hint,
    required dynamic c,
    TextInputType keyboardType = TextInputType.text,
    bool obscureText = false,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: const TextStyle(color: Colors.white70, fontSize: 13)),
        const SizedBox(height: 6),
        TextField(
          controller: controller,
          keyboardType: keyboardType,
          obscureText: obscureText,
          style: const TextStyle(color: Colors.white, fontSize: 14),
          decoration: InputDecoration(
            hintText: hint,
            hintStyle: const TextStyle(color: Colors.white30),
            filled: true,
            fillColor: Colors.white.withValues(alpha: 0.07),
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(8),
              borderSide: BorderSide(color: Colors.white24),
            ),
            enabledBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(8),
              borderSide: BorderSide(color: Colors.white24),
            ),
            focusedBorder: OutlineInputBorder(
              borderRadius: BorderRadius.circular(8),
              borderSide: BorderSide(color: c.accentBlue),
            ),
            contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
          ),
        ),
      ],
    );
  }

  Future<void> _connectManual() async {
    final url = _urlController.text.trim();
    final token = _tokenController.text.trim();
    final name = _nameController.text.trim().isEmpty
        ? 'My Machine'
        : _nameController.text.trim();

    if (url.isEmpty) {
      setState(() => _manualError = 'Daemon URL is required');
      return;
    }
    if (token.isEmpty) {
      setState(() => _manualError = 'API Token is required');
      return;
    }

    setState(() { _manualLoading = true; _manualError = null; });

    try {
      final auth = context.read<AuthService>();
      final cred = await auth.addManual(url, token, name);

      if (mounted) {
        final c = context.vibeColors;
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Connected to ${cred.machineName}'),
            backgroundColor: c.accentGreen,
          ),
        );
        Navigator.pushAndRemoveUntil(
          context,
          MaterialPageRoute(builder: (_) => const HomeScreen()),
          (route) => false,
        );
      }
    } catch (e) {
      setState(() {
        _manualError = e.toString();
        _manualLoading = false;
      });
    }
  }

  // ── QR detect handler (unchanged) ──────────────────────────────────────────

  Future<void> _onDetect(BarcodeCapture capture) async {
    if (_processing) return;
    final barcode = capture.barcodes.firstOrNull;
    if (barcode == null || barcode.rawValue == null) return;

    final data = barcode.rawValue!;
    if (!data.startsWith('vibecody://pair')) return;

    setState(() => _processing = true);
    _scannerController.stop();

    try {
      final auth = context.read<AuthService>();
      final api = context.read<ApiClient>();
      final c = context.vibeColors;

      final uri = Uri.parse(data);
      final host = uri.queryParameters['host'] ?? 'localhost';
      final port = uri.queryParameters['port'] ?? '7878';
      final baseUrl = 'http://$host:$port';

      final healthy = await api.healthCheck(baseUrl);
      if (!healthy) {
        throw Exception('Cannot reach VibeCody daemon at $baseUrl');
      }

      final pin = uri.queryParameters['pin'] ?? '';
      final cred = await auth.addFromQrData(data, pin);

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Paired with ${cred.machineName}'),
            backgroundColor: c.accentGreen,
          ),
        );
        Navigator.pushAndRemoveUntil(
          context,
          MaterialPageRoute(builder: (_) => const HomeScreen()),
          (route) => false,
        );
      }
    } catch (e) {
      setState(() {
        _error = e.toString();
        _processing = false;
      });
      _scannerController.start();
    }
  }
}
