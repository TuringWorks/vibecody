// GoalsTileService.kt — G3.6 Wear OS Tile for the freshest active goal.
//
// Counterpart to JobRecapTileService: instead of "last job that ran"
// it shows "what we're working toward right now". Read-only, hits
// only `/watch/goals` (curated). Tap → MainActivity → goals route.

package com.vibecody.wear

import android.content.Intent
import android.util.Log
import androidx.wear.protolayout.ActionBuilders
import androidx.wear.protolayout.ColorBuilders.argb
import androidx.wear.protolayout.DimensionBuilders.dp
import androidx.wear.protolayout.LayoutElementBuilders
import androidx.wear.protolayout.ModifiersBuilders
import androidx.wear.protolayout.ResourceBuilders
import androidx.wear.protolayout.TimelineBuilders
import androidx.concurrent.futures.CallbackToFutureAdapter
import androidx.wear.tiles.RequestBuilders
import androidx.wear.tiles.TileBuilders
import androidx.wear.tiles.TileService
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

private const val GOAL_TAG = "GoalsTile"
private const val GOAL_RES_VERSION = "1"

class GoalsTileService : TileService() {

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    override fun onTileResourcesRequest(
        requestParams: RequestBuilders.ResourcesRequest
    ): ListenableFuture<ResourceBuilders.Resources> {
        return Futures.immediateFuture(
            ResourceBuilders.Resources.Builder()
                .setVersion(GOAL_RES_VERSION)
                .build()
        )
    }

    override fun onTileRequest(
        requestParams: RequestBuilders.TileRequest
    ): ListenableFuture<TileBuilders.Tile> {
        return CallbackToFutureAdapter.getFuture { completer ->
            scope.launch {
                try {
                    val state = fetchFreshestGoal()
                    completer.set(buildTile(state))
                } catch (t: Throwable) {
                    completer.set(buildTile(GoalTileState.Empty))
                }
            }
            "onTileRequest"
        }
    }

    private fun buildTile(state: GoalTileState): TileBuilders.Tile =
        TileBuilders.Tile.Builder()
            .setResourcesVersion(GOAL_RES_VERSION)
            .setTileTimeline(
                TimelineBuilders.Timeline.Builder()
                    .addTimelineEntry(
                        TimelineBuilders.TimelineEntry.Builder()
                            .setLayout(
                                LayoutElementBuilders.Layout.Builder()
                                    .setRoot(layoutFor(state))
                                    .build()
                            )
                            .build()
                    )
                    .build()
            )
            .build()

    /**
     * Fetch the freshest active goal via the curated `/watch/goals`
     * endpoint. Returns the head of the list — the daemon sorts
     * newest-updated-first and caps at 25.
     */
    private suspend fun fetchFreshestGoal(): GoalTileState {
        val auth = WearAuthManager(applicationContext)
        if (!auth.isRegistered) return GoalTileState.NotPaired
        val net = WearNetworkManager(applicationContext, auth)
        return try {
            val resp = net.listGoals()
            val arr = resp.optJSONArray("goals") ?: return GoalTileState.Empty
            if (arr.length() == 0) return GoalTileState.Empty
            val g = arr.getJSONObject(0)
            GoalTileState.Ready(
                title = g.optString("title", "(untitled)").take(80),
                workspaceLabel = g.optString("workspace_label", "global"),
                status = g.optString("status", "active"),
            )
        } catch (e: Exception) {
            Log.w(GOAL_TAG, "fetchFreshestGoal failed: ${e.message}")
            GoalTileState.Empty
        }
    }

    private fun layoutFor(state: GoalTileState): LayoutElementBuilders.LayoutElement {
        val (title, body) = when (state) {
            is GoalTileState.Ready ->
                "Goal · ${state.workspaceLabel}" to state.title
            GoalTileState.Empty -> "VibeCody" to "No active goals"
            GoalTileState.NotPaired -> "VibeCody" to "Pair on the phone"
        }
        return LayoutElementBuilders.Box.Builder()
            .setWidth(androidx.wear.protolayout.DimensionBuilders.expand())
            .setHeight(androidx.wear.protolayout.DimensionBuilders.expand())
            .setModifiers(
                ModifiersBuilders.Modifiers.Builder()
                    .setClickable(openGoalsClickable())
                    .build()
            )
            .addContent(
                LayoutElementBuilders.Column.Builder()
                    .setHorizontalAlignment(LayoutElementBuilders.HORIZONTAL_ALIGN_CENTER)
                    .addContent(
                        LayoutElementBuilders.Text.Builder()
                            .setText(title)
                            .setFontStyle(
                                LayoutElementBuilders.FontStyle.Builder()
                                    .setSize(sp(11f))
                                    .setColor(argb(0xFF888888.toInt()))
                                    .build()
                            )
                            .build()
                    )
                    .addContent(
                        LayoutElementBuilders.Spacer.Builder()
                            .setHeight(dp(2f))
                            .build()
                    )
                    .addContent(
                        LayoutElementBuilders.Text.Builder()
                            .setText(body)
                            .setMaxLines(3)
                            .setFontStyle(
                                LayoutElementBuilders.FontStyle.Builder()
                                    .setSize(sp(13f))
                                    .setWeight(LayoutElementBuilders.FONT_WEIGHT_MEDIUM)
                                    .build()
                            )
                            .build()
                    )
                    .build()
            )
            .build()
    }

    private fun openGoalsClickable(): ModifiersBuilders.Clickable {
        val intent = Intent(applicationContext, MainActivity::class.java).apply {
            putExtra("vibecody.deeplink", "goals")
            flags = Intent.FLAG_ACTIVITY_NEW_TASK
        }
        return ModifiersBuilders.Clickable.Builder()
            .setId("open_goals")
            .setOnClick(
                ActionBuilders.LaunchAction.Builder()
                    .setAndroidActivity(
                        ActionBuilders.AndroidActivity.Builder()
                            .setPackageName(intent.`package` ?: applicationContext.packageName)
                            .setClassName(MainActivity::class.java.name)
                            .build()
                    )
                    .build()
            )
            .build()
    }

    private fun sp(value: Float) =
        androidx.wear.protolayout.DimensionBuilders.sp(value)

    sealed class GoalTileState {
        data class Ready(
            val title: String,
            val workspaceLabel: String,
            val status: String,
        ) : GoalTileState()
        object Empty : GoalTileState()
        object NotPaired : GoalTileState()
    }
}
