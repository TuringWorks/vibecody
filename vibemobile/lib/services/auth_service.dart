import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Stores paired machine credentials securely.
class MachineCredential {
  final String baseUrl;
  final String token;
  final String machineId;
  final String machineName;
  final String deviceId;
  final int pairedAt;

  MachineCredential({
    required this.baseUrl,
    required this.token,
    required this.machineId,
    required this.machineName,
    required this.deviceId,
    required this.pairedAt,
  });

  Map<String, dynamic> toJson() => {
    'base_url': baseUrl,
    'token': token,
    'machine_id': machineId,
    'machine_name': machineName,
    'device_id': deviceId,
    'paired_at': pairedAt,
  };

  factory MachineCredential.fromJson(Map<String, dynamic> json) {
    return MachineCredential(
      baseUrl: json['base_url'],
      token: json['token'],
      machineId: json['machine_id'],
      machineName: json['machine_name'] ?? '',
      deviceId: json['device_id'] ?? '',
      pairedAt: json['paired_at'] ?? 0,
    );
  }
}

/// Manages authentication state and stored machine credentials.
class AuthService extends ChangeNotifier {
  static const _storageKey = 'vibecody_machines';
  static const _deviceIdKey = 'vibecody_device_id';

  final FlutterSecureStorage _secureStorage = const FlutterSecureStorage();
  late SharedPreferences _prefs;

  List<MachineCredential> _machines = [];
  String _deviceId = '';
  bool _initialized = false;

  List<MachineCredential> get machines => List.unmodifiable(_machines);
  String get deviceId => _deviceId;
  bool get isInitialized => _initialized;
  bool get hasMachines => _machines.isNotEmpty;

  /// Initialize by loading stored credentials.
  Future<void> init() async {
    _prefs = await SharedPreferences.getInstance();

    // Load or generate device ID.
    _deviceId = _prefs.getString(_deviceIdKey) ?? '';
    if (_deviceId.isEmpty) {
      _deviceId = 'vibe-mob-${DateTime.now().millisecondsSinceEpoch.toRadixString(16)}';
      await _prefs.setString(_deviceIdKey, _deviceId);
    }

    // Load stored machines from secure storage.
    final stored = await _secureStorage.read(key: _storageKey);
    if (stored != null) {
      try {
        final List<dynamic> list = jsonDecode(stored);
        _machines = list.map((m) => MachineCredential.fromJson(m)).toList();
      } catch (_) {
        _machines = [];
      }
    }

    _initialized = true;
    notifyListeners();
  }

  /// Add a paired machine credential.
  Future<void> addMachine(MachineCredential cred) async {
    // Remove existing entry for same machine.
    _machines.removeWhere((m) => m.machineId == cred.machineId);
    _machines.add(cred);
    await _persist();
    notifyListeners();
  }

  /// Remove a machine credential.
  Future<void> removeMachine(String machineId) async {
    _machines.removeWhere((m) => m.machineId == machineId);
    await _persist();
    notifyListeners();
  }

  /// Get credential for a specific machine.
  MachineCredential? getCredential(String machineId) {
    try {
      return _machines.firstWhere((m) => m.machineId == machineId);
    } catch (_) {
      return null;
    }
  }

  /// Add machine from QR code data (vibecody://pair?...).
  Future<MachineCredential> addFromQrData(String qrData, String token) async {
    final uri = Uri.parse(qrData);
    final machineId = uri.queryParameters['machine'] ?? '';
    final host = uri.queryParameters['host'] ?? 'localhost';
    final port = uri.queryParameters['port'] ?? '7878';
    final baseUrl = 'http://$host:$port';

    final cred = MachineCredential(
      baseUrl: baseUrl,
      token: token,
      machineId: machineId,
      machineName: host,
      deviceId: _deviceId,
      pairedAt: DateTime.now().millisecondsSinceEpoch ~/ 1000,
    );
    await addMachine(cred);
    return cred;
  }

  /// Add machine by manual URL + token entry.
  Future<MachineCredential> addManual(String baseUrl, String token, String name) async {
    final cred = MachineCredential(
      baseUrl: baseUrl,
      token: token,
      machineId: 'manual-${DateTime.now().millisecondsSinceEpoch.toRadixString(16)}',
      machineName: name,
      deviceId: _deviceId,
      pairedAt: DateTime.now().millisecondsSinceEpoch ~/ 1000,
    );
    await addMachine(cred);
    return cred;
  }

  Future<void> _persist() async {
    final json = jsonEncode(_machines.map((m) => m.toJson()).toList());
    await _secureStorage.write(key: _storageKey, value: json);
  }
}
