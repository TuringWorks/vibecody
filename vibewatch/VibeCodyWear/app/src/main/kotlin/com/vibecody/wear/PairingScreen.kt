// PairingScreen.kt — QR-code pairing flow for Wear OS.
//
// The user scans the QR code shown in the VibeUI "Apple Watch" settings panel
// (which also covers Wear OS — same /watch/challenge endpoint).  The QR payload
// contains: endpoint, nonce, machine_id, expires_at, version.
//
// On Wear OS the camera scan uses the built-in QR scanner via an implicit
// intent (supported on Pixel Watch, Galaxy Watch 7+, Fossil Gen 6+).

package com.vibecody.wear

import android.app.Activity
import android.content.Intent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.wear.compose.material.*
import kotlinx.coroutines.*
import org.json.JSONObject
import java.util.Base64

@Composable
fun PairingScreen(auth: WearAuthManager, onPaired: () -> Unit) {
    val scope = rememberCoroutineScope()
    var status by remember { mutableStateOf("Scan QR to pair") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    // QR scanner result launcher
    val qrLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        val qrData = result.data?.getStringExtra("SCAN_RESULT") ?: return@rememberLauncherForActivityResult
        scope.launch {
            busy = true
            error = null
            status = "Registering…"
            try {
                register(auth, qrData)
                onPaired()
            } catch (e: Exception) {
                error = e.message ?: "Registration failed"
                status = "Scan QR to pair"
            } finally {
                busy = false
            }
        }
    }

    ScalingLazyColumn(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        item {
            Text(
                "VibeCody",
                style = MaterialTheme.typography.title2,
                textAlign = TextAlign.Center,
            )
        }
        item {
            Text(
                status,
                style = MaterialTheme.typography.body2,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
        }
        if (error != null) {
            item {
                Text(
                    error!!,
                    color = MaterialTheme.colors.error,
                    style = MaterialTheme.typography.caption2,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(horizontal = 8.dp),
                )
            }
        }
        item {
            Button(
                onClick = {
                    // Launch QR scanner intent (ZXing / Google Code Scanner)
                    val intent = Intent("com.google.zxing.client.android.SCAN").apply {
                        putExtra("SCAN_MODE", "QR_CODE_MODE")
                    }
                    qrLauncher.launch(intent)
                },
                enabled = !busy,
                modifier = Modifier.padding(top = 8.dp),
            ) {
                if (busy) CircularProgressIndicator(modifier = Modifier.size(20.dp))
                else Text("Scan QR")
            }
        }
    }
}

// ── Registration flow ─────────────────────────────────────────────────────────

private suspend fun register(auth: WearAuthManager, qrJson: String) = withContext(Dispatchers.IO) {
    val payload = JSONObject(qrJson)
    val endpoint = payload.getString("endpoint")
    val nonce = payload.getString("nonce")
    val machineId = payload.optString("machine_id", "")

    // Generate a stable device ID
    val deviceId = "wear-${auth.freshNonce()}"
    val issuedAt = System.currentTimeMillis() / 1000

    // Build registration signature (matches watch_auth.rs verify_ed25519_signature)
    val signature = auth.buildRegistrationSignature(nonce, deviceId, issuedAt)

    val body = JSONObject().apply {
        put("device_id", deviceId)
        put("name", android.os.Build.MODEL)
        put("model", "${android.os.Build.MANUFACTURER} ${android.os.Build.MODEL}")
        put("os_version", android.os.Build.VERSION.RELEASE)
        put("nonce", nonce)
        put("issued_at", issuedAt)
        put("public_key", auth.publicKeyBase64)
        put("signature", signature)
    }.toString()

    val client = okhttp3.OkHttpClient()
    val req = okhttp3.Request.Builder()
        .url("$endpoint/watch/register")
        .post(okhttp3.RequestBody.create(okhttp3.MediaType.parse("application/json"), body))
        .build()

    val resp = client.newCall(req).execute()
    val respJson = JSONObject(resp.body()?.string() ?: "{}")
    if (!resp.isSuccessful) {
        throw Exception(respJson.optString("error", "Registration failed (${resp.code()})"))
    }

    auth.saveRegistration(
        deviceId = respJson.getString("device_id"),
        deviceName = android.os.Build.MODEL,
        daemonUrl = endpoint,
        accessToken = respJson.getString("access_token"),
        refreshToken = respJson.getString("refresh_token"),
        expiresAt = respJson.getLong("expires_at"),
    )
}
