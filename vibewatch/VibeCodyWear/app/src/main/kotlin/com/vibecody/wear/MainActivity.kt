// MainActivity.kt — Wear OS entry point for VibeCody.
//
// Shows the main tab navigation (Sessions / Sandbox / Settings) when paired,
// or the PairingScreen when no credentials are stored.

package com.vibecody.wear

import android.app.Activity
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.*
import androidx.wear.compose.navigation.SwipeDismissableNavHost
import androidx.wear.compose.navigation.composable
import androidx.wear.compose.navigation.rememberSwipeDismissableNavController

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            VibeCodyWearApp(this)
        }
    }
}

@Composable
fun VibeCodyWearApp(activity: Activity) {
    val auth = remember { WearAuthManager(activity) }
    val net = remember { WearNetworkManager(activity, auth) }
    val navController = rememberSwipeDismissableNavController()

    val isRegistered = remember { mutableStateOf(auth.isRegistered) }

    if (!isRegistered.value) {
        PairingScreen(auth = auth) {
            isRegistered.value = true
        }
        return
    }

    SwipeDismissableNavHost(navController = navController, startDestination = "sessions") {
        composable("sessions") {
            SessionListScreen(
                net = net,
                onOpenSession = { id -> navController.navigate("conversation/$id") },
                onNewSession = { navController.navigate("conversation/new") },
            )
        }
        composable("conversation/{sessionId}") { back ->
            val sessionId = back.arguments?.getString("sessionId")
                ?.takeIf { it != "new" }
            ConversationScreen(net = net, sessionId = sessionId)
        }
        composable("sandbox") {
            SandboxStatusScreen(net = net)
        }
        composable("settings") {
            SettingsScreen(auth = auth) {
                // After unpair, restart
                isRegistered.value = false
            }
        }
    }
}
