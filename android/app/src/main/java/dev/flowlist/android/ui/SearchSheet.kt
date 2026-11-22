package dev.flowlist.android.ui

import androidx.compose.foundation.background
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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.Tag
import androidx.compose.material.icons.rounded.BookmarkBorder
import androidx.compose.material.icons.rounded.ChevronRight
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.FolderDto
import dev.flowlist.android.data.PerspectiveDto
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskView
import dev.flowlist.android.data.taskViews
import kotlinx.coroutines.delay

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchSheet(
    db: Db,
    pending: Set<String>,
    viewTaskIds: Set<String>?,
    viewName: String?,
    onDismiss: () -> Unit,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onProject: (ProjectDto) -> Unit,
    onFolder: (FolderDto) -> Unit,
    onTag: (TagDto) -> Unit,
    onPerspective: (PerspectiveDto) -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var query by remember { mutableStateOf("") }
    var scope by rememberSaveable {
        mutableStateOf(if (viewTaskIds == null) "library" else "view")
    }
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) {
        delay(220)
        focus.requestFocus()
    }
    val needle = query.trim().lowercase()
    val scoped = scope == "view" && viewTaskIds != null
    val tasks = remember(db, needle, scoped, viewTaskIds) {
        if (needle.isEmpty()) {
            emptyList()
        } else {
            db.taskViews()
                .filter { task -> !scoped || task.id in viewTaskIds.orEmpty() }
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
    val projects = remember(db, needle, scoped) {
        if (needle.isEmpty() || scoped) {
            emptyList()
        } else {
            db.projects.values
                .filter { it.name.lowercase().contains(needle) }
                .sortedBy { it.name.lowercase() }
                .take(10)
        }
    }
    val folders = remember(db, needle, scoped) {
        if (needle.isEmpty() || scoped) {
            emptyList()
        } else {
            db.folders.values
                .filter { it.name.lowercase().contains(needle) }
                .sortedBy { it.name.lowercase() }
                .take(10)
        }
    }
    val tags = remember(db, needle, scoped) {
        if (needle.isEmpty() || scoped) {
            emptyList()
        } else {
            db.tags.values
                .filter { it.name.lowercase().contains(needle) }
                .sortedBy { it.name.lowercase() }
                .take(10)
        }
    }
    val perspectives = remember(db, needle, scoped) {
        if (needle.isEmpty() || scoped) {
            emptyList()
        } else {
            db.perspectives.values
                .filter { it.name.lowercase().contains(needle) }
                .sortedBy { it.name.lowercase() }
                .take(10)
        }
    }
    val empty = tasks.isEmpty() && projects.isEmpty() && folders.isEmpty() &&
        tags.isEmpty() && perspectives.isEmpty()

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
                placeholder = {
                    Text("Tasks, projects, folders, tags…", color = colors.faint)
                },
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
            if (viewTaskIds != null) {
                Spacer(modifier = Modifier.height(10.dp))
                Row(
                    modifier = Modifier.padding(horizontal = 20.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    SearchScopePill(
                        label = viewName?.let { "This view · $it" } ?: "This view",
                        selected = scope == "view",
                    ) { scope = "view" }
                    SearchScopePill("Library", selected = scope == "library") {
                        scope = "library"
                    }
                }
            }
            Spacer(modifier = Modifier.height(10.dp))
            when {
                needle.isEmpty() -> Text(
                    text = "Search the library cached on this phone.",
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.faint,
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 20.dp),
                )
                empty -> Text(
                    text = "No matches for “$query”.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.faint,
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 24.dp),
                )
                else -> LazyColumn(modifier = Modifier.fillMaxWidth()) {
                    if (tasks.isNotEmpty()) {
                        item(key = "tasks-header") {
                            SearchSection("${tasks.size} task${if (tasks.size == 1) "" else "s"}")
                        }
                        items(tasks, key = { "task-${it.id}" }) { task ->
                            TaskRow(
                                task = task.copy(pending = task.id in pending),
                                onToggle = { onToggle(task) },
                                onClick = { onClick(task) },
                            )
                            Hairline(Modifier.padding(start = 58.dp))
                        }
                    }
                    if (projects.isNotEmpty()) {
                        item(key = "projects-header") { SearchSection("Projects") }
                        items(projects, key = { "project-${it.id}" }) { project ->
                            EntityRow(Icons.Outlined.Folder, project.name) { onProject(project) }
                        }
                    }
                    if (folders.isNotEmpty()) {
                        item(key = "folders-header") { SearchSection("Folders") }
                        items(folders, key = { "folder-${it.id}" }) { folder ->
                            EntityRow(Icons.Outlined.Folder, folder.name) { onFolder(folder) }
                        }
                    }
                    if (tags.isNotEmpty()) {
                        item(key = "tags-header") { SearchSection("Tags") }
                        items(tags, key = { "tag-${it.id}" }) { tag ->
                            EntityRow(Icons.Outlined.Tag, "#${tag.name}") { onTag(tag) }
                        }
                    }
                    if (perspectives.isNotEmpty()) {
                        item(key = "perspectives-header") { SearchSection("Perspectives") }
                        items(perspectives, key = { "perspective-${it.id}" }) { perspective ->
                            EntityRow(Icons.Rounded.BookmarkBorder, perspective.name) {
                                onPerspective(perspective)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun SearchSection(label: String) {
    val colors = flow
    Text(
        text = label.uppercase(),
        style = MaterialTheme.typography.labelSmall,
        fontWeight = FontWeight.SemiBold,
        color = colors.faint,
        modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
    )
}

@Composable
private fun EntityRow(icon: ImageVector, title: String, onClick: () -> Unit) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, contentDescription = null, tint = colors.faint, modifier = Modifier.size(18.dp))
        Spacer(modifier = Modifier.width(12.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.bodyMedium,
            color = colors.ink,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        Icon(
            Icons.Rounded.ChevronRight,
            contentDescription = null,
            tint = colors.faint,
            modifier = Modifier.size(18.dp),
        )
    }
    Hairline(Modifier.padding(start = 50.dp))
}

@Composable
private fun SearchScopePill(label: String, selected: Boolean, onClick: () -> Unit) {
    val colors = flow
    Box(
        modifier = Modifier
            .heightIn(min = 32.dp)
            .background(
                color = if (selected) colors.field else Color.Transparent,
                shape = RoundedCornerShape(9.dp),
            )
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 6.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = if (selected) colors.ink else colors.faint,
        )
    }
}

/** Tasks carrying one tag, opened from a search result or Nearby. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TagTasksSheet(
    tag: TagDto,
    tasks: List<TaskView>,
    pending: Set<String>,
    onToggle: (TaskView) -> Unit,
    onClick: (TaskView) -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(modifier = Modifier.fillMaxWidth().padding(bottom = 20.dp)) {
            Text(
                text = "#${tag.name}",
                style = MaterialTheme.typography.titleLarge,
                color = colors.ink,
                modifier = Modifier.padding(horizontal = 20.dp),
            )
            Text(
                text = if (tasks.size == 1) "1 task" else "${tasks.size} tasks",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp),
            )
            if (tasks.isEmpty()) {
                Text(
                    text = "No open tasks with this tag.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.faint,
                    modifier = Modifier.padding(20.dp),
                )
            } else {
                LazyColumn(modifier = Modifier.fillMaxWidth()) {
                    items(tasks, key = { it.id }) { task ->
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
