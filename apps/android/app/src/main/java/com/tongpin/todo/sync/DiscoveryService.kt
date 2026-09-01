package com.tongpin.todo.sync

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.util.Log

/**
 * mDNS (NSD) discovery for LAN peer finding. Registers a local service so other
 * devices can discover us, and discovers/resolves peers of the same service
 * type. Listener objects are retained as fields to avoid premature GC (an NSD
 * gotcha that silently stops callbacks).
 *
 * NOTE: the core's DiscoveryHint (HMAC-SHA256 of the pairing secret) should
 * eventually be folded into the instance name so only devices sharing a secret
 * are discoverable; that wiring lands once the discovery FFI surface is added.
 */
class DiscoveryService(
    context: Context,
    private val listener: Listener,
) {

    interface Listener {
        fun onPeerFound(peer: LanPeer)
        fun onPeerLost(peer: LanPeer)
        fun onRegistered()
        fun onError(errorCode: Int)
    }

    private val nsdManager =
        context.getSystemService(Context.NSD_SERVICE) as NsdManager

    private var registered = false
    private var discovering = false

    private val registrationListener = object : NsdManager.RegistrationListener {
        override fun onServiceRegistered(info: NsdServiceInfo) {
            registered = true
            listener.onRegistered()
        }

        override fun onRegistrationFailed(info: NsdServiceInfo, errorCode: Int) {
            listener.onError(errorCode)
        }

        override fun onServiceUnregistered(info: NsdServiceInfo) {
            registered = false
        }

        override fun onUnregistrationFailed(info: NsdServiceInfo, errorCode: Int) {
            listener.onError(errorCode)
        }
    }

    private val discoveryListener = object : NsdManager.DiscoveryListener {
        override fun onDiscoveryStarted(serviceType: String) {
            discovering = true
        }

        override fun onServiceFound(serviceInfo: NsdServiceInfo) {
            nsdManager.resolveService(serviceInfo, resolveListener)
        }

        override fun onServiceLost(serviceInfo: NsdServiceInfo) {
            listener.onPeerLost(toPeer(serviceInfo))
        }

        override fun onDiscoveryStopped(serviceType: String) {
            discovering = false
        }

        override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
            listener.onError(errorCode)
        }

        override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
            listener.onError(errorCode)
        }
    }

    private val resolveListener = object : NsdManager.ResolveListener {
        override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
            listener.onError(errorCode)
        }

        override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
            listener.onPeerFound(toPeer(serviceInfo))
        }
    }

    /** Registers the local service and begins discovery. Idempotent. */
    fun start(serviceName: String, port: Int) {
        if (!registered) {
            val info = NsdServiceInfo().apply {
                this.serviceName = serviceName
                serviceType = SERVICE_TYPE
                this.port = port
            }
            nsdManager.registerService(info, NsdManager.PROTOCOL_DNS_SD, registrationListener)
        }
        if (!discovering) {
            nsdManager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, discoveryListener)
        }
    }

    /** Stops discovery and unregisters the local service. Idempotent. */
    fun stop() {
        if (discovering) {
            discovering = false
            nsdManager.stopServiceDiscovery(discoveryListener)
        }
        if (registered) {
            registered = false
            nsdManager.unregisterService(registrationListener)
        }
    }

    private fun toPeer(info: NsdServiceInfo): LanPeer = LanPeer(
        serviceName = info.serviceName,
        host = info.host?.hostAddress.orEmpty(),
        port = info.port,
    )

    companion object {
        const val SERVICE_TYPE = "_tongpin-todo._tcp."
        private const val TAG = "DiscoveryService"
    }
}
