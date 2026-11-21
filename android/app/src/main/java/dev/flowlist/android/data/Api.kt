package dev.flowlist.android.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
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

    suspend fun unpair(base: String, token: String) = withContext(Dispatchers.IO) {
        execute(
            Request.Builder()
                .url("${base.trimEnd('/')}/api/v1/unpair")
                .header("Authorization", "Bearer $token")
                .post("{}".toRequestBody(mediaType))
                .build(),
        )
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

fun updateCommand(id: String, patch: JsonObject): JsonObject = buildJsonObject {
    put("update_task", buildJsonObject { put("id", id); put("patch", patch) })
}

fun settingsCommand(patch: JsonObject): JsonObject = buildJsonObject {
    put("update_settings", patch)
}

fun repeatCommand(unit: String): JsonObject = buildJsonObject {
    put("mode", "due_again")
    put("schedule", "fixed")
    put("every", 1)
    put("unit", unit)
    put("end", buildJsonObject { put("kind", "never") })
    put("completed_count", 0)
}

fun deleteSeriesCommand(ids: List<String>): JsonObject = buildJsonObject {
    put(
        "delete_task_series",
        buildJsonObject {
            put("ids", buildJsonArray { ids.forEach { add(kotlinx.serialization.json.JsonPrimitive(it)) } })
        },
    )
}

fun reviewCommand(id: String, reviewedOn: String): JsonObject = buildJsonObject {
    put(
        "complete_review",
        buildJsonObject {
            put("id", id)
            put("reviewed_on", reviewedOn)
        },
    )
}

fun purgeCommand(ids: List<String>): JsonObject = buildJsonObject {
    put(
        "purge_tasks",
        buildJsonObject {
            put("ids", buildJsonArray { ids.forEach { add(kotlinx.serialization.json.JsonPrimitive(it)) } })
        },
    )
}

fun createSubtaskCommand(
    id: String,
    title: String,
    parentId: String,
    projectId: String?,
): JsonObject = buildJsonObject {
    put(
        "create_task",
        buildJsonObject {
            put("id", id)
            put("title", title)
            put("notes", "")
            put("project", projectId?.let { JsonPrimitive(it) } ?: JsonNull)
            put("project_path", JsonNull)
            put("parent", parentId)
            put("tags", buildJsonArray {})
            put("tag_paths", buildJsonArray {})
            put("due", JsonNull)
            put("defer", JsonNull)
            put("estimate_min", JsonNull)
            put("flagged", false)
            put("priority", JsonNull)
            put("attachments", buildJsonArray {})
            put("repeat", JsonNull)
            put("sequential", false)
            put("position", "end")
        },
    )
}

fun checklistCommand(
    taskId: String,
    itemId: String,
    title: String,
    completed: Boolean,
): JsonObject = buildJsonObject {
    put(
        "update_checklist_item",
        buildJsonObject {
            put("task", taskId)
            put("item", itemId)
            put("title", title)
            put("completed", completed)
        },
    )
}

fun setTagsCommand(id: String, added: List<String>, removed: List<String>): JsonObject = buildJsonObject {
    put(
        "set_task_tags",
        buildJsonObject {
            put("ids", buildJsonArray { add(kotlinx.serialization.json.JsonPrimitive(id)) })
            put("add", buildJsonArray { added.forEach { add(kotlinx.serialization.json.JsonPrimitive(it)) } })
            put(
                "remove",
                buildJsonArray { removed.forEach { add(kotlinx.serialization.json.JsonPrimitive(it)) } },
            )
        },
    )
}

fun stampJson(stamp: StampDto): JsonObject = buildJsonObject {
    put("at", stamp.at)
    put("has_time", stamp.hasTime)
}

fun restoreCommand(ids: List<String>): JsonObject = buildJsonObject {
    put(
        "restore_tasks",
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

fun createProjectCommand(name: String, folder: String?): JsonObject = buildJsonObject {
    put(
        "create_project",
        buildJsonObject {
            put("name", name)
            put("notes", "")
            put("folder", folder?.let { JsonPrimitive(it) } ?: JsonNull)
            put("tags", buildJsonArray {})
            put("due", JsonNull)
            put("defer", JsonNull)
            put("review_interval_days", JsonNull)
            put("next_review", JsonNull)
            put("sequential", false)
            put("kind", "parallel")
            put("icon", "project")
            put("attachments", buildJsonArray {})
            put("position", "end")
        },
    )
}

fun updateProjectCommand(id: String, patch: JsonObject): JsonObject = buildJsonObject {
    put("update_project", buildJsonObject { put("id", id); put("patch", patch) })
}

fun setProjectStatusCommand(id: String, status: String): JsonObject = buildJsonObject {
    put("set_project_status", buildJsonObject { put("id", id); put("status", status) })
}

fun deleteProjectCommand(id: String): JsonObject = buildJsonObject {
    put("delete_project", buildJsonObject { put("id", id) })
}

fun createFolderCommand(name: String, parent: String?): JsonObject = buildJsonObject {
    put(
        "create_folder",
        buildJsonObject {
            put("name", name)
            put("parent", parent?.let { JsonPrimitive(it) } ?: JsonNull)
            put("position", "end")
        },
    )
}

fun renameFolderCommand(id: String, name: String): JsonObject = buildJsonObject {
    put("rename_folder", buildJsonObject { put("id", id); put("name", name) })
}

fun moveFolderCommand(id: String, parent: String?): JsonObject = buildJsonObject {
    put(
        "move_folder",
        buildJsonObject {
            put("id", id)
            put(
                "placement",
                buildJsonObject {
                    put("parent", parent?.let { JsonPrimitive(it) } ?: JsonNull)
                    put("position", "end")
                },
            )
        },
    )
}

fun deleteFolderCommand(id: String): JsonObject = buildJsonObject {
    put("delete_folder", buildJsonObject { put("id", id) })
}

fun createTagCommand(name: String, parent: String?): JsonObject = buildJsonObject {
    put(
        "create_tag",
        buildJsonObject {
            put("name", name)
            put("parent", parent?.let { JsonPrimitive(it) } ?: JsonNull)
            put("status", "available")
            put("location", JsonNull)
            put("position", "end")
        },
    )
}

fun updateTagCommand(id: String, name: String, parent: String?): JsonObject = buildJsonObject {
    put(
        "update_tag",
        buildJsonObject {
            put("id", id)
            put(
                "patch",
                buildJsonObject {
                    put("name", name)
                    put("parent", parent?.let { JsonPrimitive(it) } ?: JsonNull)
                },
            )
        },
    )
}

fun deleteTagCommand(id: String): JsonObject = buildJsonObject {
    put("delete_tag", buildJsonObject { put("id", id) })
}

fun addChecklistCommand(taskId: String, title: String): JsonObject = buildJsonObject {
    put(
        "add_checklist_item",
        buildJsonObject {
            put("task", taskId)
            put("title", title)
            put("position", JsonNull)
        },
    )
}

fun deleteChecklistCommand(taskId: String, itemId: String): JsonObject = buildJsonObject {
    put(
        "delete_checklist_item",
        buildJsonObject {
            put("task", taskId)
            put("item", itemId)
        },
    )
}

fun moveChecklistCommand(taskId: String, itemId: String, to: Int): JsonObject = buildJsonObject {
    put(
        "move_checklist_item",
        buildJsonObject {
            put("task", taskId)
            put("item", itemId)
            put("to", to)
        },
    )
}

fun moveSubtaskCommand(childId: String, parentId: String): JsonObject = buildJsonObject {
    put(
        "move_tasks",
        buildJsonObject {
            put("ids", buildJsonArray { add(JsonPrimitive(childId)) })
            put(
                "placement",
                buildJsonObject {
                    put(
                        "parent",
                        buildJsonObject {
                            put("parent", parentId)
                            put("position", "end")
                        },
                    )
                },
            )
        },
    )
}
