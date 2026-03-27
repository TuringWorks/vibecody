import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:provider/provider.dart';
import '../services/auth_service.dart';
import '../services/api_client.dart';
import '../theme/app_theme.dart';
import 'home_screen.dart';

/// QR code scanner screen for pairing with a VibeCody daemon.
class PairScreen extends StatefulWidget {
  const PairScreen({super.key});

  @override
  State<PairScreen> createState() => _PairScreenState();
}

class _PairScreenState extends State<PairScreen> {
  final MobileScannerController _scannerController = MobileScannerController();
  bool _processing = false;
  String? _error;

  @override
  void dispose() {
    _scannerController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = context.vibeColors;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Scan QR Code'),
        backgroundColor: Colors.transparent,
        elevation: 0,
      ),
      extendBodyBehindAppBar: true,
      body: Stack(
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
      ),
    );
  }

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
