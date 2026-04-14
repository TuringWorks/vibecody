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
        Dialog(
            showDialog = true,
            onDismissRequest = { showConfirm = false },
        ) {
            Alert(
                title = { Text("Unpair?", textAlign = TextAlign.Center) },
                message = { Text("This removes all tokens and keys from this device.", textAlign = TextAlign.Center) },
                negativeButton = {
                    Button(onClick = { showConfirm = false }, colors = ButtonDefaults.secondaryButtonColors()) {
                        Text("Cancel")
                    }
                },
                positiveButton = {
                    Button(onClick = {
                        auth.clearRegistration()
                        onUnpaired()
                    }, colors = ButtonDefaults.primaryButtonColors()) {
                        Text("Unpair")
                    }
                },
            )
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
