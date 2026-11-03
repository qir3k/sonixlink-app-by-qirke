package com.mattiadoronzo.sonixlink

import android.content.Context

/**
 * Preferences kept between launches: the last player that answered, the stamps
 * of the downloaded index and covers, the accent, and the notification prompt.
 */
object Settings {

    private const val FILE = "sonixlink"
    private const val KEY_HOST = "last_host"
    private const val KEY_PORT = "last_port"
    private const val KEY_DB_OWNER = "db_owner"
    private const val KEY_DB_VERSION = "db_version"
    private const val KEY_COVERS_OWNER = "covers_owner"
    private const val KEY_COVERS_VERSION = "covers_version"
    private const val KEY_ACCENT = "accent"
    private const val KEY_ACCENT_PREF = "accent_preference"
    private const val KEY_DEVICE_ACCENT = "device_accent"
    private const val KEY_ASKED_NOTIFICATIONS = "asked_notifications"

    const val ACCENT_PREF_DEVICE = "device"
    private const val KEY_TAB_ORDER = "tab_order"
    private const val KEY_DISABLED_TABS = "disabled_tabs"
    private const val KEY_FAV_MARK_PREF = "fav_mark_preference"
    private const val KEY_DEVICE_FAV_MARK = "device_fav_mark"

    const val FAV_MARK_PREF_DEVICE = "device"
    const val FAV_MARK_PREF_HEART = "heart"
    const val FAV_MARK_PREF_STAR = "star"

    val DEFAULT_TAB_ORDER: List<Section> = listOf(
        Section.TRACKS,
        Section.ALBUMS,
        Section.ARTISTS,
        Section.ALBUM_ARTISTS,
        Section.GENRES,
        Section.PLAYLISTS,
        Section.FAVOURITES,
        Section.FOLDERS,
    )

    fun lastHost(context: Context): Pair<String, Int>? {
        val preferences = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)
        val host = preferences.getString(KEY_HOST, null) ?: return null
        if (host.isBlank()) return null
        return host to preferences.getInt(KEY_PORT, PlayerClient.DEFAULT_PORT)
    }

    fun rememberHost(context: Context, host: String, port: Int) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_HOST, host)
            .putInt(KEY_PORT, port)
            .apply()
    }

    /**
     * The owner and version of the index on the phone: the player's serial
     * (identical over Wi-Fi and Bluetooth) and the index's fingerprint. A
     * mismatch with the player's means the index must be fetched again.
     */
    fun databaseStamp(context: Context): Pair<String, String> {
        val preferences = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)
        return preferences.getString(KEY_DB_OWNER, "").orEmpty() to preferences.getString(KEY_DB_VERSION, "").orEmpty()
    }

    fun rememberDatabaseStamp(context: Context, owner: String, version: String) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_DB_OWNER, owner)
            .putString(KEY_DB_VERSION, version)
            .apply()
    }

    fun coversStamp(context: Context): Pair<String, String> {
        val preferences = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)
        return preferences.getString(KEY_COVERS_OWNER, "").orEmpty() to
            preferences.getString(KEY_COVERS_VERSION, "").orEmpty()
    }

    fun rememberCoversStamp(context: Context, owner: String, version: String) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_COVERS_OWNER, owner)
            .putString(KEY_COVERS_VERSION, version)
            .apply()
    }

    /** The accent last seen, applied at startup before the player reports one. */
    fun accent(context: Context): String =
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).getString(KEY_ACCENT, "").orEmpty()

    fun rememberAccent(context: Context, accent: String) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_ACCENT, accent)
            .apply()
    }

    /**
     * User's accent preference: "device" (default) or a specific hex color like "#3584e4".
     */
    fun accentPreference(context: Context): String =
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).getString(KEY_ACCENT_PREF, ACCENT_PREF_DEVICE) ?: ACCENT_PREF_DEVICE

    fun rememberAccentPreference(context: Context, pref: String) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_ACCENT_PREF, pref)
            .apply()
    }

    /**
     * The accent reported by the connected device/player.
     */
    fun deviceAccent(context: Context): String =
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).getString(KEY_DEVICE_ACCENT, "").orEmpty()

    fun rememberDeviceAccent(context: Context, accentHex: String) {
        if (accentHex.isBlank()) return
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_DEVICE_ACCENT, accentHex)
            .putString(KEY_ACCENT, accentHex)
            .apply()
    }

    /**
     * Computes the effective accent color based on user preference and player state.
     */
    fun effectiveAccent(context: Context, incomingDeviceHex: String = ""): Int {
        val pref = accentPreference(context)
        if (pref != ACCENT_PREF_DEVICE) {
            return Session.parseAccent(pref)
        }
        val devHex = incomingDeviceHex.ifEmpty { deviceAccent(context).ifEmpty { accent(context) } }
        return if (devHex.isNotEmpty()) Session.parseAccent(devHex) else Session.DEFAULT_ACCENT
    }

    /** Whether the notification permission has been asked for already. */
    fun askedNotifications(context: Context): Boolean =
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).getBoolean(KEY_ASKED_NOTIFICATIONS, false)

    fun rememberAskedNotifications(context: Context) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putBoolean(KEY_ASKED_NOTIFICATIONS, true)
            .apply()
    }

    fun allTabsInOrder(context: Context): List<Section> {
        val raw = context.getSharedPreferences(FILE, Context.MODE_PRIVATE).getString(KEY_TAB_ORDER, null)
        if (raw.isNullOrBlank()) return DEFAULT_TAB_ORDER
        val saved = raw.split(",").mapNotNull { name ->
            try { Section.valueOf(name.trim()) } catch (e: Exception) { null }
        }.distinct()
        if (saved.isEmpty()) return DEFAULT_TAB_ORDER
        val missing = DEFAULT_TAB_ORDER.filter { it !in saved }
        return saved + missing
    }

    fun disabledTabs(context: Context): Set<Section> {
        val raw = context.getSharedPreferences(FILE, Context.MODE_PRIVATE).getString(KEY_DISABLED_TABS, null)
        if (raw.isNullOrBlank()) return emptySet()
        return raw.split(",").mapNotNull { name ->
            try { Section.valueOf(name.trim()) } catch (e: Exception) { null }
        }.toSet()
    }

    fun tabOrder(context: Context): List<Section> {
        val disabled = disabledTabs(context)
        val active = allTabsInOrder(context).filter { it !in disabled }
        return if (active.isNotEmpty()) active else listOf(Section.TRACKS)
    }

    fun rememberTabs(context: Context, sections: List<Section>, disabled: Set<Section>) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_TAB_ORDER, sections.joinToString(",") { it.name })
            .putString(KEY_DISABLED_TABS, disabled.joinToString(",") { it.name })
            .apply()
    }

    fun resetTabOrder(context: Context) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .remove(KEY_TAB_ORDER)
            .remove(KEY_DISABLED_TABS)
            .apply()
    }

    fun favMarkPreference(context: Context): String =
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE)
            .getString(KEY_FAV_MARK_PREF, FAV_MARK_PREF_DEVICE) ?: FAV_MARK_PREF_DEVICE

    fun rememberFavMarkPreference(context: Context, pref: String) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_FAV_MARK_PREF, pref)
            .apply()
    }

    fun deviceFavMark(context: Context): String =
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE)
            .getString(KEY_DEVICE_FAV_MARK, FAV_MARK_PREF_HEART) ?: FAV_MARK_PREF_HEART

    fun rememberDeviceFavMark(context: Context, mark: String) {
        if (mark.isBlank()) return
        val normalized = if (mark.equals("star", ignoreCase = true)) FAV_MARK_PREF_STAR else FAV_MARK_PREF_HEART
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .putString(KEY_DEVICE_FAV_MARK, normalized)
            .apply()
    }

    fun effectiveFavMark(context: Context): String {
        return when (favMarkPreference(context)) {
            FAV_MARK_PREF_STAR -> FAV_MARK_PREF_STAR
            FAV_MARK_PREF_HEART -> FAV_MARK_PREF_HEART
            else -> deviceFavMark(context)
        }
    }

    fun forgetHost(context: Context) {
        context.getSharedPreferences(FILE, Context.MODE_PRIVATE).edit()
            .remove(KEY_HOST)
            .remove(KEY_PORT)
            .apply()
    }
}
