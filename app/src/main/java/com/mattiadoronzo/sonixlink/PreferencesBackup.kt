package com.mattiadoronzo.sonixlink

import android.annotation.SuppressLint
import android.app.Activity
import android.app.Application
import android.content.ContentUris
import android.content.ContentValues
import android.content.Context
import android.content.SharedPreferences
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import android.util.Base64
import android.util.Log
import org.json.JSONObject
import java.io.File
import java.util.concurrent.Executors

/**
 * Keeps the app's settings somewhere an uninstall does not reach.
 *
 * Everything the user sets up -- the tabs and their order, the accent, the
 * favourite mark, the volume step, the language, the player last used -- lives
 * in this app's private storage, and the package installer deletes that when a
 * build has to be reinstalled. A debug build often does have to be: a build
 * signed with another key cannot replace it, so the old one is removed first
 * and its settings go with it.
 *
 * The whole shared_prefs directory is mirrored, exactly as it stands, into a
 * small JSON file at Download/SonixLink/sonixlink-settings.json, which survives
 * the uninstall. A build that starts with no preferences of its own reads it
 * back before the first screen asks for one, so the tabs, the language and the
 * rest are as they were. Nothing else is copied: not the library index, not the
 * covers, not the logs.
 *
 * Android 10 and later only, where writing to Downloads needs no permission. On
 * older releases nothing is saved, and nothing is lost beyond what an uninstall
 * already takes.
 */
object PreferencesBackup {

    private const val TAG = "PreferencesBackup"
    private const val FOLDER = "SonixLink"
    private const val FILE_NAME = "sonixlink-settings.json"
    private const val PREFS = "sonixlink"
    private val PREF_FILE_NAME = Regex("[A-Za-z0-9._-]+\\.xml")

    /** How long a burst of changes is allowed to settle before one write. */
    private const val SAVE_MS = 1200L

    private val handler = Handler(Looper.getMainLooper())
    private val writer = Executors.newSingleThreadExecutor()
    private var app: Context? = null

    private val saveTask = Runnable {
        val context = app ?: return@Runnable
        writer.execute {
            try {
                write(context, snapshot(context))
            } catch (error: Exception) {
                Log.e(TAG, "Could not back up app settings", error)
            }
        }
    }

    /**
     * Held here on purpose: SharedPreferences keeps only a weak reference to the
     * listeners it is given, and a collected one never fires.
     */
    private val changeListener =
        SharedPreferences.OnSharedPreferenceChangeListener { _, _ -> schedule() }

    /**
     * Reads the settings back, and only into a build that has none. Called from
     * Application.onCreate, before anything reads a preference.
     */
    fun restore(context: Context) {
        val dir = prefsDir(context) ?: return
        // An install that already has preferences of its own keeps them: this is
        // a floor for a fresh install, never an override.
        if (dir.listFiles()?.any { it.isFile && it.name.endsWith(".xml") } == true) return
        val payload = read(context) ?: return
        val files = runCatching { JSONObject(payload).optJSONObject("files") }.getOrNull() ?: return
        if (files.length() == 0) return
        dir.mkdirs()
        for (name in files.keys()) {
            if (!PREF_FILE_NAME.matches(name) || File(name).name != name) continue
            val bytes = runCatching { Base64.decode(files.optString(name), Base64.NO_WRAP) }
                .getOrNull() ?: continue
            runCatching { File(dir, name).writeBytes(bytes) }
        }
    }

    /** Writes the settings out whenever they change, and as screens close. */
    fun watch(context: Context) {
        val application = context.applicationContext as? Application ?: return
        app = application
        application.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .registerOnSharedPreferenceChangeListener(changeListener)
        application.registerActivityLifecycleCallbacks(object : Application.ActivityLifecycleCallbacks {
            override fun onActivityCreated(activity: Activity, state: Bundle?) {}
            override fun onActivityStarted(activity: Activity) {}
            override fun onActivityResumed(activity: Activity) {}
            override fun onActivityPaused(activity: Activity) {}
            override fun onActivityStopped(activity: Activity) = schedule()
            override fun onActivitySaveInstanceState(activity: Activity, state: Bundle) {}
            override fun onActivityDestroyed(activity: Activity) {}
        })
    }

    fun backupNow(context: Context, onComplete: (Result<Unit>) -> Unit) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) {
            onComplete(Result.failure(IllegalStateException("Settings backup to Downloads requires Android 10 or newer.")))
            return
        }
        val application = context.applicationContext
        writer.execute {
            val result = runCatching { write(application, snapshot(application)) }
            handler.post { onComplete(result) }
        }
    }

    private fun schedule() {
        handler.removeCallbacks(saveTask)
        handler.postDelayed(saveTask, SAVE_MS)
    }

    private fun prefsDir(context: Context): File? =
        runCatching { File(context.dataDir, "shared_prefs") }.getOrNull()

    private fun snapshot(context: Context): String {
        val files = JSONObject()
        prefsDir(context)?.listFiles()?.forEach { file ->
            if (file.isFile && file.name.endsWith(".xml")) {
                files.put(file.name, Base64.encodeToString(file.readBytes(), Base64.NO_WRAP))
            }
        }
        return JSONObject().put("version", 1).put("files", files).toString()
    }

    @SuppressLint("NewApi")
    private fun write(context: Context, payload: String) {
        check(Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            "Settings backup to Downloads requires Android 10 or newer."
        }
        val values = ContentValues().apply {
            put(MediaStore.Downloads.DISPLAY_NAME, FILE_NAME)
            put(MediaStore.Downloads.MIME_TYPE, "application/json")
            put(MediaStore.Downloads.RELATIVE_PATH, relativePath())
            put(MediaStore.Downloads.IS_PENDING, 1)
        }
        val resolver = context.contentResolver
        val uri = find(context) ?: resolver.insert(collection(), values)
            ?: error("Android could not create the settings backup in Downloads.")
        val output = resolver.openOutputStream(uri, "wt")
            ?: error("Android could not open the settings backup in Downloads.")
        output.use { it.write(payload.toByteArray(Charsets.UTF_8)) }
        val updated = resolver.update(
            uri,
            ContentValues().apply { put(MediaStore.Downloads.IS_PENDING, 0) },
            null,
            null,
        )
        check(updated > 0) { "Android could not finish saving the settings backup." }
    }

    @SuppressLint("NewApi")
    private fun read(context: Context): String? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) return null
        val uri = find(context) ?: return null
        return runCatching {
            context.contentResolver.openInputStream(uri)?.use { it.readBytes().toString(Charsets.UTF_8) }
        }.getOrNull()
    }

    @SuppressLint("NewApi")
    private fun find(context: Context): Uri? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) return null
        val projection = arrayOf(MediaStore.Downloads._ID)
        val selection = "${MediaStore.Downloads.DISPLAY_NAME}=? AND ${MediaStore.Downloads.RELATIVE_PATH}=?"
        context.contentResolver
            .query(collection(), projection, selection, arrayOf(FILE_NAME, relativePath()), null)
            ?.use { cursor ->
                if (cursor.moveToFirst()) {
                    return ContentUris.withAppendedId(collection(), cursor.getLong(0))
                }
            }
        return null
    }

    @SuppressLint("NewApi")
    private fun collection(): Uri = MediaStore.Downloads.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)

    @SuppressLint("NewApi")
    private fun relativePath(): String = Environment.DIRECTORY_DOWNLOADS + "/" + FOLDER + "/"
}
