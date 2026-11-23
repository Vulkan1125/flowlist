package dev.flowlist.android.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.AttachFile
import androidx.compose.material.icons.outlined.DriveFileMove
import androidx.compose.material.icons.outlined.Link
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.ArrowDownward
import androidx.compose.material.icons.rounded.ArrowUpward
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.Download
import androidx.compose.material.icons.rounded.Edit
import androidx.compose.material.icons.rounded.Link
import androidx.compose.material.icons.rounded.Place
import androidx.compose.material.icons.rounded.DownloadDone
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
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.data.FaviconLoader
import androidx.compose.ui.zIndex
import kotlin.math.roundToInt
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import dev.flowlist.android.data.ChecklistDto
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.attachmentDisplayName
import dev.flowlist.android.data.attachmentUrl
import dev.flowlist.android.data.canPreviewAttachment
import dev.flowlist.android.data.StampDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskDto
import dev.flowlist.android.data.TaskEdit
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import dev.flowlist.android.data.repeatCommand
import dev.flowlist.android.data.isOverdue
import dev.flowlist.android.data.isToday
import dev.flowlist.android.data.label

private enum class StepMode { Parallel, Sequential, OneOff }

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun TaskEditSheet(
    task: TaskDto,
    projects: List<ProjectDto>,
    tags: List<TagDto>,
    subtasks: List<TaskDto>,
    moveTargets: List<TaskDto>,
    pending: Boolean,
    weekStartsMonday: Boolean = true,
    defaultTimeMinutes: Int = 9 * 60,
    onDismiss: () -> Unit,
    onSave: (TaskEdit) -> Unit,
    onCreateProject: (String) -> Unit,
    onCreateTag: (String) -> Unit,
    savedAttachments: Set<String>,
    onAddAttachment: () -> Unit,
    onSaveAttachment: (String) -> Unit,
    onRemoveSavedAttachment: (String) -> Unit,
    onOpenAttachment: (String) -> Unit,
    onPreviewAttachment: (String) -> Unit,
    onRemoveAttachment: (String) -> Unit,
    onAddSubtask: (String) -> Unit,
    onToggleSubtask: (TaskDto, Boolean) -> Unit,
    onMoveSubtask: (TaskDto, TaskDto) -> Unit,
    onDeleteSubtask: (TaskDto) -> Unit,
    onReorderSubtasks: (List<TaskDto>) -> Unit,
    onAddLink: (String, String) -> Unit,
    onRenameLink: (String, String, String) -> Unit,
    onChecklist: (ChecklistDto, Boolean) -> Unit,
    onAddChecklist: (String) -> Unit,
    onDeleteChecklist: (ChecklistDto) -> Unit,
    onMoveChecklist: (ChecklistDto, Int) -> Unit,
    onDeleteSeries: () -> Unit,
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
    var stepMode by remember(task.id) {
        mutableStateOf(
            when {
                task.stepsOneOff -> StepMode.OneOff
                task.sequential -> StepMode.Sequential
                else -> StepMode.Parallel
            },
        )
    }
    var confirmOneOff by remember(task.id) { mutableStateOf(false) }
    var estimate by remember(task.id) {
        mutableStateOf(task.estimateMin?.toString().orEmpty())
    }
    var selectedTags by remember(task.id) { mutableStateOf(task.tags.toSet()) }
    var movingSubtask by remember { mutableStateOf<TaskDto?>(null) }
    var targetFilter by remember { mutableStateOf("") }
    var addingSubtask by remember { mutableStateOf(false) }
    var subtaskTitle by remember { mutableStateOf("") }
    var orderedSubtasks by remember(task.id) { mutableStateOf(subtasks) }
    LaunchedEffect(task.id, subtasks.map { it.id }) { orderedSubtasks = subtasks }
    var subtaskDragId by remember(task.id) { mutableStateOf<String?>(null) }
    var subtaskDragFrom by remember(task.id) { mutableIntStateOf(-1) }
    var subtaskDragTo by remember(task.id) { mutableIntStateOf(-1) }
    var subtaskDragOffset by remember(task.id) { mutableFloatStateOf(0f) }
    var subtaskSettleFrom by remember(task.id) { mutableFloatStateOf(0f) }
    var subtaskSettleTo by remember(task.id) { mutableFloatStateOf(0f) }
    var subtaskSettling by remember(task.id) { mutableStateOf(false) }
    var subtaskRowHeight by remember(task.id) { mutableFloatStateOf(0f) }
    val subtaskRowPx = if (subtaskRowHeight > 0f) {
        subtaskRowHeight
    } else {
        with(LocalDensity.current) { 48.dp.toPx() }
    }
    val subtaskSettle = remember { Animatable(0f) }
    val subtaskScope = rememberCoroutineScope()
    var linkOpen by remember { mutableStateOf(false) }
    var linkText by remember { mutableStateOf("") }
    var linkName by remember { mutableStateOf("") }
    var renamingLink by remember { mutableStateOf<String?>(null) }
    var oneOffDragId by remember(task.id) { mutableStateOf<String?>(null) }
    var oneOffDragOffset by remember(task.id) { mutableFloatStateOf(0f) }
    var oneOffRowHeight by remember(task.id) { mutableFloatStateOf(0f) }
    var repeat by remember(task.id) { mutableStateOf(task.repeat) }
    var confirmSeries by remember { mutableStateOf(false) }
    val subtaskFocus = remember { FocusRequester() }
    var pendingProjectName by remember { mutableStateOf<String?>(null) }
    var pendingTagName by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(addingSubtask) {
        if (addingSubtask) {
            delay(120)
            subtaskFocus.requestFocus()
        }
    }
    LaunchedEffect(projects, pendingProjectName) {
        val wanted = pendingProjectName ?: return@LaunchedEffect
        projects.firstOrNull { it.name.equals(wanted, ignoreCase = true) }?.let { created ->
            project = created.id
            pendingProjectName = null
        }
    }
    LaunchedEffect(tags, pendingTagName) {
        val wanted = pendingTagName ?: return@LaunchedEffect
        tags.firstOrNull { it.name.equals(wanted, ignoreCase = true) }?.let { created ->
            selectedTags = selectedTags + created.id
            pendingTagName = null
        }
    }
    var projectMenu by remember { mutableStateOf(false) }
    var projectMenuWidth by remember { mutableStateOf(0.dp) }
    val density = LocalDensity.current
    var newProjectOpen by remember { mutableStateOf(false) }
    var newTagOpen by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }
    var datePickerForDue by remember { mutableStateOf(true) }
    var showDueSheet by remember { mutableStateOf(false) }
    var dateError by remember { mutableStateOf<String?>(null) }
    var pendingSave by remember(task.id) { mutableStateOf<TaskEdit?>(null) }

    val repeatUnit = ((repeat as? JsonObject)?.get("unit") as? JsonPrimitive)?.content
    fun setRepeat(unit: String?) {
        repeat = unit?.let { repeatCommand(it) }
    }

    fun submitSubtask() {
        val name = subtaskTitle.trim()
        if (name.isEmpty()) return
        if (stepMode == StepMode.OneOff) onAddChecklist(name) else onAddSubtask(name)
        subtaskTitle = ""
        addingSubtask = false
    }

    fun applyStamp(value: StampDto?) {
        if (datePickerForDue) due = value else defer = value
        dateError = null
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        modifier = Modifier.windowInsetsPadding(
            WindowInsets.statusBars.only(WindowInsetsSides.Top),
        ),
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .imePadding()
                .padding(horizontal = 20.dp)
                .padding(bottom = 30.dp),
        ) {
            Spacer(modifier = Modifier.height(10.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                CheckCircle(
                    checked = task.completed,
                    onClick = onComplete,
                    priority = task.priority,
                )
                Spacer(modifier = Modifier.width(14.dp))
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    placeholder = { Text("Task name", color = colors.faint) },
                    textStyle = TextStyle(
                        fontSize = 21.sp,
                        fontWeight = FontWeight.SemiBold,
                        color = colors.ink,
                    ),
                    colors = borderlessColors(),
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Rounded.Close, contentDescription = "Close", tint = colors.faint)
                }
            }
            if (pending) {
                Text(
                    text = "Waiting to sync",
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.warning,
                    modifier = Modifier.padding(start = 50.dp, bottom = 2.dp),
                )
            }

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

            Spacer(modifier = Modifier.height(12.dp))
            Label("Attachments")
            task.attachments.forEach { path ->
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable {
                            if (canPreviewAttachment(path)) {
                                onPreviewAttachment(path)
                            } else {
                                onOpenAttachment(path)
                            }
                        }
                        .padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    val isLink = path.startsWith("http://") || path.startsWith("https://")
                    if (isLink) {
                        SiteFavicon(path, colors.faint)
                    } else {
                        Icon(
                            imageVector = Icons.Outlined.AttachFile,
                            contentDescription = null,
                            tint = colors.faint,
                            modifier = Modifier.size(17.dp),
                        )
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = attachmentDisplayName(path),
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.ink,
                        maxLines = 1,
                        modifier = Modifier.weight(1f),
                    )
                    val saved = path in savedAttachments
                    if (!isLink) {
                        IconButton(
                            onClick = {
                                if (saved) onRemoveSavedAttachment(path) else onSaveAttachment(path)
                            },
                            modifier = Modifier.size(30.dp),
                        ) {
                            Icon(
                                imageVector = if (saved) {
                                    Icons.Rounded.DownloadDone
                                } else {
                                    Icons.Rounded.Download
                                },
                                contentDescription = if (saved) {
                                    "Remove offline copy"
                                } else {
                                    "Save for offline"
                                },
                                tint = if (saved) colors.accent else colors.faint,
                                modifier = Modifier.size(16.dp),
                            )
                        }
                    }
                    if (isLink) {
                        IconButton(
                            onClick = {
                                renamingLink = path
                                linkText = attachmentUrl(path)
                                linkName = path.substringAfter('\t', "").trim()
                                linkOpen = true
                            },
                            modifier = Modifier.size(30.dp),
                        ) {
                            Icon(
                                imageVector = Icons.Rounded.Edit,
                                contentDescription = "Rename link",
                                tint = colors.faint,
                                modifier = Modifier.size(16.dp),
                            )
                        }
                    }
                    IconButton(
                        onClick = { onRemoveAttachment(path) },
                        modifier = Modifier.size(30.dp),
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.Close,
                            contentDescription = "Remove attachment",
                            tint = colors.faint,
                            modifier = Modifier.size(15.dp),
                        )
                    }
                }
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable(onClick = onAddAttachment)
                    .padding(vertical = 8.dp),
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
                    text = "Add attachment",
                    style = MaterialTheme.typography.labelLarge,
                    color = colors.faint,
                )
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable {
                        renamingLink = null
                        linkText = ""
                        linkName = ""
                        linkOpen = true
                    }
                    .padding(vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    imageVector = Icons.Outlined.Link,
                    contentDescription = null,
                    tint = colors.accent,
                    modifier = Modifier.size(18.dp),
                )
                Spacer(modifier = Modifier.width(10.dp))
                Text(
                    text = "Add link",
                    style = MaterialTheme.typography.labelLarge,
                    color = colors.faint,
                )
            }

            Spacer(modifier = Modifier.height(12.dp))
            Label("Repeat")
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                PriorityPill("None", repeat == null, colors.accent) { setRepeat(null) }
                PriorityPill("Daily", repeatUnit == "days", colors.accent) { setRepeat("days") }
                PriorityPill("Weekly", repeatUnit == "weeks", colors.accent) { setRepeat("weeks") }
                PriorityPill("Monthly", repeatUnit == "months", colors.accent) { setRepeat("months") }
                PriorityPill("Yearly", repeatUnit == "years", colors.accent) { setRepeat("years") }
            }

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
                onClear = {
                    due = null
                    dateError = null
                },
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
                onClear = {
                    defer = null
                    dateError = null
                },
                onClick = {
                    datePickerForDue = false
                    showDueSheet = true
                },
            )

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .onSizeChanged { size ->
                        projectMenuWidth = with(density) { size.width.toDp() }
                    },
            ) {
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
                    modifier = if (projectMenuWidth > 0.dp) {
                        Modifier.width(projectMenuWidth).heightIn(max = 360.dp)
                    } else {
                        Modifier
                    },
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
                    DropdownMenuItem(
                        text = {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(
                                    imageVector = Icons.Rounded.Add,
                                    contentDescription = null,
                                    tint = colors.accent,
                                    modifier = Modifier.size(17.dp),
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Text("Create new project", color = colors.accent)
                            }
                        },
                        onClick = {
                            projectMenu = false
                            newProjectOpen = true
                        },
                    )
                }
            }

            Spacer(modifier = Modifier.height(16.dp))
            val activeDone = if (stepMode == StepMode.OneOff) {
                task.checklist.count { it.completed }
            } else {
                orderedSubtasks.count { it.completed }
            }
            val activeTotal = if (stepMode == StepMode.OneOff) {
                task.checklist.size
            } else {
                orderedSubtasks.size
            }
            Label(
                if (activeTotal == 0) {
                    "Subtasks"
                } else {
                    "Subtasks · $activeDone/$activeTotal"
                },
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                PriorityPill("Parallel", stepMode == StepMode.Parallel, colors.accent) {
                    stepMode = StepMode.Parallel
                }
                PriorityPill("Sequential", stepMode == StepMode.Sequential, colors.accent) {
                    stepMode = StepMode.Sequential
                }
                PriorityPill("One-off", stepMode == StepMode.OneOff, colors.accent) {
                    if (stepMode != StepMode.OneOff && orderedSubtasks.isNotEmpty()) {
                        confirmOneOff = true
                    } else {
                        stepMode = StepMode.OneOff
                    }
                }
            }
            Spacer(modifier = Modifier.height(6.dp))
            if (stepMode != StepMode.OneOff) {
                var firstIncomplete: String? = null
                orderedSubtasks.forEachIndexed { index, child ->
                key(child.id) {
                    val blockedBy = if (stepMode == StepMode.Sequential && !child.completed) {
                        firstIncomplete
                    } else {
                        null
                    }
                    val dragging = subtaskDragId == child.id
                    val settlingThis = dragging && subtaskSettling
                    val shift = when {
                        dragging -> 0f
                        subtaskDragFrom >= 0 &&
                            subtaskDragTo > subtaskDragFrom &&
                            index in (subtaskDragFrom + 1)..subtaskDragTo -> -subtaskRowPx
                        subtaskDragFrom >= 0 &&
                            subtaskDragTo < subtaskDragFrom &&
                            index in subtaskDragTo until subtaskDragFrom -> subtaskRowPx
                        else -> 0f
                    }
                    val animatedShift by animateFloatAsState(
                        targetValue = shift,
                        animationSpec = if (subtaskDragFrom >= 0) {
                            spring(dampingRatio = 0.82f, stiffness = 700f)
                        } else {
                            snap()
                        },
                        label = "subtask-shift-$index",
                    )
                    val translation = when {
                        settlingThis ->
                            subtaskSettleFrom +
                                (subtaskSettleTo - subtaskSettleFrom) * subtaskSettle.value
                        dragging -> subtaskDragOffset
                        else -> animatedShift
                    }
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .onSizeChanged { size ->
                                if (subtaskRowHeight <= 0f && size.height > 0) {
                                    subtaskRowHeight = size.height.toFloat()
                                }
                            }
                            .zIndex(if (dragging) 1f else 0f)
                            .graphicsLayer { translationY = translation }
                            .padding(vertical = 3.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        CheckCircle(
                            checked = child.completed,
                            onClick = { onToggleSubtask(child, !child.completed) },
                            modifier = if (blockedBy != null) Modifier.alpha(0.45f) else Modifier,
                            priority = child.priority,
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Text(
                            text = child.title,
                            style = MaterialTheme.typography.bodyMedium,
                            color = if (child.completed || blockedBy != null) colors.faint else colors.ink,
                            textDecoration = if (child.completed) {
                                androidx.compose.ui.text.style.TextDecoration.LineThrough
                            } else {
                                null
                            },
                            maxLines = 1,
                            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                            modifier = Modifier.weight(1f),
                        )
                        IconButton(
                            onClick = {
                                movingSubtask = child
                                targetFilter = ""
                            },
                            modifier = Modifier.size(32.dp),
                        ) {
                            Icon(
                                imageVector = Icons.Outlined.DriveFileMove,
                                contentDescription = "Move subtask",
                                tint = colors.faint,
                                modifier = Modifier.size(17.dp),
                            )
                        }
                        IconButton(
                            onClick = { onDeleteSubtask(child) },
                            modifier = Modifier.size(32.dp),
                        ) {
                            Icon(
                                imageVector = Icons.Rounded.Close,
                                contentDescription = "Delete subtask",
                                tint = colors.faint,
                                modifier = Modifier.size(17.dp),
                            )
                        }
                        Box(
                            modifier = Modifier
                                .size(24.dp)
                                .pointerInput(orderedSubtasks, child.id, subtaskRowPx) {
                                    detectDragGesturesAfterLongPress(
                                        onDragStart = {
                                            subtaskDragId = child.id
                                            subtaskDragFrom = index
                                            subtaskDragTo = index
                                            subtaskDragOffset = 0f
                                            subtaskSettling = false
                                        },
                                        onDrag = { change, dragAmount ->
                                            change.consume()
                                            subtaskDragOffset += dragAmount.y
                                            if (subtaskDragFrom >= 0) {
                                                val next = (
                                                    subtaskDragFrom +
                                                        (subtaskDragOffset / subtaskRowPx).roundToInt()
                                                    ).coerceIn(0, orderedSubtasks.lastIndex)
                                                if (next != subtaskDragTo) {
                                                    subtaskDragTo = next
                                                }
                                            }
                                        },
                                        onDragEnd = {
                                            val from = subtaskDragFrom
                                            val to = subtaskDragTo
                                            if (from < 0) {
                                                subtaskDragId = null
                                                return@detectDragGesturesAfterLongPress
                                            }
                                            subtaskSettleFrom = subtaskDragOffset
                                            subtaskSettleTo = if (to != from) {
                                                (to - from) * subtaskRowPx
                                            } else {
                                                0f
                                            }
                                            subtaskSettling = true
                                            subtaskScope.launch {
                                                subtaskSettle.snapTo(0f)
                                                subtaskSettle.animateTo(
                                                    1f,
                                                    tween(
                                                        durationMillis = 180,
                                                        easing = FastOutSlowInEasing,
                                                    ),
                                                )
                                                if (from >= 0 && to != from) {
                                                    val reordered =
                                                        orderedSubtasks.toMutableList()
                                                    val item = reordered.removeAt(from)
                                                    reordered.add(to, item)
                                                    orderedSubtasks = reordered
                                                    onReorderSubtasks(reordered)
                                                }
                                                subtaskDragId = null
                                                subtaskDragFrom = -1
                                                subtaskDragTo = -1
                                                subtaskDragOffset = 0f
                                                subtaskSettleFrom = 0f
                                                subtaskSettleTo = 0f
                                                subtaskSettling = false
                                                subtaskSettle.snapTo(0f)
                                            }
                                        },
                                        onDragCancel = {
                                            if (subtaskDragFrom < 0) {
                                                subtaskDragId = null
                                                return@detectDragGesturesAfterLongPress
                                            }
                                            subtaskSettleFrom = subtaskDragOffset
                                            subtaskSettleTo = 0f
                                            subtaskSettling = true
                                            subtaskScope.launch {
                                                subtaskSettle.snapTo(0f)
                                                subtaskSettle.animateTo(
                                                    1f,
                                                    tween(
                                                        durationMillis = 160,
                                                        easing = FastOutSlowInEasing,
                                                    ),
                                                )
                                                subtaskDragId = null
                                                subtaskDragFrom = -1
                                                subtaskDragTo = -1
                                                subtaskDragOffset = 0f
                                                subtaskSettleFrom = 0f
                                                subtaskSettleTo = 0f
                                                subtaskSettling = false
                                                subtaskSettle.snapTo(0f)
                                            }
                                        },
                                    )
                                },
                            contentAlignment = Alignment.Center,
                        ) {
                            Text(
                                text = "⠿",
                                style = MaterialTheme.typography.labelMedium,
                                color = colors.faint,
                            )
                        }
                    }
                    if (
                        stepMode == StepMode.Sequential &&
                        !child.completed &&
                        firstIncomplete == null
                    ) {
                        firstIncomplete = child.title
                    }
                }
            }
            }
            if (stepMode == StepMode.OneOff) {
            task.checklist.forEachIndexed { index, item ->
                    val oneOffDragging = oneOffDragId == item.id
                    val rowHeightPx = if (oneOffRowHeight > 0f) {
                        oneOffRowHeight
                    } else {
                        with(LocalDensity.current) { 40.dp.toPx() }
                    }
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .onSizeChanged { size ->
                                if (oneOffRowHeight <= 0f && size.height > 0) {
                                    oneOffRowHeight = size.height.toFloat()
                                }
                            }
                            .zIndex(if (oneOffDragging) 1f else 0f)
                            .graphicsLayer {
                                if (oneOffDragging) translationY = oneOffDragOffset
                            }
                            .padding(vertical = 2.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        CheckCircle(
                            checked = item.completed,
                            onClick = { onChecklist(item, !item.completed) },
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Text(
                            text = item.title,
                            style = MaterialTheme.typography.bodyMedium,
                            color = if (item.completed) colors.faint else colors.ink,
                            textDecoration = if (item.completed) {
                                androidx.compose.ui.text.style.TextDecoration.LineThrough
                            } else {
                                null
                            },
                            maxLines = 1,
                            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                            modifier = Modifier.weight(1f),
                        )
                        IconButton(
                            onClick = { onDeleteChecklist(item) },
                            modifier = Modifier.size(30.dp),
                        ) {
                            Icon(
                                imageVector = Icons.Rounded.Close,
                                contentDescription = "Delete one-off",
                                tint = colors.faint,
                                modifier = Modifier.size(15.dp),
                            )
                        }
                        Box(
                            modifier = Modifier
                                .size(24.dp)
                                .pointerInput(task.id, item.id, rowHeightPx) {
                                    detectDragGesturesAfterLongPress(
                                        onDragStart = {
                                            oneOffDragId = item.id
                                            oneOffDragOffset = 0f
                                        },
                                        onDrag = { change, dragAmount ->
                                            change.consume()
                                            oneOffDragOffset += dragAmount.y
                                        },
                                        onDragEnd = {
                                            val from = task.checklist.indexOfFirst { it.id == item.id }
                                            if (from >= 0) {
                                                val to = (
                                                    from + (oneOffDragOffset / rowHeightPx).roundToInt()
                                                    ).coerceIn(0, task.checklist.lastIndex)
                                                if (to != from) onMoveChecklist(item, to)
                                            }
                                            oneOffDragId = null
                                            oneOffDragOffset = 0f
                                        },
                                        onDragCancel = {
                                            oneOffDragId = null
                                            oneOffDragOffset = 0f
                                        },
                                    )
                                },
                            contentAlignment = Alignment.Center,
                        ) {
                            Text(
                                text = "⠿",
                                style = MaterialTheme.typography.labelMedium,
                                color = colors.faint,
                            )
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(8.dp))
            if (addingSubtask) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = { addingSubtask = false; subtaskTitle = "" }) {
                        Text("Cancel", color = colors.faint)
                    }
                    TextButton(onClick = { submitSubtask() }) {
                        Text("Add", color = colors.accent)
                    }
                }
                OutlinedTextField(
                    value = subtaskTitle,
                    onValueChange = { subtaskTitle = it },
                    placeholder = {
                        Text(
                            if (stepMode == StepMode.OneOff) {
                                "One-off step"
                            } else {
                                "Subtask name"
                            },
                            color = colors.faint,
                        )
                    },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { submitSubtask() }),
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
                        .focusRequester(subtaskFocus),
                )
            } else {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { addingSubtask = true }
                        .padding(vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        Icons.Rounded.Add,
                        contentDescription = null,
                        tint = colors.accent,
                        modifier = Modifier.size(18.dp),
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = if (stepMode == StepMode.OneOff) {
                            "Add one-off"
                        } else {
                            "Add subtask"
                        },
                        style = MaterialTheme.typography.labelLarge,
                        color = colors.faint,
                    )
                }
            }

            Spacer(modifier = Modifier.height(14.dp))
            Label("Priority")
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                PriorityPill("None", priority == null, colors.ink) { priority = null }
                PriorityPill("H", priority == "high", colors.danger) { priority = "high" }
                PriorityPill("M", priority == "medium", colors.warning) { priority = "medium" }
                PriorityPill("L", priority == "low", LowPriorityBlue) { priority = "low" }
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
                FlowRow(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
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
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                if (tag.location != null) {
                                    Icon(
                                        imageVector = Icons.Rounded.Place,
                                        contentDescription = "Geotagged",
                                        tint = if (active) colors.accent else colors.faint,
                                        modifier = Modifier.size(11.dp),
                                    )
                                    Spacer(modifier = Modifier.width(3.dp))
                                }
                                Text(
                                    text = "#${tag.name}",
                                    style = MaterialTheme.typography.labelMedium,
                                    color = if (active) colors.accent else colors.faint,
                                )
                            }
                        }
                    }
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(9.dp))
                            .background(colors.field)
                            .clickable { newTagOpen = true }
                            .padding(horizontal = 12.dp, vertical = 6.dp),
                    ) {
                        Text(
                            text = "+",
                            style = MaterialTheme.typography.labelLarge,
                            color = colors.accent,
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(26.dp))
            dateError?.let { message ->
                Text(
                    text = message,
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.danger,
                    modifier = Modifier.padding(bottom = 8.dp),
                )
            }
            Button(
                onClick = {
                    val edit = TaskEdit(
                        title = title.trim(),
                        notes = notes,
                        project = project,
                        due = due,
                        defer = defer,
                        priority = priority,
                        flagged = flagged,
                        estimateMin = estimate.toIntOrNull(),
                        tags = selectedTags,
                        repeat = repeat,
                        sequential = stepMode == StepMode.Sequential,
                        stepsOneOff = stepMode == StepMode.OneOff,
                    )
                    val dueDate = due?.local()?.toLocalDate()
                    val deferDate = defer?.local()?.toLocalDate()
                    dateError = if (dueDate != null && deferDate != null && deferDate.isAfter(dueDate)) {
                        "Defer can't be after the due date"
                    } else {
                        null
                    }
                    if (dateError == null) {
                        val repeats = task.repeat != null && edit.repeat != null
                        if (repeats && edit.priority != task.priority) {
                            pendingSave = edit
                        } else {
                            onSave(edit)
                        }
                    }
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
            if (task.repeat != null) {
                TextButton(
                    onClick = { confirmSeries = true },
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text("Delete whole series", color = colors.danger)
                }
            }
        }
    }

    if (confirmSeries) {
        AlertDialog(
            onDismissRequest = { confirmSeries = false },
            title = { Text("Delete the whole series?") },
            text = { Text("Every occurrence of “${task.title}” goes to Trash on your Mac.") },
            confirmButton = {
                TextButton(onClick = { confirmSeries = false; onDeleteSeries() }) {
                    Text("Delete series", color = colors.danger)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmSeries = false }) { Text("Cancel") }
            },
            containerColor = colors.background,
        )
    }

    if (confirmOneOff) {
        AlertDialog(
            onDismissRequest = { confirmOneOff = false },
            title = { Text("Convert to one-off steps?") },
            text = {
                Text(
                    if (orderedSubtasks.size == 1) {
                        "1 subtask becomes a checklist step. Notes, dates, tags and priority are removed; the title and completion state stay."
                    } else {
                        "${orderedSubtasks.size} subtasks become checklist steps. Notes, dates, tags and priority are removed; titles and completion states stay."
                    },
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmOneOff = false
                        stepMode = StepMode.OneOff
                    },
                ) {
                    Text("Convert", color = colors.accent)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmOneOff = false }) { Text("Cancel") }
            },
            containerColor = colors.background,
        )
    }

    if (showDueSheet) {
        DueDateSheet(
            title = if (datePickerForDue) "Due date" else "Defer date",
            initial = if (datePickerForDue) due else defer,
            weekStartsMonday = weekStartsMonday,
            defaultTime = java.time.LocalTime.of(
                defaultTimeMinutes / 60,
                defaultTimeMinutes % 60,
            ),
            onDismiss = { showDueSheet = false },
            onSet = { value ->
                applyStamp(value)
                showDueSheet = false
            },
        )
    }

    if (linkOpen) {
        AlertDialog(
            onDismissRequest = { linkOpen = false },
            title = { Text(if (renamingLink == null) "Add a link" else "Rename link") },
            text = {
                Column {
                    OutlinedTextField(
                        value = linkText,
                        onValueChange = { linkText = it },
                        placeholder = { Text("https://…", color = colors.faint) },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
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
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = linkName,
                        onValueChange = { linkName = it },
                        placeholder = { Text("Name (optional)", color = colors.faint) },
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
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        val url = linkText.trim()
                        if (url.startsWith("http://") || url.startsWith("https://")) {
                            val original = renamingLink
                            if (original == null) {
                                onAddLink(url, linkName)
                            } else {
                                onRenameLink(original, url, linkName)
                            }
                            linkOpen = false
                            linkText = ""
                            linkName = ""
                            renamingLink = null
                        }
                    },
                ) {
                    Text(if (renamingLink == null) "Add" else "Save", color = colors.accent)
                }
            },
            dismissButton = {
                TextButton(onClick = { linkOpen = false }) { Text("Cancel", color = colors.faint) }
            },
            containerColor = colors.background,
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

    val pending = pendingSave
    if (pending != null) {
        AlertDialog(
            onDismissRequest = { pendingSave = null },
            title = { Text("Apply to repeating tasks?") },
            text = {
                Text(
                    "“${task.title}” repeats. Apply this priority to just this occurrence, " +
                        "or to every occurrence in the series?",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        pendingSave = null
                        onSave(pending.copy(priorityAll = true))
                    },
                ) {
                    Text("All repeats", color = colors.accent)
                }
            },
            dismissButton = {
                Row {
                    TextButton(onClick = { pendingSave = null }) { Text("Cancel") }
                    TextButton(
                        onClick = {
                            pendingSave = null
                            onSave(pending)
                        },
                    ) {
                        Text("This task", color = colors.accent)
                    }
                }
            },
            containerColor = colors.background,
        )
    }

    if (newProjectOpen) {
        NamePromptDialog(
            title = "New project",
            placeholder = "Project name",
            confirmLabel = "Create",
            onDismiss = { newProjectOpen = false },
        ) { name ->
            newProjectOpen = false
            pendingProjectName = name
            onCreateProject(name)
        }
    }

    if (newTagOpen) {
        NamePromptDialog(
            title = "New tag",
            placeholder = "Tag name",
            confirmLabel = "Create",
            onDismiss = { newTagOpen = false },
        ) { name ->
            newTagOpen = false
            val cleaned = name.removePrefix("#").trim()
            pendingTagName = cleaned
            onCreateTag(cleaned)
        }
    }

    movingSubtask?.let { child ->
        val candidates = moveTargets
            .filter { it.id != child.id && it.id != task.id }
            .filter { it.title.contains(targetFilter.trim(), ignoreCase = true) }
            .sortedBy { it.title.lowercase() }
        AlertDialog(
            onDismissRequest = {
                movingSubtask = null
                targetFilter = ""
            },
            title = { Text("Move “${child.title}”") },
            text = {
                Column {
                    OutlinedTextField(
                        value = targetFilter,
                        onValueChange = { targetFilter = it },
                        placeholder = { Text("Find a task", color = colors.faint) },
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
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    if (candidates.isEmpty()) {
                        Text(
                            text = "No matching tasks.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = colors.faint,
                            modifier = Modifier.padding(vertical = 10.dp),
                        )
                    } else {
                        LazyColumn(modifier = Modifier.heightIn(max = 340.dp)) {
                            items(candidates, key = { it.id }) { target ->
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .clip(RoundedCornerShape(9.dp))
                                        .clickable {
                                            onMoveSubtask(child, target)
                                            movingSubtask = null
                                            targetFilter = ""
                                        }
                                        .padding(vertical = 11.dp, horizontal = 6.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        text = target.title,
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = colors.ink,
                                        modifier = Modifier.weight(1f),
                                    )
                                    Icon(
                                        imageVector = Icons.Outlined.DriveFileMove,
                                        contentDescription = null,
                                        tint = colors.faint,
                                        modifier = Modifier.size(16.dp),
                                    )
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                TextButton(
                    onClick = {
                        movingSubtask = null
                        targetFilter = ""
                    },
                ) {
                    Text("Cancel", color = colors.faint)
                }
            },
            containerColor = colors.background,
        )
    }
}

@Composable
private fun NamePromptDialog(
    title: String,
    placeholder: String,
    confirmLabel: String,
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit,
) {
    val colors = flow
    var value by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            OutlinedTextField(
                value = value,
                onValueChange = { value = it },
                placeholder = { Text(placeholder, color = colors.faint) },
                singleLine = true,
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = colors.accent,
                    unfocusedBorderColor = colors.hairline,
                    focusedContainerColor = colors.background,
                    unfocusedContainerColor = colors.background,
                    focusedTextColor = colors.ink,
                    unfocusedTextColor = colors.ink,
                    cursorColor = colors.accent,
                ),
                modifier = Modifier.fillMaxWidth(),
            )
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(value.trim()) },
                enabled = value.isNotBlank(),
            ) {
                Text(confirmLabel, color = colors.accent)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel", color = colors.faint) }
        },
        containerColor = colors.background,
    )
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
            .padding(horizontal = 12.dp, vertical = 7.dp),
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelLarge,
            color = if (selected) tone else colors.faint,
            maxLines = 1,
            softWrap = false,
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


@Composable
private fun SiteFavicon(path: String, tint: Color) {
    var bitmap by remember(path) { mutableStateOf<androidx.compose.ui.graphics.ImageBitmap?>(null) }
    LaunchedEffect(path) {
        bitmap = FaviconLoader.load(path)?.asImageBitmap()
    }
    val image = bitmap
    if (image != null) {
        Image(
            bitmap = image,
            contentDescription = null,
            modifier = Modifier
                .size(17.dp)
                .clip(RoundedCornerShape(4.dp)),
        )
    } else {
        Icon(
            imageVector = Icons.Rounded.Link,
            contentDescription = null,
            tint = tint,
            modifier = Modifier.size(17.dp),
        )
    }
}
