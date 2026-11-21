package dev.flowlist.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.taskViews
import kotlinx.coroutines.delay

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchSheet(
    db: Db,
    pending: Set<String>,
    onDismiss: () -> Unit,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var query by remember { mutableStateOf("") }
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) {
        delay(220)
        focus.requestFocus()
    }
    val needle = query.trim().lowercase()
    val results = remember(db, needle) {
        if (needle.isEmpty()) {
            emptyList()
        } else {
            db.taskViews()
                .filter { task ->
                    task.title.lowercase().contains(needle) ||
                        task.notes.lowercase().contains(needle) ||
                        task.projectName?.lowercase()?.contains(needle) == true ||
                        task.tags.any { it.lowercase().contains(needle) }
                }
                .sortedBy { it.title.lowercase() }
                .take(80)
        }
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(modifier = Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
            Text(
                text = "Search",
                style = MaterialTheme.typography.headlineMedium,
                color = colors.ink,
                modifier = Modifier.padding(horizontal = 20.dp),
            )
            Spacer(modifier = Modifier.height(10.dp))
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                placeholder = { Text("Tasks, notes, projects or tags", color = colors.faint) },
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                trailingIcon = {
                    if (query.isNotEmpty()) {
                        IconButton(onClick = { query = "" }) {
                            Icon(Icons.Rounded.Close, "Clear", tint = colors.faint)
                        }
                    }
                },
                shape = RoundedCornerShape(12.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = colors.accent,
                    unfocusedBorderColor = colors.hairline,
                    focusedContainerColor = colors.field,
                    unfocusedContainerColor = colors.field,
                    focusedTextColor = colors.ink,
                    unfocusedTextColor = colors.ink,
                    cursorColor = colors.accent,
                ),
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp)
                    .focusRequester(focus),
            )
            Spacer(modifier = Modifier.height(10.dp))
            when {
                needle.isEmpty() -> Text(
                    text = "Type to search the library cached on this phone.",
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.faint,
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 20.dp),
                )
                results.isEmpty() -> Text(
                    text = "No matches for “$query”.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.faint,
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 24.dp),
                )
                else -> LazyColumn(modifier = Modifier.fillMaxWidth()) {
                    item(key = "count") {
                        Text(
                            text = "${results.size} matching tasks",
                            style = MaterialTheme.typography.labelSmall,
                            color = colors.faint,
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                        )
                    }
                    items(results, key = { it.id }) { task ->
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
    }
}
