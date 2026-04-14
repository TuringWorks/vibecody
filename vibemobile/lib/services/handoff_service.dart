import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import 'auth_service.dart';

/// Beacon data returned by the daemon's /mobile/beacon endpoint.
class BeaconResponse {
  final String machineId;
  final String hostname;
  final int port;
  final List<String> lanIps;
  final String? tailscaleIp;
  final String? publicUrl;
  final int uptimeSecs;
  final ActiveSessionInfo? activeSession;

  BeaconResponse({
    required this.machineId,
    required this.hostname,
    required this.port,
    required this.lanIps,
    this.tailscaleIp,
    this.publicUrl,
    required this.uptimeSecs,
    this.activeSession,
  });

  factory BeaconResponse.fromJson(Map<String, dynamic> json) {
    return BeaconResponse(
      machineId: json['machine_id'] ?? '',
      hostname: json['hostname'] ?? '',
      port: json['port'] ?? 7878,
      lanIps: List<String>.from(json['lan_ips'] ?? []),
      tailscaleIp: json['tailscale_ip'],
      publicUrl: json['public_url'],
      uptimeSecs: json['uptime_secs'] ?? 0,
      activeSession: json['active_session'] != null
          ? ActiveSessionInfo.fromJson(json['active_session'])
          : null,
    );
  }
}

/// Info about the active (or most recent) session on a machine.
class ActiveSessionInfo {
  final String sessionId;
  final String task;
  final String provider;
  final String status;
  final int startedAt;
  final int messageCount;
  final String? summary;

  ActiveSessionInfo({
    required this.sessionId,
    required this.task,
    required this.provider,
    required this.status,
    required this.startedAt,
    required this.messageCount,
    this.summary,
  });

  factory ActiveSessionInfo.fromJson(Map<String, dynamic> json) {
    return ActiveSessionInfo(
      sessionId: json['session_id'] ?? '',
      task: json['task'] ?? '',
      provider: json['provider'] ?? '',
      status: json['status'] ?? 'unknown',
      startedAt: json['started_at'] ?? 0,
      messageCount: json['message_count'] ?? 0,
      summary: json['summary'],
    );
  }
}

/// A machine + session pair that can be handed off to the mobile client.
class HandoffCandidate {
  final String machineId;
  final String machineName;
  final String resolvedUrl;
  final String token;
  final ActiveSessionInfo session;

  HandoffCandidate({
    required this.machineId,
    required this.machineName,
    required this.resolvedUrl,
    required this.token,
    required this.session,
  });
}

/// Discovers active sessions on paired machines and resolves the best
/// network URL (LAN → Tailscale → public) for each machine.
class HandoffService extends ChangeNotifier {
  final AuthService _auth;
  final http.Client _http = http.Client();

  /// machineId -> best resolved URL
  final Map<String, String> _resolvedUrls = {};

  /// machineId -> most recent beacon
  final Map<String, BeaconResponse> _beacons = {};

  List<HandoffCandidate> _candidates = [];
  bool _probing = false;
  Timer? _timer;

  List<HandoffCandidate> get candidates => List.unmodifiable(_candidates);
  bool get hasHandoff => _candidates.isNotEmpty;
  bool get probing => _probing;

  /// Returns the best known URL for a machine, falling back to stored baseUrl.
  String resolvedUrl(String machineId) =>
      _resolvedUrls[machineId] ??
      _auth.getCredential(machineId)?.baseUrl ??
      '';

  HandoffService(this._auth) {
    _auth.addListener(_onMachinesChanged);
    _timer = Timer.periodic(const Duration(seconds: 60), (_) => probe());
    if (_auth.isInitialized) probe();
  }

  void _onMachinesChanged() {
    probe();
  }

  /// Probe all paired machines: resolve best URL + detect handoff candidates.
  Future<void> probe() async {
    if (_auth.machines.isEmpty) return;
    _probing = true;
    notifyListeners();

    for (final cred in _auth.machines) {
      // Step 1 — try to fetch a beacon using the current best or stored URL.
      final currentBest = _resolvedUrls[cred.machineId] ?? cred.baseUrl;
      final beacon = await _fetchBeacon(currentBest);

      if (beacon != null) {
        _beacons[cred.machineId] = beacon;
      }

      // Step 2 — collect all candidate URLs.
      final candidates = <String>{cred.baseUrl};
      final storedBeacon = _beacons[cred.machineId];
      if (storedBeacon != null) {
        for (final ip in storedBeacon.lanIps) {
          candidates.add('http://$ip:${storedBeacon.port}');
        }
        if (storedBeacon.tailscaleIp != null) {
          candidates.add('http://${storedBeacon.tailscaleIp}:${storedBeacon.port}');
        }
        if (storedBeacon.publicUrl != null) {
          candidates.add(storedBeacon.publicUrl!);
        }
      }

      // Step 3 — race all URLs, pick the fastest.
      final best = await _resolveUrl(candidates.toList());
      if (best != null) {
        _resolvedUrls[cred.machineId] = best;
        // Fetch fresh beacon from the best URL if we haven't yet.
        if (beacon == null || best != currentBest) {
          final freshBeacon = await _fetchBeacon(best);
          if (freshBeacon != null) {
            _beacons[cred.machineId] = freshBeacon;
          }
        }
      }
    }

    // Step 4 — build handoff candidates from beacons with recent/active sessions.
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final newCandidates = <HandoffCandidate>[];

    for (final cred in _auth.machines) {
      final beacon = _beacons[cred.machineId];
      if (beacon == null) continue;
      final session = beacon.activeSession;
      if (session == null) continue;

      final ageSeconds = now - session.startedAt;
      final isRecentOrLive =
          session.status == 'running' || ageSeconds < (30 * 60);

      if (isRecentOrLive) {
        newCandidates.add(HandoffCandidate(
          machineId: cred.machineId,
          machineName: cred.machineName.isNotEmpty ? cred.machineName : beacon.hostname,
          resolvedUrl: resolvedUrl(cred.machineId),
          token: cred.token,
          session: session,
        ));
      }
    }

    // Sort: running sessions first, then by recency.
    newCandidates.sort((a, b) {
      if (a.session.status == 'running' && b.session.status != 'running') return -1;
      if (b.session.status == 'running' && a.session.status != 'running') return 1;
      return b.session.startedAt.compareTo(a.session.startedAt);
    });

    _candidates = newCandidates;
    _probing = false;
    notifyListeners();
  }

  /// Race a list of base URLs and return the one that responds fastest.
  Future<String?> _resolveUrl(List<String> urls) async {
    if (urls.isEmpty) return null;
    if (urls.length == 1) {
      final ok = await _ping(urls.first);
      return ok ? urls.first : null;
    }

    final completer = Completer<String?>();
    int remaining = urls.length;

    for (final url in urls) {
      _pingTimed(url).then((result) {
        remaining--;
        if (result != null && !completer.isCompleted) {
          completer.complete(result);
        } else if (remaining == 0 && !completer.isCompleted) {
          completer.complete(null);
        }
      });
    }

    return completer.future;
  }

  /// Returns the url if it responds within 3 seconds, else null.
  Future<String?> _pingTimed(String baseUrl) async {
    try {
      final resp = await _http
          .get(Uri.parse('$baseUrl/health'))
          .timeout(const Duration(seconds: 3));
      if (resp.statusCode == 200) return baseUrl;
    } catch (_) {}
    return null;
  }

  Future<bool> _ping(String baseUrl) async {
    return await _pingTimed(baseUrl) != null;
  }

  /// Fetch beacon from a base URL, returning null on any error.
  Future<BeaconResponse?> _fetchBeacon(String baseUrl) async {
    try {
      final resp = await _http
          .get(Uri.parse('$baseUrl/mobile/beacon'))
          .timeout(const Duration(seconds: 3));
      if (resp.statusCode == 200) {
        return BeaconResponse.fromJson(jsonDecode(resp.body));
      }
    } catch (_) {}
    return null;
  }

  @override
  void dispose() {
    _timer?.cancel();
    _auth.removeListener(_onMachinesChanged);
    _http.close();
    super.dispose();
  }
}
