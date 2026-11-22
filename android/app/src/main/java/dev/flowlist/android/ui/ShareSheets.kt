package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Checklist
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.label

/** Asks whether a shared link or file starts a new task or joins an existing one. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShareChoiceSheet(
    label: String,
    onNewTask: () -> Unit,
    onExisting: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(horizontal = 20.dp)
                .padding(bottom = 26.dp),
        ) {
            Text(
                text = "Share to Flowlist",
                style = MaterialTheme.typography.titleLarge,
                color = colors.ink,
            )
            Spacer(modifier = Modifier.height(6.dp))
            Text(
                text = label,
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Spacer(modifier = Modifier.height(18.dp))
            ShareOption(
                icon = Icons.Rounded.Add,
                title = "New task",
                subtitle = "Name it, then attach the link or file",
                onClick = onNewTask,
            )
            Spacer(modifier = Modifier.height(10.dp))
            ShareOption(
                icon = Icons.Rounded.Checklist,
                title = "Add to existing task",
                subtitle = "Find a task and attach it there",
                onClick = onExisting,
            )
        }
    }
}

@Composable
private fun ShareOption(
    icon: ImageVector,
    title: String,
    subtitle: String,
    onClick: () -> Unit,
) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .background(colors.field, RoundedCornerShape(14.dp))
            .padding(horizontal = 14.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .size(38.dp)
                .background(colors.accent.copy(alpha = 0.12f), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(icon, contentDescription = null, tint = colors.accent, modifier = Modifier.size(20.dp))
        }
        Spacer(modifier = Modifier.width(14.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleSmall, color = colors.ink)
            Text(subtitle, style = MaterialTheme.typography.labelMedium, color = colors.faint)
        }
        Icon(
            Icons.Rounded.ChevronRight,
            contentDescription = null,
            tint = colors.faint,
            modifier = Modifier.size(18.dp),
        )
    }
}

/** Search and tap the task that should receive the shared link or file. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShareTaskPickerSheet(
    label: String,
    tasks: List<TaskView>,
    onPick: (TaskView) -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var query by remember { mutableStateOf("") }
    val needle = query.trim().lowercase()
    val filtered = remember(tasks, needle) {
        if (needle.isEmpty()) {
            tasks
        } else {
            tasks.filter { task ->
                task.title.lowercase().contains(needle) ||
                    task.projectName?.lowercase()?.contains(needle) == true ||
                    task.tags.any { it.lowercase().contains(needle) }
            }
        }
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
                .navigationBarsPadding()
                .padding(horizontal = 20.dp)
                .padding(bottom = 16.dp),
        ) {
            Text(
                text = "Add to task",
                style = MaterialTheme.typography.titleLarge,
                color = colors.ink,
            )
            Spacer(modifier = Modifier.height(6.dp))
            Text(
                text = label,
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Spacer(modifier = Modifier.height(14.dp))
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                placeholder = { Text("Search tasks", color = colors.faint) },
                singleLine = true,
                shape = RoundedCornerShape(12.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = colors.accent,
                    unfocusedBorderColor = colors.hairline,
                    cursorColor = colors.accent,
                    focusedContainerColor = colors.background,
                    unfocusedContainerColor = colors.background,
                ),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(modifier = Modifier.height(8.dp))
            LazyColumn(modifier = Modifier.heightIn(max = 420.dp)) {
                items(filtered, key = { it.id }) { task ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onPick(task) }
                            .padding(vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = task.title,
                                style = MaterialTheme.typography.bodyMedium,
                                color = colors.ink,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            val details = listOfNotNull(task.projectName, task.due?.label())
                                .joinToString(" · ")
                            if (details.isNotEmpty()) {
                                Text(
                                    text = details,
                                    style = MaterialTheme.typography.labelMedium,
                                    color = colors.faint,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                        Icon(
                            Icons.Rounded.ChevronRight,
                            contentDescription = null,
                            tint = colors.faint,
                            modifier = Modifier.size(18.dp),
                        )
                    }
                    Hairline(Modifier.padding(start = 0.dp))
                }
                if (filtered.isEmpty()) {
                    item(key = "share-empty") {
                        Text(
                            text = "No tasks match.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = colors.faint,
                            modifier = Modifier.padding(16.dp),
                        )
                    }
                }
            }
        }
    }
}
