import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:multicast_dns/multicast_dns.dart';

/// A discovered VibeCLI daemon instance on the local network.
class DiscoveredDaemon {
  final String host;   // e.g. "my-mac.local" or raw IP
  final int port;      // e.g. 7878
  final String? machineId; // from TXT record, if available
  final String? version;   // from TXT record, if available

  DiscoveredDaemon({
    required this.host,
    required this.port,
    this.machineId,
    this.version,
  });

  /// Resolved HTTP base URL, e.g. "http://10.0.0.5:7878"
  String get baseUrl => 'http://$host:$port';

  @override
  String toString() => 'DiscoveredDaemon($host:$port id=$machineId)';
}

/// Discovers VibeCLI daemons on the LAN via mDNS/DNS-SD.
///
/// Queries for `_vibecli._tcp.local.` PTR records, then resolves the SRV
/// and TXT records to get host, port, and metadata.
///
/// Usage:
/// ```dart
/// final daemons = await DiscoveryService.discover();
/// ```
class DiscoveryService extends ChangeNotifier {
  List<DiscoveredDaemon> _daemons = [];
  bool _scanning = false;
  Timer? _timer;

  List<DiscoveredDaemon> get daemons => List.unmodifiable(_daemons);
  bool get scanning => _scanning;

  DiscoveryService() {
    // Auto-scan on startup and every 90 s thereafter.
    _scan();
    _timer = Timer.periodic(const Duration(seconds: 90), (_) => _scan());
  }

  Future<void> _scan() async {
    _scanning = true;
    notifyListeners();
    try {
      _daemons = await discover();
    } catch (_) {
      // Non-fatal: mDNS may not be available on this network.
    }
    _scanning = false;
    notifyListeners();
  }

  /// Trigger an immediate rescan.
  Future<void> rescan() => _scan();

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  // ── Static discovery helper ─────────────────────────────────────────────────

  /// Performs a one-shot mDNS query for `_vibecli._tcp.local.` and
  /// returns a list of discovered daemons.  Completes after [timeout].
  static Future<List<DiscoveredDaemon>> discover({
    Duration timeout = const Duration(seconds: 5),
  }) async {
    final results = <String, DiscoveredDaemon>{};

    final client = MDnsClient();
    try {
      await client.start();

      // Step 1 — PTR query: discover service instance names.
      final instanceNames = <String>[];
      await for (final ptr in client
          .lookup<PtrResourceRecord>(
            ResourceRecordQuery.serverPointer('_vibecli._tcp'),
          )
          .timeout(timeout, onTimeout: (sink) => sink.close())) {
        instanceNames.add(ptr.domainName);
      }

      // Step 2 — For each instance, resolve SRV + TXT.
      for (final instance in instanceNames) {
        int port = 7878;
        String? host;
        String? machineId;
        String? version;

        // SRV → host + port
        await for (final srv in client
            .lookup<SrvResourceRecord>(
              ResourceRecordQuery.service(instance),
            )
            .timeout(const Duration(seconds: 3), onTimeout: (sink) => sink.close())) {
          host = srv.target;
          port = srv.port;
        }

        // TXT → metadata key=value pairs
        await for (final txt in client
            .lookup<TxtResourceRecord>(
              ResourceRecordQuery.text(instance),
            )
            .timeout(const Duration(seconds: 3), onTimeout: (sink) => sink.close())) {
          for (final kv in txt.text.split('\n')) {
            if (kv.startsWith('machine_id=')) {
              machineId = kv.substring('machine_id='.length);
            } else if (kv.startsWith('version=')) {
              version = kv.substring('version='.length);
            }
          }
        }

        if (host != null) {
          // Resolve <hostname>.local. → IP via A record lookup.
          String resolvedHost = host;
          await for (final a in client
              .lookup<IPAddressResourceRecord>(
                ResourceRecordQuery.addressIPv4(host),
              )
              .timeout(const Duration(seconds: 3), onTimeout: (sink) => sink.close())) {
            resolvedHost = a.address.address;
            break; // take first
          }

          final key = machineId ?? '$resolvedHost:$port';
          results[key] = DiscoveredDaemon(
            host: resolvedHost,
            port: port,
            machineId: machineId,
            version: version,
          );
        }
      }
    } catch (e) {
      debugPrint('[mdns] discovery error: $e');
    } finally {
      client.stop();
    }

    return results.values.toList();
  }
}
