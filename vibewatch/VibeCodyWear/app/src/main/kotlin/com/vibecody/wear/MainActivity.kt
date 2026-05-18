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
        // W1.2 — `vibecody.deeplink=jobs` opens the Jobs tab on
        // launch. The Tile uses this when the user taps the recap
        // tile so they land on the job list instead of Sessions.
        val deeplink = intent?.getStringExtra("vibecody.deeplink")
        setContent {
            VibeCodyWearApp(this, startDestination = startDestinationFor(deeplink))
        }
    }
}

private fun startDestinationFor(deeplink: String?): String = when (deeplink) {
    "jobs" -> "jobs"
    "goals" -> "goals"
    else -> "sessions"
}

@Composable
fun VibeCodyWearApp(activity: Activity, startDestination: String = "sessions") {
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

    SwipeDismissableNavHost(navController = navController, startDestination = startDestination) {
        composable("sessions") {
            SessionListScreen(
                net = net,
                onOpenSession = { id -> navController.navigate("conversation/$id") },
                onNewSession = { navController.navigate("conversation/new") },
                onOpenRecap = { id, preview ->
                    val safe = java.net.URLEncoder.encode(preview, "UTF-8")
                    navController.navigate("recap/$id/$safe")
                },
            )
        }
        composable("conversation/{sessionId}") { back ->
            val sessionId = back.arguments?.getString("sessionId")
                ?.takeIf { it != "new" }
            ConversationScreen(net = net, sessionId = sessionId)
        }
        composable("recap/{sessionId}/{taskPreview}") { back ->
            val sid = back.arguments?.getString("sessionId").orEmpty()
            val preview = java.net.URLDecoder.decode(
                back.arguments?.getString("taskPreview").orEmpty(), "UTF-8"
            )
            RecapScreen(
                net = net,
                sessionId = sid,
                taskPreview = preview,
                onContinueOnPhone = { recap ->
                    // Hand off to phone via Wearable Data Layer.
                    WearDataLayerClient.handoffRecapToPhone(activity, recap)
                },
            )
        }
        // W1.2 — Jobs list + job-recap routes mirror the session pair.
        composable("jobs") {
            JobListScreen(
                net = net,
                onOpenRecap = { id, preview ->
                    val safe = java.net.URLEncoder.encode(preview, "UTF-8")
                    navController.navigate("job-recap/$id/$safe")
                },
            )
        }
        composable("job-recap/{jobId}/{taskPreview}") { back ->
            val sid = back.arguments?.getString("jobId").orEmpty()
            val preview = java.net.URLDecoder.decode(
                back.arguments?.getString("taskPreview").orEmpty(), "UTF-8"
            )
            RecapScreen(
                net = net,
                sessionId = sid,
                taskPreview = preview,
                kind = WearRecapKind.Job,
                onContinueOnPhone = { recap ->
                    WearDataLayerClient.handoffRecapToPhone(activity, recap)
                },
            )
        }
        composable("sandbox") {
            SandboxStatusScreen(
                net = net,
                onOpenSession = { id -> navController.navigate("conversation/$id") },
            )
        }
        // G2.3 — Goals list (display-only on Wear v1). Apple Watch has
        // a detail + start-session view; Wear matches its read-mostly
        // surface today and defers the start path to a follow-up. The
        // user always has VibeUI / mobile / CLI for actions.
        composable("goals") {
            GoalsScreen(
                net = net,
                onOpenGoal = { _id, _title ->
                    // No-op for v1 — the row tap is a placeholder for the
                    // future detail screen. Goals stay strictly read-only
                    // on Wear until there's a UX need.
                },
            )
        }
        composable("settings") {
            SettingsScreen(auth = auth) {
                // After unpair, restart
                isRegistered.value = false
            }
        }
    }
}
