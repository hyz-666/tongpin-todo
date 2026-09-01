package com.tongpin.todo.di

import android.content.Context
import com.tongpin.todo.core.CoreSession
import com.tongpin.todo.data.CoreRepository
import com.tongpin.todo.security.DeviceIdentityProvider
import com.tongpin.todo.security.KeyProvider
import java.io.File

/**
 * Hand-built application graph (no DI framework). Owns the singleton
 * [CoreSession] and [CoreRepository] wired to the Keystore-backed key and
 * device identity. Created once in [com.tongpin.todo.TodoApplication].
 */
class AppContainer(context: Context) {

    private val appContext = context.applicationContext
    private val keyProvider = KeyProvider(appContext)
    private val deviceIdentityProvider = DeviceIdentityProvider(appContext)

    /** Encrypted profile directory used by the Rust core. */
    val profilePath: String = File(appContext.filesDir, "profile").absolutePath

    val session: CoreSession by lazy {
        CoreSession.open(profilePath, keyProvider.databaseKey(), deviceIdentityProvider.deviceId())
    }

    val repository: CoreRepository by lazy { CoreRepository(session) }
}
