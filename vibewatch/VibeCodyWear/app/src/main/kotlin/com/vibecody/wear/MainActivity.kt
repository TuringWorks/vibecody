// MainActivity.kt — Wear OS entry point for VibeCody.
//
// Shows the main tab navigation (Sessions / Sandbox / Settings) when paired,
// or the PairingScreen when no credentials are stored.

package com.vibecody.wear

import android.app.Activity
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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
    "skills" -> "skills"
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

    // DREAD #1 Slice G part 3 — tainted-confirmation overlay sits on
    // top of the nav host so every screen surfaces a pending prompt
    // when one is queued. Renders nothing when idle (zero overhead).
    Box(modifier = Modifier.fillMaxSize()) {
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
        // G2.3 — Goals list. G3.6 added a detail screen behind a tap;
        // mutations still happen via VibeUI / mobile / CLI.
        composable("goals") {
            GoalsScreen(
                net = net,
                onOpenGoal = { id, _title ->
                    val safe = java.net.URLEncoder.encode(id, "UTF-8")
                    navController.navigate("goal-detail/$safe")
                },
            )
        }
        composable("goal-detail/{goalId}") { back ->
            val goalId = java.net.URLDecoder.decode(
                back.arguments?.getString("goalId").orEmpty(), "UTF-8"
            )
            GoalDetailScreen(net = net, goalId = goalId)
        }
        // G5 — SkillForge catalogue + detail. Read-only browse; the heavy
        // score/train/promote mutations stay desktop-only (STRICT).
        composable("skills") {
            SkillforgeScreen(
                net = net,
                onOpenSkill = { name ->
                    val safe = java.net.URLEncoder.encode(name, "UTF-8")
                    navController.navigate("skill-detail/$safe")
                },
            )
        }
        composable("skill-detail/{skillName}") { back ->
            val skillName = java.net.URLDecoder.decode(
                back.arguments?.getString("skillName").orEmpty(), "UTF-8"
            )
            SkillforgeDetailScreen(net = net, skillName = skillName)
        }
        composable("settings") {
            SettingsScreen(auth = auth) {
                // After unpair, restart
                isRegistered.value = false
            }
        }
    }
        Box(modifier = Modifier.align(Alignment.BottomCenter)) {
            TaintedConfirmationOverlay(net = net)
        }
    }
}
