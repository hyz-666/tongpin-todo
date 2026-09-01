package com.tongpin.todo.sync

import android.content.Context
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * Coordinates LAN sync on the Android side: owns the mDNS [DiscoveryService]
 * and exposes discovered peers as a [StateFlow]. Foregrounding starts discovery
 * and triggers a sync pass; backgrounding tears both down.
 *
 * The sync pass currently routes through the injected [SyncController]; wiring
 * it to the core SyncOrchestrator is the remaining FFI integration point.
 */
class LanSyncManager(
    context: Context,
    private val controller: SyncController,
    private val serviceName: String,
    private val port: Int,
) {

    private val _peers = MutableStateFlow<List<LanPeer>>(emptyList())

    /** Currently discovered peers on the LAN. */
    val peers: StateFlow<List<LanPeer>> = _peers.asStateFlow()

    private val discoveryService = DiscoveryService(
        context,
        object : DiscoveryService.Listener {
            override fun onPeerFound(peer: LanPeer) {
                _peers.update { current -> (current + peer).distinctBy { it.host to it.port } }
            }

            override fun onPeerLost(peer: LanPeer) {
                _peers.update { current -> current.filterNot { it.host == peer.host && it.port == peer.port } }
            }

            override fun onRegistered() = Unit

            override fun onError(errorCode: Int) = Unit
        },
    )

    fun onForeground() {
        controller.onForeground()
        discoveryService.start(serviceName, port)
    }

    fun onBackground() {
        discoveryService.stop()
        controller.onBackground()
    }
}
