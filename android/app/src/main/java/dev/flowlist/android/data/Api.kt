package dev.flowlist.android.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.util.concurrent.TimeUnit

class ApiException(message: String) : IOException(message)

class Api(
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(6, TimeUnit.SECONDS)
        .readTimeout(40, TimeUnit.SECONDS)
        .writeTimeout(15, TimeUnit.SECONDS)
        .build(),
) {
    private val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
        encodeDefaults = true
    }
    private val mediaType = "application/json; charset=utf-8".toMediaType()

    suspend fun info(base: String): Long = withContext(Dispatchers.IO) {
        val response = execute(Request.Builder().url("${base.trimEnd('/')}/api/v1/info").get().build())
        val value = json.parseToJsonElement(response).let { it as JsonObject }
        value["revision"]?.toString()?.trim('"')?.toLongOrNull() ?: 0L
    }

    suspend fun pair(base: String, code: String, deviceName: String): PairResponse =
        withContext(Dispatchers.IO) {
            val body = buildJsonObject {
                put("code", code.trim())
                put("device_name", deviceName)
            }.toString()
            val text = execute(
                Request.Builder()
                    .url("${base.trimEnd('/')}/api/v1/pair")
                    .post(body.toRequestBody(mediaType))
                    .build(),
            )
            json.decodeFromString<PairResponse>(text)
        }

    suspend fun snapshot(base: String, token: String): Snapshot = withContext(Dispatchers.IO) {
        val text = execute(
            Request.Builder()
                .url("${base.trimEnd('/')}/api/v1/snapshot")
                .header("Authorization", "Bearer $token")
                .get()
                .build(),
        )
        json.decodeFromString<Snapshot>(text)
    }

    suspend fun sendOps(
        base: String,
        token: String,
        ops: List<PendingOp>,
        baseRevision: Long,
    ): OpsResponse = withContext(Dispatchers.IO) {
        val payload = buildJsonObject {
            put("base_revision", baseRevision)
            put("snapshot", true)
            put(
                "ops",
                buildJsonArray {
                    ops.forEach { op ->
                        add(
                            buildJsonObject {
                                put("id", op.id)
                                op.text?.let { put("text", it) }
                                op.taskId?.let { put("task_id", it) }
                                op.command?.let { put("command", it) }
                            },
                        )
                    }
                },
            )
        }.toString()
        val text = execute(
            Request.Builder()
                .url("${base.trimEnd('/')}/api/v1/ops")
                .header("Authorization", "Bearer $token")
                .post(payload.toRequestBody(mediaType))
                .build(),
        )
        json.decodeFromString<OpsResponse>(text)
    }

    private fun execute(request: Request): String {
        client.newCall(request).execute().use { response ->
            val text = response.body?.string().orEmpty()
            if (!response.isSuccessful) {
                val message = runCatching {
                    json.decodeFromString<ApiErrorResponse>(text).error
                }.getOrElse { "HTTP ${response.code}" }
                throw ApiException(message)
            }
            return text
        }
    }
}

fun normalizeBase(input: String): String? {
    var value = input.trim()
    if (value.isEmpty()) return null
    if (!value.startsWith("http://") && !value.startsWith("https://")) {
        value = "http://$value"
    }
    value = value.trimEnd('/')
    val withoutScheme = value.substringAfter("://")
    if (!withoutScheme.contains(':')) return null
    val port = withoutScheme.substringAfterLast(':')
    if (port.toIntOrNull() == null) return null
    return value
}

fun completeCommand(ids: List<String>, completed: Boolean): JsonObject = buildJsonObject {
    put(
        "complete_tasks",
        buildJsonObject {
            put(
                "ids",
                buildJsonArray { ids.forEach { add(kotlinx.serialization.json.JsonPrimitive(it)) } },
            )
            put("completed", completed)
        },
    )
}

fun deleteCommand(ids: List<String>): JsonObject = buildJsonObject {
    put(
        "delete_tasks",
        buildJsonObject {
            put(
                "ids",
                buildJsonArray { ids.forEach { add(kotlinx.serialization.json.JsonPrimitive(it)) } },
            )
        },
    )
}

fun patchCommand(id: String, patch: JsonObject): JsonObject = buildJsonObject {
    put("update_task", buildJsonObject { put("id", id); put("patch", patch) })
}

fun jsonString(value: String?): kotlinx.serialization.json.JsonElement? =
    value?.let { kotlinx.serialization.json.JsonPrimitive(it) }
