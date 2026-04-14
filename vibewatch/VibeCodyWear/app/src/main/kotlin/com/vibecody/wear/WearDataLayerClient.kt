// WearDataLayerClient.kt — Wearable Data Layer offline relay.
//
// When the watch has no direct network path to the daemon (no LAN, no
// Tailscale), requests are forwarded to the paired Android phone via the
// Wearable Data Layer API (com.google.android.gms:play-services-wearable).
// The phone's WearDataLayerService relays them to the daemon using its
// bearer token and sends the response back as a Data Layer message.
//
// Message paths:
//   Watch → Phone:  /vibecody/dispatch   (dispatch request JSON)
//   Phone → Watch:  /vibecody/response   (dispatch response JSON)

package com.vibecody.wear

import android.content.Context
import android.util.Log
import com.google.android.gms.wearable.ChannelClient
import com.google.android.gms.wearable.MessageClient
import com.google.android.gms.wearable.Wearable
import org.json.JSONObject

private const val TAG = "WearDataLayerClient"

object WearDataLayerClient {

    /**
     * Send [data] to the connected phone node via a Capability message.
     * The phone's WearDataLayerService handles path [path] and replies on
     * /vibecody/response.
     */
    fun sendMessage(
        context: Context,
        path: String,
        data: ByteArray,
        onResult: (JSONObject) -> Unit,
        onError: (String) -> Unit,
    ) {
        val nodeClient = Wearable.getNodeClient(context)
        nodeClient.connectedNodes.addOnSuccessListener { nodes ->
            val phone = nodes.firstOrNull()
            if (phone == null) {
                onError("No connected phone node found")
                return@addOnSuccessListener
            }
            val msgClient = Wearable.getMessageClient(context)
            msgClient.sendMessage(phone.id, path, data)
                .addOnSuccessListener {
                    Log.d(TAG, "Message sent to ${phone.displayName} on path $path")
                    // Response arrives via WearResponseListenerService
                }
                .addOnFailureListener { e ->
                    onError("Data Layer send failed: ${e.message}")
                }
        }.addOnFailureListener { e ->
            onError("Failed to get connected nodes: ${e.message}")
        }
    }
}
