package dev.flowlist.android.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ChevronLeft
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material.icons.rounded.ExpandLess
import androidx.compose.material.icons.rounded.ExpandMore
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.Snapshot
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.forecastProjectDays
import dev.flowlist.android.data.forecastTaskDays
import dev.flowlist.android.data.forecastUndated
import dev.flowlist.android.data.isActive
import dev.flowlist.android.data.taskViews
import kotlin.math.abs
import java.time.DayOfWeek
import java.time.LocalDate
import java.time.YearMonth
import java.time.format.DateTimeFormatter
import java.util.Locale

@Composable
fun ForecastScreen(
    view: String,
    sort: String,
    priority: String = "all",
    availableOnly: Boolean = false,
    dueOnly: Boolean = false,
    planned: Boolean = true,
    onDueOnlyChange: (Boolean) -> Unit = {},
    onPlannedChange: (Boolean) -> Unit = {},
    onViewChange: (String) -> Unit,
    snapshot: Snapshot?,
    weekStartsMonday: Boolean,
    swipeActions: Boolean,
    children: Map<String, List<TaskView>>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
    onProject: (dev.flowlist.android.data.ProjectDto) -> Unit,
) {
    val colors = flow
    val monthMode = view != "agenda"
    val today = LocalDate.now()
    var selectedKey by rememberSaveable { mutableStateOf(today.toString()) }
    var expanded by rememberSaveable { mutableStateOf(false) }
    val selected = LocalDate.parse(selectedKey)
    val month = YearMonth.from(selected)
    val monthFormatter = remember { DateTimeFormatter.ofPattern("MMMM yyyy", Locale.getDefault()) }
    val dayFormatter = remember { DateTimeFormatter.ofPattern("EEEE, d MMMM", Locale.getDefault()) }

    val visibleIds = remember(snapshot, priority, availableOnly) {
        val db = snapshot?.db ?: return@remember emptySet<String>()
        filterViewTasks(db, db.taskViews(), priority, availableOnly).map { it.id }.toSet()
    }
    val taskDaysAll = remember(snapshot, dueOnly, planned) {
        snapshot?.db?.forecastTaskDays(dueOnly, planned).orEmpty()
    }
    val taskDays = remember(taskDaysAll, visibleIds) {
        taskDaysAll.mapValues { (_, tasks) -> tasks.filter { it.id in visibleIds } }
            .filterValues { it.isNotEmpty() }
    }
    val projectDays = remember(snapshot, dueOnly, planned) {
        snapshot?.db?.forecastProjectDays(dueOnly, planned).orEmpty()
    }
    val undated = remember(snapshot, visibleIds) {
        val (tagged, flagged) = snapshot?.db?.forecastUndated()
            ?: (emptyList<TaskView>() to emptyList<TaskView>())
        tagged.filter { it.id in visibleIds } to flagged.filter { it.id in visibleIds }
    }
    val tagged = undated.first
    val flagged = undated.second
    val counts = remember(taskDays, projectDays) {
        val merged = taskDays.keys + projectDays.keys
        merged.associateWith { date ->
            (taskDays[date]?.size ?: 0) + (projectDays[date]?.size ?: 0)
        }
    }
    val dayTasks = remember(taskDays, selected, sort) {
        sortTasks(taskDays[selected].orEmpty(), sort)
    }
    val dayProjects = projectDays[selected].orEmpty()
    val agendaHorizon = today.plusDays(30)
    val agendaDays = remember(taskDays) {
        taskDays.filterKeys { !it.isAfter(agendaHorizon) }
    }
    val agendaProjects = remember(projectDays) {
        projectDays.filterKeys { !it.isAfter(agendaHorizon) }
    }

    fun shiftMonth(step: Long) {
        val next = YearMonth.from(selected).plusMonths(step)
        selectedKey = if (YearMonth.from(today) == next) today.toString() else next.atDay(1).toString()
    }

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier.padding(horizontal = 20.dp, vertical = 2.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            ModePill("Month", monthMode) { onViewChange("month") }
            ModePill("Agenda", !monthMode) { onViewChange("agenda") }
            Spacer(modifier = Modifier.weight(1f))
            if (!dueOnly) {
                ModePill("Planned", planned) { onPlannedChange(!planned) }
            }
            ModePill("Due only", dueOnly) { onDueOnlyChange(!dueOnly) }
        }

        if (!monthMode) {
            ForecastList(
                modifier = Modifier.weight(1f),
                swipeActions = swipeActions,
                children = children,
                groups = agendaDays,
                projectGroups = agendaProjects,
                tagged = tagged,
                flagged = flagged,
                sort = sort,
                pending = pending,
                onToggle = onToggle,
                onClick = onClick,
                onTrash = onTrash,
                onProject = onProject,
            )
            return@Column
        }

        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 20.dp, end = 8.dp, top = 10.dp),
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { expanded = !expanded },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = month.format(monthFormatter),
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.SemiBold,
                    color = colors.ink,
                )
                Spacer(modifier = Modifier.width(6.dp))
                Icon(
                    imageVector = if (expanded) Icons.Rounded.ExpandLess else Icons.Rounded.ExpandMore,
                    contentDescription = if (expanded) "Show week" else "Show full month",
                    tint = colors.faint,
                    modifier = Modifier.size(22.dp),
                )
            }
            AnimatedVisibility(
                visible = expanded,
                enter = fadeIn(tween(160)) + expandVertically(tween(220)),
                exit = fadeOut(tween(120)) + shrinkVertically(tween(180)),
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(8.dp))
                            .clickable {
                                selectedKey = today.toString()
                            }
                            .padding(vertical = 10.dp),
                    ) {
                        Text(
                            text = "Today",
                            style = MaterialTheme.typography.titleMedium,
                            color = colors.accent,
                        )
                    }
                    Spacer(modifier = Modifier.weight(1f))
                    IconButton(onClick = { shiftMonth(-1) }, modifier = Modifier.size(46.dp)) {
                        Icon(Icons.Rounded.ChevronLeft, "Previous month", tint = colors.faint, modifier = Modifier.size(26.dp))
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    IconButton(onClick = { shiftMonth(1) }, modifier = Modifier.size(46.dp)) {
                        Icon(Icons.Rounded.ChevronRight, "Next month", tint = colors.faint, modifier = Modifier.size(26.dp))
                    }
                }
            }
        }

        LazyColumn(
            modifier = Modifier.weight(1f),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                top = 4.dp,
                bottom = 100.dp,
            ),
        ) {
            item(key = "month-calendar") {
                ForecastCalendar(
                    expanded = expanded,
                    today = today,
                    selected = selected,
                    month = month,
                    weekStartsMonday = weekStartsMonday,
                    counts = counts,
                    onSelect = { selectedKey = it.toString() },
                    onWeekChange = { selectedKey = it.toString() },
                    onExpandedChange = {
                        expanded = it
                        if (!it && selected < today) {
                            selectedKey = today.toString()
                        }
                    },
                )
            }
            item(key = "month-day-heading") {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(start = 20.dp, end = 20.dp, top = 16.dp, bottom = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = selected.format(dayFormatter),
                        style = MaterialTheme.typography.titleSmall,
                        color = colors.ink,
                        modifier = Modifier.weight(1f),
                    )
                    val total = dayTasks.size + dayProjects.size
                    if (total > 0) {
                        Text(
                            text = "$total",
                            style = MaterialTheme.typography.labelMedium,
                            color = colors.faint,
                        )
                    }
                }
            }
            if (dayTasks.isEmpty() && dayProjects.isEmpty()) {
                item(key = "month-day-empty") {
                    Text(
                        text = "Nothing on this day.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.faint,
                        modifier = Modifier.padding(horizontal = 20.dp, vertical = 18.dp),
                    )
                }
            } else {
                if (dayProjects.isNotEmpty()) {
                    item(key = "month-day-projects") { SectionLabel("Projects") }
                    items(dayProjects, key = { "project-${it.id}" }) { project ->
                        ForecastProjectRow(project = project, onOpen = { onProject(project) })
                    }
                }
                items(dayTasks, key = { it.id }) { task ->
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
}

@Composable
fun ForecastProjectRow(project: dev.flowlist.android.data.ProjectDto, onOpen: () -> Unit) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onOpen)
            .padding(horizontal = 20.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = projectIcon(project.icon),
            contentDescription = null,
            tint = colors.accent,
            modifier = Modifier.size(17.dp),
        )
        Spacer(modifier = Modifier.width(10.dp))
        Text(
            text = project.name,
            style = MaterialTheme.typography.bodyMedium,
            color = colors.ink,
            maxLines = 1,
            modifier = Modifier.weight(1f),
        )
        Text(
            text = project.status.replace('_', ' ').replaceFirstChar { it.uppercase() },
            style = MaterialTheme.typography.labelSmall,
            color = colors.faint,
        )
    }
    Hairline(Modifier.padding(start = 47.dp))
}

@Composable
private fun ForecastCalendar(
    expanded: Boolean,
    today: LocalDate,
    selected: LocalDate,
    month: YearMonth,
    weekStartsMonday: Boolean,
    counts: Map<LocalDate, Int>,
    onSelect: (LocalDate) -> Unit,
    onWeekChange: (LocalDate) -> Unit,
    onExpandedChange: (Boolean) -> Unit,
) {
    val colors = flow
    val density = LocalDensity.current
    var dragTotal by remember { mutableFloatStateOf(0f) }
    var dragTriggered by remember { mutableStateOf(false) }
    val threshold = with(density) { 34.dp.toPx() }
    val progress by animateFloatAsState(
        targetValue = if (expanded) 1f else 0f,
        animationSpec = tween(260),
        label = "calendar-expansion",
    )
    val weeks = weeksInMonth(month, weekStartsMonday)
    val collapsedHeight = with(density) { 70.dp.toPx() }
    val expandedHeight = with(density) { (22.dp + 50.dp * weeks).toPx() }
    val height = collapsedHeight + (expandedHeight - collapsedHeight) * progress
    val stripAlpha = (1f - progress * 1.6f).coerceIn(0f, 1f)
    val fullAlpha = ((progress - 0.15f) / 0.85f).coerceIn(0f, 1f)

    fun dragStarted() {
        dragTotal = 0f
        dragTriggered = false
    }

    fun dragBy(amount: Float) {
        dragTotal += amount
        if (!dragTriggered && abs(dragTotal) > threshold) {
            dragTriggered = true
            onExpandedChange(dragTotal > 0f)
        }
    }

    fun dragEnded() {
        dragTotal = 0f
        dragTriggered = false
    }

    Column(modifier = Modifier.fillMaxWidth()) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(with(density) { height.toDp() })
                .clipToBounds(),
        ) {
            if (progress > 0.005f) {
                Column(modifier = Modifier.alpha(fullAlpha)) {
                    WeekdayHeader(weekStartsMonday)
                    MonthGrid(
                        month = month,
                        weekStartsMonday = weekStartsMonday,
                        selected = selected,
                        counts = counts,
                        onSelect = onSelect,
                    )
                }
            }
            if (progress < 0.995f) {
                Column(modifier = Modifier.alpha(stripAlpha)) {
                    Spacer(modifier = Modifier.height(10.dp))
                    VerticalDragSurface(
                        onDragStart = ::dragStarted,
                        onDrag = ::dragBy,
                        onDragEnd = ::dragEnded,
                    ) {
                        WeekStrip(
                            today = today,
                            selected = selected,
                            weekStartsMonday = weekStartsMonday,
                            counts = counts,
                            onSelect = onSelect,
                            onWeekChange = onWeekChange,
                        )
                    }
                }
            }
        }
        VerticalDragSurface(
            modifier = Modifier.height(30.dp),
            onDragStart = ::dragStarted,
            onDrag = ::dragBy,
            onDragEnd = ::dragEnded,
        ) {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier = Modifier
                        .width(36.dp)
                        .height(4.dp)
                        .clip(CircleShape)
                        .background(colors.hairline),
                )
            }
        }
    }
}


@Composable
private fun VerticalDragSurface(
    onDragStart: () -> Unit,
    onDrag: (Float) -> Unit,
    onDragEnd: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    val dragStart = rememberUpdatedState(onDragStart)
    val drag = rememberUpdatedState(onDrag)
    val dragEnd = rememberUpdatedState(onDragEnd)
    val state = rememberDraggableState { delta ->
        drag.value(delta)
    }
    Box(
        modifier = modifier
            .fillMaxWidth()
            .draggable(
                state = state,
                orientation = Orientation.Vertical,
                onDragStarted = { dragStart.value() },
                onDragStopped = { dragEnd.value() },
            ),
    ) {
        content()
    }
}


private fun weeksInMonth(month: YearMonth, weekStartsMonday: Boolean): Int {
    val startIndex = if (weekStartsMonday) 1 else 7
    val firstOffset = (month.atDay(1).dayOfWeek.value - startIndex + 7) % 7
    return (firstOffset + month.lengthOfMonth() + 6) / 7
}

@Composable
private fun WeekStrip(
    today: LocalDate,
    selected: LocalDate,
    weekStartsMonday: Boolean,
    counts: Map<LocalDate, Int>,
    onSelect: (LocalDate) -> Unit,
    onWeekChange: (LocalDate) -> Unit,
) {
    val base = startOfWeek(today, weekStartsMonday)
    val pagerState = rememberPagerState(
        initialPage = weekPage(selected, base, weekStartsMonday),
        pageCount = { STRIP_PAGE_COUNT },
    )
    LaunchedEffect(pagerState.settledPage) {
        val start = base.plusWeeks(pagerState.settledPage.toLong())
        if (selected < start || selected > start.plusDays(6)) {
            val offset =
                ((selected.dayOfWeek.value - start.dayOfWeek.value + 7) % 7).toLong()
            onWeekChange(start.plusDays(offset))
        }
    }
    LaunchedEffect(selected) {
        val page = weekPage(selected, base, weekStartsMonday)
        if (page != pagerState.currentPage) {
            pagerState.animateScrollToPage(page)
        }
    }
    HorizontalPager(
        state = pagerState,
        modifier = Modifier
            .fillMaxWidth()
            .height(60.dp),
        beyondViewportPageCount = 1,
    ) { page ->
        val start = base.plusWeeks(page.toLong())
        Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp)) {
            repeat(7) { index ->
                val date = start.plusDays(index.toLong())
                WeekDayCell(
                    date = date,
                    selected = date == selected,
                    today = date == today,
                    count = counts[date] ?: 0,
                    onClick = { onSelect(date) },
                )
            }
        }
    }
}

@Composable
private fun RowScope.WeekDayCell(
    date: LocalDate,
    selected: Boolean,
    today: Boolean,
    count: Int,
    onClick: () -> Unit,
) {
    val colors = flow
    val drag = LocalTaskDrag.current
    val dayTarget = drag?.target?.kind == TaskDropKind.Day &&
        drag.target?.id == date.toString()
    val labelFormatter = remember { DateTimeFormatter.ofPattern("EEEEE", Locale.getDefault()) }
    Column(
        modifier = Modifier
            .weight(1f)
            .height(58.dp)
            .padding(horizontal = 2.dp)
            .clip(RoundedCornerShape(10.dp))
            .background(
                when {
                    dayTarget -> colors.accent.copy(alpha = 0.24f)
                    selected -> colors.accent.copy(alpha = 0.16f)
                    today -> colors.field
                    else -> Color.Transparent
                },
            )
            .onGloballyPositioned {
                drag?.bounds["day:$date"] = it.boundsInWindow()
            }
            .clickable(onClick = onClick)
            .padding(vertical = 5.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = date.format(labelFormatter)
                .uppercase(Locale.getDefault()),
            style = MaterialTheme.typography.labelSmall,
            color = colors.faint,
        )
        Spacer(modifier = Modifier.height(2.dp))
        Text(
            text = "${date.dayOfMonth}",
            style = MaterialTheme.typography.titleSmall,
            fontWeight = if (selected || today) FontWeight.SemiBold else FontWeight.Normal,
            color = if (selected || today) colors.accent else colors.ink,
        )
        Box(modifier = Modifier.height(12.dp), contentAlignment = Alignment.Center) {
            if (count > 0) {
                Text(
                    text = "$count",
                    style = MaterialTheme.typography.labelSmall,
                    color = if (selected) colors.accent else colors.faint,
                )
            }
        }
    }
}

private const val STRIP_PAGE_COUNT = 5_000

private fun startOfWeek(date: LocalDate, weekStartsMonday: Boolean): LocalDate {
    val first = if (weekStartsMonday) DayOfWeek.MONDAY else DayOfWeek.SUNDAY
    return date.minusDays(((date.dayOfWeek.value - first.value + 7) % 7).toLong())
}

private fun weekPage(
    date: LocalDate,
    base: LocalDate,
    weekStartsMonday: Boolean,
): Int {
    val target = startOfWeek(date, weekStartsMonday)
    val weeks = (target.toEpochDay() - base.toEpochDay()) / 7
    return weeks.coerceIn(0, (STRIP_PAGE_COUNT - 1).toLong()).toInt()
}

@Composable
private fun ModePill(label: String, selected: Boolean, onClick: () -> Unit) {
    val colors = flow
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(9.dp))
            .background(if (selected) colors.field else colors.background)
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 7.dp),
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = if (selected) colors.ink else colors.faint,
        )
    }
}

@Composable
private fun WeekdayHeader(weekStartsMonday: Boolean) {
    val colors = flow
    val days = if (weekStartsMonday) {
        listOf("M", "T", "W", "T", "F", "S", "S")
    } else {
        listOf("S", "M", "T", "W", "T", "F", "S")
    }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(22.dp)
            .padding(horizontal = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        days.forEach { day ->
            Text(
                text = day,
                style = MaterialTheme.typography.labelSmall,
                color = colors.faint,
                textAlign = TextAlign.Center,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun MonthGrid(
    month: YearMonth,
    weekStartsMonday: Boolean,
    selected: LocalDate,
    counts: Map<LocalDate, Int>,
    onSelect: (LocalDate) -> Unit,
) {
    val colors = flow
    val drag = LocalTaskDrag.current
    val today = LocalDate.now()
    val startIndex = if (weekStartsMonday) 1 else 7
    val firstOffset = (month.atDay(1).dayOfWeek.value - startIndex + 7) % 7
    val total = firstOffset + month.lengthOfMonth()
    val weeks = (total + 6) / 7

    Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp)) {
        repeat(weeks) { week ->
            Row(modifier = Modifier.fillMaxWidth()) {
                repeat(7) { column ->
                    val index = week * 7 + column
                    val day = index - firstOffset + 1
                    if (day in 1..month.lengthOfMonth()) {
                        val date = month.atDay(day)
                        val isToday = date == today
                        val isSelected = date == selected
                        val count = counts[date] ?: 0
                        val dayTarget = drag?.target?.kind == TaskDropKind.Day &&
                            drag.target?.id == date.toString()
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .height(50.dp)
                                .padding(2.dp)
                                .clip(RoundedCornerShape(9.dp))
                                .background(
                                    when {
                                        dayTarget -> colors.accent.copy(alpha = 0.24f)
                                        isSelected -> colors.accent.copy(alpha = 0.16f)
                                        isToday -> colors.field
                                        else -> colors.background
                                    },
                                )
                                .onGloballyPositioned {
                                    drag?.bounds["day:$date"] = it.boundsInWindow()
                                }
                                .clickable { onSelect(date) },
                            contentAlignment = Alignment.Center,
                        ) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Text(
                                    text = "$day",
                                    style = MaterialTheme.typography.labelMedium,
                                    fontWeight = if (isToday || isSelected) FontWeight.SemiBold else FontWeight.Normal,
                                    color = when {
                                        isSelected -> colors.accent
                                        isToday -> colors.accent
                                        else -> colors.ink
                                    },
                                )
                                if (count > 0) {
                                    Text(
                                        text = "$count",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = if (isSelected) colors.accent else colors.faint,
                                    )
                                }
                            }
                        }
                    } else {
                        Spacer(modifier = Modifier.weight(1f).height(50.dp).padding(2.dp))
                    }
                }
            }
        }
    }
}
