package com.tongpin.todo.sync

import java.util.concurrent.atomic.AtomicReference

/**
 * Default [SyncController]: a no-op recorder used until the sync orchestrator is
 * exposed over the FFI. Keeps the last trigger so the UI/telemetry can show that
 * the lifecycle wiring is live without a real sync backend.
 */
class NoopSyncController : SyncController {

    private val lastTrigger = AtomicReference<SyncTrigger?>(null)

    val currentTrigger: SyncTrigger? get() = lastTrigger.get()

    override fun onTrigger(trigger: SyncTrigger) {
        lastTrigger.set(trigger)
    }
}
