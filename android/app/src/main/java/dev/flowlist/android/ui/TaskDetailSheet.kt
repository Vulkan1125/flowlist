package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.label

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskDetailSheet(
    task: TaskView,
    pending: Boolean,
    onDismiss: () -> Unit,
    onComplete: () -> Unit,
    onDelete: () -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var confirmDelete by remember { mutableStateOf(false) }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp)
                .padding(bottom = 30.dp),
        ) {
            Row(verticalAlignment = Alignment.Top) {
                CheckCircle(checked = task.completed, onClick = onComplete)
                Spacer(modifier = Modifier.size(14.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = task.title,
                        style = MaterialTheme.typography.titleLarge,
                        color = if (task.completed) colors.faint else colors.ink,
                    )
                    Spacer(modifier = Modifier.height(5.dp))
                    Text(
                        text = buildList {
                            task.projectName?.let { add(it) }
                            if (pending) add("Waiting to sync")
                            if (task.flagged) add("Flagged")
                            task.priority?.let { add(it.replaceFirstChar { c -> c.uppercase() } + " priority") }
                        }.joinToString("  ·  "),
                        style = MaterialTheme.typography.labelMedium,
                        color = if (pending) colors.warning else colors.faint,
                    )
                }
            }

            Spacer(modifier = Modifier.height(20.dp))
            DetailRow("Due", task.due?.label() ?: "None")
            DetailRow("Defer", task.defer?.label() ?: "None")
            if (task.tags.isNotEmpty()) DetailRow("Tags", task.tags.joinToString("  ") { "#$it" })
            if (task.estimateMin != null) DetailRow("Estimate", "${task.estimateMin} minutes")
            if (task.notes.isNotBlank()) {
                Spacer(modifier = Modifier.height(18.dp))
                Text(
                    text = "Notes",
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.SemiBold,
                    letterSpacing = 0.7.sp,
                    color = colors.faint,
                )
                Spacer(modifier = Modifier.height(5.dp))
                Text(
                    text = task.notes,
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.ink,
                    lineHeight = 22.sp,
                )
            }
            if (task.checklistTotal > 0) {
                Spacer(modifier = Modifier.height(18.dp))
                Text(
                    text = "Checklist  ${task.checklistDone}/${task.checklistTotal}",
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.SemiBold,
                    letterSpacing = 0.7.sp,
                    color = colors.faint,
                )
            }

            Spacer(modifier = Modifier.height(26.dp))
            Button(
                onClick = onComplete,
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = colors.accent,
                    contentColor = colors.onAccent,
                ),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp),
            ) {
                if (task.completed) {
                    Text("Reopen task")
                } else {
                    Icon(Icons.Rounded.Check, contentDescription = null, modifier = Modifier.size(17.dp))
                    Spacer(modifier = Modifier.size(7.dp))
                    Text("Complete")
                }
            }
            Spacer(modifier = Modifier.height(6.dp))
            TextButton(
                onClick = { confirmDelete = true },
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Move to Trash", color = colors.danger, style = MaterialTheme.typography.labelLarge)
            }
            Text(
                text = "Editing on the phone arrives next; for now changes happen on the Mac.",
                style = MaterialTheme.typography.labelSmall,
                color = colors.faint,
                modifier = Modifier.fillMaxWidth(),
                textAlign = androidx.compose.ui.text.style.TextAlign.Center,
            )
        }
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
}

@Composable
private fun DetailRow(label: String, value: String) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(label, style = MaterialTheme.typography.labelLarge, color = colors.faint)
        Text(value, style = MaterialTheme.typography.labelLarge, color = colors.ink)
    }
    Hairline()
}
