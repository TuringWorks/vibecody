// GoalDetailScreen.kt — G3.6 Wear OS goal detail.
//
// Mirrors RecapScreen's read-only ScalingLazyColumn pattern. Pulls
// `/watch/goals/:id` (curated route — watch never hits `/v1/*` direct)
// and renders title + status + statement + linked-session count.
// Action surface stays minimal: tile/list → detail → done. Mutations
// (status flip, link, plan, start) all happen via the daemon REPL,
// VibeUI, or the Apple Watch detail view.

package com.vibecody.wear

import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.wear.compose.material.*

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
    var error by remember { mutableStateOf<String?>(null) }

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
                    Text(
                        title,
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
            }
        }
    }
}
