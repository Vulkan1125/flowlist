package dev.flowlist.android.data

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.core.content.FileProvider
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
import java.io.File
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
    /// Attachment paths with a durable copy on this phone.
    val savedAttachments: Set<String> = emptySet(),
    val savingAttachments: Boolean = false,
)

class Repository(context: Context) {
    private val appContext = context.applicationContext
    private val api = Api()
    private val session = SessionStore(context)
    private val files = FileStore(context)
    private val mutex = Mutex()

    private var outbox: MutableList<PendingOp> = files.loadOutbox().toMutableList()
    private val savedDir = File(appContext.filesDir, "attachments").apply { mkdirs() }
    private val savedIndex = File(savedDir, "index.txt")
    private val savedPaths: MutableSet<String> = savedIndex
        .takeIf { it.exists() }
        ?.readLines()
        ?.filter { it.isNotBlank() && savedFile(it).exists() }
        ?.toMutableSet()
        ?: mutableSetOf()
    private val _state = MutableStateFlow(
        AppState(
            paired = session.paired,
            snapshot = files.loadSnapshot(),
            pending = outbox.mapNotNull { it.taskId }.toSet(),
            pendingCount = outbox.size,
            server = session.baseUrl.orEmpty(),
            offlineSince = if (session.paired) System.currentTimeMillis() else null,
            savedAttachments = savedPaths.toSet(),
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
        if (edit.priorityAll && task.repeat != null) {
            enqueue(
                PendingOp(
                    id = "op_${UUID.randomUUID()}",
                    command = priorityCommand(listOf(task.id), edit.priority, series = true),
                    label = edit.title,
                ),
            )
        }
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

    suspend fun addChecklist(task: TaskDto, title: String) {
        val trimmed = title.trim()
        if (trimmed.isEmpty()) return
        val snapshot = _state.value.snapshot ?: return
        val db = snapshot.db
        val current = db.tasks[task.id] ?: return
        val item = ChecklistDto(id = Ulid.next(), title = trimmed, completed = false)
        val updatedTask = current.copy(
            checklist = current.checklist + item,
            modifiedAt = java.time.Instant.now().toString(),
        )
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks + (task.id to updatedTask)))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = addChecklistCommand(task.id, trimmed),
                label = trimmed,
            ),
        )
        sync()
    }

    suspend fun deleteChecklist(task: TaskDto, item: ChecklistDto) {
        val snapshot = _state.value.snapshot ?: return
        val db = snapshot.db
        val current = db.tasks[task.id] ?: return
        val updatedTask = current.copy(
            checklist = current.checklist.filterNot { it.id == item.id },
            modifiedAt = java.time.Instant.now().toString(),
        )
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks + (task.id to updatedTask)))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = deleteChecklistCommand(task.id, item.id),
                label = item.title,
            ),
        )
        sync()
    }

    suspend fun moveChecklist(task: TaskDto, item: ChecklistDto, to: Int) {
        val snapshot = _state.value.snapshot ?: return
        val db = snapshot.db
        val current = db.tasks[task.id] ?: return
        val from = current.checklist.indexOfFirst { it.id == item.id }
        if (from < 0) return
        val target = to.coerceIn(0, current.checklist.lastIndex)
        if (target == from) return
        val items = current.checklist.toMutableList()
        items.removeAt(from)
        items.add(target, item)
        val updatedTask = current.copy(
            checklist = items,
            modifiedAt = java.time.Instant.now().toString(),
        )
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks + (task.id to updatedTask)))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = moveChecklistCommand(task.id, item.id, target),
                label = "Reorder ${item.title}",
            ),
        )
        sync()
    }

    suspend fun moveSubtask(child: TaskDto, parent: TaskDto) {
        if (child.id == parent.id) return
        send(moveSubtaskCommand(child.id, parent.id), "Move “${child.title}” under “${parent.title}”")
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

    suspend fun uploadAttachment(taskId: String, uri: Uri): Boolean {
        val base = session.baseUrl ?: return false
        val token = session.token ?: return false
        return try {
            val resolver = appContext.contentResolver
            val name = resolver.query(
                uri,
                arrayOf(OpenableColumns.DISPLAY_NAME),
                null,
                null,
                null,
            )?.use { cursor ->
                if (cursor.moveToFirst()) cursor.getString(0) else null
            } ?: uri.lastPathSegment ?: "attachment"
            val bytes = resolver.openInputStream(uri)?.use { it.readBytes() }
                ?: throw IllegalStateException("Could not read the file.")
            if (bytes.size > 40 * 1024 * 1024) {
                throw IllegalStateException("Files are limited to 40 MB.")
            }
            val path = api.uploadFile(base, token, name, bytes)
            updateAttachments(taskId, (currentAttachments(taskId) + path).distinct())
            true
        } catch (error: Exception) {
            _state.update { it.copy(error = friendly(error)) }
            false
        }
    }

    suspend fun saveAllAttachments(): Pair<Int, Int> {
        val snapshot = _state.value.snapshot ?: return 0 to 0
        val paths = (
            snapshot.db.tasks.values.flatMap { it.attachments } +
                snapshot.db.projects.values.flatMap { it.attachments }
            ).filter { it.startsWith("/") }.distinct()
        _state.update { it.copy(savingAttachments = true) }
        var saved = 0
        var failed = 0
        for (path in paths) {
            if (path in savedPaths) {
                saved++
            } else if (saveAttachmentForOffline(path)) {
                saved++
            } else {
                failed++
            }
        }
        _state.update { it.copy(savingAttachments = false) }
        return saved to failed
    }

    suspend fun removeAllSavedAttachments() {
        savedDir.listFiles()
            ?.filter { it.isFile && it.name != "index.txt" }
            ?.forEach { it.delete() }
        savedPaths.clear()
        persistSaved()
    }

    suspend fun removeAttachment(taskId: String, path: String) {
        val next = currentAttachments(taskId).filterNot { it == path }
        updateAttachments(taskId, next)
        if (path !in next) removeSavedAttachment(path)
    }

    suspend fun saveAttachmentForOffline(path: String): Boolean {
        val base = session.baseUrl ?: return false
        val token = session.token ?: return false
        return try {
            val file = savedFile(path)
            if (!file.exists()) {
                val bytes = api.downloadFile(base, token, path)
                val temp = File(file.parentFile, "${file.name}.part")
                temp.writeBytes(bytes)
                if (!temp.renameTo(file)) {
                    file.writeBytes(bytes)
                    temp.delete()
                }
            }
            savedPaths.add(path)
            persistSaved()
            true
        } catch (error: Exception) {
            android.util.Log.w("FlowlistAttachments", "Could not save $path", error)
            _state.update { it.copy(error = friendly(error)) }
            false
        }
    }

    suspend fun removeSavedAttachment(path: String) {
        savedFile(path).delete()
        savedPaths.remove(path)
        persistSaved()
    }

    private fun savedFile(path: String): File {
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest(path.toByteArray())
            .joinToString("") { "%02x".format(it) }
        val extension = path.substringAfterLast('/', "")
            .substringAfterLast('.', "")
            .take(10)
            .lowercase()
        val name = if (extension.isBlank()) digest else "$digest.$extension"
        return File(savedDir, name)
    }

    private fun persistSaved() {
        runCatching {
            savedIndex.writeText(savedPaths.sorted().joinToString("\n"))
        }
        _state.update { it.copy(savedAttachments = savedPaths.toSet()) }
    }

    suspend fun openAttachment(path: String): Boolean {
        val base = session.baseUrl ?: return false
        val token = session.token ?: return false
        return try {
            val saved = savedFile(path).takeIf { path in savedPaths && it.exists() }
            val file = saved ?: run {
                val bytes = api.downloadFile(base, token, path)
                val dir = File(appContext.cacheDir, "attachments").apply { mkdirs() }
                val name = path.substringAfterLast('/').ifBlank { "attachment" }
                File(dir, name).apply { writeBytes(bytes) }
            }
            val uri = FileProvider.getUriForFile(
                appContext,
                "${appContext.packageName}.files",
                file,
            )
            val intent = Intent(Intent.ACTION_VIEW)
                .setDataAndType(uri, mimeFor(path.substringAfterLast('/')))
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            appContext.startActivity(
                Intent.createChooser(intent, "Open attachment")
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
            true
        } catch (error: Exception) {
            _state.update { it.copy(error = friendly(error)) }
            false
        }
    }

    private fun currentAttachments(taskId: String): List<String> =
        _state.value.snapshot?.db?.tasks?.get(taskId)?.attachments.orEmpty()

    private suspend fun updateAttachments(taskId: String, attachments: List<String>) {
        val snapshot = _state.value.snapshot ?: return
        val task = snapshot.db.tasks[taskId] ?: return
        val db = snapshot.db
        val updatedTask = task.copy(
            attachments = attachments,
            modifiedAt = java.time.Instant.now().toString(),
        )
        val updated = snapshot.copy(db = db.copy(tasks = db.tasks + (taskId to updatedTask)))
        files.saveSnapshot(updated)
        _state.update { it.copy(snapshot = updated) }
        val patch = buildJsonObject {
            put(
                "attachments",
                kotlinx.serialization.json.buildJsonArray {
                    attachments.forEach { add(JsonPrimitive(it)) }
                },
            )
        }
        enqueue(
            PendingOp(
                id = "op_${UUID.randomUUID()}",
                command = updateCommand(taskId, patch),
                label = "Attachment",
            ),
        )
        sync()
    }

    private fun mimeFor(name: String): String = when (name.substringAfterLast('.', "").lowercase()) {
        "png" -> "image/png"
        "jpg", "jpeg" -> "image/jpeg"
        "gif" -> "image/gif"
        "webp" -> "image/webp"
        "pdf" -> "application/pdf"
        "txt", "md" -> "text/plain"
        "json" -> "application/json"
        "mp4" -> "video/mp4"
        "mp3" -> "audio/mpeg"
        else -> "*/*"
    }

    suspend fun savePerspective(
        id: String,
        name: String,
        filter: kotlinx.serialization.json.JsonElement,
        groupBy: String,
        sort: String,
        showCompleted: Boolean,
    ) {
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        send(
            savePerspectiveCommand(id, trimmed, filter, groupBy, sort, showCompleted),
            "Save perspective “$trimmed”",
        )
    }

    suspend fun deletePerspective(perspective: PerspectiveDto) {
        send(deletePerspectiveCommand(perspective.id), "Delete perspective “${perspective.name}”")
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
