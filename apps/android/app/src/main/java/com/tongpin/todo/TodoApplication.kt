package com.tongpin.todo

import android.app.Application
import androidx.lifecycle.ProcessLifecycleOwner
import com.tongpin.todo.di.AppContainer
import com.tongpin.todo.lifecycle.NetworkMonitor
import com.tongpin.todo.lifecycle.SyncLifecycleObserver
import com.tongpin.todo.security.DeviceIdentityProvider
import com.tongpin.todo.sync.LanSyncManager
import com.tongpin.todo.sync.NoopSyncController
import com.tongpin.todo.sync.SyncController

/**
 * Application entry point. Builds the [AppContainer] (and, transitively, the
 * native core session) once per process, and wires process-lifecycle + network
 * monitoring + mDNS discovery into the LAN sync manager. Registered in
 * AndroidManifest.xml.
 */
class TodoApplication : Application() {

    lateinit var container: AppContainer
        private set

    lateinit var syncController: SyncController
        private set

    lateinit var syncManager: LanSyncManager
        private set

    private lateinit var networkMonitor: NetworkMonitor

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
        syncController = NoopSyncController()

        // Instance name is derived from the device identity so each device
        // advertises a distinct, stable mDNS name.
        val deviceId = DeviceIdentityProvider(this).deviceId()
        val serviceName = "tongpin-" + deviceId.copyOf(8).joinToString("") { "%02x".format(it) }

        syncManager = LanSyncManager(
            context = this,
            controller = syncController,
            serviceName = serviceName,
            port = SYNC_PORT,
        )

        ProcessLifecycleOwner.get()
            .lifecycle
            .addObserver(SyncLifecycleObserver(syncManager))

        networkMonitor = NetworkMonitor(this) { available ->
            syncController.onNetworkChanged(available)
        }
        networkMonitor.start()
    }

    private companion object {
        // Nominal sync listener port; the real listener is bound by the core
        // once the sync FFI surface is exposed.
        const val SYNC_PORT = 7654
    }
}
