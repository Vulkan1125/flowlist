package dev.flowlist.android.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.NavigationRailItemDefaults
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Archive
import androidx.compose.material.icons.outlined.BookmarkBorder
import androidx.compose.material.icons.outlined.CalendarMonth
import androidx.compose.material.icons.outlined.CreateNewFolder
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Flag
import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.Inbox
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material.icons.rounded.CheckCircle
import androidx.compose.material.icons.rounded.Checklist
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.Delete
import androidx.compose.material.icons.rounded.DeleteForever
import androidx.compose.material.icons.rounded.ExpandMore
import androidx.compose.material.icons.rounded.History
import androidx.compose.material.icons.rounded.Menu
import androidx.compose.material.icons.rounded.MoreVert
import androidx.compose.material.icons.rounded.Redo
import androidx.compose.material.icons.rounded.Undo
import androidx.compose.material.icons.outlined.WbSunny
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.FloatingActionButtonDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.NavigationDrawerItem
import androidx.compose.material3.rememberDrawerState
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Text
import androidx.compose.material3.Surface
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.activity.compose.BackHandler
import androidx.compose.ui.unit.sp
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import dev.flowlist.android.data.AppPreferences
import dev.flowlist.android.data.AppState
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.attachmentTasks
import dev.flowlist.android.data.changedTasks
import dev.flowlist.android.data.completedTasks
import dev.flowlist.android.data.flaggedTasks
import dev.flowlist.android.data.priorityGroups
import dev.flowlist.android.data.FolderDto
import dev.flowlist.android.data.PerspectiveDto
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.Repository
import dev.flowlist.android.data.SharedPayload
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.forecast
import dev.flowlist.android.data.inbox
import dev.flowlist.android.data.inboxPlacement
import dev.flowlist.android.data.parentPlacement
import dev.flowlist.android.data.projectPlacement
import dev.flowlist.android.data.orderedChildTasks
import dev.flowlist.android.data.isActive
import dev.flowlist.android.data.isOverdue
import dev.flowlist.android.data.label
import dev.flowlist.android.data.linkWithName
import dev.flowlist.android.data.today
import dev.flowlist.android.data.taskViews
import dev.flowlist.android.data.tagBlocking
import dev.flowlist.android.data.tasksForTag
import dev.flowlist.android.data.trashedTasks
import dev.flowlist.android.data.isToday
import kotlinx.coroutines.delay
import kotlin.math.roundToInt
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

enum class Tab(val label: String, val icon: ImageVector) {
    Inbox("Inbox", Icons.Outlined.Inbox),
    Today("Today", Icons.Outlined.WbSunny),
    Forecast("Forecast", Icons.Outlined.CalendarMonth),
    Browse("Browse", Icons.Outlined.FolderOpen),
    Settings("Settings", Icons.Outlined.Settings),
}

enum class Perspective(
    val key: String,
    val label: String,
    val icon: ImageVector,
    val tab: Tab? = null,
    val more: MoreView? = null,
) {
    Inbox("inbox", "Inbox", Icons.Outlined.Inbox, tab = Tab.Inbox),
    Today("today", "Today", Icons.Outlined.WbSunny, tab = Tab.Today),
    Forecast("forecast", "Forecast", Icons.Outlined.CalendarMonth, tab = Tab.Forecast),
    Flagged("flagged", "Flagged", Icons.Outlined.Flag, more = MoreView.Flagged),
    Library("browse", "Library", Icons.Outlined.FolderOpen, tab = Tab.Browse),
    Review("review", "Review", MoreView.Review.icon, more = MoreView.Review),
    Completed("completed", "Completed", MoreView.Completed.icon, more = MoreView.Completed),
    Archive("archive", "Archive", MoreView.Archive.icon, more = MoreView.Archive),
    Trash("trash", "Trash", MoreView.Trash.icon, more = MoreView.Trash),
    Tags("tags", "Tags", MoreView.Tags.icon, more = MoreView.Tags),
    Nearby("nearby", "Nearby", MoreView.Nearby.icon, more = MoreView.Nearby),
    Priorities("priorities", "Priorities", MoreView.Priorities.icon, more = MoreView.Priorities),
    Changed("changed", "Changed", MoreView.Changed.icon, more = MoreView.Changed),
    Attachments("attachments", "Attachments", MoreView.Attachments.icon, more = MoreView.Attachments),
}

fun resolveBottomBar(keys: List<String>): List<Perspective> {
    val chosen = keys
        .mapNotNull { key -> Perspective.entries.firstOrNull { it.key == key } }
        .distinct()
        .take(4)
        .toMutableList()
    for (item in Perspective.entries) {
        if (chosen.size >= 4) break
        if (item !in chosen) chosen.add(item)
    }
    return chosen
}

/// Apply the saved drawer order, then append any perspectives the saved order
/// does not know about yet (for example after an update adds one).
fun orderedPerspectives(order: List<String>): List<Perspective> {
    val result = mutableListOf<Perspective>()
    order.forEach { key ->
        Perspective.entries.firstOrNull { it.key == key }?.let { item ->
            if (item !in result) result.add(item)
        }
    }
    Perspective.entries.forEach { item -> if (item !in result) result.add(item) }
    return result
}

/// Move one drawer row and keep every other row (including bottom-bar picks)
/// in the slot it already occupied.
fun reorderPerspectiveKeys(
    order: List<String>,
    moving: List<String>,
    from: Int,
    to: Int,
): List<String> {
    if (from == to || moving.isEmpty()) return order
    val next = moving.toMutableList()
    val item = next.removeAt(from)
    next.add(to, item)
    val slots = order.withIndex().filter { it.value in moving }.map { it.index }
    if (slots.size != next.size) return order
    val result = order.toMutableList()
    next.forEachIndexed { index, key -> result[slots[index]] = key }
    return result
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
    shared: SharedPayload? = null,
    onSharedConsumed: () -> Unit = {},
    initialMore: String? = null,
    onMoreConsumed: () -> Unit = {},
    preferences: AppPreferences,
    onPreferencesChange: (AppPreferences) -> Unit,
) {
    val colors = flow
    var tab by rememberSaveable {
        mutableStateOf(
            when (preferences.startTab) {
                "today" -> Tab.Today
                "forecast" -> Tab.Forecast
                "browse" -> Tab.Browse
                else -> Tab.Inbox
            },
        )
    }
    var capture by rememberSaveable { mutableStateOf(false) }
    var captureInitial by rememberSaveable { mutableStateOf("") }
    var captureAttachment by remember { mutableStateOf<String?>(null) }
    var captureAttachmentLabel by remember { mutableStateOf<String?>(null) }
    var sharePrompt by remember { mutableStateOf<SharedPayload?>(null) }
    var previewTarget by remember { mutableStateOf<PreviewTarget?>(null) }
    var tagTasks by remember { mutableStateOf<TagDto?>(null) }
    var sharePicking by remember { mutableStateOf(false) }
    var selecting by rememberSaveable { mutableStateOf(false) }
    var selectedIds by remember { mutableStateOf<Set<String>>(emptySet()) }
    var search by rememberSaveable { mutableStateOf(false) }
    var detail by remember { mutableStateOf<TaskView?>(null) }
    var project by remember { mutableStateOf<ProjectDto?>(null) }
    var moreView by remember { mutableStateOf<MoreView?>(null) }
    var projectEditor by remember { mutableStateOf<ProjectEditorTarget?>(null) }
    var folderEditor by remember { mutableStateOf<FolderEditorTarget?>(null) }
    var tagEditor by remember { mutableStateOf<TagEditorTarget?>(null) }
    var perspectiveEditor by remember { mutableStateOf<PerspectiveEditorTarget?>(null) }
    var savedPerspective by remember { mutableStateOf<PerspectiveDto?>(null) }
    var confirm by remember { mutableStateOf<ConfirmRequest?>(null) }
    BackHandler(enabled = moreView != null) { moreView = null }
    BackHandler(enabled = savedPerspective != null) { savedPerspective = null }
    val drawer = rememberDrawerState(DrawerValue.Closed)
    val haptics = LocalHapticFeedback.current
    var offlineWarning by remember { mutableStateOf(false) }
    LaunchedEffect(state.offlineSince) {
        if (state.offlineSince != null) {
            delay(60_000)
            offlineWarning = true
        } else {
            offlineWarning = false
        }
    }
    LaunchedEffect(initialMore) {
        if (initialMore == "nearby") {
            moreView = MoreView.Nearby
            savedPerspective = null
        }
        if (initialMore != null) onMoreConsumed()
    }
    LaunchedEffect(initialTab) {
        when (initialTab) {
            "today" -> tab = Tab.Today
            "forecast" -> tab = Tab.Forecast
            "browse" -> tab = Tab.Browse
        }
        if (initialTab != null) onTabConsumed()
    }
    val snapshot = state.snapshot
    LaunchedEffect(preferences.saveAllAttachments, snapshot?.revision, state.online) {
        if (preferences.saveAllAttachments && state.online) {
            val paths = (
                snapshot?.db?.tasks?.values?.flatMap { it.attachments }.orEmpty() +
                    snapshot?.db?.projects?.values?.flatMap { it.attachments }.orEmpty()
                ).filter { it.startsWith("/") }.distinct()
            if (paths.any { it !in state.savedAttachments }) {
                repository.saveAllAttachments()
            }
        }
    }
    val children = remember(snapshot, preferences.keepCompletedSubtasks) {
        val db = snapshot?.db
        if (db == null) {
            emptyMap()
        } else {
            val views = db.taskViews(includeCompleted = preferences.keepCompletedSubtasks)
                .filter { it.parent != null }
            val byId = views.associateBy { it.id }
            db.tasks.values.associate { dto ->
                val ordered = dto.childOrder.mapNotNull { id ->
                    byId[id]?.takeIf { view -> view.parent == dto.id }
                }
                val seen = ordered.map { it.id }.toSet()
                val extras = views.filter { it.parent == dto.id && it.id !in seen }
                dto.id to (ordered + extras)
            }.filterValues { it.isNotEmpty() }
        }
    }
    LaunchedEffect(tab, moreView, savedPerspective) {
        selecting = false
        selectedIds = emptySet()
    }
    val toasts = remember { Toasts(scope) }
    val snackbarHost = toasts.host
    LaunchedEffect(shared) {
        val payload = shared ?: return@LaunchedEffect
        when (payload) {
            is SharedPayload.Text -> {
                captureInitial = payload.text
                captureAttachment = null
                captureAttachmentLabel = null
                capture = true
            }
            is SharedPayload.Link -> sharePrompt = payload
            is SharedPayload.File -> sharePrompt = payload
        }
        onSharedConsumed()
    }
    val attachmentPicker = rememberLauncherForActivityResult(
        ActivityResultContracts.GetContent(),
    ) { uri ->
        val current = detail
        if (uri != null && current != null) {
            scope.launch {
                val uploaded = repository.uploadAttachment(current.id, uri)
                toasts.show(
                    message = if (uploaded) "Attachment added" else "Couldn't add the attachment",
                    duration = SnackbarDuration.Short,
                )
            }
        }
    }
    LaunchedEffect(state.error, state.online) {
        val message = state.error
        if (message != null && state.online) {
            toasts.show(message, duration = SnackbarDuration.Short)
            repository.clearError()
        }
    }
    val bottomPerspectives = remember(preferences.bottomBar) {
        resolveBottomBar(preferences.bottomBar)
    }
    val drawerOrder = remember(preferences.perspectiveOrder) {
        orderedPerspectives(preferences.perspectiveOrder)
    }
    val taskDrag = remember { TaskDragController() }
    taskDrag.onDrop = { task, drop ->
        val db = state.snapshot?.db
        if (db != null && task.id != drop.id && !isTaskDescendant(db, task.id, drop.id)) {
            when (drop.kind) {
                TaskDropKind.Nest -> {
                    taskDrag.revealed[drop.id] = true
                    scope.launch {
                        repository.moveTask(
                            task.id,
                            parentPlacement(drop.id, JsonPrimitive("end")),
                        )
                    }
                }
                TaskDropKind.RootAfter -> {
                    val target = db.tasks[drop.id]
                    val position = buildJsonObject { put("after", drop.id) }
                    val placement = if (target?.project != null) {
                        projectPlacement(target.project, position)
                    } else {
                        inboxPlacement(position)
                    }
                    scope.launch { repository.moveTask(task.id, placement) }
                }
            }
        }
    }
    val openPerspective: (Perspective) -> Unit = { item ->
        savedPerspective = null
        val targetTab = item.tab
        if (targetTab != null) {
            tab = targetTab
            moreView = null
        } else {
            moreView = item.more
        }
    }

    val complete: (TaskView, Boolean) -> Unit = { task, done ->
        if (preferences.haptics) {
            haptics.performHapticFeedback(HapticFeedbackType.LongPress)
        }
        scope.launch {
            repository.complete(task.id, done)
            toasts.show(
                message = if (done) "Completed “${task.title}”" else "Reopened “${task.title}”",
                actionLabel = "Undo",
                duration = SnackbarDuration.Long,
                onAction = { repository.complete(task.id, !done) },
            )
        }
    }
    val trash: (TaskView) -> Unit = { task ->
        scope.launch {
            val removed = repository.delete(task.id)
            toasts.show(
                message = "Moved “${task.title}” to Trash",
                actionLabel = "Undo",
                duration = SnackbarDuration.Long,
                onAction = { if (removed != null) repository.restore(removed) },
            )
        }
    }

    val closeSelection: () -> Unit = {
        selecting = false
        selectedIds = emptySet()
    }
    val selectAllTrash: () -> Unit = {
        selectedIds = snapshot?.db?.trashedTasks()?.map { it.id }?.toSet().orEmpty()
    }
    val bulkComplete: () -> Unit = {
        val ids = selectedIds.toList()
        if (ids.isNotEmpty()) {
            val reopen = moreView == MoreView.Completed
            closeSelection()
            scope.launch {
                repository.completeMany(ids, !reopen)
                toasts.show(
                    message = if (reopen) {
                        "Reopened ${taskCount(ids.size)}"
                    } else {
                        "Completed ${taskCount(ids.size)}"
                    },
                    actionLabel = "Undo",
                    duration = SnackbarDuration.Long,
                    onAction = { repository.completeMany(ids, reopen) },
                )
            }
        }
    }
    val bulkTrash: () -> Unit = {
        val ids = selectedIds.toList()
        if (ids.isNotEmpty()) {
            closeSelection()
            scope.launch {
                val removed = repository.deleteMany(ids)
                toasts.show(
                    message = "Moved ${taskCount(ids.size)} to Trash",
                    actionLabel = "Undo",
                    duration = SnackbarDuration.Long,
                    onAction = { repository.restoreMany(removed) },
                )
            }
        }
    }
    val bulkPurge: () -> Unit = {
        val ids = selectedIds.toList()
        if (ids.isNotEmpty()) {
            confirm = ConfirmRequest(
                title = if (ids.size == 1) {
                    "Delete task forever?"
                } else {
                    "Delete ${ids.size} tasks forever?"
                },
                body = "This cannot be undone.",
                confirmLabel = "Delete forever",
            ) {
                closeSelection()
                scope.launch {
                    repository.purgeMany(ids)
                    toasts.show(
                        message = "Deleted ${taskCount(ids.size)}",
                        duration = SnackbarDuration.Short,
                    )
                }
            }
        }
    }

    BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
    // A tablet has room for a rail; landscape phones keep the bottom bar.
    val wide = maxWidth >= 720.dp && maxHeight >= 560.dp
    LaunchedEffect(wide) {
        if (wide) drawer.close()
    }
    ModalNavigationDrawer(
        drawerState = drawer,
        gesturesEnabled = !wide,
        drawerContent = {
            ModalDrawerSheet(
                modifier = Modifier.width(310.dp).fillMaxHeight(),
                drawerShape = androidx.compose.ui.graphics.RectangleShape,
                drawerContainerColor = colors.background,
            ) {
                LibraryDrawer(
                    open = drawer.currentValue == DrawerValue.Open,
                    tab = tab,
                    more = moreView,
                    bottom = bottomPerspectives,
                    order = drawerOrder,
                    onReorder = { moving, from, to ->
                        val full = orderedPerspectives(preferences.perspectiveOrder)
                            .map { it.key }
                        onPreferencesChange(
                            preferences.copy(
                                perspectiveOrder = reorderPerspectiveKeys(full, moving, from, to),
                            ),
                        )
                    },
                    saved = snapshot?.db?.perspectives?.values
                        ?.sortedBy { it.position }
                        .orEmpty(),
                    state = state,
                    canUndo = state.canUndo && state.online,
                    canRedo = state.canRedo && state.online,
                    undoLabel = state.undoLabel,
                    redoLabel = state.redoLabel,
                    onUndo = { scope.launch { repository.undo() } },
                    onRedo = { scope.launch { repository.redo() } },
                    onPerspective = { item ->
                        val targetTab = item.tab
                        if (targetTab != null) {
                            tab = targetTab
                            moreView = null
                        } else {
                            moreView = item.more
                        }
                        savedPerspective = null
                        scope.launch { drawer.close() }
                    },
                    onSaved = { item ->
                        moreView = null
                        savedPerspective = item
                        scope.launch { drawer.close() }
                    },
                    onNewPerspective = {
                        perspectiveEditor = PerspectiveEditorTarget()
                        scope.launch { drawer.close() }
                    },
                    onEditPerspective = { item ->
                        perspectiveEditor = PerspectiveEditorTarget(item)
                        scope.launch { drawer.close() }
                    },
                    onDeletePerspective = { item ->
                        confirm = ConfirmRequest(
                            title = "Delete perspective?",
                            body = "“${item.name}” is removed from the drawer.",
                            confirmLabel = "Delete",
                        ) {
                            if (savedPerspective?.id == item.id) savedPerspective = null
                            scope.launch { repository.deletePerspective(item) }
                        }
                    },
                )
            }
        },
    ) {
    Scaffold(
        containerColor = colors.background,
        snackbarHost = { FlowSnackbarHost(snackbarHost) },
        bottomBar = {
            if (selecting) {
                SelectionBar(
                    count = selectedIds.size,
                    trashMode = moreView == MoreView.Trash,
                    reopenMode = moreView == MoreView.Completed,
                    canSelectAll = moreView == MoreView.Trash,
                    onClose = closeSelection,
                    onSelectAll = selectAllTrash,
                    onComplete = bulkComplete,
                    onTrash = bulkTrash,
                    onPurge = bulkPurge,
                )
            } else if (!wide) {
            NavigationBar(
                containerColor = colors.background,
                tonalElevation = 0.dp,
            ) {
                bottomPerspectives.forEach { item ->
                    val selected = savedPerspective == null && (
                        (item.tab != null && moreView == null && tab == item.tab) ||
                            (item.more != null && moreView == item.more)
                        )
                    NavigationBarItem(
                        selected = selected,
                        onClick = { openPerspective(item) },
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
                NavigationBarItem(
                    selected = tab == Tab.Settings && moreView == null && savedPerspective == null,
                    onClick = {
                        tab = Tab.Settings
                        moreView = null
                        savedPerspective = null
                    },
                    icon = {
                        Icon(
                            imageVector = Icons.Outlined.Settings,
                            contentDescription = "Settings",
                            modifier = Modifier.size(21.dp),
                        )
                    },
                    label = {
                        Text("Settings", style = MaterialTheme.typography.labelSmall)
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
            if (tab != Tab.Settings) {
                FloatingActionButton(
                    onClick = {
                    if (preferences.haptics) {
                        haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                    }
                    captureInitial = ""
                    captureAttachment = null
                    captureAttachmentLabel = null
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
            }
        },
    ) { padding ->
        val selectionState = if (selecting) {
            TaskSelection(selectedIds) { id ->
                selectedIds = if (id in selectedIds) selectedIds - id else selectedIds + id
            }
        } else {
            null
        }
        CompositionLocalProvider(
            LocalTaskDrag provides taskDrag,
            LocalTaskSelection provides selectionState,
        ) {
        Row(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .background(colors.background),
        ) {
            if (wide) {
                TabletRail(
                    tab = tab,
                    more = moreView,
                    savedPerspective = savedPerspective,
                    perspectives = bottomPerspectives,
                    onPerspective = { item -> openPerspective(item) },
                    onSettings = {
                        tab = Tab.Settings
                        moreView = null
                        savedPerspective = null
                    },
                )
                Box(
                    modifier = Modifier
                        .width(1.dp)
                        .fillMaxHeight()
                        .background(colors.hairline),
                )
            }
            Column(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .background(colors.background),
            ) {
            HorizontalDivider(thickness = 1.dp, color = colors.background)
            val activeMore = moreView
            val activeSaved = savedPerspective
            val canSelect = activeSaved != null ||
                activeMore == MoreView.Flagged ||
                activeMore == MoreView.Completed ||
                activeMore == MoreView.Trash ||
                activeMore == MoreView.Changed ||
                activeMore == MoreView.Priorities ||
                (activeMore == null &&
                    (tab == Tab.Inbox || tab == Tab.Today || tab == Tab.Forecast))
            Header(
                title = activeSaved?.name ?: activeMore?.label ?: tab.label,
                subtitle = when {
                    activeSaved != null -> "Saved perspective"
                    else -> headerSubtitle(activeMore, tab, state)
                },
                onMenu = { scope.launch { drawer.open() } },
                onSearch = { search = true },
                onSelect = if (canSelect) {
                    {
                        selectedIds = emptySet()
                        selecting = true
                    }
                } else {
                    null
                },
            )
            if (offlineWarning) {
                OfflineNotice(onRetry = { scope.launch { repository.sync() } })
            }
            if (activeSaved != null) {
                SavedPerspectiveView(
                    perspective = activeSaved,
                    snapshot = snapshot,
                    swipeActions = preferences.swipeActions,
                    pending = state.pending,
                    onToggle = { task -> complete(task, !task.completed) },
                    onClick = { detail = it },
                    onTrash = trash,
                )
            } else if (activeMore != null) {
                val db = snapshot?.db
                if (db != null) {
                    MoreDestination(
                        view = activeMore,
                        db = db,
                        pending = state.pending,
                        onTask = { task -> detail = task },
                        onToggle = { task -> complete(task, !task.completed) },
                        onRestore = { id ->
                            db.tasks[id]?.let { dto -> scope.launch { repository.restore(dto) } }
                        },
                        onPurge = { id -> scope.launch { repository.purge(id) } },
                        onReview = { projectItem ->
                            scope.launch { repository.reviewProject(projectItem) }
                        },
                        onProject = { projectItem -> project = projectItem },
                        onNewTag = { tagEditor = TagEditorTarget() },
                        onEditTag = { item -> tagEditor = TagEditorTarget(item, item.parent) },
                        onDeleteTag = { item ->
                            confirm = ConfirmRequest(
                                title = "Delete tag?",
                                body = "“#${item.name}” is removed from every task and project.",
                                confirmLabel = "Delete",
                            ) {
                                scope.launch { repository.deleteTag(item) }
                            }
                        },
                        onOpenTag = { item -> tagTasks = item },
                    )
                }
            } else when (tab) {
                Tab.Inbox -> InboxList(
                    swipeActions = preferences.swipeActions,
                    children = children,
                    tasks = snapshot?.db?.inbox().orEmpty(),
                    pending = state.pending,
                    onToggle = { task -> complete(task, !task.completed) },
                    onClick = { detail = it },
                    onTrash = trash,
                )
                Tab.Today -> TodayList(
                    swipeActions = preferences.swipeActions,
                    children = children,
                    tasks = snapshot?.db?.today().orEmpty(),
                    pending = state.pending,
                    onToggle = { task -> complete(task, !task.completed) },
                    onClick = { detail = it },
                    onTrash = trash,
                )
                Tab.Forecast -> ForecastScreen(
                    view = preferences.forecastView,
                    onViewChange = { onPreferencesChange(preferences.copy(forecastView = it)) },
                    snapshot = snapshot,
                    weekStartsMonday = preferences.weekStartsMonday,
                    swipeActions = preferences.swipeActions,
                    children = children,
                    pending = state.pending,
                    onToggle = { task -> complete(task, !task.completed) },
                    onClick = { detail = it },
                    onTrash = trash,
                )
                Tab.Browse -> BrowseList(
                    snapshot = snapshot,
                    onOpen = { project = it },
                    onNewProject = { folder -> projectEditor = ProjectEditorTarget(null, folder) },
                    onNewFolder = { parent -> folderEditor = FolderEditorTarget(null, parent) },
                    onEditProject = { projectEditor = ProjectEditorTarget(it, it.folder) },
                    onArchiveProject = { item ->
                        scope.launch {
                            repository.setProjectStatus(item, "done")
                            toasts.show(
                                message = "Archived “${item.name}”",
                                duration = SnackbarDuration.Short,
                            )
                        }
                    },
                    onDeleteProject = { item ->
                        confirm = ConfirmRequest(
                            title = "Delete project?",
                            body = "“${item.name}” and its tasks move to Trash. You can restore them from Trash.",
                            confirmLabel = "Delete",
                        ) {
                            scope.launch { repository.deleteProject(item) }
                        }
                    },
                    onEditFolder = { folderEditor = FolderEditorTarget(it, it.parent) },
                    onDeleteFolder = { item ->
                        confirm = ConfirmRequest(
                            title = "Delete folder?",
                            body = "“${item.name}”, anything inside it, and its tasks move to Trash.",
                            confirmLabel = "Delete",
                        ) {
                            scope.launch { repository.deleteFolder(item) }
                        }
                    },
                )
                Tab.Settings -> SettingsScreen(
                    state = state,
                    theme = theme,
                    onThemeChange = onThemeChange,
                    preferences = preferences,
                    onPreferencesChange = onPreferencesChange,
                    librarySettings = snapshot?.db?.settings ?: dev.flowlist.android.data.SettingsDto(),
                    onLibrarySettings = { updated ->
                        scope.launch { repository.updateLibrarySettings(updated) }
                    },
                    onSync = { scope.launch { repository.sync() } },
                    onSignOut = { scope.launch { repository.signOut() } },
                    onSaveAllPreference = { enabled ->
                        onPreferencesChange(preferences.copy(saveAllAttachments = enabled))
                    },
                    onRemoveAllSaved = {
                        onPreferencesChange(preferences.copy(saveAllAttachments = false))
                        scope.launch {
                            repository.removeAllSavedAttachments()
                            toasts.show(
                                message = "Offline attachment copies removed",
                                duration = SnackbarDuration.Short,
                            )
                        }
                    },
                )
            }
        }
    }
    }
    }
    }

    previewTarget?.let { target ->
        AttachmentPreviewSheet(
            target = target,
            onOpen = {
                scope.launch { repository.openAttachment(target.path) }
            },
            onDismiss = { previewTarget = null },
        )
    }

    sharePrompt?.let { payload ->
        val label = when (payload) {
            is SharedPayload.Link -> payload.title?.takeIf { it.isNotBlank() } ?: payload.url
            is SharedPayload.File -> payload.name ?: "Shared file"
            is SharedPayload.Text -> payload.text
        }
        if (sharePicking) {
            ShareTaskPickerSheet(
                label = label,
                tasks = snapshot?.db?.taskViews().orEmpty(),
                onDismiss = {
                    sharePicking = false
                    sharePrompt = null
                },
                onPick = { task ->
                    sharePicking = false
                    sharePrompt = null
                    when (payload) {
                        is SharedPayload.Link -> scope.launch {
                            val dto = snapshot?.db?.tasks?.get(task.id)
                            if (dto != null) {
                                repository.addAttachmentLink(dto, payload.url, payload.title.orEmpty())
                                toasts.show(
                                    message = "Added link to “${task.title}”",
                                    duration = SnackbarDuration.Short,
                                )
                            }
                        }
                        is SharedPayload.File -> scope.launch {
                            toasts.show(
                                message = "Uploading “${payload.name ?: "file"}”…",
                                duration = SnackbarDuration.Short,
                            )
                            val path = repository.uploadSharedFile(payload.uri)
                            if (path == null) {
                                toasts.show(
                                    message = "Couldn't attach that file",
                                    duration = SnackbarDuration.Short,
                                )
                            } else {
                                repository.addAttachment(task.id, path)
                                toasts.show(
                                    message = "Attached to “${task.title}”",
                                    duration = SnackbarDuration.Short,
                                )
                            }
                        }
                        is SharedPayload.Text -> Unit
                    }
                },
            )
        } else {
            ShareChoiceSheet(
                label = label,
                onNewTask = {
                    sharePrompt = null
                    when (payload) {
                        is SharedPayload.Link -> {
                            captureInitial = payload.title?.takeIf { it.isNotBlank() }
                                ?: android.net.Uri.parse(payload.url).host
                                ?: payload.url
                            captureAttachment = linkWithName(payload.url, payload.title.orEmpty())
                            captureAttachmentLabel =
                                payload.title?.takeIf { it.isNotBlank() } ?: payload.url
                            capture = true
                        }
                        is SharedPayload.File -> scope.launch {
                            val title = payload.name
                                ?.substringBeforeLast('.', payload.name)
                                ?.takeIf { it.isNotBlank() }
                                ?: "Shared file"
                            val path = repository.uploadSharedFile(payload.uri)
                            if (path == null) {
                                toasts.show(
                                    message = "Couldn't attach that file",
                                    duration = SnackbarDuration.Short,
                                )
                            } else {
                                captureInitial = title
                                captureAttachment = path
                                captureAttachmentLabel = payload.name ?: path.substringAfterLast('/')
                                capture = true
                            }
                        }
                        is SharedPayload.Text -> Unit
                    }
                },
                onExisting = { sharePicking = true },
                onDismiss = { sharePrompt = null },
            )
        }
    }

    detail?.let { taskView ->
        val db = snapshot?.db
        val dto = db?.tasks?.get(taskView.id)
        if (dto != null) {
            TaskEditSheet(
                task = dto,
                projects = db.projects.values.toList(),
                tags = db.tags.values.toList(),
                subtasks = db.orderedChildTasks(taskView.id),
                moveTargets = db.tasks.values
                    .filter { it.isActive && it.parent == null && it.id != taskView.id }
                    .sortedBy { it.title.lowercase() },
                pending = taskView.id in state.pending,
                weekStartsMonday = preferences.weekStartsMonday,
                defaultTimeMinutes = preferences.defaultTimeMinutes,
                onDismiss = { detail = null },
                onSave = { edit ->
                    detail = null
                    scope.launch {
                        repository.update(dto, edit)
                        toasts.show(
                            message = "Saved “${edit.title}”",
                            duration = SnackbarDuration.Short,
                        )
                    }
                },
                savedAttachments = state.savedAttachments,
                onCreateProject = { name ->
                    scope.launch { repository.createProject(name, null) }
                },
                onCreateTag = { name ->
                    scope.launch { repository.createTag(name, null) }
                },
                onAddAttachment = { attachmentPicker.launch("*/*") },
                onSaveAttachment = { path ->
                    scope.launch {
                        val saved = repository.saveAttachmentForOffline(path)
                        toasts.show(
                            message = if (saved) {
                                "Saved for offline"
                            } else {
                                "Couldn't save the attachment"
                            },
                            duration = SnackbarDuration.Short,
                        )
                    }
                },
                onRemoveSavedAttachment = { path ->
                    scope.launch { repository.removeSavedAttachment(path) }
                },
                onOpenAttachment = { path ->
                    scope.launch {
                        if (!repository.openAttachment(path)) {
                            toasts.show(
                                message = "Couldn't open that attachment",
                                duration = SnackbarDuration.Short,
                            )
                        }
                    }
                },
                onPreviewAttachment = { path ->
                    scope.launch {
                        val file = repository.attachmentFile(path)
                        if (file == null) {
                            toasts.show(
                                message = "Couldn't load that attachment",
                                duration = SnackbarDuration.Short,
                            )
                        } else {
                            previewTarget = PreviewTarget(path, file)
                        }
                    }
                },
                onRemoveAttachment = { path ->
                    scope.launch { repository.removeAttachment(dto.id, path) }
                },
                onAddSubtask = { title ->
                    scope.launch { repository.addSubtask(dto, title) }
                },
                onToggleSubtask = { child, completed ->
                    scope.launch { repository.complete(child.id, completed) }
                },
                onMoveSubtask = { child, parent ->
                    scope.launch { repository.moveSubtask(child, parent) }
                },
                onDeleteSubtask = { child ->
                    scope.launch {
                        val removed = repository.delete(child.id)
                        toasts.show(
                            message = "Moved “${child.title}” to Trash",
                            actionLabel = "Undo",
                            duration = SnackbarDuration.Long,
                            onAction = { if (removed != null) repository.restore(removed) },
                        )
                    }
                },
                onReorderSubtasks = { ordered ->
                    scope.launch { repository.reorderSubtasks(dto.id, ordered) }
                },
                onAddLink = { url, name ->
                    scope.launch { repository.addAttachmentLink(dto, url, name) }
                },
                onRenameLink = { original, url, name ->
                    scope.launch { repository.renameAttachmentLink(dto, original, url, name) }
                },
                onChecklist = { item, completed ->
                    scope.launch { repository.toggleChecklist(dto, item.id, completed) }
                },
                onAddChecklist = { title ->
                    scope.launch { repository.addChecklist(dto, title) }
                },
                onDeleteChecklist = { item ->
                    scope.launch { repository.deleteChecklist(dto, item) }
                },
                onMoveChecklist = { item, to ->
                    scope.launch { repository.moveChecklist(dto, item, to) }
                },
                onDeleteSeries = {
                    detail = null
                    scope.launch { repository.deleteSeries(dto) }
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
                        blockedByTag = db?.tagBlocking(task)?.name,
                    )
                }
                .orEmpty(),
            onDismiss = { project = null },
            onEdit = {
                val editing = value
                project = null
                projectEditor = ProjectEditorTarget(editing, editing.folder)
            },
            onToggle = { task -> complete(task, !task.completed) },
            onClick = { task ->
                project = null
                detail = task
            },
        )
    }

    projectEditor?.let { target ->
        val editing = target.project
        ProjectEditorSheet(
            target = target,
            folders = snapshot?.db?.folders?.values?.toList().orEmpty(),
            defaultReviewDays = snapshot?.db?.settings?.defaultReviewDays ?: 7,
            onDismiss = { projectEditor = null },
            onSave = { name, folder, status, kind, notes, reviewDays, icon ->
                projectEditor = null
                scope.launch {
                    if (editing == null) {
                        repository.createProject(name, folder, kind, notes, reviewDays, icon)
                    } else {
                        repository.updateProject(
                            editing,
                            name,
                            folder,
                            status,
                            kind,
                            notes,
                            reviewDays,
                            icon,
                        )
                    }
                }
            },
            onDelete = editing?.let { item ->
                {
                    projectEditor = null
                    confirm = ConfirmRequest(
                        title = "Delete project?",
                        body = "“${item.name}” and its tasks move to Trash.",
                        confirmLabel = "Delete",
                    ) {
                        scope.launch { repository.deleteProject(item) }
                    }
                }
            },
        )
    }

    folderEditor?.let { target ->
        val editing = target.folder
        FolderEditorDialog(
            target = target,
            folders = snapshot?.db?.folders?.values?.toList().orEmpty(),
            onDismiss = { folderEditor = null },
            onSave = { name, parent ->
                folderEditor = null
                scope.launch {
                    if (editing == null) {
                        repository.createFolder(name, parent)
                    } else {
                        repository.updateFolder(editing, name, parent)
                    }
                }
            },
            onDelete = editing?.let { item ->
                {
                    folderEditor = null
                    confirm = ConfirmRequest(
                        title = "Delete folder?",
                        body = "“${item.name}”, anything inside it, and its tasks move to Trash.",
                        confirmLabel = "Delete",
                    ) {
                        scope.launch { repository.deleteFolder(item) }
                    }
                }
            },
        )
    }

    tagEditor?.let { target ->
        val editing = target.tag
        TagEditorDialog(
            target = target,
            tags = snapshot?.db?.tags?.values?.toList().orEmpty(),
            onDismiss = { tagEditor = null },
            onSave = { name, parent, location, status ->
                tagEditor = null
                scope.launch {
                    if (editing == null) {
                        repository.createTag(name, parent, location, status)
                    } else {
                        repository.updateTag(editing, name, parent, location, status)
                    }
                }
            },
            onDelete = editing?.let { item ->
                {
                    tagEditor = null
                    confirm = ConfirmRequest(
                        title = "Delete tag?",
                        body = "“#${item.name}” is removed from every task and project.",
                        confirmLabel = "Delete",
                    ) {
                        scope.launch { repository.deleteTag(item) }
                    }
                }
            },
        )
    }

    perspectiveEditor?.let { target ->
        val editing = target.perspective
        PerspectiveEditorSheet(
            target = target,
            projects = snapshot?.db?.projects?.values?.toList().orEmpty(),
            tags = snapshot?.db?.tags?.values?.toList().orEmpty(),
            onDismiss = { perspectiveEditor = null },
            onSave = { draft ->
                perspectiveEditor = null
                scope.launch {
                    repository.savePerspective(
                        id = editing?.id.orEmpty(),
                        name = draft.name,
                        filter = draft.toFilter(),
                        groupBy = draft.group,
                        sort = draft.sort,
                        showCompleted = draft.showCompleted,
                    )
                }
            },
            onDelete = editing?.let { item ->
                {
                    perspectiveEditor = null
                    confirm = ConfirmRequest(
                        title = "Delete perspective?",
                        body = "“${item.name}” is removed from the drawer.",
                        confirmLabel = "Delete",
                    ) {
                        if (savedPerspective?.id == item.id) savedPerspective = null
                        scope.launch { repository.deletePerspective(item) }
                    }
                }
            },
        )
    }

    confirm?.let { request ->
        ConfirmDialog(
            title = request.title,
            body = request.body,
            confirmLabel = request.confirmLabel,
            onConfirm = {
                confirm = null
                request.action()
            },
            onDismiss = { confirm = null },
        )
    }

    if (search) {
        val searchDb = snapshot?.db
        val viewTasks = when {
            searchDb == null -> null
            savedPerspective != null -> null
            moreView == MoreView.Flagged -> searchDb.flaggedTasks()
            moreView == MoreView.Completed -> searchDb.completedTasks()
            moreView == MoreView.Changed -> searchDb.changedTasks()
            moreView == MoreView.Attachments -> searchDb.attachmentTasks()
            moreView == MoreView.Trash -> searchDb.trashedTasks()
            moreView == MoreView.Priorities -> searchDb.priorityGroups().flatMap { it.second }
            moreView != null -> null
            tab == Tab.Inbox -> searchDb.inbox()
            tab == Tab.Today -> searchDb.today()
            tab == Tab.Forecast -> searchDb.forecast().values.flatten()
            else -> null
        }
        SearchSheet(
            db = searchDb ?: dev.flowlist.android.data.Db(),
            pending = state.pending,
            viewTaskIds = viewTasks?.map { it.id }?.toSet(),
            viewName = when {
                savedPerspective != null -> savedPerspective?.name
                moreView != null -> moreView?.label
                else -> tab.label
            },
            onDismiss = { search = false },
            onToggle = { task -> complete(task, !task.completed) },
            onClick = { task ->
                search = false
                detail = task
            },
            onProject = { projectItem ->
                search = false
                project = projectItem
                tab = Tab.Browse
            },
            onFolder = {
                search = false
                tab = Tab.Browse
            },
            onTag = { tag ->
                search = false
                tagTasks = tag
            },
            onPerspective = { perspective ->
                search = false
                savedPerspective = perspective
                moreView = null
            },
            onBulkComplete = { ids ->
                search = false
                scope.launch {
                    repository.completeMany(ids, true)
                    toasts.show(
                        message = "Completed ${taskCount(ids.size)}",
                        actionLabel = "Undo",
                        duration = SnackbarDuration.Long,
                        onAction = { repository.completeMany(ids, false) },
                    )
                }
            },
            onBulkTrash = { ids ->
                search = false
                scope.launch {
                    val removed = repository.deleteMany(ids)
                    toasts.show(
                        message = "Moved ${taskCount(ids.size)} to Trash",
                        actionLabel = "Undo",
                        duration = SnackbarDuration.Long,
                        onAction = { repository.restoreMany(removed) },
                    )
                }
            },
        )
    }

    tagTasks?.let { tag ->
        TagTasksSheet(
            tag = tag,
            tasks = snapshot?.db?.tasksForTag(tag.id).orEmpty(),
            pending = state.pending,
            onToggle = { task -> complete(task, !task.completed) },
            onClick = { task ->
                tagTasks = null
                detail = task
            },
            onDismiss = { tagTasks = null },
        )
    }



    if (capture) {
        CaptureSheet(
            initialText = captureInitial,
            attachmentLabel = captureAttachmentLabel,
            onDismiss = {
                capture = false
                captureAttachment = null
                captureAttachmentLabel = null
            },
            onAdd = { text ->
                capture = false
                val attachment = captureAttachment
                captureAttachment = null
                captureAttachmentLabel = null
                if (preferences.haptics) {
                    haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                }
                scope.launch {
                    if (attachment == null) {
                        repository.capture(text)
                    } else {
                        val task = repository.captureShared(text, listOf(attachment))
                        if (task != null) {
                            toasts.show(
                                message = "Added “${task.title}” with its attachment",
                                duration = SnackbarDuration.Short,
                            )
                        }
                    }
                }
            },
        )
    }
}

@Composable
private fun SelectionBar(
    count: Int,
    trashMode: Boolean,
    reopenMode: Boolean,
    canSelectAll: Boolean,
    onClose: () -> Unit,
    onSelectAll: () -> Unit,
    onComplete: () -> Unit,
    onTrash: () -> Unit,
    onPurge: () -> Unit,
) {
    val colors = flow
    Surface(color = colors.background, tonalElevation = 0.dp) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(horizontal = 8.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onClose) {
                Icon(Icons.Rounded.Close, contentDescription = "Cancel selection", tint = colors.ink)
            }
            Text(
                text = if (count == 1) "1 selected" else "$count selected",
                style = MaterialTheme.typography.titleSmall,
                color = colors.ink,
                modifier = Modifier.weight(1f),
            )
            if (canSelectAll) {
                TextButton(onClick = onSelectAll) {
                    Text("Select all", color = colors.accent)
                }
            }
            if (trashMode) {
                IconButton(onClick = onPurge, enabled = count > 0) {
                    Icon(
                        imageVector = Icons.Rounded.DeleteForever,
                        contentDescription = "Delete forever",
                        tint = if (count > 0) colors.danger else colors.faint,
                    )
                }
            } else {
                IconButton(onClick = onComplete, enabled = count > 0) {
                    Icon(
                        imageVector = if (reopenMode) Icons.Rounded.Undo else Icons.Rounded.CheckCircle,
                        contentDescription = if (reopenMode) "Reopen" else "Complete",
                        tint = if (count > 0) colors.success else colors.faint,
                    )
                }
                IconButton(onClick = onTrash, enabled = count > 0) {
                    Icon(
                        imageVector = Icons.Rounded.Delete,
                        contentDescription = "Trash",
                        tint = if (count > 0) colors.danger else colors.faint,
                    )
                }
            }
        }
    }
}

@Composable
private fun TabletRail(
    tab: Tab,
    more: MoreView?,
    savedPerspective: PerspectiveDto?,
    perspectives: List<Perspective>,
    onPerspective: (Perspective) -> Unit,
    onSettings: () -> Unit,
) {
    val colors = flow
    NavigationRail(
        modifier = Modifier.fillMaxHeight(),
        containerColor = colors.background,
    ) {
        Spacer(modifier = Modifier.height(8.dp))
        perspectives.forEach { item ->
            val selected = savedPerspective == null && (
                (item.tab != null && more == null && tab == item.tab) ||
                    (item.more != null && more == item.more)
                )
            NavigationRailItem(
                selected = selected,
                onClick = { onPerspective(item) },
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
                alwaysShowLabel = true,
                colors = NavigationRailItemDefaults.colors(
                    selectedIconColor = colors.accent,
                    selectedTextColor = colors.accent,
                    unselectedIconColor = colors.faint,
                    unselectedTextColor = colors.faint,
                    indicatorColor = Color.Transparent,
                ),
            )
        }
        Spacer(modifier = Modifier.weight(1f))
        NavigationRailItem(
            selected = tab == Tab.Settings && more == null && savedPerspective == null,
            onClick = onSettings,
            icon = {
                Icon(
                    imageVector = Icons.Outlined.Settings,
                    contentDescription = "Settings",
                    modifier = Modifier.size(21.dp),
                )
            },
            label = {
                Text("Settings", style = MaterialTheme.typography.labelSmall)
            },
            alwaysShowLabel = true,
            colors = NavigationRailItemDefaults.colors(
                selectedIconColor = colors.accent,
                selectedTextColor = colors.accent,
                unselectedIconColor = colors.faint,
                unselectedTextColor = colors.faint,
                indicatorColor = Color.Transparent,
            ),
        )
        Spacer(modifier = Modifier.height(8.dp))
    }
}

@Composable
private fun LibraryDrawer(
    open: Boolean,
    tab: Tab,
    more: MoreView?,
    bottom: List<Perspective>,
    order: List<Perspective>,
    onReorder: (List<String>, Int, Int) -> Unit,
    saved: List<PerspectiveDto>,
    state: AppState,
    canUndo: Boolean,
    canRedo: Boolean,
    undoLabel: String?,
    redoLabel: String?,
    onUndo: () -> Unit,
    onRedo: () -> Unit,
    onPerspective: (Perspective) -> Unit,
    onSaved: (PerspectiveDto) -> Unit,
    onNewPerspective: () -> Unit,
    onEditPerspective: (PerspectiveDto) -> Unit,
    onDeletePerspective: (PerspectiveDto) -> Unit,
) {
    val colors = flow
    val remaining = order.filterNot { it in bottom }
    var draggingKey by remember { mutableStateOf<String?>(null) }
    var dragFrom by remember { mutableIntStateOf(-1) }
    var dragTo by remember { mutableIntStateOf(-1) }
    var dragOffset by remember { mutableFloatStateOf(0f) }
    var settleFrom by remember { mutableFloatStateOf(0f) }
    var settleTo by remember { mutableFloatStateOf(0f) }
    var settling by remember { mutableStateOf(false) }
    // History stays folded away so a stray tap near the drawer's foot cannot
    // undo or redo a change. Keying on `open` folds it again every time the
    // drawer is reopened.
    var historyOpen by remember(open) { mutableStateOf(false) }
    var measuredRowHeight by remember { mutableFloatStateOf(0f) }
    val fallbackRowHeight = with(LocalDensity.current) { 42.dp.toPx() }
    val rowHeight = if (measuredRowHeight > 0f) measuredRowHeight else fallbackRowHeight
    val haptics = LocalHapticFeedback.current
    val scope = rememberCoroutineScope()
    val settleAnim = remember { Animatable(0f) }
    Column(
        modifier = Modifier
            .fillMaxHeight()
            .statusBarsPadding()
            .navigationBarsPadding()
            .padding(horizontal = 10.dp, vertical = 12.dp),
    ) {
        Text(
            text = "Perspectives",
            style = MaterialTheme.typography.titleMedium,
            color = colors.ink,
            modifier = Modifier.padding(start = 12.dp, top = 6.dp, bottom = 12.dp),
        )
        remaining.forEachIndexed { index, item ->
            key(item.key) {
                val selected = if (item.tab != null) {
                    more == null && tab == item.tab
                } else {
                    more == item.more
                }
                val dragging = draggingKey == item.key
                val settlingThis = dragging && settling
                // Rows between the drag origin and the live target step aside, so
                // a gap opens exactly where the row will land.
                val shift = when {
                    dragging -> 0f
                    dragFrom >= 0 && dragTo > dragFrom && index in (dragFrom + 1)..dragTo -> -rowHeight
                    dragFrom >= 0 && dragTo < dragFrom && index in dragTo until dragFrom -> rowHeight
                    else -> 0f
                }
                val animatedShift by animateFloatAsState(
                    targetValue = shift,
                    // The snap when the drag clears keeps the reordered rows from
                    // animating a second time as their base position changes.
                    animationSpec = if (dragFrom >= 0) {
                        spring(dampingRatio = 0.82f, stiffness = 700f)
                    } else {
                        snap()
                    },
                    label = "drawer-shift-$index",
                )
                val translation = when {
                    settlingThis -> settleFrom + (settleTo - settleFrom) * settleAnim.value
                    dragging -> dragOffset
                    else -> animatedShift
                }
                DrawerRow(
                    label = item.label,
                    icon = item.icon,
                    selected = selected,
                    dragging = dragging,
                    onClick = { onPerspective(item) },
                    modifier = Modifier
                        .onSizeChanged { size ->
                            if (measuredRowHeight <= 0f && size.height > 0) {
                                measuredRowHeight = size.height.toFloat()
                            }
                        }
                        .zIndex(if (dragging) 1f else 0f)
                        .graphicsLayer {
                            translationY = translation
                        }
                        .alpha(if (dragging) 0.95f else 1f)
                        .pointerInput(remaining, item.key, rowHeight) {
                            detectDragGesturesAfterLongPress(
                                onDragStart = {
                                    draggingKey = item.key
                                    dragFrom = index
                                    dragTo = index
                                    dragOffset = 0f
                                    settling = false
                                },
                                onDrag = { change, dragAmount ->
                                    change.consume()
                                    dragOffset += dragAmount.y
                                    if (dragFrom >= 0) {
                                        val next = (dragFrom + (dragOffset / rowHeight).roundToInt())
                                            .coerceIn(0, remaining.lastIndex)
                                        if (next != dragTo) {
                                            dragTo = next
                                            haptics.performHapticFeedback(
                                                HapticFeedbackType.TextHandleMove,
                                            )
                                        }
                                    }
                                },
                                onDragEnd = {
                                    val from = dragFrom
                                    val to = dragTo
                                    if (from < 0) {
                                        draggingKey = null
                                        return@detectDragGesturesAfterLongPress
                                    }
                                    settleFrom = dragOffset
                                    settleTo = if (to != from) (to - from) * rowHeight else 0f
                                    settling = true
                                    scope.launch {
                                        settleAnim.snapTo(0f)
                                        settleAnim.animateTo(
                                            1f,
                                            tween(
                                                durationMillis = 190,
                                                easing = FastOutSlowInEasing,
                                            ),
                                        )
                                        if (from >= 0 && to != from) {
                                            onReorder(remaining.map { it.key }, from, to)
                                        }
                                        draggingKey = null
                                        dragFrom = -1
                                        dragTo = -1
                                        dragOffset = 0f
                                        settleFrom = 0f
                                        settleTo = 0f
                                        settling = false
                                        settleAnim.snapTo(0f)
                                    }
                                },
                                onDragCancel = {
                                    if (dragFrom < 0) return@detectDragGesturesAfterLongPress
                                    settleFrom = dragOffset
                                    settleTo = 0f
                                    settling = true
                                    scope.launch {
                                        settleAnim.snapTo(0f)
                                        settleAnim.animateTo(
                                            1f,
                                            tween(
                                                durationMillis = 160,
                                                easing = FastOutSlowInEasing,
                                            ),
                                        )
                                        draggingKey = null
                                        dragFrom = -1
                                        dragTo = -1
                                        dragOffset = 0f
                                        settleFrom = 0f
                                        settleTo = 0f
                                        settling = false
                                        settleAnim.snapTo(0f)
                                    }
                                },
                            )
                        },
                )
            }
        }

        Spacer(modifier = Modifier.height(8.dp))
        HorizontalDivider(color = colors.hairline, modifier = Modifier.padding(horizontal = 8.dp))
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 12.dp, end = 4.dp, top = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "SAVED",
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.SemiBold,
                letterSpacing = 0.7.sp,
                color = colors.faint,
                modifier = Modifier.weight(1f),
            )
            IconButton(onClick = onNewPerspective, modifier = Modifier.size(30.dp)) {
                Icon(
                    imageVector = Icons.Rounded.Add,
                    contentDescription = "New perspective",
                    tint = colors.faint,
                    modifier = Modifier.size(18.dp),
                )
            }
        }
        if (saved.isEmpty()) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(10.dp))
                    .clickable(onClick = onNewPerspective)
                    .padding(horizontal = 12.dp, vertical = 9.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    imageVector = Icons.Outlined.BookmarkBorder,
                    contentDescription = null,
                    tint = colors.faint,
                    modifier = Modifier.size(17.dp),
                )
                Spacer(modifier = Modifier.width(10.dp))
                Text(
                    text = "New perspective",
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.faint,
                )
            }
        }
        saved.forEach { item ->
            var menu by remember { mutableStateOf(false) }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = 40.dp)
                    .clip(RoundedCornerShape(10.dp)),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(
                    modifier = Modifier
                        .weight(1f)
                        .clickable { onSaved(item) }
                        .padding(start = 12.dp, top = 7.dp, bottom = 7.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        imageVector = Icons.Outlined.BookmarkBorder,
                        contentDescription = null,
                        tint = colors.faint,
                        modifier = Modifier.size(17.dp),
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = item.name,
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.ink,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                Box {
                    IconButton(onClick = { menu = true }, modifier = Modifier.size(30.dp)) {
                        Icon(
                            imageVector = Icons.Rounded.MoreVert,
                            contentDescription = "Perspective actions",
                            tint = colors.faint,
                            modifier = Modifier.size(17.dp),
                        )
                    }
                    DropdownMenu(
                        expanded = menu,
                        onDismissRequest = { menu = false },
                        containerColor = colors.raised,
                    ) {
                        DropdownMenuItem(
                            text = { Text("Edit perspective") },
                            onClick = {
                                menu = false
                                onEditPerspective(item)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Delete perspective", color = colors.danger) },
                            onClick = {
                                menu = false
                                onDeletePerspective(item)
                            },
                        )
                    }
                }
            }
        }

        Spacer(modifier = Modifier.weight(1f))
        HorizontalDivider(color = colors.hairline, modifier = Modifier.padding(horizontal = 8.dp))
        HistoryHeader(
            open = historyOpen,
            available = canUndo || canRedo,
            onToggle = { historyOpen = !historyOpen },
        )
        if (historyOpen) {
            HistoryRow(
                icon = Icons.Rounded.Undo,
                label = "Undo",
                detail = undoLabel,
                enabled = canUndo,
                onClick = onUndo,
            )
            HistoryRow(
                icon = Icons.Rounded.Redo,
                label = "Redo",
                detail = redoLabel,
                enabled = canRedo,
                onClick = onRedo,
            )
        }
        HorizontalDivider(color = colors.hairline, modifier = Modifier.padding(horizontal = 8.dp))
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier = Modifier
                    .size(6.dp)
                    .clip(CircleShape)
                    .background(if (state.online) colors.success else colors.danger),
            )
            Spacer(modifier = Modifier.width(6.dp))
            Text(
                text = if (state.online) "Connected to Mac" else "Offline",
                style = MaterialTheme.typography.labelSmall,
                color = colors.faint,
                maxLines = 1,
            )
        }
    }
}

@Composable
private fun HistoryHeader(
    open: Boolean,
    available: Boolean,
    onToggle: () -> Unit,
) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .clickable(onClick = onToggle)
            .padding(horizontal = 12.dp, vertical = 9.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = Icons.Rounded.History,
            contentDescription = null,
            tint = colors.faint,
            modifier = Modifier.size(17.dp),
        )
        Spacer(modifier = Modifier.width(10.dp))
        Text(
            text = "History",
            style = MaterialTheme.typography.labelLarge,
            color = colors.ink,
            modifier = Modifier.weight(1f),
        )
        Text(
            text = if (available) "Undo available" else "Nothing yet",
            style = MaterialTheme.typography.labelSmall,
            color = colors.faint,
        )
        Spacer(modifier = Modifier.width(6.dp))
        Icon(
            imageVector = if (open) Icons.Rounded.ExpandMore else Icons.Rounded.ChevronRight,
            contentDescription = if (open) "Hide history" else "Show history",
            tint = colors.faint,
            modifier = Modifier.size(18.dp),
        )
    }
}

@Composable
private fun HistoryRow(
    icon: ImageVector,
    label: String,
    detail: String?,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = if (enabled) colors.ink else colors.faint.copy(alpha = 0.45f),
            modifier = Modifier.size(18.dp),
        )
        Spacer(modifier = Modifier.width(10.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelLarge,
                color = if (enabled) colors.ink else colors.faint,
            )
            Text(
                text = detail ?: if (label == "Undo") "Nothing to undo" else "Nothing to redo",
                style = MaterialTheme.typography.labelSmall,
                color = colors.faint,
                maxLines = 1,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun DrawerRow(
    label: String,
    icon: ImageVector,
    selected: Boolean,
    dragging: Boolean = false,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = flow
    Row(
        modifier = modifier
            .fillMaxWidth()
            .heightIn(min = 42.dp)
            .clip(RoundedCornerShape(10.dp))
            .background(
                when {
                    dragging -> colors.field
                    selected -> colors.field
                    else -> Color.Transparent
                },
            )
            .then(
                if (dragging) {
                    Modifier.border(1.dp, colors.accent, RoundedCornerShape(10.dp))
                } else {
                    Modifier
                },
            )
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = if (selected) colors.accent else colors.faint,
            modifier = Modifier.size(18.dp),
        )
        Spacer(modifier = Modifier.width(12.dp))
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = if (selected) colors.accent else colors.ink,
        )
    }
}


@Composable
private fun Header(
    title: String,
    subtitle: String,
    onMenu: () -> Unit,
    onSearch: () -> Unit,
    onSelect: (() -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    val colors = flow
    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(start = 20.dp, end = 20.dp, top = 0.dp, bottom = 10.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = onMenu, modifier = Modifier.size(32.dp)) {
                Icon(
                    imageVector = Icons.Rounded.Menu,
                    contentDescription = "Library",
                    tint = colors.faint,
                    modifier = Modifier.size(21.dp),
                )
            }
            Spacer(modifier = Modifier.width(8.dp))
            Row(
                modifier = Modifier
                    .weight(1f)
                    .height(38.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(colors.field)
                    .clickable(onClick = onSearch)
                    .padding(horizontal = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    imageVector = Icons.Outlined.Search,
                    contentDescription = null,
                    tint = colors.faint,
                    modifier = Modifier.size(18.dp),
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "Search tasks",
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.faint,
                )
            }
            if (onSelect != null) {
                Spacer(modifier = Modifier.width(4.dp))
                IconButton(onClick = onSelect, modifier = Modifier.size(32.dp)) {
                    Icon(
                        imageVector = Icons.Rounded.Checklist,
                        contentDescription = "Select tasks",
                        tint = colors.faint,
                        modifier = Modifier.size(20.dp),
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(20.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.displaySmall,
            color = colors.ink,
        )
        Spacer(modifier = Modifier.height(3.dp))
        Text(
            text = subtitle.ifBlank { " " },
            style = MaterialTheme.typography.labelMedium,
            color = if (subtitle.isBlank()) Color.Transparent else colors.faint,
        )
    }
}

private fun taskCount(count: Int): String = if (count == 1) "1 task" else "$count tasks"

private fun headerSubtitle(more: MoreView?, tab: Tab, state: AppState): String {
    if (more != null) {
        return when (more) {
            MoreView.Review -> "Projects due for a check-in"
            MoreView.Completed -> "Recently finished"
            MoreView.Archive -> "Projects put away"
            MoreView.Trash -> "Deleted tasks, still recoverable"
            MoreView.Tags -> "Browse the library by tag"
            MoreView.Flagged -> "Everything you flagged"
            MoreView.Priorities -> "Flagged and high-priority work"
            MoreView.Changed -> "Edited in the last few weeks"
            MoreView.Attachments -> "Files attached to tasks"
            MoreView.Nearby -> "Tagged places near the library"
        }
    }
    return when (tab) {
        Tab.Inbox -> state.snapshot?.let { snapshot ->
            val count = snapshot.db.inbox().size
            if (count == 0) "Nothing waiting" else "$count to sort"
        } ?: "Loading your library"

        Tab.Today -> LocalDate.now().format(
            DateTimeFormatter.ofPattern("EEEE, d MMMM", Locale.getDefault()),
        )

        Tab.Forecast -> "Your month at a glance"
        Tab.Browse -> "${state.snapshot?.db?.projects?.size ?: 0} projects"
        Tab.Settings -> ""
    }
}

@Composable
private fun OfflineNotice(onRetry: () -> Unit) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 2.dp)
            .clickable(onClick = onRetry),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .size(6.dp)
                .clip(CircleShape)
                .background(colors.warning),
        )
        Spacer(modifier = Modifier.width(7.dp))
        Text(
            text = "Not connected to your Mac · Tap to retry",
            style = MaterialTheme.typography.labelMedium,
            color = colors.faint,
        )
    }
}

@Composable
private fun InboxList(
    swipeActions: Boolean,
    children: Map<String, List<TaskView>>,
    tasks: List<TaskView>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
) {
    LazyColumn(contentPadding = PaddingValues(bottom = 100.dp)) {
        if (tasks.isEmpty()) {
            item(key = "empty") {
                Box(
                    modifier = Modifier.fillParentMaxSize(),
                    contentAlignment = Alignment.Center,
                ) {
                    EmptyState(
                        title = "Inbox zero",
                        body = "A clean slate. Capture something and it lands here until you give it a date or project.",
                        doodle = { InboxDoodle() },
                    )
                }
            }
        }
        items(tasks, key = { it.id }) { task ->
            if (swipeActions) {
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
            } else {
                TaskRow(
                    task = task.copy(pending = task.id in pending),
                    onToggle = { onToggle(task) },
                    onClick = { onClick(task) },
                )
                Hairline(Modifier.padding(start = 58.dp))
            }
        }
    }
}

@Composable
private fun TodayList(
    swipeActions: Boolean,
    children: Map<String, List<TaskView>>,
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
                Box(
                    modifier = Modifier.fillParentMaxSize(),
                    contentAlignment = Alignment.Center,
                ) {
                    EmptyState(
                        title = "Nothing due today",
                        body = "Enjoy the quiet, or look ahead in Forecast.",
                        doodle = { TodayDoodle() },
                    )
                }
            }
        }
        if (overdue.isNotEmpty()) {
            item { SectionLabel("Overdue") }
            items(overdue, key = { it.id }) { task ->
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
        if (today.isNotEmpty()) {
            item { SectionLabel(if (overdue.isEmpty()) "Due today" else "Today") }
            items(today, key = { it.id }) { task ->
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

@Composable
fun ForecastList(
    modifier: Modifier = Modifier,
    swipeActions: Boolean,
    children: Map<String, List<TaskView>>,
    groups: Map<LocalDate, List<TaskView>>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
) {
    LazyColumn(modifier = modifier, contentPadding = PaddingValues(bottom = 100.dp)) {
        if (groups.values.all { it.isEmpty() }) {
            item(key = "empty") {
                Box(
                    modifier = Modifier.fillParentMaxSize(),
                    contentAlignment = Alignment.Center,
                ) {
                    EmptyState(
                        title = "An open fortnight",
                        body = "Give a task a date and it appears here, grouped by the day it is due.",
                        doodle = { ForecastDoodle() },
                    )
                }
            }
        }
        groups.forEach { (date, tasks) ->
            item(key = "day-$date") { ForecastHeader(date, tasks.size) }
            items(tasks, key = { it.id }) { task ->
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

private data class ConfirmRequest(
    val title: String,
    val body: String,
    val confirmLabel: String,
    val action: () -> Unit,
)

private sealed interface BrowseEntry {
    data class Folder(val folder: FolderDto, val depth: Int) : BrowseEntry
    data class Project(val project: ProjectDto, val depth: Int) : BrowseEntry
}

@Composable
private fun BrowseList(
    snapshot: dev.flowlist.android.data.Snapshot?,
    onOpen: (ProjectDto) -> Unit,
    onNewProject: (String?) -> Unit,
    onNewFolder: (String?) -> Unit,
    onEditProject: (ProjectDto) -> Unit,
    onArchiveProject: (ProjectDto) -> Unit,
    onDeleteProject: (ProjectDto) -> Unit,
    onEditFolder: (FolderDto) -> Unit,
    onDeleteFolder: (FolderDto) -> Unit,
) {
    val colors = flow
    val db = snapshot?.db
    val projects = db?.projects?.values.orEmpty()
    val folders = db?.folders?.values.orEmpty()
    val entries = remember(snapshot) {
        val open = projects
            .filter { it.status == "active" || it.status == "on_hold" }
            .sortedBy { it.name.lowercase() }
        val orderedFolders = folders.sortedBy { it.name.lowercase() }

        fun collect(folderId: String?, depth: Int): List<BrowseEntry> {
            val rows = mutableListOf<BrowseEntry>()
            open.filter { it.folder == folderId }.forEach {
                rows += BrowseEntry.Project(it, depth)
            }
            orderedFolders.filter { it.parent == folderId }.forEach { folder ->
                rows += BrowseEntry.Folder(folder, depth)
                rows += collect(folder.id, depth + 1)
            }
            return rows
        }
        collect(null, 0)
    }

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 2.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextButton(onClick = { onNewProject(null) }) {
                Icon(Icons.Rounded.Add, contentDescription = null, modifier = Modifier.size(17.dp))
                Spacer(modifier = Modifier.width(5.dp))
                Text("New project")
            }
            TextButton(onClick = { onNewFolder(null) }) {
                Icon(
                    Icons.Outlined.CreateNewFolder,
                    contentDescription = null,
                    modifier = Modifier.size(17.dp),
                )
                Spacer(modifier = Modifier.width(5.dp))
                Text("New folder")
            }
        }

        if (entries.isEmpty()) {
            Box(
                modifier = Modifier.weight(1f).fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                EmptyState(
                    title = "No projects yet",
                    body = "Create a project here and it syncs straight to the Mac.",
                    doodle = { BrowseDoodle() },
                )
            }
            return@Column
        }

        LazyColumn(
            modifier = Modifier.weight(1f),
            contentPadding = PaddingValues(bottom = 100.dp),
        ) {
            items(entries) { entry ->
                when (entry) {
                    is BrowseEntry.Folder -> {
                        var menu by remember { mutableStateOf(false) }
                        val projectCount = projects.count {
                            it.folder == entry.folder.id &&
                                (it.status == "active" || it.status == "on_hold")
                        }
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(
                                    start = 20.dp + (entry.depth * 16).dp,
                                    end = 8.dp,
                                    top = 14.dp,
                                    bottom = 6.dp,
                                ),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Icon(
                                imageVector = Icons.Outlined.FolderOpen,
                                contentDescription = null,
                                tint = colors.faint,
                                modifier = Modifier.size(18.dp),
                            )
                            Spacer(modifier = Modifier.width(12.dp))
                            Text(
                                text = entry.folder.name,
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.SemiBold,
                                color = colors.ink,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f),
                            )
                            Text(
                                text = "$projectCount",
                                style = MaterialTheme.typography.labelMedium,
                                color = colors.faint,
                            )
                            Box {
                                IconButton(
                                    onClick = { menu = true },
                                    modifier = Modifier.size(32.dp),
                                ) {
                                    Icon(
                                        imageVector = Icons.Rounded.MoreVert,
                                        contentDescription = "Folder actions",
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
                                        text = { Text("Edit folder") },
                                        onClick = {
                                            menu = false
                                            onEditFolder(entry.folder)
                                        },
                                    )
                                    DropdownMenuItem(
                                        text = { Text("New project here") },
                                        onClick = {
                                            menu = false
                                            onNewProject(entry.folder.id)
                                        },
                                    )
                                    DropdownMenuItem(
                                        text = { Text("New folder inside") },
                                        onClick = {
                                            menu = false
                                            onNewFolder(entry.folder.id)
                                        },
                                    )
                                    DropdownMenuItem(
                                        text = { Text("Delete folder", color = colors.danger) },
                                        onClick = {
                                            menu = false
                                            onDeleteFolder(entry.folder)
                                        },
                                    )
                                }
                            }
                        }
                        Hairline(Modifier.padding(start = (20 + entry.depth * 16).dp))
                    }

                    is BrowseEntry.Project -> {
                        var menu by remember { mutableStateOf(false) }
                        val active = db?.tasks?.values?.count {
                            it.project == entry.project.id && it.isActive
                        } ?: 0
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { onOpen(entry.project) }
                                .padding(
                                    start = 20.dp + (entry.depth * 16).dp,
                                    end = 8.dp,
                                    top = 4.dp,
                                    bottom = 4.dp,
                                ),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Icon(
                                imageVector = projectIcon(entry.project.icon),
                                contentDescription = null,
                                tint = if (active == 0) colors.faint else colors.accent,
                                modifier = Modifier.size(16.dp),
                            )
                            Spacer(modifier = Modifier.width(12.dp))
                            Text(
                                text = entry.project.name,
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
                            Box {
                                IconButton(
                                    onClick = { menu = true },
                                    modifier = Modifier.size(32.dp),
                                ) {
                                    Icon(
                                        imageVector = Icons.Rounded.MoreVert,
                                        contentDescription = "Project actions",
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
                                        text = { Text("Edit project") },
                                        onClick = {
                                            menu = false
                                            onEditProject(entry.project)
                                        },
                                    )
                                    DropdownMenuItem(
                                        text = { Text("Archive") },
                                        onClick = {
                                            menu = false
                                            onArchiveProject(entry.project)
                                        },
                                    )
                                    DropdownMenuItem(
                                        text = { Text("Delete project", color = colors.danger) },
                                        onClick = {
                                            menu = false
                                            onDeleteProject(entry.project)
                                        },
                                    )
                                }
                            }
                        }
                        Hairline(Modifier.padding(start = (42 + entry.depth * 16).dp))
                    }
                }
            }
        }
    }
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
private fun ProjectSheet(
    project: ProjectDto,
    tasks: List<TaskView>,
    onDismiss: () -> Unit,
    onEdit: () -> Unit,
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
            Row(
                modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = project.name,
                    style = MaterialTheme.typography.headlineMedium,
                    color = colors.ink,
                    modifier = Modifier.weight(1f).padding(start = 8.dp),
                )
                IconButton(onClick = onEdit) {
                    Icon(
                        imageVector = Icons.Outlined.Edit,
                        contentDescription = "Edit project",
                        tint = colors.faint,
                        modifier = Modifier.size(20.dp),
                    )
                }
            }
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

private fun isTaskDescendant(db: dev.flowlist.android.data.Db, ancestorId: String, candidateId: String): Boolean {
    var current = db.tasks[candidateId]?.parent
    while (current != null) {
        if (current == ancestorId) return true
        current = db.tasks[current]?.parent
    }
    return false
}
