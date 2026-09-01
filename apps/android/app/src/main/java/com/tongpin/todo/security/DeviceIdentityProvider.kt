package com.tongpin.todo.security

import android.content.Context
import java.security.SecureRandom

/**
 * Provides a stable 32-byte device identity, persisted across launches.
 *
 * The identity is used as the CRDT operation origin (DeviceId) for every local
 * write, so it must never change for the lifetime of a profile. It is a random
 * identifier (not a secret) and is stored as hex in SharedPreferences.
 */
class DeviceIdentityProvider(context: Context) {

    private val prefs = context.applicationContext
        .getSharedPreferences("tongpin_identity", Context.MODE_PRIVATE)

    /** Returns the stable 32-byte device identity, generating it on first use. */
    fun deviceId(): ByteArray {
        val hex = prefs.getString(KEY, null)
        if (hex != null && hex.length == KEY_BYTES * 2) {
            return hexToBytes(hex)
        }
        val fresh = ByteArray(KEY_BYTES).also { SecureRandom().nextBytes(it) }
        prefs.edit().putString(KEY, bytesToHex(fresh)).apply()
        return fresh
    }

    private fun hexToBytes(hex: String): ByteArray =
        ByteArray(hex.length / 2) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }

    private fun bytesToHex(bytes: ByteArray): String =
        bytes.joinToString("") { "%02x".format(it.toInt() and 0xFF) }

    private companion object {
        const val KEY = "device_id_hex"
        const val KEY_BYTES = 32
    }
}
