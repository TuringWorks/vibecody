// relay_bridge.dart — pushes paired-machine credentials into a native
// keystore that the iOS/Android phone-relay companions can read.
//
// Watch app may not have direct internet (no LTE, off-WiFi). When that
// happens it falls back to WatchConnectivity (iOS) or Wearable Data Layer
// (Android) and asks the phone to relay HTTP requests to the VibeCody
// daemon. The native services that handle those relays cannot reach
// flutter_secure_storage directly — its key layout is plugin-internal —
// so this bridge writes a stable, documented set of keys to:
//   iOS:     Keychain  (kSecAttrService = "com.turingworks.vibecody.companion")
//   Android: SharedPreferences("vibecody_companion", MODE_PRIVATE)
//
// Companions on the receiving end:
//   - vibemobile/ios/Runner/WatchConnectivityBridge.swift
//   - vibemobile/android/app/src/main/kotlin/.../wear/WearDataLayerService.kt

import 'package:flutter/services.dart';

class RelayBridge {
  static const _channel = MethodChannel('vibecody.relay/credentials');

  /// Push the active machine credentials to the native keystore so the
  /// companion can authenticate when the watch relays a request through
  /// the phone. Safe to call on every machine change.
  static Future<void> setActiveMachine({
    required String baseUrl,
    required String bearerToken,
    required String deviceId,
    required String machineId,
  }) async {
    try {
      await _channel.invokeMethod('setActiveMachine', {
        'base_url': baseUrl,
        'bearer_token': bearerToken,
        'device_id': deviceId,
        'machine_id': machineId,
      });
    } on MissingPluginException {
      // Native handler not registered (e.g. running in tests / on web) — no-op.
    } on PlatformException {
      // Native store unavailable — relay will fall back to direct daemon access.
    }
  }

  /// Clear the cached credentials when the user unpairs / signs out.
  static Future<void> clearActiveMachine() async {
    try {
      await _channel.invokeMethod('clearActiveMachine');
    } on MissingPluginException {
      // No-op when no native handler.
    } on PlatformException {
      // Ignore.
    }
  }
}
