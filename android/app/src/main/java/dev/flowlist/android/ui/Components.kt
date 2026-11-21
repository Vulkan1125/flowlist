package dev.flowlist.android.ui

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.Flag
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.TextButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.isOverdue
import dev.flowlist.android.data.label

@Composable
fun CheckCircle(checked: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
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
    Box(
        modifier = modifier
            .size(24.dp)
            .scale(scale)
            .clip(CircleShape)
            .background(background)
            .border(
                width = 1.5.dp,
                color = if (checked) colors.accent else colors.hairline,
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
                modifier = Modifier.size(15.dp),
            )
        }
    }
}

@Composable
fun TaskRow(
    task: TaskView,
    onToggle: () -> Unit,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = flow
    Row(
        modifier = modifier
            .fillMaxWidth()
            .heightIn(min = 54.dp)
            .padding(horizontal = 20.dp, vertical = 10.dp),
        verticalAlignment = Alignment.Top,
    ) {
        Spacer(modifier = Modifier.height(1.dp))
        CheckCircle(
            checked = task.completed,
            onClick = onToggle,
            modifier = Modifier.padding(top = 1.dp),
        )
        Spacer(modifier = Modifier.width(14.dp))
        Column(
            modifier = Modifier
                .weight(1f)
                .clickable(onClick = onClick),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = task.title,
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (task.completed) colors.faint else colors.ink,
                    textDecoration = if (task.completed) TextDecoration.LineThrough else null,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false),
                )
                if (task.flagged && !task.completed) {
                    Spacer(modifier = Modifier.width(6.dp))
                    Icon(
                        imageVector = Icons.Rounded.Flag,
                        contentDescription = "Flagged",
                        tint = colors.warning,
                        modifier = Modifier.size(15.dp),
                    )
                }
            }
            val details = buildList {
                task.projectName?.let { add(it) }
                task.due?.let { add(it.label()) }
                if (task.checklistTotal > 0) add("${task.checklistDone}/${task.checklistTotal}")
                if (task.estimateMin != null) add("${task.estimateMin}m")
                if (task.tags.isNotEmpty()) add(task.tags.joinToString(" ") { "#$it" })
            }
            if (details.isNotEmpty()) {
                Spacer(modifier = Modifier.height(3.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = details.joinToString("  ·  "),
                        style = MaterialTheme.typography.labelMedium,
                        color = when {
                            task.due?.isOverdue() == true && !task.completed -> colors.danger
                            else -> colors.faint
                        },
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
        if (task.pending) {
            Spacer(modifier = Modifier.width(8.dp))
            PendingChip()
        }
        if (task.priority != null && !task.completed) {
            Spacer(modifier = Modifier.width(8.dp))
            PriorityMark(task.priority)
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
fun PriorityMark(priority: String?) {
    val colors = flow
    val (label, color) = when (priority) {
        "high" -> "High" to colors.danger
        "medium" -> "Medium" to colors.warning
        else -> "Low" to colors.faint
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(
            modifier = Modifier
                .size(6.dp)
                .clip(CircleShape)
                .background(color),
        )
        Spacer(modifier = Modifier.width(5.dp))
        Text(
            text = label,
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
        modifier = modifier.padding(horizontal = 20.dp, vertical = 8.dp),
    )
}

@Composable
fun EmptyState(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    title: String,
    body: String,
    modifier: Modifier = Modifier,
) {
    val colors = flow
    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 40.dp, vertical = 64.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier
                .size(52.dp)
                .clip(CircleShape)
                .background(colors.field),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = colors.faint,
                modifier = Modifier.size(22.dp),
            )
        }
        Spacer(modifier = Modifier.height(16.dp))
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
        )
    }
}

@Composable
fun FlowSnackbarHost(hostState: SnackbarHostState) {
    val colors = flow
    SnackbarHost(hostState = hostState) { data ->
        Row(
            modifier = Modifier
                .padding(horizontal = 16.dp, vertical = 10.dp)
                .fillMaxWidth()
                .clip(RoundedCornerShape(12.dp))
                .background(colors.ink)
                .padding(start = 16.dp, end = 6.dp, top = 6.dp, bottom = 6.dp),
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
