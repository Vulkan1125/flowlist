package dev.flowlist.android.ui

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
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.CalendarMonth
import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.Inbox
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.outlined.WbSunny
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.FloatingActionButtonDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.data.AppState
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.Repository
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.forecast
import dev.flowlist.android.data.inbox
import dev.flowlist.android.data.isActive
import dev.flowlist.android.data.isOverdue
import dev.flowlist.android.data.label
import dev.flowlist.android.data.today
import dev.flowlist.android.data.isToday
import kotlinx.coroutines.launch
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

private enum class Tab(val label: String, val icon: ImageVector) {
    Inbox("Inbox", Icons.Outlined.Inbox),
    Today("Today", Icons.Outlined.WbSunny),
    Forecast("Forecast", Icons.Outlined.CalendarMonth),
    Browse("Browse", Icons.Outlined.FolderOpen),
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
fun MainScreen(
    repository: Repository,
    state: AppState,
    scope: kotlinx.coroutines.CoroutineScope,
    theme: ThemeSettings,
    onThemeChange: (ThemeSettings) -> Unit,
    initialTab: String?,
    onTabConsumed: () -> Unit,
) {
    val colors = flow
    var tab by rememberSaveable { mutableStateOf(Tab.Inbox) }
    var capture by rememberSaveable { mutableStateOf(false) }
    var settings by rememberSaveable { mutableStateOf(false) }
    var detail by remember { mutableStateOf<TaskView?>(null) }
    var project by remember { mutableStateOf<ProjectDto?>(null) }
    val haptics = LocalHapticFeedback.current
    LaunchedEffect(initialTab) {
        when (initialTab) {
            "today" -> tab = Tab.Today
            "forecast" -> tab = Tab.Forecast
            "browse" -> tab = Tab.Browse
        }
        if (initialTab != null) onTabConsumed()
    }
    val snapshot = state.snapshot
    val snackbarHost = remember { SnackbarHostState() }

    val complete: (TaskView, Boolean) -> Unit = { task, done ->
        haptics.performHapticFeedback(HapticFeedbackType.LongPress)
        scope.launch {
            repository.complete(task.id, done)
            val result = snackbarHost.showSnackbar(
                message = if (done) "Completed “${task.title}”" else "Reopened “${task.title}”",
                actionLabel = "Undo",
                withDismissAction = false,
                duration = SnackbarDuration.Long,
            )
            if (result == SnackbarResult.ActionPerformed) {
                repository.complete(task.id, !done)
            }
        }
    }
    val trash: (TaskView) -> Unit = { task ->
        scope.launch {
            val removed = repository.delete(task.id)
            val result = snackbarHost.showSnackbar(
                message = "Moved “${task.title}” to Trash",
                actionLabel = "Undo",
                withDismissAction = false,
                duration = SnackbarDuration.Long,
            )
            if (result == SnackbarResult.ActionPerformed && removed != null) {
                repository.restore(removed)
            }
        }
    }

    Scaffold(
        containerColor = colors.background,
        snackbarHost = { FlowSnackbarHost(snackbarHost) },
        bottomBar = {
            NavigationBar(
                containerColor = colors.background,
                tonalElevation = 0.dp,
            ) {
                Tab.entries.forEach { item ->
                    NavigationBarItem(
                        selected = tab == item,
                        onClick = { tab = item },
                        icon = {
                            Icon(
                                imageVector = item.icon,
                                contentDescription = item.label,
                                modifier = Modifier.size(21.dp),
                            )
                        },
                        label = {
                            Text(item.label, style = MaterialTheme.typography.labelSmall)
                        },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = colors.accent,
                            selectedTextColor = colors.accent,
                            unselectedIconColor = colors.faint,
                            unselectedTextColor = colors.faint,
                            indicatorColor = Color.Transparent,
                        ),
                    )
                }
            }
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = {
                    haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                    capture = true
                },
                containerColor = colors.accent,
                contentColor = colors.onAccent,
                shape = CircleShape,
                elevation = FloatingActionButtonDefaults.elevation(defaultElevation = 3.dp),
            ) {
                Icon(
                    imageVector = Icons.Rounded.Add,
                    contentDescription = "New task",
                    modifier = Modifier.size(26.dp),
                )
            }
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .background(colors.background),
        ) {
            HorizontalDivider(thickness = 1.dp, color = colors.background)
            Header(
                tab = tab,
                state = state,
                onSettings = { settings = true },
                modifier = Modifier.statusBarsPadding(),
            )
            if (state.error != null) {
                ErrorBanner(
                    message = state.error,
                    onRetry = { scope.launch { repository.sync() } },
                    onDismiss = repository::clearError,
                )
            }
            when (tab) {
                Tab.Inbox -> InboxList(
                    tasks = snapshot?.db?.inbox().orEmpty(),
                    pending = state.pending,
                    onToggle = { task -> complete(task, true) },
                    onClick = { detail = it },
                    onTrash = trash,
                )
                Tab.Today -> TodayList(
                    tasks = snapshot?.db?.today().orEmpty(),
                    pending = state.pending,
                    onToggle = { task -> complete(task, true) },
                    onClick = { detail = it },
                    onTrash = trash,
                )
                Tab.Forecast -> ForecastList(
                    groups = snapshot?.db?.forecast().orEmpty(),
                    pending = state.pending,
                    onToggle = { task -> complete(task, true) },
                    onClick = { detail = it },
                    onTrash = trash,
                )
                Tab.Browse -> BrowseList(
                    projects = snapshot?.db?.projects?.values?.sortedBy { it.name.lowercase() }.orEmpty(),
                    snapshot = snapshot,
                    onOpen = { project = it },
                )
            }
        }
    }

    if (capture) {
        CaptureSheet(
            onDismiss = { capture = false },
            onAdd = { text ->
                capture = false
                haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                scope.launch { repository.capture(text) }
            },
        )
    }

    detail?.let { taskView ->
        val db = snapshot?.db
        val dto = db?.tasks?.get(taskView.id)
        if (dto != null) {
            TaskEditSheet(
                task = dto,
                projects = db.projects.values.toList(),
                tags = db.tags.values.toList(),
                pending = taskView.id in state.pending,
                onDismiss = { detail = null },
                onSave = { edit ->
                    detail = null
                    scope.launch {
                        repository.update(dto, edit)
                        snackbarHost.showSnackbar(
                            message = "Saved “${edit.title}”",
                            duration = SnackbarDuration.Short,
                        )
                    }
                },
                onComplete = {
                    complete(taskView, !taskView.completed)
                    detail = null
                },
                onDelete = {
                    trash(taskView)
                    detail = null
                },
            )
        }
    }

    project?.let { value ->
        val db = snapshot?.db
        ProjectSheet(
            project = value,
            tasks = db?.tasks?.values
                ?.filter { it.project == value.id && it.isActive }
                ?.sortedBy { it.title.lowercase() }
                ?.map { task ->
                    TaskView(
                        id = task.id,
                        title = task.title,
                        notes = task.notes,
                        projectName = value.name,
                        projectId = value.id,
                        parent = task.parent,
                        tags = task.tags.mapNotNull { db?.tags?.get(it)?.name },
                        due = task.due,
                        defer = task.defer,
                        flagged = task.flagged,
                        priority = task.priority,
                        completed = task.completed,
                        pending = task.id in state.pending,
                        checklistDone = task.checklist.count { it.completed },
                        checklistTotal = task.checklist.size,
                        estimateMin = task.estimateMin,
                    )
                }
                .orEmpty(),
            onDismiss = { project = null },
            onToggle = { task -> complete(task, true) },
            onClick = { task ->
                project = null
                detail = task
            },
        )
    }

    if (settings) {
        SettingsSheet(
            state = state,
            onDismiss = { settings = false },
            onSync = { scope.launch { repository.sync() } },
            onSignOut = {
                settings = false
                scope.launch { repository.signOut() }
            },
            theme = theme,
            onThemeChange = onThemeChange,
        )
    }
}

@Composable
private fun Header(
    tab: Tab,
    state: AppState,
    onSettings: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = flow
    val today = LocalDate.now()
    Column(modifier = modifier.fillMaxWidth().padding(start = 20.dp, end = 8.dp, top = 18.dp, bottom = 12.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = tab.label,
                    style = MaterialTheme.typography.displaySmall,
                    color = colors.ink,
                )
                Spacer(modifier = Modifier.height(3.dp))
                Text(
                    text = when (tab) {
                        Tab.Inbox -> state.snapshot?.let { snapshot ->
                            val count = snapshot.db.inbox().size
                            if (count == 0) "Nothing waiting" else "$count to sort"
                        } ?: "Loading your library"

                        Tab.Today -> today.format(
                            DateTimeFormatter.ofPattern("EEEE, d MMMM", Locale.getDefault()),
                        )

                        Tab.Forecast -> "The next two weeks"
                        Tab.Browse -> "${state.snapshot?.db?.projects?.size ?: 0} projects"
                    },
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.faint,
                )
            }
            when {
                state.pendingCount > 0 -> SyncDot("${state.pendingCount} waiting", colors.warning)
                state.syncing -> SyncDot("Syncing", colors.faint)
                !state.online -> SyncDot("Offline", colors.danger)
                else -> SyncDot("Synced", colors.success)
            }
            Spacer(modifier = Modifier.width(4.dp))
            IconButton(onClick = onSettings) {
                Icon(
                    imageVector = Icons.Outlined.Settings,
                    contentDescription = "Settings",
                    tint = colors.faint,
                    modifier = Modifier.size(20.dp),
                )
            }
        }
    }
}

@Composable
private fun ErrorBanner(message: String, onRetry: () -> Unit, onDismiss: () -> Unit) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 6.dp)
            .clip(RoundedCornerShape(10.dp))
            .background(colors.field)
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.labelMedium,
            color = colors.ink,
            modifier = Modifier.weight(1f),
        )
        TextButton(onClick = onRetry) {
            Text("Retry", color = colors.ink, style = MaterialTheme.typography.labelMedium)
        }
        TextButton(onClick = onDismiss) {
            Text("Dismiss", color = colors.faint, style = MaterialTheme.typography.labelMedium)
        }
    }
}

@Composable
private fun InboxList(
    tasks: List<TaskView>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
) {
    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        if (tasks.isEmpty()) {
            item(key = "empty") {
                EmptyState(
                    title = "Inbox zero",
                    body = "A clean slate. Capture something and it lands here until you give it a date or project.",
                    doodle = { InboxDoodle() },
                )
            }
        }
        items(tasks, key = { it.id }) { task ->
            SwipeTaskRow(
                task = task,
                pending = task.id in pending,
                onToggle = { onToggle(task) },
                onTrash = { onTrash(task) },
                onClick = { onClick(task) },
            )
        }
    }
}

@Composable
private fun TodayList(
    tasks: List<TaskView>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
) {
    val overdue = tasks.filter { it.due?.isOverdue() == true }
    val today = tasks - overdue.toSet()
    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        if (tasks.isEmpty()) {
            item(key = "empty") {
                EmptyState(
                    title = "Nothing due today",
                    body = "Enjoy the quiet, or look ahead in Forecast.",
                    doodle = { TodayDoodle() },
                )
            }
        }
        if (overdue.isNotEmpty()) {
            item { SectionLabel("Overdue") }
            items(overdue, key = { it.id }) { task ->
                SwipeTaskRow(
                    task = task,
                    pending = task.id in pending,
                    onToggle = { onToggle(task) },
                    onTrash = { onTrash(task) },
                    onClick = { onClick(task) },
                )
            }
        }
        if (today.isNotEmpty()) {
            item { SectionLabel(if (overdue.isEmpty()) "Due today" else "Today") }
            items(today, key = { it.id }) { task ->
                SwipeTaskRow(
                    task = task,
                    pending = task.id in pending,
                    onToggle = { onToggle(task) },
                    onTrash = { onTrash(task) },
                    onClick = { onClick(task) },
                )
            }
        }
    }
}

@Composable
private fun ForecastList(
    groups: Map<LocalDate, List<TaskView>>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
) {
    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        if (groups.values.all { it.isEmpty() }) {
            item(key = "empty") {
                EmptyState(
                    title = "An open fortnight",
                    body = "Give a task a date and it appears here, grouped by the day it is due.",
                    doodle = { ForecastDoodle() },
                )
            }
        }
        groups.forEach { (date, tasks) ->
            item(key = "day-$date") { ForecastHeader(date, tasks.size) }
            items(tasks, key = { it.id }) { task ->
                SwipeTaskRow(
                    task = task,
                    pending = task.id in pending,
                    onToggle = { onToggle(task) },
                    onTrash = { onTrash(task) },
                    onClick = { onClick(task) },
                )
            }
        }
    }
}

@Composable
private fun ForecastHeader(date: LocalDate, count: Int) {
    val colors = flow
    val today = LocalDate.now()
    val title = when (date) {
        today -> "Today"
        today.plusDays(1) -> "Tomorrow"
        else -> date.format(DateTimeFormatter.ofPattern("EEEE, d MMM", Locale.getDefault()))
    }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(colors.background)
            .padding(start = 20.dp, end = 20.dp, top = 18.dp, bottom = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            color = if (date.isBefore(today)) colors.danger else colors.ink,
        )
        Spacer(modifier = Modifier.width(8.dp))
        Text(
            text = "$count",
            style = MaterialTheme.typography.labelMedium,
            color = colors.faint,
        )
    }
}

@Composable
private fun BrowseList(
    projects: List<ProjectDto>,
    snapshot: dev.flowlist.android.data.Snapshot?,
    onOpen: (ProjectDto) -> Unit,
) {
    val colors = flow
    if (projects.isEmpty()) {
        EmptyState(
            title = "No projects yet",
            body = "Projects created on the Mac appear here with their open tasks.",
            doodle = { BrowseDoodle() },
        )
        return
    }
    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        items(projects, key = { it.id }) { project ->
            val active = snapshot?.db?.tasks?.values?.count {
                it.project == project.id && it.isActive
            } ?: 0
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { onOpen(project) }
                    .padding(horizontal = 20.dp, vertical = 15.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier = Modifier
                        .size(8.dp)
                        .clip(CircleShape)
                        .background(if (active == 0) colors.hairline else colors.accent),
                )
                Spacer(modifier = Modifier.width(14.dp))
                Text(
                    text = project.name,
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.ink,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                Text(
                    text = if (active == 0) "Done" else "$active",
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.faint,
                )
            }
            Hairline(Modifier.padding(start = 42.dp))
        }
    }
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
private fun ProjectSheet(
    project: ProjectDto,
    tasks: List<TaskView>,
    onDismiss: () -> Unit,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(modifier = Modifier.fillMaxWidth().padding(bottom = 28.dp)) {
            Text(
                text = project.name,
                style = MaterialTheme.typography.headlineMedium,
                color = colors.ink,
                modifier = Modifier.padding(horizontal = 20.dp),
            )
            Text(
                text = if (tasks.isEmpty()) "Everything is done" else "${tasks.size} open",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp),
            )
            Spacer(modifier = Modifier.height(8.dp))
            tasks.forEach { task ->
                TaskRow(task = task, onToggle = { onToggle(task) }, onClick = { onClick(task) })
                Hairline(Modifier.padding(start = 58.dp))
            }
        }
    }
}

@Composable
fun SheetHandle() {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier = Modifier
                .width(38.dp)
                .height(4.dp)
                .clip(CircleShape)
                .background(flow.hairline),
        )
    }
}
