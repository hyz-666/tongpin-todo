package com.tongpin.todo.core

/**
 * Loads the Rust core shared library (libtodo_uniffi.so) exactly once.
 *
 * The UniFFI-generated bindings use JNA to call into the native symbols, but on
 * Android the .so must be pulled into the process via [System.loadLibrary] first
 * (the APK ships per-ABI copies under jniLibs/<abi>/ built by scripts/build-android.ps1).
 * Loading is idempotent and thread-safe.
 */
object NativeCoreLoader {

    const val LIBRARY_NAME: String = "todo_uniffi"

    @Volatile
    private var loaded: Boolean = false

    /** Idempotently loads the native library. Safe to call from any thread. */
    fun ensureLoaded() {
        if (loaded) return
        synchronized(this) {
            if (!loaded) {
                System.loadLibrary(LIBRARY_NAME)
                loaded = true
            }
        }
    }

    /** Whether the native library has already been loaded into this process. */
    fun isLoaded(): Boolean = loaded
}
