package com.tongpin.todo.sync

/**
 * Sources that can kick off a sync pass. Mirrors the core's SyncTriggers
 * (todo-core/src/triggers.rs): network change and manual/foreground overrides
 * run eagerly, while periodic passes are deferred to the OS.
 */
enum class SyncTrigger {
    Startup,
    Foreground,
    NetworkChange,
    Manual,
    Periodic,
}
