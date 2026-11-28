package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.ExpandMore
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.BatchDate
import dev.flowlist.android.data.BatchEdit
import dev.flowlist.android.data.FolderDto
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.StampDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.label

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun BatchEditSheet(
    count: Int,
    projects: List<ProjectDto>,
    folders: List<FolderDto>,
    tags: List<TagDto>,
    weekStartsMonday: Boolean,
    defaultTimeMinutes: Int,
    onDismiss: () -> Unit,
    onApply: (BatchEdit) -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var projectChoice by remember { mutableStateOf("keep") }
    var addTags by remember { mutableStateOf<Set<String>>(emptySet()) }
    var removeTags by remember { mutableStateOf<Set<String>>(emptySet()) }
    var due by remember { mutableStateOf<BatchDate>(BatchDate.Keep) }
    var defer by remember { mutableStateOf<BatchDate>(BatchDate.Keep) }
    var planned by remember { mutableStateOf<BatchDate>(BatchDate.Keep) }
    var priorityChoice by remember { mutableStateOf("keep") }
    var flagChoice by remember { mutableStateOf("keep") }
    var estimateMode by remember { mutableStateOf("keep") }
    var estimateText by remember { mutableStateOf("") }
    var dateTarget by remember { mutableStateOf<String?>(null) }
    var projectMenu by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val folderNames = folders.associate { it.id to it.name }
    fun projectLabel(project: ProjectDto): String =
        project.folder?.let { folderNames[it] }?.let { "$it / ${project.name}" }
            ?: project.name

    fun dateValue(value: BatchDate): String = when (value) {
        BatchDate.Keep -> "Leave unchanged"
        BatchDate.Clear -> "No date"
        is BatchDate.Set -> value.stamp.label()
    }

    fun reset() {
        projectChoice = "keep"
        addTags = emptySet()
        removeTags = emptySet()
        due = BatchDate.Keep
        defer = BatchDate.Keep
        planned = BatchDate.Keep
        priorityChoice = "keep"
        flagChoice = "keep"
        estimateMode = "keep"
        estimateText = ""
        error = null
    }

    fun toggleAdd(id: String) {
        val next = if (id in addTags) addTags - id else addTags + id
        addTags = next
        if (id in next) removeTags = removeTags - id
    }

    fun toggleRemove(id: String) {
        val next = if (id in removeTags) removeTags - id else removeTags + id
        removeTags = next
        if (id in next) addTags = addTags - id
    }

    val hasChanges = projectChoice != "keep" ||
        addTags.isNotEmpty() ||
        removeTags.isNotEmpty() ||
        due != BatchDate.Keep ||
        defer != BatchDate.Keep ||
        planned != BatchDate.Keep ||
        priorityChoice != "keep" ||
        flagChoice != "keep" ||
        estimateMode != "keep"

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
                .padding(bottom = 28.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Edit ${if (count == 1) "1 task" else "$count tasks"}",
                        style = MaterialTheme.typography.titleLarge,
                        color = colors.ink,
                    )
                    Text(
                        text = "Only filled fields are applied.",
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.faint,
                    )
                }
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Rounded.Close, "Close", tint = colors.faint)
                }
            }

            BatchLabel("Project")
            Box {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(10.dp))
                        .border(1.dp, colors.hairline, RoundedCornerShape(10.dp))
                        .clickable { projectMenu = true }
                        .padding(horizontal = 12.dp, vertical = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = when {
                            projectChoice == "keep" -> "Leave unchanged"
                            projectChoice == "inbox" -> "Inbox"
                            else -> projects.firstOrNull { it.id == projectChoice }
                                ?.let { projectLabel(it) }
                                ?: "Project"
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (projectChoice == "keep") colors.faint else colors.ink,
                        modifier = Modifier.weight(1f),
                    )
                    Icon(
                        imageVector = Icons.Rounded.ExpandMore,
                        contentDescription = null,
                        tint = colors.faint,
                        modifier = Modifier.size(20.dp),
                    )
                }
                DropdownMenu(
                    expanded = projectMenu,
                    onDismissRequest = { projectMenu = false },
                    containerColor = colors.raised,
                ) {
                    DropdownMenuItem(
                        text = { Text("Leave unchanged") },
                        onClick = { projectMenu = false; projectChoice = "keep" },
                    )
                    DropdownMenuItem(
                        text = { Text("Inbox") },
                        onClick = { projectMenu = false; projectChoice = "inbox" },
                    )
                    projects.sortedBy { it.name.lowercase() }.forEach { project ->
                        DropdownMenuItem(
                            text = { Text(projectLabel(project)) },
                            onClick = { projectMenu = false; projectChoice = project.id },
                        )
                    }
                }
            }

            if (tags.isNotEmpty()) {
                BatchLabel("Add tags")
                FlowRow(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    tags.sortedBy { it.name.lowercase() }.forEach { tag ->
                        TagPill("#${tag.name}", tag.id in addTags) { toggleAdd(tag.id) }
                    }
                }
                BatchLabel("Remove tags")
                FlowRow(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    tags.sortedBy { it.name.lowercase() }.forEach { tag ->
                        TagPill("#${tag.name}", tag.id in removeTags, danger = true) {
                            toggleRemove(tag.id)
                        }
                    }
                }
            }

            BatchLabel("Dates")
            BatchDateRow("Due", dateValue(due)) { dateTarget = "due" }
            BatchDateRow("Defer", dateValue(defer)) { dateTarget = "defer" }
            BatchDateRow("Planned", dateValue(planned)) { dateTarget = "planned" }

            BatchLabel("Priority")
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf(
                    "keep" to "Keep",
                    "none" to "None",
                    "high" to "H",
                    "medium" to "M",
                    "low" to "L",
                ).forEach { (value, label) ->
                    TagPill(label, priorityChoice == value) { priorityChoice = value }
                }
            }

            BatchLabel("Flag")
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf("keep" to "Keep", "flag" to "Flag", "unflag" to "Unflag").forEach {
                    (value, label) ->
                    TagPill(label, flagChoice == value) { flagChoice = value }
                }
            }

            BatchLabel("Estimate")
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf("keep" to "Keep", "set" to "Set", "clear" to "Clear").forEach {
                    (value, label) ->
                    TagPill(label, estimateMode == value) { estimateMode = value }
                }
            }
            if (estimateMode == "set") {
                OutlinedTextField(
                    value = estimateText,
                    onValueChange = { estimateText = it.filter { c -> c.isDigit() }.take(4) },
                    placeholder = { Text("Minutes", color = colors.faint) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    colors = perspectiveFieldColors(),
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                )
            }

            if (error != null) {
                Text(
                    text = error!!,
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.warning,
                    modifier = Modifier.padding(top = 10.dp),
                )
            }

            Spacer(modifier = Modifier.height(18.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                TextButton(onClick = { reset() }) {
                    Text("Clear", color = colors.faint)
                }
                Spacer(modifier = Modifier.weight(1f))
                Button(
                    onClick = {
                        val estimate = estimateText.toIntOrNull()
                        if (estimateMode == "set" && estimate == null) {
                            error = "Enter minutes or choose Clear."
                            return@Button
                        }
                        if (!hasChanges) {
                            error = "Nothing to apply."
                            return@Button
                        }
                        onApply(
                            BatchEdit(
                                projectId = if (projectChoice == "keep" || projectChoice == "inbox") {
                                    null
                                } else {
                                    projectChoice
                                },
                                clearProject = projectChoice == "inbox",
                                addTagIds = addTags,
                                removeTagIds = removeTags,
                                due = due,
                                defer = defer,
                                planned = planned,
                                priority = priorityChoice.takeIf {
                                    it == "high" || it == "medium" || it == "low"
                                },
                                clearPriority = priorityChoice == "none",
                                flagged = when (flagChoice) {
                                    "flag" -> true
                                    "unflag" -> false
                                    else -> null
                                },
                                estimate = if (estimateMode == "clear") null else estimate,
                                clearEstimate = estimateMode == "clear",
                            ),
                        )
                    },
                    enabled = hasChanges,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = colors.accent,
                        contentColor = colors.onAccent,
                        disabledContainerColor = colors.field,
                        disabledContentColor = colors.faint,
                    ),
                ) {
                    Text("Apply to $count")
                }
            }
        }
    }

    if (dateTarget != null) {
        val current = when (dateTarget) {
            "due" -> due
            "defer" -> defer
            else -> planned
        }
        DueDateSheet(
            title = when (dateTarget) {
                "due" -> "Due date"
                "defer" -> "Defer date"
                else -> "Planned date"
            },
            initial = (current as? BatchDate.Set)?.stamp,
            weekStartsMonday = weekStartsMonday,
            defaultTime = java.time.LocalTime.of(
                defaultTimeMinutes / 60,
                defaultTimeMinutes % 60,
            ),
            onDismiss = { dateTarget = null },
            onSet = { value: StampDto? ->
                val change = value?.let { BatchDate.Set(it) } ?: BatchDate.Clear
                when (dateTarget) {
                    "due" -> due = change
                    "defer" -> defer = change
                    else -> planned = change
                }
                dateTarget = null
            },
        )
    }
}

@Composable
private fun BatchLabel(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelSmall,
        fontWeight = FontWeight.SemiBold,
        color = flow.faint,
        modifier = Modifier.padding(top = 16.dp, bottom = 6.dp),
    )
}

@Composable
private fun BatchDateRow(label: String, value: String, onClick: () -> Unit) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .clickable(onClick = onClick)
            .padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = colors.ink,
            modifier = Modifier.width(92.dp),
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            color = if (value == "Leave unchanged") colors.faint else colors.ink,
            modifier = Modifier.weight(1f),
        )
        Icon(
            imageVector = Icons.Rounded.ExpandMore,
            contentDescription = null,
            tint = colors.faint,
            modifier = Modifier.size(18.dp),
        )
    }
}

@Composable
private fun TagPill(
    label: String,
    selected: Boolean,
    danger: Boolean = false,
    onClick: () -> Unit,
) {
    val colors = flow
    Text(
        text = label,
        style = MaterialTheme.typography.labelMedium,
        color = when {
            selected && danger -> colors.danger
            selected -> colors.accent
            else -> colors.faint
        },
        modifier = Modifier
            .clip(RoundedCornerShape(percent = 50))
            .background(if (selected) colors.field else colors.background)
            .border(
                1.dp,
                if (selected) {
                    if (danger) colors.danger else colors.accent
                } else {
                    colors.hairline
                },
                RoundedCornerShape(percent = 50),
            )
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 6.dp),
    )
}
