package com.tongpin.todo.sync

/**
 * Platform-side sync control surface. Receives trigger signals from lifecycle
 * and network monitoring (see [com.tongpin.todo.lifecycle.SyncLifecycleObserver]
 * and [com.tongpin.todo.lifecycle.NetworkMonitor]) and routes them to the core
 * sync runtime.
 *
 * NOTE: the Plan 1 FFI contract currently exposes only CRUD/query; the sync
 * orchestrator (todo-core SyncOrchestrator / PairingFlow / discovery) is not yet
 * exposed over UniFFI. Once that FFI surface is added, a real implementation of
 * this interface will drive it. Until then the default is a no-op that simply
 * records the last trigger.
 */
interface SyncController {

    fun onTrigger(trigger: SyncTrigger)

    fun onForeground() = onTrigger(SyncTrigger.Foreground)

    fun onBackground() = Unit

    fun onNetworkChanged(available: Boolean) {
        if (available) onTrigger(SyncTrigger.NetworkChange)
    }
}
