// GoalDetailScreen.kt — G3.6 Wear OS goal detail (G5.1 added Start).
//
// Mirrors RecapScreen's read-only ScalingLazyColumn pattern. Pulls
// `/watch/goals/:id` (curated route — watch never hits `/v1/*` direct)
// and renders title + status + statement + linked-session count.
// G5.1 added a `Start session` chip that POSTs to the curated
// `/watch/goals/:id/start` route. Other mutations (status flip, link,
// plan, reparent) still happen via the daemon REPL or VibeUI.

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
fun GoalDetailScreen(
    net: WearNetworkManager,
    goalId: String,
) {
    var loading by remember { mutableStateOf(true) }
    var title by remember { mutableStateOf("") }
    var status by remember { mutableStateOf("") }
    var statement by remember { mutableStateOf("") }
    var linkCount by remember { mutableStateOf(0) }
    // G12.1 — envelope-level `pinned` flag from `/watch/goals/:id`.
    var pinned by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var starting by remember { mutableStateOf(false) }
    var startResult by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(goalId) {
        val json = net.getGoal(goalId)
        if (json == null) {
            error = "Goal vanished or daemon unreachable"
            loading = false
            return@LaunchedEffect
        }
        val goal = json.optJSONObject("goal")
        if (goal == null) {
            error = "Daemon returned empty goal envelope"
            loading = false
            return@LaunchedEffect
        }
        title = goal.optString("title", "(untitled)")
        status = goal.optString("status", "active")
        statement = goal.optString("statement", "")
        linkCount = json.optJSONArray("links")?.length() ?: 0
        // G12.1 — envelope-level flag, absent on pre-G12.1 daemons.
        pinned = json.optBoolean("pinned", false)
        loading = false
    }

    ScalingLazyColumn(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        item { ListHeader { Text("Goal") } }

        when {
            loading -> item { CircularProgressIndicator() }
            error != null -> item {
                Text(
                    error!!,
                    color = MaterialTheme.colors.error,
                    style = MaterialTheme.typography.caption2,
                    textAlign = TextAlign.Center,
                )
            }
            else -> {
                item {
                    // G12.1 — prefix the title with ★ when the daemon
                    // marked this goal as the current pin. Same glyph
                    // the goal list uses for cross-surface consistency.
                    Text(
                        if (pinned) "★ $title" else title,
                        style = MaterialTheme.typography.body1,
                        textAlign = TextAlign.Center,
                        overflow = TextOverflow.Ellipsis,
                        maxLines = 3,
                    )
                }
                item {
                    Chip(
                        label = { Text(status) },
                        onClick = { /* read-only */ },
                        enabled = false,
                        colors = ChipDefaults.secondaryChipColors(),
                        modifier = Modifier.padding(horizontal = 8.dp),
                    )
                }
                if (statement.isNotBlank()) {
                    item { Spacer(modifier = Modifier.height(4.dp)) }
                    item {
                        Text(
                            statement,
                            style = MaterialTheme.typography.caption1,
                            color = MaterialTheme.colors.onSurface,
                            textAlign = TextAlign.Start,
                            overflow = TextOverflow.Ellipsis,
                            maxLines = 6,
                            modifier = Modifier.padding(horizontal = 8.dp),
                        )
                    }
                }
                item { Spacer(modifier = Modifier.height(4.dp)) }
                item {
                    Text(
                        "$linkCount linked",
                        style = MaterialTheme.typography.caption2,
                        color = MaterialTheme.colors.onSurfaceVariant,
                    )
                }
                item { Spacer(modifier = Modifier.height(4.dp)) }
                item {
                    Chip(
                        label = {
                            Text(
                                if (starting) "Starting…" else "Start session",
                                style = MaterialTheme.typography.button,
                            )
                        },
                        onClick = {
                            if (!starting) {
                                starting = true
                                scope.launch {
                                    val sid = net.startGoal(goalId)
                                    starting = false
                                    startResult = if (sid != null) {
                                        "Started ${sid.take(8)}"
                                    } else {
                                        "Start failed"
                                    }
                                }
                            }
                        },
                        enabled = !starting,
                        colors = ChipDefaults.primaryChipColors(),
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 8.dp),
                    )
                }
                if (startResult != null) {
                    item {
                        Text(
                            startResult!!,
                            style = MaterialTheme.typography.caption2,
                            color = MaterialTheme.colors.onSurfaceVariant,
                            textAlign = TextAlign.Center,
                        )
                    }
                }
            }
        }
    }
}
