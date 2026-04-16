// ConversationScreen.kt — Chat conversation view for Wear OS.
//
// - ScalingLazyColumn: no maxLines, full text shown and scrollable
// - Active session sync: tells daemon which session we're viewing
// - Polling loop: refreshes messages every 2s for Google Docs-style sync
// - SSE streaming: shows live streaming delta while agent responds
// - Polling fallback: if SSE misses a response, poll catches it

package com.vibecody.wear

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.wear.compose.material.*
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import org.json.JSONObject

@Composable
fun ConversationScreen(
    net: WearNetworkManager,
    sessionId: String?,
) {
    val scope = rememberCoroutineScope()
    var messages by remember { mutableStateOf<List<WearMessage>>(emptyList()) }
    var streamingText by remember { mutableStateOf<String?>(null) }
    var isStreaming by remember { mutableStateOf(false) }
    var activeSessionId by remember { mutableStateOf(sessionId) }
    var loading by remember { mutableStateOf(sessionId != null) }
    var showVoice by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val listState = rememberScalingLazyListState()

    // Load messages and announce active session when opened
    LaunchedEffect(activeSessionId) {
        val sid = activeSessionId ?: return@LaunchedEffect
        loading = true

        // Tell daemon which session we're viewing (Google Docs-style session lock)
        net.setActiveSession(sid)

        try {
            val resp = net.getMessages(sid)
            val arr = resp.optJSONArray("messages")
            messages = buildList {
                if (arr != null) for (i in 0 until arr.length()) {
                    val m = arr.getJSONObject(i)
                    add(WearMessage(
                        id = m.getLong("id"),
                        role = m.getString("role"),
                        content = m.getString("content"),
                    ))
                }
            }
            // Subscribe to SSE for live streaming tokens
            net.openStream(
                sessionId = sid,
                onEvent = { ev ->
                    val kind = ev.optString("kind")
                    when (kind) {
                        "delta" -> {
                            val delta = ev.optString("delta", "")
                            streamingText = (streamingText ?: "") + delta
                            isStreaming = true
                        }
                        "done" -> {
                            streamingText = null
                            isStreaming = false
                        }
                        "error" -> {
                            error = ev.optString("error")
                            streamingText = null
                            isStreaming = false
                        }
                    }
                },
                onError = { e ->
                    error = e.message
                    isStreaming = false
                },
                onComplete = {
                    streamingText = null
                    isStreaming = false
                },
            )
        } catch (e: Exception) {
            error = e.message
        } finally {
            loading = false
        }
    }

    // Real-time sync loop: poll every 2s for new messages from VibeUI (Google Docs style)
    LaunchedEffect(activeSessionId) {
        val sid = activeSessionId ?: return@LaunchedEffect
        while (isActive) {
            delay(2_000)
            if (isStreaming) continue
            try {
                val resp = net.getMessages(sid)
                val arr = resp.optJSONArray("messages") ?: continue
                val updated = buildList<WearMessage> {
                    for (i in 0 until arr.length()) {
                        val m = arr.getJSONObject(i)
                        add(WearMessage(m.getLong("id"), m.getString("role"), m.getString("content")))
                    }
                }
                val localMax = messages.maxOfOrNull { it.id } ?: 0L
                val remoteMax = updated.maxOfOrNull { it.id } ?: 0L
                if (remoteMax > localMax) {
                    messages = updated
                }
            } catch (_: Exception) {}
        }
    }

    if (showVoice) {
        VoiceInputScreen(
            onDismiss = { showVoice = false },
            onSend = { text ->
                showVoice = false
                scope.launch {
                    try {
                        val resp = net.dispatch(text, activeSessionId)
                        if (activeSessionId == null) {
                            activeSessionId = resp.optString("session_id").takeIf { it.isNotEmpty() }
                        }
                        val sid = activeSessionId ?: resp.optString("session_id")
                        val msgId = resp.optLong("message_id", -1)
                        messages = messages + WearMessage(msgId, "user", text)
                        streamingText = ""
                        isStreaming = true

                        // Polling fallback: guarantees response even if SSE misses it
                        val allMessages = net.pollForResponse(sid)
                        if (allMessages.isNotEmpty()) {
                            messages = allMessages
                        }
                        streamingText = null
                        isStreaming = false
                    } catch (e: Exception) {
                        error = e.message
                        isStreaming = false
                    }
                }
            }
        )
        return
    }

    ScalingLazyColumn(
        state = listState,
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        reverseLayout = false,
    ) {
        item { ListHeader { Text(if (activeSessionId != null) "Chat" else "New Chat") } }

        if (loading) {
            item { CircularProgressIndicator() }
        }

        if (error != null) {
            item {
                Text(error!!, color = MaterialTheme.colors.error,
                    style = MaterialTheme.typography.caption2)
            }
        }

        items(messages.size) { i ->
            MessageBubble(messages[i])
        }

        if (streamingText != null) {
            item { StreamingBubble(streamingText ?: "") }
        }

        item {
            Button(
                onClick = { showVoice = true },
                enabled = !isStreaming,
                colors = ButtonDefaults.primaryButtonColors(),
                modifier = Modifier.padding(top = 4.dp),
            ) {
                Text(if (isStreaming) "…" else "🎤", textAlign = TextAlign.Center)
            }
        }
    }
}

@Composable
private fun MessageBubble(msg: WearMessage) {
    val isUser = msg.role == "user"
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp, vertical = 2.dp),
        contentAlignment = if (isUser) Alignment.CenterEnd else Alignment.CenterStart,
    ) {
        Text(
            msg.content,
            style = MaterialTheme.typography.caption1,
            // No maxLines / no Ellipsis — full text always shown, Digital Crown scrolls
            softWrap = true,
            modifier = Modifier
                .background(
                    color = if (isUser) MaterialTheme.colors.primary.copy(alpha = 0.2f)
                            else MaterialTheme.colors.surface,
                    shape = RoundedCornerShape(8.dp),
                )
                .padding(6.dp)
                .widthIn(max = 152.dp),
        )
    }
}

@Composable
private fun StreamingBubble(text: String) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp, vertical = 2.dp),
        contentAlignment = Alignment.CenterStart,
    ) {
        Text(
            if (text.isEmpty()) "…" else text,
            style = MaterialTheme.typography.caption1,
            color = MaterialTheme.colors.secondaryVariant,
            softWrap = true,
            modifier = Modifier
                .background(
                    color = MaterialTheme.colors.surface,
                    shape = RoundedCornerShape(8.dp),
                )
                .padding(6.dp)
                .widthIn(max = 152.dp),
        )
    }
}

data class WearMessage(val id: Long, val role: String, val content: String)
