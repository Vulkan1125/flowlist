package dev.flowlist.android.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
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
        .readTimeout(180, TimeUnit.SECONDS)
        .writeTimeout(180, TimeUnit.SECONDS)
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

    suspend fun history(base: String, token: String, undo: Boolean): HistoryResponse =
        withContext(Dispatchers.IO) {
            val path = if (undo) "undo" else "redo"
            val text = execute(
                Request.Builder()
                    .url("${base.trimEnd('/')}/api/v1/$path")
                    .header("Authorization", "Bearer $token")
                    .post("".toRequestBody(mediaType))
                    .build(),
            )
            json.decodeFromString<HistoryResponse>(text)
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

    suspend fun dismissReminder(base: String, token: String, taskId: String) =
        withContext(Dispatchers.IO) {
            val body = buildJsonObject { put("task_id", taskId) }.toString()
            execute(
                Request.Builder()
                    .url("${base.trimEnd('/')}/api/v1/reminders/dismiss")
                    .header("Authorization", "Bearer $token")
                    .post(body.toRequestBody(mediaType))
                    .build(),
            )
        }

    suspend fun reminderDismissals(
        base: String,
        token: String,
    ): ReminderDismissalsResponse = withContext(Dispatchers.IO) {
        val text = execute(
            Request.Builder()
                .url("${base.trimEnd('/')}/api/v1/reminders/dismissed?after=0")
                .header("Authorization", "Bearer $token")
                .get()
                .build(),
        )
        json.decodeFromString<ReminderDismissalsResponse>(text)
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

    suspend fun uploadFile(
        base: String,
        token: String,
        name: String,
        bytes: ByteArray,
    ): String = withContext(Dispatchers.IO) {
        val encoded = java.net.URLEncoder.encode(name, "UTF-8")
        val request = Request.Builder()
            .url("${base.trimEnd('/')}/api/v1/files?name=$encoded")
            .header("Authorization", "Bearer $token")
            .post(bytes.toRequestBody("application/octet-stream".toMediaType()))
            .build()
        val text = execute(request)
        val value = json.parseToJsonElement(text) as JsonObject
        value["path"]?.toString()?.trim('"') ?: throw ApiException("The Mac did not return a file path.")
    }

    suspend fun downloadFile(base: String, token: String, path: String): ByteArray =
        withContext(Dispatchers.IO) {
            val encoded = java.net.URLEncoder.encode(path, "UTF-8")
            val request = Request.Builder()
                .url("${base.trimEnd('/')}/api/v1/files?path=$encoded")
                .header("Authorization", "Bearer $token")
                .get()
                .build()
            client.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    throw ApiException("The Mac could not send that file (${response.code}).")
                }
                response.body?.bytes() ?: ByteArray(0)
            }
        }
}

@Serializable
data class ReminderDismissalDto(
    @SerialName("task_id") val taskId: String = "",
    val seq: Long = 0,
    val at: Long = 0,
)

@Serializable
data class ReminderDismissalsResponse(
    val seq: Long = 0,
    val dismissals: List<ReminderDismissalDto> = emptyList(),
)

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

fun addTaskCommand(title: String, attachments: List<String> = emptyList()): JsonObject =
    buildJsonObject {
        put(
            "create_task",
            buildJsonObject {
                put("title", title)
                put("notes", "")
                put("project", JsonNull)
                put("project_path", JsonNull)
                put("parent", JsonNull)
                put("tags", buildJsonArray {})
                put("tag_paths", buildJsonArray {})
                put("due", JsonNull)
                put("defer", JsonNull)
                put("estimate_min", JsonNull)
                put("flagged", false)
                put("priority", JsonNull)
                put(
                    "attachments",
                    buildJsonArray { attachments.forEach { add(JsonPrimitive(it)) } },
                )
                put("repeat", JsonNull)
                put("sequential", false)
                put("position", JsonPrimitive("end"))
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


fun repeatRuleJson(
    mode: String,
    schedule: String,
    every: Int,
    unit: String,
    endKind: String,
    endDate: String?,
    endCount: Int,
    completedCount: Int,
    weekdays: JsonElement?,
): JsonObject = buildJsonObject {
    put("mode", mode)
    put("schedule", schedule)
    put("every", every)
    put("unit", unit)
    put(
        "end",
        when (endKind) {
            "on_date" -> buildJsonObject {
                put("kind", "on_date")
                put("date", endDate.orEmpty())
            }
            "after_count" -> buildJsonObject {
                put("kind", "after_count")
                put("date", endCount)
            }
            else -> buildJsonObject { put("kind", "never") }
        },
    )
    put("completed_count", completedCount)
    put("weekdays", weekdays ?: JsonNull)
}

fun priorityCommand(ids: List<String>, priority: String?, series: Boolean): JsonObject =
    buildJsonObject {
        put(
            "set_task_priority",
            buildJsonObject {
                put("ids", buildJsonArray { ids.forEach { add(JsonPrimitive(it)) } })
                put("priority", priority?.let { JsonPrimitive(it) } ?: JsonNull)
                put("series", series)
            },
        )
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
            // A subtask inherits its parent's project. Sending both would make
            // the engine reject the command, which is why adding from the
            // phone used to fail silently.
            put("project", JsonNull)
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

fun createProjectCommand(
    name: String,
    folder: String?,
    kind: String = "parallel",
    notes: String = "",
    reviewDays: Int? = null,
    icon: String = "project",
    fullscreenReminder: Boolean = false,
    completeWithLastAction: Boolean = false,
    due: StampDto? = null,
    defer: StampDto? = null,
    planned: StampDto? = null,
    tags: Set<String> = emptySet(),
    attachments: List<String> = emptyList(),
): JsonObject = buildJsonObject {
    put(
        "create_project",
        buildJsonObject {
            put("name", name)
            put("notes", notes)
            put("folder", folder?.let { JsonPrimitive(it) } ?: JsonNull)
            put("tags", buildJsonArray { tags.forEach { add(JsonPrimitive(it)) } })
            put("due", due?.let { stampJson(it) } ?: JsonNull)
            put("defer", defer?.let { stampJson(it) } ?: JsonNull)
            put("planned", planned?.let { stampJson(it) } ?: JsonNull)
            put("review_interval_days", reviewDays?.let { JsonPrimitive(it) } ?: JsonNull)
            put("next_review", JsonNull)
            put("complete_with_last_action", completeWithLastAction)
            put("sequential", kind == "sequential")
            put("kind", kind)
            put("icon", icon)
            put("attachments", buildJsonArray { attachments.forEach { add(JsonPrimitive(it)) } })
            put("fullscreen_reminder", fullscreenReminder)
            put("position", "end")
        },
    )
}

fun archiveProjectsCommand(ids: List<String>, archived: Boolean): JsonObject = buildJsonObject {
    put(
        "archive_projects",
        buildJsonObject {
            put("ids", buildJsonArray { ids.forEach { add(JsonPrimitive(it)) } })
            put("archived", archived)
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

fun createFolderCommand(name: String, parent: String?, fullscreenReminder: Boolean = false): JsonObject = buildJsonObject {
    put(
        "create_folder",
        buildJsonObject {
            put("name", name)
            put("parent", parent?.let { JsonPrimitive(it) } ?: JsonNull)
            put("fullscreen_reminder", fullscreenReminder)
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

fun geoJson(place: dev.flowlist.android.data.GeoDto): JsonObject = buildJsonObject {
    put("lat", place.lat)
    put("lon", place.lon)
}

fun createTagCommand(
    name: String,
    parent: String?,
    location: dev.flowlist.android.data.GeoDto? = null,
    status: String = "available",
): JsonObject = buildJsonObject {
    put(
        "create_tag",
        buildJsonObject {
            put("name", name)
            put("parent", parent?.let { JsonPrimitive(it) } ?: JsonNull)
            put("status", status)
            put("location", location?.let(::geoJson) ?: JsonNull)
            put("position", "end")
        },
    )
}

fun updateTagCommand(
    id: String,
    name: String,
    parent: String?,
    location: dev.flowlist.android.data.GeoDto?,
    locationChanged: Boolean,
    status: String? = null,
): JsonObject = buildJsonObject {
    put(
        "update_tag",
        buildJsonObject {
            put("id", id)
            put(
                "patch",
                buildJsonObject {
                    put("name", name)
                    put("parent", parent?.let { JsonPrimitive(it) } ?: JsonNull)
                    status?.let { put("status", it) }
                    if (locationChanged) {
                        put("location", location?.let(::geoJson) ?: JsonNull)
                    }
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

fun moveTaskCommand(taskId: String, placement: JsonObject): JsonObject = buildJsonObject {
    put(
        "move_tasks",
        buildJsonObject {
            put("ids", buildJsonArray { add(JsonPrimitive(taskId)) })
            put("placement", placement)
        },
    )
}

fun parentPlacement(parentId: String, position: JsonElement): JsonObject = buildJsonObject {
    put(
        "parent",
        buildJsonObject {
            put("parent", parentId)
            put("position", position)
        },
    )
}

fun projectPlacement(projectId: String, position: JsonElement): JsonObject = buildJsonObject {
    put(
        "project",
        buildJsonObject {
            put("project", projectId)
            put("position", position)
        },
    )
}

fun inboxPlacement(position: JsonElement): JsonObject = buildJsonObject {
    put("inbox", position)
}

fun restoreOneOffsCommand(taskId: String): JsonObject = buildJsonObject {
    put(
        "convert_one_off_to_subtasks",
        buildJsonObject { put("id", taskId) },
    )
}

fun convertSubtasksCommand(taskId: String): JsonObject = buildJsonObject {
    put(
        "convert_subtasks_to_one_off",
        buildJsonObject { put("id", taskId) },
    )
}

fun setTaskStepModeCommand(
    taskId: String,
    sequential: Boolean,
    stepsOneOff: Boolean,
): JsonObject = buildJsonObject {
    put(
        "set_task_step_mode",
        buildJsonObject {
            put("id", taskId)
            put("sequential", sequential)
            put("steps_one_off", stepsOneOff)
        },
    )
}

fun savePerspectiveCommand(
    id: String,
    name: String,
    filter: kotlinx.serialization.json.JsonElement,
    groupBy: String,
    sort: String,
    projectGroupBy: String,
    projectSort: String,
    showCompleted: Boolean,
    pinned: Boolean,
    position: Int,
    icon: String?,
    description: String,
): JsonObject = buildJsonObject {
    put(
        "save_perspective",
        buildJsonObject {
            put(
                "perspective",
                buildJsonObject {
                    put("id", id)
                    put("name", name)
                    put("filter", filter)
                    put("group_by", groupBy)
                    put("sort", sort)
                    put("project_group_by", projectGroupBy)
                    put("project_sort", projectSort)
                    put("show_completed", showCompleted)
                    put("pinned", pinned)
                    put("position", position)
                    put("description", description)
                    icon?.let { put("icon", it) }
                },
            )
        },
    )
}

fun deletePerspectiveCommand(id: String): JsonObject = buildJsonObject {
    put("delete_perspective", buildJsonObject { put("id", id) })
}

@Serializable
data class HistoryResponse(
    val ok: Boolean = false,
    val label: String? = null,
    val error: String? = null,
    val revision: Long = 0,
    val history: HistoryInfo? = null,
    val db: Db? = null,
)
