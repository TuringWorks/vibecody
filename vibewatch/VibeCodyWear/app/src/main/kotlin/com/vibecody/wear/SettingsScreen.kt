// SettingsScreen.kt — Settings and unpair for Wear OS.

package com.vibecody.wear

import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.wear.compose.material.*

@Composable
fun SettingsScreen(auth: WearAuthManager, onUnpaired: () -> Unit) {
    var showConfirm by remember { mutableStateOf(false) }

    if (showConfirm) {
        // Confirmation screen: full-screen instead of dialog for Wear OS compatibility
        ScalingLazyColumn(
            modifier = Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            item {
                Text(
                    "Unpair device?",
                    style = MaterialTheme.typography.title3,
                    textAlign = TextAlign.Center,
                )
            }
            item {
                Text(
                    "This removes all tokens and keys from this device.",
                    style = MaterialTheme.typography.caption2,
                    color = MaterialTheme.colors.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(horizontal = 8.dp),
                )
            }
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(
                        onClick = {
                            auth.clearRegistration()
                            onUnpaired()
                        },
                        colors = ButtonDefaults.primaryButtonColors(),
                    ) { Text("Unpair") }
                    Button(
                        onClick = { showConfirm = false },
                        colors = ButtonDefaults.secondaryButtonColors(),
                    ) { Text("Cancel") }
                }
            }
        }
        return
    }

    ScalingLazyColumn(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        item { ListHeader { Text("Settings") } }
        item {
            Text(
                "Daemon: ${auth.daemonUrl.ifEmpty { "—" }}",
                style = MaterialTheme.typography.caption2,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(horizontal = 8.dp),
            )
        }
        item {
            Chip(
                label = { Text("Unpair Device") },
                onClick = { showConfirm = true },
                colors = ChipDefaults.chipColors(backgroundColor = MaterialTheme.colors.error),
                modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp),
            )
        }
    }
}
