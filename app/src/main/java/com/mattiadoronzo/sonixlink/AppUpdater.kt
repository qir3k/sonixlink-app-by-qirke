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
import android.widget.Toast
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
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
    private const val CHANNEL = "app_updates"
    private const val NOTIFICATION_ID = 7402
    private const val APK_MIME = "application/vnd.android.package-archive"

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

    private fun enqueueDownload(context: Context, release: AppRelease) {
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
            .apply()
        Toast.makeText(context, R.string.update_download_started, Toast.LENGTH_LONG).show()
    }

    internal fun onDownloadComplete(context: Context, intent: Intent) {
        if (intent.action != DownloadManager.ACTION_DOWNLOAD_COMPLETE) return
        val id = intent.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1L)
        val expected = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getLong(DOWNLOAD_ID, -1L)
        if (id < 0 || id != expected) return

        val manager = context.getSystemService(DownloadManager::class.java)
        val query = DownloadManager.Query().setFilterById(id)
        val apkUri = manager.query(query)?.use { cursor ->
            if (!cursor.moveToFirst()) return
            if (cursor.getInt(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_STATUS)) !=
                DownloadManager.STATUS_SUCCESSFUL
            ) {
                context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().remove(DOWNLOAD_ID).apply()
                return
            }
            manager.getUriForDownloadedFile(id)
        } ?: return

        val installIntent = Intent(Intent.ACTION_INSTALL_PACKAGE)
            .setDataAndType(apkUri, APK_MIME)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
        val pendingIntent = PendingIntent.getActivity(
            context,
            NOTIFICATION_ID,
            installIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

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
            NotificationManagerCompat.from(context).notify(
                NOTIFICATION_ID,
                NotificationCompat.Builder(context, CHANNEL)
                    .setSmallIcon(R.drawable.ic_notification)
                    .setContentTitle(context.getString(R.string.update_ready_title))
                    .setContentText(context.getString(R.string.update_ready_body))
                    .setContentIntent(pendingIntent)
                    .setAutoCancel(true)
                    .setPriority(NotificationCompat.PRIORITY_DEFAULT)
                    .build(),
            )
        }
    }
}

class AppUpdateDownloadReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        AppUpdater.onDownloadComplete(context, intent)
    }
}
