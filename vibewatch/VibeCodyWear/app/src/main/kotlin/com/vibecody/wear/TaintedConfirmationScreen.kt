// TaintedConfirmationScreen.kt — DREAD #1 Slice G part 3 (Wear OS).
//
// Tile-style approve/deny prompt for a tainted-argument event the
// daemon enqueued via the shared `HttpPromptQueue` (same queue as
// the desktop modal, mobile sheet, and watchOS overlay).
//
// Threat-model invariants:
//   * Payload bytes never reach this screen — `summary` carries only
//     the audit summary (kind, origin fields, audit_id).
//   * Deny-by-default: leaving the screen without tapping Approve
//     does not send anything; daemon timeout (5 min) denies the
//     agent loop.
//   * Only an explicit Approve tap fires `approve=true`.

package com.vibecody.wear

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.wear.compose.material.Button
import androidx.wear.compose.material.ButtonDefaults
import androidx.wear.compose.material.MaterialTheme
import androidx.wear.compose.material.Text
import kotlinx.coroutines.DelicateCoroutinesApi
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import okhttp3.sse.EventSource
import org.json.JSONObject

/** Mirror of the daemon's `PendingPromptEvent` JSON. */
data class TaintedPromptEvent(
    val requestId: String,
    val auditId: String,
    val summary: String,
    val sink: String,
    val issuedAt: Long,
) {
    companion object {
        fun fromJson(j: JSONObject): TaintedPromptEvent = TaintedPromptEvent(
            requestId = j.optString("request_id", ""),
            auditId = j.optString("audit_id", ""),
            summary = j.optString("summary", ""),
            sink = j.optString("sink", "ToolCallArgument"),
            issuedAt = j.optLong("issued_at", 0),
        )
    }
}

/** Human-friendly sink label for the header row. */
private fun sinkLabel(sink: String): String = when (sink) {
    "ToolCallArgument" -> "Run tool"
    "McpArgument" -> "Call MCP tool"
    "RagDocument" -> "Use document"
    "WebFetch" -> "Fetch URL"
    "LlmRequestBody" -> "Send to LLM"
    "LogLine" -> "Emit log"
    "ShellCommand" -> "Run shell"
    else -> "Confirm action"
}

/**
 * Owns the SSE subscription and the FIFO queue. Mounted by
 * [TaintedConfirmationOverlay] so it lives as long as a paired
 * activity is on-screen.
 */
@OptIn(DelicateCoroutinesApi::class)
class TaintedConfirmationQueueState(private val net: WearNetworkManager) {
    var pending = mutableStateListOf<TaintedPromptEvent>()
        private set
    private val seen = mutableSetOf<String>()
    private val resolved = mutableSetOf<String>()
    private var source: EventSource? = null
    private var backoffMs: Long = 1_000L
    private val maxBackoffMs: Long = 30_000L
    private var active = false

    val head: TaintedPromptEvent?
        get() = pending.firstOrNull()

    val queuedBehind: Int
        get() = (pending.size - 1).coerceAtLeast(0)

    fun start() {
        if (active) return
        active = true
        connect()
    }

    fun stop() {
        active = false
        source?.cancel()
        source = null
    }

    private fun connect() {
        if (!active) return
        GlobalScope.launch {
            try {
                source = net.openTaintedPendingStream(
                    onEvent = { json -> onEvent(json) },
                    onError = { scheduleReconnect() },
                    onComplete = { scheduleReconnect() },
                )
                backoffMs = 1_000L
            } catch (e: Exception) {
                scheduleReconnect()
            }
        }
    }

    private fun scheduleReconnect() {
        source?.cancel()
        source = null
        if (!active) return
        val delayMs = backoffMs
        backoffMs = (backoffMs * 2).coerceAtMost(maxBackoffMs)
        GlobalScope.launch {
            delay(delayMs)
            if (active) connect()
        }
    }

    private fun onEvent(json: JSONObject) {
        val event = TaintedPromptEvent.fromJson(json)
        if (event.requestId.isEmpty()) return
        if (resolved.contains(event.requestId)) return
        if (!seen.add(event.requestId)) return
        pending.add(event)
    }

    /** Optimistic pop + POST. Failure does NOT re-queue; daemon
     *  timeout denies on its own.  */
    fun respond(event: TaintedPromptEvent, approve: Boolean) {
        resolved.add(event.requestId)
        pending.removeAll { it.requestId == event.requestId }
        GlobalScope.launch {
            try {
                net.taintedRespond(event.requestId, approve)
            } catch (_: Exception) {
                // Swallow — daemon will deny on timeout.
            }
        }
    }
}

/** Always-on overlay; renders nothing when the queue is empty. */
@Composable
fun TaintedConfirmationOverlay(net: WearNetworkManager) {
    val state = remember { TaintedConfirmationQueueState(net) }
    DisposableEffect(Unit) {
        state.start()
        onDispose { state.stop() }
    }
    val head = state.head ?: return
    TaintedConfirmationScreen(
        prompt = head,
        queuedBehind = state.queuedBehind,
        onApprove = { state.respond(head, true) },
        onDeny = { state.respond(head, false) },
    )
}

@Composable
fun TaintedConfirmationScreen(
    prompt: TaintedPromptEvent,
    queuedBehind: Int,
    onApprove: () -> Unit,
    onDeny: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = 0.92f))
            .padding(8.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = "⚠ ${sinkLabel(prompt.sink)}",
                fontSize = 12.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colors.error,
            )
        }

        Text(
            text = "Untrusted data — review before approving.",
            fontSize = 10.sp,
            color = MaterialTheme.colors.onSurface.copy(alpha = 0.7f),
        )

        Column(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(4.dp))
                .background(Color.DarkGray.copy(alpha = 0.5f))
                .padding(6.dp),
        ) {
            Text(
                text = prompt.summary,
                fontSize = 9.sp,
                fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colors.onSurface,
            )
        }

        Row(modifier = Modifier.fillMaxWidth()) {
            Text(
                text = prompt.auditId.take(8) + "…",
                fontSize = 8.sp,
                color = MaterialTheme.colors.onSurface.copy(alpha = 0.5f),
            )
            if (queuedBehind > 0) {
                Spacer(modifier = Modifier.fillMaxWidth(0.5f))
                Text(
                    text = "+$queuedBehind more",
                    fontSize = 8.sp,
                    color = MaterialTheme.colors.onSurface.copy(alpha = 0.5f),
                )
            }
        }

        Spacer(modifier = Modifier.height(4.dp))

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Button(
                onClick = onDeny,
                colors = ButtonDefaults.secondaryButtonColors(),
                modifier = Modifier.fillMaxWidth(0.5f),
            ) {
                Text("Deny", fontSize = 11.sp)
            }
            Button(
                onClick = onApprove,
                colors = ButtonDefaults.buttonColors(
                    backgroundColor = MaterialTheme.colors.error,
                ),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Approve", fontSize = 11.sp)
            }
        }
    }
}
