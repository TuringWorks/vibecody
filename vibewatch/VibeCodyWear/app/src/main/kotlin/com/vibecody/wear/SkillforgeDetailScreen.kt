// SkillforgeDetailScreen.kt — G5 Wear OS SkillForge skill detail.
//
// Read-only counterpart to GoalDetailScreen. Pulls the curated
// `/watch/skilllens/skills/:name` route (one-line `{name, category,
// summary}`). Score/train/promote stay desktop-only — the watch renders
// the skill summary and a "managed on desktop" hint, nothing more.

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
fun SkillforgeDetailScreen(
    net: WearNetworkManager,
    skillName: String,
) {
    var loading by remember { mutableStateOf(true) }
    var name by remember { mutableStateOf(skillName) }
    var category by remember { mutableStateOf("") }
    var summary by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(skillName) {
        try {
            val json = net.skilllensSkill(skillName)
            name = json.optString("name", skillName)
            category = json.optString("category", "")
            summary = json.optString("summary", "")
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
        item { ListHeader { Text("Skill") } }

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
                        name,
                        style = MaterialTheme.typography.body1,
                        textAlign = TextAlign.Center,
                        overflow = TextOverflow.Ellipsis,
                        maxLines = 3,
                    )
                }
                if (category.isNotEmpty()) {
                    item {
                        Chip(
                            label = { Text(category) },
                            onClick = { /* read-only */ },
                            enabled = false,
                            colors = ChipDefaults.secondaryChipColors(),
                            modifier = Modifier.padding(horizontal = 8.dp),
                        )
                    }
                }
                if (summary.isNotEmpty()) {
                    item { Spacer(modifier = Modifier.height(4.dp)) }
                    item {
                        Text(
                            summary,
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
                        "Score / train / promote stay on desktop.",
                        style = MaterialTheme.typography.caption2,
                        color = MaterialTheme.colors.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                    )
                }
            }
        }
    }
}