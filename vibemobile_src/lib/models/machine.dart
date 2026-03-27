/// Represents a registered VibeCody machine.
class Machine {
  final String machineId;
  final String name;
  final String hostname;
  final String os;
  final String arch;
  final String status;
  final int daemonPort;
  final String daemonVersion;
  final String workspaceRoot;
  final List<String> capabilities;
  final int activeSessions;
  final int maxSessions;
  final int cpuCores;
  final double memoryGb;
  final double diskFreeGb;
  final int registeredAt;
  final int lastHeartbeat;
  final String? tailscaleIp;
  final String? publicUrl;
  final List<String> tags;

  Machine({
    required this.machineId,
    required this.name,
    required this.hostname,
    required this.os,
    this.arch = '',
    required this.status,
    this.daemonPort = 7878,
    this.daemonVersion = '',
    this.workspaceRoot = '',
    this.capabilities = const [],
    this.activeSessions = 0,
    this.maxSessions = 8,
    this.cpuCores = 0,
    this.memoryGb = 0,
    this.diskFreeGb = 0,
    this.registeredAt = 0,
    this.lastHeartbeat = 0,
    this.tailscaleIp,
    this.publicUrl,
    this.tags = const [],
  });

  factory Machine.fromJson(Map<String, dynamic> json) {
    return Machine(
      machineId: json['machine_id'] ?? '',
      name: json['name'] ?? '',
      hostname: json['hostname'] ?? '',
      os: json['os'] ?? '',
      arch: json['arch'] ?? '',
      status: json['status'] ?? 'offline',
      daemonPort: json['daemon_port'] ?? 7878,
      daemonVersion: json['daemon_version'] ?? '',
      workspaceRoot: json['workspace_root'] ?? json['workspace'] ?? '',
      capabilities: List<String>.from(json['capabilities'] ?? []),
      activeSessions: json['active_sessions'] ?? json['active_tasks'] ?? 0,
      maxSessions: json['max_sessions'] ?? 8,
      cpuCores: json['cpu_cores'] ?? 0,
      memoryGb: (json['memory_gb'] ?? 0).toDouble(),
      diskFreeGb: (json['disk_free_gb'] ?? 0).toDouble(),
      registeredAt: json['registered_at'] ?? 0,
      lastHeartbeat: json['last_heartbeat'] ?? 0,
      tailscaleIp: json['tailscale_ip'],
      publicUrl: json['public_url'],
      tags: List<String>.from(json['tags'] ?? []),
    );
  }

  bool get isOnline => status == 'online' || status == 'idle' || status == 'busy';
  bool get isBusy => status == 'busy';

  String get osIcon {
    switch (os.toLowerCase()) {
      case 'macos': return '🍎';
      case 'linux': return '🐧';
      case 'windows': return '🪟';
      case 'docker': return '🐳';
      case 'wsl': return '🐧';
      default: return '💻';
    }
  }
}

/// A dispatched task from mobile to machine.
class DispatchTask {
  final String taskId;
  final String machineId;
  final String deviceId;
  final String dispatchType;
  final String payload;
  final String status;
  final int createdAt;
  final int? startedAt;
  final int? completedAt;
  final String? result;
  final String? error;
  final String? sessionId;

  DispatchTask({
    required this.taskId,
    required this.machineId,
    this.deviceId = '',
    required this.dispatchType,
    required this.payload,
    required this.status,
    required this.createdAt,
    this.startedAt,
    this.completedAt,
    this.result,
    this.error,
    this.sessionId,
  });

  factory DispatchTask.fromJson(Map<String, dynamic> json) {
    return DispatchTask(
      taskId: json['task_id'] ?? '',
      machineId: json['machine_id'] ?? '',
      deviceId: json['device_id'] ?? '',
      dispatchType: json['dispatch_type'] ?? 'chat',
      payload: json['payload'] ?? '',
      status: json['status'] ?? 'queued',
      createdAt: json['created_at'] ?? 0,
      startedAt: json['started_at'],
      completedAt: json['completed_at'],
      result: json['result'],
      error: json['error'],
      sessionId: json['session_id'],
    );
  }

  bool get isRunning => status == 'running' || status == 'sent' || status == 'queued';
  bool get isComplete => status == 'completed';
  bool get isFailed => status == 'failed' || status == 'timed_out';
}

/// Gateway statistics.
class GatewayStats {
  final int totalMachines;
  final int onlineMachines;
  final int totalDevices;
  final int totalDispatches;
  final int activeDispatches;
  final int completedDispatches;
  final int failedDispatches;
  final int pendingNotifications;
  final int pendingPairings;

  GatewayStats({
    this.totalMachines = 0,
    this.onlineMachines = 0,
    this.totalDevices = 0,
    this.totalDispatches = 0,
    this.activeDispatches = 0,
    this.completedDispatches = 0,
    this.failedDispatches = 0,
    this.pendingNotifications = 0,
    this.pendingPairings = 0,
  });

  factory GatewayStats.fromJson(Map<String, dynamic> json) {
    return GatewayStats(
      totalMachines: json['total_machines'] ?? 0,
      onlineMachines: json['online_machines'] ?? 0,
      totalDevices: json['total_devices'] ?? 0,
      totalDispatches: json['total_dispatches'] ?? 0,
      activeDispatches: json['active_dispatches'] ?? 0,
      completedDispatches: json['completed_dispatches'] ?? 0,
      failedDispatches: json['failed_dispatches'] ?? 0,
      pendingNotifications: json['pending_notifications'] ?? 0,
      pendingPairings: json['pending_pairings'] ?? 0,
    );
  }
}
