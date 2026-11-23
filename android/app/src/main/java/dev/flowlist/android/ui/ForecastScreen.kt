package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.Snapshot
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.forecast
import dev.flowlist.android.data.isActive
import dev.flowlist.android.data.taskViews
import java.time.DayOfWeek
import kotlin.math.abs
import java.time.LocalDate
import java.time.YearMonth
import java.time.format.DateTimeFormatter
import java.util.Locale

@Composable
fun ForecastScreen(
    view: String,
    sort: String,
    onViewChange: (String) -> Unit,
    snapshot: Snapshot?,
    weekStartsMonday: Boolean,
    swipeActions: Boolean,
    children: Map<String, List<TaskView>>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onTrash: (TaskView) -> Unit,
) {
    val colors = flow
    val monthMode = view != "agenda"
    var selectedKey by rememberSaveable { mutableStateOf(LocalDate.now().toString()) }
    var expanded by rememberSaveable { mutableStateOf(false) }
    val selected = LocalDate.parse(selectedKey)
    val month = YearMonth.from(selected)

    val byDate = remember(snapshot) {
        snapshot?.db?.tasks?.values
            ?.asSequence()
            ?.filter { it.isActive && it.parent == null && it.due != null }
            ?.mapNotNull { task ->
                val date = task.due?.local()?.toLocalDate() ?: return@mapNotNull null
                date to task.id
            }
            ?.groupBy({ it.first }, { it.second })
            .orEmpty()
    }
    val allViews = remember(snapshot) {
        snapshot?.db?.taskViews().orEmpty().associateBy { it.id }
    }

    fun shiftMonth(step: Long) {
        val next = YearMonth.from(selected).plusMonths(step)
        val today = LocalDate.now()
        selectedKey = if (YearMonth.from(today) == next) today.toString() else next.atDay(1).toString()
    }

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier.padding(horizontal = 20.dp, vertical = 2.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            ModePill("Month", monthMode) { onViewChange("month") }
            ModePill("Agenda", !monthMode) { onViewChange("agenda") }
        }

        if (!monthMode) {
            ForecastList(
                modifier = Modifier.weight(1f),
                swipeActions = swipeActions,
                children = children,
                groups = snapshot?.db?.forecast().orEmpty(),
                sort = sort,
                pending = pending,
                onToggle = onToggle,
                onClick = onClick,
                onTrash = onTrash,
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
                    text = month.format(DateTimeFormatter.ofPattern("MMMM yyyy", Locale.getDefault())),
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.SemiBold,
                    color = colors.ink,
                    modifier = Modifier.weight(1f),
                )
                Icon(
                    imageVector = if (expanded) Icons.Rounded.ExpandLess else Icons.Rounded.ExpandMore,
                    contentDescription = if (expanded) "Show week" else "Show full month",
                    tint = colors.faint,
                    modifier = Modifier.size(22.dp),
                )
            }
            Spacer(modifier = Modifier.height(6.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .clickable {
                            selectedKey = LocalDate.now().toString()
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

        ForecastCalendar(
            expanded = expanded,
            selected = selected,
            month = month,
            weekStartsMonday = weekStartsMonday,
            counts = byDate.mapValues { it.value.size },
            onSelect = { selectedKey = it.toString() },
            onWeekChange = { selectedKey = it.toString() },
            onExpandedChange = { expanded = it },
        )

        val dayTasks = sortTasks(
            byDate[selected].orEmpty().mapNotNull { allViews[it] },
            sort,
        )
        LazyColumn(
            modifier = Modifier.weight(1f),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                top = 4.dp,
                bottom = 100.dp,
            ),
        ) {
            item(key = "month-day-heading") {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(start = 20.dp, end = 20.dp, top = 16.dp, bottom = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = selected.format(
                            DateTimeFormatter.ofPattern("EEEE, d MMMM", Locale.getDefault()),
                        ),
                        style = MaterialTheme.typography.titleSmall,
                        color = colors.ink,
                        modifier = Modifier.weight(1f),
                    )
                    if (dayTasks.isNotEmpty()) {
                        Text(
                            text = "${dayTasks.size}",
                            style = MaterialTheme.typography.labelMedium,
                            color = colors.faint,
                        )
                    }
                }
            }
            if (dayTasks.isEmpty()) {
                item(key = "month-day-empty") {
                    Text(
                        text = "Nothing due on this day.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.faint,
                        modifier = Modifier.padding(horizontal = 20.dp, vertical = 18.dp),
                    )
                }
            } else {
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
private fun ForecastCalendar(
    expanded: Boolean,
    selected: LocalDate,
    month: YearMonth,
    weekStartsMonday: Boolean,
    counts: Map<LocalDate, Int>,
    onSelect: (LocalDate) -> Unit,
    onWeekChange: (LocalDate) -> Unit,
    onExpandedChange: (Boolean) -> Unit,
) {
    val colors = flow
    val threshold = with(LocalDensity.current) { 44.dp.toPx() }
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .pointerInput(expanded) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    var lastY = down.position.y
                    var totalY = 0f
                    var vertical = false
                    while (true) {
                        val event = awaitPointerEvent(PointerEventPass.Initial)
                        val change = event.changes.firstOrNull { it.id == down.id } ?: break
                        totalY += change.position.y - lastY
                        lastY = change.position.y
                        val totalX = change.position.x - down.position.x
                        if (!vertical &&
                            abs(totalY) > viewConfiguration.touchSlop &&
                            abs(totalY) > abs(totalX)
                        ) {
                            vertical = true
                        }
                        if (vertical) {
                            change.consume()
                        }
                        if (!change.pressed) break
                    }
                    if (vertical) {
                        if (totalY > threshold) {
                            onExpandedChange(true)
                        } else if (totalY < -threshold) {
                            onExpandedChange(false)
                        }
                    }
                }
            },
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(24.dp)
                .clickable { onExpandedChange(!expanded) },
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
        if (expanded) {
            Column {
                WeekdayHeader(weekStartsMonday)
                MonthGrid(
                    month = month,
                    weekStartsMonday = weekStartsMonday,
                    selected = selected,
                    counts = counts,
                    onSelect = onSelect,
                )
            }
        } else {
            key(weekStartsMonday) {
                WeekStrip(
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

@Composable
private fun WeekStrip(
    selected: LocalDate,
    weekStartsMonday: Boolean,
    counts: Map<LocalDate, Int>,
    onSelect: (LocalDate) -> Unit,
    onWeekChange: (LocalDate) -> Unit,
) {
    val anchor = if (weekStartsMonday) WEEK_ANCHOR_MONDAY else WEEK_ANCHOR_SUNDAY
    val pagerState = rememberPagerState(
        initialPage = weekPage(selected, weekStartsMonday),
        pageCount = { WEEK_PAGE_COUNT },
    )
    LaunchedEffect(pagerState.settledPage) {
        val start = anchor.plusWeeks(pagerState.settledPage.toLong())
        val first = if (weekStartsMonday) DayOfWeek.MONDAY else DayOfWeek.SUNDAY
        val offset = ((selected.dayOfWeek.value - first.value + 7) % 7).toLong()
        val date = start.plusDays(offset)
        if (date != selected) {
            onWeekChange(date)
        }
    }
    LaunchedEffect(selected) {
        val page = weekPage(selected, weekStartsMonday)
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
        val start = anchor.plusWeeks(page.toLong())
        Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp)) {
            repeat(7) { index ->
                val date = start.plusDays(index.toLong())
                WeekDayCell(
                    date = date,
                    selected = date == selected,
                    today = date == LocalDate.now(),
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
    Column(
        modifier = Modifier
            .weight(1f)
            .height(58.dp)
            .padding(horizontal = 2.dp)
            .clip(RoundedCornerShape(10.dp))
            .background(
                when {
                    selected -> colors.accent.copy(alpha = 0.16f)
                    today -> colors.field
                    else -> Color.Transparent
                },
            )
            .clickable(onClick = onClick)
            .padding(vertical = 5.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = date.format(DateTimeFormatter.ofPattern("EEEEE", Locale.getDefault()))
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

private const val WEEK_PAGE_COUNT = 10_000
private val WEEK_ANCHOR_MONDAY = LocalDate.of(2000, 1, 3)
private val WEEK_ANCHOR_SUNDAY = LocalDate.of(2000, 1, 2)

private fun startOfWeek(date: LocalDate, weekStartsMonday: Boolean): LocalDate {
    val first = if (weekStartsMonday) DayOfWeek.MONDAY else DayOfWeek.SUNDAY
    return date.minusDays(((date.dayOfWeek.value - first.value + 7) % 7).toLong())
}

private fun weekPage(date: LocalDate, weekStartsMonday: Boolean): Int {
    val anchor = if (weekStartsMonday) WEEK_ANCHOR_MONDAY else WEEK_ANCHOR_SUNDAY
    val weeks = (startOfWeek(date, weekStartsMonday).toEpochDay() - anchor.toEpochDay()) / 7
    return weeks.coerceIn(0, (WEEK_PAGE_COUNT - 1).toLong()).toInt()
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
    Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 14.dp)) {
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
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .height(50.dp)
                                .padding(2.dp)
                                .clip(RoundedCornerShape(9.dp))
                                .background(
                                    when {
                                        isSelected -> colors.accent.copy(alpha = 0.16f)
                                        isToday -> colors.field
                                        else -> colors.background
                                    },
                                )
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
