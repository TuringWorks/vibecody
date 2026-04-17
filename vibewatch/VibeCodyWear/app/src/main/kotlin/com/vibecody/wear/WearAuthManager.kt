// WearAuthManager.kt — Android Keystore-backed auth for VibeCody Wear OS.
//
// Security model mirrors WatchAuthManager.swift:
//   • ECDSA P-256 key pair generated in Android Keystore (backed by StrongBox /
//     Trusted Execution Environment where hardware allows).
//   • Private key is never extractable from the device.
//   • Challenge-response registration: sign SHA-256(nonce || deviceId || issuedAt).
//   • Token refresh: sign SHA-256(refreshToken || timestamp) as proof of possession.
//   • Access tokens stored in EncryptedSharedPreferences (AES-256-GCM, key in Keystore).
//   • On-body detection locks the session when the watch is removed.

package com.vibecody.wear

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKeys
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.PublicKey
import java.security.Signature
import java.security.spec.ECGenParameterSpec
import java.util.Base64

private const val TAG = "WearAuthManager"
private const val KEYSTORE_ALIAS = "vibecody_wear_key"
private const val PREFS_FILE = "vibecody_wear_auth"
private const val KEY_DEVICE_ID = "device_id"
private const val KEY_DEVICE_NAME = "device_name"
private const val KEY_ACCESS_TOKEN = "access_token"
private const val KEY_REFRESH_TOKEN = "refresh_token"
private const val KEY_TOKEN_EXPIRES_AT = "token_expires_at"
private const val KEY_DAEMON_URL = "daemon_url"
private const val KEY_REGISTERED = "registered"

class WearAuthManager(private val context: Context) {

    // ── Android Keystore ──────────────────────────────────────────────────────

    private val keyStore: KeyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    /** Returns the P-256 public key, generating the key pair if needed. */
    val publicKey: PublicKey
        get() {
            if (!keyStore.containsAlias(KEYSTORE_ALIAS)) {
                generateKeyPair()
            }
            return keyStore.getCertificate(KEYSTORE_ALIAS).publicKey
        }

    val publicKeyBase64: String
        get() {
            val encoded = publicKey.encoded  // 91-byte DER SubjectPublicKeyInfo for P256
            // Layout: 27-byte ASN.1 header + 0x04 prefix + 32-byte x + 32-byte y = 91 bytes total.
            // The Rust verifier expects the raw 64-byte x||y (no 0x04 prefix, no header).
            // Rust uses URL_SAFE_NO_PAD base64 — match it here.
            return Base64.getUrlEncoder().withoutPadding()
                .encodeToString(encoded.copyOfRange(encoded.size - 64, encoded.size))
        }

    private fun generateKeyPair() {
        val spec = KeyGenParameterSpec.Builder(
            KEYSTORE_ALIAS,
            KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY
        )
            .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
            .setDigests(KeyProperties.DIGEST_SHA256)
            .setUserAuthenticationRequired(false)  // watch always-on: no biometric gate
            .setIsStrongBoxBacked(isStrongBoxAvailable())  // hardware enclave if present
            .build()

        KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, "AndroidKeyStore").apply {
            initialize(spec)
            generateKeyPair()
        }
        Log.i(TAG, "Generated P-256 key pair in AndroidKeyStore (StrongBox=${isStrongBoxAvailable()})")
    }

    private fun isStrongBoxAvailable(): Boolean =
        context.packageManager.hasSystemFeature("android.hardware.strongbox_keystore")

    /**
     * Sign [data] with the device private key.
     * Returns the signature as a 64-byte compact IEEE P1363 (r||s) — the format
     * expected by the Rust p256 verifier.  Android Keystore returns ASN.1 DER,
     * so we convert here.
     */
    fun sign(data: ByteArray): ByteArray {
        if (!keyStore.containsAlias(KEYSTORE_ALIAS)) generateKeyPair()
        val privateKey = keyStore.getKey(KEYSTORE_ALIAS, null) as PrivateKey
        val der = Signature.getInstance("SHA256withECDSA").run {
            initSign(privateKey)
            update(data)
            sign()
        }
        return derToCompact(der)
    }

    /**
     * Convert an ASN.1 DER-encoded ECDSA signature to the 64-byte compact
     * IEEE P1363 (r||s) format.
     * DER layout: 30 <seqLen> 02 <rLen> [r…] 02 <sLen> [s…]
     * r and s are big-endian and may have a leading 0x00 byte (sign extension)
     * or fewer than 32 bytes (leading zeros stripped).
     */
    private fun derToCompact(der: ByteArray): ByteArray {
        var pos = 0
        require(der[pos++] == 0x30.toByte()) { "Expected SEQUENCE" }
        pos++ // skip sequence length
        require(der[pos++] == 0x02.toByte()) { "Expected INTEGER for r" }
        val rLen = der[pos++].toInt() and 0xFF
        val r = der.copyOfRange(pos, pos + rLen); pos += rLen
        require(der[pos++] == 0x02.toByte()) { "Expected INTEGER for s" }
        val sLen = der[pos++].toInt() and 0xFF
        val s = der.copyOfRange(pos, pos + sLen)

        val out = ByteArray(64)
        // Strip leading 0x00 sign byte; pad left to 32 bytes if shorter
        val rClean = if (r.size == 33 && r[0] == 0x00.toByte()) r.copyOfRange(1, 33) else r
        val sClean = if (s.size == 33 && s[0] == 0x00.toByte()) s.copyOfRange(1, 33) else s
        rClean.copyInto(out, destinationOffset = 32 - rClean.size)
        sClean.copyInto(out, destinationOffset = 64 - sClean.size)
        return out
    }

    // ── Encrypted preferences ─────────────────────────────────────────────────

    private val prefs by lazy {
        val masterKeyAlias = MasterKeys.getOrCreate(MasterKeys.AES256_GCM_SPEC)
        EncryptedSharedPreferences.create(
            PREFS_FILE, masterKeyAlias, context,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    }

    // ── Registration state ────────────────────────────────────────────────────

    val isRegistered: Boolean get() = prefs.getBoolean(KEY_REGISTERED, false)

    val deviceId: String get() = prefs.getString(KEY_DEVICE_ID, "") ?: ""
    val daemonUrl: String get() = prefs.getString(KEY_DAEMON_URL, "") ?: ""

    val accessToken: String? get() {
        val token = prefs.getString(KEY_ACCESS_TOKEN, null) ?: return null
        val expiresAt = prefs.getLong(KEY_TOKEN_EXPIRES_AT, 0L)
        return if (System.currentTimeMillis() / 1000 < expiresAt - 30) token else null
    }
    val refreshToken: String? get() = prefs.getString(KEY_REFRESH_TOKEN, null)

    fun saveRegistration(
        deviceId: String,
        deviceName: String,
        daemonUrl: String,
        accessToken: String,
        refreshToken: String,
        expiresAt: Long,
    ) {
        prefs.edit()
            .putBoolean(KEY_REGISTERED, true)
            .putString(KEY_DEVICE_ID, deviceId)
            .putString(KEY_DEVICE_NAME, deviceName)
            .putString(KEY_DAEMON_URL, daemonUrl)
            .putString(KEY_ACCESS_TOKEN, accessToken)
            .putString(KEY_REFRESH_TOKEN, refreshToken)
            .putLong(KEY_TOKEN_EXPIRES_AT, expiresAt)
            .apply()
    }

    fun saveTokens(accessToken: String, refreshToken: String, expiresAt: Long) {
        prefs.edit()
            .putString(KEY_ACCESS_TOKEN, accessToken)
            .putString(KEY_REFRESH_TOKEN, refreshToken)
            .putLong(KEY_TOKEN_EXPIRES_AT, expiresAt)
            .apply()
    }

    fun clearRegistration() {
        prefs.edit().clear().apply()
        if (keyStore.containsAlias(KEYSTORE_ALIAS)) {
            keyStore.deleteEntry(KEYSTORE_ALIAS)
        }
    }

    // ── Challenge signing ─────────────────────────────────────────────────────

    /**
     * Build the registration signature.
     * Message: nonce_bytes || device_id_bytes || issued_at_big_endian_u64
     * SHA-256 hashing is done inside SHA256withECDSA — do NOT pre-hash.
     */
    fun buildRegistrationSignature(nonce: String, deviceId: String, issuedAt: Long): String {
        val ts = issuedAt.toBeBytes()
        val msg = nonce.toByteArray(Charsets.UTF_8) + deviceId.toByteArray(Charsets.UTF_8) + ts
        return b64(sign(msg))
    }

    /**
     * Build the refresh proof signature.
     * Message: refresh_token_bytes || timestamp_big_endian_u64
     */
    fun buildRefreshSignature(refreshToken: String, timestamp: Long): String {
        val msg = refreshToken.toByteArray(Charsets.UTF_8) + timestamp.toBeBytes()
        return b64(sign(msg))
    }

    /**
     * Build the wrist event signature.
     * Message: device_id_bytes || on_wrist_byte || timestamp_big_endian_u64
     */
    fun buildWristSignature(deviceId: String, onWrist: Boolean, timestamp: Long): String {
        val msg = deviceId.toByteArray(Charsets.UTF_8) +
                byteArrayOf(if (onWrist) 1 else 0) +
                timestamp.toBeBytes()
        return b64(sign(msg))
    }

    /** URL-safe base64 without padding — matches Rust's URL_SAFE_NO_PAD. */
    private fun b64(bytes: ByteArray): String =
        Base64.getUrlEncoder().withoutPadding().encodeToString(bytes)

    private fun Long.toBeBytes(): ByteArray {
        val out = ByteArray(8)
        var v = this
        for (i in 7 downTo 0) { out[i] = (v and 0xFF).toByte(); v = v shr 8 }
        return out
    }

    // ── Nonce generation ──────────────────────────────────────────────────────

    fun freshNonce(): String = java.util.UUID.randomUUID().toString().replace("-", "")
}
