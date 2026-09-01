package com.tongpin.todo.lifecycle

import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import com.tongpin.todo.sync.SyncController

/**
 * Observes the process lifecycle and maps foreground/background transitions to
 * sync triggers. Foregrounding triggers a sync pass; backgrounding can pause
 * eager syncing (the OS already defers periodic work).
 */
class SyncLifecycleObserver(
    private val controller: SyncController,
) : DefaultLifecycleObserver {

    override fun onStart(owner: LifecycleOwner) {
        controller.onForeground()
    }

    override fun onStop(owner: LifecycleOwner) {
        controller.onBackground()
    }
}
