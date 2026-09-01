package com.tongpin.todo

import android.app.Application
import com.tongpin.todo.di.AppContainer

/**
 * Application entry point. Builds the [AppContainer] (and, transitively, the
 * native core session) once per process. Registered in AndroidManifest.xml.
 */
class TodoApplication : Application() {

    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
    }
}
