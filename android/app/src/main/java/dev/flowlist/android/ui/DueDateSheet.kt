package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ChevronLeft
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimeInput
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.data.StampDto
import dev.flowlist.android.data.label
import java.time.DayOfWeek
import java.time.LocalDate
import java.time.LocalTime
import java.time.YearMonth
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.time.format.TextStyle
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DueDateSheet(
    title: String,
    initial: StampDto?,
    onDismiss: () -> Unit,
    onSet: (StampDto?) -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var selected by remember {
        mutableStateOf(initial?.local()?.toLocalDate() ?: LocalDate.now())
    }
    var month by remember { mutableStateOf(YearMonth.from(selected)) }
    var hasTime by remember { mutableStateOf(initial?.hasTime ?: false) }
    var time by remember {
        mutableStateOf(initial?.local()?.toLocalTime() ?: LocalTime.of(9, 0))
    }
    var customTime by remember { mutableStateOf(false) }

    fun stamp(): StampDto {
        val zone = ZoneId.systemDefault()
        val moment = ZonedDateTime.of(selected, if (hasTime) time else LocalTime.MIDNIGHT, zone)
        return StampDto(moment.toInstant().toString(), hasTime)
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
                .padding(horizontal = 20.dp)
                .padding(bottom = 24.dp),
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.headlineMedium,
                color = colors.ink,
            )
            Spacer(modifier = Modifier.height(3.dp))
            Text(
                text = if (hasTime) {
                    "${selected.format(DateTimeFormatter.ofPattern("EEE, d MMM", Locale.getDefault()))} · ${time.format(DateTimeFormatter.ofPattern("HH:mm"))}"
                } else {
                    selected.format(DateTimeFormatter.ofPattern("EEEE, d MMMM", Locale.getDefault()))
                },
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
            )

            Spacer(modifier = Modifier.height(14.dp))
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                QuickDate("Today", LocalDate.now(), selected == LocalDate.now()) {
                    selected = LocalDate.now()
                    month = YearMonth.from(selected)
                }
                QuickDate("Tomorrow", LocalDate.now().plusDays(1), selected == LocalDate.now().plusDays(1)) {
                    selected = LocalDate.now().plusDays(1)
                    month = YearMonth.from(selected)
                }
                QuickDate("Next week", LocalDate.now().plusWeeks(1), selected == LocalDate.now().plusWeeks(1)) {
                    selected = LocalDate.now().plusWeeks(1)
                    month = YearMonth.from(selected)
                }
                Pill("No date", selected = false, tone = colors.danger) { onSet(null) }
            }

            Spacer(modifier = Modifier.height(18.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(
                    onClick = { month = month.minusMonths(1) },
                    modifier = Modifier.size(32.dp),
                ) {
                    Icon(Icons.Rounded.ChevronLeft, "Previous month", tint = colors.faint)
                }
                Text(
                    text = month.format(DateTimeFormatter.ofPattern("MMMM yyyy", Locale.getDefault())),
                    style = MaterialTheme.typography.titleMedium,
                    color = colors.ink,
                    modifier = Modifier.weight(1f),
                )
                IconButton(
                    onClick = { month = month.plusMonths(1) },
                    modifier = Modifier.size(32.dp),
                ) {
                    Icon(Icons.Rounded.ChevronRight, "Next month", tint = colors.faint)
                }
            }

            Row(modifier = Modifier.fillMaxWidth()) {
                DayOfWeek.entries.forEach { day ->
                    Text(
                        text = day.getDisplayName(TextStyle.NARROW, Locale.getDefault()),
                        style = MaterialTheme.typography.labelSmall,
                        color = colors.faint,
                        modifier = Modifier.weight(1f),
                        textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                    )
                }
            }
            Spacer(modifier = Modifier.height(4.dp))

            val first = month.atDay(1)
            val offset = (first.dayOfWeek.value - DayOfWeek.MONDAY.value + 7) % 7
            val days = month.lengthOfMonth()
            val cells = 42
            for (row in 0 until 6) {
                Row(modifier = Modifier.fillMaxWidth()) {
                    for (column in 0 until 7) {
                        val index = row * 7 + column
                        val day = index - offset + 1
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .height(40.dp),
                            contentAlignment = Alignment.Center,
                        ) {
                            if (day in 1..days) {
                                val date = month.atDay(day)
                                val isSelected = date == selected
                                val isToday = date == LocalDate.now()
                                Box(
                                    modifier = Modifier
                                        .size(34.dp)
                                        .clip(CircleShape)
                                        .background(if (isSelected) colors.accent else Color.Transparent)
                                        .then(
                                            if (isToday && !isSelected) {
                                                Modifier.border(1.dp, colors.accent, CircleShape)
                                            } else {
                                                Modifier
                                            },
                                        )
                                        .clickable {
                                            selected = date
                                        },
                                    contentAlignment = Alignment.Center,
                                ) {
                                    Text(
                                        text = "$day",
                                        style = MaterialTheme.typography.labelLarge,
                                        color = when {
                                            isSelected -> colors.onAccent
                                            isToday -> colors.accent
                                            else -> colors.ink
                                        },
                                    )
                                }
                            }
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(10.dp))
            Hairline()
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "TIME",
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.SemiBold,
                letterSpacing = 0.7.sp,
                color = colors.faint,
            )
            Spacer(modifier = Modifier.height(8.dp))
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Pill("All day", !hasTime, colors.accent) {
                    hasTime = false
                }
                listOf(9 to 0, 13 to 0, 18 to 0).forEach { (hour, minute) ->
                    val preset = LocalTime.of(hour, minute)
                    Pill(
                        label = preset.format(DateTimeFormatter.ofPattern("HH:mm")),
                        selected = hasTime && time == preset,
                        tone = colors.accent,
                    ) {
                        time = preset
                        hasTime = true
                    }
                }
                Pill("Custom", hasTime && time !in setOf(
                    LocalTime.of(9, 0),
                    LocalTime.of(13, 0),
                    LocalTime.of(18, 0),
                ), colors.accent) {
                    customTime = true
                }
            }

            Spacer(modifier = Modifier.height(22.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                TextButton(
                    onClick = onDismiss,
                    modifier = Modifier.weight(1f),
                ) {
                    Text("Cancel", color = colors.faint)
                }
                Button(
                    onClick = { onSet(stamp()) },
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = colors.accent,
                        contentColor = colors.onAccent,
                    ),
                    modifier = Modifier
                        .weight(1.4f)
                        .height(48.dp),
                ) {
                    Text("Set")
                }
            }
        }
    }

    if (customTime) {
        val state = rememberTimePickerState(
            initialHour = time.hour,
            initialMinute = time.minute,
            is24Hour = true,
        )
        AlertDialog(
            onDismissRequest = { customTime = false },
            title = { Text("Time") },
            text = { TimeInput(state = state) },
            confirmButton = {
                TextButton(
                    onClick = {
                        time = LocalTime.of(state.hour, state.minute)
                        hasTime = true
                        customTime = false
                    },
                ) {
                    Text("Set", color = colors.accent)
                }
            },
            dismissButton = {
                TextButton(onClick = { customTime = false }) {
                    Text("Cancel", color = colors.faint)
                }
            },
            containerColor = colors.background,
        )
    }
}

@Composable
private fun QuickDate(label: String, date: LocalDate, selected: Boolean, onClick: () -> Unit) {
    Pill(label, selected, flow.accent, onClick)
}

@Composable
private fun Pill(
    label: String,
    selected: Boolean,
    tone: Color,
    onClick: () -> Unit,
) {
    val colors = flow
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(9.dp))
            .background(if (selected) tone.copy(alpha = 0.14f) else colors.background)
            .border(
                width = 1.dp,
                color = if (selected) tone else colors.hairline,
                shape = RoundedCornerShape(9.dp),
            )
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 7.dp),
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = if (selected) tone else colors.ink,
        )
    }
}
