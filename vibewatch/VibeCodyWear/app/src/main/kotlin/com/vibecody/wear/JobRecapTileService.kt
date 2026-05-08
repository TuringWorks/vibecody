// JobRecapTileService.kt — W1.2 Wear OS Tile.
//
// Surfaces the freshest terminal-state job recap as a tile in the
// user's carousel. Counterpart to the watchOS WidgetKit complication.
// Read-only: the watch never composes a recap, only displays what
// the daemon's J1.2 hook has stored (see docs/design/recap-resume/02-job.md).
//
// Patent / privacy: this Tile only fetches `/watch/jobs/:id/recap`,
// never `/v1/recap` directly. Tokens stay on-device.

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
import org.json.JSONObject

private const val TAG = "JobRecapTile"
private const val RESOURCES_VERSION = "1"

class JobRecapTileService : TileService() {

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    override fun onTileResourcesRequest(
        requestParams: RequestBuilders.ResourcesRequest
    ): ListenableFuture<ResourceBuilders.Resources> {
        return Futures.immediateFuture(
            ResourceBuilders.Resources.Builder()
                .setVersion(RESOURCES_VERSION)
                .build()
        )
    }

    override fun onTileRequest(
        requestParams: RequestBuilders.TileRequest
    ): ListenableFuture<TileBuilders.Tile> {
        // CallbackToFutureAdapter bridges the ListenableFuture API
        // expected by TileService into the suspend world without
        // pulling in kotlinx-coroutines-guava just for this one call.
        return CallbackToFutureAdapter.getFuture { completer ->
            scope.launch {
                try {
                    val state = fetchLatestRecap()
                    completer.set(buildTile(state))
                } catch (t: Throwable) {
                    completer.set(buildTile(TileState.Empty))
                }
            }
            "onTileRequest"
        }
    }

    private fun buildTile(state: TileState): TileBuilders.Tile =
        TileBuilders.Tile.Builder()
            .setResourcesVersion(RESOURCES_VERSION)
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
     * Fetch the freshest recap by:
     *   1. listing /watch/jobs and picking the head terminal job;
     *   2. fetching /watch/jobs/{id}/recap on that subject.
     * Returns null on any failure so the tile renders the placeholder.
     */
    private suspend fun fetchLatestRecap(): TileState {
        val auth = WearAuthManager(applicationContext)
        if (!auth.isRegistered) return TileState.NotPaired
        val net = WearNetworkManager(applicationContext, auth)
        return try {
            val jobsResp = net.listJobs()
            val arr = jobsResp.optJSONArray("jobs") ?: return TileState.Empty
            var picked: JSONObject? = null
            for (i in 0 until arr.length()) {
                val j = arr.getJSONObject(i)
                val s = j.optString("status")
                if (s == "complete" || s == "failed" || s == "cancelled") {
                    picked = j
                    break
                }
            }
            val job = picked ?: return TileState.Empty
            val sid = job.optString("session_id")
            if (sid.isEmpty()) return TileState.Empty
            val recap = net.getJobRecap(sid)
            if (recap != null) {
                TileState.Ready(headline = recap.headline, status = job.optString("status"))
            } else {
                TileState.Ready(
                    headline = job.optString("task_preview").ifEmpty { "Job complete" },
                    status = job.optString("status"),
                )
            }
        } catch (e: Exception) {
            Log.w(TAG, "fetchLatestRecap failed: ${e.message}")
            TileState.Empty
        }
    }

    private fun layoutFor(state: TileState): LayoutElementBuilders.LayoutElement {
        val (title, body) = when (state) {
            is TileState.Ready -> "Last job · ${state.status}" to state.headline
            TileState.Empty -> "VibeCody" to "No job recap yet"
            TileState.NotPaired -> "VibeCody" to "Pair on the phone"
        }
        return LayoutElementBuilders.Box.Builder()
            .setWidth(androidx.wear.protolayout.DimensionBuilders.expand())
            .setHeight(androidx.wear.protolayout.DimensionBuilders.expand())
            .setModifiers(
                ModifiersBuilders.Modifiers.Builder()
                    .setClickable(launchAppClickable())
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

    private fun launchAppClickable(): ModifiersBuilders.Clickable {
        // Intent that lands on MainActivity; the routing inside the
        // app sends the user to the Jobs tab when this extra is set.
        val intent = Intent(applicationContext, MainActivity::class.java).apply {
            putExtra("vibecody.deeplink", "jobs")
            flags = Intent.FLAG_ACTIVITY_NEW_TASK
        }
        return ModifiersBuilders.Clickable.Builder()
            .setId("open_jobs")
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

    sealed class TileState {
        data class Ready(val headline: String, val status: String) : TileState()
        object Empty : TileState()
        object NotPaired : TileState()
    }
}
