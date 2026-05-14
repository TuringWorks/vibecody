// JobListScreen.kt — W1.2 Wear OS background-jobs list.
//
// Mirrors SessionListScreen but reads /watch/jobs. Tapping a row
// opens RecapScreen in kind=job mode. Reachable from the Tile and
// from the in-app navigation.

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
fun JobListScreen(
    net: WearNetworkManager,
    onOpenRecap: (String, String) -> Unit,
) {
    var jobs by remember { mutableStateOf<List<WearJob>>(emptyList()) }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        try {
            val resp = net.listJobs()
            val arr = resp.optJSONArray("jobs")
            jobs = buildList {
                if (arr != null) for (i in 0 until arr.length()) {
                    val s = arr.getJSONObject(i)
                    add(
                        WearJob(
                            id = s.getString("session_id"),
                            preview = s.optString("task_preview", "—").take(60),
                            status = s.optString("status", "unknown"),
                            startedAt = s.optLong("started_at", 0),
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
        item { ListHeader { Text("Jobs") } }
        if (loading) {
            item { CircularProgressIndicator() }
        } else if (error != null) {
            item {
                Text(
                    error!!,
                    color = MaterialTheme.colors.error,
                    style = MaterialTheme.typography.caption2,
                    textAlign = TextAlign.Center,
                )
            }
        } else if (jobs.isEmpty()) {
            item {
                Text(
                    "No jobs yet",
                    style = MaterialTheme.typography.caption2,
                    color = MaterialTheme.colors.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
            }
        } else {
            items(jobs.size) { i ->
                val j = jobs[i]
                Chip(
                    label = {
                        Text(j.preview, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    },
                    secondaryLabel = {
                        Text(
                            j.status,
                            color = if (j.status == "running") MaterialTheme.colors.primary
                                    else MaterialTheme.colors.onSurfaceVariant,
                        )
                    },
                    onClick = { onOpenRecap(j.id, j.preview) },
                    colors = ChipDefaults.secondaryChipColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

data class WearJob(
    val id: String,
    val preview: String,
    val status: String,
    val startedAt: Long,
)
