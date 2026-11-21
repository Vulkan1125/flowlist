package dev.flowlist.android.data

import android.content.Context
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.put
import java.util.UUID

data class AppState(
    val paired: Boolean = false,
    val snapshot: Snapshot? = null,
    val pending: Set<String> = emptySet(),
    val pendingCount: Int = 0,
    val syncing: Boolean = false,
    val online: Boolean = true,
    val error: String? = null,
    val server: String = "",
    /// When the Mac last answered. Used for the one-minute offline notice.
    val lastSyncAt: Long? = null,
    /// When the connection was lost, or when the app opened unpaired.
    val offlineSince: Long? = null,
)

class Repository(context: Context) {
    private val api = Api()
    private val session = SessionStore(context)
    private val files = FileStore(context)
    private val mutex = Mutex()

    private var outbox: MutableList<PendingOp> = files.loadOutbox().toMutableList()
    private val _state = MutableStateFlow(
        AppState(
            paired = session.paired,
            snapshot = files.loadSnapshot(),
            pending = outbox.mapNotNull { it.taskId }.toSet(),
            pendingCount = outbox.size,
            server = session.baseUrl.orEmpty(),
            offlineSince = if (session.paired) System.currentTimeMillis() else null,
        ),
    )
    val state: StateFlow<AppState> = _state.asStateFlow()

    init {
        if (!session.paired) {
            // A token without an address (or the reverse) is unusable; start
            // pairing again rather than showing a half-configured app.
            session.clear()
        }
    }

    suspend fun pair(input: String, code: String): Boolean {
        val base = normalizeBase(input) ?: run {
            _state.update { it.copy(error = "Enter the address shown on your Mac, including the port.") }
            return false
        }
        val name = android.os.Build.MANUFACTURER
            .replaceFirstChar { it.uppercase() }
            .plus(" ")
            .plus(android.os.Build.MODEL)
            .trim()
        _state.update { it.copy(syncing = true, error = null) }
        return try {
            val result = api.pair(base, code, name)
            session.baseUrl = base
            session.token = result.token
            session.deviceName = result.deviceName.ifEmpty { name }
            _state.update {
                it.copy(paired = true, server = base, syncing = false)
            }
            sync()
            true
        } catch (error: Exception) {
            _state.update {
                it.copy(syncing = false, error = friendly(error))
            }
            false
        }
    }

    suspend fun sync() {
        if (!session.paired) return
        mutex.withLock {
            _state.update { it.copy(syncing = true) }
            try {
                flushLocked()
                val snapshot = api.snapshot(session.baseUrl!!, session.token!!)
                files.saveSnapshot(snapshot)
                _state.update {
                    it.copy(
                        snapshot = snapshot,
                        online = true,
                        syncing = false,
                        error = null,
                        lastSyncAt = System.currentTimeMillis(),
                        offlineSince = null,
                    )
                }
            } catch (error: Exception) {
                _state.update {
                    it.copy(
                        syncing = false,
                        online = false,
                        error = friendly(error),
                        offlineSince = it.offlineSince ?: System.currentTimeMillis(),
                    )
                }
            }
        }
    }

    suspend fun capture(text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        val id = Ulid.next()
        val title = displayTitle(trimmed)
        val now = java.time.Instant.now().toString()
        val task = TaskDto(
            id = id,
            title = title,
            createdAt = now,
            modifiedAt = now,
        )
        val snapshot = _state.value.snapshot ?: Snapshot()
        val db = snapshot.db
        val updated = snapshot.copy(
            db = db.copy(
                tasks = db.tasks + (id to task),
                inboxOrder = db.inboxOrder + id,
            ),
        )
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(PendingOp(id = "op_${UUID.randomUUID()}", text = trimmed, taskId = id, label = title), id)
        sync()
    }

    suspend fun complete(taskId: String, completed: Boolean) {
        val snapshot = _state.value.snapshot ?: return
        val task = snapshot.db.tasks[taskId] ?: return
        val db = snapshot.db
        val updatedTask = task.copy(
            completed = completed,
            completedAt = if (completed) java.time.Instant.now().toString() else null,
        )
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks + (taskId to updatedTask)))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = completeCommand(listOf(taskId), completed),
                label = task.title,
            ),
        )
        sync()
    }

    suspend fun update(task: TaskDto, edit: TaskEdit) {
        val snapshot = _state.value.snapshot ?: return
        if (edit.title.isBlank()) return
        val db = snapshot.db
        val updatedTask = task.copy(
            title = edit.title,
            notes = edit.notes,
            project = edit.project,
            due = edit.due,
            defer = edit.defer,
            priority = edit.priority,
            flagged = edit.flagged,
            estimateMin = edit.estimateMin,
            tags = edit.tags.toList(),
            repeat = edit.repeat,
            modifiedAt = java.time.Instant.now().toString(),
        )
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks + (task.id to updatedTask)))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }

        val projectJson: JsonElement = edit.project?.let { JsonPrimitive(it) } ?: JsonNull
        val priorityJson: JsonElement = edit.priority?.let { JsonPrimitive(it) } ?: JsonNull
        val estimateJson: JsonElement = edit.estimateMin?.let { JsonPrimitive(it) } ?: JsonNull
        val dueJson: JsonElement = if (edit.due != null) stampJson(edit.due) else JsonNull
        val deferJson: JsonElement = if (edit.defer != null) stampJson(edit.defer) else JsonNull
        val patch = kotlinx.serialization.json.buildJsonObject {
            put("title", edit.title)
            put("notes", edit.notes)
            put("project", projectJson)
            put("due", dueJson)
            put("defer", deferJson)
            put("priority", priorityJson)
            put("flagged", edit.flagged)
            put("estimate_min", estimateJson)
            put("repeat", edit.repeat ?: JsonNull)
        }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = updateCommand(task.id, patch),
                label = edit.title,
            ),
        )
        val before = task.tags.toSet()
        val add = (edit.tags - before).toList()
        val remove = (before - edit.tags).toList()
        if (add.isNotEmpty() || remove.isNotEmpty()) {
            enqueue(
                PendingOp(
                    id = "op_${UUID.randomUUID()}",
                    command = setTagsCommand(task.id, add, remove),
                    label = edit.title,
                ),
            )
        }
        sync()
    }

    suspend fun addSubtask(parent: TaskDto, title: String) {
        val trimmed = title.trim()
        if (trimmed.isEmpty()) return
        val snapshot = _state.value.snapshot ?: return
        val id = Ulid.next()
        val now = java.time.Instant.now().toString()
        val child = TaskDto(
            id = id,
            title = trimmed,
            parent = parent.id,
            project = parent.project,
            createdAt = now,
            modifiedAt = now,
        )
        val updated = snapshot.copy(db = snapshot.db.copy(tasks = snapshot.db.tasks + (id to child)))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = createSubtaskCommand(id, trimmed, parent.id, parent.project),
                label = trimmed,
            ),
        )
        sync()
    }

    suspend fun toggleChecklist(task: TaskDto, itemId: String, completed: Boolean) {
        val snapshot = _state.value.snapshot ?: return
        val db = snapshot.db
        val current = db.tasks[task.id] ?: return
        val item = current.checklist.firstOrNull { it.id == itemId } ?: return
        val updatedTask = current.copy(
            checklist = current.checklist.map { if (it.id == itemId) it.copy(completed = completed) else it },
            modifiedAt = java.time.Instant.now().toString(),
        )
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks + (task.id to updatedTask)))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = checklistCommand(task.id, itemId, item.title, completed),
                label = item.title,
            ),
        )
        sync()
    }

    suspend fun updateLibrarySettings(updated: SettingsDto) {
        val current = _state.value.snapshot?.db?.settings ?: SettingsDto()
        val patch = buildJsonObject {
            if (updated.defaultReviewDays != current.defaultReviewDays) {
                put("default_review_days", updated.defaultReviewDays)
            }
            if (updated.showCompletedDays != current.showCompletedDays) {
                put("show_completed_days", updated.showCompletedDays)
            }
            if (updated.changedDays != current.changedDays) {
                put("changed_days", updated.changedDays)
            }
            if (updated.trashRetentionDays != current.trashRetentionDays) {
                put("trash_retention_days", updated.trashRetentionDays)
            }
        }
        if (patch.isEmpty()) return
        updateSettings(updated, patch)
    }

    suspend fun updateSettings(updated: dev.flowlist.android.data.SettingsDto, patch: kotlinx.serialization.json.JsonObject) {
        val snapshot = _state.value.snapshot ?: return
        val db = snapshot.db
        val next = snapshot.copy(db = db.copy(settings = updated))
        files.saveSnapshot(next)
        _state.update { it.copy(snapshot = next) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = settingsCommand(patch),
                label = "Settings",
            ),
        )
        sync()
    }

    suspend fun deleteSeries(task: TaskDto) {
        val snapshot = _state.value.snapshot ?: return
        val series = task.repeatSeries ?: task.id
        val db = snapshot.db
        val removed = db.tasks.filterValues { it.repeatSeries == series || it.id == task.id }.keys
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks - removed))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = deleteSeriesCommand(removed.toList()),
                label = task.title,
            ),
        )
        sync()
    }

    suspend fun reviewProject(project: ProjectDto) {
        val snapshot = _state.value.snapshot ?: return
        val today = java.time.LocalDate.now()
        val next = today.plusDays(project.reviewIntervalDays.toLong())
        val db = snapshot.db
        val updated = snapshot.copy(
            db = db.copy(
                projects = db.projects + (project.id to project.copy(nextReview = next.toString())),
            ),
        )
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = reviewCommand(project.id, today.toString()),
                label = project.name,
            ),
        )
        sync()
    }

    suspend fun purge(taskId: String) {
        val snapshot = _state.value.snapshot ?: return
        val db = snapshot.db
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks - taskId))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = purgeCommand(listOf(taskId)),
                label = "Purge task",
            ),
        )
        sync()
    }

    suspend fun delete(taskId: String): TaskDto? {
        val snapshot = _state.value.snapshot ?: return null
        val task = snapshot.db.tasks[taskId] ?: return null
        val db = snapshot.db
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks - taskId))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = deleteCommand(listOf(taskId)),
                label = task.title,
            ),
        )
        sync()
        return task
    }

    suspend fun restore(task: TaskDto) {
        val snapshot = _state.value.snapshot ?: return
        val db = snapshot.db
        val updated = snapshot.copy(
            db = db.copy(
                tasks = db.tasks + (task.id to task),
                inboxOrder = if (task.project == null && task.parent == null) {
                    (db.inboxOrder + task.id).distinct()
                } else {
                    db.inboxOrder
                },
            ),
        )
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = restoreCommand(listOf(task.id)),
                label = task.title,
            ),
        )
        sync()
    }

    suspend fun createProject(name: String, folder: String?) {
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        send(createProjectCommand(trimmed, folder), "New project “$trimmed”")
    }

    suspend fun updateProject(project: ProjectDto, name: String, folder: String?, status: String) {
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        var queued = false
        val patch = buildJsonObject {
            if (trimmed != project.name) put("name", trimmed)
            if (folder != project.folder) put("folder", folder?.let { JsonPrimitive(it) } ?: JsonNull)
        }
        if (patch.isNotEmpty()) {
            enqueue(
                PendingOp(
                    id = "op_${UUID.randomUUID()}",
                    command = updateProjectCommand(project.id, patch),
                    label = "Edit project “${project.name}”",
                ),
            )
            queued = true
        }
        if (status != project.status) {
            enqueue(
                PendingOp(
                    id = "op_${UUID.randomUUID()}",
                    command = setProjectStatusCommand(project.id, status),
                    label = "Set project “${project.name}” to $status",
                ),
            )
            queued = true
        }
        if (queued) sync()
    }

    suspend fun setProjectStatus(project: ProjectDto, status: String) {
        send(setProjectStatusCommand(project.id, status), "Set project “${project.name}” to $status")
    }

    suspend fun deleteProject(project: ProjectDto) {
        send(deleteProjectCommand(project.id), "Delete project “${project.name}”")
    }

    suspend fun createFolder(name: String, parent: String?) {
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        send(createFolderCommand(trimmed, parent), "New folder “$trimmed”")
    }

    suspend fun updateFolder(folder: FolderDto, name: String, parent: String?) {
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        var queued = false
        if (trimmed != folder.name) {
            enqueue(
                PendingOp(
                    id = "op_${UUID.randomUUID()}",
                    command = renameFolderCommand(folder.id, trimmed),
                    label = "Rename folder “${folder.name}”",
                ),
            )
            queued = true
        }
        if (parent != folder.parent) {
            enqueue(
                PendingOp(
                    id = "op_${UUID.randomUUID()}",
                    command = moveFolderCommand(folder.id, parent),
                    label = "Move folder “${folder.name}”",
                ),
            )
            queued = true
        }
        if (queued) sync()
    }

    suspend fun deleteFolder(folder: FolderDto) {
        send(deleteFolderCommand(folder.id), "Delete folder “${folder.name}”")
    }

    suspend fun createTag(name: String, parent: String?) {
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        send(createTagCommand(trimmed, parent), "New tag “$trimmed”")
    }

    suspend fun updateTag(tag: TagDto, name: String, parent: String?) {
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        if (trimmed == tag.name && parent == tag.parent) return
        send(updateTagCommand(tag.id, trimmed, parent), "Edit tag “${tag.name}”")
    }

    suspend fun deleteTag(tag: TagDto) {
        send(deleteTagCommand(tag.id), "Delete tag “${tag.name}”")
    }

    private suspend fun send(command: kotlinx.serialization.json.JsonObject, label: String) {
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = command,
                label = label,
            ),
        )
        sync()
    }

    suspend fun signOut() {
        val base = session.baseUrl
        val token = session.token
        session.clear()
        files.clear()
        outbox.clear()
        _state.value = AppState(paired = false)
        // Revoke on the Mac after the app has already returned to pairing, so
        // an unreachable server never leaves the screen waiting.
        if (base != null && token != null) {
            runCatching { api.unpair(base, token) }
        }
    }

    fun clearError() {
        _state.update { it.copy(error = null) }
    }

    private fun enqueue(op: PendingOp, optimisticId: String? = null) {
        outbox.add(op)
        files.saveOutbox(outbox)
        _state.update {
            it.copy(
                pending = it.pending + listOfNotNull(optimisticId),
                pendingCount = outbox.size,
            )
        }
    }

    private suspend fun flushLocked() {
        if (outbox.isEmpty()) return
        val current = _state.value.snapshot ?: Snapshot()
        val batch = outbox.toList()
        val response = api.sendOps(
            session.baseUrl!!,
            session.token!!,
            batch,
            current.revision,
        )
        // The server applies a batch in order and answers every operation; a
        // failed one is not retried forever. Network errors keep the whole
        // batch above.
        outbox.removeAll(batch.toSet())
        files.saveOutbox(outbox)
        if (response.db != null) {
            val snapshot = Snapshot(response.revision, db = response.db)
            files.saveSnapshot(snapshot)
            _state.update { it.copy(snapshot = snapshot) }
        }
        val failure = response.results.firstOrNull { !it.ok && it.error != null }
        _state.update {
            it.copy(
                pending = outbox.mapNotNull { op -> op.taskId }.toSet(),
                pendingCount = outbox.size,
                error = failure?.error ?: it.error,
            )
        }
    }

    private fun friendly(error: Exception): String = when (error) {
        is ApiException -> error.message ?: "The Mac refused the request."
        is java.net.SocketTimeoutException -> "The Mac did not answer. Is Flowlist open and the server running?"
        is java.net.ConnectException -> "Cannot reach the Mac. Check Wi-Fi or Tailscale and the address."
        is java.net.UnknownHostException -> "That address could not be found."
        else -> error.message ?: "Sync failed."
    }
}

object Ulid {
    private const val ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    private val random = java.security.SecureRandom()

    fun next(): String {
        val chars = CharArray(26)
        var time = System.currentTimeMillis()
        for (index in 9 downTo 0) {
            chars[index] = ALPHABET[(time and 31L).toInt()]
            time = time shr 5
        }
        for (index in 10 until 26) {
            chars[index] = ALPHABET[random.nextInt(ALPHABET.length)]
        }
        return String(chars)
    }
}

/** The title a person sees before the Mac parses the chips. */
fun displayTitle(text: String): String {
    val stripped = text
        .replace(Regex("""(^|\s)(#[^\s]+|::[^\s]+|due:\S+|defer:\S+|est:\S+|repeat:\S+)"""), " ")
        .replace(Regex("""\s+"""), " ")
        .trim()
        .removeSuffix("!")
        .trim()
    return stripped.ifEmpty { text.trim() }
}
