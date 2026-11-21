package dev.flowlist.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Archive
import androidx.compose.material.icons.outlined.AttachFile
import androidx.compose.material.icons.outlined.CheckCircle
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Flag
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.RateReview
import androidx.compose.material.icons.outlined.Tag
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.archivedProjects
import dev.flowlist.android.data.attachmentTasks
import dev.flowlist.android.data.changedTasks
import dev.flowlist.android.data.completedTasks
import dev.flowlist.android.data.priorityGroups
import dev.flowlist.android.data.reviewProjects
import dev.flowlist.android.data.tasksForTag
import dev.flowlist.android.data.tagUsageCount
import dev.flowlist.android.data.trashedTasks

enum class MoreView(val label: String, val icon: ImageVector) {
    Review("Review", Icons.Outlined.RateReview),
    Completed("Completed", Icons.Outlined.CheckCircle),
    Archive("Archive", Icons.Outlined.Archive),
    Trash("Trash", Icons.Outlined.Delete),
    Tags("Tags", Icons.Outlined.Tag),
    Priorities("Priorities", Icons.Outlined.Flag),
    Changed("Changed", Icons.Outlined.History),
    Attachments("Attachments", Icons.Outlined.AttachFile),
}

@Composable
fun MoreScreen(db: Db?, onOpen: (MoreView) -> Unit) {
    val colors = flow
    val counts = remember(db) {
        if (db == null) emptyMap() else mapOf(
            MoreView.Review to db.reviewProjects().size,
            MoreView.Completed to db.completedTasks().size,
            MoreView.Archive to db.archivedProjects().size,
            MoreView.Trash to db.trashedTasks().size,
            MoreView.Tags to db.tags.size,
            MoreView.Priorities to db.priorityGroups().sumOf { it.second.size },
            MoreView.Changed to db.changedTasks().size,
            MoreView.Attachments to db.attachmentTasks().size,
        )
    }
    LazyColumn(
        contentPadding = PaddingValues(bottom = 110.dp),
    ) {
        items(MoreView.entries, key = { it.name }) { view ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { onOpen(view) }
                    .padding(horizontal = 20.dp, vertical = 13.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier = Modifier
                        .size(34.dp)
                        .clip(CircleShape)
                        .background(colors.field),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = view.icon,
                        contentDescription = null,
                        tint = colors.faint,
                        modifier = Modifier.size(18.dp),
                    )
                }
                Spacer(modifier = Modifier.width(14.dp))
                Text(
                    text = view.label,
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.ink,
                    modifier = Modifier.weight(1f),
                )
                val count = counts[view] ?: 0
                if (count > 0) {
                    Text(
                        text = "$count",
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.faint,
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                }
                Icon(
                    imageVector = Icons.Outlined.ChevronRight,
                    contentDescription = null,
                    tint = colors.faint,
                    modifier = Modifier.size(16.dp),
                )
            }
            Hairline(Modifier.padding(start = 68.dp))
        }
    }
}

@Composable
fun MoreDestination(
    view: MoreView,
    db: Db,
    pending: Set<String>,
    onTask: (TaskView) -> Unit,
    onToggle: (TaskView) -> Unit,
    onRestore: (String) -> Unit,
    onPurge: (String) -> Unit,
    onReview: (ProjectDto) -> Unit,
    onProject: (ProjectDto) -> Unit,
) {
    val colors = flow
    var tag by remember { mutableStateOf<TagDto?>(null) }
    var purgeTarget by remember { mutableStateOf<TaskView?>(null) }

    LaunchedEffect(view) {
        tag = null
        purgeTarget = null
    }
    BackHandler(enabled = tag != null) { tag = null }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(top = 4.dp, bottom = 20.dp),
    ) {
        if (tag != null) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "#${tag!!.name}",
                    style = MaterialTheme.typography.titleLarge,
                    color = colors.ink,
                    modifier = Modifier.weight(1f),
                )
                TextButton(onClick = { tag = null }) {
                    Text("All tags", color = colors.accent)
                }
            }
        }
        when {
            tag != null -> TaskList(db.tasksForTag(tag!!.id), pending, onToggle, onTask)
            view == MoreView.Review -> LazyColumn {
                if (db.reviewProjects().isEmpty()) {
                    item(key = "review-empty") {
                        EmptyLine("No projects are due for review.")
                    }
                }
                items(db.reviewProjects(), key = { it.id }) { project ->
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(project.name, style = MaterialTheme.typography.bodyMedium, color = colors.ink)
                            Text(
                                text = project.nextReview?.let { "Review due $it" } ?: "Review due",
                                style = MaterialTheme.typography.labelMedium,
                                color = colors.warning,
                            )
                        }
                        TextButton(onClick = { onReview(project) }) {
                            Text("Reviewed", color = colors.accent)
                        }
                    }
                    Hairline(Modifier.padding(horizontal = 20.dp))
                }
            }
            view == MoreView.Completed -> TaskList(db.completedTasks(), pending, onToggle, onTask)
            view == MoreView.Archive -> LazyColumn {
                if (db.archivedProjects().isEmpty()) {
                    item(key = "archive-empty") {
                        EmptyLine("No archived projects yet.")
                    }
                }
                items(db.archivedProjects(), key = { it.id }) { project ->
                    val count = db.tasks.values.count { it.project == project.id && !it.completed }
                    Row(
                        modifier = Modifier.fillMaxWidth().clickable { onProject(project) }
                            .padding(horizontal = 20.dp, vertical = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            project.name,
                            style = MaterialTheme.typography.bodyMedium,
                            color = colors.ink,
                            modifier = Modifier.weight(1f),
                        )
                        Text("$count", style = MaterialTheme.typography.labelMedium, color = colors.faint)
                    }
                    Hairline(Modifier.padding(start = 20.dp))
                }
            }
            view == MoreView.Trash -> LazyColumn {
                if (db.trashedTasks().isEmpty()) {
                    item(key = "trash-empty") {
                        EmptyLine("Trash is empty.")
                    }
                }
                items(db.trashedTasks(), key = { it.id }) { task ->
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(task.title, style = MaterialTheme.typography.bodyMedium, color = colors.ink)
                            Text(
                                text = listOfNotNull(task.projectName, "In Trash").joinToString(" · "),
                                style = MaterialTheme.typography.labelMedium,
                                color = colors.faint,
                            )
                        }
                        TextButton(onClick = { onRestore(task.id) }) {
                            Text("Restore", color = colors.accent)
                        }
                        TextButton(onClick = { purgeTarget = task }) {
                            Text("Delete", color = colors.danger)
                        }
                    }
                    Hairline(Modifier.padding(horizontal = 20.dp))
                }
            }
            view == MoreView.Tags -> LazyColumn {
                if (db.tags.isEmpty()) {
                    item(key = "tags-empty") {
                        EmptyLine("No tags yet.")
                    }
                }
                items(
                    db.tags.values.sortedWith(
                        compareByDescending<TagDto> { tagValue ->
                            db.tagUsageCount(tagValue.id)
                        }.thenBy { it.name.lowercase() },
                    ),
                    key = { it.id },
                ) { item ->
                    val count = db.tagUsageCount(item.id)
                    Row(
                        modifier = Modifier.fillMaxWidth().clickable { tag = item }
                            .padding(horizontal = 20.dp, vertical = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            "#${item.name}",
                            style = MaterialTheme.typography.bodyMedium,
                            color = colors.ink,
                            modifier = Modifier.weight(1f),
                        )
                        Text("$count", style = MaterialTheme.typography.labelMedium, color = colors.faint)
                    }
                    Hairline(Modifier.padding(start = 20.dp))
                }
            }
            view == MoreView.Priorities -> LazyColumn {
                db.priorityGroups().forEach { (label, tasks) ->
                    item(key = "prio-$label") {
                        Text(
                            text = label.uppercase(),
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.SemiBold,
                            color = colors.faint,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 10.dp),
                        )
                    }
                    items(tasks, key = { it.id }) { task ->
                        TaskRow(
                            task = task.copy(pending = task.id in pending),
                            onToggle = { onToggle(task) },
                            onClick = { onTask(task) },
                        )
                        Hairline(Modifier.padding(start = 58.dp))
                    }
                }
            }
            view == MoreView.Changed -> TaskList(db.changedTasks(), pending, onToggle, onTask)
            view == MoreView.Attachments -> LazyColumn {
                if (db.attachmentTasks().isEmpty()) {
                    item(key = "attachments-empty") {
                        EmptyLine("No attachments yet.")
                    }
                }
                items(db.attachmentTasks(), key = { it.id }) { task ->
                    Column(
                        modifier = Modifier.clickable { onTask(task) }
                            .padding(horizontal = 20.dp, vertical = 10.dp),
                    ) {
                        Text(task.title, style = MaterialTheme.typography.bodyMedium, color = colors.ink)
                        Text(
                            text = task.attachments.joinToString(" · ") { it.substringAfterLast('/') },
                            style = MaterialTheme.typography.labelMedium,
                            color = colors.faint,
                        )
                    }
                    Hairline(Modifier.padding(start = 20.dp))
                }
            }
        }
    }

    if (purgeTarget != null) {
        AlertDialog(
            onDismissRequest = { purgeTarget = null },
            title = { Text("Delete forever?") },
            text = { Text("“${purgeTarget!!.title}” cannot be recovered.") },
            confirmButton = {
                TextButton(onClick = { onPurge(purgeTarget!!.id); purgeTarget = null }) {
                    Text("Delete", color = colors.danger)
                }
            },
            dismissButton = { TextButton(onClick = { purgeTarget = null }) { Text("Cancel") } },
            containerColor = colors.background,
        )
    }
}

@Composable
private fun EmptyLine(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyMedium,
        color = flow.faint,
        modifier = Modifier.padding(horizontal = 20.dp, vertical = 24.dp),
    )
}

@Composable
private fun TaskList(
    tasks: List<TaskView>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onTask: (TaskView) -> Unit,
) {
    LazyColumn(contentPadding = PaddingValues(bottom = 20.dp)) {
        items(tasks, key = { it.id }) { task ->
            TaskRow(
                task = task.copy(pending = task.id in pending),
                onToggle = { onToggle(task) },
                onClick = { onTask(task) },
            )
            Hairline(Modifier.padding(start = 58.dp))
        }
        if (tasks.isEmpty()) {
            item(key = "empty") {
                Text(
                    text = "Nothing here.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = flow.faint,
                    modifier = Modifier.padding(20.dp),
                )
            }
        }
    }
}
