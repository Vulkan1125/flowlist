package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.TagDto

/// The shelf appears while a task is being dragged. The phone has no visible
/// sidebar during a drag, so the shelf puts project, tag and Inbox drop targets
/// on screen instead.
@Composable
fun TaskDragShelf(
    projects: List<ProjectDto>,
    tags: List<TagDto>,
    modifier: Modifier = Modifier,
) {
    val colors = flow
    val drag = LocalTaskDrag.current ?: return
    if (drag.dragging == null) return
    Surface(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(18.dp),
        color = colors.raised,
        tonalElevation = 3.dp,
        shadowElevation = 10.dp,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .border(1.dp, colors.hairline, RoundedCornerShape(18.dp))
                .padding(horizontal = 12.dp, vertical = 10.dp),
        ) {
            Text(
                text = "Drop to move",
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.SemiBold,
                color = colors.faint,
            )
            Spacer(modifier = Modifier.padding(top = 2.dp))
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                ShelfChip(
                    label = "Inbox",
                    key = "inbox",
                    active = drag.target?.kind == TaskDropKind.Inbox,
                )
                ShelfChip(
                    label = "Choose project…",
                    key = "choose-project",
                    active = drag.target?.kind == TaskDropKind.ChooseProject,
                )
                projects.forEach { project ->
                    ShelfChip(
                        label = project.name,
                        key = "project:${project.id}",
                        active = drag.target?.kind == TaskDropKind.Project &&
                            drag.target?.id == project.id,
                    )
                }
            }
            if (tags.isNotEmpty()) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 6.dp)
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    ShelfChip(
                        label = "Choose tag…",
                        key = "choose-tag",
                        active = drag.target?.kind == TaskDropKind.ChooseTag,
                    )
                    tags.forEach { tag ->
                        ShelfChip(
                            label = "#${tag.name}",
                            key = "tag:${tag.id}",
                            active = drag.target?.kind == TaskDropKind.Tag &&
                                drag.target?.id == tag.id,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ShelfChip(label: String, key: String, active: Boolean) {
    val colors = flow
    val drag = LocalTaskDrag.current ?: return
    Text(
        text = label,
        style = MaterialTheme.typography.labelMedium,
        color = if (active) colors.onAccent else colors.ink,
        modifier = Modifier
            .onGloballyPositioned { drag.bounds[key] = it.boundsInWindow() }
            .clip(RoundedCornerShape(percent = 50))
            .background(if (active) colors.accent else colors.field)
            .border(
                1.dp,
                if (active) colors.accent else colors.hairline,
                RoundedCornerShape(percent = 50),
            )
            .padding(horizontal = 12.dp, vertical = 7.dp),
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DragPickerSheet(
    title: String,
    options: List<Pair<String, String>>,
    onDismiss: () -> Unit,
    onPick: (String) -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(modifier = Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
            Text(
                text = title,
                style = MaterialTheme.typography.headlineMedium,
                color = colors.ink,
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
            )
            LazyColumn(modifier = Modifier.heightIn(max = 480.dp)) {
                items(options, key = { it.first }) { (id, label) ->
                    Text(
                        text = label,
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.ink,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onPick(id) }
                            .padding(horizontal = 20.dp, vertical = 14.dp),
                    )
                    HorizontalDivider(color = colors.hairline)
                }
            }
        }
    }
}
