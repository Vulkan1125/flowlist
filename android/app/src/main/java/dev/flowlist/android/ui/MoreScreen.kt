package dev.flowlist.android.ui

import android.Manifest
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import dev.flowlist.android.notify.DeviceLocation
import kotlinx.coroutines.launch
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.material.icons.outlined.LocalCafe
import androidx.compose.material.icons.outlined.NearMe
import androidx.compose.material.icons.outlined.SignalCellularAlt
import androidx.compose.material.icons.outlined.Tag
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.MoreVert
import androidx.compose.material.icons.rounded.Pause
import androidx.compose.material.icons.rounded.Undo
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.GeoDto
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.archivedProjects
import dev.flowlist.android.data.attachmentTasks
import dev.flowlist.android.data.changedTasks
import dev.flowlist.android.data.geotaggedTaskCount
import dev.flowlist.android.data.taggedTaskCount
import dev.flowlist.android.data.completedTasks
import dev.flowlist.android.data.flaggedTasks
import dev.flowlist.android.data.priorityGroups
import dev.flowlist.android.data.reviewProjects
import dev.flowlist.android.data.tasksForTag
import dev.flowlist.android.data.tagUsageCount
import dev.flowlist.android.data.tagIsBlocked
import dev.flowlist.android.ui.sortTasks
import dev.flowlist.android.data.tagStatusLabel
import dev.flowlist.android.data.trashedTasks

enum class MoreView(val label: String, val icon: ImageVector) {
    Flagged("Flagged", Icons.Outlined.Flag),
    Review("Review", Icons.Outlined.LocalCafe),
    Completed("Completed", Icons.Outlined.CheckCircle),
    Archive("Archive", Icons.Outlined.Archive),
    Trash("Trash", Icons.Outlined.Delete),
    Tags("Tags", Icons.Outlined.Tag),
    Nearby("Nearby", Icons.Outlined.NearMe),
    Priorities("Priorities", Icons.Outlined.SignalCellularAlt),
    Changed("Changed", Icons.Outlined.History),
    Attachments("Attachments", Icons.Outlined.AttachFile),
}

@Composable
fun MoreScreen(db: Db?, onOpen: (MoreView) -> Unit) {
    val colors = flow
    val counts = remember(db) {
        if (db == null) emptyMap() else mapOf(
            MoreView.Flagged to db.flaggedTasks().size,
            MoreView.Review to db.reviewProjects().size,
            MoreView.Completed to db.completedTasks().size,
            MoreView.Archive to db.archivedProjects().size,
            MoreView.Trash to db.trashedTasks().size,
            MoreView.Tags to db.taggedTaskCount(),
            MoreView.Nearby to db.geotaggedTaskCount(),
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TrashRow(
    task: TaskView,
    selected: Boolean,
    swipe: String,
    onSelect: () -> Unit,
    onRestore: () -> Unit,
    onDelete: () -> Unit,
) {
    val colors = flow
    val selection = LocalTaskSelection.current
    val haptics = LocalHapticFeedback.current
    val hapticsEnabled = LocalHapticsEnabled.current
    fun tick() {
        if (hapticsEnabled) haptics.performHapticFeedback(HapticFeedbackType.LongPress)
    }
    // Material may confirm the same swipe twice while it settles; run the
    // action once and replay the first answer for the duplicate.
    var lastActionAt by remember { mutableStateOf(0L) }
    var lastResult by remember { mutableStateOf(false) }
    val restoreRight = swipe != "restore_left"
    val swipeEnabled = selection == null && swipe != "off"
    val currentRestoreRight by rememberUpdatedState(restoreRight)
    val currentSwipeEnabled by rememberUpdatedState(swipeEnabled)
    val currentRestore by rememberUpdatedState(onRestore)
    val currentDelete by rememberUpdatedState(onDelete)
    val state = rememberSwipeToDismissBoxState(
        confirmValueChange = { value ->
            if (!currentSwipeEnabled || value == SwipeToDismissBoxValue.Settled) {
                false
            } else {
                val now = System.currentTimeMillis()
                if (now - lastActionAt < 600) {
                    lastResult
                } else {
                    lastActionAt = now
                    val result = when (value) {
                        SwipeToDismissBoxValue.StartToEnd -> {
                            tick()
                            if (currentRestoreRight) {
                                currentRestore()
                                true
                            } else {
                                currentDelete()
                                false
                            }
                        }
                        else -> {
                            tick()
                            if (currentRestoreRight) {
                                currentDelete()
                                false
                            } else {
                                currentRestore()
                                true
                            }
                        }
                    }
                    lastResult = result
                    result
                }
            }
        },
    )
    if (!swipeEnabled) {
        TrashContent(
            task = task,
            selected = selected,
            showActions = selection == null,
            onSelect = onSelect,
            onRestore = onRestore,
            onDelete = onDelete,
        )
        return
    }
    SwipeToDismissBox(
        state = state,
        enableDismissFromStartToEnd = true,
        enableDismissFromEndToStart = true,
        backgroundContent = {
            val start = state.dismissDirection == SwipeToDismissBoxValue.StartToEnd
            val restores = if (start) restoreRight else !restoreRight
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(if (restores) colors.success else colors.danger)
                    .padding(horizontal = 24.dp),
                contentAlignment = if (start) Alignment.CenterStart else Alignment.CenterEnd,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Icon(
                        imageVector = if (restores) Icons.Rounded.Undo else Icons.Outlined.Delete,
                        contentDescription = if (restores) "Restore" else "Delete",
                        tint = Color.White,
                        modifier = Modifier.size(20.dp),
                    )
                    Text(
                        text = if (restores) "Restore" else "Delete",
                        style = MaterialTheme.typography.labelLarge,
                        color = Color.White,
                    )
                }
            }
        },
    ) {
        TrashContent(
            task = task,
            selected = selected,
            showActions = false,
            onSelect = onSelect,
            onRestore = onRestore,
            onDelete = onDelete,
        )
    }
}

@Composable
private fun TrashContent(
    task: TaskView,
    selected: Boolean,
    showActions: Boolean,
    onSelect: () -> Unit,
    onRestore: () -> Unit,
    onDelete: () -> Unit,
) {
    val colors = flow
    val selection = LocalTaskSelection.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(
                if (selected) colors.accent.copy(alpha = 0.10f) else colors.background,
            )
            .clickable(enabled = selection != null, onClick = onSelect)
            .padding(horizontal = 20.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (selection != null) {
            Box(
                modifier = Modifier
                    .size(22.dp)
                    .clip(CircleShape)
                    .border(
                        1.5.dp,
                        if (selected) colors.accent else colors.hairline,
                        CircleShape,
                    )
                    .background(if (selected) colors.accent else Color.Transparent),
                contentAlignment = Alignment.Center,
            ) {
                if (selected) {
                    Icon(
                        imageVector = Icons.Rounded.Check,
                        contentDescription = "Selected",
                        tint = colors.onAccent,
                        modifier = Modifier.size(14.dp),
                    )
                }
            }
            Spacer(modifier = Modifier.width(12.dp))
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(task.title, style = MaterialTheme.typography.bodyMedium, color = colors.ink)
            Text(
                text = listOfNotNull(task.projectName, "In Trash").joinToString(" · "),
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
            )
        }
        if (showActions) {
            TextButton(onClick = onRestore) {
                Text("Restore", color = colors.accent)
            }
            TextButton(onClick = onDelete) {
                Text("Delete", color = colors.danger)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ArchiveProjectRow(
    project: ProjectDto,
    done: Int,
    total: Int,
    restoreLabel: String?,
    onOpen: () -> Unit,
    onRestore: () -> Unit,
    onDelete: () -> Unit,
) {
    val colors = flow
    val haptics = LocalHapticFeedback.current
    val hapticsEnabled = LocalHapticsEnabled.current
    val currentRestoreLabel by rememberUpdatedState(restoreLabel)
    val currentRestore by rememberUpdatedState(onRestore)
    val currentDelete by rememberUpdatedState(onDelete)
    fun tick() {
        if (hapticsEnabled) haptics.performHapticFeedback(HapticFeedbackType.LongPress)
    }
    // Material may confirm the same swipe twice while it settles; run the
    // action once and replay the first answer for the duplicate.
    var lastActionAt by remember { mutableStateOf(0L) }
    var lastResult by remember { mutableStateOf(false) }
    val state = rememberSwipeToDismissBoxState(
        confirmValueChange = { value ->
            if (value == SwipeToDismissBoxValue.Settled) {
                false
            } else {
                val now = System.currentTimeMillis()
                if (now - lastActionAt < 600) {
                    lastResult
                } else {
                    lastActionAt = now
                    val result = when (value) {
                        SwipeToDismissBoxValue.StartToEnd -> {
                            if (currentRestoreLabel == null) {
                                false
                            } else {
                                tick()
                                currentRestore()
                                // Snap back and let the list drop the row when
                                // the new status arrives; dismissing here lets
                                // the next row catch the tail of the gesture.
                                false
                            }
                        }
                        else -> {
                            tick()
                            currentDelete()
                            false
                        }
                    }
                    lastResult = result
                    result
                }
            }
        },
    )
    SwipeToDismissBox(
        state = state,
        enableDismissFromStartToEnd = restoreLabel != null,
        enableDismissFromEndToStart = true,
        backgroundContent = {
            val start = state.dismissDirection == SwipeToDismissBoxValue.StartToEnd
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(if (start) colors.success else colors.danger)
                    .padding(horizontal = 24.dp),
                contentAlignment = if (start) Alignment.CenterStart else Alignment.CenterEnd,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Icon(
                        imageVector = if (start) Icons.Rounded.Undo else Icons.Outlined.Delete,
                        contentDescription = if (start) currentRestoreLabel else "Delete",
                        tint = Color.White,
                        modifier = Modifier.size(20.dp),
                    )
                    Text(
                        text = if (start) currentRestoreLabel.orEmpty() else "Delete",
                        style = MaterialTheme.typography.labelLarge,
                        color = Color.White,
                    )
                }
            }
        },
    ) {
        ArchiveProjectContent(
            project = project,
            done = done,
            total = total,
            onOpen = onOpen,
        )
    }
}

@Composable
private fun ArchiveProjectContent(
    project: ProjectDto,
    done: Int,
    total: Int,
    onOpen: () -> Unit,
) {
    val colors = flow
    val statusIcon = when (project.status) {
        "done" -> Icons.Rounded.Check
        "dropped" -> Icons.Rounded.Close
        "on_hold" -> Icons.Rounded.Pause
        else -> null
    }
    val statusColor = if (project.status == "done") colors.success else colors.faint
    val statusLabel = when (project.status) {
        "done" -> "Done"
        "dropped" -> "Dropped"
        "on_hold" -> "On Hold"
        else -> "Active"
    }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(colors.background)
            .clickable { onOpen() }
            .padding(horizontal = 20.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = projectIcon(project.icon),
            contentDescription = null,
            tint = colors.faint,
            modifier = Modifier.size(15.dp),
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                project.name,
                style = MaterialTheme.typography.bodyMedium,
                color = colors.ink,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    "$done of $total done",
                    style = MaterialTheme.typography.labelSmall,
                    color = colors.faint,
                )
                Spacer(modifier = Modifier.width(8.dp))
                if (statusIcon != null) {
                    Icon(
                        imageVector = statusIcon,
                        contentDescription = statusLabel,
                        tint = statusColor,
                        modifier = Modifier.size(12.dp),
                    )
                    Spacer(modifier = Modifier.width(3.dp))
                }
                Text(
                    statusLabel,
                    style = MaterialTheme.typography.labelSmall,
                    color = statusColor,
                )
            }
        }
    }
}

@Composable
fun MoreDestination(
    view: MoreView,
    db: Db,
    sort: String,
    priority: String = "all",
    availableOnly: Boolean = false,
    group: String = "none",
    trashSwipe: String,
    pending: Set<String>,
    onTask: (TaskView) -> Unit,
    onToggle: (TaskView) -> Unit,
    onRestore: (String) -> Unit,
    onPurge: (String) -> Unit,
    onReview: (ProjectDto) -> Unit,
    onProject: (ProjectDto) -> Unit,
    onUnarchive: (ProjectDto) -> Unit,
    onReopen: (ProjectDto) -> Unit,
    onDeleteProject: (ProjectDto) -> Unit,
    onNewTag: () -> Unit,
    onEditTag: (TagDto) -> Unit,
    onDeleteTag: (TagDto) -> Unit,
    onOpenTag: (TagDto) -> Unit,
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
            tag != null -> TaskList(
                tasks = sortTasks(
                    filterViewTasks(db, db.tasksForTag(tag!!.id), priority, availableOnly),
                    sort,
                ),
                sort = sort,
                group = group,
                db = db,
                pending = pending,
                onToggle = onToggle,
                onTask = onTask,
            )
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
            view == MoreView.Flagged -> TaskList(
                tasks = sortTasks(
                    filterViewTasks(db, db.flaggedTasks(), priority, availableOnly),
                    sort,
                ),
                sort = sort,
                group = group,
                db = db,
                pending = pending,
                onToggle = onToggle,
                onTask = onTask,
            )
            view == MoreView.Completed -> TaskList(
                tasks = sortTasks(
                    filterViewTasks(db, db.completedTasks(), priority, availableOnly),
                    sort,
                ),
                sort = sort,
                group = group,
                db = db,
                pending = pending,
                onToggle = onToggle,
                onTask = onTask,
            )
            view == MoreView.Archive -> LazyColumn {
                if (db.archivedProjects().isEmpty()) {
                    item(key = "archive-empty") {
                        EmptyLine("No archived projects yet.")
                    }
                }
                items(db.archivedProjects(), key = { it.id }) { project ->
                    val projectTasks = db.tasks.values.filter {
                        it.project == project.id && it.trashedAt == null
                    }
                    val done = projectTasks.count { it.completed }
                    val restoreLabel = when {
                        project.archived && project.status == "active" -> "Unarchive"
                        project.status != "active" -> "Reopen"
                        else -> null
                    }
                    ArchiveProjectRow(
                        project = project,
                        done = done,
                        total = projectTasks.size,
                        restoreLabel = restoreLabel,
                        onOpen = { onProject(project) },
                        onRestore = {
                            if (restoreLabel == "Unarchive") {
                                onUnarchive(project)
                            } else {
                                onReopen(project)
                            }
                        },
                        onDelete = { onDeleteProject(project) },
                    )
                    Hairline(Modifier.padding(start = 20.dp))
                }
            }
            view == MoreView.Trash -> LazyColumn {
                val trashTasks = sortTasks(db.trashedTasks(), sort)
                if (trashTasks.isEmpty()) {
                    item(key = "trash-empty") {
                        EmptyLine("Trash is empty.")
                    }
                }
                taskSections(trashTasks, sort).forEach { section ->
                    section.label?.let { label ->
                        item(key = "trash-section-$label") { SectionLabel(label) }
                    }
                items(section.tasks, key = { it.id }) { task ->
                    val selection = LocalTaskSelection.current
                    TrashRow(
                        task = task,
                        selected = selection?.ids?.contains(task.id) == true,
                        swipe = trashSwipe,
                        onSelect = { selection?.toggle(task.id) },
                        onRestore = { onRestore(task.id) },
                        onDelete = { purgeTarget = task },
                    )
                    Hairline(Modifier.padding(horizontal = 20.dp))
                }
                }
            }
            view == MoreView.Nearby -> NearbyList(db, onOpenTag)
            view == MoreView.Tags -> LazyColumn {
                item(key = "new-tag") {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable(onClick = onNewTag)
                            .padding(horizontal = 20.dp, vertical = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.Add,
                            contentDescription = null,
                            tint = colors.accent,
                            modifier = Modifier.size(18.dp),
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Text(
                            text = "New tag",
                            style = MaterialTheme.typography.bodyMedium,
                            color = colors.accent,
                        )
                    }
                    Hairline(Modifier.padding(start = 20.dp))
                }
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
                    var menu by remember { mutableStateOf(false) }
                    val count = db.tagUsageCount(item.id)
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Row(
                            modifier = Modifier
                                .weight(1f)
                                .clickable { tag = item }
                                .padding(start = 20.dp, top = 14.dp, bottom = 14.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                "#${item.name}",
                                style = MaterialTheme.typography.bodyMedium,
                                color = if (db.tagIsBlocked(item.id)) colors.faint else colors.ink,
                                modifier = Modifier.weight(1f),
                            )
                            if (item.status != "available") {
                                Text(
                                    tagStatusLabel(item.status),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = colors.warning,
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                            }
                            Text(
                                "$count",
                                style = MaterialTheme.typography.labelMedium,
                                color = colors.faint,
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                        }
                        Box {
                            IconButton(
                                onClick = { menu = true },
                                modifier = Modifier.size(38.dp),
                            ) {
                                Icon(
                                    imageVector = Icons.Rounded.MoreVert,
                                    contentDescription = "Tag actions",
                                    tint = colors.faint,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                            DropdownMenu(
                                expanded = menu,
                                onDismissRequest = { menu = false },
                                containerColor = colors.raised,
                            ) {
                                DropdownMenuItem(
                                    text = { Text("Edit tag") },
                                    onClick = {
                                        menu = false
                                        onEditTag(item)
                                    },
                                )
                                DropdownMenuItem(
                                    text = { Text("Delete tag", color = colors.danger) },
                                    onClick = {
                                        menu = false
                                        onDeleteTag(item)
                                    },
                                )
                            }
                        }
                    }
                    Hairline(Modifier.padding(start = 20.dp))
                }
            }
            view == MoreView.Priorities -> LazyColumn {
                db.priorityGroups().forEach { (label, rawTasks) ->
                    val tasks = sortTasks(
                        filterViewTasks(db, rawTasks, priority, availableOnly),
                        sort,
                    )
                    if (tasks.isEmpty()) return@forEach
                    item(key = "prio-$label") {
                        Text(
                            text = label.uppercase(),
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.SemiBold,
                            letterSpacing = 0.8.sp,
                            color = colors.faint,
                            modifier = Modifier.padding(
                                start = 20.dp,
                                end = 20.dp,
                                top = 18.dp,
                                bottom = 6.dp,
                            ),
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
            view == MoreView.Changed -> TaskList(
                tasks = sortTasks(
                    filterViewTasks(db, db.changedTasks(), priority, availableOnly),
                    sort,
                ),
                sort = sort,
                group = group,
                db = db,
                pending = pending,
                onToggle = onToggle,
                onTask = onTask,
            )
            view == MoreView.Attachments -> LazyColumn {
                val attachments = sortTasks(
                    filterViewTasks(db, db.attachmentTasks(), priority, availableOnly),
                    sort,
                )
                if (attachments.isEmpty()) {
                    item(key = "attachments-empty") {
                        EmptyLine("No attachments yet.")
                    }
                }
                groupSections(db, attachments, group, sort).forEach { section ->
                    section.label?.let { label ->
                        item(key = "attach-section-$label") { AttachmentSectionLabel(label) }
                    }
                items(section.tasks, key = { it.id }) { task ->
                    val selection = LocalTaskSelection.current
                    val selected = selection?.ids?.contains(task.id) == true
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .background(
                                if (selected) colors.accent.copy(alpha = 0.10f) else Color.Transparent,
                            )
                            .clickable {
                                if (selection != null) {
                                    selection.toggle(task.id)
                                } else {
                                    onTask(task)
                                }
                            }
                            .padding(horizontal = 20.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        if (selection != null) {
                            Box(
                                modifier = Modifier
                                    .size(22.dp)
                                    .clip(CircleShape)
                                    .border(
                                        1.5.dp,
                                        if (selected) colors.accent else colors.hairline,
                                        CircleShape,
                                    )
                                    .background(if (selected) colors.accent else Color.Transparent),
                                contentAlignment = Alignment.Center,
                            ) {
                                if (selected) {
                                    Icon(
                                        imageVector = Icons.Rounded.Check,
                                        contentDescription = "Selected",
                                        tint = colors.onAccent,
                                        modifier = Modifier.size(14.dp),
                                    )
                                }
                            }
                            Spacer(modifier = Modifier.width(12.dp))
                        }
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                task.title,
                                style = MaterialTheme.typography.bodyMedium,
                                color = colors.ink,
                            )
                            Text(
                                text = task.attachments.joinToString(" · ") {
                                    it.substringAfterLast('/')
                                },
                                style = MaterialTheme.typography.labelMedium,
                                color = colors.faint,
                            )
                        }
                    }
                    Hairline(
                        Modifier.padding(start = if (selection != null) 70.dp else 20.dp),
                    )
                }
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
private fun AttachmentSectionLabel(label: String) {
    val colors = flow
    val pill = label == "No priority" || label.startsWith("No ")
    if (pill) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 20.dp, end = 20.dp, top = 16.dp, bottom = 6.dp),
        ) {
            val pillBackground = if (colors.dark) Color.White else colors.field
            val pillText = if (colors.dark) Color(0xFF707070) else colors.ink
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(6.dp))
                    .background(pillBackground)
                    .padding(horizontal = 8.dp, vertical = 4.dp),
            ) {
                Text(
                    text = label,
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.SemiBold,
                    letterSpacing = 0.8.sp,
                    color = pillText,
                )
            }
        }
        return
    }
    val color = when (label) {
        "High priority" -> colors.danger
        "Medium priority" -> colors.warning
        "Low priority" -> LowPriorityBlue
        else -> colors.accent
    }
    Text(
        text = label.uppercase(),
        style = MaterialTheme.typography.labelSmall,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.8.sp,
        color = color,
        modifier = Modifier.padding(start = 20.dp, end = 20.dp, top = 16.dp, bottom = 6.dp),
    )
}

@Composable
private fun TaskList(
    tasks: List<TaskView>,
    sort: String,
    group: String,
    db: Db,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onTask: (TaskView) -> Unit,
) {
    val sections = groupSections(db, tasks, group, sort)
    LazyColumn(contentPadding = PaddingValues(bottom = 20.dp)) {
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
        sections.forEach { section ->
            section.label?.let { label ->
                item(key = "section-$label") { SectionLabel(label) }
            }
            items(section.tasks, key = { it.id }) { task ->
                TaskRow(
                    task = task.copy(pending = task.id in pending),
                    onToggle = { onToggle(task) },
                    onClick = { onTask(task) },
                )
                Hairline(Modifier.padding(start = 58.dp))
            }
        }
    }
}

@Composable
private fun NearbyList(db: Db, onOpenTag: (TagDto) -> Unit) {
    val colors = flow
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var deviceLocation by remember { mutableStateOf<GeoDto?>(null) }
    var requested by remember { mutableStateOf(false) }
    val locationPermission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { grants ->
        if (grants.values.any { it }) {
            scope.launch { deviceLocation = DeviceLocation.current(context) }
        }
    }
    LaunchedEffect(Unit) {
        if (DeviceLocation.hasForeground(context)) {
            deviceLocation = DeviceLocation.current(context)
        }
    }
    val origin = deviceLocation ?: db.settings.currentLocation
    if (origin == null) {
        val hasPermission = DeviceLocation.hasForeground(context)
        Column(modifier = Modifier.fillMaxWidth()) {
            EmptyLine(
                if (hasPermission) {
                    "Finding your location…"
                } else {
                    "Allow location access or set the library location on the Mac in Settings → Nearby."
                },
            )
            TextButton(
                onClick = {
                    if (hasPermission) {
                        scope.launch { deviceLocation = DeviceLocation.current(context) }
                    } else {
                        requested = true
                        locationPermission.launch(
                            arrayOf(
                                Manifest.permission.ACCESS_FINE_LOCATION,
                                Manifest.permission.ACCESS_COARSE_LOCATION,
                            ),
                        )
                    }
                },
                modifier = Modifier.padding(horizontal = 20.dp),
            ) {
                Text(
                    text = if (hasPermission) "Try again" else "Use my location",
                    color = colors.accent,
                )
            }
        }
        return
    }
    val radius = db.settings.nearbyRadiusKm
    val nearby = db.tags.values
        .mapNotNull { tag ->
            tag.location?.let { place -> tag to distanceKm(origin, place) }
        }
        .filter { (_, km) -> km <= radius }
        .sortedBy { (_, km) -> km }
    if (nearby.isEmpty()) {
        EmptyLine("No tagged places within ${radius.toInt()} km.")
        return
    }
    LazyColumn(contentPadding = PaddingValues(bottom = 20.dp)) {
        item(key = "nearby-header") {
            Column(modifier = Modifier.padding(horizontal = 20.dp, vertical = 10.dp)) {
                Text(
                    text = "WITHIN ${radius.toInt()} KM",
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.SemiBold,
                    color = colors.faint,
                )
                Text(
                    text = if (deviceLocation != null) {
                        "Using your phone's location"
                    } else {
                        "Using the library location"
                    },
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.faint,
                    modifier = Modifier.padding(top = 2.dp),
                )
            }
        }
        items(nearby, key = { it.first.id }) { (tag, km) ->
            val count = db.tagUsageCount(tag.id)
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { onOpenTag(tag) }
                    .padding(horizontal = 20.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    imageVector = Icons.Outlined.Tag,
                    contentDescription = null,
                    tint = colors.faint,
                    modifier = Modifier.size(18.dp),
                )
                Spacer(modifier = Modifier.width(12.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "#${tag.name}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.ink,
                    )
                    Text(
                        text = "${"%.1f".format(km)} km away · $count task${if (count == 1) "" else "s"}",
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.faint,
                    )
                }
                Icon(
                    imageVector = Icons.Outlined.ChevronRight,
                    contentDescription = null,
                    tint = colors.faint,
                    modifier = Modifier.size(18.dp),
                )
            }
            Hairline(Modifier.padding(start = 50.dp))
        }
    }
}

private fun distanceKm(origin: GeoDto, place: GeoDto): Double {
    val earth = 6371.0
    val dLat = Math.toRadians(place.lat - origin.lat)
    val dLon = Math.toRadians(place.lon - origin.lon)
    val a = kotlin.math.sin(dLat / 2) * kotlin.math.sin(dLat / 2) +
        kotlin.math.cos(Math.toRadians(origin.lat)) *
        kotlin.math.cos(Math.toRadians(place.lat)) *
        kotlin.math.sin(dLon / 2) * kotlin.math.sin(dLon / 2)
    return earth * 2 * kotlin.math.atan2(kotlin.math.sqrt(a), kotlin.math.sqrt(1 - a))
}
