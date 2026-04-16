// PairingScreen.kt — Pairing flow for Wear OS.
//
// Two options:
//   1. Scan QR — uses the ZXing intent, performs full P256 challenge registration
//   2. Enter URL — enter daemon URL + Bearer token directly (simple mode, ideal for emulators)

package com.vibecody.wear

import android.content.Intent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.wear.compose.material.*
import kotlinx.coroutines.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.Base64

@Composable
fun PairingScreen(auth: WearAuthManager, onPaired: () -> Unit) {
    val scope = rememberCoroutineScope()
    var mode by remember { mutableStateOf<PairMode>(PairMode.Choose) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    // QR scanner result launcher
    val qrLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        val qrData = result.data?.getStringExtra("SCAN_RESULT") ?: return@rememberLauncherForActivityResult
        scope.launch {
            busy = true; error = null
            try { register(auth, qrData); onPaired() }
            catch (e: Exception) { error = e.message ?: "Registration failed" }
            finally { busy = false }
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

        if (error != null) {
            item {
                Text(
                    error!!,
                    color = MaterialTheme.colors.error,
                    style = MaterialTheme.typography.caption2,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                )
            }
        }

        when (val m = mode) {
            PairMode.Choose -> {
                item {
                    Text(
                        "Choose pairing method",
                        style = MaterialTheme.typography.body2,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
                    )
                }
                item {
                    Button(
                        onClick = {
                            val intent = Intent("com.google.zxing.client.android.SCAN").apply {
                                putExtra("SCAN_MODE", "QR_CODE_MODE")
                            }
                            qrLauncher.launch(intent)
                        },
                        enabled = !busy,
                        modifier = Modifier.padding(top = 4.dp),
                    ) {
                        Text("Scan QR")
                    }
                }
                item {
                    CompactButton(
                        onClick = { mode = PairMode.Url("", ""); error = null },
                        enabled = !busy,
                        modifier = Modifier.padding(top = 4.dp),
                    ) {
                        Text("Enter URL", style = MaterialTheme.typography.caption2)
                    }
                }
            }

            is PairMode.Url -> {
                item {
                    Text(
                        "Daemon URL",
                        style = MaterialTheme.typography.caption2,
                        color = MaterialTheme.colors.onSurfaceVariant,
                        modifier = Modifier.padding(top = 8.dp, bottom = 2.dp),
                    )
                }
                item {
                    PairTextField(
                        value = m.url,
                        onValueChange = { mode = m.copy(url = it) },
                        placeholder = "http://192.168.x.x:7878",
                    )
                }
                item {
                    Text(
                        "API Token",
                        style = MaterialTheme.typography.caption2,
                        color = MaterialTheme.colors.onSurfaceVariant,
                        modifier = Modifier.padding(top = 8.dp, bottom = 2.dp),
                    )
                }
                item {
                    PairTextField(
                        value = m.token,
                        onValueChange = { mode = m.copy(token = it) },
                        placeholder = "Bearer token from daemon",
                    )
                }
                item {
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        modifier = Modifier.padding(top = 8.dp),
                    ) {
                        Button(
                            onClick = {
                                val url = m.url.trim().trimEnd('/')
                                val token = m.token.trim()
                                if (url.isEmpty() || token.isEmpty()) {
                                    error = "URL and token are required"; return@Button
                                }
                                scope.launch {
                                    busy = true; error = null
                                    try {
                                        // Verify reachability before saving
                                        verifyBearer(url, token)
                                        auth.saveSimpleAuth(url, token)
                                        onPaired()
                                    } catch (e: Exception) {
                                        error = e.message ?: "Connection failed"
                                    } finally {
                                        busy = false
                                    }
                                }
                            },
                            enabled = !busy,
                        ) {
                            if (busy) CircularProgressIndicator(modifier = Modifier.size(16.dp))
                            else Text("Connect")
                        }
                        CompactButton(
                            onClick = { mode = PairMode.Choose; error = null },
                            enabled = !busy,
                        ) {
                            Text("Back", style = MaterialTheme.typography.caption2)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun PairTextField(value: String, onValueChange: (String) -> Unit, placeholder: String) {
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        textStyle = TextStyle(color = MaterialTheme.colors.onSurface, fontSize = 11.sp),
        singleLine = true,
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 8.dp)
            .background(MaterialTheme.colors.surface, RoundedCornerShape(6.dp))
            .padding(horizontal = 8.dp, vertical = 6.dp),
        decorationBox = { inner ->
            if (value.isEmpty()) {
                Text(
                    placeholder,
                    style = TextStyle(
                        color = MaterialTheme.colors.onSurface.copy(alpha = 0.4f),
                        fontSize = 11.sp,
                    ),
                )
            }
            inner()
        },
    )
}

private sealed class PairMode {
    object Choose : PairMode()
    data class Url(val url: String, val token: String) : PairMode()
}

// ── Simple verification — hit /watch/sessions with Bearer ─────────────────────

private suspend fun verifyBearer(url: String, token: String) = withContext(Dispatchers.IO) {
    val client = okhttp3.OkHttpClient.Builder()
        .connectTimeout(5, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(5, java.util.concurrent.TimeUnit.SECONDS)
        .build()
    val req = okhttp3.Request.Builder()
        .url("$url/watch/sessions")
        .header("Authorization", "Bearer $token")
        .build()
    val resp = client.newCall(req).execute()
    if (!resp.isSuccessful) {
        val body = resp.body?.string() ?: ""
        throw Exception("Daemon returned ${resp.code}: $body")
    }
}

// ── Full P256 registration (QR path) ─────────────────────────────────────────

private suspend fun register(auth: WearAuthManager, qrJson: String) = withContext(Dispatchers.IO) {
    val payload = JSONObject(qrJson)
    val endpoint = payload.getString("endpoint")
    val nonce = payload.getString("nonce")

    val deviceId = "wear-${auth.freshNonce()}"
    val issuedAt = System.currentTimeMillis() / 1000
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
        .post(body.toRequestBody("application/json".toMediaType()))
        .build()

    val resp = client.newCall(req).execute()
    val respJson = JSONObject(resp.body?.string() ?: "{}")
    if (!resp.isSuccessful) {
        throw Exception(respJson.optString("error", "Registration failed (${resp.code})"))
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
