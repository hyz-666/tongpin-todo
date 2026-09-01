package com.tongpin.todo.lifecycle

import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import com.tongpin.todo.sync.LanSyncManager

/**
 * Observes the process lifecycle and maps foreground/background transitions to
 * LAN sync lifecycle: foregrounding starts mDNS discovery and triggers a sync
 * pass; backgrounding tears both down (the OS already defers periodic work).
 */
class SyncLifecycleObserver(
    private val syncManager: LanSyncManager,
) : DefaultLifecycleObserver {

    override fun onStart(owner: LifecycleOwner) {
        syncManager.onForeground()
    }

    override fun onStop(owner: LifecycleOwner) {
        syncManager.onBackground()
    }
}
