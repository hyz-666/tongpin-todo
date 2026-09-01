package com.tongpin.todo.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Provides the 32-byte SQLCipher database key, protected by an Android Keystore
 * AES-GCM master key. The DB key never touches disk in plaintext: it is wrapped
 * with AES/GCM/NoPadding and stored alongside its 12-byte IV in the app's files
 * directory. The master key is non-exportable and held in hardware-backed
 * storage where available.
 */
class KeyProvider(context: Context) {

    private val appContext = context.applicationContext
    private val keyStore: KeyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
    private val wrappedFile: File = File(appContext.filesDir, "security/db_key.bin")

    /** Returns the stable 32-byte database key, generating it on first use. */
    fun databaseKey(): ByteArray {
        val master = getOrCreateMasterKey()
        val existing = readWrapped()
        if (existing != null) {
            return unwrap(master, existing)
        }
        val fresh = ByteArray(KEY_BYTES).also { SecureRandom().nextBytes(it) }
        storeWrapped(wrap(master, fresh))
        return fresh
    }

    private fun getOrCreateMasterKey(): SecretKey {
        (keyStore.getKey(MASTER_ALIAS, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                MASTER_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return generator.generateKey()
    }

    private fun wrap(master: SecretKey, plain: ByteArray): WrappedKey {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, master)
        return WrappedKey(cipher.iv, cipher.doFinal(plain))
    }

    private fun unwrap(master: SecretKey, wrapped: WrappedKey): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, master, GCMParameterSpec(GCM_TAG_BITS, wrapped.iv))
        return cipher.doFinal(wrapped.ciphertext)
    }

    private fun readWrapped(): WrappedKey? {
        if (!wrappedFile.exists()) return null
        val bytes = wrappedFile.readBytes()
        // Layout: [1-byte ivLen][iv][ciphertext]
        val ivLen = bytes[0].toInt() and 0xFF
        val iv = bytes.copyOfRange(1, 1 + ivLen)
        val ciphertext = bytes.copyOfRange(1 + ivLen, bytes.size)
        return WrappedKey(iv, ciphertext)
    }

    private fun storeWrapped(wrapped: WrappedKey) {
        wrappedFile.parentFile?.mkdirs()
        val out = ByteArray(1 + wrapped.iv.size + wrapped.ciphertext.size)
        out[0] = wrapped.iv.size.toByte()
        wrapped.iv.copyInto(out, 1)
        wrapped.ciphertext.copyInto(out, 1 + wrapped.iv.size)
        wrappedFile.writeBytes(out)
    }

    private data class WrappedKey(val iv: ByteArray, val ciphertext: ByteArray)

    private companion object {
        const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val MASTER_ALIAS = "tongpin_db_master_key"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val GCM_TAG_BITS = 128
        const val KEY_BYTES = 32
    }
}
