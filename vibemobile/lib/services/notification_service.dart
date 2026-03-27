import 'package:flutter/foundation.dart';

/// Manages push notification registration and display.
///
/// On iOS: APNs token via firebase_messaging.
/// On Android: FCM token via firebase_messaging.
class NotificationService extends ChangeNotifier {
  String? _pushToken;
  bool _permissionGranted = false;
  final List<AppNotification> _notifications = [];

  String? get pushToken => _pushToken;
  bool get permissionGranted => _permissionGranted;
  List<AppNotification> get notifications => List.unmodifiable(_notifications);
  int get unreadCount => _notifications.where((n) => !n.read).length;

  /// Initialize push notification service.
  /// In production this would call FirebaseMessaging.instance.getToken().
  Future<void> init() async {
    // Placeholder — actual FCM/APNs init happens here.
    _permissionGranted = true;
    _pushToken = 'mock-push-token-${DateTime.now().millisecondsSinceEpoch}';
    notifyListeners();
  }

  /// Request notification permissions.
  Future<bool> requestPermission() async {
    // In production: FirebaseMessaging.instance.requestPermission()
    _permissionGranted = true;
    notifyListeners();
    return true;
  }

  /// Add a local notification.
  void addNotification(AppNotification notification) {
    _notifications.insert(0, notification);
    if (_notifications.length > 100) {
      _notifications.removeLast();
    }
    notifyListeners();
  }

  /// Mark a notification as read.
  void markRead(String id) {
    final idx = _notifications.indexWhere((n) => n.id == id);
    if (idx >= 0) {
      _notifications[idx] = _notifications[idx].copyWith(read: true);
      notifyListeners();
    }
  }

  /// Mark all notifications as read.
  void markAllRead() {
    for (int i = 0; i < _notifications.length; i++) {
      _notifications[i] = _notifications[i].copyWith(read: true);
    }
    notifyListeners();
  }

  /// Clear all notifications.
  void clear() {
    _notifications.clear();
    notifyListeners();
  }
}

class AppNotification {
  final String id;
  final String title;
  final String body;
  final String category;
  final String? machineId;
  final String? taskId;
  final DateTime createdAt;
  final bool read;

  AppNotification({
    required this.id,
    required this.title,
    required this.body,
    required this.category,
    this.machineId,
    this.taskId,
    DateTime? createdAt,
    this.read = false,
  }) : createdAt = createdAt ?? DateTime.now();

  AppNotification copyWith({bool? read}) {
    return AppNotification(
      id: id,
      title: title,
      body: body,
      category: category,
      machineId: machineId,
      taskId: taskId,
      createdAt: createdAt,
      read: read ?? this.read,
    );
  }
}
