// WearDataLayerService.kt — Android phone-relay for Wear OS.
//
// Wear watch may not have direct network access. When that happens it
// sends a request on the Wearable Data Layer (path-prefix /vibecody/)
// and this service relays it to the VibeCody daemon over the phone's
// network, then ships the response back on /vibecody/response.
//
// Registered in AndroidManifest.xml with action
//   com.google.android.gms.wearable.MESSAGE_RECEIVED
// and a path-prefix filter for /vibecody/.
//
// Credentials are written by the Flutter side via MethodChannel
// "vibecody.relay/credentials" (see lib/services/relay_bridge.dart and
// MainActivity.kt) into SharedPreferences("vibecody_companion", MODE_PRIVATE).

package dev.vibecody.vibecody_mobile.wear

import android.content.Context
import android.util.Log
import com.google.android.gms.wearable.MessageEvent
import com.google.android.gms.wearable.Wearable
import com.google.android.gms.wearable.WearableListenerService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.concurrent.TimeUnit

class WearDataLayerService : WearableListenerService() {

    companion object {
        private const val TAG = "VibeRelay"
        const val PREFS = "vibecody_companion"
        const val KEY_BASE_URL = "base_url"
        const val KEY_BEARER_TOKEN = "bearer_token"
        const val KEY_DEVICE_ID = "device_id"
        const val KEY_MACHINE_ID = "machine_id"
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    override fun onMessageReceived(event: MessageEvent) {
        val payload = String(event.data, Charsets.UTF_8)
        Log.d(TAG, "Relay request on ${event.path} from ${event.sourceNodeId}")

        scope.launch {
            val responseBytes = try {
                handleMessage(event.path, payload).toString().toByteArray()
            } catch (e: Exception) {
                Log.e(TAG, "Relay error: ${e.message}", e)
                JSONObject().apply { put("error", e.message ?: "Unknown error") }
                    .toString().toByteArray()
            }
            Wearable.getMessageClient(this@WearDataLayerService)
                .sendMessage(event.sourceNodeId, "/vibecody/response", responseBytes)
        }
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    private suspend fun handleMessage(path: String, payload: String): JSONObject {
        val creds = loadCredentials()
        val daemonUrl = creds.baseUrl
        val bearer = creds.bearerToken

        return when (path) {
            "/vibecody/dispatch" -> {
                val req = JSONObject(payload)
                val body = JSONObject().apply {
                    if (req.has("session_id")) put("session_id", req.opt("session_id"))
                    put("content", req.getString("content"))
                    put("nonce", req.optString("nonce"))
                    put("timestamp", req.optLong("timestamp", System.currentTimeMillis() / 1000))
                }.toString()
                httpPost("$daemonUrl/watch/dispatch", body, bearer)
            }

            "/vibecody/sessions" -> {
                httpGet("$daemonUrl/watch/sessions", bearer)
            }

            "/vibecody/messages" -> {
                val req = JSONObject(payload)
                val sessionId = req.getString("session_id")
                httpGet("$daemonUrl/watch/sessions/$sessionId/messages", bearer)
            }

            else -> JSONObject().apply { put("error", "Unknown path: $path") }
        }
    }

    // ── HTTP helpers ──────────────────────────────────────────────────────────
    //
    // Daemon /watch/dispatch + /watch/sessions[/:id/messages] accept either
    // `Authorization: Watch-Token <jwt>` or `Authorization: Bearer <token>`
    // (extract_any_auth in vibecli/vibecli-cli/src/watch_bridge.rs). The
    // phone-relay uses Bearer directly — the watch's own JWT never crosses
    // the air gap.

    private suspend fun httpPost(url: String, body: String, bearer: String): JSONObject =
        withContext(Dispatchers.IO) {
            val req = Request.Builder()
                .url(url)
                .post(body.toRequestBody("application/json".toMediaType()))
                .header("Authorization", "Bearer $bearer")
                .build()
            val resp = client.newCall(req).execute()
            JSONObject(resp.body?.string() ?: "{}")
        }

    private suspend fun httpGet(url: String, bearer: String): JSONObject =
        withContext(Dispatchers.IO) {
            val req = Request.Builder()
                .url(url)
                .header("Authorization", "Bearer $bearer")
                .build()
            val resp = client.newCall(req).execute()
            JSONObject(resp.body?.string() ?: "{}")
        }

    // ── Credentials — written by Flutter via MainActivity's MethodChannel ────

    data class Credentials(
        val baseUrl: String,
        val bearerToken: String,
        val deviceId: String,
        val machineId: String,
    )

    private fun loadCredentials(): Credentials {
        val prefs = getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val url = prefs.getString(KEY_BASE_URL, null)
            ?: throw IllegalStateException("Daemon URL not set — phone not paired?")
        val token = prefs.getString(KEY_BEARER_TOKEN, null)
            ?: throw IllegalStateException("Bearer token not set — phone not paired?")
        val deviceId = prefs.getString(KEY_DEVICE_ID, "") ?: ""
        val machineId = prefs.getString(KEY_MACHINE_ID, "") ?: ""
        return Credentials(url, token, deviceId, machineId)
    }
}
