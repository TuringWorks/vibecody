// RecapScreen.kt — W1.1 read-only recap on Wear OS.
//
// Reachable via long-press on a session row in SessionListScreen.
// Mirrors the SwiftUI `RecapView`: headline, generator badge, bullets,
// next actions, artifacts, and a "Continue on phone" button that
// hands off to the paired Android phone via the Wearable Data Layer.

package com.vibecody.wear

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.wear.compose.material.*
import org.json.JSONObject

// ── Wire shape ────────────────────────────────────────────────────────────

data class WearRecapArtifact(
    val kind: String,
    val label: String,
    val locator: String,
)

data class WearRecapGenerator(
    val type: String,
    val provider: String? = null,
    val model: String? = null,
) {
    fun label(): String = when (type) {
        "llm" -> "LLM · ${provider ?: "?"}/${model ?: "?"}"
        "user_edited" -> "user-edited"
        else -> "heuristic"
    }
}

data class WearRecap(
    val id: String,
    val kind: String,
    val subjectId: String,
    val headline: String,
    val bullets: List<String>,
    val nextActions: List<String>,
    val artifacts: List<WearRecapArtifact>,
    val generator: WearRecapGenerator,
    val schemaVersion: Int,
) {
    companion object {
        fun fromJson(j: JSONObject): WearRecap {
            val bullets = j.optJSONArray("bullets")?.let { arr ->
                List(arr.length()) { i -> arr.getString(i) }
            } ?: emptyList()
            val nextActions = j.optJSONArray("next_actions")?.let { arr ->
                List(arr.length()) { i -> arr.getString(i) }
            } ?: emptyList()
            val artifacts = j.optJSONArray("artifacts")?.let { arr ->
                List(arr.length()) { i ->
                    val a = arr.getJSONObject(i)
                    WearRecapArtifact(
                        kind = a.optString("kind", ""),
                        label = a.optString("label", ""),
                        locator = a.optString("locator", ""),
                    )
                }
            } ?: emptyList()
            val gen = j.optJSONObject("generator") ?: JSONObject(mapOf("type" to "heuristic"))
            return WearRecap(
                id = j.optString("id", ""),
                kind = j.optString("kind", "session"),
                subjectId = j.optString("subject_id", ""),
                headline = j.optString("headline", ""),
                bullets = bullets,
                nextActions = nextActions,
                artifacts = artifacts,
                generator = WearRecapGenerator(
                    type = gen.optString("type", "heuristic"),
                    provider = gen.optString("provider", null).takeIf { !it.isNullOrEmpty() },
                    model = gen.optString("model", null).takeIf { !it.isNullOrEmpty() },
                ),
                schemaVersion = j.optInt("schema_version", 1),
            )
        }
    }
}

// ── Screen ────────────────────────────────────────────────────────────────

@Composable
fun RecapScreen(
    net: WearNetworkManager,
    sessionId: String,
    taskPreview: String,
    onContinueOnPhone: (WearRecap) -> Unit,
) {
    var recap by remember { mutableStateOf<WearRecap?>(null) }
    var loading by remember { mutableStateOf(true) }

    LaunchedEffect(sessionId) {
        recap = net.getSessionRecap(sessionId)
        loading = false
    }

    ScalingLazyColumn(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.Start,
    ) {
        item { ListHeader { Text("Recap") } }
        when {
            loading -> item { CircularProgressIndicator() }
            recap == null -> item {
                EmptyRecap(taskPreview)
            }
            else -> renderRecap(recap!!, onContinueOnPhone)
        }
    }
}

private fun androidx.wear.compose.material.ScalingLazyListScope.renderRecap(
    r: WearRecap,
    onContinueOnPhone: (WearRecap) -> Unit,
) {
    item {
        Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(
                r.headline,
                fontWeight = FontWeight.SemiBold,
                style = MaterialTheme.typography.body2,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            GeneratorBadge(r.generator)
        }
    }
    if (r.bullets.isNotEmpty()) {
        item { SectionLabel("WHAT") }
        items(r.bullets.size) { i -> BulletRow(r.bullets[i]) }
    }
    if (r.nextActions.isNotEmpty()) {
        item { SectionLabel("NEXT") }
        items(r.nextActions.size) { i -> BulletRow(r.nextActions[i]) }
    }
    if (r.artifacts.isNotEmpty()) {
        item { SectionLabel("ARTIFACTS") }
        items(r.artifacts.size) { i -> ArtifactRow(r.artifacts[i]) }
    }
    item {
        Chip(
            label = { Text("Continue on phone", style = MaterialTheme.typography.caption2) },
            onClick = { onContinueOnPhone(r) },
            colors = ChipDefaults.primaryChipColors(),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun GeneratorBadge(g: WearRecapGenerator) {
    val tint = when (g.type) {
        "llm" -> MaterialTheme.colors.primary.copy(alpha = 0.25f)
        "user_edited" -> Color(0xFFFFB74D).copy(alpha = 0.25f)
        else -> MaterialTheme.colors.surface
    }
    Text(
        g.label(),
        fontSize = 9.sp,
        modifier = Modifier
            .padding(top = 1.dp)
            .background(tint, androidx.compose.foundation.shape.RoundedCornerShape(8.dp))
            .padding(horizontal = 5.dp, vertical = 1.dp),
    )
}

@Composable
private fun SectionLabel(text: String) {
    Text(
        text,
        fontSize = 10.sp,
        fontWeight = FontWeight.SemiBold,
        color = MaterialTheme.colors.onSurfaceVariant,
        modifier = Modifier.padding(top = 4.dp, start = 4.dp),
    )
}

@Composable
private fun BulletRow(text: String) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text("•", style = MaterialTheme.typography.caption2,
            color = MaterialTheme.colors.onSurfaceVariant)
        Text(text, style = MaterialTheme.typography.caption2,
            maxLines = 3, overflow = TextOverflow.Ellipsis)
    }
}

@Composable
private fun ArtifactRow(a: WearRecapArtifact) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(iconFor(a.kind), fontSize = 9.sp)
        Text(a.label, style = MaterialTheme.typography.caption2)
        Text(a.locator,
            fontSize = 9.sp,
            color = MaterialTheme.colors.onSurfaceVariant,
            maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

@Composable
private fun EmptyRecap(taskPreview: String) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(top = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text("No recap yet",
            style = MaterialTheme.typography.caption1,
            color = MaterialTheme.colors.onSurfaceVariant)
        Text(taskPreview,
            fontSize = 9.sp,
            color = MaterialTheme.colors.onSurfaceVariant,
            textAlign = TextAlign.Center,
            maxLines = 2, overflow = TextOverflow.Ellipsis)
    }
}

private fun iconFor(kind: String): String = when (kind) {
    "file" -> "📄"
    "diff" -> "↔"
    "job"  -> "💼"
    "url"  -> "🔗"
    else   -> "•"
}

// Compose preview can't construct a real WearNetworkManager (needs
// Context + WearAuthManager), so this preview renders the recap body
// directly from a literal WearRecap.
@Preview
@Composable
private fun RecapScreenPreview() {
    val r = WearRecap(
        id = "rcp_abc",
        kind = "session",
        subjectId = "sess_xyz",
        headline = "Wired auth refresh-token rotation",
        bullets = listOf("Ran cargo test (3x)", "Edited src/auth.rs"),
        nextActions = listOf("Wire refresh token to frontend"),
        artifacts = listOf(WearRecapArtifact("file", "auth.rs", "src/auth.rs")),
        generator = WearRecapGenerator("heuristic"),
        schemaVersion = 1,
    )
    ScalingLazyColumn {
        item { ListHeader { Text("Recap") } }
        renderRecap(r) { /* preview */ }
    }
}
