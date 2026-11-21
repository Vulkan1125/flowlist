package dev.flowlist.android.data

import android.content.Context
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File

class SessionStore(context: Context) {
    private val prefs = context.getSharedPreferences("flowlist.session", Context.MODE_PRIVATE)

    var baseUrl: String?
        get() = prefs.getString("base_url", null)
        set(value) {
            prefs.edit().putString("base_url", value).apply()
        }

    var token: String?
        get() = prefs.getString("token", null)
        set(value) {
            prefs.edit().putString("token", value).apply()
        }

    var deviceName: String?
        get() = prefs.getString("device_name", null)
        set(value) {
            prefs.edit().putString("device_name", value).apply()
        }

    val paired: Boolean
        get() = !token.isNullOrEmpty() && !baseUrl.isNullOrEmpty()

    fun clear() {
        prefs.edit().clear().apply()
    }
}

@Serializable
data class PendingOp(
    val id: String,
    val text: String? = null,
    val taskId: String? = null,
    val command: kotlinx.serialization.json.JsonObject? = null,
    val label: String = "",
)

class FileStore(context: Context) {
    private val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
    }
    private val directory = File(context.filesDir, "flowlist").apply { mkdirs() }
    private val snapshotFile = File(directory, "snapshot.json")
    private val outboxFile = File(directory, "outbox.json")

    fun loadSnapshot(): Snapshot? = runCatching {
        if (!snapshotFile.exists()) return null
        json.decodeFromString<Snapshot>(snapshotFile.readText())
    }.getOrNull()

    fun saveSnapshot(snapshot: Snapshot) {
        write(snapshotFile, json.encodeToString(snapshot))
    }

    fun loadOutbox(): List<PendingOp> = runCatching {
        if (!outboxFile.exists()) return emptyList()
        json.decodeFromString<List<PendingOp>>(outboxFile.readText())
    }.getOrDefault(emptyList())

    fun saveOutbox(ops: List<PendingOp>) {
        write(outboxFile, json.encodeToString(ops))
    }

    fun clear() {
        snapshotFile.delete()
        outboxFile.delete()
    }

    private fun write(file: File, text: String) {
        val tmp = File(file.parentFile, "${file.name}.tmp")
        tmp.writeText(text)
        if (!tmp.renameTo(file)) {
            file.writeText(text)
            tmp.delete()
        }
    }
}
