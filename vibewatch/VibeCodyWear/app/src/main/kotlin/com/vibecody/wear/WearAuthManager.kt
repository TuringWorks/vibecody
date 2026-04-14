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
import java.security.*
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
        get() = Base64.getEncoder().encodeToString(publicKey.encoded)

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

    /** Sign data with the device private key (never leaves Keystore). */
    fun sign(data: ByteArray): ByteArray {
        val privateKey = keyStore.getKey(KEYSTORE_ALIAS, null) as PrivateKey
        return Signature.getInstance("SHA256withECDSA").run {
            initSign(privateKey)
            update(data)
            sign()
        }
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
     * Build the registration signature payload:
     *   SHA-256(nonce_bytes || device_id_bytes || issued_at_big_endian_u64)
     * Matches the verification in watch_auth.rs verify_ed25519_signature().
     */
    fun buildRegistrationSignature(nonce: String, deviceId: String, issuedAt: Long): String {
        val md = MessageDigest.getInstance("SHA-256")
        md.update(nonce.toByteArray(Charsets.UTF_8))
        md.update(deviceId.toByteArray(Charsets.UTF_8))
        val ts = ByteArray(8)
        var t = issuedAt
        for (i in 7 downTo 0) { ts[i] = (t and 0xFF).toByte(); t = t shr 8 }
        md.update(ts)
        val digest = md.digest()
        val sig = sign(digest)
        return Base64.getEncoder().encodeToString(sig)
    }

    /**
     * Build the refresh signature:
     *   SHA-256(refresh_token_bytes || timestamp_big_endian_u64)
     */
    fun buildRefreshSignature(refreshToken: String, timestamp: Long): String {
        val md = MessageDigest.getInstance("SHA-256")
        md.update(refreshToken.toByteArray(Charsets.UTF_8))
        val ts = ByteArray(8)
        var t = timestamp
        for (i in 7 downTo 0) { ts[i] = (t and 0xFF).toByte(); t = t shr 8 }
        md.update(ts)
        val digest = md.digest()
        return Base64.getEncoder().encodeToString(sign(digest))
    }

    /**
     * Build the wrist event signature:
     *   SHA-256(device_id_bytes || on_wrist_byte || timestamp_big_endian_u64)
     */
    fun buildWristSignature(deviceId: String, onWrist: Boolean, timestamp: Long): String {
        val md = MessageDigest.getInstance("SHA-256")
        md.update(deviceId.toByteArray(Charsets.UTF_8))
        md.update(if (onWrist) byteArrayOf(1) else byteArrayOf(0))
        val ts = ByteArray(8)
        var t = timestamp
        for (i in 7 downTo 0) { ts[i] = (t and 0xFF).toByte(); t = t shr 8 }
        md.update(ts)
        val digest = md.digest()
        return Base64.getEncoder().encodeToString(sign(digest))
    }

    // ── Nonce generation ──────────────────────────────────────────────────────

    fun freshNonce(): String = java.util.UUID.randomUUID().toString().replace("-", "")
}
