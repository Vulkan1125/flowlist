package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.ExpandMore
import androidx.compose.material3.AlertDialog
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
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.StampDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskDto
import dev.flowlist.android.data.TaskEdit
import dev.flowlist.android.data.isOverdue
import dev.flowlist.android.data.isToday
import dev.flowlist.android.data.label

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskEditSheet(
    task: TaskDto,
    projects: List<ProjectDto>,
    tags: List<TagDto>,
    pending: Boolean,
    onDismiss: () -> Unit,
    onSave: (TaskEdit) -> Unit,
    onComplete: () -> Unit,
    onDelete: () -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var title by remember(task.id) { mutableStateOf(task.title) }
    var notes by remember(task.id) { mutableStateOf(task.notes) }
    var project by remember(task.id) { mutableStateOf(task.project) }
    var due by remember(task.id) { mutableStateOf(task.due) }
    var defer by remember(task.id) { mutableStateOf(task.defer) }
    var priority by remember(task.id) { mutableStateOf(task.priority) }
    var flagged by remember(task.id) { mutableStateOf(task.flagged) }
    var estimate by remember(task.id) {
        mutableStateOf(task.estimateMin?.toString().orEmpty())
    }
    var selectedTags by remember(task.id) { mutableStateOf(task.tags.toSet()) }
    var projectMenu by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }
    var datePickerForDue by remember { mutableStateOf(true) }
    var showDueSheet by remember { mutableStateOf(false) }

    fun applyStamp(value: StampDto?) {
        if (datePickerForDue) due = value else defer = value
    }

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
                CheckCircle(checked = task.completed, onClick = onComplete)
                Spacer(modifier = Modifier.width(14.dp))
                Text(
                    text = if (pending) "Waiting to sync" else "Task",
                    style = MaterialTheme.typography.labelMedium,
                    color = if (pending) colors.warning else colors.faint,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Rounded.Close, contentDescription = "Close", tint = colors.faint)
                }
            }

            OutlinedTextField(
                value = title,
                onValueChange = { title = it },
                placeholder = { Text("Task name", color = colors.faint) },
                textStyle = TextStyle(fontSize = 21.sp, fontWeight = FontWeight.SemiBold, color = colors.ink),
                colors = borderlessColors(),
                modifier = Modifier.fillMaxWidth(),
            )

            OutlinedTextField(
                value = notes,
                onValueChange = { notes = it },
                placeholder = { Text("Add a note", color = colors.faint) },
                textStyle = TextStyle(fontSize = 15.sp, lineHeight = 21.sp, color = colors.ink),
                colors = borderlessColors(),
                minLines = 2,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 2.dp),
            )

            Spacer(modifier = Modifier.height(10.dp))
            EditRow(
                label = "Due",
                value = due?.label() ?: "No date",
                valueColor = when {
                    due?.isOverdue() == true -> colors.danger
                    due?.isToday() == true -> colors.success
                    due != null -> colors.ink
                    else -> colors.faint
                },
                clearable = due != null,
                onClear = { due = null },
                onClick = {
                    datePickerForDue = true
                    showDueSheet = true
                },
            )
            EditRow(
                label = "Defer",
                value = defer?.label() ?: "No date",
                valueColor = if (defer != null) colors.ink else colors.faint,
                clearable = defer != null,
                onClear = { defer = null },
                onClick = {
                    datePickerForDue = false
                    showDueSheet = true
                },
            )

            Box {
                EditRow(
                    label = "Project",
                    value = projects.firstOrNull { it.id == project }?.name ?: "Inbox",
                    valueColor = colors.ink,
                    clearable = false,
                    onClick = { projectMenu = true },
                    trailing = {
                        Icon(
                            Icons.Rounded.ExpandMore,
                            contentDescription = null,
                            tint = colors.faint,
                            modifier = Modifier.size(18.dp),
                        )
                    },
                )
                DropdownMenu(
                    expanded = projectMenu,
                    onDismissRequest = { projectMenu = false },
                    containerColor = colors.raised,
                ) {
                    DropdownMenuItem(
                        text = { Text("Inbox", color = colors.ink) },
                        onClick = {
                            project = null
                            projectMenu = false
                        },
                    )
                    projects.sortedBy { it.name.lowercase() }.forEach { item ->
                        DropdownMenuItem(
                            text = { Text(item.name, color = colors.ink) },
                            onClick = {
                                project = item.id
                                projectMenu = false
                            },
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(14.dp))
            Label("Priority")
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                PriorityPill("None", priority == null, colors.ink) { priority = null }
                PriorityPill("P1", priority == "high", colors.danger) { priority = "high" }
                PriorityPill("P2", priority == "medium", colors.warning) { priority = "medium" }
                PriorityPill("P3", priority == "low", colors.accent) { priority = "low" }
            }

            Spacer(modifier = Modifier.height(16.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Label("Flag", modifier = Modifier.weight(1f))
                Switch(
                    checked = flagged,
                    onCheckedChange = { flagged = it },
                    colors = SwitchDefaults.colors(
                        checkedThumbColor = colors.onAccent,
                        checkedTrackColor = colors.accent,
                        uncheckedThumbColor = colors.faint,
                        uncheckedTrackColor = colors.field,
                        uncheckedBorderColor = colors.hairline,
                    ),
                )
            }

            Spacer(modifier = Modifier.height(12.dp))
            Label("Estimate")
            OutlinedTextField(
                value = estimate,
                onValueChange = { value -> estimate = value.filter(Char::isDigit).take(4) },
                placeholder = { Text("Minutes", color = colors.faint) },
                suffix = { if (estimate.isNotEmpty()) Text("min", color = colors.faint) },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                singleLine = true,
                shape = RoundedCornerShape(10.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = colors.accent,
                    unfocusedBorderColor = colors.hairline,
                    focusedContainerColor = colors.background,
                    unfocusedContainerColor = colors.background,
                    focusedTextColor = colors.ink,
                    unfocusedTextColor = colors.ink,
                    cursorColor = colors.accent,
                ),
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 6.dp),
            )

            if (tags.isNotEmpty()) {
                Spacer(modifier = Modifier.height(16.dp))
                Label("Tags")
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    tags.sortedBy { it.name.lowercase() }.forEach { tag ->
                        val active = tag.id in selectedTags
                        Box(
                            modifier = Modifier
                                .clip(RoundedCornerShape(9.dp))
                                .background(if (active) colors.field else colors.background)
                                .clickable {
                                    selectedTags = if (active) {
                                        selectedTags - tag.id
                                    } else {
                                        selectedTags + tag.id
                                    }
                                }
                                .padding(horizontal = 11.dp, vertical = 6.dp),
                        ) {
                            Text(
                                text = "#${tag.name}",
                                style = MaterialTheme.typography.labelMedium,
                                color = if (active) colors.accent else colors.faint,
                            )
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(26.dp))
            Button(
                onClick = {
                    onSave(
                        TaskEdit(
                            title = title.trim(),
                            notes = notes,
                            project = project,
                            due = due,
                            defer = defer,
                            priority = priority,
                            flagged = flagged,
                            estimateMin = estimate.toIntOrNull(),
                            tags = selectedTags,
                        ),
                    )
                },
                enabled = title.isNotBlank(),
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = colors.accent,
                    contentColor = colors.onAccent,
                    disabledContainerColor = colors.field,
                    disabledContentColor = colors.faint,
                ),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(50.dp),
            ) {
                Text("Save")
            }
            Spacer(modifier = Modifier.height(4.dp))
            TextButton(
                onClick = { confirmDelete = true },
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Move to Trash", color = colors.danger)
            }
        }
    }

    if (showDueSheet) {
        DueDateSheet(
            title = if (datePickerForDue) "Due date" else "Defer date",
            initial = if (datePickerForDue) due else defer,
            onDismiss = { showDueSheet = false },
            onSet = { value ->
                applyStamp(value)
                showDueSheet = false
            },
        )
    }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text("Move to Trash?") },
            text = { Text("“${task.title}” goes to Trash on your Mac and can be restored there.") },
            confirmButton = {
                TextButton(onClick = { confirmDelete = false; onDelete() }) {
                    Text("Move to Trash", color = colors.danger)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmDelete = false }) { Text("Cancel") }
            },
            containerColor = colors.background,
        )
    }
}

@Composable
private fun Label(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text.uppercase(),
        style = MaterialTheme.typography.labelSmall,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.7.sp,
        color = flow.faint,
        modifier = modifier.padding(bottom = 6.dp),
    )
}

@Composable
private fun EditRow(
    label: String,
    value: String,
    valueColor: androidx.compose.ui.graphics.Color,
    clearable: Boolean,
    onClick: () -> Unit,
    onClear: (() -> Unit)? = null,
    trailing: @Composable (() -> Unit)? = null,
) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(vertical = 11.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = colors.faint,
            modifier = Modifier.width(74.dp),
        )
        Text(
            text = value,
            style = MaterialTheme.typography.labelLarge,
            color = valueColor,
            modifier = Modifier.weight(1f),
            maxLines = 1,
        )
        if (onClear != null && clearable) {
            IconButton(onClick = onClear, modifier = Modifier.size(28.dp)) {
                Icon(
                    Icons.Rounded.Close,
                    contentDescription = "Clear",
                    tint = colors.faint,
                    modifier = Modifier.size(16.dp),
                )
            }
        }
        trailing?.invoke()
    }
    Hairline()
}

@Composable
private fun PriorityPill(
    text: String,
    selected: Boolean,
    tone: androidx.compose.ui.graphics.Color,
    onClick: () -> Unit,
) {
    val colors = flow
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(9.dp))
            .background(if (selected) tone.copy(alpha = 0.14f) else colors.background)
            .clickable(onClick = onClick)
            .padding(horizontal = 14.dp, vertical = 7.dp),
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelLarge,
            color = if (selected) tone else colors.faint,
        )
    }
}

@Composable
private fun borderlessColors() = OutlinedTextFieldDefaults.colors(
    focusedBorderColor = androidx.compose.ui.graphics.Color.Transparent,
    unfocusedBorderColor = androidx.compose.ui.graphics.Color.Transparent,
    focusedContainerColor = flow.background,
    unfocusedContainerColor = flow.background,
    cursorColor = flow.accent,
    focusedTextColor = flow.ink,
    unfocusedTextColor = flow.ink,
)
