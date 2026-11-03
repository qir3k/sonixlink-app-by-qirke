package com.mattiadoronzo.sonixlink

import android.app.DownloadManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

internal data class AppRelease(
    val version: String,
    val apkUrl: String,
    val notes: String,
)

internal object AppUpdater {
    private const val REPOSITORY = "qir3k/sonixlink-app-by-qirke"
    private const val RELEASE_API = "https://api.github.com/repos/$REPOSITORY/releases/latest"
    private const val PREFS = "app_updates"
    private const val DOWNLOAD_ID = "download_id"
    private const val DOWNLOAD_VERSION = "download_version"
    private const val DISMISSED_READY_ID = "dismissed_ready_id"
    private const val CHANNEL = "app_updates"
    private const val NOTIFICATION_ID = 7402
    private const val APK_MIME = "application/vnd.android.package-archive"

    private data class DownloadState(
        val status: Int,
        val downloaded: Long,
        val total: Long,
        val reason: Int,
    )

    private var monitorGeneration = 0
    private var monitorJob: Job? = null
    private var progressDialog: AlertDialog? = null
    private var progressLabel: TextView? = null
    private var progressBar: ProgressBar? = null

    fun checkForUpdates(activity: MainActivity) {
        val progress = MaterialAlertDialogBuilder(activity)
            .setMessage(R.string.update_checking)
            .setCancelable(false)
            .show()

        activity.lifecycleScope.launch {
            try {
                val release = withContext(Dispatchers.IO) { fetchLatestRelease() }
                progress.dismiss()
                if (activity.isFinishing || activity.isDestroyed) return@launch

                if (compareVersions(release.version, BuildConfig.VERSION_NAME) <= 0) {
                    MaterialAlertDialogBuilder(activity)
                        .setMessage(activity.getString(R.string.update_current, release.version))
                        .setPositiveButton(android.R.string.ok, null)
                        .show()
                } else {
                    MaterialAlertDialogBuilder(activity)
                        .setTitle(activity.getString(R.string.update_available_title, release.version))
                        .setMessage(
                            release.notes.ifBlank {
                                activity.getString(R.string.update_available_body, release.version)
                            },
                        )
                        .setNegativeButton(android.R.string.cancel, null)
                        .setPositiveButton(R.string.update_download) { _, _ ->
                            enqueueDownload(activity, release)
                        }
                        .show()
                }
            } catch (cancelled: CancellationException) {
                progress.dismiss()
                throw cancelled
            } catch (error: Exception) {
                progress.dismiss()
                if (!activity.isFinishing && !activity.isDestroyed) {
                    MaterialAlertDialogBuilder(activity)
                        .setMessage(
                            activity.getString(
                                R.string.update_check_failed,
                                error.localizedMessage ?: error.javaClass.simpleName,
                            ),
                        )
                        .setPositiveButton(android.R.string.ok, null)
                        .show()
                }
            }
        }
    }

    private fun fetchLatestRelease(): AppRelease {
        val connection = URL(RELEASE_API).openConnection() as HttpURLConnection
        connection.connectTimeout = 15_000
        connection.readTimeout = 15_000
        connection.requestMethod = "GET"
        connection.setRequestProperty("Accept", "application/vnd.github+json")
        connection.setRequestProperty("User-Agent", "SonixLink-Android")
        try {
            if (connection.responseCode != HttpURLConnection.HTTP_OK) {
                error("GitHub returned HTTP ${connection.responseCode}")
            }
            val release = JSONObject(connection.inputStream.bufferedReader().use { it.readText() })
            val version = release.getString("tag_name").removePrefix("v")
            parseVersion(version) ?: error("The latest release has an invalid version tag")
            val assets = release.getJSONArray("assets")
            val apk = (0 until assets.length())
                .map(assets::getJSONObject)
                .firstOrNull { it.optString("name").endsWith(".apk", ignoreCase = true) }
                ?: error("The latest release does not include an APK")
            val apkUrl = apk.getString("browser_download_url")
            val downloadUri = Uri.parse(apkUrl)
            if (downloadUri.scheme != "https" || downloadUri.host != "github.com" ||
                !downloadUri.path.orEmpty().startsWith("/$REPOSITORY/releases/download/")
            ) {
                error("The release APK URL is not trusted")
            }
            return AppRelease(version, apkUrl, release.optString("body"))
        } finally {
            connection.disconnect()
        }
    }

    private fun parseVersion(version: String): List<Int>? {
        val match = Regex("^v?(\\d+(?:\\.\\d+)*)(?:[-+].*)?$").matchEntire(version) ?: return null
        return match.groupValues[1].split('.').map { it.toIntOrNull() ?: return null }
    }

    private fun compareVersions(left: String, right: String): Int {
        val leftParts = parseVersion(left) ?: error("Invalid release version: $left")
        val rightParts = parseVersion(right) ?: error("Invalid installed version: $right")
        for (index in 0 until maxOf(leftParts.size, rightParts.size)) {
            val comparison = (leftParts.getOrElse(index) { 0 }).compareTo(rightParts.getOrElse(index) { 0 })
            if (comparison != 0) return comparison
        }
        return 0
    }

    private fun enqueueDownload(context: MainActivity, release: AppRelease) {
        try {
            val manager = context.getSystemService(DownloadManager::class.java)
            val request = DownloadManager.Request(Uri.parse(release.apkUrl))
                .setTitle(context.getString(R.string.update_downloading_title, release.version))
                .setDescription(context.getString(R.string.update_downloading_body))
                .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE)
                .setAllowedOverMetered(true)
                .setDestinationInExternalFilesDir(
                    context,
                    Environment.DIRECTORY_DOWNLOADS,
                    "sonixlink-${release.version}-${System.currentTimeMillis()}.apk",
                )
            val id = manager.enqueue(request)
            context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
                .edit()
                .putLong(DOWNLOAD_ID, id)
                .putString(DOWNLOAD_VERSION, release.version)
                .apply()
            monitorDownload(context, id)
        } catch (error: Exception) {
            showMessage(context, R.string.update_download_failed, error.localizedMessage ?: error.javaClass.simpleName)
        }
    }

    fun resumeDownload(activity: MainActivity) {
        if (activity.isFinishing || activity.isDestroyed) return
        val preferences = activity.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val id = preferences.getLong(DOWNLOAD_ID, -1L)
        if (id < 0) return
        val version = preferences.getString(DOWNLOAD_VERSION, "").orEmpty()
        if (version.isBlank() || compareVersions(BuildConfig.VERSION_NAME, version) >= 0) {
            clearDownload(activity, id)
            return
        }
        if (preferences.getLong(DISMISSED_READY_ID, -1L) == id) return
        monitorDownload(activity, id)
    }

    private fun monitorDownload(activity: MainActivity, id: Long) {
        val generation = ++monitorGeneration
        monitorJob?.cancel()
        progressDialog?.dismiss()
        val label = TextView(activity).apply {
            setPadding(0, 0, 0, dp(activity, 12))
        }
        val bar = ProgressBar(activity, null, android.R.attr.progressBarStyleHorizontal).apply {
            isIndeterminate = true
            max = 100
        }
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            val padding = dp(activity, 24)
            setPadding(padding, dp(activity, 8), padding, padding)
            addView(label, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT))
            addView(bar, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT))
        }
        progressLabel = label
        progressBar = bar
        progressDialog = MaterialAlertDialogBuilder(activity)
            .setTitle(R.string.update_progress_title)
            .setView(content)
            .setCancelable(true)
            .setNegativeButton(R.string.update_hide_progress, null)
            .show()

        monitorJob = activity.lifecycleScope.launch {
            try {
                while (true) {
                    val state = try {
                        withContext(Dispatchers.IO) { queryDownload(activity, id) }
                    } catch (cancelled: CancellationException) {
                        throw cancelled
                    } catch (error: Exception) {
                        closeProgress()
                        if (isForeground(activity)) clearDownload(activity, id)
                        showMessage(
                            activity,
                            R.string.update_download_failed,
                            error.localizedMessage ?: error.javaClass.simpleName,
                        )
                        return@launch
                    }
                    if (state == null) {
                        closeProgress()
                        if (isForeground(activity)) clearDownload(activity, id)
                        showMessage(activity, R.string.update_download_failed, activity.getString(R.string.update_download_status_missing))
                        return@launch
                    }
                    when (state.status) {
                        DownloadManager.STATUS_PENDING,
                        DownloadManager.STATUS_RUNNING,
                        DownloadManager.STATUS_PAUSED -> {
                            updateProgress(activity, state)
                            delay(750)
                        }
                        DownloadManager.STATUS_SUCCESSFUL -> {
                            closeProgress()
                            showInstallReady(activity, id)
                            return@launch
                        }
                        DownloadManager.STATUS_FAILED -> {
                            closeProgress()
                            if (isForeground(activity)) clearDownload(activity, id)
                            showMessage(
                                activity,
                                R.string.update_download_failed,
                                activity.getString(R.string.update_download_error_code, state.reason),
                            )
                            return@launch
                        }
                        else -> delay(750)
                    }
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } finally {
                if (monitorGeneration == generation) {
                    monitorJob = null
                    closeProgress()
                }
            }
        }
    }

    private fun queryDownload(context: Context, id: Long): DownloadState? {
        val manager = context.getSystemService(DownloadManager::class.java)
        return manager.query(DownloadManager.Query().setFilterById(id))?.use { cursor ->
            if (!cursor.moveToFirst()) return null
            DownloadState(
                status = cursor.getInt(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_STATUS)),
                downloaded = cursor.getLong(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_BYTES_DOWNLOADED_SO_FAR)),
                total = cursor.getLong(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_TOTAL_SIZE_BYTES)),
                reason = cursor.getInt(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_REASON)),
            )
        }
    }

    private fun updateProgress(activity: MainActivity, state: DownloadState) {
        val label = progressLabel ?: return
        val bar = progressBar ?: return
        if (state.total > 0) {
            val percent = ((state.downloaded * 100L) / state.total).toInt().coerceIn(0, 100)
            bar.isIndeterminate = false
            bar.progress = percent
            label.text = activity.getString(
                R.string.update_progress,
                percent,
                formatBytes(state.downloaded),
                formatBytes(state.total),
            )
        } else {
            bar.isIndeterminate = true
            label.setText(R.string.update_progress_waiting)
        }
        if (state.status == DownloadManager.STATUS_PAUSED) {
            label.append("\n")
            label.append(activity.getString(R.string.update_download_paused))
        }
    }

    private fun showInstallReady(activity: MainActivity, id: Long) {
        if (!isForeground(activity)) return
        MaterialAlertDialogBuilder(activity)
            .setTitle(R.string.update_ready_title)
            .setMessage(R.string.update_ready_body)
            .setCancelable(false)
            .setNegativeButton(R.string.update_later) { _, _ ->
                activity.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
                    .edit()
                    .putLong(DISMISSED_READY_ID, id)
                    .apply()
            }
            .setPositiveButton(R.string.update_install) { _, _ ->
                installDownloadedApk(activity, id)
            }
            .show()
    }

    private fun installDownloadedApk(context: Context, id: Long) {
        val manager = context.getSystemService(DownloadManager::class.java)
        val apkUri = manager.getUriForDownloadedFile(id)
        if (apkUri == null) {
            showMessage(context, R.string.update_download_failed, context.getString(R.string.update_download_status_missing))
            return
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !context.packageManager.canRequestPackageInstalls()) {
            MaterialAlertDialogBuilder(context)
                .setMessage(R.string.update_allow_install)
                .setNegativeButton(android.R.string.cancel, null)
                .setPositiveButton(R.string.update_open_settings) { _, _ ->
                    context.startActivity(
                        Intent(
                            android.provider.Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                            Uri.parse("package:${context.packageName}"),
                        ),
                    )
                }
                .show()
            return
        }
        val installIntent = Intent(Intent.ACTION_VIEW)
            .setDataAndType(apkUri, APK_MIME)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        try {
            context.startActivity(installIntent)
        } catch (error: Exception) {
            showMessage(context, R.string.update_install_failed, error.localizedMessage ?: error.javaClass.simpleName)
        }
    }

    private fun closeProgress() {
        progressDialog?.dismiss()
        progressDialog = null
        progressLabel = null
        progressBar = null
    }

    private fun clearDownload(context: Context, id: Long) {
        val preferences = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        if (preferences.getLong(DOWNLOAD_ID, -1L) == id) {
            preferences.edit()
                .remove(DOWNLOAD_ID)
                .remove(DOWNLOAD_VERSION)
                .remove(DISMISSED_READY_ID)
                .apply()
        }
        context.getSystemService(DownloadManager::class.java).remove(id)
    }

    private fun showMessage(context: Context, title: Int, message: String) {
        if (context !is MainActivity || !isForeground(context)) return
        MaterialAlertDialogBuilder(context)
            .setTitle(title)
            .setMessage(message)
            .setPositiveButton(android.R.string.ok, null)
            .show()
    }

    private fun isForeground(activity: MainActivity): Boolean =
        !activity.isFinishing && !activity.isDestroyed &&
            activity.lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)

    private fun dp(context: Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()

    private fun formatBytes(bytes: Long): String {
        if (bytes < 1024) return "$bytes B"
        val kib = bytes / 1024.0
        if (kib < 1024) return "%.1f KB".format(kib)
        return "%.1f MB".format(kib / 1024)
    }

    internal fun onDownloadComplete(context: Context, intent: Intent) {
        if (intent.action != DownloadManager.ACTION_DOWNLOAD_COMPLETE) return
        val id = intent.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1L)
        val expected = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getLong(DOWNLOAD_ID, -1L)
        if (id < 0 || id != expected) return

        val manager = context.getSystemService(DownloadManager::class.java)
        val query = DownloadManager.Query().setFilterById(id)
        val state = manager.query(query)?.use { cursor ->
            if (!cursor.moveToFirst()) return
            DownloadState(
                cursor.getInt(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_STATUS)),
                cursor.getLong(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_BYTES_DOWNLOADED_SO_FAR)),
                cursor.getLong(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_TOTAL_SIZE_BYTES)),
                cursor.getInt(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_REASON)),
            )
        } ?: return

        if (state.status != DownloadManager.STATUS_SUCCESSFUL) {
            showDownloadNotification(
                context,
                context.getString(R.string.update_download_failed),
                context.getString(R.string.update_download_error_code, state.reason),
                null,
            )
            return
        }
        val apkUri = manager.getUriForDownloadedFile(id) ?: return

        val installIntent = Intent(Intent.ACTION_VIEW)
            .setDataAndType(apkUri, APK_MIME)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
        val pendingIntent = PendingIntent.getActivity(
            context,
            NOTIFICATION_ID,
            installIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        showDownloadNotification(
            context,
            context.getString(R.string.update_ready_title),
            context.getString(R.string.update_ready_body),
            pendingIntent,
        )
    }

    private fun showDownloadNotification(
        context: Context,
        title: String,
        message: String,
        pendingIntent: PendingIntent?,
    ) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = android.app.NotificationChannel(
                CHANNEL,
                context.getString(R.string.update_notification_channel),
                android.app.NotificationManager.IMPORTANCE_DEFAULT,
            )
            context.getSystemService(android.app.NotificationManager::class.java).createNotificationChannel(channel)
        }
        if (Build.VERSION.SDK_INT < 33 ||
            ContextCompat.checkSelfPermission(context, android.Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            val notification = NotificationCompat.Builder(context, CHANNEL)
                .setSmallIcon(R.drawable.ic_notification)
                .setContentTitle(title)
                .setContentText(message)
                .setAutoCancel(pendingIntent != null)
                .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            if (pendingIntent != null) notification.setContentIntent(pendingIntent)
            NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, notification.build())
        }
    }
}

class AppUpdateDownloadReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        AppUpdater.onDownloadComplete(context, intent)
    }
}
