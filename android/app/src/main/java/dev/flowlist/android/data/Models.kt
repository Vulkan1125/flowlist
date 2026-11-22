package dev.flowlist.android.data

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.util.Locale

@Serializable
data class Snapshot(
    val revision: Long = 0,
    @SerialName("schema_version") val schemaVersion: Int = 1,
    val db: Db = Db(),
)

@Serializable
data class Db(
    val tasks: Map<String, TaskDto> = emptyMap(),
    val projects: Map<String, ProjectDto> = emptyMap(),
    val folders: Map<String, FolderDto> = emptyMap(),
    val tags: Map<String, TagDto> = emptyMap(),
    val perspectives: Map<String, PerspectiveDto> = emptyMap(),
    @SerialName("inbox_order") val inboxOrder: List<String> = emptyList(),
    val root: List<JsonElement> = emptyList(),
    val settings: SettingsDto = SettingsDto(),
)

@Serializable
data class SettingsDto(
    @SerialName("default_review_days") val defaultReviewDays: Int = 7,
    @SerialName("show_completed_days") val showCompletedDays: Int = 30,
    @SerialName("changed_days") val changedDays: Int = 7,
    @SerialName("trash_retention_days") val trashRetentionDays: Int = 30,
    @SerialName("default_due_time") val defaultDueTime: String? = "17:00:00",
    @SerialName("default_defer_time") val defaultDeferTime: String? = null,
)

@Serializable
data class TaskDto(
    val id: String,
    val title: String = "",
    val notes: String = "",
    val project: String? = null,
    val parent: String? = null,
    val tags: List<String> = emptyList(),
    val checklist: List<ChecklistDto> = emptyList(),
    val attachments: List<String> = emptyList(),
    val due: StampDto? = null,
    val defer: StampDto? = null,
    @SerialName("estimate_min") val estimateMin: Int? = null,
    val flagged: Boolean = false,
    val priority: String? = null,
    val completed: Boolean = false,
    @SerialName("completed_at") val completedAt: String? = null,
    @SerialName("trashed_at") val trashedAt: String? = null,
    @SerialName("child_order") val childOrder: List<String> = emptyList(),
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("modified_at") val modifiedAt: String = "",
    val repeat: JsonElement? = null,
    @SerialName("repeat_series") val repeatSeries: String? = null,
)

@Serializable
data class ChecklistDto(
    val id: String = "",
    val title: String = "",
    val completed: Boolean = false,
)

@Serializable
data class StampDto(
    val at: String,
    @SerialName("has_time") val hasTime: Boolean = false,
) {
    fun local(): ZonedDateTime? = runCatching {
        Instant.parse(at).atZone(ZoneId.systemDefault())
    }.getOrNull()
}

@Serializable
data class ProjectDto(
    val id: String,
    val name: String = "",
    val status: String = "active",
    val folder: String? = null,
    @SerialName("next_review") val nextReview: String? = null,
    @SerialName("review_interval_days") val reviewIntervalDays: Int = 7,
    @SerialName("task_order") val taskOrder: List<String> = emptyList(),
    val attachments: List<String> = emptyList(),
    @SerialName("modified_at") val modifiedAt: String = "",
)

@Serializable
data class FolderDto(
    val id: String,
    val name: String = "",
    val parent: String? = null,
    @SerialName("child_order") val childOrder: List<JsonElement> = emptyList(),
)

@Serializable
data class TagDto(
    val id: String,
    val name: String = "",
    val parent: String? = null,
)

@Serializable
data class PerspectiveDto(
    val id: String = "",
    val name: String = "",
    val filter: JsonElement? = null,
    @SerialName("group_by") val groupBy: String = "none",
    val sort: String = "manual",
    @SerialName("show_completed") val showCompleted: Boolean = false,
    val position: Int = 0,
)

@Serializable
data class PairResponse(
    val token: String,
    @SerialName("device_id") val deviceId: String = "",
    @SerialName("device_name") val deviceName: String = "",
)

@Serializable
data class OpsResponse(
    val revision: Long = 0,
    val stale: Boolean = false,
    val results: List<OpResult> = emptyList(),
    val db: Db? = null,
)

@Serializable
data class OpResult(
    val id: String = "",
    val ok: Boolean = false,
    val error: String? = null,
    val duplicate: Boolean = false,
)

@Serializable
data class ApiErrorResponse(val error: String = "Something went wrong")

data class TaskEdit(
    val title: String,
    val notes: String,
    val project: String?,
    val due: StampDto?,
    val defer: StampDto?,
    val priority: String?,
    val flagged: Boolean,
    val estimateMin: Int?,
    val tags: Set<String>,
    val repeat: JsonElement? = null,
    /// Apply the priority to the whole repeating series, not just this task.
    val priorityAll: Boolean = false,
)

data class TaskView(
    val id: String,
    val title: String,
    val notes: String,
    val projectName: String?,
    val projectId: String?,
    val parent: String?,
    val tags: List<String>,
    val due: StampDto?,
    val defer: StampDto?,
    val flagged: Boolean,
    val priority: String?,
    val completed: Boolean,
    val pending: Boolean,
    val checklistDone: Int,
    val checklistTotal: Int,
    val estimateMin: Int?,
    val attachments: List<String> = emptyList(),
    val completedAt: String? = null,
    val modifiedAt: String = "",
    val trashed: Boolean = false,
)

val TaskDto.isActive: Boolean
    get() = !completed && trashedAt == null

fun Db.taskViews(
    pending: Set<String> = emptySet(),
    includeCompleted: Boolean = false,
    includeTrashed: Boolean = false,
): List<TaskView> =
    tasks.values
        .asSequence()
        .filter { task ->
            (includeCompleted || !task.completed) &&
                (includeTrashed || task.trashedAt == null)
        }
        .map { task ->
            TaskView(
                id = task.id,
                title = task.title,
                notes = task.notes,
                projectName = task.project?.let { projects[it]?.name },
                projectId = task.project,
                parent = task.parent,
                tags = task.tags.mapNotNull { tags[it]?.name },
                due = task.due,
                defer = task.defer,
                flagged = task.flagged,
                priority = task.priority,
                completed = task.completed,
                pending = task.id in pending,
                checklistDone = task.checklist.count { it.completed },
                checklistTotal = task.checklist.size,
                estimateMin = task.estimateMin,
                attachments = task.attachments,
                completedAt = task.completedAt,
                modifiedAt = task.modifiedAt,
                trashed = task.trashedAt != null,
            )
        }
        .toList()

fun Db.inbox(): List<TaskView> {
    val byId = tasks
    val order = inboxOrder.ifEmpty {
        byId.values.filter { it.project == null && it.parent == null }.map { it.id }
    }
    return order
        .mapNotNull { byId[it] }
        .filter {
            // Inbox means unscheduled, exactly as on the Mac: giving a task a
            // due or defer date moves it to Today or Forecast.
            it.isActive &&
                it.project == null &&
                it.parent == null &&
                it.due == null &&
                it.defer == null
        }
        .map { it.toView(this) }
}

fun Db.today(): List<TaskView> {
    val today = LocalDate.now()
    return tasks.values
        .asSequence()
        .filter { it.isActive && it.parent == null }
        .filter { task ->
            val due = task.due?.local()?.toLocalDate() ?: return@filter false
            !due.isAfter(today)
        }
        .sortedWith(compareBy({ it.due?.at ?: "" }, { it.title.lowercase() }))
        .map { it.toView(this) }
        .toList()
}

fun Db.forecast(): Map<LocalDate, List<TaskView>> {
    val today = LocalDate.now()
    val horizon = today.plusDays(14)
    return tasks.values
        .asSequence()
        .filter { it.isActive && it.parent == null }
        .filter { task ->
            val due = task.due?.local()?.toLocalDate() ?: return@filter false
            !due.isAfter(horizon)
        }
        .map { it.toView(this) }
        .groupBy { it.due?.local()?.toLocalDate() ?: today }
        .toSortedMap()
}

private fun TaskDto.toView(db: Db) = TaskView(
    id = id,
    title = title,
    notes = notes,
    projectName = project?.let { db.projects[it]?.name },
    projectId = project,
    parent = parent,
    tags = tags.mapNotNull { db.tags[it]?.name },
    due = due,
    defer = defer,
    flagged = flagged,
    priority = priority,
    completed = completed,
    pending = false,
    checklistDone = checklist.count { it.completed },
    checklistTotal = checklist.size,
    estimateMin = estimateMin,
)

fun StampDto.label(): String {
    val moment = local() ?: return ""
    val date = moment.toLocalDate()
    val today = LocalDate.now()
    val clock = if (hasTime) {
        moment.format(DateTimeFormatter.ofPattern("HH:mm", Locale.getDefault()))
    } else {
        ""
    }
    val day = when {
        date == today -> "Today"
        date == today.minusDays(1) -> "Yesterday"
        date == today.plusDays(1) -> "Tomorrow"
        date.year == today.year -> {
            moment.format(DateTimeFormatter.ofPattern("EEE, d MMM", Locale.getDefault()))
        }
        else -> moment.format(DateTimeFormatter.ofPattern("d MMM yyyy", Locale.getDefault()))
    }
    return if (clock.isEmpty()) day else "$day · $clock"
}

fun StampDto.isToday(): Boolean = local()?.toLocalDate() == LocalDate.now()

fun StampDto.isOverdue(): Boolean {
    val moment = local() ?: return false
    val now = ZonedDateTime.now()
    return if (hasTime) moment.isBefore(now) else moment.toLocalDate().isBefore(LocalDate.now())
}

fun Db.completedTasks(): List<TaskView> {
    val days = settings.showCompletedDays.toLong()
    val cutoff = if (days > 0) {
        System.currentTimeMillis() - days * 24 * 60 * 60 * 1000
    } else {
        Long.MIN_VALUE
    }
    return taskViews(includeCompleted = true)
        .filter { it.completed && !it.trashed }
        .filter { task ->
            task.completedAt?.let {
                runCatching { Instant.parse(it).toEpochMilli() }.getOrNull() ?: Long.MAX_VALUE
            }?.let { it >= cutoff } ?: true
        }
        .sortedByDescending { it.completedAt.orEmpty() }
}

fun Db.trashedTasks(): List<TaskView> =
    taskViews(includeCompleted = true, includeTrashed = true)
        .filter { it.trashed }
        .sortedByDescending { it.modifiedAt }

fun Db.archivedProjects(): List<ProjectDto> =
    projects.values
        .filter { it.status == "done" || it.status == "dropped" }
        .sortedBy { it.name.lowercase() }

fun Db.reviewProjects(): List<ProjectDto> =
    projects.values
        .filter { it.status == "active" }
        .filter { project ->
            val next = project.nextReview ?: return@filter false
            runCatching { LocalDate.parse(next) }.getOrNull()?.let { !it.isAfter(LocalDate.now()) } == true
        }
        .sortedBy { it.nextReview.orEmpty() }

fun Db.priorityGroups(): List<Pair<String, List<TaskView>>> {
    val views = taskViews()
    return listOf(
        "High" to views.filter { it.priority == "high" },
        "Medium" to views.filter { it.priority == "medium" },
        "Low" to views.filter { it.priority == "low" },
    ).filter { it.second.isNotEmpty() }
}

fun Db.changedTasks(days: Long = settings.changedDays.toLong()): List<TaskView> {
    val cutoff = System.currentTimeMillis() - days * 24 * 60 * 60 * 1000
    return taskViews()
        .filter { task ->
            runCatching { Instant.parse(task.modifiedAt).toEpochMilli() }
                .getOrNull()
                ?.let { it >= cutoff } == true
        }
        .sortedByDescending { it.modifiedAt }
}

fun Db.attachmentTasks(): List<TaskView> =
    taskViews(includeCompleted = true).filter { it.attachments.isNotEmpty() }

private fun Db.tagFamily(tagId: String): Set<String> {
    val ids = mutableSetOf(tagId)
    var changed = true
    while (changed) {
        changed = false
        tags.values.forEach { tag ->
            if (tag.parent in ids && ids.add(tag.id)) changed = true
        }
    }
    return ids
}

fun Db.tasksForTag(tagId: String): List<TaskView> {
    val ids = tagFamily(tagId)
    return taskViews().filter { task ->
        task.tags.any { name -> tags.values.any { it.name == name && it.id in ids } }
    }
}

fun Db.tagUsageCount(tagId: String): Int {
    val ids = tagFamily(tagId)
    return tasks.values.count { task ->
        !task.completed && task.trashedAt == null && task.tags.any { it in ids }
    }
}
