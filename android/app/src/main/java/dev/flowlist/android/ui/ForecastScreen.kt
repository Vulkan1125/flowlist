package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ChevronLeft
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.Snapshot
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.forecast
import dev.flowlist.android.data.isActive
import dev.flowlist.android.data.taskViews
import java.time.LocalDate
import java.time.YearMonth
import java.time.format.DateTimeFormatter
import java.util.Locale

@Composable
fun ForecastScreen(
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
    var monthMode by rememberSaveable { mutableStateOf(true) }
    var monthKey by rememberSaveable { mutableStateOf(YearMonth.now().toString()) }
    var selectedKey by rememberSaveable { mutableStateOf(LocalDate.now().toString()) }
    val month = YearMonth.parse(monthKey)
    val selected = LocalDate.parse(selectedKey)

    val byDate = remember(snapshot?.revision) {
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
    val allViews = remember(snapshot?.revision) {
        snapshot?.db?.taskViews().orEmpty().associateBy { it.id }
    }

    fun shiftMonth(step: Long) {
        val next = month.plusMonths(step)
        monthKey = next.toString()
        val today = LocalDate.now()
        selectedKey = if (YearMonth.from(today) == next) today.toString() else next.atDay(1).toString()
    }

    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 20.dp, end = 8.dp, top = 0.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = month.format(DateTimeFormatter.ofPattern("MMMM yyyy", Locale.getDefault())),
                style = MaterialTheme.typography.titleMedium,
                color = colors.ink,
                modifier = Modifier.weight(1f),
            )
            TextButton(onClick = {
                val today = LocalDate.now()
                monthKey = YearMonth.from(today).toString()
                selectedKey = today.toString()
            }) {
                Text("Today", color = colors.accent)
            }
            IconButton(onClick = { shiftMonth(-1) }, modifier = Modifier.size(34.dp)) {
                Icon(Icons.Rounded.ChevronLeft, "Previous month", tint = colors.faint, modifier = Modifier.size(20.dp))
            }
            IconButton(onClick = { shiftMonth(1) }, modifier = Modifier.size(34.dp)) {
                Icon(Icons.Rounded.ChevronRight, "Next month", tint = colors.faint, modifier = Modifier.size(20.dp))
            }
        }

        Row(
            modifier = Modifier.padding(horizontal = 20.dp, vertical = 2.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            ModePill("Month", monthMode) { monthMode = true }
            ModePill("Agenda", !monthMode) { monthMode = false }
        }

        if (!monthMode) {
            ForecastList(
                modifier = Modifier.weight(1f),
                swipeActions = swipeActions,
                children = children,
                groups = snapshot?.db?.forecast().orEmpty(),
                pending = pending,
                onToggle = onToggle,
                onClick = onClick,
                onTrash = onTrash,
            )
            return@Column
        }

        Spacer(modifier = Modifier.height(8.dp))
        WeekdayHeader(weekStartsMonday)
        MonthGrid(
            month = month,
            weekStartsMonday = weekStartsMonday,
            selected = selected,
            counts = byDate.mapValues { it.value.size },
            onSelect = { selectedKey = it.toString() },
        )

        val dayTasks = byDate[selected].orEmpty().mapNotNull { allViews[it] }
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

        if (dayTasks.isEmpty()) {
            Text(
                text = "Nothing due on this day.",
                style = MaterialTheme.typography.bodyMedium,
                color = colors.faint,
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 18.dp),
            )
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                contentPadding = androidx.compose.foundation.layout.PaddingValues(bottom = 100.dp),
            ) {
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
private fun ModePill(label: String, selected: Boolean, onClick: () -> Unit) {
    val colors = flow
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(9.dp))
            .background(if (selected) colors.field else colors.background)
            .clickable(onClick = onClick)
            .padding(horizontal = 13.dp, vertical = 6.dp),
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
                                .height(46.dp)
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
                        Spacer(modifier = Modifier.weight(1f).height(46.dp).padding(2.dp))
                    }
                }
            }
        }
    }
}
