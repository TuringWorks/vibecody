// ConversationScreen.kt — Chat conversation view for Wear OS.
//
// Displays messages in a ScalingLazyColumn (auto-scrolls to newest).
// Voice input is launched via VoiceInputScreen.
// SSE streaming shows a live typing indicator while the assistant responds.

package com.vibecody.wear

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.wear.compose.material.*
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
    var activeSessionId by remember { mutableStateOf(sessionId) }
    var loading by remember { mutableStateOf(sessionId != null) }
    var showVoice by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val listState = rememberScalingLazyListState()

    // Load existing messages
    LaunchedEffect(activeSessionId) {
        val sid = activeSessionId ?: return@LaunchedEffect
        loading = true
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
            // Subscribe to SSE for live updates
            net.openStream(
                sessionId = sid,
                onEvent = { ev ->
                    val kind = ev.optString("kind")
                    when (kind) {
                        "delta" -> {
                            val delta = ev.optString("delta", "")
                            streamingText = (streamingText ?: "") + delta
                        }
                        "done" -> {
                            val final = streamingText
                            if (final != null) {
                                messages = messages + WearMessage(-1, "assistant", final)
                                streamingText = null
                            }
                        }
                        "error" -> {
                            error = ev.optString("error")
                            streamingText = null
                        }
                    }
                },
                onError = { e -> error = e.message },
                onComplete = { streamingText = null },
            )
        } catch (e: Exception) {
            error = e.message
        } finally {
            loading = false
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
                            activeSessionId = resp.optString("session_id")
                        }
                        val msgId = resp.optLong("message_id", -1)
                        messages = messages + WearMessage(msgId, "user", text)
                        streamingText = ""  // streaming starts
                    } catch (e: Exception) {
                        error = e.message
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
            item {
                StreamingBubble(streamingText ?: "")
            }
        }

        item {
            Button(
                onClick = { showVoice = true },
                colors = ButtonDefaults.primaryButtonColors(),
                modifier = Modifier.padding(top = 4.dp),
            ) {
                Text("🎤", textAlign = TextAlign.Center)
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
            maxLines = 6,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier
                .background(
                    color = if (isUser) MaterialTheme.colors.primary.copy(alpha = 0.2f)
                            else MaterialTheme.colors.surface,
                    shape = RoundedCornerShape(8.dp),
                )
                .padding(6.dp)
                .widthIn(max = 140.dp),
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
            maxLines = 6,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier
                .background(
                    color = MaterialTheme.colors.surface,
                    shape = RoundedCornerShape(8.dp),
                )
                .padding(6.dp)
                .widthIn(max = 140.dp),
        )
    }
}

data class WearMessage(val id: Long, val role: String, val content: String)
