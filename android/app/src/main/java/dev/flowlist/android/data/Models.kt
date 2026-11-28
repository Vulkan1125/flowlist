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
    val history: HistoryInfo? = null,
)

@Serializable
data class HistoryInfo(
    @SerialName("can_undo") val canUndo: Boolean = false,
    @SerialName("can_redo") val canRedo: Boolean = false,
    @SerialName("undo_label") val undoLabel: String? = null,
    @SerialName("redo_label") val redoLabel: String? = null,
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
data class GeoDto(val lat: Double = 0.0, val lon: Double = 0.0)

@Serializable
data class SettingsDto(
    @SerialName("default_review_days") val defaultReviewDays: Int = 7,
    @SerialName("show_completed_days") val showCompletedDays: Int = 30,
    @SerialName("changed_days") val changedDays: Int = 7,
    @SerialName("trash_retention_days") val trashRetentionDays: Int = 30,
    @SerialName("default_due_time") val defaultDueTime: String? = "17:00:00",
    @SerialName("default_defer_time") val defaultDeferTime: String? = null,
    @SerialName("current_location") val currentLocation: GeoDto? = null,
    @SerialName("nearby_radius_km") val nearbyRadiusKm: Double = 25.0,
    @SerialName("due_soon_days") val dueSoonDays: Int = 2,
    @SerialName("auto_archive_closed_projects") val autoArchiveClosedProjects: Boolean = true,
    @SerialName("forecast_tag") val forecastTag: String? = null,
    @SerialName("forecast_flagged") val forecastFlagged: Boolean = false,
    val sidebar: SidebarDto = SidebarDto(),
)

@Serializable
data class SidebarDto(
    @SerialName("show_perspectives") val showPerspectives: Boolean = true,
    @SerialName("show_priorities") val showPriorities: Boolean = true,
    @SerialName("show_library") val showLibrary: Boolean = true,
    @SerialName("show_saved") val showSaved: Boolean = true,
    @SerialName("show_tags") val showTags: Boolean = true,
    @SerialName("counts_perspectives") val countsPerspectives: Boolean = false,
    @SerialName("counts_priorities") val countsPriorities: Boolean = false,
    @SerialName("counts_library") val countsLibrary: Boolean = true,
    @SerialName("counts_tags") val countsTags: Boolean = false,
    val hidden: Set<String> = emptySet(),
    @SerialName("counts_hidden") val countsHidden: Set<String> = emptySet(),
    @SerialName("counts_configured") val countsConfigured: Boolean = false,
) {
    /// Android keeps its current counts visible until the first count toggle;
    /// after that it follows the shared per-row set exactly.
    fun isCountHidden(key: String): Boolean =
        if (countsConfigured) key in countsHidden else false
}

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
    val planned: StampDto? = null,
    @SerialName("estimate_min") val estimateMin: Int? = null,
    val flagged: Boolean = false,
    @SerialName("fullscreen_reminder") val fullscreenReminder: Boolean = false,
    @SerialName("reminder_at") val reminderAt: StampDto? = null,
    @SerialName("reminder_match_due") val reminderMatchDue: Boolean = true,
    val priority: String? = null,
    val completed: Boolean = false,
    @SerialName("completed_at") val completedAt: String? = null,
    @SerialName("trashed_at") val trashedAt: String? = null,
    @SerialName("child_order") val childOrder: List<String> = emptyList(),
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("modified_at") val modifiedAt: String = "",
    val repeat: JsonElement? = null,
    @SerialName("repeat_series") val repeatSeries: String? = null,
    val sequential: Boolean = false,
    @SerialName("steps_one_off") val stepsOneOff: Boolean = false,
)

/// A date in a batch edit: leave it alone, clear it, or set one value.
sealed class BatchDate {
    data object Keep : BatchDate()
    data object Clear : BatchDate()
    data class Set(val stamp: StampDto) : BatchDate()
}

/// Only the fields the user filled in are applied to the selection.
data class BatchEdit(
    val projectId: String? = null,
    val clearProject: Boolean = false,
    val addTagIds: Set<String> = emptySet(),
    val removeTagIds: Set<String> = emptySet(),
    val due: BatchDate = BatchDate.Keep,
    val defer: BatchDate = BatchDate.Keep,
    val planned: BatchDate = BatchDate.Keep,
    val priority: String? = null,
    val clearPriority: Boolean = false,
    val flagged: Boolean? = null,
    val estimate: Int? = null,
    val clearEstimate: Boolean = false,
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
    val notes: String = "",
    val status: String = "active",
    val flagged: Boolean = false,
    @SerialName("completed_at") val completedAt: String? = null,
    @SerialName("dropped_at") val droppedAt: String? = null,
    val folder: String? = null,
    @SerialName("fullscreen_reminder") val fullscreenReminder: Boolean = false,
    @SerialName("next_review") val nextReview: String? = null,
    @SerialName("review_interval_days") val reviewIntervalDays: Int = 7,
    @SerialName("complete_with_last_action") val completeWithLastAction: Boolean = false,
    val archived: Boolean = false,
    @SerialName("task_order") val taskOrder: List<String> = emptyList(),
    val sequential: Boolean = false,
    val kind: String = "parallel",
    val icon: String = "project",
    val tags: List<String> = emptyList(),
    val attachments: List<String> = emptyList(),
    val due: StampDto? = null,
    val defer: StampDto? = null,
    val planned: StampDto? = null,
    @SerialName("modified_at") val modifiedAt: String = "",
)

@Serializable
data class FolderDto(
    val id: String,
    val name: String = "",
    val parent: String? = null,
    @SerialName("fullscreen_reminder") val fullscreenReminder: Boolean = false,
    @SerialName("child_order") val childOrder: List<JsonElement> = emptyList(),
)

@Serializable
data class TagDto(
    val id: String,
    val name: String = "",
    val parent: String? = null,
    val status: String = "available",
    val location: GeoDto? = null,
)

@Serializable
data class PerspectiveDto(
    val id: String = "",
    val name: String = "",
    val filter: JsonElement? = null,
    @SerialName("group_by") val groupBy: String = "none",
    val sort: String = "manual",
    @SerialName("project_group_by") val projectGroupBy: String = "none",
    @SerialName("project_sort") val projectSort: String = "manual",
    @SerialName("show_completed") val showCompleted: Boolean = false,
    val position: Int = 0,
    val pinned: Boolean = false,
    val icon: String? = null,
    val description: String = "",
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
    val planned: StampDto?,
    val priority: String?,
    val flagged: Boolean,
    val fullscreenReminder: Boolean,
    val reminderAt: StampDto?,
    val reminderMatchDue: Boolean,
    val estimateMin: Int?,
    val tags: Set<String>,
    val repeat: JsonElement? = null,
    /// Apply the priority to the whole repeating series, not just this task.
    val priorityAll: Boolean = false,
    val sequential: Boolean = false,
    val stepsOneOff: Boolean = false,
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
    val planned: StampDto? = null,
    val flagged: Boolean,
    val fullscreenReminder: Boolean,
    val reminderAt: StampDto?,
    val reminderMatchDue: Boolean,
    val priority: String?,
    val completed: Boolean,
    val pending: Boolean,
    val checklistDone: Int,
    val checklistTotal: Int,
    val estimateMin: Int?,
    val attachments: List<String> = emptyList(),
    val completedAt: String? = null,
    val createdAt: String = "",
    val modifiedAt: String = "",
    val trashed: Boolean = false,
    /// The first unavailable tag (own or inherited) blocking this task.
    val blockedByTag: String? = null,
)

val TaskDto.isActive: Boolean
    get() = !completed && trashedAt == null

/// A task opts into full-screen reminders itself, through a parent task, or
/// through its project or any folder above the project.
fun Db.reminderStamp(task: TaskDto): StampDto? =
    if (task.reminderMatchDue) task.due else task.reminderAt

fun Db.wantsFullscreenReminder(task: TaskDto): Boolean {
    if (task.fullscreenReminder) return true
    var parent = task.parent
    while (parent != null) {
        val ancestor = tasks[parent] ?: break
        if (ancestor.fullscreenReminder) return true
        parent = ancestor.parent
    }
    var projectId = task.project
    var cursor = task.parent
    while (projectId == null && cursor != null) {
        val ancestor = tasks[cursor] ?: break
        projectId = ancestor.project
        cursor = ancestor.parent
    }
    val project = projectId?.let { projects[it] } ?: return false
    if (project.fullscreenReminder) return true
    var folder = project.folder
    while (folder != null) {
        val node = folders[folder] ?: break
        if (node.fullscreenReminder) return true
        folder = node.parent
    }
    return false
}

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
                planned = task.planned,
                flagged = task.flagged,
                fullscreenReminder = task.fullscreenReminder,
                reminderAt = task.reminderAt,
                reminderMatchDue = task.reminderMatchDue,
                priority = task.priority,
                completed = task.completed,
                pending = task.id in pending,
                checklistDone = task.checklist.count { it.completed },
                checklistTotal = task.checklist.size,
                estimateMin = task.estimateMin,
                attachments = task.attachments,
                completedAt = task.completedAt,
                createdAt = task.createdAt,
                modifiedAt = task.modifiedAt,
                trashed = task.trashedAt != null,
                blockedByTag = tagBlocking(task)?.name,
            )
        }
        .toList()

/// Every task that belongs to a project, including subtasks whose own project
/// field is empty because their parent carries it.
fun Db.projectTaskIds(project: String, includeCompleted: Boolean = false): Set<String> {
    val ids = tasks.values
        .filter {
            it.project == project &&
                it.trashedAt == null &&
                (includeCompleted || !it.completed)
        }
        .map { it.id }
        .toMutableSet()
    var grew = true
    while (grew) {
        grew = false
        for (task in tasks.values) {
            if (task.id in ids) continue
            if (!includeCompleted && task.completed) continue
            if (task.trashedAt != null) continue
            if (task.parent in ids) {
                ids += task.id
                grew = true
            }
        }
    }
    return ids
}

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
            val due = task.due?.local()?.toLocalDate()
            val planned = task.planned?.local()?.toLocalDate()
            (due != null && !due.isAfter(today)) || planned == today
        }
        .sortedWith(
            compareBy(
                { it.due?.at ?: it.planned?.at ?: "" },
                { it.title.lowercase() },
            ),
        )
        .map { it.toView(this) }
        .toList()
}

/// Tasks a home-screen widget shows for one of its modes. Completed rows are
/// appended only when the widget opts in, newest first, so checking one off
/// leaves a short visible trail instead of making the row vanish.
fun Db.widgetTasks(mode: String, showCompleted: Boolean): List<TaskView> {
    val today = LocalDate.now()
    val active = when (mode) {
        "inbox" -> inbox()
        "upcoming" -> tasks.values
            .asSequence()
            .filter { it.isActive && it.parent == null }
            .filter { task ->
                val due = task.due?.local()?.toLocalDate()
                due != null && due.isAfter(today) && !due.isAfter(today.plusDays(14))
            }
            .sortedWith(compareBy({ it.due?.at ?: "" }, { it.title.lowercase() }))
            .map { it.toView(this) }
            .toList()
        else -> today()
    }
    if (!showCompleted) return active
    val done = tasks.values
        .asSequence()
        .filter { it.completed && it.trashedAt == null }
        .filter { task ->
            when (mode) {
                "inbox" -> task.project == null && task.parent == null
                "upcoming" -> {
                    val due = task.due?.local()?.toLocalDate()
                    due != null && due.isAfter(today) && !due.isAfter(today.plusDays(14))
                }
                else -> {
                    val completedToday = task.completedAt
                        ?.let { runCatching { Instant.parse(it) }.getOrNull() }
                        ?.atZone(ZoneId.systemDefault())
                        ?.toLocalDate() == today
                    completedToday || task.due?.local()?.toLocalDate() == today
                }
            }
        }
        .sortedByDescending { it.completedAt.orEmpty() }
        .take(3)
        .map { it.toView(this) }
        .toList()
    return active + done
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

/// Forecast's dated tasks: due dates always, defer dates unless due-only, and
/// planned dates when the planned toggle is on. A task appears once per day
/// even when two of its dates land together.
fun Db.forecastTaskDays(
    dueOnly: Boolean,
    includePlanned: Boolean,
): Map<LocalDate, List<TaskView>> {
    val days = sortedMapOf<LocalDate, LinkedHashMap<String, TaskView>>()
    for (task in tasks.values) {
        if (!task.isActive || task.parent != null) continue
        val project = task.project?.let { projects[it] }
        if (project != null && (project.archived || (project.status != "active" && project.status != "on_hold"))) {
            continue
        }
        val dates = linkedSetOf<LocalDate>()
        task.due?.local()?.toLocalDate()?.let { dates += it }
        if (!dueOnly) {
            task.defer?.local()?.toLocalDate()?.let { dates += it }
            if (includePlanned) task.planned?.local()?.toLocalDate()?.let { dates += it }
        }
        for (date in dates) {
            days.getOrPut(date) { linkedMapOf() }[task.id] = task.toView(this)
        }
    }
    return days.mapValues { (_, values) ->
        values.values.sortedWith(
            compareBy(
                { it.due?.local()?.toLocalDate() ?: it.planned?.local()?.toLocalDate() ?: it.defer?.local()?.toLocalDate() },
                { it.title.lowercase() },
            ),
        )
    }
}

/// Project deadlines on their due date, plus defer and planned dates unless
/// due-only. A project appears once per day.
fun Db.forecastProjectDays(
    dueOnly: Boolean,
    includePlanned: Boolean,
): Map<LocalDate, List<ProjectDto>> {
    val days = sortedMapOf<LocalDate, LinkedHashMap<String, ProjectDto>>()
    for (project in projects.values) {
        if (project.archived || (project.status != "active" && project.status != "on_hold")) {
            continue
        }
        val dates = linkedSetOf<LocalDate>()
        project.due?.local()?.toLocalDate()?.let { dates += it }
        if (!dueOnly) {
            project.defer?.local()?.toLocalDate()?.let { dates += it }
            if (includePlanned) project.planned?.local()?.toLocalDate()?.let { dates += it }
        }
        for (date in dates) {
            days.getOrPut(date) { linkedMapOf() }[project.id] = project
        }
    }
    return days.mapValues { (_, values) ->
        values.values.sortedWith(
            compareBy(
                {
                    it.due?.local()?.toLocalDate()
                        ?: it.planned?.local()?.toLocalDate()
                        ?: it.defer?.local()?.toLocalDate()
                },
                { it.name.lowercase() },
            ),
        )
    }
}

/// Undated tasks that belong in Forecast because of the chosen tag or the
/// flagged option. Tagged wins, matching the Mac.
fun Db.forecastUndated(): Pair<List<TaskView>, List<TaskView>> {
    val tagged = mutableListOf<TaskView>()
    val flagged = mutableListOf<TaskView>()
    val tag = settings.forecastTag
    for (task in tasks.values) {
        if (!task.isActive || task.parent != null) continue
        if (task.due != null || task.defer != null || task.planned != null) continue
        val project = task.project?.let { projects[it] }
        if (project != null && (project.archived || (project.status != "active" && project.status != "on_hold"))) {
            continue
        }
        if (tag != null && task.tags.contains(tag)) {
            tagged += task.toView(this)
        } else if (settings.forecastFlagged && task.flagged) {
            flagged += task.toView(this)
        }
    }
    return tagged.sortedBy { it.title.lowercase() } to
        flagged.sortedBy { it.title.lowercase() }
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
    planned = planned,
    flagged = flagged,
    fullscreenReminder = fullscreenReminder,
    reminderAt = reminderAt,
    reminderMatchDue = reminderMatchDue,
    priority = priority,
    completed = completed,
    pending = false,
    checklistDone = checklist.count { it.completed },
    checklistTotal = checklist.size,
    estimateMin = estimateMin,
    attachments = attachments,
    completedAt = completedAt,
    createdAt = createdAt,
    modifiedAt = modifiedAt,
    trashed = trashedAt != null,
    blockedByTag = db.tagBlocking(this)?.name,
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
        .filter { it.archived || it.status == "done" || it.status == "dropped" }
        .sortedBy { it.name.lowercase() }

fun Db.reviewProjects(): List<ProjectDto> =
    projects.values
        .filter { it.status == "active" }
        .filter { project ->
            val next = project.nextReview ?: return@filter false
            runCatching { LocalDate.parse(next) }.getOrNull()?.let { !it.isAfter(LocalDate.now()) } == true
        }
        .sortedBy { it.nextReview.orEmpty() }

fun Db.flaggedTasks(): List<TaskView> =
    taskViews().filter { it.flagged }

/// Children in their outline order, with any child missing from the parent's
/// order appended by title so a hand-edited library still shows everything.
fun Db.orderedChildTasks(parentId: String): List<TaskDto> {
    val parent = tasks[parentId] ?: return emptyList()
    val seen = mutableSetOf<String>()
    val result = mutableListOf<TaskDto>()
    parent.childOrder.forEach { id ->
        tasks[id]?.takeIf { it.parent == parentId && it.isActive }?.let {
            result.add(it)
            seen.add(id)
        }
    }
    tasks.values
        .filter { it.parent == parentId && it.isActive && it.id !in seen }
        .sortedBy { it.title.lowercase() }
        .forEach { result.add(it) }
    return result
}

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
    taskViews().filter { it.attachments.isNotEmpty() }

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

/// Distinct active tasks carrying at least one tag. The Tags destination is a
/// list of tag rows, but its drawer badge counts the work behind them.
fun Db.taggedTaskCount(): Int =
    tasks.values.count { it.isActive && it.tags.isNotEmpty() }

/// Distinct active tasks carrying a tag that has a place attached.
fun Db.geotaggedTaskCount(): Int {
    val located = tags.values.filter { it.location != null }.map { it.id }.toSet()
    return tasks.values.count { it.isActive && it.tags.any { id -> id in located } }
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

/// The URL part of an attachment. A named link stores `url<TAB>name`.
fun attachmentUrl(value: String): String = value.substringBefore('\t')

/// The name shown in an attachment row: the optional name, or the last path
/// segment for an unnamed link or file.
fun attachmentDisplayName(value: String): String {
    val name = value.substringAfter('\t', "").trim()
    if (name.isNotEmpty()) return name
    return value.substringAfterLast('/')
}

/// Files the phone can render itself: images and PDFs.
fun canPreviewAttachment(path: String): Boolean {
    if (path.startsWith("http://") || path.startsWith("https://")) return false
    val extension = path.substringAfterLast('.', "").lowercase()
    return extension in setOf("png", "jpg", "jpeg", "webp", "gif", "bmp", "pdf")
}

/// The site's own favicon. Fetching it from the host avoids a third-party
/// favicon service; Coil shows the chain icon when it is missing.
fun faviconUrls(value: String): List<String> {
    val url = attachmentUrl(value)
    val host = runCatching { java.net.URI(url).host }.getOrNull() ?: return emptyList()
    val schemes = if (url.startsWith("http://")) listOf("http", "https") else listOf("https", "http")
    return schemes.flatMap { scheme ->
        listOf(
            "$scheme://$host/favicon.ico",
            "$scheme://$host/favicon.png",
            "$scheme://$host/apple-touch-icon.png",
            "$scheme://$host/apple-touch-icon-precomposed.png",
        )
    }
}

fun linkWithName(url: String, name: String): String {
    val trimmed = name.trim()
    return if (trimmed.isEmpty()) url.trim() else "${url.trim()}\t$trimmed"
}

/// The first tag in a task's own or project's tag chain that is not
/// available. The chain includes ancestors, matching the Mac's inherited
/// blocked/on-hold rules.
fun Db.tagBlocking(task: TaskDto): TagDto? {
    val ids = buildList {
        addAll(task.tags)
        task.project?.let { project -> addAll(projects[project]?.tags.orEmpty()) }
    }
    for (id in ids) {
        var tag = tags[id]
        while (tag != null) {
            if (tag.status != "available") return tag
            tag = tag.parent?.let { tags[it] }
        }
    }
    return null
}

/// True when the tag or any ancestor is blocked or on hold.
fun Db.tagIsBlocked(tagId: String): Boolean {
    var tag = tags[tagId]
    while (tag != null) {
        if (tag.status != "available") return true
        tag = tag.parent?.let { tags[it] }
    }
    return false
}

fun tagStatusLabel(status: String): String = when (status) {
    "blocked" -> "Blocked"
    "on_hold" -> "On Hold"
    else -> "Available"
}

/// A tag's `parent/child` path, as the batch and quick-entry commands expect.
fun Db.tagPath(id: String): String? {
    val names = mutableListOf<String>()
    var cursor: TagDto? = tags[id]
    var guard = 0
    while (cursor != null && guard < 64) {
        names.add(0, cursor.name)
        cursor = cursor.parent?.let { tags[it] }
        guard += 1
    }
    return names.takeIf { it.isNotEmpty() }?.joinToString("/")
}

/// Whether a task can be worked on right now, matching the Mac's availability
/// rules: open project, defer dates passed, parent available, no blocked tag
/// and no earlier task in a sequential list.
fun Db.isAvailableNow(task: TaskView): Boolean {
    if (task.completed) return false
    val now = ZonedDateTime.now()
    if (task.defer?.local()?.isAfter(now) == true) return false
    var projectId = task.projectId
    var parentId = task.parent
    while (parentId != null) {
        val parent = tasks[parentId] ?: break
        if (parent.completed) return false
        if (parent.defer?.local()?.isAfter(now) == true) return false
        if (tagBlocking(parent) != null) return false
        if (sequenceBlocker(parent.id) != null) return false
        if (projectId == null) projectId = parent.project
        parentId = parent.parent
    }
    val project = projectId?.let { projects[it] }
    if (project != null) {
        if (project.status == "done" || project.status == "dropped" || project.status == "on_hold") {
            return false
        }
        if (project.defer?.local()?.isAfter(now) == true) return false
    }
    if (task.blockedByTag != null) return false
    return sequenceBlocker(task.id) == null
}

/// The earlier incomplete task that blocks this one in a sequential list, or
/// null when completing it is allowed.
fun Db.sequenceBlocker(taskId: String): TaskDto? {
    val task = tasks[taskId] ?: return null
    if (task.completed) return null
    val parent = task.parent?.let { tasks[it] }
    val project = task.project?.let { projects[it] }
    val (order, sequential) = when {
        parent != null -> parent.childOrder to parent.sequential
        project != null -> project.taskOrder to (project.sequential || project.kind == "sequential")
        else -> return null
    }
    if (!sequential) return null
    for (id in order) {
        if (id == taskId) break
        val sibling = tasks[id] ?: continue
        if (!sibling.completed && sibling.trashedAt == null) return sibling
    }
    return null
}
