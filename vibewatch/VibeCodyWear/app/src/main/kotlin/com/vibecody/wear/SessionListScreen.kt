// SessionListScreen.kt — Scrollable session list for Wear OS.

package com.vibecody.wear

import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.wear.compose.material.*
import kotlinx.coroutines.launch

@Composable
fun SessionListScreen(
    net: WearNetworkManager,
    onOpenSession: (String) -> Unit,
    onNewSession: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    var sessions by remember { mutableStateOf<List<WearSession>>(emptyList()) }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        try {
            val resp = net.listSessions()
            val arr = resp.optJSONArray("sessions")
            sessions = buildList {
                if (arr != null) for (i in 0 until arr.length()) {
                    val s = arr.getJSONObject(i)
                    add(WearSession(
                        id = s.getString("session_id"),
                        preview = s.optString("task_preview", "—").take(60),
                        status = s.optString("status", "unknown"),
                        lastActivity = s.optLong("last_activity", 0),
                    ))
                }
            }
        } catch (e: Exception) {
            error = e.message
        } finally {
            loading = false
        }
    }

    ScalingLazyColumn(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        item {
            ListHeader { Text("Sessions") }
        }
        if (loading) {
            item { CircularProgressIndicator() }
        } else if (error != null) {
            item {
                Text(error!!, color = MaterialTheme.colors.error,
                    style = MaterialTheme.typography.caption2, textAlign = TextAlign.Center)
            }
        } else {
            item {
                Chip(
                    label = { Text("New Session") },
                    onClick = onNewSession,
                    colors = ChipDefaults.primaryChipColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            items(sessions.size) { i ->
                val s = sessions[i]
                Chip(
                    label = {
                        Text(s.preview, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    },
                    secondaryLabel = {
                        Text(
                            s.status,
                            color = if (s.status == "running") MaterialTheme.colors.primary
                                    else MaterialTheme.colors.onSurfaceVariant,
                        )
                    },
                    onClick = { onOpenSession(s.id) },
                    colors = ChipDefaults.secondaryChipColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

data class WearSession(
    val id: String,
    val preview: String,
    val status: String,
    val lastActivity: Long,
)
