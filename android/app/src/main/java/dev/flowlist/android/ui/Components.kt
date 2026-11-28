package dev.flowlist.android.ui

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.InlineTextContent
import androidx.compose.foundation.text.appendInlineContent
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.*
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material.icons.rounded.Delete
import androidx.compose.material.icons.rounded.ExpandMore
import androidx.compose.material.icons.rounded.Flag
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.TextButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.Placeholder
import androidx.compose.ui.text.PlaceholderVerticalAlign
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.zIndex
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.isOverdue
import dev.flowlist.android.data.isToday
import dev.flowlist.android.data.label
import dev.flowlist.android.data.repeatRowLabel
import java.time.Instant

@Composable
fun CheckCircle(
    checked: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    priority: String? = null,
) {
    val colors = flow
    val background by animateColorAsState(
        if (checked) colors.accent else Color.Transparent,
        label = "check-background",
    )
    val scale by animateFloatAsState(
        targetValue = if (checked) 0.92f else 1f,
        animationSpec = spring(dampingRatio = 0.6f, stiffness = 700f),
        label = "check-scale",
    )
    val priorityTone = when (priority) {
        "high" -> colors.danger
        "medium" -> colors.warning
        "low" -> LowPriorityBlue
        else -> colors.hairline
    }
    val ring = if (checked) colors.accent else priorityTone
    val ringWidth = if (!checked && (priority == "high" || priority == "medium")) 2.dp else 1.5.dp
    Box(
        modifier = modifier
            .size(22.dp)
            .scale(scale)
            .clip(CircleShape)
            .background(background)
            .border(
                width = ringWidth,
                color = ring,
                shape = CircleShape,
            )
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        if (checked) {
            Icon(
                imageVector = Icons.Rounded.Check,
                contentDescription = "Completed",
                tint = colors.onAccent,
                modifier = Modifier.size(14.dp),
            )
        }
    }
}

/// Low priority stays blue in every theme, so it never reads as high (red).
internal val LowPriorityBlue = androidx.compose.ui.graphics.Color(0xFF3B82F6)

/** Set while the list is in select mode; rows then toggle ids instead of opening. */
class TaskSelection(val ids: Set<String>, val toggle: (String) -> Unit)
val LocalTaskSelection = compositionLocalOf<TaskSelection?> { null }

/// One segment of a row's metadata line: text, or a glyph the Mac also draws
/// (the defer mark and the repeat arrow).
private sealed interface DetailPart {
    data class Label(val text: String, val color: Color) : DetailPart
    data class Glyph(val icon: ImageVector, val description: String) : DetailPart
}

private fun isUpcoming(at: String?): Boolean =
    at?.let { runCatching { Instant.parse(it) }.getOrNull() }?.isAfter(Instant.now()) == true

@Composable
fun TaskRow(
    task: TaskView,
    onToggle: () -> Unit,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    depth: Int = 0,
    hasChildren: Boolean = false,
    childCount: Int = 0,
    childDone: Int = 0,
    expanded: Boolean = false,
    onToggleExpanded: (() -> Unit)? = null,
    dragging: Boolean = false,
    nestTarget: Boolean = false,
    rootTarget: Boolean = false,
) {
    val colors = flow
    val selection = LocalTaskSelection.current
    val selected = selection?.ids?.contains(task.id) == true
    Row(
        modifier = modifier
            .fillMaxWidth()
            .heightIn(min = 54.dp)
            .background(
                when {
                    dragging -> colors.raised
                    rootTarget -> colors.accent.copy(alpha = 0.15f)
                    selected -> colors.accent.copy(alpha = 0.10f)
                    else -> Color.Transparent
                },
            )
            .then(
                if (dragging || nestTarget) {
                    Modifier.border(1.dp, colors.accent, RoundedCornerShape(10.dp))
                } else {
                    Modifier
                },
            )
            .clickable {
                if (selection != null) selection.toggle(task.id) else onClick()
            }
            .padding(start = (20 + depth * 18).dp, end = 20.dp, top = 10.dp, bottom = 10.dp),
        verticalAlignment = Alignment.Top,
    ) {
        Spacer(modifier = Modifier.height(1.dp))
        if (selection != null) {
            Box(
                modifier = Modifier
                    .padding(top = 1.dp)
                    .size(22.dp)
                    .clip(CircleShape)
                    .border(
                        1.5.dp,
                        if (selected) colors.accent else colors.hairline,
                        CircleShape,
                    )
                    .background(if (selected) colors.accent else Color.Transparent)
                    .clickable { selection.toggle(task.id) },
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
        } else {
            CheckCircle(
                checked = task.completed,
                onClick = onToggle,
                modifier = Modifier.padding(top = 1.dp),
                priority = task.priority,
            )
        }
        Spacer(modifier = Modifier.width(14.dp))
        Column(
            modifier = Modifier.weight(1f),
        ) {
            Row(verticalAlignment = Alignment.Top) {
                Text(
                    text = task.title,
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (task.completed) colors.faint else colors.ink,
                    textDecoration = if (task.completed) TextDecoration.LineThrough else null,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                if (task.pending) {
                    Spacer(modifier = Modifier.width(8.dp))
                    PendingChip()
                }
                // The chevron and the flag share one 20dp-tall row so their
                // centres land on the task title's first line together.
                Row(
                    modifier = Modifier.height(20.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (hasChildren && onToggleExpanded != null) {
                        Box(
                            modifier = Modifier
                                .height(20.dp)
                                .width(30.dp)
                                .clickable {
                                    if (selection != null) {
                                        selection.toggle(task.id)
                                    } else {
                                        onToggleExpanded()
                                    }
                                },
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(
                                imageVector = if (expanded) {
                                    Icons.Rounded.ExpandMore
                                } else {
                                    Icons.Rounded.ChevronRight
                                },
                                contentDescription = if (expanded) {
                                    "Collapse subtasks"
                                } else {
                                    "Expand subtasks"
                                },
                                tint = colors.faint,
                                modifier = Modifier.size(18.dp),
                            )
                        }
                    }
                    if (task.flagged && !task.completed) {
                        Spacer(modifier = Modifier.width(6.dp))
                        Icon(
                            imageVector = Icons.Rounded.Flag,
                            contentDescription = "Flagged",
                            tint = colors.warning,
                            modifier = Modifier.size(16.dp),
                        )
                    }
                }
            }
            val dueColor = when {
                task.due?.isOverdue() == true && !task.completed -> colors.danger
                task.due?.isToday() == true && !task.completed -> colors.success
                else -> colors.faint
            }
            val details = buildList {
                if (childCount > 0) {
                    add(DetailPart.Label("[$childDone/$childCount]", colors.faint))
                }
                task.projectName?.let { add(DetailPart.Label(it, colors.faint)) }
                if (task.blockedByTag != null && !task.completed) {
                    add(DetailPart.Label("Blocked by #${task.blockedByTag}", colors.warning))
                }
                task.due?.let { add(DetailPart.Label(it.label(), dueColor)) }
                task.planned?.let { add(DetailPart.Label("Plan ${it.label()}", colors.faint)) }
                val defer = task.defer
                if (defer != null && isUpcoming(defer.at)) {
                    add(DetailPart.Glyph(Icons.Outlined.Schedule, "Deferred"))
                    add(DetailPart.Label(defer.label(), colors.faint))
                }
                repeatRowLabel(task.repeat)?.let { label ->
                    add(DetailPart.Glyph(Icons.Outlined.Repeat, "Repeats"))
                    add(DetailPart.Label(label, colors.faint))
                }
                if (task.checklistTotal > 0) add(DetailPart.Label("${task.checklistDone}/${task.checklistTotal}", colors.faint))
                if (task.estimateMin != null) add(DetailPart.Label("${task.estimateMin}m", colors.faint))
                if (task.tags.isNotEmpty()) add(DetailPart.Label(task.tags.joinToString(" ") { "#$it" }, colors.faint))
            }
            if (details.isNotEmpty()) {
                Spacer(modifier = Modifier.height(3.dp))
                val inline = buildMap {
                    details.forEachIndexed { index, part ->
                        if (part is DetailPart.Glyph) {
                            put(
                                "glyph-$index",
                                InlineTextContent(
                                    Placeholder(
                                        width = 12.sp,
                                        height = 12.sp,
                                        placeholderVerticalAlign = PlaceholderVerticalAlign.Center,
                                    ),
                                ) {
                                    Icon(
                                        imageVector = part.icon,
                                        contentDescription = null,
                                        tint = colors.faint,
                                        modifier = Modifier.fillMaxSize(),
                                    )
                                },
                            )
                        }
                    }
                }
                val text = buildAnnotatedString {
                    details.forEachIndexed { index, part ->
                        // A glyph and its label are one unit; only separate the
                        // units from each other.
                        if (index > 0 && details[index - 1] !is DetailPart.Glyph) {
                            withStyle(SpanStyle(color = colors.faint)) { append("  ·  ") }
                        }
                        when (part) {
                            is DetailPart.Label -> withStyle(SpanStyle(color = part.color)) {
                                append(part.text)
                            }
                            is DetailPart.Glyph -> {
                                appendInlineContent("glyph-$index", part.description)
                                append(" ")
                            }
                        }
                    }
                }
                Text(
                    text = text,
                    inlineContent = inline,
                    style = MaterialTheme.typography.labelMedium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SwipeTaskRow(
    task: TaskView,
    pending: Boolean,
    onToggle: () -> Unit,
    onTrash: () -> Unit,
    onClick: () -> Unit,
    depth: Int = 0,
    hasChildren: Boolean = false,
    childCount: Int = 0,
    childDone: Int = 0,
    expanded: Boolean = false,
    onToggleExpanded: (() -> Unit)? = null,
    modifier: Modifier = Modifier,
    dragging: Boolean = false,
    nestTarget: Boolean = false,
    rootTarget: Boolean = false,
) {
    val colors = flow
    val selection = LocalTaskSelection.current
    val state = rememberSwipeToDismissBoxState(
        confirmValueChange = { value ->
            when (value) {
                SwipeToDismissBoxValue.StartToEnd -> {
                    onToggle()
                    true
                }
                SwipeToDismissBoxValue.EndToStart -> {
                    onTrash()
                    true
                }
                SwipeToDismissBoxValue.Settled -> false
            }
        },
    )
    Column {
        SwipeToDismissBox(
            state = state,
            enableDismissFromStartToEnd = selection == null,
            enableDismissFromEndToStart = selection == null,
            backgroundContent = {
                val start = state.dismissDirection == SwipeToDismissBoxValue.StartToEnd
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(if (start) colors.success else colors.danger)
                        .padding(horizontal = 24.dp),
                    contentAlignment = if (start) Alignment.CenterStart else Alignment.CenterEnd,
                ) {
                    Icon(
                        imageVector = if (start) Icons.Rounded.Check else Icons.Rounded.Delete,
                        contentDescription = if (start) "Complete" else "Move to Trash",
                        tint = Color.White,
                        modifier = Modifier.size(22.dp),
                    )
                }
            },
        ) {
            TaskRow(
                task = task.copy(pending = pending),
                onToggle = onToggle,
                onClick = onClick,
                modifier = Modifier.background(colors.background).then(modifier),
                depth = depth,
                hasChildren = hasChildren,
                childCount = childCount,
                childDone = childDone,
                expanded = expanded,
                onToggleExpanded = onToggleExpanded,
                dragging = dragging,
                nestTarget = nestTarget,
                rootTarget = rootTarget,
            )
        }
        Hairline(Modifier.padding(start = 58.dp))
    }
}

@Composable
fun TaskTreeItem(
    task: TaskView,
    children: Map<String, List<TaskView>>,
    pending: Boolean,
    swipeActions: Boolean,
    depth: Int,
    onToggle: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
) {
    val kids = children[task.id].orEmpty()
    var opened by rememberSaveable(task.id) { mutableStateOf(false) }
    val drag = LocalTaskDrag.current
    val expanded = opened || drag?.revealed?.containsKey(task.id) == true
    val hasChildren = kids.isNotEmpty()
    val toggleExpanded = {
        val next = !expanded
        opened = next
        if (!next) drag?.revealed?.remove(task.id)
    }
    val colors = flow
    val selection = LocalTaskSelection.current
    val drop = drag?.target
    val nestTarget = drop?.kind == TaskDropKind.Nest &&
        drop.id == task.id &&
        drag?.dragging?.id != task.id
    val rootTarget = (drop?.kind == TaskDropKind.Before || drop?.kind == TaskDropKind.After) &&
        drop.id == task.id &&
        drag?.dragging?.id != task.id
    val dragging = drag?.dragging?.id == task.id
    val dragModifier = if (drag == null || selection != null) {
        Modifier
    } else {
        Modifier
            .onGloballyPositioned { coordinates ->
                drag.bounds["task:${task.id}"] = coordinates.boundsInWindow()
            }
            .graphicsLayer {
                if (drag.dragging?.id == task.id) {
                    translationY = drag.offset.y
                }
            }
            .zIndex(if (drag.dragging?.id == task.id) 1f else 0f)
            .pointerInput(task.id, drag) {
                detectDragGesturesAfterLongPress(
                    onDragStart = { local ->
                        drag.bounds["task:${task.id}"]?.let { row ->
                            drag.start(task, row, local)
                        }
                    },
                    onDrag = { change, dragAmount ->
                        change.consume()
                        drag.move(dragAmount)
                    },
                    onDragEnd = { drag.end() },
                    onDragCancel = { drag.cancel() },
                )
            }
    }
    if (drag != null) {
        DisposableEffect(drag, task.id, depth) {
            if (depth == 0) {
                drag.roots[task.id] = true
            } else {
                drag.roots.remove(task.id)
            }
            onDispose {
                drag.bounds.remove("task:${task.id}")
                drag.roots.remove(task.id)
            }
        }
    }
    if (swipeActions && depth == 0) {
        SwipeTaskRow(
            task = task,
            pending = pending,
            onToggle = { onToggle(task) },
            onTrash = { onTrash(task) },
            onClick = { onClick(task) },
            hasChildren = hasChildren,
            childCount = kids.size,
            childDone = kids.count { it.completed },
            expanded = expanded,
            onToggleExpanded = toggleExpanded,
            modifier = dragModifier,
            dragging = dragging,
            nestTarget = nestTarget,
            rootTarget = rootTarget,
        )
    } else {
        TaskRow(
            task = task.copy(pending = pending),
            onToggle = { onToggle(task) },
            onClick = { onClick(task) },
            modifier = dragModifier,
            depth = depth,
            hasChildren = hasChildren,
            childCount = kids.size,
            childDone = kids.count { it.completed },
            expanded = expanded,
            onToggleExpanded = toggleExpanded,
            dragging = dragging,
            nestTarget = nestTarget,
            rootTarget = rootTarget,
        )
        Hairline(Modifier.padding(start = (58 + depth * 18).dp))
    }
    if (expanded) {
        kids.forEach { child ->
            TaskTreeItem(
                task = child,
                children = children,
                pending = pending,
                swipeActions = swipeActions,
                depth = depth + 1,
                onToggle = onToggle,
                onTrash = onTrash,
                onClick = onClick,
            )
        }
    }
}

@Composable
fun PendingChip() {
    val colors = flow
    Row(
        modifier = Modifier
            .clip(RoundedCornerShape(6.dp))
            .background(colors.field)
            .padding(horizontal = 7.dp, vertical = 3.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier
                .size(5.dp)
                .clip(CircleShape)
                .background(colors.warning),
        )
        Spacer(modifier = Modifier.width(5.dp))
        Text(
            text = "Waiting",
            style = MaterialTheme.typography.labelSmall,
            color = colors.faint,
        )
    }
}

@Composable
fun Hairline(modifier: Modifier = Modifier) {
    HorizontalDivider(
        modifier = modifier,
        thickness = 1.dp,
        color = flow.hairline,
    )
}

@Composable
fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text.uppercase(),
        style = MaterialTheme.typography.labelSmall,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.8.sp,
        color = flow.faint,
        modifier = modifier.padding(start = 20.dp, end = 20.dp, top = 16.dp, bottom = 6.dp),
    )
}

@Composable
fun EmptyState(
    title: String,
    body: String,
    modifier: Modifier = Modifier,
    doodle: @Composable () -> Unit,
) {
    val colors = flow
    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier.height(150.dp),
            contentAlignment = Alignment.Center,
        ) {
            doodle()
        }
        Spacer(modifier = Modifier.height(10.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            color = colors.ink,
        )
        Spacer(modifier = Modifier.height(6.dp))
        Text(
            text = body,
            style = MaterialTheme.typography.bodyMedium,
            color = colors.faint,
            lineHeight = 20.sp,
            textAlign = androidx.compose.ui.text.style.TextAlign.Center,
        )
    }
}

@Composable
fun FlowSnackbarHost(hostState: SnackbarHostState) {
    val colors = flow
    SnackbarHost(hostState = hostState) { data ->
        // A fresh dismiss state per message, so a swiped-away toast cannot
        // leave the next one already half off screen.
        key(data) {
            val dismissState = rememberSwipeToDismissBoxState(
                confirmValueChange = { value ->
                    if (value == SwipeToDismissBoxValue.Settled) {
                        false
                    } else {
                        data.dismiss()
                        true
                    }
                },
            )
            SwipeToDismissBox(
                state = dismissState,
                backgroundContent = {},
            ) {
                Row(
                    modifier = Modifier
                        .padding(horizontal = 16.dp, vertical = 10.dp)
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(12.dp))
                        .background(colors.ink)
                        .padding(horizontal = 16.dp, vertical = 14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = data.visuals.message,
                        style = MaterialTheme.typography.labelLarge,
                        color = colors.background,
                        modifier = Modifier.weight(1f),
                    )
                    data.visuals.actionLabel?.let { label ->
                        TextButton(onClick = { data.performAction() }) {
                            Text(
                                text = label,
                                style = MaterialTheme.typography.labelLarge,
                                fontWeight = FontWeight.SemiBold,
                                color = colors.background,
                            )
                        }
                    }
                }
            }
        }
    }
}

/**
 * One snackbar at a time. A new message replaces the visible one instead of
 * queueing behind it, so repeated taps cannot leave a minutes-long parade of
 * toasts. Swipe a toast left or right to dismiss it.
 */
class Toasts(private val scope: CoroutineScope) {
    val host = SnackbarHostState()
    private var job: Job? = null

    fun show(
        message: String,
        actionLabel: String? = null,
        duration: SnackbarDuration = SnackbarDuration.Short,
        onAction: (suspend () -> Unit)? = null,
    ) {
        job?.cancel()
        host.currentSnackbarData?.dismiss()
        job = scope.launch {
            val result = host.showSnackbar(
                message = message,
                actionLabel = actionLabel,
                duration = duration,
            )
            if (result == SnackbarResult.ActionPerformed) {
                onAction?.invoke()
            }
        }
    }
}

@Composable
fun SyncDot(label: String, color: Color, modifier: Modifier = Modifier) {
    Row(
        modifier = modifier,
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(5.dp),
    ) {
        Box(
            modifier = Modifier
                .size(6.dp)
                .clip(CircleShape)
                .background(color),
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = flow.faint,
        )
    }
}

/** The fixed project icon set, mirrored from the Mac's `ProjectIcon`. */
val projectIconChoices = listOf(
    "project" to "Project",
    "folder" to "Folder",
    "star" to "Star",
    "briefcase" to "Briefcase",
    "home" to "Home",
    "heart" to "Heart",
    "book" to "Book",
    "code" to "Code",
    "rocket" to "Rocket",
    "coffee" to "Coffee",
    "hash" to "Hash",
    "plane" to "Plane",
    "apple" to "Apple",
    "basketball" to "Basketball",
    "beach_ball" to "Beach ball",
    "cake" to "Cake",
    "book_open" to "Open book",
    "garden" to "Garden",
    "record" to "Record",
    "square" to "Square",
    "growth" to "Growth",
    "checklist" to "Checklist",
    "terminal" to "Terminal",
    "analytics" to "Analytics",
    "document" to "Document",
    "dog" to "Dog",
    "money" to "Dollar",
    "rupee" to "Rupee",
    "euro" to "Euro",
    "pound" to "Pound",
    "yen" to "Yen",
    "won" to "Won",
    "ruble" to "Ruble",
    "bitcoin" to "Bitcoin",
    "franc" to "Franc",
    "lira" to "Lira",
    "mail" to "Mail",
    "smile" to "Smile",
    "flag" to "Flag",
    "gift" to "Gift",
    "support" to "Support",
    "flask" to "Science",
    "laptop" to "Laptop",
    "phone" to "Phone",
    "lightbulb" to "Idea",
    "podcast" to "Podcast",
    "moon" to "Moon",
    "paintbrush" to "Design",
    "palette" to "Palette",
    "person" to "Person",
    "team" to "Team",
    "pill" to "Health",
    "pin" to "Pin",
    "pizza" to "Pizza",
    "puzzle" to "Puzzle",
    "study" to "Study",
    "sofa" to "Home life",
    "sparkles" to "Sparkles",
    "stethoscope" to "Doctor",
    "sun" to "Sun",
    "telescope" to "Telescope",
    "video" to "Video",
    "clock" to "Clock",
)
fun projectIcon(slug: String): ImageVector = when (slug) {
    "folder" -> Icons.Outlined.Folder
    "star" -> Icons.Outlined.Star
    "briefcase" -> Icons.Outlined.Work
    "home" -> Icons.Outlined.Home
    "heart" -> Icons.Outlined.Favorite
    "book" -> Icons.Outlined.MenuBook
    "code" -> Icons.Outlined.Code
    "rocket" -> Icons.Outlined.RocketLaunch
    "coffee" -> Icons.Outlined.LocalCafe
    "hash" -> Icons.Outlined.Numbers
    "plane" -> Icons.Outlined.Flight
    "apple" -> Icons.Outlined.Eco
    "basketball" -> Icons.Outlined.SportsBasketball
    "beach_ball" -> Icons.Outlined.SportsVolleyball
    "cake" -> Icons.Outlined.Cake
    "book_open" -> Icons.Outlined.AutoStories
    "garden" -> Icons.Outlined.Yard
    "record" -> Icons.Outlined.Album
    "square" -> Icons.Outlined.CropSquare
    "growth" -> Icons.Outlined.TrendingUp
    "checklist" -> Icons.Outlined.Checklist
    "terminal" -> Icons.Outlined.Terminal
    "analytics" -> Icons.Outlined.PieChart
    "document" -> Icons.Outlined.Description
    "dog" -> Icons.Outlined.Pets
    "money" -> Icons.Outlined.AttachMoney
    "rupee" -> Icons.Outlined.CurrencyRupee
    "euro" -> Icons.Outlined.Euro
    "pound" -> Icons.Outlined.CurrencyPound
    "yen" -> Icons.Outlined.CurrencyYen
    "won" -> Icons.Outlined.Payments
    "ruble" -> Icons.Outlined.CurrencyRuble
    "bitcoin" -> Icons.Outlined.CurrencyBitcoin
    "franc" -> Icons.Outlined.CurrencyFranc
    "lira" -> Icons.Outlined.CurrencyLira
    "mail" -> Icons.Outlined.MailOutline
    "smile" -> Icons.Outlined.Mood
    "flag" -> Icons.Outlined.Flag
    "gift" -> Icons.Outlined.CardGiftcard
    "support" -> Icons.Outlined.HeadsetMic
    "flask" -> Icons.Outlined.Science
    "laptop" -> Icons.Outlined.Laptop
    "phone" -> Icons.Outlined.Smartphone
    "lightbulb" -> Icons.Outlined.Lightbulb
    "podcast" -> Icons.Outlined.Mic
    "moon" -> Icons.Outlined.DarkMode
    "paintbrush" -> Icons.Outlined.Brush
    "palette" -> Icons.Outlined.Palette
    "person" -> Icons.Outlined.Person
    "team" -> Icons.Outlined.People
    "pill" -> Icons.Outlined.Medication
    "pin" -> Icons.Outlined.PushPin
    "pizza" -> Icons.Outlined.LocalPizza
    "puzzle" -> Icons.Outlined.Extension
    "study" -> Icons.Outlined.School
    "sofa" -> Icons.Outlined.Weekend
    "sparkles" -> Icons.Outlined.AutoAwesome
    "stethoscope" -> Icons.Outlined.MonitorHeart
    "sun" -> Icons.Outlined.LightMode
    "telescope" -> Icons.Outlined.TravelExplore
    "video" -> Icons.Outlined.Videocam
    "clock" -> Icons.Outlined.AccessTime
    else -> Icons.Outlined.Assignment
}
