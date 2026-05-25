package dev.vibecody.vibecody_mobile

import android.content.Context
import dev.vibecody.vibecody_mobile.wear.WearDataLayerService
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {

    companion object {
        private const val CHANNEL = "vibecody.relay/credentials"
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        // MethodChannel "vibecody.relay/credentials" — Flutter pushes the
        // active machine credentials here so the WearDataLayerService can
        // authenticate when relaying requests from VibeCodyWear to the
        // VibeCody daemon. See lib/services/relay_bridge.dart.
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                val prefs = getSharedPreferences(
                    WearDataLayerService.PREFS,
                    Context.MODE_PRIVATE
                )
                when (call.method) {
                    "setActiveMachine" -> {
                        val baseUrl = call.argument<String>("base_url")
                        val bearer = call.argument<String>("bearer_token")
                        val deviceId = call.argument<String>("device_id")
                        val machineId = call.argument<String>("machine_id")
                        if (baseUrl == null || bearer == null) {
                            result.error("bad_args", "Missing base_url or bearer_token", null)
                            return@setMethodCallHandler
                        }
                        prefs.edit()
                            .putString(WearDataLayerService.KEY_BASE_URL, baseUrl)
                            .putString(WearDataLayerService.KEY_BEARER_TOKEN, bearer)
                            .putString(WearDataLayerService.KEY_DEVICE_ID, deviceId ?: "")
                            .putString(WearDataLayerService.KEY_MACHINE_ID, machineId ?: "")
                            .apply()
                        result.success(null)
                    }

                    "clearActiveMachine" -> {
                        prefs.edit().clear().apply()
                        result.success(null)
                    }

                    else -> result.notImplemented()
                }
            }
    }
}
