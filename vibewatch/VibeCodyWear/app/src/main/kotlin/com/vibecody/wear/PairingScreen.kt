// PairingScreen.kt — Pairing flow for Wear OS.
//
// Two options:
//   1. Scan QR — uses the ZXing intent (real devices with camera)
//   2. Enter URL — enter daemon URL only; app fetches challenge and completes
//      the full P256 registration automatically. No token required.

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
import java.util.concurrent.TimeUnit

@Composable
fun PairingScreen(auth: WearAuthManager, onPaired: () -> Unit) {
    val scope = rememberCoroutineScope()
    var mode by remember { mutableStateOf<PairMode>(PairMode.Choose) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    // QR scanner result launcher (real devices)
    val qrLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        val qrData = result.data?.getStringExtra("SCAN_RESULT") ?: return@rememberLauncherForActivityResult
        scope.launch {
            busy = true; error = null
            try {
                val payload = JSONObject(qrData)
                registerWithChallenge(
                    auth = auth,
                    endpoint = payload.getString("endpoint"),
                    nonce = payload.getString("nonce"),
                    issuedAt = payload.getLong("issued_at"),
                )
                onPaired()
            } catch (e: Exception) {
                error = e.message ?: "Registration failed"
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
                    ) { Text("Scan QR") }
                }
                item {
                    CompactButton(
                        onClick = { mode = PairMode.Url(""); error = null },
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
                        placeholder = "http://10.0.2.2:7878",
                    )
                }
                item {
                    Text(
                        "App fetches a challenge\nand registers automatically.",
                        style = MaterialTheme.typography.caption2,
                        color = MaterialTheme.colors.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp),
                    )
                }
                item {
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        modifier = Modifier.padding(top = 4.dp),
                    ) {
                        Button(
                            onClick = {
                                val url = m.url.trim().trimEnd('/')
                                if (url.isEmpty()) { error = "Enter the daemon URL"; return@Button }
                                scope.launch {
                                    busy = true; error = null
                                    try {
                                        val (nonce, issuedAt) = fetchChallenge(url)
                                        registerWithChallenge(auth, url, nonce, issuedAt)
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
    data class Url(val url: String) : PairMode()
}

// ── Step 1: fetch a registration challenge from the daemon (no auth needed) ───

private data class ChallengeResult(val nonce: String, val issuedAt: Long)

private suspend fun fetchChallenge(url: String): ChallengeResult = withContext(Dispatchers.IO) {
    val client = okhttp3.OkHttpClient.Builder()
        .connectTimeout(8, TimeUnit.SECONDS)
        .readTimeout(8, TimeUnit.SECONDS)
        .build()
    val req = okhttp3.Request.Builder()
        .url("$url/watch/challenge")
        .post("{}".toRequestBody("application/json".toMediaType()))
        .build()
    val resp = client.newCall(req).execute()
    val body = resp.body?.string() ?: "{}"
    if (!resp.isSuccessful) {
        val msg = runCatching { JSONObject(body).optString("error") }.getOrNull()
        throw Exception(msg?.ifEmpty { null } ?: "Challenge failed (${resp.code})")
    }
    val json = JSONObject(body)
    ChallengeResult(
        nonce = json.getString("nonce"),
        issuedAt = json.getLong("issued_at"),
    )
}

// ── Step 2: register the device with the P256 key + challenge signature ───────

private suspend fun registerWithChallenge(
    auth: WearAuthManager,
    endpoint: String,
    nonce: String,
    issuedAt: Long,
) = withContext(Dispatchers.IO) {
    val deviceId = "wear-${auth.freshNonce()}"
    val signature = auth.buildRegistrationSignature(nonce, deviceId, issuedAt)

    val body = JSONObject().apply {
        put("device_id", deviceId)
        put("name", android.os.Build.MODEL)
        put("model", "${android.os.Build.MANUFACTURER} ${android.os.Build.MODEL}")
        put("os_version", android.os.Build.VERSION.RELEASE)
        put("nonce", nonce)
        put("public_key_b64", auth.publicKeyBase64)
        put("signature_b64", signature)
    }.toString()

    val client = okhttp3.OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()
    val req = okhttp3.Request.Builder()
        .url("$endpoint/watch/register")
        .post(body.toRequestBody("application/json".toMediaType()))
        .build()

    val resp = client.newCall(req).execute()
    val respJson = JSONObject(resp.body?.string() ?: "{}")
    if (!resp.isSuccessful) {
        throw Exception(respJson.optString("error", "Registration failed (${resp.code})"))
    }

    val expiresIn = respJson.getLong("expires_in")
    auth.saveRegistration(
        deviceId = respJson.getString("device_id"),
        deviceName = android.os.Build.MODEL,
        daemonUrl = endpoint.trimEnd('/'),
        accessToken = respJson.getString("access_token"),
        refreshToken = respJson.getString("refresh_token"),
        expiresAt = System.currentTimeMillis() / 1000 + expiresIn,
    )
}
