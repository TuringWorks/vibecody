// WearNetworkManager.kt — HTTP + SSE client for VibeCody Wear OS.
//
// Transport resolution (same priority as watchOS):
//   1. LAN (direct HTTP to daemon on local network)
//   2. Tailscale IP (stored during pairing if available)
//   3. Wearable Data Layer relay (through paired Android phone when offline)
//
// All requests carry "Watch-Token <jwt>" header.  SSE streaming uses OkHttp's
// EventSource to consume Server-Sent Events from /watch/stream/{session_id}.

package com.vibecody.wear

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.sse.EventSource
import okhttp3.sse.EventSourceListener
import okhttp3.sse.EventSources
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

private const val TAG = "WearNetworkManager"

class WearNetworkManager(
    private val context: Context,
    private val auth: WearAuthManager,
) {
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(0, TimeUnit.SECONDS)   // SSE: no read timeout
        .build()

    // ── Token management ──────────────────────────────────────────────────────

    /** Returns a valid access token, refreshing if needed. */
    private suspend fun validToken(): String {
        val cached = auth.accessToken
        if (cached != null) return cached
        return refreshAccessToken()
    }

    private suspend fun refreshAccessToken(): String = withContext(Dispatchers.IO) {
        val refreshToken = auth.refreshToken
            ?: throw IllegalStateException("No refresh token — re-pairing required")
        val timestamp = System.currentTimeMillis() / 1000
        val signature = auth.buildRefreshSignature(refreshToken, timestamp)

        val body = JSONObject().apply {
            put("device_id", auth.deviceId)
            put("refresh_token", refreshToken)
            put("timestamp", timestamp)
            put("signature", signature)
        }.toString().toRequestBody("application/json".toMediaType())

        val req = Request.Builder()
            .url("${auth.daemonUrl}/watch/refresh-token")
            .post(body)
            .build()

        val resp = client.newCall(req).awaitResponse()
        val json = JSONObject(resp.body?.string() ?: "{}")
        if (!resp.isSuccessful) throw IOException("Refresh failed: ${json.optString("error")}")

        val newAccess = json.getString("access_token")
        val newRefresh = json.getString("refresh_token")
        val expiresAt = json.getLong("expires_at")
        auth.saveTokens(newAccess, newRefresh, expiresAt)
        newAccess
    }

    // ── Watch-authenticated request builder ───────────────────────────────────

    private suspend fun watchRequest(url: String): Request.Builder {
        val token = validToken()
        return Request.Builder()
            .url(url)
            .header("Authorization", "Watch-Token $token")
    }

    // ── Sessions ──────────────────────────────────────────────────────────────

    suspend fun listSessions(): JSONObject = withContext(Dispatchers.IO) {
        val req = watchRequest("${auth.daemonUrl}/watch/sessions").get().build()
        val resp = client.newCall(req).awaitResponse()
        JSONObject(resp.body?.string() ?: "{}")
    }

    suspend fun getMessages(sessionId: String): JSONObject = withContext(Dispatchers.IO) {
        val req = watchRequest("${auth.daemonUrl}/watch/sessions/$sessionId/messages").get().build()
        val resp = client.newCall(req).awaitResponse()
        JSONObject(resp.body?.string() ?: "{}")
    }

    // ── Active session (Google Docs-style sync) ───────────────────────────────

    /** Tell the daemon which session this device is currently viewing. */
    suspend fun setActiveSession(sessionId: String) = withContext(Dispatchers.IO) {
        try {
            val body = JSONObject().put("session_id", sessionId).toString()
                .toRequestBody("application/json".toMediaType())
            val req = watchRequest("${auth.daemonUrl}/watch/active-session")
                .put(body)
                .build()
            client.newCall(req).awaitResponse()
        } catch (e: Exception) {
            Log.w(TAG, "setActiveSession failed (ignored): ${e.message}")
        }
    }

    /** Poll the daemon for the active session on VibeUI (for auto-switching). */
    suspend fun getActiveSession(): String? = withContext(Dispatchers.IO) {
        try {
            val req = watchRequest("${auth.daemonUrl}/watch/active-session").get().build()
            val resp = client.newCall(req).awaitResponse()
            JSONObject(resp.body?.string() ?: "{}").optString("session_id").takeIf { it.isNotEmpty() }
        } catch (e: Exception) {
            Log.w(TAG, "getActiveSession failed: ${e.message}")
            null
        }
    }

    // ── Sandbox chat session ──────────────────────────────────────────────────

    /** Fetch the VibeUI sandbox chat session ID so Sandbox tab shows the AI Chat card. */
    suspend fun getSandboxChatSession(): String? = withContext(Dispatchers.IO) {
        try {
            val req = watchRequest("${auth.daemonUrl}/watch/sandbox/chat-session").get().build()
            val resp = client.newCall(req).awaitResponse()
            JSONObject(resp.body?.string() ?: "{}").optString("session_id").takeIf { it.isNotEmpty() }
        } catch (e: Exception) {
            Log.w(TAG, "getSandboxChatSession failed: ${e.message}")
            null
        }
    }

    // ── Dispatch ──────────────────────────────────────────────────────────────

    suspend fun dispatch(content: String, sessionId: String? = null, provider: String? = null): JSONObject =
        withContext(Dispatchers.IO) {
            val nonce = auth.freshNonce()
            val timestamp = System.currentTimeMillis() / 1000
            val bodyJson = JSONObject().apply {
                if (sessionId != null) put("session_id", sessionId)
                put("content", content)
                put("nonce", nonce)
                put("timestamp", timestamp)
                if (provider != null) put("provider", provider)
            }.toString()

            val req = watchRequest("${auth.daemonUrl}/watch/dispatch")
                .post(bodyJson.toRequestBody("application/json".toMediaType()))
                .build()
            val resp = client.newCall(req).awaitResponse()
            JSONObject(resp.body?.string() ?: "{}")
        }

    // ── Poll for response (reliable fallback / complement to SSE) ─────────────

    /**
     * Poll every 1 second until the session has a new assistant message
     * (status = "complete" or "failed").  Returns the full message list.
     * Times out after [timeoutSeconds] (default 60).
     */
    suspend fun pollForResponse(sessionId: String, timeoutSeconds: Int = 60): List<WearMessage> =
        withContext(Dispatchers.IO) {
            var elapsed = 0
            while (elapsed < timeoutSeconds) {
                try {
                    val resp = getMessages(sessionId)
                    val arr = resp.optJSONArray("messages")
                    val msgs = buildList {
                        if (arr != null) for (i in 0 until arr.length()) {
                            val m = arr.getJSONObject(i)
                            add(WearMessage(m.getLong("id"), m.getString("role"), m.getString("content")))
                        }
                    }
                    val status = resp.optString("status", "running")
                    val hasAssistant = msgs.any { it.role == "assistant" }
                    val isDone = status == "complete" || status == "failed"
                    if (hasAssistant && isDone) return@withContext msgs
                } catch (_: Exception) {}
                delay(1_000)
                elapsed += 1
            }
            emptyList()
        }

    // ── SSE streaming ─────────────────────────────────────────────────────────

    /**
     * Open an SSE stream for the given session.  Events are delivered via
     * [onEvent].  Returns a [EventSource] that the caller can [EventSource.cancel]
     * to close the stream.
     */
    suspend fun openStream(
        sessionId: String,
        onEvent: (JSONObject) -> Unit,
        onError: (Throwable) -> Unit,
        onComplete: () -> Unit,
    ): EventSource {
        val token = validToken()
        val req = Request.Builder()
            .url("${auth.daemonUrl}/watch/stream/$sessionId")
            .header("Authorization", "Watch-Token $token")
            .header("Accept", "text/event-stream")
            .build()

        val factory = EventSources.createFactory(client)
        return factory.newEventSource(req, object : EventSourceListener() {
            override fun onEvent(eventSource: EventSource, id: String?, type: String?, data: String) {
                try { onEvent(JSONObject(data)) } catch (e: Exception) {
                    Log.w(TAG, "Bad SSE JSON: $data", e)
                }
            }
            override fun onFailure(eventSource: EventSource, t: Throwable?, response: Response?) {
                if (t != null) onError(t)
                else onComplete()
            }
            override fun onClosed(eventSource: EventSource) = onComplete()
        })
    }

    // ── Wrist event ───────────────────────────────────────────────────────────

    suspend fun reportWristEvent(onWrist: Boolean) = withContext(Dispatchers.IO) {
        val ts = System.currentTimeMillis() / 1000
        val sig = auth.buildWristSignature(auth.deviceId, onWrist, ts)
        val bodyJson = JSONObject().apply {
            put("device_id", auth.deviceId)
            put("on_wrist", onWrist)
            put("timestamp", ts)
            put("signature", sig)
        }.toString()

        try {
            val req = watchRequest("${auth.daemonUrl}/watch/wrist")
                .post(bodyJson.toRequestBody("application/json".toMediaType()))
                .build()
            client.newCall(req).awaitResponse()
        } catch (e: Exception) {
            Log.w(TAG, "Wrist event failed (will retry): ${e.message}")
        }
    }

    // ── Data Layer relay (offline fallback) ───────────────────────────────────

    fun relayDispatchViaPhone(
        context: Context,
        content: String,
        sessionId: String?,
        onResult: (JSONObject) -> Unit,
        onError: (String) -> Unit,
    ) {
        val payload = JSONObject().apply {
            put("action", "dispatch")
            put("content", content)
            if (sessionId != null) put("session_id", sessionId)
            put("nonce", auth.freshNonce())
            put("timestamp", System.currentTimeMillis() / 1000)
        }
        WearDataLayerClient.sendMessage(context, "/vibecody/dispatch", payload.toString().toByteArray(), onResult, onError)
    }
}

// ── OkHttp coroutine extension ────────────────────────────────────────────────

private suspend fun Call.awaitResponse(): Response = suspendCancellableCoroutine { cont ->
    enqueue(object : Callback {
        override fun onResponse(call: Call, response: Response) = cont.resume(response)
        override fun onFailure(call: Call, e: IOException) = cont.resumeWithException(e)
    })
    cont.invokeOnCancellation { cancel() }
}

private fun String.toReqBody() = toRequestBody("application/json".toMediaType())
