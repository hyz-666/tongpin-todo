package com.tongpin.todo.lifecycle

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network

/**
 * Monitors default-network availability and reports changes via [onChange].
 * Used to trigger a sync pass when connectivity returns (matching the core's
 * NetworkChange trigger).
 */
class NetworkMonitor(
    context: Context,
    private val onChange: (available: Boolean) -> Unit,
) {
    private val connectivityManager =
        context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager

    private val callback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) = onChange(true)
        override fun onLost(network: Network) = onChange(false)
    }

    fun start() {
        connectivityManager.registerDefaultNetworkCallback(callback)
    }

    fun stop() {
        connectivityManager.unregisterNetworkCallback(callback)
    }
}
