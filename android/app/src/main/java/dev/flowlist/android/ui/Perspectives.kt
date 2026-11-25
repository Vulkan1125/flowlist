package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.clickable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CheckboxDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.FolderDto
import dev.flowlist.android.data.PerspectiveDto
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.Snapshot
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.reminderStamp
import dev.flowlist.android.data.taskViews
import dev.flowlist.android.data.wantsFullscreenReminder
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
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
    val projects: Set<String> = emptySet(),
    val tags: Set<String> = emptySet(),
    val priorities: Set<String> = emptySet(),
    val dues: Set<String> = emptySet(),
    val defers: Set<String> = emptySet(),
    val inboxOnly: Boolean = false,
    val flaggedOnly: Boolean = false,
    val status: String = "any",
    val tagChildren: Boolean = true,
    val estimateMin: Int? = null,
    val estimateMax: Int? = null,
    val group: String = "project",
    val sort: String = "due",
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
    val projects = mutableSetOf<String>()
    val tags = mutableSetOf<String>()
    val priorities = mutableSetOf<String>()
    val dues = mutableSetOf<String>()
    val defers = mutableSetOf<String>()
    var inboxOnly = false
    var flaggedOnly = false
    var status = "any"
    var tagChildren = true
    var estimateMin: Int? = null
    var estimateMax: Int? = null

    fun visit(element: JsonElement?) {
        val node = element as? JsonObject ?: return
        when (node.text("type")) {
            "all", "any" -> (node["nodes"] as? JsonArray)?.forEach { visit(it) }
            "condition" -> {
                val condition = node["condition"] as? JsonObject ?: return
                when (condition.text("type")) {
                    "text" -> text = condition.text("value").orEmpty()
                    "project" -> condition.text("id")?.let { projects += it }
                    "tag" -> {
                        condition.text("id")?.let { tags += it }
                        condition.boolean("include_children")?.let { tagChildren = it }
                    }
                    "priority" -> condition.text("value")?.let { priorities += it }
                    "due" -> condition.text("window")?.let { dues += it }
                    "deferred" -> condition.text("window")?.let { defers += it }
                    "in_inbox" -> inboxOnly = true
                    "flagged" -> {
                        if (condition.boolean("value") != false) flaggedOnly = true
                    }
                    "status" -> status = condition.text("value") ?: "any"
                    "estimate" -> {
                        estimateMin = condition.number("min_minutes")
                        estimateMax = condition.number("max_minutes")
                    }
                }
            }
        }
    }
    visit(perspective.filter)
    return PerspectiveDraft(
        name = perspective.name,
        text = text,
        projects = projects,
        tags = tags,
        priorities = priorities,
        dues = dues,
        defers = defers,
        inboxOnly = inboxOnly,
        flaggedOnly = flaggedOnly,
        status = status,
        tagChildren = tagChildren,
        estimateMin = estimateMin,
        estimateMax = estimateMax,
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
            fun condition(value: JsonObject) {
                add(
                    buildJsonObject {
                        put("type", "condition")
                        put("condition", value)
                    },
                )
            }

            fun group(values: List<JsonObject>) {
                when (values.size) {
                    0 -> Unit
                    1 -> condition(values.first())
                    else -> add(
                        buildJsonObject {
                            put("type", "any")
                            put(
                                "nodes",
                                buildJsonArray {
                                    values.forEach { value ->
                                        add(
                                            buildJsonObject {
                                                put("type", "condition")
                                                put("condition", value)
                                            },
                                        )
                                    }
                                },
                            )
                        },
                    )
                }
            }

            if (text.isNotBlank()) {
                condition(buildJsonObject { put("type", "text"); put("value", text.trim()) })
            }
            if (inboxOnly) {
                condition(buildJsonObject { put("type", "in_inbox") })
            }
            if (flaggedOnly) {
                condition(buildJsonObject { put("type", "flagged"); put("value", true) })
            }
            if (status != "any") {
                condition(buildJsonObject { put("type", "status"); put("value", status) })
            }
            group(
                projects.sorted().map { id ->
                    buildJsonObject { put("type", "project"); put("id", id) }
                },
            )
            group(
                tags.sorted().map { id ->
                    buildJsonObject {
                        put("type", "tag")
                        put("id", id)
                        put("include_children", tagChildren)
                    }
                },
            )
            group(
                priorities.sorted().map { value ->
                    buildJsonObject { put("type", "priority"); put("value", value) }
                },
            )
            group(
                dues.sorted().map { window ->
                    buildJsonObject { put("type", "due"); put("window", window) }
                },
            )
            group(
                defers.sorted().map { window ->
                    buildJsonObject { put("type", "deferred"); put("window", window) }
                },
            )
            if (estimateMin != null || estimateMax != null) {
                condition(
                    buildJsonObject {
                        put("type", "estimate")
                        put("min_minutes", estimateMin?.let { JsonPrimitive(it) } ?: JsonNull)
                        put("max_minutes", estimateMax?.let { JsonPrimitive(it) } ?: JsonNull)
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
    var projectIds by remember(target.perspective?.id) { mutableStateOf(initial.projects) }
    var tagIds by remember(target.perspective?.id) { mutableStateOf(initial.tags) }
    var priorityIds by remember(target.perspective?.id) { mutableStateOf(initial.priorities) }
    var dueIds by remember(target.perspective?.id) { mutableStateOf(initial.dues) }
    var deferIds by remember(target.perspective?.id) { mutableStateOf(initial.defers) }
    var inboxOnly by remember(target.perspective?.id) { mutableStateOf(initial.inboxOnly) }
    var flaggedOnly by remember(target.perspective?.id) { mutableStateOf(initial.flaggedOnly) }
    var status by remember(target.perspective?.id) { mutableStateOf(initial.status) }
    var tagChildren by remember(target.perspective?.id) { mutableStateOf(initial.tagChildren) }
    var estimateMin by remember(target.perspective?.id) {
        mutableStateOf(initial.estimateMin?.toString().orEmpty())
    }
    var estimateMax by remember(target.perspective?.id) {
        mutableStateOf(initial.estimateMax?.toString().orEmpty())
    }
    var group by remember(target.perspective?.id) { mutableStateOf(initial.group) }
    var sort by remember(target.perspective?.id) { mutableStateOf(initial.sort) }
    var showCompleted by remember(target.perspective?.id) { mutableStateOf(initial.showCompleted) }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        modifier = Modifier.statusBarsPadding(),
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

            Row(
                modifier = Modifier.fillMaxWidth().padding(top = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(
                    modifier = Modifier
                        .weight(1f)
                        .clickable {
                            inboxOnly = !inboxOnly
                            if (inboxOnly) {
                                projectIds = emptySet()
                                tagIds = emptySet()
                            }
                        },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Checkbox(
                        checked = inboxOnly,
                        onCheckedChange = null,
                        colors = CheckboxDefaults.colors(
                            checkedColor = colors.accent,
                            uncheckedColor = colors.hairline,
                        ),
                    )
                    Text("Inbox only", style = MaterialTheme.typography.bodyMedium, color = colors.ink)
                }
                Row(
                    modifier = Modifier
                        .weight(1f)
                        .clickable { flaggedOnly = !flaggedOnly },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Checkbox(
                        checked = flaggedOnly,
                        onCheckedChange = null,
                        colors = CheckboxDefaults.colors(
                            checkedColor = colors.accent,
                            uncheckedColor = colors.hairline,
                        ),
                    )
                    Text("Flagged only", style = MaterialTheme.typography.bodyMedium, color = colors.ink)
                }
            }

            PerspectiveDropdown(
                label = "Status",
                value = when (status) {
                    "available" -> "Available"
                    "blocked" -> "Blocked"
                    "deferred" -> "Deferred"
                    "completed" -> "Completed"
                    else -> "Any status"
                },
                options = listOf(
                    "any" to "Any status",
                    "available" to "Available",
                    "blocked" to "Blocked",
                    "deferred" to "Deferred",
                    "completed" to "Completed",
                ),
            ) { status = it ?: "any" }

            Spacer(modifier = Modifier.height(6.dp))
            PerspectiveMultiSelect(
                label = "Projects",
                selected = projectIds,
                anyLabel = "Any project",
                options = projects.sortedBy { it.name.lowercase() }.map { it.id to it.name },
                onToggle = { id ->
                    projectIds = if (id in projectIds) projectIds - id else projectIds + id
                },
                onClear = { projectIds = emptySet() },
            )

            PerspectiveMultiSelect(
                label = "Tags",
                selected = tagIds,
                anyLabel = "Any tag",
                options = tags.sortedBy { it.name.lowercase() }.map { it.id to "#${it.name}" },
                onToggle = { id ->
                    tagIds = if (id in tagIds) tagIds - id else tagIds + id
                },
                onClear = { tagIds = emptySet() },
            )
            if (tagIds.isNotEmpty()) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(top = 2.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "Include child tags",
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.ink,
                        modifier = Modifier.weight(1f),
                    )
                    Switch(
                        checked = tagChildren,
                        onCheckedChange = { tagChildren = it },
                        colors = SwitchDefaults.colors(
                            checkedThumbColor = colors.onAccent,
                            checkedTrackColor = colors.accent,
                        ),
                    )
                }
            }

            PerspectiveMultiSelect(
                label = "Priorities",
                selected = priorityIds,
                anyLabel = "Any priority",
                options = listOf("high" to "P1", "medium" to "P2", "low" to "P3"),
                onToggle = { value ->
                    priorityIds = if (value in priorityIds) {
                        priorityIds - value
                    } else {
                        priorityIds + value
                    }
                },
                onClear = { priorityIds = emptySet() },
            )

            PerspectiveMultiSelect(
                label = "Due dates",
                selected = dueIds,
                anyLabel = "Any date",
                options = listOf(
                    "overdue" to "Overdue",
                    "today" to "Today",
                    "tomorrow" to "Tomorrow",
                    "next_7_days" to "Next 7 days",
                    "next_30_days" to "Next 30 days",
                    "no_date" to "No date",
                ),
                onToggle = { value ->
                    dueIds = if (value in dueIds) dueIds - value else dueIds + value
                },
                onClear = { dueIds = emptySet() },
            )

            PerspectiveMultiSelect(
                label = "Defer dates",
                selected = deferIds,
                anyLabel = "Any defer date",
                options = listOf(
                    "today" to "Today",
                    "tomorrow" to "Tomorrow",
                    "next_7_days" to "Next 7 days",
                    "next_30_days" to "Next 30 days",
                    "no_date" to "No date",
                ),
                onToggle = { value ->
                    deferIds = if (value in deferIds) deferIds - value else deferIds + value
                },
                onClear = { deferIds = emptySet() },
            )

            Row(
                modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                OutlinedTextField(
                    value = estimateMin,
                    onValueChange = { value ->
                        estimateMin = value.filter { it.isDigit() }.take(4)
                    },
                    placeholder = { Text("Min minutes", color = colors.faint) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    colors = perspectiveFieldColors(),
                    modifier = Modifier.weight(1f),
                )
                OutlinedTextField(
                    value = estimateMax,
                    onValueChange = { value ->
                        estimateMax = value.filter { it.isDigit() }.take(4)
                    },
                    placeholder = { Text("Max minutes", color = colors.faint) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    colors = perspectiveFieldColors(),
                    modifier = Modifier.weight(1f),
                )
            }
            Text(
                text = "Filter by estimate. Leave either blank for any duration.",
                style = MaterialTheme.typography.labelSmall,
                color = colors.faint,
                modifier = Modifier.padding(start = 2.dp, top = 4.dp),
            )

            PerspectiveDropdown(
                label = "Group by",
                value = when (group) {
                    "project" -> "Project"
                    "due_date" -> "Due date"
                    "defer_date" -> "Defer date"
                    "tag" -> "Tag"
                    "folder" -> "Folder"
                    "flagged" -> "Flagged"
                    "estimate" -> "Estimate"
                    else -> "None"
                },
                options = listOf(
                    "none" to "None",
                    "project" to "Project",
                    "folder" to "Folder",
                    "tag" to "Tag",
                    "due_date" to "Due date",
                    "defer_date" to "Defer date",
                    "flagged" to "Flagged",
                    "estimate" to "Estimate",
                ),
            ) { group = it ?: "none" }

            PerspectiveDropdown(
                label = "Sort",
                value = when (sort) {
                    "due" -> "Due"
                    "defer" -> "Defer"
                    "name" -> "Name"
                    "priority" -> "Priority"
                    "flagged" -> "Flagged"
                    "estimate" -> "Estimate"
                    "created" -> "Created"
                    "completed_at" -> "Completed"
                    else -> "Manual"
                },
                options = listOf(
                    "manual" to "Manual",
                    "due" to "Due",
                    "defer" to "Defer",
                    "name" to "Name",
                    "priority" to "Priority",
                    "flagged" to "Flagged",
                    "estimate" to "Estimate",
                    "created" to "Created",
                    "completed_at" to "Completed",
                ),
            ) { sort = it ?: "manual" }

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
                                projects = projectIds,
                                tags = tagIds,
                                priorities = priorityIds,
                                dues = dueIds,
                                defers = deferIds,
                                inboxOnly = inboxOnly,
                                flaggedOnly = flaggedOnly,
                                status = status,
                                tagChildren = tagChildren,
                                estimateMin = estimateMin.toIntOrNull(),
                                estimateMax = estimateMax.toIntOrNull(),
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PerspectiveDropdown(
    label: String,
    value: String,
    options: List<Pair<String?, String>>,
    onSelect: (String?) -> Unit,
) {
    val colors = flow
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
        modifier = Modifier.padding(top = 12.dp),
    ) {
        OutlinedTextField(
            value = value,
            onValueChange = {},
            readOnly = true,
            singleLine = true,
            label = { Text(label) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            shape = RoundedCornerShape(10.dp),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = colors.hairline,
                unfocusedBorderColor = colors.hairline,
                focusedContainerColor = colors.background,
                unfocusedContainerColor = colors.background,
                focusedTextColor = colors.ink,
                unfocusedTextColor = colors.ink,
                focusedLabelColor = colors.faint,
                unfocusedLabelColor = colors.faint,
            ),
            modifier = Modifier
                .fillMaxWidth()
                .menuAnchor(),
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
            modifier = Modifier.heightIn(max = 320.dp),
            shape = RoundedCornerShape(12.dp),
            containerColor = colors.raised,
        ) {
            options.forEach { (key, text) ->
                DropdownMenuItem(
                    text = { Text(text, color = colors.ink) },
                    onClick = {
                        onSelect(key)
                        expanded = false
                    },
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PerspectiveMultiSelect(
    label: String,
    selected: Set<String>,
    anyLabel: String,
    options: List<Pair<String, String>>,
    onToggle: (String) -> Unit,
    onClear: () -> Unit,
) {
    val colors = flow
    var expanded by remember { mutableStateOf(false) }
    val chosen = options.filter { it.first in selected }.map { it.second }
    val summary = when {
        chosen.isEmpty() -> anyLabel
        chosen.size <= 2 -> chosen.joinToString(", ")
        else -> "${chosen.size} selected"
    }
    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
        modifier = Modifier.padding(top = 12.dp),
    ) {
        OutlinedTextField(
            value = summary,
            onValueChange = {},
            readOnly = true,
            singleLine = true,
            label = { Text(label) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            shape = RoundedCornerShape(10.dp),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = colors.hairline,
                unfocusedBorderColor = colors.hairline,
                focusedContainerColor = colors.background,
                unfocusedContainerColor = colors.background,
                focusedTextColor = colors.ink,
                unfocusedTextColor = colors.ink,
                focusedLabelColor = colors.faint,
                unfocusedLabelColor = colors.faint,
            ),
            modifier = Modifier
                .fillMaxWidth()
                .menuAnchor(),
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
            modifier = Modifier.heightIn(max = 360.dp),
            shape = RoundedCornerShape(12.dp),
            containerColor = colors.raised,
        ) {
            DropdownMenuItem(
                text = { Text(anyLabel, color = colors.ink) },
                leadingIcon = {
                    Checkbox(
                        checked = selected.isEmpty(),
                        onCheckedChange = null,
                        modifier = Modifier.size(20.dp),
                        colors = CheckboxDefaults.colors(
                            checkedColor = colors.accent,
                            uncheckedColor = colors.hairline,
                        ),
                    )
                },
                onClick = onClear,
            )
            options.forEach { (key, text) ->
                val active = key in selected
                DropdownMenuItem(
                    text = { Text(text, color = if (active) colors.accent else colors.ink) },
                    leadingIcon = {
                        Checkbox(
                            checked = active,
                            onCheckedChange = null,
                            modifier = Modifier.size(20.dp),
                            colors = CheckboxDefaults.colors(
                                checkedColor = colors.accent,
                                uncheckedColor = colors.hairline,
                            ),
                        )
                    },
                    onClick = { onToggle(key) },
                )
            }
        }
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

/// Number of tasks a saved perspective matches right now. Subtasks count too,
/// matching the Mac sidebar badge, which counts every row the query returns.
fun countPerspectiveTasks(db: Db?, perspective: PerspectiveDto): Int {
    db ?: return 0
    return db.taskViews(includeCompleted = perspective.showCompleted)
        .count { task -> matchesFilter(db, task, perspective.filter) }
}

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
    val all = remember(snapshot, perspective) {
        db?.taskViews(includeCompleted = perspective.showCompleted).orEmpty()
    }
    val children = remember(snapshot) {
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
            taskSections(tasks, perspective.sort).map { it.label to it.tasks }
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
            "available" -> !task.completed && task.blockedByTag == null
            "blocked" -> !task.completed && task.blockedByTag != null
            "deferred" -> task.defer?.local()?.toLocalDate()?.isAfter(today) == true
            else -> true
        }
        "due" -> windowMatches(due, condition.text("window"), today)
        "deferred" -> windowMatches(task.defer?.local()?.toLocalDate(), condition.text("window"), today)
        "has_repeat" -> {
            val expected = condition.boolean("value") ?: true
            (db?.tasks?.get(task.id)?.repeat != null) == expected
        }
        "fullscreen_reminder" -> {
            val expected = condition.boolean("value") ?: true
            val data = db ?: return false
            val dto = data.tasks[task.id] ?: return false
            // Match the Mac rule: a mark only counts when it has a due or
            // explicit reminder time to fire.
            val effective = data.wantsFullscreenReminder(dto) &&
                data.reminderStamp(dto) != null
            effective == expected
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
        "later" -> date == null || date.isAfter(today)
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

fun sortTasks(tasks: List<TaskView>, sort: String): List<TaskView> {
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
        "defer" -> compareBy<TaskView> { it.defer?.local()?.toLocalDate() ?: LocalDate.MAX }
            .thenBy { it.title.lowercase() }
        "estimate" -> compareBy<TaskView> { it.estimateMin ?: Int.MAX_VALUE }
            .thenBy { it.title.lowercase() }
        "created" -> compareBy<TaskView> { it.createdAt }.thenBy { it.title.lowercase() }
        "completed_at" -> compareByDescending<TaskView> { it.completedAt.orEmpty() }
            .thenBy { it.title.lowercase() }
        else -> return tasks
    }
    return tasks.sortedWith(comparator)
}

private fun groupLabel(db: Db?, task: TaskView, group: String): String {
    val due = task.due?.local()?.toLocalDate()
    val defer = task.defer?.local()?.toLocalDate()
    val today = LocalDate.now()
    return when (group) {
        "project" -> task.projectName ?: "No project"
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
        "defer_date" -> when {
            defer == null -> "No defer date"
            defer.isBefore(today) -> "Earlier"
            defer == today -> "Today"
            defer == today.plusDays(1) -> "Tomorrow"
            else -> defer.format(DateTimeFormatter.ofPattern("EEEE, d MMMM", Locale.getDefault()))
        }
        "flagged" -> if (task.flagged) "Flagged" else "Not flagged"
        "estimate" -> {
            val minutes = task.estimateMin
            when {
                minutes == null -> "No estimate"
                minutes < 15 -> "Under 15m"
                minutes <= 30 -> "15–30m"
                minutes <= 60 -> "30m–1h"
                minutes <= 120 -> "1–2h"
                else -> "Over 2h"
            }
        }
        else -> "Tasks"
    }
}

fun sortOptionLabel(sort: String): String = when (sort) {
    "due" -> "Due"
    "defer" -> "Defer"
    "name" -> "Name"
    "priority" -> "Priority"
    "flagged" -> "Flagged"
    "estimate" -> "Estimate"
    "created" -> "Created"
    "completed_at" -> "Completed"
    else -> "Manual"
}

val sortOptions: List<String> = listOf(
    "manual", "due", "defer", "name", "priority", "flagged",
    "estimate", "created", "completed_at",
)

data class TaskSection(val label: String?, val tasks: List<TaskView>)

/// Date sections used when a task list is sorted by a date. Other sorts stay
/// as one flat run so their own order is not broken up by headings.
fun taskSections(tasks: List<TaskView>, sort: String): List<TaskSection> {
    if (sort == "priority") {
        val groups = LinkedHashMap<String, MutableList<TaskView>>()
        tasks.forEach { task ->
            val label = when (task.priority) {
                "high" -> "High priority"
                "medium" -> "Medium priority"
                "low" -> "Low priority"
                else -> "No priority"
            }
            groups.getOrPut(label) { mutableListOf() }.add(task)
        }
        return groups.map { TaskSection(it.key, it.value) }
    }
    if (sort !in setOf("due", "defer", "created", "completed_at")) {
        return listOf(TaskSection(null, tasks))
    }
    val today = LocalDate.now()
    val groups = LinkedHashMap<String, MutableList<TaskView>>()
    tasks.forEach { task ->
        val date = when (sort) {
            "due" -> task.due?.local()?.toLocalDate()
            "defer" -> task.defer?.local()?.toLocalDate()
            "created" -> isoDate(task.createdAt)
            "completed_at" -> isoDate(task.completedAt)
            else -> null
        }
        val label = when {
            date == null -> when (sort) {
                "due" -> "No due date"
                "defer" -> "No defer date"
                "completed_at" -> "No completion date"
                else -> "No date"
            }
            sort == "due" && date.isBefore(today) -> "Overdue"
            date == today -> "Today"
            date == today.minusDays(1) -> "Yesterday"
            date == today.plusDays(1) -> "Tomorrow"
            else -> date.format(DateTimeFormatter.ofPattern("EEE, d MMM", Locale.getDefault()))
        }
        groups.getOrPut(label) { mutableListOf() }.add(task)
    }
    return groups.map { TaskSection(it.key, it.value) }
}

private fun isoDate(value: String?): LocalDate? =
    value?.let {
        runCatching { Instant.parse(it).atZone(ZoneId.systemDefault()).toLocalDate() }.getOrNull()
    }
