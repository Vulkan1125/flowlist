package dev.flowlist.android.data

import dev.flowlist.android.BuildConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

data class UpdateInfo(
    val latest: String,
    val downloadUrl: String,
    val releaseUrl: String,
)

object UpdateChecker {
    private const val API = "https://api.github.com/repos/Vulkan1125/flowlist/releases/latest"
    private val client = OkHttpClient.Builder()
        .connectTimeout(8, TimeUnit.SECONDS)
        .readTimeout(12, TimeUnit.SECONDS)
        .build()
    private val json = Json { ignoreUnknownKeys = true }

    suspend fun check(): Result<UpdateInfo?> = withContext(Dispatchers.IO) {
        runCatching {
            val request = Request.Builder()
                .url(API)
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "Flowlist-Android")
                .get()
                .build()
            client.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    error("GitHub answered ${response.code}")
                }
                val release = json.decodeFromString<Release>(response.body?.string().orEmpty())
                if (!newer(release.tagName, BuildConfig.VERSION_NAME)) {
                    null
                } else {
                    val apk = release.assets.firstOrNull {
                        it.name.startsWith("Flowlist-Android-") && it.name.endsWith(".apk")
                    }
                    UpdateInfo(
                        latest = release.tagName.removePrefix("v"),
                        downloadUrl = apk?.url ?: release.htmlUrl,
                        releaseUrl = release.htmlUrl,
                    )
                }
            }
        }
    }

    private fun newer(candidate: String, current: String): Boolean {
        val left = candidate.removePrefix("v").split('.').mapNotNull { it.toIntOrNull() }
        val right = current.removePrefix("v").split('.').mapNotNull { it.toIntOrNull() }
        for (index in 0 until maxOf(left.size, right.size)) {
            val a = left.getOrElse(index) { 0 }
            val b = right.getOrElse(index) { 0 }
            if (a != b) return a > b
        }
        return false
    }
}

@Serializable
private data class Release(
    @SerialName("tag_name") val tagName: String = "",
    @SerialName("html_url") val htmlUrl: String = "",
    val assets: List<Asset> = emptyList(),
)

@Serializable
private data class Asset(
    val name: String = "",
    @SerialName("browser_download_url") val url: String = "",
)
