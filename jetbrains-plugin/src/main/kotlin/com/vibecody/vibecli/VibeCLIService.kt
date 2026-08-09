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

    /**
     * Parse one `/stream/{id}` SSE payload.
     *
     * The daemon's wire shape is `AgentEventPayload` in `job_manager.rs`:
     * `{type, content, step_num, tool_name, success, …}`. This used to look
     * for `thinking`/`text`/`tool_call`/`tool_result` and read `summary` /
     * `message` / `text` — kinds and fields the daemon has never emitted — so
     * every streamed token fell through to `null` and the agent tool window
     * stayed blank while `complete` rendered an empty summary.
     */
    private fun parseEvent(raw: String): AgentEvent? = try {
        val obj = gson.fromJson(raw, JsonObject::class.java)
        val content = obj.get("content")?.takeIf { !it.isJsonNull }?.asString ?: ""
        when (obj.get("type")?.asString) {
            "chunk"    -> AgentEvent.Text(content)
            "step"     -> AgentEvent.Step(
                stepNum = obj.get("step_num")?.takeIf { !it.isJsonNull }?.asInt ?: 0,
                tool = obj.get("tool_name")?.takeIf { !it.isJsonNull }?.asString ?: "tool",
                // Nullable on purpose: `?: true` would paint a step whose
                // outcome the daemon never reported as a success.
                success = obj.get("success")?.takeIf { !it.isJsonNull }?.asBoolean,
            )
            "system"   -> AgentEvent.System(content)
            "retry"    -> AgentEvent.Retry(
                message = content,
                attempt = obj.get("attempt")?.takeIf { !it.isJsonNull }?.asInt ?: 0,
                maxAttempts = obj.get("max_attempts")?.takeIf { !it.isJsonNull }?.asInt ?: 0,
                backoffMs = obj.get("backoff_ms")?.takeIf { !it.isJsonNull }?.asLong ?: 0L,
            )
            "complete" -> AgentEvent.Complete(content)
            "partial"  -> AgentEvent.Partial(
                summary = content,
                stepsCompleted = obj.get("steps_completed")?.takeIf { !it.isJsonNull }?.asInt ?: 0,
                stepsPlanned = obj.get("steps_planned")?.takeIf { !it.isJsonNull }?.asInt ?: 0,
                remainingPlan = obj.get("remaining_plan")
                    ?.takeIf { it.isJsonArray }
                    ?.asJsonArray
                    ?.map { it.asString }
                    ?: emptyList(),
            )
            "error"    -> AgentEvent.Error(content.ifEmpty { "unknown error" })
            // `user` is replay-only; anything else is a kind added after this
            // build. Both are safely ignored.
            else       -> null
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

/** Mirrors the daemon's `AgentEventPayload` kinds — see `job_manager.rs`. */
sealed interface AgentEvent {
    /** A streamed model token (`chunk`). */
    data class Text(val text: String) : AgentEvent

    /** A completed tool call (`step`). `success` is null when the daemon
     *  reported no outcome — rendered as unknown, never as ok. */
    data class Step(val stepNum: Int, val tool: String, val success: Boolean?) : AgentEvent

    /** A daemon advisory that isn't model output (`system`). */
    data class System(val text: String) : AgentEvent

    /** Non-terminal: a transient failure is being backed off and retried. */
    data class Retry(
        val message: String,
        val attempt: Int,
        val maxAttempts: Int,
        val backoffMs: Long,
    ) : AgentEvent

    /** Terminal: the agent finished the task. */
    data class Complete(val summary: String) : AgentEvent

    /** Terminal, but the agent stopped with planned work outstanding. */
    data class Partial(
        val summary: String,
        val stepsCompleted: Int,
        val stepsPlanned: Int,
        val remainingPlan: List<String>,
    ) : AgentEvent

    /** Terminal: the run failed. */
    data class Error(val message: String) : AgentEvent
}

data class JobRecord(
    val sessionId: String,
    val task: String,
    val status: String,
    val provider: String,
    val startedAt: Long,
    val summary: String?,
)
