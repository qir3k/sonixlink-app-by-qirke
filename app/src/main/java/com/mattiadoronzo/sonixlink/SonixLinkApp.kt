package com.mattiadoronzo.sonixlink

import android.app.Application

/** Installs the crash recorder before any activity or service starts. */
class SonixLinkApp : Application() {

    override fun onCreate() {
        super.onCreate()
        // First, before anything reads a preference: a build that was just
        // reinstalled gets back the settings the last one left in Downloads.
        PreferencesBackup.restore(this)
        CrashLog.install(this)
        PreferencesBackup.watch(this)
    }
}
