import Flutter
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    // Activate the Apple Watch phone-relay bridge so WCSession messages
    // from VibeCodyWatch arrive whenever the iPhone app is running.
    WatchConnectivityBridge.shared.activate()

    GeneratedPluginRegistrant.register(with: self)
    registerRelayCredentialsChannel(with: self)

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  /// MethodChannel "vibecody.relay/credentials" — receives active-machine
  /// credentials from Flutter (RelayBridge.setActiveMachine) and stores
  /// them in Keychain where WatchConnectivityBridge can read them on each
  /// relay request.
  private func registerRelayCredentialsChannel(with registry: FlutterPluginRegistry) {
    guard let messenger = registry.registrar(forPlugin: "RelayCredentials")?.messenger() else {
      return
    }
    let channel = FlutterMethodChannel(
      name: "vibecody.relay/credentials",
      binaryMessenger: messenger
    )
    channel.setMethodCallHandler { call, result in
      switch call.method {
      case "setActiveMachine":
        guard let args = call.arguments as? [String: Any],
              let baseUrl     = args["base_url"]     as? String,
              let bearerToken = args["bearer_token"] as? String,
              let deviceId    = args["device_id"]    as? String,
              let machineId   = args["machine_id"]   as? String
        else {
          result(FlutterError(code: "bad_args", message: "Missing required field", details: nil))
          return
        }
        do {
          try RelayCredentialStore.writeActiveMachine(
            baseUrl:     baseUrl,
            bearerToken: bearerToken,
            deviceId:    deviceId,
            machineId:   machineId
          )
          result(nil)
        } catch {
          result(FlutterError(code: "keychain_write", message: error.localizedDescription, details: nil))
        }

      case "clearActiveMachine":
        RelayCredentialStore.clearActiveMachine()
        result(nil)

      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }
}
