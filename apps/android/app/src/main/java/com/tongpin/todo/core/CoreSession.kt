package com.tongpin.todo.core

import com.tongpin.todo.Core
import com.tongpin.todo.FfiCommand
import com.tongpin.todo.FfiMutationReceipt
import com.tongpin.todo.FfiPage
import com.tongpin.todo.FfiPagedTasks
import com.tongpin.todo.FfiSearchHit
import com.tongpin.todo.FfiTaskQuery

/**
 * A thin, lifetime-managed wrapper over the UniFFI [Core] object.
 *
 * The wrapped [Core] is internally serialized by the Rust side (Mutex-guarded
 * repository), so a single instance is safe to share across the app. Creation
 * requires the database key and device identity — see
 * [com.tongpin.todo.security.KeyProvider] and
 * [com.tongpin.todo.security.DeviceIdentityProvider].
 */
class CoreSession private constructor(private val core: Core) : AutoCloseable {

    fun dispatch(command: FfiCommand): FfiMutationReceipt = core.dispatch(command)

    fun listTasks(query: FfiTaskQuery, page: FfiPage, today: String): FfiPagedTasks =
        core.listTasks(query, page, today)

    fun search(text: String, limit: UInt): List<FfiSearchHit> = core.search(text, limit)

    override fun close() {
        core.close()
    }

    companion object {
        /**
         * Opens a session against the encrypted profile at [profilePath].
         *
         * @param dbKey 32-byte SQLCipher key (see [com.tongpin.todo.security.KeyProvider]).
         * @param deviceId 32-byte device identity (see [com.tongpin.todo.security.DeviceIdentityProvider]).
         */
        fun open(profilePath: String, dbKey: ByteArray, deviceId: ByteArray): CoreSession {
            require(dbKey.size == 32) { "dbKey must be 32 bytes, got ${dbKey.size}" }
            require(deviceId.size == 32) { "deviceId must be 32 bytes, got ${deviceId.size}" }
            NativeCoreLoader.ensureLoaded()
            return CoreSession(Core.open(profilePath, dbKey, deviceId))
        }
    }
}
