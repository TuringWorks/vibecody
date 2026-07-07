// SkillforgeScreen.kt — G5 Wear OS SkillForge catalogue list.
//
// Mirrors GoalsScreen but reads the curated `/watch/skilllens/skills`
// route (`{count, top5:[{name, category, summary}]}`). Tapping a row
// opens the read-only detail screen. The heavy score/train/promote
// mutations stay desktop-only (STRICT — Wear surfaces no toolbar LLM).

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
fun SkillforgeScreen(
    net: WearNetworkManager,
    onOpenSkill: (String) -> Unit,
) {
    var skills by remember { mutableStateOf<List<WearSkill>>(emptyList()) }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        try {
            val resp = net.skilllensSkills()
            val arr = resp.optJSONArray("top5")
            skills = buildList {
                if (arr != null) for (i in 0 until arr.length()) {
                    val s = arr.getJSONObject(i)
                    add(
                        WearSkill(
                            name = s.getString("name"),
                            category = s.optString("category", ""),
                            summary = s.optString("summary", ""),
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
        item { ListHeader { Text("Skills") } }
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
            skills.isEmpty() -> item {
                Text(
                    "No skills surfaced",
                    style = MaterialTheme.typography.caption2,
                    color = MaterialTheme.colors.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
            }
            else -> items(skills.size) { i ->
                val s = skills[i]
                Chip(
                    label = {
                        Text(
                            s.name,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                    },
                    secondaryLabel = {
                        Text(
                            if (s.category.isNotEmpty()) s.category else "skilllens",
                            color = MaterialTheme.colors.onSurfaceVariant,
                        )
                    },
                    onClick = { onOpenSkill(s.name) },
                    colors = ChipDefaults.secondaryChipColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

data class WearSkill(
    val name: String,
    val category: String,
    val summary: String,
)