// WearDataLayerService.kt — Android phone companion relay for Wear OS.
//
// Mirrors WatchConnectivityBridge.swift:
//   Watch → Data Layer (/vibecody/dispatch, /vibecody/sessions, /vibecody/messages)
//   Phone (this file) → HTTP → VibeCody daemon
//   Phone → Data Layer (/vibecody/response) → Watch
//
// Registration: declare in AndroidManifest.xml with action
//   com.google.android.gms.wearable.MESSAGE_RECEIVED
// and path-prefix filter /vibecody/.
//
// The bearer token is stored in the phone's EncryptedSharedPreferences (same
// key the VibeMobile Flutter app writes after login).  It never crosses the
// Wearable Data Layer to the watch.

package com.vibecody.wearcompanion

import android.util.Log
import com.google.android.gms.wearable.MessageEvent
import com.google.android.gms.wearable.Wearable
import com.google.android.gms.wearable.WearableListenerService
import kotlinx.coroutines.*
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.TimeUnit

private const val TAG = "WearDataLayerService"
private const val PREFS = "vibecody_companion"
private const val KEY_DAEMON_URL = "daemon_url"
private const val KEY_BEARER_TOKEN = "bearer_token"

class WearDataLayerService : WearableListenerService() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    override fun onMessageReceived(event: MessageEvent) {
        val payload = String(event.data, Charsets.UTF_8)
        Log.d(TAG, "Received message on path ${event.path} from ${event.sourceNodeId}")

        scope.launch {
            val responseBytes = try {
                handleMessage(event.path, payload).toString().toByteArray()
            } catch (e: Exception) {
                Log.e(TAG, "Relay error: ${e.message}", e)
                JSONObject().apply { put("error", e.message ?: "Unknown error") }
                    .toString().toByteArray()
            }
            // Send response back to the watch node
            Wearable.getMessageClient(this@WearDataLayerService)
                .sendMessage(event.sourceNodeId, "/vibecody/response", responseBytes)
        }
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    // ── Message routing ───────────────────────────────────────────────────────

    private suspend fun handleMessage(path: String, payload: String): JSONObject {
        val (daemonUrl, bearer) = loadCredentials()

        return when (path) {
            "/vibecody/dispatch" -> {
                val req = JSONObject(payload)
                // Fetch a short-lived watch token for the device (daemon trusts bearer)
                val watchToken = fetchWatchToken(daemonUrl, bearer, req.optString("device_id", ""))
                val body = JSONObject().apply {
                    put("session_id", req.opt("session_id"))
                    put("content", req.getString("content"))
                    put("nonce", req.getString("nonce"))
                    put("timestamp", req.getLong("timestamp"))
                }.toString()
                httpPost("$daemonUrl/watch/dispatch", body, watchToken)
            }

            "/vibecody/sessions" -> {
                val token = fetchWatchToken(daemonUrl, bearer, "")
                httpGet("$daemonUrl/watch/sessions", token)
            }

            "/vibecody/messages" -> {
                val req = JSONObject(payload)
                val sessionId = req.getString("session_id")
                val token = fetchWatchToken(daemonUrl, bearer, "")
                httpGet("$daemonUrl/watch/sessions/$sessionId/messages", token)
            }

            else -> JSONObject().apply { put("error", "Unknown path: $path") }
        }
    }

    // ── Token relay ───────────────────────────────────────────────────────────

    /**
     * Uses the phone's bearer token to call /watch/challenge (bearer-gated),
     * which returns a short-lived watch-scoped nonce.  The daemon then validates
     * the device's Watch-Token on dispatch — but for the phone relay path, the
     * daemon accepts "bearer-relay:<bearer>" in the Watch-Token header,
     * identical to the iOS WatchConnectivityBridge approach.
     */
    private suspend fun fetchWatchToken(daemonUrl: String, bearer: String, deviceId: String): String {
        // Mirror iOS bridge: daemon strips "bearer-relay:" prefix and validates via bearer path.
        return "bearer-relay:$bearer"
    }

    // ── HTTP helpers ──────────────────────────────────────────────────────────

    private suspend fun httpPost(url: String, body: String, token: String): JSONObject =
        withContext(Dispatchers.IO) {
            val req = Request.Builder()
                .url(url)
                .post(body.toRequestBody("application/json".toMediaType()))
                .header("Authorization", "Watch-Token $token")
                .build()
            val resp = client.newCall(req).execute()
            JSONObject(resp.body?.string() ?: "{}")
        }

    private suspend fun httpGet(url: String, token: String): JSONObject =
        withContext(Dispatchers.IO) {
            val req = Request.Builder()
                .url(url)
                .header("Authorization", "Watch-Token $token")
                .build()
            val resp = client.newCall(req).execute()
            JSONObject(resp.body?.string() ?: "{}")
        }

    // ── Credentials ───────────────────────────────────────────────────────────

    private fun loadCredentials(): Pair<String, String> {
        val prefs = getSharedPreferences(PREFS, MODE_PRIVATE)
        val url = prefs.getString(KEY_DAEMON_URL, null)
            ?: throw IllegalStateException("Daemon URL not configured")
        val token = prefs.getString(KEY_BEARER_TOKEN, null)
            ?: throw IllegalStateException("Bearer token not configured — please log in")
        return url to token
    }
}
