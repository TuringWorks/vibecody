// SandboxStatusScreen.kt — Sandbox container status view for Wear OS.
// Shows CPU/RAM usage bars and pause/resume/stop controls.

package com.vibecody.wear

import androidx.compose.foundation.layout.*
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
fun SandboxStatusScreen(net: WearNetworkManager) {
    val scope = rememberCoroutineScope()
    var status by remember { mutableStateOf<SandboxInfo?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    // Poll every 5 seconds
    LaunchedEffect(Unit) {
        while (isActive) {
            try {
                // Sandbox status via existing /sandbox/status endpoint
                // (same daemon, routed through the bearer-auth path)
                val resp = net.listSessions()   // stub: replace with sandbox endpoint
                // Parse first running sandbox if available
                status = SandboxInfo(
                    containerId = resp.optString("container_id", "—"),
                    state = resp.optString("state", "unknown"),
                    cpuPct = resp.optDouble("cpu_pct", 0.0).toFloat(),
                    memMb = resp.optLong("mem_mb", 0),
                    memLimitMb = resp.optLong("mem_limit_mb", 512),
                    uptimeSecs = resp.optLong("uptime_secs", 0),
                )
            } catch (_: Exception) {}
            delay(5_000)
        }
    }

    ScalingLazyColumn(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        item { ListHeader { Text("Sandbox") } }

        val s = status
        if (s == null) {
            item { CircularProgressIndicator() }
        } else {
            item {
                Text(s.state.uppercase(), style = MaterialTheme.typography.title3,
                    color = if (s.state == "running") MaterialTheme.colors.primary
                            else MaterialTheme.colors.onSurfaceVariant)
            }
            item {
                Column(modifier = Modifier.padding(horizontal = 12.dp)) {
                    ResourceBar("CPU", s.cpuPct / 100f, "${s.cpuPct.toInt()}%")
                    Spacer(Modifier.height(4.dp))
                    ResourceBar("MEM", s.memMb.toFloat() / s.memLimitMb, "${s.memMb}/${s.memLimitMb} MB")
                }
            }
            item {
                Text("Up ${formatUptime(s.uptimeSecs)}", style = MaterialTheme.typography.caption2,
                    color = MaterialTheme.colors.onSurfaceVariant)
            }
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    if (s.state == "running") {
                        CompactChip(label = { Text("Pause") }, onClick = {
                            scope.launch { sendControl(net, "pause") }
                        })
                    } else if (s.state == "paused") {
                        CompactChip(label = { Text("Resume") }, onClick = {
                            scope.launch { sendControl(net, "resume") }
                        })
                    }
                    CompactChip(
                        label = { Text("Stop") },
                        onClick = { scope.launch { sendControl(net, "stop") } },
                        colors = ChipDefaults.chipColors(backgroundColor = MaterialTheme.colors.error),
                    )
                }
            }
        }

        if (error != null) {
            item {
                Text(error!!, color = MaterialTheme.colors.error,
                    style = MaterialTheme.typography.caption2, textAlign = TextAlign.Center)
            }
        }
    }
}

@Composable
private fun ResourceBar(label: String, fraction: Float, desc: String) {
    Column {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(label, style = MaterialTheme.typography.caption2)
            Text(desc, style = MaterialTheme.typography.caption2)
        }
        LinearProgressIndicator(
            progress = fraction.coerceIn(0f, 1f),
            modifier = Modifier.fillMaxWidth().height(4.dp),
        )
    }
}

private suspend fun sendControl(net: WearNetworkManager, action: String) {
    // POST to /sandbox/control — stub (binary-level endpoint)
    try {
        net.dispatch("__sandbox_$action", null)
    } catch (_: Exception) {}
}

private fun formatUptime(secs: Long): String {
    val h = secs / 3600
    val m = (secs % 3600) / 60
    val s = secs % 60
    return if (h > 0) "${h}h ${m}m" else "${m}m ${s}s"
}

data class SandboxInfo(
    val containerId: String,
    val state: String,
    val cpuPct: Float,
    val memMb: Long,
    val memLimitMb: Long,
    val uptimeSecs: Long,
)
