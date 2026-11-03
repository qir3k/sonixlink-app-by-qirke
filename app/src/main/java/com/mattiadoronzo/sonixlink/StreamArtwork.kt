package com.mattiadoronzo.sonixlink

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.os.Handler
import android.os.Looper
import android.widget.ImageView
import java.io.ByteArrayOutputStream
import java.lang.ref.WeakReference
import java.net.HttpURLConnection
import java.net.URL
import java.util.LinkedHashMap
import java.util.concurrent.Executors

/** Small, bounded in-memory cache for the artwork URLs returned with stream rows. */
object StreamArtwork {

    private const val MAX_BYTES = 4 * 1024 * 1024
    private const val MAX_SIDE = 512
    private const val CACHE_SIZE = 48

    private data class Target(val image: WeakReference<ImageView>, val fallback: WeakReference<ImageView>)

    private val lock = Any()
    private val main = Handler(Looper.getMainLooper())
    private val workers = Executors.newFixedThreadPool(2)
    private val cache = object : LinkedHashMap<String, Bitmap?>(CACHE_SIZE, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, Bitmap?>?): Boolean = size > CACHE_SIZE
    }
    private val pending = HashMap<String, MutableList<Target>>()

    /** Binds a cover or leaves the service icon visible while it loads. */
    fun bind(image: ImageView, fallback: ImageView, address: String): Boolean {
        image.tag = address
        val cached: Bitmap?
        val present: Boolean
        synchronized(lock) {
            present = cache.containsKey(address)
            cached = cache[address]
            if (!present) {
                val targets = pending.getOrPut(address) { ArrayList() }
                targets.add(Target(WeakReference(image), WeakReference(fallback)))
                if (targets.size == 1) workers.execute { fetch(address) }
            }
        }
        if (present && cached != null) {
            image.setImageBitmap(cached)
            image.visibility = android.view.View.VISIBLE
            fallback.visibility = android.view.View.GONE
            return true
        }
        image.setImageDrawable(null)
        image.visibility = android.view.View.GONE
        fallback.visibility = android.view.View.VISIBLE
        return false
    }

    private fun fetch(address: String) {
        val bitmap = try {
            val url = URL(address)
            if (url.protocol != "http" && url.protocol != "https") {
                null
            } else {
                val connection = url.openConnection() as? HttpURLConnection
                if (connection == null) {
                    null
                } else {
                    connection.connectTimeout = 5000
                    connection.readTimeout = 5000
                    connection.instanceFollowRedirects = true
                    connection.setRequestProperty("Accept", "image/*")
                    try {
                        if (connection.responseCode !in 200..299) null
                        else {
                            val bytes = connection.inputStream.use { input ->
                                val output = ByteArrayOutputStream()
                                val buffer = ByteArray(8192)
                                var total = 0
                                while (true) {
                                    val count = input.read(buffer)
                                    if (count < 0) break
                                    total += count
                                    if (total > MAX_BYTES) return@use ByteArray(0)
                                    output.write(buffer, 0, count)
                                }
                                output.toByteArray()
                            }
                            if (bytes.isEmpty()) null else decode(bytes)
                        }
                    } finally {
                        connection.disconnect()
                    }
                }
            }
        } catch (_: Exception) {
            null
        }
        finish(address, bitmap)
    }

    private fun decode(bytes: ByteArray): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        var sample = 1
        while (bounds.outWidth / sample > MAX_SIDE * 2 || bounds.outHeight / sample > MAX_SIDE * 2) sample *= 2
        return BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply {
            inSampleSize = sample
            inPreferredConfig = Bitmap.Config.RGB_565
        })
    }

    private fun finish(address: String, bitmap: Bitmap?) {
        val targets = synchronized(lock) {
            cache[address] = bitmap
            pending.remove(address).orEmpty().toList()
        }
        main.post {
            for (target in targets) {
                val image = target.image.get() ?: continue
                if (image.tag != address) continue
                val fallback = target.fallback.get()
                if (bitmap == null) {
                    image.setImageDrawable(null)
                    image.visibility = android.view.View.GONE
                    fallback?.visibility = android.view.View.VISIBLE
                } else {
                    image.setImageBitmap(bitmap)
                    image.visibility = android.view.View.VISIBLE
                    fallback?.visibility = android.view.View.GONE
                }
            }
        }
    }
}
