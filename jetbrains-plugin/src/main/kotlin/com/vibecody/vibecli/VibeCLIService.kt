package com.vibecody.vibecli

import com.google.gson.Gson
import com.google.gson.JsonObject
import com.intellij.openapi.components.Service
import com.intellij.openapi.diagnostic.thisLogger
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.CompletableFuture
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Application-level service that communicates with a running `vibecli serve` daemon.
 *
 * All network calls run on background threads via [CompletableFuture.supplyAsync].
 *
 * ## Daemon API used
 * | Method | Path | Description |
 * |--------|------|-------------|
 * | GET | /health | Liveness probe |
 * | POST | /chat | Stateless single-turn chat |
 * | POST | /agent | Start an agent session → returns session_id |
 * | GET | /stream/{id} | SSE stream of agent events |
 * | GET | /jobs | List persisted jobs |
 */
@Service(Service.Level.APP)
class VibeCLIService {

    private val gson = Gson()
    private val log = thisLogger()

    // ── Health ─────────────────────────────────────────────────────────────────

    fun isHealthy(): Boolean = try {
        val url = URL("${settings.daemonUrl}/health")
        val conn = url.openConnection() as HttpURLConnection
        conn.connectTimeout = 2_000
        conn.readTimeout = 2_000
        conn.requestMethod = "GET"
        conn.responseCode == 200
    } catch (_: Exception) {
        false
    }

    // ── Chat (single turn, non-streaming) ──────────────────────────────────────

    /**
     * POST /chat — single-turn stateless chat.
     * Returns the assistant reply text.
     */
    fun chat(message: String): CompletableFuture<String> =
        CompletableFuture.supplyAsync {
            val body = gson.toJson(
                mapOf(
                    "message" to message,
                    "provider" to settings.provider,
                    "model" to settings.model,
                )
            )
            val resp = postJson("/chat", body)
            resp.get("response")?.asString ?: resp.get("error")?.asString ?: "(empty response)"
        }

    // ── Agent ──────────────────────────────────────────────────────────────────

    /**
     * POST /agent — start an agent session.
     * Returns the `session_id` for streaming.
     */
    fun startAgent(task: String): CompletableFuture<String> =
        CompletableFuture.supplyAsync {
            val body = gson.toJson(
                mapOf(
                    "task" to task,
                    "provider" to settings.provider,
                    "model" to settings.model,
                    "approval" to settings.approvalMode,
                )
            )
            val resp = postJson("/agent", body)
            resp.get("session_id")?.asString
                ?: throw RuntimeException(resp.get("error")?.asString ?: "No session_id")
        }

    /**
     * GET /stream/{sessionId} — Server-Sent Events.
     *
     * Calls [onEvent] for each SSE data line. Calls [onDone] when the stream ends.
     * Returns an [AtomicBoolean] that the caller can set to `false` to cancel streaming.
     */
    fun streamSession(
        sessionId: String,
        onEvent: (AgentEvent) -> Unit,
        onDone: () -> Unit,
    ): AtomicBoolean {
        val active = AtomicBoolean(true)
        Thread {
            try {
                val url = URL("${settings.daemonUrl}/stream/$sessionId")
                val conn = url.openConnection() as HttpURLConnection
                conn.requestMethod = "GET"
                conn.setRequestProperty("Accept", "text/event-stream")
                conn.connectTimeout = 5_000
                conn.readTimeout = 0 // streaming — no read timeout
                conn.connect()

                BufferedReader(InputStreamReader(conn.inputStream)).use { reader ->
                    var dataBuffer = StringBuilder()
                    while (active.get()) {
                        val line = reader.readLine() ?: break
                        when {
                            line.startsWith("data:") -> dataBuffer.append(line.removePrefix("data:").trim())
                            line.isEmpty() && dataBuffer.isNotEmpty() -> {
                                val raw = dataBuffer.toString()
                                dataBuffer = StringBuilder()
                                parseEvent(raw)?.let(onEvent)
                            }
                        }
                    }
                }
            } catch (e: Exception) {
                if (active.get()) log.warn("SSE stream error for $sessionId: ${e.message}")
            } finally {
                onDone()
            }
        }.also {
            it.isDaemon = true
            it.name = "vibecli-sse-$sessionId"
            it.start()
        }
        return active
    }

    // ── Jobs ───────────────────────────────────────────────────────────────────

    /**
     * GET /jobs — list persisted agent jobs.
     */
    fun listJobs(): CompletableFuture<List<JobRecord>> =
        CompletableFuture.supplyAsync {
            try {
                val resp = getJson("/jobs")
                resp.asJsonArray.map { el ->
                    val obj = el.asJsonObject
                    JobRecord(
                        sessionId = obj.get("session_id")?.asString ?: "",
                        task = obj.get("task")?.asString ?: "",
                        status = obj.get("status")?.asString ?: "unknown",
                        provider = obj.get("provider")?.asString ?: "",
                        startedAt = obj.get("started_at")?.asLong ?: 0L,
                        summary = obj.get("summary")?.asString,
                    )
                }
            } catch (_: Exception) {
                emptyList()
            }
        }

    // ── Internal helpers ───────────────────────────────────────────────────────

    private fun postJson(path: String, body: String): JsonObject {
        val url = URL("${settings.daemonUrl}$path")
        val conn = url.openConnection() as HttpURLConnection
        conn.requestMethod = "POST"
        conn.setRequestProperty("Content-Type", "application/json")
        conn.setRequestProperty("Accept", "application/json")
        conn.doOutput = true
        conn.connectTimeout = 5_000
        conn.readTimeout = 60_000
        conn.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
        val text = conn.inputStream.bufferedReader().readText()
        return gson.fromJson(text, JsonObject::class.java)
    }

    private fun getJson(path: String): com.google.gson.JsonElement {
        val url = URL("${settings.daemonUrl}$path")
        val conn = url.openConnection() as HttpURLConnection
        conn.requestMethod = "GET"
        conn.setRequestProperty("Accept", "application/json")
        conn.connectTimeout = 5_000
        conn.readTimeout = 15_000
        val text = conn.inputStream.bufferedReader().readText()
        return gson.fromJson(text, com.google.gson.JsonElement::class.java)
    }

    private fun parseEvent(raw: String): AgentEvent? = try {
        val obj = gson.fromJson(raw, JsonObject::class.java)
        when (obj.get("type")?.asString) {
            "thinking"   -> AgentEvent.Thinking(obj.get("text")?.asString ?: "")
            "text"       -> AgentEvent.Text(obj.get("text")?.asString ?: "")
            "tool_call"  -> AgentEvent.ToolCall(
                name = obj.get("name")?.asString ?: "",
                input = obj.get("input")?.toString() ?: "",
            )
            "tool_result" -> AgentEvent.ToolResult(
                name = obj.get("name")?.asString ?: "",
                output = obj.get("output")?.asString ?: "",
            )
            "complete"   -> AgentEvent.Complete(obj.get("summary")?.asString ?: "")
            "error"      -> AgentEvent.Error(obj.get("message")?.asString ?: "unknown error")
            else         -> null
        }
    } catch (_: Exception) {
        null
    }

    private val settings get() = VibeCLISettings.getInstance().state

    companion object {
        fun getInstance(): VibeCLIService =
            com.intellij.openapi.application.ApplicationManager
                .getApplication()
                .getService(VibeCLIService::class.java)
    }
}

// ── Data classes ───────────────────────────────────────────────────────────────

sealed interface AgentEvent {
    data class Thinking(val text: String)  : AgentEvent
    data class Text(val text: String)       : AgentEvent
    data class ToolCall(val name: String, val input: String) : AgentEvent
    data class ToolResult(val name: String, val output: String) : AgentEvent
    data class Complete(val summary: String) : AgentEvent
    data class Error(val message: String)   : AgentEvent
}

data class JobRecord(
    val sessionId: String,
    val task: String,
    val status: String,
    val provider: String,
    val startedAt: Long,
    val summary: String?,
)
