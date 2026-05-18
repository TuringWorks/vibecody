// GoalsScreen.kt — G2.3 Wear OS goals list.
//
// Mirrors JobListScreen but reads /watch/goals. Tapping a row opens
// the detail screen; the detail screen calls /watch/dispatch to start
// a session bound to the goal (same path as Apple Watch's GoalsView).

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
fun GoalsScreen(
    net: WearNetworkManager,
    onOpenGoal: (String, String) -> Unit,
) {
    var goals by remember { mutableStateOf<List<WearGoal>>(emptyList()) }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        try {
            val resp = net.listGoals()
            val arr = resp.optJSONArray("goals")
            goals = buildList {
                if (arr != null) for (i in 0 until arr.length()) {
                    val g = arr.getJSONObject(i)
                    add(
                        WearGoal(
                            id = g.getString("id"),
                            title = g.optString("title", "(untitled)").take(80),
                            status = g.optString("status", "active"),
                            workspaceLabel = g.optString("workspace_label", "global"),
                        )
                    )
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
        item { ListHeader { Text("Goals") } }
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
            goals.isEmpty() -> item {
                Text(
                    "No active goals",
                    style = MaterialTheme.typography.caption2,
                    color = MaterialTheme.colors.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
            }
            else -> items(goals.size) { i ->
                val g = goals[i]
                Chip(
                    label = {
                        Text(g.title, maxLines = 2, overflow = TextOverflow.Ellipsis)
                    },
                    secondaryLabel = {
                        Text(
                            "${g.workspaceLabel} · ${g.status}",
                            color = if (g.status == "active") MaterialTheme.colors.primary
                                    else MaterialTheme.colors.onSurfaceVariant,
                        )
                    },
                    onClick = { onOpenGoal(g.id, g.title) },
                    colors = ChipDefaults.secondaryChipColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

data class WearGoal(
    val id: String,
    val title: String,
    val status: String,
    val workspaceLabel: String,
)
