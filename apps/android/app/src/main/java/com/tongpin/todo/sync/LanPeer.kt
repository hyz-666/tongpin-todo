package com.tongpin.todo.sync

/**
 * A peer discovered on the local network via mDNS. [host] and [port] are the
 * resolved dial endpoints (the port targets the core's sync TCP listener).
 */
data class LanPeer(
    val serviceName: String,
    val host: String,
    val port: Int,
)
