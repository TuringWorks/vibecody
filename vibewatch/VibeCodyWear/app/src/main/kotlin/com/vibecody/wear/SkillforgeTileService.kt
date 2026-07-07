// SkillforgeTileService.kt — G5 Wear OS Tile for the SkillForge catalogue.
//
// Counterpart to GoalsTileService: instead of "the freshest active goal"
// it shows "the top skill in the catalogue" (head of /watch/skilllens
// `top5`). Read-only, hits only the curated `/watch/skilllens/skills`
// route. Tap → MainActivity → skills route.

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

private const val SKILL_TAG = "SkillforgeTile"
private const val SKILL_RES_VERSION = "1"

class SkillforgeTileService : TileService() {

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    override fun onTileResourcesRequest(
        requestParams: RequestBuilders.ResourcesRequest
    ): ListenableFuture<ResourceBuilders.Resources> {
        return Futures.immediateFuture(
            ResourceBuilders.Resources.Builder()
                .setVersion(SKILL_RES_VERSION)
                .build()
        )
    }

    override fun onTileRequest(
        requestParams: RequestBuilders.TileRequest
    ): ListenableFuture<TileBuilders.Tile> {
        return CallbackToFutureAdapter.getFuture { completer ->
            scope.launch {
                try {
                    val state = fetchTopSkill()
                    completer.set(buildTile(state))
                } catch (t: Throwable) {
                    completer.set(buildTile(SkillTileState.Empty))
                }
            }
            "onTileRequest"
        }
    }

    private fun buildTile(state: SkillTileState): TileBuilders.Tile =
        TileBuilders.Tile.Builder()
            .setResourcesVersion(SKILL_RES_VERSION)
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
     * Fetch the head of the curated `/watch/skilllens/skills` `top5`
     * array — the daemon picks the five skills worth surfacing on a
     * wrist; we show the first.
     */
    private suspend fun fetchTopSkill(): SkillTileState {
        val auth = WearAuthManager(applicationContext)
        if (!auth.isRegistered) return SkillTileState.NotPaired
        val net = WearNetworkManager(applicationContext, auth)
        return try {
            val resp = net.skilllensSkills()
            val arr = resp.optJSONArray("top5") ?: return SkillTileState.Empty
            if (arr.length() == 0) return SkillTileState.Empty
            val picked = arr.getJSONObject(0)
            SkillTileState.Ready(
                name = picked.optString("name", "(unnamed)").take(80),
                category = picked.optString("category", ""),
            )
        } catch (e: Exception) {
            Log.w(SKILL_TAG, "fetchTopSkill failed: ${e.message}")
            SkillTileState.Empty
        }
    }

    private fun layoutFor(state: SkillTileState): LayoutElementBuilders.LayoutElement {
        val (title, body) = when (state) {
            is SkillTileState.Ready ->
                "SkillForge · ${state.category.ifEmpty { "skilllens" }}" to state.name
            SkillTileState.Empty -> "VibeCody" to "No skills surfaced"
            SkillTileState.NotPaired -> "VibeCody" to "Pair on the phone"
        }
        return LayoutElementBuilders.Box.Builder()
            .setWidth(androidx.wear.protolayout.DimensionBuilders.expand())
            .setHeight(androidx.wear.protolayout.DimensionBuilders.expand())
            .setModifiers(
                ModifiersBuilders.Modifiers.Builder()
                    .setClickable(openSkillsClickable())
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

    private fun openSkillsClickable(): ModifiersBuilders.Clickable {
        val intent = Intent(applicationContext, MainActivity::class.java).apply {
            putExtra("vibecody.deeplink", "skills")
            flags = Intent.FLAG_ACTIVITY_NEW_TASK
        }
        return ModifiersBuilders.Clickable.Builder()
            .setId("open_skills")
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

    sealed class SkillTileState {
        data class Ready(
            val name: String,
            val category: String,
        ) : SkillTileState()
        object Empty : SkillTileState()
        object NotPaired : SkillTileState()
    }
}