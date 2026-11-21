package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.clickable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.FolderDto
import dev.flowlist.android.data.PerspectiveDto
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.Snapshot
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.taskViews
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

data class PerspectiveEditorTarget(val perspective: PerspectiveDto? = null)

data class PerspectiveDraft(
    val name: String = "",
    val text: String = "",
    val project: String? = null,
    val tag: String? = null,
    val priority: String? = null,
    val due: String? = null,
    val group: String = "none",
    val sort: String = "manual",
    val showCompleted: Boolean = false,
)

private fun JsonObject.text(key: String): String? =
    (this[key] as? JsonPrimitive)?.takeIf { it.isString }?.content

private fun JsonObject.boolean(key: String): Boolean? =
    (this[key] as? JsonPrimitive)?.content?.toBooleanStrictOrNull()

private fun JsonObject.number(key: String): Int? =
    (this[key] as? JsonPrimitive)?.content?.toIntOrNull()

fun draftFrom(perspective: PerspectiveDto?): PerspectiveDraft {
    if (perspective == null) return PerspectiveDraft()
    var text = ""
    var project: String? = null
    var tag: String? = null
    var priority: String? = null
    var due: String? = null
    val root = perspective.filter as? JsonObject
    val nodes = root?.get("nodes") as? JsonArray
    nodes?.forEach { element ->
        val condition = (element as? JsonObject)?.get("condition") as? JsonObject ?: return@forEach
        when (condition.text("type")) {
            "text" -> text = condition.text("value").orEmpty()
            "project" -> project = condition.text("id")
            "tag" -> tag = condition.text("id")
            "priority" -> priority = condition.text("value")
            "due" -> due = condition.text("window")
        }
    }
    return PerspectiveDraft(
        name = perspective.name,
        text = text,
        project = project,
        tag = tag,
        priority = priority,
        due = due,
        group = perspective.groupBy,
        sort = perspective.sort,
        showCompleted = perspective.showCompleted,
    )
}

fun PerspectiveDraft.toFilter(): JsonObject = buildJsonObject {
    put("type", "all")
    put(
        "nodes",
        buildJsonArray {
            if (text.isNotBlank()) {
                add(
                    buildJsonObject {
                        put("type", "condition")
                        put(
                            "condition",
                            buildJsonObject {
                                put("type", "text")
                                put("value", text.trim())
                            },
                        )
                    },
                )
            }
            project?.let { id ->
                add(
                    buildJsonObject {
                        put("type", "condition")
                        put("condition", buildJsonObject { put("type", "project"); put("id", id) })
                    },
                )
            }
            tag?.let { id ->
                add(
                    buildJsonObject {
                        put("type", "condition")
                        put(
                            "condition",
                            buildJsonObject {
                                put("type", "tag")
                                put("id", id)
                                put("include_children", true)
                            },
                        )
                    },
                )
            }
            priority?.let { value ->
                add(
                    buildJsonObject {
                        put("type", "condition")
                        put("condition", buildJsonObject { put("type", "priority"); put("value", value) })
                    },
                )
            }
            due?.let { window ->
                add(
                    buildJsonObject {
                        put("type", "condition")
                        put(
                            "condition",
                            buildJsonObject { put("type", "due"); put("window", window) },
                        )
                    },
                )
            }
        },
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PerspectiveEditorSheet(
    target: PerspectiveEditorTarget,
    projects: List<ProjectDto>,
    tags: List<TagDto>,
    onDismiss: () -> Unit,
    onSave: (PerspectiveDraft) -> Unit,
    onDelete: (() -> Unit)?,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val initial = remember(target.perspective?.id) { draftFrom(target.perspective) }
    var name by remember(target.perspective?.id) { mutableStateOf(initial.name) }
    var text by remember(target.perspective?.id) { mutableStateOf(initial.text) }
    var project by remember(target.perspective?.id) { mutableStateOf(initial.project) }
    var tag by remember(target.perspective?.id) { mutableStateOf(initial.tag) }
    var priority by remember(target.perspective?.id) { mutableStateOf(initial.priority) }
    var due by remember(target.perspective?.id) { mutableStateOf(initial.due) }
    var group by remember(target.perspective?.id) { mutableStateOf(initial.group) }
    var sort by remember(target.perspective?.id) { mutableStateOf(initial.sort) }
    var showCompleted by remember(target.perspective?.id) { mutableStateOf(initial.showCompleted) }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp)
                .padding(bottom = 30.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = if (target.perspective == null) "New perspective" else "Edit perspective",
                    style = MaterialTheme.typography.titleLarge,
                    color = colors.ink,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Rounded.Close, "Close", tint = colors.faint)
                }
            }
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                placeholder = { Text("Perspective name", color = colors.faint) },
                singleLine = true,
                colors = perspectiveFieldColors(),
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = text,
                onValueChange = { text = it },
                placeholder = { Text("Contains text (optional)", color = colors.faint) },
                singleLine = true,
                colors = perspectiveFieldColors(),
                modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
            )

            PerspectiveLabel("Project")
            PillsRow {
                PerspectivePill("Any", project == null) { project = null }
                projects.sortedBy { it.name.lowercase() }.forEach { item ->
                    PerspectivePill(item.name, project == item.id) { project = item.id }
                }
            }

            PerspectiveLabel("Tag")
            PillsRow {
                PerspectivePill("Any", tag == null) { tag = null }
                tags.sortedBy { it.name.lowercase() }.forEach { item ->
                    PerspectivePill("#${item.name}", tag == item.id) { tag = item.id }
                }
            }

            PerspectiveLabel("Priority")
            PillsRow {
                PerspectivePill("Any", priority == null) { priority = null }
                PerspectivePill("P1", priority == "high") { priority = "high" }
                PerspectivePill("P2", priority == "medium") { priority = "medium" }
                PerspectivePill("P3", priority == "low") { priority = "low" }
            }

            PerspectiveLabel("Due")
            PillsRow {
                listOf(
                    null to "Any",
                    "overdue" to "Overdue",
                    "today" to "Today",
                    "tomorrow" to "Tomorrow",
                    "next_7_days" to "Next 7 days",
                    "next_30_days" to "Next 30 days",
                    "no_date" to "No date",
                ).forEach { (key, label) ->
                    PerspectivePill(label, due == key) { due = key }
                }
            }

            PerspectiveLabel("Group by")
            PillsRow {
                listOf(
                    "none" to "None",
                    "project" to "Project",
                    "due_date" to "Due date",
                    "tag" to "Tag",
                    "folder" to "Folder",
                ).forEach { (key, label) ->
                    PerspectivePill(label, group == key) { group = key }
                }
            }

            PerspectiveLabel("Sort")
            PillsRow {
                listOf(
                    "manual" to "Manual",
                    "due" to "Due",
                    "name" to "Name",
                    "priority" to "Priority",
                    "flagged" to "Flagged",
                    "estimate" to "Estimate",
                ).forEach { (key, label) ->
                    PerspectivePill(label, sort == key) { sort = key }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth().padding(top = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "Show completed tasks",
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.ink,
                    modifier = Modifier.weight(1f),
                )
                Switch(
                    checked = showCompleted,
                    onCheckedChange = { showCompleted = it },
                    colors = SwitchDefaults.colors(
                        checkedThumbColor = colors.onAccent,
                        checkedTrackColor = colors.accent,
                    ),
                )
            }

            Spacer(modifier = Modifier.height(18.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (onDelete != null) {
                    TextButton(onClick = onDelete) { Text("Delete", color = colors.danger) }
                }
                Spacer(modifier = Modifier.weight(1f))
                TextButton(onClick = onDismiss) { Text("Cancel", color = colors.faint) }
                Spacer(modifier = Modifier.width(6.dp))
                Button(
                    onClick = {
                        onSave(
                            PerspectiveDraft(
                                name = name.trim(),
                                text = text.trim(),
                                project = project,
                                tag = tag,
                                priority = priority,
                                due = due,
                                group = group,
                                sort = sort,
                                showCompleted = showCompleted,
                            ),
                        )
                    },
                    enabled = name.isNotBlank(),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = colors.accent,
                        contentColor = colors.onAccent,
                    ),
                ) {
                    Text("Save")
                }
            }
        }
    }
}

@Composable
private fun PerspectiveLabel(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelMedium,
        fontWeight = FontWeight.SemiBold,
        color = flow.faint,
        modifier = Modifier.padding(top = 16.dp, bottom = 8.dp),
    )
}

@Composable
private fun PillsRow(content: @Composable () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        content()
    }
}

@Composable
private fun PerspectivePill(label: String, selected: Boolean, onClick: () -> Unit) {
    val colors = flow
    Box(
        modifier = Modifier
            .heightIn(min = 34.dp)
            .background(
                color = if (selected) colors.field else Color.Transparent,
                shape = RoundedCornerShape(9.dp),
            )
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 7.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = if (selected) colors.ink else colors.faint,
            maxLines = 1,
            softWrap = false,
        )
    }
}

@Composable
private fun perspectiveFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedBorderColor = flow.hairline,
    unfocusedBorderColor = flow.hairline,
    focusedContainerColor = flow.background,
    unfocusedContainerColor = flow.background,
    cursorColor = flow.accent,
    focusedTextColor = flow.ink,
    unfocusedTextColor = flow.ink,
)

@Composable
fun SavedPerspectiveView(
    perspective: PerspectiveDto,
    snapshot: Snapshot?,
    swipeActions: Boolean,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
) {
    val colors = flow
    val db = snapshot?.db
    val all = remember(snapshot?.revision, perspective) {
        db?.taskViews(includeCompleted = perspective.showCompleted).orEmpty()
    }
    val children = remember(snapshot?.revision) {
        db?.taskViews()
            ?.filter { it.parent != null }
            ?.groupBy { it.parent!! }
            .orEmpty()
    }
    val tasks = remember(all, perspective) {
        val matched = all.filter { task ->
            task.parent == null && matchesFilter(db, task, perspective.filter)
        }
        sortTasks(matched, perspective.sort)
    }
    val sections = remember(tasks, perspective) {
        if (perspective.groupBy == "none") {
            listOf(null to tasks)
        } else {
            val map = LinkedHashMap<String, MutableList<TaskView>>()
            tasks.forEach { task ->
                val label = groupLabel(db, task, perspective.groupBy)
                map.getOrPut(label) { mutableListOf() }.add(task)
            }
            map.map { it.key to it.value.toList() }
        }
    }

    if (tasks.isEmpty()) {
        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Text(
                text = "No tasks match this perspective.",
                style = MaterialTheme.typography.bodyMedium,
                color = colors.faint,
            )
        }
        return
    }

    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        sections.forEach { (label, items) ->
            if (label != null) {
                item(key = "section-$label") { SectionLabel(label) }
            }
            items(items) { task ->
                TaskTreeItem(
                    task = task,
                    children = children,
                    pending = task.id in pending,
                    swipeActions = swipeActions,
                    depth = 0,
                    onToggle = onToggle,
                    onTrash = onTrash,
                    onClick = onClick,
                )
            }
        }
    }
}

private fun matchesFilter(db: Db?, task: TaskView, filter: JsonElement?): Boolean {
    if (filter == null) return true
    return matchesNode(db, task, filter)
}

private fun matchesNode(db: Db?, task: TaskView, element: JsonElement): Boolean {
    val node = element as? JsonObject ?: return true
    return when (node.text("type")) {
        "all" -> (node["nodes"] as? JsonArray)?.all { matchesNode(db, task, it) } ?: true
        "any" -> (node["nodes"] as? JsonArray)?.any { matchesNode(db, task, it) } ?: true
        "not" -> (node["node"] as? JsonElement)?.let { !matchesNode(db, task, it) } ?: true
        "condition" -> {
            val condition = node["condition"] as? JsonObject ?: return true
            matchesCondition(db, task, condition)
        }
        else -> true
    }
}

private fun matchesCondition(db: Db?, task: TaskView, condition: JsonObject): Boolean {
    val today = LocalDate.now()
    val due = task.due?.local()?.toLocalDate()
    return when (condition.text("type")) {
        "text" -> {
            val needle = condition.text("value").orEmpty().lowercase()
            needle.isBlank() ||
                task.title.lowercase().contains(needle) ||
                task.notes.lowercase().contains(needle) ||
                task.projectName?.lowercase()?.contains(needle) == true ||
                task.tags.any { it.lowercase().contains(needle) }
        }
        "project" -> task.projectId == condition.text("id")
        "folder" -> {
            val folder = condition.text("id") ?: return true
            val includeChildren = condition.boolean("include_children") ?: false
            folderContains(db, task.projectId?.let { db?.projects?.get(it)?.folder }, folder, includeChildren)
        }
        "tag" -> {
            val tagId = condition.text("id") ?: return true
            val includeChildren = condition.boolean("include_children") ?: false
            val names = tagNames(db, tagId, includeChildren)
            task.tags.any { it in names }
        }
        "in_inbox" -> task.projectId == null && task.parent == null
        "flagged" -> task.flagged == (condition.boolean("value") ?: true)
        "priority" -> task.priority == condition.text("value")
        "status" -> when (condition.text("value")) {
            "completed" -> task.completed
            "available" -> !task.completed
            "deferred" -> task.defer?.local()?.toLocalDate()?.isAfter(today) == true
            else -> true
        }
        "due" -> windowMatches(due, condition.text("window"), today)
        "deferred" -> windowMatches(task.defer?.local()?.toLocalDate(), condition.text("window"), today)
        "has_repeat" -> {
            val expected = condition.boolean("value") ?: true
            (db?.tasks?.get(task.id)?.repeat != null) == expected
        }
        "estimate" -> {
            val minutes = task.estimateMin ?: return true
            val min = condition.number("min_minutes")
            val max = condition.number("max_minutes")
            (min == null || minutes >= min) && (max == null || minutes <= max)
        }
        else -> true
    }
}

private fun windowMatches(date: LocalDate?, window: String?, today: LocalDate): Boolean {
    if (window == null) return true
    return when (window) {
        "overdue" -> date != null && date.isBefore(today)
        "today" -> date == today
        "tomorrow" -> date == today.plusDays(1)
        "next_7_days" -> date != null && !date.isBefore(today) && !date.isAfter(today.plusDays(7))
        "next_30_days" -> date != null && !date.isBefore(today) && !date.isAfter(today.plusDays(30))
        "no_date" -> date == null
        else -> true
    }
}

private fun folderContains(db: Db?, folder: String?, target: String, includeChildren: Boolean): Boolean {
    if (folder == null) return false
    if (!includeChildren) return folder == target
    var current: String? = folder
    while (current != null) {
        if (current == target) return true
        current = db?.folders?.get(current)?.parent
    }
    return false
}

private fun tagNames(db: Db?, root: String, includeChildren: Boolean): Set<String> {
    if (db == null) return emptySet()
    val ids = mutableSetOf(root)
    if (includeChildren) {
        var changed = true
        while (changed) {
            changed = false
            db.tags.values.forEach { tag ->
                if (tag.parent in ids && ids.add(tag.id)) changed = true
            }
        }
    }
    return ids.mapNotNull { db.tags[it]?.name }.toSet()
}

private fun sortTasks(tasks: List<TaskView>, sort: String): List<TaskView> {
    val comparator: Comparator<TaskView> = when (sort) {
        "due" -> compareBy<TaskView> { it.due?.local()?.toLocalDate() ?: LocalDate.MAX }
            .thenBy { it.title.lowercase() }
        "name" -> compareBy { it.title.lowercase() }
        "priority" -> compareBy<TaskView> {
            when (it.priority) {
                "high" -> 0
                "medium" -> 1
                "low" -> 2
                else -> 3
            }
        }.thenBy { it.title.lowercase() }
        "flagged" -> compareByDescending<TaskView> { it.flagged }.thenBy { it.title.lowercase() }
        "estimate" -> compareBy<TaskView> { it.estimateMin ?: Int.MAX_VALUE }
            .thenBy { it.title.lowercase() }
        else -> return tasks
    }
    return tasks.sortedWith(comparator)
}

private fun groupLabel(db: Db?, task: TaskView, group: String): String {
    val due = task.due?.local()?.toLocalDate()
    val today = LocalDate.now()
    return when (group) {
        "project" -> task.projectName ?: "Inbox"
        "tag" -> task.tags.firstOrNull()?.let { "#$it" } ?: "No tag"
        "folder" -> task.projectId
            ?.let { db?.projects?.get(it)?.folder }
            ?.let { db?.folders?.get(it)?.name }
            ?: "No folder"
        "due_date" -> when {
            due == null -> "No date"
            due.isBefore(today) -> "Overdue"
            due == today -> "Today"
            due == today.plusDays(1) -> "Tomorrow"
            else -> due.format(DateTimeFormatter.ofPattern("EEEE, d MMMM", Locale.getDefault()))
        }
        else -> "Tasks"
    }
}
