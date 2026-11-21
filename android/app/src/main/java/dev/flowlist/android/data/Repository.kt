package dev.flowlist.android.data

import android.content.Context
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
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
                    )
                }
            } catch (error: Exception) {
                _state.update {
                    it.copy(
                        syncing = false,
                        online = false,
                        error = friendly(error),
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

    suspend fun signOut() {
        val base = session.baseUrl
        val token = session.token
        if (base != null && token != null) {
            runCatching { api.unpair(base, token) }
        }
        session.clear()
        files.clear()
        outbox.clear()
        _state.value = AppState(paired = false)
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
