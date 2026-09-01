package com.tongpin.todo

import android.app.Application
import androidx.lifecycle.ProcessLifecycleOwner
import com.tongpin.todo.di.AppContainer
import com.tongpin.todo.lifecycle.NetworkMonitor
import com.tongpin.todo.lifecycle.SyncLifecycleObserver
import com.tongpin.todo.sync.NoopSyncController
import com.tongpin.todo.sync.SyncController

/**
 * Application entry point. Builds the [AppContainer] (and, transitively, the
 * native core session) once per process, and wires process-lifecycle + network
 * monitoring into the [SyncController]. Registered in AndroidManifest.xml.
 */
class TodoApplication : Application() {

    lateinit var container: AppContainer
        private set

    lateinit var syncController: SyncController
        private set

    private lateinit var networkMonitor: NetworkMonitor

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
        syncController = NoopSyncController()

        ProcessLifecycleOwner.get()
            .lifecycle
            .addObserver(SyncLifecycleObserver(syncController))

        networkMonitor = NetworkMonitor(this) { available ->
            syncController.onNetworkChanged(available)
        }
        networkMonitor.start()
    }
}
