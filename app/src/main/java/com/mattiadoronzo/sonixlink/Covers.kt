package com.mattiadoronzo.sonixlink

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.os.Handler
import android.os.Looper
import android.util.LruCache
import java.io.File
import java.nio.ByteBuffer
import java.util.Locale

/**
 * The player's thumbnails and album art, fetched as needed.
 *
 * Supports both pre-rendered thumbnails from the player (/api/thumbs)
 * and direct artwork extraction (/api/art) for tracks/albums that haven't been
 * browsed on the device screen yet.
 */
class Covers(context: Context) {

    companion object {
        /** THUMB_SIZE from the player's lists (medialist.c, browser.c). */
        const val BOX = 72
        /** Compact profile box size (TempoTec V1 uses bp_pick(36, 72) = 36). */
        const val BOX_V1 = 36

        /** Keys in one request: a screenful and a bit, about 300 KB. Over Bluetooth, PER_ASK_SLOW. */
        private const val PER_ASK = 32
        private const val PER_ASK_SLOW = 8

        /** The pause between two requests over a slow link. */
        private const val SLOW_PAUSE_MS = 150L

        /** Most keys waiting at once; past this the oldest are dropped. */
        private const val WANTED_MAX = 160
        private const val ART_WANTED_MAX = 80

        /** After a failed request, the wait before the next. */
        private const val BACKOFF_MS = 3000L

        /** The same FNV-1a as thumb_hash() in src/gui/nowplaying/cover.c. */
        fun key(
            path: String,
            mtime: Long,
            size: Long,
            box: Int = BOX,
            isDir: Boolean = false,
            epoch: Boolean = false,
        ): String {
            val ep = if (epoch || isDir || path.endsWith(".wav", ignoreCase = true)) "|w2" else ""
            val material = "$path|$box|${if (isDir) 'd' else 'f'}|$mtime|$size$ep"
            var h = 1469598103934665603L
            for (b in material.toByteArray(Charsets.UTF_8)) {
                h = h xor (b.toLong() and 0xFF)
                h *= 1099511628211L
            }
            return String.format(Locale.US, "%016x", h)
        }

        fun candidateKeys(path: String, mtime: Long, size: Long, isDir: Boolean = false): List<String> {
            val list = ArrayList<String>(4)
            val isWav = isDir || path.endsWith(".wav", ignoreCase = true)
            list.add(key(path, mtime, size, box = BOX_V1, isDir = isDir, epoch = false))
            list.add(key(path, mtime, size, box = BOX, isDir = isDir, epoch = false))
            if (isWav) {
                list.add(key(path, mtime, size, box = BOX_V1, isDir = isDir, epoch = true))
                list.add(key(path, mtime, size, box = BOX, isDir = isDir, epoch = true))
            }
            return list
        }

        private val main = Handler(Looper.getMainLooper())
        private val watchers = LinkedHashSet<() -> Unit>()

        /**
         * Registers [watcher] to run on the main thread when thumbnails arrive.
         * Pair with [unwatch]: a watcher holds a reference to its screen.
         */
        fun watch(watcher: () -> Unit) {
            watchers.add(watcher)
        }

        fun unwatch(watcher: () -> Unit) {
            watchers.remove(watcher)
        }

        private fun announce() {
            main.post { watchers.toList().forEach { it() } }
        }
    }

    private data class ArtTask(val path: String, val keys: List<String>, val album: String)

    private val file = File(context.filesDir, "thumbnails.db")
    private val keysFile = File(context.filesDir, "thumbkeys.txt")
    private var db: SQLiteDatabase? = null

    // Guards the database and the state below: the main thread reads, the
    // fetcher writes, and syncing closes and reopens the file. Closing a
    // database mid-query throws IllegalStateException, not SQLiteException.
    private val gate = Any()

    private val cache = object : LruCache<String, Bitmap>(400) {
        override fun sizeOf(key: String, value: Bitmap): Int = 1
    }

    /** Keys already looked up and not found, or with no artwork. */
    private val misses = HashSet<String>()

    /**
     * The keys the player holds, with a picture or with "no artwork". Null
     * when the player cannot send the list (older firmware): nothing is
     * fetched, and only stored thumbnails are shown.
     */
    private var remote: Set<String>? = null

    /** Keys waiting to be fetched, newest last, and the ones on their way. */
    private val wanted = LinkedHashSet<String>()
    private val inFlight = HashSet<String>()

    private val artWanted = LinkedHashSet<ArtTask>()
    private val artInFlight = HashSet<String>()

    private var fetcher: Thread? = null
    private var stopped = false

    /** Fetches thumbnails for a set of keys; set by the owner of the player's client. */
    @Volatile
    var source: ((Collection<String>) -> List<Thumb>)? = null

    /** Fetches single artwork directly from track path; set by the owner of the player's client. */
    @Volatile
    var artSource: ((String) -> ByteArray?)? = null

    /**
     * Over Bluetooth: smaller requests with a pause between them, so state
     * reads and commands can get through and the headphones' audio, on the
     * same radio, is not starved.
     */
    @Volatile
    var slowLink = false

    /**
     * Album to the key of the track that stands in as its cover.
     */
    private var albumKeys: Map<String, String> = emptyMap()
    private var albumTracks: Map<String, Row> = emptyMap()

    val databaseFile: File get() = file

    /** Opens the local store, creating it the first time. */
    fun open(): Boolean = synchronized(gate) {
        closeLocked()
        try {
            val database = SQLiteDatabase.openDatabase(
                file.absolutePath, null,
                SQLiteDatabase.OPEN_READWRITE or SQLiteDatabase.CREATE_IF_NECESSARY or
                    SQLiteDatabase.NO_LOCALIZED_COLLATORS,
            )
            // The player's own schema, so a whole copy of its thumbnail file
            // is readable as is.
            database.execSQL(
                "CREATE TABLE IF NOT EXISTS thumbs(key TEXT PRIMARY KEY, w INTEGER NOT NULL, " +
                    "h INTEGER NOT NULL, pixels BLOB)"
            )
            db = database
            stopped = false
            true
        } catch (e: Exception) {
            db = null
            false
        }
    }

    fun close() {
        synchronized(gate) {
            stopped = true
            wanted.clear()
            artWanted.clear()
            (gate as java.lang.Object).notifyAll()
            closeLocked()
        }
    }

    private fun closeLocked() {
        try {
            db?.close()
        } catch (e: Exception) {
            // Already closed.
        }
        db = null
        cache.evictAll()
        misses.clear()
        albumKeys = emptyMap()
        albumTracks = emptyMap()
    }

    // --- The player's list of keys ---

    /** The list as the player last sent it, from disk. False when there is none. */
    fun loadRemoteKeys(): Boolean {
        val keys = try {
            if (!keysFile.exists()) return false
            keysFile.readLines().filter { it.length == 16 }.toHashSet()
        } catch (e: Exception) {
            return false
        }
        synchronized(gate) {
            remote = keys
            misses.clear()
        }
        return true
    }

    /** A fresh list from the player, kept on disk for the next connection. */
    fun setRemoteKeys(keys: Set<String>) {
        try {
            val temporary = File(keysFile.parentFile, keysFile.name + ".part")
            temporary.writeText(keys.joinToString("\n"))
            if (!temporary.renameTo(keysFile)) temporary.delete()
        } catch (e: Exception) {
            // Not saved: the list is asked for again at the next connection.
        }
        synchronized(gate) {
            remote = keys
            misses.clear()
        }
    }

    /** The player cannot send a list: nothing will be fetched. */
    fun clearRemoteKeys() {
        synchronized(gate) { remote = null }
    }

    /**
     * Fills the album covers from [rows]: maps each album to its representative track.
     * Call off the main thread.
     */
    fun indexAlbums(rows: List<Row>) {
        val present = HashSet(localKeys())
        synchronized(gate) { remote?.let { present.addAll(it) } }
        val map = HashMap<String, String>()
        val tracks = HashMap<String, Row>()
        for (row in rows) {
            if (row.album.isEmpty()) continue
            if (row.album !in tracks && row.artPath.isNotEmpty()) {
                tracks[row.album] = row
            }
            if (row.album in map) continue
            if (row.artPath.isEmpty() || row.artMtime <= 0 || row.artSize <= 0) continue
            val candidates = candidateKeys(row.artPath, row.artMtime, row.artSize)
            val matched = candidates.firstOrNull { it in present }
            if (matched != null) {
                map[row.album] = matched
            }
        }
        for ((album, row) in tracks) {
            if (album !in map && row.artMtime > 0 && row.artSize > 0) {
                map[album] = key(row.artPath, row.artMtime, row.artSize, box = BOX_V1)
            }
        }
        synchronized(gate) {
            albumKeys = map
            albumTracks = tracks
        }
    }

    /** Every key with a picture already here: one read, then comparisons. */
    private fun localKeys(): Set<String> = synchronized(gate) {
        val database = db ?: return@synchronized emptySet()
        val out = HashSet<String>()
        try {
            database.rawQuery("SELECT key FROM thumbs WHERE w > 0 AND h > 0", null).use { cursor ->
                while (cursor.moveToNext()) {
                    out.add(cursor.getString(0) ?: continue)
                }
            }
        } catch (e: Exception) {
            return@synchronized emptySet()
        }
        out
    }

    // --- Reading ---

    /** An album's cover, taken from the track that has one. */
    fun forAlbum(album: String): Bitmap? {
        if (album.isEmpty()) return null
        val assignedKey = synchronized(gate) { albumKeys[album] }
        if (assignedKey != null) {
            val bmp = forCandidates(listOf(assignedKey), fallbackPath = "", album = album)
            if (bmp != null) return bmp
        }
        val track = synchronized(gate) { albumTracks[album] } ?: return null
        val candidates = candidateKeys(track.artPath, track.artMtime, track.artSize)
        return forCandidates(candidates, fallbackPath = track.artPath, album = album)
    }

    /** A track's thumbnail, or null until it is here (it is then fetched). */
    fun forTrack(path: String, mtime: Long, size: Long, album: String = ""): Bitmap? {
        if (path.isEmpty()) return null
        val candidates = candidateKeys(path, mtime, size)
        return forCandidates(candidates, fallbackPath = path, album = album)
    }

    private fun forCandidates(keys: List<String>, fallbackPath: String, album: String): Bitmap? = synchronized(gate) {
        for (k in keys) {
            val cached = cache.get(k)
            if (cached != null) return@synchronized cached
        }
        if (keys.all { it in misses }) return@synchronized null

        val database = db ?: return@synchronized null
        var found: Bitmap? = null
        var matchedKey: String? = null
        var stored = false

        for (k in keys) {
            try {
                database.rawQuery("SELECT w,h,pixels FROM thumbs WHERE key=? LIMIT 1", arrayOf(k)).use { cursor ->
                    if (cursor.moveToFirst()) {
                        stored = true
                        matchedKey = k
                        found = bitmapOf(cursor.getInt(0), cursor.getInt(1), cursor.getBlob(2))
                    }
                }
            } catch (e: Exception) {
                return@synchronized null
            }
            if (found != null || stored) break
        }

        val bitmap = found
        when {
            bitmap != null && matchedKey != null -> {
                cache.put(matchedKey, bitmap)
                for (k in keys) cache.put(k, bitmap)
            }
            stored -> {
                for (k in keys) miss(k)
            }
            else -> {
                val remoteKey = keys.firstOrNull { remote?.contains(it) == true }
                if (remoteKey != null) {
                    want(remoteKey)
                } else if (fallbackPath.isNotEmpty()) {
                    wantArt(fallbackPath, keys, album)
                } else {
                    for (k in keys) miss(k)
                }
            }
        }
        bitmap
    }

    /** w and h at zero: the player has looked and found nothing. The rest is RGB565. */
    private fun bitmapOf(w: Int, h: Int, pixels: ByteArray?): Bitmap? {
        if (w <= 0 || h <= 0 || pixels == null || pixels.size < w * h * 2) return null
        val bitmap = Bitmap.createBitmap(w, h, Bitmap.Config.RGB_565)
        bitmap.copyPixelsFromBuffer(ByteBuffer.wrap(pixels, 0, w * h * 2))
        return bitmap
    }

    private fun miss(key: String) {
        // Bounded: a large library without artwork would grow it without limit.
        if (misses.size > 4000) {
            misses.clear()
        }
        misses.add(key)
    }

    // --- Fetching ---

    private fun ensureFetcherRunning() {
        if (fetcher == null && !stopped) {
            fetcher = Thread({ fetchLoop() }, "thumbnails").apply {
                isDaemon = true
                start()
            }
        }
        (gate as java.lang.Object).notifyAll()
    }

    /** Under the gate. The newest request goes last, which is where the fetcher takes from. */
    private fun want(key: String) {
        if (stopped || key in inFlight) return
        wanted.remove(key)
        wanted.add(key)
        while (wanted.size > WANTED_MAX) {
            wanted.remove(wanted.first())
        }
        ensureFetcherRunning()
    }

    private fun wantArt(path: String, keys: List<String>, album: String) {
        if (stopped || path in artInFlight || keys.any { it in inFlight }) return
        val task = ArtTask(path, keys, album)
        artWanted.remove(task)
        artWanted.add(task)
        while (artWanted.size > ART_WANTED_MAX) {
            artWanted.remove(artWanted.first())
        }
        ensureFetcherRunning()
    }

    /** Processes thumbnail batches from player and on-demand artwork requests. */
    private fun fetchLoop() {
        while (true) {
            var batch: List<String> = emptyList()
            var artTask: ArtTask? = null

            synchronized(gate) {
                while (!stopped && (wanted.isEmpty() || source == null) && (artWanted.isEmpty() || artSource == null)) {
                    try {
                        (gate as java.lang.Object).wait(5000)
                    } catch (e: InterruptedException) {
                        fetcher = null
                        return
                    }
                }
                if (stopped) {
                    fetcher = null
                    return
                }

                if (wanted.isNotEmpty() && source != null) {
                    val newest = wanted.toList().takeLast(if (slowLink) PER_ASK_SLOW else PER_ASK)
                    wanted.removeAll(newest.toSet())
                    inFlight.addAll(newest)
                    batch = newest
                } else if (artWanted.isNotEmpty() && artSource != null) {
                    val newest = artWanted.last()
                    artWanted.remove(newest)
                    artInFlight.add(newest.path)
                    artTask = newest
                }
            }

            if (batch.isNotEmpty()) {
                val fetch = source
                val thumbs = try {
                    fetch?.invoke(batch)
                } catch (e: Exception) {
                    null
                }

                synchronized(gate) {
                    inFlight.removeAll(batch.toSet())
                    if (thumbs != null) {
                        store(thumbs)
                        val answered = thumbs.mapTo(HashSet()) { it.key }
                        batch.filter { it !in answered }.forEach { miss(it) }
                    }
                }
                if (thumbs == null) {
                    Thread.sleep(BACKOFF_MS)
                } else {
                    if (thumbs.isNotEmpty()) announce()
                    if (slowLink) Thread.sleep(SLOW_PAUSE_MS)
                }
            } else if (artTask != null) {
                val fetchArt = artSource
                val task = artTask
                val bytes = try {
                    fetchArt?.invoke(task.path)
                } catch (e: Exception) {
                    null
                }

                var success = false
                synchronized(gate) {
                    artInFlight.remove(task.path)
                    if (bytes != null && bytes.isNotEmpty()) {
                        val bitmap = decodeThumbnail(bytes, 144)
                        if (bitmap != null) {
                            storeBitmap(task.keys, bitmap)
                            if (task.album.isNotEmpty()) {
                                task.keys.firstOrNull()?.let { k ->
                                    val updated = HashMap(albumKeys)
                                    updated[task.album] = k
                                    albumKeys = updated
                                }
                            }
                            success = true
                        } else {
                            storeEmpty(task.keys)
                        }
                    } else if (bytes != null) {
                        storeEmpty(task.keys)
                    }
                }

                if (success) {
                    announce()
                    if (slowLink) Thread.sleep(SLOW_PAUSE_MS)
                } else {
                    if (slowLink) Thread.sleep(SLOW_PAUSE_MS)
                }
            }
        }
    }

    private fun decodeThumbnail(bytes: ByteArray, target: Int): Bitmap? {
        return try {
            val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
            if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null

            var sample = 1
            while (bounds.outWidth / (sample * 2) >= target && bounds.outHeight / (sample * 2) >= target) {
                sample *= 2
            }

            val opts = BitmapFactory.Options().apply {
                inSampleSize = sample
                inPreferredConfig = Bitmap.Config.RGB_565
            }
            val bmp = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, opts) ?: return null
            if (bmp.width == target && bmp.height == target) {
                bmp
            } else {
                val scaled = Bitmap.createScaledBitmap(bmp, target, target, true)
                if (scaled != bmp) bmp.recycle()
                scaled
            }
        } catch (e: Throwable) {
            null
        }
    }

    private fun storeBitmap(keys: List<String>, bitmap: Bitmap) {
        val database = db ?: return
        val rgb = if (bitmap.config == Bitmap.Config.RGB_565) bitmap else bitmap.copy(Bitmap.Config.RGB_565, false)
        val w = rgb.width
        val h = rgb.height
        val pixels = ByteArray(w * h * 2)
        val buf = ByteBuffer.wrap(pixels)
        rgb.copyPixelsToBuffer(buf)

        try {
            database.beginTransaction()
            try {
                for (k in keys) {
                    val values = ContentValues().apply {
                        put("key", k)
                        put("w", w)
                        put("h", h)
                        put("pixels", pixels)
                    }
                    database.insertWithOnConflict("thumbs", null, values, SQLiteDatabase.CONFLICT_REPLACE)
                }
                database.setTransactionSuccessful()
            } finally {
                database.endTransaction()
            }
        } catch (e: Exception) {
            // Not stored.
        }

        for (k in keys) {
            cache.put(k, rgb)
            misses.remove(k)
        }
    }

    private fun storeEmpty(keys: List<String>) {
        val database = db ?: return
        try {
            database.beginTransaction()
            try {
                for (k in keys) {
                    val values = ContentValues().apply {
                        put("key", k)
                        put("w", 0)
                        put("h", 0)
                        put("pixels", ByteArray(0))
                    }
                    database.insertWithOnConflict("thumbs", null, values, SQLiteDatabase.CONFLICT_REPLACE)
                }
                database.setTransactionSuccessful()
            } finally {
                database.endTransaction()
            }
        } catch (e: Exception) {
            // Not stored.
        }
        for (k in keys) {
            miss(k)
        }
    }

    /** Under the gate: into the store for good, and into memory for the next draw. */
    private fun store(thumbs: List<Thumb>) {
        val database = db ?: return
        try {
            database.beginTransaction()
            try {
                for (thumb in thumbs) {
                    val values = ContentValues().apply {
                        put("key", thumb.key)
                        put("w", thumb.w)
                        put("h", thumb.h)
                        put("pixels", thumb.pixels)
                    }
                    database.insertWithOnConflict("thumbs", null, values, SQLiteDatabase.CONFLICT_REPLACE)
                }
                database.setTransactionSuccessful()
            } finally {
                database.endTransaction()
            }
        } catch (e: Exception) {
            // Not stored: still cached below, and fetched again next session.
        }
        for (thumb in thumbs) {
            val bitmap = bitmapOf(thumb.w, thumb.h, thumb.pixels)
            if (bitmap != null) {
                cache.put(thumb.key, bitmap)
                misses.remove(thumb.key)
            } else {
                miss(thumb.key)
            }
        }
    }
}
