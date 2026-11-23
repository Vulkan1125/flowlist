package dev.flowlist.android.data

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import java.util.Collections
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request

/// Loads a site's favicon without a third-party service. Many sites still serve
/// an ICO, which Android's BitmapFactory cannot read, so the ICO directory is
/// walked and its embedded PNG entries are decoded directly.
object FaviconLoader {
    private val client = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(5, TimeUnit.SECONDS)
        .build()
    private val cache = ConcurrentHashMap<String, Bitmap>()
    private val misses = Collections.newSetFromMap(ConcurrentHashMap<String, Boolean>())

    suspend fun load(path: String): Bitmap? = withContext(Dispatchers.IO) {
        cache[path]?.let { return@withContext it }
        if (path in misses) return@withContext null
        val loaded = faviconUrls(path).firstNotNullOfOrNull { url ->
            runCatching {
                val request = Request.Builder()
                    .url(url)
                    .header(
                        "User-Agent",
                        "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 " +
                            "(KHTML, like Gecko) Chrome/120 Mobile Safari/537.36",
                    )
                    .build()
                client.newCall(request).execute().use { response ->
                    if (!response.isSuccessful) return@use null
                    response.body?.bytes()?.let(::decode)
                }
            }.getOrNull()
        }
        if (loaded != null) cache[path] = loaded else misses.add(path)
        loaded
    }

    private fun decode(bytes: ByteArray): Bitmap? {
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size)?.let { return it }
        if (bytes.size < 6) return null
        val reserved = u16(bytes, 0)
        val type = u16(bytes, 2)
        if (reserved != 0 || type != 1) return null
        val count = u16(bytes, 4)
        for (index in 0 until count) {
            val entry = 6 + index * 16
            if (entry + 16 > bytes.size) break
            val size = u32(bytes, entry + 8)
            val offset = u32(bytes, entry + 12)
            if (size <= 0 || offset < 0 || offset + size > bytes.size) continue
            val slice = bytes.copyOfRange(offset, offset + size)
            if (slice.size < 8) continue
            val png = slice[0] == 0x89.toByte() &&
                slice[1] == 0x50.toByte() &&
                slice[2] == 0x4E.toByte() &&
                slice[3] == 0x47.toByte()
            if (png) BitmapFactory.decodeByteArray(slice, 0, slice.size)?.let { return it }
        }
        return null
    }

    private fun u16(bytes: ByteArray, at: Int): Int =
        (bytes[at].toInt() and 0xFF) or ((bytes[at + 1].toInt() and 0xFF) shl 8)

    private fun u32(bytes: ByteArray, at: Int): Int =
        (bytes[at].toInt() and 0xFF) or
            ((bytes[at + 1].toInt() and 0xFF) shl 8) or
            ((bytes[at + 2].toInt() and 0xFF) shl 16) or
            ((bytes[at + 3].toInt() and 0xFF) shl 24)
}
